use std::sync::{Arc, Condvar, Mutex, RwLock};

use crate::constants::*;
use crate::dc_array::*;
use crate::dc_chat::*;
use crate::dc_contact::*;
use crate::dc_imap::*;
use crate::dc_job::*;
use crate::dc_jobthread::*;
use crate::dc_key::*;
use crate::dc_log::*;
use crate::dc_loginparam::*;
use crate::dc_lot::dc_lot_t;
use crate::dc_move::*;
use crate::dc_msg::*;
use crate::dc_receive_imf::*;
use crate::dc_smtp::*;
use crate::dc_sqlite3::*;
use crate::dc_stock::*;
use crate::dc_strbuilder::*;
use crate::dc_tools::*;
use crate::types::*;
use crate::x::*;

#[repr(C)]
pub struct dc_context_t {
    pub userdata: *mut libc::c_void,
    pub dbfile: Arc<RwLock<*mut libc::c_char>>,
    pub blobdir: Arc<RwLock<*mut libc::c_char>>,
    pub sql: Arc<RwLock<dc_sqlite3_t>>,
    pub inbox: Arc<RwLock<dc_imap_t>>,
    pub perform_inbox_jobs_needed: Arc<RwLock<i32>>,
    pub probe_imap_network: Arc<RwLock<i32>>,
    pub sentbox_thread: Arc<Mutex<dc_jobthread_t>>,
    pub mvbox_thread: Arc<Mutex<dc_jobthread_t>>,
    pub smtp: Arc<Mutex<Smtp>>,
    pub smtp_state: Arc<(Mutex<SmtpState>, Condvar)>,
    pub oauth2_critical: Arc<Mutex<()>>,
    pub cb: dc_callback_t,
    pub os_name: *mut libc::c_char,
    pub cmdline_sel_chat_id: Arc<RwLock<u32>>,
    pub bob: Arc<RwLock<BobStatus>>,
    pub last_smeared_timestamp: Arc<RwLock<time_t>>,
    pub running_state: Arc<RwLock<RunningState>>,
}

unsafe impl std::marker::Send for dc_context_t {}
unsafe impl std::marker::Sync for dc_context_t {}

#[derive(Debug)]
pub struct RunningState {
    pub ongoing_running: bool,
    pub shall_stop_ongoing: bool,
}

