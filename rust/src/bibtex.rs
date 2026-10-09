use indexmap::IndexMap;
use regex::Regex;

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
            name.to_lowercase().contains(&query)
                || value.to_lowercase().contains(&query)
                || display_bibtex_text(value).to_lowercase().contains(&query)
        })
}

/// Converts common BibTeX/LaTeX text accents to readable Unicode for display.
/// The stored BibTeX values remain unchanged for editing and export.
pub fn display_bibtex_text(value: &str) -> String {
    let characters = value.chars().collect::<Vec<_>>();
    let mut index = 0;
    decode_tex_display(&characters, &mut index, false)
}

pub fn display_bibtex_names(value: &str) -> Vec<String> {
    split_bibtex_names(value)
        .into_iter()
        .map(|name| display_bibtex_text(&name))
        .filter(|name| !name.trim().is_empty())
        .collect()
}

fn decode_tex_display(characters: &[char], index: &mut usize, in_group: bool) -> String {
    let mut output = String::new();
    while *index < characters.len() {
        match characters[*index] {
            '}' if in_group => break,
            '{' => {
                *index += 1;
                output.push_str(&decode_tex_display(characters, index, true));
                if characters.get(*index) == Some(&'}') {
                    *index += 1;
                }
            }
            '\\' => output.push_str(&decode_tex_command(characters, index)),
            character => {
                output.push(character);
                *index += 1;
            }
        }
    }
    output
}

fn decode_tex_command(characters: &[char], index: &mut usize) -> String {
    *index += 1;
    let Some(&first) = characters.get(*index) else {
        return "\\".to_owned();
    };

    let accent = match first {
        '\'' => Some('´'),
        '`' => Some('`'),
        '^' => Some('^'),
        '~' => Some('~'),
        '"' => Some('¨'),
        '=' => Some('¯'),
        '.' => Some('˙'),
        _ => None,
    };
    if let Some(accent) = accent {
        *index += 1;
        return read_tex_display_argument(characters, index)
            .map(|argument| apply_tex_accent(&argument, accent))
            .unwrap_or_else(|| format!("\\{first}"));
    }

    if first.is_ascii_alphabetic() {
        let start = *index;
        while characters
            .get(*index)
            .is_some_and(|character| character.is_ascii_alphabetic())
        {
            *index += 1;
        }
        let command = characters[start..*index].iter().collect::<String>();
        let accent = match command.as_str() {
            "c" => Some('¸'),
            "v" => Some('ˇ'),
            "u" => Some('˘'),
            "H" => Some('˝'),
            "r" => Some('˚'),
            _ => None,
        };
        if let Some(accent) = accent {
            return read_tex_display_argument(characters, index)
                .map(|argument| apply_tex_accent(&argument, accent))
                .unwrap_or_else(|| format!("\\{command}"));
        }
        return match command.as_str() {
            "i" => "ı".to_owned(),
            "j" => "ȷ".to_owned(),
            "ae" => "æ".to_owned(),
            "AE" => "Æ".to_owned(),
            "oe" => "œ".to_owned(),
            "OE" => "Œ".to_owned(),
            "aa" => "å".to_owned(),
            "AA" => "Å".to_owned(),
            "o" => "ø".to_owned(),
            "O" => "Ø".to_owned(),
            "ss" => "ß".to_owned(),
            "l" => "ł".to_owned(),
            "L" => "Ł".to_owned(),
            "dh" => "ð".to_owned(),
            "DH" => "Ð".to_owned(),
            "th" => "þ".to_owned(),
            "TH" => "Þ".to_owned(),
            "ng" => "ŋ".to_owned(),
            "NG" => "Ŋ".to_owned(),
            _ => format!("\\{command}"),
        };
    }

    *index += 1;
    match first {
        '&' | '%' | '$' | '#' | '_' | '{' | '}' => first.to_string(),
        '\\' => "\\".to_owned(),
        ' ' => " ".to_owned(),
        _ => format!("\\{first}"),
    }
}

