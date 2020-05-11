use std::sync::{Arc, Condvar, Mutex};

use crate::context::Context;
use crate::error::{format_err, Result};
use crate::imap::Imap;

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

        self.imap.interrupt_idle(context);

        let &(ref lock, ref cvar) = &*self.state.clone();
        let mut state = lock.lock().unwrap();

        state.idle = true;
        cvar.notify_one();
        info!(context, "Interrupting {}-IDLE... finished", self.name);
    }

    pub async fn fetch(&mut self, context: &Context, use_network: bool) {
        {
            let &(ref lock, _) = &*self.state.clone();
            let mut state = lock.lock().unwrap();

            if state.suspended {
                return;
            }

            state.using_handle = true;
        }

        if use_network {
            if let Err(err) = self.connect_and_fetch(context).await {
                warn!(context, "connect+fetch failed: {}, reconnect & retry", err);
                self.imap.trigger_reconnect();
                if let Err(err) = self.connect_and_fetch(context).await {
                    warn!(context, "connect+fetch failed: {}", err);
                }
            }
        }
        self.state.0.lock().unwrap().using_handle = false;
    }

    async fn connect_and_fetch(&mut self, context: &Context) -> Result<()> {
        let prefix = format!("{}-fetch", self.name);
        self.imap.connect_configured(context)?;
        if let Some(watch_folder) = self.get_watch_folder(context) {
            let start = std::time::Instant::now();
            info!(context, "{} started...", prefix);
            let res = self
                .imap
                .fetch(context, &watch_folder)
                .await
                .map_err(Into::into);
            let elapsed = start.elapsed().as_millis();
            info!(context, "{} done in {:.3} ms.", prefix, elapsed);

            res
        } else {
            Err(format_err!("WatchFolder not found: not-set"))
        }
    }

    fn get_watch_folder(&self, context: &Context) -> Option<String> {
        match context.sql.get_raw_config(context, self.folder_config_name) {
            Some(name) => Some(name),
            None => {
                if self.folder_config_name == "configured_inbox_folder" {
                    // initialized with old version, so has not set configured_inbox_folder
                    Some("INBOX".to_string())
                } else {
                    None
                }
            }
        }
    }

    pub fn idle(&self, context: &Context, use_network: bool) {
        {
            let &(ref lock, ref cvar) = &*self.state.clone();
            let mut state = lock.lock().unwrap();

            if state.jobs_needed {
                info!(
                    context,
                    "{}-IDLE will not be started as it was interrupted while not idling.",
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
        let do_fake_idle = match self.imap.connect_configured(context) {
            Ok(()) => {
                if !self.imap.can_idle() {
                    true // we have to do fake_idle
                } else {
                    let watch_folder = self.get_watch_folder(context);
                    info!(context, "{} started...", prefix);
                    let res = self.imap.idle(context, watch_folder);
                    info!(context, "{} ended...", prefix);
                    if let Err(err) = res {
                        warn!(context, "{} failed: {} -> reconnecting", prefix, err);
                        // something is borked, let's start afresh on the next occassion
                        self.imap.disconnect(context);
                    }
                    false
                }
            }
            Err(err) => {
                info!(context, "{}-IDLE connection fail: {:?}", self.name, err);
                // if the connection fails, use fake_idle to retry periodically
                // fake_idle() will be woken up by interrupt_idle() as
                // well so will act on maybe_network events
                true
            }
        };
        if do_fake_idle {
            let watch_folder = self.get_watch_folder(context);
            self.imap.fake_idle(context, watch_folder);
        }

        self.state.0.lock().unwrap().using_handle = false;
    }
}
