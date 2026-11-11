use super::*;

impl Default for LatexEditor {
    fn default() -> Self {
        Self::new()
    }
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
            visual_cursor_before_visual: None,
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
}
