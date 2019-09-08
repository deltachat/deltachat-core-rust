#![deny(clippy::correctness)]
// TODO: make all of these errors, such that clippy actually passes.
#![warn(clippy::all, clippy::perf, clippy::not_unsafe_ptr_arg_deref)]
// This is nice, but for now just annoying.
#![allow(clippy::unreadable_literal)]
#![feature(ptr_wrapping_offset_from)]

#[macro_use]
extern crate failure_derive;
#[macro_use]
extern crate num_derive;
#[macro_use]
extern crate smallvec;
#[macro_use]
extern crate rusqlite;
extern crate strum;
#[macro_use]
extern crate strum_macros;

#[macro_use]
mod log;
#[macro_use]
pub mod error;

mod aheader;
pub mod chat;
pub mod chatlist;
pub mod config;
pub mod configure;
pub mod constants;
pub mod contact;
pub mod context;
mod e2ee;
mod imap;
pub mod job;
mod job_thread;
pub mod key;
pub mod keyring;
pub mod location;
pub mod lot;
pub mod message;
pub mod oauth2;
mod param;
pub mod peerstate;
pub mod pgp;
pub mod qr;
mod smtp;
pub mod sql;
mod stock;
pub mod types;
pub mod x;

pub mod dc_array;
mod dc_dehtml;
pub mod dc_imex;
mod dc_mimefactory;
pub mod dc_mimeparser;
pub mod dc_receive_imf;
mod dc_simplify;
mod dc_strencode;
mod dc_token;
pub mod dc_tools;
mod login_param;
pub mod securejoin;

#[cfg(test)]
mod test_utils;
