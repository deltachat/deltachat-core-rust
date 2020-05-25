//! # SQLite wrapper

use std::collections::HashSet;
use std::path::Path;

use async_std::prelude::*;
use async_std::sync::RwLock;
use sqlx::sqlite::*;

use crate::chat::{update_device_icon, update_saved_messages_icon};
use crate::constants::DC_CHAT_ID_TRASH;
use crate::context::Context;
use crate::dc_tools::*;
use crate::param::*;
use crate::peerstate::*;

#[macro_use]
mod macros;
mod migrations;

pub use macros::*;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Sqlite: Connection closed")]
    SqlNoConnection,
    #[error("Sqlite: Already open")]
    SqlAlreadyOpen,
    #[error("Sqlite: Failed to open")]
    SqlFailedToOpen,
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("{0:?}")]
    BlobError(#[from] crate::blob::BlobError),
    #[error("{0}")]
    Other(#[from] crate::error::Error),
    #[error("{0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("{0}: {1}")]
    SqlxWithContext(String, #[source] sqlx::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

/// A wrapper around the underlying Sqlite3 object.
#[derive(Debug)]
pub struct Sql {
    xpool: RwLock<Option<sqlx::SqlitePool>>,
}

impl Default for Sql {
    fn default() -> Self {
        Self {
            xpool: RwLock::new(None),
        }
    }
}

impl Sql {
    pub fn new() -> Sql {
        Self::default()
    }

    /// Returns `true` if there is a working sqlite connection, `false` otherwise.
    pub async fn is_open(&self) -> bool {
        let pool = self.xpool.read().await;
        pool.is_some() && !pool.as_ref().unwrap().is_closed()
    }

    /// Shuts down all sqlite connections.
    pub async fn close(&self) {
        if let Some(pool) = self.xpool.write().await.take() {
            pool.close().await;
        }
    }

    pub async fn open<T: AsRef<Path>>(
        &self,
        context: &Context,
        dbfile: T,
        readonly: bool,
    ) -> crate::error::Result<()> {
        if let Err(err) = open(context, self, dbfile, readonly).await {
            return match err.downcast_ref::<Error>() {
                Some(Error::SqlAlreadyOpen) => Err(err),
                _ => {
                    self.close().await;
                    Err(err)
                }
            };
        }

        Ok(())
    }

    /// Execute a single query.
    pub async fn execute<'a, P>(&self, statement: &'a str, params: P) -> Result<usize>
    where
        P: sqlx::IntoArguments<'a, Sqlite> + 'a,
    {
        let lock = self.xpool.read().await;
        let xpool = lock.as_ref().ok_or_else(|| Error::SqlNoConnection)?;

        let count = sqlx::query_with(statement, params).execute(xpool).await?;

        Ok(count as usize)
    }

    /// Execute a list of statements, without any bindings
    pub async fn execute_batch(&self, statement: &str) -> Result<()> {
        let lock = self.xpool.read().await;
        let xpool = lock.as_ref().ok_or_else(|| Error::SqlNoConnection)?;

        sqlx::query(statement.as_ref()).execute(xpool).await?;

        Ok(())
    }

    pub async fn get_pool(&self) -> Result<SqlitePool> {
        let lock = self.xpool.read().await;
        lock.as_ref().cloned().ok_or_else(|| Error::SqlNoConnection)
    }

    /// Starts a new transaction.
    pub async fn begin(
        &self,
    ) -> Result<sqlx::Transaction<'static, Sqlite, sqlx::pool::PoolConnection<Sqlite>>> {
        let lock = self.xpool.read().await;
        let pool = lock.as_ref().ok_or_else(|| Error::SqlNoConnection)?;

        let tx = pool.begin().await?;
        Ok(tx)
    }

    /// Execute a query which is expected to return zero or more rows.
    pub async fn query_rows<'a, T, P>(&self, statement: &'a str, params: P) -> Result<Vec<T>>
    where
        P: sqlx::IntoArguments<'a, Sqlite> + 'a,
        T: for<'b> sqlx::FromRow<'b, SqliteRow> + Unpin + Send,
    {
        let lock = self.xpool.read().await;
        let xpool = lock.as_ref().ok_or_else(|| Error::SqlNoConnection)?;
        let rows = sqlx::query_with(statement.as_ref(), params)
            .try_map(|row: SqliteRow| sqlx::FromRow::from_row(&row))
            .fetch_all(xpool)
            .await?;

        Ok(rows)
    }

    /// Execute a query which is expected to return zero or more rows.
    pub async fn query_values<'a, T, P>(&self, statement: &'a str, params: P) -> Result<Vec<T>>
    where
        P: sqlx::IntoArguments<'a, Sqlite> + 'a,
        T: for<'b> sqlx::decode::Decode<'b, Sqlite>,
        T: sqlx::Type<Sqlite>,
        T: 'static + Unpin + Send,
    {
        let lock = self.xpool.read().await;
        let xpool = lock.as_ref().ok_or_else(|| Error::SqlNoConnection)?;
        let rows = sqlx::query_with(statement.as_ref(), params)
            .try_map(|row: SqliteRow| {
                let (val,): (T,) = sqlx::FromRow::from_row(&row)?;
                Ok(val)
            })
            .fetch_all(xpool)
            .await?;

        Ok(rows)
    }

    /// Return `true` if a query in the SQL statement it executes returns one or more
    /// rows and false if the SQL returns an empty set.
    pub async fn exists<'a, P>(&self, statement: &'a str, params: P) -> Result<bool>
    where
        P: sqlx::IntoArguments<'a, Sqlite> + 'a,
    {
        let lock = self.xpool.read().await;
        let xpool = lock.as_ref().ok_or_else(|| Error::SqlNoConnection)?;

        let mut rows = sqlx::query_with(statement, params).fetch(xpool);

        match rows.next().await {
            Some(Ok(_)) => Ok(true),
            None => Ok(false),
            Some(Err(sqlx::Error::RowNotFound)) => Ok(false),
            Some(Err(err)) => Err(Error::SqlxWithContext(
                format!("exists: '{}'", statement),
                err,
            )),
        }
    }

    /// Execute a query which is expected to return one row.
    pub async fn query_row<'a, T, P>(&self, statement: &'a str, params: P) -> Result<T>
    where
        P: sqlx::IntoArguments<'a, Sqlite> + 'a,
        T: for<'b> sqlx::FromRow<'b, SqliteRow> + Unpin + Send,
    {
        let lock = self.xpool.read().await;
        let xpool = lock.as_ref().ok_or_else(|| Error::SqlNoConnection)?;
        let row = sqlx::query_with(statement.as_ref(), params)
            .try_map(|row: SqliteRow| sqlx::FromRow::from_row(&row))
            .fetch_one(xpool)
            .await?;

        Ok(row)
    }

    /// Execute a query which is expected to return zero or one row.
    pub async fn query_row_optional<'a, T, P>(
        &self,
        statement: &'a str,
        params: P,
    ) -> Result<Option<T>>
    where
        P: sqlx::IntoArguments<'a, Sqlite> + 'a,
        T: for<'b> sqlx::FromRow<'b, SqliteRow> + Unpin + Send,
    {
        let lock = self.xpool.read().await;
        let xpool = lock.as_ref().ok_or_else(|| Error::SqlNoConnection)?;
        let row = sqlx::query_with(statement.as_ref(), params)
            .try_map(|row: SqliteRow| sqlx::FromRow::from_row(&row))
            .fetch_optional(xpool)
            .await?;

        Ok(row)
    }

    pub async fn query_value_optional<'a, T, P>(
        &self,
        statement: &'a str,
        params: P,
    ) -> Result<Option<T>>
    where
        P: sqlx::IntoArguments<'a, Sqlite> + 'a,
        T: for<'b> sqlx::decode::Decode<'b, Sqlite>,
        T: sqlx::Type<Sqlite>,
        T: 'static + Unpin + Send,
    {
        match self.query_row_optional(statement, params).await? {
            Some((val,)) => Ok(Some(val)),
            None => Ok(None),
        }
    }

    pub async fn query_value<'a, T, P>(&self, statement: &'a str, params: P) -> Result<T>
    where
        P: sqlx::IntoArguments<'a, Sqlite> + 'a,
        T: for<'b> sqlx::decode::Decode<'b, Sqlite>,
        T: sqlx::Type<Sqlite>,
        T: 'static + Unpin + Send,
    {
        let (val,): (T,) = self.query_row(statement, params).await?;
        Ok(val)
    }

    pub async fn table_exists(&self, name: impl AsRef<str>) -> Result<bool> {
        self.exists(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name=?",
            paramsx![name.as_ref()],
        )
        .await
    }

    /// Set private configuration options.
    ///
    /// Setting `None` deletes the value.  On failure an error message
    /// will already have been logged.
    pub async fn set_raw_config(
        &self,
        _context: &Context,
        key: impl AsRef<str>,
        value: Option<&str>,
    ) -> Result<()> {
        let key = key.as_ref();

        if let Some(ref value) = value {
            let exists = self
                .exists("SELECT value FROM config WHERE keyname=?;", paramsx![key])
                .await?;
            if exists {
                self.execute(
                    "UPDATE config SET value=? WHERE keyname=?;",
                    paramsx![value, key],
                )
                .await?;
            } else {
                self.execute(
                    "INSERT INTO config (keyname, value) VALUES (?, ?);",
                    paramsx![key, value],
                )
                .await?;
            }
        } else {
            self.execute("DELETE FROM config WHERE keyname=?;", paramsx![key])
                .await?;
        }

        Ok(())
    }

    /// Get configuration options from the database.
    pub async fn get_raw_config(&self, key: impl AsRef<str>) -> Option<String> {
        if !self.is_open().await || key.as_ref().is_empty() {
            return None;
        }
        self.query_row(
            "SELECT value FROM config WHERE keyname=?;",
            paramsx![key.as_ref().to_string()],
        )
        .await
        .ok()
        .map(|(res,)| res)
    }

    pub async fn set_raw_config_int(
        &self,
        context: &Context,
        key: impl AsRef<str>,
        value: i32,
    ) -> Result<()> {
        self.set_raw_config(context, key, Some(&format!("{}", value)))
            .await
    }

    pub async fn get_raw_config_int(&self, key: impl AsRef<str>) -> Option<i32> {
        self.get_raw_config(key).await.and_then(|s| s.parse().ok())
    }

    pub async fn get_raw_config_bool(&self, key: impl AsRef<str>) -> bool {
        // Not the most obvious way to encode bool as string, but it is matter
        // of backward compatibility.
        let res = self.get_raw_config_int(key).await;
        res.unwrap_or_default() > 0
    }

    pub async fn set_raw_config_bool<T>(&self, context: &Context, key: T, value: bool) -> Result<()>
    where
        T: AsRef<str>,
    {
        let value = if value { Some("1") } else { None };
        self.set_raw_config(context, key, value).await
    }

    pub async fn set_raw_config_int64(
        &self,
        context: &Context,
        key: impl AsRef<str>,
        value: i64,
    ) -> Result<()> {
        self.set_raw_config(context, key, Some(&format!("{}", value)))
            .await
    }

    pub async fn get_raw_config_int64(&self, key: impl AsRef<str>) -> Option<i64> {
        self.get_raw_config(key).await.and_then(|r| r.parse().ok())
    }

    /// Alternative to sqlite3_last_insert_rowid() which MUST NOT be used due to race conditions, see comment above.
    /// the ORDER BY ensures, this function always returns the most recent id,
    /// eg. if a Message-ID is split into different messages.
    pub async fn get_rowid(
        &self,
        table: impl AsRef<str>,
        field: impl AsRef<str>,
        value: impl AsRef<str>,
    ) -> Result<u32> {
        // alternative to sqlite3_last_insert_rowid() which MUST NOT be used due to race conditions, see comment above.
        // the ORDER BY ensures, this function always returns the most recent id,
        // eg. if a Message-ID is split into different messages.
        let query = format!(
            "SELECT id FROM {} WHERE {}=? ORDER BY id DESC",
            table.as_ref(),
            field.as_ref(),
        );

        let res: i64 = self.query_value(&query, paramsx![value.as_ref()]).await?;

        Ok(res as u32)
    }

    pub async fn get_rowid2(
        &self,
        table: impl AsRef<str>,
        field: impl AsRef<str>,
        value: i64,
        field2: impl AsRef<str>,
        value2: i32,
    ) -> Result<u32> {
        let query = format!(
            "SELECT id FROM {} WHERE {}=? AND {}=? ORDER BY id DESC",
            table.as_ref(),
            field.as_ref(),
            field2.as_ref(),
        );

        let res: i64 = self
            .query_value(query.as_ref(), paramsx![value, value2])
            .await?;

        Ok(res as u32)
    }
}

