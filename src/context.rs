//! Context module.

use std::collections::{BTreeMap, HashMap};
use std::ffi::OsString;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

use anyhow::{bail, ensure, Context as _, Result};
use async_channel::{self as channel, Receiver, Sender};
use ratelimit::Ratelimit;
use tokio::sync::{Mutex, RwLock};
use tokio::task;

use crate::chat::{get_chat_cnt, ChatId};
use crate::config::Config;
use crate::constants::DC_VERSION_STR;
use crate::contact::Contact;
use crate::debug_logging::DebugEventLogData;
use crate::events::{Event, EventEmitter, EventType, Events};
use crate::key::{DcKey, SignedPublicKey};
use crate::login_param::LoginParam;
use crate::message::{self, MessageState, MsgId};
use crate::qr::Qr;
use crate::quota::QuotaInfo;
use crate::scheduler::Scheduler;
use crate::sql::Sql;
use crate::stock_str::StockStrings;
use crate::tools::{duration_to_str, time};

/// Builder for the [`Context`].
///
/// Many arguments to the [`Context`] are kind of optional and only needed to handle
/// multiple contexts, for which the [account manager](crate::accounts::Accounts) should be
/// used.  This builder makes creating a new context simpler, especially for the
/// standalone-context case.
///
/// # Examples
///
/// Creating a new unecrypted database:
///
/// ```
/// # let rt = tokio::runtime::Runtime::new().unwrap();
/// # rt.block_on(async move {
/// use deltachat::context::ContextBuilder;
///
/// let dir = tempfile::tempdir().unwrap();
/// let context = ContextBuilder::new(dir.path().join("db"))
///      .open()
///      .await
///      .unwrap();
/// drop(context);
/// # });
/// ```
///
/// To use an encrypted database provide a password.  If the database does not yet exist it
/// will be created:
///
/// ```
/// # let rt = tokio::runtime::Runtime::new().unwrap();
/// # rt.block_on(async move {
/// use deltachat::context::ContextBuilder;
///
/// let dir = tempfile::tempdir().unwrap();
/// let context = ContextBuilder::new(dir.path().join("db"))
///      .with_password("secret".into())
///      .open()
///      .await
///      .unwrap();
/// drop(context);
/// # });
/// ```
#[derive(Clone, Debug)]
pub struct ContextBuilder {
    dbfile: PathBuf,
    id: u32,
    events: Events,
    stock_strings: StockStrings,
    password: Option<String>,
}

impl ContextBuilder {
    /// Create the builder using the given database file.
    ///
    /// The *dbfile* should be in a dedicated directory and this directory must exist.  The
    /// [`Context`] will create other files and folders in the same directory as the
    /// database file used.
    pub fn new(dbfile: PathBuf) -> Self {
        ContextBuilder {
            dbfile,
            id: rand::random(),
            events: Events::new(),
            stock_strings: StockStrings::new(),
            password: None,
        }
    }

    /// Sets the context ID.
    ///
    /// This identifier is used e.g. in [`Event`]s to identify which [`Context`] an event
    /// belongs to.  The only real limit on it is that it should not conflict with any other
    /// [`Context`]s you currently have open.  So if you handle multiple [`Context`]s you
    /// may want to use this.
    ///
    /// Note that the [account manager](crate::accounts::Accounts) is designed to handle the
    /// common case for using multiple [`Context`] instances.
    pub fn with_id(mut self, id: u32) -> Self {
        self.id = id;
        self
    }

    /// Sets the event channel for this [`Context`].
    ///
    /// Mostly useful when using multiple [`Context`]s, this allows creating one [`Events`]
    /// channel and passing it to all [`Context`]s so all events are recieved on the same
    /// channel.
    ///
    /// Note that the [account manager](crate::accounts::Accounts) is designed to handle the
    /// common case for using multiple [`Context`] instances.
    pub fn with_events(mut self, events: Events) -> Self {
        self.events = events;
        self
    }

    /// Sets the [`StockStrings`] map to use for this [`Context`].
    ///
    /// This is useful in order to share the same translation strings in all [`Context`]s.
    /// The mapping may be empty when set, it will be populated by
    /// [`Context::set_stock-translation`] or [`Accounts::set_stock_translation`] calls.
    ///
    /// Note that the [account manager](crate::accounts::Accounts) is designed to handle the
    /// common case for using multiple [`Context`] instances.
    ///
    /// [`Accounts::set_stock_translation`]: crate::accounts::Accounts::set_stock_translation
    pub fn with_stock_strings(mut self, stock_strings: StockStrings) -> Self {
        self.stock_strings = stock_strings;
        self
    }

    /// Sets the password to unlock the database.
    ///
    /// If an encrypted database is used it must be opened with a password.  Setting a
    /// password on a new database will enable encryption.
    pub fn with_password(mut self, password: String) -> Self {
        self.password = Some(password);
        self
    }

    /// Opens the [`Context`].
    pub async fn open(self) -> Result<Context> {
        let context =
            Context::new_closed(&self.dbfile, self.id, self.events, self.stock_strings).await?;
        let password = self.password.unwrap_or_default();
        match context.open(password).await? {
            true => Ok(context),
            false => bail!("database could not be decrypted, incorrect or missing password"),
        }
    }
}

/// The context for a single DeltaChat account.
///
/// This contains all the state for a single DeltaChat account, including background tasks
/// running in Tokio to operate the account.  The [`Context`] can be cheaply cloned.
///
/// Each context, and thus each account, must be associated with an directory where all the
/// state is kept.  This state is also preserved between restarts.
///
/// To use multiple accounts it is best to look at the [accounts
/// manager][crate::accounts::Accounts] which handles storing multiple accounts in a single
/// directory structure and handles loading them all concurrently.
#[derive(Clone, Debug)]
pub struct Context {
    pub(crate) inner: Arc<InnerContext>,
}

