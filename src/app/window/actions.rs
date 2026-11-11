use super::*;

pub(super) fn find_search_panel(page: &gtk::Box) -> gtk::Box {
    let mut child = page.first_child();
    while let Some(widget) = child {
        if let Some(row) = widget.downcast_ref::<gtk::Box>() {
            if row
                .first_child()
                .is_some_and(|child| child.is::<gtk::SearchEntry>())
            {
                return row.clone();
            }
        }
        child = widget.next_sibling();
    }
    gtk::Box::new(gtk::Orientation::Horizontal, 0)
}

pub(super) fn find_search_entry(page: &gtk::Box) -> gtk::SearchEntry {
    let panel = find_search_panel(page);
    panel
        .first_child()
        .and_downcast::<gtk::SearchEntry>()
        .unwrap_or_else(gtk::SearchEntry::new)
}

pub(super) fn find_search_result(page: &gtk::Box) -> gtk::Label {
    let panel = find_search_panel(page);
    panel
        .first_child()
        .and_then(|entry| entry.next_sibling())
        .and_then(|previous| previous.next_sibling())
        .and_then(|next| next.next_sibling())
        .and_downcast::<gtk::Label>()
        .unwrap_or_else(|| gtk::Label::new(None))
}

pub(super) fn connect_navigation(state: &Rc<State>) {
    let search_entry = state.editor_ui.search_entry.clone();
    search_entry.connect_search_changed({
        let state = state.clone();
        move |entry| {
            let query = entry.text();
            if query.trim().is_empty() {
                state.editor_ui.search_result.set_label("");
            } else {
                let found = state.editor.search_from_start(query.as_str(), true);
                state.editor_ui.search_result.set_label(&tr(if found {
                    "Match found"
                } else {
                    "No results"
                }));
            }
        }
    });
    state.library_ui.library_search.connect_search_changed({
        let state = state.clone();
        move |_| refresh_library(&state)
    });
    state.library_ui.tag_search.connect_search_changed({
        let state = state.clone();
        move |_| refresh_tags(&state)
    });
    state.library_ui.tag_sort.connect_selected_notify({
        let state = state.clone();
        move |_| refresh_tags(&state)
    });
    state.library_ui.tag_sort_descending.connect_toggled({
        let state = state.clone();
        move |_| refresh_tags(&state)
    });
    state.library_ui.author_search.connect_search_changed({
        let state = state.clone();
        move |_| refresh_authors(&state)
    });
    state.library_ui.reference_sort.connect_selected_notify({
        let state = state.clone();
        move |_| refresh_library(&state)
    });
    state.library_ui.reference_sort_descending.connect_toggled({
        let state = state.clone();
        move |_| refresh_library(&state)
    });
    state.library_ui.author_sort.connect_selected_notify({
        let state = state.clone();
        move |_| refresh_authors(&state)
    });
    state.library_ui.author_sort_descending.connect_toggled({
        let state = state.clone();
        move |_| refresh_authors(&state)
    });
    let state_weak = Rc::downgrade(state);
    for filter in [
        state.library_ui.reference_filters.author.clone(),
        state.library_ui.reference_filters.year.clone(),
        state.library_ui.reference_filters.tag.clone(),
        state.library_ui.reference_filters.kind.clone(),
    ] {
        let state_weak = state_weak.clone();
        filter.connect_selected_notify(move |_| {
            if let Some(state) = state_weak.upgrade() {
                if !state.library_ui.library_filters_updating.get() {
                    refresh_library(&state);
                }
            }
        });
    }
}

pub(super) fn install_action(
    state: &Rc<State>,
    name: &str,
    callback: impl Fn(Rc<State>) + 'static,
) {
    let action = gio::SimpleAction::new(name, None);
    let weak = Rc::downgrade(state);
    action.connect_activate(move |_, _| {
        if let Some(state) = weak.upgrade() {
            callback(state);
        }
    });
    state.window.add_action(&action);
}

