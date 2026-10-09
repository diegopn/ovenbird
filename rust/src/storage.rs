use crate::bibtex::{parse_bibtex, serialize_bibtex, BibEntry, Bibliography};
use serde::{Deserialize, Serialize};
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorProfile {
    #[serde(default)]
    pub full_name: String,
    #[serde(default)]
    pub citation_name: String,
    #[serde(default)]
    pub orcid: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub institution: String,
}

impl AuthorProfile {
    pub fn from_citation_name(citation_name: &str) -> Self {
        let citation_name = citation_name.trim();
        let display_name = crate::bibtex::display_bibtex_text(citation_name);
        let full_name = if citation_name.starts_with('{') && citation_name.ends_with('}') {
            display_name
        } else if let Some((family, given)) = display_name.split_once(',') {
            format!("{} {}", given.trim(), family.trim())
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        } else {
            display_name
        };
        Self {
            full_name,
            citation_name: citation_name.to_owned(),
            orcid: String::new(),
            email: String::new(),
            institution: String::new(),
        }
    }

    pub fn from_full_name(full_name: &str) -> Self {
        let full_name = full_name.split_whitespace().collect::<Vec<_>>().join(" ");
        let parts = full_name.split_whitespace().collect::<Vec<_>>();
        let citation_name = if parts.len() < 2 {
            full_name.clone()
        } else {
            let mut family_start = parts.len() - 1;
            while family_start > 1 && is_surname_particle(parts[family_start - 1]) {
                family_start -= 1;
            }
            let given = parts[..family_start].join(" ");
            let family = parts[family_start..].join(" ");
            format!("{family}, {given}")
        };
        Self {
            full_name,
            citation_name,
            orcid: String::new(),
            email: String::new(),
            institution: String::new(),
        }
    }
}

fn is_surname_particle(word: &str) -> bool {
    matches!(
        word.to_lowercase().as_str(),
        "da" | "das"
            | "de"
            | "del"
            | "della"
            | "den"
            | "der"
            | "di"
            | "do"
            | "dos"
            | "du"
            | "la"
            | "le"
            | "ten"
            | "ter"
            | "van"
            | "von"
    )
}

#[derive(Debug, Clone)]
pub struct LocalLibrary {
    pub directory: PathBuf,
    pub path: PathBuf,
    pub bibliography: Bibliography,
    pub tags: Vec<String>,
    pub authors: Vec<AuthorProfile>,
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
            tags: Vec::new(),
            authors: Vec::new(),
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
        let tags_path = directory.join("tags.json");
        let mut tags = match fs::read(&tags_path) {
            Ok(bytes) => serde_json::from_slice::<Vec<String>>(&bytes).map_err(io::Error::other)?,
            Err(error) if error.kind() == io::ErrorKind::NotFound => Vec::new(),
            Err(error) => return Err(error),
        };
        for entry in &bibliography.entries {
            tags.extend(split_tags(entry.get("tags")));
        }
        tags = unique_tags(tags);
        let authors_path = directory.join("authors.json");
        let author_names = match fs::read(&authors_path) {
            Ok(bytes) => serde_json::from_slice::<Vec<String>>(&bytes).map_err(io::Error::other)?,
            Err(error) if error.kind() == io::ErrorKind::NotFound => Vec::new(),
            Err(error) => return Err(error),
        };
        let profiles_path = directory.join("author_profiles.json");
        let profiles = match fs::read(&profiles_path) {
            Ok(bytes) => {
                serde_json::from_slice::<Vec<AuthorProfile>>(&bytes).map_err(io::Error::other)?
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => Vec::new(),
            Err(error) => return Err(error),
        };
        let authors = merge_author_profiles(author_names, profiles, &bibliography);
        Ok(Self {
            directory,
            path,
            bibliography,
            tags,
            authors,
        })
    }

