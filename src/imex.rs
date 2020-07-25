//! # Import/export module

use std::any::Any;
use std::{
    cmp::{max, min},
    ffi::OsStr,
};

use async_std::path::{Path, PathBuf};
use async_std::{
    fs::{self, File},
    prelude::*,
};
use rand::{thread_rng, Rng};

use crate::blob::BlobObject;
use crate::chat;
use crate::chat::delete_and_reset_all_device_msgs;
use crate::config::Config;
use crate::constants::*;
use crate::context::Context;
use crate::dc_tools::*;
use crate::e2ee;
use crate::error::*;
use crate::events::Event;
use crate::key::{self, DcKey, DcSecretKey, SignedPublicKey, SignedSecretKey};
use crate::message::{Message, MsgId};
use crate::mimeparser::SystemMessage;
use crate::param::*;
use crate::pgp;
use crate::sql::{self, Sql};
use crate::stock::StockMessage;
use async_tar::Archive;

// Name of the database file in the backup.
const DBFILE_BACKUP_NAME: &str = "dc_database_backup.sqlite";
const BLOBS_BACKUP_NAME: &str = "blobs_backup";

#[derive(Debug, Display, Copy, Clone, PartialEq, Eq, FromPrimitive, ToPrimitive)]
#[repr(i32)]
pub enum ImexMode {
    /// Export all private keys and all public keys of the user to the
    /// directory given as `param1`.  The default key is written to the files `public-key-default.asc`
    /// and `private-key-default.asc`, if there are more keys, they are written to files as
    /// `public-key-<id>.asc` and `private-key-<id>.asc`
    ExportSelfKeys = 1,

    /// Import private keys found in the directory given as `param1`.
    /// The last imported key is made the default keys unless its name contains the string `legacy`.
    /// Public keys are not imported.
    ImportSelfKeys = 2,

    /// Export a backup to the directory given as `param1`.
    /// The backup contains all contacts, chats, images and other data and device independent settings.
    /// The backup does not contain device dependent settings as ringtones or LED notification settings.
    /// The name of the backup is typically `delta-chat.<day>.tar`, if more than one backup is create on a day,
    /// the format is `delta-chat.<day>-<number>.tar`
    ExportBackup = 11,

    /// `param1` is the file (not: directory) to import. The file is normally
    /// created by DC_IMEX_EXPORT_BACKUP and detected by dc_imex_has_backup(). Importing a backup
    /// is only possible as long as the context is not configured or used in another way.
    ImportBackup = 12,
}

/// Import/export things.
///
/// What to do is defined by the *what* parameter.
///
/// During execution of the job,
/// some events are sent out:
///
/// - A number of #DC_EVENT_IMEX_PROGRESS events are sent and may be used to create
///   a progress bar or stuff like that. Moreover, you'll be informed when the imex-job is done.
///
/// - For each file written on export, the function sends #DC_EVENT_IMEX_FILE_WRITTEN
///
/// Only one import-/export-progress can run at the same time.
/// To cancel an import-/export-progress, drop the future returned by this function.
pub async fn imex(
    context: &Context,
    what: ImexMode,
    param1: Option<impl AsRef<Path>>,
) -> Result<()> {
    use futures::future::FutureExt;

    let cancel = context.alloc_ongoing().await?;
    let res = imex_inner(context, what, param1)
        .race(cancel.recv().map(|_| Err(format_err!("canceled"))))
        .await;

    context.free_ongoing().await;

    res
}

/// Returns the filename of the backup found (otherwise an error)
pub async fn has_backup(context: &Context, dir_name: impl AsRef<Path>) -> Result<String> {
    let dir_name = dir_name.as_ref();
    let mut dir_iter = async_std::fs::read_dir(dir_name).await?;
    let mut newest_backup_name = "".to_string();
    let mut newest_backup_path: Option<PathBuf> = None;

    while let Some(dirent) = dir_iter.next().await {
        if let Ok(dirent) = dirent {
            let path = dirent.path();
            let name = dirent.file_name();
            let name: String = name.to_string_lossy().into();
            if name.starts_with("delta-chat") && name.ends_with(".tar") {
                if newest_backup_name.is_empty() || name > newest_backup_name {
                    // We just use string comparison to determine which backup is newer.
                    // This works fine because the filenames have the form ...delta-chat-backup-2020-07-24-00.tar
                    newest_backup_path = Some(path);
                    newest_backup_name = name;
                }
            }
        }
    }

    match newest_backup_path {
        Some(path) => Ok(path.to_string_lossy().into_owned()),
        None => has_backup_old(context, dir_name).await,
        // When we decide to remove support for .bak backups, we can replace this with `None => bail!("no backup found in {}", dir_name.display()),`.
    }
}

