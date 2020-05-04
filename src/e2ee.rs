//! End-to-end encryption support.

use std::collections::HashSet;

use mailparse::ParsedMail;
use num_traits::FromPrimitive;

use crate::aheader::*;
use crate::config::Config;
use crate::context::Context;
use crate::error::*;
use crate::headerdef::{HeaderDef, HeaderDefMap};
use crate::key::{DcKey, Key, SignedPublicKey, SignedSecretKey};
use crate::keyring::*;
use crate::peerstate::*;
use crate::pgp;
use crate::securejoin::handle_degrade_event;

#[derive(Debug)]
pub struct EncryptHelper {
    pub prefer_encrypt: EncryptPreference,
    pub addr: String,
    pub public_key: SignedPublicKey,
}

impl EncryptHelper {
    pub fn new(context: &Context) -> Result<EncryptHelper> {
        let prefer_encrypt =
            EncryptPreference::from_i32(context.get_config_int(Config::E2eeEnabled))
                .unwrap_or_default();
        let addr = match context.get_config(Config::ConfiguredAddr) {
            None => {
                bail!("addr not configured!");
            }
            Some(addr) => addr,
        };

        let public_key = SignedPublicKey::load_self(context)?;

        Ok(EncryptHelper {
            prefer_encrypt,
            addr,
            public_key,
        })
    }

    pub fn get_aheader(&self) -> Aheader {
        let pk = self.public_key.clone();
        let addr = self.addr.to_string();
        Aheader::new(addr, pk, self.prefer_encrypt)
    }

    /// Determines if we can and should encrypt.
    pub fn should_encrypt(
        &self,
        context: &Context,
        e2ee_guaranteed: bool,
        peerstates: &[(Option<Peerstate>, &str)],
    ) -> Result<bool> {
        if !(self.prefer_encrypt == EncryptPreference::Mutual || e2ee_guaranteed) {
            return Ok(false);
        }

        for (peerstate, addr) in peerstates {
            match peerstate {
                Some(peerstate) => {
                    if peerstate.prefer_encrypt != EncryptPreference::Mutual && !e2ee_guaranteed {
                        info!(context, "peerstate for {:?} is no-encrypt", addr);
                        return Ok(false);
                    }
                }
                None => {
                    let msg = format!("peerstate for {:?} missing, cannot encrypt", addr);
                    if e2ee_guaranteed {
                        return Err(format_err!("{}", msg));
                    } else {
                        info!(context, "{}", msg);
                        return Ok(false);
                    }
                }
            }
        }

        Ok(true)
    }

    /// Tries to encrypt the passed in `mail`.
    pub fn encrypt(
        &mut self,
        context: &Context,
        min_verified: PeerstateVerifiedStatus,
        mail_to_encrypt: lettre_email::PartBuilder,
        peerstates: &[(Option<Peerstate>, &str)],
    ) -> Result<String> {
        let mut keyring = Keyring::default();

        for (peerstate, addr) in peerstates
            .iter()
            .filter_map(|(state, addr)| state.as_ref().map(|s| (s, addr)))
        {
            let key = peerstate.peek_key(min_verified).ok_or_else(|| {
                format_err!("proper enc-key for {} missing, cannot encrypt", addr)
            })?;
            keyring.add_ref(key);
        }
        let public_key = Key::from(self.public_key.clone());
        keyring.add_ref(&public_key);
        let sign_key = Key::from(SignedSecretKey::load_self(context)?);

        let raw_message = mail_to_encrypt.build().as_string().into_bytes();

        let ctext = pgp::pk_encrypt(&raw_message, &keyring, Some(&sign_key))?;

        Ok(ctext)
    }
}

