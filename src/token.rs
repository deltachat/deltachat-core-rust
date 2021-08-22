//! # Token module.
//!
//! Functions to read/write token from/to the database. A token is any string associated with a key.
//!
//! Tokens are used in countermitm verification protocols.

use anyhow::Result;
use deltachat_derive::{FromSql, ToSql};

use crate::chat::ChatId;
use crate::context::Context;
use crate::dc_tools::{dc_create_id, time};

/// Token namespace
#[derive(
    Debug, Display, Clone, Copy, PartialEq, Eq, FromPrimitive, ToPrimitive, ToSql, FromSql,
)]
#[repr(u32)]
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

/// Saves a token to the database.
pub async fn save(
    context: &Context,
    namespace: Namespace,
    foreign_id: Option<ChatId>,
    token: &str,
) -> Result<()> {
    match foreign_id {
        Some(foreign_id) => context
            .sql
            .execute(
                "INSERT INTO tokens (namespc, foreign_id, token, timestamp) VALUES (?, ?, ?, ?);",
                paramsv![namespace, foreign_id, token, time()],
            )
            .await?,
        None => {
            context
                .sql
                .execute(
                    "INSERT INTO tokens (namespc, token, timestamp) VALUES (?, ?, ?);",
                    paramsv![namespace, token, time()],
                )
                .await?
        }
    };

    Ok(())
}

/// Lookup most recently created token for a namespace/chat combination.
///
/// As there may be more than one valid token for a chat-id,
/// (eg. when a qr code token is withdrawn, recreated and revived later),
/// use lookup() for qr-code creation only;
/// do not use lookup() to check for token validity.
///
/// To check if a given token is valid, use exists().
pub async fn lookup(
    context: &Context,
    namespace: Namespace,
    chat: Option<ChatId>,
) -> Result<Option<String>> {
    let token = match chat {
        Some(chat_id) => {
            context
                .sql
                .query_get_value(
                    "SELECT token FROM tokens WHERE namespc=? AND foreign_id=? ORDER BY timestamp DESC LIMIT 1;",
                    paramsv![namespace, chat_id],
                )
                .await?
        }
        // foreign_id is declared as `INTEGER DEFAULT 0` in the schema.
        None => {
            context
                .sql
                .query_get_value(
                    "SELECT token FROM tokens WHERE namespc=? AND foreign_id=0 ORDER BY timestamp DESC LIMIT 1;",
                    paramsv![namespace],
                )
                .await?
        }
    };
    Ok(token)
}

pub async fn lookup_or_new(
    context: &Context,
    namespace: Namespace,
    foreign_id: Option<ChatId>,
) -> String {
    if let Ok(Some(token)) = lookup(context, namespace, foreign_id).await {
        return token;
    }

    let token = dc_create_id();
    save(context, namespace, foreign_id, &token).await.ok();
    token
}

pub async fn exists(context: &Context, namespace: Namespace, token: &str) -> bool {
    context
        .sql
        .exists(
            "SELECT COUNT(*) FROM tokens WHERE namespc=? AND token=?;",
            paramsv![namespace, token],
        )
        .await
        .unwrap_or_default()
}

pub async fn delete(context: &Context, namespace: Namespace, token: &str) -> Result<()> {
    context
        .sql
        .execute(
            "DELETE FROM tokens WHERE namespc=? AND token=?;",
            paramsv![namespace, token],
        )
        .await?;
    Ok(())
}
