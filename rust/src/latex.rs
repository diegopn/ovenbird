#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Text { text: String, marks: Vec<String> },
    Raw(String),
    Paragraph(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LatexDocument {
    pub valid: bool,
    pub preamble: String,
    pub body: String,
    pub ending: String,
    pub original: String,
}

const HEADING_COMMANDS: [&str; 5] = [
    "section",
    "subsection",
    "subsubsection",
    "paragraph",
    "subparagraph",
];
const INLINE_COMMANDS: [(&str, &str); 5] = [
    ("textbf", "bold"),
    ("textit", "italic"),
    ("emph", "italic"),
    ("underline", "underline"),
    ("texttt", "monospace"),
];

pub fn split_document(source: &str) -> LatexDocument {
    let Some((_begin_start, begin_end)) = find_document_marker(source, "begin") else {
        return invalid_document(source);
    };
    let Some((end_start, _end_end)) = find_last_document_marker(source, "end") else {
        return invalid_document(source);
    };
    if end_start < begin_end {
        return invalid_document(source);
    }
    LatexDocument {
        valid: true,
        preamble: source[..begin_end].to_owned(),
        body: source[begin_end..end_start].to_owned(),
        ending: source[end_start..].to_owned(),
        original: source.to_owned(),
    }
}

fn find_document_marker(source: &str, marker: &str) -> Option<(usize, usize)> {
    let command = format!("\\{marker}");
    for (start, _) in source.match_indices(&command) {
        let mut position = start + command.len();
        while position < source.len() {
            let ch = source[position..].chars().next()?;
            if !ch.is_whitespace() {
                break;
            }
            position += ch.len_utf8();
        }
        if source[position..].starts_with("{document}") {
            return Some((start, position + "{document}".len()));
        }
    }
    None
}

fn find_last_document_marker(source: &str, marker: &str) -> Option<(usize, usize)> {
    let command = format!("\\{marker}");
    let mut found = None;
    for (start, _) in source.match_indices(&command) {
        let mut position = start + command.len();
        while position < source.len() {
            let ch = source[position..].chars().next()?;
            if !ch.is_whitespace() {
                break;
            }
            position += ch.len_utf8();
        }
        if source[position..].starts_with("{document}") {
            found = Some((start, position + "{document}".len()));
        }
    }
    found
}

fn invalid_document(source: &str) -> LatexDocument {
    LatexDocument {
        valid: false,
        preamble: String::new(),
        body: String::new(),
        ending: String::new(),
        original: source.to_owned(),
    }
}

fn read_group(source: &str, start: usize, open: char, close: char) -> Option<(String, usize)> {
    if source[start..].chars().next()? != open {
        return None;
    }
    let mut depth = 0usize;
    let mut escaped = false;
    for (offset, ch) in source[start..].char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == open {
            depth += 1;
        } else if ch == close {
            depth -= 1;
            if depth == 0 {
                let end = start + offset + ch.len_utf8();
                return Some((
                    source[start + open.len_utf8()..start + offset].to_owned(),
                    end,
                ));
            }
        }
    }
    None
}

fn skip_whitespace(source: &str, mut index: usize) -> usize {
    while index < source.len() {
        let ch = source[index..].chars().next().unwrap();
        if !ch.is_whitespace() {
            break;
        }
        index += ch.len_utf8();
    }
    index
}

fn command_at(source: &str, start: usize) -> Option<(String, usize)> {
    if !source[start..].starts_with('\\') {
        return None;
    }
    let after_slash = start + 1;
    if after_slash >= source.len() {
        return Some(("\\".to_owned(), after_slash));
    }
    let first = source[after_slash..].chars().next()?;
    if first.is_ascii_alphabetic() {
        let mut end = after_slash + first.len_utf8();
        while end < source.len() {
            let ch = source[end..].chars().next().unwrap();
            if !ch.is_ascii_alphabetic() {
                break;
            }
            end += ch.len_utf8();
        }
        Some((source[after_slash..end].to_owned(), end))
    } else {
        Some((first.to_string(), after_slash + first.len_utf8()))
    }
}

fn command_arguments(source: &str, start: usize) -> (Vec<String>, Vec<String>, usize, bool) {
    let mut index = start;
    let mut last_end = start;
    let mut optional = Vec::new();
    let mut required = Vec::new();
    loop {
        let candidate = skip_whitespace(source, index);
        if candidate >= source.len() {
            break;
        }
        let ch = source[candidate..].chars().next().unwrap();
        if ch == '[' {
            let Some((group, end)) = read_group(source, candidate, '[', ']') else {
                return (required, optional, source.len(), true);
            };
            optional.push(group);
            last_end = end;
            index = end;
        } else if ch == '{' {
            let Some((group, end)) = read_group(source, candidate, '{', '}') else {
                return (required, optional, source.len(), true);
            };
            required.push(group);
            last_end = end;
            index = end;
        } else {
            break;
        }
    }
    (required, optional, last_end, false)
}

