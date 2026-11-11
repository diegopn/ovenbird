// shortcut: The existing dialogs and portal-backed file choosers use GTK 4
// compatibility APIs; remove this exception when those dialogs are migrated.
#![allow(deprecated)]

use adw::prelude::*;
use gtk::gio;
use gtk::{gdk, glib};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::mpsc;
use std::time::Duration;

use crate::application_menu::ThemePreference;
use crate::bibtex::{self, BibEntry, REFERENCE_TYPES};
use crate::commands::{self, LatexInsertion};
use crate::editor::LatexEditor;
use crate::history::EditorMode;
use crate::i18n::ngettext;
use crate::project::{self, ProjectEntryKind, ProjectFileKind};
use crate::reference_exchange::{self, TransferAction, TransferDirection};
use crate::reference_guides::REFERENCE_MANAGER_GUIDES;
use crate::storage::{AppSettings, AuthorProfile, LocalLibrary, StartupView};

const MAIN_HEADER_HEIGHT: i32 = 48;

pub struct OvenbirdWindow {
    window: adw::ApplicationWindow,
}

struct State {
    window: adw::ApplicationWindow,
    editor: Rc<LatexEditor>,
    pdf_preview: crate::pdf_preview::PdfPreview,
    file_pdf_preview: crate::pdf_preview::PdfPreview,
    file_preview_stack: gtk::Stack,
    image_preview: gtk::Picture,
    library: Rc<RefCell<LocalLibrary>>,
    library_ready: Cell<bool>,
    library_loading: Cell<bool>,
    library_busy: Cell<bool>,
    library_load_error: RefCell<Option<String>>,
    project_root: RefCell<Option<PathBuf>>,
    project_target: RefCell<Option<PathBuf>>,
    current_file: RefCell<Option<PathBuf>>,
    file_encoding: Cell<project::TextEncoding>,
    main_tex: RefCell<Option<PathBuf>>,
    expanded_folders: RefCell<HashSet<PathBuf>>,
    open_request_id: Cell<u64>,
    project_scan_id: Cell<u64>,
    project_open_id: Cell<u64>,
    save_in_progress: Cell<bool>,
    close_confirmed: Cell<bool>,
    startup_restore_cancelled: Cell<bool>,
    startup_view: StartupView,
    file_list: gtk::Box,
    file_tree: gtk::ListView,
    file_tree_selection: gtk::SingleSelection,
    project_files_scroll: gtk::Adjustment,
    library_list: gtk::ListView,
    library_list_model: gio::ListStore,
    tags_list: gtk::Box,
    tag_search: gtk::SearchEntry,
    tag_sort: gtk::DropDown,
    tag_sort_descending: gtk::ToggleButton,
    authors_list: gtk::Box,
    library_search: gtk::SearchEntry,
    reference_sort: gtk::DropDown,
    reference_sort_descending: gtk::ToggleButton,
    reference_filters: ReferenceFilterWidgets,
    author_search: gtk::SearchEntry,
    author_sort: gtk::DropDown,
    author_sort_descending: gtk::ToggleButton,
    library_filters_updating: Cell<bool>,
    page_stack: gtk::Stack,
    sidebar_pages: gtk::Stack,
    editor_tab_button: gtk::ToggleButton,
    references_tab_button: gtk::ToggleButton,
    search_panel: gtk::Box,
    search_entry: gtk::SearchEntry,
    search_result: gtk::Label,
    mode_toolbar: gtk::Box,
    editor_toolbar: gtk::ScrolledWindow,
    latex_toolbar: gtk::Box,
    undo_button: gtk::Button,
    redo_button: gtk::Button,
    code_mode_button: gtk::ToggleButton,
    visual_mode_button: gtk::ToggleButton,
    find_document_button: gtk::Button,
    bibtex_citation_button: gtk::Button,
    title: gtk::Label,
    status: gtk::Label,
    reference_summary: RefCell<Option<gtk::Label>>,
    reference_summary_group: RefCell<Option<gtk::Box>>,
    build_status_widgets: RefCell<Option<BuildStatusWidgets>>,
    omni_bar: RefCell<Option<super::libpanel::OmniBar>>,
    compile_action: RefCell<Option<gio::SimpleAction>>,
    build_state: RefCell<String>,
    build_failure_message: RefCell<Option<String>>,
    last_build_at: RefCell<String>,
    save_button: RefCell<Option<gtk::Button>>,
    toast: adw::ToastOverlay,
    layout: gtk::Paned,
    sidebar_revealer: gtk::Revealer,
    sidebar_width: Cell<i32>,
    toolbar_view: adw::ToolbarView,
}

