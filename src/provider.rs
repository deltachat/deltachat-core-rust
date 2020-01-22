use crate::dc_tools::EmailAddress;

#[derive(Debug, Copy, Clone, PartialEq, ToPrimitive)]
#[repr(u8)]
pub enum Status {
    OK = 1,
    PREPARATION = 2,
    BROKEN = 3,
}

#[derive(Debug)]
#[repr(u8)]
pub enum ServerType {
    SMTP = 1,
    IMAP = 2,
}

#[derive(Debug)]
#[repr(u8)]
pub enum ServerSocket {
    STARTTLS = 1,
    SSL = 2,
}

#[derive(Debug)]
pub struct Server {
    pub stype: ServerType,
    pub socket: ServerSocket,
    pub port: u16,
    pub server: &'static str,
    pub username: &'static str,
}

#[derive(Debug, PartialEq)]
pub struct Provider {
    pub domains: &'static str,
    pub status: Status,
    pub before_login_hint: &'static str,
    pub overview_page: &'static str,
}

// TODO: the database will be auto-generated from the provider-db
const DATABASE: [Provider; 3] = [
    Provider {
        domains: "nauta.cu",
        status: Status::OK,
        before_login_hint: "",
        overview_page: "",
    },
    Provider {
        domains: "outlook.com live.com",
        status: Status::BROKEN,
        before_login_hint: "this provider is broken, sorry :(",
        overview_page: "https://provider.delta.chat/outlook.com",
    },
    Provider {
        domains: "gmail.com",
        status: Status::PREPARATION,
        before_login_hint: "please enable less-secure-apps",
        overview_page: "https://provider.delta.chat/gmail.com",
    },
];

pub fn get_provider_info(addr: &str) -> Option<&Provider> {
    let domain = match EmailAddress::new(addr) {
        Ok(addr) => addr.domain,
        Err(_err) => return None,
    }
    .to_lowercase();

    for record in &DATABASE {
        for record_domain in record.domains.split(" ") {
            if record_domain == domain {
                return Some(record);
            }
        }
    }

    None
}
