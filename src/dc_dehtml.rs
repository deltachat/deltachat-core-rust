use libc;
extern "C" {
    #[no_mangle]
    fn free(_: *mut libc::c_void);
    #[no_mangle]
    fn memset(_: *mut libc::c_void, _: libc::c_int, _: libc::c_ulong) -> *mut libc::c_void;
    #[no_mangle]
    fn strcmp(_: *const libc::c_char, _: *const libc::c_char) -> libc::c_int;
    #[no_mangle]
    fn strlen(_: *const libc::c_char) -> libc::c_ulong;
    /* string tools */
    #[no_mangle]
    fn dc_strdup(_: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_strdup_keep_null(_: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_trim(_: *mut libc::c_char);
    #[no_mangle]
    fn dc_strbuilder_init(_: *mut dc_strbuilder_t, init_bytes: libc::c_int);
    #[no_mangle]
    fn dc_strbuilder_cat(_: *mut dc_strbuilder_t, text: *const libc::c_char) -> *mut libc::c_char;
    #[no_mangle]
    fn dc_saxparser_parse(_: *mut dc_saxparser_t, text: *const libc::c_char);
    #[no_mangle]
    fn dc_saxparser_set_text_handler(_: *mut dc_saxparser_t, _: dc_saxparser_text_cb_t);
    #[no_mangle]
    fn dc_attr_find(attr: *mut *mut libc::c_char, key: *const libc::c_char) -> *const libc::c_char;
    #[no_mangle]
    fn dc_saxparser_set_tag_handler(
        _: *mut dc_saxparser_t,
        _: dc_saxparser_starttag_cb_t,
        _: dc_saxparser_endtag_cb_t,
    );
    #[no_mangle]
    fn dc_saxparser_init(_: *mut dc_saxparser_t, userData: *mut libc::c_void);
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _dc_strbuilder {
    pub buf: *mut libc::c_char,
    pub allocated: libc::c_int,
    pub free: libc::c_int,
    pub eos: *mut libc::c_char,
}
pub type dc_strbuilder_t = _dc_strbuilder;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct dehtml_t {
    pub strbuilder: dc_strbuilder_t,
    pub add_text: libc::c_int,
    pub last_href: *mut libc::c_char,
}
pub type dc_saxparser_t = _dc_saxparser;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _dc_saxparser {
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
/* ** library-internal *********************************************************/
/* dc_dehtml() returns way too many lineends; however, an optimisation on this issue is not needed as the lineends are typically remove in further processing by the caller */
#[no_mangle]
pub unsafe extern "C" fn dc_dehtml(mut buf_terminated: *mut libc::c_char) -> *mut libc::c_char {
    dc_trim(buf_terminated);
    if *buf_terminated.offset(0isize) as libc::c_int == 0i32 {
        return dc_strdup(b"\x00" as *const u8 as *const libc::c_char);
    } else {
        let mut dehtml: dehtml_t = dehtml_t {
            strbuilder: _dc_strbuilder {
                buf: 0 as *mut libc::c_char,
                allocated: 0,
                free: 0,
                eos: 0 as *mut libc::c_char,
            },
            add_text: 0,
            last_href: 0 as *mut libc::c_char,
        };
        let mut saxparser: dc_saxparser_t = _dc_saxparser {
            starttag_cb: None,
            endtag_cb: None,
            text_cb: None,
            userdata: 0 as *mut libc::c_void,
        };
        memset(
            &mut dehtml as *mut dehtml_t as *mut libc::c_void,
            0i32,
            ::std::mem::size_of::<dehtml_t>() as libc::c_ulong,
        );
        dehtml.add_text = 1i32;
        dc_strbuilder_init(
            &mut dehtml.strbuilder,
            strlen(buf_terminated) as libc::c_int,
        );
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
        return dehtml.strbuilder.buf;
    };
}
unsafe extern "C" fn dehtml_text_cb(
    mut userdata: *mut libc::c_void,
    mut text: *const libc::c_char,
    mut len: libc::c_int,
) {
    let mut dehtml: *mut dehtml_t = userdata as *mut dehtml_t;
    if (*dehtml).add_text != 0i32 {
        let mut last_added: *mut libc::c_char = dc_strbuilder_cat(&mut (*dehtml).strbuilder, text);
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
unsafe extern "C" fn dehtml_endtag_cb(
    mut userdata: *mut libc::c_void,
    mut tag: *const libc::c_char,
) {
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
unsafe extern "C" fn dehtml_starttag_cb(
    mut userdata: *mut libc::c_void,
    mut tag: *const libc::c_char,
    mut attr: *mut *mut libc::c_char,
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
