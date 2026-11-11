use super::*;

pub(in crate::app::window) fn load_library_async(state: &Rc<State>) {
    if state.library_ui.library_loading.replace(true) {
        return;
    }
    state.library_ui.library_load_error.borrow_mut().take();
    refresh_library(state);
    refresh_tags(state);
    refresh_authors(state);
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let result = LocalLibrary::open().map_err(|error| error.to_string());
        let _ = sender.send(result);
    });
    let state = state.clone();
    poll_result(receiver, move |result| match result {
        Ok(library) => {
            state.library_ui.library_loading.set(false);
            *state.library.borrow_mut() = library;
            state.library_ui.library_ready.set(true);
            refresh_library(&state);
            refresh_tags(&state);
            refresh_authors(&state);
        }
        Err(error) => {
            state.library_ui.library_loading.set(false);
            *state.library_ui.library_load_error.borrow_mut() = Some(error.clone());
            show_toast(
                &state,
                &format!("{}: {error}", tr("Could not load the local library")),
            );
            refresh_library(&state);
            refresh_tags(&state);
            refresh_authors(&state);
        }
    });
}

pub(in crate::app::window) fn persist_library_async(
    state: Rc<State>,
    updated: LocalLibrary,
    success_message: String,
) {
    if !state.library_ui.library_ready.get() || state.library_ui.library_busy.replace(true) {
        show_toast(
            &state,
            &tr("The local library is busy. Try again in a moment."),
        );
        return;
    }
    state.editor_ui.status.set_label(&tr("Saving references…"));
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let result = updated.save().map_err(|error| error.to_string());
        let _ = sender.send((result, updated));
    });
    poll_result(receiver, move |(result, updated)| {
        state.library_ui.library_busy.set(false);
        state.editor_ui.status.set_label(&tr("Ready"));
        match result {
            Ok(()) => {
                *state.library.borrow_mut() = updated;
                refresh_library(&state);
                refresh_tags(&state);
                refresh_authors(&state);
                show_toast(&state, &success_message);
            }
            Err(error) => show_toast(
                &state,
                &format!("{}: {error}", tr("Could not save the local library")),
            ),
        }
    });
}

pub(in crate::app::window) fn library_is_ready(state: &Rc<State>) -> bool {
    if !state.library_ui.library_ready.get() {
        show_toast(state, &tr("The local library is still loading."));
        return false;
    }
    if state.library_ui.library_busy.get() {
        show_toast(
            state,
            &tr("The local library is busy. Try again in a moment."),
        );
        return false;
    }
    true
}

pub(in crate::app::window) fn refresh_library(state: &Rc<State>) {
    if !state.library_ui.library_ready.get() {
        if let Some(error) = state.library_ui.library_load_error.borrow().as_ref() {
            set_reference_list_message(
                &state.library_ui.library_list_model,
                format!("{}: {error}", tr("Could not load the local library")),
                true,
            );
        } else {
            set_reference_list_message(
                &state.library_ui.library_list_model,
                tr("Loading local library…"),
                false,
            );
        }
        update_primary_action(state);
        return;
    }
    let term = state.library_ui.library_search.text();
    let library = state.library.borrow();
    let mut filter_options = reference_filter_values(&library.bibliography.entries);
    filter_options.authors =
        reference_filter_author_names(&filter_options.authors, &library.authors);
    filter_options.tags.extend(library.tags.iter().cloned());
    filter_options.tags = sorted_reference_values(filter_options.tags);
    state.library_ui.library_filters_updating.set(true);
    set_reference_filter_options(
        &state.library_ui.reference_filters.author,
        &tr("All authors"),
        filter_options.authors,
    );
    set_reference_filter_options(
        &state.library_ui.reference_filters.year,
        &tr("All years"),
        filter_options.years,
    );
    set_reference_filter_options(
        &state.library_ui.reference_filters.tag,
        &tr("All tags"),
        filter_options.tags,
    );
    set_reference_filter_options(
        &state.library_ui.reference_filters.kind,
        &tr("All types"),
        filter_options.kinds,
    );
    state.library_ui.library_filters_updating.set(false);
    let author_filter = selected_reference_filter(&state.library_ui.reference_filters.author);
    let year_filter = selected_reference_filter(&state.library_ui.reference_filters.year);
    let tag_filter = selected_reference_filter(&state.library_ui.reference_filters.tag);
    let kind_filter = selected_reference_filter(&state.library_ui.reference_filters.kind);
    let filters = ReferenceFilterSelection {
        author: author_filter.as_deref(),
        year: year_filter.as_deref(),
        tag: tag_filter.as_deref(),
        kind: kind_filter.as_deref(),
        author_profiles: &library.authors,
    };
    let entries = filtered_reference_entries(
        &library.bibliography.entries,
        &term,
        &filters,
        state.library_ui.reference_sort.selected(),
        state.library_ui.reference_sort_descending.is_active(),
    );
    if entries.is_empty() {
        let message = if library.bibliography.entries.is_empty() {
            tr("Your library starts here: add a reference or import BibTeX.")
        } else {
            tr("No references match the current search and filters.")
        };
        set_reference_list_message(&state.library_ui.library_list_model, message, false);
    } else {
        replace_reference_list_entries(&state.library_ui.library_list_model, entries);
    }
    update_primary_action(state);
}

