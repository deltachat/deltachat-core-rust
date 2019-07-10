use std::ffi::CString;

use failure::format_err;
use mmime::mailmime_content::*;
use mmime::mmapstring::*;
use mmime::other::*;
use rand::{thread_rng, Rng};

use crate::constants::*;
use crate::context::Context;
use crate::dc_chat::*;
use crate::dc_configure::*;
use crate::dc_e2ee::*;
use crate::dc_job::*;
use crate::dc_log::*;
use crate::dc_msg::*;
use crate::dc_param::*;
use crate::dc_sqlite3::*;
use crate::dc_stock::*;
use crate::dc_tools::*;
use crate::key::*;
use crate::pgp::*;
use crate::types::*;
use crate::x::*;

// import/export and tools
// param1 is a directory where the keys are written to
// param1 is a directory where the keys are searched in and read from
// param1 is a directory where the backup is written to
// param1 is the file with the backup to import
pub unsafe fn dc_imex(
    context: &Context,
    what: libc::c_int,
    param1: *const libc::c_char,
    param2: *const libc::c_char,
) {
    let param: *mut dc_param_t = dc_param_new();
    dc_param_set_int(param, 'S' as i32, what);
    dc_param_set(param, 'E' as i32, param1);
    dc_param_set(param, 'F' as i32, param2);
    dc_job_kill_action(context, 910i32);
    dc_job_add(context, 910i32, 0i32, (*param).packed, 0i32);
    dc_param_unref(param);
}

