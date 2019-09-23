use libc;

use libc::toupper;

use crate::clist::*;
use crate::mailimf::*;
use crate::mailimf_types::*;
use crate::mailmime_decode::*;
use crate::mailmime_disposition::*;
use crate::mailmime_types::*;
use crate::other::*;

pub const MAILMIME_COMPOSITE_TYPE_EXTENSION: libc::c_uint = 3;
pub const MAILMIME_COMPOSITE_TYPE_MULTIPART: libc::c_uint = 2;
pub const MAILMIME_COMPOSITE_TYPE_MESSAGE: libc::c_uint = 1;
pub const MAILMIME_COMPOSITE_TYPE_ERROR: libc::c_uint = 0;

pub const MAILMIME_TYPE_COMPOSITE_TYPE: libc::c_uint = 2;
pub const MAILMIME_TYPE_DISCRETE_TYPE: libc::c_uint = 1;
pub const MAILMIME_TYPE_ERROR: libc::c_uint = 0;
pub const FIELD_STATE_L: libc::c_uint = 3;
pub const FIELD_STATE_D: libc::c_uint = 2;
pub const FIELD_STATE_T: libc::c_uint = 1;
pub const FIELD_STATE_START: libc::c_uint = 0;

pub const MAILMIME_DISCRETE_TYPE_EXTENSION: libc::c_uint = 6;
pub const MAILMIME_DISCRETE_TYPE_APPLICATION: libc::c_uint = 5;
pub const MAILMIME_DISCRETE_TYPE_VIDEO: libc::c_uint = 4;
pub const MAILMIME_DISCRETE_TYPE_AUDIO: libc::c_uint = 3;
pub const MAILMIME_DISCRETE_TYPE_IMAGE: libc::c_uint = 2;
pub const MAILMIME_DISCRETE_TYPE_TEXT: libc::c_uint = 1;
pub const MAILMIME_DISCRETE_TYPE_ERROR: libc::c_uint = 0;

pub unsafe fn mailmime_content_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut mailmime_content,
) -> libc::c_int {
    let mut current_block: u64;
    let mut cur_token: size_t = 0;
    let mut type_0: *mut mailmime_type = 0 as *mut mailmime_type;
    let mut subtype: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut parameters_list: *mut clist = 0 as *mut clist;
    let mut content: *mut mailmime_content = 0 as *mut mailmime_content;
    let mut r: libc::c_int = 0;
    let mut res: libc::c_int = 0;
    cur_token = *indx;
    mailimf_cfws_parse(message, length, &mut cur_token);
    type_0 = 0 as *mut mailmime_type;
    r = mailmime_type_parse(message, length, &mut cur_token, &mut type_0);
    if r != MAILIMF_NO_ERROR as libc::c_int {
        res = r
    } else {
        r = mailimf_unstrict_char_parse(
            message,
            length,
            &mut cur_token,
            '/' as i32 as libc::c_char,
        );
        match r {
            0 => {
                r = mailimf_cfws_parse(message, length, &mut cur_token);
                if r != MAILIMF_NO_ERROR as libc::c_int && r != MAILIMF_ERROR_PARSE as libc::c_int {
                    res = r;
                    current_block = 10242373397628622958;
                } else {
                    r = mailmime_subtype_parse(message, length, &mut cur_token, &mut subtype);
                    if r != MAILIMF_NO_ERROR as libc::c_int {
                        res = r;
                        current_block = 10242373397628622958;
                    } else {
                        current_block = 1109700713171191020;
                    }
                }
            }
            1 => {
                subtype = strdup(b"unknown\x00" as *const u8 as *const libc::c_char);
                current_block = 1109700713171191020;
            }
            _ => {
                res = r;
                current_block = 10242373397628622958;
            }
        }
        match current_block {
            1109700713171191020 => {
                parameters_list = clist_new();
                if parameters_list.is_null() {
                    res = MAILIMF_ERROR_MEMORY as libc::c_int
                } else {
                    loop {
                        let mut final_token: size_t = 0;
                        let mut parameter: *mut mailmime_parameter = 0 as *mut mailmime_parameter;
                        final_token = cur_token;
                        r = mailimf_unstrict_char_parse(
                            message,
                            length,
                            &mut cur_token,
                            ';' as i32 as libc::c_char,
                        );
                        if r != MAILIMF_NO_ERROR as libc::c_int {
                            cur_token = final_token;
                            current_block = 12497913735442871383;
                            break;
                        } else {
                            r = mailimf_cfws_parse(message, length, &mut cur_token);
                            if r != MAILIMF_NO_ERROR as libc::c_int
                                && r != MAILIMF_ERROR_PARSE as libc::c_int
                            {
                                res = r;
                                current_block = 6276274620003476740;
                                break;
                            } else {
                                r = mailmime_parameter_parse(
                                    message,
                                    length,
                                    &mut cur_token,
                                    &mut parameter,
                                );
                                if r == MAILIMF_NO_ERROR as libc::c_int {
                                    r = clist_insert_after(
                                        parameters_list,
                                        (*parameters_list).last,
                                        parameter as *mut libc::c_void,
                                    );
                                    if !(r < 0i32) {
                                        continue;
                                    }
                                    mailmime_parameter_free(parameter);
                                    res = MAILIMF_ERROR_MEMORY as libc::c_int;
                                    current_block = 5731074241326334034;
                                    break;
                                } else if r == MAILIMF_ERROR_PARSE as libc::c_int {
                                    cur_token = final_token;
                                    current_block = 12497913735442871383;
                                    break;
                                } else {
                                    res = r;
                                    current_block = 6276274620003476740;
                                    break;
                                }
                            }
                        }
                    }
                    match current_block {
                        6276274620003476740 => {}
                        _ => {
                            match current_block {
                                12497913735442871383 => {
                                    content =
                                        mailmime_content_new(type_0, subtype, parameters_list);
                                    if content.is_null() {
                                        res = MAILIMF_ERROR_MEMORY as libc::c_int
                                    } else {
                                        *result = content;
                                        *indx = cur_token;
                                        return MAILIMF_NO_ERROR as libc::c_int;
                                    }
                                }
                                _ => {}
                            }
                            clist_foreach(
                                parameters_list,
                                ::std::mem::transmute::<
                                    Option<unsafe fn(_: *mut mailmime_parameter) -> ()>,
                                    clist_func,
                                >(Some(mailmime_parameter_free)),
                                0 as *mut libc::c_void,
                            );
                            clist_free(parameters_list);
                        }
                    }
                }
                mailmime_subtype_free(subtype);
            }
            _ => {}
        }
        mailmime_type_free(type_0);
    }
    return res;
}