fn push_text(tokens: &mut Vec<Token>, text: &str, marks: &[String]) {
    if text.is_empty() {
        return;
    }
    if let Some(Token::Text {
        text: previous,
        marks: previous_marks,
    }) = tokens.last_mut()
    {
        if previous_marks == marks {
            previous.push_str(text);
            return;
        }
    }
    tokens.push(Token::Text {
        text: text.to_owned(),
        marks: marks.to_vec(),
    });
}

fn matching_environment_end(source: &str, name: &str, from: usize) -> Option<usize> {
    let begin = format!("\\begin{{{name}}}");
    let end = format!("\\end{{{name}}}");
    let mut depth = 1usize;
    let mut position = from;
    while position < source.len() {
        let next_begin = source[position..]
            .find(&begin)
            .map(|offset| position + offset);
        let next_end = source[position..]
            .find(&end)
            .map(|offset| position + offset);
        match (next_begin, next_end) {
            (_, None) => return None,
            (Some(begin_index), Some(end_index)) if begin_index < end_index => {
                depth += 1;
                position = begin_index + begin.len();
            }
            (_, Some(end_index)) => {
                depth -= 1;
                let after = end_index + end.len();
                if depth == 0 {
                    return Some(after);
                }
                position = after;
            }
        }
    }
    None
}

fn parse_inline(source: &str, marks: &[String], tokens: &mut Vec<Token>) {
    let mut position = 0usize;
    while position < source.len() {
        let ch = source[position..].chars().next().unwrap();
        if ch == '$' || ch == '%' {
            let end = if ch == '%' {
                source[position..]
                    .find('\n')
                    .map(|offset| position + offset)
                    .unwrap_or(source.len())
            } else {
                let mut end = position + 1;
                while end < source.len() {
                    if source[end..].starts_with('$') && !source[..end].ends_with('\\') {
                        end += 1;
                        break;
                    }
                    end += source[end..].chars().next().unwrap().len_utf8();
                }
                end
            };
            tokens.push(Token::Raw(source[position..end].to_owned()));
            position = end;
            continue;
        }

        if ch == '\\' {
            let Some((name, after_command)) = command_at(source, position) else {
                push_text(tokens, "\\", marks);
                position += 1;
                continue;
            };
            if name == "par" {
                tokens.push(Token::Paragraph("\n\n".to_owned()));
                position = after_command;
                continue;
            }
            if name == "\\" {
                let mut finish = after_command;
                if source[finish..].starts_with('*') {
                    finish += 1;
                }
                let optional_start = skip_whitespace(source, finish);
                if let Some((_, end)) = read_group(source, optional_start, '[', ']') {
                    finish = end;
                }
                tokens.push(Token::Raw(source[position..finish].to_owned()));
                position = finish;
                continue;
            }
            if name == "verb" {
                let mut delimiter_at = after_command;
                if source[delimiter_at..].starts_with('*') {
                    delimiter_at += 1;
                }
                if delimiter_at < source.len() {
                    let delimiter = source[delimiter_at..].chars().next().unwrap();
                    let content_start = delimiter_at + delimiter.len_utf8();
                    let finish = source[content_start..]
                        .find(delimiter)
                        .map(|offset| content_start + offset + delimiter.len_utf8())
                        .unwrap_or(source.len());
                    tokens.push(Token::Raw(source[position..finish].to_owned()));
                    position = finish;
                    continue;
                }
            }
            if HEADING_COMMANDS.contains(&name.as_str()) && source[after_command..].starts_with('*')
            {
                let (required, _, finish, malformed) = command_arguments(source, after_command + 1);
                let end = if malformed || required.is_empty() {
                    after_command + 1
                } else {
                    finish
                };
                tokens.push(Token::Raw(source[position..end].to_owned()));
                position = end;
                continue;
            }

            let (required, optional, finish, malformed) = command_arguments(source, after_command);
            if malformed {
                tokens.push(Token::Raw(source[position..].to_owned()));
                return;
            }
            let argument = required.first();
            if name == "begin" {
                if let Some(environment) = argument {
                    if let Some(end) = matching_environment_end(source, environment.trim(), finish)
                    {
                        tokens.push(Token::Raw(source[position..end].to_owned()));
                        position = end;
                    } else {
                        tokens.push(Token::Raw(source[position..].to_owned()));
                        return;
                    }
                    continue;
                }
            }
            if let Some(level) = HEADING_COMMANDS.iter().position(|heading| *heading == name) {
                if optional.is_empty() {
                    if let Some(content) = argument {
                        let mut heading_marks = marks.to_vec();
                        heading_marks.push(format!("heading{}", level + 1));
                        parse_inline(content, &heading_marks, tokens);
                        tokens.push(Token::Paragraph("\n\n".to_owned()));
                        position = finish;
                        continue;
                    }
                }
            }
            if let Some((_, mark)) = INLINE_COMMANDS.iter().find(|(command, _)| *command == name) {
                if optional.is_empty() {
                    if let Some(content) = argument {
                        let mut nested_marks = marks.to_vec();
                        nested_marks.push((*mark).to_owned());
                        parse_inline(content, &nested_marks, tokens);
                        position = finish;
                        continue;
                    }
                }
            }
            if [
                "cite",
                "parencite",
                "textcite",
                "autocite",
                "footcite",
                "citeauthor",
                "citeyear",
                "nocite",
            ]
            .contains(&name.as_str())
            {
                if !required.is_empty() {
                    tokens.push(Token::Raw(source[position..finish].to_owned()));
                    position = finish;
                    continue;
                }
            }
            if finish > after_command {
                tokens.push(Token::Raw(source[position..finish].to_owned()));
                position = finish;
            } else {
                tokens.push(Token::Raw(source[position..after_command].to_owned()));
                position = after_command;
            }
            continue;
        }

        let next = source[position..]
            .char_indices()
            .find(|(_, character)| matches!(character, '\\' | '$' | '%'))
            .map(|(offset, _)| position + offset)
            .unwrap_or(source.len());
        let plain = &source[position..next];
        let mut chunk_start = 0;
        let mut iterator = plain.match_indices("\n\n").peekable();
        while let Some((offset, delimiter)) = iterator.next() {
            if offset > chunk_start {
                push_text(tokens, &plain[chunk_start..offset], marks);
            }
            let mut end = offset + delimiter.len();
            while end < plain.len() && plain.as_bytes()[end] == b'\n' {
                end += 1;
            }
            tokens.push(Token::Paragraph(plain[offset..end].to_owned()));
            chunk_start = end;
        }
        if chunk_start < plain.len() {
            push_text(tokens, &plain[chunk_start..], marks);
        }
        position = next;
    }
}

