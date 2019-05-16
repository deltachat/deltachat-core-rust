use mmime::mmapstring::*;

use std::ffi::CStr;
use std::time::{Duration, SystemTime};

use rand::{thread_rng, Rng};

use crate::constants::Event;
use crate::dc_chat::*;
use crate::dc_configure::*;
use crate::dc_context::dc_context_t;
use crate::dc_imap::*;
use crate::dc_imex::*;
use crate::dc_jobthread::*;
use crate::dc_keyhistory::*;
use crate::dc_location::*;
use crate::dc_log::*;
use crate::dc_loginparam::*;
use crate::dc_mimefactory::*;
use crate::dc_msg::*;
use crate::dc_param::*;
use crate::dc_sqlite3::*;
use crate::dc_tools::*;
use crate::types::*;
use crate::x::*;

// thread IDs
// jobs in the INBOX-thread, range from DC_IMAP_THREAD..DC_IMAP_THREAD+999
// low priority ...
// ... high priority
// jobs in the SMTP-thread, range from DC_SMTP_THREAD..DC_SMTP_THREAD+999
// low priority ...
// ... high priority
// timeouts until actions are aborted.
// this may also affects IDLE to return, so a re-connect may take this time.
// mailcore2 uses 30 seconds, k-9 uses 10 seconds
#[derive(Copy, Clone)]
#[repr(C)]
pub struct dc_job_t {
    pub job_id: uint32_t,
    pub action: libc::c_int,
    pub foreign_id: uint32_t,
    pub desired_timestamp: time_t,
    pub added_timestamp: time_t,
    pub tries: libc::c_int,
    pub param: *mut dc_param_t,
    pub try_again: libc::c_int,
    pub pending_error: *mut libc::c_char,
}

pub unsafe fn dc_perform_imap_jobs(context: &dc_context_t) {
    dc_log_info(
        context,
        0i32,
        b"INBOX-jobs started...\x00" as *const u8 as *const libc::c_char,
    );

    let probe_imap_network = *context.probe_imap_network.clone().read().unwrap();
    *context.probe_imap_network.write().unwrap() = 0;
    *context.perform_inbox_jobs_needed.write().unwrap() = 0;

    dc_job_perform(context, 100, probe_imap_network);
    dc_log_info(
        context,
        0i32,
        b"INBOX-jobs ended.\x00" as *const u8 as *const libc::c_char,
    );
}
unsafe fn dc_job_perform(context: &dc_context_t, thread: libc::c_int, probe_network: libc::c_int) {
    let mut select_stmt: *mut sqlite3_stmt;
    let mut job = dc_job_t {
        job_id: 0,
        action: 0,
        foreign_id: 0,
        desired_timestamp: 0,
        added_timestamp: 0,
        tries: 0,
        param: 0 as *mut dc_param_t,
        try_again: 0,
        pending_error: 0 as *mut libc::c_char,
    };
    job.param = dc_param_new();

    if probe_network == 0i32 {
        select_stmt =
            dc_sqlite3_prepare(
                context,
                &context.sql.clone().read().unwrap(),
                b"SELECT id, action, foreign_id, param, added_timestamp, desired_timestamp, tries FROM jobs WHERE thread=? AND desired_timestamp<=? ORDER BY action DESC, added_timestamp;\x00"
                    as *const u8 as *const libc::c_char);
        sqlite3_bind_int64(select_stmt, 1i32, thread as sqlite3_int64);
        sqlite3_bind_int64(select_stmt, 2i32, time(0 as *mut time_t) as sqlite3_int64);
    } else {
        select_stmt =
            dc_sqlite3_prepare(
                context,
                &context.sql.clone().read().unwrap(),
                b"SELECT id, action, foreign_id, param, added_timestamp, desired_timestamp, tries FROM jobs WHERE thread=? AND tries>0 ORDER BY desired_timestamp, action DESC;\x00"
                                       as *const u8 as *const libc::c_char);
        sqlite3_bind_int64(select_stmt, 1i32, thread as sqlite3_int64);
    }
    while sqlite3_step(select_stmt) == 100i32 {
        job.job_id = sqlite3_column_int(select_stmt, 0i32) as uint32_t;
        job.action = sqlite3_column_int(select_stmt, 1i32);
        job.foreign_id = sqlite3_column_int(select_stmt, 2i32) as uint32_t;
        dc_param_set_packed(
            job.param,
            sqlite3_column_text(select_stmt, 3i32) as *mut libc::c_char,
        );
        job.added_timestamp = sqlite3_column_int64(select_stmt, 4i32) as time_t;
        job.desired_timestamp = sqlite3_column_int64(select_stmt, 5i32) as time_t;
        job.tries = sqlite3_column_int(select_stmt, 6i32);
        dc_log_info(
            context,
            0i32,
            b"%s-job #%i, action %i started...\x00" as *const u8 as *const libc::c_char,
            if thread == 100i32 {
                b"INBOX\x00" as *const u8 as *const libc::c_char
            } else {
                b"SMTP\x00" as *const u8 as *const libc::c_char
            },
            job.job_id as libc::c_int,
            job.action as libc::c_int,
        );
        if 900i32 == job.action || 910i32 == job.action {
            dc_job_kill_action(context, job.action);
            sqlite3_finalize(select_stmt);
            select_stmt = 0 as *mut sqlite3_stmt;
            dc_jobthread_suspend(context, &context.sentbox_thread.clone().read().unwrap(), 1);
            dc_jobthread_suspend(context, &context.mvbox_thread.clone().read().unwrap(), 1);
            dc_suspend_smtp_thread(context, 1i32);
        }
        let mut tries: libc::c_int = 0i32;
        while tries <= 1i32 {
            job.try_again = 0i32;
            match job.action {
                5901 => {
                    dc_job_do_DC_JOB_SEND(context, &mut job);
                }
                110 => {
                    dc_job_do_DC_JOB_DELETE_MSG_ON_IMAP(context, &mut job);
                }
                130 => {
                    dc_job_do_DC_JOB_MARKSEEN_MSG_ON_IMAP(context, &mut job);
                }
                120 => {
                    dc_job_do_DC_JOB_MARKSEEN_MDN_ON_IMAP(context, &mut job);
                }
                200 => {
                    dc_job_do_DC_JOB_MOVE_MSG(context, &mut job);
                }
                5011 => {
                    dc_job_do_DC_JOB_SEND(context, &mut job);
                }
                900 => {
                    dc_job_do_DC_JOB_CONFIGURE_IMAP(context, &mut job);
                }
                910 => {
                    dc_job_do_DC_JOB_IMEX_IMAP(context, &mut job);
                }
                5005 => {
                    dc_job_do_DC_JOB_MAYBE_SEND_LOCATIONS(context, &mut job);
                }
                5007 => {
                    dc_job_do_DC_JOB_MAYBE_SEND_LOC_ENDED(context, &mut job);
                }
                105 => {
                    dc_housekeeping(context);
                }
                _ => {}
            }
            if job.try_again != -1i32 {
                break;
            }
            tries += 1
        }
        if 900i32 == job.action || 910i32 == job.action {
            dc_jobthread_suspend(
                context,
                &mut context.sentbox_thread.clone().read().unwrap(),
                0,
            );
            dc_jobthread_suspend(
                context,
                &mut context.mvbox_thread.clone().read().unwrap(),
                0,
            );
            dc_suspend_smtp_thread(context, 0i32);
            break;
        } else if job.try_again == 2i32 {
            dc_log_info(
                context,
                0i32,
                b"%s-job #%i not yet ready and will be delayed.\x00" as *const u8
                    as *const libc::c_char,
                if thread == 100i32 {
                    b"INBOX\x00" as *const u8 as *const libc::c_char
                } else {
                    b"SMTP\x00" as *const u8 as *const libc::c_char
                },
                job.job_id as libc::c_int,
            );
        } else if job.try_again == -1i32 || job.try_again == 3i32 {
            let tries_0: libc::c_int = job.tries + 1i32;
            if tries_0 < 17i32 {
                job.tries = tries_0;
                let time_offset: time_t = get_backoff_time_offset(tries_0);
                job.desired_timestamp = job.added_timestamp + time_offset;
                dc_job_update(context, &mut job);
                dc_log_info(context, 0i32,
                                b"%s-job #%i not succeeded on try #%i, retry in ADD_TIME+%i (in %i seconds).\x00"
                                    as *const u8 as *const libc::c_char,
                                if thread == 100i32 {
                                    b"INBOX\x00" as *const u8 as
                                        *const libc::c_char
                                } else {
                                    b"SMTP\x00" as *const u8 as
                                        *const libc::c_char
                                }, job.job_id as libc::c_int, tries_0,
                                time_offset,
                                job.added_timestamp + time_offset -
                                    time(0 as *mut time_t));
                if thread == 5000i32 && tries_0 < 17i32 - 1i32 {
                    context
                        .smtp_state
                        .clone()
                        .0
                        .lock()
                        .unwrap()
                        .perform_jobs_needed = 2i32;
                }
            } else {
                if job.action == 5901i32 {
                    dc_set_msg_failed(context, job.foreign_id, job.pending_error);
                }
                dc_job_delete(context, &mut job);
            }
            if !(0 != probe_network) {
                continue;
            }
            // on dc_maybe_network() we stop trying here;
            // these jobs are already tried once.
            // otherwise, we just continue with the next job
            // to give other jobs a chance being tried at least once.
            break;
        } else {
            dc_job_delete(context, &mut job);
        }
    }
    dc_param_unref(job.param);
    free(job.pending_error as *mut libc::c_void);
    sqlite3_finalize(select_stmt);
}