pub unsafe fn mailmime_parameter_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut mailmime_parameter,
) -> libc::c_int {
    let mut attribute: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut value: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut parameter: *mut mailmime_parameter = 0 as *mut mailmime_parameter;
    let mut cur_token: size_t = 0;
    let mut r: libc::c_int = 0;
    let mut res: libc::c_int = 0;
    cur_token = *indx;
    r = mailmime_attribute_parse(message, length, &mut cur_token, &mut attribute);
    if r != MAILIMF_NO_ERROR as libc::c_int {
        res = r
    } else {
        r = mailimf_unstrict_char_parse(
            message,
            length,
            &mut cur_token,
            '=' as i32 as libc::c_char,
        );
        if r != MAILIMF_NO_ERROR as libc::c_int {
            res = r
        } else {
            r = mailimf_cfws_parse(message, length, &mut cur_token);
            if r != MAILIMF_NO_ERROR as libc::c_int && r != MAILIMF_ERROR_PARSE as libc::c_int {
                res = r
            } else {
                r = mailmime_value_parse(message, length, &mut cur_token, &mut value);
                if r != MAILIMF_NO_ERROR as libc::c_int {
                    res = r
                } else {
                    parameter = mailmime_parameter_new(attribute, value);
                    if parameter.is_null() {
                        res = MAILIMF_ERROR_MEMORY as libc::c_int;
                        mailmime_value_free(value);
                    } else {
                        *result = parameter;
                        *indx = cur_token;
                        return MAILIMF_NO_ERROR as libc::c_int;
                    }
                }
            }
        }
        mailmime_attribute_free(attribute);
    }
    return res;
}

pub unsafe fn mailmime_value_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut libc::c_char,
) -> libc::c_int {
    let mut r: libc::c_int = 0;
    r = mailimf_atom_parse(message, length, indx, result);
    if r == MAILIMF_ERROR_PARSE as libc::c_int {
        r = mailimf_quoted_string_parse(message, length, indx, result)
    }
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    return MAILIMF_NO_ERROR as libc::c_int;
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
 * $Id: mailmime.c,v 1.29 2011/01/06 00:09:52 hoa Exp $
 */
/*
 RFC 2045
 RFC 2046
 RFC 2047
 RFC 2048
 RFC 2049
 RFC 2231
 RFC 2387
 RFC 2424
 RFC 2557

 RFC 2183 Content-Disposition

 RFC 1766  Language
*/
unsafe fn mailmime_attribute_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut libc::c_char,
) -> libc::c_int {
    return mailmime_token_parse(message, length, indx, result);
}
unsafe fn mailmime_token_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut token: *mut *mut libc::c_char,
) -> libc::c_int {
    return mailimf_custom_string_parse(message, length, indx, token, Some(is_token));
}
unsafe fn is_token(mut ch: libc::c_char) -> libc::c_int {
    let mut uch: libc::c_uchar = ch as libc::c_uchar;
    if uch as libc::c_int > 0x7fi32 {
        return 0i32;
    }
    if uch as libc::c_int == ' ' as i32 {
        return 0i32;
    }
    if 0 != is_tspecials(ch) {
        return 0i32;
    }
    return 1i32;
}
unsafe fn is_tspecials(mut ch: libc::c_char) -> libc::c_int {
    match ch as libc::c_int {
        40 | 41 | 60 | 62 | 64 | 44 | 59 | 58 | 92 | 34 | 47 | 91 | 93 | 63 | 61 => return 1i32,
        _ => return 0i32,
    };
}
unsafe fn mailmime_subtype_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut libc::c_char,
) -> libc::c_int {
    return mailmime_extension_token_parse(message, length, indx, result);
}

