#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LatexDiagnostic {
    pub line: usize,
    pub column: usize,
    pub kind: LatexDiagnosticKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LatexDiagnosticKind {
    UnmatchedClosingBrace,
    UnclosedOpeningBrace,
    UnmatchedEnvironmentEnd(String),
    UnclosedEnvironment(String),
    UnmatchedMathDelimiter(String),
    UnclosedMathDelimiter(String),
    NestedMathDelimiter(String),
}

#[derive(Debug)]
struct OpenEnvironment {
    name: String,
    line: usize,
    column: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MathDelimiter {
    InlineDollar,
    DisplayDollar,
    Parentheses,
    Brackets,
}

impl MathDelimiter {
    fn opening_token(self) -> &'static str {
        match self {
            Self::InlineDollar => "$",
            Self::DisplayDollar => "$$",
            Self::Parentheses => r"\(",
            Self::Brackets => r"\[",
        }
    }
}

#[derive(Debug)]
struct OpenMathDelimiter {
    delimiter: MathDelimiter,
    line: usize,
    column: usize,
}

fn open_math_delimiter(
    stack: &mut Vec<OpenMathDelimiter>,
    diagnostics: &mut Vec<LatexDiagnostic>,
    delimiter: MathDelimiter,
    line: usize,
    column: usize,
    inside_math_environment: bool,
) {
    if !stack.is_empty() || inside_math_environment {
        diagnostics.push(LatexDiagnostic {
            line,
            column,
            kind: LatexDiagnosticKind::NestedMathDelimiter(delimiter.opening_token().to_owned()),
        });
    }
    stack.push(OpenMathDelimiter {
        delimiter,
        line,
        column,
    });
}

fn close_math_delimiter(
    stack: &mut Vec<OpenMathDelimiter>,
    diagnostics: &mut Vec<LatexDiagnostic>,
    expected: MathDelimiter,
    token: &str,
    line: usize,
    column: usize,
) {
    if stack.last().is_some_and(|open| open.delimiter == expected) {
        stack.pop();
    } else {
        diagnostics.push(LatexDiagnostic {
            line,
            column,
            kind: LatexDiagnosticKind::UnmatchedMathDelimiter(token.to_owned()),
        });
    }
}

fn control_sequence(chars: &[char], start: usize) -> (String, usize, bool) {
    let mut end = start + 1;
    if end >= chars.len() {
        return (String::new(), end, false);
    }

    if chars[end].is_ascii_alphabetic() {
        let name_start = end;
        while end < chars.len() && chars[end].is_ascii_alphabetic() {
            end += 1;
        }
        (chars[name_start..end].iter().collect(), end, true)
    } else {
        let symbol = chars[end].to_string();
        end += 1;
        (symbol, end, false)
    }
}

fn braced_argument(chars: &[char], mut start: usize) -> Option<(String, usize)> {
    while start < chars.len() && chars[start].is_whitespace() {
        start += 1;
    }
    if chars.get(start) != Some(&'{') {
        return None;
    }

    let content_start = start + 1;
    let mut end = content_start;
    while end < chars.len() && chars[end] != '}' {
        if matches!(chars[end], '{' | '\\' | '#') {
            return None;
        }
        end += 1;
    }
    if end == chars.len() {
        return None;
    }

    let name = chars[content_start..end]
        .iter()
        .collect::<String>()
        .trim()
        .to_owned();
    (!name.is_empty()).then_some((name, end + 1))
}

fn is_verbatim_environment(name: &str) -> bool {
    matches!(
        name,
        "verbatim" | "verbatim*" | "Verbatim" | "Verbatim*" | "lstlisting" | "minted" | "comment"
    )
}

fn is_math_environment(name: &str) -> bool {
    matches!(
        name,
        "math"
            | "displaymath"
            | "equation"
            | "equation*"
            | "align"
            | "align*"
            | "flalign"
            | "flalign*"
            | "gather"
            | "gather*"
            | "multline"
            | "multline*"
            | "alignat"
            | "alignat*"
            | "xalignat"
            | "xalignat*"
            | "xxalignat"
            | "xxalignat*"
            | "split"
            | "aligned"
            | "gathered"
            | "cases"
            | "matrix"
            | "pmatrix"
            | "bmatrix"
            | "Bmatrix"
            | "vmatrix"
            | "Vmatrix"
            | "array"
    )
}

