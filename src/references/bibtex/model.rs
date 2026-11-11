use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BibEntry {
    pub entry_type: String,
    pub key: String,
    pub fields: IndexMap<String, String>,
    pub raw_fields: IndexMap<String, String>,
}

impl BibEntry {
    pub fn new(entry_type: impl Into<String>, key: impl Into<String>) -> Self {
        Self {
            entry_type: entry_type.into(),
            key: key.into(),
            fields: IndexMap::new(),
            raw_fields: IndexMap::new(),
        }
    }

    pub fn get(&self, field: &str) -> &str {
        self.fields
            .get(&field.to_ascii_lowercase())
            .map(String::as_str)
            .unwrap_or("")
    }

    pub fn set(&mut self, field: impl Into<String>, value: impl Into<String>) {
        self.fields
            .insert(field.into().to_ascii_lowercase(), value.into());
    }
}

pub fn entry_matches_search(entry: &BibEntry, query: &str) -> bool {
    let query = query.trim().to_lowercase();
    query.is_empty()
        || entry.key.to_lowercase().contains(&query)
        || entry.fields.iter().any(|(name, value)| {
            name.to_lowercase().contains(&query)
                || value.to_lowercase().contains(&query)
                || display_bibtex_text(value).to_lowercase().contains(&query)
        })
}

pub struct Bibliography {
    pub entries: Vec<BibEntry>,
    pub directives: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BibError(pub String);

impl std::fmt::Display for BibError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for BibError {}