impl Deref for Context {
    type Target = InnerContext;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

/// Actual context, expensive to clone.
#[derive(Debug)]
pub struct InnerContext {
    /// Blob directory path
    pub(crate) blobdir: PathBuf,
    pub(crate) sql: Sql,
    pub(crate) last_smeared_timestamp: RwLock<i64>,
    /// The global "ongoing" process state.
    ///
    /// This is a global mutex-like state for operations which should be modal in the
    /// clients.
    running_state: RwLock<RunningState>,
    /// Mutex to avoid generating the key for the user more than once.
    pub(crate) generating_key_mutex: Mutex<()>,
    /// Mutex to enforce only a single running oauth2 is running.
    pub(crate) oauth2_mutex: Mutex<()>,
    /// Mutex to prevent a race condition when a "your pw is wrong" warning is sent, resulting in multiple messeges being sent.
    pub(crate) wrong_pw_warning_mutex: Mutex<()>,
    pub(crate) translated_stockstrings: StockStrings,
    pub(crate) events: Events,

    pub(crate) scheduler: RwLock<Option<Scheduler>>,
    pub(crate) ratelimit: RwLock<Ratelimit>,

    /// Recently loaded quota information, if any.
    /// Set to `None` if quota was never tried to load.
    pub(crate) quota: RwLock<Option<QuotaInfo>>,

    /// Server ID response if ID capability is supported
    /// and the server returned non-NIL on the inbox connection.
    /// <https://datatracker.ietf.org/doc/html/rfc2971>
    pub(crate) server_id: RwLock<Option<HashMap<String, String>>>,

    pub(crate) last_full_folder_scan: Mutex<Option<Instant>>,

    /// ID for this `Context` in the current process.
    ///
    /// This allows for multiple `Context`s open in a single process where each context can
    /// be identified by this ID.
    pub(crate) id: u32,

    creation_time: SystemTime,

    /// The text of the last error logged and emitted as an event.
    /// If the ui wants to display an error after a failure,
    /// `last_error` should be used to avoid races with the event thread.
    pub(crate) last_error: std::sync::RwLock<String>,

    /// If debug logging is enabled, this contains all neccesary information
    pub(crate) debug_logging: RwLock<Option<DebugLogging>>,

    /// QR code for currently running [`BackupProvider`].
    ///
    /// This is only available if a backup export is currently running, it will also be
    /// holding the ongoing process while running.
    ///
    /// [`BackupProvider`]: crate::imex::BackupProvider
    pub(crate) export_provider: std::sync::Mutex<Option<Qr>>,
}

#[derive(Debug)]
pub(crate) struct DebugLogging {
    /// The message containing the logging xdc
    pub(crate) msg_id: MsgId,
    /// Handle to the background task responisble for sending
    pub(crate) loop_handle: task::JoinHandle<()>,
    /// Channel that log events should be send to
    /// A background loop will receive and handle them
    pub(crate) sender: Sender<DebugEventLogData>,
}

/// The state of ongoing process.
#[derive(Debug)]
enum RunningState {
    /// Ongoing process is allocated.
    Running { cancel_sender: Sender<()> },

    /// Cancel signal has been sent, waiting for ongoing process to be freed.
    ShallStop,

    /// There is no ongoing process, a new one can be allocated.
    Stopped,
}

impl Default for RunningState {
    fn default() -> Self {
        Self::Stopped
    }
}

/// Return some info about deltachat-core
///
/// This contains information mostly about the library itself, the
/// actual keys and their values which will be present are not
/// guaranteed.  Calling [Context::get_info] also includes information
/// about the context on top of the information here.
pub fn get_info() -> BTreeMap<&'static str, String> {
    let mut res = BTreeMap::new();
    res.insert("deltachat_core_version", format!("v{}", &*DC_VERSION_STR));
    res.insert("sqlite_version", rusqlite::version().to_string());
    res.insert("arch", (std::mem::size_of::<usize>() * 8).to_string());
    res.insert("num_cpus", num_cpus::get().to_string());
    res.insert("level", "awesome".into());
    res
}

impl Context {
    /// Creates new context and opens the database.
    pub async fn new(
        dbfile: &Path,
        id: u32,
        events: Events,
        stock_strings: StockStrings,
    ) -> Result<Context> {
        let context = Self::new_closed(dbfile, id, events, stock_strings).await?;

        // Open the database if is not encrypted.
        if context.check_passphrase("".to_string()).await? {
            context.sql.open(&context, "".to_string()).await?;
        }
        Ok(context)
    }

    /// Creates new context without opening the database.
    pub async fn new_closed(
        dbfile: &Path,
        id: u32,
        events: Events,
        stockstrings: StockStrings,
    ) -> Result<Context> {
        let mut blob_fname = OsString::new();
        blob_fname.push(dbfile.file_name().unwrap_or_default());
        blob_fname.push("-blobs");
        let blobdir = dbfile.with_file_name(blob_fname);
        if !blobdir.exists() {
            tokio::fs::create_dir_all(&blobdir).await?;
        }
        let context = Context::with_blobdir(dbfile.into(), blobdir, id, events, stockstrings)?;
        Ok(context)
    }

