use super::parser::four_digit_year;
use super::*;

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
