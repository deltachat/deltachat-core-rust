use std::iter::{self, once};
use std::num::NonZeroUsize;
use std::sync::atomic::Ordering;

use anyhow::{bail, Context as _, Error, Result};
use async_channel::{self as channel, Receiver, Sender};
use futures::future::try_join_all;
use futures_lite::FutureExt;
use tokio::sync::{oneshot, RwLock, RwLockWriteGuard};
use tokio::task;

use self::connectivity::ConnectivityStore;
use crate::config::Config;
use crate::contact::{ContactId, RecentlySeenLoop};
use crate::context::Context;
use crate::ephemeral::{self, delete_expired_imap_messages};
use crate::events::EventType;
use crate::imap::{FolderMeaning, Imap};
use crate::job;
use crate::location;
use crate::log::LogExt;
use crate::smtp::{send_smtp_messages, Smtp};
use crate::sql;
use crate::tools::time;
use crate::tools::{duration_to_str, maybe_add_time_based_warnings};

pub(crate) mod connectivity;

/// State of the IO scheduler, as stored on the [`Context`].
///
/// The IO scheduler can be stopped or started, but core can also pause it.  After pausing
/// the IO scheduler will be restarted only if it was running before paused or
/// [`Context::start_io`] was called in the meantime while it was paused.
#[derive(Debug, Default)]
pub(crate) struct SchedulerState {
    inner: RwLock<InnerSchedulerState>,
}

impl SchedulerState {
    pub(crate) fn new() -> Self {
        Default::default()
    }

    /// Whether the scheduler is currently running.
    pub(crate) async fn is_running(&self) -> bool {
        let inner = self.inner.read().await;
        matches!(*inner, InnerSchedulerState::Started(_))
    }

    /// Starts the scheduler if it is not yet started.
    pub(crate) async fn start(&self, context: Context) {
        let mut inner = self.inner.write().await;
        match *inner {
            InnerSchedulerState::Started(_) => (),
            InnerSchedulerState::Stopped => Self::do_start(inner, context).await,
            InnerSchedulerState::Paused {
                ref mut started, ..
            } => *started = true,
        }
    }

    /// Starts the scheduler if it is not yet started.
    async fn do_start(mut inner: RwLockWriteGuard<'_, InnerSchedulerState>, context: Context) {
        info!(context, "starting IO");
        let ctx = context.clone();
        match Scheduler::start(context).await {
            Ok(scheduler) => *inner = InnerSchedulerState::Started(scheduler),
            Err(err) => error!(&ctx, "Failed to start IO: {:#}", err),
        }
    }

    /// Stops the scheduler if it is currently running.
    pub(crate) async fn stop(&self, context: &Context) {
        let mut inner = self.inner.write().await;
        match *inner {
            InnerSchedulerState::Started(_) => {
                Self::do_stop(inner, context, InnerSchedulerState::Stopped).await
            }
            InnerSchedulerState::Stopped => (),
            InnerSchedulerState::Paused {
                ref mut started, ..
            } => *started = false,
        }
    }

    /// Stops the scheduler if it is currently running.
    async fn do_stop(
        mut inner: RwLockWriteGuard<'_, InnerSchedulerState>,
        context: &Context,
        new_state: InnerSchedulerState,
    ) {
        // Sending an event wakes up event pollers (get_next_event)
        // so the caller of stop_io() can arrange for proper termination.
        // For this, the caller needs to instruct the event poller
        // to terminate on receiving the next event and then call stop_io()
        // which will emit the below event(s)
        info!(context, "stopping IO");
        if let Some(debug_logging) = context.debug_logging.read().await.as_ref() {
            debug_logging.loop_handle.abort();
        }
        let prev_state = std::mem::replace(&mut *inner, new_state);
        match prev_state {
            InnerSchedulerState::Started(scheduler) => scheduler.stop(context).await,
            InnerSchedulerState::Stopped | InnerSchedulerState::Paused { .. } => (),
        }
    }

