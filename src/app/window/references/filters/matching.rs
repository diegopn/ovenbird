use super::*;

pub(in crate::app::window) fn reference_filter_values(
    entries: &[BibEntry],
) -> ReferenceFilterOptions {
    let authors = sorted_reference_values(entries.iter().flat_map(reference_authors));
    let years = sorted_reference_values(entries.iter().filter_map(reference_year));
    let tags = sorted_reference_values(entries.iter().flat_map(reference_tags));
    let kinds = sorted_reference_values(
        entries
            .iter()
            .map(|entry| bibtex::reference_type_label(&entry.entry_type)),
    );
    ReferenceFilterOptions {
        authors,
        years,
        tags,
        kinds,
    }
}

pub(in crate::app::window) fn reference_filter_author_names(
    citation_names: &[String],
    profiles: &[AuthorProfile],
) -> Vec<String> {
    sorted_reference_values(citation_names.iter().map(|citation_name| {
        profiles
            .iter()
            .find(|profile| {
                author_selection_identity(&profile.citation_name)
                    == author_selection_identity(citation_name)
            })
            .map(|profile| bibtex::display_bibtex_text(&profile.full_name))
            .filter(|full_name| !full_name.trim().is_empty())
            .unwrap_or_else(|| bibtex::display_bibtex_text(citation_name))
    }))
}

pub(in crate::app::window) fn sorted_reference_values(
    values: impl IntoIterator<Item = String>,
) -> Vec<String> {
    let mut values = values
        .into_iter()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    values.sort_by_key(|value| value.to_lowercase());
    values
}

pub(in crate::app::window) fn reference_authors(entry: &BibEntry) -> Vec<String> {
    let author = if entry.get("author").trim().is_empty() {
        entry.get("editor")
    } else {
        entry.get("author")
    };
    bibtex::display_bibtex_names(author)
}

pub(in crate::app::window) fn reference_year(entry: &BibEntry) -> Option<String> {
    let year = entry.get("year").trim();
    if !year.is_empty() {
        return Some(year.to_owned());
    }
    let date = entry.get("date").trim();
    let year = date.chars().take(4).collect::<String>();
    (year.len() == 4 && year.chars().all(|character| character.is_ascii_digit())).then_some(year)
}