type SaveCallback = Rc<dyn Fn(Rc<State>)>;

enum CompileFailure {
    MissingCitations(Vec<crate::build::MissingCitation>),
    Other(String),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ReferencePickerTarget {
    Latex,
    Bibtex,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LatexInsertChoice {
    Citation,
    Bibliography,
}

struct BuildStatusWidgets {
    status_row: gtk::Box,
    icon: gtk::Image,
    details_popover: gtk::Popover,
    details_file: gtk::Label,
    details_last: gtk::Label,
    details_failure: gtk::Label,
}

struct SidebarWidgets {
    root: gtk::Box,
    #[cfg(test)]
    brand: gtk::CenterBox,
    file_list: gtk::Box,
    file_tree: gtk::ListView,
    file_tree_selection: gtk::SingleSelection,
    project_files_scroll: gtk::Adjustment,
    pages: gtk::Stack,
    editor_tab: gtk::ToggleButton,
    references_tab: gtk::ToggleButton,
}

struct ReferenceFilterWidgets {
    author: gtk::DropDown,
    year: gtk::DropDown,
    tag: gtk::DropDown,
    kind: gtk::DropDown,
}

struct ReferenceFilterOptions {
    authors: Vec<String>,
    years: Vec<String>,
    tags: Vec<String>,
    kinds: Vec<String>,
}

struct ReferenceFilterSelection<'a> {
    author: Option<&'a str>,
    year: Option<&'a str>,
    tag: Option<&'a str>,
    kind: Option<&'a str>,
    author_profiles: &'a [AuthorProfile],
}

struct LibraryPageWidgets {
    page: gtk::Box,
    list: gtk::ListView,
    list_model: gio::ListStore,
    search: gtk::SearchEntry,
    sort_by: gtk::DropDown,
    sort_descending: gtk::ToggleButton,
    filters: ReferenceFilterWidgets,
}

struct AuthorsPageWidgets {
    page: gtk::Box,
    list: gtk::Box,
    add: gtk::Button,
    search: gtk::SearchEntry,
    sort_by: gtk::DropDown,
    sort_descending: gtk::ToggleButton,
}

struct TagsPageWidgets {
    page: gtk::Box,
    list: gtk::Box,
    add: gtk::Button,
    search: gtk::SearchEntry,
    sort_by: gtk::DropDown,
    sort_descending: gtk::ToggleButton,
}

#[derive(Clone)]
enum ReferenceListItem {
    Entry(BibEntry),
    Message { text: String, retry: bool },
}

#[derive(Clone)]
struct ProjectTreeNode {
    entry: project::ProjectEntry,
    children: Option<gio::ListStore>,
}

impl OvenbirdWindow {
    pub fn new(application: &adw::Application) -> Self {
        let window = adw::ApplicationWindow::builder()
            .application(application)
            .title("Ovenbird")
            .default_width(1440)
            .default_height(900)
            .decorated(true)
            .resizable(true)
            .build();
        let theme_preference = load_theme_preference();
        let startup_view = load_startup_view_preference();
        apply_theme_preference(theme_preference);
        let editor = Rc::new(LatexEditor::new());
        let library = LocalLibrary::empty();
        let toast = adw::ToastOverlay::new();
        let layout = gtk::Paned::new(gtk::Orientation::Horizontal);
        layout.set_resize_start_child(false);
        layout.set_resize_end_child(true);
        layout.set_shrink_start_child(true);
        layout.set_shrink_end_child(true);
        layout.set_position(340);
        let (
            editor_panel,
            editor_page,
            pdf_preview,
            file_pdf_preview,
            file_preview_stack,
            image_preview,
            mode_toolbar,
            editor_toolbar,
            latex_toolbar,
            undo_button,
            redo_button,
            code_mode_button,
            visual_mode_button,
            find_document_button,
            bibtex_citation_button,
        ) = build_editor_page(&editor);
        let library_page = build_library_page();
        let tags_page = build_tags_manager_content();
        let authors_page = build_authors_manager_content();
        let sidebar_widgets = build_sidebar(startup_view);
        let page_stack = gtk::Stack::builder()
            .hexpand(true)
            .vexpand(true)
            .transition_type(gtk::StackTransitionType::Crossfade)
            .build();
        page_stack.add_named(&editor_page, Some("editor"));
        page_stack.add_named(&library_page.page, Some("references"));
        page_stack.add_named(&tags_page.page, Some("tags"));
        page_stack.add_named(&authors_page.page, Some("authors"));
        page_stack.set_visible_child_name(startup_page_name(startup_view));
        let toolbar_view = adw::ToolbarView::new();
        toolbar_view.set_hexpand(true);
        toolbar_view.set_vexpand(true);
        toolbar_view.set_content(Some(&page_stack));
        let sidebar_revealer = gtk::Revealer::builder()
            .transition_type(gtk::RevealerTransitionType::SlideRight)
            .transition_duration(220)
            .reveal_child(true)
            .child(&sidebar_widgets.root)
            .build();
        layout.set_start_child(Some(&sidebar_revealer));
        layout.set_end_child(Some(&toolbar_view));
        layout.set_hexpand(true);
        layout.set_vexpand(true);
        toast.set_child(Some(&layout));
        window.set_content(Some(&toast));

        let title = gtk::Label::new(Some(&tr(match startup_view {
            StartupView::Editor => "LaTeX editor",
            StartupView::References => "References",
        })));
        title.add_css_class("ovenbird-header-title");
        let status = gtk::Label::new(Some(&crate::i18n::gettext("Ready")));
        status.add_css_class("ovenbird-header-status");
        let state = Rc::new(State {
            window: window.clone(),
            editor,
            pdf_preview,
            file_pdf_preview,
            file_preview_stack,
            image_preview,
            library: Rc::new(RefCell::new(library)),
            library_ready: Cell::new(false),
            library_loading: Cell::new(false),
            library_busy: Cell::new(false),
            library_load_error: RefCell::new(None),
            project_root: RefCell::new(None),
            project_target: RefCell::new(None),
            current_file: RefCell::new(None),
            file_encoding: Cell::new(project::TextEncoding::Utf8),
            main_tex: RefCell::new(None),
            expanded_folders: RefCell::new(HashSet::new()),
            open_request_id: Cell::new(0),
            project_scan_id: Cell::new(0),
            project_open_id: Cell::new(0),
            save_in_progress: Cell::new(false),
            close_confirmed: Cell::new(false),
            startup_restore_cancelled: Cell::new(false),
            startup_view,
            file_list: sidebar_widgets.file_list,
            file_tree: sidebar_widgets.file_tree,
            file_tree_selection: sidebar_widgets.file_tree_selection,
            project_files_scroll: sidebar_widgets.project_files_scroll,
            library_search: library_page.search,
            reference_sort: library_page.sort_by,
            reference_sort_descending: library_page.sort_descending,
            library_list: library_page.list,
            library_list_model: library_page.list_model,
            tags_list: tags_page.list,
            tag_search: tags_page.search,
            tag_sort: tags_page.sort_by,
            tag_sort_descending: tags_page.sort_descending,
            authors_list: authors_page.list,
            reference_filters: library_page.filters,
            author_search: authors_page.search,
            author_sort: authors_page.sort_by,
            author_sort_descending: authors_page.sort_descending,
            library_filters_updating: Cell::new(false),
            page_stack,
            sidebar_pages: sidebar_widgets.pages,
            editor_tab_button: sidebar_widgets.editor_tab,
            references_tab_button: sidebar_widgets.references_tab,
            search_panel: find_search_panel(&editor_panel),
            search_entry: find_search_entry(&editor_panel),
            search_result: find_search_result(&editor_panel),
            mode_toolbar,
            editor_toolbar,
            latex_toolbar,
            undo_button,
            redo_button,
            code_mode_button,
            visual_mode_button,
            find_document_button,
            bibtex_citation_button,
            title,
            status,
            reference_summary: RefCell::new(None),
            reference_summary_group: RefCell::new(None),
            build_status_widgets: RefCell::new(None),
            omni_bar: RefCell::new(None),
            compile_action: RefCell::new(None),
            build_state: RefCell::new("Ready".to_owned()),
            build_failure_message: RefCell::new(None),
            last_build_at: RefCell::new(tr("Not built yet")),
            save_button: RefCell::new(None),
            toast,
            layout: layout.clone(),
            sidebar_revealer,
            sidebar_width: Cell::new(340),
            toolbar_view,
        });
        let state_for_add_tag = state.clone();
        tags_page
            .add
            .connect_clicked(move |_| edit_tag(&state_for_add_tag, None));
        let state_for_add_author = state.clone();
        authors_page
            .add
            .connect_clicked(move |_| edit_author(&state_for_add_author, None));
        setup_reference_list_view(&state);
        setup_project_file_view(&state);
        build_header(&state);
        connect_navigation(&state);
        connect_actions(&state);
        connect_window_close_guard(&state);
        install_root_drop_target(&state);
        configure_editor_callbacks(&state);
        refresh_project_files(&state);
        refresh_library(&state);
        load_library_async(&state);
        schedule_last_project_restore(&state);
        Self { window }
    }

