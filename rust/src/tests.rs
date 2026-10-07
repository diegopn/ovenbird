use crate::application_menu::APPLICATION_MENU_GROUPS;
use crate::bibtex::*;
use crate::commands::*;
use crate::history::{EditorHistory, EditorMode, EditorState};
use crate::latex::*;
use crate::project::*;
use crate::search::find_document_match;
use crate::templates::{safe_archive_listing, valid_project_name};
use indexmap::IndexMap;
use std::collections::HashSet;
use std::time::{Duration, Instant};

fn test_build_path(name: &str) -> std::path::PathBuf {
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(std::path::PathBuf::from)
        .map(|path| {
            if path.is_absolute() {
                path
            } else {
                manifest.join(path)
            }
        });
    let build_root = target
        .and_then(|path| path.parent().map(std::path::Path::to_path_buf))
        .unwrap_or_else(|| manifest.join("rust-build"));
    build_root
        .join("test-tmp")
        .join(format!("{name}-{}", std::process::id()))
}

#[test]
fn bibtex_retains_macros_raw_values_and_directives() {
    let source = "@string{venue = {Revista de Pesquisa}}\n@article{key,\n title = {Texto {aninhado}},\n journal = venue,\n year = 2024\n}\n";
    let parsed = parse_bibtex(source).unwrap();
    assert_eq!(parsed.entries[0].get("title"), "Texto {aninhado}");
    assert_eq!(parsed.entries[0].get("journal"), "Revista de Pesquisa");
    let serialized = serialize_bibtex(&parsed);
    assert!(serialized.contains("@string{venue = {Revista de Pesquisa}}"));
    assert!(serialized.contains("journal = venue"));
}

#[test]
fn reference_fields_match_type_and_keep_unknown_fields() {
    let mut values = IndexMap::new();
    values.insert("date".into(), "2026-10-06".into());
    values.insert("title".into(), " Título ".into());
    let mut existing = IndexMap::new();
    existing.insert("abstract".into(), "Resumo".into());
    existing.insert("journal".into(), "Periódico".into());
    let fields = build_reference_fields("book", &values, &existing);
    assert_eq!(fields.get("year").map(String::as_str), Some("2026"));
    assert_eq!(fields.get("abstract").map(String::as_str), Some("Resumo"));
    assert!(!fields.contains_key("journal"));
    assert!(fields_for_reference_type("article").contains(&"journal"));
    assert!(!fields_for_reference_type("article").contains(&"booktitle"));
    assert!(fields_for_reference_type("book").contains(&"publisher"));
    assert!(fields_for_reference_type("book").contains(&"edition"));
    assert!(!fields_for_reference_type("book").contains(&"journal"));
    assert!(fields_for_reference_type("incollection").contains(&"booktitle"));
    assert!(fields_for_reference_type("incollection").contains(&"chapter"));
    assert!(fields_for_reference_type("inproceedings").contains(&"booktitle"));
    assert!(fields_for_reference_type("phdthesis").contains(&"school"));
    assert!(fields_for_reference_type("mastersthesis").contains(&"school"));
    assert!(fields_for_reference_type("techreport").contains(&"institution"));
    assert!(fields_for_reference_type("online").contains(&"urldate"));
    assert!(fields_for_reference_type("online").contains(&"url"));
    assert!(fields_for_reference_type("misc").contains(&"howpublished"));

    let mut conference_values = IndexMap::new();
    conference_values.insert("title".into(), "A Conference Paper".into());
    conference_values.insert("date".into(), "2024-05-01".into());
    conference_values.insert("journal".into(), "Old journal value".into());
    conference_values.insert("booktitle".into(), "Proceedings of the Conference".into());
    let mut old_fields = IndexMap::new();
    old_fields.insert("journal".into(), "Old journal value".into());
    old_fields.insert("customfield".into(), "keep me".into());
    let saved = build_reference_fields("inproceedings", &conference_values, &old_fields);
    assert_eq!(
        saved.get("booktitle").map(String::as_str),
        Some("Proceedings of the Conference")
    );
    assert!(!saved.contains_key("journal"));
    assert_eq!(saved.get("year").map(String::as_str), Some("2024"));
    assert_eq!(
        saved.get("customfield").map(String::as_str),
        Some("keep me")
    );
}

#[test]
fn build_cache_is_stable_and_separate_for_each_source() {
    use crate::build::build_output_directory;
    use std::path::Path;

    let first = build_output_directory(Path::new("/home/example/project/main.tex"));
    let same = build_output_directory(Path::new("/home/example/project/main.tex"));
    let other = build_output_directory(Path::new("/home/example/other/main.tex"));
    assert_eq!(first, same);
    assert_ne!(first, other);
    assert!(first.starts_with(crate::storage::xdg_cache_home().join("ovenbird/build")));
}

