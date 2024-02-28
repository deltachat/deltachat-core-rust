use std::ops::{Deref, DerefMut};

use anyhow::Result;
use async_imap::types::Mailbox;
use async_imap::Session as ImapSession;
use futures::TryStreamExt;

use crate::imap::capabilities::Capabilities;
use crate::net::session::SessionStream;

#[derive(Debug)]
pub(crate) struct Session {
    pub(super) inner: ImapSession<Box<dyn SessionStream>>,

    pub capabilities: Capabilities,

    /// Selected folder name.
    pub selected_folder: Option<String>,

    /// Mailbox structure returned by IMAP server.
    pub selected_mailbox: Option<Mailbox>,

    pub selected_folder_needs_expunge: bool,
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

    /// Returns the names of all folders on the IMAP server.
    pub async fn list_folders(&mut self) -> Result<Vec<async_imap::types::Name>> {
        let list = self.list(Some(""), Some("*")).await?.try_collect().await?;
        Ok(list)
    }

    /// Like fetch_after(), but not for new messages but existing ones (the DC_FETCH_EXISTING_MSGS_COUNT newest messages)
    async fn prefetch_existing_msgs(&mut self) -> Result<Vec<(u32, async_imap::types::Fetch)>> {
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