    pub fn present(&self) {
        self.window.present();
    }
}

mod actions;
mod dialogs;
mod documents;
mod editor;
mod latex_tools;
mod preferences;
mod presentation;
mod project_files;
mod references;

use actions::*;
use dialogs::*;
use documents::*;
use editor::*;
use latex_tools::*;
use preferences::*;
use presentation::*;
use project_files::*;
use references::*;

fn tr(message: &str) -> String {
    crate::i18n::gettext(message)
}

fn tr_dynamic(message: &str) -> String {
    match message {
        "New project" => tr("New project"),
        "Open document" => tr("Open document"),
        "Open project" => tr("Open project"),
        "Open project folder" => tr("Open project folder"),
        "Close project" => tr("Close project"),
        "Quit" => tr("Quit"),
        "Automatic" => tr("Automatic"),
        "Light" => tr("Light"),
        "Dark" => tr("Dark"),
        "Export PDF" => tr("Export PDF"),
        "Keyboard shortcuts" => tr("Keyboard shortcuts"),
        "About Ovenbird" => tr("About Ovenbird"),
        "Dedicated to my wife, Karina, who always encourages me to go further." => {
            tr("Dedicated to my wife, Karina, who always encourages me to go further.")
        }
        "Report an issue" => tr("Report an issue"),
        "Project repository" => tr("Project repository"),
        "Project website" => tr("Project website"),
        "Project creator's website" => tr("Project creator's website"),
        "New LaTeX document" => tr("New LaTeX document"),
        "New bibliography (.bib)" => tr("New bibliography (.bib)"),
        "New style file (.sty)" => tr("New style file (.sty)"),
        "Previous match" => tr("Previous match"),
        "Next match" => tr("Next match"),
        "New reference" => tr("New reference"),
        "Edit reference" => tr("Edit reference"),
        "Add" => tr("Add"),
        "Add a reference to the local library" => tr("Add a reference to the local library"),
        "Bold" => tr("Bold"),
        "Italic" => tr("Italic"),
        "Underline" => tr("Underline"),
        "Monospace" => tr("Monospace"),
        "Section" => tr("Section"),
        "Subsection" => tr("Subsection"),
        "Subsubsection" => tr("Subsubsection"),
        "Paragraph" => tr("Paragraph"),
        "Subparagraph" => tr("Subparagraph"),
        "Inline math" => tr("Inline math"),
        "Display math" => tr("Display math"),
        "α  Alpha" => tr("α  Alpha"),
        "β  Beta" => tr("β  Beta"),
        "γ  Gamma" => tr("γ  Gamma"),
        "π  Pi" => tr("π  Pi"),
        "≤  Less than or equal to" => tr("≤  Less than or equal to"),
        "≥  Greater than or equal to" => tr("≥  Greater than or equal to"),
        "≠  Not equal to" => tr("≠  Not equal to"),
        "×  Multiplication" => tr("×  Multiplication"),
        "±  Plus or minus" => tr("±  Plus or minus"),
        "∞  Infinity" => tr("∞  Infinity"),
        "∑  Summation" => tr("∑  Summation"),
        "∫  Integral" => tr("∫  Integral"),
        "√  Square root" => tr("√  Square root"),
        "Bulleted list" => tr("Bulleted list"),
        "Numbered list" => tr("Numbered list"),
        "Quotation block" => tr("Quotation block"),
        "Increase indent" => tr("Increase indent"),
        "Decrease indent" => tr("Decrease indent"),
        "Insert image" => tr("Insert image"),
        "Insert table" => tr("Insert table"),
        "LaTeX comment" => tr("LaTeX comment"),
        "Footnote" => tr("Footnote"),
        "Review note" => tr("Review note"),
        "Add label" => tr("Add label"),
        "Cross-reference" => tr("Cross-reference"),
        "Compile" => tr("Compile"),
        "Toggle sidebar" => tr("Toggle sidebar"),
        "Undo" => tr("Undo"),
        "Redo" => tr("Redo"),
        "Choose a project folder" => tr("Choose a project folder"),
        "Choose where to create the project" => tr("Choose where to create the project"),
        "Import" => tr("Import"),
        "Label identifier" => tr("Label identifier"),
        message => tr(message),
    }
}

#[cfg(test)]
mod startup_view_tests {
    use super::{startup_page_name, startup_view_from_menu_index, startup_view_menu_index};
    use crate::storage::StartupView;

