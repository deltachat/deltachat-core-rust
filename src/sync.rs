//! # Synchronize things between devices.

use crate::chat::ChatId;
use crate::constants::{Viewtype, DC_CONTACT_ID_SELF};
use crate::context::Context;
use crate::message::Message;
use crate::mimeparser::SystemMessage;
use crate::param::Param;
use crate::sync::SyncItem::{AddToken, DeleteToken};
use crate::{chat, stock_str, token};
use anyhow::Result;
use itertools::Itertools;
use lettre_email::mime::{self};
use lettre_email::PartBuilder;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct TokenData {
    pub(crate) namespace: token::Namespace,
    pub(crate) token: String,
    pub(crate) grpid: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) enum SyncItem {
    AddToken(TokenData),
    DeleteToken(TokenData),
}

#[derive(Debug, Deserialize)]
pub(crate) struct SyncItems {
    items: Vec<SyncItem>,
}

impl Context {
    /// Adds an item to the list of things that should be synchronized to other devices.
    pub(crate) async fn add_sync_item(&self, item: SyncItem) -> Result<()> {
        let item = serde_json::to_string(&item)?;
        self.sql
            .execute(
                "INSERT INTO multi_device_sync (item) VALUES(?);",
                paramsv![item],
            )
            .await?;

        Ok(())
    }

    /// Sends out a self-sent message with items to be synchronized, if any.
    pub async fn send_sync_msg(&self) -> Result<()> {
        if let Some((json, ids)) = self.build_sync_json().await? {
            // TODO: we should not create the self-chat only for sending sync-messages,
            // if we keep the general approach, we should set the chat to hidden.
            // advantage of using self-sent chat is that we can piggyback sync messages easily on other messages.
            let chat_id = ChatId::create_for_contact(self, DC_CONTACT_ID_SELF).await?;

            let mut msg = Message {
                chat_id,
                viewtype: Viewtype::Text,
                text: Some(stock_str::sync_msg_body(self).await),
                hidden: true,
                subject: stock_str::sync_msg_subject(self).await,
                ..Default::default()
            };
            msg.param.set_cmd(SystemMessage::MultiDeviceSyncOnly);
            msg.param.set(Param::Arg, json);
            msg.param.set(Param::Arg2, ids);
            msg.param.set_int(Param::GuaranteeE2ee, 1);
            msg.param.set_int(Param::SkipAutocrypt, 1);
            chat::send_msg(self, chat_id, &mut msg).await?;
        }
        Ok(())
    }

    /// Copies all sync items to a JSON string and clears the sync-table.
    /// Returns the JSON string and the IDs used.
    pub(crate) async fn build_sync_json(&self) -> Result<Option<(String, String)>> {
        let (ids, serialized) = self
            .sql
            .query_map(
                "SELECT id, item FROM multi_device_sync ORDER BY id;",
                paramsv![],
                |row| Ok((row.get::<_, u32>(0)?, row.get::<_, String>(1)?)),
                |rows| {
                    let mut ids = vec![];
                    let mut serialized = String::default();
                    for row in rows {
                        let (id, item) = row?;
                        ids.push(id);
                        if !serialized.is_empty() {
                            serialized.push_str(",\n");
                        }
                        serialized.push_str(&item);
                    }
                    Ok((ids, serialized))
                },
            )
            .await?;

        if ids.is_empty() {
            Ok(None)
        } else {
            Ok(Some((
                format!("{{\"items\":[\n{}\n]}}", serialized),
                ids.iter().map(|x| x.to_string()).join(","),
            )))
        }
    }

    pub(crate) async fn build_sync_part(&self, json: String) -> PartBuilder {
        PartBuilder::new()
            .content_type(&"application/json".parse::<mime::Mime>().unwrap())
            .header((
                "Content-Disposition",
                "attachment; filename=\"multi-device-sync.json\"",
            ))
            .body(json)
    }

    /// Deletes IDs as returned by `build_sync_json()`.
    pub(crate) async fn delete_sync_ids(&self, ids: String) -> Result<()> {
        self.sql
            .execute(
                format!("DELETE FROM multi_device_sync WHERE id IN ({});", ids),
                paramsv![],
            )
            .await?;
        Ok(())
    }

    /// Takes a JSON string created by `build_sync_json()`
    /// and construct `SyncItems` from it.
    pub(crate) async fn parse_sync_items(&self, serialized: String) -> Result<SyncItems> {
        let sync_items: SyncItems = serde_json::from_str(&serialized)?;
        Ok(sync_items)
    }

