#![allow(
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    non_upper_case_globals,
    non_camel_case_types,
    non_snake_case
)]

#[macro_use]
extern crate human_panic;
extern crate num_traits;
#[macro_use]
extern crate rental;

use std::convert::TryInto;
use std::ffi::CString;
use std::ptr;
use std::str::FromStr;
use std::sync::RwLock;

use libc::uintptr_t;
use num_traits::{FromPrimitive, ToPrimitive};

use deltachat::config::Config;
use deltachat::constants::Event;
use deltachat::contact::Contact;
use deltachat::context::Context;
use deltachat::dc_tools::{as_path, as_str, to_string, Strdup};
use deltachat::*;

// as C lacks a good and portable error handling,
// in general, the C Interface is forgiving wrt to bad parameters.
// - objects returned by some functions
//   should be passable to the functions handling that object.
// - if in doubt, the empty string is returned on failures;
//   this avoids panics if the ui just forgets to handle a case
// - finally, this behaviour matches the old core-c API and UIs already depend on it

// TODO: constants

/// The FFI callback type that should be passed to [dc_context_new].
///
/// @memberof Context
///
/// # Parameters
///
/// The callback should accept the following arguments:
///
/// * `context` - The context object as returned by [dc_context_new].
/// * `event` - one of the @ref DC_EVENT constants.
/// * `data1` - depends on the event parameter.
/// * `data2` - depends on the event parameter.
///
/// # Return value
///
/// This callback should return 0 unless stated otherwise in the event
/// parameter documentation.
pub type dc_callback_t =
    unsafe extern "C" fn(_: &ContextWrapper, _: Event, _: uintptr_t, _: uintptr_t) -> uintptr_t;

/// The FFI context struct.
///
/// This structure represents the [Context] on the FFI interface.
/// Since it is returned by [dc_context_new] before it is initialised
/// by [dc_context_open] it needs to store the actual [Context] in an
/// [Option] and protected by an [RwLock].  Other than that it needs
/// to store the data which is passed into [dc_context_new].
pub struct ContextWrapper {
    cb: Option<dc_callback_t>,
    userdata: *mut libc::c_void,
    os_name: String,
    inner: RwLock<Option<context::Context>>,
}

/// The FFI context type.
pub type dc_context_t = ContextWrapper;

// A few wrappers for structs which keep a reference to the context.

rental! {
    /// FFI wrappers to keep a reference to the context.
    ///
    /// A few returned objects hold a reference to the [Context],
    /// which itself is protected by an [RwLock] inside the
    /// [ContextWrapper] used on the FFI layer.  This means the
    /// objects returned by the FFI layer also need to return the lock
    /// used to keep the [Context] reference alive.
    ///
    /// These structs each contain a reference to the lock guard and
    /// the object being kept alive by the lock.  Because the latter
    /// needs to have the lifetime of the former they are
    /// self-referential structs and need to be created using the
    /// rental crate.
    pub mod rentals {
        use super::*;

        /// FFI wrapper around [chatlist::Chatlist].
        #[rental]
        pub struct ChatlistWrapper<'a> {
            guard: std::sync::RwLockReadGuard<'a, Option<Context>>,
            list: chatlist::Chatlist<'guard>,
        }

        /// FFI wrapper around [message::Message].
        #[rental]
        pub struct MessageWrapper<'a> {
            guard: std::sync::RwLockReadGuard<'a, Option<Context>>,
            msg: message::Message<'guard>,
        }

        /// FFI wrapper around [chat::Chat].
        #[rental]
        pub struct ChatWrapper<'a> {
            guard: std::sync::RwLockReadGuard<'a, Option<Context>>,
            chat: chat::Chat<'guard>,
        }

        /// FFI wrapper around [contact::Contact].
        #[rental]
        pub struct ContactWrapper<'a> {
            guard: std::sync::RwLockReadGuard<'a, Option<Context>>,
            contact: contact::Contact<'guard>,
        }
    }
}

pub use rentals::*;

#[no_mangle]
pub type dc_msg_t<'a> = MessageWrapper<'a>;

#[no_mangle]
pub type dc_chatlist_t<'a> = ChatlistWrapper<'a>;

#[no_mangle]
pub type dc_chat_t<'a> = ChatWrapper<'a>;

#[no_mangle]
pub type dc_contact_t<'a> = ContactWrapper<'a>;

impl ContextWrapper {
    /// Log an error on the FFI context.
    ///
    /// As soon as a [ContextWrapper] exist it can be used to log an
    /// error using the callback, even before [dc_context_open] is
    /// called and an actual [Context] exists.
    ///
    /// This function makes it easy to log an error.
    fn error(&self, msg: &str) {
        if let Some(cb) = self.cb {
            let msg_c =
                CString::new(msg).unwrap_or(CString::new("[invalid error message]").unwrap());
            unsafe { cb(self, Event::ERROR, 0, msg_c.as_ptr() as libc::uintptr_t) };
        }
    }
}

// dc_context_t implementations

/// Create a new context object.
///
/// After creation it is usually opened, connected and mails are
/// fetched.
///
/// @memberof [dc_context_t]
///
/// # Parameters
///
/// * `cb` - a callback function that is called for events (update,
///   state changes etc.) and to get some information from the client
///   (eg. translation for a given string).
///
///   See @ref DC_EVENT for a list of possible events that may be passed to the callback.
///
///     - The callback MAY be called from _any_ thread, not only the
///       main/GUI thread!
///     - The callback MUST NOT call any dc_* and related functions
///       unless stated otherwise!
///     - The callback SHOULD return _fast_, for GUI updates etc. you
///       should post yourself an asynchronous message to your GUI
///       thread, if needed.
///     - If not mentioned otherweise, the callback should return 0.
///
///   This must have the same lifetime as the context otherwise events
///   will call garbage.
///
/// * `userdata` - can be used by the client for any purpuse.  Can be
///   retrieved using [dc_get_userdata].  This is never freed by
///   deltachat itself, even after the context is destroyed.  It is
///   assumed this has the same lifetime as the context itself,
///   otherwise [dc_get_userdata] will return a pointer to freed
///   memory.
///
/// * `os_name` - is only for decorative use and is shown eg. in the
///   `X-Mailer:` header in the form "Delta Chat Core
///   <version>/<os_name>".  You can give the name of the app, the
///   operating system, the used environment and/or the version here.
///   It is okay to give NULL, in this case `X-Mailer:` header is set
///   to "Delta Chat Core <version>".
///
///   This is never freed by deltachat itself.  It can be destroyed as
///   soon as this function returns.
///
/// # Returns
///
/// A context object with some public members.  The object must be
/// passed to the other context functions and must be freed using
/// [dc_context_unref] after usage.
#[no_mangle]
pub unsafe extern "C" fn dc_context_new(
    cb: Option<dc_callback_t>,
    userdata: *mut libc::c_void,
    os_name: *const libc::c_char,
) -> *mut dc_context_t {
    setup_panic!();
    let wrapper = ContextWrapper {
        cb,
        userdata,
        os_name: to_string(os_name),
        inner: RwLock::new(None),
    };
    Box::into_raw(Box::new(wrapper))
}

/// Free a context object.
///
/// If app runs can only be terminated by a forced kill, this may be superfluous.
/// Before the context object is freed, connections to SMTP, IMAP and database
/// are closed. You can also do this explicitly by calling [dc_close] on your own
/// before calling [dc_context_unref].
///
/// @memberof dc_context_t
///
/// # Parameters
///
/// * `context` - The context object as created by [dc_context_new].
///   If `NULL` is given, nothing is done.
#[no_mangle]
pub unsafe extern "C" fn dc_context_unref(context: *mut dc_context_t) {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_context_unref()");
        return;
    }

    let wrapper: &mut ContextWrapper = &mut *context;
    Box::from_raw(context); // Drops the wrapper and contained context
}

/// Get user data associated with a context object.
///
/// @memberof [dc_context_t]
///
/// # Parameters
///
/// * `context` - The context object as created by [dc_context_new].
///
/// # Returns
///
/// The user data is returned, this is the second parameter given to
/// [dc_context_new].
#[no_mangle]
pub unsafe extern "C" fn dc_get_userdata(context: *mut dc_context_t) -> *mut libc::c_void {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_get_userdata()");
        return ptr::null_mut();
    }
    let wrapper = &mut *context;
    wrapper.userdata
}

/// Open context database.
///
/// If the given file does not exist, it is created and can be set up
/// using [dc_set_config] afterwards.
///
/// @memberof [dc_context_t]
///
/// @param context The context object as created by [dc_context_new].
/// @param dbfile The file to use to store the database, something like `~/file` won't
///     work on all systems, if in doubt, use absolute paths.
/// @param blobdir A directory to store the blobs in; a trailing slash is not needed.
///     If you pass NULL or the empty string, deltachat-core creates a directory
///     beside _dbfile_ with the same name and the suffix `-blobs`.
///
/// Returns `1` on success, `0` on failure eg. if the file is not
/// writable or if there is already a database opened for the context.
#[no_mangle]
pub unsafe extern "C" fn dc_open(
    context: *mut dc_context_t,
    dbfile: *mut libc::c_char,
    blobdir: *mut libc::c_char,
) -> libc::c_int {
    if conext.is_null() || dbfile.is_null() {
        eprintln!("ignoring careless call to dc_open()");
        return 0;
    }
    let rust_cb = move |_ctx: &Context, evt: Event, d0: uintptr_t, d1: uintptr_t| {
        let wrapper: &ContextWrapper = &*context;
        match wrapper.cb {
            Some(ffi_cb) => ffi_cb(wrapper, evt, d0, d1),
            None => 0,
        }
    };
    let wrapper: &ContextWrapper = &*context;
    let new_context = if blobdir.is_null() {
        Context::new(
            Box::new(rust_cb),
            wrapper.os_name.clone(),
            as_path(dbfile).to_path_buf(),
        )
    } else {
        Context::with_blobdir(
            Box::new(rust_cb),
            wrapper.os_name.clone(),
            as_path(dbfile).to_path_buf(),
            as_path(blobdir),
        )
    };
    match new_context {
        Ok(mut ctx) => {
            // Some structs, e.g. Chatlist, have a reference to the
            // Context and allow to retrieve the context again.
            // Because on the C API we need to return the
            // ContextWrapper rather than the context we store the
            // wrapper as userdata on the Context.  The actual
            // userdata exposed by the C API is stored on the
            // ContextWrapper.
            let wrapper_ptr: *const ContextWrapper = wrapper;
            ctx.userdata = wrapper_ptr as *mut libc::c_void;
            let mut inner_guard = wrapper.inner.write().unwrap();
            *inner_guard = Some(ctx);
            1
        }
        Err(_) => 0,
    }
}

/// Close the context.
///
/// This will disconnect the IMAP and SMTP connections and free some
/// structures.  The context will still be using some memory until
/// [dc_context_unref] is called.
///
/// This method is **not** thread safe.
///
/// This will **block** if there are still objects referencing the
/// context due to the lock protecting the context.

/// Close context database opened by [dc_open].
///
/// Before this, connections to SMTP and IMAP are closed; these
/// connections are started automatically as needed eg. by sending for
/// fetching messages.  This function is also implicitly called by
/// [dc_context_unref].  Multiple calls to this functions are okay,
/// the function takes care not to free objects twice.
///
/// @memberof [dc_context_t]
///
/// @param context The context object as created by [dc_context_new].
#[no_mangle]
pub unsafe extern "C" fn dc_close(context: *mut dc_context_t) {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_close()");
        return;
    }
    let wrapper: &mut ContextWrapper = &mut *context;
    wrapper.inner.write().unwrap().take();
    // Context's Drop impl will close the context.

    // let mut inner_guard = wrapper.inner.write().unwrap();
    // if let Some(ref ctx) = &*inner_guard {
    //     context::dc_close(ctx);
    //     *inner_guard = None;
    // }
}

/// Checks if the context is open.
///
/// Returns `0` if the context is open, `1` if not.
///
/// Note that this is inherently race-condition prone.
#[no_mangle]
pub unsafe extern "C" fn dc_is_open(context: *mut dc_context_t) -> libc::c_int {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_is_open()");
        return 0;
    }
    let wrapper: &mut ContextWrapper = &mut *context;
    let inner_guard = wrapper.inner.read().unwrap();
    match *inner_guard {
        Some(_) => 0,
        None => 1,
    }
}

/// Get the blob directory.
///
/// @memberof [dc_context_t]
///
/// @param context The context object as created by [dc_context_new].
///
/// Returns blob directory associated with the context object, empty
/// string if unset or on errors. NULL is never returned.  The
/// returned string must be free()'d.
#[no_mangle]
pub unsafe extern "C" fn dc_get_blobdir(context: *mut dc_context_t) -> *mut libc::c_char {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_get_blobdir()");
        return dc_strdup(ptr::null());
    }
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    context::dc_get_blobdir(context)
}

/// Sets a configuration option.
///
/// The configuration is handled by key=value pairs as:
///
/// * `addr`         - address to display (always needed)
/// * `mail_server`  - IMAP-server, guessed if left out
/// * `mail_user`    - IMAP-username, guessed if left out
/// * `mail_pw`      - IMAP-password (always needed)
/// * `mail_port`    - IMAP-port, guessed if left out
/// * `send_server`  - SMTP-server, guessed if left out
/// * `send_user`    - SMTP-user, guessed if left out
/// * `send_pw`      - SMTP-password, guessed if left out
/// * `send_port`    - SMTP-port, guessed if left out
/// * `server_flags` - IMAP-/SMTP-flags as a combination of @ref DC_LP flags,
///                    guessed if left out
/// * `displayname`  - Own name to use when sending messages.  MUAs are allowed
///                    to spread this way eg. using CC, defaults to empty
/// * `selfstatus`   - Own status to display eg. in email footers, defaults to
///                    a standard text
/// * `selfavatar`   - File containing avatar. Will be copied to blob directory.
///                    NULL to remove the avatar.
///                    It is planned for future versions
///                    to send this image together with the next messages.
/// * `e2ee_enabled` - 0=no end-to-end-encryption, 1=prefer end-to-end-encryption (default)
/// * `mdns_enabled` - 0=do not send or request read receipts,
///                    1=send and request read receipts (default)
/// * `inbox_watch`  - 1=watch `INBOX`-folder for changes (default),
///                    0=do not watch the `INBOX`-folder
/// * `sentbox_watch`- 1=watch `Sent`-folder for changes (default),
///                    0=do not watch the `Sent`-folder
/// * `mvbox_watch`  - 1=watch `DeltaChat`-folder for changes (default),
///                    0=do not watch the `DeltaChat`-folder
/// * `mvbox_move`   - 1=heuristically detect chat-messages
///                    and move them to the `DeltaChat`-folder,
///                    0=do not move chat-messages
/// * `show_emails`  - DC_SHOW_EMAILS_OFF (0)=
///                    show direct replies to chats only (default),
///                    DC_SHOW_EMAILS_ACCEPTED_CONTACTS (1)=
///                    also show all mails of confirmed contacts,
///                    DC_SHOW_EMAILS_ALL (2)=
///                    also show mails of unconfirmed contacts in the deaddrop.
/// * `save_mime_headers` - 1=save mime headers
///                    and make dc_get_mime_headers() work for subsequent calls,
///                    0=do not save mime headers (default)
///
/// If you want to retrieve a value, use [dc_get_config].
///
/// @memberof [dc_context_t]
///
/// @param context The context object
/// @param key The option to change, see above.
/// @param value The value to save for "key"
///
/// Returns `0` for failure and `1` for success.
#[no_mangle]
pub unsafe extern "C" fn dc_set_config(
    context: *mut dc_context_t,
    key: *mut libc::c_char,
    value: *mut libc::c_char,
) -> libc::c_int {
    if context.is_null() || key.is_null() {
        eprintln!("ignoring careless call to dc_set_config()");
        return 0;
    }
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    match Config::from_str(as_str(key)) {
        // context.set_config() did already log (TODO, it shouldn't)
        Ok(key) => context
            .set_config(key, as_opt_str(value))
            .and(Ok(1))
            .or(Err(0))
            .unwrap(),
        Err(_) => {
            wrapper.error("dc_set_config(): invalid key");
            0
        }
    }
}

