use std::borrow::Cow;
use std::ffi::{CStr, CString};
use std::fs;
use std::time::SystemTime;

use chrono::{Local, TimeZone};
use mmime::mailimf_types::*;
use rand::{thread_rng, Rng};

use crate::context::Context;
use crate::dc_array::*;
use crate::error::Error;
use crate::types::*;
use crate::x::*;

const ELLIPSE: &'static str = "[...]";

/* Some tools and enhancements to the used libraries, there should be
no references to Context and other "larger" classes here. */
// for carray etc.
/* ** library-private **********************************************************/
/* math tools */
pub fn dc_exactly_one_bit_set(v: libc::c_int) -> bool {
    0 != v && 0 == v & v - 1i32
}

/* string tools */
/* dc_strdup() returns empty string if NULL is given, never returns NULL (exits on errors) */
pub unsafe fn dc_strdup(s: *const libc::c_char) -> *mut libc::c_char {
    let ret: *mut libc::c_char;
    if !s.is_null() {
        ret = strdup(s);
        assert!(!ret.is_null());
    } else {
        ret = calloc(1, 1) as *mut libc::c_char;
        assert!(!ret.is_null());
    }

    ret
}

/* strdup(NULL) is undefined, safe_strdup_keep_null(NULL) returns NULL in this case */
pub unsafe fn dc_strdup_keep_null(s: *const libc::c_char) -> *mut libc::c_char {
    return if !s.is_null() {
        dc_strdup(s)
    } else {
        0 as *mut libc::c_char
    };
}

pub unsafe fn dc_atoi_null_is_0(s: *const libc::c_char) -> libc::c_int {
    if !s.is_null() {
        as_str(s).parse().unwrap_or_default()
    } else {
        0
    }
}

pub fn dc_atof(s: *const libc::c_char) -> libc::c_double {
    if s.is_null() {
        return 0.;
    }

    as_str(s).parse().unwrap_or_default()
}

pub unsafe fn dc_str_replace(
    haystack: *mut *mut libc::c_char,
    needle: *const libc::c_char,
    replacement: *const libc::c_char,
) -> libc::c_int {
    let mut replacements: libc::c_int = 0i32;
    let mut start_search_pos: libc::c_int = 0i32;
    let needle_len: libc::c_int;
    let replacement_len: libc::c_int;
    if haystack.is_null()
        || (*haystack).is_null()
        || needle.is_null()
        || *needle.offset(0isize) as libc::c_int == 0i32
    {
        return 0i32;
    }
    needle_len = strlen(needle) as libc::c_int;
    replacement_len = (if !replacement.is_null() {
        strlen(replacement)
    } else {
        0
    }) as libc::c_int;
    loop {
        let mut p2: *mut libc::c_char =
            strstr((*haystack).offset(start_search_pos as isize), needle);
        if p2.is_null() {
            break;
        }
        start_search_pos =
            (p2.wrapping_offset_from(*haystack) + replacement_len as isize) as libc::c_int;
        *p2 = 0i32 as libc::c_char;
        p2 = p2.offset(needle_len as isize);
        let new_string: *mut libc::c_char = dc_mprintf(
            b"%s%s%s\x00" as *const u8 as *const libc::c_char,
            *haystack,
            if !replacement.is_null() {
                replacement
            } else {
                b"\x00" as *const u8 as *const libc::c_char
            },
            p2,
        );
        free(*haystack as *mut libc::c_void);
        *haystack = new_string;
        replacements += 1
    }

    replacements
}

pub unsafe fn dc_ftoa(f: libc::c_double) -> *mut libc::c_char {
    // hack around printf(%f) that may return `,` as decimal point on mac
    let test: *mut libc::c_char = dc_mprintf(b"%f\x00" as *const u8 as *const libc::c_char, 1.2f64);
    *test.offset(2isize) = 0i32 as libc::c_char;
    let mut str: *mut libc::c_char = dc_mprintf(b"%f\x00" as *const u8 as *const libc::c_char, f);
    dc_str_replace(
        &mut str,
        test.offset(1isize),
        b".\x00" as *const u8 as *const libc::c_char,
    );
    free(test as *mut libc::c_void);

    str
}

pub unsafe fn dc_ltrim(buf: *mut libc::c_char) {
    let mut len: size_t;
    let mut cur: *const libc::c_uchar;
    if !buf.is_null() && 0 != *buf as libc::c_int {
        len = strlen(buf);
        cur = buf as *const libc::c_uchar;
        while 0 != *cur as libc::c_int && 0 != libc::isspace(*cur as libc::c_int) {
            cur = cur.offset(1isize);
            len = len.wrapping_sub(1)
        }
        if buf as *const libc::c_uchar != cur {
            memmove(
                buf as *mut libc::c_void,
                cur as *const libc::c_void,
                len.wrapping_add(1),
            );
        }
    };
}

pub unsafe fn dc_rtrim(buf: *mut libc::c_char) {
    let mut len: size_t;
    let mut cur: *mut libc::c_uchar;
    if !buf.is_null() && 0 != *buf as libc::c_int {
        len = strlen(buf);
        cur = (buf as *mut libc::c_uchar)
            .offset(len as isize)
            .offset(-1isize);
        while cur != buf as *mut libc::c_uchar && 0 != libc::isspace(*cur as libc::c_int) {
            cur = cur.offset(-1isize);
            len = len.wrapping_sub(1)
        }
        *cur.offset(
            (if 0 != libc::isspace(*cur as libc::c_int) {
                0i32
            } else {
                1i32
            }) as isize,
        ) = '\u{0}' as i32 as libc::c_uchar
    };
}

pub unsafe fn dc_trim(buf: *mut libc::c_char) {
    dc_ltrim(buf);
    dc_rtrim(buf);
}

/* the result must be free()'d */
pub unsafe fn dc_strlower(in_0: *const libc::c_char) -> *mut libc::c_char {
    to_cstring(to_string(in_0).to_lowercase())
}

pub unsafe fn dc_strlower_in_place(in_0: *mut libc::c_char) {
    let raw = to_cstring(to_string(in_0).to_lowercase());
    assert_eq!(strlen(in_0), strlen(raw));
    memcpy(in_0 as *mut _, raw as *const _, strlen(in_0));
    free(raw as *mut _);
}

pub unsafe fn dc_str_contains(
    haystack: *const libc::c_char,
    needle: *const libc::c_char,
) -> libc::c_int {
    if haystack.is_null() || needle.is_null() {
        return 0i32;
    }
    if !strstr(haystack, needle).is_null() {
        return 1i32;
    }
    let haystack_lower: *mut libc::c_char = dc_strlower(haystack);
    let needle_lower: *mut libc::c_char = dc_strlower(needle);
    let ret: libc::c_int = if !strstr(haystack_lower, needle_lower).is_null() {
        1i32
    } else {
        0i32
    };
    free(haystack_lower as *mut libc::c_void);
    free(needle_lower as *mut libc::c_void);

    ret
}

/* the result must be free()'d */
pub unsafe fn dc_null_terminate(
    in_0: *const libc::c_char,
    bytes: libc::c_int,
) -> *mut libc::c_char {
    let out: *mut libc::c_char = malloc(bytes as usize + 1) as *mut libc::c_char;
    assert!(!out.is_null());
    if !in_0.is_null() && bytes > 0i32 {
        strncpy(out, in_0, bytes as usize);
    }
    *out.offset(bytes as isize) = 0i32 as libc::c_char;

    out
}

pub unsafe fn dc_binary_to_uc_hex(buf: *const uint8_t, bytes: size_t) -> *mut libc::c_char {
    if buf.is_null() || bytes == 0 {
        return std::ptr::null_mut();
    }

    let buf = std::slice::from_raw_parts(buf, bytes);
    let raw = hex::encode_upper(buf);
    to_cstring(raw)
}

/* remove all \r characters from string */
pub unsafe fn dc_remove_cr_chars(buf: *mut libc::c_char) {
    /* search for first `\r` */
    let mut p1: *const libc::c_char = buf;
    while 0 != *p1 {
        if *p1 as libc::c_int == '\r' as i32 {
            break;
        }
        p1 = p1.offset(1isize)
    }
    /* p1 is `\r` or null-byte; start removing `\r` */
    let mut p2: *mut libc::c_char = p1 as *mut libc::c_char;
    while 0 != *p1 {
        if *p1 as libc::c_int != '\r' as i32 {
            *p2 = *p1;
            p2 = p2.offset(1isize)
        }
        p1 = p1.offset(1isize)
    }
    *p2 = 0i32 as libc::c_char;
}

pub unsafe fn dc_unify_lineends(buf: *mut libc::c_char) {
    dc_remove_cr_chars(buf);
}

