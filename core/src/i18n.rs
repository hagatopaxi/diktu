//! Translations through the C library's gettext, in the `diktu` domain.
//! Messages are extracted by `po/update.sh`; a message with `{name}` placeholders goes
//! through [`trf`], so translators can reorder them.

use std::ffi::{CStr, CString, c_char, c_int};
use std::path::Path;

pub const DOMAIN: &str = "diktu";
/// glibc's value; Diktu only targets GNU/Linux.
const LC_ALL: c_int = 6;

unsafe extern "C" {
    fn setlocale(category: c_int, locale: *const c_char) -> *mut c_char;
    fn bindtextdomain(domain: *const c_char, dir: *const c_char) -> *mut c_char;
    fn bind_textdomain_codeset(domain: *const c_char, codeset: *const c_char) -> *mut c_char;
    fn dgettext(domain: *const c_char, msgid: *const c_char) -> *mut c_char;
}

/// Applies the user's locale and looks for catalogs in `locale_dir`.
pub fn init(locale_dir: &Path) {
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
