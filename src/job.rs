//! # Job module
//!
//! This module implements a job queue maintained in the SQLite database
//! and job types.

use std::{fmt, time};

use deltachat_derive::{FromSql, ToSql};
use itertools::Itertools;
use rand::{thread_rng, Rng};

use async_smtp::smtp::response::Category;
use async_smtp::smtp::response::Code;
use async_smtp::smtp::response::Detail;
use async_std::task;

use crate::blob::BlobObject;
use crate::chat::{self, ChatId};
use crate::config::Config;
use crate::configure::*;
use crate::constants::*;
use crate::contact::Contact;
use crate::context::{Context, PerformJobsNeeded};
use crate::dc_tools::*;
use crate::error::{bail, ensure, format_err, Error, Result};
use crate::events::Event;
use crate::imap::*;
use crate::imex::*;
use crate::location;
use crate::login_param::LoginParam;
use crate::message::MsgId;
use crate::message::{self, Message, MessageState};
use crate::mimefactory::{MimeFactory, RenderedEmail};
use crate::param::*;
use crate::sql;

// results in ~3 weeks for the last backoff timespan
const JOB_RETRIES: u32 = 17;

/// Thread IDs
#[derive(Debug, Display, Copy, Clone, PartialEq, Eq, FromPrimitive, ToPrimitive, FromSql, ToSql)]
#[repr(i32)]
enum Thread {
    Unknown = 0,
    Imap = 100,
    Smtp = 5000,
}

/// Job try result.
#[derive(Debug, Display)]
pub enum Status {
    Finished(std::result::Result<(), Error>),
    RetryNow,
    RetryLater,
}

#[macro_export]
macro_rules! job_try {
    ($expr:expr) => {
        match $expr {
            ::std::result::Result::Ok(val) => val,
            ::std::result::Result::Err(err) => {
                return $crate::job::Status::Finished(Err(err.into()));
            }
        }
    };
    ($expr:expr,) => {
        $crate::job_try!($expr)
    };
}

impl Default for Thread {
    fn default() -> Self {
        Thread::Unknown
    }
}

#[derive(
    Debug,
    Display,
    Copy,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    FromPrimitive,
    ToPrimitive,
    FromSql,
    ToSql,
)]
#[repr(i32)]
pub enum Action {
    Unknown = 0,

    // Jobs in the INBOX-thread, range from DC_IMAP_THREAD..DC_IMAP_THREAD+999
    Housekeeping = 105, // low priority ...
    EmptyServer = 107,
    OldDeleteMsgOnImap = 110,
    MarkseenMsgOnImap = 130,

    // Moving message is prioritized lower than deletion so we don't
    // bother moving message if it is already scheduled for deletion.
    MoveMsg = 200,

    DeleteMsgOnImap = 210,
    ConfigureImap = 900,
    ImexImap = 910, // ... high priority

    // Jobs in the SMTP-thread, range from DC_SMTP_THREAD..DC_SMTP_THREAD+999
    MaybeSendLocations = 5005, // low priority ...
    MaybeSendLocationsEnded = 5007,
    SendMdn = 5010,
    SendMsgToSmtp = 5901, // ... high priority
}

impl Default for Action {
    fn default() -> Self {
        Action::Unknown
    }
}

impl From<Action> for Thread {
    fn from(action: Action) -> Thread {
        use Action::*;

        match action {
            Unknown => Thread::Unknown,

            Housekeeping => Thread::Imap,
            OldDeleteMsgOnImap => Thread::Imap,
            DeleteMsgOnImap => Thread::Imap,
            EmptyServer => Thread::Imap,
            MarkseenMsgOnImap => Thread::Imap,
            MoveMsg => Thread::Imap,
            ConfigureImap => Thread::Imap,
            ImexImap => Thread::Imap,

            MaybeSendLocations => Thread::Smtp,
            MaybeSendLocationsEnded => Thread::Smtp,
            SendMdn => Thread::Smtp,
            SendMsgToSmtp => Thread::Smtp,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Job {
    pub job_id: u32,
    pub action: Action,
    pub foreign_id: u32,
    pub desired_timestamp: i64,
    pub added_timestamp: i64,
    pub tries: u32,
    pub param: Params,
    pub pending_error: Option<String>,
}

impl fmt::Display for Job {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "#{}, action {}", self.job_id, self.action)
    }
}

impl Job {
    fn new(action: Action, foreign_id: u32, param: Params, delay_seconds: i64) -> Self {
        let timestamp = time();

        Self {
            job_id: 0,
            action,
            foreign_id,
            desired_timestamp: timestamp + delay_seconds,
            added_timestamp: timestamp,
            tries: 0,
            param,
            pending_error: None,
        }
    }

    /// Deletes the job from the database.
    fn delete(&self, context: &Context) -> bool {
        if self.job_id != 0 {
            context
                .sql
                .execute("DELETE FROM jobs WHERE id=?;", params![self.job_id as i32])
                .is_ok()
        } else {
            // Already deleted.
            true
        }
    }

