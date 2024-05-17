//! Contact-related tools, like parsing vcards and sanitizing name and address

#![forbid(unsafe_code)]
#![warn(
    unused,
    clippy::correctness,
    missing_debug_implementations,
    missing_docs,
    clippy::all,
    clippy::wildcard_imports,
    clippy::needless_borrow,
    clippy::cast_lossless,
    clippy::unused_async,
    clippy::explicit_iter_loop,
    clippy::explicit_into_iter_loop,
    clippy::cloned_instead_of_copied
)]
#![cfg_attr(not(test), warn(clippy::indexing_slicing))]
#![allow(
    clippy::match_bool,
    clippy::mixed_read_write_in_expression,
    clippy::bool_assert_comparison,
    clippy::manual_split_once,
    clippy::format_push_string,
    clippy::bool_to_int_with_if
)]

use std::fmt;
use std::ops::Deref;

use anyhow::bail;
use anyhow::Context as _;
use anyhow::Result;
use chrono::{DateTime, NaiveDateTime};
use once_cell::sync::Lazy;
use regex::Regex;

// TODOs to clean up:
// - Check if sanitizing is done correctly everywhere
// - Apply lints everywhere (https://doc.rust-lang.org/cargo/reference/workspaces.html#the-lints-table)

#[derive(Debug)]
/// A Contact, as represented in a VCard.
pub struct VcardContact {
    /// The email address, vcard property `email`
    pub addr: String,
    /// This must be the name authorized by the contact itself, not a locally given name. Vcard
    /// property `fn`. Can be empty, one should use `display_name()` to obtain the display name.
    pub authname: String,
    /// The contact's public PGP key in Base64, vcard property `key`
    pub key: Option<String>,
    /// The contact's profile image (=avatar) in Base64, vcard property `photo`
    pub profile_image: Option<String>,
    /// The timestamp when the vcard was created / last updated, vcard property `rev`
    pub timestamp: Result<i64>,
}

impl VcardContact {
    /// Returns the contact's display name.
    pub fn display_name(&self) -> &str {
        match self.authname.is_empty() {
            false => &self.authname,
            true => &self.addr,
        }
    }
}

/// Returns a vCard containing given contacts.
///
/// Calling [`parse_vcard()`] on the returned result is a reverse operation.
pub fn make_vcard(contacts: &[VcardContact]) -> String {
    fn format_timestamp(c: &VcardContact) -> Option<String> {
        let timestamp = *c.timestamp.as_ref().ok()?;
        let datetime = DateTime::from_timestamp(timestamp, 0)?;
        Some(datetime.format("%Y%m%dT%H%M%SZ").to_string())
    }

    let mut res = "".to_string();
    for c in contacts {
        let addr = &c.addr;
        let display_name = c.display_name();
        res += &format!(
            "BEGIN:VCARD\n\
             VERSION:4.0\n\
             EMAIL:{addr}\n\
             FN:{display_name}\n"
        );
        if let Some(key) = &c.key {
            res += &format!("KEY:data:application/pgp-keys;base64,{key}\n");
        }
        if let Some(profile_image) = &c.profile_image {
            res += &format!("PHOTO:data:image/jpeg;base64,{profile_image}\n");
        }
        if let Some(timestamp) = format_timestamp(c) {
            res += &format!("REV:{timestamp}\n");
        }
        res += "END:VCARD\n";
    }
    res
}

