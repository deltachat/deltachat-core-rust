//! # SOCKS5 support.

use std::fmt;
use std::pin::Pin;
use std::time::Duration;

use anyhow::Result;
use fast_socks5::client::{Config, Socks5Stream};
use fast_socks5::util::target_addr::ToTargetAddr;
use fast_socks5::AuthenticationMethod;
use fast_socks5::Socks5Command;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use tokio::net::TcpStream;
use tokio_io_timeout::TimeoutStream;

use crate::context::Context;
use crate::net::connect_tcp;
use crate::sql::Sql;

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Socks5Config {
    pub host: String,
    pub port: u16,
    pub user_password: Option<(String, String)>,
}

impl Socks5Config {
    /// Reads SOCKS5 configuration from the database.
    pub async fn from_database(sql: &Sql) -> Result<Option<Self>> {
        let enabled = sql.get_raw_config_bool("socks5_enabled").await?;
        if enabled {
            let host = sql.get_raw_config("socks5_host").await?.unwrap_or_default();
            let port: u16 = sql
                .get_raw_config_int("socks5_port")
                .await?
                .unwrap_or_default() as u16;
            let user = sql.get_raw_config("socks5_user").await?.unwrap_or_default();
            let password = sql
                .get_raw_config("socks5_password")
                .await?
                .unwrap_or_default();

            let socks5_config = Self {
                host,
                port,
                user_password: if !user.is_empty() {
                    Some((user, password))
                } else {
                    None
                },
            };
            Ok(Some(socks5_config))
        } else {
            Ok(None)
        }
    }

    /// Converts SOCKS5 configuration into URL.
    pub fn to_url(&self) -> String {
        // `socks5h` means that hostname is resolved into address by the proxy
        // and DNS requests should not leak.
        let mut url = "socks5h://".to_string();
        if let Some((username, password)) = &self.user_password {
            let username_urlencoded = utf8_percent_encode(username, NON_ALPHANUMERIC).to_string();
            let password_urlencoded = utf8_percent_encode(password, NON_ALPHANUMERIC).to_string();
            url += &format!("{username_urlencoded}:{password_urlencoded}@");
        }
        url += &format!("{}:{}", self.host, self.port);
        url
    }

    /// If `load_dns_cache` is true, loads cached DNS resolution results.
    /// Use this only if the connection is going to be protected with TLS checks.
    pub async fn connect(
        &self,
        context: &Context,
        target_host: &str,
        target_port: u16,
        timeout_val: Duration,
        load_dns_cache: bool,
    ) -> Result<Socks5Stream<Pin<Box<TimeoutStream<TcpStream>>>>> {
        let tcp_stream =
            connect_tcp(context, &self.host, self.port, timeout_val, load_dns_cache).await?;

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
            Socks5Stream::use_stream(tcp_stream, authentication_method, Config::default()).await?;
        let target_addr = (target_host, target_port).to_target_addr()?;
        socks_stream
            .request(Socks5Command::TCPConnect, target_addr)
            .await?;

        Ok(socks_stream)
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

    #[test]
    fn test_socks5h_url() {
        let config = Socks5Config {
            host: "127.0.0.1".to_string(),
            port: 9050,
            user_password: None,
        };
        assert_eq!(config.to_url(), "socks5h://127.0.0.1:9050");

        let config = Socks5Config {
            host: "example.org".to_string(),
            port: 1080,
            user_password: Some(("root".to_string(), "toor".to_string())),
        };
        assert_eq!(config.to_url(), "socks5h://root:toor@example.org:1080");

        let config = Socks5Config {
            host: "example.org".to_string(),
            port: 1080,
            user_password: Some(("root".to_string(), "foo/?\\@".to_string())),
        };
        assert_eq!(
            config.to_url(),
            "socks5h://root:foo%2F%3F%5C%40@example.org:1080"
        );
    }
}
