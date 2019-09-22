use crate::clist::*;
use crate::mailimf::*;
use crate::mailimf_types::*;
use crate::mailmime::*;
use crate::mailmime_types::*;
use crate::mailmime_types_helper::*;
use crate::mmapstring::*;
use crate::other::*;

pub const MAILMIME_DEFAULT_TYPE_TEXT_PLAIN: libc::c_uint = 0;
pub const MULTIPART_NEXT_STATE_2: libc::c_uint = 2;
pub const MULTIPART_NEXT_STATE_1: libc::c_uint = 1;
pub const MULTIPART_NEXT_STATE_0: libc::c_uint = 0;
pub const MULTIPART_CLOSE_STATE_4: libc::c_uint = 4;
pub const MULTIPART_CLOSE_STATE_3: libc::c_uint = 3;
pub const MULTIPART_CLOSE_STATE_2: libc::c_uint = 2;
pub const MULTIPART_CLOSE_STATE_1: libc::c_uint = 1;
pub const MULTIPART_CLOSE_STATE_0: libc::c_uint = 0;
pub const BODY_PART_DASH2_STATE_0: libc::c_uint = 0;
pub const BODY_PART_DASH2_STATE_6: libc::c_uint = 6;
pub const BODY_PART_DASH2_STATE_5: libc::c_uint = 5;
pub const BODY_PART_DASH2_STATE_4: libc::c_uint = 4;
pub const BODY_PART_DASH2_STATE_2: libc::c_uint = 2;
pub const BODY_PART_DASH2_STATE_1: libc::c_uint = 1;
pub const BODY_PART_DASH2_STATE_3: libc::c_uint = 3;
pub const PREAMBLE_STATE_A: libc::c_uint = 1;
pub const PREAMBLE_STATE_E: libc::c_uint = 6;
pub const PREAMBLE_STATE_D: libc::c_uint = 5;
pub const PREAMBLE_STATE_A0: libc::c_uint = 0;
pub const PREAMBLE_STATE_C: libc::c_uint = 4;
pub const PREAMBLE_STATE_B: libc::c_uint = 3;
pub const PREAMBLE_STATE_A1: libc::c_uint = 2;
pub const MAILMIME_DEFAULT_TYPE_MESSAGE: libc::c_uint = 1;
pub const STATE_NORMAL: libc::c_uint = 0;
pub const STATE_CR: libc::c_uint = 3;
pub const STATE_CODED: libc::c_uint = 1;
pub const STATE_OUT: libc::c_uint = 2;

pub unsafe fn mailmime_content_charset_get(
    mut content: *mut mailmime_content,
) -> *mut libc::c_char {
    let mut charset: *mut libc::c_char = 0 as *mut libc::c_char;
    charset = mailmime_content_param_get(
        content,
        b"charset\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
    );
    if charset.is_null() {
        return b"us-ascii\x00" as *const u8 as *const libc::c_char as *mut libc::c_char;
    } else {
        return charset;
    };
}

pub unsafe fn mailmime_content_param_get(
    mut content: *mut mailmime_content,
    mut name: *mut libc::c_char,
) -> *mut libc::c_char {
    let mut cur: *mut clistiter = 0 as *mut clistiter;
    cur = (*(*content).ct_parameters).first;
    while !cur.is_null() {
        let mut param: *mut mailmime_parameter = 0 as *mut mailmime_parameter;
        param = (if !cur.is_null() {
            (*cur).data
        } else {
            0 as *mut libc::c_void
        }) as *mut mailmime_parameter;
        if strcasecmp((*param).pa_name, name) == 0i32 {
            return (*param).pa_value;
        }
        cur = if !cur.is_null() {
            (*cur).next
        } else {
            0 as *mut clistcell
        }
    }
    return 0 as *mut libc::c_char;
}

pub unsafe fn mailmime_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut mailmime,
) -> libc::c_int {
    let mut mime: *mut mailmime = 0 as *mut mailmime;
    let mut r: libc::c_int = 0;
    let mut res: libc::c_int = 0;
    let mut content_message: *mut mailmime_content = 0 as *mut mailmime_content;
    let mut cur_token: size_t = 0;
    let mut mime_fields: *mut mailmime_fields = 0 as *mut mailmime_fields;
    let mut data_str: *const libc::c_char = 0 as *const libc::c_char;
    let mut data_size: size_t = 0;
    let mut bp_token: size_t = 0;
    cur_token = *indx;
    content_message = mailmime_get_content_message();
    if content_message.is_null() {
        res = MAILIMF_ERROR_MEMORY as libc::c_int
    } else {
        mime_fields = mailmime_fields_new_empty();
        if mime_fields.is_null() {
            mailmime_content_free(content_message);
            res = MAILIMF_ERROR_MEMORY as libc::c_int
        } else {
            data_str = message.offset(cur_token as isize);
            data_size = length.wrapping_sub(cur_token);
            bp_token = 0i32 as size_t;
            r = mailmime_parse_with_default(
                data_str,
                data_size,
                &mut bp_token,
                MAILMIME_DEFAULT_TYPE_TEXT_PLAIN as libc::c_int,
                content_message,
                mime_fields,
                &mut mime,
            );
            cur_token = (cur_token as libc::size_t).wrapping_add(bp_token) as size_t as size_t;
            if r != MAILIMF_NO_ERROR as libc::c_int {
                mailmime_fields_free(mime_fields);
                res = r;
                mailmime_fields_free(mime_fields);
            } else {
                *indx = cur_token;
                *result = mime;
                return MAILIMF_NO_ERROR as libc::c_int;
            }
        }
    }
    return res;
}
/*
 * libEtPan! -- a mail stuff library
 *
 * Copyright (C) 2001, 2005 - DINH Viet Hoa
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. Neither the name of the libEtPan! project nor the names of its
 *    contributors may be used to endorse or promote products derived
 *    from this software without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE AUTHORS AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE AUTHORS OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 */
/*
 * $Id: mailmime_content.c,v 1.47 2011/06/28 22:13:36 hoa Exp $
 */
