//! # Proxy support.
//!
//! Delta Chat supports SOCKS5 and Shadowsocks protocols.

use std::fmt;
use std::pin::Pin;

use anyhow::{format_err, Context as _, Result};
use fast_socks5::client::Socks5Stream;
use fast_socks5::util::target_addr::ToTargetAddr;
use fast_socks5::AuthenticationMethod;
use fast_socks5::Socks5Command;
use percent_encoding::{percent_encode, NON_ALPHANUMERIC};
use pin_project::pin_project;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tokio_io_timeout::TimeoutStream;

use crate::config::Config;
use crate::context::Context;
use crate::net::connect_tcp;
use crate::net::session::SessionStream;
use crate::sql::Sql;

/// Default SOCKS5 port according to [RFC 1928](https://tools.ietf.org/html/rfc1928).
pub const DEFAULT_SOCKS_PORT: u16 = 1080;

#[derive(Debug, Clone)]
pub struct ShadowsocksConfig {
    pub server_config: shadowsocks::config::ServerConfig,
}

impl PartialEq for ShadowsocksConfig {
    fn eq(&self, other: &Self) -> bool {
        self.server_config.to_url() == other.server_config.to_url()
    }
}

impl Eq for ShadowsocksConfig {}

/// Wrapper for Shadowsocks stream implementing
/// `Debug` and `SessionStream`.
///
/// Passes `AsyncRead` and `AsyncWrite` traits through.
#[pin_project]
pub(crate) struct ShadowsocksStream<S> {
    #[pin]
    pub(crate) stream: shadowsocks::ProxyClientStream<S>,
}

impl<S> std::fmt::Debug for ShadowsocksStream<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ShadowsocksStream")
    }
}

impl<S> AsyncRead for ShadowsocksStream<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        self.project().stream.poll_read(cx, buf)
    }
}

impl<S> AsyncWrite for ShadowsocksStream<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        self.project().stream.poll_write(cx, buf)
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        self.project().stream.poll_flush(cx)
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        self.project().stream.poll_shutdown(cx)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Socks5Config {
    pub host: String,
    pub port: u16,
    pub user_password: Option<(String, String)>,
}

