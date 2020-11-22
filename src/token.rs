//! # Token module
//!
//! Functions to read/write token from/to the database. A token is any string associated with a key.
//!
//! Tokens are used in countermitm verification protocols.

use deltachat_derive::{FromSql, ToSql};

use crate::chat::ChatId;
use crate::context::Context;
use crate::dc_tools::{dc_create_id, time};

/// Token namespace
#[derive(
    Debug, Display, Clone, Copy, PartialEq, Eq, FromPrimitive, ToPrimitive, ToSql, FromSql,
)]
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
///
/// Returns created token.
pub async fn save(context: &Context, namespace: Namespace, chat: Option<ChatId>) -> String {
    let token = dc_create_id();
    match chat {
        Some(chat_id) => context
            .sql
            .execute(
                "INSERT INTO tokens (namespc, foreign_id, token, timestamp) VALUES (?, ?, ?, ?);",
                paramsv![namespace, chat_id, token, time()],
            )
            .await
            .ok(),
        None => context
            .sql
            .execute(
                "INSERT INTO tokens (namespc, token, timestamp) VALUES (?, ?, ?);",
                paramsv![namespace, token, time()],
            )
            .await
            .ok(),
    };
    token
}

pub async fn lookup(
    context: &Context,
    namespace: Namespace,
    chat: Option<ChatId>,
) -> crate::sql::Result<Option<String>> {
    match chat {
        Some(chat_id) => {
            context
                .sql
                .query_get_value::<String>(
                    "SELECT token FROM tokens WHERE namespc=? AND foreign_id=?;",
                    paramsv![namespace, chat_id],
                )
                .await
        }
        // foreign_id is declared as `INTEGER DEFAULT 0` in the schema.
        None => {
            context
                .sql
                .query_get_value::<String>(
                    "SELECT token FROM tokens WHERE namespc=? AND foreign_id=0;",
                    paramsv![namespace],
                )
                .await
        }
    }
}

pub async fn lookup_or_new(
    context: &Context,
    namespace: Namespace,
    foreign_id: Option<ChatId>,
) -> String {
    if let Ok(Some(token)) = lookup(context, namespace, foreign_id).await {
        return token;
    }

    save(context, namespace, foreign_id).await
}

pub async fn exists(context: &Context, namespace: Namespace, token: &str) -> bool {
    context
        .sql
        .exists(
            "SELECT id FROM tokens WHERE namespc=? AND token=?;",
            paramsv![namespace, token],
        )
        .await
        .unwrap_or_default()
}
