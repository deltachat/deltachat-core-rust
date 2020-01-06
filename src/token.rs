//! # Token module
//!
//! Functions to read/write token from/to the database. A token is any string associated with a key.
//!
//! Tokens are used in countermitm verification protocols.

use deltachat_derive::*;

use crate::chat::ChatId;
use crate::context::Context;
use crate::dc_tools::*;
use crate::sql;

/// Token namespace
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, FromPrimitive, ToPrimitive, ToSql, FromSql)]
#[repr(i32)]
pub enum Namespace {
    Unknown = 0,
    Auth = 110,
    InviteNumber = 100,
}

impl Default for Namespace {
    fn default() -> Self {
        Namespace::Unknown
    }
}

/// Creates a new token and saves it into the database.
/// Returns created token.
pub fn save(context: &Context, namespace: Namespace, foreign_id: ChatId) -> String {
    // foreign_id may be 0
    let token = dc_create_id();
    sql::execute(
        context,
        &context.sql,
        "INSERT INTO tokens (namespc, foreign_id, token, timestamp) VALUES (?, ?, ?, ?);",
        params![namespace, foreign_id, &token, time()],
    )
    .ok();
    token
}

pub fn lookup(context: &Context, namespace: Namespace, foreign_id: ChatId) -> Option<String> {
    context.sql.query_get_value::<_, String>(
        context,
        "SELECT token FROM tokens WHERE namespc=? AND foreign_id=?;",
        params![namespace, foreign_id],
    )
}

pub fn lookup_or_new(context: &Context, namespace: Namespace, foreign_id: ChatId) -> String {
    lookup(context, namespace, foreign_id).unwrap_or_else(|| save(context, namespace, foreign_id))
}

pub fn exists(context: &Context, namespace: Namespace, token: &str) -> bool {
    context
        .sql
        .exists(
            "SELECT id FROM tokens WHERE namespc=? AND token=?;",
            params![namespace, token],
        )
        .unwrap_or_default()
}
