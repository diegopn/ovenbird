use super::*;

pub(super) fn setup_reference_list_view(state: &Rc<State>) {
    let factory = gtk::SignalListItemFactory::new();
    let state_for_setup = Rc::downgrade(state);
    factory.connect_setup(move |_, object| {
        let Some(item) = object.downcast_ref::<gtk::ListItem>() else {
            return;
        };
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        row.set_hexpand(true);
        row.set_margin_top(4);
        row.set_margin_bottom(4);

        let details = gtk::Box::new(gtk::Orientation::Vertical, 3);
        let title = gtk::Label::new(None);
        title.set_xalign(0.0);
        title.set_hexpand(true);
        title.set_wrap(true);
        title.set_wrap_mode(gtk::pango::WrapMode::WordChar);
        title.add_css_class("heading");
        details.append(&title);
        let subtitle = gtk::Label::new(None);
        subtitle.set_xalign(0.0);
        subtitle.set_hexpand(true);
        subtitle.set_wrap(true);
        subtitle.set_wrap_mode(gtk::pango::WrapMode::WordChar);
        subtitle.add_css_class("dim-label");
        details.append(&subtitle);
        details.set_hexpand(true);
        let summary = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        summary.set_hexpand(true);
        summary.append(&details);
        let key = gtk::Label::new(None);
        key.add_css_class("dim-label");
        summary.append(&key);

        let open_details = gtk::Button::new();
        open_details.set_has_frame(false);
        open_details.add_css_class("flat");
        open_details.set_hexpand(true);
        open_details.set_halign(gtk::Align::Fill);
        open_details.set_child(Some(&summary));
        open_details.set_tooltip_text(Some(&tr("Show reference details")));
        let item_for_details = item.downgrade();
        let state_for_details = state_for_setup.clone();
        open_details.connect_clicked(move |_| {
            let Some(state) = state_for_details.upgrade() else {
                return;
            };
            let Some(item) = item_for_details.upgrade() else {
                return;
            };
            let Some(entry) = reference_list_entry(&item) else {
                return;
            };
            show_reference_details(&state, entry);
        });
        row.append(&open_details);

        let edit = button("document-edit-symbolic", &tr("Edit reference"));
        let item_for_edit = item.downgrade();
        let state_for_edit = state_for_setup.clone();
        edit.connect_clicked(move |_| {
            let Some(state) = state_for_edit.upgrade() else {
                return;
            };
            let Some(item) = item_for_edit.upgrade() else {
                return;
            };
            let Some(entry) = reference_list_entry(&item) else {
                return;
            };
            edit_reference(&state, Some(entry));
        });
        row.append(&edit);

        let delete = button("user-trash-symbolic", &tr("Delete reference"));
        delete.add_css_class("destructive-action");
        let item_for_delete = item.downgrade();
        let state_for_delete = state_for_setup.clone();
        delete.connect_clicked(move |_| {
            let Some(state) = state_for_delete.upgrade() else {
                return;
            };
            let Some(item) = item_for_delete.upgrade() else {
                return;
            };
            let Some(entry) = reference_list_entry(&item) else {
                return;
            };
            let parent: &gtk::Window = state.window.upcast_ref();
            request_reference_deletion(&state, parent, entry, || {});
        });
        row.append(&delete);

        let message_page = gtk::Box::new(gtk::Orientation::Vertical, 8);
        message_page.set_margin_top(24);
        message_page.set_margin_bottom(24);
        let message = gtk::Label::new(None);
        message.set_xalign(0.0);
        message.set_wrap(true);
        message.add_css_class("dim-label");
        message_page.append(&message);
        let retry = gtk::Button::with_label(&tr("Retry"));
        retry.set_halign(gtk::Align::Start);
        let state_for_retry = state_for_setup.clone();
        retry.connect_clicked(move |_| {
            if let Some(state) = state_for_retry.upgrade() {
                load_library_async(&state);
            }
        });
        message_page.append(&retry);

        let stack = gtk::Stack::new();
        stack.set_transition_type(gtk::StackTransitionType::None);
        stack.add_named(&row, Some("entry"));
        stack.add_named(&message_page, Some("message"));
        item.set_child(Some(&stack));
    });

    factory.connect_bind(|_, object| {
        let Some(item) = object.downcast_ref::<gtk::ListItem>() else {
            return;
        };
        bind_reference_list_item(item);
    });
    state.library_list.set_factory(Some(&factory));
}

pub(super) fn reference_list_entry(item: &gtk::ListItem) -> Option<BibEntry> {
    let item = item.item()?.downcast::<glib::BoxedAnyObject>().ok()?;
    let row = item.borrow::<ReferenceListItem>();
    match &*row {
        ReferenceListItem::Entry(entry) => Some(entry.clone()),
        ReferenceListItem::Message { .. } => None,
    }
}

pub(super) fn bind_reference_list_item(item: &gtk::ListItem) {
    let Some(list_item) = item
        .item()
        .and_then(|item| item.downcast::<glib::BoxedAnyObject>().ok())
    else {
        return;
    };
    let list_item = list_item.borrow::<ReferenceListItem>();
    let Some(stack) = item
        .child()
        .and_then(|child| child.downcast::<gtk::Stack>().ok())
    else {
        return;
    };

    match &*list_item {
        ReferenceListItem::Message { text, retry } => {
            let Some(message_page) = stack
                .child_by_name("message")
                .and_then(|child| child.downcast::<gtk::Box>().ok())
            else {
                return;
            };
            let Some(message) = message_page
                .first_child()
                .and_then(|child| child.downcast::<gtk::Label>().ok())
            else {
                return;
            };
            let Some(retry_button) = message
                .next_sibling()
                .and_then(|child| child.downcast::<gtk::Button>().ok())
            else {
                return;
            };
            message.set_text(text);
            retry_button.set_visible(*retry);
            stack.set_visible_child_name("message");
        }
        ReferenceListItem::Entry(entry) => {
            let Some(row) = stack
                .child_by_name("entry")
                .and_then(|child| child.downcast::<gtk::Box>().ok())
            else {
                return;
            };
            let Some(open_details) = row
                .first_child()
                .and_then(|child| child.downcast::<gtk::Button>().ok())
            else {
                return;
            };
            let Some(summary) = open_details
                .child()
                .and_then(|child| child.downcast::<gtk::Box>().ok())
            else {
                return;
            };
            let Some(details) = summary
                .first_child()
                .and_then(|child| child.downcast::<gtk::Box>().ok())
            else {
                return;
            };
            let Some(title_label) = details
                .first_child()
                .and_then(|child| child.downcast::<gtk::Label>().ok())
            else {
                return;
            };
            let Some(subtitle_label) = title_label
                .next_sibling()
                .and_then(|child| child.downcast::<gtk::Label>().ok())
            else {
                return;
            };
            let Some(key_label) = details
                .next_sibling()
                .and_then(|child| child.downcast::<gtk::Label>().ok())
            else {
                return;
            };

            let title = if entry.get("title").is_empty() {
                bibtex::display_bibtex_text(&entry.key)
            } else {
                bibtex::display_bibtex_text(entry.get("title"))
            };
            let author_field = if entry.get("author").is_empty() {
                entry.get("editor")
            } else {
                entry.get("author")
            };
            let author = bibtex::display_bibtex_names(author_field).join(" · ");
            let subtitle = format!(
                "{}{}{}",
                author,
                if entry.get("year").is_empty() {
                    ""
                } else {
                    " · "
                },
                entry.get("year")
            );
            title_label.set_text(&title);
            subtitle_label.set_text(&subtitle);
            key_label.set_text(&entry.key);
            stack.set_visible_child_name("entry");
        }
    }
}

