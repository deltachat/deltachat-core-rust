//! # SOCKS5 support.

use std::fmt;
use std::pin::Pin;
use std::time::Duration;

use anyhow::{Context as _, Result};
pub use async_smtp::ServerAddress;
use tokio::net::{self, TcpStream};
use tokio::time::timeout;
use tokio_io_timeout::TimeoutStream;

use crate::context::Context;
use fast_socks5::client::{Config, Socks5Stream};
use fast_socks5::AuthenticationMethod;

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Socks5Config {
    pub host: String,
    pub port: u16,
    pub user_password: Option<(String, String)>,
}

impl Socks5Config {
    /// Reads SOCKS5 configuration from the database.
    pub async fn from_database(context: &Context) -> Result<Option<Self>> {
        let sql = &context.sql;

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

    pub async fn connect(
        &self,
        target_addr: impl net::ToSocketAddrs,
        timeout_val: Duration,
    ) -> Result<Socks5Stream<Pin<Box<TimeoutStream<TcpStream>>>>> {
        let tcp_stream = timeout(timeout_val, TcpStream::connect(target_addr))
            .await
            .context("connection timeout")?
            .context("connection failure")?;
        let mut timeout_stream = TimeoutStream::new(tcp_stream);
        timeout_stream.set_write_timeout(Some(timeout_val));
        timeout_stream.set_read_timeout(Some(timeout_val));
        let timeout_stream = Box::pin(timeout_stream);

        let authentication_method = if let Some((username, password)) = self.user_password.as_ref()
        {
            Some(AuthenticationMethod::Password {
                username: username.into(),
                password: password.into(),
            })
        } else {
            None
        };
        let socks_stream =
            Socks5Stream::use_stream(timeout_stream, authentication_method, Config::default())
                .await?;

        Ok(socks_stream)
    }

    pub fn to_async_smtp_socks5_config(&self) -> async_smtp::smtp::Socks5Config {
        async_smtp::smtp::Socks5Config {
            host: self.host.clone(),
            port: self.port,
            user_password: self.user_password.clone(),
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
