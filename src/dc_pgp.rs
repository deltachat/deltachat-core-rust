use crate::dc_context::dc_context_t;
use crate::dc_hash::*;
use crate::dc_key::*;
use crate::dc_keyring::*;
use crate::dc_log::*;
use crate::dc_tools::*;
use crate::pgp as rpgp;
use crate::types::*;
use crate::x::*;

pub unsafe fn dc_pgp_exit() {}

// TODO should return bool /rtn
pub unsafe fn dc_split_armored_data(
    buf: *mut libc::c_char,
    ret_headerline: *mut *const libc::c_char,
    ret_setupcodebegin: *mut *const libc::c_char,
    ret_preferencrypt: *mut *const libc::c_char,
    ret_base64: *mut *const libc::c_char,
) -> libc::c_int {
    let mut success: libc::c_int = 0i32;
    let mut line_chars: size_t = 0i32 as size_t;
    let mut line: *mut libc::c_char = buf;
    let mut p1: *mut libc::c_char = buf;
    let mut p2: *mut libc::c_char;
    let mut headerline: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut base64: *mut libc::c_char = 0 as *mut libc::c_char;
    if !ret_headerline.is_null() {
        *ret_headerline = 0 as *const libc::c_char
    }
    if !ret_setupcodebegin.is_null() {
        *ret_setupcodebegin = 0 as *const libc::c_char
    }
    if !ret_preferencrypt.is_null() {
        *ret_preferencrypt = 0 as *const libc::c_char
    }
    if !ret_base64.is_null() {
        *ret_base64 = 0 as *const libc::c_char
    }
    if !(buf.is_null() || ret_headerline.is_null()) {
        dc_remove_cr_chars(buf);
        while 0 != *p1 {
            if *p1 as libc::c_int == '\n' as i32 {
                *line.offset(line_chars as isize) = 0i32 as libc::c_char;
                if headerline.is_null() {
                    dc_trim(line);
                    if strncmp(
                        line,
                        b"-----BEGIN \x00" as *const u8 as *const libc::c_char,
                        1,
                    ) == 0i32
                        && strncmp(
                            &mut *line.offset(strlen(line).wrapping_sub(5) as isize),
                            b"-----\x00" as *const u8 as *const libc::c_char,
                            5,
                        ) == 0i32
                    {
                        headerline = line;
                        if !ret_headerline.is_null() {
                            *ret_headerline = headerline
                        }
                    }
                } else if strspn(line, b"\t\r\n \x00" as *const u8 as *const libc::c_char)
                    == strlen(line)
                {
                    base64 = p1.offset(1isize);
                    break;
                } else {
                    p2 = strchr(line, ':' as i32);
                    if p2.is_null() {
                        *line.offset(line_chars as isize) = '\n' as i32 as libc::c_char;
                        base64 = line;
                        break;
                    } else {
                        *p2 = 0i32 as libc::c_char;
                        dc_trim(line);
                        if strcasecmp(
                            line,
                            b"Passphrase-Begin\x00" as *const u8 as *const libc::c_char,
                        ) == 0i32
                        {
                            p2 = p2.offset(1isize);
                            dc_trim(p2);
                            if !ret_setupcodebegin.is_null() {
                                *ret_setupcodebegin = p2
                            }
                        } else if strcasecmp(
                            line,
                            b"Autocrypt-Prefer-Encrypt\x00" as *const u8 as *const libc::c_char,
                        ) == 0i32
                        {
                            p2 = p2.offset(1isize);
                            dc_trim(p2);
                            if !ret_preferencrypt.is_null() {
                                *ret_preferencrypt = p2
                            }
                        }
                    }
                }
                p1 = p1.offset(1isize);
                line = p1;
                line_chars = 0i32 as size_t
            } else {
                p1 = p1.offset(1isize);
                line_chars = line_chars.wrapping_add(1)
            }
        }
        if !(headerline.is_null() || base64.is_null()) {
            /* now, line points to beginning of base64 data, search end */
            /*the trailing space makes sure, this is not a normal base64 sequence*/
            p1 = strstr(base64, b"-----END \x00" as *const u8 as *const libc::c_char);
            if !(p1.is_null()
                || strncmp(
                    p1.offset(9isize),
                    headerline.offset(11isize),
                    strlen(headerline.offset(11isize)),
                ) != 0i32)
            {
                *p1 = 0i32 as libc::c_char;
                dc_trim(base64);
                if !ret_base64.is_null() {
                    *ret_base64 = base64
                }
                success = 1i32
            }
        }
    }

    success
}