pub unsafe fn mailmime_extension_token_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut libc::c_char,
) -> libc::c_int {
    return mailmime_token_parse(message, length, indx, result);
}
unsafe fn mailmime_type_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut mailmime_type,
) -> libc::c_int {
    let mut discrete_type: *mut mailmime_discrete_type = 0 as *mut mailmime_discrete_type;
    let mut composite_type: *mut mailmime_composite_type = 0 as *mut mailmime_composite_type;
    let mut cur_token: size_t = 0;
    let mut mime_type: *mut mailmime_type = 0 as *mut mailmime_type;
    let mut type_0: libc::c_int = 0;
    let mut res: libc::c_int = 0;
    let mut r: libc::c_int = 0;
    cur_token = *indx;
    discrete_type = 0 as *mut mailmime_discrete_type;
    composite_type = 0 as *mut mailmime_composite_type;
    type_0 = MAILMIME_TYPE_ERROR as libc::c_int;
    r = mailmime_composite_type_parse(message, length, &mut cur_token, &mut composite_type);
    if r == MAILIMF_NO_ERROR as libc::c_int {
        type_0 = MAILMIME_TYPE_COMPOSITE_TYPE as libc::c_int
    }
    if r == MAILIMF_ERROR_PARSE as libc::c_int {
        r = mailmime_discrete_type_parse(message, length, &mut cur_token, &mut discrete_type);
        if r == MAILIMF_NO_ERROR as libc::c_int {
            type_0 = MAILMIME_TYPE_DISCRETE_TYPE as libc::c_int
        }
    }
    if r != MAILIMF_NO_ERROR as libc::c_int {
        res = r
    } else {
        mime_type = mailmime_type_new(type_0, discrete_type, composite_type);
        if mime_type.is_null() {
            res = r;
            if !discrete_type.is_null() {
                mailmime_discrete_type_free(discrete_type);
            }
            if !composite_type.is_null() {
                mailmime_composite_type_free(composite_type);
            }
        } else {
            *result = mime_type;
            *indx = cur_token;
            return MAILIMF_NO_ERROR as libc::c_int;
        }
    }
    return res;
}
unsafe fn mailmime_discrete_type_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut mailmime_discrete_type,
) -> libc::c_int {
    let mut extension: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut type_0: libc::c_int = 0;
    let mut discrete_type: *mut mailmime_discrete_type = 0 as *mut mailmime_discrete_type;
    let mut cur_token: size_t = 0;
    let mut r: libc::c_int = 0;
    let mut res: libc::c_int = 0;
    cur_token = *indx;
    extension = 0 as *mut libc::c_char;
    type_0 = MAILMIME_DISCRETE_TYPE_ERROR as libc::c_int;
    r = mailimf_token_case_insensitive_len_parse(
        message,
        length,
        &mut cur_token,
        b"text\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
        strlen(b"text\x00" as *const u8 as *const libc::c_char),
    );
    if r == MAILIMF_NO_ERROR as libc::c_int {
        type_0 = MAILMIME_DISCRETE_TYPE_TEXT as libc::c_int
    }
    if r == MAILIMF_ERROR_PARSE as libc::c_int {
        r = mailimf_token_case_insensitive_len_parse(
            message,
            length,
            &mut cur_token,
            b"image\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
            strlen(b"image\x00" as *const u8 as *const libc::c_char),
        );
        if r == MAILIMF_NO_ERROR as libc::c_int {
            type_0 = MAILMIME_DISCRETE_TYPE_IMAGE as libc::c_int
        }
    }
    if r == MAILIMF_ERROR_PARSE as libc::c_int {
        r = mailimf_token_case_insensitive_len_parse(
            message,
            length,
            &mut cur_token,
            b"audio\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
            strlen(b"audio\x00" as *const u8 as *const libc::c_char),
        );
        if r == MAILIMF_NO_ERROR as libc::c_int {
            type_0 = MAILMIME_DISCRETE_TYPE_AUDIO as libc::c_int
        }
    }
    if r == MAILIMF_ERROR_PARSE as libc::c_int {
        r = mailimf_token_case_insensitive_len_parse(
            message,
            length,
            &mut cur_token,
            b"video\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
            strlen(b"video\x00" as *const u8 as *const libc::c_char),
        );
        if r == MAILIMF_NO_ERROR as libc::c_int {
            type_0 = MAILMIME_DISCRETE_TYPE_VIDEO as libc::c_int
        }
    }
    if r == MAILIMF_ERROR_PARSE as libc::c_int {
        r = mailimf_token_case_insensitive_len_parse(
            message,
            length,
            &mut cur_token,
            b"application\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
            strlen(b"application\x00" as *const u8 as *const libc::c_char),
        );
        if r == MAILIMF_NO_ERROR as libc::c_int {
            type_0 = MAILMIME_DISCRETE_TYPE_APPLICATION as libc::c_int
        }
    }
    if r == MAILIMF_ERROR_PARSE as libc::c_int {
        r = mailmime_extension_token_parse(message, length, &mut cur_token, &mut extension);
        if r == MAILIMF_NO_ERROR as libc::c_int {
            type_0 = MAILMIME_DISCRETE_TYPE_EXTENSION as libc::c_int
        }
    }
    if r != MAILIMF_NO_ERROR as libc::c_int {
        res = r
    } else {
        discrete_type = mailmime_discrete_type_new(type_0, extension);
        if discrete_type.is_null() {
            res = MAILIMF_ERROR_MEMORY as libc::c_int;
            mailmime_extension_token_free(extension);
        } else {
            *result = discrete_type;
            *indx = cur_token;
            return MAILIMF_NO_ERROR as libc::c_int;
        }
    }
    return res;
}
unsafe fn mailmime_composite_type_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut mailmime_composite_type,
) -> libc::c_int {
    let mut extension_token: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut type_0: libc::c_int = 0;
    let mut ct: *mut mailmime_composite_type = 0 as *mut mailmime_composite_type;
    let mut cur_token: size_t = 0;
    let mut r: libc::c_int = 0;
    let mut res: libc::c_int = 0;
    cur_token = *indx;
    extension_token = 0 as *mut libc::c_char;
    type_0 = MAILMIME_COMPOSITE_TYPE_ERROR as libc::c_int;
    r = mailimf_token_case_insensitive_len_parse(
        message,
        length,
        &mut cur_token,
        b"message\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
        strlen(b"message\x00" as *const u8 as *const libc::c_char),
    );
    if r == MAILIMF_NO_ERROR as libc::c_int {
        type_0 = MAILMIME_COMPOSITE_TYPE_MESSAGE as libc::c_int
    }
    if r == MAILIMF_ERROR_PARSE as libc::c_int {
        r = mailimf_token_case_insensitive_len_parse(
            message,
            length,
            &mut cur_token,
            b"multipart\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
            strlen(b"multipart\x00" as *const u8 as *const libc::c_char),
        );
        if r == MAILIMF_NO_ERROR as libc::c_int {
            type_0 = MAILMIME_COMPOSITE_TYPE_MULTIPART as libc::c_int
        }
    }
    if r != MAILIMF_NO_ERROR as libc::c_int {
        res = r
    } else {
        ct = mailmime_composite_type_new(type_0, extension_token);
        if ct.is_null() {
            res = MAILIMF_ERROR_MEMORY as libc::c_int;
            if !extension_token.is_null() {
                mailmime_extension_token_free(extension_token);
            }
        } else {
            *result = ct;
            *indx = cur_token;
            return MAILIMF_NO_ERROR as libc::c_int;
        }
    }
    return res;
}

