//! This is a CLI program and a little testing frame.  This file must not be
//! included when using Delta Chat Core as a library.
//!
//! Usage:  messenger-backend <databasefile>
//! (for "Code::Blocks, use Project / Set programs' arguments")
//! all further options can be set using the set-command (type ? for help).

#![allow(
    mutable_transmutes,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    non_upper_case_globals,
    non_camel_case_types,
    non_snake_case
)]

#[macro_use]
extern crate deltachat;
#[macro_use]
extern crate failure;

use std::ffi::CString;
use std::io::{self, Write};
use std::sync::{Arc, RwLock};

use deltachat::constants::*;
use deltachat::context::*;
use deltachat::dc_configure::*;
use deltachat::dc_job::*;
use deltachat::dc_securejoin::*;
use deltachat::dc_tools::*;
use deltachat::oauth2::*;
use deltachat::types::*;
use deltachat::x::*;

mod cmdline;
use self::cmdline::*;

/* ******************************************************************************
 * Event Handler
 ******************************************************************************/

unsafe extern "C" fn receive_event(
    _context: &Context,
    event: Event,
    data1: uintptr_t,
    data2: uintptr_t,
) -> uintptr_t {
    match event as u32 {
        2091 => {}
        100 => {
            /* do not show the event as this would fill the screen */
            println!("{}", to_string(data2 as *const _),);
        }
        101 => {
            println!("[DC_EVENT_SMTP_CONNECTED] {}", to_string(data2 as *const _));
        }
        102 => {
            println!("[DC_EVENT_IMAP_CONNECTED] {}", to_string(data2 as *const _),);
        }
        103 => {
            println!(
                "[DC_EVENT_SMTP_MESSAGE_SENT] {}",
                to_string(data2 as *const _),
            );
        }
        300 => {
            println!("[Warning] {}", to_string(data2 as *const _),);
        }
        400 => {
            println!(
                "\x1b[31m[DC_EVENT_ERROR] {}\x1b[0m",
                to_string(data2 as *const _),
            );
        }
        401 => {
            println!(
                "\x1b[31m[DC_EVENT_ERROR_NETWORK] first={}, msg={}\x1b[0m",
                data1 as libc::c_int,
                to_string(data2 as *const _),
            );
        }
        410 => {
            println!(
                "\x1b[31m[DC_EVENT_ERROR_SELF_NOT_IN_GROUP] {}\x1b[0m",
                to_string(data2 as *const _),
            );
        }
        2081 => {
            print!("\x1b[33m{{Received DC_EVENT_IS_OFFLINE()}}\n\x1b[0m");
        }
        2000 => {
            print!(
                "\x1b[33m{{Received DC_EVENT_MSGS_CHANGED({}, {})}}\n\x1b[0m",
                data1 as libc::c_int, data2 as libc::c_int,
            );
        }
        2030 => {
            print!("\x1b[33m{{Received DC_EVENT_CONTACTS_CHANGED()}}\n\x1b[0m");
        }
        2035 => {
            print!(
                "\x1b[33m{{Received DC_EVENT_LOCATION_CHANGED(contact={})}}\n\x1b[0m",
                data1 as libc::c_int,
            );
        }
        2041 => {
            print!(
                "\x1b[33m{{Received DC_EVENT_CONFIGURE_PROGRESS({} ‰)}}\n\x1b[0m",
                data1 as libc::c_int,
            );
        }
        2051 => {
            print!(
                "\x1b[33m{{Received DC_EVENT_IMEX_PROGRESS({} ‰)}}\n\x1b[0m",
                data1 as libc::c_int,
            );
        }
        2052 => {
            print!(
                "\x1b[33m{{Received DC_EVENT_IMEX_FILE_WRITTEN({})}}\n\x1b[0m",
                to_string(data1 as *const _)
            );
        }
        2055 => {
            print!(
                "\x1b[33m{{Received DC_EVENT_FILE_COPIED({})}}\n\x1b[0m",
                to_string(data1 as *const _)
            );
        }
        2020 => {
            print!(
                "\x1b[33m{{Received DC_EVENT_CHAT_MODIFIED({})}}\n\x1b[0m",
                data1 as libc::c_int,
            );
        }
        _ => {
            print!(
                "\x1b[33m{{Received DC_EVENT_{}({}, {})}}\n\x1b[0m",
                event as libc::c_int, data1 as libc::c_int, data2 as libc::c_int,
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
    c: Arc<RwLock<Context>>,
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
    context: &Context,
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

unsafe fn main_0(argc: libc::c_int, argv: *mut *mut libc::c_char) -> libc::c_int {
    let mut cmd: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut context = dc_context_new(
        receive_event,
        0 as *mut libc::c_void,
        b"CLI\x00" as *const u8 as *const libc::c_char,
    );

    dc_cmdline_skip_auth();

    if argc == 2i32 {
        if 0 == dc_open(&mut context, *argv.offset(1isize), 0 as *const libc::c_char) {
            println!(
                "ERROR: Cannot open {}.",
                to_string(*argv.offset(1isize) as *const _)
            );
        }
    } else if argc != 1i32 {
        println!("ERROR: Bad arguments");
    }

    println!("Delta Chat Core is awaiting your commands.");

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
                println!("smtp-jobs are already running in a thread.",);
            } else {
                dc_perform_smtp_jobs(&ctx.read().unwrap());
            }
        } else if strcmp(cmd, b"imap-jobs\x00" as *const u8 as *const libc::c_char) == 0i32 {
            if 0 != run_threads {
                println!("imap-jobs are already running in a thread.");
            } else {
                dc_perform_imap_jobs(&ctx.read().unwrap());
            }
        } else if strcmp(cmd, b"configure\x00" as *const u8 as *const libc::c_char) == 0i32 {
            handles = { Some(start_threads(ctx.clone())) };
            dc_configure(&ctx.read().unwrap());
        } else if strcmp(cmd, b"oauth2\x00" as *const u8 as *const libc::c_char) == 0i32 {
            let addr: *mut libc::c_char = dc_get_config(
                &ctx.read().unwrap(),
                b"addr\x00" as *const u8 as *const libc::c_char,
            );
            if addr.is_null() || *addr.offset(0isize) as libc::c_int == 0i32 {
                println!("oauth2: set addr first.");
            } else {
                let oauth2_url = dc_get_oauth2_url(
                    &ctx.read().unwrap(),
                    to_str(addr),
                    "chat.delta:/com.b44t.messenger",
                );
                if oauth2_url.is_none() {
                    println!("OAuth2 not available for {}.", to_string(addr));
                } else {
                    println!("Open the following url, set mail_pw to the generated token and server_flags to 2:\n{}", oauth2_url.unwrap());
                }
            }
            free(addr as *mut libc::c_void);
        } else if strcmp(cmd, b"clear\x00" as *const u8 as *const libc::c_char) == 0i32 {
            println!("\n\n\n");
            print!("\x1b[1;1H\x1b[2J");
        } else if strcmp(cmd, b"getqr\x00" as *const u8 as *const libc::c_char) == 0i32
            || strcmp(cmd, b"getbadqr\x00" as *const u8 as *const libc::c_char) == 0i32
        {
            handles = Some(start_threads(ctx.clone()));
            let qrstr: *mut libc::c_char =
                dc_get_securejoin_qr(&ctx.read().unwrap(), dc_atoi_null_is_0(arg1) as u32);
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
                println!("{}", to_string(qrstr as *const _));
                let syscmd: *mut libc::c_char = dc_mprintf(
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
                match dc_cmdline(&ctx.read().unwrap(), &cmdline) {
                    Ok(_) => {}
                    Err(err) => println!("ERROR: {}"),
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
    let _ = pretty_env_logger::try_init();

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
