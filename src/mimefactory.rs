use std::path::Path;
use std::ptr;

use chrono::TimeZone;
use mmime::clist::*;
use mmime::mailimf::types::*;
use mmime::mailimf::types_helper::*;
use mmime::mailmime::disposition::*;
use mmime::mailmime::types::*;
use mmime::mailmime::types_helper::*;
use mmime::mailmime::write_mem::*;
use mmime::mmapstring::*;
use mmime::other::*;

use crate::chat::{self, Chat};
use crate::config::Config;
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
use crate::wrapmime;

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum Loaded {
    Nothing,
    Message,
    MDN, // TODO: invent more descriptive name
}

#[derive(Clone)]
pub struct MimeFactory<'a> {
    pub from_addr: String,
    pub from_displayname: String,
    pub selfstatus: String,
    pub recipients_names: Vec<String>,
    pub recipients_addr: Vec<String>,
    pub timestamp: i64,
    pub rfc724_mid: String,
    pub loaded: Loaded,
    pub msg: Message,
    pub chat: Option<Chat>,
    pub increation: bool,
    pub in_reply_to: String,
    pub references: String,
    pub req_mdn: bool,
    pub out: *mut MMAPString,
    pub out_encrypted: bool,
    pub out_gossiped: bool,
    pub out_last_added_location_id: u32,
    pub context: &'a Context,
}

impl<'a> MimeFactory<'a> {
    fn new(context: &'a Context, msg: Message) -> Self {
        MimeFactory {
            from_addr: context
                .get_config(Config::ConfiguredAddr)
                .unwrap_or_default(),
            from_displayname: context.get_config(Config::Displayname).unwrap_or_default(),
            selfstatus: context
                .get_config(Config::Selfstatus)
                .unwrap_or_else(|| context.stock_str(StockMessage::StatusLine).to_string()),
            recipients_names: Vec::with_capacity(5),
            recipients_addr: Vec::with_capacity(5),
            timestamp: 0,
            rfc724_mid: String::default(),
            loaded: Loaded::Nothing,
            msg,
            chat: None,
            increation: false,
            in_reply_to: String::default(),
            references: String::default(),
            req_mdn: false,
            out: ptr::null_mut(),
            out_encrypted: false,
            out_gossiped: false,
            out_last_added_location_id: 0,
            context,
        }
    }

    pub fn finalize_mime_message(
        &mut self,
        message: *mut Mailmime,
        encrypted: bool,
        gossiped: bool,
    ) -> Result<(), Error> {
        unsafe {
            assert!(self.out.is_null()); // guard against double-calls
            self.out = mmap_string_new(b"\x00" as *const u8 as *const libc::c_char);
            let mut col: libc::c_int = 0;
            ensure_eq!(
                mailmime_write_mem(self.out, &mut col, message),
                0,
                "mem-error"
            );
        }
        self.out_encrypted = encrypted;
        self.out_gossiped = encrypted && gossiped;
        Ok(())
    }

