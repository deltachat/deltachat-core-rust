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
use anyhow::format_err;
use anyhow::Context as _;
use anyhow::Result;
use chrono::DateTime;
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
    /// The contact's display name, vcard property `fn`
    pub display_name: String,
    /// The contact's public PGP key, vcard property `key`
    pub key: Option<String>,
    /// The contact's profile photo (=avatar), vcard property `photo`
    pub profile_photo: Option<String>,
    /// The timestamp when the vcard was created / last updated, vcard property `rev`
    pub timestamp: Result<u64>,
}

pub fn parse_vcard(vcard: String) -> Result<Vec<VcardContact>> {
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

        // TODO this doesn't handle the case where there are quotes around a colon
        let (_params, value) = remainder.split_once(':')?;
        Some(value)
    }
    fn parse_datetime(datetime: Option<&str>) -> Result<u64> {
        let datetime = datetime.context("No timestamp in vcard")?;

        // According to https://www.rfc-editor.org/rfc/rfc6350#section-4.3.5, the timestamp
        // is in ISO.8601.2004 format. DateTime::parse_from_rfc3339() apparently parses
        // ISO.8601, but fails to parse any of the examples given.
        // So, instead just parse using a format string.
        let datetime =
            DateTime::parse_from_str(datetime, "%Y%m%dT%H%M%S%#z") // Parses 19961022T140000Z, 19961022T140000-05, or 19961022T140000-0500
                .or_else(|_| DateTime::parse_from_str(datetime, "%Y%m%dT%H%M%S"))?; // Parses 19961022T140000
        let timestamp = datetime.timestamp().try_into()?;
        Ok(timestamp)
    }

    let mut lines = vcard.lines().peekable();
    let mut contacts = Vec::new();

    while lines.peek().is_some() {
        // Skip to the start of the vcard:
        for line in lines.by_ref() {
            if line.eq_ignore_ascii_case("BEGIN:VCARD") {
                break;
            }
        }

        let mut display_name = "";
        let mut addr = "";
        let mut key = None;
        let mut photo = None;
        let mut datetime = None;

        for line in lines.by_ref() {
            if let Some(email) = vcard_property(line, "email") {
                addr = email;
            } else if let Some(name) = vcard_property(line, "fn") {
                display_name = name;
            } else if let Some(k) = remove_prefix(line, "KEY;PGP;ENCODING=BASE64:")
                .or_else(|| remove_prefix(line, "KEY;TYPE=PGP;ENCODING=b:"))
                .or_else(|| remove_prefix(line, "KEY:data:application/pgp-keys;base64,"))
            {
                key = Some(key.unwrap_or(k));
            } else if let Some(p) = remove_prefix(line, "PHOTO;JPEG;ENCODING=BASE64:")
                .or_else(|| remove_prefix(line, "PHOTO;TYPE=JPEG;ENCODING=b:"))
                .or_else(|| remove_prefix(line, "PHOTO;ENCODING=BASE64;TYPE=JPEG:"))
            {
                photo = Some(photo.unwrap_or(p));
            } else if let Some(rev) = vcard_property(line, "rev") {
                datetime = Some(datetime.unwrap_or(rev));
            } else if line.eq_ignore_ascii_case("END:VCARD") {
                break;
            }
        }

        let (display_name, addr) = sanitize_name_and_addr(display_name, addr);

        contacts.push(VcardContact {
            display_name,
            addr,
            key: key.map(|s| s.to_string()),
            profile_photo: photo.map(|s| s.to_string()),
            timestamp: parse_datetime(datetime),
        });
    }

    Ok(contacts)
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
    use chrono::NaiveDateTime;

    use super::*;

    #[test]
    fn test_thunderbird() {
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
"
            .to_string(),
        )
        .unwrap();

        assert_eq!(contacts[0].addr, "alice.mueller@posteo.de".to_string());
        assert_eq!(contacts[0].display_name, "Alice Mueller".to_string());
        assert_eq!(contacts[0].key, None);
        assert_eq!(contacts[0].profile_photo, None);
        assert!(contacts[0].timestamp.is_err());

        assert_eq!(contacts[1].addr, "bobzzz@freenet.de".to_string());
        assert_eq!(contacts[1].display_name, "".to_string());
        assert_eq!(contacts[1].key, None);
        assert_eq!(contacts[1].profile_photo, None);
        assert!(contacts[1].timestamp.is_err());

        assert_eq!(contacts.len(), 2);
    }

    #[test]
    fn test_simple_example() {
        let contacts = parse_vcard(
            "BEGIN:VCARD
VERSION:4.0
FN:Alice Wonderland
N:Wonderland;Alice;;;Ms.
GENDER:W
EMAIL;TYPE=work:alice@example.com
KEY;TYPE=PGP;ENCODING=b:[base64-data]
REV:20240418T184242Z

END:VCARD"
                .to_string(),
        )
        .unwrap();

        assert_eq!(contacts[0].addr, "alice@example.com".to_string());
        assert_eq!(contacts[0].display_name, "Alice Wonderland".to_string());
        assert_eq!(contacts[0].key, Some("[base64-data]".to_string()));
        assert_eq!(contacts[0].profile_photo, None);
        assert_eq!(*contacts[0].timestamp.as_ref().unwrap(), 1713465762); // I did not check whether this timestamp is correct

        assert_eq!(contacts.len(), 1);
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
}
