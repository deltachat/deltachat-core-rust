use std::ffi::CStr;

use crate::clist::*;
use crate::mailimf_write_generic::*;
use crate::mailmime::*;
use crate::mailmime_content::*;
use crate::mailmime_types::*;
use crate::mailmime_types_helper::*;
use crate::other::*;

pub const STATE_INIT: libc::c_uint = 0;
pub const STATE_SPACE_CR: libc::c_uint = 3;
pub const STATE_SPACE: libc::c_uint = 2;
pub const STATE_CR: libc::c_uint = 1;

pub unsafe fn mailmime_fields_write_driver(
    mut do_write: Option<
        unsafe fn(_: *mut libc::c_void, _: *const libc::c_char, _: size_t) -> libc::c_int,
    >,
    mut data: *mut libc::c_void,
    mut col: *mut libc::c_int,
    mut fields: *mut mailmime_fields,
) -> libc::c_int {
    let mut r: libc::c_int = 0;
    let mut cur: *mut clistiter = 0 as *mut clistiter;
    cur = (*(*fields).fld_list).first;
    while !cur.is_null() {
        let mut field: *mut mailmime_field = 0 as *mut mailmime_field;
        field = (*cur).data as *mut mailmime_field;
        r = mailmime_field_write_driver(do_write, data, col, field);
        if r != MAILIMF_NO_ERROR as libc::c_int {
            return r;
        }
        cur = if !cur.is_null() {
            (*cur).next
        } else {
            0 as *mut clistcell
        }
    }
    return MAILIMF_NO_ERROR as libc::c_int;
}

