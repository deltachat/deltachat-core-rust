use failure::Fail;
use std::ffi::{CStr, CString};

/// Duplicates a string
///
/// returns an empty string if NULL is given, never returns NULL (exits on errors)
///
/// # Examples
///
/// ```rust,norun
/// use deltachat::dc_tools::{dc_strdup, to_string_lossy};
/// unsafe {
///     let str_a = b"foobar\x00" as *const u8 as *const libc::c_char;
///     let str_a_copy = dc_strdup(str_a);
///     assert_eq!(to_string_lossy(str_a_copy), "foobar");
///     assert_ne!(str_a, str_a_copy);
/// }
/// ```
pub unsafe fn dc_strdup(s: *const libc::c_char) -> *mut libc::c_char {
    let ret: *mut libc::c_char;
    if !s.is_null() {
        ret = libc::strdup(s);
        assert!(!ret.is_null());
    } else {
        ret = libc::calloc(1, 1) as *mut libc::c_char;
        assert!(!ret.is_null());
    }

    ret
}

/// Error type for the [OsStrExt] trait
#[derive(Debug, Fail, PartialEq)]
pub enum CStringError {
    /// The string contains an interior null byte
    #[fail(display = "String contains an interior null byte")]
    InteriorNullByte,
    /// The string is not valid Unicode
    #[fail(display = "String is not valid unicode")]
    NotUnicode,
}

/// Extra convenience methods on [std::ffi::OsStr] to work with `*libc::c_char`.
///
/// The primary function of this trait is to more easily convert
/// [OsStr], [OsString] or [Path] into pointers to C strings.  This always
/// allocates a new string since it is very common for the source
/// string not to have the required terminal null byte.
///
/// It is implemented for `AsRef<std::ffi::OsStr>>` trait, which
/// allows any type which implements this trait to transparently use
/// this.  This is how the conversion for [Path] works.
///
/// [OsStr]: std::ffi::OsStr
/// [OsString]: std::ffi::OsString
/// [Path]: std::path::Path
///
/// # Example
///
/// ```
/// use deltachat::dc_tools::{dc_strdup, OsStrExt};
/// let path = std::path::Path::new("/some/path");
/// let path_c = path.to_c_string().unwrap();
/// unsafe {
///     let mut c_ptr: *mut libc::c_char = dc_strdup(path_c.as_ptr());
/// }
/// ```
pub trait OsStrExt {
    /// Convert a  [std::ffi::OsStr] to an [std::ffi::CString]
    ///
    /// This is useful to convert e.g. a [std::path::Path] to
    /// [*libc::c_char] by using
    /// [Path::as_os_str()](std::path::Path::as_os_str) and
    /// [CStr::as_ptr()](std::ffi::CStr::as_ptr).
    ///
    /// This returns [CString] and not [&CStr] because not all [OsStr]
    /// slices end with a null byte, particularly those coming from
    /// [Path] do not have a null byte and having to handle this as
    /// the caller would defeat the point of this function.
    ///
    /// On Windows this requires that the [OsStr] contains valid
    /// unicode, which should normally be the case for a [Path].
    ///
    /// [CString]: std::ffi::CString
    /// [CStr]: std::ffi::CStr
    /// [OsStr]: std::ffi::OsStr
    /// [Path]: std::path::Path
    ///
    /// # Errors
    ///
    /// Since a C `*char` is terminated by a NULL byte this conversion
    /// will fail, when the [OsStr] has an interior null byte.  The
    /// function will return
    /// `[Err]([CStringError::InteriorNullByte])`.  When converting
    /// from a [Path] it should be safe to
    /// [`.unwrap()`](std::result::Result::unwrap) this anyway since a
    /// [Path] should not contain interior null bytes.
    ///
    /// On windows when the string contains invalid Unicode
    /// `[Err]([CStringError::NotUnicode])` is returned.
    fn to_c_string(&self) -> Result<CString, CStringError>;
}

