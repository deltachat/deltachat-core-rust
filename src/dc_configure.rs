use crate::constants::Event;
use crate::dc_context::dc_context_t;
use crate::dc_e2ee::*;
use crate::dc_job::*;
use crate::dc_log::*;
use crate::dc_loginparam::*;
use crate::dc_saxparser::*;
use crate::dc_sqlite3::*;
use crate::dc_strencode::*;
use crate::dc_tools::*;
use crate::imap::*;
use crate::oauth2::*;
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
#[derive(Copy, Clone)]
#[repr(C)]
pub struct moz_autoconfigure_t {
    pub in_0: *const dc_loginparam_t,
    pub in_emaildomain: *mut libc::c_char,
    pub in_emaillocalpart: *mut libc::c_char,
    pub out: *mut dc_loginparam_t,
    pub out_imap_set: libc::c_int,
    pub out_smtp_set: libc::c_int,
    pub tag_server: libc::c_int,
    pub tag_config: libc::c_int,
}

/* ******************************************************************************
 * Outlook's Autodiscover
 ******************************************************************************/
#[derive(Copy, Clone)]
#[repr(C)]
pub struct outlk_autodiscover_t {
    pub in_0: *const dc_loginparam_t,
    pub out: *mut dc_loginparam_t,
    pub out_imap_set: libc::c_int,
    pub out_smtp_set: libc::c_int,
    pub tag_config: libc::c_int,
    pub config: [*mut libc::c_char; 6],
    pub redirect: *mut libc::c_char,
}
// connect
pub unsafe fn dc_configure(context: &dc_context_t) {
    if 0 != dc_has_ongoing(context) {
        dc_log_warning(
            context,
            0i32,
            b"There is already another ongoing process running.\x00" as *const u8
                as *const libc::c_char,
        );
        return;
    }
    dc_job_kill_action(context, 900i32);
    dc_job_add(context, 900i32, 0i32, 0 as *const libc::c_char, 0i32);
}
pub unsafe fn dc_has_ongoing(context: &dc_context_t) -> libc::c_int {
    let s_a = context.running_state.clone();
    let s = s_a.read().unwrap();

    if s.ongoing_running || !s.shall_stop_ongoing {
        1
    } else {
        0
    }
}
pub unsafe fn dc_is_configured(context: &dc_context_t) -> libc::c_int {
    return if 0
        != dc_sqlite3_get_config_int(
            context,
            &context.sql.clone().read().unwrap(),
            b"configured\x00" as *const u8 as *const libc::c_char,
            0i32,
        ) {
        1i32
    } else {
        0i32
    };
}
pub unsafe fn dc_stop_ongoing_process(context: &dc_context_t) {
    let s_a = context.running_state.clone();
    let mut s = s_a.write().unwrap();

    if s.ongoing_running && !s.shall_stop_ongoing {
        dc_log_info(
            context,
            0i32,
            b"Signaling the ongoing process to stop ASAP.\x00" as *const u8 as *const libc::c_char,
        );
        s.shall_stop_ongoing = true;
    } else {
        dc_log_info(
            context,
            0i32,
            b"No ongoing process to stop.\x00" as *const u8 as *const libc::c_char,
        );
    };
}
// the other dc_job_do_DC_JOB_*() functions are declared static in the c-file
pub unsafe fn dc_job_do_DC_JOB_CONFIGURE_IMAP(context: &dc_context_t, _job: *mut dc_job_t) {
    let flags: libc::c_int;
    let mut current_block: u64;
    let mut success: libc::c_int = 0i32;
    let mut imap_connected_here: libc::c_int = 0i32;
    let mut smtp_connected_here: libc::c_int = 0i32;
    let mut ongoing_allocated_here: libc::c_int = 0i32;
    let mvbox_folder: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut param: *mut dc_loginparam_t = 0 as *mut dc_loginparam_t;
    /* just a pointer inside param, must not be freed! */
    let mut param_domain: *mut libc::c_char;
    let mut param_addr_urlencoded: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut param_autoconfig: *mut dc_loginparam_t = 0 as *mut dc_loginparam_t;

    if !(0 == dc_alloc_ongoing(context)) {
        ongoing_allocated_here = 1i32;
        if 0 == dc_sqlite3_is_open(&context.sql.clone().read().unwrap()) {
            dc_log_error(
                context,
                0i32,
                b"Cannot configure, database not opened.\x00" as *const u8 as *const libc::c_char,
            );
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
            dc_log_info(
                context,
                0i32,
                b"Configure ...\x00" as *const u8 as *const libc::c_char,
            );

            let s_a = context.running_state.clone();
            let s = s_a.read().unwrap();

            if !s.shall_stop_ongoing {
                (context.cb)(
                    context,
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
                param = dc_loginparam_new();
                dc_loginparam_read(
                    context,
                    param,
                    &context.sql.clone().read().unwrap(),
                    b"\x00" as *const u8 as *const libc::c_char,
                );

                if (*param).addr.is_null() {
                    dc_log_error(
                        context,
                        0i32,
                        b"Please enter the email address.\x00" as *const u8 as *const libc::c_char,
                    );
                } else {
                    dc_trim((*param).addr);
                    if 0 != (*param).server_flags & 0x2i32 {
                        // the used oauth2 addr may differ, check this.
                        // if dc_get_oauth2_addr() is not available in the oauth2 implementation,
                        // just use the given one.
                        if s.shall_stop_ongoing {
                            current_block = 2927484062889439186;
                        } else {
                            (context.cb)(
                                context,
                                Event::CONFIGURE_PROGRESS,
                                (if 10i32 < 1i32 {
                                    1i32
                                } else if 10i32 > 999i32 {
                                    999i32
                                } else {
                                    10i32
                                }) as uintptr_t,
                                0i32 as uintptr_t,
                            );
                            let oauth2_addr = dc_get_oauth2_addr(
                                context,
                                to_str((*param).addr),
                                to_str((*param).mail_pw),
                            );
                            if oauth2_addr.is_some() {
                                free((*param).addr as *mut libc::c_void);
                                (*param).addr = strdup(to_cstring(oauth2_addr.unwrap()).as_ptr());
                                dc_sqlite3_set_config(
                                    context,
                                    &context.sql.clone().read().unwrap(),
                                    b"addr\x00" as *const u8 as *const libc::c_char,
                                    (*param).addr,
                                );
                            }
                            if s.shall_stop_ongoing {
                                current_block = 2927484062889439186;
                            } else {
                                (context.cb)(
                                    context,
                                    Event::CONFIGURE_PROGRESS,
                                    (if 20i32 < 1i32 {
                                        1i32
                                    } else if 20i32 > 999i32 {
                                        999i32
                                    } else {
                                        20i32
                                    }) as uintptr_t,
                                    0i32 as uintptr_t,
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
                            param_domain = strchr((*param).addr, '@' as i32);
                            if param_domain.is_null()
                                || *param_domain.offset(0isize) as libc::c_int == 0i32
                            {
                                dc_log_error(
                                    context,
                                    0i32,
                                    b"Bad email-address.\x00" as *const u8 as *const libc::c_char,
                                );
                            } else {
                                param_domain = param_domain.offset(1isize);
                                param_addr_urlencoded = dc_urlencode((*param).addr);
                                if (*param).mail_pw.is_null() {
                                    (*param).mail_pw = dc_strdup(0 as *const libc::c_char)
                                }
                                if !s.shall_stop_ongoing {
                                    (context.cb)(
                                        context,
                                        Event::CONFIGURE_PROGRESS,
                                        (if 200i32 < 1i32 {
                                            1i32
                                        } else if 200i32 > 999i32 {
                                            999i32
                                        } else {
                                            200i32
                                        }) as uintptr_t,
                                        0i32 as uintptr_t,
                                    );
                                    /* 2.  Autoconfig
                                     **************************************************************************/
                                    if (*param).mail_server.is_null()
                                        && (*param).mail_port == 0i32
                                        && (*param).send_server.is_null()
                                        && (*param).send_port == 0i32
                                        && (*param).send_user.is_null()
                                        && (*param).server_flags & !0x2i32 == 0i32
                                    {
                                        /*&&param->mail_user   ==NULL -- the user can enter a loginname which is used by autoconfig then */
                                        /*&&param->send_pw     ==NULL -- the password cannot be auto-configured and is no criterion for autoconfig or not */
                                        /* flags but OAuth2 avoid autoconfig */
                                        let keep_flags: libc::c_int =
                                            (*param).server_flags & 0x2i32;
                                        /* A.  Search configurations from the domain used in the email-address, prefer encrypted */
                                        if param_autoconfig.is_null() {
                                            let  url:
                                            *mut libc::c_char =
                                                dc_mprintf(b"https://autoconfig.%s/mail/config-v1.1.xml?emailaddress=%s\x00"
                                                           as
                                                           *const u8
                                                           as
                                                           *const libc::c_char,
                                                           param_domain,
                                                           param_addr_urlencoded);
                                            param_autoconfig =
                                                moz_autoconfigure(context, url, param);
                                            free(url as *mut libc::c_void);
                                            if s.shall_stop_ongoing {
                                                current_block = 2927484062889439186;
                                            } else {
                                                (context.cb)(
                                                    context,
                                                    Event::CONFIGURE_PROGRESS,
                                                    (if 300i32 < 1i32 {
                                                        1i32
                                                    } else if 300i32 > 999i32 {
                                                        999i32
                                                    } else {
                                                        300i32
                                                    })
                                                        as uintptr_t,
                                                    0i32 as uintptr_t,
                                                );
                                                current_block = 13325891313334703151;
                                            }
                                        } else {
                                            current_block = 13325891313334703151;
                                        }
                                        match current_block {
                                            2927484062889439186 => {}
                                            _ => {
                                                if param_autoconfig.is_null() {
                                                    // the doc does not mention `emailaddress=`, however, Thunderbird adds it, see https://releases.mozilla.org/pub/thunderbird/ ,  which makes some sense
                                                    let  url_0:
                                                    *mut libc::c_char =
                                                        dc_mprintf(b"https://%s/.well-known/autoconfig/mail/config-v1.1.xml?emailaddress=%s\x00"
                                                                   as
                                                                   *const u8
                                                                   as
                                                                   *const libc::c_char,
                                                                   param_domain,
                                                                   param_addr_urlencoded);
                                                    param_autoconfig =
                                                        moz_autoconfigure(context, url_0, param);
                                                    free(url_0 as *mut libc::c_void);
                                                    if s.shall_stop_ongoing {
                                                        current_block = 2927484062889439186;
                                                    } else {
                                                        (context.cb)(
                                                            context,
                                                            Event::CONFIGURE_PROGRESS,
                                                            (if 310i32 < 1i32 {
                                                                1i32
                                                            } else if 310i32 > 999i32 {
                                                                999i32
                                                            } else {
                                                                310i32
                                                            })
                                                                as uintptr_t,
                                                            0i32 as uintptr_t,
                                                        );
                                                        current_block = 5597585068398118923;
                                                    }
                                                } else {
                                                    current_block = 5597585068398118923;
                                                }
                                                match current_block {
                                                    2927484062889439186 => {}
                                                    _ => {
                                                        let mut i: libc::c_int = 0i32;
                                                        loop {
                                                            if !(i <= 1i32) {
                                                                current_block =
                                                                    12961834331865314435;
                                                                break;
                                                            }
                                                            if param_autoconfig.is_null() {
                                                                /* Outlook uses always SSL but different domains */
                                                                let  url_1:
                                                                *mut libc::c_char =
                                                                    dc_mprintf(b"https://%s%s/autodiscover/autodiscover.xml\x00"
                                                                               as
                                                                               *const u8
                                                                               as
                                                                               *const libc::c_char,
                                                                               if i
                                                                               ==
                                                                               0i32
                                                                               {
                                                                                   b"\x00"
                                                                                       as
                                                                                       *const u8
                                                                                       as
                                                                                       *const libc::c_char
                                                                               } else {
                                                                                   b"autodiscover.\x00"
                                                                                       as
                                                                                       *const u8
                                                                                       as
                                                                                       *const libc::c_char
                                                                               },
                                                                               param_domain);
                                                                param_autoconfig =
                                                                    outlk_autodiscover(
                                                                        context, url_1, param,
                                                                    );
                                                                free(url_1 as *mut libc::c_void);
                                                                if s.shall_stop_ongoing {
                                                                    current_block =
                                                                        2927484062889439186;
                                                                    break;
                                                                }
                                                                (context.cb)(
                                                                    context,
                                                                    Event::CONFIGURE_PROGRESS,
                                                                    (if 320i32 + i * 10i32 < 1i32 {
                                                                        1i32
                                                                    } else if 320i32 + i * 10i32
                                                                        > 999i32
                                                                    {
                                                                        999i32
                                                                    } else {
                                                                        320i32 + i * 10i32
                                                                    })
                                                                        as uintptr_t,
                                                                    0i32 as uintptr_t,
                                                                );
                                                            }
                                                            i += 1
                                                        }
                                                        match current_block {
                                                            2927484062889439186 => {}
                                                            _ => {
                                                                if param_autoconfig.is_null() {
                                                                    let  url_2:
                                                                    *mut libc::c_char =
                                                                        dc_mprintf(b"http://autoconfig.%s/mail/config-v1.1.xml?emailaddress=%s\x00"
                                                                                   as
                                                                                   *const u8
                                                                                   as
                                                                                   *const libc::c_char,
                                                                                   param_domain,
                                                                                   param_addr_urlencoded);
                                                                    param_autoconfig =
                                                                        moz_autoconfigure(
                                                                            context, url_2, param,
                                                                        );
                                                                    free(
                                                                        url_2 as *mut libc::c_void,
                                                                    );

                                                                    if s.shall_stop_ongoing {
                                                                        current_block =
                                                                            2927484062889439186;
                                                                    } else {
                                                                        (context.cb)(
                                                                            context,
                                                                            Event::CONFIGURE_PROGRESS,
                                                                            (if 340i32
                                                                             <
                                                                             1i32
                                                                             {
                                                                                 1i32
                                                                             } else if 340i32
                                                                             >
                                                                             999i32
                                                                             {
                                                                                 999i32
                                                                             } else {
                                                                                 340i32
                                                                             })
                                                                                as
                                                                                uintptr_t,
                                                                            0i32
                                                                                as
                                                                                uintptr_t
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
                                                                            .is_null()
                                                                        {
                                                                            // do not transfer the email-address unencrypted
                                                                            let  url_3:
                                                                            *mut libc::c_char =
                                                                                dc_mprintf(b"http://%s/.well-known/autoconfig/mail/config-v1.1.xml\x00"
                                                                                           as
                                                                                           *const u8
                                                                                           as
                                                                                           *const libc::c_char,
                                                                                           param_domain);
                                                                            param_autoconfig =
                                                                                moz_autoconfigure(
                                                                                    context, url_3,
                                                                                    param,
                                                                                );
                                                                            free(url_3
                                                                                 as
                                                                                 *mut libc::c_void);
                                                                            if s.shall_stop_ongoing
                                                                            {
                                                                                current_block
                                                                                    =
                                                                                    2927484062889439186;
                                                                            } else {
                                                                                (context.cb)(context,
                                                                                                Event::CONFIGURE_PROGRESS,
                                                                                                (if 350i32
                                                                                                 <
                                                                                                 1i32
                                                                                                 {
                                                                                                     1i32
                                                                                                 } else if 350i32
                                                                                                 >
                                                                                                 999i32
                                                                                                 {
                                                                                                     999i32
                                                                                                 } else {
                                                                                                     350i32
                                                                                                 })
                                                                                                as
                                                                                                uintptr_t,
                                                                                                0i32
                                                                                                as
                                                                                                uintptr_t);
                                                                                current_block
                                                                                    =
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
                                                                                    .is_null()
                                                                                {
                                                                                    /* always SSL for Thunderbird's database */
                                                                                    let  url_4:
                                                                                    *mut libc::c_char =
                                                                                        dc_mprintf(b"https://autoconfig.thunderbird.net/v1.1/%s\x00"
                                                                                                   as
                                                                                                   *const u8
                                                                                                   as
                                                                                                   *const libc::c_char,
                                                                                                   param_domain);
                                                                                    param_autoconfig
                                                                                        =
                                                                                        moz_autoconfigure(context,
                                                                                                          url_4,
                                                                                                          param);
                                                                                    free(url_4
                                                                                         as
                                                                                         *mut libc::c_void);
                                                                                    if s.shall_stop_ongoing
                                                                                    {
                                                                                        current_block
                                                                                            =
                                                                                            2927484062889439186;
                                                                                    } else {
                                                                                        (context.cb)(context,
                                                                                                        Event::CONFIGURE_PROGRESS,
                                                                                                        (if 500i32
                                                                                                         <
                                                                                                         1i32
                                                                                                         {
                                                                                                             1i32
                                                                                                         } else if 500i32
                                                                                                         >
                                                                                                         999i32
                                                                                                         {
                                                                                                             999i32
                                                                                                         } else {
                                                                                                             500i32
                                                                                                         })
                                                                                                        as
                                                                                                        uintptr_t,
                                                                                                        0i32
                                                                                                        as
                                                                                                        uintptr_t);
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
                                                                                        if !param_autoconfig.is_null()
                                                                                        {
                                                                                            let  r:
                                                                                            *mut libc::c_char =
                                                                                                dc_loginparam_get_readable(param_autoconfig);
                                                                                            dc_log_info(context,
                                                                                                        0i32,
                                                                                                        b"Got autoconfig: %s\x00"
                                                                                                        as
                                                                                                        *const u8
                                                                                                        as
                                                                                                        *const libc::c_char,
                                                                                                        r);
                                                                                            free(r
                                                                                                 as
                                                                                                 *mut libc::c_void);
                                                                                            if !(*param_autoconfig).mail_user.is_null()
                                                                                            {
                                                                                                free((*param).mail_user
                                                                                                     as
                                                                                                     *mut libc::c_void);
                                                                                                (*param).mail_user
                                                                                                    =
                                                                                                    dc_strdup_keep_null((*param_autoconfig).mail_user)
                                                                                            }
                                                                                            (*param).mail_server
                                                                                                =
                                                                                                dc_strdup_keep_null((*param_autoconfig).mail_server);
                                                                                            (*param).mail_port
                                                                                                =
                                                                                                (*param_autoconfig).mail_port;
                                                                                            (*param).send_server
                                                                                                =
                                                                                                dc_strdup_keep_null((*param_autoconfig).send_server);
                                                                                            (*param).send_port
                                                                                                =
                                                                                                (*param_autoconfig).send_port;
                                                                                            (*param).send_user
                                                                                                =
                                                                                                dc_strdup_keep_null((*param_autoconfig).send_user);
                                                                                            (*param).server_flags
                                                                                                =
                                                                                                (*param_autoconfig).server_flags
                                                                                        }
                                                                                        (*param).server_flags
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
                                            if (*param).mail_server.is_null() {
                                                (*param).mail_server = dc_mprintf(
                                                    b"imap.%s\x00" as *const u8
                                                        as *const libc::c_char,
                                                    param_domain,
                                                )
                                            }
                                            if (*param).mail_port == 0i32 {
                                                (*param).mail_port = if 0
                                                    != (*param).server_flags & (0x100i32 | 0x400i32)
                                                {
                                                    143i32
                                                } else {
                                                    993i32
                                                }
                                            }
                                            if (*param).mail_user.is_null() {
                                                (*param).mail_user = dc_strdup((*param).addr)
                                            }
                                            if (*param).send_server.is_null()
                                                && !(*param).mail_server.is_null()
                                            {
                                                (*param).send_server =
                                                    dc_strdup((*param).mail_server);
                                                if strncmp(
                                                    (*param).send_server,
                                                    b"imap.\x00" as *const u8
                                                        as *const libc::c_char,
                                                    5,
                                                ) == 0i32
                                                {
                                                    memcpy(
                                                        (*param).send_server as *mut libc::c_void,
                                                        b"smtp\x00" as *const u8
                                                            as *const libc::c_char
                                                            as *const libc::c_void,
                                                        4,
                                                    );
                                                }
                                            }
                                            if (*param).send_port == 0i32 {
                                                (*param).send_port = if 0
                                                    != (*param).server_flags & 0x10000i32
                                                {
                                                    587i32
                                                } else if 0 != (*param).server_flags & 0x40000i32 {
                                                    25i32
                                                } else {
                                                    465i32
                                                }
                                            }
                                            if (*param).send_user.is_null()
                                                && !(*param).mail_user.is_null()
                                            {
                                                (*param).send_user = dc_strdup((*param).mail_user)
                                            }
                                            if (*param).send_pw.is_null()
                                                && !(*param).mail_pw.is_null()
                                            {
                                                (*param).send_pw = dc_strdup((*param).mail_pw)
                                            }
                                            if 0 == dc_exactly_one_bit_set(
                                                (*param).server_flags & (0x2i32 | 0x4i32),
                                            ) {
                                                (*param).server_flags &= !(0x2i32 | 0x4i32);
                                                (*param).server_flags |= 0x4i32
                                            }
                                            if 0 == dc_exactly_one_bit_set(
                                                (*param).server_flags
                                                    & (0x100i32 | 0x200i32 | 0x400i32),
                                            ) {
                                                (*param).server_flags &=
                                                    !(0x100i32 | 0x200i32 | 0x400i32);
                                                (*param).server_flags |=
                                                    if (*param).send_port == 143i32 {
                                                        0x100i32
                                                    } else {
                                                        0x200i32
                                                    }
                                            }
                                            if 0 == dc_exactly_one_bit_set(
                                                (*param).server_flags
                                                    & (0x10000i32 | 0x20000i32 | 0x40000i32),
                                            ) {
                                                (*param).server_flags &=
                                                    !(0x10000i32 | 0x20000i32 | 0x40000i32);
                                                (*param).server_flags |=
                                                    if (*param).send_port == 587i32 {
                                                        0x10000i32
                                                    } else if (*param).send_port == 25i32 {
                                                        0x40000i32
                                                    } else {
                                                        0x20000i32
                                                    }
                                            }
                                            /* do we have a complete configuration? */
                                            if (*param).addr.is_null()
                                                || (*param).mail_server.is_null()
                                                || (*param).mail_port == 0i32
                                                || (*param).mail_user.is_null()
                                                || (*param).mail_pw.is_null()
                                                || (*param).send_server.is_null()
                                                || (*param).send_port == 0i32
                                                || (*param).send_user.is_null()
                                                || (*param).send_pw.is_null()
                                                || (*param).server_flags == 0i32
                                            {
                                                dc_log_error(
                                                    context,
                                                    0i32,
                                                    b"Account settings incomplete.\x00" as *const u8
                                                        as *const libc::c_char,
                                                );
                                            } else if !s.shall_stop_ongoing {
                                                (context.cb)(
                                                    context,
                                                    Event::CONFIGURE_PROGRESS,
                                                    (if 600i32 < 1i32 {
                                                        1i32
                                                    } else if 600i32 > 999i32 {
                                                        999i32
                                                    } else {
                                                        600i32
                                                    })
                                                        as uintptr_t,
                                                    0i32 as uintptr_t,
                                                );
                                                /* try to connect to IMAP - if we did not got an autoconfig,
                                                do some further tries with different settings and username variations */
                                                let mut username_variation: libc::c_int = 0i32;
                                                loop {
                                                    if !(username_variation <= 1i32) {
                                                        current_block = 14187386403465544025;
                                                        break;
                                                    }
                                                    let r_0: *mut libc::c_char =
                                                        dc_loginparam_get_readable(param);
                                                    dc_log_info(
                                                        context,
                                                        0i32,
                                                        b"Trying: %s\x00" as *const u8
                                                            as *const libc::c_char,
                                                        r_0,
                                                    );
                                                    free(r_0 as *mut libc::c_void);
                                                    if 0 != context
                                                        .inbox
                                                        .read()
                                                        .unwrap()
                                                        .connect(context, param)
                                                    {
                                                        current_block = 14187386403465544025;
                                                        break;
                                                    }
                                                    if !param_autoconfig.is_null() {
                                                        current_block = 2927484062889439186;
                                                        break;
                                                    }
                                                    // probe STARTTLS/993
                                                    if s.shall_stop_ongoing {
                                                        current_block = 2927484062889439186;
                                                        break;
                                                    }
                                                    (context.cb)(
                                                        context,
                                                        Event::CONFIGURE_PROGRESS,
                                                        (if 650i32 + username_variation * 30i32
                                                            < 1i32
                                                        {
                                                            1i32
                                                        } else if 650i32
                                                            + username_variation * 30i32
                                                            > 999i32
                                                        {
                                                            999i32
                                                        } else {
                                                            650i32 + username_variation * 30i32
                                                        })
                                                            as uintptr_t,
                                                        0i32 as uintptr_t,
                                                    );
                                                    (*param).server_flags &=
                                                        !(0x100i32 | 0x200i32 | 0x400i32);
                                                    (*param).server_flags |= 0x100i32;
                                                    let r_1: *mut libc::c_char =
                                                        dc_loginparam_get_readable(param);
                                                    dc_log_info(
                                                        context,
                                                        0i32,
                                                        b"Trying: %s\x00" as *const u8
                                                            as *const libc::c_char,
                                                        r_1,
                                                    );
                                                    free(r_1 as *mut libc::c_void);
                                                    if 0 != context
                                                        .inbox
                                                        .read()
                                                        .unwrap()
                                                        .connect(context, param)
                                                    {
                                                        current_block = 14187386403465544025;
                                                        break;
                                                    }
                                                    // probe STARTTLS/143
                                                    if s.shall_stop_ongoing {
                                                        current_block = 2927484062889439186;
                                                        break;
                                                    }
                                                    (context.cb)(
                                                        context,
                                                        Event::CONFIGURE_PROGRESS,
                                                        (if 660i32 + username_variation * 30i32
                                                            < 1i32
                                                        {
                                                            1i32
                                                        } else if 660i32
                                                            + username_variation * 30i32
                                                            > 999i32
                                                        {
                                                            999i32
                                                        } else {
                                                            660i32 + username_variation * 30i32
                                                        })
                                                            as uintptr_t,
                                                        0i32 as uintptr_t,
                                                    );
                                                    (*param).mail_port = 143i32;
                                                    let r_2: *mut libc::c_char =
                                                        dc_loginparam_get_readable(param);
                                                    dc_log_info(
                                                        context,
                                                        0i32,
                                                        b"Trying: %s\x00" as *const u8
                                                            as *const libc::c_char,
                                                        r_2,
                                                    );
                                                    free(r_2 as *mut libc::c_void);
                                                    if 0 != context
                                                        .inbox
                                                        .read()
                                                        .unwrap()
                                                        .connect(context, param)
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
                                                    (context.cb)(
                                                        context,
                                                        Event::CONFIGURE_PROGRESS,
                                                        (if 670i32 + username_variation * 30i32
                                                            < 1i32
                                                        {
                                                            1i32
                                                        } else if 670i32
                                                            + username_variation * 30i32
                                                            > 999i32
                                                        {
                                                            999i32
                                                        } else {
                                                            670i32 + username_variation * 30i32
                                                        })
                                                            as uintptr_t,
                                                        0i32 as uintptr_t,
                                                    );
                                                    (*param).server_flags &=
                                                        !(0x100i32 | 0x200i32 | 0x400i32);
                                                    (*param).server_flags |= 0x200i32;
                                                    (*param).mail_port = 993i32;
                                                    let mut at: *mut libc::c_char =
                                                        strchr((*param).mail_user, '@' as i32);
                                                    if !at.is_null() {
                                                        *at = 0i32 as libc::c_char
                                                    }
                                                    at = strchr((*param).send_user, '@' as i32);
                                                    if !at.is_null() {
                                                        *at = 0i32 as libc::c_char
                                                    }
                                                    username_variation += 1
                                                }
                                                match current_block {
                                                    2927484062889439186 => {}
                                                    _ => {
                                                        imap_connected_here = 1i32;
                                                        if !s.shall_stop_ongoing {
                                                            (context.cb)(
                                                                context,
                                                                Event::CONFIGURE_PROGRESS,
                                                                (if 800i32 < 1i32 {
                                                                    1i32
                                                                } else if 800i32 > 999i32 {
                                                                    999i32
                                                                } else {
                                                                    800i32
                                                                })
                                                                    as uintptr_t,
                                                                0i32 as uintptr_t,
                                                            );
                                                            /* try to connect to SMTP - if we did not got an autoconfig, the first try was SSL-465 and we do a second try with STARTTLS-587 */
                                                            if 0 == context
                                                                .smtp
                                                                .clone()
                                                                .lock()
                                                                .unwrap()
                                                                .connect(context, param)
                                                            {
                                                                if !param_autoconfig.is_null() {
                                                                    current_block =
                                                                        2927484062889439186;
                                                                } else if s.shall_stop_ongoing {
                                                                    current_block =
                                                                        2927484062889439186;
                                                                } else {
                                                                    (context.cb)(
                                                                        context,
                                                                        Event::CONFIGURE_PROGRESS,
                                                                        (if 850i32 < 1i32 {
                                                                            1i32
                                                                        } else if 850i32 > 999i32 {
                                                                            999i32
                                                                        } else {
                                                                            850i32
                                                                        })
                                                                            as uintptr_t,
                                                                        0i32 as uintptr_t,
                                                                    );
                                                                    (*param).server_flags &=
                                                                        !(0x10000i32
                                                                            | 0x20000i32
                                                                            | 0x40000i32);
                                                                    (*param).server_flags |=
                                                                        0x10000i32;
                                                                    (*param).send_port = 587i32;
                                                                    let r_3: *mut libc::c_char =
                                                                        dc_loginparam_get_readable(
                                                                            param,
                                                                        );
                                                                    dc_log_info(
                                                                        context,
                                                                        0i32,
                                                                        b"Trying: %s\x00"
                                                                            as *const u8
                                                                            as *const libc::c_char,
                                                                        r_3,
                                                                    );
                                                                    free(r_3 as *mut libc::c_void);
                                                                    if 0 == context
                                                                        .smtp
                                                                        .clone()
                                                                        .lock()
                                                                        .unwrap()
                                                                        .connect(context, param)
                                                                    {
                                                                        if s.shall_stop_ongoing {
                                                                            current_block =
                                                                                2927484062889439186;
                                                                        } else {
                                                                            (context.cb)(context,
                                                                                            Event::CONFIGURE_PROGRESS,
                                                                                            (if 860i32
                                                                                             <
                                                                                             1i32
                                                                                             {
                                                                                                 1i32
                                                                                             } else if 860i32
                                                                                             >
                                                                                             999i32
                                                                                             {
                                                                                                 999i32
                                                                                             } else {
                                                                                                 860i32
                                                                                             })
                                                                                            as
                                                                                            uintptr_t,
                                                                                            0i32
                                                                                            as
                                                                                            uintptr_t);
                                                                            (*param)
                                                                                .server_flags &=
                                                                                !(0x10000i32
                                                                                    | 0x20000i32
                                                                                    | 0x40000i32);
                                                                            (*param)
                                                                                .server_flags |=
                                                                                0x10000i32;
                                                                            (*param).send_port =
                                                                                25i32;
                                                                            let  r_4:
                                                                            *mut libc::c_char =
                                                                                dc_loginparam_get_readable(param);
                                                                            dc_log_info(context,
                                                                                        0i32,
                                                                                        b"Trying: %s\x00"
                                                                                        as
                                                                                        *const u8
                                                                                        as
                                                                                        *const libc::c_char,
                                                                                        r_4);
                                                                            free(r_4
                                                                                 as
                                                                                 *mut libc::c_void);
                                                                            if 0 == context
                                                                                .smtp
                                                                                .clone()
                                                                                .lock()
                                                                                .unwrap()
                                                                                .connect(
                                                                                    context, param,
                                                                                )
                                                                            {
                                                                                current_block
                                                                                    =
                                                                                    2927484062889439186;
                                                                            } else {
                                                                                current_block
                                                                                    =
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
                                                                    smtp_connected_here = 1i32;
                                                                    if !s.shall_stop_ongoing {
                                                                        (context.cb)(context,
                                                                                        Event::CONFIGURE_PROGRESS,
                                                                                        (if 900i32
                                                                                         <
                                                                                         1i32
                                                                                         {
                                                                                             1i32
                                                                                         } else if 900i32
                                                                                         >
                                                                                         999i32
                                                                                         {
                                                                                             999i32
                                                                                         } else {
                                                                                             900i32
                                                                                         })
                                                                                        as
                                                                                        uintptr_t,
                                                                                        0i32
                                                                                        as
                                                                                        uintptr_t);
                                                                        flags
                                                                            =
                                                                            if 0
                                                                            !=
                                                                            dc_sqlite3_get_config_int(context, &context.sql.clone().read().unwrap(),
                                                                                                      b"mvbox_watch\x00"
                                                                                                      as
                                                                                                      *const u8
                                                                                                      as
                                                                                                      *const libc::c_char,
                                                                                                      1i32)
                                                                            ||
                                                                            0
                                                                            !=
                                                                            dc_sqlite3_get_config_int(context, &context.sql.clone().read().unwrap(),
                                                                                                      b"mvbox_move\x00"
                                                                                                      as
                                                                                                      *const u8
                                                                                                      as
                                                                                                      *const libc::c_char,
                                                                                                      1i32)
                                                                        {
                                                                            0x1i32
                                                                        } else {
                                                                            0i32
                                                                        };

                                                                        context
                                                                            .inbox
                                                                            .read()
                                                                            .unwrap()
                                                                            .configure_folders(
                                                                                context, flags,
                                                                            );
                                                                        if !s.shall_stop_ongoing {
                                                                            (context.cb)(context,
                                                                                            Event::CONFIGURE_PROGRESS,
                                                                                            (if 910i32
                                                                                             <
                                                                                             1i32
                                                                                             {
                                                                                                 1i32
                                                                                             } else if 910i32
                                                                                             >
                                                                                             999i32
                                                                                             {
                                                                                                 999i32
                                                                                             } else {
                                                                                                 910i32
                                                                                             })
                                                                                            as
                                                                                            uintptr_t,
                                                                                            0i32
                                                                                            as
                                                                                            uintptr_t);
                                                                            dc_loginparam_write(context, param,
                                                                                                &context.sql.clone().read().unwrap(),
                                                                                                b"configured_\x00"
                                                                                                as
                                                                                                *const u8
                                                                                                as
                                                                                                *const libc::c_char);
                                                                            dc_sqlite3_set_config_int(context, &context.sql.clone().read().unwrap(),
                                                                                                      b"configured\x00"
                                                                                                      as
                                                                                                      *const u8
                                                                                                      as
                                                                                                      *const libc::c_char,
                                                                                                      1i32);
                                                                            if !s.shall_stop_ongoing
                                                                            {
                                                                                (context.cb)(context,
                                                                                                Event::CONFIGURE_PROGRESS,
                                                                                                (if 920i32
                                                                                                 <
                                                                                                 1i32
                                                                                                 {
                                                                                                     1i32
                                                                                                 } else if 920i32
                                                                                                 >
                                                                                                 999i32
                                                                                                 {
                                                                                                     999i32
                                                                                                 } else {
                                                                                                     920i32
                                                                                                 })
                                                                                                as
                                                                                                uintptr_t,
                                                                                                0i32
                                                                                                as
                                                                                                uintptr_t);
                                                                                dc_ensure_secret_key_exists(context);
                                                                                success = 1i32;
                                                                                dc_log_info(context,
                                                                                            0i32,
                                                                                            b"Configure completed.\x00"
                                                                                            as
                                                                                            *const u8
                                                                                            as
                                                                                            *const libc::c_char);
                                                                                if !s.shall_stop_ongoing
                                                                                {
                                                                                    (context.cb)(context,
                                                                                                    Event::CONFIGURE_PROGRESS,
                                                                                                    (if 940i32
                                                                                                     <
                                                                                                     1i32
                                                                                                     {
                                                                                                         1i32
                                                                                                     } else if 940i32
                                                                                                     >
                                                                                                     999i32
                                                                                                     {
                                                                                                         999i32
                                                                                                     } else {
                                                                                                         940i32
                                                                                                     })
                                                                                                    as
                                                                                                    uintptr_t,
                                                                                                    0i32
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

    if 0 != imap_connected_here {
        context.inbox.read().unwrap().disconnect(context);
    }
    if 0 != smtp_connected_here {
        context.smtp.clone().lock().unwrap().disconnect();
    }
    dc_loginparam_unref(param);
    dc_loginparam_unref(param_autoconfig);
    free(param_addr_urlencoded as *mut libc::c_void);
    if 0 != ongoing_allocated_here {
        dc_free_ongoing(context);
    }
    free(mvbox_folder as *mut libc::c_void);
    (context.cb)(
        context,
        Event::CONFIGURE_PROGRESS,
        (if 0 != success { 1000i32 } else { 0i32 }) as uintptr_t,
        0i32 as uintptr_t,
    );
}

pub unsafe fn dc_free_ongoing(context: &dc_context_t) {
    let s_a = context.running_state.clone();
    let mut s = s_a.write().unwrap();

    s.ongoing_running = false;
    s.shall_stop_ongoing = true;
}

unsafe fn moz_autoconfigure(
    context: &dc_context_t,
    url: *const libc::c_char,
    param_in: *const dc_loginparam_t,
) -> *mut dc_loginparam_t {
    let p: *mut libc::c_char;
    let mut saxparser: dc_saxparser_t;
    let xml_raw: *mut libc::c_char;
    let mut moz_ac: moz_autoconfigure_t = moz_autoconfigure_t {
        in_0: 0 as *const dc_loginparam_t,
        in_emaildomain: 0 as *mut libc::c_char,
        in_emaillocalpart: 0 as *mut libc::c_char,
        out: 0 as *mut dc_loginparam_t,
        out_imap_set: 0,
        out_smtp_set: 0,
        tag_server: 0,
        tag_config: 0,
    };
    memset(
        &mut moz_ac as *mut moz_autoconfigure_t as *mut libc::c_void,
        0i32,
        ::std::mem::size_of::<moz_autoconfigure_t>(),
    );
    xml_raw = read_autoconf_file(context, url);
    if !xml_raw.is_null() {
        moz_ac.in_0 = param_in;
        moz_ac.in_emaillocalpart = dc_strdup((*param_in).addr);
        p = strchr(moz_ac.in_emaillocalpart, '@' as i32);
        if !p.is_null() {
            *p = 0i32 as libc::c_char;
            moz_ac.in_emaildomain = dc_strdup(p.offset(1isize));
            moz_ac.out = dc_loginparam_new();
            saxparser = dc_saxparser_t {
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
            if (*moz_ac.out).mail_server.is_null()
                || (*moz_ac.out).mail_port == 0i32
                || (*moz_ac.out).send_server.is_null()
                || (*moz_ac.out).send_port == 0i32
            {
                let r: *mut libc::c_char = dc_loginparam_get_readable(moz_ac.out);
                dc_log_warning(
                    context,
                    0i32,
                    b"Bad or incomplete autoconfig: %s\x00" as *const u8 as *const libc::c_char,
                    r,
                );
                free(r as *mut libc::c_void);
                dc_loginparam_unref(moz_ac.out);
                moz_ac.out = 0 as *mut dc_loginparam_t
            }
        }
    }
    free(xml_raw as *mut libc::c_void);
    free(moz_ac.in_emaildomain as *mut libc::c_void);
    free(moz_ac.in_emaillocalpart as *mut libc::c_void);
    return moz_ac.out;
}

unsafe fn moz_autoconfigure_text_cb(
    userdata: *mut libc::c_void,
    text: *const libc::c_char,
    _len: libc::c_int,
) {
    let mut moz_ac: *mut moz_autoconfigure_t = userdata as *mut moz_autoconfigure_t;
    let mut val: *mut libc::c_char = dc_strdup(text);
    dc_trim(val);
    dc_str_replace(
        &mut val,
        b"%EMAILADDRESS%\x00" as *const u8 as *const libc::c_char,
        (*(*moz_ac).in_0).addr,
    );
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
    if (*moz_ac).tag_server == 1i32 {
        match (*moz_ac).tag_config {
            10 => {
                free((*(*moz_ac).out).mail_server as *mut libc::c_void);
                (*(*moz_ac).out).mail_server = val;
                val = 0 as *mut libc::c_char
            }
            11 => (*(*moz_ac).out).mail_port = atoi(val),
            12 => {
                free((*(*moz_ac).out).mail_user as *mut libc::c_void);
                (*(*moz_ac).out).mail_user = val;
                val = 0 as *mut libc::c_char
            }
            13 => {
                if strcasecmp(val, b"ssl\x00" as *const u8 as *const libc::c_char) == 0i32 {
                    (*(*moz_ac).out).server_flags |= 0x200i32
                }
                if strcasecmp(val, b"starttls\x00" as *const u8 as *const libc::c_char) == 0i32 {
                    (*(*moz_ac).out).server_flags |= 0x100i32
                }
                if strcasecmp(val, b"plain\x00" as *const u8 as *const libc::c_char) == 0i32 {
                    (*(*moz_ac).out).server_flags |= 0x400i32
                }
            }
            _ => {}
        }
    } else if (*moz_ac).tag_server == 2i32 {
        match (*moz_ac).tag_config {
            10 => {
                free((*(*moz_ac).out).send_server as *mut libc::c_void);
                (*(*moz_ac).out).send_server = val;
                val = 0 as *mut libc::c_char
            }
            11 => (*(*moz_ac).out).send_port = atoi(val),
            12 => {
                free((*(*moz_ac).out).send_user as *mut libc::c_void);
                (*(*moz_ac).out).send_user = val;
                val = 0 as *mut libc::c_char
            }
            13 => {
                if strcasecmp(val, b"ssl\x00" as *const u8 as *const libc::c_char) == 0i32 {
                    (*(*moz_ac).out).server_flags |= 0x20000i32
                }
                if strcasecmp(val, b"starttls\x00" as *const u8 as *const libc::c_char) == 0i32 {
                    (*(*moz_ac).out).server_flags |= 0x10000i32
                }
                if strcasecmp(val, b"plain\x00" as *const u8 as *const libc::c_char) == 0i32 {
                    (*(*moz_ac).out).server_flags |= 0x40000i32
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
    ) == 0i32
    {
        (*moz_ac).tag_server = 0i32;
        (*moz_ac).tag_config = 0i32;
        (*moz_ac).out_imap_set = 1i32
    } else if strcmp(
        tag,
        b"outgoingserver\x00" as *const u8 as *const libc::c_char,
    ) == 0i32
    {
        (*moz_ac).tag_server = 0i32;
        (*moz_ac).tag_config = 0i32;
        (*moz_ac).out_smtp_set = 1i32
    } else {
        (*moz_ac).tag_config = 0i32
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
    ) == 0i32
    {
        (*moz_ac).tag_server = if (*moz_ac).out_imap_set == 0i32
            && {
                p1 = dc_attr_find(attr, b"type\x00" as *const u8 as *const libc::c_char);
                !p1.is_null()
            }
            && strcasecmp(p1, b"imap\x00" as *const u8 as *const libc::c_char) == 0i32
        {
            1i32
        } else {
            0i32
        };
        (*moz_ac).tag_config = 0i32
    } else if strcmp(
        tag,
        b"outgoingserver\x00" as *const u8 as *const libc::c_char,
    ) == 0i32
    {
        (*moz_ac).tag_server = if (*moz_ac).out_smtp_set == 0i32 {
            2i32
        } else {
            0i32
        };
        (*moz_ac).tag_config = 0i32
    } else if strcmp(tag, b"hostname\x00" as *const u8 as *const libc::c_char) == 0i32 {
        (*moz_ac).tag_config = 10i32
    } else if strcmp(tag, b"port\x00" as *const u8 as *const libc::c_char) == 0i32 {
        (*moz_ac).tag_config = 11i32
    } else if strcmp(tag, b"sockettype\x00" as *const u8 as *const libc::c_char) == 0i32 {
        (*moz_ac).tag_config = 13i32
    } else if strcmp(tag, b"username\x00" as *const u8 as *const libc::c_char) == 0i32 {
        (*moz_ac).tag_config = 12i32
    };
}

fn read_autoconf_file(context: &dc_context_t, url: *const libc::c_char) -> *mut libc::c_char {
    info!(context, 0, "Testing %s ...", url);

    match reqwest::Client::new()
        .get(to_str(url))
        .send()
        .and_then(|mut res| res.text())
    {
        Ok(res) => unsafe { libc::strdup(to_cstring(res).as_ptr()) },
        Err(_err) => {
            info!(context, 0, "Can\'t read file.",);

            std::ptr::null_mut()
        }
    }
}

unsafe fn outlk_autodiscover(
    context: &dc_context_t,
    url__: *const libc::c_char,
    param_in: *const dc_loginparam_t,
) -> *mut dc_loginparam_t {
    let current_block: u64;
    let mut xml_raw: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut url: *mut libc::c_char = dc_strdup(url__);
    let mut outlk_ad: outlk_autodiscover_t = outlk_autodiscover_t {
        in_0: 0 as *const dc_loginparam_t,
        out: 0 as *mut dc_loginparam_t,
        out_imap_set: 0,
        out_smtp_set: 0,
        tag_config: 0,
        config: [0 as *mut libc::c_char; 6],
        redirect: 0 as *mut libc::c_char,
    };
    let mut i: libc::c_int = 0;
    loop {
        if !(i < 10i32) {
            current_block = 11584701595673473500;
            break;
        }
        memset(
            &mut outlk_ad as *mut outlk_autodiscover_t as *mut libc::c_void,
            0i32,
            ::std::mem::size_of::<outlk_autodiscover_t>(),
        );
        xml_raw = read_autoconf_file(context, url);
        if xml_raw.is_null() {
            current_block = 3070887585260837332;
            break;
        }
        outlk_ad.in_0 = param_in;
        outlk_ad.out = dc_loginparam_new();
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
        dc_loginparam_unref(outlk_ad.out);
        outlk_clean_config(&mut outlk_ad);
        free(xml_raw as *mut libc::c_void);
        xml_raw = 0 as *mut libc::c_char;
        i += 1
    }
    match current_block {
        11584701595673473500 => {
            if (*outlk_ad.out).mail_server.is_null()
                || (*outlk_ad.out).mail_port == 0i32
                || (*outlk_ad.out).send_server.is_null()
                || (*outlk_ad.out).send_port == 0i32
            {
                let r: *mut libc::c_char = dc_loginparam_get_readable(outlk_ad.out);
                dc_log_warning(
                    context,
                    0i32,
                    b"Bad or incomplete autoconfig: %s\x00" as *const u8 as *const libc::c_char,
                    r,
                );
                free(r as *mut libc::c_void);
                dc_loginparam_unref(outlk_ad.out);
                outlk_ad.out = 0 as *mut dc_loginparam_t
            }
        }
        _ => {}
    }
    free(url as *mut libc::c_void);
    free(xml_raw as *mut libc::c_void);
    outlk_clean_config(&mut outlk_ad);
    return outlk_ad.out;
}
unsafe fn outlk_clean_config(mut outlk_ad: *mut outlk_autodiscover_t) {
    let mut i: libc::c_int = 0;
    while i < 6i32 {
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
    if strcmp(tag, b"protocol\x00" as *const u8 as *const libc::c_char) == 0i32 {
        if !(*outlk_ad).config[1usize].is_null() {
            let port: libc::c_int = dc_atoi_null_is_0((*outlk_ad).config[3usize]);
            let ssl_on: libc::c_int = (!(*outlk_ad).config[4usize].is_null()
                && strcasecmp(
                    (*outlk_ad).config[4usize],
                    b"on\x00" as *const u8 as *const libc::c_char,
                ) == 0i32) as libc::c_int;
            let ssl_off: libc::c_int = (!(*outlk_ad).config[4usize].is_null()
                && strcasecmp(
                    (*outlk_ad).config[4usize],
                    b"off\x00" as *const u8 as *const libc::c_char,
                ) == 0i32) as libc::c_int;
            if strcasecmp(
                (*outlk_ad).config[1usize],
                b"imap\x00" as *const u8 as *const libc::c_char,
            ) == 0i32
                && (*outlk_ad).out_imap_set == 0i32
            {
                (*(*outlk_ad).out).mail_server = dc_strdup_keep_null((*outlk_ad).config[2usize]);
                (*(*outlk_ad).out).mail_port = port;
                if 0 != ssl_on {
                    (*(*outlk_ad).out).server_flags |= 0x200i32
                } else if 0 != ssl_off {
                    (*(*outlk_ad).out).server_flags |= 0x400i32
                }
                (*outlk_ad).out_imap_set = 1i32
            } else if strcasecmp(
                (*outlk_ad).config[1usize],
                b"smtp\x00" as *const u8 as *const libc::c_char,
            ) == 0i32
                && (*outlk_ad).out_smtp_set == 0i32
            {
                (*(*outlk_ad).out).send_server = dc_strdup_keep_null((*outlk_ad).config[2usize]);
                (*(*outlk_ad).out).send_port = port;
                if 0 != ssl_on {
                    (*(*outlk_ad).out).server_flags |= 0x20000i32
                } else if 0 != ssl_off {
                    (*(*outlk_ad).out).server_flags |= 0x40000i32
                }
                (*outlk_ad).out_smtp_set = 1i32
            }
        }
        outlk_clean_config(outlk_ad);
    }
    (*outlk_ad).tag_config = 0i32;
}
unsafe fn outlk_autodiscover_starttag_cb(
    userdata: *mut libc::c_void,
    tag: *const libc::c_char,
    _attr: *mut *mut libc::c_char,
) {
    let mut outlk_ad: *mut outlk_autodiscover_t = userdata as *mut outlk_autodiscover_t;
    if strcmp(tag, b"protocol\x00" as *const u8 as *const libc::c_char) == 0i32 {
        outlk_clean_config(outlk_ad);
    } else if strcmp(tag, b"type\x00" as *const u8 as *const libc::c_char) == 0i32 {
        (*outlk_ad).tag_config = 1i32
    } else if strcmp(tag, b"server\x00" as *const u8 as *const libc::c_char) == 0i32 {
        (*outlk_ad).tag_config = 2i32
    } else if strcmp(tag, b"port\x00" as *const u8 as *const libc::c_char) == 0i32 {
        (*outlk_ad).tag_config = 3i32
    } else if strcmp(tag, b"ssl\x00" as *const u8 as *const libc::c_char) == 0i32 {
        (*outlk_ad).tag_config = 4i32
    } else if strcmp(tag, b"redirecturl\x00" as *const u8 as *const libc::c_char) == 0i32 {
        (*outlk_ad).tag_config = 5i32
    };
}
pub unsafe fn dc_alloc_ongoing(context: &dc_context_t) -> libc::c_int {
    if 0 != dc_has_ongoing(context) {
        dc_log_warning(
            context,
            0i32,
            b"There is already another ongoing process running.\x00" as *const u8
                as *const libc::c_char,
        );
        return 0i32;
    }
    let s_a = context.running_state.clone();
    let mut s = s_a.write().unwrap();

    s.ongoing_running = true;
    s.shall_stop_ongoing = false;

    1
}

pub unsafe fn dc_connect_to_configured_imap(context: &dc_context_t, imap: &Imap) -> libc::c_int {
    let mut ret_connected: libc::c_int = 0i32;
    let param: *mut dc_loginparam_t = dc_loginparam_new();
    if imap.is_connected() {
        ret_connected = 1i32
    } else if dc_sqlite3_get_config_int(
        context,
        &context.sql.clone().read().unwrap(),
        b"configured\x00" as *const u8 as *const libc::c_char,
        0i32,
    ) == 0i32
    {
        dc_log_warning(
            context,
            0i32,
            b"Not configured, cannot connect.\x00" as *const u8 as *const libc::c_char,
        );
    } else {
        dc_loginparam_read(
            context,
            param,
            &context.sql.clone().read().unwrap(),
            b"configured_\x00" as *const u8 as *const libc::c_char,
        );
        /*the trailing underscore is correct*/
        if !(0 == imap.connect(context, param)) {
            ret_connected = 2i32
        }
    }
    dc_loginparam_unref(param);
    ret_connected
}
