//! # Login parameters

use std::borrow::Cow;
use std::fmt;

use crate::{
    context::Context,
    provider::{Protocol, Socket, UsernamePattern},
};

#[derive(Copy, Clone, Debug, Display, FromPrimitive, PartialEq, Eq)]
#[repr(i32)]
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

#[derive(Debug)]
pub struct ServerParams {
    pub protocol: Protocol,
    pub socket: Socket,
    pub hostname: String,
    pub port: u16,
    pub username_pattern: UsernamePattern,
}

pub type ImapServers = Vec<ServerParams>;
pub type SmtpServers = Vec<ServerParams>;

#[derive(Debug)]
pub struct LoginParamNew {
    pub addr: String,
    pub imap: ImapServers,
    pub smtp: SmtpServers,
}

impl ServerParams {
    pub fn apply_username_pattern(&self, addr: String) -> String {
        match self.username_pattern {
            UsernamePattern::EMAIL => addr,
            UsernamePattern::EMAILLOCALPART => {
                if let Some(at) = addr.find('@') {
                    return addr.split_at(at).0.to_string();
                }
                addr
            }
        }
    }
}

/// Login parameters for a single server, either IMAP or SMTP
#[derive(Default, Debug, Clone)]
pub struct ServerLoginParam {
    pub server: String,
    pub user: String,
    pub password: String,
    pub port: u16,
    pub security: Socket,

    /// TLS options: whether to allow invalid certificates and/or
    /// invalid hostnames
    pub certificate_checks: CertificateChecks,
}

#[derive(Default, Debug, Clone)]
pub struct LoginParam {
    pub addr: String,
    pub imap: ServerLoginParam,
    pub smtp: ServerLoginParam,
    pub server_flags: i32,
}

