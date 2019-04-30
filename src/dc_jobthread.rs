use std::sync::{Condvar, Mutex};

use libc;

use crate::dc_configure::*;
use crate::dc_context::dc_context_t;
use crate::dc_imap::dc_imap_t;
use crate::dc_imap::*;
use crate::dc_log::*;
use crate::dc_sqlite3::*;
use crate::dc_tools::*;
use crate::types::*;
use crate::x::*;

#[repr(C)]
pub struct dc_jobthread_t {
    // TODO: remove
    pub context: *mut dc_context_t,
    pub name: *mut libc::c_char,
    pub folder_config_name: *mut libc::c_char,
    pub imap: *mut dc_imap_t,
    pub idle: (Mutex<bool>, Condvar),
    pub jobs_needed: libc::c_int,
    pub suspended: libc::c_int,
    pub using_handle: libc::c_int,
}

pub unsafe fn dc_jobthread_init(
    name: *const libc::c_char,
    folder_config_name: *const libc::c_char,
    imap: *mut dc_imap_t,
) -> dc_jobthread_t {
    dc_jobthread_t {
        context: std::ptr::null_mut(),
        name: dc_strdup(name),
        folder_config_name: dc_strdup(folder_config_name),
        imap,
        idle: (Mutex::new(false), Condvar::new()),
        jobs_needed: 0i32,
        suspended: 0i32,
        using_handle: 0i32,
    }
}

pub unsafe fn dc_jobthread_exit(mut jobthread: *mut dc_jobthread_t) {
    if jobthread.is_null() {
        return;
    }

    free((*jobthread).name as *mut libc::c_void);
    (*jobthread).name = 0 as *mut libc::c_char;
    free((*jobthread).folder_config_name as *mut libc::c_void);
    (*jobthread).folder_config_name = 0 as *mut libc::c_char;
}

pub unsafe fn dc_jobthread_suspend(mut jobthread: *mut dc_jobthread_t, mut suspend: libc::c_int) {
    if jobthread.is_null() {
        return;
    }
    if 0 != suspend {
        dc_log_info(
            (*jobthread).context,
            0i32,
            b"Suspending %s-thread.\x00" as *const u8 as *const libc::c_char,
            (*jobthread).name,
        );
        pthread_mutex_lock(&mut (*jobthread).mutex);
        (*jobthread).suspended = 1i32;
        pthread_mutex_unlock(&mut (*jobthread).mutex);
        dc_jobthread_interrupt_idle(jobthread);
        loop {
            pthread_mutex_lock(&mut (*jobthread).mutex);
            if (*jobthread).using_handle == 0i32 {
                pthread_mutex_unlock(&mut (*jobthread).mutex);
                return;
            }
            pthread_mutex_unlock(&mut (*jobthread).mutex);
            usleep((300i32 * 1000i32) as useconds_t);
        }
    } else {
        dc_log_info(
            (*jobthread).context,
            0i32,
            b"Unsuspending %s-thread.\x00" as *const u8 as *const libc::c_char,
            (*jobthread).name,
        );
        pthread_mutex_lock(&mut (*jobthread).mutex);
        (*jobthread).suspended = 0i32;
        (*jobthread).idle_condflag = 1i32;
        pthread_cond_signal(&mut (*jobthread).idle_cond);
        pthread_mutex_unlock(&mut (*jobthread).mutex);
    };
}
pub unsafe extern "C" fn dc_jobthread_interrupt_idle(mut jobthread: *mut dc_jobthread_t) {
    if jobthread.is_null() {
        return;
    }
    pthread_mutex_lock(&mut (*jobthread).mutex);
    (*jobthread).jobs_needed = 1i32;
    pthread_mutex_unlock(&mut (*jobthread).mutex);
    dc_log_info(
        (*jobthread).context,
        0i32,
        b"Interrupting %s-IDLE...\x00" as *const u8 as *const libc::c_char,
        (*jobthread).name,
    );
    if !(*jobthread).imap.is_null() {
        dc_imap_interrupt_idle((*jobthread).imap);
    }
    pthread_mutex_lock(&mut (*jobthread).mutex);
    (*jobthread).idle_condflag = 1i32;
    pthread_cond_signal(&mut (*jobthread).idle_cond);
    pthread_mutex_unlock(&mut (*jobthread).mutex);
}
pub unsafe fn dc_jobthread_fetch(mut jobthread: *mut dc_jobthread_t, mut use_network: libc::c_int) {
    let mut start: libc::clock_t = 0;
    if jobthread.is_null() {
        return;
    }
    pthread_mutex_lock(&mut (*jobthread).mutex);
    if 0 != (*jobthread).suspended {
        pthread_mutex_unlock(&mut (*jobthread).mutex);
        return;
    }
    (*jobthread).using_handle = 1i32;
    pthread_mutex_unlock(&mut (*jobthread).mutex);
    if !(0 == use_network || (*jobthread).imap.is_null()) {
        start = clock();
        if !(0 == connect_to_imap(jobthread)) {
            dc_log_info(
                (*jobthread).context,
                0i32,
                b"%s-fetch started...\x00" as *const u8 as *const libc::c_char,
                (*jobthread).name,
            );
            dc_imap_fetch((*jobthread).imap);
            if 0 != (*(*jobthread).imap).should_reconnect {
                dc_log_info(
                    (*jobthread).context,
                    0i32,
                    b"%s-fetch aborted, starting over...\x00" as *const u8 as *const libc::c_char,
                    (*jobthread).name,
                );
                dc_imap_fetch((*jobthread).imap);
            }
            dc_log_info(
                (*jobthread).context,
                0i32,
                b"%s-fetch done in %.0f ms.\x00" as *const u8 as *const libc::c_char,
                (*jobthread).name,
                clock().wrapping_sub(start) as libc::c_double * 1000.0f64
                    / 1000000i32 as libc::c_double,
            );
        }
    }
    pthread_mutex_lock(&mut (*jobthread).mutex);
    (*jobthread).using_handle = 0i32;
    pthread_mutex_unlock(&mut (*jobthread).mutex);
}
/* ******************************************************************************
 * the typical fetch, idle, interrupt-idle
 ******************************************************************************/
