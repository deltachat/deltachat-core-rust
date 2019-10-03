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
    pub fn connect(&mut self, context: &Context, lp: &LoginParam) -> bool {
        if self.is_connected() {
            warn!(context, "SMTP already connected.");
            return true;
        }

        if lp.send_server.is_empty() || lp.send_port == 0 {
            context.call_cb(Event::ErrorNetwork("SMTP bad parameters.".into()));
        }

        self.from = if let Ok(addr) = EmailAddress::new(lp.addr.clone()) {
            Some(addr)
        } else {
            None
        };

        if self.from.is_none() {
            // TODO: print error
            return false;
        }

        let domain = &lp.send_server;
        let port = lp.send_port as u16;

        let tls = dc_build_tls(lp.smtp_certificate_checks).unwrap();
        let tls_parameters = ClientTlsParameters::new(domain.to_string(), tls);

        let creds = if 0 != lp.server_flags & (DC_LP_AUTH_OAUTH2 as i32) {
            // oauth2
            let addr = &lp.addr;
            let send_pw = &lp.send_pw;
            let access_token = dc_get_oauth2_access_token(context, addr, send_pw, false);
            if access_token.is_none() {
                return false;
            }
            let user = &lp.send_user;

            lettre::smtp::authentication::Credentials::new(
                user.to_string(),
                access_token.unwrap_or_default(),
            )
        } else {
            // plain
            let user = lp.send_user.clone();
            let pw = lp.send_pw.clone();
            lettre::smtp::authentication::Credentials::new(user, pw)
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
                    .connection_reuse(lettre::smtp::ConnectionReuseParameters::ReuseUnlimited);
                self.transport = Some(client.transport());
                context.call_cb(Event::SmtpConnected(format!(
                    "SMTP-LOGIN as {} ok",
                    lp.send_user,
                )));
                true
            }
            Err(err) => {
                warn!(context, "SMTP: failed to establish connection {:?}", err);
                false
            }
        }
    }

    /// SMTP-Send a prepared mail to recipients.
    /// returns boolean whether send was successful.
    pub fn send<'a>(
        &mut self,
        context: &Context,
        recipients: Vec<EmailAddress>,
        message: Vec<u8>,
    ) -> Result<(), Error> {
        let message_len = message.len();

        let recipients_display = recipients
            .iter()
            .map(|x| format!("{}", x))
            .collect::<Vec<String>>()
            .join(",");

        if let Some(ref mut transport) = self.transport {
            let envelope = Envelope::new(self.from.clone(), recipients);
            ensure!(envelope.is_ok(), "internal smtp-message construction fail");
            let envelope = envelope.unwrap();
            let mail = SendableEmail::new(
                envelope,
                "mail-id".into(), // TODO: random id
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
