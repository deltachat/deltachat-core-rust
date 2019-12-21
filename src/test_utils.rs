//! Utilities to help writing tests.
//!
//! This module is only compiled for test runs.

use tempfile::{tempdir, TempDir};

use crate::config::Config;
use crate::constants::KeyType;
use crate::context::{Context, ContextCallback};
use crate::events::Event;
use crate::key;

/// A Context and temporary directory.
///
/// The temporary directory can be used to store the SQLite database,
/// see e.g. [test_context] which does this.
pub struct TestContext {
    pub ctx: Context,
    pub dir: TempDir,
}

/// Create a new, opened [TestContext] using given callback.
///
/// The [Context] will be opened with the SQLite database named
/// "db.sqlite" in the [TestContext.dir] directory.
///
/// [Context]: crate::context::Context
pub fn test_context(callback: Option<Box<ContextCallback>>) -> TestContext {
    let dir = tempdir().unwrap();
    let dbfile = dir.path().join("db.sqlite");
    let cb: Box<ContextCallback> = match callback {
        Some(cb) => cb,
        None => Box::new(|_, _| ()),
    };
    let ctx = Context::new(cb, "FakeOs".into(), dbfile).unwrap();
    TestContext { ctx, dir }
}

/// Return a dummy [TestContext].
///
/// The context will be opened and use the SQLite database as
/// specified in [test_context] but there is no callback hooked up,
/// i.e. [Context::call_cb] will always return `0`.
pub fn dummy_context() -> TestContext {
    test_context(None)
}

pub fn logging_cb(_ctx: &Context, evt: Event) {
    match evt {
        Event::Info(msg) => println!("I: {}", msg),
        Event::Warning(msg) => println!("W: {}", msg),
        Event::Error(msg) => println!("E: {}", msg),
        _ => (),
    }
}

/// Creates Alice with a pre-generated keypair.
///
/// Returns the address of the keypair created (alice@example.org).
pub fn configure_alice_keypair(ctx: &Context) -> String {
    let addr = String::from("alice@example.org");
    ctx.set_config(Config::ConfiguredAddr, Some(&addr)).unwrap();

    // The keypair was created using:
    //   let (public, private) = crate::pgp::dc_pgp_create_keypair("alice@example.com")
    //       .unwrap();
    //   println!("{}", public.to_base64(64));
    //   println!("{}", private.to_base64(64));
    let public =
        key::Key::from_base64(include_str!("../test-data/key/public.asc"), KeyType::Public)
            .unwrap();
    let private = key::Key::from_base64(
        include_str!("../test-data/key/private.asc"),
        KeyType::Private,
    )
    .unwrap();
    let saved = key::dc_key_save_self_keypair(&ctx, &public, &private, &addr, true, &ctx.sql);
    assert_eq!(saved, true, "Failed to save Alice's key");
    addr
}
