//! # Imap handling module
//!
//! uses [async-email/async-imap](https://github.com/async-email/async-imap)
//! to implement connect, fetch, delete functionality with standard IMAP servers.

use std::{cmp, cmp::max, collections::BTreeMap};

use anyhow::Context as _;
use async_imap::{
    error::Result as ImapResult,
    types::{Capability, Fetch, Flag, Mailbox, Name, NameAttribute},
};
use async_std::prelude::*;
use async_std::sync::Receiver;
use num_traits::FromPrimitive;

use crate::constants::*;
use crate::context::Context;
use crate::dc_receive_imf::{from_field_to_contact_id, is_msgrmsg_rfc724_mid_in_list};
use crate::error::{bail, format_err, Result};
use crate::events::EventType;
use crate::headerdef::{HeaderDef, HeaderDefMap};
use crate::job::{self, Action};
use crate::login_param::{CertificateChecks, LoginParam, ServerLoginParam};
use crate::message::{self, update_server_uid, MessageState};
use crate::mimeparser;
use crate::oauth2::dc_get_oauth2_access_token;
use crate::param::Params;
use crate::provider::{get_provider_info, Socket};
use crate::{
    chat, dc_tools::dc_extract_grpid_from_rfc724_mid, scheduler::InterruptInfo, stock::StockMessage,
};
use crate::{config::*, dc_receive_imf::dc_receive_imf_inner};

mod client;
mod idle;
pub mod scan_folders;
pub mod select_folder;
mod session;

use chat::get_chat_id_by_grpid;
use client::Client;
use mailparse::SingleInfo;
use message::Message;
use session::Session;

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq)]
pub enum ImapActionResult {
    Failed,
    RetryLater,
    AlreadyDone,
    Success,
}

/// Prefetch:
/// - Message-ID to check if we already have the message.
/// - In-Reply-To and References to check if message is a reply to chat message.
/// - Chat-Version to check if a message is a chat message
/// - Autocrypt-Setup-Message to check if a message is an autocrypt setup message,
///   not necessarily sent by Delta Chat.
const PREFETCH_FLAGS: &str = "(UID BODY.PEEK[HEADER.FIELDS (\
                              MESSAGE-ID \
                              FROM \
                              IN-REPLY-TO REFERENCES \
                              CHAT-VERSION \
                              AUTOCRYPT-SETUP-MESSAGE\
                              )])";
const DELETE_CHECK_FLAGS: &str = "(UID BODY.PEEK[HEADER.FIELDS (MESSAGE-ID)])";
const RFC724MID_UID: &str = "(UID BODY.PEEK[HEADER.FIELDS (MESSAGE-ID)])";
const JUST_UID: &str = "(UID)";
const BODY_FLAGS: &str = "(FLAGS BODY.PEEK[])";

#[derive(Debug)]
pub struct Imap {
    idle_interrupt: Receiver<InterruptInfo>,
    config: ImapConfig,
    session: Option<Session>,
    connected: bool,
    interrupt: Option<stop_token::StopSource>,
    should_reconnect: bool,
    login_failed_once: bool,
}

#[derive(Debug)]
struct OAuth2 {
    user: String,
    access_token: String,
}

impl async_imap::Authenticator for OAuth2 {
    type Response = String;

    fn process(&mut self, _data: &[u8]) -> Self::Response {
        format!(
            "user={}\x01auth=Bearer {}\x01\x01",
            self.user, self.access_token
        )
    }
}

#[derive(Debug, PartialEq)]
enum FolderMeaning {
    Unknown,
    SentObjects,
    Other,
}

#[derive(Debug)]
struct ImapConfig {
    pub addr: String,
    pub lp: ServerLoginParam,
    pub strict_tls: bool,
    pub oauth2: bool,
    pub selected_folder: Option<String>,
    pub selected_mailbox: Option<Mailbox>,
    pub selected_folder_needs_expunge: bool,
    pub can_idle: bool,

    /// True if the server has MOVE capability as defined in
    /// https://tools.ietf.org/html/rfc6851
    pub can_move: bool,
}

impl Default for ImapConfig {
    fn default() -> Self {
        ImapConfig {
            addr: "".into(),
            lp: Default::default(),
            strict_tls: false,
            oauth2: false,
            selected_folder: None,
            selected_mailbox: None,
            selected_folder_needs_expunge: false,
            can_idle: false,
            can_move: false,
        }
    }
}

impl Imap {
    pub fn new(idle_interrupt: Receiver<InterruptInfo>) -> Self {
        Imap {
            idle_interrupt,
            config: Default::default(),
            session: Default::default(),
            connected: Default::default(),
            interrupt: Default::default(),
            should_reconnect: Default::default(),
            login_failed_once: Default::default(),
        }
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }

    pub fn should_reconnect(&self) -> bool {
        self.should_reconnect
    }

    pub fn trigger_reconnect(&mut self) {
        self.should_reconnect = true;
    }

    /// Connects or reconnects if needed.
    ///
    /// It is safe to call this function if already connected, actions
    /// are performed only as needed.
    async fn try_setup_handle(&mut self, context: &Context) -> Result<()> {
        if self.config.lp.server.is_empty() {
            bail!("IMAP operation attempted while it is torn down");
        }

        if self.should_reconnect() {
            self.unsetup_handle(context).await;
            self.should_reconnect = false;
        } else if self.is_connected() {
            return Ok(());
        }

        let oauth2 = self.config.oauth2;

        let connection_res: ImapResult<Client> = if self.config.lp.security == Socket::STARTTLS
            || self.config.lp.security == Socket::Plain
        {
            let config = &mut self.config;
            let imap_server: &str = config.lp.server.as_ref();
            let imap_port = config.lp.port;

            match Client::connect_insecure((imap_server, imap_port)).await {
                Ok(client) => {
                    if config.lp.security == Socket::STARTTLS {
                        client.secure(imap_server, config.strict_tls).await
                    } else {
                        Ok(client)
                    }
                }
                Err(err) => Err(err),
            }
        } else {
            let config = &self.config;
            let imap_server: &str = config.lp.server.as_ref();
            let imap_port = config.lp.port;

            Client::connect_secure((imap_server, imap_port), imap_server, config.strict_tls).await
        };

        let login_res = match connection_res {
            Ok(client) => {
                let config = &self.config;
                let imap_user: &str = config.lp.user.as_ref();
                let imap_pw: &str = config.lp.password.as_ref();

                if oauth2 {
                    let addr: &str = config.addr.as_ref();

                    if let Some(token) =
                        dc_get_oauth2_access_token(context, addr, imap_pw, true).await
                    {
                        let auth = OAuth2 {
                            user: imap_user.into(),
                            access_token: token,
                        };
                        client.authenticate("XOAUTH2", auth).await
                    } else {
                        bail!("IMAP Could not get OAUTH token");
                    }
                } else {
                    client.login(imap_user, imap_pw).await
                }
            }
            Err(err) => {
                bail!(err);
            }
        };

        self.should_reconnect = false;

        match login_res {
            Ok(session) => {
                // needs to be set here to ensure it is set on reconnects.
                self.connected = true;
                self.session = Some(session);
                self.login_failed_once = false;
                Ok(())
            }

            Err((err, _)) => {
                let imap_user = self.config.lp.user.to_owned();
                let message = context
                    .stock_string_repl_str(StockMessage::CannotLogin, &imap_user)
                    .await;

                warn!(context, "{} ({})", message, err);

                let lock = context.wrong_pw_warning_mutex.lock().await;
                if self.login_failed_once
                    && context.get_config_bool(Config::NotifyAboutWrongPw).await
                {
                    if let Err(e) = context.set_config(Config::NotifyAboutWrongPw, None).await {
                        warn!(context, "{}", e);
                    }
                    drop(lock);

                    let mut msg = Message::new(Viewtype::Text);
                    msg.text = Some(message.clone());
                    if let Err(e) =
                        chat::add_device_msg_with_importance(context, None, Some(&mut msg), true)
                            .await
                    {
                        warn!(context, "{}", e);
                    }
                } else {
                    self.login_failed_once = true;
                }

                self.trigger_reconnect();
                Err(format_err!("{}\n\n{}", message, err))
            }
        }
    }

