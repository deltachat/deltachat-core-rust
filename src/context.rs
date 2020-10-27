//! Context module

use std::collections::{BTreeMap, HashMap};
use std::ffi::OsString;
use std::ops::Deref;

use async_std::path::{Path, PathBuf};
use async_std::sync::{channel, Arc, Mutex, Receiver, RwLock, Sender};
use async_std::task;

use crate::chat::*;
use crate::config::Config;
use crate::constants::*;
use crate::contact::*;
use crate::dc_tools::duration_to_str;
use crate::error::*;
use crate::events::{Event, EventEmitter, EventType, Events};
use crate::key::{DcKey, SignedPublicKey};
use crate::login_param::LoginParam;
use crate::message::{self, MsgId};
use crate::scheduler::Scheduler;
use crate::securejoin::Bob;
use crate::sql::Sql;
use std::time::SystemTime;

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

#[derive(Debug)]
pub struct InnerContext {
    /// Database file path
    pub(crate) dbfile: PathBuf,
    /// Blob directory path
    pub(crate) blobdir: PathBuf,
    pub(crate) sql: Sql,
    pub(crate) os_name: Option<String>,
    pub(crate) bob: RwLock<Bob>,
    pub(crate) last_smeared_timestamp: RwLock<i64>,
    pub(crate) running_state: RwLock<RunningState>,
    /// Mutex to avoid generating the key for the user more than once.
    pub(crate) generating_key_mutex: Mutex<()>,
    /// Mutex to enforce only a single running oauth2 is running.
    pub(crate) oauth2_mutex: Mutex<()>,
    /// Mutex to prevent a race condition when a "your pw is wrong" warning is sent, resulting in multiple messeges being sent.
    pub(crate) wrong_pw_warning_mutex: Mutex<()>,
    pub(crate) translated_stockstrings: RwLock<HashMap<usize, String>>,
    pub(crate) events: Events,

    pub(crate) scheduler: RwLock<Scheduler>,
    pub(crate) ephemeral_task: RwLock<Option<task::JoinHandle<()>>>,

    /// Id for this context on the current device.
    pub(crate) id: u32,

    creation_time: SystemTime,
}

#[derive(Debug)]
pub struct RunningState {
    pub ongoing_running: bool,
    shall_stop_ongoing: bool,
    cancel_sender: Option<Sender<()>>,
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
    res.insert("level", "awesome".into());
    res
}

impl Context {
    /// Creates new context.
    pub async fn new(os_name: String, dbfile: PathBuf, id: u32) -> Result<Context> {
        // pretty_env_logger::try_init_timed().ok();

        let mut blob_fname = OsString::new();
        blob_fname.push(dbfile.file_name().unwrap_or_default());
        blob_fname.push("-blobs");
        let blobdir = dbfile.with_file_name(blob_fname);
        if !blobdir.exists().await {
            async_std::fs::create_dir_all(&blobdir).await?;
        }
        Context::with_blobdir(os_name, dbfile, blobdir, id).await
    }

    pub(crate) async fn with_blobdir(
        os_name: String,
        dbfile: PathBuf,
        blobdir: PathBuf,
        id: u32,
    ) -> Result<Context> {
        ensure!(
            blobdir.is_dir().await,
            "Blobdir does not exist: {}",
            blobdir.display()
        );

        let inner = InnerContext {
            id,
            blobdir,
            dbfile,
            os_name: Some(os_name),
            running_state: RwLock::new(Default::default()),
            sql: Sql::new(),
            bob: RwLock::new(Default::default()),
            last_smeared_timestamp: RwLock::new(0),
            generating_key_mutex: Mutex::new(()),
            oauth2_mutex: Mutex::new(()),
            wrong_pw_warning_mutex: Mutex::new(()),
            translated_stockstrings: RwLock::new(HashMap::new()),
            events: Events::default(),
            scheduler: RwLock::new(Scheduler::Stopped),
            ephemeral_task: RwLock::new(None),
            creation_time: std::time::SystemTime::now(),
        };

        let ctx = Context {
            inner: Arc::new(inner),
        };
        ctx.sql.open(&ctx, &ctx.dbfile, false).await?;

        Ok(ctx)
    }

    /// Starts the IO scheduler.
    pub async fn start_io(&self) {
        info!(self, "starting IO");
        if self.is_io_running().await {
            info!(self, "IO is already running");
            return;
        }

        {
            let l = &mut *self.inner.scheduler.write().await;
            l.start(self.clone()).await;
        }
    }

    /// Returns if the IO scheduler is running.
    pub async fn is_io_running(&self) -> bool {
        self.inner.is_io_running().await
    }

    /// Stops the IO scheduler.
    pub async fn stop_io(&self) {
        info!(self, "stopping IO");

        self.inner.stop_io().await;
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
        self.dbfile.as_path()
    }

