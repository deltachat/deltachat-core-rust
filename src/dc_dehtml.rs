use crate::dc_saxparser::*;
use crate::dc_tools::*;
use crate::x::*;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct dehtml_t {
    pub strbuilder: String,
    pub add_text: libc::c_int,
    pub last_href: *mut libc::c_char,
}

/* ** library-internal *********************************************************/
/* dc_dehtml() returns way too many lineends; however, an optimisation on this issue is not needed as the lineends are typically remove in further processing by the caller */
pub unsafe fn dc_dehtml(buf_terminated: *mut libc::c_char) -> *mut libc::c_char {
    dc_trim(buf_terminated);
    if *buf_terminated.offset(0isize) as libc::c_int == 0i32 {
        dc_strdup(b"\x00" as *const u8 as *const libc::c_char)
    } else {
        let mut dehtml: dehtml_t = dehtml_t {
            strbuilder: String::with_capacity(strlen(buf_terminated)),
            add_text: 0,
            last_href: 0 as *mut libc::c_char,
        };
        let mut saxparser: dc_saxparser_t = dc_saxparser_t {
            starttag_cb: None,
            endtag_cb: None,
            text_cb: None,
            userdata: 0 as *mut libc::c_void,
        };
        memset(
            &mut dehtml as *mut dehtml_t as *mut libc::c_void,
            0,
            ::std::mem::size_of::<dehtml_t>(),
        );
        dehtml.add_text = 1i32;
        dc_saxparser_init(
            &mut saxparser,
            &mut dehtml as *mut dehtml_t as *mut libc::c_void,
        );
        dc_saxparser_set_tag_handler(
            &mut saxparser,
            Some(dehtml_starttag_cb),
            Some(dehtml_endtag_cb),
        );
        dc_saxparser_set_text_handler(&mut saxparser, Some(dehtml_text_cb));
        dc_saxparser_parse(&mut saxparser, buf_terminated);
        free(dehtml.last_href as *mut libc::c_void);

        strdup(to_cstring(strbuilder).as_ptr())
    }
}

