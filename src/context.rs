use std::sync::{Arc, Condvar, Mutex, RwLock};

use crate::constants::*;
use crate::dc_array::*;
use crate::dc_chat::*;
use crate::dc_contact::*;
use crate::dc_job::*;
use crate::dc_jobthread::*;
use crate::dc_loginparam::*;
use crate::dc_lot::dc_lot_t;
use crate::dc_move::*;
use crate::dc_msg::*;
use crate::dc_receive_imf::*;
use crate::dc_tools::*;
use crate::imap::*;
use crate::key::*;
use crate::smtp::*;
use crate::sql::Sql;
use crate::types::*;
use crate::x::*;

#[repr(C)]
pub struct Context {
    pub userdata: *mut libc::c_void,
    pub dbfile: Arc<RwLock<*mut libc::c_char>>,
    pub blobdir: Arc<RwLock<*mut libc::c_char>>,
    pub sql: Sql,
    pub inbox: Arc<RwLock<Imap>>,
    pub perform_inbox_jobs_needed: Arc<RwLock<i32>>,
    pub probe_imap_network: Arc<RwLock<i32>>,
    pub sentbox_thread: Arc<RwLock<dc_jobthread_t>>,
    pub mvbox_thread: Arc<RwLock<dc_jobthread_t>>,
    pub smtp: Arc<Mutex<Smtp>>,
    pub smtp_state: Arc<(Mutex<SmtpState>, Condvar)>,
    pub oauth2_critical: Arc<Mutex<()>>,
    pub cb: Option<dc_callback_t>,
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

