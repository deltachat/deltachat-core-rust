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

/// Create a new, opened [TestContext] using given callback.
///
/// The [Context] will be opened with the SQLite database named
/// "db.sqlite" in the [TestContext.dir] directory.
///
/// [Context]: crate::context::Context
pub(crate) async fn test_context() -> TestContext {
    let dir = tempdir().unwrap();
    let dbfile = dir.path().join("db.sqlite");
    let ctx = Context::new("FakeOs".into(), dbfile.into()).await.unwrap();
    TestContext { ctx, dir }
}

/// Return a dummy [TestContext].
///
/// The context will be opened and use the SQLite database as
/// specified in [test_context] but there is no callback hooked up,
/// i.e. [Context::call_cb] will always return `0`.
pub(crate) async fn dummy_context() -> TestContext {
    test_context().await
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

/// Creates Alice with a pre-generated keypair.
///
/// Returns the address of the keypair created (alice@example.com).
pub(crate) async fn configure_alice_keypair(ctx: &Context) -> String {
    let keypair = alice_keypair();
    ctx.set_config(Config::ConfiguredAddr, Some(&keypair.addr.to_string()))
        .await
        .unwrap();
    key::store_self_keypair(&ctx, &keypair, key::KeyPairUse::Default)
        .await
        .expect("Failed to save Alice's key");
    keypair.addr.to_string()
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