/// Gets a configuration option.
///
/// The configuration option is set by dc_set_config() or by the library itself.
///
/// Beside the options shown at dc_set_config(), this function can be
/// used to query some global system values:
///
/// * `sys.version` - get the version string eg. as `1.2.3` or as
///                   `1.2.3special4`
/// * `sys.msgsize_max_recommended` - maximal recommended attachment
///                   size in bytes.  All possible overheads are
///                   already subtracted and this value can be used
///                   eg. for direct comparison with the size of a
///                   file the user wants to attach. If an attachment
///                   is larger than this value, an error (no warning
///                   as it should be shown to the user) is logged
///                   but the attachment is sent anyway.
/// * `sys.config_keys` - get a space-separated list of all
///                   config-keys available.  The config-keys are the
///                   keys that can be passed to the parameter `key`
///                   of this function.
///
/// @memberof [dc_context_t]
///
/// @param context The context object as created by
///     dc_context_new(). For querying system values, this can be NULL.
/// @param key The key to query.
///
/// Returns current value of "key", if "key" is unset, the default value is returned.
///     The returned value must be free()'d, NULL is never returned.
#[no_mangle]
pub unsafe extern "C" fn dc_get_config(
    context: *mut dc_context_t,
    key: *mut libc::c_char,
) -> *mut libc::c_char {
    if context.is_null() || key.is_null() {
        eprintln!("ignoring careless call to dc_get_config()");
        return dc_strdup(ptr::null());
    }
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    let key = Config::from_str(as_str(key)).expect("invalid key");
    // TODO: Translating None to NULL would be more sensible than translating None
    // to "", as it is now.
    context.get_config(key).unwrap_or_default().strdup()
}

/// Gets information about the context.
///
/// The information is returned as a multi-line string and contains
/// information about the current configuration.
///
/// @memberof [dc_context_t]
///
/// @param context The context as created by dc_context_new().
///
/// Returns a string which must be free()'d after usage.  Never
/// returns NULL.
#[no_mangle]
pub unsafe extern "C" fn dc_get_info(context: *mut dc_context_t) -> *mut libc::c_char {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_get_info()");
        return dc_strdup(ptr::null());
    }
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    context::dc_get_info(context)
}

/// Gets url that can be used to initiate an OAuth2 authorisation.
///
/// If an OAuth2 authorization is possible for a given e-mail-address,
/// this function returns the URL that should be opened in a browser.
///
/// If the user authorizes access, the given redirect_uri is called by
/// the provider.  It's up to the UI to handle this call.
///
/// The provider will attach some parameters to the url, most
/// important the parameter `code` that should be set as the
/// `mail_pw`.  With `server_flags` set to #DC_LP_AUTH_OAUTH2,
/// dc_configure() can be called as usual afterwards.
///
/// @memberof [dc_context_t]
///
/// @param context The context object as created by [dc_context_new].
/// @param addr E-mail address the user has entered.
///     In case the user selects a different e-mail-address during
///     authorization, this is corrected in [dc_configure]
/// @param redirect_uri URL that will get `code` that is used as `mail_pw` then.
///     Not all URLs are allowed here, however, the following should work:
///     `chat.delta:/PATH`, `http://localhost:PORT/PATH`,
///     `https://localhost:PORT/PATH`, `urn:ietf:wg:oauth:2.0:oob`
///     (the latter just displays the code the user can copy+paste then)
///
/// Returns URL that can be opened in the browser to start OAuth2.
///     If OAuth2 is not possible for the given e-mail-address, NULL is returned.
#[no_mangle]
pub unsafe extern "C" fn dc_get_oauth2_url(
    context: *mut dc_context_t,
    addr: *mut libc::c_char,
    redirect: *mut libc::c_char,
) -> *mut libc::c_char {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_get_oauth2_url()");
        return ptr::null_mut(); // NULL explicitly defined as "unknown"
    }
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    let addr = to_string(addr);
    let redirect = to_string(redirect);
    match oauth2::dc_get_oauth2_url(context, addr, redirect) {
        Some(res) => res.strdup(),
        None => std::ptr::null_mut(),
    }
}

/// Find out the version of the Delta Chat core library.
///
/// Deprecated, use dc_get_config() instead.
///
/// @memberof [dc_context_t]
///
/// Returns string with version number as `major.minor.revision`. The
/// return value must be free()'d.
#[no_mangle]
pub unsafe extern "C" fn dc_get_version_str() -> *mut libc::c_char {
    context::dc_get_version_str()
}

/// Configures a context.
///
/// For this purpose, the function creates a job that is executed in
/// the IMAP-thread then; this requires to call dc_perform_imap_jobs()
/// regularly.  If the context is already configured, this function
/// will try to change the configuration.
///
/// * Before you call this function, you must set at least `addr` and
///   `mail_pw` using dc_set_config().
///
/// * Use `mail_user` to use a different user name than `addr` and
///   `send_pw` to use a different password for the SMTP server.
///
///     * If _no_ more options are specified,
///       the function **uses autoconfigure/autodiscover**
///       to get the full configuration from well-known URLs.
///
///     * If _more_ options as `mail_server`, `mail_port`, `send_server`,
///       `send_port`, `send_user` or `server_flags` are specified,
///       **autoconfigure/autodiscover is skipped**.
///
/// While this function returns immediately, the started
/// configuration-job may take a while.
///
/// During configuration, #DC_EVENT_CONFIGURE_PROGRESS events are
/// emitted; they indicate a successful configuration as well as
/// errors and may be used to create a progress bar.
///
/// Additional calls to this function while a config-job is running
/// are ignored.  To interrupt a configuration prematurely, use
/// [dc_stop_ongoing_process]; this is not needed if
/// #DC_EVENT_CONFIGURE_PROGRESS reports success.
///
/// On a successfull configuration, the core makes a copy of the
/// parameters mentioned above: the original parameters as are never
/// modified by the core.
///
/// UI-implementors should keep this in mind - eg. if the UI wants to
/// prefill a configure-edit-dialog with these parameters, the UI
/// should reset them if the user cancels the dialog after a
/// configure-attempts has failed.  Otherwise the parameters may not
/// reflect the current configuration.
///
/// @memberof [dc_context_t]
///
/// @param context The context object as created by [dc_context_new].
///
/// There is no need to call [dc_configure] on every program start,
/// the configuration result is saved in the database and you can use
/// the connection directly:
///
/// ```
/// if (!dc_is_configured(context)) {
///     dc_configure(context);
///     // wait for progress events
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn dc_configure(context: *mut dc_context_t) {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_configure()");
        return;
    }
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    configure::configure(context)
}

/// Checks if the context is already configured.
///
/// Typically, for unconfigured accounts, the user is prompted to
/// enter some settings and [dc_configure] is called in a thread then.
///
/// @memberof [dc_context_t]
///
/// @param context The context object as created by dc_context_new().
///
///Returns `1` if the context is configured and can be used; `0` if
///    the context is not configured and a configuration by
///    [dc_configure] is required.
#[no_mangle]
pub unsafe extern "C" fn dc_is_configured(context: *mut dc_context_t) -> libc::c_int {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_is_configured()");
        return 0;
    }
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    configure::dc_is_configured(context)
}

/// Executes pending imap-jobs.
///
/// This function and [dc_perform_imap_fetch] and [dc_perform_imap_idle]
/// must be called from the same thread, typically in a loop.
///
/// Example:
///
/// ```
/// void* imap_thread_func(void* context)
/// {
///     while (true) {
///         dc_perform_imap_jobs(context);
///         dc_perform_imap_fetch(context);
///         dc_perform_imap_idle(context);
///     }
/// }
///
/// // start imap-thread that runs forever
/// pthread_t imap_thread;
/// pthread_create(&imap_thread, NULL, imap_thread_func, context);
///
/// ... program runs ...
///
/// // network becomes available again -
/// // the interrupt causes dc_perform_imap_idle() in the thread above
/// // to return so that jobs are executed and messages are fetched.
/// dc_maybe_network(context);
/// ```
///
/// @memberof [dc_context_t]
///
/// @param context The context as created by [dc_context_new].
#[no_mangle]
pub unsafe extern "C" fn dc_perform_imap_jobs(context: *mut dc_context_t) {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_perform_imap_jobs()");
        return;
    }
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    job::perform_imap_jobs(context)
}

/// Fetches new messages, if any.
///
/// This function and [dc_perform_imap_jobs] and
/// [dc_perform_imap_idle] must be called from the same thread,
/// typically in a loop.
///
/// See [dc_perform_imap_jobs] for an example.
///
/// @memberof [dc_context_t]
///
/// @param context The context as created by [dc_context_new].
#[no_mangle]
pub unsafe extern "C" fn dc_perform_imap_fetch(context: *mut dc_context_t) {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_perform_imap_fetch()");
        return;
    }
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    job::perform_imap_fetch(context)
}

/// Waits for messages or jobs.
///
/// This function and [dc_perform_imap_jobs] and
/// [dc_perform_imap_fetch] must be called from the same thread,
/// typically in a loop.
///
/// You should call this function directly after calling [dc_perform_imap_fetch].
///
/// See [dc_perform_imap_jobs] for an example.
///
/// @memberof [dc_context_t]
///
/// @param context The context as created by [dc_context_new].
#[no_mangle]
pub unsafe extern "C" fn dc_perform_imap_idle(context: *mut dc_context_t) {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_perform_imap_idle()");
        return;
    }
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    job::perform_imap_idle(context)
}

/// Interrupts waiting for imap-jobs.
///
/// If dc_perform_imap_jobs(), dc_perform_imap_fetch() and
/// dc_perform_imap_idle() are called in a loop, calling this function
/// causes imap-jobs to be executed and messages to be fetched.
///
/// [dc_interrupt_imap_idle] does _not_ [interrupt
/// dc_perform_imap_jobs] or [dc_perform_imap_fetch].  If the
/// imap-thread is inside one of these functions when
/// [dc_interrupt_imap_idle] is called, however, the next call of the
/// imap-thread to [dc_perform_imap_idle] is interrupted immediately.
///
/// Internally, this function is called whenever a imap-jobs should be
/// processed (delete message, markseen etc.).
///
/// When you need to call this function just because to get jobs done
/// after network changes, use [dc_maybe_network] instead.
///
/// @memberof [dc_context_t]
///
/// # Parameters
///
/// * `context` - The context as created by [dc_context_new].
///
/// # Panics
///
/// It is safe to call this function on a closed context.
#[no_mangle]
pub unsafe extern "C" fn dc_interrupt_imap_idle(context: *mut dc_context_t) {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_interrupt_imap_idle()");
        return;
    }
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    if inner_guard.is_some() {
        let context = inner_guard.as_ref().expect("context not open");
        job::interrupt_imap_idle(context);
    }
}

/// Fetches new messages from the MVBOX, if any.
///
/// The MVBOX is a folder on the account where chat messages are moved
/// to.  The moving is done to not disturb shared accounts that are
/// used by both, Delta Chat and a classical MUA.
///
/// This function and [dc_perform_mvbox_idle] must be called from the
/// same thread, typically in a loop.
///
/// Example:
///
/// ```
/// void* mvbox_thread_func(void* context)
/// {
///     while (true) {
///         dc_perform_mvbox_fetch(context);
///         dc_perform_mvbox_idle(context);
///     }
/// }
///
/// // start mvbox-thread that runs forever
/// pthread_t mvbox_thread;
/// pthread_create(&mvbox_thread, NULL, mvbox_thread_func, context);
///
/// ... program runs ...
///
/// // network becomes available again -
/// // the interrupt causes dc_perform_mvbox_idle() in the thread above
/// // to return so that and messages are fetched.
/// dc_maybe_network(context);
/// ```
///
/// @memberof [dc_context_t]
///
/// @param context The context as created by dc_context_new().
#[no_mangle]
pub unsafe extern "C" fn dc_perform_mvbox_fetch(context: *mut dc_context_t) {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_perform_mvbox_fetch()");
        return;
    }
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    job::perform_mvbox_fetch(context)
}

/// Waits for messages or jobs in the MVBOX-thread.
///
/// This function and [dc_perform_mvbox_fetch].  must be called from
/// the same thread, typically in a loop.
///
/// You should call this function directly after calling
/// [dc_perform_mvbox_fetch].
///
/// See [dc_perform_mvbox_fetch] for an example.
///
/// @memberof [dc_context_t]
///
/// @param context The context as created by [dc_context_new].
#[no_mangle]
pub unsafe extern "C" fn dc_perform_mvbox_idle(context: *mut dc_context_t) {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_perform_mvbox_idle()");
        return;
    }
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    job::perform_mvbox_idle(context)
}

/// Interrupts waiting for MVBOX-fetch.
///
/// [dc_interrupt_mvbox_idle] does _not_ interrupt
/// [dc_perform_mvbox_fetch].  If the MVBOX-thread is inside this
/// function when [dc_interrupt_mvbox_idle] is called, however, the
/// next call of the MVBOX-thread to [dc_perform_mvbox_idle] is
/// interrupted immediately.
///
/// Internally, this function is called whenever a imap-jobs should be
/// processed.
///
/// When you need to call this function just because to get jobs done
/// after network changes, use [dc_maybe_network] instead.
///
/// @memberof [dc_context_t]
///
/// @param context The context as created by [dc_context_new].
#[no_mangle]
pub unsafe extern "C" fn dc_interrupt_mvbox_idle(context: *mut dc_context_t) {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_interrupt_mvbox_idle()");
        return;
    }
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    job::interrupt_mvbox_idle(context)
}

/// Fetches new messages from the Sent folder, if any.
///
/// This function and [dc_perform_sentbox_idle] must be called from
/// the same thread, typically in a loop.
///
/// @memberof [dc_context_t]
///
/// @param context The context as created by [dc_context_new].
#[no_mangle]
pub unsafe extern "C" fn dc_perform_sentbox_fetch(context: *mut dc_context_t) {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_perform_sentbox_fetch()");
        return;
    }
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    job::perform_sentbox_fetch(context)
}

/// Waits for messages or jobs in the SENTBOX-thread.
///
/// This function and [dc_perform_sentbox_fetch] must be called from
/// the same thread, typically in a loop.
///
/// @memberof [dc_context_t]
///
/// @param context The context as created by [dc_context_new].
#[no_mangle]
pub unsafe extern "C" fn dc_perform_sentbox_idle(context: *mut dc_context_t) {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_perform_sentbox_idle()");
        return;
    }
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    job::perform_sentbox_idle(context)
}

