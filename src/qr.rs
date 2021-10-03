//! # QR code module.

use anyhow::{bail, ensure, format_err, Context as _, Error, Result};
use once_cell::sync::Lazy;
use percent_encoding::percent_decode_str;
use serde::Deserialize;
use std::collections::BTreeMap;

use crate::chat::{self, get_chat_id_by_grpid, ChatIdBlocked};
use crate::config::Config;
use crate::constants::Blocked;
use crate::contact::{addr_normalize, may_be_valid_addr, Contact, Origin};
use crate::context::Context;
use crate::dc_tools::time;
use crate::key::Fingerprint;
use crate::message::Message;
use crate::peerstate::Peerstate;
use crate::token;

const OPENPGP4FPR_SCHEME: &str = "OPENPGP4FPR:"; // yes: uppercase
const DCACCOUNT_SCHEME: &str = "DCACCOUNT:";
const DCWEBRTC_SCHEME: &str = "DCWEBRTC:";
const MAILTO_SCHEME: &str = "mailto:";
const MATMSG_SCHEME: &str = "MATMSG:";
const VCARD_SCHEME: &str = "BEGIN:VCARD";
const SMTP_SCHEME: &str = "SMTP:";
const HTTP_SCHEME: &str = "http://";
const HTTPS_SCHEME: &str = "https://";

#[derive(Debug, Clone, PartialEq)]
pub enum Qr {
    AskVerifyContact {
        contact_id: u32,
        fingerprint: Fingerprint,
        invitenumber: String,
        authcode: String,
    },
    AskVerifyGroup {
        grpname: String,
        grpid: String,
        contact_id: u32,
        fingerprint: Fingerprint,
        invitenumber: String,
        authcode: String,
    },
    FprOk {
        contact_id: u32,
    },
    FprMismatch {
        contact_id: Option<u32>,
    },
    FprWithoutAddr {
        fingerprint: String,
    },
    Account {
        domain: String,
    },
    WebrtcInstance {
        domain: String,
        instance_pattern: String,
    },
    Addr {
        contact_id: u32,
    },
    Url {
        url: String,
    },
    Text {
        text: String,
    },
    WithdrawVerifyContact {
        contact_id: u32,
        fingerprint: Fingerprint,
        invitenumber: String,
        authcode: String,
    },
    WithdrawVerifyGroup {
        grpname: String,
        grpid: String,
        contact_id: u32,
        fingerprint: Fingerprint,
        invitenumber: String,
        authcode: String,
    },
    ReviveVerifyContact {
        contact_id: u32,
        fingerprint: Fingerprint,
        invitenumber: String,
        authcode: String,
    },
    ReviveVerifyGroup {
        grpname: String,
        grpid: String,
        contact_id: u32,
        fingerprint: Fingerprint,
        invitenumber: String,
        authcode: String,
    },
}

fn starts_with_ignore_case(string: &str, pattern: &str) -> bool {
    string.to_lowercase().starts_with(&pattern.to_lowercase())
}

/// Check a scanned QR code.
/// The function should be called after a QR code is scanned.
/// The function takes the raw text scanned and checks what can be done with it.
pub async fn check_qr(context: &Context, qr: &str) -> Result<Qr> {
    info!(context, "Scanned QR code: {}", qr);

    let qrcode = if starts_with_ignore_case(qr, OPENPGP4FPR_SCHEME) {
        decode_openpgp(context, qr)
            .await
            .context("failed to decode OPENPGP4FPR QR code")?
    } else if starts_with_ignore_case(qr, DCACCOUNT_SCHEME) {
        decode_account(qr)?
    } else if starts_with_ignore_case(qr, DCWEBRTC_SCHEME) {
        decode_webrtc_instance(context, qr)?
    } else if qr.starts_with(MAILTO_SCHEME) {
        decode_mailto(context, qr).await?
    } else if qr.starts_with(SMTP_SCHEME) {
        decode_smtp(context, qr).await?
    } else if qr.starts_with(MATMSG_SCHEME) {
        decode_matmsg(context, qr).await?
    } else if qr.starts_with(VCARD_SCHEME) {
        decode_vcard(context, qr).await?
    } else if qr.starts_with(HTTP_SCHEME) || qr.starts_with(HTTPS_SCHEME) {
        Qr::Url {
            url: qr.to_string(),
        }
    } else {
        Qr::Text {
            text: qr.to_string(),
        }
    };
    Ok(qrcode)
}

