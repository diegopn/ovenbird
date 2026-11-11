use super::*;

pub(in crate::app::window) fn author_form_row(content: &gtk::Box, label: &str, input: &gtk::Entry) {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let label = gtk::Label::new(Some(label));
    label.set_xalign(1.0);
    label.set_width_chars(20);
    row.append(&label);
    input.set_hexpand(true);
    row.append(input);
    content.append(&row);
}

pub(in crate::app::window) fn edit_author(state: &Rc<State>, previous: Option<String>) {
    if !library_is_ready(state) {
        return;
    }
    let dialog = gtk::Dialog::builder()
        .title(tr(if previous.is_some() {
            "Edit author"
        } else {
            "Add author"
        }))
        .transient_for(&state.window)
        .modal(true)
        .default_width(580)
        .build();
    dialog.add_button(&tr("Cancel"), gtk::ResponseType::Cancel);
    let save = dialog.add_button(&tr("Save"), gtk::ResponseType::Accept);
    save.add_css_class("suggested-action");
    dialog.set_default_response(gtk::ResponseType::Accept);

    let content = gtk::Box::new(gtk::Orientation::Vertical, 8);
    content.set_margin_top(18);
    content.set_margin_bottom(18);
    content.set_margin_start(20);
    content.set_margin_end(20);
    let existing_profile = previous.as_ref().and_then(|previous| {
        state
            .library
            .borrow()
            .authors
            .iter()
            .find(|author| {
                author_selection_identity(&author.citation_name)
                    == author_selection_identity(previous)
            })
            .cloned()
    });
    let full_name = gtk::Entry::new();
    full_name.set_placeholder_text(Some(&tr("Full name")));
    full_name.set_activates_default(true);
    full_name.set_text(
        existing_profile
            .as_ref()
            .map(|profile| profile.full_name.as_str())
            .unwrap_or_default(),
    );
    author_form_row(&content, &tr("Full name"), &full_name);

    let citation_name = gtk::Entry::new();
    citation_name.set_placeholder_text(Some(&tr("Name used in references")));
    citation_name.set_activates_default(true);
    let initial_citation_display = existing_profile
        .as_ref()
        .map(|profile| bibtex::display_bibtex_text(&profile.citation_name))
        .unwrap_or_default();
    citation_name.set_text(&initial_citation_display);
    let citation_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let citation_label = gtk::Label::new(Some(&tr("Name used in references")));
    citation_label.set_xalign(1.0);
    citation_label.set_width_chars(20);
    citation_row.append(&citation_label);
    citation_name.set_hexpand(true);
    citation_row.append(&citation_name);
    let generate = labeled_button("document-edit-symbolic", &tr("Generate"));
    citation_row.append(&generate);
    content.append(&citation_row);

    let orcid = gtk::Entry::new();
    orcid.set_placeholder_text(Some(&tr("ORCID")));
    orcid.set_text(
        existing_profile
            .as_ref()
            .map(|profile| profile.orcid.as_str())
            .unwrap_or_default(),
    );
    author_form_row(&content, &tr("ORCID"), &orcid);

    let email = gtk::Entry::new();
    email.set_placeholder_text(Some(&tr("Email")));
    email.set_text(
        existing_profile
            .as_ref()
            .map(|profile| profile.email.as_str())
            .unwrap_or_default(),
    );
    author_form_row(&content, &tr("Email"), &email);

    let institution = gtk::Entry::new();
    institution.set_placeholder_text(Some(&tr("Institution")));
    institution.set_text(
        existing_profile
            .as_ref()
            .map(|profile| profile.institution.as_str())
            .unwrap_or_default(),
    );
    author_form_row(&content, &tr("Institution"), &institution);

    let citation_was_manually_edited = Rc::new(Cell::new(false));
    let updating_citation = Rc::new(Cell::new(false));
    {
        let citation_was_manually_edited = citation_was_manually_edited.clone();
        let updating_citation = updating_citation.clone();
        citation_name.connect_changed(move |_| {
            if !updating_citation.get() {
                citation_was_manually_edited.set(true);
            }
        });
    }
    {
        let citation_name = citation_name.clone();
        let citation_was_manually_edited = citation_was_manually_edited.clone();
        let updating_citation = updating_citation.clone();
        full_name.connect_changed(move |input| {
            if citation_was_manually_edited.get() {
                return;
            }
            let generated = AuthorProfile::from_full_name(input.text().as_str()).citation_name;
            updating_citation.set(true);
            citation_name.set_text(&generated);
            updating_citation.set(false);
        });
    }
    {
        let full_name = full_name.clone();
        let citation_name = citation_name.clone();
        let citation_was_manually_edited = citation_was_manually_edited.clone();
        let updating_citation = updating_citation.clone();
        generate.connect_clicked(move |_| {
            citation_was_manually_edited.set(false);
            let generated = AuthorProfile::from_full_name(full_name.text().as_str()).citation_name;
            updating_citation.set(true);
            citation_name.set_text(&generated);
            updating_citation.set(false);
        });
    }
    let original_citation = existing_profile
        .as_ref()
        .map(|profile| profile.citation_name.clone());
    dialog.content_area().append(&content);

    let previous_for_response = previous.clone();
    let state_for_response = state.clone();
    let full_name_for_response = full_name.clone();
    let citation_name_for_response = citation_name.clone();
    let orcid_for_response = orcid.clone();
    let email_for_response = email.clone();
    let institution_for_response = institution.clone();
    let original_citation_for_response = original_citation.clone();
    let initial_citation_for_response = initial_citation_display.clone();
    dialog.connect_response(move |dialog, response| {
        if response == gtk::ResponseType::Accept {
            let mut updated = state_for_response.library.borrow().clone();
            let entered_citation = citation_name_for_response.text().to_string();
            let citation_name = if original_citation_for_response.is_some()
                && entered_citation == initial_citation_for_response
            {
                original_citation_for_response.clone().unwrap_or_default()
            } else {
                entered_citation
            };
            let profile = AuthorProfile {
                full_name: full_name_for_response.text().to_string(),
                citation_name,
                orcid: orcid_for_response.text().to_string(),
                email: email_for_response.text().to_string(),
                institution: institution_for_response.text().to_string(),
            };
            let result = if let Some(previous) = previous_for_response.as_deref() {
                updated.update_author_profile(previous, profile)
            } else {
                updated.add_author_profile(profile)
            };
            match result {
                Ok(()) => {
                    persist_library_async(state_for_response.clone(), updated, tr("Author saved"));
                    dialog.close();
                }
                Err(error) => show_toast(&state_for_response, &tr(&error)),
            }
        } else {
            dialog.close();
        }
    });
    dialog.present();
    full_name.grab_focus();
}

