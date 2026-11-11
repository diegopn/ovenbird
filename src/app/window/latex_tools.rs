use super::*;

pub(super) fn choose_pdf_export(state: Rc<State>) {
    let dialog = gtk::FileChooserNative::new(
        Some(&tr("Export PDF")),
        Some(&state.window),
        gtk::FileChooserAction::Save,
        Some(&tr("Export")),
        Some(&tr("Cancel")),
    );
    set_default_file_dialog_folder(&dialog);
    dialog.set_current_name("document.pdf");
    let state2 = state.clone();
    dialog.connect_response(move |dialog, response| {
        if response == gtk::ResponseType::Accept {
            if let Some(path) = dialog.file().and_then(|file| file.path()) {
                compile(&state2, Some(path));
            }
        }
        dialog.destroy();
    });
    dialog.show();
}

pub(super) fn compile(state: &Rc<State>, export_to: Option<PathBuf>) {
    if state.editor.dirty() {
        if let Some(path) = state.project.current_file.borrow().clone() {
            let export_after_save = export_to.clone();
            let after: SaveCallback =
                Rc::new(move |state| compile(&state, export_after_save.clone()));
            save_document_to_path(state.clone(), path, Some(after));
        } else {
            show_toast(state, &tr("Save the document before compiling."));
        }
        return;
    }
    let main = state.project.main_tex.borrow().clone().or_else(|| {
        state.project.current_file.borrow().clone().filter(|path| {
            path.extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("tex"))
        })
    });
    let Some(main) = main else {
        if let Some(root) = state.project.project_root.borrow().clone() {
            let (sender, receiver) = mpsc::channel();
            let root_for_worker = root.clone();
            std::thread::spawn(move || {
                let result =
                    project::find_tex_files(&root_for_worker).map_err(|error| error.to_string());
                let _ = sender.send(result);
            });
            let state_for_result = state.clone();
            poll_result(receiver, move |result| match result {
                Ok(files) => {
                    if let Some(main) = files.first() {
                        *state_for_result.project.main_tex.borrow_mut() = Some(main.clone());
                        compile(&state_for_result, export_to);
                    } else {
                        show_no_main_document(&state_for_result);
                    }
                }
                Err(error) => {
                    set_build_status(&state_for_result, "Error");
                    show_toast(
                        &state_for_result,
                        &format!("{}: {error}", tr("Could not list the project folder")),
                    );
                }
            });
        } else {
            show_no_main_document(state);
        }
        return;
    };
    state.project.main_tex.replace(Some(main.clone()));
    set_build_status(state, "Building…");
    let active_source = if state.project.current_file.borrow().as_deref() == Some(main.as_path()) {
        Some(state.editor.text())
    } else {
        None
    };
    let (sender, receiver) = mpsc::channel();
    let export_in_worker = export_to.clone();
    std::thread::spawn(move || {
        let source = match active_source {
            Some(source) => Ok(source),
            None => std::fs::read_to_string(&main).map_err(|error| error.to_string()),
        };
        let result = match source {
            Ok(source) => match crate::build::find_missing_citations(&main, &source) {
                Ok(missing) if !missing.is_empty() => {
                    Err(CompileFailure::MissingCitations(missing))
                }
                Ok(_) => crate::build::compile_latex(&main).map_err(CompileFailure::Other),
                Err(error) => Err(CompileFailure::Other(error)),
            },
            Err(error) => Err(CompileFailure::Other(error)),
        };
        let export_result = match (&result, export_in_worker) {
            (Ok(build), Some(destination)) => Some(
                std::fs::copy(&build.pdf_path, destination)
                    .map(|_| ())
                    .map_err(|error| error.to_string()),
            ),
            _ => None,
        };
        let _ = sender.send((result, export_result));
    });
    let state = state.clone();
    let disconnected_state = state.clone();
    poll_result_with_disconnect(
        receiver,
        move |(result, export_result)| match result {
            Ok(build) => {
                if export_to.is_some() {
                    match export_result.unwrap_or_else(|| Err(tr("Could not export PDF"))) {
                        Ok(()) => {
                            set_build_status(&state, "Built");
                            show_toast(&state, &tr("PDF exported"));
                        }
                        Err(error) => {
                            set_build_status(&state, "Error");
                            show_toast(&state, &format!("{}: {error}", tr("Could not export PDF")));
                        }
                    }
                } else {
                    match state.pdf_preview.open(&build.pdf_path) {
                        Ok(()) => {
                            set_build_status(&state, "Built");
                            show_toast(&state, &tr("Compilation finished"));
                        }
                        Err(_) => {
                            let message = tr("Could not open the compiled PDF");
                            set_build_status(&state, "Error");
                            state.pdf_preview.set_message(&message);
                            show_toast(&state, &message);
                        }
                    }
                }
            }
            Err(CompileFailure::MissingCitations(citations)) => {
                let summary = missing_citations_summary(&citations);
                report_compile_failure(&state, &summary);
                show_missing_citations(&state, &citations);
            }
            Err(CompileFailure::Other(error)) => {
                let summary = compile_failure_summary(&error);
                report_compile_failure(&state, &summary);
            }
        },
        move || {
            let message = tr("PDF generation failed: Compilation stopped unexpectedly.");
            report_compile_failure(&disconnected_state, &message);
        },
    );
}