    /// Connects or reconnects if not already connected.
    ///
    /// This function emits network error if it fails.  It should not
    /// be used during configuration to avoid showing failed attempt
    /// errors to the user.
    async fn setup_handle(&mut self, context: &Context) -> Result<()> {
        let res = self.try_setup_handle(context).await;
        if let Err(ref err) = res {
            emit_event!(context, EventType::ErrorNetwork(err.to_string()));
        }
        res
    }

    async fn unsetup_handle(&mut self, context: &Context) {
        // Close folder if messages should be expunged
        if let Err(err) = self.close_folder(context).await {
            warn!(context, "failed to close folder: {:?}", err);
        }

        // Logout from the server
        if let Some(mut session) = self.session.take() {
            if let Err(err) = session.logout().await {
                warn!(context, "failed to logout: {:?}", err);
            }
        }
        self.connected = false;
        self.config.selected_folder = None;
        self.config.selected_mailbox = None;
    }

    async fn free_connect_params(&mut self) {
        let mut cfg = &mut self.config;

        cfg.addr = "".into();
        cfg.lp = Default::default();

        cfg.can_idle = false;
        cfg.can_move = false;
    }

    /// Connects to IMAP account using already-configured parameters.
    ///
    /// Emits network error if connection fails.
    pub async fn connect_configured(&mut self, context: &Context) -> Result<()> {
        if self.is_connected() && !self.should_reconnect() {
            return Ok(());
        }
        if !context.is_configured().await {
            bail!("IMAP Connect without configured params");
        }

        let param = LoginParam::from_database(context, "configured_").await;
        // the trailing underscore is correct

        if let Err(err) = self
            .connect(
                context,
                &param.imap,
                &param.addr,
                param.server_flags & DC_LP_AUTH_OAUTH2 != 0,
            )
            .await
        {
            bail!("IMAP Connection Failed with params {}: {}", param, err);
        } else {
            self.ensure_configured_folders(context, true).await
        }
    }

    /// Tries connecting to imap account using the specific login parameters.
    ///
    /// `addr` is used to renew token if OAuth2 authentication is used.
    ///
    /// Does not emit network errors, can be used to try various
    /// parameters during autoconfiguration.
    pub async fn connect(
        &mut self,
        context: &Context,
        lp: &ServerLoginParam,
        addr: &str,
        oauth2: bool,
    ) -> Result<()> {
        if lp.server.is_empty() || lp.user.is_empty() || lp.password.is_empty() {
            bail!("Incomplete IMAP connection parameters");
        }

        {
            let mut config = &mut self.config;
            config.addr = addr.to_string();
            config.lp = lp.clone();
            let provider = get_provider_info(&addr);
            config.strict_tls = match lp.certificate_checks {
                CertificateChecks::Automatic => {
                    provider.map_or(false, |provider| provider.strict_tls)
                }
                CertificateChecks::Strict => true,
                CertificateChecks::AcceptInvalidCertificates
                | CertificateChecks::AcceptInvalidCertificates2 => false,
            };
            config.oauth2 = oauth2;
        }

        if let Err(err) = self.try_setup_handle(context).await {
            warn!(context, "try_setup_handle: {}", err);
            self.free_connect_params().await;
            return Err(err);
        }

        let teardown = match &mut self.session {
            Some(ref mut session) => match session.capabilities().await {
                Ok(caps) => {
                    if !context.sql.is_open().await {
                        warn!(context, "IMAP-LOGIN as {} ok but ABORTING", lp.user,);
                        true
                    } else {
                        let can_idle = caps.has_str("IDLE");
                        let can_move = caps.has_str("MOVE");
                        let caps_list = caps.iter().fold(String::new(), |s, c| {
                            if let Capability::Atom(x) = c {
                                s + &format!(" {}", x)
                            } else {
                                s + &format!(" {:?}", c)
                            }
                        });

                        self.config.can_idle = can_idle;
                        self.config.can_move = can_move;
                        self.connected = true;
                        emit_event!(
                            context,
                            EventType::ImapConnected(format!(
                                "IMAP-LOGIN as {}, capabilities: {}",
                                lp.user, caps_list,
                            ))
                        );
                        false
                    }
                }
                Err(err) => {
                    info!(context, "CAPABILITY command error: {}", err);
                    true
                }
            },
            None => true,
        };

        if teardown {
            self.disconnect(context).await;

            warn!(
                context,
                "IMAP disconnected immediately after connecting due to error"
            );
        }
        Ok(())
    }

    pub async fn disconnect(&mut self, context: &Context) {
        self.unsetup_handle(context).await;
        self.free_connect_params().await;
    }

    pub async fn fetch(&mut self, context: &Context, watch_folder: &str) -> Result<()> {
        if !context.sql.is_open().await {
            // probably shutdown
            bail!("IMAP operation attempted while it is torn down");
        }
        self.setup_handle(context).await?;

        while self
            .fetch_new_messages(context, &watch_folder, false)
            .await?
        {
            // We fetch until no more new messages are there.
        }
        Ok(())
    }

