use super::*;

pub(super) fn latex_syntax_check_supported(file: Option<&Path>) -> bool {
    file.is_none_or(|path| {
        path.extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| {
                matches!(
                    extension.to_ascii_lowercase().as_str(),
                    "tex" | "cls" | "sty"
                )
            })
    })
}

pub(super) fn clear_syntax_diagnostics(
    buffer: &sourceview5::Buffer,
    list: &gtk::Box,
    error_tag: &TextTag,
) {
    let start = buffer.start_iter();
    let end = buffer.end_iter();
    buffer.remove_tag(error_tag, &start, &end);
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
}

pub(super) fn diagnostic_text(diagnostic: &LatexDiagnostic) -> String {
    let message = match &diagnostic.kind {
        LatexDiagnosticKind::UnmatchedClosingBrace => {
            crate::i18n::gettext("Unmatched closing brace")
        }
        LatexDiagnosticKind::UnclosedOpeningBrace => {
            crate::i18n::gettext("Opening brace is not closed")
        }
        LatexDiagnosticKind::UnmatchedEnvironmentEnd(name) => format!(
            "{}{}",
            crate::i18n::gettext("Environment end has no matching begin: "),
            name
        ),
        LatexDiagnosticKind::UnclosedEnvironment(name) => format!(
            "{}{}",
            crate::i18n::gettext("Environment is not closed: "),
            name
        ),
        LatexDiagnosticKind::UnmatchedMathDelimiter(delimiter) => format!(
            "{}{}",
            crate::i18n::gettext("Math delimiter has no matching opener: "),
            delimiter
        ),
        LatexDiagnosticKind::UnclosedMathDelimiter(delimiter) => format!(
            "{}{}",
            crate::i18n::gettext("Math delimiter is not closed: "),
            delimiter
        ),
        LatexDiagnosticKind::NestedMathDelimiter(delimiter) => format!(
            "{}{}",
            crate::i18n::gettext("Math delimiter opens inside another math expression: "),
            delimiter
        ),
    };
    format!(
        "{} {}: {}",
        crate::i18n::gettext("Line"),
        diagnostic.line,
        message
    )
}

pub(super) fn render_syntax_diagnostics(
    buffer: &sourceview5::Buffer,
    view: &sourceview5::View,
    list: &gtk::Box,
    (revealer, heading): (&gtk::Revealer, &gtk::Label),
    error_tag: &TextTag,
    diagnostics: &[LatexDiagnostic],
) {
    clear_syntax_diagnostics(buffer, list, error_tag);
    revealer.set_reveal_child(true);
    if diagnostics.is_empty() {
        heading.set_label(&crate::i18n::gettext(
            "The document appears ready to compile.",
        ));
        heading.remove_css_class("error");
        heading.add_css_class("success");
        return;
    }
    heading.set_label(&crate::i18n::gettext("Syntax problems"));
    heading.remove_css_class("success");
    heading.add_css_class("error");

    for diagnostic in diagnostics.iter().take(MAX_VISIBLE_DIAGNOSTICS) {
        if let Some(start) = buffer.iter_at_line(diagnostic.line.saturating_sub(1) as i32) {
            let mut end = start;
            end.forward_to_line_end();
            if end.offset() > start.offset() {
                buffer.apply_tag(error_tag, &start, &end);
            }
        }

        let button = gtk::Button::with_label(&diagnostic_text(diagnostic));
        button.set_halign(gtk::Align::Fill);
        button.add_css_class("flat");
        let buffer_for_click = buffer.clone();
        let view_for_click = view.clone();
        let line = diagnostic.line.saturating_sub(1) as i32;
        let column = diagnostic.column.saturating_sub(1).min(i32::MAX as usize) as i32;
        button.connect_clicked(move |_| {
            let _ = view_for_click.activate_action("win.mode-code", None);
            if let Some(mut iter) = buffer_for_click.iter_at_line(line) {
                iter.forward_chars(column);
                buffer_for_click.place_cursor(&iter);
                view_for_click.scroll_to_iter(&mut iter, 0.15, true, 0.0, 0.3);
                view_for_click.grab_focus();
            }
        });
        list.append(&button);
    }

    if diagnostics.len() > MAX_VISIBLE_DIAGNOSTICS {
        let more = gtk::Label::new(Some(&crate::i18n::gettext(
            "Additional syntax problems are not shown.",
        )));
        more.set_halign(gtk::Align::Start);
        more.add_css_class("dim-label");
        list.append(&more);
    }
}

pub(super) fn schedule_syntax_check(
    buffer: &sourceview5::Buffer,
    view: &sourceview5::View,
    list: &gtk::Box,
    (revealer, heading): (&gtk::Revealer, &gtk::Label),
    error_tag: &TextTag,
    timer: &SyntaxTimer,
    runtime: &Rc<RefCell<Runtime>>,
) {
    if let Some(previous) = timer.borrow_mut().take() {
        previous.remove();
    }

    let enabled = latex_syntax_check_supported(runtime.borrow().file.as_deref());
    if !enabled {
        clear_syntax_diagnostics(buffer, list, error_tag);
        revealer.set_reveal_child(false);
        return;
    }
    heading.set_label(&crate::i18n::gettext("Checking document…"));
    heading.remove_css_class("error");
    heading.remove_css_class("success");
    revealer.set_reveal_child(true);

    let buffer = buffer.downgrade();
    let view = view.downgrade();
    let list = list.downgrade();
    let revealer = revealer.downgrade();
    let heading = heading.downgrade();
    let error_tag = error_tag.downgrade();
    let runtime = runtime.clone();
    let timer_for_callback = timer.clone();
    let source_id = gtk::glib::timeout_add_local_once(SYNTAX_CHECK_DELAY, move || {
        timer_for_callback.borrow_mut().take();
        let (Some(buffer), Some(view), Some(list), Some(revealer), Some(heading), Some(error_tag)) = (
            buffer.upgrade(),
            view.upgrade(),
            list.upgrade(),
            revealer.upgrade(),
            heading.upgrade(),
            error_tag.upgrade(),
        ) else {
            return;
        };
        if !latex_syntax_check_supported(runtime.borrow().file.as_deref()) {
            clear_syntax_diagnostics(&buffer, &list, &error_tag);
            revealer.set_reveal_child(false);
            return;
        }
        let diagnostics = latex_diagnostics::check_structure(&buffer_text(&buffer));
        render_syntax_diagnostics(
            &buffer,
            &view,
            &list,
            (&revealer, &heading),
            &error_tag,
            &diagnostics,
        );
    });
    *timer.borrow_mut() = Some(source_id);
}
