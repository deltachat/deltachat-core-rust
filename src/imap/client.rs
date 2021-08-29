use std::{
    ops::{Deref, DerefMut},
    time::Duration,
};

use anyhow::{Context as _, Result};

use async_imap::Client as ImapClient;

use async_smtp::ServerAddress;
use async_std::net::{self, TcpStream};

use super::session::Session;
use crate::login_param::{dc_build_tls, Socks5Config};

use super::session::SessionStream;

/// IMAP write and read timeout in seconds.
const IMAP_TIMEOUT: u64 = 30;

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

impl Client {
    pub async fn login(self, username: &str, password: &str) -> Result<Session> {
        let Client { inner, .. } = self;
        let session = inner
            .login(username, password)
            .await
            .map_err(|(err, _client)| err)?;
        Ok(Session { inner: session })
    }

    pub async fn authenticate(
        self,
        auth_type: &str,
        authenticator: impl async_imap::Authenticator,
    ) -> Result<Session> {
        let Client { inner, .. } = self;
        let session = inner
            .authenticate(auth_type, authenticator)
            .await
            .map_err(|(err, _client)| err)?;
        Ok(Session { inner: session })
    }

    pub async fn connect_secure(
        addr: impl net::ToSocketAddrs,
        domain: &str,
        strict_tls: bool,
    ) -> Result<Self> {
        let stream = TcpStream::connect(addr).await?;
        let tls = dc_build_tls(strict_tls);
        let tls_stream: Box<dyn SessionStream> = Box::new(tls.connect(domain, stream).await?);
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
        let stream: Box<dyn SessionStream> = Box::new(TcpStream::connect(addr).await?);

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
        target_addr: &ServerAddress,
        strict_tls: bool,
        socks5_config: Socks5Config,
    ) -> Result<Self> {
        let socks5_stream: Box<dyn SessionStream> = Box::new(
            socks5_config
                .connect(target_addr, Some(Duration::from_secs(IMAP_TIMEOUT)))
                .await?,
        );

        let tls = dc_build_tls(strict_tls);
        let tls_stream: Box<dyn SessionStream> =
            Box::new(tls.connect(target_addr.host.clone(), socks5_stream).await?);
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
        target_addr: &ServerAddress,
        socks5_config: Socks5Config,
    ) -> Result<Self> {
        let socks5_stream: Box<dyn SessionStream> = Box::new(
            socks5_config
                .connect(target_addr, Some(Duration::from_secs(IMAP_TIMEOUT)))
                .await?,
        );

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

    pub async fn secure(self, domain: &str, strict_tls: bool) -> Result<Client> {
        if self.is_secure {
            Ok(self)
        } else {
            let Client { mut inner, .. } = self;
            let tls = dc_build_tls(strict_tls);
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
