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
use crate::storage::{LocalLibrary, ZoteroSettings};

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
    file_list: gtk::Box,
    file_tree: gtk::ListView,
    file_tree_selection: gtk::SingleSelection,
    project_files_scroll: gtk::Adjustment,
    library_list: gtk::Box,
    library_search: gtk::SearchEntry,
    reference_filters: ReferenceFilterWidgets,
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
    compact_find_document_button: gtk::Button,
    title: gtk::Label,
    status: gtk::Label,
    reference_summary: RefCell<Option<gtk::Label>>,
    reference_summary_group: RefCell<Option<gtk::Box>>,
    build_status_widgets: RefCell<Option<BuildStatusWidgets>>,
    omni_bar: RefCell<Option<crate::libpanel::OmniBar>>,
    compile_action: RefCell<Option<gio::SimpleAction>>,
    build_state: RefCell<String>,
    last_build_at: RefCell<String>,
    last_build_message: RefCell<String>,
    save_button: RefCell<Option<gtk::Button>>,
    toast: adw::ToastOverlay,
    sync_button: gtk::Button,
    layout: gtk::Paned,
    sidebar_revealer: gtk::Revealer,
    sidebar_width: Cell<i32>,
    toolbar_view: adw::ToolbarView,
}

type SaveCallback = Rc<dyn Fn(Rc<State>)>;

struct BuildStatusWidgets {
    status_row: gtk::Box,
    icon: gtk::Image,
    details_popover: gtk::Popover,
    details_file: gtk::Label,
    details_path: gtk::Label,
    details_engine: gtk::Label,
    details_state: gtk::Label,
    details_last: gtk::Label,
    details_output: gtk::Label,
}

struct SidebarWidgets {
    root: gtk::Box,
    brand: gtk::CenterBox,
    file_list: gtk::Box,
    file_tree: gtk::ListView,
    file_tree_selection: gtk::SingleSelection,
    project_files_scroll: gtk::Adjustment,
    library_search: gtk::SearchEntry,
    pages: gtk::Stack,
    editor_tab: gtk::ToggleButton,
    references_tab: gtk::ToggleButton,
}

struct ReferenceFilterWidgets {
    author: gtk::DropDown,
    year: gtk::DropDown,
    tag: gtk::DropDown,
}

struct ReferenceFilterOptions {
    authors: Vec<String>,
    years: Vec<String>,
    tags: Vec<String>,
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
            compact_find_document_button,
        ) = build_editor_page(&editor);
        let library_page = build_library_page();
        let sidebar_widgets = build_sidebar(theme_preference);
        let page_stack = gtk::Stack::builder()
            .hexpand(true)
            .vexpand(true)
            .transition_type(gtk::StackTransitionType::Crossfade)
            .build();
        page_stack.add_named(&editor_page, Some("editor"));
        page_stack.add_named(&library_page.0, Some("references"));
        page_stack.set_visible_child_name("editor");
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

        let title = gtk::Label::new(Some(&tr("LaTeX editor")));
        title.add_css_class("ovenbird-header-title");
        let status = gtk::Label::new(Some(&crate::i18n::gettext("Ready")));
        status.add_css_class("ovenbird-header-status");
        let sync_button = button("network-server-symbolic", &tr("Sync with Zotero"));
        sync_button.set_action_name(Some("win.sync-zotero"));
        sync_button.add_css_class("flat");
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
            file_list: sidebar_widgets.file_list,
            file_tree: sidebar_widgets.file_tree,
            file_tree_selection: sidebar_widgets.file_tree_selection,
            project_files_scroll: sidebar_widgets.project_files_scroll,
            library_search: sidebar_widgets.library_search,
            library_list: library_page.1,
            reference_filters: library_page.2,
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
            compact_find_document_button,
            title,
            status,
            reference_summary: RefCell::new(None),
            reference_summary_group: RefCell::new(None),
            build_status_widgets: RefCell::new(None),
            omni_bar: RefCell::new(None),
            compile_action: RefCell::new(None),
            build_state: RefCell::new("Ready".to_owned()),
            last_build_at: RefCell::new(tr("Not built yet")),
            last_build_message: RefCell::new(tr("This document has not been compiled yet.")),
            save_button: RefCell::new(None),
            toast,
            sync_button,
            layout: layout.clone(),
            sidebar_revealer,
            sidebar_width: Cell::new(340),
            toolbar_view,
        });
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
        Self { window }
    }

    pub fn present(&self) {
        self.window.present();
    }
}

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

fn button(icon: &str, tooltip: &str) -> gtk::Button {
    let button = gtk::Button::from_icon_name(icon);
    button.set_tooltip_text(Some(tooltip));
    button
}

fn labeled_button(icon: &str, label: &str) -> gtk::Button {
    let button = gtk::Button::new();
    button.set_child(Some(&labeled_content(icon, label)));
    button
}

fn sidebar_tab_button(icon: &str, label: &str) -> gtk::ToggleButton {
    let label = tr(label);
    let button = gtk::ToggleButton::new();
    let content = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    content.set_halign(gtk::Align::Center);
    content.append(&gtk::Image::from_icon_name(icon));
    content.append(&gtk::Label::new(Some(&label)));
    button.set_child(Some(&content));
    button.set_tooltip_text(Some(&label));
    button
}

fn editor_mode_button(icon: &str, label: &str) -> gtk::ToggleButton {
    let label = tr(label);
    let button = gtk::ToggleButton::new();
    let content = gtk::Box::new(gtk::Orientation::Horizontal, 5);
    let image = gtk::Image::from_icon_name(icon);
    image.set_pixel_size(14);
    content.append(&image);
    content.append(&gtk::Label::new(Some(&label)));
    button.set_child(Some(&content));
    button.set_tooltip_text(Some(&label));
    button
}

fn labeled_content(icon: &str, label: &str) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    if !icon.is_empty() {
        row.append(&gtk::Image::from_icon_name(icon));
    }
    row.append(&gtk::Label::new(Some(label)));
    row
}

fn build_sidebar(theme_preference: ThemePreference) -> SidebarWidgets {
    let sidebar = gtk::Box::new(gtk::Orientation::Vertical, 0);
    sidebar.set_size_request(340, -1);
    sidebar.set_hexpand(false);
    sidebar.set_vexpand(true);
    sidebar.add_css_class("ovenbird-sidebar");

    let brand = gtk::CenterBox::new();
    brand.set_margin_start(8);
    brand.set_margin_end(8);
    brand.set_size_request(-1, MAIN_HEADER_HEIGHT);
    brand.set_hexpand(true);
    brand.set_valign(gtk::Align::Center);
    brand.add_css_class("ovenbird-sidebar-brand");
    let name = gtk::Label::new(Some("Ovenbird"));
    name.set_hexpand(true);
    name.add_css_class("ovenbird-brand");
    name.set_justify(gtk::Justification::Center);
    name.set_halign(gtk::Align::Center);
    let menu = gtk::MenuButton::new();
    menu.set_icon_name("open-menu-symbolic");
    menu.set_tooltip_text(Some(&tr("Main menu")));
    menu.set_popover(Some(&build_main_menu(theme_preference)));
    brand.set_center_widget(Some(&name));
    brand.set_end_widget(Some(&menu));
    sidebar.append(&brand);
    let tabs = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    tabs.set_margin_start(8);
    tabs.set_margin_end(8);
    tabs.set_margin_bottom(8);
    tabs.set_homogeneous(true);
    tabs.add_css_class("ovenbird-sidebar-tabs");
    let editor_tab = sidebar_tab_button("document-edit-symbolic", "LaTeX editor");
    editor_tab.set_action_name(Some("win.show-editor"));
    editor_tab.set_active(true);
    editor_tab.set_hexpand(true);
    editor_tab.set_halign(gtk::Align::Fill);
    editor_tab.add_css_class("flat");
    let references_tab = sidebar_tab_button("view-list-symbolic", "References");
    references_tab.set_group(Some(&editor_tab));
    references_tab.set_action_name(Some("win.show-references"));
    references_tab.set_hexpand(true);
    references_tab.set_halign(gtk::Align::Fill);
    references_tab.add_css_class("flat");
    tabs.append(&editor_tab);
    tabs.append(&references_tab);
    sidebar.append(&tabs);

    let pages = gtk::Stack::builder()
        .hexpand(true)
        .vexpand(true)
        .transition_type(gtk::StackTransitionType::Crossfade)
        .build();

    let files_panel = gtk::Box::new(gtk::Orientation::Vertical, 2);
    files_panel.set_hexpand(true);
    files_panel.set_vexpand(true);
    let files_header = gtk::Box::new(gtk::Orientation::Horizontal, 2);
    files_header.set_margin_start(12);
    files_header.set_margin_end(8);
    let files_title = gtk::Label::new(Some(&tr("Project files")));
    files_title.set_xalign(0.0);
    files_title.set_hexpand(true);
    files_title.add_css_class("dim-label");
    files_title.add_css_class("sidebar-section");
    files_header.append(&files_title);
    let new_item = gtk::MenuButton::new();
    new_item.set_icon_name("list-add-symbolic");
    new_item.set_tooltip_text(Some(&tr("Create files and folders")));
    new_item.set_popover(Some(&build_new_item_menu()));
    files_header.append(&new_item);
    files_panel.append(&files_header);
    let file_list = gtk::Box::new(gtk::Orientation::Vertical, 1);
    file_list.set_hexpand(true);
    file_list.set_vexpand(true);
    file_list.add_css_class("ovenbird-project-drop-target");
    let file_tree_selection = gtk::SingleSelection::new(None::<gio::ListModel>);
    file_tree_selection.set_autoselect(false);
    file_tree_selection.set_can_unselect(true);
    let file_tree = gtk::ListView::new(
        Some(file_tree_selection.clone()),
        None::<gtk::ListItemFactory>,
    );
    file_tree.set_single_click_activate(true);
    file_tree.add_css_class("navigation-sidebar");
    file_list.append(&file_tree);
    let file_scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Automatic)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .min_content_width(0)
        .max_content_width(340)
        .min_content_height(70)
        .propagate_natural_width(false)
        .hexpand(true)
        .vexpand(true)
        .child(&file_list)
        .build();
    let project_files_scroll = file_scroll.vadjustment();
    files_panel.append(&file_scroll);

    pages.add_named(&files_panel, Some("editor"));

    let references_panel = gtk::Box::new(gtk::Orientation::Vertical, 8);
    references_panel.set_margin_top(10);
    references_panel.set_margin_start(12);
    references_panel.set_margin_end(12);
    references_panel.set_hexpand(true);
    references_panel.set_vexpand(true);
    let references_title = gtk::Label::new(Some(&tr("Local library")));
    references_title.set_xalign(0.0);
    references_title.add_css_class("sidebar-section");
    references_title.add_css_class("dim-label");
    references_panel.append(&references_title);
    let library_search = gtk::SearchEntry::new();
    library_search.set_placeholder_text(Some(&tr("Search references…")));
    library_search.set_hexpand(true);
    references_panel.append(&library_search);
    for (icon, label, action) in [
        ("list-add-symbolic", "Add reference", "win.add-reference"),
        (
            "document-open-symbolic",
            "Import BibTeX",
            "win.import-bibtex",
        ),
        (
            "document-save-symbolic",
            "Export BibTeX",
            "win.export-bibtex",
        ),
    ] {
        let item = labeled_button(icon, &tr(label));
        item.set_action_name(Some(action));
        item.set_halign(gtk::Align::Fill);
        item.set_hexpand(true);
        references_panel.append(&item);
    }
    pages.add_named(&references_panel, Some("references"));
    pages.set_visible_child_name("editor");
    sidebar.append(&pages);

    SidebarWidgets {
        root: sidebar,
        brand,
        file_list,
        file_tree,
        file_tree_selection,
        project_files_scroll,
        library_search,
        pages,
        editor_tab,
        references_tab,
    }
}

fn build_main_menu(theme_preference: ThemePreference) -> gtk::Popover {
    let popover = gtk::Popover::new();
    let content = gtk::Box::new(gtk::Orientation::Vertical, 2);
    content.set_margin_top(6);
    content.set_margin_bottom(6);
    content.set_margin_start(6);
    content.set_margin_end(6);
    content.append(&build_theme_choices(theme_preference));
    content.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    let append_item = |icon: &str, label: &str, action: &str, shortcut: &str| {
        let item = gtk::Button::new();
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        row.append(&gtk::Image::from_icon_name(icon));
        let label = tr_dynamic(label);
        let title = gtk::Label::new(Some(&label));
        title.set_xalign(0.0);
        title.set_hexpand(true);
        row.append(&title);
        if !shortcut.is_empty() {
            let accelerator = gtk::Label::new(Some(shortcut));
            accelerator.add_css_class("dim-label");
            row.append(&accelerator);
        }
        item.set_child(Some(&row));
        item.set_action_name(Some(action));
        item.set_halign(gtk::Align::Fill);
        item.add_css_class("flat");
        item.add_css_class("ovenbird-app-menu-item");
        content.append(&item);
    };
    for (group_index, group) in crate::application_menu::APPLICATION_MENU_GROUPS
        .iter()
        .enumerate()
    {
        if group_index > 0 {
            content.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
        }
        for item in *group {
            append_item(item.icon, item.label, item.action, item.shortcut);
        }
    }
    popover.set_child(Some(&content));
    popover
}

fn load_theme_preference() -> ThemePreference {
    ZoteroSettings::load()
        .ok()
        .and_then(|settings| {
            settings
                .extra
                .get("colorScheme")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        })
        .as_deref()
        .map(|value| ThemePreference::from_setting(Some(value)))
        .unwrap_or(ThemePreference::System)
}

fn apply_theme_preference(preference: ThemePreference) {
    let scheme = match preference {
        ThemePreference::System => adw::ColorScheme::Default,
        ThemePreference::Light => adw::ColorScheme::ForceLight,
        ThemePreference::Dark => adw::ColorScheme::ForceDark,
    };
    adw::StyleManager::default().set_color_scheme(scheme);
}

fn save_theme_preference(preference: ThemePreference) {
    let mut settings = match ZoteroSettings::load() {
        Ok(settings) => settings,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => ZoteroSettings::default(),
        Err(error) => {
            eprintln!("Could not load appearance settings: {error}");
            return;
        }
    };
    settings.extra.insert(
        "colorScheme".to_owned(),
        serde_json::Value::String(preference.setting_value().to_owned()),
    );
    if let Err(error) = settings.save() {
        eprintln!("Could not save appearance preference: {error}");
    }
}

fn build_theme_choices(current: ThemePreference) -> gtk::Box {
    let choices = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    choices.set_halign(gtk::Align::Center);
    choices.set_margin_top(8);
    choices.set_margin_bottom(8);
    choices.add_css_class("ovenbird-theme-choices");
    let mut first: Option<gtk::ToggleButton> = None;
    for choice in crate::application_menu::THEME_MENU_CHOICES {
        let preference = match choice.action {
            "theme-light" => ThemePreference::Light,
            "theme-dark" => ThemePreference::Dark,
            _ => ThemePreference::System,
        };
        let toggle = gtk::ToggleButton::new();
        toggle.add_css_class("ovenbird-theme-choice");
        toggle.set_tooltip_text(Some(&tr_dynamic(choice.label)));
        let swatch = gtk::Box::new(gtk::Orientation::Vertical, 0);
        swatch.add_css_class("ovenbird-theme-swatch");
        swatch.add_css_class(match preference {
            ThemePreference::System => "theme-auto",
            ThemePreference::Light => "theme-light",
            ThemePreference::Dark => "theme-dark",
        });
        let overlay = gtk::Overlay::new();
        overlay.set_child(Some(&swatch));
        let check = gtk::Image::from_icon_name("object-select-symbolic");
        check.add_css_class("ovenbird-theme-check");
        check.set_halign(gtk::Align::End);
        check.set_valign(gtk::Align::End);
        overlay.add_overlay(&check);
        toggle.set_child(Some(&overlay));
        if let Some(first) = first.as_ref() {
            toggle.set_group(Some(first));
        } else {
            first = Some(toggle.clone());
        }
        toggle.set_active(preference == current);
        check.set_visible(preference == current);
        toggle.connect_toggled(move |toggle| {
            check.set_visible(toggle.is_active());
            if toggle.is_active() {
                apply_theme_preference(preference);
                save_theme_preference(preference);
            }
        });
        choices.append(&toggle);
    }
    choices
}

fn build_new_item_menu() -> gtk::Popover {
    let popover = gtk::Popover::new();
    let content = gtk::Box::new(gtk::Orientation::Vertical, 2);
    for (icon, label, action) in [
        (
            "text-x-generic-symbolic",
            "New LaTeX document",
            "win.create-tex",
        ),
        (
            "text-x-generic-symbolic",
            "New bibliography (.bib)",
            "win.create-bib",
        ),
        (
            "text-x-generic-symbolic",
            "New style file (.sty)",
            "win.create-sty",
        ),
    ] {
        let item = labeled_button(icon, &tr_dynamic(label));
        item.set_action_name(Some(action));
        item.set_halign(gtk::Align::Fill);
        item.add_css_class("flat");
        content.append(&item);
    }
    content.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    let folder = labeled_button("folder-new-symbolic", &tr("New folder"));
    folder.set_action_name(Some("win.create-folder"));
    folder.set_halign(gtk::Align::Fill);
    folder.add_css_class("flat");
    content.append(&folder);
    popover.set_child(Some(&content));
    popover
}

