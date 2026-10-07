use std::ffi::{c_char, CStr, CString};
use std::path::PathBuf;
use std::sync::OnceLock;

const DOMAIN: &str = "ovenbird";
const LC_ALL: i32 = 6;
static INITIALIZED: OnceLock<()> = OnceLock::new();

unsafe extern "C" {
    fn setlocale(category: i32, locale: *const c_char) -> *mut c_char;
    fn bindtextdomain(domainname: *const c_char, dirname: *const c_char) -> *mut c_char;
    fn bind_textdomain_codeset(domainname: *const c_char, codeset: *const c_char) -> *mut c_char;
    fn textdomain(domainname: *const c_char) -> *mut c_char;
    #[link_name = "gettext"]
    fn c_gettext(msgid: *const c_char) -> *mut c_char;
    #[link_name = "ngettext"]
    fn c_ngettext(singular: *const c_char, plural: *const c_char, count: usize) -> *mut c_char;
}

fn generic_locale(value: &str) -> bool {
    let value = value.to_ascii_uppercase();
    value == "C" || value == "POSIX" || value.starts_with("C.") || value.starts_with("C_")
}

fn preferred_messages_locale<'a>(
    lang: &'a str,
    messages: &'a str,
    language: &'a str,
) -> Option<&'a str> {
    if !messages.is_empty() && !generic_locale(messages) {
        Some(messages)
    } else if !lang.is_empty() && !generic_locale(lang) {
        Some(lang)
    } else {
        language
            .split(':')
            .find(|locale| !locale.is_empty() && !generic_locale(locale))
    }
}

fn locale_directory() -> PathBuf {
    if let Some(directory) = std::env::var_os("OVENBIRD_LOCALEDIR") {
        return PathBuf::from(directory);
    }
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(PathBuf::from))
        .and_then(|bin| bin.parent().map(|prefix| prefix.join("share/locale")))
        .unwrap_or_else(|| PathBuf::from("/app/share/locale"))
}

pub fn initialize() {
    INITIALIZED.get_or_init(|| {
        let lc_all = std::env::var("LC_ALL").unwrap_or_default();
        let lang = std::env::var("LANG").unwrap_or_default();
        let mut messages_locale = std::env::var("LC_MESSAGES").unwrap_or_default();
        let language = std::env::var("LANGUAGE").unwrap_or_default();
        let preferred = preferred_messages_locale(&lang, &messages_locale, &language);
        if generic_locale(&lc_all) && preferred.is_some() {
            std::env::remove_var("LC_ALL");
        }
        if let Some(preferred) = preferred {
            if generic_locale(&messages_locale) {
                messages_locale = preferred.to_owned();
            }
            std::env::set_var("LC_MESSAGES", messages_locale);
        }

        let domain = CString::new(DOMAIN).expect("gettext domain is a static string");
        let path = CString::new(locale_directory().to_string_lossy().as_bytes())
            .expect("locale path must not contain NUL");
        let utf8 = CString::new("UTF-8").expect("UTF-8 has no NUL");
        unsafe {
            setlocale(LC_ALL, c"".as_ptr());
            bindtextdomain(domain.as_ptr(), path.as_ptr());
            bind_textdomain_codeset(domain.as_ptr(), utf8.as_ptr());
            textdomain(domain.as_ptr());
        }
    });
}

pub fn gettext(message: &str) -> String {
    initialize();
    let input = CString::new(message).unwrap_or_else(|_| CString::new("").unwrap());
    let translated = unsafe { c_gettext(input.as_ptr()) };
    if translated.is_null() {
        return message.to_owned();
    }
    unsafe { CStr::from_ptr(translated).to_string_lossy().into_owned() }
}

pub fn ngettext(singular: &str, plural: &str, count: usize) -> String {
    initialize();
    let singular_c = CString::new(singular).unwrap_or_else(|_| CString::new("").unwrap());
    let plural_c = CString::new(plural).unwrap_or_else(|_| CString::new("").unwrap());
    let translated = unsafe { c_ngettext(singular_c.as_ptr(), plural_c.as_ptr(), count) };
    if translated.is_null() {
        return if count == 1 { singular } else { plural }.to_owned();
    }
    unsafe { CStr::from_ptr(translated).to_string_lossy().into_owned() }
}

#[cfg(test)]
mod tests {
    use super::preferred_messages_locale;

    #[test]
    fn generic_message_locale_falls_back_to_system_language() {
        assert_eq!(
            preferred_messages_locale("pt_BR.UTF-8", "C.UTF-8", ""),
            Some("pt_BR.UTF-8")
        );
        assert_eq!(
            preferred_messages_locale("C.UTF-8", "es_ES.UTF-8", "pt_BR"),
            Some("es_ES.UTF-8")
        );
        assert_eq!(
            preferred_messages_locale("C.UTF-8", "C.UTF-8", "pt_BR:pt"),
            Some("pt_BR")
        );
        assert_eq!(preferred_messages_locale("C.UTF-8", "C.UTF-8", ""), None);
    }
}
