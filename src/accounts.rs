//! # Account manager module.

use std::collections::BTreeMap;

use async_std::channel::{self, Receiver, Sender};
use async_std::fs;
use async_std::path::PathBuf;
use async_std::prelude::*;
use async_std::sync::{Arc, RwLock};
use uuid::Uuid;

use anyhow::{ensure, Context as _, Result};
use serde::{Deserialize, Serialize};

use crate::context::Context;
use crate::events::Event;

/// Account manager, that can handle multiple accounts in a single place.
#[derive(Debug)]
pub struct Accounts {
    dir: PathBuf,
    config: Config,
    accounts: BTreeMap<u32, Context>,
    emitter: EventEmitter,

    /// Sender side of the fake event channel.
    ///
    /// We never send any events over this channel, but hold it during the account manager lifetime
    /// to prevent `EventEmitter` from returning `None` as long as account manager is alive, even if
    /// it holds no accounts which could emit events.
    fake_sender: Sender<crate::events::Event>,
}

impl Accounts {
    /// Loads or creates an accounts folder at the given `dir`.
    pub async fn new(os_name: String, dir: PathBuf) -> Result<Self> {
        if !dir.exists().await {
            Accounts::create(os_name, &dir).await?;
        }

        Accounts::open(dir).await
    }

    /// Creates a new default structure.
    pub async fn create(os_name: String, dir: &PathBuf) -> Result<()> {
        fs::create_dir_all(dir)
            .await
            .context("failed to create folder")?;

        Config::new(os_name.clone(), dir).await?;

        Ok(())
    }

    /// Opens an existing accounts structure. Will error if the folder doesn't exist,
    /// no account exists and no config exists.
    pub async fn open(dir: PathBuf) -> Result<Self> {
        ensure!(dir.exists().await, "directory does not exist");

        let config_file = dir.join(CONFIG_NAME);
        ensure!(config_file.exists().await, "accounts.toml does not exist");

        let config = Config::from_file(config_file).await?;
        let accounts = config.load_accounts().await?;

        let emitter = EventEmitter::new();

        // Fake event stream to prevent event emitter from closing.
        let (fake_sender, fake_receiver) = channel::bounded(1);
        emitter.sender.send(fake_receiver).await?;

        for account in accounts.values() {
            emitter.add_account(account).await?;
        }

        Ok(Self {
            dir,
            config,
            accounts,
            emitter,
            fake_sender,
        })
    }

    /// Get an account by its `id`:
    pub async fn get_account(&self, id: u32) -> Option<Context> {
        self.accounts.get(&id).cloned()
    }

    /// Get the currently selected account.
    pub async fn get_selected_account(&self) -> Option<Context> {
        let id = self.config.get_selected_account().await;
        self.accounts.get(&id).cloned()
    }

    /// Returns the currently selected account's id or None if no account is selected.
    pub async fn get_selected_account_id(&self) -> Option<u32> {
        match self.config.get_selected_account().await {
            0 => None,
            id => Some(id),
        }
    }

    /// Select the given account.
    pub async fn select_account(&mut self, id: u32) -> Result<()> {
        self.config.select_account(id).await?;

        Ok(())
    }

    /// Add a new account.
    pub async fn add_account(&mut self) -> Result<u32> {
        let os_name = self.config.os_name().await;
        let account_config = self.config.new_account(&self.dir).await?;

        let ctx = Context::new(os_name, account_config.dbfile().into(), account_config.id).await?;
        self.emitter.add_account(&ctx).await?;
        self.accounts.insert(account_config.id, ctx);

        Ok(account_config.id)
    }

    /// Remove an account.
    pub async fn remove_account(&mut self, id: u32) -> Result<()> {
        let ctx = self.accounts.remove(&id);
        ensure!(ctx.is_some(), "no account with this id: {}", id);
        let ctx = ctx.unwrap();
        ctx.stop_io().await;
        drop(ctx);

        if let Some(cfg) = self.config.get_account(id).await {
            fs::remove_dir_all(async_std::path::PathBuf::from(&cfg.dir))
                .await
                .context("failed to remove account data")?;
        }
        self.config.remove_account(id).await?;

        Ok(())
    }

