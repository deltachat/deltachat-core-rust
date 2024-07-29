use std::time::Duration;

use anyhow::{bail, Context as _, Result};
use async_channel::Receiver;
use async_imap::extensions::idle::IdleResponse;
use futures_lite::FutureExt;
use tokio::time::timeout;

use super::session::Session;
use super::Imap;
use crate::context::Context;
use crate::imap::FolderMeaning;
use crate::net::TIMEOUT;
use crate::tools::{self, time_elapsed};

/// Timeout after which IDLE is finished
/// if there are no responses from the server.
///
/// If `* OK Still here` keepalives are sent more frequently
/// than this duration, timeout should never be triggered.
/// For example, Dovecot sends keepalives every 2 minutes by default.
const IDLE_TIMEOUT: Duration = Duration::from_secs(5 * 60);

impl Session {
    pub async fn idle(
        mut self,
        context: &Context,
        idle_interrupt_receiver: Receiver<()>,
        folder: &str,
    ) -> Result<Self> {
        use futures::future::FutureExt;

        self.select_with_uidvalidity(context, folder).await?;

        if self.server_sent_unsolicited_exists(context)? {
            self.new_mail = true;
        }

        if self.new_mail {
            return Ok(self);
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
        // disable read timeout, we would get a timeout after `crate::net::TIMEOUT`, which is a lot
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
        session.as_mut().set_read_timeout(Some(TIMEOUT));
        self.inner = session;

        // Fetch mail once we exit IDLE.
        self.new_mail = true;

        Ok(self)
    }
}

impl Imap {
    /// Idle using polling.
    pub(crate) async fn fake_idle(
        &mut self,
        context: &Context,
        session: &mut Session,
        watch_folder: String,
        folder_meaning: FolderMeaning,
    ) -> Result<()> {
        let fake_idle_start_time = tools::Time::now();

        info!(context, "IMAP-fake-IDLEing folder={:?}", watch_folder);

        // Loop until we are interrupted or until we fetch something.
        loop {
            match timeout(Duration::from_secs(60), self.idle_interrupt_receiver.recv()).await {
                Err(_) => {
                    // Let's see if fetching messages results
                    // in anything.  If so, we behave as if IDLE had data but
                    // will have already fetched the messages so perform_*_fetch
                    // will not find any new.
                    let res = self
                        .fetch_new_messages(context, session, &watch_folder, folder_meaning, false)
                        .await?;

                    info!(context, "fetch_new_messages returned {:?}", res);

                    if res {
                        break;
                    }
                }
                Ok(_) => {
                    info!(context, "Fake IDLE interrupted.");
                    break;
                }
            }
        }

        info!(
            context,
            "IMAP-fake-IDLE done after {:.4}s",
            time_elapsed(&fake_idle_start_time).as_millis() as f64 / 1000.,
        );
        Ok(())
    }
}
