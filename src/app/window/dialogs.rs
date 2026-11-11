use super::*;

pub(super) fn with_discard_confirmation(state: &Rc<State>, callback: impl Fn(Rc<State>) + 'static) {
    if state.save_in_progress.get() {
        show_toast(state, &tr("A document save is still in progress."));
        return;
    }
    if !state.editor.dirty() {
        callback(state.clone());
        return;
    }
    let dialog = gtk::Dialog::builder()
        .title(tr("Unsaved changes"))
        .transient_for(&state.window)
        .modal(true)
        .build();
    dialog.add_button(&tr("Cancel"), gtk::ResponseType::Cancel);
    dialog.add_button(&tr("Discard"), gtk::ResponseType::Reject);
    dialog.add_button(&tr("Save"), gtk::ResponseType::Accept);
    let label = gtk::Label::new(Some(&tr("Save changes before continuing?")));
    label.set_margin_top(24);
    label.set_margin_bottom(24);
    label.set_margin_start(24);
    label.set_margin_end(24);
    dialog.content_area().append(&label);
    let state = state.clone();
    let callback: Rc<dyn Fn(Rc<State>)> = Rc::new(callback);
    dialog.connect_response(move |dialog, response| {
        if response == gtk::ResponseType::Reject {
            callback(state.clone());
            dialog.close();
        } else if response == gtk::ResponseType::Accept {
            if state.current_file.borrow().is_some() {
                let path = state.current_file.borrow().clone().unwrap();
                save_document_to_path(state.clone(), path, Some(callback.clone()));
                dialog.close();
            } else {
                let callback = callback.clone();
                choose_file(
                    &state,
                    gtk::FileChooserAction::Save,
                    "Save document",
                    "Save",
                    None,
                    move |state, path| {
                        if !project::file_kind(&path).is_text_editable() {
                            show_toast(&state, &tr("Choose a supported text file type."));
                            return;
                        }
                        state.editor.set_file(Some(path.clone()));
                        *state.current_file.borrow_mut() = Some(path.clone());
                        set_project_root(&state, path.parent().map(Path::to_path_buf));
                        update_editor_capabilities(&state);
                        save_document_to_path(state, path, Some(callback.clone()));
                    },
                );
                dialog.close();
            }
        } else {
            dialog.close();
        }
    });
    dialog.present();
}

pub(super) fn confirm_destructive_action(
    state: &Rc<State>,
    parent: &gtk::Window,
    title: &str,
    message: &str,
    action_label: &str,
    callback: impl FnOnce(Rc<State>) + 'static,
) {
    let dialog = gtk::Dialog::builder()
        .title(title)
        .transient_for(parent)
        .modal(true)
        .default_width(600)
        .build();
    dialog.set_resizable(false);
    dialog.add_button(&tr("Cancel"), gtk::ResponseType::Cancel);
    let confirm = dialog.add_button(action_label, gtk::ResponseType::Accept);
    confirm.add_css_class("destructive-action");
    dialog.set_default_response(gtk::ResponseType::Cancel);

    let label = gtk::Label::new(Some(message));
    label.set_wrap(true);
    label.set_max_width_chars(68);
    label.set_justify(gtk::Justification::Center);
    label.set_xalign(0.5);
    label.set_margin_top(24);
    label.set_margin_bottom(24);
    label.set_margin_start(24);
    label.set_margin_end(24);
    dialog.content_area().append(&label);

    let state = state.clone();
    let callback = RefCell::new(Some(callback));
    dialog.connect_response(move |dialog, response| {
        dialog.close();
        if response == gtk::ResponseType::Accept {
            if let Some(callback) = callback.borrow_mut().take() {
                callback(state.clone());
            }
        }
    });
    dialog.present();
}

