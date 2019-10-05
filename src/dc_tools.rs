//! Some tools and enhancements to the used libraries, there should be
//! no references to Context and other "larger" entities here.

use core::cmp::max;
use std::borrow::Cow;
use std::ffi::{CStr, CString};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::SystemTime;
use std::{fmt, fs, ptr};

use chrono::{Local, TimeZone};
use libc::{memcpy, strlen};
use mmime::clist::*;
use mmime::mailimf::types::*;
use rand::{thread_rng, Rng};

use crate::context::Context;
use crate::error::Error;
use crate::events::Event;

pub(crate) fn dc_exactly_one_bit_set(v: libc::c_int) -> bool {
    0 != v && 0 == v & (v - 1)
}

/// Duplicates a string
///
/// returns an empty string if NULL is given, never returns NULL (exits on errors)
///
/// # Examples
///
/// ```
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
        ret = strdup(s);
        assert!(!ret.is_null());
    } else {
        ret = libc::calloc(1, 1) as *mut libc::c_char;
        assert!(!ret.is_null());
    }

    ret
}

pub(crate) fn dc_atoi_null_is_0(s: *const libc::c_char) -> libc::c_int {
    if !s.is_null() {
        as_str(s).parse().unwrap_or_default()
    } else {
        0
    }
}

unsafe fn dc_ltrim(buf: *mut libc::c_char) {
    let mut len: libc::size_t;
    let mut cur: *const libc::c_uchar;
    if !buf.is_null() && 0 != *buf as libc::c_int {
        len = strlen(buf);
        cur = buf as *const libc::c_uchar;
        while 0 != *cur as libc::c_int && 0 != libc::isspace(*cur as libc::c_int) {
            cur = cur.offset(1isize);
            len = len.wrapping_sub(1)
        }
        if buf as *const libc::c_uchar != cur {
            libc::memmove(
                buf as *mut libc::c_void,
                cur as *const libc::c_void,
                len.wrapping_add(1),
            );
        }
    };
}

unsafe fn dc_rtrim(buf: *mut libc::c_char) {
    let mut len: libc::size_t;
    let mut cur: *mut libc::c_uchar;
    if !buf.is_null() && 0 != *buf as libc::c_int {
        len = strlen(buf);
        cur = (buf as *mut libc::c_uchar)
            .offset(len as isize)
            .offset(-1isize);
        while cur != buf as *mut libc::c_uchar && 0 != libc::isspace(*cur as libc::c_int) {
            cur = cur.offset(-1isize);
            len = len.wrapping_sub(1)
        }
        *cur.offset(
            (if 0 != libc::isspace(*cur as libc::c_int) {
                0
            } else {
                1
            }) as isize,
        ) = '\u{0}' as i32 as libc::c_uchar
    };
}

pub(crate) unsafe fn dc_trim(buf: *mut libc::c_char) {
    dc_ltrim(buf);
    dc_rtrim(buf);
}

/* remove all \r characters from string */
pub(crate) unsafe fn dc_remove_cr_chars(buf: *mut libc::c_char) {
    /* search for first `\r` */
    let mut p1: *const libc::c_char = buf;
    while 0 != *p1 {
        if *p1 as libc::c_int == '\r' as i32 {
            break;
        }
        p1 = p1.offset(1isize)
    }
    /* p1 is `\r` or null-byte; start removing `\r` */
    let mut p2: *mut libc::c_char = p1 as *mut libc::c_char;
    while 0 != *p1 {
        if *p1 as libc::c_int != '\r' as i32 {
            *p2 = *p1;
            p2 = p2.offset(1isize)
        }
        p1 = p1.offset(1isize)
    }
    *p2 = 0 as libc::c_char;
}

/// Shortens a string to a specified length and adds "..." or "[...]" to the end of
/// the shortened string.
pub(crate) fn dc_truncate(buf: &str, approx_chars: usize, do_unwrap: bool) -> Cow<str> {
    let ellipse = if do_unwrap { "..." } else { "[...]" };

    let count = buf.chars().count();
    if approx_chars > 0 && count > approx_chars + ellipse.len() {
        let end_pos = buf
            .char_indices()
            .nth(approx_chars)
            .map(|(n, _)| n)
            .unwrap_or_default();

        if let Some(index) = buf[..end_pos].rfind(|c| c == ' ' || c == '\n') {
            Cow::Owned(format!("{}{}", &buf[..=index], ellipse))
        } else {
            Cow::Owned(format!("{}{}", &buf[..end_pos], ellipse))
        }
    } else {
        Cow::Borrowed(buf)
    }
}

pub(crate) unsafe fn dc_str_from_clist(
    list: *const clist,
    delimiter: *const libc::c_char,
) -> *mut libc::c_char {
    let mut res = String::new();

    if !list.is_null() {
        let mut cur: *mut clistiter = (*list).first;
        while !cur.is_null() {
            let rfc724_mid = (if !cur.is_null() {
                (*cur).data
            } else {
                ptr::null_mut()
            }) as *const libc::c_char;

            if !rfc724_mid.is_null() {
                if !res.is_empty() && !delimiter.is_null() {
                    res += as_str(delimiter);
                }
                res += as_str(rfc724_mid);
            }
            cur = if !cur.is_null() {
                (*cur).next
            } else {
                ptr::null_mut()
            }
        }
    }

    res.strdup()
}