pub(super) fn show_citation_picker(state: Rc<State>) {
    show_reference_picker(state, ReferencePickerTarget::Latex);
}

pub(super) fn show_bibtex_reference_picker(state: Rc<State>) {
    show_reference_picker(state, ReferencePickerTarget::Bibtex);
}

pub(super) fn show_reference_picker(state: Rc<State>, target: ReferencePickerTarget) {
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

pub(super) fn insert_latex_citation(state: &Rc<State>, entry: &BibEntry) -> Result<(), String> {
    let current_file = state
        .current_file
        .borrow()
        .clone()
        .ok_or_else(|| tr("Save the document before inserting a citation."))?;
    if project::file_kind(&current_file) != ProjectFileKind::Latex {
        return Err(tr("Open a LaTeX document before inserting a citation."));
    }
    let snippet = format!("\\cite{{{}}}", entry.key);
    state
        .editor
        .insert_at_cursor(&snippet, snippet.chars().count(), "insert:citation");
    Ok(())
}

pub(super) fn insert_latex_bibliography_reference(
    state: &Rc<State>,
    entry: &BibEntry,
    style: bibtex::ReferenceStyle,
) -> Result<(), String> {
    let current_file = state
        .current_file
        .borrow()
        .clone()
        .ok_or_else(|| tr("Save the document before inserting a citation."))?;
    let main_path = state
        .main_tex
        .borrow()
        .clone()
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("tex"))
        })
        .or_else(|| {
            current_file
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("tex"))
                .then(|| current_file.clone())
        })
        .ok_or_else(|| tr("Open a LaTeX document before inserting a citation."))?;
    let main_is_open = current_file == main_path;
    let (main_source, source_encoding) = if main_is_open {
        (state.editor.text(), state.file_encoding.get())
    } else {
        let bytes = std::fs::read(&main_path).map_err(|error| error.to_string())?;
        project::decode_text(&bytes)
    };
    let update = crate::build::prepare_inline_bibliography_update(
        &main_source,
        source_encoding,
        entry,
        style,
    )?;
    match update {
        crate::build::CitationBibliographyUpdate::InlineTex {
            content,
            inserted_text,
            insertion_offset,
            encoding,
        } => {
            project::encode_text(&content, encoding)?;
            if main_is_open {
                state.editor.insert_source_at_offset(
                    &inserted_text,
                    insertion_offset,
                    "insert:citation",
                );
            } else {
                let bytes = project::encode_text(&content, encoding)?;
                crate::storage::atomic_write(&main_path, &bytes)
                    .map_err(|error| error.to_string())?;
                refresh_project_files(state);
            }
        }
        crate::build::CitationBibliographyUpdate::BibFile { .. } => unreachable!(),
        crate::build::CitationBibliographyUpdate::AlreadyPresent => {}
    }
    Ok(())
}

pub(super) fn insert_bibtex_reference(state: &Rc<State>, entry: &BibEntry) -> Result<(), String> {
    let current_file = state
        .current_file
        .borrow()
        .clone()
        .ok_or_else(|| tr("Open a BibTeX file before adding a bibliographic reference."))?;
    if project::file_kind(&current_file) != ProjectFileKind::Bibtex {
        return Err(tr(
            "Open a BibTeX file before adding a bibliographic reference.",
        ));
    }
    let source = state.editor.text();
    let bibliography = bibtex::parse_bibtex(&source).map_err(|error| error.to_string())?;
    if bibliography
        .entries
        .iter()
        .any(|existing| existing.key.eq_ignore_ascii_case(&entry.key))
    {
        return Err(tr("This reference is already in the BibTeX file."));
    }

    let line_ending = if source.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let serialized = bibtex::serialize_bibtex(&bibtex::Bibliography {
        entries: vec![entry.clone()],
        directives: Vec::new(),
    })
    .replace("\r\n", "\n")
    .replace('\n', line_ending);
    let separator = if source.is_empty() {
        String::new()
    } else if source.ends_with('\n') {
        line_ending.to_owned()
    } else {
        format!("{line_ending}{line_ending}")
    };
    let insertion = format!("{separator}{serialized}");
    let offset = source.chars().count();
    state
        .editor
        .insert_source_at_offset(&insertion, offset, "insert:bibtex-reference");
    Ok(())
}