/// Interrupts waiting for messages or jobs in the SENTBOX-thread.
///
/// @memberof [dc_context_t]
///
/// @param context The context as created by [dc_context_new].
#[no_mangle]
pub unsafe extern "C" fn dc_interrupt_sentbox_idle(context: *mut dc_context_t) {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_interrupt_sentbox_idle()");
        return;
    }
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    job::interrupt_sentbox_idle(context)
}

/// Executes pending smtp-jobs.
///
/// This function and dc_perform_smtp_idle() must be called from the
/// same thread, typically in a loop.
///
/// Example:
///
/// ```
/// void* smtp_thread_func(void* context)
/// {
///     while (true) {
///         dc_perform_smtp_jobs(context);
///         dc_perform_smtp_idle(context);
///     }
/// }
///
/// // start smtp-thread that runs forever
/// pthread_t smtp_thread;
/// pthread_create(&smtp_thread, NULL, smtp_thread_func, context);
///
/// ... program runs ...
///
/// // network becomes available again -
/// // the interrupt causes dc_perform_smtp_idle() in the thread above
/// // to return so that jobs are executed
/// dc_maybe_network(context);
/// ```
///
/// @memberof [dc_context_t]
///
/// @param context The context as created by [dc_context_new].
#[no_mangle]
pub unsafe extern "C" fn dc_perform_smtp_jobs(context: *mut dc_context_t) {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_perform_smtp_jobs()");
        return;
    }
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    job::perform_smtp_jobs(context)
}

/// Waits for smtp-jobs.
///
/// This function and [dc_perform_smtp_jobs] must be called from the
/// same thread, typically in a loop.
///
/// See [dc_interrupt_smtp_idle] for an example.
///
/// @memberof [dc_context_t]
///
/// @param context The context as created by [dc_context_new].
#[no_mangle]
pub unsafe extern "C" fn dc_perform_smtp_idle(context: *mut dc_context_t) {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_perform_smtp_idle()");
        return;
    }
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    job::perform_smtp_idle(context)
}

/// Interrupts waiting for smtp-jobs.
///
/// If [dc_perform_smtp_jobs] and [dc_perform_smtp_idle] are called in
/// a loop, calling this function causes jobs to be executed.
///
/// [dc_interrupt_smtp_idle] does _not_ interrupt
/// [dc_perform_smtp_jobs].  If the smtp-thread is inside this
/// function when [dc_interrupt_smtp_idle] is called, however, the
/// next call of the smtp-thread to [dc_perform_smtp_idle] is
/// interrupted immediately.
///
/// Internally, this function is called whenever a message is to be
/// sent.
///
/// When you need to call this function just because to get jobs done
/// after network changes, use [dc_maybe_network] instead.
///
/// @memberof [dc_context_t]
///
/// # Arguments
///
/// * `context` - The context as created by [dc_context_new].
///
/// # Panics
///
/// It is safe to call this function on a closed context.
#[no_mangle]
pub unsafe extern "C" fn dc_interrupt_smtp_idle(context: *mut dc_context_t) {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_interrupt_smtp_idle()");
        return;
    }
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    if inner_guard.is_some() {
        let context = inner_guard.as_ref().expect("context not open");
        job::interrupt_smtp_idle(context)
    }
}

/// Signals possible network availability.
///
/// This function can be called whenever there is a hint.  that the
/// network is available again.  The library will try to send pending
/// messages out.
///
/// @memberof [dc_context_t]
///
/// @param context The context as created by [dc_context_new].
#[no_mangle]
pub unsafe extern "C" fn dc_maybe_network(context: *mut dc_context_t) {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_maybe_network()");
        return;
    }
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    job::maybe_network(context)
}

/// Gets a list of chats.
///
/// The list can be filtered by query parameters.
///
/// The list is already sorted and starts with the most recent chat in
/// use.  The sorting takes care of invalid sending dates, drafts and
/// chats without messages.  Clients should not try to re-sort the
/// list as this would be an expensive action and would result in
/// inconsistencies between clients.
///
/// To get information about each entry, use
/// eg. [dc_chatlist_get_summary].
///
/// By default, the function adds some special entries to the list.
/// These special entries can be identified by the ID returned by
/// [dc_chatlist_get_chat_id]:
///
/// * DC_CHAT_ID_DEADDROP (1) - this special chat is present if there
///   are messages from addresses that have no relationship to the
///   configured account.  The last of these messages is represented
///   by DC_CHAT_ID_DEADDROP and you can retrieve details about it
///   with [dc_chatlist_get_msg_id]. Typically, the UI asks the user
///   "Do you want to chat with NAME?"  and offers the options "Yes"
///   (call [dc_create_chat_by_msg_id]), "Never" (call
///   [dc_block_contact]) or "Not now".  The UI can also offer a
///   "Close" button that calls [dc_marknoticed_contact] then.
///
/// * DC_CHAT_ID_ARCHIVED_LINK (6) - this special chat is present if
///   the user has archived _any_ chat using [dc_archive_chat]. The UI
///   should show a link as "Show archived chats", if the user clicks
///   this item, the UI should show a list of all archived chats that
///   can be created by this function hen using the
///   DC_GCL_ARCHIVED_ONLY flag.
///
/// * DC_CHAT_ID_ALLDONE_HINT (7) - this special chat is present if
///   DC_GCL_ADD_ALLDONE_HINT is added to listflags and if there are
///   only archived chats.
///
/// @memberof [dc_context_t]
///
/// @param context The context object as returned by [dc_context_new]
/// @param listflags A combination of flags:
///     - if the flag DC_GCL_ARCHIVED_ONLY is set, only archived chats are returned.
///       if DC_GCL_ARCHIVED_ONLY is not set, only unarchived chats are returned and
///       the pseudo-chat DC_CHAT_ID_ARCHIVED_LINK is added if there are _any_ archived
///       chats
///     - if the flag DC_GCL_NO_SPECIALS is set, deaddrop and archive link are not added
///       to the list (may be used eg. for selecting chats on forwarding, the flag is
///       not needed when DC_GCL_ARCHIVED_ONLY is already set)
///     - if the flag DC_GCL_ADD_ALLDONE_HINT is set, DC_CHAT_ID_ALLDONE_HINT
///       is added as needed.
/// @param query_str An optional query for filtering the list.  Only
///     chats matching this query are returned.  Give NULL for no
///     filtering.
/// @param query_id An optional contact ID for filtering the list.
///     Only chats including this contact ID are returned.  Give 0 for
///     no filtering.
///
/// Returns a chatlist as a [dc_chatlist_t] object.  On errors, NULL is
/// returned.  Must be freed using [dc_chatlist_unref] when no longer
/// used.
///
/// See also: [dc_get_chat_msgs] to get the messages of a single chat.
#[no_mangle]
pub unsafe extern "C" fn dc_get_chatlist<'a>(
    context: *mut dc_context_t,
    flags: libc::c_int,
    query_str: *mut libc::c_char,
    query_id: u32,
) -> *mut dc_chatlist_t<'a> {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_get_chatlist()");
        return ptr::null_mut();
    }
    let wrapper: &ContextWrapper = &*context;
    let qs = if query_str.is_null() {
        None
    } else {
        Some(as_str(query_str))
    };
    let qi = if query_id == 0 { None } else { Some(query_id) };
    let maybe_wrapper = ChatlistWrapper::try_new(wrapper.inner.read().unwrap(), |guard| {
        let context = guard.as_ref().expect("context not open");
        chatlist::Chatlist::try_load(context, flags as usize, qs, qi)
    });
    match maybe_wrapper {
        Ok(list) => Box::into_raw(Box::new(list)),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Create a normal chat or a group chat by a messages ID that comes
/// typically from the deaddrop, DC_CHAT_ID_DEADDROP (1).
///
/// If the given message ID already belongs to a normal chat or to a
/// group chat, the chat ID of this chat is returned and no new chat
/// is created.  If a new chat is created, the given message ID is
/// moved to this chat, however, there may be more messages moved to
/// the chat from the deaddrop. To get the chat messages, use
/// [dc_get_chat_msgs].
///
/// If the user is asked before creation, they should be asked whether
/// they wants to chat with the _contact_ belonging to the message; the
/// group names may be really weird when taken from the subject of
/// implicit groups and this may look confusing.
///
/// Moreover, this function also scales up the origin of the contact belonging
/// to the message and, depending on the contacts origin, messages from the
/// same group may be shown or not - so, all in all, it is fine to show the
/// contact name only.
///
/// @memberof [dc_context_t]
///
/// @param context The context object as returned from [dc_context_new].
/// @param msg_id The message ID to create the chat for.
///
/// Returns the created or reused chat ID on success. `0` on errors.
#[no_mangle]
pub unsafe extern "C" fn dc_create_chat_by_msg_id(context: *mut dc_context_t, msg_id: u32) -> u32 {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_create_chat_by_msg_id()");
        return 0;
    }
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    chat::create_by_msg_id(context, msg_id).unwrap_or_log_default(context, "Failed to create chag")
}

/// Create a normal chat with a single user.
///
/// To create group chats, see [dc_create_group_chat].
///
/// If a chat already exists, this ID is returned, otherwise a new
/// chat is created; this new chat may already contain messages,
/// eg. from the deaddrop, to get the chat messages, use
/// [dc_get_chat_msgs].
///
/// @memberof [dc_context_t]
///
/// @param context The context object as returned from [dc_context_new].
/// @param contact_id The contact ID to create the chat for.  If there is already
///     a chat with this contact, the already existing ID is returned.
///
/// Returns the created or reused chat ID on success. `0` on errors.
#[no_mangle]
pub unsafe extern "C" fn dc_create_chat_by_contact_id(
    context: *mut dc_context_t,
    contact_id: u32,
) -> u32 {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_create_chat_by_contact_id()");
        return 0;
    }
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    chat::create_by_contact_id(context, contact_id)
        .unwrap_or_log_default(context, "Failed to create chat")
}

/// Check, if there is a normal chat with a given contact.
///
/// To get the chat messages, use [dc_get_chat_msgs].
///
/// @memberof [dc_context_t]
///
/// @param context The context object as returned from [dc_context_new].
/// @param contact_id The contact ID to check.
///
/// If there is a normal chat with the given contact_id, this chat_id
/// is returned.  If there is no normal chat with the contact_id, the
/// function returns `0`.
#[no_mangle]
pub unsafe extern "C" fn dc_get_chat_id_by_contact_id(
    context: *mut dc_context_t,
    contact_id: u32,
) -> u32 {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_get_chat_id_by_contact_id()");
        return 0;
    }
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    chat::get_by_contact_id(context, contact_id)
        .unwrap_or_log_default(context, "Failed to get chat")
}

/// Prepares a message for sending.
///
/// Call this function if the file to be sent is still in creation.
/// Once you're done with creating the file, call [dc_send_msg] as
/// usual and the message will really be sent.
///
/// This is useful as the user can already send the next messages
/// while e.g. the recoding of a video is not yet finished. Or the
/// user can even forward the message with the file being still in
/// creation to other groups.
///
/// Files being sent with the increation-method must be placed in the
/// blob directory, see [dc_get_blobdir].  If the increation-method is
/// not used - which is probably the normal case - dc_send_msg()
/// copies the file to the blob directory if it is not yet there.  To
/// distinguish the two cases, msg->state must be set properly. The
/// easiest way to ensure this is to re-use the same object for both
/// calls.
///
/// # Example
///
/// ```c
/// dc_msg_t* msg = dc_msg_new(context, DC_MSG_VIDEO);
/// dc_msg_set_file(msg, "/file/to/send.mp4", NULL);
/// dc_prepare_msg(context, chat_id, msg);
/// // ... after /file/to/send.mp4 is ready:
/// dc_send_msg(context, chat_id, msg);
/// ```
///
/// @memberof [dc_context_t]
///
/// @param context The context object as returned from [dc_context_new].
/// @param chat_id Chat ID to send the message to.
/// @param msg Message object to send to the chat defined by the chat ID.
///     On succcess, `msg_id` and state of the object are set up,
///     The function does not take ownership of the object,
///     so you have to free it using [dc_msg_unref] as usual.
///
/// Retuns the ID of the message that is being prepared.
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
    let msg = &mut *msg;
    msg.rent_mut(|m| {
        chat::prepare_msg(m.context, chat_id, m)
            .unwrap_or_log_default(m.context, "Failed to prepare message")
    })
}

/// Sends a message defined by a dc_msg_t object to a chat.
///
/// Sends the event #DC_EVENT_MSGS_CHANGED on succcess.  However, this
/// does not imply, the message really reached the recipient - sending
/// may be delayed eg. due to network problems. However, from your
/// view, you're done with the message. Sooner or later it will find
/// its way.
///
/// # Example
///
/// ```c
/// dc_msg_t* msg = dc_msg_new(context, DC_MSG_IMAGE);
/// dc_msg_set_file(msg, "/file/to/send.jpg", NULL);
/// dc_send_msg(context, chat_id, msg);
/// ```
///
/// @memberof [dc_context_t]
/// @param context The context object as returned from [dc_context_new].
/// @param chat_id Chat ID to send the message to.
///     If [dc_prepare_msg] was called before, this parameter can be 0.
/// @param msg Message object to send to the chat defined by the chat ID.
///     On succcess, msg_id of the object is set up,
///     The function does not take ownership of the object,
///     so you have to free it using [dc_msg_unref] as usual.
///
/// Returns the ID of the message that is about to be sent. `0` in
/// case of errors.
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
    let msg = &mut *msg;
    msg.rent_mut(|m| {
        chat::send_msg(m.context, chat_id, m)
            .unwrap_or_log_default(m.context, "Failed to send message")
    })
}

/// Sends a simple text message a given chat.
///
/// Sends the event #DC_EVENT_MSGS_CHANGED on succcess.  However, this
/// does not imply, the message really reached the recipient - sending
/// may be delayed eg. due to network problems. However, from your
/// view, you're done with the message. Sooner or later it will find
/// its way.
///
/// See also [dc_send_msg].
///
/// @memberof [dc_context_t]
///
/// @param context The context object as returned from [dc_context_new].
/// @param chat_id Chat ID to send the text message to.
/// @param text_to_send Text to send to the chat defined by the chat ID.
///     Passing an empty text here causes an empty text to be sent,
///     it's up to the caller to handle this if undesired.
///     Passing NULL as the text causes the function to return 0.
///
/// Returns the ID of the message that is about being sent.
#[no_mangle]
pub unsafe extern "C" fn dc_send_text_msg(
    context: *mut dc_context_t,
    chat_id: u32,
    text_to_send: *mut libc::c_char,
) -> u32 {
    if context.is_null() || text_to_send.is_null() {
        eprintln!("ignoring careless call to dc_send_text_msg()");
        return 0;
    }
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    let text_to_send = dc_tools::to_string_lossy(text_to_send);
    chat::send_text_msg(context, chat_id, text_to_send)
        .unwrap_or_log_default(context, "Failed to send text message")
}