    #[test]
    fn startup_view_uses_the_matching_page_and_menu_choice() {
        for (view, page, index) in [
            (StartupView::Editor, "editor", 0),
            (StartupView::References, "references", 1),
        ] {
            assert_eq!(startup_page_name(view), page);
            assert_eq!(startup_view_menu_index(view), index);
            assert_eq!(startup_view_from_menu_index(index), view);
        }
        assert_eq!(startup_view_from_menu_index(u32::MAX), StartupView::Editor);
    }
}

type EditorPageWidgets = (
    gtk::Box,
    gtk::Box,
    crate::pdf_preview::PdfPreview,
    crate::pdf_preview::PdfPreview,
    gtk::Stack,
    gtk::Picture,
    gtk::Box,
    gtk::ScrolledWindow,
    gtk::Box,
    gtk::Button,
    gtk::Button,
    gtk::ToggleButton,
    gtk::ToggleButton,
    gtk::Button,
    gtk::Button,
);

#[cfg(test)]
mod reference_filter_tests {
    use super::{
        filtered_reference_entries, reference_filter_values, reference_matches_filters,
        replace_reference_list_entries, ReferenceFilterSelection, ReferenceListItem,
    };
    use crate::bibtex::BibEntry;
    use gtk::gio::prelude::*;

