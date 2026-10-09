use crate::bibtex::{parse_bibtex, serialize_bibtex, BibEntry, Bibliography};
use crate::project::TextEncoding;
use crate::storage::xdg_cache_home;
use gtk::gio::prelude::FileExt;
use regex::Regex;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const BUILD_TIMEOUT: Duration = Duration::from_secs(300);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LatexEngine {
    LatexMk,
    PdfLatex,
    Tectonic,
}

impl LatexEngine {
    fn executable(self) -> &'static str {
        match self {
            Self::LatexMk => "latexmk",
            Self::PdfLatex => "pdflatex",
            Self::Tectonic => "tectonic",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildResult {
    pub pdf_path: PathBuf,
    pub engine: LatexEngine,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingCitation {
    pub key: String,
    pub line: usize,
}

/// Extracts the source line from fatal LaTeX diagnostics without mistaking
/// unrelated warnings (for example Fontconfig warnings) for a source error.
pub fn compile_error_line(message: &str) -> Option<usize> {
    let lines = message.lines().collect::<Vec<_>>();
    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.to_ascii_lowercase().starts_with("error:") {
            for (offset, _) in line.match_indices(':') {
                let before = &line[..offset];
                if !before.to_ascii_lowercase().ends_with(".tex") {
                    continue;
                }
                let after = line[offset + 1..].trim_start();
                let digits = after
                    .chars()
                    .take_while(char::is_ascii_digit)
                    .collect::<String>();
                if let Ok(number) = digits.parse::<usize>() {
                    if number > 0 {
                        return Some(number);
                    }
                }
            }
        }

        if trimmed.starts_with('!') {
            for following in lines.iter().skip(index + 1) {
                let following = following.trim_start();
                if following.starts_with('!') {
                    break;
                }
                if let Some(line_number) = following.strip_prefix("l.") {
                    let digits = line_number
                        .chars()
                        .take_while(char::is_ascii_digit)
                        .collect::<String>();
                    if let Ok(number) = digits.parse::<usize>() {
                        if number > 0 {
                            return Some(number);
                        }
                    }
                }
            }
        }
    }
    None
}

pub enum CitationBibliographyUpdate {
    BibFile {
        path: PathBuf,
        content: String,
        encoding: TextEncoding,
    },
    InlineTex {
        content: String,
        inserted_text: String,
        insertion_offset: usize,
        encoding: TextEncoding,
    },
    AlreadyPresent,
}

pub fn build_output_directory(source_path: &Path) -> PathBuf {
    let digest = Sha256::digest(source_path.to_string_lossy().as_bytes());
    let source_id = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    xdg_cache_home()
        .join("ovenbird")
        .join("build")
        .join(source_id)
}

pub fn find_latex_engine() -> Option<(LatexEngine, PathBuf)> {
    for engine in [
        LatexEngine::LatexMk,
        LatexEngine::PdfLatex,
        LatexEngine::Tectonic,
    ] {
        if let Some(path) = find_program(engine.executable()) {
            return Some((engine, path));
        }
    }
    None
}

fn find_program(program: &str) -> Option<PathBuf> {
    for directory in std::env::split_paths(&std::env::var_os("PATH")?) {
        let candidate = directory.join(program);
        if candidate.is_file() {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if fs::metadata(&candidate).ok()?.permissions().mode() & 0o111 == 0 {
                    continue;
                }
            }
            return Some(candidate);
        }
    }
    None
}

fn resource_path(value: &str) -> Option<String> {
    let normalized = value.trim().replace('\\', "/");
    if normalized.is_empty() || normalized.starts_with('/') || normalized.starts_with('~') {
        return None;
    }
    let path = Path::new(&normalized);
    if path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return None;
    }
    let with_extension = match path.extension().and_then(|extension| extension.to_str()) {
        Some(extension) if extension.eq_ignore_ascii_case("bib") => normalized,
        Some(_) => return None,
        None => format!("{normalized}.bib"),
    };
    Some(with_extension)
}

pub fn bib_resources(source: &str) -> Vec<String> {
    let add_resource = Regex::new(r"(?s)\\addbibresource(?:\s*\[[^\]]*\])?\s*\{([^}]+)\}").unwrap();
    let bibliography = Regex::new(r"(?s)\\bibliography\s*\{([^}]+)\}").unwrap();
    let mut resources = add_resource
        .captures_iter(source)
        .filter_map(|capture| resource_path(&capture[1]))
        .collect::<Vec<_>>();
    for capture in bibliography.captures_iter(source) {
        resources.extend(capture[1].split(',').filter_map(resource_path));
    }
    resources
}

