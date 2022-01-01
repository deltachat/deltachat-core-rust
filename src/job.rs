//! # Job module.
//!
//! This module implements a job queue maintained in the SQLite database
//! and job types.
use std::fmt;

use anyhow::{bail, format_err, Context as _, Error, Result};
use deltachat_derive::{FromSql, ToSql};
use rand::{thread_rng, Rng};

use crate::config::Config;
use crate::contact::{normalize_name, Contact, Modifier, Origin};
use crate::context::Context;
use crate::dc_tools::time;
use crate::events::EventType;
use crate::imap::{Imap, ImapActionResult};
use crate::location;
use crate::log::LogExt;
use crate::message::{Message, MsgId};
use crate::mimefactory::MimeFactory;
use crate::param::{Param, Params};
use crate::scheduler::InterruptInfo;
use crate::smtp::{smtp_send, Smtp};
use crate::sql;

// results in ~3 weeks for the last backoff timespan
const JOB_RETRIES: u32 = 17;

/// Thread IDs
#[derive(
    Debug, Display, Copy, Clone, PartialEq, Eq, FromPrimitive, ToPrimitive, FromSql, ToSql,
)]
#[repr(u32)]
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
#[repr(u32)]
pub enum Action {
    Unknown = 0,

    // Jobs in the INBOX-thread, range from DC_IMAP_THREAD..DC_IMAP_THREAD+999
    Housekeeping = 105, // low priority ...
    FetchExistingMsgs = 110,
    MarkseenMsgOnImap = 130,

    // this is user initiated so it should have a fairly high priority
    UpdateRecentQuota = 140,

    // This job will download partially downloaded messages completely
    // and is added when download_full() is called.
    // Most messages are downloaded automatically on fetch
    // and do not go through this job.
    DownloadMsg = 250,

    // UID synchronization is high-priority to make sure correct UIDs
    // are used by message moving/deletion.
    ResyncFolders = 300,

    // Jobs in the SMTP-thread, range from DC_SMTP_THREAD..DC_SMTP_THREAD+999
    MaybeSendLocations = 5005, // low priority ...
    MaybeSendLocationsEnded = 5007,
    SendMdn = 5010,
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
            FetchExistingMsgs => Thread::Imap,
            ResyncFolders => Thread::Imap,
            MarkseenMsgOnImap => Thread::Imap,
            UpdateRecentQuota => Thread::Imap,
            DownloadMsg => Thread::Imap,

