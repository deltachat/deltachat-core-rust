//! # Synchronize things between devices.

use crate::chat::ChatId;
use crate::config::Config;
use crate::constants::{Viewtype, DC_CONTACT_ID_SELF};
use crate::context::Context;
use crate::dc_tools::dc_create_outgoing_rfc724_mid;
use crate::message::Message;
use crate::mimeparser::SystemMessage;
use crate::param::Param;
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

    /// Copies all sync items to a json string and clears the sync-table.
    async fn flush_sync_items(&self) -> Result<Option<String>> {
        let (ids, items) = self
            .sql
            .query_map(
                "SELECT id, item FROM multi_device_sync ORDER BY id;",
                paramsv![],
                |row| Ok((row.get::<_, u32>(0)?, row.get::<_, String>(1)?)),
                |rows| {
                    let mut ids = vec![];
                    let mut items = String::default();
                    for row in rows {
                        let (id, item) = row?;
                        ids.push(id);
                        if !items.is_empty() {
                            items.push_str(",\n");
                        }
                        items.push_str(&item);
                    }
                    Ok((ids, items))
                },
            )
            .await?;

        if items.is_empty() {
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
            Ok(Some(format!("{{\"items\":[\n{}\n]}}", items)))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::sync::{SyncItem, TokenData};
    use crate::test_utils::TestContext;
    use crate::token::Namespace;

    #[async_std::test]
    async fn test_flush_sync_items() -> anyhow::Result<()> {
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

        assert_eq!(
            t.flush_sync_items().await?.unwrap(),
            r#"{"items":[
{"AddToken":{"namespace":"Auth","token":"testtoken","grpid":"group123"}},
{"DeleteToken":{"namespace":"InviteNumber","token":"123!?\":.;{}","grpid":null}}
]}"#
        );

        assert!(t.flush_sync_items().await?.is_none());

        Ok(())
    }
}
