use indexmap::IndexMap;

fn tr(message: &str) -> String {
    crate::i18n::gettext(message)
}

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
            name.to_lowercase().contains(&query) || value.to_lowercase().contains(&query)
        })
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

pub const REFERENCE_TYPES: &[&str] = &[
    "article",
    "book",
    "incollection",
    "inproceedings",
    "phdthesis",
    "mastersthesis",
    "techreport",
    "online",
    "misc",
];

pub fn reference_type_label(entry_type: &str) -> String {
    match entry_type {
        "article" => tr("Journal article"),
        "book" => tr("Book"),
        "incollection" => tr("Book chapter"),
        "inproceedings" => tr("Conference paper"),
        "phdthesis" => tr("Doctoral thesis"),
        "mastersthesis" => tr("Master's thesis"),
        "techreport" => tr("Technical report"),
        "online" => tr("Online resource"),
        _ => tr("Miscellaneous"),
    }
}

pub fn reference_field_label(name: &str, entry_type: &str) -> String {
    match name {
        "title" => tr("Title"),
        "author" => tr("Author(s)"),
        "editor" => tr("Editor(s)"),
        "date" => tr("Year / date"),
        "journal" => tr("Journal"),
        "publisher" => tr("Publisher"),
        "volume" => tr("Volume"),
        "number" => tr("Number / issue"),
        "edition" => tr("Edition"),
        "pages" => tr("Pages"),
        "doi" => tr("DOI"),
        "url" => tr("URL"),
        "keywords" => tr("Keywords"),
        "series" => tr("Series"),
        "address" => tr("Publication location"),
        "isbn" => tr("ISBN"),
        "chapter" => tr("Chapter"),
        "organization" if entry_type == "online" => tr("Website or organization"),
        "organization" => tr("Organization"),
        "school" => tr("University or academic institution"),
        "institution" => tr("Institution"),
        "type" => tr("Report type"),
        "month" => tr("Month"),
        "note" => tr("Note"),
        "howpublished" => tr("Publication method"),
        "urldate" => tr("Access date"),
        "booktitle" if entry_type == "inproceedings" => tr("Proceedings title"),
        "booktitle" => tr("Book title"),
        _ => name.to_owned(),
    }
}

pub fn fields_for_reference_type(entry_type: &str) -> &'static [&'static str] {
    match entry_type {
        "article" => &[
            "title", "author", "date", "journal", "volume", "number", "pages", "month", "doi",
            "url", "note", "keywords",
        ],
        "book" => &[
            "title",
            "author",
            "editor",
            "date",
            "publisher",
            "edition",
            "volume",
            "number",
            "series",
            "address",
            "isbn",
            "month",
            "doi",
            "url",
            "note",
            "keywords",
        ],
        "incollection" => &[
            "title",
            "author",
            "editor",
            "booktitle",
            "date",
            "publisher",
            "edition",
            "volume",
            "number",
            "chapter",
            "pages",
            "series",
            "address",
            "month",
            "doi",
            "url",
            "note",
            "keywords",
        ],
        "inproceedings" => &[
            "title",
            "author",
            "editor",
            "booktitle",
            "date",
            "publisher",
            "organization",
            "volume",
            "number",
            "pages",
            "address",
            "month",
            "doi",
            "url",
            "note",
            "keywords",
        ],
        "phdthesis" | "mastersthesis" => &[
            "title", "author", "date", "school", "address", "month", "doi", "url", "note",
            "keywords",
        ],
        "techreport" => &[
            "title",
            "author",
            "date",
            "institution",
            "type",
            "number",
            "address",
            "month",
            "doi",
            "url",
            "note",
            "keywords",
        ],
        "online" => &[
            "title",
            "author",
            "organization",
            "date",
            "url",
            "urldate",
            "doi",
            "note",
            "keywords",
        ],
        _ => &[
            "title",
            "author",
            "date",
            "howpublished",
            "doi",
            "url",
            "note",
            "keywords",
        ],
    }
}

fn known_reference_fields() -> impl Iterator<Item = &'static str> {
    REFERENCE_TYPES
        .iter()
        .flat_map(|entry_type| fields_for_reference_type(entry_type).iter().copied())
        .chain(["year"])
}

pub fn build_reference_fields(
    entry_type: &str,
    values: &IndexMap<String, String>,
    existing: &IndexMap<String, String>,
) -> IndexMap<String, String> {
    let known = known_reference_fields().collect::<std::collections::HashSet<_>>();
    let mut result = existing
        .iter()
        .filter(|(name, _)| !known.contains(name.as_str()))
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect::<IndexMap<_, _>>();

    for name in fields_for_reference_type(entry_type) {
        if let Some(value) = values
            .get(*name)
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
        {
            result.insert((*name).to_owned(), value.to_owned());
        }
    }
    if let Some(date) = result.get("date").cloned() {
        let year = four_digit_year(&date).unwrap_or(date.as_str()).to_owned();
        if !year.is_empty() {
            result.insert("year".to_owned(), year);
        }
    }
    result
}