    /// Saves the job to the database, creating a new entry if necessary.
    ///
    /// The Job is consumed by this method.
    fn save(self, context: &Context) -> bool {
        let thread: Thread = self.action.into();

        if self.job_id != 0 {
            sql::execute(
                context,
                &context.sql,
                "UPDATE jobs SET desired_timestamp=?, tries=?, param=? WHERE id=?;",
                params![
                    self.desired_timestamp,
                    self.tries as i64,
                    self.param.to_string(),
                    self.job_id as i32,
                ],
            )
            .is_ok()
        } else {
            sql::execute(
                context,
                &context.sql,
                "INSERT INTO jobs (added_timestamp, thread, action, foreign_id, param, desired_timestamp) VALUES (?,?,?,?,?,?);",
                params![
                    self.added_timestamp,
                    thread,
                    self.action,
                    self.foreign_id,
                    self.param.to_string(),
                    self.desired_timestamp
                ]
            ).is_ok()
        }
    }

    fn smtp_send<F>(
        &mut self,
        context: &Context,
        recipients: Vec<async_smtp::EmailAddress>,
        message: Vec<u8>,
        job_id: u32,
        success_cb: F,
    ) -> Status
    where
        F: FnOnce() -> Result<()>,
    {
        // hold the smtp lock during sending of a job and
        // its ok/error response processing. Note that if a message
        // was sent we need to mark it in the database ASAP as we
        // otherwise might send it twice.
        let mut smtp = context.smtp.lock().unwrap();
        if std::env::var(crate::DCC_MIME_DEBUG).is_ok() {
            info!(context, "smtp-sending out mime message:");
            println!("{}", String::from_utf8_lossy(&message));
        }
        match task::block_on(smtp.send(context, recipients, message, job_id)) {
            Err(crate::smtp::send::Error::SendError(err)) => {
                // Remote error, retry later.
                warn!(context, "SMTP failed to send: {}", err);
                self.pending_error = Some(err.to_string());

                let res = match err {
                    async_smtp::smtp::error::Error::Permanent(ref response) => {
                        match response.code {
                            // Sometimes servers send a permanent error when actually it is a temporary error
                            // For documentation see https://tools.ietf.org/html/rfc3463

                            // Code 5.5.0, see https://support.delta.chat/t/every-other-message-gets-stuck/877/2
                            Code {
                                category: Category::MailSystem,
                                detail: Detail::Zero,
                                ..
                            } => Status::RetryLater,

                            _ => {
                                // If we do not retry, add an info message to the chat
                                // Error 5.7.1 should definitely go here: Yandex sends 5.7.1 with a link when it thinks that the email is SPAM.
                                match Message::load_from_db(context, MsgId::new(self.foreign_id)) {
                                    Ok(message) => chat::add_info_msg(
                                        context,
                                        message.chat_id,
                                        err.to_string(),
                                    ),
                                    Err(e) => warn!(
                                        context,
                                        "couldn't load chat_id to inform user about SMTP error: {}",
                                        e
                                    ),
                                };

                                Status::Finished(Err(format_err!("Permanent SMTP error: {}", err)))
                            }
                        }
                    }
                    async_smtp::smtp::error::Error::Transient(_) => {
                        // We got a transient 4xx response from SMTP server.
                        // Give some time until the server-side error maybe goes away.
                        Status::RetryLater
                    }
                    _ => {
                        if smtp.has_maybe_stale_connection() {
                            info!(context, "stale connection? immediately reconnecting");
                            Status::RetryNow
                        } else {
                            Status::RetryLater
                        }
                    }
                };

                // this clears last_success info
                smtp.disconnect();

                res
            }
            Err(crate::smtp::send::Error::EnvelopeError(err)) => {
                // Local error, job is invalid, do not retry.
                smtp.disconnect();
                warn!(context, "SMTP job is invalid: {}", err);
                Status::Finished(Err(err.into()))
            }
            Err(crate::smtp::send::Error::NoTransport) => {
                // Should never happen.
                // It does not even make sense to disconnect here.
                error!(context, "SMTP job failed because SMTP has no transport");
                Status::Finished(Err(format_err!("SMTP has not transport")))
            }
            Ok(()) => {
                job_try!(success_cb());
                Status::Finished(Ok(()))
            }
        }
    }

    #[allow(non_snake_case)]
    fn SendMsgToSmtp(&mut self, context: &Context) -> Status {
        /* connect to SMTP server, if not yet done */
        if !context.smtp.lock().unwrap().is_connected() {
            let loginparam = LoginParam::from_database(context, "configured_");
            if let Err(err) = context.smtp.lock().unwrap().connect(context, &loginparam) {
                warn!(context, "SMTP connection failure: {:?}", err);
                return Status::RetryLater;
            }
        }

        let filename = job_try!(job_try!(self
            .param
            .get_path(Param::File, context)
            .map_err(|_| format_err!("Can't get filename")))
        .ok_or_else(|| format_err!("Can't get filename")));
        let body = job_try!(dc_read_file(context, &filename));
        let recipients = job_try!(self.param.get(Param::Recipients).ok_or_else(|| {
            warn!(context, "Missing recipients for job {}", self.job_id);
            format_err!("Missing recipients")
        }));

        let recipients_list = recipients
            .split('\x1e')
            .filter_map(
                |addr| match async_smtp::EmailAddress::new(addr.to_string()) {
                    Ok(addr) => Some(addr),
                    Err(err) => {
                        warn!(context, "invalid recipient: {} {:?}", addr, err);
                        None
                    }
                },
            )
            .collect::<Vec<_>>();

        /* if there is a msg-id and it does not exist in the db, cancel sending.
        this happends if dc_delete_msgs() was called
        before the generated mime was sent out */
        if 0 != self.foreign_id && !message::exists(context, MsgId::new(self.foreign_id)) {
            return Status::Finished(Err(format_err!(
                "Not sending Message {} as it was deleted",
                self.foreign_id
            )));
        };

        let foreign_id = self.foreign_id;
        self.smtp_send(context, recipients_list, body, self.job_id, || {
            // smtp success, update db ASAP, then delete smtp file
            if 0 != foreign_id {
                set_delivered(context, MsgId::new(foreign_id));
            }
            // now also delete the generated file
            dc_delete_file(context, filename);
            Ok(())
        })
    }

