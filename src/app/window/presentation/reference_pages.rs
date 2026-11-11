use super::*;

pub(in crate::app::window) fn sort_direction_button() -> gtk::ToggleButton {
    let ascending = tr("Ascending");
    let descending = tr("Descending");
    let button = gtk::ToggleButton::new();
    button.set_child(Some(&labeled_content(
        "view-sort-ascending-symbolic",
        &ascending,
    )));
    button.set_tooltip_text(Some(&ascending));
    button.connect_toggled(move |button| {
        let (icon, label) = if button.is_active() {
            ("view-sort-descending-symbolic", descending.as_str())
        } else {
            ("view-sort-ascending-symbolic", ascending.as_str())
        };
        button.set_child(Some(&labeled_content(icon, label)));
        button.set_tooltip_text(Some(label));
    });
    button
}

pub(in crate::app::window) fn build_library_page() -> LibraryPageWidgets {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 10);
    page.set_margin_top(20);
    page.set_margin_bottom(12);
    page.set_margin_start(22);
    page.set_margin_end(22);

    let header = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let title = gtk::Label::new(Some(&tr("Manage references")));
    title.set_xalign(0.0);
    title.add_css_class("title-2");
    title.set_hexpand(true);
    header.append(&title);
    let add = labeled_button("list-add-symbolic", &tr("Add reference"));
    add.set_action_name(Some("win.add-reference"));
    add.add_css_class("suggested-action");
    header.append(&add);
    page.append(&header);

    let search_and_sort = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let search = gtk::SearchEntry::new();
    search.set_placeholder_text(Some(&tr("Search references…")));
    search.set_hexpand(true);
    search_and_sort.append(&search);
    let sort_by =
        gtk::DropDown::from_strings(&[&tr("Title"), &tr("Author"), &tr("Year"), &tr("Tag")]);
    sort_by.set_tooltip_text(Some(&tr("Sort by")));
    sort_by.set_selected(0);
    search_and_sort.append(&sort_by);
    let sort_descending = sort_direction_button();
    search_and_sort.append(&sort_descending);
    page.append(&search_and_sort);

    let filter_bar = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    filter_bar.add_css_class("ovenbird-reference-filters");
    let author = reference_filter_dropdown("All authors");
    let year = reference_filter_dropdown("All years");
    let tag = reference_filter_dropdown("All tags");
    let kind = reference_filter_dropdown("All types");
    for dropdown in [&author, &year, &tag, &kind] {
        dropdown.set_hexpand(true);
        dropdown.set_width_request(150);
        filter_bar.append(dropdown);
    }
    page.append(&filter_bar);
    let list_model = gio::ListStore::new::<glib::BoxedAnyObject>();
    let list_selection = gtk::NoSelection::new(Some(list_model.clone()));
    let list = gtk::ListView::new(Some(list_selection), None::<gtk::ListItemFactory>);
    list.add_css_class("ovenbird-reference-list");
    list.set_show_separators(true);
    list.set_margin_end(30);
    list.set_vexpand(true);
    list.set_hexpand(true);
    let scroll = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .child(&list)
        .build();
    page.append(&scroll);
    LibraryPageWidgets {
        page,
        list,
        list_model,
        search,
        sort_by,
        sort_descending,
        filters: ReferenceFilterWidgets {
            author,
            year,
            tag,
            kind,
        },
    }
}

pub(in crate::app::window) fn build_tags_manager_content() -> TagsPageWidgets {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 10);
    page.set_hexpand(true);
    page.set_vexpand(true);
    page.set_margin_top(20);
    page.set_margin_bottom(12);
    page.set_margin_start(22);
    page.set_margin_end(22);
    let header = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let title = gtk::Label::new(Some(&tr("Manage tags")));
    title.set_xalign(0.0);
    title.add_css_class("title-2");
    title.set_hexpand(true);
    header.append(&title);
    let add = labeled_button("list-add-symbolic", &tr("Add tag"));
    add.add_css_class("suggested-action");
    header.append(&add);
    page.append(&header);

    let search_and_sort = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let search = gtk::SearchEntry::new();
    search.set_placeholder_text(Some(&tr("Search tags…")));
    search.set_hexpand(true);
    search_and_sort.append(&search);
    let sort_by = gtk::DropDown::from_strings(&[&tr("Name")]);
    sort_by.set_selected(0);
    sort_by.set_tooltip_text(Some(&tr("Sort by")));
    search_and_sort.append(&sort_by);
    let sort_descending = sort_direction_button();
    search_and_sort.append(&sort_descending);
    page.append(&search_and_sort);

    let list = gtk::Box::new(gtk::Orientation::Vertical, 3);
    list.set_margin_end(30);
    let scroll = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .child(&list)
        .build();
    page.append(&scroll);
    TagsPageWidgets {
        page,
        list,
        add,
        search,
        sort_by,
        sort_descending,
    }
}

pub(in crate::app::window) fn build_authors_manager_content() -> AuthorsPageWidgets {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 10);
    page.set_hexpand(true);
    page.set_vexpand(true);
    page.set_margin_top(20);
    page.set_margin_bottom(12);
    page.set_margin_start(22);
    page.set_margin_end(22);
    let header = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let title = gtk::Label::new(Some(&tr("Manage authors")));
    title.set_xalign(0.0);
    title.add_css_class("title-2");
    title.set_hexpand(true);
    header.append(&title);
    let add = labeled_button("list-add-symbolic", &tr("Add author"));
    add.add_css_class("suggested-action");
    header.append(&add);
    page.append(&header);

    let search_and_sort = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let search = gtk::SearchEntry::new();
    search.set_placeholder_text(Some(&tr("Search authors…")));
    search.set_hexpand(true);
    search_and_sort.append(&search);
    let sort_by = gtk::DropDown::from_strings(&[&tr("Full name"), &tr("Institution")]);
    sort_by.set_selected(0);
    sort_by.set_tooltip_text(Some(&tr("Sort by")));
    search_and_sort.append(&sort_by);
    let sort_descending = sort_direction_button();
    search_and_sort.append(&sort_descending);
    page.append(&search_and_sort);

    let list = gtk::Box::new(gtk::Orientation::Vertical, 3);
    list.set_margin_end(30);
    let scroll = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .child(&list)
        .build();
    page.append(&scroll);
    AuthorsPageWidgets {
        page,
        list,
        add,
        search,
        sort_by,
        sort_descending,
    }
}

pub(in crate::app::window) fn reference_filter_dropdown(all_label: &str) -> gtk::DropDown {
    let all_label = tr(all_label);
    gtk::DropDown::from_strings(&[&all_label])
}