pub(super) fn connect_actions(state: &Rc<State>) {
    install_action(state, "about", |state| show_about_dialog(&state));
    install_action(state, "show-editor", |s| show_page(&s, "editor"));
    install_action(state, "show-references", |s| show_page(&s, "references"));
    install_action(state, "show-tags", |s| show_page(&s, "tags"));
    install_action(state, "show-authors", |s| show_page(&s, "authors"));
    install_action(state, "show-preferences", |s| show_preferences_dialog(&s));
    install_action(state, "add-tag", |s| edit_tag(&s, None));
    install_action(state, "add-author", |s| edit_author(&s, None));
    install_action(state, "find-document", toggle_search);
    install_action(state, "toggle-sidebar", |s| {
        set_sidebar_visible(
            &s.window_ui.layout,
            &s.window_ui.sidebar_revealer,
            &s.window_ui.sidebar_width,
            s.window_ui.layout.start_child().is_none(),
        );
    });
    install_action(state, "search-next", |s| search_document(&s, true));
    install_action(state, "search-previous", |s| search_document(&s, false));
    install_action(state, "open-document", choose_open_document);
    install_action(state, "open-project", choose_project_folder);
    install_action(state, "close-project", close_project);
    install_action(state, "save-document", save_document);
    let string_parameter = String::static_variant_type();
    let open_path = gio::SimpleAction::new("open-path", Some(string_parameter.as_ref()));
    let weak = Rc::downgrade(state);
    open_path.connect_activate(move |_, parameter| {
        let Some(state) = weak.upgrade() else {
            return;
        };
        state.project.startup_restore_cancelled.set(true);
        let Some(path) = parameter.and_then(|value| value.get::<String>()) else {
            return;
        };
        with_discard_confirmation(&state, move |state| {
            open_document(&state, Path::new(&path), None)
        });
    });
    state.window.add_action(&open_path);
    install_action(state, "new-project", show_template_picker);
    install_action(state, "create-tex", |s| create_project_file(&s, "tex"));
    install_action(state, "create-bib", |s| create_project_file(&s, "bib"));
    install_action(state, "create-sty", |s| create_project_file(&s, "sty"));
    install_action(state, "create-folder", create_project_folder);
    install_action(state, "mode-code", |s| {
        s.editor.set_mode(EditorMode::Code);
        s.editor_ui.code_mode_button.set_active(true);
    });
    install_action(state, "mode-visual", |s| {
        let supports_visual = s.editor.file().is_none_or(|path| {
            path.extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("tex"))
        });
        if supports_visual {
            s.editor.set_mode(EditorMode::Visual);
            s.editor_ui.visual_mode_button.set_active(true);
        } else {
            s.editor_ui.code_mode_button.set_active(true);
            show_toast(
                &s,
                &tr("Visual mode is available only for LaTeX documents."),
            );
        }
    });
    install_action(state, "undo", |s| s.editor.undo());
    install_action(state, "redo", |s| s.editor.redo());
    for (name, format) in [
        ("format-bold", "bold"),
        ("format-italic", "italic"),
        ("format-underline", "underline"),
        ("format-monospace", "monospace"),
    ] {
        install_action(state, name, move |s| {
            if let Err(error) = s.editor.apply_format(format) {
                show_toast(&s, &error);
            }
        });
    }
    for (name, style) in [
        ("heading-normal", "normal"),
        ("heading-section", "section"),
        ("heading-subsection", "subsection"),
        ("heading-subsubsection", "subsubsection"),
        ("heading-paragraph", "paragraph"),
        ("heading-subparagraph", "subparagraph"),
    ] {
        install_action(state, name, move |s| {
            if let Err(error) = s.editor.apply_heading(style) {
                show_toast(&s, &error);
            }
        });
    }
    install_action(state, "math-inline", |s| {
        insert_snippet(
            &s,
            commands::create_math_snippet(false, &s.editor.selected_text()),
        )
    });
    install_action(state, "math-display", |s| {
        insert_snippet(
            &s,
            commands::create_math_snippet(true, &s.editor.selected_text()),
        )
    });
    for (name, command) in [
        ("insert-alpha", "\\alpha"),
        ("insert-beta", "\\beta"),
        ("insert-gamma", "\\gamma"),
        ("insert-pi", "\\pi"),
        ("insert-leq", "\\leq"),
        ("insert-geq", "\\geq"),
        ("insert-neq", "\\neq"),
        ("insert-times", "\\times"),
        ("insert-pm", "\\pm"),
        ("insert-infty", "\\infty"),
        ("insert-sum", "\\sum"),
        ("insert-int", "\\int"),
    ] {
        install_action(state, name, move |s| {
            insert_snippet(&s, commands::create_math_snippet(false, command));
        });
    }
    install_action(state, "insert-sqrt", |s| {
        let selected = s.editor.selected_text();
        let start = "\\(\\sqrt{".chars().count();
        let end = start + selected.chars().count();
        let text = format!("\\(\\sqrt{{{selected}}}\\)");
        let insertion = LatexInsertion {
            text,
            cursor_offset: if selected.is_empty() { start } else { end },
            selection: (!selected.is_empty()).then_some((start, end)),
        };
        s.editor.insert_latex(&insertion, "insert:symbol");
    });
    for (name, kind) in [
        ("list-itemize", "itemize"),
        ("list-enumerate", "enumerate"),
        ("list-quote", "quote"),
    ] {
        install_action(state, name, move |s| {
            let selected = s.editor.selected_text();
            match commands::create_list_snippet(kind, &selected) {
                Ok(snippet) => s.editor.insert_latex(&snippet, &format!("insert:{kind}")),
                Err(error) => show_toast(&s, &error),
            }
        });
    }
    install_action(state, "indent-more", |s| indent_selection(&s, true));
    install_action(state, "indent-less", |s| indent_selection(&s, false));
    install_action(state, "insert-link", show_link_dialog);
    install_action(state, "insert-image", choose_image);
    install_action(state, "insert-table", show_table_dialog);
    install_action(state, "insert-comment", |s| s.editor.insert_comment());
    install_action(state, "insert-footnote", |s| s.editor.insert_footnote());
    install_action(state, "insert-review-note", |s| {
        insert_label_or_note(&s, "review")
    });
    install_action(state, "insert-label", |s| insert_label_or_note(&s, "label"));
    install_action(state, "insert-reference", |s| {
        insert_label_or_note(&s, "ref")
    });
    install_action(state, "insert-citation", show_citation_picker);
    install_action(
        state,
        "insert-bibtex-reference",
        show_bibtex_reference_picker,
    );
    let compile_action = gio::SimpleAction::new("compile", None);
    let weak = Rc::downgrade(state);
    compile_action.connect_activate(move |_, _| {
        if let Some(state) = weak.upgrade() {
            compile(&state, None);
        }
    });
    state.window.add_action(&compile_action);
    state.build_ui.compile_action.replace(Some(compile_action));
    install_action(state, "export-pdf", choose_pdf_export);
    install_action(state, "add-reference", |s| edit_reference(&s, None));
    install_action(state, "import-references", |s| {
        show_reference_transfer_dialog(s, TransferDirection::Import)
    });
    install_action(state, "export-references", |s| {
        show_reference_transfer_dialog(s, TransferDirection::Export)
    });
    install_action(state, "import-bibtex", import_bibtex);
    install_action(state, "export-bibtex", export_bibtex);
    install_action(state, "shortcuts", show_shortcuts);
    install_action(state, "close-window", |s| s.window.close());
    install_action(state, "quit", |s| s.window.close());
}

