//! # SMTP transport module.

pub mod send;

use std::time::{Duration, SystemTime};

use async_smtp::smtp::client::net::ClientTlsParameters;
use async_smtp::{error, smtp, EmailAddress, ServerAddress};

use crate::constants::DC_LP_AUTH_OAUTH2;
use crate::events::EventType;
use crate::login_param::{
    dc_build_tls, CertificateChecks, LoginParam, ServerLoginParam, Socks5Config,
};
use crate::oauth2::dc_get_oauth2_access_token;
use crate::provider::Socket;
use crate::{context::Context, scheduler::connectivity::ConnectivityStore};

/// SMTP write and read timeout in seconds.
const SMTP_TIMEOUT: u64 = 30;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Bad parameters")]
    BadParameters,
    #[error("Invalid login address {address}: {error}")]
    InvalidLoginAddress {
        address: String,
        #[source]
        error: error::Error,
    },
    #[error("SMTP failed to connect: {0}")]
    ConnectionFailure(#[source] smtp::error::Error),
    #[error("SMTP oauth2 error {address}")]
    Oauth2 { address: String },
    #[error("TLS error {0}")]
    Tls(#[from] async_native_tls::Error),
    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Default)]
pub(crate) struct Smtp {
    transport: Option<smtp::SmtpTransport>,

    /// Email address we are sending from.
    from: Option<EmailAddress>,

    /// Timestamp of last successful send/receive network interaction
    /// (eg connect or send succeeded). On initialization and disconnect
    /// it is set to None.
    last_success: Option<SystemTime>,

    pub(crate) connectivity: ConnectivityStore,

    /// If sending the last message failed, contains the error message.
    pub(crate) last_send_error: Option<String>,
}

impl Smtp {
    /// Create a new Smtp instances.
    pub fn new() -> Self {
        Default::default()
    }

    /// Disconnect the SMTP transport and drop it entirely.
    pub async fn disconnect(&mut self) {
        if let Some(mut transport) = self.transport.take() {
            transport.close().await.ok();
        }
        self.last_success = None;
    }

    /// Return true if smtp was connected but is not known to
    /// have been successfully used the last 60 seconds
    pub async fn has_maybe_stale_connection(&self) -> bool {
        if let Some(last_success) = self.last_success {
            SystemTime::now()
                .duration_since(last_success)
                .unwrap_or_default()
                .as_secs()
                > 60
        } else {
            false
        }
    }

    /// Check whether we are connected.
    pub async fn is_connected(&self) -> bool {
        self.transport
            .as_ref()
            .map(|t| t.is_connected())
            .unwrap_or_default()
    }

    /// Connect using configured parameters.
    pub async fn connect_configured(&mut self, context: &Context) -> Result<()> {
        if self.is_connected().await {
            return Ok(());
        }

        self.connectivity.set_connecting(context).await;
        let lp = LoginParam::from_database(context, "configured_").await?;
        self.connect(
            context,
            &lp.smtp,
            &lp.socks5_config,
            &lp.addr,
            lp.server_flags & DC_LP_AUTH_OAUTH2 != 0,
            lp.provider
                .map_or(lp.socks5_config.is_some(), |provider| provider.strict_tls),
        )
        .await
    }

    /// Connect using the provided login params.
    pub async fn connect(
        &mut self,
        context: &Context,
        lp: &ServerLoginParam,
        socks5_config: &Option<Socks5Config>,
        addr: &str,
        oauth2: bool,
        provider_strict_tls: bool,
    ) -> Result<()> {
        if self.is_connected().await {
            warn!(context, "SMTP already connected.");
            return Ok(());
        }

        if lp.server.is_empty() || lp.port == 0 {
            return Err(Error::BadParameters);
        }

        let from =
            EmailAddress::new(addr.to_string()).map_err(|err| Error::InvalidLoginAddress {
                address: addr.to_string(),
                error: err,
            })?;

        self.from = Some(from);

        let domain = &lp.server;
        let port = lp.port;

        let strict_tls = match lp.certificate_checks {
            CertificateChecks::Automatic => provider_strict_tls,
            CertificateChecks::Strict => true,
            CertificateChecks::AcceptInvalidCertificates
            | CertificateChecks::AcceptInvalidCertificates2 => false,
        };
        let tls_config = dc_build_tls(strict_tls);
        let tls_parameters = ClientTlsParameters::new(domain.to_string(), tls_config);

        let (creds, mechanism) = if oauth2 {
            // oauth2
            let send_pw = &lp.password;
            let access_token = dc_get_oauth2_access_token(context, addr, send_pw, false).await?;
            if access_token.is_none() {
                return Err(Error::Oauth2 {
                    address: addr.to_string(),
                });
            }
            let user = &lp.user;
            (
                smtp::authentication::Credentials::new(
                    user.to_string(),
                    access_token.unwrap_or_default(),
                ),
                vec![smtp::authentication::Mechanism::Xoauth2],
            )
        } else {
            // plain
            let user = lp.user.clone();
            let pw = lp.password.clone();
            (
                smtp::authentication::Credentials::new(user, pw),
                vec![
                    smtp::authentication::Mechanism::Plain,
                    smtp::authentication::Mechanism::Login,
                ],
            )
        };

        let security = match lp.security {
            Socket::Plain => smtp::ClientSecurity::None,
            Socket::Starttls => smtp::ClientSecurity::Required(tls_parameters),
            _ => smtp::ClientSecurity::Wrapper(tls_parameters),
        };

        let client =
            smtp::SmtpClient::with_security(ServerAddress::new(domain.to_string(), port), security);

        let mut client = client
            .smtp_utf8(true)
            .credentials(creds)
            .authentication_mechanism(mechanism)
            .connection_reuse(smtp::ConnectionReuseParameters::ReuseUnlimited)
            .timeout(Some(Duration::from_secs(SMTP_TIMEOUT)));

        if let Some(socks5_config) = socks5_config {
            client = client.use_socks5(socks5_config.to_async_smtp_socks5_config());
        }

        let mut trans = client.into_transport();
        if let Err(err) = trans.connect().await {
            return Err(Error::ConnectionFailure(err));
        }

        self.transport = Some(trans);
        self.last_success = Some(SystemTime::now());

        context.emit_event(EventType::SmtpConnected(format!(
            "SMTP-LOGIN as {} ok",
            lp.user,
        )));

        Ok(())
    }
}