/* replace bad UTF-8 characters by sequences of `_` (to avoid problems in filenames, we do not use eg. `?`) the function is useful if strings are unexpectingly encoded eg. as ISO-8859-1 */
pub unsafe fn dc_replace_bad_utf8_chars(buf: *mut libc::c_char) {
    let mut OK_TO_CONTINUE = true;
    if buf.is_null() {
        return;
    }
    /* force unsigned - otherwise the `> ' '` comparison will fail */
    let mut p1: *mut libc::c_uchar = buf as *mut libc::c_uchar;
    let p1len: libc::c_int = strlen(buf) as libc::c_int;
    let mut c: libc::c_int;
    let mut i: libc::c_int;
    let ix: libc::c_int;
    let mut n: libc::c_int;
    let mut j: libc::c_int;
    i = 0i32;
    ix = p1len;
    's_36: loop {
        if !(i < ix) {
            break;
        }
        c = *p1.offset(i as isize) as libc::c_int;
        if c > 0i32 && c <= 0x7fi32 {
            n = 0i32
        } else if c & 0xe0i32 == 0xc0i32 {
            n = 1i32
        } else if c == 0xedi32
            && i < ix - 1i32
            && *p1.offset((i + 1i32) as isize) as libc::c_int & 0xa0i32 == 0xa0i32
        {
            /* U+d800 to U+dfff */
            OK_TO_CONTINUE = false;
            break;
        } else if c & 0xf0i32 == 0xe0i32 {
            n = 2i32
        } else if c & 0xf8i32 == 0xf0i32 {
            n = 3i32
        } else {
            //else if ((c & 0xFC) == 0xF8)                          { n=4; }        /* 111110bb - not valid in https://tools.ietf.org/html/rfc3629 */
            //else if ((c & 0xFE) == 0xFC)                          { n=5; }        /* 1111110b - not valid in https://tools.ietf.org/html/rfc3629 */
            OK_TO_CONTINUE = false;
            break;
        }
        j = 0i32;
        while j < n && i < ix {
            /* n bytes matching 10bbbbbb follow ? */
            i += 1;
            if i == ix || *p1.offset(i as isize) as libc::c_int & 0xc0i32 != 0x80i32 {
                OK_TO_CONTINUE = false;
                break 's_36;
            }
            j += 1
        }
        i += 1
    }
    if OK_TO_CONTINUE == false {
            while 0 != *p1 {
                if *p1 as libc::c_int > 0x7fi32 {
                    *p1 = '_' as i32 as libc::c_uchar
                }
                p1 = p1.offset(1isize)
            }
            return;
    }
}

pub unsafe fn dc_utf8_strlen(s: *const libc::c_char) -> size_t {
    if s.is_null() {
        return 0i32 as size_t;
    }
    let mut i: size_t = 0i32 as size_t;
    let mut j: size_t = 0i32 as size_t;
    while 0 != *s.offset(i as isize) {
        if *s.offset(i as isize) as libc::c_int & 0xc0i32 != 0x80i32 {
            j = j.wrapping_add(1)
        }
        i = i.wrapping_add(1)
    }

    j
}

pub fn dc_truncate_str(buf: &str, approx_chars: usize) -> Cow<str> {
    if approx_chars > 0 && buf.len() > approx_chars + ELLIPSE.len() {
        if let Some(index) = buf[..approx_chars].rfind(|c| c == ' ' || c == '\n') {
            Cow::Owned(format!("{}{}", &buf[..index + 1], ELLIPSE))
        } else {
            Cow::Owned(format!("{}{}", &buf[..approx_chars], ELLIPSE))
        }
    } else {
        Cow::Borrowed(buf)
    }
}

#[allow(non_snake_case)]
pub unsafe fn dc_truncate_n_unwrap_str(
    buf: *mut libc::c_char,
    approx_characters: libc::c_int,
    do_unwrap: libc::c_int,
) {
    /* Function unwraps the given string and removes unnecessary whitespace.
    Function stops processing after approx_characters are processed.
    (as we're using UTF-8, for simplicity, we cut the string only at whitespaces). */
    /* a single line is truncated `...` instead of `[...]` (the former is typically also used by the UI to fit strings in a rectangle) */
    let ellipse_utf8: *const libc::c_char = if 0 != do_unwrap {
        b" ...\x00" as *const u8 as *const libc::c_char
    } else {
        b" [...]\x00" as *const u8 as *const libc::c_char
    };
    let mut lastIsCharacter: libc::c_int = 0i32;
    /* force unsigned - otherwise the `> ' '` comparison will fail */
    let mut p1: *mut libc::c_uchar = buf as *mut libc::c_uchar;
    while 0 != *p1 {
        if *p1 as libc::c_int > ' ' as i32 {
            lastIsCharacter = 1i32
        } else if 0 != lastIsCharacter {
            let used_bytes: size_t = (p1 as uintptr_t).wrapping_sub(buf as uintptr_t) as size_t;
            if dc_utf8_strnlen(buf, used_bytes) >= approx_characters as usize {
                let buf_bytes: size_t = strlen(buf);
                if buf_bytes.wrapping_sub(used_bytes) >= strlen(ellipse_utf8) {
                    strcpy(p1 as *mut libc::c_char, ellipse_utf8);
                }
                break;
            } else {
                lastIsCharacter = 0i32;
                if 0 != do_unwrap {
                    *p1 = ' ' as i32 as libc::c_uchar
                }
            }
        } else if 0 != do_unwrap {
            *p1 = '\r' as i32 as libc::c_uchar
        }
        p1 = p1.offset(1isize)
    }
    if 0 != do_unwrap {
        dc_remove_cr_chars(buf);
    };
}

unsafe fn dc_utf8_strnlen(s: *const libc::c_char, n: size_t) -> size_t {
    if s.is_null() {
        return 0i32 as size_t;
    }
    let mut i: size_t = 0i32 as size_t;
    let mut j: size_t = 0i32 as size_t;
    while i < n {
        if *s.offset(i as isize) as libc::c_int & 0xc0i32 != 0x80i32 {
            j = j.wrapping_add(1)
        }
        i = i.wrapping_add(1)
    }

    j
}

/* split string into lines*/
pub unsafe fn dc_split_into_lines(buf_terminated: *const libc::c_char) -> *mut carray {
    let lines: *mut carray = carray_new(1024i32 as libc::c_uint);
    let mut line_chars = 0;
    let mut p1: *const libc::c_char = buf_terminated;
    let mut line_start: *const libc::c_char = p1;
    let mut l_indx: libc::c_uint = 0i32 as libc::c_uint;
    while 0 != *p1 {
        if *p1 as libc::c_int == '\n' as i32 {
            carray_add(
                lines,
                strndup(line_start, line_chars) as *mut libc::c_void,
                &mut l_indx,
            );
            p1 = p1.offset(1isize);
            line_start = p1;
            line_chars = 0;
        } else {
            p1 = p1.offset(1isize);
            line_chars = line_chars.wrapping_add(1)
        }
    }
    carray_add(
        lines,
        strndup(line_start, line_chars) as *mut libc::c_void,
        &mut l_indx,
    );

    lines
}

pub unsafe fn dc_free_splitted_lines(lines: *mut carray) {
    if !lines.is_null() {
        let mut i: libc::c_int;
        let cnt: libc::c_int = carray_count(lines) as libc::c_int;
        i = 0i32;
        while i < cnt {
            free(carray_get(lines, i as libc::c_uint));
            i += 1
        }
        carray_free(lines);
    };
}

/* insert a break every n characters, the return must be free()'d */
pub unsafe fn dc_insert_breaks(
    in_0: *const libc::c_char,
    break_every: libc::c_int,
    break_chars: *const libc::c_char,
) -> *mut libc::c_char {
    if in_0.is_null() || break_every <= 0i32 || break_chars.is_null() {
        return dc_strdup(in_0);
    }
    let mut out_len = strlen(in_0);
    let mut chars_added = 0;
    let break_chars_len = strlen(break_chars);
    out_len += (out_len / break_every as usize + 1) * break_chars_len + 1;
    let out: *mut libc::c_char = malloc(out_len) as *mut libc::c_char;
    if out.is_null() {
        return 0 as *mut libc::c_char;
    }
    let mut i: *const libc::c_char = in_0;
    let mut o: *mut libc::c_char = out;
    while 0 != *i {
        let fresh1 = o;
        o = o.offset(1);
        let fresh0 = i;
        i = i.offset(1);
        *fresh1 = *fresh0;
        chars_added += 1;
        if chars_added == break_every && 0 != *i as libc::c_int {
            strcpy(o, break_chars);
            o = o.offset(break_chars_len as isize);
            chars_added = 0i32
        }
    }
    *o = 0i32 as libc::c_char;

    out
}

pub unsafe fn dc_str_from_clist(
    list: *const clist,
    delimiter: *const libc::c_char,
) -> *mut libc::c_char {
    let mut res = String::new();

    if !list.is_null() {
        let mut cur: *mut clistiter = (*list).first;
        while !cur.is_null() {
            let rfc724_mid = (if !cur.is_null() {
                (*cur).data
            } else {
                0 as *mut libc::c_void
            }) as *const libc::c_char;

            if !rfc724_mid.is_null() {
                if !res.is_empty() && !delimiter.is_null() {
                    res += as_str(delimiter);
                }
                res += as_str(rfc724_mid);
            }
            cur = if !cur.is_null() {
                (*cur).next
            } else {
                0 as *mut clistcell
            }
        }
    }

    to_cstring(res)
}

