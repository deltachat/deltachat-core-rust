#![allow(
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    non_upper_case_globals,
    non_camel_case_types,
    non_snake_case
)]
#![feature(c_variadic, ptr_wrapping_offset_from)]

#[macro_use]
extern crate failure_derive;
#[macro_use]
extern crate num_derive;
#[macro_use]
extern crate smallvec;
#[macro_use]
extern crate rusqlite;

#[macro_use]
mod log;

pub mod aheader;
pub mod config;
pub mod constants;
pub mod context;
pub mod error;
pub mod imap;
pub mod key;
pub mod keyhistory;
pub mod keyring;
pub mod oauth2;
pub mod peerstate;
pub mod pgp;
pub mod smtp;
pub mod sql;
pub mod stock;
pub mod types;
pub mod x;

pub mod dc_array;
pub mod dc_chat;
pub mod dc_chatlist;
pub mod dc_configure;
pub mod dc_contact;
pub mod dc_dehtml;
pub mod dc_e2ee;
pub mod dc_imex;
pub mod dc_job;
pub mod dc_jobthread;
pub mod dc_location;
pub mod dc_loginparam;
pub mod dc_lot;
pub mod dc_mimefactory;
pub mod dc_mimeparser;
pub mod dc_move;
pub mod dc_msg;
pub mod dc_param;
pub mod dc_qr;
pub mod dc_receive_imf;
pub mod dc_saxparser;
pub mod dc_securejoin;
pub mod dc_simplify;
pub mod dc_strencode;
pub mod dc_token;
pub mod dc_tools;

pub use self::constants::*;

#[cfg(test)]
pub mod test_utils;
