//! # Account manager module.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{ensure, Context as _, Result};
use serde::{Deserialize, Serialize};
use tokio::fs;
use uuid::Uuid;

use crate::context::Context;
use crate::events::{Event, EventEmitter, EventType, Events};
use crate::stock_str::StockStrings;

/// Account manager, that can handle multiple accounts in a single place.
#[derive(Debug)]
pub struct Accounts {
    dir: PathBuf,
    config: Config,
    accounts: BTreeMap<u32, Context>,

    /// Event channel to emit account manager errors.
    events: Events,

    /// Stock string translations shared by all created contexts.
    ///
    /// This way changing a translation for one context automatically
    /// changes it for all other contexts.
    pub(crate) stockstrings: StockStrings,
}

impl Accounts {
    /// Loads or creates an accounts folder at the given `dir`.
    pub async fn new(dir: PathBuf) -> Result<Self> {
        if !dir.exists() {
            Accounts::create(&dir).await?;
        }

        Accounts::open(dir).await
    }

    /// Creates a new default structure.
    pub async fn create(dir: &Path) -> Result<()> {
        fs::create_dir_all(dir)
            .await
            .context("failed to create folder")?;

        Config::new(dir).await?;

        Ok(())
    }

    /// Opens an existing accounts structure. Will error if the folder doesn't exist,
    /// no account exists and no config exists.
    pub async fn open(dir: PathBuf) -> Result<Self> {
        ensure!(dir.exists(), "directory does not exist");

        let config_file = dir.join(CONFIG_NAME);
        ensure!(config_file.exists(), "{:?} does not exist", config_file);

        let config = Config::from_file(config_file)
            .await
            .context("failed to load accounts config")?;
        let events = Events::new();
        let stockstrings = StockStrings::new();
        let accounts = config
            .load_accounts(&events, &stockstrings, &dir)
            .await
            .context("failed to load accounts")?;

        Ok(Self {
            dir,
            config,
            accounts,
            events,
            stockstrings,
        })
    }

    /// Get an account by its `id`:
    pub fn get_account(&self, id: u32) -> Option<Context> {
        self.accounts.get(&id).cloned()
    }

    /// Get the currently selected account.
    pub fn get_selected_account(&self) -> Option<Context> {
        let id = self.config.get_selected_account();
        self.accounts.get(&id).cloned()
    }

    /// Returns the currently selected account's id or None if no account is selected.
    pub fn get_selected_account_id(&self) -> Option<u32> {
        match self.config.get_selected_account() {
            0 => None,
            id => Some(id),
        }
    }

    /// Select the given account.
    pub async fn select_account(&mut self, id: u32) -> Result<()> {
        self.config.select_account(id).await?;

        Ok(())
    }

    /// Add a new account and opens it.
    ///
    /// Returns account ID.
    pub async fn add_account(&mut self) -> Result<u32> {
        let account_config = self.config.new_account().await?;
        let dbfile = account_config.dbfile(&self.dir);

        let ctx = Context::new(
            &dbfile,
            account_config.id,
            self.events.clone(),
            self.stockstrings.clone(),
        )
        .await?;
        self.accounts.insert(account_config.id, ctx);

        Ok(account_config.id)
    }

    /// Adds a new closed account.
    pub async fn add_closed_account(&mut self) -> Result<u32> {
        let account_config = self.config.new_account().await?;

        let ctx = Context::new_closed(
            &account_config.dbfile(&self.dir),
            account_config.id,
            self.events.clone(),
            self.stockstrings.clone(),
        )
        .await?;
        self.accounts.insert(account_config.id, ctx);

        Ok(account_config.id)
    }

    /// Remove an account.
    pub async fn remove_account(&mut self, id: u32) -> Result<()> {
        let ctx = self
            .accounts
            .remove(&id)
            .with_context(|| format!("no account with id {id}"))?;
        ctx.stop_io().await;
        drop(ctx);

        if let Some(cfg) = self.config.get_account(id) {
            let account_path = self.dir.join(cfg.dir);

            fs::remove_dir_all(&account_path)
                .await
                .context("failed to remove account data")?;
        }
        self.config.remove_account(id).await?;

        Ok(())
    }

