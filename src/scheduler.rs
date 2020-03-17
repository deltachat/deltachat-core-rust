use async_std::prelude::*;
use async_std::sync::{channel, Receiver, Sender};

const MAX_JOBS_WAITING: usize = 50;

use crate::imap::Imap;
use crate::smtp::Smtp;

/// Job and connection scheduler.
#[derive(Debug)]
pub(crate) enum Scheduler {
    Stopped,
    Running {
        inbox: ImapConnectionState<InboxJob>,
        mvbox: ImapConnectionState<MvboxJob>,
        sentbox: ImapConnectionState<SentboxJob>,
        smtp: SmtpConnectionState,
    },
}

impl Scheduler {
    /// Start the scheduler, panics if it is already running.
    pub async fn run(&mut self) {
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
            }
            Scheduler::Running { .. } => {
                // TODO: return an error
                panic!("WARN: already running");
            }
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
struct ConnectionState<T> {
    /// Channel to notify that shutdown has completed.
    shutdown_receiver: Receiver<()>,
    /// Channel to interrupt the whole connection.
    stop_sender: Sender<()>,
    /// Channel to receive new jobs.
    jobs_receiver: Receiver<T>,
    /// Channel to schedule new jobs.
    jobs_sender: Sender<T>,
}

impl<T> ConnectionState<T> {
    /// Send a new job.
    pub async fn send_job(&self, job: T) {
        self.jobs_sender.send(job).await;
    }

    /// Shutdown this connection completely.
    pub async fn stop(&self) {
        // Trigger shutdown of the run loop.
        self.stop_sender.send(()).await;
        // Wait for a notification that the run loop has been shutdown.
        self.shutdown_receiver.recv().await;
    }
}

#[derive(Debug)]
pub(crate) struct SmtpConnectionState {
    state: ConnectionState<SmtpJob>,
}

impl SmtpConnectionState {
    async fn new() -> (Self, SmtpConnectionHandlers) {
        let (jobs_sender, jobs_receiver) = channel(50);
        let (stop_sender, stop_receiver) = channel(1);
        let (shutdown_sender, shutdown_receiver) = channel(1);

        let handlers = SmtpConnectionHandlers {
            connection: Smtp::new(),
            stop_receiver,
            shutdown_sender,
        };

        let state = ConnectionState {
            shutdown_receiver,
            stop_sender,
            jobs_sender,
            jobs_receiver,
        };

        let conn = SmtpConnectionState { state };

        (conn, handlers)
    }

    /// Send a new job.
    async fn send_job(&self, job: SmtpJob) {
        self.state.send_job(job).await;
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
}

#[derive(Debug)]
pub(crate) struct ImapConnectionState<T> {
    /// Channel to interrupt idle.
    idle_interrupt_sender: Sender<()>,
    state: ConnectionState<T>,
}

impl<T> ImapConnectionState<T> {
    /// Construct a new connection.
    async fn new() -> (Self, ImapConnectionHandlers) {
        let (jobs_sender, jobs_receiver) = channel(MAX_JOBS_WAITING);
        let (stop_sender, stop_receiver) = channel(1);
        let (idle_interrupt_sender, idle_interrupt_receiver) = channel(1);
        let (shutdown_sender, shutdown_receiver) = channel(1);

        let handlers = ImapConnectionHandlers {
            connection: Imap::new(idle_interrupt_receiver),
            stop_receiver,
            shutdown_sender,
        };

        let state = ConnectionState {
            shutdown_receiver,
            stop_sender,
            jobs_sender,
            jobs_receiver,
        };

        let conn = ImapConnectionState {
            idle_interrupt_sender,
            state,
        };

        (conn, handlers)
    }

    /// Send a new job.
    async fn send_job(&self, job: T) {
        self.state
            .send_job(job)
            .join(self.idle_interrupt_sender.send(()))
            .await;
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

/// Jobs handled by the inbox connection.
#[derive(Debug)]
pub enum InboxJob {}

/// Jobs handled by the mvbox connection.
#[derive(Debug)]
pub enum MvboxJob {}

/// Jobs handled by the sentbox connection.
#[derive(Debug)]
pub enum SentboxJob {}

/// Jobs handled by the smtp connection.
#[derive(Debug)]
pub enum SmtpJob {}
