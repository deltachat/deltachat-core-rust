//! # Common network utilities.
use std::net::SocketAddr;
use std::pin::Pin;
use std::time::Duration;

use anyhow::{format_err, Context as _, Result};
use async_native_tls::TlsStream;
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio_io_timeout::TimeoutStream;

use crate::context::Context;
use crate::sql::Sql;
use crate::tools::time;

pub(crate) mod dns;
pub(crate) mod http;
pub(crate) mod session;
pub(crate) mod tls;

use dns::lookup_host_with_cache;
pub use http::{read_url, read_url_blob, Response as HttpResponse};
use tls::wrap_tls;

/// Connection, write and read timeout.
///
/// This constant should be more than the largest expected RTT.
pub(crate) const TIMEOUT: Duration = Duration::from_secs(60);

/// TTL for caches in seconds.
pub(crate) const CACHE_TTL: u64 = 30 * 24 * 60 * 60;

/// Removes connection history entries after `CACHE_TTL`.
pub(crate) async fn prune_connection_history(context: &Context) -> Result<()> {
    let now = time();
    context
        .sql
        .execute(
            "DELETE FROM connection_history
             WHERE ? > timestamp + ?",
            (now, CACHE_TTL),
        )
        .await?;
    Ok(())
}

pub(crate) async fn update_connection_history(
    context: &Context,
    alpn: &str,
    host: &str,
    port: u16,
    addr: &str,
    now: i64,
) -> Result<()> {
    context
        .sql
        .execute(
            "INSERT INTO connection_history (host, port, alpn, addr, timestamp)
             VALUES (?, ?, ?, ?, ?)
             ON CONFLICT (host, port, alpn, addr)
             DO UPDATE SET timestamp=excluded.timestamp",
            (host, port, alpn, addr, now),
        )
        .await?;
    Ok(())
}

/// Returns timestamp of the most recent successful connection
/// to the host and port for given protocol.
pub(crate) async fn load_connection_timestamp(
    sql: &Sql,
    alpn: &str,
    host: &str,
    port: u16,
    addr: Option<&str>,
) -> Result<Option<i64>> {
    let timestamp = sql
        .query_get_value(
            "SELECT timestamp FROM connection_history
             WHERE host = ?
               AND port = ?
               AND alpn = ?
               AND addr = IFNULL(?, addr)",
            (host, port, alpn, addr),
        )
        .await?;
    Ok(timestamp)
}

/// Returns a TCP connection stream with read/write timeouts set
/// and Nagle's algorithm disabled with `TCP_NODELAY`.
///
/// `TCP_NODELAY` ensures writing to the stream always results in immediate sending of the packet
/// to the network, which is important to reduce the latency of interactive protocols such as IMAP.
pub(crate) async fn connect_tcp_inner(
    addr: SocketAddr,
) -> Result<Pin<Box<TimeoutStream<TcpStream>>>> {
    let tcp_stream = timeout(TIMEOUT, TcpStream::connect(addr))
        .await
        .context("connection timeout")?
        .context("connection failure")?;

    // Disable Nagle's algorithm.
    tcp_stream.set_nodelay(true)?;

    let mut timeout_stream = TimeoutStream::new(tcp_stream);
    timeout_stream.set_write_timeout(Some(TIMEOUT));
    timeout_stream.set_read_timeout(Some(TIMEOUT));

    Ok(Box::pin(timeout_stream))
}

/// Attempts to establish TLS connection
/// given the result of the hostname to address resolution.
pub(crate) async fn connect_tls_inner(
    addr: SocketAddr,
    host: &str,
    strict_tls: bool,
    alpn: &[&str],
) -> Result<TlsStream<Pin<Box<TimeoutStream<TcpStream>>>>> {
    let tcp_stream = connect_tcp_inner(addr).await?;
    let tls_stream = wrap_tls(strict_tls, host, alpn, tcp_stream).await?;
    Ok(tls_stream)
}

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
    load_cache: bool,
) -> Result<Pin<Box<TimeoutStream<TcpStream>>>> {
    let mut first_error = None;

    for resolved_addr in lookup_host_with_cache(context, host, port, "", load_cache).await? {
        match connect_tcp_inner(resolved_addr).await {
            Ok(stream) => {
                return Ok(stream);
            }
            Err(err) => {
                warn!(
                    context,
                    "Failed to connect to {}: {:#}.", resolved_addr, err
                );
                first_error.get_or_insert(err);
            }
        }
    }

    Err(first_error.unwrap_or_else(|| format_err!("no DNS resolution results for {host}")))
}
