//! # Import/export module.

use std::any::Any;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use ::pgp::types::KeyTrait;
use anyhow::{bail, ensure, format_err, Context as _, Result};
use futures::StreamExt;
use futures_lite::FutureExt;
use rand::{thread_rng, Rng};
use tokio::fs::{self, File};
use tokio::io::BufWriter;
use tokio_tar::Archive;

use crate::blob::{BlobDirContents, BlobObject};
use crate::chat::{self, delete_and_reset_all_device_msgs, ChatId};
use crate::config::Config;
use crate::contact::ContactId;
use crate::context::Context;
use crate::e2ee;
use crate::events::EventType;
use crate::key::{
    self, load_self_secret_key, DcKey, DcSecretKey, SignedPublicKey, SignedSecretKey,
};
use crate::log::LogExt;
use crate::message::{Message, MsgId, Viewtype};
use crate::mimeparser::SystemMessage;
use crate::param::Param;
use crate::pgp;
use crate::sql;
use crate::stock_str;
use crate::tools::{
    create_folder, delete_file, get_filesuffix_lc, open_file_std, read_file, time, write_file,
    EmailAddress,
};

mod transfer;

pub use transfer::{get_backup, BackupProvider};

// Name of the database file in the backup.
const DBFILE_BACKUP_NAME: &str = "dc_database_backup.sqlite";
pub(crate) const BLOBS_BACKUP_NAME: &str = "blobs_backup";

/// Import/export command.
#[derive(Debug, Display, Copy, Clone, PartialEq, Eq, FromPrimitive, ToPrimitive)]
#[repr(u32)]
pub enum ImexMode {
    /// Export all private keys and all public keys of the user to the
    /// directory given as `path`.  The default key is written to the files `public-key-default.asc`
    /// and `private-key-default.asc`, if there are more keys, they are written to files as
    /// `public-key-<id>.asc` and `private-key-<id>.asc`
    ExportSelfKeys = 1,

