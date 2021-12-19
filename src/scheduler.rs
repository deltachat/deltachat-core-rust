use anyhow::{bail, Result};
use async_std::prelude::*;
use async_std::{
    channel::{self, Receiver, Sender},
    task,
};

use crate::config::Config;
use crate::context::Context;
use crate::dc_tools::maybe_add_time_based_warnings;
use crate::imap::Imap;
use crate::job::{self, Thread};
use crate::message::MsgId;
use crate::smtp::Smtp;

use self::connectivity::ConnectivityStore;

pub(crate) mod connectivity;

pub(crate) struct StopToken;

/// Job and connection scheduler.
#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub(crate) enum Scheduler {
    Stopped,
    Running {
        inbox: ImapConnectionState,
        inbox_handle: Option<task::JoinHandle<()>>,
        mvbox: ImapConnectionState,
        mvbox_handle: Option<task::JoinHandle<()>>,
        sentbox: ImapConnectionState,
        sentbox_handle: Option<task::JoinHandle<()>>,
        smtp: SmtpConnectionState,
        smtp_handle: Option<task::JoinHandle<()>>,
    },
}

impl Context {
    /// Indicate that the network likely has come back.
    pub async fn maybe_network(&self) {
        let lock = self.scheduler.read().await;
        lock.maybe_network().await;
        connectivity::idle_interrupted(lock).await;
    }

    /// Indicate that the network likely is lost.
    pub async fn maybe_network_lost(&self) {
        let lock = self.scheduler.read().await;
        lock.maybe_network_lost().await;
        connectivity::maybe_network_lost(self, lock).await;
    }

    pub(crate) async fn interrupt_inbox(&self, info: InterruptInfo) {
        self.scheduler.read().await.interrupt_inbox(info).await;
    }

    pub(crate) async fn interrupt_smtp(&self, info: InterruptInfo) {
        self.scheduler.read().await.interrupt_smtp(info).await;
    }
}

