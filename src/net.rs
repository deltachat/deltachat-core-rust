//! # Common network utilities.
use std::net::SocketAddr;
use std::pin::Pin;
use std::time::Duration;

use anyhow::{format_err, Context as _, Result};
use async_native_tls::TlsStream;
use tokio::io::BufStream;
use tokio::io::BufWriter;
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio_io_timeout::TimeoutStream;

use crate::context::Context;

pub(crate) mod dns;
pub(crate) mod http;
pub(crate) mod session;
pub(crate) mod tls;

use dns::lookup_host_with_cache;
pub use http::{read_url, read_url_blob, Response as HttpResponse};
use tls::wrap_tls;

/// Returns a TCP connection stream with read/write timeouts set
/// and Nagle's algorithm disabled with `TCP_NODELAY`.
///
/// `TCP_NODELAY` ensures writing to the stream always results in immediate sending of the packet
/// to the network, which is important to reduce the latency of interactive protocols such as IMAP.
async fn connect_tcp_inner(
    addr: SocketAddr,
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

    Ok(Box::pin(timeout_stream))
}

/// Attempts to establish TLS connection
/// given the result of the hostname to address resolution.
async fn connect_tls_inner(
    addr: SocketAddr,
    timeout_val: Duration,
    host: &str,
    strict_tls: bool,
    alpn: &str,
) -> Result<TlsStream<Pin<Box<TimeoutStream<TcpStream>>>>> {
    let tcp_stream = connect_tcp_inner(addr, timeout_val).await?;
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
    timeout_val: Duration,
    load_cache: bool,
) -> Result<Pin<Box<TimeoutStream<TcpStream>>>> {
    let mut first_error = None;

    for resolved_addr in
        lookup_host_with_cache(context, host, port, timeout_val, load_cache).await?
    {
        match connect_tcp_inner(resolved_addr, timeout_val).await {
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

pub(crate) async fn connect_tls(
    context: &Context,
    host: &str,
    port: u16,
    timeout_val: Duration,
    strict_tls: bool,
    alpn: &str,
) -> Result<TlsStream<Pin<Box<TimeoutStream<TcpStream>>>>> {
    let mut first_error = None;

    for resolved_addr in
        lookup_host_with_cache(context, host, port, timeout_val, strict_tls).await?
    {
        match connect_tls_inner(resolved_addr, timeout_val, host, strict_tls, alpn).await {
            Ok(tls_stream) => {
                if strict_tls {
                    dns::update_connect_timestamp(context, host, &resolved_addr.ip().to_string())
                        .await?;
                }
                return Ok(tls_stream);
            }
            Err(err) => {
                warn!(context, "Failed to connect to {resolved_addr}: {err:#}.");
                first_error.get_or_insert(err);
            }
        }
    }

    Err(first_error.unwrap_or_else(|| format_err!("no DNS resolution results for {host}")))
}

async fn connect_starttls_imap_inner(
    addr: SocketAddr,
    host: &str,
    timeout_val: Duration,
    strict_tls: bool,
) -> Result<TlsStream<Pin<Box<TimeoutStream<TcpStream>>>>> {
    let tcp_stream = connect_tcp_inner(addr, timeout_val).await?;

    // Run STARTTLS command and convert the client back into a stream.
    let buffered_tcp_stream = BufWriter::new(tcp_stream);
    let mut client = async_imap::Client::new(buffered_tcp_stream);
    let _greeting = client
        .read_response()
        .await
        .context("failed to read greeting")??;
    client
        .run_command_and_check_ok("STARTTLS", None)
        .await
        .context("STARTTLS command failed")?;
    let buffered_tcp_stream = client.into_inner();
    let tcp_stream = buffered_tcp_stream.into_inner();

    let tls_stream = wrap_tls(strict_tls, host, "imap", tcp_stream)
        .await
        .context("STARTTLS upgrade failed")?;

    Ok(tls_stream)
}

pub(crate) async fn connect_starttls_imap(
    context: &Context,
    host: &str,
    port: u16,
    timeout_val: Duration,
    strict_tls: bool,
) -> Result<TlsStream<Pin<Box<TimeoutStream<TcpStream>>>>> {
    let mut first_error = None;

    for resolved_addr in
        lookup_host_with_cache(context, host, port, timeout_val, strict_tls).await?
    {
        match connect_starttls_imap_inner(resolved_addr, host, timeout_val, strict_tls).await {
            Ok(tls_stream) => {
                if strict_tls {
                    dns::update_connect_timestamp(context, host, &resolved_addr.ip().to_string())
                        .await?;
                }
                return Ok(tls_stream);
            }
            Err(err) => {
                warn!(context, "Failed to connect to {resolved_addr}: {err:#}.");
                first_error.get_or_insert(err);
            }
        }
    }

    Err(first_error.unwrap_or_else(|| format_err!("no DNS resolution results for {host}")))
}

async fn connect_starttls_smtp_inner(
    addr: SocketAddr,
    host: &str,
    timeout_val: Duration,
    strict_tls: bool,
) -> Result<TlsStream<Pin<Box<TimeoutStream<TcpStream>>>>> {
    let tcp_stream = connect_tcp_inner(addr, timeout_val).await?;

    // Run STARTTLS command and convert the client back into a stream.
    let client = async_smtp::SmtpClient::new().smtp_utf8(true);
    let transport = async_smtp::SmtpTransport::new(client, BufStream::new(tcp_stream)).await?;
    let tcp_stream = transport.starttls().await?.into_inner();
    let tls_stream = wrap_tls(strict_tls, host, "smtp", tcp_stream)
        .await
        .context("STARTTLS upgrade failed")?;
    Ok(tls_stream)
}

pub(crate) async fn connect_starttls_smtp(
    context: &Context,
    host: &str,
    port: u16,
    timeout_val: Duration,
    strict_tls: bool,
) -> Result<TlsStream<Pin<Box<TimeoutStream<TcpStream>>>>> {
    let mut first_error = None;

    for resolved_addr in
        lookup_host_with_cache(context, host, port, timeout_val, strict_tls).await?
    {
        match connect_starttls_smtp_inner(resolved_addr, host, timeout_val, strict_tls).await {
            Ok(tls_stream) => {
                if strict_tls {
                    dns::update_connect_timestamp(context, host, &resolved_addr.ip().to_string())
                        .await?;
                }
                return Ok(tls_stream);
            }
            Err(err) => {
                warn!(context, "Failed to connect to {resolved_addr}: {err:#}.");
                first_error.get_or_insert(err);
            }
        }
    }

    Err(first_error.unwrap_or_else(|| format_err!("no DNS resolution results for {host}")))
}
