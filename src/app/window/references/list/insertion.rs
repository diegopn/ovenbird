use super::*;

pub(in crate::app::window) fn insert_latex_citation(
    state: &Rc<State>,
    entry: &BibEntry,
) -> Result<(), String> {
    let current_file = state
        .project
        .current_file
        .borrow()
        .clone()
        .ok_or_else(|| tr("Save the document before inserting a citation."))?;
    if project::file_kind(&current_file) != ProjectFileKind::Latex {
        return Err(tr("Open a LaTeX document before inserting a citation."));
    }
    let snippet = format!("\\cite{{{}}}", entry.key);
    state
        .editor
        .insert_at_cursor(&snippet, snippet.chars().count(), "insert:citation");
    Ok(())
}

pub(in crate::app::window) fn insert_latex_bibliography_reference(
    state: &Rc<State>,
    entry: &BibEntry,
    style: bibtex::ReferenceStyle,
) -> Result<(), String> {
    let current_file = state
        .project
        .current_file
        .borrow()
        .clone()
        .ok_or_else(|| tr("Save the document before inserting a citation."))?;
    let main_path = state
        .project
        .main_tex
        .borrow()
        .clone()
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("tex"))
        })
        .or_else(|| {
            current_file
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("tex"))
                .then(|| current_file.clone())
        })
        .ok_or_else(|| tr("Open a LaTeX document before inserting a citation."))?;
    let main_is_open = current_file == main_path;
    let (main_source, source_encoding) = if main_is_open {
        (state.editor.text(), state.project.file_encoding.get())
    } else {
        let bytes = std::fs::read(&main_path).map_err(|error| error.to_string())?;
        project::decode_text(&bytes)
    };
    let update = crate::build::prepare_inline_bibliography_update(
        &main_source,
        source_encoding,
        entry,
        style,
    )?;
    match update {
        crate::build::CitationBibliographyUpdate::InlineTex {
            content,
            inserted_text,
            insertion_offset,
            encoding,
        } => {
            project::encode_text(&content, encoding)?;
            if main_is_open {
                state.editor.insert_source_at_offset(
                    &inserted_text,
                    insertion_offset,
                    "insert:citation",
                );
            } else {
                let bytes = project::encode_text(&content, encoding)?;
                crate::storage::atomic_write(&main_path, &bytes)
                    .map_err(|error| error.to_string())?;
                refresh_project_files(state);
            }
        }
        crate::build::CitationBibliographyUpdate::BibFile { .. } => unreachable!(),
        crate::build::CitationBibliographyUpdate::AlreadyPresent => {}
    }
    Ok(())
}

pub(in crate::app::window) fn insert_bibtex_reference(
    state: &Rc<State>,
    entry: &BibEntry,
) -> Result<(), String> {
    let current_file = state
        .project
        .current_file
        .borrow()
        .clone()
        .ok_or_else(|| tr("Open a BibTeX file before adding a bibliographic reference."))?;
    if project::file_kind(&current_file) != ProjectFileKind::Bibtex {
        return Err(tr(
            "Open a BibTeX file before adding a bibliographic reference.",
        ));
    }
    let source = state.editor.text();
    let bibliography = bibtex::parse_bibtex(&source).map_err(|error| error.to_string())?;
    if bibliography
        .entries
        .iter()
        .any(|existing| existing.key.eq_ignore_ascii_case(&entry.key))
    {
        return Err(tr("This reference is already in the BibTeX file."));
    }

    let line_ending = if source.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let serialized = bibtex::serialize_bibtex(&bibtex::Bibliography {
        entries: vec![entry.clone()],
        directives: Vec::new(),
    })
    .replace("\r\n", "\n")
    .replace('\n', line_ending);
    let separator = if source.is_empty() {
        String::new()
    } else if source.ends_with('\n') {
        line_ending.to_owned()
    } else {
        format!("{line_ending}{line_ending}")
    };
    let insertion = format!("{separator}{serialized}");
    let offset = source.chars().count();
    state
        .editor
        .insert_source_at_offset(&insertion, offset, "insert:bibtex-reference");
    Ok(())
}
