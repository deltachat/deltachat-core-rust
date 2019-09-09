use std::borrow::Cow;
use std::ffi::CString;
use std::ptr;

use charset::Charset;
use mmime::mailmime_decode::*;
use mmime::mmapstring::*;
use mmime::other::*;
use percent_encoding::{percent_decode, utf8_percent_encode, AsciiSet, CONTROLS};

use crate::dc_tools::*;
use crate::types::*;
use crate::x::*;

pub unsafe fn dc_encode_header_words(to_encode: *const libc::c_char) -> *mut libc::c_char {
    let mut ok_to_continue = true;
    let mut ret_str: *mut libc::c_char = ptr::null_mut();
    let mut cur: *const libc::c_char = to_encode;
    let mmapstr: *mut MMAPString = mmap_string_new(b"\x00" as *const u8 as *const libc::c_char);
    if to_encode.is_null() || mmapstr.is_null() {
        ok_to_continue = false;
    }
    loop {
        if !ok_to_continue {
            if !mmapstr.is_null() {
                mmap_string_free(mmapstr);
            }
            break;
        } else {
            if *cur as libc::c_int != '\u{0}' as i32 {
                let begin: *const libc::c_char;
                let mut end: *const libc::c_char;
                let mut do_quote: bool;
                let mut quote_words: libc::c_int;
                begin = cur;
                end = begin;
                quote_words = 0i32;
                do_quote = true;
                while *cur as libc::c_int != '\u{0}' as i32 {
                    get_word(cur, &mut cur, &mut do_quote);
                    if !do_quote {
                        break;
                    }
                    quote_words = 1i32;
                    end = cur;
                    if *cur as libc::c_int != '\u{0}' as i32 {
                        cur = cur.offset(1isize)
                    }
                }
                if 0 != quote_words {
                    if !quote_word(
                        b"utf-8\x00" as *const u8 as *const libc::c_char,
                        mmapstr,
                        begin,
                        end.wrapping_offset_from(begin) as size_t,
                    ) {
                        ok_to_continue = false;
                        continue;
                    }
                    if *end as libc::c_int == ' ' as i32 || *end as libc::c_int == '\t' as i32 {
                        if mmap_string_append_c(mmapstr, *end).is_null() {
                            ok_to_continue = false;
                            continue;
                        }
                        end = end.offset(1isize)
                    }
                    if *end as libc::c_int != '\u{0}' as i32 {
                        if mmap_string_append_len(
                            mmapstr,
                            end,
                            cur.wrapping_offset_from(end) as size_t,
                        )
                        .is_null()
                        {
                            ok_to_continue = false;
                            continue;
                        }
                    }
                } else if mmap_string_append_len(
                    mmapstr,
                    begin,
                    cur.wrapping_offset_from(begin) as size_t,
                )
                .is_null()
                {
                    ok_to_continue = false;
                    continue;
                }
                if !(*cur as libc::c_int == ' ' as i32 || *cur as libc::c_int == '\t' as i32) {
                    continue;
                }
                if mmap_string_append_c(mmapstr, *cur).is_null() {
                    ok_to_continue = false;
                    continue;
                }
                cur = cur.offset(1isize);
            } else {
                ret_str = strdup((*mmapstr).str_0);
                ok_to_continue = false;
            }
        }
    }

    ret_str
}

unsafe fn quote_word(
    display_charset: *const libc::c_char,
    mmapstr: *mut MMAPString,
    word: *const libc::c_char,
    size: size_t,
) -> bool {
    let mut cur: *const libc::c_char;
    let mut i: size_t = 0i32 as size_t;
    let mut hex: [libc::c_char; 4] = [0; 4];
    // let mut col: libc::c_int = 0i32;
    if mmap_string_append(mmapstr, b"=?\x00" as *const u8 as *const libc::c_char).is_null() {
        return false;
    }
    if mmap_string_append(mmapstr, display_charset).is_null() {
        return false;
    }
    if mmap_string_append(mmapstr, b"?Q?\x00" as *const u8 as *const libc::c_char).is_null() {
        return false;
    }
    // col = (*mmapstr).len as libc::c_int;
    cur = word;
    while i < size {
        let mut do_quote_char = false;
        match *cur as u8 as char {
            ',' | ':' | '!' | '"' | '#' | '$' | '@' | '[' | '\\' | ']' | '^' | '`' | '{' | '|'
            | '}' | '~' | '=' | '?' | '_' => do_quote_char = true,
            _ => {
                if *cur as u8 >= 128 {
                    do_quote_char = true;
                }
            }
        }
        if do_quote_char {
            print_hex(hex.as_mut_ptr(), cur);
            if mmap_string_append(mmapstr, hex.as_mut_ptr()).is_null() {
                return false;
            }
        // col += 3i32
        } else {
            if *cur as libc::c_int == ' ' as i32 {
                if mmap_string_append_c(mmapstr, '_' as i32 as libc::c_char).is_null() {
                    return false;
                }
            } else if mmap_string_append_c(mmapstr, *cur).is_null() {
                return false;
            }
            // col += 3i32
        }
        cur = cur.offset(1isize);
        i = i.wrapping_add(1)
    }
    if mmap_string_append(mmapstr, b"?=\x00" as *const u8 as *const libc::c_char).is_null() {
        return false;
    }

    true
}

