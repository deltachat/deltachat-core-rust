//! Utilities to help writing tests.
//!
//! This module is only compiled for test runs.

use tempfile::{tempdir, TempDir};

use crate::context::{dc_context_new, dc_open, Context};
use crate::types::dc_callback_t;

use crate::dc_tools::OsStrExt;

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
        let mut ctx = dc_context_new(cb, std::ptr::null_mut(), std::ptr::null_mut());
        let dir = tempdir().unwrap();
        let dbfile = dir.path().join("db.sqlite");
        let dbfile_c = dbfile.to_c_string().unwrap();
        assert_eq!(
            dc_open(&mut ctx, dbfile_c.as_ptr(), std::ptr::null()),
            1,
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