impl LoginParam {
    /// Create a new `LoginParam` with default values.
    pub fn new() -> Self {
        Default::default()
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

        let key = format!("{}mail_server", prefix);
        let mail_server = sql.get_raw_config(context, key).await.unwrap_or_default();

        let key = format!("{}mail_port", prefix);
        let mail_port = sql
            .get_raw_config_int(context, key)
            .await
            .unwrap_or_default();

        let key = format!("{}mail_user", prefix);
        let mail_user = sql.get_raw_config(context, key).await.unwrap_or_default();

        let key = format!("{}mail_pw", prefix);
        let mail_pw = sql.get_raw_config(context, key).await.unwrap_or_default();

        let key = format!("{}mail_security", prefix);
        let mail_security = sql
            .get_raw_config_int(context, key)
            .await
            .and_then(num_traits::FromPrimitive::from_i32)
            .unwrap_or_default();

        let key = format!("{}imap_certificate_checks", prefix);
        let imap_certificate_checks =
            if let Some(certificate_checks) = sql.get_raw_config_int(context, key).await {
                num_traits::FromPrimitive::from_i32(certificate_checks).unwrap()
            } else {
                Default::default()
            };

        let key = format!("{}send_server", prefix);
        let send_server = sql.get_raw_config(context, key).await.unwrap_or_default();

        let key = format!("{}send_port", prefix);
        let send_port = sql
            .get_raw_config_int(context, key)
            .await
            .unwrap_or_default();

        let key = format!("{}send_user", prefix);
        let send_user = sql.get_raw_config(context, key).await.unwrap_or_default();

        let key = format!("{}send_pw", prefix);
        let send_pw = sql.get_raw_config(context, key).await.unwrap_or_default();

        let key = format!("{}send_security", prefix);
        let send_security = sql
            .get_raw_config_int(context, key)
            .await
            .and_then(num_traits::FromPrimitive::from_i32)
            .unwrap_or_default();

        let key = format!("{}smtp_certificate_checks", prefix);
        let smtp_certificate_checks =
            if let Some(certificate_checks) = sql.get_raw_config_int(context, key).await {
                num_traits::FromPrimitive::from_i32(certificate_checks).unwrap()
            } else {
                Default::default()
            };

        let key = format!("{}server_flags", prefix);
        let server_flags = sql
            .get_raw_config_int(context, key)
            .await
            .unwrap_or_default();

        LoginParam {
            addr,
            imap: ServerLoginParam {
                server: mail_server,
                user: mail_user,
                password: mail_pw,
                port: mail_port as u16,
                security: mail_security,
                certificate_checks: imap_certificate_checks,
            },
            smtp: ServerLoginParam {
                server: send_server,
                user: send_user,
                password: send_pw,
                port: send_port as u16,
                security: send_security,
                certificate_checks: smtp_certificate_checks,
            },
            server_flags,
        }
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

        let key = format!("{}mail_server", prefix);
        sql.set_raw_config(context, key, Some(&self.imap.server))
            .await?;

        let key = format!("{}mail_port", prefix);
        sql.set_raw_config_int(context, key, self.imap.port as i32)
            .await?;

        let key = format!("{}mail_user", prefix);
        sql.set_raw_config(context, key, Some(&self.imap.user))
            .await?;

        let key = format!("{}mail_pw", prefix);
        sql.set_raw_config(context, key, Some(&self.imap.password))
            .await?;

        let key = format!("{}mail_security", prefix);
        sql.set_raw_config_int(context, key, self.imap.security as i32)
            .await?;

        let key = format!("{}imap_certificate_checks", prefix);
        sql.set_raw_config_int(context, key, self.imap.certificate_checks as i32)
            .await?;

        let key = format!("{}send_server", prefix);
        sql.set_raw_config(context, key, Some(&self.smtp.server))
            .await?;

        let key = format!("{}send_port", prefix);
        sql.set_raw_config_int(context, key, self.smtp.port as i32)
            .await?;

        let key = format!("{}send_user", prefix);
        sql.set_raw_config(context, key, Some(&self.smtp.user))
            .await?;

        let key = format!("{}send_pw", prefix);
        sql.set_raw_config(context, key, Some(&self.smtp.password))
            .await?;

        let key = format!("{}send_security", prefix);
        sql.set_raw_config_int(context, key, self.smtp.security as i32)
            .await?;

        let key = format!("{}smtp_certificate_checks", prefix);
        sql.set_raw_config_int(context, key, self.smtp.certificate_checks as i32)
            .await?;

        let key = format!("{}server_flags", prefix);
        sql.set_raw_config_int(context, key, self.server_flags)
            .await?;

        Ok(())
    }
}

impl fmt::Display for LoginParam {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let unset = "0";
        let pw = "***";

        let flags_readable = get_readable_flags(self.server_flags);

        write!(
            f,
            "{} imap:{}:{}:{}:{}:cert_{} smtp:{}:{}:{}:{}:cert_{} {}",
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
            unset_empty(&self.smtp.user),
            if !self.smtp.password.is_empty() {
                pw
            } else {
                unset
            },
            unset_empty(&self.smtp.server),
            self.smtp.port,
            self.smtp.certificate_checks,
            flags_readable,
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

#[allow(clippy::useless_let_if_seq)]
fn get_readable_flags(flags: i32) -> String {
    let mut res = String::new();
    for bit in 0..31 {
        if 0 != flags & 1 << bit {
            let mut flag_added = false;
            if 1 << bit == 0x2 {
                res += "OAUTH2 ";
                flag_added = true;
            }
            if 1 << bit == 0x4 {
                res += "AUTH_NORMAL ";
                flag_added = true;
            }
            if flag_added {
                res += &format!("{:#0x}", 1 << bit);
            }
        }
    }
    if res.is_empty() {
        res += "0";
    }

    res
}

pub fn dc_build_tls(strict_tls: bool) -> async_native_tls::TlsConnector {
    let tls_builder = async_native_tls::TlsConnector::new();

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

    #[test]
    fn test_certificate_checks_display() {
        use std::string::ToString;

        assert_eq!(
            "accept_invalid_certificates".to_string(),
            CertificateChecks::AcceptInvalidCertificates.to_string()
        );
    }
}
