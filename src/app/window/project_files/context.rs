use super::*;

pub(in crate::app::window) fn show_project_file_context_menu(
    state: &Rc<State>,
    anchor: &gtk::Box,
    path: &Path,
) {
    let file = path.to_path_buf();
    let is_directory = file.is_dir();
    let target = if is_directory {
        file.clone()
    } else {
        file.parent().unwrap_or(&file).to_path_buf()
    };
    *state.project.project_target.borrow_mut() = Some(target.clone());
    let popover = gtk::Popover::new();
    let menu = gtk::Box::new(gtk::Orientation::Vertical, 2);

    if is_directory {
        for (icon, label, extension) in [
            ("text-x-generic-symbolic", "New LaTeX document", "tex"),
            ("text-x-generic-symbolic", "New bibliography (.bib)", "bib"),
            ("text-x-generic-symbolic", "New style file (.sty)", "sty"),
        ] {
            let state = state.clone();
            let target = target.clone();
            append_file_context_action(&menu, &popover, icon, &tr(label), move || {
                *state.project.project_target.borrow_mut() = Some(target);
                create_project_file(&state, extension);
            });
        }
        let state = state.clone();
        let target = target.clone();
        append_file_context_action(
            &menu,
            &popover,
            "folder-new-symbolic",
            &tr("New folder"),
            move || {
                *state.project.project_target.borrow_mut() = Some(target);
                create_project_folder(state);
            },
        );
        menu.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    } else if project::openable_extension(&file) {
        let state = state.clone();
        let file = file.clone();
        let root = state.project.project_root.borrow().clone();
        append_file_context_action(
            &menu,
            &popover,
            "document-open-symbolic",
            &tr("Open"),
            move || {
                let file = file.clone();
                with_discard_confirmation(&state, move |state| {
                    open_document(&state, &file, root.clone())
                });
            },
        );
        menu.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    } else {
        let file = file.clone();
        let state = state.clone();
        append_file_context_action(
            &menu,
            &popover,
            "document-open-symbolic",
            &tr("Open with default application"),
            move || open_with_default_application(&state, &file),
        );
        menu.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    }

    let file_for_copy = file.clone();
    let state_for_copy = state.clone();
    append_file_context_action(
        &menu,
        &popover,
        "edit-copy-symbolic",
        &tr("Copy path"),
        move || {
            if let Some(display) = gdk::Display::default() {
                display
                    .clipboard()
                    .set_text(&file_for_copy.to_string_lossy());
            } else {
                show_toast(&state_for_copy, &tr("Could not access the clipboard."));
            }
        },
    );
    let file_for_parent = file.clone();
    let state_for_parent = state.clone();
    append_file_context_action(
        &menu,
        &popover,
        "folder-open-symbolic",
        &tr("Show in Files"),
        move || open_containing_folder(&state_for_parent, &file_for_parent),
    );
    menu.append(&gtk::Separator::new(gtk::Orientation::Horizontal));

    let state_for_rename = state.clone();
    let file_for_rename = file.clone();
    append_file_context_action(
        &menu,
        &popover,
        "document-edit-symbolic",
        &tr("Rename"),
        move || rename_project_file(&state_for_rename, &file_for_rename),
    );
    let state_for_trash = state.clone();
    let file_for_trash = file.clone();
    append_file_context_action(
        &menu,
        &popover,
        "user-trash-symbolic",
        &tr("Move to Trash"),
        move || trash_project_file(&state_for_trash, &file_for_trash),
    );
    popover.set_child(Some(&menu));
    popover.set_parent(anchor);
    popover.popup();
}

pub(in crate::app::window) fn append_file_context_action(
    menu: &gtk::Box,
    popover: &gtk::Popover,
    icon: &str,
    label: &str,
    callback: impl FnOnce() + 'static,
) {
    let item = labeled_button(icon, label);
    item.set_halign(gtk::Align::Fill);
    item.set_has_frame(false);
    item.add_css_class("flat");
    let popover = popover.clone();
    let callback = RefCell::new(Some(callback));
    item.connect_clicked(move |_| {
        popover.popdown();
        if let Some(callback) = callback.borrow_mut().take() {
            callback();
        }
    });
    menu.append(&item);
}

