//! # IMAP handling module.
//!
//! uses [async-email/async-imap](https://github.com/async-email/async-imap)
//! to implement connect, fetch, delete functionality with standard IMAP servers.

use std::{
    cmp,
    cmp::max,
    collections::{BTreeMap, BTreeSet, HashMap},
    iter::Peekable,
    mem::take,
};

use anyhow::{bail, format_err, Context as _, Result};
use async_channel::Receiver;
use async_imap::types::{Fetch, Flag, Name, NameAttribute, UnsolicitedResponse};
use futures::{StreamExt, TryStreamExt};
use num_traits::FromPrimitive;

use crate::chat::{self, ChatId, ChatIdBlocked};
use crate::config::Config;
use crate::constants::{
    Blocked, Chattype, ShowEmails, DC_FETCH_EXISTING_MSGS_COUNT, DC_FOLDERS_CONFIGURED_VERSION,
};
use crate::contact::{normalize_name, Contact, ContactAddress, ContactId, Modifier, Origin};
use crate::context::Context;
use crate::events::EventType;
use crate::headerdef::{HeaderDef, HeaderDefMap};
use crate::login_param::{CertificateChecks, LoginParam, ServerLoginParam};
use crate::message::{self, Message, MessageState, MessengerMessage, MsgId, Viewtype};
use crate::mimeparser;
use crate::oauth2::get_oauth2_access_token;
use crate::provider::Socket;
use crate::receive_imf::{
    from_field_to_contact_id, get_prefetch_parent_message, receive_imf_inner, ReceivedMsg,
};
use crate::scheduler::connectivity::ConnectivityStore;
use crate::socks::Socks5Config;
use crate::sql;
use crate::stock_str;
use crate::tools::create_id;

pub(crate) mod capabilities;
mod client;
mod idle;
pub mod scan_folders;
pub mod select_folder;
pub(crate) mod session;

use client::Client;
use mailparse::SingleInfo;
use session::Session;

use self::select_folder::NewlySelected;

pub(crate) const GENERATED_PREFIX: &str = "GEN_";

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
const PREFETCH_FLAGS: &str = "(UID INTERNALDATE RFC822.SIZE BODY.PEEK[HEADER.FIELDS (\
                              MESSAGE-ID \
                              X-MICROSOFT-ORIGINAL-MESSAGE-ID \
                              FROM \
                              IN-REPLY-TO REFERENCES \
                              CHAT-VERSION \
                              AUTOCRYPT-SETUP-MESSAGE\
                              )])";
const RFC724MID_UID: &str = "(UID BODY.PEEK[HEADER.FIELDS (\
                             MESSAGE-ID \
                             X-MICROSOFT-ORIGINAL-MESSAGE-ID\
                             )])";
const BODY_FULL: &str = "(FLAGS BODY.PEEK[])";
const BODY_PARTIAL: &str = "(FLAGS RFC822.SIZE BODY.PEEK[HEADER])";

#[derive(Debug)]
pub struct Imap {
    pub(crate) idle_interrupt_receiver: Receiver<()>,
    config: ImapConfig,
    pub(crate) session: Option<Session>,
    login_failed_once: bool,

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

#[derive(Debug, Display, PartialEq, Eq, Clone, Copy)]
pub enum FolderMeaning {
    Unknown,

    /// Spam folder.
    Spam,
    Inbox,
    Mvbox,
    Sent,
    Trash,
    Drafts,

    /// Virtual folders.
    ///
    /// On Gmail there are virtual folders marked as \\All, \\Important and \\Flagged.
    /// Delta Chat ignores these folders because the same messages can be fetched
    /// from the real folder and the result of moving and deleting messages via
    /// virtual folder is unclear.
    Virtual,
}

impl FolderMeaning {
    pub fn to_config(self) -> Option<Config> {
        match self {
            FolderMeaning::Unknown => None,
            FolderMeaning::Spam => None,
            FolderMeaning::Inbox => Some(Config::ConfiguredInboxFolder),
            FolderMeaning::Mvbox => Some(Config::ConfiguredMvboxFolder),
            FolderMeaning::Sent => Some(Config::ConfiguredSentboxFolder),
            FolderMeaning::Trash => Some(Config::ConfiguredTrashFolder),
            FolderMeaning::Drafts => None,
            FolderMeaning::Virtual => None,
        }
    }
}

#[derive(Debug)]
struct ImapConfig {
    /// Email address.
    pub addr: String,
    pub lp: ServerLoginParam,

    /// SOCKS 5 configuration.
    pub socks5_config: Option<Socks5Config>,
    pub strict_tls: bool,
}

struct UidGrouper<T: Iterator<Item = (i64, u32, String)>> {
    inner: Peekable<T>,
}

impl<T, I> From<I> for UidGrouper<T>
where
    T: Iterator<Item = (i64, u32, String)>,
    I: IntoIterator<IntoIter = T>,
{
    fn from(inner: I) -> Self {
        Self {
            inner: inner.into_iter().peekable(),
        }
    }
}

impl<T: Iterator<Item = (i64, u32, String)>> Iterator for UidGrouper<T> {
    // Tuple of folder, row IDs, and UID range as a string.
    type Item = (String, Vec<i64>, String);

    fn next(&mut self) -> Option<Self::Item> {
        let (_, _, folder) = self.inner.peek().cloned()?;

        let mut uid_set = String::new();
        let mut rowid_set = Vec::new();

        while uid_set.len() < 1000 {
            // Construct a new range.
            if let Some((start_rowid, start_uid, _)) = self
                .inner
                .next_if(|(_, _, start_folder)| start_folder == &folder)
            {
                rowid_set.push(start_rowid);
                let mut end_uid = start_uid;

                while let Some((next_rowid, next_uid, _)) =
                    self.inner.next_if(|(_, next_uid, next_folder)| {
                        next_folder == &folder && (*next_uid == end_uid + 1 || *next_uid == end_uid)
                    })
                {
                    end_uid = next_uid;
                    rowid_set.push(next_rowid);
                }

                let uid_range = UidRange {
                    start: start_uid,
                    end: end_uid,
                };
                if !uid_set.is_empty() {
                    uid_set.push(',');
                }
                uid_set.push_str(&uid_range.to_string());
            } else {
                break;
            }
        }

        Some((folder, rowid_set, uid_set))
    }
}

impl Imap {
    /// Creates new disconnected IMAP client using the specific login parameters.
    ///
    /// `addr` is used to renew token if OAuth2 authentication is used.
    pub fn new(
        lp: &ServerLoginParam,
        socks5_config: Option<Socks5Config>,
        addr: &str,
        provider_strict_tls: bool,
        idle_interrupt_receiver: Receiver<()>,
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
        };

        let imap = Imap {
            idle_interrupt_receiver,
            config,
            session: None,
            login_failed_once: false,
            connectivity: Default::default(),
        };

        Ok(imap)
    }

