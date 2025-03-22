//! # QR code module.

mod dclogin_scheme;
use std::collections::BTreeMap;

use anyhow::{anyhow, bail, ensure, Context as _, Result};
pub use dclogin_scheme::LoginOptions;
use deltachat_contact_tools::{addr_normalize, may_be_valid_addr, ContactAddress};
use once_cell::sync::Lazy;
use percent_encoding::{percent_decode_str, percent_encode, NON_ALPHANUMERIC};
use serde::Deserialize;

use self::dclogin_scheme::configure_from_login_qr;
use crate::chat::ChatIdBlocked;
use crate::config::Config;
use crate::constants::Blocked;
use crate::contact::{Contact, ContactId, Origin};
use crate::context::Context;
use crate::events::EventType;
use crate::key::Fingerprint;
use crate::message::Message;
use crate::net::http::post_empty;
use crate::net::proxy::{ProxyConfig, DEFAULT_SOCKS_PORT};
use crate::peerstate::Peerstate;
use crate::token;
use crate::tools::validate_id;

const OPENPGP4FPR_SCHEME: &str = "OPENPGP4FPR:"; // yes: uppercase
const IDELTACHAT_SCHEME: &str = "https://i.delta.chat/#";
const IDELTACHAT_NOSLASH_SCHEME: &str = "https://i.delta.chat#";
const DCACCOUNT_SCHEME: &str = "DCACCOUNT:";
pub(super) const DCLOGIN_SCHEME: &str = "DCLOGIN:";
const DCWEBRTC_SCHEME: &str = "DCWEBRTC:";
const TG_SOCKS_SCHEME: &str = "https://t.me/socks";
const MAILTO_SCHEME: &str = "mailto:";
const MATMSG_SCHEME: &str = "MATMSG:";
const VCARD_SCHEME: &str = "BEGIN:VCARD";
const SMTP_SCHEME: &str = "SMTP:";
const HTTPS_SCHEME: &str = "https://";
const SHADOWSOCKS_SCHEME: &str = "ss://";

/// Backup transfer based on iroh-net.
pub(crate) const DCBACKUP_SCHEME_PREFIX: &str = "DCBACKUP";

/// Version written to Backups and Backup-QR-Codes.
/// Imports will fail when they have a larger version.
pub(crate) const DCBACKUP_VERSION: i32 = 2;