    /// Pauses the IO scheduler.
    ///
    /// If it is currently running the scheduler will be stopped.  When the
    /// [`IoPausedGuard`] is dropped the scheduler is started again.
    ///
    /// If in the meantime [`SchedulerState::start`] or [`SchedulerState::stop`] is called
    /// resume will do the right thing and restore the scheduler to the state requested by
    /// the last call.
    pub(crate) async fn pause<'a>(&'_ self, context: Context) -> Result<IoPausedGuard> {
        {
            let mut inner = self.inner.write().await;
            match *inner {
                InnerSchedulerState::Started(_) => {
                    let new_state = InnerSchedulerState::Paused {
                        started: true,
                        pause_guards_count: NonZeroUsize::new(1).unwrap(),
                    };
                    Self::do_stop(inner, &context, new_state).await;
                }
                InnerSchedulerState::Stopped => {
                    *inner = InnerSchedulerState::Paused {
                        started: false,
                        pause_guards_count: NonZeroUsize::new(1).unwrap(),
                    };
                }
                InnerSchedulerState::Paused {
                    ref mut pause_guards_count,
                    ..
                } => {
                    *pause_guards_count = pause_guards_count
                        .checked_add(1)
                        .ok_or_else(|| Error::msg("Too many pause guards active"))?
                }
            }
        }

        let (tx, rx) = oneshot::channel();
        tokio::spawn(async move {
            rx.await.ok();
            let mut inner = context.scheduler.inner.write().await;
            match *inner {
                InnerSchedulerState::Started(_) => {
                    warn!(&context, "IoPausedGuard resume: started instead of paused");
                }
                InnerSchedulerState::Stopped => {
                    warn!(&context, "IoPausedGuard resume: stopped instead of paused");
                }
                InnerSchedulerState::Paused {
                    ref started,
                    ref mut pause_guards_count,
                } => {
                    if *pause_guards_count == NonZeroUsize::new(1).unwrap() {
                        match *started {
                            true => SchedulerState::do_start(inner, context.clone()).await,
                            false => *inner = InnerSchedulerState::Stopped,
                        }
                    } else {
                        let new_count = pause_guards_count.get() - 1;
                        // SAFETY: Value was >=2 before due to if condition
                        *pause_guards_count = NonZeroUsize::new(new_count).unwrap();
                    }
                }
            }
        });
        Ok(IoPausedGuard { sender: Some(tx) })
    }

    /// Restarts the scheduler, only if it is running.
    pub(crate) async fn restart(&self, context: &Context) {
        info!(context, "restarting IO");
        if self.is_running().await {
            self.stop(context).await;
            self.start(context.clone()).await;
        }
    }

    /// Indicate that the network likely has come back.
    pub(crate) async fn maybe_network(&self) {
        let inner = self.inner.read().await;
        let (inbox, oboxes) = match *inner {
            InnerSchedulerState::Started(ref scheduler) => {
                scheduler.maybe_network();
                let inbox = scheduler.inbox.conn_state.state.connectivity.clone();
                let oboxes = scheduler
                    .oboxes
                    .iter()
                    .map(|b| b.conn_state.state.connectivity.clone())
                    .collect::<Vec<_>>();
                (inbox, oboxes)
            }
            _ => return,
        };
        drop(inner);
        connectivity::idle_interrupted(inbox, oboxes).await;
    }

    /// Indicate that the network likely is lost.
    pub(crate) async fn maybe_network_lost(&self, context: &Context) {
        let inner = self.inner.read().await;
        let stores = match *inner {
            InnerSchedulerState::Started(ref scheduler) => {
                scheduler.maybe_network_lost();
                scheduler
                    .boxes()
                    .map(|b| b.conn_state.state.connectivity.clone())
                    .collect()
            }
            _ => return,
        };
        drop(inner);
        connectivity::maybe_network_lost(context, stores).await;
    }

    pub(crate) async fn interrupt_inbox(&self, info: InterruptInfo) {
        let inner = self.inner.read().await;
        if let InnerSchedulerState::Started(ref scheduler) = *inner {
            scheduler.interrupt_inbox(info);
        }
    }

    pub(crate) async fn interrupt_smtp(&self, info: InterruptInfo) {
        let inner = self.inner.read().await;
        if let InnerSchedulerState::Started(ref scheduler) = *inner {
            scheduler.interrupt_smtp(info);
        }
    }

    pub(crate) async fn interrupt_ephemeral_task(&self) {
        let inner = self.inner.read().await;
        if let InnerSchedulerState::Started(ref scheduler) = *inner {
            scheduler.interrupt_ephemeral_task();
        }
    }

    pub(crate) async fn interrupt_location(&self) {
        let inner = self.inner.read().await;
        if let InnerSchedulerState::Started(ref scheduler) = *inner {
            scheduler.interrupt_location();
        }
    }

    pub(crate) async fn interrupt_recently_seen(&self, contact_id: ContactId, timestamp: i64) {
        let inner = self.inner.read().await;
        if let InnerSchedulerState::Started(ref scheduler) = *inner {
            scheduler.interrupt_recently_seen(contact_id, timestamp);
        }
    }
}