    pub fn load_mdn(context: &'a Context, msg_id: u32) -> Result<MimeFactory, Error> {
        if !context.get_config_bool(Config::MdnsEnabled) {
            // MDNs not enabled - check this is late, in the job. the
            // user may have changed its choice while offline ...
            bail!("MDNs meanwhile disabled")
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

        factory
            .recipients_names
            .push(contact.get_authname().to_string());
        factory.recipients_addr.push(contact.get_addr().to_string());
        factory.timestamp = dc_create_smeared_timestamp(factory.context);
        factory.rfc724_mid = dc_create_outgoing_rfc724_mid(None, &factory.from_addr);
        factory.loaded = Loaded::MDN;

        Ok(factory)
    }

    /*******************************************************************************
     * Render a basic email
     ******************************************************************************/
    // XXX restrict unsafe to parts, introduce wrapmime helpers where appropriate
    pub unsafe fn render(&mut self) -> Result<(), Error> {
        if self.loaded == Loaded::Nothing || !self.out.is_null() {
            bail!("Invalid use of mimefactory-object.");
        }
        let context = &self.context;
        let from = wrapmime::new_mailbox_list(&self.from_displayname, &self.from_addr);

        let to = mailimf_address_list_new_empty();
        let name_iter = self.recipients_names.iter();
        let addr_iter = self.recipients_addr.iter();
        for (name, addr) in name_iter.zip(addr_iter) {
            mailimf_address_list_add(
                to,
                mailimf_address_new(
                    MAILIMF_ADDRESS_MAILBOX as libc::c_int,
                    mailimf_mailbox_new(
                        if !name.is_empty() {
                            dc_encode_header_words(&name).strdup()
                        } else {
                            ptr::null_mut()
                        },
                        addr.strdup(),
                    ),
                    ptr::null_mut(),
                ),
            );
        }
        let references_list = if !self.references.is_empty() {
            dc_str_to_clist(&self.references, " ")
        } else {
            ptr::null_mut()
        };
        let in_reply_to_list = if !self.in_reply_to.is_empty() {
            dc_str_to_clist(&self.in_reply_to, " ")
        } else {
            ptr::null_mut()
        };

        let imf_fields = mailimf_fields_new_with_data_all(
            mailimf_get_date(self.timestamp as i64),
            from,
            ptr::null_mut(),
            ptr::null_mut(),
            to,
            ptr::null_mut(),
            ptr::null_mut(),
            self.rfc724_mid.strdup(),
            in_reply_to_list,
            references_list,
            ptr::null_mut(),
        );

        let os_name = &self.context.os_name;
        let os_part = os_name
            .as_ref()
            .map(|s| format!("/{}", s))
            .unwrap_or_default();
        let version = get_version_str();
        let headerval = format!("Delta Chat Core {}{}", version, os_part);

        /* Add a X-Mailer header.
        This is only informational for debugging and may be removed in the release.
        We do not rely on this header as it may be removed by MTAs. */
        wrapmime::new_custom_field(imf_fields, "X-Mailer", &headerval);
        wrapmime::new_custom_field(imf_fields, "Chat-Version", "1.0");
        if self.req_mdn {
            /* we use "Chat-Disposition-Notification-To"
            because replies to "Disposition-Notification-To" are weird in many cases
            eg. are just freetext and/or do not follow any standard. */
            wrapmime::new_custom_field(
                imf_fields,
                "Chat-Disposition-Notification-To",
                &self.from_addr,
            );
        }

        let cleanup = |message: *mut Mailmime| {
            if !message.is_null() {
                mailmime_free(message);
            }
        };
        let message = mailmime_new_message_data(0 as *mut Mailmime);
        ensure!(!message.is_null(), "could not create mime message data");

        mailmime_set_imf_fields(message, imf_fields);

        // 1=add Autocrypt-header (needed eg. for handshaking), 2=no Autocrypte-header (used for MDN)
        let mut e2ee_guaranteed = false;
        let mut min_verified: libc::c_int = 0;
        let mut do_gossip = false;
        let mut grpimage = None;
        let force_plaintext: libc::c_int;
        let subject_str = match self.loaded {
            Loaded::Message => {
                /* Render a normal message
                 *********************************************************************/
                let chat = self.chat.as_ref().unwrap();
                let mut meta_part: *mut Mailmime = ptr::null_mut();
                let mut placeholdertext = None;

                if chat.typ == Chattype::VerifiedGroup {
                    wrapmime::new_custom_field(imf_fields, "Chat-Verified", "1");
                    force_plaintext = 0;
                    e2ee_guaranteed = true;
                    min_verified = 2
                } else {
                    force_plaintext = self
                        .msg
                        .param
                        .get_int(Param::ForcePlaintext)
                        .unwrap_or_default();
                    if force_plaintext == 0 {
                        e2ee_guaranteed = self
                            .msg
                            .param
                            .get_int(Param::GuranteeE2ee)
                            .unwrap_or_default()
                            != 0;
                    }
                }

                /* beside key- and member-changes, force re-gossip every 48 hours */
                if chat.gossiped_timestamp == 0
                    || (chat.gossiped_timestamp + (2 * 24 * 60 * 60)) < time()
                {
                    do_gossip = true
                }

                /* build header etc. */
                let command = self.msg.param.get_cmd();
                if chat.typ == Chattype::Group || chat.typ == Chattype::VerifiedGroup {
                    wrapmime::new_custom_field(imf_fields, "Chat-Group-ID", &chat.grpid);

                    let encoded = dc_encode_header_words(&chat.name);
                    wrapmime::new_custom_field(imf_fields, "Chat-Group-Name", &encoded);

                    match command {
                        SystemMessage::MemberRemovedFromGroup => {
                            let email_to_remove =
                                self.msg.param.get(Param::Arg).unwrap_or_default();
                            if !email_to_remove.is_empty() {
                                wrapmime::new_custom_field(
                                    imf_fields,
                                    "Chat-Group-Member-Removed",
                                    &email_to_remove,
                                );
                            }
                        }
                        SystemMessage::MemberAddedToGroup => {
                            let msg = &self.msg;
                            do_gossip = true;
                            let email_to_add = msg.param.get(Param::Arg).unwrap_or_default();
                            if !email_to_add.is_empty() {
                                wrapmime::new_custom_field(
                                    imf_fields,
                                    "Chat-Group-Member-Added",
                                    &email_to_add,
                                );
                                grpimage = chat.param.get(Param::ProfileImage);
                            }
                            if 0 != msg.param.get_int(Param::Arg2).unwrap_or_default() & 0x1 {
                                info!(
                                    context,
                                    "sending secure-join message \'{}\' >>>>>>>>>>>>>>>>>>>>>>>>>",
                                    "vg-member-added",
                                );
                                wrapmime::new_custom_field(
                                    imf_fields,
                                    "Secure-Join",
                                    "vg-member-added",
                                );
                            }
                        }
                        SystemMessage::GroupNameChanged => {
                            let msg = &self.msg;
                            let value_to_add = msg.param.get(Param::Arg).unwrap_or_default();

                            wrapmime::new_custom_field(
                                imf_fields,
                                "Chat-Group-Name-Changed",
                                &value_to_add,
                            );
                        }
                        SystemMessage::GroupImageChanged => {
                            let msg = &self.msg;
                            grpimage = msg.param.get(Param::Arg);
                            if grpimage.is_none() {
                                wrapmime::new_custom_field(imf_fields, "Chat-Group-Image", "0");
                            }
                        }
                        _ => {}
                    }
                }

                match command {
                    SystemMessage::LocationStreamingEnabled => {
                        wrapmime::new_custom_field(
                            imf_fields,
                            "Chat-Content",
                            "location-streaming-enabled",
                        );
                    }
                    SystemMessage::AutocryptSetupMessage => {
                        wrapmime::new_custom_field(imf_fields, "Autocrypt-Setup-Message", "v1");
                        placeholdertext = Some(
                            self.context
                                .stock_str(StockMessage::AcSetupMsgBody)
                                .to_string(),
                        );
                    }
                    SystemMessage::SecurejoinMessage => {
                        let msg = &self.msg;
                        let step = msg.param.get(Param::Arg).unwrap_or_default();
                        if !step.is_empty() {
                            info!(
                                context,
                                "sending secure-join message \'{}\' >>>>>>>>>>>>>>>>>>>>>>>>>",
                                step,
                            );
                            wrapmime::new_custom_field(imf_fields, "Secure-Join", &step);
                            let param2 = msg.param.get(Param::Arg2).unwrap_or_default();
                            if !param2.is_empty() {
                                wrapmime::new_custom_field(
                                    imf_fields,
                                    if step == "vg-request-with-auth"
                                        || step == "vc-request-with-auth"
                                    {
                                        "Secure-Join-Auth"
                                    } else {
                                        "Secure-Join-Invitenumber"
                                    },
                                    param2,
                                )
                            }
                            let fingerprint = msg.param.get(Param::Arg3).unwrap_or_default();
                            if !fingerprint.is_empty() {
                                wrapmime::new_custom_field(
                                    imf_fields,
                                    "Secure-Join-Fingerprint",
                                    &fingerprint,
                                );
                            }
                            if let Some(id) = msg.param.get(Param::Arg4) {
                                wrapmime::new_custom_field(imf_fields, "Secure-Join-Group", &id);
                            };
                        }
                    }
                    _ => {}
                }

                if let Some(grpimage) = grpimage {
                    info!(self.context, "setting group image '{}'", grpimage);
                    let mut meta = Message::default();
                    meta.type_0 = Viewtype::Image;
                    meta.param.set(Param::File, grpimage);

                    let res = build_body_file(context, &meta, "group-image")?;
                    meta_part = res.0;
                    let filename_as_sent = res.1;
                    if !meta_part.is_null() {
                        wrapmime::new_custom_field(
                            imf_fields,
                            "Chat-Group-Image",
                            &filename_as_sent,
                        )
                    }
                }

                if self.msg.type_0 == Viewtype::Sticker {
                    wrapmime::new_custom_field(imf_fields, "Chat-Content", "sticker");
                }

                if self.msg.type_0 == Viewtype::Voice
                    || self.msg.type_0 == Viewtype::Audio
                    || self.msg.type_0 == Viewtype::Video
                {
                    if self.msg.type_0 == Viewtype::Voice {
                        wrapmime::new_custom_field(imf_fields, "Chat-Voice-Message", "1");
                    }
                    let duration_ms = self.msg.param.get_int(Param::Duration).unwrap_or_default();
                    if duration_ms > 0 {
                        let dur = duration_ms.to_string();
                        wrapmime::new_custom_field(imf_fields, "Chat-Duration", &dur);
                    }
                }

                /* add text part - we even add empty text and force a MIME-multipart-message as:
                - some Apps have problems with Non-text in the main part (eg. "Mail" from stock Android)
                - we can add "forward hints" this way
                - it looks better */
                let afwd_email = self.msg.param.exists(Param::Forwarded);
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
                    } else if let Some(ref text) = self.msg.text {
                        text
                    } else {
                        ""
                    }
                };

                let footer = &self.selfstatus;
                let message_text = format!(
                    "{}{}{}{}{}",
                    fwdhint.unwrap_or_default(),
                    &final_text,
                    if !final_text.is_empty() && !footer.is_empty() {
                        "\r\n\r\n"
                    } else {
                        ""
                    },
                    if !footer.is_empty() { "-- \r\n" } else { "" },
                    footer
                );
                let text_part = wrapmime::build_body_text(&message_text)?;
                mailmime_smart_add_part(message, text_part);

                /* add attachment part */
                if chat::msgtype_has_file(self.msg.type_0) {
                    if !is_file_size_okay(context, &self.msg) {
                        cleanup(message);
                        bail!(
                            "Message exceeds the recommended {} MB.",
                            24 * 1024 * 1024 / 4 * 3 / 1000 / 1000,
                        );
                    } else {
                        let (file_part, _) = build_body_file(context, &self.msg, "")?;
                        mailmime_smart_add_part(message, file_part);
                    }
                }
                if !meta_part.is_null() {
                    mailmime_smart_add_part(message, meta_part);
                }
                if self.msg.param.exists(Param::SetLatitude) {
                    let param = &self.msg.param;
                    let kml_file = location::get_message_kml(
                        self.msg.timestamp_sort,
                        param.get_float(Param::SetLatitude).unwrap_or_default(),
                        param.get_float(Param::SetLongitude).unwrap_or_default(),
                    );
                    wrapmime::add_filename_part(
                        message,
                        "message.kml",
                        "application/vnd.google-earth.kml+xml",
                        &kml_file,
                    )?;
                }

                if location::is_sending_locations_to_chat(context, self.msg.chat_id) {
                    if let Ok((kml_file, last_added_location_id)) =
                        location::get_kml(context, self.msg.chat_id)
                    {
                        wrapmime::add_filename_part(
                            message,
                            "location.kml",
                            "application/vnd.google-earth.kml+xml",
                            &kml_file,
                        )?;
                        if !self.msg.param.exists(Param::SetLatitude) {
                            // otherwise, the independent location is already filed
                            self.out_last_added_location_id = last_added_location_id;
                        }
                    }
                }
                get_subject(context, self.chat.as_ref(), &mut self.msg, afwd_email)
            }
            Loaded::MDN => {
                /* Render a MDN
                 *********************************************************************/
                /* RFC 6522, this also requires the `report-type` parameter which is equal
                to the MIME subtype of the second body part of the multipart/report */
                let multipart = mailmime_multiple_new(
                    b"multipart/report\x00" as *const u8 as *const libc::c_char,
                );
                wrapmime::append_ct_param(
                    (*multipart).mm_content_type,
                    "report-type",
                    "disposition-notification",
                )?;

                mailmime_add_part(message, multipart);

                /* first body part: always human-readable, always REQUIRED by RFC 6522 */
                let p1 = if 0
                    != self
                        .msg
                        .param
                        .get_int(Param::GuranteeE2ee)
                        .unwrap_or_default()
                {
                    self.context
                        .stock_str(StockMessage::EncryptedMsg)
                        .into_owned()
                } else {
                    self.msg.get_summarytext(context, 32)
                };
                let p2 = self
                    .context
                    .stock_string_repl_str(StockMessage::ReadRcptMailBody, p1);
                let message_text = format!("{}\r\n", p2);
                let human_mime_part = wrapmime::build_body_text(&message_text)?;
                mailmime_add_part(multipart, human_mime_part);

                /* second body part: machine-readable, always REQUIRED by RFC 6522 */
                let version = get_version_str();
                let message_text2 = format!(
                    "Reporting-UA: Delta Chat {}\r\nOriginal-Recipient: rfc822;{}\r\nFinal-Recipient: rfc822;{}\r\nOriginal-Message-ID: <{}>\r\nDisposition: manual-action/MDN-sent-automatically; displayed\r\n",
                    version,
                    self.from_addr,
                    self.from_addr,
                    self.msg.rfc724_mid
                );

                let content_type_0 =
                    wrapmime::new_content_type("message/disposition-notification")?;
                let mime_fields_0: *mut mailmime_fields =
                    mailmime_fields_new_encoding(MAILMIME_MECHANISM_8BIT as libc::c_int);
                let mach_mime_part: *mut Mailmime =
                    mailmime_new_empty(content_type_0, mime_fields_0);
                wrapmime::set_body_text(mach_mime_part, &message_text2)?;
                mailmime_add_part(multipart, mach_mime_part);
                force_plaintext = DC_FP_NO_AUTOCRYPT_HEADER;
                /* currently, we do not send MDNs encrypted:
                - in a multi-device-setup that is not set up properly, MDNs would disturb the communication as they
                  are send automatically which may lead to spreading outdated Autocrypt headers.
                - they do not carry any information but the Message-ID
                - this save some KB
                - in older versions, we did not encrypt messages to ourself when they to to SMTP - however, if these messages
                  are forwarded for any reasons (eg. gmail always forwards to IMAP), we have no chance to decrypt them;
                  this issue is fixed with 0.9.4 */
                let e = self.context.stock_str(StockMessage::ReadRcpt);
                format!("Chat: {}", e).to_string()
            }
            _ => {
                cleanup(message);
                bail!("No message loaded.");
            }
        };

