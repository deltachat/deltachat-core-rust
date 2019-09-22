use libc;
use libc::toupper;

use crate::charconv::*;
use crate::mailimf::*;
use crate::mailmime_content::*;
use crate::mailmime_types::*;
use crate::mmapstring::*;
use crate::other::*;

pub const MAIL_CHARCONV_ERROR_CONV: libc::c_uint = 3;
pub const MAIL_CHARCONV_ERROR_UNKNOWN_CHARSET: libc::c_uint = 1;
pub const MAIL_CHARCONV_ERROR_MEMORY: libc::c_uint = 2;
pub const TYPE_WORD: libc::c_uint = 1;
pub const TYPE_ENCODED_WORD: libc::c_uint = 2;
pub const MAILMIME_ENCODING_Q: libc::c_uint = 1;
pub const MAILMIME_ENCODING_B: libc::c_uint = 0;
pub const TYPE_ERROR: libc::c_uint = 0;

pub unsafe fn mailmime_encoded_phrase_parse(
    mut default_fromcode: *const libc::c_char,
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut tocode: *const libc::c_char,
    mut result: *mut *mut libc::c_char,
) -> libc::c_int {
    let mut current_block: u64;
    let mut gphrase: *mut MMAPString = 0 as *mut MMAPString;
    let mut word: *mut mailmime_encoded_word = 0 as *mut mailmime_encoded_word;
    let mut first: libc::c_int = 0;
    let mut cur_token: size_t = 0;
    let mut r: libc::c_int = 0;
    let mut res: libc::c_int = 0;
    let mut str: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut wordutf8: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut type_0: libc::c_int = 0;
    let mut missing_closing_quote: libc::c_int = 0;
    cur_token = *indx;
    gphrase = mmap_string_new(b"\x00" as *const u8 as *const libc::c_char);
    if gphrase.is_null() {
        res = MAILIMF_ERROR_MEMORY as libc::c_int
    } else {
        first = 1i32;
        type_0 = TYPE_ERROR as libc::c_int;
        loop {
            let mut has_fwd: libc::c_int = 0;
            word = 0 as *mut mailmime_encoded_word;
            r = mailmime_encoded_word_parse(
                message,
                length,
                &mut cur_token,
                &mut word,
                &mut has_fwd,
                &mut missing_closing_quote,
            );
            if r == MAILIMF_NO_ERROR as libc::c_int {
                if 0 == first && 0 != has_fwd {
                    if type_0 != TYPE_ENCODED_WORD as libc::c_int {
                        if mmap_string_append_c(gphrase, ' ' as i32 as libc::c_char).is_null() {
                            mailmime_encoded_word_free(word);
                            res = MAILIMF_ERROR_MEMORY as libc::c_int;
                            current_block = 13246848547199022064;
                            break;
                        }
                    }
                }
                type_0 = TYPE_ENCODED_WORD as libc::c_int;
                wordutf8 = 0 as *mut libc::c_char;
                r = charconv(
                    tocode,
                    (*word).wd_charset,
                    (*word).wd_text,
                    strlen((*word).wd_text),
                    &mut wordutf8,
                );
                match r {
                    2 => {
                        mailmime_encoded_word_free(word);
                        res = MAILIMF_ERROR_MEMORY as libc::c_int;
                        current_block = 13246848547199022064;
                        break;
                    }
                    1 => {
                        r = charconv(
                            tocode,
                            b"iso-8859-1\x00" as *const u8 as *const libc::c_char,
                            (*word).wd_text,
                            strlen((*word).wd_text),
                            &mut wordutf8,
                        )
                    }
                    3 => {
                        mailmime_encoded_word_free(word);
                        res = MAILIMF_ERROR_PARSE as libc::c_int;
                        current_block = 13246848547199022064;
                        break;
                    }
                    _ => {}
                }
                match r {
                    2 => {
                        mailmime_encoded_word_free(word);
                        res = MAILIMF_ERROR_MEMORY as libc::c_int;
                        current_block = 13246848547199022064;
                        break;
                    }
                    3 => {
                        mailmime_encoded_word_free(word);
                        res = MAILIMF_ERROR_PARSE as libc::c_int;
                        current_block = 13246848547199022064;
                        break;
                    }
                    _ => {
                        if !wordutf8.is_null() {
                            if mmap_string_append(gphrase, wordutf8).is_null() {
                                mailmime_encoded_word_free(word);
                                free(wordutf8 as *mut libc::c_void);
                                res = MAILIMF_ERROR_MEMORY as libc::c_int;
                                current_block = 13246848547199022064;
                                break;
                            } else {
                                free(wordutf8 as *mut libc::c_void);
                            }
                        }
                        mailmime_encoded_word_free(word);
                        first = 0i32
                    }
                }
            } else if !(r == MAILIMF_ERROR_PARSE as libc::c_int) {
                /* do nothing */
                res = r;
                current_block = 13246848547199022064;
                break;
            }
            if !(r == MAILIMF_ERROR_PARSE as libc::c_int) {
                continue;
            }
            let mut raw_word: *mut libc::c_char = 0 as *mut libc::c_char;
            raw_word = 0 as *mut libc::c_char;
            r = mailmime_non_encoded_word_parse(
                message,
                length,
                &mut cur_token,
                &mut raw_word,
                &mut has_fwd,
            );
            if r == MAILIMF_NO_ERROR as libc::c_int {
                if 0 == first && 0 != has_fwd {
                    if mmap_string_append_c(gphrase, ' ' as i32 as libc::c_char).is_null() {
                        free(raw_word as *mut libc::c_void);
                        res = MAILIMF_ERROR_MEMORY as libc::c_int;
                        current_block = 13246848547199022064;
                        break;
                    }
                }
                type_0 = TYPE_WORD as libc::c_int;
                wordutf8 = 0 as *mut libc::c_char;
                r = charconv(
                    tocode,
                    default_fromcode,
                    raw_word,
                    strlen(raw_word),
                    &mut wordutf8,
                );
                match r {
                    2 => {
                        free(raw_word as *mut libc::c_void);
                        res = MAILIMF_ERROR_MEMORY as libc::c_int;
                        current_block = 13246848547199022064;
                        break;
                    }
                    1 | 3 => {
                        free(raw_word as *mut libc::c_void);
                        res = MAILIMF_ERROR_PARSE as libc::c_int;
                        current_block = 13246848547199022064;
                        break;
                    }
                    _ => {
                        if mmap_string_append(gphrase, wordutf8).is_null() {
                            free(wordutf8 as *mut libc::c_void);
                            free(raw_word as *mut libc::c_void);
                            res = MAILIMF_ERROR_MEMORY as libc::c_int;
                            current_block = 13246848547199022064;
                            break;
                        } else {
                            free(wordutf8 as *mut libc::c_void);
                            free(raw_word as *mut libc::c_void);
                            first = 0i32
                        }
                    }
                }
            } else if r == MAILIMF_ERROR_PARSE as libc::c_int {
                r = mailimf_fws_parse(message, length, &mut cur_token);
                if r != MAILIMF_NO_ERROR as libc::c_int {
                    current_block = 5005389895767293342;
                    break;
                }
                if mmap_string_append_c(gphrase, ' ' as i32 as libc::c_char).is_null() {
                    res = MAILIMF_ERROR_MEMORY as libc::c_int;
                    current_block = 13246848547199022064;
                    break;
                } else {
                    first = 0i32;
                    current_block = 5005389895767293342;
                    break;
                }
            } else {
                res = r;
                current_block = 13246848547199022064;
                break;
            }
        }
        match current_block {
            5005389895767293342 => {
                if 0 != first {
                    if cur_token != length {
                        res = MAILIMF_ERROR_PARSE as libc::c_int;
                        current_block = 13246848547199022064;
                    } else {
                        current_block = 7072655752890836508;
                    }
                } else {
                    current_block = 7072655752890836508;
                }
                match current_block {
                    13246848547199022064 => {}
                    _ => {
                        str = strdup((*gphrase).str_0);
                        if str.is_null() {
                            res = MAILIMF_ERROR_MEMORY as libc::c_int
                        } else {
                            mmap_string_free(gphrase);
                            *result = str;
                            *indx = cur_token;
                            return MAILIMF_NO_ERROR as libc::c_int;
                        }
                    }
                }
            }
            _ => {}
        }
        mmap_string_free(gphrase);
    }
    return res;
}
unsafe fn mailmime_non_encoded_word_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut libc::c_char,
    mut p_has_fwd: *mut libc::c_int,
) -> libc::c_int {
    let mut end: libc::c_int = 0;
    let mut cur_token: size_t = 0;
    let mut res: libc::c_int = 0;
    let mut text: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut r: libc::c_int = 0;
    let mut begin: size_t = 0;
    let mut state: libc::c_int = 0;
    let mut has_fwd: libc::c_int = 0;
    cur_token = *indx;
    has_fwd = 0i32;
    r = mailimf_fws_parse(message, length, &mut cur_token);
    if r == MAILIMF_NO_ERROR as libc::c_int {
        has_fwd = 1i32
    }
    if r != MAILIMF_NO_ERROR as libc::c_int && r != MAILIMF_ERROR_PARSE as libc::c_int {
        res = r
    } else {
        begin = cur_token;
        state = 0i32;
        end = 0i32;
        while !(cur_token >= length) {
            let mut current_block_17: u64;
            match *message.offset(cur_token as isize) as libc::c_int {
                32 | 9 | 13 | 10 => {
                    state = 0i32;
                    end = 1i32;
                    current_block_17 = 16924917904204750491;
                }
                61 => {
                    state = 1i32;
                    current_block_17 = 16924917904204750491;
                }
                63 => {
                    if state == 1i32 {
                        cur_token = cur_token.wrapping_sub(1);
                        end = 1i32
                    }
                    current_block_17 = 10192508258555769664;
                }
                _ => {
                    current_block_17 = 10192508258555769664;
                }
            }
            match current_block_17 {
                10192508258555769664 => state = 0i32,
                _ => {}
            }
            if 0 != end {
                break;
            }
            cur_token = cur_token.wrapping_add(1)
        }
        if cur_token.wrapping_sub(begin) == 0i32 as libc::size_t {
            res = MAILIMF_ERROR_PARSE as libc::c_int
        } else {
            text = malloc(
                cur_token
                    .wrapping_sub(begin)
                    .wrapping_add(1i32 as libc::size_t),
            ) as *mut libc::c_char;
            if text.is_null() {
                res = MAILIMF_ERROR_MEMORY as libc::c_int
            } else {
                memcpy(
                    text as *mut libc::c_void,
                    message.offset(begin as isize) as *const libc::c_void,
                    cur_token.wrapping_sub(begin),
                );
                *text.offset(cur_token.wrapping_sub(begin) as isize) =
                    '\u{0}' as i32 as libc::c_char;
                *indx = cur_token;
                *result = text;
                *p_has_fwd = has_fwd;
                return MAILIMF_NO_ERROR as libc::c_int;
            }
        }
    }
    return res;
}