/* public key encryption */
// TODO should return bool /rtn
pub unsafe fn dc_pgp_create_keypair(
    context: &dc_context_t,
    addr: *const libc::c_char,
    ret_public_key: *mut dc_key_t,
    ret_private_key: *mut dc_key_t,
) -> libc::c_int {
    let mut success: libc::c_int = 0i32;
    let skey: *mut rpgp::signed_secret_key;
    let mut pkey: *mut rpgp::signed_public_key = 0 as *mut rpgp::signed_public_key;
    let mut skey_bytes: *mut rpgp::cvec = 0 as *mut rpgp::cvec;
    let mut pkey_bytes: *mut rpgp::cvec = 0 as *mut rpgp::cvec;
    let user_id: *mut libc::c_char;
    user_id = dc_mprintf(b"<%s>\x00" as *const u8 as *const libc::c_char, addr);
    skey = rpgp::rpgp_create_rsa_skey(2048i32 as uint32_t, user_id);
    if !(0 != dc_pgp_handle_rpgp_error(context)) {
        skey_bytes = rpgp::rpgp_skey_to_bytes(skey);
        if !(0 != dc_pgp_handle_rpgp_error(context)) {
            pkey = rpgp::rpgp_skey_public_key(skey);
            if !(0 != dc_pgp_handle_rpgp_error(context)) {
                pkey_bytes = rpgp::rpgp_pkey_to_bytes(pkey);
                if !(0 != dc_pgp_handle_rpgp_error(context)) {
                    dc_key_set_from_binary(
                        ret_private_key,
                        rpgp::rpgp_cvec_data(skey_bytes) as *const libc::c_void,
                        rpgp::rpgp_cvec_len(skey_bytes) as libc::c_int,
                        1i32,
                    );
                    if !(0 != dc_pgp_handle_rpgp_error(context)) {
                        dc_key_set_from_binary(
                            ret_public_key,
                            rpgp::rpgp_cvec_data(pkey_bytes) as *const libc::c_void,
                            rpgp::rpgp_cvec_len(pkey_bytes) as libc::c_int,
                            0i32,
                        );
                        if !(0 != dc_pgp_handle_rpgp_error(context)) {
                            success = 1i32
                        }
                    }
                }
            }
        }
    }
    /* cleanup */
    if !skey.is_null() {
        rpgp::rpgp_skey_drop(skey);
    }
    if !skey_bytes.is_null() {
        rpgp::rpgp_cvec_drop(skey_bytes);
    }
    if !pkey.is_null() {
        rpgp::rpgp_pkey_drop(pkey);
    }
    if !pkey_bytes.is_null() {
        rpgp::rpgp_cvec_drop(pkey_bytes);
    }
    if !user_id.is_null() {
        free(user_id as *mut libc::c_void);
    }

    success
}

/* returns 0 if there is no error, otherwise logs the error if a context is provided and returns 1*/
// TODO should return bool /rtn
pub unsafe fn dc_pgp_handle_rpgp_error(context: &dc_context_t) -> libc::c_int {
    let mut success: libc::c_int = 0i32;
    let len: libc::c_int;
    let mut msg: *mut libc::c_char = 0 as *mut libc::c_char;
    len = rpgp::rpgp_last_error_length();
    if !(len == 0i32) {
        msg = rpgp::rpgp_last_error_message();
        dc_log_info(
            context,
            0i32,
            b"[rpgp][error] %s\x00" as *const u8 as *const libc::c_char,
            msg,
        );
        success = 1i32
    }
    if !msg.is_null() {
        rpgp::rpgp_string_drop(msg);
    }

    success
}