fn read_tex_display_argument(characters: &[char], index: &mut usize) -> Option<String> {
    while characters.get(*index).is_some_and(|character| character.is_whitespace()) {
        *index += 1;
    }
    match characters.get(*index).copied()? {
        '{' => {
            *index += 1;
            let argument = decode_tex_display(characters, index, true);
            if characters.get(*index) == Some(&'}') {
                *index += 1;
            }
            Some(argument)
        }
        _ => Some(decode_tex_command_or_character(characters, index)),
    }
}

fn decode_tex_command_or_character(characters: &[char], index: &mut usize) -> String {
    if characters.get(*index) == Some(&'\\') {
        decode_tex_command(characters, index)
    } else {
        let Some(&character) = characters.get(*index) else {
            return String::new();
        };
        *index += 1;
        character.to_string()
    }
}

fn apply_tex_accent(value: &str, accent: char) -> String {
    let mut characters = value.chars();
    let Some(base) = characters.next() else {
        return value.to_owned();
    };
    let rest = characters.collect::<String>();
    let composed = match (base, accent) {
        ('A', '´') => 'Á', ('E', '´') => 'É', ('I', '´') => 'Í', ('O', '´') => 'Ó',
        ('U', '´') => 'Ú', ('Y', '´') => 'Ý', ('C', '´') => 'Ć', ('N', '´') => 'Ń',
        ('a', '´') => 'á', ('e', '´') => 'é', ('i', '´') => 'í', ('o', '´') => 'ó',
        ('u', '´') => 'ú', ('y', '´') => 'ý', ('c', '´') => 'ć', ('n', '´') => 'ń',
        ('A', '`') => 'À', ('E', '`') => 'È', ('I', '`') => 'Ì', ('O', '`') => 'Ò',
        ('U', '`') => 'Ù', ('a', '`') => 'à', ('e', '`') => 'è', ('i', '`') => 'ì',
        ('o', '`') => 'ò', ('u', '`') => 'ù',
        ('A', '^') => 'Â', ('E', '^') => 'Ê', ('I', '^') => 'Î', ('O', '^') => 'Ô',
        ('U', '^') => 'Û', ('a', '^') => 'â', ('e', '^') => 'ê', ('i', '^') => 'î',
        ('o', '^') => 'ô', ('u', '^') => 'û',
        ('A', '~') => 'Ã', ('N', '~') => 'Ñ', ('O', '~') => 'Õ', ('a', '~') => 'ã',
        ('n', '~') => 'ñ', ('o', '~') => 'õ',
        ('A', '¨') => 'Ä', ('E', '¨') => 'Ë', ('I', '¨') => 'Ï', ('O', '¨') => 'Ö',
        ('U', '¨') => 'Ü', ('Y', '¨') => 'Ÿ', ('a', '¨') => 'ä', ('e', '¨') => 'ë',
        ('i', '¨') => 'ï', ('o', '¨') => 'ö', ('u', '¨') => 'ü', ('y', '¨') => 'ÿ',
        ('C', '¸') => 'Ç', ('S', '¸') => 'Ş', ('T', '¸') => 'Ţ', ('c', '¸') => 'ç',
        ('s', '¸') => 'ş', ('t', '¸') => 'ţ',
        ('A', 'ˇ') => 'Ǎ', ('C', 'ˇ') => 'Č', ('D', 'ˇ') => 'Ď', ('E', 'ˇ') => 'Ě',
        ('L', 'ˇ') => 'Ľ', ('N', 'ˇ') => 'Ň', ('R', 'ˇ') => 'Ř', ('S', 'ˇ') => 'Š',
        ('T', 'ˇ') => 'Ť', ('Z', 'ˇ') => 'Ž', ('a', 'ˇ') => 'ǎ', ('c', 'ˇ') => 'č',
        ('d', 'ˇ') => 'ď', ('e', 'ˇ') => 'ě', ('l', 'ˇ') => 'ľ', ('n', 'ˇ') => 'ň',
        ('r', 'ˇ') => 'ř', ('s', 'ˇ') => 'š', ('t', 'ˇ') => 'ť', ('z', 'ˇ') => 'ž',
        ('A', '˘') => 'Ă', ('G', '˘') => 'Ğ', ('U', '˘') => 'Ŭ', ('a', '˘') => 'ă',
        ('g', '˘') => 'ğ', ('u', '˘') => 'ŭ',
        ('O', '˝') => 'Ő', ('U', '˝') => 'Ű', ('o', '˝') => 'ő', ('u', '˝') => 'ű',
        ('A', '˚') => 'Å', ('U', '˚') => 'Ů', ('a', '˚') => 'å', ('u', '˚') => 'ů',
        _ => '\0',
    };
    let mut output = String::new();
    if composed != '\0' {
        output.push(composed);
    } else {
        output.push(base);
        output.push(match accent {
            '´' => '\u{0301}', '`' => '\u{0300}', '^' => '\u{0302}', '~' => '\u{0303}',
            '¨' => '\u{0308}', '¯' => '\u{0304}', '˙' => '\u{0307}', '¸' => '\u{0327}',
            'ˇ' => '\u{030c}', '˘' => '\u{0306}', '˝' => '\u{030b}', '˚' => '\u{030a}',
            _ => accent,
        });
    }
    output.push_str(&rest);
    output
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
        "year" => tr("Year"),
        "journal" => tr("Journal"),
        "publisher" => tr("Publisher"),
        "volume" => tr("Volume"),
        "number" => tr("Number / issue"),
        "edition" => tr("Edition"),
        "pages" => tr("Pages"),
        "doi" => tr("DOI"),
        "url" => tr("URL"),
        "keywords" => tr("Keywords"),
        "tags" => tr("Tags"),
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
            "url", "note", "keywords", "tags",
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
            "tags",
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
            "tags",
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
            "tags",
        ],
        "phdthesis" | "mastersthesis" => &[
            "title", "author", "date", "school", "address", "month", "doi", "url", "note",
            "keywords", "tags",
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
            "tags",
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
            "tags",
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
            "tags",
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceStyle {
    Abnt,
    Apa7,
    Ieee,
    ChicagoAuthorDate,
    Mla,
    Ams,
    Harvard,
    Vancouver,
}

pub fn reference_style_for_document(source: &str) -> ReferenceStyle {
    let source = source.to_ascii_lowercase();
    let class_pattern = Regex::new(r"(?i)\\documentclass(?:\s*\[[^\]]*\])?\s*\{([^}]+)\}")
        .unwrap();
    let bibliography_pattern =
        Regex::new(r"(?i)\\bibliographystyle\s*\{([^}]+)\}").unwrap();
    let option_style_pattern = Regex::new(r"(?i)\bstyle\s*=\s*([a-z0-9-]+)").unwrap();
    let classes = class_pattern
        .captures_iter(&source)
        .map(|capture| capture[1].trim().to_owned())
        .collect::<Vec<_>>();
    let bibliography_styles = bibliography_pattern
        .captures_iter(&source)
        .map(|capture| capture[1].trim().to_owned())
        .collect::<Vec<_>>();
    let option_styles = option_style_pattern
        .captures_iter(&source)
        .map(|capture| capture[1].trim().to_owned())
        .collect::<Vec<_>>();
    let identifies = |markers: &[&str]| {
        classes
            .iter()
            .chain(bibliography_styles.iter())
            .chain(option_styles.iter())
            .any(|value| markers.iter().any(|marker| value.contains(marker)))
    };

    if identifies(&["ieee", "ieeetr"]) {
        ReferenceStyle::Ieee
    } else if identifies(&["ams"]) || source.contains("amsrefs") {
        ReferenceStyle::Ams
    } else if identifies(&["mla"]) {
        ReferenceStyle::Mla
    } else if identifies(&["vancouver"]) {
        ReferenceStyle::Vancouver
    } else if identifies(&["harvard", "agsm", "dcu", "authordate"]) {
        ReferenceStyle::Harvard
    } else if identifies(&["apa", "apalike", "apacite"]) {
        ReferenceStyle::Apa7
    } else if identifies(&["chicago"]) {
        ReferenceStyle::ChicagoAuthorDate
    } else {
        ReferenceStyle::Abnt
    }
}

pub fn format_reference_citation(entry: &BibEntry, style: ReferenceStyle) -> String {
    let author_field = if entry.get("author").trim().is_empty() {
        entry.get("editor")
    } else {
        entry.get("author")
    };
    let authors = format_authors(author_field, style);
    match style {
        ReferenceStyle::Mla => return format_mla_reference(entry, &authors),
        ReferenceStyle::Ams => return format_ams_reference(entry, &authors),
        ReferenceStyle::Harvard => return format_harvard_reference(entry, &authors),
        ReferenceStyle::Vancouver => return format_vancouver_reference(entry, &authors),
        _ => {}
    }
    let title = nonempty(entry.get("title"));
    let year = publication_year(entry);
    let details = publication_details(entry, style);
    let mut parts = Vec::new();

    match style {
        ReferenceStyle::Abnt => {
            push_value(&mut parts, nonempty(&authors));
            push_value(&mut parts, title);
            push_value(&mut parts, nonempty(&details));
            push_value(&mut parts, year);
        }
        ReferenceStyle::Apa7 => {
            push_value(
                &mut parts,
                if authors.is_empty() {
                    year.map(|year| format!("({year})"))
                } else {
                    Some(format!(
                        "{authors}{}",
                        year.map(|year| format!(" ({year})")).unwrap_or_default()
                    ))
                },
            );
            push_value(&mut parts, title);
            push_value(&mut parts, nonempty(&details));
        }
        ReferenceStyle::Ieee => {
            push_value(&mut parts, nonempty(&authors));
            push_value(&mut parts, title.map(|title| format!("“{title}”")));
            push_value(&mut parts, nonempty(&details));
            push_value(&mut parts, year);
        }
        ReferenceStyle::ChicagoAuthorDate => {
            push_value(&mut parts, nonempty(&authors));
            push_value(&mut parts, year);
            let title = title.map(|title| match entry.entry_type.as_str() {
                "article" | "incollection" | "inproceedings" => format!("“{title}”"),
                _ => title,
            });
            push_value(&mut parts, title);
            push_value(&mut parts, nonempty(&details));
        }
        ReferenceStyle::Mla
        | ReferenceStyle::Ams
        | ReferenceStyle::Harvard
        | ReferenceStyle::Vancouver => unreachable!("handled above"),
    }

    let doi = nonempty(entry.get("doi"));
    let url = nonempty(entry.get("url"));
    let has_url = doi.is_some() || url.is_some();
    let url = doi.map(|doi| format_doi_url(&doi)).or(url);
    if let Some(url) = url {
        let label = if style == ReferenceStyle::Abnt {
            format!("Disponível em: {url}")
        } else {
            url
        };
        push_value(&mut parts, Some(label));
    }
    if style == ReferenceStyle::Abnt {
        push_value(
            &mut parts,
            nonempty(entry.get("urldate")).map(|date| format!("Acesso em: {date}")),
        );
    }
    if parts.is_empty() {
        return entry.key.clone();
    }

    let citation = parts.join(if style == ReferenceStyle::Ieee { ", " } else { ". " });
    if style != ReferenceStyle::Abnt && has_url {
        citation
    } else {
        format!("{}.", citation.trim_end_matches(['.', ' ']))
    }
}

fn format_mla_reference(entry: &BibEntry, authors: &str) -> String {
    let mut parts = Vec::new();
    push_value(
        &mut parts,
        nonempty(authors).map(|authors| format!("{authors}.")),
    );
    push_value(
        &mut parts,
        nonempty(entry.get("title")).map(|title| {
            if matches!(
                entry.entry_type.as_str(),
                "article" | "incollection" | "inproceedings"
            ) {
                format!("“{title}.”")
            } else {
                format!("{title}.")
            }
        }),
    );
    push_value(
        &mut parts,
        nonempty(&publication_details(entry, ReferenceStyle::Mla))
            .map(|details| format!("{details}.")),
    );
    append_web_access(&mut parts, entry, "Accessed");
    join_reference_parts(parts)
}

fn format_ams_reference(entry: &BibEntry, authors: &str) -> String {
    let mut parts = Vec::new();
    push_value(&mut parts, nonempty(authors));
    push_value(
        &mut parts,
        nonempty(entry.get("title")).map(|title| {
            if matches!(
                entry.entry_type.as_str(),
                "article" | "incollection" | "inproceedings"
            ) {
                format!("“{title}”")
            } else {
                title
            }
        }),
    );
    push_value(
        &mut parts,
        nonempty(&publication_details(entry, ReferenceStyle::Ams)),
    );
    append_web_access(&mut parts, entry, "Accessed");
    let citation = parts.join(", ");
    if citation.is_empty() {
        entry.key.clone()
    } else {
        format!("{}.", citation.trim_end_matches(['.', ' ', ',']))
    }
}

fn format_harvard_reference(entry: &BibEntry, authors: &str) -> String {
    let mut parts = Vec::new();
    push_value(
        &mut parts,
        nonempty(authors).map(|authors| format!("{authors}.")),
    );
    push_value(
        &mut parts,
        Some(format!(
            "({}).",
            publication_year(entry).unwrap_or_else(|| "n.d.".to_owned())
        )),
    );
    push_value(
        &mut parts,
        nonempty(entry.get("title")).map(|title| {
            if matches!(
                entry.entry_type.as_str(),
                "article" | "incollection" | "inproceedings"
            ) {
                format!("‘{title}’.")
            } else {
                format!("{title}.")
            }
        }),
    );
    push_value(
        &mut parts,
        nonempty(&publication_details(entry, ReferenceStyle::Harvard))
            .map(|details| format!("{details}.")),
    );
    append_web_access(&mut parts, entry, "Accessed");
    join_reference_parts(parts)
}

fn format_vancouver_reference(entry: &BibEntry, authors: &str) -> String {
    let mut parts = Vec::new();
    push_value(
        &mut parts,
        nonempty(authors).map(|authors| format!("{authors}.")),
    );
    push_value(
        &mut parts,
        nonempty(entry.get("title")).map(|title| format!("{title}.")),
    );
    push_value(
        &mut parts,
        nonempty(&publication_details(entry, ReferenceStyle::Vancouver))
            .map(|details| format!("{details}.")),
    );
    append_web_access(&mut parts, entry, "Cited");
    join_reference_parts(parts)
}

fn append_web_access(parts: &mut Vec<String>, entry: &BibEntry, access_label: &str) {
    let url = nonempty(entry.get("doi"))
        .map(|doi| format_doi_url(&doi))
        .or_else(|| nonempty(entry.get("url")));
    push_value(parts, url.map(|url| format!("{url}.")));
    push_value(
        parts,
        nonempty(entry.get("urldate")).map(|date| format!("{access_label} {date}.")),
    );
}

fn join_reference_parts(parts: Vec<String>) -> String {
    let citation = parts
        .into_iter()
        .filter(|part| !part.trim().is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if citation.is_empty() {
        String::new()
    } else if citation.ends_with(['.', '!', '?']) {
        citation
    } else {
        format!("{citation}.")
    }
}

fn publication_details(entry: &BibEntry, style: ReferenceStyle) -> String {
    let mut parts = Vec::new();
    match entry.entry_type.as_str() {
        "article" => {
            let journal = nonempty(entry.get("journal"));
            let volume = nonempty(entry.get("volume"));
            let issue = nonempty(entry.get("number"));
            let year = publication_year(entry);
            let pages = nonempty(entry.get("pages"));

            if style == ReferenceStyle::Vancouver {
                push_value(&mut parts, journal);
                let volume_issue = match (volume, issue) {
                    (Some(volume), Some(issue)) => format!("{volume}({issue})"),
                    (Some(volume), None) => volume,
                    (None, Some(issue)) => format!("({issue})"),
                    (None, None) => String::new(),
                };
                let mut year_volume_pages = year.unwrap_or_default();
                if !volume_issue.is_empty() {
                    if !year_volume_pages.is_empty() {
                        year_volume_pages.push(';');
                    }
                    year_volume_pages.push_str(&volume_issue);
                }
                if let Some(pages) = pages {
                    year_volume_pages.push(':');
                    year_volume_pages.push_str(&pages);
                }
                push_value(&mut parts, nonempty(&year_volume_pages));
                return parts.join(". ");
            }

            if style == ReferenceStyle::Mla {
                push_value(&mut parts, journal);
                push_value(&mut parts, volume.map(|volume| format!("vol. {volume}")));
                push_value(&mut parts, issue.map(|issue| format!("no. {issue}")));
                push_value(&mut parts, year);
                push_value(&mut parts, pages.map(|pages| format!("pp. {pages}")));
                return parts.join(", ");
            }

            if style == ReferenceStyle::Ams {
                push_value(&mut parts, journal);
                let volume_year = match (volume, year) {
                    (Some(volume), Some(year)) => format!("{volume} ({year})"),
                    (Some(volume), None) => volume,
                    (None, Some(year)) => format!("({year})"),
                    (None, None) => String::new(),
                };
                push_value(&mut parts, nonempty(&volume_year));
                push_value(&mut parts, issue.map(|issue| format!("no. {issue}")));
                push_value(&mut parts, pages);
                return parts.join(", ");
            }

            push_value(&mut parts, journal);
            let volume_issue = match (volume, issue, style) {
                (Some(volume), Some(issue), ReferenceStyle::Abnt) => {
                    format!("v. {volume}, n. {issue}")
                }
                (Some(volume), Some(issue), ReferenceStyle::Ieee) => {
                    format!("vol. {volume}, no. {issue}")
                }
                (Some(volume), Some(issue), ReferenceStyle::ChicagoAuthorDate) => {
                    format!("{volume}, no. {issue}")
                }
                (Some(volume), Some(issue), _) => format!("{volume}({issue})"),
                (Some(volume), None, ReferenceStyle::Abnt) => format!("v. {volume}"),
                (Some(volume), None, ReferenceStyle::Ieee) => format!("vol. {volume}"),
                (Some(volume), None, _) => volume,
                (None, Some(issue), ReferenceStyle::Ieee) => format!("no. {issue}"),
                (None, Some(issue), ReferenceStyle::Abnt) => format!("n. {issue}"),
                (None, Some(issue), _) => format!("no. {issue}"),
                (None, None, _) => String::new(),
            };
            push_value(&mut parts, nonempty(&volume_issue));
            push_value(
                &mut parts,
                nonempty(entry.get("pages")).map(|pages| match style {
                    ReferenceStyle::Abnt => format!("p. {pages}"),
                    ReferenceStyle::Ieee => format!("pp. {pages}"),
                    ReferenceStyle::Mla | ReferenceStyle::Harvard => format!("pp. {pages}"),
                    ReferenceStyle::Vancouver => format!("p. {pages}"),
                    _ => pages,
                }),
            );
        }
        "book" => {
            if matches!(
                style,
                ReferenceStyle::Mla | ReferenceStyle::Ams | ReferenceStyle::Vancouver
            ) {
                push_value(
                    &mut parts,
                    nonempty(entry.get("edition")).map(|edition| format!("{edition} ed.")),
                );
                let publisher = match (
                    nonempty(entry.get("address")),
                    nonempty(entry.get("publisher")),
                    style,
                ) {
                    (Some(address), Some(publisher), ReferenceStyle::Vancouver) => {
                        Some(format!("{address}: {publisher}"))
                    }
                    (_, Some(publisher), _) => Some(publisher),
                    (Some(address), None, _) => Some(address),
                    _ => None,
                };
                let year = publication_year(entry);
                if style == ReferenceStyle::Vancouver {
                    let publication = match (publisher, year) {
                        (Some(publisher), Some(year)) => Some(format!("{publisher}; {year}")),
                        (Some(publisher), None) => Some(publisher),
                        (None, Some(year)) => Some(year),
                        (None, None) => None,
                    };
                    push_value(&mut parts, publication);
                    return parts.join(" ");
                }
                push_value(&mut parts, publisher);
                push_value(&mut parts, year);
                return parts.join(", ");
            }
            push_value(
                &mut parts,
                nonempty(entry.get("edition")).map(|edition| format!("{edition} ed.")),
            );
            let publisher = match (
                nonempty(entry.get("address")),
                nonempty(entry.get("publisher")),
            ) {
                (Some(address), Some(publisher)) if style == ReferenceStyle::Abnt => {
                    Some(format!("{address}: {publisher}"))
                }
                (_, Some(publisher)) => Some(publisher),
                (Some(address), None) => Some(address),
                _ => None,
            };
            push_value(&mut parts, publisher);
        }
        "incollection" | "inproceedings" => {
            let booktitle = nonempty(entry.get("booktitle"));
            let editors = format_authors(entry.get("editor"), style);
            let container = match (booktitle, nonempty(&editors), style) {
                (Some(title), Some(editors), ReferenceStyle::Abnt) => {
                    Some(format!("In: {editors}. {title}"))
                }
                (Some(title), Some(editors), ReferenceStyle::Apa7) => {
                    Some(format!("In {editors} (Eds.), {title}"))
                }
                (Some(title), Some(editors), ReferenceStyle::ChicagoAuthorDate) => {
                    Some(format!("In {title}, edited by {editors}"))
                }
                (Some(title), Some(editors), ReferenceStyle::Mla) => {
                    Some(format!("{title}, edited by {editors}"))
                }
                (Some(title), Some(editors), ReferenceStyle::Harvard) => {
                    Some(format!("In: {title}, edited by {editors}"))
                }
                (Some(title), _, ReferenceStyle::Ieee) => Some(format!("in {title}")),
                (Some(title), _, _) => Some(format!("In: {title}")),
                (None, Some(editors), _) => Some(editors),
                (None, None, _) => None,
            };
            push_value(&mut parts, container);
            push_value(
                &mut parts,
                nonempty(entry.get("pages")).map(|pages| match style {
                    ReferenceStyle::Abnt => format!("p. {pages}"),
                    ReferenceStyle::Ieee => format!("pp. {pages}"),
                    ReferenceStyle::Mla | ReferenceStyle::Harvard => format!("pp. {pages}"),
                    ReferenceStyle::Vancouver => format!("p. {pages}"),
                    _ => pages,
                }),
            );
            push_value(&mut parts, nonempty(entry.get("publisher")));
        }
        "phdthesis" | "mastersthesis" => {
            push_value(
                &mut parts,
                Some(match entry.entry_type.as_str() {
                    "phdthesis" => "Doctoral thesis".to_owned(),
                    _ => "Master's thesis".to_owned(),
                }),
            );
            push_value(&mut parts, nonempty(entry.get("school")));
        }
        "techreport" => {
            push_value(&mut parts, nonempty(entry.get("type")));
            push_value(&mut parts, nonempty(entry.get("institution")));
            push_value(&mut parts, nonempty(entry.get("number")));
        }
        "online" => push_value(&mut parts, nonempty(entry.get("organization"))),
        _ => {
            push_value(&mut parts, nonempty(entry.get("howpublished")));
            push_value(&mut parts, nonempty(entry.get("publisher")));
        }
    }
    if matches!(
        style,
        ReferenceStyle::Mla | ReferenceStyle::Ams | ReferenceStyle::Vancouver
    ) {
        push_value(&mut parts, publication_year(entry));
    }
    parts.join(", ")
}

fn format_authors(value: &str, style: ReferenceStyle) -> String {
    let raw_names = split_bibtex_names(value);
    if style == ReferenceStyle::Mla {
        return match raw_names.as_slice() {
            [] => String::new(),
            [only] => format_author_name(only, style),
            [first, second] => {
                let first = format_author_name(first, style);
                let second = format_author_name(second, ReferenceStyle::ChicagoAuthorDate);
                let second = second
                    .split_once(", ")
                    .map(|(family, given)| format!("{given} {family}"))
                    .unwrap_or(second);
                format!("{first}, and {second}")
            }
            [first, ..] => format!("{}, et al.", format_author_name(first, style)),
        };
    }

    let mut names = raw_names
        .iter()
        .map(|name| format_author_name(name, style))
        .filter(|name| !name.is_empty())
        .collect::<Vec<_>>();
    if style == ReferenceStyle::Vancouver && names.len() > 6 {
        names.truncate(6);
        names.push("et al.".to_owned());
    }
    if style == ReferenceStyle::ChicagoAuthorDate {
        for name in names.iter_mut().skip(1) {
            if let Some((family, given)) = name.split_once(", ") {
                *name = format!("{given} {family}");
            }
        }
    }
    let conjunction = match style {
        ReferenceStyle::Apa7 => "&",
        ReferenceStyle::Ieee
        | ReferenceStyle::ChicagoAuthorDate
        | ReferenceStyle::Ams
        | ReferenceStyle::Harvard => "and",
        ReferenceStyle::Vancouver => return names.join(", "),
        ReferenceStyle::Abnt => return names.join("; "),
        ReferenceStyle::Mla => unreachable!("handled above"),
    };
    match names.as_slice() {
        [] => String::new(),
        [only] => only.clone(),
        [first, second] => format!("{first} {conjunction} {second}"),
        _ => format!(
            "{}, {conjunction} {}",
            names[..names.len() - 1].join(", "),
            names.last().unwrap()
        ),
    }
}

fn format_author_name(name: &str, style: ReferenceStyle) -> String {
    let name = name.trim();
    let corporate = name.starts_with('{') && name.ends_with('}');
    let clean = name.trim_matches(['{', '}']).trim();
    if corporate {
        return if style == ReferenceStyle::Abnt {
            clean.to_uppercase()
        } else {
            clean.to_owned()
        };
    }

    let pieces = clean.split(',').map(str::trim).collect::<Vec<_>>();
    let (given, family) = if pieces.len() > 1 {
        (
            pieces.get(1).copied().unwrap_or_default().to_owned(),
            pieces[0].to_owned(),
        )
    } else {
        let words = clean.split_whitespace().collect::<Vec<_>>();
        if words.len() < 2 {
            return clean.to_owned();
        }
        let mut family_start = words.len() - 1;
        while family_start > 0 && is_surname_particle(words[family_start - 1]) {
            family_start -= 1;
        }
        (words[..family_start].join(" "), words[family_start..].join(" "))
    };
    match style {
        ReferenceStyle::Abnt => format!(
            "{}{}",
            family.to_uppercase(),
            if given.is_empty() {
                String::new()
            } else {
                format!(", {given}")
            }
        ),
        ReferenceStyle::Apa7 => format!("{}, {}", family, initials(&given)),
        ReferenceStyle::Ieee | ReferenceStyle::Ams => {
            format!("{} {}", initials(&given), family).trim().to_owned()
        }
        ReferenceStyle::Harvard => format!("{}, {}", family, initials(&given)),
        ReferenceStyle::Mla => {
            if given.is_empty() {
                family
            } else {
                format!("{family}, {given}")
            }
        }
        ReferenceStyle::Vancouver => format!(
            "{} {}",
            family,
            initials(&given).replace('.', "")
        )
        .trim()
        .to_owned(),
        ReferenceStyle::ChicagoAuthorDate => format!("{}, {}", family, given),
    }
}

pub fn split_bibtex_names(value: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    for (index, character) in value.char_indices() {
        match character {
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            _ => {}
        }
        if depth == 0
            && value[index..]
                .get(..5)
                .is_some_and(|delimiter| delimiter.eq_ignore_ascii_case(" and "))
        {
            names.push(value[start..index].trim().to_owned());
            start = index + 5;
        }
    }
    let last = value[start..].trim();
    if !last.is_empty() {
        names.push(last.to_owned());
    }
    names
}

fn is_surname_particle(word: &str) -> bool {
    matches!(
        word.to_ascii_lowercase().as_str(),
        "da" | "das" | "de" | "del" | "della" | "di" | "do" | "dos" | "du" | "la"
            | "le" | "van" | "von"
    )
}

fn initials(given: &str) -> String {
    given
        .split_whitespace()
        .flat_map(|word| word.split('-'))
        .filter_map(|part| part.trim_matches('.').chars().next())
        .map(|initial| format!("{initial}."))
        .collect::<Vec<_>>()
        .join(" ")
}

fn publication_year(entry: &BibEntry) -> Option<String> {
    nonempty(entry.get("year")).or_else(|| {
        entry
            .get("date")
            .split(|character: char| !character.is_ascii_digit())
            .find(|part| part.len() == 4)
            .map(str::to_owned)
    })
}

fn format_doi_url(doi: &str) -> String {
    let doi = doi.trim();
    if doi.starts_with("https://") || doi.starts_with("http://") {
        doi.to_owned()
    } else {
        format!("https://doi.org/{}", doi.trim_start_matches("doi:"))
    }
}

fn push_value(parts: &mut Vec<String>, value: Option<String>) {
    if let Some(value) = value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
    {
        parts.push(value);
    }
}

fn nonempty(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_owned())
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
