use anyhow::{bail, Context as _, Result};
use async_channel::{self as channel, Receiver, Sender};
use futures::{join, try_join};
use futures_lite::FutureExt;
use tokio::task;

use crate::config::Config;
use crate::context::Context;
use crate::ephemeral::{self, delete_expired_imap_messages};
use crate::imap::Imap;
use crate::job;
use crate::location;
use crate::log::LogExt;
use crate::smtp::{send_smtp_messages, Smtp};
use crate::sql;
use crate::tools::time;
use crate::tools::{duration_to_str, maybe_add_time_based_warnings};

use self::connectivity::ConnectivityStore;

pub(crate) mod connectivity;

/// Job and connection scheduler.
#[derive(Debug)]
pub(crate) struct Scheduler {
    inbox: ImapConnectionState,
    inbox_handle: task::JoinHandle<()>,
    mvbox: ImapConnectionState,
    mvbox_handle: Option<task::JoinHandle<()>>,
    sentbox: ImapConnectionState,
    sentbox_handle: Option<task::JoinHandle<()>>,
    smtp: SmtpConnectionState,
    smtp_handle: task::JoinHandle<()>,
    ephemeral_handle: task::JoinHandle<()>,
    ephemeral_interrupt_send: Sender<()>,
    location_handle: task::JoinHandle<()>,
    location_interrupt_send: Sender<()>,
}

impl Context {
    /// Indicate that the network likely has come back.
    pub async fn maybe_network(&self) {
        let lock = self.scheduler.read().await;
        if let Some(scheduler) = &*lock {
            scheduler.maybe_network().await;
        }
        connectivity::idle_interrupted(lock).await;
    }

    /// Indicate that the network likely is lost.
    pub async fn maybe_network_lost(&self) {
        let lock = self.scheduler.read().await;
        if let Some(scheduler) = &*lock {
            scheduler.maybe_network_lost().await;
        }
        connectivity::maybe_network_lost(self, lock).await;
    }

    pub(crate) async fn interrupt_inbox(&self, info: InterruptInfo) {
        if let Some(scheduler) = &*self.scheduler.read().await {
            scheduler.interrupt_inbox(info).await;
        }
    }

    pub(crate) async fn interrupt_smtp(&self, info: InterruptInfo) {
        if let Some(scheduler) = &*self.scheduler.read().await {
            scheduler.interrupt_smtp(info).await;
        }
    }

    pub(crate) async fn interrupt_ephemeral_task(&self) {
        if let Some(scheduler) = &*self.scheduler.read().await {
            scheduler.interrupt_ephemeral_task().await;
        }
    }

    pub(crate) async fn interrupt_location(&self) {
        if let Some(scheduler) = &*self.scheduler.read().await {
            scheduler.interrupt_location().await;
        }
    }
}

async fn inbox_loop(ctx: Context, started: Sender<()>, inbox_handlers: ImapConnectionHandlers) {
    use futures::future::FutureExt;

    info!(ctx, "starting inbox loop");
    let ImapConnectionHandlers {
        mut connection,
        stop_receiver,
    } = inbox_handlers;

    let ctx1 = ctx.clone();
    let fut = async move {
        let ctx = ctx1;
        if let Err(err) = started.send(()).await {
            warn!(ctx, "inbox loop, missing started receiver: {}", err);
            return;
        };

        let mut info = InterruptInfo::default();
        loop {
            let job = match job::load_next(&ctx, &info).await {
                Err(err) => {
                    error!(ctx, "Failed loading job from the database: {:#}.", err);
                    None
                }
                Ok(job) => job,
            };

            match job {
                Some(job) => {
                    job::perform_job(&ctx, job::Connection::Inbox(&mut connection), job).await;
                    info = Default::default();
                }
                None => {
                    maybe_add_time_based_warnings(&ctx).await;

                    match ctx.get_config_i64(Config::LastHousekeeping).await {
                        Ok(last_housekeeping_time) => {
                            let next_housekeeping_time =
                                last_housekeeping_time.saturating_add(60 * 60 * 24);
                            if next_housekeeping_time <= time() {
                                sql::housekeeping(&ctx).await.ok_or_log(&ctx);
                            }
                        }
                        Err(err) => {
                            warn!(ctx, "Failed to get last housekeeping time: {}", err);
                        }
                    };

                    match ctx.get_config_bool(Config::FetchedExistingMsgs).await {
                        Ok(fetched_existing_msgs) => {
                            if !fetched_existing_msgs {
                                if let Err(err) = connection.fetch_existing_msgs(&ctx).await {
                                    warn!(ctx, "Failed to fetch existing messages: {:#}", err);
                                }
                            }
                        }
                        Err(err) => {
                            warn!(ctx, "Can't get Config::FetchedExistingMsgs: {:#}", err);
                        }
                    }

                    info = fetch_idle(&ctx, &mut connection, Config::ConfiguredInboxFolder).await;
                }
            }
        }
    };

    stop_receiver
        .recv()
        .map(|_| {
            info!(ctx, "shutting down inbox loop");
        })
        .race(fut)
        .await;
}

