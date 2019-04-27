use libc;

use crate::dc_tools::*;
use crate::types::*;
use crate::x::*;

#[inline]
unsafe fn isascii(mut _c: libc::c_int) -> libc::c_int {
    return (_c & !0x7fi32 == 0i32) as libc::c_int;
}

#[inline]
unsafe fn __isctype(mut _c: __darwin_ct_rune_t, mut _f: libc::c_ulong) -> __darwin_ct_rune_t {
    return if _c < 0i32 || _c >= 1i32 << 8i32 {
        0i32
    } else {
        (0 != _DefaultRuneLocale.__runetype[_c as usize] as libc::c_ulong & _f) as libc::c_int
    };
}

#[inline]
pub fn isalnum(mut _c: libc::c_int) -> libc::c_int {
    if _c < std::u8::MAX as libc::c_int {
        (_c as u8 as char).is_ascii_alphanumeric() as libc::c_int
    } else {
        0
    }
}

#[cfg(test)]
#[test]
fn test_isalnum() {
    assert_eq!(isalnum(0), 0);
    assert_eq!(isalnum('5' as libc::c_int), 1);
    assert_eq!(isalnum('Q' as libc::c_int), 1);
}

#[inline]
pub fn isdigit(mut _c: libc::c_int) -> libc::c_int {
    if _c < std::u8::MAX as libc::c_int {
        (_c as u8 as char).is_ascii_digit() as libc::c_int
    } else {
        0
    }
}

pub unsafe extern "C" fn dc_urlencode(mut to_encode: *const libc::c_char) -> *mut libc::c_char {
    let mut pstr: *const libc::c_char = to_encode;
    if to_encode.is_null() {
        return dc_strdup(b"\x00" as *const u8 as *const libc::c_char);
    }
    let mut buf: *mut libc::c_char = malloc(
        strlen(to_encode)
            .wrapping_mul(3i32 as libc::c_ulong)
            .wrapping_add(1i32 as libc::c_ulong),
    ) as *mut libc::c_char;
    let mut pbuf: *mut libc::c_char = buf;
    if buf.is_null() {
        exit(46i32);
    }
    while 0 != *pstr {
        if 0 != isalnum(*pstr as libc::c_int)
            || *pstr as libc::c_int == '-' as i32
            || *pstr as libc::c_int == '_' as i32
            || *pstr as libc::c_int == '.' as i32
            || *pstr as libc::c_int == '~' as i32
        {
            let fresh0 = pbuf;
            pbuf = pbuf.offset(1);
            *fresh0 = *pstr
        } else if *pstr as libc::c_int == ' ' as i32 {
            let fresh1 = pbuf;
            pbuf = pbuf.offset(1);
            *fresh1 = '+' as i32 as libc::c_char
        } else {
            let fresh2 = pbuf;
            pbuf = pbuf.offset(1);
            *fresh2 = '%' as i32 as libc::c_char;
            let fresh3 = pbuf;
            pbuf = pbuf.offset(1);
            *fresh3 = int_2_uppercase_hex((*pstr as libc::c_int >> 4i32) as libc::c_char);
            let fresh4 = pbuf;
            pbuf = pbuf.offset(1);
            *fresh4 = int_2_uppercase_hex((*pstr as libc::c_int & 15i32) as libc::c_char)
        }
        pstr = pstr.offset(1isize)
    }
    *pbuf = '\u{0}' as i32 as libc::c_char;
    return buf;
}
/* ******************************************************************************
 * URL encoding and decoding, RFC 3986
 ******************************************************************************/