impl dc_context_t {
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

#[derive(Debug)]
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
    pub timestamp: time_t,
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
) -> dc_context_t {
    dc_context_t {
        blobdir: Arc::new(RwLock::new(std::ptr::null_mut())),
        dbfile: Arc::new(RwLock::new(std::ptr::null_mut())),
        inbox: Arc::new(RwLock::new({
            dc_imap_new(
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
        last_smeared_timestamp: Arc::new(RwLock::new(0 as time_t)),
        cmdline_sel_chat_id: Arc::new(RwLock::new(0)),
        sentbox_thread: Arc::new(Mutex::new(unsafe {
            dc_jobthread_init(
                b"SENTBOX\x00" as *const u8 as *const libc::c_char,
                b"configured_sentbox_folder\x00" as *const u8 as *const libc::c_char,
                dc_imap_new(
                    cb_get_config,
                    cb_set_config,
                    cb_precheck_imf,
                    cb_receive_imf,
                ),
            )
        })),
        mvbox_thread: Arc::new(Mutex::new(unsafe {
            dc_jobthread_init(
                b"MVBOX\x00" as *const u8 as *const libc::c_char,
                b"configured_mvbox_folder\x00" as *const u8 as *const libc::c_char,
                dc_imap_new(
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
    context: &dc_context_t,
    imf_raw_not_terminated: *const libc::c_char,
    imf_raw_bytes: size_t,
    server_folder: *const libc::c_char,
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
    context: &dc_context_t,
    rfc724_mid: *const libc::c_char,
    server_folder: *const libc::c_char,
    server_uid: uint32_t,
) -> libc::c_int {
    let mut rfc724_mid_exists: libc::c_int = 0i32;
    let mut msg_id: uint32_t;
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
        } else if strcmp(old_server_folder, server_folder) != 0i32 {
            dc_log_info(
                context,
                0i32,
                b"[move] detected moved message %s\x00" as *const u8 as *const libc::c_char,
                rfc724_mid,
            );
            dc_update_msg_move_state(context, rfc724_mid, DC_MOVE_STATE_STAY);
        }
        if strcmp(old_server_folder, server_folder) != 0i32 || old_server_uid != server_uid {
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

unsafe fn cb_set_config(
    context: &dc_context_t,
    key: *const libc::c_char,
    value: *const libc::c_char,
) {
    dc_sqlite3_set_config(context, &context.sql.clone().read().unwrap(), key, value);
}

/* *
 * The following three callback are given to dc_imap_new() to read/write configuration
 * and to handle received messages. As the imap-functions are typically used in
 * a separate user-thread, also these functions may be called from a different thread.
 *
 * @private @memberof dc_context_t
 */
unsafe fn cb_get_config(
    context: &dc_context_t,
    key: *const libc::c_char,
    def: *const libc::c_char,
) -> *mut libc::c_char {
    dc_sqlite3_get_config(context, &context.sql.clone().read().unwrap(), key, def)
}

pub unsafe fn dc_context_unref(context: &mut dc_context_t) {
    if 0 != dc_is_open(context) {
        dc_close(context);
    }
    dc_sqlite3_unref(context, &mut context.sql.clone().write().unwrap());

    dc_jobthread_exit(&mut context.sentbox_thread.clone().lock().unwrap());
    dc_jobthread_exit(&mut context.mvbox_thread.clone().lock().unwrap());

    free(context.os_name as *mut libc::c_void);
}

pub unsafe fn dc_close(context: &dc_context_t) {
    context.inbox.read().unwrap().disconnect(context);
    context
        .sentbox_thread
        .lock()
        .unwrap()
        .imap
        .lock()
        .unwrap()
        .disconnect(context);
    context
        .mvbox_thread
        .lock()
        .unwrap()
        .imap
        .lock()
        .unwrap()
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

pub unsafe fn dc_is_open(context: &dc_context_t) -> libc::c_int {
    dc_sqlite3_is_open(&context.sql.clone().read().unwrap())
}

pub unsafe fn dc_get_userdata(context: &mut dc_context_t) -> *mut libc::c_void {
    context.userdata as *mut _
}

pub unsafe fn dc_open(
    context: &dc_context_t,
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

pub unsafe fn dc_get_blobdir(context: &dc_context_t) -> *mut libc::c_char {
    dc_strdup(*context.blobdir.clone().read().unwrap())
}

pub unsafe fn dc_set_config(
    context: &dc_context_t,
    key: *const libc::c_char,
    value: *const libc::c_char,
) -> libc::c_int {
    let mut ret = 0;
    let mut rel_path = 0 as *mut libc::c_char;

    if key.is_null() || 0 == is_settable_config_key(key) {
        return 0;
    }
    if strcmp(key, b"selfavatar\x00" as *const u8 as *const libc::c_char) == 0 && !value.is_null() {
        rel_path = dc_strdup(value);
        if !(0 == dc_make_rel_and_copy(context, &mut rel_path)) {
            ret =
                dc_sqlite3_set_config(context, &context.sql.clone().read().unwrap(), key, rel_path)
        }
    } else if strcmp(key, b"inbox_watch\x00" as *const u8 as *const libc::c_char) == 0 {
        ret = dc_sqlite3_set_config(context, &context.sql.clone().read().unwrap(), key, value);
        dc_interrupt_imap_idle(context);
    } else if strcmp(
        key,
        b"sentbox_watch\x00" as *const u8 as *const libc::c_char,
    ) == 0
    {
        ret = dc_sqlite3_set_config(context, &context.sql.clone().read().unwrap(), key, value);
        dc_interrupt_sentbox_idle(context);
    } else if strcmp(key, b"mvbox_watch\x00" as *const u8 as *const libc::c_char) == 0 {
        ret = dc_sqlite3_set_config(context, &context.sql.clone().read().unwrap(), key, value);
        dc_interrupt_mvbox_idle(context);
    } else if strcmp(key, b"selfstatus\x00" as *const u8 as *const libc::c_char) == 0 {
        let mut def = dc_stock_str(context, 13);
        ret = dc_sqlite3_set_config(
            context,
            &context.sql.clone().read().unwrap(),
            key,
            if value.is_null() || strcmp(value, def) == 0 {
                0 as *const libc::c_char
            } else {
                value
            },
        );
        free(def as *mut libc::c_void);
    } else {
        ret = dc_sqlite3_set_config(context, &context.sql.clone().read().unwrap(), key, value);
    }
    free(rel_path as *mut libc::c_void);
    ret
}

/* ******************************************************************************
 * INI-handling, Information
 ******************************************************************************/

unsafe fn is_settable_config_key(key: *const libc::c_char) -> libc::c_int {
    let mut i = 0;
    while i
        < (::std::mem::size_of::<[*const libc::c_char; 33]>())
            .wrapping_div(::std::mem::size_of::<*mut libc::c_char>())
    {
        if strcmp(key, config_keys[i as usize]) == 0 {
            return 1;
        }
        i += 1
    }
    0
}

static mut config_keys: [*const libc::c_char; 33] = [
    b"addr\x00" as *const u8 as *const libc::c_char,
    b"mail_server\x00" as *const u8 as *const libc::c_char,
    b"mail_user\x00" as *const u8 as *const libc::c_char,
    b"mail_pw\x00" as *const u8 as *const libc::c_char,
    b"mail_port\x00" as *const u8 as *const libc::c_char,
    b"send_server\x00" as *const u8 as *const libc::c_char,
    b"send_user\x00" as *const u8 as *const libc::c_char,
    b"send_pw\x00" as *const u8 as *const libc::c_char,
    b"send_port\x00" as *const u8 as *const libc::c_char,
    b"server_flags\x00" as *const u8 as *const libc::c_char,
    b"imap_folder\x00" as *const u8 as *const libc::c_char,
    b"displayname\x00" as *const u8 as *const libc::c_char,
    b"selfstatus\x00" as *const u8 as *const libc::c_char,
    b"selfavatar\x00" as *const u8 as *const libc::c_char,
    b"e2ee_enabled\x00" as *const u8 as *const libc::c_char,
    b"mdns_enabled\x00" as *const u8 as *const libc::c_char,
    b"inbox_watch\x00" as *const u8 as *const libc::c_char,
    b"sentbox_watch\x00" as *const u8 as *const libc::c_char,
    b"mvbox_watch\x00" as *const u8 as *const libc::c_char,
    b"mvbox_move\x00" as *const u8 as *const libc::c_char,
    b"show_emails\x00" as *const u8 as *const libc::c_char,
    b"save_mime_headers\x00" as *const u8 as *const libc::c_char,
    b"configured_addr\x00" as *const u8 as *const libc::c_char,
    b"configured_mail_server\x00" as *const u8 as *const libc::c_char,
    b"configured_mail_user\x00" as *const u8 as *const libc::c_char,
    b"configured_mail_pw\x00" as *const u8 as *const libc::c_char,
    b"configured_mail_port\x00" as *const u8 as *const libc::c_char,
    b"configured_send_server\x00" as *const u8 as *const libc::c_char,
    b"configured_send_user\x00" as *const u8 as *const libc::c_char,
    b"configured_send_pw\x00" as *const u8 as *const libc::c_char,
    b"configured_send_port\x00" as *const u8 as *const libc::c_char,
    b"configured_server_flags\x00" as *const u8 as *const libc::c_char,
    b"configured\x00" as *const u8 as *const libc::c_char,
];

pub unsafe fn dc_get_config(context: &dc_context_t, key: *const libc::c_char) -> *mut libc::c_char {
    let mut value = 0 as *mut libc::c_char;
    if !key.is_null()
        && *key.offset(0isize) as libc::c_int == 's' as i32
        && *key.offset(1isize) as libc::c_int == 'y' as i32
        && *key.offset(2isize) as libc::c_int == 's' as i32
        && *key.offset(3isize) as libc::c_int == '.' as i32
    {
        return get_sys_config_str(key);
    }

    if key.is_null() || 0 == is_gettable_config_key(key) {
        return dc_strdup(b"\x00" as *const u8 as *const libc::c_char);
    }

    if strcmp(key, b"selfavatar\x00" as *const u8 as *const libc::c_char) == 0 {
        let mut rel_path: *mut libc::c_char = dc_sqlite3_get_config(
            context,
            &context.sql.clone().read().unwrap(),
            key,
            0 as *const libc::c_char,
        );
        if !rel_path.is_null() {
            value = dc_get_abs_path(context, rel_path);
            free(rel_path as *mut libc::c_void);
        }
    } else {
        value = dc_sqlite3_get_config(
            context,
            &context.sql.clone().read().unwrap(),
            key,
            0 as *const libc::c_char,
        )
    }

    if value.is_null() {
        if strcmp(key, b"e2ee_enabled\x00" as *const u8 as *const libc::c_char) == 0 {
            value = dc_mprintf(b"%i\x00" as *const u8 as *const libc::c_char, 1)
        } else if strcmp(key, b"mdns_enabled\x00" as *const u8 as *const libc::c_char) == 0 {
            value = dc_mprintf(b"%i\x00" as *const u8 as *const libc::c_char, 1)
        } else if strcmp(key, b"imap_folder\x00" as *const u8 as *const libc::c_char) == 0 {
            value = dc_strdup(b"INBOX\x00" as *const u8 as *const libc::c_char)
        } else if strcmp(key, b"inbox_watch\x00" as *const u8 as *const libc::c_char) == 0 {
            value = dc_mprintf(b"%i\x00" as *const u8 as *const libc::c_char, 1)
        } else if strcmp(
            key,
            b"sentbox_watch\x00" as *const u8 as *const libc::c_char,
        ) == 0
        {
            value = dc_mprintf(b"%i\x00" as *const u8 as *const libc::c_char, 1)
        } else if strcmp(key, b"mvbox_watch\x00" as *const u8 as *const libc::c_char) == 0 {
            value = dc_mprintf(b"%i\x00" as *const u8 as *const libc::c_char, 1)
        } else if strcmp(key, b"mvbox_move\x00" as *const u8 as *const libc::c_char) == 0 {
            value = dc_mprintf(b"%i\x00" as *const u8 as *const libc::c_char, 1)
        } else if strcmp(key, b"show_emails\x00" as *const u8 as *const libc::c_char) == 0 {
            value = dc_mprintf(b"%i\x00" as *const u8 as *const libc::c_char, 0)
        } else if strcmp(key, b"selfstatus\x00" as *const u8 as *const libc::c_char) == 0 {
            value = dc_stock_str(context, 13)
        } else {
            value = dc_mprintf(b"\x00" as *const u8 as *const libc::c_char)
        }
    }

    value
}

unsafe fn is_gettable_config_key(key: *const libc::c_char) -> libc::c_int {
    let mut i = 0;
    while i
        < (::std::mem::size_of::<[*const libc::c_char; 3]>())
            .wrapping_div(::std::mem::size_of::<*mut libc::c_char>())
    {
        if strcmp(key, sys_config_keys[i as usize]) == 0 {
            return 1;
        }
        i += 1
    }

    is_settable_config_key(key)
}

// deprecated
static mut sys_config_keys: [*const libc::c_char; 3] = [
    b"sys.version\x00" as *const u8 as *const libc::c_char,
    b"sys.msgsize_max_recommended\x00" as *const u8 as *const libc::c_char,
    b"sys.config_keys\x00" as *const u8 as *const libc::c_char,
];

unsafe fn get_sys_config_str(key: *const libc::c_char) -> *mut libc::c_char {
    if strcmp(key, b"sys.version\x00" as *const u8 as *const libc::c_char) == 0 {
        return dc_strdup(VERSION as *const u8 as *const libc::c_char);
    } else if strcmp(
        key,
        b"sys.msgsize_max_recommended\x00" as *const u8 as *const libc::c_char,
    ) == 0
    {
        return dc_mprintf(
            b"%i\x00" as *const u8 as *const libc::c_char,
            24 * 1024 * 1024 / 4 * 3,
        );
    } else if strcmp(
        key,
        b"sys.config_keys\x00" as *const u8 as *const libc::c_char,
    ) == 0
    {
        return get_config_keys_str();
    } else {
        return dc_strdup(0 as *const libc::c_char);
    };
}

unsafe fn get_config_keys_str() -> *mut libc::c_char {
    let mut ret = dc_strbuilder_t {
        buf: std::ptr::null_mut(),
        allocated: 0,
        free: 0,
        eos: std::ptr::null_mut(),
    };
    dc_strbuilder_init(&mut ret, 0);

    let mut i = 0;
    while i
        < (::std::mem::size_of::<[*const libc::c_char; 33]>())
            .wrapping_div(::std::mem::size_of::<*mut libc::c_char>())
    {
        if strlen(ret.buf) > 0 {
            dc_strbuilder_cat(&mut ret, b" \x00" as *const u8 as *const libc::c_char);
        }
        dc_strbuilder_cat(&mut ret, config_keys[i as usize]);
        i += 1
    }

    let mut i = 0;
    while i
        < (::std::mem::size_of::<[*const libc::c_char; 3]>())
            .wrapping_div(::std::mem::size_of::<*mut libc::c_char>())
    {
        if strlen(ret.buf) > 0 {
            dc_strbuilder_cat(&mut ret, b" \x00" as *const u8 as *const libc::c_char);
        }
        dc_strbuilder_cat(&mut ret, sys_config_keys[i as usize]);
        i += 1
    }

    ret.buf
}

pub unsafe fn dc_get_info(context: &dc_context_t) -> *mut libc::c_char {
    let mut unset = b"0\x00" as *const u8 as *const libc::c_char;
    let mut displayname;
    let mut temp;
    let mut l_readable_str;
    let mut l2_readable_str;
    let mut fingerprint_str;
    let mut l;
    let mut l2;
    let mut inbox_watch;
    let mut sentbox_watch;
    let mut mvbox_watch;
    let mut mvbox_move;
    let mut folders_configured;
    let mut configured_sentbox_folder;
    let mut configured_mvbox_folder;
    let mut contacts;
    let mut chats;
    let mut real_msgs;
    let mut deaddrop_msgs;
    let mut is_configured;
    let mut dbversion;
    let mut mdns_enabled;
    let mut e2ee_enabled;
    let mut prv_key_cnt;
    let mut pub_key_cnt;
    let mut self_public = dc_key_new();
    let mut rpgp_enabled = 1;

    let mut ret = dc_strbuilder_t {
        buf: 0 as *mut libc::c_char,
        allocated: 0,
        free: 0,
        eos: 0 as *mut libc::c_char,
    };
    dc_strbuilder_init(&mut ret, 0);
    l = dc_loginparam_new();
    l2 = dc_loginparam_new();
    dc_loginparam_read(
        context,
        l,
        &context.sql.clone().read().unwrap(),
        b"\x00" as *const u8 as *const libc::c_char,
    );
    dc_loginparam_read(
        context,
        l2,
        &context.sql.clone().read().unwrap(),
        b"configured_\x00" as *const u8 as *const libc::c_char,
    );
    displayname = dc_sqlite3_get_config(
        context,
        &context.sql.clone().read().unwrap(),
        b"displayname\x00" as *const u8 as *const libc::c_char,
        0 as *const libc::c_char,
    );
    chats = dc_get_chat_cnt(context) as libc::c_int;
    real_msgs = dc_get_real_msg_cnt(context) as libc::c_int;
    deaddrop_msgs = dc_get_deaddrop_msg_cnt(context) as libc::c_int;
    contacts = dc_get_real_contact_cnt(context) as libc::c_int;
    is_configured = dc_sqlite3_get_config_int(
        context,
        &context.sql.clone().read().unwrap(),
        b"configured\x00" as *const u8 as *const libc::c_char,
        0,
    );
    dbversion = dc_sqlite3_get_config_int(
        context,
        &context.sql.clone().read().unwrap(),
        b"dbversion\x00" as *const u8 as *const libc::c_char,
        0,
    );
    e2ee_enabled = dc_sqlite3_get_config_int(
        context,
        &context.sql.clone().read().unwrap(),
        b"e2ee_enabled\x00" as *const u8 as *const libc::c_char,
        1,
    );
    mdns_enabled = dc_sqlite3_get_config_int(
        context,
        &context.sql.clone().read().unwrap(),
        b"mdns_enabled\x00" as *const u8 as *const libc::c_char,
        1,
    );
    let mut stmt: *mut sqlite3_stmt = dc_sqlite3_prepare(
        context,
        &context.sql.clone().read().unwrap(),
        b"SELECT COUNT(*) FROM keypairs;\x00" as *const u8 as *const libc::c_char,
    );
    sqlite3_step(stmt);
    prv_key_cnt = sqlite3_column_int(stmt, 0);
    sqlite3_finalize(stmt);
    stmt = dc_sqlite3_prepare(
        context,
        &context.sql.clone().read().unwrap(),
        b"SELECT COUNT(*) FROM acpeerstates;\x00" as *const u8 as *const libc::c_char,
    );
    sqlite3_step(stmt);
    pub_key_cnt = sqlite3_column_int(stmt, 0);
    sqlite3_finalize(stmt);
    if 0 != dc_key_load_self_public(
        context,
        self_public,
        (*l2).addr,
        &context.sql.clone().read().unwrap(),
    ) {
        fingerprint_str = dc_key_get_fingerprint(context, self_public)
    } else {
        fingerprint_str = dc_strdup(b"<Not yet calculated>\x00" as *const u8 as *const libc::c_char)
    }
    l_readable_str = dc_loginparam_get_readable(l);
    l2_readable_str = dc_loginparam_get_readable(l2);
    inbox_watch = dc_sqlite3_get_config_int(
        context,
        &context.sql.clone().read().unwrap(),
        b"inbox_watch\x00" as *const u8 as *const libc::c_char,
        1,
    );
    sentbox_watch = dc_sqlite3_get_config_int(
        context,
        &context.sql.clone().read().unwrap(),
        b"sentbox_watch\x00" as *const u8 as *const libc::c_char,
        1,
    );
    mvbox_watch = dc_sqlite3_get_config_int(
        context,
        &context.sql.clone().read().unwrap(),
        b"mvbox_watch\x00" as *const u8 as *const libc::c_char,
        1,
    );
    mvbox_move = dc_sqlite3_get_config_int(
        context,
        &context.sql.clone().read().unwrap(),
        b"mvbox_move\x00" as *const u8 as *const libc::c_char,
        1,
    );
    folders_configured = dc_sqlite3_get_config_int(
        context,
        &context.sql.clone().read().unwrap(),
        b"folders_configured\x00" as *const u8 as *const libc::c_char,
        0,
    );
    configured_sentbox_folder = dc_sqlite3_get_config(
        context,
        &context.sql.clone().read().unwrap(),
        b"configured_sentbox_folder\x00" as *const u8 as *const libc::c_char,
        b"<unset>\x00" as *const u8 as *const libc::c_char,
    );
    configured_mvbox_folder = dc_sqlite3_get_config(
        context,
        &context.sql.clone().read().unwrap(),
        b"configured_mvbox_folder\x00" as *const u8 as *const libc::c_char,
        b"<unset>\x00" as *const u8 as *const libc::c_char,
    );

    temp = dc_mprintf(
        b"deltachat_core_version=v%s\n\
          sqlite_version=%s\n\
          sqlite_thread_safe=%i\n\
          libetpan_version=%i.%i\n\
          openssl_version=%i.%i.%i%c\n\
          rpgp_enabled=%i\n\
          compile_date=Apr 26 2019, 00:51:50\n\
          arch=%i\n\
          number_of_chats=%i\n\
          number_of_chat_messages=%i\n\
          messages_in_contact_requests=%i\n\
          number_of_contacts=%i\n\
          database_dir=%s\n\
          database_version=%i\n\
          blobdir=%s\n\
          display_name=%s\n\
          is_configured=%i\n\
          entered_account_settings=%s\n\
          used_account_settings=%s\n\
          inbox_watch=%i\n\
          sentbox_watch=%i\n\
          mvbox_watch=%i\n\
          mvbox_move=%i\n\
          folders_configured=%i\n\
          configured_sentbox_folder=%s\n\
          configured_mvbox_folder=%s\n\
          mdns_enabled=%i\n\
          e2ee_enabled=%i\n\
          private_key_count=%i\n\
          public_key_count=%i\n\
          fingerprint=%s\n\x00" as *const u8 as *const libc::c_char,
        VERSION as *const u8 as *const libc::c_char,
        libsqlite3_sys::SQLITE_VERSION as *const u8 as *const libc::c_char,
        sqlite3_threadsafe(),
        // no libetpan
        0,
        0,
        // openssl (none used, so setting to 0)
        0 as libc::c_int,
        0 as libc::c_int,
        0 as libc::c_int,
        'a' as libc::c_char as libc::c_int,
        rpgp_enabled,
        // arch
        (::std::mem::size_of::<*mut libc::c_void>()).wrapping_mul(8),
        chats,
        real_msgs,
        deaddrop_msgs,
        contacts,
        if context.has_dbfile() {
            context.get_dbfile()
        } else {
            unset
        },
        dbversion,
        if context.has_blobdir() {
            context.get_blobdir()
        } else {
            unset
        },
        if !displayname.is_null() {
            displayname
        } else {
            unset
        },
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
        prv_key_cnt,
        pub_key_cnt,
        fingerprint_str,
    );

    dc_strbuilder_cat(&mut ret, temp);
    free(temp as *mut libc::c_void);
    dc_loginparam_unref(l);
    dc_loginparam_unref(l2);
    free(displayname as *mut libc::c_void);
    free(l_readable_str as *mut libc::c_void);
    free(l2_readable_str as *mut libc::c_void);
    free(configured_sentbox_folder as *mut libc::c_void);
    free(configured_mvbox_folder as *mut libc::c_void);
    free(fingerprint_str as *mut libc::c_void);
    dc_key_unref(self_public);

    ret.buf
}

pub unsafe fn dc_get_version_str() -> *mut libc::c_char {
    dc_strdup(VERSION as *const u8 as *const libc::c_char)
}

pub unsafe fn dc_get_fresh_msgs(context: &dc_context_t) -> *mut dc_array_t {
    let mut show_deaddrop = 0;
    let mut ret = dc_array_new(128 as size_t);
    let mut stmt = 0 as *mut sqlite3_stmt;
    if !ret.is_null() {
        stmt = dc_sqlite3_prepare(
            context,
            &context.sql.clone().read().unwrap(),
            b"SELECT m.id FROM msgs m LEFT JOIN contacts ct \
              ON m.from_id=ct.id LEFT JOIN chats c ON m.chat_id=c.id WHERE m.state=?   \
              AND m.hidden=0   \
              AND m.chat_id>?   \
              AND ct.blocked=0   \
              AND (c.blocked=0 OR c.blocked=?) ORDER BY m.timestamp DESC,m.id DESC;\x00"
                as *const u8 as *const libc::c_char,
        );
        sqlite3_bind_int(stmt, 1, 10);
        sqlite3_bind_int(stmt, 2, 9);
        sqlite3_bind_int(stmt, 3, if 0 != show_deaddrop { 2 } else { 0 });
        while sqlite3_step(stmt) == 100 {
            dc_array_add_id(ret, sqlite3_column_int(stmt, 0) as uint32_t);
        }
    }
    sqlite3_finalize(stmt);
    ret
}

pub unsafe fn dc_search_msgs(
    context: &dc_context_t,
    chat_id: uint32_t,
    query: *const libc::c_char,
) -> *mut dc_array_t {
    let mut success = 0;
    let mut ret = dc_array_new(100 as size_t);
    let mut strLikeInText = 0 as *mut libc::c_char;
    let mut strLikeBeg = 0 as *mut libc::c_char;
    let mut real_query = 0 as *mut libc::c_char;
    let mut stmt = 0 as *mut sqlite3_stmt;

    if !(ret.is_null() || query.is_null()) {
        real_query = dc_strdup(query);
        dc_trim(real_query);
        if *real_query.offset(0isize) as libc::c_int == 0 {
            success = 1
        } else {
            strLikeInText = dc_mprintf(
                b"%%%s%%\x00" as *const u8 as *const libc::c_char,
                real_query,
            );
            strLikeBeg = dc_mprintf(b"%s%%\x00" as *const u8 as *const libc::c_char, real_query);
            if 0 != chat_id {
                stmt = dc_sqlite3_prepare(
                    context,
                    &context.sql.clone().read().unwrap(),
                    b"SELECT m.id, m.timestamp FROM msgs m LEFT JOIN contacts ct ON m.from_id=ct.id WHERE m.chat_id=?  \
                      AND m.hidden=0  \
                      AND ct.blocked=0 AND (txt LIKE ? OR ct.name LIKE ?) ORDER BY m.timestamp,m.id;\x00"
                        as *const u8 as *const libc::c_char
                );
                sqlite3_bind_int(stmt, 1, chat_id as libc::c_int);
                sqlite3_bind_text(stmt, 2, strLikeInText, -1, None);
                sqlite3_bind_text(stmt, 3, strLikeBeg, -1, None);
            } else {
                let mut show_deaddrop = 0;
                stmt = dc_sqlite3_prepare(
                    context,
                    &context.sql.clone().read().unwrap(),
                    b"SELECT m.id, m.timestamp FROM msgs m LEFT JOIN contacts ct ON m.from_id=ct.id \
                      LEFT JOIN chats c ON m.chat_id=c.id WHERE m.chat_id>9 AND m.hidden=0  \
                      AND (c.blocked=0 OR c.blocked=?) \
                      AND ct.blocked=0 AND (m.txt LIKE ? OR ct.name LIKE ?) ORDER BY m.timestamp DESC,m.id DESC;\x00"
                        as *const u8 as *const libc::c_char
                );
                sqlite3_bind_int(stmt, 1, if 0 != show_deaddrop { 2 } else { 0 });
                sqlite3_bind_text(stmt, 2, strLikeInText, -1, None);
                sqlite3_bind_text(stmt, 3, strLikeBeg, -1, None);
            }
            while sqlite3_step(stmt) == 100 {
                dc_array_add_id(ret, sqlite3_column_int(stmt, 0) as uint32_t);
            }
            success = 1
        }
    }

    free(strLikeInText as *mut libc::c_void);
    free(strLikeBeg as *mut libc::c_void);
    free(real_query as *mut libc::c_void);
    sqlite3_finalize(stmt);

    if 0 != success {
        ret
    } else {
        if !ret.is_null() {
            dc_array_unref(ret);
        }
        0 as *mut dc_array_t
    }
}

pub unsafe fn dc_is_inbox(
    _context: &dc_context_t,
    folder_name: *const libc::c_char,
) -> libc::c_int {
    let mut is_inbox = 0;
    if !folder_name.is_null() {
        is_inbox = if strcasecmp(
            b"INBOX\x00" as *const u8 as *const libc::c_char,
            folder_name,
        ) == 0
        {
            1
        } else {
            0
        }
    }
    is_inbox
}

pub unsafe fn dc_is_sentbox(
    context: &dc_context_t,
    folder_name: *const libc::c_char,
) -> libc::c_int {
    let mut sentbox_name = dc_sqlite3_get_config(
        context,
        &context.sql.clone().read().unwrap(),
        b"configured_sentbox_folder\x00" as *const u8 as *const libc::c_char,
        0 as *const libc::c_char,
    );
    let mut is_sentbox = 0;
    if !sentbox_name.is_null() && !folder_name.is_null() {
        is_sentbox = if strcasecmp(sentbox_name, folder_name) == 0 {
            1
        } else {
            0
        }
    }
    free(sentbox_name as *mut libc::c_void);
    is_sentbox
}

pub unsafe fn dc_is_mvbox(context: &dc_context_t, folder_name: *const libc::c_char) -> libc::c_int {
    let mut mvbox_name = dc_sqlite3_get_config(
        context,
        &context.sql.clone().read().unwrap(),
        b"configured_mvbox_folder\x00" as *const u8 as *const libc::c_char,
        0 as *const libc::c_char,
    );
    let mut is_mvbox = 0;
    if !mvbox_name.is_null() && !folder_name.is_null() {
        is_mvbox = if strcasecmp(mvbox_name, folder_name) == 0 {
            1
        } else {
            0
        }
    }
    free(mvbox_name as *mut libc::c_void);
    is_mvbox
}