    /// Migrate an existing account into this structure.
    ///
    /// Returns the ID of new account.
    pub async fn migrate_account(&mut self, dbfile: PathBuf) -> Result<u32> {
        let blobdir = Context::derive_blobdir(&dbfile);
        let walfile = Context::derive_walfile(&dbfile);

        ensure!(dbfile.exists(), "no database found: {}", dbfile.display());
        ensure!(blobdir.exists(), "no blobdir found: {}", blobdir.display());

        let old_id = self.config.get_selected_account();

        // create new account
        let account_config = self
            .config
            .new_account()
            .await
            .context("failed to create new account")?;

        let new_dbfile = account_config.dbfile(&self.dir);
        let new_blobdir = Context::derive_blobdir(&new_dbfile);
        let new_walfile = Context::derive_walfile(&new_dbfile);

        let res = {
            fs::create_dir_all(self.dir.join(&account_config.dir))
                .await
                .context("failed to create dir")?;
            fs::rename(&dbfile, &new_dbfile)
                .await
                .context("failed to rename dbfile")?;
            fs::rename(&blobdir, &new_blobdir)
                .await
                .context("failed to rename blobdir")?;
            if walfile.exists() {
                fs::rename(&walfile, &new_walfile)
                    .await
                    .context("failed to rename walfile")?;
            }
            Ok(())
        };

        match res {
            Ok(_) => {
                let ctx = Context::new(
                    &new_dbfile,
                    account_config.id,
                    self.events.clone(),
                    self.stockstrings.clone(),
                )
                .await?;
                self.accounts.insert(account_config.id, ctx);
                Ok(account_config.id)
            }
            Err(err) => {
                let account_path = std::path::PathBuf::from(&account_config.dir);
                fs::remove_dir_all(&account_path)
                    .await
                    .context("failed to remove account data")?;
                self.config.remove_account(account_config.id).await?;

                // set selection back
                self.select_account(old_id).await?;

                Err(err)
            }
        }
    }

    /// Get a list of all account ids.
    pub fn get_all(&self) -> Vec<u32> {
        self.accounts.keys().copied().collect()
    }

    /// This is meant especially for iOS, because iOS needs to tell the system when its background work is done.
    ///
    /// Returns whether all accounts finished their background work.
    /// DC_EVENT_CONNECTIVITY_CHANGED will be sent when this turns to true.
    ///
    /// iOS can:
    /// - call dc_start_io() (in case IO was not running)
    /// - call dc_maybe_network()
    /// - while dc_accounts_all_work_done() returns false:
    ///   -  Wait for DC_EVENT_CONNECTIVITY_CHANGED
    pub async fn all_work_done(&self) -> bool {
        for account in self.accounts.values() {
            if !account.all_work_done().await {
                return false;
            }
        }
        true
    }

    /// Starts background tasks such as IMAP and SMTP loops for all accounts.
    pub async fn start_io(&self) {
        for account in self.accounts.values() {
            account.start_io().await;
        }
    }

    /// Stops background tasks for all accounts.
    pub async fn stop_io(&self) {
        // Sending an event here wakes up event loop even
        // if there are no accounts.
        info!(self, "Stopping IO for all accounts.");
        for account in self.accounts.values() {
            account.stop_io().await;
        }
    }

    /// Notifies all accounts that the network may have become available.
    pub async fn maybe_network(&self) {
        for account in self.accounts.values() {
            account.scheduler.maybe_network().await;
        }
    }

    /// Notifies all accounts that the network connection may have been lost.
    pub async fn maybe_network_lost(&self) {
        for account in self.accounts.values() {
            account.scheduler.maybe_network_lost(account).await;
        }
    }

    /// Emits a single event.
    pub fn emit_event(&self, event: EventType) {
        self.events.emit(Event { id: 0, typ: event })
    }

    /// Returns event emitter.
    pub fn get_event_emitter(&self) -> EventEmitter {
        self.events.get_emitter()
    }
}

/// Configuration file name.
pub const CONFIG_NAME: &str = "accounts.toml";

/// Database file name.
pub const DB_NAME: &str = "dc.db";

/// Account manager configuration file.
#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    file: PathBuf,
    inner: InnerConfig,
}

/// Account manager configuration file contents.
///
/// This is serialized into TOML.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct InnerConfig {
    /// The currently selected account.
    pub selected_account: u32,
    pub next_id: u32,
    pub accounts: Vec<AccountConfig>,
}

impl Config {
    /// Creates a new configuration file in the given account manager directory.
    pub async fn new(dir: &Path) -> Result<Self> {
        let inner = InnerConfig {
            accounts: Vec::new(),
            selected_account: 0,
            next_id: 1,
        };
        let cfg = Config {
            file: dir.join(CONFIG_NAME),
            inner,
        };

        cfg.sync().await?;

        Ok(cfg)
    }

