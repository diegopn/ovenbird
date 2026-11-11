use super::*;

pub(super) fn button(icon: &str, tooltip: &str) -> gtk::Button {
    let button = gtk::Button::from_icon_name(icon);
    button.set_tooltip_text(Some(tooltip));
    button
}

pub(super) fn labeled_button(icon: &str, label: &str) -> gtk::Button {
    let button = gtk::Button::new();
    button.set_child(Some(&labeled_content(icon, label)));
    button
}

pub(super) fn sidebar_tab_button(icon: &str, label: &str) -> gtk::ToggleButton {
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

pub(super) fn editor_mode_button(icon: &str, label: &str) -> gtk::ToggleButton {
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

pub(super) fn labeled_content(icon: &str, label: &str) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    if !icon.is_empty() {
        row.append(&gtk::Image::from_icon_name(icon));
    }
    row.append(&gtk::Label::new(Some(label)));
    row
}

pub(super) fn build_sidebar(startup_view: StartupView) -> SidebarWidgets {
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

pub(super) fn build_main_menu() -> gtk::Popover {
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

pub(super) fn build_new_item_menu() -> gtk::Popover {
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

pub(super) fn build_editor_page(editor: &LatexEditor) -> EditorPageWidgets {
    let editor_panel = gtk::Box::new(gtk::Orientation::Vertical, 0);
    editor_panel.add_css_class("ovenbird-code-panel");
    let top = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    top.add_css_class("ovenbird-mode-toolbar");
    top.set_hexpand(true);
    top.set_margin_top(5);
    top.set_margin_bottom(4);
    top.set_margin_start(6);
    top.set_margin_end(6);
    let code_mode_button = editor_mode_button("text-x-generic-symbolic", "Code");
    code_mode_button.set_action_name(Some("win.mode-code"));
    code_mode_button.set_active(true);
    let visual_mode_button = editor_mode_button("format-text-rich-symbolic", "Visual");
    visual_mode_button.set_group(Some(&code_mode_button));
    visual_mode_button.set_action_name(Some("win.mode-visual"));
    top.append(&code_mode_button);
    top.append(&visual_mode_button);
    let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    top.append(&spacer);
    let find_document = button("system-search-symbolic", &tr("Find in document"));
    find_document.set_action_name(Some("win.find-document"));
    top.append(&find_document);
    let mode_help = tr("Switch modes at any time without creating another version of the document");
    code_mode_button.set_tooltip_text(Some(&mode_help));
    visual_mode_button.set_tooltip_text(Some(&mode_help));
    editor_panel.append(&top);

    let search = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    search.set_size_request(0, -1);
    search.set_hexpand(true);
    search.set_halign(gtk::Align::Fill);
    search.set_margin_start(6);
    search.set_margin_end(6);
    search.set_margin_bottom(6);
    search.set_visible(false);
    let entry = gtk::SearchEntry::new();
    entry.set_placeholder_text(Some(&tr("Find in document…")));
    entry.set_width_chars(1);
    entry.set_hexpand(true);
    search.append(&entry);
    for (icon, action, tip) in [
        ("go-up-symbolic", "win.search-previous", "Previous match"),
        ("go-down-symbolic", "win.search-next", "Next match"),
    ] {
        let b = button(icon, &tr_dynamic(tip));
        b.set_action_name(Some(action));
        search.append(&b);
    }
    let result = gtk::Label::new(None);
    result.add_css_class("dim-label");
    search.append(&result);
    let close = button("window-close-symbolic", &tr("Close search"));
    close.set_action_name(Some("win.find-document"));
    search.append(&close);
    editor_panel.append(&search);

    let toolbar = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    toolbar.add_css_class("ovenbird-editor-toolbar");
    toolbar.set_hexpand(true);
    toolbar.set_margin_start(8);
    toolbar.set_margin_end(8);
    toolbar.set_margin_bottom(6);
    let latex_toolbar = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    let undo_button = action_button("edit-undo-symbolic", "Undo", "win.undo");
    let redo_button = action_button("edit-redo-symbolic", "Redo", "win.redo");
    undo_button.set_sensitive(false);
    redo_button.set_sensitive(false);
    toolbar.append(&undo_button);
    toolbar.append(&redo_button);
    latex_toolbar.append(&gtk::Separator::new(gtk::Orientation::Vertical));
    let style = gtk::MenuButton::new();
    style.set_label(&tr("Normal text"));
    style.set_popover(Some(&heading_menu()));
    latex_toolbar.append(&style);
    latex_toolbar.append(&gtk::Separator::new(gtk::Orientation::Vertical));
    for (icon, label, action) in [
        ("format-text-bold-symbolic", "Bold", "win.format-bold"),
        ("format-text-italic-symbolic", "Italic", "win.format-italic"),
        (
            "format-text-underline-symbolic",
            "Underline",
            "win.format-underline",
        ),
        (
            "format-text-plaintext-symbolic",
            "Monospace",
            "win.format-monospace",
        ),
    ] {
        let b = button(icon, &tr_dynamic(label));
        b.set_action_name(Some(action));
        latex_toolbar.append(&b);
    }
    let link = action_button("mail-attachment-symbolic", "Insert link", "win.insert-link");
    latex_toolbar.append(&link);
    latex_toolbar.append(&gtk::Separator::new(gtk::Orientation::Vertical));
    let math = gtk::MenuButton::new();
    math.set_label(&tr("Math"));
    math.set_popover(Some(&math_menu()));
    latex_toolbar.append(&math);
    let symbols = gtk::MenuButton::new();
    symbols.set_label("Ω");
    symbols.set_tooltip_text(Some(&tr("Math symbols")));
    symbols.set_popover(Some(&symbol_menu()));
    latex_toolbar.append(&symbols);
    let cite = gtk::Button::with_label(&tr("Insert citation"));
    cite.set_tooltip_text(Some(&tr("Find and insert a bibliography reference")));
    cite.set_action_name(Some("win.insert-citation"));
    latex_toolbar.append(&cite);
    latex_toolbar.append(&gtk::Separator::new(gtk::Orientation::Vertical));
    let lists = gtk::MenuButton::new();
    lists.set_icon_name("view-list-symbolic");
    lists.set_tooltip_text(Some(&tr("Lists and indentation")));
    lists.set_popover(Some(&list_menu()));
    latex_toolbar.append(&lists);
    let more = gtk::MenuButton::new();
    more.set_icon_name("view-more-symbolic");
    more.set_tooltip_text(Some(&tr("More")));
    more.set_popover(Some(&more_menu()));
    latex_toolbar.append(&more);
    toolbar.append(&latex_toolbar);
    let toolbar_spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    toolbar_spacer.set_hexpand(true);
    toolbar.append(&toolbar_spacer);
    let bibtex_citation = gtk::Button::with_label(&tr("Insert citation"));
    bibtex_citation.set_tooltip_text(Some(&tr("Find and insert a bibliography reference")));
    bibtex_citation.set_action_name(Some("win.insert-bibtex-reference"));
    bibtex_citation.set_visible(false);
    toolbar.append(&bibtex_citation);
    let toolbar_scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Automatic)
        .vscrollbar_policy(gtk::PolicyType::Never)
        .min_content_width(0)
        .propagate_natural_width(false)
        .hexpand(true)
        .child(&toolbar)
        .build();
    editor_panel.append(&toolbar_scroll);
    let file_preview_stack = gtk::Stack::builder().hexpand(true).vexpand(true).build();
    file_preview_stack.add_named(&editor.root, Some("text"));
    let image_preview = gtk::Picture::new();
    image_preview.set_can_shrink(true);
    image_preview.set_content_fit(gtk::ContentFit::Contain);
    let image_scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Automatic)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .hexpand(true)
        .vexpand(true)
        .child(&image_preview)
        .build();
    file_preview_stack.add_named(&image_scroll, Some("image"));
    let file_pdf_preview = crate::pdf_preview::PdfPreview::new();
    file_preview_stack.add_named(file_pdf_preview.widget(), Some("pdf"));
    file_preview_stack.set_visible_child_name("text");
    editor_panel.append(&file_preview_stack);

    let preview = crate::pdf_preview::PdfPreview::new();
    let page = gtk::Box::new(gtk::Orientation::Vertical, 0);
    page.add_css_class("ovenbird-editor-page");
    page.set_hexpand(true);
    page.set_vexpand(true);
    editor_panel.set_hexpand(true);
    editor_panel.set_vexpand(true);
    let workspace = gtk::Paned::new(gtk::Orientation::Horizontal);
    workspace.set_wide_handle(true);
    workspace.set_resize_start_child(true);
    workspace.set_resize_end_child(true);
    workspace.set_shrink_start_child(true);
    workspace.set_shrink_end_child(true);
    workspace.set_start_child(Some(&editor_panel));
    workspace.set_end_child(Some(preview.widget()));
    workspace.set_hexpand(true);
    workspace.set_vexpand(true);
    page.append(&workspace);
    (
        editor_panel,
        page,
        preview,
        file_pdf_preview,
        file_preview_stack,
        image_preview,
        top,
        toolbar_scroll,
        latex_toolbar,
        undo_button,
        redo_button,
        code_mode_button,
        visual_mode_button,
        find_document,
        bibtex_citation,
    )
}

pub(super) fn action_button(icon: &str, tooltip: &str, action: &str) -> gtk::Button {
    let b = button(icon, &tr_dynamic(tooltip));
    b.set_action_name(Some(action));
    b
}

pub(super) fn menu_with_actions(items: &[(&str, &str, &str)]) -> gtk::Popover {
    let popover = gtk::Popover::new();
    let content = gtk::Box::new(gtk::Orientation::Vertical, 2);
    content.set_margin_top(5);
    content.set_margin_bottom(5);
    content.set_margin_start(5);
    content.set_margin_end(5);
    for (icon, label, action) in items {
        if label.is_empty() {
            content.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
            continue;
        }
        let item = labeled_button(icon, &tr_dynamic(label));
        item.set_action_name(Some(action));
        item.set_halign(gtk::Align::Fill);
        item.add_css_class("flat");
        item.add_css_class("ovenbird-app-menu-item");
        content.append(&item);
    }
    popover.set_child(Some(&content));
    popover
}

pub(super) fn action_menu(items: &[(&str, &str)]) -> gio::Menu {
    let model = gio::Menu::new();
    for (label, action) in items {
        model.append(Some(&tr_dynamic(label)), Some(action));
    }
    model
}

pub(super) fn heading_menu() -> gtk::Popover {
    menu_with_actions(&[
        (
            "format-justify-left-symbolic",
            "Normal text",
            "win.heading-normal",
        ),
        (
            "format-text-rich-symbolic",
            "Section",
            "win.heading-section",
        ),
        (
            "format-text-rich-symbolic",
            "Subsection",
            "win.heading-subsection",
        ),
        (
            "format-text-rich-symbolic",
            "Subsubsection",
            "win.heading-subsubsection",
        ),
        (
            "format-text-rich-symbolic",
            "Paragraph",
            "win.heading-paragraph",
        ),
        (
            "format-text-rich-symbolic",
            "Subparagraph",
            "win.heading-subparagraph",
        ),
    ])
}

pub(super) fn math_menu() -> gtk::Popover {
    menu_with_actions(&[
        ("", "Inline math", "win.math-inline"),
        ("", "Display math", "win.math-display"),
    ])
}

pub(super) fn symbol_menu() -> gtk::Popover {
    menu_with_actions(&[
        ("", "α  Alpha", "win.insert-alpha"),
        ("", "β  Beta", "win.insert-beta"),
        ("", "γ  Gamma", "win.insert-gamma"),
        ("", "π  Pi", "win.insert-pi"),
        ("", "≤  Less than or equal to", "win.insert-leq"),
        ("", "≥  Greater than or equal to", "win.insert-geq"),
        ("", "≠  Not equal to", "win.insert-neq"),
        ("", "×  Multiplication", "win.insert-times"),
        ("", "±  Plus or minus", "win.insert-pm"),
        ("", "∞  Infinity", "win.insert-infty"),
        ("", "∑  Summation", "win.insert-sum"),
        ("", "∫  Integral", "win.insert-int"),
        ("", "√  Square root", "win.insert-sqrt"),
    ])
}

pub(super) fn list_menu() -> gtk::Popover {
    menu_with_actions(&[
        (
            "view-list-bullet-symbolic",
            "Bulleted list",
            "win.list-itemize",
        ),
        (
            "view-list-ordered-symbolic",
            "Numbered list",
            "win.list-enumerate",
        ),
        (
            "format-text-rich-symbolic",
            "Quotation block",
            "win.list-quote",
        ),
        (
            "format-indent-more-symbolic",
            "Increase indent",
            "win.indent-more",
        ),
        (
            "format-indent-less-symbolic",
            "Decrease indent",
            "win.indent-less",
        ),
    ])
}

pub(super) fn more_menu() -> gtk::Popover {
    menu_with_actions(&[
        ("insert-image-symbolic", "Insert image", "win.insert-image"),
        ("view-grid-symbolic", "Insert table", "win.insert-table"),
        (
            "chat-message-new-symbolic",
            "LaTeX comment",
            "win.insert-comment",
        ),
        ("insert-text-symbolic", "Footnote", "win.insert-footnote"),
        (
            "document-edit-symbolic",
            "Review note",
            "win.insert-review-note",
        ),
        ("bookmark-new-symbolic", "Add label", "win.insert-label"),
        (
            "insert-link-symbolic",
            "Cross-reference",
            "win.insert-reference",
        ),
    ])
}

pub(super) fn sort_direction_button() -> gtk::ToggleButton {
    let ascending = tr("Ascending");
    let descending = tr("Descending");
    let button = gtk::ToggleButton::new();
    button.set_child(Some(&labeled_content(
        "view-sort-ascending-symbolic",
        &ascending,
    )));
    button.set_tooltip_text(Some(&ascending));
    button.connect_toggled(move |button| {
        let (icon, label) = if button.is_active() {
            ("view-sort-descending-symbolic", descending.as_str())
        } else {
            ("view-sort-ascending-symbolic", ascending.as_str())
        };
        button.set_child(Some(&labeled_content(icon, label)));
        button.set_tooltip_text(Some(label));
    });
    button
}

pub(super) fn build_library_page() -> LibraryPageWidgets {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 10);
    page.set_margin_top(20);
    page.set_margin_bottom(12);
    page.set_margin_start(22);
    page.set_margin_end(22);

    let header = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let title = gtk::Label::new(Some(&tr("Manage references")));
    title.set_xalign(0.0);
    title.add_css_class("title-2");
    title.set_hexpand(true);
    header.append(&title);
    let add = labeled_button("list-add-symbolic", &tr("Add reference"));
    add.set_action_name(Some("win.add-reference"));
    add.add_css_class("suggested-action");
    header.append(&add);
    page.append(&header);

    let search_and_sort = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let search = gtk::SearchEntry::new();
    search.set_placeholder_text(Some(&tr("Search references…")));
    search.set_hexpand(true);
    search_and_sort.append(&search);
    let sort_by =
        gtk::DropDown::from_strings(&[&tr("Title"), &tr("Author"), &tr("Year"), &tr("Tag")]);
    sort_by.set_tooltip_text(Some(&tr("Sort by")));
    sort_by.set_selected(0);
    search_and_sort.append(&sort_by);
    let sort_descending = sort_direction_button();
    search_and_sort.append(&sort_descending);
    page.append(&search_and_sort);

    let filter_bar = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    filter_bar.add_css_class("ovenbird-reference-filters");
    let author = reference_filter_dropdown("All authors");
    let year = reference_filter_dropdown("All years");
    let tag = reference_filter_dropdown("All tags");
    let kind = reference_filter_dropdown("All types");
    for dropdown in [&author, &year, &tag, &kind] {
        dropdown.set_hexpand(true);
        dropdown.set_width_request(150);
        filter_bar.append(dropdown);
    }
    page.append(&filter_bar);
    let list_model = gio::ListStore::new::<glib::BoxedAnyObject>();
    let list_selection = gtk::NoSelection::new(Some(list_model.clone()));
    let list = gtk::ListView::new(Some(list_selection), None::<gtk::ListItemFactory>);
    list.add_css_class("ovenbird-reference-list");
    list.set_show_separators(true);
    list.set_margin_end(30);
    list.set_vexpand(true);
    list.set_hexpand(true);
    let scroll = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .child(&list)
        .build();
    page.append(&scroll);
    LibraryPageWidgets {
        page,
        list,
        list_model,
        search,
        sort_by,
        sort_descending,
        filters: ReferenceFilterWidgets {
            author,
            year,
            tag,
            kind,
        },
    }
}

pub(super) fn build_tags_manager_content() -> TagsPageWidgets {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 10);
    page.set_hexpand(true);
    page.set_vexpand(true);
    page.set_margin_top(20);
    page.set_margin_bottom(12);
    page.set_margin_start(22);
    page.set_margin_end(22);
    let header = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let title = gtk::Label::new(Some(&tr("Manage tags")));
    title.set_xalign(0.0);
    title.add_css_class("title-2");
    title.set_hexpand(true);
    header.append(&title);
    let add = labeled_button("list-add-symbolic", &tr("Add tag"));
    add.add_css_class("suggested-action");
    header.append(&add);
    page.append(&header);

    let search_and_sort = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let search = gtk::SearchEntry::new();
    search.set_placeholder_text(Some(&tr("Search tags…")));
    search.set_hexpand(true);
    search_and_sort.append(&search);
    let sort_by = gtk::DropDown::from_strings(&[&tr("Name")]);
    sort_by.set_selected(0);
    sort_by.set_tooltip_text(Some(&tr("Sort by")));
    search_and_sort.append(&sort_by);
    let sort_descending = sort_direction_button();
    search_and_sort.append(&sort_descending);
    page.append(&search_and_sort);

    let list = gtk::Box::new(gtk::Orientation::Vertical, 3);
    list.set_margin_end(30);
    let scroll = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .child(&list)
        .build();
    page.append(&scroll);
    TagsPageWidgets {
        page,
        list,
        add,
        search,
        sort_by,
        sort_descending,
    }
}

