//! Some tools and enhancements to the used libraries, there should be
//! no references to Context and other "larger" entities here.

#![allow(missing_docs)]

use std::borrow::Cow;
use std::io::{Cursor, Write};
use std::mem;
use std::ops::{AddAssign, Deref};
use std::path::{Path, PathBuf};
use std::str::from_utf8;
// If a time value doesn't need to be sent to another host, saved to the db or otherwise used across
// program restarts, a monotonically nondecreasing clock (`Instant`) should be used. But as
// `Instant` may use `libc::clock_gettime(CLOCK_MONOTONIC)`, e.g. on Android, and does not advance
// while being in deep sleep mode, we use `SystemTime` instead, but add an alias for it to document
// why `Instant` isn't used in those places. Also this can help to switch to another clock impl if
// we find any.
use std::time::Duration;
pub use std::time::SystemTime as Time;
#[cfg(not(test))]
pub use std::time::SystemTime;

use anyhow::{bail, ensure, Context as _, Result};
use base64::Engine as _;
use chrono::{Local, NaiveDateTime, NaiveTime, TimeZone};
use deltachat_contact_tools::EmailAddress;
#[cfg(test)]
pub use deltachat_time::SystemTimeTools as SystemTime;
use futures::TryStreamExt;
use mailparse::dateparse;
use mailparse::headers::Headers;
use mailparse::MailHeaderMap;
use num_traits::PrimInt;
use rand::{thread_rng, Rng};
use tokio::{fs, io};
use url::Url;

use crate::chat::{add_device_msg, add_device_msg_with_importance};
use crate::config::Config;
use crate::constants::{self, DC_ELLIPSIS, DC_OUTDATED_WARNING_DAYS};
use crate::context::Context;
use crate::events::EventType;
use crate::message::{Message, Viewtype};
use crate::stock_str;

