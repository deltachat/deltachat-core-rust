use std::sync::{Arc, Condvar, Mutex, RwLock};

use crate::constants::*;
use crate::dc_array::*;
use crate::dc_chat::*;
use crate::dc_contact::*;
use crate::dc_job::*;
use crate::dc_jobthread::*;
use crate::dc_log::*;
use crate::dc_loginparam::*;
use crate::dc_lot::dc_lot_t;
use crate::dc_move::*;
use crate::dc_msg::*;
use crate::dc_receive_imf::*;
use crate::dc_sqlite3::*;
use crate::dc_stock::*;
use crate::dc_tools::*;
use crate::imap::*;
use crate::key::*;
use crate::smtp::*;
use crate::types::*;
use crate::x::*;

const CONFIG_KEYS: [&'static str; 33] = [
    "addr",
    "mail_server",
    "mail_user",
    "mail_pw",
    "mail_port",
    "send_server",
    "send_user",
    "send_pw",
    "send_port",
    "server_flags",
    "imap_folder",
    "displayname",
    "selfstatus",
    "selfavatar",
    "e2ee_enabled",
    "mdns_enabled",
    "inbox_watch",
    "sentbox_watch",
    "mvbox_watch",
    "mvbox_move",
    "show_emails",
    "save_mime_headers",
    "configured_addr",
    "configured_mail_server",
    "configured_mail_user",
    "configured_mail_pw",
    "configured_mail_port",
    "configured_send_server",
    "configured_send_user",
    "configured_send_pw",
    "configured_send_port",
    "configured_server_flags",
    "configured",
];

// deprecated
const SYS_CONFIG_KEYS: [&'static str; 3] = [
    "sys.version",
    "sys.msgsize_max_recommended",
    "sys.config_keys",
];

#[repr(C)]
pub struct Context {
    pub userdata: *mut libc::c_void,
    pub dbfile: Arc<RwLock<*mut libc::c_char>>,
    pub blobdir: Arc<RwLock<*mut libc::c_char>>,
    pub sql: Arc<RwLock<dc_sqlite3_t>>,
    pub inbox: Arc<RwLock<Imap>>,
    pub perform_inbox_jobs_needed: Arc<RwLock<i32>>,
    pub probe_imap_network: Arc<RwLock<i32>>,
    pub sentbox_thread: Arc<RwLock<dc_jobthread_t>>,
    pub mvbox_thread: Arc<RwLock<dc_jobthread_t>>,
    pub smtp: Arc<Mutex<Smtp>>,
    pub smtp_state: Arc<(Mutex<SmtpState>, Condvar)>,
    pub oauth2_critical: Arc<Mutex<()>>,
    pub cb: dc_callback_t,
    pub os_name: *mut libc::c_char,
    pub cmdline_sel_chat_id: Arc<RwLock<u32>>,
    pub bob: Arc<RwLock<BobStatus>>,
    pub last_smeared_timestamp: Arc<RwLock<i64>>,
    pub running_state: Arc<RwLock<RunningState>>,
}

unsafe impl std::marker::Send for Context {}
unsafe impl std::marker::Sync for Context {}

#[derive(Debug, PartialEq, Eq)]
pub struct RunningState {
    pub ongoing_running: bool,
    pub shall_stop_ongoing: bool,
}

impl Context {
    pub fn has_dbfile(&self) -> bool {
        !self.get_dbfile().is_null()
    }

    pub fn has_blobdir(&self) -> bool {
        !self.get_blobdir().is_null()
    }

    pub fn get_dbfile(&self) -> *const libc::c_char {
        *self.dbfile.clone().read().unwrap()
    }

    pub fn get_blobdir(&self) -> *const libc::c_char {
        *self.blobdir.clone().read().unwrap()
    }
}

