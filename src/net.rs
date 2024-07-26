//! # Common network utilities.
use std::net::SocketAddr;
use std::pin::Pin;
use std::time::Duration;

use anyhow::{format_err, Context as _, Result};
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio_io_timeout::TimeoutStream;

use crate::context::Context;
use crate::tools::time;

pub(crate) mod dns;
pub(crate) mod http;
pub(crate) mod session;
pub(crate) mod tls;

use dns::lookup_host_with_cache;
pub use http::{read_url, read_url_blob, Response as HttpResponse};

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
///
/// If `load_cache` is true, may use cached DNS results.
/// Because the cache may be poisoned with incorrect results by networks hijacking DNS requests,
/// this option should only be used when connection is authenticated,
/// for example using TLS.
/// If TLS is not used or invalid TLS certificates are allowed,
/// this option should be disabled.
pub(crate) async fn connect_tcp(
    context: &Context,
    host: &str,
    port: u16,
    timeout_val: Duration,
    load_cache: bool,
) -> Result<Pin<Box<TimeoutStream<TcpStream>>>> {
    let mut tcp_stream = None;
    let mut last_error = None;

    for resolved_addr in
        lookup_host_with_cache(context, host, port, timeout_val, load_cache).await?
    {
        match connect_tcp_inner(resolved_addr, timeout_val).await {
            Ok(stream) => {
                tcp_stream = Some(stream);

                // Update timestamp of this cached entry
                // or insert a new one if cached entry does not exist.
                //
                // This increases priority of existing cached entries
                // and copies fallback addresses from build-in cache
                // into database cache on successful use.
                //
                // Unlike built-in cache,
                // database cache is used even if DNS
                // resolver returns a non-empty
                // (but potentially incorrect and unusable) result.
                context
                    .sql
                    .execute(
                        "INSERT INTO dns_cache (hostname, address, timestamp)
                         VALUES (?, ?, ?)
                         ON CONFLICT (hostname, address)
                         DO UPDATE SET timestamp=excluded.timestamp",
                        (host, resolved_addr.ip().to_string(), time()),
                    )
                    .await?;
                break;
            }
            Err(err) => {
                warn!(
                    context,
                    "Failed to connect to {}: {:#}.", resolved_addr, err
                );
                last_error = Some(err);
            }
        }
    }

    let tcp_stream = match tcp_stream {
        Some(tcp_stream) => tcp_stream,
        None => {
            return Err(
                last_error.unwrap_or_else(|| format_err!("no DNS resolution results for {host}"))
            );
        }
    };

    // Disable Nagle's algorithm.
    tcp_stream.set_nodelay(true)?;

    let mut timeout_stream = TimeoutStream::new(tcp_stream);
    timeout_stream.set_write_timeout(Some(timeout_val));
    timeout_stream.set_read_timeout(Some(timeout_val));
    let pinned_stream = Box::pin(timeout_stream);

    Ok(pinned_stream)
}