unsafe fn mailmime_field_write_driver(
    mut do_write: Option<
        unsafe fn(_: *mut libc::c_void, _: *const libc::c_char, _: size_t) -> libc::c_int,
    >,
    mut data: *mut libc::c_void,
    mut col: *mut libc::c_int,
    mut field: *mut mailmime_field,
) -> libc::c_int {
    let mut r: libc::c_int = 0;
    match (*field).fld_type {
        1 => r = mailmime_content_write_driver(do_write, data, col, (*field).fld_data.fld_content),
        2 => {
            r = mailmime_encoding_write_driver(do_write, data, col, (*field).fld_data.fld_encoding)
        }
        3 => r = mailmime_id_write_driver(do_write, data, col, (*field).fld_data.fld_id),
        4 => {
            r = mailmime_description_write_driver(
                do_write,
                data,
                col,
                (*field).fld_data.fld_description,
            )
        }
        5 => r = mailmime_version_write_driver(do_write, data, col, (*field).fld_data.fld_version),
        6 => {
            r = mailmime_disposition_write_driver(
                do_write,
                data,
                col,
                (*field).fld_data.fld_disposition,
            )
        }
        7 => {
            r = mailmime_language_write_driver(do_write, data, col, (*field).fld_data.fld_language)
        }
        8 => {
            r = mailmime_location_write_driver(do_write, data, col, (*field).fld_data.fld_location)
        }
        _ => r = MAILIMF_ERROR_INVAL as libc::c_int,
    }
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    return MAILIMF_NO_ERROR as libc::c_int;
}
unsafe fn mailmime_location_write_driver(
    mut do_write: Option<
        unsafe fn(_: *mut libc::c_void, _: *const libc::c_char, _: size_t) -> libc::c_int,
    >,
    mut data: *mut libc::c_void,
    mut col: *mut libc::c_int,
    mut location: *mut libc::c_char,
) -> libc::c_int {
    let mut r: libc::c_int = 0;
    let mut len: libc::c_int = strlen(location) as libc::c_int;
    r = mailimf_string_write_driver(
        do_write,
        data,
        col,
        b"Content-Location: \x00" as *const u8 as *const libc::c_char,
        18i32 as size_t,
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    if *col > 1i32 && *col + len > 78i32 {
        r = mailimf_string_write_driver(
            do_write,
            data,
            col,
            b"\r\n \x00" as *const u8 as *const libc::c_char,
            3i32 as size_t,
        );
        if r != MAILIMF_NO_ERROR as libc::c_int {
            return r;
        }
    }
    r = mailimf_string_write_driver(do_write, data, col, location, len as size_t);
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    r = mailimf_string_write_driver(
        do_write,
        data,
        col,
        b"\r\n\x00" as *const u8 as *const libc::c_char,
        2i32 as size_t,
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    return MAILIMF_NO_ERROR as libc::c_int;
}
unsafe fn mailmime_language_write_driver(
    mut do_write: Option<
        unsafe fn(_: *mut libc::c_void, _: *const libc::c_char, _: size_t) -> libc::c_int,
    >,
    mut data: *mut libc::c_void,
    mut col: *mut libc::c_int,
    mut language: *mut mailmime_language,
) -> libc::c_int {
    let mut r: libc::c_int = 0;
    let mut cur: *mut clistiter = 0 as *mut clistiter;
    let mut first: libc::c_int = 0;
    r = mailimf_string_write_driver(
        do_write,
        data,
        col,
        b"Content-Language: \x00" as *const u8 as *const libc::c_char,
        18i32 as size_t,
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    first = 1i32;
    cur = (*(*language).lg_list).first;
    while !cur.is_null() {
        let mut lang: *mut libc::c_char = 0 as *mut libc::c_char;
        let mut len: size_t = 0;
        lang = (if !cur.is_null() {
            (*cur).data
        } else {
            0 as *mut libc::c_void
        }) as *mut libc::c_char;
        len = strlen(lang);
        if 0 == first {
            r = mailimf_string_write_driver(
                do_write,
                data,
                col,
                b", \x00" as *const u8 as *const libc::c_char,
                2i32 as size_t,
            );
            if r != MAILIMF_NO_ERROR as libc::c_int {
                return r;
            }
        } else {
            first = 0i32
        }
        if *col > 1i32 {
            if (*col as libc::size_t).wrapping_add(len) > 78i32 as libc::size_t {
                r = mailimf_string_write_driver(
                    do_write,
                    data,
                    col,
                    b"\r\n \x00" as *const u8 as *const libc::c_char,
                    3i32 as size_t,
                );
                if r != MAILIMF_NO_ERROR as libc::c_int {
                    return r;
                }
            }
        }
        r = mailimf_string_write_driver(do_write, data, col, lang, len);
        if r != MAILIMF_NO_ERROR as libc::c_int {
            return r;
        }
        cur = if !cur.is_null() {
            (*cur).next
        } else {
            0 as *mut clistcell
        }
    }
    r = mailimf_string_write_driver(
        do_write,
        data,
        col,
        b"\r\n\x00" as *const u8 as *const libc::c_char,
        2i32 as size_t,
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    return MAILIMF_NO_ERROR as libc::c_int;
}
unsafe fn mailmime_disposition_write_driver(
    mut do_write: Option<
        unsafe fn(_: *mut libc::c_void, _: *const libc::c_char, _: size_t) -> libc::c_int,
    >,
    mut data: *mut libc::c_void,
    mut col: *mut libc::c_int,
    mut disposition: *mut mailmime_disposition,
) -> libc::c_int {
    let mut dsp_type: *mut mailmime_disposition_type = 0 as *mut mailmime_disposition_type;
    let mut r: libc::c_int = 0;
    let mut cur: *mut clistiter = 0 as *mut clistiter;
    dsp_type = (*disposition).dsp_type;
    r = mailimf_string_write_driver(
        do_write,
        data,
        col,
        b"Content-Disposition: \x00" as *const u8 as *const libc::c_char,
        21i32 as size_t,
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    match (*dsp_type).dsp_type {
        1 => {
            r = mailimf_string_write_driver(
                do_write,
                data,
                col,
                b"inline\x00" as *const u8 as *const libc::c_char,
                6i32 as size_t,
            )
        }
        2 => {
            r = mailimf_string_write_driver(
                do_write,
                data,
                col,
                b"attachment\x00" as *const u8 as *const libc::c_char,
                10i32 as size_t,
            )
        }
        3 => {
            r = mailimf_string_write_driver(
                do_write,
                data,
                col,
                (*dsp_type).dsp_extension,
                strlen((*dsp_type).dsp_extension),
            )
        }
        _ => r = MAILIMF_ERROR_INVAL as libc::c_int,
    }
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    cur = (*(*disposition).dsp_parms).first;
    while !cur.is_null() {
        let mut param: *mut mailmime_disposition_parm = 0 as *mut mailmime_disposition_parm;
        param = (*cur).data as *mut mailmime_disposition_parm;
        r = mailimf_string_write_driver(
            do_write,
            data,
            col,
            b"; \x00" as *const u8 as *const libc::c_char,
            2i32 as size_t,
        );
        if r != MAILIMF_NO_ERROR as libc::c_int {
            return r;
        }
        r = mailmime_disposition_param_write_driver(do_write, data, col, param);
        if r != MAILIMF_NO_ERROR as libc::c_int {
            return r;
        }
        cur = if !cur.is_null() {
            (*cur).next
        } else {
            0 as *mut clistcell
        }
    }
    r = mailimf_string_write_driver(
        do_write,
        data,
        col,
        b"\r\n\x00" as *const u8 as *const libc::c_char,
        2i32 as size_t,
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    return MAILIMF_NO_ERROR as libc::c_int;
}
unsafe fn mailmime_disposition_param_write_driver(
    mut do_write: Option<
        unsafe fn(_: *mut libc::c_void, _: *const libc::c_char, _: size_t) -> libc::c_int,
    >,
    mut data: *mut libc::c_void,
    mut col: *mut libc::c_int,
    mut param: *mut mailmime_disposition_parm,
) -> libc::c_int {
    let mut len: size_t = 0;
    let mut r: libc::c_int = 0;
    let mut sizestr: *mut libc::c_char = std::ptr::null_mut();

    match (*param).pa_type {
        0 => {
            len = strlen(b"filename=\x00" as *const u8 as *const libc::c_char)
                .wrapping_add(strlen((*param).pa_data.pa_filename))
        }
        1 => {
            len = strlen(b"creation-date=\x00" as *const u8 as *const libc::c_char)
                .wrapping_add(strlen((*param).pa_data.pa_creation_date))
        }
        2 => {
            len = strlen(b"modification-date=\x00" as *const u8 as *const libc::c_char)
                .wrapping_add(strlen((*param).pa_data.pa_modification_date))
        }
        3 => {
            len = strlen(b"read-date=\x00" as *const u8 as *const libc::c_char)
                .wrapping_add(strlen((*param).pa_data.pa_read_date))
        }
        4 => {
            let value = (*param).pa_data.pa_size as u32;
            let raw = format!("{}", value);
            let raw_c = std::ffi::CString::new(raw).unwrap();
            sizestr = strdup(raw_c.as_ptr());
            len = strlen(b"size=\x00" as *const u8 as *const libc::c_char)
                .wrapping_add(strlen(sizestr))
        }
        5 => {
            len = strlen((*(*param).pa_data.pa_parameter).pa_name)
                .wrapping_add(1i32 as libc::size_t)
                .wrapping_add(strlen((*(*param).pa_data.pa_parameter).pa_value))
        }
        _ => return MAILIMF_ERROR_INVAL as libc::c_int,
    }
    if *col > 1i32 {
        if (*col as libc::size_t).wrapping_add(len) > 78i32 as libc::size_t {
            r = mailimf_string_write_driver(
                do_write,
                data,
                col,
                b"\r\n \x00" as *const u8 as *const libc::c_char,
                3i32 as size_t,
            );
            if r != MAILIMF_NO_ERROR as libc::c_int {
                return r;
            }
        }
    }
    match (*param).pa_type {
        0 => {
            r = mailimf_string_write_driver(
                do_write,
                data,
                col,
                b"filename=\x00" as *const u8 as *const libc::c_char,
                9i32 as size_t,
            );
            if r != MAILIMF_NO_ERROR as libc::c_int {
                return r;
            }
            r = mailimf_quoted_string_write_driver(
                do_write,
                data,
                col,
                (*param).pa_data.pa_filename,
                strlen((*param).pa_data.pa_filename),
            );
            if r != MAILIMF_NO_ERROR as libc::c_int {
                return r;
            }
        }
        1 => {
            r = mailimf_string_write_driver(
                do_write,
                data,
                col,
                b"creation-date=\x00" as *const u8 as *const libc::c_char,
                14i32 as size_t,
            );
            if r != MAILIMF_NO_ERROR as libc::c_int {
                return r;
            }
            r = mailimf_quoted_string_write_driver(
                do_write,
                data,
                col,
                (*param).pa_data.pa_creation_date,
                strlen((*param).pa_data.pa_creation_date),
            );
            if r != MAILIMF_NO_ERROR as libc::c_int {
                return r;
            }
        }
        2 => {
            r = mailimf_string_write_driver(
                do_write,
                data,
                col,
                b"modification-date=\x00" as *const u8 as *const libc::c_char,
                18i32 as size_t,
            );
            if r != MAILIMF_NO_ERROR as libc::c_int {
                return r;
            }
            r = mailimf_quoted_string_write_driver(
                do_write,
                data,
                col,
                (*param).pa_data.pa_modification_date,
                strlen((*param).pa_data.pa_modification_date),
            );
            if r != MAILIMF_NO_ERROR as libc::c_int {
                return r;
            }
        }
        3 => {
            r = mailimf_string_write_driver(
                do_write,
                data,
                col,
                b"read-date=\x00" as *const u8 as *const libc::c_char,
                10i32 as size_t,
            );
            if r != MAILIMF_NO_ERROR as libc::c_int {
                return r;
            }
            r = mailimf_quoted_string_write_driver(
                do_write,
                data,
                col,
                (*param).pa_data.pa_read_date,
                strlen((*param).pa_data.pa_read_date),
            );
            if r != MAILIMF_NO_ERROR as libc::c_int {
                return r;
            }
        }
        4 => {
            r = mailimf_string_write_driver(
                do_write,
                data,
                col,
                b"size=\x00" as *const u8 as *const libc::c_char,
                5i32 as size_t,
            );
            if r != MAILIMF_NO_ERROR as libc::c_int {
                return r;
            }
            r = mailimf_string_write_driver(do_write, data, col, sizestr, strlen(sizestr));
            if r != MAILIMF_NO_ERROR as libc::c_int {
                return r;
            }
        }
        5 => {
            r = mailmime_parameter_write_driver(do_write, data, col, (*param).pa_data.pa_parameter);
            if r != MAILIMF_NO_ERROR as libc::c_int {
                return r;
            }
        }
        _ => {}
    }
    return MAILIMF_NO_ERROR as libc::c_int;
}
unsafe fn mailmime_parameter_write_driver(
    mut do_write: Option<
        unsafe fn(_: *mut libc::c_void, _: *const libc::c_char, _: size_t) -> libc::c_int,
    >,
    mut data: *mut libc::c_void,
    mut col: *mut libc::c_int,
    mut param: *mut mailmime_parameter,
) -> libc::c_int {
    let mut r: libc::c_int = 0;
    r = mailimf_string_write_driver(
        do_write,
        data,
        col,
        (*param).pa_name,
        strlen((*param).pa_name),
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    r = mailimf_string_write_driver(
        do_write,
        data,
        col,
        b"=\x00" as *const u8 as *const libc::c_char,
        1i32 as size_t,
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    r = mailimf_quoted_string_write_driver(
        do_write,
        data,
        col,
        (*param).pa_value,
        strlen((*param).pa_value),
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    return MAILIMF_NO_ERROR as libc::c_int;
}
unsafe fn mailmime_version_write_driver(
    mut do_write: Option<
        unsafe fn(_: *mut libc::c_void, _: *const libc::c_char, _: size_t) -> libc::c_int,
    >,
    mut data: *mut libc::c_void,
    mut col: *mut libc::c_int,
    mut version: uint32_t,
) -> libc::c_int {
    let mut r: libc::c_int = 0;

    r = mailimf_string_write_driver(
        do_write,
        data,
        col,
        b"MIME-Version: \x00" as *const u8 as *const libc::c_char,
        14i32 as size_t,
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }

    let raw = format!("{}.{}", (version >> 16) as i32, (version & 0xffff) as i32);
    let raw_c = std::ffi::CString::new(raw).unwrap();
    let mut versionstr = strdup(raw_c.as_ptr());
    r = mailimf_string_write_driver(do_write, data, col, versionstr, strlen(versionstr));
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    r = mailimf_string_write_driver(
        do_write,
        data,
        col,
        b"\r\n\x00" as *const u8 as *const libc::c_char,
        2i32 as size_t,
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    return MAILIMF_NO_ERROR as libc::c_int;
}
unsafe fn mailmime_description_write_driver(
    mut do_write: Option<
        unsafe fn(_: *mut libc::c_void, _: *const libc::c_char, _: size_t) -> libc::c_int,
    >,
    mut data: *mut libc::c_void,
    mut col: *mut libc::c_int,
    mut descr: *mut libc::c_char,
) -> libc::c_int {
    let mut r: libc::c_int = 0;
    r = mailimf_string_write_driver(
        do_write,
        data,
        col,
        b"Content-Description: \x00" as *const u8 as *const libc::c_char,
        21i32 as size_t,
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    r = mailimf_string_write_driver(do_write, data, col, descr, strlen(descr));
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    r = mailimf_string_write_driver(
        do_write,
        data,
        col,
        b"\r\n\x00" as *const u8 as *const libc::c_char,
        2i32 as size_t,
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    return MAILIMF_NO_ERROR as libc::c_int;
}
unsafe fn mailmime_id_write_driver(
    mut do_write: Option<
        unsafe fn(_: *mut libc::c_void, _: *const libc::c_char, _: size_t) -> libc::c_int,
    >,
    mut data: *mut libc::c_void,
    mut col: *mut libc::c_int,
    mut id: *mut libc::c_char,
) -> libc::c_int {
    let mut r: libc::c_int = 0;
    r = mailimf_string_write_driver(
        do_write,
        data,
        col,
        b"Content-ID: \x00" as *const u8 as *const libc::c_char,
        12i32 as size_t,
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    r = mailimf_string_write_driver(
        do_write,
        data,
        col,
        b"<\x00" as *const u8 as *const libc::c_char,
        1i32 as size_t,
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    r = mailimf_string_write_driver(do_write, data, col, id, strlen(id));
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    r = mailimf_string_write_driver(
        do_write,
        data,
        col,
        b">\x00" as *const u8 as *const libc::c_char,
        1i32 as size_t,
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    r = mailimf_string_write_driver(
        do_write,
        data,
        col,
        b"\r\n\x00" as *const u8 as *const libc::c_char,
        2i32 as size_t,
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    return MAILIMF_NO_ERROR as libc::c_int;
}
unsafe fn mailmime_encoding_write_driver(
    mut do_write: Option<
        unsafe fn(_: *mut libc::c_void, _: *const libc::c_char, _: size_t) -> libc::c_int,
    >,
    mut data: *mut libc::c_void,
    mut col: *mut libc::c_int,
    mut encoding: *mut mailmime_mechanism,
) -> libc::c_int {
    let mut r: libc::c_int = 0;
    r = mailimf_string_write_driver(
        do_write,
        data,
        col,
        b"Content-Transfer-Encoding: \x00" as *const u8 as *const libc::c_char,
        27i32 as size_t,
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    match (*encoding).enc_type {
        1 => {
            r = mailimf_string_write_driver(
                do_write,
                data,
                col,
                b"7bit\x00" as *const u8 as *const libc::c_char,
                4i32 as size_t,
            )
        }
        2 => {
            r = mailimf_string_write_driver(
                do_write,
                data,
                col,
                b"8bit\x00" as *const u8 as *const libc::c_char,
                4i32 as size_t,
            )
        }
        3 => {
            r = mailimf_string_write_driver(
                do_write,
                data,
                col,
                b"binary\x00" as *const u8 as *const libc::c_char,
                6i32 as size_t,
            )
        }
        4 => {
            r = mailimf_string_write_driver(
                do_write,
                data,
                col,
                b"quoted-printable\x00" as *const u8 as *const libc::c_char,
                16i32 as size_t,
            )
        }
        5 => {
            r = mailimf_string_write_driver(
                do_write,
                data,
                col,
                b"base64\x00" as *const u8 as *const libc::c_char,
                6i32 as size_t,
            )
        }
        6 => {
            r = mailimf_string_write_driver(
                do_write,
                data,
                col,
                (*encoding).enc_token,
                strlen((*encoding).enc_token),
            )
        }
        _ => r = MAILIMF_ERROR_INVAL as libc::c_int,
    }
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    r = mailimf_string_write_driver(
        do_write,
        data,
        col,
        b"\r\n\x00" as *const u8 as *const libc::c_char,
        2i32 as size_t,
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    return MAILIMF_NO_ERROR as libc::c_int;
}

pub unsafe fn mailmime_content_write_driver(
    mut do_write: Option<
        unsafe fn(_: *mut libc::c_void, _: *const libc::c_char, _: size_t) -> libc::c_int,
    >,
    mut data: *mut libc::c_void,
    mut col: *mut libc::c_int,
    mut content: *mut mailmime_content,
) -> libc::c_int {
    let mut r: libc::c_int = 0;
    r = mailimf_string_write_driver(
        do_write,
        data,
        col,
        b"Content-Type: \x00" as *const u8 as *const libc::c_char,
        14i32 as size_t,
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    r = mailmime_content_type_write_driver(do_write, data, col, content);
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    r = mailimf_string_write_driver(
        do_write,
        data,
        col,
        b"\r\n\x00" as *const u8 as *const libc::c_char,
        2i32 as size_t,
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    return MAILIMF_NO_ERROR as libc::c_int;
}

pub unsafe fn mailmime_content_type_write_driver(
    mut do_write: Option<
        unsafe fn(_: *mut libc::c_void, _: *const libc::c_char, _: size_t) -> libc::c_int,
    >,
    mut data: *mut libc::c_void,
    mut col: *mut libc::c_int,
    mut content: *mut mailmime_content,
) -> libc::c_int {
    let mut cur: *mut clistiter = 0 as *mut clistiter;
    let mut len: size_t = 0;
    let mut r: libc::c_int = 0;
    r = mailmime_type_write_driver(do_write, data, col, (*content).ct_type);
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    r = mailimf_string_write_driver(
        do_write,
        data,
        col,
        b"/\x00" as *const u8 as *const libc::c_char,
        1i32 as size_t,
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    r = mailimf_string_write_driver(
        do_write,
        data,
        col,
        (*content).ct_subtype,
        strlen((*content).ct_subtype),
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    if !(*content).ct_parameters.is_null() {
        cur = (*(*content).ct_parameters).first;
        while !cur.is_null() {
            let mut param: *mut mailmime_parameter = 0 as *mut mailmime_parameter;
            param = (*cur).data as *mut mailmime_parameter;
            r = mailimf_string_write_driver(
                do_write,
                data,
                col,
                b"; \x00" as *const u8 as *const libc::c_char,
                2i32 as size_t,
            );
            if r != MAILIMF_NO_ERROR as libc::c_int {
                return r;
            }
            len = strlen((*param).pa_name)
                .wrapping_add(1i32 as libc::size_t)
                .wrapping_add(strlen((*param).pa_value));
            if *col > 1i32 {
                if (*col as libc::size_t).wrapping_add(len) > 78i32 as libc::size_t {
                    r = mailimf_string_write_driver(
                        do_write,
                        data,
                        col,
                        b"\r\n \x00" as *const u8 as *const libc::c_char,
                        3i32 as size_t,
                    );
                    if r != MAILIMF_NO_ERROR as libc::c_int {
                        return r;
                    }
                }
            }
            r = mailmime_parameter_write_driver(do_write, data, col, param);
            if r != MAILIMF_NO_ERROR as libc::c_int {
                return r;
            }
            cur = if !cur.is_null() {
                (*cur).next
            } else {
                0 as *mut clistcell
            }
        }
    }
    return MAILIMF_NO_ERROR as libc::c_int;
}
/*
static int mailmime_content_write_driver(int (* do_write)(void *, const char *, size_t), void * data, int * col,
                  struct mailmime_content * content);
*/
unsafe fn mailmime_type_write_driver(
    mut do_write: Option<
        unsafe fn(_: *mut libc::c_void, _: *const libc::c_char, _: size_t) -> libc::c_int,
    >,
    mut data: *mut libc::c_void,
    mut col: *mut libc::c_int,
    mut type_0: *mut mailmime_type,
) -> libc::c_int {
    let mut r: libc::c_int = 0;
    match (*type_0).tp_type {
        1 => {
            r = mailmime_discrete_type_write_driver(
                do_write,
                data,
                col,
                (*type_0).tp_data.tp_discrete_type,
            )
        }
        2 => {
            r = mailmime_composite_type_write_driver(
                do_write,
                data,
                col,
                (*type_0).tp_data.tp_composite_type,
            )
        }
        _ => r = MAILIMF_ERROR_INVAL as libc::c_int,
    }
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    return MAILIMF_NO_ERROR as libc::c_int;
}
unsafe fn mailmime_composite_type_write_driver(
    mut do_write: Option<
        unsafe fn(_: *mut libc::c_void, _: *const libc::c_char, _: size_t) -> libc::c_int,
    >,
    mut data: *mut libc::c_void,
    mut col: *mut libc::c_int,
    mut composite_type: *mut mailmime_composite_type,
) -> libc::c_int {
    let mut r: libc::c_int = 0;
    match (*composite_type).ct_type {
        1 => {
            r = mailimf_string_write_driver(
                do_write,
                data,
                col,
                b"message\x00" as *const u8 as *const libc::c_char,
                7i32 as size_t,
            )
        }
        2 => {
            r = mailimf_string_write_driver(
                do_write,
                data,
                col,
                b"multipart\x00" as *const u8 as *const libc::c_char,
                9i32 as size_t,
            )
        }
        3 => {
            r = mailimf_string_write_driver(
                do_write,
                data,
                col,
                (*composite_type).ct_token,
                strlen((*composite_type).ct_token),
            )
        }
        _ => r = MAILIMF_ERROR_INVAL as libc::c_int,
    }
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    return MAILIMF_NO_ERROR as libc::c_int;
}
unsafe fn mailmime_discrete_type_write_driver(
    mut do_write: Option<
        unsafe fn(_: *mut libc::c_void, _: *const libc::c_char, _: size_t) -> libc::c_int,
    >,
    mut data: *mut libc::c_void,
    mut col: *mut libc::c_int,
    mut discrete_type: *mut mailmime_discrete_type,
) -> libc::c_int {
    let mut r: libc::c_int = 0;
    match (*discrete_type).dt_type {
        1 => {
            r = mailimf_string_write_driver(
                do_write,
                data,
                col,
                b"text\x00" as *const u8 as *const libc::c_char,
                4i32 as size_t,
            )
        }
        2 => {
            r = mailimf_string_write_driver(
                do_write,
                data,
                col,
                b"image\x00" as *const u8 as *const libc::c_char,
                5i32 as size_t,
            )
        }
        3 => {
            r = mailimf_string_write_driver(
                do_write,
                data,
                col,
                b"audio\x00" as *const u8 as *const libc::c_char,
                5i32 as size_t,
            )
        }
        4 => {
            r = mailimf_string_write_driver(
                do_write,
                data,
                col,
                b"video\x00" as *const u8 as *const libc::c_char,
                5i32 as size_t,
            )
        }
        5 => {
            r = mailimf_string_write_driver(
                do_write,
                data,
                col,
                b"application\x00" as *const u8 as *const libc::c_char,
                11i32 as size_t,
            )
        }
        6 => {
            r = mailimf_string_write_driver(
                do_write,
                data,
                col,
                (*discrete_type).dt_extension,
                strlen((*discrete_type).dt_extension),
            )
        }
        _ => r = MAILIMF_ERROR_INVAL as libc::c_int,
    }
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    return MAILIMF_NO_ERROR as libc::c_int;
}

pub unsafe fn mailmime_write_driver(
    mut do_write: Option<
        unsafe fn(_: *mut libc::c_void, _: *const libc::c_char, _: size_t) -> libc::c_int,
    >,
    mut data: *mut libc::c_void,
    mut col: *mut libc::c_int,
    mut build_info: *mut mailmime,
) -> libc::c_int {
    if !(*build_info).mm_parent.is_null() {
        return mailmime_sub_write_driver(do_write, data, col, build_info);
    } else {
        return mailmime_part_write_driver(do_write, data, col, build_info);
    };
}
/*
static int mailmime_base64_write_driver(int (* do_write)(void *, const char *, size_t), void * data, int * col,
                 char * text, size_t size);

static int mailmime_quoted_printable_write_driver(int (* do_write)(void *, const char *, size_t), void * data, int * col, int istext,
                       char * text, size_t size);
*/
unsafe fn mailmime_part_write_driver(
    mut do_write: Option<
        unsafe fn(_: *mut libc::c_void, _: *const libc::c_char, _: size_t) -> libc::c_int,
    >,
    mut data: *mut libc::c_void,
    mut col: *mut libc::c_int,
    mut build_info: *mut mailmime,
) -> libc::c_int {
    let mut current_block: u64;
    let mut cur: *mut clistiter = 0 as *mut clistiter;
    let mut first: libc::c_int = 0;
    let mut r: libc::c_int = 0;
    let mut boundary: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut istext: libc::c_int = 0;
    let mut res: libc::c_int = 0;
    istext = 1i32;
    boundary = 0 as *mut libc::c_char;
    if !(*build_info).mm_content_type.is_null() {
        if (*build_info).mm_type == MAILMIME_MULTIPLE as libc::c_int {
            boundary = mailmime_extract_boundary((*build_info).mm_content_type);
            if boundary.is_null() {
                boundary = mailmime_generate_boundary();
                if boundary.is_null() {
                    res = MAILIMF_ERROR_MEMORY as libc::c_int;
                    current_block = 13530634675565645571;
                } else {
                    current_block = 13586036798005543211;
                }
            } else {
                current_block = 13586036798005543211;
            }
        } else {
            current_block = 13586036798005543211;
        }
        match current_block {
            13530634675565645571 => {}
            _ => {
                if (*(*(*build_info).mm_content_type).ct_type).tp_type
                    == MAILMIME_TYPE_DISCRETE_TYPE as libc::c_int
                {
                    if (*(*(*(*build_info).mm_content_type).ct_type)
                        .tp_data
                        .tp_discrete_type)
                        .dt_type
                        != MAILMIME_DISCRETE_TYPE_TEXT as libc::c_int
                    {
                        istext = 0i32
                    }
                }
                current_block = 8457315219000651999;
            }
        }
    } else {
        current_block = 8457315219000651999;
    }
    match current_block {
        8457315219000651999 => {
            match (*build_info).mm_type {
                1 => {
                    /* 1-part body */
                    if !(*build_info).mm_data.mm_single.is_null() {
                        r = mailmime_data_write_driver(
                            do_write,
                            data,
                            col,
                            (*build_info).mm_data.mm_single,
                            istext,
                        );
                        if r != MAILIMF_NO_ERROR as libc::c_int {
                            res = r;
                            current_block = 16754986508692159943;
                        } else {
                            current_block = 7639320476250304355;
                        }
                    } else {
                        current_block = 7639320476250304355;
                    }
                }
                2 => {
                    /* multi-part */
                    /* preamble */
                    if !(*build_info).mm_data.mm_multipart.mm_preamble.is_null() {
                        r = mailmime_data_write_driver(
                            do_write,
                            data,
                            col,
                            (*build_info).mm_data.mm_multipart.mm_preamble,
                            1i32,
                        );
                        if r != MAILIMF_NO_ERROR as libc::c_int {
                            res = r;
                            current_block = 16754986508692159943;
                        } else {
                            r = mailimf_string_write_driver(
                                do_write,
                                data,
                                col,
                                b"\r\n\x00" as *const u8 as *const libc::c_char,
                                2i32 as size_t,
                            );
                            if r != MAILIMF_NO_ERROR as libc::c_int {
                                res = r;
                                current_block = 16754986508692159943;
                            } else {
                                current_block = 17500079516916021833;
                            }
                        }
                    } else {
                        current_block = 17500079516916021833;
                    }
                    match current_block {
                        16754986508692159943 => {}
                        _ => {
                            first = 1i32;
                            cur = (*(*build_info).mm_data.mm_multipart.mm_mp_list).first;
                            loop {
                                if cur.is_null() {
                                    current_block = 3546145585875536353;
                                    break;
                                }
                                let mut subpart: *mut mailmime = 0 as *mut mailmime;
                                subpart = (*cur).data as *mut mailmime;
                                if 0 == first {
                                    r = mailimf_string_write_driver(
                                        do_write,
                                        data,
                                        col,
                                        b"\r\n\x00" as *const u8 as *const libc::c_char,
                                        2i32 as size_t,
                                    );
                                    if r != MAILIMF_NO_ERROR as libc::c_int {
                                        res = r;
                                        current_block = 16754986508692159943;
                                        break;
                                    }
                                } else {
                                    first = 0i32
                                }
                                r = mailimf_string_write_driver(
                                    do_write,
                                    data,
                                    col,
                                    b"--\x00" as *const u8 as *const libc::c_char,
                                    2i32 as size_t,
                                );
                                if r != MAILIMF_NO_ERROR as libc::c_int {
                                    res = r;
                                    current_block = 16754986508692159943;
                                    break;
                                } else if boundary.is_null() {
                                    res = MAILIMF_ERROR_MEMORY as libc::c_int;
                                    current_block = 16754986508692159943;
                                    break;
                                } else {
                                    r = mailimf_string_write_driver(
                                        do_write,
                                        data,
                                        col,
                                        boundary,
                                        strlen(boundary),
                                    );
                                    if r != MAILIMF_NO_ERROR as libc::c_int {
                                        res = r;
                                        current_block = 16754986508692159943;
                                        break;
                                    } else {
                                        r = mailimf_string_write_driver(
                                            do_write,
                                            data,
                                            col,
                                            b"\r\n\x00" as *const u8 as *const libc::c_char,
                                            2i32 as size_t,
                                        );
                                        if r != MAILIMF_NO_ERROR as libc::c_int {
                                            res = r;
                                            current_block = 16754986508692159943;
                                            break;
                                        } else {
                                            r = mailmime_sub_write_driver(
                                                do_write, data, col, subpart,
                                            );
                                            if r != MAILIMF_NO_ERROR as libc::c_int {
                                                res = r;
                                                current_block = 16754986508692159943;
                                                break;
                                            } else {
                                                cur = if !cur.is_null() {
                                                    (*cur).next
                                                } else {
                                                    0 as *mut clistcell
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            match current_block {
                                16754986508692159943 => {}
                                _ => {
                                    r = mailimf_string_write_driver(
                                        do_write,
                                        data,
                                        col,
                                        b"\r\n\x00" as *const u8 as *const libc::c_char,
                                        2i32 as size_t,
                                    );
                                    if r != MAILIMF_NO_ERROR as libc::c_int {
                                        res = r;
                                        current_block = 16754986508692159943;
                                    } else {
                                        r = mailimf_string_write_driver(
                                            do_write,
                                            data,
                                            col,
                                            b"--\x00" as *const u8 as *const libc::c_char,
                                            2i32 as size_t,
                                        );
                                        if r != MAILIMF_NO_ERROR as libc::c_int {
                                            res = r;
                                            current_block = 16754986508692159943;
                                        } else if boundary.is_null() {
                                            res = MAILIMF_ERROR_MEMORY as libc::c_int;
                                            current_block = 16754986508692159943;
                                        } else {
                                            r = mailimf_string_write_driver(
                                                do_write,
                                                data,
                                                col,
                                                boundary,
                                                strlen(boundary),
                                            );
                                            if r != MAILIMF_NO_ERROR as libc::c_int {
                                                res = r;
                                                current_block = 16754986508692159943;
                                            } else {
                                                r = mailimf_string_write_driver(
                                                    do_write,
                                                    data,
                                                    col,
                                                    b"--\x00" as *const u8 as *const libc::c_char,
                                                    2i32 as size_t,
                                                );
                                                if r != MAILIMF_NO_ERROR as libc::c_int {
                                                    res = r;
                                                    current_block = 16754986508692159943;
                                                } else {
                                                    r = mailimf_string_write_driver(
                                                        do_write,
                                                        data,
                                                        col,
                                                        b"\r\n\x00" as *const u8
                                                            as *const libc::c_char,
                                                        2i32 as size_t,
                                                    );
                                                    if r != MAILIMF_NO_ERROR as libc::c_int {
                                                        res = r;
                                                        current_block = 16754986508692159943;
                                                    } else if !(*build_info)
                                                        .mm_data
                                                        .mm_multipart
                                                        .mm_epilogue
                                                        .is_null()
                                                    {
                                                        r = mailmime_data_write_driver(
                                                            do_write,
                                                            data,
                                                            col,
                                                            (*build_info)
                                                                .mm_data
                                                                .mm_multipart
                                                                .mm_epilogue,
                                                            1i32,
                                                        );
                                                        if r != MAILIMF_NO_ERROR as libc::c_int {
                                                            res = r;
                                                            current_block = 16754986508692159943;
                                                        } else {
                                                            current_block = 7639320476250304355;
                                                        }
                                                    } else {
                                                        current_block = 7639320476250304355;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                3 => {
                    if !(*build_info).mm_data.mm_message.mm_fields.is_null() {
                        r = mailimf_fields_write_driver(
                            do_write,
                            data,
                            col,
                            (*build_info).mm_data.mm_message.mm_fields,
                        );
                        if r != MAILIMF_NO_ERROR as libc::c_int {
                            res = r;
                            current_block = 16754986508692159943;
                        } else {
                            current_block = 2798392256336243897;
                        }
                    } else {
                        current_block = 2798392256336243897;
                    }
                    match current_block {
                        16754986508692159943 => {}
                        _ => {
                            if !(*build_info).mm_mime_fields.is_null() {
                                let mut r_0: libc::c_int = 0;
                                let mut cur_0: *mut clistiter = 0 as *mut clistiter;
                                cur_0 = (*(*(*build_info).mm_mime_fields).fld_list).first;
                                loop {
                                    if cur_0.is_null() {
                                        current_block = 562309032768341766;
                                        break;
                                    }
                                    let mut field: *mut mailmime_field = 0 as *mut mailmime_field;
                                    field = (*cur_0).data as *mut mailmime_field;
                                    if (*field).fld_type == MAILMIME_FIELD_VERSION as libc::c_int {
                                        r_0 =
                                            mailmime_field_write_driver(do_write, data, col, field);
                                        if r_0 != MAILIMF_NO_ERROR as libc::c_int {
                                            res = r_0;
                                            current_block = 16754986508692159943;
                                            break;
                                        }
                                    }
                                    cur_0 = if !cur_0.is_null() {
                                        (*cur_0).next
                                    } else {
                                        0 as *mut clistcell
                                    }
                                }
                            } else {
                                current_block = 562309032768341766;
                            }
                            match current_block {
                                16754986508692159943 => {}
                                _ => {
                                    /* encapsuled message */
                                    if !(*build_info).mm_data.mm_message.mm_msg_mime.is_null() {
                                        r = mailmime_sub_write_driver(
                                            do_write,
                                            data,
                                            col,
                                            (*build_info).mm_data.mm_message.mm_msg_mime,
                                        );
                                        if r != MAILIMF_NO_ERROR as libc::c_int {
                                            res = r;
                                            current_block = 16754986508692159943;
                                        } else {
                                            current_block = 7639320476250304355;
                                        }
                                    } else {
                                        current_block = 7639320476250304355;
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {
                    current_block = 7639320476250304355;
                }
            }
            match current_block {
                16754986508692159943 => {
                    free(boundary as *mut libc::c_void);
                }
                _ => {
                    free(boundary as *mut libc::c_void);
                    return MAILIMF_NO_ERROR as libc::c_int;
                }
            }
        }
        _ => {}
    }
    return res;
}
unsafe fn mailmime_sub_write_driver(
    mut do_write: Option<
        unsafe fn(_: *mut libc::c_void, _: *const libc::c_char, _: size_t) -> libc::c_int,
    >,
    mut data: *mut libc::c_void,
    mut col: *mut libc::c_int,
    mut build_info: *mut mailmime,
) -> libc::c_int {
    let mut r: libc::c_int = 0;
    if !(*build_info).mm_content_type.is_null() {
        r = mailmime_content_write_driver(do_write, data, col, (*build_info).mm_content_type);
        if r != MAILIMF_NO_ERROR as libc::c_int {
            return r;
        }
    }
    if (*build_info).mm_type != MAILMIME_MESSAGE as libc::c_int {
        if !(*build_info).mm_mime_fields.is_null() {
            r = mailmime_fields_write_driver(do_write, data, col, (*build_info).mm_mime_fields);
            if r != MAILIMF_NO_ERROR as libc::c_int {
                return r;
            }
        }
    } else if !(*build_info).mm_mime_fields.is_null() {
        let mut r_0: libc::c_int = 0;
        let mut cur: *mut clistiter = 0 as *mut clistiter;
        cur = (*(*(*build_info).mm_mime_fields).fld_list).first;
        while !cur.is_null() {
            let mut field: *mut mailmime_field = 0 as *mut mailmime_field;
            field = (*cur).data as *mut mailmime_field;
            if (*field).fld_type != MAILMIME_FIELD_VERSION as libc::c_int {
                r_0 = mailmime_field_write_driver(do_write, data, col, field);
                if r_0 != MAILIMF_NO_ERROR as libc::c_int {
                    return r_0;
                }
            }
            cur = if !cur.is_null() {
                (*cur).next
            } else {
                0 as *mut clistcell
            }
        }
    }
    r = mailimf_string_write_driver(
        do_write,
        data,
        col,
        b"\r\n\x00" as *const u8 as *const libc::c_char,
        2i32 as size_t,
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    return mailmime_part_write_driver(do_write, data, col, build_info);
}

pub unsafe fn mailmime_data_write_driver(
    mut do_write: Option<
        unsafe fn(_: *mut libc::c_void, _: *const libc::c_char, _: size_t) -> libc::c_int,
    >,
    mut data: *mut libc::c_void,
    mut col: *mut libc::c_int,
    mut mime_data: *mut mailmime_data,
    mut istext: libc::c_int,
) -> libc::c_int {
    let mut current_block: u64 = 0;
    let mut fd: libc::c_int = 0;
    let mut r: libc::c_int = 0;
    let mut text: *mut libc::c_char = 0 as *mut libc::c_char;

    let mut res: libc::c_int = 0;
    match (*mime_data).dt_type {
        0 => {
            if 0 != (*mime_data).dt_encoded {
                r = mailimf_string_write_driver(
                    do_write,
                    data,
                    col,
                    (*mime_data).dt_data.dt_text.dt_data,
                    (*mime_data).dt_data.dt_text.dt_length,
                );
                if r != MAILIMF_NO_ERROR as libc::c_int {
                    return r;
                }
            } else {
                r = mailmime_text_content_write_driver(
                    do_write,
                    data,
                    col,
                    (*mime_data).dt_encoding,
                    istext,
                    (*mime_data).dt_data.dt_text.dt_data,
                    (*mime_data).dt_data.dt_text.dt_length,
                );
                if r != MAILIMF_NO_ERROR as libc::c_int {
                    return r;
                }
            }
        }
        1 => {
            let filename = CStr::from_ptr((*mime_data).dt_data.dt_filename)
                .to_str()
                .unwrap();
            if let Ok(file) = std::fs::File::open(filename) {
                if let Ok(mut text) = memmap::MmapOptions::new().map_copy(&file) {
                    if 0 != (*mime_data).dt_encoded {
                        r = mailimf_string_write_driver(
                            do_write,
                            data,
                            col,
                            text.as_ptr() as *const _,
                            text.len(),
                        );
                        if r != MAILIMF_NO_ERROR as libc::c_int {
                            res = r;
                            current_block = 1055471768422549395;
                        } else {
                            current_block = 1538046216550696469;
                        }
                    } else {
                        r = mailmime_text_content_write_driver(
                            do_write,
                            data,
                            col,
                            (*mime_data).dt_encoding,
                            istext,
                            text.as_ptr() as *const _,
                            text.len(),
                        );
                        if r != MAILIMF_NO_ERROR as libc::c_int {
                            res = r;
                            current_block = 1055471768422549395;
                        } else {
                            current_block = 1538046216550696469;
                        }
                    }
                    match current_block {
                        1055471768422549395 => {
                            current_block = 5221028069996397600;
                        }
                        _ => {
                            current_block = 9853141518545631134;
                        }
                    }
                } else {
                    res = MAILIMF_ERROR_FILE as libc::c_int;
                    current_block = 5221028069996397600;
                }
                match current_block {
                    5221028069996397600 => {}
                    _ => {
                        close(fd);
                        if r != MAILIMF_NO_ERROR as libc::c_int {
                            return r;
                        }
                        current_block = 10891380440665537214;
                    }
                }
            } else {
                res = MAILIMF_ERROR_FILE as libc::c_int;
                current_block = 10275258781883576179;
            }
            match current_block {
                10891380440665537214 => {}
                _ => {
                    current_block = 10275258781883576179;
                }
            }
            match current_block {
                10891380440665537214 => {}
                _ => return res,
            }
        }
        _ => {}
    }
    return MAILIMF_NO_ERROR as libc::c_int;
}
/* ****************************************************************** */
/* message */
/*
static int mailmime_data_write_driver(int (* do_write)(void *, const char *, size_t), void * data, int * col,
                   struct mailmime_data * data,
                   int is_text);
*/
unsafe fn mailmime_text_content_write_driver(
    mut do_write: Option<
        unsafe fn(_: *mut libc::c_void, _: *const libc::c_char, _: size_t) -> libc::c_int,
    >,
    mut data: *mut libc::c_void,
    mut col: *mut libc::c_int,
    mut encoding: libc::c_int,
    mut istext: libc::c_int,
    mut text: *const libc::c_char,
    mut size: size_t,
) -> libc::c_int {
    match encoding {
        4 => {
            return mailmime_quoted_printable_write_driver(do_write, data, col, istext, text, size)
        }
        5 => return mailmime_base64_write_driver(do_write, data, col, text, size),
        1 | 2 | 3 | _ => return mailimf_string_write_driver(do_write, data, col, text, size),
    };
}

pub unsafe fn mailmime_base64_write_driver(
    mut do_write: Option<
        unsafe fn(_: *mut libc::c_void, _: *const libc::c_char, _: size_t) -> libc::c_int,
    >,
    mut data: *mut libc::c_void,
    mut col: *mut libc::c_int,
    mut text: *const libc::c_char,
    mut size: size_t,
) -> libc::c_int {
    let mut a: libc::c_int = 0;
    let mut b: libc::c_int = 0;
    let mut c: libc::c_int = 0;
    let mut remains: size_t = 0;
    let mut p: *const libc::c_char = 0 as *const libc::c_char;
    let mut count: size_t = 0;
    let mut ogroup: [libc::c_char; 4] = [0; 4];
    let mut r: libc::c_int = 0;
    remains = size;
    p = text;
    while remains > 0i32 as libc::size_t {
        match remains {
            1 => {
                a = *p.offset(0isize) as libc::c_uchar as libc::c_int;
                b = 0i32;
                c = 0i32;
                count = 1i32 as size_t
            }
            2 => {
                a = *p.offset(0isize) as libc::c_uchar as libc::c_int;
                b = *p.offset(1isize) as libc::c_uchar as libc::c_int;
                c = 0i32;
                count = 2i32 as size_t
            }
            _ => {
                a = *p.offset(0isize) as libc::c_uchar as libc::c_int;
                b = *p.offset(1isize) as libc::c_uchar as libc::c_int;
                c = *p.offset(2isize) as libc::c_uchar as libc::c_int;
                count = 3i32 as size_t
            }
        }
        ogroup[0usize] = base64_encoding[(a >> 2i32) as usize];
        ogroup[1usize] = base64_encoding[((a & 3i32) << 4i32 | b >> 4i32) as usize];
        ogroup[2usize] = base64_encoding[((b & 0xfi32) << 2i32 | c >> 6i32) as usize];
        ogroup[3usize] = base64_encoding[(c & 0x3fi32) as usize];
        match count {
            1 => {
                ogroup[2usize] = '=' as i32 as libc::c_char;
                ogroup[3usize] = '=' as i32 as libc::c_char
            }
            2 => ogroup[3usize] = '=' as i32 as libc::c_char,
            _ => {}
        }
        if *col + 4i32 > 76i32 {
            r = mailimf_string_write_driver(
                do_write,
                data,
                col,
                b"\r\n\x00" as *const u8 as *const libc::c_char,
                2i32 as size_t,
            );
            if r != MAILIMF_NO_ERROR as libc::c_int {
                return r;
            }
        }
        r = mailimf_string_write_driver(do_write, data, col, ogroup.as_mut_ptr(), 4i32 as size_t);
        if r != MAILIMF_NO_ERROR as libc::c_int {
            return r;
        }
        remains = (remains as libc::size_t).wrapping_sub(count) as size_t as size_t;
        p = p.offset(count as isize)
    }
    r = mailimf_string_write_driver(
        do_write,
        data,
        col,
        b"\r\n\x00" as *const u8 as *const libc::c_char,
        2i32 as size_t,
    );
    return MAILIMF_NO_ERROR as libc::c_int;
}
static mut base64_encoding: [libc::c_char; 65] = [
    65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88,
    89, 90, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114,
    115, 116, 117, 118, 119, 120, 121, 122, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 43, 47, 0,
];

pub unsafe fn mailmime_quoted_printable_write_driver(
    mut do_write: Option<
        unsafe fn(_: *mut libc::c_void, _: *const libc::c_char, _: size_t) -> libc::c_int,
    >,
    mut data: *mut libc::c_void,
    mut col: *mut libc::c_int,
    mut istext: libc::c_int,
    mut text: *const libc::c_char,
    mut size: size_t,
) -> libc::c_int {
    let mut i: size_t = 0;
    let mut start: *const libc::c_char = 0 as *const libc::c_char;
    let mut len: size_t = 0;
    let mut r: libc::c_int = 0;
    let mut state: libc::c_int = 0;
    start = text;
    len = 0i32 as size_t;
    state = STATE_INIT as libc::c_int;
    i = 0i32 as size_t;
    while i < size {
        let mut ch: libc::c_uchar = 0;
        if (*col as libc::size_t).wrapping_add(len) > 72i32 as libc::size_t {
            r = write_remaining(do_write, data, col, &mut start, &mut len);
            if r != MAILIMF_NO_ERROR as libc::c_int {
                return r;
            }
            start = text.offset(i as isize);
            r = mailimf_string_write_driver(
                do_write,
                data,
                col,
                b"=\r\n\x00" as *const u8 as *const libc::c_char,
                3i32 as size_t,
            );
            if r != MAILIMF_NO_ERROR as libc::c_int {
                return r;
            }
        }
        ch = *text.offset(i as isize) as libc::c_uchar;
        match state {
            0 => {
                let mut current_block_50: u64;
                match ch as libc::c_int {
                    32 | 9 => {
                        state = STATE_SPACE as libc::c_int;
                        len = len.wrapping_add(1);
                        i = i.wrapping_add(1);
                        current_block_50 = 3546145585875536353;
                    }
                    13 => {
                        state = STATE_CR as libc::c_int;
                        i = i.wrapping_add(1);
                        current_block_50 = 3546145585875536353;
                    }
                    33 | 34 | 35 | 36 | 64 | 91 | 92 | 93 | 94 | 96 | 123 | 124 | 125 | 126
                    | 61 | 63 | 95 => {
                        /* there is no more 'From' at the beginning of a line */
                        current_block_50 = 177397332496894159;
                    }
                    70 => {
                        current_block_50 = 177397332496894159;
                    }
                    _ => {
                        if 0 != istext && ch as libc::c_int == '\n' as i32 {
                            r = write_remaining(do_write, data, col, &mut start, &mut len);
                            if r != MAILIMF_NO_ERROR as libc::c_int {
                                return r;
                            }
                            start = text.offset(i as isize).offset(1isize);
                            r = mailimf_string_write_driver(
                                do_write,
                                data,
                                col,
                                b"\r\n\x00" as *const u8 as *const libc::c_char,
                                2i32 as size_t,
                            );
                            if r != MAILIMF_NO_ERROR as libc::c_int {
                                return r;
                            }
                            i = i.wrapping_add(1)
                        } else if ch as libc::c_int >= 33i32 && ch as libc::c_int <= 60i32
                            || ch as libc::c_int >= 62i32 && ch as libc::c_int <= 126i32
                        {
                            len = len.wrapping_add(1);
                            i = i.wrapping_add(1)
                        } else {
                            r = write_remaining(do_write, data, col, &mut start, &mut len);
                            if r != MAILIMF_NO_ERROR as libc::c_int {
                                return r;
                            }
                            start = text.offset(i as isize).offset(1isize);

                            let raw = format!("={:02X}", (ch as libc::c_int));
                            let raw_c = std::ffi::CString::new(raw).unwrap();
                            let mut hexstr = strdup(raw_c.as_ptr());
                            r = mailimf_string_write_driver(
                                do_write,
                                data,
                                col,
                                hexstr,
                                3i32 as size_t,
                            );
                            if r != MAILIMF_NO_ERROR as libc::c_int {
                                return r;
                            }
                            i = i.wrapping_add(1)
                        }
                        current_block_50 = 3546145585875536353;
                    }
                }
                match current_block_50 {
                    177397332496894159 => {
                        r = write_remaining(do_write, data, col, &mut start, &mut len);
                        if r != MAILIMF_NO_ERROR as libc::c_int {
                            return r;
                        }
                        start = text.offset(i as isize).offset(1isize);
                        let raw = format!("={:02X}", ch as libc::c_int);
                        let raw_c = std::ffi::CString::new(raw).unwrap();
                        let mut hexstr = strdup(raw_c.as_ptr());
                        r = mailimf_string_write_driver(
                            do_write,
                            data,
                            col,
                            hexstr,
                            3i32 as size_t,
                        );
                        if r != MAILIMF_NO_ERROR as libc::c_int {
                            return r;
                        }
                        i = i.wrapping_add(1)
                    }
                    _ => {}
                }
            }
            1 => match ch as libc::c_int {
                10 => {
                    r = write_remaining(do_write, data, col, &mut start, &mut len);
                    if r != MAILIMF_NO_ERROR as libc::c_int {
                        return r;
                    }
                    start = text.offset(i as isize).offset(1isize);
                    r = mailimf_string_write_driver(
                        do_write,
                        data,
                        col,
                        b"\r\n\x00" as *const u8 as *const libc::c_char,
                        2i32 as size_t,
                    );
                    if r != MAILIMF_NO_ERROR as libc::c_int {
                        return r;
                    }
                    i = i.wrapping_add(1);
                    state = STATE_INIT as libc::c_int
                }
                _ => {
                    r = write_remaining(do_write, data, col, &mut start, &mut len);
                    if r != MAILIMF_NO_ERROR as libc::c_int {
                        return r;
                    }
                    start = text.offset(i as isize);
                    let raw = format!("={:02X}", b'\r' as i32);
                    let raw_c = std::ffi::CString::new(raw).unwrap();
                    let mut hexstr = strdup(raw_c.as_ptr());
                    r = mailimf_string_write_driver(do_write, data, col, hexstr, 3i32 as size_t);
                    if r != MAILIMF_NO_ERROR as libc::c_int {
                        return r;
                    }
                    state = STATE_INIT as libc::c_int
                }
            },
            2 => match ch as libc::c_int {
                13 => {
                    state = STATE_SPACE_CR as libc::c_int;
                    i = i.wrapping_add(1)
                }
                10 => {
                    r = write_remaining(do_write, data, col, &mut start, &mut len);
                    if r != MAILIMF_NO_ERROR as libc::c_int {
                        return r;
                    }
                    start = text.offset(i as isize).offset(1isize);
                    let raw = format!(
                        "={:02X}\r\n",
                        *text.offset(i.wrapping_sub(1i32 as libc::size_t) as isize) as libc::c_int
                    );
                    let raw_c = std::ffi::CString::new(raw).unwrap();
                    let mut hexstr = strdup(raw_c.as_ptr());

                    r = mailimf_string_write_driver(do_write, data, col, hexstr, strlen(hexstr));
                    if r != MAILIMF_NO_ERROR as libc::c_int {
                        return r;
                    }
                    state = STATE_INIT as libc::c_int;
                    i = i.wrapping_add(1)
                }
                32 | 9 => {
                    len = len.wrapping_add(1);
                    i = i.wrapping_add(1)
                }
                _ => state = STATE_INIT as libc::c_int,
            },
            3 => match ch as libc::c_int {
                10 => {
                    r = write_remaining(do_write, data, col, &mut start, &mut len);
                    if r != MAILIMF_NO_ERROR as libc::c_int {
                        return r;
                    }
                    start = text.offset(i as isize).offset(1isize);
                    let raw = format!(
                        "={:02X}\r\n",
                        *text.offset(i.wrapping_sub(2i32 as libc::size_t) as isize) as libc::c_int
                    );
                    let raw_c = std::ffi::CString::new(raw).unwrap();
                    let mut hexstr = strdup(raw_c.as_ptr());

                    r = mailimf_string_write_driver(do_write, data, col, hexstr, strlen(hexstr));
                    if r != MAILIMF_NO_ERROR as libc::c_int {
                        return r;
                    }
                    state = STATE_INIT as libc::c_int;
                    i = i.wrapping_add(1)
                }
                _ => {
                    r = write_remaining(do_write, data, col, &mut start, &mut len);
                    if r != MAILIMF_NO_ERROR as libc::c_int {
                        return r;
                    }
                    start = text.offset(i as isize).offset(1isize);
                    let raw = format!(
                        "{}={:02X}\r\n",
                        (*text.offset(i.wrapping_sub(2i32 as libc::size_t) as isize) as u8 as char),
                        b'\r' as i32
                    );
                    let raw_c = std::ffi::CString::new(raw).unwrap();
                    let mut hexstr = strdup(raw_c.as_ptr());

                    r = mailimf_string_write_driver(do_write, data, col, hexstr, strlen(hexstr));
                    if r != MAILIMF_NO_ERROR as libc::c_int {
                        return r;
                    }
                    state = STATE_INIT as libc::c_int
                }
            },
            _ => {}
        }
    }
    r = write_remaining(do_write, data, col, &mut start, &mut len);
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    return MAILIMF_NO_ERROR as libc::c_int;
}
#[inline]
unsafe fn write_remaining(
    mut do_write: Option<
        unsafe fn(_: *mut libc::c_void, _: *const libc::c_char, _: size_t) -> libc::c_int,
    >,
    mut data: *mut libc::c_void,
    mut col: *mut libc::c_int,
    mut pstart: *mut *const libc::c_char,
    mut plen: *mut size_t,
) -> libc::c_int {
    let mut r: libc::c_int = 0;
    if *plen > 0i32 as libc::size_t {
        r = mailimf_string_write_driver(do_write, data, col, *pstart, *plen);
        if r != MAILIMF_NO_ERROR as libc::c_int {
            return r;
        }
        *plen = 0i32 as size_t
    }
    return MAILIMF_NO_ERROR as libc::c_int;
}