    /// Synchronizes UIDs in the database with UIDs on the server.
    ///
    /// It is assumed that no operations are taking place on the same
    /// folder at the moment. Make sure to run it in the same
    /// thread/task as other network operations on this folder to
    /// avoid race conditions.
    pub(crate) async fn resync_folder_uids(
        &mut self,
        context: &Context,
        folder: String,
    ) -> Result<()> {
        // Collect pairs of UID and Message-ID.
        let mut msg_ids = BTreeMap::new();

        self.select_folder(context, Some(&folder)).await?;

        let session = if let Some(ref mut session) = &mut self.session {
            session
        } else {
            bail!("IMAP No Connection established");
        };

        match session.uid_fetch("1:*", RFC724MID_UID).await {
            Ok(mut list) => {
                while let Some(fetch) = list.next().await {
                    let msg = fetch?;

                    // Get Message-ID
                    let message_id = get_fetch_headers(&msg)
                        .and_then(|headers| prefetch_get_message_id(&headers))
                        .ok();

                    if let (Some(uid), Some(rfc724_mid)) = (msg.uid, message_id) {
                        msg_ids.insert(uid, rfc724_mid);
                    }
                }
            }
            Err(err) => {
                bail!("Can't resync folder {}: {}", folder, err);
            }
        }

        info!(
            context,
            "Resync: collected {} message IDs in folder {}",
            msg_ids.len(),
            &folder
        );

        // Write collected UIDs to SQLite database.
        context
            .sql
            .with_conn(move |mut conn| {
                let conn2 = &mut conn;
                let tx = conn2.transaction()?;
                tx.execute(
                    "UPDATE msgs SET server_uid=0 WHERE server_folder=?",
                    params![folder],
                )?;
                for (uid, rfc724_mid) in &msg_ids {
                    // This may detect previously undetected moved
                    // messages, so we update server_folder too.
                    tx.execute(
                        "UPDATE msgs \
                         SET server_folder=?,server_uid=? WHERE rfc724_mid=?",
                        params![folder, uid, rfc724_mid],
                    )?;
                }
                tx.commit()?;
                Ok(())
            })
            .await?;
        Ok(())
    }

    /// return Result with (uid_validity, last_seen_uid, new_emails) tuple.
    /// If in doubt, returns new_emails=true so emails are fetched.
    pub(crate) async fn select_with_uidvalidity(
        &mut self,
        context: &Context,
        folder: &str,
    ) -> Result<(u32, u32, bool)> {
        let newly_selected = self.select_folder(context, Some(folder)).await?;
        warn!(context, "dbg selected {}", folder);

        // compare last seen UIDVALIDITY against the current one
        let (uid_validity, last_seen_uid) = get_config_last_seen_uid(context, &folder).await;

        let mailbox = &mut self.config.selected_mailbox.as_ref();
        let mailbox =
            mailbox.with_context(|| format!("No mailbox selected, folder: {}", folder))?;

        let new_uid_validity = mailbox
            .uid_validity
            .with_context(|| format!("No UIDVALIDITY for folder {}", folder))?;

        //TODO in some cases we don't need largest_uid
        let largest_uid = match mailbox.uid_next {
            Some(uid_next) => max(uid_next, 1) - 1,
            None => {
                warn!(
                    context,
                    "IMAP folder has no uid_next, fall back to fetching"
                );
                let session = self
                    .session
                    .as_mut()
                    .context("select: IMAP No Connection established")?;
                // note that we use fetch by sequence number
                // and thus we only need to get exactly the
                // last-index message.
                let set = format!("{}", mailbox.exists);
                let mut list = session
                    .fetch(set, JUST_UID)
                    .await
                    .map_err(|e| format_err!("select: IMAP Could not fetch: {}", e))?;

                let mut new_last_seen_uid = None;
                while let Some(fetch) = list.next().await.transpose()? {
                    if fetch.message == mailbox.exists && fetch.uid.is_some() {
                        new_last_seen_uid = fetch.uid;
                    }
                }
                new_last_seen_uid.context("select: failed to fetch")?
            }
        };

        warn!(
            context,
            "dbg largest_uid {}, lastseen_uid {}", largest_uid, last_seen_uid
        );

        if new_uid_validity == uid_validity {
            let new_emails = if newly_selected {
                largest_uid > last_seen_uid
            } else {
                true
                // The folder was not newly selected i.e. no SELECT command was run. This means that mailbox.uid_next
                // was not updated and largest_uid may contain an incorrect value. So, just return true so that
                // the caller tries to fetch new messages (we could of course run a SELECT command but it would be
                // unnecessary as trying to fetch new messages is only one command, just as a SELECT command)
            };
            return Ok((uid_validity, last_seen_uid, new_emails));
        }

        if mailbox.exists == 0 {
            info!(context, "Folder \"{}\" is empty.", folder);

            // set lastseenuid=0 for empty folders.
            // id we do not do this here, we'll miss the first message
            // as we will get in here again and fetch from lastseenuid+1 then

            set_config_last_seen_uid(context, &folder, new_uid_validity, 0).await;
            return Ok((new_uid_validity, 0, false));
        }

        // uid_validity has changed or is being set the first time.
        // Set the lastseen uid to the largest UID in the mailbox.
        // TODO what if UIDvalidity changed and since then new messages arrived?
        // Currently we will miss these messages
        set_config_last_seen_uid(context, &folder, new_uid_validity, largest_uid).await;
        if uid_validity != 0 || last_seen_uid != 0 {
            job::schedule_resync(context).await;
        }
        info!(
            context,
            "uid/validity change: new {}/{} current {}/{}",
            largest_uid,
            new_uid_validity,
            uid_validity,
            last_seen_uid
        );
        Ok((new_uid_validity, largest_uid, true))
    }

