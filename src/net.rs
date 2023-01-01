///! # Common network utilities.
use std::pin::Pin;
use std::time::Duration;

use anyhow::{Context as _, Result};
use tokio::net::{TcpStream, ToSocketAddrs};
use tokio::time::timeout;
use tokio_io_timeout::TimeoutStream;

/// Returns a TCP connection with read/write timeouts set.
pub(crate) async fn connect_tcp(
    addr: impl ToSocketAddrs,
    timeout_val: Duration,
) -> Result<Pin<Box<TimeoutStream<TcpStream>>>> {
    let tcp_stream = timeout(timeout_val, TcpStream::connect(addr))
        .await
        .context("connection timeout")?
        .context("connection failure")?;

    let mut timeout_stream = TimeoutStream::new(tcp_stream);
    timeout_stream.set_write_timeout(Some(timeout_val));
    timeout_stream.set_read_timeout(Some(timeout_val));
    let pinned_stream = Box::pin(timeout_stream);

    Ok(pinned_stream)
}
