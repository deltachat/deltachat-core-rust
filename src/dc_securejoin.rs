use mmime::mailimf_types::*;

use crate::aheader::EncryptPreference;
use crate::constants::Event;
use crate::dc_array::*;
use crate::dc_chat::*;
use crate::dc_configure::*;
use crate::dc_contact::*;
use crate::dc_context::dc_context_t;
use crate::dc_e2ee::*;
use crate::dc_log::*;
use crate::dc_lot::*;
use crate::dc_mimeparser::*;
use crate::dc_msg::*;
use crate::dc_param::*;
use crate::dc_qr::*;
use crate::dc_sqlite3::*;
use crate::dc_stock::*;
use crate::dc_strencode::*;
use crate::dc_token::*;
use crate::dc_tools::*;
use crate::key::*;
use crate::peerstate::*;
use crate::types::*;
use crate::x::*;

pub unsafe fn dc_get_securejoin_qr(
    context: &dc_context_t,
    group_chat_id: uint32_t,
) -> *mut libc::c_char {
    let current_block: u64;
    /* =========================================================
    ====             Alice - the inviter side            ====
    ====   Step 1 in "Setup verified contact" protocol   ====
    ========================================================= */
    let mut qr: *mut libc::c_char = 0 as *mut libc::c_char;
    let self_addr: *mut libc::c_char;
    let mut self_addr_urlencoded: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut self_name: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut self_name_urlencoded: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut fingerprint: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut invitenumber: *mut libc::c_char;
    let mut auth: *mut libc::c_char;
    let mut chat: *mut dc_chat_t = 0 as *mut dc_chat_t;
    let mut group_name: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut group_name_urlencoded: *mut libc::c_char = 0 as *mut libc::c_char;

    dc_ensure_secret_key_exists(context);
    invitenumber = dc_token_lookup(context, DC_TOKEN_INVITENUMBER, group_chat_id);
    if invitenumber.is_null() {
        invitenumber = dc_create_id();
        dc_token_save(context, DC_TOKEN_INVITENUMBER, group_chat_id, invitenumber);
    }
    auth = dc_token_lookup(context, DC_TOKEN_AUTH, group_chat_id);
    if auth.is_null() {
        auth = dc_create_id();
        dc_token_save(context, DC_TOKEN_AUTH, group_chat_id, auth);
    }
    self_addr = dc_sqlite3_get_config(
        context,
        &context.sql.clone().read().unwrap(),
        b"configured_addr\x00" as *const u8 as *const libc::c_char,
        0 as *const libc::c_char,
    );
    if self_addr.is_null() {
        dc_log_error(
            context,
            0i32,
            b"Not configured, cannot generate QR code.\x00" as *const u8 as *const libc::c_char,
        );
    } else {
        self_name = dc_sqlite3_get_config(
            context,
            &context.sql.clone().read().unwrap(),
            b"displayname\x00" as *const u8 as *const libc::c_char,
            b"\x00" as *const u8 as *const libc::c_char,
        );
        fingerprint = get_self_fingerprint(context);
        if !fingerprint.is_null() {
            self_addr_urlencoded = dc_urlencode(self_addr);
            self_name_urlencoded = dc_urlencode(self_name);
            if 0 != group_chat_id {
                chat = dc_get_chat(context, group_chat_id);
                if chat.is_null() {
                    dc_log_error(
                        context,
                        0i32,
                        b"Cannot get QR-code for chat-id %i\x00" as *const u8
                            as *const libc::c_char,
                        group_chat_id,
                    );
                    current_block = 9531737720721467826;
                } else {
                    group_name = dc_chat_get_name(chat);
                    group_name_urlencoded = dc_urlencode(group_name);
                    qr = dc_mprintf(
                        b"OPENPGP4FPR:%s#a=%s&g=%s&x=%s&i=%s&s=%s\x00" as *const u8
                            as *const libc::c_char,
                        fingerprint,
                        self_addr_urlencoded,
                        group_name_urlencoded,
                        (*chat).grpid,
                        invitenumber,
                        auth,
                    );
                    current_block = 1118134448028020070;
                }
            } else {
                qr = dc_mprintf(
                    b"OPENPGP4FPR:%s#a=%s&n=%s&i=%s&s=%s\x00" as *const u8 as *const libc::c_char,
                    fingerprint,
                    self_addr_urlencoded,
                    self_name_urlencoded,
                    invitenumber,
                    auth,
                );
                current_block = 1118134448028020070;
            }
            match current_block {
                9531737720721467826 => {}
                _ => {
                    dc_log_info(
                        context,
                        0i32,
                        b"Generated QR code: %s\x00" as *const u8 as *const libc::c_char,
                        qr,
                    );
                }
            }
        }
    }

    free(self_addr_urlencoded as *mut libc::c_void);
    free(self_addr as *mut libc::c_void);
    free(self_name as *mut libc::c_void);
    free(self_name_urlencoded as *mut libc::c_void);
    free(fingerprint as *mut libc::c_void);
    free(invitenumber as *mut libc::c_void);
    free(auth as *mut libc::c_void);
    dc_chat_unref(chat);
    free(group_name as *mut libc::c_void);
    free(group_name_urlencoded as *mut libc::c_void);
    return if !qr.is_null() {
        qr
    } else {
        dc_strdup(0 as *const libc::c_char)
    };
}