pub(super) fn build_authors_manager_content() -> AuthorsPageWidgets {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 10);
    page.set_hexpand(true);
    page.set_vexpand(true);
    page.set_margin_top(20);
    page.set_margin_bottom(12);
    page.set_margin_start(22);
    page.set_margin_end(22);
    let header = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let title = gtk::Label::new(Some(&tr("Manage authors")));
    title.set_xalign(0.0);
    title.add_css_class("title-2");
    title.set_hexpand(true);
    header.append(&title);
    let add = labeled_button("list-add-symbolic", &tr("Add author"));
    add.add_css_class("suggested-action");
    header.append(&add);
    page.append(&header);

    let search_and_sort = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let search = gtk::SearchEntry::new();
    search.set_placeholder_text(Some(&tr("Search authors…")));
    search.set_hexpand(true);
    search_and_sort.append(&search);
    let sort_by = gtk::DropDown::from_strings(&[&tr("Full name"), &tr("Institution")]);
    sort_by.set_selected(0);
    sort_by.set_tooltip_text(Some(&tr("Sort by")));
    search_and_sort.append(&sort_by);
    let sort_descending = sort_direction_button();
    search_and_sort.append(&sort_descending);
    page.append(&search_and_sort);

    let list = gtk::Box::new(gtk::Orientation::Vertical, 3);
    list.set_margin_end(30);
    let scroll = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .child(&list)
        .build();
    page.append(&scroll);
    AuthorsPageWidgets {
        page,
        list,
        add,
        search,
        sort_by,
        sort_descending,
    }
}

