//! # Import/export module.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use ::pgp::types::KeyTrait;
use anyhow::{bail, ensure, format_err, Context as _, Result};
use deltachat_contact_tools::EmailAddress;
use futures::StreamExt;
use futures_lite::FutureExt;

use tokio::fs::{self, File};
use tokio_tar::Archive;

use crate::blob::BlobDirContents;
use crate::chat::{self, delete_and_reset_all_device_msgs};
use crate::context::Context;
use crate::e2ee;
use crate::events::EventType;
use crate::key::{self, DcKey, DcSecretKey, SignedPublicKey, SignedSecretKey};
use crate::log::LogExt;
use crate::message::{Message, Viewtype};
use crate::pgp;
use crate::sql;
use crate::tools::{
    create_folder, delete_file, get_filesuffix_lc, read_file, time, write_file, TempPathGuard,
};

mod key_transfer;
mod transfer;

pub use key_transfer::{continue_key_transfer, initiate_key_transfer};
pub use transfer::{get_backup, BackupProvider};

// Name of the database file in the backup.
const DBFILE_BACKUP_NAME: &str = "dc_database_backup.sqlite";
pub(crate) const BLOBS_BACKUP_NAME: &str = "blobs_backup";

/// Import/export command.
#[derive(Debug, Display, Copy, Clone, PartialEq, Eq, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum ImexMode {
    /// Export all private keys and all public keys of the user to the
    /// directory given as `path`. The default key is written to the files
    /// `{public,private}-key-<addr>-default-<fingerprint>.asc`, if there are more keys, they are
    /// written to files as `{public,private}-key-<addr>-<id>-<fingerprint>.asc`.
    ExportSelfKeys = 1,

    /// Import private keys found in `path` if it is a directory, otherwise import a private key
    /// from `path`.
    /// The last imported key is made the default keys unless its name contains the string `legacy`.
    /// Public keys are not imported.
    ImportSelfKeys = 2,

    /// Export a backup to the directory given as `path` with the given `passphrase`.
    /// The backup contains all contacts, chats, images and other data and device independent settings.
    /// The backup does not contain device dependent settings as ringtones or LED notification settings.
    /// The name of the backup is `delta-chat-backup-<day>-<number>-<addr>.tar`.
    ExportBackup = 11,

    /// `path` is the file (not: directory) to import. The file is normally
    /// created by DC_IMEX_EXPORT_BACKUP and detected by imex_has_backup(). Importing a backup
    /// is only possible as long as the context is not configured or used in another way.
    ImportBackup = 12,
}

/// Import/export things.
///
/// What to do is defined by the `what` parameter.
///
/// During execution of the job,
/// some events are sent out:
///
/// - A number of `DC_EVENT_IMEX_PROGRESS` events are sent and may be used to create
///   a progress bar or stuff like that. Moreover, you'll be informed when the imex-job is done.
///
/// - For each file written on export, the function sends `DC_EVENT_IMEX_FILE_WRITTEN`
///
/// Only one import-/export-progress can run at the same time.
/// To cancel an import-/export-progress, drop the future returned by this function.
pub async fn imex(
    context: &Context,
    what: ImexMode,
    path: &Path,
    passphrase: Option<String>,
) -> Result<()> {
    let cancel = context.alloc_ongoing().await?;

    let res = {
        let _guard = context.scheduler.pause(context.clone()).await?;
        imex_inner(context, what, path, passphrase)
            .race(async {
                cancel.recv().await.ok();
                Err(format_err!("canceled"))
            })
            .await
    };
    context.free_ongoing().await;

    if let Err(err) = res.as_ref() {
        // We are using Anyhow's .context() and to show the inner error, too, we need the {:#}:
        error!(context, "IMEX failed to complete: {:#}", err);
        context.emit_event(EventType::ImexProgress(0));
    } else {
        info!(context, "IMEX successfully completed");
        context.emit_event(EventType::ImexProgress(1000));
    }

    res
}

