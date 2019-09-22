#![allow(unused_attributes)]
#![allow(unused_variables)]
#![allow(mutable_transmutes)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(unused_assignments)]
#![allow(unused_mut)]
#![allow(unused_must_use)]
#![feature(extern_types)]
#![feature(const_raw_ptr_to_usize_cast)]
#![feature(ptr_wrapping_offset_from)]

pub mod charconv;
pub mod chash;
pub mod clist;
pub mod mailimf;
pub mod mailimf_types;
pub mod mailimf_types_helper;
pub mod mailimf_write_generic;
pub mod mailmime;
pub mod mailmime_content;
pub mod mailmime_decode;
pub mod mailmime_disposition;
pub mod mailmime_types;
pub mod mailmime_types_helper;
pub mod mailmime_write_generic;
pub mod mailmime_write_mem;
pub mod mmapstring;
pub mod other;

pub use self::charconv::*;
pub use self::chash::*;
pub use self::clist::*;
pub use self::mailimf::*;
pub use self::mailimf_types::*;
pub use self::mailimf_types_helper::*;
pub use self::mailimf_write_generic::*;
pub use self::mailmime::*;
pub use self::mailmime_content::*;
pub use self::mailmime_decode::*;
pub use self::mailmime_disposition::*;
pub use self::mailmime_types::*;
pub use self::mailmime_types_helper::*;
pub use self::mailmime_write_generic::*;
pub use self::mailmime_write_mem::*;
pub use self::mmapstring::*;
pub use self::other::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mailmime_types::{mailmime, mailmime_content, mailmime_disposition};
    use std::ffi::CStr;

    #[test]
    fn mailmime_parse_test() {
        unsafe {
            let data = "MIME-Version: 1.0\
                        Content-Type: multipart/mixed; boundary=frontier\
                        \
                        This is a message with multiple parts in MIME format.\
                        --frontier\
                        Content-Type: text/plain\
                        \
                        This is the body of the message.\
                        --frontier\
                        Content-Type: application/octet-stream\
                        Content-Transfer-Encoding: base64\
                        \
                        PGh0bWw+CiAgPGhlYWQ+CiAgPC9oZWFkPgogIDxib2R5PgogICAgPHA+VGhpcyBpcyB0aGUg\
                        Ym9keSBvZiB0aGUgbWVzc2FnZS48L3A+CiAgPC9ib2R5Pgo8L2h0bWw+Cg==\
                        --frontier--";
            let c_data = std::ffi::CString::new(data).unwrap();

            let mut current_index = 0;
            let mut mime = std::ptr::null_mut();
            let res = crate::mailmime_content::mailmime_parse(
                c_data.as_ptr(),
                data.len() as usize,
                &mut current_index,
                &mut mime,
            );

            assert_eq!(res, MAIL_NO_ERROR as libc::c_int);
            assert!(!mime.is_null());

            display_mime(mime);

            mailmime_types::mailmime_free(mime);
        }
    }

    unsafe fn display_mime(mut mime: *mut mailmime) {
        let mut cur: *mut clistiter = 0 as *mut clistiter;
        println!("{}", (*mime).mm_type);

        match (*mime).mm_type {
            1 => {
                println!("single part");
            }
            2 => {
                println!("multipart");
            }
            3 => println!("message"),
            _ => {}
        }
        if !(*mime).mm_mime_fields.is_null() {
            if !(*(*(*mime).mm_mime_fields).fld_list).first.is_null() {
                print!("MIME headers begin");
                display_mime_fields((*mime).mm_mime_fields);
                println!("MIME headers end");
            }
        }
        display_mime_content((*mime).mm_content_type);
        match (*mime).mm_type {
            1 => {
                display_mime_data((*mime).mm_data.mm_single);
            }
            2 => {
                cur = (*(*mime).mm_data.mm_multipart.mm_mp_list).first;
                while !cur.is_null() {
                    display_mime(
                        (if !cur.is_null() {
                            (*cur).data
                        } else {
                            0 as *mut libc::c_void
                        }) as *mut mailmime,
                    );
                    cur = if !cur.is_null() {
                        (*cur).next
                    } else {
                        0 as *mut clistcell
                    }
                }
            }
            3 => {
                if !(*mime).mm_data.mm_message.mm_fields.is_null() {
                    if !(*(*(*mime).mm_data.mm_message.mm_fields).fld_list)
                        .first
                        .is_null()
                    {
                        println!("headers begin");
                        display_fields((*mime).mm_data.mm_message.mm_fields);
                        println!("headers end");
                    }
                    if !(*mime).mm_data.mm_message.mm_msg_mime.is_null() {
                        display_mime((*mime).mm_data.mm_message.mm_msg_mime);
                    }
                }
            }
            _ => {}
        };
    }

    unsafe fn display_mime_content(mut content_type: *mut mailmime_content) {
        print!("type: ");
        display_mime_type((*content_type).ct_type);
        println!(
            "/{}",
            CStr::from_ptr((*content_type).ct_subtype).to_str().unwrap()
        );
    }
    unsafe fn display_mime_type(mut type_0: *mut mailmime_type) {
        match (*type_0).tp_type {
            1 => {
                display_mime_discrete_type((*type_0).tp_data.tp_discrete_type);
            }
            2 => {
                display_mime_composite_type((*type_0).tp_data.tp_composite_type);
            }
            _ => {}
        };
    }
    unsafe fn display_mime_composite_type(mut ct: *mut mailmime_composite_type) {
        match (*ct).ct_type {
            1 => {
                print!("message");
            }
            2 => {
                print!("multipart");
            }
            3 => {
                print!("{}", CStr::from_ptr((*ct).ct_token).to_str().unwrap());
            }
            _ => {}
        };
    }
    unsafe fn display_mime_discrete_type(mut discrete_type: *mut mailmime_discrete_type) {
        match (*discrete_type).dt_type {
            1 => {
                print!("text");
            }
            2 => {
                print!("image");
            }
            3 => {
                print!("audio");
            }
            4 => {
                print!("video");
            }
            5 => {
                print!("application");
            }
            6 => {
                print!("{}", (*discrete_type).dt_extension as u8 as char);
            }
            _ => {}
        };
    }
    unsafe fn display_mime_data(mut data: *mut mailmime_data) {
        match (*data).dt_type {
            0 => {
                println!(
                    "data : {} bytes",
                    (*data).dt_data.dt_text.dt_length as libc::c_uint,
                );
            }
            1 => {
                println!(
                    "data (file) : {}",
                    CStr::from_ptr((*data).dt_data.dt_filename)
                        .to_str()
                        .unwrap()
                );
            }
            _ => {}
        };
    }
    unsafe fn display_mime_dsp_parm(mut param: *mut mailmime_disposition_parm) {
        match (*param).pa_type {
            0 => {
                println!(
                    "filename: {}",
                    CStr::from_ptr((*param).pa_data.pa_filename)
                        .to_str()
                        .unwrap()
                );
            }
            _ => {}
        };
    }
    unsafe fn display_mime_disposition(mut disposition: *mut mailmime_disposition) {
        let mut cur: *mut clistiter = 0 as *mut clistiter;
        cur = (*(*disposition).dsp_parms).first;
        while !cur.is_null() {
            let mut param: *mut mailmime_disposition_parm = 0 as *mut mailmime_disposition_parm;
            param = (if !cur.is_null() {
                (*cur).data
            } else {
                0 as *mut libc::c_void
            }) as *mut mailmime_disposition_parm;
            display_mime_dsp_parm(param);
            cur = if !cur.is_null() {
                (*cur).next
            } else {
                0 as *mut clistcell
            }
        }
    }
    unsafe fn display_mime_field(mut field: *mut mailmime_field) {
        match (*field).fld_type {
            1 => {
                print!("content-type: ");
                display_mime_content((*field).fld_data.fld_content);
                println!("");
            }
            6 => {
                display_mime_disposition((*field).fld_data.fld_disposition);
            }
            _ => {}
        };
    }
    unsafe fn display_mime_fields(mut fields: *mut mailmime_fields) {
        let mut cur: *mut clistiter = 0 as *mut clistiter;
        cur = (*(*fields).fld_list).first;
        while !cur.is_null() {
            let mut field: *mut mailmime_field = 0 as *mut mailmime_field;
            field = (if !cur.is_null() {
                (*cur).data
            } else {
                0 as *mut libc::c_void
            }) as *mut mailmime_field;
            display_mime_field(field);
            cur = if !cur.is_null() {
                (*cur).next
            } else {
                0 as *mut clistcell
            }
        }
    }
    unsafe fn display_date_time(mut d: *mut mailimf_date_time) {
        print!(
            "{:02}/{:02}/{:02} {:02}:{:02}:{:02} +{:04}",
            (*d).dt_day,
            (*d).dt_month,
            (*d).dt_year,
            (*d).dt_hour,
            (*d).dt_min,
            (*d).dt_sec,
            (*d).dt_zone,
        );
    }
    unsafe fn display_orig_date(mut orig_date: *mut mailimf_orig_date) {
        display_date_time((*orig_date).dt_date_time);
    }
    unsafe fn display_mailbox(mut mb: *mut mailimf_mailbox) {
        if !(*mb).mb_display_name.is_null() {
            print!(
                "{}",
                CStr::from_ptr((*mb).mb_display_name).to_str().unwrap()
            );
        }
        print!("<{}>", CStr::from_ptr((*mb).mb_addr_spec).to_str().unwrap());
    }
    unsafe fn display_mailbox_list(mut mb_list: *mut mailimf_mailbox_list) {
        let mut cur: *mut clistiter = 0 as *mut clistiter;
        cur = (*(*mb_list).mb_list).first;
        while !cur.is_null() {
            let mut mb: *mut mailimf_mailbox = 0 as *mut mailimf_mailbox;
            mb = (if !cur.is_null() {
                (*cur).data
            } else {
                0 as *mut libc::c_void
            }) as *mut mailimf_mailbox;
            display_mailbox(mb);
            if !if !cur.is_null() {
                (*cur).next
            } else {
                0 as *mut clistcell
            }
            .is_null()
            {
                print!(", ");
            }
            cur = if !cur.is_null() {
                (*cur).next
            } else {
                0 as *mut clistcell
            }
        }
    }
    unsafe fn display_group(mut group: *mut mailimf_group) {
        let mut cur: *mut clistiter = 0 as *mut clistiter;
        print!(
            "{}: ",
            CStr::from_ptr((*group).grp_display_name).to_str().unwrap()
        );
        cur = (*(*(*group).grp_mb_list).mb_list).first;
        while !cur.is_null() {
            let mut mb: *mut mailimf_mailbox = 0 as *mut mailimf_mailbox;
            mb = (if !cur.is_null() {
                (*cur).data
            } else {
                0 as *mut libc::c_void
            }) as *mut mailimf_mailbox;
            display_mailbox(mb);
            cur = if !cur.is_null() {
                (*cur).next
            } else {
                0 as *mut clistcell
            }
        }
        print!("; ");
    }
    unsafe fn display_address(mut a: *mut mailimf_address) {
        match (*a).ad_type {
            2 => {
                display_group((*a).ad_data.ad_group);
            }
            1 => {
                display_mailbox((*a).ad_data.ad_mailbox);
            }
            _ => {}
        };
    }
    unsafe fn display_address_list(mut addr_list: *mut mailimf_address_list) {
        let mut cur: *mut clistiter = 0 as *mut clistiter;
        cur = (*(*addr_list).ad_list).first;
        while !cur.is_null() {
            let mut addr: *mut mailimf_address = 0 as *mut mailimf_address;
            addr = (if !cur.is_null() {
                (*cur).data
            } else {
                0 as *mut libc::c_void
            }) as *mut mailimf_address;
            display_address(addr);
            if !if !cur.is_null() {
                (*cur).next
            } else {
                0 as *mut clistcell
            }
            .is_null()
            {
                print!(", ");
            }
            cur = if !cur.is_null() {
                (*cur).next
            } else {
                0 as *mut clistcell
            }
        }
    }
    unsafe fn display_from(mut from: *mut mailimf_from) {
        display_mailbox_list((*from).frm_mb_list);
    }
    unsafe fn display_to(mut to: *mut mailimf_to) {
        display_address_list((*to).to_addr_list);
    }
    unsafe fn display_cc(mut cc: *mut mailimf_cc) {
        display_address_list((*cc).cc_addr_list);
    }
    unsafe fn display_subject(mut subject: *mut mailimf_subject) {
        print!("{}", CStr::from_ptr((*subject).sbj_value).to_str().unwrap());
    }
    unsafe fn display_field(mut field: *mut mailimf_field) {
        match (*field).fld_type {
            9 => {
                print!("Date: ");
                display_orig_date((*field).fld_data.fld_orig_date);
                println!("");
            }
            10 => {
                print!("From: ");
                display_from((*field).fld_data.fld_from);
                println!("");
            }
            13 => {
                print!("To: ");
                display_to((*field).fld_data.fld_to);
                println!("");
            }
            14 => {
                print!("Cc: ");
                display_cc((*field).fld_data.fld_cc);
                println!("");
            }
            19 => {
                print!("Subject: ");
                display_subject((*field).fld_data.fld_subject);
                println!("");
            }
            16 => {
                println!(
                    "Message-ID: {}",
                    CStr::from_ptr((*(*field).fld_data.fld_message_id).mid_value)
                        .to_str()
                        .unwrap(),
                );
            }
            _ => {}
        };
    }
    unsafe fn display_fields(mut fields: *mut mailimf_fields) {
        let mut cur: *mut clistiter = 0 as *mut clistiter;
        cur = (*(*fields).fld_list).first;
        while !cur.is_null() {
            let mut f: *mut mailimf_field = 0 as *mut mailimf_field;
            f = (if !cur.is_null() {
                (*cur).data
            } else {
                0 as *mut libc::c_void
            }) as *mut mailimf_field;
            display_field(f);
            cur = if !cur.is_null() {
                (*cur).next
            } else {
                0 as *mut clistcell
            }
        }
    }
}
