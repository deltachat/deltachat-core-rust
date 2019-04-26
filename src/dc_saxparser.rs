use libc;

use crate::dc_tools::*;
use crate::types::*;
use crate::x::*;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct dc_saxparser_t {
    pub starttag_cb: dc_saxparser_starttag_cb_t,
    pub endtag_cb: dc_saxparser_endtag_cb_t,
    pub text_cb: dc_saxparser_text_cb_t,
    pub userdata: *mut libc::c_void,
}

/* len is only informational, text is already null-terminated */
pub type dc_saxparser_text_cb_t = Option<
    unsafe extern "C" fn(_: *mut libc::c_void, _: *const libc::c_char, _: libc::c_int) -> (),
>;
pub type dc_saxparser_endtag_cb_t =
    Option<unsafe extern "C" fn(_: *mut libc::c_void, _: *const libc::c_char) -> ()>;
pub type dc_saxparser_starttag_cb_t = Option<
    unsafe extern "C" fn(
        _: *mut libc::c_void,
        _: *const libc::c_char,
        _: *mut *mut libc::c_char,
    ) -> (),
>;

#[inline]
unsafe extern "C" fn isascii(mut _c: libc::c_int) -> libc::c_int {
    return (_c & !0x7fi32 == 0i32) as libc::c_int;
}
#[inline]
unsafe extern "C" fn __istype(mut _c: __darwin_ct_rune_t, mut _f: libc::c_ulong) -> libc::c_int {
    return if 0 != isascii(_c) {
        (0 != _DefaultRuneLocale.__runetype[_c as usize] as libc::c_ulong & _f) as libc::c_int
    } else {
        (0 != __maskrune(_c, _f)) as libc::c_int
    };
}

#[no_mangle]
pub unsafe extern "C" fn dc_saxparser_init(
    mut saxparser: *mut dc_saxparser_t,
    mut userdata: *mut libc::c_void,
) {
    (*saxparser).userdata = userdata;
    (*saxparser).starttag_cb = Some(def_starttag_cb);
    (*saxparser).endtag_cb = Some(def_endtag_cb);
    (*saxparser).text_cb = Some(def_text_cb);
}
unsafe extern "C" fn def_text_cb(
    mut userdata: *mut libc::c_void,
    mut text: *const libc::c_char,
    mut len: libc::c_int,
) {
}
unsafe extern "C" fn def_endtag_cb(mut userdata: *mut libc::c_void, mut tag: *const libc::c_char) {}
/* ******************************************************************************
 * Tools
 ******************************************************************************/