    /// Migrate an existing account into this structure.
    pub async fn migrate_account(&mut self, dbfile: PathBuf) -> Result<u32> {
        let blobdir = Context::derive_blobdir(&dbfile);
        let walfile = Context::derive_walfile(&dbfile);

        ensure!(
            dbfile.exists().await,
            "no database found: {}",
            dbfile.display()
        );
        ensure!(
            blobdir.exists().await,
            "no blobdir found: {}",
            blobdir.display()
        );

        let old_id = self.config.get_selected_account().await;

        // create new account
        let account_config = self
            .config
            .new_account(&self.dir)
            .await
            .context("failed to create new account")?;

        let new_dbfile = account_config.dbfile().into();
        let new_blobdir = Context::derive_blobdir(&new_dbfile);
        let new_walfile = Context::derive_walfile(&new_dbfile);

        let res = {
            fs::create_dir_all(&account_config.dir)
                .await
                .context("failed to create dir")?;
            fs::rename(&dbfile, &new_dbfile)
                .await
                .context("failed to rename dbfile")?;
            fs::rename(&blobdir, &new_blobdir)
                .await
                .context("failed to rename blobdir")?;
            if walfile.exists().await {
                fs::rename(&walfile, &new_walfile)
                    .await
                    .context("failed to rename walfile")?;
            }
            Ok(())
        };

        match res {
            Ok(_) => {
                let ctx = Context::with_blobdir(
                    self.config.os_name().await,
                    new_dbfile,
                    new_blobdir,
                    account_config.id,
                )
                .await?;
                self.emitter.add_account(&ctx).await?;
                self.accounts.insert(account_config.id, ctx);
                Ok(account_config.id)
            }
            Err(err) => {
                // remove temp account
                fs::remove_dir_all(async_std::path::PathBuf::from(&account_config.dir))
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
    pub async fn get_all(&self) -> Vec<u32> {
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

    pub async fn start_io(&self) {
        for account in self.accounts.values() {
            account.start_io().await;
        }
    }

    pub async fn stop_io(&self) {
        for account in self.accounts.values() {
            account.stop_io().await;
        }
    }

    pub async fn maybe_network(&self) {
        for account in self.accounts.values() {
            account.maybe_network().await;
        }
    }

    pub async fn maybe_network_lost(&self) {
        for account in self.accounts.values() {
            account.maybe_network_lost().await;
        }
    }

    /// Returns unified event emitter.
    pub async fn get_event_emitter(&self) -> EventEmitter {
        self.emitter.clone()
    }
}

/// Unified event emitter for multiple accounts.
#[derive(Debug, Clone)]
pub struct EventEmitter {
    /// Aggregate stream of events from all accounts.
    stream: Arc<RwLock<futures::stream::SelectAll<Receiver<crate::events::Event>>>>,

    /// Sender for the channel where new account emitters will be pushed.
    sender: Sender<Receiver<crate::events::Event>>,

    /// Receiver for the channel where new account emitters will be pushed.
    receiver: Receiver<Receiver<crate::events::Event>>,
}

impl EventEmitter {
    pub fn new() -> Self {
        let (sender, receiver) = channel::unbounded();
        Self {
            stream: Arc::new(RwLock::new(futures::stream::SelectAll::new())),
            sender,
            receiver,
        }
    }

    /// Blocking recv of an event. Return `None` if all `Sender`s have been droped.
    pub fn recv_sync(&mut self) -> Option<Event> {
        async_std::task::block_on(self.recv()).unwrap_or_default()
    }

    /// Async recv of an event. Return `None` if all `Sender`s have been dropped.
    pub async fn recv(&mut self) -> Result<Option<Event>> {
        let mut stream = self.stream.write().await;
        loop {
            match futures::future::select(self.receiver.recv(), stream.next()).await {
                futures::future::Either::Left((emitter, _)) => {
                    stream.push(emitter?);
                }
                futures::future::Either::Right((ev, _)) => return Ok(ev),
            }
        }
    }

    /// Add event emitter of a new account to the aggregate event emitter.
    pub async fn add_account(&self, context: &Context) -> Result<()> {
        self.sender
            .send(context.get_event_emitter().into_inner())
            .await?;
        Ok(())
    }
}

impl Default for EventEmitter {
    fn default() -> Self {
        Self::new()
    }
}

impl async_std::stream::Stream for EventEmitter {
    type Item = Event;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        std::pin::Pin::new(&mut self).poll_next(cx)
    }
}

pub const CONFIG_NAME: &str = "accounts.toml";
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
    pub os_name: String,
    /// The currently selected account.
    pub selected_account: u32,
    pub next_id: u32,
    pub accounts: Vec<AccountConfig>,
}

impl Config {
    pub async fn new(os_name: String, dir: &PathBuf) -> Result<Self> {
        let inner = InnerConfig {
            os_name,
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

    pub async fn os_name(&self) -> String {
        self.inner.os_name.clone()
    }

    /// Sync the inmemory representation to disk.
    async fn sync(&self) -> Result<()> {
        fs::write(&self.file, toml::to_string_pretty(&self.inner)?)
            .await
            .context("failed to write config")
    }

    /// Read a configuration from the given file into memory.
    pub async fn from_file(file: PathBuf) -> Result<Self> {
        let bytes = fs::read(&file).await.context("failed to read file")?;
        let inner: InnerConfig = toml::from_slice(&bytes).context("failed to parse config")?;

        Ok(Config { file, inner })
    }

    pub async fn load_accounts(&self) -> Result<BTreeMap<u32, Context>> {
        let mut accounts = BTreeMap::new();
        for account_config in &self.inner.accounts {
            let ctx = Context::new(
                self.inner.os_name.clone(),
                account_config.dbfile().into(),
                account_config.id,
            )
            .await?;
            accounts.insert(account_config.id, ctx);
        }

        Ok(accounts)
    }

    /// Create a new account in the given root directory.
    async fn new_account(&mut self, dir: &PathBuf) -> Result<AccountConfig> {
        let id = {
            let id = self.inner.next_id;
            let uuid = Uuid::new_v4();
            let target_dir = dir.join(uuid.to_simple_ref().to_string());

            self.inner.accounts.push(AccountConfig {
                id,
                dir: target_dir.into(),
                uuid,
            });
            self.inner.next_id += 1;
            id
        };

        self.sync().await?;

        self.select_account(id).await.expect("just added");
        let cfg = self.get_account(id).await.expect("just added");
        Ok(cfg)
    }

    /// Removes an existing acccount entirely.
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

    async fn get_account(&self, id: u32) -> Option<AccountConfig> {
        self.inner.accounts.iter().find(|e| e.id == id).cloned()
    }

    pub async fn get_selected_account(&self) -> u32 {
        self.inner.selected_account
    }

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
    pub dir: std::path::PathBuf,
    pub uuid: Uuid,
}

impl AccountConfig {
    /// Get the canoncial dbfile name for this configuration.
    pub fn dbfile(&self) -> std::path::PathBuf {
        self.dir.join(DB_NAME)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[async_std::test]
    async fn test_account_new_open() {
        let dir = tempfile::tempdir().unwrap();
        let p: PathBuf = dir.path().join("accounts1").into();

        let mut accounts1 = Accounts::new("my_os".into(), p.clone()).await.unwrap();
        accounts1.add_account().await.unwrap();

        let accounts2 = Accounts::open(p).await.unwrap();

        assert_eq!(accounts1.accounts.len(), 1);
        assert_eq!(accounts1.config.get_selected_account().await, 1);

        assert_eq!(accounts1.dir, accounts2.dir);
        assert_eq!(accounts1.config, accounts2.config,);
        assert_eq!(accounts1.accounts.len(), accounts2.accounts.len());
    }

    #[async_std::test]
    async fn test_account_new_add_remove() {
        let dir = tempfile::tempdir().unwrap();
        let p: PathBuf = dir.path().join("accounts").into();

        let mut accounts = Accounts::new("my_os".into(), p.clone()).await.unwrap();
        assert_eq!(accounts.accounts.len(), 0);
        assert_eq!(accounts.config.get_selected_account().await, 0);

        let id = accounts.add_account().await.unwrap();
        assert_eq!(id, 1);
        assert_eq!(accounts.accounts.len(), 1);
        assert_eq!(accounts.config.get_selected_account().await, 1);

        let id = accounts.add_account().await.unwrap();
        assert_eq!(id, 2);
        assert_eq!(accounts.config.get_selected_account().await, id);
        assert_eq!(accounts.accounts.len(), 2);

        accounts.select_account(1).await.unwrap();
        assert_eq!(accounts.config.get_selected_account().await, 1);

        accounts.remove_account(1).await.unwrap();
        assert_eq!(accounts.config.get_selected_account().await, 2);
        assert_eq!(accounts.accounts.len(), 1);
    }

    #[async_std::test]
    async fn test_accounts_remove_last() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let p: PathBuf = dir.path().join("accounts").into();

        let mut accounts = Accounts::new("my_os".into(), p.clone()).await?;
        assert!(accounts.get_selected_account().await.is_none());
        assert_eq!(accounts.config.get_selected_account().await, 0);

        let id = accounts.add_account().await?;
        assert!(accounts.get_selected_account().await.is_some());
        assert_eq!(id, 1);
        assert_eq!(accounts.accounts.len(), 1);
        assert_eq!(accounts.config.get_selected_account().await, id);

        accounts.remove_account(id).await?;
        assert!(accounts.get_selected_account().await.is_none());

        Ok(())
    }

    #[async_std::test]
    async fn test_migrate_account() {
        let dir = tempfile::tempdir().unwrap();
        let p: PathBuf = dir.path().join("accounts").into();

        let mut accounts = Accounts::new("my_os".into(), p.clone()).await.unwrap();
        assert_eq!(accounts.accounts.len(), 0);
        assert_eq!(accounts.config.get_selected_account().await, 0);

        let extern_dbfile: PathBuf = dir.path().join("other").into();
        let ctx = Context::new("my_os".into(), extern_dbfile.clone(), 0)
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
        assert_eq!(accounts.config.get_selected_account().await, 1);

        let ctx = accounts.get_selected_account().await.unwrap();
        assert_eq!(
            "me@mail.com",
            ctx.get_config(crate::config::Config::Addr)
                .await
                .unwrap()
                .unwrap()
        );
    }

    /// Tests that accounts are sorted by ID.
    #[async_std::test]
    async fn test_accounts_sorted() {
        let dir = tempfile::tempdir().unwrap();
        let p: PathBuf = dir.path().join("accounts").into();

        let mut accounts = Accounts::new("my_os".into(), p.clone()).await.unwrap();

        for expected_id in 1..10 {
            let id = accounts.add_account().await.unwrap();
            assert_eq!(id, expected_id);
        }

        let ids = accounts.get_all().await;
        for (i, expected_id) in (1..10).enumerate() {
            assert_eq!(ids.get(i), Some(&expected_id));
        }
    }

    #[async_std::test]
    async fn test_accounts_ids_unique_increasing_and_persisted() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let p: PathBuf = dir.path().join("accounts").into();
        let dummy_accounts = 10;

        let (id0, id1, id2) = {
            let mut accounts = Accounts::new("my_os".into(), p.clone()).await?;
            accounts.add_account().await?;
            let ids = accounts.get_all().await;
            assert_eq!(ids.len(), 1);

            let id0 = *ids.get(0).unwrap();
            let ctx = accounts.get_account(id0).await.unwrap();
            ctx.set_config(crate::config::Config::Addr, Some("one@example.org"))
                .await?;

            let id1 = accounts.add_account().await?;
            let ctx = accounts.get_account(id1).await.unwrap();
            ctx.set_config(crate::config::Config::Addr, Some("two@example.org"))
                .await?;

            // add and remove some accounts and force a gap (ids must not be reused)
            for _ in 0..dummy_accounts {
                let to_delete = accounts.add_account().await?;
                accounts.remove_account(to_delete).await?;
            }

            let id2 = accounts.add_account().await?;
            let ctx = accounts.get_account(id2).await.unwrap();
            ctx.set_config(crate::config::Config::Addr, Some("three@example.org"))
                .await?;

            accounts.select_account(id1).await?;

            (id0, id1, id2)
        };
        assert!(id0 > 0);
        assert!(id1 > id0);
        assert!(id2 > id1 + dummy_accounts);

        let (id0_reopened, id1_reopened, id2_reopened) = {
            let accounts = Accounts::new("my_os".into(), p.clone()).await?;
            let ctx = accounts.get_selected_account().await.unwrap();
            assert_eq!(
                ctx.get_config(crate::config::Config::Addr).await?,
                Some("two@example.org".to_string())
            );

            let ids = accounts.get_all().await;
            assert_eq!(ids.len(), 3);

            let id0 = *ids.get(0).unwrap();
            let ctx = accounts.get_account(id0).await.unwrap();
            assert_eq!(
                ctx.get_config(crate::config::Config::Addr).await?,
                Some("one@example.org".to_string())
            );

            let id1 = *ids.get(1).unwrap();
            let t = accounts.get_account(id1).await.unwrap();
            assert_eq!(
                t.get_config(crate::config::Config::Addr).await?,
                Some("two@example.org".to_string())
            );

            let id2 = *ids.get(2).unwrap();
            let ctx = accounts.get_account(id2).await.unwrap();
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

    #[async_std::test]
    async fn test_no_accounts_event_emitter() -> Result<()> {
        let dir = tempfile::tempdir().unwrap();
        let p: PathBuf = dir.path().join("accounts").into();

        let accounts = Accounts::new("my_os".into(), p.clone()).await?;

        // Make sure there are no accounts.
        assert_eq!(accounts.accounts.len(), 0);

        // Create event emitter.
        let mut event_emitter = accounts.get_event_emitter().await;

        // Test that event emitter does not return `None` immediately.
        let duration = std::time::Duration::from_millis(1);
        assert!(async_std::future::timeout(duration, event_emitter.recv())
            .await
            .is_err());

        // When account manager is dropped, event emitter is exhausted.
        drop(accounts);
        assert_eq!(event_emitter.recv().await?, None);

        Ok(())
    }
}