pub(crate) fn dc_str_to_clist(str: &str, delimiter: &str) -> *mut clist {
    unsafe {
        let list: *mut clist = clist_new();
        for cur in str.split(&delimiter) {
            clist_insert_after(list, (*list).last, cur.strdup().cast());
        }
        list
    }
}

/* the colors must fulfill some criterions as:
- contrast to black and to white
- work as a text-color
- being noticeable on a typical map
- harmonize together while being different enough
(therefore, we cannot just use random rgb colors :) */
const COLORS: [u32; 16] = [
    0xe56555, 0xf28c48, 0x8e85ee, 0x76c84d, 0x5bb6cc, 0x549cdd, 0xd25c99, 0xb37800, 0xf23030,
    0x39b249, 0xbb243b, 0x964078, 0x66874f, 0x308ab9, 0x127ed0, 0xbe450c,
];

pub(crate) fn dc_str_to_color(s: impl AsRef<str>) -> u32 {
    let str_lower = s.as_ref().to_lowercase();
    let mut checksum = 0;
    let bytes = str_lower.as_bytes();
    for (i, byte) in bytes.iter().enumerate() {
        checksum += (i + 1) * *byte as usize;
        checksum %= 0xffffff;
    }
    let color_index = checksum % COLORS.len();

    COLORS[color_index]
}

/* date/time tools */
/* the result is UTC or DC_INVALID_TIMESTAMP */
pub(crate) fn dc_timestamp_from_date(date_time: *mut mailimf_date_time) -> i64 {
    assert!(!date_time.is_null());
    let dt = unsafe { *date_time };

    let sec = dt.dt_sec;
    let min = dt.dt_min;
    let hour = dt.dt_hour;
    let day = dt.dt_day;
    let month = dt.dt_month;
    let year = dt.dt_year;

    let ts = chrono::NaiveDateTime::new(
        chrono::NaiveDate::from_ymd(year, month as u32, day as u32),
        chrono::NaiveTime::from_hms(hour as u32, min as u32, sec as u32),
    );

    let (zone_hour, zone_min) = if dt.dt_zone >= 0 {
        (dt.dt_zone / 100, dt.dt_zone % 100)
    } else {
        (-(-dt.dt_zone / 100), -(-dt.dt_zone % 100))
    };

    ts.timestamp() - (zone_hour * 3600 + zone_min * 60) as i64
}

/* ******************************************************************************
 * date/time tools
 ******************************************************************************/

pub fn dc_timestamp_to_str(wanted: i64) -> String {
    let ts = chrono::Utc.timestamp(wanted, 0);
    ts.format("%Y.%m.%d %H:%M:%S").to_string()
}

pub(crate) fn dc_gm2local_offset() -> i64 {
    let lt = Local::now();
    ((lt.offset().local_minus_utc() / (60 * 60)) * 100) as i64
}

/* timesmearing */
pub(crate) fn dc_smeared_time(context: &Context) -> i64 {
    /* function returns a corrected time(NULL) */
    let mut now = time();
    let ts = *context.last_smeared_timestamp.clone().read().unwrap();
    if ts >= now {
        now = ts + 1;
    }

    now
}

pub(crate) fn dc_create_smeared_timestamp(context: &Context) -> i64 {
    let now = time();
    let mut ret = now;

    let ts = *context.last_smeared_timestamp.clone().write().unwrap();
    if ret <= ts {
        ret = ts + 1;
        if ret - now > 5 {
            ret = now + 5
        }
    }

    ret
}

pub(crate) fn dc_create_smeared_timestamps(context: &Context, count: usize) -> i64 {
    /* get a range to timestamps that can be used uniquely */
    let now = time();
    let start = now + (if count < 5 { count } else { 5 }) as i64 - count as i64;

    let ts = *context.last_smeared_timestamp.clone().write().unwrap();
    if ts + 1 > start {
        ts + 1
    } else {
        start
    }
}

/* Message-ID tools */
pub(crate) fn dc_create_id() -> String {
    /* generate an id. the generated ID should be as short and as unique as possible:
    - short, because it may also used as part of Message-ID headers or in QR codes
    - unique as two IDs generated on two devices should not be the same. However, collisions are not world-wide but only by the few contacts.
    IDs generated by this function are 66 bit wide and are returned as 11 base64 characters.
    If possible, RNG of OpenSSL is used.

    Additional information when used as a message-id or group-id:
    - for OUTGOING messages this ID is written to the header as `Chat-Group-ID:` and is added to the message ID as Gr.<grpid>.<random>@<random>
    - for INCOMING messages, the ID is taken from the Chat-Group-ID-header or from the Message-ID in the In-Reply-To: or References:-Header
    - the group-id should be a string with the characters [a-zA-Z0-9\-_] */

    let mut rng = thread_rng();
    let buf: [u32; 3] = [rng.gen(), rng.gen(), rng.gen()];

    encode_66bits_as_base64(buf[0usize], buf[1usize], buf[2usize])
}