/// scheme: `OPENPGP4FPR:FINGERPRINT#a=ADDR&n=NAME&i=INVITENUMBER&s=AUTH`
///     or: `OPENPGP4FPR:FINGERPRINT#a=ADDR&g=GROUPNAME&x=GROUPID&i=INVITENUMBER&s=AUTH`
///     or: `OPENPGP4FPR:FINGERPRINT#a=ADDR`
#[allow(clippy::indexing_slicing)]
async fn decode_openpgp(context: &Context, qr: &str) -> Result<Qr> {
    let payload = &qr[OPENPGP4FPR_SCHEME.len()..];

    let (fingerprint, fragment) = match payload.find('#').map(|offset| {
        let (fp, rest) = payload.split_at(offset);
        // need to remove the # from the fragment
        (fp, &rest[1..])
    }) {
        Some(pair) => pair,
        None => (payload, ""),
    };
    let fingerprint: Fingerprint = fingerprint
        .parse()
        .context("Failed to parse fingerprint in the QR code")?;

    let param: BTreeMap<&str, &str> = fragment
        .split('&')
        .filter_map(|s| {
            if let [key, value] = s.splitn(2, '=').collect::<Vec<_>>()[..] {
                Some((key, value))
            } else {
                None
            }
        })
        .collect();

    let addr = if let Some(addr) = param.get("a") {
        Some(normalize_address(addr)?)
    } else {
        None
    };

    let name = if let Some(encoded_name) = param.get("n") {
        let encoded_name = encoded_name.replace("+", "%20"); // sometimes spaces are encoded as `+`
        match percent_decode_str(&encoded_name).decode_utf8() {
            Ok(name) => name.to_string(),
            Err(err) => bail!("Invalid name: {}", err),
        }
    } else {
        "".to_string()
    };

    let invitenumber = param.get("i").map(|s| s.to_string());
    let authcode = param.get("s").map(|s| s.to_string());
    let grpid = param.get("x").map(|s| s.to_string());

    let grpname = if grpid.is_some() {
        if let Some(encoded_name) = param.get("g") {
            let encoded_name = encoded_name.replace("+", "%20"); // sometimes spaces are encoded as `+`
            match percent_decode_str(&encoded_name).decode_utf8() {
                Ok(name) => Some(name.to_string()),
                Err(err) => bail!("Invalid group name: {}", err),
            }
        } else {
            None
        }
    } else {
        None
    };

    // retrieve known state for this fingerprint
    let peerstate = Peerstate::from_fingerprint(context, &context.sql, &fingerprint)
        .await
        .context("Can't load peerstate")?;

    if let (Some(addr), Some(invitenumber), Some(authcode)) = (&addr, invitenumber, authcode) {
        let contact_id = Contact::add_or_lookup(context, &name, addr, Origin::UnhandledQrScan)
            .await
            .map(|(id, _)| id)
            .with_context(|| format!("failed to add or lookup contact for address {:?}", addr))?;

        if let (Some(grpid), Some(grpname)) = (grpid, grpname) {
            if context
                .is_self_addr(addr)
                .await
                .with_context(|| format!("can't check if address {:?} is our address", addr))?
            {
                if token::exists(context, token::Namespace::InviteNumber, &*invitenumber).await {
                    Ok(Qr::WithdrawVerifyGroup {
                        grpname,
                        grpid,
                        contact_id,
                        fingerprint,
                        invitenumber,
                        authcode,
                    })
                } else {
                    Ok(Qr::ReviveVerifyGroup {
                        grpname,
                        grpid,
                        contact_id,
                        fingerprint,
                        invitenumber,
                        authcode,
                    })
                }
            } else {
                Ok(Qr::AskVerifyGroup {
                    grpname,
                    grpid,
                    contact_id,
                    fingerprint,
                    invitenumber,
                    authcode,
                })
            }
        } else if context.is_self_addr(addr).await? {
            if token::exists(context, token::Namespace::InviteNumber, &*invitenumber).await {
                Ok(Qr::WithdrawVerifyContact {
                    contact_id,
                    fingerprint,
                    invitenumber,
                    authcode,
                })
            } else {
                Ok(Qr::ReviveVerifyContact {
                    contact_id,
                    fingerprint,
                    invitenumber,
                    authcode,
                })
            }
        } else {
            Ok(Qr::AskVerifyContact {
                contact_id,
                fingerprint,
                invitenumber,
                authcode,
            })
        }
    } else if let Some(addr) = addr {
        if let Some(peerstate) = peerstate {
            let contact_id =
                Contact::add_or_lookup(context, &name, &peerstate.addr, Origin::UnhandledQrScan)
                    .await
                    .map(|(id, _)| id)?;
            let chat = ChatIdBlocked::get_for_contact(context, contact_id, Blocked::Request)
                .await
                .context("Failed to create (new) chat for contact")?;
            chat::add_info_msg(
                context,
                chat.id,
                format!("{} verified.", peerstate.addr),
                time(),
            )
            .await?;
            Ok(Qr::FprOk { contact_id })
        } else {
            let contact_id = Contact::lookup_id_by_addr(context, &addr, Origin::Unknown)
                .await
                .with_context(|| format!("Error looking up contact {:?}", addr))?;
            Ok(Qr::FprMismatch { contact_id })
        }
    } else {
        Ok(Qr::FprWithoutAddr {
            fingerprint: fingerprint.to_string(),
        })
    }
}

