use std::time::Duration;

use deltachat_derive::{FromSql, ToSql};
use rand::{thread_rng, Rng};

use crate::blob::BlobObject;
use crate::chat;
use crate::config::Config;
use crate::configure::*;
use crate::constants::*;
use crate::context::Context;
use crate::dc_tools::*;
use crate::error::Error;
use crate::events::Event;
use crate::imap::*;
use crate::imex::*;
use crate::location;
use crate::login_param::LoginParam;
use crate::message::MsgId;
use crate::message::{self, Message, MessageState};
use crate::mimefactory::{vec_contains_lowercase, Loaded, MimeFactory};
use crate::param::*;
use crate::sql;

// results in ~3 weeks for the last backoff timespan
const JOB_RETRIES: i32 = 17;

/// Thread IDs
#[derive(Debug, Display, Copy, Clone, PartialEq, Eq, FromPrimitive, ToPrimitive, FromSql, ToSql)]
#[repr(i32)]
enum Thread {
    Unknown = 0,
    Imap = 100,
    Smtp = 5000,
}

#[derive(Debug, Display, Copy, Clone, PartialEq, Eq, FromPrimitive, ToPrimitive)]
enum TryAgain {
    Dont,
    AtOnce,
    StandardDelay,
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
    SendMdnOld = 5010,
    SendMdn = 5011,
    SendMsgToSmtpOld = 5900,
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
            SendMdnOld => Thread::Smtp,
            SendMdn => Thread::Smtp,
            SendMsgToSmtpOld => Thread::Smtp,
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
    pub tries: i32,
    pub param: Params,
    try_again: TryAgain,
    pub pending_error: Option<String>,
}

impl Job {
    fn delete(&self, context: &Context) -> bool {
        context
            .sql
            .execute("DELETE FROM jobs WHERE id=?;", params![self.job_id as i32])
            .is_ok()
    }

    fn update(&self, context: &Context) -> bool {
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
    }

    #[allow(non_snake_case)]
    fn SendMsgToSmtp(&mut self, context: &Context) {
        /* connect to SMTP server, if not yet done */
        if !context.smtp.lock().unwrap().is_connected() {
            let loginparam = LoginParam::from_database(context, "configured_");
            let connected = context.smtp.lock().unwrap().connect(context, &loginparam);
            if connected.is_err() {
                self.try_again_later(TryAgain::StandardDelay, None);
                return;
            }
        }

        if let Some(filename) = self.param.get_path(Param::File, context).unwrap_or(None) {
            if let Ok(body) = dc_read_file(context, &filename) {
                if let Some(recipients) = self.param.get(Param::Recipients) {
                    let recipients_list = recipients
                        .split('\x1e')
                        .filter_map(|addr| match lettre::EmailAddress::new(addr.to_string()) {
                            Ok(addr) => Some(addr),
                            Err(err) => {
                                warn!(context, "invalid recipient: {} {:?}", addr, err);
                                None
                            }
                        })
                        .collect::<Vec<_>>();

                    /* if there is a msg-id and it does not exist in the db, cancel sending.
                    this happends if dc_delete_msgs() was called
                    before the generated mime was sent out */
                    if 0 != self.foreign_id
                        && !message::exists(context, MsgId::new(self.foreign_id))
                    {
                        warn!(
                            context,
                            "Not sending Message {} as it was deleted", self.foreign_id
                        );
                        return;
                    };

                    // hold the smtp lock during sending of a job and
                    // its ok/error response processing. Note that if a message
                    // was sent we need to mark it in the database ASAP as we
                    // otherwise might send it twice.
                    let mut smtp = context.smtp.lock().unwrap();
                    match smtp.send(context, recipients_list, body, self.job_id) {
                        Err(err) => {
                            smtp.disconnect();
                            warn!(context, "smtp failed: {}", err);
                            self.try_again_later(TryAgain::AtOnce, Some(err.to_string()));
                        }
                        Ok(()) => {
                            // smtp success, update db ASAP, then delete smtp file
                            if 0 != self.foreign_id {
                                set_delivered(context, MsgId::new(self.foreign_id));
                            }
                            // now also delete the generated file
                            dc_delete_file(context, filename);
                        }
                    }
                } else {
                    warn!(context, "Missing recipients for job {}", self.job_id,);
                }
            }
        }
    }

