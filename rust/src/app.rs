use adw::prelude::*;
use gtk::gio;
use gtk::glib;

#[link(name = "gtk-4")]
unsafe extern "C" {
    fn gtk_style_context_add_provider_for_display(
        display: *mut gtk::gdk::ffi::GdkDisplay,
        provider: *mut gtk::ffi::GtkStyleProvider,
        priority: u32,
    );
}

fn install_css() {
    let provider = gtk::CssProvider::new();
    provider.load_from_string(include_str!("../../data/ovenbird.css"));
    if let Some(display) = gtk::gdk::Display::default() {
        unsafe {
            gtk_style_context_add_provider_for_display(
                display.as_ptr(),
                provider.as_ptr().cast(),
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }
    }
}

fn use_existing_window_or_create<W>(
    active_window: Option<W>,
    present_existing: impl FnOnce(W),
    create: impl FnOnce(),
) {
    if let Some(window) = active_window {
        present_existing(window);
    } else {
        create();
    }
}

pub fn run() -> glib::ExitCode {
    let application = adw::Application::builder()
        .application_id("io.github.diegopn.ovenbird")
        .flags(gio::ApplicationFlags::HANDLES_OPEN)
        .build();
    application.connect_activate(|application| {
        use_existing_window_or_create(
            application.active_window(),
            |window| window.present(),
            || crate::window::OvenbirdWindow::new(application).present(),
        );
    });
    application.connect_open(|application, files, _hint| {
        application.activate();
        let Some(window) = application.active_window() else {
            return;
        };
        window.present();
        if let Some(path) = files.iter().find_map(|file| file.path()) {
            let path = path.to_string_lossy().into_owned();
            let _ = window.activate_action("win.open-path", Some(&path.to_variant()));
        }
    });
    application.connect_startup(|application| {
        crate::libpanel::initialize();
        install_css();
        application.set_accels_for_action("win.new-project", &["<Primary><Alt>n"]);
        application.set_accels_for_action("win.open-document", &["<Primary>o"]);
        application.set_accels_for_action("win.open-project", &["<Primary><Alt>o"]);
        application.set_accels_for_action("win.close-project", &["<Primary>w"]);
        application.set_accels_for_action("win.save-document", &["<Primary>s"]);
        application.set_accels_for_action("win.find-document", &["<Primary>f"]);
        application.set_accels_for_action("win.compile", &["<Primary><Shift>r"]);
        application.set_accels_for_action("win.insert-citation", &["<Primary><Shift>c"]);
        application.set_accels_for_action("win.toggle-sidebar", &["F9"]);
        application.set_accels_for_action("win.undo", &["<Primary>z"]);
        application.set_accels_for_action("win.redo", &["<Primary><Shift>z"]);
        application.set_accels_for_action("win.shortcuts", &["<Primary>question"]);
        application.set_accels_for_action("win.quit", &["<Primary>q"]);
    });
    application.run()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn activation_presents_existing_window_without_creating_another() {
        let presented = Cell::new(None);
        let created = Cell::new(false);

        use_existing_window_or_create(
            Some("active"),
            |window| presented.set(Some(window)),
            || created.set(true),
        );

        assert_eq!(presented.get(), Some("active"));
        assert!(!created.get());
    }

    #[test]
    fn activation_creates_window_when_none_is_active() {
        let presented = Cell::new(false);
        let created = Cell::new(false);

        use_existing_window_or_create(None::<()>, |_| presented.set(true), || created.set(true));

        assert!(created.get());
        assert!(!presented.get());
    }
}