impl<T: AsRef<std::ffi::OsStr>> OsStrExt for T {
    #[cfg(not(target_os = "windows"))]
    fn to_c_string(&self) -> Result<CString, CStringError> {
        use std::os::unix::ffi::OsStrExt;
        CString::new(self.as_ref().as_bytes()).map_err(|err| match err {
            std::ffi::NulError { .. } => CStringError::InteriorNullByte,
        })
    }

    #[cfg(target_os = "windows")]
    fn to_c_string(&self) -> Result<CString, CStringError> {
        os_str_to_c_string_unicode(&self)
    }
}

// Implementation for os_str_to_c_string on windows.
#[allow(dead_code)]
fn os_str_to_c_string_unicode(
    os_str: &dyn AsRef<std::ffi::OsStr>,
) -> Result<CString, CStringError> {
    match os_str.as_ref().to_str() {
        Some(val) => CString::new(val.as_bytes()).map_err(|err| match err {
            std::ffi::NulError { .. } => CStringError::InteriorNullByte,
        }),
        None => Err(CStringError::NotUnicode),
    }
}

/// Convenience methods/associated functions for working with [CString]
///
/// This is helps transitioning from unsafe code.
pub trait CStringExt {
    /// Create a new [CString], yolo style
    ///
    /// This unwrap the result, panicking when there are embedded NULL
    /// bytes.
    fn yolo<T: Into<Vec<u8>>>(t: T) -> CString {
        CString::new(t).expect("String contains null byte, can not be CString")
    }
}

impl CStringExt for CString {}

/// Convenience methods to make transitioning from raw C strings easier.
///
/// To interact with (legacy) C APIs we often need to convert from
/// Rust strings to raw C strings.  This can be clumsy to do correctly
/// and the compiler sometimes allows it in an unsafe way.  These
/// methods make it more succinct and help you get it right.
pub trait StrExt {
    /// Allocate a new raw C `*char` version of this string.
    ///
    /// This allocates a new raw C string which must be freed using
    /// `free`.  It takes care of some common pitfalls with using
    /// [CString.as_ptr].
    ///
    /// [CString.as_ptr]: std::ffi::CString.as_ptr
    ///
    /// # Panics
    ///
    /// This function will panic when the original string contains an
    /// interior null byte as this can not be represented in raw C
    /// strings.
    unsafe fn strdup(&self) -> *mut libc::c_char;
}

impl<T: AsRef<str>> StrExt for T {
    unsafe fn strdup(&self) -> *mut libc::c_char {
        let tmp = CString::yolo(self.as_ref());
        dc_strdup(tmp.as_ptr())
    }
}

pub fn to_string_lossy(s: *const libc::c_char) -> String {
    if s.is_null() {
        return "".into();
    }

    let cstr = unsafe { CStr::from_ptr(s) };

    cstr.to_string_lossy().to_string()
}

pub fn to_opt_string_lossy(s: *const libc::c_char) -> Option<String> {
    if s.is_null() {
        return None;
    }

    Some(to_string_lossy(s))
}

/// Convert a C `*char` pointer to a [std::path::Path] slice.
///
/// This converts a `*libc::c_char` pointer to a [Path] slice.  This
/// essentially has to convert the pointer to [std::ffi::OsStr] to do
/// so and thus is the inverse of [OsStrExt::to_c_string].  Just like
/// [OsStrExt::to_c_string] requires valid Unicode on Windows, this
/// requires that the pointer contains valid UTF-8 on Windows.
///
/// Because this returns a reference the [Path] silce can not outlive
/// the original pointer.
///
/// [Path]: std::path::Path
#[cfg(not(target_os = "windows"))]
pub fn as_path<'a>(s: *const libc::c_char) -> &'a std::path::Path {
    assert!(!s.is_null(), "cannot be used on null pointers");
    use std::os::unix::ffi::OsStrExt;
    unsafe {
        let c_str = std::ffi::CStr::from_ptr(s).to_bytes();
        let os_str = std::ffi::OsStr::from_bytes(c_str);
        std::path::Path::new(os_str)
    }
}

// as_path() implementation for windows, documented above.
#[cfg(target_os = "windows")]
pub fn as_path<'a>(s: *const libc::c_char) -> &'a std::path::Path {
    as_path_unicode(s)
}