    // this value does not increase the number of tries
    fn try_again_later(&mut self, try_again: TryAgain, pending_error: Option<String>) {
        self.try_again = try_again;
        self.pending_error = pending_error;
    }

    #[allow(non_snake_case)]
    fn MoveMsg(&mut self, context: &Context) {
        let imap_inbox = &context.inbox_thread.read().unwrap().imap;

        if let Ok(msg) = Message::load_from_db(context, MsgId::new(self.foreign_id)) {
            if context
                .sql
                .get_raw_config_int(context, "folders_configured")
                .unwrap_or_default()
                < 3
            {
                imap_inbox.configure_folders(context, true);
            }
            let dest_folder = context
                .sql
                .get_raw_config(context, "configured_mvbox_folder");

            if let Some(dest_folder) = dest_folder {
                let server_folder = msg.server_folder.as_ref().unwrap();
                let mut dest_uid = 0;

                match imap_inbox.mv(
                    context,
                    server_folder,
                    msg.server_uid,
                    &dest_folder,
                    &mut dest_uid,
                ) {
                    ImapActionResult::RetryLater => {
                        self.try_again_later(TryAgain::StandardDelay, None);
                    }
                    ImapActionResult::Success => {
                        message::update_server_uid(
                            context,
                            &msg.rfc724_mid,
                            &dest_folder,
                            dest_uid,
                        );
                    }
                    ImapActionResult::Failed | ImapActionResult::AlreadyDone => {}
                }
            }
        }
    }

    #[allow(non_snake_case)]
    fn DeleteMsgOnImap(&mut self, context: &Context) {
        let imap_inbox = &context.inbox_thread.read().unwrap().imap;

        if let Ok(mut msg) = Message::load_from_db(context, MsgId::new(self.foreign_id)) {
            if !msg.rfc724_mid.is_empty() {
                /* eg. device messages have no Message-ID */
                if message::rfc724_mid_cnt(context, &msg.rfc724_mid) > 1 {
                    info!(
                        context,
                        "The message is deleted from the server when all parts are deleted.",
                    );
                } else {
                    /* if this is the last existing part of the message,
                    we delete the message from the server */
                    let mid = msg.rfc724_mid;
                    let server_folder = msg.server_folder.as_ref().unwrap();
                    let res =
                        imap_inbox.delete_msg(context, &mid, server_folder, &mut msg.server_uid);
                    if res == ImapActionResult::RetryLater {
                        self.try_again_later(TryAgain::AtOnce, None);
                        return;
                    }
                }
                Message::delete_from_db(context, msg.id);
            }
        }
    }

    #[allow(non_snake_case)]
    fn EmptyServer(&mut self, context: &Context) {
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
    }

    #[allow(non_snake_case)]
    fn MarkseenMsgOnImap(&mut self, context: &Context) {
        let imap_inbox = &context.inbox_thread.read().unwrap().imap;

        if let Ok(msg) = Message::load_from_db(context, MsgId::new(self.foreign_id)) {
            let folder = msg.server_folder.as_ref().unwrap();
            match imap_inbox.set_seen(context, folder, msg.server_uid) {
                ImapActionResult::RetryLater => {
                    self.try_again_later(TryAgain::StandardDelay, None);
                }
                ImapActionResult::AlreadyDone => {}
                ImapActionResult::Success | ImapActionResult::Failed => {
                    // XXX the message might just have been moved
                    // we want to send out an MDN anyway
                    // The job will not be retried so locally
                    // there is no risk of double-sending MDNs.
                    if 0 != msg.param.get_int(Param::WantsMdn).unwrap_or_default()
                        && context.get_config_bool(Config::MdnsEnabled)
                    {
                        if let Err(err) = send_mdn(context, msg.id) {
                            warn!(context, "could not send out mdn for {}: {}", msg.id, err);
                        }
                    }
                }
            }
        }
    }

