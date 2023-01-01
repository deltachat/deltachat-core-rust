use std::{
    ops::{Deref, DerefMut},
    time::Duration,
};

use anyhow::{Context as _, Result};

use async_imap::Client as ImapClient;
use async_imap::Session as ImapSession;

use tokio::net;

use super::capabilities::Capabilities;
use super::session::Session;
use crate::login_param::build_tls;
use crate::net::connect_buffered;
use crate::socks::Socks5Config;

use super::session::SessionStream;

/// IMAP write and read timeout in seconds.
pub(crate) const IMAP_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug)]
pub(crate) struct Client {
    is_secure: bool,
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

    pub async fn connect_secure(hostname: &str, port: u16, strict_tls: bool) -> Result<Self> {
        let buffered_stream = connect_buffered((hostname, port), IMAP_TIMEOUT).await?;

        let tls = build_tls(strict_tls);
        let tls_stream: Box<dyn SessionStream> =
            Box::new(tls.connect(hostname, buffered_stream).await?);
        let mut client = ImapClient::new(tls_stream);

        let _greeting = client
            .read_response()
            .await
            .context("failed to read greeting")?;

        Ok(Client {
            is_secure: true,
            inner: client,
        })
    }

    pub async fn connect_insecure(addr: impl net::ToSocketAddrs) -> Result<Self> {
        let buffered_stream = connect_buffered(addr, IMAP_TIMEOUT).await?;

        let stream: Box<dyn SessionStream> = Box::new(buffered_stream);

        let mut client = ImapClient::new(stream);
        let _greeting = client
            .read_response()
            .await
            .context("failed to read greeting")?;

        Ok(Client {
            is_secure: false,
            inner: client,
        })
    }

    pub async fn connect_secure_socks5(
        target_addr: impl net::ToSocketAddrs,
        domain: &str,
        strict_tls: bool,
        socks5_config: Socks5Config,
    ) -> Result<Self> {
        let socks5_stream: Box<dyn SessionStream> =
            Box::new(socks5_config.connect(target_addr, IMAP_TIMEOUT).await?);

        let tls = build_tls(strict_tls);
        let tls_stream: Box<dyn SessionStream> =
            Box::new(tls.connect(domain, socks5_stream).await?);
        let mut client = ImapClient::new(tls_stream);

        let _greeting = client
            .read_response()
            .await
            .context("failed to read greeting")?;

        Ok(Client {
            is_secure: true,
            inner: client,
        })
    }

    pub async fn connect_insecure_socks5(
        target_addr: impl net::ToSocketAddrs,
        socks5_config: Socks5Config,
    ) -> Result<Self> {
        let socks5_stream: Box<dyn SessionStream> =
            Box::new(socks5_config.connect(target_addr, IMAP_TIMEOUT).await?);

        let mut client = ImapClient::new(socks5_stream);
        let _greeting = client
            .read_response()
            .await
            .context("failed to read greeting")?;

        Ok(Client {
            is_secure: false,
            inner: client,
        })
    }

    pub async fn secure(self, domain: &str, strict_tls: bool) -> Result<Self> {
        if self.is_secure {
            Ok(self)
        } else {
            let Client { mut inner, .. } = self;
            let tls = build_tls(strict_tls);
            inner.run_command_and_check_ok("STARTTLS", None).await?;

            let stream = inner.into_inner();
            let ssl_stream = tls.connect(domain, stream).await?;
            let boxed: Box<dyn SessionStream> = Box::new(ssl_stream);

            Ok(Client {
                is_secure: true,
                inner: ImapClient::new(boxed),
            })
        }
    }
}