pub fn try_decrypt(
    context: &Context,
    mail: &ParsedMail<'_>,
    message_time: i64,
) -> Result<(Option<Vec<u8>>, HashSet<String>)> {
    let from = mail
        .headers
        .iter()
        .filter(|header| {
            header
                .get_key()
                .eq_ignore_ascii_case(HeaderDef::From_.get_headername())
        })
        .next()
        .and_then(|from_addr| mailparse::addrparse_header(&from_addr).ok())
        .and_then(|from| from.extract_single_info())
        .map(|from| from.addr)
        .unwrap_or_default();

    let mut peerstate = None;
    let autocryptheader = Aheader::from_headers(context, &from, &mail.headers);

    if message_time > 0 {
        peerstate = Peerstate::from_addr(context, &context.sql, &from);

        if let Some(ref mut peerstate) = peerstate {
            if let Some(ref header) = autocryptheader {
                peerstate.apply_header(&header, message_time);
                peerstate.save_to_db(&context.sql, false)?;
            } else if message_time > peerstate.last_seen_autocrypt && !contains_report(mail) {
                peerstate.degrade_encryption(message_time);
                peerstate.save_to_db(&context.sql, false)?;
            }
        } else if let Some(ref header) = autocryptheader {
            let p = Peerstate::from_header(context, header, message_time);
            p.save_to_db(&context.sql, true)?;
            peerstate = Some(p);
        }
    }

    /* possibly perform decryption */
    let mut private_keyring = Keyring::default();
    let mut public_keyring_for_validate = Keyring::default();
    let mut out_mail = None;
    let mut signatures = HashSet::default();
    let self_addr = context.get_config(Config::ConfiguredAddr);

    if let Some(self_addr) = self_addr {
        if private_keyring.load_self_private_for_decrypting(context, self_addr, &context.sql) {
            if peerstate.as_ref().map(|p| p.last_seen).unwrap_or_else(|| 0) == 0 {
                peerstate = Peerstate::from_addr(&context, &context.sql, &from);
            }
            if let Some(ref peerstate) = peerstate {
                if peerstate.degrade_event.is_some() {
                    handle_degrade_event(context, &peerstate)?;
                }
                if let Some(ref key) = peerstate.gossip_key {
                    public_keyring_for_validate.add_ref(key);
                }
                if let Some(ref key) = peerstate.public_key {
                    public_keyring_for_validate.add_ref(key);
                }
            }

            out_mail = decrypt_if_autocrypt_message(
                context,
                mail,
                &private_keyring,
                &public_keyring_for_validate,
                &mut signatures,
            )?;
        }
    }
    Ok((out_mail, signatures))
}

/// Returns a reference to the encrypted payload and validates the autocrypt structure.
fn get_autocrypt_mime<'a, 'b>(mail: &'a ParsedMail<'b>) -> Result<&'a ParsedMail<'b>> {
    ensure!(
        mail.ctype.mimetype == "multipart/encrypted",
        "Not a multipart/encrypted message: {}",
        mail.ctype.mimetype
    );
    ensure!(
        mail.subparts.len() == 2,
        "Invalid Autocrypt Level 1 Mime Parts"
    );

    ensure!(
        mail.subparts[0].ctype.mimetype == "application/pgp-encrypted",
        "Invalid Autocrypt Level 1 version part: {:?}",
        mail.subparts[0].ctype,
    );

    ensure!(
        mail.subparts[1].ctype.mimetype == "application/octet-stream",
        "Invalid Autocrypt Level 1 encrypted part: {:?}",
        mail.subparts[1].ctype
    );

    Ok(&mail.subparts[1])
}

fn decrypt_if_autocrypt_message<'a>(
    context: &Context,
    mail: &ParsedMail<'a>,
    private_keyring: &Keyring,
    public_keyring_for_validate: &Keyring,
    ret_valid_signatures: &mut HashSet<String>,
) -> Result<Option<Vec<u8>>> {
    //  The returned bool is true if we detected an Autocrypt-encrypted
    // message and successfully decrypted it. Decryption then modifies the
    // passed in mime structure in place. The returned bool is false
    // if it was not an Autocrypt message.
    //
    // Errors are returned for failures related to decryption of AC-messages.

    let encrypted_data_part = match get_autocrypt_mime(mail) {
        Err(_) => {
            // not an autocrypt mime message, abort and ignore
            return Ok(None);
        }
        Ok(res) => res,
    };
    info!(context, "Detected Autocrypt-mime message");

    decrypt_part(
        context,
        encrypted_data_part,
        private_keyring,
        public_keyring_for_validate,
        ret_valid_signatures,
    )
}

