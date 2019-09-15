use std::ffi::CString;
use std::ptr;
use std::str::FromStr;

use deltachat::chat::{self, Chat};
use deltachat::chatlist::*;
use deltachat::config;
use deltachat::configure::*;
use deltachat::constants::*;
use deltachat::contact::*;
use deltachat::context::*;
use deltachat::dc_imex::*;
use deltachat::dc_receive_imf::*;
use deltachat::dc_tools::*;
use deltachat::error::Error;
use deltachat::job::*;
use deltachat::location;
use deltachat::lot::LotState;
use deltachat::message::*;
use deltachat::peerstate::*;
use deltachat::qr::*;
use deltachat::sql;
use deltachat::x::*;
use deltachat::Event;

/// Reset database tables. This function is called from Core cmdline.
/// Argument is a bitmask, executing single or multiple actions in one call.
/// e.g. bitmask 7 triggers actions definded with bits 1, 2 and 4.
pub unsafe fn dc_reset_tables(context: &Context, bits: i32) -> i32 {
    info!(context, "Resetting tables ({})...", bits);
    if 0 != bits & 1 {
        sql::execute(context, &context.sql, "DELETE FROM jobs;", params![]).unwrap();
        info!(context, "(1) Jobs reset.");
    }
    if 0 != bits & 2 {
        sql::execute(
            context,
            &context.sql,
            "DELETE FROM acpeerstates;",
            params![],
        )
        .unwrap();
        info!(context, "(2) Peerstates reset.");
    }
    if 0 != bits & 4 {
        sql::execute(context, &context.sql, "DELETE FROM keypairs;", params![]).unwrap();
        info!(context, "(4) Private keypairs reset.");
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
        info!(context, "(8) Rest but server config reset.");
    }

    context.call_cb(Event::MsgsChanged {
        chat_id: 0,
        msg_id: 0,
    });

    1
}

unsafe fn dc_poke_eml_file(context: &Context, filename: *const libc::c_char) -> libc::c_int {
    /* mainly for testing, may be called by dc_import_spec() */
    let mut success: libc::c_int = 0i32;
    let mut data: *mut libc::c_char = ptr::null_mut();
    let mut data_bytes = 0;
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
        error!(context, "Import: Database not opened.");
        return 0;
    }

    let ok_to_continue;
    let mut success: libc::c_int = 0;
    let real_spec: *mut libc::c_char;
    let mut suffix: *mut libc::c_char = ptr::null_mut();
    let mut read_cnt: libc::c_int = 0;

    /* if `spec` is given, remember it for later usage; if it is not given, try to use the last one */
    if !spec.is_null() {
        real_spec = dc_strdup(spec);
        context
            .sql
            .set_config(context, "import_spec", Some(as_str(real_spec)))
            .unwrap();
        ok_to_continue = true;
    } else {
        let rs = context.sql.get_config(context, "import_spec");
        if rs.is_none() {
            error!(context, "Import: No file or folder given.");
            ok_to_continue = false;
        } else {
            ok_to_continue = true;
        }
        real_spec = rs.unwrap_or_default().strdup();
    }
    if ok_to_continue {
        let ok_to_continue2;
        suffix = dc_get_filesuffix_lc(as_str(real_spec));
        if !suffix.is_null() && strcmp(suffix, b"eml\x00" as *const u8 as *const libc::c_char) == 0
        {
            if 0 != dc_poke_eml_file(context, real_spec) {
                read_cnt += 1
            }
            ok_to_continue2 = true;
        } else {
            /* import a directory */
            let dir_name = std::path::Path::new(as_str(real_spec));
            let dir = std::fs::read_dir(dir_name);
            if dir.is_err() {
                error!(
                    context,
                    "Import: Cannot open directory \"{}\".",
                    as_str(real_spec),
                );
                ok_to_continue2 = false;
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
                        info!(context, "Import: {}", path_plus_name);
                        let path_plus_name_c = CString::yolo(path_plus_name);
                        if 0 != dc_poke_eml_file(context, path_plus_name_c.as_ptr()) {
                            read_cnt += 1
                        }
                    }
                }
                ok_to_continue2 = true;
            }
        }
        if ok_to_continue2 {
            info!(
                context,
                "Import: {} items read from \"{}\".",
                read_cnt,
                as_str(real_spec)
            );
            if read_cnt > 0 {
                context.call_cb(Event::MsgsChanged {
                    chat_id: 0,
                    msg_id: 0,
                });
            }
            success = 1
        }
    }

    free(real_spec as *mut libc::c_void);
    free(suffix as *mut libc::c_void);
    success
}

