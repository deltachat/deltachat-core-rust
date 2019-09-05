use lettre::smtp::client::net::*;
use lettre::*;

use crate::constants::Event;
use crate::constants::*;
use crate::context::Context;
use crate::dc_loginparam::*;
use crate::oauth2::*;

pub struct Smtp {
    transport: Option<lettre::smtp::SmtpTransport>,
    transport_connected: bool,
    /// Email address we are sending from.
    from: Option<EmailAddress>,
    pub error: Option<String>,
}

impl Smtp {
    /// Create a new Smtp instances.
    pub fn new() -> Self {
        Smtp {
            transport: None,
            transport_connected: false,
            from: None,
            error: None,
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
    pub fn connect(&mut self, context: &Context, lp: &dc_loginparam_t) -> bool {
        if self.is_connected() {
            warn!(context, 0, "SMTP already connected.");
            return true;
        }

        if lp.send_server.is_empty() || lp.send_port == 0 {
            log_event!(context, Event::ERROR_NETWORK, 0, "SMTP bad parameters.",);
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

        let tls = native_tls::TlsConnector::builder()
            // FIXME: unfortunately this is needed to make things work on macos + testrun.org
            .danger_accept_invalid_hostnames(true)
            .min_protocol_version(Some(DEFAULT_TLS_PROTOCOLS[0]))
            .build()
            .unwrap();

        let tls_parameters = ClientTlsParameters::new(domain.to_string(), tls);

        let creds = if 0 != lp.server_flags & (DC_LP_AUTH_OAUTH2 as i32) {
            // oauth2
            let addr = &lp.addr;
            let send_pw = &lp.send_pw;
            let access_token = dc_get_oauth2_access_token(context, addr, send_pw, 0);
            if access_token.is_none() {
                return false;
            }
            let user = &lp.send_user;

            lettre::smtp::authentication::Credentials::new(user.to_string(), access_token.unwrap())
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
                log_event!(
                    context,
                    Event::SMTP_CONNECTED,
                    0,
                    "SMTP-LOGIN as {} ok",
                    lp.send_user,
                );
                true
            }
            Err(err) => {
                warn!(context, 0, "SMTP: failed to establish connection {:?}", err);
                false
            }
        }
    }

    pub fn send<'a>(
        &mut self,
        context: &Context,
        recipients: Vec<EmailAddress>,
        body: Vec<u8>,
    ) -> usize {
        if let Some(ref mut transport) = self.transport {
            let envelope = Envelope::new(self.from.clone(), recipients).expect("invalid envelope");
            let mail = SendableEmail::new(
                envelope,
                "mail-id".into(), // TODO: random id
                body,
            );

            match transport.send(mail) {
                Ok(_) => {
                    log_event!(
                        context,
                        Event::SMTP_MESSAGE_SENT,
                        0,
                        "Message was sent to SMTP server",
                    );
                    self.transport_connected = true;
                    1
                }
                Err(err) => {
                    warn!(context, 0, "SMTP failed to send message: {}", err);
                    0
                }
            }
        } else {
            // TODO: log error
            0
        }
    }
}