pub(in crate::app::window) fn show_author_details(state: &Rc<State>, citation_name: &str) {
    let profile = state
        .library
        .borrow()
        .authors
        .iter()
        .find(|author| {
            author_selection_identity(&author.citation_name)
                == author_selection_identity(citation_name)
        })
        .cloned();
    let Some(profile) = profile else {
        return;
    };
    let reference_count = state
        .library
        .borrow()
        .references_for_author(&profile.citation_name)
        .len();
    let dialog = gtk::Dialog::builder()
        .title(tr("Author details"))
        .transient_for(&state.window)
        .modal(true)
        .default_width(600)
        .build();
    let edit = dialog.add_button(&tr("Edit author"), gtk::ResponseType::Accept);
    edit.add_css_class("suggested-action");
    let content = gtk::Box::new(gtk::Orientation::Vertical, 5);
    content.set_margin_top(16);
    content.set_margin_bottom(16);
    content.set_margin_start(18);
    content.set_margin_end(18);
    content.append(&author_detail_row(
        state,
        &tr("Full name"),
        &profile.full_name,
    ));
    content.append(&author_detail_row(
        state,
        &tr("Name used in references"),
        &profile.citation_name,
    ));
    content.append(&author_detail_row(state, &tr("ORCID"), &profile.orcid));
    content.append(&author_detail_row(state, &tr("Email"), &profile.email));
    content.append(&author_detail_row(
        state,
        &tr("Institution"),
        &profile.institution,
    ));
    content.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    let count = gtk::Label::new(Some(
        &ngettext("%d reference", "%d references", reference_count)
            .replace("%d", &reference_count.to_string()),
    ));
    count.set_xalign(0.0);
    count.set_margin_top(4);
    content.append(&count);
    dialog.content_area().append(&content);
    let state_for_edit = state.clone();
    let author_for_edit = profile.citation_name.clone();
    dialog.connect_response(move |dialog, response| {
        dialog.close();
        if response == gtk::ResponseType::Accept {
            let state = state_for_edit.clone();
            let author = author_for_edit.clone();
            glib::idle_add_local_once(move || edit_author(&state, Some(author)));
        }
    });
    dialog.present();
}

