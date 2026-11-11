use super::*;

pub(super) fn setup_project_file_view(state: &Rc<State>) {
    let factory = gtk::SignalListItemFactory::new();
    let state_for_setup = state.clone();
    factory.connect_setup(move |_, object| {
        let Some(item) = object.downcast_ref::<gtk::ListItem>() else {
            return;
        };
        let expander = gtk::TreeExpander::new();
        expander.set_hexpand(true);
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        row.set_hexpand(true);
        row.set_margin_start(4);
        row.set_margin_end(8);
        row.set_margin_top(3);
        row.set_margin_bottom(3);
        row.add_css_class("ovenbird-project-file-row");
        let icon = gtk::Image::new();
        icon.set_pixel_size(16);
        let label = gtk::Label::new(None);
        label.set_xalign(0.0);
        label.set_hexpand(true);
        label.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
        label.add_css_class("ovenbird-project-file-name");
        row.append(&icon);
        row.append(&label);
        expander.set_child(Some(&row));
        item.set_child(Some(&expander));
        install_project_row_controllers(&row, item, &state_for_setup);
    });

    let state_for_bind = state.clone();
    factory.connect_bind(move |_, object| {
        let Some(item) = object.downcast_ref::<gtk::ListItem>() else {
            return;
        };
        let Some(tree_row) = item
            .item()
            .and_then(|item| item.downcast::<gtk::TreeListRow>().ok())
        else {
            return;
        };
        let Some(entry) = project_entry_from_tree_row(&tree_row) else {
            return;
        };
        let Some(expander) = item
            .child()
            .and_then(|child| child.downcast::<gtk::TreeExpander>().ok())
        else {
            return;
        };
        expander.set_list_row(Some(&tree_row));
        let Some(row) = expander
            .child()
            .and_then(|child| child.downcast::<gtk::Box>().ok())
        else {
            return;
        };
        let Some(icon) = row
            .first_child()
            .and_then(|child| child.downcast::<gtk::Image>().ok())
        else {
            return;
        };
        let Some(label) = icon
            .next_sibling()
            .and_then(|child| child.downcast::<gtk::Label>().ok())
        else {
            return;
        };
        label.set_text(&entry.name);
        let is_directory = entry.kind == ProjectEntryKind::Directory;
        if is_directory {
            row.add_css_class("ovenbird-project-drop-target");
        } else {
            row.remove_css_class("ovenbird-project-drop-target");
        }
        icon.set_icon_name(Some(project_file_icon(&entry, tree_row.is_expanded())));
        label.remove_css_class("dim-label");
        if entry.kind == ProjectEntryKind::Resource {
            label.add_css_class("dim-label");
            row.set_tooltip_text(Some(&tr(
                "This file is listed as a project resource and cannot be edited in Ovenbird yet.",
            )));
        } else if entry.kind == ProjectEntryKind::Previewable {
            row.set_tooltip_text(Some(&tr("Open preview in the editor pane.")));
        } else {
            row.set_tooltip_text(None);
        }
        if is_directory {
            let path = entry.path.clone();
            let state = state_for_bind.clone();
            let item_weak = item.downgrade();
            let icon = icon.clone();
            tree_row.connect_expanded_notify(move |tree_row| {
                if tree_row.is_expanded() {
                    state.expanded_folders.borrow_mut().insert(path.clone());
                } else {
                    state.expanded_folders.borrow_mut().remove(&path);
                }
                if item_weak
                    .upgrade()
                    .and_then(|item| project_entry_from_list_item(&item))
                    .is_some_and(|entry| entry.path == path)
                {
                    icon.set_icon_name(Some(if tree_row.is_expanded() {
                        "folder-open-symbolic"
                    } else {
                        "folder-symbolic"
                    }));
                }
            });
        }
    });
    state.file_tree.set_factory(Some(&factory));

    let state_for_activate = state.clone();
    state.file_tree.connect_activate(move |_, position| {
        let Some(model) = state_for_activate
            .file_tree_selection
            .model()
            .and_then(|model| model.downcast::<gtk::TreeListModel>().ok())
        else {
            return;
        };
        let Some(tree_row) = model.row(position) else {
            return;
        };
        let Some(entry) = project_entry_from_tree_row(&tree_row) else {
            return;
        };
        let target = if entry.kind == ProjectEntryKind::Directory {
            entry.path.clone()
        } else {
            entry.path.parent().unwrap_or(&entry.path).to_path_buf()
        };
        *state_for_activate.project_target.borrow_mut() = Some(target);
        if entry.kind == ProjectEntryKind::Directory || entry.kind == ProjectEntryKind::Resource {
            return;
        }
        let path = entry.path.clone();
        let root = state_for_activate.project_root.borrow().clone();
        with_discard_confirmation(&state_for_activate, move |state| {
            open_document(&state, &path, root.clone())
        });
    });

    let state_for_selection = state.clone();
    state
        .file_tree_selection
        .connect_selected_notify(move |selection| {
            let Some(tree_row) = selection
                .selected_item()
                .and_then(|item| item.downcast::<gtk::TreeListRow>().ok())
            else {
                return;
            };
            let Some(entry) = project_entry_from_tree_row(&tree_row) else {
                return;
            };
            let target = if entry.kind == ProjectEntryKind::Directory {
                entry.path
            } else {
                entry.path.parent().unwrap_or(&entry.path).to_path_buf()
            };
            *state_for_selection.project_target.borrow_mut() = Some(target);
        });
}

