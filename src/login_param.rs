//! # Login parameters

use std::borrow::Cow;
use std::fmt;

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

#[derive(Default, Debug, Clone)]
pub struct LoginParam {
    pub addr: String,
    pub mail_server: String,
    pub mail_user: String,
    pub mail_pw: String,
    pub mail_port: i32,
    /// IMAP TLS options: whether to allow invalid certificates and/or invalid hostnames
    pub imap_certificate_checks: CertificateChecks,
    pub send_server: String,
    pub send_user: String,
    pub send_pw: String,
    pub send_port: i32,
    /// SMTP TLS options: whether to allow invalid certificates and/or invalid hostnames
    pub smtp_certificate_checks: CertificateChecks,
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
            mail_server,
            mail_user,
            mail_pw,
            mail_port,
            imap_certificate_checks,
            send_server,
            send_user,
            send_pw,
            send_port,
            smtp_certificate_checks,
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
        sql.set_raw_config(context, key, Some(&self.mail_server))
            .await?;

        let key = format!("{}mail_port", prefix);
        sql.set_raw_config_int(context, key, self.mail_port).await?;

        let key = format!("{}mail_user", prefix);
        sql.set_raw_config(context, key, Some(&self.mail_user))
            .await?;

        let key = format!("{}mail_pw", prefix);
        sql.set_raw_config(context, key, Some(&self.mail_pw))
            .await?;

        let key = format!("{}imap_certificate_checks", prefix);
        sql.set_raw_config_int(context, key, self.imap_certificate_checks as i32)
            .await?;

        let key = format!("{}send_server", prefix);
        sql.set_raw_config(context, key, Some(&self.send_server))
            .await?;

        let key = format!("{}send_port", prefix);
        sql.set_raw_config_int(context, key, self.send_port).await?;

        let key = format!("{}send_user", prefix);
        sql.set_raw_config(context, key, Some(&self.send_user))
            .await?;

        let key = format!("{}send_pw", prefix);
        sql.set_raw_config(context, key, Some(&self.send_pw))
            .await?;

        let key = format!("{}smtp_certificate_checks", prefix);
        sql.set_raw_config_int(context, key, self.smtp_certificate_checks as i32)
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
            unset_empty(&self.mail_user),
            if !self.mail_pw.is_empty() { pw } else { unset },
            unset_empty(&self.mail_server),
            self.mail_port,
            self.imap_certificate_checks,
            unset_empty(&self.send_user),
            if !self.send_pw.is_empty() { pw } else { unset },
            unset_empty(&self.send_server),
            self.send_port,
            self.smtp_certificate_checks,
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
            if 1 << bit == 0x100 {
                res += "IMAP_STARTTLS ";
                flag_added = true;
            }
            if 1 << bit == 0x200 {
                res += "IMAP_SSL ";
                flag_added = true;
            }
            if 1 << bit == 0x400 {
                res += "IMAP_PLAIN ";
                flag_added = true;
            }
            if 1 << bit == 0x10000 {
                res += "SMTP_STARTTLS ";
                flag_added = true;
            }
            if 1 << bit == 0x20000 {
                res += "SMTP_SSL ";
                flag_added = true;
            }
            if 1 << bit == 0x40000 {
                res += "SMTP_PLAIN ";
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