    #[test]
    fn author_year_and_tag_filters_match_bibtex_entries() {
        let mut entry = BibEntry::new("article", "doe2024");
        entry.set("author", "Doe, Jane and Smith, John");
        entry.set("date", "2024-06-12");
        entry.set("keywords", "Writing, science; Typography");

        assert!(reference_matches_filters(
            &entry,
            Some("smith, john"),
            Some("2024"),
            Some("SCIENCE")
        ));
        assert!(!reference_matches_filters(
            &entry,
            Some("Roe, Jane"),
            None,
            None
        ));
        assert!(!reference_matches_filters(&entry, None, Some("2023"), None));
        assert!(!reference_matches_filters(
            &entry,
            None,
            None,
            Some("history")
        ));
    }

    #[test]
    fn filter_choices_are_sorted_and_unique() {
        let mut first = BibEntry::new("article", "doe2024");
        first.set("author", "Zed, A and Ann, B");
        first.set("date", "2024-06-12");
        first.set("keywords", "science, writing");

        let mut second = BibEntry::new("book", "ann2023");
        second.set("author", "Ann, B");
        second.set("year", "2023");
        second.set("keywords", "writing; history");

        let filters = reference_filter_values(&[first, second]);
        assert_eq!(filters.authors, ["Ann, B", "Zed, A"]);
        assert_eq!(filters.years, ["2023", "2024"]);
        assert_eq!(filters.tags, ["history", "science", "writing"]);
    }