fn build_editor_page(
    editor: &LatexEditor,
) -> (
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
) {
    let editor_panel = gtk::Box::new(gtk::Orientation::Vertical, 0);
    editor_panel.add_css_class("ovenbird-code-panel");
    let top = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    top.add_css_class("ovenbird-mode-toolbar");
    top.set_hexpand(true);
    top.set_margin_top(5);
    top.set_margin_bottom(4);
    top.set_margin_start(6);
    top.set_margin_end(6);
    let code_mode_button = editor_mode_button("text-x-generic-symbolic", "Code");
    code_mode_button.set_action_name(Some("win.mode-code"));
    code_mode_button.set_active(true);
    let visual_mode_button = editor_mode_button("format-text-rich-symbolic", "Visual");
    visual_mode_button.set_group(Some(&code_mode_button));
    visual_mode_button.set_action_name(Some("win.mode-visual"));
    top.append(&code_mode_button);
    top.append(&visual_mode_button);
    let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    top.append(&spacer);
    let find_document = button("system-search-symbolic", &tr("Find in document"));
    find_document.set_action_name(Some("win.find-document"));
    top.append(&find_document);
    let mode_help = tr("Switch modes at any time without creating another version of the document");
    code_mode_button.set_tooltip_text(Some(&mode_help));
    visual_mode_button.set_tooltip_text(Some(&mode_help));
    editor_panel.append(&top);

    let search = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    search.set_size_request(0, -1);
    search.set_hexpand(true);
    search.set_halign(gtk::Align::Fill);
    search.set_margin_start(6);
    search.set_margin_end(6);
    search.set_margin_bottom(6);
    search.set_visible(false);
    let entry = gtk::SearchEntry::new();
    entry.set_placeholder_text(Some(&tr("Find in document…")));
    entry.set_width_chars(1);
    entry.set_hexpand(true);
    search.append(&entry);
    for (icon, action, tip) in [
        ("go-up-symbolic", "win.search-previous", "Previous match"),
        ("go-down-symbolic", "win.search-next", "Next match"),
    ] {
        let b = button(icon, &tr_dynamic(tip));
        b.set_action_name(Some(action));
        search.append(&b);
    }
    let result = gtk::Label::new(None);
    result.add_css_class("dim-label");
    search.append(&result);
    let close = button("window-close-symbolic", &tr("Close search"));
    close.set_action_name(Some("win.find-document"));
    search.append(&close);
    editor_panel.append(&search);

    let toolbar = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    toolbar.add_css_class("ovenbird-editor-toolbar");
    toolbar.set_hexpand(true);
    toolbar.set_margin_start(8);
    toolbar.set_margin_end(8);
    toolbar.set_margin_bottom(6);
    let latex_toolbar = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    let undo_button = action_button("edit-undo-symbolic", "Undo", "win.undo");
    let redo_button = action_button("edit-redo-symbolic", "Redo", "win.redo");
    undo_button.set_sensitive(false);
    redo_button.set_sensitive(false);
    toolbar.append(&undo_button);
    toolbar.append(&redo_button);
    latex_toolbar.append(&gtk::Separator::new(gtk::Orientation::Vertical));
    let style = gtk::MenuButton::new();
    style.set_label(&tr("Normal text"));
    style.set_popover(Some(&heading_menu()));
    latex_toolbar.append(&style);
    latex_toolbar.append(&gtk::Separator::new(gtk::Orientation::Vertical));
    for (icon, label, action) in [
        ("format-text-bold-symbolic", "Bold", "win.format-bold"),
        ("format-text-italic-symbolic", "Italic", "win.format-italic"),
        (
            "format-text-underline-symbolic",
            "Underline",
            "win.format-underline",
        ),
        (
            "format-text-plaintext-symbolic",
            "Monospace",
            "win.format-monospace",
        ),
    ] {
        let b = button(icon, &tr_dynamic(label));
        b.set_action_name(Some(action));
        latex_toolbar.append(&b);
    }
    let link = action_button("mail-attachment-symbolic", "Insert link", "win.insert-link");
    latex_toolbar.append(&link);
    latex_toolbar.append(&gtk::Separator::new(gtk::Orientation::Vertical));
    let math = gtk::MenuButton::new();
    math.set_label(&tr("Math"));
    math.set_popover(Some(&math_menu()));
    latex_toolbar.append(&math);
    let symbols = gtk::MenuButton::new();
    symbols.set_label("Ω");
    symbols.set_tooltip_text(Some(&tr("Math symbols")));
    symbols.set_popover(Some(&symbol_menu()));
    latex_toolbar.append(&symbols);
    let cite = gtk::Button::with_label(&tr("Insert citation"));
    cite.set_tooltip_text(Some(&tr("Find and insert a bibliography reference")));
    cite.set_action_name(Some("win.insert-citation"));
    latex_toolbar.append(&cite);
    latex_toolbar.append(&gtk::Separator::new(gtk::Orientation::Vertical));
    let lists = gtk::MenuButton::new();
    lists.set_icon_name("view-list-symbolic");
    lists.set_tooltip_text(Some(&tr("Lists and indentation")));
    lists.set_popover(Some(&list_menu()));
    latex_toolbar.append(&lists);
    let more = gtk::MenuButton::new();
    more.set_icon_name("view-more-symbolic");
    more.set_tooltip_text(Some(&tr("More")));
    more.set_popover(Some(&more_menu()));
    latex_toolbar.append(&more);
    toolbar.append(&latex_toolbar);
    let toolbar_spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    toolbar_spacer.set_hexpand(true);
    toolbar.append(&toolbar_spacer);
    let compact_find_document = button("system-search-symbolic", &tr("Find in document"));
    compact_find_document.set_action_name(Some("win.find-document"));
    compact_find_document.set_visible(false);
    toolbar.append(&compact_find_document);
    let toolbar_scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Automatic)
        .vscrollbar_policy(gtk::PolicyType::Never)
        .min_content_width(0)
        .propagate_natural_width(false)
        .hexpand(true)
        .child(&toolbar)
        .build();
    editor_panel.append(&toolbar_scroll);
    let file_preview_stack = gtk::Stack::builder().hexpand(true).vexpand(true).build();
    file_preview_stack.add_named(&editor.root, Some("text"));
    let image_preview = gtk::Picture::new();
    image_preview.set_can_shrink(true);
    image_preview.set_content_fit(gtk::ContentFit::Contain);
    let image_scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Automatic)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .hexpand(true)
        .vexpand(true)
        .child(&image_preview)
        .build();
    file_preview_stack.add_named(&image_scroll, Some("image"));
    let file_pdf_preview = crate::pdf_preview::PdfPreview::new();
    file_preview_stack.add_named(file_pdf_preview.widget(), Some("pdf"));
    file_preview_stack.set_visible_child_name("text");
    editor_panel.append(&file_preview_stack);

    let preview = crate::pdf_preview::PdfPreview::new();
    let page = gtk::Box::new(gtk::Orientation::Vertical, 0);
    page.add_css_class("ovenbird-editor-page");
    page.set_hexpand(true);
    page.set_vexpand(true);
    editor_panel.set_hexpand(true);
    editor_panel.set_vexpand(true);
    let workspace = gtk::Paned::new(gtk::Orientation::Horizontal);
    workspace.set_wide_handle(true);
    workspace.set_resize_start_child(true);
    workspace.set_resize_end_child(true);
    workspace.set_shrink_start_child(true);
    workspace.set_shrink_end_child(true);
    workspace.set_start_child(Some(&editor_panel));
    workspace.set_end_child(Some(preview.widget()));
    workspace.set_hexpand(true);
    workspace.set_vexpand(true);
    page.append(&workspace);
    (
        editor_panel,
        page,
        preview,
        file_pdf_preview,
        file_preview_stack,
        image_preview,
        top,
        toolbar_scroll,
        latex_toolbar,
        undo_button,
        redo_button,
        code_mode_button,
        visual_mode_button,
        find_document,
        compact_find_document,
    )
}

fn action_button(icon: &str, tooltip: &str, action: &str) -> gtk::Button {
    let b = button(icon, &tr_dynamic(tooltip));
    b.set_action_name(Some(action));
    b
}

fn menu_with_actions(items: &[(&str, &str, &str)]) -> gtk::Popover {
    let popover = gtk::Popover::new();
    let content = gtk::Box::new(gtk::Orientation::Vertical, 2);
    content.set_margin_top(5);
    content.set_margin_bottom(5);
    content.set_margin_start(5);
    content.set_margin_end(5);
    for (icon, label, action) in items {
        if label.is_empty() {
            content.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
            continue;
        }
        let item = labeled_button(icon, &tr_dynamic(label));
        item.set_action_name(Some(action));
        item.set_halign(gtk::Align::Fill);
        item.add_css_class("flat");
        item.add_css_class("ovenbird-app-menu-item");
        content.append(&item);
    }
    popover.set_child(Some(&content));
    popover
}

fn action_menu(items: &[(&str, &str)]) -> gio::Menu {
    let model = gio::Menu::new();
    for (label, action) in items {
        model.append(Some(&tr_dynamic(label)), Some(action));
    }
    model
}

fn heading_menu() -> gtk::Popover {
    menu_with_actions(&[
        (
            "format-justify-left-symbolic",
            "Normal text",
            "win.heading-normal",
        ),
        (
            "format-text-heading-symbolic",
            "Section",
            "win.heading-section",
        ),
        (
            "format-text-heading-symbolic",
            "Subsection",
            "win.heading-subsection",
        ),
        (
            "format-text-heading-symbolic",
            "Subsubsection",
            "win.heading-subsubsection",
        ),
        (
            "format-text-heading-symbolic",
            "Paragraph",
            "win.heading-paragraph",
        ),
        (
            "format-text-heading-symbolic",
            "Subparagraph",
            "win.heading-subparagraph",
        ),
    ])
}
fn math_menu() -> gtk::Popover {
    menu_with_actions(&[
        ("insert-math-symbolic", "Inline math", "win.math-inline"),
        ("insert-math-symbolic", "Display math", "win.math-display"),
    ])
}
fn symbol_menu() -> gtk::Popover {
    menu_with_actions(&[
        ("", "α  Alpha", "win.insert-alpha"),
        ("", "β  Beta", "win.insert-beta"),
        ("", "γ  Gamma", "win.insert-gamma"),
        ("", "π  Pi", "win.insert-pi"),
        ("", "≤  Less than or equal to", "win.insert-leq"),
        ("", "≥  Greater than or equal to", "win.insert-geq"),
        ("", "≠  Not equal to", "win.insert-neq"),
        ("", "×  Multiplication", "win.insert-times"),
        ("", "±  Plus or minus", "win.insert-pm"),
        ("", "∞  Infinity", "win.insert-infty"),
        ("", "∑  Summation", "win.insert-sum"),
        ("", "∫  Integral", "win.insert-int"),
        ("", "√  Square root", "win.insert-sqrt"),
    ])
}
fn list_menu() -> gtk::Popover {
    menu_with_actions(&[
        (
            "view-list-bullet-symbolic",
            "Bulleted list",
            "win.list-itemize",
        ),
        (
            "view-list-ordered-symbolic",
            "Numbered list",
            "win.list-enumerate",
        ),
        ("quotation-mark", "Quotation block", "win.list-quote"),
        (
            "format-indent-more-symbolic",
            "Increase indent",
            "win.indent-more",
        ),
        (
            "format-indent-less-symbolic",
            "Decrease indent",
            "win.indent-less",
        ),
    ])
}
fn more_menu() -> gtk::Popover {
    menu_with_actions(&[
        ("insert-image-symbolic", "Insert image", "win.insert-image"),
        ("view-grid-symbolic", "Insert table", "win.insert-table"),
        (
            "chat-message-new-symbolic",
            "LaTeX comment",
            "win.insert-comment",
        ),
        ("insert-text-symbolic", "Footnote", "win.insert-footnote"),
        (
            "document-edit-symbolic",
            "Review note",
            "win.insert-review-note",
        ),
        ("bookmark-new-symbolic", "Add label", "win.insert-label"),
        (
            "insert-link-symbolic",
            "Cross-reference",
            "win.insert-reference",
        ),
    ])
}

fn build_library_page() -> (gtk::Box, gtk::Box, ReferenceFilterWidgets) {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 10);
    page.set_margin_top(20);
    page.set_margin_bottom(12);
    page.set_margin_start(22);
    page.set_margin_end(22);
    let filter_bar = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    filter_bar.add_css_class("ovenbird-reference-filters");
    let author = reference_filter_dropdown("All authors");
    let year = reference_filter_dropdown("All years");
    let tag = reference_filter_dropdown("All tags");
    for dropdown in [&author, &year, &tag] {
        dropdown.set_hexpand(true);
        dropdown.set_width_request(150);
        filter_bar.append(dropdown);
    }
    page.append(&filter_bar);
    let list = gtk::Box::new(gtk::Orientation::Vertical, 3);
    let scroll = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .child(&list)
        .build();
    page.append(&scroll);
    (page, list, ReferenceFilterWidgets { author, year, tag })
}

fn reference_filter_dropdown(all_label: &str) -> gtk::DropDown {
    let all_label = tr(all_label);
    gtk::DropDown::from_strings(&[&all_label])
}

fn build_header(state: &Rc<State>) {
    let header = adw::HeaderBar::new();
    header.add_css_class("ovenbird-main-toolbar");
    header.set_size_request(-1, MAIN_HEADER_HEIGHT);

    let omni_bar = crate::libpanel::OmniBar::new();
    let center = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    center.set_valign(gtk::Align::Center);
    state.title.set_label(&tr("LaTeX editor"));
    center.append(&state.title);

    let build_status = build_status_widgets(state.status.clone());
    center.append(&build_status.status_row);

    let reference_summary_group = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    reference_summary_group.set_valign(gtk::Align::Center);
    reference_summary_group.append(&gtk::Separator::new(gtk::Orientation::Vertical));
    let reference_summary = gtk::Label::new(None);
    reference_summary.add_css_class("dim-label");
    reference_summary_group.append(&reference_summary);
    reference_summary_group.set_visible(false);
    center.append(&reference_summary_group);

    omni_bar.add_prefix(0, &center);
    omni_bar.set_action_name("win.compile");
    omni_bar.set_action_tooltip(&tr("Compile and create PDF"));
    omni_bar.set_icon_name("media-playback-start-symbolic");
    header.set_title_widget(Some(omni_bar.widget()));

    state.reference_summary.replace(Some(reference_summary));
    state
        .reference_summary_group
        .replace(Some(reference_summary_group));
    state.build_status_widgets.replace(Some(build_status));
    state.omni_bar.replace(Some(omni_bar));

    let toggle = button("sidebar-show-symbolic", &tr("Show or hide the sidebar"));
    toggle.set_action_name(Some("win.toggle-sidebar"));
    header.pack_start(&toggle);

    let weak_state = Rc::downgrade(state);
    state.layout.connect_position_notify(move |layout| {
        if layout.start_child().is_some() {
            let width = layout.position();
            if width > 0 {
                if let Some(state) = weak_state.upgrade() {
                    state.sidebar_width.set(width);
                }
            }
        }
    });

    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let save = button("document-save-symbolic", &tr("Save document"));
    save.set_action_name(Some("win.save-document"));
    save.add_css_class("flat");
    actions.append(&save);
    state.save_button.replace(Some(save));
    actions.append(&state.sync_button);
    header.pack_end(&actions);

    install_main_header(&state.toolbar_view, &header);
    update_primary_action(state);
    refresh_build_status(state);
}

fn install_main_header(toolbar_view: &adw::ToolbarView, header: &adw::HeaderBar) {
    header.set_decoration_layout(Some("minimize,maximize,close"));
    header.set_show_start_title_buttons(false);
    header.set_show_end_title_buttons(true);
    toolbar_view.add_top_bar(header);
}

fn build_status_widgets(status: gtk::Label) -> BuildStatusWidgets {
    let status_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    status_row.set_valign(gtk::Align::Center);
    status_row.append(&gtk::Separator::new(gtk::Orientation::Vertical));
    let status_contents = gtk::Box::new(gtk::Orientation::Horizontal, 5);
    status_contents.set_valign(gtk::Align::Center);
    let icon = gtk::Image::from_icon_name("emblem-ok-symbolic");
    icon.set_pixel_size(14);
    status_contents.append(&icon);
    status_contents.append(&status);
    status_row.append(&status_contents);

    let details_popover = gtk::Popover::new();
    details_popover.set_has_arrow(true);
    let content = gtk::Box::new(gtk::Orientation::Vertical, 10);
    content.set_size_request(340, -1);
    content.set_margin_top(14);
    content.set_margin_bottom(14);
    content.set_margin_start(16);
    content.set_margin_end(16);

    let heading = gtk::Label::new(Some(&tr("Current document")));
    heading.set_xalign(0.0);
    heading.add_css_class("title-4");
    content.append(&heading);

    let details_file = gtk::Label::new(None);
    details_file.set_xalign(0.0);
    details_file.set_wrap(true);
    details_file.set_selectable(true);
    content.append(&details_file);

    let details_path = gtk::Label::new(None);
    details_path.set_xalign(0.0);
    details_path.set_wrap(true);
    details_path.set_selectable(true);
    details_path.add_css_class("dim-label");
    content.append(&details_path);

    let grid = gtk::Grid::new();
    grid.set_column_spacing(14);
    grid.set_row_spacing(6);
    let details_engine = gtk::Label::new(None);
    let details_state = gtk::Label::new(None);
    let details_last = gtk::Label::new(None);
    let engine_name = crate::build::find_latex_engine()
        .map(|(engine, _)| {
            tr(match engine {
                crate::build::LatexEngine::LatexMk => "latexmk",
                crate::build::LatexEngine::PdfLatex => "pdflatex",
                crate::build::LatexEngine::Tectonic => "Tectonic",
            })
        })
        .unwrap_or_else(|| tr("Not found"));
    details_engine.set_label(&engine_name);
    for (row, (caption, value)) in [
        ("Compiler", &details_engine),
        ("Status", &details_state),
        ("Last build", &details_last),
    ]
    .into_iter()
    .enumerate()
    {
        let label = gtk::Label::new(Some(&tr(caption)));
        label.set_xalign(1.0);
        label.add_css_class("dim-label");
        value.set_xalign(0.0);
        value.set_hexpand(true);
        value.set_wrap(true);
        grid.attach(&label, 0, row as i32, 1, 1);
        grid.attach(value, 1, row as i32, 1, 1);
    }
    content.append(&grid);

    let details_output = gtk::Label::new(None);
    details_output.set_xalign(0.0);
    details_output.set_wrap(true);
    details_output.set_selectable(true);
    details_output.set_max_width_chars(48);
    details_output.add_css_class("dim-label");
    content.append(&details_output);
    details_popover.set_child(Some(&content));

    BuildStatusWidgets {
        status_row,
        icon,
        details_popover,
        details_file,
        details_path,
        details_engine,
        details_state,
        details_last,
        details_output,
    }
}

fn update_primary_action(state: &Rc<State>) {
    let references = state.page_stack.visible_child_name().as_deref() == Some("references");
    if let Some(omni_bar) = state.omni_bar.borrow().as_ref() {
        if references {
            omni_bar.set_action_name("win.add-reference");
            omni_bar.set_action_tooltip(&tr("Add a reference to the local library"));
            omni_bar.set_icon_name("list-add-symbolic");
        } else {
            omni_bar.set_action_name("win.compile");
            omni_bar.set_action_tooltip(&tr("Compile and create PDF"));
            omni_bar.set_icon_name("media-playback-start-symbolic");
        }
        let actions = if references {
            Some(action_menu(&[
                ("Import BibTeX", "win.import-bibtex"),
                ("Export BibTeX", "win.export-bibtex"),
            ]))
        } else if current_file_kind(state).is_none_or(ProjectFileKind::is_text_editable) {
            Some(action_menu(&[
                ("Save document", "win.save-document"),
                ("Find in document", "win.find-document"),
            ]))
        } else {
            None
        };
        omni_bar.set_menu_model(
            actions
                .as_ref()
                .map(|menu| menu.upcast_ref::<gio::MenuModel>()),
        );
        let details = state.build_status_widgets.borrow();
        let popover = if references {
            None
        } else {
            details.as_ref().map(|widgets| &widgets.details_popover)
        };
        omni_bar.set_popover(popover);
    }
    if let Some(action) = state.compile_action.borrow().as_ref() {
        action.set_enabled(state.build_state.borrow().as_str() != "Building…");
    }
    if let Some(status) = state.build_status_widgets.borrow().as_ref() {
        status.status_row.set_visible(!references);
    }
    if let Some(summary) = state.reference_summary.borrow().as_ref() {
        let count = state.library.borrow().bibliography.entries.len();
        summary.set_label(
            &ngettext("%d reference", "%d references", count).replace("%d", &count.to_string()),
        );
        summary.set_visible(references);
    }
    if let Some(group) = state.reference_summary_group.borrow().as_ref() {
        group.set_visible(references);
    }
    if let Some(button) = state.save_button.borrow().as_ref() {
        button.set_visible(!references);
    }
    state.sync_button.set_visible(references);
}

fn refresh_build_status(state: &Rc<State>) {
    let status_ref = state.build_status_widgets.borrow();
    let Some(status) = status_ref.as_ref() else {
        return;
    };
    let dirty = state.editor.dirty();
    let build_state = state.build_state.borrow().clone();
    let display_state = if build_state == "Error" {
        tr("Error")
    } else if dirty {
        tr("Modified")
    } else {
        tr(&build_state)
    };
    status.icon.set_icon_name(Some(if build_state == "Error" {
        "dialog-error-symbolic"
    } else if dirty {
        "document-edit-symbolic"
    } else if build_state == "Building…" {
        "view-refresh-symbolic"
    } else {
        "checkbox-checked-symbolic"
    }));
    state.status.set_label(&display_state);

    let file = state.current_file.borrow().clone();
    let filename = file
        .as_ref()
        .and_then(|path| path.file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| tr("New document not saved yet"));
    status.details_file.set_label(&filename);
    let file_path = file
        .as_ref()
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_else(|| tr("Save the document to set the project file."));
    let project_folder = state
        .project_root
        .borrow()
        .clone()
        .or_else(|| {
            file.as_ref()
                .and_then(|path| path.parent().map(Path::to_path_buf))
        })
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_else(|| tr("No project folder selected"));
    let path_details = tr("File: %s\nProject: %s")
        .replacen("%s", &file_path, 1)
        .replacen("%s", &project_folder, 1);
    status.details_path.set_label(&path_details);

    let details_state = if dirty {
        tr("%s · local changes pending").replacen("%s", &tr(&build_state), 1)
    } else {
        tr(&build_state)
    };
    status.details_state.set_label(&details_state);
    status
        .details_last
        .set_label(state.last_build_at.borrow().as_str());
    status
        .details_output
        .set_label(state.last_build_message.borrow().as_str());
}

