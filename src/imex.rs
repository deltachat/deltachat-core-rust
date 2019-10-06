use core::cmp::{max, min};
use std::ffi::CString;
use std::path::{Path, PathBuf};
use std::ptr;

use num_traits::FromPrimitive;
use rand::{thread_rng, Rng};

use crate::chat;
use crate::config::Config;
use crate::configure::*;
use crate::constants::*;
use crate::context::Context;
use crate::dc_tools::*;
use crate::e2ee;
use crate::error::*;
use crate::events::Event;
use crate::job::*;
use crate::key::*;
use crate::message::Message;
use crate::param::*;
use crate::pgp::*;
use crate::sql::{self, Sql};
use crate::stock::StockMessage;

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
    /// The name of the backup is typically `delta-chat.<day>.bak`, if more than one backup is create on a day,
    /// the format is `delta-chat.<day>-<number>.bak`
    ExportBackup = 11,
    /// `param1` is the file (not: directory) to import. The file is normally
    /// created by DC_IMEX_EXPORT_BACKUP and detected by dc_imex_has_backup(). Importing a backup
    /// is only possible as long as the context is not configured or used in another way.
    ImportBackup = 12,
}

/// Import/export things.
/// For this purpose, the function creates a job that is executed in the IMAP-thread then;
/// this requires to call dc_perform_imap_jobs() regularly.
///
/// What to do is defined by the _what_ parameter.
///
/// While dc_imex() returns immediately, the started job may take a while,
/// you can stop it using dc_stop_ongoing_process(). During execution of the job,
/// some events are sent out:
///
/// - A number of #DC_EVENT_IMEX_PROGRESS events are sent and may be used to create
///   a progress bar or stuff like that. Moreover, you'll be informed when the imex-job is done.
///
/// - For each file written on export, the function sends #DC_EVENT_IMEX_FILE_WRITTEN
///
/// Only one import-/export-progress can run at the same time.
/// To cancel an import-/export-progress, use dc_stop_ongoing_process().
pub fn imex(context: &Context, what: ImexMode, param1: Option<impl AsRef<Path>>) {
    let mut param = Params::new();
    param.set_int(Param::Cmd, what as i32);
    if let Some(param1) = param1 {
        param.set(Param::Arg, param1.as_ref().to_string_lossy());
    }

    job_kill_action(context, Action::ImexImap);
    job_add(context, Action::ImexImap, 0, param, 0);
}

/// Returns the filename of the backup if found, nullptr otherwise.
pub fn has_backup(context: &Context, dir_name: impl AsRef<Path>) -> Result<String> {
    let dir_name = dir_name.as_ref();
    let dir_iter = std::fs::read_dir(dir_name)?;
    let mut newest_backup_time = 0;
    let mut newest_backup_path: Option<std::path::PathBuf> = None;
    for dirent in dir_iter {
        match dirent {
            Ok(dirent) => {
                let path = dirent.path();
                let name = dirent.file_name();
                let name = name.to_string_lossy();
                if name.starts_with("delta-chat") && name.ends_with(".bak") {
                    let sql = Sql::new();
                    if sql.open(context, &path, 0x1) {
                        let curr_backup_time =
                            sql.get_raw_config_int(context, "backup_time")
                                .unwrap_or_default() as u64;
                        if curr_backup_time > newest_backup_time {
                            newest_backup_path = Some(path);
                            newest_backup_time = curr_backup_time;
                        }
                    }
                }
            }
            Err(_) => (),
        }
    }
    match newest_backup_path {
        Some(path) => Ok(path.to_string_lossy().into_owned()),
        None => bail!("no backup found"),
    }
}

pub fn initiate_key_transfer(context: &Context) -> Result<String> {
    ensure!(context.alloc_ongoing(), "could not allocate ongoing");
    let res = do_initiate_key_transfer(context);
    context.free_ongoing();
    res
}

