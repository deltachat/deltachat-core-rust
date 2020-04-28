//! # QR code module

use lazy_static::lazy_static;
use percent_encoding::percent_decode_str;
use reqwest::Url;
use serde::Deserialize;

use crate::chat;
use crate::config::*;
use crate::constants::Blocked;
use crate::contact::*;
use crate::context::Context;
use crate::error::{bail, ensure, format_err, Error};
use crate::key::dc_format_fingerprint;
use crate::key::dc_normalize_fingerprint;
use crate::lot::{Lot, LotState};
use crate::param::*;
use crate::peerstate::*;

const OPENPGP4FPR_SCHEME: &str = "OPENPGP4FPR:"; // yes: uppercase
const DCACCOUNT_SCHEME: &str = "DCACCOUNT:";
const MAILTO_SCHEME: &str = "mailto:";
const MATMSG_SCHEME: &str = "MATMSG:";
const VCARD_SCHEME: &str = "BEGIN:VCARD";
const SMTP_SCHEME: &str = "SMTP:";
const HTTP_SCHEME: &str = "http://";
const HTTPS_SCHEME: &str = "https://";

// Make it easy to convert errors into the final `Lot`.
impl Into<Lot> for Error {
    fn into(self) -> Lot {
        let mut l = Lot::new();
        l.state = LotState::QrError;
        l.text1 = Some(self.to_string());

        l
    }
}

fn starts_with_ignore_case(string: &str, pattern: &str) -> bool {
    string.to_lowercase().starts_with(&pattern.to_lowercase())
}

/// Check a scanned QR code.
/// The function should be called after a QR code is scanned.
/// The function takes the raw text scanned and checks what can be done with it.
pub fn check_qr(context: &Context, qr: impl AsRef<str>) -> Lot {
    let qr = qr.as_ref();

    info!(context, "Scanned QR code: {}", qr);

    if starts_with_ignore_case(qr, OPENPGP4FPR_SCHEME) {
        decode_openpgp(context, qr)
    } else if starts_with_ignore_case(qr, DCACCOUNT_SCHEME) {
        decode_account(context, qr)
    } else if qr.starts_with(MAILTO_SCHEME) {
        decode_mailto(context, qr)
    } else if qr.starts_with(SMTP_SCHEME) {
        decode_smtp(context, qr)
    } else if qr.starts_with(MATMSG_SCHEME) {
        decode_matmsg(context, qr)
    } else if qr.starts_with(VCARD_SCHEME) {
        decode_vcard(context, qr)
    } else if qr.starts_with(HTTP_SCHEME) || qr.starts_with(HTTPS_SCHEME) {
        Lot::from_url(qr)
    } else {
        Lot::from_text(qr)
    }
}

