use crate::bibtex::{create_citation_key, BibEntry};
use crate::storage::LocalLibrary;
use reqwest::blocking::{Client, Response};
use reqwest::header::HeaderMap;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

const API: &str = "https://api.zotero.org";
const API_VERSION: &str = "3";
const LEGACY_LOCAL_HASH_FIELDS: &[&str] = &[
    "title",
    "author",
    "editor",
    "translator",
    "date",
    "year",
    "journal",
    "booktitle",
    "publisher",
    "institution",
    "school",
    "edition",
    "type",
    "volume",
    "number",
    "pages",
    "doi",
    "url",
    "abstract",
    "keywords",
];
const ITEM_TYPES: &[(&str, &str)] = &[
    ("journalArticle", "article"),
    ("book", "book"),
    ("bookSection", "incollection"),
    ("conferencePaper", "inproceedings"),
    ("thesis", "phdthesis"),
    ("report", "techreport"),
    ("webpage", "online"),
    ("blogPost", "online"),
    ("magazineArticle", "article"),
    ("newspaperArticle", "article"),
    ("preprint", "article"),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncConflict {
    pub key: String,
    pub title: String,
    pub remote_key: String,
}

#[derive(Debug, Clone, Default)]
pub struct SyncReport {
    pub imported: usize,
    pub updated_locally: usize,
    pub matched: usize,
    pub created: usize,
    pub failed: usize,
    pub conflicts: Vec<SyncConflict>,
}

#[derive(Debug)]
pub struct ZoteroClient {
    user_id: String,
    api_key: String,
    client: Client,
    templates: HashMap<String, ItemTemplate>,
}

#[derive(Debug, Clone)]
struct ItemTemplate {
    template: Value,
    creator_types: HashSet<String>,
}

impl ZoteroClient {
    pub fn new(user_id: impl Into<String>, api_key: impl Into<String>) -> Result<Self, String> {
        let client = Client::builder()
            .user_agent("Ovenbird/0.1.0")
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|error| error.to_string())?;
        Ok(Self {
            user_id: user_id.into().trim().to_owned(),
            api_key: api_key.into(),
            client,
            templates: HashMap::new(),
        })
    }

    fn request(
        &self,
        method: reqwest::Method,
        url: &str,
        body: Option<&Value>,
        headers: &[(&str, String)],
    ) -> Result<(Value, HeaderMap), String> {
        let mut request = self
            .client
            .request(method, url)
            .header("Zotero-API-Version", API_VERSION)
            .header("Zotero-API-Key", &self.api_key);
        for (name, value) in headers {
            request = request.header(*name, value);
        }
        if let Some(body) = body {
            request = request.json(body);
        }
        let response = request.send().map_err(|error| error.to_string())?;
        parse_response(response)
    }

    fn item_template(&mut self, item_type: &str) -> Result<ItemTemplate, String> {
        if let Some(template) = self.templates.get(item_type) {
            return Ok(template.clone());
        }
        let query = format!("itemType={}", encode_component(item_type));
        let (template, _) = self.request(
            reqwest::Method::GET,
            &format!("{API}/items/new?{query}"),
            None,
            &[],
        )?;
        let (creators, _) = self.request(
            reqwest::Method::GET,
            &format!("{API}/itemTypeCreatorTypes?{query}"),
            None,
            &[],
        )?;
        let creator_types = creators
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|item| {
                item.get("creatorType")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            })
            .collect();
        let result = ItemTemplate {
            template,
            creator_types,
        };
        self.templates.insert(item_type.to_owned(), result.clone());
        Ok(result)
    }

    fn fetch_all_items(&self) -> Result<(Vec<Value>, Option<String>), String> {
        let mut items = Vec::new();
        let mut start = 0usize;
        let mut version = None;
        loop {
            let url = format!(
                "{API}/users/{}/items/top?limit=100&start={start}",
                encode_component(&self.user_id)
            );
            let (page, headers) = self.request(reqwest::Method::GET, &url, None, &[])?;
            version = headers
                .get("Last-Modified-Version")
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned)
                .or(version);
            let page = page.as_array().cloned().unwrap_or_default();
            let page_len = page.len();
            items.extend(page.into_iter().filter(|item| {
                !matches!(
                    item.pointer("/data/itemType").and_then(Value::as_str),
                    Some("attachment" | "note")
                )
            }));
            if page_len < 100 {
                break;
            }
            start += page_len;
        }
        Ok((items, version))
    }

    pub fn sync(&mut self, library: &mut LocalLibrary) -> Result<SyncReport, String> {
        let state_path = library.directory.join("zotero-sync.json");
        let mut state = read_sync_state(&state_path);
        let (remote_items, fetched_version) = self.fetch_all_items()?;
        let mut library_version = fetched_version;
        let mut existing_by_zotero = HashMap::<String, usize>::new();
        let mut existing_by_doi = HashMap::<String, usize>::new();
        let mut used_keys = HashSet::new();
        for (index, entry) in library.bibliography.entries.iter().enumerate() {
            if !entry.get("zotero_key").is_empty() {
                existing_by_zotero.insert(entry.get("zotero_key").to_owned(), index);
            }
            if !entry.get("doi").is_empty() {
                existing_by_doi.insert(entry.get("doi").to_ascii_lowercase(), index);
            }
            used_keys.insert(entry.key.to_ascii_lowercase());
        }

        let mut report = SyncReport::default();
        let mut pending_updates = Vec::<(usize, Value)>::new();
        let mut pending_creates = Vec::<usize>::new();
        let mut remote_keys = HashSet::new();

        for remote in &remote_items {
            let remote_key = value_string(remote.get("key"));
            if remote_key.is_empty() {
                continue;
            }
            remote_keys.insert(remote_key.clone());
            let remote_data = remote.get("data").cloned().unwrap_or_else(|| json!({}));
            let remote_hash = checksum(&serde_json::to_string(&remote_data).unwrap_or_default());
            let doi = remote_data
                .get("DOI")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_ascii_lowercase();
            let existing_index = existing_by_zotero.get(&remote_key).copied().or_else(|| {
                (!doi.is_empty())
                    .then(|| existing_by_doi.get(&doi).copied())
                    .flatten()
            });
            let Some(index) = existing_index else {
                let entry = to_bibtex(remote, &mut used_keys, None);
                let new_index = library.bibliography.entries.len();
                existing_by_zotero.insert(remote_key.clone(), new_index);
                library.bibliography.entries.push(entry);
                let entry_ref = &library.bibliography.entries[new_index];
                state_snapshot(&mut state, &remote_key, remote, entry_ref, &remote_hash);
                report.imported += 1;
                continue;
            };

            report.matched += 1;
            {
                let local = &mut library.bibliography.entries[index];
                if local.get("zotero_key") != remote_key {
                    local.set("zotero_key", remote_key.clone());
                    local.set("zotero_version", value_string(remote.get("version")));
                }
            }
            let local_hash = local_hash(&library.bibliography.entries[index]);
            let previous = state
                .get("items")
                .and_then(|items| items.get(&remote_key))
                .cloned();
            if previous.is_none() {
                let item_type = remote_data
                    .get("itemType")
                    .and_then(Value::as_str)
                    .unwrap_or("document");
                let template = self.item_template(item_type)?;
                if remote_differs(
                    &library.bibliography.entries[index],
                    &remote_data,
                    &template.creator_types,
                ) {
                    pending_updates.push((index, remote.clone()));
                } else {
                    state_snapshot(
                        &mut state,
                        &remote_key,
                        remote,
                        &library.bibliography.entries[index],
                        &remote_hash,
                    );
                }
                continue;
            }
            let previous = previous.unwrap_or_default();
            let local_changed = previous
                .get("localHash")
                .and_then(Value::as_str)
                .is_none_or(|previous_hash| {
                    previous_hash != local_hash.as_str()
                        && previous_hash != sparse_local_hash(&library.bibliography.entries[index])
                });
            let remote_changed = previous.get("remoteHash").and_then(Value::as_str)
                != Some(&remote_hash)
                || previous.get("version") != remote.get("version");
            if local_changed && remote_changed {
                report.conflicts.push(SyncConflict {
                    key: library.bibliography.entries[index].key.clone(),
                    title: {
                        let title = library.bibliography.entries[index].get("title");
                        if title.is_empty() {
                            library.bibliography.entries[index].key.clone()
                        } else {
                            title.to_owned()
                        }
                    },
                    remote_key,
                });
            } else if remote_changed && !local_changed {
                let preferred = library.bibliography.entries[index].key.clone();
                let updated = to_bibtex(remote, &mut used_keys, Some(&preferred));
                library.bibliography.entries[index] = updated;
                state_snapshot(
                    &mut state,
                    &remote_key,
                    remote,
                    &library.bibliography.entries[index],
                    &remote_hash,
                );
                report.updated_locally += 1;
            } else if local_changed {
                pending_updates.push((index, remote.clone()));
            } else {
                state_snapshot(
                    &mut state,
                    &remote_key,
                    remote,
                    &library.bibliography.entries[index],
                    &remote_hash,
                );
            }
        }

        for index in 0..library.bibliography.entries.len() {
            let entry = &library.bibliography.entries[index];
            if !entry.get("zotero_key").is_empty() {
                continue;
            }
            let doi = entry.get("doi");
            let duplicate = if doi.is_empty() {
                None
            } else {
                remote_items.iter().find(|item| {
                    item.pointer("/data/DOI")
                        .and_then(Value::as_str)
                        .is_some_and(|remote_doi| remote_doi.eq_ignore_ascii_case(doi))
                })
            };
            if let Some(remote) = duplicate {
                let remote_key = value_string(remote.get("key"));
                let version = value_string(remote.get("version"));
                let entry = &mut library.bibliography.entries[index];
                entry.set("zotero_key", remote_key.clone());
                entry.set("zotero_version", version);
                let hash = checksum(&serde_json::to_string(&remote["data"]).unwrap_or_default());
                state_snapshot(&mut state, &remote_key, remote, entry, &hash);
            } else {
                pending_creates.push(index);
            }
        }

        for (index, remote) in pending_updates {
            let remote_data = remote.get("data").cloned().unwrap_or_else(|| json!({}));
            let item_type = remote_data
                .get("itemType")
                .and_then(Value::as_str)
                .unwrap_or("document");
            let template = self.item_template(item_type)?;
            let mapped = to_zotero_update(
                &library.bibliography.entries[index],
                &remote_data,
                &template.template,
                &template.creator_types,
            );
            let mut data = remote_data;
            merge_json_object(&mut data, mapped);
            if let Some(key) = remote.get("key") {
                data["key"] = key.clone();
            }
            if let Some(version) = remote.get("version") {
                data["version"] = version.clone();
            }
            let remote_key = value_string(remote.get("key"));
            let url = format!(
                "{API}/users/{}/items/{remote_key}",
                encode_component(&self.user_id)
            );
            let (updated, headers) = self.request(reqwest::Method::PUT, &url, Some(&data), &[])?;
            library_version = headers
                .get("Last-Modified-Version")
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned)
                .or(library_version);
            let (refreshed, headers) = self.request(reqwest::Method::GET, &url, None, &[])?;
            library_version = headers
                .get("Last-Modified-Version")
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned)
                .or(library_version);
            let refreshed = if refreshed.is_null() {
                updated
            } else {
                refreshed
            };
            let entry = &mut library.bibliography.entries[index];
            entry.set("zotero_version", value_string(refreshed.get("version")));
            let hash = checksum(
                &serde_json::to_string(refreshed.get("data").unwrap_or(&Value::Null))
                    .unwrap_or_default(),
            );
            state_snapshot(&mut state, &remote_key, &refreshed, entry, &hash);
        }

        for batch in pending_creates.chunks(50) {
            let Some(version) = library_version.clone() else {
                return Err("Zotero did not report the current library version; no references were uploaded.".to_owned());
            };
            let mut body = Vec::with_capacity(batch.len());
            for index in batch {
                let entry = &library.bibliography.entries[*index];
                let item_type = bib_to_zotero(&entry.entry_type);
                let template = self.item_template(item_type)?;
                body.push(to_zotero(
                    entry,
                    Some(&template.template),
                    Some(&template.creator_types),
                ));
            }
            let url = format!("{API}/users/{}/items", encode_component(&self.user_id));
            let (response, headers) = self.request(
                reqwest::Method::POST,
                &url,
                Some(&Value::Array(body)),
                &[("If-Unmodified-Since-Version", version)],
            )?;
            library_version = headers
                .get("Last-Modified-Version")
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned)
                .or(library_version);
            let successful = response
                .get("successful")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            let failed = response
                .get("failed")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            report.failed += failed.len();
            for (position, result) in successful {
                let Ok(batch_index) = position.parse::<usize>() else {
                    continue;
                };
                let Some(index) = batch.get(batch_index).copied() else {
                    continue;
                };
                let remote_key = if result.is_string() {
                    value_string(Some(&result))
                } else {
                    value_string(result.get("key"))
                };
                if remote_key.is_empty() {
                    continue;
                }
                library.bibliography.entries[index].set("zotero_key", remote_key.clone());
                library.save().map_err(|error| error.to_string())?;
                report.created += 1;
                let url = format!(
                    "{API}/users/{}/items/{remote_key}",
                    encode_component(&self.user_id)
                );
                let refreshed = if result.get("data").is_some() {
                    result.clone()
                } else {
                    match self.request(reqwest::Method::GET, &url, None, &[]) {
                        Ok((value, _)) => value,
                        Err(_) => Value::Null,
                    }
                };
                if !refreshed.is_null() {
                    library.bibliography.entries[index]
                        .set("zotero_version", value_string(refreshed.get("version")));
                    let hash = checksum(
                        &serde_json::to_string(refreshed.get("data").unwrap_or(&Value::Null))
                            .unwrap_or_default(),
                    );
                    state_snapshot(
                        &mut state,
                        &remote_key,
                        &refreshed,
                        &library.bibliography.entries[index],
                        &hash,
                    );
                } else {
                    state["items"][&remote_key] = json!({
                        "version": null,
                        "remoteHash": null,
                        "localHash": local_hash(&library.bibliography.entries[index]),
                    });
                    report.failed += 1;
                }
            }
        }

        library.save().map_err(|error| error.to_string())?;
        state["items"] = Value::Object(
            state
                .get("items")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default(),
        );
        let mut serialized_state =
            serde_json::to_vec_pretty(&state).map_err(|error| error.to_string())?;
        serialized_state.push(b'\n');
        crate::storage::atomic_write(&state_path, &serialized_state)
            .map_err(|error| error.to_string())?;
        let _ = remote_keys;
        Ok(report)
    }
}

