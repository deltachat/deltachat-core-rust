use std::path::Path;
use std::ptr;

use chrono::TimeZone;
use libc::{free, strcmp};
use mmime::clist::*;
use mmime::mailimf_types::*;
use mmime::mailimf_types_helper::*;
use mmime::mailmime_disposition::*;
use mmime::mailmime_types::*;
use mmime::mailmime_types_helper::*;
use mmime::mailmime_write_mem::*;
use mmime::mmapstring::*;
use mmime::other::*;

use crate::chat::{self, Chat};
use crate::constants::*;
use crate::contact::*;
use crate::context::{get_version_str, Context};
use crate::dc_mimeparser::SystemMessage;
use crate::dc_strencode::*;
use crate::dc_tools::*;
use crate::e2ee::*;
use crate::error::Error;
use crate::location;
use crate::message::{self, Message};
use crate::param::*;
use crate::stock::StockMessage;

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum Loaded {
    Nothing,
    Message,
    MDN, // TODO: invent more descriptive name
}

#[derive(Clone)]
pub struct MimeFactory<'a> {
    pub from_addr: *mut libc::c_char,
    pub from_displayname: *mut libc::c_char,
    pub selfstatus: Option<String>,
    pub recipients_names: *mut clist,
    pub recipients_addr: *mut clist,
    pub timestamp: i64,
    pub rfc724_mid: String,
    pub loaded: Loaded,
    pub msg: Message,
    pub chat: Option<Chat>,
    pub increation: bool,
    pub in_reply_to: *mut libc::c_char,
    pub references: *mut libc::c_char,
    pub req_mdn: libc::c_int,
    pub out: *mut MMAPString,
    pub out_encrypted: bool,
    pub out_gossiped: bool,
    pub out_last_added_location_id: u32,
    pub error: *mut libc::c_char,
    pub context: &'a Context,
}

impl<'a> MimeFactory<'a> {
    fn new(context: &'a Context, msg: Message) -> Self {
        MimeFactory {
            from_addr: ptr::null_mut(),
            from_displayname: ptr::null_mut(),
            selfstatus: None,
            recipients_names: clist_new(),
            recipients_addr: clist_new(),
            timestamp: 0,
            rfc724_mid: String::default(),
            loaded: Loaded::Nothing,
            msg,
            chat: None,
            increation: false,
            in_reply_to: ptr::null_mut(),
            references: ptr::null_mut(),
            req_mdn: 0,
            out: ptr::null_mut(),
            out_encrypted: false,
            out_gossiped: false,
            out_last_added_location_id: 0,
            error: ptr::null_mut(),
            context,
        }
    }
}

impl<'a> Drop for MimeFactory<'a> {
    fn drop(&mut self) {
        unsafe {
            free(self.from_addr as *mut libc::c_void);
            free(self.from_displayname as *mut libc::c_void);
            if !self.recipients_names.is_null() {
                clist_free_content(self.recipients_names);
                clist_free(self.recipients_names);
            }
            if !self.recipients_addr.is_null() {
                clist_free_content(self.recipients_addr);
                clist_free(self.recipients_addr);
            }

            free(self.in_reply_to as *mut libc::c_void);
            free(self.references as *mut libc::c_void);
            if !self.out.is_null() {
                mmap_string_free(self.out);
            }
            free(self.error as *mut libc::c_void);
        }
    }
}