    pub fn call_cb(&self, event: Event, data1: uintptr_t, data2: uintptr_t) -> uintptr_t {
        if let Some(cb) = self.cb {
            unsafe { cb(self, event, data1, data2) }
        } else {
            0
        }
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

// create/open/config/information
pub fn dc_context_new(
    cb: Option<dc_callback_t>,
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
        sql: Sql::new(),
        smtp: Arc::new(Mutex::new(Smtp::new())),
        smtp_state: Arc::new((Mutex::new(Default::default()), Condvar::new())),
        oauth2_critical: Arc::new(Mutex::new(())),
        bob: Arc::new(RwLock::new(Default::default())),
        last_smeared_timestamp: Arc::new(RwLock::new(0)),
        cmdline_sel_chat_id: Arc::new(RwLock::new(0)),
        sentbox_thread: Arc::new(RwLock::new(dc_jobthread_init(
            "SENTBOX",
            "configured_sentbox_folder",
            Imap::new(
                cb_get_config,
                cb_set_config,
                cb_precheck_imf,
                cb_receive_imf,
            ),
        ))),
        mvbox_thread: Arc::new(RwLock::new(dc_jobthread_init(
            "MVBOX",
            "configured_mvbox_folder",
            Imap::new(
                cb_get_config,
                cb_set_config,
                cb_precheck_imf,
                cb_receive_imf,
            ),
        ))),
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
            info!(
                context,
                0,
                "[move] detected bbc-self {}",
                as_str(rfc724_mid),
            );
            mark_seen = 1i32
        } else if as_str(old_server_folder) != server_folder {
            info!(
                context,
                0,
                "[move] detected moved message {}",
                as_str(rfc724_mid),
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
    context.sql.set_config(context, as_str(key), v).ok();
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
    let res = context
        .sql
        .get_config(context, as_str(key))
        .unwrap_or_else(|| to_string(def));
    to_cstring(res)
}

pub unsafe fn dc_context_unref(context: &mut Context) {
    if 0 != dc_is_open(context) {
        dc_close(context);
    }

    free(context.os_name as *mut libc::c_void);
}

pub unsafe fn dc_close(context: &Context) {
    info!(context, 0, "disconnecting INBOX-watch",);
    context.inbox.read().unwrap().disconnect(context);
    info!(context, 0, "disconnecting sentbox-thread",);
    context
        .sentbox_thread
        .read()
        .unwrap()
        .imap
        .disconnect(context);
    info!(context, 0, "disconnecting mvbox-thread",);
    context
        .mvbox_thread
        .read()
        .unwrap()
        .imap
        .disconnect(context);

    info!(context, 0, "disconnecting SMTP");
    context.smtp.clone().lock().unwrap().disconnect();

    context.sql.close(context);
    let mut dbfile = context.dbfile.write().unwrap();
    free(*dbfile as *mut libc::c_void);
    *dbfile = 0 as *mut libc::c_char;
    let mut blobdir = context.blobdir.write().unwrap();
    free(*blobdir as *mut libc::c_void);
    *blobdir = 0 as *mut libc::c_char;
}

pub unsafe fn dc_is_open(context: &Context) -> libc::c_int {
    match context.sql.is_open() {
        true => 1,
        false => 0,
    }
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
        if context.sql.open(context, as_path(dbfile), 0) {
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

/* ******************************************************************************
 * INI-handling, Information
 ******************************************************************************/

pub unsafe fn dc_get_info(context: &Context) -> *mut libc::c_char {
    let unset = "0";
    let l = dc_loginparam_read(context, &context.sql, "");
    let l2 = dc_loginparam_read(context, &context.sql, "configured_");
    let displayname = context.sql.get_config(context, "displayname");
    let chats = dc_get_chat_cnt(context) as usize;
    let real_msgs = dc_get_real_msg_cnt(context) as usize;
    let deaddrop_msgs = dc_get_deaddrop_msg_cnt(context) as usize;
    let contacts = dc_get_real_contact_cnt(context) as usize;
    let is_configured = context
        .sql
        .get_config_int(context, "configured")
        .unwrap_or_default();
    let dbversion = context
        .sql
        .get_config_int(context, "dbversion")
        .unwrap_or_default();
    let e2ee_enabled = context
        .sql
        .get_config_int(context, "e2ee_enabled")
        .unwrap_or_else(|| 1);
    let mdns_enabled = context
        .sql
        .get_config_int(context, "mdns_enabled")
        .unwrap_or_else(|| 1);

    let prv_key_cnt: Option<isize> = context.sql.query_row_col(
        context,
        "SELECT COUNT(*) FROM keypairs;",
        rusqlite::NO_PARAMS,
        0,
    );

    let pub_key_cnt: Option<isize> = context.sql.query_row_col(
        context,
        "SELECT COUNT(*) FROM acpeerstates;",
        rusqlite::NO_PARAMS,
        0,
    );

    let fingerprint_str = if let Some(key) = Key::from_self_public(context, &l2.addr, &context.sql)
    {
        key.fingerprint()
    } else {
        "<Not yet calculated>".into()
    };

    let l_readable_str = dc_loginparam_get_readable(&l);
    let l2_readable_str = dc_loginparam_get_readable(&l2);
    let inbox_watch = context
        .sql
        .get_config_int(context, "inbox_watch")
        .unwrap_or_else(|| 1);
    let sentbox_watch = context
        .sql
        .get_config_int(context, "sentbox_watch")
        .unwrap_or_else(|| 1);
    let mvbox_watch = context
        .sql
        .get_config_int(context, "mvbox_watch")
        .unwrap_or_else(|| 1);
    let mvbox_move = context
        .sql
        .get_config_int(context, "mvbox_move")
        .unwrap_or_else(|| 1);
    let folders_configured = context
        .sql
        .get_config_int(context, "folders_configured")
        .unwrap_or_default();
    let configured_sentbox_folder = context
        .sql
        .get_config(context, "configured_sentbox_folder")
        .unwrap_or_else(|| "<unset>".to_string());
    let configured_mvbox_folder = context
        .sql
        .get_config(context, "configured_mvbox_folder")
        .unwrap_or_else(|| "<unset>".to_string());

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
        configured_sentbox_folder,
        configured_mvbox_folder,
        mdns_enabled,
        e2ee_enabled,
        prv_key_cnt.unwrap_or_default(),
        pub_key_cnt.unwrap_or_default(),
        fingerprint_str,
    );

    to_cstring(res)
}

pub unsafe fn dc_get_version_str() -> *mut libc::c_char {
    dc_strdup(DC_VERSION_STR as *const u8 as *const libc::c_char)
}

pub fn dc_get_fresh_msgs(context: &Context) -> *mut dc_array_t {
    let show_deaddrop = 0;

    context
        .sql
        .query_map(
            "SELECT m.id FROM msgs m LEFT JOIN contacts ct \
             ON m.from_id=ct.id LEFT JOIN chats c ON m.chat_id=c.id WHERE m.state=?   \
             AND m.hidden=0   \
             AND m.chat_id>?   \
             AND ct.blocked=0   \
             AND (c.blocked=0 OR c.blocked=?) ORDER BY m.timestamp DESC,m.id DESC;",
            &[10, 9, if 0 != show_deaddrop { 2 } else { 0 }],
            |row| row.get(0),
            |rows| {
                let mut ret = dc_array_t::new(128);

                for row in rows {
                    let id = row?;
                    ret.add_id(id);
                }
                Ok(ret.as_ptr())
            },
        )
        .unwrap()
}

pub fn dc_search_msgs(
    context: &Context,
    chat_id: uint32_t,
    query: *const libc::c_char,
) -> *mut dc_array_t {
    if query.is_null() {
        return std::ptr::null_mut();
    }

    let real_query = to_string(query).trim().to_string();
    if real_query.is_empty() {
        return std::ptr::null_mut();
    }
    let strLikeInText = format!("%{}%", &real_query);
    let strLikeBeg = format!("{}%", &real_query);

    let query = if 0 != chat_id {
        "SELECT m.id, m.timestamp FROM msgs m LEFT JOIN contacts ct ON m.from_id=ct.id WHERE m.chat_id=?  \
         AND m.hidden=0  \
         AND ct.blocked=0 AND (txt LIKE ? OR ct.name LIKE ?) ORDER BY m.timestamp,m.id;"
    } else {
        "SELECT m.id, m.timestamp FROM msgs m LEFT JOIN contacts ct ON m.from_id=ct.id \
         LEFT JOIN chats c ON m.chat_id=c.id WHERE m.chat_id>9 AND m.hidden=0  \
         AND (c.blocked=0 OR c.blocked=?) \
         AND ct.blocked=0 AND (m.txt LIKE ? OR ct.name LIKE ?) ORDER BY m.timestamp DESC,m.id DESC;"
    };

    let mut ret = dc_array_t::new(100);

    let success = context
        .sql
        .query_map(
            query,
            params![chat_id as libc::c_int, &strLikeInText, &strLikeBeg],
            |row| row.get::<_, i32>(0),
            |rows| {
                for id in rows {
                    ret.add_id(id? as u32);
                }
                Ok(())
            },
        )
        .is_ok();

    if success {
        return ret.as_ptr();
    }

    std::ptr::null_mut()
}

pub fn dc_is_inbox(_context: &Context, folder_name: impl AsRef<str>) -> bool {
    folder_name.as_ref() == "INBOX"
}

pub fn dc_is_sentbox(context: &Context, folder_name: impl AsRef<str>) -> bool {
    let sentbox_name = context.sql.get_config(context, "configured_sentbox_folder");
    if let Some(name) = sentbox_name {
        name == folder_name.as_ref()
    } else {
        false
    }
}

pub fn dc_is_mvbox(context: &Context, folder_name: impl AsRef<str>) -> bool {
    let mvbox_name = context.sql.get_config(context, "configured_mvbox_folder");

    if let Some(name) = mvbox_name {
        name == folder_name.as_ref()
    } else {
        false
    }
}
