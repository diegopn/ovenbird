use super::*;

pub(in crate::app::window) fn setup_reference_list_view(state: &Rc<State>) {
    let factory = gtk::SignalListItemFactory::new();
    let state_for_setup = Rc::downgrade(state);
    factory.connect_setup(move |_, object| {
        let Some(item) = object.downcast_ref::<gtk::ListItem>() else {
            return;
        };
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        row.set_hexpand(true);
        row.set_margin_top(4);
        row.set_margin_bottom(4);

        let details = gtk::Box::new(gtk::Orientation::Vertical, 3);
        let title = gtk::Label::new(None);
        title.set_xalign(0.0);
        title.set_hexpand(true);
        title.set_wrap(true);
        title.set_wrap_mode(gtk::pango::WrapMode::WordChar);
        title.add_css_class("heading");
        details.append(&title);
        let subtitle = gtk::Label::new(None);
        subtitle.set_xalign(0.0);
        subtitle.set_hexpand(true);
        subtitle.set_wrap(true);
        subtitle.set_wrap_mode(gtk::pango::WrapMode::WordChar);
        subtitle.add_css_class("dim-label");
        details.append(&subtitle);
        details.set_hexpand(true);
        let summary = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        summary.set_hexpand(true);
        summary.append(&details);
        let key = gtk::Label::new(None);
        key.add_css_class("dim-label");
        summary.append(&key);

        let open_details = gtk::Button::new();
        open_details.set_has_frame(false);
        open_details.add_css_class("flat");
        open_details.set_hexpand(true);
        open_details.set_halign(gtk::Align::Fill);
        open_details.set_child(Some(&summary));
        open_details.set_tooltip_text(Some(&tr("Show reference details")));
        let item_for_details = item.downgrade();
        let state_for_details = state_for_setup.clone();
        open_details.connect_clicked(move |_| {
            let Some(state) = state_for_details.upgrade() else {
                return;
            };
            let Some(item) = item_for_details.upgrade() else {
                return;
            };
            let Some(entry) = reference_list_entry(&item) else {
                return;
            };
            show_reference_details(&state, entry);
        });
        row.append(&open_details);

        let edit = button("document-edit-symbolic", &tr("Edit reference"));
        let item_for_edit = item.downgrade();
        let state_for_edit = state_for_setup.clone();
        edit.connect_clicked(move |_| {
            let Some(state) = state_for_edit.upgrade() else {
                return;
            };
            let Some(item) = item_for_edit.upgrade() else {
                return;
            };
            let Some(entry) = reference_list_entry(&item) else {
                return;
            };
            edit_reference(&state, Some(entry));
        });
        row.append(&edit);

        let delete = button("user-trash-symbolic", &tr("Delete reference"));
        delete.add_css_class("destructive-action");
        let item_for_delete = item.downgrade();
        let state_for_delete = state_for_setup.clone();
        delete.connect_clicked(move |_| {
            let Some(state) = state_for_delete.upgrade() else {
                return;
            };
            let Some(item) = item_for_delete.upgrade() else {
                return;
            };
            let Some(entry) = reference_list_entry(&item) else {
                return;
            };
            let parent: &gtk::Window = state.window.upcast_ref();
            request_reference_deletion(&state, parent, entry, || {});
        });
        row.append(&delete);

        let message_page = gtk::Box::new(gtk::Orientation::Vertical, 8);
        message_page.set_margin_top(24);
        message_page.set_margin_bottom(24);
        let message = gtk::Label::new(None);
        message.set_xalign(0.0);
        message.set_wrap(true);
        message.add_css_class("dim-label");
        message_page.append(&message);
        let retry = gtk::Button::with_label(&tr("Retry"));
        retry.set_halign(gtk::Align::Start);
        let state_for_retry = state_for_setup.clone();
        retry.connect_clicked(move |_| {
            if let Some(state) = state_for_retry.upgrade() {
                load_library_async(&state);
            }
        });
        message_page.append(&retry);

        let stack = gtk::Stack::new();
        stack.set_transition_type(gtk::StackTransitionType::None);
        stack.add_named(&row, Some("entry"));
        stack.add_named(&message_page, Some("message"));
        item.set_child(Some(&stack));
    });

    factory.connect_bind(|_, object| {
        let Some(item) = object.downcast_ref::<gtk::ListItem>() else {
            return;
        };
        bind_reference_list_item(item);
    });
    state.library_ui.library_list.set_factory(Some(&factory));
}

pub(in crate::app::window) fn reference_list_entry(item: &gtk::ListItem) -> Option<BibEntry> {
    let item = item.item()?.downcast::<glib::BoxedAnyObject>().ok()?;
    let row = item.borrow::<ReferenceListItem>();
    match &*row {
        ReferenceListItem::Entry(entry) => Some(entry.clone()),
        ReferenceListItem::Message { .. } => None,
    }
}

pub(in crate::app::window) fn bind_reference_list_item(item: &gtk::ListItem) {
    let Some(list_item) = item
        .item()
        .and_then(|item| item.downcast::<glib::BoxedAnyObject>().ok())
    else {
        return;
    };
    let list_item = list_item.borrow::<ReferenceListItem>();
    let Some(stack) = item
        .child()
        .and_then(|child| child.downcast::<gtk::Stack>().ok())
    else {
        return;
    };

    match &*list_item {
        ReferenceListItem::Message { text, retry } => {
            let Some(message_page) = stack
                .child_by_name("message")
                .and_then(|child| child.downcast::<gtk::Box>().ok())
            else {
                return;
            };
            let Some(message) = message_page
                .first_child()
                .and_then(|child| child.downcast::<gtk::Label>().ok())
            else {
                return;
            };
            let Some(retry_button) = message
                .next_sibling()
                .and_then(|child| child.downcast::<gtk::Button>().ok())
            else {
                return;
            };
            message.set_text(text);
            retry_button.set_visible(*retry);
            stack.set_visible_child_name("message");
        }
        ReferenceListItem::Entry(entry) => {
            let Some(row) = stack
                .child_by_name("entry")
                .and_then(|child| child.downcast::<gtk::Box>().ok())
            else {
                return;
            };
            let Some(open_details) = row
                .first_child()
                .and_then(|child| child.downcast::<gtk::Button>().ok())
            else {
                return;
            };
            let Some(summary) = open_details
                .child()
                .and_then(|child| child.downcast::<gtk::Box>().ok())
            else {
                return;
            };
            let Some(details) = summary
                .first_child()
                .and_then(|child| child.downcast::<gtk::Box>().ok())
            else {
                return;
            };
            let Some(title_label) = details
                .first_child()
                .and_then(|child| child.downcast::<gtk::Label>().ok())
            else {
                return;
            };
            let Some(subtitle_label) = title_label
                .next_sibling()
                .and_then(|child| child.downcast::<gtk::Label>().ok())
            else {
                return;
            };
            let Some(key_label) = details
                .next_sibling()
                .and_then(|child| child.downcast::<gtk::Label>().ok())
            else {
                return;
            };

            let title = if entry.get("title").is_empty() {
                bibtex::display_bibtex_text(&entry.key)
            } else {
                bibtex::display_bibtex_text(entry.get("title"))
            };
            let author_field = if entry.get("author").is_empty() {
                entry.get("editor")
            } else {
                entry.get("author")
            };
            let author = bibtex::display_bibtex_names(author_field).join(" · ");
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
            title_label.set_text(&title);
            subtitle_label.set_text(&subtitle);
            key_label.set_text(&entry.key);
            stack.set_visible_child_name("entry");
        }
    }
}