    /// Get `SendMdn` jobs with foreign_id equal to `contact_id` excluding the `job_id` job.
    fn get_additional_mdn_jobs(
        &self,
        context: &Context,
        contact_id: u32,
    ) -> sql::Result<(Vec<u32>, Vec<String>)> {
        // Extract message IDs from job parameters
        let res: Vec<(u32, MsgId)> = context.sql.query_map(
            "SELECT id, param FROM jobs WHERE foreign_id=? AND id!=?",
            params![contact_id, self.job_id],
            |row| {
                let job_id: u32 = row.get(0)?;
                let params_str: String = row.get(1)?;
                let params: Params = params_str.parse().unwrap_or_default();
                Ok((job_id, params))
            },
            |jobs| {
                let res = jobs
                    .filter_map(|row| {
                        let (job_id, params) = row.ok()?;
                        let msg_id = params.get_msg_id()?;
                        Some((job_id, msg_id))
                    })
                    .collect();
                Ok(res)
            },
        )?;

        // Load corresponding RFC724 message IDs
        let mut job_ids = Vec::new();
        let mut rfc724_mids = Vec::new();
        for (job_id, msg_id) in res {
            if let Ok(Message { rfc724_mid, .. }) = Message::load_from_db(context, msg_id) {
                job_ids.push(job_id);
                rfc724_mids.push(rfc724_mid);
            }
        }
        Ok((job_ids, rfc724_mids))
    }

    #[allow(non_snake_case)]
    fn SendMdn(&mut self, context: &Context) -> Status {
        if !context.get_config_bool(Config::MdnsEnabled) {
            // User has disabled MDNs after job scheduling but before
            // execution.
            return Status::Finished(Err(format_err!("MDNs are disabled")));
        }

        let contact_id = self.foreign_id;
        let contact = job_try!(Contact::load_from_db(context, contact_id));
        if contact.is_blocked() {
            return Status::Finished(Err(format_err!("Contact is blocked")));
        }

        let msg_id = if let Some(msg_id) = self.param.get_msg_id() {
            msg_id
        } else {
            return Status::Finished(Err(format_err!(
                "SendMdn job has invalid parameters: {}",
                self.param
            )));
        };

        // Try to aggregate other SendMdn jobs and send a combined MDN.
        let (additional_job_ids, additional_rfc724_mids) = self
            .get_additional_mdn_jobs(context, contact_id)
            .unwrap_or_default();

        if !additional_rfc724_mids.is_empty() {
            info!(
                context,
                "SendMdn job: aggregating {} additional MDNs",
                additional_rfc724_mids.len()
            )
        }

        let msg = job_try!(Message::load_from_db(context, msg_id));
        let mimefactory = job_try!(MimeFactory::from_mdn(context, &msg, additional_rfc724_mids));
        let rendered_msg = job_try!(mimefactory.render());
        let body = rendered_msg.message;

        let addr = contact.get_addr();
        let recipient = job_try!(async_smtp::EmailAddress::new(addr.to_string())
            .map_err(|err| format_err!("invalid recipient: {} {:?}", addr, err)));
        let recipients = vec![recipient];

        /* connect to SMTP server, if not yet done */
        if !context.smtp.lock().unwrap().is_connected() {
            let loginparam = LoginParam::from_database(context, "configured_");
            if let Err(err) = context.smtp.lock().unwrap().connect(context, &loginparam) {
                warn!(context, "SMTP connection failure: {:?}", err);
                return Status::RetryLater;
            }
        }

        self.smtp_send(context, recipients, body, self.job_id, || {
            // Remove additional SendMdn jobs we have aggregated into this one.
            job_kill_ids(context, &additional_job_ids)?;
            Ok(())
        })
    }

    #[allow(non_snake_case)]
    fn MoveMsg(&mut self, context: &Context) -> Status {
        let imap_inbox = &context.inbox_thread.read().unwrap().imap;

        let msg = job_try!(Message::load_from_db(context, MsgId::new(self.foreign_id)));

        if let Err(err) = imap_inbox.ensure_configured_folders(context, true) {
            warn!(context, "could not configure folders: {:?}", err);
            return Status::RetryLater;
        }
        let dest_folder = context
            .sql
            .get_raw_config(context, "configured_mvbox_folder");

        if let Some(dest_folder) = dest_folder {
            let server_folder = msg.server_folder.as_ref().unwrap();

            match imap_inbox.mv(context, server_folder, msg.server_uid, &dest_folder) {
                ImapActionResult::RetryLater => Status::RetryLater,
                ImapActionResult::Success => {
                    // XXX Rust-Imap provides no target uid on mv, so just set it to 0
                    message::update_server_uid(context, &msg.rfc724_mid, &dest_folder, 0);
                    Status::Finished(Ok(()))
                }
                ImapActionResult::Failed => {
                    Status::Finished(Err(format_err!("IMAP action failed")))
                }
                ImapActionResult::AlreadyDone => Status::Finished(Ok(())),
            }
        } else {
            Status::Finished(Err(format_err!("No mvbox folder configured")))
        }
    }

