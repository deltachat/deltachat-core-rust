//! # Login parameters.

use std::fmt;

use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::constants::{DC_LP_AUTH_FLAGS, DC_LP_AUTH_NORMAL, DC_LP_AUTH_OAUTH2};
use crate::context::Context;
use crate::net::{ConnectionCandidate, ConnectionSecurity};
use crate::provider::{get_provider_by_id, Provider, Socket};
use crate::socks::Socks5Config;

/// Enumeration for values stored in `imap_certificate_checks`,
/// `smtp_certificate_checks`, `configured_imap_certificate_checks`
/// and `configured_smtp_certificate_checks`.
#[derive(Copy, Clone, Debug, Default, Display, FromPrimitive, ToPrimitive, PartialEq, Eq)]
#[repr(u32)]
#[strum(serialize_all = "snake_case")]
pub enum CertificateChecks {
    /// Same as AcceptInvalidCertificates if stored in the database
    /// as `configured_{imap,smtp}_certificate_checks`.
    ///
    /// Previous Delta Chat versions stored this in `configured_*`
    /// if Automatic configuration
    /// was selected, configuration with strict TLS checks failed
    /// and configuration without strict TLS checks succeeded.
    ///
    /// Currently Delta Chat stores only
    /// `Strict` or `AcceptInvalidCertificates` variants
    /// in `configured_*` settings.
    ///
    /// `Automatic` in `{imap,smtp}_certificate_checks`
    /// means that provider database setting should be taken.
    /// If there is no provider database setting for certificate checks,
    /// `Automatic` is the same as `Strict`.
    #[default]
    Automatic = 0,

    Strict = 1,

    /// Same as AcceptInvalidCertificates
    /// Previously known as AcceptInvalidHostnames, now deprecated.
    AcceptInvalidCertificates2 = 2,

    AcceptInvalidCertificates = 3,
}

/// Login parameters for a single server, either IMAP or SMTP
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnteredServerLoginParam {
    /// Server hostname or IP address.
    pub server: String,

    /// Server port.
    ///
    /// 0 if not specified.
    pub port: u16,

    /// Socket security.
    pub security: Socket,

    /// Username.
    ///
    /// Empty string if not specified.
    pub user: String,

    /// Password.
    pub password: String,
}

/// Login parameters entered by the user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnteredLoginParam {
    /// Email address.
    pub addr: String,

    /// IMAP settings.
    pub imap: EnteredServerLoginParam,

    /// SMTP settings.
    pub smtp: EnteredServerLoginParam,

    /// TLS options: whether to allow invalid certificates and/or
    /// invalid hostnames
    pub certificate_checks: CertificateChecks,

    pub socks5_config: Option<Socks5Config>,

    pub oauth2: bool,
}

impl EnteredLoginParam {
    /// Loads entered account settings.
    pub async fn load(context: &Context) -> Result<Self> {
        let sql = &context.sql;

        let addr = sql
            .get_raw_config("addr")
            .await?
            .unwrap_or_default()
            .trim()
            .to_string();

        let mail_server = sql.get_raw_config("mail_server").await?.unwrap_or_default();
        let mail_port = sql
            .get_raw_config_int("mail_port")
            .await?
            .unwrap_or_default();
        let mail_security = sql
            .get_raw_config_int("mail_security")
            .await?
            .and_then(num_traits::FromPrimitive::from_i32)
            .unwrap_or_default();
        let mail_user = sql.get_raw_config("mail_user").await?.unwrap_or_default();
        let mail_pw = sql.get_raw_config("mail_pw").await?.unwrap_or_default();

        // The setting is named `imap_certificate_checks`
        // for backwards compatibility,
        // but now it is a global setting applied to all protocols,
        // while `smtp_certificate_checks` is ignored.
        let certificate_checks = if let Some(certificate_checks) =
            sql.get_raw_config_int("imap_ceritifacte_checks").await?
        {
            num_traits::FromPrimitive::from_i32(certificate_checks).unwrap()
        } else {
            Default::default()
        };

        let send_server = sql.get_raw_config("send_server").await?.unwrap_or_default();
        let send_port = sql
            .get_raw_config_int("send_port")
            .await?
            .unwrap_or_default();
        let send_security = sql
            .get_raw_config_int("send_security")
            .await?
            .and_then(num_traits::FromPrimitive::from_i32)
            .unwrap_or_default();
        let send_user = sql.get_raw_config("send_user").await?.unwrap_or_default();
        let send_pw = sql.get_raw_config("send_pw").await?.unwrap_or_default();

        let server_flags = sql
            .get_raw_config_int("server_flags")
            .await?
            .unwrap_or_default();
        let oauth2 = matches!(server_flags & DC_LP_AUTH_FLAGS, DC_LP_AUTH_OAUTH2);

        let socks5_config = Socks5Config::from_database(&context.sql).await?;

        Ok(EnteredLoginParam {
            addr,
            imap: EnteredServerLoginParam {
                server: mail_server,
                port: mail_port as u16,
                security: mail_security,
                user: mail_user,
                password: mail_pw,
            },
            smtp: EnteredServerLoginParam {
                server: send_server,
                port: send_port as u16,
                security: send_security,
                user: send_user,
                password: send_pw,
            },
            certificate_checks,
            socks5_config,
            oauth2,
        })
    }
}