/// Scanned QR code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Qr {
    /// Ask the user whether to verify the contact.
    ///
    /// If the user agrees, pass this QR code to [`crate::securejoin::join_securejoin`].
    AskVerifyContact {
        /// ID of the contact.
        contact_id: ContactId,

        /// Fingerprint of the contact key as scanned from the QR code.
        fingerprint: Fingerprint,

        /// Invite number.
        invitenumber: String,

        /// Authentication code.
        authcode: String,
    },

    /// Ask the user whether to join the group.
    AskVerifyGroup {
        /// Group name.
        grpname: String,

        /// Group ID.
        grpid: String,

        /// ID of the contact.
        contact_id: ContactId,

        /// Fingerprint of the contact key as scanned from the QR code.
        fingerprint: Fingerprint,

        /// Invite number.
        invitenumber: String,

        /// Authentication code.
        authcode: String,
    },

    /// Contact fingerprint is verified.
    ///
    /// Ask the user if they want to start chatting.
    FprOk {
        /// Contact ID.
        contact_id: ContactId,
    },

    /// Scanned fingerprint does not match the last seen fingerprint.
    FprMismatch {
        /// Contact ID.
        contact_id: Option<ContactId>,
    },

    /// The scanned QR code contains a fingerprint but no e-mail address.
    FprWithoutAddr {
        /// Key fingerprint.
        fingerprint: String,
    },

    /// Ask the user if they want to create an account on the given domain.
    Account {
        /// Server domain name.
        domain: String,
    },

    /// Provides a backup that can be retrieved using iroh-net based backup transfer protocol.
    Backup2 {
        /// Iroh node address.
        node_addr: iroh::NodeAddr,

        /// Authentication token.
        auth_token: String,
    },

    /// The QR code is a backup, but it is too new. The user has to update its Delta Chat.
    BackupTooNew {},

    /// Ask the user if they want to use the given service for video chats.
    WebrtcInstance {
        /// Server domain name.
        domain: String,

        /// URL pattern for video chat rooms.
        instance_pattern: String,
    },

    /// Ask the user if they want to use the given proxy.
    ///
    /// Note that HTTP(S) URLs without a path
    /// and query parameters are treated as HTTP(S) proxy URL.
    /// UI may want to still offer to open the URL
    /// in the browser if QR code contents
    /// starts with `http://` or `https://`
    /// and the QR code was not scanned from
    /// the proxy configuration screen.
    Proxy {
        /// Proxy URL.
        ///
        /// This is the URL that is going to be added.
        url: String,

        /// Host extracted from the URL to display in the UI.
        host: String,

        /// Port extracted from the URL to display in the UI.
        port: u16,
    },

    /// Contact address is scanned.
    ///
    /// Optionally, a draft message could be provided.
    /// Ask the user if they want to start chatting.
    Addr {
        /// Contact ID.
        contact_id: ContactId,

        /// Draft message.
        draft: Option<String>,
    },

    /// URL scanned.
    ///
    /// Ask the user if they want to open a browser or copy the URL to clipboard.
    Url {
        /// URL.
        url: String,
    },

    /// Text scanned.
    ///
    /// Ask the user if they want to copy the text to clipboard.
    Text {
        /// Scanned text.
        text: String,
    },

    /// Ask the user if they want to withdraw their own QR code.
    WithdrawVerifyContact {
        /// Contact ID.
        contact_id: ContactId,

        /// Fingerprint of the contact key as scanned from the QR code.
        fingerprint: Fingerprint,

        /// Invite number.
        invitenumber: String,

        /// Authentication code.
        authcode: String,
    },

    /// Ask the user if they want to withdraw their own group invite QR code.
    WithdrawVerifyGroup {
        /// Group name.
        grpname: String,

        /// Group ID.
        grpid: String,

        /// Contact ID.
        contact_id: ContactId,

        /// Fingerprint of the contact key as scanned from the QR code.
        fingerprint: Fingerprint,

        /// Invite number.
        invitenumber: String,

        /// Authentication code.
        authcode: String,
    },

    /// Ask the user if they want to revive their own QR code.
    ReviveVerifyContact {
        /// Contact ID.
        contact_id: ContactId,

        /// Fingerprint of the contact key as scanned from the QR code.
        fingerprint: Fingerprint,

        /// Invite number.
        invitenumber: String,

        /// Authentication code.
        authcode: String,
    },

    /// Ask the user if they want to revive their own group invite QR code.
    ReviveVerifyGroup {
        /// Group name.
        grpname: String,

        /// Group ID.
        grpid: String,

        /// Contact ID.
        contact_id: ContactId,

        /// Fingerprint of the contact key as scanned from the QR code.
        fingerprint: Fingerprint,

        /// Invite number.
        invitenumber: String,

        /// Authentication code.
        authcode: String,
    },

    /// `dclogin:` scheme parameters.
    ///
    /// Ask the user if they want to login with the email address.
    Login {
        /// Email address.
        address: String,

        /// Login parameters.
        options: LoginOptions,
    },
}