pub(super) fn report_compile_failure(state: &Rc<State>, message: &str) {
    set_build_failure(state, message);
    state.pdf_preview.set_message(message);
    show_toast(state, message);
}

pub(super) fn missing_citations_summary(citations: &[crate::build::MissingCitation]) -> String {
    format!(
        "{}: {}",
        tr("PDF generation failed"),
        missing_citations_context(citations)
    )
}

pub(super) fn missing_citations_context(citations: &[crate::build::MissingCitation]) -> String {
    const MAX_VISIBLE_LINES: usize = 8;
    let mut lines = Vec::new();
    for citation in citations {
        if !lines.contains(&citation.line) {
            lines.push(citation.line);
        }
    }
    if lines.len() == 1 {
        tr("Citation on line %d has no bibliographic reference in the document.").replacen(
            "%d",
            &lines[0].to_string(),
            1,
        )
    } else {
        let line_numbers = lines
            .iter()
            .take(MAX_VISIBLE_LINES)
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        let mut context = tr(
            "Citations on lines %s have no bibliographic references in the document.",
        )
        .replacen("%s", &line_numbers, 1);
        if lines.len() > MAX_VISIBLE_LINES {
            context.push(' ');
            context.push_str(&tr("%d more lines have missing citations.").replacen(
                "%d",
                &(lines.len() - MAX_VISIBLE_LINES).to_string(),
                1,
            ));
        }
        context
    }
}

