use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectEntryKind {
    Directory,
    Editable,
    Previewable,
    Resource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectFileKind {
    Latex,
    Bibtex,
    PlainText,
    LatexSupport,
    Image,
    Pdf,
    Unsupported,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TextEncoding {
    #[default]
    Utf8,
    Latin1,
}

pub fn decode_text(bytes: &[u8]) -> (String, TextEncoding) {
    match std::str::from_utf8(bytes) {
        Ok(text) => (text.to_owned(), TextEncoding::Utf8),
        Err(_) => (
            bytes.iter().copied().map(char::from).collect(),
            TextEncoding::Latin1,
        ),
    }
}

pub fn encode_text(text: &str, encoding: TextEncoding) -> Result<Vec<u8>, String> {
    match encoding {
        TextEncoding::Utf8 => Ok(text.as_bytes().to_vec()),
        TextEncoding::Latin1 => text
            .chars()
            .map(|character| {
                u8::try_from(character as u32).map_err(|_| {
                    "This document uses Latin-1 and cannot save characters outside that encoding."
                        .to_owned()
                })
            })
            .collect(),
    }
}

impl ProjectFileKind {
    pub fn is_text_editable(self) -> bool {
        matches!(
            self,
            Self::Latex | Self::Bibtex | Self::PlainText | Self::LatexSupport
        )
    }

    pub fn is_previewable(self) -> bool {
        matches!(self, Self::Image | Self::Pdf)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectEntry {
    pub path: PathBuf,
    pub name: String,
    pub kind: ProjectEntryKind,
    pub depth: usize,
}

pub fn openable_extension(path: &Path) -> bool {
    file_kind(path) != ProjectFileKind::Unsupported
}

pub fn file_kind(path: &Path) -> ProjectFileKind {
    let Some(extension) = path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
    else {
        return ProjectFileKind::Unsupported;
    };

    match extension.as_str() {
        "tex" => ProjectFileKind::Latex,
        "bib" => ProjectFileKind::Bibtex,
        "txt" => ProjectFileKind::PlainText,
        "bst" | "cls" | "sty" => ProjectFileKind::LatexSupport,
        "pdf" => ProjectFileKind::Pdf,
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "tif" | "tiff" | "svg" | "webp" => {
            ProjectFileKind::Image
        }
        _ => ProjectFileKind::Unsupported,
    }
}

pub fn path_contains(parent: &Path, child: &Path) -> bool {
    child == parent || child.starts_with(parent)
}

pub fn rewrite_path_prefix(path: &Path, old_prefix: &Path, new_prefix: &Path) -> PathBuf {
    path.strip_prefix(old_prefix)
        .map(|suffix| new_prefix.join(suffix))
        .unwrap_or_else(|_| path.to_path_buf())
}

pub fn relative_path(base: &Path, destination: &Path) -> Option<PathBuf> {
    let base = base.components().collect::<Vec<_>>();
    let destination = destination.components().collect::<Vec<_>>();
    let common = base
        .iter()
        .zip(&destination)
        .take_while(|(left, right)| left == right)
        .count();
    if common == 0
        && (base.first().is_some_and(|part| {
            matches!(
                part,
                std::path::Component::RootDir | std::path::Component::Prefix(_)
            )
        }) || destination.first().is_some_and(|part| {
            matches!(
                part,
                std::path::Component::RootDir | std::path::Component::Prefix(_)
            )
        }))
    {
        return None;
    }
    if base[common..]
        .iter()
        .any(|part| !matches!(part, std::path::Component::Normal(_)))
        || destination[common..]
            .iter()
            .any(|part| !matches!(part, std::path::Component::Normal(_)))
    {
        return None;
    }
    let mut result = PathBuf::new();
    for _ in &base[common..] {
        result.push("..");
    }
    for part in &destination[common..] {
        result.push(part.as_os_str());
    }
    Some(if result.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        result
    })
}

pub fn entries(root: &Path) -> std::io::Result<Vec<ProjectEntry>> {
    let mut result = Vec::new();
    visit(root, 0, &mut result)?;
    Ok(result)
}

fn visit(folder: &Path, depth: usize, output: &mut Vec<ProjectEntry>) -> std::io::Result<()> {
    let mut children = fs::read_dir(folder)?
        .filter_map(Result::ok)
        .collect::<Vec<_>>();
    children.retain(|entry| !entry.file_name().to_string_lossy().starts_with('.'));
    children.sort_by(|left, right| {
        let left_dir = left.file_type().map(|kind| kind.is_dir()).unwrap_or(false);
        let right_dir = right.file_type().map(|kind| kind.is_dir()).unwrap_or(false);
        right_dir.cmp(&left_dir).then_with(|| {
            left.file_name()
                .to_string_lossy()
                .to_lowercase()
                .cmp(&right.file_name().to_string_lossy().to_lowercase())
        })
    });
    for entry in children {
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            continue;
        }
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if file_type.is_dir() {
            output.push(ProjectEntry {
                path: path.clone(),
                name,
                kind: ProjectEntryKind::Directory,
                depth,
            });
            visit(&path, depth + 1, output)?;
        } else if file_type.is_file() {
            let kind = match file_kind(&path) {
                ProjectFileKind::Latex
                | ProjectFileKind::Bibtex
                | ProjectFileKind::PlainText
                | ProjectFileKind::LatexSupport => ProjectEntryKind::Editable,
                ProjectFileKind::Image | ProjectFileKind::Pdf => ProjectEntryKind::Previewable,
                ProjectFileKind::Unsupported => ProjectEntryKind::Resource,
            };
            output.push(ProjectEntry {
                path,
                name,
                kind,
                depth,
            });
        }
    }
    Ok(())
}

pub fn move_file_within_project(
    source: &Path,
    destination: &Path,
    project_root: &Path,
) -> Result<PathBuf, String> {
    let root = project_root
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let source_type = fs::symlink_metadata(source)
        .map_err(|error| error.to_string())?
        .file_type();
    if !source_type.is_file() && !source_type.is_dir() {
        return Err("Only project files and folders can be moved.".to_owned());
    }
    let source_real = source.canonicalize().map_err(|error| error.to_string())?;
    if !source_real.starts_with(&root) || source_real == root {
        return Err("The item is outside the project folder.".to_owned());
    }
    let destination_real = destination
        .canonicalize()
        .map_err(|error| error.to_string())?;
    if !destination_real.starts_with(&root) {
        return Err("The destination is outside the project folder.".to_owned());
    }
    if !destination_real.is_dir() {
        return Err("Choose a project folder as the destination.".to_owned());
    }
    if source_real == destination_real {
        return Ok(source_real);
    }
    if source_type.is_dir() && destination_real.starts_with(&source_real) {
        return Err("A folder cannot be moved into itself or one of its subfolders.".to_owned());
    }
    if source_real.parent() == Some(destination_real.as_path()) {
        return Ok(source_real);
    }
    let target = destination_real.join(source.file_name().ok_or("The file name is invalid.")?);
    if target.exists() {
        return Err("An item with that name already exists.".to_owned());
    }
    fs::rename(&source_real, &target).map_err(|error| error.to_string())?;
    Ok(target)
}

pub fn rename_project_file(
    source: &Path,
    new_name: &str,
    project_root: &Path,
) -> Result<PathBuf, String> {
    if new_name.trim().is_empty()
        || new_name.trim() != new_name
        || new_name == "."
        || new_name == ".."
        || new_name.chars().any(|ch| ch == '/' || ch == '\\')
        || new_name.chars().any(char::is_control)
    {
        return Err("Enter a valid file name.".to_owned());
    }
    let root = project_root
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let source_real = source.canonicalize().map_err(|error| error.to_string())?;
    if !source_real.starts_with(&root) || source_real == root {
        return Err("The file is outside the project folder.".to_owned());
    }
    let target = source_real
        .parent()
        .ok_or("The file has no parent folder.")?
        .join(new_name);
    if target.exists() {
        return Err("A file with that name already exists.".to_owned());
    }
    fs::rename(source_real, &target).map_err(|error| error.to_string())?;
    Ok(target)
}

pub fn find_tex_files(root: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut files = entries(root)?
        .into_iter()
        .filter(|entry| {
            entry.kind == ProjectEntryKind::Editable
                && entry
                    .path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("tex"))
        })
        .map(|entry| entry.path)
        .collect::<Vec<_>>();
    files.sort_by_key(|path| {
        path.strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .to_lowercase()
    });
    Ok(files)
}