unsafe fn dc_job_delete(context: &dc_context_t, job: &dc_job_t) {
    let delete_stmt: *mut sqlite3_stmt = dc_sqlite3_prepare(
        context,
        &context.sql.clone().read().unwrap(),
        b"DELETE FROM jobs WHERE id=?;\x00" as *const u8 as *const libc::c_char,
    );
    sqlite3_bind_int(delete_stmt, 1i32, job.job_id as libc::c_int);
    sqlite3_step(delete_stmt);
    sqlite3_finalize(delete_stmt);
}

/* ******************************************************************************
 * Tools
 ******************************************************************************/
unsafe fn get_backoff_time_offset(c_tries: libc::c_int) -> time_t {
    // results in ~3 weeks for the last backoff timespan
    let mut N = 2_i32.pow((c_tries - 1) as u32);
    N = N * 60;
    let mut rng = thread_rng();
    let n: i32 = rng.gen();
    let mut seconds = n % (N + 1);
    if seconds < 1 {
        seconds = 1;
    }
    seconds as time_t
}
unsafe fn dc_job_update(context: &dc_context_t, job: &dc_job_t) {
    let stmt: *mut sqlite3_stmt = dc_sqlite3_prepare(
        context,
        &context.sql.clone().read().unwrap(),
        b"UPDATE jobs SET desired_timestamp=?, tries=?, param=? WHERE id=?;\x00" as *const u8
            as *const libc::c_char,
    );
    sqlite3_bind_int64(stmt, 1i32, job.desired_timestamp as sqlite3_int64);
    sqlite3_bind_int64(stmt, 2i32, job.tries as sqlite3_int64);
    sqlite3_bind_text(stmt, 3i32, (*job.param).packed, -1i32, None);
    sqlite3_bind_int(stmt, 4i32, job.job_id as libc::c_int);
    sqlite3_step(stmt);
    sqlite3_finalize(stmt);
}
unsafe fn dc_suspend_smtp_thread(context: &dc_context_t, suspend: libc::c_int) {
    context.smtp_state.0.lock().unwrap().suspended = suspend;
    if 0 != suspend {
        loop {
            if context.smtp_state.0.lock().unwrap().doing_jobs == 0 {
                return;
            }
            std::thread::sleep(std::time::Duration::from_micros(300 * 1000));
        }
    }
}
unsafe fn dc_job_do_DC_JOB_SEND(context: &dc_context_t, job: &mut dc_job_t) {
    let mut current_block: u64;
    let mut filename: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut buf: *mut libc::c_void = 0 as *mut libc::c_void;
    let mut buf_bytes: size_t = 0i32 as size_t;
    let mut recipients: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    /* connect to SMTP server, if not yet done */
    if !context.smtp.lock().unwrap().is_connected() {
        let loginparam: *mut dc_loginparam_t = dc_loginparam_new();
        dc_loginparam_read(
            context,
            loginparam,
            &context.sql.clone().read().unwrap(),
            b"configured_\x00" as *const u8 as *const libc::c_char,
        );
        let connected = context.smtp.lock().unwrap().connect(context, loginparam);
        dc_loginparam_unref(loginparam);
        if 0 == connected {
            dc_job_try_again_later(job, 3i32, 0 as *const libc::c_char);
            current_block = 14216916617354591294;
        } else {
            current_block = 13109137661213826276;
        }
    } else {
        current_block = 13109137661213826276;
    }
    match current_block {
        13109137661213826276 => {
            filename = dc_param_get(job.param, 'f' as i32, 0 as *const libc::c_char);
            if filename.is_null() {
                dc_log_warning(
                    context,
                    0i32,
                    b"Missing file name for job %d\x00" as *const u8 as *const libc::c_char,
                    job.job_id,
                );
            } else if !(0 == dc_read_file(context, filename, &mut buf, &mut buf_bytes)) {
                recipients = dc_param_get(job.param, 'R' as i32, 0 as *const libc::c_char);
                if recipients.is_null() {
                    dc_log_warning(
                        context,
                        0i32,
                        b"Missing recipients for job %d\x00" as *const u8 as *const libc::c_char,
                        job.job_id,
                    );
                } else {
                    let recipients_list = std::ffi::CStr::from_ptr(recipients)
                        .to_str()
                        .unwrap()
                        .split("\x1e")
                        .filter_map(|addr| match lettre::EmailAddress::new(addr.to_string()) {
                            Ok(addr) => Some(addr),
                            Err(err) => {
                                eprintln!("WARNING: invalid recipient: {} {:?}", addr, err);
                                None
                            }
                        })
                        .collect::<Vec<_>>();
                    /* if there is a msg-id and it does not exist in the db, cancel sending.
                    this happends if dc_delete_msgs() was called
                    before the generated mime was sent out */
                    if 0 != job.foreign_id {
                        if 0 == dc_msg_exists(context, job.foreign_id) {
                            dc_log_warning(
                                context,
                                0i32,
                                b"Message %i for job %i does not exist\x00" as *const u8
                                    as *const libc::c_char,
                                job.foreign_id,
                                job.job_id,
                            );
                            current_block = 14216916617354591294;
                        } else {
                            current_block = 11194104282611034094;
                        }
                    } else {
                        current_block = 11194104282611034094;
                    }
                    match current_block {
                        14216916617354591294 => {}
                        _ => {
                            /* send message */
                            let body =
                                std::slice::from_raw_parts(buf as *const u8, buf_bytes).to_vec();
                            if 0 == context.smtp.lock().unwrap().send(
                                context,
                                recipients_list,
                                body,
                            ) {
                                context.smtp.lock().unwrap().disconnect();
                                dc_job_try_again_later(
                                    job,
                                    -1i32,
                                    (*&mut context.smtp.clone().lock().unwrap()).error,
                                );
                            } else {
                                dc_delete_file(context, filename);
                                if 0 != job.foreign_id {
                                    dc_update_msg_state(context, job.foreign_id, 26i32);
                                    stmt = dc_sqlite3_prepare(
                                        context,
                                        &context.sql.clone().read().unwrap(),
                                        b"SELECT chat_id FROM msgs WHERE id=?\x00" as *const u8
                                            as *const libc::c_char,
                                    );
                                    sqlite3_bind_int(stmt, 1i32, job.foreign_id as libc::c_int);
                                    let chat_id: libc::c_int = if sqlite3_step(stmt) == 100i32 {
                                        sqlite3_column_int(stmt, 0i32)
                                    } else {
                                        0i32
                                    };
                                    (context.cb)(
                                        context,
                                        Event::MSG_DELIVERED,
                                        chat_id as uintptr_t,
                                        job.foreign_id as uintptr_t,
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
        _ => {}
    }
    sqlite3_finalize(stmt);
    free(recipients as *mut libc::c_void);
    free(buf);
    free(filename as *mut libc::c_void);
}
// this value does not increase the number of tries
pub unsafe fn dc_job_try_again_later(
    job: &mut dc_job_t,
    try_again: libc::c_int,
    pending_error: *const libc::c_char,
) {
    job.try_again = try_again;
    free(job.pending_error as *mut libc::c_void);
    job.pending_error = dc_strdup_keep_null(pending_error);
}

unsafe fn dc_job_do_DC_JOB_MOVE_MSG(context: &dc_context_t, job: &mut dc_job_t) {
    let mut current_block: u64;
    let msg: *mut dc_msg_t = dc_msg_new_untyped(context);
    let mut dest_folder: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut dest_uid: uint32_t = 0i32 as uint32_t;

    let inbox = context.inbox.read().unwrap();

    if !inbox.is_connected() {
        connect_to_inbox(context, &inbox);
        if !inbox.is_connected() {
            dc_job_try_again_later(job, 3i32, 0 as *const libc::c_char);
            current_block = 2238328302157162973;
        } else {
            current_block = 2473556513754201174;
        }
    } else {
        current_block = 2473556513754201174;
    }
    match current_block {
        2473556513754201174 => {
            if dc_msg_load_from_db(msg, context, job.foreign_id) {
                if dc_sqlite3_get_config_int(
                    context,
                    &context.sql.clone().read().unwrap(),
                    b"folders_configured\x00" as *const u8 as *const libc::c_char,
                    0i32,
                ) < 3i32
                {
                    inbox.configure_folders(context, 0x1i32);
                }
                dest_folder = dc_sqlite3_get_config(
                    context,
                    &context.sql.clone().read().unwrap(),
                    b"configured_mvbox_folder\x00" as *const u8 as *const libc::c_char,
                    0 as *const libc::c_char,
                );

                let server_folder = CStr::from_ptr((*msg).server_folder).to_str().unwrap();
                match inbox.mv(
                    context,
                    server_folder,
                    (*msg).server_uid,
                    CStr::from_ptr(dest_folder).to_str().unwrap(),
                    &mut dest_uid,
                ) as libc::c_uint
                {
                    1 => {
                        current_block = 6379107252614456477;
                        match current_block {
                            12072121998757195963 => {
                                dc_update_server_uid(
                                    context,
                                    (*msg).rfc724_mid,
                                    dest_folder,
                                    dest_uid,
                                );
                            }
                            _ => {
                                dc_job_try_again_later(job, 3i32, 0 as *const libc::c_char);
                            }
                        }
                    }
                    3 => {
                        current_block = 12072121998757195963;
                        match current_block {
                            12072121998757195963 => {
                                dc_update_server_uid(
                                    context,
                                    (*msg).rfc724_mid,
                                    dest_folder,
                                    dest_uid,
                                );
                            }
                            _ => {
                                dc_job_try_again_later(job, 3i32, 0 as *const libc::c_char);
                            }
                        }
                    }
                    0 | 2 | _ => {}
                }
            }
        }
        _ => {}
    }
    free(dest_folder as *mut libc::c_void);
    dc_msg_unref(msg);
}
/* ******************************************************************************
 * IMAP-jobs
 ******************************************************************************/
unsafe fn connect_to_inbox(context: &dc_context_t, inbox: &Imap) -> libc::c_int {
    let ret_connected: libc::c_int;

    ret_connected = dc_connect_to_configured_imap(context, inbox);
    if !(0 == ret_connected) {
        inbox.set_watch_folder(b"INBOX\x00" as *const u8 as *const libc::c_char);
    }
    ret_connected
}

unsafe fn dc_job_do_DC_JOB_MARKSEEN_MDN_ON_IMAP(context: &dc_context_t, job: &mut dc_job_t) {
    let current_block: u64;
    let folder: *mut libc::c_char = dc_param_get(job.param, 'Z' as i32, 0 as *const libc::c_char);
    let uid: uint32_t = dc_param_get_int(job.param, 'z' as i32, 0i32) as uint32_t;
    let mut dest_folder: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut dest_uid: uint32_t = 0i32 as uint32_t;
    let inbox = context.inbox.read().unwrap();

    if !inbox.is_connected() {
        connect_to_inbox(context, &inbox);
        if !inbox.is_connected() {
            dc_job_try_again_later(job, 3i32, 0 as *const libc::c_char);
            current_block = 2670689566614003383;
        } else {
            current_block = 11006700562992250127;
        }
    } else {
        current_block = 11006700562992250127;
    }
    match current_block {
        11006700562992250127 => {
            let folder = CStr::from_ptr(folder).to_str().unwrap();
            if inbox.set_seen(context, folder, uid) as libc::c_uint == 0i32 as libc::c_uint {
                dc_job_try_again_later(job, 3i32, 0 as *const libc::c_char);
            }
            if 0 != dc_param_get_int(job.param, 'M' as i32, 0i32) {
                if dc_sqlite3_get_config_int(
                    context,
                    &context.sql.clone().read().unwrap(),
                    b"folders_configured\x00" as *const u8 as *const libc::c_char,
                    0i32,
                ) < 3i32
                {
                    inbox.configure_folders(context, 0x1i32);
                }
                dest_folder = dc_sqlite3_get_config(
                    context,
                    &context.sql.clone().read().unwrap(),
                    b"configured_mvbox_folder\x00" as *const u8 as *const libc::c_char,
                    0 as *const libc::c_char,
                );
                let dest_folder = CStr::from_ptr(dest_folder).to_str().unwrap();
                match inbox.mv(context, folder, uid, dest_folder, &mut dest_uid) as libc::c_uint {
                    1 => {
                        dc_job_try_again_later(job, 3i32, 0 as *const libc::c_char);
                    }
                    0 | _ => {}
                }
            }
        }
        _ => {}
    }
    free(folder as *mut libc::c_void);
    free(dest_folder as *mut libc::c_void);
}
unsafe fn dc_job_do_DC_JOB_MARKSEEN_MSG_ON_IMAP(context: &dc_context_t, job: &mut dc_job_t) {
    let mut current_block: u64;
    let msg: *mut dc_msg_t = dc_msg_new_untyped(context);
    let inbox = context.inbox.read().unwrap();

    if !inbox.is_connected() {
        connect_to_inbox(context, &inbox);
        if !inbox.is_connected() {
            dc_job_try_again_later(job, 3i32, 0 as *const libc::c_char);
            current_block = 17792648348530113339;
        } else {
            current_block = 15240798224410183470;
        }
    } else {
        current_block = 15240798224410183470;
    }
    match current_block {
        15240798224410183470 => {
            if dc_msg_load_from_db(msg, context, job.foreign_id) {
                let server_folder = CStr::from_ptr((*msg).server_folder).to_str().unwrap();
                match inbox.set_seen(context, server_folder, (*msg).server_uid) as libc::c_uint {
                    0 => {}
                    1 => {
                        current_block = 12392248546350854223;
                        match current_block {
                            12392248546350854223 => {
                                dc_job_try_again_later(job, 3i32, 0 as *const libc::c_char);
                            }
                            _ => {
                                if 0 != dc_param_get_int((*msg).param, 'r' as i32, 0i32)
                                    && 0 != dc_sqlite3_get_config_int(
                                        context,
                                        &context.sql.clone().read().unwrap(),
                                        b"mdns_enabled\x00" as *const u8 as *const libc::c_char,
                                        1i32,
                                    )
                                {
                                    let folder =
                                        CStr::from_ptr((*msg).server_folder).to_str().unwrap();
                                    match inbox.set_mdnsent(context, folder, (*msg).server_uid)
                                        as libc::c_uint
                                    {
                                        1 => {
                                            current_block = 4016212065805849280;
                                            match current_block {
                                                6186957421461061791 => {
                                                    dc_send_mdn(context, (*msg).id);
                                                }
                                                _ => {
                                                    dc_job_try_again_later(
                                                        job,
                                                        3i32,
                                                        0 as *const libc::c_char,
                                                    );
                                                }
                                            }
                                        }
                                        3 => {
                                            current_block = 6186957421461061791;
                                            match current_block {
                                                6186957421461061791 => {
                                                    dc_send_mdn(context, (*msg).id);
                                                }
                                                _ => {
                                                    dc_job_try_again_later(
                                                        job,
                                                        3i32,
                                                        0 as *const libc::c_char,
                                                    );
                                                }
                                            }
                                        }
                                        0 | 2 | _ => {}
                                    }
                                }
                            }
                        }
                    }
                    _ => {
                        current_block = 7746791466490516765;
                        match current_block {
                            12392248546350854223 => {
                                dc_job_try_again_later(job, 3i32, 0 as *const libc::c_char);
                            }
                            _ => {
                                if 0 != dc_param_get_int((*msg).param, 'r' as i32, 0i32)
                                    && 0 != dc_sqlite3_get_config_int(
                                        context,
                                        &context.sql.clone().read().unwrap(),
                                        b"mdns_enabled\x00" as *const u8 as *const libc::c_char,
                                        1i32,
                                    )
                                {
                                    let folder =
                                        CStr::from_ptr((*msg).server_folder).to_str().unwrap();

                                    match inbox.set_mdnsent(context, folder, (*msg).server_uid)
                                        as libc::c_uint
                                    {
                                        1 => {
                                            current_block = 4016212065805849280;
                                            match current_block {
                                                6186957421461061791 => {
                                                    dc_send_mdn(context, (*msg).id);
                                                }
                                                _ => {
                                                    dc_job_try_again_later(
                                                        job,
                                                        3i32,
                                                        0 as *const libc::c_char,
                                                    );
                                                }
                                            }
                                        }
                                        3 => {
                                            current_block = 6186957421461061791;
                                            match current_block {
                                                6186957421461061791 => {
                                                    dc_send_mdn(context, (*msg).id);
                                                }
                                                _ => {
                                                    dc_job_try_again_later(
                                                        job,
                                                        3i32,
                                                        0 as *const libc::c_char,
                                                    );
                                                }
                                            }
                                        }
                                        0 | 2 | _ => {}
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        _ => {}
    }
    dc_msg_unref(msg);
}
unsafe fn dc_send_mdn(context: &dc_context_t, msg_id: uint32_t) {
    let mut mimefactory: dc_mimefactory_t = dc_mimefactory_t {
        from_addr: 0 as *mut libc::c_char,
        from_displayname: 0 as *mut libc::c_char,
        selfstatus: 0 as *mut libc::c_char,
        recipients_names: 0 as *mut clist,
        recipients_addr: 0 as *mut clist,
        timestamp: 0,
        rfc724_mid: 0 as *mut libc::c_char,
        loaded: DC_MF_NOTHING_LOADED,
        msg: 0 as *mut dc_msg_t,
        chat: 0 as *mut dc_chat_t,
        increation: 0,
        in_reply_to: 0 as *mut libc::c_char,
        references: 0 as *mut libc::c_char,
        req_mdn: 0,
        out: 0 as *mut MMAPString,
        out_encrypted: 0,
        out_gossiped: 0,
        out_last_added_location_id: 0,
        error: 0 as *mut libc::c_char,
        context,
    };
    dc_mimefactory_init(&mut mimefactory, context);
    if !(0 == dc_mimefactory_load_mdn(&mut mimefactory, msg_id)
        || 0 == dc_mimefactory_render(&mut mimefactory))
    {
        dc_add_smtp_job(context, 5011i32, &mut mimefactory);
    }
    dc_mimefactory_empty(&mut mimefactory);
}
/* ******************************************************************************
 * SMTP-jobs
 ******************************************************************************/
/* *
 * Store the MIME message in a file and send it later with a new SMTP job.
 *
 * @param context The context object as created by dc_context_new()
 * @param action One of the DC_JOB_SEND_ constants
 * @param mimefactory An instance of dc_mimefactory_t with a loaded and rendered message or MDN
 * @return 1=success, 0=error
 */
unsafe fn dc_add_smtp_job(
    context: &dc_context_t,
    action: libc::c_int,
    mimefactory: *mut dc_mimefactory_t,
) -> libc::c_int {
    let pathNfilename: *mut libc::c_char;
    let mut success: libc::c_int = 0i32;
    let mut recipients: *mut libc::c_char = 0 as *mut libc::c_char;
    let param: *mut dc_param_t = dc_param_new();
    pathNfilename = dc_get_fine_pathNfilename(
        context,
        b"$BLOBDIR\x00" as *const u8 as *const libc::c_char,
        (*mimefactory).rfc724_mid,
    );
    if pathNfilename.is_null() {
        dc_log_error(
            context,
            0i32,
            b"Could not find free file name for message with ID <%s>.\x00" as *const u8
                as *const libc::c_char,
            (*mimefactory).rfc724_mid,
        );
    } else if 0
        == dc_write_file(
            context,
            pathNfilename,
            (*(*mimefactory).out).str_0 as *const libc::c_void,
            (*(*mimefactory).out).len,
        )
    {
        dc_log_error(
            context,
            0i32,
            b"Could not write message <%s> to \"%s\".\x00" as *const u8 as *const libc::c_char,
            (*mimefactory).rfc724_mid,
            pathNfilename,
        );
    } else {
        recipients = dc_str_from_clist(
            (*mimefactory).recipients_addr,
            b"\x1e\x00" as *const u8 as *const libc::c_char,
        );
        dc_param_set(param, 'f' as i32, pathNfilename);
        dc_param_set(param, 'R' as i32, recipients);
        dc_job_add(
            context,
            action,
            (if (*mimefactory).loaded as libc::c_uint
                == DC_MF_MSG_LOADED as libc::c_int as libc::c_uint
            {
                (*(*mimefactory).msg).id
            } else {
                0i32 as libc::c_uint
            }) as libc::c_int,
            (*param).packed,
            0i32,
        );
        success = 1i32
    }
    dc_param_unref(param);
    free(recipients as *mut libc::c_void);
    free(pathNfilename as *mut libc::c_void);
    return success;
}
pub unsafe fn dc_job_add(
    context: &dc_context_t,
    action: libc::c_int,
    foreign_id: libc::c_int,
    param: *const libc::c_char,
    delay_seconds: libc::c_int,
) {
    let timestamp: time_t = time(0 as *mut time_t);
    let stmt: *mut sqlite3_stmt;
    let thread: libc::c_int;
    if action >= 100i32 && action < 100i32 + 1000i32 {
        thread = 100i32
    } else if action >= 5000i32 && action < 5000i32 + 1000i32 {
        thread = 5000i32
    } else {
        return;
    }
    stmt =
        dc_sqlite3_prepare(
            context,
            &context.sql.clone().read().unwrap(),
            b"INSERT INTO jobs (added_timestamp, thread, action, foreign_id, param, desired_timestamp) VALUES (?,?,?,?,?,?);\x00"
                               as *const u8 as *const libc::c_char);
    sqlite3_bind_int64(stmt, 1i32, timestamp as sqlite3_int64);
    sqlite3_bind_int(stmt, 2i32, thread);
    sqlite3_bind_int(stmt, 3i32, action);
    sqlite3_bind_int(stmt, 4i32, foreign_id);
    sqlite3_bind_text(
        stmt,
        5i32,
        if !param.is_null() {
            param
        } else {
            b"\x00" as *const u8 as *const libc::c_char
        },
        -1i32,
        None,
    );
    sqlite3_bind_int64(
        stmt,
        6i32,
        (timestamp + delay_seconds as time_t) as sqlite3_int64,
    );
    sqlite3_step(stmt);
    sqlite3_finalize(stmt);
    if thread == 100i32 {
        dc_interrupt_imap_idle(context);
    } else {
        dc_interrupt_smtp_idle(context);
    };
}
pub unsafe fn dc_interrupt_smtp_idle(context: &dc_context_t) {
    dc_log_info(
        context,
        0i32,
        b"Interrupting SMTP-idle...\x00" as *const u8 as *const libc::c_char,
    );

    let &(ref lock, ref cvar) = &*context.smtp_state.clone();
    let mut state = lock.lock().unwrap();

    state.perform_jobs_needed = 1;
    state.idle = true;
    cvar.notify_one();
}

pub unsafe fn dc_interrupt_imap_idle(context: &dc_context_t) {
    dc_log_info(
        context,
        0i32,
        b"Interrupting IMAP-IDLE...\x00" as *const u8 as *const libc::c_char,
    );

    *context.perform_inbox_jobs_needed.write().unwrap() = 1;
    context.inbox.read().unwrap().interrupt_idle();
}

unsafe fn dc_job_do_DC_JOB_DELETE_MSG_ON_IMAP(context: &dc_context_t, job: &mut dc_job_t) {
    let mut current_block: u64;
    let mut delete_from_server: libc::c_int = 1i32;
    let msg: *mut dc_msg_t = dc_msg_new_untyped(context);
    let inbox = context.inbox.read().unwrap();

    if !(!dc_msg_load_from_db(msg, context, job.foreign_id)
        || (*msg).rfc724_mid.is_null()
        || *(*msg).rfc724_mid.offset(0isize) as libc::c_int == 0i32)
    {
        /* eg. device messages have no Message-ID */
        if dc_rfc724_mid_cnt(context, (*msg).rfc724_mid) != 1i32 {
            dc_log_info(
                context,
                0i32,
                b"The message is deleted from the server when all parts are deleted.\x00"
                    as *const u8 as *const libc::c_char,
            );
            delete_from_server = 0i32
        }
        /* if this is the last existing part of the message, we delete the message from the server */
        if 0 != delete_from_server {
            if !inbox.is_connected() {
                connect_to_inbox(context, &inbox);
                if !inbox.is_connected() {
                    dc_job_try_again_later(job, 3i32, 0 as *const libc::c_char);
                    current_block = 8913536887710889399;
                } else {
                    current_block = 5399440093318478209;
                }
            } else {
                current_block = 5399440093318478209;
            }
            match current_block {
                8913536887710889399 => {}
                _ => {
                    let mid = CStr::from_ptr((*msg).rfc724_mid).to_str().unwrap();
                    let server_folder = CStr::from_ptr((*msg).server_folder).to_str().unwrap();
                    if 0 == inbox.delete_msg(context, mid, server_folder, &mut (*msg).server_uid) {
                        dc_job_try_again_later(job, -1i32, 0 as *const libc::c_char);
                        current_block = 8913536887710889399;
                    } else {
                        current_block = 17407779659766490442;
                    }
                }
            }
        } else {
            current_block = 17407779659766490442;
        }
        match current_block {
            8913536887710889399 => {}
            _ => {
                dc_delete_msg_from_db(context, (*msg).id);
            }
        }
    }
    dc_msg_unref(msg);
}

/* delete all pending jobs with the given action */
pub unsafe fn dc_job_kill_action(context: &dc_context_t, action: libc::c_int) {
    let stmt = dc_sqlite3_prepare(
        context,
        &context.sql.clone().read().unwrap(),
        b"DELETE FROM jobs WHERE action=?;\x00" as *const u8 as *const libc::c_char,
    );
    sqlite3_bind_int(stmt, 1i32, action);
    sqlite3_step(stmt);
    sqlite3_finalize(stmt);
}

pub unsafe fn dc_perform_imap_fetch(context: &dc_context_t) {
    let inbox = context.inbox.read().unwrap();

    let start: libc::clock_t = clock();
    if 0 == connect_to_inbox(context, &inbox) {
        return;
    }
    if dc_sqlite3_get_config_int(
        context,
        &context.sql.clone().read().unwrap(),
        b"inbox_watch\x00" as *const u8 as *const libc::c_char,
        1i32,
    ) == 0i32
    {
        dc_log_info(
            context,
            0i32,
            b"INBOX-watch disabled.\x00" as *const u8 as *const libc::c_char,
        );
        return;
    }
    dc_log_info(
        context,
        0i32,
        b"INBOX-fetch started...\x00" as *const u8 as *const libc::c_char,
    );
    inbox.fetch(context);
    if inbox.should_reconnect() {
        dc_log_info(
            context,
            0i32,
            b"INBOX-fetch aborted, starting over...\x00" as *const u8 as *const libc::c_char,
        );
        inbox.fetch(context);
    }
    dc_log_info(
        context,
        0i32,
        b"INBOX-fetch done in %.0f ms.\x00" as *const u8 as *const libc::c_char,
        clock().wrapping_sub(start) as libc::c_double * 1000.0f64 / 1000000i32 as libc::c_double,
    );
}

pub unsafe fn dc_perform_imap_idle(context: &dc_context_t) {
    let inbox = context.inbox.read().unwrap();

    connect_to_inbox(context, &inbox);

    if 0 != *context.perform_inbox_jobs_needed.clone().read().unwrap() {
        dc_log_info(
            context,
            0i32,
            b"INBOX-IDLE will not be started because of waiting jobs.\x00" as *const u8
                as *const libc::c_char,
        );
        return;
    }
    dc_log_info(
        context,
        0i32,
        b"INBOX-IDLE started...\x00" as *const u8 as *const libc::c_char,
    );
    inbox.idle(context);
    dc_log_info(
        context,
        0i32,
        b"INBOX-IDLE ended.\x00" as *const u8 as *const libc::c_char,
    );
}

pub unsafe fn dc_perform_mvbox_fetch(context: &dc_context_t) {
    let use_network: libc::c_int = dc_sqlite3_get_config_int(
        context,
        &context.sql.clone().read().unwrap(),
        b"mvbox_watch\x00" as *const u8 as *const libc::c_char,
        1i32,
    );
    dc_jobthread_fetch(
        context,
        &mut context.mvbox_thread.clone().write().unwrap(),
        use_network,
    );
}

pub unsafe fn dc_perform_mvbox_idle(context: &dc_context_t) {
    let use_network: libc::c_int = dc_sqlite3_get_config_int(
        context,
        &context.sql.clone().read().unwrap(),
        b"mvbox_watch\x00" as *const u8 as *const libc::c_char,
        1i32,
    );
    dc_jobthread_idle(
        context,
        &context.mvbox_thread.clone().read().unwrap(),
        use_network,
    );
}

pub unsafe fn dc_interrupt_mvbox_idle(context: &dc_context_t) {
    dc_jobthread_interrupt_idle(context, &context.mvbox_thread.clone().read().unwrap());
}

pub unsafe fn dc_perform_sentbox_fetch(context: &dc_context_t) {
    let use_network: libc::c_int = dc_sqlite3_get_config_int(
        context,
        &context.sql.clone().read().unwrap(),
        b"sentbox_watch\x00" as *const u8 as *const libc::c_char,
        1i32,
    );
    dc_jobthread_fetch(
        context,
        &mut context.sentbox_thread.clone().write().unwrap(),
        use_network,
    );
}

pub unsafe fn dc_perform_sentbox_idle(context: &dc_context_t) {
    let use_network: libc::c_int = dc_sqlite3_get_config_int(
        context,
        &context.sql.clone().read().unwrap(),
        b"sentbox_watch\x00" as *const u8 as *const libc::c_char,
        1i32,
    );
    dc_jobthread_idle(
        context,
        &context.sentbox_thread.clone().read().unwrap(),
        use_network,
    );
}

pub unsafe fn dc_interrupt_sentbox_idle(context: &dc_context_t) {
    dc_jobthread_interrupt_idle(context, &context.sentbox_thread.clone().read().unwrap());
}

pub unsafe fn dc_perform_smtp_jobs(context: &dc_context_t) {
    let probe_smtp_network = {
        let &(ref lock, _) = &*context.smtp_state.clone();
        let mut state = lock.lock().unwrap();

        let probe_smtp_network = state.probe_network;
        state.probe_network = 0;
        state.perform_jobs_needed = 0;

        if 0 != state.suspended {
            dc_log_info(
                context,
                0i32,
                b"SMTP-jobs suspended.\x00" as *const u8 as *const libc::c_char,
            );
            return;
        }
        state.doing_jobs = 1;
        probe_smtp_network
    };

    dc_log_info(
        context,
        0,
        b"SMTP-jobs started...\x00" as *const u8 as *const libc::c_char,
    );
    dc_job_perform(context, 5000, probe_smtp_network);
    dc_log_info(
        context,
        0i32,
        b"SMTP-jobs ended.\x00" as *const u8 as *const libc::c_char,
    );

    {
        let &(ref lock, _) = &*context.smtp_state.clone();
        let mut state = lock.lock().unwrap();

        state.doing_jobs = 0;
    }
}

pub unsafe fn dc_perform_smtp_idle(context: &dc_context_t) {
    dc_log_info(
        context,
        0i32,
        b"SMTP-idle started...\x00" as *const u8 as *const libc::c_char,
    );
    {
        let &(ref lock, ref cvar) = &*context.smtp_state.clone();
        let mut state = lock.lock().unwrap();

        if state.perform_jobs_needed == 1 {
            dc_log_info(
                context,
                0,
                b"SMTP-idle will not be started because of waiting jobs.\x00" as *const u8
                    as *const libc::c_char,
            );
        } else {
            let dur = get_next_wakeup_time(context, 5000);

            loop {
                let res = cvar.wait_timeout(state, dur).unwrap();
                state = res.0;

                if state.idle == true || res.1.timed_out() {
                    // We received the notification and the value has been updated, we can leave.
                    break;
                }
            }
            state.idle = false;
        }
    }

    dc_log_info(
        context,
        0i32,
        b"SMTP-idle ended.\x00" as *const u8 as *const libc::c_char,
    );
}

unsafe fn get_next_wakeup_time(context: &dc_context_t, thread: libc::c_int) -> Duration {
    let stmt = dc_sqlite3_prepare(
        context,
        &context.sql.clone().read().unwrap(),
        b"SELECT MIN(desired_timestamp) FROM jobs WHERE thread=?;\x00" as *const u8
            as *const libc::c_char,
    );
    sqlite3_bind_int(stmt, 1, thread);

    let mut wakeup_time = Duration::new(10 * 60, 0);

    if sqlite3_step(stmt) == 100 {
        let t = sqlite3_column_int(stmt, 0) as u64;
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap();
        if t > 0 {
            let t = Duration::new(t, 0);
            if t > now {
                wakeup_time = t - now;
            } else {
                wakeup_time = Duration::new(0, 0);
            }
        }
    }

    sqlite3_finalize(stmt);
    wakeup_time
}

pub unsafe fn dc_maybe_network(context: &dc_context_t) {
    {
        let &(ref lock, _) = &*context.smtp_state.clone();
        let mut state = lock.lock().unwrap();
        state.probe_network = 1;

        *context.probe_imap_network.write().unwrap() = 1;
    }

    dc_interrupt_smtp_idle(context);
    dc_interrupt_imap_idle(context);
    dc_interrupt_mvbox_idle(context);
    dc_interrupt_sentbox_idle(context);
}

pub unsafe fn dc_job_action_exists(context: &dc_context_t, action: libc::c_int) -> libc::c_int {
    let job_exists: libc::c_int;
    let stmt;
    stmt = dc_sqlite3_prepare(
        context,
        &context.sql.clone().read().unwrap(),
        b"SELECT id FROM jobs WHERE action=?;\x00" as *const u8 as *const libc::c_char,
    );
    sqlite3_bind_int(stmt, 1i32, action);
    job_exists = (sqlite3_step(stmt) == 100i32) as libc::c_int;
    sqlite3_finalize(stmt);
    return job_exists;
}
/* special case for DC_JOB_SEND_MSG_TO_SMTP */
pub unsafe fn dc_job_send_msg(context: &dc_context_t, msg_id: uint32_t) -> libc::c_int {
    let mut success: libc::c_int = 0i32;
    let mut mimefactory = dc_mimefactory_t {
        from_addr: 0 as *mut libc::c_char,
        from_displayname: 0 as *mut libc::c_char,
        selfstatus: 0 as *mut libc::c_char,
        recipients_names: 0 as *mut clist,
        recipients_addr: 0 as *mut clist,
        timestamp: 0,
        rfc724_mid: 0 as *mut libc::c_char,
        loaded: DC_MF_NOTHING_LOADED,
        msg: 0 as *mut dc_msg_t,
        chat: 0 as *mut dc_chat_t,
        increation: 0,
        in_reply_to: 0 as *mut libc::c_char,
        references: 0 as *mut libc::c_char,
        req_mdn: 0,
        out: 0 as *mut MMAPString,
        out_encrypted: 0,
        out_gossiped: 0,
        out_last_added_location_id: 0,
        error: 0 as *mut libc::c_char,
        context,
    };
    dc_mimefactory_init(&mut mimefactory, context);
    /* load message data */
    if 0 == dc_mimefactory_load_msg(&mut mimefactory, msg_id) || mimefactory.from_addr.is_null() {
        dc_log_warning(
            context,
            0i32,
            b"Cannot load data to send, maybe the message is deleted in between.\x00" as *const u8
                as *const libc::c_char,
        );
    } else {
        // no redo, no IMAP. moreover, as the data does not exist, there is no need in calling dc_set_msg_failed()
        if (*mimefactory.msg).type_0 == 20i32
            || (*mimefactory.msg).type_0 == 21i32
            || (*mimefactory.msg).type_0 == 40i32
            || (*mimefactory.msg).type_0 == 41i32
            || (*mimefactory.msg).type_0 == 50i32
            || (*mimefactory.msg).type_0 == 60i32
        {
            let pathNfilename = dc_param_get(
                (*mimefactory.msg).param,
                'f' as i32,
                0 as *const libc::c_char,
            );
            if !pathNfilename.is_null() {
                if ((*mimefactory.msg).type_0 == 20i32 || (*mimefactory.msg).type_0 == 21i32)
                    && 0 == dc_param_exists((*mimefactory.msg).param, 'w' as i32)
                {
                    let mut buf: *mut libc::c_uchar = 0 as *mut libc::c_uchar;
                    let mut buf_bytes: size_t = 0;
                    let mut w: uint32_t = 0;
                    let mut h: uint32_t = 0;
                    dc_param_set_int((*mimefactory.msg).param, 'w' as i32, 0i32);
                    dc_param_set_int((*mimefactory.msg).param, 'h' as i32, 0i32);
                    if 0 != dc_read_file(
                        context,
                        pathNfilename,
                        &mut buf as *mut *mut libc::c_uchar as *mut *mut libc::c_void,
                        &mut buf_bytes,
                    ) {
                        if 0 != dc_get_filemeta(
                            buf as *const libc::c_void,
                            buf_bytes,
                            &mut w,
                            &mut h,
                        ) {
                            dc_param_set_int((*mimefactory.msg).param, 'w' as i32, w as int32_t);
                            dc_param_set_int((*mimefactory.msg).param, 'h' as i32, h as int32_t);
                        }
                    }
                    free(buf as *mut libc::c_void);
                    dc_msg_save_param_to_disk(mimefactory.msg);
                }
            }
            free(pathNfilename as *mut libc::c_void);
        }
        /* create message */
        if 0 == dc_mimefactory_render(&mut mimefactory) {
            dc_set_msg_failed(context, msg_id, mimefactory.error);
        } else if 0 != dc_param_get_int((*mimefactory.msg).param, 'c' as i32, 0i32)
            && 0 == mimefactory.out_encrypted
        {
            dc_set_msg_failed(
                context,
                msg_id,
                b"End-to-end-encryption unavailable unexpectedly.\x00" as *const u8
                    as *const libc::c_char,
            );
        } else {
            /* unrecoverable */
            if clist_search_string_nocase(mimefactory.recipients_addr, mimefactory.from_addr)
                == 0i32
            {
                clist_insert_after(
                    mimefactory.recipients_names,
                    (*mimefactory.recipients_names).last,
                    0 as *mut libc::c_void,
                );
                clist_insert_after(
                    mimefactory.recipients_addr,
                    (*mimefactory.recipients_addr).last,
                    dc_strdup(mimefactory.from_addr) as *mut libc::c_void,
                );
            }
            if 0 != mimefactory.out_gossiped {
                dc_set_gossiped_timestamp(
                    context,
                    (*mimefactory.msg).chat_id,
                    time(0 as *mut time_t),
                );
            }
            if 0 != mimefactory.out_last_added_location_id {
                dc_set_kml_sent_timestamp(
                    context,
                    (*mimefactory.msg).chat_id,
                    time(0 as *mut time_t),
                );
                if 0 == (*mimefactory.msg).hidden {
                    dc_set_msg_location_id(
                        context,
                        (*mimefactory.msg).id,
                        mimefactory.out_last_added_location_id,
                    );
                }
            }
            if 0 != mimefactory.out_encrypted
                && dc_param_get_int((*mimefactory.msg).param, 'c' as i32, 0i32) == 0i32
            {
                dc_param_set_int((*mimefactory.msg).param, 'c' as i32, 1i32);
                dc_msg_save_param_to_disk(mimefactory.msg);
            }
            dc_add_to_keyhistory(
                context,
                0 as *const libc::c_char,
                0i32 as time_t,
                0 as *const libc::c_char,
                0 as *const libc::c_char,
            );
            success = dc_add_smtp_job(context, 5901i32, &mut mimefactory)
        }
    }
    dc_mimefactory_empty(&mut mimefactory);
    return success;
}