/// Shortens a string to a specified length and adds "[...]" to the
/// end of the shortened string.
pub(crate) fn truncate(buf: &str, approx_chars: usize) -> Cow<str> {
    let count = buf.chars().count();
    if count <= approx_chars + DC_ELLIPSIS.len() {
        return Cow::Borrowed(buf);
    }
    let end_pos = buf
        .char_indices()
        .nth(approx_chars)
        .map(|(n, _)| n)
        .unwrap_or_default();

    if let Some(index) = buf.get(..end_pos).and_then(|s| s.rfind([' ', '\n'])) {
        Cow::Owned(format!(
            "{}{}",
            &buf.get(..=index).unwrap_or_default(),
            DC_ELLIPSIS
        ))
    } else {
        Cow::Owned(format!(
            "{}{}",
            &buf.get(..end_pos).unwrap_or_default(),
            DC_ELLIPSIS
        ))
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
                if let Some(index) = buffer.rfind([' ', '\n']) {
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

/// Shortens a message text if necessary according to the configuration. Adds "[...]" to the end of
/// the shortened text.
///
/// Returns the resulting text and a bool telling whether a truncation was done.
pub(crate) async fn truncate_msg_text(context: &Context, text: String) -> Result<(String, bool)> {
    if context.get_config_bool(Config::Bot).await? {
        return Ok((text, false));
    }
    // Truncate text if it has too many lines
    Ok(truncate_by_lines(
        text,
        constants::DC_DESIRED_TEXT_LINES,
        constants::DC_DESIRED_TEXT_LINE_LEN,
    ))
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
    .and_utc()
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
        msg.text = stock_str::bad_time_msg_body(
            context,
            &Local.timestamp_opt(now, 0).single().map_or_else(
                || "YY-MM-DD hh:mm:ss".to_string(),
                |ts| ts.format("%Y-%m-%d %H:%M:%S").to_string(),
            ),
        )
        .await;
        if let Some(timestamp) = chrono::DateTime::<chrono::Utc>::from_timestamp(now, 0) {
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
        let mut msg = Message::new_text(stock_str::update_reminder_msg_body(context).await);
        if let Some(timestamp) = chrono::DateTime::<chrono::Utc>::from_timestamp(now, 0) {
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

/// Generate an unique ID.
///
/// The generated ID should be short but unique:
/// - short, because it used in Message-ID and Chat-Group-ID headers and in QR codes
/// - unique as two IDs generated on two devices should not be the same
///
/// IDs generated by this function have 144 bits of entropy
/// and are returned as 24 Base64 characters, each containing 6 bits of entropy.
/// 144 is chosen because it is sufficiently secure
/// (larger than AES-128 keys used for message encryption)
/// and divides both by 8 (byte size) and 6 (number of bits in a single Base64 character).
pub(crate) fn create_id() -> String {
    // ThreadRng implements CryptoRng trait and is supposed to be cryptographically secure.
    let mut rng = thread_rng();

    // Generate 144 random bits.
    let mut arr = [0u8; 18];
    rng.fill(&mut arr[..]);

    base64::engine::general_purpose::URL_SAFE.encode(arr)
}

/// Returns true if given string is a valid ID.
///
/// All IDs generated with `create_id()` should be considered valid.
pub(crate) fn validate_id(s: &str) -> bool {
    let alphabet = base64::alphabet::URL_SAFE.as_str();
    s.chars().all(|c| alphabet.contains(c)) && s.len() > 10 && s.len() <= 32
}

/// Function generates a Message-ID that can be used for a new outgoing message.
/// - this function is called for all outgoing messages.
/// - the message ID should be globally unique
/// - do not add a counter or any private data as this leaks information unnecessarily
pub(crate) fn create_outgoing_rfc724_mid() -> String {
    format!("{}@localhost", create_id())
}

// the returned suffix is lower-case
pub fn get_filesuffix_lc(path_filename: &str) -> Option<String> {
    Path::new(path_filename)
        .extension()
        .map(|p| p.to_string_lossy().to_lowercase())
}

/// Returns the `(width, height)` of the given image buffer.
pub fn get_filemeta(buf: &[u8]) -> Result<(u32, u32)> {
    let image = image::ImageReader::new(Cursor::new(buf)).with_guessed_format()?;
    let dimensions = image.into_dimensions()?;
    Ok(dimensions)
}

/// Expand paths relative to $BLOBDIR into absolute paths.
///
/// If `path` starts with "$BLOBDIR", replaces it with the blobdir path.
/// Otherwise, returns path as is.
pub(crate) fn get_abs_path(context: &Context, path: &Path) -> PathBuf {
    if let Ok(p) = path.strip_prefix("$BLOBDIR") {
        context.get_blobdir().join(p)
    } else {
        path.into()
    }
}

pub(crate) async fn get_filebytes(context: &Context, path: impl AsRef<Path>) -> Result<u64> {
    let path_abs = get_abs_path(context, path.as_ref());
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

/// Create a safe name based on a messy input string.
///
/// The safe name will be a valid filename on Unix and Windows and
/// not contain any path separators.  The input can contain path
/// segments separated by either Unix or Windows path separators,
/// the rightmost non-empty segment will be used as name,
/// sanitised for special characters.
pub(crate) fn sanitize_filename(mut name: &str) -> String {
    for part in name.rsplit('/') {
        if !part.is_empty() {
            name = part;
            break;
        }
    }
    for part in name.rsplit('\\') {
        if !part.is_empty() {
            name = part;
            break;
        }
    }

    let opts = sanitize_filename::Options {
        truncate: true,
        windows: true,
        replacement: "",
    };
    let name = sanitize_filename::sanitize_with_options(name, opts);

    if name.starts_with('.') || name.is_empty() {
        format!("file{name}")
    } else {
        name
    }
}

/// A guard which will remove the path when dropped.
///
/// It implements [`Deref`] so it can be used as a `&Path`.
#[derive(Debug)]
pub(crate) struct TempPathGuard {
    path: PathBuf,
}

impl TempPathGuard {
    pub(crate) fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl Drop for TempPathGuard {
    fn drop(&mut self) {
        let path = self.path.clone();
        std::fs::remove_file(path).ok();
    }
}

impl Deref for TempPathGuard {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        &self.path
    }
}

impl AsRef<Path> for TempPathGuard {
    fn as_ref(&self) -> &Path {
        self
    }
}

pub(crate) async fn create_folder(
    context: &Context,
    path: impl AsRef<Path>,
) -> Result<(), io::Error> {
    let path_abs = get_abs_path(context, path.as_ref());
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
    let path_abs = get_abs_path(context, path.as_ref());
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
    let path_abs = get_abs_path(context, path.as_ref());

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
    let path_abs = get_abs_path(context, path.as_ref());

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

pub fn open_file_std(context: &Context, path: impl AsRef<Path>) -> Result<std::fs::File> {
    let path_abs = get_abs_path(context, path.as_ref());

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

pub(crate) fn time_elapsed(time: &Time) -> Duration {
    time.elapsed().unwrap_or_default()
}

/// Struct containing all mailto information
#[derive(Debug, Default, Eq, PartialEq)]
pub struct MailTo {
    pub to: Vec<EmailAddress>,
    pub subject: Option<String>,
    pub body: Option<String>,
}

/// Parse mailto urls
pub fn parse_mailto(mailto_url: &str) -> Option<MailTo> {
    if let Ok(url) = Url::parse(mailto_url) {
        if url.scheme() == "mailto" {
            let mut mailto: MailTo = Default::default();
            // Extract the email address
            url.path().split(',').for_each(|email| {
                if let Ok(email) = EmailAddress::new(email) {
                    mailto.to.push(email);
                }
            });

            // Extract query parameters
            for (key, value) in url.query_pairs() {
                if key == "subject" {
                    mailto.subject = Some(value.to_string());
                } else if key == "body" {
                    mailto.body = Some(value.to_string());
                }
            }
            Some(mailto)
        } else {
            None
        }
    } else {
        None
    }
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

/// Increments `*t` and checks that it equals to `expected` after that.
pub(crate) fn inc_and_check<T: PrimInt + AddAssign + std::fmt::Debug>(
    t: &mut T,
    expected: T,
) -> Result<()> {
    *t += T::one();
    ensure!(*t == expected, "Incremented value != {expected:?}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use proptest::prelude::*;

    use super::*;
    use crate::chatlist::Chatlist;
    use crate::{chat, test_utils};
    use crate::{receive_imf::receive_imf, test_utils::TestContext};

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
            "Hop: From: oxbsltgw18.schlund.de; By: mrelayeu.kundenserver.de; Date: Thu, 6 Aug 2020 16:40:31 +0000\n\
             Hop: From: mout.kundenserver.de; By: dd37930.kasserver.com; Date: Thu, 6 Aug 2020 16:40:32 +0000";
        check_parse_receive_headers(raw, expected);

        let raw = include_bytes!("../test-data/message/posteo_ndn.eml");
        let expected =
            "Hop: By: mout01.posteo.de; Date: Tue, 9 Jun 2020 18:44:22 +0000\n\
             Hop: From: mout01.posteo.de; By: mx04.posteo.de; Date: Tue, 9 Jun 2020 18:44:22 +0000\n\
             Hop: From: mx04.posteo.de; By: mailin06.posteo.de; Date: Tue, 9 Jun 2020 18:44:23 +0000\n\
             Hop: From: mailin06.posteo.de; By: proxy02.posteo.de; Date: Tue, 9 Jun 2020 18:44:23 +0000\n\
             Hop: From: proxy02.posteo.de; By: proxy02.posteo.name; Date: Tue, 9 Jun 2020 18:44:23 +0000\n\
             Hop: From: proxy02.posteo.name; By: dovecot03.posteo.local; Date: Tue, 9 Jun 2020 18:44:24 +0000";
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

DKIM Results: Passed=true";
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

DKIM Results: Passed=true";
        check_parse_receive_headers_integration(raw, expected).await;
    }

    async fn check_parse_receive_headers_integration(raw: &[u8], expected: &str) {
        let t = TestContext::new_alice().await;
        receive_imf(&t, raw, false).await.unwrap();
        let msg = t.get_last_msg().await;
        let msg_info = msg.id.get_info(&t).await.unwrap();

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
        assert_eq!(buf.len(), 24);
    }

    #[test]
    fn test_validate_id() {
        for _ in 0..10 {
            assert!(validate_id(&create_id()));
        }

        assert_eq!(validate_id("aaaaaaaaaaaa"), true);
        assert_eq!(validate_id("aa-aa_aaaXaa"), true);

        // ID cannot contain whitespace.
        assert_eq!(validate_id("aaaaa aaaaaa"), false);
        assert_eq!(validate_id("aaaaa\naaaaaa"), false);

        // ID cannot contain "/", "+".
        assert_eq!(validate_id("aaaaa/aaaaaa"), false);
        assert_eq!(validate_id("aaaaaaaa+aaa"), false);

        // Too long ID.
        assert_eq!(validate_id("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"), false);
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
    fn test_create_outgoing_rfc724_mid() {
        let mid = create_outgoing_rfc724_mid();
        assert_eq!(mid.len(), 34);
        assert!(mid.ends_with("@localhost"));
    }

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

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_maybe_warn_on_bad_time() {
        let t = TestContext::new().await;
        let timestamp_now = time();
        let timestamp_future = timestamp_now + 60 * 60 * 24 * 7;
        let timestamp_past = NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2020, 9, 1).unwrap(),
            NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        )
        .and_utc()
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
        .and_utc()
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

    #[test]
    fn test_parse_mailto() {
        let mailto_url = "mailto:someone@example.com";
        let reps = parse_mailto(mailto_url);
        assert_eq!(
            Some(MailTo {
                to: vec![EmailAddress {
                    local: "someone".to_string(),
                    domain: "example.com".to_string()
                }],
                subject: None,
                body: None
            }),
            reps
        );

        let mailto_url = "mailto:someone@example.com?subject=Hello%20World";
        let reps = parse_mailto(mailto_url);
        assert_eq!(
            Some(MailTo {
                to: vec![EmailAddress {
                    local: "someone".to_string(),
                    domain: "example.com".to_string()
                }],
                subject: Some("Hello World".to_string()),
                body: None
            }),
            reps
        );

        let mailto_url = "mailto:someone@example.com,someoneelse@example.com?subject=Hello%20World&body=This%20is%20a%20test";
        let reps = parse_mailto(mailto_url);
        assert_eq!(
            Some(MailTo {
                to: vec![
                    EmailAddress {
                        local: "someone".to_string(),
                        domain: "example.com".to_string()
                    },
                    EmailAddress {
                        local: "someoneelse".to_string(),
                        domain: "example.com".to_string()
                    }
                ],
                subject: Some("Hello World".to_string()),
                body: Some("This is a test".to_string())
            }),
            reps
        );
    }
}
