//! # Thunderbird's Autoconfiguration implementation
//!
//! Documentation: https://developer.mozilla.org/en-US/docs/Mozilla/Thunderbird/Autoconfiguration */
use quick_xml;
use quick_xml::events::{BytesEnd, BytesStart, BytesText};

use crate::constants::*;
use crate::context::Context;
use crate::login_param::LoginParam;

use super::read_url::read_url;

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "Invalid email address: {:?}", _0)]
    InvalidEmailAddress(String),

    #[fail(display = "XML error at position {}", position)]
    InvalidXml {
        position: usize,
        #[cause]
        error: quick_xml::Error,
    },

    #[fail(display = "Bad or incomplete autoconfig")]
    IncompleteAutoconfig(LoginParam),

    #[fail(display = "Failed to get URL {}", _0)]
    ReadUrlError(#[cause] super::read_url::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

impl From<super::read_url::Error> for Error {
    fn from(err: super::read_url::Error) -> Error {
        Error::ReadUrlError(err)
    }
}

#[derive(Debug)]
struct MozAutoconfigure<'a> {
    pub in_emailaddr: &'a str,
    pub in_emaildomain: &'a str,
    pub in_emaillocalpart: &'a str,
    pub out: LoginParam,
    pub out_imap_set: bool,
    pub out_smtp_set: bool,
    pub tag_server: MozServer,
    pub tag_config: MozConfigTag,
}

#[derive(Debug, PartialEq)]
enum MozServer {
    Undefined,
    Imap,
    Smtp,
}

#[derive(Debug)]
enum MozConfigTag {
    Undefined,
    Hostname,
    Port,
    Sockettype,
    Username,
}

fn parse_xml(in_emailaddr: &str, xml_raw: &str) -> Result<LoginParam> {
    let mut reader = quick_xml::Reader::from_str(xml_raw);
    reader.trim_text(true);

    // Split address into local part and domain part.
    let p = in_emailaddr
        .find('@')
        .ok_or_else(|| Error::InvalidEmailAddress(in_emailaddr.to_string()))?;
    let (in_emaillocalpart, in_emaildomain) = in_emailaddr.split_at(p);
    let in_emaildomain = &in_emaildomain[1..];

    let mut moz_ac = MozAutoconfigure {
        in_emailaddr,
        in_emaildomain,
        in_emaillocalpart,
        out: LoginParam::new(),
        out_imap_set: false,
        out_smtp_set: false,
        tag_server: MozServer::Undefined,
        tag_config: MozConfigTag::Undefined,
    };

    let mut buf = Vec::new();
    loop {
        let event = reader
            .read_event(&mut buf)
            .map_err(|error| Error::InvalidXml {
                position: reader.buffer_position(),
                error,
            })?;

        match event {
            quick_xml::events::Event::Start(ref e) => {
                moz_autoconfigure_starttag_cb(e, &mut moz_ac, &reader)
            }
            quick_xml::events::Event::End(ref e) => moz_autoconfigure_endtag_cb(e, &mut moz_ac),
            quick_xml::events::Event::Text(ref e) => {
                moz_autoconfigure_text_cb(e, &mut moz_ac, &reader)
            }
            quick_xml::events::Event::Eof => break,
            _ => (),
        }
        buf.clear();
    }

    if moz_ac.out.mail_server.is_empty()
        || moz_ac.out.mail_port == 0
        || moz_ac.out.send_server.is_empty()
        || moz_ac.out.send_port == 0
    {
        Err(Error::IncompleteAutoconfig(moz_ac.out))
    } else {
        Ok(moz_ac.out)
    }
}

pub fn moz_autoconfigure(
    context: &Context,
    url: &str,
    param_in: &LoginParam,
) -> Result<LoginParam> {
    let xml_raw = read_url(context, url)?;

    let res = parse_xml(&param_in.addr, &xml_raw);
    if let Err(err) = &res {
        warn!(
            context,
            "Failed to parse Thunderbird autoconfiguration XML: {}", err
        );
    }
    res
}

