use crate::dc_dehtml::*;
use crate::dc_strbuilder::*;
use crate::dc_tools::*;
use crate::types::*;
use crate::x::*;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct dc_simplify_t {
    pub is_forwarded: libc::c_int,
    pub is_cut_at_begin: libc::c_int,
    pub is_cut_at_end: libc::c_int,
}

pub unsafe fn dc_simplify_new() -> *mut dc_simplify_t {
    let simplify: *mut dc_simplify_t;
    simplify = calloc(1, ::std::mem::size_of::<dc_simplify_t>()) as *mut dc_simplify_t;
    if simplify.is_null() {
        exit(31i32);
    }

    simplify
}

pub unsafe fn dc_simplify_unref(simplify: *mut dc_simplify_t) {
    if simplify.is_null() {
        return;
    }
    free(simplify as *mut libc::c_void);
}

/* Simplify and normalise text: Remove quotes, signatures, unnecessary
lineends etc.
The data returned from Simplify() must be free()'d when no longer used, private */
pub unsafe fn dc_simplify_simplify(
    mut simplify: *mut dc_simplify_t,
    in_unterminated: *const libc::c_char,
    in_bytes: libc::c_int,
    is_html: libc::c_int,
    is_msgrmsg: libc::c_int,
) -> *mut libc::c_char {
    /* create a copy of the given buffer */
    let mut out: *mut libc::c_char;
    let mut temp: *mut libc::c_char;
    if simplify.is_null() || in_unterminated.is_null() || in_bytes <= 0i32 {
        return dc_strdup(b"\x00" as *const u8 as *const libc::c_char);
    }
    (*simplify).is_forwarded = 0i32;
    (*simplify).is_cut_at_begin = 0i32;
    (*simplify).is_cut_at_end = 0i32;
    out = strndup(
        in_unterminated as *mut libc::c_char,
        in_bytes as libc::c_ulong,
    );
    if out.is_null() {
        return dc_strdup(b"\x00" as *const u8 as *const libc::c_char);
    }
    if 0 != is_html {
        temp = dc_dehtml(out);
        if !temp.is_null() {
            free(out as *mut libc::c_void);
            out = temp
        }
    }
    dc_remove_cr_chars(out);
    temp = dc_simplify_simplify_plain_text(simplify, out, is_msgrmsg);
    if !temp.is_null() {
        free(out as *mut libc::c_void);
        out = temp
    }
    dc_remove_cr_chars(out);

    out
}

/* ******************************************************************************
 * Simplify Plain Text
 ******************************************************************************/
