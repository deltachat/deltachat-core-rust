//! # SOCKS5 support.

use std::fmt;
use std::time::Duration;

use anyhow::Result;
pub use async_smtp::ServerAddress;
use tokio::{io, net::TcpStream};

use crate::context::Context;
use fast_socks5::client::Socks5Stream;

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
        target_addr: &ServerAddress,
        timeout: Option<Duration>,
    ) -> io::Result<Socks5Stream<TcpStream>> {
        self.to_async_smtp_socks5_config()
            .connect(target_addr, timeout)
            .await
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
