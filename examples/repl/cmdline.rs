extern crate dirs;

use std::str::FromStr;

use anyhow::{bail, ensure, Error};
use async_std::path::Path;
use deltachat::chat::{
    self, Chat, ChatId, ChatItem, ChatVisibility, MuteDuration, ProtectionStatus,
};
use deltachat::chatlist::*;
use deltachat::constants::*;
use deltachat::contact::*;
use deltachat::context::*;
use deltachat::dc_receive_imf::*;
use deltachat::dc_tools::*;
use deltachat::imex::*;
use deltachat::location;
use deltachat::log::LogExt;
use deltachat::lot::LotState;
use deltachat::message::{self, ContactRequestDecision, Message, MessageState, MsgId};
use deltachat::peerstate::*;
use deltachat::qr::*;
use deltachat::sql;
use deltachat::EventType;
use deltachat::{config, provider};
use std::fs;
use std::time::{Duration, SystemTime};

/// Reset database tables.
/// Argument is a bitmask, executing single or multiple actions in one call.
/// e.g. bitmask 7 triggers actions definded with bits 1, 2 and 4.
async fn reset_tables(context: &Context, bits: i32) {
    println!("Resetting tables ({})...", bits);
    if 0 != bits & 1 {
        context
            .sql()
            .execute("DELETE FROM jobs;", paramsv![])
            .await
            .unwrap();
        println!("(1) Jobs reset.");
    }
    if 0 != bits & 2 {
        context
            .sql()
            .execute("DELETE FROM acpeerstates;", paramsv![])
            .await
            .unwrap();
        println!("(2) Peerstates reset.");
    }
    if 0 != bits & 4 {
        context
            .sql()
            .execute("DELETE FROM keypairs;", paramsv![])
            .await
            .unwrap();
        println!("(4) Private keypairs reset.");
    }
    if 0 != bits & 8 {
        context
            .sql()
            .execute("DELETE FROM contacts WHERE id>9;", paramsv![])
            .await
            .unwrap();
        context
            .sql()
            .execute("DELETE FROM chats WHERE id>9;", paramsv![])
            .await
            .unwrap();
        context
            .sql()
            .execute("DELETE FROM chats_contacts;", paramsv![])
            .await
            .unwrap();
        context
            .sql()
            .execute("DELETE FROM msgs WHERE id>9;", paramsv![])
            .await
            .unwrap();
        context
            .sql()
            .execute(
                "DELETE FROM config WHERE keyname LIKE 'imap.%' OR keyname LIKE 'configured%';",
                paramsv![],
            )
            .await
            .unwrap();
        context
            .sql()
            .execute("DELETE FROM leftgrps;", paramsv![])
            .await
            .unwrap();
        println!("(8) Rest but server config reset.");
    }

    context.emit_event(EventType::MsgsChanged {
        chat_id: ChatId::new(0),
        msg_id: MsgId::new(0),
    });
}

async fn poke_eml_file(context: &Context, filename: impl AsRef<Path>) -> Result<(), anyhow::Error> {
    let data = dc_read_file(context, filename).await?;

    if let Err(err) = dc_receive_imf(context, &data, "import", 0, false).await {
        println!("dc_receive_imf errored: {:?}", err);
    }
    Ok(())
}

/// Import a file to the database.
/// For testing, import a folder with eml-files, a single eml-file, e-mail plus public key and so on.
/// For normal importing, use imex().
async fn poke_spec(context: &Context, spec: Option<&str>) -> bool {
    let mut read_cnt: usize = 0;

    let real_spec: String;

    // if `spec` is given, remember it for later usage; if it is not given, try to use the last one
    if let Some(spec) = spec {
        real_spec = spec.to_string();
        context
            .sql()
            .set_raw_config("import_spec", Some(&real_spec))
            .await
            .unwrap();
    } else {
        let rs = context.sql().get_raw_config("import_spec").await.unwrap();
        if rs.is_none() {
            error!(context, "Import: No file or folder given.");
            return false;
        }
        real_spec = rs.unwrap();
    }
    if let Some(suffix) = dc_get_filesuffix_lc(&real_spec) {
        if suffix == "eml" && poke_eml_file(context, &real_spec).await.is_ok() {
            read_cnt += 1
        }
    } else {
        /* import a directory */
        let dir_name = std::path::Path::new(&real_spec);
        let dir = std::fs::read_dir(dir_name);
        if dir.is_err() {
            error!(context, "Import: Cannot open directory \"{}\".", &real_spec,);
            return false;
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
                    let path_plus_name = format!("{}/{}", &real_spec, name);
                    println!("Import: {}", path_plus_name);
                    if poke_eml_file(context, path_plus_name).await.is_ok() {
                        read_cnt += 1
                    }
                }
            }
        }
    }
    println!("Import: {} items read from \"{}\".", read_cnt, &real_spec);
    if read_cnt > 0 {
        context.emit_event(EventType::MsgsChanged {
            chat_id: ChatId::new(0),
            msg_id: MsgId::new(0),
        });
    }
    true
}

