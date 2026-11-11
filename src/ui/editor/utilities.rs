use super::*;

pub(super) fn character_count(text: &str) -> usize {
    text.chars().count()
}

pub(super) fn buffer_text(buffer: &impl IsA<TextBuffer>) -> String {
    let buffer = buffer.as_ref();
    buffer
        .text(&buffer.start_iter(), &buffer.end_iter(), true)
        .to_string()
}

pub(super) fn source_style_scheme_id(is_dark: bool) -> &'static str {
    if is_dark {
        "Adwaita-dark"
    } else {
        "Adwaita"
    }
}

pub(super) fn apply_source_style_scheme(buffer: &sourceview5::Buffer, is_dark: bool) {
    let manager = sourceview5::StyleSchemeManager::default();
    buffer.set_style_scheme(manager.scheme(source_style_scheme_id(is_dark)).as_ref());
}
