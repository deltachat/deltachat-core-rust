//! # Synchronize things between devices.

use crate::chat::ChatId;
use crate::config::Config;
use crate::constants::{Viewtype, DC_CONTACT_ID_SELF};
use crate::context::Context;
use crate::dc_tools::dc_create_outgoing_rfc724_mid;
use crate::message::Message;
use crate::mimeparser::SystemMessage;
use crate::param::Param;
use crate::sync::SyncItem::{AddToken, DeleteToken};
use crate::{chat, stock_str, token};
use anyhow::Result;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize)]
pub(crate) struct TokenData {
    pub(crate) namespace: token::Namespace,
    pub(crate) token: String,
    pub(crate) grpid: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub(crate) enum SyncItem {
    AddToken(TokenData),
    DeleteToken(TokenData),
}

#[derive(Deserialize)]
struct SyncItems {
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
        if let Some(json) = self.flush_sync_items().await? {
            // TODO: we should not create the self-chat only for sending sync-messages,
            // if we keep the general approach, we should set the chat to hidden.
            let chat_id = ChatId::create_for_contact(self, DC_CONTACT_ID_SELF).await?;

            let mut msg = Message {
                chat_id,
                viewtype: Viewtype::Text,
                text: Some(json),
                hidden: true,
                subject: stock_str::sync_msg_subject(self).await,
                rfc724_mid: dc_create_outgoing_rfc724_mid(
                    None,
                    &self
                        .get_config(Config::ConfiguredAddr)
                        .await?
                        .unwrap_or_default(),
                ),
                ..Default::default()
            };
            msg.param.set_cmd(SystemMessage::SyncMessage);
            msg.param.set_int(Param::GuaranteeE2ee, 1);
            msg.param.set_int(Param::SkipAutocrypt, 1);
            chat::send_msg(self, chat_id, &mut msg).await?;
        }
        Ok(())
    }

    /// Copies all sync items to a JSON string and clears the sync-table.
    async fn flush_sync_items(&self) -> Result<Option<String>> {
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
            // As new items may be added in between, delete only the rendered ones.
            self.sql
                .execute(
                    format!(
                        "DELETE FROM multi_device_sync WHERE id IN ({});",
                        ids.iter().map(|x| x.to_string()).join(",")
                    ),
                    paramsv![],
                )
                .await?;
            Ok(Some(format!("{{\"items\":[\n{}\n]}}", serialized)))
        }
    }

    /// Takes a JSON string created by `flush_sync_items()`
    /// and construct `SyncItems` from it.
    async fn parse_sync_items(&self, serialized: String) -> Result<SyncItems> {
        let sync_items: SyncItems = serde_json::from_str(&serialized)?;
        Ok(sync_items)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::TestContext;
    use crate::token::Namespace;
    use anyhow::bail;

    #[async_std::test]
    async fn test_flush_sync_items() -> Result<()> {
        let t = TestContext::new_alice().await;

        let x = t.flush_sync_items().await;
        info!(t, "{:?}", x);
        assert!(t.flush_sync_items().await?.is_none());

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

        let serialized = t.flush_sync_items().await?.unwrap();
        assert_eq!(
            serialized,
            r#"{"items":[
{"AddToken":{"namespace":"Auth","token":"testtoken","grpid":"group123"}},
{"DeleteToken":{"namespace":"InviteNumber","token":"123!?\":.;{}","grpid":null}}
]}"#
        );

        assert!(t.flush_sync_items().await?.is_none());

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
}
