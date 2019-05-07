//! This is a CLI program and a little testing frame.  This file must not be
//! included when using Delta Chat Core as a library.
//!
//! Usage:  messenger-backend <databasefile>
//! (for "Code::Blocks, use Project / Set programs' arguments")
//! all further options can be set using the set-command (type ? for help).

#![allow(
    unused_imports,
    mutable_transmutes,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    unused_mut,
    unused_attributes,
    non_upper_case_globals,
    non_camel_case_types,
    non_snake_case
)]

use std::ffi::CString;
use std::io::{self, Write};
use std::sync::{Arc, RwLock};

use deltachat::constants::*;
use deltachat::dc_aheader::*;
use deltachat::dc_apeerstate::*;
use deltachat::dc_array::*;
use deltachat::dc_chat::*;
use deltachat::dc_chatlist::*;
use deltachat::dc_configure::*;
use deltachat::dc_contact::*;
use deltachat::dc_context::*;
use deltachat::dc_dehtml::*;
use deltachat::dc_e2ee::*;
use deltachat::dc_hash::*;
use deltachat::dc_imap::*;
use deltachat::dc_imex::*;
use deltachat::dc_job::*;
use deltachat::dc_jobthread::*;
use deltachat::dc_jsmn::*;
use deltachat::dc_key::*;
use deltachat::dc_keyhistory::*;
use deltachat::dc_keyring::*;
use deltachat::dc_location::*;
use deltachat::dc_log::*;
use deltachat::dc_loginparam::*;
use deltachat::dc_lot::*;
use deltachat::dc_mimefactory::*;
use deltachat::dc_mimeparser::*;
use deltachat::dc_move::*;
use deltachat::dc_msg::*;
use deltachat::dc_oauth2::*;
use deltachat::dc_param::*;
use deltachat::dc_pgp::*;
use deltachat::dc_qr::*;
use deltachat::dc_receive_imf::*;
use deltachat::dc_saxparser::*;
use deltachat::dc_securejoin::*;
use deltachat::dc_simplify::*;
use deltachat::dc_smtp::*;
use deltachat::dc_sqlite3::*;
use deltachat::dc_stock::*;
use deltachat::dc_strbuilder::*;
use deltachat::dc_strencode::*;
use deltachat::dc_token::*;
use deltachat::dc_tools::*;
use deltachat::types::*;
use deltachat::x::*;
mod cmdline;

use self::cmdline::*;

/* ******************************************************************************
 * Event Handler
 ******************************************************************************/