    /// Creates new disconnected IMAP client using configured parameters.
    pub async fn new_configured(
        context: &Context,
        idle_interrupt_receiver: Receiver<()>,
    ) -> Result<Self> {
        if !context.is_configured().await? {
            bail!("IMAP Connect without configured params");
        }

        let param = LoginParam::load_configured_params(context).await?;
        // the trailing underscore is correct

        let imap = Self::new(
            &param.imap,
            param.socks5_config.clone(),
            &param.addr,
            param
                .provider
                .map_or(param.socks5_config.is_some(), |provider| {
                    provider.opt.strict_tls
                }),
            idle_interrupt_receiver,
        )?;
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

        if self.session.is_some() {
            return Ok(());
        }

        self.connectivity.set_connecting(context).await;

        let oauth2 = self.config.lp.oauth2;

        info!(context, "Connecting to IMAP server");
        let connection_res: Result<Client> = if self.config.lp.security == Socket::Starttls
            || self.config.lp.security == Socket::Plain
        {
            let config = &mut self.config;
            let imap_server: &str = config.lp.server.as_ref();
            let imap_port = config.lp.port;

            if let Some(socks5_config) = &config.socks5_config {
                if config.lp.security == Socket::Starttls {
                    Client::connect_starttls_socks5(
                        context,
                        imap_server,
                        imap_port,
                        socks5_config.clone(),
                        config.strict_tls,
                    )
                    .await
                } else {
                    Client::connect_insecure_socks5(
                        context,
                        imap_server,
                        imap_port,
                        socks5_config.clone(),
                    )
                    .await
                }
            } else if config.lp.security == Socket::Starttls {
                Client::connect_starttls(context, imap_server, imap_port, config.strict_tls).await
            } else {
                Client::connect_insecure(context, imap_server, imap_port).await
            }
        } else {
            let config = &self.config;
            let imap_server: &str = config.lp.server.as_ref();
            let imap_port = config.lp.port;

            if let Some(socks5_config) = &config.socks5_config {
                Client::connect_secure_socks5(
                    context,
                    imap_server,
                    imap_port,
                    config.strict_tls,
                    socks5_config.clone(),
                )
                .await
            } else {
                Client::connect_secure(context, imap_server, imap_port, config.strict_tls).await
            }
        };

        let client = connection_res?;
        let config = &self.config;
        let imap_user: &str = config.lp.user.as_ref();
        let imap_pw: &str = config.lp.password.as_ref();

        let login_res = if oauth2 {
            info!(context, "Logging into IMAP server with OAuth 2");
            let addr: &str = config.addr.as_ref();

            let token = get_oauth2_access_token(context, addr, imap_pw, true)
                .await?
                .context("IMAP could not get OAUTH token")?;
            let auth = OAuth2 {
                user: imap_user.into(),
                access_token: token,
            };
            client.authenticate("XOAUTH2", auth).await
        } else {
            info!(context, "Logging into IMAP server with LOGIN");
            client.login(imap_user, imap_pw).await
        };

        match login_res {
            Ok(session) => {
                // Store server ID in the context to display in account info.
                let mut lock = context.server_id.write().await;
                *lock = session.capabilities.server_id.clone();

                self.session = Some(session);
                self.login_failed_once = false;
                context.emit_event(EventType::ImapConnected(format!(
                    "IMAP-LOGIN as {}",
                    self.config.lp.user
                )));
                self.connectivity.set_connected(context).await;
                info!(context, "Successfully logged into IMAP server");
                Ok(())
            }

            Err(err) => {
                let imap_user = self.config.lp.user.to_owned();
                let message = stock_str::cannot_login(context, &imap_user).await;

                warn!(context, "{} ({:#})", message, err);

                let lock = context.wrong_pw_warning_mutex.lock().await;
                if self.login_failed_once
                    && err.to_string().to_lowercase().contains("authentication")
                    && context.get_config_bool(Config::NotifyAboutWrongPw).await?
                {
                    if let Err(e) = context.set_config(Config::NotifyAboutWrongPw, None).await {
                        warn!(context, "{:#}", e);
                    }
                    drop(lock);

                    let mut msg = Message::new(Viewtype::Text);
                    msg.text = message.clone();
                    if let Err(e) =
                        chat::add_device_msg_with_importance(context, None, Some(&mut msg), true)
                            .await
                    {
                        warn!(context, "{:#}", e);
                    }
                } else {
                    self.login_failed_once = true;
                }

                Err(format_err!("{}\n\n{:#}", message, err))
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
        Ok(())
    }

    /// Drops the session without disconnecting properly.
    /// Useful in case of an IMAP error, when it's unclear if it's in a correct state and it's
    /// easier to setup a new connection.
    pub fn trigger_reconnect(&mut self, context: &Context) {
        info!(context, "Dropping an IMAP connection.");
        self.session = None;
    }

    /// FETCH-MOVE-DELETE iteration.
    ///
    /// Prefetches headers and downloads new message from the folder, moves messages away from the
    /// folder and deletes messages in the folder.
    pub async fn fetch_move_delete(
        &mut self,
        context: &Context,
        watch_folder: &str,
        folder_meaning: FolderMeaning,
    ) -> Result<()> {
        if !context.sql.is_open().await {
            // probably shutdown
            bail!("IMAP operation attempted while it is torn down");
        }
        self.prepare(context).await?;

        let msgs_fetched = self
            .fetch_new_messages(context, watch_folder, folder_meaning, false)
            .await
            .context("fetch_new_messages")?;
        if msgs_fetched && context.get_config_delete_device_after().await?.is_some() {
            // New messages were fetched and shall be deleted later, restart ephemeral loop.
            // Note that the `Config::DeleteDeviceAfter` timer starts as soon as the messages are
            // fetched while the per-chat ephemeral timers start as soon as the messages are marked
            // as noticed.
            context.scheduler.interrupt_ephemeral_task().await;
        }

        let session = self
            .session
            .as_mut()
            .context("no IMAP connection established")?;
        session
            .move_delete_messages(context, watch_folder)
            .await
            .context("move_delete_messages")?;

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
        folder: &str,
        folder_meaning: FolderMeaning,
    ) -> Result<()> {
        // Collect pairs of UID and Message-ID.
        let mut msgs = BTreeMap::new();

        let session = self
            .session
            .as_mut()
            .context("IMAP No connection established")?;

        session.select_folder(context, Some(folder)).await?;

        let mut list = session
            .uid_fetch("1:*", RFC724MID_UID)
            .await
            .with_context(|| format!("can't resync folder {folder}"))?;
        while let Some(fetch) = list.try_next().await? {
            let headers = match get_fetch_headers(&fetch) {
                Ok(headers) => headers,
                Err(err) => {
                    warn!(context, "Failed to parse FETCH headers: {}", err);
                    continue;
                }
            };
            let message_id = prefetch_get_message_id(&headers);

            if let (Some(uid), Some(rfc724_mid)) = (fetch.uid, message_id) {
                msgs.insert(
                    uid,
                    (
                        rfc724_mid,
                        target_folder(context, folder, folder_meaning, &headers).await?,
                    ),
                );
            }
        }

        info!(
            context,
            "Resync: collected {} message IDs in folder {}",
            msgs.len(),
            folder,
        );

        let uid_validity = get_uidvalidity(context, folder).await?;

        // Write collected UIDs to SQLite database.
        context
            .sql
            .transaction(move |transaction| {
                transaction.execute("DELETE FROM imap WHERE folder=?", (folder,))?;
                for (uid, (rfc724_mid, target)) in &msgs {
                    // This may detect previously undetected moved
                    // messages, so we update server_folder too.
                    transaction.execute(
                        "INSERT INTO imap (rfc724_mid, folder, uid, uidvalidity, target)
                         VALUES           (?1,         ?2,     ?3,  ?4,          ?5)
                         ON CONFLICT(folder, uid, uidvalidity)
                         DO UPDATE SET rfc724_mid=excluded.rfc724_mid,
                                       target=excluded.target",
                        (rfc724_mid, folder, uid, uid_validity, target),
                    )?;
                }
                Ok(())
            })
            .await?;
        Ok(())
    }

    /// Selects a folder and takes care of UIDVALIDITY changes.
    ///
    /// When selecting a folder for the first time, sets the uid_next to the current
    /// mailbox.uid_next so that no old emails are fetched.
    ///
    /// Makes sure that UIDNEXT is known for `selected_mailbox`
    /// and errors out if UIDNEXT cannot be determined.
    ///
    /// Returns Result<new_emails> (i.e. whether new emails arrived),
    /// if in doubt, returns new_emails=true so emails are fetched.
    pub(crate) async fn select_with_uidvalidity(
        &mut self,
        context: &Context,
        folder: &str,
    ) -> Result<bool> {
        let session = self.session.as_mut().context("no session")?;
        let newly_selected = session
            .select_or_create_folder(context, folder)
            .await
            .with_context(|| format!("failed to select or create folder {folder}"))?;
        let mailbox = session
            .selected_mailbox
            .as_mut()
            .with_context(|| format!("No mailbox selected, folder: {folder}"))?;

        let new_uid_validity = mailbox
            .uid_validity
            .with_context(|| format!("No UIDVALIDITY for folder {folder}"))?;
        let new_uid_next = if let Some(uid_next) = mailbox.uid_next {
            uid_next
        } else {
            warn!(
                context,
                "SELECT response for IMAP folder {folder:?} has no UIDNEXT, fall back to STATUS command."
            );

            // RFC 3501 says STATUS command SHOULD NOT be used
            // on the currently selected mailbox because the same
            // information can be obtained by other means,
            // such as reading SELECT response.
            //
            // However, it also says that UIDNEXT is REQUIRED
            // in the SELECT response and if we are here,
            // it is actually not returned.
            //
            // In particular, Winmail Pro Mail Server 5.1.0616
            // never returns UIDNEXT in SELECT response,
            // but responds to "STATUS INBOX (UIDNEXT)" command.
            let status = session
                .inner
                .status(folder, "(UIDNEXT)")
                .await
                .with_context(|| format!("STATUS (UIDNEXT) error for {folder:?}"))?;

            status
                .uid_next
                .with_context(|| format!("STATUS {folder} (UIDNEXT) did not return UIDNEXT"))?
        };
        mailbox.uid_next = Some(new_uid_next);

        let old_uid_validity = get_uidvalidity(context, folder)
            .await
            .with_context(|| format!("failed to get old UID validity for folder {folder}"))?;
        let old_uid_next = get_uid_next(context, folder)
            .await
            .with_context(|| format!("failed to get old UID NEXT for folder {folder}"))?;

        if new_uid_validity == old_uid_validity {
            let new_emails = if newly_selected == NewlySelected::No {
                // The folder was not newly selected i.e. no SELECT command was run. This means that mailbox.uid_next
                // was not updated and may contain an incorrect value. So, just return true so that
                // the caller tries to fetch new messages (we could of course run a SELECT command now, but trying to fetch
                // new messages is only one command, just as a SELECT command)
                true
            } else {
                if new_uid_next < old_uid_next {
                    warn!(
                        context,
                        "The server illegally decreased the uid_next of folder {folder:?} from {old_uid_next} to {new_uid_next} without changing validity ({new_uid_validity}), resyncing UIDs...",
                    );
                    set_uid_next(context, folder, new_uid_next).await?;
                    context.schedule_resync().await?;
                }
                new_uid_next != old_uid_next // If UIDNEXT changed, there are new emails
            };
            return Ok(new_emails);
        }

        // UIDVALIDITY is modified, reset highest seen MODSEQ.
        set_modseq(context, folder, 0).await?;

        // ==============  uid_validity has changed or is being set the first time.  ==============

        set_uid_next(context, folder, new_uid_next).await?;
        set_uidvalidity(context, folder, new_uid_validity).await?;

        // Collect garbage entries in `imap` table.
        context
            .sql
            .execute(
                "DELETE FROM imap WHERE folder=? AND uidvalidity!=?",
                (&folder, new_uid_validity),
            )
            .await?;

        if old_uid_validity != 0 || old_uid_next != 0 {
            context.schedule_resync().await?;
        }
        info!(
            context,
            "uid/validity change folder {}: new {}/{} previous {}/{}.",
            folder,
            new_uid_next,
            new_uid_validity,
            old_uid_next,
            old_uid_validity,
        );
        Ok(false)
    }

    /// Fetches new messages.
    ///
    /// Returns true if at least one message was fetched.
    pub(crate) async fn fetch_new_messages(
        &mut self,
        context: &Context,
        folder: &str,
        folder_meaning: FolderMeaning,
        fetch_existing_msgs: bool,
    ) -> Result<bool> {
        if should_ignore_folder(context, folder, folder_meaning).await? {
            info!(context, "Not fetching from {folder:?}.");
            return Ok(false);
        }

        let new_emails = self
            .select_with_uidvalidity(context, folder)
            .await
            .with_context(|| format!("Failed to select folder {folder:?}"))?;

        if !new_emails && !fetch_existing_msgs {
            info!(context, "No new emails in folder {folder:?}.");
            return Ok(false);
        }

        let uid_validity = get_uidvalidity(context, folder).await?;
        let old_uid_next = get_uid_next(context, folder).await?;

        let msgs = if fetch_existing_msgs {
            self.prefetch_existing_msgs()
                .await
                .context("prefetch_existing_msgs")?
        } else {
            self.prefetch(old_uid_next).await.context("prefetch")?
        };
        let read_cnt = msgs.len();

        let download_limit = context.download_limit().await?;
        let mut uids_fetch = Vec::<(_, bool /* partially? */)>::with_capacity(msgs.len() + 1);
        let mut uid_message_ids = BTreeMap::new();
        let mut largest_uid_skipped = None;

        // Store the info about IMAP messages in the database.
        for (uid, ref fetch_response) in msgs {
            let headers = match get_fetch_headers(fetch_response) {
                Ok(headers) => headers,
                Err(err) => {
                    warn!(context, "Failed to parse FETCH headers: {err:#}.");
                    continue;
                }
            };

            let message_id = prefetch_get_message_id(&headers);

            // Determine the target folder where the message should be moved to.
            //
            // If we have seen the message on the IMAP server before, do not move it.
            // This is required to avoid infinite MOVE loop on IMAP servers
            // that alias `DeltaChat` folder to other names.
            // For example, some Dovecot servers alias `DeltaChat` folder to `INBOX.DeltaChat`.
            // In this case Delta Chat configured with `DeltaChat` as the destination folder
            // would detect messages in the `INBOX.DeltaChat` folder
            // and try to move them to the `DeltaChat` folder.
            // Such move to the same folder results in the messages
            // getting a new UID, so the messages will be detected as new
            // in the `INBOX.DeltaChat` folder again.
            let target = if let Some(message_id) = &message_id {
                if context
                    .sql
                    .exists(
                        "SELECT COUNT (*) FROM imap WHERE rfc724_mid=?",
                        (message_id,),
                    )
                    .await?
                {
                    info!(
                        context,
                        "Not moving the message {} that we have seen before.", &message_id
                    );
                    folder.to_string()
                } else {
                    target_folder(context, folder, folder_meaning, &headers).await?
                }
            } else {
                // Do not move the messages without Message-ID.
                // We cannot reliably determine if we have seen them before,
                // so it is safer not to move them.
                warn!(
                    context,
                    "Not moving the message that does not have a Message-ID."
                );
                folder.to_string()
            };

            // Generate a fake Message-ID to identify the message in the database
            // if the message has no real Message-ID.
            let message_id = message_id.unwrap_or_else(create_message_id);

            context
                .sql
                .execute(
                    "INSERT INTO imap (rfc724_mid, folder, uid, uidvalidity, target)
                       VALUES         (?1,         ?2,     ?3,  ?4,          ?5)
                       ON CONFLICT(folder, uid, uidvalidity)
                       DO UPDATE SET rfc724_mid=excluded.rfc724_mid,
                                     target=excluded.target",
                    (&message_id, &folder, uid, uid_validity, &target),
                )
                .await?;

            // Download only the messages which have reached their target folder if there are
            // multiple devices. This prevents race conditions in multidevice case, where one
            // device tries to download the message while another device moves the message at the
            // same time. Even in single device case it is possible to fail downloading the first
            // message, move it to the movebox and then download the second message before
            // downloading the first one, if downloading from inbox before moving is allowed.
            if folder == target
                // Never download messages directly from the spam folder.
                // If the sender is known, the message will be moved to the Inbox or Mvbox
                // and then we download the message from there.
                // Also see `spam_target_folder_cfg()`.
                && folder_meaning != FolderMeaning::Spam
                && prefetch_should_download(
                    context,
                    &headers,
                    &message_id,
                    fetch_response.flags(),
                )
                .await.context("prefetch_should_download")?
            {
                match download_limit {
                    Some(download_limit) => uids_fetch.push((
                        uid,
                        fetch_response.size.unwrap_or_default() > download_limit,
                    )),
                    None => uids_fetch.push((uid, false)),
                }
                uid_message_ids.insert(uid, message_id);
            } else {
                largest_uid_skipped = Some(uid);
            }
        }

        if !uids_fetch.is_empty() {
            self.connectivity.set_working(context).await;
        }

        // Actually download messages.
        let mut largest_uid_fetched: u32 = 0;
        let mut received_msgs = Vec::with_capacity(uids_fetch.len());
        let mut uids_fetch_in_batch = Vec::with_capacity(max(uids_fetch.len(), 1));
        let mut fetch_partially = false;
        uids_fetch.push((0, !uids_fetch.last().unwrap_or(&(0, false)).1));
        for (uid, fp) in uids_fetch {
            if fp != fetch_partially {
                let (largest_uid_fetched_in_batch, received_msgs_in_batch) = self
                    .fetch_many_msgs(
                        context,
                        folder,
                        uids_fetch_in_batch.split_off(0),
                        &uid_message_ids,
                        fetch_partially,
                        fetch_existing_msgs,
                    )
                    .await
                    .context("fetch_many_msgs")?;
                received_msgs.extend(received_msgs_in_batch);
                largest_uid_fetched = max(
                    largest_uid_fetched,
                    largest_uid_fetched_in_batch.unwrap_or(0),
                );
                fetch_partially = fp;
            }
            uids_fetch_in_batch.push(uid);
        }

        // Advance uid_next to the maximum of the largest known UID plus 1
        // and mailbox UIDNEXT.
        // Largest known UID is normally less than UIDNEXT,
        // but a message may have arrived between determining UIDNEXT
        // and executing the FETCH command.
        let mailbox_uid_next = self
            .session
            .as_ref()
            .context("No IMAP session")?
            .selected_mailbox
            .as_ref()
            .with_context(|| format!("Expected {folder:?} to be selected"))?
            .uid_next
            .with_context(|| {
                format!(
                    "Expected UIDNEXT to be determined for {folder:?} by select_with_uidvalidity"
                )
            })?;
        let new_uid_next = max(
            max(largest_uid_fetched, largest_uid_skipped.unwrap_or(0)) + 1,
            mailbox_uid_next,
        );

        if new_uid_next > old_uid_next {
            set_uid_next(context, folder, new_uid_next).await?;
        }

        info!(context, "{} mails read from \"{}\".", read_cnt, folder);

        let msg_ids: Vec<MsgId> = received_msgs
            .iter()
            .flat_map(|m| m.msg_ids.clone())
            .collect();
        if !msg_ids.is_empty() {
            context.emit_event(EventType::IncomingMsgBunch { msg_ids });
        }

        chat::mark_old_messages_as_noticed(context, received_msgs).await?;

        Ok(read_cnt > 0)
    }

    /// Read the recipients from old emails sent by the user and add them as contacts.
    /// This way, we can already offer them some email addresses they can write to.
    ///
    /// Then, Fetch the last messages DC_FETCH_EXISTING_MSGS_COUNT emails from the server
    /// and show them in the chat list.
    pub(crate) async fn fetch_existing_msgs(&mut self, context: &Context) -> Result<()> {
        if context.get_config_bool(Config::Bot).await? {
            return Ok(()); // Bots don't want those messages
        }
        self.prepare(context).await.context("could not connect")?;

        add_all_recipients_as_contacts(context, self, Config::ConfiguredSentboxFolder)
            .await
            .context("failed to get recipients from the sentbox")?;
        add_all_recipients_as_contacts(context, self, Config::ConfiguredMvboxFolder)
            .await
            .context("failed to ge recipients from the movebox")?;
        add_all_recipients_as_contacts(context, self, Config::ConfiguredInboxFolder)
            .await
            .context("failed to get recipients from the inbox")?;

        if context.get_config_bool(Config::FetchExistingMsgs).await? {
            for meaning in [
                FolderMeaning::Mvbox,
                FolderMeaning::Inbox,
                FolderMeaning::Sent,
            ] {
                let config = match meaning.to_config() {
                    Some(c) => c,
                    None => continue,
                };
                if let Some(folder) = context.get_config(config).await? {
                    info!(
                        context,
                        "Fetching existing messages from folder {folder:?}."
                    );
                    self.fetch_new_messages(context, &folder, meaning, true)
                        .await
                        .context("could not fetch existing messages")?;
                }
            }
        }

        info!(context, "Done fetching existing messages.");
        Ok(())
    }

    /// Synchronizes UIDs for all folders.
    pub(crate) async fn resync_folders(&mut self, context: &Context) -> Result<()> {
        self.prepare(context).await?;

        let all_folders = self
            .list_folders(context)
            .await
            .context("listing folders for resync")?;
        for folder in all_folders {
            let folder_meaning = get_folder_meaning(&folder);
            if folder_meaning != FolderMeaning::Virtual {
                self.resync_folder_uids(context, folder.name(), folder_meaning)
                    .await?;
            }
        }
        Ok(())
    }
}

impl Session {
    /// Deletes batch of messages identified by their UID from the currently
    /// selected folder.
    async fn delete_message_batch(
        &mut self,
        context: &Context,
        uid_set: &str,
        row_ids: Vec<i64>,
    ) -> Result<()> {
        // mark the message for deletion
        self.add_flag_finalized_with_set(uid_set, "\\Deleted")
            .await?;
        context
            .sql
            .execute(
                &format!(
                    "DELETE FROM imap WHERE id IN ({})",
                    sql::repeat_vars(row_ids.len())
                ),
                rusqlite::params_from_iter(row_ids),
            )
            .await
            .context("cannot remove deleted messages from imap table")?;

        context.emit_event(EventType::ImapMessageDeleted(format!(
            "IMAP messages {uid_set} marked as deleted"
        )));
        Ok(())
    }

    /// Moves batch of messages identified by their UID from the currently
    /// selected folder to the target folder.
    async fn move_message_batch(
        &mut self,
        context: &Context,
        set: &str,
        row_ids: Vec<i64>,
        target: &str,
    ) -> Result<()> {
        if self.can_move() {
            match self.uid_mv(set, &target).await {
                Ok(()) => {
                    // Messages are moved or don't exist, IMAP returns OK response in both cases.
                    context
                        .sql
                        .execute(
                            &format!(
                                "DELETE FROM imap WHERE id IN ({})",
                                sql::repeat_vars(row_ids.len())
                            ),
                            rusqlite::params_from_iter(row_ids),
                        )
                        .await
                        .context("cannot delete moved messages from imap table")?;
                    context.emit_event(EventType::ImapMessageMoved(format!(
                        "IMAP messages {set} moved to {target}"
                    )));
                    return Ok(());
                }
                Err(err) => {
                    if context.should_delete_to_trash().await? {
                        error!(
                            context,
                            "Cannot move messages {} to {}, no fallback to COPY/DELETE because \
                            delete_to_trash is set. Error: {:#}",
                            set,
                            target,
                            err,
                        );
                        return Err(err.into());
                    }
                    warn!(
                        context,
                        "Cannot move messages, fallback to COPY/DELETE {} to {}: {}",
                        set,
                        target,
                        err
                    );
                }
            }
        }

        // Server does not support MOVE or MOVE failed.
        // Copy messages to the destination folder if needed and mark records for deletion.
        let copy = !context.is_trash(target).await?;
        if copy {
            info!(
                context,
                "Server does not support MOVE, fallback to COPY/DELETE {} to {}", set, target
            );
            self.uid_copy(&set, &target).await?;
        } else {
            error!(
                context,
                "Server does not support MOVE, fallback to DELETE {} to {}", set, target,
            );
        }
        context
            .sql
            .execute(
                &format!(
                    "UPDATE imap SET target='' WHERE id IN ({})",
                    sql::repeat_vars(row_ids.len())
                ),
                rusqlite::params_from_iter(row_ids),
            )
            .await
            .context("cannot plan deletion of messages")?;
        if copy {
            context.emit_event(EventType::ImapMessageMoved(format!(
                "IMAP messages {set} copied to {target}"
            )));
        }
        Ok(())
    }

    /// Moves and deletes messages as planned in the `imap` table.
    ///
    /// This is the only place where messages are moved or deleted on the IMAP server.
    async fn move_delete_messages(&mut self, context: &Context, folder: &str) -> Result<()> {
        let rows = context
            .sql
            .query_map(
                "SELECT id, uid, target FROM imap
        WHERE folder = ?
        AND target != folder
        ORDER BY target, uid",
                (folder,),
                |row| {
                    let rowid: i64 = row.get(0)?;
                    let uid: u32 = row.get(1)?;
                    let target: String = row.get(2)?;
                    Ok((rowid, uid, target))
                },
                |rows| rows.collect::<Result<Vec<_>, _>>().map_err(Into::into),
            )
            .await?;

        for (target, rowid_set, uid_set) in UidGrouper::from(rows) {
            // Select folder inside the loop to avoid selecting it if there are no pending
            // MOVE/DELETE operations. This does not result in multiple SELECT commands
            // being sent because `select_folder()` does nothing if the folder is already
            // selected.
            self.select_folder(context, Some(folder)).await?;

            // Empty target folder name means messages should be deleted.
            if target.is_empty() {
                self.delete_message_batch(context, &uid_set, rowid_set)
                    .await
                    .with_context(|| format!("cannot delete batch of messages {:?}", &uid_set))?;
            } else {
                self.move_message_batch(context, &uid_set, rowid_set, &target)
                    .await
                    .with_context(|| {
                        format!(
                            "cannot move batch of messages {:?} to folder {:?}",
                            &uid_set, target
                        )
                    })?;
            }
        }

        // Expunge folder if needed, e.g. if some jobs have
        // deleted messages on the server.
        if let Err(err) = self.maybe_close_folder(context).await {
            warn!(context, "failed to close folder: {:?}", err);
        }

        Ok(())
    }

    /// Stores pending `\Seen` flags for messages in `imap_markseen` table.
    pub(crate) async fn store_seen_flags_on_imap(&mut self, context: &Context) -> Result<()> {
        let rows = context
            .sql
            .query_map(
                "SELECT imap.id, uid, folder FROM imap, imap_markseen
                 WHERE imap.id = imap_markseen.id AND target = folder
                 ORDER BY folder, uid",
                [],
                |row| {
                    let rowid: i64 = row.get(0)?;
                    let uid: u32 = row.get(1)?;
                    let folder: String = row.get(2)?;
                    Ok((rowid, uid, folder))
                },
                |rows| rows.collect::<Result<Vec<_>, _>>().map_err(Into::into),
            )
            .await?;

        for (folder, rowid_set, uid_set) in UidGrouper::from(rows) {
            self.select_folder(context, Some(&folder))
                .await
                .context("failed to select folder")?;

            if let Err(err) = self.add_flag_finalized_with_set(&uid_set, "\\Seen").await {
                warn!(
                    context,
                    "Cannot mark messages {} in folder {} as seen, will retry later: {}.",
                    uid_set,
                    folder,
                    err
                );
            } else {
                info!(
                    context,
                    "Marked messages {} in folder {} as seen.", uid_set, folder
                );
                context
                    .sql
                    .execute(
                        &format!(
                            "DELETE FROM imap_markseen WHERE id IN ({})",
                            sql::repeat_vars(rowid_set.len())
                        ),
                        rusqlite::params_from_iter(rowid_set),
                    )
                    .await
                    .context("cannot remove messages marked as seen from imap_markseen table")?;
            }
        }

        Ok(())
    }
}

impl Imap {
    /// Synchronizes `\Seen` flags using `CONDSTORE` extension.
    pub(crate) async fn sync_seen_flags(&mut self, context: &Context, folder: &str) -> Result<()> {
        let session = self
            .session
            .as_mut()
            .with_context(|| format!("No IMAP connection established, folder: {folder}"))?;

        if !session.can_condstore() {
            info!(
                context,
                "Server does not support CONDSTORE, skipping flag synchronization."
            );
            return Ok(());
        }

        session
            .select_folder(context, Some(folder))
            .await
            .context("failed to select folder")?;

        let mailbox = session
            .selected_mailbox
            .as_ref()
            .with_context(|| format!("No mailbox selected, folder: {folder}"))?;

        // Check if the mailbox supports MODSEQ.
        // We are not interested in actual value of HIGHESTMODSEQ.
        if mailbox.highest_modseq.is_none() {
            info!(
                context,
                "Mailbox {} does not support mod-sequences, skipping flag synchronization.", folder
            );
            return Ok(());
        }

        let mut updated_chat_ids = BTreeSet::new();
        let uid_validity = get_uidvalidity(context, folder)
            .await
            .with_context(|| format!("failed to get UID validity for folder {folder}"))?;
        let mut highest_modseq = get_modseq(context, folder)
            .await
            .with_context(|| format!("failed to get MODSEQ for folder {folder}"))?;
        let mut list = session
            .uid_fetch("1:*", format!("(FLAGS) (CHANGEDSINCE {highest_modseq})"))
            .await
            .context("failed to fetch flags")?;

        while let Some(fetch) = list
            .try_next()
            .await
            .context("failed to get FETCH result")?
        {
            let uid = if let Some(uid) = fetch.uid {
                uid
            } else {
                info!(context, "FETCH result contains no UID, skipping");
                continue;
            };
            let is_seen = fetch.flags().any(|flag| flag == Flag::Seen);
            if is_seen {
                if let Some(chat_id) = mark_seen_by_uid(context, folder, uid_validity, uid)
                    .await
                    .with_context(|| {
                        format!("failed to update seen status for msg {folder}/{uid}")
                    })?
                {
                    updated_chat_ids.insert(chat_id);
                }
            }

            if let Some(modseq) = fetch.modseq {
                if modseq > highest_modseq {
                    highest_modseq = modseq;
                }
            } else {
                warn!(context, "FETCH result contains no MODSEQ");
            }
        }

        set_modseq(context, folder, highest_modseq)
            .await
            .with_context(|| format!("failed to set MODSEQ for folder {folder}"))?;
        for updated_chat_id in updated_chat_ids {
            context.emit_event(EventType::MsgsNoticed(updated_chat_id));
        }

        Ok(())
    }

    /// Gets the from, to and bcc addresses from all existing outgoing emails.
    pub async fn get_all_recipients(&mut self, context: &Context) -> Result<Vec<SingleInfo>> {
        let session = self
            .session
            .as_mut()
            .context("IMAP No Connection established")?;

        let mut uids: Vec<_> = session
            .uid_search(get_imap_self_sent_search_command(context).await?)
            .await?
            .into_iter()
            .collect();
        uids.sort_unstable();

        let mut result = Vec::new();
        for (_, uid_set) in build_sequence_sets(&uids)? {
            let mut list = session
                .uid_fetch(uid_set, "(UID BODY.PEEK[HEADER.FIELDS (FROM TO CC BCC)])")
                .await
                .context("IMAP Could not fetch")?;

            while let Some(msg) = list.try_next().await? {
                match get_fetch_headers(&msg) {
                    Ok(headers) => {
                        if let Some(from) = mimeparser::get_from(&headers) {
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

    /// Prefetch all messages greater than or equal to `uid_next`. Returns a list of fetch results
    /// in the order of ascending delivery time to the server (INTERNALDATE).
    async fn prefetch(&mut self, uid_next: u32) -> Result<Vec<(u32, async_imap::types::Fetch)>> {
        let session = self
            .session
            .as_mut()
            .context("no IMAP connection established")?;

        // fetch messages with larger UID than the last one seen
        let set = format!("{uid_next}:*");
        let mut list = session
            .uid_fetch(set, PREFETCH_FLAGS)
            .await
            .context("IMAP could not fetch")?;

        let mut msgs = BTreeMap::new();
        while let Some(msg) = list.try_next().await? {
            if let Some(msg_uid) = msg.uid {
                // If the mailbox is not empty, results always include
                // at least one UID, even if last_seen_uid+1 is past
                // the last UID in the mailbox.  It happens because
                // uid:* is interpreted the same way as *:uid.
                // See <https://tools.ietf.org/html/rfc3501#page-61> for
                // standard reference. Therefore, sometimes we receive
                // already seen messages and have to filter them out.
                if msg_uid >= uid_next {
                    msgs.insert((msg.internal_date(), msg_uid), msg);
                }
            }
        }

        Ok(msgs.into_iter().map(|((_, uid), msg)| (uid, msg)).collect())
    }

    /// Like fetch_after(), but not for new messages but existing ones (the DC_FETCH_EXISTING_MSGS_COUNT newest messages)
    async fn prefetch_existing_msgs(&mut self) -> Result<Vec<(u32, async_imap::types::Fetch)>> {
        let session = self.session.as_mut().context("no IMAP session")?;
        let exists: i64 = {
            let mailbox = session.selected_mailbox.as_ref().context("no mailbox")?;
            mailbox.exists.into()
        };

        // Fetch last DC_FETCH_EXISTING_MSGS_COUNT (100) messages.
        // Sequence numbers are sequential. If there are 1000 messages in the inbox,
        // we can fetch the sequence numbers 900-1000 and get the last 100 messages.
        let first = cmp::max(1, exists - DC_FETCH_EXISTING_MSGS_COUNT + 1);
        let set = format!("{first}:{exists}");
        let mut list = session
            .fetch(&set, PREFETCH_FLAGS)
            .await
            .context("IMAP Could not fetch")?;

        let mut msgs = BTreeMap::new();
        while let Some(msg) = list.try_next().await? {
            if let Some(msg_uid) = msg.uid {
                msgs.insert((msg.internal_date(), msg_uid), msg);
            }
        }

        Ok(msgs.into_iter().map(|((_, uid), msg)| (uid, msg)).collect())
    }

    /// Fetches a list of messages by server UID.
    ///
    /// Returns the last UID fetched successfully and the info about each downloaded message.
    /// If the message is incorrect or there is a failure to write a message to the database,
    /// it is skipped and the error is logged.
    pub(crate) async fn fetch_many_msgs(
        &mut self,
        context: &Context,
        folder: &str,
        request_uids: Vec<u32>,
        uid_message_ids: &BTreeMap<u32, String>,
        fetch_partially: bool,
        fetching_existing_messages: bool,
    ) -> Result<(Option<u32>, Vec<ReceivedMsg>)> {
        let mut last_uid = None;
        let mut received_msgs = Vec::new();

        if request_uids.is_empty() {
            return Ok((last_uid, received_msgs));
        }

        let session = self.session.as_mut().context("no IMAP session")?;
        for (request_uids, set) in build_sequence_sets(&request_uids)? {
            info!(
                context,
                "Starting a {} FETCH of message set \"{}\".",
                if fetch_partially { "partial" } else { "full" },
                set
            );
            let mut fetch_responses = session
                .uid_fetch(
                    &set,
                    if fetch_partially {
                        BODY_PARTIAL
                    } else {
                        BODY_FULL
                    },
                )
                .await
                .with_context(|| {
                    format!("fetching messages {} from folder \"{}\"", &set, folder)
                })?;

            // Map from UIDs to unprocessed FETCH results. We put unprocessed FETCH results here
            // when we want to process other messages first.
            let mut uid_msgs = HashMap::with_capacity(request_uids.len());

            let mut count = 0;
            for &request_uid in &request_uids {
                // Check if FETCH response is already in `uid_msgs`.
                let mut fetch_response = uid_msgs.remove(&request_uid);

                // Try to find a requested UID in returned FETCH responses.
                while fetch_response.is_none() {
                    let next_fetch_response =
                        if let Some(next_fetch_response) = fetch_responses.next().await {
                            next_fetch_response
                        } else {
                            // No more FETCH responses received from the server.
                            break;
                        };

                    let next_fetch_response =
                        next_fetch_response.context("Failed to process IMAP FETCH result")?;

                    if let Some(next_uid) = next_fetch_response.uid {
                        if next_uid == request_uid {
                            fetch_response = Some(next_fetch_response);
                        } else if !request_uids.contains(&next_uid) {
                            // (size of `request_uids` is bounded by IMAP command length limit,
                            // search in this vector is always fast)

                            // Unwanted UIDs are possible because of unsolicited responses, e.g. if
                            // another client changes \Seen flag on a message after we do a prefetch but
                            // before fetch. It's not an error if we receive such unsolicited response.
                            info!(
                                context,
                                "Skipping not requested FETCH response for UID {}.", next_uid
                            );
                        } else if uid_msgs.insert(next_uid, next_fetch_response).is_some() {
                            warn!(context, "Got duplicated UID {}.", next_uid);
                        }
                    } else {
                        info!(context, "Skipping FETCH response without UID.");
                    }
                }

                let fetch_response = match fetch_response {
                    Some(fetch) => fetch,
                    None => {
                        warn!(
                            context,
                            "Missed UID {} in the server response.", request_uid
                        );
                        continue;
                    }
                };
                count += 1;

                let is_deleted = fetch_response.flags().any(|flag| flag == Flag::Deleted);
                let (body, partial) = if fetch_partially {
                    (fetch_response.header(), fetch_response.size) // `BODY.PEEK[HEADER]` goes to header() ...
                } else {
                    (fetch_response.body(), None) // ... while `BODY.PEEK[]` goes to body() - and includes header()
                };

                if is_deleted {
                    info!(context, "Not processing deleted msg {}.", request_uid);
                    last_uid = Some(request_uid);
                    continue;
                }

                let body = if let Some(body) = body {
                    body
                } else {
                    info!(
                        context,
                        "Not processing message {} without a BODY.", request_uid
                    );
                    last_uid = Some(request_uid);
                    continue;
                };

                let is_seen = fetch_response.flags().any(|flag| flag == Flag::Seen);

                let rfc724_mid = if let Some(rfc724_mid) = uid_message_ids.get(&request_uid) {
                    rfc724_mid
                } else {
                    error!(
                        context,
                        "No Message-ID corresponding to UID {} passed in uid_messsage_ids.",
                        request_uid
                    );
                    continue;
                };

                info!(
                    context,
                    "Passing message UID {} to receive_imf().", request_uid
                );
                match receive_imf_inner(
                    context,
                    rfc724_mid,
                    body,
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
                    }
                    Err(err) => {
                        warn!(context, "receive_imf error: {:#}.", err);
                    }
                };
                last_uid = Some(request_uid)
            }

            // If we don't process the whole response, IMAP client is left in a broken state where
            // it will try to process the rest of response as the next response.
            while fetch_responses.next().await.is_some() {}

            if count != request_uids.len() {
                warn!(
                    context,
                    "Failed to fetch all UIDs: got {}, requested {}, we requested the UIDs {:?}.",
                    count,
                    request_uids.len(),
                    request_uids,
                );
            } else {
                info!(
                    context,
                    "Successfully received {} UIDs.",
                    request_uids.len()
                );
            }
        }

        Ok((last_uid, received_msgs))
    }
}

impl Session {
    /// Returns success if we successfully set the flag or we otherwise
    /// think add_flag should not be retried: Disconnection during setting
    /// the flag, or other imap-errors, returns true as well.
    ///
    /// Returning error means that the operation can be retried.
    async fn add_flag_finalized_with_set(&mut self, uid_set: &str, flag: &str) -> Result<()> {
        if flag == "\\Deleted" {
            self.selected_folder_needs_expunge = true;
        }
        let query = format!("+FLAGS ({flag})");
        let mut responses = self
            .uid_store(uid_set, &query)
            .await
            .with_context(|| format!("IMAP failed to store: ({uid_set}, {query})"))?;
        while let Some(_response) = responses.next().await {
            // Read all the responses
        }
        Ok(())
    }
}

impl Imap {
    pub(crate) async fn prepare_imap_operation_on_msg(
        &mut self,
        context: &Context,
        folder: &str,
        uid: u32,
    ) -> Option<ImapActionResult> {
        if uid == 0 {
            return Some(ImapActionResult::RetryLater);
        }
        if let Err(err) = self.prepare(context).await {
            warn!(context, "prepare_imap_op failed: {}", err);
            return Some(ImapActionResult::RetryLater);
        }

        let session = match self
            .session
            .as_mut()
            .context("no IMAP connection established")
        {
            Err(err) => {
                error!(context, "Failed to prepare IMAP operation: {:#}", err);
                return Some(ImapActionResult::Failed);
            }
            Ok(session) => session,
        };

        match session.select_folder(context, Some(folder)).await {
            Ok(_) => None,
            Err(select_folder::Error::ConnectionLost) => {
                warn!(context, "Lost imap connection");
                Some(ImapActionResult::RetryLater)
            }
            Err(select_folder::Error::BadFolderName(folder_name)) => {
                warn!(context, "invalid folder name: {:?}", folder_name);
                Some(ImapActionResult::Failed)
            }
            Err(err) => {
                warn!(context, "failed to select folder {:?}: {:#}", folder, err);
                Some(ImapActionResult::RetryLater)
            }
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

    /// Attempts to configure mvbox.
    ///
    /// Tries to find any folder in the given list of `folders`. If none is found, tries to create
    /// any of them in the same order. This method does not use LIST command to ensure that
    /// configuration works even if mailbox lookup is forbidden via Access Control List (see
    /// <https://datatracker.ietf.org/doc/html/rfc4314>).
    ///
    /// Returns first found or created folder name.
    async fn configure_mvbox<'a>(
        &mut self,
        context: &Context,
        folders: &[&'a str],
        create_mvbox: bool,
    ) -> Result<Option<&'a str>> {
        let session = self
            .session
            .as_mut()
            .context("no IMAP connection established")?;

        // Close currently selected folder if needed.
        // We are going to select folders using low-level EXAMINE operations below.
        session.select_folder(context, None).await?;

        for folder in folders {
            info!(context, "Looking for MVBOX-folder \"{}\"...", &folder);
            let res = session.examine(&folder).await;
            if res.is_ok() {
                info!(
                    context,
                    "MVBOX-folder {:?} successfully selected, using it.", &folder
                );
                session.close().await?;
                return Ok(Some(folder));
            }
        }

        if create_mvbox {
            for folder in folders {
                match session.create(&folder).await {
                    Ok(_) => {
                        info!(context, "MVBOX-folder {} created.", &folder);
                        return Ok(Some(folder));
                    }
                    Err(err) => {
                        warn!(context, "Cannot create MVBOX-folder {:?}: {}", &folder, err);
                    }
                }
            }
        }

        Ok(None)
    }

    pub async fn configure_folders(&mut self, context: &Context, create_mvbox: bool) -> Result<()> {
        let session = self
            .session
            .as_mut()
            .context("no IMAP connection established")?;

        let mut folders = session
            .list(Some(""), Some("*"))
            .await
            .context("list_folders failed")?;
        let mut delimiter = ".".to_string();
        let mut delimiter_is_default = true;
        let mut folder_configs = BTreeMap::new();

        while let Some(folder) = folders.try_next().await? {
            info!(context, "Scanning folder: {:?}", folder);

            // Update the delimiter iff there is a different one, but only once.
            if let Some(d) = folder.delimiter() {
                if delimiter_is_default && !d.is_empty() && delimiter != d {
                    delimiter = d.to_string();
                    delimiter_is_default = false;
                }
            }

            let folder_meaning = get_folder_meaning_by_attrs(folder.attributes());
            let folder_name_meaning = get_folder_meaning_by_name(folder.name());
            if let Some(config) = folder_meaning.to_config() {
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

        let fallback_folder = format!("INBOX{delimiter}DeltaChat");
        let mvbox_folder = self
            .configure_mvbox(context, &["DeltaChat", &fallback_folder], create_mvbox)
            .await
            .context("failed to configure mvbox")?;

        context
            .set_config(Config::ConfiguredInboxFolder, Some("INBOX"))
            .await?;
        if let Some(mvbox_folder) = mvbox_folder {
            info!(context, "Setting MVBOX FOLDER TO {}", &mvbox_folder);
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
}

impl Session {
    /// Return whether the server sent an unsolicited EXISTS response.
    /// Drains all responses from `session.unsolicited_responses` in the process.
    /// If this returns `true`, this means that new emails arrived and you should
    /// fetch again, even if you just fetched.
    fn server_sent_unsolicited_exists(&self, context: &Context) -> Result<bool> {
        use async_imap::imap_proto::Response;
        use async_imap::imap_proto::ResponseCode;
        use UnsolicitedResponse::*;

        let mut unsolicited_exists = false;
        while let Ok(response) = self.unsolicited_responses.try_recv() {
            match response {
                Exists(_) => {
                    info!(
                        context,
                        "Need to fetch again, got unsolicited EXISTS {:?}", response
                    );
                    unsolicited_exists = true;
                }

                // We are not interested in the following responses and they are are
                // sent quite frequently, so, we ignore them without logging them
                Expunge(_) | Recent(_) => {}
                Other(response_data)
                    if matches!(
                        response_data.parsed(),
                        Response::Fetch { .. }
                            | Response::Done {
                                code: Some(ResponseCode::CopyUid(_, _, _)),
                                ..
                            }
                    ) => {}

                _ => {
                    info!(context, "got unsolicited response {:?}", response)
                }
            }
        }
        Ok(unsolicited_exists)
    }
}

async fn should_move_out_of_spam(
    context: &Context,
    headers: &[mailparse::MailHeader<'_>],
) -> Result<bool> {
    if headers.get_header_value(HeaderDef::ChatVersion).is_some() {
        // If this is a chat message (i.e. has a ChatVersion header), then this might be
        // a securejoin message. We can't find out at this point as we didn't prefetch
        // the SecureJoin header. So, we always move chat messages out of Spam.
        // Two possibilities to change this would be:
        // 1. Remove the `&& !context.is_spam_folder(folder).await?` check from
        // `fetch_new_messages()`, and then let `receive_imf()` check
        // if it's a spam message and should be hidden.
        // 2. Or add a flag to the ChatVersion header that this is a securejoin
        // request, and return `true` here only if the message has this flag.
        // `receive_imf()` can then check if the securejoin request is valid.
        return Ok(true);
    }

    if let Some(msg) = get_prefetch_parent_message(context, headers).await? {
        if msg.chat_blocked != Blocked::Not {
            // Blocked or contact request message in the spam folder, leave it there.
            return Ok(false);
        }
    } else {
        let from = match mimeparser::get_from(headers) {
            Some(f) => f,
            None => return Ok(false),
        };
        // No chat found.
        let (from_id, blocked_contact, _origin) =
            match from_field_to_contact_id(context, &from, true)
                .await
                .context("from_field_to_contact_id")?
            {
                Some(res) => res,
                None => {
                    warn!(
                        context,
                        "Contact with From address {:?} cannot exist, not moving out of spam", from
                    );
                    return Ok(false);
                }
            };
        if blocked_contact {
            // Contact is blocked, leave the message in spam.
            return Ok(false);
        }

        if let Some(chat_id_blocked) = ChatIdBlocked::lookup_by_contact(context, from_id).await? {
            if chat_id_blocked.blocked != Blocked::Not {
                return Ok(false);
            }
        } else if from_id != ContactId::SELF {
            // No chat with this contact found.
            return Ok(false);
        }
    }

    Ok(true)
}

/// Returns target folder for a message found in the Spam folder.
/// If this returns None, the message will not be moved out of the
/// Spam folder, and as `fetch_new_messages()` doesn't download
/// messages from the Spam folder, the message will be ignored.
async fn spam_target_folder_cfg(
    context: &Context,
    headers: &[mailparse::MailHeader<'_>],
) -> Result<Option<Config>> {
    if !should_move_out_of_spam(context, headers).await? {
        return Ok(None);
    }

    if needs_move_to_mvbox(context, headers).await?
        // If OnlyFetchMvbox is set, we don't want to move the message to
        // the inbox or sentbox where we wouldn't fetch it again:
        || context.get_config_bool(Config::OnlyFetchMvbox).await?
    {
        Ok(Some(Config::ConfiguredMvboxFolder))
    } else {
        Ok(Some(Config::ConfiguredInboxFolder))
    }
}

/// Returns `ConfiguredInboxFolder`, `ConfiguredMvboxFolder` or `ConfiguredSentboxFolder` if
/// the message needs to be moved from `folder`. Otherwise returns `None`.
pub async fn target_folder_cfg(
    context: &Context,
    folder: &str,
    folder_meaning: FolderMeaning,
    headers: &[mailparse::MailHeader<'_>],
) -> Result<Option<Config>> {
    if context.is_mvbox(folder).await? {
        return Ok(None);
    }

    if folder_meaning == FolderMeaning::Spam {
        spam_target_folder_cfg(context, headers).await
    } else if needs_move_to_mvbox(context, headers).await? {
        Ok(Some(Config::ConfiguredMvboxFolder))
    } else {
        Ok(None)
    }
}

pub async fn target_folder(
    context: &Context,
    folder: &str,
    folder_meaning: FolderMeaning,
    headers: &[mailparse::MailHeader<'_>],
) -> Result<String> {
    match target_folder_cfg(context, folder, folder_meaning, headers).await? {
        Some(config) => match context.get_config(config).await? {
            Some(target) => Ok(target),
            None => Ok(folder.to_string()),
        },
        None => Ok(folder.to_string()),
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

fn get_folder_meaning_by_attrs(folder_attrs: &[NameAttribute]) -> FolderMeaning {
    for attr in folder_attrs {
        match attr {
            NameAttribute::Trash => return FolderMeaning::Trash,
            NameAttribute::Sent => return FolderMeaning::Sent,
            NameAttribute::Junk => return FolderMeaning::Spam,
            NameAttribute::Drafts => return FolderMeaning::Drafts,
            NameAttribute::All | NameAttribute::Flagged => return FolderMeaning::Virtual,
            NameAttribute::Extension(ref label) => {
                match label.as_ref() {
                    "\\Spam" => return FolderMeaning::Spam,
                    "\\Important" => return FolderMeaning::Virtual,
                    _ => {}
                };
            }
            _ => {}
        }
    }
    FolderMeaning::Unknown
}

pub(crate) fn get_folder_meaning(folder: &Name) -> FolderMeaning {
    match get_folder_meaning_by_attrs(folder.attributes()) {
        FolderMeaning::Unknown => get_folder_meaning_by_name(folder.name()),
        meaning => meaning,
    }
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

pub(crate) fn prefetch_get_message_id(headers: &[mailparse::MailHeader]) -> Option<String> {
    headers
        .get_header_value(HeaderDef::XMicrosoftOriginalMessageId)
        .or_else(|| headers.get_header_value(HeaderDef::MessageId))
        .and_then(|msgid| mimeparser::parse_message_id(&msgid).ok())
}

pub(crate) fn create_message_id() -> String {
    format!("{}{}", GENERATED_PREFIX, create_id())
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
) -> Result<bool> {
    if message::rfc724_mid_exists(context, message_id)
        .await?
        .is_some()
    {
        markseen_on_imap_table(context, message_id).await?;
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

    let from = match mimeparser::get_from(headers) {
        Some(f) => f,
        None => return Ok(false),
    };
    let (_from_id, blocked_contact, origin) =
        match from_field_to_contact_id(context, &from, true).await? {
            Some(res) => res,
            None => return Ok(false),
        };
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

    let show_emails =
        ShowEmails::from_i32(context.get_config_int(Config::ShowEmails).await?).unwrap_or_default();

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

/// Marks messages in `msgs` table as seen, searching for them by UID.
///
/// Returns updated chat ID if any message was marked as seen.
async fn mark_seen_by_uid(
    context: &Context,
    folder: &str,
    uid_validity: u32,
    uid: u32,
) -> Result<Option<ChatId>> {
    if let Some((msg_id, chat_id)) = context
        .sql
        .query_row_optional(
            "SELECT id, chat_id FROM msgs
                 WHERE id > 9 AND rfc724_mid IN (
                   SELECT rfc724_mid FROM imap
                   WHERE folder=?1
                   AND uidvalidity=?2
                   AND uid=?3
                   LIMIT 1
                 )",
            (&folder, uid_validity, uid),
            |row| {
                let msg_id: MsgId = row.get(0)?;
                let chat_id: ChatId = row.get(1)?;
                Ok((msg_id, chat_id))
            },
        )
        .await
        .with_context(|| format!("failed to get msg and chat ID for IMAP message {folder}/{uid}"))?
    {
        let updated = context
            .sql
            .execute(
                "UPDATE msgs SET state=?1
                     WHERE (state=?2 OR state=?3)
                     AND id=?4",
                (
                    MessageState::InSeen,
                    MessageState::InFresh,
                    MessageState::InNoticed,
                    msg_id,
                ),
            )
            .await
            .with_context(|| format!("failed to update msg {msg_id} state"))?
            > 0;

        if updated {
            msg_id
                .start_ephemeral_timer(context)
                .await
                .with_context(|| format!("failed to start ephemeral timer for message {msg_id}"))?;
            Ok(Some(chat_id))
        } else {
            // Message state has not changed.
            Ok(None)
        }
    } else {
        // There is no message is `msgs` table matching the given UID.
        Ok(None)
    }
}

/// Schedule marking the message as Seen on IMAP by adding all known IMAP messages corresponding to
/// the given Message-ID to `imap_markseen` table.
pub(crate) async fn markseen_on_imap_table(context: &Context, message_id: &str) -> Result<()> {
    context
        .sql
        .execute(
            "INSERT OR IGNORE INTO imap_markseen (id)
             SELECT id FROM imap WHERE rfc724_mid=?",
            (message_id,),
        )
        .await?;
    context.scheduler.interrupt_inbox().await;

    Ok(())
}

/// uid_next is the next unique identifier value from the last time we fetched a folder
/// See <https://tools.ietf.org/html/rfc3501#section-2.3.1.1>
/// This function is used to update our uid_next after fetching messages.
pub(crate) async fn set_uid_next(context: &Context, folder: &str, uid_next: u32) -> Result<()> {
    context
        .sql
        .execute(
            "INSERT INTO imap_sync (folder, uid_next) VALUES (?,?)
                ON CONFLICT(folder) DO UPDATE SET uid_next=excluded.uid_next",
            (folder, uid_next),
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
        .query_get_value("SELECT uid_next FROM imap_sync WHERE folder=?;", (folder,))
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
            "INSERT INTO imap_sync (folder, uidvalidity) VALUES (?,?)
                ON CONFLICT(folder) DO UPDATE SET uidvalidity=excluded.uidvalidity",
            (folder, uidvalidity),
        )
        .await?;
    Ok(())
}

async fn get_uidvalidity(context: &Context, folder: &str) -> Result<u32> {
    Ok(context
        .sql
        .query_get_value(
            "SELECT uidvalidity FROM imap_sync WHERE folder=?;",
            (folder,),
        )
        .await?
        .unwrap_or(0))
}

pub(crate) async fn set_modseq(context: &Context, folder: &str, modseq: u64) -> Result<()> {
    context
        .sql
        .execute(
            "INSERT INTO imap_sync (folder, modseq) VALUES (?,?)
                ON CONFLICT(folder) DO UPDATE SET modseq=excluded.modseq",
            (folder, modseq),
        )
        .await?;
    Ok(())
}

async fn get_modseq(context: &Context, folder: &str) -> Result<u64> {
    Ok(context
        .sql
        .query_get_value("SELECT modseq FROM imap_sync WHERE folder=?;", (folder,))
        .await?
        .unwrap_or(0))
}

/// Compute the imap search expression for all self-sent mails (for all self addresses)
pub(crate) async fn get_imap_self_sent_search_command(context: &Context) -> Result<String> {
    // See https://www.rfc-editor.org/rfc/rfc3501#section-6.4.4 for syntax of SEARCH and OR
    let mut search_command = format!("FROM \"{}\"", context.get_primary_self_addr().await?);

    for item in context.get_secondary_self_addrs().await? {
        search_command = format!("OR ({search_command}) (FROM \"{item}\")");
    }

    Ok(search_command)
}

/// Deprecated, use get_uid_next() and get_uidvalidity()
pub async fn get_config_last_seen_uid(context: &Context, folder: &str) -> Result<(u32, u32)> {
    let key = format!("imap.mailbox.{folder}");
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

/// Whether to ignore fetching messages from a folder.
///
/// This caters for the [`Config::OnlyFetchMvbox`] setting which means mails from folders
/// not explicitly watched should not be fetched.
async fn should_ignore_folder(
    context: &Context,
    folder: &str,
    folder_meaning: FolderMeaning,
) -> Result<bool> {
    if !context.get_config_bool(Config::OnlyFetchMvbox).await? {
        return Ok(false);
    }
    if context.is_sentbox(folder).await? {
        // Still respect the SentboxWatch setting.
        return Ok(!context.get_config_bool(Config::SentboxWatch).await?);
    }
    Ok(!(context.is_mvbox(folder).await? || folder_meaning == FolderMeaning::Spam))
}

/// Builds a list of sequence/uid sets. The returned sets have each no more than around 1000
/// characters because according to <https://tools.ietf.org/html/rfc2683#section-3.2.1.5>
/// command lines should not be much more than 1000 chars (servers should allow at least 8000 chars)
fn build_sequence_sets(uids: &[u32]) -> Result<Vec<(Vec<u32>, String)>> {
    // first, try to find consecutive ranges:
    let mut ranges: Vec<UidRange> = vec![];

    for &current in uids {
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
    let mut result = vec![];
    let (mut last_uids, mut last_str) = (Vec::new(), String::new());
    for range in ranges {
        last_uids.reserve((range.end - range.start + 1).try_into()?);
        (range.start..=range.end).for_each(|u| last_uids.push(u));
        if !last_str.is_empty() {
            last_str.push(',');
        }
        last_str.push_str(&range.to_string());

        if last_str.len() > 990 {
            result.push((take(&mut last_uids), take(&mut last_str)));
        }
    }
    result.push((last_uids, last_str));

    result.retain(|(_, s)| !s.is_empty());
    Ok(result)
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
async fn add_all_recipients_as_contacts(
    context: &Context,
    imap: &mut Imap,
    folder: Config,
) -> Result<()> {
    let mailbox = if let Some(m) = context.get_config(folder).await? {
        m
    } else {
        info!(
            context,
            "Folder {} is not configured, skipping fetching contacts from it.", folder
        );
        return Ok(());
    };
    imap.select_with_uidvalidity(context, &mailbox)
        .await
        .with_context(|| format!("could not select {mailbox}"))?;

    let recipients = imap
        .get_all_recipients(context)
        .await
        .context("could not get recipients")?;

    let mut any_modified = false;
    for recipient in recipients {
        let display_name_normalized = recipient
            .display_name
            .as_ref()
            .map(|s| normalize_name(s))
            .unwrap_or_default();

        let recipient_addr = match ContactAddress::new(&recipient.addr) {
            Err(err) => {
                warn!(
                    context,
                    "Could not add contact for recipient with address {:?}: {:#}",
                    recipient.addr,
                    err
                );
                continue;
            }
            Ok(recipient_addr) => recipient_addr,
        };

        let (_, modified) = Contact::add_or_lookup(
            context,
            &display_name_normalized,
            &recipient_addr,
            Origin::OutgoingTo,
        )
        .await?;
        if modified != Modifier::None {
            any_modified = true;
        }
    }
    if any_modified {
        context.emit_event(EventType::ContactsChanged(None));
    }

    Ok(())
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
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
        assert_eq!(build_sequence_sets(&[]).unwrap(), vec![]);

        let cases = vec![
            (vec![1], "1"),
            (vec![3291], "3291"),
            (vec![1, 3, 5, 7, 9, 11], "1,3,5,7,9,11"),
            (vec![1, 2, 3], "1:3"),
            (vec![1, 4, 5, 6], "1,4:6"),
            ((1..=500).collect(), "1:500"),
            (vec![3, 4, 8, 9, 10, 11, 39, 50, 2], "3:4,8:11,39,50,2"),
        ];
        for (input, s) in cases {
            assert_eq!(
                build_sequence_sets(&input).unwrap(),
                vec![(input, s.into())]
            );
        }

        let has_number = |(uids, s): &(Vec<u32>, String), number| {
            uids.iter().any(|&n| n == number)
                && s.split(',').any(|n| n.parse::<u32>().unwrap() == number)
        };

        let numbers: Vec<_> = (2..=500).step_by(2).collect();
        let result = build_sequence_sets(&numbers).unwrap();
        for (_, set) in &result {
            assert!(set.len() < 1010);
            assert!(!set.ends_with(','));
            assert!(!set.starts_with(','));
        }
        assert!(result.len() == 1); // these UIDs fit in one set
        for &number in &numbers {
            assert!(result.iter().any(|r| has_number(r, number)));
        }

        let numbers: Vec<_> = (1..=1000).step_by(3).collect();
        let result = build_sequence_sets(&numbers).unwrap();
        for (_, set) in &result {
            assert!(set.len() < 1010);
            assert!(!set.ends_with(','));
            assert!(!set.starts_with(','));
        }
        let (last_uids, last_str) = result.last().unwrap();
        assert_eq!(
            last_uids.get((last_uids.len() - 2)..).unwrap(),
            &[997, 1000]
        );
        assert!(last_str.ends_with("997,1000"));
        assert!(result.len() == 2); // This time we need 2 sets
        for &number in &numbers {
            assert!(result.iter().any(|r| has_number(r, number)));
        }

        let numbers: Vec<_> = (30000000..=30002500).step_by(4).collect();
        let result = build_sequence_sets(&numbers).unwrap();
        for (_, set) in &result {
            assert!(set.len() < 1010);
            assert!(!set.ends_with(','));
            assert!(!set.starts_with(','));
        }
        assert_eq!(result.len(), 6);
        for &number in &numbers {
            assert!(result.iter().any(|r| has_number(r, number)));
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
    ) -> Result<()> {
        println!("Testing: For folder {folder}, mvbox_move {mvbox_move}, chat_msg {chat_msg}, accepted {accepted_chat}, outgoing {outgoing}, setupmessage {setupmessage}");

        let t = TestContext::new_alice().await;
        t.ctx
            .set_config(Config::ConfiguredMvboxFolder, Some("DeltaChat"))
            .await?;
        t.ctx
            .set_config(Config::ConfiguredSentboxFolder, Some("Sent"))
            .await?;
        t.ctx
            .set_config(Config::MvboxMove, Some(if mvbox_move { "1" } else { "0" }))
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
        let actual = if let Some(config) =
            target_folder_cfg(&t, folder, get_folder_meaning_by_name(folder), &headers).await?
        {
            t.get_config(config).await?
        } else {
            None
        };

        let expected = if expected_destination == folder {
            None
        } else {
            Some(expected_destination)
        };
        assert_eq!(expected, actual.as_deref(), "For folder {folder}, mvbox_move {mvbox_move}, chat_msg {chat_msg}, accepted {accepted_chat}, outgoing {outgoing}, setupmessage {setupmessage}: expected {expected:?}, got {actual:?}");
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

    // These are the same as above, but non-chat messages in Spam stay in Spam
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
        ("Spam", false, true, "INBOX"),
        ("Spam", true, false, "Spam"),
        ("Spam", true, true, "DeltaChat"),
    ];

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
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
            )
            .await?;
        }
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
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
            )
            .await?;
        }
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_target_folder_outgoing() -> Result<()> {
        // Test outgoing emails
        for (folder, mvbox_move, chat_msg, expected_destination) in COMBINATIONS_ACCEPTED_CHAT {
            check_target_folder_combination(
                folder,
                *mvbox_move,
                *chat_msg,
                expected_destination,
                true,
                true,
                false,
            )
            .await?;
        }
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
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
            )
            .await?;
        }
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_get_imap_search_command() -> Result<()> {
        let t = TestContext::new_alice().await;
        assert_eq!(
            get_imap_self_sent_search_command(&t.ctx).await?,
            r#"FROM "alice@example.org""#
        );

        t.ctx.set_primary_self_addr("alice@another.com").await?;
        assert_eq!(
            get_imap_self_sent_search_command(&t.ctx).await?,
            r#"OR (FROM "alice@another.com") (FROM "alice@example.org")"#
        );

        t.ctx.set_primary_self_addr("alice@third.com").await?;
        assert_eq!(
            get_imap_self_sent_search_command(&t.ctx).await?,
            r#"OR (OR (FROM "alice@third.com") (FROM "alice@another.com")) (FROM "alice@example.org")"#
        );

        Ok(())
    }

    #[test]
    fn test_uid_grouper() {
        // Input: sequence of (rowid: i64, uid: u32, target: String)
        // Output: sequence of (target: String, rowid_set: Vec<i64>, uid_set: String)
        let grouper = UidGrouper::from([(1, 2, "INBOX".to_string())]);
        let res: Vec<(String, Vec<i64>, String)> = grouper.into_iter().collect();
        assert_eq!(res, vec![("INBOX".to_string(), vec![1], "2".to_string())]);

        let grouper = UidGrouper::from([(1, 2, "INBOX".to_string()), (2, 3, "INBOX".to_string())]);
        let res: Vec<(String, Vec<i64>, String)> = grouper.into_iter().collect();
        assert_eq!(
            res,
            vec![("INBOX".to_string(), vec![1, 2], "2:3".to_string())]
        );

        let grouper = UidGrouper::from([
            (1, 2, "INBOX".to_string()),
            (2, 2, "INBOX".to_string()),
            (3, 3, "INBOX".to_string()),
        ]);
        let res: Vec<(String, Vec<i64>, String)> = grouper.into_iter().collect();
        assert_eq!(
            res,
            vec![("INBOX".to_string(), vec![1, 2, 3], "2:3".to_string())]
        );
    }
}
