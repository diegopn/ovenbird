use super::*;

pub(super) fn show_preferences_dialog(state: &Rc<State>) {
    let dialog = gtk::Dialog::builder()
        .title(tr("Preferences"))
        .transient_for(&state.window)
        .modal(true)
        .default_width(560)
        .build();
    dialog.add_button(&tr("Close"), gtk::ResponseType::Close);

    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.set_margin_top(18);
    content.set_margin_bottom(18);
    content.set_margin_start(20);
    content.set_margin_end(20);
    content.append(&build_theme_choices(load_theme_preference()));
    content.append(&gtk::Separator::new(gtk::Orientation::Horizontal));

    let startup_view_options = [tr("LaTeX editor"), tr("References")];
    let startup_view_labels = startup_view_options
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let startup_view_dropdown = gtk::DropDown::from_strings(&startup_view_labels);
    startup_view_dropdown.set_selected(startup_view_menu_index(load_startup_view_preference()));
    startup_view_dropdown.connect_selected_notify(|dropdown| {
        save_startup_view_preference(startup_view_from_menu_index(dropdown.selected()));
    });
    let startup_view_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let startup_view_label = gtk::Label::new(Some(&tr("Open at startup")));
    startup_view_label.set_xalign(0.0);
    startup_view_label.set_hexpand(true);
    startup_view_row.append(&startup_view_label);
    startup_view_row.append(&startup_view_dropdown);
    content.append(&startup_view_row);

    let reopen_last_project = gtk::CheckButton::with_label(&tr("Reopen last project at startup"));
    reopen_last_project.set_active(load_reopen_last_project_preference());
    reopen_last_project.connect_toggled(|button| {
        save_reopen_last_project_preference(button.is_active());
    });
    content.append(&reopen_last_project);
    content.append(&gtk::Separator::new(gtk::Orientation::Horizontal));

    let default_folder_title = gtk::Label::new(Some(&tr("Default folder for file dialogs")));
    default_folder_title.set_xalign(0.0);
    default_folder_title.add_css_class("heading");
    content.append(&default_folder_title);
    let default_folder_description = gtk::Label::new(Some(&tr(
        "Dialogs start in Downloads unless you choose another folder.",
    )));
    default_folder_description.set_xalign(0.0);
    default_folder_description.add_css_class("dim-label");
    content.append(&default_folder_description);

    let default_folder_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let default_folder_label = gtk::Label::new(Some(&default_folder_display_text()));
    default_folder_label.set_xalign(0.0);
    default_folder_label.set_hexpand(true);
    default_folder_label.set_selectable(true);
    default_folder_label.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
    default_folder_row.append(&default_folder_label);

    let reset_folder = gtk::Button::with_label(&tr("Reset"));
    reset_folder.set_tooltip_text(Some(&tr("Restore the system default folder")));
    reset_folder.add_css_class("flat");
    reset_folder.set_visible(has_custom_file_dialog_folder());

    let choose_folder = gtk::Button::with_label(&tr("Choose folder…"));
    let folder_label_for_choice = default_folder_label.clone();
    let reset_folder_for_choice = reset_folder.clone();
    let state_for_choice = state.clone();
    choose_folder.connect_clicked(move |_| {
        let folder_label = folder_label_for_choice.clone();
        let reset_folder = reset_folder_for_choice.clone();
        choose_file(
            &state_for_choice,
            gtk::FileChooserAction::SelectFolder,
            "Choose default folder",
            "Open",
            None,
            move |state, folder| {
                if let Err(error) = save_default_file_dialog_folder(Some(&folder)) {
                    show_toast(&state, &error.to_string());
                    return;
                }
                folder_label.set_text(&folder.display().to_string());
                reset_folder.set_visible(true);
            },
        );
    });
    default_folder_row.append(&choose_folder);
    let folder_label_for_reset = default_folder_label.clone();
    let reset_folder_for_reset = reset_folder.clone();
    let state_for_reset = state.clone();
    reset_folder.connect_clicked(move |_| {
        if let Err(error) = save_default_file_dialog_folder(None) {
            show_toast(&state_for_reset, &error.to_string());
            return;
        }
        folder_label_for_reset.set_text(&default_folder_display_text());
        reset_folder_for_reset.set_visible(false);
    });
    default_folder_row.append(&reset_folder);
    content.append(&default_folder_row);

    dialog.content_area().append(&content);
    dialog.connect_response(|dialog, _| dialog.close());
    dialog.present();
}