/// Returns the filename of the backup found (otherwise an error)
pub async fn has_backup_old(context: &Context, dir_name: impl AsRef<Path>) -> Result<String> {
    let dir_name = dir_name.as_ref();
    let mut dir_iter = async_std::fs::read_dir(dir_name).await?;
    let mut newest_backup_time = 0;
    let mut newest_backup_path: Option<PathBuf> = None;
    while let Some(dirent) = dir_iter.next().await {
        if let Ok(dirent) = dirent {
            let path = dirent.path();
            let name = dirent.file_name();
            let name = name.to_string_lossy();
            if name.starts_with("delta-chat") && name.ends_with(".bak") {
                let sql = Sql::new();
                if sql.open(context, &path, true).await {
                    let curr_backup_time = sql
                        .get_raw_config_int(context, "backup_time")
                        .await
                        .unwrap_or_default();
                    if curr_backup_time > newest_backup_time {
                        newest_backup_path = Some(path);
                        newest_backup_time = curr_backup_time;
                    }
                    info!(context, "backup_time of {} is {}", name, curr_backup_time);
                    sql.close().await;
                }
            }
        }
    }
    match newest_backup_path {
        Some(path) => Ok(path.to_string_lossy().into_owned()),
        None => bail!("no backup found in {}", dir_name.display()),
    }
}

pub async fn initiate_key_transfer(context: &Context) -> Result<String> {
    use futures::future::FutureExt;

    let cancel = context.alloc_ongoing().await?;
    let res = do_initiate_key_transfer(context)
        .race(cancel.recv().map(|_| Err(format_err!("canceled"))))
        .await;

    context.free_ongoing().await;
    res
}

async fn do_initiate_key_transfer(context: &Context) -> Result<String> {
    let mut msg: Message;
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

    let chat_id = chat::create_by_contact_id(context, DC_CONTACT_ID_SELF).await?;
    msg = Message::default();
    msg.viewtype = Viewtype::File;
    msg.param.set(Param::File, setup_file_blob.as_name());

    msg.param
        .set(Param::MimeType, "application/autocrypt-setup");
    msg.param.set_cmd(SystemMessage::AutocryptSetupMessage);
    msg.param.set_int(
        Param::ForcePlaintext,
        ForcePlaintext::NoAutocryptHeader as i32,
    );

    let msg_id = chat::send_msg(context, chat_id, &mut msg).await?;
    info!(context, "Wait for setup message being sent ...",);
    while !context.shall_stop_ongoing().await {
        async_std::task::sleep(std::time::Duration::from_secs(1)).await;
        if let Ok(msg) = Message::load_from_db(context, msg_id).await {
            if msg.is_sent() {
                info!(context, "... setup message sent.",);
                break;
            }
        }
    }
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
    let private_key = SignedSecretKey::load_self(context).await?;
    let ac_headers = match context.get_config_bool(Config::E2eeEnabled).await {
        false => None,
        true => Some(("Autocrypt-Prefer-Encrypt", "mutual")),
    };
    let private_key_asc = private_key.to_asc(ac_headers);
    let encr = pgp::symm_encrypt(&passphrase, private_key_asc.as_bytes()).await?;

    let replacement = format!(
        concat!(
            "-----BEGIN PGP MESSAGE-----\r\n",
            "Passphrase-Format: numeric9x4\r\n",
            "Passphrase-Begin: {}"
        ),
        passphrase_begin
    );
    let pgp_msg = encr.replace("-----BEGIN PGP MESSAGE-----", &replacement);

    let msg_subj = context.stock_str(StockMessage::AcSetupMsgSubject).await;
    let msg_body = context.stock_str(StockMessage::AcSetupMsgBody).await;
    let msg_body_html = msg_body.replace("\r", "").replace("\n", "<br>");
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
    if !context.sql.get_raw_config_bool(context, "bcc_self").await {
        let mut msg = Message::new(Viewtype::Text);
        // TODO: define this as a stockstring once the wording is settled.
        msg.text = Some(
            "It seems you are using multiple devices with Delta Chat. Great!\n\n\
             If you also want to synchronize outgoing messages accross all devices, \
             go to the settings and enable \"Send copy to self\"."
                .to_string(),
        );
        chat::add_device_msg(context, Some("bcc-self-hint"), Some(&mut msg)).await?;
    }
    Ok(())
}

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
        let file = dc_open_file_std(context, filename)?;
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
                .set_raw_config_int(context, "e2ee_enabled", e2ee_enabled)
                .await?;
        }
        None => {
            if prefer_encrypt_required {
                bail!("missing Autocrypt-Prefer-Encrypt header");
            }
        }
    };

    let self_addr = context.get_config(Config::ConfiguredAddr).await;
    ensure!(self_addr.is_some(), "Missing self addr");
    let addr = EmailAddress::new(&self_addr.unwrap_or_default())?;
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

