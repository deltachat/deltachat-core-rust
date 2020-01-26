use crate::dc_tools::EmailAddress;
use crate::provider::Protocol::*;
use crate::provider::Socket::*;
use crate::provider::UsernamePattern::*;

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
    pub server: &'static str,
    pub port: u16,
    pub username_pattern: UsernamePattern,
}

#[derive(Debug)]
pub struct Provider {
    pub domains: &'static str,
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

lazy_static::lazy_static! {
    static ref DATABASE: Vec<Provider> = vec![
        Provider {
            domains: "nauta.cu",
            status: Status::OK,
            before_login_hint: "",
            after_login_hint: "",
            overview_page: "",
            server: vec![
                Server { protocol: IMAP, socket: STARTTLS, server: "imap.nauta.cu", port: 143, username_pattern: EMAIL },
                Server { protocol: SMTP, socket: STARTTLS, server: "smtp.nauta.cu", port: 25, username_pattern: EMAIL },
            ],
        },
        Provider {
            domains: "outlook.com hotmail.com live.com",
            status: Status::BROKEN,
            before_login_hint: "Outlook-e-mail-addresses will not work as expected \
                                as these servers remove some important transport information.\n\n\
                                Hopefully sooner or later there will be a fix; \
                                for now, we suggest to use another e-mail-address \
                                or try Delta Chat again when the issue is fixed.",
            after_login_hint: "",
            overview_page: "https://provider.delta.chat/outlook.com",
            server: vec![
            ],
        },
        Provider {
            domains: "gmail.com googlemail.com",
            status: Status::PREPARATION,
            before_login_hint: "For Gmail Accounts, you need to create an App-Password \
                                if you have \"2-Step Verification\" enabled. \
                                If this setting is not available, \
                                you need to enable \"Less secure apps\".",
            after_login_hint: "",
            overview_page: "https://provider.delta.chat/gmail.com",
            server: vec![
            ],
        },
    ];
}

pub fn get_provider_info(addr: &str) -> Option<&Provider> {
    let domain = match EmailAddress::new(addr) {
        Ok(addr) => addr.domain,
        Err(_err) => return None,
    }
    .to_lowercase();

    for record in DATABASE.iter() {
        for record_domain in record.domains.split(' ') {
            if record_domain == domain {
                return Some(record);
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_provider_info() {
        let provider = get_provider_info("user@unexistant.org");
        assert!(provider.is_none());

        let provider = get_provider_info("nauta.cu"); // this is no email address
        assert!(provider.is_none());

        let provider = get_provider_info("user@nauta.cu").unwrap();
        assert!(provider.status == Status::OK);
        let server = provider.get_imap_server().unwrap();
        assert_eq!(server.protocol, IMAP);
        assert_eq!(server.socket, STARTTLS);
        assert_eq!(server.server, "imap.nauta.cu");
        assert_eq!(server.port, 143);
        assert_eq!(server.username_pattern, EMAIL);
        let server = provider.get_smtp_server().unwrap();
        assert_eq!(server.protocol, SMTP);
        assert_eq!(server.socket, STARTTLS);
        assert_eq!(server.server, "smtp.nauta.cu");
        assert_eq!(server.port, 25);
        assert_eq!(server.username_pattern, EMAIL);

        let provider = get_provider_info("user@gmail.com").unwrap();
        assert!(provider.status == Status::PREPARATION);
        assert!(!provider.before_login_hint.is_empty());
        assert!(!provider.overview_page.is_empty());

        let provider = get_provider_info("user@googlemail.com").unwrap();
        assert!(provider.status == Status::PREPARATION);
    }
}
