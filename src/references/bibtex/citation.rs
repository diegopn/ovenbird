use super::{tr, BibEntry, IndexMap, Regex};

mod authors;
mod render;
mod style;

pub use self::authors::split_bibtex_names;
use self::authors::{format_author_name, format_authors};
pub use self::render::format_reference_citation;
pub use self::style::{create_citation_key, reference_style_for_document, ReferenceStyle};
