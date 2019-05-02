use std::ffi::{CStr, CString};

use lettre::smtp::client::net::*;
use lettre::*;
use libc;
use native_tls::TlsConnector;

use crate::constants::Event;
use crate::constants::*;
use crate::dc_context::dc_context_t;
use crate::dc_log::*;
use crate::dc_loginparam::*;
use crate::dc_oauth2::*;
use crate::x::*;

pub struct Smtp {
    transport: Option<lettre::smtp::SmtpTransport>,
    transport_connected: bool,
    /// Email address we are sending from.
    from: Option<EmailAddress>,
    pub error: *mut libc::c_char,
}

impl Smtp {
    /// Create a new Smtp instances.
    pub fn new() -> Self {
        Smtp {
            transport: None,
            transport_connected: false,
            from: None,
            error: std::ptr::null_mut(),
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
    pub fn connect(&mut self, context: &dc_context_t, lp: *const dc_loginparam_t) -> usize {
        if lp.is_null() {
            return 0;
        }

        if self.is_connected() {
            unsafe {
                dc_log_warning(
                    context,
                    0,
                    b"SMTP already connected.\x00" as *const u8 as *const libc::c_char,
                )
            };

            return 1;
        }

        // Safe because we checked for null pointer above.
        let lp = unsafe { *lp };

        if lp.addr.is_null() || lp.send_server.is_null() || lp.send_port == 0 {
            unsafe {
                dc_log_event(
                    context,
                    Event::ERROR_NETWORK,
                    0,
                    b"SMTP bad parameters.\x00" as *const u8 as *const libc::c_char,
                );
            }
        }

        let raw_addr = unsafe {
            CStr::from_ptr(lp.addr)
                .to_str()
                .expect("invalid from address")
                .to_string()
        };
        self.from = if let Ok(addr) = EmailAddress::new(raw_addr) {
            Some(addr)
        } else {
            None
        };

        if self.from.is_none() {
            // TODO: print error
            return 0;
        }

        let domain = unsafe {
            CStr::from_ptr(lp.send_server)
                .to_str()
                .expect("invalid send server")
        };
        let port = lp.send_port as u16;

        let mut tls_builder = TlsConnector::builder();
        tls_builder.min_protocol_version(Some(DEFAULT_TLS_PROTOCOLS[0]));

        let tls_parameters =
            ClientTlsParameters::new(domain.to_string(), tls_builder.build().unwrap());

        let creds = if 0 != lp.server_flags & (DC_LP_AUTH_OAUTH2 as i32) {
            // oauth2

            let mut access_token =
                unsafe { dc_get_oauth2_access_token(context, lp.addr, lp.send_pw, 0i32) };
            if access_token.is_null() {
                return 0;
            }

            let user = unsafe { CStr::from_ptr(lp.send_user).to_str().unwrap().to_string() };
            let token = unsafe { CStr::from_ptr(access_token).to_str().unwrap().to_string() };
            unsafe { free(access_token as *mut libc::c_void) };

            lettre::smtp::authentication::Credentials::new(user, token)
        } else {
            // plain
            let user = unsafe { CStr::from_ptr(lp.send_user).to_str().unwrap().to_string() };
            let pw = unsafe { CStr::from_ptr(lp.send_pw).to_str().unwrap().to_string() };
            lettre::smtp::authentication::Credentials::new(user, pw)
        };

        let security = if 0
            != lp.server_flags & (DC_LP_SMTP_SOCKET_STARTTLS | DC_LP_SMTP_SOCKET_PLAIN) as i32
        {
            lettre::smtp::ClientSecurity::Opportunistic(tls_parameters)
        } else {
            lettre::smtp::ClientSecurity::Wrapper(tls_parameters)
        };

        let client = lettre::smtp::SmtpClient::new((domain, port), security)
            .expect("failed to construct stmp client")
            // .smtp_utf8(true)
            .credentials(creds)
            .connection_reuse(lettre::smtp::ConnectionReuseParameters::ReuseUnlimited);

        self.transport = Some(client.transport());

        1
    }

    pub fn send<'a>(
        &mut self,
        context: &dc_context_t,
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
                    unsafe {
                        dc_log_event(
                            context,
                            Event::SMTP_MESSAGE_SENT,
                            0,
                            b"Message was sent to SMTP server\x00" as *const u8
                                as *const libc::c_char,
                        );
                    }
                    self.transport_connected = true;
                    1
                }
                Err(err) => {
                    let error_msg = format!("SMTP failed to send message: {:?}", err);
                    let msg = CString::new(error_msg).unwrap();
                    self.error = unsafe { libc::strdup(msg.as_ptr()) };

                    unsafe {
                        dc_log_warning(
                            context,
                            0,
                            b"%s\x00" as *const u8 as *const libc::c_char,
                            msg,
                        );
                    }

                    0
                }
            }
        } else {
            // TODO: log error
            0
        }
    }
}