/// Returns the filename of the backup found (otherwise an error)
pub async fn has_backup(_context: &Context, dir_name: &Path) -> Result<String> {
    let mut dir_iter = tokio::fs::read_dir(dir_name).await?;
    let mut newest_backup_name = "".to_string();
    let mut newest_backup_path: Option<PathBuf> = None;

    while let Ok(Some(dirent)) = dir_iter.next_entry().await {
        let path = dirent.path();
        let name = dirent.file_name();
        let name: String = name.to_string_lossy().into();
        if name.starts_with("delta-chat")
            && name.ends_with(".tar")
            && (newest_backup_name.is_empty() || name > newest_backup_name)
        {
            // We just use string comparison to determine which backup is newer.
            // This works fine because the filenames have the form `delta-chat-backup-2023-10-18-00-foo@example.com.tar`
            newest_backup_path = Some(path);
            newest_backup_name = name;
        }
    }

    match newest_backup_path {
        Some(path) => Ok(path.to_string_lossy().into_owned()),
        None => bail!("no backup found in {}", dir_name.display()),
    }
}

async fn maybe_add_bcc_self_device_msg(context: &Context) -> Result<()> {
    if !context.sql.get_raw_config_bool("bcc_self").await? {
        let mut msg = Message::new(Viewtype::Text);
        // TODO: define this as a stockstring once the wording is settled.
        msg.text = "It seems you are using multiple devices with Delta Chat. Great!\n\n\
             If you also want to synchronize outgoing messages across all devices, \
             go to \"Settings → Advanced\" and enable \"Send Copy to Self\"."
            .to_string();
        chat::add_device_msg(context, Some("bcc-self-hint"), Some(&mut msg)).await?;
    }
    Ok(())
}

async fn set_self_key(context: &Context, armored: &str, set_default: bool) -> Result<()> {
    // try hard to only modify key-state
    let (private_key, header) = SignedSecretKey::from_asc(armored)?;
    let public_key = private_key.split_public_key()?;
    if let Some(preferencrypt) = header.get("Autocrypt-Prefer-Encrypt") {
        let e2ee_enabled = match preferencrypt.as_str() {
            "nopreference" => 0,
            "mutual" => 1,
            _ => {
                bail!("invalid Autocrypt-Prefer-Encrypt header: {:?}", header);
            }
        };
        context
            .sql
            .set_raw_config_int("e2ee_enabled", e2ee_enabled)
            .await?;
    } else {
        // `Autocrypt-Prefer-Encrypt` is not included
        // in keys exported to file.
        //
        // `Autocrypt-Prefer-Encrypt` also SHOULD be sent
        // in Autocrypt Setup Message according to Autocrypt specification,
        // but K-9 6.802 does not include this header.
        //
        // We keep current setting in this case.
        info!(context, "No Autocrypt-Prefer-Encrypt header.");
    };

    let self_addr = context.get_primary_self_addr().await?;
    let addr = EmailAddress::new(&self_addr)?;
    let keypair = pgp::KeyPair {
        addr,
        public: public_key,
        secret: private_key,
    };
    key::store_self_keypair(
        context,
        &keypair,
        if set_default {
            key::KeyPairUse::Default
        } else {
            key::KeyPairUse::ReadOnly
        },
    )
    .await?;

    info!(context, "stored self key: {:?}", keypair.secret.key_id());
    Ok(())
}

