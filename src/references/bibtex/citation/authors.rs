use super::*;

pub(super) fn format_authors(value: &str, style: ReferenceStyle) -> String {
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

pub(super) fn format_author_name(name: &str, style: ReferenceStyle) -> String {
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
        (
            words[..family_start].join(" "),
            words[family_start..].join(" "),
        )
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
        ReferenceStyle::Vancouver => format!("{} {}", family, initials(&given).replace('.', ""))
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
        "da" | "das"
            | "de"
            | "del"
            | "della"
            | "di"
            | "do"
            | "dos"
            | "du"
            | "la"
            | "le"
            | "van"
            | "von"
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
