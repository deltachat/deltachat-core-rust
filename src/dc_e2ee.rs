use std::ffi::{CStr, CString};
use std::str::FromStr;

use mmime::clist::*;
use mmime::mailimf::*;
use mmime::mailimf_types::*;
use mmime::mailimf_types_helper::*;
use mmime::mailmime::*;
use mmime::mailmime_content::*;
use mmime::mailmime_types::*;
use mmime::mailmime_types_helper::*;
use mmime::mailmime_write_mem::*;
use mmime::mailprivacy_prepare_mime;
use mmime::mmapstring::*;
use mmime::{mailmime_substitute, MAILIMF_NO_ERROR, MAIL_NO_ERROR};

use crate::dc_aheader::*;
use crate::dc_apeerstate::*;
use crate::dc_context::dc_context_t;
use crate::dc_hash::*;
use crate::dc_key::*;
use crate::dc_keyring::*;
use crate::dc_log::*;
use crate::dc_mimeparser::*;
use crate::dc_pgp::*;
use crate::dc_securejoin::*;
use crate::dc_sqlite3::*;
use crate::dc_tools::*;
use crate::types::*;
use crate::x::*;

// backups
// attachments of 25 mb brutto should work on the majority of providers
// (brutto examples: web.de=50, 1&1=40, t-online.de=32, gmail=25, posteo=50, yahoo=25, all-inkl=100).
// as an upper limit, we double the size; the core won't send messages larger than this
// to get the netto sizes, we substract 1 mb header-overhead and the base64-overhead.
// some defaults
#[derive(Copy, Clone)]
#[repr(C)]
pub struct dc_e2ee_helper_t {
    pub encryption_successfull: libc::c_int,
    pub cdata_to_free: *mut libc::c_void,
    pub encrypted: libc::c_int,
    pub signatures: *mut dc_hash_t,
    pub gossipped_addr: *mut dc_hash_t,
}