#[derive(Debug, Default)]
enum InnerSchedulerState {
    Started(Scheduler),
    #[default]
    Stopped,
    Paused {
        started: bool,
        pause_guards_count: NonZeroUsize,
    },
}

/// Guard to make sure the IO Scheduler is resumed.
///
/// Returned by [`SchedulerState::pause`].  To resume the IO scheduler simply drop this
/// guard.
#[derive(Debug)]
pub(crate) struct IoPausedGuard {
    sender: Option<oneshot::Sender<()>>,
}

impl Drop for IoPausedGuard {
    fn drop(&mut self) {
        if let Some(sender) = self.sender.take() {
            // Can only fail if receiver is dropped, but then we're already resumed.
            sender.send(()).ok();
        }
    }
}

#[derive(Debug)]
struct SchedBox {
    meaning: FolderMeaning,
    conn_state: ImapConnectionState,

    /// IMAP loop task handle.
    handle: task::JoinHandle<()>,
}

/// Job and connection scheduler.
#[derive(Debug)]
pub(crate) struct Scheduler {
    inbox: SchedBox,
    /// Optional boxes -- mvbox, sentbox.
    oboxes: Vec<SchedBox>,
    smtp: SmtpConnectionState,
    smtp_handle: task::JoinHandle<()>,
    ephemeral_handle: task::JoinHandle<()>,
    ephemeral_interrupt_send: Sender<()>,
    location_handle: task::JoinHandle<()>,
    location_interrupt_send: Sender<()>,

    recently_seen_loop: RecentlySeenLoop,
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
                    let quota_requested = ctx.quota_update_request.swap(false, Ordering::Relaxed);
                    if quota_requested {
                        if let Err(err) = ctx.update_recent_quota(&mut connection).await {
                            warn!(ctx, "Failed to update quota: {:#}.", err);
                        }
                    }

                    let resync_requested = ctx.resync_request.swap(false, Ordering::Relaxed);
                    if resync_requested {
                        if let Err(err) = connection.resync_folders(&ctx).await {
                            warn!(ctx, "Failed to resync folders: {:#}.", err);
                            ctx.resync_request.store(true, Ordering::Relaxed);
                        }
                    }

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
                                // Consider it done even if we fail.
                                //
                                // This operation is not critical enough to retry,
                                // especially if the error is persistent.
                                if let Err(err) =
                                    ctx.set_config_bool(Config::FetchedExistingMsgs, true).await
                                {
                                    warn!(ctx, "Can't set Config::FetchedExistingMsgs: {:#}", err);
                                }

                                if let Err(err) = connection.fetch_existing_msgs(&ctx).await {
                                    warn!(ctx, "Failed to fetch existing messages: {:#}", err);
                                    connection.trigger_reconnect(&ctx);
                                }
                            }
                        }
                        Err(err) => {
                            warn!(ctx, "Can't get Config::FetchedExistingMsgs: {:#}", err);
                        }
                    }

                    info = fetch_idle(&ctx, &mut connection, FolderMeaning::Inbox).await;
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