    pub(crate) async fn fetch_new_messages<S: AsRef<str>>(
        &mut self,
        context: &Context,
        folder: S,
        fetch_existing_msgs: bool,
    ) -> Result<bool> {
        let show_emails = ShowEmails::from_i32(context.get_config_int(Config::ShowEmails).await)
            .unwrap_or_default();

        let (uid_validity, last_seen_uid, new_emails) = self
            .select_with_uidvalidity(context, folder.as_ref())
            .await?;

        if !new_emails && !fetch_existing_msgs {
            info!(context, "No new emails in folder {}", folder.as_ref());
            return Ok(false);
        }

        let msgs = if fetch_existing_msgs {
            self.fetch_existing_msgs_prefetch().await?
        } else {
            self.fetch_after(context, last_seen_uid).await?
        };
        let read_cnt = msgs.len();
        let folder: &str = folder.as_ref();

        let mut read_errors = 0;
        let mut uids = Vec::with_capacity(msgs.len());
        let mut new_last_seen_uid = None;

        for (current_uid, msg) in msgs.into_iter() {
            let (headers, msg_id) = match get_fetch_headers(&msg) {
                Ok(headers) => {
                    let msg_id = prefetch_get_message_id(&headers).unwrap_or_default();
                    (headers, msg_id)
                }
                Err(err) => {
                    warn!(context, "{}", err);
                    read_errors += 1;
                    continue;
                }
            };

            if message_needs_processing(
                context,
                current_uid,
                &headers,
                &msg_id,
                folder,
                show_emails,
            )
            .await
            {
                // Trigger download and processing for this message.
                uids.push(current_uid);
            } else if read_errors == 0 {
                // No errors so far, but this was skipped, so mark as last_seen_uid
                new_last_seen_uid = Some(current_uid);
            }
        }

        // check passed, go fetch the emails
        let (new_last_seen_uid_processed, error_cnt) = self
            .fetch_many_msgs(context, &folder, &uids, fetch_existing_msgs)
            .await;
        read_errors += error_cnt;

        // determine which last_seen_uid to use to update  to
        let new_last_seen_uid_processed = new_last_seen_uid_processed.unwrap_or_default();
        let new_last_seen_uid = new_last_seen_uid.unwrap_or_default();
        let last_one = new_last_seen_uid.max(new_last_seen_uid_processed);

        if last_one > last_seen_uid {
            set_config_last_seen_uid(context, &folder, uid_validity, last_one).await;
        }

        if read_errors == 0 {
            info!(context, "{} mails read from \"{}\".", read_cnt, folder,);
        } else {
            warn!(
                context,
                "{} mails read from \"{}\" with {} errors.", read_cnt, folder, read_errors
            );
        }

        Ok(read_cnt > 0)
    }

    /// Gets the from, to and bcc addresses from all existing outgoing emails.
    pub async fn get_all_recipients(&mut self, context: &Context) -> Result<Vec<SingleInfo>> {
        if self.session.is_none() {
            bail!("IMAP No Connection established");
        }

        let session = self.session.as_mut().unwrap();
        let self_addr = context
            .get_config(Config::ConfiguredAddr)
            .await
            .ok_or_else(|| format_err!("Not configured"))?;

        let search_command = format!("FROM \"{}\"", self_addr);
        let uids = session.uid_search(search_command).await?;
        let uid_strings: Vec<String> = uids.into_iter().map(|s| s.to_string()).collect();

        let mut result = Vec::new();
        // We fetch the emails in chunks of 100 because according to https://tools.ietf.org/html/rfc2683#section-3.2.1.5
        // command lines should not be much more than 1000 chars and UIDs can get up to 9- or 10-digit
        // (servers should allow at least 8000 chars)
        for uid_chunk in uid_strings.chunks(100) {
            let uid_set = uid_chunk.join(",");

            let mut list = session
                .uid_fetch(uid_set, "(UID BODY.PEEK[HEADER.FIELDS (FROM TO CC BCC)])")
                .await
                .map_err(|err| {
                    format_err!("IMAP Could not fetch (get_all_recipients()): {}", err)
                })?;

            while let Some(fetch) = list.next().await {
                let msg = fetch?;
                match get_fetch_headers(&msg) {
                    Ok(headers) => {
                        let (from_id, _, _) =
                            from_field_to_contact_id(context, &mimeparser::get_from(&headers))
                                .await?;
                        if from_id == DC_CONTACT_ID_SELF {
                            result.extend(mimeparser::get_recipients(&headers));
                        }
                    }

                    Err(err) => {
                        warn!(context, "{}", err);
                        continue;
                    }
                };
            }
        }
        Ok(result)
    }

    /// Fetch all uids larger than the passed in. Returns a sorted list of fetch results.
    async fn fetch_after(
        &mut self,
        context: &Context,
        uid: u32,
    ) -> Result<BTreeMap<u32, async_imap::types::Fetch>> {
        let session = self.session.as_mut();
        let session = session.context("fetch_after(): IMAP No Connection established")?;

        // fetch messages with larger UID than the last one seen
        // `(UID FETCH lastseenuid+1:*)`, see RFC 4549
        let set = format!("{}:*", uid + 1);
        let mut list = session
            .uid_fetch(set, PREFETCH_FLAGS)
            .await
            .map_err(|err| format_err!("IMAP Could not fetch: {}", err))?;

        let mut msgs = BTreeMap::new();
        while let Some(fetch) = list.next().await {
            let msg = fetch?;
            if let Some(msg_uid) = msg.uid {
                msgs.insert(msg_uid, msg);
            }
        }
        drop(list);

        // If the mailbox is not empty, results always include
        // at least one UID, even if last_seen_uid+1 is past
        // the last UID in the mailbox.  It happens because
        // uid+1:* is interpreted the same way as *:uid+1.
        // See https://tools.ietf.org/html/rfc3501#page-61 for
        // standard reference. Therefore, sometimes we receive
        // already seen messages and have to filter them out.
        let new_msgs = msgs.split_off(&(uid + 1));

        for current_uid in msgs.keys() {
            info!(
                context,
                "fetch_new_messages: ignoring uid {}, last seen was {}", current_uid, uid
            );
        }

        Ok(new_msgs)
    }

    /// Like fetch_after(), but not for new messages but existing ones (the DC_FETCH_EXISTING_MSGS_COUNT newest messages)
    async fn fetch_existing_msgs_prefetch(
        &mut self,
    ) -> Result<BTreeMap<u32, async_imap::types::Fetch>> {
        let exists: i64 = {
            let mailbox = self.config.selected_mailbox.as_ref();
            let mailbox = mailbox.context("fetch_existing_msgs_prefetch(): no mailbox selected")?;
            mailbox.exists.into()
        };
        let session = self.session.as_mut();
        let session =
            session.context("fetch_existing_msgs_prefetch(): IMAP No Connection established")?;

        // Fetch last DC_FETCH_EXISTING_MSGS_COUNT (100) messages.
        // Sequence numbers are sequential. If there are 1000 messages in the inbox,
        // we can fetch the sequence numbers 900-1000 and get the last 100 messages.
        let first = cmp::max(1, exists - DC_FETCH_EXISTING_MSGS_COUNT);
        let set = format!("{}:*", first);
        let mut list = session
            .fetch(&set, PREFETCH_FLAGS)
            .await
            .map_err(|err| format_err!("IMAP Could not fetch: {}", err))?;

        let mut msgs = BTreeMap::new();
        while let Some(fetch) = list.next().await {
            let msg = fetch?;
            if let Some(msg_uid) = msg.uid {
                msgs.insert(msg_uid, msg);
            }
        }

        Ok(msgs)
    }