fn four_digit_year(value: &str) -> Option<&str> {
    value
        .as_bytes()
        .windows(4)
        .position(|chunk| chunk.iter().all(u8::is_ascii_digit))
        .map(|start| &value[start..start + 4])
}

fn skip_space(source: &str, mut position: usize) -> usize {
    while position < source.len() {
        let character = source[position..]
            .chars()
            .next()
            .expect("valid UTF-8 boundary");
        if !character.is_whitespace() {
            break;
        }
        position += character.len_utf8();
    }
    position
}

fn read_balanced(
    source: &str,
    start: usize,
    open: char,
    close: char,
) -> Result<(String, usize), BibError> {
    let mut depth = 0usize;
    let mut escaped = false;
    for (offset, character) in source[start..].char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if character == '\\' {
            escaped = true;
            continue;
        }
        if character == open {
            depth += 1;
        }
        if character == close {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                let end = start + offset + character.len_utf8();
                return Ok((
                    source[start + open.len_utf8()..start + offset].to_owned(),
                    end,
                ));
            }
        }
    }
    Err(BibError("BibTeX contains an unclosed value.".to_owned()))
}

fn read_expression(source: &str, mut position: usize, closing: char) -> (String, usize) {
    let start = position;
    let mut brace_depth = 0usize;
    let mut quoted = false;
    let mut escaped = false;
    while position < source.len() {
        let character = source[position..]
            .chars()
            .next()
            .expect("valid UTF-8 boundary");
        if escaped {
            escaped = false;
            position += character.len_utf8();
            continue;
        }
        if character == '\\' {
            escaped = true;
            position += character.len_utf8();
            continue;
        }
        if character == '"' && brace_depth == 0 {
            quoted = !quoted;
        } else if !quoted && character == '{' {
            brace_depth += 1;
        } else if !quoted && character == '}' && brace_depth > 0 {
            brace_depth -= 1;
        } else if !quoted && brace_depth == 0 && (character == ',' || character == closing) {
            break;
        }
        position += character.len_utf8();
    }
    (source[start..position].trim().to_owned(), position)
}

fn macro_definitions(directives: &[String]) -> IndexMap<String, String> {
    let months = [
        "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
    ];
    let names = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];
    let mut macros = months
        .iter()
        .zip(names)
        .map(|(key, value)| ((*key).to_owned(), value.to_owned()))
        .collect::<IndexMap<_, _>>();
    for directive in directives {
        let Some((body, _)) = directive_body(directive, "string") else {
            continue;
        };
        let body = body.trim();
        let Some(equal) = body.find('=') else {
            continue;
        };
        let name = body[..equal].trim().to_ascii_lowercase();
        let (raw, _) = read_expression(body, equal + 1, '\0');
        if !name.is_empty() {
            macros.insert(name, raw);
        }
    }
    macros
}

fn directive_body(source: &str, expected_type: &str) -> Option<(String, char)> {
    let trimmed = source.trim_start();
    let rest = trimmed.strip_prefix('@')?;
    let name_end = rest.find(|ch: char| ch.is_whitespace() || ch == '{' || ch == '(')?;
    if !rest[..name_end].eq_ignore_ascii_case(expected_type) {
        return None;
    }
    let after_type = skip_space(rest, name_end);
    let open = rest[after_type..].chars().next()?;
    let close = match open {
        '{' => '}',
        '(' => ')',
        _ => return None,
    };
    let start = source.len() - trimmed.len() + 1 + after_type;
    let (body, _) = read_balanced(source, start, open, close).ok()?;
    Some((body, close))
}