fn set_build_status(state: &Rc<State>, status: &str) {
    set_build_status_with_message(state, status, None);
}

fn set_build_status_with_message(state: &Rc<State>, status: &str, message: Option<&str>) {
    state.build_state.replace(status.to_owned());
    if status == "Ready" {
        state.last_build_at.replace(tr("Not built yet"));
        if message.is_none() {
            state
                .last_build_message
                .replace(tr("This document has not been compiled yet."));
        }
    } else if status == "Built" || status == "Error" {
        state.last_build_at.replace(
            glib::DateTime::now_local()
                .ok()
                .and_then(|now| now.format("%H:%M:%S").ok())
                .map(|time| time.to_string())
                .unwrap_or_default(),
        );
    }
    if let Some(message) = message {
        let start = message.chars().count().saturating_sub(600);
        state
            .last_build_message
            .replace(message.chars().skip(start).collect());
    }
    if let Some(omni_bar) = state.omni_bar.borrow().as_ref() {
        if status == "Building…" {
            omni_bar.start_pulsing();
        } else {
            omni_bar.stop_pulsing();
        }
    }
    refresh_build_status(state);
    update_primary_action(state);
}

fn find_search_panel(page: &gtk::Box) -> gtk::Box {
    let mut child = page.first_child();
    while let Some(widget) = child {
        if let Some(row) = widget.downcast_ref::<gtk::Box>() {
            if row
                .first_child()
                .is_some_and(|child| child.is::<gtk::SearchEntry>())
            {
                return row.clone();
            }
        }
        child = widget.next_sibling();
    }
    gtk::Box::new(gtk::Orientation::Horizontal, 0)
}

fn find_search_entry(page: &gtk::Box) -> gtk::SearchEntry {
    let panel = find_search_panel(page);
    panel
        .first_child()
        .and_downcast::<gtk::SearchEntry>()
        .unwrap_or_else(gtk::SearchEntry::new)
}

fn find_search_result(page: &gtk::Box) -> gtk::Label {
    let panel = find_search_panel(page);
    panel
        .first_child()
        .and_then(|entry| entry.next_sibling())
        .and_then(|previous| previous.next_sibling())
        .and_then(|next| next.next_sibling())
        .and_downcast::<gtk::Label>()
        .unwrap_or_else(|| gtk::Label::new(None))
}

fn connect_navigation(state: &Rc<State>) {
    let search_entry = state.search_entry.clone();
    search_entry.connect_search_changed({
        let state = state.clone();
        move |entry| {
            let query = entry.text();
            if query.trim().is_empty() {
                state.search_result.set_label("");
            } else {
                let found = state.editor.search_from_start(query.as_str(), true);
                state.search_result.set_label(&tr(if found {
                    "Match found"
                } else {
                    "No results"
                }));
            }
        }
    });
    state.library_search.connect_search_changed({
        let state = state.clone();
        move |_| refresh_library(&state)
    });
    let state_weak = Rc::downgrade(state);
    for filter in [
        state.reference_filters.author.clone(),
        state.reference_filters.year.clone(),
        state.reference_filters.tag.clone(),
    ] {
        let state_weak = state_weak.clone();
        filter.connect_selected_notify(move |_| {
            if let Some(state) = state_weak.upgrade() {
                if !state.library_filters_updating.get() {
                    refresh_library(&state);
                }
            }
        });
    }
}

fn install_action(state: &Rc<State>, name: &str, callback: impl Fn(Rc<State>) + 'static) {
    let action = gio::SimpleAction::new(name, None);
    let weak = Rc::downgrade(state);
    action.connect_activate(move |_, _| {
        if let Some(state) = weak.upgrade() {
            callback(state);
        }
    });
    state.window.add_action(&action);
}

fn connect_actions(state: &Rc<State>) {
    install_action(state, "show-editor", |s| show_page(&s, "editor"));
    install_action(state, "show-references", |s| show_page(&s, "references"));
    install_action(state, "find-document", toggle_search);
    install_action(state, "toggle-sidebar", |s| {
        set_sidebar_visible(
            &s.layout,
            &s.sidebar_revealer,
            &s.sidebar_width,
            s.layout.start_child().is_none(),
        );
    });
    install_action(state, "search-next", |s| search_document(&s, true));
    install_action(state, "search-previous", |s| search_document(&s, false));
    install_action(state, "open-document", choose_open_document);
    install_action(state, "open-project", choose_project_folder);
    install_action(state, "close-project", close_project);
    install_action(state, "save-document", save_document);
    let string_parameter = String::static_variant_type();
    let open_path = gio::SimpleAction::new("open-path", Some(string_parameter.as_ref()));
    let weak = Rc::downgrade(state);
    open_path.connect_activate(move |_, parameter| {
        let Some(state) = weak.upgrade() else {
            return;
        };
        let Some(path) = parameter.and_then(|value| value.get::<String>()) else {
            return;
        };
        with_discard_confirmation(&state, move |state| {
            open_document(&state, Path::new(&path), None)
        });
    });
    state.window.add_action(&open_path);
    install_action(state, "new-project", show_template_picker);
    install_action(state, "create-tex", |s| create_project_file(&s, "tex"));
    install_action(state, "create-bib", |s| create_project_file(&s, "bib"));
    install_action(state, "create-sty", |s| create_project_file(&s, "sty"));
    install_action(state, "create-folder", create_project_folder);
    install_action(state, "mode-code", |s| {
        s.editor.set_mode(EditorMode::Code);
        s.code_mode_button.set_active(true);
    });
    install_action(state, "mode-visual", |s| {
        let supports_visual = s.editor.file().is_none_or(|path| {
            path.extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("tex"))
        });
        if supports_visual {
            s.editor.set_mode(EditorMode::Visual);
            s.visual_mode_button.set_active(true);
        } else {
            s.code_mode_button.set_active(true);
            show_toast(
                &s,
                &tr("Visual mode is available only for LaTeX documents."),
            );
        }
    });
    install_action(state, "undo", |s| s.editor.undo());
    install_action(state, "redo", |s| s.editor.redo());
    for (name, format) in [
        ("format-bold", "bold"),
        ("format-italic", "italic"),
        ("format-underline", "underline"),
        ("format-monospace", "monospace"),
    ] {
        install_action(state, name, move |s| {
            if let Err(error) = s.editor.apply_format(format) {
                show_toast(&s, &error);
            }
        });
    }
    for (name, style) in [
        ("heading-normal", "normal"),
        ("heading-section", "section"),
        ("heading-subsection", "subsection"),
        ("heading-subsubsection", "subsubsection"),
        ("heading-paragraph", "paragraph"),
        ("heading-subparagraph", "subparagraph"),
    ] {
        install_action(state, name, move |s| {
            if let Err(error) = s.editor.apply_heading(style) {
                show_toast(&s, &error);
            }
        });
    }
    install_action(state, "math-inline", |s| {
        insert_snippet(
            &s,
            commands::create_math_snippet(false, &s.editor.selected_text()),
        )
    });
    install_action(state, "math-display", |s| {
        insert_snippet(
            &s,
            commands::create_math_snippet(true, &s.editor.selected_text()),
        )
    });
    for (name, command) in [
        ("insert-alpha", "\\alpha"),
        ("insert-beta", "\\beta"),
        ("insert-gamma", "\\gamma"),
        ("insert-pi", "\\pi"),
        ("insert-leq", "\\leq"),
        ("insert-geq", "\\geq"),
        ("insert-neq", "\\neq"),
        ("insert-times", "\\times"),
        ("insert-pm", "\\pm"),
        ("insert-infty", "\\infty"),
        ("insert-sum", "\\sum"),
        ("insert-int", "\\int"),
    ] {
        install_action(state, name, move |s| {
            insert_snippet(&s, commands::create_math_snippet(false, command));
        });
    }
    install_action(state, "insert-sqrt", |s| {
        let selected = s.editor.selected_text();
        let start = "\\(\\sqrt{".chars().count();
        let end = start + selected.chars().count();
        let text = format!("\\(\\sqrt{{{selected}}}\\)");
        let insertion = LatexInsertion {
            text,
            cursor_offset: if selected.is_empty() { start } else { end },
            selection: (!selected.is_empty()).then_some((start, end)),
        };
        s.editor.insert_latex(&insertion, "insert:symbol");
    });
    for (name, kind) in [
        ("list-itemize", "itemize"),
        ("list-enumerate", "enumerate"),
        ("list-quote", "quote"),
    ] {
        install_action(state, name, move |s| {
            let selected = s.editor.selected_text();
            match commands::create_list_snippet(kind, &selected) {
                Ok(snippet) => s.editor.insert_latex(&snippet, &format!("insert:{kind}")),
                Err(error) => show_toast(&s, &error),
            }
        });
    }
    install_action(state, "indent-more", |s| indent_selection(&s, true));
    install_action(state, "indent-less", |s| indent_selection(&s, false));
    install_action(state, "insert-link", show_link_dialog);
    install_action(state, "insert-image", choose_image);
    install_action(state, "insert-table", show_table_dialog);
    install_action(state, "insert-comment", |s| s.editor.insert_comment());
    install_action(state, "insert-footnote", |s| s.editor.insert_footnote());
    install_action(state, "insert-review-note", |s| {
        insert_label_or_note(&s, "review")
    });
    install_action(state, "insert-label", |s| insert_label_or_note(&s, "label"));
    install_action(state, "insert-reference", |s| {
        insert_label_or_note(&s, "ref")
    });
    install_action(state, "insert-citation", show_citation_picker);
    let compile_action = gio::SimpleAction::new("compile", None);
    let weak = Rc::downgrade(state);
    compile_action.connect_activate(move |_, _| {
        if let Some(state) = weak.upgrade() {
            compile(&state, None);
        }
    });
    state.window.add_action(&compile_action);
    state.compile_action.replace(Some(compile_action));
    install_action(state, "export-pdf", choose_pdf_export);
    install_action(state, "add-reference", |s| edit_reference(&s, None));
    install_action(state, "import-bibtex", import_bibtex);
    install_action(state, "export-bibtex", export_bibtex);
    install_action(state, "zotero", show_zotero_settings);
    install_action(state, "sync-zotero", sync_or_configure_zotero);
    install_action(state, "shortcuts", show_shortcuts);
    install_action(state, "close-window", |s| s.window.close());
    install_action(state, "quit", |s| s.window.close());
}

fn connect_window_close_guard(state: &Rc<State>) {
    state.window.connect_close_request({
        let state = Rc::downgrade(state);
        move |_| {
            let Some(state) = state.upgrade() else {
                return glib::Propagation::Proceed;
            };
            if state.close_confirmed.replace(false) {
                return glib::Propagation::Proceed;
            }
            if state.save_in_progress.get() {
                show_toast(&state, &tr("A document save is still in progress."));
                return glib::Propagation::Stop;
            }
            if !state.editor.dirty() {
                return glib::Propagation::Proceed;
            }
            with_discard_confirmation(&state, move |state| {
                state.close_confirmed.set(true);
                state.window.close();
            });
            glib::Propagation::Stop
        }
    });
}

fn close_project(state: Rc<State>) {
    with_discard_confirmation(&state, move |state| {
        state
            .open_request_id
            .set(state.open_request_id.get().wrapping_add(1));
        state
            .project_scan_id
            .set(state.project_scan_id.get().wrapping_add(1));
        state
            .project_open_id
            .set(state.project_open_id.get().wrapping_add(1));
        state.project_root.borrow_mut().take();
        state.project_target.borrow_mut().take();
        state.current_file.borrow_mut().take();
        state.file_encoding.set(project::TextEncoding::Utf8);
        state.main_tex.borrow_mut().take();
        state.expanded_folders.borrow_mut().clear();
        state.editor.set_file(None);
        state.editor.load_text(&LatexEditor::new_document_text());
        state.image_preview.set_filename(None::<&Path>);
        state
            .file_pdf_preview
            .set_message(&tr("Select a PDF file to preview it here."));
        state.search_entry.set_text("");
        state.search_panel.set_visible(false);
        state
            .pdf_preview
            .set_message(&tr("The compiled PDF will appear here"));
        update_editor_capabilities(&state);
        set_build_status(&state, "Ready");
        show_page(&state, "editor");
        refresh_project_files(&state);
        update_title(&state, false);
        show_toast(&state, &tr("Project closed"));
    });
}

fn configure_editor_callbacks(state: &Rc<State>) {
    state.editor.set_dirty_changed({
        let state = state.clone();
        move |dirty| update_title(&state, dirty)
    });
    state.editor.set_history_changed({
        let state = state.clone();
        move |undo, redo| {
            state.undo_button.set_sensitive(undo);
            state.redo_button.set_sensitive(redo);
            refresh_build_status(&state);
        }
    });
}

fn update_editor_capabilities(state: &Rc<State>) {
    let kind = current_file_kind(state);
    let supports_latex = kind.is_none_or(|kind| kind == ProjectFileKind::Latex);
    let text_editable = kind.is_none_or(ProjectFileKind::is_text_editable);
    let supports_search = text_editable;

    state.mode_toolbar.set_visible(supports_latex);
    state.code_mode_button.set_visible(supports_latex);
    state.visual_mode_button.set_visible(supports_latex);
    state.find_document_button.set_visible(supports_latex);
    state
        .compact_find_document_button
        .set_visible(supports_search && !supports_latex);
    state.latex_toolbar.set_visible(supports_latex);
    state.editor_toolbar.set_visible(text_editable);
    if !supports_search {
        state.search_panel.set_visible(false);
    }
    if let Some(button) = state.save_button.borrow().as_ref() {
        button.set_visible(text_editable);
    }

    state.file_preview_stack.set_visible_child_name(match kind {
        Some(ProjectFileKind::Image) => "image",
        Some(ProjectFileKind::Pdf) => "pdf",
        _ => "text",
    });

    if !supports_latex {
        state.editor.set_mode(EditorMode::Code);
        state.code_mode_button.set_active(true);
    }
}

fn supports_latex_document(state: &Rc<State>) -> bool {
    current_file_kind(state).is_none_or(|kind| kind == ProjectFileKind::Latex)
}

fn current_file_kind(state: &Rc<State>) -> Option<ProjectFileKind> {
    state
        .current_file
        .borrow()
        .as_deref()
        .map(project::file_kind)
}

fn show_page(state: &Rc<State>, page: &str) {
    state.page_stack.set_visible_child_name(page);
    state.sidebar_pages.set_visible_child_name(page);
    state.editor_tab_button.set_active(page == "editor");
    state.references_tab_button.set_active(page == "references");
    state.title.set_label(&tr(if page == "references" {
        "References"
    } else {
        "LaTeX editor"
    }));
    update_primary_action(state);
    refresh_build_status(state);
    if page == "references" {
        refresh_library(state);
    }
}

fn set_sidebar_visible(
    layout: &gtk::Paned,
    sidebar: &gtk::Revealer,
    sidebar_width: &Cell<i32>,
    visible: bool,
) {
    if visible {
        layout.set_start_child(Some(sidebar));
        let width = sidebar_width.get().max(1);
        layout.set_position(width);
        sidebar.set_reveal_child(true);
    } else {
        let width = layout.position();
        if width > 0 {
            sidebar_width.set(width);
        }
        sidebar.set_reveal_child(false);
        layout.set_start_child(None::<&gtk::Widget>);
    }
}

fn toggle_search(state: Rc<State>) {
    if search_is_references_page(&state.page_stack) {
        state.library_search.grab_focus();
        return;
    }
    let kind = current_file_kind(&state);
    if kind.is_some_and(|kind| !kind.is_text_editable()) {
        return;
    }
    if state.page_stack.visible_child_name().as_deref() != Some("editor") {
        show_page(&state, "editor");
    }
    let open = !state.search_panel.is_visible();
    state.search_panel.set_visible(open);
    if open {
        state.editor.focus();
        state.search_entry.grab_focus();
        state.search_entry.select_region(0, -1);
    } else {
        state.editor.focus();
    }
}

fn search_is_references_page(page_stack: &gtk::Stack) -> bool {
    page_stack.visible_child_name().as_deref() == Some("references")
}

fn search_document(state: &Rc<State>, forward: bool) {
    let query = state.search_entry.text();
    let found = state.editor.search(query.as_str(), forward);
    let message = if query.trim().is_empty() {
        String::new()
    } else if found {
        tr("Match found")
    } else {
        tr("No results")
    };
    state.search_result.set_label(&message);
}

fn show_toast(state: &Rc<State>, message: &str) {
    state.toast.add_toast(adw::Toast::new(message));
}

fn update_title(state: &Rc<State>, dirty: bool) {
    let path = state.current_file.borrow().clone();
    let name = path
        .as_ref()
        .and_then(|file| file.file_name())
        .and_then(|name| name.to_str())
        .unwrap_or("Ovenbird");
    state.window.set_title(Some(&format!(
        "{}{} — Ovenbird",
        name,
        if dirty { " •" } else { "" }
    )));
    refresh_build_status(state);
}

fn show_shortcuts(state: Rc<State>) {
    let dialog = gtk::Dialog::builder()
        .title(tr("Keyboard shortcuts"))
        .transient_for(&state.window)
        .modal(true)
        .build();
    dialog.add_button(&tr("Close"), gtk::ResponseType::Close);
    let content = dialog.content_area();
    content.set_spacing(8);
    content.set_margin_top(18);
    content.set_margin_bottom(18);
    content.set_margin_start(20);
    content.set_margin_end(20);
    for (action, shortcut) in [
        ("New project", "Ctrl+Alt+N"),
        ("Open document", "Ctrl+O"),
        ("Open project", "Ctrl+Alt+O"),
        ("Close project", "Ctrl+W"),
        ("Save document", "Ctrl+S"),
        ("Find in document", "Ctrl+F"),
        ("Compile", "Ctrl+Shift+R"),
        ("Insert citation", "Ctrl+Shift+C"),
        ("Toggle sidebar", "F9"),
        ("Undo", "Ctrl+Z"),
        ("Redo", "Ctrl+Shift+Z"),
    ] {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        row.append(&gtk::Label::new(Some(&tr_dynamic(action))));
        let key = gtk::Label::new(Some(shortcut));
        key.set_hexpand(true);
        key.set_halign(gtk::Align::End);
        key.add_css_class("dim-label");
        row.append(&key);
        content.append(&row);
    }
    dialog.connect_response(|dialog, _| dialog.close());
    dialog.present();
}

fn choose_file(
    state: &Rc<State>,
    action: gtk::FileChooserAction,
    title: &str,
    accept: &str,
    callback: impl Fn(Rc<State>, PathBuf) + 'static,
) {
    let dialog = gtk::FileChooserNative::new(
        Some(&tr_dynamic(title)),
        Some(&state.window),
        action,
        Some(&tr_dynamic(accept)),
        Some(&tr("Cancel")),
    );
    let filter = gtk::FileFilter::new();
    filter.set_name(Some(&tr("Supported documents and images")));
    for pattern in [
        "*.tex", "*.bib", "*.txt", "*.bst", "*.cls", "*.sty", "*.pdf", "*.png", "*.jpg", "*.jpeg",
        "*.gif", "*.bmp", "*.tif", "*.tiff", "*.svg", "*.webp",
    ] {
        filter.add_pattern(pattern);
    }
    dialog.add_filter(&filter);
    let state = state.clone();
    dialog.connect_response(move |dialog, response| {
        if response == gtk::ResponseType::Accept {
            if let Some(path) = dialog.file().and_then(|file| file.path()) {
                callback(state.clone(), path);
            }
        }
        dialog.destroy();
    });
    dialog.show();
}

fn choose_open_document(state: Rc<State>) {
    choose_file(
        &state,
        gtk::FileChooserAction::Open,
        "Open document",
        "Open",
        |state, path| {
            with_discard_confirmation(&state, move |state| open_document(&state, &path, None));
        },
    );
}

