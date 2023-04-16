//! Some tools and enhancements to the used libraries, there should be
//! no references to Context and other "larger" entities here.

#![allow(missing_docs)]

use std::borrow::Cow;
use std::fmt;
use std::io::{Cursor, Write};
use std::mem;
use std::path::{Path, PathBuf};
use std::str::from_utf8;
use std::time::{Duration, SystemTime};

use anyhow::{bail, Context as _, Result};
use base64::Engine as _;
use chrono::{Local, NaiveDateTime, NaiveTime, TimeZone};
use futures::{StreamExt, TryStreamExt};
use mailparse::dateparse;
use mailparse::headers::Headers;
use mailparse::MailHeaderMap;
use rand::{thread_rng, Rng};
use tokio::{fs, io};

use crate::chat::{add_device_msg, add_device_msg_with_importance};
use crate::constants::{DC_ELLIPSIS, DC_OUTDATED_WARNING_DAYS};
use crate::context::Context;
use crate::events::EventType;
use crate::message::{Message, Viewtype};
use crate::stock_str;

/// Shortens a string to a specified length and adds "[...]" to the
/// end of the shortened string.
#[allow(clippy::indexing_slicing)]
pub(crate) fn truncate(buf: &str, approx_chars: usize) -> Cow<str> {
    let count = buf.chars().count();
    if count > approx_chars + DC_ELLIPSIS.len() {
        let end_pos = buf
            .char_indices()
            .nth(approx_chars)
            .map(|(n, _)| n)
            .unwrap_or_default();

        if let Some(index) = buf[..end_pos].rfind(|c| c == ' ' || c == '\n') {
            Cow::Owned(format!("{}{}", &buf[..=index], DC_ELLIPSIS))
        } else {
            Cow::Owned(format!("{}{}", &buf[..end_pos], DC_ELLIPSIS))
        }
    } else {
        Cow::Borrowed(buf)
    }
}

/// Shortens a string to a specified line count and adds "[...]" to the
/// end of the shortened string.
///
/// returns tuple with the String and a boolean whether is was truncated
pub(crate) fn truncate_by_lines(
    buf: String,
    max_lines: usize,
    max_line_len: usize,
) -> (String, bool) {
    let mut lines = 0;
    let mut line_chars = 0;
    let mut break_point: Option<usize> = None;

    for (index, char) in buf.char_indices() {
        if char == '\n' {
            line_chars = 0;
            lines += 1;
        } else {
            line_chars += 1;
            if line_chars > max_line_len {
                line_chars = 1;
                lines += 1;
            }
        }
        if lines == max_lines {
            break_point = Some(index);
            break;
        }
    }

    if let Some(end_pos) = break_point {
        // Text has too many lines and needs to be truncated.
        let text = {
            if let Some(buffer) = buf.get(..end_pos) {
                if let Some(index) = buffer.rfind(|c| c == ' ' || c == '\n') {
                    buf.get(..=index)
                } else {
                    buf.get(..end_pos)
                }
            } else {
                None
            }
        };

        if let Some(truncated_text) = text {
            (format!("{truncated_text}{DC_ELLIPSIS}"), true)
        } else {
            // In case of indexing/slicing error, we return an error
            // message as a preview and add HTML version. This should
            // never happen.
            let error_text = "[Truncation of the message failed, this is a bug in the Delta Chat core. Please report it.\nYou can still open the full text to view the original message.]";
            (error_text.to_string(), true)
        }
    } else {
        // text is unchanged
        (buf, false)
    }
}

/* ******************************************************************************
 * date/time tools
 ******************************************************************************/

/// Converts Unix time in seconds to a local timestamp string.
pub fn timestamp_to_str(wanted: i64) -> String {
    if let Some(ts) = Local.timestamp_opt(wanted, 0).single() {
        ts.format("%Y.%m.%d %H:%M:%S").to_string()
    } else {
        // Out of range number of seconds.
        "??.??.?? ??:??:??".to_string()
    }
}

/// Converts duration to string representation suitable for logs.
pub fn duration_to_str(duration: Duration) -> String {
    let secs = duration.as_secs();
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = (secs % 3600) % 60;
    format!("{h}h {m}m {s}s")
}

pub(crate) fn gm2local_offset() -> i64 {
    /* returns the offset that must be _added_ to an UTC/GMT-time to create the localtime.
    the function may return negative values. */
    let lt = Local::now();
    i64::from(lt.offset().local_minus_utc())
}

/// Returns the current smeared timestamp,
///
/// The returned timestamp MUST NOT be sent out.
pub(crate) fn smeared_time(context: &Context) -> i64 {
    let now = time();
    let ts = context.smeared_timestamp.current();
    std::cmp::max(ts, now)
}

/// Returns a timestamp that is guaranteed to be unique.
pub(crate) fn create_smeared_timestamp(context: &Context) -> i64 {
    let now = time();
    context.smeared_timestamp.create(now)
}

// creates `count` timestamps that are guaranteed to be unique.
// the first created timestamps is returned directly,
// get the other timestamps just by adding 1..count-1
pub(crate) fn create_smeared_timestamps(context: &Context, count: usize) -> i64 {
    let now = time();
    context.smeared_timestamp.create_n(now, count as i64)
}

/// Returns the last release timestamp as a unix timestamp compatible for comparison with time() and
/// database times.
pub fn get_release_timestamp() -> i64 {
    NaiveDateTime::new(
        *crate::release::DATE,
        NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
    )
    .timestamp_millis()
        / 1_000
}

