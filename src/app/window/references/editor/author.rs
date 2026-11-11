use super::*;

pub(super) fn build_reference_author_editor(
    state: &Rc<State>,
    entry: &Option<BibEntry>,
) -> ReferenceAuthorWidgets {
    let selected_authors = Rc::new(RefCell::new(
        entry
            .as_ref()
            .map(|entry| bibtex::split_bibtex_names(entry.get("author")))
            .unwrap_or_default(),
    ));
    let available_authors = Rc::new(RefCell::new(
        state
            .library
            .borrow()
            .authors
            .iter()
            .map(|author| author.citation_name.clone())
            .collect::<Vec<_>>(),
    ));
    for author in selected_authors.borrow().iter() {
        let missing = {
            let available = available_authors.borrow();
            !available.iter().any(|existing| {
                author_selection_identity(existing) == author_selection_identity(author)
            })
        };
        if missing {
            available_authors.borrow_mut().push(author.clone());
        }
    }
    let author_list = gtk::Box::new(gtk::Orientation::Vertical, 4);
    author_list.set_hexpand(true);
    author_list.set_halign(gtk::Align::Fill);
    refresh_reference_author_list(&author_list, &selected_authors);

    let author_editor = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    author_editor.set_hexpand(true);
    author_editor.append(&author_list);

    let author_popover = gtk::Popover::new();
    let author_picker_content = gtk::Box::new(gtk::Orientation::Vertical, 8);
    author_picker_content.set_margin_top(8);
    author_picker_content.set_margin_bottom(8);
    author_picker_content.set_margin_start(8);
    author_picker_content.set_margin_end(8);
    author_picker_content.set_size_request(280, -1);
    let author_search = gtk::SearchEntry::new();
    author_search.set_placeholder_text(Some(&tr("Find authors…")));
    author_picker_content.append(&author_search);
    let author_choices = gtk::Box::new(gtk::Orientation::Vertical, 2);
    author_choices.set_hexpand(true);
    let author_choices_scroll = gtk::ScrolledWindow::builder()
        .min_content_height(36)
        .max_content_height(180)
        .child(&author_choices)
        .build();
    author_picker_content.append(&author_choices_scroll);
    let create_author_row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let new_author_entry = gtk::Entry::new();
    new_author_entry.set_placeholder_text(Some(&tr("New author")));
    new_author_entry.set_hexpand(true);
    let create_author_button = gtk::Button::with_label(&tr("Create"));
    create_author_row.append(&new_author_entry);
    create_author_row.append(&create_author_button);
    author_picker_content.append(&create_author_row);
    author_popover.set_child(Some(&author_picker_content));
    refresh_reference_author_picker(
        &author_choices,
        &author_list,
        &selected_authors,
        &available_authors,
        "",
    );
    author_search.connect_search_changed({
        let author_choices = author_choices.clone();
        let author_list = author_list.clone();
        let selected_authors = selected_authors.clone();
        let available_authors = available_authors.clone();
        move |search| {
            refresh_reference_author_picker(
                &author_choices,
                &author_list,
                &selected_authors,
                &available_authors,
                search.text().as_str(),
            );
        }
    });
    create_author_button.connect_clicked({
        let new_author_entry = new_author_entry.clone();
        let author_choices = author_choices.clone();
        let author_list = author_list.clone();
        let selected_authors = selected_authors.clone();
        let available_authors = available_authors.clone();
        move |_| {
            if add_author_selection(
                new_author_entry.text().as_str(),
                &available_authors,
                &selected_authors,
            ) {
                new_author_entry.set_text("");
                refresh_reference_author_list(&author_list, &selected_authors);
                refresh_reference_author_picker(
                    &author_choices,
                    &author_list,
                    &selected_authors,
                    &available_authors,
                    "",
                );
            }
        }
    });
    create_author_button.set_tooltip_text(Some(&tr("Create and select this author")));
    new_author_entry.connect_activate({
        let create_author_button = create_author_button.clone();
        move |_| create_author_button.emit_clicked()
    });
    let add_author_button = gtk::MenuButton::new();
    add_author_button.set_child(Some(&labeled_content(
        "list-add-symbolic",
        &tr("Add author"),
    )));
    add_author_button.set_tooltip_text(Some(&tr("Select or create authors")));
    add_author_button.set_popover(Some(&author_popover));
    add_author_button.set_valign(gtk::Align::Start);
    add_author_button.connect_active_notify({
        let author_choices = author_choices.clone();
        let author_list = author_list.clone();
        let selected_authors = selected_authors.clone();
        let available_authors = available_authors.clone();
        let author_search = author_search.clone();
        move |button| {
            if button.is_active() {
                refresh_reference_author_picker(
                    &author_choices,
                    &author_list,
                    &selected_authors,
                    &available_authors,
                    author_search.text().as_str(),
                );
            }
        }
    });
    author_editor.append(&add_author_button);
    let author_label = gtk::Label::new(Some(&tr("Authors")));
    author_label.set_xalign(1.0);
    author_label.add_css_class("dim-label");
    ReferenceAuthorWidgets {
        label: author_label,
        editor: author_editor,
        selected: selected_authors,
    }
}
