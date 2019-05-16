use mmime::mailimf_types::*;
use mmime::mailimf_types_helper::*;
use mmime::mailmime_disposition::*;
use mmime::mailmime_types::*;
use mmime::mailmime_types_helper::*;
use mmime::mailmime_write_mem::*;
use mmime::mmapstring::*;
use mmime::other::*;

use crate::constants::VERSION;
use crate::dc_chat::*;
use crate::dc_contact::*;
use crate::dc_context::dc_context_t;
use crate::dc_e2ee::*;
use crate::dc_hash::*;
use crate::dc_location::*;
use crate::dc_log::*;
use crate::dc_msg::*;
use crate::dc_param::*;
use crate::dc_sqlite3::*;
use crate::dc_stock::*;
use crate::dc_strencode::*;
use crate::dc_tools::*;
use crate::types::*;
use crate::x::*;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct dc_mimefactory_t<'a> {
    pub from_addr: *mut libc::c_char,
    pub from_displayname: *mut libc::c_char,
    pub selfstatus: *mut libc::c_char,
    pub recipients_names: *mut clist,
    pub recipients_addr: *mut clist,
    pub timestamp: time_t,
    pub rfc724_mid: *mut libc::c_char,
    pub loaded: dc_mimefactory_loaded_t,
    pub msg: *mut dc_msg_t<'a>,
    pub chat: *mut dc_chat_t<'a>,
    pub increation: libc::c_int,
    pub in_reply_to: *mut libc::c_char,
    pub references: *mut libc::c_char,
    pub req_mdn: libc::c_int,
    pub out: *mut MMAPString,
    pub out_encrypted: libc::c_int,
    pub out_gossiped: libc::c_int,
    pub out_last_added_location_id: uint32_t,
    pub error: *mut libc::c_char,
    pub context: &'a dc_context_t,
}

pub type dc_mimefactory_loaded_t = libc::c_uint;
pub const DC_MF_MDN_LOADED: dc_mimefactory_loaded_t = 2;
pub const DC_MF_MSG_LOADED: dc_mimefactory_loaded_t = 1;
pub const DC_MF_NOTHING_LOADED: dc_mimefactory_loaded_t = 0;

pub unsafe fn dc_mimefactory_init<'a>(
    factory: *mut dc_mimefactory_t<'a>,
    context: &'a dc_context_t,
) {
    if factory.is_null() {
        return;
    }
    memset(
        factory as *mut libc::c_void,
        0,
        ::std::mem::size_of::<dc_mimefactory_t>(),
    );
    (*factory).context = context;
}

pub unsafe fn dc_mimefactory_empty(mut factory: *mut dc_mimefactory_t) {
    if factory.is_null() {
        return;
    }
    free((*factory).from_addr as *mut libc::c_void);
    (*factory).from_addr = 0 as *mut libc::c_char;
    free((*factory).from_displayname as *mut libc::c_void);
    (*factory).from_displayname = 0 as *mut libc::c_char;
    free((*factory).selfstatus as *mut libc::c_void);
    (*factory).selfstatus = 0 as *mut libc::c_char;
    free((*factory).rfc724_mid as *mut libc::c_void);
    (*factory).rfc724_mid = 0 as *mut libc::c_char;
    if !(*factory).recipients_names.is_null() {
        clist_free_content((*factory).recipients_names);
        clist_free((*factory).recipients_names);
        (*factory).recipients_names = 0 as *mut clist
    }
    if !(*factory).recipients_addr.is_null() {
        clist_free_content((*factory).recipients_addr);
        clist_free((*factory).recipients_addr);
        (*factory).recipients_addr = 0 as *mut clist
    }
    dc_msg_unref((*factory).msg);
    (*factory).msg = 0 as *mut dc_msg_t;
    dc_chat_unref((*factory).chat);
    (*factory).chat = 0 as *mut dc_chat_t;
    free((*factory).in_reply_to as *mut libc::c_void);
    (*factory).in_reply_to = 0 as *mut libc::c_char;
    free((*factory).references as *mut libc::c_void);
    (*factory).references = 0 as *mut libc::c_char;
    if !(*factory).out.is_null() {
        mmap_string_free((*factory).out);
        (*factory).out = 0 as *mut MMAPString
    }
    (*factory).out_encrypted = 0i32;
    (*factory).loaded = DC_MF_NOTHING_LOADED;
    free((*factory).error as *mut libc::c_void);
    (*factory).error = 0 as *mut libc::c_char;
    (*factory).timestamp = 0i32 as time_t;
}