fn do_initiate_key_transfer(context: &Context) -> Result<String> {
    let mut msg: Message;
    let setup_code = create_setup_code(context);
    /* this may require a keypair to be created. this may take a second ... */
    ensure!(!context.shall_stop_ongoing(), "canceled");
    let setup_file_content = render_setup_file(context, &setup_code)?;
    /* encrypting may also take a while ... */
    ensure!(!context.shall_stop_ongoing(), "canceled");
    let setup_file_name = context.new_blob_file(
        "autocrypt-setup-message.html",
        setup_file_content.as_bytes(),
    )?;

    let chat_id = chat::create_by_contact_id(context, 1)?;
    msg = Message::default();
    msg.type_0 = Viewtype::File;
    msg.param.set(Param::File, setup_file_name);

    msg.param
        .set(Param::MimeType, "application/autocrypt-setup");
    msg.param.set_int(Param::Cmd, 6);
    msg.param
        .set_int(Param::ForcePlaintext, DC_FP_NO_AUTOCRYPT_HEADER);

    ensure!(!context.shall_stop_ongoing(), "canceled");
    let msg_id = chat::send_msg(context, chat_id, &mut msg)?;
    info!(context, "Wait for setup message being sent ...",);
    while !context.shall_stop_ongoing() {
        std::thread::sleep(std::time::Duration::from_secs(1));
        if let Ok(msg) = Message::load_from_db(context, msg_id) {
            if msg.is_sent() {
                info!(context, "... setup message sent.",);
                break;
            }
        }
    }
    Ok(setup_code)
}

/// Renders HTML body of a setup file message.
///
/// The `passphrase` must be at least 2 characters long.
pub fn render_setup_file(context: &Context, passphrase: &str) -> Result<String> {
    ensure!(
        passphrase.len() >= 2,
        "Passphrase must be at least 2 chars long."
    );
    let self_addr = e2ee::ensure_secret_key_exists(context)?;
    let private_key = Key::from_self_private(context, self_addr, &context.sql)
        .ok_or(format_err!("Failed to get private key."))?;
    let ac_headers = match context.get_config_bool(Config::E2eeEnabled) {
        false => None,
        true => Some(("Autocrypt-Prefer-Encrypt", "mutual")),
    };
    let private_key_asc = private_key.to_asc(ac_headers);
    let encr = dc_pgp_symm_encrypt(&passphrase, private_key_asc.as_bytes())?;

    let replacement = format!(
        concat!(
            "-----BEGIN PGP MESSAGE-----\r\n",
            "Passphrase-Format: numeric9x4\r\n",
            "Passphrase-Begin: {}"
        ),
        &passphrase[..2]
    );
    let pgp_msg = encr.replace("-----BEGIN PGP MESSAGE-----", &replacement);

    let msg_subj = context.stock_str(StockMessage::AcSetupMsgSubject);
    let msg_body = context.stock_str(StockMessage::AcSetupMsgBody);
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
            if !(random_val as usize > 60000) {
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

pub fn continue_key_transfer(context: &Context, msg_id: u32, setup_code: &str) -> Result<()> {
    ensure!(msg_id > DC_MSG_ID_LAST_SPECIAL, "wrong id");

    let msg = Message::load_from_db(context, msg_id);
    if msg.is_err() {
        bail!("Message is no Autocrypt Setup Message.");
    }
    let msg = msg.unwrap_or_default();
    ensure!(
        msg.is_setupmessage(),
        "Message is no Autocrypt Setup Message."
    );

    if let Some(filename) = msg.get_file(context) {
        if let Ok(ref mut buf) = dc_read_file(context, filename) {
            let sc = normalize_setup_code(setup_code);
            if let Ok(armored_key) = decrypt_setup_file(context, sc, buf) {
                set_self_key(context, &armored_key, true, true)?;
            } else {
                bail!("Bad setup code.")
            }

            Ok(())
        } else {
            bail!("Cannot read Autocrypt Setup Message file.");
        }
    } else {
        bail!("Message is no Autocrypt Setup Message.");
    }
}

fn set_self_key(
    context: &Context,
    armored: &str,
    set_default: bool,
    prefer_encrypt_required: bool,
) -> Result<()> {
    // try hard to only modify key-state
    let keys = Key::from_armored_string(armored, KeyType::Private)
        .and_then(|(k, h)| if k.verify() { Some((k, h)) } else { None })
        .and_then(|(k, h)| k.split_key().map(|pub_key| (k, pub_key, h)));

    ensure!(keys.is_some(), "Not a valid private key");

    let (private_key, public_key, header) = keys.unwrap();
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
                .set_raw_config_int(context, "e2ee_enabled", e2ee_enabled)?;
        }
        None => {
            if prefer_encrypt_required {
                bail!("missing Autocrypt-Prefer-Encrypt header");
            }
        }
    };

    let self_addr = context.get_config(Config::ConfiguredAddr);
    ensure!(self_addr.is_some(), "Missing self addr");

    // XXX maybe better make dc_key_save_self_keypair delete things
    sql::execute(
        context,
        &context.sql,
        "DELETE FROM keypairs WHERE public_key=? OR private_key=?;",
        params![public_key.to_bytes(), private_key.to_bytes()],
    )?;

    if set_default {
        sql::execute(
            context,
            &context.sql,
            "UPDATE keypairs SET is_default=0;",
            params![],
        )?;
    }

    if !dc_key_save_self_keypair(
        context,
        &public_key,
        &private_key,
        self_addr.unwrap_or_default(),
        set_default,
        &context.sql,
    ) {
        bail!("Cannot save keypair, internal key-state possibly corrupted now!");
    }
    Ok(())
}