pub unsafe fn dc_mimefactory_load_msg(
    context: &Context,
    msg_id: u32,
) -> Result<MimeFactory, Error> {
    ensure!(msg_id > DC_CHAT_ID_LAST_SPECIAL, "Invalid chat id");

    let msg = Message::load_from_db(context, msg_id)?;
    let chat = Chat::load_from_db(context, msg.chat_id)?;
    let mut factory = MimeFactory::new(context, msg);
    factory.chat = Some(chat);

    load_from(&mut factory);

    // just set the chat above
    let chat = factory.chat.as_ref().unwrap();

    if chat.is_self_talk() {
        clist_insert_after(
            factory.recipients_names,
            (*factory.recipients_names).last,
            dc_strdup_keep_null(factory.from_displayname) as *mut libc::c_void,
        );
        clist_insert_after(
            factory.recipients_addr,
            (*factory.recipients_addr).last,
            dc_strdup(factory.from_addr) as *mut libc::c_void,
        );
    } else {
        context
            .sql
            .query_map(
                "SELECT c.authname, c.addr  \
                 FROM chats_contacts cc  \
                 LEFT JOIN contacts c ON cc.contact_id=c.id  \
                 WHERE cc.chat_id=? AND cc.contact_id>9;",
                params![factory.msg.chat_id as i32],
                |row| {
                    let authname: String = row.get(0)?;
                    let addr: String = row.get(1)?;
                    Ok((authname, addr))
                },
                |rows| {
                    for row in rows {
                        let (authname, addr) = row?;
                        let addr_c = addr.strdup();
                        if !clist_search_string_nocase(factory.recipients_addr, addr_c) {
                            clist_insert_after(
                                factory.recipients_names,
                                (*factory.recipients_names).last,
                                if !authname.is_empty() {
                                    authname.strdup()
                                } else {
                                    std::ptr::null_mut()
                                } as *mut libc::c_void,
                            );
                            clist_insert_after(
                                factory.recipients_addr,
                                (*factory.recipients_addr).last,
                                addr_c as *mut libc::c_void,
                            );
                        }
                    }
                    Ok(())
                },
            )
            .unwrap();

        let command = factory.msg.param.get_cmd();
        let msg = &factory.msg;

        if command == SystemMessage::MemberRemovedFromGroup {
            let email_to_remove = msg.param.get(Param::Arg).unwrap_or_default();
            let email_to_remove_c = email_to_remove.strdup();

            let self_addr = context
                .sql
                .get_config(context, "configured_addr")
                .unwrap_or_default();

            if !email_to_remove.is_empty() && email_to_remove != self_addr {
                if !clist_search_string_nocase(factory.recipients_addr, email_to_remove_c) {
                    clist_insert_after(
                        factory.recipients_names,
                        (*factory.recipients_names).last,
                        ptr::null_mut(),
                    );
                    clist_insert_after(
                        factory.recipients_addr,
                        (*factory.recipients_addr).last,
                        email_to_remove_c as *mut libc::c_void,
                    );
                }
            }
        }
        if command != SystemMessage::AutocryptSetupMessage
            && command != SystemMessage::SecurejoinMessage
            && 0 != context
                .sql
                .get_config_int(context, "mdns_enabled")
                .unwrap_or_else(|| 1)
        {
            factory.req_mdn = 1;
        }
    }
    let row = context.sql.query_row(
        "SELECT mime_in_reply_to, mime_references FROM msgs WHERE id=?",
        params![factory.msg.id as i32],
        |row| {
            let in_reply_to: String = row.get(0)?;
            let references: String = row.get(1)?;

            Ok((in_reply_to, references))
        },
    );
    match row {
        Ok((in_reply_to, references)) => {
            factory.in_reply_to = in_reply_to.strdup();
            factory.references = references.strdup();
        }
        Err(err) => {
            error!(
                context,
                "mimefactory: failed to load mime_in_reply_to: {:?}", err
            );
        }
    }

    factory.loaded = Loaded::Message;
    factory.timestamp = factory.msg.timestamp_sort;
    factory.rfc724_mid = factory.msg.rfc724_mid.clone();
    factory.increation = factory.msg.is_increation();

    Ok(factory)
}

unsafe fn load_from(factory: &mut MimeFactory) {
    let context = factory.context;
    factory.from_addr = context
        .sql
        .get_config(context, "configured_addr")
        .unwrap_or_default()
        .strdup();

    factory.from_displayname = context
        .sql
        .get_config(context, "displayname")
        .unwrap_or_default()
        .strdup();

    factory.selfstatus = context.sql.get_config(context, "selfstatus");

    if factory.selfstatus.is_none() {
        factory.selfstatus = Some(
            factory
                .context
                .stock_str(StockMessage::StatusLine)
                .to_string(),
        );
    }
}

pub unsafe fn dc_mimefactory_load_mdn<'a>(
    context: &'a Context,
    msg_id: u32,
) -> Result<MimeFactory, Error> {
    if 0 == context
        .sql
        .get_config_int(context, "mdns_enabled")
        .unwrap_or_else(|| 1)
    {
        // MDNs not enabled - check this is late, in the job. the use may have changed its
        // choice while offline ...

        bail!("MDNs disabled ")
    }

    let msg = Message::load_from_db(context, msg_id)?;
    let mut factory = MimeFactory::new(context, msg);
    let contact = Contact::load_from_db(factory.context, factory.msg.from_id)?;

    // Do not send MDNs trash etc.; chats.blocked is already checked by the caller
    // in dc_markseen_msgs()
    ensure!(!contact.is_blocked(), "Contact blocked");
    ensure!(
        factory.msg.chat_id > DC_CHAT_ID_LAST_SPECIAL,
        "Invalid chat id"
    );

    clist_insert_after(
        factory.recipients_names,
        (*factory.recipients_names).last,
        (if !contact.get_authname().is_empty() {
            contact.get_authname().strdup()
        } else {
            ptr::null_mut()
        }) as *mut libc::c_void,
    );
    clist_insert_after(
        factory.recipients_addr,
        (*factory.recipients_addr).last,
        contact.get_addr().strdup() as *mut libc::c_void,
    );
    load_from(&mut factory);
    factory.timestamp = dc_create_smeared_timestamp(factory.context);
    factory.rfc724_mid = dc_create_outgoing_rfc724_mid(None, as_str(factory.from_addr));
    factory.loaded = Loaded::MDN;

    Ok(factory)
}

