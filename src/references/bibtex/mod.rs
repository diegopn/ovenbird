use indexmap::IndexMap;
use regex::Regex;

mod citation;
mod display;
mod model;
mod parser;
mod schema;

pub use self::citation::{
    create_citation_key, format_reference_citation, reference_style_for_document,
    split_bibtex_names, ReferenceStyle,
};
pub use self::display::{display_bibtex_names, display_bibtex_text};
pub use self::model::{entry_matches_search, BibEntry, BibError, Bibliography};
pub use self::parser::{parse_bibtex, serialize_bibtex};
pub use self::schema::{
    build_reference_fields, fields_for_reference_type, reference_field_label, reference_type_label,
    REFERENCE_TYPES,
};

fn tr(message: &str) -> String {
    crate::i18n::gettext(message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn generated_citation_keys_preserve_js_capitalization() {
        let fields = IndexMap::from([
            ("author".to_owned(), "Lópes, Ana".to_owned()),
            ("year".to_owned(), "2024".to_owned()),
            ("title".to_owned(), "Árvore e Pesquisa".to_owned()),
        ]);

        assert_eq!(
            create_citation_key(&fields, &HashSet::new()),
            "Lopes2024Arvore"
        );
        assert_eq!(
            create_citation_key(&fields, &HashSet::from(["lopes2024arvore".to_owned()])),
            "Lopes2024Arvore2",
        );
    }

    #[test]
    fn reference_search_includes_all_fields_case_insensitively() {
        let mut entry = BibEntry::new("article", "key2024");
        entry.set("title", "A useful paper");
        entry.set("journal", "Journal of Example Research");
        entry.set("doi", "10.1234/DOI-EXAMPLE");

        assert!(entry_matches_search(&entry, "example research"));
        assert!(entry_matches_search(&entry, "10.1234/doi-example"));
        assert!(entry_matches_search(&entry, "  "));
        assert!(!entry_matches_search(&entry, "missing"));
    }
}