fn moz_autoconfigure_text_cb<B: std::io::BufRead>(
    event: &BytesText,
    moz_ac: &mut MozAutoconfigure,
    reader: &quick_xml::Reader<B>,
) {
    let val = event.unescape_and_decode(reader).unwrap_or_default();

    let addr = moz_ac.in_emailaddr;
    let email_local = moz_ac.in_emaillocalpart;
    let email_domain = moz_ac.in_emaildomain;

    let val = val
        .trim()
        .replace("%EMAILADDRESS%", addr)
        .replace("%EMAILLOCALPART%", email_local)
        .replace("%EMAILDOMAIN%", email_domain);

    match moz_ac.tag_server {
        MozServer::Imap => match moz_ac.tag_config {
            MozConfigTag::Hostname => moz_ac.out.mail_server = val,
            MozConfigTag::Port => moz_ac.out.mail_port = val.parse().unwrap_or_default(),
            MozConfigTag::Username => moz_ac.out.mail_user = val,
            MozConfigTag::Sockettype => {
                let val_lower = val.to_lowercase();
                if val_lower == "ssl" {
                    moz_ac.out.server_flags |= DC_LP_IMAP_SOCKET_SSL as i32
                }
                if val_lower == "starttls" {
                    moz_ac.out.server_flags |= DC_LP_IMAP_SOCKET_STARTTLS as i32
                }
                if val_lower == "plain" {
                    moz_ac.out.server_flags |= DC_LP_IMAP_SOCKET_PLAIN as i32
                }
            }
            _ => {}
        },
        MozServer::Smtp => match moz_ac.tag_config {
            MozConfigTag::Hostname => moz_ac.out.send_server = val,
            MozConfigTag::Port => moz_ac.out.send_port = val.parse().unwrap_or_default(),
            MozConfigTag::Username => moz_ac.out.send_user = val,
            MozConfigTag::Sockettype => {
                let val_lower = val.to_lowercase();
                if val_lower == "ssl" {
                    moz_ac.out.server_flags |= DC_LP_SMTP_SOCKET_SSL as i32
                }
                if val_lower == "starttls" {
                    moz_ac.out.server_flags |= DC_LP_SMTP_SOCKET_STARTTLS as i32
                }
                if val_lower == "plain" {
                    moz_ac.out.server_flags |= DC_LP_SMTP_SOCKET_PLAIN as i32
                }
            }
            _ => {}
        },
        MozServer::Undefined => {}
    }
}

fn moz_autoconfigure_endtag_cb(event: &BytesEnd, moz_ac: &mut MozAutoconfigure) {
    let tag = String::from_utf8_lossy(event.name()).trim().to_lowercase();

    if tag == "incomingserver" {
        if moz_ac.tag_server == MozServer::Imap {
            moz_ac.out_imap_set = true;
        }
        moz_ac.tag_server = MozServer::Undefined;
        moz_ac.tag_config = MozConfigTag::Undefined;
    } else if tag == "outgoingserver" {
        if moz_ac.tag_server == MozServer::Smtp {
            moz_ac.out_smtp_set = true;
        }
        moz_ac.tag_server = MozServer::Undefined;
        moz_ac.tag_config = MozConfigTag::Undefined;
    } else {
        moz_ac.tag_config = MozConfigTag::Undefined;
    }
}