pub unsafe fn dc_str_to_clist(
    str: *const libc::c_char,
    delimiter: *const libc::c_char,
) -> *mut clist {
    let list: *mut clist = clist_new();
    assert!(!list.is_null());

    if !str.is_null() && !delimiter.is_null() && strlen(delimiter) >= 1 {
        let mut p1: *const libc::c_char = str;
        loop {
            let p2: *const libc::c_char = strstr(p1, delimiter);
            if p2.is_null() {
                clist_insert_after(list, (*list).last, strdup(p1) as *mut libc::c_void);
                break;
            } else {
                clist_insert_after(
                    list,
                    (*list).last,
                    strndup(p1, p2.wrapping_offset_from(p1) as libc::c_ulong) as *mut libc::c_void,
                );
                p1 = p2.offset(strlen(delimiter) as isize)
            }
        }
    }

    list
}

pub unsafe fn dc_str_to_color(str: *const libc::c_char) -> libc::c_int {
    let str_lower: *mut libc::c_char = dc_strlower(str);
    /* the colors must fulfill some criterions as:
    - contrast to black and to white
    - work as a text-color
    - being noticeable on a typical map
    - harmonize together while being different enough
    (therefore, we cannot just use random rgb colors :) */
    static mut COLORS: [uint32_t; 16] = [
        0xe56555i32 as uint32_t,
        0xf28c48i32 as uint32_t,
        0x8e85eei32 as uint32_t,
        0x76c84di32 as uint32_t,
        0x5bb6cci32 as uint32_t,
        0x549cddi32 as uint32_t,
        0xd25c99i32 as uint32_t,
        0xb37800i32 as uint32_t,
        0xf23030i32 as uint32_t,
        0x39b249i32 as uint32_t,
        0xbb243bi32 as uint32_t,
        0x964078i32 as uint32_t,
        0x66874fi32 as uint32_t,
        0x308ab9i32 as uint32_t,
        0x127ed0i32 as uint32_t,
        0xbe450ci32 as uint32_t,
    ];
    let mut checksum: libc::c_int = 0i32;
    let str_len: libc::c_int = strlen(str_lower) as libc::c_int;
    let mut i: libc::c_int = 0i32;
    while i < str_len {
        checksum += (i + 1i32) * *str_lower.offset(i as isize) as libc::c_int;
        checksum %= 0xffffffi32;
        i += 1
    }
    let color_index: libc::c_int = (checksum as libc::c_ulong).wrapping_rem(
        (::std::mem::size_of::<[uint32_t; 16]>() as libc::c_ulong)
            .wrapping_div(::std::mem::size_of::<uint32_t>() as libc::c_ulong),
    ) as libc::c_int;
    free(str_lower as *mut libc::c_void);

    COLORS[color_index as usize] as libc::c_int
}

/* clist tools */
/* calls free() for each item content */
pub unsafe fn clist_free_content(haystack: *const clist) {
    let mut iter: *mut clistiter = (*haystack).first;
    while !iter.is_null() {
        free((*iter).data);
        (*iter).data = 0 as *mut libc::c_void;
        iter = if !iter.is_null() {
            (*iter).next
        } else {
            0 as *mut clistcell
        }
    }
}

pub unsafe fn clist_search_string_nocase(
    haystack: *const clist,
    needle: *const libc::c_char,
) -> libc::c_int {
    let mut iter: *mut clistiter = (*haystack).first;
    while !iter.is_null() {
        if strcasecmp((*iter).data as *const libc::c_char, needle) == 0i32 {
            return 1i32;
        }
        iter = if !iter.is_null() {
            (*iter).next
        } else {
            0 as *mut clistcell
        }
    }

    0
}

/* date/time tools */
/* the result is UTC or DC_INVALID_TIMESTAMP */
pub unsafe fn dc_timestamp_from_date(date_time: *mut mailimf_date_time) -> i64 {
    let sec = (*date_time).dt_sec;
    let min = (*date_time).dt_min;
    let hour = (*date_time).dt_hour;
    let day = (*date_time).dt_day;
    let month = (*date_time).dt_month;
    let year = (*date_time).dt_year;

    let ts = chrono::NaiveDateTime::new(
        chrono::NaiveDate::from_ymd(year, month as u32, day as u32),
        chrono::NaiveTime::from_hms(hour as u32, min as u32, sec as u32),
    );

    let (zone_hour, zone_min) = if (*date_time).dt_zone >= 0 {
        ((*date_time).dt_zone / 100i32, (*date_time).dt_zone % 100i32)
    } else {
        (
            -(-(*date_time).dt_zone / 100i32),
            -(-(*date_time).dt_zone % 100i32),
        )
    };

    ts.timestamp() - (zone_hour * 3600 + zone_min * 60) as i64
}

/* ******************************************************************************
 * date/time tools
 ******************************************************************************/

pub fn dc_timestamp_to_str(wanted: i64) -> String {
    let ts = chrono::Utc.timestamp(wanted, 0);
    ts.format("%Y.%m.%d %H:%M:%S").to_string()
}

pub fn dc_gm2local_offset() -> i64 {
    let lt = Local::now();
    ((lt.offset().local_minus_utc() / (60 * 60)) * 100) as i64
}

/* timesmearing */
pub unsafe fn dc_smeared_time(context: &Context) -> i64 {
    /* function returns a corrected time(NULL) */
    let mut now = time();
    let ts = *context.last_smeared_timestamp.clone().read().unwrap();
    if ts >= now {
        now = ts + 1;
    }

    now
}

pub unsafe fn dc_create_smeared_timestamp(context: &Context) -> i64 {
    let now = time();
    let mut ret = now;

    let ts = *context.last_smeared_timestamp.clone().write().unwrap();
    if ret <= ts {
        ret = ts + 1;
        if ret - now > 5 {
            ret = now + 5
        }
    }

    ret
}

pub unsafe fn dc_create_smeared_timestamps(context: &Context, count: libc::c_int) -> i64 {
    /* get a range to timestamps that can be used uniquely */
    let now = time();
    let start = now + (if count < 5 { count } else { 5 }) as i64 - count as i64;

    let ts = *context.last_smeared_timestamp.clone().write().unwrap();
    if ts + 1 > start {
        ts + 1
    } else {
        start
    }
}

/* Message-ID tools */
pub fn dc_create_id() -> String {
    /* generate an id. the generated ID should be as short and as unique as possible:
    - short, because it may also used as part of Message-ID headers or in QR codes
    - unique as two IDs generated on two devices should not be the same. However, collisions are not world-wide but only by the few contacts.
    IDs generated by this function are 66 bit wide and are returned as 11 base64 characters.
    If possible, RNG of OpenSSL is used.

    Additional information when used as a message-id or group-id:
    - for OUTGOING messages this ID is written to the header as `Chat-Group-ID:` and is added to the message ID as Gr.<grpid>.<random>@<random>
    - for INCOMING messages, the ID is taken from the Chat-Group-ID-header or from the Message-ID in the In-Reply-To: or References:-Header
    - the group-id should be a string with the characters [a-zA-Z0-9\-_] */

    let mut rng = thread_rng();
    let buf: [uint32_t; 3] = [rng.gen(), rng.gen(), rng.gen()];

    encode_66bits_as_base64(buf[0usize], buf[1usize], buf[2usize])
}

/// Encode 66 bits as a base64 string.
/// This is useful for ID generating with short strings as we save 5 character
/// in each id compared to 64 bit hex encoding. For a typical group ID, these
/// are 10 characters (grpid+msgid):
///    hex:    64 bit, 4 bits/character, length = 64/4 = 16 characters
///    base64: 64 bit, 6 bits/character, length = 64/6 = 11 characters (plus 2 additional bits)
/// Only the lower 2 bits of `fill` are used.
fn encode_66bits_as_base64(v1: u32, v2: u32, fill: u32) -> String {
    use byteorder::{BigEndian, WriteBytesExt};

    let mut wrapped_writer = Vec::new();
    {
        let mut enc = base64::write::EncoderWriter::new(&mut wrapped_writer, base64::URL_SAFE);
        enc.write_u32::<BigEndian>(v1).unwrap();
        enc.write_u32::<BigEndian>(v2).unwrap();
        enc.write_u8(((fill & 0x3) as u8) << 6).unwrap();
        enc.finish().unwrap();
    }
    assert_eq!(wrapped_writer.pop(), Some('A' as u8)); // Remove last "A"
    String::from_utf8(wrapped_writer).unwrap()
}