pub(super) fn default_folder_display_text() -> String {
    default_file_dialog_folder()
        .map(|folder| folder.display().to_string())
        .unwrap_or_else(|| tr("No starting folder available"))
}

pub(super) fn load_theme_preference() -> ThemePreference {
    AppSettings::load()
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

pub(super) fn load_reopen_last_project_preference() -> bool {
    AppSettings::load()
        .map(|settings| settings.reopen_last_project_on_startup)
        .unwrap_or(true)
}

pub(super) fn load_startup_view_preference() -> StartupView {
    AppSettings::load()
        .map(|settings| settings.startup_view)
        .unwrap_or_default()
}

pub(super) fn save_startup_view_preference(startup_view: StartupView) {
    if let Err(error) = update_app_settings(|settings| {
        settings.startup_view = startup_view;
    }) {
        eprintln!("Could not save startup preference: {error}");
    }
}

pub(super) fn save_default_file_dialog_folder(folder: Option<&Path>) -> std::io::Result<()> {
    let folder_uri = folder.map(|folder| gio::File::for_path(folder).uri().to_string());
    update_app_settings(|settings| settings.default_folder_uri = folder_uri)
}

pub(super) fn has_custom_file_dialog_folder() -> bool {
    AppSettings::load()
        .ok()
        .is_some_and(|settings| settings.default_folder_uri.is_some())
}

pub(super) fn default_file_dialog_folder() -> Option<PathBuf> {
    let settings = AppSettings::load().unwrap_or_default();
    preferred_file_dialog_folder(settings.default_folder_uri.as_deref(), downloads_folder())
}

pub(super) fn preferred_file_dialog_folder(
    configured_uri: Option<&str>,
    downloads_folder: Option<PathBuf>,
) -> Option<PathBuf> {
    let configured = configured_uri
        .and_then(|uri| gio::File::for_uri(uri).path())
        .filter(|folder| folder.is_dir());
    configured.or_else(|| downloads_folder.filter(|folder| folder.is_dir()))
}

pub(super) fn downloads_folder() -> Option<PathBuf> {
    let fallback = glib::home_dir().join("Downloads");
    glib::user_special_dir(glib::UserDirectory::Downloads)
        .filter(|path| path.is_dir())
        .or_else(|| fallback.is_dir().then_some(fallback))
}

pub(super) fn startup_page_name(startup_view: StartupView) -> &'static str {
    match startup_view {
        StartupView::Editor => "editor",
        StartupView::References => "references",
    }
}

pub(super) fn startup_view_menu_index(startup_view: StartupView) -> u32 {
    match startup_view {
        StartupView::Editor => 0,
        StartupView::References => 1,
    }
}

pub(super) fn startup_view_from_menu_index(index: u32) -> StartupView {
    match index {
        1 => StartupView::References,
        _ => StartupView::Editor,
    }
}

pub(super) fn update_app_settings(update: impl FnOnce(&mut AppSettings)) -> std::io::Result<()> {
    let mut settings = match AppSettings::load() {
        Ok(settings) => settings,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => AppSettings::default(),
        Err(error) => return Err(error),
    };
    update(&mut settings);
    settings.save()
}

pub(super) fn save_reopen_last_project_preference(enabled: bool) {
    if let Err(error) = update_app_settings(|settings| {
        settings.reopen_last_project_on_startup = enabled;
    }) {
        eprintln!("Could not save startup preference: {error}");
    }
}

pub(super) fn remember_last_project_folder(folder: &Path) {
    let uri = gio::File::for_path(folder).uri().to_string();
    if let Err(error) = update_app_settings(|settings| settings.last_project_uri = Some(uri)) {
        eprintln!("Could not save the last project folder: {error}");
    }
}

