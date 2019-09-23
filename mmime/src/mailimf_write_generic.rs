use crate::clist::*;
use crate::mailimf_types::*;
use crate::other::*;

pub const STATE_WORD: libc::c_uint = 1;
pub const STATE_SPACE: libc::c_uint = 2;
pub const STATE_BEGIN: libc::c_uint = 0;

/*
  mailimf_string_write writes a string to a given stream

  @param f is the stream
  @param col (* col) is the column number where we will start to
    write the text, the ending column will be stored in (* col)
  @param str is the string to write
*/
pub unsafe fn mailimf_string_write_driver(
    mut do_write: Option<
        unsafe fn(_: *mut libc::c_void, _: *const libc::c_char, _: size_t) -> libc::c_int,
    >,
    mut data: *mut libc::c_void,
    mut col: *mut libc::c_int,
    mut str: *const libc::c_char,
    mut length: size_t,
) -> libc::c_int {
    let mut r: libc::c_int = 0;
    let mut count: size_t = 0;
    let mut block_begin: *const libc::c_char = 0 as *const libc::c_char;
    let mut p: *const libc::c_char = 0 as *const libc::c_char;
    let mut done: libc::c_int = 0;
    p = str;
    block_begin = str;
    count = 0i32 as size_t;
    while length > 0i32 as libc::size_t {
        if count >= 998i32 as libc::size_t {
            r = flush_buf(do_write, data, block_begin, count);
            if r != MAILIMF_NO_ERROR as libc::c_int {
                return r;
            }
            r = do_write.expect("non-null function pointer")(
                data,
                b"\r\n\x00" as *const u8 as *const libc::c_char,
                (::std::mem::size_of::<[libc::c_char; 3]>() as libc::size_t)
                    .wrapping_sub(1i32 as libc::size_t),
            );
            if r == 0i32 {
                return MAILIMF_ERROR_FILE as libc::c_int;
            }
            count = 0i32 as size_t;
            block_begin = p;
            *col = 0i32
        }
        match *p as libc::c_int {
            10 => {
                r = flush_buf(do_write, data, block_begin, count);
                if r != MAILIMF_NO_ERROR as libc::c_int {
                    return r;
                }
                r = do_write.expect("non-null function pointer")(
                    data,
                    b"\r\n\x00" as *const u8 as *const libc::c_char,
                    (::std::mem::size_of::<[libc::c_char; 3]>() as libc::size_t)
                        .wrapping_sub(1i32 as libc::size_t),
                );
                if r == 0i32 {
                    return MAILIMF_ERROR_FILE as libc::c_int;
                }
                p = p.offset(1isize);
                length = length.wrapping_sub(1);
                count = 0i32 as size_t;
                block_begin = p;
                *col = 0i32
            }
            13 => {
                done = 0i32;
                if length >= 2i32 as libc::size_t {
                    if *p.offset(1isize) as libc::c_int == '\n' as i32 {
                        r = flush_buf(do_write, data, block_begin, count);
                        if r != MAILIMF_NO_ERROR as libc::c_int {
                            return r;
                        }
                        r = do_write.expect("non-null function pointer")(
                            data,
                            b"\r\n\x00" as *const u8 as *const libc::c_char,
                            (::std::mem::size_of::<[libc::c_char; 3]>() as libc::size_t)
                                .wrapping_sub(1i32 as libc::size_t),
                        );
                        if r == 0i32 {
                            return MAILIMF_ERROR_FILE as libc::c_int;
                        }
                        p = p.offset(2isize);
                        length = (length as libc::size_t).wrapping_sub(2i32 as libc::size_t)
                            as size_t as size_t;
                        count = 0i32 as size_t;
                        block_begin = p;
                        *col = 0i32;
                        done = 1i32
                    }
                }
                if 0 == done {
                    r = flush_buf(do_write, data, block_begin, count);
                    if r != MAILIMF_NO_ERROR as libc::c_int {
                        return r;
                    }
                    r = do_write.expect("non-null function pointer")(
                        data,
                        b"\r\n\x00" as *const u8 as *const libc::c_char,
                        (::std::mem::size_of::<[libc::c_char; 3]>() as libc::size_t)
                            .wrapping_sub(1i32 as libc::size_t),
                    );
                    if r == 0i32 {
                        return MAILIMF_ERROR_FILE as libc::c_int;
                    }
                    p = p.offset(1isize);
                    length = length.wrapping_sub(1);
                    count = 0i32 as size_t;
                    block_begin = p;
                    *col = 0i32
                }
            }
            _ => {
                p = p.offset(1isize);
                count = count.wrapping_add(1);
                length = length.wrapping_sub(1)
            }
        }
    }
    r = flush_buf(do_write, data, block_begin, count);
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    *col = (*col as libc::size_t).wrapping_add(count) as libc::c_int as libc::c_int;
    return MAILIMF_NO_ERROR as libc::c_int;
}
/* ************************ */
#[inline]
unsafe fn flush_buf(
    mut do_write: Option<
        unsafe fn(_: *mut libc::c_void, _: *const libc::c_char, _: size_t) -> libc::c_int,
    >,
    mut data: *mut libc::c_void,
    mut str: *const libc::c_char,
    mut length: size_t,
) -> libc::c_int {
    if length != 0i32 as libc::size_t {
        let mut r: libc::c_int = 0;
        if length > 0i32 as libc::size_t {
            r = do_write.expect("non-null function pointer")(data, str, length);
            if r == 0i32 {
                return MAILIMF_ERROR_FILE as libc::c_int;
            }
        }
    }
    return MAILIMF_NO_ERROR as libc::c_int;
}
/*
  mailimf_fields_write writes the fields to a given stream

  @param f is the stream
  @param col (* col) is the column number where we will start to
    write the text, the ending column will be stored in (* col)
  @param fields is the fields to write
*/