pub async fn housekeeping(context: &Context) -> Result<()> {
    let mut files_in_use = HashSet::new();
    let mut unreferenced_count = 0usize;

    info!(context, "Start housekeeping...");
    maybe_add_from_param(
        context,
        &mut files_in_use,
        "SELECT param FROM msgs WHERE chat_id!=3 AND type!=10;",
        Param::File,
    )
    .await?;
    maybe_add_from_param(
        context,
        &mut files_in_use,
        "SELECT param FROM jobs;",
        Param::File,
    )
    .await?;
    maybe_add_from_param(
        context,
        &mut files_in_use,
        "SELECT param FROM chats;",
        Param::ProfileImage,
    )
    .await?;
    maybe_add_from_param(
        context,
        &mut files_in_use,
        "SELECT param FROM contacts;",
        Param::ProfileImage,
    )
    .await?;

    let pool = context.sql.get_pool().await?;
    let mut rows = sqlx::query_as("SELECT value FROM config;").fetch(&pool);

    while let Some(row) = rows.next().await {
        let (row,): (String,) = row?;
        maybe_add_file(&mut files_in_use, row);
    }

    info!(context, "{} files in use.", files_in_use.len(),);
    /* go through directory and delete unused files */
    let p = context.get_blobdir();
    match async_std::fs::read_dir(p).await {
        Ok(mut dir_handle) => {
            /* avoid deletion of files that are just created to build a message object */
            let diff = std::time::Duration::from_secs(60 * 60);
            let keep_files_newer_than = std::time::SystemTime::now().checked_sub(diff).unwrap();

            while let Some(entry) = dir_handle.next().await {
                if entry.is_err() {
                    break;
                }
                let entry = entry.unwrap();
                let name_f = entry.file_name();
                let name_s = name_f.to_string_lossy();

                if is_file_in_use(&files_in_use, None, &name_s)
                    || is_file_in_use(&files_in_use, Some(".increation"), &name_s)
                    || is_file_in_use(&files_in_use, Some(".waveform"), &name_s)
                    || is_file_in_use(&files_in_use, Some("-preview.jpg"), &name_s)
                {
                    continue;
                }

                unreferenced_count += 1;

                if let Ok(stats) = async_std::fs::metadata(entry.path()).await {
                    let recently_created =
                        stats.created().is_ok() && stats.created().unwrap() > keep_files_newer_than;
                    let recently_modified = stats.modified().is_ok()
                        && stats.modified().unwrap() > keep_files_newer_than;
                    let recently_accessed = stats.accessed().is_ok()
                        && stats.accessed().unwrap() > keep_files_newer_than;

                    if recently_created || recently_modified || recently_accessed {
                        info!(
                            context,
                            "Housekeeping: Keeping new unreferenced file #{}: {:?}",
                            unreferenced_count,
                            entry.file_name(),
                        );
                        continue;
                    }
                }
                info!(
                    context,
                    "Housekeeping: Deleting unreferenced file #{}: {:?}",
                    unreferenced_count,
                    entry.file_name()
                );
                let path = entry.path();
                dc_delete_file(context, path).await;
            }
        }
        Err(err) => {
            warn!(
                context,
                "Housekeeping: Cannot open {}. ({})",
                context.get_blobdir().display(),
                err
            );
        }
    }

    if let Err(err) = prune_tombstones(context).await {
        warn!(
            context,
            "Houskeeping: Cannot prune message tombstones: {}", err
        );
    }

    info!(context, "Housekeeping done.",);

    Ok(())
}