unsafe extern "C" fn def_starttag_cb(
    mut userdata: *mut libc::c_void,
    mut tag: *const libc::c_char,
    mut attr: *mut *mut libc::c_char,
) {
}
#[no_mangle]
pub unsafe extern "C" fn dc_saxparser_set_tag_handler(
    mut saxparser: *mut dc_saxparser_t,
    mut starttag_cb: dc_saxparser_starttag_cb_t,
    mut endtag_cb: dc_saxparser_endtag_cb_t,
) {
    if saxparser.is_null() {
        return;
    }
    (*saxparser).starttag_cb = if starttag_cb.is_some() {
        starttag_cb
    } else {
        Some(def_starttag_cb)
    };
    (*saxparser).endtag_cb = if endtag_cb.is_some() {
        endtag_cb
    } else {
        Some(def_endtag_cb)
    };
}
#[no_mangle]
pub unsafe extern "C" fn dc_saxparser_set_text_handler(
    mut saxparser: *mut dc_saxparser_t,
    mut text_cb: dc_saxparser_text_cb_t,
) {
    if saxparser.is_null() {
        return;
    }
    (*saxparser).text_cb = if text_cb.is_some() {
        text_cb
    } else {
        Some(def_text_cb)
    };
}
#[no_mangle]
pub unsafe extern "C" fn dc_saxparser_parse(
    mut saxparser: *mut dc_saxparser_t,
    mut buf_start__: *const libc::c_char,
) {
    let mut current_block: u64;
    let mut bak: libc::c_char = 0i32 as libc::c_char;
    let mut buf_start: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut last_text_start: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut p: *mut libc::c_char = 0 as *mut libc::c_char;
    /* attributes per tag - a fixed border here is a security feature, not a limit */
    /* attributes as key/value pairs, +1 for terminating the list */
    let mut attr: [*mut libc::c_char; 202] = [0 as *mut libc::c_char; 202];
    /* free the value at attr[i*2+1]? */
    let mut free_attr: [libc::c_int; 100] = [0; 100];
    attr[0usize] = 0 as *mut libc::c_char;
    if saxparser.is_null() {
        return;
    }
    buf_start = dc_strdup(buf_start__);
    last_text_start = buf_start;
    p = buf_start;
    loop {
        if !(0 != *p) {
            current_block = 13425230902034816933;
            break;
        }
        if *p as libc::c_int == '<' as i32 {
            call_text_cb(
                saxparser,
                last_text_start,
                p.wrapping_offset_from(last_text_start) as libc::c_long as size_t,
                '&' as i32 as libc::c_char,
            );
            p = p.offset(1isize);
            if strncmp(
                p,
                b"!--\x00" as *const u8 as *const libc::c_char,
                3i32 as libc::c_ulong,
            ) == 0i32
            {
                p = strstr(p, b"-->\x00" as *const u8 as *const libc::c_char);
                if p.is_null() {
                    current_block = 7627180618761592946;
                    break;
                }
                p = p.offset(3isize)
            } else if strncmp(
                p,
                b"![CDATA[\x00" as *const u8 as *const libc::c_char,
                8i32 as libc::c_ulong,
            ) == 0i32
            {
                /* process <![CDATA[ ... ]]> text
                 **************************************************************/
                let mut text_beg: *mut libc::c_char = p.offset(8isize);
                p = strstr(p, b"]]>\x00" as *const u8 as *const libc::c_char);
                if !p.is_null() {
                    call_text_cb(
                        saxparser,
                        text_beg,
                        p.wrapping_offset_from(text_beg) as libc::c_long as size_t,
                        'c' as i32 as libc::c_char,
                    );
                    p = p.offset(3isize)
                } else {
                    call_text_cb(
                        saxparser,
                        text_beg,
                        strlen(text_beg),
                        'c' as i32 as libc::c_char,
                    );
                    current_block = 7627180618761592946;
                    break;
                }
            } else if strncmp(
                p,
                b"!DOCTYPE\x00" as *const u8 as *const libc::c_char,
                8i32 as libc::c_ulong,
            ) == 0i32
            {
                while 0 != *p as libc::c_int
                    && *p as libc::c_int != '[' as i32
                    && *p as libc::c_int != '>' as i32
                {
                    p = p.offset(1isize)
                }
                if *p as libc::c_int == 0i32 {
                    /* unclosed doctype */
                    current_block = 7627180618761592946;
                    break;
                } else if *p as libc::c_int == '[' as i32 {
                    p = strstr(p, b"]>\x00" as *const u8 as *const libc::c_char);
                    if p.is_null() {
                        /* unclosed inline doctype */
                        current_block = 7627180618761592946;
                        break;
                    } else {
                        p = p.offset(2isize)
                    }
                } else {
                    p = p.offset(1isize)
                }
            } else if *p as libc::c_int == '?' as i32 {
                p = strstr(p, b"?>\x00" as *const u8 as *const libc::c_char);
                if p.is_null() {
                    /* unclosed processing instruction */
                    current_block = 7627180618761592946;
                    break;
                } else {
                    p = p.offset(2isize)
                }
            } else {
                p = p
                    .offset(strspn(p, b"\t\r\n \x00" as *const u8 as *const libc::c_char) as isize);
                if *p as libc::c_int == '/' as i32 {
                    p = p.offset(1isize);
                    p = p.offset(
                        strspn(p, b"\t\r\n \x00" as *const u8 as *const libc::c_char) as isize,
                    );
                    let mut beg_tag_name: *mut libc::c_char = p;
                    p = p.offset(
                        strcspn(p, b"\t\r\n />\x00" as *const u8 as *const libc::c_char) as isize,
                    );
                    if p != beg_tag_name {
                        bak = *p;
                        *p = '\u{0}' as i32 as libc::c_char;
                        dc_strlower_in_place(beg_tag_name);
                        (*saxparser).endtag_cb.expect("non-null function pointer")(
                            (*saxparser).userdata,
                            beg_tag_name,
                        );
                        *p = bak
                    }
                } else {
                    do_free_attr(attr.as_mut_ptr(), free_attr.as_mut_ptr());
                    let mut beg_tag_name_0: *mut libc::c_char = p;
                    p = p.offset(
                        strcspn(p, b"\t\r\n />\x00" as *const u8 as *const libc::c_char) as isize,
                    );
                    if p != beg_tag_name_0 {
                        let mut after_tag_name: *mut libc::c_char = p;
                        let mut attr_index: libc::c_int = 0i32;
                        while 0 != isspace(*p as libc::c_int) {
                            p = p.offset(1isize)
                        }
                        while 0 != *p as libc::c_int
                            && *p as libc::c_int != '/' as i32
                            && *p as libc::c_int != '>' as i32
                        {
                            let mut beg_attr_name: *mut libc::c_char = p;
                            let mut beg_attr_value: *mut libc::c_char = 0 as *mut libc::c_char;
                            let mut beg_attr_value_new: *mut libc::c_char = 0 as *mut libc::c_char;
                            if '=' as i32 == *beg_attr_name as libc::c_int {
                                p = p.offset(1isize)
                            } else {
                                p = p.offset(strcspn(
                                    p,
                                    b"\t\r\n =/>\x00" as *const u8 as *const libc::c_char,
                                ) as isize);
                                if p != beg_attr_name {
                                    let mut after_attr_name: *mut libc::c_char = p;
                                    p = p.offset(strspn(
                                        p,
                                        b"\t\r\n \x00" as *const u8 as *const libc::c_char,
                                    ) as isize);
                                    if *p as libc::c_int == '=' as i32 {
                                        p = p.offset(strspn(
                                            p,
                                            b"\t\r\n =\x00" as *const u8 as *const libc::c_char,
                                        )
                                            as isize);
                                        let mut quote: libc::c_char = *p;
                                        if quote as libc::c_int == '\"' as i32
                                            || quote as libc::c_int == '\'' as i32
                                        {
                                            p = p.offset(1isize);
                                            beg_attr_value = p;
                                            while 0 != *p as libc::c_int
                                                && *p as libc::c_int != quote as libc::c_int
                                            {
                                                p = p.offset(1isize)
                                            }
                                            if 0 != *p {
                                                *p = '\u{0}' as i32 as libc::c_char;
                                                p = p.offset(1isize)
                                            }
                                            beg_attr_value_new = xml_decode(
                                                beg_attr_value,
                                                ' ' as i32 as libc::c_char,
                                            )
                                        } else {
                                            beg_attr_value = p;
                                            p = p.offset(strcspn(
                                                p,
                                                b"\t\r\n />\x00" as *const u8
                                                    as *const libc::c_char,
                                            )
                                                as isize);
                                            bak = *p;
                                            *p = '\u{0}' as i32 as libc::c_char;
                                            let mut temp: *mut libc::c_char =
                                                dc_strdup(beg_attr_value);
                                            beg_attr_value_new =
                                                xml_decode(temp, ' ' as i32 as libc::c_char);
                                            if beg_attr_value_new != temp {
                                                free(temp as *mut libc::c_void);
                                            }
                                            *p = bak
                                        }
                                    } else {
                                        beg_attr_value_new = dc_strdup(0 as *const libc::c_char)
                                    }
                                    if attr_index < 100i32 {
                                        let mut beg_attr_name_new: *mut libc::c_char =
                                            beg_attr_name;
                                        let mut free_bits: libc::c_int =
                                            if beg_attr_value_new != beg_attr_value {
                                                0x2i32
                                            } else {
                                                0i32
                                            };
                                        if after_attr_name == p {
                                            bak = *after_attr_name;
                                            *after_attr_name = '\u{0}' as i32 as libc::c_char;
                                            beg_attr_name_new = dc_strdup(beg_attr_name);
                                            *after_attr_name = bak;
                                            free_bits |= 0x1i32
                                        } else {
                                            *after_attr_name = '\u{0}' as i32 as libc::c_char
                                        }
                                        dc_strlower_in_place(beg_attr_name_new);
                                        attr[attr_index as usize] = beg_attr_name_new;
                                        attr[(attr_index + 1i32) as usize] = beg_attr_value_new;
                                        attr[(attr_index + 2i32) as usize] = 0 as *mut libc::c_char;
                                        free_attr[(attr_index >> 1i32) as usize] = free_bits;
                                        attr_index += 2i32
                                    }
                                }
                                while 0 != isspace(*p as libc::c_int) {
                                    p = p.offset(1isize)
                                }
                            }
                        }
                        let mut bak_0: libc::c_char = *after_tag_name;
                        *after_tag_name = 0i32 as libc::c_char;
                        dc_strlower_in_place(beg_tag_name_0);
                        (*saxparser).starttag_cb.expect("non-null function pointer")(
                            (*saxparser).userdata,
                            beg_tag_name_0,
                            attr.as_mut_ptr(),
                        );
                        *after_tag_name = bak_0;
                        p = p.offset(
                            strspn(p, b"\t\r\n \x00" as *const u8 as *const libc::c_char) as isize,
                        );
                        if *p as libc::c_int == '/' as i32 {
                            p = p.offset(1isize);
                            *after_tag_name = 0i32 as libc::c_char;
                            (*saxparser).endtag_cb.expect("non-null function pointer")(
                                (*saxparser).userdata,
                                beg_tag_name_0,
                            );
                        }
                    }
                }
                p = strchr(p, '>' as i32);
                if p.is_null() {
                    /* unclosed start-tag or end-tag */
                    current_block = 7627180618761592946;
                    break;
                } else {
                    p = p.offset(1isize)
                }
            }
            last_text_start = p
        } else {
            p = p.offset(1isize)
        }
    }
    match current_block {
        13425230902034816933 => {
            call_text_cb(
                saxparser,
                last_text_start,
                p.wrapping_offset_from(last_text_start) as libc::c_long as size_t,
                '&' as i32 as libc::c_char,
            );
        }
        _ => {}
    }
    do_free_attr(attr.as_mut_ptr(), free_attr.as_mut_ptr());
    free(buf_start as *mut libc::c_void);
}
unsafe extern "C" fn do_free_attr(
    mut attr: *mut *mut libc::c_char,
    mut free_attr: *mut libc::c_int,
) {
    /* "attr" are key/value pairs; the function frees the data if the corresponding bit in "free_attr" is set.
    (we need this as we try to use the strings from the "main" document instead of allocating small strings) */
    let mut i: libc::c_int = 0i32;
    while !(*attr.offset(i as isize)).is_null() {
        if 0 != *free_attr.offset((i >> 1i32) as isize) & 0x1i32
            && !(*attr.offset(i as isize)).is_null()
        {
            free(*attr.offset(i as isize) as *mut libc::c_void);
        }
        if 0 != *free_attr.offset((i >> 1i32) as isize) & 0x2i32
            && !(*attr.offset((i + 1i32) as isize)).is_null()
        {
            free(*attr.offset((i + 1i32) as isize) as *mut libc::c_void);
        }
        i += 2i32
    }
    let ref mut fresh0 = *attr.offset(0isize);
    *fresh0 = 0 as *mut libc::c_char;
}
unsafe extern "C" fn call_text_cb(
    mut saxparser: *mut dc_saxparser_t,
    mut text: *mut libc::c_char,
    mut len: size_t,
    mut type_0: libc::c_char,
) {
    if !text.is_null() && 0 != len {
        let mut bak: libc::c_char = *text.offset(len as isize);
        let mut text_new: *mut libc::c_char = 0 as *mut libc::c_char;
        *text.offset(len as isize) = '\u{0}' as i32 as libc::c_char;
        text_new = xml_decode(text, type_0);
        (*saxparser).text_cb.expect("non-null function pointer")(
            (*saxparser).userdata,
            text_new,
            len as libc::c_int,
        );
        if text != text_new {
            free(text_new as *mut libc::c_void);
        }
        *text.offset(len as isize) = bak
    };
}
/* Convert entities as &auml; to UTF-8 characters.

- The first strings MUST NOT start with `&` and MUST end with `;`.
- take care not to miss a comma between the strings.
- It's also possible to specify the destination as a character reference as `&#34;` (they are converted in a second pass without a table). */
/* basic XML/HTML */
/* advanced HTML */
/* MUST be last */
/* Recursively decodes entity and character references and normalizes new lines.
set "type" to ...
'&' for general entity decoding,
'%' for parameter entity decoding (currently not needed),
'c' for cdata sections,
' ' for attribute normalization, or
'*' for non-cdata attribute normalization (currently not needed).
Returns s, or if the decoded string is longer than s, returns a malloced string
that must be freed.
Function based upon ezxml_decode() from the "ezxml" parser which is
Copyright 2004-2006 Aaron Voisine <aaron@voisine.org> */
unsafe extern "C" fn xml_decode(
    mut s: *mut libc::c_char,
    mut type_0: libc::c_char,
) -> *mut libc::c_char {
    let mut e: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut r: *mut libc::c_char = s;
    let mut original_buf: *const libc::c_char = s;
    let mut b: libc::c_long = 0i32 as libc::c_long;
    let mut c: libc::c_long = 0i32 as libc::c_long;
    let mut d: libc::c_long = 0i32 as libc::c_long;
    let mut l: libc::c_long = 0i32 as libc::c_long;
    while 0 != *s {
        while *s as libc::c_int == '\r' as i32 {
            let fresh1 = s;
            s = s.offset(1);
            *fresh1 = '\n' as i32 as libc::c_char;
            if *s as libc::c_int == '\n' as i32 {
                memmove(
                    s as *mut libc::c_void,
                    s.offset(1isize) as *const libc::c_void,
                    strlen(s),
                );
            }
        }
        s = s.offset(1isize)
    }
    s = r;
    loop {
        while 0 != *s as libc::c_int
            && *s as libc::c_int != '&' as i32
            && 0 == isspace(*s as libc::c_int)
        {
            s = s.offset(1isize)
        }
        if 0 == *s {
            break;
        }
        if type_0 as libc::c_int != 'c' as i32
            && 0 == strncmp(
                s,
                b"&#\x00" as *const u8 as *const libc::c_char,
                2i32 as libc::c_ulong,
            )
        {
            if *s.offset(2isize) as libc::c_int == 'x' as i32 {
                c = strtol(s.offset(3isize), &mut e, 16i32)
            } else {
                c = strtol(s.offset(2isize), &mut e, 10i32)
            }
            if 0 == c || *e as libc::c_int != ';' as i32 {
                s = s.offset(1isize)
            } else {
                /* not a character ref */
                if c < 0x80i32 as libc::c_long {
                    let fresh2 = s;
                    s = s.offset(1);
                    *fresh2 = c as libc::c_char
                } else {
                    b = 0i32 as libc::c_long;
                    d = c;
                    while 0 != d {
                        b += 1;
                        d /= 2i32 as libc::c_long
                    }
                    b = (b - 2i32 as libc::c_long) / 5i32 as libc::c_long;
                    let fresh3 = s;
                    s = s.offset(1);
                    *fresh3 = ((0xffi32 << 7i32 as libc::c_long - b) as libc::c_long
                        | c >> 6i32 as libc::c_long * b)
                        as libc::c_char;
                    while 0 != b {
                        let fresh4 = s;
                        s = s.offset(1);
                        b -= 1;
                        *fresh4 = (0x80i32 as libc::c_long
                            | c >> 6i32 as libc::c_long * b & 0x3fi32 as libc::c_long)
                            as libc::c_char
                    }
                }
                memmove(
                    s as *mut libc::c_void,
                    strchr(s, ';' as i32).offset(1isize) as *const libc::c_void,
                    strlen(strchr(s, ';' as i32)),
                );
            }
        } else if *s as libc::c_int == '&' as i32
            && (type_0 as libc::c_int == '&' as i32 || type_0 as libc::c_int == ' ' as i32)
        {
            b = 0i32 as libc::c_long;
            while !s_ent[b as usize].is_null()
                && 0 != strncmp(
                    s.offset(1isize),
                    s_ent[b as usize],
                    strlen(s_ent[b as usize]),
                )
            {
                b += 2i32 as libc::c_long
            }
            let fresh5 = b;
            b = b + 1;
            if !s_ent[fresh5 as usize].is_null() {
                c = strlen(s_ent[b as usize]) as libc::c_long;
                e = strchr(s, ';' as i32);
                if c - 1i32 as libc::c_long > e.wrapping_offset_from(s) as libc::c_long {
                    d = s.wrapping_offset_from(r) as libc::c_long;
                    l = ((d + c) as libc::c_ulong).wrapping_add(strlen(e)) as libc::c_long;
                    if r == original_buf as *mut libc::c_char {
                        let mut new_ret: *mut libc::c_char =
                            malloc(l as libc::c_ulong) as *mut libc::c_char;
                        if new_ret.is_null() {
                            return r;
                        }
                        strcpy(new_ret, r);
                        r = new_ret
                    } else {
                        let mut new_ret_0: *mut libc::c_char =
                            realloc(r as *mut libc::c_void, l as libc::c_ulong)
                                as *mut libc::c_char;
                        if new_ret_0.is_null() {
                            return r;
                        }
                        r = new_ret_0
                    }
                    s = r.offset(d as isize);
                    e = strchr(s, ';' as i32)
                }
                memmove(
                    s.offset(c as isize) as *mut libc::c_void,
                    e.offset(1isize) as *const libc::c_void,
                    strlen(e),
                );
                strncpy(s, s_ent[b as usize], c as libc::c_ulong);
            } else {
                s = s.offset(1isize)
            }
        } else if type_0 as libc::c_int == ' ' as i32 && 0 != isspace(*s as libc::c_int) {
            let fresh6 = s;
            s = s.offset(1);
            *fresh6 = ' ' as i32 as libc::c_char
        } else {
            s = s.offset(1isize)
        }
    }
    return r;
}
/* dc_saxparser_t parses XML and HTML files that may not be wellformed
and spits out all text and tags found.

- Attributes are recognized with single, double or no quotes
- Whitespace ignored inside tags
- Self-closing tags are issued as open-tag plus close-tag
- CDATA is supoorted; DTA, comments, processing instruction are
  skipped properly
- The parser does not care about hierarchy, if needed this can be
  done by the user.
- Input and output strings must be UTF-8 encoded.
- Tag and attribute names are converted to lower case.
- Parsing does not stop on errors; instead errors are recovered.

NB: SAX = Simple API for XML */
/* ******************************************************************************
 * Decoding text
 ******************************************************************************/