unsafe fn connect_to_imap(mut jobthread: *mut dc_jobthread_t) -> libc::c_int {
    let mut ret_connected: libc::c_int = 0i32;
    let mut mvbox_name: *mut libc::c_char = 0 as *mut libc::c_char;
    if 0 != dc_imap_is_connected((*jobthread).imap) {
        ret_connected = 1i32
    } else {
        ret_connected = dc_connect_to_configured_imap((*jobthread).context, (*jobthread).imap);
        if !(0 == ret_connected) {
            if dc_sqlite3_get_config_int(
                (*(*jobthread).context).sql,
                b"folders_configured\x00" as *const u8 as *const libc::c_char,
                0i32,
            ) < 3i32
            {
                dc_configure_folders((*jobthread).context, (*jobthread).imap, 0x1i32);
            }
            mvbox_name = dc_sqlite3_get_config(
                (*(*jobthread).context).sql,
                (*jobthread).folder_config_name,
                0 as *const libc::c_char,
            );
            if mvbox_name.is_null() {
                dc_imap_disconnect((*jobthread).imap);
                ret_connected = 0i32
            } else {
                dc_imap_set_watch_folder((*jobthread).imap, mvbox_name);
            }
        }
    }
    free(mvbox_name as *mut libc::c_void);
    return ret_connected;
}
pub unsafe fn dc_jobthread_idle(mut jobthread: *mut dc_jobthread_t, mut use_network: libc::c_int) {
    if jobthread.is_null() {
        return;
    }
    pthread_mutex_lock(&mut (*jobthread).mutex);
    if 0 != (*jobthread).jobs_needed {
        dc_log_info(
            (*jobthread).context,
            0i32,
            b"%s-IDLE will not be started as it was interrupted while not ideling.\x00" as *const u8
                as *const libc::c_char,
            (*jobthread).name,
        );
        (*jobthread).jobs_needed = 0i32;
        pthread_mutex_unlock(&mut (*jobthread).mutex);
        return;
    }
    if 0 != (*jobthread).suspended {
        while (*jobthread).idle_condflag == 0i32 {
            pthread_cond_wait(&mut (*jobthread).idle_cond, &mut (*jobthread).mutex);
        }
        (*jobthread).idle_condflag = 0i32;
        pthread_mutex_unlock(&mut (*jobthread).mutex);
        return;
    }
    (*jobthread).using_handle = 1i32;
    pthread_mutex_unlock(&mut (*jobthread).mutex);
    if 0 == use_network || (*jobthread).imap.is_null() {
        pthread_mutex_lock(&mut (*jobthread).mutex);
        (*jobthread).using_handle = 0i32;
        while (*jobthread).idle_condflag == 0i32 {
            pthread_cond_wait(&mut (*jobthread).idle_cond, &mut (*jobthread).mutex);
        }
        (*jobthread).idle_condflag = 0i32;
        pthread_mutex_unlock(&mut (*jobthread).mutex);
        return;
    }
    connect_to_imap(jobthread);
    dc_log_info(
        (*jobthread).context,
        0i32,
        b"%s-IDLE started...\x00" as *const u8 as *const libc::c_char,
        (*jobthread).name,
    );
    dc_imap_idle((*jobthread).imap);
    dc_log_info(
        (*jobthread).context,
        0i32,
        b"%s-IDLE ended.\x00" as *const u8 as *const libc::c_char,
        (*jobthread).name,
    );
    pthread_mutex_lock(&mut (*jobthread).mutex);
    (*jobthread).using_handle = 0i32;
    pthread_mutex_unlock(&mut (*jobthread).mutex);
}