        /* Create the mime message
         *************************************************************************/

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
                mailimf_subject_new(dc_encode_header_words(subject_str).strdup()),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
            ),
        );

        /*just a pointer into mailmime structure, must not be freed*/
        let imffields_unprotected = wrapmime::mailmime_find_mailimf_fields(message);
        ensure!(
            !imffields_unprotected.is_null(),
            "could not find mime fields"
        );

        let mut encrypt_helper = EncryptHelper::new(&context)?;
        if force_plaintext != DC_FP_NO_AUTOCRYPT_HEADER {
            // unless determined otherwise we add Autocrypt header
            let aheader = encrypt_helper.get_aheader().to_string();
            wrapmime::new_custom_field(imffields_unprotected, "Autocrypt", &aheader);
        }
        let finalized = if force_plaintext == 0 {
            encrypt_helper.try_encrypt(
                self,
                e2ee_guaranteed,
                min_verified,
                do_gossip,
                message,
                imffields_unprotected,
            )?
        } else {
            false
        };
        if !finalized {
            self.finalize_mime_message(message, false, false)?;
        }
        cleanup(message);
        Ok(())
    }

    pub fn load_msg(context: &Context, msg_id: u32) -> Result<MimeFactory, Error> {
        ensure!(msg_id > DC_CHAT_ID_LAST_SPECIAL, "Invalid chat id");

        let msg = Message::load_from_db(context, msg_id)?;
        let chat = Chat::load_from_db(context, msg.chat_id)?;
        let mut factory = MimeFactory::new(context, msg);
        factory.chat = Some(chat);

        // just set the chat above
        let chat = factory.chat.as_ref().unwrap();

        if chat.is_self_talk() {
            factory
                .recipients_names
                .push(factory.from_displayname.to_string());
            factory.recipients_addr.push(factory.from_addr.to_string());
        } else {
            context.sql.query_map(
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
                        if !vec_contains_lowercase(&factory.recipients_addr, &addr) {
                            factory.recipients_addr.push(addr);
                            factory.recipients_names.push(authname);
                        }
                    }
                    Ok(())
                },
            )?;

            let command = factory.msg.param.get_cmd();
            let msg = &factory.msg;

            /* for added members, the list is just fine */
            if command == SystemMessage::MemberRemovedFromGroup {
                let email_to_remove = msg.param.get(Param::Arg).unwrap_or_default();

                let self_addr = context
                    .get_config(Config::ConfiguredAddr)
                    .unwrap_or_default();

                if !email_to_remove.is_empty() && email_to_remove != self_addr {
                    if !vec_contains_lowercase(&factory.recipients_addr, &email_to_remove) {
                        factory.recipients_names.push("".to_string());
                        factory.recipients_addr.push(email_to_remove.to_string());
                    }
                }
            }
            if command != SystemMessage::AutocryptSetupMessage
                && command != SystemMessage::SecurejoinMessage
                && context.get_config_bool(Config::MdnsEnabled)
            {
                factory.req_mdn = true;
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
                factory.in_reply_to = in_reply_to;
                factory.references = references;
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
}