    /// Opens the database with the given passphrase.
    ///
    /// Returns true if passphrase is correct, false is passphrase is not correct. Fails on other
    /// errors.
    pub async fn open(&self, passphrase: String) -> Result<bool> {
        if self.sql.check_passphrase(passphrase.clone()).await? {
            self.sql.open(self, passphrase).await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Returns true if database is open.
    pub async fn is_open(&self) -> bool {
        self.sql.is_open().await
    }

    /// Tests the database passphrase.
    ///
    /// Returns true if passphrase is correct.
    ///
    /// Fails if database is already open.
    pub(crate) async fn check_passphrase(&self, passphrase: String) -> Result<bool> {
        self.sql.check_passphrase(passphrase).await
    }

    pub(crate) fn with_blobdir(
        dbfile: PathBuf,
        blobdir: PathBuf,
        id: u32,
        events: Events,
        stockstrings: StockStrings,
    ) -> Result<Context> {
        ensure!(
            blobdir.is_dir(),
            "Blobdir does not exist: {}",
            blobdir.display()
        );

        let inner = InnerContext {
            id,
            blobdir,
            running_state: RwLock::new(Default::default()),
            sql: Sql::new(dbfile),
            last_smeared_timestamp: RwLock::new(0),
            generating_key_mutex: Mutex::new(()),
            oauth2_mutex: Mutex::new(()),
            wrong_pw_warning_mutex: Mutex::new(()),
            translated_stockstrings: stockstrings,
            events,
            scheduler: RwLock::new(None),
            ratelimit: RwLock::new(Ratelimit::new(Duration::new(60, 0), 6.0)), // Allow to send 6 messages immediately, no more than once every 10 seconds.
            quota: RwLock::new(None),
            server_id: RwLock::new(None),
            creation_time: std::time::SystemTime::now(),
            last_full_folder_scan: Mutex::new(None),
            last_error: std::sync::RwLock::new("".to_string()),
            debug_logging: RwLock::new(None),
            export_provider: std::sync::Mutex::new(None),
        };

        let ctx = Context {
            inner: Arc::new(inner),
        };

        Ok(ctx)
    }

    /// Starts the IO scheduler.
    pub async fn start_io(&self) {
        if let Ok(false) = self.is_configured().await {
            warn!(self, "can not start io on a context that is not configured");
            return;
        }

        info!(self, "starting IO");
        let mut lock = self.inner.scheduler.write().await;
        if lock.is_none() {
            match Scheduler::start(self.clone()).await {
                Err(err) => error!(self, "Failed to start IO: {:#}", err),
                Ok(scheduler) => *lock = Some(scheduler),
            }
        }
    }

    /// Stops the IO scheduler.
    pub async fn stop_io(&self) {
        // Sending an event wakes up event pollers (get_next_event)
        // so the caller of stop_io() can arrange for proper termination.
        // For this, the caller needs to instruct the event poller
        // to terminate on receiving the next event and then call stop_io()
        // which will emit the below event(s)
        info!(self, "stopping IO");
        if let Some(debug_logging) = self.debug_logging.read().await.as_ref() {
            debug_logging.loop_handle.abort();
        }
        if let Some(scheduler) = self.inner.scheduler.write().await.take() {
            scheduler.stop(self).await;
        }
    }

    /// Restarts the IO scheduler if it was running before
    /// when it is not running this is an no-op
    pub async fn restart_io_if_running(&self) {
        info!(self, "restarting IO");
        let is_running = { self.inner.scheduler.read().await.is_some() };
        if is_running {
            self.stop_io().await;
            self.start_io().await;
        }
    }

    /// Returns a reference to the underlying SQL instance.
    ///
    /// Warning: this is only here for testing, not part of the public API.
    #[cfg(feature = "internals")]
    pub fn sql(&self) -> &Sql {
        &self.inner.sql
    }

    /// Returns database file path.
    pub fn get_dbfile(&self) -> &Path {
        self.sql.dbfile.as_path()
    }

    /// Returns blob directory path.
    pub fn get_blobdir(&self) -> &Path {
        self.blobdir.as_path()
    }

    /// Emits a single event.
    pub fn emit_event(&self, event: EventType) {
        if self
            .debug_logging
            .try_read()
            .ok()
            .map(|inner| inner.is_some())
            == Some(true)
        {
            self.send_log_event(event.clone()).ok();
        };
        self.events.emit(Event {
            id: self.id,
            typ: event,
        });
    }

    pub(crate) fn send_log_event(&self, event: EventType) -> anyhow::Result<()> {
        if let Ok(lock) = self.debug_logging.try_read() {
            if let Some(DebugLogging {
                msg_id: xdc_id,
                sender,
                ..
            }) = &*lock
            {
                let event_data = DebugEventLogData {
                    time: time(),
                    msg_id: *xdc_id,
                    event,
                };

                sender.try_send(event_data).ok();
            }
        }
        Ok(())
    }

    /// Emits a generic MsgsChanged event (without chat or message id)
    pub fn emit_msgs_changed_without_ids(&self) {
        self.emit_event(EventType::MsgsChanged {
            chat_id: ChatId::new(0),
            msg_id: MsgId::new(0),
        });
    }

    /// Emits a MsgsChanged event with specified chat and message ids
    pub fn emit_msgs_changed(&self, chat_id: ChatId, msg_id: MsgId) {
        self.emit_event(EventType::MsgsChanged { chat_id, msg_id });
    }

    /// Emits an IncomingMsg event with specified chat and message ids
    pub fn emit_incoming_msg(&self, chat_id: ChatId, msg_id: MsgId) {
        self.emit_event(EventType::IncomingMsg { chat_id, msg_id });
    }

    /// Returns a receiver for emitted events.
    ///
    /// Multiple emitters can be created, but note that in this case each emitted event will
    /// only be received by one of the emitters, not by all of them.
    pub fn get_event_emitter(&self) -> EventEmitter {
        self.events.get_emitter()
    }

    /// Get the ID of this context.
    pub fn get_id(&self) -> u32 {
        self.id
    }

    // Ongoing process allocation/free/check

    /// Tries to acquire the global UI "ongoing" mutex.
    ///
    /// This is for modal operations during which no other user actions are allowed.  Only
    /// one such operation is allowed at any given time.
    ///
    /// The return value is a cancel token, which will release the ongoing mutex when
    /// dropped.
    pub(crate) async fn alloc_ongoing(&self) -> Result<Receiver<()>> {
        let mut s = self.running_state.write().await;
        ensure!(
            matches!(*s, RunningState::Stopped),
            "There is already another ongoing process running."
        );

        let (sender, receiver) = channel::bounded(1);
        *s = RunningState::Running {
            cancel_sender: sender,
        };

        Ok(receiver)
    }

    pub(crate) async fn free_ongoing(&self) {
        let mut s = self.running_state.write().await;
        *s = RunningState::Stopped;
    }

    /// Signal an ongoing process to stop.
    pub async fn stop_ongoing(&self) {
        let mut s = self.running_state.write().await;
        match &*s {
            RunningState::Running { cancel_sender } => {
                if let Err(err) = cancel_sender.send(()).await {
                    warn!(self, "could not cancel ongoing: {:#}", err);
                }
                info!(self, "Signaling the ongoing process to stop ASAP.",);
                *s = RunningState::ShallStop;
            }
            RunningState::ShallStop | RunningState::Stopped => {
                info!(self, "No ongoing process to stop.",);
            }
        }
    }

    #[allow(unused)]
    pub(crate) async fn shall_stop_ongoing(&self) -> bool {
        match &*self.running_state.read().await {
            RunningState::Running { .. } => false,
            RunningState::ShallStop | RunningState::Stopped => true,
        }
    }

    /// Returns the QR-code of the currently running [`BackupProvider`].
    ///
    /// [`BackupProvider`]: crate::imex::BackupProvider
    pub fn backup_export_qr(&self) -> Option<Qr> {
        self.export_provider
            .lock()
            .expect("poisoned lock")
            .as_ref()
            .cloned()
    }

    /*******************************************************************************
     * UI chat/message related API
     ******************************************************************************/

    /// Returns information about the context as key-value pairs.
    pub async fn get_info(&self) -> Result<BTreeMap<&'static str, String>> {
        let unset = "0";
        let l = LoginParam::load_candidate_params_unchecked(self).await?;
        let l2 = LoginParam::load_configured_params(self).await?;
        let secondary_addrs = self.get_secondary_self_addrs().await?.join(", ");
        let displayname = self.get_config(Config::Displayname).await?;
        let chats = get_chat_cnt(self).await?;
        let unblocked_msgs = message::get_unblocked_msg_cnt(self).await;
        let request_msgs = message::get_request_msg_cnt(self).await;
        let contacts = Contact::get_real_cnt(self).await?;
        let is_configured = self.get_config_int(Config::Configured).await?;
        let socks5_enabled = self.get_config_int(Config::Socks5Enabled).await?;
        let dbversion = self
            .sql
            .get_raw_config_int("dbversion")
            .await?
            .unwrap_or_default();
        let journal_mode = self
            .sql
            .query_get_value("PRAGMA journal_mode;", paramsv![])
            .await?
            .unwrap_or_else(|| "unknown".to_string());
        let e2ee_enabled = self.get_config_int(Config::E2eeEnabled).await?;
        let mdns_enabled = self.get_config_int(Config::MdnsEnabled).await?;
        let bcc_self = self.get_config_int(Config::BccSelf).await?;
        let send_sync_msgs = self.get_config_int(Config::SendSyncMsgs).await?;

        let prv_key_cnt = self
            .sql
            .count("SELECT COUNT(*) FROM keypairs;", paramsv![])
            .await?;

        let pub_key_cnt = self
            .sql
            .count("SELECT COUNT(*) FROM acpeerstates;", paramsv![])
            .await?;
        let fingerprint_str = match SignedPublicKey::load_self(self).await {
            Ok(key) => key.fingerprint().hex(),
            Err(err) => format!("<key failure: {err}>"),
        };

        let sentbox_watch = self.get_config_int(Config::SentboxWatch).await?;
        let mvbox_move = self.get_config_int(Config::MvboxMove).await?;
        let only_fetch_mvbox = self.get_config_int(Config::OnlyFetchMvbox).await?;
        let folders_configured = self
            .sql
            .get_raw_config_int("folders_configured")
            .await?
            .unwrap_or_default();

        let configured_inbox_folder = self
            .get_config(Config::ConfiguredInboxFolder)
            .await?
            .unwrap_or_else(|| "<unset>".to_string());
        let configured_sentbox_folder = self
            .get_config(Config::ConfiguredSentboxFolder)
            .await?
            .unwrap_or_else(|| "<unset>".to_string());
        let configured_mvbox_folder = self
            .get_config(Config::ConfiguredMvboxFolder)
            .await?
            .unwrap_or_else(|| "<unset>".to_string());
        let configured_trash_folder = self
            .get_config(Config::ConfiguredTrashFolder)
            .await?
            .unwrap_or_else(|| "<unset>".to_string());

        let mut res = get_info();

        // insert values
        res.insert("bot", self.get_config_int(Config::Bot).await?.to_string());
        res.insert("number_of_chats", chats.to_string());
        res.insert("number_of_chat_messages", unblocked_msgs.to_string());
        res.insert("messages_in_contact_requests", request_msgs.to_string());
        res.insert("number_of_contacts", contacts.to_string());
        res.insert("database_dir", self.get_dbfile().display().to_string());
        res.insert("database_version", dbversion.to_string());
        res.insert(
            "database_encrypted",
            self.sql
                .is_encrypted()
                .await
                .map_or_else(|| "closed".to_string(), |b| b.to_string()),
        );
        res.insert("journal_mode", journal_mode);
        res.insert("blobdir", self.get_blobdir().display().to_string());
        res.insert("display_name", displayname.unwrap_or_else(|| unset.into()));
        res.insert(
            "selfavatar",
            self.get_config(Config::Selfavatar)
                .await?
                .unwrap_or_else(|| "<unset>".to_string()),
        );
        res.insert("is_configured", is_configured.to_string());
        res.insert("socks5_enabled", socks5_enabled.to_string());
        res.insert("entered_account_settings", l.to_string());
        res.insert("used_account_settings", l2.to_string());

        if let Some(server_id) = &*self.server_id.read().await {
            res.insert("imap_server_id", format!("{server_id:?}"));
        }

        res.insert("secondary_addrs", secondary_addrs);
        res.insert(
            "fetch_existing_msgs",
            self.get_config_int(Config::FetchExistingMsgs)
                .await?
                .to_string(),
        );
        res.insert(
            "fetched_existing_msgs",
            self.get_config_bool(Config::FetchedExistingMsgs)
                .await?
                .to_string(),
        );
        res.insert(
            "show_emails",
            self.get_config_int(Config::ShowEmails).await?.to_string(),
        );
        res.insert(
            "download_limit",
            self.get_config_int(Config::DownloadLimit)
                .await?
                .to_string(),
        );
        res.insert("sentbox_watch", sentbox_watch.to_string());
        res.insert("mvbox_move", mvbox_move.to_string());
        res.insert("only_fetch_mvbox", only_fetch_mvbox.to_string());
        res.insert("folders_configured", folders_configured.to_string());
        res.insert("configured_inbox_folder", configured_inbox_folder);
        res.insert("configured_sentbox_folder", configured_sentbox_folder);
        res.insert("configured_mvbox_folder", configured_mvbox_folder);
        res.insert("configured_trash_folder", configured_trash_folder);
        res.insert("mdns_enabled", mdns_enabled.to_string());
        res.insert("e2ee_enabled", e2ee_enabled.to_string());
        res.insert(
            "key_gen_type",
            self.get_config_int(Config::KeyGenType).await?.to_string(),
        );
        res.insert("bcc_self", bcc_self.to_string());
        res.insert("send_sync_msgs", send_sync_msgs.to_string());
        res.insert("private_key_count", prv_key_cnt.to_string());
        res.insert("public_key_count", pub_key_cnt.to_string());
        res.insert("fingerprint", fingerprint_str);
        res.insert(
            "webrtc_instance",
            self.get_config(Config::WebrtcInstance)
                .await?
                .unwrap_or_else(|| "<unset>".to_string()),
        );
        res.insert(
            "media_quality",
            self.get_config_int(Config::MediaQuality).await?.to_string(),
        );
        res.insert(
            "delete_device_after",
            self.get_config_int(Config::DeleteDeviceAfter)
                .await?
                .to_string(),
        );
        res.insert(
            "delete_server_after",
            self.get_config_int(Config::DeleteServerAfter)
                .await?
                .to_string(),
        );
        res.insert(
            "delete_to_trash",
            self.get_config(Config::DeleteToTrash)
                .await?
                .unwrap_or_else(|| "<unset>".to_string()),
        );
        res.insert(
            "last_housekeeping",
            self.get_config_int(Config::LastHousekeeping)
                .await?
                .to_string(),
        );
        res.insert(
            "scan_all_folders_debounce_secs",
            self.get_config_int(Config::ScanAllFoldersDebounceSecs)
                .await?
                .to_string(),
        );
        res.insert(
            "quota_exceeding",
            self.get_config_int(Config::QuotaExceeding)
                .await?
                .to_string(),
        );
        res.insert(
            "authserv_id_candidates",
            self.get_config(Config::AuthservIdCandidates)
                .await?
                .unwrap_or_default(),
        );

        res.insert(
            "debug_logging",
            self.get_config_int(Config::DebugLogging).await?.to_string(),
        );

        let elapsed = self.creation_time.elapsed();
        res.insert("uptime", duration_to_str(elapsed.unwrap_or_default()));

        Ok(res)
    }

    /// Get a list of fresh, unmuted messages in unblocked chats.
    ///
    /// The list starts with the most recent message
    /// and is typically used to show notifications.
    /// Moreover, the number of returned messages
    /// can be used for a badge counter on the app icon.
    pub async fn get_fresh_msgs(&self) -> Result<Vec<MsgId>> {
        let list = self
            .sql
            .query_map(
                concat!(
                    "SELECT m.id",
                    " FROM msgs m",
                    " LEFT JOIN contacts ct",
                    "        ON m.from_id=ct.id",
                    " LEFT JOIN chats c",
                    "        ON m.chat_id=c.id",
                    " WHERE m.state=?",
                    "   AND m.hidden=0",
                    "   AND m.chat_id>9",
                    "   AND ct.blocked=0",
                    "   AND c.blocked=0",
                    "   AND NOT(c.muted_until=-1 OR c.muted_until>?)",
                    " ORDER BY m.timestamp DESC,m.id DESC;"
                ),
                paramsv![MessageState::InFresh, time()],
                |row| row.get::<_, MsgId>(0),
                |rows| {
                    let mut list = Vec::new();
                    for row in rows {
                        list.push(row?);
                    }
                    Ok(list)
                },
            )
            .await?;
        Ok(list)
    }

    /// Searches for messages containing the query string.
    ///
    /// If `chat_id` is provided this searches only for messages in this chat, if `chat_id`
    /// is `None` this searches messages from all chats.
    pub async fn search_msgs(&self, chat_id: Option<ChatId>, query: &str) -> Result<Vec<MsgId>> {
        let real_query = query.trim();
        if real_query.is_empty() {
            return Ok(Vec::new());
        }
        let str_like_in_text = format!("%{real_query}%");

        let do_query = |query, params| {
            self.sql.query_map(
                query,
                params,
                |row| row.get::<_, MsgId>("id"),
                |rows| {
                    let mut ret = Vec::new();
                    for id in rows {
                        ret.push(id?);
                    }
                    Ok(ret)
                },
            )
        };

        let list = if let Some(chat_id) = chat_id {
            do_query(
                "SELECT m.id AS id
                 FROM msgs m
                 LEFT JOIN contacts ct
                        ON m.from_id=ct.id
                 WHERE m.chat_id=?
                   AND m.hidden=0
                   AND ct.blocked=0
                   AND txt LIKE ?
                 ORDER BY m.timestamp,m.id;",
                paramsv![chat_id, str_like_in_text],
            )
            .await?
        } else {
            // For performance reasons results are sorted only by `id`, that is in the order of
            // message reception.
            //
            // Unlike chat view, sorting by `timestamp` is not necessary but slows down the query by
            // ~25% according to benchmarks.
            //
            // To speed up incremental search, where queries for few characters usually return lots
            // of unwanted results that are discarded moments later, we added `LIMIT 1000`.
            // According to some tests, this limit speeds up eg. 2 character searches by factor 10.
            // The limit is documented and UI may add a hint when getting 1000 results.
            do_query(
                "SELECT m.id AS id
                 FROM msgs m
                 LEFT JOIN contacts ct
                        ON m.from_id=ct.id
                 LEFT JOIN chats c
                        ON m.chat_id=c.id
                 WHERE m.chat_id>9
                   AND m.hidden=0
                   AND c.blocked!=1
                   AND ct.blocked=0
                   AND m.txt LIKE ?
                 ORDER BY m.id DESC LIMIT 1000",
                paramsv![str_like_in_text],
            )
            .await?
        };

        Ok(list)
    }

    /// Returns true if given folder name is the name of the inbox.
    pub async fn is_inbox(&self, folder_name: &str) -> Result<bool> {
        let inbox = self.get_config(Config::ConfiguredInboxFolder).await?;
        Ok(inbox.as_deref() == Some(folder_name))
    }

    /// Returns true if given folder name is the name of the "sent" folder.
    pub async fn is_sentbox(&self, folder_name: &str) -> Result<bool> {
        let sentbox = self.get_config(Config::ConfiguredSentboxFolder).await?;
        Ok(sentbox.as_deref() == Some(folder_name))
    }

    /// Returns true if given folder name is the name of the "Delta Chat" folder.
    pub async fn is_mvbox(&self, folder_name: &str) -> Result<bool> {
        let mvbox = self.get_config(Config::ConfiguredMvboxFolder).await?;
        Ok(mvbox.as_deref() == Some(folder_name))
    }

    /// Returns true if given folder name is the name of the trash folder.
    pub async fn is_trash(&self, folder_name: &str) -> Result<bool> {
        let trash = self.get_config(Config::ConfiguredTrashFolder).await?;
        Ok(trash.as_deref() == Some(folder_name))
    }

    pub(crate) async fn should_delete_to_trash(&self) -> Result<bool> {
        if let Some(v) = self.get_config_bool_opt(Config::DeleteToTrash).await? {
            return Ok(v);
        }
        if let Some(provider) = self.get_configured_provider().await? {
            return Ok(provider.opt.delete_to_trash);
        }
        Ok(false)
    }

    /// Returns `target` for deleted messages as per `imap` table. Empty string means "delete w/o
    /// moving to trash".
    pub(crate) async fn get_delete_msgs_target(&self) -> Result<String> {
        if !self.should_delete_to_trash().await? {
            return Ok("".into());
        }
        self.get_config(Config::ConfiguredTrashFolder)
            .await?
            .context("No configured trash folder")
    }

    pub(crate) fn derive_blobdir(dbfile: &Path) -> PathBuf {
        let mut blob_fname = OsString::new();
        blob_fname.push(dbfile.file_name().unwrap_or_default());
        blob_fname.push("-blobs");
        dbfile.with_file_name(blob_fname)
    }

    pub(crate) fn derive_walfile(dbfile: &Path) -> PathBuf {
        let mut wal_fname = OsString::new();
        wal_fname.push(dbfile.file_name().unwrap_or_default());
        wal_fname.push("-wal");
        dbfile.with_file_name(wal_fname)
    }
}

/// Returns core version as a string.
pub fn get_version_str() -> &'static str {
    &DC_VERSION_STR
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use anyhow::Context as _;
    use strum::IntoEnumIterator;
    use tempfile::tempdir;