impl Socks5Config {
    async fn connect(
        &self,
        context: &Context,
        target_host: &str,
        target_port: u16,
        load_dns_cache: bool,
    ) -> Result<Socks5Stream<Pin<Box<TimeoutStream<TcpStream>>>>> {
        let tcp_stream = connect_tcp(context, &self.host, self.port, load_dns_cache)
            .await
            .context("Failed to connect to SOCKS5 proxy")?;

        let authentication_method = if let Some((username, password)) = self.user_password.as_ref()
        {
            Some(AuthenticationMethod::Password {
                username: username.into(),
                password: password.into(),
            })
        } else {
            None
        };
        let mut socks_stream =
            Socks5Stream::use_stream(tcp_stream, authentication_method, Default::default()).await?;
        let target_addr = (target_host, target_port).to_target_addr()?;
        socks_stream
            .request(Socks5Command::TCPConnect, target_addr)
            .await?;

        Ok(socks_stream)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProxyConfig {
    Socks5(Socks5Config),

    Shadowsocks(ShadowsocksConfig),
}

impl ProxyConfig {
    /// Creates a new proxy configuration by parsing given proxy URL.
    fn from_url(url: &str) -> Result<Self> {
        let url = url::Url::parse(url).context("Cannot parse proxy URL")?;
        match url.scheme() {
            "ss" => {
                let server_config = shadowsocks::config::ServerConfig::from_url(url.as_str())?;
                let shadowsocks_config = ShadowsocksConfig { server_config };
                Ok(Self::Shadowsocks(shadowsocks_config))
            }

            // Because of `curl` convention,
            // `socks5` URL scheme may be expected to resolve domain names locally
            // with `socks5h` URL scheme meaning that hostnames are passed to the proxy.
            // Resolving hostnames locally is not supported
            // in Delta Chat when using a proxy
            // to prevent DNS leaks.
            // Because of this we do not distinguish
            // between `socks5` and `socks5h`.
            "socks5" => {
                let host = url
                    .host_str()
                    .context("socks5 URL has no host")?
                    .to_string();
                let port = url.port().unwrap_or(DEFAULT_SOCKS_PORT);
                let user_password = if let Some(password) = url.password() {
                    let username = percent_encoding::percent_decode_str(url.username())
                        .decode_utf8()
                        .context("SOCKS5 username is not a valid UTF-8")?
                        .to_string();
                    let password = percent_encoding::percent_decode_str(password)
                        .decode_utf8()
                        .context("SOCKS5 password is not a valid UTF-8")?
                        .to_string();
                    Some((username, password))
                } else {
                    None
                };
                let socks5_config = Socks5Config {
                    host,
                    port,
                    user_password,
                };
                Ok(Self::Socks5(socks5_config))
            }
            scheme => Err(format_err!("Unknown URL scheme {scheme:?}")),
        }
    }

    /// Migrates legacy `socks5_host`, `socks5_port`, `socks5_user` and `socks5_password`
    /// config into `proxy_url` if `proxy_url` is unset or empty.
    ///
    /// Unsets `socks5_host`, `socks5_port`, `socks5_user` and `socks5_password` in any case.
    async fn migrate_socks_config(sql: &Sql) -> Result<()> {
        if sql.get_raw_config("proxy_url").await?.is_none() {
            // Load legacy SOCKS5 settings.
            if let Some(host) = sql
                .get_raw_config("socks5_host")
                .await?
                .filter(|s| !s.is_empty())
            {
                let port: u16 = sql
                    .get_raw_config_int("socks5_port")
                    .await?
                    .unwrap_or(DEFAULT_SOCKS_PORT.into()) as u16;
                let user = sql.get_raw_config("socks5_user").await?.unwrap_or_default();
                let pass = sql
                    .get_raw_config("socks5_password")
                    .await?
                    .unwrap_or_default();

                let mut proxy_url = "socks5://".to_string();
                if !pass.is_empty() {
                    proxy_url += &percent_encode(user.as_bytes(), NON_ALPHANUMERIC).to_string();
                    proxy_url += ":";
                    proxy_url += &percent_encode(pass.as_bytes(), NON_ALPHANUMERIC).to_string();
                    proxy_url += "@";
                };
                proxy_url += &host;
                proxy_url += ":";
                proxy_url += &port.to_string();

                sql.set_raw_config("proxy_url", Some(&proxy_url)).await?;
            } else {
                sql.set_raw_config("proxy_url", Some("")).await?;
            }

            let socks5_enabled = sql.get_raw_config("socks5_enabled").await?;
            sql.set_raw_config("proxy_enabled", socks5_enabled.as_deref())
                .await?;
        }

        sql.set_raw_config("socks5_enabled", None).await?;
        sql.set_raw_config("socks5_host", None).await?;
        sql.set_raw_config("socks5_port", None).await?;
        sql.set_raw_config("socks5_user", None).await?;
        sql.set_raw_config("socks5_password", None).await?;
        Ok(())
    }

    /// Reads proxy configuration from the database.
    pub async fn load(context: &Context) -> Result<Option<Self>> {
        Self::migrate_socks_config(&context.sql)
            .await
            .context("Failed to migrate legacy SOCKS config")?;

        let enabled = context.get_config_bool(Config::ProxyEnabled).await?;
        if !enabled {
            return Ok(None);
        }

        let proxy_url = context
            .get_config(Config::ProxyUrl)
            .await?
            .unwrap_or_default();
        let proxy_url = proxy_url
            .split_once('\n')
            .map_or(proxy_url.clone(), |(first_url, _rest)| {
                first_url.to_string()
            });
        let proxy_config = Self::from_url(&proxy_url).context("Failed to parse proxy URL")?;
        Ok(Some(proxy_config))
    }

    /// If `load_dns_cache` is true, loads cached DNS resolution results.
    /// Use this only if the connection is going to be protected with TLS checks.
    pub async fn connect(
        &self,
        context: &Context,
        target_host: &str,
        target_port: u16,
        load_dns_cache: bool,
    ) -> Result<Box<dyn SessionStream>> {
        match self {
            ProxyConfig::Socks5(socks5_config) => {
                let socks5_stream = socks5_config
                    .connect(context, target_host, target_port, load_dns_cache)
                    .await?;
                Ok(Box::new(socks5_stream))
            }
            ProxyConfig::Shadowsocks(ShadowsocksConfig { server_config }) => {
                let shadowsocks_context = shadowsocks::context::Context::new_shared(
                    shadowsocks::config::ServerType::Local,
                );

                let tcp_stream = {
                    let server_addr = server_config.addr();
                    let host = server_addr.host();
                    let port = server_addr.port();
                    connect_tcp(context, &host, port, load_dns_cache)
                        .await
                        .context("Failed to connect to Shadowsocks proxy")?
                };

                let proxy_client_stream = shadowsocks::ProxyClientStream::from_stream(
                    shadowsocks_context,
                    tcp_stream,
                    server_config,
                    (target_host.to_string(), target_port),
                );
                let shadowsocks_stream = ShadowsocksStream {
                    stream: proxy_client_stream,
                };

                Ok(Box::new(shadowsocks_stream))
            }
        }
    }
}

impl fmt::Display for Socks5Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "host:{},port:{},user_password:{}",
            self.host,
            self.port,
            if let Some(user_password) = self.user_password.clone() {
                format!("user: {}, password: ***", user_password.0)
            } else {
                "user: None".to_string()
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::test_utils::TestContext;

    #[test]
    fn test_socks5_url() {
        let proxy_config = ProxyConfig::from_url("socks5://127.0.0.1:9050").unwrap();
        assert_eq!(
            proxy_config,
            ProxyConfig::Socks5(Socks5Config {
                host: "127.0.0.1".to_string(),
                port: 9050,
                user_password: None
            })
        );

        let proxy_config = ProxyConfig::from_url("socks5://foo:bar@127.0.0.1:9150").unwrap();
        assert_eq!(
            proxy_config,
            ProxyConfig::Socks5(Socks5Config {
                host: "127.0.0.1".to_string(),
                port: 9150,
                user_password: Some(("foo".to_string(), "bar".to_string()))
            })
        );

        let proxy_config = ProxyConfig::from_url("socks5://%66oo:b%61r@127.0.0.1:9150").unwrap();
        assert_eq!(
            proxy_config,
            ProxyConfig::Socks5(Socks5Config {
                host: "127.0.0.1".to_string(),
                port: 9150,
                user_password: Some(("foo".to_string(), "bar".to_string()))
            })
        );
    }

    #[test]
    fn test_shadowsocks_url() {
        // Example URL from <https://shadowsocks.org/doc/sip002.html>.
        let proxy_config =
            ProxyConfig::from_url("ss://YWVzLTEyOC1nY206dGVzdA@192.168.100.1:8888#Example1")
                .unwrap();
        assert!(matches!(proxy_config, ProxyConfig::Shadowsocks(_)));
    }

    #[test]
    fn test_invalid_proxy_url() {
        assert!(ProxyConfig::from_url("foobar://127.0.0.1:9050").is_err());
        assert!(ProxyConfig::from_url("abc").is_err());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_socks5_migration() -> Result<()> {
        let t = TestContext::new().await;

        // Test that config is migrated on attempt to load even if disabled.
        t.set_config(Config::Socks5Host, Some("127.0.0.1")).await?;
        t.set_config(Config::Socks5Port, Some("9050")).await?;

        let proxy_config = ProxyConfig::load(&t).await?;
        // Even though proxy is not enabled, config should be migrated.
        assert_eq!(proxy_config, None);

        assert_eq!(
            t.get_config(Config::ProxyUrl).await?.unwrap(),
            "socks5://127.0.0.1:9050"
        );
        Ok(())
    }

    // Test SOCKS5 setting migration if proxy was never configured.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_socks5_migration_unconfigured() -> Result<()> {
        let t = TestContext::new().await;

        // Try to load config to trigger migration.
        assert_eq!(ProxyConfig::load(&t).await?, None);

        assert_eq!(t.get_config(Config::ProxyEnabled).await?, None);
        assert_eq!(
            t.get_config(Config::ProxyUrl).await?.unwrap(),
            String::new()
        );
        Ok(())
    }

    // Test SOCKS5 setting migration if SOCKS5 host is empty.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_socks5_migration_empty() -> Result<()> {
        let t = TestContext::new().await;

        t.set_config(Config::Socks5Host, Some("")).await?;

        // Try to load config to trigger migration.
        assert_eq!(ProxyConfig::load(&t).await?, None);

        assert_eq!(t.get_config(Config::ProxyEnabled).await?, None);
        assert_eq!(
            t.get_config(Config::ProxyUrl).await?.unwrap(),
            String::new()
        );
        Ok(())
    }
}
