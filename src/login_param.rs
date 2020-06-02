//! # Login parameters

use std::borrow::Cow;
use std::fmt;
use std::str::FromStr;
use std::string::ToString;
use strum_macros::{AsRefStr, Display, EnumString};

use crate::constants::{
    DC_LP_AUTH_FLAGS, DC_LP_AUTH_NORMAL, DC_LP_AUTH_OAUTH2, DC_LP_IMAP_SOCKET_FLAGS,
    DC_LP_IMAP_SOCKET_PLAIN, DC_LP_IMAP_SOCKET_SSL, DC_LP_IMAP_SOCKET_STARTTLS,
    DC_LP_SMTP_SOCKET_FLAGS, DC_LP_SMTP_SOCKET_PLAIN, DC_LP_SMTP_SOCKET_SSL,
    DC_LP_SMTP_SOCKET_STARTTLS,
};
use crate::context::Context;

#[derive(Copy, Clone, Debug, Display, FromPrimitive)]
#[repr(i32)]
#[strum(serialize_all = "snake_case")]
pub enum CertificateChecks {
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

#[derive(
    Debug, Display, Clone, Copy, PartialEq, Eq, EnumString, AsRefStr, EnumIter, EnumProperty,
)]
#[strum(serialize_all = "snake_case")]
pub enum AuthScheme {
    Plain,
    Oauth2,
}

impl AuthScheme {
    pub fn is_oauth2(&self) -> bool {
        match self {
            Self::Plain => false,
            Self::Oauth2 => true,
        }
    }
}

impl Default for AuthScheme {
    fn default() -> Self {
        Self::Plain
    }
}

#[derive(
    Debug, Display, Clone, Copy, PartialEq, Eq, EnumString, AsRefStr, EnumIter, EnumProperty,
)]
#[strum(serialize_all = "snake_case")]
pub enum ServerSecurity {
    PlainSocket,
    Ssl,
    Starttls,
}

impl ServerSecurity {
    /// Create as Option from string
    pub fn from_str_opt(s: &str) -> Option<ServerSecurity> {
        match ServerSecurity::from_str(s) {
            Ok(sec) => Some(sec),
            _ => None,
        }
    }
}

#[derive(
    Debug, Display, Clone, Copy, PartialEq, Eq, EnumString, AsRefStr, EnumIter, EnumProperty,
)]
#[strum(serialize_all = "snake_case")]
pub enum Service {
    Imap,
    Smtp,
}

pub static IDX_IMAP: usize = Service::Imap as usize;
pub static IDX_SMTP: usize = Service::Smtp as usize;

impl Service {
    fn prefixes(&self) -> (&'static str, &'static str) {
        match self {
            Self::Imap => ("mail_", "imap_"),
            Self::Smtp => ("send_", "smtp_"),
        }
    }
}

#[derive(Clone, Default, Debug)]
pub struct ServerParam {
    pub hostname: String,
    pub user: String,
    pub pw: String,
    pub port: i32,
    /// TLS options: whether to allow invalid certificates and/or invalid hostnames
    pub certificate_checks: CertificateChecks,
    /// security option Plain TCP, SSL or SARTTLS.
    pub security: Option<ServerSecurity>,
}
#[derive(Clone, Default, Debug)]
pub struct LoginParam {
    pub addr: String,
    /// Auth option OAUTH2 or plain password
    pub auth_scheme: AuthScheme,
    pub srv_params: [ServerParam; 2],
}

impl ServerParam {
    pub async fn from_database(
        context: &Context,
        prefix: impl AsRef<str>,
        service: Service,
    ) -> Self {
        let prefix = prefix.as_ref();
        let sql = &context.sql;
        let (prefix1, prefix2) = service.prefixes();

        let key = format!("{}{}server", prefix, prefix1);
        let hostname = sql.get_raw_config(context, key).await.unwrap_or_default();

        let key = format!("{}{}port", prefix, prefix1);
        let port = sql
            .get_raw_config_int(context, key)
            .await
            .unwrap_or_default();

        let key = format!("{}{}user", prefix, prefix1);
        let user = sql.get_raw_config(context, key).await.unwrap_or_default();

        let key = format!("{}{}pw", prefix, prefix1);
        let pw = sql.get_raw_config(context, key).await.unwrap_or_default();

        let key = format!("{}{}certificate_checks", prefix, prefix2);
        let certificate_checks =
            if let Some(certificate_checks) = sql.get_raw_config_int(context, key).await {
                num_traits::FromPrimitive::from_i32(certificate_checks).unwrap()
            } else {
                Default::default()
            };

        Self {
            hostname,
            user,
            pw,
            port,
            certificate_checks,
            security: None,
        }
    }

