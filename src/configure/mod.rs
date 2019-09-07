use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};

use crate::constants::Event;
use crate::constants::DC_CREATE_MVBOX;
use crate::context::Context;
use crate::dc_loginparam::*;
use crate::dc_tools::*;
use crate::e2ee;
use crate::imap::*;
use crate::job::*;
use crate::oauth2::*;
use crate::param::Params;
use crate::types::*;

mod auto_outlook;
use auto_outlook::outlk_autodiscover;
mod auto_mozilla;
use auto_mozilla::moz_autoconfigure;

macro_rules! progress {
    ($context:tt, $progress:expr) => {
        assert!(
            $progress >= 0 && $progress <= 1000,
            "value in range 0..1000 expected with: 0=error, 1..999=progress, 1000=success"
        );
        $context.call_cb(
            Event::CONFIGURE_PROGRESS,
            $progress as uintptr_t,
            0 as uintptr_t,
        );
    };
}

// connect
pub unsafe fn configure(context: &Context) {
    if dc_has_ongoing(context) {
        warn!(
            context,
            0, "There is already another ongoing process running.",
        );
        return;
    }
    job_kill_action(context, Action::ConfigureImap);
    job_add(context, Action::ConfigureImap, 0, Params::new(), 0);
}

/// Check if the context is already configured.
pub fn dc_is_configured(context: &Context) -> libc::c_int {
    if context
        .sql
        .get_config_int(context, "configured")
        .unwrap_or_default()
        > 0
    {
        1
    } else {
        0
    }
}

/*******************************************************************************
 * Configure JOB
 ******************************************************************************/
