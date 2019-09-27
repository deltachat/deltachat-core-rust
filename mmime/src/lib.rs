#![deny(clippy::correctness)]
// TODO: make all of these errors, such that clippy actually passes.
#![warn(clippy::all, clippy::perf, clippy::not_unsafe_ptr_arg_deref)]
// This is nice, but for now just annoying.
#![allow(clippy::unreadable_literal)]
#![feature(ptr_wrapping_offset_from)]
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

pub mod charconv;
pub mod chash;
pub mod clist;
pub mod display;
pub mod mailimf;
pub mod mailmime;
pub mod mmapstring;
pub mod other;

pub use self::charconv::*;
pub use self::chash::*;
pub use self::clist::*;
pub use self::display::*;
pub use self::mailimf::*;
pub use self::mailmime::*;
pub use self::mmapstring::*;
pub use self::other::*;

#[cfg(test)]
mod tests {
    use super::*;

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
            let res = crate::mailmime::content::mailmime_parse(
                c_data.as_ptr(),
                data.len() as usize,
                &mut current_index,
                &mut mime,
            );

            assert_eq!(res, MAIL_NO_ERROR as libc::c_int);
            assert!(!mime.is_null());

            display_mime(mime);

            mailmime::types::mailmime_free(mime);
        }
    }
}
