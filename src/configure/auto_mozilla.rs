//! # Thunderbird's Autoconfiguration implementation
//!
//! Documentation: https://developer.mozilla.org/en-US/docs/Mozilla/Thunderbird/Autoconfiguration
use quick_xml::events::{BytesStart, Event};

use std::io::BufRead;
use std::str::FromStr;

use crate::context::Context;
use crate::login_param::LoginParam;
use crate::provider::Socket;

use super::read_url::read_url;
use super::Error;

#[derive(Debug)]
struct Server {
    pub typ: String,
    pub hostname: String,
    pub port: u16,
    pub sockettype: Socket,
    pub username: String,
}

#[derive(Debug)]
struct MozAutoconfigure {
    pub incoming_servers: Vec<Server>,
    pub outgoing_servers: Vec<Server>,
}

#[derive(Debug)]
enum MozConfigTag {
    Undefined,
    Hostname,
    Port,
    Sockettype,
    Username,
}

impl Default for MozConfigTag {
    fn default() -> Self {
        Self::Undefined
    }
}

impl FromStr for MozConfigTag {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_ref() {
            "hostname" => Ok(MozConfigTag::Hostname),
            "port" => Ok(MozConfigTag::Port),
            "sockettype" => Ok(MozConfigTag::Sockettype),
            "username" => Ok(MozConfigTag::Username),
            _ => Err(()),
        }
    }
}

/// Parses a single IncomingServer or OutgoingServer section.
fn parse_server<B: BufRead>(
    reader: &mut quick_xml::Reader<B>,
    server_event: &BytesStart,
) -> Result<Option<Server>, quick_xml::Error> {
    let end_tag = String::from_utf8_lossy(server_event.name())
        .trim()
        .to_lowercase();

    let typ = server_event
        .attributes()
        .find(|attr| {
            attr.as_ref()
                .map(|a| String::from_utf8_lossy(a.key).trim().to_lowercase() == "type")
                .unwrap_or_default()
        })
        .map(|typ| {
            typ.unwrap()
                .unescape_and_decode_value(reader)
                .unwrap_or_default()
                .to_lowercase()
        })
        .unwrap_or_default();

    let mut hostname = None;
    let mut port = None;
    let mut sockettype = Socket::Automatic;
    let mut username = None;

    let mut tag_config = MozConfigTag::Undefined;
    let mut buf = Vec::new();
    loop {
        match reader.read_event(&mut buf)? {
            Event::Start(ref event) => {
                tag_config = String::from_utf8_lossy(event.name())
                    .parse()
                    .unwrap_or_default();
            }
            Event::End(ref event) => {
                let tag = String::from_utf8_lossy(event.name()).trim().to_lowercase();

                if tag == end_tag {
                    break;
                }
            }
            Event::Text(ref event) => {
                let val = event
                    .unescape_and_decode(reader)
                    .unwrap_or_default()
                    .trim()
                    .to_owned();

                match tag_config {
                    MozConfigTag::Hostname => hostname = Some(val),
                    MozConfigTag::Port => port = Some(val.parse().unwrap_or_default()),
                    MozConfigTag::Username => username = Some(val),
                    MozConfigTag::Sockettype => {
                        let val_lower = val.to_lowercase();
                        if val_lower == "ssl" {
                            sockettype = Socket::SSL;
                        }
                        if val_lower == "starttls" {
                            sockettype = Socket::STARTTLS;
                        }
                        if val_lower == "plain" {
                            sockettype = Socket::Plain;
                        }
                    }
                    _ => {}
                }
            }
            Event::Eof => break,
            _ => (),
        }
    }

    if let (Some(hostname), Some(port), Some(username)) = (hostname, port, username) {
        Ok(Some(Server {
            typ,
            hostname,
            port,
            sockettype,
            username,
        }))
    } else {
        Ok(None)
    }
}

fn parse_xml_reader<B: BufRead>(
    reader: &mut quick_xml::Reader<B>,
) -> Result<MozAutoconfigure, quick_xml::Error> {
    let mut incoming_servers = Vec::new();
    let mut outgoing_servers = Vec::new();

    let mut buf = Vec::new();
    loop {
        match reader.read_event(&mut buf)? {
            Event::Start(ref event) => {
                let tag = String::from_utf8_lossy(event.name()).trim().to_lowercase();

                if tag == "incomingserver" {
                    if let Some(incoming_server) = parse_server(reader, event)? {
                        incoming_servers.push(incoming_server);
                    }
                } else if tag == "outgoingserver" {
                    if let Some(outgoing_server) = parse_server(reader, event)? {
                        outgoing_servers.push(outgoing_server);
                    }
                }
            }
            Event::Eof => break,
            _ => (),
        }
        buf.clear();
    }

    Ok(MozAutoconfigure {
        incoming_servers,
        outgoing_servers,
    })
}