async fn inbox_loop(ctx: Context, started: Sender<()>, inbox_handlers: ImapConnectionHandlers) {
    use futures::future::FutureExt;

    info!(ctx, "starting inbox loop");
    let ImapConnectionHandlers {
        mut connection,
        stop_receiver,
        shutdown_sender,
    } = inbox_handlers;

    let ctx1 = ctx.clone();
    let fut = async move {
        started
            .send(())
            .await
            .expect("inbox loop, missing started receiver");
        let ctx = ctx1;

        // track number of continously executed jobs
        let mut jobs_loaded = 0;
        let mut info = InterruptInfo::default();
        loop {
            let job = match job::load_next(&ctx, Thread::Imap, &info).await {
                Err(err) => {
                    error!(ctx, "Failed loading job from the database: {:#}.", err);
                    None
                }
                Ok(job) => job,
            };

            match job {
                Some(job) if jobs_loaded <= 20 => {
                    jobs_loaded += 1;
                    job::perform_job(&ctx, job::Connection::Inbox(&mut connection), job).await;
                    info = Default::default();
                }
                Some(job) => {
                    // Let the fetch run, but return back to the job afterwards.
                    jobs_loaded = 0;
                    if ctx
                        .get_config_bool(Config::InboxWatch)
                        .await
                        .unwrap_or_default()
                    {
                        info!(ctx, "postponing imap-job {} to run fetch...", job);
                        fetch(&ctx, &mut connection).await;
                    }
                }
                None => {
                    jobs_loaded = 0;

                    maybe_add_time_based_warnings(&ctx).await;

                    info = if ctx
                        .get_config_bool(Config::InboxWatch)
                        .await
                        .unwrap_or_default()
                    {
                        fetch_idle(&ctx, &mut connection, Config::ConfiguredInboxFolder).await
                    } else {
                        if let Err(err) = connection.scan_folders(&ctx).await {
                            warn!(ctx, "{}", err);
                            connection.connectivity.set_err(&ctx, err).await;
                        } else {
                            connection.connectivity.set_not_configured(&ctx).await;
                        }
                        connection.fake_idle(&ctx, None).await
                    };
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
    shutdown_sender
        .send(())
        .await
        .expect("inbox loop, missing shutdown receiver");
}

async fn fetch(ctx: &Context, connection: &mut Imap) {
    match ctx.get_config(Config::ConfiguredInboxFolder).await {
        Ok(Some(watch_folder)) => {
            if let Err(err) = connection.prepare(ctx).await {
                warn!(ctx, "Could not connect: {}", err);
                return;
            }

            // fetch
            if let Err(err) = connection.fetch_move_delete(ctx, &watch_folder).await {
                connection.trigger_reconnect(ctx).await;
                warn!(ctx, "{:#}", err);
            }
        }
        Ok(None) => {
            info!(ctx, "Can not fetch inbox folder, not set");
        }
        Err(err) => {
            warn!(
                ctx,
                "Can not fetch inbox folder, failed to get config: {:?}", err
            );
        }
    }
}

async fn fetch_idle(ctx: &Context, connection: &mut Imap, folder: Config) -> InterruptInfo {
    match ctx.get_config(folder).await {
        Ok(Some(watch_folder)) => {
            // connect and fake idle if unable to connect
            if let Err(err) = connection.prepare(ctx).await {
                warn!(ctx, "imap connection failed: {}", err);
                return connection.fake_idle(ctx, Some(watch_folder)).await;
            }

            // Scan other folders before fetching from watched folder. This may result in the
            // messages being moved into the watched folder, for example from the Spam folder to
            // the Inbox folder.
            if folder == Config::ConfiguredInboxFolder {
                // Only scan on the Inbox thread in order to prevent parallel scans, which might lead to duplicate messages
                if let Err(err) = connection.scan_folders(ctx).await {
                    // Don't reconnect, if there is a problem with the connection we will realize this when IDLEing
                    // but maybe just one folder can't be selected or something
                    warn!(ctx, "{}", err);
                }
            }

            // fetch
            if let Err(err) = connection.fetch_move_delete(ctx, &watch_folder).await {
                connection.trigger_reconnect(ctx).await;
                warn!(ctx, "{:#}", err);
                return InterruptInfo::new(false, None);
            }

            connection.connectivity.set_connected(ctx).await;

            // idle
            if connection.can_idle() {
                match connection.idle(ctx, Some(watch_folder)).await {
                    Ok(v) => v,
                    Err(err) => {
                        connection.trigger_reconnect(ctx).await;
                        warn!(ctx, "{}", err);
                        InterruptInfo::new(false, None)
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
        shutdown_sender,
    } = inbox_handlers;

    let ctx1 = ctx.clone();

    let fut = async move {
        started
            .send(())
            .await
            .expect("simple imap loop, missing started receive");
        let ctx = ctx1;

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
    shutdown_sender
        .send(())
        .await
        .expect("simple imap loop, missing shutdown receiver");
}

async fn smtp_loop(ctx: Context, started: Sender<()>, smtp_handlers: SmtpConnectionHandlers) {
    use futures::future::FutureExt;

    info!(ctx, "starting smtp loop");
    let SmtpConnectionHandlers {
        mut connection,
        stop_receiver,
        shutdown_sender,
        idle_interrupt_receiver,
    } = smtp_handlers;

    let ctx1 = ctx.clone();
    let fut = async move {
        started
            .send(())
            .await
            .expect("smtp loop, missing started receiver");
        let ctx = ctx1;

        let mut interrupt_info = Default::default();
        loop {
            match job::load_next(&ctx, Thread::Smtp, &interrupt_info)
                .await
                .ok()
                .flatten()
            {
                Some(job) => {
                    info!(ctx, "executing smtp job");
                    job::perform_job(&ctx, job::Connection::Smtp(&mut connection), job).await;
                    interrupt_info = Default::default();
                }
                None => {
                    // Fake Idle
                    info!(ctx, "smtp fake idle - started");
                    match &connection.last_send_error {
                        None => connection.connectivity.set_connected(&ctx).await,
                        Some(err) => connection.connectivity.set_err(&ctx, err).await,
                    }

                    interrupt_info = idle_interrupt_receiver.recv().await.unwrap_or_default();
                    info!(ctx, "smtp fake idle - interrupted")
                }
            }
        }
    };

    stop_receiver
        .recv()
        .map(|_| {
            info!(ctx, "shutting down smtp loop");
        })
        .race(fut)
        .await;
    shutdown_sender
        .send(())
        .await
        .expect("smtp loop, missing shutdown receiver");
}

impl Scheduler {
    /// Start the scheduler, panics if it is already running.
    pub async fn start(&mut self, ctx: Context) -> Result<()> {
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

        let inbox_handle = {
            let ctx = ctx.clone();
            Some(task::spawn(async move {
                inbox_loop(ctx, inbox_start_send, inbox_handlers).await
            }))
        };

        if ctx.get_config_bool(Config::MvboxMove).await? {
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
                .expect("mvbox start send, missing receiver");
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
                .expect("sentbox start send, missing receiver");
            sentbox_handlers
                .connection
                .connectivity
                .set_not_configured(&ctx)
                .await
        }

        let smtp_handle = {
            let ctx = ctx.clone();
            Some(task::spawn(async move {
                smtp_loop(ctx, smtp_start_send, smtp_handlers).await
            }))
        };

        *self = Scheduler::Running {
            inbox,
            mvbox,
            sentbox,
            smtp,
            inbox_handle,
            mvbox_handle,
            sentbox_handle,
            smtp_handle,
        };

        // wait for all loops to be started
        if let Err(err) = inbox_start_recv
            .recv()
            .try_join(mvbox_start_recv.recv())
            .try_join(sentbox_start_recv.recv())
            .try_join(smtp_start_recv.recv())
            .await
        {
            bail!("failed to start scheduler: {}", err);
        }

        info!(ctx, "scheduler is running");
        Ok(())
    }

    async fn maybe_network(&self) {
        if !self.is_running() {
            return;
        }

        self.interrupt_inbox(InterruptInfo::new(true, None))
            .join(self.interrupt_mvbox(InterruptInfo::new(true, None)))
            .join(self.interrupt_sentbox(InterruptInfo::new(true, None)))
            .join(self.interrupt_smtp(InterruptInfo::new(true, None)))
            .await;
    }

    async fn maybe_network_lost(&self) {
        if !self.is_running() {
            return;
        }

        self.interrupt_inbox(InterruptInfo::new(false, None))
            .join(self.interrupt_mvbox(InterruptInfo::new(false, None)))
            .join(self.interrupt_sentbox(InterruptInfo::new(false, None)))
            .join(self.interrupt_smtp(InterruptInfo::new(false, None)))
            .await;
    }

    async fn interrupt_inbox(&self, info: InterruptInfo) {
        if let Scheduler::Running { ref inbox, .. } = self {
            inbox.interrupt(info).await;
        }
    }

    async fn interrupt_mvbox(&self, info: InterruptInfo) {
        if let Scheduler::Running { ref mvbox, .. } = self {
            mvbox.interrupt(info).await;
        }
    }

    async fn interrupt_sentbox(&self, info: InterruptInfo) {
        if let Scheduler::Running { ref sentbox, .. } = self {
            sentbox.interrupt(info).await;
        }
    }

    async fn interrupt_smtp(&self, info: InterruptInfo) {
        if let Scheduler::Running { ref smtp, .. } = self {
            smtp.interrupt(info).await;
        }
    }

    /// Halts the scheduler, must be called first, and then `stop`.
    pub(crate) async fn pre_stop(&self) -> StopToken {
        match self {
            Scheduler::Stopped => {
                panic!("WARN: already stopped");
            }
            Scheduler::Running {
                inbox,
                inbox_handle,
                mvbox,
                mvbox_handle,
                sentbox,
                sentbox_handle,
                smtp,
                smtp_handle,
                ..
            } => {
                if inbox_handle.is_some() {
                    inbox.stop().await;
                }
                if mvbox_handle.is_some() {
                    mvbox.stop().await;
                }
                if sentbox_handle.is_some() {
                    sentbox.stop().await;
                }
                if smtp_handle.is_some() {
                    smtp.stop().await;
                }

                StopToken
            }
        }
    }

    /// Halt the scheduler, must only be called after pre_stop.
    pub(crate) async fn stop(&mut self, _t: StopToken) {
        match self {
            Scheduler::Stopped => {
                panic!("WARN: already stopped");
            }
            Scheduler::Running {
                inbox_handle,
                mvbox_handle,
                sentbox_handle,
                smtp_handle,
                ..
            } => {
                if let Some(handle) = inbox_handle.take() {
                    handle.await;
                }
                if let Some(handle) = mvbox_handle.take() {
                    handle.await;
                }
                if let Some(handle) = sentbox_handle.take() {
                    handle.await;
                }
                if let Some(handle) = smtp_handle.take() {
                    handle.await;
                }

                *self = Scheduler::Stopped;
            }
        }
    }

    /// Check if the scheduler is running.
    pub fn is_running(&self) -> bool {
        matches!(self, Scheduler::Running { .. })
    }
}

/// Connection state logic shared between imap and smtp connections.
#[derive(Debug)]
struct ConnectionState {
    /// Channel to notify that shutdown has completed.
    shutdown_receiver: Receiver<()>,
    /// Channel to interrupt the whole connection.
    stop_sender: Sender<()>,
    /// Channel to interrupt idle.
    idle_interrupt_sender: Sender<InterruptInfo>,
    /// Mutex to pass connectivity info between IMAP/SMTP threads and the API
    connectivity: ConnectivityStore,
}

impl ConnectionState {
    /// Shutdown this connection completely.
    async fn stop(&self) {
        // Trigger shutdown of the run loop.
        self.stop_sender
            .send(())
            .await
            .expect("stop, missing receiver");
        // Wait for a notification that the run loop has been shutdown.
        self.shutdown_receiver.recv().await.ok();
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
        let (shutdown_sender, shutdown_receiver) = channel::bounded(1);
        let (idle_interrupt_sender, idle_interrupt_receiver) = channel::bounded(1);

        let handlers = SmtpConnectionHandlers {
            connection: Smtp::new(),
            stop_receiver,
            shutdown_sender,
            idle_interrupt_receiver,
        };

        let state = ConnectionState {
            shutdown_receiver,
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
    async fn stop(&self) {
        self.state.stop().await;
    }
}

struct SmtpConnectionHandlers {
    connection: Smtp,
    stop_receiver: Receiver<()>,
    shutdown_sender: Sender<()>,
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
        let (shutdown_sender, shutdown_receiver) = channel::bounded(1);
        let (idle_interrupt_sender, idle_interrupt_receiver) = channel::bounded(1);

        let handlers = ImapConnectionHandlers {
            connection: Imap::new_configured(context, idle_interrupt_receiver).await?,
            stop_receiver,
            shutdown_sender,
        };

        let state = ConnectionState {
            shutdown_receiver,
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
    async fn stop(&self) {
        self.state.stop().await;
    }
}

#[derive(Debug)]
struct ImapConnectionHandlers {
    connection: Imap,
    stop_receiver: Receiver<()>,
    shutdown_sender: Sender<()>,
}

#[derive(Default, Debug)]
pub struct InterruptInfo {
    pub probe_network: bool,
    pub msg_id: Option<MsgId>,
}

impl InterruptInfo {
    pub fn new(probe_network: bool, msg_id: Option<MsgId>) -> Self {
        Self {
            probe_network,
            msg_id,
        }
    }
}