fn choose_project_folder(state: Rc<State>) {
    choose_file(
        &state,
        gtk::FileChooserAction::SelectFolder,
        "Open project",
        "Open",
        |state, path| {
            with_discard_confirmation(&state, move |state| open_project(&state, &path));
        },
    );
}

fn with_discard_confirmation(state: &Rc<State>, callback: impl Fn(Rc<State>) + 'static) {
    if state.save_in_progress.get() {
        show_toast(state, &tr("A document save is still in progress."));
        return;
    }
    if !state.editor.dirty() {
        callback(state.clone());
        return;
    }
    let dialog = gtk::Dialog::builder()
        .title(tr("Unsaved changes"))
        .transient_for(&state.window)
        .modal(true)
        .build();
    dialog.add_button(&tr("Cancel"), gtk::ResponseType::Cancel);
    dialog.add_button(&tr("Discard"), gtk::ResponseType::Reject);
    dialog.add_button(&tr("Save"), gtk::ResponseType::Accept);
    let label = gtk::Label::new(Some(&tr("Save changes before continuing?")));
    label.set_margin_top(24);
    label.set_margin_bottom(24);
    label.set_margin_start(24);
    label.set_margin_end(24);
    dialog.content_area().append(&label);
    let state = state.clone();
    let callback: Rc<dyn Fn(Rc<State>)> = Rc::new(callback);
    dialog.connect_response(move |dialog, response| {
        if response == gtk::ResponseType::Reject {
            callback(state.clone());
            dialog.close();
        } else if response == gtk::ResponseType::Accept {
            if state.current_file.borrow().is_some() {
                let path = state.current_file.borrow().clone().unwrap();
                save_document_to_path(state.clone(), path, Some(callback.clone()));
                dialog.close();
            } else {
                let callback = callback.clone();
                choose_file(
                    &state,
                    gtk::FileChooserAction::Save,
                    "Save document",
                    "Save",
                    move |state, path| {
                        if !project::file_kind(&path).is_text_editable() {
                            show_toast(&state, &tr("Choose a supported text file type."));
                            return;
                        }
                        state.editor.set_file(Some(path.clone()));
                        *state.current_file.borrow_mut() = Some(path.clone());
                        set_project_root(&state, path.parent().map(Path::to_path_buf));
                        update_editor_capabilities(&state);
                        save_document_to_path(state, path, Some(callback.clone()));
                    },
                );
                dialog.close();
            }
        } else {
            dialog.close();
        }
    });
    dialog.present();
}

fn open_document(state: &Rc<State>, path: &Path, root: Option<PathBuf>) {
    let path = path.to_path_buf();
    let root = root.or_else(|| path.parent().map(Path::to_path_buf));
    let kind = project::file_kind(&path);
    if kind == ProjectFileKind::Unsupported {
        show_toast(state, &tr("This file type is not supported in the editor."));
        return;
    }
    let request_id = state.open_request_id.get().wrapping_add(1);
    state.open_request_id.set(request_id);
    state.status.set_label(&tr("Opening document…"));

    if kind.is_previewable() {
        set_active_project_file_context(state, &path, root, request_id);
        state.editor.set_file(Some(path.clone()));
        state.editor.load_text("");
        state.search_entry.set_text("");
        state.search_panel.set_visible(false);
        match kind {
            ProjectFileKind::Image => {
                state
                    .file_pdf_preview
                    .set_message(&tr("Select a PDF file to preview it here."));
                state.image_preview.set_filename(Some(path.as_path()));
            }
            ProjectFileKind::Pdf => {
                state.image_preview.set_filename(None::<&Path>);
                if let Err(error) = state.file_pdf_preview.open(&path) {
                    state
                        .file_pdf_preview
                        .set_message(&tr("Could not open this PDF file."));
                    show_toast(state, &format!("{}: {error}", tr("Could not open the PDF")));
                }
            }
            _ => unreachable!(),
        }
        update_editor_capabilities(state);
        state.status.set_label(&tr("Ready"));
        set_build_status(state, "Ready");
        refresh_project_files(state);
        update_title(state, false);
        show_page(state, "editor");
        return;
    }

    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let result = std::fs::read(&path)
            .map(|bytes| project::decode_text(&bytes))
            .map_err(|error| error.to_string());
        let _ = sender.send((request_id, path, root, result));
    });
    let state = state.clone();
    poll_result(receiver, move |(request_id, path, root, result)| {
        if state.open_request_id.get() != request_id {
            return;
        }
        match result {
            Ok((text, encoding)) => {
                set_active_project_file_context(&state, &path, root, request_id);
                state.file_encoding.set(encoding);
                state.image_preview.set_filename(None::<&Path>);
                state
                    .file_pdf_preview
                    .set_message(&tr("Select a PDF file to preview it here."));
                state.editor.set_file(Some(path));
                state.editor.load_text(&text);
                state.editor.set_mode(EditorMode::Code);
                state.search_entry.set_text("");
                state.search_panel.set_visible(false);
                state.code_mode_button.set_active(true);
                state.visual_mode_button.set_active(false);
                update_editor_capabilities(&state);
                state.status.set_label(&tr("Ready"));
                set_build_status(&state, "Ready");
                state.editor.focus();
                refresh_project_files(&state);
                update_title(&state, false);
                show_page(&state, "editor");
            }
            Err(error) => {
                state.status.set_label(&tr("Ready"));
                show_toast(
                    &state,
                    &format!("{}: {error}", tr("Could not open the document")),
                );
            }
        }
    });
}

fn set_active_project_file_context(
    state: &Rc<State>,
    path: &Path,
    root: Option<PathBuf>,
    request_id: u64,
) {
    let root = root.or_else(|| path.parent().map(Path::to_path_buf));
    let is_tex = project::file_kind(path) == ProjectFileKind::Latex;
    set_project_root(state, root.clone());
    if let Some(root) = root.as_deref() {
        let mut expanded = state.expanded_folders.borrow_mut();
        let mut parent = path.parent();
        while let Some(folder) = parent {
            if folder == root {
                break;
            }
            expanded.insert(folder.to_path_buf());
            parent = folder.parent();
        }
    }
    *state.current_file.borrow_mut() = Some(path.to_path_buf());
    if is_tex {
        *state.main_tex.borrow_mut() = Some(path.to_path_buf());
    } else {
        let main_is_available = state.main_tex.borrow().as_deref().is_some_and(|main| {
            root.as_deref()
                .is_some_and(|folder| main.starts_with(folder))
        });
        if !main_is_available {
            *state.main_tex.borrow_mut() = None;
            if let Some(root) = root.clone() {
                find_project_main_async(state, root, path.to_path_buf(), request_id);
            }
        }
    }
    state.project_target.replace(root);
}

fn set_project_root(state: &Rc<State>, root: Option<PathBuf>) {
    let root_changed = state.project_root.borrow().as_ref() != root.as_ref();
    if root_changed {
        state.expanded_folders.borrow_mut().clear();
        state.project_files_scroll.set_value(0.0);
    }
    *state.project_root.borrow_mut() = root;
}

fn find_project_main_async(
    state: &Rc<State>,
    root: PathBuf,
    current_file: PathBuf,
    request_id: u64,
) {
    let root_for_worker = root.clone();
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let main = project::find_tex_files(&root_for_worker)
            .ok()
            .and_then(|documents| documents.first().cloned());
        let _ = sender.send(main);
    });
    let state = state.clone();
    poll_result(receiver, move |main| {
        if state.open_request_id.get() != request_id
            || state.current_file.borrow().as_deref() != Some(current_file.as_path())
            || state.project_root.borrow().as_deref() != Some(root.as_path())
        {
            return;
        }
        *state.main_tex.borrow_mut() = main;
        refresh_build_status(&state);
    });
}

fn open_project(state: &Rc<State>, folder: &Path) {
    let folder = folder.to_path_buf();
    set_project_root(state, Some(folder.clone()));
    *state.project_target.borrow_mut() = Some(folder.clone());
    refresh_build_status(state);
    state.expanded_folders.borrow_mut().clear();
    state.project_files_scroll.set_value(0.0);
    refresh_project_files(state);
    let request_id = state.project_open_id.get().wrapping_add(1);
    state.project_open_id.set(request_id);
    state.status.set_label(&tr("Loading project…"));
    let folder_for_worker = folder.clone();
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let documents =
            project::find_tex_files(&folder_for_worker).map_err(|error| error.to_string());
        let _ = sender.send((request_id, folder_for_worker, documents));
    });
    let state = state.clone();
    poll_result(receiver, move |(request_id, folder, result)| {
        if state.project_open_id.get() != request_id
            || state.project_root.borrow().as_deref() != Some(folder.as_path())
        {
            return;
        }
        state.status.set_label(&tr("Ready"));
        let documents = match result {
            Ok(documents) => documents,
            Err(error) => {
                show_toast(
                    &state,
                    &format!("{}: {error}", tr("Could not list the project folder")),
                );
                Vec::new()
            }
        };
        let root_main = folder.join("main.tex");
        let folder_named_main = folder.file_name().map(|name| {
            let mut file_name = name.to_os_string();
            file_name.push(".tex");
            folder.join(file_name)
        });
        let main = documents
            .contains(&root_main)
            .then_some(root_main)
            .or_else(|| folder_named_main.filter(|path| documents.contains(path)))
            .or_else(|| {
                state
                    .main_tex
                    .borrow()
                    .clone()
                    .filter(|path| documents.contains(path))
            })
            .or_else(|| (documents.len() == 1).then(|| documents[0].clone()));
        *state.main_tex.borrow_mut() = main.clone();
        if let Some(main) = main {
            open_document(&state, &main, Some(folder));
        } else if documents.len() > 1 {
            choose_main_document(&state, &folder, documents);
        } else {
            show_page(&state, "editor");
            show_toast(&state, &tr("This project has no .tex document yet."));
        }
    });
}

fn choose_main_document(state: &Rc<State>, folder: &Path, documents: Vec<PathBuf>) {
    let dialog = gtk::Dialog::builder()
        .title(tr("Choose main document"))
        .transient_for(&state.window)
        .modal(true)
        .default_width(520)
        .default_height(500)
        .build();
    dialog.add_button(&tr("Cancel"), gtk::ResponseType::Cancel);
    dialog.add_button(&tr("Open"), gtk::ResponseType::Accept);
    dialog.set_default_response(gtk::ResponseType::Accept);
    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::Single);
    list.add_css_class("boxed-list");
    for document in &documents {
        let row = gtk::ListBoxRow::new();
        row.set_activatable(true);
        let label = gtk::Label::new(Some(
            &document
                .strip_prefix(folder)
                .unwrap_or(document)
                .display()
                .to_string(),
        ));
        label.set_xalign(0.0);
        label.set_margin_top(10);
        label.set_margin_bottom(10);
        label.set_margin_start(12);
        label.set_margin_end(12);
        row.set_child(Some(&label));
        list.append(&row);
    }
    list.select_row(list.row_at_index(0).as_ref());
    let scroll = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .child(&list)
        .build();
    scroll.set_margin_top(12);
    scroll.set_margin_bottom(12);
    scroll.set_margin_start(12);
    scroll.set_margin_end(12);
    dialog.content_area().append(&scroll);
    let dialog_for_row = dialog.clone();
    list.connect_row_activated(move |_, _| dialog_for_row.response(gtk::ResponseType::Accept));
    let state = state.clone();
    let folder = folder.to_path_buf();
    dialog.connect_response(move |dialog, response| {
        let selected = if response == gtk::ResponseType::Accept {
            list.selected_row()
                .and_then(|row| documents.get(row.index() as usize))
                .cloned()
        } else {
            None
        };
        dialog.close();
        if let Some(document) = selected {
            open_document(&state, &document, Some(folder.clone()));
        }
    });
    dialog.present();
}

fn save_document(state: Rc<State>) {
    let current_file = state.current_file.borrow().clone();
    if let Some(path) = current_file {
        if project::file_kind(&path).is_previewable() {
            return;
        }
        if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("tex"))
        {
            *state.main_tex.borrow_mut() = Some(path.clone());
        }
        save_document_to_path(state, path, None);
        return;
    }
    choose_file(
        &state,
        gtk::FileChooserAction::Save,
        "Save document",
        "Save",
        |state, path| {
            if !project::file_kind(&path).is_text_editable() {
                show_toast(&state, &tr("Choose a supported text file type."));
                return;
            }
            state.editor.set_file(Some(path.clone()));
            *state.current_file.borrow_mut() = Some(path.clone());
            set_project_root(&state, path.parent().map(Path::to_path_buf));
            if path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("tex"))
            {
                *state.main_tex.borrow_mut() = Some(path.clone());
            }
            update_editor_capabilities(&state);
            save_document_to_path(state, path, None);
        },
    );
}

fn save_document_to_path(state: Rc<State>, path: PathBuf, after: Option<SaveCallback>) {
    if !project::file_kind(&path).is_text_editable() {
        show_toast(
            &state,
            &tr("This file type cannot be saved from the text editor."),
        );
        return;
    }
    if state.save_in_progress.replace(true) {
        show_toast(&state, &tr("A document save is still in progress."));
        return;
    }
    let source = state.editor.text();
    let encoding = state.file_encoding.get();
    state.status.set_label(&tr("Saving…"));
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let result = project::encode_text(&source, encoding).and_then(|bytes| {
            crate::storage::atomic_write(&path, &bytes).map_err(|error| error.to_string())
        });
        let _ = sender.send((result, path, source));
    });
    poll_result(receiver, move |(result, path, source)| {
        state.save_in_progress.set(false);
        state.status.set_label(&tr("Ready"));
        match result {
            Ok(()) => {
                state.editor.mark_saved(&source);
                refresh_project_files(&state);
                if state.editor.dirty() {
                    if let Some(after) = after {
                        save_document_to_path(state, path, Some(after));
                    } else {
                        update_title(&state, true);
                        show_toast(
                            &state,
                            &tr("The document changed while it was being saved. Save again."),
                        );
                    }
                    return;
                }
                update_title(&state, false);
                if let Some(after) = after {
                    after(state);
                } else {
                    show_toast(&state, &tr("Document saved"));
                }
            }
            Err(error) => show_toast(&state, &error),
        }
    });
}

fn setup_project_file_view(state: &Rc<State>) {
    let factory = gtk::SignalListItemFactory::new();
    let state_for_setup = state.clone();
    factory.connect_setup(move |_, object| {
        let Some(item) = object.downcast_ref::<gtk::ListItem>() else {
            return;
        };
        let expander = gtk::TreeExpander::new();
        expander.set_hexpand(true);
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        row.set_hexpand(true);
        row.set_margin_start(4);
        row.set_margin_end(8);
        row.set_margin_top(3);
        row.set_margin_bottom(3);
        row.add_css_class("ovenbird-project-file-row");
        let icon = gtk::Image::new();
        icon.set_pixel_size(16);
        let label = gtk::Label::new(None);
        label.set_xalign(0.0);
        label.set_hexpand(true);
        label.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
        label.add_css_class("ovenbird-project-file-name");
        row.append(&icon);
        row.append(&label);
        expander.set_child(Some(&row));
        item.set_child(Some(&expander));
        install_project_row_controllers(&row, item, &state_for_setup);
    });

    let state_for_bind = state.clone();
    factory.connect_bind(move |_, object| {
        let Some(item) = object.downcast_ref::<gtk::ListItem>() else {
            return;
        };
        let Some(tree_row) = item
            .item()
            .and_then(|item| item.downcast::<gtk::TreeListRow>().ok())
        else {
            return;
        };
        let Some(entry) = project_entry_from_tree_row(&tree_row) else {
            return;
        };
        let Some(expander) = item
            .child()
            .and_then(|child| child.downcast::<gtk::TreeExpander>().ok())
        else {
            return;
        };
        expander.set_list_row(Some(&tree_row));
        let Some(row) = expander
            .child()
            .and_then(|child| child.downcast::<gtk::Box>().ok())
        else {
            return;
        };
        let Some(icon) = row
            .first_child()
            .and_then(|child| child.downcast::<gtk::Image>().ok())
        else {
            return;
        };
        let Some(label) = icon
            .next_sibling()
            .and_then(|child| child.downcast::<gtk::Label>().ok())
        else {
            return;
        };
        label.set_text(&entry.name);
        let is_directory = entry.kind == ProjectEntryKind::Directory;
        if is_directory {
            row.add_css_class("ovenbird-project-drop-target");
        } else {
            row.remove_css_class("ovenbird-project-drop-target");
        }
        icon.set_icon_name(Some(project_file_icon(&entry, tree_row.is_expanded())));
        label.remove_css_class("dim-label");
        if entry.kind == ProjectEntryKind::Resource {
            label.add_css_class("dim-label");
            row.set_tooltip_text(Some(&tr(
                "This file is listed as a project resource and cannot be edited in Ovenbird yet.",
            )));
        } else if entry.kind == ProjectEntryKind::Previewable {
            row.set_tooltip_text(Some(&tr("Open preview in the editor pane.")));
        } else {
            row.set_tooltip_text(None);
        }
        if is_directory {
            let path = entry.path.clone();
            let state = state_for_bind.clone();
            let item_weak = item.downgrade();
            let icon = icon.clone();
            tree_row.connect_expanded_notify(move |tree_row| {
                if tree_row.is_expanded() {
                    state.expanded_folders.borrow_mut().insert(path.clone());
                } else {
                    state.expanded_folders.borrow_mut().remove(&path);
                }
                if item_weak
                    .upgrade()
                    .and_then(|item| project_entry_from_list_item(&item))
                    .is_some_and(|entry| entry.path == path)
                {
                    icon.set_icon_name(Some(if tree_row.is_expanded() {
                        "folder-open-symbolic"
                    } else {
                        "folder-symbolic"
                    }));
                }
            });
        }
    });
    state.file_tree.set_factory(Some(&factory));

    let state_for_activate = state.clone();
    state.file_tree.connect_activate(move |_, position| {
        let Some(model) = state_for_activate
            .file_tree_selection
            .model()
            .and_then(|model| model.downcast::<gtk::TreeListModel>().ok())
        else {
            return;
        };
        let Some(tree_row) = model.row(position) else {
            return;
        };
        let Some(entry) = project_entry_from_tree_row(&tree_row) else {
            return;
        };
        let target = if entry.kind == ProjectEntryKind::Directory {
            entry.path.clone()
        } else {
            entry.path.parent().unwrap_or(&entry.path).to_path_buf()
        };
        *state_for_activate.project_target.borrow_mut() = Some(target);
        if entry.kind == ProjectEntryKind::Directory || entry.kind == ProjectEntryKind::Resource {
            return;
        }
        let path = entry.path.clone();
        let root = state_for_activate.project_root.borrow().clone();
        with_discard_confirmation(&state_for_activate, move |state| {
            open_document(&state, &path, root.clone())
        });
    });

    let state_for_selection = state.clone();
    state
        .file_tree_selection
        .connect_selected_notify(move |selection| {
            let Some(tree_row) = selection
                .selected_item()
                .and_then(|item| item.downcast::<gtk::TreeListRow>().ok())
            else {
                return;
            };
            let Some(entry) = project_entry_from_tree_row(&tree_row) else {
                return;
            };
            let target = if entry.kind == ProjectEntryKind::Directory {
                entry.path
            } else {
                entry.path.parent().unwrap_or(&entry.path).to_path_buf()
            };
            *state_for_selection.project_target.borrow_mut() = Some(target);
        });
}