/// Parses `VcardContact`s from a given `&str`.
pub fn parse_vcard(vcard: &str) -> Vec<VcardContact> {
    fn remove_prefix<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
        let start_of_s = s.get(..prefix.len())?;

        if start_of_s.eq_ignore_ascii_case(prefix) {
            s.get(prefix.len()..)
        } else {
            None
        }
    }
    fn vcard_property<'a>(s: &'a str, property: &str) -> Option<&'a str> {
        let remainder = remove_prefix(s, property)?;
        // If `s` is `EMAIL;TYPE=work:alice@example.com` and `property` is `EMAIL`,
        // then `remainder` is now `;TYPE=work:alice@example.com`

        // TODO this doesn't handle the case where there are quotes around a colon
        let (params, value) = remainder.split_once(':')?;
        // In the example from above, `params` is now `;TYPE=work`
        // and `value` is now `alice@example.com`

        if params
            .chars()
            .next()
            .filter(|c| !c.is_ascii_punctuation() || *c == '_')
            .is_some()
        {
            // `s` started with `property`, but the next character after it was not punctuation,
            // so this line's property is actually something else
            return None;
        }
        Some(value)
    }
    fn parse_datetime(datetime: &str) -> Result<i64> {
        // According to https://www.rfc-editor.org/rfc/rfc6350#section-4.3.5, the timestamp
        // is in ISO.8601.2004 format. DateTime::parse_from_rfc3339() apparently parses
        // ISO.8601, but fails to parse any of the examples given.
        // So, instead just parse using a format string.

        // Parses 19961022T140000Z, 19961022T140000-05, or 19961022T140000-0500.
        let timestamp = match DateTime::parse_from_str(datetime, "%Y%m%dT%H%M%S%#z") {
            Ok(datetime) => datetime.timestamp(),
            // Parses 19961022T140000.
            Err(e) => match NaiveDateTime::parse_from_str(datetime, "%Y%m%dT%H%M%S") {
                Ok(datetime) => datetime
                    .and_local_timezone(chrono::offset::Local)
                    .single()
                    .context("Could not apply local timezone to parsed date and time")?
                    .timestamp(),
                Err(_) => return Err(e.into()),
            },
        };
        Ok(timestamp)
    }

    // Remove line folding, see https://datatracker.ietf.org/doc/html/rfc6350#section-3.2
    static NEWLINE_AND_SPACE_OR_TAB: Lazy<Regex> = Lazy::new(|| Regex::new("\r?\n[\t ]").unwrap());
    let unfolded_lines = NEWLINE_AND_SPACE_OR_TAB.replace_all(vcard, "");

    let mut lines = unfolded_lines.lines().peekable();
    let mut contacts = Vec::new();

    while lines.peek().is_some() {
        // Skip to the start of the vcard:
        for line in lines.by_ref() {
            if line.eq_ignore_ascii_case("BEGIN:VCARD") {
                break;
            }
        }

        let mut display_name = None;
        let mut addr = None;
        let mut key = None;
        let mut photo = None;
        let mut datetime = None;

        for line in lines.by_ref() {
            if let Some(email) = vcard_property(line, "email") {
                addr.get_or_insert(email);
            } else if let Some(name) = vcard_property(line, "fn") {
                display_name.get_or_insert(name);
            } else if let Some(k) = remove_prefix(line, "KEY;PGP;ENCODING=BASE64:")
                .or_else(|| remove_prefix(line, "KEY;TYPE=PGP;ENCODING=b:"))
                .or_else(|| remove_prefix(line, "KEY:data:application/pgp-keys;base64,"))
            {
                key.get_or_insert(k);
            } else if let Some(p) = remove_prefix(line, "PHOTO;JPEG;ENCODING=BASE64:")
                .or_else(|| remove_prefix(line, "PHOTO;ENCODING=BASE64;JPEG:"))
                .or_else(|| remove_prefix(line, "PHOTO;TYPE=JPEG;ENCODING=b:"))
                .or_else(|| remove_prefix(line, "PHOTO;ENCODING=b;TYPE=JPEG:"))
                .or_else(|| remove_prefix(line, "PHOTO;ENCODING=BASE64;TYPE=JPEG:"))
                .or_else(|| remove_prefix(line, "PHOTO;TYPE=JPEG;ENCODING=BASE64:"))
                .or_else(|| remove_prefix(line, "PHOTO:data:image/jpeg;base64,"))
            {
                photo.get_or_insert(p);
            } else if let Some(rev) = vcard_property(line, "rev") {
                datetime.get_or_insert(rev);
            } else if line.eq_ignore_ascii_case("END:VCARD") {
                break;
            }
        }

        let (authname, addr) =
            sanitize_name_and_addr(display_name.unwrap_or(""), addr.unwrap_or(""));

        contacts.push(VcardContact {
            authname,
            addr,
            key: key.map(|s| s.to_string()),
            profile_image: photo.map(|s| s.to_string()),
            timestamp: datetime
                .context("No timestamp in vcard")
                .and_then(parse_datetime),
        });
    }

    contacts
}

