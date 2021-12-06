//! # Download large messages manually.

use anyhow::{anyhow, Result};
use deltachat_derive::{FromSql, ToSql};
use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::constants::Viewtype;
use crate::context::Context;
use crate::dc_tools::time;
use crate::imap::{Imap, ImapActionResult};
use crate::job::{self, Action, Job, Status};
use crate::message::{Message, MsgId};
use crate::mimeparser::{MimeMessage, Part};
use crate::param::Params;
use crate::{job_try, stock_str, EventType};
use std::cmp::max;

/// Download limits should not be used below `MIN_DOWNLOAD_LIMIT`.
///
/// Some messages as non-delivery-reports (NDN) or read-receipts (MDN)
/// need to be downloaded completely to handle them correctly,
/// eg. to assign them to the correct chat.
/// As these messages are typically small,
/// they're catched by `MIN_DOWNLOAD_LIMIT`.
const MIN_DOWNLOAD_LIMIT: u32 = 32768;

/// If a message is downloaded only partially
/// and `delete_server_after` is set to small timeouts (eg. "at once"),
/// the user might have no chance to actually download that message.
/// `MIN_DELETE_SERVER_AFTER` increases the timeout in this case.
pub(crate) const MIN_DELETE_SERVER_AFTER: i64 = 48 * 60 * 60;

#[derive(
    Debug,
    Display,
    Clone,
    Copy,
    PartialEq,
    Eq,
    FromPrimitive,
    ToPrimitive,
    FromSql,
    ToSql,
    Serialize,
    Deserialize,
)]
#[repr(u32)]
pub enum DownloadState {
    Done = 0,
    Available = 10,
    Failure = 20,
    InProgress = 1000,
}

impl Default for DownloadState {
    fn default() -> Self {
        DownloadState::Done
    }
}

impl Context {
    // Returns validated download limit or `None` for "no limit".
    pub(crate) async fn download_limit(&self) -> Result<Option<u32>> {
        let download_limit = self.get_config_int(Config::DownloadLimit).await?;
        if download_limit <= 0 {
            Ok(None)
        } else {
            Ok(Some(max(MIN_DOWNLOAD_LIMIT, download_limit as u32)))
        }
    }
}

impl MsgId {
    /// Schedules full message download for partially downloaded message.
    pub async fn download_full(self, context: &Context) -> Result<()> {
        let msg = Message::load_from_db(context, self).await?;
        match msg.download_state() {
            DownloadState::Done => return Err(anyhow!("Nothing to download.")),
            DownloadState::InProgress => return Err(anyhow!("Download already in progress.")),
            DownloadState::Available | DownloadState::Failure => {
                self.update_download_state(context, DownloadState::InProgress)
                    .await?;
                job::add(
                    context,
                    Job::new(Action::DownloadMsg, self.to_u32(), Params::new(), 0),
                )
                .await?;
            }
        }
        Ok(())
    }

    pub(crate) async fn update_download_state(
        self,
        context: &Context,
        download_state: DownloadState,
    ) -> Result<()> {
        let msg = Message::load_from_db(context, self).await?;
        context
            .sql
            .execute(
                "UPDATE msgs SET download_state=? WHERE id=?;",
                paramsv![download_state, self],
            )
            .await?;
        context.emit_event(EventType::MsgsChanged {
            chat_id: msg.chat_id,
            msg_id: self,
        });
        Ok(())
    }
}

impl Message {
    /// Returns the download state of the message.
    pub fn download_state(&self) -> DownloadState {
        self.download_state
    }
}

impl Job {
    /// Actually download a message.
    /// Called in response to `Action::DownloadMsg`.
    pub(crate) async fn download_msg(&self, context: &Context, imap: &mut Imap) -> Status {
        if let Err(err) = imap.prepare(context).await {
            warn!(context, "download: could not connect: {:?}", err);
            return Status::RetryNow;
        }

        let msg = job_try!(Message::load_from_db(context, MsgId::new(self.foreign_id)).await);
        let server_folder = msg.server_folder.unwrap_or_default();
        match imap
            .fetch_single_msg(context, &server_folder, msg.server_uid)
            .await
        {
            ImapActionResult::RetryLater | ImapActionResult::Failed => {
                job_try!(
                    msg.id
                        .update_download_state(context, DownloadState::Failure)
                        .await
                );
                Status::Finished(Err(anyhow!("Call download_full() again to try over.")))
            }
            ImapActionResult::Success | ImapActionResult::AlreadyDone => {
                // update_download_state() not needed as receive_imf() already
                // set the state and emitted the event.
                Status::Finished(Ok(()))
            }
        }
    }
}

impl Imap {
    /// Download a single message and pipe it to receive_imf().
    ///
    /// receive_imf() is not directly aware that this is a result of a call to download_msg(),
    /// however, implicitly knows that as the existing message is flagged as being partly.
    async fn fetch_single_msg(
        &mut self,
        context: &Context,
        folder: &str,
        uid: u32,
    ) -> ImapActionResult {
        if let Some(imapresult) = self
            .prepare_imap_operation_on_msg(context, folder, uid)
            .await
        {
            return imapresult;
        }

        // we are connected, and the folder is selected
        info!(context, "Downloading message {}/{} fully...", folder, uid);

        let (_, error_cnt, _) = self
            .fetch_many_msgs(context, folder, vec![uid], false, false)
            .await;
        if error_cnt > 0 {
            return ImapActionResult::Failed;
        }

        ImapActionResult::Success
    }
}

