use async_native_tls::TlsStream;
use fast_socks5::client::Socks5Stream;
use std::pin::Pin;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWrite, BufWriter};
use tokio_io_timeout::TimeoutStream;

pub(crate) trait SessionStream:
    AsyncRead + AsyncWrite + Unpin + Send + Sync + std::fmt::Debug
{
    /// Change the read timeout on the session stream.
    fn set_read_timeout(&mut self, timeout: Option<Duration>);
}

impl SessionStream for Box<dyn SessionStream> {
    fn set_read_timeout(&mut self, timeout: Option<Duration>) {
        self.as_mut().set_read_timeout(timeout);
    }
}
impl<T: SessionStream> SessionStream for TlsStream<T> {
    fn set_read_timeout(&mut self, timeout: Option<Duration>) {
        self.get_mut().set_read_timeout(timeout);
    }
}
impl<T: SessionStream> SessionStream for BufWriter<T> {
    fn set_read_timeout(&mut self, timeout: Option<Duration>) {
        self.get_mut().set_read_timeout(timeout);
    }
}
impl<T: AsyncRead + AsyncWrite + Send + Sync + std::fmt::Debug> SessionStream
    for Pin<Box<TimeoutStream<T>>>
{
    fn set_read_timeout(&mut self, timeout: Option<Duration>) {
        self.as_mut().set_read_timeout_pinned(timeout);
    }
}
impl<T: SessionStream> SessionStream for Socks5Stream<T> {
    fn set_read_timeout(&mut self, timeout: Option<Duration>) {
        self.get_socket_mut().set_read_timeout(timeout)
    }
}
