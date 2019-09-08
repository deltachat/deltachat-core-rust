use std::ffi::CString;
use std::ptr;

use chrono::TimeZone;
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
use crate::context::{dc_get_version_str, Context};
use crate::dc_strencode::*;
use crate::dc_tools::*;
use crate::e2ee::*;
use crate::error::Error;
use crate::location;
use crate::message::*;
use crate::param::*;
use crate::stock::StockMessage;
use crate::types::*;
use crate::x::*;

#[derive(Clone)]
#[allow(non_camel_case_types)]
pub struct dc_mimefactory_t<'a> {
    pub from_addr: *mut libc::c_char,
    pub from_displayname: *mut libc::c_char,
    pub selfstatus: *mut libc::c_char,
    pub recipients_names: *mut clist,
    pub recipients_addr: *mut clist,
    pub timestamp: i64,
    pub rfc724_mid: *mut libc::c_char,
    pub loaded: dc_mimefactory_loaded_t,
    pub msg: Message<'a>,
    pub chat: Option<Chat<'a>>,
    pub increation: libc::c_int,
    pub in_reply_to: *mut libc::c_char,
    pub references: *mut libc::c_char,
    pub req_mdn: libc::c_int,
    pub out: *mut MMAPString,
    pub out_encrypted: libc::c_int,
    pub out_gossiped: libc::c_int,
    pub out_last_added_location_id: uint32_t,
    pub error: *mut libc::c_char,
    pub context: &'a Context,
}

impl<'a> Drop for dc_mimefactory_t<'a> {
    fn drop(&mut self) {
        unsafe {
            free(self.from_addr as *mut libc::c_void);
            free(self.from_displayname as *mut libc::c_void);
            free(self.selfstatus as *mut libc::c_void);
            free(self.rfc724_mid as *mut libc::c_void);
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

#[allow(non_camel_case_types)]
type dc_mimefactory_loaded_t = libc::c_uint;
const DC_MF_MDN_LOADED: dc_mimefactory_loaded_t = 2;
pub const DC_MF_MSG_LOADED: dc_mimefactory_loaded_t = 1;
pub const DC_MF_NOTHING_LOADED: dc_mimefactory_loaded_t = 0;

pub unsafe fn dc_mimefactory_load_msg(
    context: &Context,
    msg_id: u32,
) -> Result<dc_mimefactory_t, Error> {
    ensure!(msg_id > DC_CHAT_ID_LAST_SPECIAL, "Invalid chat id");

    let msg = dc_msg_load_from_db(context, msg_id)?;
    let chat = Chat::load_from_db(context, msg.chat_id)?;
    let mut factory = dc_mimefactory_t {
        from_addr: ptr::null_mut(),
        from_displayname: ptr::null_mut(),
        selfstatus: ptr::null_mut(),
        recipients_names: clist_new(),
        recipients_addr: clist_new(),
        timestamp: 0,
        rfc724_mid: ptr::null_mut(),
        loaded: DC_MF_NOTHING_LOADED,
        msg,
        chat: Some(chat),
        increation: 0,
        in_reply_to: ptr::null_mut(),
        references: ptr::null_mut(),
        req_mdn: 0,
        out: ptr::null_mut(),
        out_encrypted: 0,
        out_gossiped: 0,
        out_last_added_location_id: 0,
        error: ptr::null_mut(),
        context,
    };

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

        let command = factory.msg.param.get_int(Param::Cmd).unwrap_or_default();
        let msg = &factory.msg;

        if command == 5 {
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
        if command != 6
            && command != 7
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
                0, "mimefactory: failed to load mime_in_reply_to: {:?}", err
            );
        }
    }

    factory.loaded = DC_MF_MSG_LOADED;
    factory.timestamp = factory.msg.timestamp_sort;
    factory.rfc724_mid = dc_strdup(factory.msg.rfc724_mid);
    factory.increation = dc_msg_is_increation(&factory.msg);

    Ok(factory)
}

unsafe fn load_from(factory: &mut dc_mimefactory_t) {
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

    factory.selfstatus = context
        .sql
        .get_config(context, "selfstatus")
        .unwrap_or_default()
        .strdup();
    if factory.selfstatus.is_null() {
        factory.selfstatus = factory.context.stock_str(StockMessage::StatusLine).strdup();
    };
}

pub unsafe fn dc_mimefactory_load_mdn<'a>(
    context: &'a Context,
    msg_id: uint32_t,
) -> Result<dc_mimefactory_t, Error> {
    if 0 == context
        .sql
        .get_config_int(context, "mdns_enabled")
        .unwrap_or_else(|| 1)
    {
        // MDNs not enabled - check this is late, in the job. the use may have changed its
        // choice while offline ...

        bail!("MDNs disabled ")
    }

    let msg = dc_msg_load_from_db(context, msg_id)?;

    let mut factory = dc_mimefactory_t {
        from_addr: ptr::null_mut(),
        from_displayname: ptr::null_mut(),
        selfstatus: ptr::null_mut(),
        recipients_names: clist_new(),
        recipients_addr: clist_new(),
        timestamp: 0,
        rfc724_mid: ptr::null_mut(),
        loaded: DC_MF_NOTHING_LOADED,
        msg,
        chat: None,
        increation: 0,
        in_reply_to: ptr::null_mut(),
        references: ptr::null_mut(),
        req_mdn: 0,
        out: ptr::null_mut(),
        out_encrypted: 0,
        out_gossiped: 0,
        out_last_added_location_id: 0,
        error: ptr::null_mut(),
        context,
    };

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
    factory.rfc724_mid = dc_create_outgoing_rfc724_mid(0 as *const libc::c_char, factory.from_addr);
    factory.loaded = DC_MF_MDN_LOADED;

    Ok(factory)
}

