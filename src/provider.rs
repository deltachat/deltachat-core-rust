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

#[derive(Debug, PartialEq)]
pub struct Server {
    pub stype: u16,
    // more fields are needed, just to keep the example short
}

#[derive(Debug, PartialEq)]
pub struct Provider {
    pub domains: &'static str,
    pub status: Status,
    pub before_login_hint: &'static str,
    pub overview_page: &'static str,
    pub server: Vec<Server>, // this seems to be okay
}

lazy_static::lazy_static! {
    // TODO: the database will be auto-generated from the provider-db
    static ref DATABASE: Vec<Provider> = vec![
        Provider {
            domains: "nauta.cu",
            status: Status::OK,
            before_login_hint: "",
            overview_page: "",
            server: vec![Server { stype: 123 }, Server { stype: 456 }],
        },
    /*
    Provider {
        domains: "outlook.com hotmail.com live.com",
        status: Status::BROKEN,
        before_login_hint: "Outlook-e-mail-addresses will not work as expected \
                            as these servers remove some important transport information.\n\n\
                            Hopefully sooner or later there will be a fix; \
                            for now, we suggest to use another e-mail-address \
                            or try Delta Chat again when the issue is fixed.",
        overview_page: "https://provider.delta.chat/outlook.com",
    },
    Provider {
        domains: "gmail.com googlemail.com",
        status: Status::PREPARATION,
        before_login_hint: "For Gmail Accounts, you need to create an App-Password \
                            if you have \"2-Step Verification\" enabled. \
                            If this setting is not available, \
                            you need to enable \"Less secure apps\".",
        overview_page: "https://provider.delta.chat/gmail.com",
    },
    */
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