    pub fn save(&self) -> io::Result<()> {
        let mut tags = self.tags.clone();
        for entry in &self.bibliography.entries {
            tags.extend(split_tags(entry.get("tags")));
        }
        let mut encoded_tags =
            serde_json::to_vec_pretty(&unique_tags(tags)).map_err(io::Error::other)?;
        encoded_tags.push(b'\n');
        atomic_write(&self.directory.join("tags.json"), &encoded_tags)?;
        let mut authors = self
            .authors
            .iter()
            .map(|author| author.citation_name.clone())
            .collect::<Vec<_>>();
        for entry in &self.bibliography.entries {
            authors.extend(crate::bibtex::split_bibtex_names(entry.get("author")));
        }
        let mut encoded_authors =
            serde_json::to_vec_pretty(&unique_authors(authors)).map_err(io::Error::other)?;
        encoded_authors.push(b'\n');
        atomic_write(&self.directory.join("authors.json"), &encoded_authors)?;
        let mut encoded_profiles =
            serde_json::to_vec_pretty(&self.authors).map_err(io::Error::other)?;
        encoded_profiles.push(b'\n');
        atomic_write(
            &self.directory.join("author_profiles.json"),
            &encoded_profiles,
        )?;
        atomic_write(&self.path, serialize_bibtex(&self.bibliography).as_bytes())
    }

    pub fn add_author(&mut self, name: &str) -> Result<(), String> {
        self.add_author_profile(AuthorProfile::from_citation_name(name))
    }

    pub fn add_author_profile(&mut self, profile: AuthorProfile) -> Result<(), String> {
        let profile = normalize_author_profile(profile);
        if profile.full_name.is_empty() || profile.citation_name.is_empty() {
            return Err("An author name is required.".to_owned());
        }
        if profile.citation_name.eq_ignore_ascii_case("others") {
            return Err("The BibTeX marker “others” cannot be used as an author name.".to_owned());
        }
        let identity = author_identity(&profile.citation_name);
        if self
            .authors
            .iter()
            .any(|author| author_identity(&author.citation_name) == identity)
        {
            return Err("That author already exists.".to_owned());
        }
        self.authors.push(profile);
        self.authors = unique_author_profiles(std::mem::take(&mut self.authors));
        Ok(())
    }

    pub fn rename_author(&mut self, previous: &str, updated: &str) -> Result<(), String> {
        let mut profile = self
            .authors
            .iter()
            .find(|author| author_identity(&author.citation_name) == author_identity(previous))
            .cloned()
            .ok_or_else(|| "The author no longer exists.".to_owned())?;
        let updated_profile = AuthorProfile::from_citation_name(updated);
        profile.citation_name = updated.trim().to_owned();
        profile.full_name = updated_profile.full_name;
        self.update_author_profile(previous, profile)
    }

    pub fn update_author_profile(
        &mut self,
        previous: &str,
        updated: AuthorProfile,
    ) -> Result<(), String> {
        let updated = normalize_author_profile(updated);
        if updated.full_name.is_empty() || updated.citation_name.is_empty() {
            return Err("An author name is required.".to_owned());
        }
        if updated.citation_name.eq_ignore_ascii_case("others") {
            return Err("The BibTeX marker “others” cannot be used as an author name.".to_owned());
        }
        let previous_identity = author_identity(previous);
        let Some(index) = self
            .authors
            .iter()
            .position(|author| author_identity(&author.citation_name) == previous_identity)
        else {
            return Err("The author no longer exists.".to_owned());
        };
        let updated_identity = author_identity(&updated.citation_name);
        if self
            .authors
            .iter()
            .enumerate()
            .any(|(other_index, author)| {
                other_index != index && author_identity(&author.citation_name) == updated_identity
            })
        {
            return Err("That author already exists.".to_owned());
        }

        self.authors[index] = updated.clone();
        for entry in &mut self.bibliography.entries {
            let mut names = crate::bibtex::split_bibtex_names(entry.get("author"));
            let mut changed = false;
            for name in &mut names {
                if author_identity(name) == previous_identity {
                    *name = replacement_author_name(name, &updated.citation_name);
                    changed = true;
                }
            }
            if changed {
                set_entry_authors(entry, names);
            }
        }
        self.sync_authors_from_references();
        Ok(())
    }

