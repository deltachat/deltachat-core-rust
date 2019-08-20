use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};

use crate::constants::Event;
use crate::context::Context;
use crate::dc_e2ee::*;
use crate::dc_loginparam::*;
use crate::dc_tools::*;
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
    if 0 != dc_has_ongoing(context) {
        warn!(
            context,
            0, "There is already another ongoing process running.",
        );
        return;
    }
    job_kill_action(context, Action::ConfigureImap);
    job_add(context, Action::ConfigureImap, 0, Params::new(), 0);
}

unsafe fn dc_has_ongoing(context: &Context) -> libc::c_int {
    let s_a = context.running_state.clone();
    let s = s_a.read().unwrap();

    if s.ongoing_running || !s.shall_stop_ongoing {
        1
    } else {
        0
    }
}
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

// the other dc_job_do_DC_JOB_*() functions are declared static in the c-file
#[allow(non_snake_case, unused_must_use)]
pub unsafe fn dc_job_do_DC_JOB_CONFIGURE_IMAP(context: &Context, _job: &Job) {
    let flags: libc::c_int;
    let mut success = false;
    let mut imap_connected_here = false;
    let mut smtp_connected_here = false;
    let mut ongoing_allocated_here = false;

    let mut param_autoconfig = None;
    if !(0 == dc_alloc_ongoing(context)) {
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

            if !s.shall_stop_ongoing {
                progress!(context, 1);

                let mut param = dc_loginparam_read(context, &context.sql, "");
                if param.addr.is_empty() {
                    error!(context, 0, "Please enter an email address.",);
                } else {
                    let ok_to_continue0;
                    if 0 != param.server_flags & 0x2 {
                        // the used oauth2 addr may differ, check this.
                        // if dc_get_oauth2_addr() is not available in the oauth2 implementation,
                        // just use the given one.
                        if s.shall_stop_ongoing {
                            ok_to_continue0 = false;
                        } else {
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
                            if s.shall_stop_ongoing {
                                ok_to_continue0 = false;
                            } else {
                                progress!(context, 20);
                                ok_to_continue0 = true;
                            }
                        }
                    } else {
                        ok_to_continue0 = true;
                    }
                    if ok_to_continue0 {
                        let parsed: Result<EmailAddress, _> = param.addr.parse();
                        let mut ok_to_continue7 = false;
                        if parsed.is_err() {
                            error!(context, 0, "Bad email-address.");
                        } else {
                            let parsed = parsed.unwrap();
                            let param_domain = parsed.domain;
                            let param_addr_urlencoded =
                                utf8_percent_encode(&param.addr, NON_ALPHANUMERIC).to_string();

                            if !s.shall_stop_ongoing {
                                progress!(context, 200);
                                /* 2.  Autoconfig
                                 **************************************************************************/
                                if param.mail_server.is_empty()
                                    && param.mail_port == 0
                                    && param.send_server.is_empty()
                                    && param.send_port == 0
                                    && param.send_user.is_empty()
                                    && param.server_flags & !0x2 == 0
                                {
                                    let ok_to_continue1;
                                    /*&&param->mail_user   ==NULL -- the user can enter a loginname which is used by autoconfig then */
                                    /*&&param->send_pw     ==NULL -- the password cannot be auto-configured and is no criterion for autoconfig or not */
                                    /* flags but OAuth2 avoid autoconfig */
                                    let keep_flags = param.server_flags & 0x2;
                                    /* A.  Search configurations from the domain used in the email-address, prefer encrypted */
                                    if param_autoconfig.is_none() {
                                        let url = format!(
                                            "https://autoconfig.{}/mail/config-v1.1.xml?emailaddress={}",
                                            param_domain,
                                            param_addr_urlencoded
                                        );
                                        param_autoconfig = moz_autoconfigure(context, &url, &param);
                                        if s.shall_stop_ongoing {
                                            ok_to_continue1 = false;
                                        } else {
                                            progress!(context, 300);
                                            ok_to_continue1 = true;
                                        }
                                    } else {
                                        ok_to_continue1 = true;
                                    }
                                    if ok_to_continue1 {
                                        let ok_to_continue2;
                                        if param_autoconfig.is_none() {
                                            // the doc does not mention `emailaddress=`, however, Thunderbird adds it, see https://releases.mozilla.org/pub/thunderbird/ ,  which makes some sense
                                            let url = format!(
                                                    "https://{}/.well-known/autoconfig/mail/config-v1.1.xml?emailaddress={}",
                                                    param_domain,
                                                    param_addr_urlencoded
                                                );
                                            param_autoconfig =
                                                moz_autoconfigure(context, &url, &param);
                                            if s.shall_stop_ongoing {
                                                ok_to_continue2 = false;
                                            } else {
                                                progress!(context, 310);
                                                ok_to_continue2 = true;
                                            }
                                        } else {
                                            ok_to_continue2 = true;
                                        }
                                        if ok_to_continue2 {
                                            let mut i: libc::c_int = 0;
                                            let ok_to_continue3;
                                            loop {
                                                if !(i <= 1) {
                                                    ok_to_continue3 = true;
                                                    break;
                                                }
                                                if param_autoconfig.is_none() {
                                                    /* Outlook uses always SSL but different domains */
                                                    let url = format!(
                                                                    "https://{}{}/autodiscover/autodiscover.xml",
                                                                    if i == 0 {
                                                                        ""
                                                                    } else {
                                                                        "autodiscover."
                                                                    },
                                                                    param_domain
                                                                );
                                                    param_autoconfig =
                                                        outlk_autodiscover(context, &url, &param);

                                                    if s.shall_stop_ongoing {
                                                        ok_to_continue3 = false;
                                                        break;
                                                    }
                                                    progress!(context, 320 + i * 10);
                                                }
                                                i += 1
                                            }
                                            if ok_to_continue3 {
                                                let ok_to_continue4;
                                                if param_autoconfig.is_none() {
                                                    let url = format!(
                                                                    "http://autoconfig.{}/mail/config-v1.1.xml?emailaddress={}",
                                                                    param_domain,
                                                                    param_addr_urlencoded
                                                                );
                                                    param_autoconfig =
                                                        moz_autoconfigure(context, &url, &param);

                                                    if s.shall_stop_ongoing {
                                                        ok_to_continue4 = false;
                                                    } else {
                                                        progress!(context, 340);
                                                        ok_to_continue4 = true;
                                                    }
                                                } else {
                                                    ok_to_continue4 = true;
                                                }
                                                if ok_to_continue4 {
                                                    let ok_to_continue5;
                                                    if param_autoconfig.is_none() {
                                                        // do not transfer the email-address unencrypted
                                                        let url = format!(
                                                                                    "http://{}/.well-known/autoconfig/mail/config-v1.1.xml",
                                                                                    param_domain
                                                                                );
                                                        param_autoconfig = moz_autoconfigure(
                                                            context, &url, &param,
                                                        );
                                                        if s.shall_stop_ongoing {
                                                            ok_to_continue5 = false;
                                                        } else {
                                                            progress!(context, 350);
                                                            ok_to_continue5 = true;
                                                        }
                                                    } else {
                                                        ok_to_continue5 = true;
                                                    }
                                                    if ok_to_continue5 {
                                                        let ok_to_continue6;
                                                        /* B.  If we have no configuration yet, search configuration in Thunderbird's centeral database */
                                                        if param_autoconfig.is_none() {
                                                            /* always SSL for Thunderbird's database */
                                                            let url =
                                                                                    format!("https://autoconfig.thunderbird.net/v1.1/{}",
                                                                                            param_domain
                                                                                        );
                                                            param_autoconfig = moz_autoconfigure(
                                                                context, &url, &param,
                                                            );
                                                            if s.shall_stop_ongoing {
                                                                ok_to_continue6 = false;
                                                            } else {
                                                                progress!(context, 500);
                                                                ok_to_continue6 = true;
                                                            }
                                                        } else {
                                                            ok_to_continue6 = true;
                                                        }
                                                        if ok_to_continue6 {
                                                            if let Some(ref cfg) = param_autoconfig
                                                            {
                                                                let r =
                                                                    dc_loginparam_get_readable(cfg);
                                                                info!(
                                                                    context,
                                                                    0, "Got autoconfig: {}", r
                                                                );
                                                                if !cfg.mail_user.is_empty() {
                                                                    param.mail_user =
                                                                        cfg.mail_user.clone();
                                                                }
                                                                param.mail_server =
                                                                    cfg.mail_server.clone();
                                                                param.mail_port = cfg.mail_port;
                                                                param.send_server =
                                                                    cfg.send_server.clone();
                                                                param.send_port = cfg.send_port;
                                                                param.send_user =
                                                                    cfg.send_user.clone();
                                                                param.server_flags =
                                                                    cfg.server_flags;
                                                            }
                                                            param.server_flags |= keep_flags;
                                                            ok_to_continue7 = true;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            } else {
                                ok_to_continue7 = true;
                            }
                            if ok_to_continue7 {
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
                                        param.send_server =
                                            param.send_server.replacen("imap", "smtp", 1);
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
                                if !dc_exactly_one_bit_set(
                                    param.server_flags & (0x100 | 0x200 | 0x400),
                                ) {
                                    param.server_flags &= !(0x100 | 0x200 | 0x400);
                                    param.server_flags |=
                                        if param.send_port == 143 { 0x100 } else { 0x200 }
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
                                    error!(context, 0, "Account settings incomplete.",);
                                } else if !s.shall_stop_ongoing {
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
                                            param.mail_user =
                                                param.mail_user.split_at(at).0.to_string();
                                        }
                                        if let Some(at) = param.send_user.find('@') {
                                            param.send_user =
                                                param.send_user.split_at(at).0.to_string();
                                        }

                                        username_variation += 1
                                    }
                                    if ok_to_continue8 {
                                        imap_connected_here = true;
                                        if !s.shall_stop_ongoing {
                                            progress!(context, 800);
                                            let ok_to_continue9;
                                            /* try to connect to SMTP - if we did not got an autoconfig, the first try was SSL-465 and we do a second try with STARTTLS-587 */
                                            if !context
                                                .smtp
                                                .clone()
                                                .lock()
                                                .unwrap()
                                                .connect(context, &param)
                                            {
                                                if !param_autoconfig.is_none() {
                                                    ok_to_continue9 = false;
                                                } else if s.shall_stop_ongoing {
                                                    ok_to_continue9 = false;
                                                } else {
                                                    progress!(context, 850);
                                                    param.server_flags &=
                                                        !(0x10000 | 0x20000 | 0x40000);
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
                                                            ok_to_continue9 = false;
                                                        } else {
                                                            progress!(context, 860);
                                                            param.server_flags &=
                                                                !(0x10000 | 0x20000 | 0x40000);
                                                            param.server_flags |= 0x10000;
                                                            param.send_port = 25;
                                                            let r_4 =
                                                                dc_loginparam_get_readable(&param);
                                                            info!(context, 0, "Trying: {}", r_4);

                                                            if !context
                                                                .smtp
                                                                .clone()
                                                                .lock()
                                                                .unwrap()
                                                                .connect(context, &param)
                                                            {
                                                                ok_to_continue9 = false;
                                                            } else {
                                                                ok_to_continue9 = true;
                                                            }
                                                        }
                                                    } else {
                                                        ok_to_continue9 = true;
                                                    }
                                                }
                                            } else {
                                                ok_to_continue9 = true;
                                            }
                                            if ok_to_continue9 {
                                                smtp_connected_here = true;
                                                if !s.shall_stop_ongoing {
                                                    progress!(context, 900);
                                                    flags = if 0
                                                        != context
                                                            .sql
                                                            .get_config_int(context, "mvbox_watch")
                                                            .unwrap_or_else(|| 1)
                                                        || 0 != context
                                                            .sql
                                                            .get_config_int(context, "mvbox_move")
                                                            .unwrap_or_else(|| 1)
                                                    {
                                                        0x1
                                                    } else {
                                                        0
                                                    };

                                                    context
                                                        .inbox
                                                        .read()
                                                        .unwrap()
                                                        .configure_folders(context, flags);
                                                    if !s.shall_stop_ongoing {
                                                        progress!(context, 910);
                                                        dc_loginparam_write(
                                                            context,
                                                            &param,
                                                            &context.sql,
                                                            "configured_",
                                                        );
                                                        context
                                                            .sql
                                                            .set_config_int(
                                                                context,
                                                                "configured",
                                                                1,
                                                            )
                                                            .ok();
                                                        if !s.shall_stop_ongoing {
                                                            progress!(context, 920);
                                                            dc_ensure_secret_key_exists(context);
                                                            success = true;
                                                            info!(
                                                                context,
                                                                0, "Configure completed."
                                                            );
                                                            if !s.shall_stop_ongoing {
                                                                progress!(context, 940);
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
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

pub unsafe fn dc_free_ongoing(context: &Context) {
    let s_a = context.running_state.clone();
    let mut s = s_a.write().unwrap();

    s.ongoing_running = false;
    s.shall_stop_ongoing = true;
}

pub unsafe fn dc_alloc_ongoing(context: &Context) -> libc::c_int {
    if 0 != dc_has_ongoing(context) {
        warn!(
            context,
            0, "There is already another ongoing process running.",
        );
        return 0;
    }
    let s_a = context.running_state.clone();
    let mut s = s_a.write().unwrap();

    s.ongoing_running = true;
    s.shall_stop_ongoing = false;

    1
}

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

pub fn read_autoconf_file(context: &Context, url: *const libc::c_char) -> *mut libc::c_char {
    info!(context, 0, "Testing {} ...", to_string(url));

    match reqwest::Client::new()
        .get(as_str(url))
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
