//! SMTP connection establishment.

use std::net::SocketAddr;

use anyhow::{bail, format_err, Context as _, Result};
use async_smtp::{SmtpClient, SmtpTransport};
use tokio::io::BufStream;

use crate::context::Context;
use crate::net::dns::{lookup_host_with_cache, update_connect_timestamp};
use crate::net::session::SessionBufStream;
use crate::net::tls::wrap_tls;
use crate::net::update_connection_history;
use crate::net::{connect_tcp_inner, connect_tls_inner};
use crate::provider::Socket;
use crate::socks::Socks5Config;
use crate::tools::time;

/// Returns TLS, STARTTLS or plaintext connection
/// using SOCKS5 or direct connection depending on the given configuration.
///
/// Connection is returned after skipping the welcome message
/// and is ready for sending commands. Because SMTP STARTTLS
/// does not send welcome message over TLS connection
/// after establishing it, welcome message is always ignored
/// to unify the result regardless of whether TLS or STARTTLS is used.
pub(crate) async fn connect_stream(
    context: &Context,
    host: &str,
    port: u16,
    strict_tls: bool,
    socks5_config: Option<Socks5Config>,
    security: Socket,
) -> Result<Box<dyn SessionBufStream>> {
    if let Some(socks5_config) = socks5_config {
        let stream = match security {
            Socket::Automatic => bail!("SMTP port security is not configured"),
            Socket::Ssl => {
                connect_secure_socks5(context, host, port, strict_tls, socks5_config.clone())
                    .await?
            }
            Socket::Starttls => {
                connect_starttls_socks5(context, host, port, strict_tls, socks5_config.clone())
                    .await?
            }
            Socket::Plain => {
                connect_insecure_socks5(context, host, port, socks5_config.clone()).await?
            }
        };
        Ok(stream)
    } else {
        let mut first_error = None;
        let load_cache = strict_tls && (security == Socket::Ssl || security == Socket::Starttls);

        for resolved_addr in lookup_host_with_cache(context, host, port, "smtp", load_cache).await?
        {
            let res = match security {
                Socket::Automatic => bail!("SMTP port security is not configured"),
                Socket::Ssl => connect_secure(resolved_addr, host, strict_tls).await,
                Socket::Starttls => connect_starttls(resolved_addr, host, strict_tls).await,
                Socket::Plain => connect_insecure(resolved_addr).await,
            };
            match res {
                Ok(stream) => {
                    let ip_addr = resolved_addr.ip().to_string();
                    if load_cache {
                        update_connect_timestamp(context, host, &ip_addr).await?;
                    }
                    update_connection_history(context, "smtp", host, port, &ip_addr, time())
                        .await?;
                    return Ok(stream);
                }
                Err(err) => {
                    warn!(context, "Failed to connect to {resolved_addr}: {err:#}.");
                    first_error.get_or_insert(err);
                }
            }
        }
        Err(first_error.unwrap_or_else(|| format_err!("no DNS resolution results for {host}")))
    }
}

/// Reads and ignores SMTP greeting.
///
/// This function is used to unify
/// TLS, STARTTLS and plaintext connection setup
/// by skipping the greeting in case of TLS
/// and STARTTLS connection setup.
async fn skip_smtp_greeting<R: tokio::io::AsyncBufReadExt + Unpin>(stream: &mut R) -> Result<()> {
    let mut line = String::with_capacity(512);
    loop {
        let read = stream.read_line(&mut line).await?;
        if read == 0 {
            bail!("Unexpected EOF while reading SMTP greeting.");
        }
        if line.starts_with("220- ") {
            continue;
        } else if line.starts_with("220 ") {
            return Ok(());
        } else {
            bail!("Unexpected greeting: {line:?}.");
        }
    }
}

async fn connect_secure_socks5(
    context: &Context,
    hostname: &str,
    port: u16,
    strict_tls: bool,
    socks5_config: Socks5Config,
) -> Result<Box<dyn SessionBufStream>> {
    let socks5_stream = socks5_config
        .connect(context, hostname, port, strict_tls)
        .await?;
    let tls_stream = wrap_tls(strict_tls, hostname, "smtp", socks5_stream).await?;
    let mut buffered_stream = BufStream::new(tls_stream);
    skip_smtp_greeting(&mut buffered_stream).await?;
    let session_stream: Box<dyn SessionBufStream> = Box::new(buffered_stream);
    Ok(session_stream)
}

async fn connect_starttls_socks5(
    context: &Context,
    hostname: &str,
    port: u16,
    strict_tls: bool,
    socks5_config: Socks5Config,
) -> Result<Box<dyn SessionBufStream>> {
    let socks5_stream = socks5_config
        .connect(context, hostname, port, strict_tls)
        .await?;

    // Run STARTTLS command and convert the client back into a stream.
    let client = SmtpClient::new().smtp_utf8(true);
    let transport = SmtpTransport::new(client, BufStream::new(socks5_stream)).await?;
    let tcp_stream = transport.starttls().await?.into_inner();
    let tls_stream = wrap_tls(strict_tls, hostname, "smtp", tcp_stream)
        .await
        .context("STARTTLS upgrade failed")?;
    let buffered_stream = BufStream::new(tls_stream);
    let session_stream: Box<dyn SessionBufStream> = Box::new(buffered_stream);
    Ok(session_stream)
}

async fn connect_insecure_socks5(
    context: &Context,
    hostname: &str,
    port: u16,
    socks5_config: Socks5Config,
) -> Result<Box<dyn SessionBufStream>> {
    let socks5_stream = socks5_config
        .connect(context, hostname, port, false)
        .await?;
    let mut buffered_stream = BufStream::new(socks5_stream);
    skip_smtp_greeting(&mut buffered_stream).await?;
    let session_stream: Box<dyn SessionBufStream> = Box::new(buffered_stream);
    Ok(session_stream)
}

async fn connect_secure(
    addr: SocketAddr,
    hostname: &str,
    strict_tls: bool,
) -> Result<Box<dyn SessionBufStream>> {
    let tls_stream = connect_tls_inner(addr, hostname, strict_tls, "smtp").await?;
    let mut buffered_stream = BufStream::new(tls_stream);
    skip_smtp_greeting(&mut buffered_stream).await?;
    let session_stream: Box<dyn SessionBufStream> = Box::new(buffered_stream);
    Ok(session_stream)
}

async fn connect_starttls(
    addr: SocketAddr,
    host: &str,
    strict_tls: bool,
) -> Result<Box<dyn SessionBufStream>> {
    let tcp_stream = connect_tcp_inner(addr).await?;

    // Run STARTTLS command and convert the client back into a stream.
    let client = async_smtp::SmtpClient::new().smtp_utf8(true);
    let transport = async_smtp::SmtpTransport::new(client, BufStream::new(tcp_stream)).await?;
    let tcp_stream = transport.starttls().await?.into_inner();
    let tls_stream = wrap_tls(strict_tls, host, "smtp", tcp_stream)
        .await
        .context("STARTTLS upgrade failed")?;

    let buffered_stream = BufStream::new(tls_stream);
    let session_stream: Box<dyn SessionBufStream> = Box::new(buffered_stream);
    Ok(session_stream)
}

async fn connect_insecure(addr: SocketAddr) -> Result<Box<dyn SessionBufStream>> {
    let tcp_stream = connect_tcp_inner(addr).await?;
    let mut buffered_stream = BufStream::new(tcp_stream);
    skip_smtp_greeting(&mut buffered_stream).await?;
    let session_stream: Box<dyn SessionBufStream> = Box::new(buffered_stream);
    Ok(session_stream)
}