fn parse_response(response: Response) -> Result<(Value, HeaderMap), String> {
    let status = response.status();
    let headers = response.headers().clone();
    let text = response.text().map_err(|error| error.to_string())?;
    if !status.is_success() {
        return Err(format!("Zotero returned {}: {}", status.as_u16(), text));
    }
    let value = if text.is_empty() {
        Value::Null
    } else {
        serde_json::from_str(&text).unwrap_or_else(|_| Value::String(text))
    };
    Ok((value, headers))
}

fn read_sync_state(path: &Path) -> Value {
    fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .filter(|value: &Value| value.is_object())
        .unwrap_or_else(|| json!({"items": {}}))
}

fn checksum(value: &str) -> String {
    Sha256::digest(value.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn local_hash(entry: &BibEntry) -> String {
    let mut fields = Map::new();
    for name in LEGACY_LOCAL_HASH_FIELDS {
        let value = entry.get(name);
        fields.insert(
            (*name).to_owned(),
            Value::String(if value.trim().is_empty() {
                String::new()
            } else {
                value.to_owned()
            }),
        );
    }
    for (name, value) in &entry.fields {
        if name == "zotero_key"
            || name == "zotero_version"
            || LEGACY_LOCAL_HASH_FIELDS.contains(&name.as_str())
            || value.trim().is_empty()
        {
            continue;
        }
        fields.insert(name.clone(), Value::String(value.clone()));
    }
    let json = json!({"type": entry.entry_type, "fields": fields});
    checksum(&serde_json::to_string(&json).unwrap_or_default())
}

fn sparse_local_hash(entry: &BibEntry) -> String {
    let fields = entry
        .fields
        .iter()
        .filter(|(name, value)| {
            name.as_str() != "zotero_key"
                && name.as_str() != "zotero_version"
                && !value.trim().is_empty()
        })
        .map(|(name, value)| (name.clone(), Value::String(value.clone())))
        .collect::<Map<_, _>>();
    checksum(
        &serde_json::to_string(&json!({"type": entry.entry_type, "fields": fields}))
            .unwrap_or_default(),
    )
}

fn state_snapshot(
    state: &mut Value,
    key: &str,
    remote: &Value,
    local: &BibEntry,
    remote_hash: &str,
) {
    let items = state.get_mut("items").and_then(Value::as_object_mut);
    if let Some(items) = items {
        items.insert(
            key.to_owned(),
            json!({
                "version": remote.get("version").cloned().unwrap_or(Value::Null),
                "remoteHash": remote_hash,
                "localHash": local_hash(local),
            }),
        );
    } else {
        state["items"] = json!({ key: {
            "version": remote.get("version").cloned().unwrap_or(Value::Null),
            "remoteHash": remote_hash,
            "localHash": local_hash(local),
        }});
    }
}

fn value_string(value: Option<&Value>) -> String {
    value
        .map(|value| match value {
            Value::String(value) => value.clone(),
            Value::Number(value) => value.to_string(),
            _ => String::new(),
        })
        .unwrap_or_default()
}

fn encode_component(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

fn item_type(entry_type: &str) -> &'static str {
    match entry_type {
        "mastersthesis" => "thesis",
        _ => ITEM_TYPES
            .iter()
            .find(|(_, bib)| *bib == entry_type)
            .map(|(zotero, _)| *zotero)
            .unwrap_or("document"),
    }
}

fn bib_to_zotero(entry_type: &str) -> &'static str {
    item_type(entry_type)
}

fn to_bibtex(
    remote: &Value,
    used_keys: &mut HashSet<String>,
    preferred_key: Option<&str>,
) -> BibEntry {
    let data = remote.get("data").unwrap_or(&Value::Null);
    let zotero_type = data.get("itemType").and_then(Value::as_str).unwrap_or("");
    let bib_type = ITEM_TYPES
        .iter()
        .find(|(zotero, _)| *zotero == zotero_type)
        .map(|(_, bib)| *bib)
        .unwrap_or("misc");
    let creators = data
        .get("creators")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let format_creators = |creator_type: &str| -> String {
        creators
            .iter()
            .filter(|creator| {
                creator.get("creatorType").and_then(Value::as_str) == Some(creator_type)
            })
            .map(|creator| creator_name(creator))
            .filter(|name| !name.is_empty())
            .collect::<Vec<_>>()
            .join(" and ")
    };
    let publication = [
        "publicationTitle",
        "bookTitle",
        "proceedingsTitle",
        "websiteTitle",
    ]
    .iter()
    .find_map(|name| data.get(*name).and_then(Value::as_str))
    .unwrap_or("");
    let date = string_field(data, "date");
    let mut entry = BibEntry::new(bib_type, "");
    for (field, value) in [
        ("title", string_field(data, "title")),
        ("author", format_creators("author")),
        ("editor", format_creators("editor")),
        ("translator", format_creators("translator")),
        ("date", date.clone()),
        ("year", digits_year(&date)),
        (
            "journal",
            if matches!(bib_type, "incollection" | "inproceedings") {
                String::new()
            } else {
                publication.to_owned()
            },
        ),
        (
            "booktitle",
            if matches!(bib_type, "incollection" | "inproceedings") {
                publication.to_owned()
            } else {
                String::new()
            },
        ),
        ("publisher", string_field(data, "publisher")),
        ("institution", string_field(data, "institution")),
        ("school", string_field(data, "university")),
        ("edition", string_field(data, "edition")),
        ("type", first_nonempty(data, &["thesisType", "reportType"])),
        ("volume", string_field(data, "volume")),
        ("number", string_field(data, "issue")),
        ("pages", string_field(data, "pages")),
        ("doi", string_field(data, "DOI")),
        ("url", string_field(data, "url")),
        ("abstract", string_field(data, "abstractNote")),
        (
            "keywords",
            data.get("tags")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|tag| tag.get("tag").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join(", "),
        ),
        ("zotero_key", value_string(remote.get("key"))),
        ("zotero_version", value_string(remote.get("version"))),
    ] {
        if !value.is_empty() {
            entry.set(field, value);
        }
    }
    let key = preferred_key
        .map(str::to_owned)
        .unwrap_or_else(|| create_citation_key(&entry.fields, used_keys));
    used_keys.insert(key.to_ascii_lowercase());
    entry.key = key;
    entry
}

fn creator_name(creator: &Value) -> String {
    if let Some(name) = creator.get("name").and_then(Value::as_str) {
        return format!("{{{name}}}");
    }
    let last = string_field(creator, "lastName");
    let first = string_field(creator, "firstName");
    if last.is_empty() {
        first
    } else if first.is_empty() {
        last
    } else {
        format!("{last}, {first}")
    }
}

fn string_field(value: &Value, field: &str) -> String {
    value
        .get(field)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned()
}

fn first_nonempty(value: &Value, fields: &[&str]) -> String {
    fields
        .iter()
        .map(|field| string_field(value, field))
        .find(|field| !field.is_empty())
        .unwrap_or_default()
}

fn digits_year(date: &str) -> String {
    date.as_bytes()
        .windows(4)
        .find(|window| window.iter().all(u8::is_ascii_digit))
        .map(|bytes| String::from_utf8_lossy(bytes).to_string())
        .unwrap_or_default()
}

fn to_zotero(
    entry: &BibEntry,
    template: Option<&Value>,
    allowed_creators: Option<&HashSet<String>>,
) -> Value {
    let kind = item_type(&entry.entry_type);
    let template_creators = template
        .and_then(|value| value.get("creators"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut creators = template_creators
        .into_iter()
        .filter(|creator| {
            !matches!(
                creator.get("creatorType").and_then(Value::as_str),
                Some("author" | "editor" | "translator")
            )
        })
        .collect::<Vec<_>>();
    for creator_type in ["author", "editor", "translator"] {
        if allowed_creators
            .is_some_and(|allowed| !allowed.is_empty() && !allowed.contains(creator_type))
        {
            continue;
        }
        for name in split_creators(entry.get(creator_type)) {
            let trimmed = name.trim();
            if trimmed.is_empty() {
                continue;
            }
            if trimmed.starts_with('{') && trimmed.ends_with('}') {
                creators.push(
                    json!({"creatorType": creator_type, "name": &trimmed[1..trimmed.len()-1]}),
                );
            } else if let Some((last, first)) = trimmed.split_once(',') {
                creators.push(json!({"creatorType": creator_type, "lastName": last.trim(), "firstName": first.trim()}));
            } else {
                let mut parts = trimmed.split_whitespace().collect::<Vec<_>>();
                let last = parts.pop().unwrap_or("");
                creators.push(json!({"creatorType": creator_type, "firstName": parts.join(" "), "lastName": last}));
            }
        }
    }
    let mut result = json!({
        "itemType": kind,
        "title": entry.get("title"),
        "creators": creators,
        "date": if entry.get("date").is_empty() { entry.get("year") } else { entry.get("date") },
        "publisher": entry.get("publisher"),
        "volume": entry.get("volume"),
        "issue": entry.get("number"),
        "pages": entry.get("pages"),
        "DOI": entry.get("doi"),
        "url": entry.get("url"),
        "abstractNote": entry.get("abstract"),
        "tags": entry.get("keywords").split(|ch| ch == ',' || ch == ';').map(str::trim)
            .filter(|tag| !tag.is_empty()).map(|tag| json!({"tag": tag})).collect::<Vec<_>>(),
    });
    match entry.entry_type.as_str() {
        "article" => result["publicationTitle"] = json!(entry.get("journal")),
        "incollection" => result["bookTitle"] = json!(entry.get("booktitle")),
        "inproceedings" => result["proceedingsTitle"] = json!(entry.get("booktitle")),
        "online" => {
            result["websiteTitle"] = json!(if entry.get("journal").is_empty() {
                entry.get("organization")
            } else {
                entry.get("journal")
            })
        }
        "techreport" => {
            result["institution"] = json!(if entry.get("institution").is_empty() {
                entry.get("publisher")
            } else {
                entry.get("institution")
            })
        }
        "phdthesis" | "mastersthesis" => {
            result["university"] = json!(if entry.get("school").is_empty() {
                entry.get("institution")
            } else {
                entry.get("school")
            });
            result["thesisType"] = json!(if entry.get("type").is_empty() {
                if entry.entry_type == "mastersthesis" {
                    "Master's thesis"
                } else {
                    "PhD thesis"
                }
            } else {
                entry.get("type")
            });
        }
        "book" => result["edition"] = json!(entry.get("edition")),
        _ => {}
    }
    if let Some(template) = template.and_then(Value::as_object) {
        let valid = template.keys().collect::<HashSet<_>>();
        result
            .as_object_mut()
            .unwrap()
            .retain(|name, _| valid.contains(name));
    }
    result
}

fn to_zotero_update(
    entry: &BibEntry,
    remote_data: &Value,
    template: &Value,
    allowed_creators: &HashSet<String>,
) -> Value {
    let mut mapped = to_zotero(entry, Some(template), Some(allowed_creators));
    let mut creators = remote_data
        .get("creators")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|creator| {
            !matches!(
                creator.get("creatorType").and_then(Value::as_str),
                Some("author" | "editor" | "translator")
            )
        })
        .cloned()
        .collect::<Vec<_>>();
    creators.extend(
        mapped
            .get("creators")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|creator| {
                matches!(
                    creator.get("creatorType").and_then(Value::as_str),
                    Some("author" | "editor" | "translator")
                )
            })
            .cloned(),
    );
    mapped["creators"] = Value::Array(creators);
    mapped
}

fn split_creators(value: &str) -> Vec<&str> {
    let mut creators = Vec::new();
    let mut start = 0;
    let lower = value.to_ascii_lowercase();
    let mut search = 0;
    while let Some(offset) = lower[search..].find(" and ") {
        let end = search + offset;
        creators.push(&value[start..end]);
        start = end + 5;
        search = start;
    }
    creators.push(&value[start..]);
    creators
}

fn remote_differs(local: &BibEntry, remote: &Value, allowed_creators: &HashSet<String>) -> bool {
    let desired = to_zotero(local, None, Some(allowed_creators));
    let Some(desired) = desired.as_object() else {
        return false;
    };
    desired.iter().any(|(name, value)| {
        let current = remote.get(name);
        if name == "creators" {
            return creator_signature(value) != current.map(creator_signature).unwrap_or_default();
        }
        if name == "tags" {
            return tag_signature(value) != current.map(tag_signature).unwrap_or_default();
        }
        let empty = matches!(value, Value::String(text) if text.is_empty());
        let current_empty = current.is_none_or(Value::is_null)
            || matches!(current, Some(Value::String(text)) if text.is_empty());
        if empty && current_empty {
            return false;
        }
        if let Value::Array(desired) = value {
            if desired.is_empty() && current.and_then(Value::as_array).is_none_or(Vec::is_empty) {
                return false;
            }
        }
        current != Some(value)
    })
}

fn creator_signature(value: &Value) -> String {
    let mut signature = value
        .as_array()
        .into_iter()
        .flatten()
        .map(|creator| {
            let creator_type = string_field(creator, "creatorType");
            let name = if let Some(name) = creator.get("name").and_then(Value::as_str) {
                name.to_owned()
            } else {
                format!(
                    "{},{}",
                    string_field(creator, "lastName"),
                    string_field(creator, "firstName")
                )
            };
            format!("{creator_type}:{name}")
        })
        .collect::<Vec<_>>();
    signature.sort();
    signature.join("|")
}

fn tag_signature(value: &Value) -> String {
    let mut tags = value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|tag| tag.get("tag").and_then(Value::as_str).map(str::to_owned))
        .collect::<Vec<_>>();
    tags.sort();
    tags.join("|")
}