unsafe fn int_2_uppercase_hex(mut code: libc::c_char) -> libc::c_char {
    static mut hex: [libc::c_char; 17] = [
        48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 65, 66, 67, 68, 69, 70, 0,
    ];
    return hex[(code as libc::c_int & 15i32) as usize];
}
pub unsafe fn dc_urldecode(mut to_decode: *const libc::c_char) -> *mut libc::c_char {
    let mut pstr: *const libc::c_char = to_decode;
    if to_decode.is_null() {
        return dc_strdup(b"\x00" as *const u8 as *const libc::c_char);
    }
    let mut buf: *mut libc::c_char =
        malloc(strlen(to_decode).wrapping_add(1i32 as libc::c_ulong)) as *mut libc::c_char;
    let mut pbuf: *mut libc::c_char = buf;
    if buf.is_null() {
        exit(50i32);
    }
    while 0 != *pstr {
        if *pstr as libc::c_int == '%' as i32 {
            if 0 != *pstr.offset(1isize) as libc::c_int && 0 != *pstr.offset(2isize) as libc::c_int
            {
                let fresh5 = pbuf;
                pbuf = pbuf.offset(1);
                *fresh5 = ((hex_2_int(*pstr.offset(1isize)) as libc::c_int) << 4i32
                    | hex_2_int(*pstr.offset(2isize)) as libc::c_int)
                    as libc::c_char;
                pstr = pstr.offset(2isize)
            }
        } else if *pstr as libc::c_int == '+' as i32 {
            let fresh6 = pbuf;
            pbuf = pbuf.offset(1);
            *fresh6 = ' ' as i32 as libc::c_char
        } else {
            let fresh7 = pbuf;
            pbuf = pbuf.offset(1);
            *fresh7 = *pstr
        }
        pstr = pstr.offset(1isize)
    }
    *pbuf = '\u{0}' as i32 as libc::c_char;
    return buf;
}
unsafe fn hex_2_int(mut ch: libc::c_char) -> libc::c_char {
    return (if 0 != isdigit(ch as libc::c_int) {
        ch as libc::c_int - '0' as i32
    } else {
        tolower(ch as libc::c_int) - 'a' as i32 + 10i32
    }) as libc::c_char;
}
pub unsafe fn dc_encode_header_words(mut to_encode: *const libc::c_char) -> *mut libc::c_char {
    let mut current_block: u64;
    let mut ret_str: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut cur: *const libc::c_char = to_encode;
    let mut mmapstr: *mut MMAPString = mmap_string_new(b"\x00" as *const u8 as *const libc::c_char);
    if to_encode.is_null() || mmapstr.is_null() {
        current_block = 8550051112593613029;
    } else {
        current_block = 4644295000439058019;
    }
    loop {
        match current_block {
            8550051112593613029 => {
                if !mmapstr.is_null() {
                    mmap_string_free(mmapstr);
                }
                break;
            }
            _ => {
                if *cur as libc::c_int != '\u{0}' as i32 {
                    let mut begin: *const libc::c_char = 0 as *const libc::c_char;
                    let mut end: *const libc::c_char = 0 as *const libc::c_char;
                    let mut do_quote: libc::c_int = 0;
                    let mut quote_words: libc::c_int = 0;
                    begin = cur;
                    end = begin;
                    quote_words = 0i32;
                    do_quote = 1i32;
                    while *cur as libc::c_int != '\u{0}' as i32 {
                        get_word(cur, &mut cur, &mut do_quote);
                        if !(0 != do_quote) {
                            break;
                        }
                        quote_words = 1i32;
                        end = cur;
                        if *cur as libc::c_int != '\u{0}' as i32 {
                            cur = cur.offset(1isize)
                        }
                    }
                    if 0 != quote_words {
                        if 0 == quote_word(
                            b"utf-8\x00" as *const u8 as *const libc::c_char,
                            mmapstr,
                            begin,
                            end.wrapping_offset_from(begin) as libc::c_long as size_t,
                        ) {
                            current_block = 8550051112593613029;
                            continue;
                        }
                        if *end as libc::c_int == ' ' as i32 || *end as libc::c_int == '\t' as i32 {
                            if mmap_string_append_c(mmapstr, *end).is_null() {
                                current_block = 8550051112593613029;
                                continue;
                            }
                            end = end.offset(1isize)
                        }
                        if *end as libc::c_int != '\u{0}' as i32 {
                            if mmap_string_append_len(
                                mmapstr,
                                end,
                                cur.wrapping_offset_from(end) as libc::c_long as size_t,
                            )
                            .is_null()
                            {
                                current_block = 8550051112593613029;
                                continue;
                            }
                        }
                    } else if mmap_string_append_len(
                        mmapstr,
                        begin,
                        cur.wrapping_offset_from(begin) as libc::c_long as size_t,
                    )
                    .is_null()
                    {
                        current_block = 8550051112593613029;
                        continue;
                    }
                    if !(*cur as libc::c_int == ' ' as i32 || *cur as libc::c_int == '\t' as i32) {
                        current_block = 4644295000439058019;
                        continue;
                    }
                    if mmap_string_append_c(mmapstr, *cur).is_null() {
                        current_block = 8550051112593613029;
                        continue;
                    }
                    cur = cur.offset(1isize);
                    current_block = 4644295000439058019;
                } else {
                    ret_str = strdup((*mmapstr).str_0);
                    current_block = 8550051112593613029;
                }
            }
        }
    }
    return ret_str;
}
unsafe fn quote_word(
    mut display_charset: *const libc::c_char,
    mut mmapstr: *mut MMAPString,
    mut word: *const libc::c_char,
    mut size: size_t,
) -> libc::c_int {
    let mut cur: *const libc::c_char = 0 as *const libc::c_char;
    let mut i: size_t = 0i32 as size_t;
    let mut hex: [libc::c_char; 4] = [0; 4];
    // let mut col: libc::c_int = 0i32;
    if mmap_string_append(mmapstr, b"=?\x00" as *const u8 as *const libc::c_char).is_null() {
        return 0i32;
    }
    if mmap_string_append(mmapstr, display_charset).is_null() {
        return 0i32;
    }
    if mmap_string_append(mmapstr, b"?Q?\x00" as *const u8 as *const libc::c_char).is_null() {
        return 0i32;
    }
    // col = (*mmapstr).len as libc::c_int;
    cur = word;
    i = 0i32 as size_t;
    while i < size {
        let mut do_quote_char: libc::c_int = 0;
        do_quote_char = 0i32;
        match *cur as libc::c_int {
            44 | 58 | 33 | 34 | 35 | 36 | 64 | 91 | 92 | 93 | 94 | 96 | 123 | 124 | 125 | 126
            | 61 | 63 | 95 => do_quote_char = 1i32,
            _ => {
                if *cur as libc::c_uchar as libc::c_int >= 128i32 {
                    do_quote_char = 1i32
                }
            }
        }
        if 0 != do_quote_char {
            snprintf(
                hex.as_mut_ptr(),
                4i32 as libc::c_ulong,
                b"=%2.2X\x00" as *const u8 as *const libc::c_char,
                *cur as libc::c_uchar as libc::c_int,
            );
            if mmap_string_append(mmapstr, hex.as_mut_ptr()).is_null() {
                return 0i32;
            }
        // col += 3i32
        } else {
            if *cur as libc::c_int == ' ' as i32 {
                if mmap_string_append_c(mmapstr, '_' as i32 as libc::c_char).is_null() {
                    return 0i32;
                }
            } else if mmap_string_append_c(mmapstr, *cur).is_null() {
                return 0i32;
            }
            // col += 3i32
        }
        cur = cur.offset(1isize);
        i = i.wrapping_add(1)
    }
    if mmap_string_append(mmapstr, b"?=\x00" as *const u8 as *const libc::c_char).is_null() {
        return 0i32;
    }
    return 1i32;
}
unsafe fn get_word(
    mut begin: *const libc::c_char,
    mut pend: *mut *const libc::c_char,
    mut pto_be_quoted: *mut libc::c_int,
) {
    let mut cur: *const libc::c_char = begin;
    while *cur as libc::c_int != ' ' as i32
        && *cur as libc::c_int != '\t' as i32
        && *cur as libc::c_int != '\u{0}' as i32
    {
        cur = cur.offset(1isize)
    }
    *pto_be_quoted = to_be_quoted(
        begin,
        cur.wrapping_offset_from(begin) as libc::c_long as size_t,
    );
    *pend = cur;
}
/* ******************************************************************************
 * Encode/decode header words, RFC 2047
 ******************************************************************************/