pub unsafe fn mailimf_fields_write_driver(
    mut do_write: Option<
        unsafe fn(_: *mut libc::c_void, _: *const libc::c_char, _: size_t) -> libc::c_int,
    >,
    mut data: *mut libc::c_void,
    mut col: *mut libc::c_int,
    mut fields: *mut mailimf_fields,
) -> libc::c_int {
    let mut cur: *mut clistiter = 0 as *mut clistiter;
    cur = (*(*fields).fld_list).first;
    while !cur.is_null() {
        let mut r: libc::c_int = 0;
        r = mailimf_field_write_driver(
            do_write,
            data,
            col,
            (if !cur.is_null() {
                (*cur).data
            } else {
                0 as *mut libc::c_void
            }) as *mut mailimf_field,
        );
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
/*
  mailimf_field_write writes a field to a given stream

  @param f is the stream
  @param col (* col) is the column number where we will start to
    write the text, the ending column will be stored in (* col)
  @param field is the field to write
*/
pub unsafe fn mailimf_field_write_driver(
    mut do_write: Option<
        unsafe fn(_: *mut libc::c_void, _: *const libc::c_char, _: size_t) -> libc::c_int,
    >,
    mut data: *mut libc::c_void,
    mut col: *mut libc::c_int,
    mut field: *mut mailimf_field,
) -> libc::c_int {
    let mut r: libc::c_int = 0;
    match (*field).fld_type {
        1 => {
            r = mailimf_return_write_driver(do_write, data, col, (*field).fld_data.fld_return_path)
        }
        2 => {
            r = mailimf_resent_date_write_driver(
                do_write,
                data,
                col,
                (*field).fld_data.fld_resent_date,
            )
        }
        3 => {
            r = mailimf_resent_from_write_driver(
                do_write,
                data,
                col,
                (*field).fld_data.fld_resent_from,
            )
        }
        4 => {
            r = mailimf_resent_sender_write_driver(
                do_write,
                data,
                col,
                (*field).fld_data.fld_resent_sender,
            )
        }
        5 => {
            r = mailimf_resent_to_write_driver(do_write, data, col, (*field).fld_data.fld_resent_to)
        }
        6 => {
            r = mailimf_resent_cc_write_driver(do_write, data, col, (*field).fld_data.fld_resent_cc)
        }
        7 => {
            r = mailimf_resent_bcc_write_driver(
                do_write,
                data,
                col,
                (*field).fld_data.fld_resent_bcc,
            )
        }
        8 => {
            r = mailimf_resent_msg_id_write_driver(
                do_write,
                data,
                col,
                (*field).fld_data.fld_resent_msg_id,
            )
        }
        9 => {
            r = mailimf_orig_date_write_driver(do_write, data, col, (*field).fld_data.fld_orig_date)
        }
        10 => r = mailimf_from_write_driver(do_write, data, col, (*field).fld_data.fld_from),
        11 => r = mailimf_sender_write_driver(do_write, data, col, (*field).fld_data.fld_sender),
        12 => {
            r = mailimf_reply_to_write_driver(do_write, data, col, (*field).fld_data.fld_reply_to)
        }
        13 => r = mailimf_to_write_driver(do_write, data, col, (*field).fld_data.fld_to),
        14 => r = mailimf_cc_write_driver(do_write, data, col, (*field).fld_data.fld_cc),
        15 => r = mailimf_bcc_write_driver(do_write, data, col, (*field).fld_data.fld_bcc),
        16 => {
            r = mailimf_message_id_write_driver(
                do_write,
                data,
                col,
                (*field).fld_data.fld_message_id,
            )
        }
        17 => {
            r = mailimf_in_reply_to_write_driver(
                do_write,
                data,
                col,
                (*field).fld_data.fld_in_reply_to,
            )
        }
        18 => {
            r = mailimf_references_write_driver(
                do_write,
                data,
                col,
                (*field).fld_data.fld_references,
            )
        }
        19 => r = mailimf_subject_write_driver(do_write, data, col, (*field).fld_data.fld_subject),
        20 => {
            r = mailimf_comments_write_driver(do_write, data, col, (*field).fld_data.fld_comments)
        }
        21 => {
            r = mailimf_keywords_write_driver(do_write, data, col, (*field).fld_data.fld_keywords)
        }
        22 => {
            r = mailimf_optional_field_write_driver(
                do_write,
                data,
                col,
                (*field).fld_data.fld_optional_field,
            )
        }
        _ => r = MAILIMF_ERROR_INVAL as libc::c_int,
    }
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    return MAILIMF_NO_ERROR as libc::c_int;
}
unsafe fn mailimf_optional_field_write_driver(
    mut do_write: Option<
        unsafe fn(_: *mut libc::c_void, _: *const libc::c_char, _: size_t) -> libc::c_int,
    >,
    mut data: *mut libc::c_void,
    mut col: *mut libc::c_int,
    mut field: *mut mailimf_optional_field,
) -> libc::c_int {
    let mut r: libc::c_int = 0;
    if strlen((*field).fld_name).wrapping_add(2i32 as libc::size_t) > 998i32 as libc::size_t {
        return MAILIMF_ERROR_INVAL as libc::c_int;
    }
    r = mailimf_string_write_driver(
        do_write,
        data,
        col,
        (*field).fld_name,
        strlen((*field).fld_name),
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    r = mailimf_string_write_driver(
        do_write,
        data,
        col,
        b": \x00" as *const u8 as *const libc::c_char,
        2i32 as size_t,
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    r = mailimf_header_string_write_driver(
        do_write,
        data,
        col,
        (*field).fld_value,
        strlen((*field).fld_value),
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
/*
  mailimf_header_string_write writes a header value and fold the header
    if needed.

  @param f is the stream
  @param col (* col) is the column number where we will start to
    write the text, the ending column will be stored in (* col)
  @param str is the string to write
*/
pub unsafe fn mailimf_header_string_write_driver(
    mut do_write: Option<
        unsafe fn(_: *mut libc::c_void, _: *const libc::c_char, _: size_t) -> libc::c_int,
    >,
    mut data: *mut libc::c_void,
    mut col: *mut libc::c_int,
    mut str: *const libc::c_char,
    mut length: size_t,
) -> libc::c_int {
    let mut state: libc::c_int = 0;
    let mut p: *const libc::c_char = 0 as *const libc::c_char;
    let mut word_begin: *const libc::c_char = 0 as *const libc::c_char;
    let mut first: libc::c_int = 0;
    state = STATE_BEGIN as libc::c_int;
    p = str;
    word_begin = p;
    first = 1i32;
    while length > 0i32 as libc::size_t {
        match state {
            0 => match *p as libc::c_int {
                13 | 10 | 32 | 9 => {
                    p = p.offset(1isize);
                    length = length.wrapping_sub(1)
                }
                _ => {
                    word_begin = p;
                    state = STATE_WORD as libc::c_int
                }
            },
            2 => match *p as libc::c_int {
                13 | 10 | 32 | 9 => {
                    p = p.offset(1isize);
                    length = length.wrapping_sub(1)
                }
                _ => {
                    word_begin = p;
                    state = STATE_WORD as libc::c_int
                }
            },
            1 => match *p as libc::c_int {
                13 | 10 | 32 | 9 => {
                    if p.wrapping_offset_from(word_begin) as libc::c_int
                        + *col as libc::c_int
                        + 1i32 as libc::c_int
                        > 72i32 as libc::c_int
                    {
                        mailimf_string_write_driver(
                            do_write,
                            data,
                            col,
                            b"\r\n \x00" as *const u8 as *const libc::c_char,
                            (::std::mem::size_of::<[libc::c_char; 4]>() as libc::size_t)
                                .wrapping_sub(1i32 as libc::size_t),
                        );
                    } else if 0 == first {
                        mailimf_string_write_driver(
                            do_write,
                            data,
                            col,
                            b" \x00" as *const u8 as *const libc::c_char,
                            1i32 as size_t,
                        );
                    }
                    first = 0i32;
                    mailimf_string_write_driver(
                        do_write,
                        data,
                        col,
                        word_begin,
                        p.wrapping_offset_from(word_begin) as libc::c_int as size_t,
                    );
                    state = STATE_SPACE as libc::c_int
                }
                _ => {
                    if p.wrapping_offset_from(word_begin) as libc::c_int + *col as libc::c_int
                        >= 998i32 as libc::c_int
                    {
                        mailimf_string_write_driver(
                            do_write,
                            data,
                            col,
                            word_begin,
                            p.wrapping_offset_from(word_begin) as libc::c_int as size_t,
                        );
                        mailimf_string_write_driver(
                            do_write,
                            data,
                            col,
                            b"\r\n \x00" as *const u8 as *const libc::c_char,
                            (::std::mem::size_of::<[libc::c_char; 4]>() as libc::size_t)
                                .wrapping_sub(1i32 as libc::size_t),
                        );
                        word_begin = p
                    }
                    p = p.offset(1isize);
                    length = length.wrapping_sub(1)
                }
            },
            _ => {}
        }
    }
    if state == STATE_WORD as libc::c_int {
        if p.wrapping_offset_from(word_begin) as libc::c_int + *col as libc::c_int
            >= 72i32 as libc::c_int
        {
            mailimf_string_write_driver(
                do_write,
                data,
                col,
                b"\r\n \x00" as *const u8 as *const libc::c_char,
                (::std::mem::size_of::<[libc::c_char; 4]>() as libc::size_t)
                    .wrapping_sub(1i32 as libc::size_t),
            );
        } else if 0 == first {
            mailimf_string_write_driver(
                do_write,
                data,
                col,
                b" \x00" as *const u8 as *const libc::c_char,
                1i32 as size_t,
            );
        }
        first = 0i32;
        mailimf_string_write_driver(
            do_write,
            data,
            col,
            word_begin,
            p.wrapping_offset_from(word_begin) as libc::c_int as size_t,
        );
    }
    return MAILIMF_NO_ERROR as libc::c_int;
}
unsafe fn mailimf_keywords_write_driver(
    mut do_write: Option<
        unsafe fn(_: *mut libc::c_void, _: *const libc::c_char, _: size_t) -> libc::c_int,
    >,
    mut data: *mut libc::c_void,
    mut col: *mut libc::c_int,
    mut keywords: *mut mailimf_keywords,
) -> libc::c_int {
    let mut r: libc::c_int = 0;
    let mut cur: *mut clistiter = 0 as *mut clistiter;
    let mut first: libc::c_int = 0;
    r = mailimf_string_write_driver(
        do_write,
        data,
        col,
        b"Keywords: \x00" as *const u8 as *const libc::c_char,
        10i32 as size_t,
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    first = 1i32;
    cur = (*(*keywords).kw_list).first;
    while !cur.is_null() {
        let mut keyword: *mut libc::c_char = 0 as *mut libc::c_char;
        let mut len: size_t = 0;
        keyword = (if !cur.is_null() {
            (*cur).data
        } else {
            0 as *mut libc::c_void
        }) as *mut libc::c_char;
        len = strlen(keyword);
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
        r = mailimf_header_string_write_driver(do_write, data, col, keyword, len);
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
unsafe fn mailimf_comments_write_driver(
    mut do_write: Option<
        unsafe fn(_: *mut libc::c_void, _: *const libc::c_char, _: size_t) -> libc::c_int,
    >,
    mut data: *mut libc::c_void,
    mut col: *mut libc::c_int,
    mut comments: *mut mailimf_comments,
) -> libc::c_int {
    let mut r: libc::c_int = 0;
    r = mailimf_string_write_driver(
        do_write,
        data,
        col,
        b"Comments: \x00" as *const u8 as *const libc::c_char,
        10i32 as size_t,
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    r = mailimf_header_string_write_driver(
        do_write,
        data,
        col,
        (*comments).cm_value,
        strlen((*comments).cm_value),
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
unsafe fn mailimf_subject_write_driver(
    mut do_write: Option<
        unsafe fn(_: *mut libc::c_void, _: *const libc::c_char, _: size_t) -> libc::c_int,
    >,
    mut data: *mut libc::c_void,
    mut col: *mut libc::c_int,
    mut subject: *mut mailimf_subject,
) -> libc::c_int {
    let mut r: libc::c_int = 0;
    r = mailimf_string_write_driver(
        do_write,
        data,
        col,
        b"Subject: \x00" as *const u8 as *const libc::c_char,
        9i32 as size_t,
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    r = mailimf_header_string_write_driver(
        do_write,
        data,
        col,
        (*subject).sbj_value,
        strlen((*subject).sbj_value),
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
unsafe fn mailimf_references_write_driver(
    mut do_write: Option<
        unsafe fn(_: *mut libc::c_void, _: *const libc::c_char, _: size_t) -> libc::c_int,
    >,
    mut data: *mut libc::c_void,
    mut col: *mut libc::c_int,
    mut references: *mut mailimf_references,
) -> libc::c_int {
    let mut r: libc::c_int = 0;
    r = mailimf_string_write_driver(
        do_write,
        data,
        col,
        b"References: \x00" as *const u8 as *const libc::c_char,
        12i32 as size_t,
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    r = mailimf_msg_id_list_write_driver(do_write, data, col, (*references).mid_list);
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
unsafe fn mailimf_msg_id_list_write_driver(
    mut do_write: Option<
        unsafe fn(_: *mut libc::c_void, _: *const libc::c_char, _: size_t) -> libc::c_int,
    >,
    mut data: *mut libc::c_void,
    mut col: *mut libc::c_int,
    mut mid_list: *mut clist,
) -> libc::c_int {
    let mut cur: *mut clistiter = 0 as *mut clistiter;
    let mut r: libc::c_int = 0;
    let mut first: libc::c_int = 0;
    first = 1i32;
    cur = (*mid_list).first;
    while !cur.is_null() {
        let mut msgid: *mut libc::c_char = 0 as *mut libc::c_char;
        let mut len: size_t = 0;
        msgid = (if !cur.is_null() {
            (*cur).data
        } else {
            0 as *mut libc::c_void
        }) as *mut libc::c_char;
        len = strlen(msgid);
        if 0 == first {
            if *col > 1i32 {
                if (*col as libc::size_t).wrapping_add(len) >= 72i32 as libc::size_t {
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
                    first = 1i32
                }
            }
        }
        if 0 == first {
            r = mailimf_string_write_driver(
                do_write,
                data,
                col,
                b" \x00" as *const u8 as *const libc::c_char,
                1i32 as size_t,
            );
            if r != MAILIMF_NO_ERROR as libc::c_int {
                return r;
            }
        } else {
            first = 0i32
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
        r = mailimf_string_write_driver(do_write, data, col, msgid, len);
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
        cur = if !cur.is_null() {
            (*cur).next
        } else {
            0 as *mut clistcell
        }
    }
    return MAILIMF_NO_ERROR as libc::c_int;
}
unsafe fn mailimf_in_reply_to_write_driver(
    mut do_write: Option<
        unsafe fn(_: *mut libc::c_void, _: *const libc::c_char, _: size_t) -> libc::c_int,
    >,
    mut data: *mut libc::c_void,
    mut col: *mut libc::c_int,
    mut in_reply_to: *mut mailimf_in_reply_to,
) -> libc::c_int {
    let mut r: libc::c_int = 0;
    r = mailimf_string_write_driver(
        do_write,
        data,
        col,
        b"In-Reply-To: \x00" as *const u8 as *const libc::c_char,
        13i32 as size_t,
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    r = mailimf_msg_id_list_write_driver(do_write, data, col, (*in_reply_to).mid_list);
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
unsafe fn mailimf_message_id_write_driver(
    mut do_write: Option<
        unsafe fn(_: *mut libc::c_void, _: *const libc::c_char, _: size_t) -> libc::c_int,
    >,
    mut data: *mut libc::c_void,
    mut col: *mut libc::c_int,
    mut message_id: *mut mailimf_message_id,
) -> libc::c_int {
    let mut r: libc::c_int = 0;
    r = mailimf_string_write_driver(
        do_write,
        data,
        col,
        b"Message-ID: \x00" as *const u8 as *const libc::c_char,
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
    r = mailimf_string_write_driver(
        do_write,
        data,
        col,
        (*message_id).mid_value,
        strlen((*message_id).mid_value),
    );
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
unsafe fn mailimf_bcc_write_driver(
    mut do_write: Option<
        unsafe fn(_: *mut libc::c_void, _: *const libc::c_char, _: size_t) -> libc::c_int,
    >,
    mut data: *mut libc::c_void,
    mut col: *mut libc::c_int,
    mut bcc: *mut mailimf_bcc,
) -> libc::c_int {
    let mut r: libc::c_int = 0;
    r = mailimf_string_write_driver(
        do_write,
        data,
        col,
        b"Bcc: \x00" as *const u8 as *const libc::c_char,
        5i32 as size_t,
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    if !(*bcc).bcc_addr_list.is_null() {
        r = mailimf_address_list_write_driver(do_write, data, col, (*bcc).bcc_addr_list);
        if r != MAILIMF_NO_ERROR as libc::c_int {
            return r;
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
pub unsafe fn mailimf_address_list_write_driver(
    mut do_write: Option<
        unsafe fn(_: *mut libc::c_void, _: *const libc::c_char, _: size_t) -> libc::c_int,
    >,
    mut data: *mut libc::c_void,
    mut col: *mut libc::c_int,
    mut addr_list: *mut mailimf_address_list,
) -> libc::c_int {
    let mut cur: *mut clistiter = 0 as *mut clistiter;
    let mut r: libc::c_int = 0;
    let mut first: libc::c_int = 0;
    first = 1i32;
    cur = (*(*addr_list).ad_list).first;
    while !cur.is_null() {
        let mut addr: *mut mailimf_address = 0 as *mut mailimf_address;
        addr = (if !cur.is_null() {
            (*cur).data
        } else {
            0 as *mut libc::c_void
        }) as *mut mailimf_address;
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
        r = mailimf_address_write_driver(do_write, data, col, addr);
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
unsafe fn mailimf_address_write_driver(
    mut do_write: Option<
        unsafe fn(_: *mut libc::c_void, _: *const libc::c_char, _: size_t) -> libc::c_int,
    >,
    mut data: *mut libc::c_void,
    mut col: *mut libc::c_int,
    mut addr: *mut mailimf_address,
) -> libc::c_int {
    let mut r: libc::c_int = 0;
    match (*addr).ad_type {
        1 => {
            r = mailimf_mailbox_write_driver(do_write, data, col, (*addr).ad_data.ad_mailbox);
            if r != MAILIMF_NO_ERROR as libc::c_int {
                return r;
            }
        }
        2 => {
            r = mailimf_group_write_driver(do_write, data, col, (*addr).ad_data.ad_group);
            if r != MAILIMF_NO_ERROR as libc::c_int {
                return r;
            }
        }
        _ => {}
    }
    return MAILIMF_NO_ERROR as libc::c_int;
}
unsafe fn mailimf_group_write_driver(
    mut do_write: Option<
        unsafe fn(_: *mut libc::c_void, _: *const libc::c_char, _: size_t) -> libc::c_int,
    >,
    mut data: *mut libc::c_void,
    mut col: *mut libc::c_int,
    mut group: *mut mailimf_group,
) -> libc::c_int {
    let mut r: libc::c_int = 0;
    r = mailimf_header_string_write_driver(
        do_write,
        data,
        col,
        (*group).grp_display_name,
        strlen((*group).grp_display_name),
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    r = mailimf_string_write_driver(
        do_write,
        data,
        col,
        b": \x00" as *const u8 as *const libc::c_char,
        2i32 as size_t,
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    if !(*group).grp_mb_list.is_null() {
        r = mailimf_mailbox_list_write_driver(do_write, data, col, (*group).grp_mb_list);
        if r != MAILIMF_NO_ERROR as libc::c_int {
            return r;
        }
    }
    r = mailimf_string_write_driver(
        do_write,
        data,
        col,
        b";\x00" as *const u8 as *const libc::c_char,
        1i32 as size_t,
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    return MAILIMF_NO_ERROR as libc::c_int;
}

pub unsafe fn mailimf_mailbox_list_write_driver(
    mut do_write: Option<
        unsafe fn(_: *mut libc::c_void, _: *const libc::c_char, _: size_t) -> libc::c_int,
    >,
    mut data: *mut libc::c_void,
    mut col: *mut libc::c_int,
    mut mb_list: *mut mailimf_mailbox_list,
) -> libc::c_int {
    let mut cur: *mut clistiter = 0 as *mut clistiter;
    let mut r: libc::c_int = 0;
    let mut first: libc::c_int = 0;
    first = 1i32;
    cur = (*(*mb_list).mb_list).first;
    while !cur.is_null() {
        let mut mb: *mut mailimf_mailbox = 0 as *mut mailimf_mailbox;
        mb = (if !cur.is_null() {
            (*cur).data
        } else {
            0 as *mut libc::c_void
        }) as *mut mailimf_mailbox;
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
        r = mailimf_mailbox_write_driver(do_write, data, col, mb);
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
unsafe fn mailimf_mailbox_write_driver(
    mut do_write: Option<
        unsafe fn(_: *mut libc::c_void, _: *const libc::c_char, _: size_t) -> libc::c_int,
    >,
    mut data: *mut libc::c_void,
    mut col: *mut libc::c_int,
    mut mb: *mut mailimf_mailbox,
) -> libc::c_int {
    let mut r: libc::c_int = 0;
    let mut do_fold: libc::c_int = 0;
    if !(*mb).mb_display_name.is_null() {
        if 0 != is_atext((*mb).mb_display_name) {
            r = mailimf_header_string_write_driver(
                do_write,
                data,
                col,
                (*mb).mb_display_name,
                strlen((*mb).mb_display_name),
            );
            if r != MAILIMF_NO_ERROR as libc::c_int {
                return r;
            }
        } else {
            if !(*mb).mb_display_name.is_null() {
                if (*col as libc::size_t).wrapping_add(strlen((*mb).mb_display_name))
                    >= 72i32 as libc::size_t
                {
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
            if strlen((*mb).mb_display_name) > (998i32 / 2i32) as libc::size_t {
                return MAILIMF_ERROR_INVAL as libc::c_int;
            }
            r = mailimf_quoted_string_write_driver(
                do_write,
                data,
                col,
                (*mb).mb_display_name,
                strlen((*mb).mb_display_name),
            );
            if r != MAILIMF_NO_ERROR as libc::c_int {
                return r;
            }
        }
        do_fold = 0i32;
        if *col > 1i32 {
            if (*col as libc::size_t)
                .wrapping_add(strlen((*mb).mb_addr_spec))
                .wrapping_add(3i32 as libc::size_t)
                >= 72i32 as libc::size_t
            {
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
                do_fold = 1i32
            }
        }
        if 0 != do_fold {
            r = mailimf_string_write_driver(
                do_write,
                data,
                col,
                b"<\x00" as *const u8 as *const libc::c_char,
                1i32 as size_t,
            )
        } else {
            r = mailimf_string_write_driver(
                do_write,
                data,
                col,
                b" <\x00" as *const u8 as *const libc::c_char,
                2i32 as size_t,
            )
        }
        if r != MAILIMF_NO_ERROR as libc::c_int {
            return r;
        }
        r = mailimf_string_write_driver(
            do_write,
            data,
            col,
            (*mb).mb_addr_spec,
            strlen((*mb).mb_addr_spec),
        );
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
    } else {
        if (*col as libc::size_t).wrapping_add(strlen((*mb).mb_addr_spec)) >= 72i32 as libc::size_t
        {
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
        r = mailimf_string_write_driver(
            do_write,
            data,
            col,
            (*mb).mb_addr_spec,
            strlen((*mb).mb_addr_spec),
        );
        if r != MAILIMF_NO_ERROR as libc::c_int {
            return r;
        }
    }
    return MAILIMF_NO_ERROR as libc::c_int;
}
/*
  mailimf_quoted_string_write writes a string that is quoted
  to a given stream

  @param f is the stream
  @param col (* col) is the column number where we will start to
    write the text, the ending column will be stored in (* col)
  @param string is the string to quote and write
*/
pub unsafe fn mailimf_quoted_string_write_driver(
    mut do_write: Option<
        unsafe fn(_: *mut libc::c_void, _: *const libc::c_char, _: size_t) -> libc::c_int,
    >,
    mut data: *mut libc::c_void,
    mut col: *mut libc::c_int,
    mut string: *const libc::c_char,
    mut len: size_t,
) -> libc::c_int {
    let mut r: libc::c_int = 0;
    let mut i: size_t = 0;
    r = do_write.expect("non-null function pointer")(
        data,
        b"\"\x00" as *const u8 as *const libc::c_char,
        1i32 as size_t,
    );
    if r == 0i32 {
        return MAILIMF_ERROR_FILE as libc::c_int;
    }
    i = 0i32 as size_t;
    while i < len {
        match *string.offset(i as isize) as libc::c_int {
            92 | 34 => {
                r = do_write.expect("non-null function pointer")(
                    data,
                    b"\\\x00" as *const u8 as *const libc::c_char,
                    1i32 as size_t,
                );
                if r == 0i32 {
                    return MAILIMF_ERROR_FILE as libc::c_int;
                }
                r = do_write.expect("non-null function pointer")(
                    data,
                    &*string.offset(i as isize),
                    1i32 as size_t,
                );
                if r == 0i32 {
                    return MAILIMF_ERROR_FILE as libc::c_int;
                }
                *col += 2i32
            }
            _ => {
                r = do_write.expect("non-null function pointer")(
                    data,
                    &*string.offset(i as isize),
                    1i32 as size_t,
                );
                if r == 0i32 {
                    return MAILIMF_ERROR_FILE as libc::c_int;
                }
                *col += 1
            }
        }
        i = i.wrapping_add(1)
    }
    r = do_write.expect("non-null function pointer")(
        data,
        b"\"\x00" as *const u8 as *const libc::c_char,
        1i32 as size_t,
    );
    if r == 0i32 {
        return MAILIMF_ERROR_FILE as libc::c_int;
    }
    return MAILIMF_NO_ERROR as libc::c_int;
}
/*
static int
atext           =       ALPHA / DIGIT / ; Any character except controls,
                        "!" / "#" /     ;  SP, and specials.
                        "$" / "%" /     ;  Used for atoms
                        "&" / "'" /
                        "*" / "+" /
                        "-" / "/" /
                        "=" / "?" /
                        "^" / "_" /
                        "`" / "{" /
                        "|" / "}" /
                        "~"
*/
unsafe fn is_atext(mut s: *const libc::c_char) -> libc::c_int {
    let mut p: *const libc::c_char = 0 as *const libc::c_char;
    p = s;
    while *p as libc::c_int != 0i32 {
        if !(0 != isalpha(*p as libc::c_uchar as libc::c_int)) {
            if !(0 != isdigit(*p as libc::c_uchar as libc::c_int)) {
                match *p as libc::c_int {
                    32 | 9 | 33 | 35 | 36 | 37 | 38 | 39 | 42 | 43 | 45 | 47 | 61 | 63 | 94
                    | 95 | 96 | 123 | 124 | 125 | 126 => {}
                    _ => return 0i32,
                }
            }
        }
        p = p.offset(1isize)
    }
    return 1i32;
}

unsafe fn mailimf_cc_write_driver(
    mut do_write: Option<
        unsafe fn(_: *mut libc::c_void, _: *const libc::c_char, _: size_t) -> libc::c_int,
    >,
    mut data: *mut libc::c_void,
    mut col: *mut libc::c_int,
    mut cc: *mut mailimf_cc,
) -> libc::c_int {
    let mut r: libc::c_int = 0;
    r = mailimf_string_write_driver(
        do_write,
        data,
        col,
        b"Cc: \x00" as *const u8 as *const libc::c_char,
        4i32 as size_t,
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    r = mailimf_address_list_write_driver(do_write, data, col, (*cc).cc_addr_list);
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
unsafe fn mailimf_to_write_driver(
    mut do_write: Option<
        unsafe fn(_: *mut libc::c_void, _: *const libc::c_char, _: size_t) -> libc::c_int,
    >,
    mut data: *mut libc::c_void,
    mut col: *mut libc::c_int,
    mut to: *mut mailimf_to,
) -> libc::c_int {
    let mut r: libc::c_int = 0;
    r = mailimf_string_write_driver(
        do_write,
        data,
        col,
        b"To: \x00" as *const u8 as *const libc::c_char,
        4i32 as size_t,
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    r = mailimf_address_list_write_driver(do_write, data, col, (*to).to_addr_list);
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
unsafe fn mailimf_reply_to_write_driver(
    mut do_write: Option<
        unsafe fn(_: *mut libc::c_void, _: *const libc::c_char, _: size_t) -> libc::c_int,
    >,
    mut data: *mut libc::c_void,
    mut col: *mut libc::c_int,
    mut reply_to: *mut mailimf_reply_to,
) -> libc::c_int {
    let mut r: libc::c_int = 0;
    r = mailimf_string_write_driver(
        do_write,
        data,
        col,
        b"Reply-To: \x00" as *const u8 as *const libc::c_char,
        10i32 as size_t,
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    r = mailimf_address_list_write_driver(do_write, data, col, (*reply_to).rt_addr_list);
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
unsafe fn mailimf_sender_write_driver(
    mut do_write: Option<
        unsafe fn(_: *mut libc::c_void, _: *const libc::c_char, _: size_t) -> libc::c_int,
    >,
    mut data: *mut libc::c_void,
    mut col: *mut libc::c_int,
    mut sender: *mut mailimf_sender,
) -> libc::c_int {
    let mut r: libc::c_int = 0;
    r = mailimf_string_write_driver(
        do_write,
        data,
        col,
        b"Sender: \x00" as *const u8 as *const libc::c_char,
        8i32 as size_t,
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    r = mailimf_mailbox_write_driver(do_write, data, col, (*sender).snd_mb);
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
unsafe fn mailimf_from_write_driver(
    mut do_write: Option<
        unsafe fn(_: *mut libc::c_void, _: *const libc::c_char, _: size_t) -> libc::c_int,
    >,
    mut data: *mut libc::c_void,
    mut col: *mut libc::c_int,
    mut from: *mut mailimf_from,
) -> libc::c_int {
    let mut r: libc::c_int = 0;
    r = mailimf_string_write_driver(
        do_write,
        data,
        col,
        b"From: \x00" as *const u8 as *const libc::c_char,
        6i32 as size_t,
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    r = mailimf_mailbox_list_write_driver(do_write, data, col, (*from).frm_mb_list);
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
 * $Id: mailimf_write_generic.c,v 1.3 2006/05/22 13:39:42 hoa Exp $
 */
unsafe fn mailimf_orig_date_write_driver(
    mut do_write: Option<
        unsafe fn(_: *mut libc::c_void, _: *const libc::c_char, _: size_t) -> libc::c_int,
    >,
    mut data: *mut libc::c_void,
    mut col: *mut libc::c_int,
    mut date: *mut mailimf_orig_date,
) -> libc::c_int {
    let mut r: libc::c_int = 0;
    r = mailimf_string_write_driver(
        do_write,
        data,
        col,
        b"Date: \x00" as *const u8 as *const libc::c_char,
        6i32 as size_t,
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    r = mailimf_date_time_write_driver(do_write, data, col, (*date).dt_date_time);
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
unsafe fn mailimf_date_time_write_driver(
    mut do_write: Option<
        unsafe fn(_: *mut libc::c_void, _: *const libc::c_char, _: size_t) -> libc::c_int,
    >,
    mut data: *mut libc::c_void,
    mut col: *mut libc::c_int,
    mut date_time: *mut mailimf_date_time,
) -> libc::c_int {
    let wday = dayofweek(
        (*date_time).dt_year,
        (*date_time).dt_month,
        (*date_time).dt_day,
    );

    let date_str = format!(
        "{}, {} {} {} {:02}:{:02}:{:02} {:+05}",
        week_of_day_str[wday as usize],
        (*date_time).dt_day,
        month_str[((*date_time).dt_month - 1i32) as usize],
        (*date_time).dt_year,
        (*date_time).dt_hour,
        (*date_time).dt_min,
        (*date_time).dt_sec,
        (*date_time).dt_zone,
    );
    let date_str_c = std::ffi::CString::new(date_str).unwrap();
    let r = mailimf_string_write_driver(
        do_write,
        data,
        col,
        date_str_c.as_ptr() as *mut _,
        strlen(date_str_c.as_ptr()),
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    return MAILIMF_NO_ERROR as libc::c_int;
}
static mut month_str: [&'static str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];
static mut week_of_day_str: [&'static str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
/* 0 = Sunday */
/* y > 1752 */
unsafe fn dayofweek(
    mut year: libc::c_int,
    mut month: libc::c_int,
    mut day: libc::c_int,
) -> libc::c_int {
    static mut offset: [libc::c_int; 12] = [
        0i32, 3i32, 2i32, 5i32, 0i32, 3i32, 5i32, 1i32, 4i32, 6i32, 2i32, 4i32,
    ];
    year -= (month < 3i32) as libc::c_int;
    return (year + year / 4i32 - year / 100i32
        + year / 400i32
        + offset[(month - 1i32) as usize]
        + day)
        % 7i32;
}
unsafe fn mailimf_resent_msg_id_write_driver(
    mut do_write: Option<
        unsafe fn(_: *mut libc::c_void, _: *const libc::c_char, _: size_t) -> libc::c_int,
    >,
    mut data: *mut libc::c_void,
    mut col: *mut libc::c_int,
    mut message_id: *mut mailimf_message_id,
) -> libc::c_int {
    let mut r: libc::c_int = 0;
    r = mailimf_string_write_driver(
        do_write,
        data,
        col,
        b"Resent-Message-ID: \x00" as *const u8 as *const libc::c_char,
        19i32 as size_t,
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
    r = mailimf_string_write_driver(
        do_write,
        data,
        col,
        (*message_id).mid_value,
        strlen((*message_id).mid_value),
    );
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
unsafe fn mailimf_resent_bcc_write_driver(
    mut do_write: Option<
        unsafe fn(_: *mut libc::c_void, _: *const libc::c_char, _: size_t) -> libc::c_int,
    >,
    mut data: *mut libc::c_void,
    mut col: *mut libc::c_int,
    mut bcc: *mut mailimf_bcc,
) -> libc::c_int {
    let mut r: libc::c_int = 0;
    r = mailimf_string_write_driver(
        do_write,
        data,
        col,
        b"Resent-Bcc: \x00" as *const u8 as *const libc::c_char,
        12i32 as size_t,
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    if !(*bcc).bcc_addr_list.is_null() {
        r = mailimf_address_list_write_driver(do_write, data, col, (*bcc).bcc_addr_list);
        if r != MAILIMF_NO_ERROR as libc::c_int {
            return r;
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
unsafe fn mailimf_resent_cc_write_driver(
    mut do_write: Option<
        unsafe fn(_: *mut libc::c_void, _: *const libc::c_char, _: size_t) -> libc::c_int,
    >,
    mut data: *mut libc::c_void,
    mut col: *mut libc::c_int,
    mut cc: *mut mailimf_cc,
) -> libc::c_int {
    let mut r: libc::c_int = 0;
    r = mailimf_string_write_driver(
        do_write,
        data,
        col,
        b"Resent-Cc: \x00" as *const u8 as *const libc::c_char,
        11i32 as size_t,
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    r = mailimf_address_list_write_driver(do_write, data, col, (*cc).cc_addr_list);
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
unsafe fn mailimf_resent_to_write_driver(
    mut do_write: Option<
        unsafe fn(_: *mut libc::c_void, _: *const libc::c_char, _: size_t) -> libc::c_int,
    >,
    mut data: *mut libc::c_void,
    mut col: *mut libc::c_int,
    mut to: *mut mailimf_to,
) -> libc::c_int {
    let mut r: libc::c_int = 0;
    r = mailimf_string_write_driver(
        do_write,
        data,
        col,
        b"Resent-To: \x00" as *const u8 as *const libc::c_char,
        11i32 as size_t,
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    r = mailimf_address_list_write_driver(do_write, data, col, (*to).to_addr_list);
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
unsafe fn mailimf_resent_sender_write_driver(
    mut do_write: Option<
        unsafe fn(_: *mut libc::c_void, _: *const libc::c_char, _: size_t) -> libc::c_int,
    >,
    mut data: *mut libc::c_void,
    mut col: *mut libc::c_int,
    mut sender: *mut mailimf_sender,
) -> libc::c_int {
    let mut r: libc::c_int = 0;
    r = mailimf_string_write_driver(
        do_write,
        data,
        col,
        b"Resent-Sender: \x00" as *const u8 as *const libc::c_char,
        15i32 as size_t,
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    r = mailimf_mailbox_write_driver(do_write, data, col, (*sender).snd_mb);
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
unsafe fn mailimf_resent_from_write_driver(
    mut do_write: Option<
        unsafe fn(_: *mut libc::c_void, _: *const libc::c_char, _: size_t) -> libc::c_int,
    >,
    mut data: *mut libc::c_void,
    mut col: *mut libc::c_int,
    mut from: *mut mailimf_from,
) -> libc::c_int {
    let mut r: libc::c_int = 0;
    r = mailimf_string_write_driver(
        do_write,
        data,
        col,
        b"Resent-From: \x00" as *const u8 as *const libc::c_char,
        13i32 as size_t,
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    r = mailimf_mailbox_list_write_driver(do_write, data, col, (*from).frm_mb_list);
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
unsafe fn mailimf_resent_date_write_driver(
    mut do_write: Option<
        unsafe fn(_: *mut libc::c_void, _: *const libc::c_char, _: size_t) -> libc::c_int,
    >,
    mut data: *mut libc::c_void,
    mut col: *mut libc::c_int,
    mut date: *mut mailimf_orig_date,
) -> libc::c_int {
    let mut r: libc::c_int = 0;
    r = mailimf_string_write_driver(
        do_write,
        data,
        col,
        b"Resent-Date: \x00" as *const u8 as *const libc::c_char,
        13i32 as size_t,
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    r = mailimf_date_time_write_driver(do_write, data, col, (*date).dt_date_time);
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
unsafe fn mailimf_return_write_driver(
    mut do_write: Option<
        unsafe fn(_: *mut libc::c_void, _: *const libc::c_char, _: size_t) -> libc::c_int,
    >,
    mut data: *mut libc::c_void,
    mut col: *mut libc::c_int,
    mut return_path: *mut mailimf_return,
) -> libc::c_int {
    let mut r: libc::c_int = 0;
    r = mailimf_string_write_driver(
        do_write,
        data,
        col,
        b"Return-Path: \x00" as *const u8 as *const libc::c_char,
        13i32 as size_t,
    );
    if r != MAILIMF_NO_ERROR as libc::c_int {
        return r;
    }
    r = mailimf_path_write_driver(do_write, data, col, (*return_path).ret_path);
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
unsafe fn mailimf_path_write_driver(
    mut do_write: Option<
        unsafe fn(_: *mut libc::c_void, _: *const libc::c_char, _: size_t) -> libc::c_int,
    >,
    mut data: *mut libc::c_void,
    mut col: *mut libc::c_int,
    mut path: *mut mailimf_path,
) -> libc::c_int {
    let mut r: libc::c_int = 0;
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
    if !(*path).pt_addr_spec.is_null() {
        r = mailimf_string_write_driver(
            do_write,
            data,
            col,
            (*path).pt_addr_spec,
            strlen((*path).pt_addr_spec),
        );
        if r != MAILIMF_NO_ERROR as libc::c_int {
            return r;
        }
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
    return MAILIMF_NO_ERROR as libc::c_int;
}
/*
  mailimf_envelope_fields_write writes only some fields to a given stream

  @param f is the stream
  @param col (* col) is the column number where we will start to
    write the text, the ending column will be stored in (* col)
  @param fields is the fields to write
*/
pub unsafe fn mailimf_envelope_fields_write_driver(
    mut do_write: Option<
        unsafe fn(_: *mut libc::c_void, _: *const libc::c_char, _: size_t) -> libc::c_int,
    >,
    mut data: *mut libc::c_void,
    mut col: *mut libc::c_int,
    mut fields: *mut mailimf_fields,
) -> libc::c_int {
    let mut cur: *mut clistiter = 0 as *mut clistiter;
    cur = (*(*fields).fld_list).first;
    while !cur.is_null() {
        let mut r: libc::c_int = 0;
        let mut field: *mut mailimf_field = 0 as *mut mailimf_field;
        field = (if !cur.is_null() {
            (*cur).data
        } else {
            0 as *mut libc::c_void
        }) as *mut mailimf_field;
        if (*field).fld_type != MAILIMF_FIELD_OPTIONAL_FIELD as libc::c_int {
            r = mailimf_field_write_driver(do_write, data, col, field);
            if r != MAILIMF_NO_ERROR as libc::c_int {
                return r;
            }
        }
        cur = if !cur.is_null() {
            (*cur).next
        } else {
            0 as *mut clistcell
        }
    }
    return MAILIMF_NO_ERROR as libc::c_int;
}