// the other dc_job_do_DC_JOB_*() functions are declared static in the c-file
#[allow(non_snake_case, unused_must_use)]
pub unsafe fn dc_job_do_DC_JOB_CONFIGURE_IMAP(context: &Context, _job: &Job) {
    let mut success = false;
    let mut imap_connected_here = false;
    let mut smtp_connected_here = false;
    let mut ongoing_allocated_here = false;

    let mut param_autoconfig: Option<dc_loginparam_t> = None;
    if dc_alloc_ongoing(context) {
        ongoing_allocated_here = true;
        if !context.sql.is_open() {
            error!(context, 0, "Cannot configure, database not opened.",);
        } else {
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
            info!(context, 0, "Configure ...",);

            let s_a = context.running_state.clone();
            let s = s_a.read().unwrap();

            // Variables that are shared between steps:
            let mut param: dc_loginparam_t = dc_loginparam_read(context, &context.sql, "");
            // need all vars here to be mutable because rust thinks the same step could be called multiple times
            // and also initialize, because otherwise rust thinks it's used while unitilized, even if thats not the case as the loop goes only forward
            let mut param_domain = "undefined.undefined".to_owned();
            let mut param_addr_urlencoded: String =
                "Internal Error: this value should never be used".to_owned();
            let mut keep_flags = std::i32::MAX;

            const STEP_3_INDEX: u8 = 13;
            let mut step_counter: u8 = 0;
            while !s.shall_stop_ongoing {
                step_counter = step_counter + 1;

                let success = match step_counter {
                    // Read login parameters from the database
                    1 => {
                        progress!(context, 1);
                        if param.addr.is_empty() {
                            error!(context, 0, "Please enter an email address.",);
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
                                    .set_config(context, "addr", Some(param.addr.as_str()))
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
                            error!(context, 0, "Bad email-address.");
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
                                            param_domain,
                                            param_addr_urlencoded
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
                            let url =
                                format!("https://autoconfig.thunderbird.net/v1.1/{}", param_domain);
                            param_autoconfig = moz_autoconfigure(context, &url, &param);
                        }
                        true
                    }
                    /* C.  Do we have any result? */
                    12 => {
                        progress!(context, 500);
                        if let Some(ref cfg) = param_autoconfig {
                            let r = dc_loginparam_get_readable(cfg);
                            info!(context, 0, "Got autoconfig: {}", r);
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
                            param.send_port = if 0 != param.server_flags & 0x10000 {
                                587
                            } else if 0 != param.server_flags & 0x40000 {
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
                        if !dc_exactly_one_bit_set(param.server_flags & (0x2 | 0x4)) {
                            param.server_flags &= !(0x2 | 0x4);
                            param.server_flags |= 0x4
                        }
                        if !dc_exactly_one_bit_set(param.server_flags & (0x100 | 0x200 | 0x400)) {
                            param.server_flags &= !(0x100 | 0x200 | 0x400);
                            param.server_flags |= if param.send_port == 143 { 0x100 } else { 0x200 }
                        }
                        if !dc_exactly_one_bit_set(
                            param.server_flags & (0x10000 | 0x20000 | 0x40000),
                        ) {
                            param.server_flags &= !(0x10000 | 0x20000 | 0x40000);
                            param.server_flags |= if param.send_port == 587 {
                                0x10000
                            } else if param.send_port == 25 {
                                0x40000
                            } else {
                                0x20000
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
                            error!(context, 0, "Account settings incomplete.");
                            false
                        } else {
                            true
                        }
                    }
                    14 => {
                        progress!(context, 600);
                        /* try to connect to IMAP - if we did not got an autoconfig,
                        do some further tries with different settings and username variations */
                        let ok_to_continue8;
                        let mut username_variation = 0;
                        loop {
                            if !(username_variation <= 1) {
                                ok_to_continue8 = true;
                                break;
                            }
                            let r_0 = dc_loginparam_get_readable(&param);
                            info!(context, 0, "Trying: {}", r_0,);

                            if context.inbox.read().unwrap().connect(context, &param) {
                                ok_to_continue8 = true;
                                break;
                            }
                            if !param_autoconfig.is_none() {
                                ok_to_continue8 = false;
                                break;
                            }
                            // probe STARTTLS/993
                            if s.shall_stop_ongoing {
                                ok_to_continue8 = false;
                                break;
                            }
                            progress!(context, 650 + username_variation * 30);
                            param.server_flags &= !(0x100 | 0x200 | 0x400);
                            param.server_flags |= 0x100;
                            let r_1 = dc_loginparam_get_readable(&param);
                            info!(context, 0, "Trying: {}", r_1,);

                            if context.inbox.read().unwrap().connect(context, &param) {
                                ok_to_continue8 = true;
                                break;
                            }
                            // probe STARTTLS/143
                            if s.shall_stop_ongoing {
                                ok_to_continue8 = false;
                                break;
                            }
                            progress!(context, 660 + username_variation * 30);
                            param.mail_port = 143;
                            let r_2 = dc_loginparam_get_readable(&param);
                            info!(context, 0, "Trying: {}", r_2,);

                            if context.inbox.read().unwrap().connect(context, &param) {
                                ok_to_continue8 = true;
                                break;
                            }
                            if 0 != username_variation {
                                ok_to_continue8 = false;
                                break;
                            }
                            // next probe round with only the localpart of the email-address as the loginname
                            if s.shall_stop_ongoing {
                                ok_to_continue8 = false;
                                break;
                            }
                            progress!(context, 670 + username_variation * 30);
                            param.server_flags &= !(0x100 | 0x200 | 0x400);
                            param.server_flags |= 0x200;
                            param.mail_port = 993;

                            if let Some(at) = param.mail_user.find('@') {
                                param.mail_user = param.mail_user.split_at(at).0.to_string();
                            }
                            if let Some(at) = param.send_user.find('@') {
                                param.send_user = param.send_user.split_at(at).0.to_string();
                            }

                            username_variation += 1
                        }
                        if ok_to_continue8 {
                            // success, so we are connected and should disconnect in cleanup
                            imap_connected_here = true;
                        }
                        ok_to_continue8
                    }
                    15 => {
                        progress!(context, 800);
                        let success;
                        /* try to connect to SMTP - if we did not got an autoconfig, the first try was SSL-465 and we do a second try with STARTTLS-587 */
                        if !context
                            .smtp
                            .clone()
                            .lock()
                            .unwrap()
                            .connect(context, &param)
                        {
                            if !param_autoconfig.is_none() {
                                success = false;
                            } else if s.shall_stop_ongoing {
                                success = false;
                            } else {
                                progress!(context, 850);
                                param.server_flags &= !(0x10000 | 0x20000 | 0x40000);
                                param.server_flags |= 0x10000;
                                param.send_port = 587;
                                let r_3 = dc_loginparam_get_readable(&param);
                                info!(context, 0, "Trying: {}", r_3,);

                                if !context
                                    .smtp
                                    .clone()
                                    .lock()
                                    .unwrap()
                                    .connect(context, &param)
                                {
                                    if s.shall_stop_ongoing {
                                        success = false;
                                    } else {
                                        progress!(context, 860);
                                        param.server_flags &= !(0x10000 | 0x20000 | 0x40000);
                                        param.server_flags |= 0x10000;
                                        param.send_port = 25;
                                        let r_4 = dc_loginparam_get_readable(&param);
                                        info!(context, 0, "Trying: {}", r_4);

                                        if !context
                                            .smtp
                                            .clone()
                                            .lock()
                                            .unwrap()
                                            .connect(context, &param)
                                        {
                                            success = false;
                                        } else {
                                            success = true;
                                        }
                                    }
                                } else {
                                    success = true;
                                }
                            }
                        } else {
                            success = true;
                        }
                        if success {
                            smtp_connected_here = true;
                        }
                        success
                    }
                    16 => {
                        progress!(context, 900);
                        let flags: libc::c_int = if 0
                            != context
                                .sql
                                .get_config_int(context, "mvbox_watch")
                                .unwrap_or_else(|| 1)
                            || 0 != context
                                .sql
                                .get_config_int(context, "mvbox_move")
                                .unwrap_or_else(|| 1)
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
                        dc_loginparam_write(
                            context,
                            &param,
                            &context.sql,
                            "configured_", /*the trailing underscore is correct*/
                        );
                        context.sql.set_config_int(context, "configured", 1).ok();
                        true
                    }
                    18 => {
                        progress!(context, 920);
                        // we generate the keypair just now - we could also postpone this until the first message is sent, however,
                        // this may result in a unexpected and annoying delay when the user sends his very first message
                        // (~30 seconds on a Moto G4 play) and might looks as if message sending is always that slow.
                        e2ee::ensure_secret_key_exists(context);
                        success = true;
                        info!(context, 0, "Configure completed.");
                        progress!(context, 940);
                        break; // We are done here
                    }

                    _ => {
                        error!(context, 0, "Internal error: step counter out of bound",);
                        break;
                    }
                };

                if !success {
                    break;
                }
            }
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
    if ongoing_allocated_here {
        dc_free_ongoing(context);
    }

    progress!(context, (if success { 1000 } else { 0 }));
}

/*******************************************************************************
 * Ongoing process allocation/free/check
 ******************************************************************************/

pub fn dc_alloc_ongoing(context: &Context) -> bool {
    if dc_has_ongoing(context) {
        warn!(
            context,
            0, "There is already another ongoing process running.",
        );

        false
    } else {
        let s_a = context.running_state.clone();
        let mut s = s_a.write().unwrap();

        s.ongoing_running = true;
        s.shall_stop_ongoing = false;

        true
    }
}

pub fn dc_free_ongoing(context: &Context) {
    let s_a = context.running_state.clone();
    let mut s = s_a.write().unwrap();

    s.ongoing_running = false;
    s.shall_stop_ongoing = true;
}

fn dc_has_ongoing(context: &Context) -> bool {
    let s_a = context.running_state.clone();
    let s = s_a.read().unwrap();

    s.ongoing_running || !s.shall_stop_ongoing
}

/*******************************************************************************
 * Connect to configured account
 ******************************************************************************/
pub fn dc_connect_to_configured_imap(context: &Context, imap: &Imap) -> libc::c_int {
    let mut ret_connected = 0;

    if imap.is_connected() {
        ret_connected = 1
    } else if context
        .sql
        .get_config_int(context, "configured")
        .unwrap_or_default()
        == 0
    {
        warn!(context, 0, "Not configured, cannot connect.",);
    } else {
        let param = dc_loginparam_read(context, &context.sql, "configured_");
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

/// Signal an ongoing process to stop.
pub fn dc_stop_ongoing_process(context: &Context) {
    let s_a = context.running_state.clone();
    let mut s = s_a.write().unwrap();

    if s.ongoing_running && !s.shall_stop_ongoing {
        info!(context, 0, "Signaling the ongoing process to stop ASAP.",);
        s.shall_stop_ongoing = true;
    } else {
        info!(context, 0, "No ongoing process to stop.",);
    };
}

pub fn read_autoconf_file(context: &Context, url: &str) -> *mut libc::c_char {
    info!(context, 0, "Testing {} ...", url);

    match reqwest::Client::new()
        .get(url)
        .send()
        .and_then(|mut res| res.text())
    {
        Ok(res) => unsafe { res.strdup() },
        Err(_err) => {
            info!(context, 0, "Can\'t read file.",);

            std::ptr::null_mut()
        }
    }
}