    /// Fetches a list of messages by server UID.
    /// The passed in list of uids must be sorted.
    ///
    /// Returns the last uid fetch successfully and an error count.
    async fn fetch_many_msgs<S: AsRef<str>>(
        &mut self,
        context: &Context,
        folder: S,
        server_uids: &[u32],
        fetching_existing_messages: bool,
    ) -> (Option<u32>, usize) {
        let set = match server_uids {
            [] => return (None, 0),
            [server_uid] => server_uid.to_string(),
            [first_uid, .., last_uid] => {
                // XXX: it is assumed that UIDs are sorted and
                // contiguous. If UIDs are not contiguous, more
                // messages than needed will be downloaded.
                debug_assert!(first_uid < last_uid, "uids must be sorted");
                format!("{}:{}", first_uid, last_uid)
            }
        };

        if !self.is_connected() {
            warn!(context, "Not connected");
            return (None, server_uids.len());
        }

        if self.session.is_none() {
            // we could not get a valid imap session, this should be retried
            self.trigger_reconnect();
            warn!(context, "Could not get IMAP session");
            return (None, server_uids.len());
        }

        let session = self.session.as_mut().unwrap();

        let mut msgs = match session.uid_fetch(&set, BODY_FLAGS).await {
            Ok(msgs) => msgs,
            Err(err) => {
                // TODO: maybe differentiate between IO and input/parsing problems
                // so we don't reconnect if we have a (rare) input/output parsing problem?
                self.should_reconnect = true;
                warn!(
                    context,
                    "Error on fetching messages #{} from folder \"{}\"; error={}.",
                    &set,
                    folder.as_ref(),
                    err
                );
                return (None, server_uids.len());
            }
        };

        let folder = folder.as_ref().to_string();

        let mut read_errors = 0;
        let mut last_uid = None;
        let mut count = 0;

        while let Some(Ok(msg)) = msgs.next().await {
            let server_uid = msg.uid.unwrap_or_default();

            if !server_uids.contains(&server_uid) {
                // skip if there are some in between we are not interested in
                continue;
            }
            count += 1;

            let is_deleted = msg.flags().any(|flag| flag == Flag::Deleted);
            if is_deleted || msg.body().is_none() {
                // No need to process these.
                continue;
            }

            // XXX put flags into a set and pass them to dc_receive_imf
            let context = context.clone();
            let folder = folder.clone();

            // safe, as we checked above that there is a body.
            let body = msg.body().unwrap();
            let is_seen = msg.flags().any(|flag| flag == Flag::Seen);

            match dc_receive_imf_inner(
                &context,
                &body,
                &folder,
                server_uid,
                is_seen,
                fetching_existing_messages,
            )
            .await
            {
                Ok(_) => last_uid = Some(server_uid),
                Err(err) => {
                    warn!(context, "dc_receive_imf error: {}", err);
                    read_errors += 1;
                }
            };
        }

        if count != server_uids.len() {
            warn!(
                context,
                "failed to fetch all uids: got {}, requested {}",
                count,
                server_uids.len()
            );
        }

        (last_uid, read_errors)
    }

    pub async fn can_move(&self) -> bool {
        self.config.can_move
    }

    pub async fn mv(
        &mut self,
        context: &Context,
        folder: &str,
        uid: u32,
        dest_folder: &str,
    ) -> ImapActionResult {
        if folder == dest_folder {
            info!(
                context,
                "Skip moving message; message {}/{} is already in {}...", folder, uid, dest_folder,
            );
            return ImapActionResult::AlreadyDone;
        }
        if let Some(imapresult) = self
            .prepare_imap_operation_on_msg(context, folder, uid)
            .await
        {
            return imapresult;
        }
        // we are connected, and the folder is selected
        let set = format!("{}", uid);
        let display_folder_id = format!("{}/{}", folder, uid);

        if self.can_move().await {
            if let Some(ref mut session) = &mut self.session {
                match session.uid_mv(&set, &dest_folder).await {
                    Ok(_) => {
                        emit_event!(
                            context,
                            EventType::ImapMessageMoved(format!(
                                "IMAP Message {} moved to {}",
                                display_folder_id, dest_folder
                            ))
                        );
                        return ImapActionResult::Success;
                    }
                    Err(err) => {
                        warn!(
                            context,
                            "Cannot move message, fallback to COPY/DELETE {}/{} to {}: {}",
                            folder,
                            uid,
                            dest_folder,
                            err
                        );
                    }
                }
            } else {
                unreachable!();
            };
        } else {
            info!(
                context,
                "Server does not support MOVE, fallback to COPY/DELETE {}/{} to {}",
                folder,
                uid,
                dest_folder
            );
        }

        if let Some(ref mut session) = &mut self.session {
            if let Err(err) = session.uid_copy(&set, &dest_folder).await {
                warn!(context, "Could not copy message: {}", err);
                return ImapActionResult::Failed;
            }
        } else {
            unreachable!();
        }

        if !self.add_flag_finalized(context, uid, "\\Deleted").await {
            warn!(context, "Cannot mark {} as \"Deleted\" after copy.", uid);
            emit_event!(
                context,
                EventType::ImapMessageMoved(format!(
                    "IMAP Message {} copied to {} (delete FAILED)",
                    display_folder_id, dest_folder
                ))
            );
            ImapActionResult::Failed
        } else {
            self.config.selected_folder_needs_expunge = true;
            emit_event!(
                context,
                EventType::ImapMessageMoved(format!(
                    "IMAP Message {} copied to {} (delete successfull)",
                    display_folder_id, dest_folder
                ))
            );
            ImapActionResult::Success
        }
    }

    async fn add_flag_finalized(&mut self, context: &Context, server_uid: u32, flag: &str) -> bool {
        // return true if we successfully set the flag or we otherwise
        // think add_flag should not be retried: Disconnection during setting
        // the flag, or other imap-errors, returns true as well.
        //
        // returning false means that the operation can be retried.
        if server_uid == 0 {
            return true; // might be moved but we don't want to have a stuck job
        }
        let s = server_uid.to_string();
        self.add_flag_finalized_with_set(context, &s, flag).await
    }

    async fn add_flag_finalized_with_set(
        &mut self,
        context: &Context,
        uid_set: &str,
        flag: &str,
    ) -> bool {
        if self.should_reconnect() {
            return false;
        }
        if let Some(ref mut session) = &mut self.session {
            let query = format!("+FLAGS ({})", flag);
            match session.uid_store(uid_set, &query).await {
                Ok(mut responses) => {
                    while let Some(_response) = responses.next().await {
                        // Read all the responses
                    }
                }
                Err(err) => {
                    warn!(
                        context,
                        "IMAP failed to store: ({}, {}) {:?}", uid_set, query, err
                    );
                }
            }
            true // we tried once, that's probably enough for setting flag
        } else {
            unreachable!();
        }
    }

