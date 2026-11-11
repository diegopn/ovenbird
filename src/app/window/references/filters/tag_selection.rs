use super::*;

pub(in crate::app::window) fn parse_tag_values(value: &str) -> Vec<String> {
    let mut tags = Vec::new();
    for tag in value
        .split([',', ';'])
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
    {
        if !tags
            .iter()
            .any(|existing: &String| existing.eq_ignore_ascii_case(tag))
        {
            tags.push(tag.to_owned());
        }
    }
    tags
}

pub(in crate::app::window) fn add_tag_selection(
    name: &str,
    available: &Rc<RefCell<Vec<String>>>,
    selected: &Rc<RefCell<Vec<String>>>,
) -> bool {
    let name = name.trim();
    if name.is_empty() {
        return false;
    }
    let existing = available
        .borrow()
        .iter()
        .find(|tag| tag.eq_ignore_ascii_case(name))
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
        .any(|tag| tag.eq_ignore_ascii_case(&canonical))
    {
        return true;
    }
    selected.push(canonical);
    true
}

pub(in crate::app::window) fn refresh_reference_tag_chips(
    chips: &gtk::FlowBox,
    selected: &Rc<RefCell<Vec<String>>>,
) {
    while let Some(child) = chips.first_child() {
        chips.remove(&child);
    }
    for tag in selected.borrow().iter() {
        let chip = gtk::Button::with_label(&format!("{tag}  ×"));
        chip.add_css_class("pill");
        chip.add_css_class("reference-tag-chip");
        chip.set_tooltip_text(Some(&tr("Remove tag from this reference")));
        let selected = selected.clone();
        let chips_for_click = chips.clone();
        let remove = tag.clone();
        chip.connect_clicked(move |_| {
            selected
                .borrow_mut()
                .retain(|tag| !tag.eq_ignore_ascii_case(&remove));
            refresh_reference_tag_chips(&chips_for_click, &selected);
        });
        chips.append(&chip);
    }
}

pub(in crate::app::window) fn refresh_reference_tag_picker(
    list: &gtk::Box,
    chips: &gtk::FlowBox,
    selected: &Rc<RefCell<Vec<String>>>,
    available: &Rc<RefCell<Vec<String>>>,
    query: &str,
) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
    let query = query.trim().to_lowercase();
    let choices = available
        .borrow()
        .iter()
        .filter(|tag| query.is_empty() || tag.to_lowercase().contains(&query))
        .cloned()
        .collect::<Vec<_>>();
    if choices.is_empty() {
        let empty = gtk::Label::new(Some(&tr("No matching tags")));
        empty.set_xalign(0.0);
        empty.add_css_class("dim-label");
        list.append(&empty);
        return;
    }
    for tag in choices {
        let choice = gtk::CheckButton::with_label(&tag);
        choice.set_active(
            selected
                .borrow()
                .iter()
                .any(|selected| selected.eq_ignore_ascii_case(&tag)),
        );
        let selected = selected.clone();
        let chips = chips.clone();
        let tag_for_toggle = tag.clone();
        choice.connect_toggled(move |choice| {
            let mut selected_tags = selected.borrow_mut();
            if choice.is_active() {
                if !selected_tags
                    .iter()
                    .any(|selected| selected.eq_ignore_ascii_case(&tag_for_toggle))
                {
                    selected_tags.push(tag_for_toggle.clone());
                }
            } else {
                selected_tags.retain(|selected| !selected.eq_ignore_ascii_case(&tag_for_toggle));
            }
            drop(selected_tags);
            refresh_reference_tag_chips(&chips, &selected);
        });
        list.append(&choice);
    }
}