pub fn prepare_citation_bibliography_update(
    source_path: &Path,
    source: &str,
    source_encoding: TextEncoding,
    entry: &BibEntry,
) -> Result<CitationBibliographyUpdate, String> {
    if entry.key.trim().is_empty() {
        return Err("The reference has no citation key.".to_owned());
    }
    let project_folder = source_path
        .parent()
        .ok_or("The document has no parent folder.")?;
    let root_source = clean_tex_source(source);
    let resources = bib_resources(&root_source);
    if !resources.is_empty() {
        let mut target = None;
        for resource in resources {
            let path = project_folder.join(&resource);
            let (text, encoding) = match fs::read(&path) {
                Ok(bytes) => {
                    let (text, encoding) = crate::project::decode_text(&bytes);
                    (text, encoding)
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => (
                    String::new(),
                    TextEncoding::Utf8,
                ),
                Err(error) => return Err(format!("Could not read {resource}: {error}")),
            };
            let bibliography = if text.is_empty() {
                Bibliography {
                    entries: Vec::new(),
                    directives: Vec::new(),
                }
            } else {
                parse_bibtex(&text)
                    .map_err(|error| format!("Could not read {resource}: {error}"))?
            };
            if bibliography
                .entries
                .iter()
                .any(|candidate| candidate.key.eq_ignore_ascii_case(&entry.key))
            {
                return Ok(CitationBibliographyUpdate::AlreadyPresent);
            }
            if target.is_none() {
                target = Some((path, text, encoding));
            }
        }
        let Some((path, original, encoding)) = target else {
            return Err("No valid bibliography file is configured in the document.".to_owned());
        };
        let serialized_entry = serialize_bibtex(&Bibliography {
            entries: vec![entry.clone()],
            directives: Vec::new(),
        });
        let separator = if original.is_empty() {
            ""
        } else if original.ends_with('\n') {
            "\n"
        } else {
            "\n\n"
        };
        return Ok(CitationBibliographyUpdate::BibFile {
            path,
            content: format!("{original}{separator}{serialized_entry}"),
            encoding,
        });
    }

    let style = crate::bibtex::reference_style_for_document(&root_source);
    prepare_inline_bibliography_update(source, source_encoding, entry, style)
}

pub fn prepare_inline_bibliography_update(
    source: &str,
    source_encoding: TextEncoding,
    entry: &BibEntry,
    style: crate::bibtex::ReferenceStyle,
) -> Result<CitationBibliographyUpdate, String> {
    if entry.key.trim().is_empty() {
        return Err("The reference has no citation key.".to_owned());
    }
    let root_source = clean_tex_source(source);
    let bibitem = Regex::new(r"(?i)\\bibitem\*?(?:\s*\[[^\]]*\])?\s*\{([^{}]+)\}")
        .unwrap();
    if bibitem
        .captures_iter(&root_source)
        .any(|capture| capture[1].trim().eq_ignore_ascii_case(&entry.key))
    {
        return Ok(CitationBibliographyUpdate::AlreadyPresent);
    }

    let begin_marker = "\\begin{thebibliography}";
    let end_marker = "\\end{thebibliography}";
    let has_begin = root_source.contains(begin_marker);
    let end_offset = last_active_command_offset(source, end_marker);
    if has_begin != end_offset.is_some() {
        return Err("The document has an incomplete bibliography environment.".to_owned());
    }

    let citation = escape_inline_latex_text(&crate::bibtex::format_reference_citation(entry, style));
    let bibitem_text = format!("\\bibitem{{{}}} {citation}\n", entry.key);
    let (byte_offset, inserted_text) = if let Some(offset) = end_offset {
        (offset, bibitem_text)
    } else {
        let bibliography = format!(
            "\\begin{{thebibliography}}{{00}}\n{bibitem_text}\\end{{thebibliography}}\n\n"
        );
        (
            last_active_command_offset(source, "\\end{document}").unwrap_or(source.len()),
            format!("\n{bibliography}"),
        )
    };
    let content = format!(
        "{}{}{}",
        &source[..byte_offset],
        inserted_text,
        &source[byte_offset..]
    );
    Ok(CitationBibliographyUpdate::InlineTex {
        content,
        inserted_text,
        insertion_offset: source[..byte_offset].chars().count(),
        encoding: source_encoding,
    })
}

fn last_active_command_offset(source: &str, command: &str) -> Option<usize> {
    let mut line_offset = 0;
    let mut last = None;
    for line in source.split_inclusive('\n') {
        let line_without_newline = line.strip_suffix('\n').unwrap_or(line);
        let code = before_unescaped_comment(line_without_newline);
        if let Some(offset) = code.rfind(command) {
            last = Some(line_offset + offset);
        }
        line_offset += line.len();
    }
    last
}

fn before_unescaped_comment(line: &str) -> &str {
    let mut backslashes = 0;
    for (offset, character) in line.char_indices() {
        if character == '%' && backslashes % 2 == 0 {
            return &line[..offset];
        }
        if character == '\\' {
            backslashes += 1;
        } else {
            backslashes = 0;
        }
    }
    line
}

fn escape_inline_latex_text(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut backslashes = 0;
    for character in value.chars() {
        let already_escaped = backslashes % 2 == 1;
        match character {
            '#' | '$' | '%' | '&' | '_' if !already_escaped => {
                output.push('\\');
                output.push(character);
            }
            '^' if !already_escaped => output.push_str("\\textasciicircum{}"),
            '~' if !already_escaped => output.push_str("\\textasciitilde{}"),
            _ => output.push(character),
        }
        if character == '\\' {
            backslashes += 1;
        } else {
            backslashes = 0;
        }
    }
    output
}

pub fn find_missing_citations(
    source_path: &Path,
    source: &str,
) -> Result<Vec<MissingCitation>, String> {
    let root_source = clean_tex_source(source);
    let sources = project_tex_sources(source_path, &root_source);
    let mut cited = HashMap::new();
    let citation = Regex::new(
        r"(?i)\\([A-Za-z@]*cite[A-Za-z@]*\*?)(?:\s*\[[^\]]*\]|\s*\{[^{}]*\})+",
    )
    .unwrap();
    let citation_options = Regex::new(r"\[[^\]]*\]").unwrap();
    let citation_keys = Regex::new(r"\{([^{}]*)\}").unwrap();
    let bibitem = Regex::new(r"(?i)\\bibitem\*?(?:\s*\[[^\]]*\])?\s*\{([^{}]+)\}")
        .unwrap();
    let mut available = HashMap::new();

    for (_, text) in &sources {
        for capture in citation.captures_iter(text) {
            let is_multi_citation = capture[1].to_ascii_lowercase().ends_with("cites");
            let citation_without_options = citation_options.replace_all(&capture[0], "");
            let line = text[..capture.get(0).unwrap().start()]
                .bytes()
                .filter(|byte| *byte == b'\n')
                .count()
                + 1;
            for (index, key_capture) in citation_keys
                .captures_iter(&citation_without_options)
                .enumerate()
            {
                if index > 0 && !is_multi_citation {
                    break;
                }
                for key in key_capture[1].split(',').map(str::trim).filter(|key| {
                    !key.is_empty() && *key != "*" && !key.contains('\\')
                }) {
                    cited
                        .entry(key.to_ascii_lowercase())
                        .or_insert_with(|| MissingCitation {
                            key: key.to_owned(),
                            line,
                        });
                }
            }
        }
        available.extend(bibitem.captures_iter(text).filter_map(|capture| {
            let key = capture[1].trim();
            (!key.is_empty()).then(|| (key.to_ascii_lowercase(), key.to_owned()))
        }));
    }

    let project_folder = source_path
        .parent()
        .ok_or("The document has no parent folder.")?;
    // The main document determines which .bib resources are part of this build.
    let resources = declared_bibliography_paths(std::slice::from_ref(&root_source), project_folder);
    for (path, display_path) in resources {
        match fs::read_to_string(&path) {
            Ok(text) => {
                let bibliography = parse_bibtex(&text).map_err(|error| {
                    format!("Could not read configured bibliography {display_path}: {error}")
                })?;
                available.extend(bibliography.entries.into_iter().map(|entry| {
                    let normalized = entry.key.to_ascii_lowercase();
                    (normalized, entry.key)
                }));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(format!(
                    "Could not read configured bibliography {display_path}: {error}"
                ));
            }
        }
    }

    let mut missing = cited
        .into_iter()
        .filter_map(|(normalized, citation)| {
            (!available.contains_key(&normalized)).then_some(citation)
        })
        .collect::<Vec<_>>();
    missing.sort_by(|left, right| {
        left.line
            .cmp(&right.line)
            .then_with(|| left.key.cmp(&right.key))
    });
    Ok(missing)
}

fn declared_bibliography_paths(sources: &[String], project_folder: &Path) -> Vec<(PathBuf, String)> {
    let add_resource = Regex::new(r"(?s)\\addbibresource(?:\s*\[[^\]]*\])?\s*\{([^}]+)\}")
        .unwrap();
    let bibliography = Regex::new(r"(?s)\\bibliography\s*\{([^}]+)\}").unwrap();
    let mut paths = Vec::new();
    let mut seen = HashSet::new();
    for source in sources {
        let names = add_resource
            .captures_iter(source)
            .map(|capture| capture[1].trim().to_owned())
            .chain(
                bibliography
                    .captures_iter(source)
                    .flat_map(|capture| {
                        capture[1]
                            .split(',')
                            .map(|name| name.trim().to_owned())
                            .collect::<Vec<_>>()
                    }),
            );
        for name in names {
            let Some((path, display_path)) = bibliography_path(&name, project_folder) else {
                continue;
            };
            let identity = fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
            if seen.insert(identity) {
                paths.push((path, display_path));
            }
        }
    }
    paths
}

fn bibliography_path(name: &str, project_folder: &Path) -> Option<(PathBuf, String)> {
    let name = name.trim();
    if name.is_empty() || name.starts_with('~') || name.contains('\\') || name.contains('$') {
        return None;
    }
    let relative = PathBuf::from(resource_path(name)?);
    let display_path = relative.to_string_lossy().into_owned();
    let path = if relative.is_absolute() {
        relative
    } else {
        project_folder.join(relative)
    };
    Some((path, display_path))
}

fn project_tex_sources(source_path: &Path, root_source: &str) -> Vec<(PathBuf, String)> {
    let include = Regex::new(r"\\(?:input|include|subfile)\s*(?:\{([^{}]+)\}|([^\s{}]+))")
        .unwrap();
    let mut pending = vec![(source_path.to_path_buf(), root_source.to_owned())];
    let mut seen = HashSet::new();
    let mut sources = Vec::new();
    while let Some((path, source)) = pending.pop() {
        let identity = fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
        if !seen.insert(identity) {
            continue;
        }
        let cleaned = clean_tex_source(&source);
        for capture in include.captures_iter(&cleaned) {
            let Some(name) = capture.get(1).or_else(|| capture.get(2)).map(|name| name.as_str())
            else {
                continue;
            };
            let name = name.trim();
            if name.is_empty() || name.contains('\\') || name.contains('$') {
                continue;
            }
            let mut included = PathBuf::from(name);
            if !included.is_absolute() {
                included = path.parent().unwrap_or(Path::new(".")).join(included);
            }
            if !included.is_file() && included.extension().is_none() {
                included.set_extension("tex");
            }
            if let Ok(contents) = fs::read_to_string(&included) {
                pending.push((included, contents));
            }
        }
        sources.push((path, cleaned));
    }
    sources
}

fn clean_tex_source(source: &str) -> String {
    let mut uncommented = String::new();
    for line in source.lines() {
        let mut backslashes = 0;
        for character in line.chars() {
            if character == '%' && backslashes % 2 == 0 {
                break;
            }
            uncommented.push(character);
            if character == '\\' {
                backslashes += 1;
            } else {
                backslashes = 0;
            }
        }
        uncommented.push('\n');
    }

    let mut cleaned = uncommented;
    for environment in [
        "verbatim",
        "verbatim*",
        "Verbatim",
        "Verbatim*",
        "lstlisting",
        "minted",
        "comment",
    ] {
        let start_marker = format!("\\begin{{{environment}}}");
        let end_marker = format!("\\end{{{environment}}}");
        while let Some(start) = cleaned.find(&start_marker) {
            let content_start = start + start_marker.len();
            let Some(end_offset) = cleaned[content_start..].find(&end_marker) else {
                cleaned.truncate(start);
                break;
            };
            let end = content_start + end_offset + end_marker.len();
            let replacement = cleaned[start..end]
                .chars()
                .map(|character| if character == '\n' { '\n' } else { ' ' })
                .collect::<String>();
            cleaned.replace_range(start..end, &replacement);
        }
    }
    strip_inline_verb(&cleaned)
}

fn strip_inline_verb(source: &str) -> String {
    let mut cleaned = String::new();
    let mut cursor = 0;
    while let Some(offset) = source[cursor..].find("\\verb") {
        let start = cursor + offset;
        let preceding_slashes = source[..start]
            .chars()
            .rev()
            .take_while(|character| *character == '\\')
            .count();
        let command_end = start + "\\verb".len();
        let follows_command = source[command_end..]
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_alphabetic() || character == '@');
        if preceding_slashes % 2 == 1 || follows_command {
            cleaned.push_str(&source[cursor..command_end]);
            cursor = command_end;
            continue;
        }
        let delimiter_start = if source[command_end..].starts_with('*') {
            command_end + 1
        } else {
            command_end
        };
        let Some(delimiter) = source[delimiter_start..].chars().next() else {
            cleaned.push_str(&source[cursor..]);
            return cleaned;
        };
        let content_start = delimiter_start + delimiter.len_utf8();
        let Some(end_offset) = source[content_start..].find(delimiter) else {
            cleaned.push_str(&source[cursor..start]);
            return cleaned;
        };
        cleaned.push_str(&source[cursor..start]);
        cursor = content_start + end_offset + delimiter.len_utf8();
    }
    cleaned.push_str(&source[cursor..]);
    cleaned
}

