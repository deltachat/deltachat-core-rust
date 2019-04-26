#![allow(
    unused_imports,
    dead_code,
    mutable_transmutes,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    unused_assignments,
    unused_mut,
    unused_attributes,
    non_upper_case_globals,
    non_camel_case_types,
    non_snake_case
)]
#![feature(
    c_variadic,
    const_raw_ptr_to_usize_cast,
    extern_types,
    ptr_wrapping_offset_from
)]

pub mod types;
pub mod x;

pub mod dc_aheader;
pub mod dc_apeerstate;
pub mod dc_array;
pub mod dc_chat;
// pub mod dc_chatlist;
pub mod dc_configure;
pub mod dc_contact;
pub mod dc_context;
// pub mod dc_dehtml;
pub mod dc_e2ee;
pub mod dc_hash;
pub mod dc_imap;
// pub mod dc_imex;
pub mod dc_job;
pub mod dc_jobthread;
pub mod dc_jsmn;
pub mod dc_key;
pub mod dc_keyhistory;
pub mod dc_keyring;
pub mod dc_location;
pub mod dc_log;
pub mod dc_loginparam;
pub mod dc_lot;
pub mod dc_mimefactory;
// pub mod dc_mimeparser;
// pub mod dc_move;
pub mod dc_msg;
pub mod dc_oauth2;
// pub mod dc_openssl;
pub mod dc_param;
pub mod dc_pgp;
// pub mod dc_qr;
// pub mod dc_receive_imf;
pub mod dc_saxparser;
// pub mod dc_securejoin;
// pub mod dc_simplify;
pub mod dc_smtp;
pub mod dc_sqlite3;
pub mod dc_stock;
pub mod dc_strbuilder;
pub mod dc_strencode;
// pub mod dc_token;
pub mod dc_tools;

// #[cfg(test)]
// mod tests {
//     use std::ffi::{CStr, CString};
//     use std::os::raw::c_int;
//     use std::ptr::NonNull;

//     use crate::dc_chat::*;
//     use crate::dc_chatlist::*;
//     use crate::dc_configure::dc_configure;
//     use crate::dc_contact::*;
//     use crate::dc_context::*;
//     use crate::dc_imap::*;
//     use crate::dc_job::{
//         dc_perform_imap_fetch, dc_perform_imap_idle, dc_perform_imap_jobs, dc_perform_smtp_idle,
//         dc_perform_smtp_jobs,
//     };
//     use crate::dc_lot::*;

//     extern "C" fn cb(ctx: *mut dc_context_t, event: c_int, data1: u64, data2: u64) -> u64 {
//         println!("event: {} ({}, {})", event, data1, data2);
//         if data2 > 10000 {
//             println!(
//                 "  {}",
//                 unsafe { CStr::from_ptr(data2 as *const _) }
//                     .to_str()
//                     .unwrap()
//             );
//         }
//         0
//     }

//     struct Wrapper(NonNull<dc_context_t>);

//     unsafe impl std::marker::Send for Wrapper {}
//     unsafe impl std::marker::Sync for Wrapper {}

//     #[test]
//     fn test_basics() {
//         unsafe {
//             let mut ctx = dc_context_new(Some(cb), std::ptr::null_mut(), std::ptr::null_mut());
//             let info = dc_get_info(ctx);
//             let info_s = CStr::from_ptr(info);
//             println!("info: {}", info_s.to_str().unwrap());

//             let sendable_ctx = Wrapper(NonNull::new(ctx).unwrap());
//             let t1 = std::thread::spawn(move || loop {
//                 dc_perform_imap_jobs(sendable_ctx.0.as_ptr());
//                 dc_perform_imap_fetch(sendable_ctx.0.as_ptr());
//                 dc_perform_imap_idle(sendable_ctx.0.as_ptr());
//             });

//             let sendable_ctx = Wrapper(NonNull::new(ctx).unwrap());
//             let t2 = std::thread::spawn(move || loop {
//                 dc_perform_smtp_jobs(sendable_ctx.0.as_ptr());
//                 dc_perform_smtp_idle(sendable_ctx.0.as_ptr());
//             });

//             let dbfile = CString::new("../deltachat-core/build/hello.db").unwrap();
//             println!("opening dir");
//             dc_open(ctx, dbfile.as_ptr(), std::ptr::null());

//             dc_configure(ctx);

//             std::thread::sleep_ms(4000);

//             let email = CString::new("dignifiedquire@gmail.com").unwrap();
//             println!("sending a message");
//             let contact_id = dc_create_contact(ctx, std::ptr::null(), email.as_ptr());
//             let chat_id = dc_create_chat_by_contact_id(ctx, contact_id);
//             let msg_text = CString::new("Hi, here is my first message!").unwrap();
//             dc_send_text_msg(ctx, chat_id, msg_text.as_ptr());

//             println!("fetching chats..");
//             let chats = dc_get_chatlist(ctx, 0, std::ptr::null(), 0);

//             for i in 0..dc_chatlist_get_cnt(chats) {
//                 let summary = dc_chatlist_get_summary(chats, 0, std::ptr::null_mut());
//                 let text1 = dc_lot_get_text1(summary);
//                 let text2 = dc_lot_get_text2(summary);

//                 let text1_s = if !text1.is_null() {
//                     Some(CStr::from_ptr(text1))
//                 } else {
//                     None
//                 };
//                 let text2_s = if !text2.is_null() {
//                     Some(CStr::from_ptr(text2))
//                 } else {
//                     None
//                 };
//                 println!("chat: {} - {:?} - {:?}", i, text1_s, text2_s,);
//                 dc_lot_unref(summary);
//             }
//             dc_chatlist_unref(chats);

//             // let msglist = dc_get_chat_msgs(ctx, chat_id, 0, 0);
//             // for i in 0..dc_array_get_cnt(msglist) {
//             //     let msg_id = dc_array_get_id(msglist, i);
//             //     let msg = dc_get_msg(context, msg_id);
//             //     let text = CStr::from_ptr(dc_msg_get_text(msg)).unwrap();
//             //     println!("Message {}: {}\n", i + 1, text.to_str().unwrap());
//             //     dc_msg_unref(msg);
//             // }
//             // dc_array_unref(msglist);

//             t1.join().unwrap();
//             t2.join().unwrap();
//         }
//     }
// }
