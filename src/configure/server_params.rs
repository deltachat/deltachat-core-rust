//! Variable server parameters lists

use crate::provider::{Protocol, Socket};

/// Set of variable parameters to try during configuration.
///
/// Can be loaded from offline provider database, online configuraiton
/// or derived from user entered parameters.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ServerParams {
    /// Protocol, such as IMAP or SMTP.
    pub protocol: Protocol,

    /// Server hostname, empty if unknown.
    pub hostname: String,

    /// Server port, zero if unknown.
    pub port: u16,

    /// Socket security, such as TLS or STARTTLS, Socket::Automatic if unknown.
    pub socket: Socket,

    /// Username, empty if unknown.
    pub username: String,

    /// Whether TLS certificates should be strictly checked or not, `None` for automatic.
    pub strict_tls: Option<bool>,
}

impl ServerParams {
    fn expand_usernames(self, addr: &str) -> Vec<ServerParams> {
        let mut res = Vec::new();

        if self.username.is_empty() {
            res.push(Self {
                username: addr.to_string(),
                ..self.clone()
            });

            if let Some(at) = addr.find('@') {
                res.push(Self {
                    username: addr.split_at(at).0.to_string(),
                    ..self
                });
            }
        } else {
            res.push(self)
        }
        res
    }

    fn expand_hostnames(self, param_domain: &str) -> Vec<ServerParams> {
        if self.hostname.is_empty() {
            vec![
                Self {
                    hostname: param_domain.to_string(),
                    ..self.clone()
                },
                Self {
                    hostname: match self.protocol {
                        Protocol::Imap => "imap.".to_string() + param_domain,
                        Protocol::Smtp => "smtp.".to_string() + param_domain,
                    },
                    ..self.clone()
                },
                Self {
                    hostname: "mail.".to_string() + param_domain,
                    ..self
                },
            ]
        } else {
            vec![self]
        }
    }

    fn expand_ports(mut self) -> Vec<ServerParams> {
        // Try to infer port from socket security.
        if self.port == 0 {
            self.port = match self.socket {
                Socket::Ssl => match self.protocol {
                    Protocol::Imap => 993,
                    Protocol::Smtp => 465,
                },
                Socket::Starttls | Socket::Plain => match self.protocol {
                    Protocol::Imap => 143,
                    Protocol::Smtp => 587,
                },
                Socket::Automatic => 0,
            }
        }

        if self.port == 0 {
            // Neither port nor security is set.
            //
            // Try common secure combinations.

            vec![
                // Try STARTTLS
                Self {
                    socket: Socket::Starttls,
                    port: match self.protocol {
                        Protocol::Imap => 143,
                        Protocol::Smtp => 587,
                    },
                    ..self.clone()
                },
                // Try TLS
                Self {
                    socket: Socket::Ssl,
                    port: match self.protocol {
                        Protocol::Imap => 993,
                        Protocol::Smtp => 465,
                    },
                    ..self
                },
            ]
        } else if self.socket == Socket::Automatic {
            vec![
                // Try TLS over user-provided port.
                Self {
                    socket: Socket::Ssl,
                    ..self.clone()
                },
                // Try STARTTLS over user-provided port.
                Self {
                    socket: Socket::Starttls,
                    ..self
                },
            ]
        } else {
            vec![self]
        }
    }

    fn expand_strict_tls(self) -> Vec<ServerParams> {
        if self.strict_tls.is_none() {
            vec![
                Self {
                    strict_tls: Some(true), // Strict.
                    ..self.clone()
                },
                Self {
                    strict_tls: None, // Automatic.
                    ..self
                },
            ]
        } else {
            vec![self]
        }
    }
}