fn run_process(
    executable: &Path,
    args: &[String],
    working_directory: &Path,
    env: &HashMap<String, String>,
) -> Result<(), String> {
    let mut command = Command::new(executable);
    command
        .args(args)
        .current_dir(working_directory)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command.envs(env);
    let mut child = command.spawn().map_err(|error| error.to_string())?;
    let mut stdout = child.stdout.take().expect("stdout is piped");
    let mut stderr = child.stderr.take().expect("stderr is piped");
    let stdout_reader = thread::spawn(move || {
        let mut output = Vec::new();
        let _ = stdout.read_to_end(&mut output);
        output
    });
    let stderr_reader = thread::spawn(move || {
        let mut output = Vec::new();
        let _ = stderr.read_to_end(&mut output);
        output
    });
    let started = Instant::now();
    let status = loop {
        match child.try_wait().map_err(|error| error.to_string())? {
            Some(status) => break status,
            None if started.elapsed() >= BUILD_TIMEOUT => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "Compilation stopped after {} minutes.",
                    BUILD_TIMEOUT.as_secs() / 60
                ));
            }
            None => thread::sleep(Duration::from_millis(100)),
        }
    };
    let output = stdout_reader.join().unwrap_or_default();
    let error = stderr_reader.join().unwrap_or_default();
    if status.success() {
        Ok(())
    } else {
        let mut message = String::from_utf8_lossy(&output).into_owned();
        if !error.is_empty() {
            if !message.is_empty() && !message.ends_with('\n') {
                message.push('\n');
            }
            message.push_str(&String::from_utf8_lossy(&error));
        }
        Err(message.trim().to_owned())
    }
}

