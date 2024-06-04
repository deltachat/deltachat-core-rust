use std::cmp;
use std::collections::BTreeMap;
use std::ops::{Deref, DerefMut};

use anyhow::{Context as _, Result};
use async_imap::types::Mailbox;
use async_imap::Session as ImapSession;
use futures::TryStreamExt;

use crate::constants::DC_FETCH_EXISTING_MSGS_COUNT;
use crate::imap::capabilities::Capabilities;
use crate::net::session::SessionStream;

/// Prefetch:
/// - Message-ID to check if we already have the message.
/// - In-Reply-To and References to check if message is a reply to chat message.
/// - Chat-Version to check if a message is a chat message
/// - Autocrypt-Setup-Message to check if a message is an autocrypt setup message,
///   not necessarily sent by Delta Chat.
const PREFETCH_FLAGS: &str = "(UID INTERNALDATE RFC822.SIZE BODY.PEEK[HEADER.FIELDS (\
                              MESSAGE-ID \
                              DATE \
                              X-MICROSOFT-ORIGINAL-MESSAGE-ID \
                              FROM \
                              IN-REPLY-TO REFERENCES \
                              CHAT-VERSION \
                              AUTOCRYPT-SETUP-MESSAGE\
                              )])";

#[derive(Debug)]
pub(crate) struct Session {
    pub(super) inner: ImapSession<Box<dyn SessionStream>>,

    pub capabilities: Capabilities,

    /// Selected folder name.
    pub selected_folder: Option<String>,

    /// Mailbox structure returned by IMAP server.
    pub selected_mailbox: Option<Mailbox>,

    pub selected_folder_needs_expunge: bool,

    /// True if currently selected folder has new messages.
    ///
    /// Should be false if no folder is currently selected.
    pub new_mail: bool,
}

impl Deref for Session {
    type Target = ImapSession<Box<dyn SessionStream>>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for Session {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl Session {
    pub(crate) fn new(
        inner: ImapSession<Box<dyn SessionStream>>,
        capabilities: Capabilities,
    ) -> Self {
        Self {
            inner,
            capabilities,
            selected_folder: None,
            selected_mailbox: None,
            selected_folder_needs_expunge: false,
            new_mail: false,
        }
    }

    pub fn can_idle(&self) -> bool {
        self.capabilities.can_idle
    }

    pub fn can_move(&self) -> bool {
        self.capabilities.can_move
    }

    pub fn can_check_quota(&self) -> bool {
        self.capabilities.can_check_quota
    }

    pub fn can_condstore(&self) -> bool {
        self.capabilities.can_condstore
    }

    pub fn can_metadata(&self) -> bool {
        self.capabilities.can_metadata
    }

    pub fn can_push(&self) -> bool {
        self.capabilities.can_push
    }

    // Returns true if IMAP server has `XCHATMAIL` capability.
    pub fn is_chatmail(&self) -> bool {
        self.capabilities.is_chatmail
    }

    /// Returns the names of all folders on the IMAP server.
    pub async fn list_folders(&mut self) -> Result<Vec<async_imap::types::Name>> {
        let list = self.list(Some(""), Some("*")).await?.try_collect().await?;
        Ok(list)
    }

    /// Prefetch all messages greater than or equal to `uid_next`. Returns a list of fetch results
    /// in the order of ascending delivery time to the server (INTERNALDATE).
    pub(crate) async fn prefetch(
        &mut self,
        uid_next: u32,
    ) -> Result<Vec<(u32, async_imap::types::Fetch)>> {
        // fetch messages with larger UID than the last one seen
        let set = format!("{uid_next}:*");
        let mut list = self
            .uid_fetch(set, PREFETCH_FLAGS)
            .await
            .context("IMAP could not fetch")?;

        let mut msgs = BTreeMap::new();
        while let Some(msg) = list.try_next().await? {
            if let Some(msg_uid) = msg.uid {
                // If the mailbox is not empty, results always include
                // at least one UID, even if last_seen_uid+1 is past
                // the last UID in the mailbox.  It happens because
                // uid:* is interpreted the same way as *:uid.
                // See <https://tools.ietf.org/html/rfc3501#page-61> for
                // standard reference. Therefore, sometimes we receive
                // already seen messages and have to filter them out.
                if msg_uid >= uid_next {
                    msgs.insert((msg.internal_date(), msg_uid), msg);
                }
            }
        }

        Ok(msgs.into_iter().map(|((_, uid), msg)| (uid, msg)).collect())
    }

    /// Like prefetch(), but not for new messages but existing ones (the DC_FETCH_EXISTING_MSGS_COUNT newest messages)
    pub(crate) async fn prefetch_existing_msgs(
        &mut self,
    ) -> Result<Vec<(u32, async_imap::types::Fetch)>> {
        let exists: i64 = {
            let mailbox = self.selected_mailbox.as_ref().context("no mailbox")?;
            mailbox.exists.into()
        };

        // Fetch last DC_FETCH_EXISTING_MSGS_COUNT (100) messages.
        // Sequence numbers are sequential. If there are 1000 messages in the inbox,
        // we can fetch the sequence numbers 900-1000 and get the last 100 messages.
        let first = cmp::max(1, exists - DC_FETCH_EXISTING_MSGS_COUNT + 1);
        let set = format!("{first}:{exists}");
        let mut list = self
            .fetch(&set, PREFETCH_FLAGS)
            .await
            .context("IMAP Could not fetch")?;

        let mut msgs = BTreeMap::new();
        while let Some(msg) = list.try_next().await? {
            if let Some(msg_uid) = msg.uid {
                msgs.insert((msg.internal_date(), msg_uid), msg);
            }
        }

        Ok(msgs.into_iter().map(|((_, uid), msg)| (uid, msg)).collect())
    }
}
