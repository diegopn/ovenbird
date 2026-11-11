use super::*;

pub(super) fn choose_file(
    state: &Rc<State>,
    action: gtk::FileChooserAction,
    title: &str,
    accept: &str,
    default_name: Option<&str>,
    callback: impl Fn(Rc<State>, PathBuf) + 'static,
) {
    let dialog = gtk::FileChooserNative::new(
        Some(&tr_dynamic(title)),
        Some(&state.window),
        action,
        Some(&tr_dynamic(accept)),
        Some(&tr("Cancel")),
    );
    set_default_file_dialog_folder(&dialog);
    if let Some(name) = default_name {
        dialog.set_current_name(name);
    }
    let filter = gtk::FileFilter::new();
    filter.set_name(Some(&tr("Supported documents and images")));
    for pattern in [
        "*.tex", "*.bib", "*.txt", "*.bst", "*.cls", "*.sty", "*.pdf", "*.png", "*.jpg", "*.jpeg",
        "*.gif", "*.bmp", "*.tif", "*.tiff", "*.svg", "*.webp",
    ] {
        filter.add_pattern(pattern);
    }
    dialog.add_filter(&filter);
    let state = state.clone();
    dialog.connect_response(move |dialog, response| {
        if response == gtk::ResponseType::Accept {
            if let Some(path) = dialog.file().and_then(|file| file.path()) {
                callback(state.clone(), path);
            }
        }
        dialog.destroy();
    });
    dialog.show();
}

pub(super) fn set_default_file_dialog_folder(dialog: &impl gtk::prelude::FileChooserExtManual) {
    if let Some(folder) = default_file_dialog_folder() {
        let _ = dialog.set_current_folder(Some(&gio::File::for_path(folder)));
    }
}

pub(super) fn choose_open_document(state: Rc<State>) {
    choose_file(
        &state,
        gtk::FileChooserAction::Open,
        "Open document",
        "Open",
        None,
        |state, path| {
            with_discard_confirmation(&state, move |state| open_document(&state, &path, None));
        },
    );
}

pub(super) fn choose_project_folder(state: Rc<State>) {
    choose_file(
        &state,
        gtk::FileChooserAction::SelectFolder,
        "Open project",
        "Open",
        None,
        |state, path| {
            with_discard_confirmation(&state, move |state| {
                open_project(&state, &path, false, state.startup_view)
            });
        },
    );
}

pub(super) fn open_document(state: &Rc<State>, path: &Path, root: Option<PathBuf>) {
    open_document_with_page(state, path, root, None);
}

pub(super) fn open_document_with_page(
    state: &Rc<State>,
    path: &Path,
    root: Option<PathBuf>,
    page_after_open: Option<&'static str>,
) {
    let path = path.to_path_buf();
    let root = root.or_else(|| path.parent().map(Path::to_path_buf));
    let kind = project::file_kind(&path);
    if kind == ProjectFileKind::Unsupported {
        show_toast(state, &tr("This file type is not supported in the editor."));
        return;
    }
    let request_id = state.open_request_id.get().wrapping_add(1);
    state.open_request_id.set(request_id);
    state.status.set_label(&tr("Opening document…"));

    if kind.is_previewable() {
        set_active_project_file_context(state, &path, root, request_id);
        state.editor.set_file(Some(path.clone()));
        state.editor.load_text("");
        state.search_entry.set_text("");
        state.search_panel.set_visible(false);
        match kind {
            ProjectFileKind::Image => {
                state
                    .file_pdf_preview
                    .set_message(&tr("Select a PDF file to preview it here."));
                state.image_preview.set_filename(Some(path.as_path()));
            }
            ProjectFileKind::Pdf => {
                state.image_preview.set_filename(None::<&Path>);
                if let Err(error) = state.file_pdf_preview.open(&path) {
                    state
                        .file_pdf_preview
                        .set_message(&tr("Could not open this PDF file."));
                    show_toast(state, &format!("{}: {error}", tr("Could not open the PDF")));
                }
            }
            _ => unreachable!(),
        }
        update_editor_capabilities(state);
        state.status.set_label(&tr("Ready"));
        set_build_status(state, "Ready");
        refresh_project_files(state);
        update_title(state, false);
        show_page(state, page_after_open.unwrap_or("editor"));
        return;
    }

    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let result = std::fs::read(&path)
            .map(|bytes| project::decode_text(&bytes))
            .map_err(|error| error.to_string());
        let _ = sender.send((request_id, path, root, result));
    });
    let state = state.clone();
    poll_result(receiver, move |(request_id, path, root, result)| {
        if state.open_request_id.get() != request_id {
            return;
        }
        match result {
            Ok((text, encoding)) => {
                set_active_project_file_context(&state, &path, root, request_id);
                state.file_encoding.set(encoding);
                state.image_preview.set_filename(None::<&Path>);
                state
                    .file_pdf_preview
                    .set_message(&tr("Select a PDF file to preview it here."));
                state.editor.set_file(Some(path));
                state.editor.load_text(&text);
                state.editor.set_mode(EditorMode::Code);
                state.search_entry.set_text("");
                state.search_panel.set_visible(false);
                state.code_mode_button.set_active(true);
                state.visual_mode_button.set_active(false);
                update_editor_capabilities(&state);
                state.status.set_label(&tr("Ready"));
                set_build_status(&state, "Ready");
                state.editor.focus();
                refresh_project_files(&state);
                update_title(&state, false);
                show_page(&state, page_after_open.unwrap_or("editor"));
            }
            Err(error) => {
                state.status.set_label(&tr("Ready"));
                show_toast(
                    &state,
                    &format!("{}: {error}", tr("Could not open the document")),
                );
            }
        }
    });
}

