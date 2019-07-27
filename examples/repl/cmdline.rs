use std::str::FromStr;

use deltachat::config;
use deltachat::constants::*;
use deltachat::context::*;
use deltachat::dc_array::*;
use deltachat::dc_chat::*;
use deltachat::dc_chatlist::*;
use deltachat::dc_configure::*;
use deltachat::dc_contact::*;
use deltachat::dc_imex::*;
use deltachat::dc_job::*;
use deltachat::dc_location::*;
use deltachat::dc_lot::*;
use deltachat::dc_msg::*;
use deltachat::dc_qr::*;
use deltachat::dc_receive_imf::*;
use deltachat::dc_tools::*;
use deltachat::peerstate::*;
use deltachat::sql;
use deltachat::types::*;
use deltachat::x::*;
use num_traits::FromPrimitive;

/// Reset database tables. This function is called from Core cmdline.
/// Argument is a bitmask, executing single or multiple actions in one call.
/// e.g. bitmask 7 triggers actions definded with bits 1, 2 and 4.
pub unsafe fn dc_reset_tables(context: &Context, bits: i32) -> i32 {
    info!(context, 0, "Resetting tables ({})...", bits);
    if 0 != bits & 1 {
        sql::execute(context, &context.sql, "DELETE FROM jobs;", params![]).unwrap();
        info!(context, 0, "(1) Jobs reset.");
    }
    if 0 != bits & 2 {
        sql::execute(
            context,
            &context.sql,
            "DELETE FROM acpeerstates;",
            params![],
        )
        .unwrap();
        info!(context, 0, "(2) Peerstates reset.");
    }
    if 0 != bits & 4 {
        sql::execute(context, &context.sql, "DELETE FROM keypairs;", params![]).unwrap();
        info!(context, 0, "(4) Private keypairs reset.");
    }
    if 0 != bits & 8 {
        sql::execute(
            context,
            &context.sql,
            "DELETE FROM contacts WHERE id>9;",
            params![],
        )
        .unwrap();
        sql::execute(
            context,
            &context.sql,
            "DELETE FROM chats WHERE id>9;",
            params![],
        )
        .unwrap();
        sql::execute(
            context,
            &context.sql,
            "DELETE FROM chats_contacts;",
            params![],
        )
        .unwrap();
        sql::execute(
            context,
            &context.sql,
            "DELETE FROM msgs WHERE id>9;",
            params![],
        )
        .unwrap();
        sql::execute(
            context,
            &context.sql,
            "DELETE FROM config WHERE keyname LIKE 'imap.%' OR keyname LIKE 'configured%';",
            params![],
        )
        .unwrap();
        sql::execute(context, &context.sql, "DELETE FROM leftgrps;", params![]).unwrap();
        info!(context, 0, "(8) Rest but server config reset.");
    }

    context.call_cb(Event::MSGS_CHANGED, 0, 0);

    1
}

unsafe fn dc_poke_eml_file(context: &Context, filename: *const libc::c_char) -> libc::c_int {
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
        dc_receive_imf(context, data, data_bytes, "import", 0, 0);
        success = 1;
    }
    free(data as *mut libc::c_void);

    success
}

/// Import a file to the database.
/// For testing, import a folder with eml-files, a single eml-file, e-mail plus public key and so on.
/// For normal importing, use dc_imex().
///
/// @private @memberof Context
/// @param context The context as created by dc_context_new().
/// @param spec The file or directory to import. NULL for the last command.
/// @return 1=success, 0=error.
unsafe fn poke_spec(context: &Context, spec: *const libc::c_char) -> libc::c_int {
    if !context.sql.is_open() {
        error!(context, 0, "Import: Database not opened.");
        return 0;
    }

    let mut current_block: u64;
    let mut success: libc::c_int = 0;
    let real_spec: *mut libc::c_char;
    let mut suffix: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut read_cnt: libc::c_int = 0;

    /* if `spec` is given, remember it for later usage; if it is not given, try to use the last one */
    if !spec.is_null() {
        real_spec = dc_strdup(spec);
        context
            .sql
            .set_config(context, "import_spec", Some(as_str(real_spec)))
            .unwrap();
        current_block = 7149356873433890176;
    } else {
        let rs = context.sql.get_config(context, "import_spec");
        if rs.is_none() {
            error!(context, 0, "Import: No file or folder given.");
            current_block = 8522321847195001863;
        } else {
            current_block = 7149356873433890176;
        }
        real_spec = to_cstring(rs.unwrap_or_default());
    }
    match current_block {
        8522321847195001863 => {}
        _ => {
            suffix = dc_get_filesuffix_lc(real_spec);
            if !suffix.is_null()
                && strcmp(suffix, b"eml\x00" as *const u8 as *const libc::c_char) == 0
            {
                if 0 != dc_poke_eml_file(context, real_spec) {
                    read_cnt += 1
                }
                current_block = 1622411330066726685;
            } else {
                /* import a directory */
                let dir_name = std::path::Path::new(as_str(real_spec));
                let dir = std::fs::read_dir(dir_name);
                if dir.is_err() {
                    error!(
                        context,
                        0,
                        "Import: Cannot open directory \"{}\".",
                        as_str(real_spec),
                    );
                    current_block = 8522321847195001863;
                } else {
                    let dir = dir.unwrap();
                    for entry in dir {
                        if entry.is_err() {
                            break;
                        }
                        let entry = entry.unwrap();
                        let name_f = entry.file_name();
                        let name = name_f.to_string_lossy();
                        if name.ends_with(".eml") {
                            let path_plus_name = format!("{}/{}", as_str(real_spec), name);
                            info!(context, 0, "Import: {}", path_plus_name);
                            let path_plus_name_c = to_cstring(path_plus_name);
                            if 0 != dc_poke_eml_file(context, path_plus_name_c) {
                                read_cnt += 1
                            }
                            free(path_plus_name_c as *mut _);
                        }
                    }
                    current_block = 1622411330066726685;
                }
            }
            match current_block {
                8522321847195001863 => {}
                _ => {
                    info!(
                        context,
                        0,
                        "Import: {} items read from \"{}\".",
                        read_cnt,
                        as_str(real_spec)
                    );
                    if read_cnt > 0 {
                        context.call_cb(Event::MSGS_CHANGED, 0 as uintptr_t, 0 as uintptr_t);
                    }
                    success = 1
                }
            }
        }
    }

    free(real_spec as *mut libc::c_void);
    free(suffix as *mut libc::c_void);
    success
}