/// scheme: `DCACCOUNT:https://example.org/new_email?t=1w_7wDjgjelxeX884x96v3`
fn decode_account(qr: &str) -> Result<Qr> {
    let payload = qr
        .get(DCACCOUNT_SCHEME.len()..)
        .ok_or_else(|| format_err!("Invalid DCACCOUNT payload"))?;
    let url =
        url::Url::parse(payload).with_context(|| format!("Invalid account URL: {:?}", payload))?;
    if url.scheme() == "http" || url.scheme() == "https" {
        Ok(Qr::Account {
            domain: url
                .host_str()
                .ok_or_else(|| format_err!("Can't extract WebRTC instance domain"))?
                .to_string(),
        })
    } else {
        bail!("Bad scheme for account URL: {:?}.", payload);
    }
}

/// scheme: `DCWEBRTC:https://meet.jit.si/$ROOM`
fn decode_webrtc_instance(_context: &Context, qr: &str) -> Result<Qr> {
    let payload = qr
        .get(DCWEBRTC_SCHEME.len()..)
        .ok_or_else(|| format_err!("Invalid DCWEBRTC payload"))?;

    let (_type, url) = Message::parse_webrtc_instance(payload);
    let url =
        url::Url::parse(&url).with_context(|| format!("Invalid WebRTC instance: {:?}", payload))?;

    if url.scheme() == "http" || url.scheme() == "https" {
        Ok(Qr::WebrtcInstance {
            domain: url
                .host_str()
                .ok_or_else(|| format_err!("Can't extract WebRTC instance domain"))?
                .to_string(),
            instance_pattern: payload.to_string(),
        })
    } else {
        bail!("Bad URL scheme for WebRTC instance: {:?}", payload);
    }
}

#[derive(Debug, Deserialize)]
struct CreateAccountResponse {
    email: String,
    password: String,
}

/// take a qr of the type DC_QR_ACCOUNT, parse it's parameters,
/// download additional information from the contained url and set the parameters.
/// on success, a configure::configure() should be able to log in to the account
#[allow(clippy::indexing_slicing)]
async fn set_account_from_qr(context: &Context, qr: &str) -> Result<()> {
    let url_str = &qr[DCACCOUNT_SCHEME.len()..];

    let parsed: CreateAccountResponse = surf::post(url_str).recv_json().await.map_err(|err| {
        format_err!(
            "Cannot create account, request to {:?} failed: {}",
            url_str,
            err
        )
    })?;

    context
        .set_config(Config::Addr, Some(&parsed.email))
        .await?;
    context
        .set_config(Config::MailPw, Some(&parsed.password))
        .await?;

    Ok(())
}