fn expression_value(
    raw: &str,
    macros: &IndexMap<String, String>,
    resolving: &mut Vec<String>,
) -> Result<String, BibError> {
    let mut chunks = Vec::new();
    let mut position = 0usize;
    while position < raw.len() {
        position = skip_space(raw, position);
        if position >= raw.len() {
            break;
        }
        let character = raw[position..]
            .chars()
            .next()
            .expect("valid UTF-8 boundary");
        if character == '#' {
            position += 1;
            continue;
        }
        if character == '{' {
            let (value, end) = read_balanced(raw, position, '{', '}')?;
            chunks.push(value);
            position = end;
        } else if character == '"' {
            let start = position + 1;
            position = start;
            let mut escaped = false;
            while position < raw.len() {
                let ch = raw[position..]
                    .chars()
                    .next()
                    .expect("valid UTF-8 boundary");
                if !escaped && ch == '"' {
                    break;
                }
                if !escaped && ch == '\\' {
                    escaped = true;
                } else {
                    escaped = false;
                }
                position += ch.len_utf8();
            }
            chunks.push(raw[start..position].to_owned());
            if position < raw.len() {
                position += 1;
            }
        } else {
            let start = position;
            while position < raw.len() {
                let ch = raw[position..]
                    .chars()
                    .next()
                    .expect("valid UTF-8 boundary");
                if ch == '#' || ch.is_whitespace() {
                    break;
                }
                position += ch.len_utf8();
            }
            if position == start {
                break;
            }
            let name = raw[start..position].to_ascii_lowercase();
            if let Some(value) = macros.get(&name) {
                if resolving.contains(&name) {
                    chunks.push(raw[start..position].to_owned());
                } else {
                    resolving.push(name);
                    chunks.push(expression_value(value, macros, resolving)?);
                    resolving.pop();
                }
            } else {
                chunks.push(raw[start..position].to_owned());
            }
        }
    }
    Ok(chunks.join(""))
}

pub fn parse_bibtex(source: &str) -> Result<Bibliography, BibError> {
    let mut entries = Vec::new();
    let mut directives = Vec::new();
    let mut position = 0usize;
    while let Some(relative_at) = source[position..].find('@') {
        let at = position + relative_at;
        position = at + 1;
        position = skip_space(source, position);
        let type_start = position;
        while position < source.len() {
            let ch = source[position..]
                .chars()
                .next()
                .expect("valid UTF-8 boundary");
            if !(ch.is_ascii_alphanumeric() || ch == '_' || ch == '-') {
                break;
            }
            position += ch.len_utf8();
        }
        if position == type_start {
            continue;
        }
        let entry_type = source[type_start..position].to_ascii_lowercase();
        position = skip_space(source, position);
        let Some(open) = source[position..].chars().next() else {
            break;
        };
        let close = match open {
            '{' => '}',
            '(' => ')',
            _ => continue,
        };
        position += open.len_utf8();

        if ["comment", "preamble", "string"].contains(&entry_type.as_str()) {
            let (_, end) = read_balanced(source, position - open.len_utf8(), open, close)?;
            directives.push(source[at..end].trim().to_owned());
            position = end;
            continue;
        }

        position = skip_space(source, position);
        let key_start = position;
        while position < source.len() {
            let ch = source[position..]
                .chars()
                .next()
                .expect("valid UTF-8 boundary");
            if ch == ',' || ch == close {
                break;
            }
            position += ch.len_utf8();
        }
        let key = source[key_start..position].trim().to_owned();
        if source[position..].starts_with(close) {
            position += close.len_utf8();
            continue;
        }
        if !source[position..].starts_with(',') {
            continue;
        }
        position += 1;

        let mut entry = BibEntry::new(entry_type, key);
        while position < source.len() {
            position = skip_space(source, position);
            if position >= source.len() {
                break;
            }
            let current = source[position..]
                .chars()
                .next()
                .expect("valid UTF-8 boundary");
            if current == close {
                position += close.len_utf8();
                break;
            }
            if current == ',' {
                position += 1;
                continue;
            }
            let name_start = position;
            while position < source.len() {
                let ch = source[position..]
                    .chars()
                    .next()
                    .expect("valid UTF-8 boundary");
                if !(ch.is_ascii_alphanumeric() || ch == '_' || ch == '-') {
                    break;
                }
                position += ch.len_utf8();
            }
            if name_start == position {
                while position < source.len() {
                    let ch = source[position..]
                        .chars()
                        .next()
                        .expect("valid UTF-8 boundary");
                    position += ch.len_utf8();
                    if ch == ',' || ch == close {
                        break;
                    }
                }
                continue;
            }
            let name = source[name_start..position].to_ascii_lowercase();
            position = skip_space(source, position);
            if !source[position..].starts_with('=') {
                while position < source.len()
                    && !source[position..].starts_with(',')
                    && !source[position..].starts_with(close)
                {
                    let ch = source[position..].chars().next().unwrap();
                    position += ch.len_utf8();
                }
                continue;
            }
            position += 1;
            position = skip_space(source, position);
            let (raw, end) = read_expression(source, position, close);
            let value = expression_value(&raw, &IndexMap::new(), &mut Vec::new())?;
            entry.raw_fields.insert(name.clone(), raw);
            entry.fields.insert(name, value);
            position = end;
            if source[position..].starts_with(',') {
                position += 1;
            }
        }
        if !entry.key.is_empty() {
            entries.push(entry);
        }
    }

    let macros = macro_definitions(&directives);
    for entry in &mut entries {
        for (name, raw) in &entry.raw_fields {
            entry.fields.insert(
                name.clone(),
                expression_value(raw, &macros, &mut Vec::new())?,
            );
        }
    }
    Ok(Bibliography {
        entries,
        directives,
    })
}