pub unsafe fn mailmime_description_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut libc::c_char,
) -> libc::c_int {
    return mailimf_custom_string_parse(message, length, indx, result, Some(is_text));
}
unsafe fn is_text(mut ch: libc::c_char) -> libc::c_int {
    let mut uch: libc::c_uchar = ch as libc::c_uchar;
    if (uch as libc::c_int) < 1i32 {
        return 0i32;
    }
    if uch as libc::c_int == 10i32 || uch as libc::c_int == 13i32 {
        return 0i32;
    }
    return 1i32;
}

pub unsafe fn mailmime_location_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut libc::c_char,
) -> libc::c_int {
    return mailimf_custom_string_parse(message, length, indx, result, Some(is_text));
}

pub unsafe fn mailmime_encoding_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut mailmime_mechanism,
) -> libc::c_int {
    return mailmime_mechanism_parse(message, length, indx, result);
}
unsafe fn mailmime_mechanism_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut mailmime_mechanism,
) -> libc::c_int {
    let mut token: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut type_0: libc::c_int = 0;
    let mut mechanism: *mut mailmime_mechanism = 0 as *mut mailmime_mechanism;
    let mut cur_token: size_t = 0;
    let mut r: libc::c_int = 0;
    let mut res: libc::c_int = 0;
    cur_token = *indx;
    type_0 = MAILMIME_MECHANISM_ERROR as libc::c_int;
    token = 0 as *mut libc::c_char;
    r = mailimf_token_case_insensitive_len_parse(
        message,
        length,
        &mut cur_token,
        b"7bit\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
        strlen(b"7bit\x00" as *const u8 as *const libc::c_char),
    );
    if r == MAILIMF_NO_ERROR as libc::c_int {
        type_0 = MAILMIME_MECHANISM_7BIT as libc::c_int
    }
    if r == MAILIMF_ERROR_PARSE as libc::c_int {
        r = mailimf_token_case_insensitive_len_parse(
            message,
            length,
            &mut cur_token,
            b"8bit\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
            strlen(b"8bit\x00" as *const u8 as *const libc::c_char),
        );
        if r == MAILIMF_NO_ERROR as libc::c_int {
            type_0 = MAILMIME_MECHANISM_8BIT as libc::c_int
        }
    }
    if r == MAILIMF_ERROR_PARSE as libc::c_int {
        r = mailimf_token_case_insensitive_len_parse(
            message,
            length,
            &mut cur_token,
            b"binary\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
            strlen(b"binary\x00" as *const u8 as *const libc::c_char),
        );
        if r == MAILIMF_NO_ERROR as libc::c_int {
            type_0 = MAILMIME_MECHANISM_BINARY as libc::c_int
        }
    }
    if r == MAILIMF_ERROR_PARSE as libc::c_int {
        r = mailimf_token_case_insensitive_len_parse(
            message,
            length,
            &mut cur_token,
            b"quoted-printable\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
            strlen(b"quoted-printable\x00" as *const u8 as *const libc::c_char),
        );
        if r == MAILIMF_NO_ERROR as libc::c_int {
            type_0 = MAILMIME_MECHANISM_QUOTED_PRINTABLE as libc::c_int
        }
    }
    if r == MAILIMF_ERROR_PARSE as libc::c_int {
        r = mailimf_token_case_insensitive_len_parse(
            message,
            length,
            &mut cur_token,
            b"base64\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
            strlen(b"base64\x00" as *const u8 as *const libc::c_char),
        );
        if r == MAILIMF_NO_ERROR as libc::c_int {
            type_0 = MAILMIME_MECHANISM_BASE64 as libc::c_int
        }
    }
    if r == MAILIMF_ERROR_PARSE as libc::c_int {
        r = mailmime_token_parse(message, length, &mut cur_token, &mut token);
        if r == MAILIMF_NO_ERROR as libc::c_int {
            type_0 = MAILMIME_MECHANISM_TOKEN as libc::c_int
        }
    }
    if r != MAILIMF_NO_ERROR as libc::c_int {
        res = r
    } else {
        mechanism = mailmime_mechanism_new(type_0, token);
        if mechanism.is_null() {
            res = MAILIMF_ERROR_MEMORY as libc::c_int;
            if !token.is_null() {
                mailmime_token_free(token);
            }
        } else {
            *result = mechanism;
            *indx = cur_token;
            return MAILIMF_NO_ERROR as libc::c_int;
        }
    }
    return res;
}