#[test]
fn atomic_document_writes_replace_the_saved_contents() {
    use crate::storage::atomic_write;
    use std::fs;

    let root = test_build_path("atomic-write-test");
    fs::create_dir_all(&root).unwrap();
    let file = root.join("main.tex");
    fs::write(&file, "previous contents").unwrap();

    atomic_write(&file, b"updated contents").unwrap();

    assert_eq!(fs::read_to_string(&file).unwrap(), "updated contents");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn application_menu_keeps_project_lifecycle_without_export_pdf() {
    let groups = APPLICATION_MENU_GROUPS
        .iter()
        .map(|group| group.iter().map(|item| item.action).collect::<Vec<_>>())
        .collect::<Vec<_>>();
    assert_eq!(
        groups,
        vec![
            vec!["win.new-project", "win.open-project"],
            vec!["win.open-document", "win.close-project"],
            vec!["win.shortcuts", "win.quit"],
        ]
    );
    assert!(!groups
        .iter()
        .flatten()
        .any(|action| *action == "win.export-pdf"));
}

#[test]
fn project_bibliography_merges_local_entries_without_overwriting_project_keys() {
    use crate::build::prepare_project_bibliography;
    use std::fs;

    let project = test_build_path("bibliography-test");
    fs::create_dir_all(&project).unwrap();
    let existing = "@string{venue = {Project journal}}\n@article{shared, title={Project title}, journal=venue}\n";
    fs::write(project.join("references.bib"), existing).unwrap();
    let mut shared = BibEntry::new("article", "shared");
    shared.set("title", "Local conflicting title");
    let mut local = BibEntry::new("article", "local-only");
    local.set("title", "New reference");
    local.set("zotero_key", "SHOULD_NOT_BE_EXPORTED");
    let library = Bibliography {
        entries: vec![shared, local],
        directives: vec!["@string{organization = {Local organization}}".to_owned()],
    };

    let prepared =
        prepare_project_bibliography("\\addbibresource{references}", &project, &library).unwrap();
    assert_eq!(prepared.added, 1);
    assert_eq!(prepared.conflicts, 1);
    assert_eq!(prepared.files.len(), 1);
    let merged = parse_bibtex(&prepared.files[0].content).unwrap();
    assert_eq!(merged.entries.len(), 2);
    assert_eq!(merged.entries[0].get("title"), "Project title");
    assert_eq!(merged.entries[0].get("journal"), "Project journal");
    assert_eq!(merged.entries[1].key, "local-only");
    assert_eq!(merged.entries[1].get("zotero_key"), "");
    assert!(merged
        .directives
        .iter()
        .any(|directive| directive.contains("organization")));
    fs::remove_dir_all(project).unwrap();
}

#[test]
fn citation_key_uses_author_year_title_and_collision_suffix() {
    let mut fields = IndexMap::new();
    fields.insert("author".into(), "Sobrenome, Nome and Outra, Pessoa".into());
    fields.insert("date".into(), "2024-05-01".into());
    fields.insert("title".into(), "Ação e Pesquisa".into());
    assert_eq!(
        create_citation_key(&fields, &HashSet::new()),
        "Sobrenome2024Acao"
    );
    assert_eq!(
        create_citation_key(&fields, &HashSet::from(["sobrenome2024acao".into()])),
        "Sobrenome2024Acao2"
    );
}

#[test]
fn visual_transform_preserves_unknown_commands_and_round_trips() {
    let source = "\\documentclass{article}\n\\begin {document}\\section{Introdução}Texto \\cite{chave}.\\end{document}\n";
    let document = split_document(source);
    assert!(document.valid);
    let tokens = parse_visual_body(&document.body);
    assert!(tokens
        .iter()
        .any(|token| matches!(token, Token::Raw(raw) if raw == "\\cite{chave}")));
    assert_eq!(
        serialize_visual_tokens(&tokens),
        "\\section{Introdução}\n\nTexto \\cite{chave}."
    );
}

#[test]
fn visual_parser_keeps_the_document_ending_and_supported_heading_levels() {
    let source = "\\documentclass{article}\n\\begin{document}\n\\paragraph{Contexto}\nTexto.\n\\end{document}\n";
    let document = split_document(source);
    assert!(document.valid);
    assert_eq!(document.ending, "\\end{document}\n");
    let tokens = parse_visual_body(&document.body);
    assert!(tokens.iter().any(
        |token| matches!(token, Token::Text { marks, .. } if marks.contains(&"heading4".to_owned()))
    ));
    assert!(serialize_visual_tokens(&tokens).contains("\\paragraph{Contexto}"));
}

#[test]
fn search_uses_unicode_character_offsets_and_wraps() {
    let text = "Introdução e INTRODUÇÃO";
    assert_eq!(
        find_document_match(text, "introdução", 10, true)
            .unwrap()
            .start,
        13
    );
    assert_eq!(
        find_document_match(text, "introdução", 23, true)
            .unwrap()
            .start,
        0
    );
    assert_eq!(
        find_document_match("😀 texto TEXTO", "texto", 7, true)
            .unwrap()
            .start,
        8
    );
    assert_eq!(
        find_document_match("A seção {Intro}", "seção {Intro}", 0, true)
            .unwrap()
            .start,
        2
    );
}

#[test]
fn history_coalesces_typing_but_keeps_separate_actions() {
    let start = Instant::now();
    let initial = EditorState {
        source: "a".into(),
        source_cursor: 1,
        visual_cursor: 1,
        mode: EditorMode::Code,
    };
    let mut history = EditorHistory::new(initial.clone());
    let mut next = initial.clone();
    next.source.push('b');
    history.record(
        next.clone(),
        Some("typing"),
        start + Duration::from_millis(100),
    );
    next.source.push('c');
    history.record(
        next.clone(),
        Some("typing"),
        start + Duration::from_millis(300),
    );
    assert!(history.can_undo());
    assert_eq!(history.undo().unwrap(), &initial);
    assert_eq!(history.redo().unwrap(), &next);
    let mut toolbar = next.clone();
    toolbar.source.push_str("\\textbf{}");
    history.record(
        toolbar.clone(),
        Some("toolbar"),
        start + Duration::from_millis(350),
    );
    assert_eq!(history.undo().unwrap(), &next);
}

#[test]
fn template_path_and_project_name_checks_reject_traversal() {
    assert!(valid_project_name("Pesquisa com acentos"));
    assert!(!valid_project_name("paper/../fora"));
    assert!(safe_archive_listing("main.tex\nsamples/example.tex\n"));
    assert!(!safe_archive_listing("folder/../../outside.tex\n"));
    assert!(!safe_archive_listing("/absolute.tex\n"));
}

#[test]
fn project_file_support_includes_text_and_preview_formats() {
    assert!(openable_extension(std::path::Path::new("main.tex")));
    assert!(openable_extension(std::path::Path::new("references.bib")));
    assert!(openable_extension(std::path::Path::new("custom.sty")));
    assert!(openable_extension(std::path::Path::new("figure.pdf")));
    assert!(openable_extension(std::path::Path::new("figure.png")));
    assert!(!openable_extension(std::path::Path::new("document.docx")));
}

#[test]
fn project_tree_lists_resources_and_editable_files_recursively() {
    use std::fs;

    let root = test_build_path("project-tree-test");
    fs::create_dir_all(root.join("figures")).unwrap();
    fs::write(root.join("main.tex"), "\\documentclass{article}").unwrap();
    fs::write(root.join("figures/chart.png"), b"image").unwrap();
    fs::write(root.join(".hidden"), "hidden").unwrap();

    let entries = entries(&root).unwrap();

    assert!(entries.iter().any(
        |entry| entry.path == root.join("main.tex") && entry.kind == ProjectEntryKind::Editable
    ));
    assert!(entries
        .iter()
        .any(|entry| entry.path == root.join("figures/chart.png")
            && entry.kind == ProjectEntryKind::Previewable
            && entry.depth == 1));
    assert!(!entries.iter().any(|entry| entry.name == ".hidden"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn project_file_can_be_moved_into_a_project_subfolder() {
    use std::fs;

    let root = test_build_path("project-move-test");
    let folder = root.join("figures");
    fs::create_dir_all(&folder).unwrap();
    let source = root.join("chart.png");
    fs::write(&source, b"image").unwrap();

    let moved = move_file_within_project(&source, &folder, &root).unwrap();

    assert_eq!(moved, folder.join("chart.png"));
    assert!(!source.exists());
    assert_eq!(fs::read(moved).unwrap(), b"image");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn project_file_move_rejects_destinations_outside_project() {
    use std::fs;

    let root = test_build_path("project-move-boundary-test");
    let outside = root.with_extension("outside");
    fs::create_dir_all(&root).unwrap();
    fs::create_dir_all(&outside).unwrap();
    let source = root.join("main.tex");
    fs::write(&source, "document").unwrap();

    let result = move_file_within_project(&source, &outside, &root);

    assert!(result.is_err());
    assert!(source.exists());
    assert!(!outside.join("main.tex").exists());
    fs::remove_dir_all(root).unwrap();
    fs::remove_dir_all(outside).unwrap();
}

#[test]
fn project_folder_can_be_moved_and_cannot_be_dropped_inside_itself() {
    use std::fs;

    let root = test_build_path("project-folder-move-test");
    let source = root.join("chapters");
    let nested = source.join("nested");
    let destination = root.join("archive");
    fs::create_dir_all(&nested).unwrap();
    fs::create_dir_all(&destination).unwrap();
    fs::write(nested.join("intro.tex"), "document").unwrap();

    let invalid = move_file_within_project(&source, &nested, &root);
    assert!(invalid.is_err());
    assert!(source.exists());

    let moved = move_file_within_project(&source, &destination, &root).unwrap();
    assert_eq!(moved, destination.join("chapters"));
    assert!(moved.join("nested/intro.tex").is_file());
    assert!(!source.exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn project_file_can_be_renamed_without_leaving_its_folder() {
    use std::fs;

    let root = test_build_path("project-rename-test");
    fs::create_dir_all(&root).unwrap();
    let source = root.join("draft.tex");
    fs::write(&source, "document").unwrap();

    let renamed = rename_project_file(&source, "final.tex", &root).unwrap();

    assert_eq!(renamed, root.join("final.tex"));
    assert!(!source.exists());
    assert_eq!(fs::read_to_string(&renamed).unwrap(), "document");
    assert!(rename_project_file(&renamed, "../outside.tex", &root).is_err());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn folder_rename_rewrites_open_file_paths_below_that_folder() {
    let old_folder = std::path::Path::new("/project/chapters");
    let new_folder = std::path::Path::new("/project/sections");
    let open_file = std::path::Path::new("/project/chapters/intro/main.tex");

    assert_eq!(
        rewrite_path_prefix(open_file, old_folder, new_folder),
        std::path::PathBuf::from("/project/sections/intro/main.tex"),
    );
}

#[test]
fn deleting_a_folder_detects_an_open_descendant() {
    assert!(path_contains(
        std::path::Path::new("/project/chapters"),
        std::path::Path::new("/project/chapters/intro/main.tex"),
    ));
    assert!(!path_contains(
        std::path::Path::new("/project/chapters"),
        std::path::Path::new("/project/appendix/main.tex"),
    ));
}

#[test]
fn image_reference_is_relative_to_the_main_latex_file_directory() {
    assert_eq!(
        relative_path(
            std::path::Path::new("/project/chapters"),
            std::path::Path::new("/project/images/diagram.png"),
        ),
        Some(std::path::PathBuf::from("../images/diagram.png")),
    );
    assert_eq!(
        relative_path(
            std::path::Path::new("/project/chapters"),
            std::path::Path::new("/project/chapters/plot.png"),
        ),
        Some(std::path::PathBuf::from("plot.png")),
    );
}

#[test]
fn inline_latex_commands_return_character_offsets_for_unicode_text() {
    let command = create_inline_command("bold", "ação").unwrap();
    assert_eq!(command.text, "\\textbf{ação}");
    assert_eq!(command.cursor_offset, 12);
    assert_eq!(command.selection, Some((8, 12)));
}

#[test]
fn normal_heading_removes_only_a_supported_heading_wrapper() {
    let heading = create_heading_command("normal", "\\section{Introdução}").unwrap();
    assert_eq!(heading.text, "Introdução");
    assert_eq!(heading.selection, Some((0, 10)));
    assert_eq!(
        create_heading_command("subparagraph", "Detalhes")
            .unwrap()
            .text,
        "\\subparagraph{Detalhes}"
    );
}

#[test]
fn list_and_math_snippets_place_the_cursor_at_the_editing_point() {
    let list = create_list_snippet("itemize", "").unwrap();
    assert_eq!(list.text, "\\begin{itemize}\n\\item \n\\end{itemize}");
    assert_eq!(
        list.cursor_offset,
        "\\begin{itemize}\n\\item ".chars().count()
    );

    let math = create_math_snippet(true, "");
    assert_eq!(math.text, "\\[  \\]");
    assert_eq!(math.cursor_offset, 3);
}

#[test]
fn table_snippets_respect_supported_dimensions() {
    let table = create_table_snippet(0, 99);
    assert!(table
        .text
        .starts_with("\\begin{tabular}{|l|l|l|l|l|l|l|l|l|l|l|l|}"));
    let rows = table
        .text
        .lines()
        .filter(|line| line.ends_with("\\\\"))
        .count();
    assert_eq!(rows, 1);
    assert_eq!(
        table.cursor_offset,
        "\\begin{tabular}{|l|l|l|l|l|l|l|l|l|l|l|l|}\n\\hline\n"
            .chars()
            .count()
    );
}