/// Saves a draft for a chat in the database.
//
/// The UI should call this function if the user has prepared a
/// message and exits the compose window without clicking the "send"
/// button before.  When the user later opens the same chat again, the
/// UI can load the draft using [dc_get_draft] allowing the user to
/// continue editing and sending.
///
/// Drafts are considered when sorting messages
/// and are also returned eg. by [dc_chatlist_get_summary].
///
/// Each chat can have its own draft but only one draft per chat is
/// possible.
///
/// If the draft is modified, an #DC_EVENT_MSGS_CHANGED will be sent.
///
/// @memberof [dc_context_t]
///
/// @param context The context as created by [dc_context_new].
/// @param chat_id The chat ID to save the draft for.
/// @param msg The message to save as a draft.
///     Existing draft will be overwritten.
///     NULL deletes the existing draft, if any, without sending it.
///     Currently, also non-text-messages
///     will delete the existing drafts.
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
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    if msg.is_null() {
        chat::set_draft(context, chat_id, None)
    } else {
        let msg = &mut *msg;
        msg.rent_mut(|m| chat::set_draft(context, chat_id, Some(m)))
    }
}

/// Gets draft for a chat, if any.
///
/// See [dc_set_draft] for more details about drafts.
///
/// @memberof [dc_context_t]
///
/// @param context The context as created by [dc_context_new].
/// @param chat_id The chat ID to get the draft for.
///
/// Returns a message object.  Can be passed directly to
/// [dc_send_msg].  Must be freed using [dc_msg_unref] after usage.
/// If there is no draft, NULL is returned.
#[no_mangle]
pub unsafe extern "C" fn dc_get_draft<'a>(
    context: *mut dc_context_t,
    chat_id: u32,
) -> *mut dc_msg_t<'a> {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_get_draft()");
        return ptr::null_mut(); // NULL explicitly defined as "no draft"
    }
    let wrapper: &ContextWrapper = &*context;
    let maybe_msg = MessageWrapper::try_new(wrapper.inner.read().unwrap(), |inner_guard| {
        let context = inner_guard.as_ref().expect("context not open");
        chat::get_draft(context, chat_id)
    });
    match maybe_msg {
        Ok(msg) => Box::into_raw(Box::new(msg)),
        Err(_) => std::ptr::null_mut(), // log something
    }
}

/// Gets all message IDs belonging to a chat.
///
/// The list is already sorted and starts with the oldest message.
/// Clients should not try to re-sort the list as this would be an
/// expensive action and would result in inconsistencies between
/// clients.
///
/// Optionally, some special markers added to the ID-array may help to
/// implement virtual lists.
///
/// @memberof [dc_context_t]
///
/// @param context The context object as returned from [dc_context_new].
/// @param chat_id The chat ID of which the messages IDs should be queried.
/// @param flags If set to DC_GCM_ADDDAYMARKER, the marker
///     DC_MSG_ID_DAYMARKER will be added before each day (regarding
///     the local timezone).  Set this to 0 if you do not want this
///     behaviour.
/// @param marker1before An optional message ID.  If set, the id
///     DC_MSG_ID_MARKER1 will be added just before the given ID in
///     the returned array.  Set this to 0 if you do not want this
///     behaviour.
///
/// Returns an array of message IDs, must be [dc_array_unref]'d when
/// no longer used.
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
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    let arr = dc_array_t::from(chat::get_chat_msgs(context, chat_id, flags, marker1before));
    Box::into_raw(Box::new(arr))
}

/// Gets the total number of messages in a chat.
///
/// @memberof [dc_context_t]
///
/// @param context The context object as returned from [dc_context_new].
/// @param chat_id The ID of the chat to count the messages for.
///
/// Returns the number of total messages in the given chat. `0` for
/// errors or empty chats.
#[no_mangle]
pub unsafe extern "C" fn dc_get_msg_cnt(context: *mut dc_context_t, chat_id: u32) -> libc::c_int {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_get_msg_cnt()");
        return 0;
    }
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    chat::get_msg_cnt(context, chat_id) as libc::c_int
}

/// Gets the number of _fresh_ messages in a chat.
///
/// Typically used to implement a badge with a number in the chatlist.
///
/// @memberof [dc_context_t]
///
/// @param context The context object as returned from [dc_context_new].
/// @param chat_id The ID of the chat to count the messages for.
///
/// Returns the number of fresh messages in the given chat. `0` for
/// errors or if there are no fresh messages.
#[no_mangle]
pub unsafe extern "C" fn dc_get_fresh_msg_cnt(
    context: *mut dc_context_t,
    chat_id: u32,
) -> libc::c_int {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_get_fresh_msg_cnt()");
        return 0;
    }
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    chat::get_fresh_msg_cnt(context, chat_id) as libc::c_int
}

/// Returns the message IDs of all _fresh_ messages of any chat.
///
/// Typically used for implementing notification summaries.  The list
/// is already sorted and starts with the most recent fresh message.
///
/// @memberof [dc_context_t]
///
/// @param context The context object as returned from [dc_context_new].
///
/// Returns an array of message IDs, must be [dc_array_unref]'d when
/// no longer used.  On errors, the list is empty.  NULL is never
/// returned.
#[no_mangle]
pub unsafe extern "C" fn dc_get_fresh_msgs(
    context: *mut dc_context_t,
) -> *mut dc_array::dc_array_t {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_get_fresh_msgs()");
        return ptr::null_mut();
    }
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    let arr = dc_array_t::from(context::dc_get_fresh_msgs(context));
    Box::into_raw(Box::new(arr))
}

/// Mark all messages in a chat as _noticed_.
///
/// _Noticed_ messages are no longer _fresh_ and do not count as being unseen
/// but are still waiting for being marked as "seen" using [dc_markseen_msgs]
/// (IMAP/MDNs is not done for noticed messages).
///
/// Calling this function usually results in the event
/// #DC_EVENT_MSGS_CHANGED.  See also [dc_marknoticed_all_chats],
/// [dc_marknoticed_contact] and [dc_markseen_msgs].
///
/// @memberof [dc_context_t]
///
/// @param context The context object as returned from [dc_context_new].
/// @param chat_id The chat ID of which all messages should be marked as being noticed.
#[no_mangle]
pub unsafe extern "C" fn dc_marknoticed_chat(context: *mut dc_context_t, chat_id: u32) {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_marknoticed_chat()");
        return;
    }
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    chat::marknoticed_chat(context, chat_id).log_err(context, "Failed marknoticed chat");
}

/// Same as dc_marknoticed_chat() but for _all_ chats.
///
/// @memberof [dc_context_t]
///
/// @param context The context object as returned from [dc_context_new].
#[no_mangle]
pub unsafe extern "C" fn dc_marknoticed_all_chats(context: *mut dc_context_t) {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_marknoticed_all_chats()");
        return;
    }
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    chat::marknoticed_all_chats(context).log_err(context, "Failed marknoticed all chats");
}

fn from_prim<S, T>(s: S) -> Option<T>
where
    T: FromPrimitive,
    S: Into<i64>,
{
    FromPrimitive::from_i64(s.into())
}

/// Returns all message IDs of the given types in a chat.
///
/// Typically used to show a gallery.
/// The result must be [dc_array_unref]'d
///
/// The list is already sorted and starts with the oldest message.
/// Clients should not try to re-sort the list as this would be an
/// expensive action and would result in inconsistencies between
/// clients.
///
/// @memberof [dc_context_t]
///
/// @param context The context object as returned from [dc_context_new].
/// @param chat_id The chat ID to get all messages with media from.
/// @param msg_type Specify a message type to query here, one of the DC_MSG_* constats.
/// @param msg_type2 Alternative message type to search for. 0 to skip.
/// @param msg_type3 Alternative message type to search for. 0 to skip.
///
/// Returns an array with messages from the given chat ID that have
/// the wanted message types.
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
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");

    let msg_type = from_prim(msg_type).expect(&format!("invalid msg_type = {}", msg_type));
    let or_msg_type2 =
        from_prim(or_msg_type2).expect(&format!("incorrect or_msg_type2 = {}", or_msg_type2));
    let or_msg_type3 =
        from_prim(or_msg_type3).expect(&format!("incorrect or_msg_type3 = {}", or_msg_type3));

    let arr = dc_array_t::from(chat::get_chat_media(
        context,
        chat_id,
        msg_type,
        or_msg_type2,
        or_msg_type3,
    ));
    Box::into_raw(Box::new(arr))
}

/// Search next/previous message based on a given message and a list of types.
///
/// The Typically used to implement the "next" and "previous" buttons
/// in a gallery or in a media player.
///
/// @memberof [dc_context_t]
///
/// @param context The context object as returned from [dc_context_new].
/// @param curr_msg_id  This is the current message
///     from which the next or previous message should be searched.
/// @param dir 1=get the next message, -1=get the previous one.
/// @param msg_type Message type to search for.
///     If 0, the message type from curr_msg_id is used.
/// @param msg_type2 Alternative message type to search for. 0 to skip.
/// @param msg_type3 Alternative message type to search for. 0 to skip.
///
/// Returns the message ID that should be played next.  The returned
/// message is in the same chat as the given one and has one of the
/// given types.  Typically, this result is passed again to
/// [dc_get_next_media] later on the next swipe.  If there is not
/// next/previous message, the function returns 0.
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
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");

    let msg_type = from_prim(msg_type).expect(&format!("invalid msg_type = {}", msg_type));
    let or_msg_type2 =
        from_prim(or_msg_type2).expect(&format!("incorrect or_msg_type2 = {}", or_msg_type2));
    let or_msg_type3 =
        from_prim(or_msg_type3).expect(&format!("incorrect or_msg_type3 = {}", or_msg_type3));

    chat::get_next_media(context, msg_id, dir, msg_type, or_msg_type2, or_msg_type3)
}

/// Archives or unarchives a chat.
///
/// Archived chats are not included in the default chatlist returned
/// by dc_get_chatlist().  Instead, if there are _any_ archived chats,
/// the pseudo-chat with the chat_id DC_CHAT_ID_ARCHIVED_LINK will be
/// added the the end of the chatlist.
///
/// * To get a list of archived chats, use dc_get_chatlist() with the
///   flag DC_GCL_ARCHIVED_ONLY.
///
/// * To find out the archived state of a given chat, use
///   [dc_chat_get_archived]
///
/// * Messages in archived chats are marked as being noticed, so they
///   do not count as "fresh"
///
/// * Calling this function usually results in the event
///   #DC_EVENT_MSGS_CHANGED
///
/// @memberof [dc_context_t]
///
/// @param context The context object as returned from [dc_context_new].
/// @param chat_id The ID of the chat to archive or unarchive.
/// @param archive 1=archive chat, 0=unarchive chat, all other values
///     are reserved for future use
#[no_mangle]
pub unsafe extern "C" fn dc_archive_chat(
    context: *mut dc_context_t,
    chat_id: u32,
    archive: libc::c_int,
) {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_archive_chat()");
        return;
    }
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    let archive = if archive == 0 {
        false
    } else if archive == 1 {
        true
    } else {
        return;
    };
    chat::archive(context, chat_id, archive).log_err(context, "Failed archive chat");
}

/// Deletes a chat.
///
/// Messages are deleted from the device and the chat database entry
/// is deleted.  After that, the event #DC_EVENT_MSGS_CHANGED is
/// posted.
///
/// Things that are _not_ done implicitly:
///
/// * Messages are **not deleted from the server**.
/// * The chat or the contact is **not blocked**, so new messages from
///   the user/the group may appear and the user may create the chat
///   again.
/// * **Groups are not left** - this would be unexpected as (1)
///   deleting a normal chat also does not prevent new mails from
///   arriving, (2) leaving a group requires sending a message to all
///   group members - especially for groups not used for a longer
///   time, this is really unexpected when deletion results in
///   contacting all members again, (3) only leaving groups is also a
///   valid usecase.
///
/// To leave a chat explicitly, use dc_remove_contact_from_chat() with
/// chat_id=DC_CONTACT_ID_SELF)
///
/// @memberof [dc_context_t]
///
/// @param context The context object as returned from [dc_context_new].
/// @param chat_id The ID of the chat to delete.
#[no_mangle]
pub unsafe extern "C" fn dc_delete_chat(context: *mut dc_context_t, chat_id: u32) {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_delete_chat()");
        return;
    }
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    chat::delete(context, chat_id).log_err(context, "Failed chat delete");
}

/// Gets contact IDs belonging to a chat.
///
/// * for normal chats, the function always returns exactly one
///   contact, DC_CONTACT_ID_SELF is returned only for SELF-chats.
///
/// * for group chats all members are returned, DC_CONTACT_ID_SELF is
///   returned explicitly as it may happen that oneself gets removed
///   from a still existing group
///
/// * for the deaddrop, the list is empty
///
/// @memberof [dc_context_t]
///
/// @param context The context object as returned from [dc_context_new].
/// @param chat_id Chat ID to get the belonging contact IDs for.
///
/// Returns an array of contact IDs belonging to the chat; must be
/// freed using [dc_array_unref] when done.
#[no_mangle]
pub unsafe extern "C" fn dc_get_chat_contacts(
    context: *mut dc_context_t,
    chat_id: u32,
) -> *mut dc_array::dc_array_t {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_get_chat_contacts()");
        return ptr::null_mut();
    }
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    let arr = dc_array_t::from(chat::get_chat_contacts(context, chat_id));
    Box::into_raw(Box::new(arr))
}

/// Searches messages containing the given query string.
///
/// Searching can be done globally (chat_id=0) or in a specified chat
/// only (chat_id set).
///
/// Global chat results are typically displayed using
/// [dc_msg_get_summary], chat search results may just hilite the
/// corresponding messages and present a prev/next button.
///
/// @memberof [dc_context_t]
///
/// @param context The context object as returned from [dc_context_new].
/// @param chat_id ID of the chat to search messages in.
///     Set this to 0 for a global search.
/// @param query The query to search for.
///
/// Returns an array of message IDs. Must be freed using
/// [dc_array_unref] when no longer needed.  If nothing can be found,
/// the function returns NULL.
#[no_mangle]
pub unsafe extern "C" fn dc_search_msgs(
    context: *mut dc_context_t,
    chat_id: u32,
    query: *mut libc::c_char,
) -> *mut dc_array::dc_array_t {
    if context.is_null() || query.is_null() {
        eprintln!("ignoring careless call to dc_search_msgs()");
        return ptr::null_mut();
    }
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    let arr = dc_array_t::from(context::dc_search_msgs(context, chat_id, query));
    Box::into_raw(Box::new(arr))
}

/// Gets chat object by a chat ID.
///
/// @memberof [dc_context_t]
/// @param context The context object as returned from [dc_context_new].
/// @param chat_id The ID of the chat to get the chat object for.
///
/// Returns a chat object of the type dc_chat_t, must be freed using
/// [dc_chat_unref] when done.  On errors, NULL is returned.
#[no_mangle]
pub unsafe extern "C" fn dc_get_chat<'a>(
    context: *mut dc_context_t,
    chat_id: u32,
) -> *mut dc_chat_t<'a> {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_get_chat()");
        return ptr::null_mut();
    }
    let context_wrapper: &ContextWrapper = &*context;
    let maybe_chat_wrapper = ChatWrapper::try_new(context_wrapper.inner.read().unwrap(), |guard| {
        let context = guard.as_ref().expect("context not open");
        chat::Chat::load_from_db(context, chat_id)
    });
    match maybe_chat_wrapper {
        Ok(chat) => Box::into_raw(Box::new(chat)),
        Err(_) => ptr::null_mut(),
    }
}

