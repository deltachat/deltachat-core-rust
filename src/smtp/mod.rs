//! # SMTP transport module

pub mod send;

use async_std::sync::{channel, Receiver, RwLock, Sender};
use std::time::{Duration, Instant};

use async_smtp::smtp::client::net::*;
use async_smtp::*;

use crate::constants::*;
use crate::context::{Context, PerformJobsNeeded};
use crate::events::Event;
use crate::login_param::{dc_build_tls, LoginParam};
use crate::oauth2::*;

/// SMTP write and read timeout in seconds.
const SMTP_TIMEOUT: u64 = 30;

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "Bad parameters")]
    BadParameters,

    #[fail(display = "Invalid login address {}: {}", address, error)]
    InvalidLoginAddress {
        address: String,
        #[cause]
        error: error::Error,
    },

    #[fail(display = "SMTP failed to connect: {:?}", _0)]
    ConnectionFailure(#[cause] smtp::error::Error),

    #[fail(display = "SMTP: failed to setup connection {:?}", _0)]
    ConnectionSetupFailure(#[cause] smtp::error::Error),

    #[fail(display = "SMTP: oauth2 error {:?}", _0)]
    Oauth2Error { address: String },

    #[fail(display = "TLS error")]
    Tls(#[cause] async_native_tls::Error),
}

impl From<async_native_tls::Error> for Error {
    fn from(err: async_native_tls::Error) -> Error {
        Error::Tls(err)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct Smtp {
    inner: RwLock<SmtpInner>,
    pub(crate) state: RwLock<State>,
    pub(crate) notify_sender: Sender<()>,
    pub(crate) notify_receiver: Receiver<()>,
}

impl Default for Smtp {
    fn default() -> Self {
        let (notify_sender, notify_receiver) = channel(1);
        Smtp {
            inner: Default::default(),
            state: Default::default(),
            notify_sender,
            notify_receiver,
        }
    }
}

#[derive(Default, Debug)]
pub struct State {
    pub(crate) suspended: bool,
    pub(crate) doing_jobs: bool,
    pub(crate) perform_jobs_needed: PerformJobsNeeded,
    pub(crate) probe_network: bool,
}

#[derive(Default, DebugStub)]
struct SmtpInner {
    #[debug_stub(some = "SmtpTransport")]
    transport: Option<smtp::SmtpTransport>,

    /// Email address we are sending from.
    from: Option<EmailAddress>,

    /// Timestamp of last successful send/receive network interaction
    /// (eg connect or send succeeded). On initialization and disconnect
    /// it is set to None.
    last_success: Option<Instant>,
}

impl Smtp {
    /// Create a new Smtp instances.
    pub fn new() -> Self {
        Default::default()
    }

    /// Disconnect the SMTP transport and drop it entirely.
    pub async fn disconnect(&self) {
        let inner = &mut *self.inner.write().await;
        if let Some(mut transport) = inner.transport.take() {
            transport.close().await.ok();
        }
        inner.last_success = None;
    }

    /// Return true if smtp was connected but is not known to
    /// have been successfully used the last 60 seconds
    pub async fn has_maybe_stale_connection(&self) -> bool {
        if let Some(last_success) = self.inner.read().await.last_success {
            Instant::now().duration_since(last_success).as_secs() > 60
        } else {
            false
        }
    }

    /// Check whether we are connected.
    pub async fn is_connected(&self) -> bool {
        self.inner
            .read()
            .await
            .transport
            .as_ref()
            .map(|t| t.is_connected())
            .unwrap_or_default()
    }

    /// Connect using the provided login params.
    pub async fn connect(&self, context: &Context, lp: &LoginParam) -> Result<()> {
        if self.is_connected().await {
            warn!(context, "SMTP already connected.");
            return Ok(());
        }

        if lp.send_server.is_empty() || lp.send_port == 0 {
            context.call_cb(Event::ErrorNetwork("SMTP bad parameters.".into()));
            return Err(Error::BadParameters);
        }

        let from =
            EmailAddress::new(lp.addr.clone()).map_err(|err| Error::InvalidLoginAddress {
                address: lp.addr.clone(),
                error: err,
            })?;

        let inner = &mut *self.inner.write().await;
        inner.from = Some(from);

        let domain = &lp.send_server;
        let port = lp.send_port as u16;

        let tls_config = dc_build_tls(lp.smtp_certificate_checks);
        let tls_parameters = ClientTlsParameters::new(domain.to_string(), tls_config);

        let (creds, mechanism) = if 0 != lp.server_flags & (DC_LP_AUTH_OAUTH2 as i32) {
            // oauth2
            let addr = &lp.addr;
            let send_pw = &lp.send_pw;
            let access_token = dc_get_oauth2_access_token(context, addr, send_pw, false).await;
            if access_token.is_none() {
                return Err(Error::Oauth2Error {
                    address: addr.to_string(),
                });
            }
            let user = &lp.send_user;
            (
                smtp::authentication::Credentials::new(
                    user.to_string(),
                    access_token.unwrap_or_default(),
                ),
                vec![smtp::authentication::Mechanism::Xoauth2],
            )
        } else {
            // plain
            let user = lp.send_user.clone();
            let pw = lp.send_pw.clone();
            (
                smtp::authentication::Credentials::new(user, pw),
                vec![
                    smtp::authentication::Mechanism::Plain,
                    smtp::authentication::Mechanism::Login,
                ],
            )
        };

        let security = if 0
            != lp.server_flags & (DC_LP_SMTP_SOCKET_STARTTLS | DC_LP_SMTP_SOCKET_PLAIN) as i32
        {
            smtp::ClientSecurity::Opportunistic(tls_parameters)
        } else {
            smtp::ClientSecurity::Wrapper(tls_parameters)
        };

        let client = smtp::SmtpClient::with_security((domain.as_str(), port), security)
            .await
            .map_err(Error::ConnectionSetupFailure)?;

        let client = client
            .smtp_utf8(true)
            .credentials(creds)
            .authentication_mechanism(mechanism)
            .connection_reuse(smtp::ConnectionReuseParameters::ReuseUnlimited)
            .timeout(Some(Duration::from_secs(SMTP_TIMEOUT)));

        let mut trans = client.into_transport();
        trans.connect().await.map_err(Error::ConnectionFailure)?;

        inner.transport = Some(trans);
        inner.last_success = Some(Instant::now());

        context.call_cb(Event::SmtpConnected(format!(
            "SMTP-LOGIN as {} ok",
            lp.send_user,
        )));

        Ok(())
    }
}
