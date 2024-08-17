use std::net::SocketAddr;
use std::ops::{Deref, DerefMut};

use anyhow::{bail, format_err, Context as _, Result};
use async_imap::Client as ImapClient;
use async_imap::Session as ImapSession;
use fast_socks5::client::Socks5Stream;
use tokio::io::BufWriter;

use super::capabilities::Capabilities;
use super::session::Session;
use crate::context::Context;
use crate::net::dns::{lookup_host_with_cache, update_connect_timestamp};
use crate::net::session::SessionStream;
use crate::net::tls::wrap_tls;
use crate::net::update_connection_history;
use crate::net::{connect_tcp_inner, connect_tls_inner};
use crate::provider::Socket;
use crate::socks::Socks5Config;
use crate::tools::time;

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

/// Converts port number to ALPN list.
fn alpn(port: u16) -> &'static [&'static str] {
    if port == 993 {
        // Do not request ALPN on standard port.
        &[]
    } else {
        &["imap"]
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
        can_metadata: caps.has_str("METADATA"),
        can_push: caps.has_str("XDELTAPUSH"),
        is_chatmail: caps.has_str("XCHATMAIL"),
        server_id,
    };
    Ok(capabilities)
}

impl Client {
    fn new(stream: Box<dyn SessionStream>) -> Self {
        Self {
            inner: ImapClient::new(stream),
        }
    }

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

    pub async fn connect(
        context: &Context,
        host: &str,
        port: u16,
        strict_tls: bool,
        socks5_config: Option<Socks5Config>,
        security: Socket,
    ) -> Result<Self> {
        if let Some(socks5_config) = socks5_config {
            let client = match security {
                Socket::Automatic => bail!("IMAP port security is not configured"),
                Socket::Ssl => {
                    Client::connect_secure_socks5(context, host, port, strict_tls, socks5_config)
                        .await?
                }
                Socket::Starttls => {
                    Client::connect_starttls_socks5(context, host, port, socks5_config, strict_tls)
                        .await?
                }
                Socket::Plain => {
                    Client::connect_insecure_socks5(context, host, port, socks5_config).await?
                }
            };
            Ok(client)
        } else {
            let mut first_error = None;
            let load_cache =
                strict_tls && (security == Socket::Ssl || security == Socket::Starttls);
            for resolved_addr in
                lookup_host_with_cache(context, host, port, "imap", load_cache).await?
            {
                let res = match security {
                    Socket::Automatic => bail!("IMAP port security is not configured"),
                    Socket::Ssl => Client::connect_secure(resolved_addr, host, strict_tls).await,
                    Socket::Starttls => {
                        Client::connect_starttls(resolved_addr, host, strict_tls).await
                    }
                    Socket::Plain => Client::connect_insecure(resolved_addr).await,
                };
                match res {
                    Ok(client) => {
                        let ip_addr = resolved_addr.ip().to_string();
                        if load_cache {
                            update_connect_timestamp(context, host, &ip_addr).await?;
                        }
                        update_connection_history(context, "imap", host, port, &ip_addr, time())
                            .await?;
                        return Ok(client);
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

    async fn connect_secure(addr: SocketAddr, hostname: &str, strict_tls: bool) -> Result<Self> {
        let tls_stream = connect_tls_inner(addr, hostname, strict_tls, alpn(addr.port())).await?;
        let buffered_stream = BufWriter::new(tls_stream);
        let session_stream: Box<dyn SessionStream> = Box::new(buffered_stream);
        let mut client = Client::new(session_stream);
        let _greeting = client
            .read_response()
            .await
            .context("failed to read greeting")??;
        Ok(client)
    }

    async fn connect_insecure(addr: SocketAddr) -> Result<Self> {
        let tcp_stream = connect_tcp_inner(addr).await?;
        let buffered_stream = BufWriter::new(tcp_stream);
        let session_stream: Box<dyn SessionStream> = Box::new(buffered_stream);
        let mut client = Client::new(session_stream);
        let _greeting = client
            .read_response()
            .await
            .context("failed to read greeting")??;
        Ok(client)
    }

    async fn connect_starttls(addr: SocketAddr, host: &str, strict_tls: bool) -> Result<Self> {
        let tcp_stream = connect_tcp_inner(addr).await?;

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

        let tls_stream = wrap_tls(strict_tls, host, &[], tcp_stream)
            .await
            .context("STARTTLS upgrade failed")?;

        let buffered_stream = BufWriter::new(tls_stream);
        let session_stream: Box<dyn SessionStream> = Box::new(buffered_stream);
        let client = Client::new(session_stream);
        Ok(client)
    }

    async fn connect_secure_socks5(
        context: &Context,
        domain: &str,
        port: u16,
        strict_tls: bool,
        socks5_config: Socks5Config,
    ) -> Result<Self> {
        let socks5_stream = socks5_config
            .connect(context, domain, port, strict_tls)
            .await?;
        let tls_stream = wrap_tls(strict_tls, domain, alpn(port), socks5_stream).await?;
        let buffered_stream = BufWriter::new(tls_stream);
        let session_stream: Box<dyn SessionStream> = Box::new(buffered_stream);
        let mut client = Client::new(session_stream);
        let _greeting = client
            .read_response()
            .await
            .context("failed to read greeting")??;
        Ok(client)
    }

    async fn connect_insecure_socks5(
        context: &Context,
        domain: &str,
        port: u16,
        socks5_config: Socks5Config,
    ) -> Result<Self> {
        let socks5_stream = socks5_config.connect(context, domain, port, false).await?;
        let buffered_stream = BufWriter::new(socks5_stream);
        let session_stream: Box<dyn SessionStream> = Box::new(buffered_stream);
        let mut client = Client::new(session_stream);
        let _greeting = client
            .read_response()
            .await
            .context("failed to read greeting")??;
        Ok(client)
    }

    async fn connect_starttls_socks5(
        context: &Context,
        hostname: &str,
        port: u16,
        socks5_config: Socks5Config,
        strict_tls: bool,
    ) -> Result<Self> {
        let socks5_stream = socks5_config
            .connect(context, hostname, port, strict_tls)
            .await?;

        // Run STARTTLS command and convert the client back into a stream.
        let buffered_socks5_stream = BufWriter::new(socks5_stream);
        let mut client = ImapClient::new(buffered_socks5_stream);
        let _greeting = client
            .read_response()
            .await
            .context("failed to read greeting")??;
        client
            .run_command_and_check_ok("STARTTLS", None)
            .await
            .context("STARTTLS command failed")?;
        let buffered_socks5_stream = client.into_inner();
        let socks5_stream: Socks5Stream<_> = buffered_socks5_stream.into_inner();

        let tls_stream = wrap_tls(strict_tls, hostname, &[], socks5_stream)
            .await
            .context("STARTTLS upgrade failed")?;
        let buffered_stream = BufWriter::new(tls_stream);
        let session_stream: Box<dyn SessionStream> = Box::new(buffered_stream);
        let client = Client::new(session_stream);
        Ok(client)
    }
}