/* see comment below */
unsafe fn to_be_quoted(mut word: *const libc::c_char, mut size: size_t) -> libc::c_int {
    let mut cur: *const libc::c_char = word;
    let mut i: size_t = 0i32 as size_t;
    i = 0i32 as size_t;
    while i < size {
        match *cur as libc::c_int {
            44 | 58 | 33 | 34 | 35 | 36 | 64 | 91 | 92 | 93 | 94 | 96 | 123 | 124 | 125 | 126
            | 61 | 63 | 95 => return 1i32,
            _ => {
                if *cur as libc::c_uchar as libc::c_int >= 128i32 {
                    return 1i32;
                }
            }
        }
        cur = cur.offset(1isize);
        i = i.wrapping_add(1)
    }
    return 0i32;
}
pub unsafe fn dc_decode_header_words(mut in_0: *const libc::c_char) -> *mut libc::c_char {
    if in_0.is_null() {
        return 0 as *mut libc::c_char;
    }
    let mut out: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut cur_token: size_t = 0i32 as size_t;
    let mut r: libc::c_int = mailmime_encoded_phrase_parse(
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
    return out;
}
pub unsafe fn dc_encode_modified_utf7(
    mut to_encode: *const libc::c_char,
    mut change_spaces: libc::c_int,
) -> *mut libc::c_char {
    let mut utf8pos: libc::c_uint = 0i32 as libc::c_uint;
    let mut utf8total: libc::c_uint = 0i32 as libc::c_uint;
    let mut c: libc::c_uint = 0i32 as libc::c_uint;
    let mut utf7mode: libc::c_uint = 0i32 as libc::c_uint;
    let mut bitstogo: libc::c_uint = 0i32 as libc::c_uint;
    let mut utf16flag: libc::c_uint = 0i32 as libc::c_uint;
    let mut ucs4: libc::c_ulong = 0i32 as libc::c_ulong;
    let mut bitbuf: libc::c_ulong = 0i32 as libc::c_ulong;
    let mut dst: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut res: *mut libc::c_char = 0 as *mut libc::c_char;
    if to_encode.is_null() {
        return dc_strdup(b"\x00" as *const u8 as *const libc::c_char);
    }
    res = malloc(
        (2i32 as libc::c_ulong)
            .wrapping_mul(strlen(to_encode))
            .wrapping_add(1i32 as libc::c_ulong),
    ) as *mut libc::c_char;
    dst = res;
    if dst.is_null() {
        exit(51i32);
    }
    utf7mode = 0i32 as libc::c_uint;
    utf8total = 0i32 as libc::c_uint;
    bitstogo = 0i32 as libc::c_uint;
    utf8pos = 0i32 as libc::c_uint;
    loop {
        c = *to_encode as libc::c_uchar as libc::c_uint;
        if !(c != '\u{0}' as i32 as libc::c_uint) {
            break;
        }
        to_encode = to_encode.offset(1isize);
        // normal character?
        if c >= ' ' as i32 as libc::c_uint
            && c <= '~' as i32 as libc::c_uint
            && (c != '_' as i32 as libc::c_uint || 0 == change_spaces)
        {
            if 0 != utf7mode {
                if 0 != bitstogo {
                    let fresh8 = dst;
                    dst = dst.offset(1);
                    *fresh8 = base64chars[(bitbuf << (6i32 as libc::c_uint).wrapping_sub(bitstogo)
                        & 0x3fi32 as libc::c_ulong)
                        as usize]
                }
                let fresh9 = dst;
                dst = dst.offset(1);
                *fresh9 = '-' as i32 as libc::c_char;
                utf7mode = 0i32 as libc::c_uint;
                utf8pos = 0i32 as libc::c_uint;
                bitstogo = 0i32 as libc::c_uint;
                utf8total = 0i32 as libc::c_uint
            }
            if 0 != change_spaces && c == ' ' as i32 as libc::c_uint {
                let fresh10 = dst;
                dst = dst.offset(1);
                *fresh10 = '_' as i32 as libc::c_char
            } else {
                let fresh11 = dst;
                dst = dst.offset(1);
                *fresh11 = c as libc::c_char
            }
            if c == '&' as i32 as libc::c_uint {
                let fresh12 = dst;
                dst = dst.offset(1);
                *fresh12 = '-' as i32 as libc::c_char
            }
        } else {
            if 0 == utf7mode {
                let fresh13 = dst;
                dst = dst.offset(1);
                *fresh13 = '&' as i32 as libc::c_char;
                utf7mode = 1i32 as libc::c_uint
            }
            // encode ascii characters as themselves
            if c < 0x80i32 as libc::c_uint {
                ucs4 = c as libc::c_ulong
            } else if 0 != utf8total {
                ucs4 = ucs4 << 6i32 | c as libc::c_ulong & 0x3fu64;
                utf8pos = utf8pos.wrapping_add(1);
                if utf8pos < utf8total {
                    continue;
                }
            } else {
                utf8pos = 1i32 as libc::c_uint;
                if c < 0xe0i32 as libc::c_uint {
                    utf8total = 2i32 as libc::c_uint;
                    ucs4 = (c & 0x1fi32 as libc::c_uint) as libc::c_ulong
                } else if c < 0xf0i32 as libc::c_uint {
                    utf8total = 3i32 as libc::c_uint;
                    ucs4 = (c & 0xfi32 as libc::c_uint) as libc::c_ulong
                } else {
                    utf8total = 4i32 as libc::c_uint;
                    ucs4 = (c & 0x3i32 as libc::c_uint) as libc::c_ulong
                }
                continue;
            }
            utf8total = 0i32 as libc::c_uint;
            loop {
                if ucs4 >= 0x10000u64 {
                    ucs4 = ucs4.wrapping_sub(0x10000u64);
                    bitbuf = bitbuf << 16i32 | (ucs4 >> 10i32).wrapping_add(0xd800u64);
                    ucs4 = (ucs4 & 0x3ffu64).wrapping_add(0xdc00u64);
                    utf16flag = 1i32 as libc::c_uint
                } else {
                    bitbuf = bitbuf << 16i32 | ucs4;
                    utf16flag = 0i32 as libc::c_uint
                }
                bitstogo = bitstogo.wrapping_add(16i32 as libc::c_uint);
                while bitstogo >= 6i32 as libc::c_uint {
                    bitstogo = bitstogo.wrapping_sub(6i32 as libc::c_uint);
                    let fresh14 = dst;
                    dst = dst.offset(1);
                    *fresh14 = base64chars[(if 0 != bitstogo {
                        bitbuf >> bitstogo
                    } else {
                        bitbuf
                    } & 0x3fi32 as libc::c_ulong)
                        as usize]
                }
                if !(0 != utf16flag) {
                    break;
                }
            }
        }
    }
    if 0 != utf7mode {
        if 0 != bitstogo {
            let fresh15 = dst;
            dst = dst.offset(1);
            *fresh15 = base64chars[(bitbuf << (6i32 as libc::c_uint).wrapping_sub(bitstogo)
                & 0x3fi32 as libc::c_ulong) as usize]
        }
        let fresh16 = dst;
        dst = dst.offset(1);
        *fresh16 = '-' as i32 as libc::c_char
    }
    *dst = '\u{0}' as i32 as libc::c_char;
    return res;
}
/* ******************************************************************************
 * Encode/decode modified UTF-7 as needed for IMAP, see RFC 2192
 ******************************************************************************/
// UTF7 modified base64 alphabet
static mut base64chars: [libc::c_char; 65] = [
    65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88,
    89, 90, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114,
    115, 116, 117, 118, 119, 120, 121, 122, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 43, 44, 0,
];
pub unsafe fn dc_decode_modified_utf7(
    mut to_decode: *const libc::c_char,
    mut change_spaces: libc::c_int,
) -> *mut libc::c_char {
    let mut c: libc::c_uint = 0i32 as libc::c_uint;
    let mut i: libc::c_uint = 0i32 as libc::c_uint;
    let mut bitcount: libc::c_uint = 0i32 as libc::c_uint;
    let mut ucs4: libc::c_ulong = 0i32 as libc::c_ulong;
    let mut utf16: libc::c_ulong = 0i32 as libc::c_ulong;
    let mut bitbuf: libc::c_ulong = 0i32 as libc::c_ulong;
    let mut base64: [libc::c_uchar; 256] = [0; 256];
    let mut src: *const libc::c_char = 0 as *const libc::c_char;
    let mut dst: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut res: *mut libc::c_char = 0 as *mut libc::c_char;
    if to_decode.is_null() {
        return dc_strdup(b"\x00" as *const u8 as *const libc::c_char);
    }
    res = malloc(
        (4i32 as libc::c_ulong)
            .wrapping_mul(strlen(to_decode))
            .wrapping_add(1i32 as libc::c_ulong),
    ) as *mut libc::c_char;
    dst = res;
    src = to_decode;
    if dst.is_null() {
        exit(52i32);
    }
    memset(
        base64.as_mut_ptr() as *mut libc::c_void,
        64i32,
        ::std::mem::size_of::<[libc::c_uchar; 256]>() as libc::c_ulong,
    );
    i = 0i32 as libc::c_uint;
    while (i as libc::c_ulong) < ::std::mem::size_of::<[libc::c_char; 65]>() as libc::c_ulong {
        base64[base64chars[i as usize] as libc::c_uint as usize] = i as libc::c_uchar;
        i = i.wrapping_add(1)
    }
    while *src as libc::c_int != '\u{0}' as i32 {
        let fresh17 = src;
        src = src.offset(1);
        c = *fresh17 as libc::c_uint;
        if c != '&' as i32 as libc::c_uint || *src as libc::c_int == '-' as i32 {
            if 0 != change_spaces && c == '_' as i32 as libc::c_uint {
                let fresh18 = dst;
                dst = dst.offset(1);
                *fresh18 = ' ' as i32 as libc::c_char
            } else {
                let fresh19 = dst;
                dst = dst.offset(1);
                *fresh19 = c as libc::c_char
            }
            if c == '&' as i32 as libc::c_uint {
                src = src.offset(1isize)
            }
        } else {
            bitbuf = 0i32 as libc::c_ulong;
            bitcount = 0i32 as libc::c_uint;
            ucs4 = 0i32 as libc::c_ulong;
            loop {
                c = base64[*src as libc::c_uchar as usize] as libc::c_uint;
                if !(c != 64i32 as libc::c_uint) {
                    break;
                }
                src = src.offset(1isize);
                bitbuf = bitbuf << 6i32 | c as libc::c_ulong;
                bitcount = bitcount.wrapping_add(6i32 as libc::c_uint);
                // enough bits for a UTF-16 character?
                if !(bitcount >= 16i32 as libc::c_uint) {
                    continue;
                }
                bitcount = bitcount.wrapping_sub(16i32 as libc::c_uint);
                utf16 = if 0 != bitcount {
                    bitbuf >> bitcount
                } else {
                    bitbuf
                } & 0xffffi32 as libc::c_ulong;
                // convert UTF16 to UCS4
                if utf16 >= 0xd800u64 && utf16 <= 0xdbffu64 {
                    ucs4 = utf16.wrapping_sub(0xd800u64) << 10i32
                } else {
                    if utf16 >= 0xdc00u64 && utf16 <= 0xdfffu64 {
                        ucs4 = ucs4
                            .wrapping_add(utf16.wrapping_sub(0xdc00u64).wrapping_add(0x10000u64))
                    } else {
                        ucs4 = utf16
                    }
                    if ucs4 <= 0x7fu64 {
                        *dst.offset(0isize) = ucs4 as libc::c_char;
                        dst = dst.offset(1isize)
                    } else if ucs4 <= 0x7ffu64 {
                        *dst.offset(0isize) =
                            (0xc0i32 as libc::c_ulong | ucs4 >> 6i32) as libc::c_char;
                        *dst.offset(1isize) = (0x80i32 as libc::c_ulong
                            | ucs4 & 0x3fi32 as libc::c_ulong)
                            as libc::c_char;
                        dst = dst.offset(2isize)
                    } else if ucs4 <= 0xffffu64 {
                        *dst.offset(0isize) =
                            (0xe0i32 as libc::c_ulong | ucs4 >> 12i32) as libc::c_char;
                        *dst.offset(1isize) = (0x80i32 as libc::c_ulong
                            | ucs4 >> 6i32 & 0x3fi32 as libc::c_ulong)
                            as libc::c_char;
                        *dst.offset(2isize) = (0x80i32 as libc::c_ulong
                            | ucs4 & 0x3fi32 as libc::c_ulong)
                            as libc::c_char;
                        dst = dst.offset(3isize)
                    } else {
                        *dst.offset(0isize) =
                            (0xf0i32 as libc::c_ulong | ucs4 >> 18i32) as libc::c_char;
                        *dst.offset(1isize) = (0x80i32 as libc::c_ulong
                            | ucs4 >> 12i32 & 0x3fi32 as libc::c_ulong)
                            as libc::c_char;
                        *dst.offset(2isize) = (0x80i32 as libc::c_ulong
                            | ucs4 >> 6i32 & 0x3fi32 as libc::c_ulong)
                            as libc::c_char;
                        *dst.offset(3isize) = (0x80i32 as libc::c_ulong
                            | ucs4 & 0x3fi32 as libc::c_ulong)
                            as libc::c_char;
                        dst = dst.offset(4isize)
                    }
                }
            }
            if *src as libc::c_int == '-' as i32 {
                src = src.offset(1isize)
            }
        }
    }
    *dst = '\u{0}' as i32 as libc::c_char;
    return res;
}
pub unsafe fn dc_needs_ext_header(mut to_check: *const libc::c_char) -> libc::c_int {
    if !to_check.is_null() {
        while 0 != *to_check {
            if 0 == isalnum(*to_check as libc::c_int)
                && *to_check as libc::c_int != '-' as i32
                && *to_check as libc::c_int != '_' as i32
                && *to_check as libc::c_int != '.' as i32
                && *to_check as libc::c_int != '~' as i32
            {
                return 1i32;
            }
            to_check = to_check.offset(1isize)
        }
    }
    return 0i32;
}
pub unsafe fn dc_encode_ext_header(mut to_encode: *const libc::c_char) -> *mut libc::c_char {
    let mut pstr: *const libc::c_char = to_encode;
    if to_encode.is_null() {
        return dc_strdup(b"utf-8\'\'\x00" as *const u8 as *const libc::c_char);
    }
    let mut buf: *mut libc::c_char = malloc(
        strlen(b"utf-8\'\'\x00" as *const u8 as *const libc::c_char)
            .wrapping_add(strlen(to_encode).wrapping_mul(3i32 as libc::c_ulong))
            .wrapping_add(1i32 as libc::c_ulong),
    ) as *mut libc::c_char;
    if buf.is_null() {
        exit(46i32);
    }
    let mut pbuf: *mut libc::c_char = buf;
    strcpy(pbuf, b"utf-8\'\'\x00" as *const u8 as *const libc::c_char);
    pbuf = pbuf.offset(strlen(pbuf) as isize);
    while 0 != *pstr {
        if 0 != isalnum(*pstr as libc::c_int)
            || *pstr as libc::c_int == '-' as i32
            || *pstr as libc::c_int == '_' as i32
            || *pstr as libc::c_int == '.' as i32
            || *pstr as libc::c_int == '~' as i32
        {
            let fresh20 = pbuf;
            pbuf = pbuf.offset(1);
            *fresh20 = *pstr
        } else {
            let fresh21 = pbuf;
            pbuf = pbuf.offset(1);
            *fresh21 = '%' as i32 as libc::c_char;
            let fresh22 = pbuf;
            pbuf = pbuf.offset(1);
            *fresh22 = int_2_uppercase_hex((*pstr as libc::c_int >> 4i32) as libc::c_char);
            let fresh23 = pbuf;
            pbuf = pbuf.offset(1);
            *fresh23 = int_2_uppercase_hex((*pstr as libc::c_int & 15i32) as libc::c_char)
        }
        pstr = pstr.offset(1isize)
    }
    *pbuf = '\u{0}' as i32 as libc::c_char;
    return buf;
}
pub unsafe fn dc_decode_ext_header(mut to_decode: *const libc::c_char) -> *mut libc::c_char {
    let mut decoded: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut charset: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut p2: *const libc::c_char = 0 as *const libc::c_char;
    if !to_decode.is_null() {
        // get char set
        p2 = strchr(to_decode, '\'' as i32);
        if !(p2.is_null() || p2 == to_decode) {
            /*no empty charset allowed*/
            charset = dc_null_terminate(
                to_decode,
                p2.wrapping_offset_from(to_decode) as libc::c_long as libc::c_int,
            );
            p2 = p2.offset(1isize);
            // skip language
            p2 = strchr(p2, '\'' as i32);
            if !p2.is_null() {
                p2 = p2.offset(1isize);
                decoded = dc_urldecode(p2);
                if !charset.is_null()
                    && strcmp(charset, b"utf-8\x00" as *const u8 as *const libc::c_char) != 0i32
                    && strcmp(charset, b"UTF-8\x00" as *const u8 as *const libc::c_char) != 0i32
                {
                    let mut converted: *mut libc::c_char = 0 as *mut libc::c_char;
                    let mut r: libc::c_int = charconv(
                        b"utf-8\x00" as *const u8 as *const libc::c_char,
                        charset,
                        decoded,
                        strlen(decoded),
                        &mut converted,
                    );
                    if r == MAIL_CHARCONV_NO_ERROR as libc::c_int && !converted.is_null() {
                        free(decoded as *mut libc::c_void);
                        decoded = converted
                    } else {
                        free(converted as *mut libc::c_void);
                    }
                }
            }
        }
    }
    free(charset as *mut libc::c_void);
    return if !decoded.is_null() {
        decoded
    } else {
        dc_strdup(to_decode)
    };
}