unsafe fn log_msg(context: &Context, prefix: impl AsRef<str>, msg: &Message) {
    let contact = Contact::get_by_id(context, dc_msg_get_from_id(msg)).expect("invalid contact");
    let contact_name = contact.get_name();
    let contact_id = contact.get_id();

    let statestr = match dc_msg_get_state(msg) {
        MessageState::OutPending => " o",
        MessageState::OutDelivered => " √",
        MessageState::OutMdnRcvd => " √√",
        MessageState::OutFailed => " !!",
        _ => "",
    };
    let temp2 = dc_timestamp_to_str(dc_msg_get_timestamp(msg));
    let msgtext = dc_msg_get_text(msg);
    info!(
        context,
        "{}#{}{}{}: {} (Contact#{}): {} {}{}{}{} [{}]",
        prefix.as_ref(),
        dc_msg_get_id(msg) as libc::c_int,
        if dc_msg_get_showpadlock(msg) {
            "🔒"
        } else {
            ""
        },
        if dc_msg_has_location(msg) { "📍" } else { "" },
        &contact_name,
        contact_id,
        as_str(msgtext),
        if dc_msg_is_starred(msg) { "★" } else { "" },
        if dc_msg_get_from_id(msg) == 1 as libc::c_uint {
            ""
        } else if dc_msg_get_state(msg) == MessageState::InSeen {
            "[SEEN]"
        } else if dc_msg_get_state(msg) == MessageState::InNoticed {
            "[NOTICED]"
        } else {
            "[FRESH]"
        },
        if dc_msg_is_info(msg) { "[INFO]" } else { "" },
        statestr,
        &temp2,
    );
    free(msgtext as *mut libc::c_void);
}

unsafe fn log_msglist(context: &Context, msglist: &Vec<u32>) -> Result<(), Error> {
    let mut lines_out = 0;
    for &msg_id in msglist {
        if msg_id == 9 as libc::c_uint {
            info!(
                context,
                "--------------------------------------------------------------------------------"
            );

            lines_out += 1
        } else if msg_id > 0 {
            if lines_out == 0 {
                info!(
                    context,
                    "--------------------------------------------------------------------------------",
                );
                lines_out += 1
            }
            let msg = dc_get_msg(context, msg_id)?;
            log_msg(context, "Msg", &msg);
        }
    }
    if lines_out > 0 {
        info!(
            context,
            "--------------------------------------------------------------------------------"
        );
    }
    Ok(())
}

unsafe fn log_contactlist(context: &Context, contacts: &Vec<u32>) {
    let mut contacts = contacts.clone();
    if !contacts.contains(&1) {
        contacts.push(1);
    }
    for contact_id in contacts {
        let line;
        let mut line2 = "".to_string();
        if let Ok(contact) = Contact::get_by_id(context, contact_id) {
            let name = contact.get_name();
            let addr = contact.get_addr();
            let verified_state = contact.is_verified(context);
            let verified_str = if VerifiedStatus::Unverified != verified_state {
                if verified_state == VerifiedStatus::BidirectVerified {
                    " √√"
                } else {
                    " √"
                }
            } else {
                ""
            };
            line = format!(
                "{}{} <{}>",
                if !name.is_empty() {
                    &name
                } else {
                    "<name unset>"
                },
                verified_str,
                if !addr.is_empty() {
                    &addr
                } else {
                    "addr unset"
                }
            );
            let peerstate = Peerstate::from_addr(context, &context.sql, &addr);
            if peerstate.is_some() && contact_id != 1 as libc::c_uint {
                line2 = format!(
                    ", prefer-encrypt={}",
                    peerstate.as_ref().unwrap().prefer_encrypt
                );
            }

            info!(context, "Contact#{}: {}{}", contact_id, line, line2);
        }
    }
}

