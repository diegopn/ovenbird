pub use ovenbird_core::*;

mod app;

fn main() -> gtk::glib::ExitCode {
    ovenbird_core::i18n::initialize();
    app::run()
}