// TODO should return bool /rtn
pub unsafe fn dc_mimefactory_render(factory: &mut dc_mimefactory_t) -> libc::c_int {
    let subject: *mut mailimf_subject;
    let mut ok_to_continue = true;
    let imf_fields: *mut mailimf_fields;
    let mut message: *mut mailmime = ptr::null_mut();
    let mut message_text: *mut libc::c_char = ptr::null_mut();
    let mut message_text2: *mut libc::c_char = ptr::null_mut();
    let mut subject_str: *mut libc::c_char = ptr::null_mut();
    let mut afwd_email: libc::c_int = 0;
    let mut col: libc::c_int = 0;
    let mut success: libc::c_int = 0;
    let mut parts: libc::c_int = 0;
    let mut e2ee_guaranteed: libc::c_int = 0;
    let mut min_verified: libc::c_int = 0;
    // 1=add Autocrypt-header (needed eg. for handshaking), 2=no Autocrypte-header (used for MDN)
    let mut force_plaintext: libc::c_int = 0;
    let mut do_gossip: libc::c_int = 0;
    let mut grpimage = None;
    let mut e2ee_helper = E2eeHelper::default();

    if factory.loaded as libc::c_uint == DC_MF_NOTHING_LOADED as libc::c_int as libc::c_uint
        || !factory.out.is_null()
    {
        /*call empty() before*/
        set_error(
            factory,
            b"Invalid use of mimefactory-object.\x00" as *const u8 as *const libc::c_char,
        );
    } else {
        let from: *mut mailimf_mailbox_list = mailimf_mailbox_list_new_empty();
        mailimf_mailbox_list_add(
            from,
            mailimf_mailbox_new(
                if !factory.from_displayname.is_null() {
                    dc_encode_header_words(factory.from_displayname)
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
            let mut iter1: *mut clistiter;
            let mut iter2: *mut clistiter;
            to = mailimf_address_list_new_empty();
            iter1 = (*factory.recipients_names).first;
            iter2 = (*factory.recipients_addr).first;
            while !iter1.is_null() && !iter2.is_null() {
                let name: *const libc::c_char = (if !iter1.is_null() {
                    (*iter1).data
                } else {
                    ptr::null_mut()
                }) as *const libc::c_char;
                let addr: *const libc::c_char = (if !iter2.is_null() {
                    (*iter2).data
                } else {
                    ptr::null_mut()
                }) as *const libc::c_char;
                mailimf_address_list_add(
                    to,
                    mailimf_address_new(
                        MAILIMF_ADDRESS_MAILBOX as libc::c_int,
                        mailimf_mailbox_new(
                            if !name.is_null() {
                                dc_encode_header_words(name)
                            } else {
                                ptr::null_mut()
                            },
                            dc_strdup(addr),
                        ),
                        ptr::null_mut(),
                    ),
                );
                iter1 = if !iter1.is_null() {
                    (*iter1).next
                } else {
                    ptr::null_mut()
                };
                iter2 = if !iter2.is_null() {
                    (*iter2).next
                } else {
                    ptr::null_mut()
                }
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
            dc_strdup(factory.rfc724_mid),
            in_reply_to_list,
            references_list,
            ptr::null_mut(),
        );

        let os_name = &factory.context.os_name;
        let os_part = os_name
            .as_ref()
            .map(|s| format!("/{}", s))
            .unwrap_or_default();
        let os_part = CString::new(os_part).expect("String -> CString conversion failed");
        let version = dc_get_version_str();
        mailimf_fields_add(
            imf_fields,
            mailimf_field_new_custom(
                strdup(b"X-Mailer\x00" as *const u8 as *const libc::c_char),
                dc_mprintf(
                    b"Delta Chat Core %s%s\x00" as *const u8 as *const libc::c_char,
                    version,
                    os_part.as_ptr(),
                ),
            ),
        );
        free(version.cast());

        mailimf_fields_add(
            imf_fields,
            mailimf_field_new_custom(
                strdup(b"Chat-Version\x00" as *const u8 as *const libc::c_char),
                strdup(b"1.0\x00" as *const u8 as *const libc::c_char),
            ),
        );
        if 0 != factory.req_mdn {
            mailimf_fields_add(
                imf_fields,
                mailimf_field_new_custom(
                    strdup(
                        b"Chat-Disposition-Notification-To\x00" as *const u8 as *const libc::c_char,
                    ),
                    strdup(factory.from_addr),
                ),
            );
        }
        message = mailmime_new_message_data(0 as *mut mailmime);
        mailmime_set_imf_fields(message, imf_fields);
        if factory.loaded as libc::c_uint == DC_MF_MSG_LOADED as libc::c_int as libc::c_uint {
            /* Render a normal message
             *********************************************************************/
            let chat = factory.chat.as_ref().unwrap();
            let mut meta_part: *mut mailmime = ptr::null_mut();
            let mut placeholdertext: *mut libc::c_char = ptr::null_mut();
            if chat.typ == Chattype::VerifiedGroup {
                mailimf_fields_add(
                    imf_fields,
                    mailimf_field_new_custom(
                        strdup(b"Chat-Verified\x00" as *const u8 as *const libc::c_char),
                        strdup(b"1\x00" as *const u8 as *const libc::c_char),
                    ),
                );
                force_plaintext = 0;
                e2ee_guaranteed = 1;
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
                }
            }
            if chat.gossiped_timestamp == 0
                || (chat.gossiped_timestamp + (2 * 24 * 60 * 60)) < time()
            {
                do_gossip = 1
            }

            /* build header etc. */
            let command = factory.msg.param.get_int(Param::Cmd).unwrap_or_default();
            info!(
                factory.context,
                0, "render_message found command {}", command
            );
            if chat.typ == Chattype::Group || chat.typ == Chattype::VerifiedGroup {
                mailimf_fields_add(
                    imf_fields,
                    mailimf_field_new_custom(
                        strdup(b"Chat-Group-ID\x00" as *const u8 as *const libc::c_char),
                        chat.grpid.strdup(),
                    ),
                );
                let name = CString::yolo(chat.name.as_bytes());
                mailimf_fields_add(
                    imf_fields,
                    mailimf_field_new_custom(
                        strdup(b"Chat-Group-Name\x00" as *const u8 as *const libc::c_char),
                        dc_encode_header_words(name.as_ptr()),
                    ),
                );
                if command == DC_CMD_MEMBER_REMOVED_FROM_GROUP {
                    let email_to_remove = factory
                        .msg
                        .param
                        .get(Param::Arg)
                        .unwrap_or_default()
                        .strdup();
                    if strlen(email_to_remove) > 0 {
                        mailimf_fields_add(
                            imf_fields,
                            mailimf_field_new_custom(
                                strdup(
                                    b"Chat-Group-Member-Removed\x00" as *const u8
                                        as *const libc::c_char,
                                ),
                                email_to_remove,
                            ),
                        );
                    }
                } else if command == DC_CMD_MEMBER_ADDED_TO_GROUP {
                    let msg = &factory.msg;
                    do_gossip = 1;
                    let email_to_add = msg.param.get(Param::Arg).unwrap_or_default().strdup();
                    if strlen(email_to_add) > 0 {
                        mailimf_fields_add(
                            imf_fields,
                            mailimf_field_new_custom(
                                strdup(
                                    b"Chat-Group-Member-Added\x00" as *const u8
                                        as *const libc::c_char,
                                ),
                                email_to_add,
                            ),
                        );
                        grpimage = chat.param.get(Param::ProfileImage);
                    }
                    if 0 != msg.param.get_int(Param::Arg2).unwrap_or_default() & 0x1 {
                        info!(
                            msg.context,
                            0,
                            "sending secure-join message \'{}\' >>>>>>>>>>>>>>>>>>>>>>>>>",
                            "vg-member-added",
                        );
                        mailimf_fields_add(
                            imf_fields,
                            mailimf_field_new_custom(
                                strdup(b"Secure-Join\x00" as *const u8 as *const libc::c_char),
                                strdup(b"vg-member-added\x00" as *const u8 as *const libc::c_char),
                            ),
                        );
                    }
                } else if command == DC_CMD_GROUPNAME_CHANGED {
                    let msg = &factory.msg;

                    let value_to_add = msg.param.get(Param::Arg).unwrap_or_default().strdup();
                    mailimf_fields_add(
                        imf_fields,
                        mailimf_field_new_custom(
                            strdup(
                                b"Chat-Group-Name-Changed\x00" as *const u8 as *const libc::c_char,
                            ),
                            value_to_add,
                        ),
                    );
                } else if command == DC_CMD_GROUPIMAGE_CHANGED {
                    let msg = &factory.msg;
                    grpimage = msg.param.get(Param::Arg);
                    if grpimage.is_none() {
                        mailimf_fields_add(
                            imf_fields,
                            mailimf_field_new_custom(
                                strdup(b"Chat-Group-Image\x00" as *const u8 as *const libc::c_char),
                                dc_strdup(b"0\x00" as *const u8 as *const libc::c_char),
                            ),
                        );
                    }
                }
            }
            if command == DC_CMD_LOCATION_STREAMING_ENABLED {
                mailimf_fields_add(
                    imf_fields,
                    mailimf_field_new_custom(
                        strdup(b"Chat-Content\x00" as *const u8 as *const libc::c_char),
                        strdup(
                            b"location-streaming-enabled\x00" as *const u8 as *const libc::c_char,
                        ),
                    ),
                );
            }
            if command == DC_CMD_AUTOCRYPT_SETUP_MESSAGE {
                mailimf_fields_add(
                    imf_fields,
                    mailimf_field_new_custom(
                        strdup(b"Autocrypt-Setup-Message\x00" as *const u8 as *const libc::c_char),
                        strdup(b"v1\x00" as *const u8 as *const libc::c_char),
                    ),
                );
                placeholdertext = factory
                    .context
                    .stock_str(StockMessage::AcSetupMsgBody)
                    .strdup();
            }
            if command == DC_CMD_SECUREJOIN_MESSAGE {
                let msg = &factory.msg;
                let step = msg.param.get(Param::Arg).unwrap_or_default().strdup();
                if strlen(step) > 0 {
                    info!(
                        msg.context,
                        0,
                        "sending secure-join message \'{}\' >>>>>>>>>>>>>>>>>>>>>>>>>",
                        as_str(step),
                    );
                    mailimf_fields_add(
                        imf_fields,
                        mailimf_field_new_custom(
                            strdup(b"Secure-Join\x00" as *const u8 as *const libc::c_char),
                            step,
                        ),
                    );
                    let param2 = msg.param.get(Param::Arg2).unwrap_or_default().strdup();
                    if strlen(param2) > 0 {
                        mailimf_fields_add(
                            imf_fields,
                            mailimf_field_new_custom(
                                if strcmp(
                                    step,
                                    b"vg-request-with-auth\x00" as *const u8 as *const libc::c_char,
                                ) == 0
                                    || strcmp(
                                        step,
                                        b"vc-request-with-auth\x00" as *const u8
                                            as *const libc::c_char,
                                    ) == 0
                                {
                                    strdup(
                                        b"Secure-Join-Auth\x00" as *const u8 as *const libc::c_char,
                                    )
                                } else {
                                    strdup(
                                        b"Secure-Join-Invitenumber\x00" as *const u8
                                            as *const libc::c_char,
                                    )
                                },
                                param2,
                            ),
                        );
                    }
                    let fingerprint = msg.param.get(Param::Arg3).unwrap_or_default().strdup();
                    if strlen(fingerprint) > 0 {
                        mailimf_fields_add(
                            imf_fields,
                            mailimf_field_new_custom(
                                strdup(
                                    b"Secure-Join-Fingerprint\x00" as *const u8
                                        as *const libc::c_char,
                                ),
                                fingerprint,
                            ),
                        );
                    }
                    let grpid = match msg.param.get(Param::Arg4) {
                        Some(id) => id.strdup(),
                        None => std::ptr::null_mut(),
                    };
                    if !grpid.is_null() {
                        mailimf_fields_add(
                            imf_fields,
                            mailimf_field_new_custom(
                                strdup(
                                    b"Secure-Join-Group\x00" as *const u8 as *const libc::c_char,
                                ),
                                grpid,
                            ),
                        );
                    }
                }
            }
            info!(factory.context, 0, "grpimage {:?}", grpimage);
            if let Some(grpimage) = grpimage {
                info!(factory.context, 0, "setting group image");
                let mut meta = dc_msg_new_untyped(factory.context);
                meta.type_0 = Viewtype::Image;
                meta.param.set(Param::File, grpimage);

                let mut filename_as_sent = ptr::null_mut();
                meta_part = build_body_file(
                    &meta,
                    b"group-image\x00" as *const u8 as *const libc::c_char,
                    &mut filename_as_sent,
                );
                if !meta_part.is_null() {
                    mailimf_fields_add(
                        imf_fields,
                        mailimf_field_new_custom(
                            strdup(b"Chat-Group-Image\x00" as *const u8 as *const libc::c_char),
                            filename_as_sent,
                        ),
                    );
                }
            }

            if factory.msg.type_0 == Viewtype::Voice
                || factory.msg.type_0 == Viewtype::Audio
                || factory.msg.type_0 == Viewtype::Video
            {
                if factory.msg.type_0 == Viewtype::Voice {
                    mailimf_fields_add(
                        imf_fields,
                        mailimf_field_new_custom(
                            strdup(b"Chat-Voice-Message\x00" as *const u8 as *const libc::c_char),
                            strdup(b"1\x00" as *const u8 as *const libc::c_char),
                        ),
                    );
                }
                let duration_ms = factory
                    .msg
                    .param
                    .get_int(Param::Duration)
                    .unwrap_or_default();
                if duration_ms > 0 {
                    mailimf_fields_add(
                        imf_fields,
                        mailimf_field_new_custom(
                            strdup(b"Chat-Duration\x00" as *const u8 as *const libc::c_char),
                            dc_mprintf(
                                b"%i\x00" as *const u8 as *const libc::c_char,
                                duration_ms as libc::c_int,
                            ),
                        ),
                    );
                }
            }
            afwd_email = factory.msg.param.exists(Param::Forwarded) as libc::c_int;
            let mut fwdhint = ptr::null_mut();
            if 0 != afwd_email {
                fwdhint = dc_strdup(
                    b"---------- Forwarded message ----------\r\nFrom: Delta Chat\r\n\r\n\x00"
                        as *const u8 as *const libc::c_char,
                )
            }

            let final_text = {
                if !placeholdertext.is_null() {
                    to_string(placeholdertext)
                } else if let Some(ref text) = factory.msg.text {
                    text.clone()
                } else {
                    "".into()
                }
            };
            let final_text = CString::yolo(final_text);

            let footer: *mut libc::c_char = factory.selfstatus;
            message_text = dc_mprintf(
                b"%s%s%s%s%s\x00" as *const u8 as *const libc::c_char,
                if !fwdhint.is_null() {
                    fwdhint
                } else {
                    b"\x00" as *const u8 as *const libc::c_char
                },
                final_text.as_ptr(),
                if final_text != CString::yolo("")
                    && !footer.is_null()
                    && 0 != *footer.offset(0isize) as libc::c_int
                {
                    b"\r\n\r\n\x00" as *const u8 as *const libc::c_char
                } else {
                    b"\x00" as *const u8 as *const libc::c_char
                },
                if !footer.is_null() && 0 != *footer.offset(0isize) as libc::c_int {
                    b"-- \r\n\x00" as *const u8 as *const libc::c_char
                } else {
                    b"\x00" as *const u8 as *const libc::c_char
                },
                if !footer.is_null() && 0 != *footer.offset(0isize) as libc::c_int {
                    footer
                } else {
                    b"\x00" as *const u8 as *const libc::c_char
                },
            );
            let text_part: *mut mailmime = build_body_text(message_text);
            mailmime_smart_add_part(message, text_part);
            parts += 1;
            free(fwdhint as *mut libc::c_void);
            free(placeholdertext as *mut libc::c_void);

            /* add attachment part */
            if chat::msgtype_has_file(factory.msg.type_0) {
                if !is_file_size_okay(&factory.msg) {
                    let error: *mut libc::c_char = dc_mprintf(
                        b"Message exceeds the recommended %i MB.\x00" as *const u8
                            as *const libc::c_char,
                        24 * 1024 * 1024 / 4 * 3 / 1000 / 1000,
                    );
                    set_error(factory, error);
                    free(error as *mut libc::c_void);
                    ok_to_continue = false;
                } else {
                    let file_part: *mut mailmime =
                        build_body_file(&factory.msg, ptr::null(), ptr::null_mut());
                    if !file_part.is_null() {
                        mailmime_smart_add_part(message, file_part);
                        parts += 1
                    }
                }
            }
            if ok_to_continue {
                if parts == 0 {
                    set_error(
                        factory,
                        b"Empty message.\x00" as *const u8 as *const libc::c_char,
                    );
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
                            dc_strdup(b"message.kml\x00" as *const u8 as *const libc::c_char),
                            MAILMIME_MECHANISM_8BIT as libc::c_int,
                        );
                        let kml_mime_part = mailmime_new_empty(content_type, mime_fields);
                        mailmime_set_body_text(kml_mime_part, kml_file.strdup(), kml_file.len());
                        mailmime_smart_add_part(message, kml_mime_part);
                    }

                    if location::is_sending_locations_to_chat(
                        factory.msg.context,
                        factory.msg.chat_id,
                    ) {
                        if let Ok((kml_file, last_added_location_id)) =
                            location::get_kml(factory.msg.context, factory.msg.chat_id)
                        {
                            let content_type = mailmime_content_new_with_str(
                                b"application/vnd.google-earth.kml+xml\x00" as *const u8
                                    as *const libc::c_char,
                            );
                            let mime_fields = mailmime_fields_new_filename(
                                MAILMIME_DISPOSITION_TYPE_ATTACHMENT as libc::c_int,
                                dc_strdup(b"location.kml\x00" as *const u8 as *const libc::c_char),
                                MAILMIME_MECHANISM_8BIT as libc::c_int,
                            );
                            let kml_mime_part = mailmime_new_empty(content_type, mime_fields);
                            mailmime_set_body_text(
                                kml_mime_part,
                                kml_file.strdup(),
                                kml_file.len(),
                            );
                            mailmime_smart_add_part(message, kml_mime_part);
                            if !factory.msg.param.exists(Param::SetLatitude) {
                                // otherwise, the independent location is already filed
                                factory.out_last_added_location_id = last_added_location_id;
                            }
                        }
                    }
                }
            }
        } else if factory.loaded as libc::c_uint == DC_MF_MDN_LOADED as libc::c_int as libc::c_uint
        {
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
                to_string(dc_msg_get_summarytext(&mut factory.msg, 32))
            };
            let p2 = factory
                .context
                .stock_string_repl_str(StockMessage::ReadRcptMailBody, p1);
            message_text = format!("{}\r\n", p2).strdup();
            let human_mime_part: *mut mailmime = build_body_text(message_text);
            mailmime_add_part(multipart, human_mime_part);
            let version = dc_get_version_str();
            message_text2 =
                dc_mprintf(
                    b"Reporting-UA: Delta Chat %s\r\nOriginal-Recipient: rfc822;%s\r\nFinal-Recipient: rfc822;%s\r\nOriginal-Message-ID: <%s>\r\nDisposition: manual-action/MDN-sent-automatically; displayed\r\n\x00"
                        as *const u8 as *const libc::c_char,
                    version,
                    factory.from_addr, factory.from_addr,
                    factory.msg.rfc724_mid
                );
            free(version.cast());
            let content_type_0: *mut mailmime_content = mailmime_content_new_with_str(
                b"message/disposition-notification\x00" as *const u8 as *const libc::c_char,
            );
            let mime_fields_0: *mut mailmime_fields =
                mailmime_fields_new_encoding(MAILMIME_MECHANISM_8BIT as libc::c_int);
            let mach_mime_part: *mut mailmime = mailmime_new_empty(content_type_0, mime_fields_0);
            mailmime_set_body_text(mach_mime_part, message_text2, strlen(message_text2));
            mailmime_add_part(multipart, mach_mime_part);
            force_plaintext = 2;
        } else {
            set_error(
                factory,
                b"No message loaded.\x00" as *const u8 as *const libc::c_char,
            );
            ok_to_continue = false;
        }

        if ok_to_continue {
            if factory.loaded as libc::c_uint == DC_MF_MDN_LOADED as libc::c_int as libc::c_uint {
                let e = CString::new(factory.context.stock_str(StockMessage::ReadRcpt).as_ref())
                    .unwrap();
                subject_str = dc_mprintf(
                    b"Chat: %s\x00" as *const u8 as *const libc::c_char,
                    e.as_ptr(),
                );
            } else {
                subject_str = get_subject(factory.chat.as_ref(), &mut factory.msg, afwd_email)
            }
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
                factory.out_encrypted = 1;
                if 0 != do_gossip {
                    factory.out_gossiped = 1
                }
            }
            factory.out = mmap_string_new(b"\x00" as *const u8 as *const libc::c_char);
            mailmime_write_mem(factory.out, &mut col, message);
            success = 1;
        }
    }

    if !message.is_null() {
        mailmime_free(message);
    }
    e2ee_helper.thanks();
    free(message_text as *mut libc::c_void);
    free(message_text2 as *mut libc::c_void);
    free(subject_str as *mut libc::c_void);

    success
}

