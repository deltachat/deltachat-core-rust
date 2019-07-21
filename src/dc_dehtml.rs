use lazy_static::lazy_static;

use crate::dc_saxparser::*;
use crate::dc_tools::*;
use crate::x::*;

lazy_static! {
    static ref LINE_RE: regex::Regex = regex::Regex::new(r"(\r?\n)+").unwrap();
}

struct Dehtml {
    strbuilder: String,
    add_text: AddText,
    last_href: *mut libc::c_char,
}

#[derive(Debug, PartialEq)]
enum AddText {
    No,
    YesRemoveLineEnds,
    YesPreserveLineEnds,
}

// dc_dehtml() returns way too many lineends; however, an optimisation on this issue is not needed as
// the lineends are typically remove in further processing by the caller
pub unsafe fn dc_dehtml(buf_terminated: *mut libc::c_char) -> *mut libc::c_char {
    dc_trim(buf_terminated);
    if *buf_terminated.offset(0isize) as libc::c_int == 0i32 {
        return dc_strdup(b"\x00" as *const u8 as *const libc::c_char);
    }

    let mut dehtml = Dehtml {
        strbuilder: String::with_capacity(strlen(buf_terminated)),
        add_text: AddText::YesRemoveLineEnds,
        last_href: 0 as *mut libc::c_char,
    };
    let mut saxparser = dc_saxparser_t {
        starttag_cb: None,
        endtag_cb: None,
        text_cb: None,
        userdata: 0 as *mut libc::c_void,
    };
    dc_saxparser_init(
        &mut saxparser,
        &mut dehtml as *mut Dehtml as *mut libc::c_void,
    );
    dc_saxparser_set_tag_handler(
        &mut saxparser,
        Some(dehtml_starttag_cb),
        Some(dehtml_endtag_cb),
    );
    dc_saxparser_set_text_handler(&mut saxparser, Some(dehtml_text_cb));
    dc_saxparser_parse(&mut saxparser, buf_terminated);
    free(dehtml.last_href as *mut libc::c_void);

    to_cstring(dehtml.strbuilder)
}

unsafe fn dehtml_text_cb(
    userdata: *mut libc::c_void,
    text: *const libc::c_char,
    _len: libc::c_int,
) {
    let dehtml = &mut *(userdata as *mut Dehtml);

    if dehtml.add_text == AddText::YesPreserveLineEnds
        || dehtml.add_text == AddText::YesRemoveLineEnds
    {
        let last_added = std::ffi::CStr::from_ptr(text).to_string_lossy();

        if dehtml.add_text == AddText::YesRemoveLineEnds {
            dehtml.strbuilder += LINE_RE.replace_all(last_added.as_ref(), "\r").as_ref();
        } else {
            dehtml.strbuilder += last_added.as_ref();
        }
    }
}

unsafe fn dehtml_endtag_cb(userdata: *mut libc::c_void, tag: *const libc::c_char) {
    let mut dehtml = &mut *(userdata as *mut Dehtml);
    let tag = std::ffi::CStr::from_ptr(tag).to_string_lossy();

    match tag.as_ref() {
        "p" | "div" | "table" | "td" | "style" | "script" | "title" | "pre" => {
            dehtml.strbuilder += "\n\n";
            dehtml.add_text = AddText::YesRemoveLineEnds;
        }
        "a" => {
            if !dehtml.last_href.is_null() {
                dehtml.strbuilder += "](";
                dehtml.strbuilder += std::ffi::CStr::from_ptr((*dehtml).last_href)
                    .to_string_lossy()
                    .as_ref();
                dehtml.strbuilder += ")";
                free(dehtml.last_href as *mut libc::c_void);
                dehtml.last_href = 0 as *mut libc::c_char;
            }
        }
        "b" | "strong" => {
            dehtml.strbuilder += "*";
        }
        "i" | "em" => {
            dehtml.strbuilder += "_";
        }
        _ => {}
    }
}

unsafe fn dehtml_starttag_cb(
    userdata: *mut libc::c_void,
    tag: *const libc::c_char,
    attr: *mut *mut libc::c_char,
) {
    let mut dehtml = &mut *(userdata as *mut Dehtml);
    let tag = std::ffi::CStr::from_ptr(tag).to_string_lossy();

    match tag.as_ref() {
        "p" | "div" | "table" | "td" => {
            dehtml.strbuilder += "\n\n";
            dehtml.add_text = AddText::YesRemoveLineEnds;
        }
        "br" => {
            dehtml.strbuilder += "\n";
            dehtml.add_text = AddText::YesRemoveLineEnds;
        }
        "style" | "script" | "title" => {
            dehtml.add_text = AddText::No;
        }
        "pre" => {
            dehtml.strbuilder += "\n\n";
            dehtml.add_text = AddText::YesPreserveLineEnds;
        }
        "a" => {
            free(dehtml.last_href as *mut libc::c_void);
            dehtml.last_href = dc_strdup_keep_null(dc_attr_find(
                attr,
                b"href\x00" as *const u8 as *const libc::c_char,
            ));
            if !dehtml.last_href.is_null() {
                dehtml.strbuilder += "[";
            }
        }
        "b" | "strong" => {
            dehtml.strbuilder += "*";
        }
        "i" | "em" => {
            dehtml.strbuilder += "_";
        }
        _ => {}
    }
}