async fn log_msg(context: &Context, prefix: impl AsRef<str>, msg: &Message) {
    let contact = Contact::get_by_id(context, msg.get_from_id())
        .await
        .expect("invalid contact");
    let contact_name = if let Some(name) = msg.get_override_sender_name() {
        format!("~{}", name)
    } else {
        contact.get_display_name().to_string()
    };
    let contact_id = contact.get_id();

    let statestr = match msg.get_state() {
        MessageState::OutPending => " o",
        MessageState::OutDelivered => " √",
        MessageState::OutMdnRcvd => " √√",
        MessageState::OutFailed => " !!",
        _ => "",
    };
    let temp2 = dc_timestamp_to_str(msg.get_timestamp());
    let msgtext = msg.get_text();
    println!(
        "{}{}{}{}: {} (Contact#{}): {} {}{}{}{}{}{} [{}]",
        prefix.as_ref(),
        msg.get_id(),
        if msg.get_showpadlock() { "🔒" } else { "" },
        if msg.has_location() { "📍" } else { "" },
        &contact_name,
        contact_id,
        msgtext.unwrap_or_default(),
        if msg.has_html() { "[HAS-HTML]️" } else { "" },
        if msg.get_from_id() == 1 {
            ""
        } else if msg.get_state() == MessageState::InSeen {
            "[SEEN]"
        } else if msg.get_state() == MessageState::InNoticed {
            "[NOTICED]"
        } else {
            "[FRESH]"
        },
        if msg.is_info() { "[INFO]" } else { "" },
        if msg.get_viewtype() == Viewtype::VideochatInvitation {
            format!(
                "[VIDEOCHAT-INVITATION: {}, type={}]",
                msg.get_videochat_url().unwrap_or_default(),
                msg.get_videochat_type().unwrap_or_default()
            )
        } else {
            "".to_string()
        },
        if msg.is_forwarded() {
            "[FORWARDED]"
        } else {
            ""
        },
        statestr,
        &temp2,
    );
}

async fn log_msglist(context: &Context, msglist: &[MsgId]) -> Result<(), Error> {
    let mut lines_out = 0;
    for &msg_id in msglist {
        if msg_id == MsgId::new(DC_MSG_ID_DAYMARKER) {
            println!(
                "--------------------------------------------------------------------------------"
            );

            lines_out += 1
        } else if !msg_id.is_special() {
            if lines_out == 0 {
                println!(
                    "--------------------------------------------------------------------------------",
                );
                lines_out += 1
            }
            let msg = Message::load_from_db(context, msg_id).await?;
            log_msg(context, "", &msg).await;
        }
    }
    if lines_out > 0 {
        println!(
            "--------------------------------------------------------------------------------"
        );
    }
    Ok(())
}

async fn log_contactlist(context: &Context, contacts: &[u32]) {
    for contact_id in contacts {
        let line;
        let mut line2 = "".to_string();
        if let Ok(contact) = Contact::get_by_id(context, *contact_id).await {
            let name = contact.get_display_name();
            let addr = contact.get_addr();
            let verified_state = contact.is_verified(context).await;
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
            let peerstate = Peerstate::from_addr(context, &addr)
                .await
                .expect("peerstate error");
            if peerstate.is_some() && *contact_id != 1 {
                line2 = format!(
                    ", prefer-encrypt={}",
                    peerstate.as_ref().unwrap().prefer_encrypt
                );
            }

            println!("Contact#{}: {}{}", *contact_id, line, line2);
        }
    }
}

fn chat_prefix(chat: &Chat) -> &'static str {
    chat.typ.into()
}