pub(in crate::app::window) fn reference_tags(entry: &BibEntry) -> Vec<String> {
    ["keywords", "tags"]
        .into_iter()
        .flat_map(|field| entry.get(field).split([',', ';']))
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
pub(in crate::app::window) fn reference_matches_filters(
    entry: &BibEntry,
    author: Option<&str>,
    year: Option<&str>,
    tag: Option<&str>,
) -> bool {
    let filters = ReferenceFilterSelection {
        author,
        year,
        tag,
        kind: None,
        author_profiles: &[],
    };
    reference_matches_filters_with_type(entry, &filters)
}

pub(in crate::app::window) fn reference_matches_filters_with_type(
    entry: &BibEntry,
    filters: &ReferenceFilterSelection<'_>,
) -> bool {
    let author_matches = filters.author.is_none_or(|selected| {
        let selected_identity = author_selection_identity(selected);
        reference_authors(entry).iter().any(|candidate| {
            let candidate_identity = author_selection_identity(candidate);
            candidate_identity == selected_identity
                || filters.author_profiles.iter().any(|profile| {
                    author_selection_identity(&profile.citation_name) == candidate_identity
                        && author_selection_identity(&profile.full_name) == selected_identity
                })
        })
    });
    let year_matches = filters.year.is_none_or(|selected| {
        reference_year(entry).is_some_and(|candidate| candidate.eq_ignore_ascii_case(selected))
    });
    let tag_matches = filters.tag.is_none_or(|selected| {
        reference_tags(entry)
            .iter()
            .any(|candidate| candidate.eq_ignore_ascii_case(selected))
    });
    let kind_matches = filters.kind.is_none_or(|selected| {
        bibtex::reference_type_label(&entry.entry_type).eq_ignore_ascii_case(selected)
    });
    author_matches && year_matches && tag_matches && kind_matches
}

pub(in crate::app::window) fn filtered_reference_entries(
    entries: &[BibEntry],
    search: &str,
    filters: &ReferenceFilterSelection<'_>,
    sort_by: u32,
    descending: bool,
) -> Vec<BibEntry> {
    let mut entries = entries
        .iter()
        .filter(|entry| bibtex::entry_matches_search(entry, search))
        .filter(|entry| reference_matches_filters_with_type(entry, filters))
        .cloned()
        .collect::<Vec<_>>();
    sort_reference_entries(&mut entries, sort_by, descending);
    entries
}

pub(in crate::app::window) fn sort_reference_entries(
    entries: &mut [BibEntry],
    sort_by: u32,
    descending: bool,
) {
    entries.sort_by(|left, right| {
        let primary = match sort_by {
            1 => reference_authors(left)
                .join(" ")
                .to_lowercase()
                .cmp(&reference_authors(right).join(" ").to_lowercase()),
            2 => reference_year(left)
                .and_then(|year| year.parse::<u32>().ok())
                .cmp(&reference_year(right).and_then(|year| year.parse::<u32>().ok())),
            3 => reference_tag_sort_key(left).cmp(&reference_tag_sort_key(right)),
            _ => reference_title(left)
                .to_lowercase()
                .cmp(&reference_title(right).to_lowercase()),
        };
        let primary = if descending {
            primary.reverse()
        } else {
            primary
        };
        primary
            .then_with(|| {
                reference_title(left)
                    .to_lowercase()
                    .cmp(&reference_title(right).to_lowercase())
            })
            .then_with(|| left.key.to_lowercase().cmp(&right.key.to_lowercase()))
    });
}

pub(in crate::app::window) fn reference_tag_sort_key(entry: &BibEntry) -> Vec<String> {
    let mut tags = reference_tags(entry)
        .into_iter()
        .map(|tag| tag.to_lowercase())
        .collect::<Vec<_>>();
    tags.sort();
    tags.dedup();
    tags
}

pub(in crate::app::window) fn reference_title(entry: &BibEntry) -> String {
    if entry.get("title").trim().is_empty() {
        bibtex::display_bibtex_text(&entry.key)
    } else {
        bibtex::display_bibtex_text(entry.get("title"))
    }
}

pub(in crate::app::window) fn selected_reference_filter(
    dropdown: &gtk::DropDown,
) -> Option<String> {
    if dropdown.selected() == 0 || dropdown.selected() == gtk::INVALID_LIST_POSITION {
        return None;
    }
    dropdown
        .selected_item()?
        .downcast::<gtk::StringObject>()
        .ok()
        .map(|item| item.string().to_string())
}

pub(in crate::app::window) fn set_reference_filter_options(
    dropdown: &gtk::DropDown,
    all_label: &str,
    values: Vec<String>,
) {
    let mut options = vec![all_label.to_owned()];
    options.extend(values);

    let options_unchanged = dropdown
        .model()
        .and_then(|model| model.downcast::<gtk::StringList>().ok())
        .is_some_and(|model| {
            model.n_items() as usize == options.len()
                && options.iter().enumerate().all(|(index, option)| {
                    model
                        .string(index as u32)
                        .is_some_and(|current| current.as_str() == option)
                })
        });
    if options_unchanged {
        return;
    }

    let selection = selected_reference_filter(dropdown);
    let model_strings = options.iter().map(String::as_str).collect::<Vec<_>>();
    let model = gtk::StringList::new(&model_strings);
    dropdown.set_model(Some(&model));
    let selected = selection
        .and_then(|value| options.iter().position(|option| option == &value))
        .unwrap_or(0);
    dropdown.set_selected(selected as u32);
}