    /// Sync the inmemory representation to disk.
    async fn sync(&self) -> Result<()> {
        fs::write(&self.file, toml::to_string_pretty(&self.inner)?)
            .await
            .context("failed to write config")
    }

    /// Read a configuration from the given file into memory.
    pub async fn from_file(file: PathBuf) -> Result<Self> {
        let dir = file.parent().context("can't get config file directory")?;
        let bytes = fs::read(&file).await.context("failed to read file")?;
        let s = std::str::from_utf8(&bytes)?;
        let mut inner: InnerConfig = toml::from_str(s).context("failed to parse config")?;

        // Previous versions of the core stored absolute paths in account config.
        // Convert them to relative paths.
        let mut modified = false;
        for account in &mut inner.accounts {
            if let Ok(new_dir) = account.dir.strip_prefix(dir) {
                account.dir = new_dir.to_path_buf();
                modified = true;
            }
        }

        let config = Self { file, inner };
        if modified {
            config.sync().await?;
        }

        Ok(config)
    }

    /// Loads all accounts defined in the configuration file.
    ///
    /// Created contexts share the same event channel and stock string
    /// translations.
    pub async fn load_accounts(
        &self,
        events: &Events,
        stockstrings: &StockStrings,
        dir: &Path,
    ) -> Result<BTreeMap<u32, Context>> {
        let mut accounts = BTreeMap::new();

        for account_config in &self.inner.accounts {
            let ctx = Context::new(
                &account_config.dbfile(dir),
                account_config.id,
                events.clone(),
                stockstrings.clone(),
            )
            .await
            .with_context(|| {
                format!(
                    "failed to create context from file {:?}",
                    account_config.dbfile(dir)
                )
            })?;

            accounts.insert(account_config.id, ctx);
        }

        Ok(accounts)
    }

    /// Creates a new account in the account manager directory.
    async fn new_account(&mut self) -> Result<AccountConfig> {
        let id = {
            let id = self.inner.next_id;
            let uuid = Uuid::new_v4();
            let target_dir = PathBuf::from(uuid.to_string());

            self.inner.accounts.push(AccountConfig {
                id,
                dir: target_dir,
                uuid,
            });
            self.inner.next_id += 1;
            id
        };

        self.sync().await?;

        self.select_account(id)
            .await
            .context("failed to select just added account")?;
        let cfg = self
            .get_account(id)
            .context("failed to get just added account")?;
        Ok(cfg)
    }

    /// Removes an existing account entirely.
    pub async fn remove_account(&mut self, id: u32) -> Result<()> {
        {
            if let Some(idx) = self.inner.accounts.iter().position(|e| e.id == id) {
                // remove account from the configs
                self.inner.accounts.remove(idx);
            }
            if self.inner.selected_account == id {
                // reset selected account
                self.inner.selected_account =
                    self.inner.accounts.get(0).map(|e| e.id).unwrap_or_default();
            }
        }

        self.sync().await
    }

    /// Returns configuration file section for the given account ID.
    fn get_account(&self, id: u32) -> Option<AccountConfig> {
        self.inner.accounts.iter().find(|e| e.id == id).cloned()
    }

    /// Returns the ID of selected account.
    pub fn get_selected_account(&self) -> u32 {
        self.inner.selected_account
    }

    /// Changes selected account ID.
    pub async fn select_account(&mut self, id: u32) -> Result<()> {
        {
            ensure!(
                self.inner.accounts.iter().any(|e| e.id == id),
                "invalid account id: {}",
                id
            );

            self.inner.selected_account = id;
        }

        self.sync().await?;
        Ok(())
    }
}

/// Configuration of a single account.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct AccountConfig {
    /// Unique id.
    pub id: u32,

    /// Root directory for all data for this account.
    ///
    /// The path is relative to the account manager directory.
    pub dir: std::path::PathBuf,

    /// Universally unique account identifier.
    pub uuid: Uuid,
}