pub unsafe fn dc_create_incoming_rfc724_mid(
    message_timestamp: i64,
    contact_id_from: uint32_t,
    contact_ids_to: *mut dc_array_t,
) -> *mut libc::c_char {
    if contact_ids_to.is_null() || dc_array_get_cnt(contact_ids_to) == 0 {
        return 0 as *mut libc::c_char;
    }
    /* find out the largest receiver ID (we could also take the smallest, but it should be unique) */
    let mut i: size_t = 0i32 as size_t;
    let icnt: size_t = dc_array_get_cnt(contact_ids_to);
    let mut largest_id_to: uint32_t = 0i32 as uint32_t;
    while i < icnt {
        let cur_id: uint32_t = dc_array_get_id(contact_ids_to, i);
        if cur_id > largest_id_to {
            largest_id_to = cur_id
        }
        i = i.wrapping_add(1)
    }

    dc_mprintf(
        b"%lu-%lu-%lu@stub\x00" as *const u8 as *const libc::c_char,
        message_timestamp as libc::c_ulong,
        contact_id_from as libc::c_ulong,
        largest_id_to as libc::c_ulong,
    )
}

pub unsafe fn dc_create_outgoing_rfc724_mid(
    grpid: *const libc::c_char,
    from_addr: *const libc::c_char,
) -> *mut libc::c_char {
    /* Function generates a Message-ID that can be used for a new outgoing message.
    - this function is called for all outgoing messages.
    - the message ID should be globally unique
    - do not add a counter or any private data as as this may give unneeded information to the receiver	*/
    let mut rand1: *mut libc::c_char = 0 as *mut libc::c_char;
    let rand2: *mut libc::c_char = to_cstring(dc_create_id());
    let ret: *mut libc::c_char;
    let mut at_hostname: *const libc::c_char = strchr(from_addr, '@' as i32);
    if at_hostname.is_null() {
        at_hostname = b"@nohost\x00" as *const u8 as *const libc::c_char
    }
    if !grpid.is_null() {
        ret = dc_mprintf(
            b"Gr.%s.%s%s\x00" as *const u8 as *const libc::c_char,
            grpid,
            rand2,
            at_hostname,
        )
    } else {
        rand1 = to_cstring(dc_create_id());
        ret = dc_mprintf(
            b"Mr.%s.%s%s\x00" as *const u8 as *const libc::c_char,
            rand1,
            rand2,
            at_hostname,
        )
    }
    free(rand1 as *mut libc::c_void);
    free(rand2 as *mut libc::c_void);

    ret
}

pub unsafe fn dc_extract_grpid_from_rfc724_mid(mid: *const libc::c_char) -> *mut libc::c_char {
    /* extract our group ID from Message-IDs as `Gr.12345678901.morerandom@domain.de`; "12345678901" is the wanted ID in this example. */
    let mut success: libc::c_int = 0i32;
    let mut grpid: *mut libc::c_char = 0 as *mut libc::c_char;
    let p1: *mut libc::c_char;
    let grpid_len: libc::c_int;
    if !(mid.is_null()
        || strlen(mid) < 8
        || *mid.offset(0isize) as libc::c_int != 'G' as i32
        || *mid.offset(1isize) as libc::c_int != 'r' as i32
        || *mid.offset(2isize) as libc::c_int != '.' as i32)
    {
        grpid = dc_strdup(&*mid.offset(3isize));
        p1 = strchr(grpid, '.' as i32);
        if !p1.is_null() {
            *p1 = 0i32 as libc::c_char;
            grpid_len = strlen(grpid) as libc::c_int;
            if !(grpid_len != 11i32 && grpid_len != 16i32) {
                /* strict length comparison, the 'Gr.' magic is weak enough */
                success = 1i32
            }
        }
    }
    if success == 0i32 {
        free(grpid as *mut libc::c_void);
        grpid = 0 as *mut libc::c_char
    }
    return if 0 != success {
        grpid
    } else {
        0 as *mut libc::c_char
    };
}

pub unsafe fn dc_extract_grpid_from_rfc724_mid_list(list: *const clist) -> *mut libc::c_char {
    if !list.is_null() {
        let mut cur: *mut clistiter = (*list).first;
        while !cur.is_null() {
            let mid: *const libc::c_char = (if !cur.is_null() {
                (*cur).data
            } else {
                0 as *mut libc::c_void
            }) as *const libc::c_char;
            let grpid: *mut libc::c_char = dc_extract_grpid_from_rfc724_mid(mid);
            if !grpid.is_null() {
                return grpid;
            }
            cur = if !cur.is_null() {
                (*cur).next
            } else {
                0 as *mut clistcell
            }
        }
    }

    0 as *mut libc::c_char
}

/* file tools */
#[allow(non_snake_case)]
pub unsafe fn dc_ensure_no_slash(pathNfilename: *mut libc::c_char) {
    let path_len = strlen(pathNfilename);
    if path_len > 0 {
        if *pathNfilename.offset((path_len - 1) as isize) as libc::c_int == '/' as i32
            || *pathNfilename.offset((path_len - 1) as isize) as libc::c_int == '\\' as i32
        {
            *pathNfilename.offset((path_len - 1) as isize) = 0 as libc::c_char
        }
    };
}

pub fn dc_ensure_no_slash_safe(path: &str) -> &str {
    if path.ends_with('/') || path.ends_with('\\') {
        return &path[..path.len() - 1];
    }
    path
}

pub unsafe fn dc_validate_filename(filename: *mut libc::c_char) {
    /* function modifies the given buffer and replaces all characters not valid in filenames by a "-" */
    let mut p1: *mut libc::c_char = filename;
    while 0 != *p1 {
        if *p1 as libc::c_int == '/' as i32
            || *p1 as libc::c_int == '\\' as i32
            || *p1 as libc::c_int == ':' as i32
        {
            *p1 = '-' as i32 as libc::c_char
        }
        p1 = p1.offset(1isize)
    }
}

#[allow(non_snake_case)]
pub unsafe fn dc_get_filename(pathNfilename: *const libc::c_char) -> *mut libc::c_char {
    let mut p: *const libc::c_char = strrchr(pathNfilename, '/' as i32);
    if p.is_null() {
        p = strrchr(pathNfilename, '\\' as i32)
    }
    if !p.is_null() {
        p = p.offset(1isize);
        return dc_strdup(p);
    } else {
        return dc_strdup(pathNfilename);
    };
}

// the case of the suffix is preserved
#[allow(non_snake_case)]
pub unsafe fn dc_split_filename(
    pathNfilename: *const libc::c_char,
    ret_basename: *mut *mut libc::c_char,
    ret_all_suffixes_incl_dot: *mut *mut libc::c_char,
) {
    /* splits a filename into basename and all suffixes, eg. "/path/foo.tar.gz" is split into "foo.tar" and ".gz",
    (we use the _last_ dot which allows the usage inside the filename which are very usual;
    maybe the detection could be more intelligent, however, for the moment, it is just file)
    - if there is no suffix, the returned suffix string is empty, eg. "/path/foobar" is split into "foobar" and ""
    - the case of the returned suffix is preserved; this is to allow reconstruction of (similar) names */
    let basename: *mut libc::c_char = dc_get_filename(pathNfilename);
    let suffix: *mut libc::c_char;
    let p1: *mut libc::c_char = strrchr(basename, '.' as i32);
    if !p1.is_null() {
        suffix = dc_strdup(p1);
        *p1 = 0i32 as libc::c_char
    } else {
        suffix = dc_strdup(0 as *const libc::c_char)
    }
    if !ret_basename.is_null() {
        *ret_basename = basename
    } else {
        free(basename as *mut libc::c_void);
    }
    if !ret_all_suffixes_incl_dot.is_null() {
        *ret_all_suffixes_incl_dot = suffix
    } else {
        free(suffix as *mut libc::c_void);
    };
}

// the returned suffix is lower-case
#[allow(non_snake_case)]
pub unsafe fn dc_get_filesuffix_lc(pathNfilename: *const libc::c_char) -> *mut libc::c_char {
    if !pathNfilename.is_null() {
        let mut p: *const libc::c_char = strrchr(pathNfilename, '.' as i32);
        if !p.is_null() {
            p = p.offset(1isize);
            return dc_strlower(p);
        }
    }

    0 as *mut libc::c_char
}

