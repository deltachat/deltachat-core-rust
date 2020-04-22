#![allow(
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    non_upper_case_globals,
    non_camel_case_types,
    clippy::missing_safety_doc,
    clippy::expect_fun_call
)]

#[macro_use]
extern crate human_panic;
extern crate num_traits;
extern crate serde_json;

use std::collections::HashMap;
use std::convert::TryInto;
use std::ffi::CString;
use std::fmt::Write;
use std::ptr;
use std::str::FromStr;
use std::sync::RwLock;
use std::time::{Duration, SystemTime};

use libc::uintptr_t;
use num_traits::{FromPrimitive, ToPrimitive};

use deltachat::chat::{ChatId, ChatVisibility, MuteDuration};
use deltachat::constants::DC_MSG_ID_LAST_SPECIAL;
use deltachat::contact::{Contact, Origin};
use deltachat::context::Context;
use deltachat::key::DcKey;
use deltachat::message::MsgId;
use deltachat::stock::StockMessage;
use deltachat::*;

mod dc_array;

mod string;
use self::string::*;

// as C lacks a good and portable error handling,
// in general, the C Interface is forgiving wrt to bad parameters.
// - objects returned by some functions
//   should be passable to the functions handling that object.
// - if in doubt, the empty string is returned on failures;
//   this avoids panics if the ui just forgets to handle a case
// - finally, this behaviour matches the old core-c API and UIs already depend on it

// TODO: constants

// dc_context_t

/// The FFI context struct.
///
/// This structure represents the [Context] on the FFI interface.
/// Since it is returned by [dc_context_new] before it is initialised
/// by [dc_open] it needs to store the actual [Context] in an [Option]
/// and protected by an [RwLock].  Other than that it needs to store
/// the data which is passed into [dc_context_new].
pub struct ContextWrapper {
    cb: Option<dc_callback_t>,
    userdata: *mut libc::c_void,
    os_name: String,
    inner: RwLock<Option<context::Context>>,
}

unsafe impl Send for ContextWrapper {}
unsafe impl Sync for ContextWrapper {}

/// Callback function that should be given to [dc_context_new].
///
/// @memberof [dc_context_t]
/// @param context The context object as returned by [dc_context_new].
/// @param event one of the @ref DC_EVENT constants
/// @param data1 depends on the event parameter
/// @param data2 depends on the event parameter
/// @return return 0 unless stated otherwise in the event parameter documentation
pub type dc_callback_t =
    unsafe extern "C" fn(_: &dc_context_t, _: i32, _: uintptr_t, _: uintptr_t) -> uintptr_t;

/// Struct representing the deltachat context.
///
/// See [ContextWrapper] for implementation details.
pub type dc_context_t = ContextWrapper;

impl ContextWrapper {
    /// Log a warning on the FFI context.
    ///
    /// Like [error] but logs as a warning which only goes to the
    /// logfile rather than being shown directly to the user.
    unsafe fn warning(&self, msg: &str) {
        self.translate_cb(Event::Warning(msg.to_string()));
    }

    /// Unlock the context and execute a closure with it.
    ///
    /// This unlocks the context and gets a read lock.  The Rust
    /// [Context] object it passed as only argument to the closure
    /// which can now do Rust API calls using it.  The return value of
    /// the closure will be returned by this function.  When the
    /// closure returns the read lock is released.
    ///
    /// If the context is not open an error is logged via the callback
    /// and `Err(())` is returned.
    ///
    /// This function returns a [Result] allowing the caller to supply
    /// the appropriate return value for an error return since this
    /// differs for various functions on the FFI API: sometimes 0,
    /// NULL, an empty string etc.
    unsafe fn with_inner<T, F>(&self, ctxfn: F) -> Result<T, ()>
    where
        F: FnOnce(&Context) -> T,
    {
        self.try_inner(|ctx| Ok(ctxfn(ctx))).map_err(|err| {
            self.warning(&err.to_string());
        })
    }

    /// Unlock the context and execute a closure with it.
    ///
    /// This is like [ContextWrapper::with_inner] but uses
    /// [anyhow::Error] as error type.  This allows you to write a
    /// closure which could produce many errors, use the `?` operator
    /// to return them and handle them all as the return of this call.
    fn try_inner<T, F>(&self, ctxfn: F) -> Result<T, anyhow::Error>
    where
        F: FnOnce(&Context) -> Result<T, anyhow::Error>,
    {
        let guard = self.inner.read().unwrap();
        match guard.as_ref() {
            Some(ref ctx) => ctxfn(ctx),
            None => Err(anyhow::format_err!("context not open")),
        }
    }

    /// Translates the callback from the rust style to the C-style version.
    unsafe fn translate_cb(&self, event: Event) {
        if let Some(ffi_cb) = self.cb {
            let event_id = event.as_id();
            match event {
                Event::Info(msg)
                | Event::SmtpConnected(msg)
                | Event::ImapConnected(msg)
                | Event::SmtpMessageSent(msg)
                | Event::ImapMessageDeleted(msg)
                | Event::ImapMessageMoved(msg)
                | Event::ImapFolderEmptied(msg)
                | Event::NewBlobFile(msg)
                | Event::DeletedBlobFile(msg)
                | Event::Warning(msg)
                | Event::Error(msg)
                | Event::ErrorNetwork(msg)
                | Event::ErrorSelfNotInGroup(msg) => {
                    let data2 = CString::new(msg).unwrap_or_default();
                    ffi_cb(self, event_id, 0, data2.as_ptr() as uintptr_t);
                }
                Event::MsgsChanged { chat_id, msg_id }
                | Event::IncomingMsg { chat_id, msg_id }
                | Event::MsgDelivered { chat_id, msg_id }
                | Event::MsgFailed { chat_id, msg_id }
                | Event::MsgRead { chat_id, msg_id } => {
                    ffi_cb(
                        self,
                        event_id,
                        chat_id.to_u32() as uintptr_t,
                        msg_id.to_u32() as uintptr_t,
                    );
                }
                Event::ChatModified(chat_id) => {
                    ffi_cb(self, event_id, chat_id.to_u32() as uintptr_t, 0);
                }
                Event::ContactsChanged(id) | Event::LocationChanged(id) => {
                    let id = id.unwrap_or_default();
                    ffi_cb(self, event_id, id as uintptr_t, 0);
                }
                Event::ConfigureProgress(progress) | Event::ImexProgress(progress) => {
                    ffi_cb(self, event_id, progress as uintptr_t, 0);
                }
                Event::ImexFileWritten(file) => {
                    let data1 = file.to_c_string().unwrap_or_default();
                    ffi_cb(self, event_id, data1.as_ptr() as uintptr_t, 0);
                }
                Event::SecurejoinInviterProgress {
                    contact_id,
                    progress,
                }
                | Event::SecurejoinJoinerProgress {
                    contact_id,
                    progress,
                } => {
                    ffi_cb(
                        self,
                        event_id,
                        contact_id as uintptr_t,
                        progress as uintptr_t,
                    );
                }
            }
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn dc_context_new(
    cb: Option<dc_callback_t>,
    userdata: *mut libc::c_void,
    os_name: *const libc::c_char,
) -> *mut dc_context_t {
    setup_panic!();

    let os_name = if os_name.is_null() {
        String::from("DcFFI")
    } else {
        to_string_lossy(os_name)
    };
    let ffi_ctx = ContextWrapper {
        cb,
        userdata,
        os_name,
        inner: RwLock::new(None),
    };
    Box::into_raw(Box::new(ffi_ctx))
}

/// Release the context structure.
///
/// This function releases the memory of the `dc_context_t` structure.
#[no_mangle]
pub unsafe extern "C" fn dc_context_unref(context: *mut dc_context_t) {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_context_unref()");
        return;
    }
    let ffi_context = &mut *context;
    Box::from_raw(ffi_context);
}

#[no_mangle]
pub unsafe extern "C" fn dc_get_userdata(context: *mut dc_context_t) -> *mut libc::c_void {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_get_userdata()");
        return ptr::null_mut();
    }
    let ffi_context = &mut *context;
    ffi_context.userdata
}

#[no_mangle]
pub unsafe extern "C" fn dc_open(
    context: *mut dc_context_t,
    dbfile: *const libc::c_char,
    blobdir: *const libc::c_char,
) -> libc::c_int {
    if context.is_null() || dbfile.is_null() {
        eprintln!("ignoring careless call to dc_open()");
        return 0;
    }
    let ffi_context = &*context;
    let rust_cb = move |_ctx: &Context, evt: Event| ffi_context.translate_cb(evt);

    let ctx = if blobdir.is_null() || *blobdir == 0 {
        Context::new(
            Box::new(rust_cb),
            ffi_context.os_name.clone(),
            as_path(dbfile).to_path_buf(),
        )
    } else {
        Context::with_blobdir(
            Box::new(rust_cb),
            ffi_context.os_name.clone(),
            as_path(dbfile).to_path_buf(),
            as_path(blobdir).to_path_buf(),
        )
    };
    match ctx {
        Ok(ctx) => {
            let mut inner_guard = ffi_context.inner.write().unwrap();
            *inner_guard = Some(ctx);
            1
        }
        Err(_) => 0,
    }
}

#[no_mangle]
pub unsafe extern "C" fn dc_close(context: *mut dc_context_t) {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_close()");
        return;
    }
    let ffi_context = &mut *context;
    ffi_context.inner.write().unwrap().take();
}

#[no_mangle]
pub unsafe extern "C" fn dc_is_open(context: *mut dc_context_t) -> libc::c_int {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_is_open()");
        return 0;
    }
    let ffi_context = &*context;
    let inner_guard = ffi_context.inner.read().unwrap();
    match *inner_guard {
        Some(_) => 1,
        None => 0,
    }
}

#[no_mangle]
pub unsafe extern "C" fn dc_get_blobdir(context: *mut dc_context_t) -> *mut libc::c_char {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_get_blobdir()");
        return "".strdup();
    }
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| ctx.get_blobdir().to_string_lossy().strdup())
        .unwrap_or_else(|_| "".strdup())
}