    pub fn references_for_author(&self, name: &str) -> Vec<BibEntry> {
        let name = author_identity(name);
        self.bibliography
            .entries
            .iter()
            .filter(|entry| {
                crate::bibtex::split_bibtex_names(entry.get("author"))
                    .iter()
                    .any(|author| author_identity(author) == name)
            })
            .cloned()
            .collect()
    }

    pub fn remove_author(&mut self, name: &str, delete_references: bool) -> Result<usize, String> {
        let name = author_identity(name);
        if !self
            .authors
            .iter()
            .any(|author| author_identity(&author.citation_name) == name)
        {
            return Err("The author no longer exists.".to_owned());
        }
        let reference_count = self
            .bibliography
            .entries
            .iter()
            .filter(|entry| {
                crate::bibtex::split_bibtex_names(entry.get("author"))
                    .iter()
                    .any(|author| author_identity(author) == name)
            })
            .count();
        if reference_count > 0 && !delete_references {
            return Err("This author is associated with references.".to_owned());
        }
        if reference_count > 0 {
            self.bibliography.entries.retain(|entry| {
                !crate::bibtex::split_bibtex_names(entry.get("author"))
                    .iter()
                    .any(|author| author_identity(author) == name)
            });
        }
        self.authors
            .retain(|author| author_identity(&author.citation_name) != name);
        Ok(reference_count)
    }

    pub fn add_tag(&mut self, name: &str) -> Result<(), String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("Enter a tag name.".to_owned());
        }
        if self.tags.iter().any(|tag| tag.eq_ignore_ascii_case(name)) {
            return Err("That tag already exists.".to_owned());
        }
        self.tags.push(name.to_owned());
        self.sync_tags_from_references();
        Ok(())
    }

    pub fn rename_tag(&mut self, previous: &str, updated: &str) -> Result<(), String> {
        let updated = updated.trim();
        if updated.is_empty() {
            return Err("Enter a tag name.".to_owned());
        }
        let Some(index) = self
            .tags
            .iter()
            .position(|tag| tag.eq_ignore_ascii_case(previous))
        else {
            return Err("The tag no longer exists.".to_owned());
        };
        if self
            .tags
            .iter()
            .enumerate()
            .any(|(other_index, tag)| other_index != index && tag.eq_ignore_ascii_case(updated))
        {
            return Err("That tag already exists.".to_owned());
        }
        let previous = self.tags[index].clone();
        self.tags[index] = updated.to_owned();
        for entry in &mut self.bibliography.entries {
            let renamed = split_tags(entry.get("tags"))
                .into_iter()
                .map(|tag| {
                    if tag.eq_ignore_ascii_case(&previous) {
                        updated.to_owned()
                    } else {
                        tag
                    }
                })
                .collect::<Vec<_>>();
            set_entry_tags(entry, renamed);
        }
        self.sync_tags_from_references();
        Ok(())
    }

    pub fn remove_tag(&mut self, name: &str) -> bool {
        let previous_len = self.tags.len();
        self.tags.retain(|tag| !tag.eq_ignore_ascii_case(name));
        if previous_len == self.tags.len() {
            return false;
        }
        for entry in &mut self.bibliography.entries {
            let remaining = split_tags(entry.get("tags"))
                .into_iter()
                .filter(|tag| !tag.eq_ignore_ascii_case(name))
                .collect::<Vec<_>>();
            set_entry_tags(entry, remaining);
        }
        self.sync_tags_from_references();
        true
    }

    fn sync_tags_from_references(&mut self) {
        let mut tags = self.tags.clone();
        for entry in &self.bibliography.entries {
            tags.extend(split_tags(entry.get("tags")));
        }
        self.tags = unique_tags(tags);
    }

    fn sync_authors_from_references(&mut self) {
        let names = self
            .authors
            .iter()
            .map(|author| author.citation_name.clone())
            .collect();
        self.authors = merge_author_profiles(names, self.authors.clone(), &self.bibliography);
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
        self.sync_tags_from_references();
        self.sync_authors_from_references();
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
        self.sync_tags_from_references();
        self.sync_authors_from_references();
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
        self.sync_tags_from_references();
        self.sync_authors_from_references();
        count
    }

    pub fn find(&self, key: &str) -> Option<&BibEntry> {
        self.bibliography
            .entries
            .iter()
            .find(|entry| entry.key == key)
    }
}

