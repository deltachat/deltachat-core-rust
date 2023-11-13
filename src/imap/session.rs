use std::ops::{Deref, DerefMut};

use async_imap::types::Mailbox;
use async_imap::Session as ImapSession;

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

    /// True if NOTIFY SET command was executed in this session.
    pub notify_set: bool,
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
            notify_set: false,
        }
    }

    pub fn can_idle(&self) -> bool {
        self.capabilities.can_idle
    }

    pub fn can_notify(&self) -> bool {
        self.capabilities.can_notify
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
}
