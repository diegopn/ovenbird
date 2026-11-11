use super::*;

pub(in crate::app::window) fn button(icon: &str, tooltip: &str) -> gtk::Button {
    let button = gtk::Button::from_icon_name(icon);
    button.set_tooltip_text(Some(tooltip));
    button
}

pub(in crate::app::window) fn labeled_button(icon: &str, label: &str) -> gtk::Button {
    let button = gtk::Button::new();
    button.set_child(Some(&labeled_content(icon, label)));
    button
}

pub(in crate::app::window) fn sidebar_tab_button(icon: &str, label: &str) -> gtk::ToggleButton {
    let label = tr(label);
    let button = gtk::ToggleButton::new();
    let content = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    content.set_halign(gtk::Align::Center);
    content.append(&gtk::Image::from_icon_name(icon));
    content.append(&gtk::Label::new(Some(&label)));
    button.set_child(Some(&content));
    button.set_tooltip_text(Some(&label));
    button
}

pub(in crate::app::window) fn editor_mode_button(icon: &str, label: &str) -> gtk::ToggleButton {
    let label = tr(label);
    let button = gtk::ToggleButton::new();
    let content = gtk::Box::new(gtk::Orientation::Horizontal, 5);
    let image = gtk::Image::from_icon_name(icon);
    image.set_pixel_size(14);
    content.append(&image);
    content.append(&gtk::Label::new(Some(&label)));
    button.set_child(Some(&content));
    button.set_tooltip_text(Some(&label));
    button
}

pub(in crate::app::window) fn labeled_content(icon: &str, label: &str) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    if !icon.is_empty() {
        row.append(&gtk::Image::from_icon_name(icon));
    }
    row.append(&gtk::Label::new(Some(label)));
    row
}

pub(in crate::app::window) fn build_sidebar(startup_view: StartupView) -> SidebarWidgets {
    let sidebar = gtk::Box::new(gtk::Orientation::Vertical, 0);
    sidebar.set_size_request(340, -1);
    sidebar.set_hexpand(false);
    sidebar.set_vexpand(true);
    sidebar.add_css_class("ovenbird-sidebar");

    let brand = gtk::CenterBox::new();
    brand.set_margin_start(8);
    brand.set_margin_end(8);
    brand.set_size_request(-1, MAIN_HEADER_HEIGHT);
    brand.set_hexpand(true);
    brand.set_valign(gtk::Align::Center);
    brand.add_css_class("ovenbird-sidebar-brand");
    let name = gtk::Label::new(Some("Ovenbird"));
    name.set_hexpand(true);
    name.add_css_class("ovenbird-brand");
    name.set_justify(gtk::Justification::Center);
    name.set_halign(gtk::Align::Center);
    let menu = gtk::MenuButton::new();
    menu.set_icon_name("open-menu-symbolic");
    menu.set_tooltip_text(Some(&tr("Main menu")));
    menu.add_css_class("flat");
    menu.add_css_class("ovenbird-main-menu-button");
    menu.set_popover(Some(&build_main_menu()));
    brand.set_center_widget(Some(&name));
    brand.set_end_widget(Some(&menu));
    sidebar.append(&brand);
    let tabs = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    tabs.set_margin_start(8);
    tabs.set_margin_end(8);
    tabs.set_margin_bottom(4);
    tabs.add_css_class("ovenbird-sidebar-tabs");
    let editor_tab = sidebar_tab_button("document-edit-symbolic", "LaTeX editor");
    editor_tab.set_action_name(Some("win.show-editor"));
    editor_tab.set_active(startup_view == StartupView::Editor);
    editor_tab.set_hexpand(true);
    editor_tab.set_halign(gtk::Align::Fill);
    editor_tab.add_css_class("flat");
    let references_tab = sidebar_tab_button("view-list-symbolic", "References");
    references_tab.set_group(Some(&editor_tab));
    references_tab.set_active(startup_view == StartupView::References);
    references_tab.set_action_name(Some("win.show-references"));
    references_tab.set_hexpand(true);
    references_tab.set_halign(gtk::Align::Fill);
    references_tab.add_css_class("flat");
    tabs.append(&editor_tab);
    tabs.append(&references_tab);
    sidebar.append(&tabs);

    let pages = gtk::Stack::builder()
        .hexpand(true)
        .vexpand(true)
        .transition_type(gtk::StackTransitionType::Crossfade)
        .build();

    let files_panel = gtk::Box::new(gtk::Orientation::Vertical, 2);
    files_panel.set_hexpand(true);
    files_panel.set_vexpand(true);
    let files_header = gtk::Box::new(gtk::Orientation::Horizontal, 2);
    files_header.set_margin_start(12);
    files_header.set_margin_end(8);
    let files_title = gtk::Label::new(Some(&tr("Project files")));
    files_title.set_xalign(0.0);
    files_title.set_hexpand(true);
    files_title.add_css_class("dim-label");
    files_title.add_css_class("sidebar-section");
    files_header.append(&files_title);
    let new_item = gtk::MenuButton::new();
    new_item.set_icon_name("list-add-symbolic");
    new_item.set_tooltip_text(Some(&tr("Create files and folders")));
    new_item.set_popover(Some(&build_new_item_menu()));
    files_header.append(&new_item);
    files_panel.append(&files_header);
    let file_list = gtk::Box::new(gtk::Orientation::Vertical, 1);
    file_list.set_hexpand(true);
    file_list.set_vexpand(true);
    file_list.add_css_class("ovenbird-project-drop-target");
    let file_tree_selection = gtk::SingleSelection::new(None::<gio::ListModel>);
    file_tree_selection.set_autoselect(false);
    file_tree_selection.set_can_unselect(true);
    let file_tree = gtk::ListView::new(
        Some(file_tree_selection.clone()),
        None::<gtk::ListItemFactory>,
    );
    file_tree.set_single_click_activate(true);
    file_tree.add_css_class("navigation-sidebar");
    file_list.append(&file_tree);
    let file_scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Automatic)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .min_content_width(0)
        .max_content_width(340)
        .min_content_height(70)
        .propagate_natural_width(false)
        .hexpand(true)
        .vexpand(true)
        .child(&file_list)
        .build();
    let project_files_scroll = file_scroll.vadjustment();
    files_panel.append(&file_scroll);

    pages.add_named(&files_panel, Some("editor"));

    let references_panel = gtk::Box::new(gtk::Orientation::Vertical, 8);
    references_panel.set_margin_top(10);
    references_panel.set_margin_start(12);
    references_panel.set_margin_end(12);
    references_panel.set_hexpand(true);
    references_panel.set_vexpand(true);
    let references_title = gtk::Label::new(Some(&tr("Local library")));
    references_title.set_xalign(0.0);
    references_title.add_css_class("sidebar-section");
    references_title.add_css_class("dim-label");
    references_panel.append(&references_title);
    for (icon, label, action) in [
        (
            "view-list-symbolic",
            "View all references",
            "win.show-references",
        ),
        ("bookmark-new-symbolic", "Tags", "win.show-tags"),
        ("system-users-symbolic", "Authors", "win.show-authors"),
    ] {
        let item = labeled_button(icon, &tr(label));
        item.set_action_name(Some(action));
        item.set_halign(gtk::Align::Fill);
        item.set_hexpand(true);
        references_panel.append(&item);
    }
    pages.add_named(&references_panel, Some("references"));
    pages.set_visible_child_name(startup_page_name(startup_view));
    sidebar.append(&pages);

    SidebarWidgets {
        root: sidebar,
        #[cfg(test)]
        brand,
        file_list,
        file_tree,
        file_tree_selection,
        project_files_scroll,
        pages,
        editor_tab,
        references_tab,
    }
}

