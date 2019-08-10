use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};

use crate::constants::Event;
use crate::context::Context;
use crate::dc_e2ee::*;
use crate::dc_job::*;
use crate::dc_loginparam::*;
use crate::dc_saxparser::*;
use crate::dc_tools::*;
use crate::imap::*;
use crate::oauth2::*;
use crate::param::Params;
use crate::types::*;
use crate::x::*;

/* ******************************************************************************
 * Configure folders
 ******************************************************************************/
#[derive(Copy, Clone)]
#[repr(C)]
pub struct dc_imapfolder_t {
    pub name_to_select: *mut libc::c_char,
    pub name_utf8: *mut libc::c_char,
    pub meaning: libc::c_int,
}
/* ******************************************************************************
 * Thunderbird's Autoconfigure
 ******************************************************************************/
/* documentation: https://developer.mozilla.org/en-US/docs/Mozilla/Thunderbird/Autoconfiguration */
#[repr(C)]
pub struct moz_autoconfigure_t<'a> {
    pub in_0: &'a dc_loginparam_t,
    pub in_emaildomain: *mut libc::c_char,
    pub in_emaillocalpart: *mut libc::c_char,
    pub out: dc_loginparam_t,
    pub out_imap_set: libc::c_int,
    pub out_smtp_set: libc::c_int,
    pub tag_server: libc::c_int,
    pub tag_config: libc::c_int,
}

/* ******************************************************************************
 * Outlook's Autodiscover
 ******************************************************************************/
#[repr(C)]
pub struct outlk_autodiscover_t<'a> {
    pub in_0: &'a dc_loginparam_t,
    pub out: dc_loginparam_t,
    pub out_imap_set: libc::c_int,
    pub out_smtp_set: libc::c_int,
    pub tag_config: libc::c_int,
    pub config: [*mut libc::c_char; 6],
    pub redirect: *mut libc::c_char,
}
// connect
pub unsafe fn dc_configure(context: &Context) {
    if 0 != dc_has_ongoing(context) {
        warn!(
            context,
            0, "There is already another ongoing process running.",
        );
        return;
    }
    dc_job_kill_action(context, 900);
    dc_job_add(context, 900, 0, Params::new(), 0);
}

