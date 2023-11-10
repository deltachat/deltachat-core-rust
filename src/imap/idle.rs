use std::time::{Duration, SystemTime};

use anyhow::{bail, Context as _, Result};
use async_channel::Receiver;
use async_imap::extensions::idle::IdleResponse;
use futures_lite::FutureExt;

use super::session::Session;
use super::Imap;
use crate::config::Config;
use crate::context::Context;
use crate::imap::{client::IMAP_TIMEOUT, get_uid_next, FolderMeaning};
use crate::log::LogExt;

const IDLE_TIMEOUT: Duration = Duration::from_secs(23 * 60);

impl Session {
    pub async fn idle(
        mut self,
        context: &Context,
        idle_interrupt_receiver: Receiver<()>,
        folder: &str,
    ) -> Result<Self> {
        use futures::future::FutureExt;

        let expected_uid_next = get_uid_next(context, folder)
            .await
            .with_context(|| format!("failed to get old UID NEXT for folder {folder}"))?;
        if expected_uid_next == 0 {
            info!(
                context,
                "UIDNEXT is not set, skipping IDLE to update UIDVALIDITY/UIDNEXT and fetch"
            );
            return Ok(self);
        }

        self.select_folder(context, Some(folder)).await?;

        if self.server_sent_unsolicited_exists(context)? {
            return Ok(self);
        }

        // Despite checking for unsolicited EXISTS above,
        // we may have missed EXISTS if the message was
        // received when the folder was not selected.
        let status = self
            .status(folder, "(UIDNEXT)")
            .await
            .context("STATUS (UIDNEXT) error for {folder:?}")?;
        if let Some(uid_next) = status.uid_next {
            if uid_next > expected_uid_next {
                info!(
                    context,
                    "Skipping IDLE on {folder:?} because UIDNEXT {uid_next}>{expected_uid_next} indicates there are new messages."
                );
                return Ok(self);
            }
        } else {
            warn!(context, "STATUS {folder} (UIDNEXT) did not return UIDNEXT");
            // Go to IDLE anyway if STATUS is broken.
        }

        if let Ok(()) = idle_interrupt_receiver.try_recv() {
            info!(context, "skip idle, got interrupt");
            return Ok(self);
        }

        let mut handle = self.inner.idle();
        if let Err(err) = handle.init().await {
            bail!("IMAP IDLE protocol failed to init/complete: {}", err);
        }

        // At this point IDLE command was sent and we received a "+ idling" response. We will now
        // read from the stream without getting any data for up to `IDLE_TIMEOUT`. If we don't
        // disable read timeout, we would get a timeout after `IMAP_TIMEOUT`, which is a lot
        // shorter than `IDLE_TIMEOUT`.
        handle.as_mut().set_read_timeout(None);
        let (idle_wait, interrupt) = handle.wait_with_timeout(IDLE_TIMEOUT);

        enum Event {
            IdleResponse(IdleResponse),
            Interrupt,
        }

        info!(context, "{folder}: Idle entering wait-on-remote state");
        let fut = idle_wait.map(|ev| ev.map(Event::IdleResponse)).race(async {
            idle_interrupt_receiver.recv().await.ok();

            // cancel imap idle connection properly
            drop(interrupt);

            Ok(Event::Interrupt)
        });

        match fut.await {
            Ok(Event::IdleResponse(IdleResponse::NewData(x))) => {
                info!(context, "{folder}: Idle has NewData {:?}", x);
            }
            Ok(Event::IdleResponse(IdleResponse::Timeout)) => {
                info!(context, "{folder}: Idle-wait timeout or interruption");
            }
            Ok(Event::IdleResponse(IdleResponse::ManualInterrupt)) => {
                info!(context, "{folder}: Idle wait was interrupted manually");
            }
            Ok(Event::Interrupt) => {
                info!(context, "{folder}: Idle wait was interrupted");
            }
            Err(err) => {
                warn!(context, "{folder}: Idle wait errored: {err:?}");
            }
        }

        let mut session = tokio::time::timeout(Duration::from_secs(15), handle.done())
            .await
            .with_context(|| format!("{folder}: IMAP IDLE protocol timed out"))?
            .with_context(|| format!("{folder}: IMAP IDLE failed"))?;
        session.as_mut().set_read_timeout(Some(IMAP_TIMEOUT));
        self.inner = session;

        Ok(self)
    }
}

impl Imap {
    pub(crate) async fn fake_idle(
        &mut self,
        context: &Context,
        watch_folder: Option<String>,
        folder_meaning: FolderMeaning,
    ) {
        // Idle using polling. This is also needed if we're not yet configured -
        // in this case, we're waiting for a configure job (and an interrupt).

        let fake_idle_start_time = SystemTime::now();

        // Do not poll, just wait for an interrupt when no folder is passed in.
        let watch_folder = if let Some(watch_folder) = watch_folder {
            watch_folder
        } else {
            info!(context, "IMAP-fake-IDLE: no folder, waiting for interrupt");
            self.idle_interrupt_receiver.recv().await.ok();
            return;
        };
        info!(context, "IMAP-fake-IDLEing folder={:?}", watch_folder);

        // check every minute if there are new messages
        // TODO: grow sleep durations / make them more flexible
        let mut interval = tokio::time::interval(Duration::from_secs(60));

        enum Event {
            Tick,
            Interrupt,
        }
        // loop until we are interrupted or if we fetched something
        loop {
            use futures::future::FutureExt;
            match interval
                .tick()
                .map(|_| Event::Tick)
                .race(
                    self.idle_interrupt_receiver
                        .recv()
                        .map(|_| Event::Interrupt),
                )
                .await
            {
                Event::Tick => {
                    // try to connect with proper login params
                    // (setup_handle_if_needed might not know about them if we
                    // never successfully connected)
                    if let Err(err) = self.prepare(context).await {
                        warn!(context, "fake_idle: could not connect: {}", err);
                        continue;
                    }
                    if let Some(session) = &self.session {
                        if session.can_idle()
                            && !context
                                .get_config_bool(Config::DisableIdle)
                                .await
                                .context("Failed to get disable_idle config")
                                .log_err(context)
                                .unwrap_or_default()
                        {
                            // we only fake-idled because network was gone during IDLE, probably
                            break;
                        }
                    }
                    info!(context, "fake_idle is connected");
                    // we are connected, let's see if fetching messages results
                    // in anything.  If so, we behave as if IDLE had data but
                    // will have already fetched the messages so perform_*_fetch
                    // will not find any new.
                    match self
                        .fetch_new_messages(context, &watch_folder, folder_meaning, false)
                        .await
                    {
                        Ok(res) => {
                            info!(context, "fetch_new_messages returned {:?}", res);
                            if res {
                                break;
                            }
                        }
                        Err(err) => {
                            error!(context, "could not fetch from folder: {:#}", err);
                            self.trigger_reconnect(context);
                        }
                    }
                }
                Event::Interrupt => {
                    info!(context, "Fake IDLE interrupted");
                    break;
                }
            }
        }

        info!(
            context,
            "IMAP-fake-IDLE done after {:.4}s",
            SystemTime::now()
                .duration_since(fake_idle_start_time)
                .unwrap_or_default()
                .as_millis() as f64
                / 1000.,
        );
    }
}