pub(in crate::app::window) fn build_main_menu() -> gtk::Popover {
    let popover = gtk::Popover::new();
    let content = gtk::Box::new(gtk::Orientation::Vertical, 2);
    content.set_margin_top(6);
    content.set_margin_bottom(6);
    content.set_margin_start(6);
    content.set_margin_end(6);
    let append_item = |icon: &str, label: &str, action: &str, shortcut: &str| {
        let item = gtk::Button::new();
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        row.append(&gtk::Image::from_icon_name(icon));
        let label = tr_dynamic(label);
        let title = gtk::Label::new(Some(&label));
        title.set_xalign(0.0);
        title.set_hexpand(true);
        row.append(&title);
        if !shortcut.is_empty() {
            let accelerator = gtk::Label::new(Some(shortcut));
            accelerator.add_css_class("dim-label");
            row.append(&accelerator);
        }
        item.set_child(Some(&row));
        item.set_action_name(Some(action));
        item.set_halign(gtk::Align::Fill);
        item.add_css_class("flat");
        item.add_css_class("ovenbird-app-menu-item");
        content.append(&item);
    };
    for (group_index, group) in crate::application_menu::APPLICATION_MENU_GROUPS
        .iter()
        .enumerate()
    {
        if group_index > 0 {
            content.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
        }
        for item in *group {
            append_item(item.icon, item.label, item.action, item.shortcut);
        }
    }
    popover.set_child(Some(&content));
    popover
}

pub(in crate::app::window) fn build_new_item_menu() -> gtk::Popover {
    let popover = gtk::Popover::new();
    let content = gtk::Box::new(gtk::Orientation::Vertical, 2);
    for (icon, label, action) in [
        (
            "text-x-generic-symbolic",
            "New LaTeX document",
            "win.create-tex",
        ),
        (
            "text-x-generic-symbolic",
            "New bibliography (.bib)",
            "win.create-bib",
        ),
        (
            "text-x-generic-symbolic",
            "New style file (.sty)",
            "win.create-sty",
        ),
    ] {
        let item = labeled_button(icon, &tr_dynamic(label));
        item.set_action_name(Some(action));
        item.set_halign(gtk::Align::Fill);
        item.add_css_class("flat");
        content.append(&item);
    }
    content.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    let folder = labeled_button("folder-new-symbolic", &tr("New folder"));
    folder.set_action_name(Some("win.create-folder"));
    folder.set_halign(gtk::Align::Fill);
    folder.add_css_class("flat");
    content.append(&folder);
    popover.set_child(Some(&content));
    popover
}
