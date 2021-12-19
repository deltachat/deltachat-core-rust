//! # IMAP handling module.
//!
//! uses [async-email/async-imap](https://github.com/async-email/async-imap)
//! to implement connect, fetch, delete functionality with standard IMAP servers.

use std::{cmp, cmp::max, collections::BTreeMap};

use anyhow::{anyhow, bail, format_err, Context as _, Result};
use async_imap::types::{
    Fetch, Flag, Mailbox, Name, NameAttribute, Quota, QuotaRoot, UnsolicitedResponse,
};
use async_std::channel::Receiver;
use async_std::prelude::*;
use num_traits::FromPrimitive;

use crate::chat;
use crate::chat::ChatIdBlocked;
use crate::constants::{
    Blocked, Chattype, ShowEmails, Viewtype, DC_CONTACT_ID_SELF, DC_FETCH_EXISTING_MSGS_COUNT,
    DC_FOLDERS_CONFIGURED_VERSION, DC_LP_AUTH_OAUTH2,
};
use crate::context::Context;
use crate::dc_receive_imf::{
    dc_receive_imf_inner, from_field_to_contact_id, get_prefetch_parent_message, ReceivedMsg,
};
use crate::ephemeral::delete_expired_imap_messages;
use crate::events::EventType;
use crate::headerdef::{HeaderDef, HeaderDefMap};
use crate::job::{self, Action};
use crate::login_param::{CertificateChecks, LoginParam, ServerLoginParam};
use crate::login_param::{ServerAddress, Socks5Config};
use crate::message::{self, MessengerMessage};
use crate::mimeparser;
use crate::oauth2::dc_get_oauth2_access_token;
use crate::param::Params;
use crate::provider::Socket;
use crate::scheduler::InterruptInfo;
use crate::stock_str;
use crate::{config::Config, scheduler::connectivity::ConnectivityStore};

mod client;
mod idle;
pub mod scan_folders;
pub mod select_folder;
mod session;

use client::Client;
use mailparse::SingleInfo;
use message::Message;
use session::Session;

use self::select_folder::NewlySelected;

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq)]
pub enum ImapActionResult {
    Failed,
    RetryLater,
    Success,
}

/// Prefetch:
/// - Message-ID to check if we already have the message.
/// - In-Reply-To and References to check if message is a reply to chat message.
/// - Chat-Version to check if a message is a chat message
/// - Autocrypt-Setup-Message to check if a message is an autocrypt setup message,
///   not necessarily sent by Delta Chat.
const PREFETCH_FLAGS: &str = "(UID RFC822.SIZE BODY.PEEK[HEADER.FIELDS (\
                              MESSAGE-ID \
                              FROM \
                              IN-REPLY-TO REFERENCES \
                              CHAT-VERSION \
                              AUTOCRYPT-SETUP-MESSAGE\
                              )])";
const RFC724MID_UID: &str = "(UID BODY.PEEK[HEADER.FIELDS (\
                             MESSAGE-ID \
                             X-MICROSOFT-ORIGINAL-MESSAGE-ID\
                             )])";
const JUST_UID: &str = "(UID)";
const BODY_FULL: &str = "(FLAGS BODY.PEEK[])";
const BODY_PARTIAL: &str = "(FLAGS RFC822.SIZE BODY.PEEK[HEADER])";

#[derive(Debug)]
pub struct Imap {
    idle_interrupt: Receiver<InterruptInfo>,
    config: ImapConfig,
    session: Option<Session>,
    should_reconnect: bool,
    login_failed_once: bool,

    /// True if CAPABILITY command was run successfully once and config.can_* contain correct
    /// values.
    capabilities_determined: bool,

    pub(crate) connectivity: ConnectivityStore,
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

#[derive(Debug, PartialEq, Clone, Copy)]
enum FolderMeaning {
    Unknown,
    Spam,
    Sent,
    Drafts,
    Other,
}

impl FolderMeaning {
    fn to_config(self) -> Option<Config> {
        match self {
            FolderMeaning::Unknown => None,
            FolderMeaning::Spam => Some(Config::ConfiguredSpamFolder),
            FolderMeaning::Sent => Some(Config::ConfiguredSentboxFolder),
            FolderMeaning::Drafts => None,
            FolderMeaning::Other => None,
        }
    }
}

#[derive(Debug)]
struct ImapConfig {
    pub addr: String,
    pub lp: ServerLoginParam,
    pub socks5_config: Option<Socks5Config>,
    pub strict_tls: bool,
    pub oauth2: bool,
    pub selected_folder: Option<String>,
    pub selected_mailbox: Option<Mailbox>,
    pub selected_folder_needs_expunge: bool,

    pub can_idle: bool,

    /// True if the server has MOVE capability as defined in
    /// <https://tools.ietf.org/html/rfc6851>
    pub can_move: bool,

    /// True if the server has QUOTA capability as defined in
    /// <https://tools.ietf.org/html/rfc2087>
    pub can_check_quota: bool,
}

impl Imap {
    /// Creates new disconnected IMAP client using the specific login parameters.
    ///
    /// `addr` is used to renew token if OAuth2 authentication is used.
    pub async fn new(
        lp: &ServerLoginParam,
        socks5_config: Option<Socks5Config>,
        addr: &str,
        oauth2: bool,
        provider_strict_tls: bool,
        idle_interrupt: Receiver<InterruptInfo>,
    ) -> Result<Self> {
        if lp.server.is_empty() || lp.user.is_empty() || lp.password.is_empty() {
            bail!("Incomplete IMAP connection parameters");
        }

        let strict_tls = match lp.certificate_checks {
            CertificateChecks::Automatic => provider_strict_tls,
            CertificateChecks::Strict => true,
            CertificateChecks::AcceptInvalidCertificates
            | CertificateChecks::AcceptInvalidCertificates2 => false,
        };
        let config = ImapConfig {
            addr: addr.to_string(),
            lp: lp.clone(),
            socks5_config,
            strict_tls,
            oauth2,
            selected_folder: None,
            selected_mailbox: None,
            selected_folder_needs_expunge: false,
            can_idle: false,
            can_move: false,
            can_check_quota: false,
        };

        let imap = Imap {
            idle_interrupt,
            config,
            session: None,
            should_reconnect: false,
            login_failed_once: false,
            connectivity: Default::default(),
            capabilities_determined: false,
        };

        Ok(imap)
    }

    /// Creates new disconnected IMAP client using configured parameters.
    pub async fn new_configured(
        context: &Context,
        idle_interrupt: Receiver<InterruptInfo>,
    ) -> Result<Self> {
        if !context.is_configured().await? {
            bail!("IMAP Connect without configured params");
        }

        let param = LoginParam::from_database(context, "configured_").await?;
        // the trailing underscore is correct

        let imap = Self::new(
            &param.imap,
            param.socks5_config.clone(),
            &param.addr,
            param.server_flags & DC_LP_AUTH_OAUTH2 != 0,
            param
                .provider
                .map_or(param.socks5_config.is_some(), |provider| {
                    provider.strict_tls
                }),
            idle_interrupt,
        )
        .await?;
        Ok(imap)
    }