/// Valid contact address.
#[derive(Debug, Clone)]
pub struct ContactAddress(String);

impl Deref for ContactAddress {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<str> for ContactAddress {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ContactAddress {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl ContactAddress {
    /// Constructs a new contact address from string,
    /// normalizing and validating it.
    pub fn new(s: &str) -> Result<Self> {
        let addr = addr_normalize(s);
        if !may_be_valid_addr(&addr) {
            bail!("invalid address {:?}", s);
        }
        Ok(Self(addr.to_string()))
    }
}

/// Allow converting [`ContactAddress`] to an SQLite type.
impl rusqlite::types::ToSql for ContactAddress {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput> {
        let val = rusqlite::types::Value::Text(self.0.to_string());
        let out = rusqlite::types::ToSqlOutput::Owned(val);
        Ok(out)
    }
}

/// Make the name and address
pub fn sanitize_name_and_addr(name: &str, addr: &str) -> (String, String) {
    static ADDR_WITH_NAME_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new("(.*)<(.*)>").unwrap());
    let (name, addr) = if let Some(captures) = ADDR_WITH_NAME_REGEX.captures(addr.as_ref()) {
        (
            if name.is_empty() {
                strip_rtlo_characters(captures.get(1).map_or("", |m| m.as_str()))
            } else {
                strip_rtlo_characters(name)
            },
            captures
                .get(2)
                .map_or("".to_string(), |m| m.as_str().to_string()),
        )
    } else {
        (
            strip_rtlo_characters(&normalize_name(name)),
            addr.to_string(),
        )
    };
    let mut name = normalize_name(&name);

    // If the 'display name' is just the address, remove it:
    // Otherwise, the contact would sometimes be shown as "alice@example.com (alice@example.com)" (see `get_name_n_addr()`).
    // If the display name is empty, DC will just show the address when it needs a display name.
    if name == addr {
        name = "".to_string();
    }