pub unsafe fn dc_has_ongoing(context: &Context) -> libc::c_int {
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
pub unsafe fn dc_job_do_DC_JOB_CONFIGURE_IMAP(context: &Context, _job: *mut dc_job_t) {
    let flags: libc::c_int;
    let mut current_block: u64;
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
                context.call_cb(
                    Event::CONFIGURE_PROGRESS,
                    (if 0i32 < 1i32 {
                        1i32
                    } else if 0i32 > 999i32 {
                        999i32
                    } else {
                        0i32
                    }) as uintptr_t,
                    0i32 as uintptr_t,
                );

                let mut param = dc_loginparam_read(context, &context.sql, "");
                if param.addr.is_empty() {
                    error!(context, 0, "Please enter an email address.",);
                } else {
                    if 0 != param.server_flags & 0x2 {
                        // the used oauth2 addr may differ, check this.
                        // if dc_get_oauth2_addr() is not available in the oauth2 implementation,
                        // just use the given one.
                        if s.shall_stop_ongoing {
                            current_block = 2927484062889439186;
                        } else {
                            context.call_cb(
                                Event::CONFIGURE_PROGRESS,
                                (if 10 < 1 {
                                    1
                                } else if 10 > 999 {
                                    999
                                } else {
                                    10
                                }) as uintptr_t,
                                0 as uintptr_t,
                            );
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
                                current_block = 2927484062889439186;
                            } else {
                                context.call_cb(
                                    Event::CONFIGURE_PROGRESS,
                                    (if 20 < 1 {
                                        1
                                    } else if 20 > 999 {
                                        999
                                    } else {
                                        20
                                    }) as uintptr_t,
                                    0 as uintptr_t,
                                );
                                current_block = 7746103178988627676;
                            }
                        }
                    } else {
                        current_block = 7746103178988627676;
                    }
                    match current_block {
                        2927484062889439186 => {}
                        _ => {
                            let parsed: addr::Result<addr::Email> = param.addr.parse();
                            if parsed.is_err() {
                                error!(context, 0, "Bad email-address.");
                            } else {
                                let parsed = parsed.unwrap();
                                let param_domain = parsed.host();
                                let param_addr_urlencoded =
                                    utf8_percent_encode(&param.addr, NON_ALPHANUMERIC).to_string();

                                if !s.shall_stop_ongoing {
                                    context.call_cb(
                                        Event::CONFIGURE_PROGRESS,
                                        (if 200 < 1 {
                                            1
                                        } else if 200 > 999 {
                                            999
                                        } else {
                                            200
                                        }) as uintptr_t,
                                        0 as uintptr_t,
                                    );
                                    /* 2.  Autoconfig
                                     **************************************************************************/
                                    if param.mail_server.is_empty()
                                        && param.mail_port == 0
                                        && param.send_server.is_empty()
                                        && param.send_port == 0
                                        && param.send_user.is_empty()
                                        && param.server_flags & !0x2 == 0
                                    {
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
                                            param_autoconfig =
                                                moz_autoconfigure(context, &url, &param);
                                            if s.shall_stop_ongoing {
                                                current_block = 2927484062889439186;
                                            } else {
                                                context.call_cb(
                                                    Event::CONFIGURE_PROGRESS,
                                                    (if 300 < 1 {
                                                        1
                                                    } else if 300 > 999 {
                                                        999
                                                    } else {
                                                        300
                                                    })
                                                        as uintptr_t,
                                                    0 as uintptr_t,
                                                );
                                                current_block = 13325891313334703151;
                                            }
                                        } else {
                                            current_block = 13325891313334703151;
                                        }
                                        match current_block {
                                            2927484062889439186 => {}
                                            _ => {
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
                                                        current_block = 2927484062889439186;
                                                    } else {
                                                        context.call_cb(
                                                            Event::CONFIGURE_PROGRESS,
                                                            (if 310 < 1 {
                                                                1
                                                            } else if 310 > 999 {
                                                                999
                                                            } else {
                                                                310
                                                            })
                                                                as uintptr_t,
                                                            0 as uintptr_t,
                                                        );
                                                        current_block = 5597585068398118923;
                                                    }
                                                } else {
                                                    current_block = 5597585068398118923;
                                                }
                                                match current_block {
                                                    2927484062889439186 => {}
                                                    _ => {
                                                        let mut i: libc::c_int = 0;
                                                        loop {
                                                            if !(i <= 1) {
                                                                current_block =
                                                                    12961834331865314435;
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
                                                                    outlk_autodiscover(
                                                                        context, &url, &param,
                                                                    );

                                                                if s.shall_stop_ongoing {
                                                                    current_block =
                                                                        2927484062889439186;
                                                                    break;
                                                                }
                                                                context.call_cb(
                                                                    Event::CONFIGURE_PROGRESS,
                                                                    (if 320 + i * 10 < 1 {
                                                                        1
                                                                    } else if 320 + i * 10 > 999 {
                                                                        999
                                                                    } else {
                                                                        320 + i * 10
                                                                    })
                                                                        as uintptr_t,
                                                                    0 as uintptr_t,
                                                                );
                                                            }
                                                            i += 1
                                                        }
                                                        match current_block {
                                                            2927484062889439186 => {}
                                                            _ => {
                                                                if param_autoconfig.is_none() {
                                                                    let url = format!(
                                                                    "http://autoconfig.{}/mail/config-v1.1.xml?emailaddress={}",
                                                                    param_domain,
                                                                    param_addr_urlencoded
                                                                );
                                                                    param_autoconfig =
                                                                        moz_autoconfigure(
                                                                            context, &url, &param,
                                                                        );

                                                                    if s.shall_stop_ongoing {
                                                                        current_block =
                                                                            2927484062889439186;
                                                                    } else {
                                                                        context.call_cb(
                                                                        Event::CONFIGURE_PROGRESS,
                                                                        (if 340 < 1 {
                                                                            1
                                                                        } else if 340 > 999 {
                                                                            999
                                                                        } else {
                                                                            340
                                                                        })
                                                                            as uintptr_t,
                                                                        0,
                                                                    );
                                                                        current_block =
                                                                            10778260831612459202;
                                                                    }
                                                                } else {
                                                                    current_block =
                                                                        10778260831612459202;
                                                                }
                                                                match current_block {
                                                                    2927484062889439186 => {}
                                                                    _ => {
                                                                        if param_autoconfig
                                                                            .is_none()
                                                                        {
                                                                            // do not transfer the email-address unencrypted
                                                                            let url = format!(
                                                                                    "http://{}/.well-known/autoconfig/mail/config-v1.1.xml",
                                                                                    param_domain
                                                                                );
                                                                            param_autoconfig =
                                                                                moz_autoconfigure(
                                                                                    context, &url,
                                                                                    &param,
                                                                                );
                                                                            if s.shall_stop_ongoing
                                                                            {
                                                                                current_block =
                                                                                2927484062889439186;
                                                                            } else {
                                                                                context.call_cb(
                                                                                    Event::CONFIGURE_PROGRESS,
                                                                                    if 350 < 1 {
                                                                                        1
                                                                                    } else if 350 > 999 {
                                                                                        999
                                                                                    } else {
                                                                                        350
                                                                                    },
                                                                                    0
                                                                                );
                                                                                current_block =
                                                                                5207889489643863322;
                                                                            }
                                                                        } else {
                                                                            current_block =
                                                                                5207889489643863322;
                                                                        }
                                                                        match current_block {
                                                                            2927484062889439186 => {
                                                                            }
                                                                            _ => {
                                                                                /* B.  If we have no configuration yet, search configuration in Thunderbird's centeral database */
                                                                                if param_autoconfig
                                                                                    .is_none()
                                                                                {
                                                                                    /* always SSL for Thunderbird's database */
                                                                                    let url =
                                                                                    format!("https://autoconfig.thunderbird.net/v1.1/{}",
                                                                                            param_domain
                                                                                        );
                                                                                    param_autoconfig
                                                                                    =
                                                                                    moz_autoconfigure(
                                                                                        context,
                                                                                        &url,
                                                                                        &param
                                                                                    );
                                                                                    if s.shall_stop_ongoing
                                                                                    {
                                                                                        current_block
                                                                                            =
                                                                                            2927484062889439186;
                                                                                    } else {
                                                                                        context.call_cb(
                                                                                            Event::CONFIGURE_PROGRESS,
                                                                                            if 500 < 1 {
                                                                                                 1
                                                                                             } else if 500 > 999 {
                                                                                                 999
                                                                                             } else {
                                                                                                 500
                                                                                            },
                                                                                            0);
                                                                                        current_block
                                                                                            =
                                                                                            2798392256336243897;
                                                                                    }
                                                                                } else {
                                                                                    current_block
                                                                                        =
                                                                                        2798392256336243897;
                                                                                }
                                                                                match current_block
                                                                                {
                                                                                    2927484062889439186
                                                                                        =>
                                                                                    {
                                                                                    }
                                                                                    _
                                                                                        =>
                                                                                    {
                                                                                        if let Some(ref cfg) = param_autoconfig
                                                                                        {
                                                                                            let r = dc_loginparam_get_readable(cfg);
                                                                                            info!(
                                                                                                context,
                                                                                                0,
                                                                                                "Got autoconfig: {}",
                                                                                                r
                                                                                            );
                                                                                            if !cfg.mail_user.is_empty()
                                                                                            {
                                                                                                param.mail_user = cfg.mail_user.clone();
                                                                                            }
                                                                                            param.mail_server = cfg.mail_server.clone();
                                                                                            param.mail_port
                                                                                                =
                                                                                                cfg.mail_port;
                                                                                            param.send_server
                                                                                                =
                                                                                                cfg.send_server.clone();
                                                                                            param.send_port
                                                                                                =
                                                                                                cfg.send_port;
                                                                                            param.send_user
                                                                                                =
                                                                                                cfg.send_user.clone();
                                                                                            param.server_flags
                                                                                                =
                                                                                                cfg.server_flags;
                                                                                        }
                                                                                        param.server_flags
                                                                                            |=
                                                                                            keep_flags;
                                                                                        current_block
                                                                                            =
                                                                                            3024367268842933116;
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
                                    } else {
                                        current_block = 3024367268842933116;
                                    }
                                    match current_block {
                                        2927484062889439186 => {}
                                        _ => {
                                            if param.mail_server.is_empty() {
                                                param.mail_server =
                                                    format!("imap.{}", param_domain,)
                                            }
                                            if param.mail_port == 0 {
                                                param.mail_port =
                                                    if 0 != param.server_flags & (0x100 | 0x400) {
                                                        143
                                                    } else {
                                                        993
                                                    }
                                            }
                                            if param.mail_user.is_empty() {
                                                param.mail_user = param.addr.clone();
                                            }
                                            if param.send_server.is_empty()
                                                && !param.mail_server.is_empty()
                                            {
                                                param.send_server = param.mail_server.clone();
                                                if param.send_server.starts_with("imap.") {
                                                    param.send_server = param
                                                        .send_server
                                                        .replacen("imap", "smtp", 1);
                                                }
                                            }
                                            if param.send_port == 0 {
                                                param.send_port =
                                                    if 0 != param.server_flags & 0x10000 {
                                                        587
                                                    } else if 0 != param.server_flags & 0x40000 {
                                                        25
                                                    } else {
                                                        465
                                                    }
                                            }
                                            if param.send_user.is_empty()
                                                && !param.mail_user.is_empty()
                                            {
                                                param.send_user = param.mail_user.clone();
                                            }
                                            if param.send_pw.is_empty() && !param.mail_pw.is_empty()
                                            {
                                                param.send_pw = param.mail_pw.clone()
                                            }
                                            if !dc_exactly_one_bit_set(
                                                param.server_flags & (0x2 | 0x4),
                                            ) {
                                                param.server_flags &= !(0x2 | 0x4);
                                                param.server_flags |= 0x4
                                            }
                                            if !dc_exactly_one_bit_set(
                                                param.server_flags & (0x100 | 0x200 | 0x400),
                                            ) {
                                                param.server_flags &= !(0x100 | 0x200 | 0x400);
                                                param.server_flags |= if param.send_port == 143 {
                                                    0x100
                                                } else {
                                                    0x200
                                                }
                                            }
                                            if !dc_exactly_one_bit_set(
                                                param.server_flags & (0x10000 | 0x20000 | 0x40000),
                                            ) {
                                                param.server_flags &=
                                                    !(0x10000 | 0x20000 | 0x40000);
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
                                                context.call_cb(
                                                    Event::CONFIGURE_PROGRESS,
                                                    (if 600 < 1 {
                                                        1
                                                    } else if 600 > 999 {
                                                        999
                                                    } else {
                                                        600
                                                    })
                                                        as uintptr_t,
                                                    0,
                                                );
                                                /* try to connect to IMAP - if we did not got an autoconfig,
                                                do some further tries with different settings and username variations */
                                                let mut username_variation = 0;
                                                loop {
                                                    if !(username_variation <= 1) {
                                                        current_block = 14187386403465544025;
                                                        break;
                                                    }
                                                    let r_0 = dc_loginparam_get_readable(&param);
                                                    info!(context, 0, "Trying: {}", r_0,);

                                                    if context
                                                        .inbox
                                                        .read()
                                                        .unwrap()
                                                        .connect(context, &param)
                                                    {
                                                        current_block = 14187386403465544025;
                                                        break;
                                                    }
                                                    if !param_autoconfig.is_none() {
                                                        current_block = 2927484062889439186;
                                                        break;
                                                    }
                                                    // probe STARTTLS/993
                                                    if s.shall_stop_ongoing {
                                                        current_block = 2927484062889439186;
                                                        break;
                                                    }
                                                    context.call_cb(
                                                        Event::CONFIGURE_PROGRESS,
                                                        (if 650 + username_variation * 30 < 1 {
                                                            1
                                                        } else if 650 + username_variation * 30
                                                            > 999
                                                        {
                                                            999
                                                        } else {
                                                            650 + username_variation * 30
                                                        })
                                                            as uintptr_t,
                                                        0 as uintptr_t,
                                                    );
                                                    param.server_flags &= !(0x100 | 0x200 | 0x400);
                                                    param.server_flags |= 0x100;
                                                    let r_1 = dc_loginparam_get_readable(&param);
                                                    info!(context, 0, "Trying: {}", r_1,);

                                                    if context
                                                        .inbox
                                                        .read()
                                                        .unwrap()
                                                        .connect(context, &param)
                                                    {
                                                        current_block = 14187386403465544025;
                                                        break;
                                                    }
                                                    // probe STARTTLS/143
                                                    if s.shall_stop_ongoing {
                                                        current_block = 2927484062889439186;
                                                        break;
                                                    }
                                                    context.call_cb(
                                                        Event::CONFIGURE_PROGRESS,
                                                        (if 660 + username_variation * 30 < 1 {
                                                            1
                                                        } else if 660 + username_variation * 30
                                                            > 999
                                                        {
                                                            999
                                                        } else {
                                                            660 + username_variation * 30
                                                        })
                                                            as uintptr_t,
                                                        0 as uintptr_t,
                                                    );
                                                    param.mail_port = 143;
                                                    let r_2 = dc_loginparam_get_readable(&param);
                                                    info!(context, 0, "Trying: {}", r_2,);

                                                    if context
                                                        .inbox
                                                        .read()
                                                        .unwrap()
                                                        .connect(context, &param)
                                                    {
                                                        current_block = 14187386403465544025;
                                                        break;
                                                    }
                                                    if 0 != username_variation {
                                                        current_block = 2927484062889439186;
                                                        break;
                                                    }
                                                    // next probe round with only the localpart of the email-address as the loginname
                                                    if s.shall_stop_ongoing {
                                                        current_block = 2927484062889439186;
                                                        break;
                                                    }
                                                    context.call_cb(
                                                        Event::CONFIGURE_PROGRESS,
                                                        (if 670 + username_variation * 30 < 1 {
                                                            1
                                                        } else if 670 + username_variation * 30
                                                            > 999
                                                        {
                                                            999
                                                        } else {
                                                            670 + username_variation * 30
                                                        })
                                                            as uintptr_t,
                                                        0 as uintptr_t,
                                                    );
                                                    param.server_flags &= !(0x100 | 0x200 | 0x400);
                                                    param.server_flags |= 0x200;
                                                    param.mail_port = 993;

                                                    if let Some(at) = param.mail_user.find('@') {
                                                        param.mail_user = param
                                                            .mail_user
                                                            .split_at(at)
                                                            .0
                                                            .to_string();
                                                    }
                                                    if let Some(at) = param.send_user.find('@') {
                                                        param.send_user = param
                                                            .send_user
                                                            .split_at(at)
                                                            .0
                                                            .to_string();
                                                    }

                                                    username_variation += 1
                                                }
                                                match current_block {
                                                    2927484062889439186 => {}
                                                    _ => {
                                                        imap_connected_here = true;
                                                        if !s.shall_stop_ongoing {
                                                            context.call_cb(
                                                                Event::CONFIGURE_PROGRESS,
                                                                (if 800 < 1 {
                                                                    1
                                                                } else if 800 > 999 {
                                                                    999
                                                                } else {
                                                                    800
                                                                })
                                                                    as uintptr_t,
                                                                0 as uintptr_t,
                                                            );
                                                            /* try to connect to SMTP - if we did not got an autoconfig, the first try was SSL-465 and we do a second try with STARTTLS-587 */
                                                            if !context
                                                                .smtp
                                                                .clone()
                                                                .lock()
                                                                .unwrap()
                                                                .connect(context, &param)
                                                            {
                                                                if !param_autoconfig.is_none() {
                                                                    current_block =
                                                                        2927484062889439186;
                                                                } else if s.shall_stop_ongoing {
                                                                    current_block =
                                                                        2927484062889439186;
                                                                } else {
                                                                    context.call_cb(
                                                                        Event::CONFIGURE_PROGRESS,
                                                                        (if 850 < 1 {
                                                                            1
                                                                        } else if 850 > 999 {
                                                                            999
                                                                        } else {
                                                                            850
                                                                        })
                                                                            as uintptr_t,
                                                                        0 as uintptr_t,
                                                                    );
                                                                    param.server_flags &= !(0x10000
                                                                        | 0x20000
                                                                        | 0x40000);
                                                                    param.server_flags |= 0x10000;
                                                                    param.send_port = 587;
                                                                    let r_3 =
                                                                        dc_loginparam_get_readable(
                                                                            &param,
                                                                        );
                                                                    info!(
                                                                        context,
                                                                        0, "Trying: {}", r_3,
                                                                    );

                                                                    if !context
                                                                        .smtp
                                                                        .clone()
                                                                        .lock()
                                                                        .unwrap()
                                                                        .connect(context, &param)
                                                                    {
                                                                        if s.shall_stop_ongoing {
                                                                            current_block =
                                                                                2927484062889439186;
                                                                        } else {
                                                                            context.call_cb(
                                                                                            Event::CONFIGURE_PROGRESS,
                                                                                            (if 860
                                                                                             <
                                                                                             1
                                                                                             {
                                                                                                 1
                                                                                             } else if 860
                                                                                             >
                                                                                             999
                                                                                             {
                                                                                                 999
                                                                                             } else {
                                                                                                 860
                                                                                             })
                                                                                            as
                                                                                            uintptr_t,
                                                                                            0
                                                                                            as
                                                                                            uintptr_t);
                                                                            param.server_flags &=
                                                                                !(0x10000
                                                                                    | 0x20000
                                                                                    | 0x40000);
                                                                            param.server_flags |=
                                                                                0x10000;
                                                                            param.send_port = 25;
                                                                            let r_4 = dc_loginparam_get_readable(&param);
                                                                            info!(
                                                                                context,
                                                                                0,
                                                                                "Trying: {}",
                                                                                r_4
                                                                            );

                                                                            if !context
                                                                                .smtp
                                                                                .clone()
                                                                                .lock()
                                                                                .unwrap()
                                                                                .connect(
                                                                                    context, &param,
                                                                                )
                                                                            {
                                                                                current_block =
                                                                                2927484062889439186;
                                                                            } else {
                                                                                current_block =
                                                                                5083741289379115417;
                                                                            }
                                                                        }
                                                                    } else {
                                                                        current_block =
                                                                            5083741289379115417;
                                                                    }
                                                                }
                                                            } else {
                                                                current_block = 5083741289379115417;
                                                            }
                                                            match current_block {
                                                                2927484062889439186 => {}
                                                                _ => {
                                                                    smtp_connected_here = true;
                                                                    if !s.shall_stop_ongoing {
                                                                        context.call_cb(
                                                                        Event::CONFIGURE_PROGRESS,
                                                                        (if 900 < 1 {
                                                                            1
                                                                        } else if 900 > 999 {
                                                                            999
                                                                        } else {
                                                                            900
                                                                        })
                                                                            as uintptr_t,
                                                                        0 as uintptr_t,
                                                                    );
                                                                        flags = if 0
                                                                            != context
                                                                                .sql
                                                                                .get_config_int(
                                                                                    context,
                                                                                    "mvbox_watch",
                                                                                )
                                                                                .unwrap_or_else(
                                                                                    || 1,
                                                                                )
                                                                            || 0 != context
                                                                                .sql
                                                                                .get_config_int(
                                                                                    context,
                                                                                    "mvbox_move",
                                                                                )
                                                                                .unwrap_or_else(
                                                                                    || 1,
                                                                                ) {
                                                                            0x1
                                                                        } else {
                                                                            0
                                                                        };

                                                                        context
                                                                            .inbox
                                                                            .read()
                                                                            .unwrap()
                                                                            .configure_folders(
                                                                                context, flags,
                                                                            );
                                                                        if !s.shall_stop_ongoing {
                                                                            context.call_cb(
                                                                                Event::CONFIGURE_PROGRESS,
                                                                                (if 910
                                                                                 <
                                                                                 1
                                                                                 {
                                                                                     1
                                                                                 } else if 910
                                                                                 >
                                                                                 999
                                                                                 {
                                                                                     999
                                                                                 } else {
                                                                                     910
                                                                                 })
                                                                                    as
                                                                                    uintptr_t,
                                                                                0
                                                                                    as
                                                                                    uintptr_t
                                                                            );
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
                                                                            if !s.shall_stop_ongoing
                                                                            {
                                                                                context.call_cb(
                                                                                    Event::CONFIGURE_PROGRESS,
                                                                                    (if 920
                                                                                     <
                                                                                     1
                                                                                     {
                                                                                         1
                                                                                     } else if 920
                                                                                     >
                                                                                     999
                                                                                     {
                                                                                         999
                                                                                     } else {
                                                                                         920
                                                                                     })
                                                                                        as
                                                                                        uintptr_t,
                                                                                    0
                                                                                        as
                                                                                        uintptr_t
                                                                                );
                                                                                dc_ensure_secret_key_exists(context);
                                                                                success = true;
                                                                                info!(
                                                                                    context,
                                                                                    0,
                                                                                    "Configure completed."
                                                                                );
                                                                                if !s.shall_stop_ongoing
                                                                            {
                                                                                context.call_cb(
                                                                                        Event::CONFIGURE_PROGRESS,
                                                                                        (if 940
                                                                                         <
                                                                                         1
                                                                                         {
                                                                                             1
                                                                                         } else if 940
                                                                                         >
                                                                                         999
                                                                                         {
                                                                                             999
                                                                                         } else {
                                                                                             940
                                                                                         })
                                                                                            as
                                                                                            uintptr_t,
                                                                                        0
                                                                                            as
                                                                                            uintptr_t);
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

    context.call_cb(
        Event::CONFIGURE_PROGRESS,
        (if success { 1000 } else { 0 }) as uintptr_t,
        0 as uintptr_t,
    );
}

pub unsafe fn dc_free_ongoing(context: &Context) {
    let s_a = context.running_state.clone();
    let mut s = s_a.write().unwrap();

    s.ongoing_running = false;
    s.shall_stop_ongoing = true;
}

unsafe fn moz_autoconfigure(
    context: &Context,
    url: &str,
    param_in: &dc_loginparam_t,
) -> Option<dc_loginparam_t> {
    let mut moz_ac = moz_autoconfigure_t {
        in_0: param_in,
        in_emaildomain: std::ptr::null_mut(),
        in_emaillocalpart: std::ptr::null_mut(),
        out: dc_loginparam_new(),
        out_imap_set: 0,
        out_smtp_set: 0,
        tag_server: 0,
        tag_config: 0,
    };

    let url_c = url.strdup();
    let xml_raw = read_autoconf_file(context, url_c);
    free(url_c as *mut libc::c_void);
    if xml_raw.is_null() {
        return None;
    }

    moz_ac.in_emaillocalpart = param_in.addr.strdup();
    let p = strchr(moz_ac.in_emaillocalpart, '@' as i32);

    if p.is_null() {
        free(xml_raw as *mut libc::c_void);
        free(moz_ac.in_emaildomain as *mut libc::c_void);
        free(moz_ac.in_emaillocalpart as *mut libc::c_void);
        return None;
    }

    *p = 0 as libc::c_char;
    moz_ac.in_emaildomain = dc_strdup(p.offset(1isize));
    let mut saxparser = dc_saxparser_t {
        starttag_cb: None,
        endtag_cb: None,
        text_cb: None,
        userdata: 0 as *mut libc::c_void,
    };
    dc_saxparser_init(
        &mut saxparser,
        &mut moz_ac as *mut moz_autoconfigure_t as *mut libc::c_void,
    );
    dc_saxparser_set_tag_handler(
        &mut saxparser,
        Some(moz_autoconfigure_starttag_cb),
        Some(moz_autoconfigure_endtag_cb),
    );
    dc_saxparser_set_text_handler(&mut saxparser, Some(moz_autoconfigure_text_cb));
    dc_saxparser_parse(&mut saxparser, xml_raw);

    if moz_ac.out.mail_server.is_empty()
        || moz_ac.out.mail_port == 0
        || moz_ac.out.send_server.is_empty()
        || moz_ac.out.send_port == 0
    {
        let r = dc_loginparam_get_readable(&moz_ac.out);
        warn!(context, 0, "Bad or incomplete autoconfig: {}", r,);
        free(xml_raw as *mut libc::c_void);
        free(moz_ac.in_emaildomain as *mut libc::c_void);
        free(moz_ac.in_emaillocalpart as *mut libc::c_void);
        return None;
    }

    free(xml_raw as *mut libc::c_void);
    free(moz_ac.in_emaildomain as *mut libc::c_void);
    free(moz_ac.in_emaillocalpart as *mut libc::c_void);
    Some(moz_ac.out)
}

unsafe fn moz_autoconfigure_text_cb(
    userdata: *mut libc::c_void,
    text: *const libc::c_char,
    _len: libc::c_int,
) {
    let mut moz_ac: *mut moz_autoconfigure_t = userdata as *mut moz_autoconfigure_t;
    let mut val: *mut libc::c_char = dc_strdup(text);
    dc_trim(val);
    let addr = (*moz_ac).in_0.addr.strdup();
    dc_str_replace(
        &mut val,
        b"%EMAILADDRESS%\x00" as *const u8 as *const libc::c_char,
        addr,
    );
    free(addr as *mut libc::c_void);
    dc_str_replace(
        &mut val,
        b"%EMAILLOCALPART%\x00" as *const u8 as *const libc::c_char,
        (*moz_ac).in_emaillocalpart,
    );
    dc_str_replace(
        &mut val,
        b"%EMAILDOMAIN%\x00" as *const u8 as *const libc::c_char,
        (*moz_ac).in_emaildomain,
    );
    if (*moz_ac).tag_server == 1 {
        match (*moz_ac).tag_config {
            10 => {
                (*moz_ac).out.mail_server = to_string(val);
                val = 0 as *mut libc::c_char
            }
            11 => (*moz_ac).out.mail_port = dc_atoi_null_is_0(val),
            12 => {
                (*moz_ac).out.mail_user = to_string(val);
                val = 0 as *mut libc::c_char
            }
            13 => {
                if strcasecmp(val, b"ssl\x00" as *const u8 as *const libc::c_char) == 0 {
                    (*moz_ac).out.server_flags |= 0x200
                }
                if strcasecmp(val, b"starttls\x00" as *const u8 as *const libc::c_char) == 0 {
                    (*moz_ac).out.server_flags |= 0x100
                }
                if strcasecmp(val, b"plain\x00" as *const u8 as *const libc::c_char) == 0 {
                    (*moz_ac).out.server_flags |= 0x400
                }
            }
            _ => {}
        }
    } else if (*moz_ac).tag_server == 2 {
        match (*moz_ac).tag_config {
            10 => {
                (*moz_ac).out.send_server = to_string(val);
                val = 0 as *mut libc::c_char
            }
            11 => (*moz_ac).out.send_port = as_str(val).parse().unwrap_or_default(),
            12 => {
                (*moz_ac).out.send_user = to_string(val);
                val = 0 as *mut libc::c_char
            }
            13 => {
                if strcasecmp(val, b"ssl\x00" as *const u8 as *const libc::c_char) == 0 {
                    (*moz_ac).out.server_flags |= 0x20000
                }
                if strcasecmp(val, b"starttls\x00" as *const u8 as *const libc::c_char) == 0 {
                    (*moz_ac).out.server_flags |= 0x10000
                }
                if strcasecmp(val, b"plain\x00" as *const u8 as *const libc::c_char) == 0 {
                    (*moz_ac).out.server_flags |= 0x40000
                }
            }
            _ => {}
        }
    }
    free(val as *mut libc::c_void);
}
unsafe fn moz_autoconfigure_endtag_cb(userdata: *mut libc::c_void, tag: *const libc::c_char) {
    let mut moz_ac: *mut moz_autoconfigure_t = userdata as *mut moz_autoconfigure_t;
    if strcmp(
        tag,
        b"incomingserver\x00" as *const u8 as *const libc::c_char,
    ) == 0
    {
        (*moz_ac).tag_server = 0;
        (*moz_ac).tag_config = 0;
        (*moz_ac).out_imap_set = 1
    } else if strcmp(
        tag,
        b"outgoingserver\x00" as *const u8 as *const libc::c_char,
    ) == 0
    {
        (*moz_ac).tag_server = 0;
        (*moz_ac).tag_config = 0;
        (*moz_ac).out_smtp_set = 1
    } else {
        (*moz_ac).tag_config = 0
    };
}
unsafe fn moz_autoconfigure_starttag_cb(
    userdata: *mut libc::c_void,
    tag: *const libc::c_char,
    attr: *mut *mut libc::c_char,
) {
    let mut moz_ac: *mut moz_autoconfigure_t = userdata as *mut moz_autoconfigure_t;
    let mut p1: *const libc::c_char = 0 as *const libc::c_char;
    if strcmp(
        tag,
        b"incomingserver\x00" as *const u8 as *const libc::c_char,
    ) == 0
    {
        (*moz_ac).tag_server = if (*moz_ac).out_imap_set == 0
            && {
                p1 = dc_attr_find(attr, b"type\x00" as *const u8 as *const libc::c_char);
                !p1.is_null()
            }
            && strcasecmp(p1, b"imap\x00" as *const u8 as *const libc::c_char) == 0
        {
            1
        } else {
            0
        };
        (*moz_ac).tag_config = 0
    } else if strcmp(
        tag,
        b"outgoingserver\x00" as *const u8 as *const libc::c_char,
    ) == 0
    {
        (*moz_ac).tag_server = if (*moz_ac).out_smtp_set == 0 { 2 } else { 0 };
        (*moz_ac).tag_config = 0
    } else if strcmp(tag, b"hostname\x00" as *const u8 as *const libc::c_char) == 0 {
        (*moz_ac).tag_config = 10
    } else if strcmp(tag, b"port\x00" as *const u8 as *const libc::c_char) == 0 {
        (*moz_ac).tag_config = 11
    } else if strcmp(tag, b"sockettype\x00" as *const u8 as *const libc::c_char) == 0 {
        (*moz_ac).tag_config = 13
    } else if strcmp(tag, b"username\x00" as *const u8 as *const libc::c_char) == 0 {
        (*moz_ac).tag_config = 12
    };
}

fn read_autoconf_file(context: &Context, url: *const libc::c_char) -> *mut libc::c_char {
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

unsafe fn outlk_autodiscover(
    context: &Context,
    url__: &str,
    param_in: &dc_loginparam_t,
) -> Option<dc_loginparam_t> {
    let current_block: u64;
    let mut xml_raw: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut url = url__.strdup();
    let mut outlk_ad = outlk_autodiscover_t {
        in_0: param_in,
        out: dc_loginparam_new(),
        out_imap_set: 0,
        out_smtp_set: 0,
        tag_config: 0,
        config: [0 as *mut libc::c_char; 6],
        redirect: 0 as *mut libc::c_char,
    };
    let mut i = 0;
    loop {
        if !(i < 10) {
            current_block = 11584701595673473500;
            break;
        }
        memset(
            &mut outlk_ad as *mut outlk_autodiscover_t as *mut libc::c_void,
            0,
            ::std::mem::size_of::<outlk_autodiscover_t>(),
        );
        xml_raw = read_autoconf_file(context, url);
        if xml_raw.is_null() {
            current_block = 3070887585260837332;
            break;
        }
        let mut saxparser: dc_saxparser_t = dc_saxparser_t {
            starttag_cb: None,
            endtag_cb: None,
            text_cb: None,
            userdata: 0 as *mut libc::c_void,
        };
        dc_saxparser_init(
            &mut saxparser,
            &mut outlk_ad as *mut outlk_autodiscover_t as *mut libc::c_void,
        );
        dc_saxparser_set_tag_handler(
            &mut saxparser,
            Some(outlk_autodiscover_starttag_cb),
            Some(outlk_autodiscover_endtag_cb),
        );
        dc_saxparser_set_text_handler(&mut saxparser, Some(outlk_autodiscover_text_cb));
        dc_saxparser_parse(&mut saxparser, xml_raw);
        if !(!outlk_ad.config[5usize].is_null()
            && 0 != *outlk_ad.config[5usize].offset(0isize) as libc::c_int)
        {
            current_block = 11584701595673473500;
            break;
        }
        free(url as *mut libc::c_void);
        url = dc_strdup(outlk_ad.config[5usize]);

        outlk_clean_config(&mut outlk_ad);
        free(xml_raw as *mut libc::c_void);
        xml_raw = 0 as *mut libc::c_char;
        i += 1;
    }

    match current_block {
        11584701595673473500 => {
            if outlk_ad.out.mail_server.is_empty()
                || outlk_ad.out.mail_port == 0
                || outlk_ad.out.send_server.is_empty()
                || outlk_ad.out.send_port == 0
            {
                let r = dc_loginparam_get_readable(&outlk_ad.out);
                warn!(context, 0, "Bad or incomplete autoconfig: {}", r,);
                free(url as *mut libc::c_void);
                free(xml_raw as *mut libc::c_void);
                outlk_clean_config(&mut outlk_ad);

                return None;
            }
        }
        _ => {}
    }
    free(url as *mut libc::c_void);
    free(xml_raw as *mut libc::c_void);
    outlk_clean_config(&mut outlk_ad);
    Some(outlk_ad.out)
}

unsafe fn outlk_clean_config(mut outlk_ad: *mut outlk_autodiscover_t) {
    let mut i: libc::c_int = 0;
    while i < 6 {
        free((*outlk_ad).config[i as usize] as *mut libc::c_void);
        (*outlk_ad).config[i as usize] = 0 as *mut libc::c_char;
        i += 1
    }
}
unsafe fn outlk_autodiscover_text_cb(
    userdata: *mut libc::c_void,
    text: *const libc::c_char,
    _len: libc::c_int,
) {
    let mut outlk_ad: *mut outlk_autodiscover_t = userdata as *mut outlk_autodiscover_t;
    let val: *mut libc::c_char = dc_strdup(text);
    dc_trim(val);
    free((*outlk_ad).config[(*outlk_ad).tag_config as usize] as *mut libc::c_void);
    (*outlk_ad).config[(*outlk_ad).tag_config as usize] = val;
}
unsafe fn outlk_autodiscover_endtag_cb(userdata: *mut libc::c_void, tag: *const libc::c_char) {
    let mut outlk_ad: *mut outlk_autodiscover_t = userdata as *mut outlk_autodiscover_t;
    if strcmp(tag, b"protocol\x00" as *const u8 as *const libc::c_char) == 0 {
        if !(*outlk_ad).config[1usize].is_null() {
            let port: libc::c_int = dc_atoi_null_is_0((*outlk_ad).config[3usize]);
            let ssl_on: libc::c_int = (!(*outlk_ad).config[4usize].is_null()
                && strcasecmp(
                    (*outlk_ad).config[4usize],
                    b"on\x00" as *const u8 as *const libc::c_char,
                ) == 0) as libc::c_int;
            let ssl_off: libc::c_int = (!(*outlk_ad).config[4usize].is_null()
                && strcasecmp(
                    (*outlk_ad).config[4usize],
                    b"off\x00" as *const u8 as *const libc::c_char,
                ) == 0) as libc::c_int;
            if strcasecmp(
                (*outlk_ad).config[1usize],
                b"imap\x00" as *const u8 as *const libc::c_char,
            ) == 0
                && (*outlk_ad).out_imap_set == 0
            {
                (*outlk_ad).out.mail_server = to_string((*outlk_ad).config[2]);
                (*outlk_ad).out.mail_port = port;
                if 0 != ssl_on {
                    (*outlk_ad).out.server_flags |= 0x200
                } else if 0 != ssl_off {
                    (*outlk_ad).out.server_flags |= 0x400
                }
                (*outlk_ad).out_imap_set = 1
            } else if strcasecmp(
                (*outlk_ad).config[1usize],
                b"smtp\x00" as *const u8 as *const libc::c_char,
            ) == 0
                && (*outlk_ad).out_smtp_set == 0
            {
                (*outlk_ad).out.send_server = to_string((*outlk_ad).config[2]);
                (*outlk_ad).out.send_port = port;
                if 0 != ssl_on {
                    (*outlk_ad).out.server_flags |= 0x20000
                } else if 0 != ssl_off {
                    (*outlk_ad).out.server_flags |= 0x40000
                }
                (*outlk_ad).out_smtp_set = 1
            }
        }
        outlk_clean_config(outlk_ad);
    }
    (*outlk_ad).tag_config = 0;
}

unsafe fn outlk_autodiscover_starttag_cb(
    userdata: *mut libc::c_void,
    tag: *const libc::c_char,
    _attr: *mut *mut libc::c_char,
) {
    let mut outlk_ad: *mut outlk_autodiscover_t = userdata as *mut outlk_autodiscover_t;
    if strcmp(tag, b"protocol\x00" as *const u8 as *const libc::c_char) == 0 {
        outlk_clean_config(outlk_ad);
    } else if strcmp(tag, b"type\x00" as *const u8 as *const libc::c_char) == 0 {
        (*outlk_ad).tag_config = 1
    } else if strcmp(tag, b"server\x00" as *const u8 as *const libc::c_char) == 0 {
        (*outlk_ad).tag_config = 2
    } else if strcmp(tag, b"port\x00" as *const u8 as *const libc::c_char) == 0 {
        (*outlk_ad).tag_config = 3
    } else if strcmp(tag, b"ssl\x00" as *const u8 as *const libc::c_char) == 0 {
        (*outlk_ad).tag_config = 4
    } else if strcmp(tag, b"redirecturl\x00" as *const u8 as *const libc::c_char) == 0 {
        (*outlk_ad).tag_config = 5
    };
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