pub(super) fn install_project_row_controllers(
    row: &gtk::Box,
    item: &gtk::ListItem,
    state: &Rc<State>,
) {
    let context_gesture = gtk::GestureClick::new();
    context_gesture.set_button(gdk::BUTTON_SECONDARY);
    let item_weak = item.downgrade();
    let row_weak = row.downgrade();
    let state_for_context = state.clone();
    context_gesture.connect_pressed(move |_, _, _, _| {
        let (Some(item), Some(row)) = (item_weak.upgrade(), row_weak.upgrade()) else {
            return;
        };
        let Some(entry) = project_entry_from_list_item(&item) else {
            return;
        };
        show_project_file_context_menu(&state_for_context, &row, &entry.path);
    });
    row.add_controller(context_gesture);

    let drag = gtk::DragSource::builder()
        .actions(gdk::DragAction::MOVE)
        .build();
    let item_weak = item.downgrade();
    drag.connect_prepare(move |_, _, _| {
        let item = item_weak.upgrade()?;
        let entry = project_entry_from_list_item(&item)?;
        let value = entry.path.to_string_lossy().to_string().to_value();
        Some(gdk::ContentProvider::for_value(&value))
    });
    row.add_controller(drag);

    let drop_target = gtk::DropTarget::new(String::static_type(), gdk::DragAction::MOVE);
    let item_weak = item.downgrade();
    let state_for_drop = state.clone();
    drop_target.connect_drop(move |_, value, _, _| {
        let Some(item) = item_weak.upgrade() else {
            return false;
        };
        let Some(entry) = project_entry_from_list_item(&item) else {
            return false;
        };
        if entry.kind != ProjectEntryKind::Directory {
            return false;
        }
        let Ok(source) = value.get::<String>() else {
            return false;
        };
        move_project_file_to(&state_for_drop, PathBuf::from(source), entry.path);
        true
    });
    row.add_controller(drop_target);
}

pub(super) fn project_entry_from_list_item(item: &gtk::ListItem) -> Option<project::ProjectEntry> {
    let tree_row = item.item()?.downcast::<gtk::TreeListRow>().ok()?;
    project_entry_from_tree_row(&tree_row)
}

pub(super) fn project_entry_from_tree_row(row: &gtk::TreeListRow) -> Option<project::ProjectEntry> {
    let item = row.item()?.downcast::<glib::BoxedAnyObject>().ok()?;
    let entry = item.borrow::<ProjectTreeNode>().entry.clone();
    Some(entry)
}

pub(super) fn project_file_icon(entry: &project::ProjectEntry, expanded: bool) -> &'static str {
    if entry.kind == ProjectEntryKind::Directory {
        return if expanded {
            "folder-open-symbolic"
        } else {
            "folder-symbolic"
        };
    }
    match project::file_kind(&entry.path) {
        ProjectFileKind::Image => "image-x-generic-symbolic",
        ProjectFileKind::Pdf => "x-office-document-symbolic",
        _ => "text-x-generic-symbolic",
    }
}