#[no_mangle]
pub unsafe extern "C" fn dc_set_config(
    context: *mut dc_context_t,
    key: *const libc::c_char,
    value: *const libc::c_char,
) -> libc::c_int {
    if context.is_null() || key.is_null() {
        eprintln!("ignoring careless call to dc_set_config()");
        return 0;
    }
    let ffi_context = &*context;
    match config::Config::from_str(&to_string_lossy(key)) {
        // When ctx.set_config() fails it already logged the error.
        // TODO: Context::set_config() should not log this
        Ok(key) => ffi_context
            .with_inner(|ctx| {
                ctx.set_config(key, to_opt_string_lossy(value).as_ref().map(|x| x.as_str()))
                    .is_ok() as libc::c_int
            })
            .unwrap_or(0),
        Err(_) => {
            ffi_context.warning("dc_set_config(): invalid key");
            0
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn dc_get_config(
    context: *mut dc_context_t,
    key: *const libc::c_char,
) -> *mut libc::c_char {
    if context.is_null() || key.is_null() {
        eprintln!("ignoring careless call to dc_get_config()");
        return "".strdup();
    }
    let ffi_context = &*context;
    match config::Config::from_str(&to_string_lossy(key)) {
        Ok(key) => ffi_context
            .with_inner(|ctx| ctx.get_config(key).unwrap_or_default().strdup())
            .unwrap_or_else(|_| "".strdup()),
        Err(_) => {
            ffi_context.warning("dc_get_config(): invalid key");
            "".strdup()
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn dc_set_stock_translation(
    context: *mut dc_context_t,
    stock_id: u32,
    stock_msg: *mut libc::c_char,
) -> libc::c_int {
    if context.is_null() || stock_msg.is_null() {
        eprintln!("ignoring careless call to dc_set_stock_string");
        return 0;
    }
    let msg = to_string_lossy(stock_msg);
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| match StockMessage::from_u32(stock_id) {
            Some(id) => match ctx.set_stock_translation(id, msg) {
                Ok(()) => 1,
                Err(err) => {
                    warn!(ctx, "set_stock_translation failed: {}", err);
                    0
                }
            },
            None => {
                warn!(ctx, "invalid stock message id {}", stock_id);
                0
            }
        })
        .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn dc_set_config_from_qr(
    context: *mut dc_context_t,
    qr: *mut libc::c_char,
) -> libc::c_int {
    if context.is_null() || qr.is_null() {
        eprintln!("ignoring careless call to dc_set_config_from_qr");
        return 0;
    }
    let qr = to_string_lossy(qr);
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| match qr::set_config_from_qr(ctx, &qr) {
            Ok(()) => 1,
            Err(err) => {
                error!(ctx, "Failed to create account from QR code: {}", err);
                0
            }
        })
        .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn dc_get_info(context: *mut dc_context_t) -> *mut libc::c_char {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_get_info()");
        return "".strdup();
    }
    let ffi_context = &*context;
    let guard = ffi_context.inner.read().unwrap();
    let info = match guard.as_ref() {
        Some(ref ctx) => ctx.get_info(),
        None => context::get_info(),
    };
    render_info(info).unwrap_or_default().strdup()
}

fn render_info(
    info: HashMap<&'static str, String>,
) -> std::result::Result<String, std::fmt::Error> {
    let mut res = String::new();
    for (key, value) in &info {
        writeln!(&mut res, "{}={}", key, value)?;
    }

    Ok(res)
}

#[no_mangle]
pub unsafe extern "C" fn dc_get_oauth2_url(
    context: *mut dc_context_t,
    addr: *const libc::c_char,
    redirect: *const libc::c_char,
) -> *mut libc::c_char {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_get_oauth2_url()");
        return ptr::null_mut(); // NULL explicitly defined as "unknown"
    }
    let ffi_context = &*context;
    let addr = to_string_lossy(addr);
    let redirect = to_string_lossy(redirect);
    ffi_context
        .with_inner(|ctx| match oauth2::dc_get_oauth2_url(ctx, addr, redirect) {
            Some(res) => res.strdup(),
            None => ptr::null_mut(),
        })
        .unwrap_or_else(|_| ptr::null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn dc_configure(context: *mut dc_context_t) {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_configure()");
        return;
    }
    let ffi_context = &*context;
    ffi_context.with_inner(|ctx| ctx.configure()).unwrap_or(())
}

#[no_mangle]
pub unsafe extern "C" fn dc_is_configured(context: *mut dc_context_t) -> libc::c_int {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_is_configured()");
        return 0;
    }
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| ctx.is_configured() as libc::c_int)
        .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn dc_perform_imap_jobs(context: *mut dc_context_t) {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_perform_imap_jobs()");
        return;
    }
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| job::perform_inbox_jobs(ctx))
        .unwrap_or(())
}

#[no_mangle]
pub unsafe extern "C" fn dc_perform_imap_fetch(context: *mut dc_context_t) {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_perform_imap_fetch()");
        return;
    }
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| job::perform_inbox_fetch(ctx))
        .unwrap_or(())
}

#[no_mangle]
pub unsafe extern "C" fn dc_perform_imap_idle(context: *mut dc_context_t) {
    // TODO rename function in co-ordination with UIs
    if context.is_null() {
        eprintln!("ignoring careless call to dc_perform_imap_idle()");
        return;
    }
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| job::perform_inbox_idle(ctx))
        .unwrap_or(())
}

#[no_mangle]
pub unsafe extern "C" fn dc_interrupt_imap_idle(context: *mut dc_context_t) {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_interrupt_imap_idle()");
        return;
    }
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| job::interrupt_inbox_idle(ctx))
        .unwrap_or(())
}

#[no_mangle]
pub unsafe extern "C" fn dc_perform_mvbox_fetch(context: *mut dc_context_t) {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_perform_mvbox_fetch()");
        return;
    }
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| job::perform_mvbox_fetch(ctx))
        .unwrap_or(())
}

#[no_mangle]
pub unsafe extern "C" fn dc_perform_mvbox_jobs(context: *mut dc_context_t) {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_perform_mvbox_jobs()");
        return;
    }
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| job::perform_mvbox_jobs(ctx))
        .unwrap_or(())
}

#[no_mangle]
pub unsafe extern "C" fn dc_perform_mvbox_idle(context: *mut dc_context_t) {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_perform_mvbox_idle()");
        return;
    }
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| job::perform_mvbox_idle(ctx))
        .unwrap_or(())
}

#[no_mangle]
pub unsafe extern "C" fn dc_interrupt_mvbox_idle(context: *mut dc_context_t) {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_interrupt_mvbox_idle()");
        return;
    }
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| job::interrupt_mvbox_idle(ctx))
        .unwrap_or(())
}

#[no_mangle]
pub unsafe extern "C" fn dc_perform_sentbox_fetch(context: *mut dc_context_t) {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_perform_sentbox_fetch()");
        return;
    }
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| job::perform_sentbox_fetch(ctx))
        .unwrap_or(())
}

#[no_mangle]
pub unsafe extern "C" fn dc_perform_sentbox_jobs(context: *mut dc_context_t) {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_perform_sentbox_jobs()");
        return;
    }
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| job::perform_sentbox_jobs(ctx))
        .unwrap_or(())
}

#[no_mangle]
pub unsafe extern "C" fn dc_perform_sentbox_idle(context: *mut dc_context_t) {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_perform_sentbox_idle()");
        return;
    }
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| job::perform_sentbox_idle(ctx))
        .unwrap_or(())
}

#[no_mangle]
pub unsafe extern "C" fn dc_interrupt_sentbox_idle(context: *mut dc_context_t) {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_interrupt_sentbox_idle()");
        return;
    }
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| job::interrupt_sentbox_idle(ctx))
        .unwrap_or(())
}

#[no_mangle]
pub unsafe extern "C" fn dc_perform_smtp_jobs(context: *mut dc_context_t) {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_perform_smtp_jobs()");
        return;
    }
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| job::perform_smtp_jobs(ctx))
        .unwrap_or(())
}

#[no_mangle]
pub unsafe extern "C" fn dc_perform_smtp_idle(context: *mut dc_context_t) {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_perform_smtp_idle()");
        return;
    }
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| job::perform_smtp_idle(ctx))
        .unwrap_or(())
}

#[no_mangle]
pub unsafe extern "C" fn dc_interrupt_smtp_idle(context: *mut dc_context_t) {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_interrupt_smtp_idle()");
        return;
    }
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| job::interrupt_smtp_idle(ctx))
        .unwrap_or(())
}

#[no_mangle]
pub unsafe extern "C" fn dc_maybe_network(context: *mut dc_context_t) {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_maybe_network()");
        return;
    }
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| job::maybe_network(ctx))
        .unwrap_or(())
}

