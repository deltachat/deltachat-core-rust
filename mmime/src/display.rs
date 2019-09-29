use crate::clist::*;

use crate::mailimf::types::*;
use crate::mailmime::types::*;

use std::ffi::CStr;

pub unsafe fn display_mime(mut mime: *mut Mailmime) {
    let mut cur: *mut clistiter = 0 as *mut clistiter;
    println!("{}", (*mime).mm_type);

    match (*mime).mm_type as u32 {
        MAILMIME_SINGLE => {
            println!("single part");
        }
        MAILMIME_MULTIPLE => {
            println!("multipart");
        }
        MAILMIME_MESSAGE => println!("message"),
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
    match (*mime).mm_type as u32 {
        MAILMIME_SINGLE => {
            display_mime_data((*mime).mm_data.mm_single);
        }
        MAILMIME_MULTIPLE => {
            cur = (*(*mime).mm_data.mm_multipart.mm_mp_list).first;
            while !cur.is_null() {
                display_mime(
                    (if !cur.is_null() {
                        (*cur).data
                    } else {
                        0 as *mut libc::c_void
                    }) as *mut Mailmime,
                );
                cur = if !cur.is_null() {
                    (*cur).next
                } else {
                    0 as *mut clistcell
                }
            }
        }
        MAILMIME_MESSAGE => {
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
pub unsafe fn display_mime_data(mut data: *mut mailmime_data) {
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