fn install_project_row_controllers(row: &gtk::Box, item: &gtk::ListItem, state: &Rc<State>) {
    let context_gesture = gtk::GestureClick::new();
    context_gesture.set_button(gdk::BUTTON_SECONDARY);
    let item_weak = item.downgrade();
    let row_weak = row.downgrade();
    let state_for_context = state.clone();
    context_gesture.connect_pressed(move |_, _, _, _| {
        let (Some(item), Some(row)) = (item_weak.upgrade(), row_weak.upgrade()) else {
            return;
        };
        let Some(entry) = project_entry_from_list_item(&item) else {
            return;
        };
        show_project_file_context_menu(&state_for_context, &row, &entry.path);
    });
    row.add_controller(context_gesture);

    let drag = gtk::DragSource::builder()
        .actions(gdk::DragAction::MOVE)
        .build();
    let item_weak = item.downgrade();
    drag.connect_prepare(move |_, _, _| {
        let item = item_weak.upgrade()?;
        let entry = project_entry_from_list_item(&item)?;
        let value = entry.path.to_string_lossy().to_string().to_value();
        Some(gdk::ContentProvider::for_value(&value))
    });
    row.add_controller(drag);

    let drop_target = gtk::DropTarget::new(String::static_type(), gdk::DragAction::MOVE);
    let item_weak = item.downgrade();
    let state_for_drop = state.clone();
    drop_target.connect_drop(move |_, value, _, _| {
        let Some(item) = item_weak.upgrade() else {
            return false;
        };
        let Some(entry) = project_entry_from_list_item(&item) else {
            return false;
        };
        if entry.kind != ProjectEntryKind::Directory {
            return false;
        }
        let Ok(source) = value.get::<String>() else {
            return false;
        };
        move_project_file_to(&state_for_drop, PathBuf::from(source), entry.path);
        true
    });
    row.add_controller(drop_target);
}

fn project_entry_from_list_item(item: &gtk::ListItem) -> Option<project::ProjectEntry> {
    let tree_row = item.item()?.downcast::<gtk::TreeListRow>().ok()?;
    project_entry_from_tree_row(&tree_row)
}

fn project_entry_from_tree_row(row: &gtk::TreeListRow) -> Option<project::ProjectEntry> {
    let item = row.item()?.downcast::<glib::BoxedAnyObject>().ok()?;
    let entry = item.borrow::<ProjectTreeNode>().entry.clone();
    Some(entry)
}

fn project_file_icon(entry: &project::ProjectEntry, expanded: bool) -> &'static str {
    if entry.kind == ProjectEntryKind::Directory {
        return if expanded {
            "folder-open-symbolic"
        } else {
            "folder-symbolic"
        };
    }
    match project::file_kind(&entry.path) {
        ProjectFileKind::Image => "image-x-generic-symbolic",
        ProjectFileKind::Pdf => "x-office-document-symbolic",
        _ => "text-x-generic-symbolic",
    }
}

fn build_project_tree_store(entries: &[project::ProjectEntry]) -> gio::ListStore {
    fn append_children(
        entries: &[project::ProjectEntry],
        position: &mut usize,
        depth: usize,
        store: &gio::ListStore,
    ) {
        while let Some(entry) = entries.get(*position) {
            if entry.depth != depth {
                break;
            }
            *position += 1;
            let children = if entry.kind == ProjectEntryKind::Directory {
                let children = gio::ListStore::new::<glib::BoxedAnyObject>();
                append_children(entries, position, depth + 1, &children);
                Some(children)
            } else {
                None
            };
            store.append(&glib::BoxedAnyObject::new(ProjectTreeNode {
                entry: entry.clone(),
                children,
            }));
        }
    }

    let store = gio::ListStore::new::<glib::BoxedAnyObject>();
    let mut position = 0;
    append_children(entries, &mut position, 0, &store);
    store
}

fn restore_expanded_project_folders(
    model: &gtk::TreeListModel,
    expanded_folders: &HashSet<PathBuf>,
) {
    let mut position = 0;
    while position < model.n_items() {
        let Some(row) = model.row(position) else {
            break;
        };
        if row.is_expandable()
            && project_entry_from_tree_row(&row)
                .is_some_and(|entry| expanded_folders.contains(&entry.path))
        {
            row.set_expanded(true);
        }
        position += 1;
    }
}

fn refresh_project_files(state: &Rc<State>) {
    let scroll_position = state.project_files_scroll.value();
    let request_id = state.project_scan_id.get().wrapping_add(1);
    state.project_scan_id.set(request_id);
    while let Some(child) = state.file_list.first_child() {
        state.file_list.remove(&child);
    }
    let Some(root) = state.project_root.borrow().clone() else {
        state.file_tree_selection.set_model(None::<&gio::ListModel>);
        let empty = gtk::Label::new(Some(&tr("Open a project folder to browse its files.")));
        empty.set_xalign(0.0);
        empty.set_wrap(true);
        empty.add_css_class("dim-label");
        state.file_list.append(&empty);
        state.project_files_scroll.set_value(0.0);
        return;
    };
    let loading = gtk::Label::new(Some(&tr("Loading project files…")));
    loading.set_xalign(0.0);
    loading.add_css_class("dim-label");
    state.file_list.append(&loading);
    let root_for_worker = root.clone();
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let entries = project::entries(&root_for_worker).map_err(|error| error.to_string());
        let _ = sender.send((request_id, root_for_worker, entries));
    });
    let state = state.clone();
    poll_result(receiver, move |(request_id, root, result)| {
        if state.project_scan_id.get() != request_id
            || state.project_root.borrow().as_deref() != Some(root.as_path())
        {
            return;
        }
        while let Some(child) = state.file_list.first_child() {
            state.file_list.remove(&child);
        }
        match result {
            Ok(entries) if entries.is_empty() => {
                state.file_tree_selection.set_model(None::<&gio::ListModel>);
                let empty = gtk::Label::new(Some(&tr(
                    "This project has no files yet. Use + to create one.",
                )));
                empty.set_xalign(0.0);
                empty.set_wrap(true);
                empty.add_css_class("dim-label");
                state.file_list.append(&empty);
            }
            Ok(entries) => {
                let root_store = build_project_tree_store(&entries);
                let tree_model = gtk::TreeListModel::new(root_store, false, false, |item| {
                    let node = item.downcast_ref::<glib::BoxedAnyObject>()?;
                    let node = node.borrow::<ProjectTreeNode>();
                    node.children
                        .as_ref()
                        .filter(|children| children.n_items() > 0)
                        .map(|children| children.clone().upcast())
                });
                state.file_tree_selection.set_model(Some(&tree_model));
                restore_expanded_project_folders(&tree_model, &state.expanded_folders.borrow());
                if let Some(current_file) = state.current_file.borrow().as_ref() {
                    for position in 0..tree_model.n_items() {
                        let Some(row) = tree_model.row(position) else {
                            continue;
                        };
                        if project_entry_from_tree_row(&row)
                            .is_some_and(|entry| entry.path == *current_file)
                        {
                            state.file_tree_selection.set_selected(position);
                            break;
                        }
                    }
                }
                state.file_list.append(&state.file_tree);
            }
            Err(error) => {
                state.file_tree_selection.set_model(None::<&gio::ListModel>);
                let message = gtk::Label::new(Some(&format!(
                    "{}: {error}",
                    tr("Could not list the project folder")
                )));
                message.set_xalign(0.0);
                message.set_wrap(true);
                message.add_css_class("dim-label");
                state.file_list.append(&message);
                show_toast(
                    &state,
                    &format!("{}: {error}", tr("Could not list the project folder")),
                );
            }
        }
        let adjustment = state.project_files_scroll.clone();
        glib::idle_add_local_once(move || adjustment.set_value(scroll_position));
    });
}

fn show_project_file_context_menu(state: &Rc<State>, anchor: &gtk::Box, path: &Path) {
    let file = path.to_path_buf();
    let is_directory = file.is_dir();
    let target = if is_directory {
        file.clone()
    } else {
        file.parent().unwrap_or(&file).to_path_buf()
    };
    *state.project_target.borrow_mut() = Some(target.clone());
    let popover = gtk::Popover::new();
    let menu = gtk::Box::new(gtk::Orientation::Vertical, 2);

    if is_directory {
        for (icon, label, extension) in [
            ("text-x-generic-symbolic", "New LaTeX document", "tex"),
            ("text-x-generic-symbolic", "New bibliography (.bib)", "bib"),
            ("text-x-generic-symbolic", "New style file (.sty)", "sty"),
        ] {
            let state = state.clone();
            let target = target.clone();
            append_file_context_action(&menu, &popover, icon, &tr(label), move || {
                *state.project_target.borrow_mut() = Some(target);
                create_project_file(&state, extension);
            });
        }
        let state = state.clone();
        let target = target.clone();
        append_file_context_action(
            &menu,
            &popover,
            "folder-new-symbolic",
            &tr("New folder"),
            move || {
                *state.project_target.borrow_mut() = Some(target);
                create_project_folder(state);
            },
        );
        menu.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    } else if project::openable_extension(&file) {
        let state = state.clone();
        let file = file.clone();
        let root = state.project_root.borrow().clone();
        append_file_context_action(
            &menu,
            &popover,
            "document-open-symbolic",
            &tr("Open"),
            move || {
                let file = file.clone();
                with_discard_confirmation(&state, move |state| {
                    open_document(&state, &file, root.clone())
                });
            },
        );
        menu.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    } else {
        let file = file.clone();
        let state = state.clone();
        append_file_context_action(
            &menu,
            &popover,
            "document-open-symbolic",
            &tr("Open with default application"),
            move || open_with_default_application(&state, &file),
        );
        menu.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    }

    let file_for_copy = file.clone();
    let state_for_copy = state.clone();
    append_file_context_action(
        &menu,
        &popover,
        "edit-copy-symbolic",
        &tr("Copy path"),
        move || {
            if let Some(display) = gdk::Display::default() {
                display
                    .clipboard()
                    .set_text(&file_for_copy.to_string_lossy());
            } else {
                show_toast(&state_for_copy, &tr("Could not access the clipboard."));
            }
        },
    );
    let file_for_parent = file.clone();
    let state_for_parent = state.clone();
    append_file_context_action(
        &menu,
        &popover,
        "folder-open-symbolic",
        &tr("Show in Files"),
        move || open_containing_folder(&state_for_parent, &file_for_parent),
    );
    menu.append(&gtk::Separator::new(gtk::Orientation::Horizontal));

    let state_for_rename = state.clone();
    let file_for_rename = file.clone();
    append_file_context_action(
        &menu,
        &popover,
        "document-edit-symbolic",
        &tr("Rename"),
        move || rename_project_file(&state_for_rename, &file_for_rename),
    );
    let state_for_trash = state.clone();
    let file_for_trash = file.clone();
    append_file_context_action(
        &menu,
        &popover,
        "user-trash-symbolic",
        &tr("Move to Trash"),
        move || trash_project_file(&state_for_trash, &file_for_trash),
    );
    popover.set_child(Some(&menu));
    popover.set_parent(anchor);
    popover.popup();
}

fn append_file_context_action(
    menu: &gtk::Box,
    popover: &gtk::Popover,
    icon: &str,
    label: &str,
    callback: impl FnOnce() + 'static,
) {
    let item = labeled_button(icon, label);
    item.set_halign(gtk::Align::Fill);
    item.set_has_frame(false);
    item.add_css_class("flat");
    let popover = popover.clone();
    let callback = RefCell::new(Some(callback));
    item.connect_clicked(move |_| {
        popover.popdown();
        if let Some(callback) = callback.borrow_mut().take() {
            callback();
        }
    });
    menu.append(&item);
}

fn open_with_default_application(state: &Rc<State>, path: &Path) {
    let uri = gio::File::for_path(path).uri();
    if let Err(error) = gio::AppInfo::launch_default_for_uri(&uri, None::<&gio::AppLaunchContext>) {
        show_toast(state, &error.to_string());
    }
}

fn open_containing_folder(state: &Rc<State>, path: &Path) {
    let folder = if path.is_dir() {
        path
    } else if let Some(parent) = path.parent() {
        parent
    } else {
        show_toast(state, &tr("Could not find the containing folder."));
        return;
    };
    let uri = gio::File::for_path(folder).uri();
    if let Err(error) = gio::AppInfo::launch_default_for_uri(&uri, None::<&gio::AppLaunchContext>) {
        show_toast(state, &error.to_string());
    }
}

fn install_root_drop_target(state: &Rc<State>) {
    let target = gtk::DropTarget::new(String::static_type(), gdk::DragAction::MOVE);
    let weak = Rc::downgrade(state);
    target.connect_drop(move |_, value, _, _| {
        let Ok(path) = value.get::<String>() else {
            return false;
        };
        let Some(state) = weak.upgrade() else {
            return false;
        };
        let Some(root) = state.project_root.borrow().clone() else {
            return false;
        };
        move_project_file_to(&state, PathBuf::from(path), root)
    });
    state.file_list.add_controller(target);
}

fn move_project_file_to(state: &Rc<State>, source: PathBuf, destination: PathBuf) -> bool {
    let Some(project_root) = state.project_root.borrow().clone() else {
        return false;
    };
    let destination_for_result = destination.clone();
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let result = project::move_file_within_project(&source, &destination, &project_root);
        let _ = sender.send((source, result));
    });
    let state_for_result = state.clone();
    poll_result(receiver, move |(source, result)| match result {
        Ok(new_path) => {
            let root = state_for_result.project_root.borrow().clone();
            if root.as_deref() != Some(destination_for_result.as_path()) {
                state_for_result
                    .expanded_folders
                    .borrow_mut()
                    .insert(destination_for_result.clone());
            }
            update_project_file_path(&state_for_result, &source, &new_path);
        }
        Err(error) => show_toast(&state_for_result, &error),
    });
    true
}

fn update_project_file_path(state: &Rc<State>, old_path: &Path, new_path: &Path) {
    let current = state.current_file.borrow().clone();
    let rewritten_current = current
        .as_ref()
        .filter(|path| project::path_contains(old_path, path))
        .map(|path| project::rewrite_path_prefix(path, old_path, new_path));
    if let Some(path) = rewritten_current {
        *state.current_file.borrow_mut() = Some(path.clone());
        state.editor.set_file(Some(path));
        update_title(state, state.editor.dirty());
    }
    let main = state.main_tex.borrow().clone();
    if let Some(main) = main.filter(|path| project::path_contains(old_path, path)) {
        let rewritten = project::rewrite_path_prefix(&main, old_path, new_path);
        *state.main_tex.borrow_mut() = rewritten
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("tex"))
            .then_some(rewritten);
    }
    let target = state.project_target.borrow().clone();
    if let Some(target) = target.filter(|path| project::path_contains(old_path, path)) {
        *state.project_target.borrow_mut() =
            Some(project::rewrite_path_prefix(&target, old_path, new_path));
    }
    let expanded = std::mem::take(&mut *state.expanded_folders.borrow_mut());
    *state.expanded_folders.borrow_mut() = expanded
        .into_iter()
        .map(|folder| {
            if project::path_contains(old_path, &folder) {
                project::rewrite_path_prefix(&folder, old_path, new_path)
            } else {
                folder
            }
        })
        .collect();
    update_editor_capabilities(state);
    refresh_project_files(state);
    refresh_build_status(state);
}

fn indent_selection(state: &Rc<State>, increase: bool) {
    let selected = state.editor.selected_text();
    if selected.is_empty() {
        return;
    }
    let indent = if increase { "    " } else { "" };
    let text = selected
        .lines()
        .map(|line| {
            if increase {
                format!("{indent}{line}")
            } else {
                line.strip_prefix("    ")
                    .or_else(|| line.strip_prefix('\t'))
                    .unwrap_or(line)
                    .to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    let insertion = LatexInsertion {
        text: text.clone(),
        cursor_offset: text.chars().count(),
        selection: Some((0, text.chars().count())),
    };
    state.editor.insert_latex(
        &insertion,
        if increase {
            "indent:more"
        } else {
            "indent:less"
        },
    );
}

fn insert_snippet(state: &Rc<State>, insertion: LatexInsertion) {
    state.editor.insert_latex(&insertion, "insert:math");
}

fn create_project_file(state: &Rc<State>, extension: &str) {
    let extension = extension.to_owned();
    with_discard_confirmation(state, move |state| {
        create_project_file_confirmed(&state, &extension)
    });
}

fn create_project_file_confirmed(state: &Rc<State>, extension: &str) {
    let extension = extension.to_owned();
    if let Some(folder) = state
        .project_target
        .borrow()
        .clone()
        .or_else(|| state.project_root.borrow().clone())
    {
        prompt_create_file(state, &folder, &extension);
    } else {
        choose_file(
            state,
            gtk::FileChooserAction::SelectFolder,
            "Choose a project folder",
            "Open",
            move |state, folder| {
                set_project_root(&state, Some(folder.clone()));
                *state.project_target.borrow_mut() = Some(folder.clone());
                prompt_create_file(&state, &folder, &extension);
            },
        );
    }
}

fn prompt_create_file(state: &Rc<State>, folder: &Path, extension: &str) {
    let (title, default_name, contents) = match extension {
        "tex" => (
            "New LaTeX document",
            "document.tex",
            LatexEditor::new_document_text(),
        ),
        "bib" => ("New bibliography (.bib)", "references.bib", String::new()),
        "sty" => ("New style file (.sty)", "styles.sty", String::new()),
        _ => return,
    };
    let dialog = gtk::Dialog::builder()
        .title(tr_dynamic(title))
        .transient_for(&state.window)
        .modal(true)
        .build();
    dialog.add_button(&tr("Cancel"), gtk::ResponseType::Cancel);
    dialog.add_button(&tr("Create"), gtk::ResponseType::Accept);
    dialog.set_default_response(gtk::ResponseType::Accept);
    let content = dialog.content_area();
    content.set_spacing(10);
    content.set_margin_top(16);
    content.set_margin_bottom(16);
    content.set_margin_start(20);
    content.set_margin_end(20);
    content.append(&gtk::Label::new(Some(&tr(
        "File name in the selected folder",
    ))));
    let name = gtk::Entry::new();
    name.set_text(default_name);
    name.set_activates_default(true);
    content.append(&name);
    let name_for_response = name.clone();
    let folder = folder.to_path_buf();
    let extension = extension.to_owned();
    let contents = Rc::new(contents);
    let state2 = state.clone();
    dialog.connect_response(move |dialog, response| {
        if response != gtk::ResponseType::Accept {
            dialog.close();
            return;
        }
        let mut file_name = name_for_response.text().trim().to_owned();
        if !valid_name(&file_name) {
            show_toast(&state2, &tr("Enter a valid file name"));
            return;
        }
        if !file_name
            .to_ascii_lowercase()
            .ends_with(&format!(".{extension}"))
        {
            file_name.push_str(&format!(".{extension}"));
        }
        let path = folder.join(file_name);
        let path_for_worker = path.clone();
        let contents = contents.as_ref().clone();
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            use std::io::Write;
            let mut created = false;
            let result = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path_for_worker)
                .and_then(|mut file| {
                    created = true;
                    file.write_all(contents.as_bytes())
                });
            if created && result.is_err() {
                let _ = std::fs::remove_file(&path_for_worker);
            }
            let _ = sender.send(result.map_err(|error| error.to_string()));
        });
        let dialog_for_result = dialog.clone();
        let state_for_result = state2.clone();
        let extension_for_result = extension.clone();
        let project_root = state2
            .project_root
            .borrow()
            .clone()
            .unwrap_or(folder.clone());
        poll_result(receiver, move |result| match result {
            Ok(()) => {
                if extension_for_result == "tex" {
                    *state_for_result.main_tex.borrow_mut() = Some(path.clone());
                }
                open_document(&state_for_result, &path, Some(project_root));
                show_toast(&state_for_result, &tr("File created"));
                dialog_for_result.close();
            }
            Err(error) => show_toast(
                &state_for_result,
                &format!("{}: {error}", tr("Could not create the file")),
            ),
        });
    });
    dialog.present();
    name.grab_focus();
}

fn create_project_folder(state: Rc<State>) {
    let folder = state
        .project_target
        .borrow()
        .clone()
        .or_else(|| state.project_root.borrow().clone());
    if let Some(folder) = folder {
        prompt_create_folder(&state, &folder);
    } else {
        choose_file(
            &state,
            gtk::FileChooserAction::SelectFolder,
            "Choose a project folder",
            "Open",
            |state, folder| {
                set_project_root(&state, Some(folder.clone()));
                *state.project_target.borrow_mut() = Some(folder.clone());
                prompt_create_folder(&state, &folder);
            },
        );
    }
}

