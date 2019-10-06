use quick_xml;
use quick_xml::events::BytesEnd;

use crate::constants::*;
use crate::context::Context;
use crate::login_param::LoginParam;

use super::read_autoconf_file;

/// Outlook's Autodiscover
struct OutlookAutodiscover {
    pub out: LoginParam,
    pub out_imap_set: bool,
    pub out_smtp_set: bool,
    pub config_type: Option<String>,
    pub config_server: String,
    pub config_port: i32,
    pub config_ssl: String,
    pub config_redirecturl: Option<String>,
}

pub fn outlk_autodiscover(
    context: &Context,
    url: &str,
    _param_in: &LoginParam,
) -> Option<LoginParam> {
    let mut url = url.to_string();
    /* Follow up to 10 xml-redirects (http-redirects are followed in read_autoconf_file() */
    for _i in 0..10 {
        let mut outlk_ad = OutlookAutodiscover {
            out: LoginParam::new(),
            out_imap_set: false,
            out_smtp_set: false,
            config_type: None,
            config_server: String::new(),
            config_port: 0,
            config_ssl: String::new(),
            config_redirecturl: None,
        };

        if let Some(xml_raw) = read_autoconf_file(context, &url) {
            let mut reader = quick_xml::Reader::from_str(&xml_raw);
            reader.trim_text(true);

            let mut buf = Vec::new();

            let mut current_tag: Option<String> = None;

            loop {
                match reader.read_event(&mut buf) {
                    Ok(quick_xml::events::Event::Start(ref e)) => {
                        let tag = String::from_utf8_lossy(e.name()).trim().to_lowercase();

                        if tag == "protocol" {
                            outlk_ad.config_type = None;
                            outlk_ad.config_server = String::new();
                            outlk_ad.config_port = 0;
                            outlk_ad.config_ssl = String::new();
                            outlk_ad.config_redirecturl = None;

                            current_tag = None;
                        } else {
                            current_tag = Some(tag);
                        }
                    }
                    Ok(quick_xml::events::Event::End(ref e)) => {
                        outlk_autodiscover_endtag_cb(e, &mut outlk_ad);
                        current_tag = None;
                    }
                    Ok(quick_xml::events::Event::Text(ref e)) => {
                        let val = e.unescape_and_decode(&reader).unwrap_or_default();

                        if let Some(ref tag) = current_tag {
                            match tag.as_str() {
                                "type" => outlk_ad.config_type = Some(val.trim().to_string()),
                                "server" => outlk_ad.config_server = val.trim().to_string(),
                                "port" => {
                                    outlk_ad.config_port = val.trim().parse().unwrap_or_default()
                                }
                                "ssl" => outlk_ad.config_ssl = val.trim().to_string(),
                                "redirecturl" => {
                                    outlk_ad.config_redirecturl = Some(val.trim().to_string())
                                }
                                _ => {}
                            };
                        }
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

            // XML redirect via redirecturl
            if outlk_ad.config_redirecturl.is_none()
                || outlk_ad.config_redirecturl.as_ref().unwrap().is_empty()
            {
                if outlk_ad.out.mail_server.is_empty()
                    || outlk_ad.out.mail_port == 0
                    || outlk_ad.out.send_server.is_empty()
                    || outlk_ad.out.send_port == 0
                {
                    let r = outlk_ad.out.to_string();
                    warn!(context, "Bad or incomplete autoconfig: {}", r,);
                    return None;
                }
                return Some(outlk_ad.out);
            } else {
                url = outlk_ad.config_redirecturl.unwrap();
            }
        } else {
            return None;
        }
    }
    None
}

fn outlk_autodiscover_endtag_cb(event: &BytesEnd, outlk_ad: &mut OutlookAutodiscover) {
    let tag = String::from_utf8_lossy(event.name()).trim().to_lowercase();

    if tag == "protocol" {
        if let Some(type_) = &outlk_ad.config_type {
            let port = outlk_ad.config_port;
            let ssl_on = outlk_ad.config_ssl == "on";
            let ssl_off = outlk_ad.config_ssl == "off";
            if type_ == "imap" && !outlk_ad.out_imap_set {
                outlk_ad.out.mail_server =
                    std::mem::replace(&mut outlk_ad.config_server, String::new());
                outlk_ad.out.mail_port = port;
                if ssl_on {
                    outlk_ad.out.server_flags |= DC_LP_IMAP_SOCKET_SSL as i32
                } else if ssl_off {
                    outlk_ad.out.server_flags |= DC_LP_IMAP_SOCKET_PLAIN as i32
                }
                outlk_ad.out_imap_set = true
            } else if type_ == "smtp" && !outlk_ad.out_smtp_set {
                outlk_ad.out.send_server =
                    std::mem::replace(&mut outlk_ad.config_server, String::new());
                outlk_ad.out.send_port = outlk_ad.config_port;
                if ssl_on {
                    outlk_ad.out.server_flags |= DC_LP_SMTP_SOCKET_SSL as i32
                } else if ssl_off {
                    outlk_ad.out.server_flags |= DC_LP_SMTP_SOCKET_PLAIN as i32
                }
                outlk_ad.out_smtp_set = true
            }
        }
    }
}
