//! End-to-end decryption support.

use std::collections::HashSet;
use std::str::FromStr;

use anyhow::Result;
use mailparse::ParsedMail;
use tracing::{info, warn};

use crate::aheader::Aheader;
use crate::authres::handle_authres;
use crate::authres::{self, DkimResults};
use crate::contact::addr_cmp;
use crate::context::Context;
use crate::headerdef::{HeaderDef, HeaderDefMap};
use crate::key::{DcKey, Fingerprint, SignedPublicKey, SignedSecretKey};
use crate::keyring::Keyring;
use crate::peerstate::Peerstate;
use crate::pgp;

/// Tries to decrypt a message, but only if it is structured as an Autocrypt message.
///
/// If successful and the message is encrypted, returns decrypted body and a set of valid
/// signature fingerprints.
///
/// If the message is wrongly signed, HashSet will be empty.
pub fn try_decrypt(
    mail: &ParsedMail<'_>,
    private_keyring: &Keyring<SignedSecretKey>,
    public_keyring_for_validate: &Keyring<SignedPublicKey>,
) -> Result<Option<(Vec<u8>, HashSet<Fingerprint>)>> {
    let encrypted_data_part = match get_autocrypt_mime(mail)
        .or_else(|| get_mixed_up_mime(mail))
        .or_else(|| get_attachment_mime(mail))
    {
        None => return Ok(None),
        Some(res) => res,
    };
    info!("Detected Autocrypt-mime message");

    decrypt_part(
        encrypted_data_part,
        private_keyring,
        public_keyring_for_validate,
    )
}

pub(crate) async fn prepare_decryption(
    context: &Context,
    mail: &ParsedMail<'_>,
    from: &str,
    message_time: i64,
) -> Result<DecryptionInfo> {
    if mail.headers.get_header(HeaderDef::ListPost).is_some() {
        if mail.headers.get_header(HeaderDef::Autocrypt).is_some() {
            info!(
                "Ignoring autocrypt header since this is a mailing list message. \
                NOTE: For privacy reasons, the mailing list software should remove Autocrypt headers."
            );
        }
        return Ok(DecryptionInfo {
            from: from.to_string(),
            autocrypt_header: None,
            peerstate: None,
            message_time,
            dkim_results: DkimResults {
                dkim_passed: false,
                dkim_should_work: false,
                allow_keychange: true,
            },
        });
    }

    let autocrypt_header =
        if let Some(autocrypt_header_value) = mail.headers.get_header_value(HeaderDef::Autocrypt) {
            match Aheader::from_str(&autocrypt_header_value) {
                Ok(header) if addr_cmp(&header.addr, from) => Some(header),
                Ok(header) => {
                    warn!(
                        "Autocrypt header address {:?} is not {:?}.",
                        header.addr, from
                    );
                    None
                }
                Err(err) => {
                    warn!("Failed to parse Autocrypt header: {:#}.", err);
                    None
                }
            }
        } else {
            None
        };

    let dkim_results = handle_authres(context, mail, from, message_time).await?;

    let peerstate = get_autocrypt_peerstate(
        context,
        from,
        autocrypt_header.as_ref(),
        message_time,
        // Disallowing keychanges is disabled for now:
        true, // dkim_results.allow_keychange,
    )
    .await?;

    Ok(DecryptionInfo {
        from: from.to_string(),
        autocrypt_header,
        peerstate,
        message_time,
        dkim_results,
    })
}

#[derive(Debug)]
pub struct DecryptionInfo {
    /// The From address. This is the address from the unnencrypted, outer
    /// From header.
    pub from: String,
    pub autocrypt_header: Option<Aheader>,
    /// The peerstate that will be used to validate the signatures
    pub peerstate: Option<Peerstate>,
    /// The timestamp when the message was sent.
    /// If this is older than the peerstate's last_seen, this probably
    /// means out-of-order message arrival, We don't modify the
    /// peerstate in this case.
    pub message_time: i64,
    pub(crate) dkim_results: authres::DkimResults,
}

/// Returns a reference to the encrypted payload of a ["Mixed
/// Up"][pgpmime-message-mangling] message.
///
/// According to [RFC 3156] encrypted messages should have
/// `multipart/encrypted` MIME type and two parts, but Microsoft
/// Exchange and ProtonMail IMAP/SMTP Bridge are known to mangle this
/// structure by changing the type to `multipart/mixed` and prepending
/// an empty part at the start.
///
/// ProtonMail IMAP/SMTP Bridge prepends a part literally saying
/// "Empty Message", so we don't check its contents at all, checking
/// only for `text/plain` type.
///
/// Returns `None` if the message is not a "Mixed Up" message.
///
/// [RFC 3156]: https://www.rfc-editor.org/info/rfc3156
/// [pgpmime-message-mangling]: https://tools.ietf.org/id/draft-dkg-openpgp-pgpmime-message-mangling-00.html
fn get_mixed_up_mime<'a, 'b>(mail: &'a ParsedMail<'b>) -> Option<&'a ParsedMail<'b>> {
    if mail.ctype.mimetype != "multipart/mixed" {
        return None;
    }
    if let [first_part, second_part, third_part] = &mail.subparts[..] {
        if first_part.ctype.mimetype == "text/plain"
            && second_part.ctype.mimetype == "application/pgp-encrypted"
            && third_part.ctype.mimetype == "application/octet-stream"
        {
            Some(third_part)
        } else {
            None
        }
    } else {
        None
    }
}

