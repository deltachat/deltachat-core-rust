use std::sync::{Arc, Condvar, Mutex};

use crate::configure::*;
use crate::context::Context;
use crate::imap::Imap;

#[derive(Debug)]
pub struct JobThread {
    pub name: &'static str,
    pub folder_config_name: &'static str,
    pub state: Arc<(Mutex<JobState>, Condvar)>,
}

#[derive(Clone, Debug, Default)]
pub struct JobState {
    idle: bool,
    jobs_needed: bool,
    suspended: bool,
    using_handle: bool,
}

impl JobThread {
    pub fn new(name: &'static str, folder_config_name: &'static str) -> Self {
        JobThread {
            name,
            folder_config_name,
            state: Arc::new((Mutex::new(Default::default()), Condvar::new())),
        }
    }

    pub fn suspend(&self, context: &Context, imap: &mut Imap) {
        info!(context, "Suspending {}-thread.", self.name,);
        {
            self.state.0.lock().unwrap().suspended = true;
        }
        self.interrupt_idle(context, imap);
        loop {
            let using_handle = self.state.0.lock().unwrap().using_handle;
            if !using_handle {
                return;
            }
            std::thread::sleep(std::time::Duration::from_micros(300 * 1000));
        }
    }

    pub fn unsuspend(&self, context: &Context) {
        info!(context, "Unsuspending {}-thread.", self.name);

        let &(ref lock, ref cvar) = &*self.state.clone();
        let mut state = lock.lock().unwrap();

        state.suspended = false;
        state.idle = true;
        cvar.notify_one();
    }

    pub fn interrupt_idle(&self, context: &Context, imap: &mut Imap) {
        {
            self.state.0.lock().unwrap().jobs_needed = true;
        }

        info!(context, "Interrupting {}-IDLE...", self.name);

        imap.interrupt_idle();

        let &(ref lock, ref cvar) = &*self.state.clone();
        let mut state = lock.lock().unwrap();

        state.idle = true;
        cvar.notify_one();
    }

    pub fn fetch(&self, context: &Context, use_network: bool, imap: &mut Imap) {
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
            if self.connect_to_imap(context, imap) {
                info!(context, "{}-fetch started...", self.name);
                imap.fetch(context);

                if imap.should_reconnect() {
                    info!(context, "{}-fetch aborted, starting over...", self.name,);
                    imap.fetch(context);
                }
                info!(
                    context,
                    "{}-fetch done in {:.3} ms.",
                    self.name,
                    start.elapsed().as_millis(),
                );
            }
        }

        self.state.0.lock().unwrap().using_handle = false;
    }

    fn connect_to_imap(&self, context: &Context, imap: &mut Imap) -> bool {
        async_std::task::block_on(async move {
            if imap.is_connected().await {
                return true;
            }

            let mut ret_connected = dc_connect_to_configured_imap(context, imap) != 0;

            if ret_connected {
                if context
                    .sql
                    .get_raw_config_int(context, "folders_configured")
                    .unwrap_or_default()
                    < 3
                {
                    imap.configure_folders(context, 0x1);
                }

                if let Some(mvbox_name) =
                    context.sql.get_raw_config(context, self.folder_config_name)
                {
                    imap.set_watch_folder(mvbox_name);
                } else {
                    imap.disconnect();
                    ret_connected = false;
                }
            }

            ret_connected
        })
    }

    pub fn idle(&self, context: &Context, use_network: bool, imap: &mut Imap) {
        {
            let &(ref lock, ref cvar) = &*self.state.clone();
            let mut state = lock.lock().unwrap();

            if state.jobs_needed {
                info!(
                    context,
                    "{}-IDLE will not be started as it was interrupted while not ideling.",
                    self.name,
                );
                state.jobs_needed = false;
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

        self.connect_to_imap(context, imap);
        info!(context, "{}-IDLE started...", self.name);
        imap.idle(context);
        info!(context, "{}-IDLE ended.", self.name);

        self.state.0.lock().unwrap().using_handle = false;
    }
}
