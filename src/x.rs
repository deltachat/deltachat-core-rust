use libc;

use crate::dc_context::dc_context_t;
use crate::dc_key::dc_key_t;
use crate::dc_sqlite3::dc_sqlite3_t;
use crate::dc_strbuilder::dc_strbuilder_t;
use crate::types::*;

extern "C" {
    pub static mut _DefaultRuneLocale: _RuneLocale;

    pub fn calloc(_: libc::c_ulong, _: libc::c_ulong) -> *mut libc::c_void;
    pub fn free(_: *mut libc::c_void);
    pub fn exit(_: libc::c_int) -> !;
    pub fn strcspn(_: *const libc::c_char, _: *const libc::c_char) -> libc::c_ulong;
    pub fn strspn(_: *const libc::c_char, _: *const libc::c_char) -> libc::c_ulong;
    pub fn strcasecmp(_: *const libc::c_char, _: *const libc::c_char) -> libc::c_int;
    pub fn strlen(_: *const libc::c_char) -> libc::c_ulong;
    pub fn sqlite3_bind_blob(
        _: *mut sqlite3_stmt,
        _: libc::c_int,
        _: *const libc::c_void,
        n: libc::c_int,
        _: Option<unsafe extern "C" fn(_: *mut libc::c_void) -> ()>,
    ) -> libc::c_int;
    pub fn sqlite3_bind_int64(
        _: *mut sqlite3_stmt,
        _: libc::c_int,
        _: sqlite3_int64,
    ) -> libc::c_int;
    pub fn sqlite3_bind_text(
        _: *mut sqlite3_stmt,
        _: libc::c_int,
        _: *const libc::c_char,
        _: libc::c_int,
        _: Option<unsafe extern "C" fn(_: *mut libc::c_void) -> ()>,
    ) -> libc::c_int;
    pub fn sqlite3_step(_: *mut sqlite3_stmt) -> libc::c_int;
    pub fn sqlite3_column_int(_: *mut sqlite3_stmt, iCol: libc::c_int) -> libc::c_int;
    pub fn sqlite3_column_int64(_: *mut sqlite3_stmt, iCol: libc::c_int) -> sqlite3_int64;
    pub fn sqlite3_column_text(_: *mut sqlite3_stmt, iCol: libc::c_int) -> *const libc::c_uchar;
    pub fn sqlite3_column_type(_: *mut sqlite3_stmt, iCol: libc::c_int) -> libc::c_int;
    pub fn sqlite3_finalize(pStmt: *mut sqlite3_stmt) -> libc::c_int;
    pub fn malloc(_: libc::c_ulong) -> *mut libc::c_void;
    pub fn realloc(_: *mut libc::c_void, _: libc::c_ulong) -> *mut libc::c_void;
    pub fn qsort(
        __base: *mut libc::c_void,
        __nel: size_t,
        __width: size_t,
        __compar: Option<
            unsafe extern "C" fn(_: *const libc::c_void, _: *const libc::c_void) -> libc::c_int,
        >,
    );
    pub fn memcpy(
        _: *mut libc::c_void,
        _: *const libc::c_void,
        _: libc::c_ulong,
    ) -> *mut libc::c_void;
    pub fn strcat(_: *mut libc::c_char, _: *const libc::c_char) -> *mut libc::c_char;
    pub fn strcmp(_: *const libc::c_char, _: *const libc::c_char) -> libc::c_int;
    pub fn sprintf(_: *mut libc::c_char, _: *const libc::c_char, _: ...) -> libc::c_int;
    pub fn time(_: *mut time_t) -> time_t;
    pub fn sqlite3_bind_int(_: *mut sqlite3_stmt, _: libc::c_int, _: libc::c_int) -> libc::c_int;
    pub fn strchr(_: *const libc::c_char, _: libc::c_int) -> *mut libc::c_char;
    pub fn strstr(_: *const libc::c_char, _: *const libc::c_char) -> *mut libc::c_char;
    pub fn strncasecmp(
        _: *const libc::c_char,
        _: *const libc::c_char,
        _: libc::c_ulong,
    ) -> libc::c_int;
    pub fn usleep(_: libc::useconds_t) -> libc::c_int;
    pub fn pow(_: libc::c_double, _: libc::c_double) -> libc::c_double;
    pub fn rand() -> libc::c_int;
    pub fn memset(_: *mut libc::c_void, _: libc::c_int, _: libc::c_ulong) -> *mut libc::c_void;
    pub fn clock() -> libc::clock_t;
    pub fn pthread_cond_signal(_: *mut pthread_cond_t) -> libc::c_int;
    pub fn pthread_cond_timedwait(
        _: *mut pthread_cond_t,
        _: *mut pthread_mutex_t,
        _: *const timespec,
    ) -> libc::c_int;
    pub fn pthread_mutex_lock(_: *mut pthread_mutex_t) -> libc::c_int;
    pub fn pthread_mutex_unlock(_: *mut pthread_mutex_t) -> libc::c_int;
    pub fn clist_free(_: *mut clist);
    pub fn clist_insert_after(
        _: *mut clist,
        _: *mut clistiter,
        _: *mut libc::c_void,
    ) -> libc::c_int;
    pub fn closedir(_: *mut DIR) -> libc::c_int;
    pub fn opendir(_: *const libc::c_char) -> *mut DIR;
    pub fn readdir(_: *mut DIR) -> *mut dirent;
    pub fn sleep(_: libc::c_uint) -> libc::c_uint;
    pub fn RAND_bytes(buf: *mut libc::c_uchar, num: libc::c_int) -> libc::c_int;
    pub fn RAND_pseudo_bytes(buf: *mut libc::c_uchar, num: libc::c_int) -> libc::c_int;
    pub fn mmap_string_unref(str: *mut libc::c_char) -> libc::c_int;
    pub fn strncmp(_: *const libc::c_char, _: *const libc::c_char, _: libc::c_ulong)
        -> libc::c_int;
    pub fn strncpy(
        _: *mut libc::c_char,
        _: *const libc::c_char,
        _: libc::c_ulong,
    ) -> *mut libc::c_char;
    pub fn strndup(_: *const libc::c_char, _: libc::c_ulong) -> *mut libc::c_char;
    pub fn localtime(_: *const time_t) -> *mut tm;
    pub fn strftime(
        _: *mut libc::c_char,
        _: size_t,
        _: *const libc::c_char,
        _: *const tm,
    ) -> size_t;
    pub fn mailmime_base64_body_parse(
        message: *const libc::c_char,
        length: size_t,
        indx: *mut size_t,
        result: *mut *mut libc::c_char,
        result_len: *mut size_t,
    ) -> libc::c_int;
    pub fn sqlite3_column_blob(_: *mut sqlite3_stmt, iCol: libc::c_int) -> *const libc::c_void;
    pub fn sqlite3_column_bytes(_: *mut sqlite3_stmt, iCol: libc::c_int) -> libc::c_int;
    pub fn sqlite3_reset(pStmt: *mut sqlite3_stmt) -> libc::c_int;
    pub fn sqlite3_mprintf(_: *const libc::c_char, _: ...) -> *mut libc::c_char;
    pub fn sqlite3_free(_: *mut libc::c_void);
    pub fn __toupper(_: __darwin_ct_rune_t) -> __darwin_ct_rune_t;
    pub fn memcmp(_: *const libc::c_void, _: *const libc::c_void, _: libc::c_ulong) -> libc::c_int;
    pub fn encode_base64(in_0: *const libc::c_char, len: libc::c_int) -> *mut libc::c_char;
    pub fn mmap_string_new(init: *const libc::c_char) -> *mut MMAPString;
    pub fn mmap_string_free(string: *mut MMAPString);
    pub fn clist_new() -> *mut clist;
    pub fn mailimf_address_new(
        ad_type: libc::c_int,
        ad_mailbox: *mut mailimf_mailbox,
        ad_group: *mut mailimf_group,
    ) -> *mut mailimf_address;
    pub fn mailimf_mailbox_new(
        mb_display_name: *mut libc::c_char,
        mb_addr_spec: *mut libc::c_char,
    ) -> *mut mailimf_mailbox;
    pub fn mailimf_field_new(
        fld_type: libc::c_int,
        fld_return_path: *mut mailimf_return,
        fld_resent_date: *mut mailimf_orig_date,
        fld_resent_from: *mut mailimf_from,
        fld_resent_sender: *mut mailimf_sender,
        fld_resent_to: *mut mailimf_to,
        fld_resent_cc: *mut mailimf_cc,
        fld_resent_bcc: *mut mailimf_bcc,
        fld_resent_msg_id: *mut mailimf_message_id,
        fld_orig_date: *mut mailimf_orig_date,
        fld_from: *mut mailimf_from,
        fld_sender: *mut mailimf_sender,
        fld_reply_to: *mut mailimf_reply_to,
        fld_to: *mut mailimf_to,
        fld_cc: *mut mailimf_cc,
        fld_bcc: *mut mailimf_bcc,
        fld_message_id: *mut mailimf_message_id,
        fld_in_reply_to: *mut mailimf_in_reply_to,
        fld_references: *mut mailimf_references,
        fld_subject: *mut mailimf_subject,
        fld_comments: *mut mailimf_comments,
        fld_keywords: *mut mailimf_keywords,
        fld_optional_field: *mut mailimf_optional_field,
    ) -> *mut mailimf_field;
    pub fn mailimf_subject_new(sbj_value: *mut libc::c_char) -> *mut mailimf_subject;
    pub fn mailimf_mailbox_list_new_empty() -> *mut mailimf_mailbox_list;
    pub fn mailimf_mailbox_list_add(
        mailbox_list: *mut mailimf_mailbox_list,
        mb: *mut mailimf_mailbox,
    ) -> libc::c_int;
    pub fn mailimf_address_list_new_empty() -> *mut mailimf_address_list;
    pub fn mailimf_address_list_add(
        address_list: *mut mailimf_address_list,
        addr: *mut mailimf_address,
    ) -> libc::c_int;
    pub fn mailimf_fields_add(
        fields: *mut mailimf_fields,
        field: *mut mailimf_field,
    ) -> libc::c_int;
    pub fn mailimf_fields_new_with_data_all(
        date: *mut mailimf_date_time,
        from: *mut mailimf_mailbox_list,
        sender: *mut mailimf_mailbox,
        reply_to: *mut mailimf_address_list,
        to: *mut mailimf_address_list,
        cc: *mut mailimf_address_list,
        bcc: *mut mailimf_address_list,
        message_id: *mut libc::c_char,
        in_reply_to: *mut clist,
        references: *mut clist,
        subject: *mut libc::c_char,
    ) -> *mut mailimf_fields;
    pub fn mailimf_get_date(time_0: time_t) -> *mut mailimf_date_time;
    pub fn mailimf_field_new_custom(
        name: *mut libc::c_char,
        value: *mut libc::c_char,
    ) -> *mut mailimf_field;
    pub fn mailmime_parameter_new(
        pa_name: *mut libc::c_char,
        pa_value: *mut libc::c_char,
    ) -> *mut mailmime_parameter;
    pub fn mailmime_free(mime: *mut mailmime);
    pub fn mailmime_disposition_parm_new(
        pa_type: libc::c_int,
        pa_filename: *mut libc::c_char,
        pa_creation_date: *mut libc::c_char,
        pa_modification_date: *mut libc::c_char,
        pa_read_date: *mut libc::c_char,
        pa_size: size_t,
        pa_parameter: *mut mailmime_parameter,
    ) -> *mut mailmime_disposition_parm;
    pub fn mailmime_new_message_data(msg_mime: *mut mailmime) -> *mut mailmime;
    pub fn mailmime_new_empty(
        content: *mut mailmime_content,
        mime_fields: *mut mailmime_fields,
    ) -> *mut mailmime;
    pub fn mailmime_set_body_file(
        build_info: *mut mailmime,
        filename: *mut libc::c_char,
    ) -> libc::c_int;
    pub fn mailmime_set_body_text(
        build_info: *mut mailmime,
        data_str: *mut libc::c_char,
        length: size_t,
    ) -> libc::c_int;
    pub fn mailmime_add_part(build_info: *mut mailmime, part: *mut mailmime) -> libc::c_int;
    pub fn mailmime_set_imf_fields(build_info: *mut mailmime, fields: *mut mailimf_fields);
    pub fn mailmime_smart_add_part(mime: *mut mailmime, mime_sub: *mut mailmime) -> libc::c_int;
    pub fn mailmime_content_new_with_str(str: *const libc::c_char) -> *mut mailmime_content;
    pub fn mailmime_fields_new_encoding(type_0: libc::c_int) -> *mut mailmime_fields;
    pub fn mailmime_multiple_new(type_0: *const libc::c_char) -> *mut mailmime;
    pub fn mailmime_fields_new_filename(
        dsp_type: libc::c_int,
        filename: *mut libc::c_char,
        encoding_type: libc::c_int,
    ) -> *mut mailmime_fields;
    pub fn mailmime_param_new_with_data(
        name: *mut libc::c_char,
        value: *mut libc::c_char,
    ) -> *mut mailmime_parameter;
    pub fn mailmime_write_mem(
        f: *mut MMAPString,
        col: *mut libc::c_int,
        build_info: *mut mailmime,
    ) -> libc::c_int;
    pub fn pthread_cond_destroy(_: *mut pthread_cond_t) -> libc::c_int;
    pub fn pthread_cond_init(_: *mut pthread_cond_t, _: *const pthread_condattr_t) -> libc::c_int;
    pub fn pthread_cond_wait(_: *mut pthread_cond_t, _: *mut pthread_mutex_t) -> libc::c_int;
    pub fn pthread_mutex_destroy(_: *mut pthread_mutex_t) -> libc::c_int;
    pub fn pthread_mutex_init(
        _: *mut pthread_mutex_t,
        _: *const pthread_mutexattr_t,
    ) -> libc::c_int;
    pub fn atoi(_: *const libc::c_char) -> libc::c_int;
    pub fn strdup(_: *const libc::c_char) -> *mut libc::c_char;
    pub fn gmtime(_: *const time_t) -> *mut tm;
    pub fn sqlite3_bind_double(
        _: *mut sqlite3_stmt,
        _: libc::c_int,
        _: libc::c_double,
    ) -> libc::c_int;
    pub fn sqlite3_column_double(_: *mut sqlite3_stmt, iCol: libc::c_int) -> libc::c_double;
    pub fn pthread_self() -> pthread_t;
    pub fn clist_delete(_: *mut clist, _: *mut clistiter) -> *mut clistiter;
    pub fn mailimf_fields_free(fields: *mut mailimf_fields);
    pub fn mailimf_fields_new_empty() -> *mut mailimf_fields;
    pub fn mailimf_envelope_and_optional_fields_parse(
        message: *const libc::c_char,
        length: size_t,
        indx: *mut size_t,
        result: *mut *mut mailimf_fields,
    ) -> libc::c_int;
    pub fn mailmime_content_free(content: *mut mailmime_content);
    pub fn mailmime_mechanism_new(
        enc_type: libc::c_int,
        enc_token: *mut libc::c_char,
    ) -> *mut mailmime_mechanism;
    pub fn mailmime_mechanism_free(mechanism: *mut mailmime_mechanism);
    pub fn mailmime_fields_free(fields: *mut mailmime_fields);
    pub fn mailmime_new(
        mm_type: libc::c_int,
        mm_mime_start: *const libc::c_char,
        mm_length: size_t,
        mm_mime_fields: *mut mailmime_fields,
        mm_content_type: *mut mailmime_content,
        mm_body: *mut mailmime_data,
        mm_preamble: *mut mailmime_data,
        mm_epilogue: *mut mailmime_data,
        mm_mp_list: *mut clist,
        mm_fields: *mut mailimf_fields,
        mm_msg_mime: *mut mailmime,
    ) -> *mut mailmime;
    pub fn mailmime_fields_new_empty() -> *mut mailmime_fields;
    pub fn mailmime_fields_new_with_data(
        encoding: *mut mailmime_mechanism,
        id: *mut libc::c_char,
        description: *mut libc::c_char,
        disposition: *mut mailmime_disposition,
        language: *mut mailmime_language,
    ) -> *mut mailmime_fields;
    pub fn mailmime_get_content_message() -> *mut mailmime_content;
    pub fn mailmime_parse(
        message: *const libc::c_char,
        length: size_t,
        indx: *mut size_t,
        result: *mut *mut mailmime,
    ) -> libc::c_int;
    pub fn mailmime_part_parse(
        message: *const libc::c_char,
        length: size_t,
        indx: *mut size_t,
        encoding: libc::c_int,
        result: *mut *mut libc::c_char,
        result_len: *mut size_t,
    ) -> libc::c_int;
    pub fn mailmime_substitute(old_mime: *mut mailmime, new_mime: *mut mailmime) -> libc::c_int;
    pub fn mailprivacy_prepare_mime(mime: *mut mailmime);
    pub fn mailmime_find_mailimf_fields(_: *mut mailmime) -> *mut mailimf_fields;
    pub fn mailimf_find_first_addr(_: *const mailimf_mailbox_list) -> *mut libc::c_char;
    pub fn mailimf_find_field(
        _: *mut mailimf_fields,
        wanted_fld_type: libc::c_int,
    ) -> *mut mailimf_field;
    pub fn mailimf_get_recipients(_: *mut mailimf_fields) -> *mut dc_hash_t;
    pub fn atol(_: *const libc::c_char) -> libc::c_long;
    pub fn rpgp_create_rsa_skey(
        bits: uint32_t,
        user_id: *const libc::c_char,
    ) -> *mut rpgp_signed_secret_key;
    pub fn rpgp_cvec_data(cvec_ptr: *mut rpgp_cvec) -> *const uint8_t;
    pub fn rpgp_cvec_drop(cvec_ptr: *mut rpgp_cvec);
    pub fn rpgp_cvec_len(cvec_ptr: *mut rpgp_cvec) -> size_t;
    pub fn rpgp_encrypt_bytes_to_keys(
        bytes_ptr: *const uint8_t,
        bytes_len: size_t,
        pkeys_ptr: *const *const rpgp_signed_public_key,
        pkeys_len: size_t,
    ) -> *mut rpgp_message;
    pub fn rpgp_encrypt_bytes_with_password(
        bytes_ptr: *const uint8_t,
        bytes_len: size_t,
        password_ptr: *const libc::c_char,
    ) -> *mut rpgp_message;
    pub fn rpgp_key_drop(key_ptr: *mut rpgp_public_or_secret_key);
    pub fn rpgp_key_fingerprint(key_ptr: *mut rpgp_public_or_secret_key) -> *mut rpgp_cvec;
    pub fn rpgp_key_from_bytes(raw: *const uint8_t, len: size_t) -> *mut rpgp_public_or_secret_key;
    pub fn rpgp_key_is_public(key_ptr: *mut rpgp_public_or_secret_key) -> bool;
    pub fn rpgp_key_is_secret(key_ptr: *mut rpgp_public_or_secret_key) -> bool;
    pub fn rpgp_last_error_length() -> libc::c_int;
    pub fn rpgp_last_error_message() -> *mut libc::c_char;
    pub fn rpgp_message_decrypt_result_drop(res_ptr: *mut rpgp_message_decrypt_result);
    pub fn rpgp_msg_decrypt_no_pw(
        msg_ptr: *const rpgp_message,
        skeys_ptr: *const *const rpgp_signed_secret_key,
        skeys_len: size_t,
        pkeys_ptr: *const *const rpgp_signed_public_key,
        pkeys_len: size_t,
    ) -> *mut rpgp_message_decrypt_result;
    pub fn rpgp_msg_decrypt_with_password(
        msg_ptr: *const rpgp_message,
        password_ptr: *const libc::c_char,
    ) -> *mut rpgp_message;
    pub fn rpgp_msg_drop(msg_ptr: *mut rpgp_message);
    pub fn rpgp_msg_from_armor(msg_ptr: *const uint8_t, msg_len: size_t) -> *mut rpgp_message;
    pub fn rpgp_msg_from_bytes(msg_ptr: *const uint8_t, msg_len: size_t) -> *mut rpgp_message;
    pub fn rpgp_msg_to_armored(msg_ptr: *const rpgp_message) -> *mut rpgp_cvec;
    pub fn rpgp_msg_to_armored_str(msg_ptr: *const rpgp_message) -> *mut libc::c_char;
    pub fn rpgp_msg_to_bytes(msg_ptr: *const rpgp_message) -> *mut rpgp_cvec;
    pub fn rpgp_pkey_drop(pkey_ptr: *mut rpgp_signed_public_key);
    pub fn rpgp_pkey_from_bytes(raw: *const uint8_t, len: size_t) -> *mut rpgp_signed_public_key;
    pub fn rpgp_pkey_to_bytes(pkey_ptr: *mut rpgp_signed_public_key) -> *mut rpgp_cvec;
    pub fn rpgp_sign_encrypt_bytes_to_keys(
        bytes_ptr: *const uint8_t,
        bytes_len: size_t,
        pkeys_ptr: *const *const rpgp_signed_public_key,
        pkeys_len: size_t,
        skey_ptr: *const rpgp_signed_secret_key,
    ) -> *mut rpgp_message;
    pub fn rpgp_skey_drop(skey_ptr: *mut rpgp_signed_secret_key);
    pub fn rpgp_skey_from_bytes(raw: *const uint8_t, len: size_t) -> *mut rpgp_signed_secret_key;
    pub fn rpgp_skey_public_key(
        skey_ptr: *mut rpgp_signed_secret_key,
    ) -> *mut rpgp_signed_public_key;
    pub fn rpgp_skey_to_bytes(skey_ptr: *mut rpgp_signed_secret_key) -> *mut rpgp_cvec;
    pub fn rpgp_string_drop(p: *mut libc::c_char);
    pub fn __maskrune(_: __darwin_ct_rune_t, _: libc::c_ulong) -> libc::c_int;
    pub fn __tolower(_: __darwin_ct_rune_t) -> __darwin_ct_rune_t;
    pub fn mmap_string_append(string: *mut MMAPString, val: *const libc::c_char)
        -> *mut MMAPString;
    pub fn mmap_string_append_len(
        string: *mut MMAPString,
        val: *const libc::c_char,
        len: size_t,
    ) -> *mut MMAPString;
    pub fn mmap_string_append_c(string: *mut MMAPString, c: libc::c_char) -> *mut MMAPString;
    pub fn snprintf(
        _: *mut libc::c_char,
        _: libc::c_ulong,
        _: *const libc::c_char,
        _: ...
    ) -> libc::c_int;
    pub fn mailmime_encoded_phrase_parse(
        default_fromcode: *const libc::c_char,
        message: *const libc::c_char,
        length: size_t,
        indx: *mut size_t,
        tocode: *const libc::c_char,
        result: *mut *mut libc::c_char,
    ) -> libc::c_int;
    pub fn charconv(
        tocode: *const libc::c_char,
        fromcode: *const libc::c_char,
        str: *const libc::c_char,
        length: size_t,
        result: *mut *mut libc::c_char,
    ) -> libc::c_int;
    pub fn strcpy(_: *mut libc::c_char, _: *const libc::c_char) -> *mut libc::c_char;
    pub fn open(_: *const libc::c_char, _: libc::c_int, _: ...) -> libc::c_int;
    pub fn close(_: libc::c_int) -> libc::c_int;
    pub fn read(_: libc::c_int, _: *mut libc::c_void, _: size_t) -> ssize_t;
    pub fn write(__fd: libc::c_int, __buf: *const libc::c_void, __nbyte: size_t) -> ssize_t;
    pub fn mkdir(_: *const libc::c_char, _: mode_t) -> libc::c_int;
    pub fn stat(_: *const libc::c_char, _: *mut stat) -> libc::c_int;
    pub fn atof(_: *const libc::c_char) -> libc::c_double;
    pub fn gmtime_r(_: *const time_t, _: *mut tm) -> *mut tm;
    pub fn localtime_r(_: *const time_t, _: *mut tm) -> *mut tm;
    pub fn carray_new(initsize: libc::c_uint) -> *mut carray;
    pub fn carray_add(
        array: *mut carray,
        data: *mut libc::c_void,
        indx: *mut libc::c_uint,
    ) -> libc::c_int;
    pub fn carray_free(array: *mut carray);
    pub fn fclose(_: *mut FILE) -> libc::c_int;
    pub fn fopen(_: *const libc::c_char, _: *const libc::c_char) -> *mut FILE;
    pub fn fread(
        _: *mut libc::c_void,
        _: libc::c_ulong,
        _: libc::c_ulong,
        _: *mut FILE,
    ) -> libc::c_ulong;
    pub fn fseek(_: *mut FILE, _: libc::c_long, _: libc::c_int) -> libc::c_int;
    pub fn ftell(_: *mut FILE) -> libc::c_long;
    pub fn fwrite(
        _: *const libc::c_void,
        _: libc::c_ulong,
        _: libc::c_ulong,
        _: *mut FILE,
    ) -> libc::c_ulong;
    pub fn remove(_: *const libc::c_char) -> libc::c_int;
    pub fn vsnprintf(
        _: *mut libc::c_char,
        _: libc::c_ulong,
        _: *const libc::c_char,
        _: ::std::ffi::VaList,
    ) -> libc::c_int;
    pub fn mailimap_date_time_new(
        dt_day: libc::c_int,
        dt_month: libc::c_int,
        dt_year: libc::c_int,
        dt_hour: libc::c_int,
        dt_min: libc::c_int,
        dt_sec: libc::c_int,
        dt_zone: libc::c_int,
    ) -> *mut mailimap_date_time;
    pub fn memmove(
        _: *mut libc::c_void,
        _: *const libc::c_void,
        _: libc::c_ulong,
    ) -> *mut libc::c_void;
    pub fn strrchr(_: *const libc::c_char, _: libc::c_int) -> *mut libc::c_char;
    pub fn mailimap_xlist(
        session: *mut mailimap,
        mb: *const libc::c_char,
        list_mb: *const libc::c_char,
        result: *mut *mut clist,
    ) -> libc::c_int;
    pub fn mailimap_create(session: *mut mailimap, mb: *const libc::c_char) -> libc::c_int;
    pub fn mailimap_list(
        session: *mut mailimap,
        mb: *const libc::c_char,
        list_mb: *const libc::c_char,
        result: *mut *mut clist,
    ) -> libc::c_int;
    pub fn mailimap_list_result_free(list: *mut clist);
    pub fn mailimap_subscribe(session: *mut mailimap, mb: *const libc::c_char) -> libc::c_int;
    pub fn mailstream_close(s: *mut mailstream) -> libc::c_int;
    pub fn mailstream_wait_idle(s: *mut mailstream, max_idle_delay: libc::c_int) -> libc::c_int;
    pub fn mailstream_setup_idle(s: *mut mailstream) -> libc::c_int;
    pub fn mailstream_unsetup_idle(s: *mut mailstream);
    pub fn mailstream_interrupt_idle(s: *mut mailstream);
    pub fn mailimap_section_new(sec_spec: *mut mailimap_section_spec) -> *mut mailimap_section;
    pub fn mailimap_set_free(set: *mut mailimap_set);
    pub fn mailimap_fetch_type_free(fetch_type: *mut mailimap_fetch_type);
    pub fn mailimap_store_att_flags_free(store_att_flags: *mut mailimap_store_att_flags);
    pub fn mailimap_set_new_interval(first: uint32_t, last: uint32_t) -> *mut mailimap_set;
    pub fn mailimap_set_new_single(indx: uint32_t) -> *mut mailimap_set;
    pub fn mailimap_fetch_att_new_envelope() -> *mut mailimap_fetch_att;
    pub fn mailimap_fetch_att_new_flags() -> *mut mailimap_fetch_att;
    pub fn mailimap_fetch_att_new_uid() -> *mut mailimap_fetch_att;
    pub fn mailimap_fetch_att_new_body_peek_section(
        section: *mut mailimap_section,
    ) -> *mut mailimap_fetch_att;
    pub fn mailimap_fetch_type_new_fetch_att_list_empty() -> *mut mailimap_fetch_type;
    pub fn mailimap_fetch_type_new_fetch_att_list_add(
        fetch_type: *mut mailimap_fetch_type,
        fetch_att: *mut mailimap_fetch_att,
    ) -> libc::c_int;
    pub fn mailimap_store_att_flags_new_add_flags(
        flags: *mut mailimap_flag_list,
    ) -> *mut mailimap_store_att_flags;
    pub fn mailimap_flag_list_new_empty() -> *mut mailimap_flag_list;
    pub fn mailimap_flag_list_add(
        flag_list: *mut mailimap_flag_list,
        f: *mut mailimap_flag,
    ) -> libc::c_int;
    pub fn mailimap_flag_new_deleted() -> *mut mailimap_flag;
    pub fn mailimap_flag_new_seen() -> *mut mailimap_flag;
    pub fn mailimap_flag_new_flag_keyword(flag_keyword: *mut libc::c_char) -> *mut mailimap_flag;
    pub fn mailimap_socket_connect(
        f: *mut mailimap,
        server: *const libc::c_char,
        port: uint16_t,
    ) -> libc::c_int;
    pub fn mailimap_socket_starttls(f: *mut mailimap) -> libc::c_int;
    pub fn mailimap_ssl_connect(
        f: *mut mailimap,
        server: *const libc::c_char,
        port: uint16_t,
    ) -> libc::c_int;
    pub fn mailimap_uidplus_uid_copy(
        session: *mut mailimap,
        set: *mut mailimap_set,
        mb: *const libc::c_char,
        uidvalidity_result: *mut uint32_t,
        source_result: *mut *mut mailimap_set,
        dest_result: *mut *mut mailimap_set,
    ) -> libc::c_int;
    pub fn mailimap_uidplus_uid_move(
        session: *mut mailimap,
        set: *mut mailimap_set,
        mb: *const libc::c_char,
        uidvalidity_result: *mut uint32_t,
        source_result: *mut *mut mailimap_set,
        dest_result: *mut *mut mailimap_set,
    ) -> libc::c_int;
    pub fn mailimap_idle(session: *mut mailimap) -> libc::c_int;
    pub fn mailimap_idle_done(session: *mut mailimap) -> libc::c_int;
    pub fn mailimap_has_idle(session: *mut mailimap) -> libc::c_int;
    pub fn mailimap_has_xlist(session: *mut mailimap) -> libc::c_int;
    pub fn mailimap_oauth2_authenticate(
        session: *mut mailimap,
        auth_user: *const libc::c_char,
        access_token: *const libc::c_char,
    ) -> libc::c_int;
    pub fn mailimap_close(session: *mut mailimap) -> libc::c_int;
    pub fn mailimap_fetch(
        session: *mut mailimap,
        set: *mut mailimap_set,
        fetch_type: *mut mailimap_fetch_type,
        result: *mut *mut clist,
    ) -> libc::c_int;
    pub fn mailimap_uid_fetch(
        session: *mut mailimap,
        set: *mut mailimap_set,
        fetch_type: *mut mailimap_fetch_type,
        result: *mut *mut clist,
    ) -> libc::c_int;
    pub fn mailimap_fetch_list_free(fetch_list: *mut clist);
    pub fn mailimap_login(
        session: *mut mailimap,
        userid: *const libc::c_char,
        password: *const libc::c_char,
    ) -> libc::c_int;
    pub fn mailimap_select(session: *mut mailimap, mb: *const libc::c_char) -> libc::c_int;
    pub fn mailimap_uid_store(
        session: *mut mailimap,
        set: *mut mailimap_set,
        store_att_flags: *mut mailimap_store_att_flags,
    ) -> libc::c_int;
    pub fn mailimap_new(
        imap_progr_rate: size_t,
        imap_progr_fun: Option<unsafe extern "C" fn(_: size_t, _: size_t) -> ()>,
    ) -> *mut mailimap;
    pub fn mailimap_free(session: *mut mailimap);
    pub fn mailimap_set_timeout(session: *mut mailimap, timeout: time_t);
    pub fn libetpan_get_version_minor() -> libc::c_int;
    pub fn sqlite3_threadsafe() -> libc::c_int;
    pub fn strtol(
        _: *const libc::c_char,
        _: *mut *mut libc::c_char,
        _: libc::c_int,
    ) -> libc::c_long;
    pub fn __assert_rtn(
        _: *const libc::c_char,
        _: *const libc::c_char,
        _: libc::c_int,
        _: *const libc::c_char,
    ) -> !;
    pub fn mailmime_find_ct_parameter(
        _: *mut mailmime,
        name: *const libc::c_char,
    ) -> *mut mailmime_parameter;
    pub fn mailmime_transfer_decode(
        _: *mut mailmime,
        ret_decoded_data: *mut *const libc::c_char,
        ret_decoded_data_bytes: *mut size_t,
        ret_to_mmap_string_unref: *mut *mut libc::c_char,
    ) -> libc::c_int;
    pub fn mailimf_find_optional_field(
        _: *mut mailimf_fields,
        wanted_fld_name: *const libc::c_char,
    ) -> *mut mailimf_optional_field;
    pub fn rpgp_hash_sha256(bytes_ptr: *const uint8_t, bytes_len: size_t) -> *mut rpgp_cvec;
    pub fn carray_delete_slow(array: *mut carray, indx: libc::c_uint) -> libc::c_int;
    pub fn mailimf_mailbox_list_free(mb_list: *mut mailimf_mailbox_list);
    pub fn mailimf_mailbox_list_parse(
        message: *const libc::c_char,
        length: size_t,
        indx: *mut size_t,
        result: *mut *mut mailimf_mailbox_list,
    ) -> libc::c_int;
    pub fn mailmime_content_charset_get(content: *mut mailmime_content) -> *mut libc::c_char;
    pub fn charconv_buffer(
        tocode: *const libc::c_char,
        fromcode: *const libc::c_char,
        str: *const libc::c_char,
        length: size_t,
        result: *mut *mut libc::c_char,
        result_len: *mut size_t,
    ) -> libc::c_int;
    pub fn charconv_buffer_free(str: *mut libc::c_char);
    pub fn sscanf(_: *const libc::c_char, _: *const libc::c_char, _: ...) -> libc::c_int;
    pub fn sqlite3_close(_: *mut sqlite3) -> libc::c_int;
    pub fn sqlite3_busy_timeout(_: *mut sqlite3, ms: libc::c_int) -> libc::c_int;
    pub fn sqlite3_vmprintf(_: *const libc::c_char, _: ::std::ffi::VaList) -> *mut libc::c_char;
    pub fn sqlite3_open_v2(
        filename: *const libc::c_char,
        ppDb: *mut *mut sqlite3,
        flags: libc::c_int,
        zVfs: *const libc::c_char,
    ) -> libc::c_int;
    pub fn sqlite3_errmsg(_: *mut sqlite3) -> *const libc::c_char;
    pub fn sqlite3_prepare_v2(
        db: *mut sqlite3,
        zSql: *const libc::c_char,
        nByte: libc::c_int,
        ppStmt: *mut *mut sqlite3_stmt,
        pzTail: *mut *const libc::c_char,
    ) -> libc::c_int;

    // -- libetpan Methods

    #[no_mangle]
    pub fn gethostname(_: *mut libc::c_char, _: size_t) -> libc::c_int;
    #[no_mangle]
    pub fn mailsmtp_socket_connect(
        session: *mut mailsmtp,
        server: *const libc::c_char,
        port: uint16_t,
    ) -> libc::c_int;
    #[no_mangle]
    pub fn mailsmtp_socket_starttls(session: *mut mailsmtp) -> libc::c_int;
    #[no_mangle]
    pub fn mailsmtp_ssl_connect(
        session: *mut mailsmtp,
        server: *const libc::c_char,
        port: uint16_t,
    ) -> libc::c_int;
    #[no_mangle]
    pub fn mailsmtp_oauth2_authenticate(
        session: *mut mailsmtp,
        auth_user: *const libc::c_char,
        access_token: *const libc::c_char,
    ) -> libc::c_int;
    #[no_mangle]
    pub fn mailsmtp_new(
        progr_rate: size_t,
        progr_fun: Option<unsafe extern "C" fn(_: size_t, _: size_t) -> ()>,
    ) -> *mut mailsmtp;
    #[no_mangle]
    pub fn mailsmtp_free(session: *mut mailsmtp);
    #[no_mangle]
    pub fn mailsmtp_set_timeout(session: *mut mailsmtp, timeout: time_t);
    #[no_mangle]
    pub fn mailsmtp_auth(
        session: *mut mailsmtp,
        user: *const libc::c_char,
        pass: *const libc::c_char,
    ) -> libc::c_int;
    #[no_mangle]
    pub fn mailsmtp_helo(session: *mut mailsmtp) -> libc::c_int;
    #[no_mangle]
    pub fn mailsmtp_mail(session: *mut mailsmtp, from: *const libc::c_char) -> libc::c_int;
    #[no_mangle]
    pub fn mailsmtp_rcpt(session: *mut mailsmtp, to: *const libc::c_char) -> libc::c_int;
    #[no_mangle]
    pub fn mailsmtp_data(session: *mut mailsmtp) -> libc::c_int;
    #[no_mangle]
    pub fn mailsmtp_data_message(
        session: *mut mailsmtp,
        message: *const libc::c_char,
        size: size_t,
    ) -> libc::c_int;
    #[no_mangle]
    pub fn mailesmtp_ehlo(session: *mut mailsmtp) -> libc::c_int;
    #[no_mangle]
    pub fn mailesmtp_mail(
        session: *mut mailsmtp,
        from: *const libc::c_char,
        return_full: libc::c_int,
        envid: *const libc::c_char,
    ) -> libc::c_int;
    #[no_mangle]
    pub fn mailesmtp_rcpt(
        session: *mut mailsmtp,
        to: *const libc::c_char,
        notify: libc::c_int,
        orcpt: *const libc::c_char,
    ) -> libc::c_int;
    #[no_mangle]
    pub fn mailsmtp_strerror(errnum: libc::c_int) -> *const libc::c_char;
    #[no_mangle]
    pub fn mailesmtp_auth_sasl(
        session: *mut mailsmtp,
        auth_type: *const libc::c_char,
        server_fqdn: *const libc::c_char,
        local_ip_port: *const libc::c_char,
        remote_ip_port: *const libc::c_char,
        login: *const libc::c_char,
        auth_name: *const libc::c_char,
        password: *const libc::c_char,
        realm: *const libc::c_char,
    ) -> libc::c_int;
    #[no_mangle]
    pub fn mailsmtp_set_progress_callback(
        session: *mut mailsmtp,
        progr_fun: Option<unsafe extern "C" fn(_: size_t, _: size_t, _: *mut libc::c_void) -> ()>,
        context: *mut libc::c_void,
    );

    // -- DC Methods

    pub fn dc_strbuilder_catf(_: *mut dc_strbuilder_t, format: *const libc::c_char, _: ...);
    pub fn dc_mprintf(format: *const libc::c_char, _: ...) -> *mut libc::c_char;
}
