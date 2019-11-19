use lettre::smtp::client::net::*;
use lettre::*;

use crate::constants::*;
use crate::context::Context;
use crate::error::Error;
use crate::events::Event;
use crate::login_param::{dc_build_tls, LoginParam};
use crate::oauth2::*;

#[derive(DebugStub)]
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
        Smtp {
            transport: None,
            transport_connected: false,
            from: None,
        }
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
    pub fn connect(&mut self, context: &Context, lp: &LoginParam) -> Result<(), Error> {
        if self.is_connected() {
            warn!(context, "SMTP already connected.");
            return Ok(());
        }

        if lp.send_server.is_empty() || lp.send_port == 0 {
            context.call_cb(Event::ErrorNetwork("SMTP bad parameters.".into()));
            bail!("SMTP Bad parameters");
        }

        self.from = match EmailAddress::new(lp.addr.clone()) {
            Ok(addr) => Some(addr),
            Err(err) => {
                bail!("invalid login address {}: {}", lp.addr, err);
            }
        };

        let domain = &lp.send_server;
        let port = lp.send_port as u16;

        let tls = dc_build_tls(lp.smtp_certificate_checks).unwrap();
        let tls_parameters = ClientTlsParameters::new(domain.to_string(), tls);

        let (creds, mechanism) = if 0 != lp.server_flags & (DC_LP_AUTH_OAUTH2 as i32) {
            // oauth2
            let addr = &lp.addr;
            let send_pw = &lp.send_pw;
            let access_token = dc_get_oauth2_access_token(context, addr, send_pw, false);
            ensure!(
                access_token.is_some(),
                "could not get oaut2_access token addr={}",
                addr
            );
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

        match lettre::smtp::SmtpClient::new((domain.as_str(), port), security) {
            Ok(client) => {
                let client = client
                    .smtp_utf8(true)
                    .credentials(creds)
                    .authentication_mechanism(mechanism)
                    .connection_reuse(lettre::smtp::ConnectionReuseParameters::ReuseUnlimited);
                let mut trans = client.transport();
                match trans.connect() {
                    Ok(()) => {
                        self.transport = Some(trans);
                        self.transport_connected = true;
                        context.call_cb(Event::SmtpConnected(format!(
                            "SMTP-LOGIN as {} ok",
                            lp.send_user,
                        )));
                        return Ok(());
                    }
                    Err(err) => {
                        bail!("SMTP: failed to connect {:?}", err);
                    }
                }
            }
            Err(err) => {
                bail!("SMTP: failed to setup connection {:?}", err);
            }
        }
    }

    /// SMTP-Send a prepared mail to recipients.
    /// on successful send out Ok() is returned.
    pub fn send(
        &mut self,
        context: &Context,
        recipients: Vec<EmailAddress>,
        message: Vec<u8>,
        job_id: u32,
    ) -> Result<(), Error> {
        let message_len = message.len();

        let recipients_display = recipients
            .iter()
            .map(|x| format!("{}", x))
            .collect::<Vec<String>>()
            .join(",");

        if let Some(ref mut transport) = self.transport {
            let envelope = match Envelope::new(self.from.clone(), recipients) {
                Ok(env) => env,
                Err(err) => {
                    bail!("{}", err);
                }
            };
            let mail = SendableEmail::new(
                envelope,
                format!("{}", job_id), // only used for internal logging
                message,
            );

            match transport.send(mail) {
                Ok(_) => {
                    context.call_cb(Event::SmtpMessageSent(format!(
                        "Message len={} was smtp-sent to {}",
                        message_len, recipients_display
                    )));
                    self.transport_connected = true;
                    Ok(())
                }
                Err(err) => {
                    bail!("SMTP failed len={}: error: {}", message_len, err);
                }
            }
        } else {
            bail!(
                "uh? SMTP has no transport,  failed to send to {:?}",
                recipients_display
            );
        }
    }
}