pub(super) fn set_active_project_file_context(
    state: &Rc<State>,
    path: &Path,
    root: Option<PathBuf>,
    request_id: u64,
) {
    let root = root.or_else(|| path.parent().map(Path::to_path_buf));
    let is_tex = project::file_kind(path) == ProjectFileKind::Latex;
    set_project_root(state, root.clone());
    if let Some(root) = root.as_deref() {
        let mut expanded = state.expanded_folders.borrow_mut();
        let mut parent = path.parent();
        while let Some(folder) = parent {
            if folder == root {
                break;
            }
            expanded.insert(folder.to_path_buf());
            parent = folder.parent();
        }
    }
    *state.current_file.borrow_mut() = Some(path.to_path_buf());
    if is_tex {
        *state.main_tex.borrow_mut() = Some(path.to_path_buf());
    } else {
        let main_is_available = state.main_tex.borrow().as_deref().is_some_and(|main| {
            root.as_deref()
                .is_some_and(|folder| main.starts_with(folder))
        });
        if !main_is_available {
            *state.main_tex.borrow_mut() = None;
            if let Some(root) = root.clone() {
                find_project_main_async(state, root, path.to_path_buf(), request_id);
            }
        }
    }
    state.project_target.replace(root);
}

pub(super) fn set_project_root(state: &Rc<State>, root: Option<PathBuf>) {
    let root_changed = state.project_root.borrow().as_ref() != root.as_ref();
    if root_changed {
        state.expanded_folders.borrow_mut().clear();
        state.project_files_scroll.set_value(0.0);
    }
    *state.project_root.borrow_mut() = root;
}

pub(super) fn find_project_main_async(
    state: &Rc<State>,
    root: PathBuf,
    current_file: PathBuf,
    request_id: u64,
) {
    let root_for_worker = root.clone();
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let main = project::find_tex_files(&root_for_worker)
            .ok()
            .and_then(|documents| documents.first().cloned());
        let _ = sender.send(main);
    });
    let state = state.clone();
    poll_result(receiver, move |main| {
        if state.open_request_id.get() != request_id
            || state.current_file.borrow().as_deref() != Some(current_file.as_path())
            || state.project_root.borrow().as_deref() != Some(root.as_path())
        {
            return;
        }
        *state.main_tex.borrow_mut() = main;
        refresh_build_status(&state);
    });
}

pub(super) fn open_project(
    state: &Rc<State>,
    folder: &Path,
    restoring: bool,
    startup_view: StartupView,
) {
    let folder = folder.to_path_buf();
    set_project_root(state, Some(folder.clone()));
    *state.project_target.borrow_mut() = Some(folder.clone());
    refresh_build_status(state);
    state.expanded_folders.borrow_mut().clear();
    state.project_files_scroll.set_value(0.0);
    refresh_project_files(state);
    let request_id = state.project_open_id.get().wrapping_add(1);
    state.project_open_id.set(request_id);
    state.status.set_label(&tr("Loading project…"));
    let folder_for_worker = folder.clone();
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let documents =
            project::find_tex_files(&folder_for_worker).map_err(|error| error.to_string());
        let _ = sender.send((request_id, folder_for_worker, documents));
    });
    let state = state.clone();
    poll_result(receiver, move |(request_id, folder, result)| {
        if state.project_open_id.get() != request_id
            || state.project_root.borrow().as_deref() != Some(folder.as_path())
        {
            return;
        }
        state.status.set_label(&tr("Ready"));
        let documents = match result {
            Ok(documents) => {
                remember_last_project_folder(&folder);
                documents
            }
            Err(error) => {
                if restoring {
                    set_project_root(&state, None);
                    state.project_target.borrow_mut().take();
                    state.main_tex.borrow_mut().take();
                    refresh_project_files(&state);
                }
                show_toast(
                    &state,
                    &format!("{}: {error}", tr("Could not list the project folder")),
                );
                return;
            }
        };
        let root_main = folder.join("main.tex");
        let folder_named_main = folder.file_name().map(|name| {
            let mut file_name = name.to_os_string();
            file_name.push(".tex");
            folder.join(file_name)
        });
        let main = documents
            .contains(&root_main)
            .then_some(root_main)
            .or_else(|| folder_named_main.filter(|path| documents.contains(path)))
            .or_else(|| {
                state
                    .main_tex
                    .borrow()
                    .clone()
                    .filter(|path| documents.contains(path))
            })
            .or_else(|| (documents.len() == 1).then(|| documents[0].clone()));
        *state.main_tex.borrow_mut() = main.clone();
        let page_after_open = restoring.then(|| startup_page_name(startup_view));
        if let Some(main) = main {
            open_document_with_page(&state, &main, Some(folder), page_after_open);
        } else if documents.len() > 1 {
            choose_main_document(&state, &folder, documents, page_after_open);
        } else {
            show_page(&state, page_after_open.unwrap_or("editor"));
            show_toast(&state, &tr("This project has no .tex document yet."));
        }
    });
}