/// scheme: `OPENPGP4FPR:FINGERPRINT#a=ADDR&n=NAME&i=INVITENUMBER&s=AUTH`
///     or: `OPENPGP4FPR:FINGERPRINT#a=ADDR&g=GROUPNAME&x=GROUPID&i=INVITENUMBER&s=AUTH`
fn decode_openpgp(context: &Context, qr: &str) -> Lot {
    let payload = &qr[OPENPGP4FPR_SCHEME.len()..];

    let (fingerprint, fragment) = match payload.find('#').map(|offset| {
        let (fp, rest) = payload.split_at(offset);
        // need to remove the # from the fragment
        (fp, &rest[1..])
    }) {
        Some(pair) => pair,
        None => (payload, ""),
    };

    // replace & with \n to match expected param format
    let fragment = fragment.replace('&', "\n");

    // Then parse the parameters
    let param: Params = match fragment.parse() {
        Ok(params) => params,
        Err(err) => return err.into(),
    };

    let addr = if let Some(addr) = param.get(Param::Forwarded) {
        match normalize_address(addr) {
            Ok(addr) => Some(addr),
            Err(err) => return err.into(),
        }
    } else {
        None
    };

    // what is up with that param name?
    let name = if let Some(encoded_name) = param.get(Param::SetLongitude) {
        let encoded_name = encoded_name.replace("+", "%20"); // sometimes spaces are encoded as `+`
        match percent_decode_str(&encoded_name).decode_utf8() {
            Ok(name) => name.to_string(),
            Err(err) => return format_err!("Invalid name: {}", err).into(),
        }
    } else {
        "".to_string()
    };

    let invitenumber = param.get(Param::ProfileImage).map(|s| s.to_string());
    let auth = param.get(Param::Auth).map(|s| s.to_string());
    let grpid = param.get(Param::GroupId).map(|s| s.to_string());

    let grpname = if grpid.is_some() {
        if let Some(encoded_name) = param.get(Param::GroupName) {
            let encoded_name = encoded_name.replace("+", "%20"); // sometimes spaces are encoded as `+`
            match percent_decode_str(&encoded_name).decode_utf8() {
                Ok(name) => Some(name.to_string()),
                Err(err) => return format_err!("Invalid group name: {}", err).into(),
            }
        } else {
            None
        }
    } else {
        None
    };

    let fingerprint = dc_normalize_fingerprint(fingerprint);

    // ensure valid fingerprint
    if fingerprint.len() != 40 {
        return format_err!("Bad fingerprint length in QR code").into();
    }

    println!(
        "{:?} {:?} {:?} {:?} {:?} {:?} {:?}",
        addr, name, invitenumber, auth, grpid, grpname, fingerprint
    );

    let mut lot = Lot::new();

    // retrieve known state for this fingerprint
    let peerstate = Peerstate::from_fingerprint(context, &context.sql, &fingerprint);

    if invitenumber.is_none() || auth.is_none() {
        if let Some(peerstate) = peerstate {
            lot.state = LotState::QrFprOk;

            lot.id = Contact::add_or_lookup(
                context,
                name,
                peerstate.addr.clone(),
                Origin::UnhandledQrScan,
            )
            .map(|(id, _)| id)
            .unwrap_or_default();

            let (id, _) = chat::create_or_lookup_by_contact_id(context, lot.id, Blocked::Deaddrop)
                .unwrap_or_default();

            chat::add_info_msg(context, id, format!("{} verified.", peerstate.addr));
        } else {
            lot.state = LotState::QrFprWithoutAddr;
            lot.text1 = Some(dc_format_fingerprint(&fingerprint));
        }
    } else if let Some(addr) = addr {
        if grpid.is_some() && grpname.is_some() {
            lot.state = LotState::QrAskVerifyGroup;
            lot.text1 = grpname;
            lot.text2 = grpid
        } else {
            lot.state = LotState::QrAskVerifyContact;
        }
        lot.id = Contact::add_or_lookup(context, &name, &addr, Origin::UnhandledQrScan)
            .map(|(id, _)| id)
            .unwrap_or_default();

        lot.fingerprint = Some(fingerprint);
        lot.invitenumber = invitenumber;
        lot.auth = auth;
    } else {
        return format_err!("Missing address").into();
    }

    lot
}

/// scheme: `DCACCOUNT:https://example.org/new_email?t=1w_7wDjgjelxeX884x96v3`
fn decode_account(_context: &Context, qr: &str) -> Lot {
    let payload = &qr[DCACCOUNT_SCHEME.len()..];

    let mut lot = Lot::new();

    if let Ok(url) = Url::parse(payload) {
        if url.scheme() == "https" {
            lot.state = LotState::QrAccount;
            lot.text1 = url.host_str().map(|x| x.to_string());
        } else {
            lot.state = LotState::QrError;
            lot.text1 = Some(format!("Bad scheme for account url: {}", payload));
        }
    } else {
        lot.state = LotState::QrError;
        lot.text1 = Some(format!("Invalid account url: {}", payload));
    }

    lot
}

#[derive(Debug, Deserialize)]
struct CreateAccountResponse {
    email: String,
    password: String,
}

/// take a qr of the type DC_QR_ACCOUNT, parse it's parameters,
/// download additional information from the contained url and set the parameters.
/// on success, a configure::configure() should be able to log in to the account
pub fn set_config_from_qr(context: &Context, qr: &str) -> Result<(), Error> {
    let url_str = &qr[DCACCOUNT_SCHEME.len()..];

    let response = reqwest::blocking::Client::new().post(url_str).send();
    if response.is_err() {
        bail!("Cannot create account, request to {} failed", url_str);
    }
    let response = response.unwrap();
    if !response.status().is_success() {
        bail!("Request to {} unsuccessful: {:?}", url_str, response);
    }

    let parsed: reqwest::Result<CreateAccountResponse> = response.json();
    if parsed.is_err() {
        bail!(
            "Failed to parse JSON response from {}: error: {:?}",
            url_str,
            parsed
        );
    }
    println!("response: {:?}", &parsed);
    let parsed = parsed.unwrap();

    context.set_config(Config::Addr, Some(&parsed.email))?;
    context.set_config(Config::MailPw, Some(&parsed.password))?;

    Ok(())
}