pub(in crate::app::window) fn author_detail_row(
    state: &Rc<State>,
    name: &str,
    value: &str,
) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    row.set_margin_top(3);
    row.set_margin_bottom(3);
    let name_label = gtk::Label::new(Some(name));
    name_label.set_xalign(0.0);
    name_label.set_width_chars(24);
    name_label.set_valign(gtk::Align::Start);
    name_label.add_css_class("dim-label");
    row.append(&name_label);
    let display_value = bibtex::display_bibtex_text(value);
    let value_label = gtk::Label::new(Some(if display_value.is_empty() {
        "—"
    } else {
        &display_value
    }));
    value_label.set_xalign(0.0);
    value_label.set_valign(gtk::Align::Start);
    value_label.set_wrap(true);
    value_label.set_selectable(true);
    value_label.set_hexpand(true);
    row.append(&value_label);
    let copy = button("edit-copy-symbolic", &tr("Copy value"));
    copy.set_valign(gtk::Align::Start);
    copy.set_sensitive(!display_value.is_empty());
    let state = state.clone();
    copy.connect_clicked(move |_| copy_text_to_clipboard(&state, &display_value));
    row.append(&copy);
    row
}

pub(in crate::app::window) fn request_author_deletion(state: &Rc<State>, author: &str) {
    if !library_is_ready(state) {
        return;
    }
    let library = state.library.borrow();
    let references = library.references_for_author(author);
    drop(library);

    if references.is_empty() {
        let mut updated = state.library.borrow().clone();
        if updated.remove_author(author, false).is_ok() {
            persist_library_async(state.clone(), updated, tr("Author deleted"));
        }
        return;
    }

    let dialog = gtk::Dialog::builder()
        .title(tr("Author has associated references"))
        .transient_for(&state.window)
        .modal(true)
        .default_width(600)
        .default_height(420)
        .build();
    dialog.add_button(&tr("Cancel"), gtk::ResponseType::Cancel);
    let delete = dialog.add_button(
        &tr("Delete author and references"),
        gtk::ResponseType::Accept,
    );
    delete.add_css_class("destructive-action");

    let content = gtk::Box::new(gtk::Orientation::Vertical, 10);
    content.set_margin_top(16);
    content.set_margin_bottom(16);
    content.set_margin_start(18);
    content.set_margin_end(18);
    let author_name = bibtex::display_bibtex_text(author);
    let explanation = tr("The author ‘%s’ is associated with these references. Delete the author and all listed references?")
        .replacen("%s", &author_name, 1);
    let label = gtk::Label::new(Some(&explanation));
    label.set_xalign(0.0);
    label.set_wrap(true);
    content.append(&label);

    let titles = gtk::Box::new(gtk::Orientation::Vertical, 4);
    for entry in &references {
        let title = if entry.get("title").trim().is_empty() {
            bibtex::display_bibtex_text(&entry.key)
        } else {
            bibtex::display_bibtex_text(entry.get("title"))
        };
        let item = gtk::Label::new(Some(&format!("• {title}")));
        item.set_xalign(0.0);
        item.set_wrap(true);
        item.set_selectable(true);
        titles.append(&item);
    }
    let scroll = gtk::ScrolledWindow::builder()
        .hexpand(true)
        .vexpand(true)
        .min_content_height(100)
        .child(&titles)
        .build();
    content.append(&scroll);
    dialog.content_area().append(&content);

    let author = author.to_owned();
    let state_for_response = state.clone();
    dialog.connect_response(move |dialog, response| {
        if response == gtk::ResponseType::Accept {
            let mut updated = state_for_response.library.borrow().clone();
            match updated.remove_author(&author, true) {
                Ok(count) => persist_library_async(
                    state_for_response.clone(),
                    updated,
                    tr("Deleted author and %d references").replace("%d", &count.to_string()),
                ),
                Err(error) => show_toast(&state_for_response, &tr(&error)),
            }
        }
        dialog.close();
    });
    dialog.present();
}
