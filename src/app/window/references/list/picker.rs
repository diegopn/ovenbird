use super::*;

pub(in crate::app::window) fn show_citation_picker(state: Rc<State>) {
    show_reference_picker(state, ReferencePickerTarget::Latex);
}

pub(in crate::app::window) fn show_bibtex_reference_picker(state: Rc<State>) {
    show_reference_picker(state, ReferencePickerTarget::Bibtex);
}

pub(in crate::app::window) fn show_reference_picker(
    state: Rc<State>,
    target: ReferencePickerTarget,
) {
    show_page(&state, "editor");
    match target {
        ReferencePickerTarget::Latex if !supports_latex_document(&state) => {
            show_toast(
                &state,
                &tr("Open a LaTeX document before inserting a citation."),
            );
            return;
        }
        ReferencePickerTarget::Bibtex
            if current_file_kind(&state) != Some(ProjectFileKind::Bibtex) =>
        {
            show_toast(
                &state,
                &tr("Open a BibTeX file before adding a bibliographic reference."),
            );
            return;
        }
        _ => {}
    }
    let dialog = gtk::Dialog::builder()
        .title(tr(match target {
            ReferencePickerTarget::Latex => "Insert citation",
            ReferencePickerTarget::Bibtex => "Add bibliographic reference",
        }))
        .transient_for(&state.window)
        .modal(true)
        .build();
    dialog.add_button(&tr("Close"), gtk::ResponseType::Close);
    dialog.set_default_size(580, 560);
    let dialog_content = dialog.content_area();
    dialog_content.set_spacing(8);
    let list = gtk::Box::new(gtk::Orientation::Vertical, 2);
    list.set_margin_start(10);
    list.set_margin_end(10);
    let query = gtk::SearchEntry::new();
    query.set_placeholder_text(Some(&tr("Search references…")));
    query.set_margin_top(10);
    query.set_margin_start(10);
    query.set_margin_end(10);
    dialog_content.append(&query);

    let insert_choice = Rc::new(Cell::new(LatexInsertChoice::Citation));
    let style_selector = if target == ReferencePickerTarget::Latex {
        let choice_row = gtk::FlowBox::new();
        choice_row.set_selection_mode(gtk::SelectionMode::None);
        choice_row.set_min_children_per_line(1);
        choice_row.set_max_children_per_line(2);
        choice_row.set_column_spacing(12);
        choice_row.set_margin_start(10);
        choice_row.set_margin_end(10);

        let citation_radio = gtk::CheckButton::with_label(&tr("Citation"));
        citation_radio.set_active(true);
        let bibliography_radio = gtk::CheckButton::with_label(&tr("Bibliographic reference"));
        bibliography_radio.set_group(Some(&citation_radio));
        choice_row.append(&citation_radio);
        choice_row.append(&bibliography_radio);
        dialog_content.append(&choice_row);

        let format_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        format_row.set_margin_start(10);
        format_row.set_margin_end(10);
        let format_label = gtk::Label::new(Some(&tr("Bibliographic reference format")));
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
        let selector = gtk::DropDown::from_strings(&format_name_refs);
        selector.set_selected(0);
        selector.set_hexpand(true);
        format_row.append(&selector);
        format_row.set_visible(false);
        dialog_content.append(&format_row);

        let choice_for_citation = insert_choice.clone();
        citation_radio.connect_toggled(move |radio| {
            if radio.is_active() {
                choice_for_citation.set(LatexInsertChoice::Citation);
            }
        });
        let choice_for_bibliography = insert_choice.clone();
        bibliography_radio.connect_toggled(move |radio| {
            if radio.is_active() {
                choice_for_bibliography.set(LatexInsertChoice::Bibliography);
            }
            format_row.set_visible(radio.is_active());
        });
        Some(selector)
    } else {
        None
    };

    let scroll = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .child(&list)
        .build();
    dialog_content.append(&scroll);
    let fill = Rc::new(RefCell::new(None::<Box<dyn Fn(&str)>>));
    *fill.borrow_mut() = Some(Box::new({
        let state = state.clone();
        let dialog = dialog.clone();
        let list = list.clone();
        let insert_choice = insert_choice.clone();
        let style_selector = style_selector.clone();
        move |term: &str| {
            while let Some(child) = list.first_child() {
                list.remove(&child);
            }
            let library_is_empty = state.library.borrow().bibliography.entries.is_empty();
            let term_lower = term.trim().to_lowercase();
            for entry in state.library.borrow().bibliography.entries.iter() {
                let type_label = bibtex::reference_type_label(&entry.entry_type);
                if !bibtex::entry_matches_search(entry, term)
                    && !type_label.to_lowercase().contains(&term_lower)
                    && !entry.entry_type.to_lowercase().contains(&term_lower)
                {
                    continue;
                }
                let item = gtk::Button::new();
                item.set_has_frame(false);
                item.set_halign(gtk::Align::Fill);
                let content = gtk::Box::new(gtk::Orientation::Vertical, 2);
                content.set_hexpand(true);
                let title_row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
                let display_title = bibtex::display_bibtex_text(if entry.get("title").is_empty() {
                    &entry.key
                } else {
                    entry.get("title")
                });
                let title = gtk::Label::new(Some(&display_title));
                title.set_xalign(0.0);
                title.set_hexpand(true);
                title.set_wrap(true);
                title.add_css_class("heading");
                title_row.append(&title);
                let key = gtk::Label::new(Some(&entry.key));
                key.add_css_class("dim-label");
                key.set_valign(gtk::Align::Start);
                title_row.append(&key);
                content.append(&title_row);

                let author_value = if entry.get("author").trim().is_empty() {
                    entry.get("editor")
                } else {
                    entry.get("author")
                };
                let authors = bibtex::display_bibtex_names(author_value).join(" · ");
                let authors_label = gtk::Label::new(Some(&authors));
                authors_label.set_xalign(0.0);
                authors_label.set_hexpand(true);
                authors_label.set_wrap(true);
                authors_label.add_css_class("dim-label");
                authors_label.set_visible(!authors.is_empty());
                content.append(&authors_label);

                let type_text = tr("Type: %s").replacen("%s", &type_label, 1);
                let type_label = gtk::Label::new(Some(&type_text));
                type_label.set_xalign(0.0);
                type_label.add_css_class("dim-label");
                content.append(&type_label);
                item.set_child(Some(&content));
                let state2 = state.clone();
                let entry_value = entry.clone();
                let dialog = dialog.clone();
                let insert_choice = insert_choice.clone();
                let style_selector = style_selector.clone();
                item.connect_clicked(move |_| {
                    let result = match target {
                        ReferencePickerTarget::Latex => match insert_choice.get() {
                            LatexInsertChoice::Citation => {
                                insert_latex_citation(&state2, &entry_value)
                            }
                            LatexInsertChoice::Bibliography => {
                                let style = style_selector
                                    .as_ref()
                                    .map(|selector| reference_style_from_index(selector.selected()))
                                    .unwrap_or(bibtex::ReferenceStyle::Abnt);
                                insert_latex_bibliography_reference(&state2, &entry_value, style)
                            }
                        },
                        ReferencePickerTarget::Bibtex => {
                            insert_bibtex_reference(&state2, &entry_value)
                        }
                    };
                    if let Err(error) = result {
                        show_toast(
                            &state2,
                            &format!(
                                "{}: {error}",
                                tr(match target {
                                    ReferencePickerTarget::Latex => {
                                        "Could not add the reference to the document"
                                    }
                                    ReferencePickerTarget::Bibtex => {
                                        "Could not add the reference to the BibTeX file"
                                    }
                                })
                            ),
                        );
                        return;
                    }
                    dialog.close();
                    show_page(&state2, "editor");
                });
                list.append(&item);
            }
            if list.first_child().is_none() {
                let empty = gtk::Label::new(Some(&tr("No matching references")));
                empty.set_margin_top(20);
                list.append(&empty);
                if library_is_empty {
                    let add = labeled_button("list-add-symbolic", &tr("Add reference"));
                    let state_for_add = state.clone();
                    let dialog_for_add = dialog.clone();
                    add.connect_clicked(move |_| {
                        dialog_for_add.close();
                        show_page(
                            &state_for_add,
                            if target == ReferencePickerTarget::Latex {
                                "references"
                            } else {
                                "editor"
                            },
                        );
                        edit_reference(&state_for_add, None);
                    });
                    list.append(&add);
                    let import = labeled_button("document-open-symbolic", &tr("Import BibTeX"));
                    let state_for_import = state.clone();
                    let dialog_for_import = dialog.clone();
                    import.connect_clicked(move |_| {
                        dialog_for_import.close();
                        show_page(
                            &state_for_import,
                            if target == ReferencePickerTarget::Latex {
                                "references"
                            } else {
                                "editor"
                            },
                        );
                        import_bibtex(state_for_import.clone());
                    });
                    list.append(&import);
                }
            }
        }
    }));
    let fill_changed = fill.clone();
    query.connect_search_changed(move |entry| {
        if let Some(fill) = fill_changed.borrow().as_ref() {
            fill(entry.text().as_str());
        }
    });
    if let Some(fill) = fill.borrow().as_ref() {
        fill("");
    }
    dialog.connect_response(|dialog, _| dialog.close());
    dialog.present();
}