pub(super) fn schedule_last_project_restore(state: &Rc<State>) {
    let state = Rc::downgrade(state);
    glib::idle_add_local_once(move || {
        let Some(state) = state.upgrade() else {
            return;
        };
        if state.startup_restore_cancelled.get() {
            return;
        }

        let settings = match AppSettings::load() {
            Ok(settings) => settings,
            Err(error) => {
                eprintln!("Could not load startup settings: {error}");
                return;
            }
        };
        if !settings.reopen_last_project_on_startup {
            return;
        }
        let Some(uri) = settings.last_project_uri else {
            return;
        };
        let Some(folder) = gio::File::for_uri(&uri).path() else {
            show_toast(
                &state,
                &tr("The last project folder could not be restored."),
            );
            return;
        };
        if std::fs::read_dir(&folder).is_err() {
            show_toast(
                &state,
                &tr("The last project folder could not be restored."),
            );
            return;
        }

        open_project(&state, &folder, true, state.startup_view);
    });
}

pub(super) fn show_about_dialog(state: &Rc<State>) {
    let dialog = adw::Dialog::builder()
        .title(tr("About Ovenbird"))
        .content_width(420)
        .content_height(680)
        .build();
    let toolbar = adw::ToolbarView::new();
    let header = adw::HeaderBar::new();
    header.set_show_start_title_buttons(false);
    header.set_title_widget(Some(&gtk::Label::new(None)));
    toolbar.add_top_bar(&header);

    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.set_margin_top(12);
    content.set_margin_bottom(24);
    content.set_margin_start(24);
    content.set_margin_end(24);

    let icon = gtk::Image::from_icon_name("io.github.diegopn.ovenbird");
    icon.set_pixel_size(128);
    content.append(&icon);

    let name = gtk::Label::new(Some("Ovenbird"));
    name.add_css_class("title-1");
    content.append(&name);

    let creator = gtk::Label::new(Some(&tr("Created by Diego Pereira do Nascimento")));
    creator.set_wrap(true);
    creator.set_justify(gtk::Justification::Center);
    content.append(&creator);

    let version = gtk::Label::new(Some(env!("CARGO_PKG_VERSION")));
    version.set_halign(gtk::Align::Center);
    version.set_margin_bottom(12);
    version.add_css_class("ovenbird-about-version");
    content.append(&version);

    let dedication = gtk::Label::new(Some(&tr_dynamic(crate::application_menu::ABOUT_DEDICATION)));
    dedication.set_wrap(true);
    dedication.set_xalign(0.5);
    dedication.set_justify(gtk::Justification::Center);
    dedication.set_margin_bottom(6);
    content.append(&dedication);

    let link_groups = gtk::Box::new(gtk::Orientation::Vertical, 16);
    for group in crate::application_menu::ABOUT_LINK_GROUPS {
        let links = gtk::ListBox::new();
        links.set_selection_mode(gtk::SelectionMode::None);
        links.add_css_class("boxed-list");
        for &(label, url) in *group {
            let link = gtk::LinkButton::with_label(url, &tr_dynamic(label));
            link.add_css_class("flat");
            link.add_css_class("ovenbird-about-link");
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
            let label = gtk::Label::new(Some(&tr_dynamic(label)));
            label.set_xalign(0.0);
            label.set_hexpand(true);
            row.append(&label);
            row.append(&gtk::Image::from_icon_name("adw-external-link-symbolic"));
            link.set_child(Some(&row));
            links.append(&link);
        }
        link_groups.append(&links);
    }
    content.append(&link_groups);

    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&content)
        .build();
    toolbar.set_content(Some(&scroll));
    dialog.set_child(Some(&toolbar));
    dialog.present(Some(&state.window));
}

pub(super) fn apply_theme_preference(preference: ThemePreference) {
    let scheme = match preference {
        ThemePreference::System => adw::ColorScheme::Default,
        ThemePreference::Light => adw::ColorScheme::ForceLight,
        ThemePreference::Dark => adw::ColorScheme::ForceDark,
    };
    adw::StyleManager::default().set_color_scheme(scheme);
}

pub(super) fn save_theme_preference(preference: ThemePreference) {
    if let Err(error) = update_app_settings(|settings| {
        settings.extra.insert(
            "colorScheme".to_owned(),
            serde_json::Value::String(preference.setting_value().to_owned()),
        );
    }) {
        eprintln!("Could not save appearance preference: {error}");
    }
}

pub(super) fn build_theme_choices(current: ThemePreference) -> gtk::Box {
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
