use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};

use crate::config::Config;
use crate::constants::*;
use crate::context::Context;
use crate::dc_tools::*;
use crate::e2ee;
use crate::imap::*;
use crate::job::*;
use crate::login_param::LoginParam;
use crate::oauth2::*;
use crate::param::Params;

mod auto_outlook;
use auto_outlook::outlk_autodiscover;
mod auto_mozilla;
use auto_mozilla::moz_autoconfigure;

macro_rules! progress {
    ($context:tt, $progress:expr) => {
        assert!(
            $progress <= 1000,
            "value in range 0..1000 expected with: 0=error, 1..999=progress, 1000=success"
        );
        $context.call_cb($crate::events::Event::ConfigureProgress($progress));
    };
}

// connect
pub unsafe fn configure(context: &Context) {
    if context.has_ongoing() {
        warn!(context, "There is already another ongoing process running.",);
        return;
    }
    job_kill_action(context, Action::ConfigureImap);
    job_add(context, Action::ConfigureImap, 0, Params::new(), 0);
}

/// Check if the context is already configured.
pub fn dc_is_configured(context: &Context) -> bool {
    context.sql.get_raw_config_bool(context, "configured")
}

/*******************************************************************************
 * Configure JOB
 ******************************************************************************/
// the other dc_job_do_DC_JOB_*() functions are declared static in the c-file
#[allow(non_snake_case, unused_must_use)]
pub fn dc_job_do_DC_JOB_CONFIGURE_IMAP(context: &Context) {
    if !context.sql.is_open() {
        error!(context, "Cannot configure, database not opened.",);
        progress!(context, 0);
        return;
    }
    if !context.alloc_ongoing() {
        progress!(context, 0);
        return;
    }
    let mut success = false;
    let mut imap_connected_here = false;
    let mut smtp_connected_here = false;

    let mut param_autoconfig: Option<LoginParam> = None;

    context.inbox.read().unwrap().disconnect(context);
    context
        .sentbox_thread
        .read()
        .unwrap()
        .imap
        .disconnect(context);
    context
        .mvbox_thread
        .read()
        .unwrap()
        .imap
        .disconnect(context);
    context.smtp.clone().lock().unwrap().disconnect();
    info!(context, "Configure ...",);

    // Variables that are shared between steps:
    let mut param = LoginParam::from_database(context, "");
    // need all vars here to be mutable because rust thinks the same step could be called multiple times
    // and also initialize, because otherwise rust thinks it's used while unitilized, even if thats not the case as the loop goes only forward
    let mut param_domain = "undefined.undefined".to_owned();
    let mut param_addr_urlencoded: String =
        "Internal Error: this value should never be used".to_owned();
    let mut keep_flags = std::i32::MAX;

    const STEP_3_INDEX: u8 = 13;
    let mut step_counter: u8 = 0;
    while !context.shall_stop_ongoing() {
        step_counter = step_counter + 1;

        let success = match step_counter {
            // Read login parameters from the database
            1 => {
                progress!(context, 1);
                if param.addr.is_empty() {
                    error!(context, "Please enter an email address.",);
                }
                !param.addr.is_empty()
            }
            // Step 1: Load the parameters and check email-address and password
            2 => {
                if 0 != param.server_flags & 0x2 {
                    // the used oauth2 addr may differ, check this.
                    // if dc_get_oauth2_addr() is not available in the oauth2 implementation,
                    // just use the given one.
                    progress!(context, 10);
                    if let Some(oauth2_addr) =
                        dc_get_oauth2_addr(context, &param.addr, &param.mail_pw)
                            .and_then(|e| e.parse().ok())
                    {
                        param.addr = oauth2_addr;
                        context
                            .sql
                            .set_raw_config(context, "addr", Some(param.addr.as_str()))
                            .ok();
                    }
                    progress!(context, 20);
                }
                true // no oauth? - just continue it's no error
            }
            3 => {
                if let Ok(parsed) = param.addr.parse() {
                    let parsed: EmailAddress = parsed;
                    param_domain = parsed.domain;
                    param_addr_urlencoded =
                        utf8_percent_encode(&param.addr, NON_ALPHANUMERIC).to_string();
                    true
                } else {
                    error!(context, "Bad email-address.");
                    false
                }
            }
            // Step 2: Autoconfig
            4 => {
                progress!(context, 200);
                if param.mail_server.is_empty()
                            && param.mail_port == 0
                            /*&&param.mail_user.is_empty() -- the user can enter a loginname which is used by autoconfig then */
                            && param.send_server.is_empty()
                            && param.send_port == 0
                            && param.send_user.is_empty()
                            /*&&param.send_pw.is_empty() -- the password cannot be auto-configured and is no criterion for autoconfig or not */
                            && param.server_flags & !0x2 == 0
                {
                    keep_flags = param.server_flags & 0x2;
                } else {
                    // Autoconfig is not needed so skip it.
                    step_counter = STEP_3_INDEX - 1;
                }
                true
            }
            /* A.  Search configurations from the domain used in the email-address, prefer encrypted */
            5 => {
                if param_autoconfig.is_none() {
                    let url = format!(
                        "https://autoconfig.{}/mail/config-v1.1.xml?emailaddress={}",
                        param_domain, param_addr_urlencoded
                    );
                    param_autoconfig = moz_autoconfigure(context, &url, &param);
                }
                true
            }
            6 => {
                progress!(context, 300);
                if param_autoconfig.is_none() {
                    // the doc does not mention `emailaddress=`, however, Thunderbird adds it, see https://releases.mozilla.org/pub/thunderbird/ ,  which makes some sense
                    let url = format!(
                        "https://{}/.well-known/autoconfig/mail/config-v1.1.xml?emailaddress={}",
                        param_domain, param_addr_urlencoded
                    );
                    param_autoconfig = moz_autoconfigure(context, &url, &param);
                }
                true
            }
            /* Outlook section start ------------- */
            /* Outlook uses always SSL but different domains (this comment describes the next two steps) */
            7 => {
                progress!(context, 310);
                if param_autoconfig.is_none() {
                    let url = format!(
                        "https://{}{}/autodiscover/autodiscover.xml",
                        "", param_domain
                    );
                    param_autoconfig = outlk_autodiscover(context, &url, &param);
                }
                true
            }
            8 => {
                progress!(context, 320);
                if param_autoconfig.is_none() {
                    let url = format!(
                        "https://{}{}/autodiscover/autodiscover.xml",
                        "autodiscover.", param_domain
                    );
                    param_autoconfig = outlk_autodiscover(context, &url, &param);
                }
                true
            }
            /* ----------- Outlook section end */
            9 => {
                progress!(context, 330);
                if param_autoconfig.is_none() {
                    let url = format!(
                        "http://autoconfig.{}/mail/config-v1.1.xml?emailaddress={}",
                        param_domain, param_addr_urlencoded
                    );
                    param_autoconfig = moz_autoconfigure(context, &url, &param);
                }
                true
            }
            10 => {
                progress!(context, 340);
                if param_autoconfig.is_none() {
                    // do not transfer the email-address unencrypted
                    let url = format!(
                        "http://{}/.well-known/autoconfig/mail/config-v1.1.xml",
                        param_domain
                    );
                    param_autoconfig = moz_autoconfigure(context, &url, &param);
                }
                true
            }
            /* B.  If we have no configuration yet, search configuration in Thunderbird's centeral database */
            11 => {
                progress!(context, 350);
                if param_autoconfig.is_none() {
                    /* always SSL for Thunderbird's database */
                    let url = format!("https://autoconfig.thunderbird.net/v1.1/{}", param_domain);
                    param_autoconfig = moz_autoconfigure(context, &url, &param);
                }
                true
            }
            /* C.  Do we have any result? */
            12 => {
                progress!(context, 500);
                if let Some(ref cfg) = param_autoconfig {
                    info!(context, "Got autoconfig: {}", &cfg);
                    if !cfg.mail_user.is_empty() {
                        param.mail_user = cfg.mail_user.clone();
                    }
                    param.mail_server = cfg.mail_server.clone(); /* all other values are always NULL when entering autoconfig */
                    param.mail_port = cfg.mail_port;
                    param.send_server = cfg.send_server.clone();
                    param.send_port = cfg.send_port;
                    param.send_user = cfg.send_user.clone();
                    param.server_flags = cfg.server_flags;
                    /* although param_autoconfig's data are no longer needed from, it is important to keep the object as
                    we may enter "deep guessing" if we could not read a configuration */
                }
                param.server_flags |= keep_flags;
                true
            }
            // Step 3: Fill missing fields with defaults
            13 => {
                // if you move this, don't forget to update STEP_3_INDEX, too
                if param.mail_server.is_empty() {
                    param.mail_server = format!("imap.{}", param_domain,)
                }
                if param.mail_port == 0 {
                    param.mail_port = if 0 != param.server_flags & (0x100 | 0x400) {
                        143
                    } else {
                        993
                    }
                }
                if param.mail_user.is_empty() {
                    param.mail_user = param.addr.clone();
                }
                if param.send_server.is_empty() && !param.mail_server.is_empty() {
                    param.send_server = param.mail_server.clone();
                    if param.send_server.starts_with("imap.") {
                        param.send_server = param.send_server.replacen("imap", "smtp", 1);
                    }
                }
                if param.send_port == 0 {
                    param.send_port = if 0 != param.server_flags & DC_LP_SMTP_SOCKET_STARTTLS as i32
                    {
                        587
                    } else if 0 != param.server_flags & DC_LP_SMTP_SOCKET_PLAIN as i32 {
                        25
                    } else {
                        465
                    }
                }
                if param.send_user.is_empty() && !param.mail_user.is_empty() {
                    param.send_user = param.mail_user.clone();
                }
                if param.send_pw.is_empty() && !param.mail_pw.is_empty() {
                    param.send_pw = param.mail_pw.clone()
                }
                if !dc_exactly_one_bit_set(param.server_flags & DC_LP_AUTH_FLAGS as i32) {
                    param.server_flags &= !(DC_LP_AUTH_FLAGS as i32);
                    param.server_flags |= DC_LP_AUTH_NORMAL as i32
                }
                if !dc_exactly_one_bit_set(param.server_flags & DC_LP_IMAP_SOCKET_FLAGS as i32) {
                    param.server_flags &= !(DC_LP_IMAP_SOCKET_FLAGS as i32);
                    param.server_flags |= if param.send_port == 143 {
                        DC_LP_IMAP_SOCKET_STARTTLS as i32
                    } else {
                        DC_LP_IMAP_SOCKET_SSL as i32
                    }
                }
                if !dc_exactly_one_bit_set(param.server_flags & (DC_LP_SMTP_SOCKET_FLAGS as i32)) {
                    param.server_flags &= !(DC_LP_SMTP_SOCKET_FLAGS as i32);
                    param.server_flags |= if param.send_port == 587 {
                        DC_LP_SMTP_SOCKET_STARTTLS as i32
                    } else if param.send_port == 25 {
                        DC_LP_SMTP_SOCKET_PLAIN as i32
                    } else {
                        DC_LP_SMTP_SOCKET_SSL as i32
                    }
                }
                /* do we have a complete configuration? */
                if param.mail_server.is_empty()
                    || param.mail_port == 0
                    || param.mail_user.is_empty()
                    || param.mail_pw.is_empty()
                    || param.send_server.is_empty()
                    || param.send_port == 0
                    || param.send_user.is_empty()
                    || param.send_pw.is_empty()
                    || param.server_flags == 0
                {
                    error!(context, "Account settings incomplete.");
                    false
                } else {
                    true
                }
            }
            14 => {
                progress!(context, 600);
                /* try to connect to IMAP - if we did not got an autoconfig,
                do some further tries with different settings and username variations */
                imap_connected_here =
                    try_imap_connections(context, &mut param, param_autoconfig.is_some());
                imap_connected_here
            }
            15 => {
                progress!(context, 800);
                smtp_connected_here =
                    try_smtp_connections(context, &mut param, param_autoconfig.is_some());
                smtp_connected_here
            }
            16 => {
                progress!(context, 900);
                let flags: libc::c_int = if context.get_config_bool(Config::MvboxWatch)
                    || context.get_config_bool(Config::MvboxMove)
                {
                    DC_CREATE_MVBOX as i32
                } else {
                    0
                };
                context
                    .inbox
                    .read()
                    .unwrap()
                    .configure_folders(context, flags);
                true
            }
            17 => {
                progress!(context, 910);
                /* configuration success - write back the configured parameters with the "configured_" prefix; also write the "configured"-flag */
                param
                    .save_to_database(
                        context,
                        "configured_", /*the trailing underscore is correct*/
                    )
                    .ok();

                context.sql.set_raw_config_bool(context, "configured", true);
                true
            }
            18 => {
                progress!(context, 920);
                // we generate the keypair just now - we could also postpone this until the first message is sent, however,
                // this may result in a unexpected and annoying delay when the user sends his very first message
                // (~30 seconds on a Moto G4 play) and might looks as if message sending is always that slow.
                e2ee::ensure_secret_key_exists(context);
                success = true;
                info!(context, "Configure completed.");
                progress!(context, 940);
                break; // We are done here
            }

            _ => {
                error!(context, "Internal error: step counter out of bound",);
                break;
            }
        };

        if !success {
            break;
        }
    }
    if imap_connected_here {
        context.inbox.read().unwrap().disconnect(context);
    }
    if smtp_connected_here {
        context.smtp.clone().lock().unwrap().disconnect();
    }

    /*
    if !success {
        // disconnect if configure did not succeed
        if imap_connected_here {
            // context.inbox.read().unwrap().disconnect(context);
        }
        if smtp_connected_here {
            // context.smtp.clone().lock().unwrap().disconnect();
        }
    } else {
        assert!(imap_connected_here && smtp_connected_here);
        info!(
            context,
            0, "Keeping IMAP/SMTP connections open after successful configuration"
        );
    }
    */
    context.free_ongoing();
    progress!(context, if success { 1000 } else { 0 });
}