pub unsafe fn dc_mimefactory_render(context: &Context, factory: &mut MimeFactory) -> bool {
    let subject: *mut mailimf_subject;
    let mut ok_to_continue = true;
    let imf_fields: *mut mailimf_fields;
    let mut message: *mut mailmime = ptr::null_mut();
    let mut afwd_email = false;
    let mut col: libc::c_int = 0;
    let mut success = false;
    let mut parts: libc::c_int = 0;
    let mut e2ee_guaranteed = false;
    let mut min_verified: libc::c_int = 0;
    // 1=add Autocrypt-header (needed eg. for handshaking), 2=no Autocrypte-header (used for MDN)
    let mut force_plaintext: libc::c_int = 0;
    let mut do_gossip: libc::c_int = 0;
    let mut grpimage = None;
    let mut e2ee_helper = E2eeHelper::default();

    if factory.loaded == Loaded::Nothing || !factory.out.is_null() {
        /*call empty() before*/
        set_error(factory, "Invalid use of mimefactory-object.");
    } else {
        let from: *mut mailimf_mailbox_list = mailimf_mailbox_list_new_empty();
        mailimf_mailbox_list_add(
            from,
            mailimf_mailbox_new(
                if !factory.from_displayname.is_null() {
                    dc_encode_header_words(as_str(factory.from_displayname))
                } else {
                    ptr::null_mut()
                },
                dc_strdup(factory.from_addr),
            ),
        );
        let mut to: *mut mailimf_address_list = ptr::null_mut();
        if !factory.recipients_names.is_null()
            && !factory.recipients_addr.is_null()
            && (*factory.recipients_addr).count > 0
        {
            let name_iter = (*factory.recipients_names).into_iter();
            let addr_iter = (*factory.recipients_addr).into_iter();
            to = mailimf_address_list_new_empty();
            for (name, addr) in name_iter.zip(addr_iter) {
                let name = name as *const libc::c_char;
                let addr = addr as *const libc::c_char;
                mailimf_address_list_add(
                    to,
                    mailimf_address_new(
                        MAILIMF_ADDRESS_MAILBOX as libc::c_int,
                        mailimf_mailbox_new(
                            if !name.is_null() {
                                dc_encode_header_words(as_str(name))
                            } else {
                                ptr::null_mut()
                            },
                            dc_strdup(addr),
                        ),
                        ptr::null_mut(),
                    ),
                );
            }
        }
        let mut references_list: *mut clist = ptr::null_mut();
        if !factory.references.is_null() && 0 != *factory.references.offset(0isize) as libc::c_int {
            references_list = dc_str_to_clist(
                factory.references,
                b" \x00" as *const u8 as *const libc::c_char,
            )
        }
        let mut in_reply_to_list: *mut clist = ptr::null_mut();
        if !factory.in_reply_to.is_null() && 0 != *factory.in_reply_to.offset(0isize) as libc::c_int
        {
            in_reply_to_list = dc_str_to_clist(
                factory.in_reply_to,
                b" \x00" as *const u8 as *const libc::c_char,
            )
        }
        imf_fields = mailimf_fields_new_with_data_all(
            mailimf_get_date(factory.timestamp as i64),
            from,
            ptr::null_mut(),
            ptr::null_mut(),
            to,
            ptr::null_mut(),
            ptr::null_mut(),
            factory.rfc724_mid.strdup(),
            in_reply_to_list,
            references_list,
            ptr::null_mut(),
        );

        let os_name = &factory.context.os_name;
        let os_part = os_name
            .as_ref()
            .map(|s| format!("/{}", s))
            .unwrap_or_default();
        let version = get_version_str();
        let headerval = format!("Delta Chat Core {}{}", version, os_part);
        add_mailimf_field(imf_fields, "X-Mailer", &headerval);
        add_mailimf_field(imf_fields, "Chat-Version", "1.0");
        if 0 != factory.req_mdn {
            let headerval = to_string(factory.from_addr);
            add_mailimf_field(imf_fields, "Chat-Disposition-Notification-To", &headerval);
        }
        message = mailmime_new_message_data(0 as *mut mailmime);
        mailmime_set_imf_fields(message, imf_fields);
        if factory.loaded == Loaded::Message {
            /* Render a normal message
             *********************************************************************/
            let chat = factory.chat.as_ref().unwrap();
            let mut meta_part: *mut mailmime = ptr::null_mut();
            let mut placeholdertext = None;

            if chat.typ == Chattype::VerifiedGroup {
                add_mailimf_field(imf_fields, "Chat-Verified", "1");
                force_plaintext = 0;
                e2ee_guaranteed = true;
                min_verified = 2
            } else {
                force_plaintext = factory
                    .msg
                    .param
                    .get_int(Param::ForcePlaintext)
                    .unwrap_or_default();
                if force_plaintext == 0 {
                    e2ee_guaranteed = factory
                        .msg
                        .param
                        .get_int(Param::GuranteeE2ee)
                        .unwrap_or_default()
                        != 0;
                }
            }
            if chat.gossiped_timestamp == 0
                || (chat.gossiped_timestamp + (2 * 24 * 60 * 60)) < time()
            {
                do_gossip = 1
            }

            /* build header etc. */
            let command = factory.msg.param.get_cmd();
            if chat.typ == Chattype::Group || chat.typ == Chattype::VerifiedGroup {
                add_mailimf_field(imf_fields, "Chat-Group-ID", &chat.grpid);

                // we can't use add_mailimf_field() because
                // dc_encode_header_words returns char* and most of its call sites
                // use it rather directly to pass something to the
                // low-level mailimf_* API.
                let res = mailimf_fields_add(
                    imf_fields,
                    mailimf_field_new_custom(
                        "Chat-Group-Name".strdup(),
                        dc_encode_header_words(&chat.name),
                    ),
                );
                assert!(res == MAILIMF_NO_ERROR as i32);

                if command == SystemMessage::MemberRemovedFromGroup {
                    let email_to_remove = factory.msg.param.get(Param::Arg).unwrap_or_default();
                    if !email_to_remove.is_empty() {
                        add_mailimf_field(
                            imf_fields,
                            "Chat-Group-Member-Removed",
                            &email_to_remove,
                        );
                    }
                } else if command == SystemMessage::MemberAddedToGroup {
                    let msg = &factory.msg;
                    do_gossip = 1;
                    let email_to_add = msg.param.get(Param::Arg).unwrap_or_default();
                    if !email_to_add.is_empty() {
                        add_mailimf_field(imf_fields, "Chat-Group-Member-Added", &email_to_add);
                        grpimage = chat.param.get(Param::ProfileImage);
                    }
                    if 0 != msg.param.get_int(Param::Arg2).unwrap_or_default() & 0x1 {
                        info!(
                            context,
                            "sending secure-join message \'{}\' >>>>>>>>>>>>>>>>>>>>>>>>>",
                            "vg-member-added",
                        );
                        add_mailimf_field(imf_fields, "Secure-Join", "vg-member-added");
                    }
                } else if command == SystemMessage::GroupNameChanged {
                    let msg = &factory.msg;
                    let value_to_add = msg.param.get(Param::Arg).unwrap_or_default();

                    add_mailimf_field(imf_fields, "Chat-Group-Name-Changed", &value_to_add);
                } else if command == SystemMessage::GroupImageChanged {
                    let msg = &factory.msg;
                    grpimage = msg.param.get(Param::Arg);
                    if grpimage.is_none() {
                        add_mailimf_field(imf_fields, "Chat-Group-Image", "0");
                    }
                }
            }

            if command == SystemMessage::LocationStreamingEnabled {
                add_mailimf_field(imf_fields, "Chat-Content", "location-streaming-enabled");
            }
            if command == SystemMessage::AutocryptSetupMessage {
                add_mailimf_field(imf_fields, "Autocrypt-Setup-Message", "v1");
                placeholdertext = Some(
                    factory
                        .context
                        .stock_str(StockMessage::AcSetupMsgBody)
                        .to_string(),
                );
            }
            if command == SystemMessage::SecurejoinMessage {
                let msg = &factory.msg;
                let step = msg.param.get(Param::Arg).unwrap_or_default();
                if !step.is_empty() {
                    info!(
                        context,
                        "sending secure-join message \'{}\' >>>>>>>>>>>>>>>>>>>>>>>>>", step,
                    );
                    add_mailimf_field(imf_fields, "Secure-Join", &step);
                    let param2 = msg.param.get(Param::Arg2).unwrap_or_default();
                    if !param2.is_empty() {
                        add_mailimf_field(
                            imf_fields,
                            if step == "vg-request-with-auth" || step == "vc-request-with-auth" {
                                "Secure-Join-Auth"
                            } else {
                                "Secure-Join-Invitenumber"
                            },
                            param2,
                        )
                    }
                    let fingerprint = msg.param.get(Param::Arg3).unwrap_or_default();
                    if !fingerprint.is_empty() {
                        add_mailimf_field(imf_fields, "Secure-Join-Fingerprint", &fingerprint);
                    }
                    match msg.param.get(Param::Arg4) {
                        Some(id) => {
                            add_mailimf_field(imf_fields, "Secure-Join-Group", &id);
                        }
                        None => {}
                    };
                }
            }

            if let Some(grpimage) = grpimage {
                info!(factory.context, "setting group image '{}'", grpimage);
                let mut meta = Message::default();
                meta.type_0 = Viewtype::Image;
                meta.param.set(Param::File, grpimage);

                let res = build_body_file(context, &meta, "group-image");
                meta_part = res.0;
                let filename_as_sent = res.1;
                if !meta_part.is_null() {
                    add_mailimf_field(imf_fields, "Chat-Group-Image", &filename_as_sent)
                }
            }

            if factory.msg.type_0 == Viewtype::Voice
                || factory.msg.type_0 == Viewtype::Audio
                || factory.msg.type_0 == Viewtype::Video
            {
                if factory.msg.type_0 == Viewtype::Voice {
                    add_mailimf_field(imf_fields, "Chat-Voice-Message", "1");
                }
                let duration_ms = factory
                    .msg
                    .param
                    .get_int(Param::Duration)
                    .unwrap_or_default();
                if duration_ms > 0 {
                    let dur = duration_ms.to_string();
                    add_mailimf_field(imf_fields, "Chat-Duration", &dur);
                }
            }
            afwd_email = factory.msg.param.exists(Param::Forwarded);
            let fwdhint = if afwd_email {
                Some(
                    "---------- Forwarded message ----------\r\nFrom: Delta Chat\r\n\r\n"
                        .to_string(),
                )
            } else {
                None
            };

            let final_text = {
                if let Some(ref text) = placeholdertext {
                    text
                } else if let Some(ref text) = factory.msg.text {
                    text
                } else {
                    ""
                }
            };

            let footer = factory.selfstatus.as_ref();
            let message_text = format!(
                "{}{}{}{}{}",
                fwdhint.unwrap_or_default(),
                &final_text,
                if !final_text.is_empty() && footer.is_some() {
                    "\r\n\r\n"
                } else {
                    ""
                },
                if footer.is_some() { "-- \r\n" } else { "" },
                match footer {
                    Some(x) => x,
                    None => "",
                }
            );
            let text_part = build_body_text(&message_text);
            mailmime_smart_add_part(message, text_part);
            parts += 1;

            /* add attachment part */
            if chat::msgtype_has_file(factory.msg.type_0) {
                if !is_file_size_okay(context, &factory.msg) {
                    let error = format!(
                        "Message exceeds the recommended {} MB.",
                        24 * 1024 * 1024 / 4 * 3 / 1000 / 1000,
                    );
                    set_error(factory, &error);
                    ok_to_continue = false;
                } else {
                    let (file_part, _) = build_body_file(context, &factory.msg, "");
                    if !file_part.is_null() {
                        mailmime_smart_add_part(message, file_part);
                        parts += 1
                    }
                }
            }
            if ok_to_continue {
                if parts == 0 {
                    set_error(factory, "Empty message.");
                    ok_to_continue = false;
                } else {
                    if !meta_part.is_null() {
                        mailmime_smart_add_part(message, meta_part);
                    }
                    if factory.msg.param.exists(Param::SetLatitude) {
                        let latitude = factory
                            .msg
                            .param
                            .get_float(Param::SetLatitude)
                            .unwrap_or_default();
                        let longitude = factory
                            .msg
                            .param
                            .get_float(Param::SetLongitude)
                            .unwrap_or_default();
                        let kml_file = location::get_message_kml(
                            factory.msg.timestamp_sort,
                            latitude,
                            longitude,
                        );
                        let content_type = mailmime_content_new_with_str(
                            b"application/vnd.google-earth.kml+xml\x00" as *const u8
                                as *const libc::c_char,
                        );
                        let mime_fields = mailmime_fields_new_filename(
                            MAILMIME_DISPOSITION_TYPE_ATTACHMENT as libc::c_int,
                            "message.kml".strdup(),
                            MAILMIME_MECHANISM_8BIT as libc::c_int,
                        );
                        let kml_mime_part = mailmime_new_empty(content_type, mime_fields);
                        set_body_text(kml_mime_part, &kml_file);
                        mailmime_smart_add_part(message, kml_mime_part);
                    }

                    if location::is_sending_locations_to_chat(context, factory.msg.chat_id) {
                        if let Ok((kml_file, last_added_location_id)) =
                            location::get_kml(context, factory.msg.chat_id)
                        {
                            let content_type = mailmime_content_new_with_str(
                                b"application/vnd.google-earth.kml+xml\x00" as *const u8
                                    as *const libc::c_char,
                            );
                            let mime_fields = mailmime_fields_new_filename(
                                MAILMIME_DISPOSITION_TYPE_ATTACHMENT as libc::c_int,
                                "location.kml".strdup(),
                                MAILMIME_MECHANISM_8BIT as libc::c_int,
                            );
                            let kml_mime_part = mailmime_new_empty(content_type, mime_fields);
                            set_body_text(kml_mime_part, &kml_file);
                            mailmime_smart_add_part(message, kml_mime_part);
                            if !factory.msg.param.exists(Param::SetLatitude) {
                                // otherwise, the independent location is already filed
                                factory.out_last_added_location_id = last_added_location_id;
                            }
                        }
                    }
                }
            }
        } else if factory.loaded == Loaded::MDN {
            let multipart: *mut mailmime =
                mailmime_multiple_new(b"multipart/report\x00" as *const u8 as *const libc::c_char);
            let content: *mut mailmime_content = (*multipart).mm_content_type;
            clist_insert_after(
                (*content).ct_parameters,
                (*(*content).ct_parameters).last,
                mailmime_param_new_with_data(
                    b"report-type\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
                    b"disposition-notification\x00" as *const u8 as *const libc::c_char
                        as *mut libc::c_char,
                ) as *mut libc::c_void,
            );
            mailmime_add_part(message, multipart);
            let p1 = if 0
                != factory
                    .msg
                    .param
                    .get_int(Param::GuranteeE2ee)
                    .unwrap_or_default()
            {
                factory
                    .context
                    .stock_str(StockMessage::EncryptedMsg)
                    .into_owned()
            } else {
                factory.msg.get_summarytext(context, 32)
            };
            let p2 = factory
                .context
                .stock_string_repl_str(StockMessage::ReadRcptMailBody, p1);
            let message_text = format!("{}\r\n", p2);
            let human_mime_part: *mut mailmime = build_body_text(&message_text);
            mailmime_add_part(multipart, human_mime_part);
            let version = get_version_str();
            let message_text2 = format!(
                "Reporting-UA: Delta Chat {}\r\nOriginal-Recipient: rfc822;{}\r\nFinal-Recipient: rfc822;{}\r\nOriginal-Message-ID: <{}>\r\nDisposition: manual-action/MDN-sent-automatically; displayed\r\n",
                version,
                as_str(factory.from_addr),
                as_str(factory.from_addr),
                factory.msg.rfc724_mid
            );

            let content_type_0: *mut mailmime_content = mailmime_content_new_with_str(
                b"message/disposition-notification\x00" as *const u8 as *const libc::c_char,
            );
            let mime_fields_0: *mut mailmime_fields =
                mailmime_fields_new_encoding(MAILMIME_MECHANISM_8BIT as libc::c_int);
            let mach_mime_part: *mut mailmime = mailmime_new_empty(content_type_0, mime_fields_0);
            set_body_text(mach_mime_part, &message_text2);
            mailmime_add_part(multipart, mach_mime_part);
            force_plaintext = 2;
        } else {
            set_error(factory, "No message loaded.");
            ok_to_continue = false;
        }

        if ok_to_continue {
            let subject_str = if factory.loaded == Loaded::MDN {
                let e = factory.context.stock_str(StockMessage::ReadRcpt);
                format!("Chat: {}", e)
            } else {
                get_subject(context, factory.chat.as_ref(), &mut factory.msg, afwd_email)
            };

            subject = mailimf_subject_new(dc_encode_header_words(subject_str));
            mailimf_fields_add(
                imf_fields,
                mailimf_field_new(
                    MAILIMF_FIELD_SUBJECT as libc::c_int,
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    subject,
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                ),
            );
            if force_plaintext != 2 {
                e2ee_helper.encrypt(
                    factory.context,
                    factory.recipients_addr,
                    force_plaintext,
                    e2ee_guaranteed,
                    min_verified,
                    do_gossip,
                    message,
                );
            }
            if e2ee_helper.encryption_successfull {
                factory.out_encrypted = true;
                if 0 != do_gossip {
                    factory.out_gossiped = true;
                }
            }
            factory.out = mmap_string_new(b"\x00" as *const u8 as *const libc::c_char);
            mailmime_write_mem(factory.out, &mut col, message);
            success = true;
        }
    }

    if !message.is_null() {
        mailmime_free(message);
    }
    e2ee_helper.thanks();
    success
}