/// Returns a reference to the encrypted payload of a message turned into attachment.
///
/// Google Workspace has an option "Append footer" which appends standard footer defined
/// by administrator to all outgoing messages. However, there is no plain text part in
/// encrypted messages sent by Delta Chat, so Google Workspace turns the message into
/// multipart/mixed MIME, where the first part is an empty plaintext part with a footer
/// and the second part is the original encrypted message.
fn get_attachment_mime<'a, 'b>(mail: &'a ParsedMail<'b>) -> Option<&'a ParsedMail<'b>> {
    if mail.ctype.mimetype != "multipart/mixed" {
        return None;
    }
    if let [first_part, second_part] = &mail.subparts[..] {
        if first_part.ctype.mimetype == "text/plain"
            && second_part.ctype.mimetype == "multipart/encrypted"
        {
            get_autocrypt_mime(second_part)
        } else {
            None
        }
    } else {
        None
    }
}

/// Returns a reference to the encrypted payload of a valid PGP/MIME message.
///
/// Returns `None` if the message is not a valid PGP/MIME message.
fn get_autocrypt_mime<'a, 'b>(mail: &'a ParsedMail<'b>) -> Option<&'a ParsedMail<'b>> {
    if mail.ctype.mimetype != "multipart/encrypted" {
        return None;
    }
    if let [first_part, second_part] = &mail.subparts[..] {
        if first_part.ctype.mimetype == "application/pgp-encrypted"
            && second_part.ctype.mimetype == "application/octet-stream"
        {
            Some(second_part)
        } else {
            None
        }
    } else {
        None
    }
}

/// Returns Ok(None) if nothing encrypted was found.
fn decrypt_part(
    mail: &ParsedMail<'_>,
    private_keyring: &Keyring<SignedSecretKey>,
    public_keyring_for_validate: &Keyring<SignedPublicKey>,
) -> Result<Option<(Vec<u8>, HashSet<Fingerprint>)>> {
    let data = mail.get_body_raw()?;

    if has_decrypted_pgp_armor(&data) {
        let (plain, ret_valid_signatures) =
            pgp::pk_decrypt(data, private_keyring, public_keyring_for_validate)?;
        return Ok(Some((plain, ret_valid_signatures)));
    }

    Ok(None)
}

#[allow(clippy::indexing_slicing)]
fn has_decrypted_pgp_armor(input: &[u8]) -> bool {
    if let Some(index) = input.iter().position(|b| *b > b' ') {
        if input.len() - index > 26 {
            let start = index;
            let end = start + 27;

            return &input[start..end] == b"-----BEGIN PGP MESSAGE-----";
        }
    }

    false
}

/// Validates signatures of Multipart/Signed message part, as defined in RFC 1847.
///
/// Returns the signed part and the set of key
/// fingerprints for which there is a valid signature.
///
/// Returns None if the message is not Multipart/Signed or doesn't contain necessary parts.
pub(crate) fn validate_detached_signature<'a, 'b>(
    mail: &'a ParsedMail<'b>,
    public_keyring_for_validate: &Keyring<SignedPublicKey>,
) -> Option<(&'a ParsedMail<'b>, HashSet<Fingerprint>)> {
    if mail.ctype.mimetype != "multipart/signed" {
        return None;
    }

    if let [first_part, second_part] = &mail.subparts[..] {
        // First part is the content, second part is the signature.
        let content = first_part.raw_bytes;
        let ret_valid_signatures = match second_part.get_body_raw() {
            Ok(signature) => pgp::pk_validate(content, &signature, public_keyring_for_validate)
                .unwrap_or_default(),
            Err(_) => Default::default(),
        };
        Some((first_part, ret_valid_signatures))
    } else {
        None
    }
}

pub(crate) fn keyring_from_peerstate(peerstate: Option<&Peerstate>) -> Keyring<SignedPublicKey> {
    let mut public_keyring_for_validate: Keyring<SignedPublicKey> = Keyring::new();
    if let Some(peerstate) = peerstate {
        if let Some(key) = &peerstate.public_key {
            public_keyring_for_validate.add(key.clone());
        } else if let Some(key) = &peerstate.gossip_key {
            public_keyring_for_validate.add(key.clone());
        }
    }
    public_keyring_for_validate
}