#[no_mangle]
pub unsafe extern "C" fn dc_preconfigure_keypair(
    context: *mut dc_context_t,
    addr: *const libc::c_char,
    public_data: *const libc::c_char,
    secret_data: *const libc::c_char,
) -> i32 {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_preconfigure_keypair()");
        return 0;
    }
    let ffi_context = &*context;
    ffi_context
        .try_inner(|ctx| {
            let addr = dc_tools::EmailAddress::new(&to_string_lossy(addr))?;
            let public = key::SignedPublicKey::from_base64(&to_string_lossy(public_data))?;
            let secret = key::SignedSecretKey::from_base64(&to_string_lossy(secret_data))?;
            let keypair = key::KeyPair {
                addr,
                public,
                secret,
            };
            key::store_self_keypair(ctx, &keypair, key::KeyPairUse::Default)?;
            Ok(1)
        })
        .log_err(ffi_context, "Failed to save keypair")
        .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn dc_get_chatlist(
    context: *mut dc_context_t,
    flags: libc::c_int,
    query_str: *const libc::c_char,
    query_id: u32,
) -> *mut dc_chatlist_t {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_get_chatlist()");
        return ptr::null_mut();
    }
    let ffi_context = &*context;
    let qs = to_opt_string_lossy(query_str);

    let qi = if query_id == 0 { None } else { Some(query_id) };
    ffi_context
        .with_inner(|ctx| {
            match chatlist::Chatlist::try_load(
                ctx,
                flags as usize,
                qs.as_ref().map(|x| x.as_str()),
                qi,
            ) {
                Ok(list) => {
                    let ffi_list = ChatlistWrapper { context, list };
                    Box::into_raw(Box::new(ffi_list))
                }
                Err(_) => ptr::null_mut(),
            }
        })
        .unwrap_or_else(|_| ptr::null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn dc_create_chat_by_msg_id(context: *mut dc_context_t, msg_id: u32) -> u32 {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_create_chat_by_msg_id()");
        return 0;
    }
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| {
            chat::create_by_msg_id(ctx, MsgId::new(msg_id))
                .log_err(ffi_context, "Failed to create chat from msg_id")
                .map(|id| id.to_u32())
                .unwrap_or(0)
        })
        .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn dc_create_chat_by_contact_id(
    context: *mut dc_context_t,
    contact_id: u32,
) -> u32 {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_create_chat_by_contact_id()");
        return 0;
    }
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| {
            chat::create_by_contact_id(ctx, contact_id)
                .log_err(ffi_context, "Failed to create chat from contact_id")
                .map(|id| id.to_u32())
                .unwrap_or(0)
        })
        .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn dc_get_chat_id_by_contact_id(
    context: *mut dc_context_t,
    contact_id: u32,
) -> u32 {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_get_chat_id_by_contact_id()");
        return 0;
    }
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| {
            chat::get_by_contact_id(ctx, contact_id)
                .log_err(ffi_context, "Failed to get chat for contact_id")
                .map(|id| id.to_u32())
                .unwrap_or(0)
        })
        .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn dc_prepare_msg(
    context: *mut dc_context_t,
    chat_id: u32,
    msg: *mut dc_msg_t,
) -> u32 {
    if context.is_null() || chat_id == 0 || msg.is_null() {
        eprintln!("ignoring careless call to dc_prepare_msg()");
        return 0;
    }
    let ffi_context = &mut *context;
    let ffi_msg: &mut MessageWrapper = &mut *msg;
    ffi_context
        .with_inner(|ctx| {
            chat::prepare_msg(ctx, ChatId::new(chat_id), &mut ffi_msg.message)
                .unwrap_or_log_default(ctx, "Failed to prepare message")
        })
        .map(|msg_id| msg_id.to_u32())
        .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn dc_send_msg(
    context: *mut dc_context_t,
    chat_id: u32,
    msg: *mut dc_msg_t,
) -> u32 {
    if context.is_null() || msg.is_null() {
        eprintln!("ignoring careless call to dc_send_msg()");
        return 0;
    }
    let ffi_context = &mut *context;
    let ffi_msg = &mut *msg;
    ffi_context
        .with_inner(|ctx| {
            chat::send_msg(ctx, ChatId::new(chat_id), &mut ffi_msg.message)
                .unwrap_or_log_default(ctx, "Failed to send message")
        })
        .map(|msg_id| msg_id.to_u32())
        .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn dc_send_text_msg(
    context: *mut dc_context_t,
    chat_id: u32,
    text_to_send: *const libc::c_char,
) -> u32 {
    if context.is_null() || text_to_send.is_null() {
        eprintln!("ignoring careless call to dc_send_text_msg()");
        return 0;
    }
    let ffi_context = &*context;
    let text_to_send = to_string_lossy(text_to_send);
    ffi_context
        .with_inner(|ctx| {
            chat::send_text_msg(ctx, ChatId::new(chat_id), text_to_send)
                .map(|msg_id| msg_id.to_u32())
                .unwrap_or_log_default(ctx, "Failed to send text message")
        })
        .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn dc_set_draft(
    context: *mut dc_context_t,
    chat_id: u32,
    msg: *mut dc_msg_t,
) {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_set_draft()");
        return;
    }
    let ffi_context = &*context;
    let msg = if msg.is_null() {
        None
    } else {
        let ffi_msg: &mut MessageWrapper = &mut *msg;
        Some(&mut ffi_msg.message)
    };
    ffi_context
        .with_inner(|ctx| ChatId::new(chat_id).set_draft(ctx, msg))
        .unwrap_or(())
}

#[no_mangle]
pub unsafe extern "C" fn dc_add_device_msg(
    context: *mut dc_context_t,
    label: *const libc::c_char,
    msg: *mut dc_msg_t,
) -> u32 {
    if context.is_null() || (label.is_null() && msg.is_null()) {
        eprintln!("ignoring careless call to dc_add_device_msg()");
        return 0;
    }
    let ffi_context = &mut *context;
    let msg = if msg.is_null() {
        None
    } else {
        let ffi_msg: &mut MessageWrapper = &mut *msg;
        Some(&mut ffi_msg.message)
    };
    ffi_context
        .with_inner(|ctx| {
            chat::add_device_msg(
                ctx,
                to_opt_string_lossy(label).as_ref().map(|x| x.as_str()),
                msg,
            )
            .unwrap_or_log_default(ctx, "Failed to add device message")
        })
        .map(|msg_id| msg_id.to_u32())
        .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn dc_update_device_chats(context: *mut dc_context_t) {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_update_device_chats()");
        return;
    }
    let ffi_context = &mut *context;
    ffi_context
        .with_inner(|ctx| {
            ctx.update_device_chats()
                .unwrap_or_log_default(ctx, "Failed to add device message")
        })
        .unwrap_or(())
}

#[no_mangle]
pub unsafe extern "C" fn dc_was_device_msg_ever_added(
    context: *mut dc_context_t,
    label: *const libc::c_char,
) -> libc::c_int {
    if context.is_null() || label.is_null() {
        eprintln!("ignoring careless call to dc_was_device_msg_ever_added()");
        return 0;
    }
    let ffi_context = &mut *context;
    ffi_context
        .with_inner(|ctx| {
            chat::was_device_msg_ever_added(ctx, &to_string_lossy(label)).unwrap_or(false)
                as libc::c_int
        })
        .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn dc_get_draft(context: *mut dc_context_t, chat_id: u32) -> *mut dc_msg_t {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_get_draft()");
        return ptr::null_mut(); // NULL explicitly defined as "no draft"
    }
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| match ChatId::new(chat_id).get_draft(ctx) {
            Ok(Some(draft)) => {
                let ffi_msg = MessageWrapper {
                    context,
                    message: draft,
                };
                Box::into_raw(Box::new(ffi_msg))
            }
            Ok(None) => ptr::null_mut(),
            Err(err) => {
                error!(ctx, "Failed to get draft for chat #{}: {}", chat_id, err);
                ptr::null_mut()
            }
        })
        .unwrap_or_else(|_| ptr::null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn dc_get_chat_msgs(
    context: *mut dc_context_t,
    chat_id: u32,
    flags: u32,
    marker1before: u32,
) -> *mut dc_array::dc_array_t {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_get_chat_msgs()");
        return ptr::null_mut();
    }
    let ffi_context = &*context;
    let marker_flag = if marker1before <= DC_MSG_ID_LAST_SPECIAL {
        None
    } else {
        Some(MsgId::new(marker1before))
    };
    ffi_context
        .with_inner(|ctx| {
            let arr = dc_array_t::from(
                chat::get_chat_msgs(ctx, ChatId::new(chat_id), flags, marker_flag)
                    .iter()
                    .map(|msg_id| msg_id.to_u32())
                    .collect::<Vec<u32>>(),
            );
            Box::into_raw(Box::new(arr))
        })
        .unwrap_or_else(|_| ptr::null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn dc_get_msg_cnt(context: *mut dc_context_t, chat_id: u32) -> libc::c_int {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_get_msg_cnt()");
        return 0;
    }
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| ChatId::new(chat_id).get_msg_cnt(ctx) as libc::c_int)
        .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn dc_get_fresh_msg_cnt(
    context: *mut dc_context_t,
    chat_id: u32,
) -> libc::c_int {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_get_fresh_msg_cnt()");
        return 0;
    }
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| ChatId::new(chat_id).get_fresh_msg_cnt(ctx) as libc::c_int)
        .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn dc_estimate_deletion_cnt(
    context: *mut dc_context_t,
    from_server: libc::c_int,
    seconds: i64,
) -> libc::c_int {
    if context.is_null() || seconds < 0 {
        eprintln!("ignoring careless call to dc_estimate_deletion_cnt()");
        return 0;
    }
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| {
            message::estimate_deletion_cnt(ctx, from_server != 0, seconds).unwrap_or(0)
                as libc::c_int
        })
        .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn dc_get_fresh_msgs(
    context: *mut dc_context_t,
) -> *mut dc_array::dc_array_t {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_get_fresh_msgs()");
        return ptr::null_mut();
    }
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| {
            let arr = dc_array_t::from(
                ctx.get_fresh_msgs()
                    .iter()
                    .map(|msg_id| msg_id.to_u32())
                    .collect::<Vec<u32>>(),
            );
            Box::into_raw(Box::new(arr))
        })
        .unwrap_or_else(|_| ptr::null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn dc_marknoticed_chat(context: *mut dc_context_t, chat_id: u32) {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_marknoticed_chat()");
        return;
    }
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| {
            chat::marknoticed_chat(ctx, ChatId::new(chat_id))
                .log_err(ffi_context, "Failed marknoticed chat")
                .unwrap_or(())
        })
        .unwrap_or(())
}

#[no_mangle]
pub unsafe extern "C" fn dc_marknoticed_all_chats(context: *mut dc_context_t) {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_marknoticed_all_chats()");
        return;
    }
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| {
            chat::marknoticed_all_chats(ctx)
                .log_err(ffi_context, "Failed marknoticed all chats")
                .unwrap_or(())
        })
        .unwrap_or(())
}

fn from_prim<S, T>(s: S) -> Option<T>
where
    T: FromPrimitive,
    S: Into<i64>,
{
    FromPrimitive::from_i64(s.into())
}

#[no_mangle]
pub unsafe extern "C" fn dc_get_chat_media(
    context: *mut dc_context_t,
    chat_id: u32,
    msg_type: libc::c_int,
    or_msg_type2: libc::c_int,
    or_msg_type3: libc::c_int,
) -> *mut dc_array::dc_array_t {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_get_chat_media()");
        return ptr::null_mut();
    }
    let ffi_context = &*context;
    let msg_type = from_prim(msg_type).expect(&format!("invalid msg_type = {}", msg_type));
    let or_msg_type2 =
        from_prim(or_msg_type2).expect(&format!("incorrect or_msg_type2 = {}", or_msg_type2));
    let or_msg_type3 =
        from_prim(or_msg_type3).expect(&format!("incorrect or_msg_type3 = {}", or_msg_type3));
    ffi_context
        .with_inner(|ctx| {
            let arr = dc_array_t::from(
                chat::get_chat_media(
                    ctx,
                    ChatId::new(chat_id),
                    msg_type,
                    or_msg_type2,
                    or_msg_type3,
                )
                .iter()
                .map(|msg_id| msg_id.to_u32())
                .collect::<Vec<u32>>(),
            );
            Box::into_raw(Box::new(arr))
        })
        .unwrap_or_else(|_| ptr::null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn dc_get_next_media(
    context: *mut dc_context_t,
    msg_id: u32,
    dir: libc::c_int,
    msg_type: libc::c_int,
    or_msg_type2: libc::c_int,
    or_msg_type3: libc::c_int,
) -> u32 {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_get_next_media()");
        return 0;
    }
    let direction = if dir < 0 {
        chat::Direction::Backward
    } else {
        chat::Direction::Forward
    };

    let ffi_context = &*context;
    let msg_type = from_prim(msg_type).expect(&format!("invalid msg_type = {}", msg_type));
    let or_msg_type2 =
        from_prim(or_msg_type2).expect(&format!("incorrect or_msg_type2 = {}", or_msg_type2));
    let or_msg_type3 =
        from_prim(or_msg_type3).expect(&format!("incorrect or_msg_type3 = {}", or_msg_type3));
    ffi_context
        .with_inner(|ctx| {
            chat::get_next_media(
                ctx,
                MsgId::new(msg_id),
                direction,
                msg_type,
                or_msg_type2,
                or_msg_type3,
            )
            .map(|msg_id| msg_id.to_u32())
            .unwrap_or(0)
        })
        .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn dc_set_chat_visibility(
    context: *mut dc_context_t,
    chat_id: u32,
    archive: libc::c_int,
) {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_set_chat_visibility()");
        return;
    }
    let ffi_context = &*context;
    let visibility = match archive {
        0 => ChatVisibility::Normal,
        1 => ChatVisibility::Archived,
        2 => ChatVisibility::Pinned,
        _ => {
            ffi_context.warning(
                "ignoring careless call to dc_set_chat_visibility(): unknown archived state",
            );
            return;
        }
    };
    ffi_context
        .with_inner(|ctx| {
            ChatId::new(chat_id)
                .set_visibility(ctx, visibility)
                .log_err(ffi_context, "Failed setting chat visibility")
                .unwrap_or(())
        })
        .unwrap_or(())
}

#[no_mangle]
pub unsafe extern "C" fn dc_delete_chat(context: *mut dc_context_t, chat_id: u32) {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_delete_chat()");
        return;
    }
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| {
            ChatId::new(chat_id)
                .delete(ctx)
                .log_err(ffi_context, "Failed chat delete")
                .unwrap_or(())
        })
        .unwrap_or(())
}

