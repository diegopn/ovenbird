#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LatexInsertion {
    pub text: String,
    pub cursor_offset: usize,
    pub selection: Option<(usize, usize)>,
}

fn wrap(command: &str, selected: &str) -> LatexInsertion {
    let prefix = format!("\\{command}{{");
    let selected_len = selected.chars().count();
    let cursor_offset = prefix.chars().count() + selected_len;
    LatexInsertion {
        text: format!("{prefix}{selected}}}"),
        cursor_offset,
        selection: (!selected.is_empty()).then_some((prefix.chars().count(), cursor_offset)),
    }
}

pub fn create_inline_command(format: &str, selected: &str) -> Result<LatexInsertion, String> {
    let command = match format {
        "bold" => "textbf",
        "italic" => "emph",
        "underline" => "underline",
        "monospace" => "texttt",
        _ => return Err(crate::i18n::gettext("Unknown LaTeX formatting: %s").replace("%s", format)),
    };
    Ok(wrap(command, selected))
}

pub fn create_heading_command(style: &str, selected: &str) -> Result<LatexInsertion, String> {
    if style == "normal" {
        let text = [
            "section",
            "subsection",
            "subsubsection",
            "paragraph",
            "subparagraph",
        ]
        .iter()
        .find_map(|command| {
            selected
                .strip_prefix(&format!("\\{command}{{"))
                .and_then(|body| body.strip_suffix('}'))
        })
        .unwrap_or(selected)
        .to_owned();
        let text_len = text.chars().count();
        return Ok(LatexInsertion {
            text,
            cursor_offset: text_len,
            selection: (!selected.is_empty()).then_some((0, text_len)),
        });
    }
    let command = match style {
        "section" | "subsection" | "subsubsection" | "paragraph" | "subparagraph" => style,
        _ => return Err(crate::i18n::gettext("Unknown paragraph style: %s").replace("%s", style)),
    };
    Ok(wrap(command, selected))
}

pub fn create_list_snippet(kind: &str, selected: &str) -> Result<LatexInsertion, String> {
    if !["itemize", "enumerate", "quote"].contains(&kind) {
        return Err(crate::i18n::gettext("Unknown list environment: %s").replace("%s", kind));
    }
    if kind == "quote" {
        let prefix = "\\begin{quote}\n";
        let cursor_offset = prefix.chars().count() + selected.chars().count();
        return Ok(LatexInsertion {
            text: format!("{prefix}{selected}\n\\end{{quote}}"),
            cursor_offset,
            selection: None,
        });
    }
    let prefix = format!("\\begin{{{kind}}}\n");
    let normalized = selected.replace("\r\n", "\n");
    let lines = if normalized.is_empty() {
        vec![""]
    } else {
        normalized.split('\n').collect()
    };
    let items = lines
        .iter()
        .map(|line| format!("\\item {line}"))
        .collect::<Vec<_>>()
        .join("\n");
    let text = format!("{prefix}{items}\n\\end{{{kind}}}");
    let cursor_offset = if selected.is_empty() {
        prefix.chars().count() + "\\item ".chars().count()
    } else {
        text.chars().count()
    };
    Ok(LatexInsertion {
        text,
        cursor_offset,
        selection: None,
    })
}

pub fn create_math_snippet(display: bool, selected: &str) -> LatexInsertion {
    let (text, cursor_offset) = if display {
        (format!("\\[ {selected} \\]"), 3 + selected.chars().count())
    } else {
        (format!("\\({selected}\\)"), 2 + selected.chars().count())
    };
    LatexInsertion {
        text,
        cursor_offset,
        selection: None,
    }
}

pub fn create_table_snippet(rows: usize, columns: usize) -> LatexInsertion {
    let row_count = rows.clamp(1, 20);
    let column_count = columns.clamp(1, 12);
    let layout = format!("|{}|", vec!["l"; column_count].join("|"));
    let row = format!("{} \\\\", vec![""; column_count].join(" & "));
    let mut lines = vec!["\\hline".to_owned()];
    for _ in 0..row_count {
        lines.push(row.clone());
        lines.push("\\hline".to_owned());
    }
    let prefix = format!("\\begin{{tabular}}{{{layout}}}\n");
    let text = format!("{prefix}{}\n\\end{{tabular}}", lines.join("\n"));
    let cursor_offset = prefix.chars().count() + "\\hline\n".chars().count();
    LatexInsertion {
        text,
        cursor_offset,
        selection: None,
    }
}
