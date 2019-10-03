use libc::free;
use quick_xml;
use quick_xml::events::{BytesEnd, BytesStart, BytesText};

use crate::constants::*;
use crate::context::Context;
use crate::dc_tools::*;
use crate::login_param::LoginParam;

use super::read_autoconf_file;
/* ******************************************************************************
 * Thunderbird's Autoconfigure
 ******************************************************************************/
/* documentation: https://developer.mozilla.org/en-US/docs/Mozilla/Thunderbird/Autoconfiguration */
#[repr(C)]
struct moz_autoconfigure_t<'a> {
    pub in_0: &'a LoginParam,
    pub in_emaildomain: &'a str,
    pub in_emaillocalpart: &'a str,
    pub out: LoginParam,
    pub out_imap_set: libc::c_int,
    pub out_smtp_set: libc::c_int,
    pub tag_server: libc::c_int,
    pub tag_config: libc::c_int,
}

pub unsafe fn moz_autoconfigure(
    context: &Context,
    url: &str,
    param_in: &LoginParam,
) -> Option<LoginParam> {
    let xml_raw = read_autoconf_file(context, url);
    if xml_raw.is_null() {
        return None;
    }

    // Split address into local part and domain part.
    let p = param_in.addr.find("@");
    if p.is_none() {
        free(xml_raw as *mut libc::c_void);
        return None;
    }
    let (in_emaillocalpart, in_emaildomain) = param_in.addr.split_at(p.unwrap_or_default());
    let in_emaildomain = &in_emaildomain[1..];

    let mut reader = quick_xml::Reader::from_str(as_str(xml_raw));
    reader.trim_text(true);

    let mut buf = Vec::new();

    let mut moz_ac = moz_autoconfigure_t {
        in_0: param_in,
        in_emaildomain,
        in_emaillocalpart,
        out: LoginParam::new(),
        out_imap_set: 0,
        out_smtp_set: 0,
        tag_server: 0,
        tag_config: 0,
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
        free(xml_raw as *mut libc::c_void);
        return None;
    }

    free(xml_raw as *mut libc::c_void);
    Some(moz_ac.out)
}

fn moz_autoconfigure_text_cb<B: std::io::BufRead>(
    event: &BytesText,
    moz_ac: &mut moz_autoconfigure_t,
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

    if moz_ac.tag_server == 1 {
        match moz_ac.tag_config {
            10 => moz_ac.out.mail_server = val,
            11 => moz_ac.out.mail_port = val.parse().unwrap_or_default(),
            12 => moz_ac.out.mail_user = val,
            13 => {
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
        }
    } else if moz_ac.tag_server == 2 {
        match moz_ac.tag_config {
            10 => moz_ac.out.send_server = val,
            11 => moz_ac.out.send_port = val.parse().unwrap_or_default(),
            12 => moz_ac.out.send_user = val,
            13 => {
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
        }
    }
}

fn moz_autoconfigure_endtag_cb(event: &BytesEnd, moz_ac: &mut moz_autoconfigure_t) {
    let tag = String::from_utf8_lossy(event.name()).trim().to_lowercase();

    if tag == "incomingserver" {
        moz_ac.tag_server = 0;
        moz_ac.tag_config = 0;
        moz_ac.out_imap_set = 1;
    } else if tag == "outgoingserver" {
        moz_ac.tag_server = 0;
        moz_ac.tag_config = 0;
        moz_ac.out_smtp_set = 1;
    } else {
        moz_ac.tag_config = 0;
    }
}

fn moz_autoconfigure_starttag_cb<B: std::io::BufRead>(
    event: &BytesStart,
    moz_ac: &mut moz_autoconfigure_t,
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

            if typ == "imap" && moz_ac.out_imap_set == 0 {
                1
            } else {
                0
            }
        } else {
            0
        };
        moz_ac.tag_config = 0;
    } else if tag == "outgoingserver" {
        moz_ac.tag_server = if moz_ac.out_smtp_set == 0 { 2 } else { 0 };
        moz_ac.tag_config = 0;
    } else if tag == "hostname" {
        moz_ac.tag_config = 10;
    } else if tag == "port" {
        moz_ac.tag_config = 11;
    } else if tag == "sockettype" {
        moz_ac.tag_config = 13;
    } else if tag == "username" {
        moz_ac.tag_config = 12;
    }
}