// if the system time is not plausible, once a day, add a device message.
// for testing we're using time() as that is also used for message timestamps.
// moreover, add a warning if the app is outdated.
pub(crate) async fn maybe_add_time_based_warnings(context: &Context) {
    if !maybe_warn_on_bad_time(context, time(), get_release_timestamp()).await {
        maybe_warn_on_outdated(context, time(), get_release_timestamp()).await;
    }
}

async fn maybe_warn_on_bad_time(context: &Context, now: i64, known_past_timestamp: i64) -> bool {
    if now < known_past_timestamp {
        let mut msg = Message::new(Viewtype::Text);
        msg.text = Some(
            stock_str::bad_time_msg_body(
                context,
                &Local.timestamp_opt(now, 0).single().map_or_else(
                    || "YY-MM-DD hh:mm:ss".to_string(),
                    |ts| ts.format("%Y-%m-%d %H:%M:%S").to_string(),
                ),
            )
            .await,
        );
        if let Some(timestamp) = chrono::NaiveDateTime::from_timestamp_opt(now, 0) {
            add_device_msg_with_importance(
                context,
                Some(
                    format!(
                        "bad-time-warning-{}",
                        timestamp.format("%Y-%m-%d") // repeat every day
                    )
                    .as_str(),
                ),
                Some(&mut msg),
                true,
            )
            .await
            .ok();
        } else {
            warn!(context, "Can't convert current timestamp");
        }
        return true;
    }
    false
}

async fn maybe_warn_on_outdated(context: &Context, now: i64, approx_compile_time: i64) {
    if now > approx_compile_time + DC_OUTDATED_WARNING_DAYS * 24 * 60 * 60 {
        let mut msg = Message::new(Viewtype::Text);
        msg.text = Some(stock_str::update_reminder_msg_body(context).await);
        if let Some(timestamp) = chrono::NaiveDateTime::from_timestamp_opt(now, 0) {
            add_device_msg(
                context,
                Some(
                    format!(
                        "outdated-warning-{}",
                        timestamp.format("%Y-%m") // repeat every month
                    )
                    .as_str(),
                ),
                Some(&mut msg),
            )
            .await
            .ok();
        }
    }
}

/* Message-ID tools */

/// Generate an ID. The generated ID should be as short and as unique as possible:
/// - short, because it may also used as part of Message-ID headers or in QR codes
/// - unique as two IDs generated on two devices should not be the same. However, collisions are not world-wide but only by the few contacts.
/// IDs generated by this function are 66 bit wide and are returned as 11 base64 characters.
///
/// Additional information when used as a message-id or group-id:
/// - for OUTGOING messages this ID is written to the header as `Chat-Group-ID:` and is added to the message ID as `Gr.<grpid>.<random>@<random>`
/// - for INCOMING messages, the ID is taken from the Chat-Group-ID-header or from the Message-ID in the In-Reply-To: or References:-Header
/// - the group-id should be a string with the characters [a-zA-Z0-9\-_]
pub(crate) fn create_id() -> String {
    // ThreadRng implements CryptoRng trait and is supposed to be cryptographically secure.
    let mut rng = thread_rng();

    // Generate 72 random bits.
    let mut arr = [0u8; 9];
    rng.fill(&mut arr[..]);

    // Take 11 base64 characters containing 66 random bits.
    base64::engine::general_purpose::URL_SAFE
        .encode(arr)
        .chars()
        .take(11)
        .collect()
}

/// Function generates a Message-ID that can be used for a new outgoing message.
/// - this function is called for all outgoing messages.
/// - the message ID should be globally unique
/// - do not add a counter or any private data as this leaks information unncessarily
pub(crate) fn create_outgoing_rfc724_mid(grpid: Option<&str>, from_addr: &str) -> String {
    let hostname = from_addr
        .find('@')
        .and_then(|k| from_addr.get(k..))
        .unwrap_or("@nohost");
    match grpid {
        Some(grpid) => format!("Gr.{}.{}{}", grpid, create_id(), hostname),
        None => format!("Mr.{}.{}{}", create_id(), create_id(), hostname),
    }
}