unsafe fn log_msg(context: &Context, prefix: impl AsRef<str>, msg: *mut dc_msg_t) {
    let contact: *mut dc_contact_t = dc_get_contact(context, dc_msg_get_from_id(msg));
    let contact_name: *mut libc::c_char = dc_contact_get_name(contact);
    let contact_id: libc::c_int = dc_contact_get_id(contact) as libc::c_int;
    let statestr = match dc_msg_get_state(msg) {
        DC_STATE_OUT_PENDING => " o",
        DC_STATE_OUT_DELIVERED => " √",
        DC_STATE_OUT_MDN_RCVD => " √√",
        DC_STATE_OUT_FAILED => " !!",
        _ => "",
    };
    let temp2: *mut libc::c_char = dc_timestamp_to_str(dc_msg_get_timestamp(msg));
    let msgtext: *mut libc::c_char = dc_msg_get_text(msg);
    info!(
        context,
        0,
        "{}#{}{}{}: {} (Contact#{}): {} {}{}{}{} [{}]",
        prefix.as_ref(),
        dc_msg_get_id(msg) as libc::c_int,
        if 0 != dc_msg_get_showpadlock(msg) {
            "🔒"
        } else {
            ""
        },
        if dc_msg_has_location(msg) { "📍" } else { "" },
        as_str(contact_name),
        contact_id,
        as_str(msgtext),
        if 0 != dc_msg_is_starred(msg) {
            "★"
        } else {
            ""
        },
        if dc_msg_get_from_id(msg) == 1 as libc::c_uint {
            ""
        } else if dc_msg_get_state(msg) == DC_STATE_IN_SEEN {
            "[SEEN]"
        } else if dc_msg_get_state(msg) == DC_STATE_IN_NOTICED {
            "[NOTICED]"
        } else {
            "[FRESH]"
        },
        if 0 != dc_msg_is_info(msg) {
            "[INFO]"
        } else {
            ""
        },
        statestr,
        as_str(temp2),
    );
    free(msgtext as *mut libc::c_void);
    free(temp2 as *mut libc::c_void);
    free(contact_name as *mut libc::c_void);
    dc_contact_unref(contact);
}

unsafe fn log_msglist(context: &Context, msglist: *mut dc_array_t) {
    let cnt = dc_array_get_cnt(msglist) as usize;
    let mut lines_out = 0;
    for i in 0..cnt {
        let msg_id = dc_array_get_id(msglist, i as size_t);
        if msg_id == 9 as libc::c_uint {
            info!(
                context,
                0,
                "--------------------------------------------------------------------------------"
            );

            lines_out += 1
        } else if msg_id > 0 as libc::c_uint {
            if lines_out == 0 {
                info!(
                    context, 0,
                    "--------------------------------------------------------------------------------",
                );
                lines_out += 1
            }
            let msg = dc_get_msg(context, msg_id);
            log_msg(context, "Msg", msg);
            dc_msg_unref(msg);
        }
    }
    if lines_out > 0 {
        info!(
            context,
            0, "--------------------------------------------------------------------------------"
        );
    }
}