fn split_tags(value: &str) -> Vec<String> {
    value
        .split([',', ';'])
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .map(str::to_owned)
        .collect()
}

fn unique_tags(tags: Vec<String>) -> Vec<String> {
    let mut unique = Vec::new();
    for tag in tags.into_iter().map(|tag| tag.trim().to_owned()) {
        if !tag.is_empty()
            && !unique
                .iter()
                .any(|existing: &String| existing.eq_ignore_ascii_case(&tag))
        {
            unique.push(tag);
        }
    }
    unique.sort_by_key(|tag| tag.to_lowercase());
    unique
}

fn set_entry_tags(entry: &mut BibEntry, tags: Vec<String>) {
    let tags = unique_tags(tags).join(", ");
    if tags.is_empty() {
        entry.fields.shift_remove("tags");
        entry.raw_fields.shift_remove("tags");
    } else {
        entry.raw_fields.shift_remove("tags");
        entry.set("tags", tags);
    }
}

fn author_identity(name: &str) -> String {
    crate::bibtex::display_bibtex_text(name)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn unique_authors(authors: Vec<String>) -> Vec<String> {
    let mut unique = Vec::new();
    for author in authors.into_iter().map(|author| author.trim().to_owned()) {
        let identity = author_identity(&author);
        if !identity.is_empty()
            && !author.eq_ignore_ascii_case("others")
            && !unique
                .iter()
                .any(|existing: &String| author_identity(existing) == identity)
        {
            unique.push(author);
        }
    }
    unique.sort_by_key(|author| author_identity(author));
    unique
}

fn normalize_author_profile(mut profile: AuthorProfile) -> AuthorProfile {
    profile.full_name = profile
        .full_name
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    profile.citation_name = profile.citation_name.trim().to_owned();
    profile.orcid = profile.orcid.trim().to_owned();
    profile.email = profile.email.trim().to_owned();
    profile.institution = profile.institution.trim().to_owned();
    profile
}

fn unique_author_profiles(authors: Vec<AuthorProfile>) -> Vec<AuthorProfile> {
    let mut unique = Vec::new();
    for author in authors.into_iter().map(normalize_author_profile) {
        let identity = author_identity(&author.citation_name);
        if !identity.is_empty()
            && !author.citation_name.eq_ignore_ascii_case("others")
            && !unique.iter().any(|existing: &AuthorProfile| {
                author_identity(&existing.citation_name) == identity
            })
        {
            unique.push(author);
        }
    }
    unique.sort_by_key(|author| author_identity(&author.citation_name));
    unique
}

fn merge_author_profiles(
    names: Vec<String>,
    profiles: Vec<AuthorProfile>,
    bibliography: &Bibliography,
) -> Vec<AuthorProfile> {
    let mut names = names;
    let profiles = unique_author_profiles(profiles);
    for entry in &bibliography.entries {
        names.extend(crate::bibtex::split_bibtex_names(entry.get("author")));
    }
    let merged = unique_authors(names)
        .into_iter()
        .map(|name| {
            profiles
                .iter()
                .find(|profile| author_identity(&profile.citation_name) == author_identity(&name))
                .cloned()
                .unwrap_or_else(|| AuthorProfile::from_citation_name(&name))
        })
        .collect::<Vec<_>>();
    unique_author_profiles(merged)
}

fn replacement_author_name(previous: &str, updated: &str) -> String {
    let previous = previous.trim();
    let updated = updated.trim();
    if previous.starts_with('{')
        && previous.ends_with('}')
        && !(updated.starts_with('{') && updated.ends_with('}'))
    {
        format!("{{{updated}}}")
    } else {
        updated.to_owned()
    }
}

fn set_entry_authors(entry: &mut BibEntry, authors: Vec<String>) {
    let authors = authors.join(" and ");
    entry.raw_fields.shift_remove("author");
    if authors.trim().is_empty() {
        entry.fields.shift_remove("author");
    } else {
        entry.set("author", authors);
    }
}

#[cfg(test)]
mod author_profile_tests {
    use super::{merge_author_profiles, AuthorProfile};
    use crate::bibtex::{BibEntry, Bibliography};

    #[test]
    fn complete_name_generates_reference_name_and_keeps_surname_particles() {
        assert_eq!(
            AuthorProfile::from_full_name("Vinícius Alves").citation_name,
            "Alves, Vinícius"
        );
        assert_eq!(
            AuthorProfile::from_full_name("Maria da Silva").citation_name,
            "da Silva, Maria"
        );
        assert_eq!(
            AuthorProfile::from_full_name("SingleName").citation_name,
            "SingleName"
        );
    }

    #[test]
    fn legacy_citation_names_migrate_without_losing_profiles_or_bibtex_names() {
        let mut entry = BibEntry::new("article", "paper");
        entry.set("author", "Alves, Vinícius and {Open Research Group}");
        let bibliography = Bibliography {
            entries: vec![entry],
            directives: Vec::new(),
        };
        let mut saved = AuthorProfile::from_citation_name("Alves, Vinícius");
        saved.orcid = "0000-0002-1825-0097".to_owned();
        saved.email = "vinicius@example.org".to_owned();

        let authors = merge_author_profiles(
            vec!["Alves, Vinícius".to_owned()],
            vec![saved],
            &bibliography,
        );

        assert_eq!(authors.len(), 2);
        let researcher = authors
            .iter()
            .find(|author| author.citation_name == "Alves, Vinícius")
            .unwrap();
        assert_eq!(researcher.full_name, "Vinícius Alves");
        assert_eq!(researcher.orcid, "0000-0002-1825-0097");
        assert_eq!(researcher.email, "vinicius@example.org");
        let group = authors
            .iter()
            .find(|author| author.citation_name == "{Open Research Group}")
            .unwrap();
        assert_eq!(group.full_name, "Open Research Group");
        assert_eq!(
            bibliography.entries[0].get("author"),
            "Alves, Vinícius and {Open Research Group}"
        );
    }

    #[test]
    fn profile_update_changes_bibtex_citation_name_and_keeps_other_fields() {
        let mut library = super::LocalLibrary::empty();
        let mut entry = BibEntry::new("article", "paper");
        entry.set("author", "Doe, Jane and Smith, Alex");
        library.add_in_memory(entry).unwrap();

        let mut updated = AuthorProfile::from_full_name("Janet Doe");
        updated.orcid = "0000-0002-1825-0097".to_owned();
        updated.email = "janet@example.org".to_owned();
        updated.institution = "Example University".to_owned();
        library.update_author_profile("Doe, Jane", updated).unwrap();

        let author = library
            .authors
            .iter()
            .find(|author| author.citation_name == "Doe, Janet")
            .unwrap();
        assert_eq!(author.full_name, "Janet Doe");
        assert_eq!(author.orcid, "0000-0002-1825-0097");
        assert_eq!(author.email, "janet@example.org");
        assert_eq!(author.institution, "Example University");
        assert_eq!(
            library.find("paper").unwrap().get("author"),
            "Doe, Janet and Smith, Alex"
        );
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

impl AppSettings {
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