fn is_file_in_use(files_in_use: &HashSet<String>, namespc_opt: Option<&str>, name: &str) -> bool {
    let name_to_check = if let Some(namespc) = namespc_opt {
        let name_len = name.len();
        let namespc_len = namespc.len();
        if name_len <= namespc_len || !name.ends_with(namespc) {
            return false;
        }
        &name[..name_len - namespc_len]
    } else {
        name
    };
    files_in_use.contains(name_to_check)
}

fn maybe_add_file(files_in_use: &mut HashSet<String>, file: impl AsRef<str>) {
    if !file.as_ref().starts_with("$BLOBDIR/") {
        return;
    }

    files_in_use.insert(file.as_ref()[9..].into());
}

async fn maybe_add_from_param(
    context: &Context,
    files_in_use: &mut HashSet<String>,
    query: &str,
    param_id: Param,
) -> Result<()> {
    info!(context, "maybe_add_from_param: {}", query);
    let pool = context.sql.get_pool().await?;
    let mut rows = sqlx::query_as(query).fetch(&pool);

    while let Some(row) = rows.next().await {
        let (row,): (String,) = row?;
        info!(context, "param: {}", &row);
        let param: Params = row.parse().unwrap_or_default();

        if let Some(file) = param.get(param_id) {
            info!(context, "got file: {:?} {}", param_id, file);
            maybe_add_file(files_in_use, file);
        }
    }

    Ok(())
}

