use std::sync::{Arc, Condvar, Mutex};

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
    pub name: *mut libc::c_char,
    pub folder_config_name: *mut libc::c_char,
    pub imap: Arc<Mutex<dc_imap_t>>,
    pub state: Arc<(Mutex<JobState>, Condvar)>,
}

pub unsafe fn dc_jobthread_init(
    name: *const libc::c_char,
    folder_config_name: *const libc::c_char,
    imap: dc_imap_t,
) -> dc_jobthread_t {
    dc_jobthread_t {
        name: dc_strdup(name),
        folder_config_name: dc_strdup(folder_config_name),
        imap: Arc::new(Mutex::new(imap)),
        state: Arc::new((Mutex::new(Default::default()), Condvar::new())),
    }
}

#[derive(Debug, Default)]
pub struct JobState {
    idle: bool,
    jobs_needed: i32,
    suspended: i32,
    using_handle: i32,
}

pub unsafe fn dc_jobthread_exit(jobthread: &mut dc_jobthread_t) {
    free(jobthread.name as *mut libc::c_void);
    jobthread.name = 0 as *mut libc::c_char;
    free(jobthread.folder_config_name as *mut libc::c_void);
    jobthread.folder_config_name = 0 as *mut libc::c_char;
}

pub unsafe fn dc_jobthread_suspend(
    context: &dc_context_t,
    jobthread: &mut dc_jobthread_t,
    suspend: libc::c_int,
) {
    if 0 != suspend {
        dc_log_info(
            context,
            0i32,
            b"Suspending %s-thread.\x00" as *const u8 as *const libc::c_char,
            jobthread.name,
        );

        {
            jobthread.state.clone().0.lock().unwrap().suspended = 1;
        }
        dc_jobthread_interrupt_idle(context, jobthread);
        loop {
            let using_handle = jobthread.state.clone().0.lock().unwrap().using_handle;
            if using_handle == 0 {
                return;
            }
            usleep((300i32 * 1000i32) as useconds_t);
        }
    } else {
        dc_log_info(
            context,
            0i32,
            b"Unsuspending %s-thread.\x00" as *const u8 as *const libc::c_char,
            jobthread.name,
        );

        let &(ref lock, ref cvar) = &*jobthread.state.clone();
        let mut state = lock.lock().unwrap();

        state.suspended = 0;
        state.idle = true;
        cvar.notify_one();
    }
}

pub unsafe extern "C" fn dc_jobthread_interrupt_idle(
    context: &dc_context_t,
    jobthread: &mut dc_jobthread_t,
) {
    {
        jobthread.state.clone().0.lock().unwrap().jobs_needed = 1;
    }

    dc_log_info(
        context,
        0,
        b"Interrupting %s-IDLE...\x00" as *const u8 as *const libc::c_char,
        jobthread.name,
    );

    dc_imap_interrupt_idle(&mut jobthread.imap.clone().lock().unwrap());

    let &(ref lock, ref cvar) = &*jobthread.state.clone();
    let mut state = lock.lock().unwrap();

    state.idle = true;
    cvar.notify_one();
}

pub unsafe fn dc_jobthread_fetch(
    context: &dc_context_t,
    jobthread: &mut dc_jobthread_t,
    use_network: libc::c_int,
) {
    let mut start;

    {
        let &(ref lock, _) = &*jobthread.state.clone();
        let mut state = lock.lock().unwrap();

        if 0 != state.suspended {
            return;
        }

        state.using_handle = 1;
    }

    if !0 == use_network {
        start = clock();
        if !(0 == connect_to_imap(context, jobthread)) {
            dc_log_info(
                context,
                0,
                b"%s-fetch started...\x00" as *const u8 as *const libc::c_char,
                jobthread.name,
            );
            dc_imap_fetch(context, &mut jobthread.imap.clone().lock().unwrap());

            if 0 != jobthread.imap.clone().lock().unwrap().should_reconnect {
                dc_log_info(
                    context,
                    0i32,
                    b"%s-fetch aborted, starting over...\x00" as *const u8 as *const libc::c_char,
                    jobthread.name,
                );
                dc_imap_fetch(context, &mut jobthread.imap.clone().lock().unwrap());
            }
            dc_log_info(
                context,
                0,
                b"%s-fetch done in %.0f ms.\x00" as *const u8 as *const libc::c_char,
                jobthread.name,
                clock().wrapping_sub(start) as libc::c_double * 1000.0f64
                    / 1000000i32 as libc::c_double,
            );
        }
    }

    jobthread.state.clone().0.lock().unwrap().using_handle = 0;
}

/* ******************************************************************************
 * the typical fetch, idle, interrupt-idle
 ******************************************************************************/

unsafe fn connect_to_imap(context: &dc_context_t, jobthread: &mut dc_jobthread_t) -> libc::c_int {
    let mut ret_connected: libc::c_int;
    let mut mvbox_name: *mut libc::c_char = 0 as *mut libc::c_char;

    if 0 != dc_imap_is_connected(&mut jobthread.imap.clone().lock().unwrap()) {
        ret_connected = 1
    } else {
        ret_connected =
            dc_connect_to_configured_imap(context, &mut jobthread.imap.clone().lock().unwrap());
        if !(0 == ret_connected) {
            if dc_sqlite3_get_config_int(
                context,
                &context.sql.clone().read().unwrap(),
                b"folders_configured\x00" as *const u8 as *const libc::c_char,
                0,
            ) < 3
            {
                dc_configure_folders(context, &mut jobthread.imap.clone().lock().unwrap(), 0x1);
            }
            mvbox_name = dc_sqlite3_get_config(
                context,
                &context.sql.clone().read().unwrap(),
                jobthread.folder_config_name,
                0 as *const libc::c_char,
            );
            if mvbox_name.is_null() {
                dc_imap_disconnect(context, &mut jobthread.imap.clone().lock().unwrap());
                ret_connected = 0;
            } else {
                dc_imap_set_watch_folder(&mut jobthread.imap.clone().lock().unwrap(), mvbox_name);
            }
        }
    }
    free(mvbox_name as *mut libc::c_void);

    ret_connected
}

pub unsafe fn dc_jobthread_idle(
    context: &dc_context_t,
    jobthread: &mut dc_jobthread_t,
    use_network: libc::c_int,
) {
    {
        let &(ref lock, ref cvar) = &*jobthread.state.clone();
        let mut state = lock.lock().unwrap();

        if 0 != state.jobs_needed {
            dc_log_info(
                context,
                0,
                b"%s-IDLE will not be started as it was interrupted while not ideling.\x00"
                    as *const u8 as *const libc::c_char,
                jobthread.name,
            );
            state.jobs_needed = 0;
            return;
        }

        if 0 != state.suspended {
            while !state.idle {
                state = cvar.wait(state).unwrap();
            }
            state.idle = false;
            return;
        }

        state.using_handle = 1;

        if 0 == use_network {
            state.using_handle = 0;

            while !state.idle {
                state = cvar.wait(state).unwrap();
            }
            state.idle = false;
            return;
        }
    }

    connect_to_imap(context, jobthread);
    dc_log_info(
        context,
        0i32,
        b"%s-IDLE started...\x00" as *const u8 as *const libc::c_char,
        jobthread.name,
    );
    dc_imap_idle(context, &mut jobthread.imap.clone().lock().unwrap());
    dc_log_info(
        context,
        0i32,
        b"%s-IDLE ended.\x00" as *const u8 as *const libc::c_char,
        jobthread.name,
    );

    jobthread.state.clone().0.lock().unwrap().using_handle = 0;
}