    /// Returns blob directory path.
    pub fn get_blobdir(&self) -> &Path {
        self.blobdir.as_path()
    }

    /// Emits a single event.
    pub fn emit_event(&self, event: EventType) {
        self.events.emit(Event {
            id: self.id,
            typ: event,
        });
    }

    /// Get the next queued event.
    pub fn get_event_emitter(&self) -> EventEmitter {
        self.events.get_emitter()
    }

    /// Get the ID of this context.
    pub fn get_id(&self) -> u32 {
        self.id
    }

    // Ongoing process allocation/free/check

    pub async fn alloc_ongoing(&self) -> Result<Receiver<()>> {
        if self.has_ongoing().await {
            bail!("There is already another ongoing process running.");
        }

        let s_a = &self.running_state;
        let mut s = s_a.write().await;

        s.ongoing_running = true;
        s.shall_stop_ongoing = false;
        let (sender, receiver) = channel(1);
        s.cancel_sender = Some(sender);

        Ok(receiver)
    }

    pub async fn free_ongoing(&self) {
        let s_a = &self.running_state;
        let mut s = s_a.write().await;

        s.ongoing_running = false;
        s.shall_stop_ongoing = true;
        s.cancel_sender.take();
    }

    pub async fn has_ongoing(&self) -> bool {
        let s_a = &self.running_state;
        let s = s_a.read().await;

        s.ongoing_running || !s.shall_stop_ongoing
    }

    /// Signal an ongoing process to stop.
    pub async fn stop_ongoing(&self) {
        let s_a = &self.running_state;
        let mut s = s_a.write().await;
        if let Some(cancel) = s.cancel_sender.take() {
            cancel.send(()).await;
        }

        if s.ongoing_running && !s.shall_stop_ongoing {
            info!(self, "Signaling the ongoing process to stop ASAP.",);
            s.shall_stop_ongoing = true;
        } else {
            info!(self, "No ongoing process to stop.",);
        };
    }

    pub async fn shall_stop_ongoing(&self) -> bool {
        self.running_state.read().await.shall_stop_ongoing
    }

    /*******************************************************************************
     * UI chat/message related API
     ******************************************************************************/

    pub async fn get_info(&self) -> BTreeMap<&'static str, String> {
        let unset = "0";
        let l = LoginParam::from_database(self, "").await;
        let l2 = LoginParam::from_database(self, "configured_").await;
        let displayname = self.get_config(Config::Displayname).await;
        let chats = get_chat_cnt(self).await as usize;
        let real_msgs = message::get_real_msg_cnt(self).await as usize;
        let deaddrop_msgs = message::get_deaddrop_msg_cnt(self).await as usize;
        let contacts = Contact::get_real_cnt(self).await as usize;
        let is_configured = self.get_config_int(Config::Configured).await;
        let dbversion = self
            .sql
            .get_raw_config_int(self, "dbversion")
            .await
            .unwrap_or_default();
        let journal_mode = self
            .sql
            .query_get_value(self, "PRAGMA journal_mode;", paramsv![])
            .await
            .unwrap_or_else(|| "unknown".to_string());
        let e2ee_enabled = self.get_config_int(Config::E2eeEnabled).await;
        let mdns_enabled = self.get_config_int(Config::MdnsEnabled).await;
        let bcc_self = self.get_config_int(Config::BccSelf).await;

        let prv_key_cnt: Option<isize> = self
            .sql
            .query_get_value(self, "SELECT COUNT(*) FROM keypairs;", paramsv![])
            .await;

        let pub_key_cnt: Option<isize> = self
            .sql
            .query_get_value(self, "SELECT COUNT(*) FROM acpeerstates;", paramsv![])
            .await;
        let fingerprint_str = match SignedPublicKey::load_self(self).await {
            Ok(key) => key.fingerprint().hex(),
            Err(err) => format!("<key failure: {}>", err),
        };

        let inbox_watch = self.get_config_int(Config::InboxWatch).await;
        let sentbox_watch = self.get_config_int(Config::SentboxWatch).await;
        let mvbox_watch = self.get_config_int(Config::MvboxWatch).await;
        let mvbox_move = self.get_config_int(Config::MvboxMove).await;
        let folders_configured = self
            .sql
            .get_raw_config_int(self, "folders_configured")
            .await
            .unwrap_or_default();

        let configured_sentbox_folder = self
            .get_config(Config::ConfiguredSentboxFolder)
            .await
            .unwrap_or_else(|| "<unset>".to_string());
        let configured_mvbox_folder = self
            .get_config(Config::ConfiguredMvboxFolder)
            .await
            .unwrap_or_else(|| "<unset>".to_string());