fn chat_prefix(chat: &Chat) -> &'static str {
    chat.typ.into()
}

pub unsafe fn dc_cmdline(context: &Context, line: &str) -> Result<(), failure::Error> {
    let chat_id = *context.cmdline_sel_chat_id.read().unwrap();
    let mut sel_chat = if chat_id > 0 {
        Chat::load_from_db(context, chat_id).ok()
    } else {
        None
    };

    let mut args = line.splitn(3, ' ');
    let arg0 = args.next().unwrap_or_default();
    let arg1 = args.next().unwrap_or_default();
    let arg1_c = if arg1.is_empty() {
        std::ptr::null()
    } else {
        arg1.strdup() as *const _
    };
    let arg2 = args.next().unwrap_or_default();
    let arg2_c = if arg2.is_empty() {
        std::ptr::null()
    } else {
        arg2.strdup() as *const _
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
                 send-garbage\n\
                 sendimage <file> [<text>]\n\
                 sendfile <file> [<text>]\n\
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
                 exit or quit\n\
                 ============================================="
            ),
        },
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
            let msg_id: u32 = arg1.parse()?;
            let msg = dc_get_msg(context, msg_id)?;
            if dc_msg_is_setupmessage(&msg) {
                let setupcodebegin = dc_msg_get_setupcodebegin(context, &msg);
                println!(
                    "The setup code for setup message Msg#{} starts with: {}",
                    msg_id,
                    as_str(setupcodebegin),
                );
                free(setupcodebegin as *mut libc::c_void);
            } else {
                bail!("Msg#{} is no setup message.", msg_id,);
            }
        }
        "continue-key-transfer" => {
            ensure!(
                !arg1.is_empty() && !arg2.is_empty(),
                "Arguments <msg-id> <setup-code> expected"
            );
            if !dc_continue_key_transfer(context, arg1.parse()?, arg2_c) {
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
            dc_imex(context, 11, Some(context.get_blobdir()), ptr::null());
        }
        "import-backup" => {
            ensure!(!arg1.is_empty(), "Argument <backup-file> missing.");
            dc_imex(context, 12, Some(arg1), ptr::null());
        }
        "export-keys" => {
            dc_imex(context, 1, Some(context.get_blobdir()), ptr::null());
        }
        "import-keys" => {
            dc_imex(context, 2, Some(context.get_blobdir()), ptr::null());
        }
        "export-setup" => {
            let setup_code = dc_create_setup_code(context);
            let file_name: *mut libc::c_char = dc_mprintf(
                b"%s/autocrypt-setup-message.html\x00" as *const u8 as *const libc::c_char,
                context.get_blobdir(),
            );
            let file_content = dc_render_setup_file(context, &setup_code)?;
            std::fs::write(as_str(file_name), file_content)?;
            println!(
                "Setup message written to: {}\nSetup code: {}",
                as_str(file_name),
                &setup_code,
            );
            free(file_name as *mut libc::c_void);
        }
        "poke" => {
            ensure!(0 != poke_spec(context, arg1_c), "Poke failed");
        }
        "reset" => {
            ensure!(!arg1.is_empty(), "Argument <bits> missing: 1=jobs, 2=peerstates, 4=private keys, 8=rest but server config");
            let bits: i32 = arg1.parse()?;
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
            println!("{:#?}", context.get_info());
        }
        "maybenetwork" => {
            maybe_network(context);
        }
        "housekeeping" => {
            sql::housekeeping(context);
        }
        "listchats" | "listarchived" | "chats" => {
            let listflags = if arg0 == "listarchived" { 0x01 } else { 0 };
            let chatlist = Chatlist::try_load(
                context,
                listflags,
                if arg1.is_empty() { None } else { Some(arg1) },
                None,
            )?;

            let cnt = chatlist.len();
            if cnt > 0 {
                info!(
                    context,
                    "================================================================================"
                );

                for i in (0..cnt).rev() {
                    let chat = Chat::load_from_db(context, chatlist.get_chat_id(i))?;
                    let temp_subtitle = chat.get_subtitle(context);
                    let temp_name = chat.get_name();
                    info!(
                        context,
                        "{}#{}: {} [{}] [{} fresh]",
                        chat_prefix(&chat),
                        chat.get_id(),
                        temp_name,
                        temp_subtitle,
                        chat::get_fresh_msg_cnt(context, chat.get_id()),
                    );
                    let lot = chatlist.get_summary(context, i, Some(&chat));
                    let statestr = if chat.is_archived() {
                        " [Archived]"
                    } else {
                        match lot.get_state() {
                            LotState::MsgOutPending => " o",
                            LotState::MsgOutDelivered => " √",
                            LotState::MsgOutMdnRcvd => " √√",
                            LotState::MsgOutFailed => " !!",
                            _ => "",
                        }
                    };
                    let timestr = dc_timestamp_to_str(lot.get_timestamp());
                    let text1 = lot.get_text1();
                    let text2 = lot.get_text2();
                    info!(
                        context,
                        "{}{}{}{} [{}]{}",
                        text1.unwrap_or(""),
                        if text1.is_some() { ": " } else { "" },
                        text2.unwrap_or(""),
                        statestr,
                        &timestr,
                        if chat.is_sending_locations() {
                            "📍"
                        } else {
                            ""
                        },
                    );
                    info!(
                        context,
                        "================================================================================"
                    );
                }
            }
            if location::is_sending_locations_to_chat(context, 0) {
                info!(context, "Location streaming enabled.");
            }
            println!("{} chats", cnt);
        }
        "chat" => {
            if sel_chat.is_none() && arg1.is_empty() {
                bail!("Argument [chat-id] is missing.");
            }
            if !arg1.is_empty() {
                let chat_id = arg1.parse()?;
                println!("Selecting chat #{}", chat_id);
                sel_chat = Some(Chat::load_from_db(context, chat_id)?);
                *context.cmdline_sel_chat_id.write().unwrap() = chat_id;
            }

            ensure!(sel_chat.is_some(), "Failed to select chat");
            let sel_chat = sel_chat.as_ref().unwrap();

            let msglist = chat::get_chat_msgs(context, sel_chat.get_id(), 0x1, 0);
            let temp2 = sel_chat.get_subtitle(context);
            let temp_name = sel_chat.get_name();
            info!(
                context,
                "{}#{}: {} [{}]{}",
                chat_prefix(sel_chat),
                sel_chat.get_id(),
                temp_name,
                temp2,
                if sel_chat.is_sending_locations() {
                    "📍"
                } else {
                    ""
                },
            );
            log_msglist(context, &msglist)?;
            if let Some(draft) = chat::get_draft(context, sel_chat.get_id())? {
                log_msg(context, "Draft", &draft);
            }

            println!(
                "{} messages.",
                chat::get_msg_cnt(context, sel_chat.get_id())
            );
            chat::marknoticed_chat(context, sel_chat.get_id())?;
        }
        "createchat" => {
            ensure!(!arg1.is_empty(), "Argument <contact-id> missing.");
            let contact_id: libc::c_int = arg1.parse()?;
            let chat_id = chat::create_by_contact_id(context, contact_id as u32)?;

            println!("Single#{} created successfully.", chat_id,);
        }
        "createchatbymsg" => {
            ensure!(!arg1.is_empty(), "Argument <msg-id> missing");
            let msg_id: u32 = arg1.parse()?;
            let chat_id = chat::create_by_msg_id(context, msg_id)?;
            let chat = Chat::load_from_db(context, chat_id)?;

            println!("{}#{} created successfully.", chat_prefix(&chat), chat_id,);
        }
        "creategroup" => {
            ensure!(!arg1.is_empty(), "Argument <name> missing.");
            let chat_id = chat::create_group_chat(context, VerifiedStatus::Unverified, arg1)?;

            println!("Group#{} created successfully.", chat_id);
        }
        "createverified" => {
            ensure!(!arg1.is_empty(), "Argument <name> missing.");
            let chat_id = chat::create_group_chat(context, VerifiedStatus::Verified, arg1)?;

            println!("VerifiedGroup#{} created successfully.", chat_id);
        }
        "addmember" => {
            ensure!(sel_chat.is_some(), "No chat selected");
            ensure!(!arg1.is_empty(), "Argument <contact-id> missing.");

            let contact_id_0: libc::c_int = arg1.parse()?;
            if chat::add_contact_to_chat(
                context,
                sel_chat.as_ref().unwrap().get_id(),
                contact_id_0 as u32,
            ) {
                println!("Contact added to chat.");
            } else {
                bail!("Cannot add contact to chat.");
            }
        }
        "removemember" => {
            ensure!(sel_chat.is_some(), "No chat selected.");
            ensure!(!arg1.is_empty(), "Argument <contact-id> missing.");
            let contact_id_1: libc::c_int = arg1.parse()?;
            chat::remove_contact_from_chat(
                context,
                sel_chat.as_ref().unwrap().get_id(),
                contact_id_1 as u32,
            )?;

            println!("Contact added to chat.");
        }
        "groupname" => {
            ensure!(sel_chat.is_some(), "No chat selected.");
            ensure!(!arg1.is_empty(), "Argument <name> missing.");
            chat::set_chat_name(context, sel_chat.as_ref().unwrap().get_id(), arg1)?;

            println!("Chat name set");
        }
        "groupimage" => {
            ensure!(sel_chat.is_some(), "No chat selected.");
            ensure!(!arg1.is_empty(), "Argument <image> missing.");

            chat::set_chat_profile_image(context, sel_chat.as_ref().unwrap().get_id(), arg1)?;

            println!("Chat image set");
        }
        "chatinfo" => {
            ensure!(sel_chat.is_some(), "No chat selected.");

            let contacts = chat::get_chat_contacts(context, sel_chat.as_ref().unwrap().get_id());
            info!(context, "Memberlist:");

            log_contactlist(context, &contacts);
            println!(
                "{} contacts\nLocation streaming: {}",
                contacts.len(),
                location::is_sending_locations_to_chat(
                    context,
                    sel_chat.as_ref().unwrap().get_id()
                ),
            );
        }
        "getlocations" => {
            ensure!(sel_chat.is_some(), "No chat selected.");

            let contact_id = arg1.parse().unwrap_or_default();
            let locations = location::get_range(
                context,
                sel_chat.as_ref().unwrap().get_id(),
                contact_id,
                0,
                0,
            );
            let default_marker = "-".to_string();
            for location in &locations {
                let marker = location.marker.as_ref().unwrap_or(&default_marker);
                info!(
                    context,
                    "Loc#{}: {}: lat={} lng={} acc={} Chat#{} Contact#{} Msg#{} {}",
                    location.location_id,
                    dc_timestamp_to_str(location.timestamp),
                    location.latitude,
                    location.longitude,
                    location.accuracy,
                    location.chat_id,
                    location.contact_id,
                    location.msg_id,
                    marker
                );
            }
            if locations.is_empty() {
                info!(context, "No locations.");
            }
        }
        "sendlocations" => {
            ensure!(sel_chat.is_some(), "No chat selected.");
            ensure!(!arg1.is_empty(), "No timeout given.");

            let seconds = arg1.parse()?;
            location::send_locations_to_chat(context, sel_chat.as_ref().unwrap().get_id(), seconds);
            println!(
                "Locations will be sent to Chat#{} for {} seconds. Use 'setlocation <lat> <lng>' to play around.",
                sel_chat.as_ref().unwrap().get_id(),
                seconds
            );
        }
        "setlocation" => {
            ensure!(
                !arg1.is_empty() && !arg2.is_empty(),
                "Latitude or longitude not given."
            );
            let latitude = arg1.parse()?;
            let longitude = arg2.parse()?;

            let continue_streaming = location::set(context, latitude, longitude, 0.);
            if 0 != continue_streaming {
                println!("Success, streaming should be continued.");
            } else {
                println!("Success, streaming can be stoppped.");
            }
        }
        "dellocations" => {
            location::delete_all(context)?;
        }
        "send" => {
            ensure!(sel_chat.is_some(), "No chat selected.");
            ensure!(!arg1.is_empty(), "No message text given.");

            let msg = format!("{} {}", arg1, arg2);

            chat::send_text_msg(context, sel_chat.as_ref().unwrap().get_id(), msg)?;
        }
        "sendempty" => {
            ensure!(sel_chat.is_some(), "No chat selected.");
            chat::send_text_msg(context, sel_chat.as_ref().unwrap().get_id(), "".into())?;
        }
        "sendimage" | "sendfile" => {
            ensure!(sel_chat.is_some(), "No chat selected.");
            ensure!(!arg1.is_empty(), "No file given.");

            let mut msg = dc_msg_new(if arg0 == "sendimage" {
                Viewtype::Image
            } else {
                Viewtype::File
            });
            dc_msg_set_file(&mut msg, arg1_c, ptr::null());
            if !arg2.is_empty() {
                dc_msg_set_text(&mut msg, arg2_c);
            }
            chat::send_msg(context, sel_chat.as_ref().unwrap().get_id(), &mut msg)?;
        }
        "listmsgs" => {
            ensure!(!arg1.is_empty(), "Argument <query> missing.");

            let chat = if let Some(ref sel_chat) = sel_chat {
                sel_chat.get_id()
            } else {
                0 as libc::c_uint
            };

            let msglist = context.search_msgs(chat, arg1);

            log_msglist(context, &msglist)?;
            println!("{} messages.", msglist.len());
        }
        "draft" => {
            ensure!(sel_chat.is_some(), "No chat selected.");

            if !arg1.is_empty() {
                let mut draft = dc_msg_new(Viewtype::Text);
                dc_msg_set_text(&mut draft, arg1_c);
                chat::set_draft(
                    context,
                    sel_chat.as_ref().unwrap().get_id(),
                    Some(&mut draft),
                );
                println!("Draft saved.");
            } else {
                chat::set_draft(context, sel_chat.as_ref().unwrap().get_id(), None);
                println!("Draft deleted.");
            }
        }
        "listmedia" => {
            ensure!(sel_chat.is_some(), "No chat selected.");

            let images = chat::get_chat_media(
                context,
                sel_chat.as_ref().unwrap().get_id(),
                Viewtype::Image,
                Viewtype::Gif,
                Viewtype::Video,
            );
            println!("{} images or videos: ", images.len());
            for (i, data) in images.iter().enumerate() {
                if 0 == i {
                    print!("Msg#{}", data);
                } else {
                    print!(", Msg#{}", data);
                }
            }
            print!("\n");
        }
        "archive" | "unarchive" => {
            ensure!(!arg1.is_empty(), "Argument <chat-id> missing.");
            let chat_id = arg1.parse()?;
            chat::archive(
                context,
                chat_id,
                if arg0 == "archive" { true } else { false },
            )?;
        }
        "delchat" => {
            ensure!(!arg1.is_empty(), "Argument <chat-id> missing.");
            let chat_id = arg1.parse()?;
            chat::delete(context, chat_id)?;
        }
        "msginfo" => {
            ensure!(!arg1.is_empty(), "Argument <msg-id> missing.");
            let id = arg1.parse()?;
            let res = dc_get_msg_info(context, id);
            println!("{}", as_str(res));
        }
        "listfresh" => {
            let msglist = context.get_fresh_msgs();

            log_msglist(context, &msglist)?;
            print!("{} fresh messages.", msglist.len());
        }
        "forward" => {
            ensure!(
                !arg1.is_empty() && arg2.is_empty(),
                "Arguments <msg-id> <chat-id> expected"
            );

            let mut msg_ids = [0; 1];
            let chat_id = arg2.parse()?;
            msg_ids[0] = arg1.parse()?;
            chat::forward_msgs(context, msg_ids.as_mut_ptr(), 1, chat_id);
        }
        "markseen" => {
            ensure!(!arg1.is_empty(), "Argument <msg-id> missing.");
            let mut msg_ids = [0; 1];
            msg_ids[0] = arg1.parse()?;
            dc_markseen_msgs(context, msg_ids.as_mut_ptr(), 1);
        }
        "star" | "unstar" => {
            ensure!(!arg1.is_empty(), "Argument <msg-id> missing.");
            let mut msg_ids = [0; 1];
            msg_ids[0] = arg1.parse()?;
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
            ids[0] = arg1.parse()?;
            dc_delete_msgs(context, ids.as_mut_ptr(), 1);
        }
        "listcontacts" | "contacts" | "listverified" => {
            let contacts = Contact::get_all(
                context,
                if arg0 == "listverified" {
                    0x1 | 0x2
                } else {
                    0x2
                },
                Some(arg1),
            )?;
            log_contactlist(context, &contacts);
            println!("{} contacts.", contacts.len());
        }
        "addcontact" => {
            ensure!(!arg1.is_empty(), "Arguments [<name>] <addr> expected.");

            if !arg2.is_empty() {
                let book = format!("{}\n{}", arg1, arg2);
                Contact::add_address_book(context, book)?;
            } else {
                Contact::create(context, "", arg1)?;
            }
        }
        "contactinfo" => {
            ensure!(!arg1.is_empty(), "Argument <contact-id> missing.");

            let contact_id = arg1.parse()?;
            let contact = Contact::get_by_id(context, contact_id)?;
            let name_n_addr = contact.get_name_n_addr();

            let mut res = format!("Contact info for: {}:\n\n", name_n_addr);

            res += &Contact::get_encrinfo(context, contact_id)?;

            let chatlist = Chatlist::try_load(context, 0, None, Some(contact_id))?;
            let chatlist_cnt = chatlist.len();
            if chatlist_cnt > 0 {
                res += &format!(
                    "\n\n{} chats shared with Contact#{}: ",
                    chatlist_cnt, contact_id,
                );
                for i in 0..chatlist_cnt {
                    if 0 != i {
                        res += ", ";
                    }
                    let chat = Chat::load_from_db(context, chatlist.get_chat_id(i))?;
                    res += &format!("{}#{}", chat_prefix(&chat), chat.get_id());
                }
            }

            println!("{}", res);
        }
        "delcontact" => {
            ensure!(!arg1.is_empty(), "Argument <contact-id> missing.");
            Contact::delete(context, arg1.parse()?)?;
        }
        "checkqr" => {
            ensure!(!arg1.is_empty(), "Argument <qr-content> missing.");
            let res = check_qr(context, arg1);
            println!(
                "state={}, id={}, text1={:?}, text2={:?}",
                res.get_state(),
                res.get_id(),
                res.get_text1(),
                res.get_text2()
            );
        }
        // TODO: implement this again, unclear how to match this through though, without writing a parser.
        // "event" => {
        //     ensure!(!arg1.is_empty(), "Argument <id> missing.");
        //     let event = arg1.parse()?;
        //     let event = Event::from_u32(event).ok_or(format_err!("Event::from_u32({})", event))?;
        //     let r = context.call_cb(event, 0 as libc::uintptr_t, 0 as libc::uintptr_t);
        //     println!(
        //         "Sending event {:?}({}), received value {}.",
        //         event, event as usize, r as libc::c_int,
        //     );
        // }
        "fileinfo" => {
            ensure!(!arg1.is_empty(), "Argument <file> missing.");

            if let Some(buf) = dc_read_file_safe(context, &arg1) {
                let (width, height) = dc_get_filemeta(&buf)?;
                println!("width={}, height={}", width, height);
            } else {
                bail!("Command failed.");
            }
        }
        "" => (),
        _ => bail!("Unknown command: \"{}\" type ? for help.", arg0),
    }

    free(arg1_c as *mut _);
    free(arg2_c as *mut _);

    Ok(())
}