/// Extract the group id (grpid) from a message id (mid)
///
/// # Arguments
///
/// * `mid` - A string that holds the message id.  Leading/Trailing <>
/// characters are automatically stripped.
pub(crate) fn extract_grpid_from_rfc724_mid(mid: &str) -> Option<&str> {
    let mid = mid.trim_start_matches('<').trim_end_matches('>');

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

// the returned suffix is lower-case
pub fn get_filesuffix_lc(path_filename: &str) -> Option<String> {
    Path::new(path_filename)
        .extension()
        .map(|p| p.to_string_lossy().to_lowercase())
}

/// Returns the `(width, height)` of the given image buffer.
pub fn get_filemeta(buf: &[u8]) -> Result<(u32, u32)> {
    let image = image::io::Reader::new(Cursor::new(buf)).with_guessed_format()?;
    let dimensions = image.into_dimensions()?;
    Ok(dimensions)
}

/// Expand paths relative to $BLOBDIR into absolute paths.
///
/// If `path` starts with "$BLOBDIR", replaces it with the blobdir path.
/// Otherwise, returns path as is.
pub(crate) fn get_abs_path(context: &Context, path: impl AsRef<Path>) -> PathBuf {
    let p: &Path = path.as_ref();
    if let Ok(p) = p.strip_prefix("$BLOBDIR") {
        context.get_blobdir().join(p)
    } else {
        p.into()
    }
}

pub(crate) async fn get_filebytes(context: &Context, path: impl AsRef<Path>) -> Result<u64> {
    let path_abs = get_abs_path(context, &path);
    let meta = fs::metadata(&path_abs).await?;
    Ok(meta.len())
}

pub(crate) async fn delete_file(context: &Context, path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    let path_abs = get_abs_path(context, path);
    if !path_abs.exists() {
        bail!("path {} does not exist", path_abs.display());
    }
    if !path_abs.is_file() {
        warn!(context, "refusing to delete non-file {}.", path.display());
        bail!("not a file: \"{}\"", path.display());
    }

    let dpath = format!("{}", path.to_string_lossy());
    fs::remove_file(path_abs)
        .await
        .with_context(|| format!("cannot delete {dpath:?}"))?;
    context.emit_event(EventType::DeletedBlobFile(dpath));
    Ok(())
}

pub async fn delete_files_in_dir(context: &Context, path: impl AsRef<Path>) -> Result<()> {
    let read_dir = tokio::fs::read_dir(path)
        .await
        .context("could not read dir to delete")?;
    let mut read_dir = tokio_stream::wrappers::ReadDirStream::new(read_dir);
    while let Some(entry) = read_dir.next().await {
        match entry {
            Ok(file) => {
                delete_file(context, file.file_name()).await?;
            }
            Err(e) => warn!(context, "Could not read file to delete: {}", e),
        }
    }
    Ok(())
}

pub(crate) async fn create_folder(
    context: &Context,
    path: impl AsRef<Path>,
) -> Result<(), io::Error> {
    let path_abs = get_abs_path(context, &path);
    if !path_abs.exists() {
        match fs::create_dir_all(path_abs).await {
            Ok(_) => Ok(()),
            Err(err) => {
                warn!(
                    context,
                    "Cannot create directory \"{}\": {}",
                    path.as_ref().display(),
                    err
                );
                Err(err)
            }
        }
    } else {
        Ok(())
    }
}

/// Write a the given content to provided file path.
pub(crate) async fn write_file(
    context: &Context,
    path: impl AsRef<Path>,
    buf: &[u8],
) -> Result<(), io::Error> {
    let path_abs = get_abs_path(context, &path);
    fs::write(&path_abs, buf).await.map_err(|err| {
        warn!(
            context,
            "Cannot write {} bytes to \"{}\": {}",
            buf.len(),
            path.as_ref().display(),
            err
        );
        err
    })
}

/// Reads the file and returns its context as a byte vector.
pub async fn read_file(context: &Context, path: impl AsRef<Path>) -> Result<Vec<u8>> {
    let path_abs = get_abs_path(context, &path);

    match fs::read(&path_abs).await {
        Ok(bytes) => Ok(bytes),
        Err(err) => {
            warn!(
                context,
                "Cannot read \"{}\" or file is empty: {}",
                path.as_ref().display(),
                err
            );
            Err(err.into())
        }
    }
}

pub async fn open_file(context: &Context, path: impl AsRef<Path>) -> Result<fs::File> {
    let path_abs = get_abs_path(context, &path);

    match fs::File::open(&path_abs).await {
        Ok(bytes) => Ok(bytes),
        Err(err) => {
            warn!(
                context,
                "Cannot read \"{}\" or file is empty: {}",
                path.as_ref().display(),
                err
            );
            Err(err.into())
        }
    }
}

pub fn open_file_std<P: AsRef<std::path::Path>>(
    context: &Context,
    path: P,
) -> Result<std::fs::File> {
    let p: PathBuf = path.as_ref().into();
    let path_abs = get_abs_path(context, p);

    match std::fs::File::open(path_abs) {
        Ok(bytes) => Ok(bytes),
        Err(err) => {
            warn!(
                context,
                "Cannot read \"{}\" or file is empty: {}",
                path.as_ref().display(),
                err
            );
            Err(err.into())
        }
    }
}

/// Reads directory and returns a vector of directory entries.
pub async fn read_dir(path: &Path) -> Result<Vec<fs::DirEntry>> {
    let res = tokio_stream::wrappers::ReadDirStream::new(fs::read_dir(path).await?)
        .try_collect()
        .await?;
    Ok(res)
}

pub(crate) fn time() -> i64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

/// Very simple email address wrapper.
///
/// Represents an email address, right now just the `name@domain` portion.
///
/// # Example
///
/// ```
/// use deltachat::tools::EmailAddress;
/// let email = match EmailAddress::new("someone@example.com") {
///     Ok(addr) => addr,
///     Err(e) => panic!("Error parsing address, error was {}", e),
/// };
/// assert_eq!(&email.local, "someone");
/// assert_eq!(&email.domain, "example.com");
/// assert_eq!(email.to_string(), "someone@example.com");
/// ```
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct EmailAddress {
    /// Local part of the email address.
    pub local: String,

    /// Email address domain.
    pub domain: String,
}

impl fmt::Display for EmailAddress {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}@{}", self.local, self.domain)
    }
}

impl EmailAddress {
    /// Performs a dead-simple parse of an email address.
    pub fn new(input: &str) -> Result<EmailAddress> {
        if input.is_empty() {
            bail!("empty string is not valid");
        }
        let parts: Vec<&str> = input.rsplitn(2, '@').collect();

        if input
            .chars()
            .any(|c| c.is_whitespace() || c == '<' || c == '>')
        {
            bail!("Email {:?} must not contain whitespaces, '>' or '<'", input);
        }

        match &parts[..] {
            [domain, local] => {
                if local.is_empty() {
                    bail!("empty string is not valid for local part in {:?}", input);
                }
                if domain.is_empty() {
                    bail!("missing domain after '@' in {:?}", input);
                }
                Ok(EmailAddress {
                    local: (*local).to_string(),
                    domain: (*domain).to_string(),
                })
            }
            _ => bail!("Email {:?} must contain '@' character", input),
        }
    }
}

