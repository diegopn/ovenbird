use super::*;

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
    let class_pattern = Regex::new(r"(?i)\\documentclass(?:\s*\[[^\]]*\])?\s*\{([^}]+)\}").unwrap();
    let bibliography_pattern = Regex::new(r"(?i)\\bibliographystyle\s*\{([^}]+)\}").unwrap();
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