pub unsafe fn dc_get_filemeta(
    buf_start: *const libc::c_void,
    buf_bytes: size_t,
    ret_width: *mut uint32_t,
    ret_height: *mut uint32_t,
) -> libc::c_int {
    /* Strategy:
    reading GIF dimensions requires the first 10 bytes of the file
    reading PNG dimensions requires the first 24 bytes of the file
    reading JPEG dimensions requires scanning through jpeg chunks
    In all formats, the file is at least 24 bytes big, so we'll read that always
    inspired by http://www.cplusplus.com/forum/beginner/45217/ */
    let buf: *const libc::c_uchar = buf_start as *const libc::c_uchar;
    if buf_bytes < 24 {
        return 0i32;
    }
    if *buf.offset(0isize) as libc::c_int == 0xffi32
        && *buf.offset(1isize) as libc::c_int == 0xd8i32
        && *buf.offset(2isize) as libc::c_int == 0xffi32
    {
        let mut pos = 2;
        while *buf.offset(pos as isize) as libc::c_int == 0xffi32 {
            if *buf.offset((pos + 1) as isize) as libc::c_int == 0xc0i32
                || *buf.offset((pos + 1) as isize) as libc::c_int == 0xc1i32
                || *buf.offset((pos + 1) as isize) as libc::c_int == 0xc2i32
                || *buf.offset((pos + 1) as isize) as libc::c_int == 0xc3i32
                || *buf.offset((pos + 1) as isize) as libc::c_int == 0xc9i32
                || *buf.offset((pos + 1) as isize) as libc::c_int == 0xcai32
                || *buf.offset((pos + 1) as isize) as libc::c_int == 0xcbi32
            {
                *ret_height = (((*buf.offset((pos + 5) as isize) as libc::c_int) << 8i32)
                    + *buf.offset((pos + 6) as isize) as libc::c_int)
                    as uint32_t;
                *ret_width = (((*buf.offset((pos + 7) as isize) as libc::c_int) << 8i32)
                    + *buf.offset((pos + 8) as isize) as libc::c_int)
                    as uint32_t;
                return 1i32;
            }
            pos += 2
                + ((*buf.offset((pos + 2) as isize) as libc::c_int) << 8)
                + *buf.offset((pos + 3) as isize) as libc::c_int;
            if (pos + 12) > buf_bytes as libc::c_int {
                break;
            }
        }
    }
    if *buf.offset(0isize) as libc::c_int == 'G' as i32
        && *buf.offset(1isize) as libc::c_int == 'I' as i32
        && *buf.offset(2isize) as libc::c_int == 'F' as i32
    {
        *ret_width = (*buf.offset(6isize) as libc::c_int
            + ((*buf.offset(7isize) as libc::c_int) << 8i32)) as uint32_t;
        *ret_height = (*buf.offset(8isize) as libc::c_int
            + ((*buf.offset(9isize) as libc::c_int) << 8i32)) as uint32_t;
        return 1i32;
    }
    if *buf.offset(0isize) as libc::c_int == 0x89i32
        && *buf.offset(1isize) as libc::c_int == 'P' as i32
        && *buf.offset(2isize) as libc::c_int == 'N' as i32
        && *buf.offset(3isize) as libc::c_int == 'G' as i32
        && *buf.offset(4isize) as libc::c_int == 0xdi32
        && *buf.offset(5isize) as libc::c_int == 0xai32
        && *buf.offset(6isize) as libc::c_int == 0x1ai32
        && *buf.offset(7isize) as libc::c_int == 0xai32
        && *buf.offset(12isize) as libc::c_int == 'I' as i32
        && *buf.offset(13isize) as libc::c_int == 'H' as i32
        && *buf.offset(14isize) as libc::c_int == 'D' as i32
        && *buf.offset(15isize) as libc::c_int == 'R' as i32
    {
        *ret_width = (((*buf.offset(16isize) as libc::c_int) << 24i32)
            + ((*buf.offset(17isize) as libc::c_int) << 16i32)
            + ((*buf.offset(18isize) as libc::c_int) << 8i32)
            + ((*buf.offset(19isize) as libc::c_int) << 0i32)) as uint32_t;
        *ret_height = (((*buf.offset(20isize) as libc::c_int) << 24i32)
            + ((*buf.offset(21isize) as libc::c_int) << 16i32)
            + ((*buf.offset(22isize) as libc::c_int) << 8i32)
            + ((*buf.offset(23isize) as libc::c_int) << 0i32)) as uint32_t;
        return 1i32;
    }

    0
}

#[allow(non_snake_case)]
pub unsafe fn dc_get_abs_path(
    context: &Context,
    pathNfilename: *const libc::c_char,
) -> *mut libc::c_char {
    if pathNfilename.is_null() {
        return 0 as *mut libc::c_char;
    }

    let starts = strncmp(
        pathNfilename,
        b"$BLOBDIR\x00" as *const u8 as *const libc::c_char,
        8,
    ) == 0i32;

    if starts && !context.has_blobdir() {
        return 0 as *mut libc::c_char;
    }

    let mut pathNfilename_abs: *mut libc::c_char = dc_strdup(pathNfilename);
    if starts && context.has_blobdir() {
        dc_str_replace(
            &mut pathNfilename_abs,
            b"$BLOBDIR\x00" as *const u8 as *const libc::c_char,
            context.get_blobdir(),
        );
    }
    pathNfilename_abs
}

#[allow(non_snake_case)]
pub unsafe fn dc_file_exist(context: &Context, pathNfilename: *const libc::c_char) -> libc::c_int {
    let pathNfilename_abs = dc_get_abs_path(context, pathNfilename);
    if pathNfilename_abs.is_null() {
        return 0;
    }

    let exist = {
        let p = std::path::Path::new(as_str(pathNfilename_abs));
        p.exists()
    };

    free(pathNfilename_abs as *mut libc::c_void);
    exist as libc::c_int
}

#[allow(non_snake_case)]
pub unsafe fn dc_get_filebytes(context: &Context, pathNfilename: *const libc::c_char) -> uint64_t {
    let pathNfilename_abs = dc_get_abs_path(context, pathNfilename);
    if pathNfilename_abs.is_null() {
        return 0;
    }

    let p = std::ffi::CStr::from_ptr(pathNfilename_abs)
        .to_str()
        .unwrap();
    let filebytes = match fs::metadata(p) {
        Ok(meta) => meta.len(),
        Err(_err) => {
            return 0;
        }
    };

    free(pathNfilename_abs as *mut libc::c_void);
    filebytes as uint64_t
}

#[allow(non_snake_case)]
pub unsafe fn dc_delete_file(context: &Context, pathNfilename: *const libc::c_char) -> libc::c_int {
    let mut success: libc::c_int = 0i32;
    let pathNfilename_abs = dc_get_abs_path(context, pathNfilename);
    if pathNfilename_abs.is_null() {
        return 0;
    }
    let p = std::path::Path::new(
        std::ffi::CStr::from_ptr(pathNfilename_abs)
            .to_str()
            .unwrap(),
    );

    let res = if p.is_file() {
        fs::remove_file(p)
    } else {
        fs::remove_dir_all(p)
    };

    match res {
        Ok(_) => {
            success = 1;
        }
        Err(_err) => {
            warn!(context, 0, "Cannot delete \"{}\".", as_str(pathNfilename),);
        }
    }

    free(pathNfilename_abs as *mut libc::c_void);
    success
}

#[allow(non_snake_case)]
pub unsafe fn dc_copy_file(
    context: &Context,
    src: *const libc::c_char,
    dest: *const libc::c_char,
) -> libc::c_int {
    let mut success = 0;

    let src_abs = dc_get_abs_path(context, src);
    let dest_abs = dc_get_abs_path(context, dest);

    if src_abs.is_null() || dest_abs.is_null() {
        return 0;
    }

    let src_p = std::ffi::CStr::from_ptr(src_abs).to_str().unwrap();
    let dest_p = std::ffi::CStr::from_ptr(dest_abs).to_str().unwrap();

    match fs::copy(src_p, dest_p) {
        Ok(_) => {
            success = 1;
        }
        Err(_) => {
            error!(context, 0, "Cannot copy \"{}\" to \"{}\".", src_p, dest_p,);
        }
    }

    free(src_abs as *mut libc::c_void);
    free(dest_abs as *mut libc::c_void);
    success
}

#[allow(non_snake_case)]
pub unsafe fn dc_create_folder(
    context: &Context,
    pathNfilename: *const libc::c_char,
) -> libc::c_int {
    let mut success = 0;
    let pathNfilename_abs = dc_get_abs_path(context, pathNfilename);
    {
        let p = std::path::Path::new(as_str(pathNfilename_abs));
        if !p.exists() {
            match fs::create_dir_all(p) {
                Ok(_) => {
                    success = 1;
                }
                Err(_err) => {
                    warn!(
                        context,
                        0,
                        "Cannot create directory \"{}\".",
                        as_str(pathNfilename),
                    );
                }
            }
        } else {
            success = 1;
        }
    }

    free(pathNfilename_abs as *mut libc::c_void);
    success
}

#[allow(non_snake_case)]
pub unsafe fn dc_write_file(
    context: &Context,
    pathNfilename: *const libc::c_char,
    buf: *const libc::c_void,
    buf_bytes: size_t,
) -> libc::c_int {
    let bytes = std::slice::from_raw_parts(buf as *const u8, buf_bytes);

    dc_write_file_safe(context, as_str(pathNfilename), bytes) as libc::c_int
}

#[allow(non_snake_case)]
pub fn dc_write_file_safe(context: &Context, pathNfilename: impl AsRef<str>, buf: &[u8]) -> bool {
    let pathNfilename_abs = unsafe {
        let n = to_cstring(pathNfilename.as_ref());
        let res = dc_get_abs_path(context, n);
        free(n as *mut _);
        res
    };
    if pathNfilename_abs.is_null() {
        return false;
    }

    let p = as_str(pathNfilename_abs);

    let success = if let Err(_err) = fs::write(p, buf) {
        warn!(
            context,
            0,
            "Cannot write {} bytes to \"{}\".",
            buf.len(),
            pathNfilename.as_ref(),
        );
        false
    } else {
        true
    };

    unsafe { free(pathNfilename_abs as *mut libc::c_void) };
    success
}