unsafe fn dehtml_text_cb(
    userdata: *mut libc::c_void,
    text: *const libc::c_char,
    _len: libc::c_int,
) {
    let dehtml: *mut dehtml_t = userdata as *mut dehtml_t;
    if (*dehtml).add_text != 0i32 {
        let last_added: *mut libc::c_char = dc_strbuilder_cat(&mut (*dehtml).strbuilder, text);
        if (*dehtml).add_text == 1i32 {
            let mut p: *mut libc::c_uchar = last_added as *mut libc::c_uchar;
            while 0 != *p {
                if *p as libc::c_int == '\n' as i32 {
                    let mut last_is_lineend: libc::c_int = 1i32;
                    let mut p2: *const libc::c_uchar = p.offset(-1isize);
                    while p2 >= (*dehtml).strbuilder.buf as *const libc::c_uchar {
                        if *p2 as libc::c_int == '\r' as i32 {
                            p2 = p2.offset(-1isize)
                        } else {
                            if *p2 as libc::c_int == '\n' as i32 {
                                break;
                            }
                            last_is_lineend = 0i32;
                            break;
                        }
                    }
                    *p = (if 0 != last_is_lineend {
                        '\r' as i32
                    } else {
                        ' ' as i32
                    }) as libc::c_uchar
                }
                p = p.offset(1isize)
            }
        }
    };
}
unsafe fn dehtml_endtag_cb(userdata: *mut libc::c_void, tag: *const libc::c_char) {
    let mut dehtml: *mut dehtml_t = userdata as *mut dehtml_t;
    if strcmp(tag, b"p\x00" as *const u8 as *const libc::c_char) == 0i32
        || strcmp(tag, b"div\x00" as *const u8 as *const libc::c_char) == 0i32
        || strcmp(tag, b"table\x00" as *const u8 as *const libc::c_char) == 0i32
        || strcmp(tag, b"td\x00" as *const u8 as *const libc::c_char) == 0i32
        || strcmp(tag, b"style\x00" as *const u8 as *const libc::c_char) == 0i32
        || strcmp(tag, b"script\x00" as *const u8 as *const libc::c_char) == 0i32
        || strcmp(tag, b"title\x00" as *const u8 as *const libc::c_char) == 0i32
        || strcmp(tag, b"pre\x00" as *const u8 as *const libc::c_char) == 0i32
    {
        dc_strbuilder_cat(
            &mut (*dehtml).strbuilder,
            b"\n\n\x00" as *const u8 as *const libc::c_char,
        );
        (*dehtml).add_text = 1i32
    } else if strcmp(tag, b"a\x00" as *const u8 as *const libc::c_char) == 0i32 {
        if !(*dehtml).last_href.is_null() {
            dc_strbuilder_cat(
                &mut (*dehtml).strbuilder,
                b"](\x00" as *const u8 as *const libc::c_char,
            );
            dc_strbuilder_cat(&mut (*dehtml).strbuilder, (*dehtml).last_href);
            dc_strbuilder_cat(
                &mut (*dehtml).strbuilder,
                b")\x00" as *const u8 as *const libc::c_char,
            );
            free((*dehtml).last_href as *mut libc::c_void);
            (*dehtml).last_href = 0 as *mut libc::c_char
        }
    } else if strcmp(tag, b"b\x00" as *const u8 as *const libc::c_char) == 0i32
        || strcmp(tag, b"strong\x00" as *const u8 as *const libc::c_char) == 0i32
    {
        dc_strbuilder_cat(
            &mut (*dehtml).strbuilder,
            b"*\x00" as *const u8 as *const libc::c_char,
        );
    } else if strcmp(tag, b"i\x00" as *const u8 as *const libc::c_char) == 0i32
        || strcmp(tag, b"em\x00" as *const u8 as *const libc::c_char) == 0i32
    {
        dc_strbuilder_cat(
            &mut (*dehtml).strbuilder,
            b"_\x00" as *const u8 as *const libc::c_char,
        );
    };
}
unsafe fn dehtml_starttag_cb(
    userdata: *mut libc::c_void,
    tag: *const libc::c_char,
    attr: *mut *mut libc::c_char,
) {
    let mut dehtml: *mut dehtml_t = userdata as *mut dehtml_t;
    if strcmp(tag, b"p\x00" as *const u8 as *const libc::c_char) == 0i32
        || strcmp(tag, b"div\x00" as *const u8 as *const libc::c_char) == 0i32
        || strcmp(tag, b"table\x00" as *const u8 as *const libc::c_char) == 0i32
        || strcmp(tag, b"td\x00" as *const u8 as *const libc::c_char) == 0i32
    {
        dc_strbuilder_cat(
            &mut (*dehtml).strbuilder,
            b"\n\n\x00" as *const u8 as *const libc::c_char,
        );
        (*dehtml).add_text = 1i32
    } else if strcmp(tag, b"br\x00" as *const u8 as *const libc::c_char) == 0i32 {
        dc_strbuilder_cat(
            &mut (*dehtml).strbuilder,
            b"\n\x00" as *const u8 as *const libc::c_char,
        );
        (*dehtml).add_text = 1i32
    } else if strcmp(tag, b"style\x00" as *const u8 as *const libc::c_char) == 0i32
        || strcmp(tag, b"script\x00" as *const u8 as *const libc::c_char) == 0i32
        || strcmp(tag, b"title\x00" as *const u8 as *const libc::c_char) == 0i32
    {
        (*dehtml).add_text = 0i32
    } else if strcmp(tag, b"pre\x00" as *const u8 as *const libc::c_char) == 0i32 {
        dc_strbuilder_cat(
            &mut (*dehtml).strbuilder,
            b"\n\n\x00" as *const u8 as *const libc::c_char,
        );
        (*dehtml).add_text = 2i32
    } else if strcmp(tag, b"a\x00" as *const u8 as *const libc::c_char) == 0i32 {
        free((*dehtml).last_href as *mut libc::c_void);
        (*dehtml).last_href = dc_strdup_keep_null(dc_attr_find(
            attr,
            b"href\x00" as *const u8 as *const libc::c_char,
        ));
        if !(*dehtml).last_href.is_null() {
            dc_strbuilder_cat(
                &mut (*dehtml).strbuilder,
                b"[\x00" as *const u8 as *const libc::c_char,
            );
        }
    } else if strcmp(tag, b"b\x00" as *const u8 as *const libc::c_char) == 0i32
        || strcmp(tag, b"strong\x00" as *const u8 as *const libc::c_char) == 0i32
    {
        dc_strbuilder_cat(
            &mut (*dehtml).strbuilder,
            b"*\x00" as *const u8 as *const libc::c_char,
        );
    } else if strcmp(tag, b"i\x00" as *const u8 as *const libc::c_char) == 0i32
        || strcmp(tag, b"em\x00" as *const u8 as *const libc::c_char) == 0i32
    {
        dc_strbuilder_cat(
            &mut (*dehtml).strbuilder,
            b"_\x00" as *const u8 as *const libc::c_char,
        );
    };
}
