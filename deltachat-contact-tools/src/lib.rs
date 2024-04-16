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
use anyhow::Result;
use once_cell::sync::Lazy;
use regex::Regex;

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
    if let Some(captures) = ADDR_WITH_NAME_REGEX.captures(addr.as_ref()) {
        (
            if name.is_empty() {
                strip_rtlo_characters(
                    &captures
                        .get(1)
                        .map_or("".to_string(), |m| normalize_name(m.as_str())),
                )
            } else {
                strip_rtlo_characters(name)
            },
            captures
                .get(2)
                .map_or("".to_string(), |m| m.as_str().to_string()),
        )
    } else {
        (strip_rtlo_characters(name), addr.to_string())
    }
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
    use super::*;

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