pub(super) fn connect_window_close_guard(state: &Rc<State>) {
    state.window.connect_close_request({
        let state = Rc::downgrade(state);
        move |_| {
            let Some(state) = state.upgrade() else {
                return glib::Propagation::Proceed;
            };
            if state.project.close_confirmed.replace(false) {
                return glib::Propagation::Proceed;
            }
            if state.project.save_in_progress.get() {
                show_toast(&state, &tr("A document save is still in progress."));
                return glib::Propagation::Stop;
            }
            if !state.editor.dirty() {
                return glib::Propagation::Proceed;
            }
            with_discard_confirmation(&state, move |state| {
                state.project.close_confirmed.set(true);
                state.window.close();
            });
            glib::Propagation::Stop
        }
    });
}

pub(super) fn close_project(state: Rc<State>) {
    with_discard_confirmation(&state, move |state| {
        state
            .project
            .open_request_id
            .set(state.project.open_request_id.get().wrapping_add(1));
        state
            .project
            .project_scan_id
            .set(state.project.project_scan_id.get().wrapping_add(1));
        state
            .project
            .project_open_id
            .set(state.project.project_open_id.get().wrapping_add(1));
        state.project.project_root.borrow_mut().take();
        state.project.project_target.borrow_mut().take();
        state.project.current_file.borrow_mut().take();
        state.project.file_encoding.set(project::TextEncoding::Utf8);
        state.project.main_tex.borrow_mut().take();
        state.project.expanded_folders.borrow_mut().clear();
        state.editor.set_file(None);
        state.editor.load_text(&LatexEditor::new_document_text());
        state.image_preview.set_filename(None::<&Path>);
        state
            .file_pdf_preview
            .set_message(&tr("Select a PDF file to preview it here."));
        state.editor_ui.search_entry.set_text("");
        state.editor_ui.search_panel.set_visible(false);
        state
            .pdf_preview
            .set_message(&tr("The compiled PDF will appear here"));
        update_editor_capabilities(&state);
        set_build_status(&state, "Ready");
        show_page(&state, "editor");
        refresh_project_files(&state);
        update_title(&state, false);
        show_toast(&state, &tr("Project closed"));
    });
}