unsafe fn log_contactlist(context: &Context, contacts: *mut dc_array_t) {
    let mut contact: *mut dc_contact_t;
    if !dc_array_search_id(contacts, 1 as uint32_t, 0 as *mut size_t) {
        dc_array_add_id(contacts, 1 as uint32_t);
    }
    let cnt = dc_array_get_cnt(contacts);
    for i in 0..cnt {
        let contact_id = dc_array_get_id(contacts, i as size_t);
        let line;
        let mut line2 = "".to_string();
        contact = dc_get_contact(context, contact_id);
        if !contact.is_null() {
            let name: *mut libc::c_char = dc_contact_get_name(contact);
            let addr: *mut libc::c_char = dc_contact_get_addr(contact);
            let verified_state: libc::c_int = dc_contact_is_verified(contact);
            let verified_str = if 0 != verified_state {
                if verified_state == 2 {
                    " √√"
                } else {
                    " √"
                }
            } else {
                ""
            };
            line = format!(
                "{}{} <{}>",
                if !name.is_null() && 0 != *name.offset(0isize) as libc::c_int {
                    as_str(name)
                } else {
                    "<name unset>"
                },
                verified_str,
                if !addr.is_null() && 0 != *addr.offset(0isize) as libc::c_int {
                    as_str(addr)
                } else {
                    "addr unset"
                }
            );
            let peerstate = Peerstate::from_addr(context, &context.sql, as_str(addr));
            if peerstate.is_some() && contact_id != 1 as libc::c_uint {
                line2 = format!(
                    ", prefer-encrypt={}",
                    peerstate.as_ref().unwrap().prefer_encrypt
                );
            }
            dc_contact_unref(contact);
            free(name as *mut libc::c_void);
            free(addr as *mut libc::c_void);
            info!(context, 0, "Contact#{}: {}{}", contact_id, line, line2);
        }
    }
}

static mut S_IS_AUTH: libc::c_int = 0;

pub unsafe fn dc_cmdline_skip_auth() {
    S_IS_AUTH = 1;
}

unsafe fn chat_prefix(chat: *const Chat) -> &'static str {
    if (*chat).type_0 == 120 {
        "Group"
    } else if (*chat).type_0 == 130 {
        "VerifiedGroup"
    } else {
        "Single"
    }
}

