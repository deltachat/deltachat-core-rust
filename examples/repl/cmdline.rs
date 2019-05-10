use std::ffi::{CStr, CString};
use std::sync::{Arc, RwLock};

use deltachat::constants::*;
use deltachat::dc_aheader::*;
use deltachat::dc_apeerstate::*;
use deltachat::dc_array::*;
use deltachat::dc_chat::*;
use deltachat::dc_chatlist::*;
use deltachat::dc_configure::*;
use deltachat::dc_contact::*;
use deltachat::dc_context::*;
use deltachat::dc_dehtml::*;
use deltachat::dc_e2ee::*;
use deltachat::dc_hash::*;
use deltachat::dc_imap::*;
use deltachat::dc_imex::*;
use deltachat::dc_job::*;
use deltachat::dc_jobthread::*;
use deltachat::dc_jsmn::*;
use deltachat::dc_key::*;
use deltachat::dc_keyhistory::*;
use deltachat::dc_keyring::*;
use deltachat::dc_location::*;
use deltachat::dc_log::*;
use deltachat::dc_loginparam::*;
use deltachat::dc_lot::*;
use deltachat::dc_mimefactory::*;
use deltachat::dc_mimeparser::*;
use deltachat::dc_move::*;
use deltachat::dc_msg::*;
use deltachat::dc_oauth2::*;
use deltachat::dc_param::*;
use deltachat::dc_pgp::*;
use deltachat::dc_qr::*;
use deltachat::dc_receive_imf::*;
use deltachat::dc_saxparser::*;
use deltachat::dc_securejoin::*;
use deltachat::dc_simplify::*;
use deltachat::dc_smtp::*;
use deltachat::dc_sqlite3::*;
use deltachat::dc_stock::*;
use deltachat::dc_strbuilder::*;
use deltachat::dc_strencode::*;
use deltachat::dc_token::*;
use deltachat::dc_tools::*;
use deltachat::types::*;
use deltachat::x::*;
use num_traits::FromPrimitive;

/*
 * Reset database tables. This function is called from Core cmdline.
 *
 * Argument is a bitmask, executing single or multiple actions in one call.
 *
 * e.g. bitmask 7 triggers actions definded with bits 1, 2 and 4.
 */
pub unsafe fn dc_reset_tables(context: &dc_context_t, bits: libc::c_int) -> libc::c_int {
    dc_log_info(
        context,
        0i32,
        b"Resetting tables (%i)...\x00" as *const u8 as *const libc::c_char,
        bits,
    );
    if 0 != bits & 1i32 {
        dc_sqlite3_execute(
            context,
            &context.sql.clone().read().unwrap(),
            b"DELETE FROM jobs;\x00" as *const u8 as *const libc::c_char,
        );
        dc_log_info(
            context,
            0i32,
            b"(1) Jobs reset.\x00" as *const u8 as *const libc::c_char,
        );
    }
    if 0 != bits & 2i32 {
        dc_sqlite3_execute(
            context,
            &context.sql.clone().read().unwrap(),
            b"DELETE FROM acpeerstates;\x00" as *const u8 as *const libc::c_char,
        );
        dc_log_info(
            context,
            0i32,
            b"(2) Peerstates reset.\x00" as *const u8 as *const libc::c_char,
        );
    }
    if 0 != bits & 4i32 {
        dc_sqlite3_execute(
            context,
            &context.sql.clone().read().unwrap(),
            b"DELETE FROM keypairs;\x00" as *const u8 as *const libc::c_char,
        );
        dc_log_info(
            context,
            0i32,
            b"(4) Private keypairs reset.\x00" as *const u8 as *const libc::c_char,
        );
    }
    if 0 != bits & 8i32 {
        dc_sqlite3_execute(
            context,
            &context.sql.clone().read().unwrap(),
            b"DELETE FROM contacts WHERE id>9;\x00" as *const u8 as *const libc::c_char,
        );
        dc_sqlite3_execute(
            context,
            &context.sql.clone().read().unwrap(),
            b"DELETE FROM chats WHERE id>9;\x00" as *const u8 as *const libc::c_char,
        );
        dc_sqlite3_execute(
            context,
            &context.sql.clone().read().unwrap(),
            b"DELETE FROM chats_contacts;\x00" as *const u8 as *const libc::c_char,
        );
        dc_sqlite3_execute(
            context,
            &context.sql.clone().read().unwrap(),
            b"DELETE FROM msgs WHERE id>9;\x00" as *const u8 as *const libc::c_char,
        );
        dc_sqlite3_execute(
            context,
            &context.sql.clone().read().unwrap(),
            b"DELETE FROM config WHERE keyname LIKE \'imap.%\' OR keyname LIKE \'configured%\';\x00"
                as *const u8 as *const libc::c_char,
        );
        dc_sqlite3_execute(
            context,
            &context.sql.clone().read().unwrap(),
            b"DELETE FROM leftgrps;\x00" as *const u8 as *const libc::c_char,
        );
        dc_log_info(
            context,
            0i32,
            b"(8) Rest but server config reset.\x00" as *const u8 as *const libc::c_char,
        );
    }
    (context.cb)(
        context,
        Event::MSGS_CHANGED,
        0i32 as uintptr_t,
        0i32 as uintptr_t,
    );
    return 1i32;
}
unsafe fn dc_poke_eml_file(context: &dc_context_t, filename: *const libc::c_char) -> libc::c_int {
    /* mainly for testing, may be called by dc_import_spec() */
    let mut success: libc::c_int = 0i32;
    let mut data: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut data_bytes: size_t = 0;
    if !(dc_read_file(
        context,
        filename,
        &mut data as *mut *mut libc::c_char as *mut *mut libc::c_void,
        &mut data_bytes,
    ) == 0i32)
    {
        dc_receive_imf(
            context,
            data,
            data_bytes,
            b"import\x00" as *const u8 as *const libc::c_char,
            0i32 as uint32_t,
            0i32 as uint32_t,
        );
        success = 1i32
    }
    free(data as *mut libc::c_void);
    return success;
}
/* *
 * Import a file to the database.
 * For testing, import a folder with eml-files, a single eml-file, e-mail plus public key and so on.
 * For normal importing, use dc_imex().
 *
 * @private @memberof dc_context_t
 * @param context The context as created by dc_context_new().
 * @param spec The file or directory to import. NULL for the last command.
 * @return 1=success, 0=error.
 */