#[allow(non_snake_case)]
pub unsafe fn dc_read_file(
    context: &Context,
    pathNfilename: *const libc::c_char,
    buf: *mut *mut libc::c_void,
    buf_bytes: *mut size_t,
) -> libc::c_int {
    if pathNfilename.is_null() {
        return 0;
    }
    if let Some(mut bytes) = dc_read_file_safe(context, as_str(pathNfilename)) {
        *buf = &mut bytes[..] as *mut _ as *mut libc::c_void;
        *buf_bytes = bytes.len();
        std::mem::forget(bytes);
        1
    } else {
        0
    }
}

#[allow(non_snake_case)]
pub fn dc_read_file_safe(context: &Context, pathNfilename: impl AsRef<str>) -> Option<Vec<u8>> {
    let pathNfilename_abs = unsafe {
        let n = to_cstring(pathNfilename.as_ref());
        let p = dc_get_abs_path(context, n);
        free(n as *mut _);
        p
    };

    if pathNfilename_abs.is_null() {
        return None;
    }

    let p = as_str(pathNfilename_abs);
    let res = match fs::read(p) {
        Ok(bytes) => Some(bytes),
        Err(_err) => {
            warn!(
                context,
                0,
                "Cannot read \"{}\" or file is empty.",
                pathNfilename.as_ref(),
            );
            None
        }
    };

    unsafe { free(pathNfilename_abs as *mut libc::c_void) };

    res
}

#[allow(non_snake_case)]
pub unsafe fn dc_get_fine_pathNfilename(
    context: &Context,
    pathNfolder: *const libc::c_char,
    desired_filenameNsuffix__: *const libc::c_char,
) -> *mut libc::c_char {
    let mut ret: *mut libc::c_char = 0 as *mut libc::c_char;
    let pathNfolder_wo_slash: *mut libc::c_char;
    let filenameNsuffix: *mut libc::c_char;
    let mut basename: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut dotNSuffix: *mut libc::c_char = 0 as *mut libc::c_char;
    let now = time();
    let mut i: libc::c_int = 0i32;
    pathNfolder_wo_slash = dc_strdup(pathNfolder);
    dc_ensure_no_slash(pathNfolder_wo_slash);
    filenameNsuffix = dc_strdup(desired_filenameNsuffix__);
    dc_validate_filename(filenameNsuffix);
    dc_split_filename(filenameNsuffix, &mut basename, &mut dotNSuffix);
    while i < 1000i32 {
        /*no deadlocks, please*/
        if 0 != i {
            let idx = if i < 100 { i as i64 } else { now + i as i64 };
            ret = dc_mprintf(
                b"%s/%s-%lu%s\x00" as *const u8 as *const libc::c_char,
                pathNfolder_wo_slash,
                basename,
                idx as libc::c_ulong,
                dotNSuffix,
            )
        } else {
            ret = dc_mprintf(
                b"%s/%s%s\x00" as *const u8 as *const libc::c_char,
                pathNfolder_wo_slash,
                basename,
                dotNSuffix,
            )
        }
        if 0 == dc_file_exist(context, ret) {
            /* fine filename found */
            break;
        } else {
            free(ret as *mut libc::c_void);
            ret = 0 as *mut libc::c_char;
            i += 1
        }
    }
    free(filenameNsuffix as *mut libc::c_void);
    free(basename as *mut libc::c_void);
    free(dotNSuffix as *mut libc::c_void);
    free(pathNfolder_wo_slash as *mut libc::c_void);

    ret
}

pub unsafe fn dc_is_blobdir_path(context: &Context, path: *const libc::c_char) -> bool {
    return strncmp(path, context.get_blobdir(), strlen(context.get_blobdir())) == 0
        || strncmp(path, b"$BLOBDIR\x00" as *const u8 as *const libc::c_char, 8) == 0;
}

pub unsafe fn dc_make_rel_path(context: &Context, path: *mut *mut libc::c_char) {
    if path.is_null() || (*path).is_null() {
        return;
    }
    if strncmp(*path, context.get_blobdir(), strlen(context.get_blobdir())) == 0i32 {
        dc_str_replace(
            path,
            context.get_blobdir(),
            b"$BLOBDIR\x00" as *const u8 as *const libc::c_char,
        );
    };
}

pub unsafe fn dc_make_rel_and_copy(context: &Context, path: *mut *mut libc::c_char) -> bool {
    let mut success = false;
    let mut filename: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut blobdir_path: *mut libc::c_char = 0 as *mut libc::c_char;
    if !(path.is_null() || (*path).is_null()) {
        if dc_is_blobdir_path(context, *path) {
            dc_make_rel_path(context, path);
            success = true;
        } else {
            filename = dc_get_filename(*path);
            if !(filename.is_null()
                || {
                    blobdir_path = dc_get_fine_pathNfilename(
                        context,
                        b"$BLOBDIR\x00" as *const u8 as *const libc::c_char,
                        filename,
                    );
                    blobdir_path.is_null()
                }
                || 0 == dc_copy_file(context, *path, blobdir_path))
            {
                free(*path as *mut libc::c_void);
                *path = blobdir_path;
                blobdir_path = 0 as *mut libc::c_char;
                dc_make_rel_path(context, path);
                success = true;
            }
        }
    }
    free(blobdir_path as *mut libc::c_void);
    free(filename as *mut libc::c_void);

    success
}

/// Error type for the [OsStrExt] trait
#[derive(Debug, Fail, PartialEq)]
pub enum CStringError {
    /// The string contains an interior null byte
    #[fail(display = "String contains an interior null byte")]
    InteriorNullByte,
    /// The string is not valid Unicode
    #[fail(display = "String is not valid unicode")]
    NotUnicode,
}

/// Extra convenience methods on [std::ffi::OsStr] to work with `*libc::c_char`.
///
/// The primary function of this trait is to more easily convert
/// [OsStr], [OsString] or [Path] into pointers to C strings.  This always
/// allocates a new string since it is very common for the source
/// string not to have the required terminal null byte.
///
/// It is implemented for `AsRef<std::ffi::OsStr>>` trait, which
/// allows any type which implements this trait to transparently use
/// this.  This is how the conversion for [Path] works.
///
/// [OsStr]: std::ffi::OsStr
/// [OsString]: std::ffi::OsString
/// [Path]: std::path::Path
///
/// # Example
///
/// ```
/// use deltachat::dc_tools::{dc_strdup, OsStrExt};
/// let path = std::path::Path::new("/some/path");
/// let path_c = path.to_c_string().unwrap();
/// unsafe {
///     let mut c_ptr: *mut libc::c_char = dc_strdup(path_c.as_ptr());
/// }
/// ```
pub trait OsStrExt {
    /// Convert a  [std::ffi::OsStr] to an [std::ffi::CString]
    ///
    /// This is useful to convert e.g. a [std::path::Path] to
    /// [*libc::c_char] by using
    /// [Path::as_os_str()](std::path::Path::as_os_str) and
    /// [CStr::as_ptr()](std::ffi::CStr::as_ptr).
    ///
    /// This returns [CString] and not [&CStr] because not all [OsStr]
    /// slices end with a null byte, particularly those coming from
    /// [Path] do not have a null byte and having to handle this as
    /// the caller would defeat the point of this function.
    ///
    /// On Windows this requires that the [OsStr] contains valid
    /// unicode, which should normally be the case for a [Path].
    ///
    /// [CString]: std::ffi::CString
    /// [CStr]: std::ffi::CStr
    /// [OsStr]: std::ffi::OsStr
    /// [Path]: std::path::Path
    ///
    /// # Errors
    ///
    /// Since a C `*char` is terminated by a NULL byte this conversion
    /// will fail, when the [OsStr] has an interior null byte.  The
    /// function will return
    /// `[Err]([CStringError::InteriorNullByte])`.  When converting
    /// from a [Path] it should be safe to
    /// [`.unwrap()`](std::result::Result::unwrap) this anyway since a
    /// [Path] should not contain interior null bytes.
    ///
    /// On windows when the string contains invalid Unicode
    /// `[Err]([CStringError::NotUnicode])` is returned.
    fn to_c_string(&self) -> Result<CString, CStringError>;
}

impl<T: AsRef<std::ffi::OsStr>> OsStrExt for T {
    #[cfg(not(target_os = "windows"))]
    fn to_c_string(&self) -> Result<CString, CStringError> {
        use std::os::unix::ffi::OsStrExt;
        CString::new(self.as_ref().as_bytes()).map_err(|err| match err {
            std::ffi::NulError { .. } => CStringError::InteriorNullByte,
        })
    }

    #[cfg(target_os = "windows")]
    fn to_c_string(&self) -> Result<CString, CStringError> {
        os_str_to_c_string_unicode(&self)
    }
}

