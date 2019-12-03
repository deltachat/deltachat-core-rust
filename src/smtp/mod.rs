//! # SMTP transport module

pub mod send;

use lettre::smtp::client::net::*;
use lettre::*;

use failure::Fail;

use crate::constants::*;
use crate::context::Context;
use crate::events::Event;
use crate::login_param::{dc_build_tls_config, LoginParam};
use crate::oauth2::*;

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "Bad parameters")]
    BadParameters,
    #[fail(display = "Invalid login address {}: {}", address, error)]
    InvalidLoginAddress {
        address: String,
        #[cause]
        error: lettre::error::Error,
    },
    #[fail(display = "SMTP failed to connect: {:?}", _0)]
    ConnectionFailure(#[cause] lettre::smtp::error::Error),
    #[fail(display = "SMTP: failed to setup connection {:?}", _0)]
    ConnectionSetupFailure(#[cause] lettre::smtp::error::Error),
    #[fail(display = "SMTP: oauth2 error {:?}", _0)]
    Oauth2Error { address: String },
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Default, DebugStub)]
pub struct Smtp {
    #[debug_stub(some = "SmtpTransport")]
    transport: Option<lettre::smtp::SmtpTransport>,
    transport_connected: bool,
    /// Email address we are sending from.
    from: Option<EmailAddress>,
}

impl Smtp {
    /// Create a new Smtp instances.
    pub fn new() -> Self {
        Default::default()
    }

    /// Disconnect the SMTP transport and drop it entirely.
    pub fn disconnect(&mut self) {
        if self.transport.is_none() || !self.transport_connected {
            return;
        }

        let mut transport = self.transport.take().unwrap();
        transport.close();
        self.transport_connected = false;
    }

    /// Check if a connection already exists.
    pub fn is_connected(&self) -> bool {
        self.transport.is_some()
    }

    /// Connect using the provided login params
    pub fn connect(&mut self, context: &Context, lp: &LoginParam) -> Result<()> {
        if self.is_connected() {
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
        self.from = Some(from);

        let domain = &lp.send_server;
        let port = lp.send_port as u16;

        let tls_config = dc_build_tls_config(lp.smtp_certificate_checks);
        let tls_parameters = ClientTlsParameters::new(domain.to_string(), tls_config);

        let (creds, mechanism) = if 0 != lp.server_flags & (DC_LP_AUTH_OAUTH2 as i32) {
            // oauth2
            let addr = &lp.addr;
            let send_pw = &lp.send_pw;
            let access_token = dc_get_oauth2_access_token(context, addr, send_pw, false);
            if access_token.is_none() {
                return Err(Error::Oauth2Error {
                    address: addr.to_string(),
                });
            }
            let user = &lp.send_user;
            (
                lettre::smtp::authentication::Credentials::new(
                    user.to_string(),
                    access_token.unwrap_or_default(),
                ),
                vec![lettre::smtp::authentication::Mechanism::Xoauth2],
            )
        } else {
            // plain
            let user = lp.send_user.clone();
            let pw = lp.send_pw.clone();
            (
                lettre::smtp::authentication::Credentials::new(user, pw),
                vec![
                    lettre::smtp::authentication::Mechanism::Plain,
                    lettre::smtp::authentication::Mechanism::Login,
                ],
            )
        };

        let security = if 0
            != lp.server_flags & (DC_LP_SMTP_SOCKET_STARTTLS | DC_LP_SMTP_SOCKET_PLAIN) as i32
        {
            lettre::smtp::ClientSecurity::Opportunistic(tls_parameters)
        } else {
            lettre::smtp::ClientSecurity::Wrapper(tls_parameters)
        };

        let client = lettre::smtp::SmtpClient::new((domain.as_str(), port), security)
            .map_err(Error::ConnectionSetupFailure)?;

        let client = client
            .smtp_utf8(true)
            .credentials(creds)
            .authentication_mechanism(mechanism)
            .connection_reuse(lettre::smtp::ConnectionReuseParameters::ReuseUnlimited);
        let mut trans = client.transport();
        trans.connect().map_err(Error::ConnectionFailure)?;

        self.transport = Some(trans);
        self.transport_connected = true;
        context.call_cb(Event::SmtpConnected(format!(
            "SMTP-LOGIN as {} ok",
            lp.send_user,
        )));
        Ok(())
    }
}
