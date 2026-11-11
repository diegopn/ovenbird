#[path = "app/application_menu.rs"]
pub mod application_menu;
pub mod commands;
pub mod history;
pub mod i18n;
pub mod latex;
pub mod project;
pub mod references;
pub mod search;
#[path = "app/settings.rs"]
pub mod settings;
pub mod storage;
pub mod ui;

pub use latex::build;
pub use latex::diagnostics as latex_diagnostics;
pub use latex::templates;
pub use references::bibtex;
pub use references::exchange as reference_exchange;
pub use references::guides as reference_guides;
pub use ui::editor;
pub use ui::pdf_preview;

#[cfg(test)]
mod tests;