pub(super) fn reference_filter_dropdown(all_label: &str) -> gtk::DropDown {
    let all_label = tr(all_label);
    gtk::DropDown::from_strings(&[&all_label])
}

pub(super) fn build_header(state: &Rc<State>) {
    let header = adw::HeaderBar::new();
    header.add_css_class("ovenbird-main-toolbar");
    header.set_size_request(-1, MAIN_HEADER_HEIGHT);

    let omni_bar = crate::app::libpanel::OmniBar::new();
    let center = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    center.set_valign(gtk::Align::Center);
    state.title.set_label(&tr("LaTeX editor"));
    center.append(&state.title);

    let build_status = build_status_widgets(state.status.clone());
    center.append(&build_status.status_row);

    let reference_summary_group = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    reference_summary_group.set_valign(gtk::Align::Center);
    reference_summary_group.append(&gtk::Separator::new(gtk::Orientation::Vertical));
    let reference_summary = gtk::Label::new(None);
    reference_summary.add_css_class("dim-label");
    reference_summary_group.append(&reference_summary);
    reference_summary_group.set_visible(false);
    center.append(&reference_summary_group);

    omni_bar.add_prefix(0, &center);
    omni_bar.set_action_name("win.compile");
    omni_bar.set_action_tooltip(&tr("Compile and create PDF"));
    omni_bar.set_icon_name("media-playback-start-symbolic");
    header.set_title_widget(Some(omni_bar.widget()));

    state.reference_summary.replace(Some(reference_summary));
    state
        .reference_summary_group
        .replace(Some(reference_summary_group));
    state.build_status_widgets.replace(Some(build_status));
    state.omni_bar.replace(Some(omni_bar));

    let toggle = button("sidebar-show-symbolic", &tr("Show or hide the sidebar"));
    toggle.set_action_name(Some("win.toggle-sidebar"));
    header.pack_start(&toggle);

    let weak_state = Rc::downgrade(state);
    state.layout.connect_position_notify(move |layout| {
        if layout.start_child().is_some() {
            let width = layout.position();
            if width > 0 {
                if let Some(state) = weak_state.upgrade() {
                    state.sidebar_width.set(width);
                }
            }
        }
    });

    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let save = button("document-save-symbolic", &tr("Save document"));
    save.set_action_name(Some("win.save-document"));
    save.add_css_class("flat");
    actions.append(&save);
    state.save_button.replace(Some(save));
    header.pack_end(&actions);

    install_main_header(&state.toolbar_view, &header);
    update_primary_action(state);
    refresh_build_status(state);
}