    use super::*;
    use crate::chat::{
        get_chat_contacts, get_chat_msgs, send_msg, set_muted, Chat, ChatId, MuteDuration,
    };
    use crate::chatlist::Chatlist;
    use crate::constants::Chattype;
    use crate::contact::ContactId;
    use crate::message::{Message, Viewtype};
    use crate::receive_imf::receive_imf;
    use crate::test_utils::TestContext;
    use crate::tools::create_outgoing_rfc724_mid;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_wrong_db() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let dbfile = tmp.path().join("db.sqlite");
        tokio::fs::write(&dbfile, b"123").await?;
        let res = Context::new(&dbfile, 1, Events::new(), StockStrings::new()).await?;

        // Broken database is indistinguishable from encrypted one.
        assert_eq!(res.is_open().await, false);
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_get_fresh_msgs() {
        let t = TestContext::new().await;
        let fresh = t.get_fresh_msgs().await.unwrap();
        assert!(fresh.is_empty())
    }

    async fn receive_msg(t: &TestContext, chat: &Chat) {
        let members = get_chat_contacts(t, chat.id).await.unwrap();
        let contact = Contact::load_from_db(t, *members.first().unwrap())
            .await
            .unwrap();
        let msg = format!(
            "From: {}\n\
             To: alice@example.org\n\
             Message-ID: <{}>\n\
             Chat-Version: 1.0\n\
             Date: Sun, 22 Mar 2020 22:37:57 +0000\n\
             \n\
             hello\n",
            contact.get_addr(),
            create_outgoing_rfc724_mid(None, contact.get_addr())
        );
        println!("{msg}");
        receive_imf(t, msg.as_bytes(), false).await.unwrap();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_get_fresh_msgs_and_muted_chats() {
        // receive various mails in 3 chats
        let t = TestContext::new_alice().await;
        let bob = t.create_chat_with_contact("", "bob@g.it").await;
        let claire = t.create_chat_with_contact("", "claire@g.it").await;
        let dave = t.create_chat_with_contact("", "dave@g.it").await;
        assert_eq!(t.get_fresh_msgs().await.unwrap().len(), 0);

        receive_msg(&t, &bob).await;
        assert_eq!(get_chat_msgs(&t, bob.id).await.unwrap().len(), 1);
        assert_eq!(bob.id.get_fresh_msg_cnt(&t).await.unwrap(), 1);
        assert_eq!(t.get_fresh_msgs().await.unwrap().len(), 1);

        receive_msg(&t, &claire).await;
        receive_msg(&t, &claire).await;
        assert_eq!(get_chat_msgs(&t, claire.id).await.unwrap().len(), 2);
        assert_eq!(claire.id.get_fresh_msg_cnt(&t).await.unwrap(), 2);
        assert_eq!(t.get_fresh_msgs().await.unwrap().len(), 3);

        receive_msg(&t, &dave).await;
        receive_msg(&t, &dave).await;
        receive_msg(&t, &dave).await;
        assert_eq!(get_chat_msgs(&t, dave.id).await.unwrap().len(), 3);
        assert_eq!(dave.id.get_fresh_msg_cnt(&t).await.unwrap(), 3);
        assert_eq!(t.get_fresh_msgs().await.unwrap().len(), 6);

        // mute one of the chats
        set_muted(&t, claire.id, MuteDuration::Forever)
            .await
            .unwrap();
        assert_eq!(claire.id.get_fresh_msg_cnt(&t).await.unwrap(), 2);
        assert_eq!(t.get_fresh_msgs().await.unwrap().len(), 4); // muted claires messages are no longer counted

        // receive more messages
        receive_msg(&t, &bob).await;
        receive_msg(&t, &claire).await;
        receive_msg(&t, &dave).await;
        assert_eq!(get_chat_msgs(&t, claire.id).await.unwrap().len(), 3);
        assert_eq!(claire.id.get_fresh_msg_cnt(&t).await.unwrap(), 3);
        assert_eq!(t.get_fresh_msgs().await.unwrap().len(), 6); // muted claire is not counted

        // unmute claire again
        set_muted(&t, claire.id, MuteDuration::NotMuted)
            .await
            .unwrap();
        assert_eq!(claire.id.get_fresh_msg_cnt(&t).await.unwrap(), 3);
        assert_eq!(t.get_fresh_msgs().await.unwrap().len(), 9); // claire is counted again
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_get_fresh_msgs_and_muted_until() {
        let t = TestContext::new_alice().await;
        let bob = t.create_chat_with_contact("", "bob@g.it").await;
        receive_msg(&t, &bob).await;
        assert_eq!(get_chat_msgs(&t, bob.id).await.unwrap().len(), 1);

        // chat is unmuted by default, here and in the following assert(),
        // we check mainly that the SQL-statements in is_muted() and get_fresh_msgs()
        // have the same view to the database.
        assert!(!bob.is_muted());
        assert_eq!(t.get_fresh_msgs().await.unwrap().len(), 1);

        // test get_fresh_msgs() with mute_until in the future
        set_muted(
            &t,
            bob.id,
            MuteDuration::Until(SystemTime::now() + Duration::from_secs(3600)),
        )
        .await
        .unwrap();
        let bob = Chat::load_from_db(&t, bob.id).await.unwrap();
        assert!(bob.is_muted());
        assert_eq!(t.get_fresh_msgs().await.unwrap().len(), 0);

        // to test get_fresh_msgs() with mute_until in the past,
        // we need to modify the database directly
        t.sql
            .execute(
                "UPDATE chats SET muted_until=? WHERE id=?;",
                paramsv![time() - 3600, bob.id],
            )
            .await
            .unwrap();
        let bob = Chat::load_from_db(&t, bob.id).await.unwrap();
        assert!(!bob.is_muted());
        assert_eq!(t.get_fresh_msgs().await.unwrap().len(), 1);

        // test get_fresh_msgs() with "forever" mute_until
        set_muted(&t, bob.id, MuteDuration::Forever).await.unwrap();
        let bob = Chat::load_from_db(&t, bob.id).await.unwrap();
        assert!(bob.is_muted());
        assert_eq!(t.get_fresh_msgs().await.unwrap().len(), 0);

        // to test get_fresh_msgs() with invalid mute_until (everything < -1),
        // that results in "muted forever" by definition.
        t.sql
            .execute(
                "UPDATE chats SET muted_until=-2 WHERE id=?;",
                paramsv![bob.id],
            )
            .await
            .unwrap();
        let bob = Chat::load_from_db(&t, bob.id).await.unwrap();
        assert!(!bob.is_muted());
        assert_eq!(t.get_fresh_msgs().await.unwrap().len(), 1);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_blobdir_exists() {
        let tmp = tempfile::tempdir().unwrap();
        let dbfile = tmp.path().join("db.sqlite");
        Context::new(&dbfile, 1, Events::new(), StockStrings::new())
            .await
            .unwrap();
        let blobdir = tmp.path().join("db.sqlite-blobs");
        assert!(blobdir.is_dir());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_wrong_blogdir() {
        let tmp = tempfile::tempdir().unwrap();
        let dbfile = tmp.path().join("db.sqlite");
        let blobdir = tmp.path().join("db.sqlite-blobs");
        tokio::fs::write(&blobdir, b"123").await.unwrap();
        let res = Context::new(&dbfile, 1, Events::new(), StockStrings::new()).await;
        assert!(res.is_err());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_sqlite_parent_not_exists() {
        let tmp = tempfile::tempdir().unwrap();
        let subdir = tmp.path().join("subdir");
        let dbfile = subdir.join("db.sqlite");
        let dbfile2 = dbfile.clone();
        Context::new(&dbfile, 1, Events::new(), StockStrings::new())
            .await
            .unwrap();
        assert!(subdir.is_dir());
        assert!(dbfile2.is_file());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_with_empty_blobdir() {
        let tmp = tempfile::tempdir().unwrap();
        let dbfile = tmp.path().join("db.sqlite");
        let blobdir = PathBuf::new();
        let res = Context::with_blobdir(dbfile, blobdir, 1, Events::new(), StockStrings::new());
        assert!(res.is_err());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_with_blobdir_not_exists() {
        let tmp = tempfile::tempdir().unwrap();
        let dbfile = tmp.path().join("db.sqlite");
        let blobdir = tmp.path().join("blobs");
        let res = Context::with_blobdir(dbfile, blobdir, 1, Events::new(), StockStrings::new());
        assert!(res.is_err());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn no_crashes_on_context_deref() {
        let t = TestContext::new().await;
        std::mem::drop(t);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_get_info() {
        let t = TestContext::new().await;

        let info = t.get_info().await.unwrap();
        assert!(info.get("database_dir").is_some());
    }

    #[test]
    fn test_get_info_no_context() {
        let info = get_info();
        assert!(info.get("deltachat_core_version").is_some());
        assert!(info.get("database_dir").is_none());
        assert_eq!(info.get("level").unwrap(), "awesome");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_get_info_completeness() {
        // For easier debugging,
        // get_info() shall return all important information configurable by the Config-values.
        //
        // There are exceptions for Config-values considered to be unimportant,
        // too sensitive or summarized in another item.
        let skip_from_get_info = vec![
            "addr",
            "displayname",
            "imap_certificate_checks",
            "mail_server",
            "mail_user",
            "mail_pw",
            "mail_port",
            "mail_security",
            "notify_about_wrong_pw",
            "save_mime_headers",
            "selfstatus",
            "send_server",
            "send_user",
            "send_pw",
            "send_port",
            "send_security",
            "server_flags",
            "smtp_certificate_checks",
            "socks5_host",
            "socks5_port",
            "socks5_user",
            "socks5_password",
        ];
        let t = TestContext::new().await;
        let info = t.get_info().await.unwrap();
        for key in Config::iter() {
            let key: String = key.to_string();
            if !skip_from_get_info.contains(&&*key)
                && !key.starts_with("configured")
                && !key.starts_with("sys.")
            {
                assert!(
                    info.contains_key(&*key),
                    "'{key}' missing in get_info() output"
                );
            }
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_search_msgs() -> Result<()> {
        let alice = TestContext::new_alice().await;
        let self_talk = ChatId::create_for_contact(&alice, ContactId::SELF).await?;
        let chat = alice
            .create_chat_with_contact("Bob", "bob@example.org")
            .await;

        // Global search finds nothing.
        let res = alice.search_msgs(None, "foo").await?;
        assert!(res.is_empty());

        // Search in chat with Bob finds nothing.
        let res = alice.search_msgs(Some(chat.id), "foo").await?;
        assert!(res.is_empty());

        // Add messages to chat with Bob.
        let mut msg1 = Message::new(Viewtype::Text);
        msg1.set_text(Some("foobar".to_string()));
        send_msg(&alice, chat.id, &mut msg1).await?;

        let mut msg2 = Message::new(Viewtype::Text);
        msg2.set_text(Some("barbaz".to_string()));
        send_msg(&alice, chat.id, &mut msg2).await?;

        // Global search with a part of text finds the message.
        let res = alice.search_msgs(None, "ob").await?;
        assert_eq!(res.len(), 1);

        // Global search for "bar" matches both "foobar" and "barbaz".
        let res = alice.search_msgs(None, "bar").await?;
        assert_eq!(res.len(), 2);

        // Message added later is returned first.
        assert_eq!(res.get(0), Some(&msg2.id));
        assert_eq!(res.get(1), Some(&msg1.id));

        // Global search with longer text does not find any message.
        let res = alice.search_msgs(None, "foobarbaz").await?;
        assert!(res.is_empty());

        // Search for random string finds nothing.
        let res = alice.search_msgs(None, "abc").await?;
        assert!(res.is_empty());

        // Search in chat with Bob finds the message.
        let res = alice.search_msgs(Some(chat.id), "foo").await?;
        assert_eq!(res.len(), 1);

        // Search in Saved Messages does not find the message.
        let res = alice.search_msgs(Some(self_talk), "foo").await?;
        assert!(res.is_empty());

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_search_unaccepted_requests() -> Result<()> {
        let t = TestContext::new_alice().await;
        receive_imf(
            &t,
            b"From: BobBar <bob@example.org>\n\
                 To: alice@example.org\n\
                 Subject: foo\n\
                 Message-ID: <msg1234@example.org>\n\
                 Chat-Version: 1.0\n\
                 Date: Tue, 25 Oct 2022 13:37:00 +0000\n\
                 \n\
                 hello bob, foobar test!\n",
            false,
        )
        .await?;
        let chat_id = t.get_last_msg().await.get_chat_id();
        let chat = Chat::load_from_db(&t, chat_id).await?;
        assert_eq!(chat.get_type(), Chattype::Single);
        assert!(chat.is_contact_request());

        assert_eq!(Chatlist::try_load(&t, 0, None, None).await?.len(), 1);
        assert_eq!(
            Chatlist::try_load(&t, 0, Some("BobBar"), None).await?.len(),
            1
        );
        assert_eq!(t.search_msgs(None, "foobar").await?.len(), 1);
        assert_eq!(t.search_msgs(Some(chat_id), "foobar").await?.len(), 1);

        chat_id.block(&t).await?;

        assert_eq!(Chatlist::try_load(&t, 0, None, None).await?.len(), 0);
        assert_eq!(
            Chatlist::try_load(&t, 0, Some("BobBar"), None).await?.len(),
            0
        );
        assert_eq!(t.search_msgs(None, "foobar").await?.len(), 0);
        assert_eq!(t.search_msgs(Some(chat_id), "foobar").await?.len(), 0);

        let contact_ids = get_chat_contacts(&t, chat_id).await?;
        Contact::unblock(&t, *contact_ids.first().unwrap()).await?;

        assert_eq!(Chatlist::try_load(&t, 0, None, None).await?.len(), 1);
        assert_eq!(
            Chatlist::try_load(&t, 0, Some("BobBar"), None).await?.len(),
            1
        );
        assert_eq!(t.search_msgs(None, "foobar").await?.len(), 1);
        assert_eq!(t.search_msgs(Some(chat_id), "foobar").await?.len(), 1);

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_limit_search_msgs() -> Result<()> {
        let alice = TestContext::new_alice().await;
        let chat = alice
            .create_chat_with_contact("Bob", "bob@example.org")
            .await;

        // Add 999 messages
        let mut msg = Message::new(Viewtype::Text);
        msg.set_text(Some("foobar".to_string()));
        for _ in 0..999 {
            send_msg(&alice, chat.id, &mut msg).await?;
        }
        let res = alice.search_msgs(None, "foo").await?;
        assert_eq!(res.len(), 999);

        // Add one more message, no limit yet
        send_msg(&alice, chat.id, &mut msg).await?;
        let res = alice.search_msgs(None, "foo").await?;
        assert_eq!(res.len(), 1000);

        // Add one more message, that one is truncated then
        send_msg(&alice, chat.id, &mut msg).await?;
        let res = alice.search_msgs(None, "foo").await?;
        assert_eq!(res.len(), 1000);

        // In-chat should not be not limited
        let res = alice.search_msgs(Some(chat.id), "foo").await?;
        assert_eq!(res.len(), 1001);

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_check_passphrase() -> Result<()> {
        let dir = tempdir()?;
        let dbfile = dir.path().join("db.sqlite");

        let id = 1;
        let context = Context::new_closed(&dbfile, id, Events::new(), StockStrings::new())
            .await
            .context("failed to create context")?;
        assert_eq!(context.open("foo".to_string()).await?, true);
        assert_eq!(context.is_open().await, true);
        drop(context);

        let id = 2;
        let context = Context::new(&dbfile, id, Events::new(), StockStrings::new())
            .await
            .context("failed to create context")?;
        assert_eq!(context.is_open().await, false);
        assert_eq!(context.check_passphrase("bar".to_string()).await?, false);
        assert_eq!(context.open("false".to_string()).await?, false);
        assert_eq!(context.open("foo".to_string()).await?, true);

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_ongoing() -> Result<()> {
        let context = TestContext::new().await;

        // No ongoing process allocated.
        assert!(context.shall_stop_ongoing().await);

        let receiver = context.alloc_ongoing().await?;

        // Cannot allocate another ongoing process while the first one is running.
        assert!(context.alloc_ongoing().await.is_err());

        // Stop signal is not sent yet.
        assert!(receiver.try_recv().is_err());

        assert!(!context.shall_stop_ongoing().await);

        // Send the stop signal.
        context.stop_ongoing().await;

        // Receive stop signal.
        receiver.recv().await?;

        assert!(context.shall_stop_ongoing().await);

        // Ongoing process is still running even though stop signal was received,
        // so another one cannot be allocated.
        assert!(context.alloc_ongoing().await.is_err());

        context.free_ongoing().await;

        // No ongoing process allocated, should have been stopped already.
        assert!(context.shall_stop_ongoing().await);

        // Another ongoing process can be allocated now.
        let _receiver = context.alloc_ongoing().await?;

        Ok(())
    }
}