/// Extract address for the mailto scheme.
///
/// Scheme: `mailto:addr...?subject=...&body=..`
fn decode_mailto(context: &Context, qr: &str) -> Lot {
    let payload = &qr[MAILTO_SCHEME.len()..];

    let addr = if let Some(query_index) = payload.find('?') {
        &payload[..query_index]
    } else {
        payload
    };

    let addr = match normalize_address(addr) {
        Ok(addr) => addr,
        Err(err) => return err.into(),
    };

    let name = "".to_string();
    Lot::from_address(context, name, addr)
}

/// Extract address for the smtp scheme.
///
/// Scheme: `SMTP:addr...:subject...:body...`
fn decode_smtp(context: &Context, qr: &str) -> Lot {
    let payload = &qr[SMTP_SCHEME.len()..];

    let addr = if let Some(query_index) = payload.find(':') {
        &payload[..query_index]
    } else {
        return format_err!("Invalid SMTP found").into();
    };

    let addr = match normalize_address(addr) {
        Ok(addr) => addr,
        Err(err) => return err.into(),
    };
    let name = "".to_string();
    Lot::from_address(context, name, addr)
}

/// Extract address for the matmsg scheme.
///
/// Scheme: `MATMSG:TO:addr...;SUB:subject...;BODY:body...;`
///
/// There may or may not be linebreaks after the fields.
fn decode_matmsg(context: &Context, qr: &str) -> Lot {
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
        return format_err!("Invalid MATMSG found").into();
    };

    let addr = match normalize_address(addr) {
        Ok(addr) => addr,
        Err(err) => return err.into(),
    };

    let name = "".to_string();
    Lot::from_address(context, name, addr)
}

lazy_static! {
    static ref VCARD_NAME_RE: regex::Regex =
        regex::Regex::new(r"(?m)^N:([^;]*);([^;\n]*)").unwrap();
    static ref VCARD_EMAIL_RE: regex::Regex =
        regex::Regex::new(r"(?m)^EMAIL([^:\n]*):([^;\n]*)").unwrap();
}

/// Extract address for the matmsg scheme.
///
/// Scheme: `VCARD:BEGIN\nN:last name;first name;...;\nEMAIL;<type>:addr...;
fn decode_vcard(context: &Context, qr: &str) -> Lot {
    let name = VCARD_NAME_RE
        .captures(qr)
        .map(|caps| {
            let last_name = &caps[1];
            let first_name = &caps[2];

            format!("{} {}", first_name.trim(), last_name.trim())
        })
        .unwrap_or_default();

    let addr = if let Some(caps) = VCARD_EMAIL_RE.captures(qr) {
        match normalize_address(caps[2].trim()) {
            Ok(addr) => addr,
            Err(err) => return err.into(),
        }
    } else {
        return format_err!("Bad e-mail address").into();
    };

    Lot::from_address(context, name, addr)
}

impl Lot {
    pub fn from_text(text: impl AsRef<str>) -> Self {
        let mut l = Lot::new();
        l.state = LotState::QrText;
        l.text1 = Some(text.as_ref().to_string());

        l
    }

    pub fn from_url(url: impl AsRef<str>) -> Self {
        let mut l = Lot::new();
        l.state = LotState::QrUrl;
        l.text1 = Some(url.as_ref().to_string());

        l
    }

    pub fn from_address(context: &Context, name: String, addr: String) -> Self {
        let mut l = Lot::new();
        l.state = LotState::QrAddr;
        l.id = match Contact::add_or_lookup(context, name, addr, Origin::UnhandledQrScan) {
            Ok((id, _)) => id,
            Err(err) => return err.into(),
        };

        l
    }
}

