//! SMTP connection establishment.

use anyhow::{bail, Context as _, Result};
use async_smtp::{SmtpClient, SmtpTransport};
use tokio::io::BufStream;

use crate::context::Context;
use crate::net::session::SessionBufStream;
use crate::net::tls::wrap_tls;
use crate::net::{connect_starttls_smtp, connect_tcp, connect_tls};
use crate::provider::Socket;
use crate::socks::Socks5Config;

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
    domain: &str,
    port: u16,
    strict_tls: bool,
    socks5_config: Option<Socks5Config>,
    security: Socket,
) -> Result<Box<dyn SessionBufStream>> {
    let stream = if let Some(socks5_config) = socks5_config {
        match security {
            Socket::Automatic => bail!("SMTP port security is not configured"),
            Socket::Ssl => {
                connect_secure_socks5(context, domain, port, strict_tls, socks5_config.clone())
                    .await?
            }
            Socket::Starttls => {
                connect_starttls_socks5(context, domain, port, strict_tls, socks5_config.clone())
                    .await?
            }
            Socket::Plain => {
                connect_insecure_socks5(context, domain, port, socks5_config.clone()).await?
            }
        }
    } else {
        match security {
            Socket::Automatic => bail!("SMTP port security is not configured"),
            Socket::Ssl => connect_secure(context, domain, port, strict_tls).await?,
            Socket::Starttls => connect_starttls(context, domain, port, strict_tls).await?,
            Socket::Plain => connect_insecure(context, domain, port).await?,
        }
    };
    Ok(stream)
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
    context: &Context,
    hostname: &str,
    port: u16,
    strict_tls: bool,
) -> Result<Box<dyn SessionBufStream>> {
    let tls_stream = connect_tls(context, hostname, port, strict_tls, "smtp").await?;
    let mut buffered_stream = BufStream::new(tls_stream);
    skip_smtp_greeting(&mut buffered_stream).await?;
    let session_stream: Box<dyn SessionBufStream> = Box::new(buffered_stream);
    Ok(session_stream)
}

async fn connect_starttls(
    context: &Context,
    hostname: &str,
    port: u16,
    strict_tls: bool,
) -> Result<Box<dyn SessionBufStream>> {
    let tls_stream = connect_starttls_smtp(context, hostname, port, strict_tls).await?;

    let buffered_stream = BufStream::new(tls_stream);
    let session_stream: Box<dyn SessionBufStream> = Box::new(buffered_stream);
    Ok(session_stream)
}

async fn connect_insecure(
    context: &Context,
    hostname: &str,
    port: u16,
) -> Result<Box<dyn SessionBufStream>> {
    let tcp_stream = connect_tcp(context, hostname, port, false).await?;
    let mut buffered_stream = BufStream::new(tcp_stream);
    skip_smtp_greeting(&mut buffered_stream).await?;
    let session_stream: Box<dyn SessionBufStream> = Box::new(buffered_stream);
    Ok(session_stream)
}
