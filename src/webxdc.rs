//! # Handle webxdc messages.

use crate::constants::Viewtype;
use crate::context::Context;
use crate::dc_tools::{dc_create_smeared_timestamp, dc_open_file_std};
use crate::message::{Message, MessageState, MsgId};
use crate::mimeparser::SystemMessage;
use crate::param::Param;
use crate::{chat, EventType};
use anyhow::{bail, ensure, format_err, Result};
use lettre_email::mime::{self};
use lettre_email::PartBuilder;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::convert::TryFrom;
use std::fs::File;
use std::io::Read;
use zip::ZipArchive;

pub const WEBXDC_SUFFIX: &str = "xdc";
const WEBXDC_DEFAULT_ICON: &str = "__webxdc__/default-icon.png";

/// Defines the maximal size in bytes of an .xdc file that can be sent.
///
/// We introduce a limit to force developer to create small .xdc
/// to save user's traffic and disk space for a better ux.
///
/// The 100K limit should also let .xdc pass worse-quality auto-download filters
/// which are usually 160K incl. base64 overhead.
///
/// The limit is also an experiment to see how small we can go;
/// it is planned to raise that limit as needed in subsequent versions.
pub(crate) const WEBXDC_SENDING_LIMIT: usize = 102400;

/// Be more tolerant for .xdc sizes on receiving -
/// might be, the senders version uses already a larger limit
/// and not showing the .xdc on some devices would be even worse ux.
const WEBXDC_RECEIVING_LIMIT: usize = 1048576;

/// Raw information read from manifest.toml
#[derive(Debug, Deserialize)]
#[non_exhaustive]
struct WebxdcManifest {
    name: Option<String>,
}

/// Parsed information from WebxdcManifest and fallbacks.
#[derive(Debug, Serialize)]
pub struct WebxdcInfo {
    pub name: String,
    pub icon: String,
    pub summary: String,
}

/// Status Update ID.
#[derive(
    Debug, Copy, Clone, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
pub struct StatusUpdateId(u32);

impl StatusUpdateId {
    /// Create a new [MsgId].
    pub fn new(id: u32) -> StatusUpdateId {
        StatusUpdateId(id)
    }

    /// Gets StatusUpdateId as untyped integer.
    /// Avoid using this outside ffi.
    pub fn to_u32(self) -> u32 {
        self.0
    }
}

impl rusqlite::types::ToSql for StatusUpdateId {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput> {
        let val = rusqlite::types::Value::Integer(self.0 as i64);
        let out = rusqlite::types::ToSqlOutput::Owned(val);
        Ok(out)
    }
}

// Array of update items as sent on the wire.
#[derive(Debug, Deserialize)]
struct StatusUpdates {
    updates: Vec<StatusUpdateItem>,
}

/// Update items as sent on the wire and as stored in the database.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct StatusUpdateItem {
    payload: Value,

    #[serde(skip_serializing_if = "Option::is_none")]
    info: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    summary: Option<String>,
}

impl Context {
    pub(crate) async fn is_webxdc_file(&self, filename: &str, buf: &[u8]) -> Result<bool> {
        if filename.ends_with(WEBXDC_SUFFIX) {
            let reader = std::io::Cursor::new(buf);
            if let Ok(mut archive) = zip::ZipArchive::new(reader) {
                if let Ok(_index_html) = archive.by_name("index.html") {
                    if buf.len() <= WEBXDC_RECEIVING_LIMIT {
                        return Ok(true);
                    } else {
                        error!(
                            self,
                            "{} exceeds acceptable size of {} bytes.",
                            &filename,
                            WEBXDC_SENDING_LIMIT
                        );
                    }
                }
            }
        }
        Ok(false)
    }

    /// Takes an update-json as `{payload: PAYLOAD}` (or legacy `PAYLOAD`)
    /// writes it to the database and handles events, info-messages and summary.
    async fn create_status_update_record(
        &self,
        instance: &mut Message,
        update_str: &str,
        timestamp: i64,
    ) -> Result<StatusUpdateId> {
        let update_str = update_str.trim();
        if update_str.is_empty() {
            bail!("create_status_update_record: empty update.");
        }

        let status_update_item: StatusUpdateItem = {
            if let Ok(item) = serde_json::from_str::<StatusUpdateItem>(update_str) {
                match instance.state {
                    MessageState::Undefined
                    | MessageState::OutPreparing
                    | MessageState::OutDraft => StatusUpdateItem {
                        payload: item.payload,
                        info: None, // no info-messages in draft mode
                        summary: item.summary,
                    },
                    _ => item,
                }
            } else {
                // TODO: this fallback (legacy `PAYLOAD`) should be deleted soon, together with the test below
                let payload: Value = serde_json::from_str(update_str)?; // checks if input data are valid json
                StatusUpdateItem {
                    payload,
                    info: None,
                    summary: None,
                }
            }
        };

        if let Some(ref info) = status_update_item.info {
            chat::add_info_msg(self, instance.chat_id, info.as_str(), timestamp).await?;
        }

        if let Some(ref summary) = status_update_item.summary {
            if instance
                .param
                .update_timestamp(Param::WebxdcSummaryTimestamp, timestamp)?
            {
                instance.param.set(Param::WebxdcSummary, summary);
                instance.update_param(self).await;
                self.emit_event(EventType::MsgsChanged {
                    chat_id: instance.chat_id,
                    msg_id: instance.id,
                });
            }
        }

        let rowid = self
            .sql
            .insert(
                "INSERT INTO msgs_status_updates (msg_id, update_item) VALUES(?, ?);",
                paramsv![instance.id, serde_json::to_string(&status_update_item)?],
            )
            .await?;
        let status_update_id = StatusUpdateId(u32::try_from(rowid)?);

        self.emit_event(EventType::WebxdcStatusUpdate {
            msg_id: instance.id,
            status_update_id,
        });

        Ok(status_update_id)
    }