async fn fetch_idle(ctx: &Context, connection: &mut Imap, folder: Config) -> InterruptInfo {
    match ctx.get_config(folder).await {
        Ok(Some(watch_folder)) => {
            // connect and fake idle if unable to connect
            if let Err(err) = connection.prepare(ctx).await {
                warn!(ctx, "imap connection failed: {}", err);
                return connection.fake_idle(ctx, Some(watch_folder)).await;
            }

            if folder == Config::ConfiguredInboxFolder {
                if let Err(err) = connection
                    .store_seen_flags_on_imap(ctx)
                    .await
                    .context("store_seen_flags_on_imap failed")
                {
                    warn!(ctx, "{:#}", err);
                }
            }

            // Fetch the watched folder.
            if let Err(err) = connection
                .fetch_move_delete(ctx, &watch_folder, false)
                .await
            {
                connection.trigger_reconnect(ctx).await;
                warn!(ctx, "{:#}", err);
                return InterruptInfo::new(false);
            }

            // Mark expired messages for deletion. Marked messages will be deleted from the server
            // on the next iteration of `fetch_move_delete`. `delete_expired_imap_messages` is not
            // called right before `fetch_move_delete` because it is not well optimized and would
            // otherwise slow down message fetching.
            if let Err(err) = delete_expired_imap_messages(ctx)
                .await
                .context("delete_expired_imap_messages failed")
            {
                warn!(ctx, "{:#}", err);
            }

            // Scan additional folders only after finishing fetching the watched folder.
            //
            // On iOS the application has strictly limited time to work in background, so we may not
            // be able to scan all folders before time is up if there are many of them.
            if folder == Config::ConfiguredInboxFolder {
                // Only scan on the Inbox thread in order to prevent parallel scans, which might lead to duplicate messages
                match connection.scan_folders(ctx).await {
                    Err(err) => {
                        // Don't reconnect, if there is a problem with the connection we will realize this when IDLEing
                        // but maybe just one folder can't be selected or something
                        warn!(ctx, "{}", err);
                    }
                    Ok(true) => {
                        // Fetch the watched folder again in case scanning other folder moved messages
                        // there.
                        //
                        // In most cases this will select the watched folder and return because there are
                        // no new messages. We want to select the watched folder anyway before going IDLE
                        // there, so this does not take additional protocol round-trip.
                        if let Err(err) = connection
                            .fetch_move_delete(ctx, &watch_folder, false)
                            .await
                        {
                            connection.trigger_reconnect(ctx).await;
                            warn!(ctx, "{:#}", err);
                            return InterruptInfo::new(false);
                        }
                    }
                    Ok(false) => {}
                }
            }

            // Synchronize Seen flags.
            connection
                .sync_seen_flags(ctx, &watch_folder)
                .await
                .context("sync_seen_flags")
                .ok_or_log(ctx);

            connection.connectivity.set_connected(ctx).await;

            // idle
            if connection.can_idle() {
                match connection.idle(ctx, Some(watch_folder)).await {
                    Ok(v) => v,
                    Err(err) => {
                        connection.trigger_reconnect(ctx).await;
                        warn!(ctx, "{}", err);
                        InterruptInfo::new(false)
                    }
                }
            } else {
                connection.fake_idle(ctx, Some(watch_folder)).await
            }
        }
        Ok(None) => {
            connection.connectivity.set_not_configured(ctx).await;
            info!(ctx, "Can not watch {} folder, not set", folder);
            connection.fake_idle(ctx, None).await
        }
        Err(err) => {
            warn!(
                ctx,
                "Can not watch {} folder, failed to retrieve config: {:?}", folder, err
            );
            connection.fake_idle(ctx, None).await
        }
    }
}

