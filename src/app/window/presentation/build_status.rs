use super::*;

pub(in crate::app::window) fn build_status_widgets(status: gtk::Label) -> BuildStatusWidgets {
    let status_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    status_row.set_valign(gtk::Align::Center);
    status_row.append(&gtk::Separator::new(gtk::Orientation::Vertical));
    let status_contents = gtk::Box::new(gtk::Orientation::Horizontal, 5);
    status_contents.set_valign(gtk::Align::Center);
    let icon = gtk::Image::from_icon_name("checkbox-checked-symbolic");
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

pub(in crate::app::window) fn update_primary_action(state: &Rc<State>) {
    let page = state.editor_ui.page_stack.visible_child_name();
    let references = page.as_deref() == Some("references");
    let tags = page.as_deref() == Some("tags");
    let authors = page.as_deref() == Some("authors");
    let library_page = references || tags || authors;
    let bibtex_file = !library_page && current_file_kind(state) == Some(ProjectFileKind::Bibtex);
    if let Some(omni_bar) = state.build_ui.omni_bar.borrow().as_ref() {
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
        let details = state.build_ui.build_status_widgets.borrow();
        let popover = if library_page || bibtex_file {
            None
        } else {
            details.as_ref().map(|widgets| &widgets.details_popover)
        };
        omni_bar.set_popover(popover);
    }
    if let Some(action) = state.build_ui.compile_action.borrow().as_ref() {
        action.set_enabled(
            !library_page
                && !bibtex_file
                && state.build_ui.build_state.borrow().as_str() != "Building…",
        );
    }
    if let Some(status) = state.build_ui.build_status_widgets.borrow().as_ref() {
        status.status_row.set_visible(!library_page && !bibtex_file);
    }
    if let Some(summary) = state.build_ui.reference_summary.borrow().as_ref() {
        let count = state.library.borrow().bibliography.entries.len();
        summary.set_label(
            &ngettext("%d reference", "%d references", count).replace("%d", &count.to_string()),
        );
        summary.set_visible(references);
    }
    if let Some(group) = state.build_ui.reference_summary_group.borrow().as_ref() {
        group.set_visible(references);
    }
    if let Some(button) = state.build_ui.save_button.borrow().as_ref() {
        button.set_visible(!library_page);
    }
}

pub(in crate::app::window) fn refresh_build_status(state: &Rc<State>) {
    let status_ref = state.build_ui.build_status_widgets.borrow();
    let Some(status) = status_ref.as_ref() else {
        return;
    };
    let dirty = state.editor.dirty();
    let build_state = state.build_ui.build_state.borrow().clone();
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
    state.editor_ui.status.set_label(&display_state);

    let filename = state
        .project
        .current_file
        .borrow()
        .as_ref()
        .and_then(|path| path.file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| tr("New document not saved yet"));
    status.details_file.set_label(&filename);

    status
        .details_last
        .set_label(state.build_ui.last_build_at.borrow().as_str());
    if let Some(message) = state.build_ui.build_failure_message.borrow().as_ref() {
        status.details_failure.set_label(message);
        status.details_failure.set_visible(true);
    } else {
        status.details_failure.set_visible(false);
    }
}

pub(in crate::app::window) fn set_build_status(state: &Rc<State>, status: &str) {
    update_build_status(state, status, None);
}

pub(in crate::app::window) fn set_build_failure(state: &Rc<State>, message: &str) {
    update_build_status(state, "Error", Some(message));
}

pub(in crate::app::window) fn update_build_status(
    state: &Rc<State>,
    status: &str,
    failure_message: Option<&str>,
) {
    state.build_ui.build_state.replace(status.to_owned());
    state
        .build_ui
        .build_failure_message
        .replace(failure_message.map(str::to_owned));
    if status == "Ready" {
        state.build_ui.last_build_at.replace(tr("Not built yet"));
    } else if status == "Built" || status == "Error" {
        state.build_ui.last_build_at.replace(
            glib::DateTime::now_local()
                .ok()
                .and_then(|now| now.format("%H:%M:%S").ok())
                .map(|time| time.to_string())
                .unwrap_or_default(),
        );
    }
    if let Some(omni_bar) = state.build_ui.omni_bar.borrow().as_ref() {
        if status == "Building…" {
            omni_bar.start_pulsing();
        } else {
            omni_bar.stop_pulsing();
        }
    }
    refresh_build_status(state);
    update_primary_action(state);
}