pub unsafe fn dc_e2ee_encrypt(
    context: &dc_context_t,
    recipients_addr: *const clist,
    force_unencrypted: libc::c_int,
    e2ee_guaranteed: libc::c_int,
    min_verified: libc::c_int,
    do_gossip: libc::c_int,
    mut in_out_message: *mut mailmime,
    mut helper: *mut dc_e2ee_helper_t,
) {
    let mut current_block: u64 = 0;
    let mut col: libc::c_int = 0i32;
    let mut do_encrypt: libc::c_int = 0i32;
    /*just a pointer into mailmime structure, must not be freed*/
    let imffields_unprotected: *mut mailimf_fields;
    let mut keyring = Keyring::default();
    let plain: *mut MMAPString = mmap_string_new(b"\x00" as *const u8 as *const libc::c_char);
    let mut peerstates = Vec::new();
    if !helper.is_null() {
        memset(
            helper as *mut libc::c_void,
            0,
            ::std::mem::size_of::<dc_e2ee_helper_t>(),
        );
    }

    if !(recipients_addr.is_null()
        || in_out_message.is_null()
        || !(*in_out_message).mm_parent.is_null()
        || plain.is_null()
        || helper.is_null())
    {
        /* libEtPan's pgp_encrypt_mime() takes the parent as the new root. We just expect the root as being given to this function. */
        let prefer_encrypt = if 0
            != dc_sqlite3_get_config_int(
                context,
                &context.sql.clone().read().unwrap(),
                b"e2ee_enabled\x00" as *const u8 as *const libc::c_char,
                1,
            ) {
            EncryptPreference::Mutual
        } else {
            EncryptPreference::NoPreference
        };

        let addr = dc_sqlite3_get_config(
            context,
            &context.sql.clone().read().unwrap(),
            b"configured_addr\x00" as *const u8 as *const libc::c_char,
            0 as *const libc::c_char,
        );

        if !addr.is_null() {
            if let Some(public_key) =
                load_or_generate_self_public_key(context, addr, in_out_message)
            {
                /*only for random-seed*/
                if prefer_encrypt == EncryptPreference::Mutual || 0 != e2ee_guaranteed {
                    do_encrypt = 1i32;
                    let mut iter1: *mut clistiter;
                    iter1 = (*recipients_addr).first;
                    while !iter1.is_null() {
                        let recipient_addr: *const libc::c_char = (if !iter1.is_null() {
                            (*iter1).data
                        } else {
                            0 as *mut libc::c_void
                        })
                            as *const libc::c_char;
                        let mut peerstate = dc_apeerstate_new(context);
                        if !(strcasecmp(recipient_addr, addr) == 0i32) {
                            if 0 != dc_apeerstate_load_by_addr(
                                &mut peerstate,
                                &context.sql.clone().read().unwrap(),
                                recipient_addr,
                            ) && (peerstate.prefer_encrypt == 1i32 || 0 != e2ee_guaranteed)
                            {
                                if let Some(key_to_use) =
                                    dc_apeerstate_peek_key(&peerstate, min_verified)
                                {
                                    // TODO: avoid clone
                                    keyring.add(key_to_use.clone());
                                    peerstates.push(peerstate);
                                }
                            } else {
                                dc_apeerstate_unref(&mut peerstate);
                                do_encrypt = 0i32;
                                /* if we cannot encrypt to a single recipient, we cannot encrypt the message at all */
                                break;
                            }
                        }
                        iter1 = if !iter1.is_null() {
                            (*iter1).next
                        } else {
                            0 as *mut clistcell
                        }
                    }
                }
                let sign_key = if 0 != do_encrypt {
                    // TODO: avoid clone
                    keyring.add(public_key.clone());
                    let key =
                        Key::from_self_private(context, addr, &context.sql.clone().read().unwrap());

                    if key.is_none() {
                        do_encrypt = 0i32;
                    }
                    key
                } else {
                    None
                };
                if 0 != force_unencrypted {
                    do_encrypt = 0i32
                }
                imffields_unprotected = mailmime_find_mailimf_fields(in_out_message);
                if !imffields_unprotected.is_null() {
                    /* encrypt message, if possible */
                    if 0 != do_encrypt {
                        mailprivacy_prepare_mime(in_out_message);
                        let mut part_to_encrypt: *mut mailmime =
                            (*in_out_message).mm_data.mm_message.mm_msg_mime;
                        (*part_to_encrypt).mm_parent = 0 as *mut mailmime;
                        let imffields_encrypted: *mut mailimf_fields = mailimf_fields_new_empty();
                        /* mailmime_new_message_data() calls mailmime_fields_new_with_version() which would add the unwanted MIME-Version:-header */
                        let message_to_encrypt: *mut mailmime = mailmime_new(
                            MAILMIME_MESSAGE as libc::c_int,
                            0 as *const libc::c_char,
                            0i32 as size_t,
                            mailmime_fields_new_empty(),
                            mailmime_get_content_message(),
                            0 as *mut mailmime_data,
                            0 as *mut mailmime_data,
                            0 as *mut mailmime_data,
                            0 as *mut clist,
                            imffields_encrypted,
                            part_to_encrypt,
                        );
                        if 0 != do_gossip {
                            let iCnt: libc::c_int = peerstates.len() as libc::c_int;
                            if iCnt > 1i32 {
                                let mut i: libc::c_int = 0i32;
                                while i < iCnt {
                                    let p: *mut libc::c_char = dc_apeerstate_render_gossip_header(
                                        &peerstates[i as usize],
                                        min_verified,
                                    );
                                    if !p.is_null() {
                                        mailimf_fields_add(
                                            imffields_encrypted,
                                            mailimf_field_new_custom(
                                                strdup(
                                                    b"Autocrypt-Gossip\x00" as *const u8
                                                        as *const libc::c_char,
                                                ),
                                                p,
                                            ),
                                        );
                                    }
                                    i += 1
                                }
                            }
                        }
                        /* memoryhole headers */
                        let mut cur: *mut clistiter = (*(*imffields_unprotected).fld_list).first;
                        while !cur.is_null() {
                            let mut move_to_encrypted: libc::c_int = 0i32;
                            let field: *mut mailimf_field = (if !cur.is_null() {
                                (*cur).data
                            } else {
                                0 as *mut libc::c_void
                            })
                                as *mut mailimf_field;
                            if !field.is_null() {
                                if (*field).fld_type == MAILIMF_FIELD_SUBJECT as libc::c_int {
                                    move_to_encrypted = 1i32
                                } else if (*field).fld_type
                                    == MAILIMF_FIELD_OPTIONAL_FIELD as libc::c_int
                                {
                                    let opt_field: *mut mailimf_optional_field =
                                        (*field).fld_data.fld_optional_field;
                                    if !opt_field.is_null() && !(*opt_field).fld_name.is_null() {
                                        if strncmp(
                                            (*opt_field).fld_name,
                                            b"Secure-Join\x00" as *const u8 as *const libc::c_char,
                                            11,
                                        ) == 0
                                            || strncmp(
                                                (*opt_field).fld_name,
                                                b"Chat-\x00" as *const u8 as *const libc::c_char,
                                                5,
                                            ) == 0
                                                && strcmp(
                                                    (*opt_field).fld_name,
                                                    b"Chat-Version\x00" as *const u8
                                                        as *const libc::c_char,
                                                ) != 0
                                        {
                                            move_to_encrypted = 1
                                        }
                                    }
                                }
                            }
                            if 0 != move_to_encrypted {
                                mailimf_fields_add(imffields_encrypted, field);
                                cur = clist_delete((*imffields_unprotected).fld_list, cur)
                            } else {
                                cur = if !cur.is_null() {
                                    (*cur).next
                                } else {
                                    0 as *mut clistcell
                                }
                            }
                        }
                        let subject: *mut mailimf_subject = mailimf_subject_new(dc_strdup(
                            b"...\x00" as *const u8 as *const libc::c_char,
                        ));
                        mailimf_fields_add(
                            imffields_unprotected,
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
                        clist_insert_after(
                            (*(*part_to_encrypt).mm_content_type).ct_parameters,
                            (*(*(*part_to_encrypt).mm_content_type).ct_parameters).last,
                            mailmime_param_new_with_data(
                                b"protected-headers\x00" as *const u8 as *const libc::c_char
                                    as *mut libc::c_char,
                                b"v1\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
                            ) as *mut libc::c_void,
                        );
                        mailmime_write_mem(plain, &mut col, message_to_encrypt);
                        if (*plain).str_0.is_null() || (*plain).len <= 0 {
                            current_block = 14181132614457621749;
                        } else {
                            if let Some(ctext_v) = dc_pgp_pk_encrypt(
                                (*plain).str_0 as *const libc::c_void,
                                (*plain).len,
                                &keyring,
                                sign_key.as_ref(),
                            ) {
                                let ctext_bytes = ctext_v.len();
                                let ctext_c = CString::new(ctext_v).unwrap();
                                let ctext = strdup(ctext_c.as_ptr());

                                (*helper).cdata_to_free = ctext as *mut libc::c_void;

                                /* create MIME-structure that will contain the encrypted text */
                                let mut encrypted_part: *mut mailmime = new_data_part(
                                    0 as *mut libc::c_void,
                                    0i32 as size_t,
                                    b"multipart/encrypted\x00" as *const u8 as *const libc::c_char
                                        as *mut libc::c_char,
                                    -1i32,
                                );
                                let content: *mut mailmime_content =
                                    (*encrypted_part).mm_content_type;
                                clist_insert_after(
                                    (*content).ct_parameters,
                                    (*(*content).ct_parameters).last,
                                    mailmime_param_new_with_data(
                                        b"protocol\x00" as *const u8 as *const libc::c_char
                                            as *mut libc::c_char,
                                        b"application/pgp-encrypted\x00" as *const u8
                                            as *const libc::c_char
                                            as *mut libc::c_char,
                                    ) as *mut libc::c_void,
                                );
                                static mut version_content: [libc::c_char; 13] =
                                    [86, 101, 114, 115, 105, 111, 110, 58, 32, 49, 13, 10, 0];
                                let version_mime: *mut mailmime = new_data_part(
                                    version_content.as_mut_ptr() as *mut libc::c_void,
                                    strlen(version_content.as_mut_ptr()),
                                    b"application/pgp-encrypted\x00" as *const u8
                                        as *const libc::c_char
                                        as *mut libc::c_char,
                                    MAILMIME_MECHANISM_7BIT as libc::c_int,
                                );
                                mailmime_smart_add_part(encrypted_part, version_mime);
                                let ctext_part: *mut mailmime = new_data_part(
                                    ctext as *mut libc::c_void,
                                    ctext_bytes,
                                    b"application/octet-stream\x00" as *const u8
                                        as *const libc::c_char
                                        as *mut libc::c_char,
                                    MAILMIME_MECHANISM_7BIT as libc::c_int,
                                );
                                mailmime_smart_add_part(encrypted_part, ctext_part);
                                (*in_out_message).mm_data.mm_message.mm_msg_mime = encrypted_part;
                                (*encrypted_part).mm_parent = in_out_message;
                                mailmime_free(message_to_encrypt);
                                (*helper).encryption_successfull = 1i32;
                                current_block = 13824533195664196414;
                            }
                        }
                    } else {
                        current_block = 13824533195664196414;
                    }
                    match current_block {
                        14181132614457621749 => {}
                        _ => {
                            let addr = CStr::from_ptr(addr).to_str().unwrap();
                            let aheader = Aheader::new(addr.into(), public_key, prefer_encrypt);
                            let rendered = CString::new(aheader.to_string()).unwrap();

                            mailimf_fields_add(
                                imffields_unprotected,
                                mailimf_field_new_custom(
                                    strdup(b"Autocrypt\x00" as *const u8 as *const libc::c_char),
                                    strdup(rendered.as_ptr()),
                                ),
                            );
                        }
                    }
                }
            }
        }
    }

    if !plain.is_null() {
        mmap_string_free(plain);
    }

    for peerstate in peerstates.iter_mut() {
        dc_apeerstate_unref(peerstate);
    }
}

/*******************************************************************************
 * Tools
 ******************************************************************************/
unsafe fn new_data_part(
    data: *mut libc::c_void,
    data_bytes: size_t,
    default_content_type: *mut libc::c_char,
    default_encoding: libc::c_int,
) -> *mut mailmime {
    let mut current_block: u64;
    //char basename_buf[PATH_MAX];
    let mut encoding: *mut mailmime_mechanism;
    let content: *mut mailmime_content;
    let mime: *mut mailmime;
    //int r;
    //char * dup_filename;
    let mime_fields: *mut mailmime_fields;
    let encoding_type: libc::c_int;
    let content_type_str: *mut libc::c_char;
    let mut do_encoding: libc::c_int;
    encoding = 0 as *mut mailmime_mechanism;
    if default_content_type.is_null() {
        content_type_str =
            b"application/octet-stream\x00" as *const u8 as *const libc::c_char as *mut libc::c_char
    } else {
        content_type_str = default_content_type
    }
    content = mailmime_content_new_with_str(content_type_str);
    if content.is_null() {
        current_block = 16266721588079097885;
    } else {
        do_encoding = 1i32;
        if (*(*content).ct_type).tp_type == MAILMIME_TYPE_COMPOSITE_TYPE as libc::c_int {
            let composite: *mut mailmime_composite_type;
            composite = (*(*content).ct_type).tp_data.tp_composite_type;
            match (*composite).ct_type {
                1 => {
                    if strcasecmp(
                        (*content).ct_subtype,
                        b"rfc822\x00" as *const u8 as *const libc::c_char,
                    ) == 0i32
                    {
                        do_encoding = 0i32
                    }
                }
                2 => do_encoding = 0i32,
                _ => {}
            }
        }
        if 0 != do_encoding {
            if default_encoding == -1i32 {
                encoding_type = MAILMIME_MECHANISM_BASE64 as libc::c_int
            } else {
                encoding_type = default_encoding
            }
            encoding = mailmime_mechanism_new(encoding_type, 0 as *mut libc::c_char);
            if encoding.is_null() {
                current_block = 16266721588079097885;
            } else {
                current_block = 11057878835866523405;
            }
        } else {
            current_block = 11057878835866523405;
        }
        match current_block {
            16266721588079097885 => {}
            _ => {
                mime_fields = mailmime_fields_new_with_data(
                    encoding,
                    0 as *mut libc::c_char,
                    0 as *mut libc::c_char,
                    0 as *mut mailmime_disposition,
                    0 as *mut mailmime_language,
                );
                if mime_fields.is_null() {
                    current_block = 16266721588079097885;
                } else {
                    mime = mailmime_new_empty(content, mime_fields);
                    if mime.is_null() {
                        mailmime_fields_free(mime_fields);
                        mailmime_content_free(content);
                    } else {
                        if !data.is_null()
                            && data_bytes > 0
                            && (*mime).mm_type == MAILMIME_SINGLE as libc::c_int
                        {
                            mailmime_set_body_text(mime, data as *mut libc::c_char, data_bytes);
                        }
                        return mime;
                    }
                    current_block = 13668317689588454213;
                }
            }
        }
    }
    match current_block {
        16266721588079097885 => {
            if !encoding.is_null() {
                mailmime_mechanism_free(encoding);
            }
            if !content.is_null() {
                mailmime_content_free(content);
            }
        }
        _ => {}
    }
    return 0 as *mut mailmime;
}

/*******************************************************************************
 * Generate Keypairs
 ******************************************************************************/
unsafe fn load_or_generate_self_public_key(
    context: &dc_context_t,
    self_addr: *const libc::c_char,
    _random_data_mime: *mut mailmime,
) -> Option<Key> {
    /* avoid double creation (we unlock the database during creation) */
    static mut s_in_key_creation: libc::c_int = 0i32;

    let mut key = Key::from_self_public(context, self_addr, &context.sql.clone().read().unwrap());
    if key.is_some() {
        return key;
    }

    /* create the keypair - this may take a moment, however, as this is in a thread, this is no big deal */
    if 0 != s_in_key_creation {
        return None;
    }
    let key_creation_here = 1;
    s_in_key_creation = 1;

    let start: libc::clock_t = clock();
    dc_log_info(
        context,
        0i32,
        b"Generating keypair with %i bits, e=%i ...\x00" as *const u8 as *const libc::c_char,
        2048i32,
        65537i32,
    );

    if let Some((public_key, private_key)) = dc_pgp_create_keypair(self_addr) {
        if !dc_key_save_self_keypair(
            context,
            &public_key,
            &private_key,
            self_addr,
            1i32,
            &context.sql.clone().read().unwrap(),
        ) {
            /*set default*/
            dc_log_warning(
                context,
                0i32,
                b"Cannot save keypair.\x00" as *const u8 as *const libc::c_char,
            );
        } else {
            dc_log_info(
                context,
                0i32,
                b"Keypair generated in %.3f s.\x00" as *const u8 as *const libc::c_char,
                clock().wrapping_sub(start) as libc::c_double / 1000000i32 as libc::c_double,
            );
        }

        key = Some(public_key);
    } else {
        dc_log_warning(
            context,
            0i32,
            b"Cannot create keypair.\x00" as *const u8 as *const libc::c_char,
        );
    }

    if 0 != key_creation_here {
        s_in_key_creation = 0;
    }

    key
}

/* returns 1 if sth. was decrypted, 0 in other cases */
pub unsafe fn dc_e2ee_decrypt(
    context: &dc_context_t,
    in_out_message: *mut mailmime,
    mut helper: *mut dc_e2ee_helper_t,
) {
    let mut iterations: libc::c_int;
    /* return values: 0=nothing to decrypt/cannot decrypt, 1=sth. decrypted
    (to detect parts that could not be decrypted, simply look for left "multipart/encrypted" MIME types */
    /*just a pointer into mailmime structure, must not be freed*/
    let imffields: *mut mailimf_fields = mailmime_find_mailimf_fields(in_out_message);
    let mut message_time: time_t = 0i32 as time_t;
    let mut peerstate = dc_apeerstate_new(context);
    let mut from: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut self_addr: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut private_keyring = Keyring::default();
    let mut public_keyring_for_validate = Keyring::default();
    let mut gossip_headers: *mut mailimf_fields = 0 as *mut mailimf_fields;
    if !helper.is_null() {
        memset(
            helper as *mut libc::c_void,
            0i32,
            ::std::mem::size_of::<dc_e2ee_helper_t>(),
        );
    }
    if !(in_out_message.is_null() || helper.is_null() || imffields.is_null()) {
        if !imffields.is_null() {
            let mut field: *mut mailimf_field =
                mailimf_find_field(imffields, MAILIMF_FIELD_FROM as libc::c_int);
            if !field.is_null() && !(*field).fld_data.fld_from.is_null() {
                from = mailimf_find_first_addr((*(*field).fld_data.fld_from).frm_mb_list)
            }
            field = mailimf_find_field(imffields, MAILIMF_FIELD_ORIG_DATE as libc::c_int);
            if !field.is_null() && !(*field).fld_data.fld_orig_date.is_null() {
                let orig_date: *mut mailimf_orig_date = (*field).fld_data.fld_orig_date;
                if !orig_date.is_null() {
                    message_time = dc_timestamp_from_date((*orig_date).dt_date_time);
                    if message_time != -1i32 as libc::c_long
                        && message_time > time(0 as *mut time_t)
                    {
                        message_time = time(0 as *mut time_t)
                    }
                }
            }
        }
        let autocryptheader = Aheader::from_imffields(from, imffields);
        if message_time > 0i32 as libc::c_long && !from.is_null() {
            if 0 != dc_apeerstate_load_by_addr(
                &mut peerstate,
                &context.sql.clone().read().unwrap(),
                from,
            ) {
                if let Some(ref header) = autocryptheader {
                    dc_apeerstate_apply_header(&mut peerstate, header, message_time);
                    dc_apeerstate_save_to_db(
                        &mut peerstate,
                        &context.sql.clone().read().unwrap(),
                        0i32,
                    );
                } else if message_time > peerstate.last_seen_autocrypt
                    && 0 == contains_report(in_out_message)
                {
                    dc_apeerstate_degrade_encryption(&mut peerstate, message_time);
                    dc_apeerstate_save_to_db(
                        &peerstate,
                        &context.sql.clone().read().unwrap(),
                        0i32,
                    );
                }
            } else if let Some(ref header) = autocryptheader {
                dc_apeerstate_init_from_header(&mut peerstate, header, message_time);
                dc_apeerstate_save_to_db(&peerstate, &context.sql.clone().read().unwrap(), 1i32);
            }
        }
        /* load private key for decryption */
        self_addr = dc_sqlite3_get_config(
            context,
            &context.sql.clone().read().unwrap(),
            b"configured_addr\x00" as *const u8 as *const libc::c_char,
            0 as *const libc::c_char,
        );
        if !self_addr.is_null() {
            if private_keyring.load_self_private_for_decrypting(
                context,
                self_addr,
                &context.sql.clone().read().unwrap(),
            ) {
                if peerstate.last_seen == 0i32 as libc::c_long {
                    dc_apeerstate_load_by_addr(
                        &mut peerstate,
                        &context.sql.clone().read().unwrap(),
                        from,
                    );
                }
                if 0 != peerstate.degrade_event {
                    dc_handle_degrade_event(context, &peerstate);
                }
                // TODO: avoid clone
                if let Some(ref key) = peerstate.gossip_key {
                    public_keyring_for_validate.add(key.clone());
                }
                if let Some(ref key) = peerstate.public_key {
                    public_keyring_for_validate.add(key.clone());
                }
                (*helper).signatures = malloc(::std::mem::size_of::<dc_hash_t>()) as *mut dc_hash_t;
                dc_hash_init((*helper).signatures, 3i32, 1i32);
                iterations = 0i32;
                while iterations < 10i32 {
                    let mut has_unencrypted_parts: libc::c_int = 0i32;
                    if 0 == decrypt_recursive(
                        context,
                        in_out_message,
                        &private_keyring,
                        &public_keyring_for_validate,
                        (*helper).signatures,
                        &mut gossip_headers,
                        &mut has_unencrypted_parts,
                    ) {
                        break;
                    }
                    if iterations == 0i32 && 0 == has_unencrypted_parts {
                        (*helper).encrypted = 1i32
                    }
                    iterations += 1
                }
                if !gossip_headers.is_null() {
                    (*helper).gossipped_addr =
                        update_gossip_peerstates(context, message_time, imffields, gossip_headers)
                }
            }
        }
    }
    //mailmime_print(in_out_message);
    if !gossip_headers.is_null() {
        mailimf_fields_free(gossip_headers);
    }

    dc_apeerstate_unref(&mut peerstate);
    free(from as *mut libc::c_void);
    free(self_addr as *mut libc::c_void);
}

unsafe fn update_gossip_peerstates(
    context: &dc_context_t,
    message_time: time_t,
    imffields: *mut mailimf_fields,
    gossip_headers: *const mailimf_fields,
) -> *mut dc_hash_t {
    let mut cur1: *mut clistiter;
    let mut recipients: *mut dc_hash_t = 0 as *mut dc_hash_t;
    let mut gossipped_addr: *mut dc_hash_t = 0 as *mut dc_hash_t;
    cur1 = (*(*gossip_headers).fld_list).first;
    while !cur1.is_null() {
        let field: *mut mailimf_field = (if !cur1.is_null() {
            (*cur1).data
        } else {
            0 as *mut libc::c_void
        }) as *mut mailimf_field;
        if (*field).fld_type == MAILIMF_FIELD_OPTIONAL_FIELD as libc::c_int {
            let optional_field: *const mailimf_optional_field =
                (*field).fld_data.fld_optional_field;
            if !optional_field.is_null()
                && !(*optional_field).fld_name.is_null()
                && strcasecmp(
                    (*optional_field).fld_name,
                    b"Autocrypt-Gossip\x00" as *const u8 as *const libc::c_char,
                ) == 0i32
            {
                let value = CStr::from_ptr((*optional_field).fld_value)
                    .to_str()
                    .unwrap();
                let gossip_header = Aheader::from_str(value);
                if let Ok(ref header) = gossip_header {
                    if recipients.is_null() {
                        recipients = mailimf_get_recipients(imffields)
                    }
                    if !dc_hash_find(
                        recipients,
                        CString::new(header.addr.clone()).unwrap().as_ptr() as *const libc::c_void,
                        header.addr.len() as i32,
                    )
                    .is_null()
                    {
                        let mut peerstate = dc_apeerstate_new(context);
                        if 0 == dc_apeerstate_load_by_addr(
                            &mut peerstate,
                            &context.sql.clone().read().unwrap(),
                            CString::new(header.addr.clone()).unwrap().as_ptr(),
                        ) {
                            dc_apeerstate_init_from_gossip(&mut peerstate, header, message_time);
                            dc_apeerstate_save_to_db(
                                &mut peerstate,
                                &context.sql.clone().read().unwrap(),
                                1i32,
                            );
                        } else {
                            dc_apeerstate_apply_gossip(&mut peerstate, header, message_time);
                            dc_apeerstate_save_to_db(
                                &mut peerstate,
                                &context.sql.clone().read().unwrap(),
                                0i32,
                            );
                        }
                        if 0 != peerstate.degrade_event {
                            dc_handle_degrade_event(context, &peerstate);
                        }
                        dc_apeerstate_unref(&mut peerstate);
                        if gossipped_addr.is_null() {
                            gossipped_addr =
                                malloc(::std::mem::size_of::<dc_hash_t>()) as *mut dc_hash_t;
                            dc_hash_init(gossipped_addr, 3i32, 1i32);
                        }
                        dc_hash_insert(
                            gossipped_addr,
                            CString::new(header.addr.clone()).unwrap().as_ptr()
                                as *const libc::c_void,
                            header.addr.len() as libc::c_int,
                            1i32 as *mut libc::c_void,
                        );
                    } else {
                        dc_log_info(
                            context,
                            0i32,
                            b"Ignoring gossipped \"%s\" as the address is not in To/Cc list.\x00"
                                as *const u8 as *const libc::c_char,
                            CString::new(header.addr.clone()).unwrap().as_ptr(),
                        );
                    }
                }
            }
        }
        cur1 = if !cur1.is_null() {
            (*cur1).next
        } else {
            0 as *mut clistcell
        }
    }
    if !recipients.is_null() {
        dc_hash_clear(recipients);
        free(recipients as *mut libc::c_void);
    }

    gossipped_addr
}

// TODO should return bool /rtn
unsafe fn decrypt_recursive(
    context: &dc_context_t,
    mime: *mut mailmime,
    private_keyring: &Keyring,
    public_keyring_for_validate: &Keyring,
    ret_valid_signatures: *mut dc_hash_t,
    ret_gossip_headers: *mut *mut mailimf_fields,
    ret_has_unencrypted_parts: *mut libc::c_int,
) -> libc::c_int {
    let ct: *mut mailmime_content;
    let mut cur: *mut clistiter;
    if mime.is_null() {
        return 0i32;
    }
    if (*mime).mm_type == MAILMIME_MULTIPLE as libc::c_int {
        ct = (*mime).mm_content_type;
        if !ct.is_null()
            && !(*ct).ct_subtype.is_null()
            && strcmp(
                (*ct).ct_subtype,
                b"encrypted\x00" as *const u8 as *const libc::c_char,
            ) == 0i32
        {
            cur = (*(*mime).mm_data.mm_multipart.mm_mp_list).first;
            while !cur.is_null() {
                let mut decrypted_mime: *mut mailmime = 0 as *mut mailmime;
                if 0 != decrypt_part(
                    context,
                    (if !cur.is_null() {
                        (*cur).data
                    } else {
                        0 as *mut libc::c_void
                    }) as *mut mailmime,
                    private_keyring,
                    public_keyring_for_validate,
                    ret_valid_signatures,
                    &mut decrypted_mime,
                ) {
                    if (*ret_gossip_headers).is_null() && (*ret_valid_signatures).count > 0i32 {
                        let mut dummy: size_t = 0i32 as size_t;
                        let mut test: *mut mailimf_fields = 0 as *mut mailimf_fields;
                        if mailimf_envelope_and_optional_fields_parse(
                            (*decrypted_mime).mm_mime_start,
                            (*decrypted_mime).mm_length,
                            &mut dummy,
                            &mut test,
                        ) == MAILIMF_NO_ERROR as libc::c_int
                            && !test.is_null()
                        {
                            *ret_gossip_headers = test
                        }
                    }
                    mailmime_substitute(mime, decrypted_mime);
                    mailmime_free(mime);
                    return 1i32;
                }
                cur = if !cur.is_null() {
                    (*cur).next
                } else {
                    0 as *mut clistcell
                }
            }
            *ret_has_unencrypted_parts = 1i32
        } else {
            cur = (*(*mime).mm_data.mm_multipart.mm_mp_list).first;
            while !cur.is_null() {
                if 0 != decrypt_recursive(
                    context,
                    (if !cur.is_null() {
                        (*cur).data
                    } else {
                        0 as *mut libc::c_void
                    }) as *mut mailmime,
                    private_keyring,
                    public_keyring_for_validate,
                    ret_valid_signatures,
                    ret_gossip_headers,
                    ret_has_unencrypted_parts,
                ) {
                    return 1i32;
                }
                cur = if !cur.is_null() {
                    (*cur).next
                } else {
                    0 as *mut clistcell
                }
            }
        }
    } else if (*mime).mm_type == MAILMIME_MESSAGE as libc::c_int {
        if 0 != decrypt_recursive(
            context,
            (*mime).mm_data.mm_message.mm_msg_mime,
            private_keyring,
            public_keyring_for_validate,
            ret_valid_signatures,
            ret_gossip_headers,
            ret_has_unencrypted_parts,
        ) {
            return 1i32;
        }
    } else {
        *ret_has_unencrypted_parts = 1i32
    }

    0
}

unsafe fn decrypt_part(
    _context: &dc_context_t,
    mime: *mut mailmime,
    private_keyring: &Keyring,
    public_keyring_for_validate: &Keyring,
    ret_valid_signatures: *mut dc_hash_t,
    ret_decrypted_mime: *mut *mut mailmime,
) -> libc::c_int {
    let add_signatures: *mut dc_hash_t;
    let current_block: u64;
    let mime_data: *mut mailmime_data;
    let mut mime_transfer_encoding: libc::c_int = MAILMIME_MECHANISM_BINARY as libc::c_int;
    /* mmap_string_unref()'d if set */
    let mut transfer_decoding_buffer: *mut libc::c_char = 0 as *mut libc::c_char;
    /* must not be free()'d */
    let mut decoded_data: *const libc::c_char = 0 as *const libc::c_char;
    let mut decoded_data_bytes: size_t = 0i32 as size_t;
    let mut sth_decrypted: libc::c_int = 0i32;
    *ret_decrypted_mime = 0 as *mut mailmime;
    mime_data = (*mime).mm_data.mm_single;
    /* MAILMIME_DATA_FILE indicates, the data is in a file; AFAIK this is not used on parsing */
    if !((*mime_data).dt_type != MAILMIME_DATA_TEXT as libc::c_int
        || (*mime_data).dt_data.dt_text.dt_data.is_null()
        || (*mime_data).dt_data.dt_text.dt_length <= 0)
    {
        if !(*mime).mm_mime_fields.is_null() {
            let mut cur: *mut clistiter;
            cur = (*(*(*mime).mm_mime_fields).fld_list).first;
            while !cur.is_null() {
                let field: *mut mailmime_field = (if !cur.is_null() {
                    (*cur).data
                } else {
                    0 as *mut libc::c_void
                }) as *mut mailmime_field;
                if !field.is_null() {
                    if (*field).fld_type == MAILMIME_FIELD_TRANSFER_ENCODING as libc::c_int
                        && !(*field).fld_data.fld_encoding.is_null()
                    {
                        mime_transfer_encoding = (*(*field).fld_data.fld_encoding).enc_type
                    }
                }
                cur = if !cur.is_null() {
                    (*cur).next
                } else {
                    0 as *mut clistcell
                }
            }
        }
        /* regard `Content-Transfer-Encoding:` */
        if mime_transfer_encoding == MAILMIME_MECHANISM_7BIT as libc::c_int
            || mime_transfer_encoding == MAILMIME_MECHANISM_8BIT as libc::c_int
            || mime_transfer_encoding == MAILMIME_MECHANISM_BINARY as libc::c_int
        {
            decoded_data = (*mime_data).dt_data.dt_text.dt_data;
            decoded_data_bytes = (*mime_data).dt_data.dt_text.dt_length;
            if decoded_data.is_null() || decoded_data_bytes <= 0 {
                /* no error - but no data */
                current_block = 2554982661806928548;
            } else {
                current_block = 4488286894823169796;
            }
        } else {
            let r: libc::c_int;
            let mut current_index: size_t = 0i32 as size_t;
            r = mailmime_part_parse(
                (*mime_data).dt_data.dt_text.dt_data,
                (*mime_data).dt_data.dt_text.dt_length,
                &mut current_index,
                mime_transfer_encoding,
                &mut transfer_decoding_buffer,
                &mut decoded_data_bytes,
            );
            if r != MAILIMF_NO_ERROR as libc::c_int
                || transfer_decoding_buffer.is_null()
                || decoded_data_bytes <= 0
            {
                current_block = 2554982661806928548;
            } else {
                decoded_data = transfer_decoding_buffer;
                current_block = 4488286894823169796;
            }
        }
        match current_block {
            2554982661806928548 => {}
            _ => {
                /* encrypted, decoded data in decoded_data now ... */
                if !(0 == has_decrypted_pgp_armor(decoded_data, decoded_data_bytes as libc::c_int))
                {
                    add_signatures = if (*ret_valid_signatures).count <= 0i32 {
                        ret_valid_signatures
                    } else {
                        0 as *mut dc_hash_t
                    };
                    /*if we already have fingerprints, do not add more; this ensures, only the fingerprints from the outer-most part are collected */

                    if let Some(plain) = dc_pgp_pk_decrypt(
                        decoded_data as *const libc::c_void,
                        decoded_data_bytes,
                        &private_keyring,
                        &public_keyring_for_validate,
                        add_signatures,
                    ) {
                        let plain_bytes = plain.len();
                        let plain_c = CString::new(plain).unwrap();
                        let plain_buf = strdup(plain_c.as_ptr());

                        let mut index: size_t = 0i32 as size_t;
                        let mut decrypted_mime: *mut mailmime = 0 as *mut mailmime;
                        if mailmime_parse(
                            plain_buf as *const libc::c_char,
                            plain_bytes,
                            &mut index,
                            &mut decrypted_mime,
                        ) != MAIL_NO_ERROR as libc::c_int
                            || decrypted_mime.is_null()
                        {
                            if !decrypted_mime.is_null() {
                                mailmime_free(decrypted_mime);
                            }
                        } else {
                            *ret_decrypted_mime = decrypted_mime;
                            sth_decrypted = 1i32
                        }
                    }
                }
            }
        }
    }
    //mailmime_substitute(mime, new_mime);
    //s. mailprivacy_gnupg.c::pgp_decrypt()
    if !transfer_decoding_buffer.is_null() {
        mmap_string_unref(transfer_decoding_buffer);
    }

    sth_decrypted
}

/*******************************************************************************
 * Decrypt
 ******************************************************************************/
// TODO should return bool /rtn
unsafe fn has_decrypted_pgp_armor(
    str__: *const libc::c_char,
    mut str_bytes: libc::c_int,
) -> libc::c_int {
    let str_end: *const libc::c_uchar = (str__ as *const libc::c_uchar).offset(str_bytes as isize);
    let mut p: *const libc::c_uchar = str__ as *const libc::c_uchar;
    while p < str_end {
        if *p as libc::c_int > ' ' as i32 {
            break;
        }
        p = p.offset(1isize);
        str_bytes -= 1
    }
    if str_bytes > 27i32
        && strncmp(
            p as *const libc::c_char,
            b"-----BEGIN PGP MESSAGE-----\x00" as *const u8 as *const libc::c_char,
            27,
        ) == 0
    {
        return 1;
    }

    0
}

/**
 * Check if a MIME structure contains a multipart/report part.
 *
 * As reports are often unencrypted, we do not reset the Autocrypt header in
 * this case.
 *
 * However, Delta Chat itself has no problem with encrypted multipart/report
 * parts and MUAs should be encouraged to encrpyt multipart/reports as well so
 * that we could use the normal Autocrypt processing.
 *
 * @private
 * @param mime The mime struture to check
 * @return 1=multipart/report found in MIME, 0=no multipart/report found
 */
// TODO should return bool /rtn
unsafe fn contains_report(mime: *mut mailmime) -> libc::c_int {
    if (*mime).mm_type == MAILMIME_MULTIPLE as libc::c_int {
        if (*(*(*mime).mm_content_type).ct_type).tp_type
            == MAILMIME_TYPE_COMPOSITE_TYPE as libc::c_int
            && (*(*(*(*mime).mm_content_type).ct_type)
                .tp_data
                .tp_composite_type)
                .ct_type
                == MAILMIME_COMPOSITE_TYPE_MULTIPART as libc::c_int
            && strcmp(
                (*(*mime).mm_content_type).ct_subtype,
                b"report\x00" as *const u8 as *const libc::c_char,
            ) == 0i32
        {
            return 1i32;
        }
        let mut cur: *mut clistiter;
        cur = (*(*mime).mm_data.mm_multipart.mm_mp_list).first;
        while !cur.is_null() {
            if 0 != contains_report(
                (if !cur.is_null() {
                    (*cur).data
                } else {
                    0 as *mut libc::c_void
                }) as *mut mailmime,
            ) {
                return 1i32;
            }
            cur = if !cur.is_null() {
                (*cur).next
            } else {
                0 as *mut clistcell
            }
        }
    } else if (*mime).mm_type == MAILMIME_MESSAGE as libc::c_int {
        if 0 != contains_report((*mime).mm_data.mm_message.mm_msg_mime) {
            return 1i32;
        }
    }

    0
}

/* frees data referenced by "mailmime" but not freed by mailmime_free(). After calling this function, in_out_message cannot be used any longer! */
pub unsafe fn dc_e2ee_thanks(mut helper: *mut dc_e2ee_helper_t) {
    if helper.is_null() {
        return;
    }
    free((*helper).cdata_to_free);
    (*helper).cdata_to_free = 0 as *mut libc::c_void;
    if !(*helper).gossipped_addr.is_null() {
        dc_hash_clear((*helper).gossipped_addr);
        free((*helper).gossipped_addr as *mut libc::c_void);
        (*helper).gossipped_addr = 0 as *mut dc_hash_t
    }
    if !(*helper).signatures.is_null() {
        dc_hash_clear((*helper).signatures);
        free((*helper).signatures as *mut libc::c_void);
        (*helper).signatures = 0 as *mut dc_hash_t
    };
}

/* makes sure, the private key exists, needed only for exporting keys and the case no message was sent before */
// TODO should return bool /rtn
pub unsafe fn dc_ensure_secret_key_exists(context: &dc_context_t) -> libc::c_int {
    /* normally, the key is generated as soon as the first mail is send
    (this is to gain some extra-random-seed by the message content and the timespan between program start and message sending) */
    let mut success: libc::c_int = 0i32;

    let self_addr = dc_sqlite3_get_config(
        context,
        &context.sql.clone().read().unwrap(),
        b"configured_addr\x00" as *const u8 as *const libc::c_char,
        0 as *const libc::c_char,
    );
    if self_addr.is_null() {
        dc_log_warning(
            context,
            0i32,
            b"Cannot ensure secret key if context is not configured.\x00" as *const u8
                as *const libc::c_char,
        );
    } else if load_or_generate_self_public_key(context, self_addr, 0 as *mut mailmime).is_some() {
        /*no random text data for seeding available*/
        success = 1i32
    }

    free(self_addr as *mut libc::c_void);

    success
}