fn prompt_create_folder(state: &Rc<State>, parent: &Path) {
    let dialog = gtk::Dialog::builder()
        .title(tr("New folder"))
        .transient_for(&state.window)
        .modal(true)
        .build();
    dialog.add_button(&tr("Cancel"), gtk::ResponseType::Cancel);
    dialog.add_button(&tr("Create"), gtk::ResponseType::Accept);
    dialog.set_default_response(gtk::ResponseType::Accept);
    let entry = gtk::Entry::new();
    entry.set_placeholder_text(Some(&tr("Folder name")));
    entry.set_activates_default(true);
    entry.set_margin_top(18);
    entry.set_margin_bottom(18);
    entry.set_margin_start(20);
    entry.set_margin_end(20);
    dialog.content_area().append(&entry);
    let entry_for_response = entry.clone();
    let parent = parent.to_path_buf();
    let state2 = state.clone();
    dialog.connect_response(move |dialog, response| {
        if response != gtk::ResponseType::Accept {
            dialog.close();
            return;
        }
        let name = entry_for_response.text().trim().to_owned();
        if !valid_name(&name) {
            show_toast(&state2, &tr("Enter a valid folder name"));
            return;
        }
        let folder = parent.join(name);
        let folder_for_worker = folder.clone();
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = sender
                .send(std::fs::create_dir(&folder_for_worker).map_err(|error| error.to_string()));
        });
        let dialog_for_result = dialog.clone();
        let state_for_result = state2.clone();
        let parent_for_result = parent.clone();
        poll_result(receiver, move |result| match result {
            Ok(()) => {
                state_for_result
                    .expanded_folders
                    .borrow_mut()
                    .insert(parent_for_result.clone());
                *state_for_result.project_target.borrow_mut() = Some(folder);
                refresh_project_files(&state_for_result);
                show_toast(&state_for_result, &tr("Folder created"));
                dialog_for_result.close();
            }
            Err(error) => show_toast(
                &state_for_result,
                &format!("{}: {error}", tr("Could not create the folder")),
            ),
        });
    });
    dialog.present();
    entry.grab_focus();
}

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.trim() == name
        && name != "."
        && name != ".."
        && !name
            .chars()
            .any(|ch| ch == '/' || ch == '\\' || ch.is_control())
}

fn rename_project_file(state: &Rc<State>, path: &Path) {
    let dialog = gtk::Dialog::builder()
        .title(tr("Rename"))
        .transient_for(&state.window)
        .modal(true)
        .build();
    dialog.add_button(&tr("Cancel"), gtk::ResponseType::Cancel);
    dialog.add_button(&tr("Rename"), gtk::ResponseType::Accept);
    let entry = gtk::Entry::new();
    entry.set_text(
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default(),
    );
    entry.set_activates_default(true);
    entry.set_margin_top(18);
    entry.set_margin_bottom(18);
    entry.set_margin_start(20);
    entry.set_margin_end(20);
    dialog.content_area().append(&entry);
    let entry_for_response = entry.clone();
    let path = path.to_path_buf();
    let root = state.project_root.borrow().clone();
    let state = state.clone();
    dialog.connect_response(move |dialog, response| {
        if response == gtk::ResponseType::Accept {
            let Some(root) = root.clone() else {
                dialog.close();
                return;
            };
            let path = path.clone();
            let new_name = entry_for_response.text().to_string();
            let (sender, receiver) = mpsc::channel();
            std::thread::spawn(move || {
                let result = project::rename_project_file(&path, &new_name, &root);
                let _ = sender.send((path, result));
            });
            let state_for_result = state.clone();
            poll_result(receiver, move |(old_path, result)| match result {
                Ok(new_path) => update_project_file_path(&state_for_result, &old_path, &new_path),
                Err(error) => show_toast(&state_for_result, &error),
            });
        }
        dialog.close();
    });
    dialog.present();
    entry.grab_focus();
}

fn trash_project_file(state: &Rc<State>, path: &Path) {
    let path = path.to_path_buf();
    let active_file_is_being_removed = state
        .current_file
        .borrow()
        .as_ref()
        .is_some_and(|current| project::path_contains(&path, current));
    if active_file_is_being_removed && state.editor.dirty() {
        with_discard_confirmation(state, move |state| trash_project_file_now(&state, &path));
    } else {
        trash_project_file_now(state, &path);
    }
}

fn trash_project_file_now(state: &Rc<State>, path: &Path) {
    let file = gio::File::for_path(path);
    let path = path.to_path_buf();
    let state = state.clone();
    file.trash_async(
        glib::Priority::DEFAULT,
        None::<&gio::Cancellable>,
        move |result| match result {
            Ok(()) => {
                let current_removed = state
                    .current_file
                    .borrow()
                    .as_ref()
                    .is_some_and(|current| project::path_contains(&path, current));
                let main_removed = state
                    .main_tex
                    .borrow()
                    .as_ref()
                    .is_some_and(|main| project::path_contains(&path, main));
                if current_removed {
                    state.current_file.borrow_mut().take();
                    state.file_encoding.set(project::TextEncoding::Utf8);
                    state.editor.set_file(None);
                    state.editor.load_text(&LatexEditor::new_document_text());
                    state.image_preview.set_filename(None::<&Path>);
                    state
                        .file_pdf_preview
                        .set_message(&tr("Select a PDF file to preview it here."));
                    state.editor.set_mode(EditorMode::Code);
                    state.code_mode_button.set_active(true);
                    state.visual_mode_button.set_active(false);
                    update_editor_capabilities(&state);
                    update_title(&state, false);
                }
                if main_removed {
                    *state.main_tex.borrow_mut() = None;
                }
                if let Some(root) = state.project_root.borrow().clone() {
                    if state
                        .project_target
                        .borrow()
                        .as_ref()
                        .is_some_and(|target| project::path_contains(&path, target))
                    {
                        *state.project_target.borrow_mut() = Some(root.clone());
                    }
                    state
                        .expanded_folders
                        .borrow_mut()
                        .retain(|folder| !project::path_contains(&path, folder));
                    if main_removed {
                        open_project(&state, &root);
                    } else if current_removed {
                        if let Some(main) = state.main_tex.borrow().clone() {
                            open_document(&state, &main, Some(root));
                        } else {
                            refresh_project_files(&state);
                        }
                    } else {
                        refresh_project_files(&state);
                    }
                } else {
                    refresh_project_files(&state);
                }
                show_toast(&state, &tr("Moved to Trash"));
            }
            Err(error) => show_toast(
                &state,
                &format!("{}: {error}", tr("Could not move to Trash")),
            ),
        },
    );
}

fn show_template_picker(state: Rc<State>) {
    let dialog = gtk::Dialog::builder()
        .title(tr("New project"))
        .transient_for(&state.window)
        .modal(true)
        .default_width(680)
        .default_height(560)
        .build();
    dialog.add_button(&tr("Cancel"), gtk::ResponseType::Cancel);
    dialog.add_button(&tr("Continue"), gtk::ResponseType::Accept);
    dialog.set_default_response(gtk::ResponseType::Accept);
    let content = dialog.content_area();
    content.set_spacing(12);
    content.set_margin_top(18);
    content.set_margin_bottom(18);
    content.set_margin_start(20);
    content.set_margin_end(20);
    let introduction = gtk::Label::new(Some(&tr(
        "Choose a starting point. Template files are copied into a local project folder.",
    )));
    introduction.set_xalign(0.0);
    introduction.set_wrap(true);
    content.append(&introduction);

    let choices = [
        (
            tr("Blank LaTeX document"),
            tr("Start a new document with a simple article structure."),
            "",
        ),
        (
            tr("ABNT · abnTeX2"),
            tr("Academic work, articles, reports, and research projects based on ABNT rules."),
            "https://ctan.org/pkg/abntex2?lang=en",
        ),
        (
            tr("IEEE · IEEEtran"),
            tr("IEEE conference and journal papers using the IEEEtran class."),
            "https://journals.ieeeauthorcenter.ieee.org/create-your-ieee-journal-article/authoring-tools-and-templates/tools-for-ieee-authors/ieee-article-templates/",
        ),
        (
            tr("ACM · acmart"),
            tr("ACM journal and conference papers using the acmart class."),
            "https://www.acm.org/publications/proceedings-template",
        ),
        (
            tr("Elsevier · elsarticle"),
            tr("Elsevier manuscripts using the elsarticle package."),
            "https://www.elsevier.com/en-gb/researcher/author/policies-and-guidelines/latex-instructions",
        ),
        (
            tr("Springer Nature"),
            tr("Springer Nature journal article templates and sample files."),
            "https://www.springernature.com/gp/authors/campaigns/latex-author-support",
        ),
    ];
    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::Single);
    list.add_css_class("boxed-list");
    for (name, description, source) in choices {
        let row = gtk::ListBoxRow::new();
        row.set_activatable(true);
        let row_content = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        row_content.set_margin_top(8);
        row_content.set_margin_bottom(8);
        row_content.set_margin_start(10);
        row_content.set_margin_end(10);
        let details = gtk::Box::new(gtk::Orientation::Vertical, 3);
        details.set_hexpand(true);
        let title = gtk::Label::new(Some(&name));
        title.set_xalign(0.0);
        title.add_css_class("heading");
        let subtitle = gtk::Label::new(Some(&description));
        subtitle.set_xalign(0.0);
        subtitle.set_wrap(true);
        subtitle.add_css_class("dim-label");
        details.append(&title);
        details.append(&subtitle);
        row_content.append(&details);
        if !source.is_empty() {
            let source_link = gtk::LinkButton::with_label(source, &tr("Source"));
            source_link.set_valign(gtk::Align::Center);
            row_content.append(&source_link);
        } else {
            row_content.prepend(&gtk::Image::from_icon_name("document-new-symbolic"));
        }
        row.set_child(Some(&row_content));
        list.append(&row);
    }
    list.select_row(list.row_at_index(0).as_ref());
    let scroll = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .child(&list)
        .build();
    content.append(&scroll);
    let note = gtk::Label::new(Some(&tr(
        "Journal and conference requirements can differ. Check the current author instructions before submission.",
    )));
    note.set_xalign(0.0);
    note.set_wrap(true);
    note.add_css_class("dim-label");
    content.append(&note);

    let state2 = state.clone();
    let dialog_for_row = dialog.clone();
    list.connect_row_activated(move |_, _| dialog_for_row.response(gtk::ResponseType::Accept));
    dialog.connect_response(move |dialog, response| {
        if response != gtk::ResponseType::Accept {
            dialog.close();
            return;
        }
        let Some(row) = list.selected_row() else {
            return;
        };
        let id = match row.index() {
            1 => "abntex2",
            2 => "ieeetran",
            3 => "acmart",
            4 => "elsarticle",
            5 => "springer-nature",
            _ => "blank",
        };
        dialog.close();
        choose_file(
            &state2,
            gtk::FileChooserAction::SelectFolder,
            "Choose where to create the project",
            "Create",
            move |state, parent| {
                prompt_project_name(&state, &parent, id);
            },
        );
    });
    dialog.present();
}

fn prompt_project_name(state: &Rc<State>, parent: &Path, id: &str) {
    let dialog = gtk::Dialog::builder()
        .title(tr("New project"))
        .transient_for(&state.window)
        .modal(true)
        .build();
    dialog.add_button(&tr("Cancel"), gtk::ResponseType::Cancel);
    dialog.add_button(&tr("Create"), gtk::ResponseType::Accept);
    dialog.set_default_response(gtk::ResponseType::Accept);
    let name = gtk::Entry::new();
    name.set_text("Ovenbird project");
    name.set_activates_default(true);
    name.set_margin_top(18);
    name.set_margin_bottom(18);
    name.set_margin_start(20);
    name.set_margin_end(20);
    dialog.content_area().append(&name);
    let name_for_response = name.clone();
    let parent = parent.to_path_buf();
    let id = id.to_owned();
    let state2 = state.clone();
    dialog.connect_response(move |dialog, response| {
        if response != gtk::ResponseType::Accept {
            dialog.close();
            return;
        }
        let project_name = name_for_response.text().to_string();
        if !crate::templates::valid_project_name(&project_name) {
            show_toast(&state2, &tr("Enter a valid project name"));
            return;
        }
        let template = if id == "blank" {
            None
        } else {
            crate::templates::TEMPLATES
                .iter()
                .find(|template| template.id == id)
                .copied()
        };
        if let Some(template) = template {
            let receiver = crate::templates::create_project_async(
                template,
                parent.clone(),
                project_name.clone(),
                template_resource_root(),
            );
            dialog.close();
            let progress = gtk::Dialog::builder()
                .title(tr("Creating project"))
                .transient_for(&state2.window)
                .modal(true)
                .deletable(false)
                .build();
            let progress_content = progress.content_area();
            progress_content.set_spacing(12);
            progress_content.set_margin_top(20);
            progress_content.set_margin_bottom(20);
            progress_content.set_margin_start(24);
            progress_content.set_margin_end(24);
            let spinner = gtk::Spinner::new();
            spinner.start();
            spinner.set_halign(gtk::Align::Center);
            progress_content.append(&spinner);
            let progress_label = gtk::Label::new(Some(&tr("Preparing template project…")));
            progress_label.set_wrap(true);
            progress_content.append(&progress_label);
            progress.present();
            state2.status.set_label(&tr("Creating project…"));
            let state_for_result = state2.clone();
            poll_result(receiver, move |result| {
                progress.close();
                state_for_result.status.set_label(&tr("Ready"));
                match result {
                    Ok((folder, main)) => {
                        state_for_result.expanded_folders.borrow_mut().clear();
                        open_document(&state_for_result, &main, Some(folder));
                    }
                    Err(error) => show_toast(
                        &state_for_result,
                        &format!("{}: {error}", tr("Could not create the project")),
                    ),
                }
            });
            return;
        }
        let folder = parent.join(&project_name);
        let folder_for_worker = folder.clone();
        let source = LatexEditor::new_document_text();
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let result = std::fs::create_dir(&folder_for_worker)
                .map_err(|error| error.to_string())
                .and_then(|()| {
                    crate::storage::atomic_write(
                        &folder_for_worker.join("main.tex"),
                        source.as_bytes(),
                    )
                    .map_err(|error| {
                        let _ = std::fs::remove_dir(&folder_for_worker);
                        error.to_string()
                    })
                });
            let _ = sender.send(result.map(|()| {
                (
                    folder_for_worker.clone(),
                    folder_for_worker.join("main.tex"),
                )
            }));
        });
        let dialog_for_result = dialog.clone();
        let state_for_result = state2.clone();
        poll_result(receiver, move |result| match result {
            Ok((folder, main)) => {
                state_for_result.expanded_folders.borrow_mut().clear();
                open_document(&state_for_result, &main, Some(folder));
                dialog_for_result.close();
            }
            Err(error) => show_toast(
                &state_for_result,
                &format!("{}: {error}", tr("Could not create the project")),
            ),
        });
    });
    dialog.present();
    name.grab_focus();
}

fn template_resource_root() -> PathBuf {
    let installed = option_env!("OVENBIRD_TEMPLATE_DIR").map(Path::new);
    crate::templates::resource_root(installed, Path::new(env!("CARGO_MANIFEST_DIR")))
}

fn show_link_dialog(state: Rc<State>) {
    let dialog = gtk::Dialog::builder()
        .title(tr("Insert link"))
        .transient_for(&state.window)
        .modal(true)
        .build();
    dialog.add_button(&tr("Cancel"), gtk::ResponseType::Cancel);
    dialog.add_button(&tr("Insert link"), gtk::ResponseType::Accept);
    dialog.set_default_response(gtk::ResponseType::Accept);
    let grid = gtk::Grid::builder()
        .row_spacing(8)
        .column_spacing(10)
        .margin_top(18)
        .margin_bottom(18)
        .margin_start(20)
        .margin_end(20)
        .build();
    let label_entry = gtk::Entry::new();
    label_entry.set_text(&state.editor.selected_text());
    label_entry.set_placeholder_text(Some(&tr("Link text")));
    let url_entry = gtk::Entry::new();
    url_entry.set_placeholder_text(Some("https://"));
    grid.attach(&gtk::Label::new(Some(&tr("Text"))), 0, 0, 1, 1);
    grid.attach(&label_entry, 1, 0, 1, 1);
    grid.attach(&gtk::Label::new(Some(&tr("URL"))), 0, 1, 1, 1);
    grid.attach(&url_entry, 1, 1, 1, 1);
    dialog.content_area().append(&grid);
    let label_entry_for_response = label_entry.clone();
    let url_entry_for_response = url_entry.clone();
    let state2 = state.clone();
    dialog.connect_response(move |dialog, response| {
        if response == gtk::ResponseType::Accept {
            let text = label_entry_for_response.text().to_string();
            let url = url_entry_for_response.text().to_string();
            if url.trim().is_empty() {
                show_toast(&state2, &tr("Enter a URL"));
                return;
            }
            let snippet = format!("\\href{{{url}}}{{{text}}}");
            let cursor_offset = snippet.chars().count();
            state2.editor.insert_latex(
                &LatexInsertion {
                    text: snippet,
                    cursor_offset,
                    selection: None,
                },
                "insert:link",
            );
        }
        dialog.close();
    });
    dialog.present();
    if label_entry.text().is_empty() {
        label_entry.grab_focus();
    } else {
        url_entry.grab_focus();
    }
}

fn choose_image(state: Rc<State>) {
    if state.current_file.borrow().as_ref().is_none_or(|path| {
        !path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("tex"))
    }) {
        show_toast(
            &state,
            &tr("Open a LaTeX document before inserting an image."),
        );
        return;
    }
    let dialog = gtk::FileChooserNative::new(
        Some(&tr("Insert image")),
        Some(&state.window),
        gtk::FileChooserAction::Open,
        Some(&tr("Open")),
        Some(&tr("Cancel")),
    );
    let filter = gtk::FileFilter::new();
    filter.set_name(Some(&tr("Image and PDF files")));
    for pattern in ["*.png", "*.jpg", "*.jpeg", "*.pdf", "*.eps"] {
        filter.add_pattern(pattern);
    }
    dialog.add_filter(&filter);
    let state_for_choice = state.clone();
    dialog.connect_response(move |dialog, response| {
        if response != gtk::ResponseType::Accept {
            dialog.destroy();
            return;
        }
        let Some(source) = dialog.file().and_then(|file| file.path()) else {
            dialog.destroy();
            return;
        };
        choose_image_selected(state_for_choice.clone(), source);
        dialog.destroy();
    });
    dialog.show();
}

fn choose_image_selected(state: Rc<State>, source: PathBuf) {
    let Some(folder) = state
        .project_target
        .borrow()
        .clone()
        .or_else(|| state.project_root.borrow().clone())
    else {
        show_toast(
            &state,
            &tr("Open a project folder before inserting an image."),
        );
        return;
    };
    let Some(name) = source.file_name().map(|name| name.to_os_string()) else {
        show_toast(&state, &tr("The image name is invalid."));
        return;
    };
    let source = source.to_path_buf();
    let folder_for_worker = folder.clone();
    let root_for_insert = state
        .project_root
        .borrow()
        .clone()
        .unwrap_or(folder.clone());
    let current_file = state.current_file.borrow().clone();
    let main_file = state
        .main_tex
        .borrow()
        .clone()
        .or_else(|| current_file.clone());
    let Some(main_directory) = main_file
        .as_deref()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
    else {
        show_toast(
            &state,
            &tr("Choose a main LaTeX document before inserting an image."),
        );
        return;
    };
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let mut destination = folder_for_worker.join(name);
        let mut suffix = 2;
        while destination.exists() {
            let stem = source
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("image");
            let extension = source.extension().and_then(|s| s.to_str()).unwrap_or("");
            let new_name = if extension.is_empty() {
                format!("{stem}-{suffix}")
            } else {
                format!("{stem}-{suffix}.{extension}")
            };
            destination = folder_for_worker.join(new_name);
            suffix += 1;
        }
        let result = std::fs::copy(&source, &destination)
            .map(|_| destination)
            .map_err(|error| error.to_string());
        let _ = sender.send(result);
    });
    poll_result(receiver, move |result| match result {
        Ok(destination) => {
            if state.project_root.borrow().as_deref() != Some(root_for_insert.as_path())
                || state.current_file.borrow().as_ref() != current_file.as_ref()
            {
                if state.project_root.borrow().as_deref() == Some(root_for_insert.as_path()) {
                    refresh_project_files(&state);
                }
                show_toast(
                    &state,
                    &tr("Image copied to the project. Reopen the document to insert it."),
                );
                return;
            }
            let Some(relative) = project::relative_path(&main_directory, &destination) else {
                show_toast(
                    &state,
                    &tr("The image path cannot be represented relative to the main document."),
                );
                return;
            };
            let relative = relative.to_string_lossy().replace('\\', "/");
            let snippet = format!("\\includegraphics[width=\\linewidth]{{{relative}}}");
            state
                .editor
                .insert_at_cursor(&snippet, snippet.chars().count(), "insert:image");
            refresh_project_files(&state);
        }
        Err(error) => show_toast(
            &state,
            &format!("{}: {error}", tr("Could not add the image to the project")),
        ),
    });
}

