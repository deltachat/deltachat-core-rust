use std::sync::{Arc, Condvar, Mutex};

use crate::configure::*;
use crate::context::Context;
use crate::error::{Error, Result};
use crate::imap::{IdlePollMode, Imap};

#[derive(Debug)]
pub struct JobThread {
    pub name: &'static str,
    pub folder_config_name: &'static str,
    pub imap: Imap,
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
    pub fn new(name: &'static str, folder_config_name: &'static str, imap: Imap) -> Self {
        JobThread {
            name,
            folder_config_name,
            imap,
            state: Arc::new((Mutex::new(Default::default()), Condvar::new())),
        }
    }

    pub fn suspend(&self, context: &Context) {
        info!(context, "Suspending {}-thread.", self.name,);
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
        info!(context, "Unsuspending {}-thread.", self.name);

        let &(ref lock, ref cvar) = &*self.state.clone();
        let mut state = lock.lock().unwrap();

        state.suspended = false;
        state.idle = true;
        cvar.notify_one();
    }

    pub fn interrupt_idle(&self, context: &Context) {
        {
            self.state.0.lock().unwrap().jobs_needed = true;
        }

        info!(context, "Interrupting {}-IDLE...", self.name);

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
            let prefix = format!("{}-fetch", self.name);
            match self.connect_to_imap(context) {
                Ok(()) => {
                    let start = std::time::Instant::now();
                    info!(context, "{} started...", prefix);
                    self.imap.fetch(context);

                    if self.imap.should_reconnect() {
                        info!(context, "{} aborted, starting over...", prefix);
                        self.imap.fetch(context);
                    }
                    info!(
                        context,
                        "{} done in {:.3} ms.",
                        prefix,
                        start.elapsed().as_millis(),
                    );
                }
                Err(err) => {
                    warn!(
                        context,
                        "{} skipped, could not connect to imap {:?}", prefix, err
                    );
                }
            }
        }

        self.state.0.lock().unwrap().using_handle = false;
    }

    pub fn connect_to_imap(&self, context: &Context) -> Result<()> {
        if async_std::task::block_on(async move { self.imap.is_connected().await }) {
            return Ok(());
        }
        let watch_folder_name = match context.sql.get_raw_config(context, self.folder_config_name) {
            Some(name) => name,
            None => {
                return Err(Error::WatchFolderNotFound(
                    self.folder_config_name.to_string(),
                ));
            }
        };

        dc_connect_to_configured_imap(context, &self.imap)?;
        if context
            .sql
            .get_raw_config_int(context, "folders_configured")
            .unwrap_or_default()
            < 3
        {
            self.imap.configure_folders(context, 0x1);
        }
        self.imap.set_watch_folder(watch_folder_name);

        Ok(())
    }

    pub fn idle(&self, context: &Context, use_network: bool) {
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

        let prefix = format!("{}-IDLE", self.name);
        let poll_mode = match self.connect_to_imap(context) {
            Ok(()) => {
                info!(context, "{} started...", prefix);
                let res = self.imap.idle(context);
                info!(context, "{} ended...", prefix);
                match res {
                    Ok(()) => None,
                    Err(Error::ImapConnectionFailed(err))
                    | Err(Error::ImapIdleProtocolFailed(err)) => {
                        self.imap.trigger_reconnect();
                        warn!(context, "{} failed: {}, reconnecting", prefix, err);
                        Some(IdlePollMode::Often)
                    }
                    Err(Error::ImapInTeardown) => {
                        warn!(context, "{} aborting as imap is in teardown", prefix);
                        None
                    }
                    Err(err) => {
                        warn!(context, "{} failed fundamentally: {}", prefix, err);
                        Some(IdlePollMode::Never)
                    }
                }
            }
            Err(err) => {
                info!(context, "{}-IDLE connection fail: {:?}", self.name, err);
                Some(IdlePollMode::Often)
            }
        };
        if let Some(poll_mode) = poll_mode {
            self.imap.fake_idle(context, poll_mode);
        }

        self.state.0.lock().unwrap().using_handle = false;
    }
}
