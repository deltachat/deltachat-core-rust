use c2rust_bitfields::BitfieldStruct;
use libc;

use crate::constants::{Event, VERSION};
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
use crate::dc_pgp::*;
use crate::dc_receive_imf::*;
use crate::dc_smtp::*;
use crate::dc_sqlite3::*;
use crate::dc_stock::*;
use crate::dc_strbuilder::*;
use crate::dc_tools::*;
use crate::types::*;
use crate::x::*;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct dc_context_t {
    pub magic: uint32_t,
    pub userdata: *mut libc::c_void,
    pub dbfile: *mut libc::c_char,
    pub blobdir: *mut libc::c_char,
    pub sql: *mut dc_sqlite3_t,
    pub inbox: *mut dc_imap_t,
    pub inboxidle_condmutex: pthread_mutex_t,
    pub perform_inbox_jobs_needed: libc::c_int,
    pub probe_imap_network: libc::c_int,
    pub sentbox_thread: dc_jobthread_t,
    pub mvbox_thread: dc_jobthread_t,
    pub smtp: *mut dc_smtp_t,
    pub smtpidle_cond: pthread_cond_t,
    pub smtpidle_condmutex: pthread_mutex_t,
    pub smtpidle_condflag: libc::c_int,
    pub smtp_suspended: libc::c_int,
    pub smtp_doing_jobs: libc::c_int,
    pub perform_smtp_jobs_needed: libc::c_int,
    pub probe_smtp_network: libc::c_int,
    pub oauth2_critical: pthread_mutex_t,
    pub cb: dc_callback_t,
    pub os_name: *mut libc::c_char,
    pub cmdline_sel_chat_id: uint32_t,
    pub bob_expects: libc::c_int,
    pub bobs_status: libc::c_int,
    pub bobs_qr_scan: *mut dc_lot_t,
    pub bobs_qr_critical: pthread_mutex_t,
    pub last_smeared_timestamp: time_t,
    pub smear_critical: pthread_mutex_t,
    pub ongoing_running: libc::c_int,
    pub shall_stop_ongoing: libc::c_int,
}

unsafe impl Send for dc_context_t {}
unsafe impl Sync for dc_context_t {}

