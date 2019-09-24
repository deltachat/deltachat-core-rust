use std::collections::HashMap;
use quick_xml;

use crate::constants::*;
use crate::context::Context;
use crate::login_param::LoginParam;
use quick_xml::events::{BytesEnd};

use super::read_autoconf_file;
/* ******************************************************************************
 * Outlook's Autodiscover
 ******************************************************************************/
#[repr(C)]
struct outlk_autodiscover_t<'a> {
    pub in_0: &'a LoginParam,
    pub out: LoginParam,
    pub out_imap_set: bool,
    pub out_smtp_set: bool,
    pub config: HashMap<String, String>,
}

pub fn outlk_autodiscover(
    context: &Context,
    url__: &str,
    param_in: &LoginParam,
) -> Option<LoginParam> {
    let mut url = url__.to_string();
    let mut outlk_ad = outlk_autodiscover_t {
        in_0: param_in,
        out: LoginParam::new(),
        out_imap_set: false,
        out_smtp_set: false,
        config: HashMap::new(),
    };
    for i in 0..10 {
        let xml_raw = read_autoconf_file(context, &url);
        if xml_raw.is_err() {
            return Some(outlk_ad.out);
        }
        let xml_raw = xml_raw.unwrap();

        let mut reader = quick_xml::Reader::from_str(&xml_raw);
        reader.trim_text(true);

        let mut buf = Vec::new();
        let current_tag: Option<String> = None;
        loop {
            match reader.read_event(&mut buf) {
                Ok(quick_xml::events::Event::Start(ref e)) => {
                    current_tag = Some(String::from_utf8_lossy(e.name()).trim().to_lowercase());
                }
                Ok(quick_xml::events::Event::End(ref e)) => {
                    if "protocol" == String::from_utf8_lossy(e.name()).trim().to_lowercase() {
                        finish_settings(e, &mut outlk_ad);
                    }
                    current_tag = None;
                }
                Ok(quick_xml::events::Event::Text(ref e)) => {
                    if let Some(current_tag) = current_tag {
                        let val = e.unescape_and_decode(&reader).unwrap_or_default();
                        &outlk_ad.config.insert(current_tag, val);
                    }
                }
                Err(e) => {
                    error!(
                        context,
                        "Configure xml: Error at position {}: {:?}",
                        reader.buffer_position(),
                        e
                    );
                    break;
                }
                Ok(quick_xml::events::Event::Eof) => break,
                _ => (),
            }
            buf.clear();
        }
        if let Some(next_url) = outlk_ad.config.get("redirecturl") {
            if !next_url.is_empty() {
                url = next_url.to_string();
                continue;
            }
        }
        break;
    }

    if outlk_ad.out.mail_server.is_empty()
        || outlk_ad.out.mail_port == 0
        || outlk_ad.out.send_server.is_empty()
        || outlk_ad.out.send_port == 0
    {
        let r = outlk_ad.out.to_string();
        warn!(context, "Bad or incomplete autoconfig: {}", r,);

        return None;
    }
    Some(outlk_ad.out)
}

fn finish_settings(event: &BytesEnd, outlk_ad: &mut outlk_autodiscover_t) {
    let ssl_on = false;
    let ssl_off = false;
    let config = &outlk_ad.config;
    if let Some(type_val) = &config.get("type") {
        let port = match config.get("port") {
            None => 0,
            Some(r) => {
                r.parse::<i32>().unwrap_or_default()
            }
        };
        if let Some(ssl) = &config.get("ssl") {
            ssl_on = *ssl == "on";
            ssl_off = *ssl == "off";
        }
        let type_val = *type_val;
        if !outlk_ad.out_imap_set && type_val == "imap" {
            outlk_ad.out.mail_server = config.get("server"); //.unwrap_or_default();
            outlk_ad.out.mail_port = port;
            if ssl_on {
                outlk_ad.out.server_flags |= DC_LP_IMAP_SOCKET_SSL as i32;
            } else if ssl_off {
                outlk_ad.out.server_flags |= DC_LP_IMAP_SOCKET_PLAIN as i32;
            }
            outlk_ad.out_imap_set = true;
        } else if !outlk_ad.out_smtp_set && type_val == "smtp" {
            outlk_ad.out.send_server = &config.get("server").unwrap_or_default();
            outlk_ad.out.send_port = port;

            if ssl_on {
                outlk_ad.out.server_flags |= DC_LP_SMTP_SOCKET_SSL as i32
            } else if ssl_off {
                outlk_ad.out.server_flags |= DC_LP_SMTP_SOCKET_PLAIN as i32
            }
            outlk_ad.out_smtp_set = true;
        }
    }
}
