use std::collections::HashMap;

use crate::config::Config;
use crate::context::Context;
use crate::provider::Socket;
use crate::{contact, login_param::CertificateChecks};
use anyhow::{bail, Context as _, Result};
use num_traits::cast::ToPrimitive;

use super::{Qr, DCLOGIN_SCHEME};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoginOptions {
    UnsuportedVersion(u32),
    V1 {
        mail_pw: String,
        imap_host: Option<String>,
        imap_port: Option<u16>,
        imap_username: Option<String>,
        imap_password: Option<String>,
        imap_security: Option<Socket>,
        imap_certificate_checks: Option<CertificateChecks>,
        smtp_host: Option<String>,
        smtp_port: Option<u16>,
        smtp_username: Option<String>,
        smtp_password: Option<String>,
        smtp_security: Option<Socket>,
        smtp_certificate_checks: Option<CertificateChecks>,
    },
}

/// scheme: `dclogin://user@host/?p=password&v=1[&options]`
/// read more about the scheme at https://github.com/deltachat/interface/blob/master/uri-schemes.md#DCLOGIN
pub(super) fn decode_login(qr: &str) -> Result<Qr> {
    let url = url::Url::parse(qr).with_context(|| format!("Malformed url: {:?}", qr))?;

    let mut payload = qr
        .get(DCLOGIN_SCHEME.len()..)
        .context("invalid DCLOGIN payload E1")?;

    // if first 2 chars are `//` remove them
    if payload.get(0..2) == Some("//") {
        payload = payload.get(2..).context("invalid DCLOGIN payload E2")?;
        // todo: is there a more idiomatic way?
    }

    let addr = payload
        .get(
            ..payload
                .chars()
                .position(|c| c == '?' || c == '/')
                .context("invalid DCLOGIN payload E3a")?,
        )
        .context("invalid DCLOGIN payload E3b")?;

    let mut scheme = url.scheme().to_owned();
    scheme.make_ascii_lowercase();

    if scheme == "dclogin" {
        let options = url.query_pairs();
        if options.count() == 0 {
            bail!("invalid DCLOGIN payload E4")
        }
        // load options into hashmap
        let mut parameter_map = HashMap::with_capacity(options.count());
        for (key, value) in options {
            parameter_map.insert(key.into_owned(), value.into_owned());
        }

        // check if username is there
        println!("{}", addr);
        if !contact::may_be_valid_addr(addr) {
            bail!("invalid DCLOGIN payload: invalid username E5");
        }

        // apply to result struct
        let options: LoginOptions = match parameter_map.get("v").map(|i| i.parse::<u32>()) {
            Some(version_result) => match version_result {
                Ok(1) => LoginOptions::V1 {
                    mail_pw: parameter_map
                        .get("p")
                        .map(|s| s.to_owned())
                        .context("password missing")?,
                    imap_host: parameter_map.get("ih").map(|s| s.to_owned()),
                    imap_port: parse_port(parameter_map.get("ip"))
                        .context("could not parse imap port")?,
                    imap_username: parameter_map.get("iu").map(|s| s.to_owned()),
                    imap_password: parameter_map.get("ipw").map(|s| s.to_owned()),
                    imap_security: parse_socket_security(parameter_map.get("is"))?,
                    imap_certificate_checks: parse_certificate_checks(parameter_map.get("ic"))?,
                    smtp_host: parameter_map.get("sh").map(|s| s.to_owned()),
                    smtp_port: parse_port(parameter_map.get("sp"))
                        .context("could not parse smtp port")?,
                    smtp_username: parameter_map.get("su").map(|s| s.to_owned()),
                    smtp_password: parameter_map.get("spw").map(|s| s.to_owned()),
                    smtp_security: parse_socket_security(parameter_map.get("ss"))?,
                    smtp_certificate_checks: parse_certificate_checks(parameter_map.get("sc"))?,
                },
                Ok(v) => LoginOptions::UnsuportedVersion(v),
                Err(_) => bail!("version could not be parsed as number E6"),
            },
            None => bail!("invalid DCLOGIN payload: version missing E7"),
        };

        Ok(Qr::Login {
            address: addr.to_owned(),
            options,
        })
    } else {
        bail!("Bad scheme for account URL: {:?}.", payload);
    }
}

fn parse_port(port: Option<&String>) -> core::result::Result<Option<u16>, std::num::ParseIntError> {
    match port {
        Some(p) => Ok(Some(p.parse::<u16>()?)),
        None => Ok(None),
    }
}

fn parse_socket_security(security: Option<&String>) -> Result<Option<Socket>> {
    Ok(match security.map(|s| s.as_str()) {
        Some("ssl") => Some(Socket::Ssl),
        Some("starttls") => Some(Socket::Starttls),
        Some("default") => Some(Socket::Automatic),
        Some("plain") => Some(Socket::Plain),
        Some(other) => bail!("Unknown security level: {}", other),
        None => None,
    })
}

