use std::sync::{Arc, Condvar, Mutex};

use crate::context::Context;
use crate::dc_configure::*;
use crate::imap::Imap;

pub struct JobThread {
    pub name: &'static str,
    pub folder_config_name: &'static str,
    pub imap: Imap,
    pub state: Arc<(Mutex<JobState>, Condvar)>,
}

#[derive(Clone, Debug, Default)]
pub struct JobState {
    idle: bool,
    jobs_needed: i32,
    suspended: bool,
    using_handle: bool,
}

impl JobThread {
    pub fn new(name: &'static str, folder_config_name: &'static str, imap: Imap) -> Self {
        JobThread {
            name,
            folder_config_name,
            imap,
            state: Arc::new((Mutex::new(Default::default()), Condvar::new())),
        }
    }

    pub fn suspend(&self, context: &Context) {
        info!(context, 0, "Suspending {}-thread.", self.name,);
        {
            self.state.0.lock().unwrap().suspended = true;
        }
        self.interrupt_idle(context);
        loop {
            let using_handle = self.state.0.lock().unwrap().using_handle;
            if !using_handle {
                return;
            }
            std::thread::sleep(std::time::Duration::from_micros(300 * 1000));
        }
    }

    pub fn unsuspend(&self, context: &Context) {
        info!(context, 0, "Unsuspending {}-thread.", self.name);

        let &(ref lock, ref cvar) = &*self.state.clone();
        let mut state = lock.lock().unwrap();

        state.suspended = false;
        state.idle = true;
        cvar.notify_one();
    }

    pub fn interrupt_idle(&self, context: &Context) {
        {
            self.state.0.lock().unwrap().jobs_needed = 1;
        }

        info!(context, 0, "Interrupting {}-IDLE...", self.name);

        self.imap.interrupt_idle();

        let &(ref lock, ref cvar) = &*self.state.clone();
        let mut state = lock.lock().unwrap();

        state.idle = true;
        cvar.notify_one();
    }

    pub fn fetch(&mut self, context: &Context, use_network: bool) {
        {
            let &(ref lock, _) = &*self.state.clone();
            let mut state = lock.lock().unwrap();

            if state.suspended {
                return;
            }

            state.using_handle = true;
        }

        if use_network {
            let start = std::time::Instant::now();
            if self.connect_to_imap(context) {
                info!(context, 0, "{}-fetch started...", self.name);
                self.imap.fetch(context);

                if self.imap.should_reconnect() {
                    info!(context, 0, "{}-fetch aborted, starting over...", self.name,);
                    self.imap.fetch(context);
                }
                info!(
                    context,
                    0,
                    "{}-fetch done in {:.3} ms.",
                    self.name,
                    start.elapsed().as_millis(),
                );
            }
        }

        self.state.0.lock().unwrap().using_handle = false;
    }

    fn connect_to_imap(&self, context: &Context) -> bool {
        if self.imap.is_connected() {
            return true;
        }

        let mut ret_connected = dc_connect_to_configured_imap(context, &self.imap) != 0;

        if ret_connected {
            if context
                .sql
                .get_config_int(context, "folders_configured")
                .unwrap_or_default()
                < 3
            {
                self.imap.configure_folders(context, 0x1);
            }

            if let Some(mvbox_name) = context.sql.get_config(context, self.folder_config_name) {
                self.imap.set_watch_folder(mvbox_name);
            } else {
                self.imap.disconnect(context);
                ret_connected = false;
            }
        }

        ret_connected
    }

    pub fn idle(&self, context: &Context, use_network: bool) {
        {
            let &(ref lock, ref cvar) = &*self.state.clone();
            let mut state = lock.lock().unwrap();

            if 0 != state.jobs_needed {
                info!(
                    context,
                    0,
                    "{}-IDLE will not be started as it was interrupted while not ideling.",
                    self.name,
                );
                state.jobs_needed = 0;
                return;
            }

            if state.suspended {
                while !state.idle {
                    state = cvar.wait(state).unwrap();
                }
                state.idle = false;
                return;
            }

            state.using_handle = true;

            if !use_network {
                state.using_handle = false;

                while !state.idle {
                    state = cvar.wait(state).unwrap();
                }
                state.idle = false;
                return;
            }
        }

        self.connect_to_imap(context);
        info!(context, 0, "{}-IDLE started...", self.name,);
        self.imap.idle(context);
        info!(context, 0, "{}-IDLE ended.", self.name);

        self.state.0.lock().unwrap().using_handle = false;
    }
}