pub(super) fn install_main_header(toolbar_view: &adw::ToolbarView, header: &adw::HeaderBar) {
    header.set_decoration_layout(Some("minimize,maximize,close"));
    header.set_show_start_title_buttons(false);
    header.set_show_end_title_buttons(true);
    toolbar_view.add_top_bar(header);
}

pub(super) fn build_status_widgets(status: gtk::Label) -> BuildStatusWidgets {
    let status_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    status_row.set_valign(gtk::Align::Center);
    status_row.append(&gtk::Separator::new(gtk::Orientation::Vertical));
    let status_contents = gtk::Box::new(gtk::Orientation::Horizontal, 5);
    status_contents.set_valign(gtk::Align::Center);
    let icon = gtk::Image::from_icon_name("emblem-ok-symbolic");
    icon.set_pixel_size(14);
    status_contents.append(&icon);
    status_contents.append(&status);
    status_row.append(&status_contents);

    let details_popover = gtk::Popover::new();
    details_popover.set_has_arrow(true);
    let content = gtk::Box::new(gtk::Orientation::Vertical, 10);
    content.set_size_request(340, -1);
    content.set_margin_top(14);
    content.set_margin_bottom(14);
    content.set_margin_start(16);
    content.set_margin_end(16);

    let heading = gtk::Label::new(Some(&tr("Current document")));
    heading.set_xalign(0.0);
    heading.add_css_class("title-4");
    content.append(&heading);

    let details_file = gtk::Label::new(None);
    details_file.set_xalign(0.0);
    details_file.set_wrap(true);
    details_file.set_selectable(true);
    content.append(&details_file);

    let grid = gtk::Grid::new();
    grid.set_column_spacing(14);
    grid.set_row_spacing(6);
    let details_engine = gtk::Label::new(None);
    let details_last = gtk::Label::new(None);
    let engine_name = crate::build::find_latex_engine()
        .map(|(engine, _)| {
            tr(match engine {
                crate::build::LatexEngine::LatexMk => "latexmk",
                crate::build::LatexEngine::PdfLatex => "pdflatex",
                crate::build::LatexEngine::Tectonic => "Tectonic",
            })
        })
        .unwrap_or_else(|| tr("Not found"));
    details_engine.set_label(&engine_name);
    for (row, (caption, value)) in [("Compiler", &details_engine), ("Last build", &details_last)]
        .into_iter()
        .enumerate()
    {
        let label = gtk::Label::new(Some(&tr(caption)));
        label.set_xalign(1.0);
        label.add_css_class("dim-label");
        value.set_xalign(0.0);
        value.set_hexpand(true);
        value.set_wrap(true);
        grid.attach(&label, 0, row as i32, 1, 1);
        grid.attach(value, 1, row as i32, 1, 1);
    }
    content.append(&grid);

    let details_failure = gtk::Label::new(None);
    details_failure.set_xalign(0.0);
    details_failure.set_wrap(true);
    details_failure.set_selectable(true);
    details_failure.add_css_class("dim-label");
    details_failure.set_margin_top(6);
    details_failure.set_visible(false);
    content.append(&details_failure);

    details_popover.set_child(Some(&content));

    BuildStatusWidgets {
        status_row,
        icon,
        details_popover,
        details_file,
        details_last,
        details_failure,
    }
}