impl fmt::Display for EnteredLoginParam {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let unset = "0";
        let pw = "***";

        write!(
            f,
            "{} imap:{}:{}:{}:{}:{}:{} smtp:{}:{}:{}:{}:{}:{} cert_{}",
            unset_empty(&self.addr),
            unset_empty(&self.imap.user),
            if !self.imap.password.is_empty() {
                pw
            } else {
                unset
            },
            unset_empty(&self.imap.server),
            self.imap.port,
            self.imap.security,
            if self.oauth2 { "OAUTH2" } else { "AUTH_NORMAL" },
            unset_empty(&self.smtp.user),
            if !self.smtp.password.is_empty() {
                pw
            } else {
                unset
            },
            unset_empty(&self.smtp.server),
            self.smtp.port,
            self.smtp.security,
            if self.oauth2 { "OAUTH2" } else { "AUTH_NORMAL" },
            self.certificate_checks
        )
    }
}

fn unset_empty(s: &str) -> &str {
    if s.is_empty() {
        "unset"
    } else {
        s
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfiguredServerLoginParam {
    pub connection: ConnectionCandidate,

    /// Username.
    pub user: String,
}

/// Login parameters saved to the database
/// after successful configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfiguredLoginParam {
    /// `From:` address that was used at the time of configuration.
    pub addr: String,

    pub imap: Vec<ConfiguredServerLoginParam>,

    pub imap_password: String,

    pub smtp: Vec<ConfiguredServerLoginParam>,

    pub smtp_password: String,

    pub socks5_config: Option<Socks5Config>,

    pub provider: Option<&'static Provider>,

    /// TLS options: whether to allow invalid certificates and/or
    /// invalid hostnames
    pub certificate_checks: CertificateChecks,

    pub oauth2: bool,
}

impl fmt::Display for ConfiguredLoginParam {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} TODO", &self.addr,)
    }
}

impl ConfiguredLoginParam {
    /// Load configured account settings from the database.
    ///
    /// Returns `None` if account is not configured.
    pub async fn load(context: &Context) -> Result<Option<Self>> {
        let sql = &context.sql;

        if !context.get_config_bool(Config::Configured).await? {
            return Ok(None);
        }

        let addr = sql
            .get_raw_config("configured_addr")
            .await?
            .unwrap_or_default()
            .trim()
            .to_string();

        let certificate_checks: CertificateChecks = if let Some(certificate_checks) = sql
            .get_raw_config_int("configured_imap_certificate_checks")
            .await?
        {
            num_traits::FromPrimitive::from_i32(certificate_checks).unwrap()
        } else {
            Default::default()
        };

        let send_pw = context
            .get_config(Config::ConfiguredSendPw)
            .await?
            .context("SMTP password is not configured")?;
        let mail_pw = context
            .get_config(Config::ConfiguredMailPw)
            .await?
            .context("IMAP password is not configured")?;

        let server_flags = sql
            .get_raw_config_int("configured_server_flags")
            .await?
            .unwrap_or_default();
        let oauth2 = matches!(server_flags & DC_LP_AUTH_FLAGS, DC_LP_AUTH_OAUTH2);

        let provider = sql
            .get_raw_config("configured_provider")
            .await?
            .and_then(|provider_id| get_provider_by_id(&provider_id));

        let imap;
        let smtp;

        if let (Some(configured_mail_servers), Some(configured_imap_servers)) = (
            context.get_config(Config::ConfiguredMailServers).await?,
            context.get_config(Config::ConfiguredSendServers).await?,
        ) {
            // TODO
            imap = vec![];
            smtp = vec![];
        } else {
            // Load legacy settings storing a single IMAP and single SMTP server.
            let mail_server = sql
                .get_raw_config("configured_mail_server")
                .await?
                .unwrap_or_default();
            let mail_port = sql
                .get_raw_config_int("configured_mail_port")
                .await?
                .unwrap_or_default();

            let mail_user = sql
                .get_raw_config("configured_mail_user")
                .await?
                .unwrap_or_default();
            let mail_security: Socket = sql
                .get_raw_config_int("configured_mail_security")
                .await?
                .and_then(num_traits::FromPrimitive::from_i32)
                .unwrap_or_default();

            let send_server = context
                .get_config(Config::ConfiguredSendServer)
                .await?
                .context("SMTP server is not configured")?;
            let send_port = sql
                .get_raw_config_int("configured_send_port")
                .await?
                .unwrap_or_default();
            let send_user = sql
                .get_raw_config("configured_send_user")
                .await?
                .unwrap_or_default();
            let send_security: Socket = sql
                .get_raw_config_int("configured_send_security")
                .await?
                .and_then(num_traits::FromPrimitive::from_i32)
                .unwrap_or_default();

            imap = vec![ConfiguredServerLoginParam {
                connection: ConnectionCandidate {
                    host: mail_server,
                    port: mail_port as u16,
                    security: mail_security.try_into()?,
                },
                user: mail_user,
            }];
            smtp = vec![ConfiguredServerLoginParam {
                connection: ConnectionCandidate {
                    host: send_server,
                    port: send_port as u16,
                    security: send_security.try_into()?,
                },
                user: send_user,
            }];
        }

        let socks5_config = Socks5Config::from_database(&context.sql).await?;

        Ok(Some(ConfiguredLoginParam {
            addr,
            imap,
            imap_password: mail_pw,
            smtp,
            smtp_password: send_pw,
            certificate_checks,
            provider,
            socks5_config,
            oauth2,
        }))
    }