pub(in crate::app::window) fn set_reference_list_message(
    model: &gio::ListStore,
    text: String,
    retry: bool,
) {
    let message = glib::BoxedAnyObject::new(ReferenceListItem::Message { text, retry });
    model.splice(0, model.n_items(), &[message]);
}

pub(in crate::app::window) fn replace_reference_list_entries(
    model: &gio::ListStore,
    entries: Vec<BibEntry>,
) {
    let rows = entries
        .into_iter()
        .map(|entry| glib::BoxedAnyObject::new(ReferenceListItem::Entry(entry)))
        .collect::<Vec<_>>();
    model.splice(0, model.n_items(), &rows);
}

pub(in crate::app::window) fn refresh_tags(state: &Rc<State>) {
    while let Some(child) = state.library_ui.tags_list.first_child() {
        state.library_ui.tags_list.remove(&child);
    }
    if !state.library_ui.library_ready.get() {
        let message = if let Some(error) = state.library_ui.library_load_error.borrow().as_ref() {
            format!("{}: {error}", tr("Could not load the local library"))
        } else {
            tr("Loading local library…")
        };
        let label = gtk::Label::new(Some(&message));
        label.set_xalign(0.0);
        label.set_wrap(true);
        label.set_margin_top(24);
        label.add_css_class("dim-label");
        state.library_ui.tags_list.append(&label);
        return;
    }

    let tags = state.library.borrow().tags.clone();
    if tags.is_empty() {
        let empty = gtk::Label::new(Some(&tr(
            "No tags yet. Add a tag to organize your references.",
        )));
        empty.set_xalign(0.0);
        empty.set_margin_top(18);
        empty.add_css_class("dim-label");
        state.library_ui.tags_list.append(&empty);
        return;
    }

    let query = state.library_ui.tag_search.text().to_lowercase();
    let mut tags = tags
        .into_iter()
        .filter(|tag| query.is_empty() || tag.to_lowercase().contains(&query))
        .collect::<Vec<_>>();
    tags.sort_by_key(|tag| tag.to_lowercase());
    if state.library_ui.tag_sort_descending.is_active() {
        tags.reverse();
    }
    if tags.is_empty() {
        let empty = gtk::Label::new(Some(&tr("No tags match your search.")));
        empty.set_xalign(0.0);
        empty.set_margin_top(18);
        empty.add_css_class("dim-label");
        state.library_ui.tags_list.append(&empty);
        return;
    }

    for tag in tags {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        row.set_margin_top(3);
        row.set_margin_bottom(3);
        let label = gtk::Label::new(Some(&tag));
        label.set_xalign(0.0);
        label.set_hexpand(true);
        row.append(&label);

        let edit = button("document-edit-symbolic", &tr("Edit tag"));
        let state_for_edit = state.clone();
        let tag_for_edit = tag.clone();
        edit.connect_clicked(move |_| edit_tag(&state_for_edit, Some(tag_for_edit.clone())));
        row.append(&edit);

        let delete = button("user-trash-symbolic", &tr("Remove tag"));
        delete.add_css_class("destructive-action");
        let state_for_delete = state.clone();
        let tag_for_delete = tag.clone();
        delete.connect_clicked(move |_| request_tag_deletion(&state_for_delete, &tag_for_delete));
        row.append(&delete);
        state.library_ui.tags_list.append(&row);
        state
            .tags_list
            .append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    }
}

