//! # Job module
//!
//! This module implements a job queue maintained in the SQLite database
//! and job types.

use std::fmt;
use std::future::Future;

use async_smtp::smtp::response::Category;
use async_smtp::smtp::response::Code;
use async_smtp::smtp::response::Detail;
use async_std::prelude::*;
use deltachat_derive::*;
use itertools::Itertools;
use rand::{thread_rng, Rng};

use crate::blob::BlobObject;
use crate::chat::{self, ChatId};
use crate::config::Config;
use crate::constants::*;
use crate::contact::Contact;
use crate::context::Context;
use crate::dc_tools::*;
use crate::error::{bail, ensure, format_err, Error, Result};
use crate::events::Event;
use crate::imap::*;
use crate::location;
use crate::login_param::LoginParam;
use crate::message::MsgId;
use crate::message::{self, Message, MessageState};
use crate::mimefactory::MimeFactory;
use crate::param::*;
use crate::smtp::Smtp;
use crate::{scheduler::InterruptInfo, sql};

// results in ~3 weeks for the last backoff timespan
const JOB_RETRIES: u32 = 17;

/// Thread IDs
#[derive(Debug, Display, Copy, Clone, PartialEq, Eq, FromPrimitive, ToPrimitive, Sqlx)]
#[repr(i32)]
pub(crate) enum Thread {
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

#[derive(
    Debug, Display, Copy, Clone, PartialEq, Eq, PartialOrd, FromPrimitive, ToPrimitive, Sqlx,
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

impl<'a> sqlx::FromRow<'a, sqlx::sqlite::SqliteRow> for Job {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;
        let foreign_id: i64 = row.try_get("foreign_id")?;
        if foreign_id < 0 {
            return Err(sqlx::Error::Decode(
                anyhow::anyhow!("invalid foreign_id").into(),
            ));
        }

        Ok(Job {
            job_id: row.try_get::<i64, _>("id")? as u32,
            action: row.try_get("action")?,
            foreign_id: foreign_id as u32,
            desired_timestamp: row.try_get("desired_timestamp")?,
            added_timestamp: row.try_get("added_timestamp")?,
            tries: row.try_get::<i64, _>("tries")? as u32,
            param: row
                .try_get::<String, _>("param")?
                .parse()
                .unwrap_or_default(),
            pending_error: None,
        })
    }
}