    #[test]
    fn reference_search_filters_and_sorting_keep_the_same_matching_entries() {
        let mut matching = BibEntry::new("article", "doe2024search");
        matching.set("title", "Search systems in science");
        matching.set("author", "Doe, Jane");
        matching.set("year", "2024");
        matching.set("keywords", "science, writing");

        let mut matching_for_computing = BibEntry::new("article", "doe2024computing");
        matching_for_computing.set("title", "Search systems for computing");
        matching_for_computing.set("author", "Doe, Jane");
        matching_for_computing.set("year", "2024");
        matching_for_computing.set("keywords", "science, writing");

        let mut wrong_author = BibEntry::new("article", "roe2024search");
        wrong_author.set("title", "Search networks");
        wrong_author.set("author", "Roe, Jane");
        wrong_author.set("year", "2024");
        wrong_author.set("keywords", "science, writing");

        let mut wrong_year = BibEntry::new("article", "doe2023search");
        wrong_year.set("title", "Search systems");
        wrong_year.set("author", "Doe, Jane");
        wrong_year.set("year", "2023");
        wrong_year.set("keywords", "science, writing");

        let mut wrong_tag = BibEntry::new("article", "doe2024other");
        wrong_tag.set("title", "Search systems in science");
        wrong_tag.set("author", "Doe, Jane");
        wrong_tag.set("year", "2024");
        wrong_tag.set("keywords", "science");

        let filters = ReferenceFilterSelection {
            author: Some("Doe, Jane"),
            year: Some("2024"),
            tag: Some("writing"),
            kind: None,
            author_profiles: &[],
        };
        let actual = filtered_reference_entries(
            &[
                wrong_year,
                matching,
                wrong_tag,
                matching_for_computing,
                wrong_author,
            ],
            "search",
            &filters,
            0,
            false,
        );

        assert_eq!(
            actual
                .iter()
                .map(|entry| entry.key.as_str())
                .collect::<Vec<_>>(),
            ["doe2024computing", "doe2024search"]
        );
    }

    #[test]
    fn reference_list_model_keeps_all_ten_thousand_entries_in_order() {
        let model = gtk::gio::ListStore::new::<gtk::glib::BoxedAnyObject>();
        let change_count = std::rc::Rc::new(std::cell::Cell::new(0));
        let change_count_for_signal = change_count.clone();
        model.connect_items_changed(move |_, _, _, _| {
            change_count_for_signal.set(change_count_for_signal.get() + 1);
        });
        let entries = (0..10_000)
            .map(|index| BibEntry::new("article", format!("ref-{index:05}")))
            .collect();

        replace_reference_list_entries(&model, entries);

        assert_eq!(model.n_items(), 10_000);
        assert_eq!(change_count.get(), 1);
        for (index, expected_key) in [(0, "ref-00000"), (9_999, "ref-09999")] {
            let item = model
                .item(index)
                .and_then(|item| item.downcast::<gtk::glib::BoxedAnyObject>().ok())
                .unwrap();
            let row = item.borrow::<ReferenceListItem>();
            let ReferenceListItem::Entry(entry) = &*row else {
                panic!("expected a reference model row");
            };
            assert_eq!(entry.key, expected_key);
        }
    }
}

#[cfg(test)]
mod file_dialog_folder_tests {
    use super::preferred_file_dialog_folder;
    use gtk::gio::prelude::*;
    use std::path::PathBuf;