// Implementation for os_str_to_c_string on windows.
#[allow(dead_code)]
fn os_str_to_c_string_unicode(
    os_str: &dyn AsRef<std::ffi::OsStr>,
) -> Result<CString, CStringError> {
    match os_str.as_ref().to_str() {
        Some(val) => CString::new(val.as_bytes()).map_err(|err| match err {
            std::ffi::NulError { .. } => CStringError::InteriorNullByte,
        }),
        None => Err(CStringError::NotUnicode),
    }
}

/// Needs to free the result after use!
pub unsafe fn to_cstring<S: AsRef<str>>(s: S) -> *mut libc::c_char {
    let cstr = CString::new(s.as_ref()).expect("invalid string converted");
    dc_strdup(cstr.as_ref().as_ptr())
}

pub fn to_string(s: *const libc::c_char) -> String {
    if s.is_null() {
        return "".into();
    }

    let cstr = unsafe { CStr::from_ptr(s) };

    cstr.to_str().map(|s| s.to_string()).unwrap_or_else(|err| {
        panic!(
            "Non utf8 string: '{:?}' ({:?})",
            cstr.to_string_lossy(),
            err
        );
    })
}

pub fn to_string_lossy(s: *const libc::c_char) -> String {
    if s.is_null() {
        return "".into();
    }

    let cstr = unsafe { CStr::from_ptr(s) };

    cstr.to_str()
        .map(|s| s.to_string())
        .unwrap_or_else(|_| cstr.to_string_lossy().to_string())
}

pub fn as_str<'a>(s: *const libc::c_char) -> &'a str {
    as_str_safe(s).unwrap_or_else(|err| panic!("{}", err))
}

pub fn as_str_safe<'a>(s: *const libc::c_char) -> Result<&'a str, Error> {
    assert!(!s.is_null(), "cannot be used on null pointers");

    let cstr = unsafe { CStr::from_ptr(s) };

    cstr.to_str()
        .map_err(|err| format_err!("Non utf8 string: '{:?}' ({:?})", cstr.to_bytes(), err))
}

/// Convert a C `*char` pointer to a [std::path::Path] slice.
///
/// This converts a `*libc::c_char` pointer to a [Path] slice.  This
/// essentially has to convert the pointer to [std::ffi::OsStr] to do
/// so and thus is the inverse of [OsStrExt::to_c_string].  Just like
/// [OsStrExt::to_c_string] requires valid Unicode on Windows, this
/// requires that the pointer contains valid UTF-8 on Windows.
///
/// Because this returns a reference the [Path] silce can not outlive
/// the original pointer.
///
/// [Path]: std::path::Path
#[cfg(not(target_os = "windows"))]
pub fn as_path<'a>(s: *const libc::c_char) -> &'a std::path::Path {
    assert!(!s.is_null(), "cannot be used on null pointers");
    use std::os::unix::ffi::OsStrExt;
    unsafe {
        let c_str = std::ffi::CStr::from_ptr(s).to_bytes();
        let os_str = std::ffi::OsStr::from_bytes(c_str);
        std::path::Path::new(os_str)
    }
}

// as_path() implementation for windows, documented above.
#[cfg(target_os = "windows")]
pub fn as_path<'a>(s: *const libc::c_char) -> &'a std::path::Path {
    as_path_unicode(s)
}

// Implementation for as_path() on Windows.
//
// Having this as a separate function means it can be tested on unix
// too.
#[allow(dead_code)]
fn as_path_unicode<'a>(s: *const libc::c_char) -> &'a std::path::Path {
    assert!(!s.is_null(), "cannot be used on null pointers");
    std::path::Path::new(as_str(s))
}