unsafe fn get_self_fingerprint(context: &dc_context_t) -> *mut libc::c_char {
    let self_addr = dc_sqlite3_get_config(
        context,
        &context.sql.clone().read().unwrap(),
        b"configured_addr\x00" as *const u8 as *const libc::c_char,
        0 as *const libc::c_char,
    );
    if self_addr.is_null() {
        return std::ptr::null_mut();
    }

    if let Some(key) =
        Key::from_self_public(context, self_addr, &context.sql.clone().read().unwrap())
    {
        return key.fingerprint_c();
    }

    std::ptr::null_mut()
}

pub unsafe fn dc_join_securejoin(context: &dc_context_t, qr: *const libc::c_char) -> uint32_t {
    /* ==========================================================
    ====             Bob - the joiner's side             =====
    ====   Step 2 in "Setup verified contact" protocol   =====
    ========================================================== */
    let mut ret_chat_id: libc::c_int = 0i32;
    let ongoing_allocated: libc::c_int;
    let mut contact_chat_id: uint32_t = 0i32 as uint32_t;
    let mut join_vg: libc::c_int = 0i32;
    let mut qr_scan: *mut dc_lot_t = 0 as *mut dc_lot_t;
    dc_log_info(
        context,
        0i32,
        b"Requesting secure-join ...\x00" as *const u8 as *const libc::c_char,
    );
    dc_ensure_secret_key_exists(context);
    ongoing_allocated = dc_alloc_ongoing(context);
    if !(ongoing_allocated == 0i32) {
        qr_scan = dc_check_qr(context, qr);
        if qr_scan.is_null() || (*qr_scan).state != 200i32 && (*qr_scan).state != 202i32 {
            dc_log_error(
                context,
                0i32,
                b"Unknown QR code.\x00" as *const u8 as *const libc::c_char,
            );
        } else {
            contact_chat_id = dc_create_chat_by_contact_id(context, (*qr_scan).id);
            if contact_chat_id == 0i32 as libc::c_uint {
                dc_log_error(
                    context,
                    0i32,
                    b"Unknown contact.\x00" as *const u8 as *const libc::c_char,
                );
            } else if !(context
                .running_state
                .clone()
                .read()
                .unwrap()
                .shall_stop_ongoing)
            {
                join_vg = ((*qr_scan).state == 202i32) as libc::c_int;
                let bob_a = context.bob.clone();
                let mut bob = bob_a.write().unwrap();
                bob.status = 0;
                bob.qr_scan = qr_scan;
                if 0 != fingerprint_equals_sender(context, (*qr_scan).fingerprint, contact_chat_id)
                {
                    dc_log_info(
                        context,
                        0i32,
                        b"Taking protocol shortcut.\x00" as *const u8 as *const libc::c_char,
                    );
                    bob.expects = 6;
                    (context.cb)(
                        context,
                        Event::SECUREJOIN_JOINER_PROGRESS,
                        chat_id_2_contact_id(context, contact_chat_id) as uintptr_t,
                        400i32 as uintptr_t,
                    );
                    let own_fingerprint: *mut libc::c_char = get_self_fingerprint(context);
                    send_handshake_msg(
                        context,
                        contact_chat_id,
                        if 0 != join_vg {
                            b"vg-request-with-auth\x00" as *const u8 as *const libc::c_char
                        } else {
                            b"vc-request-with-auth\x00" as *const u8 as *const libc::c_char
                        },
                        (*qr_scan).auth,
                        own_fingerprint,
                        if 0 != join_vg {
                            (*qr_scan).text2
                        } else {
                            0 as *mut libc::c_char
                        },
                    );
                    free(own_fingerprint as *mut libc::c_void);
                } else {
                    bob.expects = 2;
                    send_handshake_msg(
                        context,
                        contact_chat_id,
                        if 0 != join_vg {
                            b"vg-request\x00" as *const u8 as *const libc::c_char
                        } else {
                            b"vc-request\x00" as *const u8 as *const libc::c_char
                        },
                        (*qr_scan).invitenumber,
                        0 as *const libc::c_char,
                        0 as *const libc::c_char,
                    );
                }
                // Bob -> Alice
                while !(context
                    .running_state
                    .clone()
                    .read()
                    .unwrap()
                    .shall_stop_ongoing)
                {
                    std::thread::sleep(std::time::Duration::from_micros(300 * 1000));
                }
            }
        }
    }
    let bob_a = context.bob.clone();
    let mut bob = bob_a.write().unwrap();

    bob.expects = 0;
    if bob.status == 1 {
        if 0 != join_vg {
            ret_chat_id = dc_get_chat_id_by_grpid(
                context,
                (*qr_scan).text2,
                0 as *mut libc::c_int,
                0 as *mut libc::c_int,
            ) as libc::c_int
        } else {
            ret_chat_id = contact_chat_id as libc::c_int
        }
    }
    bob.qr_scan = std::ptr::null_mut();

    dc_lot_unref(qr_scan);
    if 0 != ongoing_allocated {
        dc_free_ongoing(context);
    }
    ret_chat_id as uint32_t
}

