//! Utilities to help writing tests.
//!
//! This module is only compiled for test runs.

use std::ffi::CStr;

use libc::uintptr_t;
use tempfile::{tempdir, TempDir};

use crate::config::Config;
use crate::constants::{Event, KeyType};
use crate::context::{dc_context_new, dc_open, Context};
use crate::key;
use crate::types::dc_callback_t;

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
pub fn test_context(cb: Option<dc_callback_t>) -> TestContext {
    unsafe {
        let mut ctx = dc_context_new(cb, std::ptr::null_mut(), None);
        let dir = tempdir().unwrap();
        let dbfile = dir.path().join("db.sqlite");
        assert!(
            dc_open(&mut ctx, dbfile.to_str().unwrap(), None),
            "Failed to open {}",
            dbfile.display(),
        );
        TestContext { ctx: ctx, dir: dir }
    }
}

/// Return a dummy [TestContext].
///
/// The context will be opened and use the SQLite database as
/// specified in [test_context] but there is no callback hooked up,
/// i.e. [Context::call_cb] will always return `0`.
pub fn dummy_context() -> TestContext {
    test_context(None)
}

pub unsafe extern "C" fn logging_cb(
    _ctx: &Context,
    evt: Event,
    _d1: uintptr_t,
    d2: uintptr_t,
) -> uintptr_t {
    let to_str = |x| CStr::from_ptr(x as *const libc::c_char).to_str().unwrap();
    match evt {
        Event::INFO => println!("I: {}", to_str(d2)),
        Event::WARNING => println!("W: {}", to_str(d2)),
        Event::ERROR => println!("E: {}", to_str(d2)),
        _ => (),
    }
    0
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
    let public = key::Key::from_base64(
        concat!(
            "xsBNBF086ewBCACmJKuoxIO6T87yi4Q3MyNpMch3Y8KrtHDQyUszU36eqM3Pmd1l",
            "FrbcCd8ZWo2pq6OJSwsM/jjRGn1zo2YOaQeJRRrC+KrKGqSUtRSYQBPrPjE2YjSX",
            "AMbu8jBI6VVUhHeghriBkK79PY9O/oRhIJC0p14IJe6CQ6OI2fTmTUHF9i/nJ3G4",
            "Wb3/K1bU3yVfyPZjTZQPYPvvh03vxHULKurtYkX5DTEMSWsF4qzLMps+l87VuLV9",
            "iQnbN7YMRLHHx2KkX5Ivi7JCefhCa54M0K3bDCCxuVWAM5wjQwNZjzR+WL+dYchw",
            "oFvuF8NrlzjM9dSv+2rn+J7f99ijSXarzPC7ABEBAAHNEzxhbGljZUBleGFtcGxl",
            "LmNvbT7CwIkEEAEIADMCGQEFAl086fgCGwMECwkIBwYVCAkKCwIDFgIBFiEE+iai",
            "x4d0doj87Q0ek6DcNkbrcegACgkQk6DcNkbrcei3ogf/cruUmQ+th52TFHTHdkw9",
            "OHUl3MrXtZ7QmHyOAFvbXE/6n5Eeh+eZoF8MWWV72m14Wbs+vTcNQkFVTdOPptkK",
            "A8e4cJqwDOHsyAnvQXZ7WNje9+BMzcoipIUawHP4ORFaPDsKLZQ0b4wBbKn8ziea",
            "6zjGF0/qljTdoxTtsYpv5wXYuhwbYklrLOqgSa5M7LXUe7E3g9mbg+9iX1GuB8m6",
            "GkquJN814Y+xny4xhZzGOfue6SeP12jJMNSjSP7416dRq7794VGnkkW9V/7oFEUK",
            "u5wO9FFbgDySOSlEjByGejSGuBmho0iJSjcPjZ7EY/j3M3orq4dpza5C82OeSvxD",
            "Fc7ATQRdPOnsAQgA5oLxXRLnyugzOmNCy7dxV3JrDZscA6JNlJlDWIShT0YSs+zG",
            "9JzDeQql+sYXgUSxOoIayItuXtnFn7tstwGoOnYvadm/e5/7V5fKAQRtCtdN51av",
            "62n18Venlm0yNKpROPcZ6M/sc4m6uU6YRZ/a1idal8VGY0wLKlghjIBuIiBoVQ/R",
            "noW+/fhmwIg08dQ5m8hQe3GEOZEeLrTWL/9awPlXK7Y+DmJOoR4qbHWEcRfbzS6q",
            "4zW8vk2ztB8ngwbnqYy8zrN1DCICN1gYdlU++uVw6Bb1XfY8Cdldh1VLKpF35mAm",
            "jxLZfVDcoObFH3Cv2GB7BEYxv86KC2Y6T74Q/wARAQABwsB2BBgBCAAgBQJdPOn4",
            "AhsMFiEE+iaix4d0doj87Q0ek6DcNkbrcegACgkQk6DcNkbrcegLxwf/dXshJnoW",
            "qEnRsf6rVb9/Mc66ti+NVQLfNd275dybh/QIJdK3NmSxdnTPIEJRVspJywJoupwX",
            "FNrnHG2Ff6QPvHqI+/oNu986r9d7Z1oQibbLHKt8t6kpOfg/xGxotagJuiCQvR9m",
            "MjD1DqsdB37SjDxGupZOOJSXWi6KX60IE+uM+QOBfeOZziQwuFmA5wV6RDXIeYJf",
            "qrcbeXeR1d0nfNpPHQR1gBiqmxNb6KBbdXD2+EXW60axC7D2b1APmzMlammDliPw",
            "sK9U1rc9nuquEBvGDOJf4K+Dzn+mDCqRpP6uAuQ7RKHyim4uyN0wwKObzPqgJCGw",
            "jTglkixw+aSTXw=="
        ),
        KeyType::Public,
    )
    .unwrap();
    let private = key::Key::from_base64(
        concat!(
            "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",
            "zPqgJCGwjTglkixw+aSTXw=="
        ),
        KeyType::Private,
    )
    .unwrap();
    let saved = key::dc_key_save_self_keypair(&ctx, &public, &private, &addr, 1, &ctx.sql);
    assert_eq!(saved, true, "Failed to save Alice's key");
    addr
}