fn parse_certificate_checks(
    certificate_checks: Option<&String>,
) -> Result<Option<CertificateChecks>> {
    Ok(match certificate_checks.map(|s| s.as_str()) {
        Some("0") => Some(CertificateChecks::Automatic),
        Some("1") => Some(CertificateChecks::Strict),
        Some("3") => Some(CertificateChecks::AcceptInvalidCertificates),
        Some(other) => bail!("Unknown certificatecheck level: {}", other),
        None => None,
    })
}

pub(crate) async fn apply_from_login_qr(
    context: &Context,
    address: &str,
    options: LoginOptions,
) -> Result<()> {
    context.set_config(Config::Addr, Some(address)).await?;

    match options {
        LoginOptions::V1 {
            mail_pw,
            imap_host,
            imap_port,
            imap_username,
            imap_password,
            imap_security,
            imap_certificate_checks,
            smtp_host,
            smtp_port,
            smtp_username,
            smtp_password,
            smtp_security,
            smtp_certificate_checks,
        } => {
            context.set_config(Config::MailPw, Some(&mail_pw)).await?;
            if let Some(value) = imap_host {
                context.set_config(Config::MailServer, Some(&value)).await?;
            }
            if let Some(value) = imap_port {
                context
                    .set_config(Config::MailPort, Some(&value.to_string()))
                    .await?;
            }
            if let Some(value) = imap_username {
                context.set_config(Config::MailUser, Some(&value)).await?;
            }
            if let Some(value) = imap_password {
                context.set_config(Config::MailPw, Some(&value)).await?;
            }
            if let Some(value) = imap_security {
                let code = value
                    .to_u8()
                    .context("could not convert imap security value to number")?;
                context
                    .set_config(Config::MailSecurity, Some(&code.to_string()))
                    .await?;
            }
            if let Some(value) = imap_certificate_checks {
                let code = value
                    .to_u32()
                    .context("could not convert imap certificate checks value to number")?;
                context
                    .set_config(Config::ImapCertificateChecks, Some(&code.to_string()))
                    .await?;
            }
            if let Some(value) = smtp_host {
                context.set_config(Config::SendServer, Some(&value)).await?;
            }
            if let Some(value) = smtp_port {
                context
                    .set_config(Config::SendPort, Some(&value.to_string()))
                    .await?;
            }
            if let Some(value) = smtp_username {
                context.set_config(Config::SendUser, Some(&value)).await?;
            }
            if let Some(value) = smtp_password {
                context.set_config(Config::SendPw, Some(&value)).await?;
            }
            if let Some(value) = smtp_security {
                let code = value
                    .to_u8()
                    .context("could not convert smtp security value to number")?;
                context
                    .set_config(Config::SendSecurity, Some(&code.to_string()))
                    .await?;
            }
            if let Some(value) = smtp_certificate_checks {
                let code = value
                    .to_u32()
                    .context("could not convert smtp certificate checks value to number")?;
                context
                    .set_config(Config::SmtpCertificateChecks, Some(&code.to_string()))
                    .await?;
            }
            Ok(())
        }
        _ => bail!("failed to apply login options"),
    }
}

#[cfg(test)]
mod test {
    use super::{decode_login, LoginOptions};
    use crate::{login_param::CertificateChecks, provider::Socket, qr::Qr};
    use anyhow::{self, bail};

    macro_rules! login_options_just_pw {
        ($pw: expr) => {
            LoginOptions::V1 {
                mail_pw: $pw,
                imap_host: None,
                imap_port: None,
                imap_username: None,
                imap_password: None,
                imap_security: None,
                imap_certificate_checks: None,
                smtp_host: None,
                smtp_port: None,
                smtp_username: None,
                smtp_password: None,
                smtp_security: None,
                smtp_certificate_checks: None,
            }
        };
    }