pub async fn cmdline(context: Context, line: &str, chat_id: &mut ChatId) -> Result<(), Error> {
    let mut sel_chat = if !chat_id.is_unset() {
        Chat::load_from_db(&context, *chat_id).await.ok()
    } else {
        None
    };

    let mut args = line.splitn(3, ' ');
    let arg0 = args.next().unwrap_or_default();
    let arg1 = args.next().unwrap_or_default();
    let arg2 = args.next().unwrap_or_default();

    let blobdir = context.get_blobdir();
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
                 connectivity\n\
                 maybenetwork\n\
                 housekeeping\n\
                 help imex (Import/Export)\n\
                 ==============================Chat commands==\n\
                 listchats [<query>]\n\
                 listarchived\n\
                 chat [<chat-id>|0]\n\
                 createchat <contact-id>\n\
                 creategroup <name>\n\
                 createprotected <name>\n\
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
                 sendsticker <file> [<text>]\n\
                 sendfile <file> [<text>]\n\
                 sendhtml <file for html-part> [<text for plain-part>]\n\
                 videochat\n\
                 draft [<text>]\n\
                 devicemsg <text>\n\
                 listmedia\n\
                 archive <chat-id>\n\
                 unarchive <chat-id>\n\
                 pin <chat-id>\n\
                 unpin <chat-id>\n\
                 mute <chat-id> [<seconds>]\n\
                 unmute <chat-id>\n\
                 protect <chat-id>\n\
                 unprotect <chat-id>\n\
                 delchat <chat-id>\n\
                 ===========================Contact requests==\n\
                 decidestartchat <msg-id>\n\
                 decideblock <msg-id>\n\
                 decidenotnow <msg-id>\n\
                 ===========================Message commands==\n\
                 listmsgs <query>\n\
                 msginfo <msg-id>\n\
                 html <msg-id>\n\
                 listfresh\n\
                 forward <msg-id> <chat-id>\n\
                 markseen <msg-id>\n\
                 delmsg <msg-id>\n\
                 ===========================Contact commands==\n\
                 listcontacts [<query>]\n\
                 listverified [<query>]\n\
                 addcontact [<name>] <addr>\n\
                 contactinfo <contact-id>\n\
                 delcontact <contact-id>\n\
                 cleanupcontacts\n\
                 block <contact-id>\n\
                 unblock <contact-id>\n\
                 listblocked\n\
                 ======================================Misc.==\n\
                 getqr [<chat-id>]\n\
                 getbadqr\n\
                 checkqr <qr-content>\n\
                 setqr <qr-content>\n\
                 providerinfo <addr>\n\
                 event <event-id to test>\n\
                 fileinfo <file>\n\
                 estimatedeletion <seconds>\n\
                 clear -- clear screen\n\
                 exit or quit\n\
                 ============================================="
            ),
        },
        "initiate-key-transfer" => match initiate_key_transfer(&context).await {
            Ok(setup_code) => println!(
                "Setup code for the transferred setup message: {}",
                setup_code,
            ),
            Err(err) => bail!("Failed to generate setup code: {}", err),
        },
        "get-setupcodebegin" => {
            ensure!(!arg1.is_empty(), "Argument <msg-id> missing.");
            let msg_id: MsgId = MsgId::new(arg1.parse()?);
            let msg = Message::load_from_db(&context, msg_id).await?;
            if msg.is_setupmessage() {
                let setupcodebegin = msg.get_setupcodebegin(&context).await;
                println!(
                    "The setup code for setup message {} starts with: {}",
                    msg_id,
                    setupcodebegin.unwrap_or_default(),
                );
            } else {
                bail!("{} is no setup message.", msg_id,);
            }
        }
        "continue-key-transfer" => {
            ensure!(
                !arg1.is_empty() && !arg2.is_empty(),
                "Arguments <msg-id> <setup-code> expected"
            );
            continue_key_transfer(&context, MsgId::new(arg1.parse()?), &arg2).await?;
        }
        "has-backup" => {
            has_backup(&context, blobdir).await?;
        }
        "export-backup" => {
            let dir = dirs::home_dir().unwrap_or_default();
            imex(&context, ImexMode::ExportBackup, dir.as_ref()).await?;
            println!("Exported to {}.", dir.to_string_lossy());
        }
        "import-backup" => {
            ensure!(!arg1.is_empty(), "Argument <backup-file> missing.");
            imex(&context, ImexMode::ImportBackup, arg1.as_ref()).await?;
        }
        "export-keys" => {
            let dir = dirs::home_dir().unwrap_or_default();
            imex(&context, ImexMode::ExportSelfKeys, dir.as_ref()).await?;
            println!("Exported to {}.", dir.to_string_lossy());
        }
        "import-keys" => {
            imex(&context, ImexMode::ImportSelfKeys, arg1.as_ref()).await?;
        }
        "export-setup" => {
            let setup_code = create_setup_code(&context);
            let file_name = blobdir.join("autocrypt-setup-message.html");
            let file_content = render_setup_file(&context, &setup_code).await?;
            async_std::fs::write(&file_name, file_content).await?;
            println!(
                "Setup message written to: {}\nSetup code: {}",
                file_name.display(),
                &setup_code,
            );
        }
        "poke" => {
            ensure!(poke_spec(&context, Some(arg1)).await, "Poke failed");
        }
        "reset" => {
            ensure!(!arg1.is_empty(), "Argument <bits> missing: 1=jobs, 2=peerstates, 4=private keys, 8=rest but server config");
            let bits: i32 = arg1.parse()?;
            ensure!(bits < 16, "<bits> must be lower than 16.");
            reset_tables(&context, bits).await;
        }
        "stop" => {
            context.stop_ongoing().await;
        }
        "set" => {
            ensure!(!arg1.is_empty(), "Argument <key> missing.");
            let key = config::Config::from_str(&arg1)?;
            let value = if arg2.is_empty() { None } else { Some(arg2) };
            context.set_config(key, value).await?;
        }
        "get" => {
            ensure!(!arg1.is_empty(), "Argument <key> missing.");
            let key = config::Config::from_str(&arg1)?;
            let val = context.get_config(key).await;
            println!("{}={:?}", key, val);
        }
        "info" => {
            println!("{:#?}", context.get_info().await);
        }
        "connectivity" => {
            let file = dirs::home_dir()
                .unwrap_or_default()
                .join("connectivity.html");
            let html = context.get_connectivity_html().await;
            fs::write(&file, html)?;
            println!("Report written to: {:#?}", file);
        }
        "maybenetwork" => {
            context.maybe_network().await;
        }
        "housekeeping" => {
            sql::housekeeping(&context).await.ok_or_log(&context);
        }
        "listchats" | "listarchived" | "chats" => {
            let listflags = if arg0 == "listarchived" { 0x01 } else { 0 };
            let time_start = std::time::SystemTime::now();
            let chatlist = Chatlist::try_load(
                &context,
                listflags,
                if arg1.is_empty() { None } else { Some(arg1) },
                None,
            )
            .await?;
            let time_needed = time_start.elapsed().unwrap_or_default();

            let cnt = chatlist.len();
            if cnt > 0 {
                println!(
                    "================================================================================"
                );

                for i in (0..cnt).rev() {
                    let chat = Chat::load_from_db(&context, chatlist.get_chat_id(i)).await?;
                    println!(
                        "{}#{}: {} [{} fresh] {}{}{}",
                        chat_prefix(&chat),
                        chat.get_id(),
                        chat.get_name(),
                        chat.get_id().get_fresh_msg_cnt(&context).await?,
                        if chat.is_muted() { "🔇" } else { "" },
                        match chat.visibility {
                            ChatVisibility::Normal => "",
                            ChatVisibility::Archived => "📦",
                            ChatVisibility::Pinned => "📌",
                        },
                        if chat.is_protected() { "🛡️" } else { "" },
                    );
                    let lot = chatlist.get_summary(&context, i, Some(&chat)).await?;
                    let statestr = if chat.visibility == ChatVisibility::Archived {
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
                    println!(
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
                    println!(
                        "================================================================================"
                    );
                }
            }
            if location::is_sending_locations_to_chat(&context, None).await {
                println!("Location streaming enabled.");
            }
            println!("{} chats", cnt);
            println!("{:?} to create this list", time_needed);
        }
        "chat" => {
            if sel_chat.is_none() && arg1.is_empty() {
                bail!("Argument [chat-id] is missing.");
            }
            if !arg1.is_empty() {
                let id = ChatId::new(arg1.parse()?);
                println!("Selecting chat {}", id);
                sel_chat = Some(Chat::load_from_db(&context, id).await?);
                *chat_id = id;
            }

            ensure!(sel_chat.is_some(), "Failed to select chat");
            let sel_chat = sel_chat.as_ref().unwrap();

            let time_start = std::time::SystemTime::now();
            let msglist =
                chat::get_chat_msgs(&context, sel_chat.get_id(), DC_GCM_ADDDAYMARKER, None).await?;
            let time_needed = time_start.elapsed().unwrap_or_default();

            let msglist: Vec<MsgId> = msglist
                .into_iter()
                .map(|x| match x {
                    ChatItem::Message { msg_id } => msg_id,
                    ChatItem::Marker1 => MsgId::new(DC_MSG_ID_MARKER1),
                    ChatItem::DayMarker { .. } => MsgId::new(DC_MSG_ID_DAYMARKER),
                })
                .collect();

            let members = chat::get_chat_contacts(&context, sel_chat.id).await?;
            let subtitle = if sel_chat.is_device_talk() {
                "device-talk".to_string()
            } else if sel_chat.get_type() == Chattype::Single && !members.is_empty() {
                let contact = Contact::get_by_id(&context, members[0]).await?;
                contact.get_addr().to_string()
            } else if sel_chat.get_type() == Chattype::Mailinglist && !members.is_empty() {
                "mailinglist".to_string()
            } else {
                format!("{} member(s)", members.len())
            };
            println!(
                "{}#{}: {} [{}]{}{}{} {}",
                chat_prefix(sel_chat),
                sel_chat.get_id(),
                sel_chat.get_name(),
                subtitle,
                if sel_chat.is_muted() { "🔇" } else { "" },
                if sel_chat.is_sending_locations() {
                    "📍"
                } else {
                    ""
                },
                match sel_chat.get_profile_image(&context).await? {
                    Some(icon) => match icon.to_str() {
                        Some(icon) => format!(" Icon: {}", icon),
                        _ => " Icon: Err".to_string(),
                    },
                    _ => "".to_string(),
                },
                if sel_chat.is_protected() {
                    "🛡️"
                } else {
                    ""
                },
            );
            log_msglist(&context, &msglist).await?;
            if let Some(draft) = sel_chat.get_id().get_draft(&context).await? {
                log_msg(&context, "Draft", &draft).await;
            }

            println!(
                "{} messages.",
                sel_chat.get_id().get_msg_cnt(&context).await?
            );

            let time_noticed_start = std::time::SystemTime::now();
            chat::marknoticed_chat(&context, sel_chat.get_id()).await?;
            let time_noticed_needed = time_noticed_start.elapsed().unwrap_or_default();

            println!(
                "{:?} to create this list, {:?} to mark all messages as noticed.",
                time_needed, time_noticed_needed
            );
        }
        "createchat" => {
            ensure!(!arg1.is_empty(), "Argument <contact-id> missing.");
            let contact_id: u32 = arg1.parse()?;
            let chat_id = ChatId::create_for_contact(&context, contact_id).await?;

            println!("Single#{} created successfully.", chat_id,);
        }
        "decidestartchat" | "createchatbymsg" => {
            ensure!(!arg1.is_empty(), "Argument <msg-id> missing");
            let msg_id = MsgId::new(arg1.parse()?);
            match message::decide_on_contact_request(
                &context,
                msg_id,
                ContactRequestDecision::StartChat,
            )
            .await
            {
                Some(chat_id) => {
                    let chat = Chat::load_from_db(&context, chat_id).await?;
                    println!("{}#{} created successfully.", chat_prefix(&chat), chat_id);
                }
                None => println!("Cannot crate chat."),
            }
        }
        "decidenotnow" => {
            ensure!(!arg1.is_empty(), "Argument <msg-id> missing");
            let msg_id = MsgId::new(arg1.parse()?);
            message::decide_on_contact_request(&context, msg_id, ContactRequestDecision::NotNow)
                .await;
        }
        "decideblock" => {
            ensure!(!arg1.is_empty(), "Argument <msg-id> missing");
            let msg_id = MsgId::new(arg1.parse()?);
            message::decide_on_contact_request(&context, msg_id, ContactRequestDecision::Block)
                .await;
        }
        "creategroup" => {
            ensure!(!arg1.is_empty(), "Argument <name> missing.");
            let chat_id =
                chat::create_group_chat(&context, ProtectionStatus::Unprotected, arg1).await?;

            println!("Group#{} created successfully.", chat_id);
        }
        "createprotected" => {
            ensure!(!arg1.is_empty(), "Argument <name> missing.");
            let chat_id =
                chat::create_group_chat(&context, ProtectionStatus::Protected, arg1).await?;

            println!("Group#{} created and protected successfully.", chat_id);
        }
        "addmember" => {
            ensure!(sel_chat.is_some(), "No chat selected");
            ensure!(!arg1.is_empty(), "Argument <contact-id> missing.");

            let contact_id_0: u32 = arg1.parse()?;
            if chat::add_contact_to_chat(
                &context,
                sel_chat.as_ref().unwrap().get_id(),
                contact_id_0,
            )
            .await
            {
                println!("Contact added to chat.");
            } else {
                bail!("Cannot add contact to chat.");
            }
        }
        "removemember" => {
            ensure!(sel_chat.is_some(), "No chat selected.");
            ensure!(!arg1.is_empty(), "Argument <contact-id> missing.");
            let contact_id_1: u32 = arg1.parse()?;
            chat::remove_contact_from_chat(
                &context,
                sel_chat.as_ref().unwrap().get_id(),
                contact_id_1,
            )
            .await?;

            println!("Contact added to chat.");
        }
        "groupname" => {
            ensure!(sel_chat.is_some(), "No chat selected.");
            ensure!(!arg1.is_empty(), "Argument <name> missing.");
            chat::set_chat_name(&context, sel_chat.as_ref().unwrap().get_id(), arg1).await?;

            println!("Chat name set");
        }
        "groupimage" => {
            ensure!(sel_chat.is_some(), "No chat selected.");
            ensure!(!arg1.is_empty(), "Argument <image> missing.");

            chat::set_chat_profile_image(&context, sel_chat.as_ref().unwrap().get_id(), arg1)
                .await?;

            println!("Chat image set");
        }
        "chatinfo" => {
            ensure!(sel_chat.is_some(), "No chat selected.");

            let contacts =
                chat::get_chat_contacts(&context, sel_chat.as_ref().unwrap().get_id()).await?;
            println!("Memberlist:");

            log_contactlist(&context, &contacts).await;
            println!(
                "{} contacts\nLocation streaming: {}",
                contacts.len(),
                location::is_sending_locations_to_chat(
                    &context,
                    Some(sel_chat.as_ref().unwrap().get_id())
                )
                .await,
            );
        }
        "getlocations" => {
            ensure!(sel_chat.is_some(), "No chat selected.");

            let contact_id: Option<u32> = arg1.parse().ok();
            let locations = location::get_range(
                &context,
                Some(sel_chat.as_ref().unwrap().get_id()),
                contact_id,
                0,
                0,
            )
            .await?;
            let default_marker = "-".to_string();
            for location in &locations {
                let marker = location.marker.as_ref().unwrap_or(&default_marker);
                println!(
                    "Loc#{}: {}: lat={} lng={} acc={} Chat#{} Contact#{} {} {}",
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
                println!("No locations.");
            }
        }
        "sendlocations" => {
            ensure!(sel_chat.is_some(), "No chat selected.");
            ensure!(!arg1.is_empty(), "No timeout given.");

            let seconds = arg1.parse()?;
            location::send_locations_to_chat(
                &context,
                sel_chat.as_ref().unwrap().get_id(),
                seconds,
            )
            .await;
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

            let continue_streaming = location::set(&context, latitude, longitude, 0.).await;
            if continue_streaming {
                println!("Success, streaming should be continued.");
            } else {
                println!("Success, streaming can be stoppped.");
            }
        }
        "dellocations" => {
            location::delete_all(&context).await?;
        }
        "send" => {
            ensure!(sel_chat.is_some(), "No chat selected.");
            ensure!(!arg1.is_empty(), "No message text given.");

            let msg = format!("{} {}", arg1, arg2);

            chat::send_text_msg(&context, sel_chat.as_ref().unwrap().get_id(), msg).await?;
        }
        "sendempty" => {
            ensure!(sel_chat.is_some(), "No chat selected.");
            chat::send_text_msg(&context, sel_chat.as_ref().unwrap().get_id(), "".into()).await?;
        }
        "sendimage" | "sendsticker" | "sendfile" => {
            ensure!(sel_chat.is_some(), "No chat selected.");
            ensure!(!arg1.is_empty(), "No file given.");

            let mut msg = Message::new(if arg0 == "sendimage" {
                Viewtype::Image
            } else if arg0 == "sendsticker" {
                Viewtype::Sticker
            } else {
                Viewtype::File
            });
            msg.set_file(arg1, None);
            if !arg2.is_empty() {
                msg.set_text(Some(arg2.to_string()));
            }
            chat::send_msg(&context, sel_chat.as_ref().unwrap().get_id(), &mut msg).await?;
        }
        "sendhtml" => {
            ensure!(sel_chat.is_some(), "No chat selected.");
            ensure!(!arg1.is_empty(), "No html-file given.");
            let path: &Path = arg1.as_ref();
            let html = &*fs::read(&path)?;
            let html = String::from_utf8_lossy(html);

            let mut msg = Message::new(Viewtype::Text);
            msg.set_html(Some(html.to_string()));
            msg.set_text(Some(if arg2.is_empty() {
                path.file_name().unwrap().to_string_lossy().to_string()
            } else {
                arg2.to_string()
            }));
            chat::send_msg(&context, sel_chat.as_ref().unwrap().get_id(), &mut msg).await?;
        }
        "videochat" => {
            ensure!(sel_chat.is_some(), "No chat selected.");
            chat::send_videochat_invitation(&context, sel_chat.as_ref().unwrap().get_id()).await?;
        }
        "listmsgs" => {
            ensure!(!arg1.is_empty(), "Argument <query> missing.");

            let chat = if let Some(ref sel_chat) = sel_chat {
                Some(sel_chat.get_id())
            } else {
                None
            };

            let time_start = std::time::SystemTime::now();
            let msglist = context.search_msgs(chat, arg1).await?;
            let time_needed = time_start.elapsed().unwrap_or_default();

            log_msglist(&context, &msglist).await?;
            println!("{} messages.", msglist.len());
            println!("{:?} to create this list", time_needed);
        }
        "draft" => {
            ensure!(sel_chat.is_some(), "No chat selected.");

            if !arg1.is_empty() {
                let mut draft = Message::new(Viewtype::Text);
                draft.set_text(Some(arg1.to_string()));
                sel_chat
                    .as_ref()
                    .unwrap()
                    .get_id()
                    .set_draft(&context, Some(&mut draft))
                    .await?;
                println!("Draft saved.");
            } else {
                sel_chat
                    .as_ref()
                    .unwrap()
                    .get_id()
                    .set_draft(&context, None)
                    .await?;
                println!("Draft deleted.");
            }
        }
        "devicemsg" => {
            ensure!(
                !arg1.is_empty(),
                "Please specify text to add as device message."
            );
            let mut msg = Message::new(Viewtype::Text);
            msg.set_text(Some(arg1.to_string()));
            chat::add_device_msg(&context, None, Some(&mut msg)).await?;
        }
        "listmedia" => {
            ensure!(sel_chat.is_some(), "No chat selected.");

            let images = chat::get_chat_media(
                &context,
                sel_chat.as_ref().unwrap().get_id(),
                Viewtype::Image,
                Viewtype::Gif,
                Viewtype::Video,
            )
            .await?;
            println!("{} images or videos: ", images.len());
            for (i, data) in images.iter().enumerate() {
                if 0 == i {
                    print!("{}", data);
                } else {
                    print!(", {}", data);
                }
            }
            println!();
        }
        "archive" | "unarchive" | "pin" | "unpin" => {
            ensure!(!arg1.is_empty(), "Argument <chat-id> missing.");
            let chat_id = ChatId::new(arg1.parse()?);
            chat_id
                .set_visibility(
                    &context,
                    match arg0 {
                        "archive" => ChatVisibility::Archived,
                        "unarchive" | "unpin" => ChatVisibility::Normal,
                        "pin" => ChatVisibility::Pinned,
                        _ => unreachable!("arg0={:?}", arg0),
                    },
                )
                .await?;
        }
        "mute" | "unmute" => {
            ensure!(!arg1.is_empty(), "Argument <chat-id> missing.");
            let chat_id = ChatId::new(arg1.parse()?);
            let duration = match arg0 {
                "mute" => {
                    if arg2.is_empty() {
                        MuteDuration::Forever
                    } else {
                        SystemTime::now()
                            .checked_add(Duration::from_secs(arg2.parse()?))
                            .map_or(MuteDuration::Forever, MuteDuration::Until)
                    }
                }
                "unmute" => MuteDuration::NotMuted,
                _ => unreachable!("arg0={:?}", arg0),
            };
            chat::set_muted(&context, chat_id, duration).await?;
        }
        "protect" | "unprotect" => {
            ensure!(!arg1.is_empty(), "Argument <chat-id> missing.");
            let chat_id = ChatId::new(arg1.parse()?);
            chat_id
                .set_protection(
                    &context,
                    match arg0 {
                        "protect" => ProtectionStatus::Protected,
                        "unprotect" => ProtectionStatus::Unprotected,
                        _ => unreachable!("arg0={:?}", arg0),
                    },
                )
                .await?;
        }
        "delchat" => {
            ensure!(!arg1.is_empty(), "Argument <chat-id> missing.");
            let chat_id = ChatId::new(arg1.parse()?);
            chat_id.delete(&context).await?;
        }
        "msginfo" => {
            ensure!(!arg1.is_empty(), "Argument <msg-id> missing.");
            let id = MsgId::new(arg1.parse()?);
            let res = message::get_msg_info(&context, id).await?;
            println!("{}", res);
        }
        "html" => {
            ensure!(!arg1.is_empty(), "Argument <msg-id> missing.");
            let id = MsgId::new(arg1.parse()?);
            let file = dirs::home_dir()
                .unwrap_or_default()
                .join(format!("msg-{}.html", id.to_u32()));
            let html = id.get_html(&context).await?.unwrap_or_default();
            fs::write(&file, html)?;
            println!("HTML written to: {:#?}", file);
        }
        "listfresh" => {
            let msglist = context.get_fresh_msgs().await?;

            log_msglist(&context, &msglist).await?;
            print!("{} fresh messages.", msglist.len());
        }
        "forward" => {
            ensure!(
                !arg1.is_empty() && !arg2.is_empty(),
                "Arguments <msg-id> <chat-id> expected"
            );

            let mut msg_ids = [MsgId::new(0); 1];
            let chat_id = ChatId::new(arg2.parse()?);
            msg_ids[0] = MsgId::new(arg1.parse()?);
            chat::forward_msgs(&context, &msg_ids, chat_id).await?;
        }
        "markseen" => {
            ensure!(!arg1.is_empty(), "Argument <msg-id> missing.");
            let mut msg_ids = vec![MsgId::new(0)];
            msg_ids[0] = MsgId::new(arg1.parse()?);
            message::markseen_msgs(&context, msg_ids).await?;
        }
        "delmsg" => {
            ensure!(!arg1.is_empty(), "Argument <msg-id> missing.");
            let mut ids = [MsgId::new(0); 1];
            ids[0] = MsgId::new(arg1.parse()?);
            message::delete_msgs(&context, &ids).await;
        }
        "listcontacts" | "contacts" | "listverified" => {
            let contacts = Contact::get_all(
                &context,
                if arg0 == "listverified" {
                    DC_GCL_VERIFIED_ONLY | DC_GCL_ADD_SELF
                } else {
                    DC_GCL_ADD_SELF
                },
                Some(arg1),
            )
            .await?;
            log_contactlist(&context, &contacts).await;
            println!("{} contacts.", contacts.len());
        }
        "addcontact" => {
            ensure!(!arg1.is_empty(), "Arguments [<name>] <addr> expected.");

            if !arg2.is_empty() {
                let book = format!("{}\n{}", arg1, arg2);
                Contact::add_address_book(&context, &book).await?;
            } else {
                Contact::create(&context, "", arg1).await?;
            }
        }
        "contactinfo" => {
            ensure!(!arg1.is_empty(), "Argument <contact-id> missing.");

            let contact_id: u32 = arg1.parse()?;
            let contact = Contact::get_by_id(&context, contact_id).await?;
            let name_n_addr = contact.get_name_n_addr();

            let mut res = format!(
                "Contact info for: {}:\nIcon: {}\n",
                name_n_addr,
                match contact.get_profile_image(&context).await? {
                    Some(image) => image.to_str().unwrap().to_string(),
                    None => "NoIcon".to_string(),
                }
            );

            res += &Contact::get_encrinfo(&context, contact_id).await?;

            let chatlist = Chatlist::try_load(&context, 0, None, Some(contact_id)).await?;
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
                    let chat = Chat::load_from_db(&context, chatlist.get_chat_id(i)).await?;
                    res += &format!("{}#{}", chat_prefix(&chat), chat.get_id());
                }
            }

            println!("{}", res);
        }
        "delcontact" => {
            ensure!(!arg1.is_empty(), "Argument <contact-id> missing.");
            Contact::delete(&context, arg1.parse()?).await?;
        }
        "block" => {
            ensure!(!arg1.is_empty(), "Argument <contact-id> missing.");
            let contact_id = arg1.parse()?;
            Contact::block(&context, contact_id).await?;
        }
        "unblock" => {
            ensure!(!arg1.is_empty(), "Argument <contact-id> missing.");
            let contact_id = arg1.parse()?;
            Contact::unblock(&context, contact_id).await?;
        }
        "listblocked" => {
            let contacts = Contact::get_all_blocked(&context).await?;
            log_contactlist(&context, &contacts).await;
            println!("{} blocked contacts.", contacts.len());
        }
        "checkqr" => {
            ensure!(!arg1.is_empty(), "Argument <qr-content> missing.");
            let res = check_qr(&context, arg1).await;
            println!(
                "state={}, id={}, text1={:?}, text2={:?}",
                res.get_state(),
                res.get_id(),
                res.get_text1(),
                res.get_text2()
            );
        }
        "setqr" => {
            ensure!(!arg1.is_empty(), "Argument <qr-content> missing.");
            match set_config_from_qr(&context, arg1).await {
                Ok(()) => println!("Config set from QR code, you can now call 'configure'"),
                Err(err) => println!("Cannot set config from QR code: {:?}", err),
            }
        }
        "providerinfo" => {
            ensure!(!arg1.is_empty(), "Argument <addr> missing.");
            match provider::get_provider_info(arg1).await {
                Some(info) => {
                    println!("Information for provider belonging to {}:", arg1);
                    println!("status: {}", info.status as u32);
                    println!("before_login_hint: {}", info.before_login_hint);
                    println!("after_login_hint: {}", info.after_login_hint);
                    println!("overview_page: {}", info.overview_page);
                    for server in info.server.iter() {
                        println!("server: {}:{}", server.hostname, server.port,);
                    }
                }
                None => {
                    println!("No information for provider belonging to {} found.", arg1);
                }
            }
        }
        // TODO: implement this again, unclear how to match this through though, without writing a parser.
        // "event" => {
        //     ensure!(!arg1.is_empty(), "Argument <id> missing.");
        //     let event = arg1.parse()?;
        //     let event = EventType::from_u32(event).ok_or(format_err!("EventType::from_u32({})", event))?;
        //     let r = context.emit_event(event, 0 as libc::uintptr_t, 0 as libc::uintptr_t);
        //     println!(
        //         "Sending event {:?}({}), received value {}.",
        //         event, event as usize, r,
        //     );
        // }
        "fileinfo" => {
            ensure!(!arg1.is_empty(), "Argument <file> missing.");

            if let Ok(buf) = dc_read_file(&context, &arg1).await {
                let (width, height) = dc_get_filemeta(&buf)?;
                println!("width={}, height={}", width, height);
            } else {
                bail!("Command failed.");
            }
        }
        "estimatedeletion" => {
            ensure!(!arg1.is_empty(), "Argument <seconds> missing");
            let seconds = arg1.parse()?;
            let device_cnt = message::estimate_deletion_cnt(&context, false, seconds).await?;
            let server_cnt = message::estimate_deletion_cnt(&context, true, seconds).await?;
            println!(
                "estimated count of messages older than {} seconds:\non device: {}\non server: {}",
                seconds, device_cnt, server_cnt
            );
        }
        "" => (),
        _ => bail!("Unknown command: \"{}\" type ? for help.", arg0),
    }

    Ok(())
}
