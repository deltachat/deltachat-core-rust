use super::Imap;

use async_imap::extensions::idle::IdleResponse;
use async_std::prelude::*;
use async_std::task;
use std::sync::atomic::Ordering;
use std::time::{Duration, SystemTime};

use crate::context::Context;
use crate::imap_client::*;

use super::select_folder;

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "IMAP IDLE protocol failed to init/complete")]
    IdleProtocolFailed(#[cause] async_imap::error::Error),

    #[fail(display = "IMAP IDLE protocol timed out")]
    IdleTimeout(#[cause] async_std::future::TimeoutError),

    #[fail(display = "IMAP server does not have IDLE capability")]
    IdleAbilityMissing,

    #[fail(display = "IMAP select folder error")]
    SelectFolderError(#[cause] select_folder::Error),

    #[fail(display = "IMAP error")]
    ImapError(#[cause] async_imap::error::Error),

    #[fail(display = "Setup handle error")]
    SetupHandleError(#[cause] super::Error),
}

impl From<select_folder::Error> for Error {
    fn from(err: select_folder::Error) -> Error {
        Error::SelectFolderError(err)
    }
}

impl Imap {
    pub fn can_idle(&self) -> bool {
        task::block_on(async move { self.config.read().await.can_idle })
    }

    pub fn idle(&self, context: &Context, watch_folder: Option<String>) -> Result<()> {
        task::block_on(async move {
            if !self.can_idle() {
                return Err(Error::IdleAbilityMissing);
            }

            self.setup_handle_if_needed(context)
                .await
                .map_err(Error::SetupHandleError)?;

            self.select_folder(context, watch_folder.clone()).await?;

            let session = self.session.lock().await.take();
            let timeout = Duration::from_secs(23 * 60);
            if let Some(session) = session {
                match session.idle() {
                    // BEWARE: If you change the Secure branch you
                    // typically also need to change the Insecure branch.
                    IdleHandle::Secure(mut handle) => {
                        if let Err(err) = handle.init().await {
                            return Err(Error::IdleProtocolFailed(err));
                        }

                        let (idle_wait, interrupt) = handle.wait_with_timeout(timeout);
                        *self.interrupt.lock().await = Some(interrupt);

                        if self.skip_next_idle_wait.load(Ordering::SeqCst) {
                            // interrupt_idle has happened before we
                            // provided self.interrupt
                            self.skip_next_idle_wait.store(false, Ordering::SeqCst);
                            std::mem::drop(idle_wait);
                            info!(context, "Idle wait was skipped");
                        } else {
                            info!(context, "Idle entering wait-on-remote state");
                            match idle_wait.await {
                                Ok(IdleResponse::NewData(_)) => {
                                    info!(context, "Idle has NewData");
                                }
                                // TODO: idle_wait does not distinguish manual interrupts
                                // from Timeouts if we would know it's a Timeout we could bail
                                // directly and reconnect .
                                Ok(IdleResponse::Timeout) => {
                                    info!(context, "Idle-wait timeout or interruption");
                                }
                                Ok(IdleResponse::ManualInterrupt) => {
                                    info!(context, "Idle wait was interrupted");
                                }
                                Err(err) => {
                                    warn!(context, "Idle wait errored: {:?}", err);
                                }
                            }
                        }
                        // if we can't properly terminate the idle
                        // protocol let's break the connection.
                        let res =
                            async_std::future::timeout(Duration::from_secs(15), handle.done())
                                .await
                                .map_err(|err| {
                                    self.trigger_reconnect();
                                    Error::IdleTimeout(err)
                                })?;

                        match res {
                            Ok(session) => {
                                *self.session.lock().await = Some(Session::Secure(session));
                            }
                            Err(err) => {
                                // if we cannot terminate IDLE it probably
                                // means that we waited long (with idle_wait)
                                // but the network went away/changed
                                self.trigger_reconnect();
                                return Err(Error::IdleProtocolFailed(err));
                            }
                        }
                    }
                    IdleHandle::Insecure(mut handle) => {
                        if let Err(err) = handle.init().await {
                            return Err(Error::IdleProtocolFailed(err));
                        }

                        let (idle_wait, interrupt) = handle.wait_with_timeout(timeout);
                        *self.interrupt.lock().await = Some(interrupt);

                        if self.skip_next_idle_wait.load(Ordering::SeqCst) {
                            // interrupt_idle has happened before we
                            // provided self.interrupt
                            self.skip_next_idle_wait.store(false, Ordering::SeqCst);
                            std::mem::drop(idle_wait);
                            info!(context, "Idle wait was skipped");
                        } else {
                            info!(context, "Idle entering wait-on-remote state");
                            match idle_wait.await {
                                Ok(IdleResponse::NewData(_)) => {
                                    info!(context, "Idle has NewData");
                                }
                                // TODO: idle_wait does not distinguish manual interrupts
                                // from Timeouts if we would know it's a Timeout we could bail
                                // directly and reconnect .
                                Ok(IdleResponse::Timeout) => {
                                    info!(context, "Idle-wait timeout or interruption");
                                }
                                Ok(IdleResponse::ManualInterrupt) => {
                                    info!(context, "Idle wait was interrupted");
                                }
                                Err(err) => {
                                    warn!(context, "Idle wait errored: {:?}", err);
                                }
                            }
                        }
                        // if we can't properly terminate the idle
                        // protocol let's break the connection.
                        let res =
                            async_std::future::timeout(Duration::from_secs(15), handle.done())
                                .await
                                .map_err(|err| {
                                    self.trigger_reconnect();
                                    Error::IdleTimeout(err)
                                })?;

                        match res {
                            Ok(session) => {
                                *self.session.lock().await = Some(Session::Insecure(session));
                            }
                            Err(err) => {
                                // if we cannot terminate IDLE it probably
                                // means that we waited long (with idle_wait)
                                // but the network went away/changed
                                self.trigger_reconnect();
                                return Err(Error::IdleProtocolFailed(err));
                            }
                        }
                    }
                }
            }

            Ok(())
        })
    }

    pub(crate) fn fake_idle(&self, context: &Context, watch_folder: Option<String>) {
        // Idle using polling. This is also needed if we're not yet configured -
        // in this case, we're waiting for a configure job (and an interrupt).
        task::block_on(async move {
            let fake_idle_start_time = SystemTime::now();

            info!(context, "IMAP-fake-IDLEing...");

            let interrupt = stop_token::StopSource::new();

            // check every minute if there are new messages
            // TODO: grow sleep durations / make them more flexible
            let interval = async_std::stream::interval(Duration::from_secs(60));
            let mut interrupt_interval = interrupt.stop_token().stop_stream(interval);
            *self.interrupt.lock().await = Some(interrupt);
            if self.skip_next_idle_wait.load(Ordering::SeqCst) {
                // interrupt_idle has happened before we
                // provided self.interrupt
                self.skip_next_idle_wait.store(false, Ordering::SeqCst);
                info!(context, "fake-idle wait was skipped");
            } else {
                // loop until we are interrupted or if we fetched something
                while let Some(_) = interrupt_interval.next().await {
                    // try to connect with proper login params
                    // (setup_handle_if_needed might not know about them if we
                    // never successfully connected)
                    if let Err(err) = self.connect_configured(context) {
                        warn!(context, "fake_idle: could not connect: {}", err);
                        continue;
                    }
                    if self.config.read().await.can_idle {
                        // we only fake-idled because network was gone during IDLE, probably
                        break;
                    }
                    info!(context, "fake_idle is connected");
                    // we are connected, let's see if fetching messages results
                    // in anything.  If so, we behave as if IDLE had data but
                    // will have already fetched the messages so perform_*_fetch
                    // will not find any new.

                    if let Some(ref watch_folder) = watch_folder {
                        match self.fetch_new_messages(context, watch_folder).await {
                            Ok(res) => {
                                info!(context, "fetch_new_messages returned {:?}", res);
                                if res {
                                    break;
                                }
                            }
                            Err(err) => {
                                error!(context, "could not fetch from folder: {}", err);
                                self.trigger_reconnect()
                            }
                        }
                    }
                }
            }
            self.interrupt.lock().await.take();

            info!(
                context,
                "IMAP-fake-IDLE done after {:.4}s",
                SystemTime::now()
                    .duration_since(fake_idle_start_time)
                    .unwrap_or_default()
                    .as_millis() as f64
                    / 1000.,
            );
        })
    }

    pub fn interrupt_idle(&self, context: &Context) {
        task::block_on(async move {
            let mut interrupt: Option<stop_token::StopSource> = self.interrupt.lock().await.take();
            if interrupt.is_none() {
                // idle wait is not running, signal it needs to skip
                self.skip_next_idle_wait.store(true, Ordering::SeqCst);

                // meanwhile idle-wait may have produced the StopSource
                interrupt = self.interrupt.lock().await.take();
            }
            // let's manually drop the StopSource
            if interrupt.is_some() {
                // the imap thread provided us a stop token but might
                // not have entered idle_wait yet, give it some time
                // for that to happen. XXX handle this without extra wait
                // https://github.com/deltachat/deltachat-core-rust/issues/925
                std::thread::sleep(Duration::from_millis(200));
                info!(context, "low-level: dropping stop-source to interrupt idle");
                std::mem::drop(interrupt)
            }
        });
    }
}