            MaybeSendLocations => Thread::Smtp,
            MaybeSendLocationsEnded => Thread::Smtp,
            SendMdn => Thread::Smtp,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Job {
    pub job_id: u32,
    pub action: Action,
    pub foreign_id: u32,
    pub desired_timestamp: i64,
    pub added_timestamp: i64,
    pub tries: u32,
    pub param: Params,
}

impl fmt::Display for Job {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "#{}, action {}", self.job_id, self.action)
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
                .execute("DELETE FROM jobs WHERE id=?;", paramsv![self.job_id as i32])
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
                    paramsv![
                        self.desired_timestamp,
                        self.tries as i64,
                        self.param.to_string(),
                        self.job_id as i32,
                    ],
                )
                .await?;
        } else {
            context.sql.execute(
                "INSERT INTO jobs (added_timestamp, thread, action, foreign_id, param, desired_timestamp) VALUES (?,?,?,?,?,?);",
                paramsv![
                    self.added_timestamp,
                    thread,
                    self.action,
                    self.foreign_id,
                    self.param.to_string(),
                    self.desired_timestamp
                ]
            ).await?;
        }

        Ok(())
    }

    /// Get `SendMdn` jobs with foreign_id equal to `contact_id` excluding the `job_id` job.
    async fn get_additional_mdn_jobs(
        &self,
        context: &Context,
        contact_id: u32,
    ) -> Result<(Vec<u32>, Vec<String>)> {
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

    async fn send_mdn(&mut self, context: &Context, smtp: &mut Smtp) -> Status {
        let mdns_enabled = job_try!(context.get_config_bool(Config::MdnsEnabled).await);
        if !mdns_enabled {
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
        let rendered_msg = job_try!(mimefactory.render(context).await);
        let body = rendered_msg.message;

        let addr = contact.get_addr();
        let recipient = job_try!(async_smtp::EmailAddress::new(addr.to_string())
            .map_err(|err| format_err!("invalid recipient: {} {:?}", addr, err)));
        let recipients = vec![recipient];

        // connect to SMTP server, if not yet done
        if let Err(err) = smtp.connect_configured(context).await {
            warn!(context, "SMTP connection failure: {:?}", err);
            smtp.last_send_error = Some(err.to_string());
            return Status::RetryLater;
        }

        let status = smtp_send(context, recipients, body, smtp, msg_id, 0).await;
        if matches!(status, Status::Finished(Ok(_))) {
            // Remove additional SendMdn jobs we have aggregated into this one.
            job_try!(kill_ids(context, &additional_job_ids).await);
        }

        status
    }

    /// Read the recipients from old emails sent by the user and add them as contacts.
    /// This way, we can already offer them some email addresses they can write to.
    ///
    /// Then, Fetch the last messages DC_FETCH_EXISTING_MSGS_COUNT emails from the server
    /// and show them in the chat list.
    async fn fetch_existing_msgs(&mut self, context: &Context, imap: &mut Imap) -> Status {
        if job_try!(context.get_config_bool(Config::Bot).await) {
            return Status::Finished(Ok(())); // Bots don't want those messages
        }
        if let Err(err) = imap.prepare(context).await {
            warn!(context, "could not connect: {:?}", err);
            return Status::RetryLater;
        }

        add_all_recipients_as_contacts(context, imap, Config::ConfiguredSentboxFolder).await;
        add_all_recipients_as_contacts(context, imap, Config::ConfiguredMvboxFolder).await;
        add_all_recipients_as_contacts(context, imap, Config::ConfiguredInboxFolder).await;

        if job_try!(context.get_config_bool(Config::FetchExistingMsgs).await) {
            for config in &[
                Config::ConfiguredMvboxFolder,
                Config::ConfiguredInboxFolder,
                Config::ConfiguredSentboxFolder,
            ] {
                if let Some(folder) = job_try!(context.get_config(*config).await) {
                    if let Err(e) = imap.fetch_new_messages(context, &folder, true).await {
                        // We are using Anyhow's .context() and to show the inner error, too, we need the {:#}:
                        warn!(context, "Could not fetch messages, retrying: {:#}", e);
                        return Status::RetryLater;
                    };
                }
            }
        }

        info!(context, "Done fetching existing messages.");
        Status::Finished(Ok(()))
    }

    /// Synchronizes UIDs for sentbox, inbox and mvbox.
    async fn resync_folders(&mut self, context: &Context, imap: &mut Imap) -> Status {
        if let Err(err) = imap.prepare(context).await {
            warn!(context, "could not connect: {:?}", err);
            return Status::RetryLater;
        }

        let sentbox_folder = job_try!(context.get_config(Config::ConfiguredSentboxFolder).await);
        if let Some(sentbox_folder) = sentbox_folder {
            job_try!(imap.resync_folder_uids(context, sentbox_folder).await);
        }

        let inbox_folder = job_try!(context.get_config(Config::ConfiguredInboxFolder).await);
        if let Some(inbox_folder) = inbox_folder {
            job_try!(imap.resync_folder_uids(context, inbox_folder).await);
        }

        let mvbox_folder = job_try!(context.get_config(Config::ConfiguredMvboxFolder).await);
        if let Some(mvbox_folder) = mvbox_folder {
            job_try!(imap.resync_folder_uids(context, mvbox_folder).await);
        }

        Status::Finished(Ok(()))
    }

    async fn markseen_msg_on_imap(&mut self, context: &Context, imap: &mut Imap) -> Status {
        if let Err(err) = imap.prepare(context).await {
            warn!(context, "could not connect: {:?}", err);
            return Status::RetryLater;
        }

        let msg = job_try!(Message::load_from_db(context, MsgId::new(self.foreign_id)).await);
        let row = job_try!(
            context
                .sql
                .query_row_optional(
                    "SELECT uid, folder FROM imap
                    WHERE rfc724_mid=? AND folder=target
                    ORDER BY uid ASC
                    LIMIT 1",
                    paramsv![msg.rfc724_mid],
                    |row| {
                        let uid: u32 = row.get(0)?;
                        let folder: String = row.get(1)?;
                        Ok((uid, folder))
                    }
                )
                .await
        );
        if let Some((server_uid, server_folder)) = row {
            let result = imap.set_seen(context, &server_folder, server_uid).await;
            match result {
                ImapActionResult::RetryLater => return Status::RetryLater,
                ImapActionResult::Success | ImapActionResult::Failed => {}
            }
        } else {
            info!(
                context,
                "Can't mark the message {} as seen on IMAP because there is no known UID",
                msg.rfc724_mid
            );
        }

        // XXX we send MDN even in case of failure to mark the messages as seen, e.g. if it was
        // already deleted on the server by another device. The job will not be retried so locally
        // there is no risk of double-sending MDNs.
        //
        // Read receipts for system messages are never sent. These messages have no place to
        // display received read receipt anyway.  And since their text is locally generated,
        // quoting them is dangerous as it may contain contact names. E.g., for original message
        // "Group left by me", a read receipt will quote "Group left by <name>", and the name can
        // be a display name stored in address book rather than the name sent in the From field by
        // the user.
        if msg.param.get_bool(Param::WantsMdn).unwrap_or_default() && !msg.is_system_message() {
            let mdns_enabled = job_try!(context.get_config_bool(Config::MdnsEnabled).await);
            if mdns_enabled {
                if let Err(err) = send_mdn(context, &msg).await {
                    warn!(context, "could not send out mdn for {}: {}", msg.id, err);
                    return Status::Finished(Err(err));
                }
            }
        }

        Status::Finished(Ok(()))
    }
}

/// Delete all pending jobs with the given action.
pub async fn kill_action(context: &Context, action: Action) -> Result<()> {
    context
        .sql
        .execute("DELETE FROM jobs WHERE action=?;", paramsv![action])
        .await?;
    Ok(())
}

/// Remove jobs with specified IDs.
async fn kill_ids(context: &Context, job_ids: &[u32]) -> Result<()> {
    let q = format!(
        "DELETE FROM jobs WHERE id IN({})",
        job_ids.iter().map(|_| "?").collect::<Vec<&str>>().join(",")
    );
    context
        .sql
        .execute(q, rusqlite::params_from_iter(job_ids))
        .await?;
    Ok(())
}

pub async fn action_exists(context: &Context, action: Action) -> Result<bool> {
    let exists = context
        .sql
        .exists(
            "SELECT COUNT(*) FROM jobs WHERE action=?;",
            paramsv![action],
        )
        .await?;
    Ok(exists)
}

async fn add_all_recipients_as_contacts(context: &Context, imap: &mut Imap, folder: Config) {
    let mailbox = if let Ok(Some(m)) = context.get_config(folder).await {
        m
    } else {
        return;
    };
    if let Err(e) = imap.select_with_uidvalidity(context, &mailbox).await {
        // We are using Anyhow's .context() and to show the inner error, too, we need the {:#}:
        warn!(context, "Could not select {}: {:#}", mailbox, e);
        return;
    }
    match imap.get_all_recipients(context).await {
        Ok(contacts) => {
            let mut any_modified = false;
            for contact in contacts {
                let display_name_normalized = contact
                    .display_name
                    .as_ref()
                    .map(|s| normalize_name(s))
                    .unwrap_or_default();

                match Contact::add_or_lookup(
                    context,
                    &display_name_normalized,
                    &contact.addr,
                    Origin::OutgoingTo,
                )
                .await
                {
                    Ok((_, modified)) => {
                        if modified != Modifier::None {
                            any_modified = true;
                        }
                    }
                    Err(e) => warn!(context, "Could not add recipient: {}", e),
                }
            }
            if any_modified {
                context.emit_event(EventType::ContactsChanged(None));
            }
        }
        Err(e) => warn!(context, "Could not add recipients: {}", e),
    };
}

pub(crate) enum Connection<'a> {
    Inbox(&'a mut Imap),
    Smtp(&'a mut Smtp),
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
                let time_offset = get_backoff_time_offset(tries, job.action);
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
                    "{} removes job {} as it failed with error {:#}", &connection, job, err
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
        Action::SendMdn => job.send_mdn(context, connection.smtp()).await,
        Action::MaybeSendLocations => location::job_maybe_send_locations(context, job).await,
        Action::MaybeSendLocationsEnded => {
            location::job_maybe_send_locations_ended(context, job).await
        }
        Action::ResyncFolders => job.resync_folders(context, connection.inbox()).await,
        Action::MarkseenMsgOnImap => job.markseen_msg_on_imap(context, connection.inbox()).await,
        Action::FetchExistingMsgs => job.fetch_existing_msgs(context, connection.inbox()).await,
        Action::Housekeeping => {
            sql::housekeeping(context).await.ok_or_log(context);
            Status::Finished(Ok(()))
        }
        Action::UpdateRecentQuota => match context.update_recent_quota(connection.inbox()).await {
            Ok(status) => status,
            Err(err) => Status::Finished(Err(err)),
        },
        Action::DownloadMsg => job.download_msg(context, connection.inbox()).await,
    };

    info!(context, "Finished immediate try {} of job {}", tries, job);

    try_res
}

fn get_backoff_time_offset(tries: u32, action: Action) -> i64 {
    match action {
        // Just try every 10s to update the quota
        // If all retries are exhausted, a new job will be created when the quota information is needed
        Action::UpdateRecentQuota => 10,

        _ => {
            // Exponential backoff
            let n = 2_i32.pow(tries - 1) * 60;
            let mut rng = thread_rng();
            let r: i32 = rng.gen();
            let mut seconds = r % (n + 1);
            if seconds < 1 {
                seconds = 1;
            }
            seconds as i64
        }
    }
}

async fn send_mdn(context: &Context, msg: &Message) -> Result<()> {
    let mut param = Params::new();
    param.set(Param::MsgId, msg.id.to_u32().to_string());

    add(context, Job::new(Action::SendMdn, msg.from_id, param, 0)).await?;

    Ok(())
}

pub(crate) async fn schedule_resync(context: &Context) -> Result<()> {
    kill_action(context, Action::ResyncFolders).await?;
    add(
        context,
        Job::new(Action::ResyncFolders, 0, Params::new(), 0),
    )
    .await?;
    Ok(())
}

/// Adds a job to the database, scheduling it.
pub async fn add(context: &Context, job: Job) -> Result<()> {
    let action = job.action;
    let delay_seconds = job.delay_seconds();
    job.save(context).await.context("failed to save job")?;

    if delay_seconds == 0 {
        match action {
            Action::Unknown => unreachable!(),
            Action::Housekeeping
            | Action::ResyncFolders
            | Action::MarkseenMsgOnImap
            | Action::FetchExistingMsgs
            | Action::UpdateRecentQuota
            | Action::DownloadMsg => {
                info!(context, "interrupt: imap");
                context.interrupt_inbox(InterruptInfo::new(false)).await;
            }
            Action::MaybeSendLocations | Action::MaybeSendLocationsEnded | Action::SendMdn => {
                info!(context, "interrupt: smtp");
                context.interrupt_smtp(InterruptInfo::new(false)).await;
            }
        }
    }
    Ok(())
}

async fn load_housekeeping_job(context: &Context) -> Result<Option<Job>> {
    let last_time = context.get_config_i64(Config::LastHousekeeping).await?;

    let next_time = last_time + (60 * 60 * 24);
    if next_time <= time() {
        kill_action(context, Action::Housekeeping).await?;
        Ok(Some(Job::new(Action::Housekeeping, 0, Params::new(), 0)))
    } else {
        Ok(None)
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
) -> Result<Option<Job>> {
    info!(context, "loading job for {}-thread", thread);

    let query;
    let params;
    let t = time();
    let thread_i = thread as i64;

    if !info.probe_network {
        // processing for first-try and after backoff-timeouts:
        // process jobs in the order they were added.
        query = r#"
SELECT id, action, foreign_id, param, added_timestamp, desired_timestamp, tries
FROM jobs
WHERE thread=? AND desired_timestamp<=?
ORDER BY action DESC, added_timestamp
LIMIT 1;
"#;
        params = paramsv![thread_i, t];
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
        params = paramsv![thread_i];
    };

    let job = loop {
        let job_res = context
            .sql
            .query_row_optional(query, params.clone(), |row| {
                let job = Job {
                    job_id: row.get("id")?,
                    action: row.get("action")?,
                    foreign_id: row.get("foreign_id")?,
                    desired_timestamp: row.get("desired_timestamp")?,
                    added_timestamp: row.get("added_timestamp")?,
                    tries: row.get("tries")?,
                    param: row.get::<_, String>("param")?.parse().unwrap_or_default(),
                };

                Ok(job)
            })
            .await;

        match job_res {
            Ok(job) => break job,
            Err(err) => {
                // Remove invalid job from the DB
                info!(context, "cleaning up job, because of {}", err);

                // TODO: improve by only doing a single query
                let id = context
                    .sql
                    .query_row(query, params.clone(), |row| row.get::<_, i32>(0))
                    .await
                    .context("Failed to retrieve invalid job ID from the database")?;
                context
                    .sql
                    .execute("DELETE FROM jobs WHERE id=?;", paramsv![id])
                    .await
                    .with_context(|| format!("Failed to delete invalid job {}", id))?;
            }
        }
    };

    match thread {
        Thread::Unknown => {
            bail!("unknown thread for job")
        }
        Thread::Imap => {
            if let Some(job) = job {
                Ok(Some(job))
            } else {
                Ok(load_housekeeping_job(context).await?)
            }
        }
        Thread::Smtp => Ok(job),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::test_utils::TestContext;

    async fn insert_job(context: &Context, foreign_id: i64, valid: bool) {
        let now = time();
        context
            .sql
            .execute(
                "INSERT INTO jobs
                   (added_timestamp, thread, action, foreign_id, param, desired_timestamp)
                 VALUES (?, ?, ?, ?, ?, ?);",
                paramsv![
                    now,
                    Thread::from(Action::DownloadMsg),
                    if valid {
                        Action::DownloadMsg as i32
                    } else {
                        -1
                    },
                    foreign_id,
                    Params::new().to_string(),
                    now
                ],
            )
            .await
            .unwrap();
    }

    #[async_std::test]
    async fn test_load_next_job_two() -> Result<()> {
        // We want to ensure that loading jobs skips over jobs which
        // fails to load from the database instead of failing to load
        // all jobs.
        let t = TestContext::new().await;
        insert_job(&t, 1, false).await; // This can not be loaded into Job struct.
        let jobs = load_next(
            &t,
            Thread::from(Action::DownloadMsg),
            &InterruptInfo::new(false),
        )
        .await?;
        // The housekeeping job should be loaded as we didn't run housekeeping in the last day:
        assert_eq!(jobs.unwrap().action, Action::Housekeeping);

        insert_job(&t, 1, true).await;
        let jobs = load_next(
            &t,
            Thread::from(Action::DownloadMsg),
            &InterruptInfo::new(false),
        )
        .await?;
        assert!(jobs.is_some());
        Ok(())
    }

    #[async_std::test]
    async fn test_load_next_job_one() -> Result<()> {
        let t = TestContext::new().await;

        insert_job(&t, 1, true).await;

        let jobs = load_next(
            &t,
            Thread::from(Action::DownloadMsg),
            &InterruptInfo::new(false),
        )
        .await?;
        assert!(jobs.is_some());
        Ok(())
    }
}
