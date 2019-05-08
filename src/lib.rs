#![allow(
    mutable_transmutes,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
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

#[macro_use]
extern crate failure;
#[macro_use]
extern crate num_derive;
// #[macro_use]
// extern crate rental;

#[macro_use]
pub mod dc_log;

mod pgp;

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
pub mod dc_loginparam;
pub mod dc_lot;
pub mod dc_mimefactory;
pub mod dc_mimeparser;
pub mod dc_move;
pub mod dc_msg;
pub mod dc_oauth2;
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
pub mod types;
pub mod x;

pub mod constants;
pub use self::constants::*;