unsafe fn dc_simplify_simplify_plain_text(
    mut simplify: *mut dc_simplify_t,
    buf_terminated: *const libc::c_char,
    is_msgrmsg: libc::c_int,
) -> *mut libc::c_char {
    /* This function ...
    ... removes all text after the line `-- ` (footer mark)
    ... removes full quotes at the beginning and at the end of the text -
        these are all lines starting with the character `>`
    ... remove a non-empty line before the removed quote (contains sth. like "On 2.9.2016, Bjoern wrote:" in different formats and lanugages) */
    /* split the given buffer into lines */
    let lines: *mut carray = dc_split_into_lines(buf_terminated);
    let mut l: libc::c_int;
    let mut l_first: libc::c_int = 0i32;
    /* if l_last is -1, there are no lines */
    let mut l_last: libc::c_int =
        carray_count(lines).wrapping_sub(1i32 as libc::c_uint) as libc::c_int;
    let mut line: *mut libc::c_char;
    let mut footer_mark: libc::c_int = 0i32;
    l = l_first;
    while l <= l_last {
        line = carray_get(lines, l as libc::c_uint) as *mut libc::c_char;
        if strcmp(line, b"-- \x00" as *const u8 as *const libc::c_char) == 0i32
            || strcmp(line, b"--  \x00" as *const u8 as *const libc::c_char) == 0i32
        {
            footer_mark = 1i32
        }
        if strcmp(line, b"--\x00" as *const u8 as *const libc::c_char) == 0i32
            || strcmp(line, b"---\x00" as *const u8 as *const libc::c_char) == 0i32
            || strcmp(line, b"----\x00" as *const u8 as *const libc::c_char) == 0i32
        {
            footer_mark = 1i32;
            (*simplify).is_cut_at_end = 1i32
        }
        if 0 != footer_mark {
            l_last = l - 1i32;
            /* done */
            break;
        } else {
            l += 1
        }
    }
    if l_last - l_first + 1i32 >= 3i32 {
        let line0: *mut libc::c_char =
            carray_get(lines, l_first as libc::c_uint) as *mut libc::c_char;
        let line1: *mut libc::c_char =
            carray_get(lines, (l_first + 1i32) as libc::c_uint) as *mut libc::c_char;
        let line2: *mut libc::c_char =
            carray_get(lines, (l_first + 2i32) as libc::c_uint) as *mut libc::c_char;
        if strcmp(
            line0,
            b"---------- Forwarded message ----------\x00" as *const u8 as *const libc::c_char,
        ) == 0i32
            && strncmp(line1, b"From: \x00" as *const u8 as *const libc::c_char, 6) == 0i32
            && *line2.offset(0isize) as libc::c_int == 0i32
        {
            (*simplify).is_forwarded = 1i32;
            l_first += 3i32
        }
    }
    l = l_first;
    while l <= l_last {
        line = carray_get(lines, l as libc::c_uint) as *mut libc::c_char;
        if strncmp(line, b"-----\x00" as *const u8 as *const libc::c_char, 5) == 0i32
            || strncmp(line, b"_____\x00" as *const u8 as *const libc::c_char, 5) == 0i32
            || strncmp(line, b"=====\x00" as *const u8 as *const libc::c_char, 5) == 0i32
            || strncmp(line, b"*****\x00" as *const u8 as *const libc::c_char, 5) == 0i32
            || strncmp(line, b"~~~~~\x00" as *const u8 as *const libc::c_char, 5) == 0i32
        {
            l_last = l - 1i32;
            (*simplify).is_cut_at_end = 1i32;
            /* done */
            break;
        } else {
            l += 1
        }
    }
    if 0 == is_msgrmsg {
        let mut l_lastQuotedLine: libc::c_int = -1i32;
        l = l_last;
        while l >= l_first {
            line = carray_get(lines, l as libc::c_uint) as *mut libc::c_char;
            if is_plain_quote(line) {
                l_lastQuotedLine = l
            } else if !is_empty_line(line) {
                break;
            }
            l -= 1
        }
        if l_lastQuotedLine != -1i32 {
            l_last = l_lastQuotedLine - 1i32;
            (*simplify).is_cut_at_end = 1i32;
            if l_last > 0i32 {
                if is_empty_line(carray_get(lines, l_last as libc::c_uint) as *mut libc::c_char) {
                    l_last -= 1
                }
            }
            if l_last > 0i32 {
                line = carray_get(lines, l_last as libc::c_uint) as *mut libc::c_char;
                if is_quoted_headline(line) {
                    l_last -= 1
                }
            }
        }
    }
    if 0 == is_msgrmsg {
        let mut l_lastQuotedLine_0: libc::c_int = -1i32;
        let mut hasQuotedHeadline: libc::c_int = 0i32;
        l = l_first;
        while l <= l_last {
            line = carray_get(lines, l as libc::c_uint) as *mut libc::c_char;
            if is_plain_quote(line) {
                l_lastQuotedLine_0 = l
            } else if !is_empty_line(line) {
                if is_quoted_headline(line) && 0 == hasQuotedHeadline && l_lastQuotedLine_0 == -1i32
                {
                    hasQuotedHeadline = 1i32
                } else {
                    /* non-quoting line found */
                    break;
                }
            }
            l += 1
        }
        if l_lastQuotedLine_0 != -1i32 {
            l_first = l_lastQuotedLine_0 + 1i32;
            (*simplify).is_cut_at_begin = 1i32
        }
    }
    /* re-create buffer from the remaining lines */
    let mut ret: dc_strbuilder_t = dc_strbuilder_t {
        buf: 0 as *mut libc::c_char,
        allocated: 0,
        free: 0,
        eos: 0 as *mut libc::c_char,
    };
    dc_strbuilder_init(&mut ret, strlen(buf_terminated) as libc::c_int);
    if 0 != (*simplify).is_cut_at_begin {
        dc_strbuilder_cat(&mut ret, b"[...] \x00" as *const u8 as *const libc::c_char);
    }
    /* we write empty lines only in case and non-empty line follows */
    let mut pending_linebreaks: libc::c_int = 0i32;
    let mut content_lines_added: libc::c_int = 0i32;
    l = l_first;
    while l <= l_last {
        line = carray_get(lines, l as libc::c_uint) as *mut libc::c_char;
        if is_empty_line(line) {
            pending_linebreaks += 1
        } else {
            if 0 != content_lines_added {
                if pending_linebreaks > 2i32 {
                    pending_linebreaks = 2i32
                }
                while 0 != pending_linebreaks {
                    dc_strbuilder_cat(&mut ret, b"\n\x00" as *const u8 as *const libc::c_char);
                    pending_linebreaks -= 1
                }
            }
            dc_strbuilder_cat(&mut ret, line);
            content_lines_added += 1;
            pending_linebreaks = 1i32
        }
        l += 1
    }
    if 0 != (*simplify).is_cut_at_end
        && (0 == (*simplify).is_cut_at_begin || 0 != content_lines_added)
    {
        dc_strbuilder_cat(&mut ret, b" [...]\x00" as *const u8 as *const libc::c_char);
    }
    dc_free_splitted_lines(lines);

    ret.buf
}

/* ******************************************************************************
 * Tools
 ******************************************************************************/
unsafe fn is_empty_line(buf: *const libc::c_char) -> bool {
    /* force unsigned - otherwise the `> ' '` comparison will fail */
    let mut p1: *const libc::c_uchar = buf as *const libc::c_uchar;
    while 0 != *p1 {
        if *p1 as libc::c_int > ' ' as i32 {
            return false;
        }
        p1 = p1.offset(1isize)
    }

    true
}

unsafe fn is_quoted_headline(buf: *const libc::c_char) -> bool {
    /* This function may be called for the line _directly_ before a quote.
    The function checks if the line contains sth. like "On 01.02.2016, xy@z wrote:" in various languages.
    - Currently, we simply check if the last character is a ':'.
    - Checking for the existance of an email address may fail (headlines may show the user's name instead of the address) */
    let buf_len: libc::c_int = strlen(buf) as libc::c_int;
    if buf_len > 80i32 {
        return false;
    }
    if buf_len > 0i32 && *buf.offset((buf_len - 1i32) as isize) as libc::c_int == ':' as i32 {
        return true;
    }

    false
}

unsafe fn is_plain_quote(buf: *const libc::c_char) -> bool {
    if *buf.offset(0isize) as libc::c_int == '>' as i32 {
        return true;
    }

    false
}