unsafe fn get_word(
    begin: *const libc::c_char,
    pend: *mut *const libc::c_char,
    pto_be_quoted: *mut bool,
) {
    let mut cur: *const libc::c_char = begin;
    while *cur as libc::c_int != ' ' as i32
        && *cur as libc::c_int != '\t' as i32
        && *cur as libc::c_int != '\u{0}' as i32
    {
        cur = cur.offset(1isize)
    }
    *pto_be_quoted = to_be_quoted(begin, cur.wrapping_offset_from(begin) as size_t);
    *pend = cur;
}

/* ******************************************************************************
 * Encode/decode header words, RFC 2047
 ******************************************************************************/

/* see comment below */
unsafe fn to_be_quoted(word: *const libc::c_char, size: size_t) -> bool {
    let mut cur: *const libc::c_char = word;
    let mut i: size_t = 0i32 as size_t;
    while i < size {
        match *cur as libc::c_int {
            44 | 58 | 33 | 34 | 35 | 36 | 64 | 91 | 92 | 93 | 94 | 96 | 123 | 124 | 125 | 126
            | 61 | 63 | 95 => return true,
            _ => {
                if *cur as libc::c_uchar as libc::c_int >= 128i32 {
                    return true;
                }
            }
        }
        cur = cur.offset(1isize);
        i = i.wrapping_add(1)
    }

    false
}

pub unsafe fn dc_decode_header_words(in_0: *const libc::c_char) -> *mut libc::c_char {
    if in_0.is_null() {
        return ptr::null_mut();
    }
    let mut out: *mut libc::c_char = ptr::null_mut();
    let mut cur_token: size_t = 0i32 as size_t;
    let r: libc::c_int = mailmime_encoded_phrase_parse(
        b"iso-8859-1\x00" as *const u8 as *const libc::c_char,
        in_0,
        strlen(in_0),
        &mut cur_token,
        b"utf-8\x00" as *const u8 as *const libc::c_char,
        &mut out,
    );
    if r != MAILIMF_NO_ERROR as libc::c_int || out.is_null() {
        out = dc_strdup(in_0)
    }

    out
}

pub fn dc_needs_ext_header(to_check: impl AsRef<str>) -> bool {
    let to_check = to_check.as_ref();

    if to_check.is_empty() {
        return false;
    }

    to_check.chars().any(|c| {
        !(c.is_ascii_alphanumeric()
            || c == '-'
            || c == '_'
            || c == '_'
            || c == '.'
            || c == '~'
            || c == '%')
    })
}

const EXT_ASCII_ST: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'-')
    .add(b'_')
    .add(b'.')
    .add(b'~')
    .add(b'%');

/// Encode an UTF-8 string to the extended header format.
pub fn dc_encode_ext_header(to_encode: impl AsRef<str>) -> String {
    let encoded = utf8_percent_encode(to_encode.as_ref(), &EXT_ASCII_ST);
    format!("utf-8''{}", encoded)
}

/// Decode an extended-header-format strings to UTF-8.
pub fn dc_decode_ext_header(to_decode: &[u8]) -> Cow<str> {
    if let Some(index) = bytes!(b'\'').find(to_decode) {
        let (charset, rest) = to_decode.split_at(index);
        if !charset.is_empty() {
            // skip language
            if let Some(index2) = bytes!(b'\'').find(&rest[1..]) {
                let decoded = percent_decode(&rest[index2 + 2..]);

                if charset != b"utf-8" && charset != b"UTF-8" {
                    if let Some(encoding) = Charset::for_label(charset) {
                        let bytes = decoded.collect::<Vec<u8>>();
                        let (res, _, _) = encoding.decode(&bytes);
                        return Cow::Owned(res.into_owned());
                    } else {
                        return decoded.decode_utf8_lossy();
                    }
                } else {
                    return decoded.decode_utf8_lossy();
                }
            }
        }
    }

    String::from_utf8_lossy(to_decode)
}