/// Encode 66 bits as a base64 string.
/// This is useful for ID generating with short strings as we save 5 character
/// in each id compared to 64 bit hex encoding. For a typical group ID, these
/// are 10 characters (grpid+msgid):
///    hex:    64 bit, 4 bits/character, length = 64/4 = 16 characters
///    base64: 64 bit, 6 bits/character, length = 64/6 = 11 characters (plus 2 additional bits)
/// Only the lower 2 bits of `fill` are used.
fn encode_66bits_as_base64(v1: u32, v2: u32, fill: u32) -> String {
    use byteorder::{BigEndian, WriteBytesExt};

    let mut wrapped_writer = Vec::new();
    {
        let mut enc = base64::write::EncoderWriter::new(&mut wrapped_writer, base64::URL_SAFE);
        enc.write_u32::<BigEndian>(v1).unwrap();
        enc.write_u32::<BigEndian>(v2).unwrap();
        enc.write_u8(((fill & 0x3) as u8) << 6).unwrap();
        enc.finish().unwrap();
    }
    assert_eq!(wrapped_writer.pop(), Some(b'A')); // Remove last "A"
    String::from_utf8(wrapped_writer).unwrap()
}

pub(crate) fn dc_create_incoming_rfc724_mid(
    message_timestamp: i64,
    contact_id_from: u32,
    contact_ids_to: &[u32],
) -> Option<String> {
    if contact_ids_to.is_empty() {
        return None;
    }
    /* find out the largest receiver ID (we could also take the smallest, but it should be unique) */
    let largest_id_to = contact_ids_to.iter().max().copied().unwrap_or_default();

    let result = format!(
        "{}-{}-{}@stub",
        message_timestamp, contact_id_from, largest_id_to
    );
    Some(result)
}

/// Function generates a Message-ID that can be used for a new outgoing message.
/// - this function is called for all outgoing messages.
/// - the message ID should be globally unique
/// - do not add a counter or any private data as this leaks information unncessarily
pub(crate) fn dc_create_outgoing_rfc724_mid(grpid: Option<&str>, from_addr: &str) -> String {
    let hostname = from_addr
        .find('@')
        .map(|k| &from_addr[k..])
        .unwrap_or("@nohost");
    match grpid {
        Some(grpid) => format!("Gr.{}.{}{}", grpid, dc_create_id(), hostname),
        None => format!("Mr.{}.{}{}", dc_create_id(), dc_create_id(), hostname),
    }
}

/// Extract the group id (grpid) from a message id (mid)
///
/// # Arguments
///
/// * `mid` - A string that holds the message id
pub(crate) fn dc_extract_grpid_from_rfc724_mid(mid: &str) -> Option<&str> {
    if mid.len() < 9 || !mid.starts_with("Gr.") {
        return None;
    }

    if let Some(mid_without_offset) = mid.get(3..) {
        if let Some(grpid_len) = mid_without_offset.find('.') {
            /* strict length comparison, the 'Gr.' magic is weak enough */
            if grpid_len == 11 || grpid_len == 16 {
                return Some(mid_without_offset.get(0..grpid_len).unwrap());
            }
        }
    }

    None
}

pub(crate) fn dc_extract_grpid_from_rfc724_mid_list(list: *const clist) -> *mut libc::c_char {
    if !list.is_null() {
        unsafe {
            for cur in (*list).into_iter() {
                let mid = as_str(cur as *const libc::c_char);

                if let Some(grpid) = dc_extract_grpid_from_rfc724_mid(mid) {
                    return grpid.strdup();
                }
            }
        }
    }

    ptr::null_mut()
}

pub(crate) fn dc_ensure_no_slash_safe(path: &str) -> &str {
    if path.ends_with('/') || path.ends_with('\\') {
        return &path[..path.len() - 1];
    }
    path
}

// Function returns a sanitized basename that does not contain
// win/linux path separators and also not any non-ascii chars
fn get_safe_basename(filename: &str) -> String {
    // return the (potentially mangled) basename of the input filename
    // this might be a path that comes in from another operating system
    let mut index: usize = 0;

    if let Some(unix_index) = filename.rfind('/') {
        index = unix_index + 1;
    }
    if let Some(win_index) = filename.rfind('\\') {
        index = max(index, win_index + 1);
    }
    if index >= filename.len() {
        "nobasename".to_string()
    } else {
        // we don't allow any non-ascii to be super-safe
        filename[index..].replace(|c: char| !c.is_ascii() || c == ':', "-")
    }
}

pub fn dc_derive_safe_stem_ext(filename: &str) -> (String, String) {
    let basename = get_safe_basename(&filename);
    let (mut stem, mut ext) = if let Some(index) = basename.rfind('.') {
        (
            basename[0..index].to_string(),
            basename[index..].to_string(),
        )
    } else {
        (basename, "".to_string())
    };
    // limit length of stem and ext
    stem.truncate(32);
    ext.truncate(32);
    (stem, ext)
}

// the returned suffix is lower-case
#[allow(non_snake_case)]
pub fn dc_get_filesuffix_lc(path_filename: impl AsRef<str>) -> Option<String> {
    if let Some(p) = Path::new(path_filename.as_ref()).extension() {
        Some(p.to_string_lossy().to_lowercase())
    } else {
        None
    }
}

/// Returns the `(width, height)` of the given image buffer.
pub fn dc_get_filemeta(buf: &[u8]) -> Result<(u32, u32), Error> {
    let meta = image_meta::load_from_buf(buf)?;

    Ok((meta.dimensions.width, meta.dimensions.height))
}