// create/open/config/information
pub unsafe fn dc_context_new(
    mut cb: dc_callback_t,
    mut userdata: *mut libc::c_void,
    mut os_name: *const libc::c_char,
) -> *mut dc_context_t {
    let mut context: *mut dc_context_t = 0 as *mut dc_context_t;
    context = calloc(
        1i32 as libc::c_ulong,
        ::std::mem::size_of::<dc_context_t>() as libc::c_ulong,
    ) as *mut dc_context_t;
    if context.is_null() {
        exit(23i32);
    }
    pthread_mutex_init(
        &mut (*context).smear_critical,
        0 as *const pthread_mutexattr_t,
    );
    pthread_mutex_init(
        &mut (*context).bobs_qr_critical,
        0 as *const pthread_mutexattr_t,
    );
    pthread_mutex_init(
        &mut (*context).inboxidle_condmutex,
        0 as *const pthread_mutexattr_t,
    );
    dc_jobthread_init(
        &mut (*context).sentbox_thread,
        context,
        b"SENTBOX\x00" as *const u8 as *const libc::c_char,
        b"configured_sentbox_folder\x00" as *const u8 as *const libc::c_char,
    );
    dc_jobthread_init(
        &mut (*context).mvbox_thread,
        context,
        b"MVBOX\x00" as *const u8 as *const libc::c_char,
        b"configured_mvbox_folder\x00" as *const u8 as *const libc::c_char,
    );
    pthread_mutex_init(
        &mut (*context).smtpidle_condmutex,
        0 as *const pthread_mutexattr_t,
    );
    pthread_cond_init(
        &mut (*context).smtpidle_cond,
        0 as *const pthread_condattr_t,
    );
    pthread_mutex_init(
        &mut (*context).oauth2_critical,
        0 as *const pthread_mutexattr_t,
    );
    (*context).magic = 0x11a11807i32 as uint32_t;
    (*context).userdata = userdata;
    (*context).cb = if cb.is_some() { cb } else { Some(cb_dummy) };
    (*context).os_name = dc_strdup_keep_null(os_name);
    (*context).shall_stop_ongoing = 1i32;
    // dc_openssl_init();
    dc_pgp_init();
    (*context).sql = dc_sqlite3_new(context);
    (*context).inbox = dc_imap_new(
        Some(cb_get_config),
        Some(cb_set_config),
        Some(cb_precheck_imf),
        Some(cb_receive_imf),
        context as *mut libc::c_void,
        context,
    );
    (*context).sentbox_thread.imap = dc_imap_new(
        Some(cb_get_config),
        Some(cb_set_config),
        Some(cb_precheck_imf),
        Some(cb_receive_imf),
        context as *mut libc::c_void,
        context,
    );
    (*context).mvbox_thread.imap = dc_imap_new(
        Some(cb_get_config),
        Some(cb_set_config),
        Some(cb_precheck_imf),
        Some(cb_receive_imf),
        context as *mut libc::c_void,
        context,
    );
    (*context).smtp = dc_smtp_new(context);
    /* Random-seed.  An additional seed with more random data is done just before key generation
    (the timespan between this call and the key generation time is typically random.
    Moreover, later, we add a hash of the first message data to the random-seed
    (it would be okay to seed with even more sensible data, the seed values cannot be recovered from the PRNG output, see OpenSSL's RAND_seed()) */
    let mut seed: [uintptr_t; 5] = [0; 5];
    seed[0usize] = time(0 as *mut time_t) as uintptr_t;
    seed[1usize] = seed.as_mut_ptr() as uintptr_t;
    seed[2usize] = context as uintptr_t;
    seed[3usize] = pthread_self() as uintptr_t;
    seed[4usize] = libc::getpid() as uintptr_t;
    dc_pgp_rand_seed(
        context,
        seed.as_mut_ptr() as *const libc::c_void,
        ::std::mem::size_of::<[uintptr_t; 5]>() as libc::c_ulong,
    );
    return context;
}
unsafe fn cb_receive_imf(
    mut imap: *mut dc_imap_t,
    mut imf_raw_not_terminated: *const libc::c_char,
    mut imf_raw_bytes: size_t,
    mut server_folder: *const libc::c_char,
    mut server_uid: uint32_t,
    mut flags: uint32_t,
) {
    let mut context: *mut dc_context_t = (*imap).userData as *mut dc_context_t;
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
    mut imap: *mut dc_imap_t,
    mut rfc724_mid: *const libc::c_char,
    mut server_folder: *const libc::c_char,
    mut server_uid: uint32_t,
) -> libc::c_int {
    let mut rfc724_mid_exists: libc::c_int = 0i32;
    let mut msg_id: uint32_t = 0i32 as uint32_t;
    let mut old_server_folder: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut old_server_uid: uint32_t = 0i32 as uint32_t;
    let mut mark_seen: libc::c_int = 0i32;
    msg_id = dc_rfc724_mid_exists(
        (*imap).context,
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
                (*imap).context,
                0i32,
                b"[move] detected bbc-self %s\x00" as *const u8 as *const libc::c_char,
                rfc724_mid,
            );
            mark_seen = 1i32
        } else if strcmp(old_server_folder, server_folder) != 0i32 {
            dc_log_info(
                (*imap).context,
                0i32,
                b"[move] detected moved message %s\x00" as *const u8 as *const libc::c_char,
                rfc724_mid,
            );
            dc_update_msg_move_state((*imap).context, rfc724_mid, DC_MOVE_STATE_STAY);
        }
        if strcmp(old_server_folder, server_folder) != 0i32 || old_server_uid != server_uid {
            dc_update_server_uid((*imap).context, rfc724_mid, server_folder, server_uid);
        }
        dc_do_heuristics_moves((*imap).context, server_folder, msg_id);
        if 0 != mark_seen {
            dc_job_add(
                (*imap).context,
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
    mut imap: *mut dc_imap_t,
    mut key: *const libc::c_char,
    mut value: *const libc::c_char,
) {
    let mut context: *mut dc_context_t = (*imap).userData as *mut dc_context_t;
    dc_sqlite3_set_config((*context).sql, key, value);
}
/* *
 * The following three callback are given to dc_imap_new() to read/write configuration
 * and to handle received messages. As the imap-functions are typically used in
 * a separate user-thread, also these functions may be called from a different thread.
 *
 * @private @memberof dc_context_t
 */
unsafe fn cb_get_config(
    mut imap: *mut dc_imap_t,
    mut key: *const libc::c_char,
    mut def: *const libc::c_char,
) -> *mut libc::c_char {
    let mut context: *mut dc_context_t = (*imap).userData as *mut dc_context_t;
    return dc_sqlite3_get_config((*context).sql, key, def);
}
/* *
 * A callback function that is used if no user-defined callback is given to dc_context_new().
 * The callback function simply returns 0 which is safe for every event.
 *
 * @private @memberof dc_context_t
 */
unsafe fn cb_dummy(
    mut context: *mut dc_context_t,
    mut event: Event,
    mut data1: uintptr_t,
    mut data2: uintptr_t,
) -> uintptr_t {
    return 0i32 as uintptr_t;
}
pub unsafe fn dc_context_unref(mut context: *mut dc_context_t) {
    if context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint {
        return;
    }
    dc_pgp_exit();
    if 0 != dc_is_open(context) {
        dc_close(context);
    }
    dc_imap_unref((*context).inbox);
    dc_imap_unref((*context).sentbox_thread.imap);
    dc_imap_unref((*context).mvbox_thread.imap);
    dc_smtp_unref((*context).smtp);
    dc_sqlite3_unref((*context).sql);

    pthread_mutex_destroy(&mut (*context).smear_critical);
    pthread_mutex_destroy(&mut (*context).bobs_qr_critical);
    pthread_mutex_destroy(&mut (*context).inboxidle_condmutex);
    dc_jobthread_exit(&mut (*context).sentbox_thread);
    dc_jobthread_exit(&mut (*context).mvbox_thread);
    pthread_cond_destroy(&mut (*context).smtpidle_cond);
    pthread_mutex_destroy(&mut (*context).smtpidle_condmutex);
    pthread_mutex_destroy(&mut (*context).oauth2_critical);
    free((*context).os_name as *mut libc::c_void);
    (*context).magic = 0i32 as uint32_t;
    free(context as *mut libc::c_void);
}
pub unsafe fn dc_close(mut context: *mut dc_context_t) {
    if context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint {
        return;
    }
    dc_imap_disconnect((*context).inbox);
    dc_imap_disconnect((*context).sentbox_thread.imap);
    dc_imap_disconnect((*context).mvbox_thread.imap);
    dc_smtp_disconnect((*context).smtp);
    if 0 != dc_sqlite3_is_open((*context).sql) {
        dc_sqlite3_close((*context).sql);
    }
    free((*context).dbfile as *mut libc::c_void);
    (*context).dbfile = 0 as *mut libc::c_char;
    free((*context).blobdir as *mut libc::c_void);
    (*context).blobdir = 0 as *mut libc::c_char;
}
pub unsafe fn dc_is_open(mut context: *const dc_context_t) -> libc::c_int {
    if context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint {
        return 0i32;
    }
    return dc_sqlite3_is_open((*context).sql);
}
pub unsafe fn dc_get_userdata(mut context: *mut dc_context_t) -> *mut libc::c_void {
    if context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint {
        return 0 as *mut libc::c_void;
    }
    return (*context).userdata;
}
pub unsafe fn dc_open(
    mut context: *mut dc_context_t,
    mut dbfile: *const libc::c_char,
    mut blobdir: *const libc::c_char,
) -> libc::c_int {
    let mut success: libc::c_int = 0i32;
    if 0 != dc_is_open(context) {
        return 0i32;
    }
    if !(context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint || dbfile.is_null())
    {
        (*context).dbfile = dc_strdup(dbfile);
        if !blobdir.is_null() && 0 != *blobdir.offset(0isize) as libc::c_int {
            (*context).blobdir = dc_strdup(blobdir);
            dc_ensure_no_slash((*context).blobdir);
        } else {
            (*context).blobdir =
                dc_mprintf(b"%s-blobs\x00" as *const u8 as *const libc::c_char, dbfile);
            dc_create_folder(context, (*context).blobdir);
        }
        /* Create/open sqlite database, this may already use the blobdir */
        if !(0 == dc_sqlite3_open((*context).sql, dbfile, 0i32)) {
            success = 1i32
        }
    }
    if 0 == success {
        dc_close(context);
    }
    return success;
}
pub unsafe fn dc_get_blobdir(mut context: *const dc_context_t) -> *mut libc::c_char {
    if context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint {
        return dc_strdup(0 as *const libc::c_char);
    }
    return dc_strdup((*context).blobdir);
}
pub unsafe fn dc_set_config(
    mut context: *mut dc_context_t,
    mut key: *const libc::c_char,
    mut value: *const libc::c_char,
) -> libc::c_int {
    let mut ret: libc::c_int = 0i32;
    let mut rel_path: *mut libc::c_char = 0 as *mut libc::c_char;
    if context.is_null()
        || (*context).magic != 0x11a11807i32 as libc::c_uint
        || key.is_null()
        || 0 == is_settable_config_key(key)
    {
        return 0i32;
    }
    if strcmp(key, b"selfavatar\x00" as *const u8 as *const libc::c_char) == 0i32
        && !value.is_null()
    {
        rel_path = dc_strdup(value);
        if !(0 == dc_make_rel_and_copy(context, &mut rel_path)) {
            ret = dc_sqlite3_set_config((*context).sql, key, rel_path)
        }
    } else if strcmp(key, b"inbox_watch\x00" as *const u8 as *const libc::c_char) == 0i32 {
        ret = dc_sqlite3_set_config((*context).sql, key, value);
        dc_interrupt_imap_idle(context);
    } else if strcmp(
        key,
        b"sentbox_watch\x00" as *const u8 as *const libc::c_char,
    ) == 0i32
    {
        ret = dc_sqlite3_set_config((*context).sql, key, value);
        dc_interrupt_sentbox_idle(context);
    } else if strcmp(key, b"mvbox_watch\x00" as *const u8 as *const libc::c_char) == 0i32 {
        ret = dc_sqlite3_set_config((*context).sql, key, value);
        dc_interrupt_mvbox_idle(context);
    } else if strcmp(key, b"selfstatus\x00" as *const u8 as *const libc::c_char) == 0i32 {
        let mut def: *mut libc::c_char = dc_stock_str(context, 13i32);
        ret = dc_sqlite3_set_config(
            (*context).sql,
            key,
            if value.is_null() || strcmp(value, def) == 0i32 {
                0 as *const libc::c_char
            } else {
                value
            },
        );
        free(def as *mut libc::c_void);
    } else {
        ret = dc_sqlite3_set_config((*context).sql, key, value)
    }
    free(rel_path as *mut libc::c_void);
    return ret;
}
/* ******************************************************************************
 * INI-handling, Information
 ******************************************************************************/
unsafe fn is_settable_config_key(mut key: *const libc::c_char) -> libc::c_int {
    let mut i: libc::c_int = 0i32;
    while (i as libc::c_ulong)
        < (::std::mem::size_of::<[*const libc::c_char; 33]>() as libc::c_ulong)
            .wrapping_div(::std::mem::size_of::<*mut libc::c_char>() as libc::c_ulong)
    {
        if strcmp(key, config_keys[i as usize]) == 0i32 {
            return 1i32;
        }
        i += 1
    }
    return 0i32;
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
pub unsafe fn dc_get_config(
    mut context: *mut dc_context_t,
    mut key: *const libc::c_char,
) -> *mut libc::c_char {
    let mut value: *mut libc::c_char = 0 as *mut libc::c_char;
    if !key.is_null()
        && *key.offset(0isize) as libc::c_int == 's' as i32
        && *key.offset(1isize) as libc::c_int == 'y' as i32
        && *key.offset(2isize) as libc::c_int == 's' as i32
        && *key.offset(3isize) as libc::c_int == '.' as i32
    {
        return get_sys_config_str(key);
    }
    if context.is_null()
        || (*context).magic != 0x11a11807i32 as libc::c_uint
        || key.is_null()
        || 0 == is_gettable_config_key(key)
    {
        return dc_strdup(b"\x00" as *const u8 as *const libc::c_char);
    }
    if strcmp(key, b"selfavatar\x00" as *const u8 as *const libc::c_char) == 0i32 {
        let mut rel_path: *mut libc::c_char =
            dc_sqlite3_get_config((*context).sql, key, 0 as *const libc::c_char);
        if !rel_path.is_null() {
            value = dc_get_abs_path(context, rel_path);
            free(rel_path as *mut libc::c_void);
        }
    } else {
        value = dc_sqlite3_get_config((*context).sql, key, 0 as *const libc::c_char)
    }
    if value.is_null() {
        if strcmp(key, b"e2ee_enabled\x00" as *const u8 as *const libc::c_char) == 0i32 {
            value = dc_mprintf(b"%i\x00" as *const u8 as *const libc::c_char, 1i32)
        } else if strcmp(key, b"mdns_enabled\x00" as *const u8 as *const libc::c_char) == 0i32 {
            value = dc_mprintf(b"%i\x00" as *const u8 as *const libc::c_char, 1i32)
        } else if strcmp(key, b"imap_folder\x00" as *const u8 as *const libc::c_char) == 0i32 {
            value = dc_strdup(b"INBOX\x00" as *const u8 as *const libc::c_char)
        } else if strcmp(key, b"inbox_watch\x00" as *const u8 as *const libc::c_char) == 0i32 {
            value = dc_mprintf(b"%i\x00" as *const u8 as *const libc::c_char, 1i32)
        } else if strcmp(
            key,
            b"sentbox_watch\x00" as *const u8 as *const libc::c_char,
        ) == 0i32
        {
            value = dc_mprintf(b"%i\x00" as *const u8 as *const libc::c_char, 1i32)
        } else if strcmp(key, b"mvbox_watch\x00" as *const u8 as *const libc::c_char) == 0i32 {
            value = dc_mprintf(b"%i\x00" as *const u8 as *const libc::c_char, 1i32)
        } else if strcmp(key, b"mvbox_move\x00" as *const u8 as *const libc::c_char) == 0i32 {
            value = dc_mprintf(b"%i\x00" as *const u8 as *const libc::c_char, 1i32)
        } else if strcmp(key, b"show_emails\x00" as *const u8 as *const libc::c_char) == 0i32 {
            value = dc_mprintf(b"%i\x00" as *const u8 as *const libc::c_char, 0i32)
        } else if strcmp(key, b"selfstatus\x00" as *const u8 as *const libc::c_char) == 0i32 {
            value = dc_stock_str(context, 13i32)
        } else {
            value = dc_mprintf(b"\x00" as *const u8 as *const libc::c_char)
        }
    }
    return value;
}
unsafe fn is_gettable_config_key(mut key: *const libc::c_char) -> libc::c_int {
    let mut i: libc::c_int = 0i32;
    while (i as libc::c_ulong)
        < (::std::mem::size_of::<[*const libc::c_char; 3]>() as libc::c_ulong)
            .wrapping_div(::std::mem::size_of::<*mut libc::c_char>() as libc::c_ulong)
    {
        if strcmp(key, sys_config_keys[i as usize]) == 0i32 {
            return 1i32;
        }
        i += 1
    }
    return is_settable_config_key(key);
}
// deprecated
static mut sys_config_keys: [*const libc::c_char; 3] = [
    b"sys.version\x00" as *const u8 as *const libc::c_char,
    b"sys.msgsize_max_recommended\x00" as *const u8 as *const libc::c_char,
    b"sys.config_keys\x00" as *const u8 as *const libc::c_char,
];
unsafe fn get_sys_config_str(mut key: *const libc::c_char) -> *mut libc::c_char {
    if strcmp(key, b"sys.version\x00" as *const u8 as *const libc::c_char) == 0i32 {
        return dc_strdup(VERSION as *const u8 as *const libc::c_char);
    } else if strcmp(
        key,
        b"sys.msgsize_max_recommended\x00" as *const u8 as *const libc::c_char,
    ) == 0i32
    {
        return dc_mprintf(
            b"%i\x00" as *const u8 as *const libc::c_char,
            24i32 * 1024i32 * 1024i32 / 4i32 * 3i32,
        );
    } else if strcmp(
        key,
        b"sys.config_keys\x00" as *const u8 as *const libc::c_char,
    ) == 0i32
    {
        return get_config_keys_str();
    } else {
        return dc_strdup(0 as *const libc::c_char);
    };
}
unsafe fn get_config_keys_str() -> *mut libc::c_char {
    let mut ret: dc_strbuilder_t = dc_strbuilder_t {
        buf: 0 as *mut libc::c_char,
        allocated: 0,
        free: 0,
        eos: 0 as *mut libc::c_char,
    };
    dc_strbuilder_init(&mut ret, 0i32);
    let mut i: libc::c_int = 0i32;
    while (i as libc::c_ulong)
        < (::std::mem::size_of::<[*const libc::c_char; 33]>() as libc::c_ulong)
            .wrapping_div(::std::mem::size_of::<*mut libc::c_char>() as libc::c_ulong)
    {
        if strlen(ret.buf) > 0i32 as libc::c_ulong {
            dc_strbuilder_cat(&mut ret, b" \x00" as *const u8 as *const libc::c_char);
        }
        dc_strbuilder_cat(&mut ret, config_keys[i as usize]);
        i += 1
    }
    let mut i_0: libc::c_int = 0i32;
    while (i_0 as libc::c_ulong)
        < (::std::mem::size_of::<[*const libc::c_char; 3]>() as libc::c_ulong)
            .wrapping_div(::std::mem::size_of::<*mut libc::c_char>() as libc::c_ulong)
    {
        if strlen(ret.buf) > 0i32 as libc::c_ulong {
            dc_strbuilder_cat(&mut ret, b" \x00" as *const u8 as *const libc::c_char);
        }
        dc_strbuilder_cat(&mut ret, sys_config_keys[i_0 as usize]);
        i_0 += 1
    }
    return ret.buf;
}
pub unsafe fn dc_get_info(mut context: *mut dc_context_t) -> *mut libc::c_char {
    let mut unset: *const libc::c_char = b"0\x00" as *const u8 as *const libc::c_char;
    let mut displayname: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut temp: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut l_readable_str: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut l2_readable_str: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut fingerprint_str: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut l: *mut dc_loginparam_t = 0 as *mut dc_loginparam_t;
    let mut l2: *mut dc_loginparam_t = 0 as *mut dc_loginparam_t;
    let mut inbox_watch: libc::c_int = 0i32;
    let mut sentbox_watch: libc::c_int = 0i32;
    let mut mvbox_watch: libc::c_int = 0i32;
    let mut mvbox_move: libc::c_int = 0i32;
    let mut folders_configured: libc::c_int = 0i32;
    let mut configured_sentbox_folder: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut configured_mvbox_folder: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut contacts: libc::c_int = 0i32;
    let mut chats: libc::c_int = 0i32;
    let mut real_msgs: libc::c_int = 0i32;
    let mut deaddrop_msgs: libc::c_int = 0i32;
    let mut is_configured: libc::c_int = 0i32;
    let mut dbversion: libc::c_int = 0i32;
    let mut mdns_enabled: libc::c_int = 0i32;
    let mut e2ee_enabled: libc::c_int = 0i32;
    let mut prv_key_cnt: libc::c_int = 0i32;
    let mut pub_key_cnt: libc::c_int = 0i32;
    let mut self_public: *mut dc_key_t = dc_key_new();
    let mut rpgp_enabled: libc::c_int = 0i32;
    rpgp_enabled = 1i32;
    let mut ret: dc_strbuilder_t = dc_strbuilder_t {
        buf: 0 as *mut libc::c_char,
        allocated: 0,
        free: 0,
        eos: 0 as *mut libc::c_char,
    };
    dc_strbuilder_init(&mut ret, 0i32);
    if context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint {
        return dc_strdup(b"ErrBadPtr\x00" as *const u8 as *const libc::c_char);
    }
    l = dc_loginparam_new();
    l2 = dc_loginparam_new();
    dc_loginparam_read(
        l,
        (*context).sql,
        b"\x00" as *const u8 as *const libc::c_char,
    );
    dc_loginparam_read(
        l2,
        (*context).sql,
        b"configured_\x00" as *const u8 as *const libc::c_char,
    );
    displayname = dc_sqlite3_get_config(
        (*context).sql,
        b"displayname\x00" as *const u8 as *const libc::c_char,
        0 as *const libc::c_char,
    );
    chats = dc_get_chat_cnt(context) as libc::c_int;
    real_msgs = dc_get_real_msg_cnt(context) as libc::c_int;
    deaddrop_msgs = dc_get_deaddrop_msg_cnt(context) as libc::c_int;
    contacts = dc_get_real_contact_cnt(context) as libc::c_int;
    is_configured = dc_sqlite3_get_config_int(
        (*context).sql,
        b"configured\x00" as *const u8 as *const libc::c_char,
        0i32,
    );
    dbversion = dc_sqlite3_get_config_int(
        (*context).sql,
        b"dbversion\x00" as *const u8 as *const libc::c_char,
        0i32,
    );
    e2ee_enabled = dc_sqlite3_get_config_int(
        (*context).sql,
        b"e2ee_enabled\x00" as *const u8 as *const libc::c_char,
        1i32,
    );
    mdns_enabled = dc_sqlite3_get_config_int(
        (*context).sql,
        b"mdns_enabled\x00" as *const u8 as *const libc::c_char,
        1i32,
    );
    let mut stmt: *mut sqlite3_stmt = dc_sqlite3_prepare(
        (*context).sql,
        b"SELECT COUNT(*) FROM keypairs;\x00" as *const u8 as *const libc::c_char,
    );
    sqlite3_step(stmt);
    prv_key_cnt = sqlite3_column_int(stmt, 0i32);
    sqlite3_finalize(stmt);
    stmt = dc_sqlite3_prepare(
        (*context).sql,
        b"SELECT COUNT(*) FROM acpeerstates;\x00" as *const u8 as *const libc::c_char,
    );
    sqlite3_step(stmt);
    pub_key_cnt = sqlite3_column_int(stmt, 0i32);
    sqlite3_finalize(stmt);
    if 0 != dc_key_load_self_public(self_public, (*l2).addr, (*context).sql) {
        fingerprint_str = dc_key_get_fingerprint(self_public)
    } else {
        fingerprint_str = dc_strdup(b"<Not yet calculated>\x00" as *const u8 as *const libc::c_char)
    }
    l_readable_str = dc_loginparam_get_readable(l);
    l2_readable_str = dc_loginparam_get_readable(l2);
    inbox_watch = dc_sqlite3_get_config_int(
        (*context).sql,
        b"inbox_watch\x00" as *const u8 as *const libc::c_char,
        1i32,
    );
    sentbox_watch = dc_sqlite3_get_config_int(
        (*context).sql,
        b"sentbox_watch\x00" as *const u8 as *const libc::c_char,
        1i32,
    );
    mvbox_watch = dc_sqlite3_get_config_int(
        (*context).sql,
        b"mvbox_watch\x00" as *const u8 as *const libc::c_char,
        1i32,
    );
    mvbox_move = dc_sqlite3_get_config_int(
        (*context).sql,
        b"mvbox_move\x00" as *const u8 as *const libc::c_char,
        1i32,
    );
    folders_configured = dc_sqlite3_get_config_int(
        (*context).sql,
        b"folders_configured\x00" as *const u8 as *const libc::c_char,
        0i32,
    );
    configured_sentbox_folder = dc_sqlite3_get_config(
        (*context).sql,
        b"configured_sentbox_folder\x00" as *const u8 as *const libc::c_char,
        b"<unset>\x00" as *const u8 as *const libc::c_char,
    );
    configured_mvbox_folder = dc_sqlite3_get_config(
        (*context).sql,
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
        libetpan_get_version_major(),
        libetpan_get_version_minor(),
        // openssl (none used, so setting to 0)
        0 as libc::c_int,
        0 as libc::c_int,
        0 as libc::c_int,
        'a' as libc::c_char as libc::c_int,
        rpgp_enabled,
        // arch
        (::std::mem::size_of::<*mut libc::c_void>() as libc::c_ulong)
            .wrapping_mul(8i32 as libc::c_ulong),
        chats,
        real_msgs,
        deaddrop_msgs,
        contacts,
        if !(*context).dbfile.is_null() {
            (*context).dbfile
        } else {
            unset
        },
        dbversion,
        if !(*context).blobdir.is_null() {
            (*context).blobdir
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
    return ret.buf;
}
pub unsafe fn dc_get_version_str() -> *mut libc::c_char {
    return dc_strdup(VERSION as *const u8 as *const libc::c_char);
}
pub unsafe fn dc_get_fresh_msgs(mut context: *mut dc_context_t) -> *mut dc_array_t {
    let mut show_deaddrop: libc::c_int = 0i32;
    let mut ret: *mut dc_array_t = dc_array_new(context, 128i32 as size_t);
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if !(context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint || ret.is_null()) {
        stmt =
            dc_sqlite3_prepare((*context).sql,
                               b"SELECT m.id FROM msgs m LEFT JOIN contacts ct ON m.from_id=ct.id LEFT JOIN chats c ON m.chat_id=c.id WHERE m.state=?   AND m.hidden=0   AND m.chat_id>?   AND ct.blocked=0   AND (c.blocked=0 OR c.blocked=?) ORDER BY m.timestamp DESC,m.id DESC;\x00"
                                   as *const u8 as *const libc::c_char);
        sqlite3_bind_int(stmt, 1i32, 10i32);
        sqlite3_bind_int(stmt, 2i32, 9i32);
        sqlite3_bind_int(stmt, 3i32, if 0 != show_deaddrop { 2i32 } else { 0i32 });
        while sqlite3_step(stmt) == 100i32 {
            dc_array_add_id(ret, sqlite3_column_int(stmt, 0i32) as uint32_t);
        }
    }
    sqlite3_finalize(stmt);
    return ret;
}
pub unsafe fn dc_search_msgs(
    mut context: *mut dc_context_t,
    mut chat_id: uint32_t,
    mut query: *const libc::c_char,
) -> *mut dc_array_t {
    //clock_t       start = clock();
    let mut success: libc::c_int = 0i32;
    let mut ret: *mut dc_array_t = dc_array_new(context, 100i32 as size_t);
    let mut strLikeInText: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut strLikeBeg: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut real_query: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if !(context.is_null()
        || (*context).magic != 0x11a11807i32 as libc::c_uint
        || ret.is_null()
        || query.is_null())
    {
        real_query = dc_strdup(query);
        dc_trim(real_query);
        if *real_query.offset(0isize) as libc::c_int == 0i32 {
            success = 1i32
        } else {
            strLikeInText = dc_mprintf(
                b"%%%s%%\x00" as *const u8 as *const libc::c_char,
                real_query,
            );
            strLikeBeg = dc_mprintf(b"%s%%\x00" as *const u8 as *const libc::c_char, real_query);
            if 0 != chat_id {
                stmt =
                    dc_sqlite3_prepare((*context).sql,
                                       b"SELECT m.id, m.timestamp FROM msgs m LEFT JOIN contacts ct ON m.from_id=ct.id WHERE m.chat_id=?  AND m.hidden=0  AND ct.blocked=0 AND (txt LIKE ? OR ct.name LIKE ?) ORDER BY m.timestamp,m.id;\x00"
                                           as *const u8 as
                                           *const libc::c_char);
                sqlite3_bind_int(stmt, 1i32, chat_id as libc::c_int);
                sqlite3_bind_text(stmt, 2i32, strLikeInText, -1i32, None);
                sqlite3_bind_text(stmt, 3i32, strLikeBeg, -1i32, None);
            } else {
                let mut show_deaddrop: libc::c_int = 0i32;
                stmt =
                    dc_sqlite3_prepare((*context).sql,
                                       b"SELECT m.id, m.timestamp FROM msgs m LEFT JOIN contacts ct ON m.from_id=ct.id LEFT JOIN chats c ON m.chat_id=c.id WHERE m.chat_id>9 AND m.hidden=0  AND (c.blocked=0 OR c.blocked=?) AND ct.blocked=0 AND (m.txt LIKE ? OR ct.name LIKE ?) ORDER BY m.timestamp DESC,m.id DESC;\x00"
                                           as *const u8 as
                                           *const libc::c_char);
                sqlite3_bind_int(stmt, 1i32, if 0 != show_deaddrop { 2i32 } else { 0i32 });
                sqlite3_bind_text(stmt, 2i32, strLikeInText, -1i32, None);
                sqlite3_bind_text(stmt, 3i32, strLikeBeg, -1i32, None);
            }
            while sqlite3_step(stmt) == 100i32 {
                dc_array_add_id(ret, sqlite3_column_int(stmt, 0i32) as uint32_t);
            }
            success = 1i32
        }
    }
    free(strLikeInText as *mut libc::c_void);
    free(strLikeBeg as *mut libc::c_void);
    free(real_query as *mut libc::c_void);
    sqlite3_finalize(stmt);
    if 0 != success {
        return ret;
    } else {
        if !ret.is_null() {
            dc_array_unref(ret);
        }
        return 0 as *mut dc_array_t;
    };
}
pub unsafe fn dc_is_inbox(
    mut context: *mut dc_context_t,
    mut folder_name: *const libc::c_char,
) -> libc::c_int {
    let mut is_inbox: libc::c_int = 0i32;
    if !folder_name.is_null() {
        is_inbox = if strcasecmp(
            b"INBOX\x00" as *const u8 as *const libc::c_char,
            folder_name,
        ) == 0i32
        {
            1i32
        } else {
            0i32
        }
    }
    return is_inbox;
}
pub unsafe fn dc_is_sentbox(
    mut context: *mut dc_context_t,
    mut folder_name: *const libc::c_char,
) -> libc::c_int {
    let mut sentbox_name: *mut libc::c_char = dc_sqlite3_get_config(
        (*context).sql,
        b"configured_sentbox_folder\x00" as *const u8 as *const libc::c_char,
        0 as *const libc::c_char,
    );
    let mut is_sentbox: libc::c_int = 0i32;
    if !sentbox_name.is_null() && !folder_name.is_null() {
        is_sentbox = if strcasecmp(sentbox_name, folder_name) == 0i32 {
            1i32
        } else {
            0i32
        }
    }
    free(sentbox_name as *mut libc::c_void);
    return is_sentbox;
}
pub unsafe fn dc_is_mvbox(
    mut context: *mut dc_context_t,
    mut folder_name: *const libc::c_char,
) -> libc::c_int {
    let mut mvbox_name: *mut libc::c_char = dc_sqlite3_get_config(
        (*context).sql,
        b"configured_mvbox_folder\x00" as *const u8 as *const libc::c_char,
        0 as *const libc::c_char,
    );
    let mut is_mvbox: libc::c_int = 0i32;
    if !mvbox_name.is_null() && !folder_name.is_null() {
        is_mvbox = if strcasecmp(mvbox_name, folder_name) == 0i32 {
            1i32
        } else {
            0i32
        }
    }
    free(mvbox_name as *mut libc::c_void);
    return is_mvbox;
}
