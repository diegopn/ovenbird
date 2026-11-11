use super::*;

pub(super) fn build_reference_tag_editor(
    state: &Rc<State>,
    entry: &Option<BibEntry>,
) -> ReferenceTagWidgets {
    let selected_tags = Rc::new(RefCell::new(
        entry
            .as_ref()
            .map(|entry| parse_tag_values(entry.get("tags")))
            .unwrap_or_default(),
    ));
    let available_tags = Rc::new(RefCell::new(state.library.borrow().tags.clone()));
    for tag in selected_tags.borrow().iter() {
        let missing = {
            let available = available_tags.borrow();
            !available
                .iter()
                .any(|existing| existing.eq_ignore_ascii_case(tag))
        };
        if missing {
            available_tags.borrow_mut().push(tag.clone());
        }
    }
    let tag_chips = gtk::FlowBox::new();
    tag_chips.set_selection_mode(gtk::SelectionMode::None);
    tag_chips.set_row_spacing(4);
    tag_chips.set_column_spacing(4);
    tag_chips.set_max_children_per_line(8);
    tag_chips.set_min_children_per_line(1);
    tag_chips.set_hexpand(true);
    tag_chips.set_halign(gtk::Align::Fill);
    refresh_reference_tag_chips(&tag_chips, &selected_tags);

    let tag_editor = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    tag_editor.set_hexpand(true);
    tag_editor.append(&tag_chips);

    let picker_popover = gtk::Popover::new();
    let picker_content = gtk::Box::new(gtk::Orientation::Vertical, 8);
    picker_content.set_margin_top(8);
    picker_content.set_margin_bottom(8);
    picker_content.set_margin_start(8);
    picker_content.set_margin_end(8);
    picker_content.set_size_request(260, -1);
    let tag_search = gtk::SearchEntry::new();
    tag_search.set_placeholder_text(Some(&tr("Find tags…")));
    picker_content.append(&tag_search);
    let tag_choices = gtk::Box::new(gtk::Orientation::Vertical, 2);
    tag_choices.set_hexpand(true);
    let tag_choices_scroll = gtk::ScrolledWindow::builder()
        .min_content_height(36)
        .max_content_height(180)
        .child(&tag_choices)
        .build();
    picker_content.append(&tag_choices_scroll);
    let create_row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let new_tag_entry = gtk::Entry::new();
    new_tag_entry.set_placeholder_text(Some(&tr("New tag")));
    new_tag_entry.set_hexpand(true);
    let create_tag_button = gtk::Button::with_label(&tr("Create"));
    create_row.append(&new_tag_entry);
    create_row.append(&create_tag_button);
    picker_content.append(&create_row);
    picker_popover.set_child(Some(&picker_content));
    refresh_reference_tag_picker(
        &tag_choices,
        &tag_chips,
        &selected_tags,
        &available_tags,
        "",
    );
    tag_search.connect_search_changed({
        let tag_choices = tag_choices.clone();
        let tag_chips = tag_chips.clone();
        let selected_tags = selected_tags.clone();
        let available_tags = available_tags.clone();
        move |search| {
            refresh_reference_tag_picker(
                &tag_choices,
                &tag_chips,
                &selected_tags,
                &available_tags,
                search.text().as_str(),
            );
        }
    });
    create_tag_button.connect_clicked({
        let new_tag_entry = new_tag_entry.clone();
        let tag_choices = tag_choices.clone();
        let tag_chips = tag_chips.clone();
        let selected_tags = selected_tags.clone();
        let available_tags = available_tags.clone();
        move |_| {
            if add_tag_selection(
                new_tag_entry.text().as_str(),
                &available_tags,
                &selected_tags,
            ) {
                new_tag_entry.set_text("");
                refresh_reference_tag_chips(&tag_chips, &selected_tags);
                refresh_reference_tag_picker(
                    &tag_choices,
                    &tag_chips,
                    &selected_tags,
                    &available_tags,
                    "",
                );
            }
        }
    });
    create_tag_button.set_tooltip_text(Some(&tr("Create and select this tag")));
    new_tag_entry.connect_activate({
        let create_tag_button = create_tag_button.clone();
        move |_| create_tag_button.emit_clicked()
    });
    let add_tag_button = gtk::MenuButton::new();
    add_tag_button.set_child(Some(&labeled_content("list-add-symbolic", &tr("Add tag"))));
    add_tag_button.set_tooltip_text(Some(&tr("Select or create tags")));
    add_tag_button.set_popover(Some(&picker_popover));
    add_tag_button.connect_active_notify({
        let tag_choices = tag_choices.clone();
        let tag_chips = tag_chips.clone();
        let selected_tags = selected_tags.clone();
        let available_tags = available_tags.clone();
        move |button| {
            if button.is_active() {
                refresh_reference_tag_picker(
                    &tag_choices,
                    &tag_chips,
                    &selected_tags,
                    &available_tags,
                    tag_search.text().as_str(),
                );
            }
        }
    });
    tag_editor.append(&add_tag_button);
    let tag_label = gtk::Label::new(Some(&tr("Tags")));
    tag_label.set_xalign(1.0);
    tag_label.add_css_class("dim-label");
    ReferenceTagWidgets {
        label: tag_label,
        editor: tag_editor,
        selected: selected_tags,
    }
}