pub(super) fn update_primary_action(state: &Rc<State>) {
    let page = state.page_stack.visible_child_name();
    let references = page.as_deref() == Some("references");
    let tags = page.as_deref() == Some("tags");
    let authors = page.as_deref() == Some("authors");
    let library_page = references || tags || authors;
    let bibtex_file = !library_page && current_file_kind(state) == Some(ProjectFileKind::Bibtex);
    if let Some(omni_bar) = state.omni_bar.borrow().as_ref() {
        if references {
            omni_bar.set_action_name("win.add-reference");
            omni_bar.set_action_tooltip(&tr("Add a reference to the local library"));
            omni_bar.set_icon_name("list-add-symbolic");
        } else if tags {
            omni_bar.set_action_name("win.add-tag");
            omni_bar.set_action_tooltip(&tr("Add tag"));
            omni_bar.set_icon_name("list-add-symbolic");
        } else if authors {
            omni_bar.set_action_name("win.add-author");
            omni_bar.set_action_tooltip(&tr("Add author"));
            omni_bar.set_icon_name("list-add-symbolic");
        } else if bibtex_file {
            omni_bar.set_action_name("win.insert-bibtex-reference");
            omni_bar.set_action_tooltip(&tr("Add bibliographic reference"));
            omni_bar.set_icon_name("list-add-symbolic");
        } else {
            omni_bar.set_action_name("win.compile");
            omni_bar.set_action_tooltip(&tr("Compile and create PDF"));
            omni_bar.set_icon_name("media-playback-start-symbolic");
        }
        let actions = if references {
            Some(action_menu(&[
                ("Import references", "win.import-references"),
                ("Export references", "win.export-references"),
            ]))
        } else if library_page {
            None
        } else if current_file_kind(state).is_none_or(ProjectFileKind::is_text_editable) {
            Some(action_menu(&[
                ("Save document", "win.save-document"),
                ("Find in document", "win.find-document"),
            ]))
        } else {
            None
        };
        omni_bar.set_menu_model(
            actions
                .as_ref()
                .map(|menu| menu.upcast_ref::<gio::MenuModel>()),
        );
        let details = state.build_status_widgets.borrow();
        let popover = if library_page || bibtex_file {
            None
        } else {
            details.as_ref().map(|widgets| &widgets.details_popover)
        };
        omni_bar.set_popover(popover);
    }
    if let Some(action) = state.compile_action.borrow().as_ref() {
        action.set_enabled(
            !library_page && !bibtex_file && state.build_state.borrow().as_str() != "Building…",
        );
    }
    if let Some(status) = state.build_status_widgets.borrow().as_ref() {
        status.status_row.set_visible(!library_page && !bibtex_file);
    }
    if let Some(summary) = state.reference_summary.borrow().as_ref() {
        let count = state.library.borrow().bibliography.entries.len();
        summary.set_label(
            &ngettext("%d reference", "%d references", count).replace("%d", &count.to_string()),
        );
        summary.set_visible(references);
    }
    if let Some(group) = state.reference_summary_group.borrow().as_ref() {
        group.set_visible(references);
    }
    if let Some(button) = state.save_button.borrow().as_ref() {
        button.set_visible(!library_page);
    }
}