unsafe fn poke_spec(context: &dc_context_t, spec: *const libc::c_char) -> libc::c_int {
    let mut current_block: u64;
    let mut success: libc::c_int = 0i32;
    let mut real_spec: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut suffix: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut dir: *mut DIR = 0 as *mut DIR;
    let mut dir_entry: *mut dirent;
    let mut read_cnt: libc::c_int = 0i32;
    let mut name: *mut libc::c_char;
    if 0 == dc_sqlite3_is_open(&context.sql.clone().read().unwrap()) {
        dc_log_error(
            context,
            0i32,
            b"Import: Database not opened.\x00" as *const u8 as *const libc::c_char,
        );
    } else {
        /* if `spec` is given, remember it for later usage; if it is not given, try to use the last one */
        if !spec.is_null() {
            real_spec = dc_strdup(spec);
            dc_sqlite3_set_config(
                context,
                &context.sql.clone().read().unwrap(),
                b"import_spec\x00" as *const u8 as *const libc::c_char,
                real_spec,
            );
            current_block = 7149356873433890176;
        } else {
            real_spec = dc_sqlite3_get_config(
                context,
                &context.sql.clone().read().unwrap(),
                b"import_spec\x00" as *const u8 as *const libc::c_char,
                0 as *const libc::c_char,
            );
            if real_spec.is_null() {
                dc_log_error(
                    context,
                    0i32,
                    b"Import: No file or folder given.\x00" as *const u8 as *const libc::c_char,
                );
                current_block = 8522321847195001863;
            } else {
                current_block = 7149356873433890176;
            }
        }
        match current_block {
            8522321847195001863 => {}
            _ => {
                suffix = dc_get_filesuffix_lc(real_spec);
                if !suffix.is_null()
                    && strcmp(suffix, b"eml\x00" as *const u8 as *const libc::c_char) == 0i32
                {
                    if 0 != dc_poke_eml_file(context, real_spec) {
                        read_cnt += 1
                    }
                    current_block = 1622411330066726685;
                } else {
                    /* import a directory */
                    dir = opendir(real_spec);
                    if dir.is_null() {
                        dc_log_error(
                            context,
                            0i32,
                            b"Import: Cannot open directory \"%s\".\x00" as *const u8
                                as *const libc::c_char,
                            real_spec,
                        );
                        current_block = 8522321847195001863;
                    } else {
                        loop {
                            dir_entry = readdir(dir);
                            if dir_entry.is_null() {
                                break;
                            }
                            name = (*dir_entry).d_name.as_mut_ptr();
                            if strlen(name) >= 4
                                && strcmp(
                                    &mut *name.offset(strlen(name).wrapping_sub(4) as isize),
                                    b".eml\x00" as *const u8 as *const libc::c_char,
                                ) == 0i32
                            {
                                let path_plus_name: *mut libc::c_char = dc_mprintf(
                                    b"%s/%s\x00" as *const u8 as *const libc::c_char,
                                    real_spec,
                                    name,
                                );
                                dc_log_info(
                                    context,
                                    0i32,
                                    b"Import: %s\x00" as *const u8 as *const libc::c_char,
                                    path_plus_name,
                                );
                                if 0 != dc_poke_eml_file(context, path_plus_name) {
                                    read_cnt += 1
                                }
                                free(path_plus_name as *mut libc::c_void);
                            }
                        }
                        current_block = 1622411330066726685;
                    }
                }
                match current_block {
                    8522321847195001863 => {}
                    _ => {
                        dc_log_info(
                            context,
                            0i32,
                            b"Import: %i items read from \"%s\".\x00" as *const u8
                                as *const libc::c_char,
                            read_cnt,
                            real_spec,
                        );
                        if read_cnt > 0i32 {
                            (context.cb)(
                                context,
                                Event::MSGS_CHANGED,
                                0i32 as uintptr_t,
                                0i32 as uintptr_t,
                            );
                        }
                        success = 1i32
                    }
                }
            }
        }
    }
    if !dir.is_null() {
        closedir(dir);
    }
    free(real_spec as *mut libc::c_void);
    free(suffix as *mut libc::c_void);
    return success;
}
unsafe fn log_msg(context: &dc_context_t, prefix: *const libc::c_char, msg: *mut dc_msg_t) {
    let contact: *mut dc_contact_t = dc_get_contact(context, dc_msg_get_from_id(msg));
    let contact_name: *mut libc::c_char = dc_contact_get_name(contact);
    let contact_id: libc::c_int = dc_contact_get_id(contact) as libc::c_int;
    let mut statestr: *const libc::c_char = b"\x00" as *const u8 as *const libc::c_char;
    match dc_msg_get_state(msg) {
        20 => statestr = b" o\x00" as *const u8 as *const libc::c_char,
        26 => statestr = b" \xe2\x88\x9a\x00" as *const u8 as *const libc::c_char,
        28 => statestr = b" \xe2\x88\x9a\xe2\x88\x9a\x00" as *const u8 as *const libc::c_char,
        24 => statestr = b" !!\x00" as *const u8 as *const libc::c_char,
        _ => {}
    }
    let temp2: *mut libc::c_char = dc_timestamp_to_str(dc_msg_get_timestamp(msg));
    let msgtext: *mut libc::c_char = dc_msg_get_text(msg);
    dc_log_info(
        context,
        0i32,
        b"%s#%i%s%s: %s (Contact#%i): %s %s%s%s%s [%s]\x00" as *const u8 as *const libc::c_char,
        prefix,
        dc_msg_get_id(msg) as libc::c_int,
        if 0 != dc_msg_get_showpadlock(msg) {
            b"\xf0\x9f\x94\x92\x00" as *const u8 as *const libc::c_char
        } else {
            b"\x00" as *const u8 as *const libc::c_char
        },
        if dc_msg_has_location(msg) {
            b"\xf0\x9f\x93\x8d\x00" as *const u8 as *const libc::c_char
        } else {
            b"\x00" as *const u8 as *const libc::c_char
        },
        contact_name,
        contact_id,
        msgtext,
        if 0 != dc_msg_is_starred(msg) {
            b" \xe2\x98\x85\x00" as *const u8 as *const libc::c_char
        } else {
            b"\x00" as *const u8 as *const libc::c_char
        },
        if dc_msg_get_from_id(msg) == 1i32 as libc::c_uint {
            b"\x00" as *const u8 as *const libc::c_char
        } else if dc_msg_get_state(msg) == 16i32 {
            b"[SEEN]\x00" as *const u8 as *const libc::c_char
        } else if dc_msg_get_state(msg) == 13i32 {
            b"[NOTICED]\x00" as *const u8 as *const libc::c_char
        } else {
            b"[FRESH]\x00" as *const u8 as *const libc::c_char
        },
        if 0 != dc_msg_is_info(msg) {
            b"[INFO]\x00" as *const u8 as *const libc::c_char
        } else {
            b"\x00" as *const u8 as *const libc::c_char
        },
        statestr,
        temp2,
    );
    free(msgtext as *mut libc::c_void);
    free(temp2 as *mut libc::c_void);
    free(contact_name as *mut libc::c_void);
    dc_contact_unref(contact);
}

unsafe fn log_msglist(context: &dc_context_t, msglist: *mut dc_array_t) {
    let mut i: libc::c_int = 0;
    let cnt: libc::c_int = dc_array_get_cnt(msglist) as libc::c_int;
    let mut lines_out: libc::c_int = 0i32;
    while i < cnt {
        let msg_id: uint32_t = dc_array_get_id(msglist, i as size_t);
        if msg_id == 9i32 as libc::c_uint {
            dc_log_info(context, 0i32,
                        b"--------------------------------------------------------------------------------\x00"
                            as *const u8 as *const libc::c_char);
            lines_out += 1
        } else if msg_id > 0i32 as libc::c_uint {
            if lines_out == 0i32 {
                dc_log_info(context, 0i32,
                            b"--------------------------------------------------------------------------------\x00"
                                as *const u8 as *const libc::c_char);
                lines_out += 1
            }
            let msg: *mut dc_msg_t = dc_get_msg(context, msg_id);
            log_msg(context, b"Msg\x00" as *const u8 as *const libc::c_char, msg);
            dc_msg_unref(msg);
        }
        i += 1
    }
    if lines_out > 0i32 {
        dc_log_info(
            context,
            0i32,
            b"--------------------------------------------------------------------------------\x00"
                as *const u8 as *const libc::c_char,
        );
    };
}
unsafe fn log_contactlist(context: &dc_context_t, contacts: *mut dc_array_t) {
    let mut contact: *mut dc_contact_t;
    let peerstate: *mut dc_apeerstate_t = dc_apeerstate_new(context);
    if 0 == dc_array_search_id(contacts, 1i32 as uint32_t, 0 as *mut size_t) {
        dc_array_add_id(contacts, 1i32 as uint32_t);
    }
    let mut i = 0;
    while i < dc_array_get_cnt(contacts) {
        let contact_id: uint32_t = dc_array_get_id(contacts, i as size_t);
        let line: *mut libc::c_char;
        let mut line2: *mut libc::c_char = 0 as *mut libc::c_char;
        contact = dc_get_contact(context, contact_id);
        if !contact.is_null() {
            let name: *mut libc::c_char = dc_contact_get_name(contact);
            let addr: *mut libc::c_char = dc_contact_get_addr(contact);
            let verified_state: libc::c_int = dc_contact_is_verified(contact);
            let verified_str: *const libc::c_char = if 0 != verified_state {
                if verified_state == 2i32 {
                    b" \xe2\x88\x9a\xe2\x88\x9a\x00" as *const u8 as *const libc::c_char
                } else {
                    b" \xe2\x88\x9a\x00" as *const u8 as *const libc::c_char
                }
            } else {
                b"\x00" as *const u8 as *const libc::c_char
            };
            line = dc_mprintf(
                b"%s%s <%s>\x00" as *const u8 as *const libc::c_char,
                if !name.is_null() && 0 != *name.offset(0isize) as libc::c_int {
                    name
                } else {
                    b"<name unset>\x00" as *const u8 as *const libc::c_char
                },
                verified_str,
                if !addr.is_null() && 0 != *addr.offset(0isize) as libc::c_int {
                    addr
                } else {
                    b"addr unset\x00" as *const u8 as *const libc::c_char
                },
            );
            let peerstate_ok: libc::c_int =
                dc_apeerstate_load_by_addr(peerstate, &context.sql.clone().read().unwrap(), addr);
            if 0 != peerstate_ok && contact_id != 1i32 as libc::c_uint {
                let pe: *mut libc::c_char;
                match (*peerstate).prefer_encrypt {
                    1 => pe = dc_strdup(b"mutual\x00" as *const u8 as *const libc::c_char),
                    0 => pe = dc_strdup(b"no-preference\x00" as *const u8 as *const libc::c_char),
                    20 => pe = dc_strdup(b"reset\x00" as *const u8 as *const libc::c_char),
                    _ => {
                        pe = dc_mprintf(
                            b"unknown-value (%i)\x00" as *const u8 as *const libc::c_char,
                            (*peerstate).prefer_encrypt,
                        )
                    }
                }
                line2 = dc_mprintf(
                    b", prefer-encrypt=%s\x00" as *const u8 as *const libc::c_char,
                    pe,
                );
                free(pe as *mut libc::c_void);
            }
            dc_contact_unref(contact);
            free(name as *mut libc::c_void);
            free(addr as *mut libc::c_void);
            dc_log_info(
                context,
                0i32,
                b"Contact#%i: %s%s\x00" as *const u8 as *const libc::c_char,
                contact_id as libc::c_int,
                line,
                if !line2.is_null() {
                    line2
                } else {
                    b"\x00" as *const u8 as *const libc::c_char
                },
            );
            free(line as *mut libc::c_void);
            free(line2 as *mut libc::c_void);
        }
        i += 1
    }
    dc_apeerstate_unref(peerstate);
}
static mut s_is_auth: libc::c_int = 0i32;

