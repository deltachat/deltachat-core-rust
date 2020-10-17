//! End-to-end encryption support.

use std::collections::HashSet;

use mailparse::ParsedMail;
use num_traits::FromPrimitive;

use crate::aheader::*;
use crate::config::Config;
use crate::context::Context;
use crate::error::*;
use crate::headerdef::HeaderDef;
use crate::headerdef::HeaderDefMap;
use crate::key::{DcKey, Fingerprint, SignedPublicKey, SignedSecretKey};
use crate::keyring::*;
use crate::peerstate::*;
use crate::pgp;

#[derive(Debug)]
pub struct EncryptHelper {
    pub prefer_encrypt: EncryptPreference,
    pub addr: String,
    pub public_key: SignedPublicKey,
}

impl EncryptHelper {
    pub async fn new(context: &Context) -> Result<EncryptHelper> {
        let prefer_encrypt =
            EncryptPreference::from_i32(context.get_config_int(Config::E2eeEnabled).await)
                .unwrap_or_default();
        let addr = match context.get_config(Config::ConfiguredAddr).await {
            None => {
                bail!("addr not configured!");
            }
            Some(addr) => addr,
        };

        let public_key = SignedPublicKey::load_self(context).await?;

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
    ///
    /// For encryption to be enabled, `e2ee_guaranteed` should be true, or strictly more than a half
    /// of peerstates should prefer encryption. Own preference is counted equally to peer
    /// preferences, even if message copy is not sent to self.
    ///
    /// `e2ee_guaranteed` should be set to true for replies to encrypted messages (as required by
    /// Autocrypt Level 1, version 1.1) and for messages sent in protected groups.
    ///
    /// Returns an error if `e2ee_guaranteed` is true, but one or more keys are missing.
    pub fn should_encrypt(
        &self,
        context: &Context,
        e2ee_guaranteed: bool,
        peerstates: &[(Option<Peerstate>, &str)],
    ) -> Result<bool> {
        let mut prefer_encrypt_count = if self.prefer_encrypt == EncryptPreference::Mutual {
            1
        } else {
            0
        };
        for (peerstate, addr) in peerstates {
            match peerstate {
                Some(peerstate) => {
                    info!(
                        context,
                        "peerstate for {:?} is {}", addr, peerstate.prefer_encrypt
                    );
                    match peerstate.prefer_encrypt {
                        EncryptPreference::NoPreference => {}
                        EncryptPreference::Mutual => prefer_encrypt_count += 1,
                        EncryptPreference::Reset => {
                            if !e2ee_guaranteed {
                                return Ok(false);
                            }
                        }
                    };
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

        // Count number of recipients, including self.
        // This does not depend on whether we send a copy to self or not.
        let recipients_count = peerstates.len() + 1;

        Ok(e2ee_guaranteed || 2 * prefer_encrypt_count > recipients_count)
    }

    /// Tries to encrypt the passed in `mail`.
    pub async fn encrypt(
        self,
        context: &Context,
        min_verified: PeerstateVerifiedStatus,
        mail_to_encrypt: lettre_email::PartBuilder,
        peerstates: Vec<(Option<Peerstate<'_>>, &str)>,
    ) -> Result<String> {
        let mut keyring: Keyring<SignedPublicKey> = Keyring::new();

        for (peerstate, addr) in peerstates
            .into_iter()
            .filter_map(|(state, addr)| state.map(|s| (s, addr)))
        {
            let key = peerstate.take_key(min_verified).ok_or_else(|| {
                format_err!("proper enc-key for {} missing, cannot encrypt", addr)
            })?;
            keyring.add(key);
        }
        keyring.add(self.public_key.clone());
        let sign_key = SignedSecretKey::load_self(context).await?;

        let raw_message = mail_to_encrypt.build().as_string().into_bytes();

        let ctext = pgp::pk_encrypt(&raw_message, keyring, Some(sign_key)).await?;

        Ok(ctext)
    }
}

/// Tries to decrypt a message, but only if it is structured as an
/// Autocrypt message.
///
/// Returns decrypted body and a set of valid signature fingerprints
/// if successful.
///
/// If the message is wrongly signed, this will still return the decrypted
/// message but the HashSet will be empty.
pub async fn try_decrypt(
    context: &Context,
    mail: &ParsedMail<'_>,
    message_time: i64,
) -> Result<(Option<Vec<u8>>, HashSet<Fingerprint>)> {
    let from = mail
        .headers
        .get_header(HeaderDef::From_)
        .and_then(|from_addr| mailparse::addrparse_header(&from_addr).ok())
        .and_then(|from| from.extract_single_info())
        .map(|from| from.addr)
        .unwrap_or_default();

    let mut peerstate = Peerstate::from_addr(context, &from).await?;

    // Apply Autocrypt header
    if let Some(ref header) = Aheader::from_headers(context, &from, &mail.headers) {
        if let Some(ref mut peerstate) = peerstate {
            peerstate.apply_header(&header, message_time);
            peerstate.save_to_db(&context.sql, false).await?;
        } else {
            let p = Peerstate::from_header(context, header, message_time);
            p.save_to_db(&context.sql, true).await?;
            peerstate = Some(p);
        }
    }

    // Possibly perform decryption
    let private_keyring: Keyring<SignedSecretKey> = Keyring::new_self(context).await?;
    let mut public_keyring_for_validate: Keyring<SignedPublicKey> = Keyring::new();
    let mut signatures = HashSet::default();

    if let Some(ref mut peerstate) = peerstate {
        peerstate.handle_fingerprint_change(context).await?;
        if let Some(key) = &peerstate.public_key {
            public_keyring_for_validate.add(key.clone());
        } else if let Some(key) = &peerstate.gossip_key {
            public_keyring_for_validate.add(key.clone());
        }
    }

    let out_mail = decrypt_if_autocrypt_message(
        context,
        mail,
        private_keyring,
        public_keyring_for_validate,
        &mut signatures,
    )
    .await?;

    if let Some(mut peerstate) = peerstate {
        // If message is not encrypted and it is not a read receipt, degrade encryption.
        if out_mail.is_none()
            && message_time > peerstate.last_seen_autocrypt
            && !contains_report(mail)
        {
            peerstate.degrade_encryption(message_time);
            peerstate.save_to_db(&context.sql, false).await?;
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
    if let [first_part, second_part] = &mail.subparts[..] {
        ensure!(
            first_part.ctype.mimetype == "application/pgp-encrypted",
            "Invalid Autocrypt Level 1 version part: {:?}",
            first_part.ctype,
        );

        ensure!(
            second_part.ctype.mimetype == "application/octet-stream",
            "Invalid Autocrypt Level 1 encrypted part: {:?}",
            second_part.ctype
        );

        Ok(second_part)
    } else {
        bail!("Invalid Autocrypt Level 1 Mime Parts")
    }
}

async fn decrypt_if_autocrypt_message(
    context: &Context,
    mail: &ParsedMail<'_>,
    private_keyring: Keyring<SignedSecretKey>,
    public_keyring_for_validate: Keyring<SignedPublicKey>,
    ret_valid_signatures: &mut HashSet<Fingerprint>,
) -> Result<Option<Vec<u8>>> {
    let encrypted_data_part = match get_autocrypt_mime(mail) {
        Err(_) => {
            // not an autocrypt mime message, abort and ignore
            return Ok(None);
        }
        Ok(res) => res,
    };
    info!(context, "Detected Autocrypt-mime message");

    decrypt_part(
        encrypted_data_part,
        private_keyring,
        public_keyring_for_validate,
        ret_valid_signatures,
    )
    .await
}

/// Returns Ok(None) if nothing encrypted was found.
async fn decrypt_part(
    mail: &ParsedMail<'_>,
    private_keyring: Keyring<SignedSecretKey>,
    public_keyring_for_validate: Keyring<SignedPublicKey>,
    ret_valid_signatures: &mut HashSet<Fingerprint>,
) -> Result<Option<Vec<u8>>> {
    let data = mail.get_body_raw()?;

    if has_decrypted_pgp_armor(&data) {
        // we should only have one decryption happening
        ensure!(ret_valid_signatures.is_empty(), "corrupt signatures");

        let plain = pgp::pk_decrypt(
            data,
            private_keyring,
            public_keyring_for_validate,
            Some(ret_valid_signatures),
        )
        .await?;

        // If the message was wrongly or not signed, still return the plain text.
        // The caller has to check the signatures then.

        return Ok(Some(plain));
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
pub async fn ensure_secret_key_exists(context: &Context) -> Result<String> {
    let self_addr = context
        .get_config(Config::ConfiguredAddr)
        .await
        .ok_or_else(|| {
            format_err!(concat!(
                "Failed to get self address, ",
                "cannot ensure secret key if not configured."
            ))
        })?;
    SignedPublicKey::load_self(context).await?;
    Ok(self_addr)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::chat;
    use crate::constants::Viewtype;
    use crate::contact::{Contact, Origin};
    use crate::message::Message;
    use crate::param::Param;
    use crate::test_utils::*;

    mod ensure_secret_key_exists {
        use super::*;

        #[async_std::test]
        async fn test_prexisting() {
            let t = TestContext::new().await;
            let test_addr = t.configure_alice().await;
            assert_eq!(ensure_secret_key_exists(&t.ctx).await.unwrap(), test_addr);
        }

        #[async_std::test]
        async fn test_not_configured() {
            let t = TestContext::new().await;
            assert!(ensure_secret_key_exists(&t.ctx).await.is_err());
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

    #[async_std::test]
    async fn test_encrypted_no_autocrypt() -> crate::error::Result<()> {
        let alice = TestContext::new_alice().await;
        let bob = TestContext::new_bob().await;

        let (contact_alice_id, _modified) = Contact::add_or_lookup(
            &bob.ctx,
            "Alice",
            "alice@example.com",
            Origin::ManuallyCreated,
        )
        .await?;
        let (contact_bob_id, _modified) = Contact::add_or_lookup(
            &alice.ctx,
            "Bob",
            "bob@example.net",
            Origin::ManuallyCreated,
        )
        .await?;

        let chat_alice = chat::create_by_contact_id(&alice.ctx, contact_bob_id).await?;
        let chat_bob = chat::create_by_contact_id(&bob.ctx, contact_alice_id).await?;

        // Alice sends unencrypted message to Bob
        let mut msg = Message::new(Viewtype::Text);
        chat::prepare_msg(&alice.ctx, chat_alice, &mut msg).await?;
        chat::send_msg(&alice.ctx, chat_alice, &mut msg).await?;
        let sent = alice.pop_sent_msg().await;

        // Bob receives unencrypted message from Alice
        let msg = bob.parse_msg(&sent).await;
        assert!(!msg.was_encrypted());

        // Parsing a message is enough to update peerstate
        let peerstate_alice = Peerstate::from_addr(&bob.ctx, "alice@example.com")
            .await?
            .expect("no peerstate found in the database");
        assert_eq!(peerstate_alice.prefer_encrypt, EncryptPreference::Mutual);

        // Bob sends encrypted message to Alice
        let mut msg = Message::new(Viewtype::Text);
        chat::prepare_msg(&bob.ctx, chat_bob, &mut msg).await?;
        chat::send_msg(&bob.ctx, chat_bob, &mut msg).await?;
        let sent = bob.pop_sent_msg().await;

        // Alice receives encrypted message from Bob
        let msg = alice.parse_msg(&sent).await;
        assert!(msg.was_encrypted());

        let peerstate_bob = Peerstate::from_addr(&alice.ctx, "bob@example.net")
            .await?
            .expect("no peerstate found in the database");
        assert_eq!(peerstate_bob.prefer_encrypt, EncryptPreference::Mutual);

        // Now Alice and Bob have established keys.

        // Alice sends encrypted message without Autocrypt header.
        let mut msg = Message::new(Viewtype::Text);
        msg.param.set_int(Param::SkipAutocrypt, 1);
        chat::prepare_msg(&alice.ctx, chat_alice, &mut msg).await?;
        chat::send_msg(&alice.ctx, chat_alice, &mut msg).await?;
        let sent = alice.pop_sent_msg().await;

        let msg = bob.parse_msg(&sent).await;
        assert!(msg.was_encrypted());
        let peerstate_alice = Peerstate::from_addr(&bob.ctx, "alice@example.com")
            .await?
            .expect("no peerstate found in the database");
        assert_eq!(peerstate_alice.prefer_encrypt, EncryptPreference::Mutual);

        // Alice sends plaintext message with Autocrypt header.
        let mut msg = Message::new(Viewtype::Text);
        msg.param.set_int(Param::ForcePlaintext, 1);
        chat::prepare_msg(&alice.ctx, chat_alice, &mut msg).await?;
        chat::send_msg(&alice.ctx, chat_alice, &mut msg).await?;
        let sent = alice.pop_sent_msg().await;

        let msg = bob.parse_msg(&sent).await;
        assert!(!msg.was_encrypted());
        let peerstate_alice = Peerstate::from_addr(&bob.ctx, "alice@example.com")
            .await?
            .expect("no peerstate found in the database");
        assert_eq!(peerstate_alice.prefer_encrypt, EncryptPreference::Mutual);

        // Alice sends plaintext message without Autocrypt header.
        let mut msg = Message::new(Viewtype::Text);
        msg.param.set_int(Param::ForcePlaintext, 1);
        msg.param.set_int(Param::SkipAutocrypt, 1);
        chat::prepare_msg(&alice.ctx, chat_alice, &mut msg).await?;
        chat::send_msg(&alice.ctx, chat_alice, &mut msg).await?;
        let sent = alice.pop_sent_msg().await;

        let msg = bob.parse_msg(&sent).await;
        assert!(!msg.was_encrypted());
        let peerstate_alice = Peerstate::from_addr(&bob.ctx, "alice@example.com")
            .await?
            .expect("no peerstate found in the database");
        assert_eq!(peerstate_alice.prefer_encrypt, EncryptPreference::Reset);

        Ok(())
    }

    fn new_peerstates(
        ctx: &Context,
        prefer_encrypt: EncryptPreference,
    ) -> Vec<(Option<Peerstate<'_>>, &str)> {
        let addr = "bob@foo.bar";
        let pub_key = bob_keypair().public;
        let peerstate = Peerstate {
            context: &ctx,
            addr: addr.into(),
            last_seen: 13,
            last_seen_autocrypt: 14,
            prefer_encrypt,
            public_key: Some(pub_key.clone()),
            public_key_fingerprint: Some(pub_key.fingerprint()),
            gossip_key: Some(pub_key.clone()),
            gossip_timestamp: 15,
            gossip_key_fingerprint: Some(pub_key.fingerprint()),
            verified_key: Some(pub_key.clone()),
            verified_key_fingerprint: Some(pub_key.fingerprint()),
            to_save: Some(ToSave::All),
            fingerprint_changed: false,
        };
        let mut peerstates = Vec::new();
        peerstates.push((Some(peerstate), addr));
        peerstates
    }

    #[async_std::test]
    async fn test_should_encrypt() {
        let t = TestContext::new_alice().await;
        let encrypt_helper = EncryptHelper::new(&t.ctx).await.unwrap();

        // test with EncryptPreference::NoPreference:
        // if e2ee_eguaranteed is unset, there is no encryption as not more than half of peers want encryption
        let ps = new_peerstates(&t.ctx, EncryptPreference::NoPreference);
        assert!(encrypt_helper.should_encrypt(&t.ctx, true, &ps).unwrap());
        assert!(!encrypt_helper.should_encrypt(&t.ctx, false, &ps).unwrap());

        // test with EncryptPreference::Reset
        let ps = new_peerstates(&t.ctx, EncryptPreference::Reset);
        assert!(encrypt_helper.should_encrypt(&t.ctx, true, &ps).unwrap());
        assert!(!encrypt_helper.should_encrypt(&t.ctx, false, &ps).unwrap());

        // test with EncryptPreference::Mutual (self is also Mutual)
        let ps = new_peerstates(&t.ctx, EncryptPreference::Mutual);
        assert!(encrypt_helper.should_encrypt(&t.ctx, true, &ps).unwrap());
        assert!(encrypt_helper.should_encrypt(&t.ctx, false, &ps).unwrap());

        // test with missing peerstate
        let mut ps = Vec::new();
        ps.push((None, "bob@foo.bar"));
        assert!(encrypt_helper.should_encrypt(&t.ctx, true, &ps).is_err());
        assert!(!encrypt_helper.should_encrypt(&t.ctx, false, &ps).unwrap());
    }
}
