//! # Login parameters.

use std::fmt;

use anyhow::{ensure, Result};
use async_native_tls::Certificate;
pub use async_smtp::ServerAddress;
use once_cell::sync::Lazy;

use crate::constants::{DC_LP_AUTH_FLAGS, DC_LP_AUTH_NORMAL, DC_LP_AUTH_OAUTH2};
use crate::provider::{get_provider_by_id, Provider};
use crate::socks::Socks5Config;
use crate::{context::Context, provider::Socket};

#[derive(Copy, Clone, Debug, Display, FromPrimitive, ToPrimitive, PartialEq, Eq)]
#[repr(u32)]
#[strum(serialize_all = "snake_case")]
pub enum CertificateChecks {
    /// Same as AcceptInvalidCertificates unless overridden by
    /// `strict_tls` setting in provider database.
    Automatic = 0,

    Strict = 1,

    /// Same as AcceptInvalidCertificates
    /// Previously known as AcceptInvalidHostnames, now deprecated.
    AcceptInvalidCertificates2 = 2,

    AcceptInvalidCertificates = 3,
}

impl Default for CertificateChecks {
    fn default() -> Self {
        Self::Automatic
    }
}

/// Login parameters for a single server, either IMAP or SMTP
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct ServerLoginParam {
    pub server: String,
    pub user: String,
    pub password: String,
    pub port: u16,
    pub security: Socket,
    pub oauth2: bool,

    /// TLS options: whether to allow invalid certificates and/or
    /// invalid hostnames
    pub certificate_checks: CertificateChecks,
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct LoginParam {
    pub addr: String,
    pub imap: ServerLoginParam,
    pub smtp: ServerLoginParam,
    pub provider: Option<&'static Provider>,
    pub socks5_config: Option<Socks5Config>,
}

impl LoginParam {
    /// Load entered (candidate) account settings
    pub async fn load_candidate_params(context: &Context) -> Result<Self> {
        let mut param = Self::load_candidate_params_unchecked(context).await?;
        ensure!(!param.addr.is_empty(), "Missing email address.");

        // Only check for IMAP password, SMTP password is an "advanced" setting.
        ensure!(!param.imap.password.is_empty(), "Missing (IMAP) password.");
        if param.smtp.password.is_empty() {
            param.smtp.password = param.imap.password.clone()
        }
        Ok(param)
    }

    /// Load entered (candidate) account settings without validation.
    ///
    /// This will result in a potentially invalid [`LoginParam`] struct as the values are
    /// not validated.  Only use this if you want to show this directly to the user e.g. in
    /// [`Context::get_info`].
    pub async fn load_candidate_params_unchecked(context: &Context) -> Result<Self> {
        LoginParam::from_database(context, "").await
    }

    /// Load configured (working) account settings
    pub async fn load_configured_params(context: &Context) -> Result<Self> {
        LoginParam::from_database(context, "configured_").await
    }

    /// Read the login parameters from the database.
    async fn from_database(context: &Context, prefix: &str) -> Result<Self> {
        let sql = &context.sql;

        let key = &format!("{prefix}addr");
        let addr = sql
            .get_raw_config(key)
            .await?
            .unwrap_or_default()
            .trim()
            .to_string();

        let key = &format!("{prefix}mail_server");
        let mail_server = sql.get_raw_config(key).await?.unwrap_or_default();

        let key = &format!("{prefix}mail_port");
        let mail_port = sql.get_raw_config_int(key).await?.unwrap_or_default();

        let key = &format!("{prefix}mail_user");
        let mail_user = sql.get_raw_config(key).await?.unwrap_or_default();

        let key = &format!("{prefix}mail_pw");
        let mail_pw = sql.get_raw_config(key).await?.unwrap_or_default();

        let key = &format!("{prefix}mail_security");
        let mail_security = sql
            .get_raw_config_int(key)
            .await?
            .and_then(num_traits::FromPrimitive::from_i32)
            .unwrap_or_default();

        let key = &format!("{prefix}imap_certificate_checks");
        let imap_certificate_checks =
            if let Some(certificate_checks) = sql.get_raw_config_int(key).await? {
                num_traits::FromPrimitive::from_i32(certificate_checks).unwrap()
            } else {
                Default::default()
            };

        let key = &format!("{prefix}send_server");
        let send_server = sql.get_raw_config(key).await?.unwrap_or_default();

        let key = &format!("{prefix}send_port");
        let send_port = sql.get_raw_config_int(key).await?.unwrap_or_default();

        let key = &format!("{prefix}send_user");
        let send_user = sql.get_raw_config(key).await?.unwrap_or_default();

        let key = &format!("{prefix}send_pw");
        let send_pw = sql.get_raw_config(key).await?.unwrap_or_default();

        let key = &format!("{prefix}send_security");
        let send_security = sql
            .get_raw_config_int(key)
            .await?
            .and_then(num_traits::FromPrimitive::from_i32)
            .unwrap_or_default();

        let key = &format!("{prefix}smtp_certificate_checks");
        let smtp_certificate_checks =
            if let Some(certificate_checks) = sql.get_raw_config_int(key).await? {
                num_traits::FromPrimitive::from_i32(certificate_checks).unwrap_or_default()
            } else {
                Default::default()
            };

        let key = &format!("{prefix}server_flags");
        let server_flags = sql.get_raw_config_int(key).await?.unwrap_or_default();
        let oauth2 = matches!(server_flags & DC_LP_AUTH_FLAGS, DC_LP_AUTH_OAUTH2);

        let key = &format!("{prefix}provider");
        let provider = sql
            .get_raw_config(key)
            .await?
            .and_then(|provider_id| get_provider_by_id(&provider_id));

        let socks5_config = Socks5Config::from_database(context).await?;

        Ok(LoginParam {
            addr,
            imap: ServerLoginParam {
                server: mail_server,
                user: mail_user,
                password: mail_pw,
                port: mail_port as u16,
                security: mail_security,
                oauth2,
                certificate_checks: imap_certificate_checks,
            },
            smtp: ServerLoginParam {
                server: send_server,
                user: send_user,
                password: send_pw,
                port: send_port as u16,
                security: send_security,
                oauth2,
                certificate_checks: smtp_certificate_checks,
            },
            provider,
            socks5_config,
        })
    }

    /// Save this loginparam to the database.
    pub async fn save_as_configured_params(&self, context: &Context) -> Result<()> {
        let prefix = "configured_";
        let sql = &context.sql;

        context.set_primary_self_addr(&self.addr).await?;

        let key = &format!("{prefix}mail_server");
        sql.set_raw_config(key, Some(&self.imap.server)).await?;

        let key = &format!("{prefix}mail_port");
        sql.set_raw_config_int(key, i32::from(self.imap.port))
            .await?;

        let key = &format!("{prefix}mail_user");
        sql.set_raw_config(key, Some(&self.imap.user)).await?;

        let key = &format!("{prefix}mail_pw");
        sql.set_raw_config(key, Some(&self.imap.password)).await?;

        let key = &format!("{prefix}mail_security");
        sql.set_raw_config_int(key, self.imap.security as i32)
            .await?;

        let key = &format!("{prefix}imap_certificate_checks");
        sql.set_raw_config_int(key, self.imap.certificate_checks as i32)
            .await?;

        let key = &format!("{prefix}send_server");
        sql.set_raw_config(key, Some(&self.smtp.server)).await?;

        let key = &format!("{prefix}send_port");
        sql.set_raw_config_int(key, i32::from(self.smtp.port))
            .await?;

        let key = &format!("{prefix}send_user");
        sql.set_raw_config(key, Some(&self.smtp.user)).await?;

        let key = &format!("{prefix}send_pw");
        sql.set_raw_config(key, Some(&self.smtp.password)).await?;

        let key = &format!("{prefix}send_security");
        sql.set_raw_config_int(key, self.smtp.security as i32)
            .await?;

        let key = &format!("{prefix}smtp_certificate_checks");
        sql.set_raw_config_int(key, self.smtp.certificate_checks as i32)
            .await?;

        // The OAuth2 flag is either set for both IMAP and SMTP or not at all.
        let key = &format!("{prefix}server_flags");
        let server_flags = match self.imap.oauth2 {
            true => DC_LP_AUTH_OAUTH2,
            false => DC_LP_AUTH_NORMAL,
        };
        sql.set_raw_config_int(key, server_flags).await?;

        if let Some(provider) = self.provider {
            let key = &format!("{prefix}provider");
            sql.set_raw_config(key, Some(provider.id)).await?;
        }

        Ok(())
    }
}

impl fmt::Display for LoginParam {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let unset = "0";
        let pw = "***";

        write!(
            f,
            "{} imap:{}:{}:{}:{}:cert_{}:{} smtp:{}:{}:{}:{}:cert_{}:{}",
            unset_empty(&self.addr),
            unset_empty(&self.imap.user),
            if !self.imap.password.is_empty() {
                pw
            } else {
                unset
            },
            unset_empty(&self.imap.server),
            self.imap.port,
            self.imap.certificate_checks,
            if self.imap.oauth2 {
                "OAUTH2"
            } else {
                "AUTH_NORMAL"
            },
            unset_empty(&self.smtp.user),
            if !self.smtp.password.is_empty() {
                pw
            } else {
                unset
            },
            unset_empty(&self.smtp.server),
            self.smtp.port,
            self.smtp.certificate_checks,
            if self.smtp.oauth2 {
                "OAUTH2"
            } else {
                "AUTH_NORMAL"
            },
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

// this certificate is missing on older android devices (eg. lg with android6 from 2017)
// certificate downloaded from https://letsencrypt.org/certificates/
static LETSENCRYPT_ROOT: Lazy<Certificate> = Lazy::new(|| {
    Certificate::from_der(include_bytes!(
        "../assets/root-certificates/letsencrypt/isrgrootx1.der"
    ))
    .unwrap()
});

pub fn build_tls(strict_tls: bool) -> async_native_tls::TlsConnector {
    let tls_builder =
        async_native_tls::TlsConnector::new().add_root_certificate(LETSENCRYPT_ROOT.clone());

    if strict_tls {
        tls_builder
    } else {
        tls_builder
            .danger_accept_invalid_hostnames(true)
            .danger_accept_invalid_certs(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::TestContext;

    #[test]
    fn test_certificate_checks_display() {
        use std::string::ToString;

        assert_eq!(
            "accept_invalid_certificates".to_string(),
            CertificateChecks::AcceptInvalidCertificates.to_string()
        );
    }

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
                certificate_checks: CertificateChecks::Strict,
            },
            smtp: ServerLoginParam {
                server: "smtp.example.com".to_string(),
                user: "alice@example.org".to_string(),
                password: "bar".to_string(),
                port: 456,
                security: Socket::Ssl,
                oauth2: false,
                certificate_checks: CertificateChecks::AcceptInvalidCertificates,
            },
            provider: get_provider_by_id("example.com"),
            // socks5_config is not saved by `save_to_database`, using default value
            socks5_config: None,
        };

        param.save_as_configured_params(&t).await?;
        let loaded = LoginParam::load_configured_params(&t).await?;

        assert_eq!(param, loaded);
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_build_tls() -> Result<()> {
        // we are using some additional root certificates.
        // make sure, they do not break construction of TlsConnector
        let _ = build_tls(true);
        let _ = build_tls(false);
        Ok(())
    }
}
