//! Outlook's Autodiscover

use quick_xml::events::BytesEnd;

use crate::context::Context;
use crate::login_param::{LoginParam, ServerSecurity, Service};

use super::read_url::read_url;
use super::Error;

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

enum ParsingResult {
    LoginParam(LoginParam),
    RedirectUrl(String),
}

fn parse_xml(xml_raw: &str) -> Result<ParsingResult, Error> {
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

    let mut reader = quick_xml::Reader::from_str(&xml_raw);
    reader.trim_text(true);

    let mut buf = Vec::new();

    let mut current_tag: Option<String> = None;

    loop {
        let event = reader
            .read_event(&mut buf)
            .map_err(|error| Error::InvalidXml {
                position: reader.buffer_position(),
                error,
            })?;

        match event {
            quick_xml::events::Event::Start(ref e) => {
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
            quick_xml::events::Event::End(ref e) => {
                outlk_autodiscover_endtag_cb(e, &mut outlk_ad);
                current_tag = None;
            }
            quick_xml::events::Event::Text(ref e) => {
                let val = e.unescape_and_decode(&reader).unwrap_or_default();

                if let Some(ref tag) = current_tag {
                    match tag.as_str() {
                        "type" => {
                            outlk_ad.config_type = Some(val.trim().to_lowercase().to_string())
                        }
                        "server" => outlk_ad.config_server = val.trim().to_string(),
                        "port" => outlk_ad.config_port = val.trim().parse().unwrap_or_default(),
                        "ssl" => outlk_ad.config_ssl = val.trim().to_string(),
                        "redirecturl" => outlk_ad.config_redirecturl = Some(val.trim().to_string()),
                        _ => {}
                    };
                }
            }
            quick_xml::events::Event::Eof => break,
            _ => (),
        }
        buf.clear();
    }

    // XML redirect via redirecturl
    let res = if outlk_ad.config_redirecturl.is_none()
        || outlk_ad.config_redirecturl.as_ref().unwrap().is_empty()
    {
        if outlk_ad.out.srv_params[Service::Imap as usize]
            .hostname
            .is_empty()
            || outlk_ad.out.srv_params[Service::Imap as usize].port == 0
            || outlk_ad.out.srv_params[Service::Smtp as usize]
                .hostname
                .is_empty()
            || outlk_ad.out.srv_params[Service::Smtp as usize].port == 0
        {
            return Err(Error::IncompleteAutoconfig(outlk_ad.out));
        }
        ParsingResult::LoginParam(outlk_ad.out)
    } else {
        ParsingResult::RedirectUrl(outlk_ad.config_redirecturl.unwrap())
    };
    Ok(res)
}

pub fn outlk_autodiscover(
    context: &Context,
    url: &str,
    _param_in: &LoginParam,
) -> Result<LoginParam, Error> {
    let mut url = url.to_string();
    /* Follow up to 10 xml-redirects (http-redirects are followed in read_url() */
    for _i in 0..10 {
        let xml_raw = read_url(context, &url)?;
        let res = parse_xml(&xml_raw);
        if let Err(err) = &res {
            warn!(context, "{}", err);
        }
        match res? {
            ParsingResult::RedirectUrl(redirect_url) => url = redirect_url,
            ParsingResult::LoginParam(login_param) => return Ok(login_param),
        }
    }
    Err(Error::RedirectionError)
}

fn outlk_autodiscover_endtag_cb(event: &BytesEnd, outlk_ad: &mut OutlookAutodiscover) {
    let tag = String::from_utf8_lossy(event.name()).trim().to_lowercase();

    if tag == "protocol" {
        if let Some(type_) = &outlk_ad.config_type {
            let port = outlk_ad.config_port;
            let ssl_on = outlk_ad.config_ssl == "on";
            let ssl_off = outlk_ad.config_ssl == "off";
            if type_ == "imap" && !outlk_ad.out_imap_set {
                outlk_ad.out.srv_params[Service::Imap as usize].hostname =
                    std::mem::replace(&mut outlk_ad.config_server, String::new());
                outlk_ad.out.srv_params[Service::Imap as usize].port = port;
                if ssl_on {
                    outlk_ad.out.srv_params[Service::Imap as usize].security =
                        Some(ServerSecurity::Ssl);
                } else if ssl_off {
                    outlk_ad.out.srv_params[Service::Imap as usize].security =
                        Some(ServerSecurity::PlainSocket);
                }
                outlk_ad.out_imap_set = true
            } else if type_ == "smtp" && !outlk_ad.out_smtp_set {
                outlk_ad.out.srv_params[Service::Smtp as usize].hostname =
                    std::mem::replace(&mut outlk_ad.config_server, String::new());
                outlk_ad.out.srv_params[Service::Smtp as usize].port = outlk_ad.config_port;
                if ssl_on {
                    outlk_ad.out.srv_params[Service::Smtp as usize].security =
                        Some(ServerSecurity::Ssl);
                } else if ssl_off {
                    outlk_ad.out.srv_params[Service::Smtp as usize].security =
                        Some(ServerSecurity::PlainSocket);
                }
                outlk_ad.out_smtp_set = true
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_redirect() {
        let res = parse_xml("
<?xml version=\"1.0\" encoding=\"utf-8\"?>
  <Autodiscover xmlns=\"http://schemas.microsoft.com/exchange/autodiscover/responseschema/2006\">
    <Response xmlns=\"http://schemas.microsoft.com/exchange/autodiscover/outlook/responseschema/2006a\">
      <Account>
        <AccountType>email</AccountType>
        <Action>redirectUrl</Action>
        <RedirectUrl>https://mail.example.com/autodiscover/autodiscover.xml</RedirectUrl>
      </Account>
    </Response>
  </Autodiscover>
 ").expect("XML is not parsed successfully");
        match res {
            ParsingResult::LoginParam(_lp) => {
                panic!("redirecturl is not found");
            }
            ParsingResult::RedirectUrl(url) => {
                assert_eq!(
                    url,
                    "https://mail.example.com/autodiscover/autodiscover.xml"
                );
            }
        }
    }

    #[test]
    fn test_parse_loginparam() {
        let res = parse_xml(
            "\
<?xml version=\"1.0\" encoding=\"utf-8\"?>
<Autodiscover xmlns=\"http://schemas.microsoft.com/exchange/autodiscover/responseschema/2006\">
  <Response xmlns=\"http://schemas.microsoft.com/exchange/autodiscover/outlook/responseschema/2006a\">
    <Account>
      <AccountType>email</AccountType>
      <Action>settings</Action>
      <Protocol>
        <Type>IMAP</Type>
        <Server>example.com</Server>
        <Port>993</Port>
        <SSL>on</SSL>
        <AuthRequired>on</AuthRequired>
      </Protocol>
      <Protocol>
        <Type>SMTP</Type>
        <Server>smtp.example.com</Server>
        <Port>25</Port>
        <SSL>off</SSL>
        <AuthRequired>on</AuthRequired>
      </Protocol>
    </Account>
  </Response>
</Autodiscover>",
        )
        .expect("XML is not parsed successfully");

        match res {
            ParsingResult::LoginParam(lp) => {
                assert_eq!(
                    lp.srv_params[Service::Imap as usize].hostname,
                    "example.com"
                );
                assert_eq!(lp.srv_params[Service::Imap as usize].port, 993);
                assert_eq!(
                    lp.srv_params[Service::Smtp as usize].hostname,
                    "smtp.example.com"
                );
                assert_eq!(lp.srv_params[Service::Smtp as usize].port, 25);
            }
            ParsingResult::RedirectUrl(_) => {
                panic!("RedirectUrl is not expected");
            }
        }
    }
}