async fn imex_inner(
    context: &Context,
    what: ImexMode,
    path: &Path,
    passphrase: Option<String>,
) -> Result<()> {
    info!(
        context,
        "{} path: {}",
        match what {
            ImexMode::ExportSelfKeys | ImexMode::ExportBackup => "Export",
            ImexMode::ImportSelfKeys | ImexMode::ImportBackup => "Import",
        },
        path.display()
    );
    ensure!(context.sql.is_open().await, "Database not opened.");
    context.emit_event(EventType::ImexProgress(10));

    if what == ImexMode::ExportBackup || what == ImexMode::ExportSelfKeys {
        // before we export anything, make sure the private key exists
        e2ee::ensure_secret_key_exists(context)
            .await
            .context("Cannot create private key or private key not available")?;

        create_folder(context, &path).await?;
    }

    match what {
        ImexMode::ExportSelfKeys => export_self_keys(context, path).await,
        ImexMode::ImportSelfKeys => import_self_keys(context, path).await,

        ImexMode::ExportBackup => {
            export_backup(context, path, passphrase.unwrap_or_default()).await
        }
        ImexMode::ImportBackup => {
            import_backup(context, path, passphrase.unwrap_or_default()).await
        }
    }
}

/// Imports backup into the currently open database.
///
/// The contents of the currently open database will be lost.
///
/// `passphrase` is the passphrase used to open backup database. If backup is unencrypted, pass
/// empty string here.
async fn import_backup(
    context: &Context,
    backup_to_import: &Path,
    passphrase: String,
) -> Result<()> {
    ensure!(
        !context.is_configured().await?,
        "Cannot import backups to accounts in use."
    );
    ensure!(
        !context.scheduler.is_running().await,
        "cannot import backup, IO is running"
    );

    let backup_file = File::open(backup_to_import).await?;
    let file_size = backup_file.metadata().await?.len();
    info!(
        context,
        "Import \"{}\" ({} bytes) to \"{}\".",
        backup_to_import.display(),
        file_size,
        context.get_dbfile().display()
    );

    let mut archive = Archive::new(backup_file);

    let mut entries = archive.entries()?;
    let mut last_progress = 0;
    while let Some(file) = entries.next().await {
        let f = &mut file?;

        let current_pos = f.raw_file_position();
        let progress = 1000 * current_pos / file_size;
        if progress != last_progress && progress > 10 && progress < 1000 {
            // We already emitted ImexProgress(10) above
            context.emit_event(EventType::ImexProgress(progress as usize));
            last_progress = progress;
        }

        if f.path()?.file_name() == Some(OsStr::new(DBFILE_BACKUP_NAME)) {
            // async_tar can't unpack to a specified file name, so we just unpack to the blobdir and then move the unpacked file.
            f.unpack_in(context.get_blobdir()).await?;
            let unpacked_database = context.get_blobdir().join(DBFILE_BACKUP_NAME);
            context
                .sql
                .import(&unpacked_database, passphrase.clone())
                .await
                .context("cannot import unpacked database")?;
            fs::remove_file(unpacked_database)
                .await
                .context("cannot remove unpacked database")?;
        } else {
            // async_tar will unpack to blobdir/BLOBS_BACKUP_NAME, so we move the file afterwards.
            f.unpack_in(context.get_blobdir()).await?;
            let from_path = context.get_blobdir().join(f.path()?);
            if from_path.is_file() {
                if let Some(name) = from_path.file_name() {
                    fs::rename(&from_path, context.get_blobdir().join(name)).await?;
                } else {
                    warn!(context, "No file name");
                }
            }
        }
    }

    context.sql.run_migrations(context).await?;
    delete_and_reset_all_device_msgs(context).await?;

    Ok(())
}

/*******************************************************************************
 * Export backup
 ******************************************************************************/

/// Returns Ok((temp_db_path, temp_path, dest_path)) on success. Unencrypted database can be
/// written to temp_db_path. The backup can then be written to temp_path. If the backup succeeded,
/// it can be renamed to dest_path. This guarantees that the backup is complete.
fn get_next_backup_path(
    folder: &Path,
    addr: &str,
    backup_time: i64,
) -> Result<(PathBuf, PathBuf, PathBuf)> {
    let folder = PathBuf::from(folder);
    let stem = chrono::DateTime::<chrono::Utc>::from_timestamp(backup_time, 0)
        .context("can't get next backup path")?
        // Don't change this file name format, in `dc_imex_has_backup` we use string comparison to determine which backup is newer:
        .format("delta-chat-backup-%Y-%m-%d")
        .to_string();

    // 64 backup files per day should be enough for everyone
    for i in 0..64 {
        let mut tempdbfile = folder.clone();
        tempdbfile.push(format!("{stem}-{i:02}-{addr}.db"));

        let mut tempfile = folder.clone();
        tempfile.push(format!("{stem}-{i:02}-{addr}.tar.part"));

        let mut destfile = folder.clone();
        destfile.push(format!("{stem}-{i:02}-{addr}.tar"));

        if !tempdbfile.exists() && !tempfile.exists() && !destfile.exists() {
            return Ok((tempdbfile, tempfile, destfile));
        }
    }
    bail!("could not create backup file, disk full?");
}