/// Returns Ok(None) if nothing encrypted was found.
fn decrypt_part(
    _context: &Context,
    mail: &ParsedMail<'_>,
    private_keyring: &Keyring,
    public_keyring_for_validate: &Keyring,
    ret_valid_signatures: &mut HashSet<String>,
) -> Result<Option<Vec<u8>>> {
    let data = mail.get_body_raw()?;

    if has_decrypted_pgp_armor(&data) {
        // we should only have one decryption happening
        ensure!(ret_valid_signatures.is_empty(), "corrupt signatures");

        let plain = pgp::pk_decrypt(
            &data,
            &private_keyring,
            &public_keyring_for_validate,
            Some(ret_valid_signatures),
        )?;

        ensure!(!ret_valid_signatures.is_empty(), "no valid signatures");
        return Ok(Some(plain));
    }

    Ok(None)
}

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

/// Check if a MIME structure contains a multipart/report part.
///
/// As reports are often unencrypted, we do not reset the Autocrypt header in
/// this case.
///
/// However, Delta Chat itself has no problem with encrypted multipart/report
/// parts and MUAs should be encouraged to encrpyt multipart/reports as well so
/// that we could use the normal Autocrypt processing.
fn contains_report(mail: &ParsedMail<'_>) -> bool {
    mail.ctype.mimetype == "multipart/report"
}

/// Ensures a private key exists for the configured user.
///
/// Normally the private key is generated when the first message is
/// sent but in a few locations there are no such guarantees,
/// e.g. when exporting keys, and calling this function ensures a
/// private key will be present.
///
/// If this succeeds you are also guaranteed that the
/// [Config::ConfiguredAddr] is configured, this address is returned.
// TODO, remove this once deltachat::key::Key no longer exists.
pub fn ensure_secret_key_exists(context: &Context) -> Result<String> {
    let self_addr = context.get_config(Config::ConfiguredAddr).ok_or_else(|| {
        format_err!(concat!(
            "Failed to get self address, ",
            "cannot ensure secret key if not configured."
        ))
    })?;
    SignedPublicKey::load_self(context)?;
    Ok(self_addr)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::test_utils::*;

    mod ensure_secret_key_exists {
        use super::*;

        #[test]
        fn test_prexisting() {
            let t = dummy_context();
            let test_addr = configure_alice_keypair(&t.ctx);
            assert_eq!(ensure_secret_key_exists(&t.ctx).unwrap(), test_addr);
        }

        #[test]
        fn test_not_configured() {
            let t = dummy_context();
            assert!(ensure_secret_key_exists(&t.ctx).is_err());
        }
    }

    #[test]
    fn test_mailmime_parse() {
        let plain = b"Chat-Disposition-Notification-To: hello@world.de
Chat-Group-ID: CovhGgau8M-
Chat-Group-Name: Delta Chat Dev
Subject: =?utf-8?Q?Chat=3A?= Delta Chat =?utf-8?Q?Dev=3A?= sidenote for
 =?utf-8?Q?all=3A?= rust core master ...
Content-Type: text/plain; charset=\"utf-8\"; protected-headers=\"v1\"
Content-Transfer-Encoding: quoted-printable

sidenote for all: things are trick atm recomm=
end not to try to run with desktop or ios unless you are ready to hunt bugs

-- =20
Sent with my Delta Chat Messenger: https://delta.chat";
        let mail = mailparse::parse_mail(plain).expect("failed to parse valid message");

        assert_eq!(mail.headers.len(), 6);
        assert!(
            mail.get_body().unwrap().starts_with(
                "sidenote for all: things are trick atm recommend not to try to run with desktop or ios unless you are ready to hunt bugs")
        );
    }

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
}
