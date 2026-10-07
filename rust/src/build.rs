use crate::bibtex::{parse_bibtex, serialize_bibtex, BibEntry, Bibliography};
use crate::storage::xdg_cache_home;
use gtk::gio::prelude::FileExt;
use regex::Regex;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

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
pub struct TemporaryBibliography {
    pub path: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BibliographyPreparation {
    pub files: Vec<TemporaryBibliography>,
    pub added: usize,
    pub conflicts: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildResult {
    pub pdf_path: PathBuf,
    pub engine: LatexEngine,
    pub added_references: usize,
    pub conflicts: usize,
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
    let with_extension = if normalized.to_ascii_lowercase().ends_with(".bib") {
        normalized
    } else {
        format!("{normalized}.bib")
    };
    Some(with_extension)
}

fn bib_resources(source: &str) -> Vec<String> {
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

fn without_zotero_fields(entry: &BibEntry) -> BibEntry {
    let mut clean = entry.clone();
    clean
        .fields
        .retain(|name, _| name != "zotero_key" && name != "zotero_version");
    clean
        .raw_fields
        .retain(|name, _| name != "zotero_key" && name != "zotero_version");
    clean
}

fn comparable_fields(entry: &BibEntry) -> BTreeMap<&str, &str> {
    entry
        .fields
        .iter()
        .filter(|(name, _)| name.as_str() != "zotero_key" && name.as_str() != "zotero_version")
        .map(|(name, value)| (name.as_str(), value.as_str()))
        .collect()
}

fn string_key(directive: &str) -> Option<String> {
    let re = Regex::new(r"(?i)^\s*@string\s*[({]\s*([^=\s,]+)").unwrap();
    re.captures(directive)
        .map(|capture| capture[1].to_ascii_lowercase())
}

pub fn prepare_project_bibliography(
    source: &str,
    project_folder: &Path,
    local: &Bibliography,
) -> Result<BibliographyPreparation, String> {
    if local.entries.is_empty() && local.directives.is_empty() {
        return Ok(BibliographyPreparation {
            files: Vec::new(),
            added: 0,
            conflicts: 0,
        });
    }
    let mut seen = HashSet::new();
    let mut output = Vec::new();
    let mut added = 0;
    let mut conflicts = 0;
    for resource in bib_resources(source) {
        if !seen.insert(resource.clone()) {
            continue;
        }
        let relative = Path::new(&resource);
        if relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
        {
            continue;
        }
        let path = project_folder.join(relative);
        let existing = match fs::read_to_string(&path) {
            Ok(text) => parse_bibtex(&text)
                .map_err(|error| format!("Could not read {}: {error}", resource))?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Bibliography {
                entries: Vec::new(),
                directives: Vec::new(),
            },
            Err(error) => return Err(format!("Could not read {}: {error}", resource)),
        };
        let mut project_entries = existing.entries;
        let mut key_index = project_entries
            .iter()
            .enumerate()
            .map(|(index, entry)| (entry.key.to_ascii_lowercase(), index))
            .collect::<HashMap<_, _>>();
        for local_entry in &local.entries {
            if let Some(index) = key_index.get(&local_entry.key.to_ascii_lowercase()) {
                let existing_entry = &project_entries[*index];
                if existing_entry.entry_type != local_entry.entry_type
                    || comparable_fields(existing_entry) != comparable_fields(local_entry)
                {
                    conflicts += 1;
                }
                continue;
            }
            let clean = without_zotero_fields(local_entry);
            key_index.insert(clean.key.to_ascii_lowercase(), project_entries.len());
            project_entries.push(clean);
            added += 1;
        }
        let mut directives = existing.directives;
        let mut string_keys = directives
            .iter()
            .filter_map(|line| string_key(line))
            .collect::<HashSet<_>>();
        for directive in &local.directives {
            let key = string_key(directive);
            if key.as_ref().is_some_and(|key| string_keys.contains(key)) {
                continue;
            }
            if !directives.contains(directive) {
                directives.push(directive.clone());
            }
            if let Some(key) = key {
                string_keys.insert(key);
            }
        }
        let updated = Bibliography {
            entries: project_entries,
            directives,
        };
        let original_content = fs::read_to_string(&path).unwrap_or_default();
        let content = serialize_bibtex(&updated);
        if content != original_content {
            output.push(TemporaryBibliography {
                path: resource,
                content,
            });
        }
    }
    Ok(BibliographyPreparation {
        files: output,
        added,
        conflicts,
    })
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
        let message = if error.is_empty() { output } else { error };
        Err(String::from_utf8_lossy(&message).trim().to_owned())
    }
}

fn write_temporary_bibliographies(
    directory: &Path,
    bibliographies: &[TemporaryBibliography],
) -> Result<(), String> {
    for bibliography in bibliographies {
        let relative = Path::new(&bibliography.path);
        if relative.is_absolute()
            || bibliography.path.starts_with('~')
            || relative.components().any(|part| {
                matches!(
                    part,
                    Component::ParentDir | Component::RootDir | Component::Prefix(_)
                )
            })
        {
            return Err("A bibliography path must be relative to the project.".to_owned());
        }
        let path = directory.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        fs::write(path, bibliography.content.as_bytes()).map_err(|error| error.to_string())?;
    }
    Ok(())
}

pub fn compile_latex(
    source_path: &Path,
    source: &str,
    local_library: &Bibliography,
) -> Result<BuildResult, String> {
    let (engine, executable) = find_latex_engine().ok_or_else(|| {
        crate::i18n::gettext("No LaTeX compiler was found. Install latexmk, Tectonic, or TeX Live.")
    })?;
    if !source_path.is_file() {
        return Err("The document must be available as a local file.".to_owned());
    }
    let project_folder = source_path
        .parent()
        .ok_or("The document has no parent folder.")?;
    let prepared = prepare_project_bibliography(source, project_folder, local_library)?;
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
    let mut temporary_directory = None;
    if !prepared.files.is_empty() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let path = build_directory.join(format!(".ovenbird-bib-{}-{stamp}", std::process::id()));
        fs::create_dir(&path).map_err(|error| error.to_string())?;
        if let Err(error) = write_temporary_bibliographies(&path, &prepared.files) {
            let _ = fs::remove_dir_all(&path);
            return Err(error);
        }
        temporary_directory = Some(path);
    }
    let result = compile_with_engine(
        engine,
        &executable,
        source_path,
        &build_directory,
        &expected_pdf,
        temporary_directory.as_deref(),
        project_folder,
    );
    if let Some(path) = temporary_directory {
        let _ = fs::remove_dir_all(path);
    }
    result?;
    Ok(BuildResult {
        pdf_path: expected_pdf,
        engine,
        added_references: prepared.added,
        conflicts: prepared.conflicts,
    })
}

fn compile_with_engine(
    engine: LatexEngine,
    executable: &Path,
    source_path: &Path,
    build_directory: &Path,
    expected_pdf: &Path,
    temporary_directory: Option<&Path>,
    project_folder: &Path,
) -> Result<(), String> {
    let existing = std::env::var("BIBINPUTS").unwrap_or_default();
    let mut search = Vec::new();
    if let Some(path) = temporary_directory {
        search.push(path.to_string_lossy().into_owned());
    }
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
