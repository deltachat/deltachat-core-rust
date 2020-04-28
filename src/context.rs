//! Context module

use std::collections::{BTreeMap, HashMap};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex, RwLock};

use crate::chat::*;
use crate::config::Config;
use crate::constants::*;
use crate::contact::*;
use crate::dc_tools::duration_to_str;
use crate::error::*;
use crate::events::Event;
use crate::imap::*;
use crate::job::*;
use crate::job_thread::JobThread;
use crate::key::{DcKey, Key, SignedPublicKey};
use crate::login_param::LoginParam;
use crate::lot::Lot;
use crate::message::{self, Message, MessengerMessage, MsgId};
use crate::param::Params;
use crate::smtp::Smtp;
use crate::sql::Sql;
use std::time::SystemTime;

/// Callback function type for [Context]
///
/// # Parameters
///
/// * `context` - The context object as returned by [Context::new].
/// * `event` - One of the [Event] items.
/// * `data1` - Depends on the event parameter, see [Event].
/// * `data2` - Depends on the event parameter, see [Event].
pub type ContextCallback = dyn Fn(&Context, Event) -> () + Send + Sync;

#[derive(DebugStub)]
pub struct Context {
    /// Database file path
    dbfile: PathBuf,
    /// Blob directory path
    blobdir: PathBuf,
    pub sql: Sql,
    pub perform_inbox_jobs_needed: Arc<RwLock<bool>>,
    pub probe_imap_network: Arc<RwLock<bool>>,
    pub inbox_thread: Arc<RwLock<JobThread>>,
    pub sentbox_thread: Arc<RwLock<JobThread>>,
    pub mvbox_thread: Arc<RwLock<JobThread>>,
    pub smtp: Arc<Mutex<Smtp>>,
    pub smtp_state: Arc<(Mutex<SmtpState>, Condvar)>,
    pub oauth2_critical: Arc<Mutex<()>>,
    #[debug_stub = "Callback"]
    cb: Box<ContextCallback>,
    pub os_name: Option<String>,
    pub cmdline_sel_chat_id: Arc<RwLock<ChatId>>,
    pub(crate) bob: Arc<RwLock<BobStatus>>,
    pub last_smeared_timestamp: RwLock<i64>,
    pub running_state: Arc<RwLock<RunningState>>,
    /// Mutex to avoid generating the key for the user more than once.
    pub generating_key_mutex: Mutex<()>,
    pub translated_stockstrings: RwLock<HashMap<usize, String>>,
    creation_time: SystemTime,
}

#[derive(Debug, PartialEq, Eq)]
pub struct RunningState {
    pub ongoing_running: bool,
    shall_stop_ongoing: bool,
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
    pub fn new(cb: Box<ContextCallback>, os_name: String, dbfile: PathBuf) -> Result<Context> {
        pretty_env_logger::try_init_timed().ok();

        let mut blob_fname = OsString::new();
        blob_fname.push(dbfile.file_name().unwrap_or_default());
        blob_fname.push("-blobs");
        let blobdir = dbfile.with_file_name(blob_fname);
        if !blobdir.exists() {
            std::fs::create_dir_all(&blobdir)?;
        }
        Context::with_blobdir(cb, os_name, dbfile, blobdir)
    }