pub unsafe fn dc_cmdline(context: &Context, line: &str) -> Result<(), failure::Error> {
    let chat_id = *context.cmdline_sel_chat_id.read().unwrap();
    let mut sel_chat = if chat_id > 0 {
        dc_get_chat(context, chat_id)
    } else {
        std::ptr::null_mut()
    };

    let mut args = line.splitn(3, ' ');
    let arg0 = args.next().unwrap_or_default();
    let arg1 = args.next().unwrap_or_default();
    let arg1_c = if arg1.is_empty() {
        std::ptr::null()
    } else {
        to_cstring(arg1) as *const _
    };
    let arg2 = args.next().unwrap_or_default();
    let arg2_c = if arg2.is_empty() {
        std::ptr::null()
    } else {
        to_cstring(arg2) as *const _
    };

    match arg0 {
        "help" | "?" => match arg1 {
            // TODO: reuse commands definition in main.rs.
            "imex" => println!(
                "====================Import/Export commands==\n\
                 initiate-key-transfer\n\
                 get-setupcodebegin <msg-id>\n\
                 continue-key-transfer <msg-id> <setup-code>\n\
                 has-backup\n\
                 export-backup\n\
                 import-backup <backup-file>\n\
                 export-keys\n\
                 import-keys\n\
                 export-setup\n\
                 poke [<eml-file>|<folder>|<addr> <key-file>]\n\
                 reset <flags>\n\
                 stop\n\
                 ============================================="
            ),
            _ => println!(
                "==========================Database commands==\n\
                 info\n\
                 open <file to open or create>\n\
                 close\n\
                 set <configuration-key> [<value>]\n\
                 get <configuration-key>\n\
                 oauth2\n\
                 configure\n\
                 connect\n\
                 disconnect\n\
                 maybenetwork\n\
                 housekeeping\n\
                 help imex (Import/Export)\n\
                 ==============================Chat commands==\n\
                 listchats [<query>]\n\
                 listarchived\n\
                 chat [<chat-id>|0]\n\
                 createchat <contact-id>\n\
                 createchatbymsg <msg-id>\n\
                 creategroup <name>\n\
                 createverified <name>\n\
                 addmember <contact-id>\n\
                 removemember <contact-id>\n\
                 groupname <name>\n\
                 groupimage [<file>]\n\
                 chatinfo\n\
                 sendlocations <seconds>\n\
                 setlocation <lat> <lng>\n\
                 dellocations\n\
                 getlocations [<contact-id>]\n\
                 send <text>\n\
                 sendimage <file> [<text>]\n\
                 sendfile <file>\n\
                 draft [<text>]\n\
                 listmedia\n\
                 archive <chat-id>\n\
                 unarchive <chat-id>\n\
                 delchat <chat-id>\n\
                 ===========================Message commands==\n\
                 listmsgs <query>\n\
                 msginfo <msg-id>\n\
                 listfresh\n\
                 forward <msg-id> <chat-id>\n\
                 markseen <msg-id>\n\
                 star <msg-id>\n\
                 unstar <msg-id>\n\
                 delmsg <msg-id>\n\
                 ===========================Contact commands==\n\
                 listcontacts [<query>]\n\
                 listverified [<query>]\n\
                 addcontact [<name>] <addr>\n\
                 contactinfo <contact-id>\n\
                 delcontact <contact-id>\n\
                 cleanupcontacts\n\
                 ======================================Misc.==\n\
                 getqr [<chat-id>]\n\
                 getbadqr\n\
                 checkqr <qr-content>\n\
                 event <event-id to test>\n\
                 fileinfo <file>\n\
                 clear -- clear screen\n\
                 exit\n\
                 ============================================="
            ),
        },
        "auth" => {
            if 0 == S_IS_AUTH {
                let is_pw = context
                    .get_config(config::Config::MailPw)
                    .unwrap_or_default();
                if arg1 == is_pw {
                    S_IS_AUTH = 1;
                } else {
                    println!("Bad password.");
                }
            } else {
                println!("Already authorized.");
            }
        }
        "open" => {
            ensure!(!arg1.is_empty(), "Argument <file> missing");
            dc_close(context);
            ensure!(
                0 != dc_open(context, arg1_c, 0 as *const libc::c_char),
                "Open failed"
            );
        }
        "close" => {
            dc_close(context);
        }
        "initiate-key-transfer" => {
            let setup_code = dc_initiate_key_transfer(context);
            if !setup_code.is_null() {
                println!(
                    "Setup code for the transferred setup message: {}",
                    as_str(setup_code),
                );
                free(setup_code as *mut libc::c_void);
            } else {
                bail!("Failed to generate setup code");
            };
        }
        "get-setupcodebegin" => {
            ensure!(!arg1.is_empty(), "Argument <msg-id> missing.");
            let msg_id: u32 = arg1.parse().unwrap();
            let msg: *mut dc_msg_t = dc_get_msg(context, msg_id);
            if dc_msg_is_setupmessage(msg) {
                let setupcodebegin = dc_msg_get_setupcodebegin(msg);
                println!(
                    "The setup code for setup message Msg#{} starts with: {}",
                    msg_id,
                    as_str(setupcodebegin),
                );
                free(setupcodebegin as *mut libc::c_void);
            } else {
                bail!("Msg#{} is no setup message.", msg_id,);
            }
            dc_msg_unref(msg);
        }
        "continue-key-transfer" => {
            ensure!(
                !arg1.is_empty() && !arg2.is_empty(),
                "Arguments <msg-id> <setup-code> expected"
            );
            if 0 == dc_continue_key_transfer(context, arg1.parse().unwrap(), arg2_c) {
                bail!("Continue key transfer failed");
            }
        }
        "has-backup" => {
            let ret = dc_imex_has_backup(context, context.get_blobdir());
            if ret.is_null() {
                println!("No backup found.");
            }
        }
        "export-backup" => {
            dc_imex(context, 11, context.get_blobdir(), 0 as *const libc::c_char);
        }
        "import-backup" => {
            ensure!(!arg1.is_empty(), "Argument <backup-file> missing.");
            dc_imex(context, 12, arg1_c, 0 as *const libc::c_char);
        }
        "export-keys" => {
            dc_imex(context, 1, context.get_blobdir(), 0 as *const libc::c_char);
        }
        "import-keys" => {
            dc_imex(context, 2, context.get_blobdir(), 0 as *const libc::c_char);
        }
        "export-setup" => {
            let setup_code: *mut libc::c_char = dc_create_setup_code(context);
            let file_name: *mut libc::c_char = dc_mprintf(
                b"%s/autocrypt-setup-message.html\x00" as *const u8 as *const libc::c_char,
                context.get_blobdir(),
            );
            let file_content: *mut libc::c_char;
            file_content = dc_render_setup_file(context, setup_code);
            if !file_content.is_null()
                && 0 != dc_write_file(
                    context,
                    file_name,
                    file_content as *const libc::c_void,
                    strlen(file_content),
                )
            {
                println!(
                    "Setup message written to: {}\nSetup code: {}",
                    as_str(file_name),
                    as_str(setup_code),
                )
            } else {
                bail!("");
            }
            free(file_content as *mut libc::c_void);
            free(file_name as *mut libc::c_void);
            free(setup_code as *mut libc::c_void);
        }
        "poke" => {
            ensure!(0 != poke_spec(context, arg1_c), "Poke failed");
        }
        "reset" => {
            ensure!(!arg1.is_empty(), "Argument <bits> missing: 1=jobs, 2=peerstates, 4=private keys, 8=rest but server config");
            let bits: i32 = arg1.parse().unwrap();
            ensure!(bits < 16, "<bits> must be lower than 16.");
            ensure!(0 != dc_reset_tables(context, bits), "Reset failed");
        }
        "stop" => {
            dc_stop_ongoing_process(context);
        }
        "set" => {
            ensure!(!arg1.is_empty(), "Argument <key> missing.");
            let key = config::Config::from_str(&arg1)?;
            let value = if arg2.is_empty() { None } else { Some(arg2) };
            context.set_config(key, value)?;
        }
        "get" => {
            ensure!(!arg1.is_empty(), "Argument <key> missing.");
            let key = config::Config::from_str(&arg1)?;
            let val = context.get_config(key);
            println!("{}={:?}", key, val);
        }
        "info" => {
            println!("{}", to_string(dc_get_info(context)));
        }
        "maybenetwork" => {
            dc_maybe_network(context);
        }
        "housekeeping" => {
            sql::housekeeping(context);
        }
        "listchats" | "listarchived" | "chats" => {
            let listflags = if arg0 == "listarchived" { 0x01 } else { 0 };
            let chatlist = dc_get_chatlist(context, listflags, arg1_c, 0 as uint32_t);
            ensure!(!chatlist.is_null(), "Failed to retrieve chatlist");

            let mut i: libc::c_int;
            let cnt = dc_chatlist_get_cnt(chatlist) as libc::c_int;
            if cnt > 0 {
                info!(
                    context, 0,
                    "================================================================================"
                );

                i = cnt - 1;

                while i >= 0 {
                    let chat = dc_get_chat(context, dc_chatlist_get_chat_id(chatlist, i as size_t));
                    let temp_subtitle = dc_chat_get_subtitle(chat);
                    let temp_name = dc_chat_get_name(chat);
                    info!(
                        context,
                        0,
                        "{}#{}: {} [{}] [{} fresh]",
                        chat_prefix(chat),
                        dc_chat_get_id(chat) as libc::c_int,
                        as_str(temp_name),
                        as_str(temp_subtitle),
                        dc_get_fresh_msg_cnt(context, dc_chat_get_id(chat)) as libc::c_int,
                    );
                    free(temp_subtitle as *mut libc::c_void);
                    free(temp_name as *mut libc::c_void);
                    let lot = dc_chatlist_get_summary(chatlist, i as size_t, chat);
                    let statestr = if 0 != dc_chat_get_archived(chat) {
                        " [Archived]"
                    } else {
                        match dc_lot_get_state(lot) {
                            20 => " o",
                            26 => " √",
                            28 => " √√",
                            24 => " !!",
                            _ => "",
                        }
                    };
                    let timestr = dc_timestamp_to_str(dc_lot_get_timestamp(lot));
                    let text1 = dc_lot_get_text1(lot);
                    let text2 = dc_lot_get_text2(lot);
                    info!(
                        context,
                        0,
                        "{}{}{}{} [{}]{}",
                        to_string(text1),
                        if !text1.is_null() { ": " } else { "" },
                        to_string(text2),
                        statestr,
                        as_str(timestr),
                        if 0 != dc_chat_is_sending_locations(chat) {
                            "📍"
                        } else {
                            ""
                        },
                    );
                    free(text1 as *mut libc::c_void);
                    free(text2 as *mut libc::c_void);
                    free(timestr as *mut libc::c_void);
                    dc_lot_unref(lot);
                    dc_chat_unref(chat);
                    info!(
                        context, 0,
                        "================================================================================"
                    );

                    i -= 1
                }
            }
            if dc_is_sending_locations_to_chat(context, 0 as uint32_t) {
                info!(context, 0, "Location streaming enabled.");
            }
            println!("{} chats", cnt);
            dc_chatlist_unref(chatlist);
        }
        "chat" => {
            if sel_chat.is_null() && arg1.is_empty() {
                bail!("Argument [chat-id] is missing.");
            }
            if !sel_chat.is_null() && !arg1.is_empty() {
                dc_chat_unref(sel_chat);
            }
            if !arg1.is_empty() {
                let chat_id = arg1.parse().unwrap();
                println!("Selecting chat #{}", chat_id);
                sel_chat = dc_get_chat(context, chat_id);
                *context.cmdline_sel_chat_id.write().unwrap() = chat_id;
            }

            ensure!(!sel_chat.is_null(), "Failed to select chat");

            let msglist = dc_get_chat_msgs(context, dc_chat_get_id(sel_chat), 0x1, 0);
            let temp2 = dc_chat_get_subtitle(sel_chat);
            let temp_name = dc_chat_get_name(sel_chat);
            info!(
                context,
                0,
                "{}#{}: {} [{}]{}",
                chat_prefix(sel_chat),
                dc_chat_get_id(sel_chat),
                as_str(temp_name),
                as_str(temp2),
                if 0 != dc_chat_is_sending_locations(sel_chat) {
                    "📍"
                } else {
                    ""
                },
            );
            free(temp_name as *mut libc::c_void);
            free(temp2 as *mut libc::c_void);
            if !msglist.is_null() {
                log_msglist(context, msglist);
                dc_array_unref(msglist);
            }
            let draft = dc_get_draft(context, dc_chat_get_id(sel_chat));
            if !draft.is_null() {
                log_msg(context, "Draft", draft);
                dc_msg_unref(draft);
            }
            println!(
                "{} messages.",
                dc_get_msg_cnt(context, dc_chat_get_id(sel_chat))
            );
            dc_marknoticed_chat(context, dc_chat_get_id(sel_chat));
        }
        "createchat" => {
            ensure!(!arg1.is_empty(), "Argument <contact-id> missing.");
            let contact_id: libc::c_int = arg1.parse().unwrap();
            let chat_id: libc::c_int =
                dc_create_chat_by_contact_id(context, contact_id as uint32_t) as libc::c_int;
            if chat_id != 0 {
                println!("Single#{} created successfully.", chat_id,);
            } else {
                bail!("Failed to create chat");
            }
        }
        "createchatbymsg" => {
            ensure!(!arg1.is_empty(), "Argument <msg-id> missing");
            let msg_id_0: libc::c_int = arg1.parse().unwrap();
            let chat_id_0: libc::c_int =
                dc_create_chat_by_msg_id(context, msg_id_0 as uint32_t) as libc::c_int;
            if chat_id_0 != 0 {
                let chat_0: *mut Chat = dc_get_chat(context, chat_id_0 as uint32_t);
                println!(
                    "{}#{} created successfully.",
                    chat_prefix(chat_0),
                    chat_id_0,
                );
                dc_chat_unref(chat_0);
            } else {
                bail!("");
            }
        }
        "creategroup" => {
            ensure!(!arg1.is_empty(), "Argument <name> missing.");
            let chat_id_1: libc::c_int = dc_create_group_chat(context, 0, arg1_c) as libc::c_int;
            if chat_id_1 != 0 {
                println!("Group#{} created successfully.", chat_id_1,);
            } else {
                bail!("Failed to create group");
            }
        }
        "createverified" => {
            ensure!(!arg1.is_empty(), "Argument <name> missing.");
            let chat_id_2: libc::c_int = dc_create_group_chat(context, 1, arg1_c) as libc::c_int;
            if chat_id_2 != 0 {
                println!("VerifiedGroup#{} created successfully.", chat_id_2,);
            } else {
                bail!("Failed to create verified group");
            }
        }
        "addmember" => {
            ensure!(!sel_chat.is_null(), "No chat selected");
            ensure!(!arg1.is_empty(), "Argument <contact-id> missing.");

            let contact_id_0: libc::c_int = arg1.parse().unwrap();
            if 0 != dc_add_contact_to_chat(
                context,
                dc_chat_get_id(sel_chat),
                contact_id_0 as uint32_t,
            ) {
                println!("Contact added to chat.");
            } else {
                bail!("Cannot add contact to chat.");
            }
        }
        "removemember" => {
            ensure!(!sel_chat.is_null(), "No chat selected.");
            ensure!(!arg1.is_empty(), "Argument <contact-id> missing.");
            let contact_id_1: libc::c_int = arg1.parse().unwrap();
            if 0 != dc_remove_contact_from_chat(
                context,
                dc_chat_get_id(sel_chat),
                contact_id_1 as uint32_t,
            ) {
                println!("Contact added to chat.");
            } else {
                bail!("Cannot remove member from chat.");
            }
        }
        "groupname" => {
            ensure!(!sel_chat.is_null(), "No chat selected.");
            ensure!(!arg1.is_empty(), "Argument <name> missing.");
            if 0 != dc_set_chat_name(context, dc_chat_get_id(sel_chat), arg1_c) {
                println!("Chat name set");
            } else {
                bail!("Failed to set chat name");
            }
        }
        "groupimage" => {
            ensure!(!sel_chat.is_null(), "No chat selected.");
            ensure!(!arg1.is_empty(), "Argument <image> missing.");

            if 0 != dc_set_chat_profile_image(
                context,
                dc_chat_get_id(sel_chat),
                if !arg1.is_empty() {
                    arg1_c
                } else {
                    std::ptr::null_mut()
                },
            ) {
                println!("Chat image set");
            } else {
                bail!("Failed to set chat image");
            }
        }
        "chatinfo" => {
            ensure!(!sel_chat.is_null(), "No chat selected.");

            let contacts = dc_get_chat_contacts(context, dc_chat_get_id(sel_chat));
            ensure!(!contacts.is_null(), "Failed to retreive contacts");
            info!(context, 0, "Memberlist:");

            log_contactlist(context, contacts);
            println!(
                "{} contacts\nLocation streaming: {}",
                dc_array_get_cnt(contacts),
                dc_is_sending_locations_to_chat(context, dc_chat_get_id(sel_chat)),
            );
            dc_array_unref(contacts);
        }
        "getlocations" => {
            let contact_id = arg1.parse().unwrap_or_default();
            let loc = dc_get_locations(context, dc_chat_get_id(sel_chat), contact_id, 0, 0);
            let mut j = 0;
            while j < dc_array_get_cnt(loc) {
                let timestr_0 = dc_timestamp_to_str(dc_array_get_timestamp(loc, j as size_t));
                let marker = dc_array_get_marker(loc, j as size_t);
                info!(
                    context,
                    0,
                    "Loc#{}: {}: lat={} lng={} acc={} Chat#{} Contact#{} Msg#{} {}",
                    dc_array_get_id(loc, j as size_t),
                    as_str(timestr_0),
                    dc_array_get_latitude(loc, j as size_t),
                    dc_array_get_longitude(loc, j as size_t),
                    dc_array_get_accuracy(loc, j as size_t),
                    dc_array_get_chat_id(loc, j as size_t),
                    dc_array_get_contact_id(loc, j as size_t),
                    dc_array_get_msg_id(loc, j as size_t),
                    if !marker.is_null() {
                        as_str(marker)
                    } else {
                        "-"
                    },
                );
                free(timestr_0 as *mut libc::c_void);
                free(marker as *mut libc::c_void);
                j += 1
            }
            if dc_array_get_cnt(loc) == 0 {
                info!(context, 0, "No locations.");
            }
            dc_array_unref(loc);
        }
        "sendlocations" => {
            ensure!(!sel_chat.is_null(), "No chat selected.");
            ensure!(!arg1.is_empty(), "No timeout given.");

            let seconds = arg1.parse().unwrap();
            dc_send_locations_to_chat(context, dc_chat_get_id(sel_chat), seconds);
            println!("Locations will be sent to Chat#{} for {} seconds. Use 'setlocation <lat> <lng>' to play around.", dc_chat_get_id(sel_chat), seconds);
        }
        "setlocation" => {
            ensure!(
                !arg1.is_empty() && !arg2.is_empty(),
                "Latitude or longitude not given."
            );
            let latitude = arg1.parse().unwrap();
            let longitude = arg2.parse().unwrap();

            let continue_streaming = dc_set_location(context, latitude, longitude, 0.);
            if 0 != continue_streaming {
                println!("Success, streaming should be continued.");
            } else {
                println!("Success, streaming can be stoppped.");
            }
        }
        "dellocations" => {
            dc_delete_all_locations(context);
        }
        "send" => {
            ensure!(!sel_chat.is_null(), "No chat selected.");
            ensure!(!arg1.is_empty(), "No message text given.");

            let msg = to_cstring(format!("{} {}", arg1, arg2));

            if 0 != dc_send_text_msg(context, dc_chat_get_id(sel_chat), msg) {
                println!("Message sent.");
                free(msg as *mut _);
            } else {
                free(msg as *mut _);
                bail!("Sending failed.");
            }
        }
        "sendempty" => {
            ensure!(!sel_chat.is_null(), "No chat selected.");
            if 0 != dc_send_text_msg(
                context,
                dc_chat_get_id(sel_chat),
                b"\x00" as *const u8 as *const libc::c_char,
            ) {
                println!("Message sent.");
            } else {
                bail!("Sending failed.");
            }
        }
        "sendimage" | "sendfile" => {
            ensure!(!sel_chat.is_null(), "No chat selected.");
            ensure!(!arg1.is_empty() && !arg2.is_empty(), "No file given.");

            let msg_0 = dc_msg_new(context, if arg0 == "sendimage" { 20 } else { 60 });
            dc_msg_set_file(msg_0, arg1_c, 0 as *const libc::c_char);
            dc_msg_set_text(msg_0, arg2_c);
            dc_send_msg(context, dc_chat_get_id(sel_chat), msg_0);
            dc_msg_unref(msg_0);
        }
        "listmsgs" => {
            ensure!(!arg1.is_empty(), "Argument <query> missing.");

            let chat = if !sel_chat.is_null() {
                dc_chat_get_id(sel_chat)
            } else {
                0 as libc::c_uint
            };

            let msglist_0 = dc_search_msgs(context, chat, arg1_c);

            if !msglist_0.is_null() {
                log_msglist(context, msglist_0);
                println!("{} messages.", dc_array_get_cnt(msglist_0));
                dc_array_unref(msglist_0);
            }
        }
        "draft" => {
            ensure!(!sel_chat.is_null(), "No chat selected.");

            if !arg1.is_empty() {
                let draft_0 = dc_msg_new(context, 10);
                dc_msg_set_text(draft_0, arg1_c);
                dc_set_draft(context, dc_chat_get_id(sel_chat), draft_0);
                dc_msg_unref(draft_0);
                println!("Draft saved.");
            } else {
                dc_set_draft(context, dc_chat_get_id(sel_chat), 0 as *mut dc_msg_t);
                println!("Draft deleted.");
            }
        }
        "listmedia" => {
            ensure!(!sel_chat.is_null(), "No chat selected.");

            let images = dc_get_chat_media(context, dc_chat_get_id(sel_chat), 20, 21, 50);
            let icnt: libc::c_int = dc_array_get_cnt(images) as libc::c_int;
            println!("{} images or videos: ", icnt);
            for i in 0..icnt {
                let data = dc_array_get_id(images, i as size_t);
                if 0 == i {
                    print!("Msg#{}", data);
                } else {
                    print!(", Msg#{}", data);
                }
            }
            print!("\n");
            dc_array_unref(images);
        }
        "archive" | "unarchive" => {
            ensure!(!arg1.is_empty(), "Argument <chat-id> missing.");
            let chat_id = arg1.parse().unwrap();
            dc_archive_chat(context, chat_id, if arg0 == "archive" { 1 } else { 0 });
        }
        "delchat" => {
            ensure!(!arg1.is_empty(), "Argument <chat-id> missing.");
            let chat_id = arg1.parse().unwrap();
            dc_delete_chat(context, chat_id);
        }
        "msginfo" => {
            ensure!(!arg1.is_empty(), "Argument <msg-id> missing.");
            let id = arg1.parse().unwrap();
            let res = dc_get_msg_info(context, id);
            println!("{}", as_str(res));
        }
        "listfresh" => {
            let msglist = dc_get_fresh_msgs(context);
            ensure!(!msglist.is_null(), "Failed to retrieve messages");

            log_msglist(context, msglist);
            print!("{} fresh messages.", dc_array_get_cnt(msglist));
            dc_array_unref(msglist);
        }
        "forward" => {
            ensure!(
                !arg1.is_empty() && arg2.is_empty(),
                "Arguments <msg-id> <chat-id> expected"
            );

            let mut msg_ids = [0; 1];
            let chat_id = arg2.parse().unwrap();
            msg_ids[0] = arg1.parse().unwrap();
            dc_forward_msgs(context, msg_ids.as_mut_ptr(), 1, chat_id);
        }
        "markseen" => {
            ensure!(!arg1.is_empty(), "Argument <msg-id> missing.");
            let mut msg_ids = [0; 1];
            msg_ids[0] = arg1.parse().unwrap();
            dc_markseen_msgs(context, msg_ids.as_mut_ptr(), 1);
        }
        "star" | "unstar" => {
            ensure!(!arg1.is_empty(), "Argument <msg-id> missing.");
            let mut msg_ids = [0; 1];
            msg_ids[0] = arg1.parse().unwrap();
            dc_star_msgs(
                context,
                msg_ids.as_mut_ptr(),
                1,
                if arg0 == "star" { 1 } else { 0 },
            );
        }
        "delmsg" => {
            ensure!(!arg1.is_empty(), "Argument <msg-id> missing.");
            let mut ids = [0; 1];
            ids[0] = arg1.parse().unwrap();
            dc_delete_msgs(context, ids.as_mut_ptr(), 1);
        }
        "listcontacts" | "contacts" | "listverified" => {
            let contacts = dc_get_contacts(
                context,
                if arg0 == "listverified" {
                    0x1 | 0x2
                } else {
                    0x2
                },
                arg1_c,
            );
            if !contacts.is_null() {
                log_contactlist(context, contacts);
                println!("{} contacts.", dc_array_get_cnt(contacts) as libc::c_int,);
                dc_array_unref(contacts);
            } else {
                bail!("");
            }
        }
        "addcontact" => {
            ensure!(!arg1.is_empty(), "Arguments [<name>] <addr> expected.");

            if !arg2.is_empty() {
                let book = dc_mprintf(
                    b"%s\n%s\x00" as *const u8 as *const libc::c_char,
                    arg1_c,
                    arg2_c,
                );
                dc_add_address_book(context, book);
                free(book as *mut libc::c_void);
            } else {
                if 0 == dc_create_contact(context, 0 as *const libc::c_char, arg1_c) {
                    bail!("Failed to create contact");
                }
            }
        }
        "contactinfo" => {
            ensure!(!arg1.is_empty(), "Argument <contact-id> missing.");

            let contact_id = arg1.parse().unwrap();
            let contact = dc_get_contact(context, contact_id);
            let name_n_addr = dc_contact_get_name_n_addr(contact);

            let mut res = format!("Contact info for: {}:\n\n", as_str(name_n_addr),);
            free(name_n_addr as *mut libc::c_void);
            dc_contact_unref(contact);

            let encrinfo = dc_get_contact_encrinfo(context, contact_id);
            res += as_str(encrinfo);
            free(encrinfo as *mut libc::c_void);

            let chatlist = dc_get_chatlist(context, 0, 0 as *const libc::c_char, contact_id);
            let chatlist_cnt = dc_chatlist_get_cnt(chatlist) as libc::c_int;
            if chatlist_cnt > 0 {
                res += &format!(
                    "\n\n{} chats shared with Contact#{}: ",
                    chatlist_cnt, contact_id,
                );
                for i in 0..chatlist_cnt {
                    if 0 != i {
                        res += ", ";
                    }
                    let chat = dc_get_chat(context, dc_chatlist_get_chat_id(chatlist, i as size_t));
                    res += &format!("{}#{}", chat_prefix(chat), dc_chat_get_id(chat));
                    dc_chat_unref(chat);
                }
            }
            dc_chatlist_unref(chatlist);
            println!("{}", res);
        }
        "delcontact" => {
            ensure!(!arg1.is_empty(), "Argument <contact-id> missing.");
            if !dc_delete_contact(context, arg1.parse().unwrap()) {
                bail!("Failed to delete contact");
            }
        }
        "checkqr" => {
            ensure!(!arg1.is_empty(), "Argument <qr-content> missing.");
            let res = dc_check_qr(context, arg1_c);
            println!(
                "state={}, id={}, text1={}, text2={}",
                (*res).state as libc::c_int,
                (*res).id,
                to_string((*res).text1),
                to_string((*res).text2)
            );
            dc_lot_unref(res);
        }
        "event" => {
            ensure!(!arg1.is_empty(), "Argument <id> missing.");
            let event = Event::from_u32(arg1.parse().unwrap()).unwrap();
            let r = context.call_cb(event, 0 as uintptr_t, 0 as uintptr_t);
            println!(
                "Sending event {:?}({}), received value {}.",
                event, event as usize, r as libc::c_int,
            );
        }
        "fileinfo" => {
            ensure!(!arg1.is_empty(), "Argument <file> missing.");
            let mut buf = 0 as *mut libc::c_uchar;
            let mut buf_bytes = 0;
            let mut w = 0;
            let mut h = 0;

            if 0 != dc_read_file(
                context,
                arg1_c,
                &mut buf as *mut *mut libc::c_uchar as *mut *mut libc::c_void,
                &mut buf_bytes,
            ) {
                dc_get_filemeta(buf as *const libc::c_void, buf_bytes, &mut w, &mut h);
                println!("width={}, height={}", w, h,);
                free(buf as *mut libc::c_void);
            } else {
                bail!("Command failed.");
            }
        }
        "" => (),
        _ => bail!("Unknown command: \"{}\" type ? for help.", arg0),
    }

    if !sel_chat.is_null() {
        dc_chat_unref(sel_chat);
    }

    free(arg1_c as *mut _);
    free(arg2_c as *mut _);

    Ok(())
}