pub fn normalize_setup_code(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c >= '0' && c <= '9' {
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
    param: Option<impl AsRef<Path>>,
) -> Result<()> {
    ensure!(param.is_some(), "No Import/export dir/file given.");

    info!(context, "Import/export process started.");
    context.emit_event(Event::ImexProgress(10));

    ensure!(context.sql.is_open().await, "Database not opened.");

    let path = param.ok_or_else(|| format_err!("Imex: Param was None"))?;
    if what == ImexMode::ExportBackup || what == ImexMode::ExportSelfKeys {
        // before we export anything, make sure the private key exists
        if e2ee::ensure_secret_key_exists(context).await.is_err() {
            bail!("Cannot create private key or private key not available.");
        } else {
            dc_create_folder(context, &path).await?;
        }
    }

    let success = match what {
        ImexMode::ExportSelfKeys => export_self_keys(context, path).await,
        ImexMode::ImportSelfKeys => import_self_keys(context, path).await,

        // TODO In some months we can change the export_backup_old() call to export_backup() and delete export_backup_old().
        // (now is 07/2020)
        ImexMode::ExportBackup => export_backup_old(context, path).await,
        // import_backup() will call import_backup_old() if this is an old backup.
        ImexMode::ImportBackup => import_backup(context, path).await,
    };

    match success {
        Ok(()) => {
            info!(context, "IMEX successfully completed");
            context.emit_event(Event::ImexProgress(1000));
            Ok(())
        }
        Err(err) => {
            context.emit_event(Event::ImexProgress(0));
            bail!("IMEX FAILED to complete: {}", err);
        }
    }
}

/// Import Backup
async fn import_backup(context: &Context, backup_to_import: impl AsRef<Path>) -> Result<()> {
    if backup_to_import
        .as_ref()
        .to_string_lossy()
        .ends_with(".bak")
    {
        // Backwards compability
        return import_backup_old(context, backup_to_import).await;
    }

    info!(
        context,
        "Import \"{}\" to \"{}\".",
        backup_to_import.as_ref().display(),
        context.get_dbfile().display()
    );

    ensure!(
        !context.is_configured().await,
        "Cannot import backups to accounts in use."
    );
    context.sql.close().await;
    dc_delete_file(context, context.get_dbfile()).await;
    ensure!(
        !context.get_dbfile().exists().await,
        "Cannot delete old database."
    );

    let backup_file = File::open(backup_to_import).await?;
    let mut archive = Archive::new(backup_file);
    let mut entries = archive.entries()?;
    while let Some(file) = entries.next().await {
        let f = &mut file?;
        if f.path()?.file_name() == Some(OsStr::new(DBFILE_BACKUP_NAME)) {
            // async_tar can't unpack to a specified file name, so we just unpack to the blobdir and then move the unpacked file.
            f.unpack_in(context.get_blobdir()).await?;
            fs::rename(
                context.get_blobdir().join(DBFILE_BACKUP_NAME),
                context.get_dbfile(),
            )
            .await?;
            context.emit_event(Event::ImexProgress(400)); // Just guess the progress, we at least have the dbfile by now
        } else {
            // async_tar will unpack to blobdir/BLOBS_BACKUP_NAME, so we move the file afterwards.
            f.unpack_in(context.get_blobdir()).await?;
            let from_path = context.get_blobdir().join(f.path()?);
            if from_path.is_file().await {
                if let Some(name) = from_path.file_name() {
                    fs::rename(&from_path, context.get_blobdir().join(name)).await?;
                } else {
                    warn!(context, "No file name");
                }
            }
        }
    }

    ensure!(
        context
            .sql
            .open(&context, &context.get_dbfile(), false)
            .await,
        "could not re-open db"
    );

    delete_and_reset_all_device_msgs(&context).await?;

    Ok(())
}

async fn import_backup_old(context: &Context, backup_to_import: impl AsRef<Path>) -> Result<()> {
    info!(
        context,
        "Import \"{}\" to \"{}\".",
        backup_to_import.as_ref().display(),
        context.get_dbfile().display()
    );

    ensure!(
        !context.is_configured().await,
        "Cannot import backups to accounts in use."
    );
    context.sql.close().await;
    dc_delete_file(context, context.get_dbfile()).await;
    ensure!(
        !context.get_dbfile().exists().await,
        "Cannot delete old database."
    );

    ensure!(
        dc_copy_file(context, backup_to_import.as_ref(), context.get_dbfile()).await,
        "could not copy file"
    );
    /* error already logged */
    /* re-open copied database file */
    ensure!(
        context
            .sql
            .open(&context, &context.get_dbfile(), false)
            .await,
        "could not re-open db"
    );

    delete_and_reset_all_device_msgs(&context).await?;

    let total_files_cnt = context
        .sql
        .query_get_value::<isize>(context, "SELECT COUNT(*) FROM backup_blobs;", paramsv![])
        .await
        .unwrap_or_default() as usize;
    info!(
        context,
        "***IMPORT-in-progress: total_files_cnt={:?}", total_files_cnt,
    );

    // Load IDs only for now, without the file contents, to avoid
    // consuming too much memory.
    let file_ids = context
        .sql
        .query_map(
            "SELECT id FROM backup_blobs ORDER BY id",
            paramsv![],
            |row| row.get(0),
            |ids| {
                ids.collect::<std::result::Result<Vec<i64>, _>>()
                    .map_err(Into::into)
            },
        )
        .await?;

    let mut all_files_extracted = true;
    for (processed_files_cnt, file_id) in file_ids.into_iter().enumerate() {
        // Load a single blob into memory
        let (file_name, file_blob) = context
            .sql
            .query_row(
                "SELECT file_name, file_content FROM backup_blobs WHERE id = ?",
                paramsv![file_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?)),
            )
            .await?;

        if context.shall_stop_ongoing().await {
            all_files_extracted = false;
            break;
        }
        let mut permille = processed_files_cnt * 1000 / total_files_cnt;
        if permille < 10 {
            permille = 10
        }
        if permille > 990 {
            permille = 990
        }
        context.emit_event(Event::ImexProgress(permille));
        if file_blob.is_empty() {
            continue;
        }

        let path_filename = context.get_blobdir().join(file_name);
        dc_write_file(context, &path_filename, &file_blob).await?;
    }

    if all_files_extracted {
        // only delete backup_blobs if all files were successfully extracted
        context
            .sql
            .execute("DROP TABLE backup_blobs;", paramsv![])
            .await?;
        context.sql.execute("VACUUM;", paramsv![]).await.ok();
        Ok(())
    } else {
        bail!("received stop signal");
    }
}