/// Applies Autocrypt header to Autocrypt peer state and saves it into the database.
///
/// If we already know this fingerprint from another contact's peerstate, return that
/// peerstate in order to make AEAP work, but don't save it into the db yet.
///
/// The param `allow_change` is used to prevent the autocrypt key from being changed
/// if we suspect that the message may be forged and have a spoofed sender identity.
///
/// Returns updated peerstate.
pub(crate) async fn get_autocrypt_peerstate(
    context: &Context,
    from: &str,
    autocrypt_header: Option<&Aheader>,
    message_time: i64,
    allow_change: bool,
) -> Result<Option<Peerstate>> {
    let mut peerstate;

    // Apply Autocrypt header
    if let Some(header) = autocrypt_header {
        // The "from_verified_fingerprint" part is for AEAP:
        // If we know this fingerprint from another addr,
        // we may want to do a transition from this other addr
        // (and keep its peerstate)
        // For security reasons, for now, we only do a transition
        // if the fingerprint is verified.
        peerstate = Peerstate::from_verified_fingerprint_or_addr(
            context,
            &header.public_key.fingerprint(),
            from,
        )
        .await?;

        if let Some(ref mut peerstate) = peerstate {
            if addr_cmp(&peerstate.addr, from) {
                if allow_change {
                    peerstate.apply_header(header, message_time);
                    peerstate.save_to_db(&context.sql).await?;
                } else {
                    info!(
                        "Refusing to update existing peerstate of {}",
                        &peerstate.addr
                    );
                }
            }
            // If `peerstate.addr` and `from` differ, this means that
            // someone is using the same key but a different addr, probably
            // because they made an AEAP transition.
            // But we don't know if that's legit until we checked the
            // signatures, so wait until then with writing anything
            // to the database.
        } else {
            let p = Peerstate::from_header(header, message_time);
            p.save_to_db(&context.sql).await?;
            peerstate = Some(p);
        }
    } else {
        peerstate = Peerstate::from_addr(context, from).await?;
    }

    Ok(peerstate)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::receive_imf::receive_imf;
    use crate::test_utils::TestContext;

    #[test]
    fn test_has_decrypted_pgp_armor() {
        let data = b" -----BEGIN PGP MESSAGE-----";
        assert_eq!(has_decrypted_pgp_armor(data), true);

        let data = b"    \n-----BEGIN PGP MESSAGE-----";
        assert_eq!(has_decrypted_pgp_armor(data), true);

        let data = b"    -----BEGIN PGP MESSAGE---";
        assert_eq!(has_decrypted_pgp_armor(data), false);

        let data = b" -----BEGIN PGP MESSAGE-----";
        assert_eq!(has_decrypted_pgp_armor(data), true);

        let data = b"blas";
        assert_eq!(has_decrypted_pgp_armor(data), false);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_mixed_up_mime() -> Result<()> {
        // "Mixed Up" mail as received when sending an encrypted
        // message using Delta Chat Desktop via ProtonMail IMAP/SMTP
        // Bridge.
        let mixed_up_mime = include_bytes!("../test-data/message/protonmail-mixed-up.eml");
        let mail = mailparse::parse_mail(mixed_up_mime)?;
        assert!(get_autocrypt_mime(&mail).is_none());
        assert!(get_mixed_up_mime(&mail).is_some());
        assert!(get_attachment_mime(&mail).is_none());

        // Same "Mixed Up" mail repaired by Thunderbird 78.9.0.
        //
        // It added `X-Enigmail-Info: Fixed broken PGP/MIME message`
        // header although the repairing is done by the built-in
        // OpenPGP support, not Enigmail.
        let repaired_mime = include_bytes!("../test-data/message/protonmail-repaired.eml");
        let mail = mailparse::parse_mail(repaired_mime)?;
        assert!(get_autocrypt_mime(&mail).is_some());
        assert!(get_mixed_up_mime(&mail).is_none());
        assert!(get_attachment_mime(&mail).is_none());

        // Another form of "Mixed Up" mail created by Google Workspace,
        // where original message is turned into attachment to empty plaintext message.
        let attachment_mime = include_bytes!("../test-data/message/google-workspace-mixed-up.eml");
        let mail = mailparse::parse_mail(attachment_mime)?;
        assert!(get_autocrypt_mime(&mail).is_none());
        assert!(get_mixed_up_mime(&mail).is_none());
        assert!(get_attachment_mime(&mail).is_some());

        let bob = TestContext::new_bob().await;
        receive_imf(&bob, attachment_mime, false).await?;
        let msg = bob.get_last_msg().await;
        assert_eq!(msg.text.as_deref(), Some("Hello from Thunderbird!"));

        Ok(())
    }
}