    /// Sends a status update for an webxdc instance.
    ///
    /// If the instance is a draft,
    /// the status update is sent once the instance is actually sent.
    ///
    /// If an update is sent immediately, the message-id of the update-message is returned,
    /// this update-message is visible in chats, however, the id may be useful.
    pub async fn send_webxdc_status_update(
        &self,
        instance_msg_id: MsgId,
        update_str: &str,
        descr: &str,
    ) -> Result<Option<MsgId>> {
        let mut instance = Message::load_from_db(self, instance_msg_id).await?;
        if instance.viewtype != Viewtype::Webxdc {
            bail!("send_webxdc_status_update: is no webxdc message");
        }

        let status_update_id = self
            .create_status_update_record(
                &mut instance,
                update_str,
                dc_create_smeared_timestamp(self).await,
            )
            .await?;
        match instance.state {
            MessageState::Undefined | MessageState::OutPreparing | MessageState::OutDraft => {
                // send update once the instance is actually send
                Ok(None)
            }
            _ => {
                // send update now
                // (also send updates on MessagesState::Failed, maybe only one member cannot receive)
                let mut status_update = Message {
                    chat_id: instance.chat_id,
                    viewtype: Viewtype::Text,
                    text: Some(descr.to_string()),
                    hidden: true,
                    ..Default::default()
                };
                status_update
                    .param
                    .set_cmd(SystemMessage::WebxdcStatusUpdate);
                status_update.param.set(
                    Param::Arg,
                    self.render_webxdc_status_update_object(
                        instance_msg_id,
                        Some(status_update_id),
                    )
                    .await?
                    .ok_or_else(|| format_err!("Status object expected."))?,
                );
                status_update.set_quote(self, Some(&instance)).await?;
                let status_update_msg_id =
                    chat::send_msg(self, instance.chat_id, &mut status_update).await?;
                Ok(Some(status_update_msg_id))
            }
        }
    }

    pub(crate) async fn build_status_update_part(&self, json: &str) -> PartBuilder {
        PartBuilder::new()
            .content_type(&"application/json".parse::<mime::Mime>().unwrap())
            .header((
                "Content-Disposition",
                "attachment; filename=\"status-update.json\"",
            ))
            .body(json)
    }

    /// Receives status updates from receive_imf to the database
    /// and sends out an event.
    ///
    /// `msg_id` may be an instance (in case there are initial status updates)
    /// or a reply to an instance (for all other updates).
    ///
    /// `json` is an array containing one or more update items as created by send_webxdc_status_update(),
    /// the array is parsed using serde, the single payloads are used as is.
    pub(crate) async fn receive_status_update(&self, msg_id: MsgId, json: &str) -> Result<()> {
        let msg = Message::load_from_db(self, msg_id).await?;
        let (timestamp, mut instance) = if msg.viewtype == Viewtype::Webxdc {
            (msg.timestamp_sort, msg)
        } else if let Some(parent) = msg.parent(self).await? {
            if parent.viewtype == Viewtype::Webxdc {
                (msg.timestamp_sort, parent)
            } else {
                bail!("receive_status_update: message is not the child of a webxdc message.")
            }
        } else {
            bail!("receive_status_update: status message has no parent.")
        };

        let updates: StatusUpdates = serde_json::from_str(json)?;
        for update_item in updates.updates {
            self.create_status_update_record(
                &mut instance,
                &*serde_json::to_string(&update_item)?,
                timestamp,
            )
            .await?;
        }

        Ok(())
    }

    /// Returns status updates as an JSON-array.
    ///
    /// Example: `[{"payload":"any update data"},{"payload":"another update data"}]`
    /// The updates may be filtered by a given status_update_id;
    /// if no updates are available, an empty JSON-array is returned.
    pub async fn get_webxdc_status_updates(
        &self,
        instance_msg_id: MsgId,
        status_update_id: Option<StatusUpdateId>,
    ) -> Result<String> {
        let json = self
            .sql
            .query_map(
                "SELECT update_item FROM msgs_status_updates WHERE msg_id=? AND (1=? OR id=?)",
                paramsv![
                    instance_msg_id,
                    if status_update_id.is_some() { 0 } else { 1 },
                    status_update_id.unwrap_or(StatusUpdateId(0))
                ],
                |row| row.get::<_, String>(0),
                |rows| {
                    let mut json = String::default();
                    for row in rows {
                        let update_item = row?;
                        if !json.is_empty() {
                            json.push_str(",\n");
                        }
                        json.push_str(&update_item);
                    }
                    Ok(json)
                },
            )
            .await?;
        Ok(format!("[{}]", json))
    }