pub(super) fn show_link_dialog(state: Rc<State>) {
    let dialog = gtk::Dialog::builder()
        .title(tr("Insert link"))
        .transient_for(&state.window)
        .modal(true)
        .build();
    dialog.add_button(&tr("Cancel"), gtk::ResponseType::Cancel);
    dialog.add_button(&tr("Insert link"), gtk::ResponseType::Accept);
    dialog.set_default_response(gtk::ResponseType::Accept);
    let grid = gtk::Grid::builder()
        .row_spacing(8)
        .column_spacing(10)
        .margin_top(18)
        .margin_bottom(18)
        .margin_start(20)
        .margin_end(20)
        .build();
    let label_entry = gtk::Entry::new();
    label_entry.set_text(&state.editor.selected_text());
    label_entry.set_placeholder_text(Some(&tr("Link text")));
    let url_entry = gtk::Entry::new();
    url_entry.set_placeholder_text(Some("https://"));
    grid.attach(&gtk::Label::new(Some(&tr("Text"))), 0, 0, 1, 1);
    grid.attach(&label_entry, 1, 0, 1, 1);
    grid.attach(&gtk::Label::new(Some(&tr("URL"))), 0, 1, 1, 1);
    grid.attach(&url_entry, 1, 1, 1, 1);
    dialog.content_area().append(&grid);
    let label_entry_for_response = label_entry.clone();
    let url_entry_for_response = url_entry.clone();
    let state2 = state.clone();
    dialog.connect_response(move |dialog, response| {
        if response == gtk::ResponseType::Accept {
            let text = label_entry_for_response.text().to_string();
            let url = url_entry_for_response.text().to_string();
            if url.trim().is_empty() {
                show_toast(&state2, &tr("Enter a URL"));
                return;
            }
            let snippet = format!("\\href{{{url}}}{{{text}}}");
            let cursor_offset = snippet.chars().count();
            state2.editor.insert_latex(
                &LatexInsertion {
                    text: snippet,
                    cursor_offset,
                    selection: None,
                },
                "insert:link",
            );
        }
        dialog.close();
    });
    dialog.present();
    if label_entry.text().is_empty() {
        label_entry.grab_focus();
    } else {
        url_entry.grab_focus();
    }
}

pub(super) fn choose_image(state: Rc<State>) {
    if state.current_file.borrow().as_ref().is_none_or(|path| {
        !path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("tex"))
    }) {
        show_toast(
            &state,
            &tr("Open a LaTeX document before inserting an image."),
        );
        return;
    }
    let dialog = gtk::FileChooserNative::new(
        Some(&tr("Insert image")),
        Some(&state.window),
        gtk::FileChooserAction::Open,
        Some(&tr("Open")),
        Some(&tr("Cancel")),
    );
    let filter = gtk::FileFilter::new();
    filter.set_name(Some(&tr("Image and PDF files")));
    for pattern in ["*.png", "*.jpg", "*.jpeg", "*.pdf", "*.eps"] {
        filter.add_pattern(pattern);
    }
    dialog.add_filter(&filter);
    set_default_file_dialog_folder(&dialog);
    let state_for_choice = state.clone();
    dialog.connect_response(move |dialog, response| {
        if response != gtk::ResponseType::Accept {
            dialog.destroy();
            return;
        }
        let Some(source) = dialog.file().and_then(|file| file.path()) else {
            dialog.destroy();
            return;
        };
        choose_image_selected(state_for_choice.clone(), source);
        dialog.destroy();
    });
    dialog.show();
}

