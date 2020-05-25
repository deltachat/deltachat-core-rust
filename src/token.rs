//! # Token module
//!
//! Functions to read/write token from/to the database. A token is any string associated with a key.
//!
//! Tokens are used in countermitm verification protocols.

use deltachat_derive::*;

use crate::chat::ChatId;
use crate::context::Context;
use crate::dc_tools::*;

/// Token namespace
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, FromPrimitive, ToPrimitive, Sqlx)]
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
pub async fn save(context: &Context, namespace: Namespace, foreign_id: ChatId) -> String {
    // foreign_id may be 0
    let token = dc_create_id();
    context
        .sql
        .execute(
            "INSERT INTO tokens (namespc, foreign_id, token, timestamp) VALUES (?, ?, ?, ?);",
            paramsx![namespace, foreign_id, &token, time()],
        )
        .await
        .ok();
    token
}

pub async fn lookup(context: &Context, namespace: Namespace, foreign_id: ChatId) -> Option<String> {
    context
        .sql
        .query_value(
            "SELECT token FROM tokens WHERE namespc=? AND foreign_id=?;",
            paramsx![namespace, foreign_id],
        )
        .await
        .ok()
}

pub async fn lookup_or_new(context: &Context, namespace: Namespace, foreign_id: ChatId) -> String {
    if let Some(token) = lookup(context, namespace, foreign_id).await {
        return token;
    }

    save(context, namespace, foreign_id).await
}

pub async fn exists(context: &Context, namespace: Namespace, token: &str) -> bool {
    context
        .sql
        .exists(
            "SELECT id FROM tokens WHERE namespc=? AND token=?;",
            paramsx![namespace, token],
        )
        .await
        .unwrap_or_default()
}