fn moz_autoconfigure_starttag_cb<B: std::io::BufRead>(
    event: &BytesStart,
    moz_ac: &mut MozAutoconfigure,
    reader: &quick_xml::Reader<B>,
) {
    let tag = String::from_utf8_lossy(event.name()).trim().to_lowercase();

    if tag == "incomingserver" {
        moz_ac.tag_server = if let Some(typ) = event.attributes().find(|attr| {
            attr.as_ref()
                .map(|a| String::from_utf8_lossy(a.key).trim().to_lowercase() == "type")
                .unwrap_or_default()
        }) {
            let typ = typ
                .unwrap()
                .unescape_and_decode_value(reader)
                .unwrap_or_default()
                .to_lowercase();

            if typ == "imap" && !moz_ac.out_imap_set {
                MozServer::Imap
            } else {
                MozServer::Undefined
            }
        } else {
            MozServer::Undefined
        };
        moz_ac.tag_config = MozConfigTag::Undefined;
    } else if tag == "outgoingserver" {
        moz_ac.tag_server = if !moz_ac.out_smtp_set {
            MozServer::Smtp
        } else {
            MozServer::Undefined
        };
        moz_ac.tag_config = MozConfigTag::Undefined;
    } else if tag == "hostname" {
        moz_ac.tag_config = MozConfigTag::Hostname;
    } else if tag == "port" {
        moz_ac.tag_config = MozConfigTag::Port;
    } else if tag == "sockettype" {
        moz_ac.tag_config = MozConfigTag::Sockettype;
    } else if tag == "username" {
        moz_ac.tag_config = MozConfigTag::Username;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_outlook_autoconfig() {
        // Copied from https://autoconfig.thunderbird.net/v1.1/outlook.com on 2019-10-11
        let xml_raw =
"<clientConfig version=\"1.1\">
  <emailProvider id=\"outlook.com\">
    <domain>hotmail.com</domain>
    <domain>hotmail.co.uk</domain>
    <domain>hotmail.co.jp</domain>
    <domain>hotmail.com.br</domain>
    <domain>hotmail.de</domain>
    <domain>hotmail.fr</domain>
    <domain>hotmail.it</domain>
    <domain>hotmail.es</domain>
    <domain>live.com</domain>
    <domain>live.co.uk</domain>
    <domain>live.co.jp</domain>
    <domain>live.de</domain>
    <domain>live.fr</domain>
    <domain>live.it</domain>
    <domain>live.jp</domain>
    <domain>msn.com</domain>
    <domain>outlook.com</domain>
    <displayName>Outlook.com (Microsoft)</displayName>
    <displayShortName>Outlook</displayShortName>
    <incomingServer type=\"exchange\">
      <hostname>outlook.office365.com</hostname>
      <port>443</port>
      <username>%EMAILADDRESS%</username>
      <socketType>SSL</socketType>
      <authentication>OAuth2</authentication>
      <owaURL>https://outlook.office365.com/owa/</owaURL>
      <ewsURL>https://outlook.office365.com/ews/exchange.asmx</ewsURL>
      <useGlobalPreferredServer>true</useGlobalPreferredServer>
    </incomingServer>
    <incomingServer type=\"imap\">
      <hostname>outlook.office365.com</hostname>
      <port>993</port>
      <socketType>SSL</socketType>
      <authentication>password-cleartext</authentication>
      <username>%EMAILADDRESS%</username>
    </incomingServer>
    <incomingServer type=\"pop3\">
      <hostname>outlook.office365.com</hostname>
      <port>995</port>
      <socketType>SSL</socketType>
      <authentication>password-cleartext</authentication>
      <username>%EMAILADDRESS%</username>
      <pop3>
        <leaveMessagesOnServer>true</leaveMessagesOnServer>
        <!-- Outlook.com docs specifically mention that POP3 deletes have effect on the main inbox on webmail and IMAP -->
      </pop3>
    </incomingServer>
    <outgoingServer type=\"smtp\">
      <hostname>smtp.office365.com</hostname>
      <port>587</port>
      <socketType>STARTTLS</socketType>
      <authentication>password-cleartext</authentication>
      <username>%EMAILADDRESS%</username>
    </outgoingServer>
    <documentation url=\"http://windows.microsoft.com/en-US/windows/outlook/send-receive-from-app\">
      <descr lang=\"en\">Set up an email app with Outlook.com</descr>
    </documentation>
  </emailProvider>
  <webMail>
    <loginPage url=\"https://www.outlook.com/\"/>
    <loginPageInfo url=\"https://www.outlook.com/\">
      <username>%EMAILADDRESS%</username>
      <usernameField id=\"i0116\" name=\"login\"/>
      <passwordField id=\"i0118\" name=\"passwd\"/>
      <loginButton id=\"idSIButton9\" name=\"SI\"/>
    </loginPageInfo>
  </webMail>
</clientConfig>";
        let res = parse_xml("example@outlook.com", xml_raw).expect("XML parsing failed");
        assert_eq!(res.mail_server, "outlook.office365.com");
        assert_eq!(res.mail_port, 993);
        assert_eq!(res.send_server, "smtp.office365.com");
        assert_eq!(res.send_port, 587);
    }
}