/*******************************************************************************
 * Export backup
 ******************************************************************************/
#[allow(unused)]
async fn export_backup(context: &Context, dir: impl AsRef<Path>) -> Result<()> {
    // get a fine backup file name (the name includes the date so that multiple backup instances are possible)
    // FIXME: we should write to a temporary file first and rename it on success. this would guarantee the backup is complete.
    // let dest_path_filename = dc_get_next_backup_file(context, dir, res);
    let now = time();
    let (temp_path, dest_path) = dc_get_next_backup_path_new(dir, now).await?;

    context
        .sql
        .set_raw_config_int(context, "backup_time", now as i32)
        .await?;
    sql::housekeeping(context).await;

    context.sql.execute("VACUUM;", paramsv![]).await.ok();

    // we close the database during the export
    context.sql.close().await;

    info!(
        context,
        "Backup '{}' to '{}'.",
        context.get_dbfile().display(),
        dest_path.display(),
    );

    let res = export_backup_inner(context, &temp_path).await;

    context
        .sql
        .open(&context, &context.get_dbfile(), false)
        .await;

    match &res {
        Ok(_) => {
            fs::rename(temp_path, &dest_path).await?;
            context.emit_event(Event::ImexFileWritten(dest_path));
        }
        Err(e) => {
            error!(context, "backup failed: {}", e);
            // Not using dc_delete_file() here because it would send a DeletedBlobFile event
            fs::remove_file(temp_path).await?;
        }
    }

    res
}