fn inside_math_environment(environments: &[OpenEnvironment]) -> bool {
    environments
        .iter()
        .any(|environment| is_math_environment(&environment.name))
}

fn verbatim_environment_end(chars: &[char], start: usize, name: &str) -> Option<usize> {
    let mut line_start = start;
    while line_start < chars.len() && chars[line_start] != '\n' {
        line_start += 1;
    }
    if line_start < chars.len() {
        line_start += 1;
    }
    while line_start < chars.len() {
        let mut candidate = line_start;
        while matches!(chars.get(candidate), Some(' ' | '\t')) {
            candidate += 1;
        }
        if chars.get(candidate) == Some(&'\\') {
            let (command, command_end, is_word) = control_sequence(chars, candidate);
            if is_word && command == "end" {
                if let Some((closing_name, mut end)) = braced_argument(chars, command_end) {
                    if closing_name == name {
                        while matches!(chars.get(end), Some(' ' | '\t' | '\r')) {
                            end += 1;
                        }
                        if end == chars.len() || chars.get(end) == Some(&'\n') {
                            return Some(end);
                        }
                    }
                }
            }
        }

        while line_start < chars.len() && chars[line_start] != '\n' {
            line_start += 1;
        }
        if line_start < chars.len() {
            line_start += 1;
        }
    }
    None
}

fn advance(
    chars: &[char],
    index: &mut usize,
    line: &mut usize,
    line_start: &mut usize,
    end: usize,
) {
    let skipped = &chars[*index..end];
    *line += skipped
        .iter()
        .filter(|character| **character == '\n')
        .count();
    if let Some(last_newline) = skipped.iter().rposition(|character| *character == '\n') {
        *line_start = *index + last_newline + 1;
    }
    *index = end;
}