// TODO should return bool /rtn
pub unsafe fn dc_pgp_is_valid_key(context: &dc_context_t, raw_key: *const dc_key_t) -> libc::c_int {
    let mut key_is_valid: libc::c_int = 0i32;
    let mut key: *mut rpgp::public_or_secret_key = 0 as *mut rpgp::public_or_secret_key;
    if !(raw_key.is_null() || (*raw_key).binary.is_null() || (*raw_key).bytes <= 0i32) {
        key = rpgp::rpgp_key_from_bytes(
            (*raw_key).binary as *const uint8_t,
            (*raw_key).bytes as usize,
        );
        if !(0 != dc_pgp_handle_rpgp_error(context)) {
            if (*raw_key).type_0 == 0i32 && 0 != rpgp::rpgp_key_is_public(key) as libc::c_int {
                key_is_valid = 1i32
            } else if (*raw_key).type_0 == 1i32 && 0 != rpgp::rpgp_key_is_secret(key) as libc::c_int
            {
                key_is_valid = 1i32
            }
        }
    }
    if !key.is_null() {
        rpgp::rpgp_key_drop(key);
    }

    key_is_valid
}

// TODO should return bool /rtn
pub unsafe fn dc_pgp_calc_fingerprint(
    context: &dc_context_t,
    raw_key: *const dc_key_t,
    ret_fingerprint: *mut *mut uint8_t,
    ret_fingerprint_bytes: *mut size_t,
) -> libc::c_int {
    let mut success: libc::c_int = 0i32;
    let mut key: *mut rpgp::public_or_secret_key = 0 as *mut rpgp::public_or_secret_key;
    let mut fingerprint: *mut rpgp::cvec = 0 as *mut rpgp::cvec;
    if !(raw_key.is_null()
        || ret_fingerprint.is_null()
        || !(*ret_fingerprint).is_null()
        || ret_fingerprint_bytes.is_null()
        || *ret_fingerprint_bytes != 0
        || (*raw_key).binary.is_null()
        || (*raw_key).bytes <= 0i32)
    {
        key = rpgp::rpgp_key_from_bytes(
            (*raw_key).binary as *const uint8_t,
            (*raw_key).bytes as usize,
        );
        if !(0 != dc_pgp_handle_rpgp_error(context)) {
            fingerprint = rpgp::rpgp_key_fingerprint(key);
            if !(0 != dc_pgp_handle_rpgp_error(context)) {
                *ret_fingerprint_bytes = rpgp::rpgp_cvec_len(fingerprint) as size_t;
                *ret_fingerprint = malloc(*ret_fingerprint_bytes) as *mut uint8_t;
                memcpy(
                    *ret_fingerprint as *mut libc::c_void,
                    rpgp::rpgp_cvec_data(fingerprint) as *const libc::c_void,
                    *ret_fingerprint_bytes,
                );
                success = 1i32
            }
        }
    }
    if !key.is_null() {
        rpgp::rpgp_key_drop(key);
    }
    if !fingerprint.is_null() {
        rpgp::rpgp_cvec_drop(fingerprint);
    }

    success
}

// TODO should return bool /rtn
pub unsafe fn dc_pgp_split_key(context: &dc_context_t, private_in: *const dc_key_t) -> Option<Key> {
    let mut success: libc::c_int = 0i32;
    let mut key: *mut rpgp::signed_secret_key = 0 as *mut rpgp::signed_secret_key;
    let mut pub_key: *mut rpgp::signed_public_key = 0 as *mut rpgp::signed_public_key;
    let mut buf: *mut rpgp::cvec = 0 as *mut rpgp::cvec;
    if !(private_in.is_null() || ret_public_key.is_null()) {
        if (*private_in).type_0 != 1i32 {
            dc_log_warning(
                context,
                0i32,
                b"Split key: Given key is no private key.\x00" as *const u8 as *const libc::c_char,
            );
        } else {
            key = rpgp::rpgp_skey_from_bytes(
                (*private_in).binary as *const uint8_t,
                (*private_in).bytes as usize,
            );
            if !(0 != dc_pgp_handle_rpgp_error(context)) {
                pub_key = rpgp::rpgp_skey_public_key(key);
                if !(0 != dc_pgp_handle_rpgp_error(context)) {
                    buf = rpgp::rpgp_pkey_to_bytes(pub_key);
                    if !(0 != dc_pgp_handle_rpgp_error(context)) {
                        dc_key_set_from_binary(
                            ret_public_key,
                            rpgp::rpgp_cvec_data(buf) as *const libc::c_void,
                            rpgp::rpgp_cvec_len(buf) as libc::c_int,
                            0i32,
                        );
                        success = 1i32
                    }
                }
            }
        }
    }
    if !key.is_null() {
        rpgp::rpgp_skey_drop(key);
    }
    if !pub_key.is_null() {
        rpgp::rpgp_pkey_drop(pub_key);
    }
    if !buf.is_null() {
        rpgp::rpgp_cvec_drop(buf);
    }

    success
}