pub(super) fn load_library_async(state: &Rc<State>) {
    if state.library_loading.replace(true) {
        return;
    }
    state.library_load_error.borrow_mut().take();
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
            state.library_loading.set(false);
            *state.library.borrow_mut() = library;
            state.library_ready.set(true);
            refresh_library(&state);
            refresh_tags(&state);
            refresh_authors(&state);
        }
        Err(error) => {
            state.library_loading.set(false);
            *state.library_load_error.borrow_mut() = Some(error.clone());
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

pub(super) fn persist_library_async(
    state: Rc<State>,
    updated: LocalLibrary,
    success_message: String,
) {
    if !state.library_ready.get() || state.library_busy.replace(true) {
        show_toast(
            &state,
            &tr("The local library is busy. Try again in a moment."),
        );
        return;
    }
    state.status.set_label(&tr("Saving references…"));
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let result = updated.save().map_err(|error| error.to_string());
        let _ = sender.send((result, updated));
    });
    poll_result(receiver, move |(result, updated)| {
        state.library_busy.set(false);
        state.status.set_label(&tr("Ready"));
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

pub(super) fn library_is_ready(state: &Rc<State>) -> bool {
    if !state.library_ready.get() {
        show_toast(state, &tr("The local library is still loading."));
        return false;
    }
    if state.library_busy.get() {
        show_toast(
            state,
            &tr("The local library is busy. Try again in a moment."),
        );
        return false;
    }
    true
}

pub(super) fn refresh_library(state: &Rc<State>) {
    if !state.library_ready.get() {
        if let Some(error) = state.library_load_error.borrow().as_ref() {
            set_reference_list_message(
                &state.library_list_model,
                format!("{}: {error}", tr("Could not load the local library")),
                true,
            );
        } else {
            set_reference_list_message(
                &state.library_list_model,
                tr("Loading local library…"),
                false,
            );
        }
        update_primary_action(state);
        return;
    }
    let term = state.library_search.text();
    let library = state.library.borrow();
    let mut filter_options = reference_filter_values(&library.bibliography.entries);
    filter_options.authors =
        reference_filter_author_names(&filter_options.authors, &library.authors);
    filter_options.tags.extend(library.tags.iter().cloned());
    filter_options.tags = sorted_reference_values(filter_options.tags);
    state.library_filters_updating.set(true);
    set_reference_filter_options(
        &state.reference_filters.author,
        &tr("All authors"),
        filter_options.authors,
    );
    set_reference_filter_options(
        &state.reference_filters.year,
        &tr("All years"),
        filter_options.years,
    );
    set_reference_filter_options(
        &state.reference_filters.tag,
        &tr("All tags"),
        filter_options.tags,
    );
    set_reference_filter_options(
        &state.reference_filters.kind,
        &tr("All types"),
        filter_options.kinds,
    );
    state.library_filters_updating.set(false);
    let author_filter = selected_reference_filter(&state.reference_filters.author);
    let year_filter = selected_reference_filter(&state.reference_filters.year);
    let tag_filter = selected_reference_filter(&state.reference_filters.tag);
    let kind_filter = selected_reference_filter(&state.reference_filters.kind);
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
        state.reference_sort.selected(),
        state.reference_sort_descending.is_active(),
    );
    if entries.is_empty() {
        let message = if library.bibliography.entries.is_empty() {
            tr("Your library starts here: add a reference or import BibTeX.")
        } else {
            tr("No references match the current search and filters.")
        };
        set_reference_list_message(&state.library_list_model, message, false);
    } else {
        replace_reference_list_entries(&state.library_list_model, entries);
    }
    update_primary_action(state);
}

pub(super) fn set_reference_list_message(model: &gio::ListStore, text: String, retry: bool) {
    let message = glib::BoxedAnyObject::new(ReferenceListItem::Message { text, retry });
    model.splice(0, model.n_items(), &[message]);
}

pub(super) fn replace_reference_list_entries(model: &gio::ListStore, entries: Vec<BibEntry>) {
    let rows = entries
        .into_iter()
        .map(|entry| glib::BoxedAnyObject::new(ReferenceListItem::Entry(entry)))
        .collect::<Vec<_>>();
    model.splice(0, model.n_items(), &rows);
}

pub(super) fn refresh_tags(state: &Rc<State>) {
    while let Some(child) = state.tags_list.first_child() {
        state.tags_list.remove(&child);
    }
    if !state.library_ready.get() {
        let message = if let Some(error) = state.library_load_error.borrow().as_ref() {
            format!("{}: {error}", tr("Could not load the local library"))
        } else {
            tr("Loading local library…")
        };
        let label = gtk::Label::new(Some(&message));
        label.set_xalign(0.0);
        label.set_wrap(true);
        label.set_margin_top(24);
        label.add_css_class("dim-label");
        state.tags_list.append(&label);
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
        state.tags_list.append(&empty);
        return;
    }

    let query = state.tag_search.text().to_lowercase();
    let mut tags = tags
        .into_iter()
        .filter(|tag| query.is_empty() || tag.to_lowercase().contains(&query))
        .collect::<Vec<_>>();
    tags.sort_by_key(|tag| tag.to_lowercase());
    if state.tag_sort_descending.is_active() {
        tags.reverse();
    }
    if tags.is_empty() {
        let empty = gtk::Label::new(Some(&tr("No tags match your search.")));
        empty.set_xalign(0.0);
        empty.set_margin_top(18);
        empty.add_css_class("dim-label");
        state.tags_list.append(&empty);
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
        state.tags_list.append(&row);
        state
            .tags_list
            .append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    }
}

pub(super) fn refresh_authors(state: &Rc<State>) {
    while let Some(child) = state.authors_list.first_child() {
        state.authors_list.remove(&child);
    }
    if !state.library_ready.get() {
        let message = if let Some(error) = state.library_load_error.borrow().as_ref() {
            format!("{}: {error}", tr("Could not load the local library"))
        } else {
            tr("Loading local library…")
        };
        let label = gtk::Label::new(Some(&message));
        label.set_xalign(0.0);
        label.set_wrap(true);
        label.set_margin_top(24);
        label.add_css_class("dim-label");
        state.authors_list.append(&label);
        return;
    }

    let search = state.author_search.text().to_lowercase();
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
        let primary = match state.author_sort.selected() {
            1 => left
                .institution
                .to_lowercase()
                .cmp(&right.institution.to_lowercase()),
            _ => left
                .full_name
                .to_lowercase()
                .cmp(&right.full_name.to_lowercase()),
        };
        let primary = if state.author_sort_descending.is_active() {
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
        state.authors_list.append(&empty);
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
        state.authors_list.append(&row);
        state
            .authors_list
            .append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    }
}

pub(super) fn author_form_row(content: &gtk::Box, label: &str, input: &gtk::Entry) {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let label = gtk::Label::new(Some(label));
    label.set_xalign(1.0);
    label.set_width_chars(20);
    row.append(&label);
    input.set_hexpand(true);
    row.append(input);
    content.append(&row);
}

pub(super) fn edit_author(state: &Rc<State>, previous: Option<String>) {
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

pub(super) fn show_author_details(state: &Rc<State>, citation_name: &str) {
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

pub(super) fn author_detail_row(state: &Rc<State>, name: &str, value: &str) -> gtk::Box {
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

pub(super) fn request_author_deletion(state: &Rc<State>, author: &str) {
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

pub(super) fn edit_tag(state: &Rc<State>, previous: Option<String>) {
    if !library_is_ready(state) {
        return;
    }
    let dialog = gtk::Dialog::builder()
        .title(tr(if previous.is_some() {
            "Edit tag"
        } else {
            "Add tag"
        }))
        .transient_for(&state.window)
        .modal(true)
        .default_width(420)
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
    let input = gtk::Entry::new();
    input.set_placeholder_text(Some(&tr("Tag name")));
    input.set_hexpand(true);
    if let Some(previous) = previous.as_ref() {
        input.set_text(previous);
    }
    content.append(&input);
    dialog.content_area().append(&content);

    let previous_for_response = previous.clone();
    let state_for_response = state.clone();
    let input_for_response = input.clone();
    dialog.connect_response(move |dialog, response| {
        if response == gtk::ResponseType::Accept {
            let mut updated = state_for_response.library.borrow().clone();
            let result = if let Some(previous) = previous_for_response.as_deref() {
                updated.rename_tag(previous, input_for_response.text().as_str())
            } else {
                updated.add_tag(input_for_response.text().as_str())
            };
            match result {
                Ok(()) => {
                    persist_library_async(state_for_response.clone(), updated, tr("Tag saved"));
                    dialog.close();
                }
                Err(error) => show_toast(&state_for_response, &tr(&error)),
            }
        } else {
            dialog.close();
        }
    });
    dialog.present();
    input.grab_focus();
}

pub(super) fn request_tag_deletion(state: &Rc<State>, tag: &str) {
    if !library_is_ready(state) {
        return;
    }
    let message = tr("Remove tag ‘%s’ from the list and all references?").replacen("%s", tag, 1);
    let parent: &gtk::Window = state.window.upcast_ref();
    let tag = tag.to_owned();
    confirm_destructive_action(
        state,
        parent,
        &tr("Confirm deletion"),
        &message,
        &tr("Remove tag"),
        move |state| {
            if !library_is_ready(&state) {
                return;
            }
            let mut updated = state.library.borrow().clone();
            if updated.remove_tag(&tag) {
                persist_library_async(state, updated, tr("Tag removed"));
            }
        },
    );
}

pub(super) fn reference_style_from_index(index: u32) -> bibtex::ReferenceStyle {
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

pub(super) fn show_reference_details(state: &Rc<State>, entry: BibEntry) {
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

pub(super) fn reference_detail_row(state: &Rc<State>, name: &str, value: &str) -> gtk::Box {
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

pub(super) fn reference_detail_tags_row(name: &str, value: &str) -> gtk::Box {
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

pub(super) fn copy_text_to_clipboard(state: &Rc<State>, text: &str) {
    if let Some(display) = gdk::Display::default() {
        display.clipboard().set_text(text);
        show_toast(state, &tr("Copied to clipboard"));
    } else {
        show_toast(state, &tr("Could not access the clipboard."));
    }
}

pub(super) fn reference_filter_values(entries: &[BibEntry]) -> ReferenceFilterOptions {
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

pub(super) fn reference_filter_author_names(
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

pub(super) fn parse_tag_values(value: &str) -> Vec<String> {
    let mut tags = Vec::new();
    for tag in value
        .split([',', ';'])
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
    {
        if !tags
            .iter()
            .any(|existing: &String| existing.eq_ignore_ascii_case(tag))
        {
            tags.push(tag.to_owned());
        }
    }
    tags
}

pub(super) fn add_tag_selection(
    name: &str,
    available: &Rc<RefCell<Vec<String>>>,
    selected: &Rc<RefCell<Vec<String>>>,
) -> bool {
    let name = name.trim();
    if name.is_empty() {
        return false;
    }
    let existing = available
        .borrow()
        .iter()
        .find(|tag| tag.eq_ignore_ascii_case(name))
        .cloned();
    let canonical = if let Some(existing) = existing {
        existing
    } else {
        let name = name.to_owned();
        available.borrow_mut().push(name.clone());
        name
    };
    let mut selected = selected.borrow_mut();
    if selected
        .iter()
        .any(|tag| tag.eq_ignore_ascii_case(&canonical))
    {
        return true;
    }
    selected.push(canonical);
    true
}

pub(super) fn refresh_reference_tag_chips(
    chips: &gtk::FlowBox,
    selected: &Rc<RefCell<Vec<String>>>,
) {
    while let Some(child) = chips.first_child() {
        chips.remove(&child);
    }
    for tag in selected.borrow().iter() {
        let chip = gtk::Button::with_label(&format!("{tag}  ×"));
        chip.add_css_class("pill");
        chip.add_css_class("reference-tag-chip");
        chip.set_tooltip_text(Some(&tr("Remove tag from this reference")));
        let selected = selected.clone();
        let chips_for_click = chips.clone();
        let remove = tag.clone();
        chip.connect_clicked(move |_| {
            selected
                .borrow_mut()
                .retain(|tag| !tag.eq_ignore_ascii_case(&remove));
            refresh_reference_tag_chips(&chips_for_click, &selected);
        });
        chips.append(&chip);
    }
}

pub(super) fn refresh_reference_tag_picker(
    list: &gtk::Box,
    chips: &gtk::FlowBox,
    selected: &Rc<RefCell<Vec<String>>>,
    available: &Rc<RefCell<Vec<String>>>,
    query: &str,
) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
    let query = query.trim().to_lowercase();
    let choices = available
        .borrow()
        .iter()
        .filter(|tag| query.is_empty() || tag.to_lowercase().contains(&query))
        .cloned()
        .collect::<Vec<_>>();
    if choices.is_empty() {
        let empty = gtk::Label::new(Some(&tr("No matching tags")));
        empty.set_xalign(0.0);
        empty.add_css_class("dim-label");
        list.append(&empty);
        return;
    }
    for tag in choices {
        let choice = gtk::CheckButton::with_label(&tag);
        choice.set_active(
            selected
                .borrow()
                .iter()
                .any(|selected| selected.eq_ignore_ascii_case(&tag)),
        );
        let selected = selected.clone();
        let chips = chips.clone();
        let tag_for_toggle = tag.clone();
        choice.connect_toggled(move |choice| {
            let mut selected_tags = selected.borrow_mut();
            if choice.is_active() {
                if !selected_tags
                    .iter()
                    .any(|selected| selected.eq_ignore_ascii_case(&tag_for_toggle))
                {
                    selected_tags.push(tag_for_toggle.clone());
                }
            } else {
                selected_tags.retain(|selected| !selected.eq_ignore_ascii_case(&tag_for_toggle));
            }
            drop(selected_tags);
            refresh_reference_tag_chips(&chips, &selected);
        });
        list.append(&choice);
    }
}

pub(super) fn author_selection_identity(name: &str) -> String {
    bibtex::display_bibtex_text(name)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

pub(super) fn add_author_selection(
    name: &str,
    available: &Rc<RefCell<Vec<String>>>,
    selected: &Rc<RefCell<Vec<String>>>,
) -> bool {
    let name = name.trim();
    if name.is_empty() {
        return false;
    }
    let identity = author_selection_identity(name);
    let existing = available
        .borrow()
        .iter()
        .find(|author| author_selection_identity(author) == identity)
        .cloned();
    let canonical = if let Some(existing) = existing {
        existing
    } else {
        let name = name.to_owned();
        available.borrow_mut().push(name.clone());
        name
    };
    let mut selected = selected.borrow_mut();
    if selected
        .iter()
        .any(|author| author_selection_identity(author) == identity)
    {
        return true;
    }
    selected.push(canonical);
    true
}

pub(super) fn refresh_reference_author_list(
    selected_list: &gtk::Box,
    selected: &Rc<RefCell<Vec<String>>>,
) {
    while let Some(child) = selected_list.first_child() {
        selected_list.remove(&child);
    }
    for (index, author) in selected.borrow().iter().cloned().enumerate() {
        let display_name = bibtex::display_bibtex_text(&author);
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        row.add_css_class("card");
        row.add_css_class("reference-author-row");
        row.set_hexpand(true);
        row.set_margin_start(2);
        row.set_margin_end(2);
        let name = gtk::Label::new(Some(&display_name));
        name.set_xalign(0.0);
        name.set_hexpand(true);
        name.set_ellipsize(gtk::pango::EllipsizeMode::End);
        row.append(&name);

        let move_up = button("go-up-symbolic", &tr("Move author up"));
        move_up.set_sensitive(index > 0);
        let selected_for_up = selected.clone();
        let list_for_up = selected_list.clone();
        move_up.connect_clicked(move |_| {
            {
                let mut authors = selected_for_up.borrow_mut();
                if index > 0 && index < authors.len() {
                    authors.swap(index, index - 1);
                }
            }
            refresh_reference_author_list(&list_for_up, &selected_for_up);
        });
        row.append(&move_up);

        let move_down = button("go-down-symbolic", &tr("Move author down"));
        move_down.set_sensitive(index + 1 < selected.borrow().len());
        let selected_for_down = selected.clone();
        let list_for_down = selected_list.clone();
        move_down.connect_clicked(move |_| {
            {
                let mut authors = selected_for_down.borrow_mut();
                if index + 1 < authors.len() {
                    authors.swap(index, index + 1);
                }
            }
            refresh_reference_author_list(&list_for_down, &selected_for_down);
        });
        row.append(&move_down);

        let remove = button(
            "window-close-symbolic",
            &tr("Remove author from this reference"),
        );
        let selected_for_remove = selected.clone();
        let list_for_remove = selected_list.clone();
        let remove_identity = author_selection_identity(&author);
        remove.connect_clicked(move |_| {
            selected_for_remove
                .borrow_mut()
                .retain(|author| author_selection_identity(author) != remove_identity);
            refresh_reference_author_list(&list_for_remove, &selected_for_remove);
        });
        row.append(&remove);
        selected_list.append(&row);
    }
}

pub(super) fn refresh_reference_author_picker(
    list: &gtk::Box,
    selected_list: &gtk::Box,
    selected: &Rc<RefCell<Vec<String>>>,
    available: &Rc<RefCell<Vec<String>>>,
    query: &str,
) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
    let query = query.trim().to_lowercase();
    let mut choices = available
        .borrow()
        .iter()
        .filter(|author| {
            query.is_empty()
                || bibtex::display_bibtex_text(author)
                    .to_lowercase()
                    .contains(&query)
        })
        .cloned()
        .collect::<Vec<_>>();
    choices.sort_by_key(|author| bibtex::display_bibtex_text(author).to_lowercase());
    if choices.is_empty() {
        let empty = gtk::Label::new(Some(&tr("No matching authors")));
        empty.set_xalign(0.0);
        empty.add_css_class("dim-label");
        list.append(&empty);
        return;
    }
    for author in choices {
        let choice = gtk::CheckButton::with_label(&bibtex::display_bibtex_text(&author));
        let identity = author_selection_identity(&author);
        choice.set_active(
            selected
                .borrow()
                .iter()
                .any(|selected| author_selection_identity(selected) == identity),
        );
        let selected = selected.clone();
        let selected_list = selected_list.clone();
        let author_for_toggle = author.clone();
        choice.connect_toggled(move |choice| {
            let identity = author_selection_identity(&author_for_toggle);
            let mut selected_authors = selected.borrow_mut();
            if choice.is_active() {
                if !selected_authors
                    .iter()
                    .any(|selected| author_selection_identity(selected) == identity)
                {
                    selected_authors.push(author_for_toggle.clone());
                }
            } else {
                selected_authors.retain(|selected| author_selection_identity(selected) != identity);
            }
            drop(selected_authors);
            refresh_reference_author_list(&selected_list, &selected);
        });
        list.append(&choice);
    }
}

pub(super) fn sorted_reference_values(values: impl IntoIterator<Item = String>) -> Vec<String> {
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

pub(super) fn reference_authors(entry: &BibEntry) -> Vec<String> {
    let author = if entry.get("author").trim().is_empty() {
        entry.get("editor")
    } else {
        entry.get("author")
    };
    bibtex::display_bibtex_names(author)
}

pub(super) fn reference_year(entry: &BibEntry) -> Option<String> {
    let year = entry.get("year").trim();
    if !year.is_empty() {
        return Some(year.to_owned());
    }
    let date = entry.get("date").trim();
    let year = date.chars().take(4).collect::<String>();
    (year.len() == 4 && year.chars().all(|character| character.is_ascii_digit())).then_some(year)
}

pub(super) fn reference_tags(entry: &BibEntry) -> Vec<String> {
    ["keywords", "tags"]
        .into_iter()
        .flat_map(|field| entry.get(field).split([',', ';']))
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
pub(super) fn reference_matches_filters(
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

pub(super) fn reference_matches_filters_with_type(
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

pub(super) fn filtered_reference_entries(
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

pub(super) fn sort_reference_entries(entries: &mut [BibEntry], sort_by: u32, descending: bool) {
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

pub(super) fn reference_tag_sort_key(entry: &BibEntry) -> Vec<String> {
    let mut tags = reference_tags(entry)
        .into_iter()
        .map(|tag| tag.to_lowercase())
        .collect::<Vec<_>>();
    tags.sort();
    tags.dedup();
    tags
}

pub(super) fn reference_title(entry: &BibEntry) -> String {
    if entry.get("title").trim().is_empty() {
        bibtex::display_bibtex_text(&entry.key)
    } else {
        bibtex::display_bibtex_text(entry.get("title"))
    }
}

pub(super) fn selected_reference_filter(dropdown: &gtk::DropDown) -> Option<String> {
    if dropdown.selected() == 0 || dropdown.selected() == gtk::INVALID_LIST_POSITION {
        return None;
    }
    dropdown
        .selected_item()?
        .downcast::<gtk::StringObject>()
        .ok()
        .map(|item| item.string().to_string())
}

pub(super) fn request_reference_deletion(
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

pub(super) fn set_reference_filter_options(
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

pub(super) fn edit_reference(state: &Rc<State>, entry: Option<BibEntry>) {
    if !library_is_ready(state) {
        return;
    }
    let dialog = gtk::Dialog::builder()
        .title(tr_dynamic(if entry.is_some() {
            "Edit reference"
        } else {
            "New reference"
        }))
        .transient_for(&state.window)
        .modal(true)
        .build();
    dialog.set_default_size(700, 660);
    let grid = gtk::Grid::builder()
        .column_spacing(12)
        .row_spacing(8)
        .margin_top(18)
        .margin_bottom(18)
        .margin_start(20)
        .margin_end(30)
        .build();
    let form_scroll = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&grid)
        .build();
    let content = dialog.content_area();
    content.append(&form_scroll);
    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    actions.set_margin_top(16);
    actions.set_margin_bottom(0);
    actions.set_margin_start(6);
    actions.set_margin_end(6);
    if entry.is_some() {
        let delete = gtk::Button::with_label(&tr("Delete"));
        delete.add_css_class("destructive-action");
        let dialog_for_delete = dialog.clone();
        delete.connect_clicked(move |_| dialog_for_delete.response(gtk::ResponseType::Reject));
        actions.append(&delete);
    }
    let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    actions.append(&spacer);
    let cancel = gtk::Button::with_label(&tr("Cancel"));
    let dialog_for_cancel = dialog.clone();
    cancel.connect_clicked(move |_| dialog_for_cancel.response(gtk::ResponseType::Cancel));
    actions.append(&cancel);
    let save_label = tr_dynamic(if entry.is_some() { "Save" } else { "Add" });
    let save = gtk::Button::with_label(&save_label);
    save.add_css_class("suggested-action");
    let dialog_for_save = dialog.clone();
    save.connect_clicked(move |_| dialog_for_save.response(gtk::ResponseType::Accept));
    actions.append(&save);
    dialog.set_default_widget(Some(&save));
    content.append(&actions);
    let translated_types = REFERENCE_TYPES
        .iter()
        .map(|kind| bibtex::reference_type_label(kind))
        .collect::<Vec<_>>();
    let type_choices = translated_types
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let type_model = gtk::StringList::new(&type_choices);
    let picker = gtk::DropDown::new(Some(type_model), gtk::Expression::NONE);
    let selected_type = entry
        .as_ref()
        .map(|entry| entry.entry_type.as_str())
        .unwrap_or("article");
    picker.set_selected(
        REFERENCE_TYPES
            .iter()
            .position(|kind| *kind == selected_type)
            .unwrap_or(0) as u32,
    );
    grid.attach(&gtk::Label::new(Some(&tr("Reference type"))), 0, 0, 1, 1);
    grid.attach(&picker, 1, 0, 1, 1);
    let field_names = REFERENCE_TYPES
        .iter()
        .flat_map(|kind| {
            crate::bibtex::fields_for_reference_type(kind)
                .iter()
                .copied()
        })
        .collect::<HashSet<_>>();
    let mut names = field_names.iter().copied().collect::<Vec<_>>();
    names.sort();
    let key_entry = gtk::Entry::new();
    if let Some(entry) = entry.as_ref() {
        key_entry.set_text(&entry.key);
    }
    let rows = Rc::new(RefCell::new(
        HashMap::<String, (gtk::Label, gtk::Entry)>::new(),
    ));
    for name in names {
        if name == "tags" || name == "author" {
            continue;
        }
        let label = gtk::Label::new(Some(&bibtex::reference_field_label(name, selected_type)));
        label.set_xalign(1.0);
        label.add_css_class("dim-label");
        let input = gtk::Entry::new();
        input.set_hexpand(true);
        if name == "title" {
            input.set_placeholder_text(Some(&tr("Full title of the work")));
        }
        if let Some(entry) = entry.as_ref() {
            let value = if name == "date" && entry.get("date").is_empty() {
                entry.get("year")
            } else {
                entry.get(name)
            };
            input.set_text(value);
        }
        rows.borrow_mut().insert(name.to_owned(), (label, input));
    }

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

    let selected_tags = Rc::new(RefCell::new(
        entry
            .as_ref()
            .map(|entry| parse_tag_values(entry.get("tags")))
            .unwrap_or_default(),
    ));
    let available_tags = Rc::new(RefCell::new(state.library.borrow().tags.clone()));
    for tag in selected_tags.borrow().iter() {
        let missing = {
            let available = available_tags.borrow();
            !available
                .iter()
                .any(|existing| existing.eq_ignore_ascii_case(tag))
        };
        if missing {
            available_tags.borrow_mut().push(tag.clone());
        }
    }
    let tag_chips = gtk::FlowBox::new();
    tag_chips.set_selection_mode(gtk::SelectionMode::None);
    tag_chips.set_row_spacing(4);
    tag_chips.set_column_spacing(4);
    tag_chips.set_max_children_per_line(8);
    tag_chips.set_min_children_per_line(1);
    tag_chips.set_hexpand(true);
    tag_chips.set_halign(gtk::Align::Fill);
    refresh_reference_tag_chips(&tag_chips, &selected_tags);

    let tag_editor = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    tag_editor.set_hexpand(true);
    tag_editor.append(&tag_chips);

    let picker_popover = gtk::Popover::new();
    let picker_content = gtk::Box::new(gtk::Orientation::Vertical, 8);
    picker_content.set_margin_top(8);
    picker_content.set_margin_bottom(8);
    picker_content.set_margin_start(8);
    picker_content.set_margin_end(8);
    picker_content.set_size_request(260, -1);
    let tag_search = gtk::SearchEntry::new();
    tag_search.set_placeholder_text(Some(&tr("Find tags…")));
    picker_content.append(&tag_search);
    let tag_choices = gtk::Box::new(gtk::Orientation::Vertical, 2);
    tag_choices.set_hexpand(true);
    let tag_choices_scroll = gtk::ScrolledWindow::builder()
        .min_content_height(36)
        .max_content_height(180)
        .child(&tag_choices)
        .build();
    picker_content.append(&tag_choices_scroll);
    let create_row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let new_tag_entry = gtk::Entry::new();
    new_tag_entry.set_placeholder_text(Some(&tr("New tag")));
    new_tag_entry.set_hexpand(true);
    let create_tag_button = gtk::Button::with_label(&tr("Create"));
    create_row.append(&new_tag_entry);
    create_row.append(&create_tag_button);
    picker_content.append(&create_row);
    picker_popover.set_child(Some(&picker_content));
    refresh_reference_tag_picker(
        &tag_choices,
        &tag_chips,
        &selected_tags,
        &available_tags,
        "",
    );
    tag_search.connect_search_changed({
        let tag_choices = tag_choices.clone();
        let tag_chips = tag_chips.clone();
        let selected_tags = selected_tags.clone();
        let available_tags = available_tags.clone();
        move |search| {
            refresh_reference_tag_picker(
                &tag_choices,
                &tag_chips,
                &selected_tags,
                &available_tags,
                search.text().as_str(),
            );
        }
    });
    create_tag_button.connect_clicked({
        let new_tag_entry = new_tag_entry.clone();
        let tag_choices = tag_choices.clone();
        let tag_chips = tag_chips.clone();
        let selected_tags = selected_tags.clone();
        let available_tags = available_tags.clone();
        move |_| {
            if add_tag_selection(
                new_tag_entry.text().as_str(),
                &available_tags,
                &selected_tags,
            ) {
                new_tag_entry.set_text("");
                refresh_reference_tag_chips(&tag_chips, &selected_tags);
                refresh_reference_tag_picker(
                    &tag_choices,
                    &tag_chips,
                    &selected_tags,
                    &available_tags,
                    "",
                );
            }
        }
    });
    create_tag_button.set_tooltip_text(Some(&tr("Create and select this tag")));
    new_tag_entry.connect_activate({
        let create_tag_button = create_tag_button.clone();
        move |_| create_tag_button.emit_clicked()
    });
    let add_tag_button = gtk::MenuButton::new();
    add_tag_button.set_child(Some(&labeled_content("list-add-symbolic", &tr("Add tag"))));
    add_tag_button.set_tooltip_text(Some(&tr("Select or create tags")));
    add_tag_button.set_popover(Some(&picker_popover));
    add_tag_button.connect_active_notify({
        let tag_choices = tag_choices.clone();
        let tag_chips = tag_chips.clone();
        let selected_tags = selected_tags.clone();
        let available_tags = available_tags.clone();
        move |button| {
            if button.is_active() {
                refresh_reference_tag_picker(
                    &tag_choices,
                    &tag_chips,
                    &selected_tags,
                    &available_tags,
                    tag_search.text().as_str(),
                );
            }
        }
    });
    tag_editor.append(&add_tag_button);
    let tag_label = gtk::Label::new(Some(&tr("Tags")));
    tag_label.set_xalign(1.0);
    tag_label.add_css_class("dim-label");

    let key_label = gtk::Label::new(Some(&tr("Citation key")));
    key_label.set_xalign(1.0);
    key_label.add_css_class("dim-label");
    grid.attach(&key_label, 0, 1, 1, 1);
    grid.attach(&key_entry, 1, 1, 1, 1);
    let update_rows = {
        let rows = rows.clone();
        let grid = grid.clone();
        let author_label = author_label.clone();
        let author_editor = author_editor.clone();
        let tag_label = tag_label.clone();
        let tag_editor = tag_editor.clone();
        move |picker: &gtk::DropDown| {
            for (label, input) in rows.borrow().values() {
                if label.parent().is_some() {
                    grid.remove(label);
                }
                if input.parent().is_some() {
                    grid.remove(input);
                }
            }
            let kind = REFERENCE_TYPES
                .get(picker.selected() as usize)
                .copied()
                .unwrap_or("article");
            let mut row_num = 2;
            if let Some((label, input)) = rows.borrow().get("title") {
                label.set_label(&bibtex::reference_field_label("title", kind));
                grid.attach(label, 0, row_num, 1, 1);
                grid.attach(input, 1, row_num, 1, 1);
                row_num += 1;
            }
            grid.attach(&author_label, 0, row_num, 1, 1);
            grid.attach(&author_editor, 1, row_num, 1, 1);
            row_num += 1;
            for name in crate::bibtex::fields_for_reference_type(kind) {
                if *name == "tags" || *name == "title" || *name == "author" {
                    continue;
                }
                if let Some((label, input)) = rows.borrow().get(*name) {
                    label.set_label(&bibtex::reference_field_label(name, kind));
                    grid.attach(label, 0, row_num, 1, 1);
                    grid.attach(input, 1, row_num, 1, 1);
                    row_num += 1;
                }
            }
            tag_label.set_label(&bibtex::reference_field_label("tags", kind));
            grid.attach(&tag_label, 0, row_num, 1, 1);
            grid.attach(&tag_editor, 1, row_num, 1, 1);
        }
    };
    update_rows(&picker);
    let update_rows_notify = update_rows;
    picker.connect_selected_notify(move |picker| update_rows_notify(picker));
    let previous = entry.clone();
    let state2 = state.clone();
    dialog.connect_response(move |dialog, response| {
        if response == gtk::ResponseType::Reject {
            if let Some(entry) = previous.as_ref() {
                let parent: &gtk::Window = dialog.upcast_ref();
                let edit_dialog = dialog.clone();
                request_reference_deletion(&state2, parent, entry.clone(), move || {
                    edit_dialog.close()
                });
            } else {
                dialog.close();
            }
            return;
        }
        if response == gtk::ResponseType::Accept {
            let kind = REFERENCE_TYPES
                .get(picker.selected() as usize)
                .copied()
                .unwrap_or("article");
            let mut values = rows
                .borrow()
                .iter()
                .map(|(name, (_, entry))| (name.clone(), entry.text().trim().to_owned()))
                .collect::<indexmap::IndexMap<_, _>>();
            values.insert("author".to_owned(), selected_authors.borrow().join(" and "));
            values.insert("tags".to_owned(), selected_tags.borrow().join(", "));
            let old_fields = previous
                .as_ref()
                .map(|entry| &entry.fields)
                .cloned()
                .unwrap_or_default();
            let fields = bibtex::build_reference_fields(kind, &values, &old_fields);
            if fields.get("title").is_none_or(String::is_empty) {
                show_toast(&state2, &tr("Enter the reference title"));
                return;
            }
            let used = state2
                .library
                .borrow()
                .bibliography
                .entries
                .iter()
                .filter(|item| previous.as_ref().is_none_or(|old| old.key != item.key))
                .map(|item| item.key.to_ascii_lowercase())
                .collect::<HashSet<_>>();
            let key = if key_entry.text().trim().is_empty() {
                bibtex::create_citation_key(&fields, &used)
            } else {
                key_entry.text().trim().to_owned()
            };
            let updated = BibEntry {
                entry_type: kind.to_owned(),
                key,
                fields,
                raw_fields: indexmap::IndexMap::new(),
            };
            let mut library = state2.library.borrow().clone();
            let result = if let Some(previous) = previous.as_ref() {
                library.update_in_memory(&previous.key, updated)
            } else {
                library.add_in_memory(updated)
            };
            match result {
                Ok(()) => {
                    persist_library_async(state2.clone(), library, tr("Reference saved"));
                    dialog.close();
                }
                Err(error) => show_toast(&state2, &error),
            }
            return;
        }
        dialog.close();
    });
    dialog.present();
}

pub(super) fn show_reference_transfer_dialog(state: Rc<State>, direction: TransferDirection) {
    let (title, description, icon_name) = match direction {
        TransferDirection::Import => (
            tr("Import references"),
            tr("Choose a format to import."),
            "document-open-symbolic",
        ),
        TransferDirection::Export => (
            tr("Export references"),
            tr("Choose a format to export."),
            "document-save-symbolic",
        ),
    };
    let dialog = gtk::Dialog::builder()
        .title(title)
        .transient_for(&state.window)
        .modal(true)
        .default_width(460)
        .build();
    dialog.add_button(&tr("Close"), gtk::ResponseType::Close);

    let content = dialog.content_area();
    content.set_spacing(12);
    content.set_margin_top(18);
    content.set_margin_bottom(18);
    content.set_margin_start(20);
    content.set_margin_end(20);
    let description = gtk::Label::new(Some(&description));
    description.set_xalign(0.0);
    description.add_css_class("dim-label");
    content.append(&description);

    for format in reference_exchange::AVAILABLE_FORMATS {
        let format = *format;
        let button = gtk::Button::new();
        button.add_css_class("card");
        button.set_hexpand(true);
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        row.set_margin_top(12);
        row.set_margin_bottom(12);
        row.set_margin_start(12);
        row.set_margin_end(12);

        let icon = gtk::Image::from_icon_name(icon_name);
        icon.set_pixel_size(24);
        row.append(&icon);

        let text = gtk::Box::new(gtk::Orientation::Vertical, 3);
        text.set_hexpand(true);
        let title = gtk::Label::new(Some(format.label()));
        title.set_xalign(0.0);
        title.add_css_class("heading");
        text.append(&title);
        let detail = gtk::Label::new(Some(&tr("Standard format for bibliographic references.")));
        detail.set_xalign(0.0);
        detail.add_css_class("dim-label");
        text.append(&detail);
        row.append(&text);

        let next = gtk::Image::from_icon_name("go-next-symbolic");
        next.add_css_class("dim-label");
        row.append(&next);
        button.set_child(Some(&row));

        let weak_dialog = dialog.downgrade();
        let weak_state = Rc::downgrade(&state);
        button.connect_clicked(move |_| {
            if let Some(dialog) = weak_dialog.upgrade() {
                dialog.close();
            }
            if let Some(state) = weak_state.upgrade() {
                match reference_exchange::action_for(direction, format) {
                    TransferAction::ImportBibtex => import_bibtex(state),
                    TransferAction::ExportBibtex => export_bibtex(state),
                }
            }
        });
        content.append(&button);
    }

    content.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    let guides_label = gtk::Label::new(Some(&tr("Platform guides")));
    guides_label.set_xalign(0.0);
    guides_label.add_css_class("dim-label");
    content.append(&guides_label);

    let guides = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    for guide in REFERENCE_MANAGER_GUIDES {
        let link = gtk::LinkButton::with_label(guide.uri, guide.name);
        link.set_halign(gtk::Align::Start);
        guides.append(&link);
    }
    content.append(&guides);

    dialog.connect_response(|dialog, _| dialog.close());
    dialog.present();
}

pub(super) fn import_bibtex(state: Rc<State>) {
    if !library_is_ready(&state) {
        return;
    }
    choose_file(
        &state,
        gtk::FileChooserAction::Open,
        "Import BibTeX",
        "Import",
        None,
        |state, path| {
            let (sender, receiver) = mpsc::channel();
            std::thread::spawn(move || {
                let result = std::fs::read_to_string(&path)
                    .map_err(|error| error.to_string())
                    .and_then(|contents| {
                        bibtex::parse_bibtex(&contents).map_err(|error| error.to_string())
                    });
                let _ = sender.send(result);
            });
            poll_result(receiver, move |result| match result {
                Ok(imported) => {
                    if !library_is_ready(&state) {
                        return;
                    }
                    let mut library = state.library.borrow().clone();
                    let count = library.merge_import(imported);
                    persist_library_async(
                        state.clone(),
                        library,
                        tr("Imported %d references").replace("%d", &count.to_string()),
                    );
                }
                Err(error) => show_toast(
                    &state,
                    &format!("{}: {error}", tr("Could not import BibTeX")),
                ),
            });
        },
    );
}

pub(super) fn export_bibtex(state: Rc<State>) {
    if !library_is_ready(&state) {
        return;
    }
    choose_file(
        &state,
        gtk::FileChooserAction::Save,
        "Export BibTeX",
        "Export",
        Some("export_bibitex_ovenbird.bib"),
        |state, path| {
            let source = bibtex::serialize_bibtex(&state.library.borrow().bibliography);
            let (sender, receiver) = mpsc::channel();
            std::thread::spawn(move || {
                let result = std::fs::write(path, source).map_err(|error| error.to_string());
                let _ = sender.send(result);
            });
            poll_result(receiver, move |result| match result {
                Ok(()) => show_toast(&state, &tr("BibTeX exported")),
                Err(error) => show_toast(
                    &state,
                    &format!("{}: {error}", tr("Could not export BibTeX")),
                ),
            });
        },
    );
}
