use super::*;

pub(in crate::app::window) fn setup_project_file_view(state: &Rc<State>) {
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
                    state
                        .project
                        .expanded_folders
                        .borrow_mut()
                        .insert(path.clone());
                } else {
                    state.project.expanded_folders.borrow_mut().remove(&path);
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
    state.project.file_tree.set_factory(Some(&factory));

    let state_for_activate = state.clone();
    state
        .project
        .file_tree
        .connect_activate(move |_, position| {
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
            *state_for_activate.project.project_target.borrow_mut() = Some(target);
            if entry.kind == ProjectEntryKind::Directory || entry.kind == ProjectEntryKind::Resource
            {
                return;
            }
            let path = entry.path.clone();
            let root = state_for_activate.project.project_root.borrow().clone();
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
            *state_for_selection.project.project_target.borrow_mut() = Some(target);
        });
}

pub(in crate::app::window) fn install_project_row_controllers(
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

pub(in crate::app::window) fn project_entry_from_list_item(
    item: &gtk::ListItem,
) -> Option<project::ProjectEntry> {
    let tree_row = item.item()?.downcast::<gtk::TreeListRow>().ok()?;
    project_entry_from_tree_row(&tree_row)
}

pub(in crate::app::window) fn project_entry_from_tree_row(
    row: &gtk::TreeListRow,
) -> Option<project::ProjectEntry> {
    let item = row.item()?.downcast::<glib::BoxedAnyObject>().ok()?;
    let entry = item.borrow::<ProjectTreeNode>().entry.clone();
    Some(entry)
}

pub(in crate::app::window) fn project_file_icon(
    entry: &project::ProjectEntry,
    expanded: bool,
) -> &'static str {
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

pub(in crate::app::window) fn build_project_tree_store(
    entries: &[project::ProjectEntry],
) -> gio::ListStore {
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

pub(in crate::app::window) fn restore_expanded_project_folders(
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

pub(in crate::app::window) fn refresh_project_files(state: &Rc<State>) {
    let scroll_position = state.project.project_files_scroll.value();
    let expanded_folders = state.project.expanded_folders.borrow().clone();
    let request_id = state.project.project_scan_id.get().wrapping_add(1);
    state.project.project_scan_id.set(request_id);
    while let Some(child) = state.project.file_list.first_child() {
        state.project.file_list.remove(&child);
    }
    let Some(root) = state.project.project_root.borrow().clone() else {
        state
            .project
            .file_tree_selection
            .set_model(None::<&gio::ListModel>);
        let empty = gtk::Label::new(Some(&tr("Open a project folder to browse its files.")));
        empty.set_xalign(0.0);
        empty.set_wrap(true);
        empty.set_margin_start(12);
        empty.set_margin_end(8);
        empty.add_css_class("dim-label");
        state.project.file_list.append(&empty);
        state.project.project_files_scroll.set_value(0.0);
        return;
    };
    let loading = gtk::Label::new(Some(&tr("Loading project files…")));
    loading.set_xalign(0.0);
    loading.add_css_class("dim-label");
    state.project.file_list.append(&loading);
    let root_for_worker = root.clone();
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let entries = project::entries(&root_for_worker).map_err(|error| error.to_string());
        let _ = sender.send((request_id, root_for_worker, entries));
    });
    let state = state.clone();
    poll_result(receiver, move |(request_id, root, result)| {
        if state.project.project_scan_id.get() != request_id
            || state.project.project_root.borrow().as_deref() != Some(root.as_path())
        {
            return;
        }
        while let Some(child) = state.project.file_list.first_child() {
            state.project.file_list.remove(&child);
        }
        match result {
            Ok(entries) if entries.is_empty() => {
                state
                    .project
                    .file_tree_selection
                    .set_model(None::<&gio::ListModel>);
                let empty = gtk::Label::new(Some(&tr(
                    "This project has no files yet. Use + to create one.",
                )));
                empty.set_xalign(0.0);
                empty.set_wrap(true);
                empty.add_css_class("dim-label");
                state.project.file_list.append(&empty);
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
                state
                    .project
                    .file_tree_selection
                    .set_model(Some(&tree_model));
                *state.project.expanded_folders.borrow_mut() = expanded_folders.clone();
                restore_expanded_project_folders(&tree_model, &expanded_folders);
                if let Some(current_file) = state.project.current_file.borrow().as_ref() {
                    for position in 0..tree_model.n_items() {
                        let Some(row) = tree_model.row(position) else {
                            continue;
                        };
                        if project_entry_from_tree_row(&row)
                            .is_some_and(|entry| entry.path == *current_file)
                        {
                            state.project.file_tree_selection.set_selected(position);
                            break;
                        }
                    }
                }
                state.project.file_list.append(&state.project.file_tree);
            }
            Err(error) => {
                state
                    .project
                    .file_tree_selection
                    .set_model(None::<&gio::ListModel>);
                let message = gtk::Label::new(Some(&format!(
                    "{}: {error}",
                    tr("Could not list the project folder")
                )));
                message.set_xalign(0.0);
                message.set_wrap(true);
                message.add_css_class("dim-label");
                state.project.file_list.append(&message);
                show_toast(
                    &state,
                    &format!("{}: {error}", tr("Could not list the project folder")),
                );
            }
        }
        let adjustment = state.project.project_files_scroll.clone();
        glib::idle_add_local_once(move || adjustment.set_value(scroll_position));
    });
}
