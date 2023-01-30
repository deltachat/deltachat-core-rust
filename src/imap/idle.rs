use std::time::{Duration, SystemTime};

use anyhow::{bail, Context as _, Result};
use async_channel::Receiver;
use async_imap::extensions::idle::IdleResponse;
use futures_lite::FutureExt;

use super::session::Session;
use super::Imap;
use crate::imap::client::IMAP_TIMEOUT;
use crate::{context::Context, scheduler::InterruptInfo};

const IDLE_TIMEOUT: Duration = Duration::from_secs(23 * 60);

impl Session {
    pub async fn idle(
        mut self,
        context: &Context,
        idle_interrupt_receiver: Receiver<InterruptInfo>,
        watch_folder: Option<String>,
    ) -> Result<(Self, InterruptInfo)> {
        use futures::future::FutureExt;

        if !self.can_idle() {
            bail!("IMAP server does not have IDLE capability");
        }

        let mut info = Default::default();

        self.select_folder(context, watch_folder.as_deref()).await?;

        if self.server_sent_unsolicited_exists(context)? {
            return Ok((self, info));
        }

        if let Ok(info) = idle_interrupt_receiver.try_recv() {
            info!(context, "skip idle, got interrupt {:?}", info);
            return Ok((self, info));
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
            Interrupt(InterruptInfo),
        }

        let folder_name = watch_folder.as_deref().unwrap_or("None");
        info!(
            context,
            "{}: Idle entering wait-on-remote state", folder_name
        );
        let fut = idle_wait.map(|ev| ev.map(Event::IdleResponse)).race(async {
            let info = idle_interrupt_receiver.recv().await;

            // cancel imap idle connection properly
            drop(interrupt);

            Ok(Event::Interrupt(info.unwrap_or_default()))
        });

        match fut.await {
            Ok(Event::IdleResponse(IdleResponse::NewData(x))) => {
                info!(context, "{}: Idle has NewData {:?}", folder_name, x);
            }
            Ok(Event::IdleResponse(IdleResponse::Timeout)) => {
                info!(
                    context,
                    "{}: Idle-wait timeout or interruption", folder_name
                );
            }
            Ok(Event::IdleResponse(IdleResponse::ManualInterrupt)) => {
                info!(
                    context,
                    "{}: Idle wait was interrupted manually", folder_name
                );
            }
            Ok(Event::Interrupt(i)) => {
                info!(
                    context,
                    "{}: Idle wait was interrupted: {:?}", folder_name, &i
                );
                info = i;
            }
            Err(err) => {
                warn!(context, "{}: Idle wait errored: {:?}", folder_name, err);
            }
        }

        let mut session = tokio::time::timeout(Duration::from_secs(15), handle.done())
            .await
            .with_context(|| format!("{folder_name}: IMAP IDLE protocol timed out"))?
            .with_context(|| format!("{folder_name}: IMAP IDLE failed"))?;
        session.as_mut().set_read_timeout(Some(IMAP_TIMEOUT));
        self.inner = session;

        Ok((self, info))
    }
}

impl Imap {
    pub(crate) async fn fake_idle(
        &mut self,
        context: &Context,
        watch_folder: Option<String>,
    ) -> InterruptInfo {
        // Idle using polling. This is also needed if we're not yet configured -
        // in this case, we're waiting for a configure job (and an interrupt).

        let fake_idle_start_time = SystemTime::now();

        // Do not poll, just wait for an interrupt when no folder is passed in.
        let watch_folder = if let Some(watch_folder) = watch_folder {
            watch_folder
        } else {
            info!(context, "IMAP-fake-IDLE: no folder, waiting for interrupt");
            return self
                .idle_interrupt_receiver
                .recv()
                .await
                .unwrap_or_default();
        };
        info!(context, "IMAP-fake-IDLEing folder={:?}", watch_folder);

        // check every minute if there are new messages
        // TODO: grow sleep durations / make them more flexible
        let mut interval = tokio::time::interval(Duration::from_secs(60));

        enum Event {
            Tick,
            Interrupt(InterruptInfo),
        }
        // loop until we are interrupted or if we fetched something
        let info = loop {
            use futures::future::FutureExt;
            match interval
                .tick()
                .map(|_| Event::Tick)
                .race(
                    self.idle_interrupt_receiver
                        .recv()
                        .map(|probe_network| Event::Interrupt(probe_network.unwrap_or_default())),
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
                        if session.can_idle() {
                            // we only fake-idled because network was gone during IDLE, probably
                            break InterruptInfo::new(false);
                        }
                    }
                    info!(context, "fake_idle is connected");
                    // we are connected, let's see if fetching messages results
                    // in anything.  If so, we behave as if IDLE had data but
                    // will have already fetched the messages so perform_*_fetch
                    // will not find any new.
                    match self
                        .fetch_new_messages(context, &watch_folder, false, false)
                        .await
                    {
                        Ok(res) => {
                            info!(context, "fetch_new_messages returned {:?}", res);
                            if res {
                                break InterruptInfo::new(false);
                            }
                        }
                        Err(err) => {
                            error!(context, "could not fetch from folder: {:#}", err);
                            self.trigger_reconnect(context);
                        }
                    }
                }
                Event::Interrupt(info) => {
                    // Interrupt
                    info!(context, "Fake IDLE interrupted");
                    break info;
                }
            }
        };

        info!(
            context,
            "IMAP-fake-IDLE done after {:.4}s",
            SystemTime::now()
                .duration_since(fake_idle_start_time)
                .unwrap_or_default()
                .as_millis() as f64
                / 1000.,
        );

        info
    }
}