pub(super) fn choose_main_document(
    state: &Rc<State>,
    folder: &Path,
    documents: Vec<PathBuf>,
    page_after_open: Option<&'static str>,
) {
    let dialog = gtk::Dialog::builder()
        .title(tr("Choose main document"))
        .transient_for(&state.window)
        .modal(true)
        .default_width(520)
        .default_height(500)
        .build();
    dialog.add_button(&tr("Cancel"), gtk::ResponseType::Cancel);
    dialog.add_button(&tr("Open"), gtk::ResponseType::Accept);
    dialog.set_default_response(gtk::ResponseType::Accept);
    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::Single);
    list.add_css_class("boxed-list");
    for document in &documents {
        let row = gtk::ListBoxRow::new();
        row.set_activatable(true);
        let label = gtk::Label::new(Some(
            &document
                .strip_prefix(folder)
                .unwrap_or(document)
                .display()
                .to_string(),
        ));
        label.set_xalign(0.0);
        label.set_margin_top(10);
        label.set_margin_bottom(10);
        label.set_margin_start(12);
        label.set_margin_end(12);
        row.set_child(Some(&label));
        list.append(&row);
    }
    list.select_row(list.row_at_index(0).as_ref());
    let scroll = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .child(&list)
        .build();
    scroll.set_margin_top(12);
    scroll.set_margin_bottom(12);
    scroll.set_margin_start(12);
    scroll.set_margin_end(12);
    dialog.content_area().append(&scroll);
    let dialog_for_row = dialog.clone();
    list.connect_row_activated(move |_, _| dialog_for_row.response(gtk::ResponseType::Accept));
    let state = state.clone();
    let folder = folder.to_path_buf();
    dialog.connect_response(move |dialog, response| {
        let selected = if response == gtk::ResponseType::Accept {
            list.selected_row()
                .and_then(|row| documents.get(row.index() as usize))
                .cloned()
        } else {
            None
        };
        dialog.close();
        if let Some(document) = selected {
            open_document_with_page(&state, &document, Some(folder.clone()), page_after_open);
        }
    });
    dialog.present();
}

pub(super) fn save_document(state: Rc<State>) {
    let current_file = state.current_file.borrow().clone();
    if let Some(path) = current_file {
        if project::file_kind(&path).is_previewable() {
            return;
        }
        if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("tex"))
        {
            *state.main_tex.borrow_mut() = Some(path.clone());
        }
        save_document_to_path(state, path, None);
        return;
    }
    choose_file(
        &state,
        gtk::FileChooserAction::Save,
        "Save document",
        "Save",
        None,
        |state, path| {
            if !project::file_kind(&path).is_text_editable() {
                show_toast(&state, &tr("Choose a supported text file type."));
                return;
            }
            state.editor.set_file(Some(path.clone()));
            *state.current_file.borrow_mut() = Some(path.clone());
            set_project_root(&state, path.parent().map(Path::to_path_buf));
            if path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("tex"))
            {
                *state.main_tex.borrow_mut() = Some(path.clone());
            }
            update_editor_capabilities(&state);
            save_document_to_path(state, path, None);
        },
    );
}

pub(super) fn save_document_to_path(state: Rc<State>, path: PathBuf, after: Option<SaveCallback>) {
    if !project::file_kind(&path).is_text_editable() {
        show_toast(
            &state,
            &tr("This file type cannot be saved from the text editor."),
        );
        return;
    }
    if state.save_in_progress.replace(true) {
        show_toast(&state, &tr("A document save is still in progress."));
        return;
    }
    let source = state.editor.text();
    let encoding = state.file_encoding.get();
    state.status.set_label(&tr("Saving…"));
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let result = project::encode_text(&source, encoding).and_then(|bytes| {
            crate::storage::atomic_write(&path, &bytes).map_err(|error| error.to_string())
        });
        let _ = sender.send((result, path, source));
    });
    poll_result(receiver, move |(result, path, source)| {
        state.save_in_progress.set(false);
        state.status.set_label(&tr("Ready"));
        match result {
            Ok(()) => {
                state.editor.mark_saved(&source);
                refresh_project_files(&state);
                if state.editor.dirty() {
                    if let Some(after) = after {
                        save_document_to_path(state, path, Some(after));
                    } else {
                        update_title(&state, true);
                        show_toast(
                            &state,
                            &tr("The document changed while it was being saved. Save again."),
                        );
                    }
                    return;
                }
                update_title(&state, false);
                if let Some(after) = after {
                    after(state);
                } else {
                    show_toast(&state, &tr("Document saved"));
                }
            }
            Err(error) => show_toast(&state, &error),
        }
    });
}
