use std::ops::{Deref, DerefMut};

use async_imap::Session as ImapSession;
use async_native_tls::TlsStream;
use fast_socks5::client::Socks5Stream;
use tokio::net::TcpStream;

#[derive(Debug)]
pub(crate) struct Session {
    pub(super) inner: ImapSession<Box<dyn SessionStream>>,
}

pub(crate) trait SessionStream:
    tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + Sync + std::fmt::Debug
{
}

impl SessionStream for TlsStream<Box<dyn SessionStream>> {}
impl SessionStream for TlsStream<TcpStream> {}
impl SessionStream for TcpStream {}
impl SessionStream for Socks5Stream<TcpStream> {}

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
    pub fn idle(self) -> async_imap::extensions::idle::Handle<Box<dyn SessionStream>> {
        let Session { inner } = self;
        inner.idle()
    }
}
