//! [Provider database](https://providers.delta.chat/) module

mod data;

use crate::config::Config;
use crate::dc_tools::EmailAddress;
use crate::provider::data::{PROVIDER_DATA, PROVIDER_UPDATED};
use chrono::{NaiveDateTime, NaiveTime};

#[derive(Debug, Display, Copy, Clone, PartialEq, FromPrimitive, ToPrimitive)]
#[repr(u8)]
pub enum Status {
    OK = 1,
    PREPARATION = 2,
    BROKEN = 3,
}

#[derive(Debug, Display, PartialEq, Copy, Clone, FromPrimitive, ToPrimitive)]
#[repr(u8)]
pub enum Protocol {
    SMTP = 1,
    IMAP = 2,
}

#[derive(Debug, Display, PartialEq, Copy, Clone, FromPrimitive, ToPrimitive)]
#[repr(u8)]
pub enum Socket {
    Automatic = 0,
    SSL = 1,
    STARTTLS = 2,
    Plain = 3,
}

impl Default for Socket {
    fn default() -> Self {
        Socket::Automatic
    }
}

#[derive(Debug, PartialEq, Clone)]
#[repr(u8)]
pub enum UsernamePattern {
    EMAIL = 1,
    EMAILLOCALPART = 2,
}

#[derive(Debug, PartialEq)]
#[repr(u8)]
pub enum Oauth2Authorizer {
    Yandex = 1,
    Gmail = 2,
}

#[derive(Debug, Clone)]
pub struct Server {
    pub protocol: Protocol,
    pub socket: Socket,
    pub hostname: &'static str,
    pub port: u16,
    pub username_pattern: UsernamePattern,
}

#[derive(Debug)]
pub struct ConfigDefault {
    pub key: Config,
    pub value: &'static str,
}

#[derive(Debug)]
pub struct Provider {
    pub status: Status,
    pub before_login_hint: &'static str,
    pub after_login_hint: &'static str,
    pub overview_page: &'static str,
    pub server: Vec<Server>,
    pub config_defaults: Option<Vec<ConfigDefault>>,
    pub strict_tls: bool,
    pub max_smtp_rcpt_to: Option<u16>,
    pub oauth2_authorizer: Option<Oauth2Authorizer>,
}

pub fn get_provider_info(addr: &str) -> Option<&Provider> {
    let domain = match addr.parse::<EmailAddress>() {
        Ok(addr) => addr.domain,
        Err(_err) => return None,
    }
    .to_lowercase();

    if let Some(provider) = PROVIDER_DATA.get(domain.as_str()) {
        return Some(*provider);
    }

    None
}

// returns update timestamp in seconds, UTC, compatible for comparison with time() and database times
pub fn get_provider_update_timestamp() -> i64 {
    NaiveDateTime::new(*PROVIDER_UPDATED, NaiveTime::from_hms(0, 0, 0)).timestamp_millis() / 1_000
}

#[cfg(test)]
mod tests {
    #![allow(clippy::indexing_slicing)]

    use super::*;
    use crate::dc_tools::time;
    use chrono::NaiveDate;

    #[test]
    fn test_get_provider_info_unexistant() {
        let provider = get_provider_info("user@unexistant.org");
        assert!(provider.is_none());
    }

    #[test]
    fn test_get_provider_info_mixed_case() {
        let provider = get_provider_info("uSer@nAUta.Cu").unwrap();
        assert!(provider.status == Status::OK);
    }

    #[test]
    fn test_get_provider_info() {
        let provider = get_provider_info("nauta.cu"); // this is no email address
        assert!(provider.is_none());

        let addr = "user@nauta.cu";
        let provider = get_provider_info(addr).unwrap();
        assert!(provider.status == Status::OK);
        let server = &provider.server[0];
        assert_eq!(server.protocol, Protocol::IMAP);
        assert_eq!(server.socket, Socket::STARTTLS);
        assert_eq!(server.hostname, "imap.nauta.cu");
        assert_eq!(server.port, 143);
        assert_eq!(server.username_pattern, UsernamePattern::EMAIL);
        let server = &provider.server[1];
        assert_eq!(server.protocol, Protocol::SMTP);
        assert_eq!(server.socket, Socket::STARTTLS);
        assert_eq!(server.hostname, "smtp.nauta.cu");
        assert_eq!(server.port, 25);
        assert_eq!(server.username_pattern, UsernamePattern::EMAIL);

        let provider = get_provider_info("user@gmail.com").unwrap();
        assert!(provider.status == Status::PREPARATION);
        assert!(!provider.before_login_hint.is_empty());
        assert!(!provider.overview_page.is_empty());

        let provider = get_provider_info("user@googlemail.com").unwrap();
        assert!(provider.status == Status::PREPARATION);
    }

    #[test]
    fn test_get_provider_update_timestamp() {
        let timestamp_past = NaiveDateTime::new(
            NaiveDate::from_ymd(2020, 9, 9),
            NaiveTime::from_hms(0, 0, 0),
        )
        .timestamp_millis()
            / 1_000;
        assert!(get_provider_update_timestamp() <= time());
        assert!(get_provider_update_timestamp() > timestamp_past);
    }
}
