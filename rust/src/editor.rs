use crate::commands::{self, LatexInsertion};
use crate::history::{EditorHistory, EditorMode, EditorState};
use crate::latex::{self, Token};
use crate::latex_diagnostics::{self, LatexDiagnostic, LatexDiagnosticKind};
use crate::search::find_document_match;
use gtk::glib::translate::IntoGlib;
use gtk::prelude::*;
use gtk::{pango, TextBuffer, TextTag, TextView};
use sourceview5::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant};

const INDENT_PREFIX: &str = r"\hspace{1em}";
const SYNTAX_CHECK_DELAY: Duration = Duration::from_millis(400);
const MAX_VISIBLE_DIAGNOSTICS: usize = 8;
type SyntaxTimer = Rc<RefCell<Option<gtk::glib::SourceId>>>;

struct Runtime {
    file: Option<PathBuf>,
    saved_text: String,
    dirty: bool,
    synchronizing: bool,
    visual_edited: bool,
    source_cursor_before_visual: Option<i32>,
    mode: EditorMode,
    history: EditorHistory,
}

type DirtyCallback = Rc<RefCell<Option<Box<dyn Fn(bool)>>>>;
type HistoryCallback = Rc<RefCell<Option<Box<dyn Fn(bool, bool)>>>>;

pub struct LatexEditor {
    pub root: gtk::Box,
    pub source_buffer: sourceview5::Buffer,
    pub source_view: sourceview5::View,
    pub visual_buffer: TextBuffer,
    pub visual_view: TextView,
    pub stack: gtk::Stack,
    runtime: Rc<RefCell<Runtime>>,
    tags: Rc<HashMap<String, TextTag>>,
    dirty_callback: DirtyCallback,
    history_callback: HistoryCallback,
    diagnostics_revealer: gtk::Revealer,
    diagnostics_heading: gtk::Label,
    diagnostics_list: gtk::Box,
    syntax_error_tag: TextTag,
    diagnostics_timer: SyntaxTimer,
}

fn character_count(text: &str) -> usize {
    text.chars().count()
}

fn buffer_text(buffer: &impl IsA<TextBuffer>) -> String {
    let buffer = buffer.as_ref();
    buffer
        .text(&buffer.start_iter(), &buffer.end_iter(), true)
        .to_string()
}

fn source_style_scheme_id(is_dark: bool) -> &'static str {
    if is_dark {
        "Adwaita-dark"
    } else {
        "Adwaita"
    }
}

fn apply_source_style_scheme(buffer: &sourceview5::Buffer, is_dark: bool) {
    let manager = sourceview5::StyleSchemeManager::default();
    buffer.set_style_scheme(manager.scheme(source_style_scheme_id(is_dark)).as_ref());
}

fn visual_mode_supported(file: Option<&Path>) -> bool {
    file.is_none_or(|path| {
        path.extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("tex"))
    })
}

