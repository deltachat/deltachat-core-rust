use libc;
use libc::toupper;

use crate::clist::*;
use crate::mailimf::*;
use crate::mailmime::*;
use crate::mailmime_types::*;
use crate::other::*;

pub const MAILMIME_DISPOSITION_TYPE_EXTENSION: libc::c_uint = 3;
pub const MAILMIME_DISPOSITION_TYPE_ATTACHMENT: libc::c_uint = 2;
pub const MAILMIME_DISPOSITION_TYPE_INLINE: libc::c_uint = 1;
pub const MAILMIME_DISPOSITION_TYPE_ERROR: libc::c_uint = 0;

pub unsafe fn mailmime_disposition_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut mailmime_disposition,
) -> libc::c_int {
    let mut current_block: u64;
    let mut final_token: size_t = 0;
    let mut cur_token: size_t = 0;
    let mut dsp_type: *mut mailmime_disposition_type = 0 as *mut mailmime_disposition_type;
    let mut list: *mut clist = 0 as *mut clist;
    let mut dsp: *mut mailmime_disposition = 0 as *mut mailmime_disposition;
    let mut r: libc::c_int = 0;
    let mut res: libc::c_int = 0;
    cur_token = *indx;
    r = mailmime_disposition_type_parse(message, length, &mut cur_token, &mut dsp_type);
    if r != MAILIMF_NO_ERROR as libc::c_int {
        res = r
    } else {
        list = clist_new();
        if list.is_null() {
            res = MAILIMF_ERROR_MEMORY as libc::c_int
        } else {
            loop {
                let mut param: *mut mailmime_disposition_parm = 0 as *mut mailmime_disposition_parm;
                final_token = cur_token;
                r = mailimf_unstrict_char_parse(
                    message,
                    length,
                    &mut cur_token,
                    ';' as i32 as libc::c_char,
                );
                if r == MAILIMF_NO_ERROR as libc::c_int {
                    param = 0 as *mut mailmime_disposition_parm;
                    r = mailmime_disposition_parm_parse(
                        message,
                        length,
                        &mut cur_token,
                        &mut param,
                    );
                    if r == MAILIMF_NO_ERROR as libc::c_int {
                        r = clist_insert_after(list, (*list).last, param as *mut libc::c_void);
                        if !(r < 0i32) {
                            continue;
                        }
                        res = MAILIMF_ERROR_MEMORY as libc::c_int;
                        current_block = 18290070879695007868;
                        break;
                    } else if r == MAILIMF_ERROR_PARSE as libc::c_int {
                        cur_token = final_token;
                        current_block = 652864300344834934;
                        break;
                    } else {
                        res = r;
                        current_block = 18290070879695007868;
                        break;
                    }
                } else {
                    /* do nothing */
                    if r == MAILIMF_ERROR_PARSE as libc::c_int {
                        current_block = 652864300344834934;
                        break;
                    }
                    res = r;
                    current_block = 18290070879695007868;
                    break;
                }
            }
            match current_block {
                652864300344834934 => {
                    dsp = mailmime_disposition_new(dsp_type, list);
                    if dsp.is_null() {
                        res = MAILIMF_ERROR_MEMORY as libc::c_int
                    } else {
                        *result = dsp;
                        *indx = cur_token;
                        return MAILIMF_NO_ERROR as libc::c_int;
                    }
                }
                _ => {}
            }
            clist_foreach(
                list,
                ::std::mem::transmute::<
                    Option<unsafe fn(_: *mut mailmime_disposition_parm) -> ()>,
                    clist_func,
                >(Some(mailmime_disposition_parm_free)),
                0 as *mut libc::c_void,
            );
            clist_free(list);
        }
        mailmime_disposition_type_free(dsp_type);
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
 * $Id: mailmime_disposition.c,v 1.17 2011/05/03 16:30:22 hoa Exp $
 */
unsafe fn mailmime_disposition_parm_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut mailmime_disposition_parm,
) -> libc::c_int {
    let mut current_block: u64;
    let mut filename: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut creation_date: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut modification_date: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut read_date: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut size: size_t = 0;
    let mut parameter: *mut mailmime_parameter = 0 as *mut mailmime_parameter;
    let mut cur_token: size_t = 0;
    let mut dsp_parm: *mut mailmime_disposition_parm = 0 as *mut mailmime_disposition_parm;
    let mut type_0: libc::c_int = 0;
    let mut guessed_type: libc::c_int = 0;
    let mut r: libc::c_int = 0;
    let mut res: libc::c_int = 0;
    cur_token = *indx;
    filename = 0 as *mut libc::c_char;
    creation_date = 0 as *mut libc::c_char;
    modification_date = 0 as *mut libc::c_char;
    read_date = 0 as *mut libc::c_char;
    size = 0i32 as size_t;
    parameter = 0 as *mut mailmime_parameter;
    r = mailimf_cfws_parse(message, length, &mut cur_token);
    if r != MAILIMF_NO_ERROR as libc::c_int && r != MAILIMF_ERROR_PARSE as libc::c_int {
        res = r
    } else {
        guessed_type = mailmime_disposition_guess_type(message, length, cur_token);
        type_0 = MAILMIME_DISPOSITION_PARM_PARAMETER as libc::c_int;
        match guessed_type {
            0 => {
                r = mailmime_filename_parm_parse(message, length, &mut cur_token, &mut filename);
                if r == MAILIMF_NO_ERROR as libc::c_int {
                    type_0 = guessed_type;
                    current_block = 13826291924415791078;
                } else if r == MAILIMF_ERROR_PARSE as libc::c_int {
                    current_block = 13826291924415791078;
                } else {
                    /* do nothing */
                    res = r;
                    current_block = 9120900589700563584;
                }
            }
            1 => {
                r = mailmime_creation_date_parm_parse(
                    message,
                    length,
                    &mut cur_token,
                    &mut creation_date,
                );
                if r == MAILIMF_NO_ERROR as libc::c_int {
                    type_0 = guessed_type;
                    current_block = 13826291924415791078;
                } else if r == MAILIMF_ERROR_PARSE as libc::c_int {
                    current_block = 13826291924415791078;
                } else {
                    /* do nothing */
                    res = r;
                    current_block = 9120900589700563584;
                }
            }
            2 => {
                r = mailmime_modification_date_parm_parse(
                    message,
                    length,
                    &mut cur_token,
                    &mut modification_date,
                );
                if r == MAILIMF_NO_ERROR as libc::c_int {
                    type_0 = guessed_type;
                    current_block = 13826291924415791078;
                } else if r == MAILIMF_ERROR_PARSE as libc::c_int {
                    current_block = 13826291924415791078;
                } else {
                    /* do nothing */
                    res = r;
                    current_block = 9120900589700563584;
                }
            }
            3 => {
                r = mailmime_read_date_parm_parse(message, length, &mut cur_token, &mut read_date);
                if r == MAILIMF_NO_ERROR as libc::c_int {
                    type_0 = guessed_type;
                    current_block = 13826291924415791078;
                } else if r == MAILIMF_ERROR_PARSE as libc::c_int {
                    current_block = 13826291924415791078;
                } else {
                    /* do nothing */
                    res = r;
                    current_block = 9120900589700563584;
                }
            }
            4 => {
                r = mailmime_size_parm_parse(message, length, &mut cur_token, &mut size);
                if r == MAILIMF_NO_ERROR as libc::c_int {
                    type_0 = guessed_type;
                    current_block = 13826291924415791078;
                } else if r == MAILIMF_ERROR_PARSE as libc::c_int {
                    current_block = 13826291924415791078;
                } else {
                    /* do nothing */
                    res = r;
                    current_block = 9120900589700563584;
                }
            }
            _ => {
                current_block = 13826291924415791078;
            }
        }
        match current_block {
            9120900589700563584 => {}
            _ => {
                if type_0 == MAILMIME_DISPOSITION_PARM_PARAMETER as libc::c_int {
                    r = mailmime_parameter_parse(message, length, &mut cur_token, &mut parameter);
                    if r != MAILIMF_NO_ERROR as libc::c_int {
                        type_0 = guessed_type;
                        res = r;
                        current_block = 9120900589700563584;
                    } else {
                        current_block = 6721012065216013753;
                    }
                } else {
                    current_block = 6721012065216013753;
                }
                match current_block {
                    9120900589700563584 => {}
                    _ => {
                        dsp_parm = mailmime_disposition_parm_new(
                            type_0,
                            filename,
                            creation_date,
                            modification_date,
                            read_date,
                            size,
                            parameter,
                        );
                        if dsp_parm.is_null() {
                            res = MAILIMF_ERROR_MEMORY as libc::c_int;
                            if !filename.is_null() {
                                mailmime_filename_parm_free(filename);
                            }
                            if !creation_date.is_null() {
                                mailmime_creation_date_parm_free(creation_date);
                            }
                            if !modification_date.is_null() {
                                mailmime_modification_date_parm_free(modification_date);
                            }
                            if !read_date.is_null() {
                                mailmime_read_date_parm_free(read_date);
                            }
                            if !parameter.is_null() {
                                mailmime_parameter_free(parameter);
                            }
                        } else {
                            *result = dsp_parm;
                            *indx = cur_token;
                            return MAILIMF_NO_ERROR as libc::c_int;
                        }
                    }
                }
            }
        }
    }
    return res;
}
unsafe fn mailmime_size_parm_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut size_t,
) -> libc::c_int {
    let mut value: uint32_t = 0;
    let mut cur_token: size_t = 0;
    let mut r: libc::c_int = 0;
    cur_token = *indx;
    r = mailimf_token_case_insensitive_len_parse(
        message,
        length,
        &mut cur_token,
        b"size\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
        strlen(b"size\x00" as *const u8 as *const libc::c_char),
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    r = mailimf_unstrict_char_parse(message, length, &mut cur_token, '=' as i32 as libc::c_char);
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    r = mailimf_cfws_parse(message, length, &mut cur_token);
    if r != MAILIMF_NO_ERROR as libc::c_int && r != MAILIMF_ERROR_PARSE as libc::c_int {
        return r;
    }
    r = mailimf_number_parse(message, length, &mut cur_token, &mut value);
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    *indx = cur_token;
    *result = value as size_t;
    return MAILIMF_NO_ERROR as libc::c_int;
}
unsafe fn mailmime_read_date_parm_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut libc::c_char,
) -> libc::c_int {
    let mut value: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut cur_token: size_t = 0;
    let mut r: libc::c_int = 0;
    cur_token = *indx;
    r = mailimf_token_case_insensitive_len_parse(
        message,
        length,
        &mut cur_token,
        b"read-date\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
        strlen(b"read-date\x00" as *const u8 as *const libc::c_char),
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    r = mailimf_unstrict_char_parse(message, length, &mut cur_token, '=' as i32 as libc::c_char);
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    r = mailimf_cfws_parse(message, length, &mut cur_token);
    if r != MAILIMF_NO_ERROR as libc::c_int && r != MAILIMF_ERROR_PARSE as libc::c_int {
        return r;
    }
    r = mailmime_quoted_date_time_parse(message, length, &mut cur_token, &mut value);
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    *indx = cur_token;
    *result = value;
    return MAILIMF_NO_ERROR as libc::c_int;
}
unsafe fn mailmime_quoted_date_time_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut libc::c_char,
) -> libc::c_int {
    return mailimf_quoted_string_parse(message, length, indx, result);
}
unsafe fn mailmime_modification_date_parm_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut libc::c_char,
) -> libc::c_int {
    let mut value: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut cur_token: size_t = 0;
    let mut r: libc::c_int = 0;
    cur_token = *indx;
    r = mailimf_token_case_insensitive_len_parse(
        message,
        length,
        &mut cur_token,
        b"modification-date\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
        strlen(b"modification-date\x00" as *const u8 as *const libc::c_char),
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    r = mailimf_unstrict_char_parse(message, length, &mut cur_token, '=' as i32 as libc::c_char);
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    r = mailimf_cfws_parse(message, length, &mut cur_token);
    if r != MAILIMF_NO_ERROR as libc::c_int && r != MAILIMF_ERROR_PARSE as libc::c_int {
        return r;
    }
    r = mailmime_quoted_date_time_parse(message, length, &mut cur_token, &mut value);
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    *indx = cur_token;
    *result = value;
    return MAILIMF_NO_ERROR as libc::c_int;
}
unsafe fn mailmime_creation_date_parm_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut libc::c_char,
) -> libc::c_int {
    let mut value: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut r: libc::c_int = 0;
    let mut cur_token: size_t = 0;
    cur_token = *indx;
    r = mailimf_token_case_insensitive_len_parse(
        message,
        length,
        &mut cur_token,
        b"creation-date\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
        strlen(b"creation-date\x00" as *const u8 as *const libc::c_char),
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    r = mailimf_unstrict_char_parse(message, length, &mut cur_token, '=' as i32 as libc::c_char);
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    r = mailimf_cfws_parse(message, length, &mut cur_token);
    if r != MAILIMF_NO_ERROR as libc::c_int && r != MAILIMF_ERROR_PARSE as libc::c_int {
        return r;
    }
    r = mailmime_quoted_date_time_parse(message, length, &mut cur_token, &mut value);
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    *indx = cur_token;
    *result = value;
    return MAILIMF_NO_ERROR as libc::c_int;
}
unsafe fn mailmime_filename_parm_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut libc::c_char,
) -> libc::c_int {
    let mut value: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut r: libc::c_int = 0;
    let mut cur_token: size_t = 0;
    cur_token = *indx;
    r = mailimf_token_case_insensitive_len_parse(
        message,
        length,
        &mut cur_token,
        b"filename\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
        strlen(b"filename\x00" as *const u8 as *const libc::c_char),
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    r = mailimf_unstrict_char_parse(message, length, &mut cur_token, '=' as i32 as libc::c_char);
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    r = mailimf_cfws_parse(message, length, &mut cur_token);
    if r != MAILIMF_NO_ERROR as libc::c_int && r != MAILIMF_ERROR_PARSE as libc::c_int {
        return r;
    }
    r = mailmime_value_parse(message, length, &mut cur_token, &mut value);
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    *indx = cur_token;
    *result = value;
    return MAILIMF_NO_ERROR as libc::c_int;
}

pub unsafe fn mailmime_disposition_guess_type(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: size_t,
) -> libc::c_int {
    if indx >= length {
        return MAILMIME_DISPOSITION_PARM_PARAMETER as libc::c_int;
    }
    match toupper(*message.offset(indx as isize) as libc::c_uchar as libc::c_int) as libc::c_char
        as libc::c_int
    {
        70 => return MAILMIME_DISPOSITION_PARM_FILENAME as libc::c_int,
        67 => return MAILMIME_DISPOSITION_PARM_CREATION_DATE as libc::c_int,
        77 => return MAILMIME_DISPOSITION_PARM_MODIFICATION_DATE as libc::c_int,
        82 => return MAILMIME_DISPOSITION_PARM_READ_DATE as libc::c_int,
        83 => return MAILMIME_DISPOSITION_PARM_SIZE as libc::c_int,
        _ => return MAILMIME_DISPOSITION_PARM_PARAMETER as libc::c_int,
    };
}

pub unsafe fn mailmime_disposition_type_parse(
    mut message: *const libc::c_char,
    mut length: size_t,
    mut indx: *mut size_t,
    mut result: *mut *mut mailmime_disposition_type,
) -> libc::c_int {
    let mut cur_token: size_t = 0;
    let mut type_0: libc::c_int = 0;
    let mut extension: *mut libc::c_char = 0 as *mut libc::c_char;
    let mut dsp_type: *mut mailmime_disposition_type = 0 as *mut mailmime_disposition_type;
    let mut r: libc::c_int = 0;
    let mut res: libc::c_int = 0;
    cur_token = *indx;
    r = mailimf_cfws_parse(message, length, &mut cur_token);
    if r != MAILIMF_NO_ERROR as libc::c_int && r != MAILIMF_ERROR_PARSE as libc::c_int {
        res = r
    } else {
        type_0 = MAILMIME_DISPOSITION_TYPE_ERROR as libc::c_int;
        extension = 0 as *mut libc::c_char;
        r = mailimf_token_case_insensitive_len_parse(
            message,
            length,
            &mut cur_token,
            b"inline\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
            strlen(b"inline\x00" as *const u8 as *const libc::c_char),
        );
        if r == MAILIMF_NO_ERROR as libc::c_int {
            type_0 = MAILMIME_DISPOSITION_TYPE_INLINE as libc::c_int
        }
        if r == MAILIMF_ERROR_PARSE as libc::c_int {
            r = mailimf_token_case_insensitive_len_parse(
                message,
                length,
                &mut cur_token,
                b"attachment\x00" as *const u8 as *const libc::c_char as *mut libc::c_char,
                strlen(b"attachment\x00" as *const u8 as *const libc::c_char),
            );
            if r == MAILIMF_NO_ERROR as libc::c_int {
                type_0 = MAILMIME_DISPOSITION_TYPE_ATTACHMENT as libc::c_int
            }
        }
        if r == MAILIMF_ERROR_PARSE as libc::c_int {
            r = mailmime_extension_token_parse(message, length, &mut cur_token, &mut extension);
            if r == MAILIMF_NO_ERROR as libc::c_int {
                type_0 = MAILMIME_DISPOSITION_TYPE_EXTENSION as libc::c_int
            }
        }
        if r != MAILIMF_NO_ERROR as libc::c_int {
            res = r
        } else {
            dsp_type = mailmime_disposition_type_new(type_0, extension);
            if dsp_type.is_null() {
                res = MAILIMF_ERROR_MEMORY as libc::c_int;
                if !extension.is_null() {
                    free(extension as *mut libc::c_void);
                }
            } else {
                *result = dsp_type;
                *indx = cur_token;
                return MAILIMF_NO_ERROR as libc::c_int;
            }
        }
    }
    return res;
}