fn decrypt_setup_file(
    _context: &Context,
    passphrase: impl AsRef<str>,
    filecontent: &mut [u8],
) -> Result<String> {
    let mut fc_headerline = String::default();
    let mut fc_base64: *const libc::c_char = ptr::null();

    let split_result = unsafe {
        dc_split_armored_data(
            filecontent.as_mut_ptr().cast(),
            &mut fc_headerline,
            ptr::null_mut(),
            ptr::null_mut(),
            &mut fc_base64,
        )
    };

    if !split_result || fc_headerline != "-----BEGIN PGP MESSAGE-----" || fc_base64.is_null() {
        bail!("Invalid armored data");
    }

    // convert base64 to binary
    let base64_encoded =
        unsafe { std::slice::from_raw_parts(fc_base64 as *const u8, libc::strlen(fc_base64)) };

    let data = base64_decode(&base64_encoded)?;

    // decrypt symmetrically
    let payload = dc_pgp_symm_decrypt(passphrase.as_ref(), &data)?;
    let payload_str = String::from_utf8(payload)?;

    Ok(payload_str)
}

/// Decode the base64 encoded slice. Handles line breaks.
fn base64_decode(input: &[u8]) -> Result<Vec<u8>> {
    use std::io::Read;
    let c = std::io::Cursor::new(input);
    let lr = pgp::line_reader::LineReader::new(c);
    let br = pgp::base64_reader::Base64Reader::new(lr);
    let mut reader = pgp::base64_decoder::Base64Decoder::new(br);

    let mut data = Vec::new();
    reader.read_to_end(&mut data)?;

    Ok(data)
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

#[allow(non_snake_case)]
pub fn job_do_DC_JOB_IMEX_IMAP(context: &Context, job: &Job) -> Result<()> {
    ensure!(context.alloc_ongoing(), "could not allocate ongoing");
    let what: Option<ImexMode> = job.param.get_int(Param::Cmd).and_then(ImexMode::from_i32);
    let param = job.param.get(Param::Arg).unwrap_or_default();

    ensure!(!param.is_empty(), "No Import/export dir/file given.");
    info!(context, "Import/export process started.");
    context.call_cb(Event::ImexProgress(10));

    ensure!(context.sql.is_open(), "Database not opened.");
    if what == Some(ImexMode::ExportBackup) || what == Some(ImexMode::ExportSelfKeys) {
        // before we export anything, make sure the private key exists
        if e2ee::ensure_secret_key_exists(context).is_err() {
            context.free_ongoing();
            bail!("Cannot create private key or private key not available.");
        } else {
            dc_create_folder(context, &param);
        }
    }
    let path = Path::new(param);
    let success = match what {
        Some(ImexMode::ExportSelfKeys) => export_self_keys(context, path),
        Some(ImexMode::ImportSelfKeys) => import_self_keys(context, path),
        Some(ImexMode::ExportBackup) => export_backup(context, path),
        Some(ImexMode::ImportBackup) => import_backup(context, path),
        None => {
            bail!("unknown IMEX type");
        }
    };
    context.free_ongoing();
    match success {
        Ok(()) => {
            info!(context, "IMEX successfully completed");
            context.call_cb(Event::ImexProgress(1000));
            Ok(())
        }
        Err(err) => {
            context.call_cb(Event::ImexProgress(0));
            bail!("IMEX FAILED to complete: {}", err);
        }
    }
}

/// Import Backup
fn import_backup(context: &Context, backup_to_import: impl AsRef<Path>) -> Result<()> {
    info!(
        context,
        "Import \"{}\" to \"{}\".",
        backup_to_import.as_ref().display(),
        context.get_dbfile().display()
    );

    ensure!(
        !dc_is_configured(context),
        "Cannot import backups to accounts in use."
    );
    context.sql.close(&context);
    dc_delete_file(context, context.get_dbfile());
    ensure!(
        !dc_file_exist(context, context.get_dbfile()),
        "Cannot delete old database."
    );

    ensure!(
        dc_copy_file(context, backup_to_import.as_ref(), context.get_dbfile()),
        "could not copy file"
    );
    /* error already logged */
    /* re-open copied database file */
    ensure!(
        context.sql.open(&context, &context.get_dbfile(), 0),
        "could not re-open db"
    );

    let total_files_cnt = context
        .sql
        .query_get_value::<_, isize>(context, "SELECT COUNT(*) FROM backup_blobs;", params![])
        .unwrap_or_default() as usize;
    info!(
        context,
        "***IMPORT-in-progress: total_files_cnt={:?}", total_files_cnt,
    );

    let res = context.sql.query_map(
        "SELECT file_name, file_content FROM backup_blobs ORDER BY id;",
        params![],
        |row| {
            let name: String = row.get(0)?;
            let blob: Vec<u8> = row.get(1)?;

            Ok((name, blob))
        },
        |files| {
            for (processed_files_cnt, file) in files.enumerate() {
                let (file_name, file_blob) = file?;
                ensure!(!context.shall_stop_ongoing(), "received stop signal");
                let mut permille = processed_files_cnt * 1000 / total_files_cnt;
                if permille < 10 {
                    permille = 10
                }
                if permille > 990 {
                    permille = 990
                }
                context.call_cb(Event::ImexProgress(permille));
                if file_blob.is_empty() {
                    continue;
                }

                let path_filename = context.get_blobdir().join(file_name);
                if dc_write_file(context, &path_filename, &file_blob) {
                    continue;
                }
                bail!(
                    "Storage full? Cannot write file {} with {} bytes.",
                    path_filename.display(),
                    file_blob.len(),
                );
            }
            Ok(())
        },
    );

    res.and_then(|_| {
        // only delete backup_blobs if all files were successfully extracted
        sql::execute(context, &context.sql, "DROP TABLE backup_blobs;", params![])?;
        sql::try_execute(context, &context.sql, "VACUUM;").ok();
        Ok(())
    })
}

/*******************************************************************************
 * Export backup
 ******************************************************************************/
/* the FILE_PROGRESS macro calls the callback with the permille of files processed.
The macro avoids weird values of 0% or 100% while still working. */
fn export_backup(context: &Context, dir: impl AsRef<Path>) -> Result<()> {
    // get a fine backup file name (the name includes the date so that multiple backup instances are possible)
    // FIXME: we should write to a temporary file first and rename it on success. this would guarantee the backup is complete.
    // let dest_path_filename = dc_get_next_backup_file(context, dir, res);
    let now = time();
    let dest_path_filename = dc_get_next_backup_path(dir, now)?;

    sql::housekeeping(context);

    sql::try_execute(context, &context.sql, "VACUUM;").ok();
    context.sql.close(context);
    info!(
        context,
        "Backup \"{}\" to \"{}\".",
        context.get_dbfile().display(),
        dest_path_filename.display(),
    );
    let copied = dc_copy_file(context, context.get_dbfile(), &dest_path_filename);
    context.sql.open(&context, &context.get_dbfile(), 0);
    if !copied {
        let s = dest_path_filename.to_string_lossy().to_string();
        bail!(
            "could not copy file from {:?} to {:?}",
            context.get_dbfile(),
            s
        );
    }
    match add_files_to_export(context, &dest_path_filename) {
        Err(err) => {
            dc_delete_file(context, &dest_path_filename);
            error!(context, "backup failed: {}", err);
            Err(err)
        }
        Ok(()) => {
            context
                .sql
                .set_raw_config_int(context, "backup_time", now as i32)?;
            context.call_cb(Event::ImexFileWritten(dest_path_filename.clone()));
            Ok(())
        }
    }
}

fn add_files_to_export(context: &Context, dest_path_filename: &PathBuf) -> Result<()> {
    // add all files as blobs to the database copy (this does not require
    // the source to be locked, neigher the destination as it is used only here)
    let sql = Sql::new();
    ensure!(
        sql.open(context, &dest_path_filename, 0),
        "could not open db"
    );
    if !sql.table_exists("backup_blobs") {
        sql::execute(
            context,
            &sql,
            "CREATE TABLE backup_blobs (id INTEGER PRIMARY KEY, file_name, file_content);",
            params![],
        )?
    }
    // copy all files from BLOBDIR into backup-db
    let mut total_files_cnt = 0;
    let dir = context.get_blobdir();
    let dir_handle = std::fs::read_dir(&dir)?;
    total_files_cnt += dir_handle.filter(|r| r.is_ok()).count();

    info!(context, "EXPORT: total_files_cnt={}", total_files_cnt);
    // scan directory, pass 2: copy files
    let dir_handle = std::fs::read_dir(&dir)?;
    sql.prepare(
        "INSERT INTO backup_blobs (file_name, file_content) VALUES (?, ?);",
        |mut stmt, _| {
            let mut processed_files_cnt = 0;
            for entry in dir_handle {
                let entry = entry?;
                ensure!(
                    !context.shall_stop_ongoing(),
                    "canceled during export-files"
                );
                processed_files_cnt += 1;
                let permille = max(min(processed_files_cnt * 1000 / total_files_cnt, 990), 10);
                context.call_cb(Event::ImexProgress(permille));

                let name_f = entry.file_name();
                let name = name_f.to_string_lossy();
                if name.starts_with("delta-chat") && name.ends_with(".bak") {
                    continue;
                }
                info!(context, "EXPORT: copying filename={}", name);
                let curr_path_filename = context.get_blobdir().join(entry.file_name());
                if let Ok(buf) = dc_read_file(context, &curr_path_filename) {
                    if buf.is_empty() {
                        continue;
                    }
                    // bail out if we can't insert
                    stmt.execute(params![name, buf])?;
                }
            }
            Ok(())
        },
    )?;
    Ok(())
}

/*******************************************************************************
 * Classic key import
 ******************************************************************************/
fn import_self_keys(context: &Context, dir: impl AsRef<Path>) -> Result<()> {
    /* hint: even if we switch to import Autocrypt Setup Files, we should leave the possibility to import
    plain ASC keys, at least keys without a password, if we do not want to implement a password entry function.
    Importing ASC keys is useful to use keys in Delta Chat used by any other non-Autocrypt-PGP implementation.

    Maybe we should make the "default" key handlong also a little bit smarter
    (currently, the last imported key is the standard key unless it contains the string "legacy" in its name) */
    let mut set_default: bool;
    let mut key: String;
    let mut imported_cnt = 0;

    let dir_name = dir.as_ref().to_string_lossy();
    let dir_handle = std::fs::read_dir(&dir)?;
    for entry in dir_handle {
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
        let ccontent = if let Ok(content) = dc_read_file(context, &path_plus_name) {
            key = String::from_utf8_lossy(&content).to_string();
            CString::new(content).unwrap_or_default()
        } else {
            continue;
        };

        /* only import if we have a private key */
        let mut buf2_headerline = String::default();
        let split_res: bool;
        unsafe {
            let buf2 = dc_strdup(ccontent.as_ptr());
            split_res = dc_split_armored_data(
                buf2,
                &mut buf2_headerline,
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
            );
            libc::free(buf2 as *mut libc::c_void);
        }
        if split_res
            && buf2_headerline.contains("-----BEGIN PGP PUBLIC KEY BLOCK-----")
            && !key.contains("-----BEGIN PGP PRIVATE KEY BLOCK")
        {
            info!(context, "ignoring public key file '{}", name_f);
            // it's fine: DC exports public with private
            continue;
        }
        if let Err(err) = set_self_key(context, &key, set_default, false) {
            error!(context, "set_self_key: {}", err);
            continue;
        }
        imported_cnt += 1
    }
    ensure!(
        imported_cnt > 0,
        "No private keys found in \"{}\".",
        dir_name
    );
    Ok(())
}

fn export_self_keys(context: &Context, dir: impl AsRef<Path>) -> Result<()> {
    let mut export_errors = 0;

    context.sql.query_map(
        "SELECT id, public_key, private_key, is_default FROM keypairs;",
        params![],
        |row| {
            let id = row.get(0)?;
            let public_key_blob: Vec<u8> = row.get(1)?;
            let public_key = Key::from_slice(&public_key_blob, KeyType::Public);
            let private_key_blob: Vec<u8> = row.get(2)?;
            let private_key = Key::from_slice(&private_key_blob, KeyType::Private);
            let is_default: i32 = row.get(3)?;

            Ok((id, public_key, private_key, is_default))
        },
        |keys| {
            for key_pair in keys {
                let (id, public_key, private_key, is_default) = key_pair?;
                let id = Some(id).filter(|_| is_default != 0);
                if let Some(key) = public_key {
                    if !export_key_to_asc_file(context, &dir, id, &key) {
                        export_errors += 1;
                    }
                } else {
                    export_errors += 1;
                }
                if let Some(key) = private_key {
                    if !export_key_to_asc_file(context, &dir, id, &key) {
                        export_errors += 1;
                    }
                } else {
                    export_errors += 1;
                }
            }

            Ok(())
        },
    )?;

    ensure!(export_errors == 0, "errors while exporting keys");
    Ok(())
}

/*******************************************************************************
 * Classic key export
 ******************************************************************************/
fn export_key_to_asc_file(
    context: &Context,
    dir: impl AsRef<Path>,
    id: Option<i64>,
    key: &Key,
) -> bool {
    let mut success = false;
    let file_name = {
        let kind = if key.is_public() { "public" } else { "private" };
        let id = id.map_or("default".into(), |i| i.to_string());

        dir.as_ref().join(format!("{}-key-{}.asc", kind, &id))
    };
    info!(context, "Exporting key {}", file_name.display());
    dc_delete_file(context, &file_name);

    if !key.write_asc_to_file(&file_name, context) {
        error!(context, "Cannot write key to {}", file_name.display());
    } else {
        context.call_cb(Event::ImexFileWritten(file_name));
        success = true;
    }

    success
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::test_utils::*;

    #[test]
    fn test_render_setup_file() {
        let t = test_context(Some(Box::new(logging_cb)));

        configure_alice_keypair(&t.ctx);
        let msg = render_setup_file(&t.ctx, "hello").unwrap();
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
        assert!(msg.contains("==\n"));
        assert!(msg.contains("-----END PGP MESSAGE-----\n"));
    }

    fn ac_setup_msg_cb(ctx: &Context, evt: Event) -> libc::uintptr_t {
        match evt {
            Event::GetString {
                id: StockMessage::AcSetupMsgBody,
                ..
            } => unsafe { "hello\r\nthere".strdup() as usize },
            _ => logging_cb(ctx, evt),
        }
    }

    #[test]
    fn otest_render_setup_file_newline_replace() {
        let t = test_context(Some(Box::new(ac_setup_msg_cb)));
        configure_alice_keypair(&t.ctx);
        let msg = render_setup_file(&t.ctx, "pw").unwrap();
        println!("{}", &msg);
        assert!(msg.contains("<p>hello<br>there</p>"));
    }

    #[test]
    fn test_create_setup_code() {
        let t = dummy_context();
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

    #[test]
    fn test_export_key_to_asc_file() {
        let context = dummy_context();
        let base64 = include_str!("../test-data/key/public.asc");
        let key = Key::from_base64(base64, KeyType::Public).unwrap();
        let blobdir = "$BLOBDIR";
        assert!(export_key_to_asc_file(&context.ctx, blobdir, None, &key));
        let blobdir = context.ctx.get_blobdir().to_str().unwrap();
        let filename = format!("{}/public-key-default.asc", blobdir);
        let bytes = std::fs::read(&filename).unwrap();

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

    #[test]
    fn test_split_and_decrypt() {
        let ctx = dummy_context();
        let context = &ctx.ctx;

        let mut headerline = String::default();
        let mut setupcodebegin = ptr::null();
        let mut preferencrypt = ptr::null();

        unsafe {
            let buf_1 = S_EM_SETUPFILE.strdup();
            let res = dc_split_armored_data(
                buf_1,
                &mut headerline,
                &mut setupcodebegin,
                &mut preferencrypt,
                ptr::null_mut(),
            );
            libc::free(buf_1 as *mut libc::c_void);
            assert!(res);
        }
        assert_eq!(headerline, "-----BEGIN PGP MESSAGE-----");
        assert!(!setupcodebegin.is_null());

        // TODO: verify that this is the right check
        assert!(S_EM_SETUPCODE.starts_with(as_str(setupcodebegin)));

        assert!(preferencrypt.is_null());

        let mut setup_file = S_EM_SETUPFILE.to_string();
        let decrypted = unsafe {
            decrypt_setup_file(context, S_EM_SETUPCODE, setup_file.as_bytes_mut()).unwrap()
        };

        unsafe {
            let buf1 = decrypted.strdup();
            assert!(dc_split_armored_data(
                buf1,
                &mut headerline,
                &mut setupcodebegin,
                &mut preferencrypt,
                ptr::null_mut(),
            ));
            libc::free(buf1 as *mut libc::c_void);
        }

        assert_eq!(headerline, "-----BEGIN PGP PRIVATE KEY BLOCK-----");
        assert!(setupcodebegin.is_null());
        assert!(!preferencrypt.is_null());
        assert_eq!(as_str(preferencrypt), "mutual",);
    }
}