    fn temporary_folder(label: &str) -> PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("ovenbird-{label}-{}-{nonce}", std::process::id()));
        std::fs::create_dir(&path).unwrap();
        path
    }

    #[test]
    fn configured_file_dialog_folder_takes_precedence_when_it_exists() {
        let configured = temporary_folder("configured-folder");
        let downloads = temporary_folder("downloads-folder");
        let configured_uri = gtk::gio::File::for_path(&configured).uri();

        assert_eq!(
            preferred_file_dialog_folder(Some(configured_uri.as_str()), Some(downloads.clone())),
            Some(configured.clone())
        );

        std::fs::remove_dir_all(configured).unwrap();
        std::fs::remove_dir_all(downloads).unwrap();
    }

    #[test]
    fn missing_configured_file_dialog_folder_falls_back_to_downloads() {
        let downloads = temporary_folder("fallback-downloads");
        let missing = downloads.join("no-longer-exists");
        let missing_uri = gtk::gio::File::for_path(&missing).uri();

        assert_eq!(
            preferred_file_dialog_folder(Some(missing_uri.as_str()), Some(downloads.clone())),
            Some(downloads.clone())
        );
        assert_eq!(preferred_file_dialog_folder(None, None), None);

        std::fs::remove_dir_all(downloads).unwrap();
    }
}

fn poll_result<T: Send + 'static>(receiver: mpsc::Receiver<T>, callback: impl FnOnce(T) + 'static) {
    poll_result_with_disconnect(receiver, callback, || {});
}

fn poll_result_with_disconnect<T: Send + 'static>(
    receiver: mpsc::Receiver<T>,
    callback: impl FnOnce(T) + 'static,
    on_disconnect: impl FnOnce() + 'static,
) {
    let callback = Rc::new(RefCell::new(Some(callback)));
    let on_disconnect = Rc::new(RefCell::new(Some(on_disconnect)));
    glib::timeout_add_local(Duration::from_millis(120), move || {
        match receiver.try_recv() {
            Ok(result) => {
                if let Some(callback) = callback.borrow_mut().take() {
                    callback(result);
                }
                glib::ControlFlow::Break
            }
            Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(mpsc::TryRecvError::Disconnected) => {
                let on_disconnect = on_disconnect.borrow_mut().take();
                if let Some(on_disconnect) = on_disconnect {
                    on_disconnect();
                }
                glib::ControlFlow::Break
            }
        }
    });
}

#[cfg(test)]
mod sidebar_tests {
    use super::set_sidebar_visible;
    use gtk::prelude::*;
    use std::cell::Cell;