    pub fn with_blobdir(
        cb: Box<ContextCallback>,
        os_name: String,
        dbfile: PathBuf,
        blobdir: PathBuf,
    ) -> Result<Context> {
        ensure!(
            blobdir.is_dir(),
            "Blobdir does not exist: {}",
            blobdir.display()
        );
        let ctx = Context {
            blobdir,
            dbfile,
            cb,
            os_name: Some(os_name),
            running_state: Arc::new(RwLock::new(Default::default())),
            sql: Sql::new(),
            smtp: Arc::new(Mutex::new(Smtp::new())),
            smtp_state: Arc::new((Mutex::new(Default::default()), Condvar::new())),
            oauth2_critical: Arc::new(Mutex::new(())),
            bob: Arc::new(RwLock::new(Default::default())),
            last_smeared_timestamp: RwLock::new(0),
            cmdline_sel_chat_id: Arc::new(RwLock::new(ChatId::new(0))),
            inbox_thread: Arc::new(RwLock::new(JobThread::new(
                "INBOX",
                "configured_inbox_folder",
                Imap::new(),
            ))),
            sentbox_thread: Arc::new(RwLock::new(JobThread::new(
                "SENTBOX",
                "configured_sentbox_folder",
                Imap::new(),
            ))),
            mvbox_thread: Arc::new(RwLock::new(JobThread::new(
                "MVBOX",
                "configured_mvbox_folder",
                Imap::new(),
            ))),
            probe_imap_network: Arc::new(RwLock::new(false)),
            perform_inbox_jobs_needed: Arc::new(RwLock::new(false)),
            generating_key_mutex: Mutex::new(()),
            translated_stockstrings: RwLock::new(HashMap::new()),
            creation_time: std::time::SystemTime::now(),
        };

        ensure!(
            ctx.sql.open(&ctx, &ctx.dbfile, false),
            "Failed opening sqlite database"
        );

        Ok(ctx)
    }

    /// Returns database file path.
    pub fn get_dbfile(&self) -> &Path {
        self.dbfile.as_path()
    }

    /// Returns blob directory path.
    pub fn get_blobdir(&self) -> &Path {
        self.blobdir.as_path()
    }

    pub fn call_cb(&self, event: Event) {
        (*self.cb)(self, event);
    }

    /*******************************************************************************
     * Ongoing process allocation/free/check
     ******************************************************************************/

    pub fn alloc_ongoing(&self) -> bool {
        if self.has_ongoing() {
            warn!(self, "There is already another ongoing process running.",);

            false
        } else {
            let s_a = self.running_state.clone();
            let mut s = s_a.write().unwrap();

            s.ongoing_running = true;
            s.shall_stop_ongoing = false;

            true
        }
    }

    pub fn free_ongoing(&self) {
        let s_a = self.running_state.clone();
        let mut s = s_a.write().unwrap();

        s.ongoing_running = false;
        s.shall_stop_ongoing = true;
    }

    pub fn has_ongoing(&self) -> bool {
        let s_a = self.running_state.clone();
        let s = s_a.read().unwrap();

        s.ongoing_running || !s.shall_stop_ongoing
    }

    /// Signal an ongoing process to stop.
    pub fn stop_ongoing(&self) {
        let s_a = self.running_state.clone();
        let mut s = s_a.write().unwrap();

        if s.ongoing_running && !s.shall_stop_ongoing {
            info!(self, "Signaling the ongoing process to stop ASAP.",);
            s.shall_stop_ongoing = true;
        } else {
            info!(self, "No ongoing process to stop.",);
        };
    }

    pub fn shall_stop_ongoing(&self) -> bool {
        self.running_state
            .clone()
            .read()
            .unwrap()
            .shall_stop_ongoing
    }

    /*******************************************************************************
     * UI chat/message related API
     ******************************************************************************/

    pub fn get_info(&self) -> BTreeMap<&'static str, String> {
        let unset = "0";
        let l = LoginParam::from_database(self, "");
        let l2 = LoginParam::from_database(self, "configured_");
        let displayname = self.get_config(Config::Displayname);
        let chats = get_chat_cnt(self) as usize;
        let real_msgs = message::get_real_msg_cnt(self) as usize;
        let deaddrop_msgs = message::get_deaddrop_msg_cnt(self) as usize;
        let contacts = Contact::get_real_cnt(self) as usize;
        let is_configured = self.get_config_int(Config::Configured);
        let dbversion = self
            .sql
            .get_raw_config_int(self, "dbversion")
            .unwrap_or_default();

        let e2ee_enabled = self.get_config_int(Config::E2eeEnabled);
        let mdns_enabled = self.get_config_int(Config::MdnsEnabled);
        let bcc_self = self.get_config_int(Config::BccSelf);

        let prv_key_cnt: Option<isize> =
            self.sql
                .query_get_value(self, "SELECT COUNT(*) FROM keypairs;", rusqlite::NO_PARAMS);

        let pub_key_cnt: Option<isize> = self.sql.query_get_value(
            self,
            "SELECT COUNT(*) FROM acpeerstates;",
            rusqlite::NO_PARAMS,
        );