/// Exports the database to a separate file with the given passphrase.
///
/// Set passphrase to empty string to export the database unencrypted.
async fn export_backup(context: &Context, dir: &Path, passphrase: String) -> Result<()> {
    // get a fine backup file name (the name includes the date so that multiple backup instances are possible)
    let now = time();
    let self_addr = context.get_primary_self_addr().await?;
    let (temp_db_path, temp_path, dest_path) = get_next_backup_path(dir, &self_addr, now)?;
    let temp_db_path = TempPathGuard::new(temp_db_path);
    let temp_path = TempPathGuard::new(temp_path);

    export_database(context, &temp_db_path, passphrase, now)
        .await
        .context("could not export database")?;

    info!(
        context,
        "Backup '{}' to '{}'.",
        context.get_dbfile().display(),
        dest_path.display(),
    );

    export_backup_inner(context, &temp_db_path, &temp_path).await?;
    fs::rename(temp_path, &dest_path).await?;
    context.emit_event(EventType::ImexFileWritten(dest_path));
    Ok(())
}

async fn export_backup_inner(
    context: &Context,
    temp_db_path: &Path,
    temp_path: &Path,
) -> Result<()> {
    let file = File::create(temp_path).await?;

    let mut builder = tokio_tar::Builder::new(file);

    builder
        .append_path_with_name(temp_db_path, DBFILE_BACKUP_NAME)
        .await?;

    let blobdir = BlobDirContents::new(context).await?;
    let mut last_progress = 0;

    for (i, blob) in blobdir.iter().enumerate() {
        let mut file = File::open(blob.to_abs_path()).await?;
        let path_in_archive = PathBuf::from(BLOBS_BACKUP_NAME).join(blob.as_name());
        builder.append_file(path_in_archive, &mut file).await?;
        let progress = 1000 * i / blobdir.len();
        if progress != last_progress && progress > 10 && progress < 1000 {
            context.emit_event(EventType::ImexProgress(progress));
            last_progress = progress;
        }
    }

    builder.finish().await?;
    Ok(())
}

/// Imports secret key from a file.
async fn import_secret_key(context: &Context, path: &Path, set_default: bool) -> Result<()> {
    let buf = read_file(context, &path).await?;
    let armored = std::string::String::from_utf8_lossy(&buf);
    set_self_key(context, &armored, set_default).await?;
    Ok(())
}

/// Imports secret keys from the provided file or directory.
///
/// If provided path is a file, ASCII-armored secret key is read from the file
/// and set as the default key.
///
/// If provided path is a directory, all files with .asc extension
/// containing secret keys are imported and the last successfully
/// imported which does not contain "legacy" in its filename
/// is set as the default.
async fn import_self_keys(context: &Context, path: &Path) -> Result<()> {
    let attr = tokio::fs::metadata(path).await?;

    if attr.is_file() {
        info!(
            context,
            "Importing secret key from {} as the default key.",
            path.display()
        );
        let set_default = true;
        import_secret_key(context, path, set_default).await?;
        return Ok(());
    }

    let mut imported_cnt = 0;

    let mut dir_handle = tokio::fs::read_dir(&path).await?;
    while let Ok(Some(entry)) = dir_handle.next_entry().await {
        let entry_fn = entry.file_name();
        let name_f = entry_fn.to_string_lossy();
        let path_plus_name = path.join(&entry_fn);
        if let Some(suffix) = get_filesuffix_lc(&name_f) {
            if suffix != "asc" {
                continue;
            }
        } else {
            continue;
        };
        let set_default = !name_f.contains("legacy");
        info!(
            context,
            "Considering key file: {}.",
            path_plus_name.display()
        );

        if let Err(err) = import_secret_key(context, &path_plus_name, set_default).await {
            warn!(
                context,
                "Failed to import secret key from {}: {:#}.",
                path_plus_name.display(),
                err
            );
            continue;
        }

        imported_cnt += 1;
    }
    ensure!(
        imported_cnt > 0,
        "No private keys found in {}.",
        path.display()
    );
    Ok(())
}

