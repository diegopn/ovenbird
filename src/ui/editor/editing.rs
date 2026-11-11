use super::*;

impl LatexEditor {
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
            let end = start;
            self.apply_insertion(&buffer, &start, &end, insertion, group);
            return;
        };
        self.apply_insertion(&buffer, &start, &end, insertion, group);
    }

    pub(super) fn apply_insertion(
        &self,
        buffer: &TextBuffer,
        start: &gtk::TextIter,
        end: &gtk::TextIter,
        insertion: &LatexInsertion,
        group: &str,
    ) {
        let base = start.offset().max(0) as usize;
        let mut start = *start;
        let mut end = *end;
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

    pub(super) fn finish_edit(&self, group: &str) {
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

    pub(super) fn sync_source_from_visual(&self) {
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
            let (start, end) = selection.unwrap_or_else(|| {
                let mut start = buffer.iter_at_mark(&buffer.get_insert());
                start.set_line_offset(0);
                let mut end = start;
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
}