fn serialize_value(
    entry: &BibEntry,
    name: &str,
    value: &str,
    macros: &IndexMap<String, String>,
) -> String {
    if let Some(raw) = entry.raw_fields.get(name) {
        if expression_value(raw, macros, &mut Vec::new()).as_deref() == Ok(value) {
            return raw.clone();
        }
    }
    let mut escaped = String::with_capacity(value.len());
    for (index, ch) in value.char_indices() {
        if ch == '%' && !value[..index].ends_with('\\') {
            escaped.push('\\');
        }
        escaped.push(ch);
    }
    format!("{{{escaped}}}")
}

pub fn serialize_bibtex(bibliography: &Bibliography) -> String {
    let macros = macro_definitions(&bibliography.directives);
    let mut parts = bibliography.directives.clone();
    for entry in &bibliography.entries {
        let fields = entry
            .fields
            .iter()
            .filter(|(_, value)| !value.trim().is_empty())
            .map(|(name, value)| {
                format!(
                    "  {name} = {}",
                    serialize_value(entry, name, value, &macros)
                )
            })
            .collect::<Vec<_>>();
        parts.push(format!(
            "@{}{{{},\n{}\n}}",
            entry.entry_type,
            entry.key,
            fields.join(",\n")
        ));
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!("{}\n", parts.join("\n\n"))
    }
}

pub fn create_citation_key(
    fields: &IndexMap<String, String>,
    used: &std::collections::HashSet<String>,
) -> String {
    let author = fields
        .get("author")
        .or_else(|| fields.get("editor"))
        .map(String::as_str)
        .unwrap_or("ref");
    let first_author = author
        .split_whitespace()
        .take_while(|word| !word.eq_ignore_ascii_case("and"))
        .collect::<Vec<_>>()
        .join(" ");
    let first_author = first_author.trim();
    let surname = if let Some((surname, _)) = first_author.split_once(',') {
        surname.trim().trim_matches(['{', '}'])
    } else {
        first_author
            .trim_matches(['{', '}'])
            .split_whitespace()
            .last()
            .unwrap_or("ref")
    };
    let title = fields
        .get("title")
        .map(String::as_str)
        .unwrap_or("")
        .chars()
        .filter(|ch| !matches!(ch, '{' | '}' | '\\'))
        .collect::<String>();
    let title_word = title
        .split_whitespace()
        .find(|word| word.chars().count() > 3)
        .unwrap_or("");
    let date_year = fields.get("date").and_then(|date| {
        date.split(|character: char| !character.is_ascii_digit())
            .find(|part| part.len() == 4)
    });
    let year = fields
        .get("year")
        .map(String::as_str)
        .or(date_year)
        .unwrap_or("");
    let base = deunicode::deunicode(&format!("{surname}{year}{title_word}"))
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .collect::<String>();
    let base = if base.is_empty() {
        "ref".to_owned()
    } else {
        base
    };
    let mut key = base.clone();
    let mut suffix = 2;
    while used
        .iter()
        .any(|existing| existing.eq_ignore_ascii_case(&key))
    {
        key = format!("{base}{suffix}");
        suffix += 1;
    }
    key
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn generated_citation_keys_preserve_js_capitalization() {
        let fields = IndexMap::from([
            ("author".to_owned(), "Lópes, Ana".to_owned()),
            ("year".to_owned(), "2024".to_owned()),
            ("title".to_owned(), "Árvore e Pesquisa".to_owned()),
        ]);

        assert_eq!(
            create_citation_key(&fields, &HashSet::new()),
            "Lopes2024Arvore"
        );
        assert_eq!(
            create_citation_key(&fields, &HashSet::from(["lopes2024arvore".to_owned()])),
            "Lopes2024Arvore2",
        );
    }

    #[test]
    fn reference_search_includes_all_fields_case_insensitively() {
        let mut entry = BibEntry::new("article", "key2024");
        entry.set("title", "A useful paper");
        entry.set("journal", "Journal of Example Research");
        entry.set("doi", "10.1234/DOI-EXAMPLE");

        assert!(entry_matches_search(&entry, "example research"));
        assert!(entry_matches_search(&entry, "10.1234/doi-example"));
        assert!(entry_matches_search(&entry, "  "));
        assert!(!entry_matches_search(&entry, "missing"));
    }
}
