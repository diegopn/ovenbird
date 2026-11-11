use super::*;

pub(in crate::app::window) fn reference_style_from_index(index: u32) -> bibtex::ReferenceStyle {
    match index {
        1 => bibtex::ReferenceStyle::Mla,
        2 => bibtex::ReferenceStyle::Ams,
        3 => bibtex::ReferenceStyle::Apa7,
        4 => bibtex::ReferenceStyle::ChicagoAuthorDate,
        5 => bibtex::ReferenceStyle::Harvard,
        6 => bibtex::ReferenceStyle::Vancouver,
        7 => bibtex::ReferenceStyle::Ieee,
        _ => bibtex::ReferenceStyle::Abnt,
    }
}

pub(in crate::app::window) fn show_reference_details(state: &Rc<State>, entry: BibEntry) {
    let dialog = gtk::Dialog::builder()
        .title(tr("Reference details"))
        .transient_for(&state.window)
        .modal(true)
        .default_width(680)
        .default_height(620)
        .build();
    let edit = dialog.add_button(&tr("Edit reference"), gtk::ResponseType::Accept);
    edit.add_css_class("suggested-action");

    let content = dialog.content_area();
    content.set_spacing(12);
    content.set_margin_top(16);
    content.set_margin_bottom(16);
    content.set_margin_start(18);
    content.set_margin_end(18);

    let title = if entry.get("title").trim().is_empty() {
        bibtex::display_bibtex_text(&entry.key)
    } else {
        bibtex::display_bibtex_text(entry.get("title"))
    };
    let title_label = gtk::Label::new(Some(&title));
    title_label.set_xalign(0.0);
    title_label.set_wrap(true);
    title_label.add_css_class("title-3");
    content.append(&title_label);

    let fields = gtk::Box::new(gtk::Orientation::Vertical, 4);
    fields.set_margin_end(30);
    fields.append(&reference_detail_row(
        state,
        &tr("Reference type"),
        &bibtex::reference_type_label(&entry.entry_type),
    ));
    fields.append(&reference_detail_row(
        state,
        &tr("Citation key"),
        &entry.key,
    ));
    for (name, value) in &entry.fields {
        if value.trim().is_empty() {
            continue;
        }
        if name == "year" && !entry.get("date").trim().is_empty() {
            continue;
        }
        let label = bibtex::reference_field_label(name, &entry.entry_type);
        let row = if name == "tags" {
            reference_detail_tags_row(&label, value)
        } else {
            reference_detail_row(state, &label, value)
        };
        fields.append(&row);
    }
    let fields_scroll = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .min_content_height(180)
        .child(&fields)
        .build();
    content.append(&fields_scroll);
    content.append(&gtk::Separator::new(gtk::Orientation::Horizontal));

    let citation_heading = gtk::Label::new(Some(&tr("Cite this reference")));
    citation_heading.set_xalign(0.0);
    citation_heading.add_css_class("heading");
    content.append(&citation_heading);

    let format_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let format_label = gtk::Label::new(Some(&tr("Citation format")));
    format_label.set_xalign(0.0);
    format_row.append(&format_label);
    let format_names = [
        tr("ABNT"),
        tr("MLA"),
        tr("AMS"),
        tr("APA 7"),
        tr("Chicago (author-date)"),
        tr("Harvard (author-date)"),
        tr("Vancouver"),
        tr("IEEE"),
    ];
    let format_name_refs = format_names.iter().map(String::as_str).collect::<Vec<_>>();
    let format_selector = gtk::DropDown::from_strings(&format_name_refs);
    format_selector.set_selected(0);
    format_selector.set_hexpand(true);
    format_row.append(&format_selector);
    content.append(&format_row);

    let citation_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    citation_row.add_css_class("card");
    citation_row.set_margin_top(2);
    citation_row.set_margin_bottom(2);
    citation_row.set_margin_start(2);
    citation_row.set_margin_end(2);
    let citation_text = gtk::Label::new(None);
    citation_text.set_xalign(0.0);
    citation_text.set_yalign(0.0);
    citation_text.set_wrap(true);
    citation_text.set_selectable(true);
    citation_text.set_hexpand(true);
    citation_text.set_margin_top(10);
    citation_text.set_margin_bottom(10);
    citation_text.set_margin_start(10);
    citation_text.set_margin_end(4);
    citation_row.append(&citation_text);
    let copy_citation = button("edit-copy-symbolic", &tr("Copy formatted reference"));
    let citation_for_copy = citation_text.clone();
    let state_for_copy = state.clone();
    copy_citation.connect_clicked(move |_| {
        let text = citation_for_copy.text();
        copy_text_to_clipboard(&state_for_copy, text.as_str());
    });
    citation_row.append(&copy_citation);
    content.append(&citation_row);

    let entry_for_format = entry.clone();
    let citation_for_format = citation_text.clone();
    format_selector.connect_selected_notify(move |selector| {
        let style = reference_style_from_index(selector.selected());
        citation_for_format.set_text(&bibtex::display_bibtex_text(
            &bibtex::format_reference_citation(&entry_for_format, style),
        ));
    });
    citation_text.set_text(&bibtex::display_bibtex_text(
        &bibtex::format_reference_citation(&entry, bibtex::ReferenceStyle::Abnt),
    ));

    let state_for_edit = state.clone();
    dialog.connect_response(move |dialog, response| {
        dialog.close();
        if response == gtk::ResponseType::Accept {
            let state = state_for_edit.clone();
            let entry = entry.clone();
            glib::idle_add_local_once(move || edit_reference(&state, Some(entry)));
        }
    });
    dialog.present();
}