fn show_table_dialog(state: Rc<State>) {
    let dialog = gtk::Dialog::builder()
        .title(tr("Insert table"))
        .transient_for(&state.window)
        .modal(true)
        .build();
    dialog.add_button(&tr("Cancel"), gtk::ResponseType::Cancel);
    dialog.add_button(&tr("Insert"), gtk::ResponseType::Accept);
    let grid = gtk::Grid::builder()
        .row_spacing(8)
        .column_spacing(10)
        .margin_top(18)
        .margin_bottom(18)
        .margin_start(20)
        .margin_end(20)
        .build();
    let rows = gtk::SpinButton::with_range(1.0, 20.0, 1.0);
    rows.set_value(3.0);
    let columns = gtk::SpinButton::with_range(1.0, 12.0, 1.0);
    columns.set_value(3.0);
    grid.attach(&gtk::Label::new(Some(&tr("Rows"))), 0, 0, 1, 1);
    grid.attach(&rows, 1, 0, 1, 1);
    grid.attach(&gtk::Label::new(Some(&tr("Columns"))), 0, 1, 1, 1);
    grid.attach(&columns, 1, 1, 1, 1);
    dialog.content_area().append(&grid);
    let state2 = state.clone();
    dialog.connect_response(move |dialog, response| {
        if response == gtk::ResponseType::Accept {
            let snippet = commands::create_table_snippet(
                rows.value_as_int() as usize,
                columns.value_as_int() as usize,
            );
            state2.editor.insert_latex(&snippet, "insert:table");
        }
        dialog.close();
    });
    dialog.present();
}

fn insert_label_or_note(state: &Rc<State>, kind: &str) {
    let (title, prompt, command) = match kind {
        "label" => ("Add label", "Label identifier", "label"),
        "ref" => ("Cross-reference", "Label identifier", "ref"),
        _ => ("Review note", "Note", "todo"),
    };
    show_text_prompt(state, title, prompt, "", move |state, value| {
        if value.trim().is_empty() {
            return;
        }
        let snippet = if command == "todo" {
            format!("\\todo{{{value}}}")
        } else {
            format!("\\{command}{{{value}}}")
        };
        let cursor = snippet.chars().count();
        state
            .editor
            .insert_at_cursor(&snippet, cursor, &format!("insert:{command}"));
    });
}

fn show_text_prompt(
    state: &Rc<State>,
    title: &str,
    prompt: &str,
    initial: &str,
    callback: impl Fn(Rc<State>, String) + 'static,
) {
    let dialog = gtk::Dialog::builder()
        .title(tr_dynamic(title))
        .transient_for(&state.window)
        .modal(true)
        .build();
    dialog.add_button(&tr("Cancel"), gtk::ResponseType::Cancel);
    dialog.add_button(&tr("OK"), gtk::ResponseType::Accept);
    dialog.set_default_response(gtk::ResponseType::Accept);
    let content = dialog.content_area();
    content.set_spacing(8);
    content.set_margin_top(18);
    content.set_margin_bottom(18);
    content.set_margin_start(20);
    content.set_margin_end(20);
    content.append(&gtk::Label::new(Some(&tr_dynamic(prompt))));
    let entry = gtk::Entry::new();
    entry.set_text(initial);
    entry.set_activates_default(true);
    content.append(&entry);
    let entry_for_response = entry.clone();
    let state = state.clone();
    dialog.connect_response(move |dialog, response| {
        if response == gtk::ResponseType::Accept {
            callback(state.clone(), entry_for_response.text().to_string());
        }
        dialog.close();
    });
    dialog.present();
    entry.grab_focus();
}

fn show_citation_picker(state: Rc<State>) {
    show_page(&state, "editor");
    if !supports_latex_document(&state) {
        show_toast(
            &state,
            &tr("Open a LaTeX document before inserting a citation."),
        );
        return;
    }
    let dialog = gtk::Dialog::builder()
        .title(tr("Insert citation"))
        .transient_for(&state.window)
        .modal(true)
        .build();
    dialog.add_button(&tr("Close"), gtk::ResponseType::Close);
    dialog.set_default_size(580, 500);
    let list = gtk::Box::new(gtk::Orientation::Vertical, 2);
    let query = gtk::SearchEntry::new();
    query.set_placeholder_text(Some(&tr("Search references…")));
    query.set_margin_top(10);
    query.set_margin_start(10);
    query.set_margin_end(10);
    let scroll = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .child(&list)
        .build();
    dialog.content_area().append(&query);
    dialog.content_area().append(&scroll);
    let fill = Rc::new(RefCell::new(None::<Box<dyn Fn(&str)>>));
    *fill.borrow_mut() = Some(Box::new({
        let state = state.clone();
        let dialog = dialog.clone();
        let list = list.clone();
        move |term: &str| {
            while let Some(child) = list.first_child() {
                list.remove(&child);
            }
            let library_is_empty = state.library.borrow().bibliography.entries.is_empty();
            for entry in state.library.borrow().bibliography.entries.iter() {
                if !bibtex::entry_matches_search(entry, term) {
                    continue;
                }
                let item = gtk::Button::new();
                item.set_has_frame(false);
                item.set_halign(gtk::Align::Fill);
                let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
                let title = gtk::Label::new(Some(if entry.get("title").is_empty() {
                    &entry.key
                } else {
                    entry.get("title")
                }));
                title.set_xalign(0.0);
                title.set_hexpand(true);
                row.append(&title);
                let key = gtk::Label::new(Some(&entry.key));
                key.add_css_class("dim-label");
                row.append(&key);
                item.set_child(Some(&row));
                let state2 = state.clone();
                let key_value = entry.key.clone();
                let dialog = dialog.clone();
                item.connect_clicked(move |_| {
                    let snippet = format!("\\cite{{{key_value}}}");
                    state2.editor.insert_at_cursor(
                        &snippet,
                        snippet.chars().count(),
                        "insert:citation",
                    );
                    dialog.close();
                    show_page(&state2, "editor");
                });
                list.append(&item);
            }
            if list.first_child().is_none() {
                let empty = gtk::Label::new(Some(&tr("No matching references")));
                empty.set_margin_top(20);
                list.append(&empty);
                if library_is_empty {
                    let add = labeled_button("list-add-symbolic", &tr("Add reference"));
                    let state_for_add = state.clone();
                    let dialog_for_add = dialog.clone();
                    add.connect_clicked(move |_| {
                        dialog_for_add.close();
                        show_page(&state_for_add, "references");
                        edit_reference(&state_for_add, None);
                    });
                    list.append(&add);
                    let import = labeled_button("document-open-symbolic", &tr("Import BibTeX"));
                    let state_for_import = state.clone();
                    let dialog_for_import = dialog.clone();
                    import.connect_clicked(move |_| {
                        dialog_for_import.close();
                        show_page(&state_for_import, "references");
                        import_bibtex(state_for_import.clone());
                    });
                    list.append(&import);
                }
            }
        }
    }));
    let fill_changed = fill.clone();
    query.connect_search_changed(move |entry| {
        if let Some(fill) = fill_changed.borrow().as_ref() {
            fill(entry.text().as_str());
        }
    });
    if let Some(fill) = fill.borrow().as_ref() {
        fill("");
    }
    dialog.connect_response(|dialog, _| dialog.close());
    dialog.present();
}

fn load_library_async(state: &Rc<State>) {
    if state.library_loading.replace(true) {
        return;
    }
    state.library_load_error.borrow_mut().take();
    refresh_library(state);
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let result = LocalLibrary::open().map_err(|error| error.to_string());
        let _ = sender.send(result);
    });
    let state = state.clone();
    poll_result(receiver, move |result| match result {
        Ok(library) => {
            state.library_loading.set(false);
            *state.library.borrow_mut() = library;
            state.library_ready.set(true);
            refresh_library(&state);
        }
        Err(error) => {
            state.library_loading.set(false);
            *state.library_load_error.borrow_mut() = Some(error.clone());
            show_toast(
                &state,
                &format!("{}: {error}", tr("Could not load the local library")),
            );
            refresh_library(&state);
        }
    });
}

fn persist_library_async(state: Rc<State>, updated: LocalLibrary, success_message: String) {
    if !state.library_ready.get() || state.library_busy.replace(true) {
        show_toast(
            &state,
            &tr("The local library is busy. Try again in a moment."),
        );
        return;
    }
    state.status.set_label(&tr("Saving references…"));
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let result = updated.save().map_err(|error| error.to_string());
        let _ = sender.send((result, updated));
    });
    poll_result(receiver, move |(result, updated)| {
        state.library_busy.set(false);
        state.status.set_label(&tr("Ready"));
        match result {
            Ok(()) => {
                *state.library.borrow_mut() = updated;
                refresh_library(&state);
                show_toast(&state, &success_message);
            }
            Err(error) => show_toast(
                &state,
                &format!("{}: {error}", tr("Could not save the local library")),
            ),
        }
    });
}

fn library_is_ready(state: &Rc<State>) -> bool {
    if !state.library_ready.get() {
        show_toast(state, &tr("The local library is still loading."));
        return false;
    }
    if state.library_busy.get() {
        show_toast(
            state,
            &tr("The local library is busy. Try again in a moment."),
        );
        return false;
    }
    true
}

fn refresh_library(state: &Rc<State>) {
    while let Some(child) = state.library_list.first_child() {
        state.library_list.remove(&child);
    }
    if !state.library_ready.get() {
        if let Some(error) = state.library_load_error.borrow().as_ref() {
            let failed = gtk::Label::new(Some(&format!(
                "{}: {error}",
                tr("Could not load the local library")
            )));
            failed.set_wrap(true);
            failed.set_margin_top(24);
            failed.set_margin_bottom(8);
            failed.add_css_class("dim-label");
            state.library_list.append(&failed);
            let retry = gtk::Button::with_label(&tr("Retry"));
            retry.set_halign(gtk::Align::Start);
            let state_for_retry = state.clone();
            retry.connect_clicked(move |_| load_library_async(&state_for_retry));
            state.library_list.append(&retry);
        } else {
            let loading = gtk::Label::new(Some(&tr("Loading local library…")));
            loading.set_margin_top(24);
            loading.set_margin_bottom(24);
            loading.add_css_class("dim-label");
            state.library_list.append(&loading);
        }
        update_primary_action(state);
        return;
    }
    let term = state.library_search.text();
    let library = state.library.borrow();
    let filter_options = reference_filter_values(&library.bibliography.entries);
    state.library_filters_updating.set(true);
    set_reference_filter_options(
        &state.reference_filters.author,
        &tr("All authors"),
        filter_options.authors,
    );
    set_reference_filter_options(
        &state.reference_filters.year,
        &tr("All years"),
        filter_options.years,
    );
    set_reference_filter_options(
        &state.reference_filters.tag,
        &tr("All tags"),
        filter_options.tags,
    );
    state.library_filters_updating.set(false);
    let author_filter = selected_reference_filter(&state.reference_filters.author);
    let year_filter = selected_reference_filter(&state.reference_filters.year);
    let tag_filter = selected_reference_filter(&state.reference_filters.tag);
    let entries = library
        .bibliography
        .entries
        .iter()
        .filter(|entry| bibtex::entry_matches_search(entry, &term))
        .filter(|entry| {
            reference_matches_filters(
                entry,
                author_filter.as_deref(),
                year_filter.as_deref(),
                tag_filter.as_deref(),
            )
        })
        .cloned()
        .collect::<Vec<_>>();
    if entries.is_empty() {
        let message = if library.bibliography.entries.is_empty() {
            tr("Your library starts here: add a reference, import BibTeX, or connect Zotero.")
        } else {
            tr("No references match the current search and filters.")
        };
        let empty = gtk::Label::new(Some(&message));
        empty.set_wrap(true);
        empty.set_margin_top(24);
        empty.set_margin_bottom(24);
        empty.add_css_class("dim-label");
        state.library_list.append(&empty);
    } else {
        for entry in entries {
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            row.set_margin_top(4);
            row.set_margin_bottom(4);
            let title = if entry.get("title").is_empty() {
                entry.key.clone()
            } else {
                entry.get("title").to_owned()
            };
            let author = if entry.get("author").is_empty() {
                entry.get("editor")
            } else {
                entry.get("author")
            };
            let subtitle = format!(
                "{}{}{}",
                author,
                if entry.get("year").is_empty() {
                    ""
                } else {
                    " · "
                },
                entry.get("year")
            );
            let details = gtk::Box::new(gtk::Orientation::Vertical, 3);
            let title_label = gtk::Label::new(Some(&title));
            title_label.set_xalign(0.0);
            title_label.add_css_class("heading");
            details.append(&title_label);
            let subtitle_label = gtk::Label::new(Some(&subtitle));
            subtitle_label.set_xalign(0.0);
            subtitle_label.add_css_class("dim-label");
            details.append(&subtitle_label);
            details.set_hexpand(true);
            row.append(&details);
            let key = gtk::Label::new(Some(&entry.key));
            key.add_css_class("dim-label");
            row.append(&key);
            let edit = button("document-edit-symbolic", &tr("Edit reference"));
            let entry_copy = entry.clone();
            let state_copy = state.clone();
            edit.connect_clicked(move |_| edit_reference(&state_copy, Some(entry_copy.clone())));
            row.append(&edit);
            let delete = button("user-trash-symbolic", &tr("Delete reference"));
            let key_copy = entry.key.clone();
            let state_copy = state.clone();
            delete.connect_clicked(move |_| {
                if !library_is_ready(&state_copy) {
                    return;
                }
                let mut updated = state_copy.library.borrow().clone();
                if updated.remove_in_memory(&key_copy) {
                    persist_library_async(state_copy.clone(), updated, tr("Reference deleted"));
                }
            });
            row.append(&delete);
            state.library_list.append(&row);
            state
                .library_list
                .append(&gtk::Separator::new(gtk::Orientation::Horizontal));
        }
    }
    update_primary_action(state);
}

fn reference_filter_values(entries: &[BibEntry]) -> ReferenceFilterOptions {
    let authors = sorted_reference_values(entries.iter().flat_map(reference_authors));
    let years = sorted_reference_values(entries.iter().filter_map(reference_year));
    let tags = sorted_reference_values(entries.iter().flat_map(reference_tags));
    ReferenceFilterOptions {
        authors,
        years,
        tags,
    }
}

fn sorted_reference_values(values: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut values = values
        .into_iter()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    values.sort_by_key(|value| value.to_lowercase());
    values
}

fn reference_authors(entry: &BibEntry) -> Vec<String> {
    let author = if entry.get("author").trim().is_empty() {
        entry.get("editor")
    } else {
        entry.get("author")
    };
    author
        .split(" and ")
        .map(str::trim)
        .filter(|author| !author.is_empty())
        .map(str::to_owned)
        .collect()
}

fn reference_year(entry: &BibEntry) -> Option<String> {
    let year = entry.get("year").trim();
    if !year.is_empty() {
        return Some(year.to_owned());
    }
    let date = entry.get("date").trim();
    let year = date.chars().take(4).collect::<String>();
    (year.len() == 4 && year.chars().all(|character| character.is_ascii_digit())).then_some(year)
}

fn reference_tags(entry: &BibEntry) -> Vec<String> {
    ["keywords", "tags"]
        .into_iter()
        .flat_map(|field| entry.get(field).split([',', ';']))
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .map(str::to_owned)
        .collect()
}

fn reference_matches_filters(
    entry: &BibEntry,
    author: Option<&str>,
    year: Option<&str>,
    tag: Option<&str>,
) -> bool {
    let author_matches = author.is_none_or(|selected| {
        reference_authors(entry)
            .iter()
            .any(|candidate| candidate.eq_ignore_ascii_case(selected))
    });
    let year_matches = year.is_none_or(|selected| {
        reference_year(entry).is_some_and(|candidate| candidate.eq_ignore_ascii_case(selected))
    });
    let tag_matches = tag.is_none_or(|selected| {
        reference_tags(entry)
            .iter()
            .any(|candidate| candidate.eq_ignore_ascii_case(selected))
    });
    author_matches && year_matches && tag_matches
}

fn selected_reference_filter(dropdown: &gtk::DropDown) -> Option<String> {
    if dropdown.selected() == 0 || dropdown.selected() == gtk::INVALID_LIST_POSITION {
        return None;
    }
    dropdown
        .selected_item()?
        .downcast::<gtk::StringObject>()
        .ok()
        .map(|item| item.string().to_string())
}

fn set_reference_filter_options(dropdown: &gtk::DropDown, all_label: &str, values: Vec<String>) {
    let selection = selected_reference_filter(dropdown);
    let mut options = vec![all_label.to_owned()];
    options.extend(values);
    let model_strings = options.iter().map(String::as_str).collect::<Vec<_>>();
    let model = gtk::StringList::new(&model_strings);
    dropdown.set_model(Some(&model));
    let selected = selection
        .and_then(|value| options.iter().position(|option| option == &value))
        .unwrap_or(0);
    dropdown.set_selected(selected as u32);
}

#[cfg(test)]
mod reference_filter_tests {
    use super::{reference_filter_values, reference_matches_filters};
    use crate::bibtex::BibEntry;

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
}