async fn simple_imap_loop(
    ctx: Context,
    started: Sender<()>,
    inbox_handlers: ImapConnectionHandlers,
    folder: Config,
) {
    use futures::future::FutureExt;

    info!(ctx, "starting simple loop for {}", folder.as_ref());
    let ImapConnectionHandlers {
        mut connection,
        stop_receiver,
    } = inbox_handlers;

    let ctx1 = ctx.clone();

    let fut = async move {
        let ctx = ctx1;
        if let Err(err) = started.send(()).await {
            warn!(&ctx, "simple imap loop, missing started receiver: {}", err);
            return;
        }

        loop {
            fetch_idle(&ctx, &mut connection, folder).await;
        }
    };

    stop_receiver
        .recv()
        .map(|_| {
            info!(ctx, "shutting down simple loop");
        })
        .race(fut)
        .await;
}

async fn smtp_loop(ctx: Context, started: Sender<()>, smtp_handlers: SmtpConnectionHandlers) {
    use futures::future::FutureExt;

    info!(ctx, "starting smtp loop");
    let SmtpConnectionHandlers {
        mut connection,
        stop_receiver,
        idle_interrupt_receiver,
    } = smtp_handlers;

    let ctx1 = ctx.clone();
    let fut = async move {
        let ctx = ctx1;
        if let Err(err) = started.send(()).await {
            warn!(&ctx, "smtp loop, missing started receiver: {}", err);
            return;
        }

        let mut timeout = None;
        loop {
            if let Err(err) = send_smtp_messages(&ctx, &mut connection).await {
                warn!(ctx, "send_smtp_messages failed: {:#}", err);
                timeout = Some(timeout.map_or(30, |timeout: u64| timeout.saturating_mul(3)))
            } else {
                let duration_until_can_send = ctx.ratelimit.read().await.until_can_send();
                if !duration_until_can_send.is_zero() {
                    info!(
                        ctx,
                        "smtp got rate limited, delaying next try by {}",
                        duration_to_str(duration_until_can_send)
                    );
                    tokio::time::timeout(duration_until_can_send, async {
                        idle_interrupt_receiver.recv().await.unwrap_or_default()
                    })
                    .await
                    .unwrap_or_default();
                    continue;
                }
                timeout = None;
            }

            // Fake Idle
            info!(ctx, "smtp fake idle - started");
            match &connection.last_send_error {
                None => connection.connectivity.set_connected(&ctx).await,
                Some(err) => connection.connectivity.set_err(&ctx, err).await,
            }

            // If send_smtp_messages() failed, we set a timeout for the fake-idle so that
            // sending is retried (at the latest) after the timeout. If sending fails
            // again, we increase the timeout exponentially, in order not to do lots of
            // unnecessary retries.
            if let Some(timeout) = timeout {
                info!(
                    ctx,
                    "smtp has messages to retry, planning to retry {} seconds later", timeout
                );
                let duration = std::time::Duration::from_secs(timeout);
                tokio::time::timeout(duration, async {
                    idle_interrupt_receiver.recv().await.unwrap_or_default()
                })
                .await
                .unwrap_or_default();
            } else {
                info!(ctx, "smtp has no messages to retry, waiting for interrupt");
                idle_interrupt_receiver.recv().await.unwrap_or_default();
            };

            info!(ctx, "smtp fake idle - interrupted")
        }
    };

    stop_receiver
        .recv()
        .map(|_| {
            info!(ctx, "shutting down smtp loop");
        })
        .race(fut)
        .await;
}