impl rusqlite::types::ToSql for EmailAddress {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput> {
        let val = rusqlite::types::Value::Text(self.to_string());
        let out = rusqlite::types::ToSqlOutput::Owned(val);
        Ok(out)
    }
}

/// Sanitizes user input
/// - strip newlines
/// - strip malicious bidi characters
pub(crate) fn improve_single_line_input(input: &str) -> String {
    strip_rtlo_characters(input.replace(['\n', '\r'], " ").trim())
}

pub(crate) trait IsNoneOrEmpty<T> {
    /// Returns true if an Option does not contain a string
    /// or contains an empty string.
    fn is_none_or_empty(&self) -> bool;
}
impl<T> IsNoneOrEmpty<T> for Option<T>
where
    T: AsRef<str>,
{
    fn is_none_or_empty(&self) -> bool {
        !matches!(self, Some(s) if !s.as_ref().is_empty())
    }
}

pub fn remove_subject_prefix(last_subject: &str) -> String {
    let subject_start = if last_subject.starts_with("Chat:") {
        0
    } else {
        // "Antw:" is the longest abbreviation in
        // <https://en.wikipedia.org/wiki/List_of_email_subject_abbreviations#Abbreviations_in_other_languages>,
        // so look at the first _5_ characters:
        match last_subject.chars().take(5).position(|c| c == ':') {
            Some(prefix_end) => prefix_end + 1,
            None => 0,
        }
    };
    last_subject
        .chars()
        .skip(subject_start)
        .collect::<String>()
        .trim()
        .to_string()
}

// Types and methods to create hop-info for message-info

fn extract_address_from_receive_header<'a>(header: &'a str, start: &str) -> Option<&'a str> {
    let header_len = header.len();
    header.find(start).and_then(|mut begin| {
        begin += start.len();
        let end = header
            .get(begin..)?
            .find(|c: char| c.is_whitespace())
            .unwrap_or(header_len);
        header.get(begin..begin + end)
    })
}

pub(crate) fn parse_receive_header(header: &str) -> String {
    let header = header.replace(&['\r', '\n'][..], "");
    let mut hop_info = String::from("Hop: ");

    if let Some(from) = extract_address_from_receive_header(&header, "from ") {
        hop_info += &format!("From: {}; ", from.trim());
    }

    if let Some(by) = extract_address_from_receive_header(&header, "by ") {
        hop_info += &format!("By: {}; ", by.trim());
    }

    if let Ok(date) = dateparse(&header) {
        // In tests, use the UTC timezone so that the test is reproducible
        #[cfg(test)]
        let date_obj = chrono::Utc.timestamp_opt(date, 0).single();
        #[cfg(not(test))]
        let date_obj = Local.timestamp_opt(date, 0).single();

        hop_info += &format!(
            "Date: {}",
            date_obj.map_or_else(|| "?".to_string(), |x| x.to_rfc2822())
        );
    };

    hop_info
}

/// parses "receive"-headers
pub(crate) fn parse_receive_headers(headers: &Headers) -> String {
    headers
        .get_all_headers("Received")
        .iter()
        .rev()
        .filter_map(|header_map_item| from_utf8(header_map_item.get_value_raw()).ok())
        .map(parse_receive_header)
        .collect::<Vec<_>>()
        .join("\n")
}

/// If `collection` contains exactly one element, return this element.
/// Otherwise, return None.
pub(crate) fn single_value<T>(collection: impl IntoIterator<Item = T>) -> Option<T> {
    let mut iter = collection.into_iter();
    if let Some(value) = iter.next() {
        if iter.next().is_none() {
            return Some(value);
        }
    }
    None
}

/// Compressor/decompressor buffer size.
const BROTLI_BUFSZ: usize = 4096;

/// Compresses `buf` to `Vec` using `brotli`.
/// Note that it handles an empty `buf` as a special value that remains empty after compression,
/// otherwise brotli would add its metadata to it which is not nice because this function is used
/// for compression of strings stored in the db and empty strings are common there. This approach is
/// not strictly correct because nowhere in the brotli documentation is said that an empty buffer
/// can't be a result of compression of some input, but i think this will never break.
pub(crate) fn buf_compress(buf: &[u8]) -> Result<Vec<u8>> {
    if buf.is_empty() {
        return Ok(Vec::new());
    }
    // level 4 is 2x faster than level 6 (and 54x faster than 10, for comparison).
    // with the adaptiveness, we aim to not slow down processing
    // single large files too much, esp. on low-budget devices.
    // in tests (see #4129), this makes a difference, without compressing much worse.
    let q: u32 = if buf.len() > 1_000_000 { 4 } else { 6 };
    let lgwin: u32 = 22; // log2(LZ77 window size), it's the default for brotli CLI tool.
    let mut compressor = brotli::CompressorWriter::new(Vec::new(), BROTLI_BUFSZ, q, lgwin);
    compressor.write_all(buf)?;
    Ok(compressor.into_inner())
}

/// Decompresses `buf` to `Vec` using `brotli`.
/// See `buf_compress()` for why we don't pass an empty buffer to brotli decompressor.
pub(crate) fn buf_decompress(buf: &[u8]) -> Result<Vec<u8>> {
    if buf.is_empty() {
        return Ok(Vec::new());
    }
    let mut decompressor = brotli::DecompressorWriter::new(Vec::new(), BROTLI_BUFSZ);
    decompressor.write_all(buf)?;
    decompressor.flush()?;
    Ok(mem::take(decompressor.get_mut()))
}

const RTLO_CHARACTERS: [char; 5] = ['\u{202A}', '\u{202B}', '\u{202C}', '\u{202D}', '\u{202E}'];
/// This method strips all occurances of the RTLO Unicode character.
/// [Why is this needed](https://github.com/deltachat/deltachat-core-rust/issues/3479)?
pub(crate) fn strip_rtlo_characters(input_str: &str) -> String {
    input_str.replace(|char| RTLO_CHARACTERS.contains(&char), "")
}

