use super::*;

pub(in crate::app::window) fn create_project_file(state: &Rc<State>, extension: &str) {
    let extension = extension.to_owned();
    with_discard_confirmation(state, move |state| {
        create_project_file_confirmed(&state, &extension)
    });
}

pub(in crate::app::window) fn create_project_file_confirmed(state: &Rc<State>, extension: &str) {
    let extension = extension.to_owned();
    if let Some(folder) = state
        .project_target
        .borrow()
        .clone()
        .or_else(|| state.project.project_root.borrow().clone())
    {
        prompt_create_file(state, &folder, &extension);
    } else {
        choose_file(
            state,
            gtk::FileChooserAction::SelectFolder,
            "Choose a project folder",
            "Open",
            None,
            move |state, folder| {
                set_project_root(&state, Some(folder.clone()));
                *state.project.project_target.borrow_mut() = Some(folder.clone());
                prompt_create_file(&state, &folder, &extension);
            },
        );
    }
}

pub(in crate::app::window) fn prompt_create_file(
    state: &Rc<State>,
    folder: &Path,
    extension: &str,
) {
    let (title, default_name, contents) = match extension {
        "tex" => (
            "New LaTeX document",
            "document.tex",
            LatexEditor::new_document_text(),
        ),
        "bib" => ("New bibliography (.bib)", "references.bib", String::new()),
        "sty" => ("New style file (.sty)", "styles.sty", String::new()),
        _ => return,
    };
    let dialog = gtk::Dialog::builder()
        .title(tr_dynamic(title))
        .transient_for(&state.window)
        .modal(true)
        .build();
    dialog.add_button(&tr("Cancel"), gtk::ResponseType::Cancel);
    dialog.add_button(&tr("Create"), gtk::ResponseType::Accept);
    dialog.set_default_response(gtk::ResponseType::Accept);
    let content = dialog.content_area();
    content.set_spacing(10);
    content.set_margin_top(16);
    content.set_margin_bottom(16);
    content.set_margin_start(20);
    content.set_margin_end(20);
    content.append(&gtk::Label::new(Some(&tr(
        "File name in the selected folder",
    ))));
    let name = gtk::Entry::new();
    name.set_text(default_name);
    name.set_activates_default(true);
    content.append(&name);
    let name_for_response = name.clone();
    let folder = folder.to_path_buf();
    let extension = extension.to_owned();
    let contents = Rc::new(contents);
    let state2 = state.clone();
    dialog.connect_response(move |dialog, response| {
        if response != gtk::ResponseType::Accept {
            dialog.close();
            return;
        }
        let mut file_name = name_for_response.text().trim().to_owned();
        if !valid_name(&file_name) {
            show_toast(&state2, &tr("Enter a valid file name"));
            return;
        }
        if !file_name
            .to_ascii_lowercase()
            .ends_with(&format!(".{extension}"))
        {
            file_name.push_str(&format!(".{extension}"));
        }
        let path = folder.join(file_name);
        let path_for_worker = path.clone();
        let contents = contents.as_ref().clone();
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            use std::io::Write;
            let mut created = false;
            let result = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path_for_worker)
                .and_then(|mut file| {
                    created = true;
                    file.write_all(contents.as_bytes())
                });
            if created && result.is_err() {
                let _ = std::fs::remove_file(&path_for_worker);
            }
            let _ = sender.send(result.map_err(|error| error.to_string()));
        });
        let dialog_for_result = dialog.clone();
        let state_for_result = state2.clone();
        let extension_for_result = extension.clone();
        let project_root = state2
            .project_root
            .borrow()
            .clone()
            .unwrap_or(folder.clone());
        poll_result(receiver, move |result| match result {
            Ok(()) => {
                if extension_for_result == "tex" {
                    *state_for_result.project.main_tex.borrow_mut() = Some(path.clone());
                }
                open_document(&state_for_result, &path, Some(project_root));
                show_toast(&state_for_result, &tr("File created"));
                dialog_for_result.close();
            }
            Err(error) => show_toast(
                &state_for_result,
                &format!("{}: {error}", tr("Could not create the file")),
            ),
        });
    });
    dialog.present();
    name.grab_focus();
}

pub(in crate::app::window) fn create_project_folder(state: Rc<State>) {
    let folder = state
        .project_target
        .borrow()
        .clone()
        .or_else(|| state.project.project_root.borrow().clone());
    if let Some(folder) = folder {
        prompt_create_folder(&state, &folder);
    } else {
        choose_file(
            &state,
            gtk::FileChooserAction::SelectFolder,
            "Choose a project folder",
            "Open",
            None,
            |state, folder| {
                set_project_root(&state, Some(folder.clone()));
                *state.project.project_target.borrow_mut() = Some(folder.clone());
                prompt_create_folder(&state, &folder);
            },
        );
    }
}

pub(in crate::app::window) fn prompt_create_folder(state: &Rc<State>, parent: &Path) {
    let dialog = gtk::Dialog::builder()
        .title(tr("New folder"))
        .transient_for(&state.window)
        .modal(true)
        .build();
    dialog.add_button(&tr("Cancel"), gtk::ResponseType::Cancel);
    dialog.add_button(&tr("Create"), gtk::ResponseType::Accept);
    dialog.set_default_response(gtk::ResponseType::Accept);
    let entry = gtk::Entry::new();
    entry.set_placeholder_text(Some(&tr("Folder name")));
    entry.set_activates_default(true);
    entry.set_margin_top(18);
    entry.set_margin_bottom(18);
    entry.set_margin_start(20);
    entry.set_margin_end(20);
    dialog.content_area().append(&entry);
    let entry_for_response = entry.clone();
    let parent = parent.to_path_buf();
    let state2 = state.clone();
    dialog.connect_response(move |dialog, response| {
        if response != gtk::ResponseType::Accept {
            dialog.close();
            return;
        }
        let name = entry_for_response.text().trim().to_owned();
        if !valid_name(&name) {
            show_toast(&state2, &tr("Enter a valid folder name"));
            return;
        }
        let folder = parent.join(name);
        let folder_for_worker = folder.clone();
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = sender
                .send(std::fs::create_dir(&folder_for_worker).map_err(|error| error.to_string()));
        });
        let dialog_for_result = dialog.clone();
        let state_for_result = state2.clone();
        let parent_for_result = parent.clone();
        poll_result(receiver, move |result| match result {
            Ok(()) => {
                state_for_result
                    .expanded_folders
                    .borrow_mut()
                    .insert(parent_for_result.clone());
                *state_for_result.project.project_target.borrow_mut() = Some(folder);
                refresh_project_files(&state_for_result);
                show_toast(&state_for_result, &tr("Folder created"));
                dialog_for_result.close();
            }
            Err(error) => show_toast(
                &state_for_result,
                &format!("{}: {error}", tr("Could not create the folder")),
            ),
        });
    });
    dialog.present();
    entry.grab_focus();
}

pub(in crate::app::window) fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.trim() == name
        && name != "."
        && name != ".."
        && !name
            .chars()
            .any(|ch| ch == '/' || ch == '\\' || ch.is_control())
}
