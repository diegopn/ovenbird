use crate::bibtex::{parse_bibtex, serialize_bibtex, BibEntry, Bibliography};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TEMP_WRITE_ID: AtomicU64 = AtomicU64::new(0);

pub fn xdg_data_home() -> PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
        .unwrap_or_else(|| PathBuf::from("/app/data"))
}

pub fn xdg_config_home() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .unwrap_or_else(|| PathBuf::from("/app/config"))
}

pub fn xdg_cache_home() -> PathBuf {
    std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))
        .unwrap_or_else(|| PathBuf::from("/app/cache"))
}

#[derive(Debug, Clone)]
pub struct LocalLibrary {
    pub directory: PathBuf,
    pub path: PathBuf,
    pub bibliography: Bibliography,
}

impl LocalLibrary {
    pub fn empty() -> Self {
        let directory = xdg_data_home().join("ovenbird");
        let path = directory.join("library.bib");
        Self {
            directory,
            path,
            bibliography: Bibliography {
                entries: Vec::new(),
                directives: Vec::new(),
            },
        }
    }

    pub fn open() -> io::Result<Self> {
        let directory = xdg_data_home().join("ovenbird");
        fs::create_dir_all(&directory)?;
        let path = directory.join("library.bib");
        let bibliography = match fs::read_to_string(&path) {
            Ok(source) => parse_bibtex(&source)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?,
            Err(error) if error.kind() == io::ErrorKind::NotFound => Bibliography {
                entries: Vec::new(),
                directives: Vec::new(),
            },
            Err(error) => return Err(error),
        };
        Ok(Self {
            directory,
            path,
            bibliography,
        })
    }

    pub fn save(&self) -> io::Result<()> {
        atomic_write(&self.path, serialize_bibtex(&self.bibliography).as_bytes())
    }

    pub fn add_in_memory(&mut self, entry: BibEntry) -> Result<(), String> {
        if self
            .bibliography
            .entries
            .iter()
            .any(|item| item.key.eq_ignore_ascii_case(&entry.key))
        {
            return Err(format!("Citation key “{}” already exists.", entry.key));
        }
        self.bibliography.entries.push(entry);
        Ok(())
    }

    pub fn update_in_memory(
        &mut self,
        previous_key: &str,
        mut updated: BibEntry,
    ) -> Result<(), String> {
        if self
            .bibliography
            .entries
            .iter()
            .any(|item| item.key.eq_ignore_ascii_case(&updated.key) && item.key != previous_key)
        {
            return Err(format!("Citation key “{}” already exists.", updated.key));
        }
        let Some(index) = self
            .bibliography
            .entries
            .iter()
            .position(|item| item.key == previous_key)
        else {
            return Err("The reference no longer exists.".to_owned());
        };
        let previous = &self.bibliography.entries[index];
        updated.raw_fields = updated
            .fields
            .iter()
            .filter_map(|(name, value)| {
                (previous.fields.get(name) == Some(value))
                    .then(|| {
                        previous
                            .raw_fields
                            .get(name)
                            .map(|raw| (name.clone(), raw.clone()))
                    })
                    .flatten()
            })
            .collect();
        self.bibliography.entries[index] = updated;
        Ok(())
    }

    pub fn remove_in_memory(&mut self, key: &str) -> bool {
        let previous_len = self.bibliography.entries.len();
        self.bibliography.entries.retain(|entry| entry.key != key);
        previous_len != self.bibliography.entries.len()
    }

    pub fn merge_import(&mut self, imported: Bibliography) -> usize {
        for directive in imported.directives {
            if !self.bibliography.directives.contains(&directive) {
                self.bibliography.directives.push(directive);
            }
        }
        let mut count = 0;
        for entry in imported.entries {
            if !self
                .bibliography
                .entries
                .iter()
                .any(|existing| existing.key.eq_ignore_ascii_case(&entry.key))
            {
                self.bibliography.entries.push(entry);
                count += 1;
            }
        }
        count
    }

    pub fn find(&self, key: &str) -> Option<&BibEntry> {
        self.bibliography
            .entries
            .iter()
            .find(|entry| entry.key == key)
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ZoteroSettings {
    #[serde(default)]
    pub user_id: String,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

impl ZoteroSettings {
    pub fn path() -> PathBuf {
        xdg_config_home().join("ovenbird/settings.json")
    }

    pub fn load() -> io::Result<Self> {
        match fs::read(Self::path()) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(io::Error::other),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(error),
        }
    }

    pub fn save(&self) -> io::Result<()> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(parent, fs::Permissions::from_mode(0o700))?;
            }
        }
        let mut json = serde_json::to_vec_pretty(self).map_err(io::Error::other)?;
        json.push(b'\n');
        atomic_write(&path, &json)
    }
}

pub fn atomic_write(path: &Path, data: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("File has no parent folder"))?;
    fs::create_dir_all(parent)?;
    let write_id = NEXT_TEMP_WRITE_ID.fetch_add(1, Ordering::Relaxed);
    let temporary = parent.join(format!(
        ".{}.tmp-{}-{write_id}",
        path.file_name().and_then(|n| n.to_str()).unwrap_or("file"),
        std::process::id(),
    ));
    fs::write(&temporary, data)?;
    if let Err(error) = fs::rename(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    Ok(())
}
