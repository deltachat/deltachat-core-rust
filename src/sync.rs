//! # Synchronize items between devices.

use crate::chat::{Chat, ChatId};
use crate::config::Config;
use crate::constants::{Blocked, Viewtype, DC_CONTACT_ID_SELF};
use crate::context::Context;
use crate::dc_tools::time;
use crate::message::{Message, MsgId};
use crate::mimeparser::SystemMessage;
use crate::param::Param;
use crate::sync::SyncData::{AddQrToken, DeleteQrToken};
use crate::token::Namespace;
use crate::{chat, stock_str, token};
use anyhow::Result;
use itertools::Itertools;
use lettre_email::mime::{self};
use lettre_email::PartBuilder;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct QrTokenData {
    pub(crate) invitenumber: String,
    pub(crate) auth: String,
    pub(crate) grpid: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) enum SyncData {
    AddQrToken(QrTokenData),
    DeleteQrToken(QrTokenData),
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct SyncItem {
    timestamp: i64,
    data: SyncData,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SyncItems {
    items: Vec<SyncItem>,
}

impl Context {
    /// Checks if sync messages shall be sent.
    /// Receiving sync messages is currently always enabled;
    /// the messages are force-encrypted anyway.
    async fn is_sync_sending_enabled(&self) -> Result<bool> {
        self.get_config_bool(Config::SendSyncMsgs).await
    }

    /// Adds an item to the list of items that should be synchronized to other devices.
    pub(crate) async fn add_sync_item(&self, data: SyncData) -> Result<()> {
        self.add_sync_item_with_timestamp(data, time()).await
    }

    /// Adds item and timestamp to the list of items that should be synchronized to other devices.
    /// If device synchronization is disabled, the function does nothing.
    async fn add_sync_item_with_timestamp(&self, data: SyncData, timestamp: i64) -> Result<()> {
        if !self.is_sync_sending_enabled().await? {
            return Ok(());
        }

        let item = SyncItem { timestamp, data };
        let item = serde_json::to_string(&item)?;
        self.sql
            .execute(
                "INSERT INTO multi_device_sync (item) VALUES(?);",
                paramsv![item],
            )
            .await?;

        Ok(())
    }

    /// Adds most recent qr-code tokens for a given chat to the list of items to be synced.
    /// If device synchronization is disabled,
    /// no tokens exist or the chat is unpromoted, the function does nothing.
    pub(crate) async fn sync_qr_code_tokens(&self, chat_id: Option<ChatId>) -> Result<()> {
        if !self.is_sync_sending_enabled().await? {
            return Ok(());
        }

        if let (Some(invitenumber), Some(auth)) = (
            token::lookup(self, Namespace::InviteNumber, chat_id).await?,
            token::lookup(self, Namespace::Auth, chat_id).await?,
        ) {
            let grpid = if let Some(chat_id) = chat_id {
                let chat = Chat::load_from_db(self, chat_id).await?;
                if !chat.is_promoted() {
                    info!(
                        self,
                        "group '{}' not yet promoted, do not sync tokens yet.", chat.grpid
                    );
                    return Ok(());
                }
                Some(chat.grpid)
            } else {
                None
            };
            self.add_sync_item(SyncData::AddQrToken(QrTokenData {
                invitenumber,
                auth,
                grpid,
            }))
            .await?;
        }
        Ok(())
    }

    // Add deleted qr-code token to the list of items to be synced
    // so that the token also gets deleted on the other devices.
    pub(crate) async fn sync_qr_code_token_deletion(
        &self,
        invitenumber: String,
        auth: String,
    ) -> Result<()> {
        self.add_sync_item(SyncData::DeleteQrToken(QrTokenData {
            invitenumber,
            auth,
            grpid: None,
        }))
        .await
    }

    /// Sends out a self-sent message with items to be synchronized, if any.
    pub async fn send_sync_msg(&self) -> Result<Option<MsgId>> {
        if let Some((json, ids)) = self.build_sync_json().await? {
            let chat_id =
                ChatId::create_for_contact_with_blocked(self, DC_CONTACT_ID_SELF, Blocked::Yes)
                    .await?;
            let mut msg = Message {
                chat_id,
                viewtype: Viewtype::Text,
                text: Some(stock_str::sync_msg_body(self).await),
                hidden: true,
                subject: stock_str::sync_msg_subject(self).await,
                ..Default::default()
            };
            msg.param.set_cmd(SystemMessage::MultiDeviceSync);
            msg.param.set(Param::Arg, json);
            msg.param.set(Param::Arg2, ids);
            msg.param.set_int(Param::GuaranteeE2ee, 1);
            Ok(Some(chat::send_msg(self, chat_id, &mut msg).await?))
        } else {
            Ok(None)
        }
    }

    /// Copies all sync items to a JSON string and clears the sync-table.
    /// Returns the JSON string and a comma-separated string of the IDs used.
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
    pub(crate) async fn execute_sync_items(&self, items: &SyncItems) -> Result<()> {
        info!(self, "executing {} sync item(s)", items.items.len());
        for item in &items.items {
            match &item.data {
                AddQrToken(token) => {
                    let chat_id = if let Some(grpid) = &token.grpid {
                        if let Some((chat_id, _, _)) =
                            chat::get_chat_id_by_grpid(self, grpid).await?
                        {
                            Some(chat_id)
                        } else {
                            warn!(
                                self,
                                "Ignoring token for nonexistent/deleted group '{}'.", grpid
                            );
                            continue;
                        }
                    } else {
                        None
                    };
                    token::save(self, Namespace::InviteNumber, chat_id, &token.invitenumber)
                        .await?;
                    token::save(self, Namespace::Auth, chat_id, &token.auth).await?;
                }
                DeleteQrToken(token) => {
                    token::delete(self, Namespace::InviteNumber, &token.invitenumber).await?;
                    token::delete(self, Namespace::Auth, &token.auth).await?;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chat::Chat;
    use crate::chatlist::Chatlist;
    use crate::test_utils::TestContext;
    use crate::token::Namespace;
    use anyhow::bail;

    #[async_std::test]
    async fn test_is_sync_sending_enabled() -> Result<()> {
        let t = TestContext::new_alice().await;
        assert!(!t.is_sync_sending_enabled().await?);
        t.set_config_bool(Config::SendSyncMsgs, true).await?;
        assert!(t.is_sync_sending_enabled().await?);
        t.set_config_bool(Config::SendSyncMsgs, false).await?;
        assert!(!t.is_sync_sending_enabled().await?);
        Ok(())
    }

    #[async_std::test]
    async fn test_build_sync_json() -> Result<()> {
        let t = TestContext::new_alice().await;
        t.set_config_bool(Config::SendSyncMsgs, true).await?;

        assert!(t.build_sync_json().await?.is_none());

        t.add_sync_item_with_timestamp(
            SyncData::AddQrToken(QrTokenData {
                invitenumber: "testinvite".to_string(),
                auth: "testauth".to_string(),
                grpid: Some("group123".to_string()),
            }),
            1631781316,
        )
        .await?;
        t.add_sync_item_with_timestamp(
            SyncData::DeleteQrToken(QrTokenData {
                invitenumber: "123!?\":.;{}".to_string(),
                auth: "456".to_string(),
                grpid: None,
            }),
            1631781317,
        )
        .await?;

        let (serialized, ids) = t.build_sync_json().await?.unwrap();
        assert_eq!(
            serialized,
            r#"{"items":[
{"timestamp":1631781316,"data":{"AddQrToken":{"invitenumber":"testinvite","auth":"testauth","grpid":"group123"}}},
{"timestamp":1631781317,"data":{"DeleteQrToken":{"invitenumber":"123!?\":.;{}","auth":"456","grpid":null}}}
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
    async fn test_build_sync_json_sync_msgs_off() -> Result<()> {
        let t = TestContext::new_alice().await;
        t.set_config_bool(Config::SendSyncMsgs, false).await?;
        t.add_sync_item(SyncData::AddQrToken(QrTokenData {
            invitenumber: "testinvite".to_string(),
            auth: "testauth".to_string(),
            grpid: Some("group123".to_string()),
        }))
        .await?;
        assert!(t.build_sync_json().await?.is_none());
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

        assert!(t.parse_sync_items(
                r#"{"items":[{"timestamp":1631781316,"data":{"BadItem":{"invitenumber":"in","auth":"a","grpid":null}}}]}"#
                    .to_string(),
            )
            .await.is_err());

        assert!(t.parse_sync_items(
                r#"{"items":[{"timestamp":1631781316,"data":{"AddQrToken":{"invitenumber":"in","auth":123}}}]}"#.to_string(),
            )
            .await
            .is_err()); // `123` is invalid for `String`

        assert!(t.parse_sync_items(
                r#"{"items":[{"timestamp":1631781316,"data":{"AddQrToken":{"invitenumber":"in","auth":true}}}]}"#.to_string(),
            )
            .await
            .is_err()); // `true` is invalid for `String`

        assert!(t.parse_sync_items(
                r#"{"items":[{"timestamp":1631781316,"data":{"AddQrToken":{"invitenumber":"in","auth":[]}}}]}"#.to_string(),
            )
            .await
            .is_err()); // `[]` is invalid for `String`

        assert!(t.parse_sync_items(
                r#"{"items":[{"timestamp":1631781316,"data":{"AddQrToken":{"invitenumber":"in","auth":{}}}}]}"#.to_string(),
            )
            .await
            .is_err()); // `{}` is invalid for `String`

        assert!(t.parse_sync_items(
                r#"{"items":[{"timestamp":1631781316,"data":{"AddQrToken":{"invitenumber":"in","grpid":null}}}]}"#.to_string(),
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
{"timestamp":1631781316,"data":{"DeleteQrToken":{"invitenumber":"in","auth":"yip","grpid":null}}},
{"timestamp":1631781316,"data":{"AddQrToken":{"invitenumber":"in","auth":"yip","additional":123,"grpid":null}}}
]}"#
                .to_string(),
            )
            .await?;
        assert_eq!(sync_items.items.len(), 2);

        let sync_items = t
            .parse_sync_items(
                r#"{"items":[
{"timestamp":1631781318,"data":{"AddQrToken":{"invitenumber":"in","auth":"yip","grpid":null}}}
],"additional":"field"}"#
                    .to_string(),
            )
            .await?;

        assert_eq!(sync_items.items.len(), 1);
        if let AddQrToken(token) = &sync_items.items.get(0).unwrap().data {
            assert_eq!(token.invitenumber, "in");
            assert_eq!(token.auth, "yip");
            assert_eq!(token.grpid, None);
        } else {
            bail!("bad item");
        }

        // to allow backward compatibility, missing `Option<>` should not break parsing
        let sync_items = t.parse_sync_items(
               r#"{"items":[{"timestamp":1631781319,"data":{"AddQrToken":{"invitenumber":"in","auth":"a"}}}]}"#.to_string(),
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
{"timestamp":1631781316,"data":{"AddQrToken":{"invitenumber":"yip-in","auth":"a"}}},
{"timestamp":1631781316,"data":{"DeleteQrToken":{"invitenumber":"in","auth":"delete unexistant, shall continue"}}},
{"timestamp":1631781316,"data":{"AddQrToken":{"invitenumber":"in","auth":"yip-auth"}}},
{"timestamp":1631781316,"data":{"AddQrToken":{"invitenumber":"in","auth":"foo","grpid":"non-existant"}}},
{"timestamp":1631781316,"data":{"AddQrToken":{"invitenumber":"in","auth":"directly deleted"}}},
{"timestamp":1631781316,"data":{"DeleteQrToken":{"invitenumber":"in","auth":"directly deleted"}}}
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

    #[async_std::test]
    async fn test_send_sync_msg() -> Result<()> {
        let alice = TestContext::new_alice().await;
        alice.set_config_bool(Config::SendSyncMsgs, true).await?;
        alice
            .add_sync_item(SyncData::AddQrToken(QrTokenData {
                invitenumber: "in".to_string(),
                auth: "testtoken".to_string(),
                grpid: None,
            }))
            .await?;
        let msg_id = alice.send_sync_msg().await?.unwrap();
        let msg = Message::load_from_db(&alice, msg_id).await?;
        let chat = Chat::load_from_db(&alice, msg.chat_id).await?;
        assert!(chat.is_self_talk());

        // check that the used self-talk is not visible to the user
        // but that creation will still work (in this case, the chat is empty)
        assert_eq!(Chatlist::try_load(&alice, 0, None, None).await?.len(), 0);
        let chat_id = ChatId::create_for_contact(&alice, DC_CONTACT_ID_SELF).await?;
        let chat = Chat::load_from_db(&alice, chat_id).await?;
        assert!(chat.is_self_talk());
        assert_eq!(Chatlist::try_load(&alice, 0, None, None).await?.len(), 1);
        let msgs = chat::get_chat_msgs(&alice, chat_id, 0, None).await?;
        assert_eq!(msgs.len(), 0);

        // let alice's other device receive and execute the sync message,
        // also here, self-talk should stay hidden
        let sent_msg = alice.pop_sent_msg().await;
        let alice2 = TestContext::new_alice().await;
        alice2.recv_msg(&sent_msg).await;
        assert!(token::exists(&alice2, token::Namespace::Auth, "testtoken").await);
        assert_eq!(Chatlist::try_load(&alice2, 0, None, None).await?.len(), 0);

        // the same sync message sent to bob must not be executed
        let bob = TestContext::new_bob().await;
        bob.recv_msg(&sent_msg).await;
        assert!(!token::exists(&bob, token::Namespace::Auth, "testtoken").await);

        Ok(())
    }
}
