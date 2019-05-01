use libc;
use rand::{thread_rng, Rng};

use crate::dc_array::*;
use crate::dc_context::dc_context_t;
use crate::dc_log::*;
use crate::dc_strbuilder::*;
use crate::types::*;
use crate::x::*;

/* Some tools and enhancements to the used libraries, there should be
no references to dc_context_t and other "larger" classes here. */
// for carray etc.
/* ** library-private **********************************************************/
/* math tools */
pub unsafe fn dc_exactly_one_bit_set(mut v: libc::c_int) -> libc::c_int {
    return (0 != v && 0 == v & v - 1i32) as libc::c_int;
}
/* string tools */
/* dc_strdup() returns empty string if NULL is given, never returns NULL (exits on errors) */
pub unsafe fn dc_strdup(mut s: *const libc::c_char) -> *mut libc::c_char {
    let mut ret: *mut libc::c_char = 0 as *mut libc::c_char;
    if !s.is_null() {
        ret = strdup(s);
        if ret.is_null() {
            exit(16i32);
        }
    } else {
        ret = calloc(1, 1) as *mut libc::c_char;
        if ret.is_null() {
            exit(17i32);
        }
    }
    return ret;
}
/* strdup(NULL) is undefined, safe_strdup_keep_null(NULL) returns NULL in this case */
pub unsafe fn dc_strdup_keep_null(mut s: *const libc::c_char) -> *mut libc::c_char {
    return if !s.is_null() {
        dc_strdup(s)
    } else {
        0 as *mut libc::c_char
    };
}
pub unsafe fn dc_atoi_null_is_0(mut s: *const libc::c_char) -> libc::c_int {
    return if !s.is_null() { atoi(s) } else { 0i32 };
}
pub unsafe fn dc_atof(mut str: *const libc::c_char) -> libc::c_double {
    // hack around atof() that may accept only `,` as decimal point on mac
    let mut test: *mut libc::c_char =
        dc_mprintf(b"%f\x00" as *const u8 as *const libc::c_char, 1.2f64);
    *test.offset(2isize) = 0i32 as libc::c_char;
    let mut str_locale: *mut libc::c_char = dc_strdup(str);
    dc_str_replace(
        &mut str_locale,
        b".\x00" as *const u8 as *const libc::c_char,
        test.offset(1isize),
    );
    let mut f: libc::c_double = atof(str_locale);
    free(test as *mut libc::c_void);
    free(str_locale as *mut libc::c_void);
    return f;
}
pub unsafe fn dc_str_replace(
    mut haystack: *mut *mut libc::c_char,
    mut needle: *const libc::c_char,
    mut replacement: *const libc::c_char,
) -> libc::c_int {
    let mut replacements: libc::c_int = 0i32;
    let mut start_search_pos: libc::c_int = 0i32;
    let mut needle_len: libc::c_int = 0i32;
    let mut replacement_len: libc::c_int = 0i32;
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
        start_search_pos = (p2.wrapping_offset_from(*haystack) as libc::c_long
            + replacement_len as libc::c_long) as libc::c_int;
        *p2 = 0i32 as libc::c_char;
        p2 = p2.offset(needle_len as isize);
        let mut new_string: *mut libc::c_char = dc_mprintf(
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
    return replacements;
}
pub unsafe fn dc_ftoa(mut f: libc::c_double) -> *mut libc::c_char {
    // hack around printf(%f) that may return `,` as decimal point on mac
    let mut test: *mut libc::c_char =
        dc_mprintf(b"%f\x00" as *const u8 as *const libc::c_char, 1.2f64);
    *test.offset(2isize) = 0i32 as libc::c_char;
    let mut str: *mut libc::c_char = dc_mprintf(b"%f\x00" as *const u8 as *const libc::c_char, f);
    dc_str_replace(
        &mut str,
        test.offset(1isize),
        b".\x00" as *const u8 as *const libc::c_char,
    );
    free(test as *mut libc::c_void);
    return str;
}
pub unsafe fn dc_ltrim(mut buf: *mut libc::c_char) {
    let mut len: size_t = 0i32 as size_t;
    let mut cur: *const libc::c_uchar = 0 as *const libc::c_uchar;
    if !buf.is_null() && 0 != *buf as libc::c_int {
        len = strlen(buf);
        cur = buf as *const libc::c_uchar;
        while 0 != *cur as libc::c_int && 0 != isspace(*cur as libc::c_int) {
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
pub unsafe fn dc_rtrim(mut buf: *mut libc::c_char) {
    let mut len: size_t = 0i32 as size_t;
    let mut cur: *mut libc::c_uchar = 0 as *mut libc::c_uchar;
    if !buf.is_null() && 0 != *buf as libc::c_int {
        len = strlen(buf);
        cur = (buf as *mut libc::c_uchar)
            .offset(len as isize)
            .offset(-1isize);
        while cur != buf as *mut libc::c_uchar && 0 != isspace(*cur as libc::c_int) {
            cur = cur.offset(-1isize);
            len = len.wrapping_sub(1)
        }
        *cur.offset(
            (if 0 != isspace(*cur as libc::c_int) {
                0i32
            } else {
                1i32
            }) as isize,
        ) = '\u{0}' as i32 as libc::c_uchar
    };
}
pub unsafe fn dc_trim(mut buf: *mut libc::c_char) {
    dc_ltrim(buf);
    dc_rtrim(buf);
}
/* the result must be free()'d */
pub unsafe fn dc_strlower(mut in_0: *const libc::c_char) -> *mut libc::c_char {
    let mut out: *mut libc::c_char = dc_strdup(in_0);
    let mut p: *mut libc::c_char = out;
    while 0 != *p {
        *p = tolower(*p as libc::c_int) as libc::c_char;
        p = p.offset(1isize)
    }
    return out;
}
pub unsafe fn dc_strlower_in_place(mut in_0: *mut libc::c_char) {
    let mut p: *mut libc::c_char = in_0;
    while 0 != *p {
        *p = tolower(*p as libc::c_int) as libc::c_char;
        p = p.offset(1isize)
    }
}
pub unsafe fn dc_str_contains(
    mut haystack: *const libc::c_char,
    mut needle: *const libc::c_char,
) -> libc::c_int {
    if haystack.is_null() || needle.is_null() {
        return 0i32;
    }
    if !strstr(haystack, needle).is_null() {
        return 1i32;
    }
    let mut haystack_lower: *mut libc::c_char = dc_strlower(haystack);
    let mut needle_lower: *mut libc::c_char = dc_strlower(needle);
    let mut ret: libc::c_int = if !strstr(haystack_lower, needle_lower).is_null() {
        1i32
    } else {
        0i32
    };
    free(haystack_lower as *mut libc::c_void);
    free(needle_lower as *mut libc::c_void);
    return ret;
}
/* the result must be free()'d */
pub unsafe fn dc_null_terminate(
    mut in_0: *const libc::c_char,
    mut bytes: libc::c_int,
) -> *mut libc::c_char {
    let mut out: *mut libc::c_char = malloc(bytes as usize + 1) as *mut libc::c_char;
    if out.is_null() {
        exit(45i32);
    }
    if !in_0.is_null() && bytes > 0i32 {
        strncpy(out, in_0, bytes as usize);
    }
    *out.offset(bytes as isize) = 0i32 as libc::c_char;
    return out;
}
pub unsafe fn dc_binary_to_uc_hex(mut buf: *const uint8_t, mut bytes: size_t) -> *mut libc::c_char {
    let mut hex: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut i = 0;
    if !(buf.is_null() || bytes <= 0) {
        hex = calloc(
            ::std::mem::size_of::<libc::c_char>(),
            bytes.wrapping_mul(2).wrapping_add(1),
        ) as *mut libc::c_char;
        if !hex.is_null() {
            i = 0;
            while i < bytes {
                snprintf(
                    &mut *hex.offset((i * 2) as isize) as *mut libc::c_char,
                    3,
                    b"%02X\x00" as *const u8 as *const libc::c_char,
                    *buf.offset(i as isize) as libc::c_int,
                );
                i += 1
            }
        }
    }
    return hex;
}
/* remove all \r characters from string */
pub unsafe extern "C" fn dc_remove_cr_chars(mut buf: *mut libc::c_char) {
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
pub unsafe fn dc_unify_lineends(mut buf: *mut libc::c_char) {
    dc_remove_cr_chars(buf);
}
/* replace bad UTF-8 characters by sequences of `_` (to avoid problems in filenames, we do not use eg. `?`) the function is useful if strings are unexpectingly encoded eg. as ISO-8859-1 */
pub unsafe fn dc_replace_bad_utf8_chars(mut buf: *mut libc::c_char) {
    let mut current_block: u64;
    if buf.is_null() {
        return;
    }
    /* force unsigned - otherwise the `> ' '` comparison will fail */
    let mut p1: *mut libc::c_uchar = buf as *mut libc::c_uchar;
    let mut p1len: libc::c_int = strlen(buf) as libc::c_int;
    let mut c: libc::c_int = 0i32;
    let mut i: libc::c_int = 0i32;
    let mut ix: libc::c_int = 0i32;
    let mut n: libc::c_int = 0i32;
    let mut j: libc::c_int = 0i32;
    i = 0i32;
    ix = p1len;
    's_36: loop {
        if !(i < ix) {
            current_block = 13550086250199790493;
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
            current_block = 2775201239069267972;
            break;
        } else if c & 0xf0i32 == 0xe0i32 {
            n = 2i32
        } else if c & 0xf8i32 == 0xf0i32 {
            n = 3i32
        } else {
            //else if ((c & 0xFC) == 0xF8)                          { n=4; }        /* 111110bb - not valid in https://tools.ietf.org/html/rfc3629 */
            //else if ((c & 0xFE) == 0xFC)                          { n=5; }        /* 1111110b - not valid in https://tools.ietf.org/html/rfc3629 */
            current_block = 2775201239069267972;
            break;
        }
        j = 0i32;
        while j < n && i < ix {
            /* n bytes matching 10bbbbbb follow ? */
            i += 1;
            if i == ix || *p1.offset(i as isize) as libc::c_int & 0xc0i32 != 0x80i32 {
                current_block = 2775201239069267972;
                break 's_36;
            }
            j += 1
        }
        i += 1
    }
    match current_block {
        13550086250199790493 => return,
        _ => {
            while 0 != *p1 {
                if *p1 as libc::c_int > 0x7fi32 {
                    *p1 = '_' as i32 as libc::c_uchar
                }
                p1 = p1.offset(1isize)
            }
            return;
        }
    };
}
pub unsafe fn dc_utf8_strlen(mut s: *const libc::c_char) -> size_t {
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
    return j;
}
pub unsafe fn dc_truncate_str(mut buf: *mut libc::c_char, mut approx_chars: libc::c_int) {
    if approx_chars > 0
        && strlen(buf)
            > approx_chars.wrapping_add(
                strlen(b"[...]\x00" as *const u8 as *const libc::c_char) as libc::c_int
            ) as usize
    {
        let mut p: *mut libc::c_char = &mut *buf.offset(approx_chars as isize) as *mut libc::c_char;
        *p = 0i32 as libc::c_char;
        if !strchr(buf, ' ' as i32).is_null() {
            while *p.offset(-1i32 as isize) as libc::c_int != ' ' as i32
                && *p.offset(-1i32 as isize) as libc::c_int != '\n' as i32
            {
                p = p.offset(-1isize);
                *p = 0i32 as libc::c_char
            }
        }
        strcat(p, b"[...]\x00" as *const u8 as *const libc::c_char);
    };
}
pub unsafe fn dc_truncate_n_unwrap_str(
    mut buf: *mut libc::c_char,
    mut approx_characters: libc::c_int,
    mut do_unwrap: libc::c_int,
) {
    /* Function unwraps the given string and removes unnecessary whitespace.
    Function stops processing after approx_characters are processed.
    (as we're using UTF-8, for simplicity, we cut the string only at whitespaces). */
    /* a single line is truncated `...` instead of `[...]` (the former is typically also used by the UI to fit strings in a rectangle) */
    let mut ellipse_utf8: *const libc::c_char = if 0 != do_unwrap {
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
            let mut used_bytes: size_t = (p1 as uintptr_t).wrapping_sub(buf as uintptr_t) as size_t;
            if dc_utf8_strnlen(buf, used_bytes) >= approx_characters as usize {
                let mut buf_bytes: size_t = strlen(buf);
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
unsafe fn dc_utf8_strnlen(mut s: *const libc::c_char, mut n: size_t) -> size_t {
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
    return j;
}
/* split string into lines*/
pub unsafe fn dc_split_into_lines(mut buf_terminated: *const libc::c_char) -> *mut carray {
    let mut lines: *mut carray = carray_new(1024i32 as libc::c_uint);
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
    return lines;
}
pub unsafe fn dc_free_splitted_lines(mut lines: *mut carray) {
    if !lines.is_null() {
        let mut i: libc::c_int = 0;
        let mut cnt: libc::c_int = carray_count(lines) as libc::c_int;
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
    mut in_0: *const libc::c_char,
    mut break_every: libc::c_int,
    mut break_chars: *const libc::c_char,
) -> *mut libc::c_char {
    if in_0.is_null() || break_every <= 0i32 || break_chars.is_null() {
        return dc_strdup(in_0);
    }
    let mut out_len = strlen(in_0);
    let mut chars_added = 0;
    let mut break_chars_len = strlen(break_chars);
    out_len += (out_len / break_every as usize + 1) * break_chars_len + 1;
    let mut out: *mut libc::c_char = malloc(out_len) as *mut libc::c_char;
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
    return out;
}
pub unsafe fn dc_str_from_clist(
    mut list: *const clist,
    mut delimiter: *const libc::c_char,
) -> *mut libc::c_char {
    let mut str: dc_strbuilder_t = dc_strbuilder_t {
        buf: 0 as *mut libc::c_char,
        allocated: 0,
        free: 0,
        eos: 0 as *mut libc::c_char,
    };
    dc_strbuilder_init(&mut str, 256i32);
    if !list.is_null() {
        let mut cur: *mut clistiter = (*list).first;
        while !cur.is_null() {
            let mut rfc724_mid: *const libc::c_char = (if !cur.is_null() {
                (*cur).data
            } else {
                0 as *mut libc::c_void
            }) as *const libc::c_char;
            if !rfc724_mid.is_null() {
                if 0 != *str.buf.offset(0isize) as libc::c_int && !delimiter.is_null() {
                    dc_strbuilder_cat(&mut str, delimiter);
                }
                dc_strbuilder_cat(&mut str, rfc724_mid);
            }
            cur = if !cur.is_null() {
                (*cur).next
            } else {
                0 as *mut clistcell_s
            }
        }
    }
    return str.buf;
}
pub unsafe fn dc_str_to_clist(
    mut str: *const libc::c_char,
    mut delimiter: *const libc::c_char,
) -> *mut clist {
    let mut list: *mut clist = clist_new();
    if list.is_null() {
        exit(54i32);
    }
    if !str.is_null() && !delimiter.is_null() && strlen(delimiter) >= 1 {
        let mut p1: *const libc::c_char = str;
        loop {
            let mut p2: *const libc::c_char = strstr(p1, delimiter);
            if p2.is_null() {
                clist_insert_after(list, (*list).last, strdup(p1) as *mut libc::c_void);
                break;
            } else {
                clist_insert_after(
                    list,
                    (*list).last,
                    strndup(
                        p1,
                        p2.wrapping_offset_from(p1) as libc::c_long as libc::c_ulong,
                    ) as *mut libc::c_void,
                );
                p1 = p2.offset(strlen(delimiter) as isize)
            }
        }
    }
    return list;
}
pub unsafe fn dc_str_to_color(mut str: *const libc::c_char) -> libc::c_int {
    let mut str_lower: *mut libc::c_char = dc_strlower(str);
    /* the colors must fulfill some criterions as:
    - contrast to black and to white
    - work as a text-color
    - being noticable on a typical map
    - harmonize together while being different enough
    (therefore, we cannot just use random rgb colors :) */
    static mut colors: [uint32_t; 16] = [
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
    let mut str_len: libc::c_int = strlen(str_lower) as libc::c_int;
    let mut i: libc::c_int = 0i32;
    while i < str_len {
        checksum += (i + 1i32) * *str_lower.offset(i as isize) as libc::c_int;
        checksum %= 0xffffffi32;
        i += 1
    }
    let mut color_index: libc::c_int = (checksum as libc::c_ulong).wrapping_rem(
        (::std::mem::size_of::<[uint32_t; 16]>() as libc::c_ulong)
            .wrapping_div(::std::mem::size_of::<uint32_t>() as libc::c_ulong),
    ) as libc::c_int;
    free(str_lower as *mut libc::c_void);
    return colors[color_index as usize] as libc::c_int;
}
/* clist tools */
/* calls free() for each item content */
pub unsafe fn clist_free_content(mut haystack: *const clist) {
    let mut iter: *mut clistiter = (*haystack).first;
    while !iter.is_null() {
        free((*iter).data);
        (*iter).data = 0 as *mut libc::c_void;
        iter = if !iter.is_null() {
            (*iter).next
        } else {
            0 as *mut clistcell_s
        }
    }
}
pub unsafe fn clist_search_string_nocase(
    mut haystack: *const clist,
    mut needle: *const libc::c_char,
) -> libc::c_int {
    let mut iter: *mut clistiter = (*haystack).first;
    while !iter.is_null() {
        if strcasecmp((*iter).data as *const libc::c_char, needle) == 0i32 {
            return 1i32;
        }
        iter = if !iter.is_null() {
            (*iter).next
        } else {
            0 as *mut clistcell_s
        }
    }
    return 0i32;
}
/* date/time tools */
/* the result is UTC or DC_INVALID_TIMESTAMP */
pub unsafe fn dc_timestamp_from_date(mut date_time: *mut mailimf_date_time) -> time_t {
    let mut tmval: tm = tm {
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
    let mut timeval: time_t = 0i32 as time_t;
    let mut zone_min: libc::c_int = 0i32;
    let mut zone_hour: libc::c_int = 0i32;
    memset(
        &mut tmval as *mut tm as *mut libc::c_void,
        0,
        ::std::mem::size_of::<tm>(),
    );
    tmval.tm_sec = (*date_time).dt_sec;
    tmval.tm_min = (*date_time).dt_min;
    tmval.tm_hour = (*date_time).dt_hour;
    tmval.tm_mday = (*date_time).dt_day;
    tmval.tm_mon = (*date_time).dt_month - 1i32;
    if (*date_time).dt_year < 1000i32 {
        tmval.tm_year = (*date_time).dt_year + 2000i32 - 1900i32
    } else {
        tmval.tm_year = (*date_time).dt_year - 1900i32
    }
    timeval = mkgmtime(&mut tmval);
    if (*date_time).dt_zone >= 0i32 {
        zone_hour = (*date_time).dt_zone / 100i32;
        zone_min = (*date_time).dt_zone % 100i32
    } else {
        zone_hour = -(-(*date_time).dt_zone / 100i32);
        zone_min = -(-(*date_time).dt_zone % 100i32)
    }
    timeval -= (zone_hour * 3600i32 + zone_min * 60i32) as libc::c_long;
    return timeval;
}
pub unsafe fn mkgmtime(mut tmp: *mut tm) -> time_t {
    let mut dir: libc::c_int = 0i32;
    let mut bits: libc::c_int = 0i32;
    let mut saved_seconds: libc::c_int = 0i32;
    let mut t: time_t = 0i32 as time_t;
    let mut yourtm: tm = tm {
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
    let mut mytm: tm = tm {
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
    yourtm = *tmp;
    saved_seconds = yourtm.tm_sec;
    yourtm.tm_sec = 0i32;
    bits = 0i32;
    t = 1i32 as time_t;
    while t > 0i32 as libc::c_long {
        bits += 1;
        t <<= 1i32
    }
    if bits > 40i32 {
        bits = 40i32
    }
    t = if t < 0i32 as libc::c_long {
        0i32 as libc::c_long
    } else {
        (1i32 as time_t) << bits
    };
    loop {
        gmtime_r(&mut t, &mut mytm);
        dir = tmcomp(&mut mytm, &mut yourtm);
        if !(dir != 0i32) {
            break;
        }
        let fresh2 = bits;
        bits = bits - 1;
        if fresh2 < 0i32 {
            return -1i32 as time_t;
        }
        if bits < 0i32 {
            t -= 1
        } else if dir > 0i32 {
            t -= (1i32 as time_t) << bits
        } else {
            t += (1i32 as time_t) << bits
        }
    }
    t += saved_seconds as libc::c_long;
    return t;
}
/* ******************************************************************************
 * date/time tools
 ******************************************************************************/
unsafe fn tmcomp(mut atmp: *mut tm, mut btmp: *mut tm) -> libc::c_int {
    let mut result: libc::c_int = 0i32;
    result = (*atmp).tm_year - (*btmp).tm_year;
    if result == 0i32
        && {
            result = (*atmp).tm_mon - (*btmp).tm_mon;
            result == 0i32
        }
        && {
            result = (*atmp).tm_mday - (*btmp).tm_mday;
            result == 0i32
        }
        && {
            result = (*atmp).tm_hour - (*btmp).tm_hour;
            result == 0i32
        }
        && {
            result = (*atmp).tm_min - (*btmp).tm_min;
            result == 0i32
        }
    {
        result = (*atmp).tm_sec - (*btmp).tm_sec
    }
    return result;
}
/* the return value must be free()'d */
pub unsafe fn dc_timestamp_to_str(mut wanted: time_t) -> *mut libc::c_char {
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
        localtime(&mut wanted) as *const libc::c_void,
        ::std::mem::size_of::<tm>(),
    );
    return dc_mprintf(
        b"%02i.%02i.%04i %02i:%02i:%02i\x00" as *const u8 as *const libc::c_char,
        wanted_struct.tm_mday as libc::c_int,
        wanted_struct.tm_mon as libc::c_int + 1i32,
        wanted_struct.tm_year as libc::c_int + 1900i32,
        wanted_struct.tm_hour as libc::c_int,
        wanted_struct.tm_min as libc::c_int,
        wanted_struct.tm_sec as libc::c_int,
    );
}
pub unsafe fn dc_timestamp_to_mailimap_date_time(mut timeval: time_t) -> *mut mailimap_date_time {
    let mut gmt: tm = tm {
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
    let mut lt: tm = tm {
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
    let mut off: libc::c_int = 0i32;
    let mut date_time: *mut mailimap_date_time = 0 as *mut mailimap_date_time;
    let mut sign: libc::c_int = 0i32;
    let mut hour: libc::c_int = 0i32;
    let mut min: libc::c_int = 0i32;
    gmtime_r(&mut timeval, &mut gmt);
    localtime_r(&mut timeval, &mut lt);
    off = ((mkgmtime(&mut lt) - mkgmtime(&mut gmt)) / 60i32 as libc::c_long) as libc::c_int;
    if off < 0i32 {
        sign = -1i32
    } else {
        sign = 1i32
    }
    off = off * sign;
    min = off % 60i32;
    hour = off / 60i32;
    off = hour * 100i32 + min;
    off = off * sign;
    date_time = mailimap_date_time_new(
        lt.tm_mday,
        lt.tm_mon + 1i32,
        lt.tm_year + 1900i32,
        lt.tm_hour,
        lt.tm_min,
        lt.tm_sec,
        off,
    );
    return date_time;
}
pub unsafe fn dc_gm2local_offset() -> libc::c_long {
    /* returns the offset that must be _added_ to an UTC/GMT-time to create the localtime.
    the function may return nagative values. */
    let mut gmtime: time_t = time(0 as *mut time_t);
    let mut timeinfo: tm = tm {
        tm_sec: 0i32,
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
    localtime_r(&mut gmtime, &mut timeinfo);
    return timeinfo.tm_gmtoff;
}
/* timesmearing */
pub unsafe fn dc_smeared_time(mut context: &dc_context_t) -> time_t {
    /* function returns a corrected time(NULL) */
    let mut now: time_t = time(0 as *mut time_t);
    let mut ts = *context.last_smeared_timestamp.clone().read().unwrap();
    if ts >= now {
        now = ts + 1;
    }
    now
}

pub unsafe fn dc_create_smeared_timestamp(mut context: &dc_context_t) -> time_t {
    let mut now: time_t = time(0 as *mut time_t);
    let mut ret: time_t = now;

    let mut ts = *context.last_smeared_timestamp.clone().write().unwrap();
    if ret <= ts {
        ret = ts + 1;
        if ret - now > 5 {
            ret = now + 5
        }
    }
    ts = ret;
    ret
}

pub unsafe fn dc_create_smeared_timestamps(
    mut context: &dc_context_t,
    mut count: libc::c_int,
) -> time_t {
    /* get a range to timestamps that can be used uniquely */
    let mut now: time_t = time(0 as *mut time_t);
    let mut start: time_t =
        now + (if count < 5i32 { count } else { 5i32 }) as libc::c_long - count as libc::c_long;

    let mut ts = *context.last_smeared_timestamp.clone().write().unwrap();
    start = if ts + 1 > start { ts + 1 } else { start };
    ts = start + ((count - 1) as time_t);
    start
}

/* Message-ID tools */
pub unsafe fn dc_create_id() -> *mut libc::c_char {
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
    let mut buf: [uint32_t; 3] = [rng.gen(), rng.gen(), rng.gen()];
    return encode_66bits_as_base64(buf[0usize], buf[1usize], buf[2usize]);
}
/* ******************************************************************************
 * generate Message-IDs
 ******************************************************************************/
unsafe fn encode_66bits_as_base64(
    mut v1: uint32_t,
    mut v2: uint32_t,
    mut fill: uint32_t,
) -> *mut libc::c_char {
    /* encode 66 bits as a base64 string. This is useful for ID generating with short strings as
    we save 5 character in each id compared to 64 bit hex encoding, for a typical group ID, these are 10 characters (grpid+msgid):
    hex:    64 bit, 4 bits/character, length = 64/4 = 16 characters
    base64: 64 bit, 6 bits/character, length = 64/6 = 11 characters (plus 2 additional bits) */
    let mut ret: *mut libc::c_char = malloc(12) as *mut libc::c_char;
    if ret.is_null() {
        exit(34i32);
    }
    static mut chars: [libc::c_char; 65] = [
        65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87,
        88, 89, 90, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112,
        113, 114, 115, 116, 117, 118, 119, 120, 121, 122, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57,
        45, 95, 0,
    ];
    *ret.offset(0isize) = chars[(v1 >> 26i32 & 0x3fi32 as libc::c_uint) as usize];
    *ret.offset(1isize) = chars[(v1 >> 20i32 & 0x3fi32 as libc::c_uint) as usize];
    *ret.offset(2isize) = chars[(v1 >> 14i32 & 0x3fi32 as libc::c_uint) as usize];
    *ret.offset(3isize) = chars[(v1 >> 8i32 & 0x3fi32 as libc::c_uint) as usize];
    *ret.offset(4isize) = chars[(v1 >> 2i32 & 0x3fi32 as libc::c_uint) as usize];
    *ret.offset(5isize) = chars
        [(v1 << 4i32 & 0x30i32 as libc::c_uint | v2 >> 28i32 & 0xfi32 as libc::c_uint) as usize];
    *ret.offset(6isize) = chars[(v2 >> 22i32 & 0x3fi32 as libc::c_uint) as usize];
    *ret.offset(7isize) = chars[(v2 >> 16i32 & 0x3fi32 as libc::c_uint) as usize];
    *ret.offset(8isize) = chars[(v2 >> 10i32 & 0x3fi32 as libc::c_uint) as usize];
    *ret.offset(9isize) = chars[(v2 >> 4i32 & 0x3fi32 as libc::c_uint) as usize];
    *ret.offset(10isize) =
        chars[(v2 << 2i32 & 0x3ci32 as libc::c_uint | fill & 0x3i32 as libc::c_uint) as usize];
    *ret.offset(11isize) = 0i32 as libc::c_char;
    return ret;
}
pub unsafe fn dc_create_incoming_rfc724_mid(
    mut message_timestamp: time_t,
    mut contact_id_from: uint32_t,
    mut contact_ids_to: *mut dc_array_t,
) -> *mut libc::c_char {
    if contact_ids_to.is_null() || dc_array_get_cnt(contact_ids_to) == 0 {
        return 0 as *mut libc::c_char;
    }
    /* find out the largest receiver ID (we could also take the smallest, but it should be unique) */
    let mut i: size_t = 0i32 as size_t;
    let mut icnt: size_t = dc_array_get_cnt(contact_ids_to);
    let mut largest_id_to: uint32_t = 0i32 as uint32_t;
    i = 0i32 as size_t;
    while i < icnt {
        let mut cur_id: uint32_t = dc_array_get_id(contact_ids_to, i);
        if cur_id > largest_id_to {
            largest_id_to = cur_id
        }
        i = i.wrapping_add(1)
    }
    return dc_mprintf(
        b"%lu-%lu-%lu@stub\x00" as *const u8 as *const libc::c_char,
        message_timestamp as libc::c_ulong,
        contact_id_from as libc::c_ulong,
        largest_id_to as libc::c_ulong,
    );
}
pub unsafe fn dc_create_outgoing_rfc724_mid(
    mut grpid: *const libc::c_char,
    mut from_addr: *const libc::c_char,
) -> *mut libc::c_char {
    /* Function generates a Message-ID that can be used for a new outgoing message.
    - this function is called for all outgoing messages.
    - the message ID should be globally unique
    - do not add a counter or any private data as as this may give unneeded information to the receiver	*/
    let mut rand1: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut rand2: *mut libc::c_char = dc_create_id();
    let mut ret: *mut libc::c_char = 0 as *mut libc::c_char;
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
        rand1 = dc_create_id();
        ret = dc_mprintf(
            b"Mr.%s.%s%s\x00" as *const u8 as *const libc::c_char,
            rand1,
            rand2,
            at_hostname,
        )
    }
    free(rand1 as *mut libc::c_void);
    free(rand2 as *mut libc::c_void);
    return ret;
}
pub unsafe fn dc_extract_grpid_from_rfc724_mid(mut mid: *const libc::c_char) -> *mut libc::c_char {
    /* extract our group ID from Message-IDs as `Gr.12345678901.morerandom@domain.de`; "12345678901" is the wanted ID in this example. */
    let mut success: libc::c_int = 0i32;
    let mut grpid: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut p1: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut grpid_len: libc::c_int = 0i32;
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
pub unsafe fn dc_extract_grpid_from_rfc724_mid_list(mut list: *const clist) -> *mut libc::c_char {
    if !list.is_null() {
        let mut cur: *mut clistiter = (*list).first;
        while !cur.is_null() {
            let mut mid: *const libc::c_char = (if !cur.is_null() {
                (*cur).data
            } else {
                0 as *mut libc::c_void
            }) as *const libc::c_char;
            let mut grpid: *mut libc::c_char = dc_extract_grpid_from_rfc724_mid(mid);
            if !grpid.is_null() {
                return grpid;
            }
            cur = if !cur.is_null() {
                (*cur).next
            } else {
                0 as *mut clistcell_s
            }
        }
    }
    return 0 as *mut libc::c_char;
}
/* file tools */
pub unsafe fn dc_ensure_no_slash(mut pathNfilename: *mut libc::c_char) {
    let mut path_len: libc::c_int = strlen(pathNfilename) as libc::c_int;
    if path_len > 0i32 {
        if *pathNfilename.offset((path_len - 1i32) as isize) as libc::c_int == '/' as i32
            || *pathNfilename.offset((path_len - 1i32) as isize) as libc::c_int == '\\' as i32
        {
            *pathNfilename.offset((path_len - 1i32) as isize) = 0i32 as libc::c_char
        }
    };
}
pub unsafe fn dc_validate_filename(mut filename: *mut libc::c_char) {
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
pub unsafe fn dc_get_filename(mut pathNfilename: *const libc::c_char) -> *mut libc::c_char {
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
pub unsafe fn dc_split_filename(
    mut pathNfilename: *const libc::c_char,
    mut ret_basename: *mut *mut libc::c_char,
    mut ret_all_suffixes_incl_dot: *mut *mut libc::c_char,
) {
    /* splits a filename into basename and all suffixes, eg. "/path/foo.tar.gz" is split into "foo.tar" and ".gz",
    (we use the _last_ dot which allows the usage inside the filename which are very usual;
    maybe the detection could be more intelligent, however, for the moment, it is just file)
    - if there is no suffix, the returned suffix string is empty, eg. "/path/foobar" is split into "foobar" and ""
    - the case of the returned suffix is preserved; this is to allow reconstruction of (similar) names */
    let mut basename: *mut libc::c_char = dc_get_filename(pathNfilename);
    let mut suffix: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut p1: *mut libc::c_char = strrchr(basename, '.' as i32);
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
pub unsafe fn dc_get_filesuffix_lc(mut pathNfilename: *const libc::c_char) -> *mut libc::c_char {
    if !pathNfilename.is_null() {
        let mut p: *const libc::c_char = strrchr(pathNfilename, '.' as i32);
        if !p.is_null() {
            p = p.offset(1isize);
            return dc_strlower(p);
        }
    }
    return 0 as *mut libc::c_char;
}
pub unsafe fn dc_get_filemeta(
    mut buf_start: *const libc::c_void,
    mut buf_bytes: size_t,
    mut ret_width: *mut uint32_t,
    mut ret_height: *mut uint32_t,
) -> libc::c_int {
    /* Strategy:
    reading GIF dimensions requires the first 10 bytes of the file
    reading PNG dimensions requires the first 24 bytes of the file
    reading JPEG dimensions requires scanning through jpeg chunks
    In all formats, the file is at least 24 bytes big, so we'll read that always
    inspired by http://www.cplusplus.com/forum/beginner/45217/ */
    let mut buf: *const libc::c_uchar = buf_start as *const libc::c_uchar;
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
            pos += (2i32
                + ((*buf.offset((pos + 2) as isize) as libc::c_int) << 8i32)
                + *buf.offset((pos + 3) as isize) as libc::c_int)
                as libc::c_long;
            if (pos + 12) > buf_bytes as libc::c_long {
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
    return 0i32;
}
pub unsafe fn dc_get_abs_path(
    mut context: &dc_context_t,
    mut pathNfilename: *const libc::c_char,
) -> *mut libc::c_char {
    let mut current_block: u64;
    let mut success: libc::c_int = 0i32;
    let mut pathNfilename_abs: *mut libc::c_char = 0 as *mut libc::c_char;
    if !pathNfilename.is_null() {
        pathNfilename_abs = dc_strdup(pathNfilename);
        if strncmp(
            pathNfilename_abs,
            b"$BLOBDIR\x00" as *const u8 as *const libc::c_char,
            8,
        ) == 0i32
        {
            if context.blobdir.is_null() {
                current_block = 3805228753452640762;
            } else {
                dc_str_replace(
                    &mut pathNfilename_abs,
                    b"$BLOBDIR\x00" as *const u8 as *const libc::c_char,
                    context.blobdir,
                );
                current_block = 6937071982253665452;
            }
        } else {
            current_block = 6937071982253665452;
        }
        match current_block {
            3805228753452640762 => {}
            _ => success = 1i32,
        }
    }
    if 0 == success {
        free(pathNfilename_abs as *mut libc::c_void);
        pathNfilename_abs = 0 as *mut libc::c_char
    }
    return pathNfilename_abs;
}
pub unsafe fn dc_file_exist(
    mut context: &dc_context_t,
    mut pathNfilename: *const libc::c_char,
) -> libc::c_int {
    let pathNfilename_abs = dc_get_abs_path(context, pathNfilename);
    let exist = {
        let p = std::path::Path::new(
            std::ffi::CStr::from_ptr(pathNfilename_abs)
                .to_str()
                .unwrap(),
        );
        p.exists()
    };

    free(pathNfilename_abs as *mut libc::c_void);

    exist as libc::c_int
}

pub unsafe fn dc_get_filebytes(
    mut context: &dc_context_t,
    mut pathNfilename: *const libc::c_char,
) -> uint64_t {
    let pathNfilename_abs = dc_get_abs_path(context, pathNfilename);

    let filebytes = {
        let p = std::ffi::CStr::from_ptr(pathNfilename_abs)
            .to_str()
            .unwrap();
        std::fs::metadata(p).unwrap().len()
    };

    free(pathNfilename_abs as *mut libc::c_void);
    filebytes as uint64_t
}

pub unsafe fn dc_delete_file(
    mut context: &dc_context_t,
    mut pathNfilename: *const libc::c_char,
) -> libc::c_int {
    let mut success: libc::c_int = 0i32;
    let mut pathNfilename_abs: *mut libc::c_char = 0 as *mut libc::c_char;
    pathNfilename_abs = dc_get_abs_path(context, pathNfilename);
    if !pathNfilename_abs.is_null() {
        if remove(pathNfilename_abs) != 0i32 {
            dc_log_warning(
                context,
                0i32,
                b"Cannot delete \"%s\".\x00" as *const u8 as *const libc::c_char,
                pathNfilename,
            );
        } else {
            success = 1i32
        }
    }
    free(pathNfilename_abs as *mut libc::c_void);
    return success;
}
pub unsafe fn dc_copy_file(
    mut context: &dc_context_t,
    mut src: *const libc::c_char,
    mut dest: *const libc::c_char,
) -> libc::c_int {
    let mut current_block: u64;
    let mut success: libc::c_int = 0i32;
    let mut src_abs: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut dest_abs: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut fd_src: libc::c_int = -1i32;
    let mut fd_dest: libc::c_int = -1i32;
    let mut buf: [libc::c_char; 4096] = [0; 4096];
    let mut bytes_read = 0;
    let mut anything_copied: libc::c_int = 0i32;
    src_abs = dc_get_abs_path(context, src);
    if !(src_abs.is_null() || {
        dest_abs = dc_get_abs_path(context, dest);
        dest_abs.is_null()
    }) {
        fd_src = open(src_abs, 0i32);
        if fd_src < 0i32 {
            dc_log_error(
                context,
                0i32,
                b"Cannot open source file \"%s\".\x00" as *const u8 as *const libc::c_char,
                src,
            );
        } else {
            fd_dest = open(dest_abs, 0x1i32 | 0x200i32 | 0x800i32, 0o666i32);
            if fd_dest < 0i32 {
                dc_log_error(
                    context,
                    0i32,
                    b"Cannot open destination file \"%s\".\x00" as *const u8 as *const libc::c_char,
                    dest,
                );
            } else {
                loop {
                    bytes_read = read(
                        fd_src,
                        buf.as_mut_ptr() as *mut libc::c_void,
                        4096i32 as size_t,
                    ) as size_t;
                    if !(bytes_read > 0) {
                        break;
                    }
                    if write(fd_dest, buf.as_mut_ptr() as *const libc::c_void, bytes_read)
                        != bytes_read as isize
                    {
                        dc_log_error(
                            context,
                            0i32,
                            b"Cannot write %i bytes to \"%s\".\x00" as *const u8
                                as *const libc::c_char,
                            bytes_read,
                            dest,
                        );
                    }
                    anything_copied = 1i32
                }
                if 0 == anything_copied {
                    close(fd_src);
                    fd_src = -1i32;
                    if dc_get_filebytes(context, src) != 0 {
                        dc_log_error(
                            context,
                            0i32,
                            b"Different size information for \"%s\".\x00" as *const u8
                                as *const libc::c_char,
                            bytes_read,
                            dest,
                        );
                        current_block = 610040589300051390;
                    } else {
                        current_block = 5634871135123216486;
                    }
                } else {
                    current_block = 5634871135123216486;
                }
                match current_block {
                    610040589300051390 => {}
                    _ => success = 1i32,
                }
            }
        }
    }
    if fd_src >= 0i32 {
        close(fd_src);
    }
    if fd_dest >= 0i32 {
        close(fd_dest);
    }
    free(src_abs as *mut libc::c_void);
    free(dest_abs as *mut libc::c_void);
    return success;
}
pub unsafe fn dc_create_folder(
    mut context: &dc_context_t,
    mut pathNfilename: *const libc::c_char,
) -> libc::c_int {
    let mut success = 0;
    let pathNfilename_abs = dc_get_abs_path(context, pathNfilename);
    {
        let p = std::path::Path::new(
            std::ffi::CStr::from_ptr(pathNfilename_abs)
                .to_str()
                .unwrap(),
        );
        if !p.exists() {
            if mkdir(pathNfilename_abs, 0o755i32 as mode_t) != 0i32 {
                dc_log_warning(
                    context,
                    0i32,
                    b"Cannot create directory \"%s\".\x00" as *const u8 as *const libc::c_char,
                    pathNfilename,
                );
            } else {
                success = 1;
            }
        } else {
            success = 1;
        }
    }

    free(pathNfilename_abs as *mut libc::c_void);
    success
}

pub unsafe fn dc_write_file(
    mut context: &dc_context_t,
    mut pathNfilename: *const libc::c_char,
    mut buf: *const libc::c_void,
    mut buf_bytes: size_t,
) -> libc::c_int {
    let mut f: *mut libc::FILE = 0 as *mut libc::FILE;
    let mut success: libc::c_int = 0i32;
    let mut pathNfilename_abs: *mut libc::c_char = 0 as *mut libc::c_char;
    pathNfilename_abs = dc_get_abs_path(context, pathNfilename);
    if !pathNfilename_abs.is_null() {
        f = fopen(
            pathNfilename_abs,
            b"wb\x00" as *const u8 as *const libc::c_char,
        );
        if !f.is_null() {
            if fwrite(buf, 1, buf_bytes, f) == buf_bytes {
                success = 1i32
            } else {
                dc_log_warning(
                    context,
                    0i32,
                    b"Cannot write %lu bytes to \"%s\".\x00" as *const u8 as *const libc::c_char,
                    buf_bytes as libc::c_ulong,
                    pathNfilename,
                );
            }
            fclose(f);
        } else {
            dc_log_warning(
                context,
                0i32,
                b"Cannot open \"%s\" for writing.\x00" as *const u8 as *const libc::c_char,
                pathNfilename,
            );
        }
    }
    free(pathNfilename_abs as *mut libc::c_void);
    return success;
}
pub unsafe fn dc_read_file(
    mut context: &dc_context_t,
    mut pathNfilename: *const libc::c_char,
    mut buf: *mut *mut libc::c_void,
    mut buf_bytes: *mut size_t,
) -> libc::c_int {
    let mut success: libc::c_int = 0i32;
    let mut pathNfilename_abs: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut f: *mut libc::FILE = 0 as *mut libc::FILE;
    if pathNfilename.is_null() || buf.is_null() || buf_bytes.is_null() {
        return 0i32;
    }
    *buf = 0 as *mut libc::c_void;
    *buf_bytes = 0i32 as size_t;
    pathNfilename_abs = dc_get_abs_path(context, pathNfilename);
    if !pathNfilename_abs.is_null() {
        f = fopen(
            pathNfilename_abs,
            b"rb\x00" as *const u8 as *const libc::c_char,
        );
        if !f.is_null() {
            fseek(f, 0, 2i32);
            *buf_bytes = ftell(f) as size_t;
            fseek(f, 0, 0i32);
            if !(*buf_bytes <= 0) {
                *buf = malloc((*buf_bytes).wrapping_add(1));
                if !(*buf).is_null() {
                    *(*buf as *mut libc::c_char).offset(*buf_bytes as isize) = 0i32 as libc::c_char;
                    if !(fread(*buf, 1, *buf_bytes, f) != *buf_bytes) {
                        success = 1i32
                    }
                }
            }
        }
    }
    if !f.is_null() {
        fclose(f);
    }
    if success == 0i32 {
        free(*buf);
        *buf = 0 as *mut libc::c_void;
        *buf_bytes = 0i32 as size_t;
        dc_log_warning(
            context,
            0i32,
            b"Cannot read \"%s\" or file is empty.\x00" as *const u8 as *const libc::c_char,
            pathNfilename,
        );
    }
    free(pathNfilename_abs as *mut libc::c_void);
    return success;
}
pub unsafe fn dc_get_fine_pathNfilename(
    mut context: &dc_context_t,
    mut pathNfolder: *const libc::c_char,
    mut desired_filenameNsuffix__: *const libc::c_char,
) -> *mut libc::c_char {
    let mut ret: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut pathNfolder_wo_slash: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut filenameNsuffix: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut basename: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut dotNSuffix: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut now: time_t = time(0 as *mut time_t);
    let mut i: libc::c_int = 0i32;
    pathNfolder_wo_slash = dc_strdup(pathNfolder);
    dc_ensure_no_slash(pathNfolder_wo_slash);
    filenameNsuffix = dc_strdup(desired_filenameNsuffix__);
    dc_validate_filename(filenameNsuffix);
    dc_split_filename(filenameNsuffix, &mut basename, &mut dotNSuffix);
    i = 0i32;
    while i < 1000i32 {
        /*no deadlocks, please*/
        if 0 != i {
            let mut idx: time_t = if i < 100i32 {
                i as libc::c_long
            } else {
                now + i as libc::c_long
            };
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
    return ret;
}
pub unsafe fn dc_is_blobdir_path(
    mut context: &dc_context_t,
    mut path: *const libc::c_char,
) -> libc::c_int {
    if strncmp(path, context.blobdir, strlen(context.blobdir)) == 0i32
        || strncmp(path, b"$BLOBDIR\x00" as *const u8 as *const libc::c_char, 8) == 0i32
    {
        return 1i32;
    }
    return 0i32;
}
pub unsafe fn dc_make_rel_path(mut context: &dc_context_t, mut path: *mut *mut libc::c_char) {
    if path.is_null() || (*path).is_null() {
        return;
    }
    if strncmp(*path, context.blobdir, strlen(context.blobdir)) == 0i32 {
        dc_str_replace(
            path,
            context.blobdir,
            b"$BLOBDIR\x00" as *const u8 as *const libc::c_char,
        );
    };
}
pub unsafe fn dc_make_rel_and_copy(
    mut context: &dc_context_t,
    mut path: *mut *mut libc::c_char,
) -> libc::c_int {
    let mut success: libc::c_int = 0i32;
    let mut filename: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut blobdir_path: *mut libc::c_char = 0 as *mut libc::c_char;
    if !(path.is_null() || (*path).is_null()) {
        if 0 != dc_is_blobdir_path(context, *path) {
            dc_make_rel_path(context, path);
            success = 1i32
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
                success = 1i32
            }
        }
    }
    free(blobdir_path as *mut libc::c_void);
    free(filename as *mut libc::c_void);
    return success;
}
