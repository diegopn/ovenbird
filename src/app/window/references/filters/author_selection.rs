use super::*;

pub(in crate::app::window) fn author_selection_identity(name: &str) -> String {
    bibtex::display_bibtex_text(name)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

pub(in crate::app::window) fn add_author_selection(
    name: &str,
    available: &Rc<RefCell<Vec<String>>>,
    selected: &Rc<RefCell<Vec<String>>>,
) -> bool {
    let name = name.trim();
    if name.is_empty() {
        return false;
    }
    let identity = author_selection_identity(name);
    let existing = available
        .borrow()
        .iter()
        .find(|author| author_selection_identity(author) == identity)
        .cloned();
    let canonical = if let Some(existing) = existing {
        existing
    } else {
        let name = name.to_owned();
        available.borrow_mut().push(name.clone());
        name
    };
    let mut selected = selected.borrow_mut();
    if selected
        .iter()
        .any(|author| author_selection_identity(author) == identity)
    {
        return true;
    }
    selected.push(canonical);
    true
}

pub(in crate::app::window) fn refresh_reference_author_list(
    selected_list: &gtk::Box,
    selected: &Rc<RefCell<Vec<String>>>,
) {
    while let Some(child) = selected_list.first_child() {
        selected_list.remove(&child);
    }
    for (index, author) in selected.borrow().iter().cloned().enumerate() {
        let display_name = bibtex::display_bibtex_text(&author);
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        row.add_css_class("card");
        row.add_css_class("reference-author-row");
        row.set_hexpand(true);
        row.set_margin_start(2);
        row.set_margin_end(2);
        let name = gtk::Label::new(Some(&display_name));
        name.set_xalign(0.0);
        name.set_hexpand(true);
        name.set_ellipsize(gtk::pango::EllipsizeMode::End);
        row.append(&name);

        let move_up = button("go-up-symbolic", &tr("Move author up"));
        move_up.set_sensitive(index > 0);
        let selected_for_up = selected.clone();
        let list_for_up = selected_list.clone();
        move_up.connect_clicked(move |_| {
            {
                let mut authors = selected_for_up.borrow_mut();
                if index > 0 && index < authors.len() {
                    authors.swap(index, index - 1);
                }
            }
            refresh_reference_author_list(&list_for_up, &selected_for_up);
        });
        row.append(&move_up);

        let move_down = button("go-down-symbolic", &tr("Move author down"));
        move_down.set_sensitive(index + 1 < selected.borrow().len());
        let selected_for_down = selected.clone();
        let list_for_down = selected_list.clone();
        move_down.connect_clicked(move |_| {
            {
                let mut authors = selected_for_down.borrow_mut();
                if index + 1 < authors.len() {
                    authors.swap(index, index + 1);
                }
            }
            refresh_reference_author_list(&list_for_down, &selected_for_down);
        });
        row.append(&move_down);

        let remove = button(
            "window-close-symbolic",
            &tr("Remove author from this reference"),
        );
        let selected_for_remove = selected.clone();
        let list_for_remove = selected_list.clone();
        let remove_identity = author_selection_identity(&author);
        remove.connect_clicked(move |_| {
            selected_for_remove
                .borrow_mut()
                .retain(|author| author_selection_identity(author) != remove_identity);
            refresh_reference_author_list(&list_for_remove, &selected_for_remove);
        });
        row.append(&remove);
        selected_list.append(&row);
    }
}

pub(in crate::app::window) fn refresh_reference_author_picker(
    list: &gtk::Box,
    selected_list: &gtk::Box,
    selected: &Rc<RefCell<Vec<String>>>,
    available: &Rc<RefCell<Vec<String>>>,
    query: &str,
) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
    let query = query.trim().to_lowercase();
    let mut choices = available
        .borrow()
        .iter()
        .filter(|author| {
            query.is_empty()
                || bibtex::display_bibtex_text(author)
                    .to_lowercase()
                    .contains(&query)
        })
        .cloned()
        .collect::<Vec<_>>();
    choices.sort_by_key(|author| bibtex::display_bibtex_text(author).to_lowercase());
    if choices.is_empty() {
        let empty = gtk::Label::new(Some(&tr("No matching authors")));
        empty.set_xalign(0.0);
        empty.add_css_class("dim-label");
        list.append(&empty);
        return;
    }
    for author in choices {
        let choice = gtk::CheckButton::with_label(&bibtex::display_bibtex_text(&author));
        let identity = author_selection_identity(&author);
        choice.set_active(
            selected
                .borrow()
                .iter()
                .any(|selected| author_selection_identity(selected) == identity),
        );
        let selected = selected.clone();
        let selected_list = selected_list.clone();
        let author_for_toggle = author.clone();
        choice.connect_toggled(move |choice| {
            let identity = author_selection_identity(&author_for_toggle);
            let mut selected_authors = selected.borrow_mut();
            if choice.is_active() {
                if !selected_authors
                    .iter()
                    .any(|selected| author_selection_identity(selected) == identity)
                {
                    selected_authors.push(author_for_toggle.clone());
                }
            } else {
                selected_authors.retain(|selected| author_selection_identity(selected) != identity);
            }
            drop(selected_authors);
            refresh_reference_author_list(&selected_list, &selected);
        });
        list.append(&choice);
    }
}