pub(in crate::app::window) fn refresh_authors(state: &Rc<State>) {
    while let Some(child) = state.library_ui.authors_list.first_child() {
        state.library_ui.authors_list.remove(&child);
    }
    if !state.library_ui.library_ready.get() {
        let message = if let Some(error) = state.library_ui.library_load_error.borrow().as_ref() {
            format!("{}: {error}", tr("Could not load the local library"))
        } else {
            tr("Loading local library…")
        };
        let label = gtk::Label::new(Some(&message));
        label.set_xalign(0.0);
        label.set_wrap(true);
        label.set_margin_top(24);
        label.add_css_class("dim-label");
        state.library_ui.authors_list.append(&label);
        return;
    }

    let search = state.library_ui.author_search.text().to_lowercase();
    let library = state.library.borrow();
    let mut authors = library.authors.clone();
    authors.retain(|author| {
        search.is_empty()
            || author.full_name.to_lowercase().contains(&search)
            || bibtex::display_bibtex_text(&author.citation_name)
                .to_lowercase()
                .contains(&search)
            || author.orcid.to_lowercase().contains(&search)
            || author.email.to_lowercase().contains(&search)
            || author.institution.to_lowercase().contains(&search)
    });
    authors.sort_by(|left, right| {
        let primary = match state.library_ui.author_sort.selected() {
            1 => left
                .institution
                .to_lowercase()
                .cmp(&right.institution.to_lowercase()),
            _ => left
                .full_name
                .to_lowercase()
                .cmp(&right.full_name.to_lowercase()),
        };
        let primary = if state.library_ui.author_sort_descending.is_active() {
            primary.reverse()
        } else {
            primary
        };
        primary
            .then_with(|| {
                left.full_name
                    .to_lowercase()
                    .cmp(&right.full_name.to_lowercase())
            })
            .then_with(|| {
                left.citation_name
                    .to_lowercase()
                    .cmp(&right.citation_name.to_lowercase())
            })
    });
    drop(library);
    if authors.is_empty() {
        let message = if search.is_empty() {
            tr("No authors yet. Add authors to organize your references.")
        } else {
            tr("No authors match this search.")
        };
        let empty = gtk::Label::new(Some(&message));
        empty.set_xalign(0.0);
        empty.set_margin_top(18);
        empty.add_css_class("dim-label");
        state.library_ui.authors_list.append(&empty);
        return;
    }

    for author in authors {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        row.set_margin_top(3);
        row.set_margin_bottom(3);
        let labels = gtk::Box::new(gtk::Orientation::Vertical, 2);
        labels.set_hexpand(true);
        let full_name = gtk::Label::new(Some(&author.full_name));
        full_name.set_xalign(0.0);
        full_name.set_ellipsize(gtk::pango::EllipsizeMode::End);
        labels.append(&full_name);
        let reference_count = state
            .library
            .borrow()
            .references_for_author(&author.citation_name)
            .len();
        let mut details = vec![ngettext("%d reference", "%d references", reference_count)
            .replace("%d", &reference_count.to_string())];
        if !author.email.trim().is_empty() {
            details.push(author.email.trim().to_owned());
        }
        if !author.orcid.trim().is_empty() {
            details.push(format!("{}: {}", tr("ORCID"), author.orcid.trim()));
        }
        let details = gtk::Label::new(Some(&details.join(" · ")));
        details.set_xalign(0.0);
        details.set_wrap(true);
        details.set_wrap_mode(gtk::pango::WrapMode::WordChar);
        details.add_css_class("dim-label");
        labels.append(&details);
        let open_details = gtk::Button::new();
        open_details.set_has_frame(false);
        open_details.add_css_class("flat");
        open_details.set_hexpand(true);
        open_details.set_halign(gtk::Align::Fill);
        open_details.set_child(Some(&labels));
        open_details.set_tooltip_text(Some(&tr("Author details")));
        let state_for_details = state.clone();
        let author_for_details = author.citation_name.clone();
        open_details.connect_clicked(move |_| {
            show_author_details(&state_for_details, &author_for_details);
        });
        row.append(&open_details);

        let edit = button("document-edit-symbolic", &tr("Edit author"));
        let state_for_edit = state.clone();
        let author_for_edit = author.citation_name.clone();
        edit.connect_clicked(move |_| {
            edit_author(&state_for_edit, Some(author_for_edit.clone()));
        });
        row.append(&edit);

        let delete = button("user-trash-symbolic", &tr("Delete author"));
        delete.add_css_class("destructive-action");
        let state_for_delete = state.clone();
        let author_for_delete = author.citation_name.clone();
        delete.connect_clicked(move |_| {
            request_author_deletion(&state_for_delete, &author_for_delete);
        });
        row.append(&delete);
        state.library_ui.authors_list.append(&row);
        state
            .authors_list
            .append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    }
}