    #[allow(non_snake_case)]
    fn MarkseenMdnOnImap(&mut self, context: &Context) {
        let folder = self
            .param
            .get(Param::ServerFolder)
            .unwrap_or_default()
            .to_string();
        let uid = self.param.get_int(Param::ServerUid).unwrap_or_default() as u32;
        let imap_inbox = &context.inbox_thread.read().unwrap().imap;
        if imap_inbox.set_seen(context, &folder, uid) == ImapActionResult::RetryLater {
            self.try_again_later(TryAgain::StandardDelay, None);
            return;
        }
        if 0 != self.param.get_int(Param::AlsoMove).unwrap_or_default() {
            if context
                .sql
                .get_raw_config_int(context, "folders_configured")
                .unwrap_or_default()
                < 3
            {
                imap_inbox.configure_folders(context, true);
            }
            let dest_folder = context
                .sql
                .get_raw_config(context, "configured_mvbox_folder");
            if let Some(dest_folder) = dest_folder {
                let mut dest_uid = 0;
                if ImapActionResult::RetryLater
                    == imap_inbox.mv(context, &folder, uid, &dest_folder, &mut dest_uid)
                {
                    self.try_again_later(TryAgain::StandardDelay, None);
                }
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

pub fn perform_inbox_fetch(context: &Context) {
    let use_network = context.get_config_bool(Config::InboxWatch);

    context
        .inbox_thread
        .write()
        .unwrap()
        .fetch(context, use_network);
}

pub fn perform_mvbox_fetch(context: &Context) {
    let use_network = context.get_config_bool(Config::MvboxWatch);

    context
        .mvbox_thread
        .write()
        .unwrap()
        .fetch(context, use_network);
}

pub fn perform_sentbox_fetch(context: &Context) {
    let use_network = context.get_config_bool(Config::SentboxWatch);

    context
        .sentbox_thread
        .write()
        .unwrap()
        .fetch(context, use_network);
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

pub fn interrupt_inbox_idle(context: &Context, block: bool) {
    info!(context, "interrupt_inbox_idle called blocking={}", block);
    if block {
        context.inbox_thread.read().unwrap().interrupt_idle(context);
    } else {
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
        state.perform_jobs_needed = 0;

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

        if state.perform_jobs_needed == 1 {
            info!(
                context,
                "SMTP-idle will not be started because of waiting jobs.",
            );
        } else {
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

    info!(context, "SMTP-idle ended.",);
}

fn get_next_wakeup_time(context: &Context, thread: Thread) -> Duration {
    let t: i64 = context
        .sql
        .query_get_value(
            context,
            "SELECT MIN(desired_timestamp) FROM jobs WHERE thread=?;",
            params![thread],
        )
        .unwrap_or_default();

    let mut wakeup_time = Duration::new(10 * 60, 0);
    let now = time();
    if t > 0 {
        if t > now {
            wakeup_time = Duration::new((t - now) as u64, 0);
        } else {
            wakeup_time = Duration::new(0, 0);
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
    interrupt_inbox_idle(context, true);
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
    let chat_id: i32 = context
        .sql
        .query_get_value(
            context,
            "SELECT chat_id FROM msgs WHERE id=?",
            params![msg_id],
        )
        .unwrap_or_default();
    context.call_cb(Event::MsgDelivered {
        chat_id: chat_id as u32,
        msg_id: msg_id,
    });
}

/* special case for DC_JOB_SEND_MSG_TO_SMTP */
#[allow(non_snake_case)]
pub fn job_send_msg(context: &Context, msg_id: MsgId) -> Result<(), Error> {
    let mut mimefactory = MimeFactory::load_msg(context, msg_id)?;

    if chat::msgtype_has_file(mimefactory.msg.type_0) {
        let file_param = mimefactory.msg.param.get_path(Param::File, context)?;
        if let Some(pathNfilename) = file_param {
            if (mimefactory.msg.type_0 == Viewtype::Image
                || mimefactory.msg.type_0 == Viewtype::Gif)
                && !mimefactory.msg.param.exists(Param::Width)
            {
                mimefactory.msg.param.set_int(Param::Width, 0);
                mimefactory.msg.param.set_int(Param::Height, 0);

                if let Ok(buf) = dc_read_file(context, pathNfilename) {
                    if let Ok((width, height)) = dc_get_filemeta(&buf) {
                        mimefactory.msg.param.set_int(Param::Width, width as i32);
                        mimefactory.msg.param.set_int(Param::Height, height as i32);
                    }
                }
                mimefactory.msg.save_param_to_disk(context);
            }
        }
    }

    /* create message */
    if let Err(msg) = unsafe { mimefactory.render() } {
        let e = msg.to_string();
        message::set_msg_failed(context, msg_id, Some(e));
        return Err(msg);
    }
    if 0 != mimefactory
        .msg
        .param
        .get_int(Param::GuaranteeE2ee)
        .unwrap_or_default()
        && !mimefactory.out_encrypted
    {
        /* unrecoverable */
        message::set_msg_failed(
            context,
            msg_id,
            Some("End-to-end-encryption unavailable unexpectedly."),
        );
        bail!(
            "e2e encryption unavailable {} - {:?}",
            msg_id,
            mimefactory.msg.param.get_int(Param::GuaranteeE2ee),
        );
    }
    if context.get_config_bool(Config::BccSelf)
        && !vec_contains_lowercase(&mimefactory.recipients_addr, &mimefactory.from_addr)
    {
        mimefactory.recipients_names.push("".to_string());
        mimefactory
            .recipients_addr
            .push(mimefactory.from_addr.to_string());
    }

    if mimefactory.recipients_addr.is_empty() {
        // may happen eg. for groups with only SELF and bcc_self disabled
        info!(
            context,
            "message {} has no recipient, skipping smtp-send", msg_id
        );
        set_delivered(context, msg_id);
        return Ok(());
    }

    if mimefactory.out_gossiped {
        chat::set_gossiped_timestamp(context, mimefactory.msg.chat_id, time());
    }
    if 0 != mimefactory.out_last_added_location_id {
        if let Err(err) = location::set_kml_sent_timestamp(context, mimefactory.msg.chat_id, time())
        {
            error!(context, "Failed to set kml sent_timestamp: {:?}", err);
        }
        if !mimefactory.msg.hidden {
            if let Err(err) = location::set_msg_location_id(
                context,
                mimefactory.msg.id,
                mimefactory.out_last_added_location_id,
            ) {
                error!(context, "Failed to set msg_location_id: {:?}", err);
            }
        }
    }
    if mimefactory.out_encrypted
        && mimefactory
            .msg
            .param
            .get_int(Param::GuaranteeE2ee)
            .unwrap_or_default()
            == 0
    {
        mimefactory.msg.param.set_int(Param::GuaranteeE2ee, 1);
        mimefactory.msg.save_param_to_disk(context);
    }
    add_smtp_job(context, Action::SendMsgToSmtp, &mut mimefactory)?;

    Ok(())
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

    let jobs: Result<Vec<Job>, _> = context
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
                    try_again: TryAgain::Dont,
                    pending_error: None,
                };

                Ok(job)
            },
            |jobs| jobs.collect::<Result<Vec<Job>, _>>().map_err(Into::into),
        )
        .map_err(|err| {
            warn!(context, "query failed: {:?}", err);
        });

    for mut job in jobs.unwrap_or_default() {
        info!(
            context,
            "{}-job #{}, action {} started...",
            if thread == Thread::Imap {
                "INBOX"
            } else {
                "SMTP"
            },
            job.job_id,
            job.action,
        );

        // some configuration jobs are "exclusive":
        // - they are always executed in the imap-thread and the smtp-thread is suspended during execution
        // - they may change the database handle change the database handle; we do not keep old pointers therefore
        // - they can be re-executed one time AT_ONCE, but they are not save in the database for later execution
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

        let mut tries = 0;
        while tries <= 1 {
            // this can be modified by a job using dc_job_try_again_later()
            job.try_again = TryAgain::Dont;

            match job.action {
                Action::Unknown => {
                    info!(context, "Unknown job id found");
                }
                Action::SendMsgToSmtp => job.SendMsgToSmtp(context),
                Action::EmptyServer => job.EmptyServer(context),
                Action::DeleteMsgOnImap => job.DeleteMsgOnImap(context),
                Action::MarkseenMsgOnImap => job.MarkseenMsgOnImap(context),
                Action::MarkseenMdnOnImap => job.MarkseenMdnOnImap(context),
                Action::MoveMsg => job.MoveMsg(context),
                Action::SendMdn => job.SendMsgToSmtp(context),
                Action::ConfigureImap => JobConfigureImap(context),
                Action::ImexImap => match JobImexImap(context, &job) {
                    Ok(()) => {}
                    Err(err) => {
                        error!(context, "{}", err);
                    }
                },
                Action::MaybeSendLocations => location::JobMaybeSendLocations(context, &job),
                Action::MaybeSendLocationsEnded => {
                    location::JobMaybeSendLocationsEnded(context, &mut job)
                }
                Action::Housekeeping => sql::housekeeping(context),
                Action::SendMdnOld => {}
                Action::SendMsgToSmtpOld => {}
            }
            if job.try_again != TryAgain::AtOnce {
                break;
            }
            tries += 1
        }
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
        } else if job.try_again == TryAgain::AtOnce || job.try_again == TryAgain::StandardDelay {
            let tries = job.tries + 1;
            if tries < JOB_RETRIES {
                job.tries = tries;
                let time_offset = get_backoff_time_offset(tries);
                job.desired_timestamp = job.added_timestamp + time_offset;
                job.update(context);
                info!(
                    context,
                    "{}-job #{} not succeeded on try #{}, retry in ADD_TIME+{} (in {} seconds).",
                    if thread == Thread::Imap {
                        "INBOX"
                    } else {
                        "SMTP"
                    },
                    job.job_id as u32,
                    tries,
                    time_offset,
                    job.added_timestamp + time_offset - time()
                );
                if thread == Thread::Smtp && tries < JOB_RETRIES - 1 {
                    context
                        .smtp_state
                        .clone()
                        .0
                        .lock()
                        .unwrap()
                        .perform_jobs_needed = 2;
                }
            } else {
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
        } else {
            job.delete(context);
        }
    }
}

#[allow(non_snake_case)]
fn get_backoff_time_offset(c_tries: libc::c_int) -> i64 {
    let N = 2_i32.pow((c_tries - 1) as u32) * 60;
    let mut rng = thread_rng();
    let n: i32 = rng.gen();
    let mut seconds = n % (N + 1);
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
            std::thread::sleep(std::time::Duration::from_micros(300 * 1000));
        }
    }
}

fn send_mdn(context: &Context, msg_id: MsgId) -> Result<(), Error> {
    let mut mimefactory = MimeFactory::load_mdn(context, msg_id)?;
    unsafe { mimefactory.render()? };
    add_smtp_job(context, Action::SendMdn, &mut mimefactory)?;

    Ok(())
}

#[allow(non_snake_case)]
fn add_smtp_job(context: &Context, action: Action, mimefactory: &MimeFactory) -> Result<(), Error> {
    ensure!(
        !mimefactory.recipients_addr.is_empty(),
        "no recipients for smtp job set"
    );
    let mut param = Params::new();
    let bytes = unsafe {
        std::slice::from_raw_parts(
            (*mimefactory.out).str_0 as *const u8,
            (*mimefactory.out).len,
        )
    };
    let blob = BlobObject::create(context, &mimefactory.rfc724_mid, bytes)?;
    let recipients = mimefactory.recipients_addr.join("\x1e");
    param.set(Param::File, blob.as_name());
    param.set(Param::Recipients, &recipients);
    job_add(
        context,
        action,
        (if mimefactory.loaded == Loaded::Message {
            mimefactory.msg.id.to_u32() as i32
        } else {
            0
        }) as libc::c_int,
        param,
        0,
    );

    Ok(())
}

pub fn job_add(
    context: &Context,
    action: Action,
    foreign_id: libc::c_int,
    param: Params,
    delay_seconds: i64,
) {
    if action == Action::Unknown {
        error!(context, "Invalid action passed to job_add");
        return;
    }

    let timestamp = time();
    let thread: Thread = action.into();

    sql::execute(
        context,
        &context.sql,
        "INSERT INTO jobs (added_timestamp, thread, action, foreign_id, param, desired_timestamp) VALUES (?,?,?,?,?,?);",
        params![
            timestamp,
            thread,
            action,
            foreign_id,
            param.to_string(),
            (timestamp + delay_seconds as i64)
        ]
    ).ok();

    match thread {
        Thread::Imap => interrupt_inbox_idle(context, false),
        Thread::Smtp => interrupt_smtp_idle(context),
        Thread::Unknown => {}
    }
}

pub fn interrupt_smtp_idle(context: &Context) {
    info!(context, "Interrupting SMTP-idle...",);

    let &(ref lock, ref cvar) = &*context.smtp_state.clone();
    let mut state = lock.lock().unwrap();

    state.perform_jobs_needed = 1;
    state.idle = true;
    cvar.notify_one();
    info!(context, "Interrupting SMTP-idle... ended",);
}