impl MimeMessage {
    /// Creates a placeholder part and add that to `parts`.
    ///
    /// To create the placeholder, only the outermost header can be used,
    /// the mime-structure itself is not available.
    ///
    /// The placeholder part currently contains a text with size and availability of the message;
    /// in the future, we may do more advanced things as previews here.
    pub(crate) async fn create_stub_from_partial_download(
        &mut self,
        context: &Context,
        org_bytes: u32,
    ) -> Result<()> {
        let mut text = format!(
            "[{}]",
            stock_str::partial_download_msg_body(context, org_bytes).await
        );
        if let Some(delete_server_after) = context.get_config_delete_server_after().await? {
            let until = stock_str::download_availability(
                context,
                time() + max(delete_server_after, MIN_DELETE_SERVER_AFTER),
            )
            .await;
            text += format!(" [{}]", until).as_str();
        };

        info!(context, "Partial download: {}", text);

        self.parts.push(Part {
            typ: Viewtype::Text,
            msg: text,
            ..Default::default()
        });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chat::send_msg;
    use crate::constants::Viewtype;
    use crate::dc_receive_imf::dc_receive_imf_inner;
    use crate::ephemeral::Timer;
    use crate::test_utils::TestContext;
    use num_traits::FromPrimitive;

    #[test]
    fn test_downloadstate_values() {
        // values may be written to disk and must not change
        assert_eq!(DownloadState::Done, DownloadState::default());
        assert_eq!(DownloadState::Done, DownloadState::from_i32(0).unwrap());
        assert_eq!(
            DownloadState::Available,
            DownloadState::from_i32(10).unwrap()
        );
        assert_eq!(DownloadState::Failure, DownloadState::from_i32(20).unwrap());
        assert_eq!(
            DownloadState::InProgress,
            DownloadState::from_i32(1000).unwrap()
        );
    }

    #[async_std::test]
    async fn test_download_limit() -> Result<()> {
        let t = TestContext::new_alice().await;

        assert_eq!(t.download_limit().await?, None);

        t.set_config(Config::DownloadLimit, Some("200000")).await?;
        assert_eq!(t.download_limit().await?, Some(200000));

        t.set_config(Config::DownloadLimit, Some("20000")).await?;
        assert_eq!(t.download_limit().await?, Some(MIN_DOWNLOAD_LIMIT));

        t.set_config(Config::DownloadLimit, None).await?;
        assert_eq!(t.download_limit().await?, None);

        for val in &["0", "-1", "-100", "", "foo"] {
            t.set_config(Config::DownloadLimit, Some(val)).await?;
            assert_eq!(t.download_limit().await?, None);
        }

        Ok(())
    }

    #[async_std::test]
    async fn test_update_download_state() -> Result<()> {
        let t = TestContext::new_alice().await;
        let chat = t.create_chat_with_contact("Bob", "bob@example.org").await;

        let mut msg = Message::new(Viewtype::Text);
        msg.set_text(Some("Hi Bob".to_owned()));
        let msg_id = send_msg(&t, chat.id, &mut msg).await?;
        let msg = Message::load_from_db(&t, msg_id).await?;
        assert_eq!(msg.download_state(), DownloadState::Done);

        for s in &[
            DownloadState::Available,
            DownloadState::InProgress,
            DownloadState::Failure,
            DownloadState::Done,
        ] {
            msg_id.update_download_state(&t, *s).await?;
            let msg = Message::load_from_db(&t, msg_id).await?;
            assert_eq!(msg.download_state(), *s);
        }

        Ok(())
    }

    #[async_std::test]
    async fn test_partial_receive_imf() -> Result<()> {
        let t = TestContext::new_alice().await;

        let header =
            "Received: (Postfix, from userid 1000); Mon, 4 Dec 2006 14:51:39 +0100 (CET)\n\
             From: bob@example.com\n\
             To: alice@example.org\n\
             Subject: foo\n\
             Message-ID: <Mr.12345678901@example.com>\n\
             Chat-Version: 1.0\n\
             Date: Sun, 22 Mar 2020 22:37:57 +0000\
             Content-Type: text/plain";

        dc_receive_imf_inner(
            &t,
            header.as_bytes(),
            "INBOX",
            1,
            false,
            Some(100000),
            false,
        )
        .await?;
        let msg = t.get_last_msg().await;
        assert_eq!(msg.download_state(), DownloadState::Available);
        assert_eq!(msg.get_subject(), "foo");
        assert!(msg
            .get_text()
            .unwrap()
            .contains(&stock_str::partial_download_msg_body(&t, 100000).await));

        dc_receive_imf_inner(
            &t,
            format!("{}\n\n100k text...", header).as_bytes(),
            "INBOX",
            1,
            false,
            None,
            false,
        )
        .await?;
        let msg = t.get_last_msg().await;
        assert_eq!(msg.download_state(), DownloadState::Done);
        assert_eq!(msg.get_subject(), "foo");
        assert_eq!(msg.get_text(), Some("100k text...".to_string()));

        Ok(())
    }

    #[async_std::test]
    async fn test_partial_download_and_ephemeral() -> Result<()> {
        let t = TestContext::new_alice().await;
        let chat_id = t
            .create_chat_with_contact("bob", "bob@example.org")
            .await
            .id;
        chat_id
            .set_ephemeral_timer(&t, Timer::Enabled { duration: 60 })
            .await?;

        // download message from bob partially, this must not change the ephemeral timer
        dc_receive_imf_inner(
            &t,
            b"From: Bob <bob@example.org>\n\
                    To: Alice <alice@example.org>\n\
                    Chat-Version: 1.0\n\
                    Subject: subject\n\
                    Message-ID: <first@example.org>\n\
                    Date: Sun, 14 Nov 2021 00:10:00 +0000\
                    Content-Type: text/plain",
            "INBOX",
            1,
            false,
            Some(100000),
            false,
        )
        .await?;
        assert_eq!(
            chat_id.get_ephemeral_timer(&t).await?,
            Timer::Enabled { duration: 60 }
        );

        Ok(())
    }
}