async fn export_backup_inner(context: &Context, temp_path: &PathBuf) -> Result<()> {
    let file = File::create(temp_path).await?;

    let mut builder = async_tar::Builder::new(file);

    // append_path_with_name() wants the source path as the first argument, append_dir_all() wants it as the second argument.
    builder
        .append_path_with_name(context.get_dbfile(), DBFILE_BACKUP_NAME)
        .await?;

    context.emit_event(Event::ImexProgress(500));

    builder
        .append_dir_all(BLOBS_BACKUP_NAME, context.get_blobdir())
        .await?;

    builder.finish().await?;
    Ok(())
}

async fn export_backup_old(context: &Context, dir: impl AsRef<Path>) -> Result<()> {
    // get a fine backup file name (the name includes the date so that multiple backup instances are possible)
    // FIXME: we should write to a temporary file first and rename it on success. this would guarantee the backup is complete.
    // let dest_path_filename = dc_get_next_backup_file(context, dir, res);
    let now = time();
    let dest_path_filename = dc_get_next_backup_path_old(dir, now).await?;
    let dest_path_string = dest_path_filename.to_string_lossy().to_string();

    sql::housekeeping(context).await;

    context.sql.execute("VACUUM;", paramsv![]).await.ok();

    // we close the database during the copy of the dbfile
    context.sql.close().await;
    info!(
        context,
        "Backup '{}' to '{}'.",
        context.get_dbfile().display(),
        dest_path_filename.display(),
    );
    let copied = dc_copy_file(context, context.get_dbfile(), &dest_path_filename).await;
    context
        .sql
        .open(&context, &context.get_dbfile(), false)
        .await;

    if !copied {
        bail!(
            "could not copy file from '{}' to '{}'",
            context.get_dbfile().display(),
            dest_path_string
        );
    }
    let dest_sql = Sql::new();
    ensure!(
        dest_sql.open(context, &dest_path_filename, false).await,
        "could not open exported database {}",
        dest_path_string
    );
    let res = match add_files_to_export(context, &dest_sql).await {
        Err(err) => {
            dc_delete_file(context, &dest_path_filename).await;
            error!(context, "backup failed: {}", err);
            Err(err)
        }
        Ok(()) => {
            dest_sql
                .set_raw_config_int(context, "backup_time", now as i32)
                .await?;
            context.emit_event(Event::ImexFileWritten(dest_path_filename));
            Ok(())
        }
    };
    dest_sql.close().await;

    Ok(res?)
}

