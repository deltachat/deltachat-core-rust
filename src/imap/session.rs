use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::time::Duration;

use async_imap::types::Mailbox;
use async_imap::Session as ImapSession;
use async_native_tls::TlsStream;
use fast_socks5::client::Socks5Stream;
use tokio::net::TcpStream;
use tokio_io_timeout::TimeoutStream;

use super::capabilities::Capabilities;

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

pub(crate) trait SessionStream:
    tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + Sync + std::fmt::Debug
{
    /// Change the read timeout on the session stream.
    fn set_read_timeout(&mut self, timeout: Option<Duration>);
}

impl SessionStream for TlsStream<Box<dyn SessionStream>> {
    fn set_read_timeout(&mut self, timeout: Option<Duration>) {
        self.get_mut().set_read_timeout(timeout);
    }
}
impl SessionStream for TlsStream<Pin<Box<TimeoutStream<TcpStream>>>> {
    fn set_read_timeout(&mut self, timeout: Option<Duration>) {
        self.get_mut().set_read_timeout(timeout);
    }
}
impl SessionStream for Pin<Box<TimeoutStream<TcpStream>>> {
    fn set_read_timeout(&mut self, timeout: Option<Duration>) {
        self.as_mut().set_read_timeout_pinned(timeout);
    }
}
impl SessionStream for Socks5Stream<TcpStream> {
    fn set_read_timeout(&mut self, _timeout: Option<Duration>) {
        // FIXME: build SOCKS streams on top of TimeoutStream, not directly TcpStream,
        // so we can set a read timeout for them.
    }
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
}
