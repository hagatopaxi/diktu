//! Translations through the C library's gettext, in the `diktu` domain.
//! Messages are extracted by `po/update.sh`; a message with `{name}` placeholders goes
//! through [`trf`], so translators can reorder them.

use std::ffi::{CStr, CString, c_char, c_int};
use std::path::Path;
use std::sync::OnceLock;

pub const DOMAIN: &str = "diktu";
/// glibc's value; Diktu only targets GNU/Linux.
const LC_ALL: c_int = 6;

unsafe extern "C" {
    fn setlocale(category: c_int, locale: *const c_char) -> *mut c_char;
    fn bindtextdomain(domain: *const c_char, dir: *const c_char) -> *mut c_char;
    fn bind_textdomain_codeset(domain: *const c_char, codeset: *const c_char) -> *mut c_char;
    fn dgettext(domain: *const c_char, msgid: *const c_char) -> *mut c_char;
}

/// `LANGUAGE` as the session set it, restored when the interface follows the desktop again.
static DESKTOP_LANGUAGE: OnceLock<Option<std::ffi::OsString>> = OnceLock::new();

/// Applies the user's locale and looks for catalogs in `locale_dir`.
pub fn init(locale_dir: &Path) {
    DESKTOP_LANGUAGE.get_or_init(|| std::env::var_os("LANGUAGE"));
    let domain = CString::new(DOMAIN).unwrap();
    let Ok(dir) = CString::new(locale_dir.as_os_str().as_encoded_bytes()) else {
        return;
    };
    // SAFETY: NUL-terminated strings that outlive the calls; gettext copies them.
    unsafe {
        setlocale(LC_ALL, c"".as_ptr());
        bindtextdomain(domain.as_ptr(), dir.as_ptr());
        bind_textdomain_codeset(domain.as_ptr(), c"UTF-8".as_ptr());
    }
}

/// Translates later messages, GTK's included, into `code`; empty follows the desktop again.
/// Child processes inherit the choice, as `LANGUAGE` is what gettext reads.
pub fn set_language(code: &str) {
    unsafe extern "C" {
        /// gettext's catalog generation: incrementing it drops translations cached for the
        /// previous `LANGUAGE`.
        static mut _nl_msg_cat_cntr: c_int;
    }
    let desktop = DESKTOP_LANGUAGE.get_or_init(|| std::env::var_os("LANGUAGE"));
    // SAFETY: glibc's setenv never frees the strings it replaces, so a gettext call racing
    // on another thread reads either the old or the new value.
    unsafe {
        match (code, desktop) {
            ("", Some(value)) => std::env::set_var("LANGUAGE", value),
            ("", None) => std::env::remove_var("LANGUAGE"),
            (code, _) => std::env::set_var("LANGUAGE", code),
        }
        _nl_msg_cat_cntr += 1;
    }
}

/// The translation of `msgid`, or `msgid` itself.
pub fn tr(msgid: &str) -> String {
    let (Ok(domain), Ok(id)) = (CString::new(DOMAIN), CString::new(msgid)) else {
        return msgid.to_owned();
    };
    // SAFETY: dgettext returns either `id` or a static string from the catalog.
    unsafe { CStr::from_ptr(dgettext(domain.as_ptr(), id.as_ptr())) }
        .to_string_lossy()
        .into_owned()
}

/// [`tr`], then each `{name}` replaced by its value.
pub fn trf(msgid: &str, args: &[(&str, &dyn std::fmt::Display)]) -> String {
    args.iter().fold(tr(msgid), |s, (name, value)| {
        s.replace(&format!("{{{name}}}"), &value.to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn untranslated_message_keeps_placeholders_filled() {
        let msgid = "{a} of {b}";
        assert_eq!(trf(msgid, &[("a", &1), ("b", &"two")]), "1 of two");
    }
}