pub unsafe fn mailmime_field_parse(
    mut field: *mut mailimf_optional_field,
    mut result: *mut *mut mailmime_field,
) -> libc::c_int {
    let mut name: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut value: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut guessed_type: libc::c_int = 0;
    let mut cur_token: size_t = 0;
    let mut content: *mut mailmime_content = 0 as *mut mailmime_content;
    let mut encoding: *mut mailmime_mechanism = 0 as *mut mailmime_mechanism;
    let mut id: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut description: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut version: uint32_t = 0;
    let mut mime_field: *mut mailmime_field = 0 as *mut mailmime_field;
    let mut language: *mut mailmime_language = 0 as *mut mailmime_language;
    let mut disposition: *mut mailmime_disposition = 0 as *mut mailmime_disposition;
    let mut location: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut res: libc::c_int = 0;
    let mut r: libc::c_int = 0;
    name = (*field).fld_name;
    value = (*field).fld_value;
    cur_token = 0i32 as size_t;
    content = 0 as *mut mailmime_content;
    encoding = 0 as *mut mailmime_mechanism;
    id = 0 as *mut libc::c_char;
    description = 0 as *mut libc::c_char;
    version = 0i32 as uint32_t;
    disposition = 0 as *mut mailmime_disposition;
    language = 0 as *mut mailmime_language;
    location = 0 as *mut libc::c_char;
    guessed_type = guess_field_type(name);
    match guessed_type {
        1 => {
            if strcasecmp(
                name,
                b"Content-Type\x00" as *const u8 as *const libc::c_char,
            ) != 0i32
            {
                return MAILIMF_ERROR_PARSE as libc::c_int;
            }
            let mut cur_token_0: size_t = 0i32 as size_t;
            let mut decoded_value: *mut libc::c_char = 0 as *mut libc::c_char;
            r = mailmime_encoded_phrase_parse(
                b"us-ascii\x00" as *const u8 as *const libc::c_char,
                value,
                strlen(value),
                &mut cur_token_0,
                b"utf-8\x00" as *const u8 as *const libc::c_char,
                &mut decoded_value,
            );
            if r != MAILIMF_NO_ERROR as libc::c_int {
                cur_token_0 = 0i32 as size_t;
                r = mailmime_content_parse(value, strlen(value), &mut cur_token_0, &mut content)
            } else {
                cur_token_0 = 0i32 as size_t;
                r = mailmime_content_parse(
                    decoded_value,
                    strlen(decoded_value),
                    &mut cur_token_0,
                    &mut content,
                );
                free(decoded_value as *mut libc::c_void);
            }
            if r != MAILIMF_NO_ERROR as libc::c_int {
                return r;
            }
        }
        2 => {
            if strcasecmp(
                name,
                b"Content-Transfer-Encoding\x00" as *const u8 as *const libc::c_char,
            ) != 0i32
            {
                return MAILIMF_ERROR_PARSE as libc::c_int;
            }
            r = mailmime_encoding_parse(value, strlen(value), &mut cur_token, &mut encoding);
            if r != MAILIMF_NO_ERROR as libc::c_int {
                return r;
            }
        }
        3 => {
            if strcasecmp(name, b"Content-ID\x00" as *const u8 as *const libc::c_char) != 0i32 {
                return MAILIMF_ERROR_PARSE as libc::c_int;
            }
            r = mailmime_id_parse(value, strlen(value), &mut cur_token, &mut id);
            if r != MAILIMF_NO_ERROR as libc::c_int {
                return r;
            }
        }
        4 => {
            if strcasecmp(
                name,
                b"Content-Description\x00" as *const u8 as *const libc::c_char,
            ) != 0i32
            {
                return MAILIMF_ERROR_PARSE as libc::c_int;
            }
            r = mailmime_description_parse(value, strlen(value), &mut cur_token, &mut description);
            if r != MAILIMF_NO_ERROR as libc::c_int {
                return r;
            }
        }
        5 => {
            if strcasecmp(
                name,
                b"MIME-Version\x00" as *const u8 as *const libc::c_char,
            ) != 0i32
            {
                return MAILIMF_ERROR_PARSE as libc::c_int;
            }
            r = mailmime_version_parse(value, strlen(value), &mut cur_token, &mut version);
            if r != MAILIMF_NO_ERROR as libc::c_int {
                return r;
            }
        }
        6 => {
            if strcasecmp(
                name,
                b"Content-Disposition\x00" as *const u8 as *const libc::c_char,
            ) != 0i32
            {
                return MAILIMF_ERROR_PARSE as libc::c_int;
            }
            r = mailmime_disposition_parse(value, strlen(value), &mut cur_token, &mut disposition);
            if r != MAILIMF_NO_ERROR as libc::c_int {
                return r;
            }
        }
        7 => {
            if strcasecmp(
                name,
                b"Content-Language\x00" as *const u8 as *const libc::c_char,
            ) != 0i32
            {
                return MAILIMF_ERROR_PARSE as libc::c_int;
            }
            r = mailmime_language_parse(value, strlen(value), &mut cur_token, &mut language);
            if r != MAILIMF_NO_ERROR as libc::c_int {
                return r;
            }
        }
        8 => {
            if strcasecmp(
                name,
                b"Content-Location\x00" as *const u8 as *const libc::c_char,
            ) != 0i32
            {
                return MAILIMF_ERROR_PARSE as libc::c_int;
            }
            r = mailmime_location_parse(value, strlen(value), &mut cur_token, &mut location);
            if r != MAILIMF_NO_ERROR as libc::c_int {
                return r;
            }
        }
        _ => return MAILIMF_ERROR_PARSE as libc::c_int,
    }
    mime_field = mailmime_field_new(
        guessed_type,
        content,
        encoding,
        id,
        description,
        version,
        disposition,
        language,
        location,
    );
    if mime_field.is_null() {
        res = MAILIMF_ERROR_MEMORY as libc::c_int;
        if !location.is_null() {
            mailmime_location_free(location);
        }
        if !language.is_null() {
            mailmime_language_free(language);
        }
        if !content.is_null() {
            mailmime_content_free(content);
        }
        if !encoding.is_null() {
            mailmime_encoding_free(encoding);
        }
        if !id.is_null() {
            mailmime_id_free(id);
        }
        if !description.is_null() {
            mailmime_description_free(description);
        }
        return res;
    } else {
        *result = mime_field;
        return MAILIMF_NO_ERROR as libc::c_int;
    };
}