pub unsafe fn dc_mimefactory_load_msg(
    mut factory: *mut dc_mimefactory_t,
    msg_id: uint32_t,
) -> libc::c_int {
    let mut success: libc::c_int = 0i32;
    let mut stmt: *mut sqlite3_stmt = 0 as *mut sqlite3_stmt;
    if !(factory.is_null() || msg_id <= 9i32 as libc::c_uint || !(*factory).msg.is_null()) {
        /*call empty() before */
        let context = (*factory).context;
        (*factory).recipients_names = clist_new();
        (*factory).recipients_addr = clist_new();
        (*factory).msg = dc_msg_new_untyped(context);
        (*factory).chat = dc_chat_new(context);
        if dc_msg_load_from_db((*factory).msg, context, msg_id)
            && dc_chat_load_from_db((*factory).chat, (*(*factory).msg).chat_id)
        {
            load_from(factory);
            (*factory).req_mdn = 0i32;
            if 0 != dc_chat_is_self_talk((*factory).chat) {
                clist_insert_after(
                    (*factory).recipients_names,
                    (*(*factory).recipients_names).last,
                    dc_strdup_keep_null((*factory).from_displayname) as *mut libc::c_void,
                );
                clist_insert_after(
                    (*factory).recipients_addr,
                    (*(*factory).recipients_addr).last,
                    dc_strdup((*factory).from_addr) as *mut libc::c_void,
                );
            } else {
                stmt =
                    dc_sqlite3_prepare(
                        context,
                        &context.sql.clone().read().unwrap(),
                        b"SELECT c.authname, c.addr  FROM chats_contacts cc  LEFT JOIN contacts c ON cc.contact_id=c.id  WHERE cc.chat_id=? AND cc.contact_id>9;\x00"
                            as *const u8 as
                            *const libc::c_char);
                sqlite3_bind_int(stmt, 1i32, (*(*factory).msg).chat_id as libc::c_int);
                while sqlite3_step(stmt) == 100i32 {
                    let authname: *const libc::c_char =
                        sqlite3_column_text(stmt, 0i32) as *const libc::c_char;
                    let addr: *const libc::c_char =
                        sqlite3_column_text(stmt, 1i32) as *const libc::c_char;
                    if clist_search_string_nocase((*factory).recipients_addr, addr) == 0i32 {
                        clist_insert_after(
                            (*factory).recipients_names,
                            (*(*factory).recipients_names).last,
                            (if !authname.is_null() && 0 != *authname.offset(0isize) as libc::c_int
                            {
                                dc_strdup(authname)
                            } else {
                                0 as *mut libc::c_char
                            }) as *mut libc::c_void,
                        );
                        clist_insert_after(
                            (*factory).recipients_addr,
                            (*(*factory).recipients_addr).last,
                            dc_strdup(addr) as *mut libc::c_void,
                        );
                    }
                }
                sqlite3_finalize(stmt);
                let command: libc::c_int =
                    dc_param_get_int((*(*factory).msg).param, 'S' as i32, 0i32);
                if command == 5i32 {
                    let email_to_remove: *mut libc::c_char = dc_param_get(
                        (*(*factory).msg).param,
                        'E' as i32,
                        0 as *const libc::c_char,
                    );
                    let self_addr: *mut libc::c_char = dc_sqlite3_get_config(
                        context,
                        &context.sql.clone().read().unwrap(),
                        b"configured_addr\x00" as *const u8 as *const libc::c_char,
                        b"\x00" as *const u8 as *const libc::c_char,
                    );
                    if !email_to_remove.is_null() && strcasecmp(email_to_remove, self_addr) != 0i32
                    {
                        if clist_search_string_nocase((*factory).recipients_addr, email_to_remove)
                            == 0i32
                        {
                            clist_insert_after(
                                (*factory).recipients_names,
                                (*(*factory).recipients_names).last,
                                0 as *mut libc::c_void,
                            );
                            clist_insert_after(
                                (*factory).recipients_addr,
                                (*(*factory).recipients_addr).last,
                                email_to_remove as *mut libc::c_void,
                            );
                        }
                    }
                    free(self_addr as *mut libc::c_void);
                }
                if command != 6i32
                    && command != 7i32
                    && 0 != dc_sqlite3_get_config_int(
                        context,
                        &context.sql.clone().read().unwrap(),
                        b"mdns_enabled\x00" as *const u8 as *const libc::c_char,
                        1i32,
                    )
                {
                    (*factory).req_mdn = 1i32
                }
            }
            stmt = dc_sqlite3_prepare(
                context,
                &context.sql.clone().read().unwrap(),
                b"SELECT mime_in_reply_to, mime_references FROM msgs WHERE id=?\x00" as *const u8
                    as *const libc::c_char,
            );
            sqlite3_bind_int(stmt, 1i32, (*(*factory).msg).id as libc::c_int);
            if sqlite3_step(stmt) == 100i32 {
                (*factory).in_reply_to =
                    dc_strdup(sqlite3_column_text(stmt, 0i32) as *const libc::c_char);
                (*factory).references =
                    dc_strdup(sqlite3_column_text(stmt, 1i32) as *const libc::c_char)
            }
            sqlite3_finalize(stmt);
            stmt = 0 as *mut sqlite3_stmt;
            success = 1i32;
            (*factory).loaded = DC_MF_MSG_LOADED;
            (*factory).timestamp = (*(*factory).msg).timestamp_sort;
            (*factory).rfc724_mid = dc_strdup((*(*factory).msg).rfc724_mid)
        }
        if 0 != success {
            (*factory).increation = dc_msg_is_increation((*factory).msg)
        }
    }
    sqlite3_finalize(stmt);
    return success;
}

unsafe fn load_from(mut factory: *mut dc_mimefactory_t) {
    (*factory).from_addr = dc_sqlite3_get_config(
        (*factory).context,
        &mut (*factory).context.sql.clone().read().unwrap(),
        b"configured_addr\x00" as *const u8 as *const libc::c_char,
        0 as *const libc::c_char,
    );
    (*factory).from_displayname = dc_sqlite3_get_config(
        (*factory).context,
        &mut (*factory).context.sql.clone().read().unwrap(),
        b"displayname\x00" as *const u8 as *const libc::c_char,
        0 as *const libc::c_char,
    );
    (*factory).selfstatus = dc_sqlite3_get_config(
        (*factory).context,
        &mut (*factory).context.sql.clone().read().unwrap(),
        b"selfstatus\x00" as *const u8 as *const libc::c_char,
        0 as *const libc::c_char,
    );
    if (*factory).selfstatus.is_null() {
        (*factory).selfstatus = dc_stock_str((*factory).context, 13i32)
    };
}