async fn export_self_keys(context: &Context, dir: &Path) -> Result<()> {
    let mut export_errors = 0;

    let keys = context
        .sql
        .query_map(
            "SELECT id, public_key, private_key, id=(SELECT value FROM config WHERE keyname='key_id') FROM keypairs;",
            (),
            |row| {
                let id = row.get(0)?;
                let public_key_blob: Vec<u8> = row.get(1)?;
                let public_key = SignedPublicKey::from_slice(&public_key_blob);
                let private_key_blob: Vec<u8> = row.get(2)?;
                let private_key = SignedSecretKey::from_slice(&private_key_blob);
                let is_default: i32 = row.get(3)?;

                Ok((id, public_key, private_key, is_default))
            },
            |keys| {
                keys.collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(Into::into)
            },
        )
        .await?;
    let self_addr = context.get_primary_self_addr().await?;
    for (id, public_key, private_key, is_default) in keys {
        let id = Some(id).filter(|_| is_default == 0);

        if let Ok(key) = public_key {
            if let Err(err) = export_key_to_asc_file(context, dir, &self_addr, id, &key).await {
                error!(context, "Failed to export public key: {:#}.", err);
                export_errors += 1;
            }
        } else {
            export_errors += 1;
        }
        if let Ok(key) = private_key {
            if let Err(err) = export_key_to_asc_file(context, dir, &self_addr, id, &key).await {
                error!(context, "Failed to export private key: {:#}.", err);
                export_errors += 1;
            }
        } else {
            export_errors += 1;
        }
    }

    ensure!(export_errors == 0, "errors while exporting keys");
    Ok(())
}

/// Returns the exported key file name inside `dir`.
async fn export_key_to_asc_file<T>(
    context: &Context,
    dir: &Path,
    addr: &str,
    id: Option<i64>,
    key: &T,
) -> Result<String>
where
    T: DcKey,
{
    let file_name = {
        let kind = match T::is_private() {
            false => "public",
            true => "private",
        };
        let id = id.map_or("default".into(), |i| i.to_string());
        let fp = DcKey::fingerprint(key).hex();
        format!("{kind}-key-{addr}-{id}-{fp}.asc")
    };
    let path = dir.join(&file_name);
    info!(
        context,
        "Exporting key {:?} to {}.",
        key.key_id(),
        path.display()
    );

    // Delete the file if it already exists.
    delete_file(context, &path).await.ok();

    let content = key.to_asc(None).into_bytes();
    write_file(context, &path, &content)
        .await
        .with_context(|| format!("cannot write key to {}", path.display()))?;
    context.emit_event(EventType::ImexFileWritten(path));
    Ok(file_name)
}