async fn add_files_to_export(context: &Context, sql: &Sql) -> Result<()> {
    // add all files as blobs to the database copy (this does not require
    // the source to be locked, neigher the destination as it is used only here)
    if !sql.table_exists("backup_blobs").await? {
        sql.execute(
            "CREATE TABLE backup_blobs (id INTEGER PRIMARY KEY, file_name, file_content);",
            paramsv![],
        )
        .await?;
    }
    // copy all files from BLOBDIR into backup-db
    let mut total_files_cnt = 0;
    let dir = context.get_blobdir();
    let dir_handle = async_std::fs::read_dir(&dir).await?;
    total_files_cnt += dir_handle.filter(|r| r.is_ok()).count().await;

    info!(context, "EXPORT: total_files_cnt={}", total_files_cnt);

    sql.with_conn_async(|conn| async move {
        // scan directory, pass 2: copy files
        let mut dir_handle = async_std::fs::read_dir(&dir).await?;

        let mut processed_files_cnt = 0;
        while let Some(entry) = dir_handle.next().await {
            let entry = entry?;
            if context.shall_stop_ongoing().await {
                return Ok(());
            }
            processed_files_cnt += 1;
            let permille = max(min(processed_files_cnt * 1000 / total_files_cnt, 990), 10);
            context.emit_event(Event::ImexProgress(permille));

            let name_f = entry.file_name();
            let name = name_f.to_string_lossy();
            if name.starts_with("delta-chat") && name.ends_with(".bak") {
                continue;
            }
            info!(context, "EXPORT: copying filename={}", name);
            let curr_path_filename = context.get_blobdir().join(entry.file_name());
            if let Ok(buf) = dc_read_file(context, &curr_path_filename).await {
                if buf.is_empty() {
                    continue;
                }
                // bail out if we can't insert
                let mut stmt = conn.prepare_cached(
                    "INSERT INTO backup_blobs (file_name, file_content) VALUES (?, ?);",
                )?;
                stmt.execute(paramsv![name, buf])?;
            }
        }
        Ok(())
    })
    .await?;

    Ok(())
}

/*******************************************************************************
 * Classic key import
 ******************************************************************************/
async fn import_self_keys(context: &Context, dir: impl AsRef<Path>) -> Result<()> {
    /* hint: even if we switch to import Autocrypt Setup Files, we should leave the possibility to import
    plain ASC keys, at least keys without a password, if we do not want to implement a password entry function.
    Importing ASC keys is useful to use keys in Delta Chat used by any other non-Autocrypt-PGP implementation.

    Maybe we should make the "default" key handlong also a little bit smarter
    (currently, the last imported key is the standard key unless it contains the string "legacy" in its name) */
    let mut set_default: bool;
    let mut imported_cnt = 0;

    let dir_name = dir.as_ref().to_string_lossy();
    let mut dir_handle = async_std::fs::read_dir(&dir).await?;
    while let Some(entry) = dir_handle.next().await {
        let entry_fn = entry?.file_name();
        let name_f = entry_fn.to_string_lossy();
        let path_plus_name = dir.as_ref().join(&entry_fn);
        match dc_get_filesuffix_lc(&name_f) {
            Some(suffix) => {
                if suffix != "asc" {
                    continue;
                }
                set_default = if name_f.contains("legacy") {
                    info!(context, "found legacy key '{}'", path_plus_name.display());
                    false
                } else {
                    true
                }
            }
            None => {
                continue;
            }
        }
        match dc_read_file(context, &path_plus_name).await {
            Ok(buf) => {
                let armored = std::string::String::from_utf8_lossy(&buf);
                if let Err(err) = set_self_key(context, &armored, set_default, false).await {
                    error!(context, "set_self_key: {}", err);
                    continue;
                }
            }
            Err(_) => continue,
        }
        imported_cnt += 1;
    }
    ensure!(
        imported_cnt > 0,
        "No private keys found in \"{}\".",
        dir_name
    );
    Ok(())
}

