//! Email accounts autoconfiguration process module

mod auto_mozilla;
mod auto_outlook;
mod read_url;

use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};

use async_std::task;

use crate::config::Config;
use crate::constants::*;
use crate::context::Context;
use crate::dc_tools::*;
use crate::job::{self, job_add, job_kill_action};
use crate::login_param::{CertificateChecks, LoginParam};
use crate::oauth2::*;
use crate::param::Params;
use crate::{chat, e2ee, provider};

use crate::message::Message;
use auto_mozilla::moz_autoconfigure;
use auto_outlook::outlk_autodiscover;

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
pub fn configure(context: &Context) {
    if context.has_ongoing() {
        warn!(context, "There is already another ongoing process running.",);
        return;
    }
    job_kill_action(context, job::Action::ConfigureImap);
    job_add(context, job::Action::ConfigureImap, 0, Params::new(), 0);
}

/// Check if the context is already configured.
pub fn dc_is_configured(context: &Context) -> bool {
    context.sql.get_raw_config_bool(context, "configured")
}

/*******************************************************************************
 * Configure JOB
 ******************************************************************************/
#[allow(non_snake_case, unused_must_use, clippy::cognitive_complexity)]
pub fn JobConfigureImap(context: &Context) -> job::Status {
    if !context.sql.is_open() {
        error!(context, "Cannot configure, database not opened.",);
        progress!(context, 0);
        return job::Status::Finished(Err(format_err!("Database not opened")));
    }
    if !context.alloc_ongoing() {
        progress!(context, 0);
        return job::Status::Finished(Err(format_err!("Cannot allocated ongoing process")));
    }
    let mut success = false;
    let mut imap_connected_here = false;
    let mut smtp_connected_here = false;

    let mut param_autoconfig: Option<LoginParam> = None;

    context
        .inbox_thread
        .read()
        .unwrap()
        .imap
        .disconnect(context);
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
    let mut keep_flags = 0;

    const STEP_12_USE_AUTOCONFIG: u8 = 12;
    const STEP_13_AFTER_AUTOCONFIG: u8 = 13;

    let mut step_counter: u8 = 0;
    while !context.shall_stop_ongoing() {
        step_counter += 1;

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
                if 0 != param.server_flags & DC_LP_AUTH_OAUTH2 {
                    // the used oauth2 addr may differ, check this.
                    // if dc_get_oauth2_addr() is not available in the oauth2 implementation,
                    // just use the given one.
                    progress!(context, 10);
                    if let Some(oauth2_addr) =
                        dc_get_oauth2_addr(context, &param.addr, &param.mail_pw)
                            .and_then(|e| e.parse().ok())
                    {
                        info!(context, "Authorized address is {}", oauth2_addr);
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
                            && (param.server_flags & !DC_LP_AUTH_OAUTH2) == 0
                {
                    // no advanced parameters entered by the user: query provider-database or do Autoconfig
                    keep_flags = param.server_flags & DC_LP_AUTH_OAUTH2;
                    if let Some(new_param) = get_offline_autoconfig(context, &param) {
                        // got parameters from our provider-database, skip Autoconfig, preserve the OAuth2 setting
                        param_autoconfig = Some(new_param);
                        step_counter = STEP_12_USE_AUTOCONFIG - 1; // minus one as step_counter is increased on next loop
                    }
                } else {
                    // advanced parameters entered by the user: skip Autoconfig
                    step_counter = STEP_13_AFTER_AUTOCONFIG - 1; // minus one as step_counter is increased on next loop
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
                    param_autoconfig = moz_autoconfigure(context, &url, &param).ok();
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
                    param_autoconfig = moz_autoconfigure(context, &url, &param).ok();
                }
                true
            }
            /* Outlook section start ------------- */
            /* Outlook uses always SSL but different domains (this comment describes the next two steps) */
            7 => {
                progress!(context, 310);
                if param_autoconfig.is_none() {
                    let url = format!("https://{}/autodiscover/autodiscover.xml", param_domain);
                    param_autoconfig = outlk_autodiscover(context, &url, &param).ok();
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
                    param_autoconfig = outlk_autodiscover(context, &url, &param).ok();
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
                    param_autoconfig = moz_autoconfigure(context, &url, &param).ok();
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
                    param_autoconfig = moz_autoconfigure(context, &url, &param).ok();
                }
                true
            }
            /* B.  If we have no configuration yet, search configuration in Thunderbird's centeral database */
            11 => {
                progress!(context, 350);
                if param_autoconfig.is_none() {
                    /* always SSL for Thunderbird's database */
                    let url = format!("https://autoconfig.thunderbird.net/v1.1/{}", param_domain);
                    param_autoconfig = moz_autoconfigure(context, &url, &param).ok();
                }
                true
            }
            /* C.  Do we have any autoconfig result?
               If you change the match-number here, also update STEP_12_COPY_AUTOCONFIG above
            */
            STEP_12_USE_AUTOCONFIG => {
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
                    /* although param_autoconfig's data are no longer needed from,
                    it is used to later to prevent trying variations of port/server/logins */
                }
                param.server_flags |= keep_flags;
                true
            }
            // Step 3: Fill missing fields with defaults
            // If you change the match-number here, also update STEP_13_AFTER_AUTOCONFIG above
            STEP_13_AFTER_AUTOCONFIG => {
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
                let create_mvbox = context.get_config_bool(Config::MvboxWatch)
                    || context.get_config_bool(Config::MvboxMove);
                let imap = &context.inbox_thread.read().unwrap().imap;
                if let Err(err) = imap.ensure_configured_folders(context, create_mvbox) {
                    warn!(context, "configuring folders failed: {:?}", err);
                    false
                } else {
                    let res = imap.select_with_uidvalidity(context, "INBOX");
                    if let Err(err) = res {
                        error!(context, "could not read INBOX status: {:?}", err);
                        false
                    } else {
                        true
                    }
                }
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
                info!(context, "key generation completed");
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
        context
            .inbox_thread
            .read()
            .unwrap()
            .imap
            .disconnect(context);
    }
    if smtp_connected_here {
        context.smtp.clone().lock().unwrap().disconnect();
    }

    // remember the entered parameters on success
    // and restore to last-entered on failure.
    // this way, the parameters visible to the ui are always in-sync with the current configuration.
    if success {
        LoginParam::from_database(context, "").save_to_database(context, "configured_raw_");
    } else {
        LoginParam::from_database(context, "configured_raw_").save_to_database(context, "");
    }

    if let Some(provider) = provider::get_provider_info(&param.addr) {
        if !provider.after_login_hint.is_empty() {
            let mut msg = Message::new(Viewtype::Text);
            msg.text = Some(provider.after_login_hint.to_string());
            if chat::add_device_msg(context, Some("core-provider-info"), Some(&mut msg)).is_err() {
                warn!(context, "cannot add after_login_hint as core-provider-info");
            }
        }
    }

    context.free_ongoing();
    progress!(context, if success { 1000 } else { 0 });
    job::Status::Finished(Ok(()))
}

#[allow(clippy::unnecessary_unwrap)]
fn get_offline_autoconfig(context: &Context, param: &LoginParam) -> Option<LoginParam> {
    info!(
        context,
        "checking internal provider-info for offline autoconfig"
    );

    if let Some(provider) = provider::get_provider_info(&param.addr) {
        match provider.status {
            provider::Status::OK | provider::Status::PREPARATION => {
                let imap = provider.get_imap_server();
                let smtp = provider.get_smtp_server();
                // clippy complains about these is_some()/unwrap() settings,
                // however, rewriting the code to "if let" would make things less obvious,
                // esp. if we allow more combinations of servers (pop, jmap).
                // therefore, #[allow(clippy::unnecessary_unwrap)] is added above.
                if imap.is_some() && smtp.is_some() {
                    let imap = imap.unwrap();
                    let smtp = smtp.unwrap();

                    let mut p = LoginParam::new();
                    p.addr = param.addr.clone();

                    p.mail_server = imap.hostname.to_string();
                    p.mail_user = imap.apply_username_pattern(param.addr.clone());
                    p.mail_port = imap.port as i32;
                    p.imap_certificate_checks = CertificateChecks::AcceptInvalidCertificates;
                    p.server_flags |= match imap.socket {
                        provider::Socket::STARTTLS => DC_LP_IMAP_SOCKET_STARTTLS,
                        provider::Socket::SSL => DC_LP_IMAP_SOCKET_SSL,
                    };

                    p.send_server = smtp.hostname.to_string();
                    p.send_user = smtp.apply_username_pattern(param.addr.clone());
                    p.send_port = smtp.port as i32;
                    p.smtp_certificate_checks = CertificateChecks::AcceptInvalidCertificates;
                    p.server_flags |= match smtp.socket {
                        provider::Socket::STARTTLS => DC_LP_SMTP_SOCKET_STARTTLS as i32,
                        provider::Socket::SSL => DC_LP_SMTP_SOCKET_SSL as i32,
                    };

                    info!(context, "offline autoconfig found: {}", p);
                    return Some(p);
                } else {
                    info!(context, "offline autoconfig found, but no servers defined");
                    return None;
                }
            }
            provider::Status::BROKEN => {
                info!(context, "offline autoconfig found, provider is broken");
                return None;
            }
        }
    }
    info!(context, "no offline autoconfig found");
    None
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
        "imap: {}@{}:{} flags=0x{:x} certificate_checks={}",
        param.mail_user,
        param.mail_server,
        param.mail_port,
        param.server_flags,
        param.imap_certificate_checks
    );
    info!(context, "Trying: {}", inf);
    if task::block_on(
        context
            .inbox_thread
            .read()
            .unwrap()
            .imap
            .connect(context, &param),
    ) {
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
    match context
        .smtp
        .clone()
        .lock()
        .unwrap()
        .connect(context, &param)
    {
        Ok(()) => {
            info!(context, "success: {}", inf);
            Some(true)
        }
        Err(err) => {
            if context.shall_stop_ongoing() {
                Some(false)
            } else {
                warn!(context, "could not connect: {}", err);
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::config::*;
    use crate::configure::JobConfigureImap;
    use crate::test_utils::*;

    #[test]
    fn test_no_panic_on_bad_credentials() {
        let t = dummy_context();
        t.ctx
            .set_config(Config::Addr, Some("probably@unexistant.addr"))
            .unwrap();
        t.ctx.set_config(Config::MailPw, Some("123456")).unwrap();
        JobConfigureImap(&t.ctx);
    }

    #[test]
    fn test_get_offline_autoconfig() {
        let context = dummy_context().ctx;

        let mut params = LoginParam::new();
        params.addr = "someone123@example.org".to_string();
        assert!(get_offline_autoconfig(&context, &params).is_none());

        let mut params = LoginParam::new();
        params.addr = "someone123@nauta.cu".to_string();
        let found_params = get_offline_autoconfig(&context, &params).unwrap();
        assert_eq!(found_params.mail_server, "imap.nauta.cu".to_string());
        assert_eq!(found_params.send_server, "smtp.nauta.cu".to_string());
    }
}