pub unsafe fn dc_cmdline_skip_auth() {
    s_is_auth = 1i32;
}
unsafe fn chat_prefix(chat: *const dc_chat_t) -> *const libc::c_char {
    if (*chat).type_0 == 120i32 {
        return b"Group\x00" as *const u8 as *const libc::c_char;
    } else if (*chat).type_0 == 130i32 {
        return b"VerifiedGroup\x00" as *const u8 as *const libc::c_char;
    } else {
        return b"Single\x00" as *const u8 as *const libc::c_char;
    };
}

pub unsafe fn dc_cmdline(context: &dc_context_t, cmdline: &str) -> *mut libc::c_char {
    let mut ret: *mut libc::c_char = 1i32 as *mut libc::c_char;
    let chat_id = *context.cmdline_sel_chat_id.read().unwrap();

    let mut sel_chat = if chat_id > 0 {
        dc_get_chat(context, chat_id)
    } else {
        std::ptr::null_mut()
    };

    let mut args = cmdline.split_whitespace();
    let arg0 = args.next().unwrap_or_default();
    let cmd = dc_strdup(CString::new(arg0).unwrap().as_ptr());

    let arg1 = args.next().unwrap_or_default();
    let arg1_c = CString::new(arg1).unwrap();
    let arg2 = args.next().unwrap_or_default();
    let arg2_c = CString::new(arg2).unwrap();

    if cmdline == "help" || cmdline == "?" {
        if arg1 == "imex" {
            ret =
                    dc_strdup(b"====================Import/Export commands==\ninitiate-key-transfer\nget-setupcodebegin <msg-id>\ncontinue-key-transfer <msg-id> <setup-code>\nhas-backup\nexport-backup\nimport-backup <backup-file>\nexport-keys\nimport-keys\nexport-setup\npoke [<eml-file>|<folder>|<addr> <key-file>]\nreset <flags>\nstop\n=============================================\x00"
                                  as *const u8 as *const libc::c_char)
        } else {
            ret =
                    dc_strdup(b"==========================Database commands==\ninfo\nopen <file to open or create>\nclose\nset <configuration-key> [<value>]\nget <configuration-key>\noauth2\nconfigure\nconnect\ndisconnect\nmaybenetwork\nhousekeeping\nhelp imex (Import/Export)\n==============================Chat commands==\nlistchats [<query>]\nlistarchived\nchat [<chat-id>|0]\ncreatechat <contact-id>\ncreatechatbymsg <msg-id>\ncreategroup <name>\ncreateverified <name>\naddmember <contact-id>\nremovemember <contact-id>\ngroupname <name>\ngroupimage [<file>]\nchatinfo\nsendlocations <seconds>\nsetlocation <lat> <lng>\ndellocations\ngetlocations [<contact-id>]\nsend <text>\nsendimage <file> [<text>]\nsendfile <file>\ndraft [<text>]\nlistmedia\narchive <chat-id>\nunarchive <chat-id>\ndelchat <chat-id>\n===========================Message commands==\nlistmsgs <query>\nmsginfo <msg-id>\nlistfresh\nforward <msg-id> <chat-id>\nmarkseen <msg-id>\nstar <msg-id>\nunstar <msg-id>\ndelmsg <msg-id>\n===========================Contact commands==\nlistcontacts [<query>]\nlistverified [<query>]\naddcontact [<name>] <addr>\ncontactinfo <contact-id>\ndelcontact <contact-id>\ncleanupcontacts\n======================================Misc.==\ngetqr [<chat-id>]\ngetbadqr\ncheckqr <qr-content>\nevent <event-id to test>\nfileinfo <file>\nclear -- clear screen\nexit\n=============================================\x00"
                                  as *const u8 as *const libc::c_char)
        }
    } else if 0 == s_is_auth {
        if strcmp(cmd, b"auth\x00" as *const u8 as *const libc::c_char) == 0i32 {
            let is_pw = dc_get_config(context, b"mail_pw\x00" as *const u8 as *const libc::c_char);
            if arg1 == CStr::from_ptr(is_pw).to_str().unwrap() {
                s_is_auth = 1;
                ret = 2i32 as *mut libc::c_char
            } else {
                ret = b"Bad password.\x00" as *const u8 as *const libc::c_char as *mut libc::c_char
            }
        } else {
            ret = dc_strdup(
                b"Please authorize yourself using: auth <password>\x00" as *const u8
                    as *const libc::c_char,
            )
        }
    } else if strcmp(cmd, b"auth\x00" as *const u8 as *const libc::c_char) == 0i32 {
        ret = dc_strdup(b"Already authorized.\x00" as *const u8 as *const libc::c_char)
    } else if strcmp(cmd, b"open\x00" as *const u8 as *const libc::c_char) == 0i32 {
        if !arg1.is_empty() {
            dc_close(context);
            ret = if 0 != dc_open(context, arg1_c.as_ptr(), 0 as *const libc::c_char) {
                2i32 as *mut libc::c_char
            } else {
                1i32 as *mut libc::c_char
            }
        } else {
            ret = dc_strdup(
                b"ERROR: Argument <file> missing.\x00" as *const u8 as *const libc::c_char,
            )
        }
    } else if strcmp(cmd, b"close\x00" as *const u8 as *const libc::c_char) == 0i32 {
        dc_close(context);
        ret = 2i32 as *mut libc::c_char
    } else if strcmp(
        cmd,
        b"initiate-key-transfer\x00" as *const u8 as *const libc::c_char,
    ) == 0i32
    {
        let setup_code: *mut libc::c_char = dc_initiate_key_transfer(context);
        ret = if !setup_code.is_null() {
            dc_mprintf(
                b"Setup code for the transferred setup message: %s\x00" as *const u8
                    as *const libc::c_char,
                setup_code,
            )
        } else {
            1i32 as *mut libc::c_char
        };
        free(setup_code as *mut libc::c_void);
    } else if strcmp(
        cmd,
        b"get-setupcodebegin\x00" as *const u8 as *const libc::c_char,
    ) == 0i32
    {
        if !arg1.is_empty() {
            let msg_id: u32 = arg1.parse().unwrap();
            let msg: *mut dc_msg_t = dc_get_msg(context, msg_id);
            if 0 != dc_msg_is_setupmessage(msg) {
                let setupcodebegin: *mut libc::c_char = dc_msg_get_setupcodebegin(msg);
                ret = dc_mprintf(
                    b"The setup code for setup message Msg#%i starts with: %s\x00" as *const u8
                        as *const libc::c_char,
                    msg_id,
                    setupcodebegin,
                );
                free(setupcodebegin as *mut libc::c_void);
            } else {
                ret = dc_mprintf(
                    b"ERROR: Msg#%i is no setup message.\x00" as *const u8 as *const libc::c_char,
                    msg_id,
                )
            }
            dc_msg_unref(msg);
        } else {
            ret = dc_strdup(
                b"ERROR: Argument <msg-id> missing.\x00" as *const u8 as *const libc::c_char,
            )
        }
    } else if strcmp(
        cmd,
        b"continue-key-transfer\x00" as *const u8 as *const libc::c_char,
    ) == 0i32
    {
        if !arg1.is_empty() && !arg2.is_empty() {
            ret = if 0 != dc_continue_key_transfer(context, arg1.parse().unwrap(), arg2_c.as_ptr())
            {
                2i32 as *mut libc::c_char
            } else {
                1i32 as *mut libc::c_char
            }
        } else {
            ret = dc_strdup(
                b"ERROR: Arguments <msg-id> <setup-code> expected.\x00" as *const u8
                    as *const libc::c_char,
            )
        }
    } else if strcmp(cmd, b"has-backup\x00" as *const u8 as *const libc::c_char) == 0i32 {
        ret = dc_imex_has_backup(context, context.get_blobdir());
        if ret.is_null() {
            ret = dc_strdup(b"No backup found.\x00" as *const u8 as *const libc::c_char)
        }
    } else if strcmp(
        cmd,
        b"export-backup\x00" as *const u8 as *const libc::c_char,
    ) == 0i32
    {
        dc_imex(
            context,
            11i32,
            context.get_blobdir(),
            0 as *const libc::c_char,
        );
        ret = 2i32 as *mut libc::c_char
    } else if strcmp(
        cmd,
        b"import-backup\x00" as *const u8 as *const libc::c_char,
    ) == 0i32
    {
        if !arg1.is_empty() {
            dc_imex(context, 12i32, arg1_c.as_ptr(), 0 as *const libc::c_char);
            ret = 2i32 as *mut libc::c_char
        } else {
            ret = dc_strdup(
                b"ERROR: Argument <backup-file> missing.\x00" as *const u8 as *const libc::c_char,
            )
        }
    } else if strcmp(cmd, b"export-keys\x00" as *const u8 as *const libc::c_char) == 0i32 {
        dc_imex(
            context,
            1i32,
            context.get_blobdir(),
            0 as *const libc::c_char,
        );
        ret = 2i32 as *mut libc::c_char
    } else if strcmp(cmd, b"import-keys\x00" as *const u8 as *const libc::c_char) == 0i32 {
        dc_imex(
            context,
            2i32,
            context.get_blobdir(),
            0 as *const libc::c_char,
        );
        ret = 2i32 as *mut libc::c_char
    } else if strcmp(cmd, b"export-setup\x00" as *const u8 as *const libc::c_char) == 0i32 {
        let setup_code_0: *mut libc::c_char = dc_create_setup_code(context);
        let file_name: *mut libc::c_char = dc_mprintf(
            b"%s/autocrypt-setup-message.html\x00" as *const u8 as *const libc::c_char,
            context.get_blobdir(),
        );
        let file_content: *mut libc::c_char;
        file_content = dc_render_setup_file(context, setup_code_0);
        if !file_content.is_null()
            && 0 != dc_write_file(
                context,
                file_name,
                file_content as *const libc::c_void,
                strlen(file_content),
            )
        {
            ret = dc_mprintf(
                b"Setup message written to: %s\nSetup code: %s\x00" as *const u8
                    as *const libc::c_char,
                file_name,
                setup_code_0,
            )
        } else {
            ret = 1i32 as *mut libc::c_char
        }
        free(file_content as *mut libc::c_void);
        free(file_name as *mut libc::c_void);
        free(setup_code_0 as *mut libc::c_void);
    } else if strcmp(cmd, b"poke\x00" as *const u8 as *const libc::c_char) == 0i32 {
        ret = if 0 != poke_spec(context, arg1_c.as_ptr()) {
            2i32 as *mut libc::c_char
        } else {
            1i32 as *mut libc::c_char
        }
    } else if strcmp(cmd, b"reset\x00" as *const u8 as *const libc::c_char) == 0i32 {
        if !arg1.is_empty() {
            let bits: libc::c_int = arg1.parse().unwrap();
            if bits > 15i32 {
                ret = dc_strdup(
                    b"ERROR: <bits> must be lower than 16.\x00" as *const u8 as *const libc::c_char,
                )
            } else {
                ret = if 0 != dc_reset_tables(context, bits) {
                    2i32 as *mut libc::c_char
                } else {
                    1i32 as *mut libc::c_char
                }
            }
        } else {
            ret =
                    dc_strdup(b"ERROR: Argument <bits> missing: 1=jobs, 2=peerstates, 4=private keys, 8=rest but server config\x00"
                                  as *const u8 as *const libc::c_char)
        }
    } else if strcmp(cmd, b"stop\x00" as *const u8 as *const libc::c_char) == 0i32 {
        dc_stop_ongoing_process(context);
        ret = 2i32 as *mut libc::c_char
    } else if strcmp(cmd, b"set\x00" as *const u8 as *const libc::c_char) == 0i32 {
        if !arg1.is_empty() {
            ret = if 0 != dc_set_config(context, arg1_c.as_ptr(), arg2_c.as_ptr()) {
                2i32 as *mut libc::c_char
            } else {
                1i32 as *mut libc::c_char
            }
        } else {
            ret =
                dc_strdup(b"ERROR: Argument <key> missing.\x00" as *const u8 as *const libc::c_char)
        }
    } else if strcmp(cmd, b"get\x00" as *const u8 as *const libc::c_char) == 0i32 {
        if !arg1.is_empty() {
            let val: *mut libc::c_char = dc_get_config(context, arg1_c.as_ptr());
            ret = dc_mprintf(
                b"%s=%s\x00" as *const u8 as *const libc::c_char,
                arg1_c.as_ptr(),
                val,
            );
            free(val as *mut libc::c_void);
        } else {
            ret =
                dc_strdup(b"ERROR: Argument <key> missing.\x00" as *const u8 as *const libc::c_char)
        }
    } else if strcmp(cmd, b"info\x00" as *const u8 as *const libc::c_char) == 0i32 {
        ret = dc_get_info(context);
        if ret.is_null() {
            ret = 1i32 as *mut libc::c_char
        }
    } else if strcmp(cmd, b"maybenetwork\x00" as *const u8 as *const libc::c_char) == 0i32 {
        dc_maybe_network(context);
        ret = 2i32 as *mut libc::c_char
    } else if strcmp(cmd, b"housekeeping\x00" as *const u8 as *const libc::c_char) == 0i32 {
        dc_housekeeping(context);
        ret = 2i32 as *mut libc::c_char
    } else if strcmp(cmd, b"listchats\x00" as *const u8 as *const libc::c_char) == 0i32
        || strcmp(cmd, b"listarchived\x00" as *const u8 as *const libc::c_char) == 0i32
        || strcmp(cmd, b"chats\x00" as *const u8 as *const libc::c_char) == 0i32
    {
        let listflags: libc::c_int =
            if strcmp(cmd, b"listarchived\x00" as *const u8 as *const libc::c_char) == 0i32 {
                0x1i32
            } else {
                0i32
            };

        let chatlist: *mut dc_chatlist_t =
            dc_get_chatlist(context, listflags, arg1_c.as_ptr(), 0i32 as uint32_t);
        if !chatlist.is_null() {
            let mut i: libc::c_int;
            let cnt: libc::c_int = dc_chatlist_get_cnt(chatlist) as libc::c_int;
            if cnt > 0i32 {
                dc_log_info(context, 0i32,
                                b"================================================================================\x00"
                                    as *const u8 as *const libc::c_char);
                i = cnt - 1i32;

                while i >= 0i32 {
                    let chat: *mut dc_chat_t =
                        dc_get_chat(context, dc_chatlist_get_chat_id(chatlist, i as size_t));
                    let temp_subtitle: *mut libc::c_char = dc_chat_get_subtitle(chat);
                    let temp_name: *mut libc::c_char = dc_chat_get_name(chat);
                    dc_log_info(
                        context,
                        0i32,
                        b"%s#%i: %s [%s] [%i fresh]\x00" as *const u8 as *const libc::c_char,
                        chat_prefix(chat),
                        dc_chat_get_id(chat) as libc::c_int,
                        temp_name,
                        temp_subtitle,
                        dc_get_fresh_msg_cnt(context, dc_chat_get_id(chat)) as libc::c_int,
                    );
                    free(temp_subtitle as *mut libc::c_void);
                    free(temp_name as *mut libc::c_void);
                    let lot: *mut dc_lot_t = dc_chatlist_get_summary(chatlist, i as size_t, chat);
                    let mut statestr: *const libc::c_char =
                        b"\x00" as *const u8 as *const libc::c_char;
                    if 0 != dc_chat_get_archived(chat) {
                        statestr = b" [Archived]\x00" as *const u8 as *const libc::c_char
                    } else {
                        match dc_lot_get_state(lot) {
                            20 => statestr = b" o\x00" as *const u8 as *const libc::c_char,
                            26 => {
                                statestr = b" \xe2\x88\x9a\x00" as *const u8 as *const libc::c_char
                            }
                            28 => {
                                statestr = b" \xe2\x88\x9a\xe2\x88\x9a\x00" as *const u8
                                    as *const libc::c_char
                            }
                            24 => statestr = b" !!\x00" as *const u8 as *const libc::c_char,
                            _ => {}
                        }
                    }
                    let timestr: *mut libc::c_char = dc_timestamp_to_str(dc_lot_get_timestamp(lot));
                    let text1: *mut libc::c_char = dc_lot_get_text1(lot);
                    let text2: *mut libc::c_char = dc_lot_get_text2(lot);
                    dc_log_info(
                        context,
                        0i32,
                        b"%s%s%s%s [%s]%s\x00" as *const u8 as *const libc::c_char,
                        if !text1.is_null() {
                            text1
                        } else {
                            b"\x00" as *const u8 as *const libc::c_char
                        },
                        if !text1.is_null() {
                            b": \x00" as *const u8 as *const libc::c_char
                        } else {
                            b"\x00" as *const u8 as *const libc::c_char
                        },
                        if !text2.is_null() {
                            text2
                        } else {
                            b"\x00" as *const u8 as *const libc::c_char
                        },
                        statestr,
                        timestr,
                        if 0 != dc_chat_is_sending_locations(chat) {
                            b"\xf0\x9f\x93\x8d\x00" as *const u8 as *const libc::c_char
                        } else {
                            b"\x00" as *const u8 as *const libc::c_char
                        },
                    );
                    free(text1 as *mut libc::c_void);
                    free(text2 as *mut libc::c_void);
                    free(timestr as *mut libc::c_void);
                    dc_lot_unref(lot);
                    dc_chat_unref(chat);
                    dc_log_info(context, 0i32,
                                    b"================================================================================\x00"
                                        as *const u8 as *const libc::c_char);
                    i -= 1
                }
            }
            if 0 != dc_is_sending_locations_to_chat(context, 0i32 as uint32_t) {
                dc_log_info(
                    context,
                    0i32,
                    b"Location streaming enabled.\x00" as *const u8 as *const libc::c_char,
                );
            }
            ret = dc_mprintf(
                b"%i chats.\x00" as *const u8 as *const libc::c_char,
                cnt as libc::c_int,
            );
            dc_chatlist_unref(chatlist);
        } else {
            ret = 1i32 as *mut libc::c_char
        }
    } else if strcmp(cmd, b"chat\x00" as *const u8 as *const libc::c_char) == 0i32 {
        if !arg1.is_empty() {
            if !sel_chat.is_null() {
                dc_chat_unref(sel_chat);
            }

            let chat_id = arg1.parse().unwrap();
            sel_chat = dc_get_chat(context, chat_id);
            *context.cmdline_sel_chat_id.write().unwrap() = chat_id;
        }
        if !sel_chat.is_null() {
            let msglist: *mut dc_array_t = dc_get_chat_msgs(
                context,
                dc_chat_get_id(sel_chat),
                0x1i32 as uint32_t,
                0i32 as uint32_t,
            );
            let temp2: *mut libc::c_char = dc_chat_get_subtitle(sel_chat);
            let temp_name_0: *mut libc::c_char = dc_chat_get_name(sel_chat);
            dc_log_info(
                context,
                0i32,
                b"%s#%i: %s [%s]%s\x00" as *const u8 as *const libc::c_char,
                chat_prefix(sel_chat),
                dc_chat_get_id(sel_chat),
                temp_name_0,
                temp2,
                if 0 != dc_chat_is_sending_locations(sel_chat) {
                    b"\xf0\x9f\x93\x8d\x00" as *const u8 as *const libc::c_char
                } else {
                    b"\x00" as *const u8 as *const libc::c_char
                },
            );
            free(temp_name_0 as *mut libc::c_void);
            free(temp2 as *mut libc::c_void);
            if !msglist.is_null() {
                log_msglist(context, msglist);
                dc_array_unref(msglist);
            }
            let draft: *mut dc_msg_t = dc_get_draft(context, dc_chat_get_id(sel_chat));
            if !draft.is_null() {
                log_msg(
                    context,
                    b"Draft\x00" as *const u8 as *const libc::c_char,
                    draft,
                );
                dc_msg_unref(draft);
            }
            ret = dc_mprintf(
                b"%i messages.\x00" as *const u8 as *const libc::c_char,
                dc_get_msg_cnt(context, dc_chat_get_id(sel_chat)),
            );
            dc_marknoticed_chat(context, dc_chat_get_id(sel_chat));
        } else {
            ret = dc_strdup(b"No chat selected.\x00" as *const u8 as *const libc::c_char)
        }
    } else if strcmp(cmd, b"createchat\x00" as *const u8 as *const libc::c_char) == 0i32 {
        if !arg1.is_empty() {
            let contact_id: libc::c_int = arg1.parse().unwrap();
            let chat_id: libc::c_int =
                dc_create_chat_by_contact_id(context, contact_id as uint32_t) as libc::c_int;
            ret = if chat_id != 0i32 {
                dc_mprintf(
                    b"Single#%lu created successfully.\x00" as *const u8 as *const libc::c_char,
                    chat_id,
                )
            } else {
                1i32 as *mut libc::c_char
            }
        } else {
            ret = dc_strdup(
                b"ERROR: Argument <contact-id> missing.\x00" as *const u8 as *const libc::c_char,
            )
        }
    } else if strcmp(
        cmd,
        b"createchatbymsg\x00" as *const u8 as *const libc::c_char,
    ) == 0i32
    {
        if !arg1.is_empty() {
            let msg_id_0: libc::c_int = arg1.parse().unwrap();
            let chat_id_0: libc::c_int =
                dc_create_chat_by_msg_id(context, msg_id_0 as uint32_t) as libc::c_int;
            if chat_id_0 != 0i32 {
                let chat_0: *mut dc_chat_t = dc_get_chat(context, chat_id_0 as uint32_t);
                ret = dc_mprintf(
                    b"%s#%lu created successfully.\x00" as *const u8 as *const libc::c_char,
                    chat_prefix(chat_0),
                    chat_id_0,
                );
                dc_chat_unref(chat_0);
            } else {
                ret = 1i32 as *mut libc::c_char
            }
        } else {
            ret = dc_strdup(
                b"ERROR: Argument <msg-id> missing.\x00" as *const u8 as *const libc::c_char,
            )
        }
    } else if strcmp(cmd, b"creategroup\x00" as *const u8 as *const libc::c_char) == 0i32 {
        if !arg1.is_empty() {
            let chat_id_1: libc::c_int =
                dc_create_group_chat(context, 0i32, arg1_c.as_ptr()) as libc::c_int;
            ret = if chat_id_1 != 0i32 {
                dc_mprintf(
                    b"Group#%lu created successfully.\x00" as *const u8 as *const libc::c_char,
                    chat_id_1,
                )
            } else {
                1i32 as *mut libc::c_char
            }
        } else {
            ret = dc_strdup(
                b"ERROR: Argument <name> missing.\x00" as *const u8 as *const libc::c_char,
            )
        }
    } else if strcmp(
        cmd,
        b"createverified\x00" as *const u8 as *const libc::c_char,
    ) == 0i32
    {
        if !arg1.is_empty() {
            let chat_id_2: libc::c_int =
                dc_create_group_chat(context, 1i32, arg1_c.as_ptr()) as libc::c_int;
            ret = if chat_id_2 != 0i32 {
                dc_mprintf(
                    b"VerifiedGroup#%lu created successfully.\x00" as *const u8
                        as *const libc::c_char,
                    chat_id_2,
                )
            } else {
                1i32 as *mut libc::c_char
            }
        } else {
            ret = dc_strdup(
                b"ERROR: Argument <name> missing.\x00" as *const u8 as *const libc::c_char,
            )
        }
    } else if strcmp(cmd, b"addmember\x00" as *const u8 as *const libc::c_char) == 0i32 {
        if !sel_chat.is_null() {
            if !arg1.is_empty() {
                let contact_id_0: libc::c_int = arg1.parse().unwrap();
                if 0 != dc_add_contact_to_chat(
                    context,
                    dc_chat_get_id(sel_chat),
                    contact_id_0 as uint32_t,
                ) {
                    ret =
                        dc_strdup(b"Contact added to chat.\x00" as *const u8 as *const libc::c_char)
                } else {
                    ret = dc_strdup(
                        b"ERROR: Cannot add contact to chat.\x00" as *const u8
                            as *const libc::c_char,
                    )
                }
            } else {
                ret = dc_strdup(
                    b"ERROR: Argument <contact-id> missing.\x00" as *const u8
                        as *const libc::c_char,
                )
            }
        } else {
            ret = dc_strdup(b"No chat selected.\x00" as *const u8 as *const libc::c_char)
        }
    } else if strcmp(cmd, b"removemember\x00" as *const u8 as *const libc::c_char) == 0i32 {
        if !sel_chat.is_null() {
            if !arg1.is_empty() {
                let contact_id_1: libc::c_int = arg1.parse().unwrap();
                if 0 != dc_remove_contact_from_chat(
                    context,
                    dc_chat_get_id(sel_chat),
                    contact_id_1 as uint32_t,
                ) {
                    ret =
                        dc_strdup(b"Contact added to chat.\x00" as *const u8 as *const libc::c_char)
                } else {
                    ret = dc_strdup(
                        b"ERROR: Cannot remove member from chat.\x00" as *const u8
                            as *const libc::c_char,
                    )
                }
            } else {
                ret = dc_strdup(
                    b"ERROR: Argument <contact-id> missing.\x00" as *const u8
                        as *const libc::c_char,
                )
            }
        } else {
            ret = dc_strdup(b"No chat selected.\x00" as *const u8 as *const libc::c_char)
        }
    } else if strcmp(cmd, b"groupname\x00" as *const u8 as *const libc::c_char) == 0i32 {
        if !sel_chat.is_null() {
            if !arg1.is_empty() {
                ret = if 0 != dc_set_chat_name(context, dc_chat_get_id(sel_chat), arg1_c.as_ptr()) {
                    2i32 as *mut libc::c_char
                } else {
                    1i32 as *mut libc::c_char
                }
            } else {
                ret = dc_strdup(
                    b"ERROR: Argument <name> missing.\x00" as *const u8 as *const libc::c_char,
                )
            }
        } else {
            ret = dc_strdup(b"No chat selected.\x00" as *const u8 as *const libc::c_char)
        }
    } else if strcmp(cmd, b"groupimage\x00" as *const u8 as *const libc::c_char) == 0i32 {
        if !sel_chat.is_null() {
            ret = if 0
                != dc_set_chat_profile_image(
                    context,
                    dc_chat_get_id(sel_chat),
                    if !arg1.is_empty() {
                        arg1_c.as_ptr()
                    } else {
                        0 as *mut libc::c_char
                    },
                ) {
                2i32 as *mut libc::c_char
            } else {
                1i32 as *mut libc::c_char
            }
        } else {
            ret = dc_strdup(b"No chat selected.\x00" as *const u8 as *const libc::c_char)
        }
    } else if strcmp(cmd, b"chatinfo\x00" as *const u8 as *const libc::c_char) == 0i32 {
        if !sel_chat.is_null() {
            let contacts: *mut dc_array_t = dc_get_chat_contacts(context, dc_chat_get_id(sel_chat));
            if !contacts.is_null() {
                dc_log_info(
                    context,
                    0i32,
                    b"Memberlist:\x00" as *const u8 as *const libc::c_char,
                );
                log_contactlist(context, contacts);
                ret = dc_mprintf(
                    b"%i contacts\nLocation streaming: %i\x00" as *const u8 as *const libc::c_char,
                    dc_array_get_cnt(contacts) as libc::c_int,
                    dc_is_sending_locations_to_chat(context, dc_chat_get_id(sel_chat)),
                );
                dc_array_unref(contacts);
            } else {
                ret = 1i32 as *mut libc::c_char
            }
        } else {
            ret = dc_strdup(b"No chat selected.\x00" as *const u8 as *const libc::c_char)
        }
    } else if strcmp(cmd, b"getlocations\x00" as *const u8 as *const libc::c_char) == 0i32 {
        let contact_id_2: libc::c_int = if !arg1.is_empty() {
            arg1.parse().unwrap()
        } else {
            0i32
        };
        let loc: *mut dc_array_t = dc_get_locations(
            context,
            dc_chat_get_id(sel_chat),
            contact_id_2 as uint32_t,
            0i32 as time_t,
            0i32 as time_t,
        );
        let mut j = 0;
        while j < dc_array_get_cnt(loc) {
            let timestr_0: *mut libc::c_char =
                dc_timestamp_to_str(dc_array_get_timestamp(loc, j as size_t));
            let marker: *mut libc::c_char = dc_array_get_marker(loc, j as size_t);
            dc_log_info(
                context,
                0i32,
                b"Loc#%i: %s: lat=%f lng=%f acc=%f Chat#%i Contact#%i Msg#%i %s\x00" as *const u8
                    as *const libc::c_char,
                dc_array_get_id(loc, j as size_t),
                timestr_0,
                dc_array_get_latitude(loc, j as size_t),
                dc_array_get_longitude(loc, j as size_t),
                dc_array_get_accuracy(loc, j as size_t),
                dc_array_get_chat_id(loc, j as size_t),
                dc_array_get_contact_id(loc, j as size_t),
                dc_array_get_msg_id(loc, j as size_t),
                if !marker.is_null() {
                    marker
                } else {
                    b"-\x00" as *const u8 as *const libc::c_char
                },
            );
            free(timestr_0 as *mut libc::c_void);
            free(marker as *mut libc::c_void);
            j += 1
        }
        if dc_array_get_cnt(loc) == 0 {
            dc_log_info(
                context,
                0i32,
                b"No locations.\x00" as *const u8 as *const libc::c_char,
            );
        }
        dc_array_unref(loc);
        ret = 2i32 as *mut libc::c_char
    } else if strcmp(
        cmd,
        b"sendlocations\x00" as *const u8 as *const libc::c_char,
    ) == 0i32
    {
        if !sel_chat.is_null() {
            if !arg1.is_empty() {
                let seconds: libc::c_int = arg1.parse().unwrap();
                dc_send_locations_to_chat(context, dc_chat_get_id(sel_chat), seconds);
                ret =
                        dc_mprintf(b"Locations will be sent to Chat#%i for %i seconds. Use \'setlocation <lat> <lng>\' to play around.\x00"
                                       as *const u8 as *const libc::c_char,
                                   dc_chat_get_id(sel_chat), seconds)
            } else {
                ret = dc_strdup(b"ERROR: No timeout given.\x00" as *const u8 as *const libc::c_char)
            }
        } else {
            ret = dc_strdup(b"No chat selected.\x00" as *const u8 as *const libc::c_char)
        }
    } else if strcmp(cmd, b"setlocation\x00" as *const u8 as *const libc::c_char) == 0i32 {
        if !arg1.is_empty() && !arg2.is_empty() {
            let latitude: libc::c_double = arg1.parse().unwrap();
            let longitude: libc::c_double = arg2.parse().unwrap();
            let continue_streaming: libc::c_int =
                dc_set_location(context, latitude, longitude, 0.0f64);
            ret = dc_strdup(if 0 != continue_streaming {
                b"Success, streaming should be continued.\x00" as *const u8 as *const libc::c_char
            } else {
                b"Success, streaming can be stoppped.\x00" as *const u8 as *const libc::c_char
            })
        } else {
            ret = dc_strdup(
                b"ERROR: Latitude or longitude not given.\x00" as *const u8 as *const libc::c_char,
            )
        }
    } else if strcmp(cmd, b"dellocations\x00" as *const u8 as *const libc::c_char) == 0i32 {
        dc_delete_all_locations(context);
        ret = 2i32 as *mut libc::c_char
    } else if strcmp(cmd, b"send\x00" as *const u8 as *const libc::c_char) == 0i32 {
        if !sel_chat.is_null() {
            if !arg1.is_empty() {
                if 0 != dc_send_text_msg(context, dc_chat_get_id(sel_chat), arg1_c.as_ptr()) {
                    ret = dc_strdup(b"Message sent.\x00" as *const u8 as *const libc::c_char)
                } else {
                    ret =
                        dc_strdup(b"ERROR: Sending failed.\x00" as *const u8 as *const libc::c_char)
                }
            } else {
                ret = dc_strdup(
                    b"ERROR: No message text given.\x00" as *const u8 as *const libc::c_char,
                )
            }
        } else {
            ret = dc_strdup(b"No chat selected.\x00" as *const u8 as *const libc::c_char)
        }
    } else if strcmp(cmd, b"sendempty\x00" as *const u8 as *const libc::c_char) == 0i32 {
        if !sel_chat.is_null() {
            if 0 != dc_send_text_msg(
                context,
                dc_chat_get_id(sel_chat),
                b"\x00" as *const u8 as *const libc::c_char,
            ) {
                ret = dc_strdup(b"Message sent.\x00" as *const u8 as *const libc::c_char)
            } else {
                ret = dc_strdup(b"ERROR: Sending failed.\x00" as *const u8 as *const libc::c_char)
            }
        } else {
            ret = dc_strdup(b"No chat selected.\x00" as *const u8 as *const libc::c_char)
        }
    } else if strcmp(cmd, b"sendimage\x00" as *const u8 as *const libc::c_char) == 0i32
        || strcmp(cmd, b"sendfile\x00" as *const u8 as *const libc::c_char) == 0i32
    {
        if !sel_chat.is_null() {
            if !arg1.is_empty() && !arg2.is_empty() {
                let msg_0: *mut dc_msg_t = dc_msg_new(
                    context,
                    if strcmp(cmd, b"sendimage\x00" as *const u8 as *const libc::c_char) == 0i32 {
                        20i32
                    } else {
                        60i32
                    },
                );
                dc_msg_set_file(msg_0, arg1_c.as_ptr(), 0 as *const libc::c_char);
                dc_msg_set_text(msg_0, arg2_c.as_ptr());
                dc_send_msg(context, dc_chat_get_id(sel_chat), msg_0);
                dc_msg_unref(msg_0);
                ret = 2i32 as *mut libc::c_char
            } else {
                ret = dc_strdup(b"ERROR: No file given.\x00" as *const u8 as *const libc::c_char)
            }
        } else {
            ret = dc_strdup(b"No chat selected.\x00" as *const u8 as *const libc::c_char)
        }
    } else if strcmp(cmd, b"listmsgs\x00" as *const u8 as *const libc::c_char) == 0i32 {
        if !arg1.is_empty() {
            let msglist_0: *mut dc_array_t = dc_search_msgs(
                context,
                if !sel_chat.is_null() {
                    dc_chat_get_id(sel_chat)
                } else {
                    0i32 as libc::c_uint
                },
                arg1_c.as_ptr(),
            );
            if !msglist_0.is_null() {
                log_msglist(context, msglist_0);
                ret = dc_mprintf(
                    b"%i messages.\x00" as *const u8 as *const libc::c_char,
                    dc_array_get_cnt(msglist_0) as libc::c_int,
                );
                dc_array_unref(msglist_0);
            }
        } else {
            ret = dc_strdup(
                b"ERROR: Argument <query> missing.\x00" as *const u8 as *const libc::c_char,
            )
        }
    } else if strcmp(cmd, b"draft\x00" as *const u8 as *const libc::c_char) == 0i32 {
        if !sel_chat.is_null() {
            if !arg1.is_empty() {
                let draft_0: *mut dc_msg_t = dc_msg_new(context, 10i32);
                dc_msg_set_text(draft_0, arg1_c.as_ptr());
                dc_set_draft(context, dc_chat_get_id(sel_chat), draft_0);
                dc_msg_unref(draft_0);
                ret = dc_strdup(b"Draft saved.\x00" as *const u8 as *const libc::c_char)
            } else {
                dc_set_draft(context, dc_chat_get_id(sel_chat), 0 as *mut dc_msg_t);
                ret = dc_strdup(b"Draft deleted.\x00" as *const u8 as *const libc::c_char)
            }
        } else {
            ret = dc_strdup(b"No chat selected.\x00" as *const u8 as *const libc::c_char)
        }
    } else if strcmp(cmd, b"listmedia\x00" as *const u8 as *const libc::c_char) == 0i32 {
        if !sel_chat.is_null() {
            let images: *mut dc_array_t =
                dc_get_chat_media(context, dc_chat_get_id(sel_chat), 20i32, 21i32, 50i32);
            let mut i_0: libc::c_int;
            let icnt: libc::c_int = dc_array_get_cnt(images) as libc::c_int;
            ret = dc_mprintf(
                b"%i images or videos: \x00" as *const u8 as *const libc::c_char,
                icnt,
            );
            i_0 = 0i32;
            while i_0 < icnt {
                let temp: *mut libc::c_char = dc_mprintf(
                    b"%s%sMsg#%i\x00" as *const u8 as *const libc::c_char,
                    if 0 != i_0 {
                        b", \x00" as *const u8 as *const libc::c_char
                    } else {
                        b"\x00" as *const u8 as *const libc::c_char
                    },
                    ret,
                    dc_array_get_id(images, i_0 as size_t) as libc::c_int,
                );
                free(ret as *mut libc::c_void);
                ret = temp;
                i_0 += 1
            }
            dc_array_unref(images);
        } else {
            ret = dc_strdup(b"No chat selected.\x00" as *const u8 as *const libc::c_char)
        }
    } else if strcmp(cmd, b"archive\x00" as *const u8 as *const libc::c_char) == 0i32
        || strcmp(cmd, b"unarchive\x00" as *const u8 as *const libc::c_char) == 0i32
    {
        if !arg1.is_empty() {
            let chat_id_3: libc::c_int = arg1.parse().unwrap();
            dc_archive_chat(
                context,
                chat_id_3 as uint32_t,
                if strcmp(cmd, b"archive\x00" as *const u8 as *const libc::c_char) == 0i32 {
                    1i32
                } else {
                    0i32
                },
            );
            ret = 2i32 as *mut libc::c_char
        } else {
            ret = dc_strdup(
                b"ERROR: Argument <chat-id> missing.\x00" as *const u8 as *const libc::c_char,
            )
        }
    } else if strcmp(cmd, b"delchat\x00" as *const u8 as *const libc::c_char) == 0i32 {
        if !arg1.is_empty() {
            let chat_id_4: libc::c_int = arg1.parse().unwrap();
            dc_delete_chat(context, chat_id_4 as uint32_t);
            ret = 2i32 as *mut libc::c_char
        } else {
            ret = dc_strdup(
                b"ERROR: Argument <chat-id> missing.\x00" as *const u8 as *const libc::c_char,
            )
        }
    } else if strcmp(cmd, b"msginfo\x00" as *const u8 as *const libc::c_char) == 0i32 {
        if !arg1.is_empty() {
            let id: libc::c_int = arg1.parse().unwrap();
            ret = dc_get_msg_info(context, id as uint32_t)
        } else {
            ret = dc_strdup(
                b"ERROR: Argument <msg-id> missing.\x00" as *const u8 as *const libc::c_char,
            )
        }
    } else if strcmp(cmd, b"listfresh\x00" as *const u8 as *const libc::c_char) == 0i32 {
        let msglist_1: *mut dc_array_t = dc_get_fresh_msgs(context);
        if !msglist_1.is_null() {
            log_msglist(context, msglist_1);
            ret = dc_mprintf(
                b"%i fresh messages.\x00" as *const u8 as *const libc::c_char,
                dc_array_get_cnt(msglist_1) as libc::c_int,
            );
            dc_array_unref(msglist_1);
        }
    } else if strcmp(cmd, b"forward\x00" as *const u8 as *const libc::c_char) == 0i32 {
        if !arg1.is_empty() && !arg2.is_empty() {
            let mut msg_ids: [uint32_t; 1] = [0; 1];
            let chat_id_5: uint32_t = arg2.parse().unwrap();
            msg_ids[0usize] = arg1.parse().unwrap();
            dc_forward_msgs(context, msg_ids.as_mut_ptr(), 1i32, chat_id_5);
            ret = 2i32 as *mut libc::c_char
        } else {
            ret = dc_strdup(
                b"ERROR: Arguments <msg-id> <chat-id> expected.\x00" as *const u8
                    as *const libc::c_char,
            )
        }
    } else if strcmp(cmd, b"markseen\x00" as *const u8 as *const libc::c_char) == 0i32 {
        if !arg1.is_empty() {
            let mut msg_ids_0: [uint32_t; 1] = [0; 1];
            msg_ids_0[0usize] = arg1.parse().unwrap();
            dc_markseen_msgs(context, msg_ids_0.as_mut_ptr(), 1i32);
            ret = 2i32 as *mut libc::c_char
        } else {
            ret = dc_strdup(
                b"ERROR: Argument <msg-id> missing.\x00" as *const u8 as *const libc::c_char,
            )
        }
    } else if strcmp(cmd, b"star\x00" as *const u8 as *const libc::c_char) == 0i32
        || strcmp(cmd, b"unstar\x00" as *const u8 as *const libc::c_char) == 0i32
    {
        if !arg1.is_empty() {
            let mut msg_ids_1: [uint32_t; 1] = [0; 1];
            msg_ids_1[0] = arg1.parse().unwrap();
            dc_star_msgs(
                context,
                msg_ids_1.as_mut_ptr(),
                1i32,
                if strcmp(cmd, b"star\x00" as *const u8 as *const libc::c_char) == 0i32 {
                    1i32
                } else {
                    0i32
                },
            );
            ret = 2i32 as *mut libc::c_char
        } else {
            ret = dc_strdup(
                b"ERROR: Argument <msg-id> missing.\x00" as *const u8 as *const libc::c_char,
            )
        }
    } else if strcmp(cmd, b"delmsg\x00" as *const u8 as *const libc::c_char) == 0i32 {
        if !arg1.is_empty() {
            let mut ids: [uint32_t; 1] = [0; 1];
            ids[0usize] = arg1.parse().unwrap();
            dc_delete_msgs(context, ids.as_mut_ptr(), 1i32);
            ret = 2i32 as *mut libc::c_char
        } else {
            ret = dc_strdup(
                b"ERROR: Argument <msg-id> missing.\x00" as *const u8 as *const libc::c_char,
            )
        }
    } else if strcmp(cmd, b"listcontacts\x00" as *const u8 as *const libc::c_char) == 0i32
        || strcmp(cmd, b"contacts\x00" as *const u8 as *const libc::c_char) == 0i32
        || strcmp(cmd, b"listverified\x00" as *const u8 as *const libc::c_char) == 0i32
    {
        let contacts_0: *mut dc_array_t = dc_get_contacts(
            context,
            (if strcmp(cmd, b"listverified\x00" as *const u8 as *const libc::c_char) == 0i32 {
                0x1i32 | 0x2i32
            } else {
                0x2i32
            }) as uint32_t,
            arg1_c.as_ptr(),
        );
        if !contacts_0.is_null() {
            log_contactlist(context, contacts_0);
            ret = dc_mprintf(
                b"%i contacts.\x00" as *const u8 as *const libc::c_char,
                dc_array_get_cnt(contacts_0) as libc::c_int,
            );
            dc_array_unref(contacts_0);
        } else {
            ret = 1i32 as *mut libc::c_char
        }
    } else if strcmp(cmd, b"addcontact\x00" as *const u8 as *const libc::c_char) == 0i32 {
        if !arg1.is_empty() && !arg2.is_empty() {
            let book: *mut libc::c_char = dc_mprintf(
                b"%s\n%s\x00" as *const u8 as *const libc::c_char,
                arg1_c.as_ptr(),
                arg2_c.as_ptr(),
            );
            dc_add_address_book(context, book);
            ret = 2i32 as *mut libc::c_char;
            free(book as *mut libc::c_void);
        } else if !arg1.is_empty() {
            ret = if 0 != dc_create_contact(context, 0 as *const libc::c_char, arg1_c.as_ptr()) {
                2i32 as *mut libc::c_char
            } else {
                1i32 as *mut libc::c_char
            }
        } else {
            ret = dc_strdup(
                b"ERROR: Arguments [<name>] <addr> expected.\x00" as *const u8
                    as *const libc::c_char,
            )
        }
    } else if strcmp(cmd, b"contactinfo\x00" as *const u8 as *const libc::c_char) == 0i32 {
        if !arg1.is_empty() {
            let contact_id_3: libc::c_int = arg1.parse().unwrap();
            let mut strbuilder: dc_strbuilder_t = dc_strbuilder_t {
                buf: 0 as *mut libc::c_char,
                allocated: 0,
                free: 0,
                eos: 0 as *mut libc::c_char,
            };

            dc_strbuilder_init(&mut strbuilder, 0i32);
            let contact: *mut dc_contact_t = dc_get_contact(context, contact_id_3 as uint32_t);
            let nameNaddr: *mut libc::c_char = dc_contact_get_name_n_addr(contact);
            dc_strbuilder_catf(
                &mut strbuilder as *mut dc_strbuilder_t,
                b"Contact info for: %s:\n\n\x00" as *const u8 as *const libc::c_char,
                nameNaddr,
            );
            free(nameNaddr as *mut libc::c_void);
            dc_contact_unref(contact);
            let encrinfo: *mut libc::c_char =
                dc_get_contact_encrinfo(context, contact_id_3 as uint32_t);
            dc_strbuilder_cat(&mut strbuilder, encrinfo);
            free(encrinfo as *mut libc::c_void);
            let chatlist_0: *mut dc_chatlist_t = dc_get_chatlist(
                context,
                0i32,
                0 as *const libc::c_char,
                contact_id_3 as uint32_t,
            );
            let chatlist_cnt: libc::c_int = dc_chatlist_get_cnt(chatlist_0) as libc::c_int;
            if chatlist_cnt > 0i32 {
                dc_strbuilder_catf(
                    &mut strbuilder as *mut dc_strbuilder_t,
                    b"\n\n%i chats shared with Contact#%i: \x00" as *const u8
                        as *const libc::c_char,
                    chatlist_cnt,
                    contact_id_3,
                );
                let mut i_1: libc::c_int = 0i32;
                while i_1 < chatlist_cnt {
                    if 0 != i_1 {
                        dc_strbuilder_cat(
                            &mut strbuilder,
                            b", \x00" as *const u8 as *const libc::c_char,
                        );
                    }
                    let chat_1: *mut dc_chat_t =
                        dc_get_chat(context, dc_chatlist_get_chat_id(chatlist_0, i_1 as size_t));
                    dc_strbuilder_catf(
                        &mut strbuilder as *mut dc_strbuilder_t,
                        b"%s#%i\x00" as *const u8 as *const libc::c_char,
                        chat_prefix(chat_1),
                        dc_chat_get_id(chat_1),
                    );
                    dc_chat_unref(chat_1);
                    i_1 += 1
                }
            }
            dc_chatlist_unref(chatlist_0);
            ret = strbuilder.buf
        } else {
            ret = dc_strdup(
                b"ERROR: Argument <contact-id> missing.\x00" as *const u8 as *const libc::c_char,
            )
        }
    } else if strcmp(cmd, b"delcontact\x00" as *const u8 as *const libc::c_char) == 0i32 {
        if !arg1.is_empty() {
            ret = if 0 != dc_delete_contact(context, arg1.parse().unwrap()) {
                2i32 as *mut libc::c_char
            } else {
                1i32 as *mut libc::c_char
            }
        } else {
            ret = dc_strdup(
                b"ERROR: Argument <contact-id> missing.\x00" as *const u8 as *const libc::c_char,
            )
        }
    } else if strcmp(cmd, b"getqr\x00" as *const u8 as *const libc::c_char) == 0i32 {
        ret = dc_get_securejoin_qr(
            context,
            (if !arg1.is_empty() {
                arg1.parse().unwrap()
            } else {
                0i32
            }) as uint32_t,
        );
        if ret.is_null() || *ret.offset(0isize) as libc::c_int == 0i32 {
            free(ret as *mut libc::c_void);
            ret = 1i32 as *mut libc::c_char
        }
    } else if strcmp(cmd, b"checkqr\x00" as *const u8 as *const libc::c_char) == 0i32 {
        if !arg1.is_empty() {
            let res: *mut dc_lot_t = dc_check_qr(context, arg1_c.as_ptr());
            ret = dc_mprintf(
                b"state=%i, id=%i, text1=%s, text2=%s\x00" as *const u8 as *const libc::c_char,
                (*res).state as libc::c_int,
                (*res).id,
                if !(*res).text1.is_null() {
                    (*res).text1
                } else {
                    b"\x00" as *const u8 as *const libc::c_char
                },
                if !(*res).text2.is_null() {
                    (*res).text2
                } else {
                    b"\x00" as *const u8 as *const libc::c_char
                },
            );
            dc_lot_unref(res);
        } else {
            ret = dc_strdup(
                b"ERROR: Argument <qr-content> missing.\x00" as *const u8 as *const libc::c_char,
            )
        }
    } else if strcmp(cmd, b"event\x00" as *const u8 as *const libc::c_char) == 0i32 {
        if !arg1.is_empty() {
            let event = Event::from_u32(arg1.parse().unwrap()).unwrap();
            let r: uintptr_t = (context.cb)(context, event, 0i32 as uintptr_t, 0i32 as uintptr_t);
            ret = dc_mprintf(
                b"Sending event %i, received value %i.\x00" as *const u8 as *const libc::c_char,
                event as libc::c_int,
                r as libc::c_int,
            )
        } else {
            ret =
                dc_strdup(b"ERROR: Argument <id> missing.\x00" as *const u8 as *const libc::c_char)
        }
    } else if strcmp(cmd, b"fileinfo\x00" as *const u8 as *const libc::c_char) == 0i32 {
        if !arg1.is_empty() {
            let mut buf: *mut libc::c_uchar = 0 as *mut libc::c_uchar;
            let mut buf_bytes: size_t = 0;
            let mut w: uint32_t = 0;
            let mut h: uint32_t = 0;

            if 0 != dc_read_file(
                context,
                arg1_c.as_ptr(),
                &mut buf as *mut *mut libc::c_uchar as *mut *mut libc::c_void,
                &mut buf_bytes,
            ) {
                dc_get_filemeta(buf as *const libc::c_void, buf_bytes, &mut w, &mut h);
                ret = dc_mprintf(
                    b"width=%i, height=%i\x00" as *const u8 as *const libc::c_char,
                    w as libc::c_int,
                    h as libc::c_int,
                )
            } else {
                ret = dc_strdup(b"ERROR: Command failed.\x00" as *const u8 as *const libc::c_char)
            }
            free(buf as *mut libc::c_void);
        } else {
            ret = dc_strdup(
                b"ERROR: Argument <file> missing.\x00" as *const u8 as *const libc::c_char,
            )
        }
    } else {
        ret = 3i32 as *mut libc::c_char
    }

    if ret == 2i32 as *mut libc::c_char {
        ret = dc_strdup(b"Command executed successfully.\x00" as *const u8 as *const libc::c_char)
    } else if ret == 1i32 as *mut libc::c_char {
        ret = dc_strdup(b"ERROR: Command failed.\x00" as *const u8 as *const libc::c_char)
    } else if ret == 3i32 as *mut libc::c_char {
        ret = dc_mprintf(
            b"ERROR: Unknown command \"%s\", type ? for help.\x00" as *const u8
                as *const libc::c_char,
            cmd,
        )
    }
    if !sel_chat.is_null() {
        dc_chat_unref(sel_chat);
    }
    free(cmd as *mut libc::c_void);
    ret
}