fn edit_reference(state: &Rc<State>, entry: Option<BibEntry>) {
    if !library_is_ready(state) {
        return;
    }
    let dialog = gtk::Dialog::builder()
        .title(tr_dynamic(if entry.is_some() {
            "Edit reference"
        } else {
            "New reference"
        }))
        .transient_for(&state.window)
        .modal(true)
        .build();
    dialog.set_default_size(700, 660);
    dialog.add_button(&tr("Cancel"), gtk::ResponseType::Cancel);
    if entry.is_some() {
        dialog.add_button(&tr("Delete"), gtk::ResponseType::Reject);
    }
    dialog.add_button(
        &tr_dynamic(if entry.is_some() { "Save" } else { "Add" }),
        gtk::ResponseType::Accept,
    );
    dialog.set_default_response(gtk::ResponseType::Accept);
    let grid = gtk::Grid::builder()
        .column_spacing(12)
        .row_spacing(8)
        .margin_top(18)
        .margin_bottom(18)
        .margin_start(20)
        .margin_end(20)
        .build();
    let form_scroll = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&grid)
        .build();
    dialog.content_area().append(&form_scroll);
    let translated_types = REFERENCE_TYPES
        .iter()
        .map(|kind| bibtex::reference_type_label(kind))
        .collect::<Vec<_>>();
    let type_choices = translated_types
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let type_model = gtk::StringList::new(&type_choices);
    let picker = gtk::DropDown::new(Some(type_model), gtk::Expression::NONE);
    let selected_type = entry
        .as_ref()
        .map(|entry| entry.entry_type.as_str())
        .unwrap_or("article");
    picker.set_selected(
        REFERENCE_TYPES
            .iter()
            .position(|kind| *kind == selected_type)
            .unwrap_or(0) as u32,
    );
    grid.attach(&gtk::Label::new(Some(&tr("Reference type"))), 0, 0, 1, 1);
    grid.attach(&picker, 1, 0, 1, 1);
    let field_names = REFERENCE_TYPES
        .iter()
        .flat_map(|kind| {
            crate::bibtex::fields_for_reference_type(kind)
                .iter()
                .copied()
        })
        .collect::<HashSet<_>>();
    let mut names = field_names.iter().copied().collect::<Vec<_>>();
    names.sort();
    let key_entry = gtk::Entry::new();
    if let Some(entry) = entry.as_ref() {
        key_entry.set_text(&entry.key);
    }
    let rows = Rc::new(RefCell::new(
        HashMap::<String, (gtk::Label, gtk::Entry)>::new(),
    ));
    for name in names {
        let label = gtk::Label::new(Some(&bibtex::reference_field_label(name, selected_type)));
        label.set_xalign(1.0);
        label.add_css_class("dim-label");
        let input = gtk::Entry::new();
        input.set_hexpand(true);
        if name == "title" {
            input.set_placeholder_text(Some(&tr("Full title of the work")));
        }
        if let Some(entry) = entry.as_ref() {
            let value = if name == "date" && entry.get("date").is_empty() {
                entry.get("year")
            } else {
                entry.get(name)
            };
            input.set_text(value);
        }
        rows.borrow_mut().insert(name.to_owned(), (label, input));
    }
    let key_label = gtk::Label::new(Some(&tr("Citation key")));
    key_label.set_xalign(1.0);
    key_label.add_css_class("dim-label");
    grid.attach(&key_label, 0, 1, 1, 1);
    grid.attach(&key_entry, 1, 1, 1, 1);
    let update_rows = {
        let rows = rows.clone();
        let grid = grid.clone();
        move |picker: &gtk::DropDown| {
            for (label, input) in rows.borrow().values() {
                if label.parent().is_some() {
                    grid.remove(label);
                }
                if input.parent().is_some() {
                    grid.remove(input);
                }
            }
            let kind = REFERENCE_TYPES
                .get(picker.selected() as usize)
                .copied()
                .unwrap_or("article");
            let mut row_num = 2;
            for name in crate::bibtex::fields_for_reference_type(kind) {
                if let Some((label, input)) = rows.borrow().get(*name) {
                    label.set_label(&bibtex::reference_field_label(name, kind));
                    grid.attach(label, 0, row_num, 1, 1);
                    grid.attach(input, 1, row_num, 1, 1);
                    row_num += 1;
                }
            }
        }
    };
    update_rows(&picker);
    let update_rows_notify = update_rows;
    picker.connect_selected_notify(move |picker| update_rows_notify(picker));
    let previous = entry.clone();
    let state2 = state.clone();
    dialog.connect_response(move |dialog, response| {
        if response == gtk::ResponseType::Reject {
            if let Some(entry) = previous.as_ref() {
                let mut updated = state2.library.borrow().clone();
                if updated.remove_in_memory(&entry.key) {
                    persist_library_async(state2.clone(), updated, tr("Reference deleted"));
                }
            }
            dialog.close();
            return;
        }
        if response == gtk::ResponseType::Accept {
            let kind = REFERENCE_TYPES
                .get(picker.selected() as usize)
                .copied()
                .unwrap_or("article");
            let values = rows
                .borrow()
                .iter()
                .map(|(name, (_, entry))| (name.clone(), entry.text().trim().to_owned()))
                .collect::<indexmap::IndexMap<_, _>>();
            let old_fields = previous
                .as_ref()
                .map(|entry| &entry.fields)
                .cloned()
                .unwrap_or_default();
            let fields = bibtex::build_reference_fields(kind, &values, &old_fields);
            if fields.get("title").is_none_or(String::is_empty) {
                show_toast(&state2, &tr("Enter the reference title"));
                return;
            }
            let used = state2
                .library
                .borrow()
                .bibliography
                .entries
                .iter()
                .filter(|item| previous.as_ref().is_none_or(|old| old.key != item.key))
                .map(|item| item.key.to_ascii_lowercase())
                .collect::<HashSet<_>>();
            let key = if key_entry.text().trim().is_empty() {
                bibtex::create_citation_key(&fields, &used)
            } else {
                key_entry.text().trim().to_owned()
            };
            let updated = BibEntry {
                entry_type: kind.to_owned(),
                key,
                fields,
                raw_fields: indexmap::IndexMap::new(),
            };
            let mut library = state2.library.borrow().clone();
            let result = if let Some(previous) = previous.as_ref() {
                library.update_in_memory(&previous.key, updated)
            } else {
                library.add_in_memory(updated)
            };
            match result {
                Ok(()) => {
                    persist_library_async(state2.clone(), library, tr("Reference saved"));
                    dialog.close();
                }
                Err(error) => show_toast(&state2, &error),
            }
            return;
        }
        dialog.close();
    });
    dialog.present();
}

fn import_bibtex(state: Rc<State>) {
    if !library_is_ready(&state) {
        return;
    }
    choose_file(
        &state,
        gtk::FileChooserAction::Open,
        "Import BibTeX",
        "Import",
        |state, path| {
            let (sender, receiver) = mpsc::channel();
            std::thread::spawn(move || {
                let result = std::fs::read_to_string(&path)
                    .map_err(|error| error.to_string())
                    .and_then(|contents| {
                        bibtex::parse_bibtex(&contents).map_err(|error| error.to_string())
                    });
                let _ = sender.send(result);
            });
            poll_result(receiver, move |result| match result {
                Ok(imported) => {
                    if !library_is_ready(&state) {
                        return;
                    }
                    let mut library = state.library.borrow().clone();
                    let count = library.merge_import(imported);
                    persist_library_async(
                        state.clone(),
                        library,
                        tr("Imported %d references").replace("%d", &count.to_string()),
                    );
                }
                Err(error) => show_toast(
                    &state,
                    &format!("{}: {error}", tr("Could not import BibTeX")),
                ),
            });
        },
    );
}

fn export_bibtex(state: Rc<State>) {
    if !library_is_ready(&state) {
        return;
    }
    choose_file(
        &state,
        gtk::FileChooserAction::Save,
        "Export BibTeX",
        "Export",
        |state, path| {
            let source = bibtex::serialize_bibtex(&state.library.borrow().bibliography);
            let (sender, receiver) = mpsc::channel();
            std::thread::spawn(move || {
                let result = std::fs::write(path, source).map_err(|error| error.to_string());
                let _ = sender.send(result);
            });
            poll_result(receiver, move |result| match result {
                Ok(()) => show_toast(&state, &tr("BibTeX exported")),
                Err(error) => show_toast(
                    &state,
                    &format!("{}: {error}", tr("Could not export BibTeX")),
                ),
            });
        },
    );
}

fn sync_or_configure_zotero(state: Rc<State>) {
    if !library_is_ready(&state) {
        show_toast(&state, &tr("The local library is still loading."));
        return;
    }
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let result = ZoteroSettings::load()
            .map_err(|error| error.to_string())
            .and_then(|settings| {
                crate::zotero::load_api_key()
                    .map(|key| {
                        !settings.user_id.trim().is_empty()
                            && key.is_some_and(|key| !key.trim().is_empty())
                    })
                    .map_err(|error| error.to_string())
            });
        let _ = sender.send(result);
    });
    poll_result(receiver, move |result| match result {
        Ok(true) => sync_zotero(state),
        Ok(false) => show_zotero_settings(state),
        Err(error) => {
            show_toast(
                &state,
                &format!("{}: {error}", tr("Could not check Zotero settings")),
            );
            show_zotero_settings(state);
        }
    });
}

fn show_zotero_settings(state: Rc<State>) {
    let dialog = gtk::Dialog::builder()
        .title(tr("Sync with Zotero"))
        .transient_for(&state.window)
        .modal(true)
        .build();
    dialog.set_default_size(620, 360);
    dialog.add_button(&tr("Cancel"), gtk::ResponseType::Cancel);
    let save_button = dialog.add_button(&tr("Save and sync"), gtk::ResponseType::Accept);
    save_button.set_sensitive(false);
    dialog.set_default_response(gtk::ResponseType::Accept);
    let content = dialog.content_area();
    content.set_spacing(10);
    content.set_margin_top(18);
    content.set_margin_bottom(18);
    content.set_margin_start(20);
    content.set_margin_end(20);
    let description = gtk::Label::new(Some(&tr(
        "The local library works without an account. To sync, enter your library ID and a Zotero key with read and write access.",
    )));
    description.set_xalign(0.0);
    description.set_wrap(true);
    content.append(&description);
    let account = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let hint = gtk::Label::new(Some(&tr("Don’t have a Zotero account?")));
    hint.add_css_class("dim-label");
    account.append(&hint);
    let link = gtk::LinkButton::with_label(
        "https://www.zotero.org/user/register/",
        &tr("Create a Zotero account"),
    );
    account.append(&link);
    content.append(&account);
    let grid = gtk::Grid::builder()
        .column_spacing(10)
        .row_spacing(8)
        .build();
    let id_entry = gtk::Entry::new();
    id_entry.set_placeholder_text(Some(&tr("Found at zotero.org/settings/keys")));
    id_entry.set_hexpand(true);
    let key_entry = gtk::PasswordEntry::new();
    key_entry.set_placeholder_text(Some(&tr("Zotero key with write access")));
    key_entry.set_show_peek_icon(true);
    key_entry.set_hexpand(true);
    grid.attach(&gtk::Label::new(Some(&tr("User ID"))), 0, 0, 1, 1);
    grid.attach(&id_entry, 1, 0, 1, 1);
    grid.attach(&gtk::Label::new(Some(&tr("API key"))), 0, 1, 1, 1);
    grid.attach(&key_entry, 1, 1, 1, 1);
    content.append(&grid);
    let settings_for_response = Rc::new(RefCell::new(None::<ZoteroSettings>));
    let state2 = state.clone();
    let settings_for_save = settings_for_response.clone();
    let id_entry_for_save = id_entry.clone();
    dialog.connect_response(move |dialog, response| {
        if response != gtk::ResponseType::Accept {
            dialog.close();
            return;
        }
        let Some(mut updated) = settings_for_save.borrow().clone() else {
            show_toast(&state2, &tr("Zotero settings are still loading."));
            return;
        };
        let user_id = id_entry_for_save.text().trim().to_owned();
        let api_key = key_entry.text().trim().to_owned();
        if user_id.is_empty() || api_key.is_empty() {
            show_toast(&state2, &tr("Enter the ID and API key"));
            return;
        }
        updated.user_id = user_id;
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let result = updated
                .save()
                .map_err(|error| error.to_string())
                .and_then(|_| crate::zotero::store_api_key(&api_key));
            let _ = sender.send(result);
        });
        dialog.close();
        state2.status.set_label(&tr("Saving Zotero settings…"));
        poll_result(receiver, {
            let state = state2.clone();
            move |result| match result {
                Ok(()) => {
                    state.status.set_label(&tr("Ready"));
                    sync_zotero(state);
                }
                Err(error) => {
                    state.status.set_label(&tr("Ready"));
                    show_toast(
                        &state,
                        &format!("{}: {error}", tr("Could not save the Zotero key")),
                    );
                }
            }
        });
    });
    dialog.present();

    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = sender.send(ZoteroSettings::load().map_err(|error| error.to_string()));
    });
    let id_entry_for_load = id_entry.clone();
    let save_button_for_load = save_button.clone();
    let settings_for_load = settings_for_response;
    let dialog_for_load = dialog.clone();
    let state_for_load = state.clone();
    poll_result(receiver, move |result| match result {
        Ok(settings) => {
            id_entry_for_load.set_text(&settings.user_id);
            *settings_for_load.borrow_mut() = Some(settings);
            save_button_for_load.set_sensitive(true);
        }
        Err(error) => {
            show_toast(
                &state_for_load,
                &format!("{}: {error}", tr("Could not load Zotero settings")),
            );
            let retry = gtk::Button::with_label(&tr("Retry"));
            retry.set_halign(gtk::Align::Start);
            let retry_parent = dialog_for_load.content_area();
            retry_parent.append(&retry);
            let id_entry = id_entry_for_load.clone();
            let save_button = save_button_for_load.clone();
            let settings = settings_for_load.clone();
            let dialog = dialog_for_load.clone();
            let state = state_for_load.clone();
            let retry_for_click = retry.clone();
            retry.connect_clicked(move |_| {
                retry_for_click.set_sensitive(false);
                let (sender, receiver) = mpsc::channel();
                std::thread::spawn(move || {
                    let _ = sender.send(ZoteroSettings::load().map_err(|error| error.to_string()));
                });
                let id_entry = id_entry.clone();
                let save_button = save_button.clone();
                let settings = settings.clone();
                let dialog = dialog.clone();
                let state = state.clone();
                let retry_for_result = retry_for_click.clone();
                poll_result(receiver, move |result| match result {
                    Ok(loaded) => {
                        id_entry.set_text(&loaded.user_id);
                        *settings.borrow_mut() = Some(loaded);
                        save_button.set_sensitive(true);
                        dialog.content_area().remove(&retry_for_result);
                    }
                    Err(error) => {
                        retry_for_result.set_sensitive(true);
                        show_toast(
                            &state,
                            &format!("{}: {error}", tr("Could not load Zotero settings")),
                        );
                    }
                });
            });
        }
    });
}

fn poll_result<T: Send + 'static>(receiver: mpsc::Receiver<T>, callback: impl FnOnce(T) + 'static) {
    let callback = Rc::new(RefCell::new(Some(callback)));
    glib::timeout_add_local(Duration::from_millis(120), move || {
        match receiver.try_recv() {
            Ok(result) => {
                if let Some(callback) = callback.borrow_mut().take() {
                    callback(result);
                }
                glib::ControlFlow::Break
            }
            Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
            Err(mpsc::TryRecvError::Disconnected) => glib::ControlFlow::Break,
        }
    });
}

fn sync_zotero(state: Rc<State>) {
    if !library_is_ready(&state) {
        return;
    }
    state.library_busy.set(true);
    state.sync_button.set_sensitive(false);
    state.status.set_label(&tr("Syncing…"));
    show_toast(&state, &tr("Syncing references…"));
    let mut library = state.library.borrow().clone();
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let result = crate::zotero::load_api_key().and_then(|key| {
            let key = key.ok_or_else(|| tr("No Zotero API key is saved."))?;
            let settings = ZoteroSettings::load().map_err(|error| error.to_string())?;
            let mut client = crate::zotero::ZoteroClient::new(settings.user_id, key)?;
            client.sync(&mut library)
        });
        let _ = sender.send((result, library));
    });
    poll_result(receiver, move |(result, updated)| {
        state.library_busy.set(false);
        state.sync_button.set_sensitive(true);
        state.status.set_label(&tr("Ready"));
        match result {
            Ok(report) => {
                *state.library.borrow_mut() = updated;
                refresh_library(&state);
                let count_message = |singular: &str, plural: &str, count: usize| {
                    ngettext(singular, plural, count).replacen("%d", &count.to_string(), 1)
                };
                let mut messages = vec![
                    count_message(
                        "%d reference downloaded",
                        "%d references downloaded",
                        report.imported,
                    ),
                    count_message(
                        "%d reference uploaded",
                        "%d references uploaded",
                        report.created,
                    ),
                    count_message(
                        "%d reference updated locally",
                        "%d references updated locally",
                        report.updated_locally,
                    ),
                ];
                if !report.conflicts.is_empty() {
                    messages.push(count_message(
                        "%d conflict kept locally",
                        "%d conflicts kept locally",
                        report.conflicts.len(),
                    ));
                }
                if report.failed > 0 {
                    messages.push(count_message(
                        "%d upload failed",
                        "%d uploads failed",
                        report.failed,
                    ));
                }
                let message = messages.join(" · ");
                show_toast(&state, &message);
            }
            Err(error) => show_toast(&state, &format!("{}: {error}", tr("Zotero sync failed"))),
        }
    });
}

fn choose_pdf_export(state: Rc<State>) {
    let dialog = gtk::FileChooserNative::new(
        Some(&tr("Export PDF")),
        Some(&state.window),
        gtk::FileChooserAction::Save,
        Some(&tr("Export")),
        Some(&tr("Cancel")),
    );
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

fn compile(state: &Rc<State>, export_to: Option<PathBuf>) {
    if state.editor.dirty() {
        if let Some(path) = state.current_file.borrow().clone() {
            let export_after_save = export_to.clone();
            let after: SaveCallback =
                Rc::new(move |state| compile(&state, export_after_save.clone()));
            save_document_to_path(state.clone(), path, Some(after));
        } else {
            show_toast(state, &tr("Save the document before compiling."));
        }
        return;
    }
    let main = state.main_tex.borrow().clone().or_else(|| {
        state.current_file.borrow().clone().filter(|path| {
            path.extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("tex"))
        })
    });
    let Some(main) = main else {
        if let Some(root) = state.project_root.borrow().clone() {
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
                        *state_for_result.main_tex.borrow_mut() = Some(main.clone());
                        compile(&state_for_result, export_to);
                    } else {
                        show_no_main_document(&state_for_result);
                    }
                }
                Err(error) => {
                    set_build_status_with_message(&state_for_result, "Error", Some(&error));
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
    state.main_tex.replace(Some(main.clone()));
    set_build_status_with_message(state, "Building…", Some(&tr("Compilation is in progress.")));
    let library = state.library.borrow().bibliography.clone();
    let active_source = if state.current_file.borrow().as_deref() == Some(main.as_path()) {
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
        let result =
            source.and_then(|source| crate::build::compile_latex(&main, &source, &library));
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
    poll_result(receiver, move |(result, export_result)| match result {
        Ok(build) => {
            if build.added_references > 0 {
                let message = ngettext(
                    "%d local reference added to the temporary bibliography for this build",
                    "%d local references added to the temporary bibliography for this build",
                    build.added_references,
                )
                .replacen("%d", &build.added_references.to_string(), 1);
                show_toast(&state, &message);
            }
            if build.conflicts > 0 {
                let message = ngettext(
                    "%d duplicate citation key was left unchanged",
                    "%d duplicate citation keys were left unchanged",
                    build.conflicts,
                )
                .replacen("%d", &build.conflicts.to_string(), 1);
                show_toast(&state, &message);
            }
            if export_to.is_some() {
                match export_result.unwrap_or_else(|| Err(tr("Could not export PDF"))) {
                    Ok(()) => {
                        set_build_status_with_message(&state, "Built", Some(&tr("PDF exported")));
                        show_toast(&state, &tr("PDF exported"));
                    }
                    Err(error) => {
                        set_build_status_with_message(&state, "Error", Some(&error));
                        show_toast(&state, &format!("{}: {error}", tr("Could not export PDF")));
                    }
                }
            } else {
                match state.pdf_preview.open(&build.pdf_path) {
                    Ok(()) => {
                        set_build_status_with_message(
                            &state,
                            "Built",
                            Some(&tr("Compilation finished")),
                        );
                        show_toast(&state, &tr("Compilation finished"));
                    }
                    Err(error) => {
                        set_build_status_with_message(&state, "Error", Some(&error));
                        state.pdf_preview.set_message(&tr("Could not open the PDF"));
                        show_toast(
                            &state,
                            &format!("{}: {error}", tr("Could not open the compiled PDF")),
                        );
                        show_build_error(&state, &error, &tr("Could not open the PDF preview"));
                    }
                }
            }
        }
        Err(error) => {
            set_build_status_with_message(&state, "Error", Some(&error));
            show_toast(
                &state,
                &format!("{}: {error}", tr("Could not compile the document")),
            );
            show_build_error(&state, &error, &tr("Could not compile the document"));
        }
    });
}

fn show_no_main_document(state: &Rc<State>) {
    let message = tr("Open or create a LaTeX project before compiling.");
    set_build_status(state, "Ready");
    show_toast(state, &message);
}

fn show_build_error(state: &Rc<State>, message: &str, title: &str) {
    state.pdf_preview.set_message(&tr("Check the build output"));
    let dialog = gtk::MessageDialog::builder()
        .transient_for(&state.window)
        .modal(true)
        .message_type(gtk::MessageType::Error)
        .buttons(gtk::ButtonsType::Close)
        .text(title)
        .secondary_text(
            message
                .chars()
                .rev()
                .take(5000)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect::<String>(),
        )
        .build();
    dialog.connect_response(|dialog, _| dialog.close());
    dialog.present();
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
        let sidebar = super::build_sidebar(super::ThemePreference::System);

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