        let mut res = get_info();
        res.insert("number_of_chats", chats.to_string());
        res.insert("number_of_chat_messages", real_msgs.to_string());
        res.insert("messages_in_contact_requests", deaddrop_msgs.to_string());
        res.insert("number_of_contacts", contacts.to_string());
        res.insert("database_dir", self.get_dbfile().display().to_string());
        res.insert("database_version", dbversion.to_string());
        res.insert("journal_mode", journal_mode);
        res.insert("blobdir", self.get_blobdir().display().to_string());
        res.insert("display_name", displayname.unwrap_or_else(|| unset.into()));
        res.insert(
            "selfavatar",
            self.get_config(Config::Selfavatar)
                .await
                .unwrap_or_else(|| "<unset>".to_string()),
        );
        res.insert("is_configured", is_configured.to_string());
        res.insert("entered_account_settings", l.to_string());
        res.insert("used_account_settings", l2.to_string());
        res.insert("inbox_watch", inbox_watch.to_string());
        res.insert("sentbox_watch", sentbox_watch.to_string());
        res.insert("mvbox_watch", mvbox_watch.to_string());
        res.insert("mvbox_move", mvbox_move.to_string());
        res.insert("folders_configured", folders_configured.to_string());
        res.insert("configured_sentbox_folder", configured_sentbox_folder);
        res.insert("configured_mvbox_folder", configured_mvbox_folder);
        res.insert("mdns_enabled", mdns_enabled.to_string());
        res.insert("e2ee_enabled", e2ee_enabled.to_string());
        res.insert("bcc_self", bcc_self.to_string());
        res.insert(
            "private_key_count",
            prv_key_cnt.unwrap_or_default().to_string(),
        );
        res.insert(
            "public_key_count",
            pub_key_cnt.unwrap_or_default().to_string(),
        );
        res.insert("fingerprint", fingerprint_str);

        let elapsed = self.creation_time.elapsed();
        res.insert("uptime", duration_to_str(elapsed.unwrap_or_default()));

        res
    }

    pub async fn get_fresh_msgs(&self) -> Vec<MsgId> {
        let show_deaddrop: i32 = 0;
        self.sql
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
                    "   AND m.chat_id>?",
                    "   AND ct.blocked=0",
                    "   AND (c.blocked=0 OR c.blocked=?)",
                    " ORDER BY m.timestamp DESC,m.id DESC;"
                ),
                paramsv![10, 9, if 0 != show_deaddrop { 2 } else { 0 }],
                |row| row.get::<_, MsgId>(0),
                |rows| {
                    let mut ret = Vec::new();
                    for row in rows {
                        ret.push(row?);
                    }
                    Ok(ret)
                },
            )
            .await
            .unwrap_or_default()
    }

    #[allow(non_snake_case)]
    pub async fn search_msgs(&self, chat_id: ChatId, query: impl AsRef<str>) -> Vec<MsgId> {
        let real_query = query.as_ref().trim();
        if real_query.is_empty() {
            return Vec::new();
        }
        let strLikeInText = format!("%{}%", real_query);
        let strLikeBeg = format!("{}%", real_query);

        let query = if !chat_id.is_unset() {
            concat!(
                "SELECT m.id AS id, m.timestamp AS timestamp",
                " FROM msgs m",
                " LEFT JOIN contacts ct",
                "        ON m.from_id=ct.id",
                " WHERE m.chat_id=?",
                "   AND m.hidden=0",
                "   AND ct.blocked=0",
                "   AND (txt LIKE ? OR ct.name LIKE ?)",
                " ORDER BY m.timestamp,m.id;"
            )
        } else {
            concat!(
                "SELECT m.id AS id, m.timestamp AS timestamp",
                " FROM msgs m",
                " LEFT JOIN contacts ct",
                "        ON m.from_id=ct.id",
                " LEFT JOIN chats c",
                "        ON m.chat_id=c.id",
                " WHERE m.chat_id>9",
                "   AND m.hidden=0",
                "   AND (c.blocked=0 OR c.blocked=?)",
                "   AND ct.blocked=0",
                "   AND (m.txt LIKE ? OR ct.name LIKE ?)",
                " ORDER BY m.timestamp DESC,m.id DESC;"
            )
        };

        self.sql
            .query_map(
                query,
                paramsv![chat_id, strLikeInText, strLikeBeg],
                |row| row.get::<_, MsgId>("id"),
                |rows| {
                    let mut ret = Vec::new();
                    for id in rows {
                        ret.push(id?);
                    }
                    Ok(ret)
                },
            )
            .await
            .unwrap_or_default()
    }

    pub async fn is_inbox(&self, folder_name: impl AsRef<str>) -> bool {
        self.get_config(Config::ConfiguredInboxFolder).await
            == Some(folder_name.as_ref().to_string())
    }

    pub async fn is_sentbox(&self, folder_name: impl AsRef<str>) -> bool {
        self.get_config(Config::ConfiguredSentboxFolder).await
            == Some(folder_name.as_ref().to_string())
    }

    pub async fn is_mvbox(&self, folder_name: impl AsRef<str>) -> bool {
        self.get_config(Config::ConfiguredMvboxFolder).await
            == Some(folder_name.as_ref().to_string())
    }

    pub fn derive_blobdir(dbfile: &PathBuf) -> PathBuf {
        let mut blob_fname = OsString::new();
        blob_fname.push(dbfile.file_name().unwrap_or_default());
        blob_fname.push("-blobs");
        dbfile.with_file_name(blob_fname)
    }
}