    /// Import private keys found in the directory given as `path`.
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

/// Initiates key transfer via Autocrypt Setup Message.
///
/// Returns setup code.
pub async fn initiate_key_transfer(context: &Context) -> Result<String> {
    let setup_code = create_setup_code(context);
    /* this may require a keypair to be created. this may take a second ... */
    let setup_file_content = render_setup_file(context, &setup_code).await?;
    /* encrypting may also take a while ... */
    let setup_file_blob = BlobObject::create(
        context,
        "autocrypt-setup-message.html",
        setup_file_content.as_bytes(),
    )
    .await?;

    let chat_id = ChatId::create_for_contact(context, ContactId::SELF).await?;
    let mut msg = Message {
        viewtype: Viewtype::File,
        ..Default::default()
    };
    msg.param.set(Param::File, setup_file_blob.as_name());
    msg.subject = stock_str::ac_setup_msg_subject(context).await;
    msg.param
        .set(Param::MimeType, "application/autocrypt-setup");
    msg.param.set_cmd(SystemMessage::AutocryptSetupMessage);
    msg.force_plaintext();
    msg.param.set_int(Param::SkipAutocrypt, 1);

    chat::send_msg(context, chat_id, &mut msg).await?;
    // no maybe_add_bcc_self_device_msg() here.
    // the ui shows the dialog with the setup code on this device,
    // it would be too much noise to have two things popping up at the same time.
    // maybe_add_bcc_self_device_msg() is called on the other device
    // once the transfer is completed.
    Ok(setup_code)
}

/// Renders HTML body of a setup file message.
///
/// The `passphrase` must be at least 2 characters long.
pub async fn render_setup_file(context: &Context, passphrase: &str) -> Result<String> {
    let passphrase_begin = if let Some(passphrase_begin) = passphrase.get(..2) {
        passphrase_begin
    } else {
        bail!("Passphrase must be at least 2 chars long.");
    };
    let private_key = load_self_secret_key(context).await?;
    let ac_headers = match context.get_config_bool(Config::E2eeEnabled).await? {
        false => None,
        true => Some(("Autocrypt-Prefer-Encrypt", "mutual")),
    };
    let private_key_asc = private_key.to_asc(ac_headers);
    let encr = pgp::symm_encrypt(passphrase, private_key_asc.as_bytes()).await?;

    let replacement = format!(
        concat!(
            "-----BEGIN PGP MESSAGE-----\r\n",
            "Passphrase-Format: numeric9x4\r\n",
            "Passphrase-Begin: {}"
        ),
        passphrase_begin
    );
    let pgp_msg = encr.replace("-----BEGIN PGP MESSAGE-----", &replacement);

    let msg_subj = stock_str::ac_setup_msg_subject(context).await;
    let msg_body = stock_str::ac_setup_msg_body(context).await;
    let msg_body_html = msg_body.replace('\r', "").replace('\n', "<br>");
    Ok(format!(
        concat!(
            "<!DOCTYPE html>\r\n",
            "<html>\r\n",
            "  <head>\r\n",
            "    <title>{}</title>\r\n",
            "  </head>\r\n",
            "  <body>\r\n",
            "    <h1>{}</h1>\r\n",
            "    <p>{}</p>\r\n",
            "    <pre>\r\n{}\r\n</pre>\r\n",
            "  </body>\r\n",
            "</html>\r\n"
        ),
        msg_subj, msg_subj, msg_body_html, pgp_msg
    ))
}

/// Creates a new setup code for Autocrypt Setup Message.
pub fn create_setup_code(_context: &Context) -> String {
    let mut random_val: u16;
    let mut rng = thread_rng();
    let mut ret = String::new();

    for i in 0..9 {
        loop {
            random_val = rng.gen();
            if random_val as usize <= 60000 {
                break;
            }
        }
        random_val = (random_val as usize % 10000) as u16;
        ret += &format!(
            "{}{:04}",
            if 0 != i { "-" } else { "" },
            random_val as usize
        );
    }

    ret
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

/// Continue key transfer via Autocrypt Setup Message.
///
/// `msg_id` is the ID of the received Autocrypt Setup Message.
/// `setup_code` is the code entered by the user.
pub async fn continue_key_transfer(
    context: &Context,
    msg_id: MsgId,
    setup_code: &str,
) -> Result<()> {
    ensure!(!msg_id.is_special(), "wrong id");

    let msg = Message::load_from_db(context, msg_id).await?;
    ensure!(
        msg.is_setupmessage(),
        "Message is no Autocrypt Setup Message."
    );

    if let Some(filename) = msg.get_file(context) {
        let file = open_file_std(context, filename)?;
        let sc = normalize_setup_code(setup_code);
        let armored_key = decrypt_setup_file(&sc, file).await?;
        set_self_key(context, &armored_key, true, true).await?;
        maybe_add_bcc_self_device_msg(context).await?;

        Ok(())
    } else {
        bail!("Message is no Autocrypt Setup Message.");
    }
}

async fn set_self_key(
    context: &Context,
    armored: &str,
    set_default: bool,
    prefer_encrypt_required: bool,
) -> Result<()> {
    // try hard to only modify key-state
    let (private_key, header) = SignedSecretKey::from_asc(armored)?;
    let public_key = private_key.split_public_key()?;
    let preferencrypt = header.get("Autocrypt-Prefer-Encrypt");
    match preferencrypt.map(|s| s.as_str()) {
        Some(headerval) => {
            let e2ee_enabled = match headerval {
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
        }
        None => {
            if prefer_encrypt_required {
                bail!("missing Autocrypt-Prefer-Encrypt header");
            }
        }
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

async fn decrypt_setup_file<T: std::io::Read + std::io::Seek>(
    passphrase: &str,
    file: T,
) -> Result<String> {
    let plain_bytes = pgp::symm_decrypt(passphrase, file).await?;
    let plain_text = std::string::String::from_utf8(plain_bytes)?;

    Ok(plain_text)
}

fn normalize_setup_code(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c.is_ascii_digit() {
            out.push(c);
            if let 4 | 9 | 14 | 19 | 24 | 29 | 34 | 39 = out.len() {
                out += "-"
            }
        }
    }
    out
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
    let _d1 = DeleteOnDrop(temp_db_path.clone());
    let _d2 = DeleteOnDrop(temp_path.clone());

    export_database(context, &temp_db_path, passphrase, now)
        .await
        .context("could not export database")?;

    info!(
        context,
        "Backup '{}' to '{}'.",
        context.get_dbfile().display(),
        dest_path.display(),
    );

    let res = export_backup_inner(context, &temp_db_path, &temp_path).await;

    match &res {
        Ok(_) => {
            fs::rename(temp_path, &dest_path).await?;
            context.emit_event(EventType::ImexFileWritten(dest_path));
        }
        Err(e) => {
            error!(context, "backup failed: {}", e);
        }
    }

    res
}
struct DeleteOnDrop(PathBuf);
impl Drop for DeleteOnDrop {
    fn drop(&mut self) {
        let file = self.0.clone();
        // Not using `tools::delete_file` here because it would send a DeletedBlobFile event
        // Hack to avoid panic in nested runtime calls of tokio
        std::fs::remove_file(file).ok();
    }
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
    set_self_key(context, &armored, set_default, false).await?;
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

    for (id, public_key, private_key, is_default) in keys {
        let id = Some(id).filter(|_| is_default == 0);

        if let Ok(key) = public_key {
            if let Err(err) = export_key_to_asc_file(context, dir, id, &key).await {
                error!(context, "Failed to export public key: {:#}.", err);
                export_errors += 1;
            }
        } else {
            export_errors += 1;
        }
        if let Ok(key) = private_key {
            if let Err(err) = export_key_to_asc_file(context, dir, id, &key).await {
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

/*******************************************************************************
 * Classic key export
 ******************************************************************************/
async fn export_key_to_asc_file<T>(
    context: &Context,
    dir: &Path,
    id: Option<i64>,
    key: &T,
) -> Result<()>
where
    T: DcKey + Any,
{
    let file_name = {
        let any_key = key as &dyn Any;
        let kind = if any_key.downcast_ref::<SignedPublicKey>().is_some() {
            "public"
        } else if any_key.downcast_ref::<SignedSecretKey>().is_some() {
            "private"
        } else {
            "unknown"
        };
        let id = id.map_or("default".into(), |i| i.to_string());
        dir.join(format!("{}-key-{}.asc", kind, &id))
    };
    info!(
        context,
        "Exporting key {:?} to {}",
        key.key_id(),
        file_name.display()
    );

    // Delete the file if it already exists.
    delete_file(context, &file_name).await.ok();

    let content = key.to_asc(None).into_bytes();
    write_file(context, &file_name, &content)
        .await
        .with_context(|| format!("cannot write key to {}", file_name.display()))?;
    context.emit_event(EventType::ImexFileWritten(file_name));
    Ok(())
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

/// Serializes the database to a file.
pub async fn serialize_database(context: &Context, filename: &str) -> Result<()> {
    let file = File::create(filename).await?;
    context.sql.serialize(BufWriter::new(file)).await?;
    Ok(())
}

/// Deserializes the database from a file.
pub async fn deserialize_database(context: &Context, filename: &str) -> Result<()> {
    let file = File::open(filename).await?;
    context.sql.deserialize(file).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use ::pgp::armor::BlockType;
    use tokio::task;

    use super::*;
    use crate::pgp::{split_armored_data, HEADER_AUTOCRYPT, HEADER_SETUPCODE};
    use crate::stock_str::StockMessage;
    use crate::test_utils::{alice_keypair, TestContext, TestContextManager};

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_render_setup_file() {
        let t = TestContext::new_alice().await;
        let msg = render_setup_file(&t, "hello").await.unwrap();
        println!("{}", &msg);
        // Check some substrings, indicating things got substituted.
        // In particular note the mixing of `\r\n` and `\n` depending
        // on who generated the strings.
        assert!(msg.contains("<title>Autocrypt Setup Message</title"));
        assert!(msg.contains("<h1>Autocrypt Setup Message</h1>"));
        assert!(msg.contains("<p>This is the Autocrypt Setup Message used to"));
        assert!(msg.contains("-----BEGIN PGP MESSAGE-----\r\n"));
        assert!(msg.contains("Passphrase-Format: numeric9x4\r\n"));
        assert!(msg.contains("Passphrase-Begin: he\n"));
        assert!(msg.contains("-----END PGP MESSAGE-----\n"));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_render_setup_file_newline_replace() {
        let t = TestContext::new_alice().await;
        t.set_stock_translation(StockMessage::AcSetupMsgBody, "hello\r\nthere".to_string())
            .await
            .unwrap();
        let msg = render_setup_file(&t, "pw").await.unwrap();
        println!("{}", &msg);
        assert!(msg.contains("<p>hello<br>there</p>"));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_create_setup_code() {
        let t = TestContext::new().await;
        let setupcode = create_setup_code(&t);
        assert_eq!(setupcode.len(), 44);
        assert_eq!(setupcode.chars().nth(4).unwrap(), '-');
        assert_eq!(setupcode.chars().nth(9).unwrap(), '-');
        assert_eq!(setupcode.chars().nth(14).unwrap(), '-');
        assert_eq!(setupcode.chars().nth(19).unwrap(), '-');
        assert_eq!(setupcode.chars().nth(24).unwrap(), '-');
        assert_eq!(setupcode.chars().nth(29).unwrap(), '-');
        assert_eq!(setupcode.chars().nth(34).unwrap(), '-');
        assert_eq!(setupcode.chars().nth(39).unwrap(), '-');
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_export_public_key_to_asc_file() {
        let context = TestContext::new().await;
        let key = alice_keypair().public;
        let blobdir = Path::new("$BLOBDIR");
        assert!(export_key_to_asc_file(&context.ctx, blobdir, None, &key)
            .await
            .is_ok());
        let blobdir = context.ctx.get_blobdir().to_str().unwrap();
        let filename = format!("{blobdir}/public-key-default.asc");
        let bytes = tokio::fs::read(&filename).await.unwrap();

        assert_eq!(bytes, key.to_asc(None).into_bytes());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_export_private_key_to_asc_file() {
        let context = TestContext::new().await;
        let key = alice_keypair().secret;
        let blobdir = Path::new("$BLOBDIR");
        assert!(export_key_to_asc_file(&context.ctx, blobdir, None, &key)
            .await
            .is_ok());
        let blobdir = context.ctx.get_blobdir().to_str().unwrap();
        let filename = format!("{blobdir}/private-key-default.asc");
        let bytes = tokio::fs::read(&filename).await.unwrap();

        assert_eq!(bytes, key.to_asc(None).into_bytes());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_export_and_import_key() {
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

        let keyfile = export_dir.path().join("private-key-default.asc");
        let context3 = TestContext::new_alice().await;
        if let Err(err) = imex(&context3.ctx, ImexMode::ImportSelfKeys, &keyfile, None).await {
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

    #[test]
    fn test_normalize_setup_code() {
        let norm = normalize_setup_code("123422343234423452346234723482349234");
        assert_eq!(norm, "1234-2234-3234-4234-5234-6234-7234-8234-9234");

        let norm =
            normalize_setup_code("\t1 2 3422343234- foo bar-- 423-45 2 34 6234723482349234      ");
        assert_eq!(norm, "1234-2234-3234-4234-5234-6234-7234-8234-9234");
    }

    /* S_EM_SETUPFILE is a AES-256 symm. encrypted setup message created by Enigmail
    with an "encrypted session key", see RFC 4880.  The code is in S_EM_SETUPCODE */
    const S_EM_SETUPCODE: &str = "1742-0185-6197-1303-7016-8412-3581-4441-0597";
    const S_EM_SETUPFILE: &str = include_str!("../test-data/message/stress.txt");

    // Autocrypt Setup Message payload "encrypted" with plaintext algorithm.
    const S_PLAINTEXT_SETUPFILE: &str =
        include_str!("../test-data/message/plaintext-autocrypt-setup.txt");

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_split_and_decrypt() {
        let buf_1 = S_EM_SETUPFILE.as_bytes().to_vec();
        let (typ, headers, base64) = split_armored_data(&buf_1).unwrap();
        assert_eq!(typ, BlockType::Message);
        assert!(S_EM_SETUPCODE.starts_with(headers.get(HEADER_SETUPCODE).unwrap()));
        assert!(!headers.contains_key(HEADER_AUTOCRYPT));

        assert!(!base64.is_empty());

        let setup_file = S_EM_SETUPFILE.to_string();
        let decrypted =
            decrypt_setup_file(S_EM_SETUPCODE, std::io::Cursor::new(setup_file.as_bytes()))
                .await
                .unwrap();

        let (typ, headers, _base64) = split_armored_data(decrypted.as_bytes()).unwrap();

        assert_eq!(typ, BlockType::PrivateKey);
        assert_eq!(headers.get(HEADER_AUTOCRYPT), Some(&"mutual".to_string()));
        assert!(!headers.contains_key(HEADER_SETUPCODE));
    }

    /// Tests that Autocrypt Setup Message encrypted with "plaintext" algorithm cannot be
    /// decrypted.
    ///
    /// According to <https://datatracker.ietf.org/doc/html/rfc4880#section-13.4>
    /// "Implementations MUST NOT use plaintext in Symmetrically Encrypted Data packets".
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_decrypt_plaintext_autocrypt_setup_message() {
        let setup_file = S_PLAINTEXT_SETUPFILE.to_string();
        let incorrect_setupcode = "0000-0000-0000-0000-0000-0000-0000-0000-0000";
        assert!(decrypt_setup_file(
            incorrect_setupcode,
            std::io::Cursor::new(setup_file.as_bytes()),
        )
        .await
        .is_err());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_key_transfer() -> Result<()> {
        let alice = TestContext::new_alice().await;

        let setup_code = initiate_key_transfer(&alice).await?;

        // Get Autocrypt Setup Message.
        let sent = alice.pop_sent_msg().await;

        // Alice sets up a second device.
        let alice2 = TestContext::new().await;
        alice2.set_name("alice2");
        alice2.configure_addr("alice@example.org").await;
        alice2.recv_msg(&sent).await;
        let msg = alice2.get_last_msg().await;
        assert!(msg.is_setupmessage());

        // Send a message that cannot be decrypted because the keys are
        // not synchronized yet.
        let sent = alice2.send_text(msg.chat_id, "Test").await;
        alice.recv_msg(&sent).await;
        assert_ne!(alice.get_last_msg().await.get_text(), "Test");

        // Transfer the key.
        continue_key_transfer(&alice2, msg.id, &setup_code).await?;

        // Alice sends a message to self from the new device.
        let sent = alice2.send_text(msg.chat_id, "Test").await;
        alice.recv_msg(&sent).await;
        assert_eq!(alice.get_last_msg().await.get_text(), "Test");

        Ok(())
    }

    /// Tests that Autocrypt Setup Messages is only clickable if it is self-sent.
    /// This prevents Bob from tricking Alice into changing the key
    /// by sending her an Autocrypt Setup Message as long as Alice's server
    /// does not allow to forge the `From:` header.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_key_transfer_non_self_sent() -> Result<()> {
        let mut tcm = TestContextManager::new();
        let alice = tcm.alice().await;
        let bob = tcm.bob().await;

        let _setup_code = initiate_key_transfer(&alice).await?;

        // Get Autocrypt Setup Message.
        let sent = alice.pop_sent_msg().await;

        let rcvd = bob.recv_msg(&sent).await;
        assert!(!rcvd.is_setupmessage());

        Ok(())
    }
}
