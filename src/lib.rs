#![allow(
    dead_code,
    mutable_transmutes,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    unused_assignments,
    unused_mut
)]
#![feature(
    c_variadic,
    const_raw_ptr_to_usize_cast,
    extern_types,
    ptr_wrapping_offset_from
)]
#![allow(unused_attributes)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

pub mod dc_aheader;
pub mod dc_apeerstate;
pub mod dc_array;
pub mod dc_chat;
pub mod dc_chatlist;
pub mod dc_configure;
pub mod dc_contact;
pub mod dc_context;
pub mod dc_dehtml;
pub mod dc_e2ee;
pub mod dc_hash;
pub mod dc_imap;
pub mod dc_imex;
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
pub mod dc_mimeparser;
pub mod dc_move;
pub mod dc_msg;
pub mod dc_oauth2;
pub mod dc_openssl;
pub mod dc_param;
pub mod dc_pgp;
pub mod dc_qr;
pub mod dc_receive_imf;
pub mod dc_saxparser;
pub mod dc_securejoin;
pub mod dc_simplify;
pub mod dc_smtp;
pub mod dc_sqlite3;
pub mod dc_stock;
pub mod dc_strbuilder;
pub mod dc_strencode;
pub mod dc_token;
pub mod dc_tools;

#[cfg(test)]
mod tests {
    use super::*;

    use std::ffi::{CStr, CString};
    use std::os::raw::c_int;
    use std::ptr::NonNull;

    use crate::dc_configure::dc_configure;
    use crate::dc_context::*;
    use crate::dc_job::{dc_perform_imap_fetch, dc_perform_imap_idle, dc_perform_imap_jobs};

    extern "C" fn cb(ctx: *mut dc_context_t, event: c_int, data1: u64, data2: u64) -> u64 {
        let info = if data2 > 0 {
            Some(unsafe { CStr::from_ptr(data2 as *const _) })
        } else {
            None
        };

        println!("event: {} - {} - {:?}", event, data1, info);

        0
    }

    struct Wrapper(NonNull<dc_context_t>);

    unsafe impl std::marker::Send for Wrapper {}
    unsafe impl std::marker::Sync for Wrapper {}

    #[test]
    fn test_basics() {
        unsafe {
            let mut ctx = dc_context_new(Some(cb), std::ptr::null_mut(), std::ptr::null_mut());
            let info = dc_get_info(ctx);
            let info_s = CStr::from_ptr(info);
            println!("info: {:?}", info_s);

            let dbfile = CString::new("hello.db").unwrap();
            let blobdir = CString::new("hello").unwrap();
            dc_open(ctx, dbfile.as_ptr(), blobdir.as_ptr());

            let sendable_ctx = Wrapper(NonNull::new(ctx).unwrap());

            dc_set_config(
                ctx,
                CString::new("addr").unwrap().as_ptr(),
                CString::new("d@testrun.org").unwrap().as_ptr(),
            );
            dc_set_config(
                ctx,
                CString::new("mail_pw").unwrap().as_ptr(),
                CString::new("__").unwrap().as_ptr(),
            );
            dc_configure(ctx);

            std::thread::spawn(move || loop {
                dc_perform_imap_jobs(sendable_ctx.0.as_ptr());
                dc_perform_imap_fetch(sendable_ctx.0.as_ptr());
                dc_perform_imap_idle(sendable_ctx.0.as_ptr());
            })
            .join();
        }
    }
}