impl InnerContext {
    async fn is_io_running(&self) -> bool {
        self.scheduler.read().await.is_running()
    }

    async fn stop_io(&self) {
        if self.is_io_running().await {
            let token = {
                let lock = &*self.scheduler.read().await;
                lock.pre_stop().await
            };
            {
                let lock = &mut *self.scheduler.write().await;
                lock.stop(token).await;
            }
        }

        if let Some(ephemeral_task) = self.ephemeral_task.write().await.take() {
            ephemeral_task.cancel().await;
        }
    }
}

impl Default for RunningState {
    fn default() -> Self {
        RunningState {
            ongoing_running: false,
            shall_stop_ongoing: true,
            cancel_sender: None,
        }
    }
}

pub fn get_version_str() -> &'static str {
    &DC_VERSION_STR
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::test_utils::*;

    #[async_std::test]
    async fn test_wrong_db() {
        let tmp = tempfile::tempdir().unwrap();
        let dbfile = tmp.path().join("db.sqlite");
        std::fs::write(&dbfile, b"123").unwrap();
        let res = Context::new("FakeOs".into(), dbfile.into(), 1).await;
        assert!(res.is_err());
    }

    #[async_std::test]
    async fn test_get_fresh_msgs() {
        let t = TestContext::new().await;
        let fresh = t.ctx.get_fresh_msgs().await;
        assert!(fresh.is_empty())
    }

    #[async_std::test]
    async fn test_blobdir_exists() {
        let tmp = tempfile::tempdir().unwrap();
        let dbfile = tmp.path().join("db.sqlite");
        Context::new("FakeOS".into(), dbfile.into(), 1)
            .await
            .unwrap();
        let blobdir = tmp.path().join("db.sqlite-blobs");
        assert!(blobdir.is_dir());
    }

    #[async_std::test]
    async fn test_wrong_blogdir() {
        let tmp = tempfile::tempdir().unwrap();
        let dbfile = tmp.path().join("db.sqlite");
        let blobdir = tmp.path().join("db.sqlite-blobs");
        std::fs::write(&blobdir, b"123").unwrap();
        let res = Context::new("FakeOS".into(), dbfile.into(), 1).await;
        assert!(res.is_err());
    }

    #[async_std::test]
    async fn test_sqlite_parent_not_exists() {
        let tmp = tempfile::tempdir().unwrap();
        let subdir = tmp.path().join("subdir");
        let dbfile = subdir.join("db.sqlite");
        let dbfile2 = dbfile.clone();
        Context::new("FakeOS".into(), dbfile.into(), 1)
            .await
            .unwrap();
        assert!(subdir.is_dir());
        assert!(dbfile2.is_file());
    }

    #[async_std::test]
    async fn test_with_empty_blobdir() {
        let tmp = tempfile::tempdir().unwrap();
        let dbfile = tmp.path().join("db.sqlite");
        let blobdir = PathBuf::new();
        let res = Context::with_blobdir("FakeOS".into(), dbfile.into(), blobdir, 1).await;
        assert!(res.is_err());
    }

    #[async_std::test]
    async fn test_with_blobdir_not_exists() {
        let tmp = tempfile::tempdir().unwrap();
        let dbfile = tmp.path().join("db.sqlite");
        let blobdir = tmp.path().join("blobs");
        let res = Context::with_blobdir("FakeOS".into(), dbfile.into(), blobdir.into(), 1).await;
        assert!(res.is_err());
    }

    #[async_std::test]
    async fn no_crashes_on_context_deref() {
        let t = TestContext::new().await;
        std::mem::drop(t.ctx);
    }

    #[async_std::test]
    async fn test_get_info() {
        let t = TestContext::new().await;

        let info = t.ctx.get_info().await;
        assert!(info.get("database_dir").is_some());
    }

    #[test]
    fn test_get_info_no_context() {
        let info = get_info();
        assert!(info.get("deltachat_core_version").is_some());
        assert!(info.get("database_dir").is_none());
        assert_eq!(info.get("level").unwrap(), "awesome");
    }
}
