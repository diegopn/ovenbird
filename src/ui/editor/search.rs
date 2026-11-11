use super::*;

impl LatexEditor {
    pub fn search(&self, query: &str, forward: bool) -> bool {
        self.search_with_start(query, forward, None)
    }

    pub fn search_from_start(&self, query: &str, forward: bool) -> bool {
        self.search_with_start(query, forward, Some(0))
    }

    pub(super) fn search_with_start(
        &self,
        query: &str,
        forward: bool,
        explicit_start: Option<usize>,
    ) -> bool {
        let buffer = self.active_buffer();
        let text = buffer_text(&buffer);
        let cursor = buffer.iter_at_mark(&buffer.get_insert()).offset().max(0) as usize;
        let selection = buffer.selection_bounds();
        let selection_offsets = selection
            .as_ref()
            .map(|(start, end)| (start.offset().max(0) as usize, end.offset().max(0) as usize));
        let search_start = explicit_start.unwrap_or(match selection_offsets {
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
}
