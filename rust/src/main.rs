pub use ovenbird_core::*;

mod app;
mod libpanel;
mod window;

fn main() -> gtk::glib::ExitCode {
    ovenbird_core::i18n::initialize();
    app::run()
}
