//! # Download large messages manually.

use anyhow::{anyhow, Result};
use deltachat_derive::{FromSql, ToSql};
use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::context::Context;
use crate::imap::{Imap, ImapActionResult};
use crate::job::{self, Action, Job, Status};
use crate::message::{Message, MsgId};
use crate::param::Params;
use crate::{job_try, EventType};
use std::cmp::max;

/// Download limits should not be used below `MIN_DOWNLOAD_LIMIT`.
///
/// Some messages as non-delivery-reports (NDN) or read-receipts (MDN)
/// need to be downloaded completely to handle them correctly,
/// eg. to assign them to the correct chat.
/// As these messages are typically small,
/// they're catched by `MIN_DOWNLOAD_LIMIT`.
const MIN_DOWNLOAD_LIMIT: u32 = 32768;

/// If `delete_server_after` is set to small timeouts (eg. "at once"),
/// the user might have no chance to actually download a message.
/// `MIN_DELETE_SERVER_AFTER` increases the timeout in this case.
pub const MIN_DELETE_SERVER_AFTER: i64 = 48 * 60 * 60;

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
    // Gets validated download limit or 0 for "no limit".
    pub(crate) async fn get_download_limit(&self) -> Result<u32> {
        let download_limit = self.get_config_int(Config::DownloadLimit).await?;
        if download_limit <= 0 {
            Ok(0)
        } else {
            Ok(max(MIN_DOWNLOAD_LIMIT, download_limit as u32))
        }
    }
}

impl MsgId {
    /// Adds the job `Action::DownloadMsg` to download a message.
    pub async fn download_full(self, context: &Context) -> Result<()> {
        let msg = Message::load_from_db(context, self).await?;
        match msg.get_download_state() {
            DownloadState::Done => return Err(anyhow!("Nothing to download.")),
            DownloadState::InProgress => return Err(anyhow!("Download already in progress.")),
            DownloadState::Available | DownloadState::Failure => {
                self.update_download_state(context, DownloadState::InProgress)
                    .await?;
                job::add(
                    context,
                    Job::new(Action::DownloadMsg, self.to_u32(), Params::new(), 0),
                )
                .await;
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
    /// Gets the download state of the message.
    pub fn get_download_state(&self) -> DownloadState {
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

        let (_, error_cnt) = self
            .fetch_many_msgs(context, folder, vec![uid], false, false)
            .await;
        if error_cnt > 0 {
            return ImapActionResult::Failed;
        }

        ImapActionResult::Success
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chat::send_msg;
    use crate::constants::Viewtype;
    use crate::dc_receive_imf::dc_receive_imf_inner;
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

        assert_eq!(t.get_download_limit().await?, 0);

        t.set_config(Config::DownloadLimit, Some("200000")).await?;
        assert_eq!(t.get_download_limit().await?, 200000);

        t.set_config(Config::DownloadLimit, Some("20000")).await?;
        assert_eq!(t.get_download_limit().await?, MIN_DOWNLOAD_LIMIT);

        t.set_config(Config::DownloadLimit, None).await?;
        assert_eq!(t.get_download_limit().await?, 0);

        for val in &["0", "-1", "-100", "", "foo"] {
            t.set_config(Config::DownloadLimit, Some(val)).await?;
            assert_eq!(t.get_download_limit().await?, 0);
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
        assert_eq!(msg.get_download_state(), DownloadState::Done);

        for s in &[
            DownloadState::Available,
            DownloadState::InProgress,
            DownloadState::Failure,
            DownloadState::Done,
        ] {
            msg_id.update_download_state(&t, *s).await?;
            let msg = Message::load_from_db(&t, msg_id).await?;
            assert_eq!(msg.get_download_state(), *s);
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
            &header.as_bytes(),
            "INBOX",
            1,
            false,
            Some(100000),
            false,
        )
        .await?;
        let msg = t.get_last_msg().await;
        assert_eq!(msg.get_download_state(), DownloadState::Available);
        assert_eq!(msg.get_subject(), "foo");

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
        assert_eq!(msg.get_download_state(), DownloadState::Done);
        assert_eq!(msg.get_subject(), "foo");
        assert_eq!(msg.get_text(), Some("100k text...".to_string()));

        Ok(())
    }
}