#[no_mangle]
pub unsafe extern "C" fn dc_get_chat_contacts(
    context: *mut dc_context_t,
    chat_id: u32,
) -> *mut dc_array::dc_array_t {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_get_chat_contacts()");
        return ptr::null_mut();
    }
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| {
            let arr = dc_array_t::from(chat::get_chat_contacts(ctx, ChatId::new(chat_id)));
            Box::into_raw(Box::new(arr))
        })
        .unwrap_or_else(|_| ptr::null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn dc_search_msgs(
    context: *mut dc_context_t,
    chat_id: u32,
    query: *const libc::c_char,
) -> *mut dc_array::dc_array_t {
    if context.is_null() || query.is_null() {
        eprintln!("ignoring careless call to dc_search_msgs()");
        return ptr::null_mut();
    }
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| {
            let arr = dc_array_t::from(
                ctx.search_msgs(ChatId::new(chat_id), to_string_lossy(query))
                    .iter()
                    .map(|msg_id| msg_id.to_u32())
                    .collect::<Vec<u32>>(),
            );
            Box::into_raw(Box::new(arr))
        })
        .unwrap_or_else(|_| ptr::null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn dc_get_chat(context: *mut dc_context_t, chat_id: u32) -> *mut dc_chat_t {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_get_chat()");
        return ptr::null_mut();
    }
    let ffi_context = &*context;
    ffi_context
        .with_inner(
            |ctx| match chat::Chat::load_from_db(ctx, ChatId::new(chat_id)) {
                Ok(chat) => {
                    let ffi_chat = ChatWrapper { context, chat };
                    Box::into_raw(Box::new(ffi_chat))
                }
                Err(_) => ptr::null_mut(),
            },
        )
        .unwrap_or_else(|_| ptr::null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn dc_create_group_chat(
    context: *mut dc_context_t,
    verified: libc::c_int,
    name: *const libc::c_char,
) -> u32 {
    if context.is_null() || name.is_null() {
        eprintln!("ignoring careless call to dc_create_group_chat()");
        return 0;
    }
    let ffi_context = &*context;
    let verified = if let Some(s) = contact::VerifiedStatus::from_i32(verified) {
        s
    } else {
        return 0;
    };
    ffi_context
        .with_inner(|ctx| {
            chat::create_group_chat(ctx, verified, to_string_lossy(name))
                .log_err(ffi_context, "Failed to create group chat")
                .map(|id| id.to_u32())
                .unwrap_or(0)
        })
        .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn dc_is_contact_in_chat(
    context: *mut dc_context_t,
    chat_id: u32,
    contact_id: u32,
) -> libc::c_int {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_is_contact_in_chat()");
        return 0;
    }
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| chat::is_contact_in_chat(ctx, ChatId::new(chat_id), contact_id))
        .unwrap_or_default()
        .into()
}

#[no_mangle]
pub unsafe extern "C" fn dc_add_contact_to_chat(
    context: *mut dc_context_t,
    chat_id: u32,
    contact_id: u32,
) -> libc::c_int {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_add_contact_to_chat()");
        return 0;
    }
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| {
            chat::add_contact_to_chat(ctx, ChatId::new(chat_id), contact_id) as libc::c_int
        })
        .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn dc_remove_contact_from_chat(
    context: *mut dc_context_t,
    chat_id: u32,
    contact_id: u32,
) -> libc::c_int {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_remove_contact_from_chat()");
        return 0;
    }
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| {
            chat::remove_contact_from_chat(ctx, ChatId::new(chat_id), contact_id)
                .map(|_| 1)
                .unwrap_or_log_default(ctx, "Failed to remove contact")
        })
        .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn dc_set_chat_name(
    context: *mut dc_context_t,
    chat_id: u32,
    name: *const libc::c_char,
) -> libc::c_int {
    if context.is_null() || chat_id <= constants::DC_CHAT_ID_LAST_SPECIAL as u32 || name.is_null() {
        eprintln!("ignoring careless call to dc_set_chat_name()");
        return 0;
    }
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| {
            chat::set_chat_name(ctx, ChatId::new(chat_id), to_string_lossy(name))
                .map(|_| 1)
                .unwrap_or_log_default(ctx, "Failed to set chat name")
        })
        .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn dc_set_chat_profile_image(
    context: *mut dc_context_t,
    chat_id: u32,
    image: *const libc::c_char,
) -> libc::c_int {
    if context.is_null() || chat_id <= constants::DC_CHAT_ID_LAST_SPECIAL as u32 {
        eprintln!("ignoring careless call to dc_set_chat_profile_image()");
        return 0;
    }
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| {
            chat::set_chat_profile_image(ctx, ChatId::new(chat_id), to_string_lossy(image))
                .map(|_| 1)
                .unwrap_or_log_default(ctx, "Failed to set profile image")
        })
        .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn dc_set_chat_mute_duration(
    context: *mut dc_context_t,
    chat_id: u32,
    duration: i64,
) -> libc::c_int {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_set_chat_mute_duration()");
        return 0;
    }
    let ffi_context = &*context;
    let muteDuration = match duration {
        0 => MuteDuration::NotMuted,
        -1 => MuteDuration::Forever,
        n if n > 0 => MuteDuration::Until(SystemTime::now() + Duration::from_secs(duration as u64)),
        _ => {
            ffi_context.warning(
                "dc_chat_set_mute_duration(): Can not use negative duration other than -1",
            );
            return 0;
        }
    };
    ffi_context
        .with_inner(|ctx| {
            chat::set_muted(ctx, ChatId::new(chat_id), muteDuration)
                .map(|_| 1)
                .unwrap_or_log_default(ctx, "Failed to set mute duration")
        })
        .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn dc_get_msg_info(
    context: *mut dc_context_t,
    msg_id: u32,
) -> *mut libc::c_char {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_get_msg_info()");
        return "".strdup();
    }
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| message::get_msg_info(ctx, MsgId::new(msg_id)).strdup())
        .unwrap_or_else(|_| ptr::null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn dc_get_mime_headers(
    context: *mut dc_context_t,
    msg_id: u32,
) -> *mut libc::c_char {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_get_mime_headers()");
        return ptr::null_mut(); // NULL explicitly defined as "no mime headers"
    }
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| {
            message::get_mime_headers(ctx, MsgId::new(msg_id))
                .map(|s| s.strdup())
                .unwrap_or_else(ptr::null_mut)
        })
        .unwrap_or_else(|_| ptr::null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn dc_delete_msgs(
    context: *mut dc_context_t,
    msg_ids: *const u32,
    msg_cnt: libc::c_int,
) {
    if context.is_null() || msg_ids.is_null() || msg_cnt <= 0 {
        eprintln!("ignoring careless call to dc_delete_msgs()");
        return;
    }
    let ffi_context = &*context;
    let msg_ids = convert_and_prune_message_ids(msg_ids, msg_cnt);
    ffi_context
        .with_inner(|ctx| message::delete_msgs(ctx, &msg_ids[..]))
        .unwrap_or(())
}

#[no_mangle]
pub unsafe extern "C" fn dc_empty_server(context: *mut dc_context_t, flags: u32) {
    if context.is_null() || flags == 0 {
        eprintln!("ignoring careless call to dc_empty_server()");
        return;
    }
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| message::dc_empty_server(ctx, flags))
        .unwrap_or(())
}

#[no_mangle]
pub unsafe extern "C" fn dc_forward_msgs(
    context: *mut dc_context_t,
    msg_ids: *const u32,
    msg_cnt: libc::c_int,
    chat_id: u32,
) {
    if context.is_null()
        || msg_ids.is_null()
        || msg_cnt <= 0
        || chat_id <= constants::DC_CHAT_ID_LAST_SPECIAL as u32
    {
        eprintln!("ignoring careless call to dc_forward_msgs()");
        return;
    }
    let msg_ids = convert_and_prune_message_ids(msg_ids, msg_cnt);
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| {
            chat::forward_msgs(ctx, &msg_ids[..], ChatId::new(chat_id))
                .unwrap_or_log_default(ctx, "Failed to forward message")
        })
        .unwrap_or_default()
}

#[no_mangle]
pub unsafe extern "C" fn dc_marknoticed_contact(context: *mut dc_context_t, contact_id: u32) {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_marknoticed_contact()");
        return;
    }
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| Contact::mark_noticed(ctx, contact_id))
        .unwrap_or(())
}

#[no_mangle]
pub unsafe extern "C" fn dc_markseen_msgs(
    context: *mut dc_context_t,
    msg_ids: *const u32,
    msg_cnt: libc::c_int,
) {
    if context.is_null() || msg_ids.is_null() || msg_cnt <= 0 {
        eprintln!("ignoring careless call to dc_markseen_msgs()");
        return;
    }
    let msg_ids = convert_and_prune_message_ids(msg_ids, msg_cnt);
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| message::markseen_msgs(ctx, &msg_ids[..]))
        .ok();
}

#[no_mangle]
pub unsafe extern "C" fn dc_star_msgs(
    context: *mut dc_context_t,
    msg_ids: *const u32,
    msg_cnt: libc::c_int,
    star: libc::c_int,
) {
    if context.is_null() || msg_ids.is_null() || msg_cnt <= 0 {
        eprintln!("ignoring careless call to dc_star_msgs()");
        return;
    }
    let msg_ids = convert_and_prune_message_ids(msg_ids, msg_cnt);
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| message::star_msgs(ctx, &msg_ids[..], star == 1))
        .ok();
}

#[no_mangle]
pub unsafe extern "C" fn dc_get_msg(context: *mut dc_context_t, msg_id: u32) -> *mut dc_msg_t {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_get_msg()");
        return ptr::null_mut();
    }
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| {
            let message = match message::Message::load_from_db(ctx, MsgId::new(msg_id)) {
                Ok(msg) => msg,
                Err(e) => {
                    if msg_id <= constants::DC_MSG_ID_LAST_SPECIAL {
                        // C-core API returns empty messages, do the same
                        warn!(
                            ctx,
                            "dc_get_msg called with special msg_id={}, returning empty msg", msg_id
                        );
                        message::Message::default()
                    } else {
                        error!(
                            ctx,
                            "dc_get_msg could not retrieve msg_id {}: {}", msg_id, e
                        );
                        return ptr::null_mut();
                    }
                }
            };
            let ffi_msg = MessageWrapper { context, message };
            Box::into_raw(Box::new(ffi_msg))
        })
        .unwrap_or_else(|_| ptr::null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn dc_may_be_valid_addr(addr: *const libc::c_char) -> libc::c_int {
    if addr.is_null() {
        eprintln!("ignoring careless call to dc_may_be_valid_addr()");
        return 0;
    }

    contact::may_be_valid_addr(&to_string_lossy(addr)) as libc::c_int
}

#[no_mangle]
pub unsafe extern "C" fn dc_lookup_contact_id_by_addr(
    context: *mut dc_context_t,
    addr: *const libc::c_char,
) -> u32 {
    if context.is_null() || addr.is_null() {
        eprintln!("ignoring careless call to dc_lookup_contact_id_by_addr()");
        return 0;
    }
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| {
            Contact::lookup_id_by_addr(ctx, to_string_lossy(addr), Origin::IncomingReplyTo)
        })
        .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn dc_create_contact(
    context: *mut dc_context_t,
    name: *const libc::c_char,
    addr: *const libc::c_char,
) -> u32 {
    if context.is_null() || addr.is_null() {
        eprintln!("ignoring careless call to dc_create_contact()");
        return 0;
    }
    let ffi_context = &*context;
    let name = to_string_lossy(name);
    ffi_context
        .with_inner(
            |ctx| match Contact::create(ctx, name, to_string_lossy(addr)) {
                Ok(id) => id,
                Err(_) => 0,
            },
        )
        .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn dc_add_address_book(
    context: *mut dc_context_t,
    addr_book: *const libc::c_char,
) -> libc::c_int {
    if context.is_null() || addr_book.is_null() {
        eprintln!("ignoring careless call to dc_add_address_book()");
        return 0;
    }
    let ffi_context = &*context;
    ffi_context
        .with_inner(
            |ctx| match Contact::add_address_book(ctx, to_string_lossy(addr_book)) {
                Ok(cnt) => cnt as libc::c_int,
                Err(_) => 0,
            },
        )
        .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn dc_get_contacts(
    context: *mut dc_context_t,
    flags: u32,
    query: *const libc::c_char,
) -> *mut dc_array::dc_array_t {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_get_contacts()");
        return ptr::null_mut();
    }
    let ffi_context = &*context;
    let query = to_opt_string_lossy(query);
    ffi_context
        .with_inner(|ctx| match Contact::get_all(ctx, flags, query) {
            Ok(contacts) => Box::into_raw(Box::new(dc_array_t::from(contacts))),
            Err(_) => ptr::null_mut(),
        })
        .unwrap_or_else(|_| ptr::null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn dc_get_blocked_cnt(context: *mut dc_context_t) -> libc::c_int {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_get_blocked_cnt()");
        return 0;
    }
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| Contact::get_blocked_cnt(ctx) as libc::c_int)
        .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn dc_get_blocked_contacts(
    context: *mut dc_context_t,
) -> *mut dc_array::dc_array_t {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_get_blocked_contacts()");
        return ptr::null_mut();
    }
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| Box::into_raw(Box::new(dc_array_t::from(Contact::get_all_blocked(ctx)))))
        .unwrap_or_else(|_| ptr::null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn dc_block_contact(
    context: *mut dc_context_t,
    contact_id: u32,
    block: libc::c_int,
) {
    if context.is_null() || contact_id <= constants::DC_CONTACT_ID_LAST_SPECIAL as u32 {
        eprintln!("ignoring careless call to dc_block_contact()");
        return;
    }
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| {
            if block == 0 {
                Contact::unblock(ctx, contact_id);
            } else {
                Contact::block(ctx, contact_id);
            }
        })
        .ok();
}

#[no_mangle]
pub unsafe extern "C" fn dc_get_contact_encrinfo(
    context: *mut dc_context_t,
    contact_id: u32,
) -> *mut libc::c_char {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_get_contact_encrinfo()");
        return "".strdup();
    }
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| {
            Contact::get_encrinfo(ctx, contact_id)
                .map(|s| s.strdup())
                .unwrap_or_else(|e| {
                    error!(ctx, "{}", e);
                    ptr::null_mut()
                })
        })
        .unwrap_or_else(|_| "".strdup())
}