impl Scheduler {
    /// Start the scheduler.
    pub async fn start(ctx: Context) -> Result<Self> {
        let (mvbox, mvbox_handlers) = ImapConnectionState::new(&ctx).await?;
        let (sentbox, sentbox_handlers) = ImapConnectionState::new(&ctx).await?;
        let (smtp, smtp_handlers) = SmtpConnectionState::new();
        let (inbox, inbox_handlers) = ImapConnectionState::new(&ctx).await?;

        let (inbox_start_send, inbox_start_recv) = channel::bounded(1);
        let (mvbox_start_send, mvbox_start_recv) = channel::bounded(1);
        let mut mvbox_handle = None;
        let (sentbox_start_send, sentbox_start_recv) = channel::bounded(1);
        let mut sentbox_handle = None;
        let (smtp_start_send, smtp_start_recv) = channel::bounded(1);
        let (ephemeral_interrupt_send, ephemeral_interrupt_recv) = channel::bounded(1);
        let (location_interrupt_send, location_interrupt_recv) = channel::bounded(1);

        let inbox_handle = {
            let ctx = ctx.clone();
            task::spawn(async move { inbox_loop(ctx, inbox_start_send, inbox_handlers).await })
        };

        if ctx.should_watch_mvbox().await? {
            let ctx = ctx.clone();
            mvbox_handle = Some(task::spawn(async move {
                simple_imap_loop(
                    ctx,
                    mvbox_start_send,
                    mvbox_handlers,
                    Config::ConfiguredMvboxFolder,
                )
                .await
            }));
        } else {
            mvbox_start_send
                .send(())
                .await
                .context("mvbox start send, missing receiver")?;
            mvbox_handlers
                .connection
                .connectivity
                .set_not_configured(&ctx)
                .await
        }

        if ctx.get_config_bool(Config::SentboxWatch).await? {
            let ctx = ctx.clone();
            sentbox_handle = Some(task::spawn(async move {
                simple_imap_loop(
                    ctx,
                    sentbox_start_send,
                    sentbox_handlers,
                    Config::ConfiguredSentboxFolder,
                )
                .await
            }));
        } else {
            sentbox_start_send
                .send(())
                .await
                .context("sentbox start send, missing receiver")?;
            sentbox_handlers
                .connection
                .connectivity
                .set_not_configured(&ctx)
                .await
        }

        let smtp_handle = {
            let ctx = ctx.clone();
            task::spawn(async move { smtp_loop(ctx, smtp_start_send, smtp_handlers).await })
        };

        let ephemeral_handle = {
            let ctx = ctx.clone();
            task::spawn(async move {
                ephemeral::ephemeral_loop(&ctx, ephemeral_interrupt_recv).await;
            })
        };

        let location_handle = {
            let ctx = ctx.clone();
            task::spawn(async move {
                location::location_loop(&ctx, location_interrupt_recv).await;
            })
        };

        let res = Self {
            inbox,
            mvbox,
            sentbox,
            smtp,
            inbox_handle,
            mvbox_handle,
            sentbox_handle,
            smtp_handle,
            ephemeral_handle,
            ephemeral_interrupt_send,
            location_handle,
            location_interrupt_send,
        };

        // wait for all loops to be started
        if let Err(err) = try_join!(
            inbox_start_recv.recv(),
            mvbox_start_recv.recv(),
            sentbox_start_recv.recv(),
            smtp_start_recv.recv()
        ) {
            bail!("failed to start scheduler: {}", err);
        }

        info!(ctx, "scheduler is running");
        Ok(res)
    }

    async fn maybe_network(&self) {
        join!(
            self.interrupt_inbox(InterruptInfo::new(true)),
            self.interrupt_mvbox(InterruptInfo::new(true)),
            self.interrupt_sentbox(InterruptInfo::new(true)),
            self.interrupt_smtp(InterruptInfo::new(true))
        );
    }

    async fn maybe_network_lost(&self) {
        join!(
            self.interrupt_inbox(InterruptInfo::new(false)),
            self.interrupt_mvbox(InterruptInfo::new(false)),
            self.interrupt_sentbox(InterruptInfo::new(false)),
            self.interrupt_smtp(InterruptInfo::new(false))
        );
    }

    async fn interrupt_inbox(&self, info: InterruptInfo) {
        self.inbox.interrupt(info).await;
    }

    async fn interrupt_mvbox(&self, info: InterruptInfo) {
        self.mvbox.interrupt(info).await;
    }

    async fn interrupt_sentbox(&self, info: InterruptInfo) {
        self.sentbox.interrupt(info).await;
    }

    async fn interrupt_smtp(&self, info: InterruptInfo) {
        self.smtp.interrupt(info).await;
    }

    async fn interrupt_ephemeral_task(&self) {
        self.ephemeral_interrupt_send.try_send(()).ok();
    }

    async fn interrupt_location(&self) {
        self.location_interrupt_send.try_send(()).ok();
    }