#[cfg(test)]
mod tests {
    #![allow(clippy::indexing_slicing)]

    use super::*;
    use crate::{message::get_msg_info, receive_imf::receive_imf, test_utils::TestContext};

    #[test]
    fn test_parse_receive_headers() {
        // Test `parse_receive_headers()` with some more-or-less random emails from the test-data
        let raw = include_bytes!("../test-data/message/mail_with_cc.txt");
        let expected =
            "Hop: From: localhost; By: hq5.merlinux.eu; Date: Sat, 14 Sep 2019 17:00:22 +0000\n\
             Hop: From: hq5.merlinux.eu; By: hq5.merlinux.eu; Date: Sat, 14 Sep 2019 17:00:25 +0000";
        check_parse_receive_headers(raw, expected);

        let raw = include_bytes!("../test-data/message/wrong-html.eml");
        let expected =
            "Hop: From: oxbsltgw18.schlund.de; By: mrelayeu.kundenserver.de; Date: Thu, 06 Aug 2020 16:40:31 +0000\n\
             Hop: From: mout.kundenserver.de; By: dd37930.kasserver.com; Date: Thu, 06 Aug 2020 16:40:32 +0000";
        check_parse_receive_headers(raw, expected);

        let raw = include_bytes!("../test-data/message/posteo_ndn.eml");
        let expected =
            "Hop: By: mout01.posteo.de; Date: Tue, 09 Jun 2020 18:44:22 +0000\n\
             Hop: From: mout01.posteo.de; By: mx04.posteo.de; Date: Tue, 09 Jun 2020 18:44:22 +0000\n\
             Hop: From: mx04.posteo.de; By: mailin06.posteo.de; Date: Tue, 09 Jun 2020 18:44:23 +0000\n\
             Hop: From: mailin06.posteo.de; By: proxy02.posteo.de; Date: Tue, 09 Jun 2020 18:44:23 +0000\n\
             Hop: From: proxy02.posteo.de; By: proxy02.posteo.name; Date: Tue, 09 Jun 2020 18:44:23 +0000\n\
             Hop: From: proxy02.posteo.name; By: dovecot03.posteo.local; Date: Tue, 09 Jun 2020 18:44:24 +0000";
        check_parse_receive_headers(raw, expected);
    }