pub(super) fn show_missing_citations(
    state: &Rc<State>,
    citations: &[crate::build::MissingCitation],
) {
    let dialog = gtk::Dialog::builder()
        .title(tr("PDF generation failed"))
        .transient_for(&state.window)
        .modal(true)
        .default_width(520)
        .default_height(380)
        .build();
    dialog.add_button(&tr("Close"), gtk::ResponseType::Close);

    let content = gtk::Box::new(gtk::Orientation::Vertical, 10);
    content.set_margin_top(12);
    content.set_margin_bottom(12);
    content.set_margin_start(12);
    content.set_margin_end(12);

    let explanation = gtk::Label::new(Some(&missing_citations_context(citations)));
    explanation.set_xalign(0.0);
    explanation.set_wrap(true);
    content.append(&explanation);

    const MAX_VISIBLE_CITATIONS: usize = 10;
    const MAX_CITATION_KEY_CHARS: usize = 80;
    let mut citation_lines = citations
        .iter()
        .take(MAX_VISIBLE_CITATIONS)
        .map(|citation| {
            let mut key_chars = citation.key.chars();
            let mut key = key_chars
                .by_ref()
                .take(MAX_CITATION_KEY_CHARS)
                .collect::<String>();
            if key_chars.next().is_some() {
                key.push('…');
            }
            tr("Line %d: %s")
                .replacen("%d", &citation.line.to_string(), 1)
                .replacen("%s", &key, 1)
        })
        .collect::<Vec<_>>();
    if citations.len() > MAX_VISIBLE_CITATIONS {
        citation_lines.push(
            tr("%d additional citation problems are not shown.").replacen(
                "%d",
                &(citations.len() - MAX_VISIBLE_CITATIONS).to_string(),
                1,
            ),
        );
    }
    let missing_keys = gtk::Label::new(Some(&citation_lines.join("\n")));
    missing_keys.set_xalign(0.0);
    missing_keys.set_yalign(0.0);
    missing_keys.set_selectable(true);
    missing_keys.add_css_class("monospace");
    missing_keys.set_margin_top(6);
    missing_keys.set_margin_bottom(6);
    missing_keys.set_margin_start(8);
    missing_keys.set_margin_end(8);
    let scroll = gtk::ScrolledWindow::builder()
        .hexpand(true)
        .vexpand(true)
        .min_content_height(100)
        .child(&missing_keys)
        .build();
    content.append(&scroll);
    dialog.content_area().append(&content);

    dialog.connect_response(|dialog, _| dialog.close());
    dialog.present();
}

pub(super) fn show_no_main_document(state: &Rc<State>) {
    let message = tr("Open or create a LaTeX project before compiling.");
    set_build_status(state, "Ready");
    show_toast(state, &message);
}

pub(super) fn missing_image_summary(message: &str) -> Option<String> {
    let marker = "Unable to load picture or PDF file '";
    let (image_filename, line_number) = message.lines().find_map(|line| {
        let position = line.find(marker)?;
        let filename = line[position + marker.len()..].split('\'').next()?.trim();
        if filename.is_empty() {
            return None;
        }
        let line_number = line[..position]
            .trim()
            .trim_end_matches(':')
            .rsplit(':')
            .next()
            .and_then(|part| part.trim().parse::<usize>().ok());
        Some((filename.to_owned(), line_number))
    })?;
    let line_number = line_number.or_else(|| {
        message.lines().find_map(|line| {
            let (before, after) = line.split_once(" not found on input line ")?;
            let warning_filename = before
                .rsplit_once("File ")?
                .1
                .trim()
                .trim_matches(|character| matches!(character, '`' | '\'' | '"'));
            if warning_filename != image_filename {
                return None;
            }
            after
                .trim_start()
                .split(|character: char| !character.is_ascii_digit())
                .next()?
                .parse::<usize>()
                .ok()
        })
    })?;

    Some(
        tr("PDF generation failed: Image on line %d was not found.").replacen(
            "%d",
            &line_number.to_string(),
            1,
        ),
    )
}

pub(super) fn compile_error_summary(message: &str) -> Option<String> {
    let line = crate::build::compile_error_line(message)?;
    Some(
        tr("PDF generation failed: An error on line %d prevents compilation and PDF generation.")
            .replacen("%d", &line.to_string(), 1),
    )
}

pub(super) fn compile_failure_summary(message: &str) -> String {
    if message.starts_with("Compilation stopped after ") {
        return tr("PDF generation failed: Compilation timed out.");
    }

    if let Some(summary) = missing_image_summary(message) {
        return summary;
    }
    if let Some(summary) = compile_error_summary(message) {
        return summary;
    }

    for known_message in [
        "No LaTeX compiler was found. Install latexmk, Tectonic, or TeX Live.",
        "This document uses biblatex and needs Biber. Install Biber, latexmk, or Tectonic.",
        "This document needs BibTeX. Install BibTeX, latexmk, or Tectonic.",
    ] {
        let localized = tr(known_message);
        if message == localized {
            return localized;
        }
    }

    tr("PDF generation failed: The document contains errors that prevent compilation.")
}