unsafe extern "C" fn receive_event(
    context: &dc_context_t,
    event: Event,
    data1: uintptr_t,
    data2: uintptr_t,
) -> uintptr_t {
    match event as u32 {
        2091 => {}
        100 => {
            /* do not show the event as this would fill the screen */
            printf(
                b"%s\n\x00" as *const u8 as *const libc::c_char,
                data2 as *mut libc::c_char,
            );
        }
        101 => {
            printf(
                b"[DC_EVENT_SMTP_CONNECTED] %s\n\x00" as *const u8 as *const libc::c_char,
                data2 as *mut libc::c_char,
            );
        }
        102 => {
            printf(
                b"[DC_EVENT_IMAP_CONNECTED] %s\n\x00" as *const u8 as *const libc::c_char,
                data2 as *mut libc::c_char,
            );
        }
        103 => {
            printf(
                b"[DC_EVENT_SMTP_MESSAGE_SENT] %s\n\x00" as *const u8 as *const libc::c_char,
                data2 as *mut libc::c_char,
            );
        }
        300 => {
            printf(
                b"[Warning] %s\n\x00" as *const u8 as *const libc::c_char,
                data2 as *mut libc::c_char,
            );
        }
        400 => {
            printf(
                b"\x1b[31m[DC_EVENT_ERROR] %s\x1b[0m\n\x00" as *const u8 as *const libc::c_char,
                data2 as *mut libc::c_char,
            );
        }
        401 => {
            printf(
                b"\x1b[31m[DC_EVENT_ERROR_NETWORK] first=%i, msg=%s\x1b[0m\n\x00" as *const u8
                    as *const libc::c_char,
                data1 as libc::c_int,
                data2 as *mut libc::c_char,
            );
        }
        410 => {
            printf(
                b"\x1b[31m[DC_EVENT_ERROR_SELF_NOT_IN_GROUP] %s\x1b[0m\n\x00" as *const u8
                    as *const libc::c_char,
                data2 as *mut libc::c_char,
            );
        }
        2100 | 2110 => {
            let mut url: *mut libc::c_char = dc_strdup(data1 as *mut libc::c_char);
            let mut param: *mut libc::c_char = strchr(url, '?' as i32);
            if !param.is_null() {
                *param = 0i32 as libc::c_char;
                param = param.offset(1isize)
            } else {
                param = b"\x00" as *const u8 as *const libc::c_char as *mut libc::c_char
            }
            let mut ret: *mut libc::c_char = 0 as *mut libc::c_char;
            let mut tempFile: *mut libc::c_char = dc_get_fine_pathNfilename(
                context,
                context.get_blobdir(),
                b"curl.result\x00" as *const u8 as *const libc::c_char,
            );
            let mut cmd: *mut libc::c_char = if event == Event::HTTP_GET {
                dc_mprintf(
                    b"curl --silent --location --fail --insecure %s%s%s > %s\x00" as *const u8
                        as *const libc::c_char,
                    url,
                    if 0 != *param.offset(0isize) as libc::c_int {
                        b"?\x00" as *const u8 as *const libc::c_char
                    } else {
                        b"\x00" as *const u8 as *const libc::c_char
                    },
                    param,
                    tempFile,
                )
            } else {
                dc_mprintf(
                    b"curl --silent -d \"%s\" %s > %s\x00" as *const u8 as *const libc::c_char,
                    param,
                    url,
                    tempFile,
                )
            };
            let mut error: libc::c_int = system(cmd);
            if error == 0i32 {
                let mut bytes: size_t = 0i32 as size_t;
                dc_read_file(
                    context,
                    tempFile,
                    &mut ret as *mut *mut libc::c_char as *mut *mut libc::c_void,
                    &mut bytes,
                );
            }
            free(cmd as *mut libc::c_void);
            free(tempFile as *mut libc::c_void);
            free(url as *mut libc::c_void);
            return ret as uintptr_t;
        }
        2081 => {
            printf(
                b"\x1b[33m{{Received DC_EVENT_IS_OFFLINE()}}\n\x1b[0m\x00" as *const u8
                    as *const libc::c_char,
            );
        }
        2000 => {
            printf(
                b"\x1b[33m{{Received DC_EVENT_MSGS_CHANGED(%i, %i)}}\n\x1b[0m\x00" as *const u8
                    as *const libc::c_char,
                data1 as libc::c_int,
                data2 as libc::c_int,
            );
        }
        2030 => {
            printf(
                b"\x1b[33m{{Received DC_EVENT_CONTACTS_CHANGED()}}\n\x1b[0m\x00" as *const u8
                    as *const libc::c_char,
            );
        }
        2035 => {
            printf(
                b"\x1b[33m{{Received DC_EVENT_LOCATION_CHANGED(contact=%i)}}\n\x1b[0m\x00"
                    as *const u8 as *const libc::c_char,
                data1 as libc::c_int,
            );
        }
        2041 => {
            printf(
                b"\x1b[33m{{Received DC_EVENT_CONFIGURE_PROGRESS(%i \xe2\x80\xb0)}}\n\x1b[0m\x00"
                    as *const u8 as *const libc::c_char,
                data1 as libc::c_int,
            );
        }
        2051 => {
            printf(
                b"\x1b[33m{{Received DC_EVENT_IMEX_PROGRESS(%i \xe2\x80\xb0)}}\n\x1b[0m\x00"
                    as *const u8 as *const libc::c_char,
                data1 as libc::c_int,
            );
        }
        2052 => {
            printf(
                b"\x1b[33m{{Received DC_EVENT_IMEX_FILE_WRITTEN(%s)}}\n\x1b[0m\x00" as *const u8
                    as *const libc::c_char,
                data1 as *mut libc::c_char,
            );
        }
        2055 => {
            printf(
                b"\x1b[33m{{Received DC_EVENT_FILE_COPIED(%s)}}\n\x1b[0m\x00" as *const u8
                    as *const libc::c_char,
                data1 as *mut libc::c_char,
            );
        }
        2020 => {
            printf(
                b"\x1b[33m{{Received DC_EVENT_CHAT_MODIFIED(%i)}}\n\x1b[0m\x00" as *const u8
                    as *const libc::c_char,
                data1 as libc::c_int,
            );
        }
        _ => {
            printf(
                b"\x1b[33m{{Received DC_EVENT_%i(%i, %i)}}\n\x1b[0m\x00" as *const u8
                    as *const libc::c_char,
                event as libc::c_int,
                data1 as libc::c_int,
                data2 as libc::c_int,
            );
        }
    }
    return 0i32 as uintptr_t;
}
/* ******************************************************************************
 * Threads for waiting for messages and for jobs
 ******************************************************************************/
static mut run_threads: libc::c_int = 0i32;