pub fn compile_latex(source_path: &Path) -> Result<BuildResult, String> {
    let (engine, executable) = find_latex_engine().ok_or_else(|| {
        crate::i18n::gettext("No LaTeX compiler was found. Install latexmk, Tectonic, or TeX Live.")
    })?;
    if !source_path.is_file() {
        return Err("The document must be available as a local file.".to_owned());
    }
    let project_folder = source_path
        .parent()
        .ok_or("The document has no parent folder.")?;
    let build_directory = build_output_directory(source_path);
    fs::create_dir_all(&build_directory)
        .map_err(|error| format!("Could not prepare the build folder: {error}"))?;
    let basename = source_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("The file name is invalid.")?;
    let job_name = Path::new(basename)
        .file_stem()
        .and_then(|name| name.to_str())
        .ok_or("The file name is invalid.")?;
    let expected_pdf = build_directory.join(format!("{job_name}.pdf"));
    compile_with_engine(
        engine,
        &executable,
        source_path,
        &build_directory,
        &expected_pdf,
        project_folder,
    )?;
    Ok(BuildResult {
        pdf_path: expected_pdf,
        engine,
    })
}

fn compile_with_engine(
    engine: LatexEngine,
    executable: &Path,
    source_path: &Path,
    build_directory: &Path,
    expected_pdf: &Path,
    project_folder: &Path,
) -> Result<(), String> {
    let existing = std::env::var("BIBINPUTS").unwrap_or_default();
    let mut search = Vec::new();
    search.push(project_folder.to_string_lossy().into_owned());
    if !existing.is_empty() {
        search.push(existing);
    }
    let env = HashMap::from([("BIBINPUTS".to_owned(), format!("{}:", search.join(":")))]);
    let basename = source_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("The file name is invalid.")?;
    let input_path = if engine == LatexEngine::Tectonic {
        fs::canonicalize(source_path)
            .map_err(|error| format!("Could not resolve the LaTeX source file: {error}"))?
            .into_os_string()
            .into_string()
            .map_err(|_| "The file name is invalid.")?
    } else {
        basename.to_owned()
    };
    let args = match engine {
        LatexEngine::LatexMk => vec![
            "-pdf".into(),
            "-interaction=nonstopmode".into(),
            "-file-line-error".into(),
            format!("-outdir={}", build_directory.display()),
            input_path.clone(),
        ],
        LatexEngine::Tectonic => vec![
            "--outdir".into(),
            build_directory.to_string_lossy().into_owned(),
            "--keep-logs".into(),
            input_path.clone(),
        ],
        LatexEngine::PdfLatex => vec![
            "-interaction=nonstopmode".into(),
            "-file-line-error".into(),
            format!("-output-directory={}", build_directory.display()),
            input_path,
        ],
    };
    let bcf_path = build_directory.join(format!(
        "{}.bcf",
        source_path.file_stem().unwrap().to_string_lossy()
    ));
    if engine == LatexEngine::PdfLatex && bcf_path.exists() {
        fs::remove_file(&bcf_path).map_err(|error| error.to_string())?;
    }
    run_process(executable, &args, project_folder, &env)?;
    if engine == LatexEngine::PdfLatex {
        if bcf_path.exists() {
            let biber = find_program("biber").ok_or_else(|| crate::i18n::gettext("This document uses biblatex and needs Biber. Install Biber, latexmk, or Tectonic."))?;
            let job = source_path
                .file_stem()
                .unwrap()
                .to_string_lossy()
                .into_owned();
            run_process(
                &biber,
                &[
                    "--input-directory".into(),
                    build_directory.to_string_lossy().into_owned(),
                    "--output-directory".into(),
                    build_directory.to_string_lossy().into_owned(),
                    job,
                ],
                build_directory,
                &env,
            )?;
            run_process(executable, &args, project_folder, &env)?;
            run_process(executable, &args, project_folder, &env)?;
        } else {
            let aux = fs::read_to_string(build_directory.join(format!(
                "{}.aux",
                source_path.file_stem().unwrap().to_string_lossy()
            )))
            .unwrap_or_default();
            if aux.contains("\\bibdata{") || aux.contains("\\bibdata ") {
                let bibtex = find_program("bibtex").ok_or_else(|| {
                    crate::i18n::gettext(
                        "This document needs BibTeX. Install BibTeX, latexmk, or Tectonic.",
                    )
                })?;
                let job = source_path
                    .file_stem()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                run_process(&bibtex, &[job], build_directory, &env)?;
                run_process(executable, &args, project_folder, &env)?;
                run_process(executable, &args, project_folder, &env)?;
            }
        }
    }
    if !expected_pdf.is_file() {
        return Err(format!("{} finished without creating the expected PDF. Check that the document has a complete LaTeX structure.", engine.executable()));
    }
    Ok(())
}

pub fn external_pdf_viewer(path: &Path) -> Result<(), String> {
    gtk::gio::AppInfo::launch_default_for_uri(
        &gtk::gio::File::for_path(path).uri(),
        None::<&gtk::gio::AppLaunchContext>,
    )
    .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::{compile_error_line, run_process};
    use std::collections::HashMap;
    use std::path::Path;

    #[test]
    fn process_error_keeps_source_line_when_stderr_also_has_output() {
        let args = [
            "-c".to_owned(),
            "printf '%s\\n' '! Undefined control sequence.' 'l.17 \\\\unknown'; printf '%s\\n' 'Fontconfig warning from stderr' >&2; exit 1".to_owned(),
        ];
        let output = run_process(
            Path::new("/bin/sh"),
            &args,
            &std::env::temp_dir(),
            &HashMap::new(),
        )
        .expect_err("the test process exits with an error");

        assert!(output.contains("Fontconfig warning from stderr"));
        assert_eq!(compile_error_line(&output), Some(17));
    }
}