/// Implement a single iteration of IMAP loop.
///
/// This function performs all IMAP operations on a single folder, selecting it if necessary and
/// handling all the errors. In case of an error, it is logged, but not propagated upwards. If
/// critical operation fails such as fetching new messages fails, connection is reset via
/// `trigger_reconnect`, so a fresh one can be opened.
async fn fetch_idle(
    ctx: &Context,
    connection: &mut Imap,
    folder_meaning: FolderMeaning,
) -> InterruptInfo {
    let folder_config = match folder_meaning.to_config() {
        Some(c) => c,
        None => {
            error!(ctx, "Bad folder meaning: {}", folder_meaning);
            return connection
                .fake_idle(ctx, None, FolderMeaning::Unknown)
                .await;
        }
    };
    let folder = match ctx.get_config(folder_config).await {
        Ok(folder) => folder,
        Err(err) => {
            warn!(
                ctx,
                "Can not watch {} folder, failed to retrieve config: {:#}", folder_config, err
            );
            return connection
                .fake_idle(ctx, None, FolderMeaning::Unknown)
                .await;
        }
    };

    let watch_folder = if let Some(watch_folder) = folder {
        watch_folder
    } else {
        connection.connectivity.set_not_configured(ctx).await;
        info!(ctx, "Can not watch {} folder, not set", folder_config);
        return connection
            .fake_idle(ctx, None, FolderMeaning::Unknown)
            .await;
    };

    // connect and fake idle if unable to connect
    if let Err(err) = connection
        .prepare(ctx)
        .await
        .context("prepare IMAP connection")
    {
        warn!(ctx, "{:#}", err);
        connection.trigger_reconnect(ctx);
        return connection
            .fake_idle(ctx, Some(watch_folder), folder_meaning)
            .await;
    }

    if folder_config == Config::ConfiguredInboxFolder {
        if let Some(session) = connection.session.as_mut() {
            session
                .store_seen_flags_on_imap(ctx)
                .await
                .context("store_seen_flags_on_imap")
                .ok_or_log(ctx);
        } else {
            warn!(ctx, "No session even though we just prepared it");
        }
    }

    // Fetch the watched folder.
    if let Err(err) = connection
        .fetch_move_delete(ctx, &watch_folder, folder_meaning)
        .await
        .context("fetch_move_delete")
    {
        connection.trigger_reconnect(ctx);
        warn!(ctx, "{:#}", err);
        return InterruptInfo::new(false);
    }

    // Mark expired messages for deletion. Marked messages will be deleted from the server
    // on the next iteration of `fetch_move_delete`. `delete_expired_imap_messages` is not
    // called right before `fetch_move_delete` because it is not well optimized and would
    // otherwise slow down message fetching.
    delete_expired_imap_messages(ctx)
        .await
        .context("delete_expired_imap_messages")
        .ok_or_log(ctx);

    // Scan additional folders only after finishing fetching the watched folder.
    //
    // On iOS the application has strictly limited time to work in background, so we may not
    // be able to scan all folders before time is up if there are many of them.
    if folder_config == Config::ConfiguredInboxFolder {
        // Only scan on the Inbox thread in order to prevent parallel scans, which might lead to duplicate messages
        match connection.scan_folders(ctx).await.context("scan_folders") {
            Err(err) => {
                // Don't reconnect, if there is a problem with the connection we will realize this when IDLEing
                // but maybe just one folder can't be selected or something
                warn!(ctx, "{:#}", err);
            }
            Ok(true) => {
                // Fetch the watched folder again in case scanning other folder moved messages
                // there.
                //
                // In most cases this will select the watched folder and return because there are
                // no new messages. We want to select the watched folder anyway before going IDLE
                // there, so this does not take additional protocol round-trip.
                if let Err(err) = connection
                    .fetch_move_delete(ctx, &watch_folder, folder_meaning)
                    .await
                    .context("fetch_move_delete after scan_folders")
                {
                    connection.trigger_reconnect(ctx);
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

    ctx.emit_event(EventType::ImapInboxIdle);
    if let Some(session) = connection.session.take() {
        if !session.can_idle() {
            info!(
                ctx,
                "IMAP session does not support IDLE, going to fake idle."
            );
            return connection
                .fake_idle(ctx, Some(watch_folder), folder_meaning)
                .await;
        }

        info!(ctx, "IMAP session supports IDLE, using it.");
        match session
            .idle(
                ctx,
                connection.idle_interrupt_receiver.clone(),
                Some(watch_folder),
            )
            .await
            .context("idle")
        {
            Ok((session, info)) => {
                connection.session = Some(session);
                info
            }
            Err(err) => {
                connection.trigger_reconnect(ctx);
                warn!(ctx, "{:#}", err);
                InterruptInfo::new(false)
            }
        }
    } else {
        warn!(ctx, "No IMAP session, going to fake idle.");
        connection
            .fake_idle(ctx, Some(watch_folder), folder_meaning)
            .await
    }
}

async fn simple_imap_loop(
    ctx: Context,
    started: Sender<()>,
    inbox_handlers: ImapConnectionHandlers,
    folder_meaning: FolderMeaning,
) {
    use futures::future::FutureExt;

    info!(ctx, "starting simple loop for {}", folder_meaning);
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
            fetch_idle(&ctx, &mut connection, folder_meaning).await;
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
                        "smtp got rate limited, waiting for {} until can send again",
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
        let (smtp, smtp_handlers) = SmtpConnectionState::new();

        let (smtp_start_send, smtp_start_recv) = channel::bounded(1);
        let (ephemeral_interrupt_send, ephemeral_interrupt_recv) = channel::bounded(1);
        let (location_interrupt_send, location_interrupt_recv) = channel::bounded(1);

        let mut oboxes = Vec::new();
        let mut start_recvs = Vec::new();

        let (conn_state, inbox_handlers) = ImapConnectionState::new(&ctx).await?;
        let (inbox_start_send, inbox_start_recv) = channel::bounded(1);
        let handle = {
            let ctx = ctx.clone();
            task::spawn(inbox_loop(ctx, inbox_start_send, inbox_handlers))
        };
        let inbox = SchedBox {
            meaning: FolderMeaning::Inbox,
            conn_state,
            handle,
        };
        start_recvs.push(inbox_start_recv);

        for (meaning, should_watch) in [
            (FolderMeaning::Mvbox, ctx.should_watch_mvbox().await),
            (
                FolderMeaning::Sent,
                ctx.get_config_bool(Config::SentboxWatch).await,
            ),
        ] {
            if should_watch? {
                let (conn_state, handlers) = ImapConnectionState::new(&ctx).await?;
                let (start_send, start_recv) = channel::bounded(1);
                let ctx = ctx.clone();
                let handle = task::spawn(simple_imap_loop(ctx, start_send, handlers, meaning));
                oboxes.push(SchedBox {
                    meaning,
                    conn_state,
                    handle,
                });
                start_recvs.push(start_recv);
            }
        }

        let smtp_handle = {
            let ctx = ctx.clone();
            task::spawn(smtp_loop(ctx, smtp_start_send, smtp_handlers))
        };
        start_recvs.push(smtp_start_recv);

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

        let recently_seen_loop = RecentlySeenLoop::new(ctx.clone());

        let res = Self {
            inbox,
            oboxes,
            smtp,
            smtp_handle,
            ephemeral_handle,
            ephemeral_interrupt_send,
            location_handle,
            location_interrupt_send,
            recently_seen_loop,
        };

        // wait for all loops to be started
        if let Err(err) = try_join_all(start_recvs.iter().map(|r| r.recv())).await {
            bail!("failed to start scheduler: {}", err);
        }

        info!(ctx, "scheduler is running");
        Ok(res)
    }

    fn boxes(&self) -> iter::Chain<iter::Once<&SchedBox>, std::slice::Iter<'_, SchedBox>> {
        once(&self.inbox).chain(self.oboxes.iter())
    }

    fn maybe_network(&self) {
        for b in self.boxes() {
            b.conn_state.interrupt(InterruptInfo::new(true));
        }
        self.interrupt_smtp(InterruptInfo::new(true));
    }

    fn maybe_network_lost(&self) {
        for b in self.boxes() {
            b.conn_state.interrupt(InterruptInfo::new(false));
        }
        self.interrupt_smtp(InterruptInfo::new(false));
    }

    fn interrupt_inbox(&self, info: InterruptInfo) {
        self.inbox.conn_state.interrupt(info);
    }

    fn interrupt_smtp(&self, info: InterruptInfo) {
        self.smtp.interrupt(info);
    }

    fn interrupt_ephemeral_task(&self) {
        self.ephemeral_interrupt_send.try_send(()).ok();
    }

    fn interrupt_location(&self) {
        self.location_interrupt_send.try_send(()).ok();
    }

    fn interrupt_recently_seen(&self, contact_id: ContactId, timestamp: i64) {
        self.recently_seen_loop.interrupt(contact_id, timestamp);
    }

    /// Halt the scheduler.
    ///
    /// It consumes the scheduler and never fails to stop it. In the worst case, long-running tasks
    /// are forcefully terminated if they cannot shutdown within the timeout.
    pub(crate) async fn stop(self, context: &Context) {
        // Send stop signals to tasks so they can shutdown cleanly.
        for b in self.boxes() {
            b.conn_state.stop().await.ok_or_log(context);
        }
        self.smtp.stop().await.ok_or_log(context);

        // Actually shutdown tasks.
        let timeout_duration = std::time::Duration::from_secs(30);
        for b in once(self.inbox).chain(self.oboxes.into_iter()) {
            tokio::time::timeout(timeout_duration, b.handle)
                .await
                .ok_or_log(context);
        }
        tokio::time::timeout(timeout_duration, self.smtp_handle)
            .await
            .ok_or_log(context);
        self.ephemeral_handle.abort();
        self.location_handle.abort();
        self.recently_seen_loop.abort();
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

    fn interrupt(&self, info: InterruptInfo) {
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
    fn interrupt(&self, info: InterruptInfo) {
        self.state.interrupt(info);
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
    fn interrupt(&self, info: InterruptInfo) {
        self.state.interrupt(info);
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