// Implementation for as_path() on Windows.
//
// Having this as a separate function means it can be tested on unix
// too.
#[allow(dead_code)]
fn as_path_unicode<'a>(s: *const libc::c_char) -> &'a std::path::Path {
    assert!(!s.is_null(), "cannot be used on null pointers");

    let cstr = unsafe { CStr::from_ptr(s) };
    let str = cstr.to_str().unwrap_or_else(|err| panic!("{}", err));

    std::path::Path::new(str)
}

#[cfg(test)]
mod tests {
    use super::*;
    use libc::{free, strcmp};

    #[test]
    fn test_os_str_to_c_string_cwd() {
        let some_dir = std::env::current_dir().unwrap();
        some_dir.as_os_str().to_c_string().unwrap();
    }

    #[test]
    fn test_os_str_to_c_string_unicode() {
        let some_str = String::from("/some/valid/utf8");
        let some_dir = std::path::Path::new(&some_str);
        assert_eq!(
            some_dir.as_os_str().to_c_string().unwrap(),
            CString::new("/some/valid/utf8").unwrap()
        );
    }

    #[test]
    fn test_os_str_to_c_string_nul() {
        let some_str = std::ffi::OsString::from("foo\x00bar");
        assert_eq!(
            some_str.to_c_string().err().unwrap(),
            CStringError::InteriorNullByte
        )
    }

    #[test]
    fn test_path_to_c_string_cwd() {
        let some_dir = std::env::current_dir().unwrap();
        some_dir.to_c_string().unwrap();
    }

    #[test]
    fn test_path_to_c_string_unicode() {
        let some_str = String::from("/some/valid/utf8");
        let some_dir = std::path::Path::new(&some_str);
        assert_eq!(
            some_dir.as_os_str().to_c_string().unwrap(),
            CString::new("/some/valid/utf8").unwrap()
        );
    }

    #[test]
    fn test_os_str_to_c_string_unicode_fn() {
        let some_str = std::ffi::OsString::from("foo");
        assert_eq!(
            os_str_to_c_string_unicode(&some_str).unwrap(),
            CString::new("foo").unwrap()
        );
    }

    #[test]
    fn test_path_to_c_string_unicode_fn() {
        let some_str = String::from("/some/path");
        let some_path = std::path::Path::new(&some_str);
        assert_eq!(
            os_str_to_c_string_unicode(&some_path).unwrap(),
            CString::new("/some/path").unwrap()
        );
    }

    #[test]
    fn test_os_str_to_c_string_unicode_fn_nul() {
        let some_str = std::ffi::OsString::from("fooz\x00bar");
        assert_eq!(
            os_str_to_c_string_unicode(&some_str).err().unwrap(),
            CStringError::InteriorNullByte
        );
    }

    #[test]
    fn test_as_path() {
        let some_path = CString::new("/some/path").unwrap();
        let ptr = some_path.as_ptr();
        assert_eq!(as_path(ptr), std::ffi::OsString::from("/some/path"))
    }

    #[test]
    fn test_as_path_unicode_fn() {
        let some_path = CString::new("/some/path").unwrap();
        let ptr = some_path.as_ptr();
        assert_eq!(as_path_unicode(ptr), std::ffi::OsString::from("/some/path"));
    }

    #[test]
    fn test_cstring_yolo() {
        assert_eq!(CString::new("hello").unwrap(), CString::yolo("hello"));
    }

    #[test]
    fn test_strdup_str() {
        unsafe {
            let s = "hello".strdup();
            let cmp = strcmp(s, b"hello\x00" as *const u8 as *const libc::c_char);
            free(s as *mut libc::c_void);
            assert_eq!(cmp, 0);
        }
    }

    #[test]
    fn test_strdup_string() {
        unsafe {
            let s = String::from("hello").strdup();
            let cmp = strcmp(s, b"hello\x00" as *const u8 as *const libc::c_char);
            free(s as *mut libc::c_void);
            assert_eq!(cmp, 0);
        }
    }
}
