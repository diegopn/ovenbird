use super::*;

mod author;
mod response;
mod tag;

use self::author::build_reference_author_editor;
use self::response::connect_reference_form_response;
use self::tag::build_reference_tag_editor;

pub(in crate::app::window) fn edit_reference(state: &Rc<State>, entry: Option<BibEntry>) {
    if !library_is_ready(state) {
        return;
    }
    let dialog = gtk::Dialog::builder()
        .title(tr_dynamic(if entry.is_some() {
            "Edit reference"
        } else {
            "New reference"
        }))
        .transient_for(&state.window)
        .modal(true)
        .build();
    dialog.set_default_size(700, 660);
    let grid = gtk::Grid::builder()
        .column_spacing(12)
        .row_spacing(8)
        .margin_top(18)
        .margin_bottom(18)
        .margin_start(20)
        .margin_end(30)
        .build();
    let form_scroll = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&grid)
        .build();
    let content = dialog.content_area();
    content.append(&form_scroll);
    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    actions.set_margin_top(16);
    actions.set_margin_bottom(0);
    actions.set_margin_start(6);
    actions.set_margin_end(6);
    if entry.is_some() {
        let delete = gtk::Button::with_label(&tr("Delete"));
        delete.add_css_class("destructive-action");
        let dialog_for_delete = dialog.clone();
        delete.connect_clicked(move |_| dialog_for_delete.response(gtk::ResponseType::Reject));
        actions.append(&delete);
    }
    let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    actions.append(&spacer);
    let cancel = gtk::Button::with_label(&tr("Cancel"));
    let dialog_for_cancel = dialog.clone();
    cancel.connect_clicked(move |_| dialog_for_cancel.response(gtk::ResponseType::Cancel));
    actions.append(&cancel);
    let save_label = tr_dynamic(if entry.is_some() { "Save" } else { "Add" });
    let save = gtk::Button::with_label(&save_label);
    save.add_css_class("suggested-action");
    let dialog_for_save = dialog.clone();
    save.connect_clicked(move |_| dialog_for_save.response(gtk::ResponseType::Accept));
    actions.append(&save);
    dialog.set_default_widget(Some(&save));
    content.append(&actions);
    let translated_types = REFERENCE_TYPES
        .iter()
        .map(|kind| bibtex::reference_type_label(kind))
        .collect::<Vec<_>>();
    let type_choices = translated_types
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let type_model = gtk::StringList::new(&type_choices);
    let picker = gtk::DropDown::new(Some(type_model), gtk::Expression::NONE);
    let selected_type = entry
        .as_ref()
        .map(|entry| entry.entry_type.as_str())
        .unwrap_or("article");
    picker.set_selected(
        REFERENCE_TYPES
            .iter()
            .position(|kind| *kind == selected_type)
            .unwrap_or(0) as u32,
    );
    grid.attach(&gtk::Label::new(Some(&tr("Reference type"))), 0, 0, 1, 1);
    grid.attach(&picker, 1, 0, 1, 1);
    let field_names = REFERENCE_TYPES
        .iter()
        .flat_map(|kind| {
            crate::bibtex::fields_for_reference_type(kind)
                .iter()
                .copied()
        })
        .collect::<HashSet<_>>();
    let mut names = field_names.iter().copied().collect::<Vec<_>>();
    names.sort();
    let key_entry = gtk::Entry::new();
    if let Some(entry) = entry.as_ref() {
        key_entry.set_text(&entry.key);
    }
    let rows = Rc::new(RefCell::new(
        HashMap::<String, (gtk::Label, gtk::Entry)>::new(),
    ));
    for name in names {
        if name == "tags" || name == "author" {
            continue;
        }
        let label = gtk::Label::new(Some(&bibtex::reference_field_label(name, selected_type)));
        label.set_xalign(1.0);
        label.add_css_class("dim-label");
        let input = gtk::Entry::new();
        input.set_hexpand(true);
        if name == "title" {
            input.set_placeholder_text(Some(&tr("Full title of the work")));
        }
        if let Some(entry) = entry.as_ref() {
            let value = if name == "date" && entry.get("date").is_empty() {
                entry.get("year")
            } else {
                entry.get(name)
            };
            input.set_text(value);
        }
        rows.borrow_mut().insert(name.to_owned(), (label, input));
    }

    let author_widgets = build_reference_author_editor(state, &entry);
    let tag_widgets = build_reference_tag_editor(state, &entry);

    let key_label = gtk::Label::new(Some(&tr("Citation key")));
    key_label.set_xalign(1.0);
    key_label.add_css_class("dim-label");
    grid.attach(&key_label, 0, 1, 1, 1);
    grid.attach(&key_entry, 1, 1, 1, 1);
    let update_rows = {
        let rows = rows.clone();
        let grid = grid.clone();
        let author_label = author_widgets.label.clone();
        let author_editor = author_widgets.editor.clone();
        let tag_label = tag_widgets.label.clone();
        let tag_editor = tag_widgets.editor.clone();
        move |picker: &gtk::DropDown| {
            for (label, input) in rows.borrow().values() {
                if label.parent().is_some() {
                    grid.remove(label);
                }
                if input.parent().is_some() {
                    grid.remove(input);
                }
            }
            let kind = REFERENCE_TYPES
                .get(picker.selected() as usize)
                .copied()
                .unwrap_or("article");
            let mut row_num = 2;
            if let Some((label, input)) = rows.borrow().get("title") {
                label.set_label(&bibtex::reference_field_label("title", kind));
                grid.attach(label, 0, row_num, 1, 1);
                grid.attach(input, 1, row_num, 1, 1);
                row_num += 1;
            }
            grid.attach(&author_label, 0, row_num, 1, 1);
            grid.attach(&author_editor, 1, row_num, 1, 1);
            row_num += 1;
            for name in crate::bibtex::fields_for_reference_type(kind) {
                if *name == "tags" || *name == "title" || *name == "author" {
                    continue;
                }
                if let Some((label, input)) = rows.borrow().get(*name) {
                    label.set_label(&bibtex::reference_field_label(name, kind));
                    grid.attach(label, 0, row_num, 1, 1);
                    grid.attach(input, 1, row_num, 1, 1);
                    row_num += 1;
                }
            }
            tag_label.set_label(&bibtex::reference_field_label("tags", kind));
            grid.attach(&tag_label, 0, row_num, 1, 1);
            grid.attach(&tag_editor, 1, row_num, 1, 1);
        }
    };
    update_rows(&picker);
    let update_rows_notify = update_rows;
    picker.connect_selected_notify(move |picker| update_rows_notify(picker));
    let form = ReferenceForm {
        picker: picker.clone(),
        key_entry: key_entry.clone(),
        rows: rows.clone(),
        selected_authors: author_widgets.selected.clone(),
        selected_tags: tag_widgets.selected.clone(),
    };
    connect_reference_form_response(&dialog, state.clone(), entry.clone(), form);
    dialog.present();
}

struct ReferenceAuthorWidgets {
    label: gtk::Label,
    editor: gtk::Box,
    selected: Rc<RefCell<Vec<String>>>,
}

struct ReferenceTagWidgets {
    label: gtk::Label,
    editor: gtk::Box,
    selected: Rc<RefCell<Vec<String>>>,
}

struct ReferenceForm {
    picker: gtk::DropDown,
    key_entry: gtk::Entry,
    rows: Rc<RefCell<HashMap<String, (gtk::Label, gtk::Entry)>>>,
    selected_authors: Rc<RefCell<Vec<String>>>,
    selected_tags: Rc<RefCell<Vec<String>>>,
}