/// Checks common structural mistakes without requiring a LaTeX distribution.
/// Commands and environments provided by packages remain the compiler's job.
pub fn check_structure(source: &str) -> Vec<LatexDiagnostic> {
    let chars = source.chars().collect::<Vec<_>>();
    let mut diagnostics = Vec::new();
    let mut brace_positions = Vec::new();
    let mut environments: Vec<OpenEnvironment> = Vec::new();
    let mut math_delimiters: Vec<OpenMathDelimiter> = Vec::new();
    let mut index = 0;
    let mut line = 1;
    let mut line_start = 0;

    while index < chars.len() {
        match chars[index] {
            '\n' => {
                line += 1;
                index += 1;
                line_start = index;
            }
            '%' => {
                while index < chars.len() && chars[index] != '\n' {
                    index += 1;
                }
            }
            '\\' => {
                let command_line = line;
                let command_column = index - line_start + 1;
                let (command, command_end, is_word) = control_sequence(&chars, index);
                if !is_word {
                    match command.as_str() {
                        "(" => open_math_delimiter(
                            &mut math_delimiters,
                            &mut diagnostics,
                            MathDelimiter::Parentheses,
                            command_line,
                            command_column,
                            inside_math_environment(&environments),
                        ),
                        "[" => open_math_delimiter(
                            &mut math_delimiters,
                            &mut diagnostics,
                            MathDelimiter::Brackets,
                            command_line,
                            command_column,
                            inside_math_environment(&environments),
                        ),
                        ")" => close_math_delimiter(
                            &mut math_delimiters,
                            &mut diagnostics,
                            MathDelimiter::Parentheses,
                            r"\)",
                            command_line,
                            command_column,
                        ),
                        "]" => close_math_delimiter(
                            &mut math_delimiters,
                            &mut diagnostics,
                            MathDelimiter::Brackets,
                            r"\]",
                            command_line,
                            command_column,
                        ),
                        _ => {}
                    }
                    advance(&chars, &mut index, &mut line, &mut line_start, command_end);
                    continue;
                }

                if command == "verb" {
                    let mut delimiter_index = command_end;
                    if chars.get(delimiter_index) == Some(&'*') {
                        delimiter_index += 1;
                    }
                    if let Some(delimiter) =
                        chars.get(delimiter_index).copied().filter(|c| *c != '\n')
                    {
                        let mut end = delimiter_index + 1;
                        while end < chars.len() && chars[end] != '\n' && chars[end] != delimiter {
                            end += 1;
                        }
                        if end < chars.len() && chars[end] == delimiter {
                            end += 1;
                        }
                        advance(&chars, &mut index, &mut line, &mut line_start, end);
                    } else {
                        advance(&chars, &mut index, &mut line, &mut line_start, command_end);
                    }
                    continue;
                }

                if command == "begin" || command == "end" {
                    if let Some((name, end)) = braced_argument(&chars, command_end) {
                        if command == "begin" {
                            environments.push(OpenEnvironment {
                                name: name.clone(),
                                line: command_line,
                                column: command_column,
                            });
                            if is_verbatim_environment(&name) {
                                if let Some(verbatim_end) =
                                    verbatim_environment_end(&chars, end, &name)
                                {
                                    if let Some(open) = environments.pop() {
                                        if open.name != name {
                                            environments.push(open);
                                        }
                                    }
                                    advance(
                                        &chars,
                                        &mut index,
                                        &mut line,
                                        &mut line_start,
                                        verbatim_end,
                                    );
                                    continue;
                                }
                                break;
                            }
                        } else if let Some(position) =
                            environments.iter().rposition(|open| open.name == name)
                        {
                            for unclosed in environments[position + 1..].iter().rev() {
                                diagnostics.push(LatexDiagnostic {
                                    line: unclosed.line,
                                    column: unclosed.column,
                                    kind: LatexDiagnosticKind::UnclosedEnvironment(
                                        unclosed.name.clone(),
                                    ),
                                });
                            }
                            environments.truncate(position);
                        } else {
                            diagnostics.push(LatexDiagnostic {
                                line: command_line,
                                column: command_column,
                                kind: LatexDiagnosticKind::UnmatchedEnvironmentEnd(name),
                            });
                        }
                        advance(&chars, &mut index, &mut line, &mut line_start, end);
                        continue;
                    }
                }

                advance(&chars, &mut index, &mut line, &mut line_start, command_end);
            }
            '{' => {
                brace_positions.push((line, index - line_start + 1));
                index += 1;
            }
            '}' => {
                if brace_positions.pop().is_none() {
                    diagnostics.push(LatexDiagnostic {
                        line,
                        column: index - line_start + 1,
                        kind: LatexDiagnosticKind::UnmatchedClosingBrace,
                    });
                }
                index += 1;
            }
            '$' => {
                let (delimiter, width) = if chars.get(index + 1) == Some(&'$') {
                    (MathDelimiter::DisplayDollar, 2)
                } else {
                    (MathDelimiter::InlineDollar, 1)
                };
                if math_delimiters
                    .last()
                    .is_some_and(|open| open.delimiter == delimiter)
                {
                    math_delimiters.pop();
                } else {
                    open_math_delimiter(
                        &mut math_delimiters,
                        &mut diagnostics,
                        delimiter,
                        line,
                        index - line_start + 1,
                        inside_math_environment(&environments),
                    );
                }
                index += width;
            }
            _ => index += 1,
        }
    }

    diagnostics.extend(
        brace_positions
            .into_iter()
            .map(|(line, column)| LatexDiagnostic {
                line,
                column,
                kind: LatexDiagnosticKind::UnclosedOpeningBrace,
            }),
    );
    diagnostics.extend(environments.into_iter().rev().map(|open| LatexDiagnostic {
        line: open.line,
        column: open.column,
        kind: LatexDiagnosticKind::UnclosedEnvironment(open.name),
    }));
    diagnostics.extend(
        math_delimiters
            .into_iter()
            .rev()
            .map(|open| LatexDiagnostic {
                line: open.line,
                column: open.column,
                kind: LatexDiagnosticKind::UnclosedMathDelimiter(
                    open.delimiter.opening_token().to_owned(),
                ),
            }),
    );
    diagnostics.sort_by_key(|diagnostic| (diagnostic.line, diagnostic.column));
    diagnostics
}