unsafe fn send_handshake_msg(
    context: &dc_context_t,
    contact_chat_id: uint32_t,
    step: *const libc::c_char,
    param2: *const libc::c_char,
    fingerprint: *const libc::c_char,
    grpid: *const libc::c_char,
) {
    let mut msg: *mut dc_msg_t = dc_msg_new_untyped(context);
    (*msg).type_0 = 10i32;
    (*msg).text = dc_mprintf(
        b"Secure-Join: %s\x00" as *const u8 as *const libc::c_char,
        step,
    );
    (*msg).hidden = 1i32;
    dc_param_set_int((*msg).param, 'S' as i32, 7i32);
    dc_param_set((*msg).param, 'E' as i32, step);
    if !param2.is_null() {
        dc_param_set((*msg).param, 'F' as i32, param2);
    }
    if !fingerprint.is_null() {
        dc_param_set((*msg).param, 'G' as i32, fingerprint);
    }
    if !grpid.is_null() {
        dc_param_set((*msg).param, 'H' as i32, grpid);
    }
    if strcmp(step, b"vg-request\x00" as *const u8 as *const libc::c_char) == 0i32
        || strcmp(step, b"vc-request\x00" as *const u8 as *const libc::c_char) == 0i32
    {
        dc_param_set_int((*msg).param, 'u' as i32, 1i32);
    } else {
        dc_param_set_int((*msg).param, 'c' as i32, 1i32);
    }
    dc_send_msg(context, contact_chat_id, msg);
    dc_msg_unref(msg);
}

unsafe fn chat_id_2_contact_id(context: &dc_context_t, contact_chat_id: uint32_t) -> uint32_t {
    let mut contact_id: uint32_t = 0i32 as uint32_t;
    let contacts: *mut dc_array_t = dc_get_chat_contacts(context, contact_chat_id);
    if !(dc_array_get_cnt(contacts) != 1) {
        contact_id = dc_array_get_id(contacts, 0i32 as size_t)
    }
    dc_array_unref(contacts);

    contact_id
}

unsafe fn fingerprint_equals_sender(
    context: &dc_context_t,
    fingerprint: *const libc::c_char,
    contact_chat_id: uint32_t,
) -> libc::c_int {
    let mut fingerprint_equal: libc::c_int = 0i32;
    let contacts: *mut dc_array_t = dc_get_chat_contacts(context, contact_chat_id);
    let contact: *mut dc_contact_t = dc_contact_new(context);

    if !(dc_array_get_cnt(contacts) != 1) {
        let peerstate = Peerstate::from_addr(
            context,
            &context.sql.clone().read().unwrap(),
            to_str((*contact).addr),
        );
        if !(!dc_contact_load_from_db(
            contact,
            &context.sql.clone().read().unwrap(),
            dc_array_get_id(contacts, 0i32 as size_t),
        ) || peerstate.is_some())
        {
            let peerstate = peerstate.as_ref().unwrap();
            let fingerprint_normalized = dc_normalize_fingerprint(to_str(fingerprint));
            if peerstate.public_key_fingerprint.is_some()
                && &fingerprint_normalized == peerstate.public_key_fingerprint.as_ref().unwrap()
            {
                fingerprint_equal = 1;
            }
        }
    }
    dc_contact_unref(contact);
    dc_array_unref(contacts);

    fingerprint_equal
}