    #[test]
    fn minimal_no_options() -> anyhow::Result<()> {
        let result = decode_login("dclogin://email@host.tld?p=123&v=1")?;
        if let Qr::Login { address, options } = result {
            assert_eq!(address, "email@host.tld".to_owned());
            assert_eq!(options, login_options_just_pw!("123".to_owned()));
        } else {
            bail!("wrong type")
        }
        let result = decode_login("dclogin://email@host.tld/?p=123456&v=1")?;
        if let Qr::Login { address, options } = result {
            assert_eq!(address, "email@host.tld".to_owned());
            assert_eq!(options, login_options_just_pw!("123456".to_owned()));
        } else {
            bail!("wrong type")
        }
        let result = decode_login("dclogin://email@host.tld/ignored/path?p=123456&v=1")?;
        if let Qr::Login { address, options } = result {
            assert_eq!(address, "email@host.tld".to_owned());
            assert_eq!(options, login_options_just_pw!("123456".to_owned()));
        } else {
            bail!("wrong type")
        }
        Ok(())
    }
    #[test]
    fn minimal_no_options_no_double_slash() -> anyhow::Result<()> {
        let result = decode_login("dclogin:email@host.tld?p=123&v=1")?;
        if let Qr::Login { address, options } = result {
            assert_eq!(address, "email@host.tld".to_owned());
            assert_eq!(options, login_options_just_pw!("123".to_owned()));
        } else {
            bail!("wrong type")
        }
        let result = decode_login("dclogin:email@host.tld/?p=123456&v=1")?;
        if let Qr::Login { address, options } = result {
            assert_eq!(address, "email@host.tld".to_owned());
            assert_eq!(options, login_options_just_pw!("123456".to_owned()));
        } else {
            bail!("wrong type")
        }
        let result = decode_login("dclogin:email@host.tld/ignored/path?p=123456&v=1")?;
        if let Qr::Login { address, options } = result {
            assert_eq!(address, "email@host.tld".to_owned());
            assert_eq!(options, login_options_just_pw!("123456".to_owned()));
        } else {
            bail!("wrong type")
        }
        Ok(())
    }

    #[test]
    fn no_version_set() {
        assert!(decode_login("dclogin:email@host.tld?p=123").is_err());
    }

    #[test]
    fn invalid_version_set() {
        assert!(decode_login("dclogin:email@host.tld?p=123&v=").is_err());
        assert!(decode_login("dclogin:email@host.tld?p=123&v=%40").is_err());
        assert!(decode_login("dclogin:email@host.tld?p=123&v=-20").is_err());
        assert!(decode_login("dclogin:email@host.tld?p=123&v=hi").is_err());
    }

    #[test]
    fn version_too_new() -> anyhow::Result<()> {
        let result = decode_login("dclogin:email@host.tld/?p=123456&v=2")?;
        if let Qr::Login { options, .. } = result {
            assert_eq!(options, LoginOptions::UnsuportedVersion(2));
        } else {
            bail!("wrong type");
        }
        let result = decode_login("dclogin:email@host.tld/?p=123456&v=5")?;
        if let Qr::Login { options, .. } = result {
            assert_eq!(options, LoginOptions::UnsuportedVersion(5));
        } else {
            bail!("wrong type");
        }
        Ok(())
    }

    #[test]
    fn all_advanced_options() -> anyhow::Result<()> {
        let result = decode_login(
            "dclogin:email@host.tld?p=secret&v=1&ih=imap.host.tld&ip=4000&iu=max&ipw=87654&is=ssl&ic=1&sh=mail.host.tld&sp=3000&su=max@host.tld&spw=3242HS&ss=plain&sc=3",
        )?;
        if let Qr::Login { address, options } = result {
            assert_eq!(address, "email@host.tld".to_owned());
            assert_eq!(
                options,
                LoginOptions::V1 {
                    mail_pw: "secret".to_owned(),
                    imap_host: Some("imap.host.tld".to_owned()),
                    imap_port: Some(4000),
                    imap_username: Some("max".to_owned()),
                    imap_password: Some("87654".to_owned()),
                    imap_security: Some(Socket::Ssl),
                    imap_certificate_checks: Some(CertificateChecks::Strict),
                    smtp_host: Some("mail.host.tld".to_owned()),
                    smtp_port: Some(3000),
                    smtp_username: Some("max@host.tld".to_owned()),
                    smtp_password: Some("3242HS".to_owned()),
                    smtp_security: Some(Socket::Plain),
                    smtp_certificate_checks: Some(CertificateChecks::AcceptInvalidCertificates),
                }
            );
        } else {
            bail!("wrong type")
        }
        Ok(())
    }

    #[test]
    fn uri_encoded_password() -> anyhow::Result<()> {
        let result = decode_login(
            "dclogin:email@host.tld?p=%7BDaehFl%3B%22as%40%21fhdodn5%24234%22%7B%7Dfg&v=1",
        )?;
        if let Qr::Login { address, options } = result {
            assert_eq!(address, "email@host.tld".to_owned());
            assert_eq!(
                options,
                login_options_just_pw!("{DaehFl;\"as@!fhdodn5$234\"{}fg".to_owned())
            );
        } else {
            bail!("wrong type")
        }
        Ok(())
    }

    #[test]
    fn email_with_plus_extension() -> anyhow::Result<()> {
        let result = decode_login("dclogin:usename+extension@host?p=1234&v=1")?;
        if let Qr::Login { address, options } = result {
            assert_eq!(address, "usename+extension@host".to_owned());
            assert_eq!(options, login_options_just_pw!("1234".to_owned()));
        } else {
            bail!("wrong type")
        }
        Ok(())
    }

    // idea: should invalid uri encoding result in error?
}