pub(super) fn refresh_build_status(state: &Rc<State>) {
    let status_ref = state.build_status_widgets.borrow();
    let Some(status) = status_ref.as_ref() else {
        return;
    };
    let dirty = state.editor.dirty();
    let build_state = state.build_state.borrow().clone();
    let display_state = if build_state == "Error" {
        tr("PDF generation failed")
    } else if dirty {
        tr("Modified")
    } else {
        tr(&build_state)
    };
    status.icon.set_icon_name(Some(if build_state == "Error" {
        "dialog-error-symbolic"
    } else if dirty {
        "document-edit-symbolic"
    } else if build_state == "Building…" {
        "view-refresh-symbolic"
    } else {
        "checkbox-checked-symbolic"
    }));
    state.status.set_label(&display_state);

    let filename = state
        .current_file
        .borrow()
        .as_ref()
        .and_then(|path| path.file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| tr("New document not saved yet"));
    status.details_file.set_label(&filename);

    status
        .details_last
        .set_label(state.last_build_at.borrow().as_str());
    if let Some(message) = state.build_failure_message.borrow().as_ref() {
        status.details_failure.set_label(message);
        status.details_failure.set_visible(true);
    } else {
        status.details_failure.set_visible(false);
    }
}

pub(super) fn set_build_status(state: &Rc<State>, status: &str) {
    update_build_status(state, status, None);
}

pub(super) fn set_build_failure(state: &Rc<State>, message: &str) {
    update_build_status(state, "Error", Some(message));
}

pub(super) fn update_build_status(state: &Rc<State>, status: &str, failure_message: Option<&str>) {
    state.build_state.replace(status.to_owned());
    state
        .build_failure_message
        .replace(failure_message.map(str::to_owned));
    if status == "Ready" {
        state.last_build_at.replace(tr("Not built yet"));
    } else if status == "Built" || status == "Error" {
        state.last_build_at.replace(
            glib::DateTime::now_local()
                .ok()
                .and_then(|now| now.format("%H:%M:%S").ok())
                .map(|time| time.to_string())
                .unwrap_or_default(),
        );
    }
    if let Some(omni_bar) = state.omni_bar.borrow().as_ref() {
        if status == "Building…" {
            omni_bar.start_pulsing();
        } else {
            omni_bar.stop_pulsing();
        }
    }
    refresh_build_status(state);
    update_primary_action(state);
}