pub unsafe fn mailmime_language_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut mailmime_language,
) -> libc::c_int {
    let mut current_block: u64;
    let mut cur_token: size_t = 0;
    let mut r: libc::c_int = 0;
    let mut res: libc::c_int = 0;
    let mut list: *mut clist = 0 as *mut clist;
    let mut language: *mut mailmime_language = 0 as *mut mailmime_language;
    cur_token = *indx;
    list = clist_new();
    if list.is_null() {
        res = MAILIMF_ERROR_MEMORY as libc::c_int
    } else {
        loop {
            let mut atom: *mut libc::c_char = 0 as *mut libc::c_char;
            r = mailimf_unstrict_char_parse(
                message,
                length,
                &mut cur_token,
                ',' as i32 as libc::c_char,
            );
            if r == MAILIMF_NO_ERROR as libc::c_int {
                r = mailimf_atom_parse(message, length, &mut cur_token, &mut atom);
                if r == MAILIMF_NO_ERROR as libc::c_int {
                    r = clist_insert_after(list, (*list).last, atom as *mut libc::c_void);
                    if !(r < 0i32) {
                        continue;
                    }
                    mailimf_atom_free(atom);
                    res = MAILIMF_ERROR_MEMORY as libc::c_int;
                    current_block = 14533943604180559553;
                    break;
                } else {
                    /* do nothing */
                    if r == MAILIMF_ERROR_PARSE as libc::c_int {
                        current_block = 6669252993407410313;
                        break;
                    }
                    res = r;
                    current_block = 11601180562230609130;
                    break;
                }
            } else {
                /* do nothing */
                if r == MAILIMF_ERROR_PARSE as libc::c_int {
                    current_block = 6669252993407410313;
                    break;
                }
                res = r;
                current_block = 11601180562230609130;
                break;
            }
        }
        match current_block {
            11601180562230609130 => {}
            _ => {
                match current_block {
                    6669252993407410313 => {
                        language = mailmime_language_new(list);
                        if language.is_null() {
                            res = MAILIMF_ERROR_MEMORY as libc::c_int
                        } else {
                            *result = language;
                            *indx = cur_token;
                            return MAILIMF_NO_ERROR as libc::c_int;
                        }
                    }
                    _ => {}
                }
                clist_foreach(
                    list,
                    ::std::mem::transmute::<
                        Option<unsafe fn(_: *mut libc::c_char) -> ()>,
                        clist_func,
                    >(Some(mailimf_atom_free)),
                    0 as *mut libc::c_void,
                );
                clist_free(list);
            }
        }
    }
    return res;
}

