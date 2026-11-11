use super::*;

pub(in crate::app::window) fn show_reference_transfer_dialog(
    state: Rc<State>,
    direction: TransferDirection,
) {
    let (title, description, icon_name) = match direction {
        TransferDirection::Import => (
            tr("Import references"),
            tr("Choose a format to import."),
            "document-open-symbolic",
        ),
        TransferDirection::Export => (
            tr("Export references"),
            tr("Choose a format to export."),
            "document-save-symbolic",
        ),
    };
    let dialog = gtk::Dialog::builder()
        .title(title)
        .transient_for(&state.window)
        .modal(true)
        .default_width(460)
        .build();
    dialog.add_button(&tr("Close"), gtk::ResponseType::Close);

    let content = dialog.content_area();
    content.set_spacing(12);
    content.set_margin_top(18);
    content.set_margin_bottom(18);
    content.set_margin_start(20);
    content.set_margin_end(20);
    let description = gtk::Label::new(Some(&description));
    description.set_xalign(0.0);
    description.add_css_class("dim-label");
    content.append(&description);

    for format in reference_exchange::AVAILABLE_FORMATS {
        let format = *format;
        let button = gtk::Button::new();
        button.add_css_class("card");
        button.set_hexpand(true);
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        row.set_margin_top(12);
        row.set_margin_bottom(12);
        row.set_margin_start(12);
        row.set_margin_end(12);

        let icon = gtk::Image::from_icon_name(icon_name);
        icon.set_pixel_size(24);
        row.append(&icon);

        let text = gtk::Box::new(gtk::Orientation::Vertical, 3);
        text.set_hexpand(true);
        let title = gtk::Label::new(Some(format.label()));
        title.set_xalign(0.0);
        title.add_css_class("heading");
        text.append(&title);
        let detail = gtk::Label::new(Some(&tr("Standard format for bibliographic references.")));
        detail.set_xalign(0.0);
        detail.add_css_class("dim-label");
        text.append(&detail);
        row.append(&text);

        let next = gtk::Image::from_icon_name("go-next-symbolic");
        next.add_css_class("dim-label");
        row.append(&next);
        button.set_child(Some(&row));

        let weak_dialog = dialog.downgrade();
        let weak_state = Rc::downgrade(&state);
        button.connect_clicked(move |_| {
            if let Some(dialog) = weak_dialog.upgrade() {
                dialog.close();
            }
            if let Some(state) = weak_state.upgrade() {
                match reference_exchange::action_for(direction, format) {
                    TransferAction::ImportBibtex => import_bibtex(state),
                    TransferAction::ExportBibtex => export_bibtex(state),
                }
            }
        });
        content.append(&button);
    }

    content.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    let guides_label = gtk::Label::new(Some(&tr("Platform guides")));
    guides_label.set_xalign(0.0);
    guides_label.add_css_class("dim-label");
    content.append(&guides_label);

    let guides = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    for guide in REFERENCE_MANAGER_GUIDES {
        let link = gtk::LinkButton::with_label(guide.uri, guide.name);
        link.set_halign(gtk::Align::Start);
        guides.append(&link);
    }
    content.append(&guides);

    dialog.connect_response(|dialog, _| dialog.close());
    dialog.present();
}

pub(in crate::app::window) fn import_bibtex(state: Rc<State>) {
    if !library_is_ready(&state) {
        return;
    }
    choose_file(
        &state,
        gtk::FileChooserAction::Open,
        "Import BibTeX",
        "Import",
        None,
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

pub(in crate::app::window) fn export_bibtex(state: Rc<State>) {
    if !library_is_ready(&state) {
        return;
    }
    choose_file(
        &state,
        gtk::FileChooserAction::Save,
        "Export BibTeX",
        "Export",
        Some("export_bibitex_ovenbird.bib"),
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