pub fn parse_visual_body(source: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    parse_inline(source, &[], &mut tokens);
    tokens
}

pub fn tokens_plain_text(tokens: &[Token]) -> String {
    tokens
        .iter()
        .map(|token| match token {
            Token::Text { text, .. } | Token::Raw(text) | Token::Paragraph(text) => text.as_str(),
        })
        .collect()
}

fn escape_latex_text(source: &str) -> String {
    let mut output = String::with_capacity(source.len());
    for ch in source.chars() {
        match ch {
            '#' | '$' | '%' | '&' | '_' | '{' | '}' => {
                output.push('\\');
                output.push(ch);
            }
            '~' => output.push_str("\\textasciitilde{}"),
            '^' => output.push_str("\\textasciicircum{}"),
            _ => output.push(ch),
        }
    }
    output
}

pub fn serialize_visual_tokens(tokens: &[Token]) -> String {
    let mut output = String::new();
    let mut index = 0;
    while index < tokens.len() {
        match &tokens[index] {
            Token::Raw(text) | Token::Paragraph(text) => {
                output.push_str(text);
                index += 1;
            }
            Token::Text { marks, .. } => {
                let mut text = String::new();
                while index < tokens.len() {
                    match &tokens[index] {
                        Token::Text {
                            text: chunk,
                            marks: current,
                        } if current == marks => {
                            text.push_str(&escape_latex_text(chunk));
                            index += 1;
                        }
                        _ => break,
                    }
                }
                let heading = marks.iter().find(|mark| mark.starts_with("heading"));
                let mut formats = marks
                    .iter()
                    .filter(|mark| !mark.starts_with("heading"))
                    .collect::<Vec<_>>();
                formats.reverse();
                for format in formats {
                    let command = match format.as_str() {
                        "bold" => "textbf",
                        "italic" => "emph",
                        "underline" => "underline",
                        "monospace" => "texttt",
                        _ => continue,
                    };
                    text = format!("\\{command}{{{text}}}");
                }
                if let Some(heading) = heading {
                    let command = match heading.as_str() {
                        "heading1" => "section",
                        "heading2" => "subsection",
                        "heading3" => "subsubsection",
                        "heading4" => "paragraph",
                        "heading5" => "subparagraph",
                        _ => "",
                    };
                    if !command.is_empty() {
                        text = format!("\\{command}{{{text}}}");
                    }
                }
                output.push_str(&text);
            }
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_structure_and_unrecognized_latex() {
        let source = "\\documentclass{article}\n\\begin{document}\\section{Intro}Hi \\cite{x}.\\end{document}\n";
        let document = split_document(source);
        assert!(document.valid);
        let tokens = parse_visual_body(&document.body);
        assert!(tokens
            .iter()
            .any(|token| matches!(token, Token::Raw(raw) if raw == "\\cite{x}")));
        assert_eq!(
            serialize_visual_tokens(&tokens),
            "\\section{Intro}\n\nHi \\cite{x}."
        );
    }

    #[test]
    fn serializes_plain_text_with_latex_escaping() {
        let tokens = vec![Token::Text {
            text: "R&D".to_owned(),
            marks: vec!["bold".to_owned()],
        }];
        assert_eq!(serialize_visual_tokens(&tokens), "\\textbf{R\\&D}");
    }
}