pub(super) fn build_project_tree_store(entries: &[project::ProjectEntry]) -> gio::ListStore {
    fn append_children(
        entries: &[project::ProjectEntry],
        position: &mut usize,
        depth: usize,
        store: &gio::ListStore,
    ) {
        while let Some(entry) = entries.get(*position) {
            if entry.depth != depth {
                break;
            }
            *position += 1;
            let children = if entry.kind == ProjectEntryKind::Directory {
                let children = gio::ListStore::new::<glib::BoxedAnyObject>();
                append_children(entries, position, depth + 1, &children);
                Some(children)
            } else {
                None
            };
            store.append(&glib::BoxedAnyObject::new(ProjectTreeNode {
                entry: entry.clone(),
                children,
            }));
        }
    }

    let store = gio::ListStore::new::<glib::BoxedAnyObject>();
    let mut position = 0;
    append_children(entries, &mut position, 0, &store);
    store
}

pub(super) fn restore_expanded_project_folders(
    model: &gtk::TreeListModel,
    expanded_folders: &HashSet<PathBuf>,
) {
    let mut position = 0;
    while position < model.n_items() {
        let Some(row) = model.row(position) else {
            break;
        };
        if row.is_expandable()
            && project_entry_from_tree_row(&row)
                .is_some_and(|entry| expanded_folders.contains(&entry.path))
        {
            row.set_expanded(true);
        }
        position += 1;
    }
}

pub(super) fn refresh_project_files(state: &Rc<State>) {
    let scroll_position = state.project_files_scroll.value();
    let expanded_folders = state.expanded_folders.borrow().clone();
    let request_id = state.project_scan_id.get().wrapping_add(1);
    state.project_scan_id.set(request_id);
    while let Some(child) = state.file_list.first_child() {
        state.file_list.remove(&child);
    }
    let Some(root) = state.project_root.borrow().clone() else {
        state.file_tree_selection.set_model(None::<&gio::ListModel>);
        let empty = gtk::Label::new(Some(&tr("Open a project folder to browse its files.")));
        empty.set_xalign(0.0);
        empty.set_wrap(true);
        empty.set_margin_start(12);
        empty.set_margin_end(8);
        empty.add_css_class("dim-label");
        state.file_list.append(&empty);
        state.project_files_scroll.set_value(0.0);
        return;
    };
    let loading = gtk::Label::new(Some(&tr("Loading project files…")));
    loading.set_xalign(0.0);
    loading.add_css_class("dim-label");
    state.file_list.append(&loading);
    let root_for_worker = root.clone();
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let entries = project::entries(&root_for_worker).map_err(|error| error.to_string());
        let _ = sender.send((request_id, root_for_worker, entries));
    });
    let state = state.clone();
    poll_result(receiver, move |(request_id, root, result)| {
        if state.project_scan_id.get() != request_id
            || state.project_root.borrow().as_deref() != Some(root.as_path())
        {
            return;
        }
        while let Some(child) = state.file_list.first_child() {
            state.file_list.remove(&child);
        }
        match result {
            Ok(entries) if entries.is_empty() => {
                state.file_tree_selection.set_model(None::<&gio::ListModel>);
                let empty = gtk::Label::new(Some(&tr(
                    "This project has no files yet. Use + to create one.",
                )));
                empty.set_xalign(0.0);
                empty.set_wrap(true);
                empty.add_css_class("dim-label");
                state.file_list.append(&empty);
            }
            Ok(entries) => {
                let root_store = build_project_tree_store(&entries);
                let tree_model = gtk::TreeListModel::new(root_store, false, false, |item| {
                    let node = item.downcast_ref::<glib::BoxedAnyObject>()?;
                    let node = node.borrow::<ProjectTreeNode>();
                    node.children
                        .as_ref()
                        .filter(|children| children.n_items() > 0)
                        .map(|children| children.clone().upcast())
                });
                state.file_tree_selection.set_model(Some(&tree_model));
                *state.expanded_folders.borrow_mut() = expanded_folders.clone();
                restore_expanded_project_folders(&tree_model, &expanded_folders);
                if let Some(current_file) = state.current_file.borrow().as_ref() {
                    for position in 0..tree_model.n_items() {
                        let Some(row) = tree_model.row(position) else {
                            continue;
                        };
                        if project_entry_from_tree_row(&row)
                            .is_some_and(|entry| entry.path == *current_file)
                        {
                            state.file_tree_selection.set_selected(position);
                            break;
                        }
                    }
                }
                state.file_list.append(&state.file_tree);
            }
            Err(error) => {
                state.file_tree_selection.set_model(None::<&gio::ListModel>);
                let message = gtk::Label::new(Some(&format!(
                    "{}: {error}",
                    tr("Could not list the project folder")
                )));
                message.set_xalign(0.0);
                message.set_wrap(true);
                message.add_css_class("dim-label");
                state.file_list.append(&message);
                show_toast(
                    &state,
                    &format!("{}: {error}", tr("Could not list the project folder")),
                );
            }
        }
        let adjustment = state.project_files_scroll.clone();
        glib::idle_add_local_once(move || adjustment.set_value(scroll_position));
    });
}