#[no_mangle]
pub unsafe extern "C" fn dc_delete_contact(
    context: *mut dc_context_t,
    contact_id: u32,
) -> libc::c_int {
    if context.is_null() || contact_id <= constants::DC_CONTACT_ID_LAST_SPECIAL as u32 {
        eprintln!("ignoring careless call to dc_delete_contact()");
        return 0;
    }
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| match Contact::delete(ctx, contact_id) {
            Ok(_) => 1,
            Err(_) => 0,
        })
        .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn dc_get_contact(
    context: *mut dc_context_t,
    contact_id: u32,
) -> *mut dc_contact_t {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_get_contact()");
        return ptr::null_mut();
    }
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| {
            Contact::get_by_id(ctx, contact_id)
                .map(|contact| Box::into_raw(Box::new(ContactWrapper { context, contact })))
                .unwrap_or_else(|_| ptr::null_mut())
        })
        .unwrap_or_else(|_| ptr::null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn dc_imex(
    context: *mut dc_context_t,
    what: libc::c_int,
    param1: *const libc::c_char,
    _param2: *const libc::c_char,
) {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_imex()");
        return;
    }
    let what = match imex::ImexMode::from_i32(what as i32) {
        Some(what) => what,
        None => {
            eprintln!("ignoring invalid argument {} to dc_imex", what);
            return;
        }
    };

    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| imex::imex(ctx, what, to_opt_string_lossy(param1)))
        .ok();
}

#[no_mangle]
pub unsafe extern "C" fn dc_imex_has_backup(
    context: *mut dc_context_t,
    dir: *const libc::c_char,
) -> *mut libc::c_char {
    if context.is_null() || dir.is_null() {
        eprintln!("ignoring careless call to dc_imex_has_backup()");
        return ptr::null_mut(); // NULL explicitly defined as "has no backup"
    }
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| match imex::has_backup(ctx, to_string_lossy(dir)) {
            Ok(res) => res.strdup(),
            Err(err) => {
                // do not bubble up error to the user,
                // the ui will expect that the file does not exist or cannot be accessed
                warn!(ctx, "dc_imex_has_backup: {}", err);
                ptr::null_mut()
            }
        })
        .unwrap_or_else(|_| ptr::null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn dc_initiate_key_transfer(context: *mut dc_context_t) -> *mut libc::c_char {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_initiate_key_transfer()");
        return ptr::null_mut(); // NULL explicitly defined as "error"
    }
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| match imex::initiate_key_transfer(ctx) {
            Ok(res) => res.strdup(),
            Err(err) => {
                error!(ctx, "dc_initiate_key_transfer(): {}", err);
                ptr::null_mut()
            }
        })
        .unwrap_or_else(|_| ptr::null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn dc_continue_key_transfer(
    context: *mut dc_context_t,
    msg_id: u32,
    setup_code: *const libc::c_char,
) -> libc::c_int {
    if context.is_null()
        || msg_id <= constants::DC_MSG_ID_LAST_SPECIAL as u32
        || setup_code.is_null()
    {
        eprintln!("ignoring careless call to dc_continue_key_transfer()");
        return 0;
    }
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| {
            match imex::continue_key_transfer(ctx, MsgId::new(msg_id), &to_string_lossy(setup_code))
            {
                Ok(()) => 1,
                Err(err) => {
                    error!(ctx, "dc_continue_key_transfer: {}", err);
                    0
                }
            }
        })
        .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn dc_stop_ongoing_process(context: *mut dc_context_t) {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_stop_ongoing_process()");
        return;
    }
    let ffi_context = &*context;
    ffi_context.with_inner(|ctx| ctx.stop_ongoing()).ok();
}

#[no_mangle]
pub unsafe extern "C" fn dc_check_qr(
    context: *mut dc_context_t,
    qr: *const libc::c_char,
) -> *mut dc_lot_t {
    if context.is_null() || qr.is_null() {
        eprintln!("ignoring careless call to dc_check_qr()");
        return ptr::null_mut();
    }
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| {
            let lot = qr::check_qr(ctx, to_string_lossy(qr));
            Box::into_raw(Box::new(lot))
        })
        .unwrap_or_else(|_| ptr::null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn dc_get_securejoin_qr(
    context: *mut dc_context_t,
    chat_id: u32,
) -> *mut libc::c_char {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_get_securejoin_qr()");
        return "".strdup();
    }
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| {
            securejoin::dc_get_securejoin_qr(ctx, ChatId::new(chat_id))
                .unwrap_or_else(|| "".to_string())
                .strdup()
        })
        .unwrap_or_else(|_| "".strdup())
}

#[no_mangle]
pub unsafe extern "C" fn dc_join_securejoin(
    context: *mut dc_context_t,
    qr: *const libc::c_char,
) -> u32 {
    if context.is_null() || qr.is_null() {
        eprintln!("ignoring careless call to dc_join_securejoin()");
        return 0;
    }
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| securejoin::dc_join_securejoin(ctx, &to_string_lossy(qr)).to_u32())
        .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn dc_send_locations_to_chat(
    context: *mut dc_context_t,
    chat_id: u32,
    seconds: libc::c_int,
) {
    if context.is_null() || chat_id <= constants::DC_CHAT_ID_LAST_SPECIAL as u32 || seconds < 0 {
        eprintln!("ignoring careless call to dc_send_locations_to_chat()");
        return;
    }
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| {
            location::send_locations_to_chat(ctx, ChatId::new(chat_id), seconds as i64)
        })
        .ok();
}

#[no_mangle]
pub unsafe extern "C" fn dc_is_sending_locations_to_chat(
    context: *mut dc_context_t,
    chat_id: u32,
) -> libc::c_int {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_is_sending_locations_to_chat()");
        return 0;
    }
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| {
            location::is_sending_locations_to_chat(ctx, ChatId::new(chat_id)) as libc::c_int
        })
        .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn dc_set_location(
    context: *mut dc_context_t,
    latitude: libc::c_double,
    longitude: libc::c_double,
    accuracy: libc::c_double,
) -> libc::c_int {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_set_location()");
        return 0;
    }
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| location::set(ctx, latitude, longitude, accuracy))
        .unwrap_or(false) as _
}

#[no_mangle]
pub unsafe extern "C" fn dc_get_locations(
    context: *mut dc_context_t,
    chat_id: u32,
    contact_id: u32,
    timestamp_begin: i64,
    timestamp_end: i64,
) -> *mut dc_array::dc_array_t {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_get_locations()");
        return ptr::null_mut();
    }
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| {
            let res = location::get_range(
                ctx,
                ChatId::new(chat_id),
                contact_id,
                timestamp_begin as i64,
                timestamp_end as i64,
            );
            Box::into_raw(Box::new(dc_array_t::from(res)))
        })
        .unwrap_or_else(|_| ptr::null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn dc_delete_all_locations(context: *mut dc_context_t) {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_delete_all_locations()");
        return;
    }
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| {
            location::delete_all(ctx).log_err(ffi_context, "Failed to delete locations")
        })
        .ok();
}

// dc_array_t

#[no_mangle]
pub type dc_array_t = dc_array::dc_array_t;

#[no_mangle]
pub unsafe extern "C" fn dc_array_unref(a: *mut dc_array::dc_array_t) {
    if a.is_null() {
        eprintln!("ignoring careless call to dc_array_unref()");
        return;
    }

    Box::from_raw(a);
}

#[no_mangle]
pub unsafe extern "C" fn dc_array_get_cnt(array: *const dc_array_t) -> libc::size_t {
    if array.is_null() {
        eprintln!("ignoring careless call to dc_array_get_cnt()");
        return 0;
    }

    (*array).len()
}
#[no_mangle]
pub unsafe extern "C" fn dc_array_get_id(array: *const dc_array_t, index: libc::size_t) -> u32 {
    if array.is_null() {
        eprintln!("ignoring careless call to dc_array_get_id()");
        return 0;
    }

    (*array).get_id(index)
}
#[no_mangle]
pub unsafe extern "C" fn dc_array_get_latitude(
    array: *const dc_array_t,
    index: libc::size_t,
) -> libc::c_double {
    if array.is_null() {
        eprintln!("ignoring careless call to dc_array_get_latitude()");
        return 0.0;
    }

    (*array).get_location(index).latitude
}
#[no_mangle]
pub unsafe extern "C" fn dc_array_get_longitude(
    array: *const dc_array_t,
    index: libc::size_t,
) -> libc::c_double {
    if array.is_null() {
        eprintln!("ignoring careless call to dc_array_get_longitude()");
        return 0.0;
    }

    (*array).get_location(index).longitude
}
#[no_mangle]
pub unsafe extern "C" fn dc_array_get_accuracy(
    array: *const dc_array_t,
    index: libc::size_t,
) -> libc::c_double {
    if array.is_null() {
        eprintln!("ignoring careless call to dc_array_get_accuracy()");
        return 0.0;
    }

    (*array).get_location(index).accuracy
}
#[no_mangle]
pub unsafe extern "C" fn dc_array_get_timestamp(
    array: *const dc_array_t,
    index: libc::size_t,
) -> i64 {
    if array.is_null() {
        eprintln!("ignoring careless call to dc_array_get_timestamp()");
        return 0;
    }

    (*array).get_location(index).timestamp
}
#[no_mangle]
pub unsafe extern "C" fn dc_array_get_chat_id(
    array: *const dc_array_t,
    index: libc::size_t,
) -> libc::c_uint {
    if array.is_null() {
        eprintln!("ignoring careless call to dc_array_get_chat_id()");
        return 0;
    }
    (*array).get_location(index).chat_id.to_u32()
}
#[no_mangle]
pub unsafe extern "C" fn dc_array_get_contact_id(
    array: *const dc_array_t,
    index: libc::size_t,
) -> libc::c_uint {
    if array.is_null() {
        eprintln!("ignoring careless call to dc_array_get_contact_id()");
        return 0;
    }

    (*array).get_location(index).contact_id
}
#[no_mangle]
pub unsafe extern "C" fn dc_array_get_msg_id(
    array: *const dc_array_t,
    index: libc::size_t,
) -> libc::c_uint {
    if array.is_null() {
        eprintln!("ignoring careless call to dc_array_get_msg_id()");
        return 0;
    }

    (*array).get_location(index).msg_id
}
#[no_mangle]
pub unsafe extern "C" fn dc_array_get_marker(
    array: *const dc_array_t,
    index: libc::size_t,
) -> *mut libc::c_char {
    if array.is_null() {
        eprintln!("ignoring careless call to dc_array_get_marker()");
        return std::ptr::null_mut(); // NULL explicitly defined as "no markers"
    }

    if let Some(s) = &(*array).get_location(index).marker {
        s.strdup()
    } else {
        std::ptr::null_mut()
    }
}

