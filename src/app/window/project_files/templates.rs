use super::*;

pub(in crate::app::window) fn show_template_picker(state: Rc<State>) {
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
            None,
            move |state, parent| {
                prompt_project_name(&state, &parent, id);
            },
        );
    });
    dialog.present();
}

pub(in crate::app::window) fn prompt_project_name(state: &Rc<State>, parent: &Path, id: &str) {
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
            state2.editor_ui.status.set_label(&tr("Creating project…"));
            let state_for_result = state2.clone();
            poll_result(receiver, move |result| {
                progress.close();
                state_for_result.editor_ui.status.set_label(&tr("Ready"));
                match result {
                    Ok((folder, main)) => {
                        state_for_result
                            .project
                            .expanded_folders
                            .borrow_mut()
                            .clear();
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
                state_for_result
                    .project
                    .expanded_folders
                    .borrow_mut()
                    .clear();
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

pub(in crate::app::window) fn template_resource_root() -> PathBuf {
    let installed = option_env!("OVENBIRD_TEMPLATE_DIR").map(Path::new);
    crate::templates::resource_root(installed, Path::new(env!("CARGO_MANIFEST_DIR")))
}