/// Exports the database to *dest*, encrypted using *passphrase*.
///
/// The directory of *dest* must already exist, if *dest* itself exists it will be
/// overwritten.
///
/// This also verifies that IO is not running during the export.
async fn export_database(
    context: &Context,
    dest: &Path,
    passphrase: String,
    timestamp: i64,
) -> Result<()> {
    ensure!(
        !context.scheduler.is_running().await,
        "cannot export backup, IO is running"
    );
    let timestamp = timestamp.try_into().context("32-bit UNIX time overflow")?;

    // TODO: Maybe introduce camino crate for UTF-8 paths where we need them.
    let dest = dest
        .to_str()
        .with_context(|| format!("path {} is not valid unicode", dest.display()))?;

    context
        .sql
        .set_raw_config_int("backup_time", timestamp)
        .await?;
    sql::housekeeping(context).await.log_err(context).ok();
    context
        .sql
        .call_write(|conn| {
            conn.execute("VACUUM;", ())
                .map_err(|err| warn!(context, "Vacuum failed, exporting anyway {err}"))
                .ok();
            conn.execute("ATTACH DATABASE ? AS backup KEY ?", (dest, passphrase))
                .context("failed to attach backup database")?;
            let res = conn
                .query_row("SELECT sqlcipher_export('backup')", [], |_row| Ok(()))
                .context("failed to export to attached backup database");
            conn.execute(
                "UPDATE backup.config SET value='0' WHERE keyname='verified_one_on_one_chats';",
                [],
            )
            .ok(); // If verified_one_on_one_chats was not set, this errors, which we ignore
            conn.execute("DETACH DATABASE backup", [])
                .context("failed to detach backup database")?;
            res?;
            Ok(())
        })
        .await
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::task;

    use super::*;
    use crate::config::Config;
    use crate::test_utils::{alice_keypair, TestContext};

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_export_public_key_to_asc_file() {
        let context = TestContext::new().await;
        let key = alice_keypair().public;
        let blobdir = Path::new("$BLOBDIR");
        let filename = export_key_to_asc_file(&context.ctx, blobdir, "a@b", None, &key)
            .await
            .unwrap();
        assert!(filename.starts_with("public-key-a@b-default-"));
        assert!(filename.ends_with(".asc"));
        let blobdir = context.ctx.get_blobdir().to_str().unwrap();
        let filename = format!("{blobdir}/{filename}");
        let bytes = tokio::fs::read(&filename).await.unwrap();

        assert_eq!(bytes, key.to_asc(None).into_bytes());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_import_private_key_exported_to_asc_file() {
        let context = TestContext::new().await;
        let key = alice_keypair().secret;
        let blobdir = Path::new("$BLOBDIR");
        let filename = export_key_to_asc_file(&context.ctx, blobdir, "a@b", None, &key)
            .await
            .unwrap();
        let fingerprint = filename
            .strip_prefix("private-key-a@b-default-")
            .unwrap()
            .strip_suffix(".asc")
            .unwrap();
        assert_eq!(fingerprint, DcKey::fingerprint(&key).hex());
        let blobdir = context.ctx.get_blobdir().to_str().unwrap();
        let filename = format!("{blobdir}/{filename}");
        let bytes = tokio::fs::read(&filename).await.unwrap();

        assert_eq!(bytes, key.to_asc(None).into_bytes());

        let alice = &TestContext::new_alice().await;
        if let Err(err) = imex(alice, ImexMode::ImportSelfKeys, Path::new(&filename), None).await {
            panic!("got error on import: {err:#}");
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_export_and_import_key_from_dir() {
        let export_dir = tempfile::tempdir().unwrap();

        let context = TestContext::new_alice().await;
        if let Err(err) = imex(
            &context.ctx,
            ImexMode::ExportSelfKeys,
            export_dir.path(),
            None,
        )
        .await
        {
            panic!("got error on export: {err:#}");
        }

        let context2 = TestContext::new_alice().await;
        if let Err(err) = imex(
            &context2.ctx,
            ImexMode::ImportSelfKeys,
            export_dir.path(),
            None,
        )
        .await
        {
            panic!("got error on import: {err:#}");
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_import_second_key() -> Result<()> {
        let alice = &TestContext::new_alice().await;
        let chat = alice.create_chat(alice).await;
        let sent = alice.send_text(chat.id, "Encrypted with old key").await;
        let export_dir = tempfile::tempdir().unwrap();

        let alice = &TestContext::new().await;
        alice.configure_addr("alice@example.org").await;
        imex(alice, ImexMode::ExportSelfKeys, export_dir.path(), None).await?;

        let alice = &TestContext::new_alice().await;
        let old_key = key::load_self_secret_key(alice).await?;

        imex(alice, ImexMode::ImportSelfKeys, export_dir.path(), None).await?;

        let new_key = key::load_self_secret_key(alice).await?;
        assert_ne!(new_key, old_key);
        assert_eq!(
            key::load_self_secret_keyring(alice).await?,
            vec![new_key, old_key]
        );

        let msg = alice.recv_msg(&sent).await;
        assert!(msg.get_showpadlock());
        assert_eq!(msg.chat_id, alice.get_self_chat().await.id);
        assert_eq!(msg.get_text(), "Encrypted with old key");

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_export_and_import_backup() -> Result<()> {
        for set_verified_oneonone_chats in [true, false] {
            let backup_dir = tempfile::tempdir().unwrap();

            let context1 = TestContext::new_alice().await;
            assert!(context1.is_configured().await?);
            if set_verified_oneonone_chats {
                context1
                    .set_config_bool(Config::VerifiedOneOnOneChats, true)
                    .await?;
            }

            let context2 = TestContext::new().await;
            assert!(!context2.is_configured().await?);
            assert!(has_backup(&context2, backup_dir.path()).await.is_err());

            // export from context1
            assert!(
                imex(&context1, ImexMode::ExportBackup, backup_dir.path(), None)
                    .await
                    .is_ok()
            );
            let _event = context1
                .evtracker
                .get_matching(|evt| matches!(evt, EventType::ImexProgress(1000)))
                .await;

            // import to context2
            let backup = has_backup(&context2, backup_dir.path()).await?;

            // Import of unencrypted backup with incorrect "foobar" backup passphrase fails.
            assert!(imex(
                &context2,
                ImexMode::ImportBackup,
                backup.as_ref(),
                Some("foobar".to_string())
            )
            .await
            .is_err());

            assert!(
                imex(&context2, ImexMode::ImportBackup, backup.as_ref(), None)
                    .await
                    .is_ok()
            );
            let _event = context2
                .evtracker
                .get_matching(|evt| matches!(evt, EventType::ImexProgress(1000)))
                .await;

            assert!(context2.is_configured().await?);
            assert_eq!(
                context2.get_config(Config::Addr).await?,
                Some("alice@example.org".to_string())
            );
            assert_eq!(
                context2
                    .get_config_bool(Config::VerifiedOneOnOneChats)
                    .await?,
                false
            );
            assert_eq!(
                context1
                    .get_config_bool(Config::VerifiedOneOnOneChats)
                    .await?,
                set_verified_oneonone_chats
            );
        }
        Ok(())
    }

    /// This is a regression test for
    /// https://github.com/deltachat/deltachat-android/issues/2263
    /// where the config cache wasn't reset properly after a backup.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_import_backup_reset_config_cache() -> Result<()> {
        let backup_dir = tempfile::tempdir()?;
        let context1 = TestContext::new_alice().await;
        let context2 = TestContext::new().await;
        assert!(!context2.is_configured().await?);

        // export from context1
        imex(&context1, ImexMode::ExportBackup, backup_dir.path(), None).await?;

        // import to context2
        let backup = has_backup(&context2, backup_dir.path()).await?;
        let context2_cloned = context2.clone();
        let handle = task::spawn(async move {
            imex(
                &context2_cloned,
                ImexMode::ImportBackup,
                backup.as_ref(),
                None,
            )
            .await
            .unwrap();
        });

        while !handle.is_finished() {
            // The database is still unconfigured;
            // fill the config cache with the old value.
            context2.is_configured().await.ok();
            tokio::time::sleep(Duration::from_micros(1)).await;
        }

        // Assert that the config cache has the new value now.
        assert!(context2.is_configured().await?);

        Ok(())
    }
}
