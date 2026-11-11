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
    visual_cursor_before_visual: Option<(i32, i32)>,
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

mod construction;
mod diagnostics;
mod document;
mod editing;
mod history;
mod history_helpers;
mod search;
mod utilities;
mod visual;

use self::construction::*;
use self::diagnostics::*;
use self::document::*;
use self::editing::*;
use self::history::*;
use self::history_helpers::*;
use self::search::*;
use self::utilities::*;
use self::visual::*;

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
