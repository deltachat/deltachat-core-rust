use quick_xml;
use quick_xml::events::{BytesEnd, BytesStart, BytesText};

use crate::constants::*;
use crate::context::Context;
use crate::login_param::LoginParam;

use super::read_autoconf_file;
/* ******************************************************************************
 * Thunderbird's Autoconfigure
 ******************************************************************************/
/* documentation: https://developer.mozilla.org/en-US/docs/Mozilla/Thunderbird/Autoconfiguration */
struct MozAutoconfigure<'a> {
    pub in_0: &'a LoginParam,
    pub in_emaildomain: &'a str,
    pub in_emaillocalpart: &'a str,
    pub out: LoginParam,
    pub out_imap_set: bool,
    pub out_smtp_set: bool,
    pub tag_server: MozServer,
    pub tag_config: MozConfigTag,
}

enum MozServer {
    Undefined,
    Imap,
    Smtp,
}

enum MozConfigTag {
    Undefined,
    Hostname,
    Port,
    Sockettype,
    Username,
}

pub fn moz_autoconfigure(
    context: &Context,
    url: &str,
    param_in: &LoginParam,
) -> Option<LoginParam> {
    let xml_raw = read_autoconf_file(context, url)?;

    // Split address into local part and domain part.
    let p = param_in.addr.find('@')?;
    let (in_emaillocalpart, in_emaildomain) = param_in.addr.split_at(p);
    let in_emaildomain = &in_emaildomain[1..];

    let mut reader = quick_xml::Reader::from_str(&xml_raw);
    reader.trim_text(true);

    let mut buf = Vec::new();

    let mut moz_ac = MozAutoconfigure {
        in_0: param_in,
        in_emaildomain,
        in_emaillocalpart,
        out: LoginParam::new(),
        out_imap_set: false,
        out_smtp_set: false,
        tag_server: MozServer::Undefined,
        tag_config: MozConfigTag::Undefined,
    };
    loop {
        match reader.read_event(&mut buf) {
            Ok(quick_xml::events::Event::Start(ref e)) => {
                moz_autoconfigure_starttag_cb(e, &mut moz_ac, &reader)
            }
            Ok(quick_xml::events::Event::End(ref e)) => moz_autoconfigure_endtag_cb(e, &mut moz_ac),
            Ok(quick_xml::events::Event::Text(ref e)) => {
                moz_autoconfigure_text_cb(e, &mut moz_ac, &reader)
            }
            Err(e) => {
                error!(
                    context,
                    "Configure xml: Error at position {}: {:?}",
                    reader.buffer_position(),
                    e
                );
            }
            Ok(quick_xml::events::Event::Eof) => break,
            _ => (),
        }
        buf.clear();
    }

    if moz_ac.out.mail_server.is_empty()
        || moz_ac.out.mail_port == 0
        || moz_ac.out.send_server.is_empty()
        || moz_ac.out.send_port == 0
    {
        let r = moz_ac.out.to_string();
        warn!(context, "Bad or incomplete autoconfig: {}", r,);
        return None;
    }

    Some(moz_ac.out)
}

fn moz_autoconfigure_text_cb<B: std::io::BufRead>(
    event: &BytesText,
    moz_ac: &mut MozAutoconfigure,
    reader: &quick_xml::Reader<B>,
) {
    let val = event.unescape_and_decode(reader).unwrap_or_default();

    let addr = &moz_ac.in_0.addr;
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
        moz_ac.tag_server = MozServer::Undefined;
        moz_ac.tag_config = MozConfigTag::Undefined;
        moz_ac.out_imap_set = true;
    } else if tag == "outgoingserver" {
        moz_ac.tag_server = MozServer::Undefined;
        moz_ac.tag_config = MozConfigTag::Undefined;
        moz_ac.out_smtp_set = true;
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