/// Expand paths relative to $BLOBDIR into absolute paths.
///
/// If `path` starts with "$BLOBDIR", replaces it with the blobdir path.
/// Otherwise, returns path as is.
pub(crate) fn dc_get_abs_path<P: AsRef<std::path::Path>>(
    context: &Context,
    path: P,
) -> std::path::PathBuf {
    let p: &std::path::Path = path.as_ref();
    if let Ok(p) = p.strip_prefix("$BLOBDIR") {
        context.get_blobdir().join(p)
    } else {
        p.into()
    }
}

pub(crate) fn dc_file_exist(context: &Context, path: impl AsRef<std::path::Path>) -> bool {
    dc_get_abs_path(context, &path).exists()
}

pub(crate) fn dc_get_filebytes(context: &Context, path: impl AsRef<std::path::Path>) -> u64 {
    let path_abs = dc_get_abs_path(context, &path);
    match fs::metadata(&path_abs) {
        Ok(meta) => meta.len() as u64,
        Err(_err) => 0,
    }
}

pub(crate) fn dc_delete_file(context: &Context, path: impl AsRef<std::path::Path>) -> bool {
    let path_abs = dc_get_abs_path(context, &path);
    if !path_abs.exists() {
        return false;
    }
    if !path_abs.is_file() {
        warn!(
            context,
            "refusing to delete non-file \"{}\".",
            path.as_ref().display()
        );
        return false;
    }

    let dpath = format!("{}", path.as_ref().to_string_lossy());
    match fs::remove_file(path_abs) {
        Ok(_) => {
            context.call_cb(Event::DeletedBlobFile(dpath));
            true
        }
        Err(_err) => {
            warn!(context, "Cannot delete \"{}\".", dpath);
            false
        }
    }
}

pub(crate) fn dc_copy_file(
    context: &Context,
    src: impl AsRef<std::path::Path>,
    dest: impl AsRef<std::path::Path>,
) -> bool {
    let src_abs = dc_get_abs_path(context, &src);
    let dest_abs = dc_get_abs_path(context, &dest);
    match fs::copy(&src_abs, &dest_abs) {
        Ok(_) => true,
        Err(_) => {
            error!(
                context,
                "Cannot copy \"{}\" to \"{}\".",
                src.as_ref().display(),
                dest.as_ref().display(),
            );
            false
        }
    }
}

pub(crate) fn dc_create_folder(context: &Context, path: impl AsRef<std::path::Path>) -> bool {
    let path_abs = dc_get_abs_path(context, &path);
    if !path_abs.exists() {
        match fs::create_dir_all(path_abs) {
            Ok(_) => true,
            Err(_err) => {
                warn!(
                    context,
                    "Cannot create directory \"{}\".",
                    path.as_ref().display(),
                );
                false
            }
        }
    } else {
        true
    }
}

/// Write a the given content to provied file path.
pub(crate) fn dc_write_file(context: &Context, path: impl AsRef<Path>, buf: &[u8]) -> bool {
    let path_abs = dc_get_abs_path(context, &path);
    if let Err(_err) = fs::write(&path_abs, buf) {
        warn!(
            context,
            "Cannot write {} bytes to \"{}\".",
            buf.len(),
            path.as_ref().display(),
        );
        false
    } else {
        true
    }
}

pub fn dc_read_file<P: AsRef<std::path::Path>>(
    context: &Context,
    path: P,
) -> Result<Vec<u8>, Error> {
    let path_abs = dc_get_abs_path(context, &path);

    match fs::read(&path_abs) {
        Ok(bytes) => Ok(bytes),
        Err(err) => {
            warn!(
                context,
                "Cannot read \"{}\" or file is empty.",
                path.as_ref().display()
            );
            Err(err.into())
        }
    }
}

pub(crate) fn dc_get_next_backup_path(
    folder: impl AsRef<Path>,
    backup_time: i64,
) -> Result<PathBuf, Error> {
    let folder = PathBuf::from(folder.as_ref());
    let stem = chrono::NaiveDateTime::from_timestamp(backup_time, 0)
        .format("delta-chat-%Y-%m-%d")
        .to_string();

    // 64 backup files per day should be enough for everyone
    for i in 0..64 {
        let mut path = folder.clone();
        path.push(format!("{}-{}.bak", stem, i));
        if !path.exists() {
            return Ok(path);
        }
    }
    bail!("could not create backup file, disk full?");
}

pub(crate) fn dc_is_blobdir_path(context: &Context, path: impl AsRef<str>) -> bool {
    context
        .get_blobdir()
        .to_str()
        .map(|s| path.as_ref().starts_with(s))
        .unwrap_or_default()
        || path.as_ref().starts_with("$BLOBDIR")
}

fn dc_make_rel_path(context: &Context, path: &mut String) {
    if context
        .get_blobdir()
        .to_str()
        .map(|s| path.starts_with(s))
        .unwrap_or_default()
    {
        *path = path.replace(
            context.get_blobdir().to_str().unwrap_or_default(),
            "$BLOBDIR",
        );
    }
}