impl Default for RunningState {
    fn default() -> Self {
        RunningState {
            ongoing_running: false,
            shall_stop_ongoing: true,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct BobStatus {
    pub expects: i32,
    pub status: i32,
    pub qr_scan: *mut dc_lot_t,
}

impl Default for BobStatus {
    fn default() -> Self {
        BobStatus {
            expects: 0,
            status: 0,
            qr_scan: std::ptr::null_mut(),
        }
    }
}

#[derive(Default, Debug)]
pub struct SmtpState {
    pub idle: bool,
    pub suspended: i32,
    pub doing_jobs: i32,
    pub perform_jobs_needed: i32,
    pub probe_network: i32,
}

// location handling
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _dc_location {
    pub location_id: uint32_t,
    pub latitude: libc::c_double,
    pub longitude: libc::c_double,
    pub accuracy: libc::c_double,
    pub timestamp: i64,
    pub contact_id: uint32_t,
    pub msg_id: uint32_t,
    pub chat_id: uint32_t,
    pub marker: *mut libc::c_char,
    pub independent: uint32_t,
}

// create/open/config/information
pub fn dc_context_new(
    cb: dc_callback_t,
    userdata: *mut libc::c_void,
    os_name: *const libc::c_char,
) -> Context {
    Context {
        blobdir: Arc::new(RwLock::new(std::ptr::null_mut())),
        dbfile: Arc::new(RwLock::new(std::ptr::null_mut())),
        inbox: Arc::new(RwLock::new({
            Imap::new(
                cb_get_config,
                cb_set_config,
                cb_precheck_imf,
                cb_receive_imf,
            )
        })),
        userdata,
        cb,
        os_name: unsafe { dc_strdup_keep_null(os_name) },
        running_state: Arc::new(RwLock::new(Default::default())),
        sql: Arc::new(RwLock::new(dc_sqlite3_new())),
        smtp: Arc::new(Mutex::new(Smtp::new())),
        smtp_state: Arc::new((Mutex::new(Default::default()), Condvar::new())),
        oauth2_critical: Arc::new(Mutex::new(())),
        bob: Arc::new(RwLock::new(Default::default())),
        last_smeared_timestamp: Arc::new(RwLock::new(0)),
        cmdline_sel_chat_id: Arc::new(RwLock::new(0)),
        sentbox_thread: Arc::new(RwLock::new(unsafe {
            dc_jobthread_init(
                b"SENTBOX\x00" as *const u8 as *const libc::c_char,
                b"configured_sentbox_folder\x00" as *const u8 as *const libc::c_char,
                Imap::new(
                    cb_get_config,
                    cb_set_config,
                    cb_precheck_imf,
                    cb_receive_imf,
                ),
            )
        })),
        mvbox_thread: Arc::new(RwLock::new(unsafe {
            dc_jobthread_init(
                b"MVBOX\x00" as *const u8 as *const libc::c_char,
                b"configured_mvbox_folder\x00" as *const u8 as *const libc::c_char,
                Imap::new(
                    cb_get_config,
                    cb_set_config,
                    cb_precheck_imf,
                    cb_receive_imf,
                ),
            )
        })),
        probe_imap_network: Arc::new(RwLock::new(0)),
        perform_inbox_jobs_needed: Arc::new(RwLock::new(0)),
    }
}

unsafe fn cb_receive_imf(
    context: &Context,
    imf_raw_not_terminated: *const libc::c_char,
    imf_raw_bytes: size_t,
    server_folder: &str,
    server_uid: uint32_t,
    flags: uint32_t,
) {
    dc_receive_imf(
        context,
        imf_raw_not_terminated,
        imf_raw_bytes,
        server_folder,
        server_uid,
        flags,
    );
}

unsafe fn cb_precheck_imf(
    context: &Context,
    rfc724_mid: *const libc::c_char,
    server_folder: &str,
    server_uid: uint32_t,
) -> libc::c_int {
    let mut rfc724_mid_exists: libc::c_int = 0i32;
    let msg_id: uint32_t;
    let mut old_server_folder: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut old_server_uid: uint32_t = 0i32 as uint32_t;
    let mut mark_seen: libc::c_int = 0i32;
    msg_id = dc_rfc724_mid_exists(
        context,
        rfc724_mid,
        &mut old_server_folder,
        &mut old_server_uid,
    );
    if msg_id != 0i32 as libc::c_uint {
        rfc724_mid_exists = 1i32;
        if *old_server_folder.offset(0isize) as libc::c_int == 0i32
            && old_server_uid == 0i32 as libc::c_uint
        {
            dc_log_info(
                context,
                0i32,
                b"[move] detected bbc-self %s\x00" as *const u8 as *const libc::c_char,
                rfc724_mid,
            );
            mark_seen = 1i32
        } else if as_str(old_server_folder) != server_folder {
            dc_log_info(
                context,
                0i32,
                b"[move] detected moved message %s\x00" as *const u8 as *const libc::c_char,
                rfc724_mid,
            );
            dc_update_msg_move_state(context, rfc724_mid, DC_MOVE_STATE_STAY);
        }
        if as_str(old_server_folder) != server_folder || old_server_uid != server_uid {
            dc_update_server_uid(context, rfc724_mid, server_folder, server_uid);
        }
        dc_do_heuristics_moves(context, server_folder, msg_id);
        if 0 != mark_seen {
            dc_job_add(
                context,
                130i32,
                msg_id as libc::c_int,
                0 as *const libc::c_char,
                0i32,
            );
        }
    }
    free(old_server_folder as *mut libc::c_void);
    return rfc724_mid_exists;
}

unsafe fn cb_set_config(context: &Context, key: *const libc::c_char, value: *const libc::c_char) {
    let v = if value.is_null() {
        None
    } else {
        Some(as_str(value))
    };
    dc_sqlite3_set_config(
        context,
        &context.sql.clone().read().unwrap(),
        as_str(key),
        v,
    );
}

/* *
 * The following three callback are given to dc_imap_new() to read/write configuration
 * and to handle received messages. As the imap-functions are typically used in
 * a separate user-thread, also these functions may be called from a different thread.
 *
 * @private @memberof Context
 */
unsafe fn cb_get_config(
    context: &Context,
    key: *const libc::c_char,
    def: *const libc::c_char,
) -> *mut libc::c_char {
    let d = if def.is_null() {
        None
    } else {
        Some(as_str(def))
    };
    let res = dc_sqlite3_get_config(
        context,
        &context.sql.clone().read().unwrap(),
        as_str(key),
        d,
    );
    if let Some(res) = res {
        strdup(to_cstring(res).as_ptr())
    } else {
        std::ptr::null_mut()
    }
}

pub unsafe fn dc_context_unref(context: &mut Context) {
    if 0 != dc_is_open(context) {
        dc_close(context);
    }

    dc_jobthread_exit(&mut context.sentbox_thread.clone().write().unwrap());
    dc_jobthread_exit(&mut context.mvbox_thread.clone().write().unwrap());

    free(context.os_name as *mut libc::c_void);
}

pub unsafe fn dc_close(context: &Context) {
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

    if 0 != dc_sqlite3_is_open(&context.sql.clone().read().unwrap()) {
        dc_sqlite3_close(context, &mut context.sql.clone().write().unwrap());
    }
    let mut dbfile = context.dbfile.write().unwrap();
    free(*dbfile as *mut libc::c_void);
    *dbfile = 0 as *mut libc::c_char;
    let mut blobdir = context.blobdir.write().unwrap();
    free(*blobdir as *mut libc::c_void);
    *blobdir = 0 as *mut libc::c_char;
}

pub unsafe fn dc_is_open(context: &Context) -> libc::c_int {
    dc_sqlite3_is_open(&context.sql.clone().read().unwrap())
}

pub unsafe fn dc_get_userdata(context: &mut Context) -> *mut libc::c_void {
    context.userdata as *mut _
}

pub unsafe fn dc_open(
    context: &Context,
    dbfile: *const libc::c_char,
    blobdir: *const libc::c_char,
) -> libc::c_int {
    let mut success = 0;
    if 0 != dc_is_open(context) {
        return 0;
    }
    if !dbfile.is_null() {
        *context.dbfile.write().unwrap() = dc_strdup(dbfile);
        if !blobdir.is_null() && 0 != *blobdir.offset(0isize) as libc::c_int {
            let dir = dc_strdup(blobdir);
            dc_ensure_no_slash(dir);
            *context.blobdir.write().unwrap() = dir;
        } else {
            let dir = dc_mprintf(b"%s-blobs\x00" as *const u8 as *const libc::c_char, dbfile);
            dc_create_folder(context, dir);
            *context.blobdir.write().unwrap() = dir;
        }
        // Create/open sqlite database, this may already use the blobdir
        if !(0 == dc_sqlite3_open(context, &mut context.sql.write().unwrap(), dbfile, 0i32)) {
            success = 1i32
        }
    }
    if 0 == success {
        dc_close(context);
    }
    success
}

pub unsafe fn dc_get_blobdir(context: &Context) -> *mut libc::c_char {
    dc_strdup(*context.blobdir.clone().read().unwrap())
}

pub fn dc_set_config(context: &Context, key: impl AsRef<str>, value: Option<&str>) -> libc::c_int {
    let mut ret = 0;

    if !is_settable_config_key(key.as_ref()) {
        return 0;
    }

    match key.as_ref() {
        "selfavatar" if value.is_some() => {
            let mut rel_path = unsafe { dc_strdup(to_cstring(value.unwrap()).as_ptr()) };
            if 0 != unsafe { dc_make_rel_and_copy(context, &mut rel_path) } {
                ret = dc_sqlite3_set_config(
                    context,
                    &context.sql.clone().read().unwrap(),
                    key,
                    Some(as_str(rel_path)),
                );
            }
            unsafe { free(rel_path as *mut libc::c_void) };
        }
        "inbox_watch" => {
            ret = dc_sqlite3_set_config(context, &context.sql.clone().read().unwrap(), key, value);
            unsafe { dc_interrupt_imap_idle(context) };
        }
        "sentbox_watch" => {
            ret = dc_sqlite3_set_config(context, &context.sql.clone().read().unwrap(), key, value);
            unsafe { dc_interrupt_sentbox_idle(context) };
        }
        "mvbox_watch" => {
            ret = dc_sqlite3_set_config(context, &context.sql.clone().read().unwrap(), key, value);
            unsafe { dc_interrupt_mvbox_idle(context) };
        }
        "selfstatus" => {
            let def = unsafe { dc_stock_str(context, 13) };
            let val = if value.is_none() || value.unwrap() == as_str(def) {
                None
            } else {
                value
            };

            ret = dc_sqlite3_set_config(context, &context.sql.clone().read().unwrap(), key, val);
            unsafe { free(def as *mut libc::c_void) };
        }
        _ => {
            ret = dc_sqlite3_set_config(context, &context.sql.clone().read().unwrap(), key, value);
        }
    }
    ret
}

/* ******************************************************************************
 * INI-handling, Information
 ******************************************************************************/

fn is_settable_config_key(key: impl AsRef<str>) -> bool {
    CONFIG_KEYS
        .into_iter()
        .find(|c| **c == key.as_ref())
        .is_some()
}

pub fn dc_get_config(context: &Context, key: impl AsRef<str>) -> String {
    if key.as_ref().starts_with("sys") {
        return get_sys_config_str(key.as_ref());
    }

    if !is_gettable_config_key(key.as_ref()) {
        return "".into();
    }

    let value = match key.as_ref() {
        "selfavatar" => {
            let rel_path = dc_sqlite3_get_config(
                context,
                &context.sql.clone().read().unwrap(),
                key.as_ref(),
                None,
            );
            rel_path.map(|p| {
                let v = unsafe { dc_get_abs_path(context, to_cstring(p).as_ptr()) };
                let r = to_string(v);
                unsafe { free(v as *mut _) };
                r
            })
        }
        _ => dc_sqlite3_get_config(
            context,
            &context.sql.clone().read().unwrap(),
            key.as_ref(),
            None,
        ),
    };

    if value.is_some() {
        return value.unwrap();
    }

    match key.as_ref() {
        "e2ee_enabled" => "1".into(),
        "mdns_enabled" => "1".into(),
        "imap_folder" => "INBOX".into(),
        "inbox_watch" => "1".into(),
        "sentbox_watch" | "mvbox_watch" | "mvbox_move" => "1".into(),
        "show_emails" => "0".into(),
        "selfstatus" => {
            let s = unsafe { dc_stock_str(context, 13) };
            let res = to_string(s);
            unsafe { free(s as *mut _) };
            res
        }
        _ => "".into(),
    }
}

fn is_gettable_config_key(key: impl AsRef<str>) -> bool {
    SYS_CONFIG_KEYS
        .into_iter()
        .find(|c| **c == key.as_ref())
        .is_some()
        || is_settable_config_key(key)
}

fn get_sys_config_str(key: impl AsRef<str>) -> String {
    match key.as_ref() {
        "sys.version" => std::str::from_utf8(DC_VERSION_STR).unwrap().into(),
        "sys.msgsize_max_recommended" => format!("{}", 24 * 1024 * 1024 / 4 * 3),
        "sys.config_keys" => get_config_keys_str(),
        _ => "".into(),
    }
}

fn get_config_keys_str() -> String {
    let keys = &CONFIG_KEYS[..].join(" ");
    let sys_keys = &SYS_CONFIG_KEYS[..].join(" ");

    format!("{} {}", keys, sys_keys)
}

pub unsafe fn dc_get_info(context: &Context) -> *mut libc::c_char {
    let unset = "0";
    let l = dc_loginparam_read(context, &context.sql.clone().read().unwrap(), "");
    let l2 = dc_loginparam_read(context, &context.sql.clone().read().unwrap(), "configured_");
    let displayname = dc_sqlite3_get_config(
        context,
        &context.sql.clone().read().unwrap(),
        "displayname",
        None,
    );
    let chats = dc_get_chat_cnt(context) as usize;
    let real_msgs = dc_get_real_msg_cnt(context) as usize;
    let deaddrop_msgs = dc_get_deaddrop_msg_cnt(context) as usize;
    let contacts = dc_get_real_contact_cnt(context) as usize;
    let is_configured = dc_sqlite3_get_config_int(
        context,
        &context.sql.clone().read().unwrap(),
        "configured",
        0,
    );
    let dbversion = dc_sqlite3_get_config_int(
        context,
        &context.sql.clone().read().unwrap(),
        "dbversion",
        0,
    );
    let e2ee_enabled = dc_sqlite3_get_config_int(
        context,
        &context.sql.clone().read().unwrap(),
        "e2ee_enabled",
        1,
    );
    let mdns_enabled = dc_sqlite3_get_config_int(
        context,
        &context.sql.clone().read().unwrap(),
        "mdns_enabled",
        1,
    );

    let prv_key_cnt: Option<isize> = dc_sqlite3_query_row(
        context,
        &context.sql.clone().read().unwrap(),
        "SELECT COUNT(*) FROM keypairs;",
        rusqlite::NO_PARAMS,
        0,
    );

    let pub_key_cnt: Option<isize> = dc_sqlite3_query_row(
        context,
        &context.sql.clone().read().unwrap(),
        "SELECT COUNT(*) FROM acpeerstates;",
        rusqlite::NO_PARAMS,
        0,
    );

    let fingerprint_str = if let Some(key) =
        Key::from_self_public(context, &l2.addr, &context.sql.clone().read().unwrap())
    {
        key.fingerprint()
    } else {
        "<Not yet calculated>".into()
    };

    let l_readable_str = dc_loginparam_get_readable(&l);
    let l2_readable_str = dc_loginparam_get_readable(&l2);
    let inbox_watch = dc_sqlite3_get_config_int(
        context,
        &context.sql.clone().read().unwrap(),
        "inbox_watch",
        1,
    );
    let sentbox_watch = dc_sqlite3_get_config_int(
        context,
        &context.sql.clone().read().unwrap(),
        "sentbox_watch",
        1,
    );
    let mvbox_watch = dc_sqlite3_get_config_int(
        context,
        &context.sql.clone().read().unwrap(),
        "mvbox_watch",
        1,
    );
    let mvbox_move = dc_sqlite3_get_config_int(
        context,
        &context.sql.clone().read().unwrap(),
        "mvbox_move",
        1,
    );
    let folders_configured = dc_sqlite3_get_config_int(
        context,
        &context.sql.clone().read().unwrap(),
        "folders_configured",
        0,
    );
    let configured_sentbox_folder = dc_sqlite3_get_config(
        context,
        &context.sql.clone().read().unwrap(),
        "configured_sentbox_folder",
        Some("<unset>"),
    );
    let configured_mvbox_folder = dc_sqlite3_get_config(
        context,
        &context.sql.clone().read().unwrap(),
        "configured_mvbox_folder",
        Some("<unset>"),
    );

    let res = format!(
        "deltachat_core_version=v{}\n\
         sqlite_version={}\n\
         sqlite_thread_safe={}\n\
         arch={}\n\
         number_of_chats={}\n\
         number_of_chat_messages={}\n\
         messages_in_contact_requests={}\n\
         number_of_contacts={}\n\
         database_dir={}\n\
         database_version={}\n\
         blobdir={}\n\
         display_name={}\n\
         is_configured={}\n\
         entered_account_settings={}\n\
         used_account_settings={}\n\
         inbox_watch={}\n\
         sentbox_watch={}\n\
         mvbox_watch={}\n\
         mvbox_move={}\n\
         folders_configured={}\n\
         configured_sentbox_folder={}\n\
         configured_mvbox_folder={}\n\
         mdns_enabled={}\n\
         e2ee_enabled={}\n\
         private_key_count={}\n\
         public_key_count={}\n\
         fingerprint={}\n\
         level=awesome\n",
        as_str(DC_VERSION_STR as *const u8 as *const _),
        rusqlite::version(),
        sqlite3_threadsafe(),
        // arch
        (::std::mem::size_of::<*mut libc::c_void>()).wrapping_mul(8),
        chats,
        real_msgs,
        deaddrop_msgs,
        contacts,
        if context.has_dbfile() {
            as_str(context.get_dbfile())
        } else {
            unset
        },
        dbversion,
        if context.has_blobdir() {
            as_str(context.get_blobdir())
        } else {
            unset
        },
        displayname.unwrap_or_else(|| unset.into()),
        is_configured,
        l_readable_str,
        l2_readable_str,
        inbox_watch,
        sentbox_watch,
        mvbox_watch,
        mvbox_move,
        folders_configured,
        configured_sentbox_folder.unwrap_or_default(),
        configured_mvbox_folder.unwrap_or_default(),
        mdns_enabled,
        e2ee_enabled,
        prv_key_cnt.unwrap_or_default(),
        pub_key_cnt.unwrap_or_default(),
        fingerprint_str,
    );

    strdup(to_cstring(res).as_ptr())
}

pub unsafe fn dc_get_version_str() -> *mut libc::c_char {
    dc_strdup(DC_VERSION_STR as *const u8 as *const libc::c_char)
}

pub fn dc_get_fresh_msgs(context: &Context) -> *mut dc_array_t {
    let show_deaddrop = 0;
    let ret = unsafe { dc_array_new(128 as size_t) };
    if !ret.is_null() {
        if let Some(ref mut stmt) = dc_sqlite3_prepare(
            context,
            &context.sql.clone().read().unwrap(),
            "SELECT m.id FROM msgs m LEFT JOIN contacts ct \
             ON m.from_id=ct.id LEFT JOIN chats c ON m.chat_id=c.id WHERE m.state=?   \
             AND m.hidden=0   \
             AND m.chat_id>?   \
             AND ct.blocked=0   \
             AND (c.blocked=0 OR c.blocked=?) ORDER BY m.timestamp DESC,m.id DESC;",
        ) {
            match stmt.query_map(&[10, 9, if 0 != show_deaddrop { 2 } else { 0 }], |row| {
                row.get(0)
            }) {
                Ok(rows) => {
                    for row in rows {
                        if let Ok(id) = row {
                            unsafe { dc_array_add_id(ret, id) };
                        }
                    }
                }
                Err(_err) => {}
            }
        }
    }

    ret
}

pub unsafe fn dc_search_msgs(
    context: &Context,
    chat_id: uint32_t,
    query: *const libc::c_char,
) -> *mut dc_array_t {
    let mut success = false;
    let ret = dc_array_new(100 as size_t);

    if !(ret.is_null() || query.is_null()) {
        let real_query = to_string(query).trim().to_string();
        if real_query.is_empty() {
            success = true;
        } else {
            let strLikeInText = format!("%{}%", &real_query);
            let strLikeBeg = format!("{}%", &real_query);

            let rows = if 0 != chat_id {
                dc_sqlite3_prepare(
                    context,
                    &context.sql.clone().read().unwrap(),
                    "SELECT m.id, m.timestamp FROM msgs m LEFT JOIN contacts ct ON m.from_id=ct.id WHERE m.chat_id=?  \
                      AND m.hidden=0  \
                      AND ct.blocked=0 AND (txt LIKE ? OR ct.name LIKE ?) ORDER BY m.timestamp,m.id;"
                ).and_then(|mut stmt| stmt.query_map(
                    params![chat_id as libc::c_int, &strLikeInText, &strLikeBeg],
                    |row| row.get::<_, i32>(0)
                ).and_then(|res| res.collect::<rusqlite::Result<Vec<i32>>>()).ok())
            } else {
                let show_deaddrop = 0;
                dc_sqlite3_prepare(
                    context,
                    &context.sql.clone().read().unwrap(),
                    "SELECT m.id, m.timestamp FROM msgs m LEFT JOIN contacts ct ON m.from_id=ct.id \
                      LEFT JOIN chats c ON m.chat_id=c.id WHERE m.chat_id>9 AND m.hidden=0  \
                      AND (c.blocked=0 OR c.blocked=?) \
                      AND ct.blocked=0 AND (m.txt LIKE ? OR ct.name LIKE ?) ORDER BY m.timestamp DESC,m.id DESC;"
                ).and_then(|mut stmt|
                    stmt.query_map(params![
                        if 0 != show_deaddrop { 2 } else { 0 },
                        strLikeInText, strLikeBeg,
                    ], |row| row.get::<_, i32>(0)).and_then(|res| res.collect::<rusqlite::Result<Vec<i32>>>()).ok()
                )
            };
            if let Some(ids) = rows {
                for id in ids {
                    unsafe { dc_array_add_id(ret, id as u32) };
                }
                success = true;
            }
        }
    }

    if success {
        ret
    } else {
        if !ret.is_null() {
            dc_array_unref(ret);
        }
        0 as *mut dc_array_t
    }
}

pub fn dc_is_inbox(_context: &Context, folder_name: impl AsRef<str>) -> bool {
    folder_name.as_ref() == "INBOX"
}

pub fn dc_is_sentbox(context: &Context, folder_name: impl AsRef<str>) -> bool {
    let sentbox_name = dc_sqlite3_get_config(
        context,
        &context.sql.clone().read().unwrap(),
        "configured_sentbox_folder",
        None,
    );
    if let Some(name) = sentbox_name {
        name == folder_name.as_ref()
    } else {
        false
    }
}

pub fn dc_is_mvbox(context: &Context, folder_name: impl AsRef<str>) -> bool {
    let mvbox_name = dc_sqlite3_get_config(
        context,
        &context.sql.clone().read().unwrap(),
        "configured_mvbox_folder",
        None,
    );

    if let Some(name) = mvbox_name {
        name == folder_name.as_ref()
    } else {
        false
    }
}
