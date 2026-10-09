use std::path::{Component, Path};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver};

static NEXT_STAGING_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArchiveTool {
    Unzip,
    Bsdtar,
}

impl ArchiveTool {
    fn program(self) -> &'static str {
        match self {
            Self::Unzip => "unzip",
            Self::Bsdtar => "bsdtar",
        }
    }

    fn list(self, archive: &Path) -> Result<String, String> {
        let output = match self {
            Self::Unzip => Command::new(self.program())
                .arg("-Z1")
                .arg(archive)
                .output(),
            Self::Bsdtar => Command::new(self.program())
                .args(["-tf"])
                .arg(archive)
                .output(),
        }
        .map_err(|error| error.to_string())?;
        if !output.status.success() {
            return Err("The template package could not be processed.".to_owned());
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }

    fn extract(self, archive: &Path, destination: &Path) -> Result<(), String> {
        let status = match self {
            Self::Unzip => Command::new(self.program())
                .args(["-q", "-o"])
                .arg(archive)
                .arg("-d")
                .arg(destination)
                .status(),
            Self::Bsdtar => Command::new(self.program())
                .args(["-xf"])
                .arg(archive)
                .arg("-C")
                .arg(destination)
                .status(),
        }
        .map_err(|error| error.to_string())?;
        if !status.success() {
            return Err("The template package could not be processed.".to_owned());
        }
        Ok(())
    }
}

fn command_exists(program: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|path| {
        std::env::split_paths(&path).any(|directory| {
            let candidate = directory.join(program);
            candidate.is_file() && {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    std::fs::metadata(candidate)
                        .is_ok_and(|metadata| metadata.permissions().mode() & 0o111 != 0)
                }
                #[cfg(not(unix))]
                {
                    true
                }
            }
        })
    })
}

fn available_archive_tool() -> Result<ArchiveTool, String> {
    if command_exists("unzip") {
        Ok(ArchiveTool::Unzip)
    } else if command_exists("bsdtar") {
        Ok(ArchiveTool::Bsdtar)
    } else {
        Err("The app runtime has no ZIP extraction utility.".to_owned())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Template {
    pub id: &'static str,
    pub archive: &'static str,
    pub main_tex: &'static str,
    pub overlay: &'static str,
}

pub const TEMPLATES: &[Template] = &[
    Template {
        id: "abntex2",
        archive: "abntex2.zip",
        main_tex: "main.tex",
        overlay: "abntex2",
    },
    Template {
        id: "ieeetran",
        archive: "ieeetran.zip",
        main_tex: "main.tex",
        overlay: "ieeetran",
    },
    Template {
        id: "acmart",
        archive: "acmart.zip",
        main_tex: "main.tex",
        overlay: "acmart",
    },
    Template {
        id: "elsarticle",
        archive: "elsarticle.zip",
        main_tex: "main.tex",
        overlay: "elsarticle",
    },
    Template {
        id: "springer-nature",
        archive: "springer-nature.zip",
        main_tex: "main.tex",
        overlay: "springer-nature",
    },
];

pub fn resource_root(installed: Option<&Path>, manifest_dir: &Path) -> std::path::PathBuf {
    installed
        .filter(|path| path.is_dir())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| manifest_dir.join("src/templates"))
}

pub fn create_project_async(
    template: Template,
    parent: std::path::PathBuf,
    name: String,
    resource_root: std::path::PathBuf,
) -> Receiver<Result<(std::path::PathBuf, std::path::PathBuf), String>> {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = sender.send(create_project(template, &parent, &name, &resource_root));
    });
    receiver
}

pub fn valid_project_name(name: &str) -> bool {
    !name.trim().is_empty()
        && name.trim() == name
        && name != "."
        && name != ".."
        && !name
            .chars()
            .any(|ch| ch == '/' || ch == '\\' || ch.is_control())
}

pub fn safe_archive_listing(listing: &str) -> bool {
    let entries = listing
        .lines()
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    !entries.is_empty()
        && entries.iter().all(|entry| {
            if entry.starts_with('/')
                || entry.starts_with('\\')
                || entry.chars().any(char::is_control)
            {
                return false;
            }
            if entry.as_bytes().get(1) == Some(&b':') {
                return false;
            }
            let path = entry.strip_suffix('/').unwrap_or(entry);
            !path.is_empty()
                && path
                    .split('/')
                    .all(|part| !part.is_empty() && part != "." && part != "..")
                && Path::new(path)
                    .components()
                    .all(|part| matches!(part, Component::Normal(_)))
        })
}

pub fn create_project(
    template: Template,
    parent: &Path,
    name: &str,
    resource_root: &Path,
) -> Result<(std::path::PathBuf, std::path::PathBuf), String> {
    create_project_with_tool(
        template,
        parent,
        name,
        resource_root,
        available_archive_tool()?,
    )
}

