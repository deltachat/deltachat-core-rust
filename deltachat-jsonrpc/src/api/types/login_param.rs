use anyhow::Result;
use deltachat::login_param as dc;
use serde::Deserialize;
use serde::Serialize;
use yerpc::TypeDef;

#[derive(Serialize, Deserialize, TypeDef, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
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

impl From<dc::EnteredServerLoginParam> for EnteredServerLoginParam {
    fn from(param: dc::EnteredServerLoginParam) -> Self {
        Self {
            server: param.server,
            port: param.port,
            security: param.security.into(),
            user: param.user,
            password: param.password,
        }
    }
}

impl From<EnteredServerLoginParam> for dc::EnteredServerLoginParam {
    fn from(param: EnteredServerLoginParam) -> Self {
        Self {
            server: param.server,
            port: param.port,
            security: param.security.into(),
            user: param.user,
            password: param.password,
        }
    }
}

/// Login parameters entered by the user.

#[derive(Serialize, Deserialize, TypeDef, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EnteredLoginParam {
    /// Email address.
    pub addr: String,

    /// IMAP settings.
    pub imap: EnteredServerLoginParam,

    /// SMTP settings.
    pub smtp: EnteredServerLoginParam,

    /// TLS options: whether to allow invalid certificates and/or
    /// invalid hostnames
    pub certificate_checks: EnteredCertificateChecks,

    /// If true, login via OAUTH2 (not recommended anymore)
    pub oauth2: bool,
}

impl From<dc::EnteredLoginParam> for EnteredLoginParam {
    fn from(param: dc::EnteredLoginParam) -> Self {
        Self {
            addr: param.addr,
            imap: param.imap.into(),
            smtp: param.smtp.into(),
            certificate_checks: param.certificate_checks.into(),
            oauth2: param.oauth2,
        }
    }
}

impl TryFrom<EnteredLoginParam> for dc::EnteredLoginParam {
    type Error = anyhow::Error;

    fn try_from(param: EnteredLoginParam) -> Result<Self> {
        Ok(Self {
            addr: param.addr,
            imap: param.imap.into(),
            smtp: param.smtp.into(),
            certificate_checks: param.certificate_checks.into(),
            oauth2: param.oauth2,
        })
    }
}

#[derive(Serialize, Deserialize, TypeDef, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Socket {
    /// Unspecified socket security, select automatically.
    Automatic,

    /// TLS connection.
    Ssl,

    /// STARTTLS connection.
    Starttls,

    /// No TLS, plaintext connection.
    Plain,
}

impl From<dc::Socket> for Socket {
    fn from(value: dc::Socket) -> Self {
        match value {
            dc::Socket::Automatic => Self::Automatic,
            dc::Socket::Ssl => Self::Ssl,
            dc::Socket::Starttls => Self::Starttls,
            dc::Socket::Plain => Self::Plain,
        }
    }
}

impl From<Socket> for dc::Socket {
    fn from(value: Socket) -> Self {
        match value {
            Socket::Automatic => Self::Automatic,
            Socket::Ssl => Self::Ssl,
            Socket::Starttls => Self::Starttls,
            Socket::Plain => Self::Plain,
        }
    }
}

#[derive(Serialize, Deserialize, TypeDef, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum EnteredCertificateChecks {
    /// `Automatic` means that provider database setting should be taken.
    /// If there is no provider database setting for certificate checks,
    /// check certificates strictly.
    Automatic,

    /// Ensure that TLS certificate is valid for the server hostname.
    Strict,

    /// Accept certificates that are expired, self-signed
    /// or otherwise not valid for the server hostname.
    AcceptInvalidCertificates,
}

impl From<dc::EnteredCertificateChecks> for EnteredCertificateChecks {
    fn from(value: dc::EnteredCertificateChecks) -> Self {
        match value {
            dc::EnteredCertificateChecks::Automatic => Self::Automatic,
            dc::EnteredCertificateChecks::Strict => Self::Strict,
            dc::EnteredCertificateChecks::AcceptInvalidCertificates => {
                Self::AcceptInvalidCertificates
            }
            dc::EnteredCertificateChecks::AcceptInvalidCertificates2 => {
                Self::AcceptInvalidCertificates
            }
        }
    }
}

impl From<EnteredCertificateChecks> for dc::EnteredCertificateChecks {
    fn from(value: EnteredCertificateChecks) -> Self {
        match value {
            EnteredCertificateChecks::Automatic => Self::Automatic,
            EnteredCertificateChecks::Strict => Self::Strict,
            EnteredCertificateChecks::AcceptInvalidCertificates => Self::AcceptInvalidCertificates,
        }
    }
}