unsafe fn get_subject(
    chat: Option<&Chat>,
    msg: &mut Message,
    afwd_email: libc::c_int,
) -> *mut libc::c_char {
    if chat.is_none() {
        return std::ptr::null_mut();
    }

    let chat = chat.unwrap();
    let context = chat.context;
    let ret: *mut libc::c_char;

    let raw_subject = {
        dc_msg_get_summarytext_by_raw(msg.type_0, msg.text.as_ref(), &mut msg.param, 32, context)
            .strdup()
    };

    let fwd = if 0 != afwd_email {
        b"Fwd: \x00" as *const u8 as *const libc::c_char
    } else {
        b"\x00" as *const u8 as *const libc::c_char
    };
    if msg.param.get_int(Param::Cmd).unwrap_or_default() == 6 {
        ret = context.stock_str(StockMessage::AcSetupMsgSubject).strdup()
    } else if chat.typ == Chattype::Group || chat.typ == Chattype::VerifiedGroup {
        ret = format!(
            "Chat: {}: {}{}",
            chat.name,
            to_string(fwd),
            to_string(raw_subject),
        )
        .strdup()
    } else {
        ret = dc_mprintf(
            b"Chat: %s%s\x00" as *const u8 as *const libc::c_char,
            fwd,
            raw_subject,
        )
    }
    free(raw_subject as *mut libc::c_void);

    ret
}