pub unsafe fn mailmime_version_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut uint32_t,
) -> libc::c_int {
    let mut cur_token: size_t = 0;
    let mut hi: uint32_t = 0;
    let mut low: uint32_t = 0;
    let mut version: uint32_t = 0;
    let mut r: libc::c_int = 0;
    cur_token = *indx;
    r = mailimf_number_parse(message, length, &mut cur_token, &mut hi);
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    r = mailimf_unstrict_char_parse(message, length, &mut cur_token, '.' as i32 as libc::c_char);
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    r = mailimf_cfws_parse(message, length, &mut cur_token);
    if r != MAILIMF_NO_ERROR as libc::c_int && r != MAILIMF_ERROR_PARSE as libc::c_int {
        return r;
    }
    r = mailimf_number_parse(message, length, &mut cur_token, &mut low);
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    version = (hi << 16i32).wrapping_add(low);
    *result = version;
    *indx = cur_token;
    return MAILIMF_NO_ERROR as libc::c_int;
}

pub unsafe fn mailmime_id_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut libc::c_char,
) -> libc::c_int {
    return mailimf_msg_id_parse(message, length, indx, result);
}
unsafe fn guess_field_type(mut name: *mut libc::c_char) -> libc::c_int {
    let mut state: libc::c_int = 0;
    if *name as libc::c_int == 'M' as i32 {
        return MAILMIME_FIELD_VERSION as libc::c_int;
    }
    if strncasecmp(
        name,
        b"Content-\x00" as *const u8 as *const libc::c_char,
        8i32 as libc::size_t,
    ) != 0i32
    {
        return MAILMIME_FIELD_NONE as libc::c_int;
    }
    name = name.offset(8isize);
    state = FIELD_STATE_START as libc::c_int;
    loop {
        match state {
            0 => {
                match toupper(*name as libc::c_uchar as libc::c_int) as libc::c_char as libc::c_int
                {
                    84 => state = FIELD_STATE_T as libc::c_int,
                    73 => return MAILMIME_FIELD_ID as libc::c_int,
                    68 => state = FIELD_STATE_D as libc::c_int,
                    76 => state = FIELD_STATE_L as libc::c_int,
                    _ => return MAILMIME_FIELD_NONE as libc::c_int,
                }
            }
            1 => {
                match toupper(*name as libc::c_uchar as libc::c_int) as libc::c_char as libc::c_int
                {
                    89 => return MAILMIME_FIELD_TYPE as libc::c_int,
                    82 => return MAILMIME_FIELD_TRANSFER_ENCODING as libc::c_int,
                    _ => return MAILMIME_FIELD_NONE as libc::c_int,
                }
            }
            2 => {
                match toupper(*name as libc::c_uchar as libc::c_int) as libc::c_char as libc::c_int
                {
                    69 => return MAILMIME_FIELD_DESCRIPTION as libc::c_int,
                    73 => return MAILMIME_FIELD_DISPOSITION as libc::c_int,
                    _ => return MAILMIME_FIELD_NONE as libc::c_int,
                }
            }
            3 => {
                match toupper(*name as libc::c_uchar as libc::c_int) as libc::c_char as libc::c_int
                {
                    65 => return MAILMIME_FIELD_LANGUAGE as libc::c_int,
                    79 => return MAILMIME_FIELD_LOCATION as libc::c_int,
                    _ => return MAILMIME_FIELD_NONE as libc::c_int,
                }
            }
            _ => {}
        }
        name = name.offset(1isize)
    }
}