pub(super) fn choose_image_selected(state: Rc<State>, source: PathBuf) {
    let Some(folder) = state
        .project_target
        .borrow()
        .clone()
        .or_else(|| state.project_root.borrow().clone())
    else {
        show_toast(
            &state,
            &tr("Open a project folder before inserting an image."),
        );
        return;
    };
    let Some(name) = source.file_name().map(|name| name.to_os_string()) else {
        show_toast(&state, &tr("The image name is invalid."));
        return;
    };
    let source = source.to_path_buf();
    let folder_for_worker = folder.clone();
    let root_for_insert = state
        .project_root
        .borrow()
        .clone()
        .unwrap_or(folder.clone());
    let current_file = state.current_file.borrow().clone();
    let main_file = state
        .main_tex
        .borrow()
        .clone()
        .or_else(|| current_file.clone());
    let Some(main_directory) = main_file
        .as_deref()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
    else {
        show_toast(
            &state,
            &tr("Choose a main LaTeX document before inserting an image."),
        );
        return;
    };
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let mut destination = folder_for_worker.join(name);
        let mut suffix = 2;
        while destination.exists() {
            let stem = source
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("image");
            let extension = source.extension().and_then(|s| s.to_str()).unwrap_or("");
            let new_name = if extension.is_empty() {
                format!("{stem}-{suffix}")
            } else {
                format!("{stem}-{suffix}.{extension}")
            };
            destination = folder_for_worker.join(new_name);
            suffix += 1;
        }
        let result = std::fs::copy(&source, &destination)
            .map(|_| destination)
            .map_err(|error| error.to_string());
        let _ = sender.send(result);
    });
    poll_result(receiver, move |result| match result {
        Ok(destination) => {
            if state.project_root.borrow().as_deref() != Some(root_for_insert.as_path())
                || state.current_file.borrow().as_ref() != current_file.as_ref()
            {
                if state.project_root.borrow().as_deref() == Some(root_for_insert.as_path()) {
                    refresh_project_files(&state);
                }
                show_toast(
                    &state,
                    &tr("Image copied to the project. Reopen the document to insert it."),
                );
                return;
            }
            let Some(relative) = project::relative_path(&main_directory, &destination) else {
                show_toast(
                    &state,
                    &tr("The image path cannot be represented relative to the main document."),
                );
                return;
            };
            let relative = relative.to_string_lossy().replace('\\', "/");
            let snippet = format!("\\includegraphics[width=\\linewidth]{{{relative}}}");
            state
                .editor
                .insert_at_cursor(&snippet, snippet.chars().count(), "insert:image");
            refresh_project_files(&state);
        }
        Err(error) => show_toast(
            &state,
            &format!("{}: {error}", tr("Could not add the image to the project")),
        ),
    });
}

pub(super) fn show_table_dialog(state: Rc<State>) {
    let dialog = gtk::Dialog::builder()
        .title(tr("Insert table"))
        .transient_for(&state.window)
        .modal(true)
        .build();
    dialog.add_button(&tr("Cancel"), gtk::ResponseType::Cancel);
    dialog.add_button(&tr("Insert"), gtk::ResponseType::Accept);
    let grid = gtk::Grid::builder()
        .row_spacing(8)
        .column_spacing(10)
        .margin_top(18)
        .margin_bottom(18)
        .margin_start(20)
        .margin_end(20)
        .build();
    let rows = gtk::SpinButton::with_range(1.0, 20.0, 1.0);
    rows.set_value(3.0);
    let columns = gtk::SpinButton::with_range(1.0, 12.0, 1.0);
    columns.set_value(3.0);
    grid.attach(&gtk::Label::new(Some(&tr("Rows"))), 0, 0, 1, 1);
    grid.attach(&rows, 1, 0, 1, 1);
    grid.attach(&gtk::Label::new(Some(&tr("Columns"))), 0, 1, 1, 1);
    grid.attach(&columns, 1, 1, 1, 1);
    dialog.content_area().append(&grid);
    let state2 = state.clone();
    dialog.connect_response(move |dialog, response| {
        if response == gtk::ResponseType::Accept {
            let snippet = commands::create_table_snippet(
                rows.value_as_int() as usize,
                columns.value_as_int() as usize,
            );
            state2.editor.insert_latex(&snippet, "insert:table");
        }
        dialog.close();
    });
    dialog.present();
}

pub(super) fn show_text_prompt(
    state: &Rc<State>,
    title: &str,
    prompt: &str,
    initial: &str,
    callback: impl Fn(Rc<State>, String) + 'static,
) {
    let dialog = gtk::Dialog::builder()
        .title(tr_dynamic(title))
        .transient_for(&state.window)
        .modal(true)
        .build();
    dialog.add_button(&tr("Cancel"), gtk::ResponseType::Cancel);
    dialog.add_button(&tr("OK"), gtk::ResponseType::Accept);
    dialog.set_default_response(gtk::ResponseType::Accept);
    let content = dialog.content_area();
    content.set_spacing(8);
    content.set_margin_top(18);
    content.set_margin_bottom(18);
    content.set_margin_start(20);
    content.set_margin_end(20);
    content.append(&gtk::Label::new(Some(&tr_dynamic(prompt))));
    let entry = gtk::Entry::new();
    entry.set_text(initial);
    entry.set_activates_default(true);
    content.append(&entry);
    let entry_for_response = entry.clone();
    let state = state.clone();
    dialog.connect_response(move |dialog, response| {
        if response == gtk::ResponseType::Accept {
            callback(state.clone(), entry_for_response.text().to_string());
        }
        dialog.close();
    });
    dialog.present();
    entry.grab_focus();
}
