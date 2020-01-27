use crate::provider::*;
use crate::provider::Protocol::*;
use crate::provider::Socket::*;
use crate::provider::UsernamePattern::*;

lazy_static::lazy_static! {
    pub static ref PROVIDER_DATA: Vec<Provider> = vec![
        Provider {
            domains: "nauta.cu",
            status: Status::OK,
            before_login_hint: "",
            after_login_hint: "",
            overview_page: "",
            server: vec![
                Server { protocol: IMAP, socket: STARTTLS, hostname: "imap.nauta.cu", port: 143, username_pattern: EMAIL },
                Server { protocol: SMTP, socket: STARTTLS, hostname: "smtp.nauta.cu", port: 25, username_pattern: EMAIL },
            ],
        },
        Provider {
            domains: "outlook.com hotmail.com live.com",
            status: Status::BROKEN,
            before_login_hint: "Outlook-e-mail-addresses will not work as expected as these servers remove some important transport information.\n\nHopefully sooner or later there will be a fix; for now, we suggest to use another e-mail-address or try Delta Chat again when the issue is fixed.",
            after_login_hint: "",
            overview_page: "https://provider.delta.chat/outlook.com",
            server: vec![
            ],
        },
        Provider {
            domains: "testrun.org",
            status: Status::OK,
            before_login_hint: "",
            after_login_hint: "testrun.org is not Delta Chat :)",
            overview_page: "",
            server: vec![
            ],
        },
        Provider {
            domains: "gmail.com googlemail.com",
            status: Status::PREPARATION,
            before_login_hint: "For Gmail Accounts, you need to create an App-Password if you have \"2-Step Verification\" enabled. If this setting is not available, you need to enable \"Less secure apps\".",
            after_login_hint: "",
            overview_page: "https://provider.delta.chat/gmail.com",
            server: vec![
            ],
        },
    ];
}
