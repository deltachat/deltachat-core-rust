mod data;

use crate::dc_tools::EmailAddress;
use crate::provider::data::PROVIDER_DATA;

#[derive(Debug, Copy, Clone, PartialEq, ToPrimitive)]
#[repr(u8)]
pub enum Status {
    OK = 1,
    PREPARATION = 2,
    BROKEN = 3,
}

#[derive(Debug, PartialEq)]
#[repr(u8)]
pub enum Protocol {
    SMTP = 1,
    IMAP = 2,
}

#[derive(Debug, PartialEq)]
#[repr(u8)]
pub enum Socket {
    STARTTLS = 1,
    SSL = 2,
}

#[derive(Debug, PartialEq)]
#[repr(u8)]
pub enum UsernamePattern {
    EMAIL = 1,
    EMAILLOCALPART = 2,
}

#[derive(Debug)]
pub struct Server {
    pub protocol: Protocol,
    pub socket: Socket,
    pub hostname: &'static str,
    pub port: u16,
    pub username_pattern: UsernamePattern,
}

impl Server {
    pub fn apply_username_pattern(&self, addr: String) -> String {
        match self.username_pattern {
            UsernamePattern::EMAIL => addr,
            UsernamePattern::EMAILLOCALPART => {
                if let Some(at) = addr.find('@') {
                    return addr.split_at(at).0.to_string();
                }
                addr
            }
        }
    }
}

#[derive(Debug)]
pub struct Provider {
    pub status: Status,
    pub before_login_hint: &'static str,
    pub after_login_hint: &'static str,
    pub overview_page: &'static str,
    pub server: Vec<Server>,
}

impl Provider {
    pub fn get_server(&self, protocol: Protocol) -> Option<&Server> {
        for record in self.server.iter() {
            if record.protocol == protocol {
                return Some(record);
            }
        }
        None
    }

    pub fn get_imap_server(&self) -> Option<&Server> {
        self.get_server(Protocol::IMAP)
    }

    pub fn get_smtp_server(&self) -> Option<&Server> {
        self.get_server(Protocol::SMTP)
    }
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

#[cfg(test)]
mod tests {
    use super::*;

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

        let provider = get_provider_info("user@nauta.cu").unwrap();
        assert!(provider.status == Status::OK);
        let server = provider.get_imap_server().unwrap();
        assert_eq!(server.protocol, Protocol::IMAP);
        assert_eq!(server.socket, Socket::STARTTLS);
        assert_eq!(server.hostname, "imap.nauta.cu");
        assert_eq!(server.port, 143);
        assert_eq!(server.username_pattern, UsernamePattern::EMAIL);
        let server = provider.get_smtp_server().unwrap();
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
}
