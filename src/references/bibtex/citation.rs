use super::{BibEntry, IndexMap, Regex};

mod authors;
mod render;
mod style;

use self::authors::format_authors;
pub use self::authors::split_bibtex_names;
pub use self::render::format_reference_citation;
pub use self::style::{create_citation_key, reference_style_for_document, ReferenceStyle};