/*
  RFC 2045
  RFC 2046
  RFC 2047

  RFC 2231
*/
unsafe fn mailmime_parse_with_default(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut default_type: libc::c_int,
    mut content_type: *mut mailmime_content,
    mut mime_fields: *mut mailmime_fields,
    mut result: *mut *mut mailmime,
) -> libc::c_int {
    let mut current_block: u64;
    let mut cur_token: size_t = 0;
    let mut body_type: libc::c_int = 0;
    let mut encoding: libc::c_int = 0;
    let mut body: *mut mailmime_data = 0 as *mut mailmime_data;
    let mut boundary: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut fields: *mut mailimf_fields = 0 as *mut mailimf_fields;
    let mut list: *mut clist = 0 as *mut clist;
    let mut msg_mime: *mut mailmime = 0 as *mut mailmime;
    let mut mime: *mut mailmime = 0 as *mut mailmime;
    let mut r: libc::c_int = 0;
    let mut res: libc::c_int = 0;
    let mut preamble: *mut mailmime_data = 0 as *mut mailmime_data;
    let mut epilogue: *mut mailmime_data = 0 as *mut mailmime_data;
    preamble = 0 as *mut mailmime_data;
    epilogue = 0 as *mut mailmime_data;
    cur_token = *indx;
    if content_type.is_null() {
        if !mime_fields.is_null() {
            let mut cur: *mut clistiter = 0 as *mut clistiter;
            cur = (*(*mime_fields).fld_list).first;
            while !cur.is_null() {
                let mut field: *mut mailmime_field = 0 as *mut mailmime_field;
                field = (if !cur.is_null() {
                    (*cur).data
                } else {
                    0 as *mut libc::c_void
                }) as *mut mailmime_field;
                if (*field).fld_type == MAILMIME_FIELD_TYPE as libc::c_int {
                    content_type = (*field).fld_data.fld_content;
                    (*field).fld_data.fld_content = 0 as *mut mailmime_content;
                    clist_delete((*mime_fields).fld_list, cur);
                    mailmime_field_free(field);
                    /*
                      there may be a leak due to the detached content type
                      in case the function fails
                    */
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
    /* set default type if no content type */
    if content_type.is_null() {
        /* content_type is detached, in any case, we will have to free it */
        if default_type == MAILMIME_DEFAULT_TYPE_TEXT_PLAIN as libc::c_int {
            content_type = mailmime_get_content_text();
            if content_type.is_null() {
                res = MAILIMF_ERROR_MEMORY as libc::c_int;
                current_block = 16594950150283576116;
            } else {
                current_block = 7828949454673616476;
            }
        } else {
            /* message */
            body_type = MAILMIME_MESSAGE as libc::c_int;
            content_type = mailmime_get_content_message();
            if content_type.is_null() {
                res = MAILIMF_ERROR_MEMORY as libc::c_int;
                current_block = 16594950150283576116;
            } else {
                current_block = 7828949454673616476;
            }
        }
    } else {
        current_block = 7828949454673616476;
    }
    match current_block {
        7828949454673616476 => {
            boundary = 0 as *mut libc::c_char;
            match (*(*content_type).ct_type).tp_type {
                2 => match (*(*(*content_type).ct_type).tp_data.tp_composite_type).ct_type {
                    2 => {
                        boundary = mailmime_extract_boundary(content_type);
                        if boundary.is_null() {
                            body_type = MAILMIME_SINGLE as libc::c_int
                        } else {
                            body_type = MAILMIME_MULTIPLE as libc::c_int
                        }
                        current_block = 11793792312832361944;
                    }
                    1 => {
                        if strcasecmp(
                            (*content_type).ct_subtype,
                            b"rfc822\x00" as *const u8 as *const libc::c_char,
                        ) == 0i32
                        {
                            body_type = MAILMIME_MESSAGE as libc::c_int
                        } else {
                            body_type = MAILMIME_SINGLE as libc::c_int
                        }
                        current_block = 11793792312832361944;
                    }
                    _ => {
                        res = MAILIMF_ERROR_INVAL as libc::c_int;
                        current_block = 18099180955076792539;
                    }
                },
                _ => {
                    body_type = MAILMIME_SINGLE as libc::c_int;
                    current_block = 11793792312832361944;
                }
            }
            match current_block {
                11793792312832361944 => {
                    if !mime_fields.is_null() {
                        encoding = mailmime_transfer_encoding_get(mime_fields)
                    } else {
                        encoding = MAILMIME_MECHANISM_8BIT as libc::c_int
                    }
                    if body_type == MAILMIME_MESSAGE as libc::c_int {
                        match encoding {
                            4 | 5 => body_type = MAILMIME_SINGLE as libc::c_int,
                            _ => {}
                        }
                    }
                    cur_token = *indx;
                    body = mailmime_data_new(
                        MAILMIME_DATA_TEXT as libc::c_int,
                        encoding,
                        1i32,
                        message.offset(cur_token as isize),
                        length.wrapping_sub(cur_token),
                        0 as *mut libc::c_char,
                    );
                    if body.is_null() {
                        free(boundary as *mut libc::c_void);
                        res = MAILIMF_ERROR_MEMORY as libc::c_int
                    } else {
                        list = 0 as *mut clist;
                        msg_mime = 0 as *mut mailmime;
                        fields = 0 as *mut mailimf_fields;
                        match body_type {
                            3 => {
                                let mut submime_fields: *mut mailmime_fields =
                                    0 as *mut mailmime_fields;
                                r = mailimf_envelope_and_optional_fields_parse(
                                    message,
                                    length,
                                    &mut cur_token,
                                    &mut fields,
                                );
                                if r != MAILIMF_NO_ERROR as libc::c_int
                                    && r != MAILIMF_ERROR_PARSE as libc::c_int
                                {
                                    res = r;
                                    current_block = 18099180955076792539;
                                } else {
                                    r = mailimf_crlf_parse(message, length, &mut cur_token);
                                    if r != MAILIMF_NO_ERROR as libc::c_int
                                        && r != MAILIMF_ERROR_PARSE as libc::c_int
                                    {
                                        mailimf_fields_free(fields);
                                        res = r;
                                        current_block = 18099180955076792539;
                                    } else {
                                        submime_fields = 0 as *mut mailmime_fields;
                                        r = mailmime_fields_parse(fields, &mut submime_fields);
                                        if r != MAILIMF_NO_ERROR as libc::c_int
                                            && r != MAILIMF_ERROR_PARSE as libc::c_int
                                        {
                                            mailimf_fields_free(fields);
                                            res = r;
                                            current_block = 18099180955076792539;
                                        } else {
                                            remove_unparsed_mime_headers(fields);
                                            r = mailmime_parse_with_default(
                                                message,
                                                length,
                                                &mut cur_token,
                                                MAILMIME_DEFAULT_TYPE_TEXT_PLAIN as libc::c_int,
                                                0 as *mut mailmime_content,
                                                submime_fields,
                                                &mut msg_mime,
                                            );
                                            if r == MAILIMF_NO_ERROR as libc::c_int {
                                                current_block = 12065775993741208975;
                                            } else if r == MAILIMF_ERROR_PARSE as libc::c_int {
                                                mailmime_fields_free(mime_fields);
                                                msg_mime = 0 as *mut mailmime;
                                                current_block = 12065775993741208975;
                                            } else {
                                                mailmime_fields_free(mime_fields);
                                                res = r;
                                                current_block = 18099180955076792539;
                                            }
                                        }
                                    }
                                }
                            }
                            2 => {
                                let mut default_subtype: libc::c_int = 0;
                                default_subtype = MAILMIME_DEFAULT_TYPE_TEXT_PLAIN as libc::c_int;
                                if !content_type.is_null() {
                                    if strcasecmp(
                                        (*content_type).ct_subtype,
                                        b"digest\x00" as *const u8 as *const libc::c_char,
                                    ) == 0i32
                                    {
                                        default_subtype =
                                            MAILMIME_DEFAULT_TYPE_MESSAGE as libc::c_int
                                    }
                                }
                                cur_token = *indx;
                                r = mailmime_multipart_body_parse(
                                    message,
                                    length,
                                    &mut cur_token,
                                    boundary,
                                    default_subtype,
                                    &mut list,
                                    &mut preamble,
                                    &mut epilogue,
                                );
                                if r == MAILIMF_NO_ERROR as libc::c_int {
                                    current_block = 4804377075063615140;
                                } else if r == MAILIMF_ERROR_PARSE as libc::c_int {
                                    list = clist_new();
                                    if list.is_null() {
                                        res = MAILIMF_ERROR_MEMORY as libc::c_int;
                                        current_block = 18099180955076792539;
                                    } else {
                                        current_block = 4804377075063615140;
                                    }
                                } else {
                                    res = r;
                                    current_block = 18099180955076792539;
                                }
                                match current_block {
                                    18099180955076792539 => {}
                                    _ => {
                                        free(boundary as *mut libc::c_void);
                                        current_block = 12065775993741208975;
                                    }
                                }
                            }
                            _ => {
                                /* do nothing */
                                current_block = 12065775993741208975;
                            }
                        }
                        match current_block {
                            18099180955076792539 => {}
                            _ => {
                                mime = mailmime_new(
                                    body_type,
                                    message,
                                    length,
                                    mime_fields,
                                    content_type,
                                    body,
                                    preamble,
                                    epilogue,
                                    list,
                                    fields,
                                    msg_mime,
                                );
                                /* preamble */
                                /* epilogue */
                                if mime.is_null() {
                                    res = MAILIMF_ERROR_MEMORY as libc::c_int;
                                    if !epilogue.is_null() {
                                        mailmime_data_free(epilogue);
                                    }
                                    if !preamble.is_null() {
                                        mailmime_data_free(preamble);
                                    }
                                    if !msg_mime.is_null() {
                                        mailmime_free(msg_mime);
                                    }
                                    if !list.is_null() {
                                        clist_foreach(
                                            list,
                                            ::std::mem::transmute::<
                                                Option<unsafe fn(_: *mut mailmime) -> ()>,
                                                clist_func,
                                            >(Some(
                                                mailmime_free,
                                            )),
                                            0 as *mut libc::c_void,
                                        );
                                        clist_free(list);
                                    }
                                } else {
                                    *result = mime;
                                    *indx = length;
                                    return MAILIMF_NO_ERROR as libc::c_int;
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
            mailmime_content_free(content_type);
        }
        _ => {}
    }
    return res;
}
unsafe fn mailmime_multipart_body_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut boundary: *mut libc::c_char,
    mut default_subtype: libc::c_int,
    mut result: *mut *mut clist,
    mut p_preamble: *mut *mut mailmime_data,
    mut p_epilogue: *mut *mut mailmime_data,
) -> libc::c_int {
    let mut current_block: u64;
    let mut cur_token: size_t = 0;
    let mut list: *mut clist = 0 as *mut clist;
    let mut r: libc::c_int = 0;
    let mut res: libc::c_int = 0;
    let mut preamble_begin: size_t = 0;
    let mut preamble_length: size_t = 0;
    let mut preamble_end: size_t = 0;
    let mut epilogue_begin: size_t = 0;
    let mut epilogue_length: size_t = 0;
    let mut preamble: *mut mailmime_data = 0 as *mut mailmime_data;
    let mut epilogue: *mut mailmime_data = 0 as *mut mailmime_data;
    let mut part_begin: size_t = 0;
    let mut final_part: libc::c_int = 0;
    preamble = 0 as *mut mailmime_data;
    epilogue = 0 as *mut mailmime_data;
    cur_token = *indx;
    preamble_begin = cur_token;
    preamble_end = preamble_begin;
    r = mailmime_preamble_parse(message, length, &mut cur_token, 1i32);
    if r == MAILIMF_NO_ERROR as libc::c_int {
        loop {
            preamble_end = cur_token;
            r = mailmime_boundary_parse(message, length, &mut cur_token, boundary);
            if r == MAILIMF_NO_ERROR as libc::c_int {
                current_block = 16924917904204750491;
                break;
            }
            if r == MAILIMF_ERROR_PARSE as libc::c_int {
                r = mailmime_preamble_parse(message, length, &mut cur_token, 0i32);
                if r == MAILIMF_NO_ERROR as libc::c_int {
                    continue;
                }
                if r == MAILIMF_ERROR_PARSE as libc::c_int {
                    current_block = 16924917904204750491;
                    break;
                }
                res = r;
                current_block = 13657460241182544761;
                break;
            } else {
                /* do nothing */
                res = r;
                current_block = 13657460241182544761;
                break;
            }
        }
    } else {
        current_block = 16924917904204750491;
    }
    match current_block {
        16924917904204750491 => {
            preamble_end = (preamble_end as libc::size_t).wrapping_sub(2i32 as libc::size_t)
                as size_t as size_t;
            if preamble_end != preamble_begin {
                if *message.offset(preamble_end.wrapping_sub(1i32 as libc::size_t) as isize)
                    as libc::c_int
                    == '\n' as i32
                {
                    preamble_end = preamble_end.wrapping_sub(1);
                    if preamble_end.wrapping_sub(1i32 as libc::size_t) >= preamble_begin {
                        if *message.offset(preamble_end.wrapping_sub(1i32 as libc::size_t) as isize)
                            as libc::c_int
                            == '\r' as i32
                        {
                            preamble_end = preamble_end.wrapping_sub(1)
                        }
                    }
                } else if *message.offset(preamble_end.wrapping_sub(1i32 as libc::size_t) as isize)
                    as libc::c_int
                    == '\r' as i32
                {
                    preamble_end = preamble_end.wrapping_sub(1)
                }
            }
            preamble_length = preamble_end.wrapping_sub(preamble_begin);
            part_begin = cur_token;
            loop {
                r = mailmime_lwsp_parse(message, length, &mut cur_token);
                if r != MAILIMF_NO_ERROR as libc::c_int && r != MAILIMF_ERROR_PARSE as libc::c_int {
                    res = r;
                    current_block = 13657460241182544761;
                    break;
                } else {
                    r = mailimf_crlf_parse(message, length, &mut cur_token);
                    if r == MAILIMF_NO_ERROR as libc::c_int {
                        part_begin = cur_token
                    } else if r == MAILIMF_ERROR_PARSE as libc::c_int {
                        /* do nothing */
                        current_block = 9353995356876505083;
                        break;
                    } else {
                        res = r;
                        current_block = 13657460241182544761;
                        break;
                    }
                }
            }
            match current_block {
                13657460241182544761 => {}
                _ => {
                    cur_token = part_begin;
                    list = clist_new();
                    if list.is_null() {
                        res = MAILIMF_ERROR_MEMORY as libc::c_int
                    } else {
                        final_part = 0i32;
                        loop {
                            if !(0 == final_part) {
                                current_block = 15447629348493591490;
                                break;
                            }
                            let mut bp_token: size_t = 0;
                            let mut mime_bp: *mut mailmime = 0 as *mut mailmime;
                            let mut data_str: *const libc::c_char = 0 as *const libc::c_char;
                            let mut data_size: size_t = 0;
                            let mut fields: *mut mailimf_fields = 0 as *mut mailimf_fields;
                            let mut mime_fields: *mut mailmime_fields = 0 as *mut mailmime_fields;
                            r = mailmime_body_part_dash2_transport_crlf_parse(
                                message,
                                length,
                                &mut cur_token,
                                boundary,
                                &mut data_str,
                                &mut data_size,
                            );
                            if r == MAILIMF_ERROR_PARSE as libc::c_int {
                                r = mailmime_body_part_dash2_close_parse(
                                    message,
                                    length,
                                    &mut cur_token,
                                    boundary,
                                    &mut data_str,
                                    &mut data_size,
                                );
                                if r == MAILIMF_NO_ERROR as libc::c_int {
                                    final_part = 1i32
                                }
                            }
                            if r == MAILIMF_NO_ERROR as libc::c_int {
                                bp_token = 0i32 as size_t;
                                r = mailimf_optional_fields_parse(
                                    data_str,
                                    data_size,
                                    &mut bp_token,
                                    &mut fields,
                                );
                                if r != MAILIMF_NO_ERROR as libc::c_int
                                    && r != MAILIMF_ERROR_PARSE as libc::c_int
                                {
                                    res = r;
                                    current_block = 6612762688763383599;
                                    break;
                                } else {
                                    r = mailimf_crlf_parse(data_str, data_size, &mut bp_token);
                                    if r != MAILIMF_NO_ERROR as libc::c_int
                                        && r != MAILIMF_ERROR_PARSE as libc::c_int
                                    {
                                        mailimf_fields_free(fields);
                                        res = r;
                                        current_block = 6612762688763383599;
                                        break;
                                    } else {
                                        mime_fields = 0 as *mut mailmime_fields;
                                        r = mailmime_fields_parse(fields, &mut mime_fields);
                                        mailimf_fields_free(fields);
                                        if r != MAILIMF_NO_ERROR as libc::c_int
                                            && r != MAILIMF_ERROR_PARSE as libc::c_int
                                        {
                                            res = r;
                                            current_block = 6612762688763383599;
                                            break;
                                        } else {
                                            r = mailmime_parse_with_default(
                                                data_str,
                                                data_size,
                                                &mut bp_token,
                                                default_subtype,
                                                0 as *mut mailmime_content,
                                                mime_fields,
                                                &mut mime_bp,
                                            );
                                            if r == MAILIMF_NO_ERROR as libc::c_int {
                                                r = clist_insert_after(
                                                    list,
                                                    (*list).last,
                                                    mime_bp as *mut libc::c_void,
                                                );
                                                if r < 0i32 {
                                                    mailmime_free(mime_bp);
                                                    res = MAILIMF_ERROR_MEMORY as libc::c_int;
                                                    current_block = 6612762688763383599;
                                                    break;
                                                } else {
                                                    r = mailmime_multipart_next_parse(
                                                        message,
                                                        length,
                                                        &mut cur_token,
                                                    );
                                                    r == MAILIMF_NO_ERROR as libc::c_int;
                                                }
                                            } else if r == MAILIMF_ERROR_PARSE as libc::c_int {
                                                mailmime_fields_free(mime_fields);
                                                current_block = 15447629348493591490;
                                                break;
                                            } else {
                                                mailmime_fields_free(mime_fields);
                                                res = r;
                                                current_block = 6612762688763383599;
                                                break;
                                            }
                                        }
                                    }
                                }
                            } else {
                                /* do nothing */
                                res = r;
                                current_block = 6612762688763383599;
                                break;
                            }
                        }
                        match current_block {
                            15447629348493591490 => {
                                epilogue_begin = length;
                                /* parse transport-padding */
                                loop {
                                    r = mailmime_lwsp_parse(message, length, &mut cur_token);
                                    if r != MAILIMF_NO_ERROR as libc::c_int
                                        && r != MAILIMF_ERROR_PARSE as libc::c_int
                                    {
                                        res = r;
                                        current_block = 6612762688763383599;
                                        break;
                                    } else if r == MAILIMF_ERROR_PARSE as libc::c_int {
                                        current_block = 13201766686570145889;
                                        break;
                                    }
                                }
                                match current_block {
                                    6612762688763383599 => {}
                                    _ => {
                                        r = mailimf_crlf_parse(message, length, &mut cur_token);
                                        if r == MAILIMF_NO_ERROR as libc::c_int {
                                            epilogue_begin = cur_token;
                                            current_block = 1739363794695357236;
                                        } else if r != MAILIMF_ERROR_PARSE as libc::c_int {
                                            res = r;
                                            current_block = 6612762688763383599;
                                        } else {
                                            current_block = 1739363794695357236;
                                        }
                                        match current_block {
                                            6612762688763383599 => {}
                                            _ => {
                                                epilogue_length =
                                                    length.wrapping_sub(epilogue_begin);
                                                if preamble_length != 0i32 as libc::size_t {
                                                    preamble = mailmime_data_new(
                                                        MAILMIME_DATA_TEXT as libc::c_int,
                                                        MAILMIME_MECHANISM_8BIT as libc::c_int,
                                                        1i32,
                                                        message.offset(preamble_begin as isize),
                                                        preamble_length,
                                                        0 as *mut libc::c_char,
                                                    );
                                                    if preamble.is_null() {
                                                        res = MAILIMF_ERROR_MEMORY as libc::c_int;
                                                        current_block = 6612762688763383599;
                                                    } else {
                                                        current_block = 5636883459695696059;
                                                    }
                                                } else {
                                                    current_block = 5636883459695696059;
                                                }
                                                match current_block {
                                                    6612762688763383599 => {}
                                                    _ => {
                                                        if epilogue_length != 0i32 as libc::size_t {
                                                            epilogue = mailmime_data_new(
                                                                MAILMIME_DATA_TEXT as libc::c_int,
                                                                MAILMIME_MECHANISM_8BIT
                                                                    as libc::c_int,
                                                                1i32,
                                                                message.offset(
                                                                    epilogue_begin as isize,
                                                                ),
                                                                epilogue_length,
                                                                0 as *mut libc::c_char,
                                                            );
                                                            if epilogue.is_null() {
                                                                res = MAILIMF_ERROR_MEMORY
                                                                    as libc::c_int;
                                                                current_block = 6612762688763383599;
                                                            } else {
                                                                current_block = 7337917895049117968;
                                                            }
                                                        } else {
                                                            current_block = 7337917895049117968;
                                                        }
                                                        match current_block {
                                                            6612762688763383599 => {}
                                                            _ => {
                                                                cur_token = length;
                                                                *result = list;
                                                                *p_preamble = preamble;
                                                                *p_epilogue = epilogue;
                                                                *indx = cur_token;
                                                                return MAILIMF_NO_ERROR
                                                                    as libc::c_int;
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                        if !epilogue.is_null() {
                            mailmime_data_free(epilogue);
                        }
                        if !preamble.is_null() {
                            mailmime_data_free(preamble);
                        }
                        clist_foreach(
                            list,
                            ::std::mem::transmute::<
                                Option<unsafe fn(_: *mut mailmime) -> ()>,
                                clist_func,
                            >(Some(mailmime_free)),
                            0 as *mut libc::c_void,
                        );
                        clist_free(list);
                    }
                }
            }
        }
        _ => {}
    }
    return res;
}
unsafe fn mailmime_lwsp_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
) -> libc::c_int {
    let mut cur_token: size_t = 0;
    cur_token = *indx;
    if cur_token >= length {
        return MAILIMF_ERROR_PARSE as libc::c_int;
    }
    while 0 != is_wsp(*message.offset(cur_token as isize)) {
        cur_token = cur_token.wrapping_add(1);
        if cur_token >= length {
            break;
        }
    }
    if cur_token == *indx {
        return MAILIMF_ERROR_PARSE as libc::c_int;
    }
    *indx = cur_token;
    return MAILIMF_NO_ERROR as libc::c_int;
}
unsafe fn is_wsp(mut ch: libc::c_char) -> libc::c_int {
    if ch as libc::c_int == ' ' as i32 || ch as libc::c_int == '\t' as i32 {
        return 1i32;
    }
    return 0i32;
}

pub unsafe fn mailmime_multipart_next_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
) -> libc::c_int {
    let mut state: libc::c_int = 0;
    let mut cur_token: size_t = 0;
    cur_token = *indx;
    state = MULTIPART_NEXT_STATE_0 as libc::c_int;
    while state != MULTIPART_NEXT_STATE_2 as libc::c_int {
        if cur_token >= length {
            return MAILIMF_ERROR_PARSE as libc::c_int;
        }
        match state {
            0 => match *message.offset(cur_token as isize) as libc::c_int {
                32 => state = MULTIPART_NEXT_STATE_0 as libc::c_int,
                9 => state = MULTIPART_NEXT_STATE_0 as libc::c_int,
                13 => state = MULTIPART_NEXT_STATE_1 as libc::c_int,
                10 => state = MULTIPART_NEXT_STATE_2 as libc::c_int,
                _ => return MAILIMF_ERROR_PARSE as libc::c_int,
            },
            1 => match *message.offset(cur_token as isize) as libc::c_int {
                10 => state = MULTIPART_NEXT_STATE_2 as libc::c_int,
                _ => return MAILIMF_ERROR_PARSE as libc::c_int,
            },
            _ => {}
        }
        cur_token = cur_token.wrapping_add(1)
    }
    *indx = cur_token;
    return MAILIMF_NO_ERROR as libc::c_int;
}
unsafe fn mailmime_body_part_dash2_close_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut boundary: *mut libc::c_char,
    mut result: *mut *const libc::c_char,
    mut result_size: *mut size_t,
) -> libc::c_int {
    let mut cur_token: size_t = 0;
    let mut r: libc::c_int = 0;
    let mut data_str: *const libc::c_char = 0 as *const libc::c_char;
    let mut data_size: size_t = 0;
    let mut begin_text: *const libc::c_char = 0 as *const libc::c_char;
    let mut end_text: *const libc::c_char = 0 as *const libc::c_char;
    cur_token = *indx;
    begin_text = message.offset(cur_token as isize);
    end_text = message.offset(cur_token as isize);
    loop {
        r = mailmime_body_part_dash2_parse(
            message,
            length,
            &mut cur_token,
            boundary,
            &mut data_str,
            &mut data_size,
        );
        if r == MAILIMF_NO_ERROR as libc::c_int {
            end_text = data_str.offset(data_size as isize)
        } else {
            return r;
        }
        /*
          There's no MIME multipart close bounary.
          Ignore the issue and succeed.
          https://github.com/MailCore/mailcore2/issues/122
        */
        if cur_token >= length {
            break;
        }
        r = mailmime_multipart_close_parse(message, length, &mut cur_token);
        if r == MAILIMF_NO_ERROR as libc::c_int {
            break;
        }
        if r == MAILIMF_ERROR_PARSE as libc::c_int {
        } else {
            return r;
        }
    }
    *indx = cur_token;
    *result = begin_text;
    *result_size = end_text.wrapping_offset_from(begin_text) as size_t;
    return MAILIMF_NO_ERROR as libc::c_int;
}
unsafe fn mailmime_multipart_close_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
) -> libc::c_int {
    let mut state: libc::c_int = 0;
    let mut cur_token: size_t = 0;
    cur_token = *indx;
    state = MULTIPART_CLOSE_STATE_0 as libc::c_int;
    while state != MULTIPART_CLOSE_STATE_4 as libc::c_int {
        match state {
            0 => {
                if cur_token >= length {
                    return MAILIMF_ERROR_PARSE as libc::c_int;
                }
                match *message.offset(cur_token as isize) as libc::c_int {
                    45 => state = MULTIPART_CLOSE_STATE_1 as libc::c_int,
                    _ => return MAILIMF_ERROR_PARSE as libc::c_int,
                }
            }
            1 => {
                if cur_token >= length {
                    return MAILIMF_ERROR_PARSE as libc::c_int;
                }
                match *message.offset(cur_token as isize) as libc::c_int {
                    45 => state = MULTIPART_CLOSE_STATE_2 as libc::c_int,
                    _ => return MAILIMF_ERROR_PARSE as libc::c_int,
                }
            }
            2 => {
                if cur_token >= length {
                    state = MULTIPART_CLOSE_STATE_4 as libc::c_int
                } else {
                    match *message.offset(cur_token as isize) as libc::c_int {
                        32 => state = MULTIPART_CLOSE_STATE_2 as libc::c_int,
                        9 => state = MULTIPART_CLOSE_STATE_2 as libc::c_int,
                        13 => state = MULTIPART_CLOSE_STATE_3 as libc::c_int,
                        10 => state = MULTIPART_CLOSE_STATE_4 as libc::c_int,
                        _ => state = MULTIPART_CLOSE_STATE_4 as libc::c_int,
                    }
                }
            }
            3 => {
                if cur_token >= length {
                    state = MULTIPART_CLOSE_STATE_4 as libc::c_int
                } else {
                    match *message.offset(cur_token as isize) as libc::c_int {
                        10 => state = MULTIPART_CLOSE_STATE_4 as libc::c_int,
                        _ => state = MULTIPART_CLOSE_STATE_4 as libc::c_int,
                    }
                }
            }
            _ => {}
        }
        cur_token = cur_token.wrapping_add(1)
    }
    *indx = cur_token;
    return MAILIMF_NO_ERROR as libc::c_int;
}
unsafe fn mailmime_body_part_dash2_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut boundary: *mut libc::c_char,
    mut result: *mut *const libc::c_char,
    mut result_size: *mut size_t,
) -> libc::c_int {
    let mut state: libc::c_int = 0;
    let mut cur_token: size_t = 0;
    let mut size: size_t = 0;
    let mut begin_text: size_t = 0;
    let mut end_text: size_t = 0;
    let mut r: libc::c_int = 0;
    cur_token = *indx;
    state = BODY_PART_DASH2_STATE_0 as libc::c_int;
    begin_text = cur_token;
    end_text = length;
    while state != BODY_PART_DASH2_STATE_5 as libc::c_int {
        if cur_token >= length {
            break;
        }
        match state {
            0 => match *message.offset(cur_token as isize) as libc::c_int {
                13 => state = BODY_PART_DASH2_STATE_1 as libc::c_int,
                10 => state = BODY_PART_DASH2_STATE_2 as libc::c_int,
                _ => state = BODY_PART_DASH2_STATE_0 as libc::c_int,
            },
            1 => match *message.offset(cur_token as isize) as libc::c_int {
                10 => state = BODY_PART_DASH2_STATE_2 as libc::c_int,
                _ => state = BODY_PART_DASH2_STATE_0 as libc::c_int,
            },
            2 => match *message.offset(cur_token as isize) as libc::c_int {
                45 => {
                    end_text = cur_token;
                    state = BODY_PART_DASH2_STATE_3 as libc::c_int
                }
                13 => state = BODY_PART_DASH2_STATE_1 as libc::c_int,
                10 => state = BODY_PART_DASH2_STATE_2 as libc::c_int,
                _ => state = BODY_PART_DASH2_STATE_0 as libc::c_int,
            },
            3 => match *message.offset(cur_token as isize) as libc::c_int {
                13 => state = BODY_PART_DASH2_STATE_1 as libc::c_int,
                10 => state = BODY_PART_DASH2_STATE_2 as libc::c_int,
                45 => state = BODY_PART_DASH2_STATE_4 as libc::c_int,
                _ => state = BODY_PART_DASH2_STATE_0 as libc::c_int,
            },
            4 => {
                r = mailmime_boundary_parse(message, length, &mut cur_token, boundary);
                if r == MAILIMF_NO_ERROR as libc::c_int {
                    state = BODY_PART_DASH2_STATE_5 as libc::c_int
                } else {
                    state = BODY_PART_DASH2_STATE_6 as libc::c_int
                }
            }
            _ => {}
        }
        if state != BODY_PART_DASH2_STATE_5 as libc::c_int
            && state != BODY_PART_DASH2_STATE_6 as libc::c_int
        {
            cur_token = cur_token.wrapping_add(1)
        }
        if state == BODY_PART_DASH2_STATE_6 as libc::c_int {
            state = BODY_PART_DASH2_STATE_0 as libc::c_int
        }
    }
    size = end_text.wrapping_sub(begin_text);
    if size >= 1i32 as libc::size_t {
        if *message.offset(end_text.wrapping_sub(1i32 as libc::size_t) as isize) as libc::c_int
            == '\r' as i32
        {
            end_text = end_text.wrapping_sub(1);
            size = size.wrapping_sub(1)
        } else if size >= 1i32 as libc::size_t {
            if *message.offset(end_text.wrapping_sub(1i32 as libc::size_t) as isize) as libc::c_int
                == '\n' as i32
            {
                end_text = end_text.wrapping_sub(1);
                size = size.wrapping_sub(1);
                if size >= 1i32 as libc::size_t {
                    if *message.offset(end_text.wrapping_sub(1i32 as libc::size_t) as isize)
                        as libc::c_int
                        == '\r' as i32
                    {
                        end_text = end_text.wrapping_sub(1);
                        size = size.wrapping_sub(1)
                    }
                }
            }
        }
    }
    size = end_text.wrapping_sub(begin_text);
    if size == 0i32 as libc::size_t {
        return MAILIMF_ERROR_PARSE as libc::c_int;
    }
    *result = message.offset(begin_text as isize);
    *result_size = size;
    *indx = cur_token;
    return MAILIMF_NO_ERROR as libc::c_int;
}
unsafe fn mailmime_boundary_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut boundary: *mut libc::c_char,
) -> libc::c_int {
    let mut cur_token: size_t = 0;
    let mut len: size_t = 0;
    cur_token = *indx;
    len = strlen(boundary);
    if cur_token.wrapping_add(len) >= length {
        return MAILIMF_ERROR_PARSE as libc::c_int;
    }
    if strncmp(message.offset(cur_token as isize), boundary, len) != 0i32 {
        return MAILIMF_ERROR_PARSE as libc::c_int;
    }
    cur_token = (cur_token as libc::size_t).wrapping_add(len) as size_t as size_t;
    *indx = cur_token;
    return MAILIMF_NO_ERROR as libc::c_int;
}
unsafe fn mailmime_body_part_dash2_transport_crlf_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut boundary: *mut libc::c_char,
    mut result: *mut *const libc::c_char,
    mut result_size: *mut size_t,
) -> libc::c_int {
    let mut cur_token: size_t = 0;
    let mut r: libc::c_int = 0;
    let mut data_str: *const libc::c_char = 0 as *const libc::c_char;
    let mut data_size: size_t = 0;
    let mut begin_text: *const libc::c_char = 0 as *const libc::c_char;
    let mut end_text: *const libc::c_char = 0 as *const libc::c_char;
    cur_token = *indx;
    begin_text = message.offset(cur_token as isize);
    end_text = message.offset(cur_token as isize);
    loop {
        r = mailmime_body_part_dash2_parse(
            message,
            length,
            &mut cur_token,
            boundary,
            &mut data_str,
            &mut data_size,
        );
        if r == MAILIMF_NO_ERROR as libc::c_int {
            end_text = data_str.offset(data_size as isize)
        } else {
            return r;
        }
        loop {
            r = mailmime_lwsp_parse(message, length, &mut cur_token);
            if r == MAILIMF_NO_ERROR as libc::c_int {
                continue;
            }
            /* do nothing */
            if r == MAILIMF_ERROR_PARSE as libc::c_int {
                break;
            }
            return r;
        }
        r = mailimf_crlf_parse(message, length, &mut cur_token);
        if r == MAILIMF_NO_ERROR as libc::c_int {
            break;
        }
        if r == MAILIMF_ERROR_PARSE as libc::c_int {
        } else {
            return r;
        }
    }
    *indx = cur_token;
    *result = begin_text;
    *result_size = end_text.wrapping_offset_from(begin_text) as size_t;
    return MAILIMF_NO_ERROR as libc::c_int;
}
unsafe fn mailmime_preamble_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut beol: libc::c_int,
) -> libc::c_int {
    let mut state: libc::c_int = 0;
    let mut cur_token: size_t = 0;
    cur_token = *indx;
    if 0 != beol {
        state = PREAMBLE_STATE_A0 as libc::c_int
    } else {
        state = PREAMBLE_STATE_A as libc::c_int
    }
    while state != PREAMBLE_STATE_E as libc::c_int {
        if cur_token >= length {
            return MAILIMF_ERROR_PARSE as libc::c_int;
        }
        match state {
            0 => match *message.offset(cur_token as isize) as libc::c_int {
                45 => state = PREAMBLE_STATE_A1 as libc::c_int,
                13 => state = PREAMBLE_STATE_B as libc::c_int,
                10 => state = PREAMBLE_STATE_C as libc::c_int,
                _ => state = PREAMBLE_STATE_A as libc::c_int,
            },
            1 => match *message.offset(cur_token as isize) as libc::c_int {
                13 => state = PREAMBLE_STATE_B as libc::c_int,
                10 => state = PREAMBLE_STATE_C as libc::c_int,
                _ => state = PREAMBLE_STATE_A as libc::c_int,
            },
            2 => match *message.offset(cur_token as isize) as libc::c_int {
                45 => state = PREAMBLE_STATE_E as libc::c_int,
                13 => state = PREAMBLE_STATE_B as libc::c_int,
                10 => state = PREAMBLE_STATE_C as libc::c_int,
                _ => state = PREAMBLE_STATE_A as libc::c_int,
            },
            3 => match *message.offset(cur_token as isize) as libc::c_int {
                13 => state = PREAMBLE_STATE_B as libc::c_int,
                10 => state = PREAMBLE_STATE_C as libc::c_int,
                45 => state = PREAMBLE_STATE_D as libc::c_int,
                _ => state = PREAMBLE_STATE_A0 as libc::c_int,
            },
            4 => match *message.offset(cur_token as isize) as libc::c_int {
                45 => state = PREAMBLE_STATE_D as libc::c_int,
                13 => state = PREAMBLE_STATE_B as libc::c_int,
                10 => state = PREAMBLE_STATE_C as libc::c_int,
                _ => state = PREAMBLE_STATE_A0 as libc::c_int,
            },
            5 => match *message.offset(cur_token as isize) as libc::c_int {
                45 => state = PREAMBLE_STATE_E as libc::c_int,
                _ => state = PREAMBLE_STATE_A as libc::c_int,
            },
            _ => {}
        }
        cur_token = cur_token.wrapping_add(1)
    }
    *indx = cur_token;
    return MAILIMF_NO_ERROR as libc::c_int;
}
unsafe fn remove_unparsed_mime_headers(mut fields: *mut mailimf_fields) {
    let mut cur: *mut clistiter = 0 as *mut clistiter;
    cur = (*(*fields).fld_list).first;
    while !cur.is_null() {
        let mut field: *mut mailimf_field = 0 as *mut mailimf_field;
        let mut delete: libc::c_int = 0;
        field = (if !cur.is_null() {
            (*cur).data
        } else {
            0 as *mut libc::c_void
        }) as *mut mailimf_field;
        match (*field).fld_type {
            22 => {
                delete = 0i32;
                if strncasecmp(
                    (*(*field).fld_data.fld_optional_field).fld_name,
                    b"Content-\x00" as *const u8 as *const libc::c_char,
                    8i32 as libc::size_t,
                ) == 0i32
                {
                    let mut name: *mut libc::c_char = 0 as *mut libc::c_char;
                    name = (*(*field).fld_data.fld_optional_field)
                        .fld_name
                        .offset(8isize);
                    if strcasecmp(name, b"Type\x00" as *const u8 as *const libc::c_char) == 0i32
                        || strcasecmp(
                            name,
                            b"Transfer-Encoding\x00" as *const u8 as *const libc::c_char,
                        ) == 0i32
                        || strcasecmp(name, b"ID\x00" as *const u8 as *const libc::c_char) == 0i32
                        || strcasecmp(name, b"Description\x00" as *const u8 as *const libc::c_char)
                            == 0i32
                        || strcasecmp(name, b"Disposition\x00" as *const u8 as *const libc::c_char)
                            == 0i32
                        || strcasecmp(name, b"Language\x00" as *const u8 as *const libc::c_char)
                            == 0i32
                    {
                        delete = 1i32
                    }
                } else if strcasecmp(
                    (*(*field).fld_data.fld_optional_field).fld_name,
                    b"MIME-Version\x00" as *const u8 as *const libc::c_char,
                ) == 0i32
                {
                    delete = 1i32
                }
                if 0 != delete {
                    cur = clist_delete((*fields).fld_list, cur);
                    mailimf_field_free(field);
                } else {
                    cur = if !cur.is_null() {
                        (*cur).next
                    } else {
                        0 as *mut clistcell
                    }
                }
            }
            _ => {
                cur = if !cur.is_null() {
                    (*cur).next
                } else {
                    0 as *mut clistcell
                }
            }
        }
    }
}

pub unsafe fn mailmime_extract_boundary(
    mut content_type: *mut mailmime_content,
) -> *mut libc::c_char {
    let mut boundary: *mut libc::c_char = 0 as *mut libc::c_char;
    boundary = mailmime_content_param_get(
        content_type,
        b"boundary\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
    );
    if !boundary.is_null() {
        let mut len: size_t = 0;
        let mut new_boundary: *mut libc::c_char = 0 as *mut libc::c_char;
        len = strlen(boundary);
        new_boundary = malloc(len.wrapping_add(1i32 as libc::size_t)) as *mut libc::c_char;
        if new_boundary.is_null() {
            return 0 as *mut libc::c_char;
        }
        if *boundary.offset(0isize) as libc::c_int == '\"' as i32 {
            strncpy(
                new_boundary,
                boundary.offset(1isize),
                len.wrapping_sub(2i32 as libc::size_t),
            );
            *new_boundary.offset(len.wrapping_sub(2i32 as libc::size_t) as isize) =
                0i32 as libc::c_char
        } else {
            strcpy(new_boundary, boundary);
        }
        boundary = new_boundary
    }
    return boundary;
}

pub unsafe fn mailmime_get_section(
    mut mime: *mut mailmime,
    mut section: *mut mailmime_section,
    mut result: *mut *mut mailmime,
) -> libc::c_int {
    return mailmime_get_section_list(mime, (*(*section).sec_list).first, result);
}
unsafe fn mailmime_get_section_list(
    mut mime: *mut mailmime,
    mut list: *mut clistiter,
    mut result: *mut *mut mailmime,
) -> libc::c_int {
    let mut id: uint32_t = 0;
    let mut data: *mut mailmime = 0 as *mut mailmime;
    let mut submime: *mut mailmime = 0 as *mut mailmime;
    if list.is_null() {
        *result = mime;
        return MAILIMF_NO_ERROR as libc::c_int;
    }
    id = *((if !list.is_null() {
        (*list).data
    } else {
        0 as *mut libc::c_void
    }) as *mut uint32_t);
    data = 0 as *mut mailmime;
    match (*mime).mm_type {
        1 => return MAILIMF_ERROR_INVAL as libc::c_int,
        2 => {
            data = clist_nth_data(
                (*mime).mm_data.mm_multipart.mm_mp_list,
                id.wrapping_sub(1i32 as libc::c_uint) as libc::c_int,
            ) as *mut mailmime;
            if data.is_null() {
                return MAILIMF_ERROR_INVAL as libc::c_int;
            }
            if !if !list.is_null() {
                (*list).next
            } else {
                0 as *mut clistcell
            }
            .is_null()
            {
                return mailmime_get_section_list(
                    data,
                    if !list.is_null() {
                        (*list).next
                    } else {
                        0 as *mut clistcell
                    },
                    result,
                );
            } else {
                *result = data;
                return MAILIMF_NO_ERROR as libc::c_int;
            }
        }
        3 => {
            submime = (*mime).mm_data.mm_message.mm_msg_mime;
            match (*submime).mm_type {
                2 => {
                    data = clist_nth_data(
                        (*submime).mm_data.mm_multipart.mm_mp_list,
                        id.wrapping_sub(1i32 as libc::c_uint) as libc::c_int,
                    ) as *mut mailmime;
                    if data.is_null() {
                        return MAILIMF_ERROR_INVAL as libc::c_int;
                    }
                    return mailmime_get_section_list(
                        data,
                        if !list.is_null() {
                            (*list).next
                        } else {
                            0 as *mut clistcell
                        },
                        result,
                    );
                }
                _ => {
                    if id != 1i32 as libc::c_uint {
                        return MAILIMF_ERROR_INVAL as libc::c_int;
                    }
                    data = submime;
                    if data.is_null() {
                        return MAILIMF_ERROR_INVAL as libc::c_int;
                    }
                    return mailmime_get_section_list(
                        data,
                        if !list.is_null() {
                            (*list).next
                        } else {
                            0 as *mut clistcell
                        },
                        result,
                    );
                }
            }
        }
        _ => return MAILIMF_ERROR_INVAL as libc::c_int,
    };
}
/* decode */
pub unsafe fn mailmime_base64_body_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut libc::c_char,
    mut result_len: *mut size_t,
) -> libc::c_int {
    return mailmime_base64_body_parse_impl(message, length, indx, result, result_len, 0i32);
}
unsafe fn mailmime_base64_body_parse_impl(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut libc::c_char,
    mut result_len: *mut size_t,
    mut partial: libc::c_int,
) -> libc::c_int {
    let mut current_block: u64;
    let mut cur_token: size_t = 0;
    let mut last_full_token_end: size_t = 0;
    let mut chunk: [libc::c_char; 4] = [0; 4];
    let mut chunk_index: libc::c_int = 0;
    let mut out: [libc::c_char; 3] = [0; 3];
    let mut mmapstr: *mut MMAPString = 0 as *mut MMAPString;
    let mut res: libc::c_int = 0;
    let mut r: libc::c_int = 0;
    let mut written: size_t = 0;
    chunk[0usize] = 0i32 as libc::c_char;
    chunk[1usize] = 0i32 as libc::c_char;
    chunk[2usize] = 0i32 as libc::c_char;
    chunk[3usize] = 0i32 as libc::c_char;
    cur_token = *indx;
    last_full_token_end = *indx;
    chunk_index = 0i32;
    written = 0i32 as size_t;
    mmapstr = mmap_string_sized_new(
        length
            .wrapping_sub(cur_token)
            .wrapping_mul(3i32 as libc::size_t)
            .wrapping_div(4i32 as libc::size_t),
    );
    if mmapstr.is_null() {
        res = MAILIMF_ERROR_MEMORY as libc::c_int
    } else {
        loop {
            let mut value: libc::c_schar = 0;
            value = -1i32 as libc::c_schar;
            while value as libc::c_int == -1i32 {
                if cur_token >= length {
                    break;
                }
                value = get_base64_value(*message.offset(cur_token as isize));
                cur_token = cur_token.wrapping_add(1)
            }
            if value as libc::c_int == -1i32 {
                current_block = 8845338526596852646;
                break;
            }
            chunk[chunk_index as usize] = value as libc::c_char;
            chunk_index += 1;
            if !(chunk_index == 4i32) {
                continue;
            }
            out[0usize] = ((chunk[0usize] as libc::c_int) << 2i32
                | chunk[1usize] as libc::c_int >> 4i32) as libc::c_char;
            out[1usize] = ((chunk[1usize] as libc::c_int) << 4i32
                | chunk[2usize] as libc::c_int >> 2i32) as libc::c_char;
            out[2usize] = ((chunk[2usize] as libc::c_int) << 6i32 | chunk[3usize] as libc::c_int)
                as libc::c_char;
            chunk[0usize] = 0i32 as libc::c_char;
            chunk[1usize] = 0i32 as libc::c_char;
            chunk[2usize] = 0i32 as libc::c_char;
            chunk[3usize] = 0i32 as libc::c_char;
            chunk_index = 0i32;
            last_full_token_end = cur_token;
            if mmap_string_append_len(mmapstr, out.as_mut_ptr(), 3i32 as size_t).is_null() {
                res = MAILIMF_ERROR_MEMORY as libc::c_int;
                current_block = 11891829943175634231;
                break;
            } else {
                written =
                    (written as libc::size_t).wrapping_add(3i32 as libc::size_t) as size_t as size_t
            }
        }
        match current_block {
            8845338526596852646 => {
                if chunk_index != 0i32 && 0 == partial {
                    let mut len: size_t = 0;
                    len = 0i32 as size_t;
                    out[0usize] = ((chunk[0usize] as libc::c_int) << 2i32
                        | chunk[1usize] as libc::c_int >> 4i32)
                        as libc::c_char;
                    len = len.wrapping_add(1);
                    if chunk_index >= 3i32 {
                        out[1usize] = ((chunk[1usize] as libc::c_int) << 4i32
                            | chunk[2usize] as libc::c_int >> 2i32)
                            as libc::c_char;
                        len = len.wrapping_add(1)
                    }
                    if mmap_string_append_len(mmapstr, out.as_mut_ptr(), len).is_null() {
                        res = MAILIMF_ERROR_MEMORY as libc::c_int;
                        current_block = 11891829943175634231;
                    } else {
                        written = (written as libc::size_t).wrapping_add(len) as size_t as size_t;
                        current_block = 16738040538446813684;
                    }
                } else {
                    current_block = 16738040538446813684;
                }
                match current_block {
                    11891829943175634231 => {}
                    _ => {
                        if 0 != partial {
                            cur_token = last_full_token_end
                        }
                        r = mmap_string_ref(mmapstr);
                        if r < 0i32 {
                            res = MAILIMF_ERROR_MEMORY as libc::c_int
                        } else {
                            *indx = cur_token;
                            *result = (*mmapstr).str_0;
                            *result_len = written;
                            return MAILIMF_NO_ERROR as libc::c_int;
                        }
                    }
                }
            }
            _ => {}
        }
        mmap_string_free(mmapstr);
    }
    return res;
}
/* ************************************************************************* */
/* MIME part decoding */
unsafe fn get_base64_value(mut ch: libc::c_char) -> libc::c_schar {
    if ch as libc::c_int >= 'A' as i32 && ch as libc::c_int <= 'Z' as i32 {
        return (ch as libc::c_int - 'A' as i32) as libc::c_schar;
    }
    if ch as libc::c_int >= 'a' as i32 && ch as libc::c_int <= 'z' as i32 {
        return (ch as libc::c_int - 'a' as i32 + 26i32) as libc::c_schar;
    }
    if ch as libc::c_int >= '0' as i32 && ch as libc::c_int <= '9' as i32 {
        return (ch as libc::c_int - '0' as i32 + 52i32) as libc::c_schar;
    }
    match ch as libc::c_int {
        43 => return 62i32 as libc::c_schar,
        47 => return 63i32 as libc::c_schar,
        61 => return -1i32 as libc::c_schar,
        _ => return -1i32 as libc::c_schar,
    };
}

pub unsafe fn mailmime_quoted_printable_body_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut libc::c_char,
    mut result_len: *mut size_t,
    mut in_header: libc::c_int,
) -> libc::c_int {
    return mailmime_quoted_printable_body_parse_impl(
        message, length, indx, result, result_len, in_header, 0i32,
    );
}
unsafe fn mailmime_quoted_printable_body_parse_impl(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut libc::c_char,
    mut result_len: *mut size_t,
    mut in_header: libc::c_int,
    mut partial: libc::c_int,
) -> libc::c_int {
    let mut current_block: u64;
    let mut cur_token: size_t = 0;
    let mut state: libc::c_int = 0;
    let mut r: libc::c_int = 0;
    let mut ch: libc::c_char = 0;
    let mut count: size_t = 0;
    let mut start: *const libc::c_char = 0 as *const libc::c_char;
    let mut mmapstr: *mut MMAPString = 0 as *mut MMAPString;
    let mut res: libc::c_int = 0;
    let mut written: size_t = 0;
    state = STATE_NORMAL as libc::c_int;
    cur_token = *indx;
    count = 0i32 as size_t;
    start = message.offset(cur_token as isize);
    written = 0i32 as size_t;
    mmapstr = mmap_string_sized_new(length.wrapping_sub(cur_token));
    if mmapstr.is_null() {
        res = MAILIMF_ERROR_MEMORY as libc::c_int
    } else {
        loop {
            if !(state != STATE_OUT as libc::c_int) {
                current_block = 12693738997172594219;
                break;
            }
            if cur_token >= length {
                state = STATE_OUT as libc::c_int;
                if 0 != partial {
                    cur_token = length
                }
                current_block = 12693738997172594219;
                break;
            } else {
                match state {
                    1 => {
                        if count > 0i32 as libc::size_t {
                            r = write_decoded_qp(mmapstr, start, count);
                            if r != MAILIMF_NO_ERROR as libc::c_int {
                                res = r;
                                current_block = 13807130624542804568;
                                break;
                            } else {
                                written = (written as libc::size_t).wrapping_add(count) as size_t
                                    as size_t;
                                count = 0i32 as size_t
                            }
                        }
                        match *message.offset(cur_token as isize) as libc::c_int {
                            61 => {
                                if cur_token.wrapping_add(1i32 as libc::size_t) >= length {
                                    if 0 != partial {
                                        state = STATE_OUT as libc::c_int
                                    } else {
                                        state = STATE_NORMAL as libc::c_int;
                                        start = message.offset(cur_token as isize);
                                        cur_token = cur_token.wrapping_add(1);
                                        count = count.wrapping_add(1)
                                    }
                                } else {
                                    match *message.offset(
                                        cur_token.wrapping_add(1i32 as libc::size_t) as isize,
                                    ) as libc::c_int
                                    {
                                        10 => {
                                            cur_token = (cur_token as libc::size_t)
                                                .wrapping_add(2i32 as libc::size_t)
                                                as size_t
                                                as size_t;
                                            start = message.offset(cur_token as isize);
                                            state = STATE_NORMAL as libc::c_int
                                        }
                                        13 => {
                                            if cur_token.wrapping_add(2i32 as libc::size_t)
                                                >= length
                                            {
                                                state = STATE_OUT as libc::c_int
                                            } else {
                                                if *message.offset(
                                                    cur_token.wrapping_add(2i32 as libc::size_t)
                                                        as isize,
                                                )
                                                    as libc::c_int
                                                    == '\n' as i32
                                                {
                                                    cur_token = (cur_token as libc::size_t)
                                                        .wrapping_add(3i32 as libc::size_t)
                                                        as size_t
                                                        as size_t
                                                } else {
                                                    cur_token = (cur_token as libc::size_t)
                                                        .wrapping_add(2i32 as libc::size_t)
                                                        as size_t
                                                        as size_t
                                                }
                                                start = message.offset(cur_token as isize);
                                                state = STATE_NORMAL as libc::c_int
                                            }
                                        }
                                        _ => {
                                            if cur_token.wrapping_add(2i32 as libc::size_t)
                                                >= length
                                            {
                                                if 0 != partial {
                                                    state = STATE_OUT as libc::c_int
                                                } else {
                                                    cur_token = cur_token.wrapping_add(1);
                                                    start = message.offset(cur_token as isize);
                                                    count = count.wrapping_add(1);
                                                    state = STATE_NORMAL as libc::c_int
                                                }
                                            } else {
                                                ch = to_char(
                                                    message
                                                        .offset(cur_token as isize)
                                                        .offset(1isize),
                                                );
                                                if mmap_string_append_c(mmapstr, ch).is_null() {
                                                    res = MAILIMF_ERROR_MEMORY as libc::c_int;
                                                    current_block = 13807130624542804568;
                                                    break;
                                                } else {
                                                    cur_token = (cur_token as libc::size_t)
                                                        .wrapping_add(3i32 as libc::size_t)
                                                        as size_t
                                                        as size_t;
                                                    written = written.wrapping_add(1);
                                                    start = message.offset(cur_token as isize);
                                                    state = STATE_NORMAL as libc::c_int
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    0 => {
                        /* end of STATE_ENCODED */
                        match *message.offset(cur_token as isize) as libc::c_int {
                            61 => {
                                state = STATE_CODED as libc::c_int;
                                current_block = 3024367268842933116;
                            }
                            10 => {
                                /* flush before writing additionnal information */
                                if count > 0i32 as libc::size_t {
                                    r = write_decoded_qp(mmapstr, start, count);
                                    if r != MAILIMF_NO_ERROR as libc::c_int {
                                        res = r;
                                        current_block = 13807130624542804568;
                                        break;
                                    } else {
                                        written = (written as libc::size_t).wrapping_add(count)
                                            as size_t
                                            as size_t;
                                        count = 0i32 as size_t
                                    }
                                }
                                r = write_decoded_qp(
                                    mmapstr,
                                    b"\r\n\x00" as *const u8 as *const libc::c_char,
                                    2i32 as size_t,
                                );
                                if r != MAILIMF_NO_ERROR as libc::c_int {
                                    res = r;
                                    current_block = 13807130624542804568;
                                    break;
                                } else {
                                    written = (written as libc::size_t)
                                        .wrapping_add(2i32 as libc::size_t)
                                        as size_t
                                        as size_t;
                                    cur_token = cur_token.wrapping_add(1);
                                    start = message.offset(cur_token as isize)
                                }
                                current_block = 3024367268842933116;
                            }
                            13 => {
                                state = STATE_CR as libc::c_int;
                                cur_token = cur_token.wrapping_add(1);
                                current_block = 3024367268842933116;
                            }
                            95 => {
                                if 0 != in_header {
                                    if count > 0i32 as libc::size_t {
                                        r = write_decoded_qp(mmapstr, start, count);
                                        if r != MAILIMF_NO_ERROR as libc::c_int {
                                            res = r;
                                            current_block = 13807130624542804568;
                                            break;
                                        } else {
                                            written = (written as libc::size_t).wrapping_add(count)
                                                as size_t
                                                as size_t;
                                            count = 0i32 as size_t
                                        }
                                    }
                                    if mmap_string_append_c(mmapstr, ' ' as i32 as libc::c_char)
                                        .is_null()
                                    {
                                        res = MAILIMF_ERROR_MEMORY as libc::c_int;
                                        current_block = 13807130624542804568;
                                        break;
                                    } else {
                                        written = written.wrapping_add(1);
                                        cur_token = cur_token.wrapping_add(1);
                                        start = message.offset(cur_token as isize)
                                    }
                                    current_block = 3024367268842933116;
                                } else {
                                    /* WARINING : must be followed by switch default action */
                                    current_block = 9784205294207992806;
                                }
                            }
                            _ => {
                                current_block = 9784205294207992806;
                            }
                        }
                        match current_block {
                            9784205294207992806 => {
                                if count >= 512i32 as libc::size_t {
                                    r = write_decoded_qp(mmapstr, start, count);
                                    if r != MAILIMF_NO_ERROR as libc::c_int {
                                        res = r;
                                        current_block = 13807130624542804568;
                                        break;
                                    } else {
                                        written = (written as libc::size_t).wrapping_add(count)
                                            as size_t
                                            as size_t;
                                        count = 0i32 as size_t;
                                        start = message.offset(cur_token as isize)
                                    }
                                }
                                count = count.wrapping_add(1);
                                cur_token = cur_token.wrapping_add(1)
                            }
                            _ => {}
                        }
                    }
                    3 => {
                        /* end of STATE_NORMAL */
                        match *message.offset(cur_token as isize) as libc::c_int {
                            10 => {
                                /* flush before writing additionnal information */
                                if count > 0i32 as libc::size_t {
                                    r = write_decoded_qp(mmapstr, start, count);
                                    if r != MAILIMF_NO_ERROR as libc::c_int {
                                        res = r;
                                        current_block = 13807130624542804568;
                                        break;
                                    } else {
                                        written = (written as libc::size_t).wrapping_add(count)
                                            as size_t
                                            as size_t;
                                        count = 0i32 as size_t
                                    }
                                }
                                r = write_decoded_qp(
                                    mmapstr,
                                    b"\r\n\x00" as *const u8 as *const libc::c_char,
                                    2i32 as size_t,
                                );
                                if r != MAILIMF_NO_ERROR as libc::c_int {
                                    res = r;
                                    current_block = 13807130624542804568;
                                    break;
                                } else {
                                    written = (written as libc::size_t)
                                        .wrapping_add(2i32 as libc::size_t)
                                        as size_t
                                        as size_t;
                                    cur_token = cur_token.wrapping_add(1);
                                    start = message.offset(cur_token as isize);
                                    state = STATE_NORMAL as libc::c_int
                                }
                            }
                            _ => {
                                /* flush before writing additionnal information */
                                if count > 0i32 as libc::size_t {
                                    r = write_decoded_qp(mmapstr, start, count);
                                    if r != MAILIMF_NO_ERROR as libc::c_int {
                                        res = r;
                                        current_block = 13807130624542804568;
                                        break;
                                    } else {
                                        written = (written as libc::size_t).wrapping_add(count)
                                            as size_t
                                            as size_t;
                                        count = 0i32 as size_t
                                    }
                                }
                                start = message.offset(cur_token as isize);
                                r = write_decoded_qp(
                                    mmapstr,
                                    b"\r\n\x00" as *const u8 as *const libc::c_char,
                                    2i32 as size_t,
                                );
                                if r != MAILIMF_NO_ERROR as libc::c_int {
                                    res = r;
                                    current_block = 13807130624542804568;
                                    break;
                                } else {
                                    written = (written as libc::size_t)
                                        .wrapping_add(2i32 as libc::size_t)
                                        as size_t
                                        as size_t;
                                    state = STATE_NORMAL as libc::c_int
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        /* end of STATE_CR */
        match current_block {
            12693738997172594219 => {
                if count > 0i32 as libc::size_t {
                    r = write_decoded_qp(mmapstr, start, count);
                    if r != MAILIMF_NO_ERROR as libc::c_int {
                        res = r;
                        current_block = 13807130624542804568;
                    } else {
                        written = (written as libc::size_t).wrapping_add(count) as size_t as size_t;
                        count = 0i32 as size_t;
                        current_block = 9255187738567101705;
                    }
                } else {
                    current_block = 9255187738567101705;
                }
                match current_block {
                    13807130624542804568 => {}
                    _ => {
                        r = mmap_string_ref(mmapstr);
                        if r < 0i32 {
                            res = MAILIMF_ERROR_MEMORY as libc::c_int
                        } else {
                            *indx = cur_token;
                            *result = (*mmapstr).str_0;
                            *result_len = written;
                            return MAILIMF_NO_ERROR as libc::c_int;
                        }
                    }
                }
            }
            _ => {}
        }
        mmap_string_free(mmapstr);
    }
    return res;
}
unsafe fn write_decoded_qp(
    mut mmapstr: *mut MMAPString,
    mut start: *const libc::c_char,
    mut count: size_t,
) -> libc::c_int {
    if mmap_string_append_len(mmapstr, start, count).is_null() {
        return MAILIMF_ERROR_MEMORY as libc::c_int;
    }
    return MAILIMF_NO_ERROR as libc::c_int;
}
#[inline]
unsafe fn to_char(mut hexa: *const libc::c_char) -> libc::c_char {
    return (hexa_to_char(*hexa.offset(0isize)) << 4i32 | hexa_to_char(*hexa.offset(1isize)))
        as libc::c_char;
}
#[inline]
unsafe fn hexa_to_char(mut hexdigit: libc::c_char) -> libc::c_int {
    if hexdigit as libc::c_int >= '0' as i32 && hexdigit as libc::c_int <= '9' as i32 {
        return hexdigit as libc::c_int - '0' as i32;
    }
    if hexdigit as libc::c_int >= 'a' as i32 && hexdigit as libc::c_int <= 'f' as i32 {
        return hexdigit as libc::c_int - 'a' as i32 + 10i32;
    }
    if hexdigit as libc::c_int >= 'A' as i32 && hexdigit as libc::c_int <= 'F' as i32 {
        return hexdigit as libc::c_int - 'A' as i32 + 10i32;
    }
    return 0i32;
}

pub unsafe fn mailmime_binary_body_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut libc::c_char,
    mut result_len: *mut size_t,
) -> libc::c_int {
    let mut mmapstr: *mut MMAPString = 0 as *mut MMAPString;
    let mut cur_token: size_t = 0;
    let mut r: libc::c_int = 0;
    let mut res: libc::c_int = 0;
    cur_token = *indx;
    mmapstr = mmap_string_new_len(
        message.offset(cur_token as isize),
        length.wrapping_sub(cur_token),
    );
    if mmapstr.is_null() {
        res = MAILIMF_ERROR_MEMORY as libc::c_int
    } else {
        r = mmap_string_ref(mmapstr);
        if r < 0i32 {
            res = MAILIMF_ERROR_MEMORY as libc::c_int;
            mmap_string_free(mmapstr);
        } else {
            *indx = length;
            *result = (*mmapstr).str_0;
            *result_len = length.wrapping_sub(cur_token);
            return MAILIMF_NO_ERROR as libc::c_int;
        }
    }
    return res;
}
/*
mailmime_part_parse()

This function gets full MIME part for parsing at once.
It is not suitable, if we want parse incomplete message in a stream mode.

@return the return code is one of MAILIMF_ERROR_XXX or
  MAILIMF_NO_ERROR codes
*/
pub unsafe fn mailmime_part_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut encoding: libc::c_int,
    mut result: *mut *mut libc::c_char,
    mut result_len: *mut size_t,
) -> libc::c_int {
    return mailmime_part_parse_impl(message, length, indx, encoding, result, result_len, 0i32);
}
unsafe fn mailmime_part_parse_impl(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut encoding: libc::c_int,
    mut result: *mut *mut libc::c_char,
    mut result_len: *mut size_t,
    mut partial: libc::c_int,
) -> libc::c_int {
    match encoding {
        5 => {
            return mailmime_base64_body_parse_impl(
                message, length, indx, result, result_len, partial,
            )
        }
        4 => {
            return mailmime_quoted_printable_body_parse_impl(
                message, length, indx, result, result_len, 0i32, partial,
            )
        }
        1 | 2 | 3 | _ => {
            return mailmime_binary_body_parse(message, length, indx, result, result_len)
        }
    };
}
/*
mailmime_part_parse_partial()

This function may parse incomplete MIME part (i.e. in streaming mode).
It stops when detect incomplete encoding unit at the end of data.
Position of the first unparsed byte will be returned in (*indx) value.

For parsing last portion of data must be used mailmime_part_parse() version.

@param message    Message for unparsed data.
@param length     Length of the unparsed data.
@param INOUT indx Index of first unparsed symbol in the message.
@param encoding   Encoding of the input data.
@param result     Parsed MIME part content. Must be freed with mmap_string_unref().
@param result_len Length of parsed data.

@return the return code is one of MAILIMF_ERROR_XXX or
  MAILIMF_NO_ERROR codes

Example Usage:
@code
uint32_t received = 0;
uint32_t partLength = bodystructure[partId]->length;
for (;;) {
  bool isThisRangeLast;
  struct imap_range_t range = { received, 1024*1024 };
  char *result;
  size_t result_len;
  int error = imap_fetch_part_range(uid, partId, range, &result, &result_len);
  if (error != NoError) {
    // handle network error
    break;
  }

  if (result_len == 0) {
    // requested range is empty. part is completely fetched
    break;
  }

  isThisRangeLast = (received + result_len >= partLength); // determine that the received data is the last,
                                                           // may be more difficult (in case of invalid metadata on the server).

  char *decoded;
  size_t decoded_len;
  if (isThisRangeLast) {
    uint32_t index = 0;
    mailmime_part_parse(result, result_len, encoding, &index, &decoded, &decoded_len);
    break;
  }
  else {
    uint32_t index = 0;
    mailmime_part_parse_partial(result, result_len, encoding, &index, &decoded, &decoded_len);
    // we may have some non-decoded bytes at the end of chunk.
    // in this case we just request it in the next chunk
    received += index;
  }
}
@endcode
*/
pub unsafe fn mailmime_part_parse_partial(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut encoding: libc::c_int,
    mut result: *mut *mut libc::c_char,
    mut result_len: *mut size_t,
) -> libc::c_int {
    return mailmime_part_parse_impl(message, length, indx, encoding, result, result_len, 1i32);
}

pub unsafe fn mailmime_get_section_id(
    mut mime: *mut mailmime,
    mut result: *mut *mut mailmime_section,
) -> libc::c_int {
    let mut current_block: u64;
    let mut list: *mut clist = 0 as *mut clist;
    let mut res: libc::c_int = 0;
    let mut section_id: *mut mailmime_section = 0 as *mut mailmime_section;
    let mut r: libc::c_int = 0;
    if (*mime).mm_parent.is_null() {
        list = clist_new();
        if list.is_null() {
            res = MAILIMF_ERROR_MEMORY as libc::c_int;
            current_block = 11086394427076829997;
        } else {
            section_id = mailmime_section_new(list);
            if section_id.is_null() {
                res = MAILIMF_ERROR_MEMORY as libc::c_int;
                current_block = 11086394427076829997;
            } else {
                current_block = 9441801433784995173;
            }
        }
    } else {
        let mut id: uint32_t = 0;
        let mut p_id: *mut uint32_t = 0 as *mut uint32_t;
        let mut cur: *mut clistiter = 0 as *mut clistiter;
        let mut parent: *mut mailmime = 0 as *mut mailmime;
        r = mailmime_get_section_id((*mime).mm_parent, &mut section_id);
        if r != MAILIMF_NO_ERROR as libc::c_int {
            res = r;
            current_block = 11086394427076829997;
        } else {
            parent = (*mime).mm_parent;
            match (*parent).mm_type {
                2 => {
                    current_block = 12048923724182970853;
                    match current_block {
                        14310756207842895454 => {
                            if (*mime).mm_type == MAILMIME_SINGLE as libc::c_int
                                || (*mime).mm_type == MAILMIME_MESSAGE as libc::c_int
                            {
                                p_id = malloc(::std::mem::size_of::<uint32_t>() as libc::size_t)
                                    as *mut uint32_t;
                                if p_id.is_null() {
                                    res = MAILIMF_ERROR_MEMORY as libc::c_int;
                                    current_block = 14847554122685898769;
                                } else {
                                    *p_id = 1i32 as uint32_t;
                                    r = clist_insert_after(
                                        (*section_id).sec_list,
                                        (*(*section_id).sec_list).last,
                                        p_id as *mut libc::c_void,
                                    );
                                    if r < 0i32 {
                                        free(p_id as *mut libc::c_void);
                                        res = MAILIMF_ERROR_MEMORY as libc::c_int;
                                        current_block = 14847554122685898769;
                                    } else {
                                        current_block = 9441801433784995173;
                                    }
                                }
                            } else {
                                current_block = 9441801433784995173;
                            }
                        }
                        _ => {
                            id = 1i32 as uint32_t;
                            cur = (*(*parent).mm_data.mm_multipart.mm_mp_list).first;
                            while !cur.is_null() {
                                if if !cur.is_null() {
                                    (*cur).data
                                } else {
                                    0 as *mut libc::c_void
                                } == mime as *mut libc::c_void
                                {
                                    break;
                                }
                                id = id.wrapping_add(1);
                                cur = if !cur.is_null() {
                                    (*cur).next
                                } else {
                                    0 as *mut clistcell
                                }
                            }
                            p_id = malloc(::std::mem::size_of::<uint32_t>() as libc::size_t)
                                as *mut uint32_t;
                            if p_id.is_null() {
                                res = MAILIMF_ERROR_MEMORY as libc::c_int;
                                current_block = 14847554122685898769;
                            } else {
                                *p_id = id;
                                r = clist_insert_after(
                                    (*section_id).sec_list,
                                    (*(*section_id).sec_list).last,
                                    p_id as *mut libc::c_void,
                                );
                                if r < 0i32 {
                                    free(p_id as *mut libc::c_void);
                                    res = MAILIMF_ERROR_MEMORY as libc::c_int;
                                    current_block = 14847554122685898769;
                                } else {
                                    current_block = 9441801433784995173;
                                }
                            }
                        }
                    }
                    match current_block {
                        9441801433784995173 => {}
                        _ => {
                            mailmime_section_free(section_id);
                            current_block = 11086394427076829997;
                        }
                    }
                }
                3 => {
                    current_block = 14310756207842895454;
                    match current_block {
                        14310756207842895454 => {
                            if (*mime).mm_type == MAILMIME_SINGLE as libc::c_int
                                || (*mime).mm_type == MAILMIME_MESSAGE as libc::c_int
                            {
                                p_id = malloc(::std::mem::size_of::<uint32_t>() as libc::size_t)
                                    as *mut uint32_t;
                                if p_id.is_null() {
                                    res = MAILIMF_ERROR_MEMORY as libc::c_int;
                                    current_block = 14847554122685898769;
                                } else {
                                    *p_id = 1i32 as uint32_t;
                                    r = clist_insert_after(
                                        (*section_id).sec_list,
                                        (*(*section_id).sec_list).last,
                                        p_id as *mut libc::c_void,
                                    );
                                    if r < 0i32 {
                                        free(p_id as *mut libc::c_void);
                                        res = MAILIMF_ERROR_MEMORY as libc::c_int;
                                        current_block = 14847554122685898769;
                                    } else {
                                        current_block = 9441801433784995173;
                                    }
                                }
                            } else {
                                current_block = 9441801433784995173;
                            }
                        }
                        _ => {
                            id = 1i32 as uint32_t;
                            cur = (*(*parent).mm_data.mm_multipart.mm_mp_list).first;
                            while !cur.is_null() {
                                if if !cur.is_null() {
                                    (*cur).data
                                } else {
                                    0 as *mut libc::c_void
                                } == mime as *mut libc::c_void
                                {
                                    break;
                                }
                                id = id.wrapping_add(1);
                                cur = if !cur.is_null() {
                                    (*cur).next
                                } else {
                                    0 as *mut clistcell
                                }
                            }
                            p_id = malloc(::std::mem::size_of::<uint32_t>() as libc::size_t)
                                as *mut uint32_t;
                            if p_id.is_null() {
                                res = MAILIMF_ERROR_MEMORY as libc::c_int;
                                current_block = 14847554122685898769;
                            } else {
                                *p_id = id;
                                r = clist_insert_after(
                                    (*section_id).sec_list,
                                    (*(*section_id).sec_list).last,
                                    p_id as *mut libc::c_void,
                                );
                                if r < 0i32 {
                                    free(p_id as *mut libc::c_void);
                                    res = MAILIMF_ERROR_MEMORY as libc::c_int;
                                    current_block = 14847554122685898769;
                                } else {
                                    current_block = 9441801433784995173;
                                }
                            }
                        }
                    }
                    match current_block {
                        9441801433784995173 => {}
                        _ => {
                            mailmime_section_free(section_id);
                            current_block = 11086394427076829997;
                        }
                    }
                }
                _ => {
                    current_block = 9441801433784995173;
                }
            }
        }
    }
    match current_block {
        11086394427076829997 => return res,
        _ => {
            *result = section_id;
            return MAILIMF_NO_ERROR as libc::c_int;
        }
    };
}
