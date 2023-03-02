//! End-to-end encryption support.

use anyhow::{format_err, Context as _, Result};
use num_traits::FromPrimitive;

use crate::aheader::{Aheader, EncryptPreference};
use crate::config::Config;
use crate::context::Context;
use crate::key::{DcKey, SignedPublicKey, SignedSecretKey};
use crate::keyring::Keyring;
use crate::peerstate::{Peerstate, PeerstateVerifiedStatus};
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
            EncryptPreference::from_i32(context.get_config_int(Config::E2eeEnabled).await?)
                .unwrap_or_default();
        let addr = context.get_primary_self_addr().await?;
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
                    let msg = format!("peerstate for {addr:?} missing, cannot encrypt");
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
        peerstates: Vec<(Option<Peerstate>, &str)>,
    ) -> Result<String> {
        let mut keyring: Keyring<SignedPublicKey> = Keyring::new();

        for (peerstate, addr) in peerstates
            .into_iter()
            .filter_map(|(state, addr)| state.map(|s| (s, addr)))
        {
            let key = peerstate
                .take_key(min_verified)
                .with_context(|| format!("proper enc-key for {addr} missing, cannot encrypt"))?;
            keyring.add(key);
        }
        keyring.add(self.public_key.clone());
        let sign_key = SignedSecretKey::load_self(context).await?;

        let raw_message = mail_to_encrypt.build().as_string().into_bytes();

        let ctext = pgp::pk_encrypt(&raw_message, keyring, Some(sign_key)).await?;

        Ok(ctext)
    }

    /// Signs the passed-in `mail` using the private key from `context`.
    /// Returns the payload and the signature.
    pub async fn sign(
        self,
        context: &Context,
        mail: lettre_email::PartBuilder,
    ) -> Result<(lettre_email::MimeMessage, String)> {
        let sign_key = SignedSecretKey::load_self(context).await?;
        let mime_message = mail.build();
        let signature = pgp::pk_calc_signature(mime_message.as_string().as_bytes(), &sign_key)?;
        Ok((mime_message, signature))
    }
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
    let self_addr = context.get_primary_self_addr().await?;
    SignedPublicKey::load_self(context).await?;
    Ok(self_addr)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chat;
    use crate::message::{Message, Viewtype};
    use crate::param::Param;
    use crate::test_utils::{bob_keypair, TestContext};

    mod ensure_secret_key_exists {
        use super::*;

        #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
        async fn test_prexisting() {
            let t = TestContext::new_alice().await;
            assert_eq!(
                ensure_secret_key_exists(&t).await.unwrap(),
                "alice@example.org"
            );
        }

        #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
        async fn test_not_configured() {
            let t = TestContext::new().await;
            assert!(ensure_secret_key_exists(&t).await.is_err());
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_encrypted_no_autocrypt() -> anyhow::Result<()> {
        let alice = TestContext::new_alice().await;
        let bob = TestContext::new_bob().await;

        let chat_alice = alice.create_chat(&bob).await.id;
        let chat_bob = bob.create_chat(&alice).await.id;

        // Alice sends unencrypted message to Bob
        let mut msg = Message::new(Viewtype::Text);
        chat::prepare_msg(&alice.ctx, chat_alice, &mut msg).await?;
        chat::send_msg(&alice.ctx, chat_alice, &mut msg).await?;
        let sent = alice.pop_sent_msg().await;

        // Bob receives unencrypted message from Alice
        let msg = bob.parse_msg(&sent).await;
        assert!(!msg.was_encrypted());

        // Parsing a message is enough to update peerstate
        let peerstate_alice = Peerstate::from_addr(&bob.ctx, "alice@example.org")
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
        let peerstate_alice = Peerstate::from_addr(&bob.ctx, "alice@example.org")
            .await?
            .expect("no peerstate found in the database");
        assert_eq!(peerstate_alice.prefer_encrypt, EncryptPreference::Mutual);

        // Alice sends plaintext message with Autocrypt header.
        let mut msg = Message::new(Viewtype::Text);
        msg.force_plaintext();
        chat::prepare_msg(&alice.ctx, chat_alice, &mut msg).await?;
        chat::send_msg(&alice.ctx, chat_alice, &mut msg).await?;
        let sent = alice.pop_sent_msg().await;

        let msg = bob.parse_msg(&sent).await;
        assert!(!msg.was_encrypted());
        let peerstate_alice = Peerstate::from_addr(&bob.ctx, "alice@example.org")
            .await?
            .expect("no peerstate found in the database");
        assert_eq!(peerstate_alice.prefer_encrypt, EncryptPreference::Mutual);

        // Alice sends plaintext message without Autocrypt header.
        let mut msg = Message::new(Viewtype::Text);
        msg.force_plaintext();
        msg.param.set_int(Param::SkipAutocrypt, 1);
        chat::prepare_msg(&alice.ctx, chat_alice, &mut msg).await?;
        chat::send_msg(&alice.ctx, chat_alice, &mut msg).await?;
        let sent = alice.pop_sent_msg().await;

        let msg = bob.parse_msg(&sent).await;
        assert!(!msg.was_encrypted());
        let peerstate_alice = Peerstate::from_addr(&bob.ctx, "alice@example.org")
            .await?
            .expect("no peerstate found in the database");
        assert_eq!(peerstate_alice.prefer_encrypt, EncryptPreference::Reset);

        Ok(())
    }

    fn new_peerstates(prefer_encrypt: EncryptPreference) -> Vec<(Option<Peerstate>, &'static str)> {
        let addr = "bob@foo.bar";
        let pub_key = bob_keypair().public;
        let peerstate = Peerstate {
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
            fingerprint_changed: false,
            verifier: None,
        };
        vec![(Some(peerstate), addr)]
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_should_encrypt() {
        let t = TestContext::new_alice().await;
        let encrypt_helper = EncryptHelper::new(&t).await.unwrap();

        // test with EncryptPreference::NoPreference:
        // if e2ee_eguaranteed is unset, there is no encryption as not more than half of peers want encryption
        let ps = new_peerstates(EncryptPreference::NoPreference);
        assert!(encrypt_helper.should_encrypt(&t, true, &ps).unwrap());
        assert!(!encrypt_helper.should_encrypt(&t, false, &ps).unwrap());

        // test with EncryptPreference::Reset
        let ps = new_peerstates(EncryptPreference::Reset);
        assert!(encrypt_helper.should_encrypt(&t, true, &ps).unwrap());
        assert!(!encrypt_helper.should_encrypt(&t, false, &ps).unwrap());

        // test with EncryptPreference::Mutual (self is also Mutual)
        let ps = new_peerstates(EncryptPreference::Mutual);
        assert!(encrypt_helper.should_encrypt(&t, true, &ps).unwrap());
        assert!(encrypt_helper.should_encrypt(&t, false, &ps).unwrap());

        // test with missing peerstate
        let ps = vec![(None, "bob@foo.bar")];
        assert!(encrypt_helper.should_encrypt(&t, true, &ps).is_err());
        assert!(!encrypt_helper.should_encrypt(&t, false, &ps).unwrap());
    }
}