unsafe fn start_threads(
    c: Arc<RwLock<dc_context_t>>,
) -> (
    std::thread::JoinHandle<()>,
    std::thread::JoinHandle<()>,
    std::thread::JoinHandle<()>,
    std::thread::JoinHandle<()>,
) {
    run_threads = 1;

    let ctx = c.clone();
    let h1 = std::thread::spawn(move || {
        let context = ctx.read().unwrap();
        while 0 != run_threads {
            dc_perform_imap_jobs(&context);
            dc_perform_imap_fetch(&context);
            if 0 != run_threads {
                dc_perform_imap_idle(&context);
            }
        }
    });

    let ctx = c.clone();
    let h2 = std::thread::spawn(move || {
        let context = ctx.read().unwrap();
        while 0 != run_threads {
            dc_perform_mvbox_fetch(&context);
            if 0 != run_threads {
                dc_perform_mvbox_idle(&context);
            }
        }
    });

    let ctx = c.clone();
    let h3 = std::thread::spawn(move || {
        let context = ctx.read().unwrap();
        while 0 != run_threads {
            dc_perform_sentbox_fetch(&context);
            if 0 != run_threads {
                dc_perform_sentbox_idle(&context);
            }
        }
    });

    let ctx = c.clone();
    let h4 = std::thread::spawn(move || {
        let context = ctx.read().unwrap();
        while 0 != run_threads {
            dc_perform_smtp_jobs(&context);
            if 0 != run_threads {
                dc_perform_smtp_idle(&context);
            }
        }
    });

    (h1, h2, h3, h4)
}

unsafe fn stop_threads(
    context: &dc_context_t,
    handles: Option<(
        std::thread::JoinHandle<()>,
        std::thread::JoinHandle<()>,
        std::thread::JoinHandle<()>,
        std::thread::JoinHandle<()>,
    )>,
) {
    run_threads = 0i32;
    dc_interrupt_imap_idle(context);
    dc_interrupt_mvbox_idle(context);
    dc_interrupt_sentbox_idle(context);
    dc_interrupt_smtp_idle(context);
    if let Some((h1, h2, h3, h4)) = handles {
        h1.join().unwrap();
        h2.join().unwrap();
        h3.join().unwrap();
        h4.join().unwrap();
    }
}

/* ******************************************************************************
 * The main loop
 ******************************************************************************/