    pub async fn save_to_database(
        &self,
        context: &Context,
        prefix: impl AsRef<str>,
        service: Service,
    ) -> crate::sql::Result<()> {
        let prefix = prefix.as_ref();
        let sql = &context.sql;
        let (prefix1, prefix2) = service.prefixes();

        let key = format!("{}{}server", prefix, prefix1);
        sql.set_raw_config(context, key, Some(&self.hostname))
            .await?;

        let key = format!("{}{}port", prefix, prefix1);
        sql.set_raw_config_int(context, key, self.port).await?;

        let key = format!("{}{}user", prefix, prefix1);
        sql.set_raw_config(context, key, Some(&self.user)).await?;

        let key = format!("{}{}pw", prefix, prefix1);
        sql.set_raw_config(context, key, Some(&self.pw)).await?;

        let key = format!("{}{}certificate_checks", prefix, prefix2);
        sql.set_raw_config_int(context, key, self.certificate_checks as i32)
            .await?;
        Ok(())
    }
}

impl LoginParam {
    /// Create a new `LoginParam` with default values.
    pub fn new() -> Self {
        Default::default()
    }

    fn set_server_flags(&mut self, flags: i32) {
        match flags & DC_LP_AUTH_FLAGS {
            DC_LP_AUTH_OAUTH2 => {
                self.auth_scheme = AuthScheme::Oauth2;
            }
            _ => {
                self.auth_scheme = AuthScheme::Plain;
            }
        }
        match flags & DC_LP_IMAP_SOCKET_FLAGS {
            DC_LP_IMAP_SOCKET_SSL => {
                self.srv_params[IDX_IMAP].security = Some(ServerSecurity::Ssl);
            }
            DC_LP_IMAP_SOCKET_STARTTLS => {
                self.srv_params[IDX_IMAP].security = Some(ServerSecurity::Starttls);
            }
            DC_LP_IMAP_SOCKET_PLAIN => {
                self.srv_params[IDX_IMAP].security = Some(ServerSecurity::PlainSocket);
            }
            _ => {
                // completely unset or multiple flags.
                self.srv_params[IDX_IMAP].security = None;
            }
        }
        match flags as usize & DC_LP_SMTP_SOCKET_FLAGS {
            DC_LP_SMTP_SOCKET_SSL => {
                self.srv_params[IDX_SMTP].security = Some(ServerSecurity::Ssl);
            }
            DC_LP_SMTP_SOCKET_STARTTLS => {
                self.srv_params[IDX_SMTP].security = Some(ServerSecurity::Starttls);
            }
            DC_LP_SMTP_SOCKET_PLAIN => {
                self.srv_params[IDX_SMTP].security = Some(ServerSecurity::PlainSocket);
            }
            _ => {
                // completely unset or multiple flags.
                self.srv_params[IDX_SMTP].security = None;
            }
        }
    }

    fn get_server_flags(&self) -> i32 {
        let auth_flags = match self.auth_scheme {
            AuthScheme::Oauth2 => DC_LP_AUTH_OAUTH2 as i32,
            AuthScheme::Plain => DC_LP_AUTH_NORMAL as i32,
        };
        let imap_flags = match self.srv_params[IDX_IMAP].security {
            Some(ServerSecurity::PlainSocket) => DC_LP_IMAP_SOCKET_PLAIN as i32,
            Some(ServerSecurity::Ssl) => DC_LP_IMAP_SOCKET_SSL as i32,
            Some(ServerSecurity::Starttls) => DC_LP_IMAP_SOCKET_STARTTLS as i32,
            _ => 0 as i32,
        };
        let smtp_flags = match self.srv_params[IDX_SMTP].security {
            Some(ServerSecurity::PlainSocket) => DC_LP_SMTP_SOCKET_PLAIN as i32,
            Some(ServerSecurity::Ssl) => DC_LP_SMTP_SOCKET_SSL as i32,
            Some(ServerSecurity::Starttls) => DC_LP_SMTP_SOCKET_STARTTLS as i32,
            _ => 0 as i32,
        };
        auth_flags | imap_flags | smtp_flags
    }

