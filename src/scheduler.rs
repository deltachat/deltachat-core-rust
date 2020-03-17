use async_std::prelude::*;
use async_std::sync::{channel, Receiver, Sender};
use async_std::task;

use std::time::Duration;

use crate::context::Context;
use crate::imap::Imap;
use crate::job::{self, Thread};
use crate::smtp::Smtp;

/// Job and connection scheduler.
#[derive(Debug)]
pub(crate) enum Scheduler {
    Stopped,
    Running {
        inbox: ImapConnectionState,
        mvbox: ImapConnectionState,
        sentbox: ImapConnectionState,
        smtp: SmtpConnectionState,
    },
}

impl Context {
    pub(crate) async fn interrupt_inbox(&self) {
        self.scheduler.read().await.interrupt_inbox().await;
    }

    pub(crate) async fn interrupt_sentbox(&self) {
        self.scheduler.read().await.interrupt_sentbox().await;
    }

    pub(crate) async fn interrupt_mvbox(&self) {
        self.scheduler.read().await.interrupt_mvbox().await;
    }

    pub(crate) async fn interrupt_smtp(&self) {
        self.scheduler.read().await.interrupt_smtp().await;
    }
}

impl Scheduler {
    /// Start the scheduler, panics if it is already running.
    pub async fn run(&mut self, ctx: Context) {
        match self {
            Scheduler::Stopped => {
                let (
                    (
                        ((inbox, inbox_handlers), (mvbox, mvbox_handlers)),
                        (sentbox, sentbox_handlers),
                    ),
                    (smtp, smtp_handlers),
                ) = ImapConnectionState::new()
                    .join(ImapConnectionState::new())
                    .join(ImapConnectionState::new())
                    .join(SmtpConnectionState::new())
                    .await;
                *self = Scheduler::Running {
                    inbox,
                    mvbox,
                    sentbox,
                    smtp,
                };

                let ctx1 = ctx.clone();
                task::spawn(async move {
                    let ImapConnectionHandlers {
                        mut connection,
                        stop_receiver,
                        shutdown_sender,
                    } = inbox_handlers;

                    let fut = async move {
                        loop {
                            // TODO: correct value
                            let probe_network = false;
                            match job::load_next(&ctx1, Thread::Imap, probe_network)
                                .timeout(Duration::from_millis(200))
                                .await
                            {
                                Ok(Some(job)) => {
                                    job::perform_job(
                                        &ctx1,
                                        job::Connection::Inbox(&mut connection),
                                        job,
                                    )
                                    .await;
                                }
                                Ok(None) | Err(async_std::future::TimeoutError { .. }) => {
                                    // fetch
                                    connection.fetch(&ctx1, "TODO").await;

                                    // idle
                                    connection.idle(&ctx1, Some("TODO".into())).await;
                                }
                            }
                        }
                    };

                    fut.race(stop_receiver.recv()).await;
                    shutdown_sender.send(()).await;
                });

                // TODO: mvbox

                // TODO: sentbox

                let ctx1 = ctx.clone();
                task::spawn(async move {
                    let SmtpConnectionHandlers {
                        mut connection,
                        stop_receiver,
                        shutdown_sender,
                        idle_interrupt_receiver,
                    } = smtp_handlers;

                    let fut = async move {
                        loop {
                            // TODO: correct value
                            let probe_network = false;
                            match job::load_next(&ctx1, Thread::Smtp, probe_network)
                                .timeout(Duration::from_millis(200))
                                .await
                            {
                                Ok(Some(job)) => {
                                    job::perform_job(
                                        &ctx1,
                                        job::Connection::Smtp(&mut connection),
                                        job,
                                    )
                                    .await;
                                }
                                Ok(None) | Err(async_std::future::TimeoutError { .. }) => {
                                    use futures::future::FutureExt;

                                    // Fake Idle
                                    async_std::task::sleep(Duration::from_millis(500))
                                        .race(idle_interrupt_receiver.recv().map(|_| ()))
                                        .await;
                                }
                            }
                        }
                    };

                    fut.race(stop_receiver.recv()).await;
                    shutdown_sender.send(()).await;
                });
            }
            Scheduler::Running { .. } => {
                // TODO: return an error
                panic!("WARN: already running");
            }
        }
    }

    fn inbox(&self) -> Option<&ImapConnectionState> {
        match self {
            Scheduler::Running { ref inbox, .. } => Some(inbox),
            _ => None,
        }
    }