#[cfg(test)]
mod tests {
    use super::{check_structure, LatexDiagnosticKind as Kind};

    fn has(source: &str, line: usize, column: usize, expected: Kind) -> bool {
        check_structure(source).iter().any(|diagnostic| {
            diagnostic.line == line && diagnostic.column == column && diagnostic.kind == expected
        })
    }

    #[test]
    fn valid_math_forms_and_common_environments_are_clean() {
        let source = r#"\documentclass{article}
\begin{document}
Inline $x^2 + y^2$ and \(a+b\).
\[
  x = y
\]
$$
  x = y
$$
\begin{equation}
a=b
\end{equation}
\begin{align*}
a &= b \\
c &= d
\end{align*}
\end{document}"#;

        assert!(check_structure(source).is_empty());
    }

    #[test]
    fn brace_and_environment_errors_include_their_columns() {
        assert!(has("first\ntext }\n{", 2, 6, Kind::UnmatchedClosingBrace));
        assert!(has("first\ncafé }", 2, 6, Kind::UnmatchedClosingBrace));
        assert!(has("first\n{", 2, 1, Kind::UnclosedOpeningBrace));

        let source = "\\begin{itemize}\n\\item x\n\\end{document}";
        assert!(has(
            source,
            3,
            1,
            Kind::UnmatchedEnvironmentEnd("document".into())
        ));
        assert!(has(
            source,
            1,
            1,
            Kind::UnclosedEnvironment("itemize".into())
        ));
    }

    #[test]
    fn math_delimiter_errors_include_their_columns() {
        assert!(has(
            "ok\n$x + y",
            2,
            1,
            Kind::UnclosedMathDelimiter("$".into())
        ));
        assert!(has(
            "line 1\n\\]",
            2,
            1,
            Kind::UnmatchedMathDelimiter("\\]".into())
        ));
        assert!(has("$$x", 1, 1, Kind::UnclosedMathDelimiter("$$".into())));
        assert!(has("\\(x", 1, 1, Kind::UnclosedMathDelimiter("\\(".into())));
        assert!(has("\\[x", 1, 1, Kind::UnclosedMathDelimiter("\\[".into())));
        assert!(has(
            "ab \\)",
            1,
            4,
            Kind::UnmatchedMathDelimiter("\\)".into())
        ));
        assert!(has(
            "\\begin{equation}\n  \\(x\\)\n\\end{equation}",
            2,
            3,
            Kind::NestedMathDelimiter("\\(".into())
        ));
    }

    #[test]
    fn comments_escaped_symbols_verb_and_verbatim_are_ignored() {
        let source = concat!(
            r#"\% \{ \} \$"#,
            "\n",
            r#"% $ { \begin{broken} \("#,
            "\n",
            r#"\verb|$ { } \begin{x} % \(|"#,
            "\n",
            r#"\verb*|$$ { } \end{bad}|"#,
            "\n",
            r#"\begin{verbatim}"#,
            "\n",
            r#"$ { } \begin{broken} \("#,
            "\n",
            r#"\end{verbatim}"#,
            "\n",
            r#"\begin{lstlisting}"#,
            "\n",
            r#"$ { } \end{broken}"#,
            "\n",
            r#"\end{lstlisting}"#,
        );

        assert!(check_structure(source).is_empty());

        assert!(has(
            "\\begin{verbatim}\n{ $\n\\end{verbatim}\n}",
            4,
            1,
            Kind::UnmatchedClosingBrace
        ));
    }

    #[test]
    fn escaped_percent_does_not_hide_following_structure() {
        assert!(has("\\% text {\n", 1, 9, Kind::UnclosedOpeningBrace));
    }

    #[test]
    fn corrected_text_clears_its_diagnostics() {
        assert!(!check_structure("\\begin{document}\n$x").is_empty());
        assert!(check_structure("\\begin{document}\n$x$\n\\end{document}").is_empty());
    }
}
