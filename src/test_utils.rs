//! Utilities to help writing tests.
//!
//! This module is only compiled for test runs.

use tempfile::{tempdir, TempDir};

use crate::config::Config;
use crate::context::Context;
use crate::dc_tools::EmailAddress;
use crate::key::{self, DcKey};

/// A Context and temporary directory.
///
/// The temporary directory can be used to store the SQLite database,
/// see e.g. [test_context] which does this.
pub(crate) struct TestContext {
    pub ctx: Context,
    pub dir: TempDir,
}

impl TestContext {
    /// Create a new [TestContext].
    ///
    /// The [Context] will be created and have an SQLite database named "db.sqlite" in the
    /// [TestContext.dir] directory.  This directory is cleaned up when the [TestContext] is
    /// dropped.
    ///
    /// [Context]: crate::context::Context
    pub async fn new() -> Self {
        use rand::Rng;

        let dir = tempdir().unwrap();
        let dbfile = dir.path().join("db.sqlite");
        let id = rand::thread_rng().gen();
        let ctx = Context::new("FakeOS".into(), dbfile.into(), id)
            .await
            .unwrap();
        Self { ctx, dir }
    }

    /// Create a new configured [TestContext].
    ///
    /// This is a shortcut which automatically calls [TestContext::configure_alice] after
    /// creating the context.
    pub async fn new_alice() -> Self {
        let t = Self::new().await;
        t.configure_alice().await;
        t
    }

    /// Configure with alice@example.com.
    ///
    /// The context will be fake-configured as the alice user, with a pre-generated secret
    /// key.  The email address of the user is returned as a string.
    pub async fn configure_alice(&self) -> String {
        let keypair = alice_keypair();
        self.configure_addr(&keypair.addr.to_string()).await;
        key::store_self_keypair(&self.ctx, &keypair, key::KeyPairUse::Default)
            .await
            .expect("Failed to save Alice's key");
        keypair.addr.to_string()
    }

    /// Configure as a given email address.
    ///
    /// The context will be configured but the key will not be pre-generated so if a key is
    /// used the fingerprint will be different every time.
    pub async fn configure_addr(&self, addr: &str) {
        self.ctx.set_config(Config::Addr, Some(addr)).await.unwrap();
        self.ctx
            .set_config(Config::ConfiguredAddr, Some(addr))
            .await
            .unwrap();
        self.ctx
            .set_config(Config::Configured, Some("1"))
            .await
            .unwrap();
    }
}

/// Load a pre-generated keypair for alice@example.com from disk.
///
/// This saves CPU cycles by avoiding having to generate a key.
///
/// The keypair was created using the crate::key::tests::gen_key test.
pub(crate) fn alice_keypair() -> key::KeyPair {
    let addr = EmailAddress::new("alice@example.com").unwrap();
    let public =
        key::SignedPublicKey::from_base64(include_str!("../test-data/key/alice-public.asc"))
            .unwrap();
    let secret =
        key::SignedSecretKey::from_base64(include_str!("../test-data/key/alice-secret.asc"))
            .unwrap();
    key::KeyPair {
        addr,
        public,
        secret,
    }
}

/// Load a pre-generated keypair for bob@example.net from disk.
///
/// Like [alice_keypair] but a different key and identity.
pub(crate) fn bob_keypair() -> key::KeyPair {
    let addr = EmailAddress::new("bob@example.net").unwrap();
    let public =
        key::SignedPublicKey::from_base64(include_str!("../test-data/key/bob-public.asc")).unwrap();
    let secret =
        key::SignedSecretKey::from_base64(include_str!("../test-data/key/bob-secret.asc")).unwrap();
    key::KeyPair {
        addr,
        public,
        secret,
    }
}