fn get_subject(
    context: &Context,
    chat: Option<&Chat>,
    msg: &mut Message,
    afwd_email: bool,
) -> String {
    if chat.is_none() {
        return String::default();
    }

    let chat = chat.unwrap();

    let raw_subject =
        message::get_summarytext_by_raw(msg.type_0, msg.text.as_ref(), &mut msg.param, 32, context);

    let fwd = if afwd_email { "Fwd: " } else { "" };

    if msg.param.get_cmd() == SystemMessage::AutocryptSetupMessage {
        context
            .stock_str(StockMessage::AcSetupMsgSubject)
            .into_owned()
    } else if chat.typ == Chattype::Group || chat.typ == Chattype::VerifiedGroup {
        format!("Chat: {}: {}{}", chat.name, fwd, raw_subject,)
    } else {
        format!("Chat: {}{}", fwd, raw_subject)
    }
}

unsafe fn set_error(factory: *mut MimeFactory, text: &str) {
    if factory.is_null() {
        return;
    }
    free((*factory).error as *mut libc::c_void);
    (*factory).error = text.strdup();
}

pub unsafe fn add_mailimf_field(fields: *mut mailimf_fields, name: &str, value: &str) {
    let field = mailimf_field_new_custom(name.strdup(), value.strdup());
    let res = mailimf_fields_add(fields, field);
    assert!(
        res as u32 == MAILIMF_NO_ERROR,
        "could not create mailimf field"
    );
}

