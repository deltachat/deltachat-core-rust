use c2rust_bitfields::BitfieldStruct;
use libc;

use crate::dc_chat::*;
use crate::dc_configure::*;
use crate::dc_context::dc_context_t;
use crate::dc_e2ee::*;
use crate::dc_job::*;
use crate::dc_key::*;
use crate::dc_log::*;
use crate::dc_loginparam::*;
use crate::dc_lot::dc_lot_t;
use crate::dc_msg::*;
use crate::dc_param::*;
use crate::dc_pgp::*;
use crate::dc_sqlite3::*;
use crate::dc_stock::*;
use crate::dc_strbuilder::*;
use crate::dc_tools::*;
use crate::types::*;
use crate::x::*;

// import/export and tools
// param1 is a directory where the keys are written to
// param1 is a directory where the keys are searched in and read from
// param1 is a directory where the backup is written to
// param1 is the file with the backup to import
#[no_mangle]
pub unsafe extern "C" fn dc_imex(
    mut context: *mut dc_context_t,
    mut what: libc::c_int,
    mut param1: *const libc::c_char,
    mut param2: *const libc::c_char,
) {
    let mut param: *mut dc_param_t = dc_param_new();
    dc_param_set_int(param, 'S' as i32, what);
    dc_param_set(param, 'E' as i32, param1);
    dc_param_set(param, 'F' as i32, param2);
    dc_job_kill_action(context, 910i32);
    dc_job_add(context, 910i32, 0i32, (*param).packed, 0i32);
    dc_param_unref(param);
}
#[no_mangle]
pub unsafe extern "C" fn dc_imex_has_backup(
    mut context: *mut dc_context_t,
    mut dir_name: *const libc::c_char,
) -> *mut libc::c_char {
    let mut ret: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut ret_backup_time: time_t = 0i32 as time_t;
    let mut dir_handle: *mut DIR = 0 as *mut DIR;
    let mut dir_entry: *mut dirent = 0 as *mut dirent;
    let mut prefix_len: libc::c_int =
        strlen(b"delta-chat\x00" as *const u8 as *const libc::c_char) as libc::c_int;
    let mut suffix_len: libc::c_int =
        strlen(b"bak\x00" as *const u8 as *const libc::c_char) as libc::c_int;
    let mut curr_pathNfilename: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut test_sql: *mut dc_sqlite3_t = 0 as *mut dc_sqlite3_t;
    if context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint {
        return 0 as *mut libc::c_char;
    }
    dir_handle = opendir(dir_name);
    if dir_handle.is_null() {
        dc_log_info(
            context,
            0i32,
            b"Backup check: Cannot open directory \"%s\".\x00" as *const u8 as *const libc::c_char,
            dir_name,
        );
    } else {
        loop {
            dir_entry = readdir(dir_handle);
            if dir_entry.is_null() {
                break;
            }
            let mut name: *const libc::c_char = (*dir_entry).d_name.as_mut_ptr();
            let mut name_len: libc::c_int = strlen(name) as libc::c_int;
            if name_len > prefix_len
                && strncmp(
                    name,
                    b"delta-chat\x00" as *const u8 as *const libc::c_char,
                    prefix_len as libc::c_ulong,
                ) == 0i32
                && name_len > suffix_len
                && strncmp(
                    &*name.offset((name_len - suffix_len - 1i32) as isize),
                    b".bak\x00" as *const u8 as *const libc::c_char,
                    suffix_len as libc::c_ulong,
                ) == 0i32
            {
                free(curr_pathNfilename as *mut libc::c_void);
                curr_pathNfilename = dc_mprintf(
                    b"%s/%s\x00" as *const u8 as *const libc::c_char,
                    dir_name,
                    name,
                );
                dc_sqlite3_unref(test_sql);
                test_sql = dc_sqlite3_new(context);
                if !test_sql.is_null() && 0 != dc_sqlite3_open(test_sql, curr_pathNfilename, 0x1i32)
                {
                    let mut curr_backup_time: time_t = dc_sqlite3_get_config_int(
                        test_sql,
                        b"backup_time\x00" as *const u8 as *const libc::c_char,
                        0i32,
                    ) as time_t;
                    if curr_backup_time > 0i32 as libc::c_long && curr_backup_time > ret_backup_time
                    {
                        free(ret as *mut libc::c_void);
                        ret = curr_pathNfilename;
                        ret_backup_time = curr_backup_time;
                        curr_pathNfilename = 0 as *mut libc::c_char
                    }
                }
            }
        }
    }
    if !dir_handle.is_null() {
        closedir(dir_handle);
    }
    free(curr_pathNfilename as *mut libc::c_void);
    dc_sqlite3_unref(test_sql);
    return ret;
}
#[no_mangle]
pub unsafe extern "C" fn dc_check_password(
    mut context: *mut dc_context_t,
    mut test_pw: *const libc::c_char,
) -> libc::c_int {
    /* Check if the given password matches the configured mail_pw.
    This is to prompt the user before starting eg. an export; this is mainly to avoid doing people bad thinkgs if they have short access to the device.
    When we start supporting OAuth some day, we should think this over, maybe force the user to re-authenticate himself with the Android password. */
    let mut loginparam: *mut dc_loginparam_t = dc_loginparam_new();
    let mut success: libc::c_int = 0i32;
    if !(context.is_null() || (*context).magic != 0x11a11807i32 as libc::c_uint) {
        dc_loginparam_read(
            loginparam,
            (*context).sql,
            b"configured_\x00" as *const u8 as *const libc::c_char,
        );
        if ((*loginparam).mail_pw.is_null()
            || *(*loginparam).mail_pw.offset(0isize) as libc::c_int == 0i32)
            && (test_pw.is_null() || *test_pw.offset(0isize) as libc::c_int == 0i32)
        {
            success = 1i32
        } else if (*loginparam).mail_pw.is_null() || test_pw.is_null() {
            success = 0i32
        } else if strcmp((*loginparam).mail_pw, test_pw) == 0i32 {
            success = 1i32
        }
    }
    dc_loginparam_unref(loginparam);
    return success;
}
#[no_mangle]
pub unsafe extern "C" fn dc_initiate_key_transfer(
    mut context: *mut dc_context_t,
) -> *mut libc::c_char {
    let mut current_block: u64;
    let mut success: libc::c_int = 0i32;
    let mut setup_code: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut setup_file_content: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut setup_file_name: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut chat_id: uint32_t = 0i32 as uint32_t;
    let mut msg: *mut dc_msg_t = 0 as *mut dc_msg_t;
    let mut msg_id: uint32_t = 0i32 as uint32_t;
    if 0 == dc_alloc_ongoing(context) {
        return 0 as *mut libc::c_char;
    }
    setup_code = dc_create_setup_code(context);
    if !setup_code.is_null() {
        /* this may require a keypair to be created. this may take a second ... */
        if !(0 != (*context).shall_stop_ongoing) {
            setup_file_content = dc_render_setup_file(context, setup_code);
            if !setup_file_content.is_null() {
                /* encrypting may also take a while ... */
                if !(0 != (*context).shall_stop_ongoing) {
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
                            if !(0 != (*context).shall_stop_ongoing) {
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
                                        if 0 != (*context).shall_stop_ongoing {
                                            current_block = 6116957410927263949;
                                            break;
                                        }
                                        sleep(1i32 as libc::c_uint);
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
    return setup_code;
}
#[no_mangle]
pub unsafe extern "C" fn dc_render_setup_file(
    mut context: *mut dc_context_t,
    mut passphrase: *const libc::c_char,
) -> *mut libc::c_char {
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut self_addr: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut curr_private_key: *mut dc_key_t = dc_key_new();
    let mut passphrase_begin: [libc::c_char; 8] = [0; 8];
    let mut encr_string: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut ret_setupfilecontent: *mut libc::c_char = 0 as *mut libc::c_char;
    if !(context.is_null()
        || (*context).magic != 0x11a11807i32 as libc::c_uint
        || passphrase.is_null()
        || strlen(passphrase) < 2i32 as libc::c_ulong
        || curr_private_key.is_null())
    {
        strncpy(
            passphrase_begin.as_mut_ptr(),
            passphrase,
            2i32 as libc::c_ulong,
        );
        passphrase_begin[2usize] = 0i32 as libc::c_char;
        /* create the payload */
        if !(0 == dc_ensure_secret_key_exists(context)) {
            self_addr = dc_sqlite3_get_config(
                (*context).sql,
                b"configured_addr\x00" as *const u8 as *const libc::c_char,
                0 as *const libc::c_char,
            );
            dc_key_load_self_private(curr_private_key, self_addr, (*context).sql);
            let mut e2ee_enabled: libc::c_int = dc_sqlite3_get_config_int(
                (*context).sql,
                b"e2ee_enabled\x00" as *const u8 as *const libc::c_char,
                1i32,
            );
            let mut payload_key_asc: *mut libc::c_char = dc_key_render_asc(
                curr_private_key,
                if 0 != e2ee_enabled {
                    b"Autocrypt-Prefer-Encrypt: mutual\r\n\x00" as *const u8 as *const libc::c_char
                } else {
                    0 as *const libc::c_char
                },
            );
            if !payload_key_asc.is_null() {
                if !(0
                    == dc_pgp_symm_encrypt(
                        context,
                        passphrase,
                        payload_key_asc as *const libc::c_void,
                        strlen(payload_key_asc),
                        &mut encr_string,
                    ))
                {
                    free(payload_key_asc as *mut libc::c_void);
                    let mut replacement: *mut libc::c_char =
                        dc_mprintf(b"-----BEGIN PGP MESSAGE-----\r\nPassphrase-Format: numeric9x4\r\nPassphrase-Begin: %s\x00"
                                       as *const u8 as *const libc::c_char,
                                   passphrase_begin.as_mut_ptr());
                    dc_str_replace(
                        &mut encr_string,
                        b"-----BEGIN PGP MESSAGE-----\x00" as *const u8 as *const libc::c_char,
                        replacement,
                    );
                    free(replacement as *mut libc::c_void);
                    let mut setup_message_title: *mut libc::c_char = dc_stock_str(context, 42i32);
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
                }
            }
        }
    }
    sqlite3_finalize(stmt);
    dc_key_unref(curr_private_key);
    free(encr_string as *mut libc::c_void);
    free(self_addr as *mut libc::c_void);
    return ret_setupfilecontent;
}
#[no_mangle]
pub unsafe extern "C" fn dc_create_setup_code(mut context: *mut dc_context_t) -> *mut libc::c_char {
    let mut random_val: uint16_t = 0i32 as uint16_t;
    let mut i: libc::c_int = 0i32;
    let mut ret: dc_strbuilder_t = dc_strbuilder_t {
        buf: 0 as *mut libc::c_char,
        allocated: 0,
        free: 0,
        eos: 0 as *mut libc::c_char,
    };
    dc_strbuilder_init(&mut ret, 0i32);
    i = 0i32;
    while i < 9i32 {
        loop {
            if 0 == RAND_bytes(
                &mut random_val as *mut uint16_t as *mut libc::c_uchar,
                ::std::mem::size_of::<uint16_t>() as libc::c_ulong as libc::c_int,
            ) {
                dc_log_warning(
                    context,
                    0i32,
                    b"Falling back to pseudo-number generation for the setup code.\x00" as *const u8
                        as *const libc::c_char,
                );
                RAND_pseudo_bytes(
                    &mut random_val as *mut uint16_t as *mut libc::c_uchar,
                    ::std::mem::size_of::<uint16_t>() as libc::c_ulong as libc::c_int,
                );
            }
            if !(random_val as libc::c_int > 60000i32) {
                break;
            }
        }
        random_val = (random_val as libc::c_int % 10000i32) as uint16_t;
        dc_strbuilder_catf(
            &mut ret as *mut dc_strbuilder_t,
            b"%s%04i\x00" as *const u8 as *const libc::c_char,
            if 0 != i {
                b"-\x00" as *const u8 as *const libc::c_char
            } else {
                b"\x00" as *const u8 as *const libc::c_char
            },
            random_val as libc::c_int,
        );
        i += 1
    }
    return ret.buf;
}
#[no_mangle]
pub unsafe extern "C" fn dc_continue_key_transfer(
    mut context: *mut dc_context_t,
    mut msg_id: uint32_t,
    mut setup_code: *const libc::c_char,
) -> libc::c_int {
    let mut success: libc::c_int = 0i32;
    let mut msg: *mut dc_msg_t = 0 as *mut dc_msg_t;
    let mut filename: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut filecontent: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut filebytes: size_t = 0i32 as size_t;
    let mut armored_key: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut norm_sc: *mut libc::c_char = 0 as *mut libc::c_char;
    if !(context.is_null()
        || (*context).magic != 0x11a11807i32 as libc::c_uint
        || msg_id <= 9i32 as libc::c_uint
        || setup_code.is_null())
    {
        msg = dc_get_msg(context, msg_id);
        if msg.is_null()
            || 0 == dc_msg_is_setupmessage(msg)
            || {
                filename = dc_msg_get_file(msg);
                filename.is_null()
            }
            || *filename.offset(0isize) as libc::c_int == 0i32
        {
            dc_log_error(
                context,
                0i32,
                b"Message is no Autocrypt Setup Message.\x00" as *const u8 as *const libc::c_char,
            );
        } else if 0
            == dc_read_file(
                context,
                filename,
                &mut filecontent as *mut *mut libc::c_char as *mut *mut libc::c_void,
                &mut filebytes,
            )
            || filecontent.is_null()
            || filebytes <= 0i32 as libc::c_ulong
        {
            dc_log_error(
                context,
                0i32,
                b"Cannot read Autocrypt Setup Message file.\x00" as *const u8
                    as *const libc::c_char,
            );
        } else {
            norm_sc = dc_normalize_setup_code(context, setup_code);
            if norm_sc.is_null() {
                dc_log_warning(
                    context,
                    0i32,
                    b"Cannot normalize Setup Code.\x00" as *const u8 as *const libc::c_char,
                );
            } else {
                armored_key = dc_decrypt_setup_file(context, norm_sc, filecontent);
                if armored_key.is_null() {
                    dc_log_warning(
                        context,
                        0i32,
                        b"Cannot decrypt Autocrypt Setup Message.\x00" as *const u8
                            as *const libc::c_char,
                    );
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
    return success;
}
unsafe extern "C" fn set_self_key(
    mut context: *mut dc_context_t,
    mut armored: *const libc::c_char,
    mut set_default: libc::c_int,
) -> libc::c_int {
    let mut success: libc::c_int = 0i32;
    let mut buf: *mut libc::c_char = 0 as *mut libc::c_char;
    // pointer inside buf, MUST NOT be free()'d
    let mut buf_headerline: *const libc::c_char = 0 as *const libc::c_char;
    //   - " -
    let mut buf_preferencrypt: *const libc::c_char = 0 as *const libc::c_char;
    //   - " -
    let mut buf_base64: *const libc::c_char = 0 as *const libc::c_char;
    let mut private_key: *mut dc_key_t = dc_key_new();
    let mut public_key: *mut dc_key_t = dc_key_new();
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut self_addr: *mut libc::c_char = 0 as *mut libc::c_char;
    buf = dc_strdup(armored);
    if 0 == dc_split_armored_data(
        buf,
        &mut buf_headerline,
        0 as *mut *const libc::c_char,
        &mut buf_preferencrypt,
        &mut buf_base64,
    ) || strcmp(
        buf_headerline,
        b"-----BEGIN PGP PRIVATE KEY BLOCK-----\x00" as *const u8 as *const libc::c_char,
    ) != 0i32
        || buf_base64.is_null()
    {
        dc_log_warning(
            context,
            0i32,
            b"File does not contain a private key.\x00" as *const u8 as *const libc::c_char,
        );
    } else if 0 == dc_key_set_from_base64(private_key, buf_base64, 1i32)
        || 0 == dc_pgp_is_valid_key(context, private_key)
        || 0 == dc_pgp_split_key(context, private_key, public_key)
    {
        dc_log_error(
            context,
            0i32,
            b"File does not contain a valid private key.\x00" as *const u8 as *const libc::c_char,
        );
    } else {
        stmt = dc_sqlite3_prepare(
            (*context).sql,
            b"DELETE FROM keypairs WHERE public_key=? OR private_key=?;\x00" as *const u8
                as *const libc::c_char,
        );
        sqlite3_bind_blob(stmt, 1i32, (*public_key).binary, (*public_key).bytes, None);
        sqlite3_bind_blob(
            stmt,
            2i32,
            (*private_key).binary,
            (*private_key).bytes,
            None,
        );
        sqlite3_step(stmt);
        sqlite3_finalize(stmt);
        stmt = 0 as *mut sqlite3_stmt;
        if 0 != set_default {
            dc_sqlite3_execute(
                (*context).sql,
                b"UPDATE keypairs SET is_default=0;\x00" as *const u8 as *const libc::c_char,
            );
        }
        self_addr = dc_sqlite3_get_config(
            (*context).sql,
            b"configured_addr\x00" as *const u8 as *const libc::c_char,
            0 as *const libc::c_char,
        );
        if 0 == dc_key_save_self_keypair(
            public_key,
            private_key,
            self_addr,
            set_default,
            (*context).sql,
        ) {
            dc_log_error(
                context,
                0i32,
                b"Cannot save keypair.\x00" as *const u8 as *const libc::c_char,
            );
        } else {
            if !buf_preferencrypt.is_null() {
                if strcmp(
                    buf_preferencrypt,
                    b"nopreference\x00" as *const u8 as *const libc::c_char,
                ) == 0i32
                {
                    dc_sqlite3_set_config_int(
                        (*context).sql,
                        b"e2ee_enabled\x00" as *const u8 as *const libc::c_char,
                        0i32,
                    );
                } else if strcmp(
                    buf_preferencrypt,
                    b"mutual\x00" as *const u8 as *const libc::c_char,
                ) == 0i32
                {
                    dc_sqlite3_set_config_int(
                        (*context).sql,
                        b"e2ee_enabled\x00" as *const u8 as *const libc::c_char,
                        1i32,
                    );
                }
            }
            success = 1i32
        }
    }
    sqlite3_finalize(stmt);
    free(buf as *mut libc::c_void);
    free(self_addr as *mut libc::c_void);
    dc_key_unref(private_key);
    dc_key_unref(public_key);
    return success;
}
#[no_mangle]
pub unsafe extern "C" fn dc_decrypt_setup_file(
    mut context: *mut dc_context_t,
    mut passphrase: *const libc::c_char,
    mut filecontent: *const libc::c_char,
) -> *mut libc::c_char {
    let mut fc_buf: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut fc_headerline: *const libc::c_char = 0 as *const libc::c_char;
    let mut fc_base64: *const libc::c_char = 0 as *const libc::c_char;
    let mut binary: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut binary_bytes: size_t = 0i32 as size_t;
    let mut indx: size_t = 0i32 as size_t;
    let mut plain: *mut libc::c_void = 0 as *mut libc::c_void;
    let mut plain_bytes: size_t = 0i32 as size_t;
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
            || binary_bytes == 0i32 as libc::c_ulong)
        {
            /* decrypt symmetrically */
            if !(0
                == dc_pgp_symm_decrypt(
                    context,
                    passphrase,
                    binary as *const libc::c_void,
                    binary_bytes,
                    &mut plain,
                    &mut plain_bytes,
                ))
            {
                payload = strndup(plain as *const libc::c_char, plain_bytes)
            }
        }
    }
    free(plain);
    free(fc_buf as *mut libc::c_void);
    if !binary.is_null() {
        mmap_string_unref(binary);
    }
    return payload;
}
#[no_mangle]
pub unsafe extern "C" fn dc_normalize_setup_code(
    mut context: *mut dc_context_t,
    mut in_0: *const libc::c_char,
) -> *mut libc::c_char {
    if in_0.is_null() {
        return 0 as *mut libc::c_char;
    }
    let mut out: dc_strbuilder_t = dc_strbuilder_t {
        buf: 0 as *mut libc::c_char,
        allocated: 0,
        free: 0,
        eos: 0 as *mut libc::c_char,
    };
    dc_strbuilder_init(&mut out, 0i32);
    let mut outlen: libc::c_int = 0i32;
    let mut p1: *const libc::c_char = in_0;
    while 0 != *p1 {
        if *p1 as libc::c_int >= '0' as i32 && *p1 as libc::c_int <= '9' as i32 {
            dc_strbuilder_catf(
                &mut out as *mut dc_strbuilder_t,
                b"%c\x00" as *const u8 as *const libc::c_char,
                *p1 as libc::c_int,
            );
            outlen = strlen(out.buf) as libc::c_int;
            if outlen == 4i32
                || outlen == 9i32
                || outlen == 14i32
                || outlen == 19i32
                || outlen == 24i32
                || outlen == 29i32
                || outlen == 34i32
                || outlen == 39i32
            {
                dc_strbuilder_cat(&mut out, b"-\x00" as *const u8 as *const libc::c_char);
            }
        }
        p1 = p1.offset(1isize)
    }
    return out.buf;
}
#[no_mangle]
pub unsafe extern "C" fn dc_job_do_DC_JOB_IMEX_IMAP(
    mut context: *mut dc_context_t,
    mut job: *mut dc_job_t,
) {
    let mut current_block: u64;
    let mut success: libc::c_int = 0i32;
    let mut ongoing_allocated_here: libc::c_int = 0i32;
    let mut what: libc::c_int = 0i32;
    let mut param1: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut param2: *mut libc::c_char = 0 as *mut libc::c_char;
    if !(context.is_null()
        || (*context).magic != 0x11a11807i32 as libc::c_uint
        || (*context).sql.is_null())
    {
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
                (*context).cb.expect("non-null function pointer")(
                    context,
                    2051i32,
                    10i32 as uintptr_t,
                    0i32 as uintptr_t,
                );
                if 0 == dc_sqlite3_is_open((*context).sql) {
                    dc_log_error(
                        context,
                        0i32,
                        b"Import/export: Database not opened.\x00" as *const u8
                            as *const libc::c_char,
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
    }
    free(param1 as *mut libc::c_void);
    free(param2 as *mut libc::c_void);
    if 0 != ongoing_allocated_here {
        dc_free_ongoing(context);
    }
    (*context).cb.expect("non-null function pointer")(
        context,
        2051i32,
        (if 0 != success { 1000i32 } else { 0i32 }) as uintptr_t,
        0i32 as uintptr_t,
    );
}
/* ******************************************************************************
 * Import backup
 ******************************************************************************/
unsafe extern "C" fn import_backup(
    mut context: *mut dc_context_t,
    mut backup_to_import: *const libc::c_char,
) -> libc::c_int {
    let mut current_block: u64;
    let mut success: libc::c_int = 0i32;
    let mut processed_files_cnt: libc::c_int = 0i32;
    let mut total_files_cnt: libc::c_int = 0i32;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut pathNfilename: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut repl_from: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut repl_to: *mut libc::c_char = 0 as *mut libc::c_char;
    dc_log_info(
        context,
        0i32,
        b"Import \"%s\" to \"%s\".\x00" as *const u8 as *const libc::c_char,
        backup_to_import,
        (*context).dbfile,
    );
    if 0 != dc_is_configured(context) {
        dc_log_error(
            context,
            0i32,
            b"Cannot import backups to accounts in use.\x00" as *const u8 as *const libc::c_char,
        );
    } else {
        if 0 != dc_sqlite3_is_open((*context).sql) {
            dc_sqlite3_close((*context).sql);
        }
        dc_delete_file(context, (*context).dbfile);
        if 0 != dc_file_exist(context, (*context).dbfile) {
            dc_log_error(
                context,
                0i32,
                b"Cannot import backups: Cannot delete the old file.\x00" as *const u8
                    as *const libc::c_char,
            );
        } else if !(0 == dc_copy_file(context, backup_to_import, (*context).dbfile)) {
            /* error already logged */
            /* re-open copied database file */
            if !(0 == dc_sqlite3_open((*context).sql, (*context).dbfile, 0i32)) {
                stmt = dc_sqlite3_prepare(
                    (*context).sql,
                    b"SELECT COUNT(*) FROM backup_blobs;\x00" as *const u8 as *const libc::c_char,
                );
                sqlite3_step(stmt);
                total_files_cnt = sqlite3_column_int(stmt, 0i32);
                sqlite3_finalize(stmt);
                stmt = 0 as *mut sqlite3_stmt;
                stmt = dc_sqlite3_prepare(
                    (*context).sql,
                    b"SELECT file_name, file_content FROM backup_blobs ORDER BY id;\x00"
                        as *const u8 as *const libc::c_char,
                );
                loop {
                    if !(sqlite3_step(stmt) == 100i32) {
                        current_block = 10891380440665537214;
                        break;
                    }
                    if 0 != (*context).shall_stop_ongoing {
                        current_block = 8648553629232744886;
                        break;
                    }
                    processed_files_cnt += 1;
                    let mut permille: libc::c_int = processed_files_cnt * 1000i32 / total_files_cnt;
                    if permille < 10i32 {
                        permille = 10i32
                    }
                    if permille > 990i32 {
                        permille = 990i32
                    }
                    (*context).cb.expect("non-null function pointer")(
                        context,
                        2051i32,
                        permille as uintptr_t,
                        0i32 as uintptr_t,
                    );
                    let mut file_name: *const libc::c_char =
                        sqlite3_column_text(stmt, 0i32) as *const libc::c_char;
                    let mut file_bytes: libc::c_int = sqlite3_column_bytes(stmt, 1i32);
                    let mut file_content: *const libc::c_void = sqlite3_column_blob(stmt, 1i32);
                    if !(file_bytes > 0i32 && !file_content.is_null()) {
                        continue;
                    }
                    free(pathNfilename as *mut libc::c_void);
                    pathNfilename = dc_mprintf(
                        b"%s/%s\x00" as *const u8 as *const libc::c_char,
                        (*context).blobdir,
                        file_name,
                    );
                    if !(0
                        == dc_write_file(
                            context,
                            pathNfilename,
                            file_content,
                            file_bytes as size_t,
                        ))
                    {
                        continue;
                    }
                    dc_log_error(
                        context,
                        0i32,
                        b"Storage full? Cannot write file %s with %i bytes.\x00" as *const u8
                            as *const libc::c_char,
                        pathNfilename,
                        file_bytes,
                    );
                    /* otherwise the user may believe the stuff is imported correctly, but there are files missing ... */
                    current_block = 8648553629232744886;
                    break;
                }
                match current_block {
                    8648553629232744886 => {}
                    _ => {
                        sqlite3_finalize(stmt);
                        stmt = 0 as *mut sqlite3_stmt;
                        dc_sqlite3_execute(
                            (*context).sql,
                            b"DROP TABLE backup_blobs;\x00" as *const u8 as *const libc::c_char,
                        );
                        dc_sqlite3_try_execute(
                            (*context).sql,
                            b"VACUUM;\x00" as *const u8 as *const libc::c_char,
                        );
                        success = 1i32
                    }
                }
            }
        }
    }
    free(pathNfilename as *mut libc::c_void);
    free(repl_from as *mut libc::c_void);
    free(repl_to as *mut libc::c_void);
    sqlite3_finalize(stmt);
    return success;
}
/* ******************************************************************************
 * Export backup
 ******************************************************************************/
/* the FILE_PROGRESS macro calls the callback with the permille of files processed.
The macro avoids weird values of 0% or 100% while still working. */
unsafe extern "C" fn export_backup(
    mut context: *mut dc_context_t,
    mut dir: *const libc::c_char,
) -> libc::c_int {
    let mut current_block: u64;
    let mut success: libc::c_int = 0i32;
    let mut closed: libc::c_int = 0i32;
    let mut dest_pathNfilename: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut dest_sql: *mut dc_sqlite3_t = 0 as *mut dc_sqlite3_t;
    let mut now: time_t = time(0 as *mut time_t);
    let mut dir_handle: *mut DIR = 0 as *mut DIR;
    let mut dir_entry: *mut dirent = 0 as *mut dirent;
    let mut prefix_len: libc::c_int =
        strlen(b"delta-chat\x00" as *const u8 as *const libc::c_char) as libc::c_int;
    let mut suffix_len: libc::c_int =
        strlen(b"bak\x00" as *const u8 as *const libc::c_char) as libc::c_int;
    let mut curr_pathNfilename: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut buf: *mut libc::c_void = 0 as *mut libc::c_void;
    let mut buf_bytes: size_t = 0i32 as size_t;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut total_files_cnt: libc::c_int = 0i32;
    let mut processed_files_cnt: libc::c_int = 0i32;
    let mut delete_dest_file: libc::c_int = 0i32;
    /* get a fine backup file name (the name includes the date so that multiple backup instances are possible)
    FIXME: we should write to a temporary file first and rename it on success. this would guarantee the backup is complete. however, currently it is not clear it the import exists in the long run (may be replaced by a restore-from-imap)*/
    let mut timeinfo: *mut tm = 0 as *mut tm;
    let mut buffer: [libc::c_char; 256] = [0; 256];
    timeinfo = localtime(&mut now);
    strftime(
        buffer.as_mut_ptr(),
        256i32 as size_t,
        b"delta-chat-%Y-%m-%d.bak\x00" as *const u8 as *const libc::c_char,
        timeinfo,
    );
    dest_pathNfilename = dc_get_fine_pathNfilename(context, dir, buffer.as_mut_ptr());
    if dest_pathNfilename.is_null() {
        dc_log_error(
            context,
            0i32,
            b"Cannot get backup file name.\x00" as *const u8 as *const libc::c_char,
        );
    } else {
        dc_housekeeping(context);
        dc_sqlite3_try_execute(
            (*context).sql,
            b"VACUUM;\x00" as *const u8 as *const libc::c_char,
        );
        dc_sqlite3_close((*context).sql);
        closed = 1i32;
        dc_log_info(
            context,
            0i32,
            b"Backup \"%s\" to \"%s\".\x00" as *const u8 as *const libc::c_char,
            (*context).dbfile,
            dest_pathNfilename,
        );
        if !(0 == dc_copy_file(context, (*context).dbfile, dest_pathNfilename)) {
            /* error already logged */
            dc_sqlite3_open((*context).sql, (*context).dbfile, 0i32);
            closed = 0i32;
            /* add all files as blobs to the database copy (this does not require the source to be locked, neigher the destination as it is used only here) */
            /*for logging only*/
            dest_sql = dc_sqlite3_new(context);
            if !(dest_sql.is_null() || 0 == dc_sqlite3_open(dest_sql, dest_pathNfilename, 0i32)) {
                /* error already logged */
                if 0 == dc_sqlite3_table_exists(
                    dest_sql,
                    b"backup_blobs\x00" as *const u8 as *const libc::c_char,
                ) {
                    if 0 ==
                           dc_sqlite3_execute(dest_sql,
                                              b"CREATE TABLE backup_blobs (id INTEGER PRIMARY KEY, file_name, file_content);\x00"
                                                  as *const u8 as
                                                  *const libc::c_char) {
                        /* error already logged */
                        current_block = 11487273724841241105;
                    } else { current_block = 14648156034262866959; }
                } else {
                    current_block = 14648156034262866959;
                }
                match current_block {
                    11487273724841241105 => {}
                    _ => {
                        total_files_cnt = 0i32;
                        dir_handle = opendir((*context).blobdir);
                        if dir_handle.is_null() {
                            dc_log_error(
                                context,
                                0i32,
                                b"Backup: Cannot get info for blob-directory \"%s\".\x00"
                                    as *const u8
                                    as *const libc::c_char,
                                (*context).blobdir,
                            );
                        } else {
                            loop {
                                dir_entry = readdir(dir_handle);
                                if dir_entry.is_null() {
                                    break;
                                }
                                total_files_cnt += 1
                            }
                            closedir(dir_handle);
                            dir_handle = 0 as *mut DIR;
                            if total_files_cnt > 0i32 {
                                /* scan directory, pass 2: copy files */
                                dir_handle = opendir((*context).blobdir);
                                if dir_handle.is_null() {
                                    dc_log_error(
                                        context,
                                        0i32,
                                        b"Backup: Cannot copy from blob-directory \"%s\".\x00"
                                            as *const u8
                                            as *const libc::c_char,
                                        (*context).blobdir,
                                    );
                                    current_block = 11487273724841241105;
                                } else {
                                    stmt =
                                        dc_sqlite3_prepare(dest_sql,
                                                           b"INSERT INTO backup_blobs (file_name, file_content) VALUES (?, ?);\x00"
                                                               as *const u8 as
                                                               *const libc::c_char);
                                    loop {
                                        dir_entry = readdir(dir_handle);
                                        if dir_entry.is_null() {
                                            current_block = 2631791190359682872;
                                            break;
                                        }
                                        if 0 != (*context).shall_stop_ongoing {
                                            delete_dest_file = 1i32;
                                            current_block = 11487273724841241105;
                                            break;
                                        } else {
                                            processed_files_cnt += 1;
                                            let mut permille: libc::c_int =
                                                processed_files_cnt * 1000i32 / total_files_cnt;
                                            if permille < 10i32 {
                                                permille = 10i32
                                            }
                                            if permille > 990i32 {
                                                permille = 990i32
                                            }
                                            (*context).cb.expect("non-null function pointer")(
                                                context,
                                                2051i32,
                                                permille as uintptr_t,
                                                0i32 as uintptr_t,
                                            );
                                            /* name without path; may also be `.` or `..` */
                                            let mut name: *mut libc::c_char =
                                                (*dir_entry).d_name.as_mut_ptr();
                                            let mut name_len: libc::c_int =
                                                strlen(name) as libc::c_int;
                                            if !(name_len == 1i32
                                                && *name.offset(0isize) as libc::c_int
                                                    == '.' as i32
                                                || name_len == 2i32
                                                    && *name.offset(0isize) as libc::c_int
                                                        == '.' as i32
                                                    && *name.offset(1isize) as libc::c_int
                                                        == '.' as i32
                                                || name_len > prefix_len
                                                    && strncmp(
                                                        name,
                                                        b"delta-chat\x00" as *const u8
                                                            as *const libc::c_char,
                                                        prefix_len as libc::c_ulong,
                                                    ) == 0i32
                                                    && name_len > suffix_len
                                                    && strncmp(
                                                        &mut *name.offset(
                                                            (name_len - suffix_len - 1i32) as isize,
                                                        ),
                                                        b".bak\x00" as *const u8
                                                            as *const libc::c_char,
                                                        suffix_len as libc::c_ulong,
                                                    ) == 0i32)
                                            {
                                                //dc_log_info(context, 0, "Backup: Skipping \"%s\".", name);
                                                free(curr_pathNfilename as *mut libc::c_void);
                                                curr_pathNfilename = dc_mprintf(
                                                    b"%s/%s\x00" as *const u8
                                                        as *const libc::c_char,
                                                    (*context).blobdir,
                                                    name,
                                                );
                                                free(buf);
                                                if 0 == dc_read_file(
                                                    context,
                                                    curr_pathNfilename,
                                                    &mut buf,
                                                    &mut buf_bytes,
                                                ) || buf.is_null()
                                                    || buf_bytes <= 0i32 as libc::c_ulong
                                                {
                                                    continue;
                                                }
                                                sqlite3_bind_text(stmt, 1i32, name, -1i32, None);
                                                sqlite3_bind_blob(
                                                    stmt,
                                                    2i32,
                                                    buf,
                                                    buf_bytes as libc::c_int,
                                                    None,
                                                );
                                                if sqlite3_step(stmt) != 101i32 {
                                                    dc_log_error(context,
                                                                 0i32,
                                                                 b"Disk full? Cannot add file \"%s\" to backup.\x00"
                                                                     as
                                                                     *const u8
                                                                     as
                                                                     *const libc::c_char,
                                                                 curr_pathNfilename);
                                                    /* this is not recoverable! writing to the sqlite database should work! */
                                                    current_block = 11487273724841241105;
                                                    break;
                                                } else {
                                                    sqlite3_reset(stmt);
                                                }
                                            }
                                        }
                                    }
                                }
                            } else {
                                dc_log_info(
                                    context,
                                    0i32,
                                    b"Backup: No files to copy.\x00" as *const u8
                                        as *const libc::c_char,
                                    (*context).blobdir,
                                );
                                current_block = 2631791190359682872;
                            }
                            match current_block {
                                11487273724841241105 => {}
                                _ => {
                                    dc_sqlite3_set_config_int(
                                        dest_sql,
                                        b"backup_time\x00" as *const u8 as *const libc::c_char,
                                        now as int32_t,
                                    );
                                    (*context).cb.expect("non-null function pointer")(
                                        context,
                                        2052i32,
                                        dest_pathNfilename as uintptr_t,
                                        0i32 as uintptr_t,
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
    if !dir_handle.is_null() {
        closedir(dir_handle);
    }
    if 0 != closed {
        dc_sqlite3_open((*context).sql, (*context).dbfile, 0i32);
    }
    sqlite3_finalize(stmt);
    dc_sqlite3_close(dest_sql);
    dc_sqlite3_unref(dest_sql);
    if 0 != delete_dest_file {
        dc_delete_file(context, dest_pathNfilename);
    }
    free(dest_pathNfilename as *mut libc::c_void);
    free(curr_pathNfilename as *mut libc::c_void);
    free(buf);
    return success;
}
/* ******************************************************************************
 * Classic key import
 ******************************************************************************/
unsafe extern "C" fn import_self_keys(
    mut context: *mut dc_context_t,
    mut dir_name: *const libc::c_char,
) -> libc::c_int {
    /* hint: even if we switch to import Autocrypt Setup Files, we should leave the possibility to import
    plain ASC keys, at least keys without a password, if we do not want to implement a password entry function.
    Importing ASC keys is useful to use keys in Delta Chat used by any other non-Autocrypt-PGP implementation.

    Maybe we should make the "default" key handlong also a little bit smarter
    (currently, the last imported key is the standard key unless it contains the string "legacy" in its name) */
    let mut imported_cnt: libc::c_int = 0i32;
    let mut dir_handle: *mut DIR = 0 as *mut DIR;
    let mut dir_entry: *mut dirent = 0 as *mut dirent;
    let mut suffix: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut path_plus_name: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut set_default: libc::c_int = 0i32;
    let mut buf: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut buf_bytes: size_t = 0i32 as size_t;
    // a pointer inside buf, MUST NOT be free()'d
    let mut private_key: *const libc::c_char = 0 as *const libc::c_char;
    let mut buf2: *mut libc::c_char = 0 as *mut libc::c_char;
    // a pointer inside buf2, MUST NOT be free()'d
    let mut buf2_headerline: *const libc::c_char = 0 as *const libc::c_char;
    if !(context.is_null()
        || (*context).magic != 0x11a11807i32 as libc::c_uint
        || dir_name.is_null())
    {
        dir_handle = opendir(dir_name);
        if dir_handle.is_null() {
            dc_log_error(
                context,
                0i32,
                b"Import: Cannot open directory \"%s\".\x00" as *const u8 as *const libc::c_char,
                dir_name,
            );
        } else {
            loop {
                dir_entry = readdir(dir_handle);
                if dir_entry.is_null() {
                    break;
                }
                free(suffix as *mut libc::c_void);
                suffix = dc_get_filesuffix_lc((*dir_entry).d_name.as_mut_ptr());
                if suffix.is_null()
                    || strcmp(suffix, b"asc\x00" as *const u8 as *const libc::c_char) != 0i32
                {
                    continue;
                }
                free(path_plus_name as *mut libc::c_void);
                path_plus_name = dc_mprintf(
                    b"%s/%s\x00" as *const u8 as *const libc::c_char,
                    dir_name,
                    (*dir_entry).d_name.as_mut_ptr(),
                );
                dc_log_info(
                    context,
                    0i32,
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
                ) || buf_bytes < 50i32 as libc::c_ulong
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
                ) == 0i32
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
                set_default = 1i32;
                if !strstr(
                    (*dir_entry).d_name.as_mut_ptr(),
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
    if !dir_handle.is_null() {
        closedir(dir_handle);
    }
    free(suffix as *mut libc::c_void);
    free(path_plus_name as *mut libc::c_void);
    free(buf as *mut libc::c_void);
    free(buf2 as *mut libc::c_void);
    return imported_cnt;
}
unsafe extern "C" fn export_self_keys(
    mut context: *mut dc_context_t,
    mut dir: *const libc::c_char,
) -> libc::c_int {
    let mut success: libc::c_int = 0i32;
    let mut export_errors: libc::c_int = 0i32;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    let mut id: libc::c_int = 0i32;
    let mut is_default: libc::c_int = 0i32;
    let mut public_key: *mut dc_key_t = dc_key_new();
    let mut private_key: *mut dc_key_t = dc_key_new();
    stmt = dc_sqlite3_prepare(
        (*context).sql,
        b"SELECT id, public_key, private_key, is_default FROM keypairs;\x00" as *const u8
            as *const libc::c_char,
    );
    if !stmt.is_null() {
        while sqlite3_step(stmt) == 100i32 {
            id = sqlite3_column_int(stmt, 0i32);
            dc_key_set_from_stmt(public_key, stmt, 1i32, 0i32);
            dc_key_set_from_stmt(private_key, stmt, 2i32, 1i32);
            is_default = sqlite3_column_int(stmt, 3i32);
            if 0 == export_key_to_asc_file(context, dir, id, public_key, is_default) {
                export_errors += 1
            }
            if 0 == export_key_to_asc_file(context, dir, id, private_key, is_default) {
                export_errors += 1
            }
        }
        if export_errors == 0i32 {
            success = 1i32
        }
    }
    sqlite3_finalize(stmt);
    dc_key_unref(public_key);
    dc_key_unref(private_key);
    return success;
}
/* ******************************************************************************
 * Classic key export
 ******************************************************************************/
unsafe extern "C" fn export_key_to_asc_file(
    mut context: *mut dc_context_t,
    mut dir: *const libc::c_char,
    mut id: libc::c_int,
    mut key: *const dc_key_t,
    mut is_default: libc::c_int,
) -> libc::c_int {
    let mut success: libc::c_int = 0i32;
    let mut file_name: *mut libc::c_char = 0 as *mut libc::c_char;
    if 0 != is_default {
        file_name = dc_mprintf(
            b"%s/%s-key-default.asc\x00" as *const u8 as *const libc::c_char,
            dir,
            if (*key).type_0 == 0i32 {
                b"public\x00" as *const u8 as *const libc::c_char
            } else {
                b"private\x00" as *const u8 as *const libc::c_char
            },
        )
    } else {
        file_name = dc_mprintf(
            b"%s/%s-key-%i.asc\x00" as *const u8 as *const libc::c_char,
            dir,
            if (*key).type_0 == 0i32 {
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
    if 0 == dc_key_render_asc_to_file(key, file_name, context) {
        dc_log_error(
            context,
            0i32,
            b"Cannot write key to %s\x00" as *const u8 as *const libc::c_char,
            file_name,
        );
    } else {
        (*context).cb.expect("non-null function pointer")(
            context,
            2052i32,
            file_name as uintptr_t,
            0i32 as uintptr_t,
        );
        success = 1i32
    }
    free(file_name as *mut libc::c_void);
    return success;
}
