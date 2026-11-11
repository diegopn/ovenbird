use super::*;

impl LatexEditor {
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

    pub(super) fn refresh_syntax_diagnostics(&self) {
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
            runtime.visual_cursor_before_visual = None;
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
        if mode == self.mode() {
            self.focus();
            return;
        }
        if mode == EditorMode::Visual {
            {
                let mut runtime = self.runtime.borrow_mut();
                runtime.source_cursor_before_visual = None;
                runtime.visual_cursor_before_visual = None;
                runtime.visual_edited = false;
            }
            let source_offset = self
                .source_buffer
                .iter_at_mark(&self.source_buffer.get_insert())
                .offset();
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
                let visual_cursor = self
                    .visual_buffer
                    .iter_at_mark(&self.visual_buffer.get_insert());
                let mut runtime = self.runtime.borrow_mut();
                runtime.source_cursor_before_visual = Some(source_offset);
                runtime.visual_cursor_before_visual =
                    Some((visual_cursor.line(), visual_cursor.line_offset()));
                runtime.visual_edited = false;
            }
        } else {
            let (visual_edited, source_anchor, visual_anchor) = {
                let runtime = self.runtime.borrow();
                (
                    runtime.visual_edited,
                    runtime.source_cursor_before_visual,
                    runtime.visual_cursor_before_visual,
                )
            };
            if visual_edited {
                let source = self.text();
                let document = latex::split_document(&source);
                if document.valid {
                    let tokens = tokens_from_visual(&self.visual_buffer, &self.tags);
                    let visual_offset = self
                        .visual_buffer
                        .iter_at_mark(&self.visual_buffer.get_insert())
                        .offset()
                        .max(0) as usize;
                    let prefix = latex::serialize_visual_tokens(&tokens_before_offset(
                        &tokens,
                        visual_offset,
                    ));
                    let source_offset =
                        character_count(&document.preamble) + character_count(&prefix);
                    self.source_buffer.place_cursor(
                        &self
                            .source_buffer
                            .iter_at_offset(source_offset.min(character_count(&source)) as i32),
                    );
                }
            } else if let (Some(source_offset), Some((visual_line, visual_column))) =
                (source_anchor, visual_anchor)
            {
                let source_cursor = self.source_buffer.iter_at_offset(source_offset);
                let visual_cursor = self
                    .visual_buffer
                    .iter_at_mark(&self.visual_buffer.get_insert());
                let target_line =
                    (source_cursor.line() + visual_cursor.line() - visual_line).max(0);
                let target_column = (source_cursor.line_offset() + visual_cursor.line_offset()
                    - visual_column)
                    .max(0);
                let mut target = self
                    .source_buffer
                    .iter_at_line(target_line)
                    .unwrap_or_else(|| self.source_buffer.end_iter());
                let mut line_end = target;
                line_end.forward_to_line_end();
                let target_column = target_column.min(line_end.offset() - target.offset());
                target.forward_chars(target_column);
                self.source_buffer.place_cursor(&target);
            }
            let mut runtime = self.runtime.borrow_mut();
            runtime.source_cursor_before_visual = None;
            runtime.visual_cursor_before_visual = None;
            runtime.visual_edited = false;
        }
        self.runtime.borrow_mut().mode = mode;
        self.stack.set_visible_child_name(match mode {
            EditorMode::Code => "code",
            EditorMode::Visual => "visual",
        });
        self.refresh_syntax_diagnostics();
        self.focus();
    }

    pub(super) fn active_buffer(&self) -> TextBuffer {
        match self.mode() {
            EditorMode::Code => self.source_buffer.clone().upcast(),
            EditorMode::Visual => self.visual_buffer.clone(),
        }
    }
}