    (name, addr)
}

/// Normalize a name.
///
/// - Remove quotes (come from some bad MUA implementations)
/// - Trims the resulting string
///
/// Typically, this function is not needed as it is called implicitly by `Contact::add_address_book`.
pub fn normalize_name(full_name: &str) -> String {
    let full_name = full_name.trim();
    if full_name.is_empty() {
        return full_name.into();
    }

    match full_name.as_bytes() {
        [b'\'', .., b'\''] | [b'\"', .., b'\"'] | [b'<', .., b'>'] => full_name
            .get(1..full_name.len() - 1)
            .map_or("".to_string(), |s| s.trim().to_string()),
        _ => full_name.to_string(),
    }
}

const RTLO_CHARACTERS: [char; 5] = ['\u{202A}', '\u{202B}', '\u{202C}', '\u{202D}', '\u{202E}'];
/// This method strips all occurrences of the RTLO Unicode character.
/// [Why is this needed](https://github.com/deltachat/deltachat-core-rust/issues/3479)?
pub fn strip_rtlo_characters(input_str: &str) -> String {
    input_str.replace(|char| RTLO_CHARACTERS.contains(&char), "")
}

/// Returns false if addr is an invalid address, otherwise true.
pub fn may_be_valid_addr(addr: &str) -> bool {
    let res = EmailAddress::new(addr);
    res.is_ok()
}

/// Returns address lowercased,
/// with whitespace trimmed and `mailto:` prefix removed.
pub fn addr_normalize(addr: &str) -> String {
    let norm = addr.trim().to_lowercase();

    if norm.starts_with("mailto:") {
        norm.get(7..).unwrap_or(&norm).to_string()
    } else {
        norm
    }
}

/// Compares two email addresses, normalizing them beforehand.
pub fn addr_cmp(addr1: &str, addr2: &str) -> bool {
    let norm1 = addr_normalize(addr1);
    let norm2 = addr_normalize(addr2);

    norm1 == norm2
}

///
/// Represents an email address, right now just the `name@domain` portion.
///
/// # Example
///
/// ```
/// use deltachat_contact_tools::EmailAddress;
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
                if domain.ends_with('.') {
                    bail!("Domain {domain:?} should not contain the dot in the end");
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

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    #[test]
    fn test_vcard_thunderbird() {
        let contacts = parse_vcard(
            "BEGIN:VCARD
VERSION:4.0
FN:'Alice Mueller'
EMAIL;PREF=1:alice.mueller@posteo.de
UID:a8083264-ca47-4be7-98a8-8ec3db1447ca
END:VCARD
BEGIN:VCARD
VERSION:4.0
FN:'bobzzz@freenet.de'
EMAIL;PREF=1:bobzzz@freenet.de
UID:cac4fef4-6351-4854-bbe4-9b6df857eaed
END:VCARD
",
        );

        assert_eq!(contacts[0].addr, "alice.mueller@posteo.de".to_string());
        assert_eq!(contacts[0].authname, "Alice Mueller".to_string());
        assert_eq!(contacts[0].key, None);
        assert_eq!(contacts[0].profile_image, None);
        assert!(contacts[0].timestamp.is_err());

        assert_eq!(contacts[1].addr, "bobzzz@freenet.de".to_string());
        assert_eq!(contacts[1].authname, "".to_string());
        assert_eq!(contacts[1].key, None);
        assert_eq!(contacts[1].profile_image, None);
        assert!(contacts[1].timestamp.is_err());

        assert_eq!(contacts.len(), 2);
    }

    #[test]
    fn test_vcard_simple_example() {
        let contacts = parse_vcard(
            "BEGIN:VCARD
VERSION:4.0
FN:Alice Wonderland
N:Wonderland;Alice;;;Ms.
GENDER:W
EMAIL;TYPE=work:alice@example.com
KEY;TYPE=PGP;ENCODING=b:[base64-data]
REV:20240418T184242Z

END:VCARD",
        );

        assert_eq!(contacts[0].addr, "alice@example.com".to_string());
        assert_eq!(contacts[0].authname, "Alice Wonderland".to_string());
        assert_eq!(contacts[0].key, Some("[base64-data]".to_string()));
        assert_eq!(contacts[0].profile_image, None);
        assert_eq!(*contacts[0].timestamp.as_ref().unwrap(), 1713465762);

        assert_eq!(contacts.len(), 1);
    }

    #[test]
    fn test_make_and_parse_vcard() {
        let contacts = [
            VcardContact {
                addr: "alice@example.org".to_string(),
                authname: "Alice Wonderland".to_string(),
                key: Some("[base64-data]".to_string()),
                profile_image: Some("image in Base64".to_string()),
                timestamp: Ok(1713465762),
            },
            VcardContact {
                addr: "bob@example.com".to_string(),
                authname: "".to_string(),
                key: None,
                profile_image: None,
                timestamp: Ok(0),
            },
        ];
        let items = [
            "BEGIN:VCARD\n\
             VERSION:4.0\n\
             EMAIL:alice@example.org\n\
             FN:Alice Wonderland\n\
             KEY:data:application/pgp-keys;base64,[base64-data]\n\
             PHOTO:data:image/jpeg;base64,image in Base64\n\
             REV:20240418T184242Z\n\
             END:VCARD\n",
            "BEGIN:VCARD\n\
             VERSION:4.0\n\
             EMAIL:bob@example.com\n\
             FN:bob@example.com\n\
             REV:19700101T000000Z\n\
             END:VCARD\n",
        ];
        let mut expected = "".to_string();
        for len in 0..=contacts.len() {
            let contacts = &contacts[0..len];
            let vcard = make_vcard(contacts);
            if len > 0 {
                expected += items[len - 1];
            }
            assert_eq!(vcard, expected);
            let parsed = parse_vcard(&vcard);
            assert_eq!(parsed.len(), contacts.len());
            for i in 0..parsed.len() {
                assert_eq!(parsed[i].addr, contacts[i].addr);
                assert_eq!(parsed[i].authname, contacts[i].authname);
                assert_eq!(parsed[i].key, contacts[i].key);
                assert_eq!(parsed[i].profile_image, contacts[i].profile_image);
                assert_eq!(
                    parsed[i].timestamp.as_ref().unwrap(),
                    contacts[i].timestamp.as_ref().unwrap()
                );
            }
        }
    }

    #[test]
    fn test_contact_address() -> Result<()> {
        let alice_addr = "alice@example.org";
        let contact_address = ContactAddress::new(alice_addr)?;
        assert_eq!(contact_address.as_ref(), alice_addr);

        let invalid_addr = "<> foobar";
        assert!(ContactAddress::new(invalid_addr).is_err());

        Ok(())
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
        assert!(EmailAddress::new("u@d.").is_err());
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

    #[test]
    fn test_vcard_android() {
        let contacts = parse_vcard(
            "BEGIN:VCARD
VERSION:2.1
N:;Bob;;;
FN:Bob
TEL;CELL:+1-234-567-890
EMAIL;HOME:bob@example.org
END:VCARD
BEGIN:VCARD
VERSION:2.1
N:;Alice;;;
FN:Alice
EMAIL;HOME:alice@example.org
END:VCARD
",
        );

        assert_eq!(contacts[0].addr, "bob@example.org".to_string());
        assert_eq!(contacts[0].authname, "Bob".to_string());
        assert_eq!(contacts[0].key, None);
        assert_eq!(contacts[0].profile_image, None);

        assert_eq!(contacts[1].addr, "alice@example.org".to_string());
        assert_eq!(contacts[1].authname, "Alice".to_string());
        assert_eq!(contacts[1].key, None);
        assert_eq!(contacts[1].profile_image, None);

        assert_eq!(contacts.len(), 2);
    }

    #[test]
    fn test_vcard_local_datetime() {
        let contacts = parse_vcard(
            "BEGIN:VCARD\n\
             VERSION:4.0\n\
             FN:Alice Wonderland\n\
             EMAIL;TYPE=work:alice@example.org\n\
             REV:20240418T184242\n\
             END:VCARD",
        );
        assert_eq!(contacts.len(), 1);
        assert_eq!(contacts[0].addr, "alice@example.org".to_string());
        assert_eq!(contacts[0].authname, "Alice Wonderland".to_string());
        assert_eq!(
            *contacts[0].timestamp.as_ref().unwrap(),
            chrono::offset::Local
                .with_ymd_and_hms(2024, 4, 18, 18, 42, 42)
                .unwrap()
                .timestamp()
        );
    }

    #[test]
    fn test_vcard_with_base64_avatar() {
        // This is not an actual base64-encoded avatar, it's just to test the parsing.
        // This one is Android-like.
        let vcard0 = "BEGIN:VCARD
VERSION:2.1
N:;Bob;;;
FN:Bob
EMAIL;HOME:bob@example.org
PHOTO;ENCODING=BASE64;JPEG:/9j/4AAQSkZJRgABAQAAAQABAAD/4gIoSUNDX1BST0ZJTEU
 AAQEAAAIYAAAAAAQwAABtbnRyUkdCIFhZWiAAAAAAAAAAAAAAAABhY3NwAAAAAAAAAAAAAAAA
 L8bRuAJYoZUYrI4ZY3VWwxw4Ay28AAGBISScmf/2Q==

END:VCARD
";
        // This one is DOS-like.
        let vcard1 = vcard0.replace('\n', "\r\n");
        for vcard in [vcard0, vcard1.as_str()] {
            let contacts = parse_vcard(vcard);
            assert_eq!(contacts.len(), 1);
            assert_eq!(contacts[0].addr, "bob@example.org".to_string());
            assert_eq!(contacts[0].authname, "Bob".to_string());
            assert_eq!(contacts[0].key, None);
            assert_eq!(contacts[0].profile_image.as_deref().unwrap(), "/9j/4AAQSkZJRgABAQAAAQABAAD/4gIoSUNDX1BST0ZJTEUAAQEAAAIYAAAAAAQwAABtbnRyUkdCIFhZWiAAAAAAAAAAAAAAAABhY3NwAAAAAAAAAAAAAAAAL8bRuAJYoZUYrI4ZY3VWwxw4Ay28AAGBISScmf/2Q==");
        }
    }
}