fn try_imap_connections(
    context: &Context,
    mut param: &mut LoginParam,
    was_autoconfig: bool,
) -> bool {
    // progress 650 and 660
    if let Some(res) = try_imap_connection(context, &mut param, was_autoconfig, 0) {
        return res;
    }
    progress!(context, 670);
    param.server_flags &= !(DC_LP_IMAP_SOCKET_FLAGS);
    param.server_flags |= DC_LP_IMAP_SOCKET_SSL;
    param.mail_port = 993;

    if let Some(at) = param.mail_user.find('@') {
        param.mail_user = param.mail_user.split_at(at).0.to_string();
    }
    if let Some(at) = param.send_user.find('@') {
        param.send_user = param.send_user.split_at(at).0.to_string();
    }
    // progress 680 and 690
    if let Some(res) = try_imap_connection(context, &mut param, was_autoconfig, 1) {
        res
    } else {
        false
    }
}

fn try_imap_connection(
    context: &Context,
    param: &mut LoginParam,
    was_autoconfig: bool,
    variation: usize,
) -> Option<bool> {
    if let Some(res) = try_imap_one_param(context, &param) {
        return Some(res);
    }
    if was_autoconfig {
        return Some(false);
    }
    progress!(context, 650 + variation * 30);
    param.server_flags &= !(DC_LP_IMAP_SOCKET_FLAGS);
    param.server_flags |= DC_LP_IMAP_SOCKET_STARTTLS;
    if let Some(res) = try_imap_one_param(context, &param) {
        return Some(res);
    }

    progress!(context, 660 + variation * 30);
    param.mail_port = 143;

    try_imap_one_param(context, &param)
}