/// Creates a new group chat.
///
/// After creation, the draft of the chat is set to a default text,
/// the group has one member with the ID DC_CONTACT_ID_SELF and is in
/// _unpromoted_ state.  This means, you can add or remove members,
/// change the name, the group image and so on without messages being
/// sent to all group members.
///
/// This changes as soon as the first message is sent to the group
/// members and the group becomes _promoted_.  After that, all changes
/// are synced with all group members by sending status message.
///
/// To check, if a chat is still unpromoted, you
/// [dc_chat_is_unpromoted].  This may be useful if you want to show
/// some help for just created groups.
///
/// @memberof [dc_context_t]
///
/// @param context The context as created by [dc_context_new].
/// @param verified If set to 1 the function creates a secure verified group.
///     Only secure-verified members are allowed in these groups
///     and end-to-end-encryption is always enabled.
/// @param chat_name The name of the group chat to create.
///     The name may be changed later using [dc_set_chat_name].
///     To find out the name of a group later, see [dc_chat_get_name]
///
/// Returns the chat ID of the new group chat, `0` on errors.
#[no_mangle]
pub unsafe extern "C" fn dc_create_group_chat(
    context: *mut dc_context_t,
    verified: libc::c_int,
    name: *mut libc::c_char,
) -> u32 {
    if context.is_null() || name.is_null() {
        eprintln!("ignoring careless call to dc_create_group_chat()");
        return 0;
    }
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    let verified = if let Some(s) = contact::VerifiedStatus::from_i32(verified) {
        s
    } else {
        return 0;
    };
    chat::create_group_chat(context, verified, as_str(name))
        .unwrap_or_log_default(context, "Failed to create group chat")
}

/// Checks if a given contact ID is a member of a group chat.
///
/// @memberof [dc_context_t]
///
/// @param context The context as created by [dc_context_new].
/// @param chat_id The chat ID to check.
/// @param contact_id The contact ID to check.  To check if yourself is member
///     of the chat, pass DC_CONTACT_ID_SELF (1) here.
///
/// Returns `1` if contact ID is member of chat ID, `0` if contact is
/// not in chat.
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
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    chat::is_contact_in_chat(context, chat_id, contact_id)
}

/// Adds a member to a group.
///
/// If the group is already _promoted_ (any message was sent to the
/// group), all group members are informed by a special status message
/// that is sent automatically by this function.
///
/// If the group is a verified group, only verified contacts can be
/// added to the group.
///
/// Sends out #DC_EVENT_CHAT_MODIFIED and #DC_EVENT_MSGS_CHANGED if a
/// status message was sent.
///
/// @memberof [dc_context_t]
///
/// @param context The context as created by dc_context_new().
/// @param chat_id The chat ID to add the contact to.  Must be a group chat.
/// @param contact_id The contact ID to add to the chat.
///
/// Returns `1` if the member added to group, `0` on error.
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
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    chat::add_contact_to_chat(context, chat_id, contact_id)
}

/// Removes a member from a group.
///
/// If the group is already _promoted_ (any message was sent to the
/// group), all group members are informed by a special status message
/// that is sent automatically by this function.
///
/// Sends out #DC_EVENT_CHAT_MODIFIED and #DC_EVENT_MSGS_CHANGED if a
/// status message was sent.
///
/// @memberof [dc_context_t]
///
/// @param context The context as created by [dc_context_new].
/// @param chat_id The chat ID to remove the contact from.  Must be a group chat.
/// @param contact_id The contact ID to remove from the chat.
///
/// Returns `1` if the member is removed from group, `0` on error.
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
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    chat::remove_contact_from_chat(context, chat_id, contact_id)
        .map(|_| 1)
        .unwrap_or_log_default(context, "Failed to remove contact")
}

/// Sets group name.
///
/// If the group is already _promoted_ (any message was sent to the
/// group), all group members are informed by a special status message
/// that is sent automatically by this function.
///
/// Sends out #DC_EVENT_CHAT_MODIFIED and #DC_EVENT_MSGS_CHANGED if a
/// status message was sent.
///
/// @memberof [dc_context_t]
///
/// @param chat_id The chat ID to set the name for.  Must be a group chat.
/// @param new_name New name of the group.
/// @param context The context as created by [dc_context_new].
///
/// Returns `1` on success, `0` on error.
#[no_mangle]
pub unsafe extern "C" fn dc_set_chat_name(
    context: *mut dc_context_t,
    chat_id: u32,
    name: *mut libc::c_char,
) -> libc::c_int {
    if context.is_null() || chat_id <= constants::DC_CHAT_ID_LAST_SPECIAL as u32 || name.is_null() {
        eprintln!("ignoring careless call to dc_set_chat_name()");
        return 0;
    }
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    chat::set_chat_name(context, chat_id, as_str(name))
        .map(|_| 1)
        .unwrap_or_log_default(context, "Failed to set chat name")
}

/// Sets group profile image.
///
/// If the group is already _promoted_ (any message was sent to the
/// group), all group members are informed by a special status message
/// that is sent automatically by this function.
///
/// Sends out #DC_EVENT_CHAT_MODIFIED and #DC_EVENT_MSGS_CHANGED if a
/// status message was sent.
///
/// To find out the profile image of a chat, use
/// [dc_chat_get_profile_image].
///
/// @memberof [dc_context_t]
///
/// @param context The context as created by [dc_context_new].
/// @param chat_id The chat ID to set the image for.
/// @param new_image Full path of the image to use as the group image.
///     If you pass NULL here, the group image is deleted (for
///     promoted groups, all members are informed about this change
///     anyway).
/// Returns `1` on success, `0` on error.
#[no_mangle]
pub unsafe extern "C" fn dc_set_chat_profile_image(
    context: *mut dc_context_t,
    chat_id: u32,
    image: *mut libc::c_char,
) -> libc::c_int {
    if context.is_null() || chat_id <= constants::DC_CHAT_ID_LAST_SPECIAL as u32 {
        eprintln!("ignoring careless call to dc_set_chat_profile_image()");
        return 0;
    }
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    chat::set_chat_profile_image(context, chat_id, as_str(image))
        .map(|_| 1)
        .unwrap_or_log_default(context, "Failed to set profile image")
}

/// Gets an informational text for a single message.
///
/// The text is multiline and may contain eg. the raw text of the
/// message.
///
/// The max. text returned is typically longer (about 100000
/// characters) than the max. text returned by [dc_msg_get_text]
/// (about 30000 characters).
///
/// @memberof [dc_context_t]
///
/// @param context The context object as created by [dc_context_new].
/// @param msg_id The message id for which information should be generated
///
/// Returns a text string, must be free()'d after usage.
#[no_mangle]
pub unsafe extern "C" fn dc_get_msg_info(
    context: *mut dc_context_t,
    msg_id: u32,
) -> *mut libc::c_char {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_get_msg_info()");
        return dc_strdup(ptr::null());
    }
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    message::dc_get_msg_info(context, msg_id)
}

/// Get the raw mime-headers of the given message.
///
/// Raw headers are saved for incoming messages only if
/// `dc_set_config(context, "save_mime_headers", "1")` was called
/// before.
///
/// @memberof [dc_context_t]
///
/// @param context The context object as created by [dc_context_new].
/// @param msg_id The message id, must be the id of an incoming message.
///
/// Returns raw headers as a multi-line string, must be free()'d after
/// usage.  Returns NULL if there are no headers saved for the given
/// message, eg. because of save_mime_headers is not set or the
/// message is not incoming.
#[no_mangle]
pub unsafe extern "C" fn dc_get_mime_headers(
    context: *mut dc_context_t,
    msg_id: u32,
) -> *mut libc::c_char {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_get_mime_headers()");
        return ptr::null_mut(); // NULL explicitly defined as "no mime headers"
    }
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    message::dc_get_mime_headers(context, msg_id)
}

/// Delete messages.
///
/// The messages are deleted on the current device and on the IMAP
/// server.
///
/// @memberof [dc_context_t]
///
/// @param context The context object as created by [dc_context_new]
/// @param msg_ids an array of uint32_t containing all message IDs that should be deleted
/// @param msg_cnt The number of messages IDs in the msg_ids array
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
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    message::dc_delete_msgs(context, msg_ids, msg_cnt)
}

/// Forward messages to another chat.
///
/// @memberof [dc_context_t]
///
/// @param context The context object as created by [dc_context_new]
/// @param msg_ids An array of uint32_t containing all message IDs
///     that should be forwarded
/// @param msg_cnt The number of messages IDs in the msg_ids array
/// @param chat_id The destination chat ID.
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
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    chat::forward_msgs(context, msg_ids, msg_cnt, chat_id)
}

/// Mark all messages sent by the given contact as _noticed_.
///
/// See also [dc_marknoticed_chat] and [dc_markseen_msgs]
///
/// Calling this function usually results in the event
/// #DC_EVENT_MSGS_CHANGED.
///
/// @memberof [dc_context_t]
///
/// @param context The context object as created by [dc_context_new]
/// @param contact_id The contact ID of which all messages should be
///     marked as noticed.
#[no_mangle]
pub unsafe extern "C" fn dc_marknoticed_contact(context: *mut dc_context_t, contact_id: u32) {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_marknoticed_contact()");
        return;
    }
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    Contact::mark_noticed(context, contact_id)
}

/// Marks a message as _seen_.
///
/// Marks a message as seen and updates the IMAP state and sends
/// MDNs. If the message is not in a real chat (eg. a contact
/// request), the message is only marked as NOTICED and no IMAP/MDNs
/// is done.  See also [dc_marknoticed_chat] and
/// [dc_marknoticed_contact]
///
/// @memberof [dc_context_t]
///
/// @param context The context object.
/// @param msg_ids An array of uint32_t containing all the messages
///     IDs that should be marked as seen.
/// @param msg_cnt The number of message IDs in msg_ids.
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
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    message::dc_markseen_msgs(context, msg_ids, msg_cnt as usize);
}

/// Star/unstar messages by setting the last parameter to 0 (unstar) or 1 (star).
///
/// Starred messages are collected in a virtual chat that can be shown
/// using [dc_get_chat_msgs] using the chat_id DC_CHAT_ID_STARRED.
///
/// @memberof [dc_context_t]
///
/// @param context The context object as created by [dc_context_new]
/// @param msg_ids An array of uint32_t message IDs defining the
///     messages to star or unstar
/// @param msg_cnt The number of IDs in msg_ids
/// @param star 0=unstar the messages in msg_ids, 1=star them
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
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    message::dc_star_msgs(context, msg_ids, msg_cnt, star);
}

/// Get a single message object of the type dc_msg_t.
///
/// For a list of messages in a chat, see [dc_get_chat_msgs]
/// For a list or chats, see [dc_get_chatlist]
///
/// @memberof [dc_context_t]
///
/// @param context The context as created by [dc_context_new].
/// @param msg_id The message ID for which the message object should be created.
///
/// Returns a dc_msg_t message object.  On errors, NULL is returned.
/// When done, the object must be freed using [dc_msg_unref].
#[no_mangle]
pub unsafe extern "C" fn dc_get_msg<'a>(
    context: *mut dc_context_t,
    msg_id: u32,
) -> *mut dc_msg_t<'a> {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_get_msg()");
        return ptr::null_mut();
    }
    let wrapper: &ContextWrapper = &*context;
    let maybe_msg = MessageWrapper::try_new(wrapper.inner.read().unwrap(), |guard| {
        let context = guard.as_ref().expect("context not open");
        message::dc_get_msg(context, msg_id)
    });
    match maybe_msg {
        Ok(msg) => Box::into_raw(Box::new(msg)),
        Err(_) => ptr::null_mut(), // TODO: log error
    }
}

/// Rough check if a string may be a valid e-mail address.
///
/// The function checks if the string contains a minimal amount of
/// characters before and after the `@` and `.` characters.
///
/// To check if a given address is a contact in the contact database
/// use [dc_lookup_contact_id_by_addr].
///
/// @memberof [dc_context_t]
///
/// @param addr The e-mail-address to check.
///
/// Returns `1` if the address may be a valid e-mail address, `0` if
/// address won't be a valid e-mail address.
#[no_mangle]
pub unsafe extern "C" fn dc_may_be_valid_addr(addr: *mut libc::c_char) -> libc::c_int {
    if addr.is_null() {
        eprintln!("ignoring careless call to dc_may_be_valid_addr()");
        return 0;
    }

    contact::may_be_valid_addr(as_str(addr)) as libc::c_int
}

/// Checks if an e-mail address belongs to a known and unblocked contact.
///
/// Known and unblocked contacts will be returned by
/// [dc_get_contacts].
///
/// To validate an e-mail address independently of the contact database
/// use [dc_may_be_valid_addr].
///
/// @memberof [dc_context_t]
///
/// @param context The context object as created by [dc_context_new].
/// @param addr The e-mail-address to check.
///
/// Returns `1` if address is a contact in use, `0` if address is not
/// a contact in use.
#[no_mangle]
pub unsafe extern "C" fn dc_lookup_contact_id_by_addr(
    context: *mut dc_context_t,
    addr: *mut libc::c_char,
) -> u32 {
    if context.is_null() || addr.is_null() {
        eprintln!("ignoring careless call to dc_lookup_contact_id_by_addr()");
        return 0;
    }
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    Contact::lookup_id_by_addr(context, as_str(addr))
}

/// Adds a single contact as a result of an _explicit_ user action.
///
/// We assume, the contact name, if any, is entered by the user and is
/// used "as is" therefore, [normalize] is _not_ called for the
/// name. If the contact is blocked, it is unblocked.
///
/// To add a number of contacts, see [dc_add_address_book] which is
/// much faster for adding a bunch of addresses.
///
/// May result in a #DC_EVENT_CONTACTS_CHANGED event.
///
/// @memberof [dc_context_t]
///
/// @param context The context object as created by [dc_context_new].
/// @param name Name of the contact to add. If you do not know the name belonging
///     to the address, you can give NULL here.
/// @param addr E-mail-address of the contact to add. If the email address
///     already exists, the name is updated and the origin is increased to
///     "manually created".
///
/// Returns a contact ID of the created or reused contact.
#[no_mangle]
pub unsafe extern "C" fn dc_create_contact(
    context: *mut dc_context_t,
    name: *mut libc::c_char,
    addr: *mut libc::c_char,
) -> u32 {
    if context.is_null() || addr.is_null() {
        eprintln!("ignoring careless call to dc_create_contact()");
        return 0;
    }
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    let name = if name.is_null() { "" } else { as_str(name) };
    match Contact::create(context, name, as_str(addr)) {
        Ok(id) => id,
        Err(_) => 0,
    }
}

/// Adds a number of contacts.
///
/// Typically used to add the whole address book from the OS. As names
/// here are typically not well formatted, we call normalize() for
/// each name given.
///
/// No email-address is added twice.  Trying to add email-addresses
/// that are already in the contact list, results in updating the name
/// unless the name was changed manually by the user.  If any
/// email-address or any name is really updated, the event
/// DC_EVENT_CONTACTS_CHANGED is sent.
///
/// To add a single contact entered by the user, you should prefer
/// [dc_create_contact], however, for adding a bunch of addresses,
/// this function is _much_ faster.
///
/// @memberof [dc_context_t]
///
/// @param context the context object as created by [dc_context_new].
/// @param adr_book A multi-line string in the format
///     `Name one\nAddress one\nName two\nAddress two`.
///      If an email address already exists, the name is updated
///      unless it was edited manually by [dc_create_contact] before.
///
/// Returns the number of modified or added contacts.
#[no_mangle]
pub unsafe extern "C" fn dc_add_address_book(
    context: *mut dc_context_t,
    addr_book: *mut libc::c_char,
) -> libc::c_int {
    if context.is_null() || addr_book.is_null() {
        eprintln!("ignoring careless call to dc_add_address_book()");
        return 0;
    }
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    match Contact::add_address_book(context, as_str(addr_book)) {
        Ok(cnt) => cnt as libc::c_int,
        Err(_) => 0,
    }
}