pub fn time() -> i64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CStr;

    #[test]
    fn test_dc_ltrim() {
        unsafe {
            let html: *const libc::c_char =
                b"\r\r\nline1<br>\r\n\r\n\r\rline2\n\r\x00" as *const u8 as *const libc::c_char;
            let out: *mut libc::c_char = strndup(html, strlen(html) as libc::c_ulong);

            dc_ltrim(out);

            assert_eq!(
                CStr::from_ptr(out as *const libc::c_char).to_str().unwrap(),
                "line1<br>\r\n\r\n\r\rline2\n\r"
            );
        }
    }

    #[test]
    fn test_dc_rtrim() {
        unsafe {
            let html: *const libc::c_char =
                b"\r\r\nline1<br>\r\n\r\n\r\rline2\n\r\x00" as *const u8 as *const libc::c_char;
            let out: *mut libc::c_char = strndup(html, strlen(html) as libc::c_ulong);

            dc_rtrim(out);

            assert_eq!(
                CStr::from_ptr(out as *const libc::c_char).to_str().unwrap(),
                "\r\r\nline1<br>\r\n\r\n\r\rline2"
            );
        }
    }

    #[test]
    fn test_dc_trim() {
        unsafe {
            let html: *const libc::c_char =
                b"\r\r\nline1<br>\r\n\r\n\r\rline2\n\r\x00" as *const u8 as *const libc::c_char;
            let out: *mut libc::c_char = strndup(html, strlen(html) as libc::c_ulong);

            dc_trim(out);

            assert_eq!(
                CStr::from_ptr(out as *const libc::c_char).to_str().unwrap(),
                "line1<br>\r\n\r\n\r\rline2"
            );
        }
    }

    #[test]
    fn test_dc_atof() {
        let f: libc::c_double = dc_atof(b"1.23\x00" as *const u8 as *const libc::c_char);
        assert!(f > 1.22f64);
        assert!(f < 1.24f64);
    }

    #[test]
    fn test_dc_ftoa() {
        unsafe {
            let s: *mut libc::c_char = dc_ftoa(1.23f64);
            assert!(dc_atof(s) > 1.22f64);
            assert!(dc_atof(s) < 1.24f64);
            free(s as *mut libc::c_void);
        }
    }

    #[test]
    fn test_rust_ftoa() {
        assert_eq!("1.22", format!("{}", 1.22));
    }

    #[test]
    fn test_dc_str_replace() {
        unsafe {
            let mut str: *mut libc::c_char = strdup(b"aaa\x00" as *const u8 as *const libc::c_char);
            let replacements: libc::c_int = dc_str_replace(
                &mut str,
                b"a\x00" as *const u8 as *const libc::c_char,
                b"ab\x00" as *const u8 as *const libc::c_char,
            );
            assert_eq!(
                CStr::from_ptr(str as *const libc::c_char).to_str().unwrap(),
                "ababab"
            );
            assert_eq!(replacements, 3);
            free(str as *mut libc::c_void);
        }
    }

    #[test]
    fn test_dc_str_truncate_1() {
        let s = "this is a little test string";
        assert_eq!(dc_truncate_str(s, 16), "this is a [...]");
    }

    #[test]
    fn test_dc_str_truncate_2() {
        assert_eq!(dc_truncate_str("1234", 2), "1234");
    }

    // This test seems wrong
    // #[test]
    // fn test_dc_str_truncate_3() {
    //     assert_eq!(dc_truncate_str("1234567", 3), "1[...]");
    // }

    #[test]
    fn test_dc_str_truncate_4() {
        assert_eq!(dc_truncate_str("123456", 4), "123456");
    }

    #[test]
    fn test_dc_insert_breaks_1() {
        unsafe {
            let str = dc_insert_breaks(
                b"just1234test\x00" as *const u8 as *const libc::c_char,
                4,
                b" \x00" as *const u8 as *const libc::c_char,
            );
            assert_eq!(
                CStr::from_ptr(str as *const libc::c_char).to_str().unwrap(),
                "just 1234 test"
            );
            free(str as *mut libc::c_void);
        }
    }

    #[test]
    fn test_dc_insert_breaks_2() {
        unsafe {
            let str: *mut libc::c_char = dc_insert_breaks(
                b"just1234tes\x00" as *const u8 as *const libc::c_char,
                4i32,
                b"--\x00" as *const u8 as *const libc::c_char,
            );
            assert_eq!(
                CStr::from_ptr(str as *const libc::c_char).to_str().unwrap(),
                "just--1234--tes"
            );
            free(str as *mut libc::c_void);
        }
    }

    #[test]
    fn test_dc_insert_breaks_3() {
        unsafe {
            let str: *mut libc::c_char = dc_insert_breaks(
                b"just1234t\x00" as *const u8 as *const libc::c_char,
                4i32,
                b"\x00" as *const u8 as *const libc::c_char,
            );
            assert_eq!(
                CStr::from_ptr(str as *const libc::c_char).to_str().unwrap(),
                "just1234t"
            );
            free(str as *mut libc::c_void);
        }
    }

    #[test]
    fn test_dc_insert_breaks_4() {
        unsafe {
            let str: *mut libc::c_char = dc_insert_breaks(
                b"\x00" as *const u8 as *const libc::c_char,
                4i32,
                b"---\x00" as *const u8 as *const libc::c_char,
            );
            assert_eq!(
                CStr::from_ptr(str as *const libc::c_char).to_str().unwrap(),
                ""
            );
            free(str as *mut libc::c_void);
        }
    }

    #[test]
    fn test_dc_null_terminate_1() {
        unsafe {
            let str: *mut libc::c_char =
                dc_null_terminate(b"abcxyz\x00" as *const u8 as *const libc::c_char, 3);
            assert_eq!(
                CStr::from_ptr(str as *const libc::c_char).to_str().unwrap(),
                "abc"
            );
            free(str as *mut libc::c_void);
        }
    }

    #[test]
    fn test_dc_null_terminate_2() {
        unsafe {
            let str: *mut libc::c_char =
                dc_null_terminate(b"abcxyz\x00" as *const u8 as *const libc::c_char, 0);
            assert_eq!(
                CStr::from_ptr(str as *const libc::c_char).to_str().unwrap(),
                ""
            );
            free(str as *mut libc::c_void);
        }
    }

    #[test]
    fn test_dc_null_terminate_3() {
        unsafe {
            let str: *mut libc::c_char =
                dc_null_terminate(0 as *const u8 as *const libc::c_char, 0);
            assert_eq!(
                CStr::from_ptr(str as *const libc::c_char).to_str().unwrap(),
                ""
            );
            free(str as *mut libc::c_void);
        }
    }

    #[test]
    fn test_dc_str_to_clist_1() {
        unsafe {
            let list: *mut clist = dc_str_to_clist(
                0 as *const libc::c_char,
                b" \x00" as *const u8 as *const libc::c_char,
            );
            assert_eq!((*list).count, 0);
            clist_free_content(list);
            clist_free(list);
        }
    }

    #[test]
    fn test_dc_str_to_clist_2() {
        unsafe {
            let list: *mut clist = dc_str_to_clist(
                b"\x00" as *const u8 as *const libc::c_char,
                b" \x00" as *const u8 as *const libc::c_char,
            );
            assert_eq!((*list).count, 1);
            clist_free_content(list);
            clist_free(list);
        }
    }

    #[test]
    fn test_dc_str_to_clist_3() {
        unsafe {
            let list: *mut clist = dc_str_to_clist(
                b" \x00" as *const u8 as *const libc::c_char,
                b" \x00" as *const u8 as *const libc::c_char,
            );
            assert_eq!((*list).count, 2);
            clist_free_content(list);
            clist_free(list);
        }
    }

    #[test]
    fn test_dc_str_to_clist_4() {
        unsafe {
            let list: *mut clist = dc_str_to_clist(
                b"foo bar test\x00" as *const u8 as *const libc::c_char,
                b" \x00" as *const u8 as *const libc::c_char,
            );
            assert_eq!((*list).count, 3);
            let str: *mut libc::c_char =
                dc_str_from_clist(list, b" \x00" as *const u8 as *const libc::c_char);

            assert_eq!(
                CStr::from_ptr(str as *const libc::c_char).to_str().unwrap(),
                "foo bar test"
            );

            clist_free_content(list);
            clist_free(list);
            free(str as *mut libc::c_void);
        }
    }

    #[test]
    fn test_dc_binary_to_uc_hex() {
        let buf = vec![0, 1, 2, 3, 255];

        let raw = unsafe { dc_binary_to_uc_hex(buf.as_ptr(), buf.len()) };
        let res = to_string(raw);
        assert_eq!(res, "00010203FF");

        unsafe { free(raw as *mut _) };
    }

    #[test]
    fn test_dc_replace_bad_utf8_chars_1() {
        unsafe {
            let buf1 = strdup(b"ol\xc3\xa1 mundo <>\"\'& \xc3\xa4\xc3\x84\xc3\xb6\xc3\x96\xc3\xbc\xc3\x9c\xc3\x9f foo\xc3\x86\xc3\xa7\xc3\x87 \xe2\x99\xa6&noent;\x00" as *const u8 as *const libc::c_char);
            let buf2 = strdup(buf1);

            dc_replace_bad_utf8_chars(buf2);

            assert_eq!(strcmp(buf1, buf2), 0);

            free(buf1 as *mut libc::c_void);
            free(buf2 as *mut libc::c_void);
        }
    }

    #[test]
    fn test_dc_replace_bad_utf8_chars_2() {
        unsafe {
            let buf1 = strdup(b"ISO-String with Ae: \xc4\x00" as *const u8 as *const libc::c_char);
            let buf2 = strdup(buf1);

            dc_replace_bad_utf8_chars(buf2);

            assert_eq!(
                CStr::from_ptr(buf2 as *const libc::c_char)
                    .to_str()
                    .unwrap(),
                "ISO-String with Ae: _"
            );

            free(buf1 as *mut libc::c_void);
            free(buf2 as *mut libc::c_void);
        }
    }

    #[test]
    fn test_dc_replace_bad_utf8_chars_3() {
        unsafe {
            let buf1 = strdup(b"\x00" as *const u8 as *const libc::c_char);
            let buf2 = strdup(buf1);

            dc_replace_bad_utf8_chars(buf2);

            assert_eq!(*buf2.offset(0), 0);

            free(buf1 as *mut libc::c_void);
            free(buf2 as *mut libc::c_void);
        }
    }

    #[test]
    fn test_dc_replace_bad_utf8_chars_4() {
        unsafe {
            dc_replace_bad_utf8_chars(0 as *mut libc::c_char);
        }
    }

    #[test]
    fn test_dc_create_id() {
        let buf = dc_create_id();
        assert_eq!(buf.len(), 11);
    }

    #[test]
    fn test_encode_66bits_as_base64() {
        assert_eq!(
            encode_66bits_as_base64(0x01234567, 0x89abcdef, 0),
            "ASNFZ4mrze8"
        );
        assert_eq!(
            encode_66bits_as_base64(0x01234567, 0x89abcdef, 1),
            "ASNFZ4mrze9"
        );
        assert_eq!(
            encode_66bits_as_base64(0x01234567, 0x89abcdef, 2),
            "ASNFZ4mrze-"
        );
        assert_eq!(
            encode_66bits_as_base64(0x01234567, 0x89abcdef, 3),
            "ASNFZ4mrze_"
        );
    }

    #[test]
    fn test_dc_utf8_strlen() {
        unsafe {
            assert_eq!(
                dc_utf8_strlen(b"c\x00" as *const u8 as *const libc::c_char),
                1
            );
            assert_eq!(
                dc_utf8_strlen(b"\xc3\xa4\x00" as *const u8 as *const libc::c_char),
                1
            );
        }
    }

    #[test]
    fn test_os_str_to_c_string_cwd() {
        let some_dir = std::env::current_dir().unwrap();
        some_dir.as_os_str().to_c_string().unwrap();
    }

    #[test]
    fn test_os_str_to_c_string_unicode() {
        let some_str = String::from("/some/valid/utf8");
        let some_dir = std::path::Path::new(&some_str);
        assert_eq!(
            some_dir.as_os_str().to_c_string().unwrap(),
            CString::new("/some/valid/utf8").unwrap()
        );
    }

    #[test]
    fn test_os_str_to_c_string_nul() {
        let some_str = std::ffi::OsString::from("foo\x00bar");
        assert_eq!(
            some_str.to_c_string().err().unwrap(),
            CStringError::InteriorNullByte
        )
    }

    #[test]
    fn test_path_to_c_string_cwd() {
        let some_dir = std::env::current_dir().unwrap();
        some_dir.to_c_string().unwrap();
    }

    #[test]
    fn test_path_to_c_string_unicode() {
        let some_str = String::from("/some/valid/utf8");
        let some_dir = std::path::Path::new(&some_str);
        assert_eq!(
            some_dir.as_os_str().to_c_string().unwrap(),
            CString::new("/some/valid/utf8").unwrap()
        );
    }

    #[test]
    fn test_os_str_to_c_string_unicode_fn() {
        let some_str = std::ffi::OsString::from("foo");
        assert_eq!(
            os_str_to_c_string_unicode(&some_str).unwrap(),
            CString::new("foo").unwrap()
        );
    }

    #[test]
    fn test_path_to_c_string_unicode_fn() {
        let some_str = String::from("/some/path");
        let some_path = std::path::Path::new(&some_str);
        assert_eq!(
            os_str_to_c_string_unicode(&some_path).unwrap(),
            CString::new("/some/path").unwrap()
        );
    }

    #[test]
    fn test_os_str_to_c_string_unicode_fn_nul() {
        let some_str = std::ffi::OsString::from("fooz\x00bar");
        assert_eq!(
            os_str_to_c_string_unicode(&some_str).err().unwrap(),
            CStringError::InteriorNullByte
        );
    }

    #[test]
    fn test_as_path() {
        let some_path = CString::new("/some/path").unwrap();
        let ptr = some_path.as_ptr();
        assert_eq!(as_path(ptr), std::ffi::OsString::from("/some/path"))
    }

    #[test]
    fn test_as_path_unicode_fn() {
        let some_path = CString::new("/some/path").unwrap();
        let ptr = some_path.as_ptr();
        assert_eq!(as_path_unicode(ptr), std::ffi::OsString::from("/some/path"));
    }
}
