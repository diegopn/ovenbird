use super::*;

pub(super) fn configure_editor_callbacks(state: &Rc<State>) {
    state.editor.set_dirty_changed({
        let state = state.clone();
        move |dirty| update_title(&state, dirty)
    });
    state.editor.set_history_changed({
        let state = state.clone();
        move |undo, redo| {
            state.editor_ui.undo_button.set_sensitive(undo);
            state.editor_ui.redo_button.set_sensitive(redo);
            refresh_build_status(&state);
        }
    });
}

pub(super) fn update_editor_capabilities(state: &Rc<State>) {
    let kind = current_file_kind(state);
    let supports_latex = kind.is_none_or(|kind| kind == ProjectFileKind::Latex);
    let text_editable = kind.is_none_or(ProjectFileKind::is_text_editable);
    let supports_search = text_editable;

    state.editor_ui.mode_toolbar.set_visible(text_editable);
    state.editor_ui.code_mode_button.set_visible(supports_latex);
    state
        .editor_ui
        .visual_mode_button
        .set_visible(supports_latex);
    state
        .editor_ui
        .find_document_button
        .set_visible(supports_search);
    state
        .bibtex_citation_button
        .set_visible(kind == Some(ProjectFileKind::Bibtex));
    state.editor_ui.latex_toolbar.set_visible(supports_latex);
    state.editor_ui.editor_toolbar.set_visible(text_editable);
    if !supports_search {
        state.editor_ui.search_panel.set_visible(false);
    }
    if let Some(button) = state.build_ui.save_button.borrow().as_ref() {
        button.set_visible(text_editable);
    }

    state.file_preview_stack.set_visible_child_name(match kind {
        Some(ProjectFileKind::Image) => "image",
        Some(ProjectFileKind::Pdf) => "pdf",
        _ => "text",
    });

    if !supports_latex {
        state.editor.set_mode(EditorMode::Code);
        state.editor_ui.code_mode_button.set_active(true);
    }
}

pub(super) fn supports_latex_document(state: &Rc<State>) -> bool {
    current_file_kind(state).is_none_or(|kind| kind == ProjectFileKind::Latex)
}

pub(super) fn current_file_kind(state: &Rc<State>) -> Option<ProjectFileKind> {
    state
        .current_file
        .borrow()
        .as_deref()
        .map(project::file_kind)
}

pub(super) fn indent_selection(state: &Rc<State>, increase: bool) {
    let selected = state.editor.selected_text();
    if selected.is_empty() {
        return;
    }
    let indent = if increase { "    " } else { "" };
    let text = selected
        .lines()
        .map(|line| {
            if increase {
                format!("{indent}{line}")
            } else {
                line.strip_prefix("    ")
                    .or_else(|| line.strip_prefix('\t'))
                    .unwrap_or(line)
                    .to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    let insertion = LatexInsertion {
        text: text.clone(),
        cursor_offset: text.chars().count(),
        selection: Some((0, text.chars().count())),
    };
    state.editor.insert_latex(
        &insertion,
        if increase {
            "indent:more"
        } else {
            "indent:less"
        },
    );
}

pub(super) fn insert_snippet(state: &Rc<State>, insertion: LatexInsertion) {
    state.editor.insert_latex(&insertion, "insert:math");
}

pub(super) fn insert_label_or_note(state: &Rc<State>, kind: &str) {
    let (title, prompt, command) = match kind {
        "label" => ("Add label", "Label identifier", "label"),
        "ref" => ("Cross-reference", "Label identifier", "ref"),
        _ => ("Review note", "Note", "todo"),
    };
    show_text_prompt(state, title, prompt, "", move |state, value| {
        if value.trim().is_empty() {
            return;
        }
        let snippet = if command == "todo" {
            format!("\\todo{{{value}}}")
        } else {
            format!("\\{command}{{{value}}}")
        };
        let cursor = snippet.chars().count();
        state
            .editor
            .insert_at_cursor(&snippet, cursor, &format!("insert:{command}"));
    });
}