    async fn interrupt_inbox(&self) {
        match self {
            Scheduler::Running { ref inbox, .. } => inbox.interrupt().await,
            _ => panic!("interrupt_imap must be called in running mode"),
        }
    }

    async fn interrupt_mvbox(&self) {
        match self {
            Scheduler::Running { ref mvbox, .. } => mvbox.interrupt().await,
            _ => panic!("interrupt_mvbox must be called in running mode"),
        }
    }

    async fn interrupt_sentbox(&self) {
        match self {
            Scheduler::Running { ref sentbox, .. } => sentbox.interrupt().await,
            _ => panic!("interrupt_sentbox must be called in running mode"),
        }
    }

    async fn interrupt_smtp(&self) {
        match self {
            Scheduler::Running { ref smtp, .. } => smtp.interrupt().await,
            _ => panic!("interrupt_smtp must be called in running mode"),
        }
    }

    /// Halt the scheduler, panics if it is already stopped.
    pub async fn stop(&mut self) {
        match self {
            Scheduler::Stopped => {
                panic!("WARN: already stopped");
            }
            Scheduler::Running {
                inbox,
                mvbox,
                sentbox,
                smtp,
            } => {
                inbox
                    .stop()
                    .join(mvbox.stop())
                    .join(sentbox.stop())
                    .join(smtp.stop())
                    .await;
            }
        }
    }

    /// Check if the scheduler is running.
    pub fn is_running(&self) -> bool {
        match self {
            Scheduler::Running { .. } => true,
            _ => false,
        }
    }

    /// Check if the scheduler is stoppd.
    pub fn is_stopped(&self) -> bool {
        match self {
            Scheduler::Stopped => true,
            _ => false,
        }
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
    idle_interrupt_sender: Sender<()>,
}

impl ConnectionState {
    /// Shutdown this connection completely.
    async fn stop(&self) {
        // Trigger shutdown of the run loop.
        self.stop_sender.send(()).await;
        // Wait for a notification that the run loop has been shutdown.
        self.shutdown_receiver.recv().await;
    }

    async fn interrupt(&self) {
        self.idle_interrupt_sender.send(()).await;
    }
}

#[derive(Debug)]
pub(crate) struct SmtpConnectionState {
    state: ConnectionState,
}

impl SmtpConnectionState {
    async fn new() -> (Self, SmtpConnectionHandlers) {
        let (stop_sender, stop_receiver) = channel(1);
        let (shutdown_sender, shutdown_receiver) = channel(1);
        let (idle_interrupt_sender, idle_interrupt_receiver) = channel(1);

        let handlers = SmtpConnectionHandlers {
            connection: Smtp::new(),
            stop_receiver,
            shutdown_sender,
            idle_interrupt_receiver,
        };

        let state = ConnectionState {
            idle_interrupt_sender,
            shutdown_receiver,
            stop_sender,
        };

        let conn = SmtpConnectionState { state };

        (conn, handlers)
    }

    /// Interrupt any form of idle.
    async fn interrupt(&self) {
        self.state.interrupt().await;
    }

    /// Shutdown this connection completely.
    async fn stop(&self) {
        self.state.stop().await;
    }
}

#[derive(Debug)]
struct SmtpConnectionHandlers {
    connection: Smtp,
    stop_receiver: Receiver<()>,
    shutdown_sender: Sender<()>,
    idle_interrupt_receiver: Receiver<()>,
}

#[derive(Debug)]
pub(crate) struct ImapConnectionState {
    state: ConnectionState,
}

impl ImapConnectionState {
    /// Construct a new connection.
    async fn new() -> (Self, ImapConnectionHandlers) {
        let (stop_sender, stop_receiver) = channel(1);
        let (idle_interrupt_sender, idle_interrupt_receiver) = channel(1);
        let (shutdown_sender, shutdown_receiver) = channel(1);

        let handlers = ImapConnectionHandlers {
            connection: Imap::new(idle_interrupt_receiver),
            stop_receiver,
            shutdown_sender,
        };

        let state = ConnectionState {
            idle_interrupt_sender,
            shutdown_receiver,
            stop_sender,
        };

        let conn = ImapConnectionState { state };

        (conn, handlers)
    }

    /// Interrupt any form of idle.
    async fn interrupt(&self) {
        self.state.interrupt().await;
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