    /// Save this loginparam to the database.
    pub async fn save_as_configured_params(&self, context: &Context) -> Result<()> {
        let sql = &context.sql;

        context.set_primary_self_addr(&self.addr).await?;

        // TODO save all IMAP configs instead of just the first one.
        let imap = self.imap.first().context("No imap config")?;

        sql.set_raw_config("configured_mail_server", Some(&imap.connection.host))
            .await?;
        sql.set_raw_config_int("configured_mail_port", i32::from(imap.connection.port))
            .await?;
        sql.set_raw_config("configured_mail_user", Some(&imap.user))
            .await?;
        sql.set_raw_config("configured_mail_pw", Some(&self.imap_password))
            .await?;

        let imap_security = match imap.connection.security {
            ConnectionSecurity::Tls => Socket::Ssl,
            ConnectionSecurity::Starttls => Socket::Starttls,
            ConnectionSecurity::Plain => Socket::Plain,
        };
        sql.set_raw_config_int("configured_mail_security", imap_security as i32)
            .await?;
        sql.set_raw_config_int(
            "configured_imap_certificate_checks",
            self.certificate_checks as i32,
        )
        .await?;

        // TODO save all SMTP configs instead of just the first one.
        let smtp = self.smtp.first().context("No smtp config")?;

        sql.set_raw_config("configured_send_server", Some(&smtp.connection.host))
            .await?;
        sql.set_raw_config_int("configured_send_port", i32::from(smtp.connection.port))
            .await?;
        sql.set_raw_config("configured_send_user", Some(&smtp.user))
            .await?;
        sql.set_raw_config("configured_send_pw", Some(&self.smtp_password))
            .await?;

        let smtp_security = match smtp.connection.security {
            ConnectionSecurity::Tls => Socket::Ssl,
            ConnectionSecurity::Starttls => Socket::Starttls,
            ConnectionSecurity::Plain => Socket::Plain,
        };
        sql.set_raw_config_int("configured_send_security", smtp_security as i32)
            .await?;
        sql.set_raw_config_int(
            "configured_smtp_certificate_checks",
            self.certificate_checks as i32,
        )
        .await?;

        let server_flags = match self.oauth2 {
            true => DC_LP_AUTH_OAUTH2,
            false => DC_LP_AUTH_NORMAL,
        };
        sql.set_raw_config_int("configured_server_flags", server_flags)
            .await?;

        sql.set_raw_config(
            "configured_provider",
            self.provider.map(|provider| provider.id),
        )
        .await?;

        Ok(())
    }

    pub fn strict_tls(&self) -> bool {
        let user_strict_tls = match self.certificate_checks {
            CertificateChecks::Automatic => None,
            CertificateChecks::Strict => Some(true),
            CertificateChecks::AcceptInvalidCertificates
            | CertificateChecks::AcceptInvalidCertificates2 => Some(false),
        };
        let provider_strict_tls = self.provider.map(|provider| provider.opt.strict_tls);
        user_strict_tls
            .or(provider_strict_tls)
            .unwrap_or(self.socks5_config.is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    //use crate::test_utils::TestContext;

    #[test]
    fn test_certificate_checks_display() {
        use std::string::ToString;

        assert_eq!(
            "accept_invalid_certificates".to_string(),
            CertificateChecks::AcceptInvalidCertificates.to_string()
        );
    }

    /*
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_save_load_login_param() -> Result<()> {
        let t = TestContext::new().await;

        let param = LoginParam {
            addr: "alice@example.org".to_string(),
            imap: ServerLoginParam {
                server: "imap.example.com".to_string(),
                user: "alice".to_string(),
                password: "foo".to_string(),
                port: 123,
                security: Socket::Starttls,
                oauth2: false,
            },
            smtp: ServerLoginParam {
                server: "smtp.example.com".to_string(),
                user: "alice@example.org".to_string(),
                password: "bar".to_string(),
                port: 456,
                security: Socket::Ssl,
                oauth2: false,
            },
            provider: get_provider_by_id("example.com"),
            // socks5_config is not saved by `save_to_database`, using default value
            socks5_config: None,
            certificate_checks: CertificateChecks::Strict,
        };

        param.save_as_configured_params(&t).await?;
        let loaded = LoginParam::load_configured_params(&t).await?;
        assert_eq!(param, loaded);

        // Remove provider.
        let param = LoginParam {
            provider: None,
            ..param
        };
        param.save_as_configured_params(&t).await?;
        let loaded = LoginParam::load_configured_params(&t).await?;
        assert_eq!(param, loaded);
        Ok(())
    }
    */
}
