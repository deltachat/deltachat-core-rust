use std::{
    ops::{Deref, DerefMut},
    time::Duration,
};

use anyhow::{Context as _, Result};
use async_imap::Client as ImapClient;
use async_imap::Session as ImapSession;
use tokio::io::BufWriter;

use super::capabilities::Capabilities;
use super::session::Session;
use super::session::SessionStream;
use crate::context::Context;
use crate::login_param::build_tls;
use crate::net::connect_tcp;
use crate::socks::Socks5Config;

/// IMAP write and read timeout in seconds.
pub(crate) const IMAP_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug)]
pub(crate) struct Client {
    inner: ImapClient<Box<dyn SessionStream>>,
}

impl Deref for Client {
    type Target = ImapClient<Box<dyn SessionStream>>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for Client {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

/// Determine server capabilities.
///
/// If server supports ID capability, send our client ID.
async fn determine_capabilities(
    session: &mut ImapSession<Box<dyn SessionStream>>,
) -> Result<Capabilities> {
    let caps = session
        .capabilities()
        .await
        .context("CAPABILITY command error")?;
    let server_id = if caps.has_str("ID") {
        session.id([("name", Some("Delta Chat"))]).await?
    } else {
        None
    };
    let capabilities = Capabilities {
        can_idle: caps.has_str("IDLE"),
        can_move: caps.has_str("MOVE"),
        can_check_quota: caps.has_str("QUOTA"),
        can_condstore: caps.has_str("CONDSTORE"),
        server_id,
    };
    Ok(capabilities)
}

impl Client {
    pub(crate) async fn login(self, username: &str, password: &str) -> Result<Session> {
        let Client { inner, .. } = self;
        let mut session = inner
            .login(username, password)
            .await
            .map_err(|(err, _client)| err)?;
        let capabilities = determine_capabilities(&mut session).await?;
        Ok(Session::new(session, capabilities))
    }

    pub(crate) async fn authenticate(
        self,
        auth_type: &str,
        authenticator: impl async_imap::Authenticator,
    ) -> Result<Session> {
        let Client { inner, .. } = self;
        let mut session = inner
            .authenticate(auth_type, authenticator)
            .await
            .map_err(|(err, _client)| err)?;
        let capabilities = determine_capabilities(&mut session).await?;
        Ok(Session::new(session, capabilities))
    }

    pub async fn connect_secure(
        context: &Context,
        hostname: &str,
        port: u16,
        strict_tls: bool,
    ) -> Result<Self> {
        let tcp_stream = connect_tcp(context, hostname, port, IMAP_TIMEOUT, strict_tls).await?;
        let tls = build_tls(strict_tls);
        let tls_stream = tls.connect(hostname, tcp_stream).await?;
        let buffered_stream = BufWriter::new(tls_stream);
        let session_stream: Box<dyn SessionStream> = Box::new(buffered_stream);
        let mut client = ImapClient::new(session_stream);

        let _greeting = client
            .read_response()
            .await
            .context("failed to read greeting")??;

        Ok(Client { inner: client })
    }

    pub async fn connect_insecure(context: &Context, hostname: &str, port: u16) -> Result<Self> {
        let tcp_stream = connect_tcp(context, hostname, port, IMAP_TIMEOUT, false).await?;
        let buffered_stream = BufWriter::new(tcp_stream);
        let session_stream: Box<dyn SessionStream> = Box::new(buffered_stream);
        let mut client = ImapClient::new(session_stream);
        let _greeting = client
            .read_response()
            .await
            .context("failed to read greeting")??;

        Ok(Client { inner: client })
    }

    pub async fn connect_starttls(
        context: &Context,
        hostname: &str,
        port: u16,
        strict_tls: bool,
    ) -> Result<Self> {
        let tcp_stream = connect_tcp(context, hostname, port, IMAP_TIMEOUT, strict_tls).await?;

        // Run STARTTLS command and convert the client back into a stream.
        let mut client = ImapClient::new(tcp_stream);
        let _greeting = client
            .read_response()
            .await
            .context("failed to read greeting")??;
        client
            .run_command_and_check_ok("STARTTLS", None)
            .await
            .context("STARTTLS command failed")?;
        let tcp_stream = client.into_inner();

        let tls = build_tls(strict_tls);
        let tls_stream = tls
            .connect(hostname, tcp_stream)
            .await
            .context("STARTTLS upgrade failed")?;

        let buffered_stream = BufWriter::new(tls_stream);
        let session_stream: Box<dyn SessionStream> = Box::new(buffered_stream);
        let client = ImapClient::new(session_stream);

        Ok(Client { inner: client })
    }

    pub async fn connect_secure_socks5(
        context: &Context,
        domain: &str,
        port: u16,
        strict_tls: bool,
        socks5_config: Socks5Config,
    ) -> Result<Self> {
        let socks5_stream = socks5_config
            .connect(context, domain, port, IMAP_TIMEOUT, strict_tls)
            .await?;
        let tls = build_tls(strict_tls);
        let tls_stream = tls.connect(domain, socks5_stream).await?;
        let buffered_stream = BufWriter::new(tls_stream);
        let session_stream: Box<dyn SessionStream> = Box::new(buffered_stream);
        let mut client = ImapClient::new(session_stream);
        let _greeting = client
            .read_response()
            .await
            .context("failed to read greeting")??;

        Ok(Client { inner: client })
    }

    pub async fn connect_insecure_socks5(
        context: &Context,
        domain: &str,
        port: u16,
        socks5_config: Socks5Config,
    ) -> Result<Self> {
        let socks5_stream = socks5_config
            .connect(context, domain, port, IMAP_TIMEOUT, false)
            .await?;
        let buffered_stream = BufWriter::new(socks5_stream);
        let session_stream: Box<dyn SessionStream> = Box::new(buffered_stream);
        let mut client = ImapClient::new(session_stream);
        let _greeting = client
            .read_response()
            .await
            .context("failed to read greeting")??;

        Ok(Client { inner: client })
    }

    pub async fn connect_starttls_socks5(
        context: &Context,
        hostname: &str,
        port: u16,
        socks5_config: Socks5Config,
        strict_tls: bool,
    ) -> Result<Self> {
        let socks5_stream = socks5_config
            .connect(context, hostname, port, IMAP_TIMEOUT, strict_tls)
            .await?;

        // Run STARTTLS command and convert the client back into a stream.
        let mut client = ImapClient::new(socks5_stream);
        let _greeting = client
            .read_response()
            .await
            .context("failed to read greeting")??;
        client
            .run_command_and_check_ok("STARTTLS", None)
            .await
            .context("STARTTLS command failed")?;
        let socks5_stream = client.into_inner();

        let tls = build_tls(strict_tls);
        let tls_stream = tls
            .connect(hostname, socks5_stream)
            .await
            .context("STARTTLS upgrade failed")?;
        let buffered_stream = BufWriter::new(tls_stream);
        let session_stream: Box<dyn SessionStream> = Box::new(buffered_stream);
        let client = ImapClient::new(session_stream);

        Ok(Client { inner: client })
    }
}