impl Job {
    pub fn new(action: Action, foreign_id: u32, param: Params, delay_seconds: i64) -> Self {
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

    pub fn delay_seconds(&self) -> i64 {
        self.desired_timestamp - self.added_timestamp
    }

    /// Deletes the job from the database.
    async fn delete(self, context: &Context) -> Result<()> {
        if self.job_id != 0 {
            context
                .sql
                .execute("DELETE FROM jobs WHERE id=?;", paramsx![self.job_id as i32])
                .await?;
        }

        Ok(())
    }

    /// Saves the job to the database, creating a new entry if necessary.
    ///
    /// The Job is consumed by this method.
    pub(crate) async fn save(self, context: &Context) -> Result<()> {
        let thread: Thread = self.action.into();

        info!(context, "saving job for {}-thread: {:?}", thread, self);

        if self.job_id != 0 {
            context
                .sql
                .execute(
                    "UPDATE jobs SET desired_timestamp=?, tries=?, param=? WHERE id=?;",
                    paramsx![
                        self.desired_timestamp,
                        self.tries as i64,
                        self.param.to_string(),
                        self.job_id as i32
                    ],
                )
                .await?;
        } else {
            context.sql.execute(
                "INSERT INTO jobs (added_timestamp, thread, action, foreign_id, param, desired_timestamp) VALUES (?,?,?,?,?,?);",
                paramsx![
                    self.added_timestamp,
                    thread,
                    self.action,
                    self.foreign_id as i32,
                    self.param.to_string(),
                    self.desired_timestamp
                ]
            ).await?;
        }

        Ok(())
    }

    async fn smtp_send<F, Fut>(
        &mut self,
        context: &Context,
        recipients: Vec<async_smtp::EmailAddress>,
        message: Vec<u8>,
        job_id: u32,
        smtp: &mut Smtp,
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
        match smtp.send(context, recipients, message, job_id).await {
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
                                match Message::load_from_db(context, MsgId::new(self.foreign_id))
                                    .await
                                {
                                    Ok(message) => {
                                        chat::add_info_msg(
                                            context,
                                            message.chat_id,
                                            err.to_string(),
                                        )
                                        .await
                                    }
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
                        if smtp.has_maybe_stale_connection().await {
                            info!(context, "stale connection? immediately reconnecting");
                            Status::RetryNow
                        } else {
                            Status::RetryLater
                        }
                    }
                };

                // this clears last_success info
                smtp.disconnect().await;

                res
            }
            Err(crate::smtp::send::Error::EnvelopeError(err)) => {
                // Local error, job is invalid, do not retry.
                smtp.disconnect().await;
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
                job_try!(success_cb().await);
                Status::Finished(Ok(()))
            }
        }
    }

    pub(crate) async fn send_msg_to_smtp(&mut self, context: &Context, smtp: &mut Smtp) -> Status {
        //  SMTP server, if not yet done
        if !smtp.is_connected().await {
            let loginparam = LoginParam::from_database(context, "configured_").await;
            if let Err(err) = smtp.connect(context, &loginparam).await {
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
        self.smtp_send(context, recipients_list, body, self.job_id, smtp, || {
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
        let mut job_ids = Vec::new();
        let mut rfc724_mids = Vec::new();

        let pool = context.sql.get_pool().await?;

        let mut rows = sqlx::query_as("SELECT id, param FROM jobs WHERE foreign_id=? AND id!=?")
            .bind(contact_id as i64)
            .bind(self.job_id as i64)
            .fetch(&pool);

        while let Some(row) = rows.next().await {
            let (job_id, params): (i64, String) = row?;
            let params: Params = params.parse().unwrap_or_default();
            let msg_id = params.get_msg_id().unwrap_or_default();

            match Message::load_from_db(context, msg_id).await {
                Ok(Message { rfc724_mid, .. }) => {
                    job_ids.push(job_id as u32);
                    rfc724_mids.push(rfc724_mid);
                }
                Err(err) => {
                    warn!(context, "failed to load mdn job message: {}", err);
                }
            }
        }

        Ok((job_ids, rfc724_mids))
    }

    async fn send_mdn(&mut self, context: &Context, smtp: &mut Smtp) -> Status {
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
        if !smtp.is_connected().await {
            let loginparam = LoginParam::from_database(context, "configured_").await;
            if let Err(err) = smtp.connect(context, &loginparam).await {
                warn!(context, "SMTP connection failure: {:?}", err);
                return Status::RetryLater;
            }
        }

        self.smtp_send(context, recipients, body, self.job_id, smtp, || {
            async move {
                // Remove additional SendMdn jobs we have aggregated into this one.
                kill_ids(context, &additional_job_ids).await?;
                Ok(())
            }
        })
        .await
    }

    async fn move_msg(&mut self, context: &Context, imap: &mut Imap) -> Status {
        if let Err(err) = imap.connect_configured(context).await {
            warn!(context, "could not connect: {:?}", err);
            return Status::RetryLater;
        }

        let msg = job_try!(Message::load_from_db(context, MsgId::new(self.foreign_id)).await);
        let dest_folder = context.get_config(Config::ConfiguredMvboxFolder).await;

        if let Some(dest_folder) = dest_folder {
            let server_folder = msg.server_folder.as_ref().unwrap();

            match imap
                .mv(context, server_folder, msg.server_uid, &dest_folder)
                .await
            {
                ImapActionResult::RetryLater => Status::RetryLater,
                ImapActionResult::Success => {
                    // Rust-Imap provides no target uid on mv, so just set it to 0, update again when precheck_imf() is called for the moved message
                    message::update_server_uid(context, &msg.rfc724_mid, &dest_folder, 0).await;
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
    async fn delete_msg_on_imap(&mut self, context: &Context, imap: &mut Imap) -> Status {
        if let Err(err) = imap.connect_configured(context).await {
            warn!(context, "could not connect: {:?}", err);
            return Status::RetryLater;
        }

        let msg = job_try!(Message::load_from_db(context, MsgId::new(self.foreign_id)).await);

        if !msg.rfc724_mid.is_empty() {
            let cnt = message::rfc724_mid_cnt(context, &msg.rfc724_mid).await;
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
                    imap.delete_msg(context, &mid, server_folder, msg.server_uid)
                        .await
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
                job_try!(msg.id.delete_from_db(context).await)
            } else {
                // Remove server UID from the database record.
                //
                // We have either just removed the message from the
                // server, in which case UID is not valid anymore, or
                // we have more refernces to the same server UID, so
                // we remove UID to reduce the number of messages
                // pointing to the corresponding UID. Once the counter
                // reaches zero, we will remove the message.
                job_try!(msg.id.unlink(context).await);
            }
            Status::Finished(Ok(()))
        } else {
            /* eg. device messages have no Message-ID */
            Status::Finished(Ok(()))
        }
    }

    async fn empty_server(&mut self, context: &Context, imap: &mut Imap) -> Status {
        if let Err(err) = imap.connect_configured(context).await {
            warn!(context, "could not connect: {:?}", err);
            return Status::RetryLater;
        }

        if self.foreign_id & DC_EMPTY_MVBOX > 0 {
            if let Some(mvbox_folder) = &context.get_config(Config::ConfiguredMvboxFolder).await {
                imap.empty_folder(context, &mvbox_folder).await;
            }
        }
        if self.foreign_id & DC_EMPTY_INBOX > 0 {
            imap.empty_folder(context, "INBOX").await;
        }
        Status::Finished(Ok(()))
    }

    async fn markseen_msg_on_imap(&mut self, context: &Context, imap: &mut Imap) -> Status {
        if let Err(err) = imap.connect_configured(context).await {
            warn!(context, "could not connect: {:?}", err);
            return Status::RetryLater;
        }

        let msg = job_try!(Message::load_from_db(context, MsgId::new(self.foreign_id)).await);

        let folder = msg.server_folder.as_ref().unwrap();
        match imap.set_seen(context, folder, msg.server_uid).await {
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
}

/// Delete all pending jobs with the given action.
pub async fn kill_action(context: &Context, action: Action) -> bool {
    context
        .sql
        .execute("DELETE FROM jobs WHERE action=?;", paramsx![action])
        .await
        .is_ok()
}

/// Remove jobs with specified IDs.
async fn kill_ids(context: &Context, job_ids: &[u32]) -> sql::Result<()> {
    use sqlx::Arguments;

    let mut args = sqlx::sqlite::SqliteArguments::default();
    for job_id in job_ids {
        args.add(*job_id as i32);
    }
    context
        .sql
        .execute(
            &format!(
                "DELETE FROM jobs WHERE id IN({})",
                job_ids.iter().map(|_| "?").join(",")
            ),
            args,
        )
        .await?;
    Ok(())
}

pub async fn action_exists(context: &Context, action: Action) -> bool {
    context
        .sql
        .exists("SELECT id FROM jobs WHERE action=?;", paramsx![action])
        .await
        .unwrap_or_default()
}

async fn set_delivered(context: &Context, msg_id: MsgId) {
    message::update_msg_state(context, msg_id, MessageState::OutDelivered).await;
    let chat_id: ChatId = context
        .sql
        .query_value("SELECT chat_id FROM msgs WHERE id=?", paramsx![])
        .await
        .unwrap_or_default();
    context.emit_event(Event::MsgDelivered { chat_id, msg_id });
}

/// Constructs a job for sending a message.
///
/// Returns `None` if no messages need to be sent out.
///
/// In order to be processed, must be `add`ded.
pub async fn send_msg_job(context: &Context, msg_id: MsgId) -> Result<Option<Job>> {
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

    // Send BCC to self if it is enabled and we are not going to
    // delete it immediately.
    if context.get_config_bool(Config::BccSelf).await
        && context.get_config_delete_server_after().await != Some(0)
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
        return Ok(None);
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
        msg.update_param(context).await;
    }

    ensure!(!recipients.is_empty(), "no recipients for smtp job set");
    let mut param = Params::new();
    let bytes = &rendered_msg.message;
    let blob = BlobObject::create(context, &rendered_msg.rfc724_mid, bytes).await?;

    let recipients = recipients.join("\x1e");
    param.set(Param::File, blob.as_name());
    param.set(Param::Recipients, &recipients);

    let job = create(Action::SendMsgToSmtp, msg_id.to_u32() as i32, param, 0)?;

    Ok(Some(job))
}

pub(crate) enum Connection<'a> {
    Inbox(&'a mut Imap),
    Smtp(&'a mut Smtp),
}

async fn load_imap_deletion_msgid(context: &Context) -> sql::Result<Option<MsgId>> {
    if let Some(delete_server_after) = context.get_config_delete_server_after().await {
        let threshold_timestamp = time() - delete_server_after;

        context
            .sql
            .query_value_optional(
                r#"
SELECT id FROM msgs
  WHERE timestamp < ? AND server_uid != 0
"#,
                paramsx![threshold_timestamp],
            )
            .await
    } else {
        Ok(None)
    }
}

async fn load_imap_deletion_job(context: &Context) -> sql::Result<Option<Job>> {
    let res = if let Some(msg_id) = load_imap_deletion_msgid(context).await? {
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

impl<'a> fmt::Display for Connection<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Connection::Inbox(_) => write!(f, "Inbox"),
            Connection::Smtp(_) => write!(f, "Smtp"),
        }
    }
}

impl<'a> Connection<'a> {
    fn inbox(&mut self) -> &mut Imap {
        match self {
            Connection::Inbox(imap) => imap,
            _ => panic!("Not an inbox"),
        }
    }

    fn smtp(&mut self) -> &mut Smtp {
        match self {
            Connection::Smtp(smtp) => smtp,
            _ => panic!("Not a smtp"),
        }
    }
}

pub(crate) async fn perform_job(context: &Context, mut connection: Connection<'_>, mut job: Job) {
    info!(context, "{}-job {} started...", &connection, &job);

    let try_res = match perform_job_action(context, &mut job, &mut connection, 0).await {
        Status::RetryNow => perform_job_action(context, &mut job, &mut connection, 1).await,
        x => x,
    };

    match try_res {
        Status::RetryNow | Status::RetryLater => {
            let tries = job.tries + 1;

            if tries < JOB_RETRIES {
                info!(
                    context,
                    "{} thread increases job {} tries to {}", &connection, job, tries
                );
                job.tries = tries;
                let time_offset = get_backoff_time_offset(tries);
                job.desired_timestamp = time() + time_offset;
                info!(
                    context,
                    "{}-job #{} not succeeded on try #{}, retry in {} seconds.",
                    &connection,
                    job.job_id as u32,
                    tries,
                    time_offset
                );
                job.save(context).await.unwrap_or_else(|err| {
                    error!(context, "failed to save job: {}", err);
                });
            } else {
                info!(
                    context,
                    "{} thread removes job {} as it exhausted {} retries",
                    &connection,
                    job,
                    JOB_RETRIES
                );
                job.delete(context).await.unwrap_or_else(|err| {
                    error!(context, "failed to delete job: {}", err);
                });
            }
        }
        Status::Finished(res) => {
            if let Err(err) = res {
                warn!(
                    context,
                    "{} removes job {} as it failed with error {:?}", &connection, job, err
                );
            } else {
                info!(
                    context,
                    "{} removes job {} as it succeeded", &connection, job
                );
            }

            job.delete(context).await.unwrap_or_else(|err| {
                error!(context, "failed to delete job: {}", err);
            });
        }
    }
}

async fn perform_job_action(
    context: &Context,
    job: &mut Job,
    connection: &mut Connection<'_>,
    tries: u32,
) -> Status {
    info!(
        context,
        "{} begin immediate try {} of job {}", &connection, tries, job
    );

    let try_res = match job.action {
        Action::Unknown => Status::Finished(Err(format_err!("Unknown job id found"))),
        Action::SendMsgToSmtp => job.send_msg_to_smtp(context, connection.smtp()).await,
        Action::SendMdn => job.send_mdn(context, connection.smtp()).await,
        Action::MaybeSendLocations => location::job_maybe_send_locations(context, job).await,
        Action::MaybeSendLocationsEnded => {
            location::job_maybe_send_locations_ended(context, job).await
        }
        Action::EmptyServer => job.empty_server(context, connection.inbox()).await,
        Action::OldDeleteMsgOnImap => job.delete_msg_on_imap(context, connection.inbox()).await,
        Action::DeleteMsgOnImap => job.delete_msg_on_imap(context, connection.inbox()).await,
        Action::MarkseenMsgOnImap => job.markseen_msg_on_imap(context, connection.inbox()).await,
        Action::MoveMsg => job.move_msg(context, connection.inbox()).await,
        Action::Housekeeping => {
            if let Err(err) = sql::housekeeping(context).await {
                error!(context, "housekeeping failed: {}", err);
            }
            Status::Finished(Ok(()))
        }
    };

    info!(
        context,
        "Inbox finished immediate try {} of job {}", tries, job
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

async fn send_mdn(context: &Context, msg: &Message) -> Result<()> {
    let mut param = Params::new();
    param.set(Param::MsgId, msg.id.to_u32().to_string());

    add(context, Job::new(Action::SendMdn, msg.from_id, param, 0)).await;

    Ok(())
}

/// Creates a job.
pub fn create(action: Action, foreign_id: i32, param: Params, delay_seconds: i64) -> Result<Job> {
    ensure!(
        action != Action::Unknown,
        "Invalid action passed to job_add"
    );

    Ok(Job::new(action, foreign_id as u32, param, delay_seconds))
}

/// Adds a job to the database, scheduling it.
pub async fn add(context: &Context, job: Job) {
    let action = job.action;
    let delay_seconds = job.delay_seconds();
    job.save(context).await.unwrap_or_else(|err| {
        error!(context, "failed to save job: {}", err);
    });

    if delay_seconds == 0 {
        match action {
            Action::Unknown => unreachable!(),
            Action::Housekeeping
            | Action::EmptyServer
            | Action::OldDeleteMsgOnImap
            | Action::DeleteMsgOnImap
            | Action::MarkseenMsgOnImap
            | Action::MoveMsg => {
                info!(context, "interrupt: imap");
                context
                    .interrupt_inbox(InterruptInfo::new(false, None))
                    .await;
            }
            Action::MaybeSendLocations
            | Action::MaybeSendLocationsEnded
            | Action::SendMdn
            | Action::SendMsgToSmtp => {
                info!(context, "interrupt: smtp");
                context
                    .interrupt_smtp(InterruptInfo::new(false, None))
                    .await;
            }
        }
    }
}

/// Load jobs from the database.
///
/// Load jobs for this "[Thread]", i.e. either load SMTP jobs or load
/// IMAP jobs.  The `probe_network` parameter decides how to query
/// jobs, this is tricky and probably wrong currently. Look at the
/// SQL queries for details.
pub(crate) async fn load_next(
    context: &Context,
    thread: Thread,
    info: &InterruptInfo,
) -> Option<Job> {
    info!(context, "loading job for {}-thread", thread);

    let query;
    let params: Box<dyn Fn() -> sqlx::sqlite::SqliteArguments<'static> + 'static + Send>;
    let t = time();
    let thread_i = thread as i64;

    if let Some(msg_id) = info.msg_id {
        query = r#"
SELECT id, action, foreign_id, param, added_timestamp, desired_timestamp, tries
FROM jobs
WHERE thread=? AND foreign_id=?
ORDER BY action DESC, added_timestamp
LIMIT 1;
"#;
        params = Box::new(move || paramsx![thread_i, msg_id]);
    } else if !info.probe_network {
        // processing for first-try and after backoff-timeouts:
        // process jobs in the order they were added.
        query = r#"
SELECT id, action, foreign_id, param, added_timestamp, desired_timestamp, tries
FROM jobs
WHERE thread=? AND desired_timestamp<=?
ORDER BY action DESC, added_timestamp
LIMIT 1;
"#;
        params = Box::new(move || paramsx![thread_i, t]);
    } else {
        // processing after call to dc_maybe_network():
        // process _all_ pending jobs that failed before
        // in the order of their backoff-times.
        query = r#"
SELECT id, action, foreign_id, param, added_timestamp, desired_timestamp, tries
FROM jobs
WHERE thread=? AND tries>0
ORDER BY desired_timestamp, action DESC
LIMIT 1;
"#;
        params = Box::new(move || paramsx![thread_i]);
    };

    let job: Option<Job> = loop {
        let p = params();
        let job_res = context.sql.query_row_optional(query, p).await;

        match job_res {
            Ok(job) => break job,
            Err(err) => {
                // Remove invalid job from the DB
                info!(context, "cleaning up job, because of {}", err);

                // TODO: improve by only doing a single query
                let p = params();
                let id: Result<i32, _> = context.sql.query_value(query, p).await;
                match id {
                    Ok(id) => {
                        context
                            .sql
                            .execute("DELETE FROM jobs WHERE id=?;", paramsx![id])
                            .await
                            .ok();
                    }
                    Err(err) => {
                        error!(context, "failed to retrieve invalid job from DB: {}", err);
                        break None;
                    }
                }
            }
        }
    };

    match thread {
        Thread::Unknown => {
            error!(context, "unknown thread for job");
            None
        }
        Thread::Imap => {
            if let Some(job) = job {
                if job.action < Action::DeleteMsgOnImap {
                    load_imap_deletion_job(context)
                        .await
                        .unwrap_or_default()
                        .or(Some(job))
                } else {
                    Some(job)
                }
            } else {
                load_imap_deletion_job(context).await.unwrap_or_default()
            }
        }
        Thread::Smtp => job,
    }
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
                paramsx![
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
    async fn test_load_next_job_two() {
        // We want to ensure that loading jobs skips over jobs which
        // fails to load from the database instead of failing to load
        // all jobs.
        let t = TestContext::new().await;
        insert_job(&t.ctx, -1).await; // This can not be loaded into Job struct.
        let jobs = load_next(
            &t.ctx,
            Thread::from(Action::MoveMsg),
            &InterruptInfo::new(false, None),
        )
        .await;
        assert!(jobs.is_none());

        insert_job(&t.ctx, 1).await;
        let jobs = load_next(
            &t.ctx,
            Thread::from(Action::MoveMsg),
            &InterruptInfo::new(false, None),
        )
        .await;
        assert!(jobs.is_some());
    }

    #[async_std::test]
    async fn test_load_next_job_one() {
        let t = TestContext::new().await;

        insert_job(&t.ctx, 1).await;

        let jobs = load_next(
            &t.ctx,
            Thread::from(Action::MoveMsg),
            &InterruptInfo::new(false, None),
        )
        .await;
        assert!(jobs.is_some());
    }
}