    /// Read the login parameters from the database.
    pub async fn from_database(context: &Context, prefix: impl AsRef<str>) -> Self {
        let prefix = prefix.as_ref();
        let sql = &context.sql;

        let key = format!("{}addr", prefix);
        let addr = sql
            .get_raw_config(context, key)
            .await
            .unwrap_or_default()
            .trim()
            .to_string();

        let key = format!("{}server_flags", prefix);
        let server_flags = sql
            .get_raw_config_int(context, key)
            .await
            .unwrap_or_default();

        let mut lp = LoginParam {
            addr,
            auth_scheme: AuthScheme::Plain,
            srv_params: [
                ServerParam::from_database(context, prefix, Service::Imap).await,
                ServerParam::from_database(context, prefix, Service::Smtp).await,
            ],
        };
        lp.set_server_flags(server_flags);
        lp
    }

    pub fn addr_str(&self) -> &str {
        self.addr.as_str()
    }

    /// Save this loginparam to the database.
    pub async fn save_to_database(
        &self,
        context: &Context,
        prefix: impl AsRef<str>,
    ) -> crate::sql::Result<()> {
        let prefix = prefix.as_ref();
        let sql = &context.sql;

        let key = format!("{}addr", prefix);
        sql.set_raw_config(context, key, Some(&self.addr)).await?;

        for service in vec![Service::Imap, Service::Smtp] {
            self.srv_params[service as usize]
                .save_to_database(context, prefix, service)
                .await?;
        }

        let key = format!("{}server_flags", prefix);
        sql.set_raw_config_int(context, key, self.get_server_flags())
            .await?;

        Ok(())
    }
}

impl fmt::Display for ServerParam {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let unset = "0";
        let pw = "***";

        write!(
            f,
            "<usr:{},pw:{},host:{},port:{},cert:{},security:{:?}>",
            unset_empty(&self.user),
            if !self.pw.is_empty() { pw } else { unset },
            unset_empty(&self.hostname),
            self.port,
            self.certificate_checks,
            self.security,
        )
    }
}

impl fmt::Display for LoginParam {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} imap:{} smtp:{} auth:{}",
            unset_empty(&self.addr),
            self.srv_params[IDX_IMAP],
            self.srv_params[IDX_SMTP],
            self.auth_scheme,
        )
    }
}

#[allow(clippy::ptr_arg)]
fn unset_empty(s: &String) -> Cow<String> {
    if s.is_empty() {
        Cow::Owned("unset".to_string())
    } else {
        Cow::Borrowed(s)
    }
}

pub fn dc_build_tls(certificate_checks: CertificateChecks) -> async_native_tls::TlsConnector {
    let tls_builder = async_native_tls::TlsConnector::new();
    match certificate_checks {
        CertificateChecks::Automatic => {
            // Same as AcceptInvalidCertificates for now.
            // TODO: use provider database when it becomes available
            tls_builder
                .danger_accept_invalid_hostnames(true)
                .danger_accept_invalid_certs(true)
        }
        CertificateChecks::Strict => tls_builder,
        CertificateChecks::AcceptInvalidCertificates
        | CertificateChecks::AcceptInvalidCertificates2 => tls_builder
            .danger_accept_invalid_hostnames(true)
            .danger_accept_invalid_certs(true),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_certificate_checks_display() {
        use std::string::ToString;

        assert_eq!(
            "accept_invalid_certificates".to_string(),
            CertificateChecks::AcceptInvalidCertificates.to_string()
        );
    }
}