impl<'a> Drop for MimeFactory<'a> {
    fn drop(&mut self) {
        unsafe {
            if !self.out.is_null() {
                mmap_string_free(self.out);
            }
        }
    }
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
        /* do not add the "Chat:" prefix for setup messages */
        context
            .stock_str(StockMessage::AcSetupMsgSubject)
            .into_owned()
    } else if chat.typ == Chattype::Group || chat.typ == Chattype::VerifiedGroup {
        format!("Chat: {}: {}{}", chat.name, fwd, raw_subject,)
    } else {
        format!("Chat: {}{}", fwd, raw_subject)
    }
}

#[allow(non_snake_case)]
fn build_body_file(
    context: &Context,
    msg: &Message,
    base_name: &str,
) -> Result<(*mut Mailmime, String), Error> {
    let path_filename = match msg.param.get(Param::File) {
        None => {
            bail!("msg has no filename");
        }
        Some(path) => path,
    };
    let suffix = dc_get_filesuffix_lc(path_filename).unwrap_or_else(|| "dat".into());

    /* get file name to use for sending
    (for privacy purposes, we do not transfer the original filenames eg. for images;
    these names are normally not needed and contain timestamps, running numbers etc.) */
    let filename_to_send = match msg.type_0 {
        Viewtype::Voice => chrono::Utc
            .timestamp(msg.timestamp_sort as i64, 0)
            .format(&format!("voice-message_%Y-%m-%d_%H-%M-%S.{}", suffix))
            .to_string(),
        Viewtype::Audio => Path::new(path_filename)
            .file_name()
            .map(|c| c.to_string_lossy().to_string())
            .unwrap_or_default(),
        Viewtype::Image | Viewtype::Gif => format!(
            "{}.{}",
            if base_name.is_empty() {
                "image"
            } else {
                base_name
            },
            &suffix,
        ),
        Viewtype::Video => format!("video.{}", &suffix),
        _ => Path::new(path_filename)
            .file_name()
            .map(|c| c.to_string_lossy().to_string())
            .unwrap_or_default(),
    };

    /* check mimetype */
    let mimetype = match msg.param.get(Param::MimeType) {
        Some(mtype) => mtype,
        None => {
            let path = Path::new(path_filename);
            if let Some(res) = message::guess_msgtype_from_suffix(&path) {
                res.1
            } else {
                "application/octet-stream"
            }
        }
    };

    let needs_ext = dc_needs_ext_header(&filename_to_send);

    unsafe {
        /* create mime part, for Content-Disposition, see RFC 2183.
        `Content-Disposition: attachment` seems not to make a difference to `Content-Disposition: inline` at least on tested Thunderbird and Gma'l in 2017.
        But I've heard about problems with inline and outl'k, so we just use the attachment-type until we run into other problems ... */
        let mime_fields = mailmime_fields_new_filename(
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
        let content = wrapmime::new_content_type(&mimetype)?;
        let filename_encoded = dc_encode_header_words(&filename_to_send);
        wrapmime::append_ct_param(content, "name", &filename_encoded)?;

        let mime_sub = mailmime_new_empty(content, mime_fields);
        let abs_path = dc_get_abs_path(context, path_filename).to_c_string()?;
        mailmime_set_body_file(mime_sub, dc_strdup(abs_path.as_ptr()));
        Ok((mime_sub, filename_to_send))
    }
}

pub(crate) fn vec_contains_lowercase(vec: &[String], part: &str) -> bool {
    let partlc = part.to_lowercase();
    for cur in vec.iter() {
        if cur.to_lowercase() == partlc {
            return true;
        }
    }
    false
}

fn is_file_size_okay(context: &Context, msg: &Message) -> bool {
    let mut file_size_okay = true;
    let path = msg.param.get(Param::File).unwrap_or_default();
    let bytes = dc_get_filebytes(context, &path);

    if bytes > (49 * 1024 * 1024 / 4 * 3) {
        file_size_okay = false;
    }

    file_size_okay
}