pub unsafe fn dc_mimefactory_load_mdn(
    mut factory: *mut dc_mimefactory_t,
    msg_id: uint32_t,
) -> libc::c_int {
    let mut success: libc::c_int = 0i32;
    let mut contact: *mut dc_contact_t = 0 as *mut dc_contact_t;
    if !factory.is_null() {
        (*factory).recipients_names = clist_new();
        (*factory).recipients_addr = clist_new();
        (*factory).msg = dc_msg_new_untyped((*factory).context);
        if !(0
            == dc_sqlite3_get_config_int(
                (*factory).context,
                &mut (*factory).context.sql.clone().read().unwrap(),
                b"mdns_enabled\x00" as *const u8 as *const libc::c_char,
                1i32,
            ))
        {
            /* MDNs not enabled - check this is late, in the job. the use may have changed its choice while offline ... */
            contact = dc_contact_new((*factory).context);
            if !(!dc_msg_load_from_db((*factory).msg, (*factory).context, msg_id)
                || !dc_contact_load_from_db(
                    contact,
                    &mut (*factory).context.sql.clone().read().unwrap(),
                    (*(*factory).msg).from_id,
                ))
            {
                if !(0 != (*contact).blocked || (*(*factory).msg).chat_id <= 9i32 as libc::c_uint) {
                    /* Do not send MDNs trash etc.; chats.blocked is already checked by the caller in dc_markseen_msgs() */
                    if !((*(*factory).msg).from_id <= 9i32 as libc::c_uint) {
                        clist_insert_after(
                            (*factory).recipients_names,
                            (*(*factory).recipients_names).last,
                            (if !(*contact).authname.is_null()
                                && 0 != *(*contact).authname.offset(0isize) as libc::c_int
                            {
                                dc_strdup((*contact).authname)
                            } else {
                                0 as *mut libc::c_char
                            }) as *mut libc::c_void,
                        );
                        clist_insert_after(
                            (*factory).recipients_addr,
                            (*(*factory).recipients_addr).last,
                            dc_strdup((*contact).addr) as *mut libc::c_void,
                        );
                        load_from(factory);
                        (*factory).timestamp = dc_create_smeared_timestamp((*factory).context);
                        (*factory).rfc724_mid = dc_create_outgoing_rfc724_mid(
                            0 as *const libc::c_char,
                            (*factory).from_addr,
                        );
                        success = 1i32;
                        (*factory).loaded = DC_MF_MDN_LOADED
                    }
                }
            }
        }
    }
    dc_contact_unref(contact);

    success
}

