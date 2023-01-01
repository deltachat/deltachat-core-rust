use std::pin::Pin;
///! # Common network utilities.
use std::time::Duration;

use anyhow::{Context as _, Result};
use tokio::io::BufWriter;
use tokio::net::{TcpStream, ToSocketAddrs};
use tokio::time::timeout;
use tokio_io_timeout::TimeoutStream;

/// Returns a TCP connection with read/write timeouts set and
/// Nagle's algorithm disabled (TCP_NODELAY set) in favor of userspace buffering.
///
/// Doing our own buffering ensures that calling `.flush()` on the socket results
/// in immediate sending of the packet, which is important to reduce latency of
/// interactive protocols such as IMAP.
pub(crate) async fn connect_buffered(
    addr: impl ToSocketAddrs,
    timeout_val: Duration,
) -> Result<BufWriter<Pin<Box<TimeoutStream<TcpStream>>>>> {
    let tcp_stream = timeout(timeout_val, TcpStream::connect(addr))
        .await
        .context("connection timeout")?
        .context("connection failure")?;
    tcp_stream
        .set_nodelay(true)
        .context("cannot set TCP_NODELAY")?;

    let mut timeout_stream = TimeoutStream::new(tcp_stream);
    timeout_stream.set_write_timeout(Some(timeout_val));
    timeout_stream.set_read_timeout(Some(timeout_val));
    let pinned_stream = Box::pin(timeout_stream);

    let buffered_stream = BufWriter::new(pinned_stream);
    Ok(buffered_stream)
}