fn try_imap_one_param(context: &Context, param: &LoginParam) -> Option<bool> {
    let inf = format!(
        "imap: {}@{}:{} flags=0x{:x}",
        param.mail_user, param.mail_server, param.mail_port, param.server_flags
    );
    info!(context, "Trying: {}", inf);
    if context.inbox.read().unwrap().connect(context, &param) {
        info!(context, "success: {}", inf);
        return Some(true);
    }
    if context.shall_stop_ongoing() {
        return Some(false);
    }
    info!(context, "Could not connect: {}", inf);
    None
}

fn try_smtp_connections(
    context: &Context,
    mut param: &mut LoginParam,
    was_autoconfig: bool,
) -> bool {
    /* try to connect to SMTP - if we did not got an autoconfig, the first try was SSL-465 and we do a second try with STARTTLS-587 */
    if let Some(res) = try_smtp_one_param(context, &param) {
        return res;
    }
    if was_autoconfig {
        return false;
    }
    progress!(context, 850);
    param.server_flags &= !(DC_LP_SMTP_SOCKET_FLAGS as i32);
    param.server_flags |= DC_LP_SMTP_SOCKET_STARTTLS as i32;
    param.send_port = 587;

    if let Some(res) = try_smtp_one_param(context, &param) {
        return res;
    }
    progress!(context, 860);
    param.server_flags &= !(DC_LP_SMTP_SOCKET_FLAGS as i32);
    param.server_flags |= DC_LP_SMTP_SOCKET_STARTTLS as i32;
    param.send_port = 25;
    if let Some(res) = try_smtp_one_param(context, &param) {
        return res;
    }
    false
}