pub(super) fn show_page(state: &Rc<State>, page: &str) {
    state.editor_ui.page_stack.set_visible_child_name(page);
    let sidebar_page = if page == "editor" {
        "editor"
    } else {
        "references"
    };
    state
        .editor_ui
        .sidebar_pages
        .set_visible_child_name(sidebar_page);
    state
        .editor_ui
        .editor_tab_button
        .set_active(page == "editor");
    state
        .editor_ui
        .references_tab_button
        .set_active(page != "editor");
    state.editor_ui.title.set_label(&tr(match page {
        "references" => "References",
        "tags" => "Tags",
        "authors" => "Authors",
        _ => "LaTeX editor",
    }));
    update_primary_action(state);
    refresh_build_status(state);
    match page {
        "references" => refresh_library(state),
        "tags" => refresh_tags(state),
        "authors" => refresh_authors(state),
        _ => {}
    }
}

pub(super) fn set_sidebar_visible(
    layout: &gtk::Paned,
    sidebar: &gtk::Revealer,
    sidebar_width: &Cell<i32>,
    visible: bool,
) {
    if visible {
        layout.set_start_child(Some(sidebar));
        let width = sidebar_width.get().max(1);
        layout.set_position(width);
        sidebar.set_reveal_child(true);
    } else {
        let width = layout.position();
        if width > 0 {
            sidebar_width.set(width);
        }
        sidebar.set_reveal_child(false);
        layout.set_start_child(None::<&gtk::Widget>);
    }
}

pub(super) fn toggle_search(state: Rc<State>) {
    if search_is_references_page(&state.editor_ui.page_stack) {
        match state.editor_ui.page_stack.visible_child_name().as_deref() {
            Some("references") => {
                state.library_ui.library_search.grab_focus();
            }
            Some("authors") => {
                state.library_ui.author_search.grab_focus();
            }
            _ => {}
        }
        return;
    }
    let kind = current_file_kind(&state);
    if kind.is_some_and(|kind| !kind.is_text_editable()) {
        return;
    }
    if state.editor_ui.page_stack.visible_child_name().as_deref() != Some("editor") {
        show_page(&state, "editor");
    }
    let open = !state.editor_ui.search_panel.is_visible();
    state.editor_ui.search_panel.set_visible(open);
    if open {
        state.editor.focus();
        state.editor_ui.search_entry.grab_focus();
        state.editor_ui.search_entry.select_region(0, -1);
    } else {
        state.editor.focus();
    }
}

pub(super) fn search_is_references_page(page_stack: &gtk::Stack) -> bool {
    matches!(
        page_stack.visible_child_name().as_deref(),
        Some("references" | "tags" | "authors")
    )
}

pub(super) fn search_document(state: &Rc<State>, forward: bool) {
    let query = state.editor_ui.search_entry.text();
    let found = state.editor.search(query.as_str(), forward);
    let message = if query.trim().is_empty() {
        String::new()
    } else if found {
        tr("Match found")
    } else {
        tr("No results")
    };
    state.editor_ui.search_result.set_label(&message);
}

pub(super) fn show_toast(state: &Rc<State>, message: &str) {
    state.window_ui.toast.add_toast(adw::Toast::new(message));
}

pub(super) fn update_title(state: &Rc<State>, dirty: bool) {
    let path = state.project.current_file.borrow().clone();
    let name = path
        .as_ref()
        .and_then(|file| file.file_name())
        .and_then(|name| name.to_str())
        .unwrap_or("Ovenbird");
    state.window.set_title(Some(&format!(
        "{}{} — Ovenbird",
        name,
        if dirty { " •" } else { "" }
    )));
    refresh_build_status(state);
}

pub(super) fn show_shortcuts(state: Rc<State>) {
    let dialog = gtk::Dialog::builder()
        .title(tr("Keyboard shortcuts"))
        .transient_for(&state.window)
        .modal(true)
        .build();
    dialog.add_button(&tr("Close"), gtk::ResponseType::Close);
    let content = dialog.content_area();
    content.set_spacing(8);
    content.set_margin_top(18);
    content.set_margin_bottom(18);
    content.set_margin_start(20);
    content.set_margin_end(20);
    for (action, shortcut) in [
        ("New project", "Ctrl+Alt+N"),
        ("Open document", "Ctrl+O"),
        ("Open project", "Ctrl+Alt+O"),
        ("Close project", "Ctrl+W"),
        ("Save document", "Ctrl+S"),
        ("Find in document", "Ctrl+F"),
        ("Compile", "Ctrl+Shift+R"),
        ("Insert citation", "Ctrl+Shift+C"),
        ("Toggle sidebar", "F9"),
        ("Undo", "Ctrl+Z"),
        ("Redo", "Ctrl+Shift+Z"),
    ] {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        row.append(&gtk::Label::new(Some(&tr_dynamic(action))));
        let key = gtk::Label::new(Some(shortcut));
        key.set_hexpand(true);
        key.set_halign(gtk::Align::End);
        key.add_css_class("dim-label");
        row.append(&key);
        content.append(&row);
    }
    dialog.connect_response(|dialog, _| dialog.close());
    dialog.present();
}