        let fingerprint_str = match SignedPublicKey::load_self(self) {
            Ok(key) => Key::from(key).fingerprint(),
            Err(err) => format!("<key failure: {}>", err),
        };

        let inbox_watch = self.get_config_int(Config::InboxWatch);
        let sentbox_watch = self.get_config_int(Config::SentboxWatch);
        let mvbox_watch = self.get_config_int(Config::MvboxWatch);
        let mvbox_move = self.get_config_int(Config::MvboxMove);
        let folders_configured = self
            .sql
            .get_raw_config_int(self, "folders_configured")
            .unwrap_or_default();

        let configured_sentbox_folder = self
            .sql
            .get_raw_config(self, "configured_sentbox_folder")
            .unwrap_or_else(|| "<unset>".to_string());
        let configured_mvbox_folder = self
            .sql
            .get_raw_config(self, "configured_mvbox_folder")
            .unwrap_or_else(|| "<unset>".to_string());

        let mut res = get_info();
        res.insert("number_of_chats", chats.to_string());
        res.insert("number_of_chat_messages", real_msgs.to_string());
        res.insert("messages_in_contact_requests", deaddrop_msgs.to_string());
        res.insert("number_of_contacts", contacts.to_string());
        res.insert("database_dir", self.get_dbfile().display().to_string());
        res.insert("database_version", dbversion.to_string());
        res.insert("blobdir", self.get_blobdir().display().to_string());
        res.insert("display_name", displayname.unwrap_or_else(|| unset.into()));
        res.insert(
            "selfavatar",
            self.get_config(Config::Selfavatar)
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

    pub fn get_fresh_msgs(&self) -> Vec<MsgId> {
        let show_deaddrop = 0;
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
                &[10, 9, if 0 != show_deaddrop { 2 } else { 0 }],
                |row| row.get::<_, MsgId>(0),
                |rows| {
                    let mut ret = Vec::new();
                    for row in rows {
                        ret.push(row?);
                    }
                    Ok(ret)
                },
            )
            .unwrap_or_default()
    }

    #[allow(non_snake_case)]
    pub fn search_msgs(&self, chat_id: ChatId, query: impl AsRef<str>) -> Vec<MsgId> {
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
                params![chat_id, &strLikeInText, &strLikeBeg],
                |row| row.get::<_, MsgId>("id"),
                |rows| {
                    let mut ret = Vec::new();
                    for id in rows {
                        ret.push(id?);
                    }
                    Ok(ret)
                },
            )
            .unwrap_or_default()
    }

    pub fn is_inbox(&self, folder_name: impl AsRef<str>) -> bool {
        folder_name.as_ref() == "INBOX"
    }

    pub fn is_sentbox(&self, folder_name: impl AsRef<str>) -> bool {
        let sentbox_name = self.sql.get_raw_config(self, "configured_sentbox_folder");
        if let Some(name) = sentbox_name {
            name == folder_name.as_ref()
        } else {
            false
        }
    }

    pub fn is_mvbox(&self, folder_name: impl AsRef<str>) -> bool {
        let mvbox_name = self.sql.get_raw_config(self, "configured_mvbox_folder");

        if let Some(name) = mvbox_name {
            name == folder_name.as_ref()
        } else {
            false
        }
    }

    pub fn do_heuristics_moves(&self, folder: &str, msg_id: MsgId) {
        if !self.get_config_bool(Config::MvboxMove) {
            return;
        }

        if self.is_mvbox(folder) {
            return;
        }
        if let Ok(msg) = Message::load_from_db(self, msg_id) {
            if msg.is_setupmessage() {
                // do not move setup messages;
                // there may be a non-delta device that wants to handle it
                return;
            }

            match msg.is_dc_message {
                MessengerMessage::No => {}
                MessengerMessage::Yes | MessengerMessage::Reply => {
                    job_add(
                        self,
                        Action::MoveMsg,
                        msg.id.to_u32() as i32,
                        Params::new(),
                        0,
                    );
                }
            }
        }
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        info!(self, "disconnecting inbox-thread",);
        self.inbox_thread.read().unwrap().imap.disconnect(self);
        info!(self, "disconnecting sentbox-thread",);
        self.sentbox_thread.read().unwrap().imap.disconnect(self);
        info!(self, "disconnecting mvbox-thread",);
        self.mvbox_thread.read().unwrap().imap.disconnect(self);
        info!(self, "disconnecting SMTP");
        self.smtp.clone().lock().unwrap().disconnect();
        self.sql.close(self);
    }
}

