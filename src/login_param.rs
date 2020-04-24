//! # Login parameters

use std::borrow::Cow;
use std::fmt;
use std::str::FromStr;
use std::string::ToString;
use strum_macros::{AsRefStr, Display, EnumString};

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

impl LoginParam {
    /// Create a new `LoginParam` with default values.
    pub fn new() -> Self {
        Default::default()
    }

    /// Read the login parameters from the database.
    pub fn from_database(context: &Context, prefix: impl AsRef<str>) -> Self {
        let prefix = prefix.as_ref();
        let sql = &context.sql;

        let key = format!("{}addr", prefix);
        let addr = sql
            .get_raw_config(context, key)
            .unwrap_or_default()
            .trim()
            .to_string();

        let key = format!("{}mail_server", prefix);
        let mail_server = sql.get_raw_config(context, key).unwrap_or_default();

        let key = format!("{}mail_port", prefix);
        let mail_port = sql.get_raw_config_int(context, key).unwrap_or_default();

        let key = format!("{}mail_user", prefix);
        let mail_user = sql.get_raw_config(context, key).unwrap_or_default();

        let key = format!("{}mail_pw", prefix);
        let mail_pw = sql.get_raw_config(context, key).unwrap_or_default();

        let key = format!("{}imap_certificate_checks", prefix);
        let imap_certificate_checks =
            if let Some(certificate_checks) = sql.get_raw_config_int(context, key) {
                num_traits::FromPrimitive::from_i32(certificate_checks).unwrap()
            } else {
                Default::default()
            };

        let key = format!("{}send_server", prefix);
        let send_server = sql.get_raw_config(context, key).unwrap_or_default();

        let key = format!("{}send_port", prefix);
        let send_port = sql.get_raw_config_int(context, key).unwrap_or_default();

        let key = format!("{}send_user", prefix);
        let send_user = sql.get_raw_config(context, key).unwrap_or_default();

        let key = format!("{}send_pw", prefix);
        let send_pw = sql.get_raw_config(context, key).unwrap_or_default();

        let key = format!("{}smtp_certificate_checks", prefix);
        let smtp_certificate_checks =
            if let Some(certificate_checks) = sql.get_raw_config_int(context, key) {
                num_traits::FromPrimitive::from_i32(certificate_checks).unwrap()
            } else {
                Default::default()
            };

        let key = format!("{}auth_scheme", prefix);
        let auth_scheme = AuthScheme::from_str(
            sql.get_raw_config(context, key)
                .unwrap_or_default()
                .as_str(),
        )
        .unwrap_or_default();
        let key = format!("{}imap_security", prefix);
        let imap_security = ServerSecurity::from_str_opt(
            sql.get_raw_config(context, key)
                .unwrap_or_default()
                .as_str(),
        );
        let key = format!("{}smtp_security", prefix);
        let smtp_security = ServerSecurity::from_str_opt(
            sql.get_raw_config(context, key)
                .unwrap_or_default()
                .as_str(),
        );

        LoginParam {
            addr,
            auth_scheme,
            srv_params: [
                ServerParam {
                    hostname: mail_server,
                    user: mail_user,
                    pw: mail_pw,
                    port: mail_port,
                    certificate_checks: imap_certificate_checks,
                    security: imap_security,
                },
                ServerParam {
                    hostname: send_server,
                    user: send_user,
                    pw: send_pw,
                    port: send_port,
                    certificate_checks: smtp_certificate_checks,
                    security: smtp_security,
                },
            ],
        }
    }

    pub fn addr_str(&self) -> &str {
        self.addr.as_str()
    }

    /// Save this loginparam to the database.
    pub fn save_to_database(
        &self,
        context: &Context,
        prefix: impl AsRef<str>,
    ) -> crate::sql::Result<()> {
        let prefix = prefix.as_ref();
        let sql = &context.sql;

        let key = format!("{}addr", prefix);
        sql.set_raw_config(context, key, Some(&self.addr))?;

        let key = format!("{}mail_server", prefix);
        sql.set_raw_config(
            context,
            key,
            Some(&self.srv_params[Service::Imap as usize].hostname),
        )?;

        let key = format!("{}mail_port", prefix);
        sql.set_raw_config_int(context, key, self.srv_params[Service::Imap as usize].port)?;

        let key = format!("{}mail_user", prefix);
        sql.set_raw_config(
            context,
            key,
            Some(&self.srv_params[Service::Imap as usize].user),
        )?;

        let key = format!("{}mail_pw", prefix);
        sql.set_raw_config(
            context,
            key,
            Some(&self.srv_params[Service::Imap as usize].pw),
        )?;

        let key = format!("{}imap_certificate_checks", prefix);
        sql.set_raw_config_int(
            context,
            key,
            self.srv_params[Service::Imap as usize].certificate_checks as i32,
        )?;

        let key = format!("{}send_server", prefix);
        sql.set_raw_config(
            context,
            key,
            Some(&self.srv_params[Service::Smtp as usize].hostname),
        )?;

        let key = format!("{}send_port", prefix);
        sql.set_raw_config_int(context, key, self.srv_params[Service::Smtp as usize].port)?;

        let key = format!("{}send_user", prefix);
        sql.set_raw_config(
            context,
            key,
            Some(&self.srv_params[Service::Smtp as usize].user),
        )?;

        let key = format!("{}send_pw", prefix);
        sql.set_raw_config(
            context,
            key,
            Some(&self.srv_params[Service::Smtp as usize].pw),
        )?;

        let key = format!("{}smtp_certificate_checks", prefix);
        sql.set_raw_config_int(
            context,
            key,
            self.srv_params[Service::Smtp as usize].certificate_checks as i32,
        )?;

        let key = format!("{}auth_scheme", prefix);
        sql.set_raw_config(context, key, Some(self.auth_scheme.as_ref()))?;

        if self.srv_params[Service::Imap as usize].security.is_some() {
            let key = format!("{}imap_security", prefix);
            let sec: ServerSecurity;
            let val = if self.srv_params[Service::Imap as usize].security.is_none() {
                None
            } else {
                sec = self.srv_params[Service::Imap as usize].security.unwrap();
                Some(sec.as_ref())
            };
            sql.set_raw_config(context, key, val)?;
        }

        if self.srv_params[Service::Smtp as usize].security.is_some() {
            let key = format!("{}smtp_security", prefix);
            let sec: ServerSecurity;
            let val = if self.srv_params[Service::Smtp as usize].security.is_none() {
                None
            } else {
                sec = self.srv_params[Service::Smtp as usize].security.unwrap();
                Some(sec.as_ref())
            };
            sql.set_raw_config(context, key, val)?;
        }

        Ok(())
    }
}

impl fmt::Display for ServerParam {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let unset = "0";
        let pw = "***";

        write!(
            f,
            "{}:{}:{}:{}:cert_{}:security_{:?}",
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
            "{} imap:{} smtp:{} auth_{}",
            unset_empty(&self.addr),
            self.srv_params[Service::Imap as usize],
            self.srv_params[Service::Smtp as usize],
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
