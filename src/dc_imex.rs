use std::ffi::CString;

use mmime::mailmime_content::*;
use mmime::mmapstring::*;
use mmime::other::*;
use rand::{thread_rng, Rng};

use crate::config::Config;
use crate::constants::*;
use crate::context::Context;
use crate::dc_chat::*;
use crate::dc_configure::*;
use crate::dc_e2ee::*;
use crate::dc_job::*;
use crate::dc_msg::*;
use crate::dc_tools::*;
use crate::key::*;
use crate::param::*;
use crate::pgp::*;
use crate::sql::{self, Sql};
use crate::stock::StockMessage;
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
    let mut param = Params::new();
    param.set_int(Param::Cmd, what as i32);
    if !param1.is_null() {
        param.set(Param::Arg, as_str(param1));
    }
    if !param2.is_null() {
        param.set(Param::Arg2, as_str(param2));
    }

    dc_job_kill_action(context, 910);
    dc_job_add(context, 910, 0, param, 0);
}

/// Returns the filename of the backup if found, nullptr otherwise.
pub unsafe fn dc_imex_has_backup(
    context: &Context,
    dir_name: *const libc::c_char,
) -> *mut libc::c_char {
    let dir_name = as_path(dir_name);
    let dir_iter = std::fs::read_dir(dir_name);
    if dir_iter.is_err() {
        info!(
            context,
            0,
            "Backup check: Cannot open directory \"{}\".\x00",
            dir_name.display(),
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
        Some(path) => match path.to_c_string() {
            Ok(cstr) => dc_strdup(cstr.as_ptr()),
            Err(err) => {
                error!(context, 0, "Invalid backup filename: {}", err);
                std::ptr::null_mut()
            }
        },
        None => std::ptr::null_mut(),
    }
}

pub unsafe fn dc_initiate_key_transfer(context: &Context) -> *mut libc::c_char {
    let mut setup_file_content: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut setup_file_name: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut msg: *mut dc_msg_t = 0 as *mut dc_msg_t;
    if dc_alloc_ongoing(context) == 0 {
        return std::ptr::null_mut();
    }
    let setup_code = CString::new(dc_create_setup_code(context)).unwrap();
    /* this may require a keypair to be created. this may take a second ... */
    if !context
        .running_state
        .clone()
        .read()
        .unwrap()
        .shall_stop_ongoing
    {
        setup_file_content = dc_render_setup_file(context, setup_code.as_ptr());
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
                    let chat_id = dc_create_chat_by_contact_id(context, 1i32 as uint32_t);
                    if !(chat_id == 0i32 as libc::c_uint) {
                        msg = dc_msg_new_untyped(context);
                        (*msg).type_0 = Viewtype::File;
                        (*msg).param.set(Param::File, as_str(setup_file_name));

                        (*msg)
                            .param
                            .set(Param::MimeType, "application/autocrypt-setup");
                        (*msg).param.set_int(Param::Cmd, 6);
                        (*msg).param.set_int(Param::ForcePlaintext, 2);

                        if !context
                            .running_state
                            .clone()
                            .read()
                            .unwrap()
                            .shall_stop_ongoing
                        {
                            let msg_id = dc_send_msg(context, chat_id, msg);
                            if msg_id != 0 {
                                dc_msg_unref(msg);
                                msg = 0 as *mut dc_msg_t;
                                info!(context, 0, "Wait for setup message being sent ...",);
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
                                    msg = dc_get_msg(context, msg_id);
                                    if 0 != dc_msg_is_sent(msg) {
                                        info!(context, 0, "... setup message sent.",);
                                        break;
                                    }
                                    dc_msg_unref(msg);
                                    msg = 0 as *mut dc_msg_t
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    free(setup_file_name as *mut libc::c_void);
    free(setup_file_content as *mut libc::c_void);
    dc_msg_unref(msg);
    dc_free_ongoing(context);

    dc_strdup(setup_code.as_ptr())
}

pub unsafe fn dc_render_setup_file(
    context: &Context,
    passphrase: *const libc::c_char,
) -> *mut libc::c_char {
    let mut ret_setupfilecontent: *mut libc::c_char = 0 as *mut libc::c_char;
    if !(passphrase.is_null() || strlen(passphrase) < 2) {
        /* create the payload */
        if !(0 == dc_ensure_secret_key_exists(context)) {
            let self_addr = context
                .sql
                .get_config(context, Config::ConfiguredAddr)
                .unwrap_or_default();
            let curr_private_key = Key::from_self_private(context, self_addr, &context.sql);
            let e2ee_enabled = context
                .sql
                .get_config_int(context, Config::E2eeEnabled)
                .unwrap_or_else(|| 1);

            let headers = if 0 != e2ee_enabled {
                Some(("Autocrypt-Prefer-Encrypt", "mutual"))
            } else {
                None
            };

            if let Some(payload_key_asc) = curr_private_key.map(|k| k.to_asc(headers)) {
                let payload_key_asc_c = CString::new(payload_key_asc).unwrap();
                if let Some(encr) = dc_pgp_symm_encrypt(
                    passphrase,
                    payload_key_asc_c.as_ptr() as *const libc::c_void,
                    payload_key_asc_c.as_bytes().len(),
                ) {
                    let replacement = format!(
                        concat!(
                            "-----BEGIN PGP MESSAGE-----\r\n",
                            "Passphrase-Format: numeric9x4\r\n",
                            "Passphrase-Begin: {}"
                        ),
                        &as_str(passphrase)[..2]
                    );
                    let pgp_msg = encr.replace("-----BEGIN PGP MESSAGE-----", &replacement);

                    let msg_subj = context.stock_str(StockMessage::AcSetupMsgSubject);
                    let msg_body = context.stock_str(StockMessage::AcSetupMsgBody);
                    let msg_body_head: &str = msg_body.split('\r').next().unwrap();
                    let msg_body_html = msg_body_head.replace("\n", "<br>");
                    ret_setupfilecontent = to_cstring(format!(
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
                    ));
                }
            }
        }
    }
    ret_setupfilecontent
}

pub fn dc_create_setup_code(_context: &Context) -> String {
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

    ret
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

    if sql::execute(
        context,
        &context.sql,
        "DELETE FROM keypairs WHERE public_key=? OR private_key=?;",
        params![public_key.to_bytes(), private_key.to_bytes()],
    )
    .is_err()
    {
        return 0;
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
            return 0;
        }
    } else {
        error!(context, 0, "File does not contain a private key.",);
    }

    let self_addr = context.sql.get_config(context, "configured_addr");

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
        Some("nopreference") => context
            .sql
            .set_config_int(context, "e2ee_enabled", 0)
            .is_ok() as libc::c_int,
        Some("mutual") => context
            .sql
            .set_config_int(context, "e2ee_enabled", 1)
            .is_ok() as libc::c_int,
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
    if dc_split_armored_data(
        fc_buf,
        &mut fc_headerline,
        0 as *mut *const libc::c_char,
        0 as *mut *const libc::c_char,
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
            if let Some(plain) =
                dc_pgp_symm_decrypt(passphrase, binary as *const libc::c_void, binary_bytes)
            {
                let payload_c = CString::new(plain).unwrap();
                payload = strdup(payload_c.as_ptr());
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

    out.strdup()
}

#[allow(non_snake_case)]
pub unsafe fn dc_job_do_DC_JOB_IMEX_IMAP(context: &Context, job: *mut dc_job_t) {
    let mut current_block: u64;
    let mut success: libc::c_int = 0;
    let mut ongoing_allocated_here: libc::c_int = 0;
    let what: libc::c_int;

    if !(0 == dc_alloc_ongoing(context)) {
        ongoing_allocated_here = 1;
        what = (*job).param.get_int(Param::Cmd).unwrap_or_default();
        let param1 = CString::yolo((*job).param.get(Param::Arg).unwrap_or_default());
        let _param2 = CString::yolo((*job).param.get(Param::Arg2).unwrap_or_default());

        if strlen(param1.as_ptr()) == 0 {
            error!(context, 0, "No Import/export dir/file given.",);
        } else {
            info!(context, 0, "Import/export process started.",);
            context.call_cb(Event::IMEX_PROGRESS, 10 as uintptr_t, 0 as uintptr_t);
            if !context.sql.is_open() {
                error!(context, 0, "Import/export: Database not opened.",);
            } else {
                if what == 1 || what == 11 {
                    /* before we export anything, make sure the private key exists */
                    if 0 == dc_ensure_secret_key_exists(context) {
                        error!(
                            context,
                            0,
                            "Import/export: Cannot create private key or private key not available.",
                        );
                        current_block = 3568988166330621280;
                    } else {
                        dc_create_folder(context, param1.as_ptr());
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
                                    if 0 == import_backup(context, param1.as_ptr()) {
                                        current_block = 3568988166330621280;
                                    } else {
                                        current_block = 1118134448028020070;
                                    }
                                }
                                11250025114629486028 => {
                                    if 0 == import_self_keys(context, param1.as_ptr()) {
                                        current_block = 3568988166330621280;
                                    } else {
                                        current_block = 1118134448028020070;
                                    }
                                }
                                12669919903773909120 => {
                                    if 0 == export_backup(context, param1.as_ptr()) {
                                        current_block = 3568988166330621280;
                                    } else {
                                        current_block = 1118134448028020070;
                                    }
                                }
                                _ => {
                                    if 0 == export_self_keys(context, param1.as_ptr()) {
                                        current_block = 3568988166330621280;
                                    } else {
                                        current_block = 1118134448028020070;
                                    }
                                }
                            }
                            match current_block {
                                3568988166330621280 => {}
                                _ => {
                                    info!(context, 0, "Import/export completed.",);
                                    success = 1
                                }
                            }
                        }
                        2 => {
                            current_block = 11250025114629486028;
                            match current_block {
                                2973387206439775448 => {
                                    if 0 == import_backup(context, param1.as_ptr()) {
                                        current_block = 3568988166330621280;
                                    } else {
                                        current_block = 1118134448028020070;
                                    }
                                }
                                11250025114629486028 => {
                                    if 0 == import_self_keys(context, param1.as_ptr()) {
                                        current_block = 3568988166330621280;
                                    } else {
                                        current_block = 1118134448028020070;
                                    }
                                }
                                12669919903773909120 => {
                                    if 0 == export_backup(context, param1.as_ptr()) {
                                        current_block = 3568988166330621280;
                                    } else {
                                        current_block = 1118134448028020070;
                                    }
                                }
                                _ => {
                                    if 0 == export_self_keys(context, param1.as_ptr()) {
                                        current_block = 3568988166330621280;
                                    } else {
                                        current_block = 1118134448028020070;
                                    }
                                }
                            }
                            match current_block {
                                3568988166330621280 => {}
                                _ => {
                                    info!(context, 0, "Import/export completed.",);
                                    success = 1
                                }
                            }
                        }
                        11 => {
                            current_block = 12669919903773909120;
                            match current_block {
                                2973387206439775448 => {
                                    if 0 == import_backup(context, param1.as_ptr()) {
                                        current_block = 3568988166330621280;
                                    } else {
                                        current_block = 1118134448028020070;
                                    }
                                }
                                11250025114629486028 => {
                                    if 0 == import_self_keys(context, param1.as_ptr()) {
                                        current_block = 3568988166330621280;
                                    } else {
                                        current_block = 1118134448028020070;
                                    }
                                }
                                12669919903773909120 => {
                                    if 0 == export_backup(context, param1.as_ptr()) {
                                        current_block = 3568988166330621280;
                                    } else {
                                        current_block = 1118134448028020070;
                                    }
                                }
                                _ => {
                                    if 0 == export_self_keys(context, param1.as_ptr()) {
                                        current_block = 3568988166330621280;
                                    } else {
                                        current_block = 1118134448028020070;
                                    }
                                }
                            }
                            match current_block {
                                3568988166330621280 => {}
                                _ => {
                                    info!(context, 0, "Import/export completed.",);
                                    success = 1
                                }
                            }
                        }
                        12 => {
                            current_block = 2973387206439775448;
                            match current_block {
                                2973387206439775448 => {
                                    if 0 == import_backup(context, param1.as_ptr()) {
                                        current_block = 3568988166330621280;
                                    } else {
                                        current_block = 1118134448028020070;
                                    }
                                }
                                11250025114629486028 => {
                                    if 0 == import_self_keys(context, param1.as_ptr()) {
                                        current_block = 3568988166330621280;
                                    } else {
                                        current_block = 1118134448028020070;
                                    }
                                }
                                12669919903773909120 => {
                                    if 0 == export_backup(context, param1.as_ptr()) {
                                        current_block = 3568988166330621280;
                                    } else {
                                        current_block = 1118134448028020070;
                                    }
                                }
                                _ => {
                                    if 0 == export_self_keys(context, param1.as_ptr()) {
                                        current_block = 3568988166330621280;
                                    } else {
                                        current_block = 1118134448028020070;
                                    }
                                }
                            }
                            match current_block {
                                3568988166330621280 => {}
                                _ => {
                                    info!(context, 0, "Import/export completed.",);
                                    success = 1
                                }
                            }
                        }
                        _ => {}
                    },
                }
            }
        }
    }

    if 0 != ongoing_allocated_here {
        dc_free_ongoing(context);
    }
    context.call_cb(
        Event::IMEX_PROGRESS,
        (if 0 != success { 1000 } else { 0 }) as uintptr_t,
        0 as uintptr_t,
    );
}

/*******************************************************************************
 * Import backup
 ******************************************************************************/

// TODO should return bool /rtn
#[allow(non_snake_case)]
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

    let total_files_cnt = context
        .sql
        .query_row_col::<_, isize>(context, "SELECT COUNT(*) FROM backup_blobs;", params![], 0)
        .unwrap_or_default() as usize;
    info!(
        context,
        0, "***IMPORT-in-progress: total_files_cnt={:?}", total_files_cnt,
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
    .is_ok() as libc::c_int
}

/*******************************************************************************
 * Export backup
 ******************************************************************************/
/* the FILE_PROGRESS macro calls the callback with the permille of files processed.
The macro avoids weird values of 0% or 100% while still working. */
// TODO should return bool /rtn
#[allow(non_snake_case)]
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
    let buffer = CString::yolo(res);
    let dest_pathNfilename = dc_get_fine_pathNfilename(context, dir, buffer.as_ptr());
    if dest_pathNfilename.is_null() {
        error!(context, 0, "Cannot get backup file name.",);

        return success;
    }

    sql::housekeeping(context);

    sql::try_execute(context, &context.sql, "VACUUM;").ok();
    context.sql.close(context);
    let mut closed = true;
    info!(
        context,
        0,
        "Backup \"{}\" to \"{}\".",
        as_str(context.get_dbfile()),
        as_str(dest_pathNfilename),
    );
    if !(0 == dc_copy_file(context, context.get_dbfile(), dest_pathNfilename)) {
        context.sql.open(&context, as_path(context.get_dbfile()), 0);
        closed = false;
        /* add all files as blobs to the database copy (this does not require the source to be locked, neigher the destination as it is used only here) */
        /*for logging only*/
        let sql = Sql::new();
        if sql.open(context, as_path(dest_pathNfilename), 0) {
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
                        error!(
                            context,
                            0,
                            "Backup: Cannot get info for blob-directory \"{}\".",
                            as_str(context.get_blobdir()),
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
                                    move |mut stmt, _| {
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
                                if sql
                                    .set_config_int(context, "backup_time", now as i32)
                                    .is_ok()
                                {
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
            error!(
                context,
                0,
                "Import: Cannot open directory \"{}\".",
                as_str(dir_name),
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
                let name_c = name_f.to_c_string().unwrap();
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
                info!(context, 0, "Checking: {}", as_str(path_plus_name));
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
                if dc_split_armored_data(
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
                    info!(
                        context,
                        0,
                        "Treating \"{}\" as a legacy private key.",
                        as_str(path_plus_name),
                    );
                    set_default = 0i32
                }
                if 0 == set_self_key(context, private_key, set_default) {
                    continue;
                }
                imported_cnt += 1
            }
            if imported_cnt == 0i32 {
                error!(
                    context,
                    0,
                    "No private keys found in \"{}\".",
                    as_str(dir_name),
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
    info!(context, 0, "Exporting key {}", as_str(file_name),);
    dc_delete_file(context, file_name);
    if !key.write_asc_to_file(file_name, context) {
        error!(context, 0, "Cannot write key to {}", as_str(file_name),);
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

#[cfg(test)]
mod tests {
    use super::*;

    use std::ffi::CStr;

    use crate::config::Config;
    use crate::key;
    use crate::test_utils::*;

    unsafe extern "C" fn logging_cb(
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

    /// Create Alice with a pre-generated keypair.
    fn create_alice_keypair(ctx: &Context) {
        ctx.set_config(Config::ConfiguredAddr, Some("alice@example.org"))
            .unwrap();

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
                "xcLYBF086ewBCACmJKuoxIO6T87yi4Q3MyNpMch3Y8KrtHDQyUszU36eqM3Pmd1l",
                "FrbcCd8ZWo2pq6OJSwsM/jjRGn1zo2YOaQeJRRrC+KrKGqSUtRSYQBPrPjE2YjSX",
                "AMbu8jBI6VVUhHeghriBkK79PY9O/oRhIJC0p14IJe6CQ6OI2fTmTUHF9i/nJ3G4",
                "Wb3/K1bU3yVfyPZjTZQPYPvvh03vxHULKurtYkX5DTEMSWsF4qzLMps+l87VuLV9",
                "iQnbN7YMRLHHx2KkX5Ivi7JCefhCa54M0K3bDCCxuVWAM5wjQwNZjzR+WL+dYchw",
                "oFvuF8NrlzjM9dSv+2rn+J7f99ijSXarzPC7ABEBAAEACAChqzVOuErmVRqvcYtq",
                "m1xt1H+ZjX20z5Sn1fhTLYAcq236AWMqJvwxCXoKlc8bt2UfB+Ls9cQb1YcVq353",
                "r0QiExiDeK3YlCxqd/peXJwFYTNKFC3QcnUhtpG9oS/jWjN+BRotGbjtu6Vj3M68",
                "JJAq+mHJ0/9OyrqrREvGfo7uLZt7iMGemDlrDakvrbIyZrPLgay+nZ3dEFKeOQ6F",
                "FrU05jyUVdoHBy0Tqx/6VpFUX9+IHcMHL2lTJB0nynBj+XZ/G4aX3WYoo3YlixHb",
                "Iu35fGFA0TChoGaGPzqcI/kg2Z+b/BryG9NM3LA2cO8iGrGXAE1nPFp91jmCrQ3V",
                "WushBADERP+uojjjfdO5J+RkmcFe9mFYDdtkhN+kV+LdePjiNNtcXMBhasstio0S",
                "ut0GKnE7DFRhX7mkN9w2apJ2ooeFeVVWot18eSdp6Rzh6/1Z7TmhYFJ3oUxxLbnQ",
                "sWIXIec1SzqWBFJUCn3IP0mCnJktFg/uGW6yLs01r5ds52uSBQQA2LSWiTwk9tEm",
                "dr9mz3tHnmrkyGiyKhKGM1Z7Rch63D5yQc1s4kUMBlyuLL2QtM/e4dtaz2JAkO8k",
                "QrYCnNgJ+2roTAK3kDZgYtymjdvK3HpQNtjVo7dds5RJVb6U618phZwU5WNFAEJW",
                "yyImmycGfjLv+18cW/3mq0QVZejkM78D/2kHaIeJAowtBOFY2zDrKyDRoBHaUSgj",
                "5BjGoviRC5rYihWDEyYDQ6mBJQstAD0Ty3MYzyUxl6ruB/BMWnMDFq5+TqtdBzu3",
                "jCtZ8OEyH8A5Kdo68Wzo/PGxzMtusOdNj9+3PBmSq4yibJxbLSrn59aVUYpGLjeG",
                "Kyvm9OTKkrOGN27NEzxhbGljZUBleGFtcGxlLmNvbT7CwIkEEAEIADMCGQEFAl08",
                "6fgCGwMECwkIBwYVCAkKCwIDFgIBFiEE+iaix4d0doj87Q0ek6DcNkbrcegACgkQ",
                "k6DcNkbrcei3ogf/cruUmQ+th52TFHTHdkw9OHUl3MrXtZ7QmHyOAFvbXE/6n5Ee",
                "h+eZoF8MWWV72m14Wbs+vTcNQkFVTdOPptkKA8e4cJqwDOHsyAnvQXZ7WNje9+BM",
                "zcoipIUawHP4ORFaPDsKLZQ0b4wBbKn8ziea6zjGF0/qljTdoxTtsYpv5wXYuhwb",
                "YklrLOqgSa5M7LXUe7E3g9mbg+9iX1GuB8m6GkquJN814Y+xny4xhZzGOfue6SeP",
                "12jJMNSjSP7416dRq7794VGnkkW9V/7oFEUKu5wO9FFbgDySOSlEjByGejSGuBmh",
                "o0iJSjcPjZ7EY/j3M3orq4dpza5C82OeSvxDFcfC2ARdPOnsAQgA5oLxXRLnyugz",
                "OmNCy7dxV3JrDZscA6JNlJlDWIShT0YSs+zG9JzDeQql+sYXgUSxOoIayItuXtnF",
                "n7tstwGoOnYvadm/e5/7V5fKAQRtCtdN51av62n18Venlm0yNKpROPcZ6M/sc4m6",
                "uU6YRZ/a1idal8VGY0wLKlghjIBuIiBoVQ/RnoW+/fhmwIg08dQ5m8hQe3GEOZEe",
                "LrTWL/9awPlXK7Y+DmJOoR4qbHWEcRfbzS6q4zW8vk2ztB8ngwbnqYy8zrN1DCIC",
                "N1gYdlU++uVw6Bb1XfY8Cdldh1VLKpF35mAmjxLZfVDcoObFH3Cv2GB7BEYxv86K",
                "C2Y6T74Q/wARAQABAAgAhSvFEYZoj1sSrXrHDjZOrryViGjCCH9t3pmkxLDrGIdd",
                "KsFyN8ORUo6KUZS745yx3yFnI9EZ1IZvm9aF+jxk2lGJFtgLvfoxFOvGckwCSy8T",
                "/MCiJZkz01hWo5s2VCLJheWL/GqTKjS5wXDcm+y8Wtilh+UawycdlDsSNr/D4MZL",
                "j3Chq9K03l5UIR8DcC7SavNi55R2oGOfboXsdvwOlrNZdCkZOlXDI4ZKFwbDHCtp",
                "Do5FS30hnJi2TecUPZWB1CaGFWnevINd4ikugVjcAoZj/QAIvfrOCgqisF/Ylg9u",
                "RMUPBapmcJUueILwd0iQqvGG0aCqtchvSmlg15/lQQQA9G1NNjNAH+NQrXvDJFJe",
                "/V1U3F3pz7jCjQa69c0dxSBUeNX1pG8XXD6tSkkd4Ni1mzZGcZXOmVUM6cA9I7RH",
                "95RqV+QIfnXVneCRrlCjV8m6OBlkivkESXc3nW5wtCIfw7oKg9w1xuVNUaAlbCt9",
                "QVLaxXJiY7ad0f5U9XJ1+w8EAPFs+M/+GZK1wOZYBL1vo7x0gL9ZggmjC4B+viBJ",
                "8Q60mqTrphYFsbXHuwKV0g9aIoZMucKyEE0QLR7imttiLEz1nD8bfEScbGy9ZG//",
                "wRfyJmCVAjA0pQ6LtB93d70PSVzzJrMHgbLKrDuSd6RChl7n9BIEdVyk7LEph0Yg",
                "9UsRBADm6DvpKL+P3lQ0eLTfAgcQTOqLZDYmI3PvqqSkHb1kHChqOXXs8hGOSSwK",
                "Gjcd4CZeNOGWR42rZyRhVgtkt6iYviIaVAWUfme6K+sLQBCeyMlmEGtykAA+LmPB",
                "f4zdyUNADfoxgZF3EKHf6I3nlVn5cdT+o/9vjdY2XAOwcls1RzaFwsB2BBgBCAAg",
                "BQJdPOn4AhsMFiEE+iaix4d0doj87Q0ek6DcNkbrcegACgkQk6DcNkbrcegLxwf/",
                "dXshJnoWqEnRsf6rVb9/Mc66ti+NVQLfNd275dybh/QIJdK3NmSxdnTPIEJRVspJ",
                "ywJoupwXFNrnHG2Ff6QPvHqI+/oNu986r9d7Z1oQibbLHKt8t6kpOfg/xGxotagJ",
                "uiCQvR9mMjD1DqsdB37SjDxGupZOOJSXWi6KX60IE+uM+QOBfeOZziQwuFmA5wV6",
                "RDXIeYJfqrcbeXeR1d0nfNpPHQR1gBiqmxNb6KBbdXD2+EXW60axC7D2b1APmzMl",
                "ammDliPwsK9U1rc9nuquEBvGDOJf4K+Dzn+mDCqRpP6uAuQ7RKHyim4uyN0wwKOb",
                "zPqgJCGwjTglkixw+aSTXw=="
            ),
            KeyType::Private,
        )
        .unwrap();
        let saved = key::dc_key_save_self_keypair(
            &ctx,
            &public,
            &private,
            "alice@example.org",
            1,
            &ctx.sql,
        );
        assert_eq!(saved, true, "Failed to save Alice's key");
    }

    #[test]
    fn test_render_setup_file() {
        let t = test_context(Some(logging_cb));

        create_alice_keypair(&t.ctx); // Trick things to think we're configured.
        let msg = unsafe {
            to_string(dc_render_setup_file(
                &t.ctx,
                b"hello\x00" as *const u8 as *const libc::c_char,
            ))
        };
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
}