pub(crate) fn dc_make_rel_and_copy(context: &Context, path: &mut String) -> bool {
    if dc_is_blobdir_path(context, &path) {
        dc_make_rel_path(context, path);
        return true;
    }
    if let Ok(blobdir_path) = context.copy_to_blobdir(&path) {
        *path = blobdir_path;
        return true;
    }
    false
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

    cstr.to_str()
        .map(|s| s.to_string())
        .unwrap_or_else(|_| cstr.to_string_lossy().to_string())
}

pub fn as_str<'a>(s: *const libc::c_char) -> &'a str {
    as_str_safe(s).unwrap_or_else(|err| panic!("{}", err))
}

/// Converts a C string to either a Rust `&str` or `None` if  it is a null pointer.
pub fn as_opt_str<'a>(s: *const libc::c_char) -> Option<&'a str> {
    if s.is_null() {
        return None;
    }
    Some(as_str(s))
}

fn as_str_safe<'a>(s: *const libc::c_char) -> Result<&'a str, Error> {
    assert!(!s.is_null(), "cannot be used on null pointers");

    let cstr = unsafe { CStr::from_ptr(s) };

    cstr.to_str()
        .map_err(|err| format_err!("Non utf8 string: '{:?}' ({:?})", cstr.to_bytes(), err))
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
    std::path::Path::new(as_str(s))
}

pub(crate) fn time() -> i64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

/// Very simple email address wrapper.
///
/// Represents an email address, right now just the `name@domain` portion.
///
/// # Example
///
/// ```
/// use deltachat::dc_tools::EmailAddress;
/// let email = match EmailAddress::new("someone@example.com") {
///     Ok(addr) => addr,
///     Err(e) => panic!("Error parsing address, error was {}", e),
/// };
/// assert_eq!(&email.local, "someone");
/// assert_eq!(&email.domain, "example.com");
/// assert_eq!(email.to_string(), "someone@example.com");
/// ```
#[derive(Debug, PartialEq, Clone)]
pub struct EmailAddress {
    pub local: String,
    pub domain: String,
}

impl EmailAddress {
    pub fn new(input: &str) -> Result<Self, Error> {
        input.parse::<EmailAddress>()
    }
}

impl fmt::Display for EmailAddress {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}@{}", self.local, self.domain)
    }
}

impl FromStr for EmailAddress {
    type Err = Error;

    /// Performs a dead-simple parse of an email address.
    fn from_str(input: &str) -> Result<EmailAddress, Error> {
        ensure!(!input.is_empty(), "empty string is not valid");
        let parts: Vec<&str> = input.rsplitn(2, '@').collect();

        ensure!(parts.len() > 1, "missing '@' character");
        let local = parts[1];
        let domain = parts[0];

        ensure!(
            !local.is_empty(),
            "empty string is not valid for local part"
        );
        ensure!(domain.len() > 3, "domain is too short");

        let dot = domain.find('.');
        ensure!(dot.is_some(), "invalid domain");
        ensure!(dot.unwrap() < domain.len() - 2, "invalid domain");

        Ok(EmailAddress {
            local: local.to_string(),
            domain: domain.to_string(),
        })
    }
}

/// Utility to check if a in the binary represantion of listflags
/// the bit at position bitindex is 1.
pub(crate) fn listflags_has(listflags: u32, bitindex: usize) -> bool {
    let listflags = listflags as usize;
    (listflags & bitindex) == bitindex
}

pub(crate) unsafe fn strdup(s: *const libc::c_char) -> *mut libc::c_char {
    if s.is_null() {
        return std::ptr::null_mut();
    }

    let slen = strlen(s);
    let result = libc::malloc(slen + 1);
    if result.is_null() {
        return std::ptr::null_mut();
    }

    memcpy(result, s as *const _, slen + 1);
    result as *mut _
}