/* library private: secure-join */
pub unsafe fn dc_handle_securejoin_handshake(
    context: &dc_context_t,
    mimeparser: &dc_mimeparser_t,
    contact_id: uint32_t,
) -> libc::c_int {
    let mut current_block: u64;
    let step: *const libc::c_char;
    let join_vg: libc::c_int;
    let mut scanned_fingerprint_of_alice: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut auth: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut own_fingerprint: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut contact_chat_id: uint32_t = 0i32 as uint32_t;
    let mut contact_chat_id_blocked: libc::c_int = 0i32;
    let mut grpid: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut ret: libc::c_int = 0i32;
    let mut contact: *mut dc_contact_t = 0 as *mut dc_contact_t;
    if !(contact_id <= 9i32 as libc::c_uint) {
        step = lookup_field(
            mimeparser,
            b"Secure-Join\x00" as *const u8 as *const libc::c_char,
        );
        if !step.is_null() {
            dc_log_info(
                context,
                0i32,
                b">>>>>>>>>>>>>>>>>>>>>>>>> secure-join message \'%s\' received\x00" as *const u8
                    as *const libc::c_char,
                step,
            );
            join_vg = (strncmp(step, b"vg-\x00" as *const u8 as *const libc::c_char, 3) == 0)
                as libc::c_int;
            dc_create_or_lookup_nchat_by_contact_id(
                context,
                contact_id,
                0i32,
                &mut contact_chat_id,
                &mut contact_chat_id_blocked,
            );
            if 0 != contact_chat_id_blocked {
                dc_unblock_chat(context, contact_chat_id);
            }
            ret = 0x2i32;
            if strcmp(step, b"vg-request\x00" as *const u8 as *const libc::c_char) == 0i32
                || strcmp(step, b"vc-request\x00" as *const u8 as *const libc::c_char) == 0i32
            {
                /* =========================================================
                ====             Alice - the inviter side            ====
                ====   Step 3 in "Setup verified contact" protocol   ====
                ========================================================= */
                // this message may be unencrypted (Bob, the joinder and the sender, might not have Alice's key yet)
                // it just ensures, we have Bobs key now. If we do _not_ have the key because eg. MitM has removed it,
                // send_message() will fail with the error "End-to-end-encryption unavailable unexpectedly.", so, there is no additional check needed here.
                // verify that the `Secure-Join-Invitenumber:`-header matches invitenumber written to the QR code
                let invitenumber: *const libc::c_char;
                invitenumber = lookup_field(
                    mimeparser,
                    b"Secure-Join-Invitenumber\x00" as *const u8 as *const libc::c_char,
                );
                if invitenumber.is_null() {
                    dc_log_warning(
                        context,
                        0i32,
                        b"Secure-join denied (invitenumber missing).\x00" as *const u8
                            as *const libc::c_char,
                    );
                    current_block = 4378276786830486580;
                } else if dc_token_exists(context, DC_TOKEN_INVITENUMBER, invitenumber) == 0i32 {
                    dc_log_warning(
                        context,
                        0i32,
                        b"Secure-join denied (bad invitenumber).\x00" as *const u8
                            as *const libc::c_char,
                    );
                    current_block = 4378276786830486580;
                } else {
                    dc_log_info(
                        context,
                        0i32,
                        b"Secure-join requested.\x00" as *const u8 as *const libc::c_char,
                    );
                    (context.cb)(
                        context,
                        Event::SECUREJOIN_INVITER_PROGRESS,
                        contact_id as uintptr_t,
                        300i32 as uintptr_t,
                    );
                    send_handshake_msg(
                        context,
                        contact_chat_id,
                        if 0 != join_vg {
                            b"vg-auth-required\x00" as *const u8 as *const libc::c_char
                        } else {
                            b"vc-auth-required\x00" as *const u8 as *const libc::c_char
                        },
                        0 as *const libc::c_char,
                        0 as *const libc::c_char,
                        0 as *const libc::c_char,
                    );
                    current_block = 10256747982273457880;
                }
            } else if strcmp(
                step,
                b"vg-auth-required\x00" as *const u8 as *const libc::c_char,
            ) == 0i32
                || strcmp(
                    step,
                    b"vc-auth-required\x00" as *const u8 as *const libc::c_char,
                ) == 0i32
            {
                let bob_a = context.bob.clone();
                let bob = bob_a.read().unwrap();
                let scan = bob.qr_scan;
                if scan.is_null() || bob.expects != 2i32 || 0 != join_vg && (*scan).state != 202i32
                {
                    dc_log_warning(
                        context,
                        0i32,
                        b"auth-required message out of sync.\x00" as *const u8
                            as *const libc::c_char,
                    );
                    // no error, just aborted somehow or a mail from another handshake
                    current_block = 4378276786830486580;
                } else {
                    scanned_fingerprint_of_alice = dc_strdup((*scan).fingerprint);
                    auth = dc_strdup((*scan).auth);
                    if 0 != join_vg {
                        grpid = dc_strdup((*scan).text2)
                    }
                    if 0 == encrypted_and_signed(mimeparser, scanned_fingerprint_of_alice) {
                        could_not_establish_secure_connection(
                            context,
                            contact_chat_id,
                            if 0 != mimeparser.e2ee_helper.encrypted {
                                b"No valid signature.\x00" as *const u8 as *const libc::c_char
                            } else {
                                b"Not encrypted.\x00" as *const u8 as *const libc::c_char
                            },
                        );
                        end_bobs_joining(context, 0i32);
                        current_block = 4378276786830486580;
                    } else if 0
                        == fingerprint_equals_sender(
                            context,
                            scanned_fingerprint_of_alice,
                            contact_chat_id,
                        )
                    {
                        could_not_establish_secure_connection(
                            context,
                            contact_chat_id,
                            b"Fingerprint mismatch on joiner-side.\x00" as *const u8
                                as *const libc::c_char,
                        );
                        end_bobs_joining(context, 0i32);
                        current_block = 4378276786830486580;
                    } else {
                        dc_log_info(
                            context,
                            0i32,
                            b"Fingerprint verified.\x00" as *const u8 as *const libc::c_char,
                        );
                        own_fingerprint = get_self_fingerprint(context);
                        (context.cb)(
                            context,
                            Event::SECUREJOIN_JOINER_PROGRESS,
                            contact_id as uintptr_t,
                            400i32 as uintptr_t,
                        );
                        context.bob.clone().write().unwrap().expects = 6;

                        send_handshake_msg(
                            context,
                            contact_chat_id,
                            if 0 != join_vg {
                                b"vg-request-with-auth\x00" as *const u8 as *const libc::c_char
                            } else {
                                b"vc-request-with-auth\x00" as *const u8 as *const libc::c_char
                            },
                            auth,
                            own_fingerprint,
                            grpid,
                        );
                        current_block = 10256747982273457880;
                    }
                }
            } else if strcmp(
                step,
                b"vg-request-with-auth\x00" as *const u8 as *const libc::c_char,
            ) == 0i32
                || strcmp(
                    step,
                    b"vc-request-with-auth\x00" as *const u8 as *const libc::c_char,
                ) == 0i32
            {
                /* ============================================================
                ====              Alice - the inviter side              ====
                ====   Steps 5+6 in "Setup verified contact" protocol   ====
                ====  Step 6 in "Out-of-band verified groups" protocol  ====
                ============================================================ */
                // verify that Secure-Join-Fingerprint:-header matches the fingerprint of Bob
                let fingerprint: *const libc::c_char;
                fingerprint = lookup_field(
                    mimeparser,
                    b"Secure-Join-Fingerprint\x00" as *const u8 as *const libc::c_char,
                );
                if fingerprint.is_null() {
                    could_not_establish_secure_connection(
                        context,
                        contact_chat_id,
                        b"Fingerprint not provided.\x00" as *const u8 as *const libc::c_char,
                    );
                    current_block = 4378276786830486580;
                } else if 0 == encrypted_and_signed(mimeparser, fingerprint) {
                    could_not_establish_secure_connection(
                        context,
                        contact_chat_id,
                        b"Auth not encrypted.\x00" as *const u8 as *const libc::c_char,
                    );
                    current_block = 4378276786830486580;
                } else if 0 == fingerprint_equals_sender(context, fingerprint, contact_chat_id) {
                    could_not_establish_secure_connection(
                        context,
                        contact_chat_id,
                        b"Fingerprint mismatch on inviter-side.\x00" as *const u8
                            as *const libc::c_char,
                    );
                    current_block = 4378276786830486580;
                } else {
                    dc_log_info(
                        context,
                        0i32,
                        b"Fingerprint verified.\x00" as *const u8 as *const libc::c_char,
                    );
                    // verify that the `Secure-Join-Auth:`-header matches the secret written to the QR code
                    let auth_0: *const libc::c_char;
                    auth_0 = lookup_field(
                        mimeparser,
                        b"Secure-Join-Auth\x00" as *const u8 as *const libc::c_char,
                    );
                    if auth_0.is_null() {
                        could_not_establish_secure_connection(
                            context,
                            contact_chat_id,
                            b"Auth not provided.\x00" as *const u8 as *const libc::c_char,
                        );
                        current_block = 4378276786830486580;
                    } else if dc_token_exists(context, DC_TOKEN_AUTH, auth_0) == 0i32 {
                        could_not_establish_secure_connection(
                            context,
                            contact_chat_id,
                            b"Auth invalid.\x00" as *const u8 as *const libc::c_char,
                        );
                        current_block = 4378276786830486580;
                    } else if 0 == mark_peer_as_verified(context, fingerprint) {
                        could_not_establish_secure_connection(
                            context,
                            contact_chat_id,
                            b"Fingerprint mismatch on inviter-side.\x00" as *const u8
                                as *const libc::c_char,
                        );
                        current_block = 4378276786830486580;
                    } else {
                        dc_scaleup_contact_origin(context, contact_id, 0x1000000i32);
                        dc_log_info(
                            context,
                            0i32,
                            b"Auth verified.\x00" as *const u8 as *const libc::c_char,
                        );
                        secure_connection_established(context, contact_chat_id);
                        (context.cb)(
                            context,
                            Event::CONTACTS_CHANGED,
                            contact_id as uintptr_t,
                            0i32 as uintptr_t,
                        );
                        (context.cb)(
                            context,
                            Event::SECUREJOIN_INVITER_PROGRESS,
                            contact_id as uintptr_t,
                            600i32 as uintptr_t,
                        );
                        if 0 != join_vg {
                            grpid = dc_strdup(lookup_field(
                                mimeparser,
                                b"Secure-Join-Group\x00" as *const u8 as *const libc::c_char,
                            ));
                            let group_chat_id: uint32_t = dc_get_chat_id_by_grpid(
                                context,
                                grpid,
                                0 as *mut libc::c_int,
                                0 as *mut libc::c_int,
                            );
                            if group_chat_id == 0i32 as libc::c_uint {
                                dc_log_error(
                                    context,
                                    0i32,
                                    b"Chat %s not found.\x00" as *const u8 as *const libc::c_char,
                                    grpid,
                                );
                                current_block = 4378276786830486580;
                            } else {
                                dc_add_contact_to_chat_ex(
                                    context,
                                    group_chat_id,
                                    contact_id,
                                    0x1i32,
                                );
                                current_block = 10256747982273457880;
                            }
                        } else {
                            send_handshake_msg(
                                context,
                                contact_chat_id,
                                b"vc-contact-confirm\x00" as *const u8 as *const libc::c_char,
                                0 as *const libc::c_char,
                                0 as *const libc::c_char,
                                0 as *const libc::c_char,
                            );
                            (context.cb)(
                                context,
                                Event::SECUREJOIN_INVITER_PROGRESS,
                                contact_id as uintptr_t,
                                1000i32 as uintptr_t,
                            );
                            current_block = 10256747982273457880;
                        }
                    }
                }
            } else if strcmp(
                step,
                b"vg-member-added\x00" as *const u8 as *const libc::c_char,
            ) == 0i32
                || strcmp(
                    step,
                    b"vc-contact-confirm\x00" as *const u8 as *const libc::c_char,
                ) == 0i32
            {
                if 0 != join_vg {
                    ret = 0x1i32
                }
                if context.bob.clone().read().unwrap().expects != 6 {
                    dc_log_info(
                        context,
                        0i32,
                        b"Message belongs to a different handshake.\x00" as *const u8
                            as *const libc::c_char,
                    );
                    current_block = 4378276786830486580;
                } else {
                    let scan = context.bob.clone().read().unwrap().qr_scan;
                    if scan.is_null() || 0 != join_vg && (*scan).state != 202i32 {
                        dc_log_warning(
                            context,
                            0i32,
                            b"Message out of sync or belongs to a different handshake.\x00"
                                as *const u8 as *const libc::c_char,
                        );
                        current_block = 4378276786830486580;
                    } else {
                        scanned_fingerprint_of_alice = dc_strdup((*scan).fingerprint);
                        if 0 != join_vg {
                            grpid = dc_strdup((*scan).text2)
                        }
                        let mut vg_expect_encrypted: libc::c_int = 1i32;
                        if 0 != join_vg {
                            let mut is_verified_group: libc::c_int = 0i32;
                            dc_get_chat_id_by_grpid(
                                context,
                                grpid,
                                0 as *mut libc::c_int,
                                &mut is_verified_group,
                            );
                            if 0 == is_verified_group {
                                vg_expect_encrypted = 0i32
                            }
                        }
                        if 0 != vg_expect_encrypted {
                            if 0 == encrypted_and_signed(mimeparser, scanned_fingerprint_of_alice) {
                                could_not_establish_secure_connection(
                                    context,
                                    contact_chat_id,
                                    b"Contact confirm message not encrypted.\x00" as *const u8
                                        as *const libc::c_char,
                                );
                                end_bobs_joining(context, 0i32);
                                current_block = 4378276786830486580;
                            } else {
                                current_block = 5195798230510548452;
                            }
                        } else {
                            current_block = 5195798230510548452;
                        }
                        match current_block {
                            4378276786830486580 => {}
                            _ => {
                                if 0 == mark_peer_as_verified(context, scanned_fingerprint_of_alice)
                                {
                                    could_not_establish_secure_connection(
                                        context,
                                        contact_chat_id,
                                        b"Fingerprint mismatch on joiner-side.\x00" as *const u8
                                            as *const libc::c_char,
                                    );
                                    current_block = 4378276786830486580;
                                } else {
                                    dc_scaleup_contact_origin(context, contact_id, 0x2000000i32);
                                    (context.cb)(
                                        context,
                                        Event::CONTACTS_CHANGED,
                                        0i32 as uintptr_t,
                                        0i32 as uintptr_t,
                                    );
                                    if 0 != join_vg {
                                        if 0 == dc_addr_equals_self(
                                            context,
                                            lookup_field(
                                                mimeparser,
                                                b"Chat-Group-Member-Added\x00" as *const u8
                                                    as *const libc::c_char,
                                            ),
                                        ) {
                                            dc_log_info(context, 0i32,
                                                        b"Message belongs to a different handshake (scaled up contact anyway to allow creation of group).\x00"
                                                            as *const u8 as
                                                            *const libc::c_char);
                                            current_block = 4378276786830486580;
                                        } else {
                                            current_block = 9180031981464905198;
                                        }
                                    } else {
                                        current_block = 9180031981464905198;
                                    }
                                    match current_block {
                                        4378276786830486580 => {}
                                        _ => {
                                            secure_connection_established(context, contact_chat_id);
                                            context.bob.clone().write().unwrap().expects = 0;
                                            if 0 != join_vg {
                                                send_handshake_msg(
                                                    context,
                                                    contact_chat_id,
                                                    b"vg-member-added-received\x00" as *const u8
                                                        as *const libc::c_char,
                                                    0 as *const libc::c_char,
                                                    0 as *const libc::c_char,
                                                    0 as *const libc::c_char,
                                                );
                                            }
                                            end_bobs_joining(context, 1i32);
                                            current_block = 10256747982273457880;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            } else if strcmp(
                step,
                b"vg-member-added-received\x00" as *const u8 as *const libc::c_char,
            ) == 0i32
            {
                /* ============================================================
                ====              Alice - the inviter side              ====
                ====  Step 8 in "Out-of-band verified groups" protocol  ====
                ============================================================ */
                contact = dc_get_contact(context, contact_id);
                if contact.is_null() || 0 == dc_contact_is_verified(contact) {
                    dc_log_warning(
                        context,
                        0i32,
                        b"vg-member-added-received invalid.\x00" as *const u8
                            as *const libc::c_char,
                    );
                    current_block = 4378276786830486580;
                } else {
                    (context.cb)(
                        context,
                        Event::SECUREJOIN_INVITER_PROGRESS,
                        contact_id as uintptr_t,
                        800i32 as uintptr_t,
                    );
                    (context.cb)(
                        context,
                        Event::SECUREJOIN_INVITER_PROGRESS,
                        contact_id as uintptr_t,
                        1000i32 as uintptr_t,
                    );
                    current_block = 10256747982273457880;
                }
            } else {
                current_block = 10256747982273457880;
            }
            match current_block {
                4378276786830486580 => {}
                _ => {
                    if 0 != ret & 0x2i32 {
                        ret |= 0x4i32
                    }
                }
            }
        }
    }
    dc_contact_unref(contact);
    free(scanned_fingerprint_of_alice as *mut libc::c_void);
    free(auth as *mut libc::c_void);
    free(own_fingerprint as *mut libc::c_void);
    free(grpid as *mut libc::c_void);

    ret
}

unsafe fn end_bobs_joining(context: &dc_context_t, status: libc::c_int) {
    context.bob.clone().write().unwrap().status = status;
    dc_stop_ongoing_process(context);
}

unsafe fn secure_connection_established(context: &dc_context_t, contact_chat_id: uint32_t) {
    let contact_id: uint32_t = chat_id_2_contact_id(context, contact_chat_id);
    let contact: *mut dc_contact_t = dc_get_contact(context, contact_id);
    let msg: *mut libc::c_char = dc_stock_str_repl_string(
        context,
        35i32,
        if !contact.is_null() {
            (*contact).addr
        } else {
            b"?\x00" as *const u8 as *const libc::c_char
        },
    );
    dc_add_device_msg(context, contact_chat_id, msg);
    (context.cb)(
        context,
        Event::CHAT_MODIFIED,
        contact_chat_id as uintptr_t,
        0i32 as uintptr_t,
    );
    free(msg as *mut libc::c_void);
    dc_contact_unref(contact);
}

unsafe fn lookup_field(
    mimeparser: &dc_mimeparser_t,
    key: *const libc::c_char,
) -> *const libc::c_char {
    let mut value: *const libc::c_char = 0 as *const libc::c_char;
    let field: *mut mailimf_field = dc_mimeparser_lookup_field(mimeparser, key);
    if field.is_null()
        || (*field).fld_type != MAILIMF_FIELD_OPTIONAL_FIELD as libc::c_int
        || (*field).fld_data.fld_optional_field.is_null()
        || {
            value = (*(*field).fld_data.fld_optional_field).fld_value;
            value.is_null()
        }
    {
        return 0 as *const libc::c_char;
    }

    value
}

unsafe fn could_not_establish_secure_connection(
    context: &dc_context_t,
    contact_chat_id: uint32_t,
    details: *const libc::c_char,
) {
    let contact_id: uint32_t = chat_id_2_contact_id(context, contact_chat_id);
    let contact = dc_get_contact(context, contact_id);
    let msg: *mut libc::c_char = dc_stock_str_repl_string(
        context,
        36i32,
        if !contact.is_null() {
            (*contact).addr
        } else {
            b"?\x00" as *const u8 as *const libc::c_char
        },
    );
    dc_add_device_msg(context, contact_chat_id, msg);
    dc_log_error(
        context,
        0i32,
        b"%s (%s)\x00" as *const u8 as *const libc::c_char,
        msg,
        details,
    );
    free(msg as *mut libc::c_void);
    dc_contact_unref(contact);
}

unsafe fn mark_peer_as_verified(
    context: &dc_context_t,
    fingerprint: *const libc::c_char,
) -> libc::c_int {
    let mut success = 0;

    if let Some(ref mut peerstate) = Peerstate::from_fingerprint(
        context,
        &context.sql.clone().read().unwrap(),
        to_str(fingerprint),
    ) {
        if peerstate.set_verified(1, to_str(fingerprint), 2) {
            peerstate.prefer_encrypt = EncryptPreference::Mutual;
            peerstate.to_save = Some(ToSave::All);
            peerstate.save_to_db(&context.sql.clone().read().unwrap(), false);
            success = 1;
        }
    }

    success
}

/* ******************************************************************************
 * Tools: Misc.
 ******************************************************************************/

// TODO should return bool
unsafe fn encrypted_and_signed(
    mimeparser: &dc_mimeparser_t,
    expected_fingerprint: *const libc::c_char,
) -> libc::c_int {
    if 0 == mimeparser.e2ee_helper.encrypted {
        dc_log_warning(
            mimeparser.context,
            0i32,
            b"Message not encrypted.\x00" as *const u8 as *const libc::c_char,
        );
        return 0i32;
    }
    if mimeparser.e2ee_helper.signatures.len() <= 0 {
        dc_log_warning(
            mimeparser.context,
            0i32,
            b"Message not signed.\x00" as *const u8 as *const libc::c_char,
        );
        return 0i32;
    }
    if expected_fingerprint.is_null() {
        dc_log_warning(
            mimeparser.context,
            0i32,
            b"Fingerprint for comparison missing.\x00" as *const u8 as *const libc::c_char,
        );
        return 0i32;
    }
    if !mimeparser
        .e2ee_helper
        .signatures
        .contains(to_str(expected_fingerprint))
    {
        dc_log_warning(
            mimeparser.context,
            0i32,
            b"Message does not match expected fingerprint %s.\x00" as *const u8
                as *const libc::c_char,
            expected_fingerprint,
        );
        return 0i32;
    }

    1
}

pub unsafe fn dc_handle_degrade_event(context: &dc_context_t, peerstate: &Peerstate) {
    let stmt;
    let contact_id: uint32_t;
    let mut contact_chat_id: uint32_t = 0i32 as uint32_t;

    // - we do not issue an warning for DC_DE_ENCRYPTION_PAUSED as this is quite normal
    // - currently, we do not issue an extra warning for DC_DE_VERIFICATION_LOST - this always comes
    //   together with DC_DE_FINGERPRINT_CHANGED which is logged, the idea is not to bother
    //   with things they cannot fix, so the user is just kicked from the verified group
    //   (and he will know this and can fix this)
    if Some(DegradeEvent::FingerprintChanged) == peerstate.degrade_event {
        stmt = dc_sqlite3_prepare(
            context,
            &context.sql.clone().read().unwrap(),
            b"SELECT id FROM contacts WHERE addr=?;\x00" as *const u8 as *const libc::c_char,
        );
        let c_addr = peerstate.addr.as_ref().map(to_cstring);
        sqlite3_bind_text(
            stmt,
            1i32,
            c_addr
                .as_ref()
                .map(|a| a.as_ptr())
                .unwrap_or_else(|| std::ptr::null()),
            -1i32,
            None,
        );
        sqlite3_step(stmt);
        contact_id = sqlite3_column_int(stmt, 0i32) as uint32_t;
        sqlite3_finalize(stmt);
        if !(contact_id == 0i32 as libc::c_uint) {
            dc_create_or_lookup_nchat_by_contact_id(
                context,
                contact_id,
                2i32,
                &mut contact_chat_id,
                0 as *mut libc::c_int,
            );
            let msg = dc_stock_str_repl_string(
                context,
                37i32,
                c_addr
                    .map(|a| a.as_ptr())
                    .unwrap_or_else(|| std::ptr::null()),
            );
            dc_add_device_msg(context, contact_chat_id, msg);
            free(msg as *mut libc::c_void);
            (context.cb)(
                context,
                Event::CHAT_MODIFIED,
                contact_chat_id as uintptr_t,
                0i32 as uintptr_t,
            );
        }
    }
}