fn try_smtp_one_param(context: &Context, param: &LoginParam) -> Option<bool> {
    let inf = format!(
        "smtp: {}@{}:{} flags: 0x{:x}",
        param.send_user, param.send_server, param.send_port, param.server_flags
    );
    info!(context, "Trying: {}", inf);
    if context
        .smtp
        .clone()
        .lock()
        .unwrap()
        .connect(context, &param)
    {
        info!(context, "success: {}", inf);
        return Some(true);
    }
    if context.shall_stop_ongoing() {
        return Some(false);
    }
    info!(context, "could not connect: {}", inf);
    None
}

/*******************************************************************************
 * Connect to configured account
 ******************************************************************************/
pub fn dc_connect_to_configured_imap(context: &Context, imap: &Imap) -> libc::c_int {
    let mut ret_connected = 0;

    if imap.is_connected() {
        ret_connected = 1
    } else if !context.sql.get_raw_config_bool(context, "configured") {
        warn!(context, "Not configured, cannot connect.",);
    } else {
        let param = LoginParam::from_database(context, "configured_");
        // the trailing underscore is correct

        if imap.connect(context, &param) {
            ret_connected = 2;
        }
    }

    ret_connected
}

/*******************************************************************************
 * Configure a Context
 ******************************************************************************/

pub fn read_autoconf_file(context: &Context, url: &str) -> Option<String> {
    info!(context, "Testing {} ...", url);

    match reqwest::Client::new()
        .get(url)
        .send()
        .and_then(|mut res| res.text())
    {
        Ok(res) => Some(res),
        Err(_err) => {
            info!(context, "Can\'t read file.",);

            None
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::config::*;
    use crate::configure::dc_job_do_DC_JOB_CONFIGURE_IMAP;
    use crate::test_utils::*;

    #[test]
    fn test_no_panic_on_bad_credentials() {
        let t = dummy_context();
        t.ctx
            .set_config(Config::Addr, Some("probably@unexistant.addr"))
            .unwrap();
        t.ctx.set_config(Config::MailPw, Some("123456")).unwrap();
        dc_job_do_DC_JOB_CONFIGURE_IMAP(&t.ctx);
    }
}
