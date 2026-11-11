use super::*;

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

    let citation = parts.join(if style == ReferenceStyle::Ieee {
        ", "
    } else {
        ". "
    });
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