    /// Halt the scheduler.
    ///
    /// It consumes the scheduler and never fails to stop it. In the worst case, long-running tasks
    /// are forcefully terminated if they cannot shutdown within the timeout.
    pub(crate) async fn stop(mut self, context: &Context) {
        // Send stop signals to tasks so they can shutdown cleanly.
        self.inbox.stop().await.ok_or_log(context);
        if self.mvbox_handle.is_some() {
            self.mvbox.stop().await.ok_or_log(context);
        }
        if self.sentbox_handle.is_some() {
            self.sentbox.stop().await.ok_or_log(context);
        }
        self.smtp.stop().await.ok_or_log(context);

        // Actually shutdown tasks.
        let timeout_duration = std::time::Duration::from_secs(30);
        tokio::time::timeout(timeout_duration, self.inbox_handle)
            .await
            .ok_or_log(context);
        if let Some(mvbox_handle) = self.mvbox_handle.take() {
            tokio::time::timeout(timeout_duration, mvbox_handle)
                .await
                .ok_or_log(context);
        }
        if let Some(sentbox_handle) = self.sentbox_handle.take() {
            tokio::time::timeout(timeout_duration, sentbox_handle)
                .await
                .ok_or_log(context);
        }
        tokio::time::timeout(timeout_duration, self.smtp_handle)
            .await
            .ok_or_log(context);
        self.ephemeral_handle.abort();
        self.location_handle.abort();
    }
}

/// Connection state logic shared between imap and smtp connections.
#[derive(Debug)]
struct ConnectionState {
    /// Channel to interrupt the whole connection.
    stop_sender: Sender<()>,
    /// Channel to interrupt idle.
    idle_interrupt_sender: Sender<InterruptInfo>,
    /// Mutex to pass connectivity info between IMAP/SMTP threads and the API
    connectivity: ConnectivityStore,
}

impl ConnectionState {
    /// Shutdown this connection completely.
    async fn stop(&self) -> Result<()> {
        // Trigger shutdown of the run loop.
        self.stop_sender
            .send(())
            .await
            .context("failed to stop, missing receiver")?;
        Ok(())
    }

    async fn interrupt(&self, info: InterruptInfo) {
        // Use try_send to avoid blocking on interrupts.
        self.idle_interrupt_sender.try_send(info).ok();
    }
}

#[derive(Debug)]
pub(crate) struct SmtpConnectionState {
    state: ConnectionState,
}

impl SmtpConnectionState {
    fn new() -> (Self, SmtpConnectionHandlers) {
        let (stop_sender, stop_receiver) = channel::bounded(1);
        let (idle_interrupt_sender, idle_interrupt_receiver) = channel::bounded(1);

        let handlers = SmtpConnectionHandlers {
            connection: Smtp::new(),
            stop_receiver,
            idle_interrupt_receiver,
        };

        let state = ConnectionState {
            stop_sender,
            idle_interrupt_sender,
            connectivity: handlers.connection.connectivity.clone(),
        };

        let conn = SmtpConnectionState { state };

        (conn, handlers)
    }

    /// Interrupt any form of idle.
    async fn interrupt(&self, info: InterruptInfo) {
        self.state.interrupt(info).await;
    }

    /// Shutdown this connection completely.
    async fn stop(&self) -> Result<()> {
        self.state.stop().await?;
        Ok(())
    }
}

struct SmtpConnectionHandlers {
    connection: Smtp,
    stop_receiver: Receiver<()>,
    idle_interrupt_receiver: Receiver<InterruptInfo>,
}

#[derive(Debug)]
pub(crate) struct ImapConnectionState {
    state: ConnectionState,
}

impl ImapConnectionState {
    /// Construct a new connection.
    async fn new(context: &Context) -> Result<(Self, ImapConnectionHandlers)> {
        let (stop_sender, stop_receiver) = channel::bounded(1);
        let (idle_interrupt_sender, idle_interrupt_receiver) = channel::bounded(1);

        let handlers = ImapConnectionHandlers {
            connection: Imap::new_configured(context, idle_interrupt_receiver).await?,
            stop_receiver,
        };

        let state = ConnectionState {
            stop_sender,
            idle_interrupt_sender,
            connectivity: handlers.connection.connectivity.clone(),
        };

        let conn = ImapConnectionState { state };

        Ok((conn, handlers))
    }

    /// Interrupt any form of idle.
    async fn interrupt(&self, info: InterruptInfo) {
        self.state.interrupt(info).await;
    }

    /// Shutdown this connection completely.
    async fn stop(&self) -> Result<()> {
        self.state.stop().await?;
        Ok(())
    }
}

#[derive(Debug)]
struct ImapConnectionHandlers {
    connection: Imap,
    stop_receiver: Receiver<()>,
}

#[derive(Default, Debug)]
pub struct InterruptInfo {
    pub probe_network: bool,
}

impl InterruptInfo {
    pub fn new(probe_network: bool) -> Self {
        Self { probe_network }
    }
}