// TODO should return bool /rtn
pub unsafe fn dc_mimefactory_render(mut factory: *mut dc_mimefactory_t) -> libc::c_int {
    let subject: *mut mailimf_subject;
    let mut current_block: u64;
    let imf_fields: *mut mailimf_fields;
    let mut message: *mut mailmime = 0 as *mut mailmime;
    let mut message_text: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut message_text2: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut subject_str: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut afwd_email: libc::c_int = 0i32;
    let mut col: libc::c_int = 0i32;
    let mut success: libc::c_int = 0i32;
    let mut parts: libc::c_int = 0i32;
    let mut e2ee_guaranteed: libc::c_int = 0i32;
    let mut min_verified: libc::c_int = 0i32;
    // 1=add Autocrypt-header (needed eg. for handshaking), 2=no Autocrypte-header (used for MDN)
    let mut force_plaintext: libc::c_int = 0i32;
    let mut do_gossip: libc::c_int = 0i32;
    let mut grpimage: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut e2ee_helper: dc_e2ee_helper_t = dc_e2ee_helper_t {
        encryption_successfull: 0,
        cdata_to_free: 0 as *mut libc::c_void,
        encrypted: 0,
        signatures: 0 as *mut dc_hash_t,
        gossipped_addr: 0 as *mut dc_hash_t,
    };
    memset(
        &mut e2ee_helper as *mut dc_e2ee_helper_t as *mut libc::c_void,
        0,
        ::std::mem::size_of::<dc_e2ee_helper_t>(),
    );
    if factory.is_null()
        || (*factory).loaded as libc::c_uint == DC_MF_NOTHING_LOADED as libc::c_int as libc::c_uint
        || !(*factory).out.is_null()
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
                if !(*factory).from_displayname.is_null() {
                    dc_encode_header_words((*factory).from_displayname)
                } else {
                    0 as *mut libc::c_char
                },
                dc_strdup((*factory).from_addr),
            ),
        );
        let mut to: *mut mailimf_address_list = 0 as *mut mailimf_address_list;
        if !(*factory).recipients_names.is_null()
            && !(*factory).recipients_addr.is_null()
            && (*(*factory).recipients_addr).count > 0i32
        {
            let mut iter1: *mut clistiter;
            let mut iter2: *mut clistiter;
            to = mailimf_address_list_new_empty();
            iter1 = (*(*factory).recipients_names).first;
            iter2 = (*(*factory).recipients_addr).first;
            while !iter1.is_null() && !iter2.is_null() {
                let name: *const libc::c_char = (if !iter1.is_null() {
                    (*iter1).data
                } else {
                    0 as *mut libc::c_void
                }) as *const libc::c_char;
                let addr: *const libc::c_char = (if !iter2.is_null() {
                    (*iter2).data
                } else {
                    0 as *mut libc::c_void
                }) as *const libc::c_char;
                mailimf_address_list_add(
                    to,
                    mailimf_address_new(
                        MAILIMF_ADDRESS_MAILBOX as libc::c_int,
                        mailimf_mailbox_new(
                            if !name.is_null() {
                                dc_encode_header_words(name)
                            } else {
                                0 as *mut libc::c_char
                            },
                            dc_strdup(addr),
                        ),
                        0 as *mut mailimf_group,
                    ),
                );
                iter1 = if !iter1.is_null() {
                    (*iter1).next
                } else {
                    0 as *mut clistcell
                };
                iter2 = if !iter2.is_null() {
                    (*iter2).next
                } else {
                    0 as *mut clistcell
                }
            }
        }
        let mut references_list: *mut clist = 0 as *mut clist;
        if !(*factory).references.is_null()
            && 0 != *(*factory).references.offset(0isize) as libc::c_int
        {
            references_list = dc_str_to_clist(
                (*factory).references,
                b" \x00" as *const u8 as *const libc::c_char,
            )
        }
        let mut in_reply_to_list: *mut clist = 0 as *mut clist;
        if !(*factory).in_reply_to.is_null()
            && 0 != *(*factory).in_reply_to.offset(0isize) as libc::c_int
        {
            in_reply_to_list = dc_str_to_clist(
                (*factory).in_reply_to,
                b" \x00" as *const u8 as *const libc::c_char,
            )
        }
        imf_fields = mailimf_fields_new_with_data_all(
            mailimf_get_date((*factory).timestamp),
            from,
            0 as *mut mailimf_mailbox,
            0 as *mut mailimf_address_list,
            to,
            0 as *mut mailimf_address_list,
            0 as *mut mailimf_address_list,
            dc_strdup((*factory).rfc724_mid),
            in_reply_to_list,
            references_list,
            0 as *mut libc::c_char,
        );
        mailimf_fields_add(
            imf_fields,
            mailimf_field_new_custom(
                strdup(b"X-Mailer\x00" as *const u8 as *const libc::c_char),
                dc_mprintf(
                    b"Delta Chat Core %s%s%s\x00" as *const u8 as *const libc::c_char,
                    VERSION as *const u8 as *const libc::c_char,
                    if !(*(*factory).context).os_name.is_null() {
                        b"/\x00" as *const u8 as *const libc::c_char
                    } else {
                        b"\x00" as *const u8 as *const libc::c_char
                    },
                    if !(*(*factory).context).os_name.is_null() {
                        (*(*factory).context).os_name
                    } else {
                        b"\x00" as *const u8 as *const libc::c_char
                    },
                ),
            ),
        );
        mailimf_fields_add(
            imf_fields,
            mailimf_field_new_custom(
                strdup(b"Chat-Version\x00" as *const u8 as *const libc::c_char),
                strdup(b"1.0\x00" as *const u8 as *const libc::c_char),
            ),
        );
        if 0 != (*factory).req_mdn {
            mailimf_fields_add(
                imf_fields,
                mailimf_field_new_custom(
                    strdup(
                        b"Chat-Disposition-Notification-To\x00" as *const u8 as *const libc::c_char,
                    ),
                    strdup((*factory).from_addr),
                ),
            );
        }
        message = mailmime_new_message_data(0 as *mut mailmime);
        mailmime_set_imf_fields(message, imf_fields);
        if (*factory).loaded as libc::c_uint == DC_MF_MSG_LOADED as libc::c_int as libc::c_uint {
            /* Render a normal message
             *********************************************************************/
            let chat: *mut dc_chat_t = (*factory).chat;
            let msg: *mut dc_msg_t = (*factory).msg;
            let mut meta_part: *mut mailmime = 0 as *mut mailmime;
            let mut placeholdertext: *mut libc::c_char = 0 as *mut libc::c_char;
            if (*chat).type_0 == 130i32 {
                mailimf_fields_add(
                    imf_fields,
                    mailimf_field_new_custom(
                        strdup(b"Chat-Verified\x00" as *const u8 as *const libc::c_char),
                        strdup(b"1\x00" as *const u8 as *const libc::c_char),
                    ),
                );
                force_plaintext = 0i32;
                e2ee_guaranteed = 1i32;
                min_verified = 2i32
            } else {
                force_plaintext = dc_param_get_int((*(*factory).msg).param, 'u' as i32, 0i32);
                if force_plaintext == 0i32 {
                    e2ee_guaranteed = dc_param_get_int((*(*factory).msg).param, 'c' as i32, 0i32)
                }
            }
            if (*chat).gossiped_timestamp == 0
                || ((*chat).gossiped_timestamp + (2 * 24 * 60 * 60)) < time(0 as *mut time_t)
            {
                do_gossip = 1i32
            }
            /* build header etc. */
            let command: libc::c_int = dc_param_get_int((*msg).param, 'S' as i32, 0i32);
            if (*chat).type_0 == 120i32 || (*chat).type_0 == 130i32 {
                mailimf_fields_add(
                    imf_fields,
                    mailimf_field_new_custom(
                        strdup(b"Chat-Group-ID\x00" as *const u8 as *const libc::c_char),
                        dc_strdup((*chat).grpid),
                    ),
                );
                mailimf_fields_add(
                    imf_fields,
                    mailimf_field_new_custom(
                        strdup(b"Chat-Group-Name\x00" as *const u8 as *const libc::c_char),
                        dc_encode_header_words((*chat).name),
                    ),
                );
                if command == 5i32 {
                    let email_to_remove: *mut libc::c_char =
                        dc_param_get((*msg).param, 'E' as i32, 0 as *const libc::c_char);
                    if !email_to_remove.is_null() {
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
                } else if command == 4i32 {
                    do_gossip = 1i32;
                    let email_to_add: *mut libc::c_char =
                        dc_param_get((*msg).param, 'E' as i32, 0 as *const libc::c_char);
                    if !email_to_add.is_null() {
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
                        grpimage = dc_param_get((*chat).param, 'i' as i32, 0 as *const libc::c_char)
                    }
                    if 0 != dc_param_get_int((*msg).param, 'F' as i32, 0i32) & 0x1i32 {
                        dc_log_info(
                            (*msg).context,
                            0i32,
                            b"sending secure-join message \'%s\' >>>>>>>>>>>>>>>>>>>>>>>>>\x00"
                                as *const u8 as *const libc::c_char,
                            b"vg-member-added\x00" as *const u8 as *const libc::c_char,
                        );
                        mailimf_fields_add(
                            imf_fields,
                            mailimf_field_new_custom(
                                strdup(b"Secure-Join\x00" as *const u8 as *const libc::c_char),
                                strdup(b"vg-member-added\x00" as *const u8 as *const libc::c_char),
                            ),
                        );
                    }
                } else if command == 2i32 {
                    mailimf_fields_add(
                        imf_fields,
                        mailimf_field_new_custom(
                            strdup(
                                b"Chat-Group-Name-Changed\x00" as *const u8 as *const libc::c_char,
                            ),
                            dc_param_get(
                                (*msg).param,
                                'E' as i32,
                                b"\x00" as *const u8 as *const libc::c_char,
                            ),
                        ),
                    );
                } else if command == 3i32 {
                    grpimage = dc_param_get((*msg).param, 'E' as i32, 0 as *const libc::c_char);
                    if grpimage.is_null() {
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
            if command == 8i32 {
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
            if command == 6i32 {
                mailimf_fields_add(
                    imf_fields,
                    mailimf_field_new_custom(
                        strdup(b"Autocrypt-Setup-Message\x00" as *const u8 as *const libc::c_char),
                        strdup(b"v1\x00" as *const u8 as *const libc::c_char),
                    ),
                );
                placeholdertext = dc_stock_str((*factory).context, 43i32)
            }
            if command == 7i32 {
                let step: *mut libc::c_char =
                    dc_param_get((*msg).param, 'E' as i32, 0 as *const libc::c_char);
                if !step.is_null() {
                    dc_log_info(
                        (*msg).context,
                        0i32,
                        b"sending secure-join message \'%s\' >>>>>>>>>>>>>>>>>>>>>>>>>\x00"
                            as *const u8 as *const libc::c_char,
                        step,
                    );
                    mailimf_fields_add(
                        imf_fields,
                        mailimf_field_new_custom(
                            strdup(b"Secure-Join\x00" as *const u8 as *const libc::c_char),
                            step,
                        ),
                    );
                    let param2: *mut libc::c_char =
                        dc_param_get((*msg).param, 'F' as i32, 0 as *const libc::c_char);
                    if !param2.is_null() {
                        mailimf_fields_add(
                            imf_fields,
                            mailimf_field_new_custom(
                                if strcmp(
                                    step,
                                    b"vg-request-with-auth\x00" as *const u8 as *const libc::c_char,
                                ) == 0i32
                                    || strcmp(
                                        step,
                                        b"vc-request-with-auth\x00" as *const u8
                                            as *const libc::c_char,
                                    ) == 0i32
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
                    let fingerprint: *mut libc::c_char =
                        dc_param_get((*msg).param, 'G' as i32, 0 as *const libc::c_char);
                    if !fingerprint.is_null() {
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
                    let grpid: *mut libc::c_char =
                        dc_param_get((*msg).param, 'H' as i32, 0 as *const libc::c_char);
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
            if !grpimage.is_null() {
                let mut meta: *mut dc_msg_t = dc_msg_new_untyped((*factory).context);
                (*meta).type_0 = 20i32;
                dc_param_set((*meta).param, 'f' as i32, grpimage);
                let mut filename_as_sent: *mut libc::c_char = 0 as *mut libc::c_char;
                meta_part = build_body_file(
                    meta,
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
                dc_msg_unref(meta);
            }
            if (*msg).type_0 == 41i32 || (*msg).type_0 == 40i32 || (*msg).type_0 == 50i32 {
                if (*msg).type_0 == 41i32 {
                    mailimf_fields_add(
                        imf_fields,
                        mailimf_field_new_custom(
                            strdup(b"Chat-Voice-Message\x00" as *const u8 as *const libc::c_char),
                            strdup(b"1\x00" as *const u8 as *const libc::c_char),
                        ),
                    );
                }
                let duration_ms: libc::c_int = dc_param_get_int((*msg).param, 'd' as i32, 0i32);
                if duration_ms > 0i32 {
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
            afwd_email = dc_param_exists((*msg).param, 'a' as i32);
            let mut fwdhint: *mut libc::c_char = 0 as *mut libc::c_char;
            if 0 != afwd_email {
                fwdhint = dc_strdup(
                    b"---------- Forwarded message ----------\r\nFrom: Delta Chat\r\n\r\n\x00"
                        as *const u8 as *const libc::c_char,
                )
            }
            let mut final_text: *const libc::c_char = 0 as *const libc::c_char;
            if !placeholdertext.is_null() {
                final_text = placeholdertext
            } else if !(*msg).text.is_null() && 0 != *(*msg).text.offset(0isize) as libc::c_int {
                final_text = (*msg).text
            }
            let footer: *mut libc::c_char = (*factory).selfstatus;
            message_text = dc_mprintf(
                b"%s%s%s%s%s\x00" as *const u8 as *const libc::c_char,
                if !fwdhint.is_null() {
                    fwdhint
                } else {
                    b"\x00" as *const u8 as *const libc::c_char
                },
                if !final_text.is_null() {
                    final_text
                } else {
                    b"\x00" as *const u8 as *const libc::c_char
                },
                if !final_text.is_null()
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
            if (*msg).type_0 == 20i32
                || (*msg).type_0 == 21i32
                || (*msg).type_0 == 40i32
                || (*msg).type_0 == 41i32
                || (*msg).type_0 == 50i32
                || (*msg).type_0 == 60i32
            {
                if 0 == is_file_size_okay(msg) {
                    let error: *mut libc::c_char = dc_mprintf(
                        b"Message exceeds the recommended %i MB.\x00" as *const u8
                            as *const libc::c_char,
                        24i32 * 1024i32 * 1024i32 / 4i32 * 3i32 / 1000i32 / 1000i32,
                    );
                    set_error(factory, error);
                    free(error as *mut libc::c_void);
                    current_block = 11328123142868406523;
                } else {
                    let file_part: *mut mailmime =
                        build_body_file(msg, 0 as *const libc::c_char, 0 as *mut *mut libc::c_char);
                    if !file_part.is_null() {
                        mailmime_smart_add_part(message, file_part);
                        parts += 1
                    }
                    current_block = 13000670339742628194;
                }
            } else {
                current_block = 13000670339742628194;
            }
            match current_block {
                11328123142868406523 => {}
                _ => {
                    if parts == 0i32 {
                        set_error(
                            factory,
                            b"Empty message.\x00" as *const u8 as *const libc::c_char,
                        );
                        current_block = 11328123142868406523;
                    } else {
                        if !meta_part.is_null() {
                            mailmime_smart_add_part(message, meta_part);
                        }
                        if 0 != dc_param_exists((*msg).param, DC_PARAM_SET_LATITUDE as libc::c_int)
                        {
                            let latitude = dc_param_get_float(
                                (*msg).param,
                                DC_PARAM_SET_LATITUDE as libc::c_int,
                                0.0,
                            );
                            let longitude = dc_param_get_float(
                                (*msg).param,
                                DC_PARAM_SET_LONGITUDE as libc::c_int,
                                0.0,
                            );
                            let kml_file =
                                dc_get_message_kml((*msg).timestamp_sort, latitude, longitude);
                            if !kml_file.is_null() {
                                let content_type = mailmime_content_new_with_str(
                                    b"application/vnd.google-earth.kml+xml\x00" as *const u8
                                        as *const libc::c_char,
                                );
                                let mime_fields = mailmime_fields_new_filename(
                                    MAILMIME_DISPOSITION_TYPE_ATTACHMENT as libc::c_int,
                                    dc_strdup(
                                        b"message.kml\x00" as *const u8 as *const libc::c_char,
                                    ),
                                    MAILMIME_MECHANISM_8BIT as libc::c_int,
                                );
                                let kml_mime_part = mailmime_new_empty(content_type, mime_fields);
                                mailmime_set_body_text(kml_mime_part, kml_file, strlen(kml_file));

                                mailmime_smart_add_part(message, kml_mime_part);
                            }
                        }

                        if 0 != dc_is_sending_locations_to_chat((*msg).context, (*msg).chat_id) {
                            let mut last_added_location_id: uint32_t = 0i32 as uint32_t;
                            let kml_file: *mut libc::c_char = dc_get_location_kml(
                                (*msg).context,
                                (*msg).chat_id,
                                &mut last_added_location_id,
                            );
                            if !kml_file.is_null() {
                                let content_type: *mut mailmime_content =
                                    mailmime_content_new_with_str(
                                        b"application/vnd.google-earth.kml+xml\x00" as *const u8
                                            as *const libc::c_char,
                                    );
                                let mime_fields: *mut mailmime_fields =
                                    mailmime_fields_new_filename(
                                        MAILMIME_DISPOSITION_TYPE_ATTACHMENT as libc::c_int,
                                        dc_strdup(
                                            b"location.kml\x00" as *const u8 as *const libc::c_char,
                                        ),
                                        MAILMIME_MECHANISM_8BIT as libc::c_int,
                                    );
                                let kml_mime_part: *mut mailmime =
                                    mailmime_new_empty(content_type, mime_fields);
                                mailmime_set_body_text(kml_mime_part, kml_file, strlen(kml_file));
                                mailmime_smart_add_part(message, kml_mime_part);
                                if 0 == dc_param_exists(
                                    (*msg).param,
                                    DC_PARAM_SET_LATITUDE as libc::c_int,
                                ) {
                                    // otherwise, the independent location is already filed
                                    (*factory).out_last_added_location_id = last_added_location_id;
                                }
                            }
                        }
                        current_block = 9952640327414195044;
                    }
                }
            }
        } else if (*factory).loaded as libc::c_uint
            == DC_MF_MDN_LOADED as libc::c_int as libc::c_uint
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
            let p1: *mut libc::c_char;
            let p2: *mut libc::c_char;
            if 0 != dc_param_get_int((*(*factory).msg).param, 'c' as i32, 0i32) {
                p1 = dc_stock_str((*factory).context, 24i32)
            } else {
                p1 = dc_msg_get_summarytext((*factory).msg, 32i32)
            }
            p2 = dc_stock_str_repl_string((*factory).context, 32i32, p1);
            message_text = dc_mprintf(b"%s\r\n\x00" as *const u8 as *const libc::c_char, p2);
            free(p2 as *mut libc::c_void);
            free(p1 as *mut libc::c_void);
            let human_mime_part: *mut mailmime = build_body_text(message_text);
            mailmime_add_part(multipart, human_mime_part);
            message_text2 =
                dc_mprintf(b"Reporting-UA: Delta Chat %s\r\nOriginal-Recipient: rfc822;%s\r\nFinal-Recipient: rfc822;%s\r\nOriginal-Message-ID: <%s>\r\nDisposition: manual-action/MDN-sent-automatically; displayed\r\n\x00"
                               as *const u8 as *const libc::c_char,
                           VERSION as *const u8 as *const libc::c_char,
                           (*factory).from_addr, (*factory).from_addr,
                           (*(*factory).msg).rfc724_mid);
            let content_type_0: *mut mailmime_content = mailmime_content_new_with_str(
                b"message/disposition-notification\x00" as *const u8 as *const libc::c_char,
            );
            let mime_fields_0: *mut mailmime_fields =
                mailmime_fields_new_encoding(MAILMIME_MECHANISM_8BIT as libc::c_int);
            let mach_mime_part: *mut mailmime = mailmime_new_empty(content_type_0, mime_fields_0);
            mailmime_set_body_text(mach_mime_part, message_text2, strlen(message_text2));
            mailmime_add_part(multipart, mach_mime_part);
            force_plaintext = 2i32;
            current_block = 9952640327414195044;
        } else {
            set_error(
                factory,
                b"No message loaded.\x00" as *const u8 as *const libc::c_char,
            );
            current_block = 11328123142868406523;
        }
        match current_block {
            11328123142868406523 => {}
            _ => {
                if (*factory).loaded as libc::c_uint
                    == DC_MF_MDN_LOADED as libc::c_int as libc::c_uint
                {
                    let e: *mut libc::c_char = dc_stock_str((*factory).context, 31i32);
                    subject_str =
                        dc_mprintf(b"Chat: %s\x00" as *const u8 as *const libc::c_char, e);
                    free(e as *mut libc::c_void);
                } else {
                    subject_str = get_subject((*factory).chat, (*factory).msg, afwd_email)
                }
                subject = mailimf_subject_new(dc_encode_header_words(subject_str));
                mailimf_fields_add(
                    imf_fields,
                    mailimf_field_new(
                        MAILIMF_FIELD_SUBJECT as libc::c_int,
                        0 as *mut mailimf_return,
                        0 as *mut mailimf_orig_date,
                        0 as *mut mailimf_from,
                        0 as *mut mailimf_sender,
                        0 as *mut mailimf_to,
                        0 as *mut mailimf_cc,
                        0 as *mut mailimf_bcc,
                        0 as *mut mailimf_message_id,
                        0 as *mut mailimf_orig_date,
                        0 as *mut mailimf_from,
                        0 as *mut mailimf_sender,
                        0 as *mut mailimf_reply_to,
                        0 as *mut mailimf_to,
                        0 as *mut mailimf_cc,
                        0 as *mut mailimf_bcc,
                        0 as *mut mailimf_message_id,
                        0 as *mut mailimf_in_reply_to,
                        0 as *mut mailimf_references,
                        subject,
                        0 as *mut mailimf_comments,
                        0 as *mut mailimf_keywords,
                        0 as *mut mailimf_optional_field,
                    ),
                );
                if force_plaintext != 2i32 {
                    dc_e2ee_encrypt(
                        (*factory).context,
                        (*factory).recipients_addr,
                        force_plaintext,
                        e2ee_guaranteed,
                        min_verified,
                        do_gossip,
                        message,
                        &mut e2ee_helper,
                    );
                }
                if 0 != e2ee_helper.encryption_successfull {
                    (*factory).out_encrypted = 1i32;
                    if 0 != do_gossip {
                        (*factory).out_gossiped = 1i32
                    }
                }
                (*factory).out = mmap_string_new(b"\x00" as *const u8 as *const libc::c_char);
                mailmime_write_mem((*factory).out, &mut col, message);
                success = 1i32
            }
        }
    }
    if !message.is_null() {
        mailmime_free(message);
    }
    dc_e2ee_thanks(&mut e2ee_helper);
    free(message_text as *mut libc::c_void);
    free(message_text2 as *mut libc::c_void);
    free(subject_str as *mut libc::c_void);
    free(grpimage as *mut libc::c_void);

    success
}

unsafe fn get_subject(
    chat: *const dc_chat_t,
    msg: *const dc_msg_t,
    afwd_email: libc::c_int,
) -> *mut libc::c_char {
    let context = (*chat).context;
    let ret: *mut libc::c_char;
    let raw_subject: *mut libc::c_char =
        dc_msg_get_summarytext_by_raw((*msg).type_0, (*msg).text, (*msg).param, 32i32, context);
    let fwd: *const libc::c_char = if 0 != afwd_email {
        b"Fwd: \x00" as *const u8 as *const libc::c_char
    } else {
        b"\x00" as *const u8 as *const libc::c_char
    };
    if dc_param_get_int((*msg).param, 'S' as i32, 0i32) == 6i32 {
        ret = dc_stock_str(context, 42i32)
    } else if (*chat).type_0 == 120i32 || (*chat).type_0 == 130i32 {
        ret = dc_mprintf(
            b"Chat: %s: %s%s\x00" as *const u8 as *const libc::c_char,
            (*chat).name,
            fwd,
            raw_subject,
        )
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

unsafe fn set_error(mut factory: *mut dc_mimefactory_t, text: *const libc::c_char) {
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

unsafe fn build_body_file(
    msg: *const dc_msg_t,
    mut base_name: *const libc::c_char,
    ret_file_name_as_sent: *mut *mut libc::c_char,
) -> *mut mailmime {
    let needs_ext: bool;
    let mime_fields: *mut mailmime_fields;
    let mut mime_sub: *mut mailmime = 0 as *mut mailmime;
    let content: *mut mailmime_content;
    let pathNfilename: *mut libc::c_char =
        dc_param_get((*msg).param, 'f' as i32, 0 as *const libc::c_char);
    let mut mimetype: *mut libc::c_char =
        dc_param_get((*msg).param, 'm' as i32, 0 as *const libc::c_char);
    let suffix: *mut libc::c_char = dc_get_filesuffix_lc(pathNfilename);
    let mut filename_to_send: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut filename_encoded: *mut libc::c_char = 0 as *mut libc::c_char;
    if !pathNfilename.is_null() {
        if (*msg).type_0 == 41i32 {
            let mut wanted_struct: tm = tm {
                tm_sec: 0,
                tm_min: 0,
                tm_hour: 0,
                tm_mday: 0,
                tm_mon: 0,
                tm_year: 0,
                tm_wday: 0,
                tm_yday: 0,
                tm_isdst: 0,
                tm_gmtoff: 0,
                tm_zone: 0 as *mut libc::c_char,
            };
            memcpy(
                &mut wanted_struct as *mut tm as *mut libc::c_void,
                localtime(&(*msg).timestamp_sort) as *const libc::c_void,
                ::std::mem::size_of::<tm>(),
            );
            filename_to_send = dc_mprintf(
                b"voice-message_%04i-%02i-%02i_%02i-%02i-%02i.%s\x00" as *const u8
                    as *const libc::c_char,
                wanted_struct.tm_year as libc::c_int + 1900i32,
                wanted_struct.tm_mon as libc::c_int + 1i32,
                wanted_struct.tm_mday as libc::c_int,
                wanted_struct.tm_hour as libc::c_int,
                wanted_struct.tm_min as libc::c_int,
                wanted_struct.tm_sec as libc::c_int,
                if !suffix.is_null() {
                    suffix
                } else {
                    b"dat\x00" as *const u8 as *const libc::c_char
                },
            )
        } else if (*msg).type_0 == 40i32 {
            filename_to_send = dc_get_filename(pathNfilename)
        } else if (*msg).type_0 == 20i32 || (*msg).type_0 == 21i32 {
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
        } else if (*msg).type_0 == 50i32 {
            filename_to_send = dc_mprintf(
                b"video.%s\x00" as *const u8 as *const libc::c_char,
                if !suffix.is_null() {
                    suffix
                } else {
                    b"dat\x00" as *const u8 as *const libc::c_char
                },
            )
        } else {
            filename_to_send = dc_get_filename(pathNfilename)
        }
        if mimetype.is_null() {
            if suffix.is_null() {
                mimetype =
                    dc_strdup(b"application/octet-stream\x00" as *const u8 as *const libc::c_char)
            } else if strcmp(suffix, b"png\x00" as *const u8 as *const libc::c_char) == 0i32 {
                mimetype = dc_strdup(b"image/png\x00" as *const u8 as *const libc::c_char)
            } else if strcmp(suffix, b"jpg\x00" as *const u8 as *const libc::c_char) == 0i32
                || strcmp(suffix, b"jpeg\x00" as *const u8 as *const libc::c_char) == 0i32
                || strcmp(suffix, b"jpe\x00" as *const u8 as *const libc::c_char) == 0i32
            {
                mimetype = dc_strdup(b"image/jpeg\x00" as *const u8 as *const libc::c_char)
            } else if strcmp(suffix, b"gif\x00" as *const u8 as *const libc::c_char) == 0i32 {
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
            needs_ext = dc_needs_ext_header(filename_to_send);
            mime_fields = mailmime_fields_new_filename(
                MAILMIME_DISPOSITION_TYPE_ATTACHMENT as libc::c_int,
                if needs_ext {
                    0 as *mut libc::c_char
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
                        0 as *mut libc::c_void
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
                                    0 as *mut libc::c_char,
                                    0 as *mut libc::c_char,
                                    0 as *mut libc::c_char,
                                    0 as *mut libc::c_char,
                                    0i32 as size_t,
                                    mailmime_parameter_new(
                                        strdup(
                                            b"filename*\x00" as *const u8 as *const libc::c_char,
                                        ),
                                        dc_encode_ext_header(filename_to_send),
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
                            0 as *mut clistcell
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
            mailmime_set_body_file(mime_sub, dc_get_abs_path((*msg).context, pathNfilename));
            if !ret_file_name_as_sent.is_null() {
                *ret_file_name_as_sent = dc_strdup(filename_to_send)
            }
        }
    }
    free(pathNfilename as *mut libc::c_void);
    free(mimetype as *mut libc::c_void);
    free(filename_to_send as *mut libc::c_void);
    free(filename_encoded as *mut libc::c_void);
    free(suffix as *mut libc::c_void);

    mime_sub
}

/*******************************************************************************
 * Render
 ******************************************************************************/

unsafe fn is_file_size_okay(msg: *const dc_msg_t) -> libc::c_int {
    let mut file_size_okay: libc::c_int = 1i32;
    let pathNfilename: *mut libc::c_char =
        dc_param_get((*msg).param, 'f' as i32, 0 as *const libc::c_char);
    let bytes: uint64_t = dc_get_filebytes((*msg).context, pathNfilename);
    if bytes > (49i32 * 1024i32 * 1024i32 / 4i32 * 3i32) as libc::c_ulonglong {
        file_size_okay = 0i32
    }
    free(pathNfilename as *mut libc::c_void);

    file_size_okay
}