    /// Execute sync items.
    ///
    /// CAVE: When changing the code to handle other sync items,
    /// take care that does not result in calls to `add_sync_item()`
    /// as otherwise we would add in a dead-loop between two devices
    /// sending message back and forth.
    ///
    /// If an error is returned, the caller shall not try over.
    /// Therefore, errors should only be returned on database errors or so.
    /// If eg. just an item cannot be deleted,
    /// that should not hold off the other items to be executed.
    async fn execute_sync_items(&self, items: &SyncItems) -> Result<()> {
        for item in &items.items {
            match item {
                AddToken(token) => {
                    let chat_id = if let Some(grpid) = &token.grpid {
                        if let Some((chat_id, _, _)) =
                            chat::get_chat_id_by_grpid(self, grpid).await?
                        {
                            Some(chat_id)
                        } else {
                            warn!(self, "Cannot assign token to unpromoted group '{}'.", grpid);
                            // TODO: really ignore? we could also save grpid instead,
                            // or do not send before promoted.
                            continue;
                        }
                    } else {
                        None
                    };
                    token::save(self, token.namespace, chat_id, &token.token, false).await?;
                }
                DeleteToken(token) => {
                    token::delete(self, token.namespace, &token.token, false).await?
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::TestContext;
    use crate::token::Namespace;
    use anyhow::bail;

    #[async_std::test]
    async fn test_build_sync_json() -> Result<()> {
        let t = TestContext::new_alice().await;

        assert!(t.build_sync_json().await?.is_none());

        t.add_sync_item(SyncItem::AddToken(TokenData {
            namespace: Namespace::Auth,
            token: "testtoken".to_string(),
            grpid: Some("group123".to_string()),
        }))
        .await?;
        t.add_sync_item(SyncItem::DeleteToken(TokenData {
            namespace: Namespace::InviteNumber,
            token: "123!?\":.;{}".to_string(),
            grpid: None,
        }))
        .await?;

        let (serialized, ids) = t.build_sync_json().await?.unwrap();
        assert_eq!(
            serialized,
            r#"{"items":[
{"AddToken":{"namespace":"Auth","token":"testtoken","grpid":"group123"}},
{"DeleteToken":{"namespace":"InviteNumber","token":"123!?\":.;{}","grpid":null}}
]}"#
        );

        assert!(t.build_sync_json().await?.is_some());
        t.delete_sync_ids(ids).await?;
        assert!(t.build_sync_json().await?.is_none());

        let sync_items = t.parse_sync_items(serialized).await?;
        assert_eq!(sync_items.items.len(), 2);

        Ok(())
    }

    #[async_std::test]
    async fn test_parse_sync_items() -> Result<()> {
        let t = TestContext::new_alice().await;

        assert!(t
            .parse_sync_items(r#"{bad json}"#.to_string())
            .await
            .is_err());

        assert!(t
            .parse_sync_items(r#"{"badname":[]}"#.to_string())
            .await
            .is_err());

        assert!(t
            .parse_sync_items(
                r#"{"items":[{"AddToken":{"namespace":"BadEnumValue","token":"yip","grpid":null}}]}"#
                    .to_string(),
            )
            .await.is_err());

        assert!(t
            .parse_sync_items(
                r#"{"items":[{"AddToken":{"namespace":"Auth","token":123}}]}"#.to_string(),
            )
            .await
            .is_err()); // `123` is invalid for `String`

        assert!(t
            .parse_sync_items(
                r#"{"items":[{"AddToken":{"namespace":"Auth","token":true}}]}"#.to_string(),
            )
            .await
            .is_err()); // `true` is invalid for `String`

        assert!(t
            .parse_sync_items(
                r#"{"items":[{"AddToken":{"namespace":"Auth","token":[]}}]}"#.to_string(),
            )
            .await
            .is_err()); // `[]` is invalid for `String`

        assert!(t
            .parse_sync_items(
                r#"{"items":[{"AddToken":{"namespace":"Auth","token":{}}}]}"#.to_string(),
            )
            .await
            .is_err()); // `{}` is invalid for `String`

        assert!(t
            .parse_sync_items(
                r#"{"items":[{"AddToken":{"namespace":"Auth","grpid":null}}]}"#.to_string(),
            )
            .await
            .is_err()); // missing field

        // empty item list is okay
        assert_eq!(
            t.parse_sync_items(r#"{"items":[]}"#.to_string())
                .await?
                .items
                .len(),
            0
        );

        // to allow forward compatibility, additional fields should not break parsing
        let sync_items = t
            .parse_sync_items(
                r#"{"items":[
{"DeleteToken":{"namespace":"Auth","token":"yip","grpid":null}},
{"AddToken":{"namespace":"Auth","token":"yip","additional":123,"grpid":null}}
]}"#
                .to_string(),
            )
            .await?;
        assert_eq!(sync_items.items.len(), 2);

        let sync_items = t
            .parse_sync_items(
                r#"{"items":[
{"AddToken":{"namespace":"Auth","token":"yip","grpid":null}}
],"additional":"field"}"#
                    .to_string(),
            )
            .await?;
        assert_eq!(sync_items.items.len(), 1);
        if let AddToken(token) = sync_items.items.get(0).unwrap() {
            assert_eq!(token.namespace, Namespace::Auth);
            assert_eq!(token.token, "yip");
            assert_eq!(token.grpid, None);
        } else {
            bail!("bad item");
        }

        // to allow backward compatibility, missing `Option<>` should not break parsing
        let sync_items = t
            .parse_sync_items(
                r#"{"items":[{"AddToken":{"namespace":"Auth","token":"yip"}}]}"#.to_string(),
            )
            .await?;
        assert_eq!(sync_items.items.len(), 1);

        Ok(())
    }

    #[async_std::test]
    async fn test_execute_sync_items() -> Result<()> {
        let t = TestContext::new_alice().await;

        assert!(!token::exists(&t, Namespace::Auth, "yip-auth").await);

        let sync_items = t
            .parse_sync_items(
                r#"{"items":[
{"AddToken":{"namespace":"InviteNumber","token":"yip-in"}},
{"DeleteToken":{"namespace":"Auth","token":"delete unexistant, shall continue"}},
{"AddToken":{"namespace":"Auth","token":"yip-auth"}},
{"AddToken":{"namespace":"Auth","token":"foo","grpid":"non-existant"}},
{"AddToken":{"namespace":"Auth","token":"directly deleted"}},
{"DeleteToken":{"namespace":"Auth","token":"directly deleted"}}
]}"#
                .to_string(),
            )
            .await?;
        t.execute_sync_items(&sync_items).await?;

        assert!(token::exists(&t, Namespace::InviteNumber, "yip-in").await);
        assert!(token::exists(&t, Namespace::Auth, "yip-auth").await);
        assert!(!token::exists(&t, Namespace::Auth, "non-existant").await);
        assert!(!token::exists(&t, Namespace::Auth, "directly deleted").await);

        Ok(())
    }
}
