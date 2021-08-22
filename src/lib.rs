//! # Delta Chat Core Library.

#![forbid(unsafe_code)]
#![deny(
    clippy::correctness,
    missing_debug_implementations,
    clippy::all,
    clippy::indexing_slicing,
    clippy::wildcard_imports,
    clippy::needless_borrow
)]
#![allow(
    clippy::match_bool,
    clippy::eval_order_dependence,
    clippy::bool_assert_comparison
)]

#[macro_use]
extern crate num_derive;
#[macro_use]
extern crate smallvec;
#[macro_use]
extern crate rusqlite;
extern crate strum;
#[macro_use]
extern crate strum_macros;

pub trait ToSql: rusqlite::ToSql + Send + Sync {}

impl<T: rusqlite::ToSql + Send + Sync> ToSql for T {}

#[macro_use]
pub mod log;
#[macro_use]
pub mod error;

#[cfg(feature = "internals")]
#[macro_use]
pub mod sql;
#[cfg(not(feature = "internals"))]
#[macro_use]
mod sql;

pub mod headerdef;

pub(crate) mod events;
pub use events::*;

mod aheader;
mod blob;
pub mod chat;
pub mod chatlist;
pub mod config;
mod configure;
pub mod constants;
pub mod contact;
pub mod context;
mod e2ee;
pub mod ephemeral;
mod imap;
pub mod imex;
mod scheduler;
#[macro_use]
mod job;
mod format_flowed;
pub mod key;
mod keyring;
pub mod location;
mod login_param;
pub mod lot;
pub mod message;
mod mimefactory;
pub mod mimeparser;
pub mod oauth2;
mod param;
pub mod peerstate;
pub mod pgp;
pub mod provider;
pub mod qr;
pub mod quota;
pub mod securejoin;
mod simplify;
mod smtp;
pub mod stock_str;
mod token;
#[macro_use]
mod dehtml;
mod color;
pub mod html;
pub mod plaintext;

pub mod dc_receive_imf;
pub mod dc_tools;

pub mod accounts;

/// if set imap/incoming and smtp/outgoing MIME messages will be printed
pub const DCC_MIME_DEBUG: &str = "DCC_MIME_DEBUG";

/// if set IMAP protocol commands and responses will be printed
pub const DCC_IMAP_DEBUG: &str = "DCC_IMAP_DEBUG";

#[cfg(test)]
mod test_utils;