/// Expands vector of `ServerParams`, replacing placeholders with
/// variants to try.
pub(crate) fn expand_param_vector(
    v: Vec<ServerParams>,
    addr: &str,
    domain: &str,
) -> Vec<ServerParams> {
    v.into_iter()
        .map(|params| {
            if params.socket == Socket::Plain {
                ServerParams {
                    // Avoid expanding plaintext configuration into configuration with and without
                    // `strict_tls` if `strict_tls` is set to `None` as `strict_tls` is not used for
                    // plaintext connections. Always setting it to "enabled", just in case.
                    strict_tls: Some(true),
                    ..params
                }
            } else {
                params
            }
        })
        // The order of expansion is important.
        //
        // Ports are expanded the last, so they are changed the first.  Username is only changed if
        // default value (address with domain) didn't work for all available hosts and ports.
        //
        // Strict TLS must be expanded first, so we try all configurations with strict TLS first
        // and only then try again without strict TLS. Otherwise we may lock to wrong hostname
        // without strict TLS when another hostname with strict TLS is available.  For example, if
        // both smtp.example.net and mail.example.net are running an SMTP server, but both use a
        // certificate that is only valid for mail.example.net, we want to skip smtp.example.net
        // and use mail.example.net with strict TLS instead of using smtp.example.net without
        // strict TLS.
        .flat_map(|params| params.expand_strict_tls().into_iter())
        .flat_map(|params| params.expand_usernames(addr).into_iter())
        .flat_map(|params| params.expand_hostnames(domain).into_iter())
        .flat_map(|params| params.expand_ports().into_iter())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_param_vector() {
        let v = expand_param_vector(
            vec![ServerParams {
                protocol: Protocol::Imap,
                hostname: "example.net".to_string(),
                port: 0,
                socket: Socket::Ssl,
                username: "foobar".to_string(),
                strict_tls: Some(true),
            }],
            "foobar@example.net",
            "example.net",
        );

        assert_eq!(
            v,
            vec![ServerParams {
                protocol: Protocol::Imap,
                hostname: "example.net".to_string(),
                port: 993,
                socket: Socket::Ssl,
                username: "foobar".to_string(),
                strict_tls: Some(true)
            }],
        );

        let v = expand_param_vector(
            vec![ServerParams {
                protocol: Protocol::Smtp,
                hostname: "example.net".to_string(),
                port: 123,
                socket: Socket::Automatic,
                username: "foobar".to_string(),
                strict_tls: None,
            }],
            "foobar@example.net",
            "example.net",
        );

        assert_eq!(
            v,
            vec![
                ServerParams {
                    protocol: Protocol::Smtp,
                    hostname: "example.net".to_string(),
                    port: 123,
                    socket: Socket::Ssl,
                    username: "foobar".to_string(),
                    strict_tls: Some(true),
                },
                ServerParams {
                    protocol: Protocol::Smtp,
                    hostname: "example.net".to_string(),
                    port: 123,
                    socket: Socket::Starttls,
                    username: "foobar".to_string(),
                    strict_tls: Some(true)
                },
                ServerParams {
                    protocol: Protocol::Smtp,
                    hostname: "example.net".to_string(),
                    port: 123,
                    socket: Socket::Ssl,
                    username: "foobar".to_string(),
                    strict_tls: None,
                },
                ServerParams {
                    protocol: Protocol::Smtp,
                    hostname: "example.net".to_string(),
                    port: 123,
                    socket: Socket::Starttls,
                    username: "foobar".to_string(),
                    strict_tls: None
                }
            ],
        );

        // Test that strict_tls is not expanded for plaintext connections.
        let v = expand_param_vector(
            vec![ServerParams {
                protocol: Protocol::Smtp,
                hostname: "example.net".to_string(),
                port: 123,
                socket: Socket::Plain,
                username: "foobar".to_string(),
                strict_tls: None,
            }],
            "foobar@example.net",
            "example.net",
        );
        assert_eq!(
            v,
            vec![ServerParams {
                protocol: Protocol::Smtp,
                hostname: "example.net".to_string(),
                port: 123,
                socket: Socket::Plain,
                username: "foobar".to_string(),
                strict_tls: Some(true)
            }],
        );
    }
}