pub(in crate::app::window) fn open_with_default_application(state: &Rc<State>, path: &Path) {
    let uri = gio::File::for_path(path).uri();
    if let Err(error) = gio::AppInfo::launch_default_for_uri(&uri, None::<&gio::AppLaunchContext>) {
        show_toast(state, &error.to_string());
    }
}

pub(in crate::app::window) fn open_containing_folder(state: &Rc<State>, path: &Path) {
    let folder = if path.is_dir() {
        path
    } else if let Some(parent) = path.parent() {
        parent
    } else {
        show_toast(state, &tr("Could not find the containing folder."));
        return;
    };
    let uri = gio::File::for_path(folder).uri();
    if let Err(error) = gio::AppInfo::launch_default_for_uri(&uri, None::<&gio::AppLaunchContext>) {
        show_toast(state, &error.to_string());
    }
}

pub(in crate::app::window) fn install_root_drop_target(state: &Rc<State>) {
    let target = gtk::DropTarget::new(String::static_type(), gdk::DragAction::MOVE);
    let weak = Rc::downgrade(state);
    target.connect_drop(move |_, value, _, _| {
        let Ok(path) = value.get::<String>() else {
            return false;
        };
        let Some(state) = weak.upgrade() else {
            return false;
        };
        let Some(root) = state.project.project_root.borrow().clone() else {
            return false;
        };
        move_project_file_to(&state, PathBuf::from(path), root)
    });
    state.project.file_list.add_controller(target);
}

pub(in crate::app::window) fn move_project_file_to(
    state: &Rc<State>,
    source: PathBuf,
    destination: PathBuf,
) -> bool {
    let Some(project_root) = state.project.project_root.borrow().clone() else {
        return false;
    };
    let destination_for_result = destination.clone();
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let result = project::move_file_within_project(&source, &destination, &project_root);
        let _ = sender.send((source, result));
    });
    let state_for_result = state.clone();
    poll_result(receiver, move |(source, result)| match result {
        Ok(new_path) => {
            let root = state_for_result.project.project_root.borrow().clone();
            if root.as_deref() != Some(destination_for_result.as_path()) {
                state_for_result
                    .expanded_folders
                    .borrow_mut()
                    .insert(destination_for_result.clone());
            }
            update_project_file_path(&state_for_result, &source, &new_path);
        }
        Err(error) => show_toast(&state_for_result, &error),
    });
    true
}

pub(in crate::app::window) fn update_project_file_path(
    state: &Rc<State>,
    old_path: &Path,
    new_path: &Path,
) {
    let current = state.project.current_file.borrow().clone();
    let rewritten_current = current
        .as_ref()
        .filter(|path| project::path_contains(old_path, path))
        .map(|path| project::rewrite_path_prefix(path, old_path, new_path));
    if let Some(path) = rewritten_current {
        *state.project.current_file.borrow_mut() = Some(path.clone());
        state.editor.set_file(Some(path));
        update_title(state, state.editor.dirty());
    }
    let main = state.project.main_tex.borrow().clone();
    if let Some(main) = main.filter(|path| project::path_contains(old_path, path)) {
        let rewritten = project::rewrite_path_prefix(&main, old_path, new_path);
        *state.project.main_tex.borrow_mut() = rewritten
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("tex"))
            .then_some(rewritten);
    }
    let target = state.project.project_target.borrow().clone();
    if let Some(target) = target.filter(|path| project::path_contains(old_path, path)) {
        *state.project.project_target.borrow_mut() =
            Some(project::rewrite_path_prefix(&target, old_path, new_path));
    }
    let expanded = std::mem::take(&mut *state.project.expanded_folders.borrow_mut());
    *state.project.expanded_folders.borrow_mut() = expanded
        .into_iter()
        .map(|folder| {
            if project::path_contains(old_path, &folder) {
                project::rewrite_path_prefix(&folder, old_path, new_path)
            } else {
                folder
            }
        })
        .collect();
    update_editor_capabilities(state);
    refresh_project_files(state);
    refresh_build_status(state);
}