/// Returns known and unblocked contacts.
///
/// To get information about a single contact, see [dc_get_contact].
///
/// @memberof [dc_context_t]
///
/// @param context The context object as created by [dc_context_new].
/// @param listflags A combination of flags:
///     - if the flag DC_GCL_ADD_SELF is set, SELF is added to the
///       list unless filtered by other parameters
///     - if the flag DC_GCL_VERIFIED_ONLY is set, only verified
///       contacts are returned.  if DC_GCL_VERIFIED_ONLY is not set,
///       verified and unverified contacts are returned.
/// @param query A string to filter the list.  Typically used to implement an
///     incremental search.  NULL for no filtering.
///
/// Returns an array containing all contact IDs.  Must be
/// [dc_array_unref]'d after usage.
#[no_mangle]
pub unsafe extern "C" fn dc_get_contacts(
    context: *mut dc_context_t,
    flags: u32,
    query: *mut libc::c_char,
) -> *mut dc_array::dc_array_t {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_get_contacts()");
        return ptr::null_mut();
    }
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    let query = if query.is_null() {
        None
    } else {
        Some(as_str(query))
    };
    match Contact::get_all(context, flags, query) {
        Ok(contacts) => Box::into_raw(Box::new(dc_array_t::from(contacts))),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Gets the number of blocked contacts.
///
/// @memberof [dc_context_t]
///
/// @param context The context object as created by [dc_context_new].
///
/// Returns the number of blocked contacts.
#[no_mangle]
pub unsafe extern "C" fn dc_get_blocked_cnt(context: *mut dc_context_t) -> libc::c_int {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_get_blocked_cnt()");
        return 0;
    }
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    Contact::get_blocked_cnt(context) as libc::c_int
}

/// Get blocked contacts.
///
/// @memberof [dc_context_t]
///
/// @param context The context object as created by [dc_context_new].
///
/// Returns an array containing all blocked contact IDs.  Must be
/// [dc_array_unref]'d after usage.
#[no_mangle]
pub unsafe extern "C" fn dc_get_blocked_contacts(
    context: *mut dc_context_t,
) -> *mut dc_array::dc_array_t {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_get_blocked_contacts()");
        return ptr::null_mut();
    }
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    Box::into_raw(Box::new(dc_array_t::from(Contact::get_all_blocked(
        context,
    ))))
}

/// Block or unblock a contact.
///
/// May result in a #DC_EVENT_CONTACTS_CHANGED event.
///
/// @memberof [dc_context_t]
///
/// @param context The context object as created by [dc_context_new].
/// @param contact_id The ID of the contact to block or unblock.
/// @param new_blocking 1=block contact, 0=unblock contact
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
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    if block == 0 {
        Contact::unblock(context, contact_id);
    } else {
        Contact::block(context, contact_id);
    }
}

/// Gets encryption info for a contact.
///
/// Get a multi-line encryption info, containing your fingerprint and
/// the fingerprint of the contact, used eg. to compare the
/// fingerprints for a simple out-of-band verification.
///
/// @memberof [dc_context_t]
///
/// @param context The context object as created by [dc_context_new].
/// @param contact_id ID of the contact to get the encryption info for.
///
/// Returns multi-line text, must be free()'d after usage.
#[no_mangle]
pub unsafe extern "C" fn dc_get_contact_encrinfo(
    context: *mut dc_context_t,
    contact_id: u32,
) -> *mut libc::c_char {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_get_contact_encrinfo()");
        return dc_strdup(ptr::null());
    }
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    Contact::get_encrinfo(context, contact_id)
        .map(|s| s.strdup())
        .unwrap_or_else(|e| {
            error!(context, 0, "{}", e);
            std::ptr::null_mut()
        })
}

/// Deletes a contact.
///
/// The contact is deleted from the local device.  It may happen that
/// this is not possible as the contact is in use.  In this case, the
/// contact can be blocked.
///
/// May result in a #DC_EVENT_CONTACTS_CHANGED event.
///
/// @memberof [dc_context_t]
///
/// @param context The context object as created by [dc_context_new].
/// @param contact_id ID of the contact to delete.
///
/// Returns `1` on success, `0` on error.
#[no_mangle]
pub unsafe extern "C" fn dc_delete_contact(
    context: *mut dc_context_t,
    contact_id: u32,
) -> libc::c_int {
    if context.is_null() || contact_id <= constants::DC_CONTACT_ID_LAST_SPECIAL as u32 {
        eprintln!("ignoring careless call to dc_delete_contact()");
        return 0;
    }
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    match Contact::delete(context, contact_id) {
        Ok(_) => 1,
        Err(_) => 0,
    }
}

/// Get a single contact object.
///
/// For a list, see eg. [dc_get_contacts].
///
/// For contact DC_CONTACT_ID_SELF (1), the function returns sth.
/// like "Me" in the selected language and the email address defined
/// by [dc_set_config].
///
/// @memberof [dc_context_t]
///
/// @param context The context object as created by [dc_context_new].
/// @param contact_id ID of the contact to get the object for.
///
/// Returns the contact object, must be freed using [dc_contact_unref]
/// when no longer used.  NULL on errors.
#[no_mangle]
pub unsafe extern "C" fn dc_get_contact<'a>(
    context: *mut dc_context_t,
    contact_id: u32,
) -> *mut dc_contact_t<'a> {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_get_contact()");
        return ptr::null_mut();
    }
    let wrapper: &ContextWrapper = &*context;
    ContactWrapper::try_new(wrapper.inner.read().unwrap(), |guard| {
        let context = guard.as_ref().expect("context not open");
        Contact::get_by_id(context, contact_id)
    })
    .map(|contact| Box::into_raw(Box::new(contact)))
    .unwrap_or_else(|_| std::ptr::null_mut())
}

/// Import/export things.
///
/// For this purpose, the function creates a job that is executed in
/// the IMAP-thread then; this requires to call [dc_perform_imap_jobs]
/// regularly.
///
/// What to do is defined by the _what_ parameter which may be one of
/// the following:
///
/// - **DC_IMEX_EXPORT_BACKUP** (11) - Export a backup to the
///   directory given as `param1`.  The backup contains all contacts,
///   chats, images and other data and device independent settings.
///   The backup does not contain device dependent settings as
///   ringtones or LED notification settings.  The name of the backup
///   is typically `delta-chat.<day>.bak`, if more than one backup is
///   create on a day, the format is `delta-chat.<day>-<number>.bak`
///
/// - **DC_IMEX_IMPORT_BACKUP** (12) - `param1` is the file (not:
///    directory) to import. The file is normally created by
///    DC_IMEX_EXPORT_BACKUP and detected by
///    [dc_imex_has_backup]. Importing a backup is only possible as
///    long as the context is not configured or used in another way.
///
/// - **DC_IMEX_EXPORT_SELF_KEYS** (1) - Export all private keys and
///   all public keys of the user to the directory given as `param1`.
///   The default key is written to the files `public-key-default.asc`
///   and `private-key-default.asc`, if there are more keys, they are
///   written to files as `public-key-<id>.asc` and
///   `private-key-<id>.asc`
///
/// - **DC_IMEX_IMPORT_SELF_KEYS** (2) - Import private keys found in
///   the directory given as `param1`.  The last imported key is made
///   the default keys unless its name contains the string `legacy`.
///   Public keys are not imported.
///
/// While dc_imex() returns immediately, the started job may take a while,
/// you can stop it using dc_stop_ongoing_process(). During execution of the job,
/// some events are sent out:
///
/// - A number of #DC_EVENT_IMEX_PROGRESS events are sent and may be
///   used to create a progress bar or stuff like that. Moreover,
///   you'll be informed when the imex-job is done.
///
/// - For each file written on export, the function sends
///   #DC_EVENT_IMEX_FILE_WRITTEN
///
/// Only one import-/export-progress can run at the same time.  To
/// cancel an import-/export-progress, use [dc_stop_ongoing_process].
///
/// @memberof [dc_context_t]
///
/// @param context The context as created by [dc_context_new].
/// @param what One of the DC_IMEX_* constants.
/// @param param1 Meaning depends on the DC_IMEX_* constants. If this
///     parameter is a directory, it should not end with a slash
///     (otherwise you'll get double slashes when receiving
///     #DC_EVENT_IMEX_FILE_WRITTEN). Set to NULL if not used.
/// @param param2 Meaning depends on the DC_IMEX_* constants. Set to
///     NULL if not used.
#[no_mangle]
pub unsafe extern "C" fn dc_imex(
    context: *mut dc_context_t,
    what: libc::c_int,
    param1: *mut libc::c_char,
    param2: *mut libc::c_char,
) {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_imex()");
        return;
    }
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    dc_imex::dc_imex(context, what, param1, param2)
}

/// Check if there is a backup file.
///
/// May only be used on fresh installations (eg. [dc_is_configured]
/// returns 0).
///
/// # Example
///
/// ```c
/// char dir[] = "/dir/to/search/backups/in";
///
/// void ask_user_for_credentials()
/// {
///     // - ask the user for email and password
///     // - save them using dc_set_config()
/// }
///
/// int ask_user_whether_to_import()
/// {
///     // - inform the user that we've found a backup
///     // - ask if he want to import it
///     // - return 1 to import, 0 to skip
///     return 1;
/// }
///
/// if (!dc_is_configured(context))
/// {
///     char* file = NULL;
///     if ((file=dc_imex_has_backup(context, dir))!=NULL && ask_user_whether_to_import())
///     {
///         dc_imex(context, DC_IMEX_IMPORT_BACKUP, file, NULL);
///         // connect
///     }
///     else
///     {
///         do {
///             ask_user_for_credentials();
///         }
///         while (!configure_succeeded())
///     }
///     free(file);
/// }
/// ```
///
/// @memberof [dc_context_t]
///
/// @param context The context as created by [dc_context_new].
/// @param dir_name Directory to search backups in.
///
/// Returns a string with the backup file, typically given to
/// [dc_imex], returned strings must be free()'d.  The function
/// returns NULL if no backup was found.
#[no_mangle]
pub unsafe extern "C" fn dc_imex_has_backup(
    context: *mut dc_context_t,
    dir: *mut libc::c_char,
) -> *mut libc::c_char {
    if context.is_null() || dir.is_null() {
        eprintln!("ignoring careless call to dc_imex_has_backup()");
        return ptr::null_mut(); // NULL explicitly defined as "has no backup"
    }
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    dc_imex::dc_imex_has_backup(context, dir)
}

/// Initiate Autocrypt Setup Transfer.
///
/// Before starting the setup transfer with this function, the user
/// should be asked:
///
/// ```text

/// "An 'Autocrypt Setup Message' securely shares your end-to-end
/// setup with other Autocrypt-compliant apps.  The setup will be
/// encrypted by a setup code which is displayed here and must be
/// typed on the other device.
/// ```
///
/// After that, this function should be called to send the Autocrypt
/// Setup Message.  The function creates the setup message and waits
/// until it is really sent.  As this may take a while, it is
/// recommended to start the function in a separate thread; to
/// interrupt it, you can use [dc_stop_ongoing_process].
///
/// After everything succeeded, the required setup code is returned in
/// the following format:
///
/// ```text
/// 1234-1234-1234-1234-1234-1234-1234-1234-1234
/// ```
///
/// The setup code should be shown to the user then:
///
/// ```text
/// Your key has been sent to yourself. Switch to the other device
/// and open the setup message. You should be prompted for a setup
/// code. Type the following digits into the prompt:
///
/// 1234 - 1234 - 1234 -
/// 1234 - 1234 - 1234 -
/// 1234 - 1234 - 1234
///
/// Once you're done, your other device will be ready to use Autocrypt.
/// ```
///
/// On the _other device_ you will call [dc_continue_key_transfer]
/// then for setup messages identified by [dc_msg_is_setupmessage].
///
/// For more details about the Autocrypt setup process, please refer to
/// https://autocrypt.org/en/latest/level1.html#autocrypt-setup-message
///
/// @memberof [dc_context_t]
/// @param context The context object.
///
/// Returns the setup code. Must be free()'d after usage.  On errors,
/// eg. if the message could not be sent, NULL is returned.
#[no_mangle]
pub unsafe extern "C" fn dc_initiate_key_transfer(context: *mut dc_context_t) -> *mut libc::c_char {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_initiate_key_transfer()");
        return ptr::null_mut(); // NULL explicitly defined as "error"
    }
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    dc_imex::dc_initiate_key_transfer(context)
}

/// Continue the Autocrypt Key Transfer on another device.
///
/// If you have started the key transfer on another device using
/// [dc_initiate_key_transfer] and you've detected a setup message
/// with [dc_msg_is_setupmessage], you should prompt the user for the
/// setup code and call this function then.
///
/// You can use [dc_msg_get_setupcodebegin] to give the user a hint
/// about the code (useful if the user has created several messages
/// and should not enter the wrong code).
///
/// @memberof [dc_context_t]
///
/// @param context The context object.
/// @param msg_id ID of the setup message to decrypt.
/// @param setup_code Setup code entered by the user. This is the same
///     setup code as returned from dc_initiate_key_transfer() on the
///     other device.  There is no need to format the string
///     correctly, the function will remove all spaces and other
///     characters and insert the `-` characters at the correct
///     places.
///
/// Returns `1` if the key successfully decrypted and imported; both
/// devices will use the same key now; `0` if key transfer failed
/// eg. due to a bad setup code.
#[no_mangle]
pub unsafe extern "C" fn dc_continue_key_transfer(
    context: *mut dc_context_t,
    msg_id: u32,
    setup_code: *mut libc::c_char,
) -> libc::c_int {
    if context.is_null()
        || msg_id <= constants::DC_MSG_ID_LAST_SPECIAL as u32
        || setup_code.is_null()
    {
        eprintln!("ignoring careless call to dc_continue_key_transfer()");
        return 0;
    }
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    dc_imex::dc_continue_key_transfer(context, msg_id, setup_code)
}

/// Signal an ongoing process to stop.
///
/// After that, [dc_stop_ongoing_process] returns _without_ waiting
/// for the ongoing process to return.
///
/// The ongoing process will return ASAP then, however, it may
/// still take a moment.  If in doubt, the caller may also decide to kill the
/// thread after a few seconds; eg. the process may hang in a
/// function not under the control of the core (eg. #DC_EVENT_HTTP_GET). Another
/// reason for [dc_stop_ongoing_process] not to wait is that otherwise it
/// would be GUI-blocking and should be started in another thread then; this
/// would make things even more complicated.
///
/// Typical ongoing processes are started by [dc_configure],
/// [dc_initiate_key_transfer] or [dc_imex]. As there is always at
/// most only one onging process at the same time, there is no need to
/// define _which_ process to exit.
///
/// @memberof [dc_context_t]
///
/// # Parameters
///
/// * `context` - The [dc_context_t] object.
///
/// # Panics
///
/// It is safe to call this function on a closed context, which is a
/// no-op.
#[no_mangle]
pub unsafe extern "C" fn dc_stop_ongoing_process(context: *mut dc_context_t) {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_stop_ongoing_process()");
        return;
    }
    let ffi_ctx: &ContextWrapper = &*context;
    match ffi_ctx.inner.read().unwrap().as_ref() {
        Some(ref ctx) => configure::dc_stop_ongoing_process(ctx),
        None => (),
    };
}