/// Returns the filename of the backup if found, nullptr otherwise.
pub unsafe fn dc_imex_has_backup(
    context: &Context,
    dir_name: *const libc::c_char,
) -> *mut libc::c_char {
    let dir_name = as_path(dir_name);
    let dir_iter = std::fs::read_dir(dir_name);
    if dir_iter.is_err() {
        dc_log_info(
            context,
            0i32,
            b"Backup check: Cannot open directory \"%s\".\x00" as *const u8 as *const libc::c_char,
            CString::new(format!("{}", dir_name.display()))
                .unwrap()
                .as_ptr(),
        );
        return 0 as *mut libc::c_char;
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
                    let sql = SQLite::new();
                    if sql.open(context, &path, 0x1) {
                        let curr_backup_time =
                            dc_sqlite3_get_config_int(context, &sql, "backup_time", 0) as u64;
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
        Some(path) => match path.to_c_string() {
            Ok(cstr) => dc_strdup(cstr.as_ptr()),
            Err(err) => {
                dc_log_error(
                    context,
                    0i32,
                    b"Invalid backup filename: %s\x00" as *const u8 as *const libc::c_char,
                    CString::new(format!("{}", err)).unwrap().as_ptr(),
                );
                std::ptr::null_mut()
            }
        },
        None => std::ptr::null_mut(),
    }
}

pub unsafe fn dc_initiate_key_transfer(context: &Context) -> *mut libc::c_char {
    let current_block: u64;
    let mut success: libc::c_int = 0i32;
    let mut setup_code: *mut libc::c_char;
    let mut setup_file_content: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut setup_file_name: *mut libc::c_char = 0 as *mut libc::c_char;
    let chat_id: uint32_t;
    let mut msg: *mut dc_msg_t = 0 as *mut dc_msg_t;
    let msg_id: uint32_t;
    if 0 == dc_alloc_ongoing(context) {
        return 0 as *mut libc::c_char;
    }
    setup_code = dc_create_setup_code(context);
    if !setup_code.is_null() {
        /* this may require a keypair to be created. this may take a second ... */
        if !context
            .running_state
            .clone()
            .read()
            .unwrap()
            .shall_stop_ongoing
        {
            setup_file_content = dc_render_setup_file(context, setup_code);
            if !setup_file_content.is_null() {
                /* encrypting may also take a while ... */
                if !context
                    .running_state
                    .clone()
                    .read()
                    .unwrap()
                    .shall_stop_ongoing
                {
                    setup_file_name = dc_get_fine_pathNfilename(
                        context,
                        b"$BLOBDIR\x00" as *const u8 as *const libc::c_char,
                        b"autocrypt-setup-message.html\x00" as *const u8 as *const libc::c_char,
                    );
                    if !(setup_file_name.is_null()
                        || 0 == dc_write_file(
                            context,
                            setup_file_name,
                            setup_file_content as *const libc::c_void,
                            strlen(setup_file_content),
                        ))
                    {
                        chat_id = dc_create_chat_by_contact_id(context, 1i32 as uint32_t);
                        if !(chat_id == 0i32 as libc::c_uint) {
                            msg = dc_msg_new_untyped(context);
                            (*msg).type_0 = 60i32;
                            dc_param_set((*msg).param, 'f' as i32, setup_file_name);
                            dc_param_set(
                                (*msg).param,
                                'm' as i32,
                                b"application/autocrypt-setup\x00" as *const u8
                                    as *const libc::c_char,
                            );
                            dc_param_set_int((*msg).param, 'S' as i32, 6i32);
                            dc_param_set_int((*msg).param, 'u' as i32, 2i32);
                            if !context
                                .running_state
                                .clone()
                                .read()
                                .unwrap()
                                .shall_stop_ongoing
                            {
                                msg_id = dc_send_msg(context, chat_id, msg);
                                if !(msg_id == 0i32 as libc::c_uint) {
                                    dc_msg_unref(msg);
                                    msg = 0 as *mut dc_msg_t;
                                    dc_log_info(
                                        context,
                                        0i32,
                                        b"Wait for setup message being sent ...\x00" as *const u8
                                            as *const libc::c_char,
                                    );
                                    loop {
                                        if context
                                            .running_state
                                            .clone()
                                            .read()
                                            .unwrap()
                                            .shall_stop_ongoing
                                        {
                                            current_block = 6116957410927263949;
                                            break;
                                        }
                                        std::thread::sleep(std::time::Duration::from_secs(1));
                                        msg = dc_get_msg(context, msg_id);
                                        if 0 != dc_msg_is_sent(msg) {
                                            current_block = 6450636197030046351;
                                            break;
                                        }
                                        dc_msg_unref(msg);
                                        msg = 0 as *mut dc_msg_t
                                    }
                                    match current_block {
                                        6116957410927263949 => {}
                                        _ => {
                                            dc_log_info(
                                                context,
                                                0i32,
                                                b"... setup message sent.\x00" as *const u8
                                                    as *const libc::c_char,
                                            );
                                            success = 1i32
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
    if 0 == success {
        free(setup_code as *mut libc::c_void);
        setup_code = 0 as *mut libc::c_char
    }
    free(setup_file_name as *mut libc::c_void);
    free(setup_file_content as *mut libc::c_void);
    dc_msg_unref(msg);
    dc_free_ongoing(context);

    setup_code
}

pub unsafe extern "C" fn dc_render_setup_file(
    context: &Context,
    passphrase: *const libc::c_char,
) -> *mut libc::c_char {
    let stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;

    let mut passphrase_begin: [libc::c_char; 8] = [0; 8];
    let mut ret_setupfilecontent: *mut libc::c_char = 0 as *mut libc::c_char;
    if !(passphrase.is_null() || strlen(passphrase) < 2) {
        strncpy(passphrase_begin.as_mut_ptr(), passphrase, 2);
        passphrase_begin[2usize] = 0i32 as libc::c_char;
        /* create the payload */
        if !(0 == dc_ensure_secret_key_exists(context)) {
            let self_addr = dc_sqlite3_get_config(context, &context.sql, "configured_addr", None)
                .unwrap_or_default();
            let curr_private_key = Key::from_self_private(context, self_addr, &context.sql);
            let e2ee_enabled = dc_sqlite3_get_config_int(context, &context.sql, "e2ee_enabled", 1);

            let headers = if 0 != e2ee_enabled {
                Some(("Autocrypt-Prefer-Encrypt", "mutual"))
            } else {
                None
            };

            if let Some(payload_key_asc) = curr_private_key.map(|k| k.to_asc_c(headers)) {
                if let Some(encr) = dc_pgp_symm_encrypt(
                    passphrase,
                    payload_key_asc as *const libc::c_void,
                    strlen(payload_key_asc),
                ) {
                    let encr_string_c = CString::new(encr).unwrap();
                    let mut encr_string = strdup(encr_string_c.as_ptr());

                    free(payload_key_asc as *mut libc::c_void);
                    let  replacement: *mut libc::c_char =
                        dc_mprintf(b"-----BEGIN PGP MESSAGE-----\r\nPassphrase-Format: numeric9x4\r\nPassphrase-Begin: %s\x00"
                                       as *const u8 as *const libc::c_char,
                                   passphrase_begin.as_mut_ptr());
                    dc_str_replace(
                        &mut encr_string,
                        b"-----BEGIN PGP MESSAGE-----\x00" as *const u8 as *const libc::c_char,
                        replacement,
                    );
                    free(replacement as *mut libc::c_void);
                    let setup_message_title: *mut libc::c_char = dc_stock_str(context, 42i32);
                    let mut setup_message_body: *mut libc::c_char = dc_stock_str(context, 43i32);
                    dc_str_replace(
                        &mut setup_message_body,
                        b"\r\x00" as *const u8 as *const libc::c_char,
                        0 as *const libc::c_char,
                    );
                    dc_str_replace(
                        &mut setup_message_body,
                        b"\n\x00" as *const u8 as *const libc::c_char,
                        b"<br>\x00" as *const u8 as *const libc::c_char,
                    );
                    ret_setupfilecontent =
                        dc_mprintf(b"<!DOCTYPE html>\r\n<html>\r\n<head>\r\n<title>%s</title>\r\n</head>\r\n<body>\r\n<h1>%s</h1>\r\n<p>%s</p>\r\n<pre>\r\n%s\r\n</pre>\r\n</body>\r\n</html>\r\n\x00"
                                       as *const u8 as *const libc::c_char,
                                   setup_message_title, setup_message_title,
                                   setup_message_body, encr_string);
                    free(setup_message_title as *mut libc::c_void);
                    free(setup_message_body as *mut libc::c_void);
                    free(encr_string as *mut libc::c_void);
                }
            }
        }
    }
    sqlite3_finalize(stmt);

    ret_setupfilecontent
}

pub unsafe fn dc_create_setup_code(_context: &Context) -> *mut libc::c_char {
    let mut random_val: uint16_t;
    let mut rng = thread_rng();
    let mut ret = String::new();

    for i in 0..9 {
        loop {
            random_val = rng.gen();
            if !(random_val as libc::c_int > 60000) {
                break;
            }
        }
        random_val = (random_val as libc::c_int % 10000) as uint16_t;
        ret += &format!(
            "{}{:04}",
            if 0 != i { "-" } else { "" },
            random_val as libc::c_int,
        );
    }

    strdup(to_cstring(ret).as_ptr())
}

// TODO should return bool /rtn
pub unsafe fn dc_continue_key_transfer(
    context: &Context,
    msg_id: uint32_t,
    setup_code: *const libc::c_char,
) -> libc::c_int {
    let mut success: libc::c_int = 0i32;
    let mut msg: *mut dc_msg_t = 0 as *mut dc_msg_t;
    let mut filename: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut filecontent: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut filebytes: size_t = 0i32 as size_t;
    let mut armored_key: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut norm_sc: *mut libc::c_char = 0 as *mut libc::c_char;
    if !(msg_id <= 9i32 as libc::c_uint || setup_code.is_null()) {
        msg = dc_get_msg(context, msg_id);
        if msg.is_null()
            || !dc_msg_is_setupmessage(msg)
            || {
                filename = dc_msg_get_file(msg);
                filename.is_null()
            }
            || *filename.offset(0isize) as libc::c_int == 0i32
        {
            error!(context, 0, "Message is no Autocrypt Setup Message.",);
        } else if 0
            == dc_read_file(
                context,
                filename,
                &mut filecontent as *mut *mut libc::c_char as *mut *mut libc::c_void,
                &mut filebytes,
            )
            || filecontent.is_null()
            || filebytes <= 0
        {
            error!(context, 0, "Cannot read Autocrypt Setup Message file.",);
        } else {
            norm_sc = dc_normalize_setup_code(context, setup_code);
            if norm_sc.is_null() {
                warn!(context, 0, "Cannot normalize Setup Code.",);
            } else {
                armored_key = dc_decrypt_setup_file(context, norm_sc, filecontent);
                if armored_key.is_null() {
                    warn!(context, 0, "Cannot decrypt Autocrypt Setup Message.",);
                } else if !(0 == set_self_key(context, armored_key, 1i32)) {
                    /*set default*/
                    /* error already logged */
                    success = 1i32
                }
            }
        }
    }
    free(armored_key as *mut libc::c_void);
    free(filecontent as *mut libc::c_void);
    free(filename as *mut libc::c_void);
    dc_msg_unref(msg);
    free(norm_sc as *mut libc::c_void);

    success
}

// TODO should return bool /rtn
fn set_self_key(
    context: &Context,
    armored_c: *const libc::c_char,
    set_default: libc::c_int,
) -> libc::c_int {
    assert!(!armored_c.is_null(), "invalid buffer");
    let armored = as_str(armored_c);

    let keys = Key::from_armored_string(armored, KeyType::Private)
        .and_then(|(k, h)| if k.verify() { Some((k, h)) } else { None })
        .and_then(|(k, h)| k.split_key().map(|pub_key| (k, pub_key, h)));

    if keys.is_none() {
        error!(context, 0, "File does not contain a valid private key.",);
        return 0;
    }

    let (private_key, public_key, header) = keys.unwrap();
    let preferencrypt = header.get("Autocrypt-Prefer-Encrypt");

    if !dc_sqlite3_execute(
        context,
        &context.sql,
        "DELETE FROM keypairs WHERE public_key=? OR private_key=?;",
        params![public_key.to_bytes(), private_key.to_bytes()],
    ) {
        return 0;
    }

    if 0 != set_default {
        if !dc_sqlite3_execute(
            context,
            &context.sql,
            "UPDATE keypairs SET is_default=0;",
            params![],
        ) {
            return 0;
        }
    } else {
        error!(context, 0, "File does not contain a private key.",);
    }

    let self_addr = dc_sqlite3_get_config(context, &context.sql, "configured_addr", None);

    if self_addr.is_none() {
        error!(context, 0, "Missing self addr");
        return 0;
    }

    if !dc_key_save_self_keypair(
        context,
        &public_key,
        &private_key,
        self_addr.unwrap(),
        set_default,
        &context.sql,
    ) {
        error!(context, 0, "Cannot save keypair.");
        return 0;
    }

    match preferencrypt.map(|s| s.as_str()) {
        Some("") => 0,
        Some("nopreference") => dc_sqlite3_set_config_int(context, &context.sql, "e2ee_enabled", 0),
        Some("mutual") => dc_sqlite3_set_config_int(context, &context.sql, "e2ee_enabled", 1),
        _ => 1,
    }
}

pub unsafe fn dc_decrypt_setup_file(
    _context: &Context,
    passphrase: *const libc::c_char,
    filecontent: *const libc::c_char,
) -> *mut libc::c_char {
    let fc_buf: *mut libc::c_char;
    let mut fc_headerline: *const libc::c_char = 0 as *const libc::c_char;
    let mut fc_base64: *const libc::c_char = 0 as *const libc::c_char;
    let mut binary: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut binary_bytes: size_t = 0i32 as size_t;
    let mut indx: size_t = 0i32 as size_t;

    let mut payload: *mut libc::c_char = 0 as *mut libc::c_char;
    fc_buf = dc_strdup(filecontent);
    if !(0
        == dc_split_armored_data(
            fc_buf,
            &mut fc_headerline,
            0 as *mut *const libc::c_char,
            0 as *mut *const libc::c_char,
            &mut fc_base64,
        )
        || fc_headerline.is_null()
        || strcmp(
            fc_headerline,
            b"-----BEGIN PGP MESSAGE-----\x00" as *const u8 as *const libc::c_char,
        ) != 0i32
        || fc_base64.is_null())
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
            if let Some(plain) =
                dc_pgp_symm_decrypt(passphrase, binary as *const libc::c_void, binary_bytes)
            {
                payload = strdup(CString::new(plain).unwrap().as_ptr());
            }
        }
    }

    free(fc_buf as *mut libc::c_void);
    if !binary.is_null() {
        mmap_string_unref(binary);
    }

    payload
}

pub unsafe fn dc_normalize_setup_code(
    _context: &Context,
    in_0: *const libc::c_char,
) -> *mut libc::c_char {
    if in_0.is_null() {
        return 0 as *mut libc::c_char;
    }
    let mut out = String::new();
    let mut outlen;
    let mut p1: *const libc::c_char = in_0;
    while 0 != *p1 {
        if *p1 as libc::c_int >= '0' as i32 && *p1 as libc::c_int <= '9' as i32 {
            out += &format!("{}", *p1 as i32 as u8 as char);
            outlen = out.len();
            if outlen == 4
                || outlen == 9
                || outlen == 14
                || outlen == 19
                || outlen == 24
                || outlen == 29
                || outlen == 34
                || outlen == 39
            {
                out += "-";
            }
        }
        p1 = p1.offset(1);
    }

    strdup(to_cstring(out).as_ptr())
}

pub unsafe fn dc_job_do_DC_JOB_IMEX_IMAP(context: &Context, job: *mut dc_job_t) {
    let mut current_block: u64;
    let mut success: libc::c_int = 0i32;
    let mut ongoing_allocated_here: libc::c_int = 0i32;
    let what: libc::c_int;
    let mut param1: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut param2: *mut libc::c_char = 0 as *mut libc::c_char;
    if !(0 == dc_alloc_ongoing(context)) {
        ongoing_allocated_here = 1i32;
        what = dc_param_get_int((*job).param, 'S' as i32, 0i32);
        param1 = dc_param_get((*job).param, 'E' as i32, 0 as *const libc::c_char);
        param2 = dc_param_get((*job).param, 'F' as i32, 0 as *const libc::c_char);
        if param1.is_null() {
            dc_log_error(
                context,
                0i32,
                b"No Import/export dir/file given.\x00" as *const u8 as *const libc::c_char,
            );
        } else {
            dc_log_info(
                context,
                0i32,
                b"Import/export process started.\x00" as *const u8 as *const libc::c_char,
            );
            context.call_cb(Event::IMEX_PROGRESS, 10i32 as uintptr_t, 0i32 as uintptr_t);
            if !context.sql.is_open() {
                dc_log_error(
                    context,
                    0i32,
                    b"Import/export: Database not opened.\x00" as *const u8 as *const libc::c_char,
                );
            } else {
                if what == 1i32 || what == 11i32 {
                    /* before we export anything, make sure the private key exists */
                    if 0 == dc_ensure_secret_key_exists(context) {
                        dc_log_error(context, 0i32,
                                         b"Import/export: Cannot create private key or private key not available.\x00"
                                             as *const u8 as
                                             *const libc::c_char);
                        current_block = 3568988166330621280;
                    } else {
                        dc_create_folder(context, param1);
                        current_block = 4495394744059808450;
                    }
                } else {
                    current_block = 4495394744059808450;
                }
                match current_block {
                    3568988166330621280 => {}
                    _ => match what {
                        1 => {
                            current_block = 10991094515395304355;
                            match current_block {
                                2973387206439775448 => {
                                    if 0 == import_backup(context, param1) {
                                        current_block = 3568988166330621280;
                                    } else {
                                        current_block = 1118134448028020070;
                                    }
                                }
                                11250025114629486028 => {
                                    if 0 == import_self_keys(context, param1) {
                                        current_block = 3568988166330621280;
                                    } else {
                                        current_block = 1118134448028020070;
                                    }
                                }
                                12669919903773909120 => {
                                    if 0 == export_backup(context, param1) {
                                        current_block = 3568988166330621280;
                                    } else {
                                        current_block = 1118134448028020070;
                                    }
                                }
                                _ => {
                                    if 0 == export_self_keys(context, param1) {
                                        current_block = 3568988166330621280;
                                    } else {
                                        current_block = 1118134448028020070;
                                    }
                                }
                            }
                            match current_block {
                                3568988166330621280 => {}
                                _ => {
                                    dc_log_info(
                                        context,
                                        0i32,
                                        b"Import/export completed.\x00" as *const u8
                                            as *const libc::c_char,
                                    );
                                    success = 1i32
                                }
                            }
                        }
                        2 => {
                            current_block = 11250025114629486028;
                            match current_block {
                                2973387206439775448 => {
                                    if 0 == import_backup(context, param1) {
                                        current_block = 3568988166330621280;
                                    } else {
                                        current_block = 1118134448028020070;
                                    }
                                }
                                11250025114629486028 => {
                                    if 0 == import_self_keys(context, param1) {
                                        current_block = 3568988166330621280;
                                    } else {
                                        current_block = 1118134448028020070;
                                    }
                                }
                                12669919903773909120 => {
                                    if 0 == export_backup(context, param1) {
                                        current_block = 3568988166330621280;
                                    } else {
                                        current_block = 1118134448028020070;
                                    }
                                }
                                _ => {
                                    if 0 == export_self_keys(context, param1) {
                                        current_block = 3568988166330621280;
                                    } else {
                                        current_block = 1118134448028020070;
                                    }
                                }
                            }
                            match current_block {
                                3568988166330621280 => {}
                                _ => {
                                    dc_log_info(
                                        context,
                                        0i32,
                                        b"Import/export completed.\x00" as *const u8
                                            as *const libc::c_char,
                                    );
                                    success = 1i32
                                }
                            }
                        }
                        11 => {
                            current_block = 12669919903773909120;
                            match current_block {
                                2973387206439775448 => {
                                    if 0 == import_backup(context, param1) {
                                        current_block = 3568988166330621280;
                                    } else {
                                        current_block = 1118134448028020070;
                                    }
                                }
                                11250025114629486028 => {
                                    if 0 == import_self_keys(context, param1) {
                                        current_block = 3568988166330621280;
                                    } else {
                                        current_block = 1118134448028020070;
                                    }
                                }
                                12669919903773909120 => {
                                    if 0 == export_backup(context, param1) {
                                        current_block = 3568988166330621280;
                                    } else {
                                        current_block = 1118134448028020070;
                                    }
                                }
                                _ => {
                                    if 0 == export_self_keys(context, param1) {
                                        current_block = 3568988166330621280;
                                    } else {
                                        current_block = 1118134448028020070;
                                    }
                                }
                            }
                            match current_block {
                                3568988166330621280 => {}
                                _ => {
                                    dc_log_info(
                                        context,
                                        0i32,
                                        b"Import/export completed.\x00" as *const u8
                                            as *const libc::c_char,
                                    );
                                    success = 1i32
                                }
                            }
                        }
                        12 => {
                            current_block = 2973387206439775448;
                            match current_block {
                                2973387206439775448 => {
                                    if 0 == import_backup(context, param1) {
                                        current_block = 3568988166330621280;
                                    } else {
                                        current_block = 1118134448028020070;
                                    }
                                }
                                11250025114629486028 => {
                                    if 0 == import_self_keys(context, param1) {
                                        current_block = 3568988166330621280;
                                    } else {
                                        current_block = 1118134448028020070;
                                    }
                                }
                                12669919903773909120 => {
                                    if 0 == export_backup(context, param1) {
                                        current_block = 3568988166330621280;
                                    } else {
                                        current_block = 1118134448028020070;
                                    }
                                }
                                _ => {
                                    if 0 == export_self_keys(context, param1) {
                                        current_block = 3568988166330621280;
                                    } else {
                                        current_block = 1118134448028020070;
                                    }
                                }
                            }
                            match current_block {
                                3568988166330621280 => {}
                                _ => {
                                    dc_log_info(
                                        context,
                                        0i32,
                                        b"Import/export completed.\x00" as *const u8
                                            as *const libc::c_char,
                                    );
                                    success = 1i32
                                }
                            }
                        }
                        _ => {}
                    },
                }
            }
        }
    }

    free(param1 as *mut libc::c_void);
    free(param2 as *mut libc::c_void);
    if 0 != ongoing_allocated_here {
        dc_free_ongoing(context);
    }
    context.call_cb(
        Event::IMEX_PROGRESS,
        (if 0 != success { 1000i32 } else { 0i32 }) as uintptr_t,
        0i32 as uintptr_t,
    );
}

/*******************************************************************************
 * Import backup
 ******************************************************************************/

// TODO should return bool /rtn
unsafe fn import_backup(context: &Context, backup_to_import: *const libc::c_char) -> libc::c_int {
    info!(
        context,
        0,
        "Import \"{}\" to \"{}\".",
        as_str(backup_to_import),
        as_str(context.get_dbfile()),
    );

    if 0 != dc_is_configured(context) {
        error!(context, 0, "Cannot import backups to accounts in use.");
        return 0;
    }
    &context.sql.close(&context);
    dc_delete_file(context, context.get_dbfile());
    if 0 != dc_file_exist(context, context.get_dbfile()) {
        error!(
            context,
            0, "Cannot import backups: Cannot delete the old file.",
        );
        return 0;
    }

    if 0 == dc_copy_file(context, backup_to_import, context.get_dbfile()) {
        return 0;
    }
    /* error already logged */
    /* re-open copied database file */
    if !context.sql.open(&context, as_path(context.get_dbfile()), 0) {
        return 0;
    }

    let total_files_cnt = dc_sqlite3_query_row::<_, isize>(
        context,
        &context.sql,
        "SELECT COUNT(*) FROM backup_blobs;",
        params![],
        0,
    )
    .unwrap_or_default() as usize;
    info!(
        context,
        0, "***IMPORT-in-progress: total_files_cnt={:?}", total_files_cnt,
    );

    context
        .sql
        .query_map(
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
                    if file.is_err() {
                        loop_success = false;
                        break;
                    }
                    let (file_name, file_blob) = file.unwrap();

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
                    context.call_cb(Event::IMEX_PROGRESS, permille as uintptr_t, 0);
                    if file_blob.is_empty() {
                        continue;
                    }

                    let pathNfilename = format!("{}/{}", as_str(context.get_blobdir()), file_name);
                    if dc_write_file_safe(context, &pathNfilename, &file_blob) {
                        continue;
                    }

                    error!(
                        context,
                        0,
                        "Storage full? Cannot write file {} with {} bytes.",
                        &pathNfilename,
                        file_blob.len(),
                    );
                    // otherwise the user may believe the stuff is imported correctly, but there are files missing ...
                    loop_success = false;
                    break;
                }

                if !loop_success {
                    return Err(format_err!("fail").into());
                }
                dc_sqlite3_execute(context, &context.sql, "DROP TABLE backup_blobs;", params![]);
                dc_sqlite3_try_execute(context, &context.sql, "VACUUM;");
                Ok(())
            },
        )
        .is_ok() as libc::c_int
}

/*******************************************************************************
 * Export backup
 ******************************************************************************/
/* the FILE_PROGRESS macro calls the callback with the permille of files processed.
The macro avoids weird values of 0% or 100% while still working. */
// TODO should return bool /rtn
unsafe fn export_backup(context: &Context, dir: *const libc::c_char) -> libc::c_int {
    let mut current_block: u64;
    let mut success: libc::c_int = 0;

    let mut delete_dest_file: libc::c_int = 0;
    // get a fine backup file name (the name includes the date so that multiple backup instances are possible)
    // FIXME: we should write to a temporary file first and rename it on success. this would guarantee the backup is complete. however, currently it is not clear it the import exists in the long run (may be replaced by a restore-from-imap)
    let now = time();
    let res = chrono::NaiveDateTime::from_timestamp(now as i64, 0)
        .format("delta-chat-%Y-%m-%d.bak")
        .to_string();
    let buffer = to_cstring(res);
    let dest_pathNfilename = dc_get_fine_pathNfilename(context, dir, buffer.as_ptr());
    if dest_pathNfilename.is_null() {
        dc_log_error(
            context,
            0,
            b"Cannot get backup file name.\x00" as *const u8 as *const libc::c_char,
        );

        return success;
    }

    dc_housekeeping(context);

    dc_sqlite3_try_execute(context, &context.sql, "VACUUM;");
    context.sql.close(context);
    let mut closed = true;
    dc_log_info(
        context,
        0,
        b"Backup \"%s\" to \"%s\".\x00" as *const u8 as *const libc::c_char,
        context.get_dbfile(),
        dest_pathNfilename,
    );
    if !(0 == dc_copy_file(context, context.get_dbfile(), dest_pathNfilename)) {
        context.sql.open(&context, as_path(context.get_dbfile()), 0);
        closed = false;
        /* add all files as blobs to the database copy (this does not require the source to be locked, neigher the destination as it is used only here) */
        /*for logging only*/
        let sql = SQLite::new();
        if sql.open(context, as_path(dest_pathNfilename), 0) {
            if !sql.table_exists("backup_blobs") {
                if !dc_sqlite3_execute(
                    context,
                    &sql,
                    "CREATE TABLE backup_blobs (id INTEGER PRIMARY KEY, file_name, file_content);",
                    params![],
                ) {
                    /* error already logged */
                    current_block = 11487273724841241105;
                } else {
                    current_block = 14648156034262866959;
                }
            } else {
                current_block = 14648156034262866959;
            }
            match current_block {
                11487273724841241105 => {}
                _ => {
                    let mut total_files_cnt = 0;
                    let dir = std::path::Path::new(as_str(context.get_blobdir()));
                    let dir_handle = std::fs::read_dir(dir);
                    if dir_handle.is_err() {
                        dc_log_error(
                            context,
                            0,
                            b"Backup: Cannot get info for blob-directory \"%s\".\x00" as *const u8
                                as *const libc::c_char,
                            context.get_blobdir(),
                        );
                    } else {
                        let dir_handle = dir_handle.unwrap();
                        total_files_cnt += dir_handle.filter(|r| r.is_ok()).count();

                        info!(context, 0, "EXPORT: total_files_cnt={}", total_files_cnt);
                        if total_files_cnt > 0 {
                            // scan directory, pass 2: copy files
                            let dir_handle = std::fs::read_dir(dir);
                            if dir_handle.is_err() {
                                error!(
                                    context,
                                    0,
                                    "Backup: Cannot copy from blob-directory \"{}\".",
                                    as_str(context.get_blobdir()),
                                );
                            } else {
                                let dir_handle = dir_handle.unwrap();

                                sql.prepare(
                                    "INSERT INTO backup_blobs (file_name, file_content) VALUES (?, ?);",
                                    move |mut stmt| {
                                        let mut processed_files_cnt = 0;
                                        for entry in dir_handle {
                                            if entry.is_err() {
                                                current_block = 2631791190359682872;
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
                                                current_block = 11487273724841241105;
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
                                                context.call_cb(
                                                    Event::IMEX_PROGRESS,
                                                    permille as uintptr_t,
                                                    0 as uintptr_t,
                                                );

                                                let name_f = entry.file_name();
                                                let name = name_f.to_string_lossy();
                                                if name.starts_with("delta-chat") && name.ends_with(".bak")
                                                {
                                                    continue;
                                                } else {
                                                    info!(context, 0, "EXPORTing filename={}", name);
                                                    let curr_pathNfilename = format!(
                                                        "{}/{}",
                                                        as_str(context.get_blobdir()),
                                                        name
                                                    );

                                                    if let Some(buf) =
                                                        dc_read_file_safe(context, &curr_pathNfilename)
                                                    {
                                                        if buf.is_empty() {
                                                            continue;
                                                        }
                                                        if stmt.execute(params![name, buf]).is_err() {
                                                            error!(
                                                                context,
                                                                0,
                                                                "Disk full? Cannot add file \"{}\" to backup.",
                                                                &curr_pathNfilename,
                                                            );
                                                            /* this is not recoverable! writing to the sqlite database should work! */
                                                            current_block = 11487273724841241105;
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
                            }
                        } else {
                            info!(context, 0, "Backup: No files to copy.",);
                            current_block = 2631791190359682872;
                        }
                        match current_block {
                            11487273724841241105 => {}
                            _ => {
                                if 0 != dc_sqlite3_set_config_int(
                                    context,
                                    &sql,
                                    "backup_time",
                                    now as i32,
                                ) {
                                    context.call_cb(
                                        Event::IMEX_FILE_WRITTEN,
                                        dest_pathNfilename as uintptr_t,
                                        0,
                                    );
                                    success = 1;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    if closed {
        context.sql.open(&context, as_path(context.get_dbfile()), 0);
    }
    if 0 != delete_dest_file {
        dc_delete_file(context, dest_pathNfilename);
    }
    free(dest_pathNfilename as *mut libc::c_void);

    success
}

/*******************************************************************************
 * Classic key import
 ******************************************************************************/
unsafe fn import_self_keys(context: &Context, dir_name: *const libc::c_char) -> libc::c_int {
    /* hint: even if we switch to import Autocrypt Setup Files, we should leave the possibility to import
    plain ASC keys, at least keys without a password, if we do not want to implement a password entry function.
    Importing ASC keys is useful to use keys in Delta Chat used by any other non-Autocrypt-PGP implementation.

    Maybe we should make the "default" key handlong also a little bit smarter
    (currently, the last imported key is the standard key unless it contains the string "legacy" in its name) */
    let mut imported_cnt: libc::c_int = 0;
    let mut suffix: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut path_plus_name: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut set_default: libc::c_int;
    let mut buf: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut buf_bytes: size_t = 0 as size_t;
    // a pointer inside buf, MUST NOT be free()'d
    let mut private_key: *const libc::c_char;
    let mut buf2: *mut libc::c_char = 0 as *mut libc::c_char;
    // a pointer inside buf2, MUST NOT be free()'d
    let mut buf2_headerline: *const libc::c_char = 0 as *const libc::c_char;
    if !dir_name.is_null() {
        let dir = std::path::Path::new(as_str(dir_name));
        let dir_handle = std::fs::read_dir(dir);
        if dir_handle.is_err() {
            dc_log_error(
                context,
                0,
                b"Import: Cannot open directory \"%s\".\x00" as *const u8 as *const libc::c_char,
                dir_name,
            );
        } else {
            let dir_handle = dir_handle.unwrap();
            for entry in dir_handle {
                if entry.is_err() {
                    break;
                }
                let entry = entry.unwrap();
                free(suffix as *mut libc::c_void);
                let name_f = entry.file_name();
                let name_c = to_cstring(name_f.to_string_lossy());
                suffix = dc_get_filesuffix_lc(name_c.as_ptr());
                if suffix.is_null()
                    || strcmp(suffix, b"asc\x00" as *const u8 as *const libc::c_char) != 0
                {
                    continue;
                }
                free(path_plus_name as *mut libc::c_void);
                path_plus_name = dc_mprintf(
                    b"%s/%s\x00" as *const u8 as *const libc::c_char,
                    dir_name,
                    name_c.as_ptr(),
                );
                dc_log_info(
                    context,
                    0,
                    b"Checking: %s\x00" as *const u8 as *const libc::c_char,
                    path_plus_name,
                );
                free(buf as *mut libc::c_void);
                buf = 0 as *mut libc::c_char;
                if 0 == dc_read_file(
                    context,
                    path_plus_name,
                    &mut buf as *mut *mut libc::c_char as *mut *mut libc::c_void,
                    &mut buf_bytes,
                ) || buf_bytes < 50
                {
                    continue;
                }
                private_key = buf;
                free(buf2 as *mut libc::c_void);
                buf2 = dc_strdup(buf);
                if 0 != dc_split_armored_data(
                    buf2,
                    &mut buf2_headerline,
                    0 as *mut *const libc::c_char,
                    0 as *mut *const libc::c_char,
                    0 as *mut *const libc::c_char,
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
                if !strstr(
                    name_c.as_ptr(),
                    b"legacy\x00" as *const u8 as *const libc::c_char,
                )
                .is_null()
                {
                    dc_log_info(
                        context,
                        0i32,
                        b"Treating \"%s\" as a legacy private key.\x00" as *const u8
                            as *const libc::c_char,
                        path_plus_name,
                    );
                    set_default = 0i32
                }
                if 0 == set_self_key(context, private_key, set_default) {
                    continue;
                }
                imported_cnt += 1
            }
            if imported_cnt == 0i32 {
                dc_log_error(
                    context,
                    0i32,
                    b"No private keys found in \"%s\".\x00" as *const u8 as *const libc::c_char,
                    dir_name,
                );
            }
        }
    }

    free(suffix as *mut libc::c_void);
    free(path_plus_name as *mut libc::c_void);
    free(buf as *mut libc::c_void);
    free(buf2 as *mut libc::c_void);

    imported_cnt
}

// TODO should return bool /rtn
unsafe fn export_self_keys(context: &Context, dir: *const libc::c_char) -> libc::c_int {
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
                let is_default = row.get(3)?;

                Ok((id, public_key, private_key, is_default))
            },
            |keys| {
                for key_pair in keys {
                    let (id, public_key, private_key, is_default) = key_pair?;
                    if let Some(key) = public_key {
                        if 0 == export_key_to_asc_file(context, dir, id, &key, is_default) {
                            export_errors += 1;
                        }
                    } else {
                        export_errors += 1;
                    }
                    if let Some(key) = private_key {
                        if 0 == export_key_to_asc_file(context, dir, id, &key, is_default) {
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

    if export_errors == 0 {
        1
    } else {
        0
    }
}

/*******************************************************************************
 * Classic key export
 ******************************************************************************/
// TODO should return bool /rtn
unsafe fn export_key_to_asc_file(
    context: &Context,
    dir: *const libc::c_char,
    id: libc::c_int,
    key: &Key,
    is_default: libc::c_int,
) -> libc::c_int {
    let mut success: libc::c_int = 0i32;
    let file_name;
    if 0 != is_default {
        file_name = dc_mprintf(
            b"%s/%s-key-default.asc\x00" as *const u8 as *const libc::c_char,
            dir,
            if key.is_public() {
                b"public\x00" as *const u8 as *const libc::c_char
            } else {
                b"private\x00" as *const u8 as *const libc::c_char
            },
        )
    } else {
        file_name = dc_mprintf(
            b"%s/%s-key-%i.asc\x00" as *const u8 as *const libc::c_char,
            dir,
            if key.is_public() {
                b"public\x00" as *const u8 as *const libc::c_char
            } else {
                b"private\x00" as *const u8 as *const libc::c_char
            },
            id,
        )
    }
    dc_log_info(
        context,
        0i32,
        b"Exporting key %s\x00" as *const u8 as *const libc::c_char,
        file_name,
    );
    dc_delete_file(context, file_name);
    if !key.write_asc_to_file(file_name, context) {
        dc_log_error(
            context,
            0i32,
            b"Cannot write key to %s\x00" as *const u8 as *const libc::c_char,
            file_name,
        );
    } else {
        context.call_cb(
            Event::IMEX_FILE_WRITTEN,
            file_name as uintptr_t,
            0i32 as uintptr_t,
        );
        success = 1i32
    }
    free(file_name as *mut libc::c_void);

    success
}