async fn export_self_keys(context: &Context, dir: impl AsRef<Path>) -> Result<()> {
    let mut export_errors = 0;

    let keys = context
        .sql
        .query_map(
            "SELECT id, public_key, private_key, is_default FROM keypairs;",
            paramsv![],
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
        let id = Some(id).filter(|_| is_default != 0);
        if let Ok(key) = public_key {
            if export_key_to_asc_file(context, &dir, id, &key)
                .await
                .is_err()
            {
                export_errors += 1;
            }
        } else {
            export_errors += 1;
        }
        if let Ok(key) = private_key {
            if export_key_to_asc_file(context, &dir, id, &key)
                .await
                .is_err()
            {
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
    dir: impl AsRef<Path>,
    id: Option<i64>,
    key: &T,
) -> std::io::Result<()>
where
    T: DcKey + Any,
{
    let file_name = {
        let any_key = key as &dyn Any;
        let kind = if any_key.downcast_ref::<SignedPublicKey>().is_some() {
            "public"
        } else if any_key.downcast_ref::<SignedPublicKey>().is_some() {
            "private"
        } else {
            "unknown"
        };
        let id = id.map_or("default".into(), |i| i.to_string());
        dir.as_ref().join(format!("{}-key-{}.asc", kind, &id))
    };
    info!(context, "Exporting key {}", file_name.display());
    dc_delete_file(context, &file_name).await;

    let content = key.to_asc(None).into_bytes();
    let res = dc_write_file(context, &file_name, &content).await;
    if res.is_err() {
        error!(context, "Cannot write key to {}", file_name.display());
    } else {
        context.emit_event(Event::ImexFileWritten(file_name));
    }
    res
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pgp::{split_armored_data, HEADER_AUTOCRYPT, HEADER_SETUPCODE};
    use crate::test_utils::*;
    use ::pgp::armor::BlockType;

    #[async_std::test]
    async fn test_render_setup_file() {
        let t = TestContext::new().await;

        t.configure_alice().await;
        let msg = render_setup_file(&t.ctx, "hello").await.unwrap();
        println!("{}", &msg);
        // Check some substrings, indicating things got substituted.
        // In particular note the mixing of `\r\n` and `\n` depending
        // on who generated the stings.
        assert!(msg.contains("<title>Autocrypt Setup Message</title"));
        assert!(msg.contains("<h1>Autocrypt Setup Message</h1>"));
        assert!(msg.contains("<p>This is the Autocrypt Setup Message used to"));
        assert!(msg.contains("-----BEGIN PGP MESSAGE-----\r\n"));
        assert!(msg.contains("Passphrase-Format: numeric9x4\r\n"));
        assert!(msg.contains("Passphrase-Begin: he\n"));
        assert!(msg.contains("-----END PGP MESSAGE-----\n"));
    }

    #[async_std::test]
    async fn test_render_setup_file_newline_replace() {
        let t = TestContext::new().await;
        t.ctx
            .set_stock_translation(StockMessage::AcSetupMsgBody, "hello\r\nthere".to_string())
            .await
            .unwrap();
        t.configure_alice().await;
        let msg = render_setup_file(&t.ctx, "pw").await.unwrap();
        println!("{}", &msg);
        assert!(msg.contains("<p>hello<br>there</p>"));
    }

    #[async_std::test]
    async fn test_create_setup_code() {
        let t = TestContext::new().await;
        let setupcode = create_setup_code(&t.ctx);
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

    #[async_std::test]
    async fn test_export_key_to_asc_file() {
        let context = TestContext::new().await;
        let key = alice_keypair().public;
        let blobdir = "$BLOBDIR";
        assert!(export_key_to_asc_file(&context.ctx, blobdir, None, &key)
            .await
            .is_ok());
        let blobdir = context.ctx.get_blobdir().to_str().unwrap();
        let filename = format!("{}/public-key-default.asc", blobdir);
        let bytes = async_std::fs::read(&filename).await.unwrap();

        assert_eq!(bytes, key.to_asc(None).into_bytes());
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

    #[async_std::test]
    async fn test_split_and_decrypt() {
        let buf_1 = S_EM_SETUPFILE.as_bytes().to_vec();
        let (typ, headers, base64) = split_armored_data(&buf_1).unwrap();
        assert_eq!(typ, BlockType::Message);
        assert!(S_EM_SETUPCODE.starts_with(headers.get(HEADER_SETUPCODE).unwrap()));
        assert!(headers.get(HEADER_AUTOCRYPT).is_none());

        assert!(!base64.is_empty());

        let setup_file = S_EM_SETUPFILE.to_string();
        let decrypted =
            decrypt_setup_file(S_EM_SETUPCODE, std::io::Cursor::new(setup_file.as_bytes()))
                .await
                .unwrap();

        let (typ, headers, _base64) = split_armored_data(decrypted.as_bytes()).unwrap();

        assert_eq!(typ, BlockType::PrivateKey);
        assert_eq!(headers.get(HEADER_AUTOCRYPT), Some(&"mutual".to_string()));
        assert!(headers.get(HEADER_SETUPCODE).is_none());
    }
}