/// Check a scanned QR code.
///
/// The function should be called after a QR code is scanned.  The
/// function takes the raw text scanned and checks what can be done
/// with it.
///
/// The QR code state is returned in dc_lot_t::state as:
///
/// - DC_QR_ASK_VERIFYCONTACT with dc_lot_t::id=Contact ID
/// - DC_QR_ASK_VERIFYGROUP withdc_lot_t::text1=Group name
/// - DC_QR_FPR_OK with dc_lot_t::id=Contact ID
/// - DC_QR_FPR_MISMATCH with dc_lot_t::id=Contact ID
/// - DC_QR_FPR_WITHOUT_ADDR with dc_lot_t::test1=Formatted fingerprint
/// - DC_QR_ADDR with dc_lot_t::id=Contact ID
/// - DC_QR_TEXT with dc_lot_t::text1=Text
/// - DC_QR_URL with dc_lot_t::text1=URL
/// - DC_QR_ERROR with dc_lot_t::text1=Error string
///
/// @memberof [dc_context_t]
/// @param context The context object.
/// @param qr The text of the scanned QR code.
///
/// Returns the parsed QR code as a [dc_lot_t] object. The returned
/// object must be freed using [dc_lot_unref] after usage.
#[no_mangle]
pub unsafe extern "C" fn dc_check_qr(
    context: *mut dc_context_t,
    qr: *mut libc::c_char,
) -> *mut dc_lot_t {
    if context.is_null() || qr.is_null() {
        eprintln!("ignoring careless call to dc_check_qr()");
        return ptr::null_mut();
    }
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    let lot = qr::check_qr(context, as_str(qr));
    Box::into_raw(Box::new(lot))
}

/// Get QR code text that will offer an secure-join verification.
///
/// The QR code is compatible to the OPENPGP4FPR format so that a
/// basic fingerprint comparison also works eg. with OpenKeychain.
///
/// The scanning device will pass the scanned content to [dc_check_qr] then;
/// if this function returns DC_QR_ASK_VERIFYCONTACT or DC_QR_ASK_VERIFYGROUP
/// an out-of-band-verification can be joined using [dc_join_securejoin].
///
/// @memberof [dc_context_t]
///
/// @param context The context object.
/// @param group_chat_id If set to a group-chat-id,
///     the group-join-protocol is offered in the QR code;
///     works for verified groups as well as for normal groups.
///     If set to 0, the setup-Verified-contact-protocol is offered in the QR code.
///
/// Returns text that should go to the QR code, On errors, an empty QR
/// code is returned, NULL is never returned.  The returned string
/// must be free()'d after usage.
#[no_mangle]
pub unsafe extern "C" fn dc_get_securejoin_qr(
    context: *mut dc_context_t,
    chat_id: u32,
) -> *mut libc::c_char {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_get_securejoin_qr()");
        return "".strdup();
    }
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    dc_securejoin::dc_get_securejoin_qr(context, chat_id)
}

/// Join an out-of-band-verification initiated on another device with
/// [dc_get_securejoin_qr].
///
/// This function is typically called when dc_check_qr() returns
/// lot.state=DC_QR_ASK_VERIFYCONTACT or
/// lot.state=DC_QR_ASK_VERIFYGROUP.
///
/// This function takes some time and sends and receives several messages.
/// You should call it in a separate thread; if you want to abort it, you should
/// call [dc_stop_ongoing_process].
///
/// @memberof [dc_context_t]
///
/// @param context The context object
/// @param qr The text of the scanned QR code. Typically, the same string as given
///     to dc_check_qr().
///
/// Returns the chat-id of the joined chat, the UI may redirect to the
/// this chat.  If the out-of-band verification failed or was aborted,
/// `0` is returned.
#[no_mangle]
pub unsafe extern "C" fn dc_join_securejoin(
    context: *mut dc_context_t,
    qr: *mut libc::c_char,
) -> u32 {
    if context.is_null() || qr.is_null() {
        eprintln!("ignoring careless call to dc_join_securejoin()");
        return 0;
    }
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    dc_securejoin::dc_join_securejoin(context, qr)
}

/// Enable or disable location streaming for a chat.
///
/// Locations are sent to all members of the chat for the given number
/// of seconds; after that, location streaming is automatically
/// disabled for the chat.  The current location streaming state of a
/// chat can be checked using [dc_is_sending_locations_to_chat].
///
/// The locations that should be sent to the chat can be set using
/// [dc_set_location].
///
/// @memberof [dc_context_t]
///
/// @param context The context object.
/// @param chat_id Chat id to enable location streaming for.
/// @param seconds >0: enable location streaming for the given number of seconds;
///     0: disable location streaming.
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
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    dc_location::dc_send_locations_to_chat(context, chat_id, seconds as i64)
}

/// Check if location streaming is enabled.
///
/// Location stream can be enabled or disabled using
/// [dc_send_locations_to_chat].  If you have already a [dc_chat_t]
/// object, [dc_chat_is_sending_locations] may be more handy.
///
/// @memberof [dc_context_t]
///
/// @param context The context object.
/// @param chat_id >0: Check if location streaming is enabled for the given chat.
///     0: Check of location streaming is enabled for any chat.
///
/// Returns `1`: location streaming is enabled for the given chat(s);
/// `0`: location streaming is disabled for the given chat(s).
#[no_mangle]
pub unsafe extern "C" fn dc_is_sending_locations_to_chat(
    context: *mut dc_context_t,
    chat_id: u32,
) -> libc::c_int {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_is_sending_locations_to_chat()");
        return 0;
    }
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    dc_location::dc_is_sending_locations_to_chat(context, chat_id) as libc::c_int
}

/// Sets current location.
///
/// The location is sent to all chats where location streaming is
/// enabled using [dc_send_locations_to_chat].
///
/// Typically results in the event #DC_EVENT_LOCATION_CHANGED with
/// contact_id set to DC_CONTACT_ID_SELF.
///
/// The UI should call this function on all location changes.  The
/// locations set by this function are not sent immediately, instead a
/// message with the last locations is sent out every some minutes or
/// when the user sends out a normal message, the last locations are
/// attached.
///
/// @memberof [dc_context_t]
///
/// @param context The context object.
/// @param latitude North-south position of the location.
///     Set to 0.0 if the latitude is not known.
/// @param longitude East-west position of the location.
///     Set to 0.0 if the longitude is not known.
/// @param accuracy Estimated accuracy of the location, radial, in meters.
///     Set to 0.0 if the accuracy is not known.
///
/// Returns `1`: location streaming is still enabled for at least one
/// chat, this dc_set_location() should be called as soon as the
/// location changes; `0`: location streaming is no longer needed,
/// [dc_is_sending_locations_to_chat] is false for all chats.
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
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    dc_location::dc_set_location(context, latitude, longitude, accuracy)
}

/// Get shared locations from the database.
///
/// The locations can be filtered by the chat-id, the contact-id and
/// by a timespan.
///
/// The number of returned locations can be retrieved using
/// [dc_array_get_cnt].  To get information for each location, use
/// [dc_array_get_latitude], [dc_array_get_longitude],
/// [dc_array_get_accuracy], [dc_array_get_timestamp],
/// [dc_array_get_contact_id] and [dc_array_get_msg_id].  The latter
/// returns 0 if there is no message bound to the location.
///
/// Note that only if [dc_array_is_independent] returns 0, the
/// location is the current or a past position of the user.  If
/// [dc_array_is_independent] returns 1, the location is any location
/// on earth that is marked by the user.
///
/// @memberof [dc_context_t]
///
/// @param context The context object.
/// @param chat_id Chat-id to get location information for.
///     0 to get locations independently of the chat.
/// @param contact_id Contact-id to get location information for.
///     If also a chat-id is given, this should be a member of the given chat.
///     0 to get locations independently of the contact.
/// @param timestamp_from Start of timespan to return.
///     Must be given in number of seconds since 00:00 hours, Jan 1, 1970 UTC.
///     0 for "start from the beginning".
/// @param timestamp_to End of timespan to return.
///     Must be given in number of seconds since 00:00 hours, Jan 1, 1970 UTC.
///     0 for "all up to now".
///
/// Returns an array of locations, NULL is never returned.  The array
/// is sorted decending; the first entry in the array is the location
/// with the newest timestamp.  Note that this is only realated to the
/// recent postion of the user if dc_array_is_independent() returns
/// `0`.  The returned array must be freed using [dc_array_unref].
///
/// # Example
///
/// ```c
/// // get locations from the last hour for a global map
/// dc_array_t* loc = dc_get_locations(context, 0, 0, time(NULL)-60*60, 0);
/// for (int i=0; i<dc_array_get_cnt(); i++) {
///     double lat = dc_array_get_latitude(loc, i);
///     ...
/// }
/// dc_array_unref(loc);
///
/// // get locations from a contact for a global map
/// dc_array_t* loc = dc_get_locations(context, 0, contact_id, 0, 0);
/// ...
///
/// // get all locations known for a given chat
/// dc_array_t* loc = dc_get_locations(context, chat_id, 0, 0, 0);
/// ...
///
/// // get locations from a single contact for a given chat
/// dc_array_t* loc = dc_get_locations(context, chat_id, contact_id, 0, 0);
/// ...
/// ```
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
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    let res = dc_location::dc_get_locations(
        context,
        chat_id,
        contact_id,
        timestamp_begin as i64,
        timestamp_end as i64,
    );
    Box::into_raw(Box::new(dc_array_t::from(res)))
}

/// Delete all locations on the current device.
///
/// Locations already sent cannot be deleted.
///
/// Typically results in the event #DC_EVENT_LOCATION_CHANGED with
/// contact_id set to 0.
///
/// @memberof [dc_context_t]
///
/// @param context The context object.
#[no_mangle]
pub unsafe extern "C" fn dc_delete_all_locations(context: *mut dc_context_t) {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_delete_all_locations()");
        return;
    }
    let wrapper: &ContextWrapper = &*context;
    let inner_guard = wrapper.inner.read().unwrap();
    let context = inner_guard.as_ref().expect("context not open");
    dc_location::dc_delete_all_locations(context);
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
pub unsafe extern "C" fn dc_array_add_id(array: *mut dc_array_t, item: libc::c_uint) {
    if array.is_null() {
        eprintln!("ignoring careless call to dc_array_add_id()");
        return;
    }

    (*array).add_id(item);
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

    (*array).get_location(index).chat_id
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

    let list = &*chatlist;
    list.rent(|l| l.len() as libc::size_t)
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

    let list = &*chatlist;
    list.rent(|l| l.get_chat_id(index as usize))
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

    let list = &*chatlist;
    list.rent(|l| l.get_msg_id(index as usize))
}

#[no_mangle]
pub unsafe extern "C" fn dc_chatlist_get_summary<'a>(
    chatlist: *mut dc_chatlist_t<'a>,
    index: libc::size_t,
    chat: *mut dc_chat_t<'a>,
) -> *mut dc_lot_t {
    if chatlist.is_null() {
        eprintln!("ignoring careless call to dc_chatlist_get_summary()");
        return ptr::null_mut();
    }
    let ffi_list = &*chatlist;
    let lot = ffi_list.rent(|l| {
        if chat.is_null() {
            l.get_summary(index as usize, None)
        } else {
            let ffi_chat = &*chat;
            ffi_chat.rent(|c| l.get_summary(index as usize, Some(c)))
        }
    });
    Box::into_raw(Box::new(lot))
}

// On the C FFI the context is actually ContextWrapper.  This struct
// is stored as userdata on the Rust Context object so that it can be
// retrieved here.
#[no_mangle]
pub unsafe extern "C" fn dc_chatlist_get_context(
    chatlist: *mut dc_chatlist_t,
) -> *const dc_context_t {
    if chatlist.is_null() {
        eprintln!("ignoring careless call to dc_chatlist_get_context()");
        return ptr::null_mut();
    }

    let list = &*chatlist;
    list.rent(|l| {
        let context: &Context = l.get_context();
        let userdata_ptr = context.userdata as *const ContextWrapper;
        let wrapper: &ContextWrapper = &*userdata_ptr;
        wrapper
    })
}

// dc_chat_t

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

    let chat = &*chat;

    chat.rent(|c| c.get_id())
}

#[no_mangle]
pub unsafe extern "C" fn dc_chat_get_type(chat: *mut dc_chat_t) -> libc::c_int {
    if chat.is_null() {
        eprintln!("ignoring careless call to dc_chat_get_type()");
        return 0;
    }

    let chat = &*chat;

    chat.rent(|c| c.get_type() as libc::c_int)
}

#[no_mangle]
pub unsafe extern "C" fn dc_chat_get_name(chat: *mut dc_chat_t) -> *mut libc::c_char {
    if chat.is_null() {
        eprintln!("ignoring careless call to dc_chat_get_name()");
        return dc_strdup(ptr::null());
    }

    let chat = &*chat;

    chat.rent(|c| c.get_name().strdup())
}

#[no_mangle]
pub unsafe extern "C" fn dc_chat_get_subtitle(chat: *mut dc_chat_t) -> *mut libc::c_char {
    if chat.is_null() {
        eprintln!("ignoring careless call to dc_chat_get_subtitle()");
        return dc_strdup(ptr::null());
    }

    let chat = &*chat;

    chat.rent(|c| c.get_subtitle().strdup())
}