pub async fn set_config_from_qr(context: &Context, qr: &str) -> Result<()> {
    match check_qr(context, qr).await? {
        Qr::Account { .. } => set_account_from_qr(context, qr).await?,
        Qr::WebrtcInstance {
            domain: _,
            instance_pattern,
        } => {
            context
                .set_config(Config::WebrtcInstance, Some(&instance_pattern))
                .await?;
        }
        Qr::WithdrawVerifyContact {
            invitenumber,
            authcode,
            ..
        } => {
            token::delete(context, token::Namespace::InviteNumber, &invitenumber).await?;
            token::delete(context, token::Namespace::Auth, &authcode).await?;
        }
        Qr::WithdrawVerifyGroup {
            invitenumber,
            authcode,
            ..
        } => {
            token::delete(context, token::Namespace::InviteNumber, &invitenumber).await?;
            token::delete(context, token::Namespace::Auth, &authcode).await?;
        }
        Qr::ReviveVerifyContact {
            invitenumber,
            authcode,
            ..
        } => {
            token::save(context, token::Namespace::InviteNumber, None, &invitenumber).await?;
            token::save(context, token::Namespace::Auth, None, &authcode).await?;
        }
        Qr::ReviveVerifyGroup {
            invitenumber,
            authcode,
            grpid,
            ..
        } => {
            let chat_id = get_chat_id_by_grpid(context, grpid)
                .await?
                .map(|(chat_id, _protected, _blocked)| chat_id);
            token::save(
                context,
                token::Namespace::InviteNumber,
                chat_id,
                &invitenumber,
            )
            .await?;
            token::save(context, token::Namespace::Auth, chat_id, &authcode).await?;
        }
        _ => bail!("qr code {:?} does not contain config", qr),
    }

    Ok(())
}

/// Extract address for the mailto scheme.
///
/// Scheme: `mailto:addr...?subject=...&body=..`
#[allow(clippy::indexing_slicing)]
async fn decode_mailto(context: &Context, qr: &str) -> Result<Qr> {
    let payload = &qr[MAILTO_SCHEME.len()..];

    let addr = if let Some(query_index) = payload.find('?') {
        &payload[..query_index]
    } else {
        payload
    };

    let addr = normalize_address(addr)?;
    let name = "".to_string();
    Qr::from_address(context, name, addr).await
}

/// Extract address for the smtp scheme.
///
/// Scheme: `SMTP:addr...:subject...:body...`
#[allow(clippy::indexing_slicing)]
async fn decode_smtp(context: &Context, qr: &str) -> Result<Qr> {
    let payload = &qr[SMTP_SCHEME.len()..];

    let addr = if let Some(query_index) = payload.find(':') {
        &payload[..query_index]
    } else {
        bail!("Invalid SMTP found");
    };

    let addr = normalize_address(addr)?;
    let name = "".to_string();
    Qr::from_address(context, name, addr).await
}

/// Extract address for the matmsg scheme.
///
/// Scheme: `MATMSG:TO:addr...;SUB:subject...;BODY:body...;`
///
/// There may or may not be linebreaks after the fields.
#[allow(clippy::indexing_slicing)]
async fn decode_matmsg(context: &Context, qr: &str) -> Result<Qr> {
    // Does not work when the text `TO:` is used in subject/body _and_ TO: is not the first field.
    // we ignore this case.
    let addr = if let Some(to_index) = qr.find("TO:") {
        let addr = qr[to_index + 3..].trim();
        if let Some(semi_index) = addr.find(';') {
            addr[..semi_index].trim()
        } else {
            addr
        }
    } else {
        bail!("Invalid MATMSG found");
    };

    let addr = normalize_address(addr)?;
    let name = "".to_string();
    Qr::from_address(context, name, addr).await
}

static VCARD_NAME_RE: Lazy<regex::Regex> =
    Lazy::new(|| regex::Regex::new(r"(?m)^N:([^;]*);([^;\n]*)").unwrap());
static VCARD_EMAIL_RE: Lazy<regex::Regex> =
    Lazy::new(|| regex::Regex::new(r"(?m)^EMAIL([^:\n]*):([^;\n]*)").unwrap());

/// Extract address for the vcard scheme.
///
/// Scheme: `VCARD:BEGIN\nN:last name;first name;...;\nEMAIL;<type>:addr...;`
#[allow(clippy::indexing_slicing)]
async fn decode_vcard(context: &Context, qr: &str) -> Result<Qr> {
    let name = VCARD_NAME_RE
        .captures(qr)
        .and_then(|caps| {
            let last_name = caps.get(1)?.as_str().trim();
            let first_name = caps.get(2)?.as_str().trim();

            Some(format!("{} {}", first_name, last_name))
        })
        .unwrap_or_default();

    let addr = if let Some(caps) = VCARD_EMAIL_RE.captures(qr) {
        normalize_address(caps[2].trim())?
    } else {
        bail!("Bad e-mail address");
    };

    Qr::from_address(context, name, addr).await
}

impl Qr {
    pub async fn from_address(context: &Context, name: String, addr: String) -> Result<Self> {
        let (contact_id, _) =
            Contact::add_or_lookup(context, &name, &addr, Origin::UnhandledQrScan).await?;
        Ok(Qr::Addr { contact_id })
    }
}