#[allow(clippy::cognitive_complexity)]
async fn open(
    context: &Context,
    sql: &Sql,
    dbfile: impl AsRef<Path>,
    readonly: bool,
) -> crate::error::Result<()> {
    if sql.is_open().await {
        error!(
            context,
            "Cannot open, database \"{:?}\" already opened.",
            dbfile.as_ref(),
        );
        return Err(Error::SqlAlreadyOpen.into());
    }

    if readonly {
        // TODO: readonly mode
    }

    let xpool = sqlx::SqlitePool::builder()
        .min_size(1)
        .max_size(4)
        .build(&format!("sqlite://{}", dbfile.as_ref().to_string_lossy()))
        .await?;

    {
        *sql.xpool.write().await = Some(xpool)
    }

    if !readonly {
        let mut exists_before_update = false;
        let mut dbversion_before_update: i32 = -1;

        if sql.table_exists("config").await? {
            exists_before_update = true;
            if let Some(version) = sql.get_raw_config_int("dbversion").await {
                dbversion_before_update = version;
            }
        }

        // (1) update low-level database structure.
        // this should be done before updates that use high-level objects that
        // rely themselves on the low-level structure.
        // --------------------------------------------------------------------

        migrations::run(context, &sql, dbversion_before_update, exists_before_update).await?;

        // general updates
        // (2) updates that require high-level objects
        // (the structure is complete now and all objects are usable)
        // --------------------------------------------------------------------
        let mut recalc_fingerprints = false;
        let mut update_icons = false;

        if dbversion_before_update < 34 {
            recalc_fingerprints = true;
        }

        if dbversion_before_update < 61 {
            update_icons = true;
        }

        if recalc_fingerprints {
            info!(context, "[migration] recalc fingerprints");
            let pool = context.sql.get_pool().await?;
            let mut rows = sqlx::query_as("SELECT addr FROM acpeerstates;").fetch(&pool);

            while let Some(addr) = rows.next().await {
                let (addr,): (String,) = addr?;

                if let Ok(ref mut peerstate) = Peerstate::from_addr(context, &addr).await {
                    peerstate.recalc_fingerprint();
                    peerstate.save_to_db(sql, false).await?;
                }
            }
        }

        if update_icons {
            update_saved_messages_icon(context).await?;
            update_device_icon(context).await?;
        }
    }

    info!(context, "Opened {:?}.", dbfile.as_ref(),);

    Ok(())
}