unsafe fn print_hex(target: *mut libc::c_char, cur: *const libc::c_char) {
    assert!(!target.is_null());
    assert!(!cur.is_null());

    let bytes = std::slice::from_raw_parts(cur as *const _, strlen(cur));
    let raw = CString::yolo(format!("={}", &hex::encode_upper(bytes)[..2]));
    libc::memcpy(target as *mut _, raw.as_ptr() as *const _, 4);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CStr;

    #[test]
    fn test_dc_decode_header_words() {
        unsafe {
            let mut buf1: *mut libc::c_char = dc_decode_header_words(
                b"=?utf-8?B?dGVzdMOkw7bDvC50eHQ=?=\x00" as *const u8 as *const libc::c_char,
            );
            assert_eq!(
                strcmp(
                    buf1,
                    b"test\xc3\xa4\xc3\xb6\xc3\xbc.txt\x00" as *const u8 as *const libc::c_char
                ),
                0
            );
            free(buf1 as *mut libc::c_void);

            buf1 =
                dc_decode_header_words(b"just ascii test\x00" as *const u8 as *const libc::c_char);
            assert_eq!(CStr::from_ptr(buf1).to_str().unwrap(), "just ascii test");
            free(buf1 as *mut libc::c_void);

            buf1 = dc_encode_header_words(b"abcdef\x00" as *const u8 as *const libc::c_char);
            assert_eq!(CStr::from_ptr(buf1).to_str().unwrap(), "abcdef");
            free(buf1 as *mut libc::c_void);

            buf1 = dc_encode_header_words(
                b"test\xc3\xa4\xc3\xb6\xc3\xbc.txt\x00" as *const u8 as *const libc::c_char,
            );
            assert_eq!(
                strncmp(buf1, b"=?utf-8\x00" as *const u8 as *const libc::c_char, 7),
                0
            );

            let buf2: *mut libc::c_char = dc_decode_header_words(buf1);
            assert_eq!(
                strcmp(
                    buf2,
                    b"test\xc3\xa4\xc3\xb6\xc3\xbc.txt\x00" as *const u8 as *const libc::c_char
                ),
                0
            );
            free(buf1 as *mut libc::c_void);
            free(buf2 as *mut libc::c_void);

            buf1 = dc_decode_header_words(
                b"=?ISO-8859-1?Q?attachment=3B=0D=0A_filename=3D?= =?ISO-8859-1?Q?=22test=E4=F6=FC=2Etxt=22=3B=0D=0A_size=3D39?=\x00" as *const u8 as *const libc::c_char
            );
            assert_eq!(
                strcmp(
                    buf1,
                    b"attachment;\r\n filename=\"test\xc3\xa4\xc3\xb6\xc3\xbc.txt\";\r\n size=39\x00" as *const u8 as *const libc::c_char,

                ),
                0
            );
            free(buf1 as *mut libc::c_void);
        }
    }

    #[test]
    fn test_dc_encode_ext_header() {
        let buf1 = dc_encode_ext_header("Björn Petersen");
        assert_eq!(&buf1, "utf-8\'\'Bj%C3%B6rn%20Petersen");
        let buf2 = dc_decode_ext_header(buf1.as_bytes());
        assert_eq!(&buf2, "Björn Petersen",);

        let buf1 = dc_decode_ext_header(b"iso-8859-1\'en\'%A3%20rates");
        assert_eq!(buf1, "£ rates",);

        let buf1 = dc_decode_ext_header(b"wrong\'format");
        assert_eq!(buf1, "wrong\'format",);

        let buf1 = dc_decode_ext_header(b"\'\'");
        assert_eq!(buf1, "\'\'");

        let buf1 = dc_decode_ext_header(b"x\'\'");
        assert_eq!(buf1, "");

        let buf1 = dc_decode_ext_header(b"\'");
        assert_eq!(buf1, "\'");

        let buf1 = dc_decode_ext_header(b"");
        assert_eq!(buf1, "");

        // regressions
        assert_eq!(
            dc_decode_ext_header(dc_encode_ext_header("%0A").as_bytes()),
            "%0A"
        );
    }

    #[test]
    fn test_dc_needs_ext_header() {
        assert_eq!(dc_needs_ext_header("Björn"), true);
        assert_eq!(dc_needs_ext_header("Bjoern"), false);
        assert_eq!(dc_needs_ext_header(""), false);
        assert_eq!(dc_needs_ext_header(" "), true);
        assert_eq!(dc_needs_ext_header("a b"), true);
    }

    #[test]
    fn test_print_hex() {
        let mut hex: [libc::c_char; 4] = [0; 4];
        let cur = b"helloworld" as *const u8 as *const libc::c_char;
        unsafe { print_hex(hex.as_mut_ptr(), cur) };
        assert_eq!(to_string(hex.as_ptr() as *const _), "=68");

        let cur = b":" as *const u8 as *const libc::c_char;
        unsafe { print_hex(hex.as_mut_ptr(), cur) };
        assert_eq!(to_string(hex.as_ptr() as *const _), "=3A");
    }

    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_ext_header_roundtrip(buf: String) {
            let encoded = dc_encode_ext_header(&buf);
            let decoded = dc_decode_ext_header(encoded.as_bytes());
            assert_eq!(buf, decoded);
        }

        #[test]
        fn test_ext_header_decode_anything(buf: Vec<u8>) {
            // make sure this never panics
            let _decoded = dc_decode_ext_header(&buf);
        }
    }
}