/// URL decodes a given address, does basic email validation on the result.
fn normalize_address(addr: &str) -> Result<String, Error> {
    // urldecoding is needed at least for OPENPGP4FPR but should not hurt in the other cases
    let new_addr = percent_decode_str(addr).decode_utf8()?;
    let new_addr = addr_normalize(&new_addr);

    ensure!(may_be_valid_addr(new_addr), "Bad e-mail address");

    Ok(new_addr.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::aheader::EncryptPreference;
    use crate::chat::{create_group_chat, ProtectionStatus};
    use crate::key::DcKey;
    use crate::peerstate::ToSave;
    use crate::securejoin::dc_get_securejoin_qr;
    use crate::test_utils::{alice_keypair, TestContext};
    use anyhow::Result;

    #[async_std::test]
    async fn test_decode_http() -> Result<()> {
        let ctx = TestContext::new().await;

        let qr = check_qr(&ctx.ctx, "http://www.hello.com").await?;
        assert_eq!(
            qr,
            Qr::Url {
                url: "http://www.hello.com".to_string()
            }
        );

        Ok(())
    }

    #[async_std::test]
    async fn test_decode_https() -> Result<()> {
        let ctx = TestContext::new().await;

        let qr = check_qr(&ctx.ctx, "https://www.hello.com").await?;
        assert_eq!(
            qr,
            Qr::Url {
                url: "https://www.hello.com".to_string()
            }
        );

        Ok(())
    }

    #[async_std::test]
    async fn test_decode_text() -> Result<()> {
        let ctx = TestContext::new().await;

        let qr = check_qr(&ctx.ctx, "I am so cool").await?;
        assert_eq!(
            qr,
            Qr::Text {
                text: "I am so cool".to_string()
            }
        );

        Ok(())
    }

    #[async_std::test]
    async fn test_decode_vcard() -> Result<()> {
        let ctx = TestContext::new().await;

        let qr = check_qr(
            &ctx.ctx,
            "BEGIN:VCARD\nVERSION:3.0\nN:Last;First\nEMAIL;TYPE=INTERNET:stress@test.local\nEND:VCARD"
        ).await?;

        if let Qr::Addr { contact_id } = qr {
            let contact = Contact::get_by_id(&ctx.ctx, contact_id).await?;
            assert_eq!(contact.get_addr(), "stress@test.local");
            assert_eq!(contact.get_name(), "First Last");
            assert_eq!(contact.get_authname(), "");
            assert_eq!(contact.get_display_name(), "First Last");
        } else {
            bail!("Wrong QR code type");
        }

        Ok(())
    }

    #[async_std::test]
    async fn test_decode_matmsg() -> Result<()> {
        let ctx = TestContext::new().await;

        let qr = check_qr(
            &ctx.ctx,
            "MATMSG:TO:\n\nstress@test.local ; \n\nSUB:\n\nSubject here\n\nBODY:\n\nhelloworld\n;;",
        )
        .await?;

        if let Qr::Addr { contact_id } = qr {
            let contact = Contact::get_by_id(&ctx.ctx, contact_id).await?;
            assert_eq!(contact.get_addr(), "stress@test.local");
        } else {
            bail!("Wrong QR code type");
        }

        Ok(())
    }

    #[async_std::test]
    async fn test_decode_mailto() -> Result<()> {
        let ctx = TestContext::new().await;

        let qr = check_qr(
            &ctx.ctx,
            "mailto:stress@test.local?subject=hello&body=world",
        )
        .await?;
        if let Qr::Addr { contact_id } = qr {
            let contact = Contact::get_by_id(&ctx.ctx, contact_id).await?;
            assert_eq!(contact.get_addr(), "stress@test.local");
        } else {
            bail!("Wrong QR code type");
        }

        let res = check_qr(&ctx.ctx, "mailto:no-questionmark@example.org").await?;
        if let Qr::Addr { contact_id } = res {
            let contact = Contact::get_by_id(&ctx.ctx, contact_id).await?;
            assert_eq!(contact.get_addr(), "no-questionmark@example.org");
        } else {
            bail!("Wrong QR code type");
        }

        let res = check_qr(&ctx.ctx, "mailto:no-addr").await;
        assert!(res.is_err());

        Ok(())
    }

    #[async_std::test]
    async fn test_decode_smtp() -> Result<()> {
        let ctx = TestContext::new().await;

        if let Qr::Addr { contact_id } =
            check_qr(&ctx.ctx, "SMTP:stress@test.local:subjecthello:bodyworld").await?
        {
            let contact = Contact::get_by_id(&ctx.ctx, contact_id).await?;
            assert_eq!(contact.get_addr(), "stress@test.local");
        } else {
            bail!("Wrong QR code type");
        }

        Ok(())
    }

    #[async_std::test]
    async fn test_decode_openpgp_group() -> Result<()> {
        let ctx = TestContext::new().await;
        let qr = check_qr(
            &ctx.ctx,
            "OPENPGP4FPR:79252762C34C5096AF57958F4FC3D21A81B0F0A7#a=cli%40deltachat.de&g=test%20%3F+test%20%21&x=h-0oKQf2CDK&i=9JEXlxAqGM0&s=0V7LzL9cxRL"
        ).await?;
        if let Qr::AskVerifyGroup {
            contact_id,
            grpname,
            ..
        } = qr
        {
            assert_ne!(contact_id, 0);
            assert_eq!(grpname, "test ? test !");
        } else {
            bail!("Wrong QR code type");
        }

        // Test it again with lowercased "openpgp4fpr:" uri scheme
        let ctx = TestContext::new().await;
        let qr = check_qr(
            &ctx.ctx,
            "openpgp4fpr:79252762C34C5096AF57958F4FC3D21A81B0F0A7#a=cli%40deltachat.de&g=test%20%3F+test%20%21&x=h-0oKQf2CDK&i=9JEXlxAqGM0&s=0V7LzL9cxRL"
        ).await?;
        if let Qr::AskVerifyGroup {
            contact_id,
            grpname,
            ..
        } = qr
        {
            assert_ne!(contact_id, 0);
            assert_eq!(grpname, "test ? test !");

            let contact = Contact::get_by_id(&ctx.ctx, contact_id).await?;
            assert_eq!(contact.get_addr(), "cli@deltachat.de");
        } else {
            bail!("Wrong QR code type");
        }

        Ok(())
    }

    #[async_std::test]
    async fn test_decode_openpgp_secure_join() -> Result<()> {
        let ctx = TestContext::new().await;

        let qr = check_qr(
            &ctx.ctx,
            "OPENPGP4FPR:79252762C34C5096AF57958F4FC3D21A81B0F0A7#a=cli%40deltachat.de&n=J%C3%B6rn%20P.+P.&i=TbnwJ6lSvD5&s=0ejvbdFSQxB"
        ).await?;

        if let Qr::AskVerifyContact { contact_id, .. } = qr {
            assert_ne!(contact_id, 0);
        } else {
            bail!("Wrong QR code type");
        }

        // Test it again with lowercased "openpgp4fpr:" uri scheme
        let qr = check_qr(
            &ctx.ctx,
            "openpgp4fpr:79252762C34C5096AF57958F4FC3D21A81B0F0A7#a=cli%40deltachat.de&n=J%C3%B6rn%20P.+P.&i=TbnwJ6lSvD5&s=0ejvbdFSQxB"
        ).await?;

        if let Qr::AskVerifyContact { contact_id, .. } = qr {
            let contact = Contact::get_by_id(&ctx.ctx, contact_id).await?;
            assert_eq!(contact.get_addr(), "cli@deltachat.de");
            assert_eq!(contact.get_name(), "Jörn P. P.");
        } else {
            bail!("Wrong QR code type");
        }

        // Regression test
        let ctx = TestContext::new().await;
        let qr = check_qr(
            &ctx.ctx,
            "openpgp4fpr:79252762C34C5096AF57958F4FC3D21A81B0F0A7#a=cli%40deltachat.de&n=&i=TbnwJ6lSvD5&s=0ejvbdFSQxB"
        ).await?;

        if let Qr::AskVerifyContact { contact_id, .. } = qr {
            let contact = Contact::get_by_id(&ctx.ctx, contact_id).await?;
            assert_eq!(contact.get_addr(), "cli@deltachat.de");
            assert_eq!(contact.get_name(), "");
        } else {
            bail!("Wrong QR code type");
        }

        Ok(())
    }

    #[async_std::test]
    async fn test_decode_openpgp_fingerprint() -> Result<()> {
        let ctx = TestContext::new().await;

        let alice_contact_id = Contact::create(&ctx, "Alice", "alice@example.com")
            .await
            .context("failed to create contact")?;
        let pub_key = alice_keypair().public;
        let peerstate = Peerstate {
            addr: "alice@example.com".to_string(),
            last_seen: 1,
            last_seen_autocrypt: 1,
            prefer_encrypt: EncryptPreference::Mutual,
            public_key: Some(pub_key.clone()),
            public_key_fingerprint: Some(pub_key.fingerprint()),
            gossip_key: None,
            gossip_timestamp: 0,
            gossip_key_fingerprint: None,
            verified_key: None,
            verified_key_fingerprint: None,
            to_save: Some(ToSave::All),
            fingerprint_changed: false,
        };
        assert!(
            peerstate.save_to_db(&ctx.ctx.sql, true).await.is_ok(),
            "failed to save peerstate"
        );

        let qr = check_qr(
            &ctx.ctx,
            "OPENPGP4FPR:1234567890123456789012345678901234567890#a=alice@example.com",
        )
        .await?;
        if let Qr::FprMismatch { contact_id, .. } = qr {
            assert_eq!(contact_id, Some(alice_contact_id));
        } else {
            bail!("Wrong QR code type");
        }

        let qr = check_qr(
            &ctx.ctx,
            &format!("OPENPGP4FPR:{}#a=alice@example.com", pub_key.fingerprint()),
        )
        .await?;
        if let Qr::FprOk { contact_id, .. } = qr {
            assert_eq!(contact_id, alice_contact_id);
        } else {
            bail!("Wrong QR code type");
        }

        assert_eq!(
            check_qr(
                &ctx.ctx,
                "OPENPGP4FPR:1234567890123456789012345678901234567890#a=bob@example.org",
            )
            .await?,
            Qr::FprMismatch { contact_id: None }
        );

        Ok(())
    }

    #[async_std::test]
    async fn test_decode_openpgp_without_addr() -> Result<()> {
        let ctx = TestContext::new().await;

        let qr = check_qr(
            &ctx.ctx,
            "OPENPGP4FPR:1234567890123456789012345678901234567890",
        )
        .await?;
        assert_eq!(
            qr,
            Qr::FprWithoutAddr {
                fingerprint: "1234 5678 9012 3456 7890\n1234 5678 9012 3456 7890".to_string()
            }
        );

        // Test it again with lowercased "openpgp4fpr:" uri scheme

        let qr = check_qr(
            &ctx.ctx,
            "openpgp4fpr:1234567890123456789012345678901234567890",
        )
        .await?;
        assert_eq!(
            qr,
            Qr::FprWithoutAddr {
                fingerprint: "1234 5678 9012 3456 7890\n1234 5678 9012 3456 7890".to_string()
            }
        );

        let res = check_qr(&ctx.ctx, "OPENPGP4FPR:12345678901234567890").await;
        assert!(res.is_err());

        Ok(())
    }

    #[async_std::test]
    async fn test_withdraw_verifycontact() -> Result<()> {
        let alice = TestContext::new_alice().await;
        let qr = dc_get_securejoin_qr(&alice, None).await?;

        // scanning own verify-contact code offers withdrawing
        assert!(matches!(
            check_qr(&alice, &qr).await?,
            Qr::WithdrawVerifyContact { .. }
        ));
        set_config_from_qr(&alice, &qr).await?;

        // scanning withdrawn verify-contact code offers reviving
        assert!(matches!(
            check_qr(&alice, &qr).await?,
            Qr::ReviveVerifyContact { .. }
        ));
        set_config_from_qr(&alice, &qr).await?;
        assert!(matches!(
            check_qr(&alice, &qr).await?,
            Qr::WithdrawVerifyContact { .. }
        ));

        // someone else always scans as ask-verify-contact
        let bob = TestContext::new_bob().await;
        assert!(matches!(
            check_qr(&bob, &qr).await?,
            Qr::AskVerifyContact { .. }
        ));
        assert!(set_config_from_qr(&bob, &qr).await.is_err());

        Ok(())
    }

    #[async_std::test]
    async fn test_withdraw_verifygroup() -> Result<()> {
        let alice = TestContext::new_alice().await;
        let chat_id = create_group_chat(&alice, ProtectionStatus::Unprotected, "foo").await?;
        let qr = dc_get_securejoin_qr(&alice, Some(chat_id)).await?;

        // scanning own verify-group code offers withdrawing
        if let Qr::WithdrawVerifyGroup { grpname, .. } = check_qr(&alice, &qr).await? {
            assert_eq!(grpname, "foo");
        } else {
            bail!("Wrong QR type, expected WithdrawVerifyGroup");
        }
        set_config_from_qr(&alice, &qr).await?;

        // scanning withdrawn verify-group code offers reviving
        if let Qr::ReviveVerifyGroup { grpname, .. } = check_qr(&alice, &qr).await? {
            assert_eq!(grpname, "foo");
        } else {
            bail!("Wrong QR type, expected ReviveVerifyGroup");
        }

        // someone else always scans as ask-verify-group
        let bob = TestContext::new_bob().await;
        if let Qr::AskVerifyGroup { grpname, .. } = check_qr(&bob, &qr).await? {
            assert_eq!(grpname, "foo");
        } else {
            bail!("Wrong QR type, expected AskVerifyGroup");
        }
        assert!(set_config_from_qr(&bob, &qr).await.is_err());

        Ok(())
    }

    #[async_std::test]
    async fn test_decode_account() -> Result<()> {
        let ctx = TestContext::new().await;

        let qr = check_qr(
            &ctx.ctx,
            "DCACCOUNT:https://example.org/new_email?t=1w_7wDjgjelxeX884x96v3",
        )
        .await?;
        assert_eq!(
            qr,
            Qr::Account {
                domain: "example.org".to_string()
            }
        );

        // Test it again with lowercased "dcaccount:" uri scheme
        let qr = check_qr(
            &ctx.ctx,
            "dcaccount:https://example.org/new_email?t=1w_7wDjgjelxeX884x96v3",
        )
        .await?;
        assert_eq!(
            qr,
            Qr::Account {
                domain: "example.org".to_string()
            }
        );

        Ok(())
    }

    #[async_std::test]
    async fn test_decode_webrtc_instance() -> Result<()> {
        let ctx = TestContext::new().await;

        let qr = check_qr(&ctx.ctx, "DCWEBRTC:basicwebrtc:https://basicurl.com/$ROOM").await?;
        assert_eq!(
            qr,
            Qr::WebrtcInstance {
                domain: "basicurl.com".to_string(),
                instance_pattern: "basicwebrtc:https://basicurl.com/$ROOM".to_string()
            }
        );

        // Test it again with mixcased "dcWebRTC:" uri scheme
        let qr = check_qr(&ctx.ctx, "dcWebRTC:https://example.org/").await?;
        assert_eq!(
            qr,
            Qr::WebrtcInstance {
                domain: "example.org".to_string(),
                instance_pattern: "https://example.org/".to_string()
            }
        );

        Ok(())
    }

    #[async_std::test]
    async fn test_decode_account_bad_scheme() {
        let ctx = TestContext::new().await;
        let res = check_qr(
            &ctx.ctx,
            "DCACCOUNT:ftp://example.org/new_email?t=1w_7wDjgjelxeX884x96v3",
        )
        .await;
        assert!(res.is_err());

        // Test it again with lowercased "dcaccount:" uri scheme
        let res = check_qr(
            &ctx.ctx,
            "dcaccount:ftp://example.org/new_email?t=1w_7wDjgjelxeX884x96v3",
        )
        .await;
        assert!(res.is_err());
    }

    #[async_std::test]
    async fn test_set_config_from_qr() -> Result<()> {
        let ctx = TestContext::new().await;

        assert!(ctx.ctx.get_config(Config::WebrtcInstance).await?.is_none());

        let res = set_config_from_qr(&ctx.ctx, "badqr:https://example.org/").await;
        assert!(res.is_err());
        assert!(ctx.ctx.get_config(Config::WebrtcInstance).await?.is_none());

        let res = set_config_from_qr(&ctx.ctx, "https://no.qr").await;
        assert!(res.is_err());
        assert!(ctx.ctx.get_config(Config::WebrtcInstance).await?.is_none());

        let res = set_config_from_qr(&ctx.ctx, "dcwebrtc:https://example.org/").await;
        assert!(res.is_ok());
        assert_eq!(
            ctx.ctx.get_config(Config::WebrtcInstance).await?.unwrap(),
            "https://example.org/"
        );

        let res =
            set_config_from_qr(&ctx.ctx, "DCWEBRTC:basicwebrtc:https://foo.bar/?$ROOM&test").await;
        assert!(res.is_ok());
        assert_eq!(
            ctx.ctx.get_config(Config::WebrtcInstance).await?.unwrap(),
            "basicwebrtc:https://foo.bar/?$ROOM&test"
        );

        Ok(())
    }
}