/// Removes from the database locally deleted messages that also don't
/// have a server UID.
async fn prune_tombstones(context: &Context) -> Result<()> {
    context
        .sql
        .execute(
            r#"
DELETE FROM msgs
  WHERE (chat_id = ? OR hidden)
  AND server_uid = 0
"#,
            paramsx![DC_CHAT_ID_TRASH as i32],
        )
        .await?;
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_maybe_add_file() {
        let mut files = Default::default();
        maybe_add_file(&mut files, "$BLOBDIR/hello");
        maybe_add_file(&mut files, "$BLOBDIR/world.txt");
        maybe_add_file(&mut files, "world2.txt");
        maybe_add_file(&mut files, "$BLOBDIR");

        assert!(files.contains("hello"));
        assert!(files.contains("world.txt"));
        assert!(!files.contains("world2.txt"));
        assert!(!files.contains("$BLOBDIR"));
    }

    #[test]
    fn test_is_file_in_use() {
        let mut files = Default::default();
        maybe_add_file(&mut files, "$BLOBDIR/hello");
        maybe_add_file(&mut files, "$BLOBDIR/world.txt");
        maybe_add_file(&mut files, "world2.txt");

        assert!(is_file_in_use(&files, None, "hello"));
        assert!(!is_file_in_use(&files, Some(".txt"), "hello"));
        assert!(is_file_in_use(&files, Some("-suffix"), "world.txt-suffix"));
    }
}