pub(super) fn show_project_file_context_menu(state: &Rc<State>, anchor: &gtk::Box, path: &Path) {
    let file = path.to_path_buf();
    let is_directory = file.is_dir();
    let target = if is_directory {
        file.clone()
    } else {
        file.parent().unwrap_or(&file).to_path_buf()
    };
    *state.project_target.borrow_mut() = Some(target.clone());
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
                *state.project_target.borrow_mut() = Some(target);
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
                *state.project_target.borrow_mut() = Some(target);
                create_project_folder(state);
            },
        );
        menu.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    } else if project::openable_extension(&file) {
        let state = state.clone();
        let file = file.clone();
        let root = state.project_root.borrow().clone();
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

pub(super) fn append_file_context_action(
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

pub(super) fn open_with_default_application(state: &Rc<State>, path: &Path) {
    let uri = gio::File::for_path(path).uri();
    if let Err(error) = gio::AppInfo::launch_default_for_uri(&uri, None::<&gio::AppLaunchContext>) {
        show_toast(state, &error.to_string());
    }
}

pub(super) fn open_containing_folder(state: &Rc<State>, path: &Path) {
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

pub(super) fn install_root_drop_target(state: &Rc<State>) {
    let target = gtk::DropTarget::new(String::static_type(), gdk::DragAction::MOVE);
    let weak = Rc::downgrade(state);
    target.connect_drop(move |_, value, _, _| {
        let Ok(path) = value.get::<String>() else {
            return false;
        };
        let Some(state) = weak.upgrade() else {
            return false;
        };
        let Some(root) = state.project_root.borrow().clone() else {
            return false;
        };
        move_project_file_to(&state, PathBuf::from(path), root)
    });
    state.file_list.add_controller(target);
}

pub(super) fn move_project_file_to(
    state: &Rc<State>,
    source: PathBuf,
    destination: PathBuf,
) -> bool {
    let Some(project_root) = state.project_root.borrow().clone() else {
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
            let root = state_for_result.project_root.borrow().clone();
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

pub(super) fn update_project_file_path(state: &Rc<State>, old_path: &Path, new_path: &Path) {
    let current = state.current_file.borrow().clone();
    let rewritten_current = current
        .as_ref()
        .filter(|path| project::path_contains(old_path, path))
        .map(|path| project::rewrite_path_prefix(path, old_path, new_path));
    if let Some(path) = rewritten_current {
        *state.current_file.borrow_mut() = Some(path.clone());
        state.editor.set_file(Some(path));
        update_title(state, state.editor.dirty());
    }
    let main = state.main_tex.borrow().clone();
    if let Some(main) = main.filter(|path| project::path_contains(old_path, path)) {
        let rewritten = project::rewrite_path_prefix(&main, old_path, new_path);
        *state.main_tex.borrow_mut() = rewritten
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("tex"))
            .then_some(rewritten);
    }
    let target = state.project_target.borrow().clone();
    if let Some(target) = target.filter(|path| project::path_contains(old_path, path)) {
        *state.project_target.borrow_mut() =
            Some(project::rewrite_path_prefix(&target, old_path, new_path));
    }
    let expanded = std::mem::take(&mut *state.expanded_folders.borrow_mut());
    *state.expanded_folders.borrow_mut() = expanded
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

pub(super) fn create_project_file(state: &Rc<State>, extension: &str) {
    let extension = extension.to_owned();
    with_discard_confirmation(state, move |state| {
        create_project_file_confirmed(&state, &extension)
    });
}

pub(super) fn create_project_file_confirmed(state: &Rc<State>, extension: &str) {
    let extension = extension.to_owned();
    if let Some(folder) = state
        .project_target
        .borrow()
        .clone()
        .or_else(|| state.project_root.borrow().clone())
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
                *state.project_target.borrow_mut() = Some(folder.clone());
                prompt_create_file(&state, &folder, &extension);
            },
        );
    }
}

pub(super) fn prompt_create_file(state: &Rc<State>, folder: &Path, extension: &str) {
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
                    *state_for_result.main_tex.borrow_mut() = Some(path.clone());
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

pub(super) fn create_project_folder(state: Rc<State>) {
    let folder = state
        .project_target
        .borrow()
        .clone()
        .or_else(|| state.project_root.borrow().clone());
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
                *state.project_target.borrow_mut() = Some(folder.clone());
                prompt_create_folder(&state, &folder);
            },
        );
    }
}