    fn check_parse_receive_headers(raw: &[u8], expected: &str) {
        let mail = mailparse::parse_mail(raw).unwrap();
        let hop_info = parse_receive_headers(&mail.get_headers());
        assert_eq!(hop_info, expected)
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_parse_receive_headers_integration() {
        let raw = include_bytes!("../test-data/message/mail_with_cc.txt");
        let expected = r"State: Fresh

hi

Message-ID: 2dfdbde7@example.org

Hop: From: localhost; By: hq5.merlinux.eu; Date: Sat, 14 Sep 2019 17:00:22 +0000
Hop: From: hq5.merlinux.eu; By: hq5.merlinux.eu; Date: Sat, 14 Sep 2019 17:00:25 +0000

DKIM Results: Passed=true, Works=true, Allow_Keychange=true";
        check_parse_receive_headers_integration(raw, expected).await;

        let raw = include_bytes!("../test-data/message/encrypted_with_received_headers.eml");
        let expected = "State: Fresh, Encrypted

Re: Message from alice@example.org

hi back\r\n\
\r\n\
-- \r\n\
Sent with my Delta Chat Messenger: https://delta.chat

Message-ID: Mr.adQpEwndXLH.LPDdlFVJ7wG@example.net

Hop: From: [127.0.0.1]; By: mail.example.org; Date: Mon, 27 Dec 2021 11:21:21 +0000
Hop: From: mout.example.org; By: hq5.example.org; Date: Mon, 27 Dec 2021 11:21:22 +0000
Hop: From: hq5.example.org; By: hq5.example.org; Date: Mon, 27 Dec 2021 11:21:22 +0000

DKIM Results: Passed=true, Works=true, Allow_Keychange=true";
        check_parse_receive_headers_integration(raw, expected).await;
    }

    async fn check_parse_receive_headers_integration(raw: &[u8], expected: &str) {
        let t = TestContext::new_alice().await;
        receive_imf(&t, raw, false).await.unwrap();
        let msg = t.get_last_msg().await;
        let msg_info = get_msg_info(&t, msg.id).await.unwrap();

        // Ignore the first rows of the msg_info because they contain a
        // received time that depends on the test time which makes it impossible to
        // compare with a static string
        let capped_result = &msg_info[msg_info.find("State").unwrap()..];
        assert_eq!(expected, capped_result);
    }

    #[test]
    fn test_rust_ftoa() {
        assert_eq!("1.22", format!("{}", 1.22));
    }

    #[test]
    fn test_truncate_1() {
        let s = "this is a little test string";
        assert_eq!(truncate(s, 16), "this is a [...]");
    }

    #[test]
    fn test_truncate_2() {
        assert_eq!(truncate("1234", 2), "1234");
    }

    #[test]
    fn test_truncate_3() {
        assert_eq!(truncate("1234567", 1), "1[...]");
    }

    #[test]
    fn test_truncate_4() {
        assert_eq!(truncate("123456", 4), "123456");
    }

    #[test]
    fn test_truncate_edge() {
        assert_eq!(truncate("", 4), "");

        assert_eq!(truncate("\n  hello \n world", 4), "\n  [...]");

        assert_eq!(truncate("𐠈0Aᝮa𫝀®!ꫛa¡0A𐢧00𐹠®A  丽ⷐએ", 1), "𐠈[...]");
        assert_eq!(truncate("𐠈0Aᝮa𫝀®!ꫛa¡0A𐢧00𐹠®A  丽ⷐએ", 0), "[...]");

        // 9 characters, so no truncation
        assert_eq!(truncate("𑒀ὐ￠🜀\u{1e01b}A a🟠", 6), "𑒀ὐ￠🜀\u{1e01b}A a🟠",);

        // 12 characters, truncation
        assert_eq!(
            truncate("𑒀ὐ￠🜀\u{1e01b}A a🟠bcd", 6),
            "𑒀ὐ￠🜀\u{1e01b}A[...]",
        );
    }

    mod truncate_by_lines {
        use super::*;

        #[test]
        fn test_just_text() {
            let s = "this is a little test string".to_string();
            assert_eq!(
                truncate_by_lines(s, 4, 6),
                ("this is a little test [...]".to_string(), true)
            );
        }

        #[test]
        fn test_with_linebreaks() {
            let s = "this\n is\n a little test string".to_string();
            assert_eq!(
                truncate_by_lines(s, 4, 6),
                ("this\n is\n a little [...]".to_string(), true)
            );
        }

        #[test]
        fn test_only_linebreaks() {
            let s = "\n\n\n\n\n\n\n".to_string();
            assert_eq!(
                truncate_by_lines(s, 4, 5),
                ("\n\n\n[...]".to_string(), true)
            );
        }

        #[test]
        fn limit_hits_end() {
            let s = "hello\n world !".to_string();
            assert_eq!(
                truncate_by_lines(s, 2, 8),
                ("hello\n world !".to_string(), false)
            );
        }

        #[test]
        fn test_edge() {
            assert_eq!(
                truncate_by_lines("".to_string(), 2, 4),
                ("".to_string(), false)
            );

            assert_eq!(
                truncate_by_lines("\n  hello \n world".to_string(), 2, 4),
                ("\n  [...]".to_string(), true)
            );
            assert_eq!(
                truncate_by_lines("𐠈0Aᝮa𫝀®!ꫛa¡0A𐢧00𐹠®A  丽ⷐએ".to_string(), 1, 2),
                ("𐠈0[...]".to_string(), true)
            );
            assert_eq!(
                truncate_by_lines("𐠈0Aᝮa𫝀®!ꫛa¡0A𐢧00𐹠®A  丽ⷐએ".to_string(), 1, 0),
                ("[...]".to_string(), true)
            );

            // 9 characters, so no truncation
            assert_eq!(
                truncate_by_lines("𑒀ὐ￠🜀\u{1e01b}A a🟠".to_string(), 1, 12),
                ("𑒀ὐ￠🜀\u{1e01b}A a🟠".to_string(), false),
            );

            // 12 characters, truncation
            assert_eq!(
                truncate_by_lines("𑒀ὐ￠🜀\u{1e01b}A a🟠bcd".to_string(), 1, 7),
                ("𑒀ὐ￠🜀\u{1e01b}A [...]".to_string(), true),
            );
        }
    }

    #[test]
    fn test_create_id() {
        let buf = create_id();
        assert_eq!(buf.len(), 11);
    }

    #[test]
    fn test_create_id_invalid_chars() {
        for _ in 1..1000 {
            let buf = create_id();
            assert!(!buf.contains('/')); // `/` must not be used to be URL-safe
            assert!(!buf.contains('.')); // `.` is used as a delimiter when extracting grpid from Message-ID
        }
    }

    #[test]
    fn test_extract_grpid_from_rfc724_mid() {
        // Should return None if we pass invalid mid
        let mid = "foobar";
        let grpid = extract_grpid_from_rfc724_mid(mid);
        assert_eq!(grpid, None);

        // Should return None if grpid has a length which is not 11 or 16
        let mid = "Gr.12345678.morerandom@domain.de";
        let grpid = extract_grpid_from_rfc724_mid(mid);
        assert_eq!(grpid, None);

        // Should return extracted grpid for grpid with length of 11
        let mid = "Gr.12345678901.morerandom@domain.de";
        let grpid = extract_grpid_from_rfc724_mid(mid);
        assert_eq!(grpid, Some("12345678901"));

        // Should return extracted grpid for grpid with length of 11
        let mid = "Gr.1234567890123456.morerandom@domain.de";
        let grpid = extract_grpid_from_rfc724_mid(mid);
        assert_eq!(grpid, Some("1234567890123456"));

        // Should return extracted grpid for grpid with length of 11
        let mid = "<Gr.12345678901.morerandom@domain.de>";
        let grpid = extract_grpid_from_rfc724_mid(mid);
        assert_eq!(grpid, Some("12345678901"));

        // Should return extracted grpid for grpid with length of 11
        let mid = "<Gr.1234567890123456.morerandom@domain.de>";
        let grpid = extract_grpid_from_rfc724_mid(mid);
        assert_eq!(grpid, Some("1234567890123456"));
    }

    #[test]
    fn test_create_outgoing_rfc724_mid() {
        // create a normal message-id
        let mid = create_outgoing_rfc724_mid(None, "foo@bar.de");
        assert!(mid.starts_with("Mr."));
        assert!(mid.ends_with("bar.de"));
        assert!(extract_grpid_from_rfc724_mid(mid.as_str()).is_none());

        // create a message-id containing a group-id
        let grpid = create_id();
        let mid = create_outgoing_rfc724_mid(Some(&grpid), "foo@bar.de");
        assert!(mid.starts_with("Gr."));
        assert!(mid.ends_with("bar.de"));
        assert_eq!(
            extract_grpid_from_rfc724_mid(mid.as_str()),
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
        assert_eq!(
            EmailAddress::new("user@localhost").unwrap(),
            EmailAddress {
                local: "user".into(),
                domain: "localhost".into()
            }
        );
        assert_eq!(EmailAddress::new("uuu").is_ok(), false);
        assert_eq!(EmailAddress::new("dd.tt").is_ok(), false);
        assert!(EmailAddress::new("tt.dd@uu").is_ok());
        assert!(EmailAddress::new("u@d").is_ok());
        assert!(EmailAddress::new("u@d.").is_ok());
        assert!(EmailAddress::new("u@d.t").is_ok());
        assert_eq!(
            EmailAddress::new("u@d.tt").unwrap(),
            EmailAddress {
                local: "u".into(),
                domain: "d.tt".into(),
            }
        );
        assert!(EmailAddress::new("u@tt").is_ok());
        assert_eq!(EmailAddress::new("@d.tt").is_ok(), false);
    }

    use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
    use proptest::prelude::*;

    use crate::chatlist::Chatlist;
    use crate::{chat, test_utils};

    proptest! {
        #[test]
        fn test_truncate(
            buf: String,
            approx_chars in 0..100usize
        ) {
            let res = truncate(&buf, approx_chars);
            let el_len = 5;
            let l = res.chars().count();
            assert!(
                l <= approx_chars + el_len,
                "buf: '{}' - res: '{}' - len {}, approx {}",
                &buf, &res, res.len(), approx_chars
            );

            if buf.chars().count() > approx_chars + el_len {
                let l = res.len();
                assert_eq!(&res[l-5..l], "[...]", "missing ellipsis in {}", &res);
            }
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_file_handling() {
        let t = TestContext::new().await;
        let context = &t;
        macro_rules! file_exist {
            ($ctx:expr, $fname:expr) => {
                $ctx.get_blobdir()
                    .join(Path::new($fname).file_name().unwrap())
                    .exists()
            };
        }

        assert!(delete_file(context, "$BLOBDIR/lkqwjelqkwlje")
            .await
            .is_err());
        assert!(write_file(context, "$BLOBDIR/foobar", b"content")
            .await
            .is_ok());
        assert!(file_exist!(context, "$BLOBDIR/foobar"));
        assert!(!file_exist!(context, "$BLOBDIR/foobarx"));
        assert_eq!(get_filebytes(context, "$BLOBDIR/foobar").await.unwrap(), 7);

        let abs_path = context
            .get_blobdir()
            .join("foobar")
            .to_string_lossy()
            .to_string();

        assert!(file_exist!(context, &abs_path));

        assert!(delete_file(context, "$BLOBDIR/foobar").await.is_ok());
        assert!(create_folder(context, "$BLOBDIR/foobar-folder")
            .await
            .is_ok());
        assert!(file_exist!(context, "$BLOBDIR/foobar-folder"));
        assert!(delete_file(context, "$BLOBDIR/foobar-folder")
            .await
            .is_err());

        let fn0 = "$BLOBDIR/data.data";
        assert!(write_file(context, &fn0, b"content").await.is_ok());

        assert!(delete_file(context, &fn0).await.is_ok());
        assert!(!file_exist!(context, &fn0));
    }

    #[test]
    fn test_duration_to_str() {
        assert_eq!(duration_to_str(Duration::from_secs(0)), "0h 0m 0s");
        assert_eq!(duration_to_str(Duration::from_secs(59)), "0h 0m 59s");
        assert_eq!(duration_to_str(Duration::from_secs(60)), "0h 1m 0s");
        assert_eq!(duration_to_str(Duration::from_secs(61)), "0h 1m 1s");
        assert_eq!(duration_to_str(Duration::from_secs(59 * 60)), "0h 59m 0s");
        assert_eq!(
            duration_to_str(Duration::from_secs(59 * 60 + 59)),
            "0h 59m 59s"
        );
        assert_eq!(
            duration_to_str(Duration::from_secs(59 * 60 + 60)),
            "1h 0m 0s"
        );
        assert_eq!(
            duration_to_str(Duration::from_secs(2 * 60 * 60 + 59 * 60 + 59)),
            "2h 59m 59s"
        );
        assert_eq!(
            duration_to_str(Duration::from_secs(2 * 60 * 60 + 59 * 60 + 60)),
            "3h 0m 0s"
        );
        assert_eq!(
            duration_to_str(Duration::from_secs(3 * 60 * 60 + 59)),
            "3h 0m 59s"
        );
        assert_eq!(
            duration_to_str(Duration::from_secs(3 * 60 * 60 + 60)),
            "3h 1m 0s"
        );
    }

    #[test]
    fn test_get_filemeta() {
        let (w, h) = get_filemeta(test_utils::AVATAR_900x900_BYTES).unwrap();
        assert_eq!(w, 900);
        assert_eq!(h, 900);

        let data = include_bytes!("../test-data/image/avatar1000x1000.jpg");
        let (w, h) = get_filemeta(data).unwrap();
        assert_eq!(w, 1000);
        assert_eq!(h, 1000);

        let data = include_bytes!("../test-data/image/image100x50.gif");
        let (w, h) = get_filemeta(data).unwrap();
        assert_eq!(w, 100);
        assert_eq!(h, 50);
    }

    #[test]
    fn test_improve_single_line_input() {
        assert_eq!(improve_single_line_input("Hi\naiae "), "Hi aiae");
        assert_eq!(improve_single_line_input("\r\nahte\n\r"), "ahte");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_maybe_warn_on_bad_time() {
        let t = TestContext::new().await;
        let timestamp_now = time();
        let timestamp_future = timestamp_now + 60 * 60 * 24 * 7;
        let timestamp_past = NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2020, 9, 1).unwrap(),
            NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        )
        .timestamp_millis()
            / 1_000;

        // a correct time must not add a device message
        maybe_warn_on_bad_time(&t, timestamp_now, get_release_timestamp()).await;
        let chats = Chatlist::try_load(&t, 0, None, None).await.unwrap();
        assert_eq!(chats.len(), 0);

        // we cannot find out if a date in the future is wrong - a device message is not added
        maybe_warn_on_bad_time(&t, timestamp_future, get_release_timestamp()).await;
        let chats = Chatlist::try_load(&t, 0, None, None).await.unwrap();
        assert_eq!(chats.len(), 0);

        // a date in the past must add a device message
        maybe_warn_on_bad_time(&t, timestamp_past, get_release_timestamp()).await;
        let chats = Chatlist::try_load(&t, 0, None, None).await.unwrap();
        assert_eq!(chats.len(), 1);
        let device_chat_id = chats.get_chat_id(0).unwrap();
        let msgs = chat::get_chat_msgs(&t, device_chat_id).await.unwrap();
        assert_eq!(msgs.len(), 1);

        // the message should be added only once a day - test that an hour later and nearly a day later
        maybe_warn_on_bad_time(&t, timestamp_past + 60 * 60, get_release_timestamp()).await;
        let msgs = chat::get_chat_msgs(&t, device_chat_id).await.unwrap();
        assert_eq!(msgs.len(), 1);

        maybe_warn_on_bad_time(
            &t,
            timestamp_past + 60 * 60 * 24 - 1,
            get_release_timestamp(),
        )
        .await;
        let msgs = chat::get_chat_msgs(&t, device_chat_id).await.unwrap();
        assert_eq!(msgs.len(), 1);

        // next day, there should be another device message
        maybe_warn_on_bad_time(&t, timestamp_past + 60 * 60 * 24, get_release_timestamp()).await;
        let chats = Chatlist::try_load(&t, 0, None, None).await.unwrap();
        assert_eq!(chats.len(), 1);
        assert_eq!(device_chat_id, chats.get_chat_id(0).unwrap());
        let msgs = chat::get_chat_msgs(&t, device_chat_id).await.unwrap();
        assert_eq!(msgs.len(), 2);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_maybe_warn_on_outdated() {
        let t = TestContext::new().await;
        let timestamp_now: i64 = time();

        // in about 6 months, the app should not be outdated
        // (if this fails, provider-db is not updated since 6 months)
        maybe_warn_on_outdated(
            &t,
            timestamp_now + 180 * 24 * 60 * 60,
            get_release_timestamp(),
        )
        .await;
        let chats = Chatlist::try_load(&t, 0, None, None).await.unwrap();
        assert_eq!(chats.len(), 0);

        // in 1 year, the app should be considered as outdated
        maybe_warn_on_outdated(
            &t,
            timestamp_now + 365 * 24 * 60 * 60,
            get_release_timestamp(),
        )
        .await;
        let chats = Chatlist::try_load(&t, 0, None, None).await.unwrap();
        assert_eq!(chats.len(), 1);
        let device_chat_id = chats.get_chat_id(0).unwrap();
        let msgs = chat::get_chat_msgs(&t, device_chat_id).await.unwrap();
        assert_eq!(msgs.len(), 1);

        // do not repeat the warning every day ...
        // (we test that for the 2 subsequent days, this may be the next month, so the result should be 1 or 2 device message)
        maybe_warn_on_outdated(
            &t,
            timestamp_now + (365 + 1) * 24 * 60 * 60,
            get_release_timestamp(),
        )
        .await;
        maybe_warn_on_outdated(
            &t,
            timestamp_now + (365 + 2) * 24 * 60 * 60,
            get_release_timestamp(),
        )
        .await;
        let chats = Chatlist::try_load(&t, 0, None, None).await.unwrap();
        assert_eq!(chats.len(), 1);
        let device_chat_id = chats.get_chat_id(0).unwrap();
        let msgs = chat::get_chat_msgs(&t, device_chat_id).await.unwrap();
        let test_len = msgs.len();
        assert!(test_len == 1 || test_len == 2);

        // ... but every month
        // (forward generous 33 days to avoid being in the same month as in the previous check)
        maybe_warn_on_outdated(
            &t,
            timestamp_now + (365 + 33) * 24 * 60 * 60,
            get_release_timestamp(),
        )
        .await;
        let chats = Chatlist::try_load(&t, 0, None, None).await.unwrap();
        assert_eq!(chats.len(), 1);
        let device_chat_id = chats.get_chat_id(0).unwrap();
        let msgs = chat::get_chat_msgs(&t, device_chat_id).await.unwrap();
        assert_eq!(msgs.len(), test_len + 1);
    }

    #[test]
    fn test_get_release_timestamp() {
        let timestamp_past = NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2020, 9, 9).unwrap(),
            NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        )
        .timestamp_millis()
            / 1_000;
        assert!(get_release_timestamp() <= time());
        assert!(get_release_timestamp() > timestamp_past);
    }

    #[test]
    fn test_remove_subject_prefix() {
        assert_eq!(remove_subject_prefix("Subject"), "Subject");
        assert_eq!(
            remove_subject_prefix("Chat: Re: Subject"),
            "Chat: Re: Subject"
        );
        assert_eq!(remove_subject_prefix("Re: Subject"), "Subject");
        assert_eq!(remove_subject_prefix("Fwd: Subject"), "Subject");
        assert_eq!(remove_subject_prefix("Fw: Subject"), "Subject");
    }
}
