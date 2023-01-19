///! # Common network utilities.
use std::net::SocketAddr;
use std::pin::Pin;
use std::time::Duration;

use anyhow::{Context as _, Result};
use tokio::net::{lookup_host, TcpStream};
use tokio::time::timeout;
use tokio_io_timeout::TimeoutStream;

use crate::context::Context;

async fn connect_tcp_inner(addr: SocketAddr, timeout_val: Duration) -> Result<TcpStream> {
    let tcp_stream = timeout(timeout_val, TcpStream::connect(addr))
        .await
        .context("connection timeout")?
        .context("connection failure")?;
    Ok(tcp_stream)
}

/// Returns a TCP connection stream with read/write timeouts set
/// and Nagle's algorithm disabled with `TCP_NODELAY`.
///
/// `TCP_NODELAY` ensures writing to the stream always results in immediate sending of the packet
/// to the network, which is important to reduce the latency of interactive protocols such as IMAP.
pub(crate) async fn connect_tcp(
    context: &Context,
    host: &str,
    port: u16,
    timeout_val: Duration,
) -> Result<Pin<Box<TimeoutStream<TcpStream>>>> {
    let mut tcp_stream = None;
    for resolved_addr in lookup_host((host, port)).await? {
        info!(
            context,
            "Resolved {}:{} into {}.", host, port, &resolved_addr
        );
        match connect_tcp_inner(resolved_addr, timeout_val).await {
            Ok(stream) => {
                tcp_stream = Some(stream);
                break;
            }
            Err(err) => {
                warn!(
                    context,
                    "Failed to connect to {}: {:#}.", resolved_addr, err
                );
            }
        }
    }
    let tcp_stream =
        tcp_stream.with_context(|| format!("failed to connect to {}:{}", host, port))?;

    // Disable Nagle's algorithm.
    tcp_stream.set_nodelay(true)?;

    let mut timeout_stream = TimeoutStream::new(tcp_stream);
    timeout_stream.set_write_timeout(Some(timeout_val));
    timeout_stream.set_read_timeout(Some(timeout_val));
    let pinned_stream = Box::pin(timeout_stream);

    Ok(pinned_stream)
}