impl AccountConfig {
    /// Get the canonical dbfile name for this configuration.
    pub fn dbfile(&self, accounts_dir: &Path) -> std::path::PathBuf {
        accounts_dir.join(&self.dir).join(DB_NAME)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stock_str::{self, StockMessage};

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_account_new_open() {
        let dir = tempfile::tempdir().unwrap();
        let p: PathBuf = dir.path().join("accounts1");

        let mut accounts1 = Accounts::new(p.clone()).await.unwrap();
        accounts1.add_account().await.unwrap();

        let accounts2 = Accounts::open(p).await.unwrap();

        assert_eq!(accounts1.accounts.len(), 1);
        assert_eq!(accounts1.config.get_selected_account(), 1);

        assert_eq!(accounts1.dir, accounts2.dir);
        assert_eq!(accounts1.config, accounts2.config,);
        assert_eq!(accounts1.accounts.len(), accounts2.accounts.len());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_account_new_add_remove() {
        let dir = tempfile::tempdir().unwrap();
        let p: PathBuf = dir.path().join("accounts");

        let mut accounts = Accounts::new(p.clone()).await.unwrap();
        assert_eq!(accounts.accounts.len(), 0);
        assert_eq!(accounts.config.get_selected_account(), 0);

        let id = accounts.add_account().await.unwrap();
        assert_eq!(id, 1);
        assert_eq!(accounts.accounts.len(), 1);
        assert_eq!(accounts.config.get_selected_account(), 1);

        let id = accounts.add_account().await.unwrap();
        assert_eq!(id, 2);
        assert_eq!(accounts.config.get_selected_account(), id);
        assert_eq!(accounts.accounts.len(), 2);

        accounts.select_account(1).await.unwrap();
        assert_eq!(accounts.config.get_selected_account(), 1);

        accounts.remove_account(1).await.unwrap();
        assert_eq!(accounts.config.get_selected_account(), 2);
        assert_eq!(accounts.accounts.len(), 1);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_accounts_remove_last() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let p: PathBuf = dir.path().join("accounts");

        let mut accounts = Accounts::new(p.clone()).await?;
        assert!(accounts.get_selected_account().is_none());
        assert_eq!(accounts.config.get_selected_account(), 0);

        let id = accounts.add_account().await?;
        assert!(accounts.get_selected_account().is_some());
        assert_eq!(id, 1);
        assert_eq!(accounts.accounts.len(), 1);
        assert_eq!(accounts.config.get_selected_account(), id);

        accounts.remove_account(id).await?;
        assert!(accounts.get_selected_account().is_none());

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_migrate_account() {
        let dir = tempfile::tempdir().unwrap();
        let p: PathBuf = dir.path().join("accounts");

        let mut accounts = Accounts::new(p.clone()).await.unwrap();
        assert_eq!(accounts.accounts.len(), 0);
        assert_eq!(accounts.config.get_selected_account(), 0);

        let extern_dbfile: PathBuf = dir.path().join("other");
        let ctx = Context::new(&extern_dbfile, 0, Events::new(), StockStrings::new())
            .await
            .unwrap();
        ctx.set_config(crate::config::Config::Addr, Some("me@mail.com"))
            .await
            .unwrap();

        drop(ctx);

        accounts
            .migrate_account(extern_dbfile.clone())
            .await
            .unwrap();
        assert_eq!(accounts.accounts.len(), 1);
        assert_eq!(accounts.config.get_selected_account(), 1);

        let ctx = accounts.get_selected_account().unwrap();
        assert_eq!(
            "me@mail.com",
            ctx.get_config(crate::config::Config::Addr)
                .await
                .unwrap()
                .unwrap()
        );
    }

    /// Tests that accounts are sorted by ID.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_accounts_sorted() {
        let dir = tempfile::tempdir().unwrap();
        let p: PathBuf = dir.path().join("accounts");

        let mut accounts = Accounts::new(p.clone()).await.unwrap();

        for expected_id in 1..10 {
            let id = accounts.add_account().await.unwrap();
            assert_eq!(id, expected_id);
        }

        let ids = accounts.get_all();
        for (i, expected_id) in (1..10).enumerate() {
            assert_eq!(ids.get(i), Some(&expected_id));
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_accounts_ids_unique_increasing_and_persisted() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let p: PathBuf = dir.path().join("accounts");
        let dummy_accounts = 10;

        let (id0, id1, id2) = {
            let mut accounts = Accounts::new(p.clone()).await?;
            accounts.add_account().await?;
            let ids = accounts.get_all();
            assert_eq!(ids.len(), 1);

            let id0 = *ids.first().unwrap();
            let ctx = accounts.get_account(id0).unwrap();
            ctx.set_config(crate::config::Config::Addr, Some("one@example.org"))
                .await?;

            let id1 = accounts.add_account().await?;
            let ctx = accounts.get_account(id1).unwrap();
            ctx.set_config(crate::config::Config::Addr, Some("two@example.org"))
                .await?;

            // add and remove some accounts and force a gap (ids must not be reused)
            for _ in 0..dummy_accounts {
                let to_delete = accounts.add_account().await?;
                accounts.remove_account(to_delete).await?;
            }

            let id2 = accounts.add_account().await?;
            let ctx = accounts.get_account(id2).unwrap();
            ctx.set_config(crate::config::Config::Addr, Some("three@example.org"))
                .await?;

            accounts.select_account(id1).await?;

            (id0, id1, id2)
        };
        assert!(id0 > 0);
        assert!(id1 > id0);
        assert!(id2 > id1 + dummy_accounts);

        let (id0_reopened, id1_reopened, id2_reopened) = {
            let accounts = Accounts::new(p.clone()).await?;
            let ctx = accounts.get_selected_account().unwrap();
            assert_eq!(
                ctx.get_config(crate::config::Config::Addr).await?,
                Some("two@example.org".to_string())
            );

            let ids = accounts.get_all();
            assert_eq!(ids.len(), 3);

            let id0 = *ids.first().unwrap();
            let ctx = accounts.get_account(id0).unwrap();
            assert_eq!(
                ctx.get_config(crate::config::Config::Addr).await?,
                Some("one@example.org".to_string())
            );

            let id1 = *ids.get(1).unwrap();
            let t = accounts.get_account(id1).unwrap();
            assert_eq!(
                t.get_config(crate::config::Config::Addr).await?,
                Some("two@example.org".to_string())
            );

            let id2 = *ids.get(2).unwrap();
            let ctx = accounts.get_account(id2).unwrap();
            assert_eq!(
                ctx.get_config(crate::config::Config::Addr).await?,
                Some("three@example.org".to_string())
            );

            (id0, id1, id2)
        };
        assert_eq!(id0, id0_reopened);
        assert_eq!(id1, id1_reopened);
        assert_eq!(id2, id2_reopened);

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_no_accounts_event_emitter() -> Result<()> {
        let dir = tempfile::tempdir().unwrap();
        let p: PathBuf = dir.path().join("accounts");

        let accounts = Accounts::new(p.clone()).await?;

        // Make sure there are no accounts.
        assert_eq!(accounts.accounts.len(), 0);

        // Create event emitter.
        let event_emitter = accounts.get_event_emitter();

        // Test that event emitter does not return `None` immediately.
        let duration = std::time::Duration::from_millis(1);
        assert!(tokio::time::timeout(duration, event_emitter.recv())
            .await
            .is_err());

        // When account manager is dropped, event emitter is exhausted.
        drop(accounts);
        assert_eq!(event_emitter.recv().await, None);

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_encrypted_account() -> Result<()> {
        let dir = tempfile::tempdir().context("failed to create tempdir")?;
        let p: PathBuf = dir.path().join("accounts");

        let mut accounts = Accounts::new(p.clone())
            .await
            .context("failed to create accounts manager")?;

        assert_eq!(accounts.accounts.len(), 0);
        let account_id = accounts
            .add_closed_account()
            .await
            .context("failed to add closed account")?;
        let account = accounts
            .get_selected_account()
            .context("failed to get account")?;
        assert_eq!(account.id, account_id);
        let passphrase_set_success = account
            .open("foobar".to_string())
            .await
            .context("failed to set passphrase")?;
        assert!(passphrase_set_success);
        drop(accounts);

        let accounts = Accounts::new(p.clone())
            .await
            .context("failed to create second accounts manager")?;
        let account = accounts
            .get_selected_account()
            .context("failed to get account")?;
        assert_eq!(account.is_open().await, false);

        // Try wrong passphrase.
        assert_eq!(account.open("barfoo".to_string()).await?, false);
        assert_eq!(account.open("".to_string()).await?, false);

        assert_eq!(account.open("foobar".to_string()).await?, true);
        assert_eq!(account.is_open().await, true);

        Ok(())
    }

    /// Tests that accounts share stock string translations.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_accounts_share_translations() -> Result<()> {
        let dir = tempfile::tempdir().unwrap();
        let p: PathBuf = dir.path().join("accounts");

        let mut accounts = Accounts::new(p.clone()).await?;
        accounts.add_account().await?;
        accounts.add_account().await?;

        let account1 = accounts.get_account(1).context("failed to get account 1")?;
        let account2 = accounts.get_account(2).context("failed to get account 2")?;

        assert_eq!(stock_str::no_messages(&account1).await, "No messages.");
        assert_eq!(stock_str::no_messages(&account2).await, "No messages.");
        account1
            .set_stock_translation(StockMessage::NoMessages, "foobar".to_string())
            .await?;
        assert_eq!(stock_str::no_messages(&account1).await, "foobar");
        assert_eq!(stock_str::no_messages(&account2).await, "foobar");

        Ok(())
    }
}