    /// Render JSON-object for status updates as used on the wire.
    pub(crate) async fn render_webxdc_status_update_object(
        &self,
        instance_msg_id: MsgId,
        status_update_id: Option<StatusUpdateId>,
    ) -> Result<Option<String>> {
        let updates_array = self
            .get_webxdc_status_updates(instance_msg_id, status_update_id)
            .await?;
        if updates_array == "[]" {
            Ok(None)
        } else {
            Ok(Some(format!(r#"{{"updates":{}}}"#, updates_array)))
        }
    }
}

async fn parse_webxdc_manifest(bytes: &[u8]) -> Result<WebxdcManifest> {
    let manifest: WebxdcManifest = toml::from_slice(bytes)?;
    Ok(manifest)
}

async fn get_blob(archive: &mut ZipArchive<File>, name: &str) -> Result<Vec<u8>> {
    let mut file = archive.by_name(name)?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;
    Ok(buf)
}

impl Message {
    /// Get handle to a webxdc ZIP-archive.
    /// To check for file existance use archive.by_name(), to read a file, use get_blob(archive).
    async fn get_webxdc_archive(&self, context: &Context) -> Result<ZipArchive<File>> {
        let path = self
            .get_file(context)
            .ok_or_else(|| format_err!("No webxdc instance file."))?;
        let file = dc_open_file_std(context, path)?;
        let archive = zip::ZipArchive::new(file)?;
        Ok(archive)
    }

    /// Return file form inside an archive.
    /// Currently, this works only if the message is an webxdc instance.
    pub async fn get_webxdc_blob(&self, context: &Context, name: &str) -> Result<Vec<u8>> {
        ensure!(self.viewtype == Viewtype::Webxdc, "No webxdc instance.");

        if name == WEBXDC_DEFAULT_ICON {
            return Ok(include_bytes!("../assets/icon-webxdc.png").to_vec());
        }

        // ignore first slash.
        // this way, files can be accessed absolutely (`/index.html`) as well as relatively (`index.html`)
        let name = if name.starts_with('/') {
            name.split_at(1).1
        } else {
            name
        };

        let mut archive = self.get_webxdc_archive(context).await?;
        get_blob(&mut archive, name).await
    }

    /// Return info from manifest.toml or from fallbacks.
    pub async fn get_webxdc_info(&self, context: &Context) -> Result<WebxdcInfo> {
        ensure!(self.viewtype == Viewtype::Webxdc, "No webxdc instance.");
        let mut archive = self.get_webxdc_archive(context).await?;

        let mut manifest = if let Ok(bytes) = get_blob(&mut archive, "manifest.toml").await {
            if let Ok(manifest) = parse_webxdc_manifest(&bytes).await {
                manifest
            } else {
                WebxdcManifest { name: None }
            }
        } else {
            WebxdcManifest { name: None }
        };

        if let Some(ref name) = manifest.name {
            let name = name.trim();
            if name.is_empty() {
                warn!(context, "empty name given in manifest");
                manifest.name = None;
            }
        }

        Ok(WebxdcInfo {
            name: if let Some(name) = manifest.name {
                name
            } else {
                self.get_filename().unwrap_or_default()
            },
            icon: if archive.by_name("icon.png").is_ok() {
                "icon.png".to_string()
            } else if archive.by_name("icon.jpg").is_ok() {
                "icon.jpg".to_string()
            } else {
                WEBXDC_DEFAULT_ICON.to_string()
            },
            summary: self
                .param
                .get(Param::WebxdcSummary)
                .unwrap_or_default()
                .to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chat::{
        create_group_chat, forward_msgs, send_msg, send_text_msg, ChatId, ProtectionStatus,
    };
    use crate::dc_receive_imf::dc_receive_imf;
    use crate::test_utils::TestContext;
    use async_std::fs::File;
    use async_std::io::WriteExt;

    #[allow(clippy::assertions_on_constants)]
    #[async_std::test]
    async fn test_webxdc_file_limits() -> Result<()> {
        assert!(WEBXDC_SENDING_LIMIT >= 32768);
        assert!(WEBXDC_SENDING_LIMIT < 16777216);
        assert!(WEBXDC_RECEIVING_LIMIT >= WEBXDC_SENDING_LIMIT * 2);
        assert!(WEBXDC_RECEIVING_LIMIT < 16777216);
        Ok(())
    }

    #[async_std::test]
    async fn test_is_webxdc_file() -> Result<()> {
        let t = TestContext::new().await;
        assert!(
            !t.is_webxdc_file(
                "bad-ext-no-zip.txt",
                include_bytes!("../test-data/message/issue_523.txt")
            )
            .await?
        );
        assert!(
            !t.is_webxdc_file(
                "bad-ext-good-zip.txt",
                include_bytes!("../test-data/webxdc/minimal.xdc")
            )
            .await?
        );
        assert!(
            !t.is_webxdc_file(
                "good-ext-no-zip.xdc",
                include_bytes!("../test-data/message/issue_523.txt")
            )
            .await?
        );
        assert!(
            !t.is_webxdc_file(
                "good-ext-no-index-html.xdc",
                include_bytes!("../test-data/webxdc/no-index-html.xdc")
            )
            .await?
        );
        assert!(
            t.is_webxdc_file(
                "good-ext-good-zip.xdc",
                include_bytes!("../test-data/webxdc/minimal.xdc")
            )
            .await?
        );
        Ok(())
    }

    async fn create_webxdc_instance(t: &TestContext, name: &str, bytes: &[u8]) -> Result<Message> {
        let file = t.get_blobdir().join(name);
        File::create(&file).await?.write_all(bytes).await?;
        let mut instance = Message::new(Viewtype::File);
        instance.set_file(file.to_str().unwrap(), None);
        Ok(instance)
    }

    async fn send_webxdc_instance(t: &TestContext, chat_id: ChatId) -> Result<Message> {
        let mut instance = create_webxdc_instance(
            t,
            "minimal.xdc",
            include_bytes!("../test-data/webxdc/minimal.xdc"),
        )
        .await?;
        let instance_msg_id = send_msg(t, chat_id, &mut instance).await?;
        Message::load_from_db(t, instance_msg_id).await
    }

    #[async_std::test]
    async fn test_send_webxdc_instance() -> Result<()> {
        let t = TestContext::new_alice().await;
        let chat_id = create_group_chat(&t, ProtectionStatus::Unprotected, "foo").await?;

        // send as .xdc file
        let instance = send_webxdc_instance(&t, chat_id).await?;
        assert_eq!(instance.viewtype, Viewtype::Webxdc);
        assert_eq!(instance.get_filename(), Some("minimal.xdc".to_string()));
        assert_eq!(instance.chat_id, chat_id);

        // sending using bad extension is not working, even when setting Viewtype to webxdc
        let file = t.get_blobdir().join("index.html");
        File::create(&file)
            .await?
            .write_all("<html>ola!</html>".as_ref())
            .await?;
        let mut instance = Message::new(Viewtype::Webxdc);
        instance.set_file(file.to_str().unwrap(), None);
        assert!(send_msg(&t, chat_id, &mut instance).await.is_err());

        Ok(())
    }

    #[async_std::test]
    async fn test_forward_webxdc_instance() -> Result<()> {
        let t = TestContext::new_alice().await;
        let chat_id = create_group_chat(&t, ProtectionStatus::Unprotected, "foo").await?;
        let instance = send_webxdc_instance(&t, chat_id).await?;
        t.send_webxdc_status_update(
            instance.id,
            r#"{"info": "foo", "summary":"bar", "payload": 42}"#,
            "descr",
        )
        .await?;
        assert!(!instance.is_forwarded());
        assert_eq!(
            t.get_webxdc_status_updates(instance.id, None).await?,
            r#"[{"payload":42,"info":"foo","summary":"bar"}]"#
        );
        assert_eq!(chat_id.get_msg_cnt(&t).await?, 2); // instance and info
        let info = Message::load_from_db(&t, instance.id)
            .await?
            .get_webxdc_info(&t)
            .await?;
        assert_eq!(info.summary, "bar".to_string());

        // forwarding an instance creates a fresh instance; updates etc. are not forwarded
        forward_msgs(&t, &[instance.get_id()], chat_id).await?;
        let instance2 = t.get_last_msg_in(chat_id).await;
        assert!(instance2.is_forwarded());
        assert_eq!(t.get_webxdc_status_updates(instance2.id, None).await?, "[]");
        assert_eq!(chat_id.get_msg_cnt(&t).await?, 3); // two instances, only one info
        let info = Message::load_from_db(&t, instance2.id)
            .await?
            .get_webxdc_info(&t)
            .await?;
        assert_eq!(info.summary, "".to_string());

        Ok(())
    }

    #[async_std::test]
    async fn test_receive_webxdc_instance() -> Result<()> {
        let t = TestContext::new_alice().await;
        dc_receive_imf(
            &t,
            include_bytes!("../test-data/message/webxdc_good_extension.eml"),
            "INBOX",
            false,
        )
        .await?;
        let instance = t.get_last_msg().await;
        assert_eq!(instance.viewtype, Viewtype::Webxdc);
        assert_eq!(instance.get_filename(), Some("minimal.xdc".to_string()));

        dc_receive_imf(
            &t,
            include_bytes!("../test-data/message/webxdc_bad_extension.eml"),
            "INBOX",
            false,
        )
        .await?;
        let instance = t.get_last_msg().await;
        assert_eq!(instance.viewtype, Viewtype::File); // we require the correct extension, only a mime type is not sufficient
        assert_eq!(instance.get_filename(), Some("index.html".to_string()));

        Ok(())
    }

    #[async_std::test]
    async fn test_delete_webxdc_instance() -> Result<()> {
        let t = TestContext::new_alice().await;
        let chat_id = create_group_chat(&t, ProtectionStatus::Unprotected, "foo").await?;

        let mut instance = create_webxdc_instance(
            &t,
            "minimal.xdc",
            include_bytes!("../test-data/webxdc/minimal.xdc"),
        )
        .await?;
        chat_id.set_draft(&t, Some(&mut instance)).await?;
        let instance = chat_id.get_draft(&t).await?.unwrap();
        t.send_webxdc_status_update(instance.id, r#"{"payload": 42}"#, "descr")
            .await?;
        assert_eq!(
            t.get_webxdc_status_updates(instance.id, None).await?,
            r#"[{"payload":42}]"#.to_string()
        );

        // set_draft(None) deletes the message without the need to simulate network
        chat_id.set_draft(&t, None).await?;
        assert_eq!(
            t.get_webxdc_status_updates(instance.id, None).await?,
            "[]".to_string()
        );
        assert_eq!(
            t.sql
                .count("SELECT COUNT(*) FROM msgs_status_updates;", paramsv![],)
                .await?,
            0
        );

        Ok(())
    }

    #[async_std::test]
    async fn test_create_status_update_record() -> Result<()> {
        let t = TestContext::new_alice().await;
        let chat_id = create_group_chat(&t, ProtectionStatus::Unprotected, "foo").await?;
        let mut instance = send_webxdc_instance(&t, chat_id).await?;

        assert_eq!(t.get_webxdc_status_updates(instance.id, None).await?, "[]");

        let id = t
            .create_status_update_record(
                &mut instance,
                "\n\n{\"payload\": {\"foo\":\"bar\"}}\n",
                1640178619,
            )
            .await?;
        assert_eq!(
            t.get_webxdc_status_updates(instance.id, Some(id)).await?,
            r#"[{"payload":{"foo":"bar"}}]"#
        );

        assert!(t
            .create_status_update_record(&mut instance, "\n\n\n", 1640178619)
            .await
            .is_err());
        assert!(t
            .create_status_update_record(&mut instance, "bad json", 1640178619)
            .await
            .is_err());
        assert_eq!(
            t.get_webxdc_status_updates(instance.id, Some(id)).await?,
            r#"[{"payload":{"foo":"bar"}}]"#
        );
        assert_eq!(
            t.get_webxdc_status_updates(instance.id, None).await?,
            r#"[{"payload":{"foo":"bar"}}]"#
        );

        let id = t
            .create_status_update_record(
                &mut instance,
                r#"{"payload" : { "foo2":"bar2"}}"#,
                1640178619,
            )
            .await?;
        assert_eq!(
            t.get_webxdc_status_updates(instance.id, Some(id)).await?,
            r#"[{"payload":{"foo2":"bar2"}}]"#
        );
        t.create_status_update_record(&mut instance, r#"{"payload":true}"#, 1640178619)
            .await?;
        assert_eq!(
            t.get_webxdc_status_updates(instance.id, None).await?,
            r#"[{"payload":{"foo":"bar"}},
{"payload":{"foo2":"bar2"}},
{"payload":true}]"#
        );

        let id = t
            .create_status_update_record(
                &mut instance,
                r#"{"payload" : 1, "sender": "that is not used"}"#,
                1640178619,
            )
            .await?;
        assert_eq!(
            t.get_webxdc_status_updates(instance.id, Some(id)).await?,
            r#"[{"payload":1}]"#
        );

        // TODO: legacy `PAYLOAD` support should be deleted soon
        let id = t
            .create_status_update_record(&mut instance, r#"{"foo" : 1}"#, 1640178619)
            .await?;
        assert_eq!(
            t.get_webxdc_status_updates(instance.id, Some(id)).await?,
            r#"[{"payload":{"foo":1}}]"#
        );

        Ok(())
    }

    #[async_std::test]
    async fn test_receive_status_update() -> Result<()> {
        let t = TestContext::new_alice().await;
        let chat_id = create_group_chat(&t, ProtectionStatus::Unprotected, "foo").await?;
        let instance = send_webxdc_instance(&t, chat_id).await?;

        assert!(t
            .receive_status_update(instance.id, r#"foo: bar"#)
            .await
            .is_err()); // no json
        assert!(t
            .receive_status_update(instance.id, r#"{"updada":[{"payload":{"foo":"bar"}}]}"#)
            .await
            .is_err()); // "updates" object missing
        assert!(t
            .receive_status_update(instance.id, r#"{"updates":[{"foo":"bar"}]}"#)
            .await
            .is_err()); // "payload" field missing
        assert!(t
            .receive_status_update(instance.id, r#"{"updates":{"payload":{"foo":"bar"}}}"#)
            .await
            .is_err()); // not an array

        t.receive_status_update(instance.id, r#"{"updates":[{"payload":{"foo":"bar"}}]}"#)
            .await?;
        assert_eq!(
            t.get_webxdc_status_updates(instance.id, None).await?,
            r#"[{"payload":{"foo":"bar"}}]"#
        );

        t.receive_status_update(
            instance.id,
            r#" {"updates": [ {"payload" :42} , {"payload": 23} ] } "#,
        )
        .await?;
        assert_eq!(
            t.get_webxdc_status_updates(instance.id, None).await?,
            r#"[{"payload":{"foo":"bar"}},
{"payload":42},
{"payload":23}]"#
        );

        t.receive_status_update(
            instance.id,
            r#" {"updates": [ {"payload" :"ok", "future_item": "test"}  ], "from": "future" } "#,
        )
        .await?; // ignore members that may be added in the future
        assert_eq!(
            t.get_webxdc_status_updates(instance.id, None).await?,
            r#"[{"payload":{"foo":"bar"}},
{"payload":42},
{"payload":23},
{"payload":"ok"}]"#
        );

        Ok(())
    }

    async fn expect_status_update_event(t: &TestContext, instance_id: MsgId) -> Result<()> {
        let event = t
            .evtracker
            .get_matching(|evt| matches!(evt, EventType::WebxdcStatusUpdate { .. }))
            .await;
        match event {
            EventType::WebxdcStatusUpdate {
                msg_id,
                status_update_id,
            } => {
                assert_eq!(
                    t.get_webxdc_status_updates(msg_id, Some(status_update_id))
                        .await?,
                    r#"[{"payload":{"foo":"bar"}}]"#
                );
                assert_eq!(msg_id, instance_id);
            }
            _ => unreachable!(),
        }
        Ok(())
    }

    #[async_std::test]
    async fn test_send_webxdc_status_update() -> Result<()> {
        let alice = TestContext::new_alice().await;
        let bob = TestContext::new_bob().await;

        // Alice sends an webxdc instance and a status update
        let alice_chat = alice.create_chat(&bob).await;
        let alice_instance = send_webxdc_instance(&alice, alice_chat.id).await?;
        let sent1 = &alice.pop_sent_msg().await;
        assert_eq!(alice_instance.viewtype, Viewtype::Webxdc);
        assert!(!sent1.payload().contains("report-type=status-update"));

        let status_update_msg_id = alice
            .send_webxdc_status_update(
                alice_instance.id,
                r#"{"payload" : {"foo":"bar"}}"#,
                "descr text",
            )
            .await?
            .unwrap();
        expect_status_update_event(&alice, alice_instance.id).await?;
        let sent2 = &alice.pop_sent_msg().await;
        let alice_update = Message::load_from_db(&alice, status_update_msg_id).await?;
        assert!(alice_update.hidden);
        assert_eq!(alice_update.viewtype, Viewtype::Text);
        assert_eq!(alice_update.get_filename(), None);
        assert_eq!(alice_update.text, Some("descr text".to_string()));
        assert_eq!(alice_update.chat_id, alice_instance.chat_id);
        assert_eq!(
            alice_update.parent(&alice).await?.unwrap().id,
            alice_instance.id
        );
        assert_eq!(alice_chat.id.get_msg_cnt(&alice).await?, 1);
        assert!(sent2.payload().contains("report-type=status-update"));
        assert!(sent2.payload().contains("descr text"));
        assert_eq!(
            alice
                .get_webxdc_status_updates(alice_instance.id, None)
                .await?,
            r#"[{"payload":{"foo":"bar"}}]"#
        );

        alice
            .send_webxdc_status_update(
                alice_instance.id,
                r#"{"payload":{"snipp":"snapp"}}"#,
                "bla text",
            )
            .await?
            .unwrap();
        assert_eq!(
            alice
                .get_webxdc_status_updates(alice_instance.id, None)
                .await?,
            r#"[{"payload":{"foo":"bar"}},
{"payload":{"snipp":"snapp"}}]"#
        );

        // Bob receives all messages
        bob.recv_msg(sent1).await;
        let bob_instance = bob.get_last_msg().await;
        let bob_chat_id = bob_instance.chat_id;
        assert_eq!(bob_instance.rfc724_mid, alice_instance.rfc724_mid);
        assert_eq!(bob_instance.viewtype, Viewtype::Webxdc);
        assert_eq!(bob_chat_id.get_msg_cnt(&bob).await?, 1);

        bob.recv_msg(sent2).await;
        expect_status_update_event(&bob, bob_instance.id).await?;
        assert_eq!(bob_chat_id.get_msg_cnt(&bob).await?, 1);

        assert_eq!(
            bob.get_webxdc_status_updates(bob_instance.id, None).await?,
            r#"[{"payload":{"foo":"bar"}}]"#
        );

        // Alice has a second device and also receives messages there
        let alice2 = TestContext::new_alice().await;
        alice2.recv_msg(sent1).await;
        alice2.recv_msg(sent2).await;
        let alice2_instance = alice2.get_last_msg().await;
        let alice2_chat_id = alice2_instance.chat_id;
        assert_eq!(alice2_instance.viewtype, Viewtype::Webxdc);
        assert_eq!(alice2_chat_id.get_msg_cnt(&alice2).await?, 1);

        Ok(())
    }

    #[async_std::test]
    async fn test_render_webxdc_status_update_object() -> Result<()> {
        let t = TestContext::new_alice().await;
        let chat_id = create_group_chat(&t, ProtectionStatus::Unprotected, "a chat").await?;
        let mut instance = create_webxdc_instance(
            &t,
            "minimal.xdc",
            include_bytes!("../test-data/webxdc/minimal.xdc"),
        )
        .await?;
        chat_id.set_draft(&t, Some(&mut instance)).await?;
        assert!(t
            .render_webxdc_status_update_object(instance.id, None)
            .await?
            .is_none());

        t.send_webxdc_status_update(instance.id, r#"{"payload": 1}"#, "bla")
            .await?;
        assert!(t
            .render_webxdc_status_update_object(instance.id, None)
            .await?
            .is_some());

        Ok(())
    }

    #[async_std::test]
    async fn test_draft_and_send_webxdc_status_update() -> Result<()> {
        let alice = TestContext::new_alice().await;
        let bob = TestContext::new_bob().await;
        let alice_chat_id = alice.create_chat(&bob).await.id;

        // prepare webxdc instance,
        // status updates are not sent for drafts, therefore send_webxdc_status_update() returns Ok(None)
        let mut alice_instance = create_webxdc_instance(
            &alice,
            "minimal.xdc",
            include_bytes!("../test-data/webxdc/minimal.xdc"),
        )
        .await?;
        alice_chat_id
            .set_draft(&alice, Some(&mut alice_instance))
            .await?;
        let mut alice_instance = alice_chat_id.get_draft(&alice).await?.unwrap();

        let status_update_msg_id = alice
            .send_webxdc_status_update(alice_instance.id, r#"{"payload": {"foo":"bar"}}"#, "descr")
            .await?;
        assert_eq!(status_update_msg_id, None);
        expect_status_update_event(&alice, alice_instance.id).await?;
        let status_update_msg_id = alice
            .send_webxdc_status_update(alice_instance.id, r#"{"payload":42, "info":"i"}"#, "descr")
            .await?;
        assert_eq!(status_update_msg_id, None);
        assert!(!alice.get_last_msg().await.is_info()); // 'info: "i"' message not added in draft mode

        // send webxdc instance,
        // the initial status updates are sent together in the same message
        let alice_instance_id = send_msg(&alice, alice_chat_id, &mut alice_instance).await?;
        let sent1 = alice.pop_sent_msg().await;
        let alice_instance = Message::load_from_db(&alice, alice_instance_id).await?;
        assert_eq!(alice_instance.viewtype, Viewtype::Webxdc);
        assert_eq!(
            alice_instance.get_filename(),
            Some("minimal.xdc".to_string())
        );
        assert_eq!(alice_instance.chat_id, alice_chat_id);

        // bob receives the instance together with the initial updates in a single message
        bob.recv_msg(&sent1).await;
        let bob_instance = bob.get_last_msg().await;
        assert_eq!(bob_instance.viewtype, Viewtype::Webxdc);
        assert_eq!(bob_instance.get_filename(), Some("minimal.xdc".to_string()));
        assert!(sent1.payload().contains("Content-Type: application/json"));
        assert!(sent1.payload().contains("status-update.json"));
        assert!(sent1.payload().contains(r#""payload":{"foo":"bar"}"#));
        assert_eq!(
            bob.get_webxdc_status_updates(bob_instance.id, None).await?,
            r#"[{"payload":{"foo":"bar"}},
{"payload":42}]"# // 'info: "i"' ignored as sent in draft mode
        );

        Ok(())
    }

    #[async_std::test]
    async fn test_send_webxdc_status_update_to_non_webxdc() -> Result<()> {
        let t = TestContext::new_alice().await;
        let chat_id = create_group_chat(&t, ProtectionStatus::Unprotected, "foo").await?;
        let msg_id = send_text_msg(&t, chat_id, "ho!".to_string()).await?;
        assert!(t
            .send_webxdc_status_update(msg_id, r#"{"foo":"bar"}"#, "descr")
            .await
            .is_err());
        Ok(())
    }

    #[async_std::test]
    async fn test_get_webxdc_blob() -> Result<()> {
        let t = TestContext::new_alice().await;
        let chat_id = create_group_chat(&t, ProtectionStatus::Unprotected, "foo").await?;
        let instance = send_webxdc_instance(&t, chat_id).await?;

        let buf = instance.get_webxdc_blob(&t, "index.html").await?;
        assert_eq!(buf.len(), 188);
        assert!(String::from_utf8_lossy(&buf).contains("document.write"));

        assert!(instance
            .get_webxdc_blob(&t, "not-existent.html")
            .await
            .is_err());
        Ok(())
    }

    #[async_std::test]
    async fn test_get_webxdc_blob_default_icon() -> Result<()> {
        let t = TestContext::new_alice().await;
        let chat_id = create_group_chat(&t, ProtectionStatus::Unprotected, "foo").await?;
        let instance = send_webxdc_instance(&t, chat_id).await?;

        let buf = instance.get_webxdc_blob(&t, WEBXDC_DEFAULT_ICON).await?;
        assert!(buf.len() > 100);
        assert!(String::from_utf8_lossy(&buf).contains("PNG\r\n"));
        Ok(())
    }

    #[async_std::test]
    async fn test_get_webxdc_blob_with_absolute_paths() -> Result<()> {
        let t = TestContext::new_alice().await;
        let chat_id = create_group_chat(&t, ProtectionStatus::Unprotected, "foo").await?;
        let instance = send_webxdc_instance(&t, chat_id).await?;

        let buf = instance.get_webxdc_blob(&t, "/index.html").await?;
        assert!(String::from_utf8_lossy(&buf).contains("document.write"));

        assert!(instance.get_webxdc_blob(&t, "/not-there").await.is_err());
        Ok(())
    }

    #[async_std::test]
    async fn test_get_webxdc_blob_with_subdirs() -> Result<()> {
        let t = TestContext::new_alice().await;
        let chat_id = create_group_chat(&t, ProtectionStatus::Unprotected, "foo").await?;
        let mut instance = create_webxdc_instance(
            &t,
            "some-files.xdc",
            include_bytes!("../test-data/webxdc/some-files.xdc"),
        )
        .await?;
        chat_id.set_draft(&t, Some(&mut instance)).await?;

        let buf = instance.get_webxdc_blob(&t, "index.html").await?;
        assert_eq!(buf.len(), 65);
        assert!(String::from_utf8_lossy(&buf).contains("many files"));

        let buf = instance.get_webxdc_blob(&t, "subdir/bla.txt").await?;
        assert_eq!(buf.len(), 4);
        assert!(String::from_utf8_lossy(&buf).starts_with("bla"));

        let buf = instance
            .get_webxdc_blob(&t, "subdir/subsubdir/text.md")
            .await?;
        assert_eq!(buf.len(), 24);
        assert!(String::from_utf8_lossy(&buf).starts_with("this is a markdown file"));

        let buf = instance
            .get_webxdc_blob(&t, "subdir/subsubdir/text2.md")
            .await?;
        assert_eq!(buf.len(), 22);
        assert!(String::from_utf8_lossy(&buf).starts_with("another markdown"));

        let buf = instance
            .get_webxdc_blob(&t, "anotherdir/anothersubsubdir/foo.txt")
            .await?;
        assert_eq!(buf.len(), 4);
        assert!(String::from_utf8_lossy(&buf).starts_with("foo"));

        Ok(())
    }

    #[async_std::test]
    async fn test_parse_webxdc_manifest() -> Result<()> {
        let result = parse_webxdc_manifest(r#"key = syntax error"#.as_bytes()).await;
        assert!(result.is_err());

        let manifest = parse_webxdc_manifest(r#"no_name = "no name, no icon""#.as_bytes()).await?;
        assert_eq!(manifest.name, None);

        let manifest = parse_webxdc_manifest(r#"name = "name, no icon""#.as_bytes()).await?;
        assert_eq!(manifest.name, Some("name, no icon".to_string()));

        let manifest = parse_webxdc_manifest(
            r#"name = "foo"
icon = "bar""#
                .as_bytes(),
        )
        .await?;
        assert_eq!(manifest.name, Some("foo".to_string()));

        let manifest = parse_webxdc_manifest(
            r#"name = "foz"
icon = "baz"
add_item = "that should be just ignored"

[section]
sth_for_the = "future""#
                .as_bytes(),
        )
        .await?;
        assert_eq!(manifest.name, Some("foz".to_string()));

        Ok(())
    }

    #[async_std::test]
    async fn test_get_webxdc_info() -> Result<()> {
        let t = TestContext::new_alice().await;
        let chat_id = create_group_chat(&t, ProtectionStatus::Unprotected, "foo").await?;

        let instance = send_webxdc_instance(&t, chat_id).await?;
        let info = instance.get_webxdc_info(&t).await?;
        assert_eq!(info.name, "minimal.xdc");
        assert_eq!(info.icon, WEBXDC_DEFAULT_ICON.to_string());

        let mut instance = create_webxdc_instance(
            &t,
            "with-manifest-empty-name.xdc",
            include_bytes!("../test-data/webxdc/with-manifest-empty-name.xdc"),
        )
        .await?;
        chat_id.set_draft(&t, Some(&mut instance)).await?;
        let info = instance.get_webxdc_info(&t).await?;
        assert_eq!(info.name, "with-manifest-empty-name.xdc");
        assert_eq!(info.icon, WEBXDC_DEFAULT_ICON.to_string());

        let mut instance = create_webxdc_instance(
            &t,
            "with-manifest-no-name.xdc",
            include_bytes!("../test-data/webxdc/with-manifest-no-name.xdc"),
        )
        .await?;
        chat_id.set_draft(&t, Some(&mut instance)).await?;
        let info = instance.get_webxdc_info(&t).await?;
        assert_eq!(info.name, "with-manifest-no-name.xdc");
        assert_eq!(info.icon, WEBXDC_DEFAULT_ICON.to_string());

        let mut instance = create_webxdc_instance(
            &t,
            "with-minimal-manifest.xdc",
            include_bytes!("../test-data/webxdc/with-minimal-manifest.xdc"),
        )
        .await?;
        chat_id.set_draft(&t, Some(&mut instance)).await?;
        let info = instance.get_webxdc_info(&t).await?;
        assert_eq!(info.name, "nice app!");
        assert_eq!(info.icon, WEBXDC_DEFAULT_ICON.to_string());

        let mut instance = create_webxdc_instance(
            &t,
            "with-manifest-and-png-icon.xdc",
            include_bytes!("../test-data/webxdc/with-manifest-and-png-icon.xdc"),
        )
        .await?;
        chat_id.set_draft(&t, Some(&mut instance)).await?;
        let info = instance.get_webxdc_info(&t).await?;
        assert_eq!(info.name, "with some icon");
        assert_eq!(info.icon, "icon.png");

        let mut instance = create_webxdc_instance(
            &t,
            "with-png-icon.xdc",
            include_bytes!("../test-data/webxdc/with-png-icon.xdc"),
        )
        .await?;
        chat_id.set_draft(&t, Some(&mut instance)).await?;
        let info = instance.get_webxdc_info(&t).await?;
        assert_eq!(info.name, "with-png-icon.xdc");
        assert_eq!(info.icon, "icon.png");

        let mut instance = create_webxdc_instance(
            &t,
            "with-jpg-icon.xdc",
            include_bytes!("../test-data/webxdc/with-jpg-icon.xdc"),
        )
        .await?;
        chat_id.set_draft(&t, Some(&mut instance)).await?;
        let info = instance.get_webxdc_info(&t).await?;
        assert_eq!(info.name, "with-jpg-icon.xdc");
        assert_eq!(info.icon, "icon.jpg");

        let msg_id = send_text_msg(&t, chat_id, "foo".to_string()).await?;
        let msg = Message::load_from_db(&t, msg_id).await?;
        let result = msg.get_webxdc_info(&t).await;
        assert!(result.is_err());

        Ok(())
    }

    #[async_std::test]
    async fn test_webxdc_info_summary() -> Result<()> {
        let alice = TestContext::new_alice().await;
        let bob = TestContext::new_bob().await;

        // Alice creates an webxdc instance and updates summary
        let alice_chat = alice.create_chat(&bob).await;
        let alice_instance = send_webxdc_instance(&alice, alice_chat.id).await?;
        let sent_instance = &alice.pop_sent_msg().await;
        let info = alice_instance.get_webxdc_info(&alice).await?;
        assert_eq!(info.summary, "".to_string());

        alice
            .send_webxdc_status_update(
                alice_instance.id,
                r#"{"summary":"sum: 1", "payload":1}"#,
                "descr",
            )
            .await?;
        let sent_update1 = &alice.pop_sent_msg().await;
        let info = Message::load_from_db(&alice, alice_instance.id)
            .await?
            .get_webxdc_info(&alice)
            .await?;
        assert_eq!(info.summary, "sum: 1".to_string());

        alice
            .send_webxdc_status_update(
                alice_instance.id,
                r#"{"summary":"sum: 2", "payload":2}"#,
                "descr",
            )
            .await?;
        let sent_update2 = &alice.pop_sent_msg().await;
        let info = Message::load_from_db(&alice, alice_instance.id)
            .await?
            .get_webxdc_info(&alice)
            .await?;
        assert_eq!(info.summary, "sum: 2".to_string());

        // Bob receives the updates
        bob.recv_msg(sent_instance).await;
        let bob_instance = bob.get_last_msg().await;
        bob.recv_msg(sent_update1).await;
        bob.recv_msg(sent_update2).await;
        let info = Message::load_from_db(&bob, bob_instance.id)
            .await?
            .get_webxdc_info(&bob)
            .await?;
        assert_eq!(info.summary, "sum: 2".to_string());

        // Alice has a second device and also receives the updates there
        let alice2 = TestContext::new_alice().await;
        alice2.recv_msg(sent_instance).await;
        let alice2_instance = alice2.get_last_msg().await;
        alice2.recv_msg(sent_update1).await;
        alice2.recv_msg(sent_update2).await;
        let info = Message::load_from_db(&alice2, alice2_instance.id)
            .await?
            .get_webxdc_info(&alice2)
            .await?;
        assert_eq!(info.summary, "sum: 2".to_string());

        Ok(())
    }

    #[async_std::test]
    async fn test_webxdc_info_msg() -> Result<()> {
        let alice = TestContext::new_alice().await;
        let bob = TestContext::new_bob().await;

        // Alice sends update with an info message
        let alice_chat = alice.create_chat(&bob).await;
        let alice_instance = send_webxdc_instance(&alice, alice_chat.id).await?;
        let sent1 = &alice.pop_sent_msg().await;
        assert_eq!(alice_chat.id.get_msg_cnt(&alice).await?, 1);

        alice
            .send_webxdc_status_update(
                alice_instance.id,
                r#"{"info":"this appears in-chat", "payload":"sth. else"}"#,
                "descr text",
            )
            .await?;
        let sent2 = &alice.pop_sent_msg().await;
        assert_eq!(alice_chat.id.get_msg_cnt(&alice).await?, 2);
        let info_msg = alice.get_last_msg().await;
        assert!(info_msg.is_info());
        assert_eq!(
            info_msg.get_text(),
            Some("this appears in-chat".to_string())
        );
        assert_eq!(
            alice
                .get_webxdc_status_updates(alice_instance.id, None)
                .await?,
            r#"[{"payload":"sth. else","info":"this appears in-chat"}]"#
        );

        // Bob receives all messages
        bob.recv_msg(sent1).await;
        let bob_instance = bob.get_last_msg().await;
        let bob_chat_id = bob_instance.chat_id;
        bob.recv_msg(sent2).await;
        assert_eq!(bob_chat_id.get_msg_cnt(&bob).await?, 2);
        let info_msg = bob.get_last_msg().await;
        assert!(info_msg.is_info());
        assert_eq!(
            info_msg.get_text(),
            Some("this appears in-chat".to_string())
        );
        assert_eq!(
            bob.get_webxdc_status_updates(bob_instance.id, None).await?,
            r#"[{"payload":"sth. else","info":"this appears in-chat"}]"#
        );

        // Alice has a second device and also receives the info message there
        let alice2 = TestContext::new_alice().await;
        alice2.recv_msg(sent1).await;
        let alice2_instance = alice2.get_last_msg().await;
        let alice2_chat_id = alice2_instance.chat_id;
        alice2.recv_msg(sent2).await;
        assert_eq!(alice2_chat_id.get_msg_cnt(&alice2).await?, 2);
        let info_msg = alice2.get_last_msg().await;
        assert!(info_msg.is_info());
        assert_eq!(
            info_msg.get_text(),
            Some("this appears in-chat".to_string())
        );
        assert_eq!(
            alice2
                .get_webxdc_status_updates(alice2_instance.id, None)
                .await?,
            r#"[{"payload":"sth. else","info":"this appears in-chat"}]"#
        );

        Ok(())
    }
}