/// Parses XML and fills in address and domain placeholders.
fn parse_xml_with_address(in_emailaddr: &str, xml_raw: &str) -> Result<MozAutoconfigure, Error> {
    // Split address into local part and domain part.
    let parts: Vec<&str> = in_emailaddr.rsplitn(2, '@').collect();
    let (in_emaillocalpart, in_emaildomain) = match &parts[..] {
        [domain, local] => (local, domain),
        _ => return Err(Error::InvalidEmailAddress(in_emailaddr.to_string())),
    };

    let mut reader = quick_xml::Reader::from_str(xml_raw);
    reader.trim_text(true);

    let moz_ac = parse_xml_reader(&mut reader).map_err(|error| Error::InvalidXml {
        position: reader.buffer_position(),
        error,
    })?;

    let fill_placeholders = |val: &str| -> String {
        val.replace("%EMAILADDRESS%", in_emailaddr)
            .replace("%EMAILLOCALPART%", in_emaillocalpart)
            .replace("%EMAILDOMAIN%", in_emaildomain)
    };

    let fill_server_placeholders = |server: Server| -> Server {
        Server {
            typ: server.typ,
            hostname: fill_placeholders(&server.hostname),
            port: server.port,
            sockettype: server.sockettype,
            username: fill_placeholders(&server.username),
        }
    };

    Ok(MozAutoconfigure {
        incoming_servers: moz_ac
            .incoming_servers
            .into_iter()
            .map(fill_server_placeholders)
            .collect(),
        outgoing_servers: moz_ac
            .outgoing_servers
            .into_iter()
            .map(fill_server_placeholders)
            .collect(),
    })
}

/// Parses XML into `LoginParam` structure.
fn parse_loginparam(in_emailaddr: &str, xml_raw: &str) -> Result<LoginParam, Error> {
    let moz_ac = parse_xml_with_address(in_emailaddr, xml_raw)?;

    let mut login_param = LoginParam::new();
    if let Some(imap_server) = moz_ac
        .incoming_servers
        .into_iter()
        .find(|incoming_server| incoming_server.typ == "imap")
    {
        login_param.imap.server = imap_server.hostname;
        login_param.imap.port = imap_server.port;
        login_param.imap.security = imap_server.sockettype;
        login_param.imap.user = imap_server.username;
    }

    if let Some(smtp_server) = moz_ac
        .outgoing_servers
        .into_iter()
        .find(|outgoing_server| outgoing_server.typ == "smtp")
    {
        login_param.smtp.server = smtp_server.hostname;
        login_param.smtp.port = smtp_server.port;
        login_param.smtp.security = smtp_server.sockettype;
        login_param.smtp.user = smtp_server.username;
    }

    if login_param.imap.server.is_empty()
        || login_param.imap.port == 0
        || login_param.smtp.server.is_empty()
        || login_param.smtp.port == 0
    {
        Err(Error::IncompleteAutoconfig(login_param))
    } else {
        Ok(login_param)
    }
}

pub async fn moz_autoconfigure(
    context: &Context,
    url: &str,
    param_in: &LoginParam,
) -> Result<LoginParam, Error> {
    let xml_raw = read_url(context, url).await?;

    let res = parse_loginparam(&param_in.addr, &xml_raw);
    if let Err(err) = &res {
        warn!(
            context,
            "Failed to parse Thunderbird autoconfiguration XML: {}", err
        );
    }
    res
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_outlook_autoconfig() {
        let xml_raw = include_str!("../../test-data/autoconfig/outlook.com.xml");
        let res = parse_loginparam("example@outlook.com", xml_raw).expect("XML parsing failed");
        assert_eq!(res.imap.server, "outlook.office365.com");
        assert_eq!(res.imap.port, 993);
        assert_eq!(res.smtp.server, "smtp.office365.com");
        assert_eq!(res.smtp.port, 587);
    }

    #[test]
    fn test_parse_lakenet_autoconfig() {
        let xml_raw = include_str!("../../test-data/autoconfig/lakenet.ch.xml");
        let res =
            parse_xml_with_address("example@lakenet.ch", xml_raw).expect("XML parsing failed");

        assert_eq!(res.incoming_servers.len(), 4);

        assert_eq!(res.incoming_servers[0].typ, "imap");
        assert_eq!(res.incoming_servers[0].hostname, "mail.lakenet.ch");
        assert_eq!(res.incoming_servers[0].port, 993);
        assert_eq!(res.incoming_servers[0].sockettype, Socket::SSL);
        assert_eq!(res.incoming_servers[0].username, "example@lakenet.ch");

        assert_eq!(res.incoming_servers[1].typ, "imap");
        assert_eq!(res.incoming_servers[1].hostname, "mail.lakenet.ch");
        assert_eq!(res.incoming_servers[1].port, 143);
        assert_eq!(res.incoming_servers[1].sockettype, Socket::STARTTLS);
        assert_eq!(res.incoming_servers[1].username, "example@lakenet.ch");

        assert_eq!(res.incoming_servers[2].typ, "pop3");
        assert_eq!(res.incoming_servers[2].hostname, "mail.lakenet.ch");
        assert_eq!(res.incoming_servers[2].port, 995);
        assert_eq!(res.incoming_servers[2].sockettype, Socket::SSL);
        assert_eq!(res.incoming_servers[2].username, "example@lakenet.ch");

        assert_eq!(res.incoming_servers[3].typ, "pop3");
        assert_eq!(res.incoming_servers[3].hostname, "mail.lakenet.ch");
        assert_eq!(res.incoming_servers[3].port, 110);
        assert_eq!(res.incoming_servers[3].sockettype, Socket::STARTTLS);
        assert_eq!(res.incoming_servers[3].username, "example@lakenet.ch");

        assert_eq!(res.outgoing_servers.len(), 1);

        assert_eq!(res.outgoing_servers[0].typ, "smtp");
        assert_eq!(res.outgoing_servers[0].hostname, "mail.lakenet.ch");
        assert_eq!(res.outgoing_servers[0].port, 587);
        assert_eq!(res.outgoing_servers[0].sockettype, Socket::STARTTLS);
        assert_eq!(res.outgoing_servers[0].username, "example@lakenet.ch");
    }
}
