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
}

impl ServerParams {
    pub(crate) fn expand_usernames(mut self, addr: &str) -> Vec<ServerParams> {
        let mut res = Vec::new();

        if self.username.is_empty() {
            self.username = addr.to_string();
            res.push(self.clone());

            if let Some(at) = addr.find('@') {
                self.username = addr.split_at(at).0.to_string();
                res.push(self);
            }
        } else {
            res.push(self)
        }
        res
    }

    pub(crate) fn expand_hostnames(mut self, param_domain: &str) -> Vec<ServerParams> {
        let mut res = Vec::new();
        if self.hostname.is_empty() {
            self.hostname = param_domain.to_string();
            res.push(self.clone());

            self.hostname = match self.protocol {
                Protocol::Imap => "imap.".to_string() + param_domain,
                Protocol::Smtp => "smtp.".to_string() + param_domain,
            };
            res.push(self.clone());

            self.hostname = "mail.".to_string() + param_domain;
            res.push(self);
        } else {
            res.push(self);
        }
        res
    }

    pub(crate) fn expand_ports(mut self) -> Vec<ServerParams> {
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

        let mut res = Vec::new();
        if self.port == 0 {
            // Neither port nor security is set.
            //
            // Try common secure combinations.

            // Try STARTTLS
            self.socket = Socket::Starttls;
            self.port = match self.protocol {
                Protocol::Imap => 143,
                Protocol::Smtp => 587,
            };
            res.push(self.clone());

            // Try TLS
            self.socket = Socket::Ssl;
            self.port = match self.protocol {
                Protocol::Imap => 993,
                Protocol::Smtp => 465,
            };
            res.push(self);
        } else if self.socket == Socket::Automatic {
            // Try TLS over user-provided port.
            self.socket = Socket::Ssl;
            res.push(self.clone());

            // Try STARTTLS over user-provided port.
            self.socket = Socket::Starttls;
            res.push(self);
        } else {
            res.push(self);
        }
        res
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
        // The order of expansion is important: ports are expanded the
        // last, so they are changed the first. Username is only
        // changed if default value (address with domain) didn't work
        // for all available hosts and ports.
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
            }],
        );
    }
}