fn read_cmd() -> String {
    print!("> ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input.trim_end().to_string()
}

unsafe fn main_0(mut argc: libc::c_int, mut argv: *mut *mut libc::c_char) -> libc::c_int {
    let mut cmd: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut context = dc_context_new(
        receive_event,
        0 as *mut libc::c_void,
        b"CLI\x00" as *const u8 as *const libc::c_char,
    );

    dc_cmdline_skip_auth();

    if argc == 2i32 {
        if 0 == dc_open(&mut context, *argv.offset(1isize), 0 as *const libc::c_char) {
            printf(
                b"ERROR: Cannot open %s.\n\x00" as *const u8 as *const libc::c_char,
                *argv.offset(1isize),
            );
        }
    } else if argc != 1i32 {
        printf(b"ERROR: Bad arguments\n\x00" as *const u8 as *const libc::c_char);
    }

    printf(b"Delta Chat Core is awaiting your commands.\n\x00" as *const u8 as *const libc::c_char);

    let mut handles = None;

    let ctx = Arc::new(RwLock::new(context));

    loop {
        /* read command */
        let cmdline = read_cmd();
        free(cmd as *mut libc::c_void);
        cmd = dc_strdup(CString::new(cmdline.clone()).unwrap().as_ptr());
        let mut arg1: *mut libc::c_char = strchr(cmd, ' ' as i32);
        if !arg1.is_null() {
            *arg1 = 0i32 as libc::c_char;
            arg1 = arg1.offset(1isize)
        }
        if strcmp(cmd, b"connect\x00" as *const u8 as *const libc::c_char) == 0i32 {
            handles = Some(start_threads(ctx.clone()));
        } else if strcmp(cmd, b"disconnect\x00" as *const u8 as *const libc::c_char) == 0i32 {
            stop_threads(&ctx.read().unwrap(), handles);
            handles = None;
        } else if strcmp(cmd, b"smtp-jobs\x00" as *const u8 as *const libc::c_char) == 0i32 {
            if 0 != run_threads {
                printf(
                    b"smtp-jobs are already running in a thread.\n\x00" as *const u8
                        as *const libc::c_char,
                );
            } else {
                dc_perform_smtp_jobs(&ctx.read().unwrap());
            }
        } else if strcmp(cmd, b"imap-jobs\x00" as *const u8 as *const libc::c_char) == 0i32 {
            if 0 != run_threads {
                printf(
                    b"imap-jobs are already running in a thread.\n\x00" as *const u8
                        as *const libc::c_char,
                );
            } else {
                dc_perform_imap_jobs(&ctx.read().unwrap());
            }
        } else if strcmp(cmd, b"configure\x00" as *const u8 as *const libc::c_char) == 0i32 {
            handles = { Some(start_threads(ctx.clone())) };
            dc_configure(&ctx.read().unwrap());
        } else if strcmp(cmd, b"oauth2\x00" as *const u8 as *const libc::c_char) == 0i32 {
            let mut addr: *mut libc::c_char = dc_get_config(
                &ctx.read().unwrap(),
                b"addr\x00" as *const u8 as *const libc::c_char,
            );
            if addr.is_null() || *addr.offset(0isize) as libc::c_int == 0i32 {
                printf(b"oauth2: set addr first.\n\x00" as *const u8 as *const libc::c_char);
            } else {
                let mut oauth2_url: *mut libc::c_char = dc_get_oauth2_url(
                    &ctx.read().unwrap(),
                    addr,
                    b"chat.delta:/com.b44t.messenger\x00" as *const u8 as *const libc::c_char,
                );
                if oauth2_url.is_null() {
                    printf(
                        b"OAuth2 not available for %s.\n\x00" as *const u8 as *const libc::c_char,
                        addr,
                    );
                } else {
                    printf(b"Open the following url, set mail_pw to the generated token and server_flags to 2:\n%s\n\x00"
                               as *const u8 as *const libc::c_char,
                           oauth2_url);
                }
                free(oauth2_url as *mut libc::c_void);
            }
            free(addr as *mut libc::c_void);
        } else if strcmp(cmd, b"clear\x00" as *const u8 as *const libc::c_char) == 0i32 {
            printf(b"\n\n\n\n\x00" as *const u8 as *const libc::c_char);
            printf(b"\x1b[1;1H\x1b[2J\x00" as *const u8 as *const libc::c_char);
        } else if strcmp(cmd, b"getqr\x00" as *const u8 as *const libc::c_char) == 0i32
            || strcmp(cmd, b"getbadqr\x00" as *const u8 as *const libc::c_char) == 0i32
        {
            handles = Some(start_threads(ctx.clone()));
            let mut qrstr: *mut libc::c_char = dc_get_securejoin_qr(
                &ctx.read().unwrap(),
                (if !arg1.is_null() { atoi(arg1) } else { 0i32 }) as uint32_t,
            );
            if !qrstr.is_null() && 0 != *qrstr.offset(0isize) as libc::c_int {
                if strcmp(cmd, b"getbadqr\x00" as *const u8 as *const libc::c_char) == 0i32
                    && strlen(qrstr) > 40
                {
                    let mut i: libc::c_int = 12i32;
                    while i < 22i32 {
                        *qrstr.offset(i as isize) = '0' as i32 as libc::c_char;
                        i += 1
                    }
                }
                printf(b"%s\n\x00" as *const u8 as *const libc::c_char, qrstr);
                let mut syscmd: *mut libc::c_char = dc_mprintf(
                    b"qrencode -t ansiutf8 \"%s\" -o -\x00" as *const u8 as *const libc::c_char,
                    qrstr,
                );
                system(syscmd);
                free(syscmd as *mut libc::c_void);
            }
            free(qrstr as *mut libc::c_void);
        } else if strcmp(cmd, b"joinqr\x00" as *const u8 as *const libc::c_char) == 0i32 {
            handles = Some(start_threads(ctx.clone()));
            if !arg1.is_null() {
                dc_join_securejoin(&ctx.read().unwrap(), arg1);
            }
        } else {
            if strcmp(cmd, b"exit\x00" as *const u8 as *const libc::c_char) == 0i32 {
                break;
            }
            if !(*cmd.offset(0isize) as libc::c_int == 0i32) {
                let mut execute_result: *mut libc::c_char =
                    dc_cmdline(&ctx.read().unwrap(), &cmdline);
                if !execute_result.is_null() {
                    printf(
                        b"%s\n\x00" as *const u8 as *const libc::c_char,
                        execute_result,
                    );
                    free(execute_result as *mut libc::c_void);
                }
            }
        }
    }

    let ctx = ctx.clone();

    {
        let mut ctx = ctx.write().unwrap();
        free(cmd as *mut libc::c_void);
        stop_threads(&ctx, handles);
        dc_close(&mut ctx);
        dc_context_unref(&mut ctx);
    }
    0
}

pub fn main() {
    let mut args: Vec<*mut libc::c_char> = Vec::new();
    for arg in ::std::env::args() {
        args.push(
            ::std::ffi::CString::new(arg)
                .expect("Failed to convert argument into CString.")
                .into_raw(),
        );
    }
    args.push(::std::ptr::null_mut());

    let res = unsafe {
        main_0(
            (args.len() - 1) as libc::c_int,
            args.as_mut_ptr() as *mut *mut libc::c_char,
        )
    };
    ::std::process::exit(res)
}
