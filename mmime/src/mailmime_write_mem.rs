use crate::mailmime_types::*;
use crate::mailmime_write_generic::*;
use crate::mmapstring::*;
use crate::other::*;

unsafe fn do_write(
    mut data: *mut libc::c_void,
    mut str: *const libc::c_char,
    mut length: size_t,
) -> libc::c_int {
    let mut f: *mut MMAPString = 0 as *mut MMAPString;
    f = data as *mut MMAPString;
    if mmap_string_append_len(f, str, length).is_null() {
        return 0i32;
    } else {
        return length as libc::c_int;
    };
}

pub unsafe fn mailmime_content_write_mem(
    mut f: *mut MMAPString,
    mut col: *mut libc::c_int,
    mut content: *mut mailmime_content,
) -> libc::c_int {
    return mailmime_content_write_driver(Some(do_write), f as *mut libc::c_void, col, content);
}

pub unsafe fn mailmime_content_type_write_mem(
    mut f: *mut MMAPString,
    mut col: *mut libc::c_int,
    mut content: *mut mailmime_content,
) -> libc::c_int {
    return mailmime_content_type_write_driver(
        Some(do_write),
        f as *mut libc::c_void,
        col,
        content,
    );
}

pub unsafe fn mailmime_write_mem(
    mut f: *mut MMAPString,
    mut col: *mut libc::c_int,
    mut build_info: *mut mailmime,
) -> libc::c_int {
    return mailmime_write_driver(Some(do_write), f as *mut libc::c_void, col, build_info);
}

pub unsafe fn mailmime_quoted_printable_write_mem(
    mut f: *mut MMAPString,
    mut col: *mut libc::c_int,
    mut istext: libc::c_int,
    mut text: *const libc::c_char,
    mut size: size_t,
) -> libc::c_int {
    return mailmime_quoted_printable_write_driver(
        Some(do_write),
        f as *mut libc::c_void,
        col,
        istext,
        text,
        size,
    );
}

pub unsafe fn mailmime_base64_write_mem(
    mut f: *mut MMAPString,
    mut col: *mut libc::c_int,
    mut text: *const libc::c_char,
    mut size: size_t,
) -> libc::c_int {
    return mailmime_base64_write_driver(Some(do_write), f as *mut libc::c_void, col, text, size);
}

pub unsafe fn mailmime_data_write_mem(
    mut f: *mut MMAPString,
    mut col: *mut libc::c_int,
    mut data: *mut mailmime_data,
    mut istext: libc::c_int,
) -> libc::c_int {
    return mailmime_data_write_driver(Some(do_write), f as *mut libc::c_void, col, data, istext);
}