    /// Deletes a message on the server.
    ///
    /// foreign_id is a MsgId pointing to a message in the trash chat
    /// or a hidden message.
    ///
    /// This job removes the database record. If there are no more
    /// records pointing to the same message on the server, the job
    /// also removes the message on the server.
    #[allow(non_snake_case)]
    fn DeleteMsgOnImap(&mut self, context: &Context) -> Status {
        let imap_inbox = &context.inbox_thread.read().unwrap().imap;

        let msg = job_try!(Message::load_from_db(context, MsgId::new(self.foreign_id)));

        if !msg.rfc724_mid.is_empty() {
            let cnt = message::rfc724_mid_cnt(context, &msg.rfc724_mid);
            info!(
                context,
                "Running delete job for message {} which has {} entries in the database",
                &msg.rfc724_mid,
                cnt
            );
            if cnt > 1 {
                info!(
                    context,
                    "The message is deleted from the server when all parts are deleted.",
                );
            } else {
                /* if this is the last existing part of the message,
                we delete the message from the server */
                let mid = msg.rfc724_mid;
                let server_folder = msg.server_folder.as_ref().unwrap();
                let res = if msg.server_uid == 0 {
                    // Message is already deleted on IMAP server.
                    ImapActionResult::AlreadyDone
                } else {
                    imap_inbox.delete_msg(context, &mid, server_folder, msg.server_uid)
                };
                match res {
                    ImapActionResult::AlreadyDone | ImapActionResult::Success => {}
                    ImapActionResult::RetryLater | ImapActionResult::Failed => {
                        // If job has failed, for example due to some
                        // IMAP bug, we postpone it instead of failing
                        // immediately. This will prevent adding it
                        // immediately again if user has enabled
                        // automatic message deletion. Without this,
                        // we might waste a lot of traffic constantly
                        // retrying message deletion.
                        return Status::RetryLater;
                    }
                }
            }
            if msg.chat_id.is_trash() || msg.hidden {
                // Messages are stored in trash chat only to keep
                // their server UID and Message-ID. Once message is
                // deleted from the server, database record can be
                // removed as well.
                //
                // Hidden messages are similar to trashed, but are
                // related to some chat. We also delete their
                // database records.
                job_try!(msg.id.delete_from_db(context))
            } else {
                // Remove server UID from the database record.
                //
                // We have either just removed the message from the
                // server, in which case UID is not valid anymore, or
                // we have more refernces to the same server UID, so
                // we remove UID to reduce the number of messages
                // pointing to the corresponding UID. Once the counter
                // reaches zero, we will remove the message.
                job_try!(msg.id.unlink(context));
            }
            Status::Finished(Ok(()))
        } else {
            /* eg. device messages have no Message-ID */
            Status::Finished(Ok(()))
        }
    }

    #[allow(non_snake_case)]
    fn EmptyServer(&mut self, context: &Context) -> Status {
        let imap_inbox = &context.inbox_thread.read().unwrap().imap;
        if self.foreign_id & DC_EMPTY_MVBOX > 0 {
            if let Some(mvbox_folder) = context
                .sql
                .get_raw_config(context, "configured_mvbox_folder")
            {
                imap_inbox.empty_folder(context, &mvbox_folder);
            }
        }
        if self.foreign_id & DC_EMPTY_INBOX > 0 {
            imap_inbox.empty_folder(context, "INBOX");
        }
        Status::Finished(Ok(()))
    }