fn create_project_with_tool(
    template: Template,
    parent: &Path,
    name: &str,
    resource_root: &Path,
    archive_tool: ArchiveTool,
) -> Result<(std::path::PathBuf, std::path::PathBuf), String> {
    if !valid_project_name(name) {
        return Err("Enter a valid project folder name".to_owned());
    }
    if !parent.is_dir() {
        return Err("Choose a local folder for the new project.".to_owned());
    }
    let archive = resource_root.join("vendor").join(template.archive);
    if !archive.is_file() {
        return Err("The bundled template package is missing.".to_owned());
    }
    let project = parent.join(name);
    if project.exists() {
        return Err("A folder with that name already exists".to_owned());
    }
    let staging_id = NEXT_STAGING_ID.fetch_add(1, Ordering::Relaxed);
    let staging = parent.join(format!(
        ".ovenbird-template-{}-{staging_id}",
        std::process::id()
    ));
    std::fs::create_dir(&staging).map_err(|error| error.to_string())?;
    let result = (|| {
        let listing = archive_tool.list(&archive)?;
        if !safe_archive_listing(&listing) {
            return Err("The template archive contains unsafe file paths.".to_owned());
        }
        let extraction = staging.join("extracted");
        std::fs::create_dir(&extraction).map_err(|error| error.to_string())?;
        archive_tool.extract(&archive, &extraction)?;
        let mut content = extraction.clone();
        let mut children = std::fs::read_dir(&extraction)
            .map_err(|error| error.to_string())?
            .collect::<std::io::Result<Vec<_>>>()
            .map_err(|error| error.to_string())?;
        if children.len() == 1 && children[0].path().is_dir() {
            content = children.remove(0).path();
        }
        copy_overlay(
            &resource_root.join("overlays").join(template.overlay),
            &content,
        )?;
        let main = content.join(template.main_tex);
        if !main.is_file() {
            return Err("The bundled template does not contain its entry document.".to_owned());
        }
        std::fs::rename(content, &project).map_err(|error| error.to_string())?;
        Ok((project.clone(), project.join(template.main_tex)))
    })();
    let _ = std::fs::remove_dir_all(&staging);
    result
}

fn copy_overlay(overlay: &Path, destination: &Path) -> Result<(), String> {
    for entry in std::fs::read_dir(overlay).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        if entry
            .file_type()
            .map_err(|error| error.to_string())?
            .is_file()
        {
            std::fs::copy(entry.path(), destination.join(entry.file_name()))
                .map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{create_project_async, resource_root, TEMPLATES};
    use std::path::{Path, PathBuf};
    use std::time::Duration;

    fn test_root(name: &str) -> PathBuf {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let target = std::env::var_os("CARGO_TARGET_DIR")
            .map(PathBuf::from)
            .map(|path| {
                if path.is_absolute() {
                    path
                } else {
                    manifest.join(path)
                }
            });
        let build_root = target
            .and_then(|path| path.parent().map(Path::to_path_buf))
            .unwrap_or_else(|| manifest.join("rust-build"));
        build_root
            .join("test-tmp")
            .join(format!("{name}-{}", std::process::id()))
    }

    #[test]
    fn falls_back_to_source_templates_when_installed_directory_is_missing() {
        let root = test_root("template-path-test");
        let source = root.join("checkout");
        let missing = root.join("installed-templates");
        std::fs::create_dir_all(&source).unwrap();

        assert_eq!(
            resource_root(Some(&missing), &source),
            source.join("src/templates")
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn creates_ieee_project_with_the_runtime_bsdtar_fallback() {
        if !super::command_exists("bsdtar") {
            return;
        }
        let root = test_root("bsdtar-template-test");
        let parent = root.join("projects");
        let templates = std::env::current_dir().unwrap().join("src/templates");
        std::fs::create_dir_all(&parent).unwrap();

        let ieee = TEMPLATES
            .iter()
            .find(|template| template.id == "ieeetran")
            .unwrap();
        let (project, main_tex) = super::create_project_with_tool(
            *ieee,
            &parent,
            "ieee-project",
            &templates,
            super::ArchiveTool::Bsdtar,
        )
        .unwrap();

        assert!(main_tex.is_file());
        assert!(project.join("IEEEtran.cls").is_file());
        assert!(project.join("IEEEtran.bst").is_file());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn creates_each_bundled_project_through_a_worker_thread() {
        let root = test_root("template-project-test");
        let parent = root.join("projects");
        let templates = std::env::current_dir().unwrap().join("src/templates");
        std::fs::create_dir_all(&parent).unwrap();

        for template in TEMPLATES {
            let name = format!("{}-project", template.id);
            let result =
                create_project_async(*template, parent.clone(), name.clone(), templates.clone())
                    .recv_timeout(Duration::from_secs(45))
                    .unwrap()
                    .unwrap();
            assert_eq!(result.0, parent.join(&name));
            assert!(
                result.1.is_file(),
                "{} must contain its entry TeX file",
                template.id
            );
            let (class_file, bibliography_style) = match template.id {
                "abntex2" => ("abntex2.cls", "abntex2-alf.bst"),
                "ieeetran" => ("IEEEtran.cls", "IEEEtran.bst"),
                "acmart" => ("acmart.cls", "ACM-Reference-Format.bst"),
                "elsarticle" => ("elsarticle.cls", "elsarticle-num.bst"),
                "springer-nature" => ("sn-jnl.cls", "sn-mathphys-num.bst"),
                _ => panic!("unexpected bundled template: {}", template.id),
            };
            assert!(
                result.0.join(class_file).is_file(),
                "{} must include its document class",
                template.id
            );
            assert!(
                result.0.join(bibliography_style).is_file(),
                "{} must include its bibliography style",
                template.id
            );
            let tex_files = crate::project::find_tex_files(&result.0).unwrap();
            assert!(
                tex_files.contains(&result.1),
                "{} entry file must appear in recursive TeX navigation",
                template.id
            );
            if template.id != "springer-nature" {
                assert!(
                    result.0.join("references.bib").is_file(),
                    "{} must include its example bibliography",
                    template.id
                );
            } else {
                assert!(
                    result.0.join("sn-bibliography.bib").is_file(),
                    "Springer Nature must retain its example bibliography"
                );
            }
        }

        std::fs::remove_dir_all(root).unwrap();
    }
}