    /// Connects or reconnects if needed.
    ///
    /// It is safe to call this function if already connected, actions are performed only as needed.
    ///
    /// Calling this function is not enough to perform IMAP operations. Use [`Imap::prepare`]
    /// instead if you are going to actually use connection rather than trying connection
    /// parameters.
    pub async fn connect(&mut self, context: &Context) -> Result<()> {
        if self.config.lp.server.is_empty() {
            bail!("IMAP operation attempted while it is torn down");
        }

        if self.should_reconnect() {
            self.disconnect(context).await;
            self.should_reconnect = false;
        } else if self.session.is_some() {
            return Ok(());
        }

        self.connectivity.set_connecting(context).await;

        let oauth2 = self.config.oauth2;

        let connection_res: Result<Client> = if self.config.lp.security == Socket::Starttls
            || self.config.lp.security == Socket::Plain
        {
            let config = &mut self.config;
            let imap_server: &str = config.lp.server.as_ref();
            let imap_port = config.lp.port;

            let connection = if let Some(socks5_config) = &config.socks5_config {
                Client::connect_insecure_socks5(
                    &ServerAddress {
                        host: imap_server.to_string(),
                        port: imap_port,
                    },
                    socks5_config.clone(),
                )
                .await
            } else {
                Client::connect_insecure((imap_server, imap_port)).await
            };

            match connection {
                Ok(client) => {
                    if config.lp.security == Socket::Starttls {
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

            if let Some(socks5_config) = &config.socks5_config {
                Client::connect_secure_socks5(
                    &ServerAddress {
                        host: imap_server.to_string(),
                        port: imap_port,
                    },
                    config.strict_tls,
                    socks5_config.clone(),
                )
                .await
            } else {
                Client::connect_secure((imap_server, imap_port), imap_server, config.strict_tls)
                    .await
            }
        };

        let login_res = match connection_res {
            Ok(client) => {
                let config = &self.config;
                let imap_user: &str = config.lp.user.as_ref();
                let imap_pw: &str = config.lp.password.as_ref();

                if oauth2 {
                    let addr: &str = config.addr.as_ref();

                    if let Some(token) =
                        dc_get_oauth2_access_token(context, addr, imap_pw, true).await?
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
                self.session = Some(session);
                self.login_failed_once = false;
                context.emit_event(EventType::ImapConnected(format!(
                    "IMAP-LOGIN as {}",
                    self.config.lp.user
                )));
                Ok(())
            }

            Err(err) => {
                let imap_user = self.config.lp.user.to_owned();
                let message = stock_str::cannot_login(context, &imap_user).await;

                warn!(context, "{} ({})", message, err);

                let lock = context.wrong_pw_warning_mutex.lock().await;
                if self.login_failed_once
                    && err.to_string().to_lowercase().contains("authentication")
                    && context.get_config_bool(Config::NotifyAboutWrongPw).await?
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

                self.trigger_reconnect(context).await;
                Err(format_err!("{}\n\n{}", message, err))
            }
        }
    }

    /// Determine server capabilities if not done yet.
    async fn determine_capabilities(&mut self) -> Result<()> {
        if self.capabilities_determined {
            return Ok(());
        }

        match &mut self.session {
            Some(ref mut session) => match session.capabilities().await {
                Ok(caps) => {
                    self.config.can_idle = caps.has_str("IDLE");
                    self.config.can_move = caps.has_str("MOVE");
                    self.config.can_check_quota = caps.has_str("QUOTA");
                    self.capabilities_determined = true;
                    Ok(())
                }
                Err(err) => {
                    bail!("CAPABILITY command error: {}", err);
                }
            },
            None => {
                bail!("Can't determine server capabilities because connection was not established")
            }
        }
    }

    /// Prepare for IMAP operation.
    ///
    /// Ensure that IMAP client is connected, folders are created and IMAP capabilities are
    /// determined.
    pub async fn prepare(&mut self, context: &Context) -> Result<()> {
        if let Err(err) = self.connect(context).await {
            self.connectivity.set_err(context, &err).await;
            return Err(err);
        }

        self.ensure_configured_folders(context, true).await?;
        self.determine_capabilities().await?;
        Ok(())
    }

    async fn disconnect(&mut self, context: &Context) {
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
        self.capabilities_determined = false;
        self.config.selected_folder = None;
        self.config.selected_mailbox = None;
    }

    pub fn should_reconnect(&self) -> bool {
        self.should_reconnect
    }

    pub async fn trigger_reconnect(&mut self, context: &Context) {
        self.connectivity.set_connecting(context).await;
        self.should_reconnect = true;
    }

    /// FETCH-MOVE-DELETE iteration.
    ///
    /// Prefetches headers and downloads new message from the folder, moves messages away from the
    /// folder and deletes messages in the folder.
    pub async fn fetch_move_delete(&mut self, context: &Context, watch_folder: &str) -> Result<()> {
        if !context.sql.is_open().await {
            // probably shutdown
            bail!("IMAP operation attempted while it is torn down");
        }
        self.prepare(context).await?;

        self.fetch_new_messages(context, watch_folder, false)
            .await
            .context("fetch_new_messages")?;

        // Mark expired messages for deletion.
        delete_expired_imap_messages(context).await?;

        self.move_messages(context, watch_folder)
            .await
            .context("move_messages")?;
        self.delete_messages(context, watch_folder)
            .await
            .context("delete_messages")?;

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
            .transaction(move |transaction| {
                transaction.execute(
                    "UPDATE msgs SET server_uid=0 WHERE server_folder=?",
                    params![folder],
                )?;
                for (uid, rfc724_mid) in &msg_ids {
                    // This may detect previously undetected moved
                    // messages, so we update server_folder too.
                    transaction.execute(
                        "UPDATE msgs \
                             SET server_folder=?,server_uid=? WHERE rfc724_mid=?",
                        params![folder, uid, rfc724_mid],
                    )?;
                }
                Ok(())
            })
            .await?;
        Ok(())
    }

    /// Select a folder and take care of uidvalidity changes.
    /// Also, when selecting a folder for the first time, sets the uid_next to the current
    /// mailbox.uid_next so that no old emails are fetched.
    /// Returns Result<new_emails> (i.e. whether new emails arrived),
    /// if in doubt, returns new_emails=true so emails are fetched.
    pub(crate) async fn select_with_uidvalidity(
        &mut self,
        context: &Context,
        folder: &str,
    ) -> Result<bool> {
        let newly_selected = self.select_or_create_folder(context, folder).await?;

        let mailbox = &mut self.config.selected_mailbox.as_ref();
        let mailbox =
            mailbox.with_context(|| format!("No mailbox selected, folder: {}", folder))?;

        let new_uid_validity = mailbox
            .uid_validity
            .with_context(|| format!("No UIDVALIDITY for folder {}", folder))?;

        let old_uid_validity = get_uidvalidity(context, folder).await?;
        let old_uid_next = get_uid_next(context, folder).await?;

        if new_uid_validity == old_uid_validity {
            let new_emails = if newly_selected == NewlySelected::No {
                // The folder was not newly selected i.e. no SELECT command was run. This means that mailbox.uid_next
                // was not updated and may contain an incorrect value. So, just return true so that
                // the caller tries to fetch new messages (we could of course run a SELECT command now, but trying to fetch
                // new messages is only one command, just as a SELECT command)
                true
            } else if let Some(uid_next) = mailbox.uid_next {
                if uid_next < old_uid_next {
                    warn!(
                        context,
                        "The server illegally decreased the uid_next of folder {} from {} to {} without changing validity ({}), resyncing UIDs...", 
                        folder, old_uid_next, uid_next, new_uid_validity,
                    );
                    set_uid_next(context, folder, uid_next).await?;
                    job::schedule_resync(context).await?;
                }
                uid_next != old_uid_next // If uid_next changed, there are new emails
            } else {
                true // We have no uid_next and if in doubt, return true
            };
            return Ok(new_emails);
        }

        if mailbox.exists == 0 {
            info!(context, "Folder \"{}\" is empty.", folder);

            // set uid_next=1 for empty folders.
            // If we do not do this here, we'll miss the first message
            // as we will get in here again and fetch from uid_next then.
            // Also, the "fall back to fetching" below would need a non-zero mailbox.exists to work.
            set_uid_next(context, folder, 1).await?;
            set_uidvalidity(context, folder, new_uid_validity).await?;
            return Ok(false);
        }

        // ==============  uid_validity has changed or is being set the first time.  ==============

        let new_uid_next = match mailbox.uid_next {
            Some(uid_next) => uid_next,
            None => {
                warn!(
                    context,
                    "IMAP folder has no uid_next, fall back to fetching"
                );
                let session = self.session.as_mut().context("Get uid_next: Nosession")?;
                // note that we use fetch by sequence number
                // and thus we only need to get exactly the
                // last-index message.
                let set = format!("{}", mailbox.exists);
                let mut list = session
                    .fetch(set, JUST_UID)
                    .await
                    .context("Error fetching UID")?;

                let mut new_last_seen_uid = None;
                while let Some(fetch) = list.next().await.transpose()? {
                    if fetch.message == mailbox.exists && fetch.uid.is_some() {
                        new_last_seen_uid = fetch.uid;
                    }
                }
                new_last_seen_uid.context("select: failed to fetch")? + 1
            }
        };

        set_uid_next(context, folder, new_uid_next).await?;
        set_uidvalidity(context, folder, new_uid_validity).await?;

        // Collect garbage entries in `imap` table.
        context
            .sql
            .execute(
                "DELETE FROM imap WHERE folder=? AND uidvalidity!=?",
                paramsv![folder, new_uid_validity],
            )
            .await?;

        if old_uid_validity != 0 || old_uid_next != 0 {
            job::schedule_resync(context).await?;
        }
        info!(
            context,
            "uid/validity change folder {}: new {}/{} previous {}/{}",
            folder,
            new_uid_next,
            new_uid_validity,
            old_uid_next,
            old_uid_validity,
        );
        Ok(false)
    }

    pub(crate) async fn fetch_new_messages(
        &mut self,
        context: &Context,
        folder: &str,
        fetch_existing_msgs: bool,
    ) -> Result<bool> {
        let new_emails = self.select_with_uidvalidity(context, folder).await?;

        if !new_emails && !fetch_existing_msgs {
            info!(context, "No new emails in folder {}", folder);
            return Ok(false);
        }

        let uid_validity = get_uidvalidity(context, folder).await?;
        let old_uid_next = get_uid_next(context, folder).await?;

        let msgs = if fetch_existing_msgs {
            self.prefetch_existing_msgs().await?
        } else {
            self.prefetch(old_uid_next).await?
        };
        let read_cnt = msgs.len();

        let show_emails = ShowEmails::from_i32(context.get_config_int(Config::ShowEmails).await?)
            .unwrap_or_default();
        let download_limit = context.download_limit().await?;
        let mut uids_fetch_fully = Vec::with_capacity(msgs.len());
        let mut uids_fetch_partially = Vec::with_capacity(msgs.len());
        let mut largest_uid_skipped = None;

        // Store the info about IMAP messages in the database.
        for (uid, ref fetch_response) in msgs {
            let headers = match get_fetch_headers(fetch_response) {
                Ok(headers) => headers,
                Err(err) => {
                    warn!(context, "Failed to parse FETCH headers: {}", err);
                    continue;
                }
            };

            let message_id = prefetch_get_message_id(&headers).unwrap_or_default();

            let target = match target_folder(context, folder, &headers).await? {
                Some(config) => match context.get_config(config).await? {
                    Some(target) => target,
                    None => folder.to_string(),
                },
                None => folder.to_string(),
            };

            let duplicate = context
                .sql
                .count(
                    "SELECT COUNT(*)
                     FROM imap
                     WHERE rfc724_mid=?
                     AND folder=?
                     AND uid<?",
                    paramsv![message_id, &target, uid],
                )
                .await?
                > 0;

            context
                .sql
                .execute(
                    "INSERT INTO imap (rfc724_mid, folder, uid, uidvalidity, target)
                       VALUES         (?1,         ?2,     ?3,  ?4,          ?5)
                       ON CONFLICT(folder, uid, uidvalidity)
                       DO UPDATE SET rfc724_mid=excluded.rfc724_mid,
                                     target=excluded.target",
                    paramsv![
                        message_id,
                        folder,
                        uid,
                        uid_validity,
                        if duplicate { "" } else { &target }
                    ],
                )
                .await?;

            // Download only the messages which have reached their target folder if there are
            // multiple devices. This prevents race conditions in multidevice case, where one
            // device tries to download the message while another device moves the message at the
            // same time. Even in single device case it is possible to fail downloading the first
            // message, move it to the movebox and then download the second message before
            // downloading the first one, if downloading from inbox before moving is allowed.
            if folder == target
                && prefetch_should_download(
                    context,
                    &headers,
                    &message_id,
                    fetch_response.flags(),
                    show_emails,
                )
                .await?
            {
                match download_limit {
                    Some(download_limit) => {
                        if fetch_response.size.unwrap_or_default() > download_limit {
                            uids_fetch_partially.push(uid);
                        } else {
                            uids_fetch_fully.push(uid)
                        }
                    }
                    None => uids_fetch_fully.push(uid),
                }
            } else {
                largest_uid_skipped = Some(uid);
            }
        }

        if !uids_fetch_fully.is_empty() || !uids_fetch_partially.is_empty() {
            self.connectivity.set_working(context).await;
        }

        // Actually download messages.
        let (largest_uid_fully_fetched, mut received_msgs) = self
            .fetch_many_msgs(
                context,
                folder,
                uids_fetch_fully,
                false,
                fetch_existing_msgs,
            )
            .await;

        let (largest_uid_partially_fetched, received_msgs_2) = self
            .fetch_many_msgs(
                context,
                folder,
                uids_fetch_partially,
                true,
                fetch_existing_msgs,
            )
            .await;
        received_msgs.extend(received_msgs_2);

        // determine which uid_next to use to update to
        // dc_receive_imf() returns an `Err` value only on recoverable errors, otherwise it just logs an error.
        // `largest_uid_processed` is the largest uid where dc_receive_imf() did NOT return an error.

        // So: Update the uid_next to the largest uid that did NOT recoverably fail. Not perfect because if there was
        // another message afterwards that succeeded, we will not retry. The upside is that we will not retry an infinite amount of times.
        let largest_uid_without_errors = max(
            max(
                largest_uid_fully_fetched.unwrap_or(0),
                largest_uid_partially_fetched.unwrap_or(0),
            ),
            largest_uid_skipped.unwrap_or(0),
        );
        let new_uid_next = largest_uid_without_errors + 1;

        if new_uid_next > old_uid_next {
            set_uid_next(context, folder, new_uid_next).await?;
        }

        info!(context, "{} mails read from \"{}\".", read_cnt, folder);

        chat::mark_old_messages_as_noticed(context, received_msgs).await?;

        Ok(read_cnt > 0)
    }

    /// Moves messages.
    ///
    /// This is the only place where messages are moved on the IMAP server.
    async fn move_messages(&mut self, context: &Context, folder: &str) -> Result<()> {
        let rows = context
            .sql
            .query_map(
                "SELECT id, uid, target FROM imap
        WHERE folder = ?
        AND target != folder
        AND target != '' -- Not planned for deletion.
        ORDER BY id",
                paramsv![folder],
                |row| {
                    let rowid: i64 = row.get(0)?;
                    let uid: u32 = row.get(1)?;
                    let target: String = row.get(2)?;
                    Ok((rowid, uid, target))
                },
                |rows| rows.collect::<Result<Vec<_>, _>>().map_err(Into::into),
            )
            .await?;

        self.prepare(context).await?;
        self.select_folder(context, Some(folder)).await?;

        for (rowid, uid, target) in rows {
            // TODO: batch moves of messages with the same destination.
            let set = uid.to_string();

            if self.config.can_move {
                if let Some(session) = &mut self.session {
                    match session.uid_mv(&set, &target).await {
                        Ok(_) => {
                            context.emit_event(EventType::ImapMessageMoved(format!(
                                "IMAP message {}/{} moved to {}",
                                folder, uid, target
                            )));
                            context
                                .sql
                                .execute("DELETE FROM imap WHERE id=?", paramsv![rowid])
                                .await?;
                            continue;
                        }
                        Err(async_imap::error::Error::No(text)) => {
                            // "NO" response, probably the message is moved already.
                            info!(
                                context,
                                "IMAP message {}/{} cannot be moved: {}", folder, uid, text
                            );
                            context
                                .sql
                                .execute("DELETE FROM imap WHERE id=?", paramsv![rowid])
                                .await?;
                            continue;
                        }
                        Err(err) => {
                            warn!(
                                context,
                                "Cannot move message, fallback to COPY/DELETE {}/{} to {}: {}",
                                folder,
                                uid,
                                target,
                                err
                            );
                        }
                    }
                } else {
                    bail!("No session while attempting to move the message");
                }
            } else {
                info!(
                    context,
                    "Server does not support MOVE, fallback to COPY/DELETE {}/{} to {}",
                    folder,
                    uid,
                    target
                );
            }

            // Server does not support MOVE or MOVE failed.
            // Copy the message to the destination folder and mark the record for deletion.
            if let Some(session) = &mut self.session {
                match session.uid_copy(&set, &target).await {
                    Ok(_) => {
                        context.emit_event(EventType::ImapMessageMoved(format!(
                            "IMAP message {}/{} copied to {}",
                            folder, uid, target
                        )));
                        // Plan deletion of the original message.
                        context
                            .sql
                            .execute("UPDATE imap SET target='' WHERE id=?", paramsv![rowid])
                            .await?;
                    }
                    Err(async_imap::error::Error::No(text)) => {
                        // "NO" response, probably the message is moved already.
                        info!(
                            context,
                            "IMAP message {}/{} cannot be copied: {}", folder, uid, text
                        );
                        context
                            .sql
                            .execute("DELETE FROM imap WHERE id=?", paramsv![rowid])
                            .await?;
                        continue;
                    }
                    Err(err) => {
                        warn!(
                            context,
                            "Could not copy message {}/{}: {}", folder, uid, err
                        );
                        // Break the loop to avoid moving messages out of order.
                        // We can't proceed until this message is moved or copied.
                        break;
                    }
                }
            } else {
                bail!("No session while attempting to copy the message");
            }
        }

        Ok(())
    }

    /// Deletes messages that are marked as planned for deletion in `imap` table.
    ///
    /// This is the only place where messages are deleted from the IMAP server.
    async fn delete_messages(&mut self, context: &Context, folder: &str) -> Result<()> {
        let rows = context
            .sql
            .query_map(
                "SELECT id, uid FROM imap
                WHERE folder=? AND target=''
                ORDER BY uid ASC
                LIMIT 50", // Do not try to delete too many messages at once.
                paramsv![folder],
                |row| {
                    let rowid: i64 = row.get(0)?;
                    let uid: u32 = row.get(1)?;
                    Ok((rowid, uid))
                },
                |rows| rows.collect::<Result<Vec<_>, _>>().map_err(Into::into),
            )
            .await?;

        if rows.is_empty() {
            return Ok(());
        }

        for (rowid, uid) in rows {
            match self.delete_msg(context, folder, uid).await {
                ImapActionResult::Failed | ImapActionResult::RetryLater => {
                    warn!(context, "Deletion of message {}/{} failed", folder, uid);
                    break;
                }
                ImapActionResult::Success => {
                    context
                        .sql
                        .execute("DELETE FROM imap WHERE id=?", paramsv![rowid])
                        .await?;
                }
            }
        }

        // Expunge folder if needed, e.g. if some jobs have
        // deleted messages on the server.
        if let Err(err) = self.maybe_close_folder(context).await {
            warn!(context, "failed to close folder: {:?}", err);
        }

        Ok(())
    }

    /// Gets the from, to and bcc addresses from all existing outgoing emails.
    pub async fn get_all_recipients(&mut self, context: &Context) -> Result<Vec<SingleInfo>> {
        if self.session.is_none() {
            bail!("IMAP No Connection established");
        }

        let session = self.session.as_mut().unwrap();
        let self_addr = context
            .get_config(Config::ConfiguredAddr)
            .await?
            .ok_or_else(|| format_err!("Not configured"))?;

        let search_command = format!("FROM \"{}\"", self_addr);
        let uids = session
            .uid_search(search_command)
            .await?
            .into_iter()
            .collect();

        let mut result = Vec::new();
        for uid_set in &build_sequence_sets(uids) {
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
                        if let Some(from) = mimeparser::get_from(&headers).first() {
                            if context.is_self_addr(&from.addr).await? {
                                result.extend(mimeparser::get_recipients(&headers));
                            }
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

    /// Prefetch all messages greater than or equal to `uid_next`. Return a list of fetch results.
    async fn prefetch(&mut self, uid_next: u32) -> Result<BTreeMap<u32, async_imap::types::Fetch>> {
        let session = self.session.as_mut();
        let session = session.context("fetch_after(): IMAP No Connection established")?;

        // fetch messages with larger UID than the last one seen
        let set = format!("{}:*", uid_next);
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
        // uid:* is interpreted the same way as *:uid.
        // See <https://tools.ietf.org/html/rfc3501#page-61> for
        // standard reference. Therefore, sometimes we receive
        // already seen messages and have to filter them out.
        let new_msgs = msgs.split_off(&uid_next);

        Ok(new_msgs)
    }

    /// Like fetch_after(), but not for new messages but existing ones (the DC_FETCH_EXISTING_MSGS_COUNT newest messages)
    async fn prefetch_existing_msgs(&mut self) -> Result<BTreeMap<u32, async_imap::types::Fetch>> {
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
    ///
    /// Returns the last uid fetch successfully and the info about each downloaded message.
    pub(crate) async fn fetch_many_msgs(
        &mut self,
        context: &Context,
        folder: &str,
        server_uids: Vec<u32>,
        fetch_partially: bool,
        fetching_existing_messages: bool,
    ) -> (Option<u32>, Vec<ReceivedMsg>) {
        let mut received_msgs = Vec::new();
        if server_uids.is_empty() {
            return (None, Vec::new());
        }

        let session = match self.session.as_mut() {
            Some(session) => session,
            None => {
                warn!(context, "Not connected");
                return (None, Vec::new());
            }
        };

        let sets = build_sequence_sets(server_uids.clone());
        let mut count = 0;
        let mut last_uid = None;

        for set in sets.iter() {
            let mut msgs = match session
                .uid_fetch(
                    &set,
                    if fetch_partially {
                        BODY_PARTIAL
                    } else {
                        BODY_FULL
                    },
                )
                .await
            {
                Ok(msgs) => msgs,
                Err(err) => {
                    // TODO: maybe differentiate between IO and input/parsing problems
                    // so we don't reconnect if we have a (rare) input/output parsing problem?
                    self.should_reconnect = true;
                    warn!(
                        context,
                        "Error on fetching messages #{} from folder \"{}\"; error={}.",
                        &set,
                        folder,
                        err
                    );
                    return (None, Vec::new());
                }
            };

            let folder = folder.to_string();

            while let Some(Ok(msg)) = msgs.next().await {
                let server_uid = msg.uid.unwrap_or_default();

                if !server_uids.contains(&server_uid) {
                    warn!(
                        context,
                        "Got unwanted uid {} not in {:?}, requested {:?}",
                        &server_uid,
                        server_uids,
                        &sets
                    );
                    continue;
                }
                count += 1;

                let is_deleted = msg.flags().any(|flag| flag == Flag::Deleted);
                let (body, partial) = if fetch_partially {
                    (msg.header(), msg.size) // `BODY.PEEK[HEADER]` goes to header() ...
                } else {
                    (msg.body(), None) // ... while `BODY.PEEK[]` goes to body() - and includes header()
                };

                if is_deleted || body.is_none() {
                    info!(
                        context,
                        "Not processing deleted or empty msg {}", server_uid
                    );
                    last_uid = Some(server_uid);
                    continue;
                }

                // XXX put flags into a set and pass them to dc_receive_imf
                let context = context.clone();
                let folder = folder.clone();

                // safe, as we checked above that there is a body.
                let body = body.unwrap();
                let is_seen = msg.flags().any(|flag| flag == Flag::Seen);

                match dc_receive_imf_inner(
                    &context,
                    body,
                    &folder,
                    is_seen,
                    partial,
                    fetching_existing_messages,
                )
                .await
                {
                    Ok(received_msg) => {
                        if let Some(m) = received_msg {
                            received_msgs.push(m);
                        }
                        last_uid = Some(server_uid)
                    }
                    Err(err) => {
                        warn!(context, "dc_receive_imf error: {}", err);
                    }
                };
            }
        }

        if count != server_uids.len() {
            warn!(
                context,
                "failed to fetch all uids: got {}, requested {}, we requested the UIDs {:?} using {:?}",
                count,
                server_uids.len(),
                server_uids,
                sets
            );
        }

        (last_uid, received_msgs)
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
        if self.session.is_none() {
            // currently jobs are only performed on the INBOX thread
            // TODO: make INBOX/SENT/MVBOX perform the jobs on their
            // respective folders to avoid select_folder network traffic
            // and the involved error states
            if let Err(err) = self.prepare(context).await {
                warn!(context, "prepare_imap_op failed: {}", err);
                return Some(ImapActionResult::RetryLater);
            }
        }
        match self.select_folder(context, Some(folder)).await {
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

        let display_imap_id = format!("{}/{}", folder, uid);

        // mark the message for deletion
        if !self.add_flag_finalized(context, uid, "\\Deleted").await {
            warn!(
                context,
                "Cannot mark message {} as \"Deleted\".", display_imap_id
            );
            ImapActionResult::RetryLater
        } else {
            context.emit_event(EventType::ImapMessageDeleted(format!(
                "IMAP Message {} marked as deleted",
                display_imap_id
            )));
            self.config.selected_folder_needs_expunge = true;
            ImapActionResult::Success
        }
    }

    pub async fn ensure_configured_folders(
        &mut self,
        context: &Context,
        create_mvbox: bool,
    ) -> Result<()> {
        let folders_configured = context.sql.get_raw_config_int("folders_configured").await?;
        if folders_configured.unwrap_or_default() >= DC_FOLDERS_CONFIGURED_VERSION {
            return Ok(());
        }

        self.configure_folders(context, create_mvbox).await
    }

    pub async fn configure_folders(&mut self, context: &Context, create_mvbox: bool) -> Result<()> {
        let session = match self.session {
            Some(ref mut session) => session,
            None => bail!("no IMAP connection established"),
        };

        let mut folders = match session.list(Some(""), Some("*")).await {
            Ok(f) => f,
            Err(err) => {
                bail!("list_folders failed: {}", err);
            }
        };

        let mut delimiter = ".".to_string();
        let mut delimiter_is_default = true;
        let mut mvbox_folder = None;
        let mut folder_configs = BTreeMap::new();
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

            let folder_meaning = get_folder_meaning(&folder);
            let folder_name_meaning = get_folder_meaning_by_name(folder.name());
            if folder.name() == "DeltaChat" {
                // Always takes precedence
                mvbox_folder = Some(folder.name().to_string());
            } else if folder.name() == fallback_folder {
                // only set if none has been already set
                if mvbox_folder.is_none() {
                    mvbox_folder = Some(folder.name().to_string());
                }
            } else if let Some(config) = folder_meaning.to_config() {
                // Always takes precedence
                folder_configs.insert(config, folder.name().to_string());
            } else if let Some(config) = folder_name_meaning.to_config() {
                // only set if none has been already set
                folder_configs
                    .entry(config)
                    .or_insert_with(|| folder.name().to_string());
            }
        }
        drop(folders);

        info!(context, "Using \"{}\" as folder-delimiter.", delimiter);

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
                        "Cannot create MVBOX-folder, trying to create INBOX subfolder. ({})", err
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
        for (config, name) in folder_configs {
            context.set_config(config, Some(&name)).await?;
        }
        context
            .sql
            .set_raw_config_int("folders_configured", DC_FOLDERS_CONFIGURED_VERSION)
            .await?;

        info!(context, "FINISHED configuring IMAP-folders.");
        Ok(())
    }

    /// Return whether the server sent an unsolicited EXISTS response.
    /// Drains all responses from `session.unsolicited_responses` in the process.
    /// If this returns `true`, this means that new emails arrived and you should
    /// fetch again, even if you just fetched.
    fn server_sent_unsolicited_exists(&self, context: &Context) -> bool {
        let session = match &self.session {
            Some(s) => s,
            None => return false,
        };
        let mut unsolicited_exists = false;
        while let Ok(response) = session.unsolicited_responses.try_recv() {
            match response {
                UnsolicitedResponse::Exists(_) => {
                    info!(
                        context,
                        "Need to fetch again, got unsolicited EXISTS {:?}", response
                    );
                    unsolicited_exists = true;
                }
                _ => info!(context, "ignoring unsolicited response {:?}", response),
            }
        }
        unsolicited_exists
    }

    pub fn can_check_quota(&self) -> bool {
        self.config.can_check_quota
    }

    pub async fn get_quota_roots(
        &mut self,
        mailbox_name: &str,
    ) -> Result<(Vec<QuotaRoot>, Vec<Quota>)> {
        if let Some(session) = self.session.as_mut() {
            let quota_roots = session.get_quota_root(mailbox_name).await?;
            Ok(quota_roots)
        } else {
            Err(anyhow!("Not connected to IMAP, no session"))
        }
    }
}

/// Returns target folder for a message found in the Spam folder.
async fn spam_target_folder(
    context: &Context,
    folder: &str,
    headers: &[mailparse::MailHeader<'_>],
) -> Result<Option<Config>> {
    if let Some(chat) = prefetch_get_chat(context, headers).await? {
        if chat.blocked != Blocked::Not {
            // Blocked or contact request message in the spam folder, leave it there.
            return Ok(None);
        }
    } else {
        // No chat found.
        let (from_id, blocked_contact, _origin) =
            from_field_to_contact_id(context, &mimeparser::get_from(headers), true).await?;
        if blocked_contact {
            // Contact is blocked, leave the message in spam.
            return Ok(None);
        }

        if let Some(chat_id_blocked) = ChatIdBlocked::lookup_by_contact(context, from_id).await? {
            if chat_id_blocked.blocked != Blocked::Not {
                return Ok(None);
            }
        } else if from_id != DC_CONTACT_ID_SELF {
            // No chat with this contact found.
            return Ok(None);
        }
    }

    if needs_move_to_mvbox(context, headers).await? {
        Ok(Some(Config::ConfiguredMvboxFolder))
    } else if needs_move_to_sentbox(context, folder, headers).await? {
        Ok(Some(Config::ConfiguredSentboxFolder))
    } else {
        Ok(Some(Config::ConfiguredInboxFolder))
    }
}

/// Returns `ConfiguredInboxFolder`, `ConfiguredMvboxFolder` or `ConfiguredSentboxFolder` if
/// the message needs to be moved from `folder`. Otherwise returns `None`.
pub async fn target_folder(
    context: &Context,
    folder: &str,
    headers: &[mailparse::MailHeader<'_>],
) -> Result<Option<Config>> {
    if context.is_mvbox(folder).await? {
        return Ok(None);
    }

    if context.is_spam_folder(folder).await? {
        spam_target_folder(context, folder, headers).await
    } else if needs_move_to_mvbox(context, headers).await? {
        Ok(Some(Config::ConfiguredMvboxFolder))
    } else if needs_move_to_sentbox(context, folder, headers).await? {
        Ok(Some(Config::ConfiguredSentboxFolder))
    } else {
        Ok(None)
    }
}

async fn needs_move_to_mvbox(
    context: &Context,
    headers: &[mailparse::MailHeader<'_>],
) -> Result<bool> {
    if !context.get_config_bool(Config::MvboxMove).await? {
        return Ok(false);
    }

    if headers
        .get_header_value(HeaderDef::AutocryptSetupMessage)
        .is_some()
    {
        // do not move setup messages;
        // there may be a non-delta device that wants to handle it
        return Ok(false);
    }

    if headers.get_header_value(HeaderDef::ChatVersion).is_some() {
        Ok(true)
    } else if let Some(parent) = get_prefetch_parent_message(context, headers).await? {
        match parent.is_dc_message {
            MessengerMessage::No => Ok(false),
            MessengerMessage::Yes | MessengerMessage::Reply => Ok(true),
        }
    } else {
        Ok(false)
    }
}

async fn prefetch_is_outgoing(
    context: &Context,
    headers: &[mailparse::MailHeader<'_>],
) -> Result<bool> {
    let from_address_list = &mimeparser::get_from(headers);

    // Only looking at the first address in the `From:` field.
    if let Some(info) = from_address_list.first() {
        if context.is_self_addr(&info.addr).await? {
            Ok(true)
        } else {
            Ok(false)
        }
    } else {
        Ok(false)
    }
}

async fn needs_move_to_sentbox(
    context: &Context,
    folder: &str,
    headers: &[mailparse::MailHeader<'_>],
) -> Result<bool> {
    let needs_move = context.get_config_bool(Config::SentboxMove).await?
        && context
            .get_config(Config::ConfiguredSentboxFolder)
            .await?
            .is_some()
        && context.is_inbox(folder).await?
        && headers.get_header_value(HeaderDef::ChatVersion).is_some()
        && headers
            .get_header_value(HeaderDef::AutocryptSetupMessage)
            .is_none()
        && prefetch_is_outgoing(context, headers).await?;

    Ok(needs_move)
}

/// Try to get the folder meaning by the name of the folder only used if the server does not support XLIST.
// TODO: lots languages missing - maybe there is a list somewhere on other MUAs?
// however, if we fail to find out the sent-folder,
// only watching this folder is not working. at least, this is no show stopper.
// CAVE: if possible, take care not to add a name here that is "sent" in one language
// but sth. different in others - a hard job.
fn get_folder_meaning_by_name(folder_name: &str) -> FolderMeaning {
    // source: <https://stackoverflow.com/questions/2185391/localized-gmail-imap-folders>
    const SENT_NAMES: &[&str] = &[
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
    const SPAM_NAMES: &[&str] = &[
        "spam",
        "junk",
        "Correio electrónico não solicitado",
        "Correo basura",
        "Lixo",
        "Nettsøppel",
        "Nevyžádaná pošta",
        "No solicitado",
        "Ongewenst",
        "Posta indesiderata",
        "Skräp",
        "Wiadomości-śmieci",
        "Önemsiz",
        "Ανεπιθύμητα",
        "Спам",
        "垃圾邮件",
        "垃圾郵件",
        "迷惑メール",
        "스팸",
    ];
    const DRAFT_NAMES: &[&str] = &[
        "Drafts",
        "Kladder",
        "Entw?rfe",
        "Borradores",
        "Brouillons",
        "Bozze",
        "Concepten",
        "Wersje robocze",
        "Rascunhos",
        "Entwürfe",
        "Koncepty",
        "Kopie robocze",
        "Taslaklar",
        "Utkast",
        "Πρόχειρα",
        "Черновики",
        "下書き",
        "草稿",
        "임시보관함",
    ];
    let lower = folder_name.to_lowercase();

    if SENT_NAMES.iter().any(|s| s.to_lowercase() == lower) {
        FolderMeaning::Sent
    } else if SPAM_NAMES.iter().any(|s| s.to_lowercase() == lower) {
        FolderMeaning::Spam
    } else if DRAFT_NAMES.iter().any(|s| s.to_lowercase() == lower) {
        FolderMeaning::Drafts
    } else {
        FolderMeaning::Unknown
    }
}

fn get_folder_meaning(folder_name: &Name) -> FolderMeaning {
    for attr in folder_name.attributes() {
        if let NameAttribute::Custom(ref label) = attr {
            match label.as_ref() {
                "\\Trash" => return FolderMeaning::Other,
                "\\Sent" => return FolderMeaning::Sent,
                "\\Spam" | "\\Junk" => return FolderMeaning::Spam,
                "\\Drafts" => return FolderMeaning::Drafts,
                _ => {}
            };
        }
    }
    FolderMeaning::Unknown
}

/// Parses the headers from the FETCH result.
fn get_fetch_headers(prefetch_msg: &Fetch) -> Result<Vec<mailparse::MailHeader>> {
    match prefetch_msg.header() {
        Some(header_bytes) => {
            let (headers, _) = mailparse::parse_headers(header_bytes)?;
            Ok(headers)
        }
        None => Ok(Vec::new()),
    }
}

fn prefetch_get_message_id(headers: &[mailparse::MailHeader]) -> Result<String> {
    if let Some(message_id) = headers.get_header_value(HeaderDef::XMicrosoftOriginalMessageId) {
        Ok(crate::mimeparser::parse_message_id(&message_id)?)
    } else if let Some(message_id) = headers.get_header_value(HeaderDef::MessageId) {
        Ok(crate::mimeparser::parse_message_id(&message_id)?)
    } else {
        bail!("prefetch: No message ID found");
    }
}

/// Returns chat by prefetched headers.
async fn prefetch_get_chat(
    context: &Context,
    headers: &[mailparse::MailHeader<'_>],
) -> Result<Option<chat::Chat>> {
    let parent = get_prefetch_parent_message(context, headers).await?;
    if let Some(parent) = &parent {
        return Ok(Some(
            chat::Chat::load_from_db(context, parent.get_chat_id()).await?,
        ));
    }

    Ok(None)
}

/// Determines whether the message should be downloaded based on prefetched headers.
pub(crate) async fn prefetch_should_download(
    context: &Context,
    headers: &[mailparse::MailHeader<'_>],
    message_id: &str,
    mut flags: impl Iterator<Item = Flag<'_>>,
    show_emails: ShowEmails,
) -> Result<bool> {
    if let Some(msg_id) = message::rfc724_mid_exists(context, message_id).await? {
        // We know the Message-ID already, it must be a Bcc: to self.
        job::add(
            context,
            job::Job::new(Action::MarkseenMsgOnImap, msg_id.to_u32(), Params::new(), 0),
        )
        .await?;
        return Ok(false);
    }

    // We do not know the Message-ID or the Message-ID is missing (in this case, we create one in
    // the further process).

    if let Some(chat) = prefetch_get_chat(context, headers).await? {
        if chat.typ == Chattype::Group && !chat.id.is_special() {
            // This might be a group command, like removing a group member.
            // We really need to fetch this to avoid inconsistent group state.
            return Ok(true);
        }
    }

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

    let (_from_id, blocked_contact, origin) =
        from_field_to_contact_id(context, &mimeparser::get_from(headers), true).await?;
    // prevent_rename=true as this might be a mailing list message and in this case it would be bad if we rename the contact.
    // (prevent_rename is the last argument of from_field_to_contact_id())

    if flags.any(|f| f == Flag::Draft) {
        info!(context, "Ignoring draft message");
        return Ok(false);
    }

    let is_chat_message = headers.get_header_value(HeaderDef::ChatVersion).is_some();
    let accepted_contact = origin.is_known();
    let is_reply_to_chat_message = get_prefetch_parent_message(context, headers)
        .await?
        .map(|parent| match parent.is_dc_message {
            MessengerMessage::No => false,
            MessengerMessage::Yes | MessengerMessage::Reply => true,
        })
        .unwrap_or_default();

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

fn get_fallback_folder(delimiter: &str) -> String {
    format!("INBOX{}DeltaChat", delimiter)
}

/// uid_next is the next unique identifier value from the last time we fetched a folder
/// See <https://tools.ietf.org/html/rfc3501#section-2.3.1.1>
/// This function is used to update our uid_next after fetching messages.
pub(crate) async fn set_uid_next(context: &Context, folder: &str, uid_next: u32) -> Result<()> {
    context
        .sql
        .execute(
            "INSERT INTO imap_sync (folder, uidvalidity, uid_next) VALUES (?,?,?)
                ON CONFLICT(folder) DO UPDATE SET uid_next=? WHERE folder=?;",
            paramsv![folder, 0u32, uid_next, uid_next, folder],
        )
        .await?;
    Ok(())
}

/// uid_next is the next unique identifier value from the last time we fetched a folder
/// See <https://tools.ietf.org/html/rfc3501#section-2.3.1.1>
/// This method returns the uid_next from the last time we fetched messages.
/// We can compare this to the current uid_next to find out whether there are new messages
/// and fetch from this value on to get all new messages.
async fn get_uid_next(context: &Context, folder: &str) -> Result<u32> {
    Ok(context
        .sql
        .query_get_value(
            "SELECT uid_next FROM imap_sync WHERE folder=?;",
            paramsv![folder],
        )
        .await?
        .unwrap_or(0))
}

pub(crate) async fn set_uidvalidity(
    context: &Context,
    folder: &str,
    uidvalidity: u32,
) -> Result<()> {
    context
        .sql
        .execute(
            "INSERT INTO imap_sync (folder, uidvalidity, uid_next) VALUES (?,?,?)
                ON CONFLICT(folder) DO UPDATE SET uidvalidity=? WHERE folder=?;",
            paramsv![folder, uidvalidity, 0u32, uidvalidity, folder],
        )
        .await?;
    Ok(())
}

async fn get_uidvalidity(context: &Context, folder: &str) -> Result<u32> {
    Ok(context
        .sql
        .query_get_value(
            "SELECT uidvalidity FROM imap_sync WHERE folder=?;",
            paramsv![folder],
        )
        .await?
        .unwrap_or(0))
}

/// Deprecated, use get_uid_next() and get_uidvalidity()
pub async fn get_config_last_seen_uid<S: AsRef<str>>(
    context: &Context,
    folder: S,
) -> Result<(u32, u32)> {
    let key = format!("imap.mailbox.{}", folder.as_ref());
    if let Some(entry) = context.sql.get_raw_config(&key).await? {
        // the entry has the format `imap.mailbox.<folder>=<uidvalidity>:<lastseenuid>`
        let mut parts = entry.split(':');
        Ok((
            parts.next().unwrap_or_default().parse().unwrap_or(0),
            parts.next().unwrap_or_default().parse().unwrap_or(0),
        ))
    } else {
        Ok((0, 0))
    }
}

/// Builds a list of sequence/uid sets. The returned sets have each no more than around 1000
/// characters because according to <https://tools.ietf.org/html/rfc2683#section-3.2.1.5>
/// command lines should not be much more than 1000 chars (servers should allow at least 8000 chars)
fn build_sequence_sets(mut uids: Vec<u32>) -> Vec<String> {
    uids.sort_unstable();

    // first, try to find consecutive ranges:
    let mut ranges: Vec<UidRange> = vec![];

    for current in uids {
        if let Some(last) = ranges.last_mut() {
            if last.end + 1 == current {
                last.end = current;
                continue;
            }
        }

        ranges.push(UidRange {
            start: current,
            end: current,
        });
    }

    // Second, sort the uids into uid sets that are each below ~1000 characters
    let mut result = vec![String::new()];
    for range in ranges {
        if let Some(last) = result.last_mut() {
            if !last.is_empty() {
                last.push(',');
            }
            last.push_str(&range.to_string());

            if last.len() > 990 {
                result.push(String::new()); // Start a new uid set
            }
        }
    }

    result.retain(|s| !s.is_empty());
    result
}

struct UidRange {
    start: u32,
    end: u32,
    // If start == end, then this range represents a single number
}

impl std::fmt::Display for UidRange {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.start == self.end {
            write!(f, "{}", self.start)
        } else {
            write!(f, "{}:{}", self.start, self.end)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chat::ChatId;
    use crate::config::Config;
    use crate::contact::Contact;
    use crate::test_utils::TestContext;

    #[test]
    fn test_get_folder_meaning_by_name() {
        assert_eq!(get_folder_meaning_by_name("Gesendet"), FolderMeaning::Sent);
        assert_eq!(get_folder_meaning_by_name("GESENDET"), FolderMeaning::Sent);
        assert_eq!(get_folder_meaning_by_name("gesendet"), FolderMeaning::Sent);
        assert_eq!(
            get_folder_meaning_by_name("Messages envoyés"),
            FolderMeaning::Sent
        );
        assert_eq!(
            get_folder_meaning_by_name("mEsSaGes envoyÉs"),
            FolderMeaning::Sent
        );
        assert_eq!(get_folder_meaning_by_name("xxx"), FolderMeaning::Unknown);
        assert_eq!(get_folder_meaning_by_name("SPAM"), FolderMeaning::Spam);
    }

    #[async_std::test]
    async fn test_set_uid_next_validity() {
        let t = TestContext::new_alice().await;
        assert_eq!(get_uid_next(&t.ctx, "Inbox").await.unwrap(), 0);
        assert_eq!(get_uidvalidity(&t.ctx, "Inbox").await.unwrap(), 0);

        set_uidvalidity(&t.ctx, "Inbox", 7).await.unwrap();
        assert_eq!(get_uidvalidity(&t.ctx, "Inbox").await.unwrap(), 7);
        assert_eq!(get_uid_next(&t.ctx, "Inbox").await.unwrap(), 0);

        set_uid_next(&t.ctx, "Inbox", 5).await.unwrap();
        set_uidvalidity(&t.ctx, "Inbox", 6).await.unwrap();
        assert_eq!(get_uid_next(&t.ctx, "Inbox").await.unwrap(), 5);
        assert_eq!(get_uidvalidity(&t.ctx, "Inbox").await.unwrap(), 6);
    }

    #[test]
    fn test_build_sequence_sets() {
        let cases = vec![
            (vec![], vec![]),
            (vec![1], vec!["1"]),
            (vec![3291], vec!["3291"]),
            (vec![1, 3, 5, 7, 9, 11], vec!["1,3,5,7,9,11"]),
            (vec![1, 2, 3], vec!["1:3"]),
            (vec![1, 4, 5, 6], vec!["1,4:6"]),
            ((1..=500).collect(), vec!["1:500"]),
            (vec![3, 4, 8, 9, 10, 11, 39, 50, 2], vec!["2:4,8:11,39,50"]),
        ];
        for (input, output) in cases {
            assert_eq!(build_sequence_sets(input), output);
        }

        let numbers: Vec<_> = (2..=500).step_by(2).collect();
        let result = build_sequence_sets(numbers.clone());
        for set in &result {
            assert!(set.len() < 1010);
            assert!(!set.ends_with(','));
            assert!(!set.starts_with(','));
        }
        assert!(result.len() == 1); // these UIDs fit in one set
        for number in &numbers {
            assert!(result
                .iter()
                .any(|set| set.split(',').any(|n| n.parse::<u32>().unwrap() == *number)));
        }

        let numbers: Vec<_> = (1..=1000).step_by(3).collect();
        let result = build_sequence_sets(numbers.clone());
        for set in &result {
            assert!(set.len() < 1010);
            assert!(!set.ends_with(','));
            assert!(!set.starts_with(','));
        }
        assert!(result.last().unwrap().ends_with("997,1000"));
        assert!(result.len() == 2); // This time we need 2 sets
        for number in &numbers {
            assert!(result
                .iter()
                .any(|set| set.split(',').any(|n| n.parse::<u32>().unwrap() == *number)));
        }

        let numbers: Vec<_> = (30000000..=30002500).step_by(4).collect();
        let result = build_sequence_sets(numbers.clone());
        for set in &result {
            assert!(set.len() < 1010);
            assert!(!set.ends_with(','));
            assert!(!set.starts_with(','));
        }
        assert_eq!(result.len(), 6);
        for number in &numbers {
            assert!(result
                .iter()
                .any(|set| set.split(',').any(|n| n.parse::<u32>().unwrap() == *number)));
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn check_target_folder_combination(
        folder: &str,
        mvbox_move: bool,
        chat_msg: bool,
        expected_destination: &str,
        accepted_chat: bool,
        outgoing: bool,
        setupmessage: bool,
        sentbox_move: bool,
    ) -> Result<()> {
        println!("Testing: For folder {}, mvbox_move {}, chat_msg {}, accepted {}, outgoing {}, setupmessage {}",
                               folder, mvbox_move, chat_msg, accepted_chat, outgoing, setupmessage);

        let t = TestContext::new_alice().await;
        t.ctx
            .set_config(Config::ConfiguredSpamFolder, Some("Spam"))
            .await?;
        t.ctx
            .set_config(Config::ConfiguredMvboxFolder, Some("DeltaChat"))
            .await?;
        t.ctx
            .set_config(Config::ConfiguredSentboxFolder, Some("Sent"))
            .await?;
        t.ctx
            .set_config(Config::MvboxMove, Some(if mvbox_move { "1" } else { "0" }))
            .await?;
        t.ctx.set_config(Config::ShowEmails, Some("2")).await?;
        t.ctx
            .set_config_bool(Config::SentboxMove, sentbox_move)
            .await?;

        if accepted_chat {
            let contact_id = Contact::create(&t.ctx, "", "bob@example.net").await?;
            ChatId::create_for_contact(&t.ctx, contact_id).await?;
        }
        let temp;

        let bytes = if setupmessage {
            include_bytes!("../test-data/message/AutocryptSetupMessage.eml")
        } else {
            temp = format!(
                "Received: (Postfix, from userid 1000); Mon, 4 Dec 2006 14:51:39 +0100 (CET)\n\
                    {}\
                    Subject: foo\n\
                    Message-ID: <abc@example.com>\n\
                    {}\
                    Date: Sun, 22 Mar 2020 22:37:57 +0000\n\
                    \n\
                    hello\n",
                if outgoing {
                    "From: alice@example.org\nTo: bob@example.net\n"
                } else {
                    "From: bob@example.net\nTo: alice@example.org\n"
                },
                if chat_msg { "Chat-Version: 1.0\n" } else { "" },
            );
            temp.as_bytes()
        };

        let (headers, _) = mailparse::parse_headers(bytes)?;

        let actual = if let Some(config) = target_folder(&t, folder, &headers).await? {
            t.get_config(config).await?
        } else {
            None
        };

        let expected = if expected_destination == folder {
            None
        } else {
            Some(expected_destination)
        };
        assert_eq!(expected, actual.as_deref(), "For folder {}, mvbox_move {}, chat_msg {}, accepted {}, outgoing {}, setupmessage {}: expected {:?}, got {:?}",
                                                     folder, mvbox_move, chat_msg, accepted_chat, outgoing, setupmessage, expected, actual);
        Ok(())
    }

    // chat_msg means that the message was sent by Delta Chat
    // The tuples are (folder, mvbox_move, chat_msg, expected_destination)
    const COMBINATIONS_ACCEPTED_CHAT: &[(&str, bool, bool, &str)] = &[
        ("INBOX", false, false, "INBOX"),
        ("INBOX", false, true, "INBOX"),
        ("INBOX", true, false, "INBOX"),
        ("INBOX", true, true, "DeltaChat"),
        ("Sent", false, false, "Sent"),
        ("Sent", false, true, "Sent"),
        ("Sent", true, false, "Sent"),
        ("Sent", true, true, "DeltaChat"),
        ("Spam", false, false, "INBOX"), // Move classical emails in accepted chats from Spam to Inbox, not 100% sure on this, we could also just never move non-chat-msgs
        ("Spam", false, true, "INBOX"),
        ("Spam", true, false, "INBOX"), // Move classical emails in accepted chats from Spam to Inbox, not 100% sure on this, we could also just never move non-chat-msgs
        ("Spam", true, true, "DeltaChat"),
    ];

    // These are the same as above, but all messages in Spam stay in Spam
    const COMBINATIONS_REQUEST: &[(&str, bool, bool, &str)] = &[
        ("INBOX", false, false, "INBOX"),
        ("INBOX", false, true, "INBOX"),
        ("INBOX", true, false, "INBOX"),
        ("INBOX", true, true, "DeltaChat"),
        ("Sent", false, false, "Sent"),
        ("Sent", false, true, "Sent"),
        ("Sent", true, false, "Sent"),
        ("Sent", true, true, "DeltaChat"),
        ("Spam", false, false, "Spam"),
        ("Spam", false, true, "Spam"),
        ("Spam", true, false, "Spam"),
        ("Spam", true, true, "Spam"),
    ];

    #[async_std::test]
    async fn test_target_folder_incoming_accepted() -> Result<()> {
        for (folder, mvbox_move, chat_msg, expected_destination) in COMBINATIONS_ACCEPTED_CHAT {
            check_target_folder_combination(
                folder,
                *mvbox_move,
                *chat_msg,
                expected_destination,
                true,
                false,
                false,
                false,
            )
            .await?;
        }
        Ok(())
    }

    #[async_std::test]
    async fn test_target_folder_incoming_request() -> Result<()> {
        for (folder, mvbox_move, chat_msg, expected_destination) in COMBINATIONS_REQUEST {
            check_target_folder_combination(
                folder,
                *mvbox_move,
                *chat_msg,
                expected_destination,
                false,
                false,
                false,
                false,
            )
            .await?;
        }
        Ok(())
    }

    #[async_std::test]
    async fn test_target_folder_outgoing() -> Result<()> {
        for sentbox_move in &[true, false] {
            // Test outgoing emails
            for (folder, mvbox_move, chat_msg, mut expected_destination) in
                COMBINATIONS_ACCEPTED_CHAT
            {
                if *folder == "INBOX" && !mvbox_move && *chat_msg && *sentbox_move {
                    expected_destination = "Sent"
                }
                check_target_folder_combination(
                    folder,
                    *mvbox_move,
                    *chat_msg,
                    expected_destination,
                    true,
                    true,
                    false,
                    *sentbox_move,
                )
                .await?;
            }
        }
        Ok(())
    }

    #[async_std::test]
    async fn test_target_folder_setupmsg() -> Result<()> {
        // Test setupmessages
        for (folder, mvbox_move, chat_msg, _expected_destination) in COMBINATIONS_ACCEPTED_CHAT {
            check_target_folder_combination(
                folder,
                *mvbox_move,
                *chat_msg,
                if folder == &"Spam" { "INBOX" } else { folder }, // Never move setup messages, except if they are in "Spam"
                false,
                true,
                true,
                false,
            )
            .await?;
        }
        Ok(())
    }
}