impl Default for RunningState {
    fn default() -> Self {
        RunningState {
            ongoing_running: false,
            shall_stop_ongoing: true,
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct BobStatus {
    pub expects: i32,
    pub status: i32,
    pub qr_scan: Option<Lot>,
}

#[derive(Debug, PartialEq)]
pub(crate) enum PerformJobsNeeded {
    Not,
    AtOnce,
    AvoidDos,
}

impl Default for PerformJobsNeeded {
    fn default() -> Self {
        Self::Not
    }
}

#[derive(Default, Debug)]
pub struct SmtpState {
    pub idle: bool,
    pub suspended: bool,
    pub doing_jobs: bool,
    pub(crate) perform_jobs_needed: PerformJobsNeeded,
    pub probe_network: bool,
}

pub fn get_version_str() -> &'static str {
    &DC_VERSION_STR
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::test_utils::*;

    #[test]
    fn test_wrong_db() {
        let tmp = tempfile::tempdir().unwrap();
        let dbfile = tmp.path().join("db.sqlite");
        std::fs::write(&dbfile, b"123").unwrap();
        let res = Context::new(Box::new(|_, _| ()), "FakeOs".into(), dbfile);
        assert!(res.is_err());
    }

    #[test]
    fn test_get_fresh_msgs() {
        let t = dummy_context();
        let fresh = t.ctx.get_fresh_msgs();
        assert!(fresh.is_empty())
    }

    #[test]
    fn test_blobdir_exists() {
        let tmp = tempfile::tempdir().unwrap();
        let dbfile = tmp.path().join("db.sqlite");
        Context::new(Box::new(|_, _| ()), "FakeOS".into(), dbfile).unwrap();
        let blobdir = tmp.path().join("db.sqlite-blobs");
        assert!(blobdir.is_dir());
    }

    #[test]
    fn test_wrong_blogdir() {
        let tmp = tempfile::tempdir().unwrap();
        let dbfile = tmp.path().join("db.sqlite");
        let blobdir = tmp.path().join("db.sqlite-blobs");
        std::fs::write(&blobdir, b"123").unwrap();
        let res = Context::new(Box::new(|_, _| ()), "FakeOS".into(), dbfile);
        assert!(res.is_err());
    }

    #[test]
    fn test_sqlite_parent_not_exists() {
        let tmp = tempfile::tempdir().unwrap();
        let subdir = tmp.path().join("subdir");
        let dbfile = subdir.join("db.sqlite");
        let dbfile2 = dbfile.clone();
        Context::new(Box::new(|_, _| ()), "FakeOS".into(), dbfile).unwrap();
        assert!(subdir.is_dir());
        assert!(dbfile2.is_file());
    }

    #[test]
    fn test_with_empty_blobdir() {
        let tmp = tempfile::tempdir().unwrap();
        let dbfile = tmp.path().join("db.sqlite");
        let blobdir = PathBuf::new();
        let res = Context::with_blobdir(Box::new(|_, _| ()), "FakeOS".into(), dbfile, blobdir);
        assert!(res.is_err());
    }

    #[test]
    fn test_with_blobdir_not_exists() {
        let tmp = tempfile::tempdir().unwrap();
        let dbfile = tmp.path().join("db.sqlite");
        let blobdir = tmp.path().join("blobs");
        let res = Context::with_blobdir(Box::new(|_, _| ()), "FakeOS".into(), dbfile, blobdir);
        assert!(res.is_err());
    }

    #[test]
    fn no_crashes_on_context_deref() {
        let t = dummy_context();
        std::mem::drop(t.ctx);
    }

    #[test]
    fn test_get_info() {
        let t = dummy_context();

        let info = t.ctx.get_info();
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
