use super::*;

pub(super) fn connect_reference_form_response(
    dialog: &gtk::Dialog,
    state: Rc<State>,
    previous: Option<BibEntry>,
    form: ReferenceForm,
) {
    dialog.connect_response(move |dialog, response| {
        if response == gtk::ResponseType::Reject {
            if let Some(entry) = previous.as_ref() {
                let parent: &gtk::Window = dialog.upcast_ref();
                let edit_dialog = dialog.clone();
                request_reference_deletion(&state, parent, entry.clone(), move || {
                    edit_dialog.close()
                });
            } else {
                dialog.close();
            }
            return;
        }
        if response == gtk::ResponseType::Accept {
            let kind = REFERENCE_TYPES
                .get(form.picker.selected() as usize)
                .copied()
                .unwrap_or("article");
            let mut values = form
                .rows
                .borrow()
                .iter()
                .map(|(name, (_, entry))| (name.clone(), entry.text().trim().to_owned()))
                .collect::<indexmap::IndexMap<_, _>>();
            values.insert(
                "author".to_owned(),
                form.selected_authors.borrow().join(" and "),
            );
            values.insert("tags".to_owned(), form.selected_tags.borrow().join(", "));
            let old_fields = previous
                .as_ref()
                .map(|entry| &entry.fields)
                .cloned()
                .unwrap_or_default();
            let fields = bibtex::build_reference_fields(kind, &values, &old_fields);
            if fields.get("title").is_none_or(String::is_empty) {
                show_toast(&state, &tr("Enter the reference title"));
                return;
            }
            let used = state
                .library
                .borrow()
                .bibliography
                .entries
                .iter()
                .filter(|item| previous.as_ref().is_none_or(|old| old.key != item.key))
                .map(|item| item.key.to_ascii_lowercase())
                .collect::<HashSet<_>>();
            let key = if form.key_entry.text().trim().is_empty() {
                bibtex::create_citation_key(&fields, &used)
            } else {
                form.key_entry.text().trim().to_owned()
            };
            let updated = BibEntry {
                entry_type: kind.to_owned(),
                key,
                fields,
                raw_fields: indexmap::IndexMap::new(),
            };
            let mut library = state.library.borrow().clone();
            let result = if let Some(previous) = previous.as_ref() {
                library.update_in_memory(&previous.key, updated)
            } else {
                library.add_in_memory(updated)
            };
            match result {
                Ok(()) => {
                    persist_library_async(state.clone(), library, tr("Reference saved"));
                    dialog.close();
                }
                Err(error) => show_toast(&state, &error),
            }
            return;
        }
        dialog.close();
    });
}