    pub async fn prepare_imap_operation_on_msg(
        &mut self,
        context: &Context,
        folder: &str,
        uid: u32,
    ) -> Option<ImapActionResult> {
        if uid == 0 {
            return Some(ImapActionResult::RetryLater);
        }
        if !self.is_connected() {
            // currently jobs are only performed on the INBOX thread
            // TODO: make INBOX/SENT/MVBOX perform the jobs on their
            // respective folders to avoid select_folder network traffic
            // and the involved error states
            if let Err(err) = self.connect_configured(context).await {
                warn!(context, "prepare_imap_op failed: {}", err);
                return Some(ImapActionResult::RetryLater);
            }
        }
        match self.select_folder(context, Some(&folder)).await {
            Ok(_) => None,
            Err(select_folder::Error::ConnectionLost) => {
                warn!(context, "Lost imap connection");
                Some(ImapActionResult::RetryLater)
            }
            Err(select_folder::Error::NoSession) => {
                warn!(context, "no imap session");
                Some(ImapActionResult::Failed)
            }
            Err(select_folder::Error::BadFolderName(folder_name)) => {
                warn!(context, "invalid folder name: {:?}", folder_name);
                Some(ImapActionResult::Failed)
            }
            Err(err) => {
                warn!(context, "failed to select folder: {:?}: {:?}", folder, err);
                Some(ImapActionResult::RetryLater)
            }
        }
    }

    pub async fn set_seen(
        &mut self,
        context: &Context,
        folder: &str,
        uid: u32,
    ) -> ImapActionResult {
        if let Some(imapresult) = self
            .prepare_imap_operation_on_msg(context, folder, uid)
            .await
        {
            return imapresult;
        }
        // we are connected, and the folder is selected
        info!(context, "Marking message {}/{} as seen...", folder, uid,);

        if self.add_flag_finalized(context, uid, "\\Seen").await {
            ImapActionResult::Success
        } else {
            warn!(
                context,
                "Cannot mark message {} in folder {} as seen, ignoring.", uid, folder
            );
            ImapActionResult::Failed
        }
    }

    pub async fn delete_msg(
        &mut self,
        context: &Context,
        message_id: &str,
        folder: &str,
        uid: u32,
    ) -> ImapActionResult {
        if let Some(imapresult) = self
            .prepare_imap_operation_on_msg(context, folder, uid)
            .await
        {
            return imapresult;
        }
        // we are connected, and the folder is selected

        let set = format!("{}", uid);
        let display_imap_id = format!("{}/{}", folder, uid);

        // double-check that we are deleting the correct message-id
        // this comes at the expense of another imap query
        if let Some(ref mut session) = &mut self.session {
            match session.uid_fetch(set, DELETE_CHECK_FLAGS).await {
                Ok(mut msgs) => {
                    let mut remote_message_id = None;

                    while let Some(response) = msgs.next().await {
                        match response {
                            Ok(fetch) => {
                                if fetch.uid == Some(uid) {
                                    remote_message_id = get_fetch_headers(&fetch)
                                        .and_then(|headers| prefetch_get_message_id(&headers))
                                        .ok();
                                }
                            }
                            Err(err) => {
                                warn!(context, "IMAP fetch error {}", err);
                                return ImapActionResult::RetryLater;
                            }
                        }
                    }

                    if let Some(remote_message_id) = remote_message_id {
                        if remote_message_id != message_id {
                            warn!(
                                context,
                                "Cannot delete on IMAP, {}: remote message-id '{}' != '{}'",
                                display_imap_id,
                                remote_message_id,
                                message_id,
                            );
                            return ImapActionResult::Failed;
                        }
                    } else {
                        warn!(
                            context,
                            "Cannot delete on IMAP, {}: imap entry gone '{}'",
                            display_imap_id,
                            message_id,
                        );
                        return ImapActionResult::AlreadyDone;
                    }
                }
                Err(err) => {
                    warn!(
                        context,
                        "Cannot delete on IMAP, {}: {}", display_imap_id, err,
                    );
                    return ImapActionResult::RetryLater;
                }
            }
        }

        // mark the message for deletion
        if !self.add_flag_finalized(context, uid, "\\Deleted").await {
            warn!(
                context,
                "Cannot mark message {} as \"Deleted\".", display_imap_id
            );
            ImapActionResult::RetryLater
        } else {
            emit_event!(
                context,
                EventType::ImapMessageDeleted(format!(
                    "IMAP Message {} marked as deleted [{}]",
                    display_imap_id, message_id
                ))
            );
            self.config.selected_folder_needs_expunge = true;
            ImapActionResult::Success
        }
    }

    pub async fn ensure_configured_folders(
        &mut self,
        context: &Context,
        create_mvbox: bool,
    ) -> Result<()> {
        let folders_configured = context
            .sql
            .get_raw_config_int(context, "folders_configured")
            .await;
        if folders_configured.unwrap_or_default() >= DC_FOLDERS_CONFIGURED_VERSION {
            return Ok(());
        }

        self.configure_folders(context, create_mvbox).await
    }