pub unsafe fn mailmime_encoded_word_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut mailmime_encoded_word,
    mut p_has_fwd: *mut libc::c_int,
    mut p_missing_closing_quote: *mut libc::c_int,
) -> libc::c_int {
    let mut current_block: u64;
    /*
    Parse the following, when a unicode character encoding is split.
    =?UTF-8?B?4Lij4Liw4LmA4Lia4Li04LiU4LiE4Lin4Liy4Lih4Lih4Lix4LiZ4Liq4LmM?=
    =?UTF-8?B?4LmA4LiV4LmH4Lih4Lie4Li04LiB4Lix4LiUIFRSQU5TRk9STUVSUyA0IOC4?=
    =?UTF-8?B?oeC4seC4meC4quC5jOC4hOC4o+C4muC4l+C4uOC4geC4o+C4sOC4muC4miDg?=
    =?UTF-8?B?uJfguLXguYjguYDguJTguLXguKLguKfguYPguJnguYDguKHguLfguK3guIfg?=
    =?UTF-8?B?uYTguJfguKI=?=
    Expected result:
    ระเบิดความมันส์เต็มพิกัด TRANSFORMERS 4 มันส์ครบทุกระบบ ที่เดียวในเมืองไทย
    libetpan result:
    ระเบิดความมันส์เต็มพิกัด TRANSFORMERS 4 ?ันส์ครบทุกระบบ ??ี่เดียวในเมือง??ทย

    See https://github.com/dinhviethoa/libetpan/pull/211
    */
    let mut cur_token: size_t = 0;
    let mut charset: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut encoding: libc::c_int = 0;
    let mut body: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut old_body_len: size_t = 0;
    let mut text: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut end_encoding: size_t = 0;
    let mut lookfwd_cur_token: size_t = 0;
    let mut lookfwd_charset: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut lookfwd_encoding: libc::c_int = 0;
    let mut copy_len: size_t = 0;
    let mut decoded_token: size_t = 0;
    let mut decoded: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut decoded_len: size_t = 0;
    let mut ew: *mut mailmime_encoded_word = 0 as *mut mailmime_encoded_word;
    let mut r: libc::c_int = 0;
    let mut res: libc::c_int = 0;
    let mut opening_quote: libc::c_int = 0;
    let mut end: libc::c_int = 0;
    let mut has_fwd: libc::c_int = 0;
    let mut missing_closing_quote: libc::c_int = 0;
    cur_token = *indx;
    text = 0 as *mut libc::c_char;
    lookfwd_charset = 0 as *mut libc::c_char;
    missing_closing_quote = 0i32;
    has_fwd = 0i32;
    r = mailimf_fws_parse(message, length, &mut cur_token);
    if r == MAILIMF_NO_ERROR as libc::c_int {
        has_fwd = 1i32
    }
    if r != MAILIMF_NO_ERROR as libc::c_int && r != MAILIMF_ERROR_PARSE as libc::c_int {
        res = r
    } else {
        opening_quote = 0i32;
        r = mailimf_char_parse(message, length, &mut cur_token, '\"' as i32 as libc::c_char);
        if r == MAILIMF_NO_ERROR as libc::c_int {
            opening_quote = 1i32;
            current_block = 17788412896529399552;
        } else if r == MAILIMF_ERROR_PARSE as libc::c_int {
            current_block = 17788412896529399552;
        } else {
            /* do nothing */
            res = r;
            current_block = 7995813543095296079;
        }
        match current_block {
            7995813543095296079 => {}
            _ => {
                r = mailimf_token_case_insensitive_len_parse(
                    message,
                    length,
                    &mut cur_token,
                    b"=?\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
                    strlen(b"=?\x00" as *const u8 as *const libc::c_char),
                );
                if r != MAILIMF_NO_ERROR as libc::c_int {
                    res = r
                } else {
                    r = mailmime_charset_parse(message, length, &mut cur_token, &mut charset);
                    if r != MAILIMF_NO_ERROR as libc::c_int {
                        res = r
                    } else {
                        r = mailimf_char_parse(
                            message,
                            length,
                            &mut cur_token,
                            '?' as i32 as libc::c_char,
                        );
                        if r != MAILIMF_NO_ERROR as libc::c_int {
                            res = r
                        } else {
                            r = mailmime_encoding_parse(
                                message,
                                length,
                                &mut cur_token,
                                &mut encoding,
                            );
                            if r != MAILIMF_NO_ERROR as libc::c_int {
                                res = r
                            } else {
                                r = mailimf_char_parse(
                                    message,
                                    length,
                                    &mut cur_token,
                                    '?' as i32 as libc::c_char,
                                );
                                if r != MAILIMF_NO_ERROR as libc::c_int {
                                    res = r
                                } else {
                                    lookfwd_cur_token = cur_token;
                                    body = 0 as *mut libc::c_char;
                                    old_body_len = 0i32 as size_t;
                                    loop {
                                        let mut has_base64_padding: libc::c_int = 0;
                                        end = 0i32;
                                        has_base64_padding = 0i32;
                                        end_encoding = cur_token;
                                        while !(end_encoding >= length) {
                                            if end_encoding.wrapping_add(1i32 as libc::size_t)
                                                < length
                                            {
                                                if *message.offset(end_encoding as isize)
                                                    as libc::c_int
                                                    == '?' as i32
                                                    && *message.offset(
                                                        end_encoding
                                                            .wrapping_add(1i32 as libc::size_t)
                                                            as isize,
                                                    )
                                                        as libc::c_int
                                                        == '=' as i32
                                                {
                                                    end = 1i32
                                                }
                                            }
                                            if 0 != end {
                                                break;
                                            }
                                            end_encoding = end_encoding.wrapping_add(1)
                                        }
                                        copy_len = end_encoding.wrapping_sub(lookfwd_cur_token);
                                        if copy_len > 0i32 as libc::size_t {
                                            if encoding == MAILMIME_ENCODING_B as libc::c_int {
                                                if end_encoding >= 1i32 as libc::size_t {
                                                    if *message.offset(
                                                        end_encoding
                                                            .wrapping_sub(1i32 as libc::size_t)
                                                            as isize,
                                                    )
                                                        as libc::c_int
                                                        == '=' as i32
                                                    {
                                                        has_base64_padding = 1i32
                                                    }
                                                }
                                            }
                                            body = realloc(
                                                body as *mut libc::c_void,
                                                old_body_len
                                                    .wrapping_add(copy_len)
                                                    .wrapping_add(1i32 as libc::size_t),
                                            )
                                                as *mut libc::c_char;
                                            if body.is_null() {
                                                res = MAILIMF_ERROR_MEMORY as libc::c_int;
                                                current_block = 13900684162107791171;
                                                break;
                                            } else {
                                                memcpy(
                                                    body.offset(old_body_len as isize)
                                                        as *mut libc::c_void,
                                                    &*message.offset(cur_token as isize)
                                                        as *const libc::c_char
                                                        as *const libc::c_void,
                                                    copy_len,
                                                );
                                                *body
                                                    .offset(old_body_len.wrapping_add(copy_len)
                                                        as isize) = '\u{0}' as i32 as libc::c_char;
                                                old_body_len = (old_body_len as libc::size_t)
                                                    .wrapping_add(copy_len)
                                                    as size_t
                                                    as size_t
                                            }
                                        }
                                        cur_token = end_encoding;
                                        r = mailimf_token_case_insensitive_len_parse(
                                            message,
                                            length,
                                            &mut cur_token,
                                            b"?=\x00" as *const u8 as *const libc::c_char
                                                as *mut libc::c_char,
                                            strlen(b"?=\x00" as *const u8 as *const libc::c_char),
                                        );
                                        if r != MAILIMF_NO_ERROR as libc::c_int {
                                            current_block = 2652804691515851435;
                                            break;
                                        }
                                        if 0 != has_base64_padding {
                                            current_block = 2652804691515851435;
                                            break;
                                        }
                                        lookfwd_cur_token = cur_token;
                                        r = mailimf_fws_parse(
                                            message,
                                            length,
                                            &mut lookfwd_cur_token,
                                        );
                                        if r != MAILIMF_NO_ERROR as libc::c_int
                                            && r != MAILIMF_ERROR_PARSE as libc::c_int
                                        {
                                            current_block = 2652804691515851435;
                                            break;
                                        }
                                        r = mailimf_token_case_insensitive_len_parse(
                                            message,
                                            length,
                                            &mut lookfwd_cur_token,
                                            b"=?\x00" as *const u8 as *const libc::c_char
                                                as *mut libc::c_char,
                                            strlen(b"=?\x00" as *const u8 as *const libc::c_char),
                                        );
                                        if r != MAILIMF_NO_ERROR as libc::c_int {
                                            current_block = 2652804691515851435;
                                            break;
                                        }
                                        r = mailmime_charset_parse(
                                            message,
                                            length,
                                            &mut lookfwd_cur_token,
                                            &mut lookfwd_charset,
                                        );
                                        if r != MAILIMF_NO_ERROR as libc::c_int {
                                            current_block = 2652804691515851435;
                                            break;
                                        }
                                        r = mailimf_char_parse(
                                            message,
                                            length,
                                            &mut lookfwd_cur_token,
                                            '?' as i32 as libc::c_char,
                                        );
                                        if r != MAILIMF_NO_ERROR as libc::c_int {
                                            current_block = 2652804691515851435;
                                            break;
                                        }
                                        r = mailmime_encoding_parse(
                                            message,
                                            length,
                                            &mut lookfwd_cur_token,
                                            &mut lookfwd_encoding,
                                        );
                                        if r != MAILIMF_NO_ERROR as libc::c_int {
                                            current_block = 2652804691515851435;
                                            break;
                                        }
                                        r = mailimf_char_parse(
                                            message,
                                            length,
                                            &mut lookfwd_cur_token,
                                            '?' as i32 as libc::c_char,
                                        );
                                        if r != MAILIMF_NO_ERROR as libc::c_int {
                                            current_block = 2652804691515851435;
                                            break;
                                        }
                                        if strcasecmp(charset, lookfwd_charset) == 0i32
                                            && encoding == lookfwd_encoding
                                        {
                                            cur_token = lookfwd_cur_token;
                                            mailmime_charset_free(lookfwd_charset);
                                            lookfwd_charset = 0 as *mut libc::c_char
                                        } else {
                                            /* the next charset is not matched with the current one,
                                            therefore exit the loop to decode the body appended so far */
                                            current_block = 2652804691515851435;
                                            break;
                                        }
                                    }
                                    match current_block {
                                        2652804691515851435 => {
                                            if !lookfwd_charset.is_null() {
                                                mailmime_charset_free(lookfwd_charset);
                                                lookfwd_charset = 0 as *mut libc::c_char
                                            }
                                            if body.is_null() {
                                                body = strdup(
                                                    b"\x00" as *const u8 as *const libc::c_char,
                                                );
                                                if body.is_null() {
                                                    res = MAILIMF_ERROR_MEMORY as libc::c_int;
                                                    current_block = 13900684162107791171;
                                                } else {
                                                    current_block = 16778110326724371720;
                                                }
                                            } else {
                                                current_block = 16778110326724371720;
                                            }
                                            match current_block {
                                                13900684162107791171 => {}
                                                _ => {
                                                    decoded_token = 0i32 as size_t;
                                                    decoded_len = 0i32 as size_t;
                                                    decoded = 0 as *mut libc::c_char;
                                                    match encoding {
                                                        0 => {
                                                            r = mailmime_base64_body_parse(
                                                                body,
                                                                strlen(body),
                                                                &mut decoded_token,
                                                                &mut decoded,
                                                                &mut decoded_len,
                                                            );
                                                            if r != MAILIMF_NO_ERROR as libc::c_int
                                                            {
                                                                res = r;
                                                                current_block =
                                                                    13900684162107791171;
                                                            } else {
                                                                current_block = 7337917895049117968;
                                                            }
                                                        }
                                                        1 => {
                                                            r =
                                                                mailmime_quoted_printable_body_parse(body,
                                                                                                     strlen(body),
                                                                                                     &mut decoded_token,
                                                                                                     &mut decoded,
                                                                                                     &mut decoded_len,
                                                                                                     1i32);
                                                            if r != MAILIMF_NO_ERROR as libc::c_int
                                                            {
                                                                res = r;
                                                                current_block =
                                                                    13900684162107791171;
                                                            } else {
                                                                current_block = 7337917895049117968;
                                                            }
                                                        }
                                                        _ => {
                                                            current_block = 7337917895049117968;
                                                        }
                                                    }
                                                    match current_block {
                                                        13900684162107791171 => {}
                                                        _ => {
                                                            text =
                                                                malloc(decoded_len.wrapping_add(
                                                                    1i32 as libc::size_t,
                                                                ))
                                                                    as *mut libc::c_char;
                                                            if text.is_null() {
                                                                res = MAILIMF_ERROR_MEMORY
                                                                    as libc::c_int
                                                            } else {
                                                                if decoded_len
                                                                    > 0i32 as libc::size_t
                                                                {
                                                                    memcpy(
                                                                        text as *mut libc::c_void,
                                                                        decoded
                                                                            as *const libc::c_void,
                                                                        decoded_len,
                                                                    );
                                                                }
                                                                *text
                                                                    .offset(decoded_len as isize) =
                                                                    '\u{0}' as i32 as libc::c_char;
                                                                if 0 != opening_quote {
                                                                    r = mailimf_char_parse(
                                                                        message,
                                                                        length,
                                                                        &mut cur_token,
                                                                        '\"' as i32 as libc::c_char,
                                                                    );
                                                                    if r == MAILIMF_ERROR_PARSE
                                                                        as libc::c_int
                                                                    {
                                                                        missing_closing_quote = 1i32
                                                                    }
                                                                }
                                                                if strcasecmp(
                                                                    charset,
                                                                    b"utf8\x00" as *const u8
                                                                        as *const libc::c_char,
                                                                ) == 0i32
                                                                {
                                                                    free(
                                                                        charset
                                                                            as *mut libc::c_void,
                                                                    );
                                                                    charset = strdup(
                                                                        b"utf-8\x00" as *const u8
                                                                            as *const libc::c_char,
                                                                    )
                                                                }
                                                                ew = mailmime_encoded_word_new(
                                                                    charset, text,
                                                                );
                                                                if ew.is_null() {
                                                                    res = MAILIMF_ERROR_MEMORY
                                                                        as libc::c_int
                                                                } else {
                                                                    *result = ew;
                                                                    *indx = cur_token;
                                                                    *p_has_fwd = has_fwd;
                                                                    *p_missing_closing_quote =
                                                                        missing_closing_quote;
                                                                    mailmime_decoded_part_free(
                                                                        decoded,
                                                                    );
                                                                    free(body as *mut libc::c_void);
                                                                    return MAILIMF_NO_ERROR
                                                                        as libc::c_int;
                                                                }
                                                            }
                                                            mailmime_decoded_part_free(decoded);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        _ => {}
                                    }
                                    free(body as *mut libc::c_void);
                                    mailmime_encoded_text_free(text);
                                }
                            }
                        }
                        mailmime_charset_free(charset);
                    }
                }
            }
        }
    }
    return res;
}
unsafe fn mailmime_encoding_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut libc::c_int,
) -> libc::c_int {
    let mut cur_token: size_t = 0;
    let mut encoding: libc::c_int = 0;
    cur_token = *indx;
    if cur_token >= length {
        return MAILIMF_ERROR_PARSE as libc::c_int;
    }
    match toupper(*message.offset(cur_token as isize) as libc::c_uchar as libc::c_int)
        as libc::c_char as libc::c_int
    {
        81 => encoding = MAILMIME_ENCODING_Q as libc::c_int,
        66 => encoding = MAILMIME_ENCODING_B as libc::c_int,
        _ => return MAILIMF_ERROR_INVAL as libc::c_int,
    }
    cur_token = cur_token.wrapping_add(1);
    *result = encoding;
    *indx = cur_token;
    return MAILIMF_NO_ERROR as libc::c_int;
}

/*
 * libEtPan! -- a mail stuff library
 *
 * Copyright (C) 2001, 2005 - DINH Viet Hoa
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. Neither the name of the libEtPan! project nor the names of its
 *    contributors may be used to endorse or promote products derived
 *    from this software without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE AUTHORS AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE AUTHORS OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 */
/*
 * $Id: mailmime_decode.c,v 1.37 2010/11/16 20:52:28 hoa Exp $
 */
/*
  RFC 2047 : MIME (Multipurpose Internet Mail Extensions) Part Three:
             Message Header Extensions for Non-ASCII Text
*/
unsafe fn mailmime_charset_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut charset: *mut *mut libc::c_char,
) -> libc::c_int {
    return mailmime_etoken_parse(message, length, indx, charset);
}
unsafe fn mailmime_etoken_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut libc::c_char,
) -> libc::c_int {
    return mailimf_custom_string_parse(message, length, indx, result, Some(is_etoken_char));
}

pub unsafe fn is_etoken_char(mut ch: libc::c_char) -> libc::c_int {
    let mut uch: libc::c_uchar = ch as libc::c_uchar;
    if (uch as libc::c_int) < 31i32 {
        return 0i32;
    }
    match uch as libc::c_int {
        32 | 40 | 41 | 60 | 62 | 64 | 44 | 59 | 58 | 34 | 47 | 91 | 93 | 63 | 61 => return 0i32,
        _ => {}
    }
    return 1i32;
}