pub(super) fn prompt_create_folder(state: &Rc<State>, parent: &Path) {
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
                *state_for_result.project_target.borrow_mut() = Some(folder);
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

pub(super) fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.trim() == name
        && name != "."
        && name != ".."
        && !name
            .chars()
            .any(|ch| ch == '/' || ch == '\\' || ch.is_control())
}

pub(super) fn rename_project_file(state: &Rc<State>, path: &Path) {
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
    let root = state.project_root.borrow().clone();
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

pub(super) fn trash_project_file(state: &Rc<State>, path: &Path) {
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

pub(super) fn trash_project_file_now(state: &Rc<State>, path: &Path) {
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

pub(super) fn finish_project_file_removal(state: &Rc<State>, path: &Path, success_message: &str) {
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
        state.current_file.borrow_mut().take();
        state.file_encoding.set(project::TextEncoding::Utf8);
        state.editor.set_file(None);
        state.editor.load_text(&LatexEditor::new_document_text());
        state.image_preview.set_filename(None::<&Path>);
        state
            .file_pdf_preview
            .set_message(&tr("Select a PDF file to preview it here."));
        state.editor.set_mode(EditorMode::Code);
        state.code_mode_button.set_active(true);
        state.visual_mode_button.set_active(false);
        update_editor_capabilities(state);
        update_title(state, false);
    }
    if main_removed {
        *state.main_tex.borrow_mut() = None;
    }
    if let Some(root) = state.project_root.borrow().clone() {
        if state
            .project_target
            .borrow()
            .as_ref()
            .is_some_and(|target| project::path_contains(path, target))
        {
            *state.project_target.borrow_mut() = Some(root.clone());
        }
        state
            .expanded_folders
            .borrow_mut()
            .retain(|folder| !project::path_contains(path, folder));
        if main_removed {
            open_project(state, &root, false, state.startup_view);
        } else if current_removed {
            if let Some(main) = state.main_tex.borrow().clone() {
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

pub(super) fn show_template_picker(state: Rc<State>) {
    let dialog = gtk::Dialog::builder()
        .title(tr("New project"))
        .transient_for(&state.window)
        .modal(true)
        .default_width(680)
        .default_height(560)
        .build();
    dialog.add_button(&tr("Cancel"), gtk::ResponseType::Cancel);
    dialog.add_button(&tr("Continue"), gtk::ResponseType::Accept);
    dialog.set_default_response(gtk::ResponseType::Accept);
    let content = dialog.content_area();
    content.set_spacing(12);
    content.set_margin_top(18);
    content.set_margin_bottom(18);
    content.set_margin_start(20);
    content.set_margin_end(20);
    let introduction = gtk::Label::new(Some(&tr(
        "Choose a starting point. Template files are copied into a local project folder.",
    )));
    introduction.set_xalign(0.0);
    introduction.set_wrap(true);
    content.append(&introduction);

    let choices = [
        (
            tr("Blank LaTeX document"),
            tr("Start a new document with a simple article structure."),
            "",
        ),
        (
            tr("ABNT · abnTeX2"),
            tr("Academic work, articles, reports, and research projects based on ABNT rules."),
            "https://ctan.org/pkg/abntex2?lang=en",
        ),
        (
            tr("IEEE · IEEEtran"),
            tr("IEEE conference and journal papers using the IEEEtran class."),
            "https://journals.ieeeauthorcenter.ieee.org/create-your-ieee-journal-article/authoring-tools-and-templates/tools-for-ieee-authors/ieee-article-templates/",
        ),
        (
            tr("ACM · acmart"),
            tr("ACM journal and conference papers using the acmart class."),
            "https://www.acm.org/publications/proceedings-template",
        ),
        (
            tr("Elsevier · elsarticle"),
            tr("Elsevier manuscripts using the elsarticle package."),
            "https://www.elsevier.com/en-gb/researcher/author/policies-and-guidelines/latex-instructions",
        ),
        (
            tr("Springer Nature"),
            tr("Springer Nature journal article templates and sample files."),
            "https://www.springernature.com/gp/authors/campaigns/latex-author-support",
        ),
    ];
    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::Single);
    list.add_css_class("boxed-list");
    for (name, description, source) in choices {
        let row = gtk::ListBoxRow::new();
        row.set_activatable(true);
        let row_content = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        row_content.set_margin_top(8);
        row_content.set_margin_bottom(8);
        row_content.set_margin_start(10);
        row_content.set_margin_end(10);
        let details = gtk::Box::new(gtk::Orientation::Vertical, 3);
        details.set_hexpand(true);
        let title = gtk::Label::new(Some(&name));
        title.set_xalign(0.0);
        title.add_css_class("heading");
        let subtitle = gtk::Label::new(Some(&description));
        subtitle.set_xalign(0.0);
        subtitle.set_wrap(true);
        subtitle.add_css_class("dim-label");
        details.append(&title);
        details.append(&subtitle);
        row_content.append(&details);
        if !source.is_empty() {
            let source_link = gtk::LinkButton::with_label(source, &tr("Source"));
            source_link.set_valign(gtk::Align::Center);
            row_content.append(&source_link);
        } else {
            row_content.prepend(&gtk::Image::from_icon_name("document-new-symbolic"));
        }
        row.set_child(Some(&row_content));
        list.append(&row);
    }
    list.select_row(list.row_at_index(0).as_ref());
    let scroll = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .child(&list)
        .build();
    content.append(&scroll);
    let note = gtk::Label::new(Some(&tr(
        "Journal and conference requirements can differ. Check the current author instructions before submission.",
    )));
    note.set_xalign(0.0);
    note.set_wrap(true);
    note.add_css_class("dim-label");
    content.append(&note);

    let state2 = state.clone();
    let dialog_for_row = dialog.clone();
    list.connect_row_activated(move |_, _| dialog_for_row.response(gtk::ResponseType::Accept));
    dialog.connect_response(move |dialog, response| {
        if response != gtk::ResponseType::Accept {
            dialog.close();
            return;
        }
        let Some(row) = list.selected_row() else {
            return;
        };
        let id = match row.index() {
            1 => "abntex2",
            2 => "ieeetran",
            3 => "acmart",
            4 => "elsarticle",
            5 => "springer-nature",
            _ => "blank",
        };
        dialog.close();
        choose_file(
            &state2,
            gtk::FileChooserAction::SelectFolder,
            "Choose where to create the project",
            "Create",
            None,
            move |state, parent| {
                prompt_project_name(&state, &parent, id);
            },
        );
    });
    dialog.present();
}

pub(super) fn prompt_project_name(state: &Rc<State>, parent: &Path, id: &str) {
    let dialog = gtk::Dialog::builder()
        .title(tr("New project"))
        .transient_for(&state.window)
        .modal(true)
        .build();
    dialog.add_button(&tr("Cancel"), gtk::ResponseType::Cancel);
    dialog.add_button(&tr("Create"), gtk::ResponseType::Accept);
    dialog.set_default_response(gtk::ResponseType::Accept);
    let name = gtk::Entry::new();
    name.set_text("Ovenbird project");
    name.set_activates_default(true);
    name.set_margin_top(18);
    name.set_margin_bottom(18);
    name.set_margin_start(20);
    name.set_margin_end(20);
    dialog.content_area().append(&name);
    let name_for_response = name.clone();
    let parent = parent.to_path_buf();
    let id = id.to_owned();
    let state2 = state.clone();
    dialog.connect_response(move |dialog, response| {
        if response != gtk::ResponseType::Accept {
            dialog.close();
            return;
        }
        let project_name = name_for_response.text().to_string();
        if !crate::templates::valid_project_name(&project_name) {
            show_toast(&state2, &tr("Enter a valid project name"));
            return;
        }
        let template = if id == "blank" {
            None
        } else {
            crate::templates::TEMPLATES
                .iter()
                .find(|template| template.id == id)
                .copied()
        };
        if let Some(template) = template {
            let receiver = crate::templates::create_project_async(
                template,
                parent.clone(),
                project_name.clone(),
                template_resource_root(),
            );
            dialog.close();
            let progress = gtk::Dialog::builder()
                .title(tr("Creating project"))
                .transient_for(&state2.window)
                .modal(true)
                .deletable(false)
                .build();
            let progress_content = progress.content_area();
            progress_content.set_spacing(12);
            progress_content.set_margin_top(20);
            progress_content.set_margin_bottom(20);
            progress_content.set_margin_start(24);
            progress_content.set_margin_end(24);
            let spinner = gtk::Spinner::new();
            spinner.start();
            spinner.set_halign(gtk::Align::Center);
            progress_content.append(&spinner);
            let progress_label = gtk::Label::new(Some(&tr("Preparing template project…")));
            progress_label.set_wrap(true);
            progress_content.append(&progress_label);
            progress.present();
            state2.status.set_label(&tr("Creating project…"));
            let state_for_result = state2.clone();
            poll_result(receiver, move |result| {
                progress.close();
                state_for_result.status.set_label(&tr("Ready"));
                match result {
                    Ok((folder, main)) => {
                        state_for_result.expanded_folders.borrow_mut().clear();
                        open_document(&state_for_result, &main, Some(folder));
                    }
                    Err(error) => show_toast(
                        &state_for_result,
                        &format!("{}: {error}", tr("Could not create the project")),
                    ),
                }
            });
            return;
        }
        let folder = parent.join(&project_name);
        let folder_for_worker = folder.clone();
        let source = LatexEditor::new_document_text();
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let result = std::fs::create_dir(&folder_for_worker)
                .map_err(|error| error.to_string())
                .and_then(|()| {
                    crate::storage::atomic_write(
                        &folder_for_worker.join("main.tex"),
                        source.as_bytes(),
                    )
                    .map_err(|error| {
                        let _ = std::fs::remove_dir(&folder_for_worker);
                        error.to_string()
                    })
                });
            let _ = sender.send(result.map(|()| {
                (
                    folder_for_worker.clone(),
                    folder_for_worker.join("main.tex"),
                )
            }));
        });
        let dialog_for_result = dialog.clone();
        let state_for_result = state2.clone();
        poll_result(receiver, move |result| match result {
            Ok((folder, main)) => {
                state_for_result.expanded_folders.borrow_mut().clear();
                open_document(&state_for_result, &main, Some(folder));
                dialog_for_result.close();
            }
            Err(error) => show_toast(
                &state_for_result,
                &format!("{}: {error}", tr("Could not create the project")),
            ),
        });
    });
    dialog.present();
    name.grab_focus();
}

pub(super) fn template_resource_root() -> PathBuf {
    let installed = option_env!("OVENBIRD_TEMPLATE_DIR").map(Path::new);
    crate::templates::resource_root(installed, Path::new(env!("CARGO_MANIFEST_DIR")))
}
