use super::*;

pub(in crate::app::window) fn rename_project_file(state: &Rc<State>, path: &Path) {
    let dialog = gtk::Dialog::builder()
        .title(tr("Rename"))
        .transient_for(&state.window)
        .modal(true)
        .build();
    dialog.add_button(&tr("Cancel"), gtk::ResponseType::Cancel);
    dialog.add_button(&tr("Rename"), gtk::ResponseType::Accept);
    let entry = gtk::Entry::new();
    entry.set_text(
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default(),
    );
    entry.set_activates_default(true);
    entry.set_margin_top(18);
    entry.set_margin_bottom(18);
    entry.set_margin_start(20);
    entry.set_margin_end(20);
    dialog.content_area().append(&entry);
    let entry_for_response = entry.clone();
    let path = path.to_path_buf();
    let root = state.project.project_root.borrow().clone();
    let state = state.clone();
    dialog.connect_response(move |dialog, response| {
        if response == gtk::ResponseType::Accept {
            let Some(root) = root.clone() else {
                dialog.close();
                return;
            };
            let path = path.clone();
            let new_name = entry_for_response.text().to_string();
            let (sender, receiver) = mpsc::channel();
            std::thread::spawn(move || {
                let result = project::rename_project_file(&path, &new_name, &root);
                let _ = sender.send((path, result));
            });
            let state_for_result = state.clone();
            poll_result(receiver, move |(old_path, result)| match result {
                Ok(new_path) => update_project_file_path(&state_for_result, &old_path, &new_path),
                Err(error) => show_toast(&state_for_result, &error),
            });
        }
        dialog.close();
    });
    dialog.present();
    entry.grab_focus();
}

pub(in crate::app::window) fn trash_project_file(state: &Rc<State>, path: &Path) {
    let path = path.to_path_buf();
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string());
    let prompt = if path.is_dir() {
        tr("Move folder ‘%s’ and its contents to Trash?")
    } else {
        tr("Move ‘%s’ to Trash? You can restore it later.")
    }
    .replacen("%s", &name, 1);
    let parent: &gtk::Window = state.window.upcast_ref();
    confirm_destructive_action(
        state,
        parent,
        &tr("Confirm deletion"),
        &prompt,
        &tr("Move to Trash"),
        move |state| {
            let active_file_is_being_removed = state
                .current_file
                .borrow()
                .as_ref()
                .is_some_and(|current| project::path_contains(&path, current));
            if active_file_is_being_removed && state.editor.dirty() {
                with_discard_confirmation(&state, move |state| {
                    trash_project_file_now(&state, &path)
                });
            } else {
                trash_project_file_now(&state, &path);
            }
        },
    );
}

pub(in crate::app::window) fn trash_project_file_now(state: &Rc<State>, path: &Path) {
    let file = gio::File::for_path(path);
    let path = path.to_path_buf();
    let state = state.clone();
    file.trash_async(
        glib::Priority::DEFAULT,
        None::<&gio::Cancellable>,
        move |result| match result {
            Ok(()) => finish_project_file_removal(&state, &path, &tr("Moved to Trash")),
            Err(error) => show_toast(
                &state,
                &format!("{}: {error}", tr("Could not move to Trash")),
            ),
        },
    );
}

pub(in crate::app::window) fn finish_project_file_removal(
    state: &Rc<State>,
    path: &Path,
    success_message: &str,
) {
    let current_removed = state
        .current_file
        .borrow()
        .as_ref()
        .is_some_and(|current| project::path_contains(path, current));
    let main_removed = state
        .main_tex
        .borrow()
        .as_ref()
        .is_some_and(|main| project::path_contains(path, main));
    if current_removed {
        state.project.current_file.borrow_mut().take();
        state.project.file_encoding.set(project::TextEncoding::Utf8);
        state.editor.set_file(None);
        state.editor.load_text(&LatexEditor::new_document_text());
        state.image_preview.set_filename(None::<&Path>);
        state
            .file_pdf_preview
            .set_message(&tr("Select a PDF file to preview it here."));
        state.editor.set_mode(EditorMode::Code);
        state.editor_ui.code_mode_button.set_active(true);
        state.editor_ui.visual_mode_button.set_active(false);
        update_editor_capabilities(state);
        update_title(state, false);
    }
    if main_removed {
        *state.project.main_tex.borrow_mut() = None;
    }
    if let Some(root) = state.project.project_root.borrow().clone() {
        if state
            .project_target
            .borrow()
            .as_ref()
            .is_some_and(|target| project::path_contains(path, target))
        {
            *state.project.project_target.borrow_mut() = Some(root.clone());
        }
        state
            .expanded_folders
            .borrow_mut()
            .retain(|folder| !project::path_contains(path, folder));
        if main_removed {
            open_project(state, &root, false, state.project.startup_view);
        } else if current_removed {
            if let Some(main) = state.project.main_tex.borrow().clone() {
                open_document(state, &main, Some(root));
            } else {
                refresh_project_files(state);
            }
        } else {
            refresh_project_files(state);
        }
    } else {
        refresh_project_files(state);
    }
    show_toast(state, success_message);
}
