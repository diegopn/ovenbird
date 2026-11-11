use super::*;

pub(in crate::app::window) fn build_editor_page(editor: &LatexEditor) -> EditorPageWidgets {
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

pub(in crate::app::window) fn action_button(
    icon: &str,
    tooltip: &str,
    action: &str,
) -> gtk::Button {
    let b = button(icon, &tr_dynamic(tooltip));
    b.set_action_name(Some(action));
    b
}

pub(in crate::app::window) fn menu_with_actions(items: &[(&str, &str, &str)]) -> gtk::Popover {
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

pub(in crate::app::window) fn action_menu(items: &[(&str, &str)]) -> gio::Menu {
    let model = gio::Menu::new();
    for (label, action) in items {
        model.append(Some(&tr_dynamic(label)), Some(action));
    }
    model
}

pub(in crate::app::window) fn heading_menu() -> gtk::Popover {
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

pub(in crate::app::window) fn math_menu() -> gtk::Popover {
    menu_with_actions(&[
        ("", "Inline math", "win.math-inline"),
        ("", "Display math", "win.math-display"),
    ])
}

pub(in crate::app::window) fn symbol_menu() -> gtk::Popover {
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

pub(in crate::app::window) fn list_menu() -> gtk::Popover {
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

pub(in crate::app::window) fn more_menu() -> gtk::Popover {
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
