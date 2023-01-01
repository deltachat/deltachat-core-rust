///! # Common network utilities.
use std::pin::Pin;
use std::time::Duration;

use anyhow::{Context as _, Result};
use tokio::net::{TcpStream, ToSocketAddrs};
use tokio::time::timeout;
use tokio_io_timeout::TimeoutStream;

/// Returns a TCP connection stream with read/write timeouts set
/// and Nagle's algorithm disabled with `TCP_NODELAY`.
///
/// `TCP_NODELAY` ensures writing to the stream always results in immediate sending of the packet
/// to the network, which is important to reduce the latency of interactive protocols such as IMAP.
pub(crate) async fn connect_tcp(
    addr: impl ToSocketAddrs,
    timeout_val: Duration,
) -> Result<Pin<Box<TimeoutStream<TcpStream>>>> {
    let tcp_stream = timeout(timeout_val, TcpStream::connect(addr))
        .await
        .context("connection timeout")?
        .context("connection failure")?;

    // Disable Nagle's algorithm.
    tcp_stream.set_nodelay(true)?;

    let mut timeout_stream = TimeoutStream::new(tcp_stream);
    timeout_stream.set_write_timeout(Some(timeout_val));
    timeout_stream.set_read_timeout(Some(timeout_val));
    let pinned_stream = Box::pin(timeout_stream);

    Ok(pinned_stream)
}