#[no_mangle]
pub unsafe extern "C" fn dc_chat_get_profile_image(chat: *mut dc_chat_t) -> *mut libc::c_char {
    if chat.is_null() {
        eprintln!("ignoring careless call to dc_chat_get_profile_image()");
        return ptr::null_mut(); // NULL explicitly defined as "no image"
    }

    let chat = &*chat;

    match chat.rent(|c| c.get_profile_image()) {
        Some(i) => i.strdup(),
        None => ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn dc_chat_get_color(chat: *mut dc_chat_t) -> u32 {
    if chat.is_null() {
        eprintln!("ignoring careless call to dc_chat_get_color()");
        return 0;
    }

    let chat = &*chat;

    chat.rent(|c| c.get_color())
}

#[no_mangle]
pub unsafe extern "C" fn dc_chat_get_archived(chat: *mut dc_chat_t) -> libc::c_int {
    if chat.is_null() {
        eprintln!("ignoring careless call to dc_chat_get_archived()");
        return 0;
    }

    let chat = &*chat;

    chat.rent(|c| c.is_archived() as libc::c_int)
}

#[no_mangle]
pub unsafe extern "C" fn dc_chat_is_unpromoted(chat: *mut dc_chat_t) -> libc::c_int {
    if chat.is_null() {
        eprintln!("ignoring careless call to dc_chat_is_unpromoted()");
        return 0;
    }

    let chat = &*chat;

    chat.rent(|c| c.is_unpromoted() as libc::c_int)
}

#[no_mangle]
pub unsafe extern "C" fn dc_chat_is_self_talk(chat: *mut dc_chat_t) -> libc::c_int {
    if chat.is_null() {
        eprintln!("ignoring careless call to dc_chat_is_self_talk()");
        return 0;
    }

    let chat = &*chat;

    chat.rent(|c| c.is_self_talk() as libc::c_int)
}

#[no_mangle]
pub unsafe extern "C" fn dc_chat_is_verified(chat: *mut dc_chat_t) -> libc::c_int {
    if chat.is_null() {
        eprintln!("ignoring careless call to dc_chat_is_verified()");
        return 0;
    }

    let chat = &*chat;

    chat.rent(|c| c.is_verified() as libc::c_int)
}

#[no_mangle]
pub unsafe extern "C" fn dc_chat_is_sending_locations(chat: *mut dc_chat_t) -> libc::c_int {
    if chat.is_null() {
        eprintln!("ignoring careless call to dc_chat_is_sending_locations()");
        return 0;
    }

    let chat = &*chat;

    chat.rent(|c| c.is_sending_locations() as libc::c_int)
}

// dc_msg_t

#[no_mangle]
pub unsafe extern "C" fn dc_msg_new<'a>(
    context: *mut dc_context_t,
    viewtype: libc::c_int,
) -> *mut dc_msg_t<'a> {
    if context.is_null() {
        eprintln!("ignoring careless call to dc_msg_new()");
        return ptr::null_mut();
    }
    let ctx_wrapper: &ContextWrapper = &*context;
    let msg_wrapper = MessageWrapper::new(ctx_wrapper.inner.read().unwrap(), |inner_guard| {
        let context = inner_guard.as_ref().expect("context not open");
        let viewtype = from_prim(viewtype).expect(&format!("invalid viewtype = {}", viewtype));
        message::dc_msg_new(context, viewtype)
    });
    Box::into_raw(Box::new(msg_wrapper))
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

    let msg = &*msg;
    msg.rent(|m| message::dc_msg_get_id(m))
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_from_id(msg: *mut dc_msg_t) -> u32 {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_get_from_id()");
        return 0;
    }

    let msg = &*msg;
    msg.rent(|m| message::dc_msg_get_from_id(m))
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_chat_id(msg: *mut dc_msg_t) -> u32 {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_get_chat_id()");
        return 0;
    }

    let msg = &*msg;
    msg.rent(|m| message::dc_msg_get_chat_id(m))
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_viewtype(msg: *mut dc_msg_t) -> libc::c_int {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_get_viewtype()");
        return 0;
    }

    let msg = &*msg;
    msg.rent(|m| {
        message::dc_msg_get_viewtype(m)
            .to_i64()
            .expect("impossible: Viewtype -> i64 conversion failed") as libc::c_int
    })
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_state(msg: *mut dc_msg_t) -> libc::c_int {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_get_state()");
        return 0;
    }

    let msg = &*msg;
    msg.rent(|m| message::dc_msg_get_state(m) as libc::c_int)
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_timestamp(msg: *mut dc_msg_t) -> i64 {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_get_received_timestamp()");
        return 0;
    }

    let msg = &*msg;
    msg.rent(|m| message::dc_msg_get_timestamp(m))
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_received_timestamp(msg: *mut dc_msg_t) -> i64 {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_get_received_timestamp()");
        return 0;
    }

    let msg = &*msg;
    msg.rent(|m| message::dc_msg_get_received_timestamp(m))
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_sort_timestamp(msg: *mut dc_msg_t) -> i64 {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_get_sort_timestamp()");
        return 0;
    }

    let msg = &*msg;
    msg.rent(|m| message::dc_msg_get_sort_timestamp(m))
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_text(msg: *mut dc_msg_t) -> *mut libc::c_char {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_get_text()");
        return dc_strdup(ptr::null());
    }

    let msg = &*msg;
    msg.rent(|m| message::dc_msg_get_text(m))
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_file(msg: *mut dc_msg_t) -> *mut libc::c_char {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_get_file()");
        return dc_strdup(ptr::null());
    }

    let msg = &*msg;
    msg.rent(|m| message::dc_msg_get_file(m))
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_filename(msg: *mut dc_msg_t) -> *mut libc::c_char {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_get_filename()");
        return dc_strdup(ptr::null());
    }

    let msg = &*msg;
    msg.rent(|m| message::dc_msg_get_filename(m))
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_filemime(msg: *mut dc_msg_t) -> *mut libc::c_char {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_get_filemime()");
        return dc_strdup(ptr::null());
    }

    let msg = &*msg;
    msg.rent(|m| message::dc_msg_get_filemime(m))
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_filebytes(msg: *mut dc_msg_t) -> u64 {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_get_filebytes()");
        return 0;
    }

    let msg = &*msg;
    msg.rent(|m| message::dc_msg_get_filebytes(m))
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_width(msg: *mut dc_msg_t) -> libc::c_int {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_get_width()");
        return 0;
    }

    let msg = &*msg;
    msg.rent(|m| message::dc_msg_get_width(m))
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_height(msg: *mut dc_msg_t) -> libc::c_int {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_get_height()");
        return 0;
    }

    let msg = &*msg;
    msg.rent(|m| message::dc_msg_get_height(m))
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_duration(msg: *mut dc_msg_t) -> libc::c_int {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_get_duration()");
        return 0;
    }

    let msg = &*msg;
    msg.rent(|m| message::dc_msg_get_duration(m))
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_showpadlock(msg: *mut dc_msg_t) -> libc::c_int {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_get_showpadlock()");
        return 0;
    }

    let msg = &*msg;
    msg.rent(|m| message::dc_msg_get_showpadlock(m))
}

// TODO: how does this work?
#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_summary<'a>(
    msg: *mut dc_msg_t<'a>,
    chat: *mut dc_chat_t<'a>,
) -> *mut dc_lot_t {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_get_summary()");
        return ptr::null_mut();
    }
    let ffi_msg = &mut *msg;
    // let lot = ffi_msg.rent_mut(|m| {
    //     if chat.is_null() {
    //         message::dc_msg_get_summary(m, None)
    //     } else {
    //         let ffi_chat = &*chat;
    //         ffi_chat.rent(|c| message::dc_msg_get_summary(m, Some(c)))
    //     }
    // });
    let lot = ffi_msg.rent_mut(|m| message::dc_msg_get_summary(m, None));
    Box::into_raw(Box::new(lot))
}

// fn msg_get_summary<'a>(msg: &'a MessageWrapper, chat: Option<&chat::Chat>) -> lot::Lot {
//     msg.rent_mut(|m: &'a mut message::Message<'a>| message::dc_msg_get_summary(m, chat))
// }

#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_summarytext(
    msg: *mut dc_msg_t,
    approx_characters: libc::c_int,
) -> *mut libc::c_char {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_get_summarytext()");
        return dc_strdup(ptr::null());
    }

    let msg = &mut *msg;
    msg.rent_mut(|m| message::dc_msg_get_summarytext(m, approx_characters.try_into().unwrap()))
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_has_deviating_timestamp(msg: *mut dc_msg_t) -> libc::c_int {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_has_deviating_timestamp()");
        return 0;
    }

    let msg = &*msg;
    msg.rent(|m| message::dc_msg_has_deviating_timestamp(m))
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_has_location(msg: *mut dc_msg_t) -> libc::c_int {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_has_location()");
        return 0;
    }

    let msg = &*msg;
    msg.rent(|m| message::dc_msg_has_location(m) as libc::c_int)
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_is_sent(msg: *mut dc_msg_t) -> libc::c_int {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_is_sent()");
        return 0;
    }

    let msg = &*msg;
    msg.rent(|m| message::dc_msg_is_sent(m))
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_is_starred(msg: *mut dc_msg_t) -> libc::c_int {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_is_starred()");
        return 0;
    }

    let msg = &*msg;
    msg.rent(|m| message::dc_msg_is_starred(m).into())
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_is_forwarded(msg: *mut dc_msg_t) -> libc::c_int {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_is_forwarded()");
        return 0;
    }

    let msg = &*msg;
    msg.rent(|m| message::dc_msg_is_forwarded(m))
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_is_info(msg: *mut dc_msg_t) -> libc::c_int {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_is_info()");
        return 0;
    }

    let msg = &*msg;
    msg.rent(|m| message::dc_msg_is_info(m))
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_is_increation(msg: *mut dc_msg_t) -> libc::c_int {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_is_increation()");
        return 0;
    }

    let msg = &*msg;
    msg.rent(|m| message::dc_msg_is_increation(m))
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_is_setupmessage(msg: *mut dc_msg_t) -> libc::c_int {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_is_setupmessage()");
        return 0;
    }

    let msg = &*msg;
    msg.rent(|m| message::dc_msg_is_setupmessage(m) as libc::c_int)
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_get_setupcodebegin(msg: *mut dc_msg_t) -> *mut libc::c_char {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_get_setupcodebegin()");
        return dc_strdup(ptr::null());
    }

    let msg = &*msg;
    msg.rent(|m| message::dc_msg_get_setupcodebegin(m))
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_set_text(msg: *mut dc_msg_t, text: *mut libc::c_char) {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_set_text()");
        return;
    }

    let msg = &mut *msg;
    // TODO: {text} equal to NULL is treated as "", which is strange. Does anyone rely on it?
    msg.rent_mut(|m| message::dc_msg_set_text(m, text))
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_set_file(
    msg: *mut dc_msg_t,
    file: *mut libc::c_char,
    filemime: *mut libc::c_char,
) {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_set_file()");
        return;
    }

    let msg = &mut *msg;
    msg.rent_mut(|m| message::dc_msg_set_file(m, file, filemime))
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

    let msg = &mut *msg;
    msg.rent_mut(|m| message::dc_msg_set_dimension(m, width, height))
}

#[no_mangle]
pub unsafe extern "C" fn dc_msg_set_duration(msg: *mut dc_msg_t, duration: libc::c_int) {
    if msg.is_null() {
        eprintln!("ignoring careless call to dc_msg_set_duration()");
        return;
    }

    let msg = &mut *msg;
    msg.rent_mut(|m| message::dc_msg_set_duration(m, duration))
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

    let msg = &mut *msg;
    msg.rent_mut(|m| message::dc_msg_set_location(m, latitude, longitude))
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

    let msg = &mut *msg;
    msg.rent_mut(|m| message::dc_msg_latefiling_mediasize(m, width, height, duration))
}

// dc_contact_t

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

    let contact = &*contact;

    contact.rent(|c| c.get_id())
}

#[no_mangle]
pub unsafe extern "C" fn dc_contact_get_addr(contact: *mut dc_contact_t) -> *mut libc::c_char {
    if contact.is_null() {
        eprintln!("ignoring careless call to dc_contact_get_addr()");
        return dc_strdup(ptr::null());
    }

    let contact = &*contact;

    contact.rent(|c| c.get_addr().strdup())
}

#[no_mangle]
pub unsafe extern "C" fn dc_contact_get_name(contact: *mut dc_contact_t) -> *mut libc::c_char {
    if contact.is_null() {
        eprintln!("ignoring careless call to dc_contact_get_name()");
        return dc_strdup(ptr::null());
    }

    let contact = &*contact;

    contact.rent(|c| c.get_name().strdup())
}

#[no_mangle]
pub unsafe extern "C" fn dc_contact_get_display_name(
    contact: *mut dc_contact_t,
) -> *mut libc::c_char {
    if contact.is_null() {
        eprintln!("ignoring careless call to dc_contact_get_display_name()");
        return dc_strdup(ptr::null());
    }

    let contact = &*contact;

    contact.rent(|c| c.get_display_name().strdup())
}

#[no_mangle]
pub unsafe extern "C" fn dc_contact_get_name_n_addr(
    contact: *mut dc_contact_t,
) -> *mut libc::c_char {
    if contact.is_null() {
        eprintln!("ignoring careless call to dc_contact_get_name_n_addr()");
        return dc_strdup(ptr::null());
    }

    let contact = &*contact;

    contact.rent(|c| c.get_name_n_addr().strdup())
}

#[no_mangle]
pub unsafe extern "C" fn dc_contact_get_first_name(
    contact: *mut dc_contact_t,
) -> *mut libc::c_char {
    if contact.is_null() {
        eprintln!("ignoring careless call to dc_contact_get_first_name()");
        return dc_strdup(ptr::null());
    }

    let contact = &*contact;

    contact.rent(|c| c.get_first_name().strdup())
}

#[no_mangle]
pub unsafe extern "C" fn dc_contact_get_profile_image(
    contact: *mut dc_contact_t,
) -> *mut libc::c_char {
    if contact.is_null() {
        eprintln!("ignoring careless call to dc_contact_get_profile_image()");
        return ptr::null_mut(); // NULL explicitly defined as "no profile image"
    }

    let contact = &*contact;

    contact.rent(|c| {
        c.get_profile_image()
            .map(|s| s.strdup())
            .unwrap_or_else(|| std::ptr::null_mut())
    })
}

#[no_mangle]
pub unsafe extern "C" fn dc_contact_get_color(contact: *mut dc_contact_t) -> u32 {
    if contact.is_null() {
        eprintln!("ignoring careless call to dc_contact_get_color()");
        return 0;
    }

    let contact = &*contact;

    contact.rent(|c| c.get_color())
}

#[no_mangle]
pub unsafe extern "C" fn dc_contact_is_blocked(contact: *mut dc_contact_t) -> libc::c_int {
    if contact.is_null() {
        eprintln!("ignoring careless call to dc_contact_is_blocked()");
        return 0;
    }

    let contact = &*contact;

    contact.rent(|c| c.is_blocked() as libc::c_int)
}

#[no_mangle]
pub unsafe extern "C" fn dc_contact_is_verified(contact: *mut dc_contact_t) -> libc::c_int {
    if contact.is_null() {
        eprintln!("ignoring careless call to dc_contact_is_verified()");
        return 0;
    }

    let contact = &*contact;

    contact.rent(|c| c.is_verified() as libc::c_int)
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
    strdup_opt(lot.get_text1())
}

#[no_mangle]
pub unsafe extern "C" fn dc_lot_get_text2(lot: *mut dc_lot_t) -> *mut libc::c_char {
    if lot.is_null() {
        eprintln!("ignoring careless call to dc_lot_get_text2()");
        return ptr::null_mut(); // NULL explicitly defined as "there is no such text"
    }

    let lot = &*lot;
    strdup_opt(lot.get_text2())
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

fn as_opt_str<'a>(s: *const libc::c_char) -> Option<&'a str> {
    if s.is_null() {
        return None;
    }

    Some(dc_tools::as_str(s))
}

pub trait ResultExt<T> {
    fn unwrap_or_log_default(self, context: &context::Context, message: &str) -> T;
    fn log_err(&self, context: &context::Context, message: &str);
}

impl<T: Default, E: std::fmt::Display> ResultExt<T> for Result<T, E> {
    fn unwrap_or_log_default(self, context: &context::Context, message: &str) -> T {
        match self {
            Ok(t) => t,
            Err(err) => {
                error!(context, 0, "{}: {}", message, err);
                Default::default()
            }
        }
    }

    fn log_err(&self, context: &context::Context, message: &str) {
        if let Err(err) = self {
            error!(context, 0, "{}: {}", message, err);
        }
    }
}

unsafe fn strdup_opt(s: Option<impl AsRef<str>>) -> *mut libc::c_char {
    match s {
        Some(s) => s.as_ref().strdup(),
        None => ptr::null_mut(),
    }
}

pub trait ResultNullableExt<T> {
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