/// URL decodes a given address, does basic email validation on the result.
fn normalize_address(addr: &str) -> Result<String, Error> {
    // urldecoding is needed at least for OPENPGP4FPR but should not hurt in the other cases
    let new_addr = percent_decode_str(addr).decode_utf8()?;
    let new_addr = addr_normalize(&new_addr);

    ensure!(may_be_valid_addr(&new_addr), "Bad e-mail address");

    Ok(new_addr.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::test_utils::dummy_context;

    #[test]
    fn test_decode_http() {
        let ctx = dummy_context();

        let res = check_qr(&ctx.ctx, "http://www.hello.com");

        assert_eq!(res.get_state(), LotState::QrUrl);
        assert_eq!(res.get_id(), 0);
        assert_eq!(res.get_text1().unwrap(), "http://www.hello.com");
        assert!(res.get_text2().is_none());
    }

    #[test]
    fn test_decode_https() {
        let ctx = dummy_context();

        let res = check_qr(&ctx.ctx, "https://www.hello.com");

        assert_eq!(res.get_state(), LotState::QrUrl);
        assert_eq!(res.get_id(), 0);
        assert_eq!(res.get_text1().unwrap(), "https://www.hello.com");
        assert!(res.get_text2().is_none());
    }

    #[test]
    fn test_decode_text() {
        let ctx = dummy_context();

        let res = check_qr(&ctx.ctx, "I am so cool");

        assert_eq!(res.get_state(), LotState::QrText);
        assert_eq!(res.get_id(), 0);
        assert_eq!(res.get_text1().unwrap(), "I am so cool");
        assert!(res.get_text2().is_none());
    }

    #[test]
    fn test_decode_vcard() {
        let ctx = dummy_context();

        let res = check_qr(
            &ctx.ctx,
            "BEGIN:VCARD\nVERSION:3.0\nN:Last;First\nEMAIL;TYPE=INTERNET:stress@test.local\nEND:VCARD"
        );

        println!("{:?}", res);
        assert_eq!(res.get_state(), LotState::QrAddr);
        assert_ne!(res.get_id(), 0);

        let contact = Contact::get_by_id(&ctx.ctx, res.get_id()).unwrap();
        assert_eq!(contact.get_addr(), "stress@test.local");
        assert_eq!(contact.get_name(), "First Last");
    }

    #[test]
    fn test_decode_matmsg() {
        let ctx = dummy_context();

        let res = check_qr(
            &ctx.ctx,
            "MATMSG:TO:\n\nstress@test.local ; \n\nSUB:\n\nSubject here\n\nBODY:\n\nhelloworld\n;;",
        );

        println!("{:?}", res);
        assert_eq!(res.get_state(), LotState::QrAddr);
        assert_ne!(res.get_id(), 0);

        let contact = Contact::get_by_id(&ctx.ctx, res.get_id()).unwrap();
        assert_eq!(contact.get_addr(), "stress@test.local");
    }

    #[test]
    fn test_decode_mailto() {
        let ctx = dummy_context();

        let res = check_qr(
            &ctx.ctx,
            "mailto:stress@test.local?subject=hello&body=world",
        );
        println!("{:?}", res);
        assert_eq!(res.get_state(), LotState::QrAddr);
        assert_ne!(res.get_id(), 0);
        let contact = Contact::get_by_id(&ctx.ctx, res.get_id()).unwrap();
        assert_eq!(contact.get_addr(), "stress@test.local");

        let res = check_qr(&ctx.ctx, "mailto:no-questionmark@example.org");
        assert_eq!(res.get_state(), LotState::QrAddr);
        assert_ne!(res.get_id(), 0);
        let contact = Contact::get_by_id(&ctx.ctx, res.get_id()).unwrap();
        assert_eq!(contact.get_addr(), "no-questionmark@example.org");

        let res = check_qr(&ctx.ctx, "mailto:no-addr");
        assert_eq!(res.get_state(), LotState::QrError);
        assert!(res.get_text1().is_some());
    }

    #[test]
    fn test_decode_smtp() {
        let ctx = dummy_context();

        let res = check_qr(&ctx.ctx, "SMTP:stress@test.local:subjecthello:bodyworld");

        println!("{:?}", res);
        assert_eq!(res.get_state(), LotState::QrAddr);
        assert_ne!(res.get_id(), 0);

        let contact = Contact::get_by_id(&ctx.ctx, res.get_id()).unwrap();
        assert_eq!(contact.get_addr(), "stress@test.local");
    }

    #[test]
    fn test_decode_openpgp_group() {
        let ctx = dummy_context();

        let res = check_qr(
            &ctx.ctx,
            "OPENPGP4FPR:79252762C34C5096AF57958F4FC3D21A81B0F0A7#a=cli%40deltachat.de&g=test%20%3F+test%20%21&x=h-0oKQf2CDK&i=9JEXlxAqGM0&s=0V7LzL9cxRL"
        );

        println!("{:?}", res);
        assert_eq!(res.get_state(), LotState::QrAskVerifyGroup);
        assert_ne!(res.get_id(), 0);
        assert_eq!(res.get_text1().unwrap(), "test ? test !");

        // Test it again with lowercased "openpgp4fpr:" uri scheme
        let res = check_qr(
            &ctx.ctx,
            "openpgp4fpr:79252762C34C5096AF57958F4FC3D21A81B0F0A7#a=cli%40deltachat.de&g=test%20%3F+test%20%21&x=h-0oKQf2CDK&i=9JEXlxAqGM0&s=0V7LzL9cxRL"
        );

        println!("{:?}", res);
        assert_eq!(res.get_state(), LotState::QrAskVerifyGroup);
        assert_ne!(res.get_id(), 0);
        assert_eq!(res.get_text1().unwrap(), "test ? test !");

        let contact = Contact::get_by_id(&ctx.ctx, res.get_id()).unwrap();
        assert_eq!(contact.get_addr(), "cli@deltachat.de");
    }

    #[test]
    fn test_decode_openpgp_secure_join() {
        let ctx = dummy_context();

        let res = check_qr(
            &ctx.ctx,
            "OPENPGP4FPR:79252762C34C5096AF57958F4FC3D21A81B0F0A7#a=cli%40deltachat.de&n=J%C3%B6rn%20P.+P.&i=TbnwJ6lSvD5&s=0ejvbdFSQxB"
        );

        println!("{:?}", res);
        assert_eq!(res.get_state(), LotState::QrAskVerifyContact);
        assert_ne!(res.get_id(), 0);

        // Test it again with lowercased "openpgp4fpr:" uri scheme
        let res = check_qr(
            &ctx.ctx,
            "openpgp4fpr:79252762C34C5096AF57958F4FC3D21A81B0F0A7#a=cli%40deltachat.de&n=J%C3%B6rn%20P.+P.&i=TbnwJ6lSvD5&s=0ejvbdFSQxB"
        );

        println!("{:?}", res);
        assert_eq!(res.get_state(), LotState::QrAskVerifyContact);
        assert_ne!(res.get_id(), 0);

        let contact = Contact::get_by_id(&ctx.ctx, res.get_id()).unwrap();
        assert_eq!(contact.get_addr(), "cli@deltachat.de");
        assert_eq!(contact.get_name(), "Jörn P. P.");
    }

    #[test]
    fn test_decode_openpgp_without_addr() {
        let ctx = dummy_context();

        let res = check_qr(
            &ctx.ctx,
            "OPENPGP4FPR:1234567890123456789012345678901234567890",
        );
        assert_eq!(res.get_state(), LotState::QrFprWithoutAddr);
        assert_eq!(
            res.get_text1().unwrap(),
            "1234 5678 9012 3456 7890\n1234 5678 9012 3456 7890"
        );
        assert_eq!(res.get_id(), 0);

        // Test it again with lowercased "openpgp4fpr:" uri scheme

        let res = check_qr(
            &ctx.ctx,
            "openpgp4fpr:1234567890123456789012345678901234567890",
        );
        assert_eq!(res.get_state(), LotState::QrFprWithoutAddr);
        assert_eq!(
            res.get_text1().unwrap(),
            "1234 5678 9012 3456 7890\n1234 5678 9012 3456 7890"
        );
        assert_eq!(res.get_id(), 0);

        let res = check_qr(&ctx.ctx, "OPENPGP4FPR:12345678901234567890");
        assert_eq!(res.get_state(), LotState::QrError);
        assert_eq!(res.get_id(), 0);
    }

    #[test]
    fn test_decode_account() {
        let ctx = dummy_context();

        let res = check_qr(
            &ctx.ctx,
            "DCACCOUNT:https://example.org/new_email?t=1w_7wDjgjelxeX884x96v3",
        );
        assert_eq!(res.get_state(), LotState::QrAccount);
        assert_eq!(res.get_text1().unwrap(), "example.org");

        // Test it again with lowercased "dcaccount:" uri scheme
        let res = check_qr(
            &ctx.ctx,
            "dcaccount:https://example.org/new_email?t=1w_7wDjgjelxeX884x96v3",
        );
        assert_eq!(res.get_state(), LotState::QrAccount);
        assert_eq!(res.get_text1().unwrap(), "example.org");
    }

    #[test]
    fn test_decode_account_bad_scheme() {
        let ctx = dummy_context();

        let res = check_qr(
            &ctx.ctx,
            "DCACCOUNT:http://example.org/new_email?t=1w_7wDjgjelxeX884x96v3",
        );
        assert_eq!(res.get_state(), LotState::QrError);
        assert!(res.get_text1().is_some());

        // Test it again with lowercased "dcaccount:" uri scheme
        let res = check_qr(
            &ctx.ctx,
            "dcaccount:http://example.org/new_email?t=1w_7wDjgjelxeX884x96v3",
        );
        assert_eq!(res.get_state(), LotState::QrError);
        assert!(res.get_text1().is_some());
    }
}
