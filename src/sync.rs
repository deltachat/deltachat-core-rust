//! # Synchronize items between devices.

use anyhow::Result;
use lettre_email::mime::{self};
use lettre_email::PartBuilder;
use serde::{Deserialize, Serialize};

use crate::chat::{self, Chat, ChatId};
use crate::config::Config;
use crate::constants::Blocked;
use crate::contact::ContactId;
use crate::context::Context;
use crate::log::LogExt;
use crate::message::{Message, MsgId, Viewtype};
use crate::mimeparser::SystemMessage;
use crate::param::Param;
use crate::sync::SyncData::{AddQrToken, AlterChat, DeleteQrToken};
use crate::token::Namespace;
use crate::tools::time;
use crate::{stock_str, token};

/// Whether to send device sync messages. Aimed for usage in the internal API.
#[derive(Debug, PartialEq)]
pub(crate) enum Sync {
    Nosync,
    Sync,
}

impl From<Sync> for bool {
    fn from(sync: Sync) -> bool {
        match sync {
            Sync::Nosync => false,
            Sync::Sync => true,
        }
    }
}

impl From<bool> for Sync {
    fn from(sync: bool) -> Sync {
        match sync {
            false => Sync::Nosync,
            true => Sync::Sync,
        }
    }
}

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
    AlterChat {
        id: chat::SyncId,
        action: chat::SyncAction,
    },
    Config {
        key: Config,
        val: String,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub(crate) enum SyncDataOrUnknown {
    SyncData(SyncData),
    Unknown(serde_json::Value),
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct SyncItem {
    timestamp: i64,

    data: SyncDataOrUnknown,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SyncItems {
    items: Vec<SyncItem>,
}

impl From<SyncData> for SyncDataOrUnknown {
    fn from(sync_data: SyncData) -> Self {
        Self::SyncData(sync_data)
    }
}

impl Context {
    /// Adds an item to the list of items that should be synchronized to other devices.
    ///
    /// NB: Private and `pub(crate)` functions shouldn't call this unless `Sync::Sync` is explicitly
    /// passed to them. This way it's always clear whether the code performs synchronisation.
    pub(crate) async fn add_sync_item(&self, data: SyncData) -> Result<()> {
        self.add_sync_item_with_timestamp(data, time()).await
    }

    /// Adds item and timestamp to the list of items that should be synchronized to other devices.
    /// If device synchronization is disabled, the function does nothing.
    async fn add_sync_item_with_timestamp(&self, data: SyncData, timestamp: i64) -> Result<()> {
        if !self.get_config_bool(Config::SyncMsgs).await? {
            return Ok(());
        }

        let item = SyncItem {
            timestamp,
            data: data.into(),
        };
        let item = serde_json::to_string(&item)?;
        self.sql
            .execute("INSERT INTO multi_device_sync (item) VALUES(?);", (item,))
            .await?;

        Ok(())
    }

    /// Adds most recent qr-code tokens for a given chat to the list of items to be synced.
    /// If device synchronization is disabled,
    /// no tokens exist or the chat is unpromoted, the function does nothing.
    pub(crate) async fn sync_qr_code_tokens(&self, chat_id: Option<ChatId>) -> Result<()> {
        if !self.get_config_bool(Config::SyncMsgs).await? {
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

    /// Adds deleted qr-code token to the list of items to be synced
    /// so that the token also gets deleted on the other devices.
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
                ChatId::create_for_contact_with_blocked(self, ContactId::SELF, Blocked::Yes)
                    .await?;
            let mut msg = Message {
                chat_id,
                viewtype: Viewtype::Text,
                text: stock_str::sync_msg_body(self).await,
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
                (),
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
                format!("{{\"items\":[\n{serialized}\n]}}"),
                ids.iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<String>>()
                    .join(","),
            )))
        }
    }

    pub(crate) fn build_sync_part(&self, json: String) -> PartBuilder {
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
                &format!("DELETE FROM multi_device_sync WHERE id IN ({ids});"),
                (),
            )
            .await?;
        Ok(())
    }

    /// Takes a JSON string created by `build_sync_json()`
    /// and construct `SyncItems` from it.
    pub(crate) fn parse_sync_items(&self, serialized: String) -> Result<SyncItems> {
        let sync_items: SyncItems = serde_json::from_str(&serialized)?;
        Ok(sync_items)
    }

    /// Executes sync items sent by other device.
    ///
    /// CAVE: When changing the code to handle other sync items,
    /// take care that does not result in calls to `add_sync_item()`
    /// as otherwise we would add in a dead-loop between two devices
    /// sending message back and forth.
    ///
    /// If an error is returned, the caller shall not try over because some sync items could be
    /// already executed. Sync items are considered independent and executed in the given order but
    /// regardless of whether executing of the previous items succeeded.
    pub(crate) async fn execute_sync_items(&self, items: &SyncItems) {
        info!(self, "executing {} sync item(s)", items.items.len());
        for item in &items.items {
            match &item.data {
                SyncDataOrUnknown::SyncData(data) => match data {
                    AddQrToken(token) => self.add_qr_token(token).await,
                    DeleteQrToken(token) => self.delete_qr_token(token).await,
                    AlterChat { id, action } => self.sync_alter_chat(id, action).await,
                    SyncData::Config { key, val } => match key.is_synced() {
                        true => self.set_config_ex(Sync::Nosync, *key, Some(val)).await,
                        false => Ok(()),
                    },
                },
                SyncDataOrUnknown::Unknown(data) => {
                    warn!(self, "Ignored unknown sync item: {data}.");
                    Ok(())
                }
            }
            .log_err(self)
            .ok();
        }
    }

    async fn add_qr_token(&self, token: &QrTokenData) -> Result<()> {
        let chat_id = if let Some(grpid) = &token.grpid {
            if let Some((chat_id, _, _)) = chat::get_chat_id_by_grpid(self, grpid).await? {
                Some(chat_id)
            } else {
                warn!(
                    self,
                    "Ignoring token for nonexistent/deleted group '{}'.", grpid
                );
                return Ok(());
            }
        } else {
            None
        };
        token::save(self, Namespace::InviteNumber, chat_id, &token.invitenumber).await?;
        token::save(self, Namespace::Auth, chat_id, &token.auth).await?;
        Ok(())
    }

    async fn delete_qr_token(&self, token: &QrTokenData) -> Result<()> {
        token::delete(self, Namespace::InviteNumber, &token.invitenumber).await?;
        token::delete(self, Namespace::Auth, &token.auth).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime};

    use anyhow::bail;

    use super::*;
    use crate::chat::Chat;
    use crate::chatlist::Chatlist;
    use crate::contact::{Contact, Origin};
    use crate::test_utils::TestContext;
    use crate::token::Namespace;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_config_sync_msgs() -> Result<()> {
        let t = TestContext::new_alice().await;
        assert!(!t.get_config_bool(Config::SyncMsgs).await?);
        t.set_config_bool(Config::SyncMsgs, true).await?;
        assert!(t.get_config_bool(Config::SyncMsgs).await?);
        t.set_config_bool(Config::SyncMsgs, false).await?;
        assert!(!t.get_config_bool(Config::SyncMsgs).await?);
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_build_sync_json() -> Result<()> {
        let t = TestContext::new_alice().await;
        t.set_config_bool(Config::SyncMsgs, true).await?;

        assert!(t.build_sync_json().await?.is_none());

        // Having one test on `SyncData::AlterChat` is sufficient here as
        // `chat::SyncAction::SetMuted` introduces enums inside items and `SystemTime`. Let's avoid
        // in-depth testing of the serialiser here which is an external crate.
        t.add_sync_item_with_timestamp(
            SyncData::AlterChat {
                id: chat::SyncId::ContactAddr("bob@example.net".to_string()),
                action: chat::SyncAction::SetMuted(chat::MuteDuration::Until(
                    SystemTime::UNIX_EPOCH + Duration::from_millis(42999),
                )),
            },
            1631781315,
        )
        .await?;

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
{"timestamp":1631781315,"data":{"AlterChat":{"id":{"ContactAddr":"bob@example.net"},"action":{"SetMuted":{"Until":{"secs_since_epoch":42,"nanos_since_epoch":999000000}}}}}},
{"timestamp":1631781316,"data":{"AddQrToken":{"invitenumber":"testinvite","auth":"testauth","grpid":"group123"}}},
{"timestamp":1631781317,"data":{"DeleteQrToken":{"invitenumber":"123!?\":.;{}","auth":"456","grpid":null}}}
]}"#
        );

        assert!(t.build_sync_json().await?.is_some());
        t.delete_sync_ids(ids).await?;
        assert!(t.build_sync_json().await?.is_none());

        let sync_items = t.parse_sync_items(serialized)?;
        assert_eq!(sync_items.items.len(), 3);

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_build_sync_json_sync_msgs_off() -> Result<()> {
        let t = TestContext::new_alice().await;
        t.set_config_bool(Config::SyncMsgs, false).await?;
        t.add_sync_item(SyncData::AddQrToken(QrTokenData {
            invitenumber: "testinvite".to_string(),
            auth: "testauth".to_string(),
            grpid: Some("group123".to_string()),
        }))
        .await?;
        assert!(t.build_sync_json().await?.is_none());
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_parse_sync_items() -> Result<()> {
        let t = TestContext::new_alice().await;

        assert!(t.parse_sync_items(r#"{bad json}"#.to_string()).is_err());

        assert!(t.parse_sync_items(r#"{"badname":[]}"#.to_string()).is_err());

        for bad_item_example in [
            r#"{"items":[{"timestamp":1631781316,"data":{"BadItem":{"invitenumber":"in","auth":"a","grpid":null}}}]}"#,
            r#"{"items":[{"timestamp":1631781316,"data":{"AddQrToken":{"invitenumber":"in","auth":123}}}]}"#, // `123` is invalid for `String`
            r#"{"items":[{"timestamp":1631781316,"data":{"AddQrToken":{"invitenumber":"in","auth":true}}}]}"#, // `true` is invalid for `String`
            r#"{"items":[{"timestamp":1631781316,"data":{"AddQrToken":{"invitenumber":"in","auth":[]}}}]}"#, // `[]` is invalid for `String`
            r#"{"items":[{"timestamp":1631781316,"data":{"AddQrToken":{"invitenumber":"in","auth":{}}}}]}"#, // `{}` is invalid for `String`
            r#"{"items":[{"timestamp":1631781316,"data":{"AddQrToken":{"invitenumber":"in","grpid":null}}}]}"#, // missing field
            r#"{"items":[{"timestamp":1631781316,"data":{"AlterChat":{"id":{"ContactAddr":"bob@example.net"},"action":"Burn"}}}]}"#, // Unknown enum value
        ] {
            let sync_items = t.parse_sync_items(bad_item_example.to_string()).unwrap();
            assert_eq!(sync_items.items.len(), 1);
            assert!(matches!(sync_items.items[0].timestamp, 1631781316));
            assert!(matches!(
                sync_items.items[0].data,
                SyncDataOrUnknown::Unknown(_)
            ));
        }

        // Test enums inside items and SystemTime
        let sync_items = t.parse_sync_items(
            r#"{"items":[{"timestamp":1631781318,"data":{"AlterChat":{"id":{"ContactAddr":"bob@example.net"},"action":{"SetMuted":{"Until":{"secs_since_epoch":42,"nanos_since_epoch":999000000}}}}}}]}"#.to_string(),
        )?;
        assert_eq!(sync_items.items.len(), 1);
        let SyncDataOrUnknown::SyncData(AlterChat { id, action }) =
            &sync_items.items.get(0).unwrap().data
        else {
            bail!("bad item");
        };
        assert_eq!(
            *id,
            chat::SyncId::ContactAddr("bob@example.net".to_string())
        );
        assert_eq!(
            *action,
            chat::SyncAction::SetMuted(chat::MuteDuration::Until(
                SystemTime::UNIX_EPOCH + Duration::from_millis(42999)
            ))
        );

        // empty item list is okay
        assert_eq!(
            t.parse_sync_items(r#"{"items":[]}"#.to_string())?
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
            ?;
        assert_eq!(sync_items.items.len(), 2);

        let sync_items = t.parse_sync_items(
            r#"{"items":[
{"timestamp":1631781318,"data":{"AddQrToken":{"invitenumber":"in","auth":"yip","grpid":null}}}
],"additional":"field"}"#
                .to_string(),
        )?;

        assert_eq!(sync_items.items.len(), 1);
        if let SyncDataOrUnknown::SyncData(AddQrToken(token)) =
            &sync_items.items.get(0).unwrap().data
        {
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
           ?;
        assert_eq!(sync_items.items.len(), 1);

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_execute_sync_items() -> Result<()> {
        let t = TestContext::new_alice().await;

        assert!(!token::exists(&t, Namespace::Auth, "yip-auth").await);

        let sync_items = t
            .parse_sync_items(
                r#"{"items":[
{"timestamp":1631781315,"data":{"AlterChat":{"id":{"ContactAddr":"bob@example.net"},"action":"Block"}}},
{"timestamp":1631781316,"data":{"AddQrToken":{"invitenumber":"yip-in","auth":"a"}}},
{"timestamp":1631781316,"data":{"DeleteQrToken":{"invitenumber":"in","auth":"delete unexistent, shall continue"}}},
{"timestamp":1631781316,"data":{"AddQrToken":{"invitenumber":"in","auth":"yip-auth"}}},
{"timestamp":1631781316,"data":{"AddQrToken":{"invitenumber":"in","auth":"foo","grpid":"non-existent"}}},
{"timestamp":1631781316,"data":{"AddQrToken":{"invitenumber":"in","auth":"directly deleted"}}},
{"timestamp":1631781316,"data":{"DeleteQrToken":{"invitenumber":"in","auth":"directly deleted"}}}
]}"#
                .to_string(),
            )
            ?;
        t.execute_sync_items(&sync_items).await;

        assert!(
            Contact::lookup_id_by_addr(&t, "bob@example.net", Origin::Unknown)
                .await?
                .is_none()
        );
        assert!(token::exists(&t, Namespace::InviteNumber, "yip-in").await);
        assert!(token::exists(&t, Namespace::Auth, "yip-auth").await);
        assert!(!token::exists(&t, Namespace::Auth, "non-existent").await);
        assert!(!token::exists(&t, Namespace::Auth, "directly deleted").await);

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_send_sync_msg() -> Result<()> {
        let alice = TestContext::new_alice().await;
        alice.set_config_bool(Config::SyncMsgs, true).await?;
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
        let chat_id = ChatId::create_for_contact(&alice, ContactId::SELF).await?;
        let chat = Chat::load_from_db(&alice, chat_id).await?;
        assert!(chat.is_self_talk());
        assert_eq!(Chatlist::try_load(&alice, 0, None, None).await?.len(), 1);
        let msgs = chat::get_chat_msgs(&alice, chat_id).await?;
        assert_eq!(msgs.len(), 0);

        // let alice's other device receive and execute the sync message,
        // also here, self-talk should stay hidden
        let sent_msg = alice.pop_sent_msg().await;
        let alice2 = TestContext::new_alice().await;
        alice2.set_config_bool(Config::SyncMsgs, true).await?;
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