pub unsafe fn mailmime_fields_parse(
    mut fields: *mut mailimf_fields,
    mut result: *mut *mut mailmime_fields,
) -> libc::c_int {
    let mut current_block: u64;
    let mut cur: *mut clistiter = 0 as *mut clistiter;
    let mut mime_fields: *mut mailmime_fields = 0 as *mut mailmime_fields;
    let mut list: *mut clist = 0 as *mut clist;
    let mut r: libc::c_int = 0;
    let mut res: libc::c_int = 0;
    list = clist_new();
    if list.is_null() {
        res = MAILIMF_ERROR_MEMORY as libc::c_int
    } else {
        cur = (*(*fields).fld_list).first;
        loop {
            if cur.is_null() {
                current_block = 1109700713171191020;
                break;
            }
            let mut field: *mut mailimf_field = 0 as *mut mailimf_field;
            let mut mime_field: *mut mailmime_field = 0 as *mut mailmime_field;
            field = (if !cur.is_null() {
                (*cur).data
            } else {
                0 as *mut libc::c_void
            }) as *mut mailimf_field;
            if (*field).fld_type == MAILIMF_FIELD_OPTIONAL_FIELD as libc::c_int {
                r = mailmime_field_parse((*field).fld_data.fld_optional_field, &mut mime_field);
                if r == MAILIMF_NO_ERROR as libc::c_int {
                    r = clist_insert_after(list, (*list).last, mime_field as *mut libc::c_void);
                    if r < 0i32 {
                        mailmime_field_free(mime_field);
                        res = MAILIMF_ERROR_MEMORY as libc::c_int;
                        current_block = 17592539310030730040;
                        break;
                    }
                } else if !(r == MAILIMF_ERROR_PARSE as libc::c_int) {
                    /* do nothing */
                    res = r;
                    current_block = 17592539310030730040;
                    break;
                }
            }
            cur = if !cur.is_null() {
                (*cur).next
            } else {
                0 as *mut clistcell
            }
        }
        match current_block {
            1109700713171191020 => {
                if (*list).first.is_null() {
                    res = MAILIMF_ERROR_PARSE as libc::c_int
                } else {
                    mime_fields = mailmime_fields_new(list);
                    if mime_fields.is_null() {
                        res = MAILIMF_ERROR_MEMORY as libc::c_int
                    } else {
                        *result = mime_fields;
                        return MAILIMF_NO_ERROR as libc::c_int;
                    }
                }
            }
            _ => {}
        }
        clist_foreach(
            list,
            ::std::mem::transmute::<Option<unsafe fn(_: *mut mailmime_field) -> ()>, clist_func>(
                Some(mailmime_field_free),
            ),
            0 as *mut libc::c_void,
        );
        clist_free(list);
    }
    return res;
}
