use std::ffi::CString;
use std::path::Path;
use std::ptr;

use libc::{free, strcmp, strlen, strstr};
use mmime::mailmime_content::*;
use mmime::mmapstring::*;
use mmime::other::*;
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

// import/export and tools
// param1 is a directory where the keys are written to
// param1 is a directory where the keys are searched in and read from
// param1 is a directory where the backup is written to
// param1 is the file with the backup to import
pub fn dc_imex(
    context: &Context,
    what: libc::c_int,
    param1: Option<impl AsRef<Path>>,
    param2: Option<impl AsRef<Path>>,
) {
    let mut param = Params::new();
    param.set_int(Param::Cmd, what as i32);
    if let Some(param1) = param1 {
        param.set(Param::Arg, param1.as_ref().to_string_lossy());
    }
    if let Some(param2) = param2 {
        param.set(Param::Arg, param2.as_ref().to_string_lossy());
    }

    job_kill_action(context, Action::ImexImap);
    job_add(context, Action::ImexImap, 0, param, 0);
}

/// Returns the filename of the backup if found, nullptr otherwise.
pub fn dc_imex_has_backup(context: &Context, dir_name: impl AsRef<Path>) -> Result<String> {
    let dir_name = dir_name.as_ref();
    let dir_iter = std::fs::read_dir(dir_name);
    if dir_iter.is_err() {
        bail!("Backup check: Cannot open directory \"{:?}\"", dir_name);
    }
    let mut newest_backup_time = 0;
    let mut newest_backup_path: Option<std::path::PathBuf> = None;
    for dirent in dir_iter.unwrap() {
        match dirent {
            Ok(dirent) => {
                let path = dirent.path();
                let name = dirent.file_name();
                let name = name.to_string_lossy();
                if name.starts_with("delta-chat") && name.ends_with(".bak") {
                    let sql = Sql::new();
                    if sql.open(context, &path, 0x1) {
                        let curr_backup_time =
                            sql.get_config_int(context, "backup_time")
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

pub unsafe fn dc_initiate_key_transfer(context: &Context) -> *mut libc::c_char {
    let mut msg: Message;
    if !dc_alloc_ongoing(context) {
        return std::ptr::null_mut();
    }
    let setup_code = dc_create_setup_code(context);
    /* this may require a keypair to be created. this may take a second ... */
    if !context
        .running_state
        .clone()
        .read()
        .unwrap()
        .shall_stop_ongoing
    {
        if let Ok(ref setup_file_content) = dc_render_setup_file(context, &setup_code) {
            /* encrypting may also take a while ... */
            if !context
                .running_state
                .clone()
                .read()
                .unwrap()
                .shall_stop_ongoing
            {
                let setup_file_name =
                    dc_get_fine_path_filename(context, "$BLOBDIR", "autocrypt-setup-message.html");
                if dc_write_file(context, &setup_file_name, setup_file_content.as_bytes()) {
                    if let Ok(chat_id) = chat::create_by_contact_id(context, 1) {
                        msg = Message::default();
                        msg.type_0 = Viewtype::File;
                        msg.param
                            .set(Param::File, setup_file_name.to_string_lossy());

                        msg.param
                            .set(Param::MimeType, "application/autocrypt-setup");
                        msg.param.set_int(Param::Cmd, 6);
                        msg.param
                            .set_int(Param::ForcePlaintext, DC_FP_NO_AUTOCRYPT_HEADER);

                        if !context
                            .running_state
                            .clone()
                            .read()
                            .unwrap()
                            .shall_stop_ongoing
                        {
                            if let Ok(msg_id) = chat::send_msg(context, chat_id, &mut msg) {
                                info!(context, "Wait for setup message being sent ...",);
                                loop {
                                    if context
                                        .running_state
                                        .clone()
                                        .read()
                                        .unwrap()
                                        .shall_stop_ongoing
                                    {
                                        break;
                                    }
                                    std::thread::sleep(std::time::Duration::from_secs(1));
                                    if let Ok(msg) = Message::load_from_db(context, msg_id) {
                                        if msg.is_sent() {
                                            info!(context, "... setup message sent.",);
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    dc_free_ongoing(context);

    setup_code.strdup()
}

/// Renders HTML body of a setup file message.
///
/// The `passphrase` must be at least 2 characters long.
pub fn dc_render_setup_file(context: &Context, passphrase: &str) -> Result<String> {
    ensure!(
        passphrase.len() >= 2,
        "Passphrase must be at least 2 chars long."
    );
    let self_addr = e2ee::ensure_secret_key_exists(context)?;
    let private_key = Key::from_self_private(context, self_addr, &context.sql)
        .ok_or(format_err!("Failed to get private key."))?;
    let ac_headers = match context
        .sql
        .get_config_int(context, Config::E2eeEnabled)
        .unwrap_or(1)
    {
        0 => None,
        _ => Some(("Autocrypt-Prefer-Encrypt", "mutual")),
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

pub fn dc_create_setup_code(_context: &Context) -> String {
    let mut random_val: u16;
    let mut rng = thread_rng();
    let mut ret = String::new();

    for i in 0..9 {
        loop {
            random_val = rng.gen();
            if !(random_val as libc::c_int > 60000) {
                break;
            }
        }
        random_val = (random_val as libc::c_int % 10000) as u16;
        ret += &format!(
            "{}{:04}",
            if 0 != i { "-" } else { "" },
            random_val as libc::c_int,
        );
    }

    ret
}

pub unsafe fn dc_continue_key_transfer(context: &Context, msg_id: u32, setup_code: &str) -> bool {
    if msg_id <= DC_MSG_ID_LAST_SPECIAL {
        return false;
    }

    let msg = Message::load_from_db(context, msg_id);
    if msg.is_err() {
        error!(context, "Message is no Autocrypt Setup Message.");
        return false;
    }
    let msg = msg.unwrap();
    if !msg.is_setupmessage() {
        error!(context, "Message is no Autocrypt Setup Message.");
        return false;
    }

    if let Some(filename) = msg.get_file(context) {
        if let Ok(buf) = dc_read_file(context, filename) {
            let norm_sc = CString::yolo(dc_normalize_setup_code(setup_code));
            let armored_key = dc_decrypt_setup_file(context, norm_sc.as_ptr(), buf.as_ptr().cast());
            if armored_key.is_null() {
                warn!(context, "Cannot decrypt Autocrypt Setup Message.",);
                false
            } else if set_self_key(context, armored_key, 1) {
                /*set default*/
                /* error already logged */
                free(armored_key as *mut libc::c_void);
                true
            } else {
                false
            }
        } else {
            error!(context, "Cannot read Autocrypt Setup Message file.",);
            false
        }
    } else {
        error!(context, "Message is no Autocrypt Setup Message.");
        false
    }
}

fn set_self_key(
    context: &Context,
    armored_c: *const libc::c_char,
    set_default: libc::c_int,
) -> bool {
    assert!(!armored_c.is_null(), "invalid buffer");
    let armored = as_str(armored_c);

    let keys = Key::from_armored_string(armored, KeyType::Private)
        .and_then(|(k, h)| if k.verify() { Some((k, h)) } else { None })
        .and_then(|(k, h)| k.split_key().map(|pub_key| (k, pub_key, h)));

    if keys.is_none() {
        error!(context, "File does not contain a valid private key.",);
        return false;
    }

    let (private_key, public_key, header) = keys.unwrap();
    let preferencrypt = header.get("Autocrypt-Prefer-Encrypt");

    if sql::execute(
        context,
        &context.sql,
        "DELETE FROM keypairs WHERE public_key=? OR private_key=?;",
        params![public_key.to_bytes(), private_key.to_bytes()],
    )
    .is_err()
    {
        return false;
    }

    if 0 != set_default {
        if sql::execute(
            context,
            &context.sql,
            "UPDATE keypairs SET is_default=0;",
            params![],
        )
        .is_err()
        {
            return false;
        }
    } else {
        error!(context, "File does not contain a private key.",);
    }

    let self_addr = context.get_config(Config::ConfiguredAddr);

    if self_addr.is_none() {
        error!(context, "Missing self addr");
        return false;
    }

    if !dc_key_save_self_keypair(
        context,
        &public_key,
        &private_key,
        self_addr.unwrap(),
        set_default,
        &context.sql,
    ) {
        error!(context, "Cannot save keypair.");
        return false;
    }

    match preferencrypt.map(|s| s.as_str()) {
        Some("") => false,
        Some("nopreference") => context
            .sql
            .set_config_int(context, "e2ee_enabled", 0)
            .is_ok(),
        Some("mutual") => context
            .sql
            .set_config_int(context, "e2ee_enabled", 1)
            .is_ok(),
        _ => true,
    }
}

pub unsafe fn dc_decrypt_setup_file(
    context: &Context,
    passphrase: *const libc::c_char,
    filecontent: *const libc::c_char,
) -> *mut libc::c_char {
    let fc_buf: *mut libc::c_char;
    let mut fc_headerline: *const libc::c_char = ptr::null();
    let mut fc_base64: *const libc::c_char = ptr::null();
    let mut binary: *mut libc::c_char = ptr::null_mut();
    let mut binary_bytes: libc::size_t = 0;
    let mut indx: libc::size_t = 0;

    let mut payload: *mut libc::c_char = ptr::null_mut();
    fc_buf = dc_strdup(filecontent);
    if dc_split_armored_data(
        fc_buf,
        &mut fc_headerline,
        ptr::null_mut(),
        ptr::null_mut(),
        &mut fc_base64,
    ) && !fc_headerline.is_null()
        && strcmp(
            fc_headerline,
            b"-----BEGIN PGP MESSAGE-----\x00" as *const u8 as *const libc::c_char,
        ) == 0
        && !fc_base64.is_null()
    {
        /* convert base64 to binary */
        /*must be freed using mmap_string_unref()*/
        if !(mailmime_base64_body_parse(
            fc_base64,
            strlen(fc_base64),
            &mut indx,
            &mut binary,
            &mut binary_bytes,
        ) != MAILIMF_NO_ERROR as libc::c_int
            || binary.is_null()
            || binary_bytes == 0)
        {
            /* decrypt symmetrically */
            match dc_pgp_symm_decrypt(
                as_str(passphrase),
                std::slice::from_raw_parts(binary as *const u8, binary_bytes),
            ) {
                Ok(plain) => {
                    let payload_c = CString::new(plain).unwrap();
                    payload = strdup(payload_c.as_ptr());
                }
                Err(err) => {
                    error!(context, "Failed to decrypt message: {:?}", err);
                }
            }
        }
    }

    free(fc_buf as *mut libc::c_void);
    if !binary.is_null() {
        mmap_string_unref(binary);
    }

    payload
}

pub fn dc_normalize_setup_code(s: &str) -> String {
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
pub fn dc_job_do_DC_JOB_IMEX_IMAP(context: &Context, job: &Job) -> Result<()> {
    if !dc_alloc_ongoing(context) {
        bail!("could not allocate ongoing")
    }
    let what = job.param.get_int(Param::Cmd).unwrap_or_default();
    let param1_s = job.param.get(Param::Arg).unwrap_or_default();
    let param1 = CString::yolo(param1_s);

    if param1_s.is_empty() {
        bail!("No Import/export dir/file given.");
    }
    info!(context, "Import/export process started.");
    context.call_cb(Event::ImexProgress(10));

    if !context.sql.is_open() {
        bail!("Database not opened.");
    }
    if what == DC_IMEX_EXPORT_BACKUP || what == DC_IMEX_EXPORT_SELF_KEYS {
        /* before we export anything, make sure the private key exists */
        if e2ee::ensure_secret_key_exists(context).is_err() {
            dc_free_ongoing(context);
            bail!("Cannot create private key or private key not available.");
        } else {
            dc_create_folder(context, &param1_s);
        }
    }
    let success = match what {
        DC_IMEX_EXPORT_SELF_KEYS => export_self_keys(context, &param1_s),
        DC_IMEX_IMPORT_SELF_KEYS => unsafe { import_self_keys(context, &param1_s) },
        DC_IMEX_EXPORT_BACKUP => unsafe { export_backup(context, param1.as_ptr()) },
        DC_IMEX_IMPORT_BACKUP => unsafe { import_backup(context, param1.as_ptr()) },
        _ => {
            bail!("unknown IMEX type: {}", what);
        }
    };
    dc_free_ongoing(context);
    match success {
        true => {
            info!(context, "IMEX successfully completed");
            context.call_cb(Event::ImexProgress(1000));
            Ok(())
        }
        false => {
            context.call_cb(Event::ImexProgress(0));
            bail!("IMEX FAILED to complete");
        }
    }
}

/*******************************************************************************
 * Import backup
 ******************************************************************************/

#[allow(non_snake_case)]
unsafe fn import_backup(context: &Context, backup_to_import: *const libc::c_char) -> bool {
    info!(
        context,
        "Import \"{}\" to \"{}\".",
        as_str(backup_to_import),
        context.get_dbfile().display()
    );

    if dc_is_configured(context) {
        error!(context, "Cannot import backups to accounts in use.");
        return false;
    }
    &context.sql.close(&context);
    dc_delete_file(context, context.get_dbfile());
    if dc_file_exist(context, context.get_dbfile()) {
        error!(
            context,
            "Cannot import backups: Cannot delete the old file.",
        );
        return false;
    }

    if !dc_copy_file(context, as_path(backup_to_import), context.get_dbfile()) {
        return false;
    }
    /* error already logged */
    /* re-open copied database file */
    if !context.sql.open(&context, &context.get_dbfile(), 0) {
        return false;
    }

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
            let mut loop_success = true;
            let mut processed_files_cnt = 0;

            for file in files {
                let (file_name, file_blob) = file?;

                if context
                    .running_state
                    .clone()
                    .read()
                    .unwrap()
                    .shall_stop_ongoing
                {
                    loop_success = false;
                    break;
                }
                processed_files_cnt += 1;
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

                let pathNfilename = context.get_blobdir().join(file_name);
                if dc_write_file(context, &pathNfilename, &file_blob) {
                    continue;
                }
                error!(
                    context,
                    "Storage full? Cannot write file {} with {} bytes.",
                    pathNfilename.display(),
                    file_blob.len(),
                );
                // otherwise the user may believe the stuff is imported correctly, but there are files missing ...
                loop_success = false;
                break;
            }

            if !loop_success {
                return Err(format_err!("fail"));
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
    .is_ok()
}

/*******************************************************************************
 * Export backup
 ******************************************************************************/
/* the FILE_PROGRESS macro calls the callback with the permille of files processed.
The macro avoids weird values of 0% or 100% while still working. */
#[allow(non_snake_case)]
unsafe fn export_backup(context: &Context, dir: *const libc::c_char) -> bool {
    let mut ok_to_continue: bool;
    let mut success = false;

    let mut delete_dest_file: libc::c_int = 0;
    // get a fine backup file name (the name includes the date so that multiple backup instances are possible)
    // FIXME: we should write to a temporary file first and rename it on success. this would guarantee the backup is complete. however, currently it is not clear it the import exists in the long run (may be replaced by a restore-from-imap)
    let now = time();
    let res = chrono::NaiveDateTime::from_timestamp(now as i64, 0)
        .format("delta-chat-%Y-%m-%d.bak")
        .to_string();

    let dest_path_filename = dc_get_fine_path_filename(context, as_path(dir), res);

    sql::housekeeping(context);

    sql::try_execute(context, &context.sql, "VACUUM;").ok();
    context.sql.close(context);
    let mut closed = true;
    info!(
        context,
        "Backup \"{}\" to \"{}\".",
        context.get_dbfile().display(),
        dest_path_filename.display(),
    );
    if dc_copy_file(context, context.get_dbfile(), &dest_path_filename) {
        context.sql.open(&context, &context.get_dbfile(), 0);
        closed = false;
        /* add all files as blobs to the database copy (this does not require the source to be locked, neigher the destination as it is used only here) */
        /*for logging only*/
        let sql = Sql::new();
        if sql.open(context, &dest_path_filename, 0) {
            if !sql.table_exists("backup_blobs") {
                if sql::execute(
                    context,
                    &sql,
                    "CREATE TABLE backup_blobs (id INTEGER PRIMARY KEY, file_name, file_content);",
                    params![],
                )
                .is_err()
                {
                    /* error already logged */
                    ok_to_continue = false;
                } else {
                    ok_to_continue = true;
                }
            } else {
                ok_to_continue = true;
            }
            if ok_to_continue {
                let mut total_files_cnt = 0;
                let dir = context.get_blobdir();
                if let Ok(dir_handle) = std::fs::read_dir(&dir) {
                    total_files_cnt += dir_handle.filter(|r| r.is_ok()).count();

                    info!(context, "EXPORT: total_files_cnt={}", total_files_cnt);
                    if total_files_cnt > 0 {
                        // scan directory, pass 2: copy files
                        if let Ok(dir_handle) = std::fs::read_dir(&dir) {
                            sql.prepare(
                                    "INSERT INTO backup_blobs (file_name, file_content) VALUES (?, ?);",
                                    move |mut stmt, _| {
                                        let mut processed_files_cnt = 0;
                                        for entry in dir_handle {
                                            if entry.is_err() {
                                                ok_to_continue = true;
                                                break;
                                            }
                                            let entry = entry.unwrap();
                                            if context
                                                .running_state
                                                .clone()
                                                .read()
                                                .unwrap()
                                                .shall_stop_ongoing
                                            {
                                                delete_dest_file = 1;
                                                ok_to_continue = false;
                                                break;
                                            } else {
                                                processed_files_cnt += 1;
                                                let mut permille =
                                                    processed_files_cnt * 1000 / total_files_cnt;
                                                if permille < 10 {
                                                    permille = 10;
                                                }
                                                if permille > 990 {
                                                    permille = 990;
                                                }
                                                context.call_cb(Event::ImexProgress(permille));

                                                let name_f = entry.file_name();
                                                let name = name_f.to_string_lossy();
                                                if name.starts_with("delta-chat") && name.ends_with(".bak")
                                                {
                                                    continue;
                                                } else {
                                                    info!(context, "EXPORTing filename={}", name);
                                                    let curr_pathNfilename = context.get_blobdir().join(entry.file_name());
                                                    if let Ok(buf) =
                                                        dc_read_file(context, &curr_pathNfilename)
                                                    {
                                                        if buf.is_empty() {
                                                            continue;
                                                        }
                                                        if stmt.execute(params![name, buf]).is_err() {
                                                            error!(
                                                                context,
                                                                "Disk full? Cannot add file \"{}\" to backup.",
                                                                curr_pathNfilename.display(),
                                                            );
                                                            /* this is not recoverable! writing to the sqlite database should work! */
                                                            ok_to_continue = false;
                                                            break;
                                                        }
                                                    } else {
                                                        continue;
                                                    }
                                                }
                                            }
                                        }
                                        Ok(())
                                    }
                                ).unwrap();
                        } else {
                            error!(
                                context,
                                "Backup: Cannot copy from blob-directory \"{}\".",
                                context.get_blobdir().display(),
                            );
                        }
                    } else {
                        info!(context, "Backup: No files to copy.",);
                        ok_to_continue = true;
                    }
                    if ok_to_continue {
                        if sql
                            .set_config_int(context, "backup_time", now as i32)
                            .is_ok()
                        {
                            context.call_cb(Event::ImexFileWritten(dest_path_filename.clone()));
                            success = true;
                        }
                    }
                } else {
                    error!(
                        context,
                        "Backup: Cannot get info for blob-directory \"{}\".",
                        context.get_blobdir().display(),
                    );
                };
            }
        }
    }
    if closed {
        context.sql.open(&context, &context.get_dbfile(), 0);
    }
    if 0 != delete_dest_file {
        dc_delete_file(context, &dest_path_filename);
    }

    success
}

/*******************************************************************************
 * Classic key import
 ******************************************************************************/
unsafe fn import_self_keys(context: &Context, dir_name: &str) -> bool {
    /* hint: even if we switch to import Autocrypt Setup Files, we should leave the possibility to import
    plain ASC keys, at least keys without a password, if we do not want to implement a password entry function.
    Importing ASC keys is useful to use keys in Delta Chat used by any other non-Autocrypt-PGP implementation.

    Maybe we should make the "default" key handlong also a little bit smarter
    (currently, the last imported key is the standard key unless it contains the string "legacy" in its name) */
    let mut set_default: libc::c_int;
    let mut buf: *mut libc::c_char = ptr::null_mut();
    // a pointer inside buf, MUST NOT be free()'d
    let mut private_key: *const libc::c_char;
    let mut buf2: *mut libc::c_char = ptr::null_mut();
    // a pointer inside buf2, MUST NOT be free()'d
    let mut buf2_headerline: *const libc::c_char = ptr::null_mut();
    let mut imported_cnt = 0;
    if !dir_name.is_empty() {
        let dir = std::path::Path::new(dir_name);
        if let Ok(dir_handle) = std::fs::read_dir(dir) {
            for entry in dir_handle {
                if let Err(err) = entry {
                    info!(context, "file-dir error: {}", err);
                    break;
                }
                let entry_fn = entry.unwrap().file_name();
                let name_f = entry_fn.to_string_lossy();

                info!(context, "Checking name_f: {}", name_f);
                match dc_get_filesuffix_lc(&name_f) {
                    Some(suffix) => {
                        if suffix != "asc" {
                            continue;
                        }
                    }
                    None => {
                        continue;
                    }
                }
                let path_plus_name = dir.join(&entry_fn);
                info!(context, "Checking: {}", path_plus_name.display());

                free(buf.cast());
                buf = ptr::null_mut();

                if let Ok(buf_r) = dc_read_file(context, &path_plus_name) {
                    buf = buf_r.as_ptr() as *mut _;
                    std::mem::forget(buf_r);
                } else {
                    continue;
                };

                private_key = buf;

                free(buf2 as *mut libc::c_void);
                buf2 = dc_strdup(buf);
                if dc_split_armored_data(
                    buf2,
                    &mut buf2_headerline,
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                ) && strcmp(
                    buf2_headerline,
                    b"-----BEGIN PGP PUBLIC KEY BLOCK-----\x00" as *const u8 as *const libc::c_char,
                ) == 0
                {
                    private_key = strstr(
                        buf,
                        b"-----BEGIN PGP PRIVATE KEY BLOCK\x00" as *const u8 as *const libc::c_char,
                    );
                    if private_key.is_null() {
                        /* this is no error but quite normal as we always export the public keys together with the private ones */
                        continue;
                    }
                }
                set_default = 1;
                if name_f.contains("legacy") {
                    info!(
                        context,
                        "Treating \"{}\" as a legacy private key.",
                        path_plus_name.display(),
                    );
                    set_default = 0i32
                }
                if !set_self_key(context, private_key, set_default) {
                    continue;
                }
                imported_cnt += 1
            }
            if imported_cnt == 0i32 {
                error!(context, "No private keys found in \"{}\".", dir_name,);
            }
        } else {
            error!(context, "Import: Cannot open directory \"{}\".", dir_name,);
        }
    }

    free(buf as *mut libc::c_void);
    free(buf2 as *mut libc::c_void);

    imported_cnt != 0
}

fn export_self_keys(context: &Context, dir: &str) -> bool {
    let mut export_errors = 0;

    context
        .sql
        .query_map(
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
                        if !export_key_to_asc_file(context, dir, id, &key) {
                            export_errors += 1;
                        }
                    } else {
                        export_errors += 1;
                    }
                    if let Some(key) = private_key {
                        if !export_key_to_asc_file(context, dir, id, &key) {
                            export_errors += 1;
                        }
                    } else {
                        export_errors += 1;
                    }
                }

                Ok(())
            },
        )
        .unwrap();

    export_errors == 0
}

/*******************************************************************************
 * Classic key export
 ******************************************************************************/
fn export_key_to_asc_file(context: &Context, dir: &str, id: Option<i64>, key: &Key) -> bool {
    let mut success = false;
    let file_name = {
        let kind = if key.is_public() { "public" } else { "private" };
        let id = id.map_or("default".into(), |i| i.to_string());

        Path::new(dir).join(format!("{}-key-{}.asc", kind, &id))
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
        let msg = dc_render_setup_file(&t.ctx, "hello").unwrap();
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
    fn test_render_setup_file_newline_replace() {
        let t = test_context(Some(Box::new(ac_setup_msg_cb)));
        configure_alice_keypair(&t.ctx);
        let msg = dc_render_setup_file(&t.ctx, "pw").unwrap();
        println!("{}", &msg);
        assert!(msg.contains("<p>hello<br>there</p>"));
    }

    #[test]
    fn test_create_setup_code() {
        let t = dummy_context();
        let setupcode = dc_create_setup_code(&t.ctx);
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
        let norm = dc_normalize_setup_code("123422343234423452346234723482349234");
        assert_eq!(norm, "1234-2234-3234-4234-5234-6234-7234-8234-9234");

        let norm = dc_normalize_setup_code(
            "\t1 2 3422343234- foo bar-- 423-45 2 34 6234723482349234      ",
        );
        assert_eq!(norm, "1234-2234-3234-4234-5234-6234-7234-8234-9234");
    }
}
