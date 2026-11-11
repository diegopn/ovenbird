use super::*;

pub(in crate::app::window) fn edit_tag(state: &Rc<State>, previous: Option<String>) {
    if !library_is_ready(state) {
        return;
    }
    let dialog = gtk::Dialog::builder()
        .title(tr(if previous.is_some() {
            "Edit tag"
        } else {
            "Add tag"
        }))
        .transient_for(&state.window)
        .modal(true)
        .default_width(420)
        .build();
    dialog.add_button(&tr("Cancel"), gtk::ResponseType::Cancel);
    let save = dialog.add_button(&tr("Save"), gtk::ResponseType::Accept);
    save.add_css_class("suggested-action");
    dialog.set_default_response(gtk::ResponseType::Accept);

    let content = gtk::Box::new(gtk::Orientation::Vertical, 8);
    content.set_margin_top(18);
    content.set_margin_bottom(18);
    content.set_margin_start(20);
    content.set_margin_end(20);
    let input = gtk::Entry::new();
    input.set_placeholder_text(Some(&tr("Tag name")));
    input.set_hexpand(true);
    if let Some(previous) = previous.as_ref() {
        input.set_text(previous);
    }
    content.append(&input);
    dialog.content_area().append(&content);

    let previous_for_response = previous.clone();
    let state_for_response = state.clone();
    let input_for_response = input.clone();
    dialog.connect_response(move |dialog, response| {
        if response == gtk::ResponseType::Accept {
            let mut updated = state_for_response.library.borrow().clone();
            let result = if let Some(previous) = previous_for_response.as_deref() {
                updated.rename_tag(previous, input_for_response.text().as_str())
            } else {
                updated.add_tag(input_for_response.text().as_str())
            };
            match result {
                Ok(()) => {
                    persist_library_async(state_for_response.clone(), updated, tr("Tag saved"));
                    dialog.close();
                }
                Err(error) => show_toast(&state_for_response, &tr(&error)),
            }
        } else {
            dialog.close();
        }
    });
    dialog.present();
    input.grab_focus();
}

pub(in crate::app::window) fn request_tag_deletion(state: &Rc<State>, tag: &str) {
    if !library_is_ready(state) {
        return;
    }
    let message = tr("Remove tag ‘%s’ from the list and all references?").replacen("%s", tag, 1);
    let parent: &gtk::Window = state.window.upcast_ref();
    let tag = tag.to_owned();
    confirm_destructive_action(
        state,
        parent,
        &tr("Confirm deletion"),
        &message,
        &tr("Remove tag"),
        move |state| {
            if !library_is_ready(&state) {
                return;
            }
            let mut updated = state.library.borrow().clone();
            if updated.remove_tag(&tag) {
                persist_library_async(state, updated, tr("Tag removed"));
            }
        },
    );
}