fn merge_json_object(target: &mut Value, source: Value) {
    if let (Some(target), Some(source)) = (target.as_object_mut(), source.as_object()) {
        for (key, value) in source {
            target.insert(key.clone(), value.clone());
        }
    }
}

pub fn store_api_key(api_key: &str) -> Result<(), String> {
    let schema = secret_schema();
    let mut attributes = HashMap::new();
    attributes.insert("application", "ovenbird");
    libsecret::password_store_sync(
        Some(&schema),
        attributes,
        Some(&libsecret::COLLECTION_DEFAULT),
        "Ovenbird Zotero API key",
        api_key,
        gtk::gio::Cancellable::NONE,
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

pub fn load_api_key() -> Result<Option<String>, String> {
    let schema = secret_schema();
    let mut attributes = HashMap::new();
    attributes.insert("application", "ovenbird");
    libsecret::password_lookup_sync(Some(&schema), attributes, gtk::gio::Cancellable::NONE)
        .map(|value| value.map(|value| value.to_string()))
        .map_err(|error| error.to_string())
}

fn secret_schema() -> libsecret::Schema {
    let mut attributes = HashMap::new();
    attributes.insert("application", libsecret::SchemaAttributeType::String);
    libsecret::Schema::new(
        "org.ovenbird.Ovenbird",
        libsecret::SchemaFlags::NONE,
        attributes,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zotero_import_preserves_reference_metadata_and_remote_identity() {
        let remote = json!({
            "key": "ABCD1234",
            "version": 17,
            "data": {
                "itemType": "journalArticle",
                "title": "Notes on the Analytical Engine",
                "creators": [
                    {"creatorType": "author", "firstName": "Ada", "lastName": "Lovelace"},
                    {"creatorType": "editor", "name": "Open Research Group"}
                ],
                "date": "1843-08-01",
                "publicationTitle": "Scientific Memoirs",
                "DOI": "10.1000/example",
                "tags": [{"tag": "history"}, {"tag": "computing"}]
            }
        });

        let entry = to_bibtex(&remote, &mut HashSet::new(), None);

        assert_eq!(entry.entry_type, "article");
        assert_eq!(entry.get("title"), "Notes on the Analytical Engine");
        assert_eq!(entry.get("author"), "Lovelace, Ada");
        assert_eq!(entry.get("editor"), "{Open Research Group}");
        assert_eq!(entry.get("year"), "1843");
        assert_eq!(entry.get("journal"), "Scientific Memoirs");
        assert_eq!(entry.get("doi"), "10.1000/example");
        assert_eq!(entry.get("keywords"), "history, computing");
        assert_eq!(entry.get("zotero_key"), "ABCD1234");
        assert_eq!(entry.get("zotero_version"), "17");
    }

    #[test]
    fn zotero_export_maps_bibtex_fields_creators_and_tags() {
        let mut entry = BibEntry::new("article", "lovelace1843notes");
        entry.set("title", "Notes on the Analytical Engine");
        entry.set("author", "Lovelace, Ada and {Open Research Group}");
        entry.set("year", "1843");
        entry.set("journal", "Scientific Memoirs");
        entry.set("doi", "10.1000/example");
        entry.set("keywords", "history, computing; mathematics");
        let allowed = HashSet::from(["author".to_owned()]);

        let item = to_zotero(&entry, None, Some(&allowed));

        assert_eq!(item["itemType"], "journalArticle");
        assert_eq!(item["title"], "Notes on the Analytical Engine");
        assert_eq!(item["date"], "1843");
        assert_eq!(item["publicationTitle"], "Scientific Memoirs");
        assert_eq!(item["DOI"], "10.1000/example");
        assert_eq!(item["creators"][0]["lastName"], "Lovelace");
        assert_eq!(item["creators"][0]["firstName"], "Ada");
        assert_eq!(item["creators"][1]["name"], "Open Research Group");
        assert_eq!(item["tags"].as_array().unwrap().len(), 3);
        assert_eq!(item["tags"][2]["tag"], "mathematics");
    }

    #[test]
    fn zotero_updates_preserve_remote_creator_roles_not_in_bibtex() {
        let mut entry = BibEntry::new("article", "key");
        entry.set("title", "Updated title");
        entry.set("author", "Doe, Jane");
        let allowed = HashSet::from(["author".to_owned()]);
        let schema = json!({
            "itemType": "journalArticle",
            "title": "",
            "creators": [{"creatorType": "author"}],
            "publicationTitle": ""
        });
        let mut remote = json!({
            "itemType": "journalArticle",
            "title": "Old title",
            "creators": [
                {"creatorType": "author", "firstName": "Old", "lastName": "Author"},
                {"creatorType": "reviewedAuthor", "firstName": "Remote", "lastName": "Creator"}
            ],
            "publicationTitle": "Journal"
        });

        let mapped = to_zotero_update(&entry, &remote, &schema, &allowed);
        merge_json_object(&mut remote, mapped);

        assert_eq!(
            remote["creators"],
            json!([
                {"creatorType": "reviewedAuthor", "firstName": "Remote", "lastName": "Creator"},
                {"creatorType": "author", "lastName": "Doe", "firstName": "Jane"}
            ])
        );
    }

    #[test]
    fn local_hash_matches_legacy_js_zotero_snapshot_after_bibtex_reload() {
        let entry = crate::bibtex::parse_bibtex(
            "@article{key,\n  title = {Notes},\n  doi = {10.1000/example},\n  zotero_key = {ABCD1234},\n  zotero_version = {17}\n}",
        ).unwrap().entries.remove(0);
        let legacy_snapshot = checksum(
            r#"{"type":"article","fields":{"title":"Notes","author":"","editor":"","translator":"","date":"","year":"","journal":"","booktitle":"","publisher":"","institution":"","school":"","edition":"","type":"","volume":"","number":"","pages":"","doi":"10.1000/example","url":"","abstract":"","keywords":""}}"#,
        );

        assert_eq!(local_hash(&entry), legacy_snapshot);
        let mut with_empty_fields = entry.clone();
        with_empty_fields.set("author", "");
        assert_eq!(local_hash(&with_empty_fields), local_hash(&entry));
    }

    #[test]
    fn master_thesis_exports_as_zotero_thesis() {
        let mut entry = BibEntry::new("mastersthesis", "research2026");
        entry.set("title", "A research thesis");
        entry.set("school", "Federal University");

        let item = to_zotero(&entry, None, None);

        assert_eq!(item["itemType"], "thesis");
        assert_eq!(item["university"], "Federal University");
        assert_eq!(item["thesisType"], "Master's thesis");
    }

    #[test]
    fn zotero_comparison_ignores_creator_and_tag_order_but_detects_changes() {
        let mut entry = BibEntry::new("article", "key");
        entry.set("title", "A title");
        entry.set("author", "Doe, Jane and Smith, Alex");
        entry.set("keywords", "one, two");
        let allowed = HashSet::from(["author".to_owned()]);
        let mut remote = to_zotero(&entry, None, Some(&allowed));
        remote["creators"].as_array_mut().unwrap().reverse();
        remote["tags"].as_array_mut().unwrap().reverse();

        assert!(!remote_differs(&entry, &remote, &allowed));
        remote["title"] = json!("Changed title");
        assert!(remote_differs(&entry, &remote, &allowed));
    }
}