    #[test]
    fn main_header_and_sidebar_helpers_keep_widgets_attached() {
        adw::init().expect("GTK must initialize for sidebar tests");

        let pages = gtk::Stack::new();
        pages.add_named(
            &gtk::Box::new(gtk::Orientation::Vertical, 0),
            Some("editor"),
        );
        pages.add_named(
            &gtk::Box::new(gtk::Orientation::Vertical, 0),
            Some("references"),
        );
        pages.set_visible_child_name("references");
        assert!(super::search_is_references_page(&pages));
        pages.set_visible_child_name("editor");
        assert!(!super::search_is_references_page(&pages));

        let toolbar_view = adw::ToolbarView::new();
        let header = adw::HeaderBar::new();
        super::install_main_header(&toolbar_view, &header);
        while gtk::glib::MainContext::default().iteration(false) {}
        assert_eq!(
            header.decoration_layout().as_deref(),
            Some("minimize,maximize,close")
        );
        assert!(
            header.shows_end_title_buttons(),
            "the header must show minimize, maximize, and close controls"
        );
        assert!(
            header.parent().is_some(),
            "the header must be attached to the app toolbar"
        );

        let layout = gtk::Paned::new(gtk::Orientation::Horizontal);
        let sidebar = gtk::Revealer::builder().reveal_child(true).build();
        let sidebar_width = Cell::new(340);
        layout.set_start_child(Some(&sidebar));
        layout.set_position(340);

        set_sidebar_visible(&layout, &sidebar, &sidebar_width, false);

        assert!(
            layout.start_child().is_none(),
            "a hidden sidebar must release its Paned slot"
        );
        assert!(!sidebar.reveals_child());

        set_sidebar_visible(&layout, &sidebar, &sidebar_width, true);

        assert!(layout.start_child().is_some());
        assert!(sidebar.reveals_child());

        let editor = crate::editor::LatexEditor::new();
        let (editor_panel, editor_page, ..) = super::build_editor_page(&editor);
        let sidebar = super::build_sidebar(super::StartupView::Editor);

        assert!(
            sidebar.brand.parent().is_some(),
            "the brand should be part of the full-height sidebar"
        );

        assert!(
            sidebar.file_list.parent().is_some(),
            "project file rows must be added to the visible scrolled list"
        );
        assert!(sidebar.editor_tab.is_active());
        assert!(sidebar.pages.visible_child_name().as_deref() == Some("editor"));
        let editor_tab_content = sidebar
            .editor_tab
            .child()
            .and_downcast::<gtk::Box>()
            .expect("the LaTeX tab should pair its icon and label");
        let editor_icon = editor_tab_content
            .first_child()
            .and_downcast::<gtk::Image>()
            .expect("the LaTeX tab should show its editor icon");
        assert_eq!(
            editor_icon.icon_name().as_deref(),
            Some("document-edit-symbolic")
        );
        assert_eq!(
            editor_icon
                .next_sibling()
                .and_downcast::<gtk::Label>()
                .expect("the LaTeX tab should retain its label")
                .text()
                .as_str(),
            "LaTeX editor"
        );
        let references_tab_content = sidebar
            .references_tab
            .child()
            .and_downcast::<gtk::Box>()
            .expect("the References tab should pair its icon and label");
        let references_icon = references_tab_content
            .first_child()
            .and_downcast::<gtk::Image>()
            .expect("the References tab should show its list icon");
        assert_eq!(
            references_icon.icon_name().as_deref(),
            Some("view-list-symbolic")
        );
        assert_eq!(
            references_icon
                .next_sibling()
                .and_downcast::<gtk::Label>()
                .expect("the References tab should retain its label")
                .text()
                .as_str(),
            "References"
        );
        assert!(sidebar.editor_tab.hexpands());
        assert!(sidebar.references_tab.hexpands());

        let references_sidebar = super::build_sidebar(super::StartupView::References);
        assert!(references_sidebar.references_tab.is_active());
        assert_eq!(
            references_sidebar.pages.visible_child_name().as_deref(),
            Some("references")
        );

        let mode_toolbar = editor_panel
            .first_child()
            .and_downcast::<gtk::Box>()
            .expect("the mode toolbar should contain document search");
        let find_button = mode_toolbar
            .last_child()
            .and_downcast::<gtk::Button>()
            .expect("the search icon should be beside the mode buttons");
        let find_icon = find_button
            .child()
            .and_downcast::<gtk::Image>()
            .expect("the search button should show a symbolic search icon");
        assert_eq!(
            find_icon.icon_name().as_deref(),
            Some("system-search-symbolic")
        );
        let document_search = super::find_search_panel(&editor_panel);
        assert!(!document_search.is_visible());
        assert_eq!(document_search.width_request(), 0);

        let workspace = editor_page
            .first_child()
            .and_downcast::<gtk::Paned>()
            .expect("the main editor page must split code and PDF horizontally");
        assert_eq!(workspace.orientation(), gtk::Orientation::Horizontal);
        let editor_split = editor_panel
            .parent()
            .and_downcast::<gtk::Paned>()
            .expect("the code editor must be in the main workspace");
        assert_eq!(editor_split.orientation(), gtk::Orientation::Horizontal);

        let page = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let mode_row = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        let search_panel = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        search_panel.append(&gtk::SearchEntry::new());
        search_panel.set_visible(false);
        page.append(&mode_row);
        page.append(&search_panel);
        page.set_visible(false);

        let found_panel = super::find_search_panel(&page);
        assert!(
            found_panel
                .first_child()
                .is_some_and(|child| child.is::<gtk::SearchEntry>()),
            "search must target the hidden search row, even before the page is presented"
        );
    }
}
