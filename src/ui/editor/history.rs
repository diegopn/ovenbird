use super::*;

impl LatexEditor {
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
            let mut line_end = start;
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

    pub(super) fn restore_history(&self, redo: bool) {
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