// TODO should return bool /rtn
pub unsafe fn dc_pgp_pk_encrypt(
    context: &dc_context_t,
    plain_text: *const libc::c_void,
    plain_bytes: size_t,
    raw_public_keys_for_encryption: *const dc_keyring_t,
    raw_private_key_for_signing: *const dc_key_t,
    use_armor: libc::c_int,
    ret_ctext: *mut *mut libc::c_void,
    ret_ctext_bytes: *mut size_t,
) -> libc::c_int {
    let mut current_block: u64;
    let mut i: libc::c_int;
    let mut success: libc::c_int = 0i32;
    let mut public_keys_len: libc::c_int = 0i32;
    let mut public_keys: *mut *mut rpgp::signed_public_key = 0 as *mut *mut rpgp::signed_public_key;
    let mut private_key: *mut rpgp::signed_secret_key = 0 as *mut rpgp::signed_secret_key;
    let mut encrypted: *mut rpgp::Message = 0 as *mut rpgp::Message;
    if !(plain_text == 0 as *mut libc::c_void
        || plain_bytes == 0
        || ret_ctext.is_null()
        || ret_ctext_bytes.is_null()
        || raw_public_keys_for_encryption.is_null()
        || (*raw_public_keys_for_encryption).count <= 0i32
        || use_armor == 0i32)
    {
        /* only support use_armor=1 */
        *ret_ctext = 0 as *mut libc::c_void;
        *ret_ctext_bytes = 0i32 as size_t;
        public_keys_len = (*raw_public_keys_for_encryption).count;
        public_keys = malloc(
            (::std::mem::size_of::<*mut rpgp::signed_public_key>())
                .wrapping_mul(public_keys_len as usize),
        ) as *mut *mut rpgp::signed_public_key;
        /* setup secret key for signing */
        if !raw_private_key_for_signing.is_null() {
            private_key = rpgp::rpgp_skey_from_bytes(
                (*raw_private_key_for_signing).binary as *const uint8_t,
                (*raw_private_key_for_signing).bytes as usize,
            );
            if private_key.is_null() || 0 != dc_pgp_handle_rpgp_error(context) {
                dc_log_warning(
                    context,
                    0i32,
                    b"No key for signing found.\x00" as *const u8 as *const libc::c_char,
                );
                current_block = 2132137392766895896;
            } else {
                current_block = 12800627514080957624;
            }
        } else {
            current_block = 12800627514080957624;
        }
        match current_block {
            2132137392766895896 => {}
            _ => {
                /* setup public keys for encryption */
                i = 0i32;
                loop {
                    if !(i < public_keys_len) {
                        current_block = 6057473163062296781;
                        break;
                    }
                    let ref mut fresh0 = *public_keys.offset(i as isize);
                    *fresh0 = rpgp::rpgp_pkey_from_bytes(
                        (**(*raw_public_keys_for_encryption).keys.offset(i as isize)).binary
                            as *const uint8_t,
                        (**(*raw_public_keys_for_encryption).keys.offset(i as isize)).bytes
                            as usize,
                    );
                    if 0 != dc_pgp_handle_rpgp_error(context) {
                        current_block = 2132137392766895896;
                        break;
                    }
                    i += 1
                }
                match current_block {
                    2132137392766895896 => {}
                    _ => {
                        /* sign & encrypt */
                        let op_clocks: libc::clock_t;
                        let start: libc::clock_t = clock();
                        if private_key.is_null() {
                            encrypted = rpgp::rpgp_encrypt_bytes_to_keys(
                                plain_text as *const uint8_t,
                                plain_bytes as usize,
                                public_keys as *const *const rpgp::signed_public_key,
                                public_keys_len as usize,
                            );
                            if 0 != dc_pgp_handle_rpgp_error(context) {
                                dc_log_warning(
                                    context,
                                    0i32,
                                    b"Encryption failed.\x00" as *const u8 as *const libc::c_char,
                                );
                                current_block = 2132137392766895896;
                            } else {
                                op_clocks = clock().wrapping_sub(start);
                                dc_log_info(
                                    context,
                                    0i32,
                                    b"Message encrypted in %.3f ms.\x00" as *const u8
                                        as *const libc::c_char,
                                    op_clocks as libc::c_double * 1000.0f64
                                        / 1000000i32 as libc::c_double,
                                );
                                current_block = 1538046216550696469;
                            }
                        } else {
                            encrypted = rpgp::rpgp_sign_encrypt_bytes_to_keys(
                                plain_text as *const uint8_t,
                                plain_bytes as usize,
                                public_keys as *const *const rpgp::signed_public_key,
                                public_keys_len as usize,
                                private_key,
                            );
                            if 0 != dc_pgp_handle_rpgp_error(context) {
                                dc_log_warning(
                                    context,
                                    0i32,
                                    b"Signing and encrypting failed.\x00" as *const u8
                                        as *const libc::c_char,
                                );
                                current_block = 2132137392766895896;
                            } else {
                                op_clocks = clock().wrapping_sub(start);
                                dc_log_info(
                                    context,
                                    0i32,
                                    b"Message signed and encrypted in %.3f ms.\x00" as *const u8
                                        as *const libc::c_char,
                                    op_clocks as libc::c_double * 1000.0f64
                                        / 1000000i32 as libc::c_double,
                                );
                                current_block = 1538046216550696469;
                            }
                        }
                        match current_block {
                            2132137392766895896 => {}
                            _ => {
                                /* convert message to armored bytes and return values */
                                let armored: *mut rpgp::cvec = rpgp::rpgp_msg_to_armored(encrypted);
                                if !(0 != dc_pgp_handle_rpgp_error(context)) {
                                    *ret_ctext = rpgp::rpgp_cvec_data(armored) as *mut libc::c_void;
                                    *ret_ctext_bytes = rpgp::rpgp_cvec_len(armored) as size_t;
                                    free(armored as *mut libc::c_void);
                                    success = 1i32
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    if !private_key.is_null() {
        rpgp::rpgp_skey_drop(private_key);
    }
    i = 0i32;
    while i < public_keys_len {
        rpgp::rpgp_pkey_drop(*public_keys.offset(i as isize));
        i += 1
    }
    if !encrypted.is_null() {
        rpgp::rpgp_msg_drop(encrypted);
    }

    success
}

// TODO should return bool /rtn
pub unsafe fn dc_pgp_pk_decrypt(
    context: &dc_context_t,
    ctext: *const libc::c_void,
    ctext_bytes: size_t,
    raw_private_keys_for_decryption: *const dc_keyring_t,
    raw_public_keys_for_validation: *const dc_keyring_t,
    use_armor: libc::c_int,
    ret_plain: *mut *mut libc::c_void,
    ret_plain_bytes: *mut size_t,
    ret_signature_fingerprints: *mut dc_hash_t,
) -> libc::c_int {
    let mut current_block: u64;
    let mut i: libc::c_int;
    let mut success: libc::c_int = 0i32;
    let mut encrypted: *mut rpgp::Message = 0 as *mut rpgp::Message;
    let mut decrypted: *mut rpgp::message_decrypt_result = 0 as *mut rpgp::message_decrypt_result;
    let mut private_keys_len: libc::c_int = 0i32;
    let mut public_keys_len: libc::c_int = 0i32;
    let mut private_keys: *mut *mut rpgp::signed_secret_key =
        0 as *mut *mut rpgp::signed_secret_key;
    let mut public_keys: *mut *mut rpgp::signed_public_key = 0 as *mut *mut rpgp::signed_public_key;
    if !(ctext == 0 as *mut libc::c_void
        || ctext_bytes == 0
        || ret_plain.is_null()
        || ret_plain_bytes.is_null()
        || raw_private_keys_for_decryption.is_null()
        || (*raw_private_keys_for_decryption).count <= 0i32
        || use_armor == 0i32)
    {
        /* only support use_armor=1 */
        *ret_plain = 0 as *mut libc::c_void;
        *ret_plain_bytes = 0i32 as size_t;
        private_keys_len = (*raw_private_keys_for_decryption).count;
        private_keys = malloc(
            (::std::mem::size_of::<*mut rpgp::signed_secret_key>())
                .wrapping_mul(private_keys_len as usize),
        ) as *mut *mut rpgp::signed_secret_key;
        if !raw_public_keys_for_validation.is_null() {
            public_keys_len = (*raw_public_keys_for_validation).count;
            public_keys = malloc(
                (::std::mem::size_of::<*mut rpgp::signed_public_key>())
                    .wrapping_mul(public_keys_len as usize),
            ) as *mut *mut rpgp::signed_public_key
        }
        /* setup secret keys for decryption */
        i = 0i32;
        loop {
            if !(i < (*raw_private_keys_for_decryption).count) {
                current_block = 15904375183555213903;
                break;
            }
            let ref mut fresh1 = *private_keys.offset(i as isize);
            *fresh1 = rpgp::rpgp_skey_from_bytes(
                (**(*raw_private_keys_for_decryption).keys.offset(i as isize)).binary
                    as *const uint8_t,
                (**(*raw_private_keys_for_decryption).keys.offset(i as isize)).bytes as usize,
            );
            if 0 != dc_pgp_handle_rpgp_error(context) {
                current_block = 11904635156640512504;
                break;
            }
            i += 1
        }
        match current_block {
            11904635156640512504 => {}
            _ => {
                /* setup public keys for validation */
                if !raw_public_keys_for_validation.is_null() {
                    i = 0i32;
                    loop {
                        if !(i < (*raw_public_keys_for_validation).count) {
                            current_block = 7172762164747879670;
                            break;
                        }
                        let ref mut fresh2 = *public_keys.offset(i as isize);
                        *fresh2 = rpgp::rpgp_pkey_from_bytes(
                            (**(*raw_public_keys_for_validation).keys.offset(i as isize)).binary
                                as *const uint8_t,
                            (**(*raw_public_keys_for_validation).keys.offset(i as isize)).bytes
                                as usize,
                        );
                        if 0 != dc_pgp_handle_rpgp_error(context) {
                            current_block = 11904635156640512504;
                            break;
                        }
                        i += 1
                    }
                } else {
                    current_block = 7172762164747879670;
                }
                match current_block {
                    11904635156640512504 => {}
                    _ => {
                        /* decrypt */
                        encrypted = rpgp::rpgp_msg_from_armor(
                            ctext as *const uint8_t,
                            ctext_bytes as usize,
                        );
                        if !(0 != dc_pgp_handle_rpgp_error(context)) {
                            decrypted = rpgp::rpgp_msg_decrypt_no_pw(
                                encrypted,
                                private_keys as *const *const rpgp::signed_secret_key,
                                private_keys_len as usize,
                                public_keys as *const *const rpgp::signed_public_key,
                                public_keys_len as usize,
                            );
                            if !(0 != dc_pgp_handle_rpgp_error(context)) {
                                let decrypted_bytes: *mut rpgp::cvec =
                                    rpgp::rpgp_msg_to_bytes((*decrypted).message_ptr);
                                if !(0 != dc_pgp_handle_rpgp_error(context)) {
                                    *ret_plain_bytes =
                                        rpgp::rpgp_cvec_len(decrypted_bytes) as size_t;
                                    *ret_plain =
                                        rpgp::rpgp_cvec_data(decrypted_bytes) as *mut libc::c_void;
                                    free(decrypted_bytes as *mut libc::c_void);
                                    if !ret_signature_fingerprints.is_null() {
                                        let mut j: uint32_t = 0i32 as uint32_t;
                                        let len: uint32_t = (*decrypted).valid_ids_len as uint32_t;
                                        while j < len {
                                            let fingerprint_hex: *mut libc::c_char =
                                                *(*decrypted).valid_ids_ptr.offset(j as isize);
                                            if !fingerprint_hex.is_null() {
                                                dc_hash_insert(
                                                    ret_signature_fingerprints,
                                                    fingerprint_hex as *const libc::c_void,
                                                    strlen(fingerprint_hex) as libc::c_int,
                                                    1i32 as *mut libc::c_void,
                                                );
                                                free(fingerprint_hex as *mut libc::c_void);
                                            }
                                            j = j.wrapping_add(1)
                                        }
                                    }
                                    success = 1i32
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    i = 0i32;
    while i < private_keys_len {
        rpgp::rpgp_skey_drop(*private_keys.offset(i as isize));
        i += 1
    }
    i = 0i32;
    while i < public_keys_len {
        rpgp::rpgp_pkey_drop(*public_keys.offset(i as isize));
        i += 1
    }
    if !encrypted.is_null() {
        rpgp::rpgp_msg_drop(encrypted);
    }
    if !decrypted.is_null() {
        rpgp::rpgp_message_decrypt_result_drop(decrypted);
    }

    success
}

/* symm. encryption */
// TODO should return bool /rtn
pub unsafe fn dc_pgp_symm_encrypt(
    context: &dc_context_t,
    passphrase: *const libc::c_char,
    plain: *const libc::c_void,
    plain_bytes: size_t,
    ret_ctext_armored: *mut *mut libc::c_char,
) -> libc::c_int {
    let mut success: libc::c_int = 0i32;
    let mut decrypted: *mut rpgp::Message = 0 as *mut rpgp::Message;
    if !(passphrase.is_null()
        || plain == 0 as *mut libc::c_void
        || plain_bytes == 0
        || ret_ctext_armored.is_null())
    {
        decrypted = rpgp::rpgp_encrypt_bytes_with_password(
            plain as *const uint8_t,
            plain_bytes as usize,
            passphrase,
        );
        if !(0 != dc_pgp_handle_rpgp_error(context)) {
            *ret_ctext_armored = rpgp::rpgp_msg_to_armored_str(decrypted);
            if !(0 != dc_pgp_handle_rpgp_error(context)) {
                success = 1i32
            }
        }
    }
    if !decrypted.is_null() {
        rpgp::rpgp_msg_drop(decrypted);
    }

    success
}

// TODO should return bool /rtn
pub unsafe fn dc_pgp_symm_decrypt(
    context: &dc_context_t,
    passphrase: *const libc::c_char,
    ctext: *const libc::c_void,
    ctext_bytes: size_t,
    ret_plain_text: *mut *mut libc::c_void,
    ret_plain_bytes: *mut size_t,
) -> libc::c_int {
    let decrypted_bytes: *mut rpgp::cvec;
    let mut success: libc::c_int = 0i32;
    let encrypted: *mut rpgp::Message;
    let mut decrypted: *mut rpgp::Message = 0 as *mut rpgp::Message;
    encrypted = rpgp::rpgp_msg_from_bytes(ctext as *const uint8_t, ctext_bytes as usize);
    if !(0 != dc_pgp_handle_rpgp_error(context)) {
        decrypted = rpgp::rpgp_msg_decrypt_with_password(encrypted, passphrase);
        if !(0 != dc_pgp_handle_rpgp_error(context)) {
            decrypted_bytes = rpgp::rpgp_msg_to_bytes(decrypted);
            if !(0 != dc_pgp_handle_rpgp_error(context)) {
                *ret_plain_text = rpgp::rpgp_cvec_data(decrypted_bytes) as *mut libc::c_void;
                *ret_plain_bytes = rpgp::rpgp_cvec_len(decrypted_bytes) as size_t;
                free(decrypted_bytes as *mut libc::c_void);
                success = 1i32
            }
        }
    }
    if !encrypted.is_null() {
        rpgp::rpgp_msg_drop(encrypted);
    }
    if !decrypted.is_null() {
        rpgp::rpgp_msg_drop(decrypted);
    }

    success
}
