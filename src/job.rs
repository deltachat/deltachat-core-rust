//! # Job module
//!
//! This module implements a job queue maintained in the SQLite database
//! and job types.

use std::future::Future;
use std::{fmt, time};

use async_std::prelude::*;

use deltachat_derive::{FromSql, ToSql};
use itertools::Itertools;
use rand::{thread_rng, Rng};

use crate::blob::BlobObject;
use crate::chat::{self, ChatId};
use crate::config::Config;
use crate::configure::*;
use crate::constants::*;
use crate::contact::Contact;
use crate::context::{Context, PerformJobsNeeded};
use crate::dc_tools::*;
use crate::error::{Error, Result};
use crate::events::Event;
use crate::imap::*;
use crate::imex::*;
use crate::job;
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
            std::result::Result::Ok(val) => val,
            std::result::Result::Err(err) => {
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

#[derive(Debug, Display, Copy, Clone, PartialEq, Eq, FromPrimitive, ToPrimitive, FromSql, ToSql)]
#[repr(i32)]
pub enum Action {
    Unknown = 0,

    // Jobs in the INBOX-thread, range from DC_IMAP_THREAD..DC_IMAP_THREAD+999
    Housekeeping = 105, // low priority ...
    EmptyServer = 107,
    DeleteMsgOnImap = 110,
    MarkseenMdnOnImap = 120,
    MarkseenMsgOnImap = 130,
    MoveMsg = 200,
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
            DeleteMsgOnImap => Thread::Imap,
            EmptyServer => Thread::Imap,
            MarkseenMdnOnImap => Thread::Imap,
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
    /// Deletes the job from the database.
    async fn delete(&self, context: &Context) -> bool {
        context
            .sql
            .execute("DELETE FROM jobs WHERE id=?;", paramsv![self.job_id as i32])
            .await
            .is_ok()
    }

    /// Updates the job already stored in the database.
    ///
    /// To add a new job, use [job_add].
    async fn update(&self, context: &Context) -> bool {
        context
            .sql
            .execute(
                "UPDATE jobs SET desired_timestamp=?, tries=?, param=? WHERE id=?;",
                paramsv![
                    self.desired_timestamp,
                    self.tries as i64,
                    self.param.to_string(),
                    self.job_id as i32,
                ],
            )
            .await
            .is_ok()
    }

    async fn smtp_send<F, Fut>(
        &mut self,
        context: &Context,
        recipients: Vec<async_smtp::EmailAddress>,
        message: Vec<u8>,
        job_id: u32,
        success_cb: F,
    ) -> Status
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<()>>,
    {
        // hold the smtp lock during sending of a job and
        // its ok/error response processing. Note that if a message
        // was sent we need to mark it in the database ASAP as we
        // otherwise might send it twice.
        if std::env::var(crate::DCC_MIME_DEBUG).is_ok() {
            info!(context, "smtp-sending out mime message:");
            println!("{}", String::from_utf8_lossy(&message));
        }
        match context
            .smtp
            .send(context, recipients, message, job_id)
            .await
        {
            Err(crate::smtp::send::Error::SendError(err)) => {
                // Remote error, retry later.
                warn!(context, "SMTP failed to send: {}", err);
                self.pending_error = Some(err.to_string());

                let res = match err {
                    async_smtp::smtp::error::Error::Permanent(_) => {
                        Status::Finished(Err(format_err!("Permanent SMTP error: {}", err)))
                    }
                    async_smtp::smtp::error::Error::Transient(_) => {
                        // We got a transient 4xx response from SMTP server.
                        // Give some time until the server-side error maybe goes away.
                        Status::RetryLater
                    }
                    _ => {
                        if context.smtp.has_maybe_stale_connection().await {
                            info!(context, "stale connection? immediately reconnecting");
                            Status::RetryNow
                        } else {
                            Status::RetryLater
                        }
                    }
                };

                // this clears last_success info
                context.smtp.disconnect().await;

                res
            }
            Err(crate::smtp::send::Error::EnvelopeError(err)) => {
                // Local error, job is invalid, do not retry.
                context.smtp.disconnect().await;
                warn!(context, "SMTP job is invalid: {}", err);
                Status::Finished(Err(Error::SmtpError(err)))
            }
            Err(crate::smtp::send::Error::NoTransport) => {
                // Should never happen.
                // It does not even make sense to disconnect here.
                error!(context, "SMTP job failed because SMTP has no transport");
                Status::Finished(Err(format_err!("SMTP has not transport")))
            }
            Ok(()) => {
                job_try!(success_cb().await);
                Status::Finished(Ok(()))
            }
        }
    }

    async fn send_msg_to_smtp(&mut self, context: &Context) -> Status {
        // connect to SMTP server, if not yet done
        if !context.smtp.is_connected().await {
            let loginparam = LoginParam::from_database(context, "configured_").await;
            if let Err(err) = context.smtp.connect(context, &loginparam).await {
                warn!(context, "SMTP connection failure: {:?}", err);
                return Status::RetryLater;
            }
        }

        let filename = job_try!(job_try!(self
            .param
            .get_path(Param::File, context)
            .map_err(|_| format_err!("Can't get filename")))
        .ok_or_else(|| format_err!("Can't get filename")));
        let body = job_try!(dc_read_file(context, &filename).await);
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
        if 0 != self.foreign_id && !message::exists(context, MsgId::new(self.foreign_id)).await {
            return Status::Finished(Err(format_err!(
                "Not sending Message {} as it was deleted",
                self.foreign_id
            )));
        };

        let foreign_id = self.foreign_id;
        self.smtp_send(context, recipients_list, body, self.job_id, || {
            async move {
                // smtp success, update db ASAP, then delete smtp file
                if 0 != foreign_id {
                    set_delivered(context, MsgId::new(foreign_id)).await;
                }
                // now also delete the generated file
                dc_delete_file(context, filename).await;
                Ok(())
            }
        })
        .await
    }

    /// Get `SendMdn` jobs with foreign_id equal to `contact_id` excluding the `job_id` job.
    async fn get_additional_mdn_jobs(
        &self,
        context: &Context,
        contact_id: u32,
    ) -> sql::Result<(Vec<u32>, Vec<String>)> {
        // Extract message IDs from job parameters
        let res: Vec<(u32, MsgId)> = context
            .sql
            .query_map(
                "SELECT id, param FROM jobs WHERE foreign_id=? AND id!=?",
                paramsv![contact_id, self.job_id],
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
            )
            .await?;

        // Load corresponding RFC724 message IDs
        let mut job_ids = Vec::new();
        let mut rfc724_mids = Vec::new();
        for (job_id, msg_id) in res {
            if let Ok(Message { rfc724_mid, .. }) = Message::load_from_db(context, msg_id).await {
                job_ids.push(job_id);
                rfc724_mids.push(rfc724_mid);
            }
        }
        Ok((job_ids, rfc724_mids))
    }

    async fn send_mdn(&mut self, context: &Context) -> Status {
        if !context.get_config_bool(Config::MdnsEnabled).await {
            // User has disabled MDNs after job scheduling but before
            // execution.
            return Status::Finished(Err(format_err!("MDNs are disabled")));
        }

        let contact_id = self.foreign_id;
        let contact = job_try!(Contact::load_from_db(context, contact_id).await);
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
            .await
            .unwrap_or_default();

        if !additional_rfc724_mids.is_empty() {
            info!(
                context,
                "SendMdn job: aggregating {} additional MDNs",
                additional_rfc724_mids.len()
            )
        }

        let msg = job_try!(Message::load_from_db(context, msg_id).await);
        let mimefactory =
            job_try!(MimeFactory::from_mdn(context, &msg, additional_rfc724_mids).await);
        let rendered_msg = job_try!(mimefactory.render().await);
        let body = rendered_msg.message;

        let addr = contact.get_addr();
        let recipient = job_try!(async_smtp::EmailAddress::new(addr.to_string())
            .map_err(|err| format_err!("invalid recipient: {} {:?}", addr, err)));
        let recipients = vec![recipient];

        // connect to SMTP server, if not yet done
        if !context.smtp.is_connected().await {
            let loginparam = LoginParam::from_database(context, "configured_").await;
            if let Err(err) = context.smtp.connect(context, &loginparam).await {
                warn!(context, "SMTP connection failure: {:?}", err);
                return Status::RetryLater;
            }
        }

        self.smtp_send(context, recipients, body, self.job_id, || {
            async move {
                // Remove additional SendMdn jobs we have aggregated into this one.
                job::kill_ids(context, &additional_job_ids).await?;
                Ok(())
            }
        })
        .await
    }

    async fn move_msg(&mut self, context: &Context) -> Status {
        let imap_inbox = &context.inbox_thread.imap;

        let msg = job_try!(Message::load_from_db(context, MsgId::new(self.foreign_id)).await);

        if let Err(err) = imap_inbox.ensure_configured_folders(context, true).await {
            warn!(context, "could not configure folders: {:?}", err);
            return Status::RetryLater;
        }
        let dest_folder = context
            .sql
            .get_raw_config(context, "configured_mvbox_folder")
            .await;

        if let Some(dest_folder) = dest_folder {
            let server_folder = msg.server_folder.as_ref().unwrap();
            let mut dest_uid = 0;

            match imap_inbox
                .mv(
                    context,
                    server_folder,
                    msg.server_uid,
                    &dest_folder,
                    &mut dest_uid,
                )
                .await
            {
                ImapActionResult::RetryLater => Status::RetryLater,
                ImapActionResult::Success => {
                    message::update_server_uid(context, &msg.rfc724_mid, &dest_folder, dest_uid)
                        .await;
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

    async fn delete_msg_on_imap(&mut self, context: &Context) -> Status {
        let imap_inbox = &context.inbox_thread.imap;

        let mut msg = job_try!(Message::load_from_db(context, MsgId::new(self.foreign_id)).await);

        if !msg.rfc724_mid.is_empty() {
            if message::rfc724_mid_cnt(context, &msg.rfc724_mid).await > 1 {
                info!(
                    context,
                    "The message is deleted from the server when all parts are deleted.",
                );
            } else {
                /* if this is the last existing part of the message,
                we delete the message from the server */
                let mid = msg.rfc724_mid;
                let server_folder = msg.server_folder.as_ref().unwrap();
                let res = imap_inbox
                    .delete_msg(context, &mid, server_folder, &mut msg.server_uid)
                    .await;
                if res == ImapActionResult::RetryLater {
                    // XXX RetryLater is converted to RetryNow here
                    return Status::RetryNow;
                }
            }
            Message::delete_from_db(context, msg.id).await;
            Status::Finished(Ok(()))
        } else {
            /* eg. device messages have no Message-ID */
            Status::Finished(Ok(()))
        }
    }

    async fn empty_server(&mut self, context: &Context) -> Status {
        let imap_inbox = &context.inbox_thread.imap;
        if self.foreign_id & DC_EMPTY_MVBOX > 0 {
            if let Some(mvbox_folder) = context
                .sql
                .get_raw_config(context, "configured_mvbox_folder")
                .await
            {
                imap_inbox.empty_folder(context, &mvbox_folder).await;
            }
        }
        if self.foreign_id & DC_EMPTY_INBOX > 0 {
            imap_inbox.empty_folder(context, "INBOX").await;
        }
        Status::Finished(Ok(()))
    }

    async fn markseen_msg_on_imap(&mut self, context: &Context) -> Status {
        let imap_inbox = &context.inbox_thread.imap;

        let msg = job_try!(Message::load_from_db(context, MsgId::new(self.foreign_id)).await);

        let folder = msg.server_folder.as_ref().unwrap();
        match imap_inbox.set_seen(context, folder, msg.server_uid).await {
            ImapActionResult::RetryLater => Status::RetryLater,
            ImapActionResult::AlreadyDone => Status::Finished(Ok(())),
            ImapActionResult::Success | ImapActionResult::Failed => {
                // XXX the message might just have been moved
                // we want to send out an MDN anyway
                // The job will not be retried so locally
                // there is no risk of double-sending MDNs.
                if msg.param.get_bool(Param::WantsMdn).unwrap_or_default()
                    && context.get_config_bool(Config::MdnsEnabled).await
                {
                    if let Err(err) = send_mdn(context, &msg).await {
                        warn!(context, "could not send out mdn for {}: {}", msg.id, err);
                        return Status::Finished(Err(err));
                    }
                }
                Status::Finished(Ok(()))
            }
        }
    }

    async fn markseen_mdn_on_imap(&mut self, context: &Context) -> Status {
        let folder = self
            .param
            .get(Param::ServerFolder)
            .unwrap_or_default()
            .to_string();
        let uid = self.param.get_int(Param::ServerUid).unwrap_or_default() as u32;
        let imap_inbox = &context.inbox_thread.imap;
        if imap_inbox.set_seen(context, &folder, uid).await == ImapActionResult::RetryLater {
            return Status::RetryLater;
        }
        if self.param.get_bool(Param::AlsoMove).unwrap_or_default() {
            if let Err(err) = imap_inbox.ensure_configured_folders(context, true).await {
                warn!(context, "configuring folders failed: {:?}", err);
                return Status::RetryLater;
            }
            let dest_folder = context
                .sql
                .get_raw_config(context, "configured_mvbox_folder")
                .await;
            if let Some(dest_folder) = dest_folder {
                let mut dest_uid = 0;
                if ImapActionResult::RetryLater
                    == imap_inbox
                        .mv(context, &folder, uid, &dest_folder, &mut dest_uid)
                        .await
                {
                    Status::RetryLater
                } else {
                    Status::Finished(Ok(()))
                }
            } else {
                Status::Finished(Err(format_err!("MVBOX is not configured")))
            }
        } else {
            Status::Finished(Ok(()))
        }
    }
}

/// Delete all pending jobs with the given action.
pub async fn kill_action(context: &Context, action: Action) -> bool {
    context
        .sql
        .execute("DELETE FROM jobs WHERE action=?;", paramsv![action])
        .await
        .is_ok()
}

/// Remove jobs with specified IDs.
pub async fn kill_ids(context: &Context, job_ids: &[u32]) -> sql::Result<()> {
    context
        .sql
        .execute(
            format!(
                "DELETE FROM jobs WHERE id IN({})",
                job_ids.iter().map(|_| "?").join(",")
            ),
            job_ids.iter().map(|i| i as &dyn crate::ToSql).collect(),
        )
        .await?;
    Ok(())
}

pub async fn perform_inbox_fetch(context: &Context) {
    let use_network = context.get_config_bool(Config::InboxWatch).await;

    context.inbox_thread.fetch(context, use_network).await;
}

pub async fn perform_mvbox_fetch(context: &Context) {
    let use_network = context.get_config_bool(Config::MvboxWatch).await;

    context.mvbox_thread.fetch(context, use_network).await;
}

pub async fn perform_sentbox_fetch(context: &Context) {
    let use_network = context.get_config_bool(Config::SentboxWatch).await;

    context.sentbox_thread.fetch(context, use_network).await;
}

pub async fn perform_inbox_idle(context: &Context) {
    if context
        .perform_inbox_jobs_needed
        .load(std::sync::atomic::Ordering::Relaxed)
    {
        info!(
            context,
            "INBOX-IDLE will not be started because of waiting jobs."
        );
        return;
    }
    let use_network = context.get_config_bool(Config::InboxWatch).await;

    context.inbox_thread.idle(context, use_network).await;
}

pub async fn perform_mvbox_idle(context: &Context) {
    let use_network = context.get_config_bool(Config::MvboxWatch).await;

    context.mvbox_thread.idle(context, use_network).await;
}

pub async fn perform_sentbox_idle(context: &Context) {
    let use_network = context.get_config_bool(Config::SentboxWatch).await;

    context.sentbox_thread.idle(context, use_network).await;
}

pub async fn interrupt_inbox_idle(context: &Context) {
    info!(context, "interrupt_inbox_idle called");
    // we do not block on trying to obtain the thread lock
    // because we don't know in which state the thread is.
    // If it's currently fetching then we can not get the lock
    // but we flag it for checking jobs so that idle will be skipped.
    if !context.inbox_thread.try_interrupt_idle(context).await {
        context
            .perform_inbox_jobs_needed
            .store(true, std::sync::atomic::Ordering::Relaxed);
        warn!(context, "could not interrupt idle");
    }
}

pub async fn interrupt_mvbox_idle(context: &Context) {
    context.mvbox_thread.interrupt_idle(context).await;
}

pub async fn interrupt_sentbox_idle(context: &Context) {
    context.sentbox_thread.interrupt_idle(context).await;
}

pub async fn perform_smtp_jobs(context: &Context) {
    let probe_smtp_network = {
        let state = &mut *context.smtp.state.write().await;

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
    job_perform(context, Thread::Smtp, probe_smtp_network).await;
    info!(context, "SMTP-jobs ended.");

    context.smtp.state.write().await.doing_jobs = false;
}

pub async fn perform_smtp_idle(context: &Context) {
    info!(context, "SMTP-idle started...");

    let perform_jobs_needed = context.smtp.state.read().await.perform_jobs_needed.clone();

    match perform_jobs_needed {
        PerformJobsNeeded::AtOnce => {
            info!(
                context,
                "SMTP-idle will not be started because of waiting jobs.",
            );
        }
        PerformJobsNeeded::Not | PerformJobsNeeded::AvoidDos => {
            let dur = get_next_wakeup_time(context, Thread::Smtp).await;

            context.smtp.notify_receiver.recv().timeout(dur).await.ok();
        }
    }

    info!(context, "SMTP-idle ended.",);
}

async fn get_next_wakeup_time(context: &Context, thread: Thread) -> time::Duration {
    let t: i64 = context
        .sql
        .query_get_value(
            context,
            "SELECT MIN(desired_timestamp) FROM jobs WHERE thread=?;",
            paramsv![thread],
        )
        .await
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

pub async fn maybe_network(context: &Context) {
    {
        context.smtp.state.write().await.probe_network = true;
        context
            .probe_imap_network
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }

    interrupt_smtp_idle(context).await;
    interrupt_inbox_idle(context).await;
    interrupt_mvbox_idle(context).await;
    interrupt_sentbox_idle(context).await;
}

pub async fn action_exists(context: &Context, action: Action) -> bool {
    context
        .sql
        .exists("SELECT id FROM jobs WHERE action=?;", paramsv![action])
        .await
        .unwrap_or_default()
}

async fn set_delivered(context: &Context, msg_id: MsgId) {
    message::update_msg_state(context, msg_id, MessageState::OutDelivered).await;
    let chat_id: ChatId = context
        .sql
        .query_get_value(
            context,
            "SELECT chat_id FROM msgs WHERE id=?",
            paramsv![msg_id],
        )
        .await
        .unwrap_or_default();
    context.call_cb(Event::MsgDelivered { chat_id, msg_id });
}

// special case for DC_JOB_SEND_MSG_TO_SMTP
pub async fn send_msg(context: &Context, msg_id: MsgId) -> Result<()> {
    let mut msg = Message::load_from_db(context, msg_id).await?;
    msg.try_calc_and_set_dimensions(context).await.ok();

    /* create message */
    let needs_encryption = msg.param.get_bool(Param::GuaranteeE2ee).unwrap_or_default();

    let attach_selfavatar = match chat::shall_attach_selfavatar(context, msg.chat_id).await {
        Ok(attach_selfavatar) => attach_selfavatar,
        Err(err) => {
            warn!(context, "job: cannot get selfavatar-state: {}", err);
            false
        }
    };

    let mimefactory = MimeFactory::from_msg(context, &msg, attach_selfavatar).await?;

    let mut recipients = mimefactory.recipients();

    let from = context
        .get_config(Config::ConfiguredAddr)
        .await
        .unwrap_or_default();
    let lowercase_from = from.to_lowercase();
    if context.get_config_bool(Config::BccSelf).await
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
        set_delivered(context, msg_id).await;
        return Ok(());
    }

    let rendered_msg = match mimefactory.render().await {
        Ok(res) => Ok(res),
        Err(err) => {
            message::set_msg_failed(context, msg_id, Some(err.to_string())).await;
            Err(err)
        }
    }?;

    if needs_encryption && !rendered_msg.is_encrypted {
        /* unrecoverable */
        message::set_msg_failed(
            context,
            msg_id,
            Some("End-to-end-encryption unavailable unexpectedly."),
        )
        .await;
        bail!(
            "e2e encryption unavailable {} - {:?}",
            msg_id,
            needs_encryption
        );
    }

    if rendered_msg.is_gossiped {
        chat::set_gossiped_timestamp(context, msg.chat_id, time()).await?;
    }

    if 0 != rendered_msg.last_added_location_id {
        if let Err(err) = location::set_kml_sent_timestamp(context, msg.chat_id, time()).await {
            error!(context, "Failed to set kml sent_timestamp: {:?}", err);
        }
        if !msg.hidden {
            if let Err(err) =
                location::set_msg_location_id(context, msg.id, rendered_msg.last_added_location_id)
                    .await
            {
                error!(context, "Failed to set msg_location_id: {:?}", err);
            }
        }
    }

    if attach_selfavatar {
        if let Err(err) = msg.chat_id.set_selfavatar_timestamp(context, time()).await {
            error!(context, "Failed to set selfavatar timestamp: {:?}", err);
        }
    }

    if rendered_msg.is_encrypted && !needs_encryption {
        msg.param.set_int(Param::GuaranteeE2ee, 1);
        msg.save_param_to_disk(context).await;
    }

    add_smtp_job(
        context,
        Action::SendMsgToSmtp,
        msg.id,
        recipients,
        &rendered_msg,
    )
    .await?;

    Ok(())
}

pub async fn perform_inbox_jobs(context: &Context) {
    info!(context, "dc_perform_inbox_jobs starting.",);

    let probe_imap_network = context
        .probe_imap_network
        .load(std::sync::atomic::Ordering::Relaxed);
    context
        .probe_imap_network
        .store(false, std::sync::atomic::Ordering::Relaxed);
    context
        .perform_inbox_jobs_needed
        .store(false, std::sync::atomic::Ordering::Relaxed);

    job_perform(context, Thread::Imap, probe_imap_network).await;
    info!(context, "dc_perform_inbox_jobs ended.",);
}

pub async fn perform_mvbox_jobs(context: &Context) {
    info!(context, "dc_perform_mbox_jobs EMPTY (for now).");
}

pub async fn perform_sentbox_jobs(context: &Context) {
    info!(context, "dc_perform_sentbox_jobs EMPTY (for now).");
}

async fn job_perform(context: &Context, thread: Thread, probe_network: bool) {
    while let Some(mut job) = load_next_job(context, thread, probe_network).await {
        info!(context, "{}-job {} started...", thread, job);

        // some configuration jobs are "exclusive":
        // - they are always executed in the imap-thread and the smtp-thread is suspended during execution
        // - they may change the database handle; we do not keep old pointers therefore
        // - they can be re-executed one time AT_ONCE, but they are not saved in the database for later execution
        if Action::ConfigureImap == job.action || Action::ImexImap == job.action {
            job::kill_action(context, job.action).await;
            context.sentbox_thread.suspend(context).await;
            context.mvbox_thread.suspend(context).await;
            suspend_smtp_thread(context, true).await;
        }

        let try_res = match perform_job_action(context, &mut job, thread, 0).await {
            Status::RetryNow => perform_job_action(context, &mut job, thread, 1).await,
            x => x,
        };

        if Action::ConfigureImap == job.action || Action::ImexImap == job.action {
            context.sentbox_thread.unsuspend(context).await;
            context.mvbox_thread.unsuspend(context).await;
            suspend_smtp_thread(context, false).await;
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
                    job.update(context).await;
                    info!(
                        context,
                        "{}-job #{} not succeeded on try #{}, retry in {} seconds.",
                        thread,
                        job.job_id as u32,
                        tries,
                        time_offset
                    );
                    if thread == Thread::Smtp && tries < JOB_RETRIES - 1 {
                        context.smtp.state.write().await.perform_jobs_needed =
                            PerformJobsNeeded::AvoidDos;
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
                        )
                        .await;
                    }
                    job.delete(context).await;
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

                job.delete(context).await;
            }
        }
    }
}

async fn perform_job_action(
    context: &Context,
    mut job: &mut Job,
    thread: Thread,
    tries: u32,
) -> Status {
    info!(
        context,
        "{} begin immediate try {} of job {}", thread, tries, job
    );

    let try_res = match job.action {
        Action::Unknown => Status::Finished(Err(format_err!("Unknown job id found"))),
        Action::SendMsgToSmtp => job.send_msg_to_smtp(context).await,
        Action::EmptyServer => job.empty_server(context).await,
        Action::DeleteMsgOnImap => job.delete_msg_on_imap(context).await,
        Action::MarkseenMsgOnImap => job.markseen_msg_on_imap(context).await,
        Action::MarkseenMdnOnImap => job.markseen_mdn_on_imap(context).await,
        Action::MoveMsg => job.move_msg(context).await,
        Action::SendMdn => job.send_mdn(context).await,
        Action::ConfigureImap => job_configure_imap(context).await,
        Action::ImexImap => match job_imex_imap(context, &job).await {
            Ok(()) => Status::Finished(Ok(())),
            Err(err) => {
                error!(context, "{}", err);
                Status::Finished(Err(err))
            }
        },
        Action::MaybeSendLocations => location::job_maybe_send_locations(context, &job).await,
        Action::MaybeSendLocationsEnded => {
            location::job_maybe_send_locations_ended(context, &mut job).await
        }
        Action::Housekeeping => {
            sql::housekeeping(context).await;
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

async fn suspend_smtp_thread(context: &Context, suspend: bool) {
    context.smtp.state.write().await.suspended = suspend;
    if suspend {
        loop {
            if !context.smtp.state.read().await.doing_jobs {
                return;
            }
            async_std::task::sleep(time::Duration::from_micros(300 * 1000)).await;
        }
    }
}

async fn send_mdn(context: &Context, msg: &Message) -> Result<()> {
    let mut param = Params::new();
    param.set(Param::MsgId, msg.id.to_u32().to_string());

    job::add(context, Action::SendMdn, msg.from_id as i32, param, 0).await;

    Ok(())
}

async fn add_smtp_job(
    context: &Context,
    action: Action,
    msg_id: MsgId,
    recipients: Vec<String>,
    rendered_msg: &RenderedEmail,
) -> Result<()> {
    ensure!(!recipients.is_empty(), "no recipients for smtp job set");
    let mut param = Params::new();
    let bytes = &rendered_msg.message;
    let blob = BlobObject::create(context, &rendered_msg.rfc724_mid, bytes).await?;

    let recipients = recipients.join("\x1e");
    param.set(Param::File, blob.as_name());
    param.set(Param::Recipients, &recipients);

    add(context, action, msg_id.to_u32() as i32, param, 0).await;

    Ok(())
}

/// Adds a job to the database, scheduling it `delay_seconds` after the current time.
pub async fn add(
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

    let timestamp = time();
    let thread: Thread = action.into();

    context.sql.execute(
        "INSERT INTO jobs (added_timestamp, thread, action, foreign_id, param, desired_timestamp) VALUES (?,?,?,?,?,?);",
        paramsv![
            timestamp,
            thread,
            action,
            foreign_id,
            param.to_string(),
            (timestamp + delay_seconds as i64)
        ]
    ).await.ok();

    match thread {
        Thread::Imap => interrupt_inbox_idle(context).await,
        Thread::Smtp => interrupt_smtp_idle(context).await,
        Thread::Unknown => {}
    }
}

pub async fn interrupt_smtp_idle(context: &Context) {
    info!(context, "Interrupting SMTP-idle...",);

    context.smtp.state.write().await.perform_jobs_needed = PerformJobsNeeded::AtOnce;
    context.smtp.notify_sender.send(()).await;

    info!(context, "Interrupting SMTP-idle... ended",);
}

/// Load jobs from the database.
///
/// Load jobs for this "[Thread]", i.e. either load SMTP jobs or load
/// IMAP jobs.  The `probe_network` parameter decides how to query
/// jobs, this is tricky and probably wrong currently. Look at the
/// SQL queries for details.
async fn load_next_job(context: &Context, thread: Thread, probe_network: bool) -> Option<Job> {
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

    let thread_i = thread as i64;
    let t = time();
    let params_no_probe = paramsv![thread_i, t];
    let params_probe = paramsv![thread_i];
    let params = if !probe_network {
        params_no_probe
    } else {
        params_probe
    };

    context
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
        .await
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::test_utils::*;

    async fn insert_job(context: &Context, foreign_id: i64) {
        let now = time();
        context
            .sql
            .execute(
                "INSERT INTO jobs
                   (added_timestamp, thread, action, foreign_id, param, desired_timestamp)
                 VALUES (?, ?, ?, ?, ?, ?);",
                paramsv![
                    now,
                    Thread::from(Action::MoveMsg),
                    Action::MoveMsg,
                    foreign_id,
                    Params::new().to_string(),
                    now
                ],
            )
            .await
            .unwrap();
    }

    #[async_std::test]
    async fn test_load_next_job() {
        // We want to ensure that loading jobs skips over jobs which
        // fails to load from the database instead of failing to load
        // all jobs.
        let t = dummy_context().await;
        insert_job(&t.ctx, -1).await; // This can not be loaded into Job struct.
        let jobs = load_next_job(&t.ctx, Thread::from(Action::MoveMsg), false).await;
        assert!(jobs.is_none());

        insert_job(&t.ctx, 1).await;
        let jobs = load_next_job(&t.ctx, Thread::from(Action::MoveMsg), false).await;
        assert!(jobs.is_some());
    }
}
