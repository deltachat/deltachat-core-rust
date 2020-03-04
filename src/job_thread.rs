use async_std::sync::{channel, Arc, Mutex, Receiver, Sender};

use crate::context::Context;
use crate::error::{Error, Result};
use crate::imap::Imap;

#[derive(Debug)]
pub struct JobThread {
    pub name: &'static str,
    pub folder_config_name: &'static str,
    pub imap: Imap,
    pub state: Arc<Mutex<JobState>>,
    notify_sender: Sender<()>,
    notify_receiver: Receiver<()>,
}

#[derive(Clone, Debug, Default)]
pub struct JobState {
    jobs_needed: bool,
    suspended: bool,
    using_handle: bool,
}

impl JobThread {
    pub fn new(name: &'static str, folder_config_name: &'static str, imap: Imap) -> Self {
        let (notify_sender, notify_receiver) = channel(1);

        JobThread {
            name,
            folder_config_name,
            imap,
            state: Arc::new(Mutex::new(Default::default())),
            notify_sender,
            notify_receiver,
        }
    }

    pub async fn suspend(&self, context: &Context) {
        info!(context, "Suspending {}-thread.", self.name,);
        {
            self.state.lock().await.suspended = true;
        }
        self.interrupt_idle(context).await;
        loop {
            let using_handle = self.state.lock().await.using_handle;
            if !using_handle {
                return;
            }
            async_std::task::sleep(std::time::Duration::from_micros(300 * 1000)).await;
        }
    }

    pub async fn unsuspend(&self, context: &Context) {
        info!(context, "Unsuspending {}-thread.", self.name);

        {
            let lock = &*self.state.clone();
            let mut state = lock.lock().await;

            state.suspended = false;
        }
        self.notify_sender.send(()).await;
    }

    pub async fn try_interrupt_idle(&self, context: &Context) -> bool {
        if self.state.lock().await.using_handle {
            self.interrupt_idle(context).await;
            return true;
        }

        false
    }

    pub async fn interrupt_idle(&self, context: &Context) {
        {
            self.state.lock().await.jobs_needed = true;
        }

        info!(context, "Interrupting {}-IDLE...", self.name);

        self.imap.interrupt_idle(context).await;

        self.notify_sender.send(()).await;

        info!(context, "Interrupting {}-IDLE... finished", self.name);
    }

    pub async fn fetch(&self, context: &Context, use_network: bool) {
        {
            let lock = &*self.state.clone();
            let mut state = lock.lock().await;

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
        self.state.lock().await.using_handle = false;
    }

    async fn connect_and_fetch(&self, context: &Context) -> Result<()> {
        let prefix = format!("{}-fetch", self.name);
        match self.imap.connect_configured(context).await {
            Ok(()) => {
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
                    Err(Error::WatchFolderNotFound("not-set".to_string()))
                }
            }
            Err(err) => Err(crate::error::Error::Message(err.to_string())),
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

    pub async fn idle(&self, context: &Context, use_network: bool) {
        {
            let lock = &*self.state.clone();
            let mut state = lock.lock().await;

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
                self.notify_receiver.recv().await;
                return;
            }

            state.using_handle = true;

            if !use_network {
                state.using_handle = false;
                self.notify_receiver.recv().await;
                return;
            }
        }

        let prefix = format!("{}-IDLE", self.name);
        let do_fake_idle = match self.imap.connect_configured(context).await {
            Ok(()) => {
                if !self.imap.can_idle().await {
                    true // we have to do fake_idle
                } else {
                    let watch_folder = self.get_watch_folder(context);
                    info!(context, "{} started...", prefix);
                    let res = self.imap.idle(context, watch_folder).await;
                    info!(context, "{} ended...", prefix);
                    if let Err(err) = res {
                        warn!(context, "{} failed: {} -> reconnecting", prefix, err);
                        // something is borked, let's start afresh on the next occassion
                        self.imap.disconnect(context).await;
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
            self.imap.fake_idle(context, watch_folder).await;
        }

        self.state.lock().await.using_handle = false;
    }
}