static mut s_ent: [*const libc::c_char; 508] = [
    b"lt;\x00" as *const u8 as *const libc::c_char,
    b"<\x00" as *const u8 as *const libc::c_char,
    b"gt;\x00" as *const u8 as *const libc::c_char,
    b">\x00" as *const u8 as *const libc::c_char,
    b"quot;\x00" as *const u8 as *const libc::c_char,
    b"\"\x00" as *const u8 as *const libc::c_char,
    b"apos;\x00" as *const u8 as *const libc::c_char,
    b"\'\x00" as *const u8 as *const libc::c_char,
    b"amp;\x00" as *const u8 as *const libc::c_char,
    b"&\x00" as *const u8 as *const libc::c_char,
    b"nbsp;\x00" as *const u8 as *const libc::c_char,
    b" \x00" as *const u8 as *const libc::c_char,
    b"iexcl;\x00" as *const u8 as *const libc::c_char,
    b"\xc2\xa1\x00" as *const u8 as *const libc::c_char,
    b"cent;\x00" as *const u8 as *const libc::c_char,
    b"\xc2\xa2\x00" as *const u8 as *const libc::c_char,
    b"pound;\x00" as *const u8 as *const libc::c_char,
    b"\xc2\xa3\x00" as *const u8 as *const libc::c_char,
    b"curren;\x00" as *const u8 as *const libc::c_char,
    b"\xc2\xa4\x00" as *const u8 as *const libc::c_char,
    b"yen;\x00" as *const u8 as *const libc::c_char,
    b"\xc2\xa5\x00" as *const u8 as *const libc::c_char,
    b"brvbar;\x00" as *const u8 as *const libc::c_char,
    b"\xc2\xa6\x00" as *const u8 as *const libc::c_char,
    b"sect;\x00" as *const u8 as *const libc::c_char,
    b"\xc2\xa7\x00" as *const u8 as *const libc::c_char,
    b"uml;\x00" as *const u8 as *const libc::c_char,
    b"\xc2\xa8\x00" as *const u8 as *const libc::c_char,
    b"copy;\x00" as *const u8 as *const libc::c_char,
    b"\xc2\xa9\x00" as *const u8 as *const libc::c_char,
    b"ordf;\x00" as *const u8 as *const libc::c_char,
    b"\xc2\xaa\x00" as *const u8 as *const libc::c_char,
    b"laquo;\x00" as *const u8 as *const libc::c_char,
    b"\xc2\xab\x00" as *const u8 as *const libc::c_char,
    b"not;\x00" as *const u8 as *const libc::c_char,
    b"\xc2\xac\x00" as *const u8 as *const libc::c_char,
    b"shy;\x00" as *const u8 as *const libc::c_char,
    b"-\x00" as *const u8 as *const libc::c_char,
    b"reg;\x00" as *const u8 as *const libc::c_char,
    b"\xc2\xae\x00" as *const u8 as *const libc::c_char,
    b"macr;\x00" as *const u8 as *const libc::c_char,
    b"\xc2\xaf\x00" as *const u8 as *const libc::c_char,
    b"deg;\x00" as *const u8 as *const libc::c_char,
    b"\xc2\xb0\x00" as *const u8 as *const libc::c_char,
    b"plusmn;\x00" as *const u8 as *const libc::c_char,
    b"\xc2\xb1\x00" as *const u8 as *const libc::c_char,
    b"sup2;\x00" as *const u8 as *const libc::c_char,
    b"\xc2\xb2\x00" as *const u8 as *const libc::c_char,
    b"sup3;\x00" as *const u8 as *const libc::c_char,
    b"\xc2\xb3\x00" as *const u8 as *const libc::c_char,
    b"acute;\x00" as *const u8 as *const libc::c_char,
    b"\xc2\xb4\x00" as *const u8 as *const libc::c_char,
    b"micro;\x00" as *const u8 as *const libc::c_char,
    b"\xc2\xb5\x00" as *const u8 as *const libc::c_char,
    b"para;\x00" as *const u8 as *const libc::c_char,
    b"\xc2\xb6\x00" as *const u8 as *const libc::c_char,
    b"middot;\x00" as *const u8 as *const libc::c_char,
    b"\xc2\xb7\x00" as *const u8 as *const libc::c_char,
    b"cedil;\x00" as *const u8 as *const libc::c_char,
    b"\xc2\xb8\x00" as *const u8 as *const libc::c_char,
    b"sup1;\x00" as *const u8 as *const libc::c_char,
    b"\xc2\xb9\x00" as *const u8 as *const libc::c_char,
    b"ordm;\x00" as *const u8 as *const libc::c_char,
    b"\xc2\xba\x00" as *const u8 as *const libc::c_char,
    b"raquo;\x00" as *const u8 as *const libc::c_char,
    b"\xc2\xbb\x00" as *const u8 as *const libc::c_char,
    b"frac14;\x00" as *const u8 as *const libc::c_char,
    b"\xc2\xbc\x00" as *const u8 as *const libc::c_char,
    b"frac12;\x00" as *const u8 as *const libc::c_char,
    b"\xc2\xbd\x00" as *const u8 as *const libc::c_char,
    b"frac34;\x00" as *const u8 as *const libc::c_char,
    b"\xc2\xbe\x00" as *const u8 as *const libc::c_char,
    b"iquest;\x00" as *const u8 as *const libc::c_char,
    b"\xc2\xbf\x00" as *const u8 as *const libc::c_char,
    b"Agrave;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\x80\x00" as *const u8 as *const libc::c_char,
    b"Aacute;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\x81\x00" as *const u8 as *const libc::c_char,
    b"Acirc;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\x82\x00" as *const u8 as *const libc::c_char,
    b"Atilde;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\x83\x00" as *const u8 as *const libc::c_char,
    b"Auml;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\x84\x00" as *const u8 as *const libc::c_char,
    b"Aring;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\x85\x00" as *const u8 as *const libc::c_char,
    b"AElig;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\x86\x00" as *const u8 as *const libc::c_char,
    b"Ccedil;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\x87\x00" as *const u8 as *const libc::c_char,
    b"Egrave;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\x88\x00" as *const u8 as *const libc::c_char,
    b"Eacute;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\x89\x00" as *const u8 as *const libc::c_char,
    b"Ecirc;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\x8a\x00" as *const u8 as *const libc::c_char,
    b"Euml;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\x8b\x00" as *const u8 as *const libc::c_char,
    b"Igrave;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\x8c\x00" as *const u8 as *const libc::c_char,
    b"Iacute;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\x8d\x00" as *const u8 as *const libc::c_char,
    b"Icirc;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\x8e\x00" as *const u8 as *const libc::c_char,
    b"Iuml;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\x8f\x00" as *const u8 as *const libc::c_char,
    b"ETH;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\x90\x00" as *const u8 as *const libc::c_char,
    b"Ntilde;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\x91\x00" as *const u8 as *const libc::c_char,
    b"Ograve;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\x92\x00" as *const u8 as *const libc::c_char,
    b"Oacute;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\x93\x00" as *const u8 as *const libc::c_char,
    b"Ocirc;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\x94\x00" as *const u8 as *const libc::c_char,
    b"Otilde;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\x95\x00" as *const u8 as *const libc::c_char,
    b"Ouml;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\x96\x00" as *const u8 as *const libc::c_char,
    b"times;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\x97\x00" as *const u8 as *const libc::c_char,
    b"Oslash;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\x98\x00" as *const u8 as *const libc::c_char,
    b"Ugrave;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\x99\x00" as *const u8 as *const libc::c_char,
    b"Uacute;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\x9a\x00" as *const u8 as *const libc::c_char,
    b"Ucirc;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\x9b\x00" as *const u8 as *const libc::c_char,
    b"Uuml;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\x9c\x00" as *const u8 as *const libc::c_char,
    b"Yacute;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\x9d\x00" as *const u8 as *const libc::c_char,
    b"THORN;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\x9e\x00" as *const u8 as *const libc::c_char,
    b"szlig;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\x9f\x00" as *const u8 as *const libc::c_char,
    b"agrave;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\xa0\x00" as *const u8 as *const libc::c_char,
    b"aacute;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\xa1\x00" as *const u8 as *const libc::c_char,
    b"acirc;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\xa2\x00" as *const u8 as *const libc::c_char,
    b"atilde;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\xa3\x00" as *const u8 as *const libc::c_char,
    b"auml;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\xa4\x00" as *const u8 as *const libc::c_char,
    b"aring;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\xa5\x00" as *const u8 as *const libc::c_char,
    b"aelig;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\xa6\x00" as *const u8 as *const libc::c_char,
    b"ccedil;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\xa7\x00" as *const u8 as *const libc::c_char,
    b"egrave;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\xa8\x00" as *const u8 as *const libc::c_char,
    b"eacute;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\xa9\x00" as *const u8 as *const libc::c_char,
    b"ecirc;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\xaa\x00" as *const u8 as *const libc::c_char,
    b"euml;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\xab\x00" as *const u8 as *const libc::c_char,
    b"igrave;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\xac\x00" as *const u8 as *const libc::c_char,
    b"iacute;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\xad\x00" as *const u8 as *const libc::c_char,
    b"icirc;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\xae\x00" as *const u8 as *const libc::c_char,
    b"iuml;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\xaf\x00" as *const u8 as *const libc::c_char,
    b"eth;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\xb0\x00" as *const u8 as *const libc::c_char,
    b"ntilde;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\xb1\x00" as *const u8 as *const libc::c_char,
    b"ograve;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\xb2\x00" as *const u8 as *const libc::c_char,
    b"oacute;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\xb3\x00" as *const u8 as *const libc::c_char,
    b"ocirc;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\xb4\x00" as *const u8 as *const libc::c_char,
    b"otilde;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\xb5\x00" as *const u8 as *const libc::c_char,
    b"ouml;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\xb6\x00" as *const u8 as *const libc::c_char,
    b"divide;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\xb7\x00" as *const u8 as *const libc::c_char,
    b"oslash;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\xb8\x00" as *const u8 as *const libc::c_char,
    b"ugrave;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\xb9\x00" as *const u8 as *const libc::c_char,
    b"uacute;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\xba\x00" as *const u8 as *const libc::c_char,
    b"ucirc;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\xbb\x00" as *const u8 as *const libc::c_char,
    b"uuml;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\xbc\x00" as *const u8 as *const libc::c_char,
    b"yacute;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\xbd\x00" as *const u8 as *const libc::c_char,
    b"thorn;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\xbe\x00" as *const u8 as *const libc::c_char,
    b"yuml;\x00" as *const u8 as *const libc::c_char,
    b"\xc3\xbf\x00" as *const u8 as *const libc::c_char,
    b"OElig;\x00" as *const u8 as *const libc::c_char,
    b"\xc5\x92\x00" as *const u8 as *const libc::c_char,
    b"oelig;\x00" as *const u8 as *const libc::c_char,
    b"\xc5\x93\x00" as *const u8 as *const libc::c_char,
    b"Scaron;\x00" as *const u8 as *const libc::c_char,
    b"\xc5\xa0\x00" as *const u8 as *const libc::c_char,
    b"scaron;\x00" as *const u8 as *const libc::c_char,
    b"\xc5\xa1\x00" as *const u8 as *const libc::c_char,
    b"Yuml;\x00" as *const u8 as *const libc::c_char,
    b"\xc5\xb8\x00" as *const u8 as *const libc::c_char,
    b"fnof;\x00" as *const u8 as *const libc::c_char,
    b"\xc6\x92\x00" as *const u8 as *const libc::c_char,
    b"circ;\x00" as *const u8 as *const libc::c_char,
    b"\xcb\x86\x00" as *const u8 as *const libc::c_char,
    b"tilde;\x00" as *const u8 as *const libc::c_char,
    b"\xcb\x9c\x00" as *const u8 as *const libc::c_char,
    b"Alpha;\x00" as *const u8 as *const libc::c_char,
    b"\xce\x91\x00" as *const u8 as *const libc::c_char,
    b"Beta;\x00" as *const u8 as *const libc::c_char,
    b"\xce\x92\x00" as *const u8 as *const libc::c_char,
    b"Gamma;\x00" as *const u8 as *const libc::c_char,
    b"\xce\x93\x00" as *const u8 as *const libc::c_char,
    b"Delta;\x00" as *const u8 as *const libc::c_char,
    b"\xce\x94\x00" as *const u8 as *const libc::c_char,
    b"Epsilon;\x00" as *const u8 as *const libc::c_char,
    b"\xce\x95\x00" as *const u8 as *const libc::c_char,
    b"Zeta;\x00" as *const u8 as *const libc::c_char,
    b"\xce\x96\x00" as *const u8 as *const libc::c_char,
    b"Eta;\x00" as *const u8 as *const libc::c_char,
    b"\xce\x97\x00" as *const u8 as *const libc::c_char,
    b"Theta;\x00" as *const u8 as *const libc::c_char,
    b"\xce\x98\x00" as *const u8 as *const libc::c_char,
    b"Iota;\x00" as *const u8 as *const libc::c_char,
    b"\xce\x99\x00" as *const u8 as *const libc::c_char,
    b"Kappa;\x00" as *const u8 as *const libc::c_char,
    b"\xce\x9a\x00" as *const u8 as *const libc::c_char,
    b"Lambda;\x00" as *const u8 as *const libc::c_char,
    b"\xce\x9b\x00" as *const u8 as *const libc::c_char,
    b"Mu;\x00" as *const u8 as *const libc::c_char,
    b"\xce\x9c\x00" as *const u8 as *const libc::c_char,
    b"Nu;\x00" as *const u8 as *const libc::c_char,
    b"\xce\x9d\x00" as *const u8 as *const libc::c_char,
    b"Xi;\x00" as *const u8 as *const libc::c_char,
    b"\xce\x9e\x00" as *const u8 as *const libc::c_char,
    b"Omicron;\x00" as *const u8 as *const libc::c_char,
    b"\xce\x9f\x00" as *const u8 as *const libc::c_char,
    b"Pi;\x00" as *const u8 as *const libc::c_char,
    b"\xce\xa0\x00" as *const u8 as *const libc::c_char,
    b"Rho;\x00" as *const u8 as *const libc::c_char,
    b"\xce\xa1\x00" as *const u8 as *const libc::c_char,
    b"Sigma;\x00" as *const u8 as *const libc::c_char,
    b"\xce\xa3\x00" as *const u8 as *const libc::c_char,
    b"Tau;\x00" as *const u8 as *const libc::c_char,
    b"\xce\xa4\x00" as *const u8 as *const libc::c_char,
    b"Upsilon;\x00" as *const u8 as *const libc::c_char,
    b"\xce\xa5\x00" as *const u8 as *const libc::c_char,
    b"Phi;\x00" as *const u8 as *const libc::c_char,
    b"\xce\xa6\x00" as *const u8 as *const libc::c_char,
    b"Chi;\x00" as *const u8 as *const libc::c_char,
    b"\xce\xa7\x00" as *const u8 as *const libc::c_char,
    b"Psi;\x00" as *const u8 as *const libc::c_char,
    b"\xce\xa8\x00" as *const u8 as *const libc::c_char,
    b"Omega;\x00" as *const u8 as *const libc::c_char,
    b"\xce\xa9\x00" as *const u8 as *const libc::c_char,
    b"alpha;\x00" as *const u8 as *const libc::c_char,
    b"\xce\xb1\x00" as *const u8 as *const libc::c_char,
    b"beta;\x00" as *const u8 as *const libc::c_char,
    b"\xce\xb2\x00" as *const u8 as *const libc::c_char,
    b"gamma;\x00" as *const u8 as *const libc::c_char,
    b"\xce\xb3\x00" as *const u8 as *const libc::c_char,
    b"delta;\x00" as *const u8 as *const libc::c_char,
    b"\xce\xb4\x00" as *const u8 as *const libc::c_char,
    b"epsilon;\x00" as *const u8 as *const libc::c_char,
    b"\xce\xb5\x00" as *const u8 as *const libc::c_char,
    b"zeta;\x00" as *const u8 as *const libc::c_char,
    b"\xce\xb6\x00" as *const u8 as *const libc::c_char,
    b"eta;\x00" as *const u8 as *const libc::c_char,
    b"\xce\xb7\x00" as *const u8 as *const libc::c_char,
    b"theta;\x00" as *const u8 as *const libc::c_char,
    b"\xce\xb8\x00" as *const u8 as *const libc::c_char,
    b"iota;\x00" as *const u8 as *const libc::c_char,
    b"\xce\xb9\x00" as *const u8 as *const libc::c_char,
    b"kappa;\x00" as *const u8 as *const libc::c_char,
    b"\xce\xba\x00" as *const u8 as *const libc::c_char,
    b"lambda;\x00" as *const u8 as *const libc::c_char,
    b"\xce\xbb\x00" as *const u8 as *const libc::c_char,
    b"mu;\x00" as *const u8 as *const libc::c_char,
    b"\xce\xbc\x00" as *const u8 as *const libc::c_char,
    b"nu;\x00" as *const u8 as *const libc::c_char,
    b"\xce\xbd\x00" as *const u8 as *const libc::c_char,
    b"xi;\x00" as *const u8 as *const libc::c_char,
    b"\xce\xbe\x00" as *const u8 as *const libc::c_char,
    b"omicron;\x00" as *const u8 as *const libc::c_char,
    b"\xce\xbf\x00" as *const u8 as *const libc::c_char,
    b"pi;\x00" as *const u8 as *const libc::c_char,
    b"\xcf\x80\x00" as *const u8 as *const libc::c_char,
    b"rho;\x00" as *const u8 as *const libc::c_char,
    b"\xcf\x81\x00" as *const u8 as *const libc::c_char,
    b"sigmaf;\x00" as *const u8 as *const libc::c_char,
    b"\xcf\x82\x00" as *const u8 as *const libc::c_char,
    b"sigma;\x00" as *const u8 as *const libc::c_char,
    b"\xcf\x83\x00" as *const u8 as *const libc::c_char,
    b"tau;\x00" as *const u8 as *const libc::c_char,
    b"\xcf\x84\x00" as *const u8 as *const libc::c_char,
    b"upsilon;\x00" as *const u8 as *const libc::c_char,
    b"\xcf\x85\x00" as *const u8 as *const libc::c_char,
    b"phi;\x00" as *const u8 as *const libc::c_char,
    b"\xcf\x86\x00" as *const u8 as *const libc::c_char,
    b"chi;\x00" as *const u8 as *const libc::c_char,
    b"\xcf\x87\x00" as *const u8 as *const libc::c_char,
    b"psi;\x00" as *const u8 as *const libc::c_char,
    b"\xcf\x88\x00" as *const u8 as *const libc::c_char,
    b"omega;\x00" as *const u8 as *const libc::c_char,
    b"\xcf\x89\x00" as *const u8 as *const libc::c_char,
    b"thetasym;\x00" as *const u8 as *const libc::c_char,
    b"\xcf\x91\x00" as *const u8 as *const libc::c_char,
    b"upsih;\x00" as *const u8 as *const libc::c_char,
    b"\xcf\x92\x00" as *const u8 as *const libc::c_char,
    b"piv;\x00" as *const u8 as *const libc::c_char,
    b"\xcf\x96\x00" as *const u8 as *const libc::c_char,
    b"ensp;\x00" as *const u8 as *const libc::c_char,
    b" \x00" as *const u8 as *const libc::c_char,
    b"emsp;\x00" as *const u8 as *const libc::c_char,
    b" \x00" as *const u8 as *const libc::c_char,
    b"thinsp;\x00" as *const u8 as *const libc::c_char,
    b" \x00" as *const u8 as *const libc::c_char,
    b"zwnj;\x00" as *const u8 as *const libc::c_char,
    b"\x00" as *const u8 as *const libc::c_char,
    b"zwj;\x00" as *const u8 as *const libc::c_char,
    b"\x00" as *const u8 as *const libc::c_char,
    b"lrm;\x00" as *const u8 as *const libc::c_char,
    b"\x00" as *const u8 as *const libc::c_char,
    b"rlm;\x00" as *const u8 as *const libc::c_char,
    b"\x00" as *const u8 as *const libc::c_char,
    b"ndash;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x80\x93\x00" as *const u8 as *const libc::c_char,
    b"mdash;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x80\x94\x00" as *const u8 as *const libc::c_char,
    b"lsquo;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x80\x98\x00" as *const u8 as *const libc::c_char,
    b"rsquo;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x80\x99\x00" as *const u8 as *const libc::c_char,
    b"sbquo;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x80\x9a\x00" as *const u8 as *const libc::c_char,
    b"ldquo;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x80\x9c\x00" as *const u8 as *const libc::c_char,
    b"rdquo;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x80\x9d\x00" as *const u8 as *const libc::c_char,
    b"bdquo;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x80\x9e\x00" as *const u8 as *const libc::c_char,
    b"dagger;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x80\xa0\x00" as *const u8 as *const libc::c_char,
    b"Dagger;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x80\xa1\x00" as *const u8 as *const libc::c_char,
    b"bull;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x80\xa2\x00" as *const u8 as *const libc::c_char,
    b"hellip;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x80\xa6\x00" as *const u8 as *const libc::c_char,
    b"permil;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x80\xb0\x00" as *const u8 as *const libc::c_char,
    b"prime;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x80\xb2\x00" as *const u8 as *const libc::c_char,
    b"Prime;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x80\xb3\x00" as *const u8 as *const libc::c_char,
    b"lsaquo;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x80\xb9\x00" as *const u8 as *const libc::c_char,
    b"rsaquo;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x80\xba\x00" as *const u8 as *const libc::c_char,
    b"oline;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x80\xbe\x00" as *const u8 as *const libc::c_char,
    b"frasl;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x81\x84\x00" as *const u8 as *const libc::c_char,
    b"euro;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x82\xac\x00" as *const u8 as *const libc::c_char,
    b"image;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x84\x91\x00" as *const u8 as *const libc::c_char,
    b"weierp;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x84\x98\x00" as *const u8 as *const libc::c_char,
    b"real;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x84\x9c\x00" as *const u8 as *const libc::c_char,
    b"trade;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x84\xa2\x00" as *const u8 as *const libc::c_char,
    b"alefsym;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x84\xb5\x00" as *const u8 as *const libc::c_char,
    b"larr;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x86\x90\x00" as *const u8 as *const libc::c_char,
    b"uarr;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x86\x91\x00" as *const u8 as *const libc::c_char,
    b"rarr;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x86\x92\x00" as *const u8 as *const libc::c_char,
    b"darr;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x86\x93\x00" as *const u8 as *const libc::c_char,
    b"harr;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x86\x94\x00" as *const u8 as *const libc::c_char,
    b"crarr;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x86\xb5\x00" as *const u8 as *const libc::c_char,
    b"lArr;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x87\x90\x00" as *const u8 as *const libc::c_char,
    b"uArr;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x87\x91\x00" as *const u8 as *const libc::c_char,
    b"rArr;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x87\x92\x00" as *const u8 as *const libc::c_char,
    b"dArr;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x87\x93\x00" as *const u8 as *const libc::c_char,
    b"hArr;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x87\x94\x00" as *const u8 as *const libc::c_char,
    b"forall;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x88\x80\x00" as *const u8 as *const libc::c_char,
    b"part;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x88\x82\x00" as *const u8 as *const libc::c_char,
    b"exist;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x88\x83\x00" as *const u8 as *const libc::c_char,
    b"empty;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x88\x85\x00" as *const u8 as *const libc::c_char,
    b"nabla;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x88\x87\x00" as *const u8 as *const libc::c_char,
    b"isin;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x88\x88\x00" as *const u8 as *const libc::c_char,
    b"notin;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x88\x89\x00" as *const u8 as *const libc::c_char,
    b"ni;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x88\x8b\x00" as *const u8 as *const libc::c_char,
    b"prod;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x88\x8f\x00" as *const u8 as *const libc::c_char,
    b"sum;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x88\x91\x00" as *const u8 as *const libc::c_char,
    b"minus;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x88\x92\x00" as *const u8 as *const libc::c_char,
    b"lowast;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x88\x97\x00" as *const u8 as *const libc::c_char,
    b"radic;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x88\x9a\x00" as *const u8 as *const libc::c_char,
    b"prop;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x88\x9d\x00" as *const u8 as *const libc::c_char,
    b"infin;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x88\x9e\x00" as *const u8 as *const libc::c_char,
    b"ang;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x88\xa0\x00" as *const u8 as *const libc::c_char,
    b"and;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x88\xa7\x00" as *const u8 as *const libc::c_char,
    b"or;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x88\xa8\x00" as *const u8 as *const libc::c_char,
    b"cap;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x88\xa9\x00" as *const u8 as *const libc::c_char,
    b"cup;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x88\xaa\x00" as *const u8 as *const libc::c_char,
    b"int;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x88\xab\x00" as *const u8 as *const libc::c_char,
    b"there4;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x88\xb4\x00" as *const u8 as *const libc::c_char,
    b"sim;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x88\xbc\x00" as *const u8 as *const libc::c_char,
    b"cong;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x89\x85\x00" as *const u8 as *const libc::c_char,
    b"asymp;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x89\x88\x00" as *const u8 as *const libc::c_char,
    b"ne;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x89\xa0\x00" as *const u8 as *const libc::c_char,
    b"equiv;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x89\xa1\x00" as *const u8 as *const libc::c_char,
    b"le;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x89\xa4\x00" as *const u8 as *const libc::c_char,
    b"ge;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x89\xa5\x00" as *const u8 as *const libc::c_char,
    b"sub;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x8a\x82\x00" as *const u8 as *const libc::c_char,
    b"sup;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x8a\x83\x00" as *const u8 as *const libc::c_char,
    b"nsub;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x8a\x84\x00" as *const u8 as *const libc::c_char,
    b"sube;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x8a\x86\x00" as *const u8 as *const libc::c_char,
    b"supe;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x8a\x87\x00" as *const u8 as *const libc::c_char,
    b"oplus;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x8a\x95\x00" as *const u8 as *const libc::c_char,
    b"otimes;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x8a\x97\x00" as *const u8 as *const libc::c_char,
    b"perp;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x8a\xa5\x00" as *const u8 as *const libc::c_char,
    b"sdot;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x8b\x85\x00" as *const u8 as *const libc::c_char,
    b"lceil;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x8c\x88\x00" as *const u8 as *const libc::c_char,
    b"rceil;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x8c\x89\x00" as *const u8 as *const libc::c_char,
    b"lfloor;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x8c\x8a\x00" as *const u8 as *const libc::c_char,
    b"rfloor;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x8c\x8b\x00" as *const u8 as *const libc::c_char,
    b"lang;\x00" as *const u8 as *const libc::c_char,
    b"<\x00" as *const u8 as *const libc::c_char,
    b"rang;\x00" as *const u8 as *const libc::c_char,
    b">\x00" as *const u8 as *const libc::c_char,
    b"loz;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x97\x8a\x00" as *const u8 as *const libc::c_char,
    b"spades;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x99\xa0\x00" as *const u8 as *const libc::c_char,
    b"clubs;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x99\xa3\x00" as *const u8 as *const libc::c_char,
    b"hearts;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x99\xa5\x00" as *const u8 as *const libc::c_char,
    b"diams;\x00" as *const u8 as *const libc::c_char,
    b"\xe2\x99\xa6\x00" as *const u8 as *const libc::c_char,
    0 as *const libc::c_char,
    0 as *const libc::c_char,
];
#[no_mangle]
pub unsafe extern "C" fn dc_attr_find(
    mut attr: *mut *mut libc::c_char,
    mut key: *const libc::c_char,
) -> *const libc::c_char {
    if !attr.is_null() && !key.is_null() {
        let mut i: libc::c_int = 0i32;
        while !(*attr.offset(i as isize)).is_null() && 0 != strcmp(key, *attr.offset(i as isize)) {
            i += 2i32
        }
        if !(*attr.offset(i as isize)).is_null() {
            return *attr.offset((i + 1i32) as isize);
        }
    }
    return 0 as *const libc::c_char;
}