pub(crate) unsafe fn strcasecmp(s1: *const libc::c_char, s2: *const libc::c_char) -> libc::c_int {
    let s1 = std::ffi::CStr::from_ptr(s1)
        .to_string_lossy()
        .to_lowercase();
    let s2 = std::ffi::CStr::from_ptr(s2)
        .to_string_lossy()
        .to_lowercase();
    if s1 == s2 {
        0
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use libc::{free, strcmp};
    use std::convert::TryInto;
    use std::ffi::CStr;

    use crate::constants::*;
    use crate::test_utils::*;

    #[test]
    fn test_dc_strdup() {
        unsafe {
            let str_a = b"foobar\x00" as *const u8 as *const libc::c_char;
            let str_a_copy = dc_strdup(str_a);

            // Value of str_a_copy should equal foobar
            assert_eq!(
                CStr::from_ptr(str_a_copy),
                CString::new("foobar").unwrap().as_c_str()
            );
            // Address of str_a should be different from str_a_copy
            assert_ne!(str_a, str_a_copy);

            let str_a = std::ptr::null() as *const libc::c_char;
            let str_a_copy = dc_strdup(str_a);
            // Value of str_a_copy should equal ""
            assert_eq!(
                CStr::from_ptr(str_a_copy),
                CString::new("").unwrap().as_c_str()
            );
            assert_ne!(str_a, str_a_copy);
        }
    }

    #[test]
    fn test_dc_ltrim() {
        unsafe {
            let html: *const libc::c_char =
                b"\r\r\nline1<br>\r\n\r\n\r\rline2\n\r\x00" as *const u8 as *const libc::c_char;
            let out: *mut libc::c_char = strndup(html, strlen(html) as libc::c_ulong);

            dc_ltrim(out);

            assert_eq!(
                CStr::from_ptr(out as *const libc::c_char).to_str().unwrap(),
                "line1<br>\r\n\r\n\r\rline2\n\r"
            );
        }
    }

    #[test]
    fn test_dc_rtrim() {
        unsafe {
            let html: *const libc::c_char =
                b"\r\r\nline1<br>\r\n\r\n\r\rline2\n\r\x00" as *const u8 as *const libc::c_char;
            let out: *mut libc::c_char = strndup(html, strlen(html) as libc::c_ulong);

            dc_rtrim(out);

            assert_eq!(
                CStr::from_ptr(out as *const libc::c_char).to_str().unwrap(),
                "\r\r\nline1<br>\r\n\r\n\r\rline2"
            );
        }
    }

    #[test]
    fn test_dc_trim() {
        unsafe {
            let html: *const libc::c_char =
                b"\r\r\nline1<br>\r\n\r\n\r\rline2\n\r\x00" as *const u8 as *const libc::c_char;
            let out: *mut libc::c_char = strndup(html, strlen(html) as libc::c_ulong);

            dc_trim(out);

            assert_eq!(
                CStr::from_ptr(out as *const libc::c_char).to_str().unwrap(),
                "line1<br>\r\n\r\n\r\rline2"
            );
        }
    }

    #[test]
    fn test_rust_ftoa() {
        assert_eq!("1.22", format!("{}", 1.22));
    }

    #[test]
    fn test_dc_truncate_1() {
        let s = "this is a little test string";
        assert_eq!(dc_truncate(s, 16, false), "this is a [...]");
        assert_eq!(dc_truncate(s, 16, true), "this is a ...");
    }

    #[test]
    fn test_dc_truncate_2() {
        assert_eq!(dc_truncate("1234", 2, false), "1234");
        assert_eq!(dc_truncate("1234", 2, true), "1234");
    }

    #[test]
    fn test_dc_truncate_3() {
        assert_eq!(dc_truncate("1234567", 1, false), "1[...]");
        assert_eq!(dc_truncate("1234567", 1, true), "1...");
    }

    #[test]
    fn test_dc_truncate_4() {
        assert_eq!(dc_truncate("123456", 4, false), "123456");
        assert_eq!(dc_truncate("123456", 4, true), "123456");
    }

    #[test]
    fn test_dc_truncate_edge() {
        assert_eq!(dc_truncate("", 4, false), "");
        assert_eq!(dc_truncate("", 4, true), "");

        assert_eq!(dc_truncate("\n  hello \n world", 4, false), "\n  [...]");
        assert_eq!(dc_truncate("\n  hello \n world", 4, true), "\n  ...");

        assert_eq!(
            dc_truncate("𐠈0Aᝮa𫝀®!ꫛa¡0A𐢧00𐹠®A  丽ⷐએ", 1, false),
            "𐠈[...]"
        );
        assert_eq!(
            dc_truncate("𐠈0Aᝮa𫝀®!ꫛa¡0A𐢧00𐹠®A  丽ⷐએ", 0, false),
            "𐠈0Aᝮa𫝀®!ꫛa¡0A𐢧00𐹠®A  丽ⷐએ"
        );

        // 9 characters, so no truncation
        assert_eq!(
            dc_truncate("𑒀ὐ￠🜀\u{1e01b}A a🟠", 6, false),
            "𑒀ὐ￠🜀\u{1e01b}A a🟠",
        );

        // 12 characters, truncation
        assert_eq!(
            dc_truncate("𑒀ὐ￠🜀\u{1e01b}A a🟠bcd", 6, false),
            "𑒀ὐ￠🜀\u{1e01b}A[...]",
        );
    }

    /* calls free() for each item content */
    unsafe fn clist_free_content(haystack: *const clist) {
        let mut iter = (*haystack).first;

        while !iter.is_null() {
            free((*iter).data);
            (*iter).data = ptr::null_mut();
            iter = if !iter.is_null() {
                (*iter).next
            } else {
                ptr::null_mut()
            }
        }
    }

    fn strndup(s: *const libc::c_char, n: libc::c_ulong) -> *mut libc::c_char {
        if s.is_null() {
            return std::ptr::null_mut();
        }

        let end = std::cmp::min(n as usize, unsafe { strlen(s) });
        unsafe {
            let result = libc::malloc(end + 1);
            memcpy(result, s as *const _, end);
            std::ptr::write_bytes(result.offset(end as isize), b'\x00', 1);

            result as *mut _
        }
    }

    #[test]
    fn test_dc_str_to_clist_1() {
        unsafe {
            let list = dc_str_to_clist("", " ");
            assert_eq!((*list).count, 1);
            clist_free_content(list);
            clist_free(list);
        }
    }

    #[test]
    fn test_dc_str_to_clist_4() {
        unsafe {
            let list: *mut clist = dc_str_to_clist("foo bar test", " ");
            assert_eq!((*list).count, 3);
            let str: *mut libc::c_char =
                dc_str_from_clist(list, b" \x00" as *const u8 as *const libc::c_char);

            assert_eq!(
                CStr::from_ptr(str as *const libc::c_char).to_str().unwrap(),
                "foo bar test"
            );

            clist_free_content(list);
            clist_free(list);
            free(str as *mut libc::c_void);
        }
    }

    #[test]
    fn test_dc_create_id() {
        let buf = dc_create_id();
        assert_eq!(buf.len(), 11);
    }

    #[test]
    fn test_encode_66bits_as_base64() {
        assert_eq!(
            encode_66bits_as_base64(0x01234567, 0x89abcdef, 0),
            "ASNFZ4mrze8"
        );
        assert_eq!(
            encode_66bits_as_base64(0x01234567, 0x89abcdef, 1),
            "ASNFZ4mrze9"
        );
        assert_eq!(
            encode_66bits_as_base64(0x01234567, 0x89abcdef, 2),
            "ASNFZ4mrze-"
        );
        assert_eq!(
            encode_66bits_as_base64(0x01234567, 0x89abcdef, 3),
            "ASNFZ4mrze_"
        );
    }

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

    #[test]
    fn test_dc_extract_grpid_from_rfc724_mid() {
        // Should return None if we pass invalid mid
        let mid = "foobar";
        let grpid = dc_extract_grpid_from_rfc724_mid(mid);
        assert_eq!(grpid, None);

        // Should return None if grpid has a length which is not 11 or 16
        let mid = "Gr.12345678.morerandom@domain.de";
        let grpid = dc_extract_grpid_from_rfc724_mid(mid);
        assert_eq!(grpid, None);

        // Should return extracted grpid for grpid with length of 11
        let mid = "Gr.12345678901.morerandom@domain.de";
        let grpid = dc_extract_grpid_from_rfc724_mid(mid);
        assert_eq!(grpid, Some("12345678901"));

        // Should return extracted grpid for grpid with length of 11
        let mid = "Gr.1234567890123456.morerandom@domain.de";
        let grpid = dc_extract_grpid_from_rfc724_mid(mid);
        assert_eq!(grpid, Some("1234567890123456"));
    }

    #[test]
    fn test_dc_create_outgoing_rfc724_mid() {
        // create a normal message-id
        let mid = dc_create_outgoing_rfc724_mid(None, "foo@bar.de");
        assert!(mid.starts_with("Mr."));
        assert!(mid.ends_with("bar.de"));
        assert!(dc_extract_grpid_from_rfc724_mid(mid.as_str()).is_none());

        // create a message-id containing a group-id
        let grpid = dc_create_id();
        let mid = dc_create_outgoing_rfc724_mid(Some(&grpid), "foo@bar.de");
        assert!(mid.starts_with("Gr."));
        assert!(mid.ends_with("bar.de"));
        assert_eq!(
            dc_extract_grpid_from_rfc724_mid(mid.as_str()),
            Some(grpid.as_str())
        );
    }

    #[test]
    fn test_emailaddress_parse() {
        assert_eq!(EmailAddress::new("").is_ok(), false);
        assert_eq!(
            EmailAddress::new("user@domain.tld").unwrap(),
            EmailAddress {
                local: "user".into(),
                domain: "domain.tld".into(),
            }
        );
        assert_eq!(EmailAddress::new("uuu").is_ok(), false);
        assert_eq!(EmailAddress::new("dd.tt").is_ok(), false);
        assert_eq!(EmailAddress::new("tt.dd@uu").is_ok(), false);
        assert_eq!(EmailAddress::new("u@d").is_ok(), false);
        assert_eq!(EmailAddress::new("u@d.").is_ok(), false);
        assert_eq!(EmailAddress::new("u@d.t").is_ok(), false);
        assert_eq!(
            EmailAddress::new("u@d.tt").unwrap(),
            EmailAddress {
                local: "u".into(),
                domain: "d.tt".into(),
            }
        );
        assert_eq!(EmailAddress::new("u@.tt").is_ok(), false);
        assert_eq!(EmailAddress::new("@d.tt").is_ok(), false);
    }

    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_dc_truncate(
            buf: String,
            approx_chars in 0..10000usize,
            do_unwrap: bool,
        ) {
            let res = dc_truncate(&buf, approx_chars, do_unwrap);
            let el_len = if do_unwrap { 3 } else { 5 };
            let l = res.chars().count();
            if approx_chars > 0 {
                assert!(
                    l <= approx_chars + el_len,
                    "buf: '{}' - res: '{}' - len {}, approx {}",
                    &buf, &res, res.len(), approx_chars
                );
            } else {
                assert_eq!(&res, &buf);
            }

            if approx_chars > 0 && buf.chars().count() > approx_chars + el_len {
                let l = res.len();
                if do_unwrap {
                    assert_eq!(&res[l-3..l], "...", "missing ellipsis in {}", &res);
                } else {
                    assert_eq!(&res[l-5..l], "[...]", "missing ellipsis in {}", &res);
                }
            }
        }
    }

    #[test]
    fn test_dc_create_incoming_rfc724_mid() {
        let res = dc_create_incoming_rfc724_mid(123, 45, &vec![6, 7]);
        assert_eq!(res, Some("123-45-7@stub".into()));
    }

    #[test]
    fn test_dc_make_rel_path() {
        let t = dummy_context();
        let mut foo: String = t
            .ctx
            .get_blobdir()
            .join("foo")
            .to_string_lossy()
            .into_owned();
        dc_make_rel_path(&t.ctx, &mut foo);
        assert_eq!(foo, format!("$BLOBDIR{}foo", std::path::MAIN_SEPARATOR));
    }

    #[test]
    fn test_strndup() {
        unsafe {
            let res = strndup(b"helloworld\x00" as *const u8 as *const libc::c_char, 4);
            assert_eq!(
                to_string_lossy(res),
                to_string_lossy(b"hell\x00" as *const u8 as *const libc::c_char)
            );
            assert_eq!(strlen(res), 4);
            free(res as *mut _);
        }
    }

    #[test]
    fn test_file_get_safe_basename() {
        assert_eq!(get_safe_basename("12312/hello"), "hello");
        assert_eq!(get_safe_basename("12312\\hello"), "hello");
        assert_eq!(get_safe_basename("//12312\\hello"), "hello");
        assert_eq!(get_safe_basename("//123:12\\hello"), "hello");
        assert_eq!(get_safe_basename("//123:12/\\\\hello"), "hello");
        assert_eq!(get_safe_basename("//123:12//hello"), "hello");
        assert_eq!(get_safe_basename("//123:12//"), "nobasename");
        assert_eq!(get_safe_basename("//123:12/"), "nobasename");
        assert!(get_safe_basename("123\x012.hello").ends_with(".hello"));
    }

    #[test]
    fn test_file_handling() {
        let t = dummy_context();
        let context = &t.ctx;

        assert!(!dc_delete_file(context, "$BLOBDIR/lkqwjelqkwlje"));
        if dc_file_exist(context, "$BLOBDIR/foobar")
            || dc_file_exist(context, "$BLOBDIR/dada")
            || dc_file_exist(context, "$BLOBDIR/foobar.dadada")
            || dc_file_exist(context, "$BLOBDIR/foobar-folder")
        {
            dc_delete_file(context, "$BLOBDIR/foobar");
            dc_delete_file(context, "$BLOBDIR/dada");
            dc_delete_file(context, "$BLOBDIR/foobar.dadada");
            dc_delete_file(context, "$BLOBDIR/foobar-folder");
        }
        assert!(dc_write_file(context, "$BLOBDIR/foobar", b"content"));
        assert!(dc_file_exist(context, "$BLOBDIR/foobar",));
        assert!(!dc_file_exist(context, "$BLOBDIR/foobarx"));
        assert_eq!(dc_get_filebytes(context, "$BLOBDIR/foobar"), 7);

        let abs_path = context
            .get_blobdir()
            .join("foobar")
            .to_string_lossy()
            .to_string();

        assert!(dc_is_blobdir_path(context, &abs_path));

        assert!(dc_is_blobdir_path(context, "$BLOBDIR/fofo",));
        assert!(!dc_is_blobdir_path(context, "/BLOBDIR/fofo",));
        assert!(dc_file_exist(context, &abs_path));

        assert!(dc_copy_file(context, "$BLOBDIR/foobar", "$BLOBDIR/dada",));

        assert_eq!(dc_get_filebytes(context, "$BLOBDIR/dada",), 7);

        let buf = dc_read_file(context, "$BLOBDIR/dada").unwrap();

        assert_eq!(buf.len(), 7);
        assert_eq!(&buf, b"content");

        assert!(dc_delete_file(context, "$BLOBDIR/foobar"));
        assert!(dc_delete_file(context, "$BLOBDIR/dada"));
        assert!(dc_create_folder(context, "$BLOBDIR/foobar-folder"));
        assert!(dc_file_exist(context, "$BLOBDIR/foobar-folder",));
        assert!(!dc_delete_file(context, "$BLOBDIR/foobar-folder"));

        let fn0 = "$BLOBDIR/data.data";
        assert!(dc_write_file(context, &fn0, b"content"));

        assert!(dc_delete_file(context, &fn0));
        assert!(!dc_file_exist(context, &fn0));
    }

    #[test]
    fn test_listflags_has() {
        let listflags: u32 = 0x1101;
        assert!(listflags_has(listflags, 0x1) == true);
        assert!(listflags_has(listflags, 0x10) == false);
        assert!(listflags_has(listflags, 0x100) == true);
        assert!(listflags_has(listflags, 0x1000) == true);
        let listflags: u32 = (DC_GCL_ADD_SELF | DC_GCL_VERIFIED_ONLY).try_into().unwrap();
        assert!(listflags_has(listflags, DC_GCL_VERIFIED_ONLY) == true);
        assert!(listflags_has(listflags, DC_GCL_ADD_SELF) == true);
        let listflags: u32 = DC_GCL_VERIFIED_ONLY.try_into().unwrap();
        assert!(listflags_has(listflags, DC_GCL_ADD_SELF) == false);
    }

    #[test]
    fn test_dc_remove_cr_chars() {
        unsafe {
            let input = "foo\r\nbar".strdup();
            dc_remove_cr_chars(input);
            assert_eq!("foo\nbar", to_string_lossy(input));
            free(input.cast());

            let input = "\rfoo\r\rbar\r".strdup();
            dc_remove_cr_chars(input);
            assert_eq!("foobar", to_string_lossy(input));
            free(input.cast());
        }
    }
}
