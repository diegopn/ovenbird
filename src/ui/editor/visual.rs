use super::*;

pub(super) fn visual_mode_supported(file: Option<&Path>) -> bool {
    file.is_none_or(|path| {
        path.extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("tex"))
    })
}

pub(super) fn indent_line_starts(
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

pub(super) fn indent_line_text(line: &str, increase: bool) -> Option<String> {
    if increase {
        (!line.trim().is_empty()).then(|| format!("{INDENT_PREFIX}{line}"))
    } else {
        line.strip_prefix(INDENT_PREFIX).map(str::to_owned)
    }
}

pub(super) fn add_tag(table: &gtk::TextTagTable, name: &str) -> TextTag {
    let tag = TextTag::new(Some(name));
    table.add(&tag);
    tag
}

pub(super) fn create_tags(buffer: &TextBuffer) -> HashMap<String, TextTag> {
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

pub(super) fn token_text(token: &Token) -> &str {
    match token {
        Token::Text { text, .. } | Token::Raw(text) | Token::Paragraph(text) => text,
    }
}

pub(super) fn tokens_text(tokens: &[Token]) -> String {
    tokens.iter().map(token_text).collect()
}

pub(super) fn render_visual(
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

pub(super) fn tokens_from_visual(
    buffer: &TextBuffer,
    tags: &HashMap<String, TextTag>,
) -> Vec<Token> {
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

pub(super) fn tokens_before_offset(tokens: &[Token], offset: usize) -> Vec<Token> {
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