fn latex_syntax_check_supported(file: Option<&Path>) -> bool {
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

fn clear_syntax_diagnostics(buffer: &sourceview5::Buffer, list: &gtk::Box, error_tag: &TextTag) {
    let start = buffer.start_iter();
    let end = buffer.end_iter();
    buffer.remove_tag(error_tag, &start, &end);
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
}

fn diagnostic_text(diagnostic: &LatexDiagnostic) -> String {
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

fn render_syntax_diagnostics(
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

fn schedule_syntax_check(
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

fn indent_line_starts(
    text: &str,
    selection: Option<(usize, usize)>,
    cursor_offset: usize,
) -> Vec<usize> {
    let mut starts = vec![0];
    for (offset, character) in text.chars().enumerate() {
        if character == '\n' {
            starts.push(offset + 1);
        }
    }

    let text_length = text.chars().count();
    let first_offset = selection
        .map_or(cursor_offset, |(start, _)| start)
        .min(text_length);
    let mut last_offset = selection
        .map_or(cursor_offset, |(_, end)| end)
        .min(text_length);
    let line_for_offset = |offset: usize| {
        starts
            .partition_point(|start| *start <= offset)
            .saturating_sub(1)
    };
    let first_line = line_for_offset(first_offset);
    let mut last_line = line_for_offset(last_offset);
    if selection.is_some() && last_line > first_line && starts[last_line] == last_offset {
        last_offset = last_offset.saturating_sub(1);
        last_line = line_for_offset(last_offset);
    }
    starts[first_line..=last_line].to_vec()
}

fn indent_line_text(line: &str, increase: bool) -> Option<String> {
    if increase {
        (!line.trim().is_empty()).then(|| format!("{INDENT_PREFIX}{line}"))
    } else {
        line.strip_prefix(INDENT_PREFIX).map(str::to_owned)
    }
}

fn add_tag(table: &gtk::TextTagTable, name: &str) -> TextTag {
    let tag = TextTag::new(Some(name));
    table.add(&tag);
    tag
}

fn create_tags(buffer: &TextBuffer) -> HashMap<String, TextTag> {
    let table = buffer.tag_table();
    let mut tags = HashMap::new();
    let bold = add_tag(&table, "bold");
    bold.set_weight(pango::Weight::Bold.into_glib());
    tags.insert("bold".to_owned(), bold);
    let italic = add_tag(&table, "italic");
    italic.set_style(pango::Style::Italic);
    tags.insert("italic".to_owned(), italic);
    let underline = add_tag(&table, "underline");
    underline.set_underline(pango::Underline::Single);
    tags.insert("underline".to_owned(), underline);
    let monospace = add_tag(&table, "monospace");
    monospace.set_family(Some("monospace"));
    tags.insert("monospace".to_owned(), monospace);
    let raw = add_tag(&table, "raw");
    raw.set_family(Some("monospace"));
    raw.set_style(pango::Style::Italic);
    tags.insert("raw".to_owned(), raw);
    for (level, scale, above, below) in [
        (1, 1.55, 14, 6),
        (2, 1.30, 11, 5),
        (3, 1.15, 9, 4),
        (4, 1.08, 7, 3),
        (5, 1.03, 6, 2),
    ] {
        let name = format!("heading{level}");
        let tag = add_tag(&table, &name);
        tag.set_weight(pango::Weight::Bold.into_glib());
        tag.set_scale(scale);
        tag.set_pixels_above_lines(above);
        tag.set_pixels_below_lines(below);
        tags.insert(name, tag);
    }
    tags
}

fn token_text(token: &Token) -> &str {
    match token {
        Token::Text { text, .. } | Token::Raw(text) | Token::Paragraph(text) => text,
    }
}

fn tokens_text(tokens: &[Token]) -> String {
    tokens.iter().map(token_text).collect()
}

fn render_visual(
    text: &str,
    buffer: &TextBuffer,
    view: &TextView,
    tags: &HashMap<String, TextTag>,
) {
    let document = latex::split_document(text);
    if !document.valid {
        buffer.set_text(&crate::i18n::gettext(
            "Visual mode needs a document with \\begin{document} and \\end{document}.\n\nSwitch to Code mode to review the document structure.",
        ));
        view.set_editable(false);
        view.set_cursor_visible(false);
        return;
    }

    view.set_editable(true);
    view.set_cursor_visible(true);
    let tokens = latex::parse_visual_body(&document.body);
    buffer.set_text(&tokens_text(&tokens));
    let start = buffer.start_iter();
    let end = buffer.end_iter();
    for tag in tags.values() {
        buffer.remove_tag(tag, &start, &end);
    }
    let mut offset = 0usize;
    for token in tokens {
        let length = character_count(token_text(&token));
        if length == 0 {
            continue;
        }
        let start = buffer.iter_at_offset(offset as i32);
        let end = buffer.iter_at_offset((offset + length) as i32);
        match token {
            Token::Raw(_) => {
                if let Some(tag) = tags.get("raw") {
                    buffer.apply_tag(tag, &start, &end);
                }
            }
            Token::Text { marks, .. } => {
                for mark in marks {
                    if let Some(tag) = tags.get(&mark) {
                        buffer.apply_tag(tag, &start, &end);
                    }
                }
            }
            Token::Paragraph(_) => {}
        }
        offset += length;
    }
}

fn snapshot(source: &sourceview5::Buffer, visual: &TextBuffer, mode: EditorMode) -> EditorState {
    EditorState {
        source: buffer_text(source),
        source_cursor: source.iter_at_mark(&source.get_insert()).offset() as usize,
        visual_cursor: visual.iter_at_mark(&visual.get_insert()).offset() as usize,
        mode,
    }
}

fn record_history(
    runtime: &Rc<RefCell<Runtime>>,
    source: &sourceview5::Buffer,
    visual: &TextBuffer,
    group: &str,
) {
    let state = runtime.borrow();
    let snapshot = snapshot(source, visual, state.mode);
    drop(state);
    runtime
        .borrow_mut()
        .history
        .record(snapshot, Some(group), Instant::now());
}

fn notify_history(runtime: &Rc<RefCell<Runtime>>, callback: &HistoryCallback) {
    let state = runtime.borrow();
    let can_undo = state.history.can_undo();
    let can_redo = state.history.can_redo();
    drop(state);
    if let Some(callback) = callback.borrow().as_ref() {
        callback(can_undo, can_redo);
    }
}

fn update_dirty(runtime: &Rc<RefCell<Runtime>>, text: &str, callback: &DirtyCallback) {
    let changed = {
        let mut state = runtime.borrow_mut();
        let dirty = text != state.saved_text;
        if dirty == state.dirty {
            false
        } else {
            state.dirty = dirty;
            true
        }
    };
    if changed {
        if let Some(callback) = callback.borrow().as_ref() {
            callback(runtime.borrow().dirty);
        }
    }
}

fn tokens_from_visual(buffer: &TextBuffer, tags: &HashMap<String, TextTag>) -> Vec<Token> {
    let mut result = Vec::<Token>::new();
    let raw_tag = tags.get("raw");
    let mut iter = buffer.start_iter();
    let end = buffer.end_iter();
    while iter.offset() < end.offset() {
        let active = iter.tags();
        let is_raw = raw_tag.is_some_and(|tag| active.contains(tag));
        let mut marks = Vec::new();
        if !is_raw {
            for name in [
                "bold",
                "italic",
                "underline",
                "monospace",
                "heading1",
                "heading2",
                "heading3",
                "heading4",
                "heading5",
            ] {
                if tags.get(name).is_some_and(|tag| active.contains(tag)) {
                    marks.push(name.to_owned());
                }
            }
        }
        let character = iter.char();
        match (is_raw, result.last_mut()) {
            (true, Some(Token::Raw(text))) => text.push(character),
            (true, _) => result.push(Token::Raw(character.to_string())),
            (
                false,
                Some(Token::Text {
                    text,
                    marks: existing,
                }),
            ) if *existing == marks => {
                text.push(character);
            }
            (false, _) => result.push(Token::Text {
                text: character.to_string(),
                marks,
            }),
        }
        iter.forward_char();
    }
    result
}

fn tokens_before_offset(tokens: &[Token], offset: usize) -> Vec<Token> {
    let mut prefix = Vec::new();
    let mut remaining = offset;
    for token in tokens {
        let text = token_text(token);
        let length = character_count(text);
        if remaining >= length {
            prefix.push(token.clone());
            remaining -= length;
            if remaining == 0 {
                break;
            }
        } else if remaining > 0 {
            let partial = text.chars().take(remaining).collect::<String>();
            prefix.push(match token {
                Token::Text { marks, .. } => Token::Text {
                    text: partial,
                    marks: marks.clone(),
                },
                Token::Raw(_) => Token::Raw(partial),
                Token::Paragraph(_) => Token::Paragraph(partial),
            });
            break;
        } else {
            break;
        }
    }
    prefix
}

impl LatexEditor {
    pub fn new() -> Self {
        let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
        root.set_hexpand(true);
        root.set_vexpand(true);

        let source_buffer = sourceview5::Buffer::new(None);
        source_buffer.set_enable_undo(false);
        source_buffer.set_highlight_syntax(true);
        let syntax_error_tag = TextTag::new(Some("latex-structural-error"));
        syntax_error_tag.set_underline(pango::Underline::Error);
        let error_color = gtk::gdk::RGBA::parse("#ff6b6b").expect("valid syntax error color");
        syntax_error_tag.set_underline_rgba(Some(&error_color));
        source_buffer.tag_table().add(&syntax_error_tag);
        let language_manager = sourceview5::LanguageManager::default();
        if let Some(language) = language_manager.language("latex") {
            source_buffer.set_language(Some(&language));
        }
        let style_manager = adw::StyleManager::default();
        apply_source_style_scheme(&source_buffer, style_manager.is_dark());
        let source_buffer_weak = source_buffer.downgrade();
        style_manager.connect_dark_notify(move |style_manager| {
            if let Some(source_buffer) = source_buffer_weak.upgrade() {
                apply_source_style_scheme(&source_buffer, style_manager.is_dark());
            }
        });
        let source_view = sourceview5::View::with_buffer(&source_buffer);
        source_view.set_monospace(true);
        source_view.set_show_line_numbers(true);
        source_view.set_show_line_marks(false);
        source_view.set_auto_indent(true);
        source_view.set_indent_width(4);
        source_view.set_tab_width(4);
        source_view.set_highlight_current_line(true);
        source_view.set_wrap_mode(gtk::WrapMode::WordChar);
        source_view.set_right_margin(25);
        source_view.add_css_class("source-view");

        let visual_buffer = TextBuffer::new(None);
        let visual_view = TextView::with_buffer(&visual_buffer);
        visual_view.set_wrap_mode(gtk::WrapMode::WordChar);
        visual_view.set_left_margin(32);
        visual_view.set_right_margin(32);
        visual_view.set_top_margin(28);
        visual_view.set_bottom_margin(28);
        visual_view.set_accepts_tab(false);
        visual_view.add_css_class("visual-document");
        let tags = Rc::new(create_tags(&visual_buffer));

        let source_scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .child(&source_view)
            .hexpand(true)
            .vexpand(true)
            .build();
        let visual_scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .child(&visual_view)
            .hexpand(true)
            .vexpand(true)
            .build();
        let stack = gtk::Stack::builder()
            .transition_type(gtk::StackTransitionType::Crossfade)
            .hexpand(true)
            .vexpand(true)
            .build();
        stack.add_named(&source_scroll, Some("code"));
        stack.add_named(&visual_scroll, Some("visual"));
        root.append(&stack);

        let diagnostics_list = gtk::Box::new(gtk::Orientation::Vertical, 2);
        let diagnostics_scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .max_content_height(132)
            .propagate_natural_height(true)
            .child(&diagnostics_list)
            .build();
        let diagnostics_panel = gtk::Box::new(gtk::Orientation::Vertical, 4);
        diagnostics_panel.add_css_class("latex-diagnostics");
        let diagnostics_heading =
            gtk::Label::new(Some(&crate::i18n::gettext("Checking document…")));
        diagnostics_heading.set_halign(gtk::Align::Start);
        diagnostics_heading.set_wrap(true);
        diagnostics_heading.set_xalign(0.0);
        diagnostics_heading.add_css_class("heading");
        diagnostics_panel.append(&diagnostics_heading);
        diagnostics_panel.append(&diagnostics_scroll);
        let diagnostics_revealer = gtk::Revealer::builder()
            .transition_type(gtk::RevealerTransitionType::SlideUp)
            .transition_duration(120)
            .reveal_child(true)
            .child(&diagnostics_panel)
            .build();
        let diagnostics_timer: SyntaxTimer = Rc::new(RefCell::new(None));
        root.append(&diagnostics_revealer);

        let empty = EditorState {
            source: String::new(),
            source_cursor: 0,
            visual_cursor: 0,
            mode: EditorMode::Code,
        };
        let runtime = Rc::new(RefCell::new(Runtime {
            file: None,
            saved_text: String::new(),
            dirty: false,
            synchronizing: false,
            visual_edited: false,
            source_cursor_before_visual: None,
            mode: EditorMode::Code,
            history: EditorHistory::new(empty),
        }));
        let dirty_callback: DirtyCallback = Rc::new(RefCell::new(None));
        let history_callback: HistoryCallback = Rc::new(RefCell::new(None));

        {
            let runtime = runtime.clone();
            let source_buffer = source_buffer.clone();
            let visual_buffer = visual_buffer.clone();
            let visual_view = visual_view.clone();
            let tags = tags.clone();
            let source_view_weak = source_view.downgrade();
            let diagnostics_list_weak = diagnostics_list.downgrade();
            let diagnostics_revealer_weak = diagnostics_revealer.downgrade();
            let diagnostics_heading_weak = diagnostics_heading.downgrade();
            let syntax_error_tag_weak = syntax_error_tag.downgrade();
            let diagnostics_timer = diagnostics_timer.clone();
            let dirty_callback = dirty_callback.clone();
            let history_callback = history_callback.clone();
            source_buffer.clone().connect_changed(move |buffer| {
                if runtime.borrow().synchronizing {
                    return;
                }
                let text = buffer_text(buffer);
                runtime.borrow_mut().synchronizing = true;
                render_visual(&text, &visual_buffer, &visual_view, &tags);
                runtime.borrow_mut().synchronizing = false;
                runtime.borrow_mut().visual_edited = false;
                update_dirty(&runtime, &text, &dirty_callback);
                let mode = runtime.borrow().mode;
                record_history(
                    &runtime,
                    &source_buffer,
                    &visual_buffer,
                    match mode {
                        EditorMode::Code => "typing:code",
                        EditorMode::Visual => "typing:visual",
                    },
                );
                notify_history(&runtime, &history_callback);
                if let (Some(view), Some(list), Some(revealer), Some(heading), Some(error_tag)) = (
                    source_view_weak.upgrade(),
                    diagnostics_list_weak.upgrade(),
                    diagnostics_revealer_weak.upgrade(),
                    diagnostics_heading_weak.upgrade(),
                    syntax_error_tag_weak.upgrade(),
                ) {
                    schedule_syntax_check(
                        buffer,
                        &view,
                        &list,
                        (&revealer, &heading),
                        &error_tag,
                        &diagnostics_timer,
                        &runtime,
                    );
                }
            });
        }
        {
            let runtime = runtime.clone();
            let source_buffer = source_buffer.clone();
            let visual_buffer = visual_buffer.clone();
            let tags = tags.clone();
            let source_view_weak = source_view.downgrade();
            let diagnostics_list_weak = diagnostics_list.downgrade();
            let diagnostics_revealer_weak = diagnostics_revealer.downgrade();
            let diagnostics_heading_weak = diagnostics_heading.downgrade();
            let syntax_error_tag_weak = syntax_error_tag.downgrade();
            let diagnostics_timer = diagnostics_timer.clone();
            let dirty_callback = dirty_callback.clone();
            let history_callback = history_callback.clone();
            visual_buffer.clone().connect_changed(move |buffer| {
                if runtime.borrow().synchronizing {
                    return;
                }
                let source = buffer_text(&source_buffer);
                let document = latex::split_document(&source);
                if !document.valid {
                    return;
                }
                let tokens = tokens_from_visual(buffer, &tags);
                let offset = buffer.iter_at_mark(&buffer.get_insert()).offset().max(0) as usize;
                let body = latex::serialize_visual_tokens(&tokens);
                let prefix = latex::serialize_visual_tokens(&tokens_before_offset(&tokens, offset));
                let updated = format!("{}{}{}", document.preamble, body, document.ending);
                runtime.borrow_mut().synchronizing = true;
                source_buffer.set_text(&updated);
                let source_offset = character_count(&document.preamble) + character_count(&prefix);
                source_buffer.place_cursor(
                    &source_buffer
                        .iter_at_offset(source_offset.min(character_count(&updated)) as i32),
                );
                runtime.borrow_mut().synchronizing = false;
                runtime.borrow_mut().visual_edited = true;
                update_dirty(&runtime, &updated, &dirty_callback);
                record_history(&runtime, &source_buffer, &visual_buffer, "typing:visual");
                notify_history(&runtime, &history_callback);
                if let (Some(view), Some(list), Some(revealer), Some(heading), Some(error_tag)) = (
                    source_view_weak.upgrade(),
                    diagnostics_list_weak.upgrade(),
                    diagnostics_revealer_weak.upgrade(),
                    diagnostics_heading_weak.upgrade(),
                    syntax_error_tag_weak.upgrade(),
                ) {
                    schedule_syntax_check(
                        &source_buffer,
                        &view,
                        &list,
                        (&revealer, &heading),
                        &error_tag,
                        &diagnostics_timer,
                        &runtime,
                    );
                }
            });
        }

        let editor = Self {
            root,
            source_buffer,
            source_view,
            visual_buffer,
            visual_view,
            stack,
            runtime,
            tags,
            dirty_callback,
            history_callback,
            diagnostics_revealer,
            diagnostics_heading,
            diagnostics_list,
            syntax_error_tag,
            diagnostics_timer,
        };
        editor.load_text(&Self::new_document_text());
        editor
    }

    pub fn new_document_text() -> String {
        format!(
            "\\documentclass{{article}}\n\\usepackage[utf8]{{inputenc}}\n\\usepackage{{graphicx}}\n\\usepackage{{hyperref}}\n\\title{{{}}}\n\\author{{}}\n\\date{{\\today}}\n\n\\begin{{document}}\n\\maketitle\n\n\\section{{{}}}\n{}\n\n\\bibliographystyle{{plain}}\n\\bibliography{{references}}\n\\end{{document}}\n",
            crate::i18n::gettext("New document"),
            crate::i18n::gettext("Introduction"),
            crate::i18n::gettext("Start writing here."),
        )
    }

    pub fn text(&self) -> String {
        buffer_text(&self.source_buffer)
    }

    pub fn file(&self) -> Option<PathBuf> {
        self.runtime.borrow().file.clone()
    }

    pub fn dirty(&self) -> bool {
        self.runtime.borrow().dirty
    }

    pub fn mode(&self) -> EditorMode {
        self.runtime.borrow().mode
    }

    fn refresh_syntax_diagnostics(&self) {
        schedule_syntax_check(
            &self.source_buffer,
            &self.source_view,
            &self.diagnostics_list,
            (&self.diagnostics_revealer, &self.diagnostics_heading),
            &self.syntax_error_tag,
            &self.diagnostics_timer,
            &self.runtime,
        );
    }

    pub fn set_dirty_changed<F: Fn(bool) + 'static>(&self, callback: F) {
        *self.dirty_callback.borrow_mut() = Some(Box::new(callback));
    }

    pub fn set_history_changed<F: Fn(bool, bool) + 'static>(&self, callback: F) {
        *self.history_callback.borrow_mut() = Some(Box::new(callback));
        notify_history(&self.runtime, &self.history_callback);
    }

    pub fn set_file(&self, file: Option<PathBuf>) {
        self.runtime.borrow_mut().file = file.clone();
        let manager = sourceview5::LanguageManager::default();
        let language = match file.as_ref() {
            None => manager.language("latex"),
            Some(path) => path
                .extension()
                .and_then(|extension| extension.to_str())
                .map(str::to_ascii_lowercase)
                .and_then(|extension| match extension.as_str() {
                    "bib" => manager.language("bibtex"),
                    "tex" | "cls" | "sty" => manager.language("latex"),
                    _ => None,
                }),
        };
        self.source_buffer.set_language(language.as_ref());
        self.refresh_syntax_diagnostics();
    }

    pub fn load_text(&self, text: &str) {
        {
            let mut runtime = self.runtime.borrow_mut();
            runtime.synchronizing = true;
            runtime.saved_text = text.to_owned();
            runtime.dirty = false;
            runtime.visual_edited = false;
            runtime.source_cursor_before_visual = None;
            runtime.mode = EditorMode::Code;
        }
        self.stack.set_visible_child_name("code");
        self.source_buffer.set_text(text);
        render_visual(text, &self.visual_buffer, &self.visual_view, &self.tags);
        self.source_buffer
            .place_cursor(&self.source_buffer.start_iter());
        self.visual_buffer
            .place_cursor(&self.visual_buffer.start_iter());
        self.runtime.borrow_mut().synchronizing = false;
        self.runtime.borrow_mut().history = EditorHistory::new(snapshot(
            &self.source_buffer,
            &self.visual_buffer,
            EditorMode::Code,
        ));
        if let Some(callback) = self.dirty_callback.borrow().as_ref() {
            callback(false);
        }
        notify_history(&self.runtime, &self.history_callback);
        self.refresh_syntax_diagnostics();
    }

    pub fn mark_saved(&self, saved_text: &str) {
        let current_text = self.text();
        let changed = {
            let mut runtime = self.runtime.borrow_mut();
            runtime.saved_text = saved_text.to_owned();
            let dirty = current_text != saved_text;
            let changed = dirty != runtime.dirty;
            runtime.dirty = dirty;
            changed
        };
        if changed {
            if let Some(callback) = self.dirty_callback.borrow().as_ref() {
                callback(self.dirty());
            }
        }
    }

    pub fn set_mode(&self, mode: EditorMode) {
        if mode == EditorMode::Visual
            && !visual_mode_supported(self.runtime.borrow().file.as_deref())
        {
            return;
        }
        if mode == EditorMode::Visual {
            let source_offset = self
                .source_buffer
                .iter_at_mark(&self.source_buffer.get_insert())
                .offset();
            self.runtime.borrow_mut().source_cursor_before_visual = Some(source_offset);
            let source = self.text();
            let document = latex::split_document(&source);
            if document.valid {
                let body_start = character_count(&document.preamble);
                let body_offset = (source_offset as usize)
                    .saturating_sub(body_start)
                    .min(character_count(&document.body));
                let body_prefix = document.body.chars().take(body_offset).collect::<String>();
                let visual_offset =
                    character_count(&tokens_text(&latex::parse_visual_body(&body_prefix)));
                self.visual_buffer
                    .place_cursor(&self.visual_buffer.iter_at_offset(visual_offset as i32));
            }
        } else {
            let runtime = self.runtime.borrow();
            if !runtime.visual_edited {
                if let Some(offset) = runtime.source_cursor_before_visual {
                    self.source_buffer
                        .place_cursor(&self.source_buffer.iter_at_offset(offset));
                }
            }
        }
        self.runtime.borrow_mut().mode = mode;
        self.stack.set_visible_child_name(match mode {
            EditorMode::Code => "code",
            EditorMode::Visual => "visual",
        });
        self.refresh_syntax_diagnostics();
        self.focus();
    }

    fn active_buffer(&self) -> TextBuffer {
        match self.mode() {
            EditorMode::Code => self.source_buffer.clone().upcast(),
            EditorMode::Visual => self.visual_buffer.clone(),
        }
    }

    pub fn selected_text(&self) -> String {
        let buffer = self.active_buffer();
        buffer
            .selection_bounds()
            .map(|(start, end)| buffer.text(&start, &end, true).to_string())
            .unwrap_or_default()
    }

    pub fn insert_latex(&self, insertion: &LatexInsertion, group: &str) {
        let buffer = self.active_buffer();
        let Some((start, end)) = buffer.selection_bounds() else {
            let insert = buffer.iter_at_mark(&buffer.get_insert());
            let start = insert;
            let end = start.clone();
            self.apply_insertion(&buffer, &start, &end, insertion, group);
            return;
        };
        self.apply_insertion(&buffer, &start, &end, insertion, group);
    }

    fn apply_insertion(
        &self,
        buffer: &TextBuffer,
        start: &gtk::TextIter,
        end: &gtk::TextIter,
        insertion: &LatexInsertion,
        group: &str,
    ) {
        let base = start.offset().max(0) as usize;
        let mut start = start.clone();
        let mut end = end.clone();
        self.runtime.borrow_mut().synchronizing = true;
        buffer.delete(&mut start, &mut end);
        let mut insert_at = buffer.iter_at_offset(base as i32);
        buffer.insert(&mut insert_at, &insertion.text);
        if self.mode() == EditorMode::Visual {
            let tag_start = buffer.iter_at_offset(base as i32);
            let tag_end = buffer.iter_at_offset((base + character_count(&insertion.text)) as i32);
            if let Some(raw) = self.tags.get("raw") {
                buffer.apply_tag(raw, &tag_start, &tag_end);
            }
        }
        if let Some((selection_start, selection_end)) = insertion.selection {
            if selection_end > selection_start {
                let selected_start = buffer.iter_at_offset((base + selection_start) as i32);
                let selected_end = buffer.iter_at_offset((base + selection_end) as i32);
                buffer.select_range(&selected_end, &selected_start);
            }
        } else {
            let cursor = (base
                + insertion
                    .cursor_offset
                    .min(character_count(&insertion.text))) as i32;
            buffer.place_cursor(&buffer.iter_at_offset(cursor));
        }
        self.runtime.borrow_mut().synchronizing = false;
        self.finish_edit(group);
    }

    fn finish_edit(&self, group: &str) {
        if self.mode() == EditorMode::Visual {
            self.sync_source_from_visual();
        } else {
            let text = self.text();
            self.runtime.borrow_mut().synchronizing = true;
            render_visual(&text, &self.visual_buffer, &self.visual_view, &self.tags);
            self.runtime.borrow_mut().synchronizing = false;
            update_dirty(&self.runtime, &text, &self.dirty_callback);
        }
        record_history(
            &self.runtime,
            &self.source_buffer,
            &self.visual_buffer,
            group,
        );
        notify_history(&self.runtime, &self.history_callback);
        self.refresh_syntax_diagnostics();
    }

    fn sync_source_from_visual(&self) {
        let source = self.text();
        let document = latex::split_document(&source);
        if !document.valid {
            return;
        }
        let tokens = tokens_from_visual(&self.visual_buffer, &self.tags);
        let body = latex::serialize_visual_tokens(&tokens);
        let offset = self
            .visual_buffer
            .iter_at_mark(&self.visual_buffer.get_insert())
            .offset()
            .max(0) as usize;
        let prefix = latex::serialize_visual_tokens(&tokens_before_offset(&tokens, offset));
        let updated = format!("{}{}{}", document.preamble, body, document.ending);
        self.runtime.borrow_mut().synchronizing = true;
        self.source_buffer.set_text(&updated);
        let source_offset = character_count(&document.preamble) + character_count(&prefix);
        self.source_buffer.place_cursor(
            &self
                .source_buffer
                .iter_at_offset(source_offset.min(character_count(&updated)) as i32),
        );
        self.runtime.borrow_mut().synchronizing = false;
        self.runtime.borrow_mut().visual_edited = true;
        update_dirty(&self.runtime, &updated, &self.dirty_callback);
    }

    pub fn apply_format(&self, name: &str) -> Result<(), String> {
        let buffer = self.active_buffer();
        let selected = self.selected_text();
        if self.mode() == EditorMode::Visual && !selected.is_empty() {
            let Some(tag) = self.tags.get(name) else {
                return Ok(());
            };
            let Some((start, end)) = buffer.selection_bounds() else {
                return Ok(());
            };
            if start.has_tag(tag) {
                buffer.remove_tag(tag, &start, &end);
            } else {
                buffer.apply_tag(tag, &start, &end);
            }
            self.finish_edit(&format!("toolbar:{name}"));
            return Ok(());
        }
        let insertion = commands::create_inline_command(name, &selected)?;
        self.insert_latex(&insertion, &format!("toolbar:{name}"));
        Ok(())
    }

    pub fn apply_heading(&self, style: &str) -> Result<(), String> {
        let buffer = self.active_buffer();
        if self.mode() == EditorMode::Visual {
            let selection = buffer.selection_bounds();
            let (start, end) = selection.clone().unwrap_or_else(|| {
                let mut start = buffer.iter_at_mark(&buffer.get_insert());
                start.set_line_offset(0);
                let mut end = start.clone();
                end.forward_to_line_end();
                (start, end)
            });
            let name = match style {
                "section" => Some("heading1"),
                "subsection" => Some("heading2"),
                "subsubsection" => Some("heading3"),
                "paragraph" => Some("heading4"),
                "subparagraph" => Some("heading5"),
                "normal" => None,
                _ => {
                    return Err(
                        crate::i18n::gettext("Unknown paragraph style: %s").replace("%s", style)
                    )
                }
            };
            if start.offset() == end.offset() {
                let insertion = commands::create_heading_command(style, "")?;
                self.insert_latex(&insertion, &format!("toolbar:heading:{style}"));
                return Ok(());
            }
            for level in 1..=5 {
                if let Some(tag) = self.tags.get(&format!("heading{level}")) {
                    buffer.remove_tag(tag, &start, &end);
                }
            }
            if let Some(name) = name {
                if let Some(tag) = self.tags.get(name) {
                    buffer.apply_tag(tag, &start, &end);
                }
            }
            self.finish_edit(&format!("toolbar:heading:{style}"));
            return Ok(());
        }
        let insertion = commands::create_heading_command(style, &self.selected_text())?;
        if style != "normal" || !self.selected_text().is_empty() {
            self.insert_latex(&insertion, &format!("toolbar:heading:{style}"));
        }
        Ok(())
    }

    pub fn insert_at_cursor(&self, snippet: &str, cursor_offset: usize, group: &str) {
        let buffer = self.active_buffer();
        if let Some((_start, end)) = buffer.selection_bounds() {
            buffer.place_cursor(&end);
        }
        self.insert_latex(
            &LatexInsertion {
                text: snippet.to_owned(),
                cursor_offset,
                selection: None,
            },
            group,
        );
    }

    pub fn insert_source_at_offset(&self, snippet: &str, offset: usize, group: &str) {
        let source = self.text();
        let offset = offset.min(character_count(&source));
        let visual_cursor = (self.mode() == EditorMode::Visual).then(|| {
            self.visual_buffer
                .iter_at_mark(&self.visual_buffer.get_insert())
                .offset()
                .max(0) as usize
        });
        let mut insert_at = self.source_buffer.iter_at_offset(offset as i32);
        self.runtime.borrow_mut().synchronizing = true;
        self.source_buffer.insert(&mut insert_at, snippet);
        let updated = self.text();
        render_visual(&updated, &self.visual_buffer, &self.visual_view, &self.tags);
        if let Some(cursor) = visual_cursor {
            let visual_length = character_count(&buffer_text(&self.visual_buffer));
            self.visual_buffer.place_cursor(
                &self
                    .visual_buffer
                    .iter_at_offset(cursor.min(visual_length) as i32),
            );
        }
        self.runtime.borrow_mut().synchronizing = false;
        update_dirty(&self.runtime, &updated, &self.dirty_callback);
        record_history(
            &self.runtime,
            &self.source_buffer,
            &self.visual_buffer,
            group,
        );
        notify_history(&self.runtime, &self.history_callback);
    }

    pub fn insert_comment(&self) {
        let selected = self.selected_text();
        let comment = if selected.is_empty() {
            "% ".to_owned()
        } else {
            selected
                .lines()
                .map(|line| {
                    if line.is_empty() {
                        "%".to_owned()
                    } else {
                        format!("% {line}")
                    }
                })
                .collect::<Vec<_>>()
                .join("\n")
        };
        let length = character_count(&comment);
        self.insert_at_cursor(&comment, length, "insert:comment");
    }

    pub fn insert_footnote(&self) {
        let selected = self.selected_text();
        let insertion = if selected.is_empty() {
            LatexInsertion {
                text: "\\footnote{}".to_owned(),
                cursor_offset: 10,
                selection: None,
            }
        } else {
            let text = format!("\\footnote{{{selected}}}");
            let cursor_offset = character_count(&text);
            LatexInsertion {
                text,
                cursor_offset,
                selection: None,
            }
        };
        self.insert_latex(&insertion, "insert:footnote");
    }

    pub fn search(&self, query: &str, forward: bool) -> bool {
        self.search_with_start(query, forward, None)
    }

    pub fn search_from_start(&self, query: &str, forward: bool) -> bool {
        self.search_with_start(query, forward, Some(0))
    }

    fn search_with_start(&self, query: &str, forward: bool, explicit_start: Option<usize>) -> bool {
        let buffer = self.active_buffer();
        let text = buffer_text(&buffer);
        let cursor = buffer.iter_at_mark(&buffer.get_insert()).offset().max(0) as usize;
        let selection = buffer.selection_bounds();
        let selection_offsets = selection
            .as_ref()
            .map(|(start, end)| (start.offset().max(0) as usize, end.offset().max(0) as usize));
        let search_start = explicit_start.unwrap_or_else(|| match selection_offsets {
            Some((_, end)) if forward => end,
            Some((start, _)) => start,
            None => cursor,
        });
        let Some(found) = find_document_match(&text, query, search_start, forward) else {
            if selection.is_some() {
                let insert = buffer.iter_at_mark(&buffer.get_insert());
                buffer.place_cursor(&insert);
            }
            return false;
        };
        let start = buffer.iter_at_offset(found.start as i32);
        let end = buffer.iter_at_offset(found.end as i32);
        buffer.select_range(&end, &start);
        let mut scroll = start;
        match self.mode() {
            EditorMode::Code => {
                self.source_view
                    .scroll_to_iter(&mut scroll, 0.15, false, 0.0, 0.5);
            }
            EditorMode::Visual => {
                self.visual_view
                    .scroll_to_iter(&mut scroll, 0.15, false, 0.0, 0.5);
            }
        }
        true
    }

    pub fn adjust_indent(&self, increase: bool) {
        let buffer = self.active_buffer();
        let text = buffer_text(&buffer);
        let selection = buffer
            .selection_bounds()
            .map(|(start, end)| (start.offset().max(0) as usize, end.offset().max(0) as usize));
        let cursor = buffer.iter_at_mark(&buffer.get_insert()).offset().max(0) as usize;
        let line_starts = indent_line_starts(&text, selection, cursor);
        if line_starts.is_empty() {
            return;
        }

        let visual = self.mode() == EditorMode::Visual;
        let mut changed = false;
        self.runtime.borrow_mut().synchronizing = true;
        for offset in line_starts.into_iter().rev() {
            let start = buffer.iter_at_offset(offset as i32);
            let mut line_end = start.clone();
            line_end.forward_to_line_end();
            let line = buffer.text(&start, &line_end, true).to_string();
            let Some(transformed) = indent_line_text(&line, increase) else {
                continue;
            };
            if transformed == line {
                continue;
            }

            if increase {
                let mut insert_at = start;
                buffer.insert(&mut insert_at, INDENT_PREFIX);
                if visual {
                    if let Some(raw) = self.tags.get("raw") {
                        let prefix_start = buffer.iter_at_offset(offset as i32);
                        let prefix_end =
                            buffer.iter_at_offset((offset + character_count(INDENT_PREFIX)) as i32);
                        buffer.apply_tag(raw, &prefix_start, &prefix_end);
                    }
                }
            } else {
                let mut delete_start = start;
                let mut delete_end =
                    buffer.iter_at_offset((offset + character_count(INDENT_PREFIX)) as i32);
                buffer.delete(&mut delete_start, &mut delete_end);
            }
            changed = true;
        }
        self.runtime.borrow_mut().synchronizing = false;
        if changed {
            self.finish_edit(if increase {
                "toolbar:indent-increase"
            } else {
                "toolbar:indent-decrease"
            });
        }
    }

    pub fn undo(&self) {
        self.restore_history(false);
    }

    pub fn redo(&self) {
        self.restore_history(true);
    }

    fn restore_history(&self, redo: bool) {
        let state = {
            let mut runtime = self.runtime.borrow_mut();
            if redo {
                runtime.history.redo().cloned()
            } else {
                runtime.history.undo().cloned()
            }
        };
        let Some(state) = state else {
            return;
        };
        self.runtime.borrow_mut().synchronizing = true;
        self.source_buffer.set_text(&state.source);
        render_visual(
            &state.source,
            &self.visual_buffer,
            &self.visual_view,
            &self.tags,
        );
        self.source_buffer.place_cursor(
            &self
                .source_buffer
                .iter_at_offset(state.source_cursor.min(character_count(&state.source)) as i32),
        );
        self.visual_buffer.place_cursor(
            &self
                .visual_buffer
                .iter_at_offset(state.visual_cursor.min(character_count(&tokens_text(
                    &latex::parse_visual_body(&latex::split_document(&state.source).body),
                ))) as i32),
        );
        self.runtime.borrow_mut().synchronizing = false;
        self.runtime.borrow_mut().mode = state.mode;
        self.runtime.borrow_mut().visual_edited = state.mode == EditorMode::Visual;
        self.stack.set_visible_child_name(match state.mode {
            EditorMode::Code => "code",
            EditorMode::Visual => "visual",
        });
        update_dirty(&self.runtime, &state.source, &self.dirty_callback);
        notify_history(&self.runtime, &self.history_callback);
        self.refresh_syntax_diagnostics();
    }

    pub fn focus(&self) {
        match self.mode() {
            EditorMode::Code => self.source_view.grab_focus(),
            EditorMode::Visual if self.visual_view.is_editable() => self.visual_view.grab_focus(),
            EditorMode::Visual => self.source_view.grab_focus(),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::{
        indent_line_starts, indent_line_text, source_style_scheme_id, visual_mode_supported,
        LatexEditor,
    };
    use crate::history::EditorMode;
    use gtk::prelude::*;
    use std::path::{Path, PathBuf};
    use std::time::{Duration, Instant};

    fn editor_with_text(text: &str) -> LatexEditor {
        let editor = LatexEditor::new();
        editor.load_text(text);
        editor
    }

    #[test]
    fn editor_interactions_preserve_search_mode_and_indentation_behavior() {
        gtk::init().expect("GTK must initialize for editor tests");

        let editor = editor_with_text("\\begin{document}\n  café $x\n\\end{document}");
        let context = gtk::glib::MainContext::default();
        let deadline = Instant::now() + Duration::from_secs(2);
        while editor.diagnostics_list.first_child().is_none() && Instant::now() < deadline {
            context.iteration(false);
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(editor.diagnostics_revealer.reveals_child());
        let button = editor
            .diagnostics_list
            .first_child()
            .expect("syntax diagnostic should be listed")
            .downcast::<gtk::Button>()
            .expect("syntax diagnostic should be clickable");
        let location = format!("{} 2:", crate::i18n::gettext("Line"));
        assert!(button.label().unwrap().contains(&location));
        button.emit_clicked();
        let cursor = editor
            .source_buffer
            .iter_at_mark(&editor.source_buffer.get_insert());
        assert_eq!((cursor.line(), cursor.line_offset()), (1, 7));

        editor
            .source_buffer
            .set_text("\\begin{document}\n  café $x$\n\\end{document}");
        let deadline = Instant::now() + Duration::from_secs(2);
        while !editor.diagnostics_heading.has_css_class("success") && Instant::now() < deadline {
            context.iteration(false);
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(editor.diagnostics_revealer.reveals_child());
        assert!(editor.diagnostics_heading.has_css_class("success"));
        assert_eq!(
            editor.diagnostics_heading.text().as_str(),
            crate::i18n::gettext("The document appears ready to compile.")
        );
        assert!(editor.diagnostics_list.first_child().is_none());

        let editor = editor_with_text("a a a");
        let buffer = editor.source_buffer.clone().upcast::<gtk::TextBuffer>();
        let start = buffer.iter_at_offset(2);
        let end = buffer.iter_at_offset(3);
        buffer.select_range(&end, &start);
        assert!(editor.search("a", false));
        assert_eq!(buffer.selection_bounds().unwrap().0.offset(), 0);

        let editor = editor_with_text("a a a");
        let buffer = editor.source_buffer.clone().upcast::<gtk::TextBuffer>();
        let start = buffer.iter_at_offset(2);
        let end = buffer.iter_at_offset(3);
        buffer.select_range(&start, &end);
        assert!(editor.search("a", true));
        assert_eq!(buffer.selection_bounds().unwrap().0.offset(), 4);

        let editor = editor_with_text("alpha beta");
        let buffer = editor.source_buffer.clone().upcast::<gtk::TextBuffer>();
        let start = buffer.iter_at_offset(0);
        let end = buffer.iter_at_offset(5);
        buffer.select_range(&end, &start);
        assert!(!editor.search("missing", true));
        assert!(buffer.selection_bounds().is_none());

        let editor = editor_with_text("a a a");
        let buffer = editor.source_buffer.clone().upcast::<gtk::TextBuffer>();
        let start = buffer.iter_at_offset(4);
        let end = buffer.iter_at_offset(5);
        buffer.select_range(&end, &start);
        assert!(editor.search_from_start("a", true));
        assert_eq!(buffer.selection_bounds().unwrap().0.offset(), 0);

        let editor = editor_with_text("content");
        for path in ["references.bib", "custom.sty"] {
            editor.set_file(Some(PathBuf::from(path)));
            editor.set_mode(EditorMode::Visual);
            assert_eq!(editor.mode(), EditorMode::Code, "{path}");
        }

        let editor = editor_with_text("alpha\nbeta");
        let buffer = editor.source_buffer.clone().upcast::<gtk::TextBuffer>();
        buffer.place_cursor(&buffer.iter_at_offset(8));
        editor.adjust_indent(true);
        assert_eq!(editor.text(), "alpha\n\\hspace{1em}beta");
        editor.undo();
        assert_eq!(editor.text(), "alpha\nbeta");

        let editor = editor_with_text("alpha\nbeta\ngamma");
        let buffer = editor.source_buffer.clone().upcast::<gtk::TextBuffer>();
        let start = buffer.iter_at_offset(1);
        let end = buffer.iter_at_offset(11);
        buffer.select_range(&end, &start);
        editor.adjust_indent(true);
        assert_eq!(
            editor.text(),
            "\\hspace{1em}alpha\n\\hspace{1em}beta\ngamma"
        );
        editor.undo();
        assert_eq!(editor.text(), "alpha\nbeta\ngamma");
    }

    #[test]
    fn visual_mode_policy_rejects_bibliography_and_style_files() {
        assert!(!visual_mode_supported(Some(Path::new("references.bib"))));
        assert!(!visual_mode_supported(Some(Path::new("custom.sty"))));
        assert!(visual_mode_supported(Some(Path::new("main.tex"))));
        assert!(visual_mode_supported(None));
    }

    #[test]
    fn source_style_scheme_tracks_light_and_dark_mode() {
        assert_eq!(source_style_scheme_id(false), "Adwaita");
        assert_eq!(source_style_scheme_id(true), "Adwaita-dark");
    }

    #[test]
    fn indentation_transforms_nonblank_lines_with_latex_spacing_only() {
        assert_eq!(
            indent_line_text("texto", true),
            Some("\\hspace{1em}texto".into())
        );
        assert_eq!(indent_line_text("   ", true), None);
        assert_eq!(
            indent_line_text("\\hspace{1em}texto", false),
            Some("texto".into())
        );
        assert_eq!(indent_line_text(" texto", false), None);
    }

    #[test]
    fn indentation_line_range_uses_cursor_or_multiline_selection() {
        assert_eq!(indent_line_starts("alpha\nbeta", None, 8), vec![6]);
        assert_eq!(
            indent_line_starts("alpha\nbeta\ngamma", Some((1, 11)), 11),
            vec![0, 6]
        );
    }
}