unsafe fn set_error(factory: *mut dc_mimefactory_t, text: *const libc::c_char) {
    if factory.is_null() {
        return;
    }
    free((*factory).error as *mut libc::c_void);
    (*factory).error = dc_strdup_keep_null(text);
}

unsafe fn build_body_text(text: *mut libc::c_char) -> *mut mailmime {
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
    mailmime_set_body_text(message_part, text, strlen(text));

    message_part
}

#[allow(non_snake_case)]
unsafe fn build_body_file(
    msg: &Message,
    mut base_name: *const libc::c_char,
    ret_file_name_as_sent: *mut *mut libc::c_char,
) -> *mut mailmime {
    let needs_ext: bool;
    let mime_fields: *mut mailmime_fields;
    let mut mime_sub: *mut mailmime = ptr::null_mut();
    let content: *mut mailmime_content;
    let path_filename = msg.param.get(Param::File);

    let mut mimetype = msg
        .param
        .get(Param::MimeType)
        .map(|s| s.strdup())
        .unwrap_or_else(|| std::ptr::null_mut());

    let mut filename_to_send = ptr::null_mut();
    let mut filename_encoded = ptr::null_mut();

    if let Some(ref path_filename) = path_filename {
        let suffix = dc_get_filesuffix_lc(path_filename);

        if msg.type_0 == Viewtype::Voice {
            let ts = chrono::Utc.timestamp(msg.timestamp_sort as i64, 0);

            let suffix = if !suffix.is_null() {
                to_string(suffix)
            } else {
                "dat".into()
            };
            let res = ts
                .format(&format!("voice-message_%Y-%m-%d_%H-%M-%S.{}", suffix))
                .to_string();
            filename_to_send = res.strdup();
        } else if msg.type_0 == Viewtype::Audio {
            filename_to_send = dc_get_filename(path_filename)
        } else if msg.type_0 == Viewtype::Image || msg.type_0 == Viewtype::Gif {
            if base_name.is_null() {
                base_name = b"image\x00" as *const u8 as *const libc::c_char
            }
            filename_to_send = dc_mprintf(
                b"%s.%s\x00" as *const u8 as *const libc::c_char,
                base_name,
                if !suffix.is_null() {
                    suffix
                } else {
                    b"dat\x00" as *const u8 as *const libc::c_char
                },
            )
        } else if msg.type_0 == Viewtype::Video {
            filename_to_send = dc_mprintf(
                b"video.%s\x00" as *const u8 as *const libc::c_char,
                if !suffix.is_null() {
                    suffix
                } else {
                    b"dat\x00" as *const u8 as *const libc::c_char
                },
            )
        } else {
            filename_to_send = dc_get_filename(path_filename)
        }
        if mimetype.is_null() {
            if suffix.is_null() {
                mimetype =
                    dc_strdup(b"application/octet-stream\x00" as *const u8 as *const libc::c_char)
            } else if strcmp(suffix, b"png\x00" as *const u8 as *const libc::c_char) == 0 {
                mimetype = dc_strdup(b"image/png\x00" as *const u8 as *const libc::c_char)
            } else if strcmp(suffix, b"jpg\x00" as *const u8 as *const libc::c_char) == 0
                || strcmp(suffix, b"jpeg\x00" as *const u8 as *const libc::c_char) == 0
                || strcmp(suffix, b"jpe\x00" as *const u8 as *const libc::c_char) == 0
            {
                mimetype = dc_strdup(b"image/jpeg\x00" as *const u8 as *const libc::c_char)
            } else if strcmp(suffix, b"gif\x00" as *const u8 as *const libc::c_char) == 0 {
                mimetype = dc_strdup(b"image/gif\x00" as *const u8 as *const libc::c_char)
            } else {
                mimetype =
                    dc_strdup(b"application/octet-stream\x00" as *const u8 as *const libc::c_char)
            }
        }
        if !mimetype.is_null() {
            /* create mime part, for Content-Disposition, see RFC 2183.
            `Content-Disposition: attachment` seems not to make a difference to `Content-Disposition: inline` at least on tested Thunderbird and Gma'l in 2017.
            But I've heard about problems with inline and outl'k, so we just use the attachment-type until we run into other problems ... */
            needs_ext = dc_needs_ext_header(as_str(filename_to_send));
            mime_fields = mailmime_fields_new_filename(
                MAILMIME_DISPOSITION_TYPE_ATTACHMENT as libc::c_int,
                if needs_ext {
                    ptr::null_mut()
                } else {
                    dc_strdup(filename_to_send)
                },
                MAILMIME_MECHANISM_BASE64 as libc::c_int,
            );
            if needs_ext {
                let mut cur1: *mut clistiter = (*(*mime_fields).fld_list).first;
                while !cur1.is_null() {
                    let field: *mut mailmime_field = (if !cur1.is_null() {
                        (*cur1).data
                    } else {
                        ptr::null_mut()
                    }) as *mut mailmime_field;
                    if !field.is_null()
                        && (*field).fld_type == MAILMIME_FIELD_DISPOSITION as libc::c_int
                        && !(*field).fld_data.fld_disposition.is_null()
                    {
                        let file_disposition: *mut mailmime_disposition =
                            (*field).fld_data.fld_disposition;
                        if !file_disposition.is_null() {
                            let parm: *mut mailmime_disposition_parm =
                                mailmime_disposition_parm_new(
                                    MAILMIME_DISPOSITION_PARM_PARAMETER as libc::c_int,
                                    ptr::null_mut(),
                                    ptr::null_mut(),
                                    ptr::null_mut(),
                                    ptr::null_mut(),
                                    0 as size_t,
                                    mailmime_parameter_new(
                                        strdup(
                                            b"filename*\x00" as *const u8 as *const libc::c_char,
                                        ),
                                        dc_encode_ext_header(as_str(filename_to_send)).strdup(),
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
                    } else {
                        cur1 = if !cur1.is_null() {
                            (*cur1).next
                        } else {
                            ptr::null_mut()
                        }
                    }
                }
            }
            content = mailmime_content_new_with_str(mimetype);
            filename_encoded = dc_encode_header_words(filename_to_send);
            clist_insert_after(
                (*content).ct_parameters,
                (*(*content).ct_parameters).last,
                mailmime_param_new_with_data(
                    b"name\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
                    filename_encoded,
                ) as *mut libc::c_void,
            );
            mime_sub = mailmime_new_empty(content, mime_fields);
            mailmime_set_body_file(mime_sub, dc_get_abs_path(msg.context, path_filename));
            if !ret_file_name_as_sent.is_null() {
                *ret_file_name_as_sent = dc_strdup(filename_to_send)
            }
        }
    }

    free(mimetype as *mut libc::c_void);
    free(filename_to_send as *mut libc::c_void);
    free(filename_encoded as *mut libc::c_void);

    mime_sub
}

/*******************************************************************************
 * Render
 ******************************************************************************/
unsafe fn is_file_size_okay(msg: &Message) -> bool {
    let mut file_size_okay = true;
    let path = msg.param.get(Param::File).unwrap_or_default();
    let bytes = dc_get_filebytes(msg.context, &path);

    if bytes > (49 * 1024 * 1024 / 4 * 3) {
        file_size_okay = false;
    }

    file_size_okay
}