pub(in crate::app::window) fn reference_detail_row(
    state: &Rc<State>,
    name: &str,
    value: &str,
) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    row.set_margin_top(3);
    row.set_margin_bottom(3);
    let name_label = gtk::Label::new(Some(name));
    name_label.set_xalign(0.0);
    name_label.set_width_chars(18);
    name_label.set_valign(gtk::Align::Start);
    name_label.add_css_class("dim-label");
    row.append(&name_label);
    let display_value = bibtex::display_bibtex_text(value);
    let value_label = gtk::Label::new(Some(&display_value));
    value_label.set_xalign(0.0);
    value_label.set_valign(gtk::Align::Start);
    value_label.set_wrap(true);
    value_label.set_selectable(true);
    value_label.set_hexpand(true);
    row.append(&value_label);
    let copy = button("edit-copy-symbolic", &tr("Copy value"));
    copy.set_valign(gtk::Align::Start);
    let state = state.clone();
    let value = display_value;
    copy.connect_clicked(move |_| copy_text_to_clipboard(&state, &value));
    row.append(&copy);
    row
}

pub(in crate::app::window) fn reference_detail_tags_row(name: &str, value: &str) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    row.set_margin_top(3);
    row.set_margin_bottom(3);
    let name_label = gtk::Label::new(Some(name));
    name_label.set_xalign(0.0);
    name_label.set_width_chars(18);
    name_label.set_valign(gtk::Align::Start);
    name_label.add_css_class("dim-label");
    row.append(&name_label);

    let chips = gtk::FlowBox::new();
    chips.set_selection_mode(gtk::SelectionMode::None);
    chips.set_row_spacing(4);
    chips.set_column_spacing(4);
    chips.set_max_children_per_line(8);
    chips.set_min_children_per_line(1);
    chips.set_hexpand(true);
    for tag in parse_tag_values(value) {
        let chip = gtk::Label::new(Some(&bibtex::display_bibtex_text(&tag)));
        chip.add_css_class("reference-detail-tag-chip");
        chip.set_selectable(true);
        chips.append(&chip);
    }
    row.append(&chips);
    row
}

pub(in crate::app::window) fn copy_text_to_clipboard(state: &Rc<State>, text: &str) {
    if let Some(display) = gdk::Display::default() {
        display.clipboard().set_text(text);
        show_toast(state, &tr("Copied to clipboard"));
    } else {
        show_toast(state, &tr("Could not access the clipboard."));
    }
}

pub(in crate::app::window) fn request_reference_deletion(
    state: &Rc<State>,
    parent: &gtk::Window,
    entry: BibEntry,
    after_delete: impl FnOnce() + 'static,
) {
    if !library_is_ready(state) {
        return;
    }
    let name = if entry.get("title").trim().is_empty() {
        bibtex::display_bibtex_text(&entry.key)
    } else {
        bibtex::display_bibtex_text(entry.get("title"))
    };
    let message = tr("Delete reference ‘%s’ from the local library? This cannot be undone.")
        .replacen("%s", &name, 1);
    let key = entry.key;
    confirm_destructive_action(
        state,
        parent,
        &tr("Confirm deletion"),
        &message,
        &tr("Delete"),
        move |state| {
            if !library_is_ready(&state) {
                return;
            }
            let mut updated = state.library.borrow().clone();
            if updated.remove_in_memory(&key) {
                persist_library_async(state, updated, tr("Reference deleted"));
                after_delete();
            }
        },
    );
}