#[no_mangle]
pub unsafe extern "C" fn dc_array_search_id(
    array: *const dc_array_t,
    needle: libc::c_uint,
    ret_index: *mut libc::size_t,
) -> libc::c_int {
    if array.is_null() {
        eprintln!("ignoring careless call to dc_array_search_id()");
        return 0;
    }

    if let Some(i) = (*array).search_id(needle) {
        if !ret_index.is_null() {
            *ret_index = i
        }
        1
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn dc_array_get_raw(array: *const dc_array_t) -> *const u32 {
    if array.is_null() {
        eprintln!("ignoring careless call to dc_array_get_raw()");
        return ptr::null_mut();
    }

    (*array).as_ptr()
}

// Return the independent-state of the location at the given index.
// Independent locations do not belong to the track of the user.
// Returns 1 if location belongs to the track of the user,
// 0 if location was reported independently.
#[no_mangle]
pub unsafe fn dc_array_is_independent(
    array: *const dc_array_t,
    index: libc::size_t,
) -> libc::c_int {
    if array.is_null() {
        eprintln!("ignoring careless call to dc_array_is_independent()");
        return 0;
    }

    (*array).get_location(index).independent as libc::c_int
}

// dc_chatlist_t

/// FFI struct for [dc_chatlist_t]
///
/// This is the structure behind [dc_chatlist_t] which is the opaque
/// structure representing a chatlist in the FFI API.  It exists
/// because the FFI API has a refernce from the message to the
/// context, but the Rust API does not, so the FFI layer needs to glue
/// these together.
pub struct ChatlistWrapper {
    context: *const dc_context_t,
    list: chatlist::Chatlist,
}

#[no_mangle]
pub type dc_chatlist_t = ChatlistWrapper;

#[no_mangle]
pub unsafe extern "C" fn dc_chatlist_unref(chatlist: *mut dc_chatlist_t) {
    if chatlist.is_null() {
        eprintln!("ignoring careless call to dc_chatlist_unref()");
        return;
    }
    Box::from_raw(chatlist);
}

#[no_mangle]
pub unsafe extern "C" fn dc_chatlist_get_cnt(chatlist: *mut dc_chatlist_t) -> libc::size_t {
    if chatlist.is_null() {
        eprintln!("ignoring careless call to dc_chatlist_get_cnt()");
        return 0;
    }
    let ffi_list = &*chatlist;
    ffi_list.list.len() as libc::size_t
}

#[no_mangle]
pub unsafe extern "C" fn dc_chatlist_get_chat_id(
    chatlist: *mut dc_chatlist_t,
    index: libc::size_t,
) -> u32 {
    if chatlist.is_null() {
        eprintln!("ignoring careless call to dc_chatlist_get_chat_id()");
        return 0;
    }
    let ffi_list = &*chatlist;
    ffi_list.list.get_chat_id(index as usize).to_u32()
}

#[no_mangle]
pub unsafe extern "C" fn dc_chatlist_get_msg_id(
    chatlist: *mut dc_chatlist_t,
    index: libc::size_t,
) -> u32 {
    if chatlist.is_null() {
        eprintln!("ignoring careless call to dc_chatlist_get_msg_id()");
        return 0;
    }
    let ffi_list = &*chatlist;
    ffi_list
        .list
        .get_msg_id(index as usize)
        .map(|msg_id| msg_id.to_u32())
        .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn dc_chatlist_get_summary(
    chatlist: *mut dc_chatlist_t,
    index: libc::size_t,
    chat: *mut dc_chat_t,
) -> *mut dc_lot_t {
    if chatlist.is_null() {
        eprintln!("ignoring careless call to dc_chatlist_get_summary()");
        return ptr::null_mut();
    }
    let maybe_chat = if chat.is_null() {
        None
    } else {
        let ffi_chat = &*chat;
        Some(&ffi_chat.chat)
    };
    let ffi_list = &*chatlist;
    let ffi_context: &ContextWrapper = &*ffi_list.context;
    ffi_context
        .with_inner(|ctx| {
            let lot = ffi_list.list.get_summary(ctx, index as usize, maybe_chat);
            Box::into_raw(Box::new(lot))
        })
        .unwrap_or_else(|_| ptr::null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn dc_chatlist_get_context(
    chatlist: *mut dc_chatlist_t,
) -> *const dc_context_t {
    if chatlist.is_null() {
        eprintln!("ignoring careless call to dc_chatlist_get_context()");
        return ptr::null_mut();
    }
    let ffi_list = &*chatlist;
    ffi_list.context
}

// dc_chat_t

/// FFI struct for [dc_chat_t]
///
/// This is the structure behind [dc_chat_t] which is the opaque
/// structure representing a chat in the FFI API.  It exists
/// because the FFI API has a refernce from the message to the
/// context, but the Rust API does not, so the FFI layer needs to glue
/// these together.
pub struct ChatWrapper {
    context: *const dc_context_t,
    chat: chat::Chat,
}

#[no_mangle]
pub type dc_chat_t = ChatWrapper;

#[no_mangle]
pub unsafe extern "C" fn dc_chat_unref(chat: *mut dc_chat_t) {
    if chat.is_null() {
        eprintln!("ignoring careless call to dc_chat_unref()");
        return;
    }

    Box::from_raw(chat);
}

#[no_mangle]
pub unsafe extern "C" fn dc_chat_get_id(chat: *mut dc_chat_t) -> u32 {
    if chat.is_null() {
        eprintln!("ignoring careless call to dc_chat_get_id()");
        return 0;
    }
    let ffi_chat = &*chat;
    ffi_chat.chat.get_id().to_u32()
}

#[no_mangle]
pub unsafe extern "C" fn dc_chat_get_type(chat: *mut dc_chat_t) -> libc::c_int {
    if chat.is_null() {
        eprintln!("ignoring careless call to dc_chat_get_type()");
        return 0;
    }
    let ffi_chat = &*chat;
    ffi_chat.chat.get_type() as libc::c_int
}

#[no_mangle]
pub unsafe extern "C" fn dc_chat_get_name(chat: *mut dc_chat_t) -> *mut libc::c_char {
    if chat.is_null() {
        eprintln!("ignoring careless call to dc_chat_get_name()");
        return "".strdup();
    }
    let ffi_chat = &*chat;
    ffi_chat.chat.get_name().strdup()
}

#[no_mangle]
pub unsafe extern "C" fn dc_chat_get_profile_image(chat: *mut dc_chat_t) -> *mut libc::c_char {
    if chat.is_null() {
        eprintln!("ignoring careless call to dc_chat_get_profile_image()");
        return ptr::null_mut(); // NULL explicitly defined as "no image"
    }
    let ffi_chat = &*chat;
    let ffi_context = &*ffi_chat.context;
    ffi_context
        .with_inner(|ctx| match ffi_chat.chat.get_profile_image(ctx) {
            Some(p) => p.to_string_lossy().strdup(),
            None => ptr::null_mut(),
        })
        .unwrap_or_else(|_| ptr::null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn dc_chat_get_color(chat: *mut dc_chat_t) -> u32 {
    if chat.is_null() {
        eprintln!("ignoring careless call to dc_chat_get_color()");
        return 0;
    }
    let ffi_chat = &*chat;
    let ffi_context = &*ffi_chat.context;
    ffi_context
        .with_inner(|ctx| ffi_chat.chat.get_color(ctx))
        .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn dc_chat_get_visibility(chat: *mut dc_chat_t) -> libc::c_int {
    if chat.is_null() {
        eprintln!("ignoring careless call to dc_chat_get_visibility()");
        return 0;
    }
    let ffi_chat = &*chat;
    match ffi_chat.chat.visibility {
        ChatVisibility::Normal => 0,
        ChatVisibility::Archived => 1,
        ChatVisibility::Pinned => 2,
    }
}

#[no_mangle]
pub unsafe extern "C" fn dc_chat_is_unpromoted(chat: *mut dc_chat_t) -> libc::c_int {
    if chat.is_null() {
        eprintln!("ignoring careless call to dc_chat_is_unpromoted()");
        return 0;
    }
    let ffi_chat = &*chat;
    ffi_chat.chat.is_unpromoted() as libc::c_int
}

#[no_mangle]
pub unsafe extern "C" fn dc_chat_is_self_talk(chat: *mut dc_chat_t) -> libc::c_int {
    if chat.is_null() {
        eprintln!("ignoring careless call to dc_chat_is_self_talk()");
        return 0;
    }
    let ffi_chat = &*chat;
    ffi_chat.chat.is_self_talk() as libc::c_int
}

#[no_mangle]
pub unsafe extern "C" fn dc_chat_is_device_talk(chat: *mut dc_chat_t) -> libc::c_int {
    if chat.is_null() {
        eprintln!("ignoring careless call to dc_chat_is_device_talk()");
        return 0;
    }
    let ffi_chat = &*chat;
    ffi_chat.chat.is_device_talk() as libc::c_int
}

#[no_mangle]
pub unsafe extern "C" fn dc_chat_can_send(chat: *mut dc_chat_t) -> libc::c_int {
    if chat.is_null() {
        eprintln!("ignoring careless call to dc_chat_can_send()");
        return 0;
    }
    let ffi_chat = &*chat;
    ffi_chat.chat.can_send() as libc::c_int
}

#[no_mangle]
pub unsafe extern "C" fn dc_chat_is_verified(chat: *mut dc_chat_t) -> libc::c_int {
    if chat.is_null() {
        eprintln!("ignoring careless call to dc_chat_is_verified()");
        return 0;
    }
    let ffi_chat = &*chat;
    ffi_chat.chat.is_verified() as libc::c_int
}

#[no_mangle]
pub unsafe extern "C" fn dc_chat_is_sending_locations(chat: *mut dc_chat_t) -> libc::c_int {
    if chat.is_null() {
        eprintln!("ignoring careless call to dc_chat_is_sending_locations()");
        return 0;
    }
    let ffi_chat = &*chat;
    ffi_chat.chat.is_sending_locations() as libc::c_int
}

#[no_mangle]
pub unsafe extern "C" fn dc_chat_is_muted(chat: *mut dc_chat_t) -> libc::c_int {
    if chat.is_null() {
        eprintln!("ignoring careless call to dc_chat_is_muted()");
        return 0;
    }
    let ffi_chat = &*chat;
    ffi_chat.chat.is_muted() as libc::c_int
}

#[no_mangle]
pub unsafe extern "C" fn dc_chat_get_remaining_mute_duration(chat: *mut dc_chat_t) -> i64 {
    if chat.is_null() {
        eprintln!("ignoring careless call to dc_chat_get_remaining_mute_duration()");
        return 0;
    }
    let ffi_chat = &*chat;
    if !ffi_chat.chat.is_muted() {
        return 0;
    }
    // If the chat was muted to before the epoch, it is not muted.
    match ffi_chat.chat.mute_duration {
        MuteDuration::NotMuted => 0,
        MuteDuration::Forever => -1,
        MuteDuration::Until(when) => when
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0),
    }
}

#[no_mangle]
pub unsafe extern "C" fn dc_chat_get_info_json(
    context: *mut dc_context_t,
    chat_id: u32,
) -> *mut libc::c_char {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_chat_get_info_json()");
        return "".strdup();
    }
    let ffi_context = &*context;
    ffi_context
        .with_inner(|ctx| {
            let chat = match chat::Chat::load_from_db(ctx, ChatId::new(chat_id)) {
                Ok(chat) => chat,
                Err(err) => {
                    error!(ctx, "dc_get_chat_info_json() failed to load chat: {}", err);
                    return "".strdup();
                }
            };
            let info = match chat.get_info(ctx) {
                Ok(info) => info,
                Err(err) => {
                    error!(
                        ctx,
                        "dc_get_chat_info_json() failed to get chat info: {}", err
                    );
                    return "".strdup();
                }
            };
            serde_json::to_string(&info)
                .unwrap_or_log_default(ctx, "dc_get_chat_info_json() failed to serialise to json")
                .strdup()
        })
        .unwrap_or_else(|_| "".strdup())
}

// dc_msg_t

/// FFI struct for [dc_msg_t]
///
/// This is the structure behind [dc_msg_t] which is the opaque
/// structure representing a message in the FFI API.  It exists
/// because the FFI API has a refernce from the message to the
/// context, but the Rust API does not, so the FFI layer needs to glue
/// these together.
pub struct MessageWrapper {
    context: *const dc_context_t,
    message: message::Message,
}

#[no_mangle]
pub type dc_msg_t = MessageWrapper;

#[no_mangle]
pub unsafe extern "C" fn dc_msg_new(
    context: *mut dc_context_t,
    viewtype: libc::c_int,
) -> *mut dc_msg_t {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_msg_new()");
        return ptr::null_mut();
    }
    let context = &*context;
    let viewtype = from_prim(viewtype).expect(&format!("invalid viewtype = {}", viewtype));
    let msg = MessageWrapper {
        context,
        message: message::Message::new(viewtype),
    };
    Box::into_raw(Box::new(msg))
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_unref(msg: *mut dc_msg_t) {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_unref()");
        return;
    }

    Box::from_raw(msg);
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_id(msg: *mut dc_msg_t) -> u32 {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_get_id()");
        return 0;
    }
    let ffi_msg = &*msg;
    ffi_msg.message.get_id().to_u32()
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_from_id(msg: *mut dc_msg_t) -> u32 {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_get_from_id()");
        return 0;
    }
    let ffi_msg = &*msg;
    ffi_msg.message.get_from_id()
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_chat_id(msg: *mut dc_msg_t) -> u32 {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_get_chat_id()");
        return 0;
    }
    let ffi_msg = &*msg;
    ffi_msg.message.get_chat_id().to_u32()
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_viewtype(msg: *mut dc_msg_t) -> libc::c_int {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_get_viewtype()");
        return 0;
    }
    let ffi_msg = &*msg;
    ffi_msg
        .message
        .get_viewtype()
        .to_i64()
        .expect("impossible: Viewtype -> i64 conversion failed") as libc::c_int
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_state(msg: *mut dc_msg_t) -> libc::c_int {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_get_state()");
        return 0;
    }
    let ffi_msg = &*msg;
    ffi_msg.message.get_state() as libc::c_int
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_timestamp(msg: *mut dc_msg_t) -> i64 {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_get_received_timestamp()");
        return 0;
    }
    let ffi_msg = &*msg;
    ffi_msg.message.get_timestamp()
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_received_timestamp(msg: *mut dc_msg_t) -> i64 {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_get_received_timestamp()");
        return 0;
    }
    let ffi_msg = &*msg;
    ffi_msg.message.get_received_timestamp()
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_sort_timestamp(msg: *mut dc_msg_t) -> i64 {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_get_sort_timestamp()");
        return 0;
    }
    let ffi_msg = &*msg;
    ffi_msg.message.get_sort_timestamp()
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_text(msg: *mut dc_msg_t) -> *mut libc::c_char {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_get_text()");
        return "".strdup();
    }
    let ffi_msg = &*msg;
    ffi_msg.message.get_text().unwrap_or_default().strdup()
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_file(msg: *mut dc_msg_t) -> *mut libc::c_char {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_get_file()");
        return "".strdup();
    }
    let ffi_msg = &*msg;
    let ffi_context = &*ffi_msg.context;
    ffi_context
        .with_inner(|ctx| {
            ffi_msg
                .message
                .get_file(ctx)
                .map(|p| p.strdup())
                .unwrap_or_else(|| "".strdup())
        })
        .unwrap_or_else(|_| "".strdup())
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_filename(msg: *mut dc_msg_t) -> *mut libc::c_char {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_get_filename()");
        return "".strdup();
    }
    let ffi_msg = &*msg;
    ffi_msg.message.get_filename().unwrap_or_default().strdup()
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_filemime(msg: *mut dc_msg_t) -> *mut libc::c_char {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_get_filemime()");
        return "".strdup();
    }
    let ffi_msg = &*msg;
    if let Some(x) = ffi_msg.message.get_filemime() {
        x.strdup()
    } else {
        "".strdup()
    }
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_filebytes(msg: *mut dc_msg_t) -> u64 {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_get_filebytes()");
        return 0;
    }
    let ffi_msg = &*msg;
    let ffi_context = &*ffi_msg.context;
    ffi_context
        .with_inner(|ctx| ffi_msg.message.get_filebytes(ctx))
        .unwrap_or(0)
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_width(msg: *mut dc_msg_t) -> libc::c_int {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_get_width()");
        return 0;
    }
    let ffi_msg = &*msg;
    ffi_msg.message.get_width()
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_height(msg: *mut dc_msg_t) -> libc::c_int {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_get_height()");
        return 0;
    }
    let ffi_msg = &*msg;
    ffi_msg.message.get_height()
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_duration(msg: *mut dc_msg_t) -> libc::c_int {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_get_duration()");
        return 0;
    }
    let ffi_msg = &*msg;
    ffi_msg.message.get_duration()
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_showpadlock(msg: *mut dc_msg_t) -> libc::c_int {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_get_showpadlock()");
        return 0;
    }
    let ffi_msg = &*msg;
    ffi_msg.message.get_showpadlock() as libc::c_int
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_summary(
    msg: *mut dc_msg_t,
    chat: *mut dc_chat_t,
) -> *mut dc_lot_t {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_get_summary()");
        return ptr::null_mut();
    }
    let maybe_chat = if chat.is_null() {
        None
    } else {
        let ffi_chat = &*chat;
        Some(&ffi_chat.chat)
    };
    let ffi_msg = &mut *msg;
    let ffi_context = &*ffi_msg.context;
    ffi_context
        .with_inner(|ctx| {
            let lot = ffi_msg.message.get_summary(ctx, maybe_chat);
            Box::into_raw(Box::new(lot))
        })
        .unwrap_or_else(|_| ptr::null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_summarytext(
    msg: *mut dc_msg_t,
    approx_characters: libc::c_int,
) -> *mut libc::c_char {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_get_summarytext()");
        return "".strdup();
    }
    let ffi_msg = &mut *msg;
    let ffi_context = &*ffi_msg.context;
    ffi_context
        .with_inner(|ctx| {
            ffi_msg
                .message
                .get_summarytext(ctx, approx_characters.try_into().unwrap_or_default())
        })
        .unwrap_or_default()
        .strdup()
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_has_deviating_timestamp(msg: *mut dc_msg_t) -> libc::c_int {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_has_deviating_timestamp()");
        return 0;
    }
    let ffi_msg = &*msg;
    ffi_msg.message.has_deviating_timestamp().into()
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_has_location(msg: *mut dc_msg_t) -> libc::c_int {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_has_location()");
        return 0;
    }
    let ffi_msg = &*msg;
    ffi_msg.message.has_location() as libc::c_int
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_is_sent(msg: *mut dc_msg_t) -> libc::c_int {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_is_sent()");
        return 0;
    }
    let ffi_msg = &*msg;
    ffi_msg.message.is_sent().into()
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_is_starred(msg: *mut dc_msg_t) -> libc::c_int {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_is_starred()");
        return 0;
    }
    let ffi_msg = &*msg;
    ffi_msg.message.is_starred().into()
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_is_forwarded(msg: *mut dc_msg_t) -> libc::c_int {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_is_forwarded()");
        return 0;
    }
    let ffi_msg = &*msg;
    ffi_msg.message.is_forwarded().into()
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_is_info(msg: *mut dc_msg_t) -> libc::c_int {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_is_info()");
        return 0;
    }
    let ffi_msg = &*msg;
    ffi_msg.message.is_info().into()
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_is_increation(msg: *mut dc_msg_t) -> libc::c_int {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_is_increation()");
        return 0;
    }
    let ffi_msg = &*msg;
    ffi_msg.message.is_increation().into()
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_is_setupmessage(msg: *mut dc_msg_t) -> libc::c_int {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_is_setupmessage()");
        return 0;
    }
    let ffi_msg = &*msg;
    ffi_msg.message.is_setupmessage().into()
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_setupcodebegin(msg: *mut dc_msg_t) -> *mut libc::c_char {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_get_setupcodebegin()");
        return "".strdup();
    }
    let ffi_msg = &*msg;
    let ffi_context = &*ffi_msg.context;
    ffi_context
        .with_inner(|ctx| ffi_msg.message.get_setupcodebegin(ctx).unwrap_or_default())
        .unwrap_or_default()
        .strdup()
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_set_text(msg: *mut dc_msg_t, text: *const libc::c_char) {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_set_text()");
        return;
    }
    let ffi_msg = &mut *msg;
    ffi_msg.message.set_text(to_opt_string_lossy(text))
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_set_file(
    msg: *mut dc_msg_t,
    file: *const libc::c_char,
    filemime: *const libc::c_char,
) {
    if msg.is_null() || file.is_null() {
        eprintln!("ignoring careless call to dc_msg_set_file()");
        return;
    }
    let ffi_msg = &mut *msg;
    ffi_msg.message.set_file(
        to_string_lossy(file),
        to_opt_string_lossy(filemime).as_ref().map(|x| x.as_str()),
    )
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_set_dimension(
    msg: *mut dc_msg_t,
    width: libc::c_int,
    height: libc::c_int,
) {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_set_dimension()");
        return;
    }
    let ffi_msg = &mut *msg;
    ffi_msg.message.set_dimension(width, height)
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_set_duration(msg: *mut dc_msg_t, duration: libc::c_int) {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_set_duration()");
        return;
    }
    let ffi_msg = &mut *msg;
    ffi_msg.message.set_duration(duration)
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_set_location(
    msg: *mut dc_msg_t,
    latitude: libc::c_double,
    longitude: libc::c_double,
) {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_set_location()");
        return;
    }
    let ffi_msg = &mut *msg;
    ffi_msg.message.set_location(latitude, longitude)
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_latefiling_mediasize(
    msg: *mut dc_msg_t,
    width: libc::c_int,
    height: libc::c_int,
    duration: libc::c_int,
) {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_latefiling_mediasize()");
        return;
    }
    let ffi_msg = &mut *msg;
    let ffi_context = &*ffi_msg.context;
    ffi_context
        .with_inner(|ctx| {
            ffi_msg
                .message
                .latefiling_mediasize(ctx, width, height, duration)
        })
        .ok();
}

// dc_contact_t

/// FFI struct for [dc_contact_t]
///
/// This is the structure behind [dc_contact_t] which is the opaque
/// structure representing a contact in the FFI API.  It exists
/// because the FFI API has a refernce from the message to the
/// context, but the Rust API does not, so the FFI layer needs to glue
/// these together.
pub struct ContactWrapper {
    context: *const dc_context_t,
    contact: contact::Contact,
}

#[no_mangle]
pub type dc_contact_t = ContactWrapper;

#[no_mangle]
pub unsafe extern "C" fn dc_contact_unref(contact: *mut dc_contact_t) {
    if contact.is_null() {
        eprintln!("ignoring careless call to dc_contact_unref()");
        return;
    }
    Box::from_raw(contact);
}

#[no_mangle]
pub unsafe extern "C" fn dc_contact_get_id(contact: *mut dc_contact_t) -> u32 {
    if contact.is_null() {
        eprintln!("ignoring careless call to dc_contact_get_id()");
        return 0;
    }
    let ffi_contact = &*contact;
    ffi_contact.contact.get_id()
}

#[no_mangle]
pub unsafe extern "C" fn dc_contact_get_addr(contact: *mut dc_contact_t) -> *mut libc::c_char {
    if contact.is_null() {
        eprintln!("ignoring careless call to dc_contact_get_addr()");
        return "".strdup();
    }
    let ffi_contact = &*contact;
    ffi_contact.contact.get_addr().strdup()
}

#[no_mangle]
pub unsafe extern "C" fn dc_contact_get_name(contact: *mut dc_contact_t) -> *mut libc::c_char {
    if contact.is_null() {
        eprintln!("ignoring careless call to dc_contact_get_name()");
        return "".strdup();
    }
    let ffi_contact = &*contact;
    ffi_contact.contact.get_name().strdup()
}

#[no_mangle]
pub unsafe extern "C" fn dc_contact_get_display_name(
    contact: *mut dc_contact_t,
) -> *mut libc::c_char {
    if contact.is_null() {
        eprintln!("ignoring careless call to dc_contact_get_display_name()");
        return "".strdup();
    }
    let ffi_contact = &*contact;
    ffi_contact.contact.get_display_name().strdup()
}

#[no_mangle]
pub unsafe extern "C" fn dc_contact_get_name_n_addr(
    contact: *mut dc_contact_t,
) -> *mut libc::c_char {
    if contact.is_null() {
        eprintln!("ignoring careless call to dc_contact_get_name_n_addr()");
        return "".strdup();
    }
    let ffi_contact = &*contact;
    ffi_contact.contact.get_name_n_addr().strdup()
}

#[no_mangle]
pub unsafe extern "C" fn dc_contact_get_first_name(
    contact: *mut dc_contact_t,
) -> *mut libc::c_char {
    if contact.is_null() {
        eprintln!("ignoring careless call to dc_contact_get_first_name()");
        return "".strdup();
    }
    let ffi_contact = &*contact;
    ffi_contact.contact.get_first_name().strdup()
}

#[no_mangle]
pub unsafe extern "C" fn dc_contact_get_profile_image(
    contact: *mut dc_contact_t,
) -> *mut libc::c_char {
    if contact.is_null() {
        eprintln!("ignoring careless call to dc_contact_get_profile_image()");
        return ptr::null_mut(); // NULL explicitly defined as "no profile image"
    }
    let ffi_contact = &*contact;
    let ffi_context = &*ffi_contact.context;
    ffi_context
        .with_inner(|ctx| {
            ffi_contact
                .contact
                .get_profile_image(ctx)
                .map(|p| p.to_string_lossy().strdup())
                .unwrap_or_else(std::ptr::null_mut)
        })
        .unwrap_or_else(|_| ptr::null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn dc_contact_get_color(contact: *mut dc_contact_t) -> u32 {
    if contact.is_null() {
        eprintln!("ignoring careless call to dc_contact_get_color()");
        return 0;
    }
    let ffi_contact = &*contact;
    ffi_contact.contact.get_color()
}

#[no_mangle]
pub unsafe extern "C" fn dc_contact_is_blocked(contact: *mut dc_contact_t) -> libc::c_int {
    if contact.is_null() {
        eprintln!("ignoring careless call to dc_contact_is_blocked()");
        return 0;
    }
    let ffi_contact = &*contact;
    ffi_contact.contact.is_blocked() as libc::c_int
}

#[no_mangle]
pub unsafe extern "C" fn dc_contact_is_verified(contact: *mut dc_contact_t) -> libc::c_int {
    if contact.is_null() {
        eprintln!("ignoring careless call to dc_contact_is_verified()");
        return 0;
    }
    let ffi_contact = &*contact;
    let ffi_context = &*ffi_contact.context;
    ffi_context
        .with_inner(|ctx| ffi_contact.contact.is_verified(ctx) as libc::c_int)
        .unwrap_or(0)
}

// dc_lot_t

#[no_mangle]
pub type dc_lot_t = lot::Lot;

#[no_mangle]
pub unsafe extern "C" fn dc_lot_unref(lot: *mut dc_lot_t) {
    if lot.is_null() {
        eprintln!("ignoring careless call to dc_lot_unref()");
        return;
    }

    Box::from_raw(lot);
}

#[no_mangle]
pub unsafe extern "C" fn dc_lot_get_text1(lot: *mut dc_lot_t) -> *mut libc::c_char {
    if lot.is_null() {
        eprintln!("ignoring careless call to dc_lot_get_text1()");
        return ptr::null_mut(); // NULL explicitly defined as "there is no such text"
    }

    let lot = &*lot;
    lot.get_text1().strdup()
}

#[no_mangle]
pub unsafe extern "C" fn dc_lot_get_text2(lot: *mut dc_lot_t) -> *mut libc::c_char {
    if lot.is_null() {
        eprintln!("ignoring careless call to dc_lot_get_text2()");
        return ptr::null_mut(); // NULL explicitly defined as "there is no such text"
    }

    let lot = &*lot;
    lot.get_text2().strdup()
}

#[no_mangle]
pub unsafe extern "C" fn dc_lot_get_text1_meaning(lot: *mut dc_lot_t) -> libc::c_int {
    if lot.is_null() {
        eprintln!("ignoring careless call to dc_lot_get_text1_meaning()");
        return 0;
    }

    let lot = &*lot;
    lot.get_text1_meaning() as libc::c_int
}

#[no_mangle]
pub unsafe extern "C" fn dc_lot_get_state(lot: *mut dc_lot_t) -> libc::c_int {
    if lot.is_null() {
        eprintln!("ignoring careless call to dc_lot_get_state()");
        return 0;
    }

    let lot = &*lot;
    lot.get_state().to_i64().expect("impossible") as libc::c_int
}

#[no_mangle]
pub unsafe extern "C" fn dc_lot_get_id(lot: *mut dc_lot_t) -> u32 {
    if lot.is_null() {
        eprintln!("ignoring careless call to dc_lot_get_id()");
        return 0;
    }

    let lot = &*lot;
    lot.get_id()
}

#[no_mangle]
pub unsafe extern "C" fn dc_lot_get_timestamp(lot: *mut dc_lot_t) -> i64 {
    if lot.is_null() {
        eprintln!("ignoring careless call to dc_lot_get_timestamp()");
        return 0;
    }

    let lot = &*lot;
    lot.get_timestamp()
}

#[no_mangle]
pub unsafe extern "C" fn dc_str_unref(s: *mut libc::c_char) {
    libc::free(s as *mut _)
}

trait ResultExt<T, E> {
    fn unwrap_or_log_default(self, context: &context::Context, message: &str) -> T;

    /// Log a warning to a [ContextWrapper] for an [Err] result.
    ///
    /// Does nothing for an [Ok].
    ///
    /// You can do this as soon as the wrapper exists, it does not
    /// have to be open (which is required for the `warn!()` macro).
    fn log_err(self, wrapper: &ContextWrapper, message: &str) -> Result<T, E>;
}

impl<T: Default, E: std::fmt::Display> ResultExt<T, E> for Result<T, E> {
    fn unwrap_or_log_default(self, context: &context::Context, message: &str) -> T {
        match self {
            Ok(t) => t,
            Err(err) => {
                error!(context, "{}: {}", message, err);
                Default::default()
            }
        }
    }

    fn log_err(self, wrapper: &ContextWrapper, message: &str) -> Result<T, E> {
        self.map_err(|err| {
            unsafe {
                wrapper.warning(&format!("{}: {}", message, err));
            }
            err
        })
    }
}

trait ResultNullableExt<T> {
    fn into_raw(self) -> *mut T;
}

impl<T, E> ResultNullableExt<T> for Result<T, E> {
    fn into_raw(self) -> *mut T {
        match self {
            Ok(t) => Box::into_raw(Box::new(t)),
            Err(_) => ptr::null_mut(),
        }
    }
}

fn convert_and_prune_message_ids(msg_ids: *const u32, msg_cnt: libc::c_int) -> Vec<MsgId> {
    let ids = unsafe { std::slice::from_raw_parts(msg_ids, msg_cnt as usize) };
    let msg_ids: Vec<MsgId> = ids
        .iter()
        .filter(|id| **id > DC_MSG_ID_LAST_SPECIAL)
        .map(|id| MsgId::new(*id))
        .collect();

    msg_ids
}

// dc_provider_t

#[no_mangle]
pub type dc_provider_t = provider::Provider;

#[no_mangle]
pub unsafe extern "C" fn dc_provider_new_from_email(
    context: *const dc_context_t,
    addr: *const libc::c_char,
) -> *const dc_provider_t {
    if context.is_null() || addr.is_null() {
        eprintln!("ignoring careless call to dc_provider_new_from_email()");
        return ptr::null();
    }
    let addr = to_string_lossy(addr);
    match provider::get_provider_info(addr.as_str()) {
        Some(provider) => provider,
        None => ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn dc_provider_get_overview_page(
    provider: *const dc_provider_t,
) -> *mut libc::c_char {
    if provider.is_null() {
        eprintln!("ignoring careless call to dc_provider_get_overview_page()");
        return "".strdup();
    }
    let provider = &*provider;
    provider.overview_page.strdup()
}

#[no_mangle]
pub unsafe extern "C" fn dc_provider_get_before_login_hint(
    provider: *const dc_provider_t,
) -> *mut libc::c_char {
    if provider.is_null() {
        eprintln!("ignoring careless call to dc_provider_get_before_login_hint()");
        return "".strdup();
    }
    let provider = &*provider;
    provider.before_login_hint.strdup()
}

#[no_mangle]
pub unsafe extern "C" fn dc_provider_get_status(provider: *const dc_provider_t) -> libc::c_int {
    if provider.is_null() {
        eprintln!("ignoring careless call to dc_provider_get_status()");
        return 0;
    }
    let provider = &*provider;
    provider.status as libc::c_int
}

#[no_mangle]
pub unsafe extern "C" fn dc_provider_unref(provider: *mut dc_provider_t) {
    if provider.is_null() {
        eprintln!("ignoring careless call to dc_provider_unref()");
        return;
    }
    // currently, there is nothing to free, the provider info is a static object.
    // this may change once we start localizing string.
}