    #[allow(non_snake_case)]
    fn MarkseenMsgOnImap(&mut self, context: &Context) -> Status {
        let imap_inbox = &context.inbox_thread.read().unwrap().imap;

        let msg = job_try!(Message::load_from_db(context, MsgId::new(self.foreign_id)));

        let folder = msg.server_folder.as_ref().unwrap();
        match imap_inbox.set_seen(context, folder, msg.server_uid) {
            ImapActionResult::RetryLater => Status::RetryLater,
            ImapActionResult::AlreadyDone => Status::Finished(Ok(())),
            ImapActionResult::Success | ImapActionResult::Failed => {
                // XXX the message might just have been moved
                // we want to send out an MDN anyway
                // The job will not be retried so locally
                // there is no risk of double-sending MDNs.
                if msg.param.get_bool(Param::WantsMdn).unwrap_or_default()
                    && context.get_config_bool(Config::MdnsEnabled)
                {
                    if let Err(err) = send_mdn(context, &msg) {
                        warn!(context, "could not send out mdn for {}: {}", msg.id, err);
                        return Status::Finished(Err(err));
                    }
                }
                Status::Finished(Ok(()))
            }
        }
    }
}

/* delete all pending jobs with the given action */
pub fn job_kill_action(context: &Context, action: Action) -> bool {
    sql::execute(
        context,
        &context.sql,
        "DELETE FROM jobs WHERE action=?;",
        params![action],
    )
    .is_ok()
}

/// Remove jobs with specified IDs.
pub fn job_kill_ids(context: &Context, job_ids: &[u32]) -> sql::Result<()> {
    sql::execute(
        context,
        &context.sql,
        format!(
            "DELETE FROM jobs WHERE id IN({})",
            job_ids.iter().map(|_| "?").join(",")
        ),
        job_ids,
    )
}

pub fn perform_inbox_fetch(context: &Context) {
    let use_network = context.get_config_bool(Config::InboxWatch);

    task::block_on(
        context
            .inbox_thread
            .write()
            .unwrap()
            .fetch(context, use_network),
    );
}

pub fn perform_mvbox_fetch(context: &Context) {
    let use_network = context.get_config_bool(Config::MvboxWatch);

    task::block_on(
        context
            .mvbox_thread
            .write()
            .unwrap()
            .fetch(context, use_network),
    );
}

pub fn perform_sentbox_fetch(context: &Context) {
    let use_network = context.get_config_bool(Config::SentboxWatch);

    task::block_on(
        context
            .sentbox_thread
            .write()
            .unwrap()
            .fetch(context, use_network),
    );
}

pub fn perform_inbox_idle(context: &Context) {
    if *context.perform_inbox_jobs_needed.clone().read().unwrap() {
        info!(
            context,
            "INBOX-IDLE will not be started because of waiting jobs."
        );
        return;
    }
    let use_network = context.get_config_bool(Config::InboxWatch);

    context
        .inbox_thread
        .read()
        .unwrap()
        .idle(context, use_network);
}

pub fn perform_mvbox_idle(context: &Context) {
    let use_network = context.get_config_bool(Config::MvboxWatch);

    context
        .mvbox_thread
        .read()
        .unwrap()
        .idle(context, use_network);
}

pub fn perform_sentbox_idle(context: &Context) {
    let use_network = context.get_config_bool(Config::SentboxWatch);

    context
        .sentbox_thread
        .read()
        .unwrap()
        .idle(context, use_network);
}

pub fn interrupt_inbox_idle(context: &Context) {
    info!(context, "interrupt_inbox_idle called");
    // we do not block on trying to obtain the thread lock
    // because we don't know in which state the thread is.
    // If it's currently fetching then we can not get the lock
    // but we flag it for checking jobs so that idle will be skipped.
    match context.inbox_thread.try_read() {
        Ok(inbox_thread) => {
            inbox_thread.interrupt_idle(context);
        }
        Err(err) => {
            *context.perform_inbox_jobs_needed.write().unwrap() = true;
            warn!(context, "could not interrupt idle: {}", err);
        }
    }
}

pub fn interrupt_mvbox_idle(context: &Context) {
    context.mvbox_thread.read().unwrap().interrupt_idle(context);
}

pub fn interrupt_sentbox_idle(context: &Context) {
    context
        .sentbox_thread
        .read()
        .unwrap()
        .interrupt_idle(context);
}

pub fn perform_smtp_jobs(context: &Context) {
    let probe_smtp_network = {
        let &(ref lock, _) = &*context.smtp_state.clone();
        let mut state = lock.lock().unwrap();

        let probe_smtp_network = state.probe_network;
        state.probe_network = false;
        state.perform_jobs_needed = PerformJobsNeeded::Not;

        if state.suspended {
            info!(context, "SMTP-jobs suspended.",);
            return;
        }
        state.doing_jobs = true;
        probe_smtp_network
    };

    info!(context, "SMTP-jobs started...",);
    job_perform(context, Thread::Smtp, probe_smtp_network);
    info!(context, "SMTP-jobs ended.");

    {
        let &(ref lock, _) = &*context.smtp_state.clone();
        let mut state = lock.lock().unwrap();

        state.doing_jobs = false;
    }
}

pub fn perform_smtp_idle(context: &Context) {
    info!(context, "SMTP-idle started...",);
    {
        let &(ref lock, ref cvar) = &*context.smtp_state.clone();
        let mut state = lock.lock().unwrap();

        match state.perform_jobs_needed {
            PerformJobsNeeded::AtOnce => {
                info!(
                    context,
                    "SMTP-idle will not be started because of waiting jobs.",
                );
            }
            PerformJobsNeeded::Not | PerformJobsNeeded::AvoidDos => {
                let dur = get_next_wakeup_time(context, Thread::Smtp);

                loop {
                    let res = cvar.wait_timeout(state, dur).unwrap();
                    state = res.0;

                    if state.idle || res.1.timed_out() {
                        // We received the notification and the value has been updated, we can leave.
                        break;
                    }
                }
                state.idle = false;
            }
        }
    }

    info!(context, "SMTP-idle ended.",);
}

fn get_next_wakeup_time(context: &Context, thread: Thread) -> time::Duration {
    let t: i64 = context
        .sql
        .query_get_value(
            context,
            "SELECT MIN(desired_timestamp) FROM jobs WHERE thread=?;",
            params![thread],
        )
        .unwrap_or_default();

    let mut wakeup_time = time::Duration::new(10 * 60, 0);
    let now = time();
    if t > 0 {
        if t > now {
            wakeup_time = time::Duration::new((t - now) as u64, 0);
        } else {
            wakeup_time = time::Duration::new(0, 0);
        }
    }

    wakeup_time
}

pub fn maybe_network(context: &Context) {
    {
        let &(ref lock, _) = &*context.smtp_state.clone();
        let mut state = lock.lock().unwrap();
        state.probe_network = true;

        *context.probe_imap_network.write().unwrap() = true;
    }

    interrupt_smtp_idle(context);
    interrupt_inbox_idle(context);
    interrupt_mvbox_idle(context);
    interrupt_sentbox_idle(context);
}

pub fn job_action_exists(context: &Context, action: Action) -> bool {
    context
        .sql
        .exists("SELECT id FROM jobs WHERE action=?;", params![action])
        .unwrap_or_default()
}

fn set_delivered(context: &Context, msg_id: MsgId) {
    message::update_msg_state(context, msg_id, MessageState::OutDelivered);
    let chat_id: ChatId = context
        .sql
        .query_get_value(
            context,
            "SELECT chat_id FROM msgs WHERE id=?",
            params![msg_id],
        )
        .unwrap_or_default();
    context.call_cb(Event::MsgDelivered { chat_id, msg_id });
}

/* special case for DC_JOB_SEND_MSG_TO_SMTP */
pub fn job_send_msg(context: &Context, msg_id: MsgId) -> Result<()> {
    let mut msg = Message::load_from_db(context, msg_id)?;
    msg.try_calc_and_set_dimensions(context).ok();

    /* create message */
    let needs_encryption = msg.param.get_bool(Param::GuaranteeE2ee).unwrap_or_default();

    let attach_selfavatar = match chat::shall_attach_selfavatar(context, msg.chat_id) {
        Ok(attach_selfavatar) => attach_selfavatar,
        Err(err) => {
            warn!(context, "job: cannot get selfavatar-state: {}", err);
            false
        }
    };

    let mimefactory = MimeFactory::from_msg(context, &msg, attach_selfavatar)?;

    let mut recipients = mimefactory.recipients();

    let from = context
        .get_config(Config::ConfiguredAddr)
        .unwrap_or_default();
    let lowercase_from = from.to_lowercase();

    // Send BCC to self if it is enabled and we are not going to
    // delete it immediately.
    if context.get_config_bool(Config::BccSelf)
        && context.get_config_delete_server_after() != Some(0)
        && !recipients
            .iter()
            .any(|x| x.to_lowercase() == lowercase_from)
    {
        recipients.push(from);
    }

    if recipients.is_empty() {
        // may happen eg. for groups with only SELF and bcc_self disabled
        info!(
            context,
            "message {} has no recipient, skipping smtp-send", msg_id
        );
        set_delivered(context, msg_id);
        return Ok(());
    }

    let rendered_msg = mimefactory.render().map_err(|err| {
        message::set_msg_failed(context, msg_id, Some(err.to_string()));
        err
    })?;

    if needs_encryption && !rendered_msg.is_encrypted {
        /* unrecoverable */
        message::set_msg_failed(
            context,
            msg_id,
            Some("End-to-end-encryption unavailable unexpectedly."),
        );
        bail!(
            "e2e encryption unavailable {} - {:?}",
            msg_id,
            needs_encryption
        );
    }

    if rendered_msg.is_gossiped {
        chat::set_gossiped_timestamp(context, msg.chat_id, time())?;
    }

    if 0 != rendered_msg.last_added_location_id {
        if let Err(err) = location::set_kml_sent_timestamp(context, msg.chat_id, time()) {
            error!(context, "Failed to set kml sent_timestamp: {:?}", err);
        }
        if !msg.hidden {
            if let Err(err) =
                location::set_msg_location_id(context, msg.id, rendered_msg.last_added_location_id)
            {
                error!(context, "Failed to set msg_location_id: {:?}", err);
            }
        }
    }

    if attach_selfavatar {
        if let Err(err) = msg.chat_id.set_selfavatar_timestamp(context, time()) {
            error!(context, "Failed to set selfavatar timestamp: {:?}", err);
        }
    }

    if rendered_msg.is_encrypted && !needs_encryption {
        msg.param.set_int(Param::GuaranteeE2ee, 1);
        msg.save_param_to_disk(context);
    }

    add_smtp_job(
        context,
        Action::SendMsgToSmtp,
        msg.id,
        recipients,
        &rendered_msg,
    )?;

    Ok(())
}

fn load_imap_deletion_msgid(context: &Context) -> sql::Result<Option<MsgId>> {
    if let Some(delete_server_after) = context.get_config_delete_server_after() {
        let threshold_timestamp = time() - delete_server_after;

        context.sql.query_row_optional(
            "SELECT id FROM msgs \
             WHERE timestamp < ? \
             AND server_uid != 0",
            params![threshold_timestamp],
            |row| row.get::<_, MsgId>(0),
        )
    } else {
        Ok(None)
    }
}

fn load_imap_deletion_job(context: &Context) -> sql::Result<Option<Job>> {
    let res = if let Some(msg_id) = load_imap_deletion_msgid(context)? {
        Some(Job::new(
            Action::DeleteMsgOnImap,
            msg_id.to_u32(),
            Params::new(),
            0,
        ))
    } else {
        None
    };
    Ok(res)
}

pub fn perform_inbox_jobs(context: &Context) {
    info!(context, "dc_perform_inbox_jobs starting.",);

    let probe_imap_network = *context.probe_imap_network.clone().read().unwrap();
    *context.probe_imap_network.write().unwrap() = false;
    *context.perform_inbox_jobs_needed.write().unwrap() = false;

    job_perform(context, Thread::Imap, probe_imap_network);
    info!(context, "dc_perform_inbox_jobs ended.",);
}

pub fn perform_mvbox_jobs(context: &Context) {
    info!(context, "dc_perform_mbox_jobs EMPTY (for now).",);
}

pub fn perform_sentbox_jobs(context: &Context) {
    info!(context, "dc_perform_sentbox_jobs EMPTY (for now).",);
}

fn job_perform(context: &Context, thread: Thread, probe_network: bool) {
    while let Some(mut job) = load_next_job(context, thread, probe_network) {
        info!(context, "{}-job {} started...", thread, job);

        // some configuration jobs are "exclusive":
        // - they are always executed in the imap-thread and the smtp-thread is suspended during execution
        // - they may change the database handle; we do not keep old pointers therefore
        // - they can be re-executed one time AT_ONCE, but they are not saved in the database for later execution
        if Action::ConfigureImap == job.action || Action::ImexImap == job.action {
            job_kill_action(context, job.action);
            context
                .sentbox_thread
                .clone()
                .read()
                .unwrap()
                .suspend(context);
            context
                .mvbox_thread
                .clone()
                .read()
                .unwrap()
                .suspend(context);
            suspend_smtp_thread(context, true);
        }

        let try_res = match perform_job_action(context, &mut job, thread, 0) {
            Status::RetryNow => perform_job_action(context, &mut job, thread, 1),
            x => x,
        };

        if Action::ConfigureImap == job.action || Action::ImexImap == job.action {
            context
                .sentbox_thread
                .clone()
                .read()
                .unwrap()
                .unsuspend(context);
            context
                .mvbox_thread
                .clone()
                .read()
                .unwrap()
                .unsuspend(context);
            suspend_smtp_thread(context, false);
            break;
        }

        match try_res {
            Status::RetryNow | Status::RetryLater => {
                let tries = job.tries + 1;

                if tries < JOB_RETRIES {
                    info!(
                        context,
                        "{} thread increases job {} tries to {}", thread, job, tries
                    );
                    job.tries = tries;
                    let time_offset = get_backoff_time_offset(tries);
                    job.desired_timestamp = time() + time_offset;
                    info!(
                        context,
                        "{}-job #{} not succeeded on try #{}, retry in {} seconds.",
                        thread,
                        job.job_id as u32,
                        tries,
                        time_offset
                    );
                    job.save(context);
                    if thread == Thread::Smtp && tries < JOB_RETRIES - 1 {
                        context
                            .smtp_state
                            .clone()
                            .0
                            .lock()
                            .unwrap()
                            .perform_jobs_needed = PerformJobsNeeded::AvoidDos;
                    }
                } else {
                    info!(
                        context,
                        "{} thread removes job {} as it exhausted {} retries",
                        thread,
                        job,
                        JOB_RETRIES
                    );
                    if job.action == Action::SendMsgToSmtp {
                        message::set_msg_failed(
                            context,
                            MsgId::new(job.foreign_id),
                            job.pending_error.as_ref(),
                        );
                    }
                    job.delete(context);
                }
                if !probe_network {
                    continue;
                }
                // on dc_maybe_network() we stop trying here;
                // these jobs are already tried once.
                // otherwise, we just continue with the next job
                // to give other jobs a chance being tried at least once.
                break;
            }
            Status::Finished(res) => {
                if let Err(err) = res {
                    warn!(
                        context,
                        "{} removes job {} as it failed with error {:?}", thread, job, err
                    );
                } else {
                    info!(context, "{} removes job {} as it succeeded", thread, job);
                }

                job.delete(context);
            }
        }
    }
}

fn perform_job_action(context: &Context, mut job: &mut Job, thread: Thread, tries: u32) -> Status {
    info!(
        context,
        "{} begin immediate try {} of job {}", thread, tries, job
    );

    let try_res = match job.action {
        Action::Unknown => Status::Finished(Err(format_err!("Unknown job id found"))),
        Action::SendMsgToSmtp => job.SendMsgToSmtp(context),
        Action::EmptyServer => job.EmptyServer(context),
        Action::OldDeleteMsgOnImap => job.DeleteMsgOnImap(context),
        Action::DeleteMsgOnImap => job.DeleteMsgOnImap(context),
        Action::MarkseenMsgOnImap => job.MarkseenMsgOnImap(context),
        Action::MoveMsg => job.MoveMsg(context),
        Action::SendMdn => job.SendMdn(context),
        Action::ConfigureImap => JobConfigureImap(context),
        Action::ImexImap => match JobImexImap(context, &job) {
            Ok(()) => Status::Finished(Ok(())),
            Err(err) => {
                error!(context, "{}", err);
                Status::Finished(Err(err))
            }
        },
        Action::MaybeSendLocations => location::JobMaybeSendLocations(context, &job),
        Action::MaybeSendLocationsEnded => location::JobMaybeSendLocationsEnded(context, &mut job),
        Action::Housekeeping => {
            sql::housekeeping(context);
            Status::Finished(Ok(()))
        }
    };

    info!(
        context,
        "{} finished immediate try {} of job {}", thread, tries, job
    );

    try_res
}

fn get_backoff_time_offset(tries: u32) -> i64 {
    let n = 2_i32.pow(tries - 1) * 60;
    let mut rng = thread_rng();
    let r: i32 = rng.gen();
    let mut seconds = r % (n + 1);
    if seconds < 1 {
        seconds = 1;
    }
    seconds as i64
}

fn suspend_smtp_thread(context: &Context, suspend: bool) {
    context.smtp_state.0.lock().unwrap().suspended = suspend;
    if suspend {
        loop {
            if !context.smtp_state.0.lock().unwrap().doing_jobs {
                return;
            }
            std::thread::sleep(time::Duration::from_micros(300 * 1000));
        }
    }
}

fn send_mdn(context: &Context, msg: &Message) -> Result<()> {
    let mut param = Params::new();
    param.set(Param::MsgId, msg.id.to_u32().to_string());

    job_add(context, Action::SendMdn, msg.from_id as i32, param, 0);

    Ok(())
}

fn add_smtp_job(
    context: &Context,
    action: Action,
    msg_id: MsgId,
    recipients: Vec<String>,
    rendered_msg: &RenderedEmail,
) -> Result<()> {
    ensure!(!recipients.is_empty(), "no recipients for smtp job set");
    let mut param = Params::new();
    let bytes = &rendered_msg.message;
    let blob = BlobObject::create(context, &rendered_msg.rfc724_mid, bytes)?;

    let recipients = recipients.join("\x1e");
    param.set(Param::File, blob.as_name());
    param.set(Param::Recipients, &recipients);

    job_add(context, action, msg_id.to_u32() as i32, param, 0);

    Ok(())
}

/// Adds a job to the database, scheduling it `delay_seconds`
/// after the current time.
pub fn job_add(
    context: &Context,
    action: Action,
    foreign_id: i32,
    param: Params,
    delay_seconds: i64,
) {
    if action == Action::Unknown {
        error!(context, "Invalid action passed to job_add");
        return;
    }

    let job = Job::new(action, foreign_id as u32, param, delay_seconds);
    job.save(context);

    if delay_seconds == 0 {
        let thread: Thread = action.into();
        match thread {
            Thread::Imap => interrupt_inbox_idle(context),
            Thread::Smtp => interrupt_smtp_idle(context),
            Thread::Unknown => {}
        }
    }
}

pub fn interrupt_smtp_idle(context: &Context) {
    info!(context, "Interrupting SMTP-idle...",);

    let &(ref lock, ref cvar) = &*context.smtp_state.clone();
    let mut state = lock.lock().unwrap();

    state.perform_jobs_needed = PerformJobsNeeded::AtOnce;
    state.idle = true;
    cvar.notify_one();
    info!(context, "Interrupting SMTP-idle... ended",);
}

/// Load jobs from the database.
///
/// Load jobs for this "[Thread]", i.e. either load SMTP jobs or load
/// IMAP jobs.  The `probe_network` parameter decides how to query
/// jobs, this is tricky and probably wrong currently. Look at the
/// SQL queries for details.
fn load_next_job(context: &Context, thread: Thread, probe_network: bool) -> Option<Job> {
    let query = if !probe_network {
        // processing for first-try and after backoff-timeouts:
        // process jobs in the order they were added.
        "SELECT id, action, foreign_id, param, added_timestamp, desired_timestamp, tries \
         FROM jobs WHERE thread=? AND desired_timestamp<=? ORDER BY action DESC, added_timestamp;"
    } else {
        // processing after call to dc_maybe_network():
        // process _all_ pending jobs that failed before
        // in the order of their backoff-times.
        "SELECT id, action, foreign_id, param, added_timestamp, desired_timestamp, tries \
         FROM jobs WHERE thread=? AND tries>0 ORDER BY desired_timestamp, action DESC;"
    };

    let params_no_probe = params![thread as i64, time()];
    let params_probe = params![thread as i64];
    let params: &[&dyn rusqlite::ToSql] = if !probe_network {
        params_no_probe
    } else {
        params_probe
    };

    let job = context
        .sql
        .query_map(
            query,
            params,
            |row| {
                let job = Job {
                    job_id: row.get(0)?,
                    action: row.get(1)?,
                    foreign_id: row.get(2)?,
                    desired_timestamp: row.get(5)?,
                    added_timestamp: row.get(4)?,
                    tries: row.get(6)?,
                    param: row.get::<_, String>(3)?.parse().unwrap_or_default(),
                    pending_error: None,
                };

                Ok(job)
            },
            |jobs| {
                for job in jobs {
                    match job {
                        Ok(j) => return Ok(Some(j)),
                        Err(e) => warn!(context, "Bad job from the database: {}", e),
                    }
                }
                Ok(None)
            },
        )
        .unwrap_or_default();

    if thread == Thread::Imap {
        if let Some(job) = job {
            if job.action < Action::DeleteMsgOnImap {
                load_imap_deletion_job(context)
                    .unwrap_or_default()
                    .or(Some(job))
            } else {
                Some(job)
            }
        } else {
            load_imap_deletion_job(context).unwrap_or_default()
        }
    } else {
        job
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::test_utils::*;

    fn insert_job(context: &Context, foreign_id: i64) {
        let now = time();
        context
            .sql
            .execute(
                "INSERT INTO jobs
                   (added_timestamp, thread, action, foreign_id, param, desired_timestamp)
                 VALUES (?, ?, ?, ?, ?, ?);",
                params![
                    now,
                    Thread::from(Action::MoveMsg),
                    Action::MoveMsg,
                    foreign_id,
                    Params::new().to_string(),
                    now
                ],
            )
            .unwrap();
    }

    #[test]
    fn test_load_next_job() {
        // We want to ensure that loading jobs skips over jobs which
        // fails to load from the database instead of failing to load
        // all jobs.
        let t = dummy_context();
        insert_job(&t.ctx, -1); // This can not be loaded into Job struct.
        let jobs = load_next_job(&t.ctx, Thread::from(Action::MoveMsg), false);
        assert!(jobs.is_none());

        insert_job(&t.ctx, 1);
        let jobs = load_next_job(&t.ctx, Thread::from(Action::MoveMsg), false);
        assert!(jobs.is_some());
    }
}
