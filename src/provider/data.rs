use crate::provider::*;
use crate::provider::Protocol::*;
use crate::provider::Socket::*;
use crate::provider::UsernamePattern::*;
use std::collections::HashMap;

lazy_static::lazy_static! {
    // nauta.cu
    static ref P1: Provider = Provider {
            status: Status::OK,
            before_login_hint: "",
            after_login_hint: "",
            overview_page: "",
            server: vec![
                Server { protocol: IMAP, socket: STARTTLS, hostname: "imap.nauta.cu", port: 143, username_pattern: EMAIL },
                Server { protocol: SMTP, socket: STARTTLS, hostname: "smtp.nauta.cu", port: 25, username_pattern: EMAIL },
            ],
        };

    // outlook.com, hotmail.com, live.com
    static ref P2: Provider = Provider {
            status: Status::BROKEN,
            before_login_hint: "Outlook-e-mail-addresses will not work as expected as these servers remove some important transport information.\n\nHopefully sooner or later there will be a fix; for now, we suggest to use another e-mail-address or try Delta Chat again when the issue is fixed.",
            after_login_hint: "",
            overview_page: "https://provider.delta.chat/outlook.com",
            server: vec![
            ],
        };

    // testrun.org
    static ref P3: Provider = Provider {
            status: Status::OK,
            before_login_hint: "",
            after_login_hint: "testrun.org is not Delta Chat :)",
            overview_page: "",
            server: vec![
            ],
        };

    // gmail.com, googlemail.com
    static ref P4: Provider = Provider {
            status: Status::PREPARATION,
            before_login_hint: "For Gmail Accounts, you need to create an App-Password if you have \"2-Step Verification\" enabled. If this setting is not available, you need to enable \"Less secure apps\".",
            after_login_hint: "",
            overview_page: "https://provider.delta.chat/gmail.com",
            server: vec![
            ],
        };

    pub static ref PROVIDER_DATA: HashMap<&'static str, &'static Provider> = [
        ("nauta.cu", &*P1),
        ("outlook.com", &*P2),
        ("hotmail.com", &*P2),
        ("live.com", &*P2),
        ("testrun.org", &*P3),
        ("gmail.com", &*P4),
        ("googlemail.com", &*P4),
    ].iter().copied().collect();
}