    pub async fn configure_folders(&mut self, context: &Context, create_mvbox: bool) -> Result<()> {
        if !self.is_connected() {
            bail!("IMAP No Connection established");
        }

        if let Some(ref mut session) = &mut self.session {
            let mut folders = match session.list(Some(""), Some("*")).await {
                Ok(f) => f,
                Err(err) => {
                    bail!("list_folders failed: {}", err);
                }
            };

            let mut delimiter = ".".to_string();
            let mut delimiter_is_default = true;
            let mut sentbox_folder = None;
            let mut mvbox_folder = None;
            let mut fallback_folder = get_fallback_folder(&delimiter);

            while let Some(folder) = folders.next().await {
                let folder = folder?;
                info!(context, "Scanning folder: {:?}", folder);

                // Update the delimiter iff there is a different one, but only once.
                if let Some(d) = folder.delimiter() {
                    if delimiter_is_default && !d.is_empty() && delimiter != d {
                        delimiter = d.to_string();
                        fallback_folder = get_fallback_folder(&delimiter);
                        delimiter_is_default = false;
                    }
                }

                if folder.name() == "DeltaChat" {
                    // Always takes precendent
                    mvbox_folder = Some(folder.name().to_string());
                } else if folder.name() == fallback_folder {
                    // only set iff none has been already set
                    if mvbox_folder.is_none() {
                        mvbox_folder = Some(folder.name().to_string());
                    }
                } else if let FolderMeaning::SentObjects = get_folder_meaning(&folder) {
                    // Always takes precedent
                    sentbox_folder = Some(folder.name().to_string());
                } else if let FolderMeaning::SentObjects =
                    get_folder_meaning_by_name(&folder.name())
                {
                    // only set iff none has been already set
                    if sentbox_folder.is_none() {
                        sentbox_folder = Some(folder.name().to_string());
                    }
                }
            }
            drop(folders);

            info!(context, "Using \"{}\" as folder-delimiter.", delimiter);
            info!(context, "sentbox folder is {:?}", sentbox_folder);

            if mvbox_folder.is_none() && create_mvbox {
                info!(context, "Creating MVBOX-folder \"DeltaChat\"...",);

                match session.create("DeltaChat").await {
                    Ok(_) => {
                        mvbox_folder = Some("DeltaChat".into());
                        info!(context, "MVBOX-folder created.",);
                    }
                    Err(err) => {
                        warn!(
                            context,
                            "Cannot create MVBOX-folder, trying to create INBOX subfolder. ({})",
                            err
                        );

                        match session.create(&fallback_folder).await {
                            Ok(_) => {
                                mvbox_folder = Some(fallback_folder);
                                info!(
                                    context,
                                    "MVBOX-folder created as INBOX subfolder. ({})", err
                                );
                            }
                            Err(err) => {
                                warn!(context, "Cannot create MVBOX-folder. ({})", err);
                            }
                        }
                    }
                }
                // SUBSCRIBE is needed to make the folder visible to the LSUB command
                // that may be used by other MUAs to list folders.
                // for the LIST command, the folder is always visible.
                if let Some(ref mvbox) = mvbox_folder {
                    if let Err(err) = session.subscribe(mvbox).await {
                        warn!(context, "could not subscribe to {:?}: {:?}", mvbox, err);
                    }
                }
            }
            context
                .set_config(Config::ConfiguredInboxFolder, Some("INBOX"))
                .await?;
            if let Some(ref mvbox_folder) = mvbox_folder {
                context
                    .set_config(Config::ConfiguredMvboxFolder, Some(mvbox_folder))
                    .await?;
            }
            if let Some(ref sentbox_folder) = sentbox_folder {
                context
                    .set_config(Config::ConfiguredSentboxFolder, Some(sentbox_folder))
                    .await?;
            }
            context
                .sql
                .set_raw_config_int(context, "folders_configured", DC_FOLDERS_CONFIGURED_VERSION)
                .await?;
        }
        info!(context, "FINISHED configuring IMAP-folders.");
        Ok(())
    }
}

/// Try to get the folder meaning by the name of the folder only used if the server does not support XLIST.
// TODO: lots languages missing - maybe there is a list somewhere on other MUAs?
// however, if we fail to find out the sent-folder,
// only watching this folder is not working. at least, this is no show stopper.
// CAVE: if possible, take care not to add a name here that is "sent" in one language
// but sth. different in others - a hard job.
fn get_folder_meaning_by_name(folder_name: &str) -> FolderMeaning {
    // source: https://stackoverflow.com/questions/2185391/localized-gmail-imap-folders
    let sent_names = vec![
        "sent",
        "sentmail",
        "sent objects",
        "gesendet",
        "Sent Mail",
        "Sendte e-mails",
        "Enviados",
        "Messages envoyés",
        "Messages envoyes",
        "Posta inviata",
        "Verzonden berichten",
        "Wyslane",
        "E-mails enviados",
        "Correio enviado",
        "Enviada",
        "Enviado",
        "Gönderildi",
        "Inviati",
        "Odeslaná pošta",
        "Sendt",
        "Skickat",
        "Verzonden",
        "Wysłane",
        "Éléments envoyés",
        "Απεσταλμένα",
        "Отправленные",
        "寄件備份",
        "已发送邮件",
        "送信済み",
        "보낸편지함",
    ];
    let lower = folder_name.to_lowercase();

    if sent_names.into_iter().any(|s| s.to_lowercase() == lower) {
        FolderMeaning::SentObjects
    } else {
        FolderMeaning::Unknown
    }
}

fn get_folder_meaning(folder_name: &Name) -> FolderMeaning {
    let special_names = vec!["\\Spam", "\\Trash", "\\Drafts", "\\Junk"];

    for attr in folder_name.attributes() {
        if let NameAttribute::Custom(ref label) = attr {
            if special_names.iter().any(|s| *s == label) {
                return FolderMeaning::Other;
            } else if label == "\\Sent" {
                return FolderMeaning::SentObjects;
            }
        }
    }
    FolderMeaning::Unknown
}

async fn precheck_imf(
    context: &Context,
    rfc724_mid: &str,
    server_folder: &str,
    server_uid: u32,
) -> Result<bool> {
    if let Some((old_server_folder, old_server_uid, msg_id)) =
        message::rfc724_mid_exists(context, &rfc724_mid).await?
    {
        if old_server_folder.is_empty() && old_server_uid == 0 {
            info!(
                context,
                "[move] detected bcc-self {} as {}/{}", rfc724_mid, server_folder, server_uid
            );

            let delete_server_after = context.get_config_delete_server_after().await;

            if delete_server_after != Some(0) {
                if msg_id
                    .needs_move(context, server_folder)
                    .await
                    .unwrap_or_default()
                {
                    // If the bcc-self message is not moved, directly
                    // add MarkSeen job, otherwise MarkSeen job is
                    // added after the Move Job completed.
                    job::add(
                        context,
                        job::Job::new(Action::MoveMsg, msg_id.to_u32(), Params::new(), 0),
                    )
                    .await;
                } else {
                    job::add(
                        context,
                        job::Job::new(Action::MarkseenMsgOnImap, msg_id.to_u32(), Params::new(), 0),
                    )
                    .await;
                }
            }
        } else if old_server_folder != server_folder {
            info!(
                context,
                "[move] detected message {} moved by other device from {}/{} to {}/{}",
                rfc724_mid,
                old_server_folder,
                old_server_uid,
                server_folder,
                server_uid
            );
        } else if old_server_uid == 0 {
            info!(
                context,
                "[move] detected message {} moved by us from {}/{} to {}/{}",
                rfc724_mid,
                old_server_folder,
                old_server_uid,
                server_folder,
                server_uid
            );
        } else if old_server_uid != server_uid {
            warn!(
                context,
                "UID for message {} in folder {} changed from {} to {}",
                rfc724_mid,
                server_folder,
                old_server_uid,
                server_uid
            );
        }

        if old_server_folder != server_folder || old_server_uid != server_uid {
            update_server_uid(context, rfc724_mid, server_folder, server_uid).await;
            if let Ok(message_state) = msg_id.get_state(context).await {
                if message_state == MessageState::InSeen || message_state.is_outgoing() {
                    job::add(
                        context,
                        job::Job::new(Action::MarkseenMsgOnImap, msg_id.to_u32(), Params::new(), 0),
                    )
                    .await;
                }
            }
            info!(context, "Updating server_uid and adding markseen job");
        }
        Ok(true)
    } else {
        Ok(false)
    }
}