unsafe fn build_body_text(text: &str) -> *mut mailmime {
    let mime_fields: *mut mailmime_fields;
    let message_part: *mut mailmime;
    let content: *mut mailmime_content;
    content = mailmime_content_new_with_str(b"text/plain\x00" as *const u8 as *const libc::c_char);
    clist_insert_after(
        (*content).ct_parameters,
        (*(*content).ct_parameters).last,
        mailmime_param_new_with_data(
            b"charset\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
            b"utf-8\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
        ) as *mut libc::c_void,
    );
    mime_fields = mailmime_fields_new_encoding(MAILMIME_MECHANISM_8BIT as libc::c_int);
    message_part = mailmime_new_empty(content, mime_fields);
    set_body_text(message_part, text);

    message_part
}

unsafe fn set_body_text(part: *mut mailmime, text: &str) {
    mailmime_set_body_text(part, text.strdup(), text.len());
}

#[allow(non_snake_case)]
unsafe fn build_body_file(
    context: &Context,
    msg: &Message,
    base_name: &str,
) -> (*mut mailmime, String) {
    let needs_ext: bool;
    let mime_fields: *mut mailmime_fields;
    let mut mime_sub: *mut mailmime = ptr::null_mut();
    let content: *mut mailmime_content;
    let path_filename = msg.param.get(Param::File);
    let mut filename_to_send = "".to_string();

    let mut mimetype = msg
        .param
        .get(Param::MimeType)
        .map(|s| s.strdup())
        .unwrap_or_else(|| std::ptr::null_mut());

    let mut filename_encoded = ptr::null_mut();

    if let Some(ref path_filename) = path_filename {
        let suffix = dc_get_filesuffix_lc(path_filename);

        filename_to_send = if msg.type_0 == Viewtype::Voice {
            let ts = chrono::Utc.timestamp(msg.timestamp_sort as i64, 0);

            let suffix = if !suffix.is_null() {
                to_string(suffix)
            } else {
                "dat".into()
            };
            ts.format(&format!("voice-message_%Y-%m-%d_%H-%M-%S.{}", suffix))
                .to_string()
        } else if msg.type_0 == Viewtype::Audio {
            Path::new(path_filename)
                .file_name()
                .map(|c| c.to_string_lossy().to_string())
                .unwrap_or_default()
        } else if msg.type_0 == Viewtype::Image || msg.type_0 == Viewtype::Gif {
            format!(
                "{}.{}",
                if base_name.is_empty() {
                    "image"
                } else {
                    base_name
                },
                if !suffix.is_null() {
                    as_str(suffix)
                } else {
                    "dat"
                },
            )
        } else if msg.type_0 == Viewtype::Video {
            format!(
                "video.{}",
                if !suffix.is_null() {
                    as_str(suffix)
                } else {
                    "dat"
                },
            )
        } else {
            Path::new(path_filename)
                .file_name()
                .map(|c| c.to_string_lossy().to_string())
                .unwrap_or_default()
        };

        if mimetype.is_null() {
            if suffix.is_null() {
                mimetype = "application/octet-stream".strdup();
            } else if strcmp(suffix, b"png\x00" as *const u8 as *const libc::c_char) == 0 {
                mimetype = "image/png".strdup();
            } else if strcmp(suffix, b"jpg\x00" as *const u8 as *const libc::c_char) == 0
                || strcmp(suffix, b"jpeg\x00" as *const u8 as *const libc::c_char) == 0
                || strcmp(suffix, b"jpe\x00" as *const u8 as *const libc::c_char) == 0
            {
                mimetype = "image/jpeg".strdup();
            } else if strcmp(suffix, b"gif\x00" as *const u8 as *const libc::c_char) == 0 {
                mimetype = "image/gif".strdup();
            } else {
                mimetype = "application/octet-stream".strdup();
            }
        }
        if !mimetype.is_null() {
            /* create mime part, for Content-Disposition, see RFC 2183.
            `Content-Disposition: attachment` seems not to make a difference to `Content-Disposition: inline` at least on tested Thunderbird and Gma'l in 2017.
            But I've heard about problems with inline and outl'k, so we just use the attachment-type until we run into other problems ... */
            needs_ext = dc_needs_ext_header(&filename_to_send);
            mime_fields = mailmime_fields_new_filename(
                MAILMIME_DISPOSITION_TYPE_ATTACHMENT as libc::c_int,
                if needs_ext {
                    ptr::null_mut()
                } else {
                    filename_to_send.strdup()
                },
                MAILMIME_MECHANISM_BASE64 as libc::c_int,
            );
            if needs_ext {
                for cur_data in (*(*mime_fields).fld_list).into_iter() {
                    let field: *mut mailmime_field = cur_data as *mut _;
                    if (*field).fld_type == MAILMIME_FIELD_DISPOSITION as libc::c_int
                        && !(*field).fld_data.fld_disposition.is_null()
                    {
                        let file_disposition = (*field).fld_data.fld_disposition;
                        if !file_disposition.is_null() {
                            let parm = mailmime_disposition_parm_new(
                                MAILMIME_DISPOSITION_PARM_PARAMETER as libc::c_int,
                                ptr::null_mut(),
                                ptr::null_mut(),
                                ptr::null_mut(),
                                ptr::null_mut(),
                                0 as libc::size_t,
                                mailmime_parameter_new(
                                    strdup(b"filename*\x00" as *const u8 as *const libc::c_char),
                                    dc_encode_ext_header(&filename_to_send).strdup(),
                                ),
                            );
                            if !parm.is_null() {
                                clist_insert_after(
                                    (*file_disposition).dsp_parms,
                                    (*(*file_disposition).dsp_parms).last,
                                    parm as *mut libc::c_void,
                                );
                            }
                        }
                        break;
                    }
                }
            }
            content = mailmime_content_new_with_str(mimetype);
            filename_encoded = dc_encode_header_words(&filename_to_send);
            clist_insert_after(
                (*content).ct_parameters,
                (*(*content).ct_parameters).last,
                mailmime_param_new_with_data(
                    b"name\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
                    filename_encoded,
                ) as *mut libc::c_void,
            );
            mime_sub = mailmime_new_empty(content, mime_fields);
            let abs_path = dc_get_abs_path(context, path_filename)
                .to_c_string()
                .unwrap();
            mailmime_set_body_file(mime_sub, dc_strdup(abs_path.as_ptr()));
        }
    }

    free(mimetype as *mut libc::c_void);
    free(filename_encoded as *mut libc::c_void);

    (mime_sub, filename_to_send)
}

/*******************************************************************************
 * Render
 ******************************************************************************/
unsafe fn is_file_size_okay(context: &Context, msg: &Message) -> bool {
    let mut file_size_okay = true;
    let path = msg.param.get(Param::File).unwrap_or_default();
    let bytes = dc_get_filebytes(context, &path);

    if bytes > (49 * 1024 * 1024 / 4 * 3) {
        file_size_okay = false;
    }

    file_size_okay
}