// hack around the changed JSON accidentally used by an iroh upgrade, see #6518 for more details and for code snippet.
// this hack is mainly needed to give ppl time to upgrade and can be removed after some months (added 2025-02)
fn fix_add_second_device_qr(qr: &str) -> String {
    qr.replacen(r#","info":{"relay_url":"#, r#","relay_url":"#, 1)
        .replacen(r#""]}}"#, r#""]}"#, 1)
}

fn starts_with_ignore_case(string: &str, pattern: &str) -> bool {
    string.to_lowercase().starts_with(&pattern.to_lowercase())
}

/// Checks a scanned QR code.
///
/// The function should be called after a QR code is scanned.
/// The function takes the raw text scanned and checks what can be done with it.
pub async fn check_qr(context: &Context, qr: &str) -> Result<Qr> {
    let qr = qr.trim();
    let qrcode = if starts_with_ignore_case(qr, OPENPGP4FPR_SCHEME) {
        decode_openpgp(context, qr)
            .await
            .context("failed to decode OPENPGP4FPR QR code")?
    } else if qr.starts_with(IDELTACHAT_SCHEME) {
        decode_ideltachat(context, IDELTACHAT_SCHEME, qr).await?
    } else if qr.starts_with(IDELTACHAT_NOSLASH_SCHEME) {
        decode_ideltachat(context, IDELTACHAT_NOSLASH_SCHEME, qr).await?
    } else if starts_with_ignore_case(qr, DCACCOUNT_SCHEME) {
        decode_account(qr)?
    } else if starts_with_ignore_case(qr, DCLOGIN_SCHEME) {
        dclogin_scheme::decode_login(qr)?
    } else if starts_with_ignore_case(qr, DCWEBRTC_SCHEME) {
        decode_webrtc_instance(context, qr)?
    } else if starts_with_ignore_case(qr, TG_SOCKS_SCHEME) {
        decode_tg_socks_proxy(context, qr)?
    } else if qr.starts_with(SHADOWSOCKS_SCHEME) {
        decode_shadowsocks_proxy(qr)?
    } else if starts_with_ignore_case(qr, DCBACKUP_SCHEME_PREFIX) {
        let qr_fixed = fix_add_second_device_qr(qr);
        decode_backup2(&qr_fixed)?
    } else if qr.starts_with(MAILTO_SCHEME) {
        decode_mailto(context, qr).await?
    } else if qr.starts_with(SMTP_SCHEME) {
        decode_smtp(context, qr).await?
    } else if qr.starts_with(MATMSG_SCHEME) {
        decode_matmsg(context, qr).await?
    } else if qr.starts_with(VCARD_SCHEME) {
        decode_vcard(context, qr).await?
    } else if let Ok(url) = url::Url::parse(qr) {
        match url.scheme() {
            "socks5" => Qr::Proxy {
                url: qr.to_string(),
                host: url.host_str().context("URL has no host")?.to_string(),
                port: url.port().unwrap_or(DEFAULT_SOCKS_PORT),
            },
            "http" | "https" => {
                // Parsing with a non-standard scheme
                // is a hack to work around the `url` crate bug
                // <https://github.com/servo/rust-url/issues/957>.
                let url = if let Some(rest) = qr.strip_prefix("http://") {
                    url::Url::parse(&format!("foobarbaz://{rest}"))?
                } else if let Some(rest) = qr.strip_prefix("https://") {
                    url::Url::parse(&format!("foobarbaz://{rest}"))?
                } else {
                    // Should not happen.
                    url
                };

                if url.port().is_none() | (url.path() != "") | url.query().is_some() {
                    // URL without a port, with a path or query cannot be a proxy URL.
                    Qr::Url {
                        url: qr.to_string(),
                    }
                } else {
                    Qr::Proxy {
                        url: qr.to_string(),
                        host: url.host_str().context("URL has no host")?.to_string(),
                        port: url
                            .port_or_known_default()
                            .context("HTTP(S) URLs are guaranteed to return Some port")?,
                    }
                }
            }
            _ => Qr::Url {
                url: qr.to_string(),
            },
        }
    } else {
        Qr::Text {
            text: qr.to_string(),
        }
    };
    Ok(qrcode)
}

/// Formats the text of the [`Qr::Backup2`] variant.
///
/// This is the inverse of [`check_qr`] for that variant only.
///
/// TODO: Refactor this so all variants have a correct [`Display`] and transform `check_qr`
/// into `FromStr`.
pub fn format_backup(qr: &Qr) -> Result<String> {
    match qr {
        Qr::Backup2 {
            ref node_addr,
            ref auth_token,
        } => {
            let node_addr = serde_json::to_string(node_addr)?;
            Ok(format!(
                "{DCBACKUP_SCHEME_PREFIX}{DCBACKUP_VERSION}:{auth_token}&{node_addr}"
            ))
        }
        _ => Err(anyhow!("Not a backup QR code")),
    }
}

/// scheme: `OPENPGP4FPR:FINGERPRINT#a=ADDR&n=NAME&i=INVITENUMBER&s=AUTH`
///     or: `OPENPGP4FPR:FINGERPRINT#a=ADDR&g=GROUPNAME&x=GROUPID&i=INVITENUMBER&s=AUTH`
///     or: `OPENPGP4FPR:FINGERPRINT#a=ADDR`
async fn decode_openpgp(context: &Context, qr: &str) -> Result<Qr> {
    let payload = qr
        .get(OPENPGP4FPR_SCHEME.len()..)
        .context("Invalid OPENPGP4FPR scheme")?;

    // macOS and iOS sometimes replace the # with %23 (uri encode it), we should be able to parse this wrong format too.
    // see issue https://github.com/deltachat/deltachat-core-rust/issues/1969 for more info
    let (fingerprint, fragment) = match payload
        .split_once('#')
        .or_else(|| payload.split_once("%23"))
    {
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
        let encoded_name = encoded_name.replace('+', "%20"); // sometimes spaces are encoded as `+`
        match percent_decode_str(&encoded_name).decode_utf8() {
            Ok(name) => name.to_string(),
            Err(err) => bail!("Invalid name: {}", err),
        }
    } else {
        "".to_string()
    };

    let invitenumber = param
        .get("i")
        .filter(|&s| validate_id(s))
        .map(|s| s.to_string());
    let authcode = param
        .get("s")
        .filter(|&s| validate_id(s))
        .map(|s| s.to_string());
    let grpid = param
        .get("x")
        .filter(|&s| validate_id(s))
        .map(|s| s.to_string());

    let grpname = if grpid.is_some() {
        if let Some(encoded_name) = param.get("g") {
            let encoded_name = encoded_name.replace('+', "%20"); // sometimes spaces are encoded as `+`
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
    let peerstate = Peerstate::from_fingerprint(context, &fingerprint)
        .await
        .context("Can't load peerstate")?;

    if let (Some(addr), Some(invitenumber), Some(authcode)) = (&addr, invitenumber, authcode) {
        let addr = ContactAddress::new(addr)?;
        let (contact_id, _) = Contact::add_or_lookup_ex(
            context,
            &name,
            &addr,
            &fingerprint.hex(),
            Origin::UnhandledSecurejoinQrScan,
        )
        .await
        .with_context(|| format!("failed to add or lookup contact for address {addr:?}"))?;

        if let (Some(grpid), Some(grpname)) = (grpid, grpname) {
            if context
                .is_self_addr(&addr)
                .await
                .with_context(|| format!("can't check if address {addr:?} is our address"))?
            {
                if token::exists(context, token::Namespace::InviteNumber, &invitenumber).await? {
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
        } else if context.is_self_addr(&addr).await? {
            if token::exists(context, token::Namespace::InviteNumber, &invitenumber).await? {
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
            let peerstate_addr = ContactAddress::new(&peerstate.addr)?;
            let (contact_id, _) =
                Contact::add_or_lookup(context, &name, &peerstate_addr, Origin::UnhandledQrScan)
                    .await
                    .context("add_or_lookup")?;
            ChatIdBlocked::get_for_contact(context, contact_id, Blocked::Request)
                .await
                .context("Failed to create (new) chat for contact")?;
            Ok(Qr::FprOk { contact_id })
        } else {
            let contact_id = Contact::lookup_id_by_addr(context, &addr, Origin::Unknown)
                .await
                .with_context(|| format!("Error looking up contact {addr:?}"))?;
            Ok(Qr::FprMismatch { contact_id })
        }
    } else {
        Ok(Qr::FprWithoutAddr {
            fingerprint: fingerprint.to_string(),
        })
    }
}

/// scheme: `https://i.delta.chat[/]#FINGERPRINT&a=ADDR[&OPTIONAL_PARAMS]`
async fn decode_ideltachat(context: &Context, prefix: &str, qr: &str) -> Result<Qr> {
    let qr = qr.replacen(prefix, OPENPGP4FPR_SCHEME, 1);
    let qr = qr.replacen('&', "#", 1);
    decode_openpgp(context, &qr)
        .await
        .with_context(|| format!("failed to decode {prefix} QR code"))
}

/// scheme: `DCACCOUNT:https://example.org/new_email?t=1w_7wDjgjelxeX884x96v3`
fn decode_account(qr: &str) -> Result<Qr> {
    let payload = qr
        .get(DCACCOUNT_SCHEME.len()..)
        .context("Invalid DCACCOUNT payload")?;
    let url = url::Url::parse(payload).context("Invalid account URL")?;
    if url.scheme() == "http" || url.scheme() == "https" {
        Ok(Qr::Account {
            domain: url
                .host_str()
                .context("can't extract account setup domain")?
                .to_string(),
        })
    } else {
        bail!("Bad scheme for account URL: {:?}.", url.scheme());
    }
}

/// scheme: `DCWEBRTC:https://meet.jit.si/$ROOM`
fn decode_webrtc_instance(_context: &Context, qr: &str) -> Result<Qr> {
    let payload = qr
        .get(DCWEBRTC_SCHEME.len()..)
        .context("Invalid DCWEBRTC payload")?;

    let (_type, url) = Message::parse_webrtc_instance(payload);
    let url = url::Url::parse(&url).context("Invalid WebRTC instance")?;

    if url.scheme() == "http" || url.scheme() == "https" {
        Ok(Qr::WebrtcInstance {
            domain: url
                .host_str()
                .context("can't extract WebRTC instance domain")?
                .to_string(),
            instance_pattern: payload.to_string(),
        })
    } else {
        bail!("Bad URL scheme for WebRTC instance: {:?}", url.scheme());
    }
}

/// scheme: `https://t.me/socks?server=foo&port=123` or `https://t.me/socks?server=1.2.3.4&port=123`
fn decode_tg_socks_proxy(_context: &Context, qr: &str) -> Result<Qr> {
    let url = url::Url::parse(qr).context("Invalid t.me/socks url")?;

    let mut host: Option<String> = None;
    let mut port: u16 = DEFAULT_SOCKS_PORT;
    let mut user: Option<String> = None;
    let mut pass: Option<String> = None;
    for (key, value) in url.query_pairs() {
        if key == "server" {
            host = Some(value.to_string());
        } else if key == "port" {
            port = value.parse().unwrap_or(DEFAULT_SOCKS_PORT);
        } else if key == "user" {
            user = Some(value.to_string());
        } else if key == "pass" {
            pass = Some(value.to_string());
        }
    }

    let Some(host) = host else {
        bail!("Bad t.me/socks url: {:?}", url);
    };

    let mut url = "socks5://".to_string();
    if let Some(pass) = pass {
        url += &percent_encode(user.unwrap_or_default().as_bytes(), NON_ALPHANUMERIC).to_string();
        url += ":";
        url += &percent_encode(pass.as_bytes(), NON_ALPHANUMERIC).to_string();
        url += "@";
    };
    url += &host;
    url += ":";
    url += &port.to_string();

    Ok(Qr::Proxy { url, host, port })
}

/// Decodes `ss://` URLs for Shadowsocks proxies.
fn decode_shadowsocks_proxy(qr: &str) -> Result<Qr> {
    let server_config = shadowsocks::config::ServerConfig::from_url(qr)?;
    let addr = server_config.addr();
    let host = addr.host().to_string();
    let port = addr.port();
    Ok(Qr::Proxy {
        url: qr.to_string(),
        host,
        port,
    })
}

/// Decodes a `DCBACKUP` QR code.
fn decode_backup2(qr: &str) -> Result<Qr> {
    let version_and_payload = qr
        .strip_prefix(DCBACKUP_SCHEME_PREFIX)
        .ok_or_else(|| anyhow!("Invalid DCBACKUP scheme"))?;
    let (version, payload) = version_and_payload
        .split_once(':')
        .context("DCBACKUP scheme separator missing")?;
    let version: i32 = version.parse().context("Not a valid number")?;
    if version > DCBACKUP_VERSION {
        return Ok(Qr::BackupTooNew {});
    }

    let (auth_token, node_addr) = payload
        .split_once('&')
        .context("Backup QR code has no separator")?;
    let auth_token = auth_token.to_string();
    let node_addr = serde_json::from_str::<iroh::NodeAddr>(node_addr)
        .context("Invalid node addr in backup QR code")?;

    Ok(Qr::Backup2 {
        node_addr,
        auth_token,
    })
}

#[derive(Debug, Deserialize)]
struct CreateAccountSuccessResponse {
    /// Email address.
    email: String,

    /// Password.
    password: String,
}
#[derive(Debug, Deserialize)]
struct CreateAccountErrorResponse {
    /// Reason for the failure to create account returned by the server.
    reason: String,
}

/// take a qr of the type DC_QR_ACCOUNT, parse it's parameters,
/// download additional information from the contained url and set the parameters.
/// on success, a configure::configure() should be able to log in to the account
pub(crate) async fn set_account_from_qr(context: &Context, qr: &str) -> Result<()> {
    let url_str = qr
        .get(DCACCOUNT_SCHEME.len()..)
        .context("Invalid DCACCOUNT scheme")?;

    if !url_str.starts_with(HTTPS_SCHEME) {
        bail!("DCACCOUNT QR codes must use HTTPS scheme");
    }

    let (response_text, response_success) = post_empty(context, url_str).await?;
    if response_success {
        let CreateAccountSuccessResponse { password, email } = serde_json::from_str(&response_text)
            .with_context(|| {
                format!("Cannot create account, response is malformed:\n{response_text:?}")
            })?;
        context
            .set_config_internal(Config::Addr, Some(&email))
            .await?;
        context
            .set_config_internal(Config::MailPw, Some(&password))
            .await?;

        Ok(())
    } else {
        match serde_json::from_str::<CreateAccountErrorResponse>(&response_text) {
            Ok(error) => Err(anyhow!(error.reason)),
            Err(parse_error) => {
                context.emit_event(EventType::Error(format!(
                    "Cannot create account, server response could not be parsed:\n{parse_error:#}\nraw response:\n{response_text}"
                )));
                bail!(
                    "Cannot create account, unexpected server response:\n{:?}",
                    response_text
                )
            }
        }
    }
}

/// Sets configuration values from a QR code.
pub async fn set_config_from_qr(context: &Context, qr: &str) -> Result<()> {
    match check_qr(context, qr).await? {
        Qr::Account { .. } => set_account_from_qr(context, qr).await?,
        Qr::WebrtcInstance {
            domain: _,
            instance_pattern,
        } => {
            context
                .set_config_internal(Config::WebrtcInstance, Some(&instance_pattern))
                .await?;
        }
        Qr::Proxy { url, .. } => {
            let old_proxy_url_value = context
                .get_config(Config::ProxyUrl)
                .await?
                .unwrap_or_default();

            // Normalize the URL.
            let url = ProxyConfig::from_url(&url)?.to_url();

            let proxy_urls: Vec<&str> = std::iter::once(url.as_str())
                .chain(
                    old_proxy_url_value
                        .split('\n')
                        .filter(|s| !s.is_empty() && *s != url),
                )
                .collect();
            context
                .set_config(Config::ProxyUrl, Some(&proxy_urls.join("\n")))
                .await?;
            context.set_config_bool(Config::ProxyEnabled, true).await?;
        }
        Qr::WithdrawVerifyContact {
            invitenumber,
            authcode,
            ..
        } => {
            token::delete(context, token::Namespace::InviteNumber, &invitenumber).await?;
            token::delete(context, token::Namespace::Auth, &authcode).await?;
            context
                .sync_qr_code_token_deletion(invitenumber, authcode)
                .await?;
        }
        Qr::WithdrawVerifyGroup {
            invitenumber,
            authcode,
            ..
        } => {
            token::delete(context, token::Namespace::InviteNumber, &invitenumber).await?;
            token::delete(context, token::Namespace::Auth, &authcode).await?;
            context
                .sync_qr_code_token_deletion(invitenumber, authcode)
                .await?;
        }
        Qr::ReviveVerifyContact {
            invitenumber,
            authcode,
            ..
        } => {
            token::save(context, token::Namespace::InviteNumber, None, &invitenumber).await?;
            token::save(context, token::Namespace::Auth, None, &authcode).await?;
            context.sync_qr_code_tokens(None).await?;
            context.scheduler.interrupt_inbox().await;
        }
        Qr::ReviveVerifyGroup {
            invitenumber,
            authcode,
            grpid,
            ..
        } => {
            token::save(
                context,
                token::Namespace::InviteNumber,
                Some(&grpid),
                &invitenumber,
            )
            .await?;
            token::save(context, token::Namespace::Auth, Some(&grpid), &authcode).await?;
            context.sync_qr_code_tokens(Some(&grpid)).await?;
            context.scheduler.interrupt_inbox().await;
        }
        Qr::Login { address, options } => {
            configure_from_login_qr(context, &address, options).await?
        }
        _ => bail!("QR code does not contain config"),
    }

    Ok(())
}

/// Extract address for the mailto scheme.
///
/// Scheme: `mailto:addr...?subject=...&body=..`
async fn decode_mailto(context: &Context, qr: &str) -> Result<Qr> {
    let payload = qr
        .get(MAILTO_SCHEME.len()..)
        .context("Invalid mailto: scheme")?;

    let (addr, query) = payload.split_once('?').unwrap_or((payload, ""));

    let param: BTreeMap<&str, &str> = query
        .split('&')
        .filter_map(|s| {
            if let [key, value] = s.splitn(2, '=').collect::<Vec<_>>()[..] {
                Some((key, value))
            } else {
                None
            }
        })
        .collect();

    let subject = if let Some(subject) = param.get("subject") {
        subject.to_string()
    } else {
        "".to_string()
    };
    let draft = if let Some(body) = param.get("body") {
        if subject.is_empty() {
            body.to_string()
        } else {
            subject + "\n" + body
        }
    } else {
        subject
    };
    let draft = draft.replace('+', "%20"); // sometimes spaces are encoded as `+`
    let draft = match percent_decode_str(&draft).decode_utf8() {
        Ok(decoded_draft) => decoded_draft.to_string(),
        Err(_err) => draft,
    };

    let addr = normalize_address(addr)?;
    let name = "";
    Qr::from_address(
        context,
        name,
        &addr,
        if draft.is_empty() { None } else { Some(draft) },
    )
    .await
}

/// Extract address for the smtp scheme.
///
/// Scheme: `SMTP:addr...:subject...:body...`
async fn decode_smtp(context: &Context, qr: &str) -> Result<Qr> {
    let payload = qr.get(SMTP_SCHEME.len()..).context("Invalid SMTP scheme")?;

    let (addr, _rest) = payload
        .split_once(':')
        .context("Invalid SMTP scheme payload")?;
    let addr = normalize_address(addr)?;
    let name = "";
    Qr::from_address(context, name, &addr, None).await
}

/// Extract address for the matmsg scheme.
///
/// Scheme: `MATMSG:TO:addr...;SUB:subject...;BODY:body...;`
///
/// There may or may not be linebreaks after the fields.
async fn decode_matmsg(context: &Context, qr: &str) -> Result<Qr> {
    // Does not work when the text `TO:` is used in subject/body _and_ TO: is not the first field.
    // we ignore this case.
    let addr = if let Some(to_index) = qr.find("TO:") {
        let addr = qr.get(to_index + 3..).unwrap_or_default().trim();
        if let Some(semi_index) = addr.find(';') {
            addr.get(..semi_index).unwrap_or_default().trim()
        } else {
            addr
        }
    } else {
        bail!("Invalid MATMSG found");
    };

    let addr = normalize_address(addr)?;
    let name = "";
    Qr::from_address(context, name, &addr, None).await
}

static VCARD_NAME_RE: Lazy<regex::Regex> =
    Lazy::new(|| regex::Regex::new(r"(?m)^N:([^;]*);([^;\n]*)").unwrap());
static VCARD_EMAIL_RE: Lazy<regex::Regex> =
    Lazy::new(|| regex::Regex::new(r"(?m)^EMAIL([^:\n]*):([^;\n]*)").unwrap());

/// Extract address for the vcard scheme.
///
/// Scheme: `VCARD:BEGIN\nN:last name;first name;...;\nEMAIL;<type>:addr...;`
async fn decode_vcard(context: &Context, qr: &str) -> Result<Qr> {
    let name = VCARD_NAME_RE
        .captures(qr)
        .and_then(|caps| {
            let last_name = caps.get(1)?.as_str().trim();
            let first_name = caps.get(2)?.as_str().trim();

            Some(format!("{first_name} {last_name}"))
        })
        .unwrap_or_default();

    let addr = if let Some(cap) = VCARD_EMAIL_RE.captures(qr).and_then(|caps| caps.get(2)) {
        normalize_address(cap.as_str().trim())?
    } else {
        bail!("Bad e-mail address");
    };

    Qr::from_address(context, &name, &addr, None).await
}

impl Qr {
    /// Creates a new scanned QR code of a contact address.
    ///
    /// May contain a message draft.
    pub async fn from_address(
        context: &Context,
        name: &str,
        addr: &str,
        draft: Option<String>,
    ) -> Result<Self> {
        let addr = ContactAddress::new(addr)?;
        let (contact_id, _) =
            Contact::add_or_lookup(context, name, &addr, Origin::UnhandledQrScan).await?;
        Ok(Qr::Addr { contact_id, draft })
    }
}

/// URL decodes a given address, does basic email validation on the result.
fn normalize_address(addr: &str) -> Result<String> {
    // urldecoding is needed at least for OPENPGP4FPR but should not hurt in the other cases
    let new_addr = percent_decode_str(addr).decode_utf8()?;
    let new_addr = addr_normalize(&new_addr);

    ensure!(may_be_valid_addr(&new_addr), "Bad e-mail address");

    Ok(new_addr.to_string())
}

#[cfg(test)]
mod qr_tests;