fn get_fetch_headers(prefetch_msg: &Fetch) -> Result<Vec<mailparse::MailHeader>> {
    let header_bytes = match prefetch_msg.header() {
        Some(header_bytes) => header_bytes,
        None => return Ok(Vec::new()),
    };
    let (headers, _) = mailparse::parse_headers(header_bytes)?;
    Ok(headers)
}

fn prefetch_get_message_id(headers: &[mailparse::MailHeader]) -> Result<String> {
    if let Some(message_id) = headers.get_header_value(HeaderDef::MessageId) {
        Ok(crate::mimeparser::parse_message_id(&message_id)?)
    } else {
        bail!("prefetch: No message ID found");
    }
}

async fn prefetch_is_reply_to_chat_message(
    context: &Context,
    headers: &[mailparse::MailHeader<'_>],
) -> bool {
    if let Some(value) = headers.get_header_value(HeaderDef::InReplyTo) {
        if is_msgrmsg_rfc724_mid_in_list(context, &value).await {
            return true;
        }
    }

    if let Some(value) = headers.get_header_value(HeaderDef::References) {
        if is_msgrmsg_rfc724_mid_in_list(context, &value).await {
            return true;
        }
    }

    false
}

pub(crate) async fn prefetch_should_download(
    context: &Context,
    headers: &[mailparse::MailHeader<'_>],
    show_emails: ShowEmails,
) -> Result<bool> {
    if let Some(rfc724_mid) = headers.get_header_value(HeaderDef::MessageId) {
        if let Some(group_id) = dc_extract_grpid_from_rfc724_mid(&rfc724_mid) {
            if let Ok((chat_id, _, _)) = get_chat_id_by_grpid(context, group_id).await {
                if !chat_id.is_unset() {
                    // This might be a group command, like removing a group member.
                    // We really need to fetch this to avoid inconsistent group state.
                    return Ok(true);
                }
            }
        }
    }

    let is_chat_message = headers.get_header_value(HeaderDef::ChatVersion).is_some();
    let is_reply_to_chat_message = prefetch_is_reply_to_chat_message(context, &headers).await;

    let maybe_ndn = if let Some(from) = headers.get_header_value(HeaderDef::From_) {
        let from = from.to_ascii_lowercase();
        from.contains("mailer-daemon") || from.contains("mail-daemon")
    } else {
        false
    };

    // Autocrypt Setup Message should be shown even if it is from non-chat client.
    let is_autocrypt_setup_message = headers
        .get_header_value(HeaderDef::AutocryptSetupMessage)
        .is_some();

    let (_contact_id, blocked_contact, origin) =
        from_field_to_contact_id(context, &mimeparser::get_from(headers)).await?;
    let accepted_contact = origin.is_known();

    let show = is_autocrypt_setup_message
        || match show_emails {
            ShowEmails::Off => is_chat_message || is_reply_to_chat_message,
            ShowEmails::AcceptedContacts => {
                is_chat_message || is_reply_to_chat_message || accepted_contact
            }
            ShowEmails::All => true,
        };
    let should_download = (show && !blocked_contact) || maybe_ndn;
    Ok(should_download)
}

async fn message_needs_processing(
    context: &Context,
    current_uid: u32,
    headers: &[mailparse::MailHeader<'_>],
    msg_id: &str,
    folder: &str,
    show_emails: ShowEmails,
) -> bool {
    let skip = match precheck_imf(context, &msg_id, folder, current_uid).await {
        Ok(skip) => skip,
        Err(err) => {
            warn!(context, "precheck_imf error: {}", err);
            true
        }
    };

    if skip {
        // we know the message-id already or don't want the message otherwise.
        info!(
            context,
            "Skipping message {} from \"{}\" by precheck.", msg_id, folder,
        );
        return false;
    }

    // we do not know the message-id
    // or the message-id is missing (in this case, we create one in the further process)
    // or some other error happened
    let show = match prefetch_should_download(context, &headers, show_emails).await {
        Ok(show) => show,
        Err(err) => {
            warn!(context, "prefetch_should_download error: {}", err);
            true
        }
    };

    if !show {
        info!(
            context,
            "Ignoring new message {} from \"{}\".", msg_id, folder,
        );
        return false;
    }

    true
}

fn get_fallback_folder(delimiter: &str) -> String {
    format!("INBOX{}DeltaChat", delimiter)
}

pub async fn set_config_last_seen_uid<S: AsRef<str>>(
    context: &Context,
    folder: S,
    uidvalidity: u32,
    lastseenuid: u32,
) {
    let key = format!("imap.mailbox.{}", folder.as_ref());
    let val = format!("{}:{}", uidvalidity, lastseenuid);

    context
        .sql
        .set_raw_config(context, &key, Some(&val))
        .await
        .ok();
}

async fn get_config_last_seen_uid<S: AsRef<str>>(context: &Context, folder: S) -> (u32, u32) {
    let key = format!("imap.mailbox.{}", folder.as_ref());
    if let Some(entry) = context.sql.get_raw_config(context, &key).await {
        // the entry has the format `imap.mailbox.<folder>=<uidvalidity>:<lastseenuid>`
        let mut parts = entry.split(':');
        (
            parts.next().unwrap_or_default().parse().unwrap_or(0),
            parts.next().unwrap_or_default().parse().unwrap_or(0),
        )
    } else {
        (0, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_get_folder_meaning_by_name() {
        assert_eq!(
            get_folder_meaning_by_name("Gesendet"),
            FolderMeaning::SentObjects
        );
        assert_eq!(
            get_folder_meaning_by_name("GESENDET"),
            FolderMeaning::SentObjects
        );
        assert_eq!(
            get_folder_meaning_by_name("gesendet"),
            FolderMeaning::SentObjects
        );
        assert_eq!(
            get_folder_meaning_by_name("Messages envoyés"),
            FolderMeaning::SentObjects
        );
        assert_eq!(
            get_folder_meaning_by_name("mEsSaGes envoyÉs"),
            FolderMeaning::SentObjects
        );
        assert_eq!(get_folder_meaning_by_name("xxx"), FolderMeaning::Unknown);
    }
}
