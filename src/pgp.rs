//! OpenPGP helper module using [rPGP facilities](https://github.com/rpgp/rpgp).

use std::collections::{BTreeMap, HashSet};
use std::io;
use std::io::Cursor;

use anyhow::{bail, ensure, format_err, Context as _, Result};
use pgp::armor::BlockType;
use pgp::composed::{
    Deserializable, KeyType as PgpKeyType, Message, SecretKeyParamsBuilder, SignedPublicKey,
    SignedPublicSubKey, SignedSecretKey, StandaloneSignature, SubkeyParamsBuilder,
};
use pgp::crypto::{HashAlgorithm, SymmetricKeyAlgorithm};
use pgp::types::{
    CompressionAlgorithm, KeyTrait, Mpi, PublicKeyTrait, SecretKeyTrait, StringToKey,
};
use rand::{thread_rng, CryptoRng, Rng};

use crate::constants::KeyGenType;
use crate::dc_tools::EmailAddress;
use crate::key::{DcKey, Fingerprint};
use crate::keyring::Keyring;

pub const HEADER_AUTOCRYPT: &str = "autocrypt-prefer-encrypt";
pub const HEADER_SETUPCODE: &str = "passphrase-begin";

/// A wrapper for rPGP public key types
#[derive(Debug)]
enum SignedPublicKeyOrSubkey<'a> {
    Key(&'a SignedPublicKey),
    Subkey(&'a SignedPublicSubKey),
}

impl<'a> KeyTrait for SignedPublicKeyOrSubkey<'a> {
    fn fingerprint(&self) -> Vec<u8> {
        match self {
            Self::Key(k) => k.fingerprint(),
            Self::Subkey(k) => k.fingerprint(),
        }
    }

    fn key_id(&self) -> pgp::types::KeyId {
        match self {
            Self::Key(k) => k.key_id(),
            Self::Subkey(k) => k.key_id(),
        }
    }

    fn algorithm(&self) -> pgp::crypto::PublicKeyAlgorithm {
        match self {
            Self::Key(k) => k.algorithm(),
            Self::Subkey(k) => k.algorithm(),
        }
    }
}

impl<'a> PublicKeyTrait for SignedPublicKeyOrSubkey<'a> {
    fn verify_signature(
        &self,
        hash: HashAlgorithm,
        data: &[u8],
        sig: &[Mpi],
    ) -> pgp::errors::Result<()> {
        match self {
            Self::Key(k) => k.verify_signature(hash, data, sig),
            Self::Subkey(k) => k.verify_signature(hash, data, sig),
        }
    }

    fn encrypt<R: Rng + CryptoRng>(
        &self,
        rng: &mut R,
        plain: &[u8],
    ) -> pgp::errors::Result<Vec<Mpi>> {
        match self {
            Self::Key(k) => k.encrypt(rng, plain),
            Self::Subkey(k) => k.encrypt(rng, plain),
        }
    }

    fn to_writer_old(&self, writer: &mut impl io::Write) -> pgp::errors::Result<()> {
        match self {
            Self::Key(k) => k.to_writer_old(writer),
            Self::Subkey(k) => k.to_writer_old(writer),
        }
    }
}

/// Split data from PGP Armored Data as defined in <https://tools.ietf.org/html/rfc4880#section-6.2>.
///
/// Returns (type, headers, base64 encoded body).
pub fn split_armored_data(buf: &[u8]) -> Result<(BlockType, BTreeMap<String, String>, Vec<u8>)> {
    use std::io::Read;

    let cursor = Cursor::new(buf);
    let mut dearmor = pgp::armor::Dearmor::new(cursor);

    let mut bytes = Vec::with_capacity(buf.len());

    dearmor.read_to_end(&mut bytes)?;
    ensure!(dearmor.typ.is_some(), "Failed to parse type");

    let typ = dearmor.typ.unwrap();

    // normalize headers
    let headers = dearmor
        .headers
        .into_iter()
        .map(|(key, value)| (key.trim().to_lowercase(), value.trim().to_string()))
        .collect();

    Ok((typ, headers, bytes))
}

/// Error with generating a PGP keypair.
///
/// Most of these are likely coding errors rather than user errors
/// since all variability is hardcoded.
#[derive(Debug, thiserror::Error)]
#[error("PgpKeygenError: {message}")]
pub struct PgpKeygenError {
    message: String,
    #[source]
    cause: anyhow::Error,
}

impl PgpKeygenError {
    fn new(message: impl Into<String>, cause: impl Into<anyhow::Error>) -> Self {
        Self {
            message: message.into(),
            cause: cause.into(),
        }
    }
}

/// A PGP keypair.
///
/// This has it's own struct to be able to keep the public and secret
/// keys together as they are one unit.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct KeyPair {
    pub addr: EmailAddress,
    pub public: SignedPublicKey,
    pub secret: SignedSecretKey,
}

/// Create a new key pair.
pub(crate) fn create_keypair(
    addr: EmailAddress,
    keygen_type: KeyGenType,
) -> std::result::Result<KeyPair, PgpKeygenError> {
    let (secret_key_type, public_key_type) = match keygen_type {
        KeyGenType::Rsa2048 => (PgpKeyType::Rsa(2048), PgpKeyType::Rsa(2048)),
        KeyGenType::Ed25519 | KeyGenType::Default => (PgpKeyType::EdDSA, PgpKeyType::ECDH),
    };

    let user_id = format!("<{}>", addr);
    let key_params = SecretKeyParamsBuilder::default()
        .key_type(secret_key_type)
        .can_create_certificates(true)
        .can_sign(true)
        .primary_user_id(user_id)
        .passphrase(None)
        .preferred_symmetric_algorithms(smallvec![
            SymmetricKeyAlgorithm::AES256,
            SymmetricKeyAlgorithm::AES192,
            SymmetricKeyAlgorithm::AES128,
        ])
        .preferred_hash_algorithms(smallvec![
            HashAlgorithm::SHA2_256,
            HashAlgorithm::SHA2_384,
            HashAlgorithm::SHA2_512,
            HashAlgorithm::SHA2_224,
            HashAlgorithm::SHA1,
        ])
        .preferred_compression_algorithms(smallvec![
            CompressionAlgorithm::ZLIB,
            CompressionAlgorithm::ZIP,
        ])
        .subkey(
            SubkeyParamsBuilder::default()
                .key_type(public_key_type)
                .can_encrypt(true)
                .passphrase(None)
                .build()
                .unwrap(),
        )
        .build()
        .map_err(|err| PgpKeygenError::new("invalid key params", format_err!(err)))?;
    let key = key_params
        .generate()
        .map_err(|err| PgpKeygenError::new("invalid params", err))?;
    let private_key = key.sign(|| "".into()).expect("failed to sign secret key");

    let public_key = private_key.public_key();
    let public_key = public_key
        .sign(&private_key, || "".into())
        .map_err(|err| PgpKeygenError::new("failed to sign public key", err))?;

    private_key
        .verify()
        .map_err(|err| PgpKeygenError::new("invalid private key generated", err))?;
    public_key
        .verify()
        .map_err(|err| PgpKeygenError::new("invalid public key generated", err))?;

    Ok(KeyPair {
        addr,
        public: public_key,
        secret: private_key,
    })
}

/// Select public key or subkey to use for encryption.
///
/// First, tries to use subkeys. If none of the subkeys are suitable
/// for encryption, tries to use primary key. Returns `None` if the public
/// key cannot be used for encryption.
///
/// TODO: take key flags and expiration dates into account
fn select_pk_for_encryption(key: &SignedPublicKey) -> Option<SignedPublicKeyOrSubkey> {
    key.public_subkeys
        .iter()
        .find(|subkey| subkey.is_encryption_key())
        .map_or_else(
            || {
                // No usable subkey found, try primary key
                if key.is_encryption_key() {
                    Some(SignedPublicKeyOrSubkey::Key(key))
                } else {
                    None
                }
            },
            |subkey| Some(SignedPublicKeyOrSubkey::Subkey(subkey)),
        )
}

/// Encrypts `plain` text using `public_keys_for_encryption`
/// and signs it using `private_key_for_signing`.
pub async fn pk_encrypt(
    plain: &[u8],
    public_keys_for_encryption: Keyring<SignedPublicKey>,
    private_key_for_signing: Option<SignedSecretKey>,
) -> Result<String> {
    let lit_msg = Message::new_literal_bytes("", plain);

    async_std::task::spawn_blocking(move || {
        let pkeys: Vec<SignedPublicKeyOrSubkey> = public_keys_for_encryption
            .keys()
            .iter()
            .filter_map(select_pk_for_encryption)
            .collect();
        let pkeys_refs: Vec<&SignedPublicKeyOrSubkey> = pkeys.iter().collect();

        let mut rng = thread_rng();

        // TODO: measure time
        let encrypted_msg = if let Some(ref skey) = private_key_for_signing {
            lit_msg
                .sign(skey, || "".into(), Default::default())
                .and_then(|msg| msg.compress(CompressionAlgorithm::ZLIB))
                .and_then(|msg| msg.encrypt_to_keys(&mut rng, Default::default(), &pkeys_refs))
        } else {
            lit_msg.encrypt_to_keys(&mut rng, Default::default(), &pkeys_refs)
        };

        let msg = encrypted_msg?;
        let encoded_msg = msg.to_armored_string(None)?;

        Ok(encoded_msg)
    })
    .await
}

/// Decrypts the message with keys from the private key keyring.
///
/// Receiver private keys are provided in
/// `private_keys_for_decryption`.
///
/// Returns decrypted message and fingerprints
/// of all keys from the `public_keys_for_validation` keyring that
/// have valid signatures there.
#[allow(clippy::implicit_hasher)]
pub async fn pk_decrypt(
    ctext: Vec<u8>,
    private_keys_for_decryption: Keyring<SignedSecretKey>,
    public_keys_for_validation: &Keyring<SignedPublicKey>,
) -> Result<(Vec<u8>, HashSet<Fingerprint>)> {
    let mut ret_signature_fingerprints: HashSet<Fingerprint> = Default::default();

    let msgs = async_std::task::spawn_blocking(move || {
        let cursor = Cursor::new(ctext);
        let (msg, _) = Message::from_armor_single(cursor)?;

        let skeys: Vec<&SignedSecretKey> = private_keys_for_decryption.keys().iter().collect();

        let (decryptor, _) = msg.decrypt(|| "".into(), || "".into(), &skeys[..])?;
        decryptor.collect::<pgp::errors::Result<Vec<_>>>()
    })
    .await?;

    if let Some(msg) = msgs.into_iter().next() {
        // get_content() will decompress the message if needed,
        // but this avoids decompressing it again to check signatures
        let msg = msg.decompress()?;

        let content = match msg.get_content()? {
            Some(content) => content,
            None => bail!("The decrypted message is empty"),
        };

        if !public_keys_for_validation.is_empty() {
            let pkeys = public_keys_for_validation.keys();

            let mut fingerprints: Vec<Fingerprint> = Vec::new();
            if let signed_msg @ pgp::composed::Message::Signed { .. } = msg {
                for pkey in pkeys {
                    if signed_msg.verify(&pkey.primary_key).is_ok() {
                        let fp = DcKey::fingerprint(pkey);
                        fingerprints.push(fp);
                    }
                }
            }

            ret_signature_fingerprints.extend(fingerprints);
        }
        Ok((content, ret_signature_fingerprints))
    } else {
        bail!("No valid messages found");
    }
}

/// Validates detached signature.
pub async fn pk_validate(
    content: &[u8],
    signature: &[u8],
    public_keys_for_validation: &Keyring<SignedPublicKey>,
) -> Result<HashSet<Fingerprint>> {
    let mut ret: HashSet<Fingerprint> = Default::default();

    let standalone_signature = StandaloneSignature::from_armor_single(Cursor::new(signature))?.0;
    let pkeys = public_keys_for_validation.keys();

    // Remove trailing CRLF before the delimiter.
    // According to RFC 3156 it is considered to be part of the MIME delimiter for the purpose of
    // OpenPGP signature calculation.
    let content = content
        .get(..content.len().saturating_sub(2))
        .context("index is out of range")?;

    for pkey in pkeys {
        if standalone_signature.verify(pkey, content).is_ok() {
            let fp = DcKey::fingerprint(pkey);
            ret.insert(fp);
        }
    }
    Ok(ret)
}

/// Symmetric encryption.
pub async fn symm_encrypt(passphrase: &str, plain: &[u8]) -> Result<String> {
    let lit_msg = Message::new_literal_bytes("", plain);
    let passphrase = passphrase.to_string();

    async_std::task::spawn_blocking(move || {
        let mut rng = thread_rng();
        let s2k = StringToKey::new_default(&mut rng);
        let msg =
            lit_msg.encrypt_with_password(&mut rng, s2k, Default::default(), || passphrase)?;

        let encoded_msg = msg.to_armored_string(None)?;

        Ok(encoded_msg)
    })
    .await
}

/// Symmetric decryption.
pub async fn symm_decrypt<T: std::io::Read + std::io::Seek>(
    passphrase: &str,
    ctext: T,
) -> Result<Vec<u8>> {
    let (enc_msg, _) = Message::from_armor_single(ctext)?;

    let passphrase = passphrase.to_string();
    async_std::task::spawn_blocking(move || {
        let decryptor = enc_msg.decrypt_with_password(|| passphrase)?;

        let msgs = decryptor.collect::<pgp::errors::Result<Vec<_>>>()?;
        if let Some(msg) = msgs.first() {
            match msg.get_content()? {
                Some(content) => Ok(content),
                None => bail!("Decrypted message is empty"),
            }
        } else {
            bail!("No valid messages found")
        }
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{alice_keypair, bob_keypair};
    use once_cell::sync::Lazy;

    #[test]
    fn test_split_armored_data_1() {
        let (typ, _headers, base64) = split_armored_data(
            b"-----BEGIN PGP MESSAGE-----\nNoVal:\n\naGVsbG8gd29ybGQ=\n-----END PGP MESSAGE----",
        )
        .unwrap();

        assert_eq!(typ, BlockType::Message);
        assert!(!base64.is_empty());
        assert_eq!(
            std::string::String::from_utf8(base64).unwrap(),
            "hello world"
        );
    }

    #[test]
    fn test_split_armored_data_2() {
        let (typ, headers, base64) = split_armored_data(
            b"-----BEGIN PGP PRIVATE KEY BLOCK-----\nAutocrypt-Prefer-Encrypt: mutual \n\naGVsbG8gd29ybGQ=\n-----END PGP PRIVATE KEY BLOCK-----"
        )
            .unwrap();

        assert_eq!(typ, BlockType::PrivateKey);
        assert!(!base64.is_empty());
        assert_eq!(headers.get(HEADER_AUTOCRYPT), Some(&"mutual".to_string()));
    }

    #[test]
    fn test_create_keypair() {
        let keypair0 = create_keypair(
            EmailAddress::new("foo@bar.de").unwrap(),
            KeyGenType::Default,
        )
        .unwrap();
        let keypair1 = create_keypair(
            EmailAddress::new("two@zwo.de").unwrap(),
            KeyGenType::Default,
        )
        .unwrap();
        assert_ne!(keypair0.public, keypair1.public);
    }

    /// [Key] objects to use in tests.
    struct TestKeys {
        alice_secret: SignedSecretKey,
        alice_public: SignedPublicKey,
        bob_secret: SignedSecretKey,
        bob_public: SignedPublicKey,
    }

    impl TestKeys {
        fn new() -> TestKeys {
            let alice = alice_keypair();
            let bob = bob_keypair();
            TestKeys {
                alice_secret: alice.secret.clone(),
                alice_public: alice.public,
                bob_secret: bob.secret.clone(),
                bob_public: bob.public,
            }
        }
    }

    /// The original text of [CTEXT_SIGNED]
    static CLEARTEXT: &[u8] = b"This is a test";

    /// Initialised [TestKeys] for tests.
    static KEYS: Lazy<TestKeys> = Lazy::new(TestKeys::new);

    /// A cyphertext encrypted to Alice & Bob, signed by Alice.
    static CTEXT_SIGNED: Lazy<String> = Lazy::new(|| {
        let mut keyring = Keyring::new();
        keyring.add(KEYS.alice_public.clone());
        keyring.add(KEYS.bob_public.clone());
        futures_lite::future::block_on(pk_encrypt(
            CLEARTEXT,
            keyring,
            Some(KEYS.alice_secret.clone()),
        ))
        .unwrap()
    });

    /// A cyphertext encrypted to Alice & Bob, not signed.
    static CTEXT_UNSIGNED: Lazy<String> = Lazy::new(|| {
        let mut keyring = Keyring::new();
        keyring.add(KEYS.alice_public.clone());
        keyring.add(KEYS.bob_public.clone());
        futures_lite::future::block_on(pk_encrypt(CLEARTEXT, keyring, None)).unwrap()
    });

    #[test]
    fn test_encrypt_signed() {
        assert!(!CTEXT_SIGNED.is_empty());
        assert!(CTEXT_SIGNED.starts_with("-----BEGIN PGP MESSAGE-----"));
    }

    #[test]
    fn test_encrypt_unsigned() {
        assert!(!CTEXT_UNSIGNED.is_empty());
        assert!(CTEXT_UNSIGNED.starts_with("-----BEGIN PGP MESSAGE-----"));
    }

    #[async_std::test]
    async fn test_decrypt_singed() {
        // Check decrypting as Alice
        let mut decrypt_keyring: Keyring<SignedSecretKey> = Keyring::new();
        decrypt_keyring.add(KEYS.alice_secret.clone());
        let mut sig_check_keyring: Keyring<SignedPublicKey> = Keyring::new();
        sig_check_keyring.add(KEYS.alice_public.clone());
        let (plain, valid_signatures) = pk_decrypt(
            CTEXT_SIGNED.as_bytes().to_vec(),
            decrypt_keyring,
            &sig_check_keyring,
        )
        .await
        .unwrap();
        assert_eq!(plain, CLEARTEXT);
        assert_eq!(valid_signatures.len(), 1);

        // Check decrypting as Bob
        let mut decrypt_keyring = Keyring::new();
        decrypt_keyring.add(KEYS.bob_secret.clone());
        let mut sig_check_keyring = Keyring::new();
        sig_check_keyring.add(KEYS.alice_public.clone());
        let (plain, valid_signatures) = pk_decrypt(
            CTEXT_SIGNED.as_bytes().to_vec(),
            decrypt_keyring,
            &sig_check_keyring,
        )
        .await
        .unwrap();
        assert_eq!(plain, CLEARTEXT);
        assert_eq!(valid_signatures.len(), 1);
    }

    #[async_std::test]
    async fn test_decrypt_no_sig_check() {
        let mut keyring = Keyring::new();
        keyring.add(KEYS.alice_secret.clone());
        let empty_keyring = Keyring::new();
        let (plain, valid_signatures) =
            pk_decrypt(CTEXT_SIGNED.as_bytes().to_vec(), keyring, &empty_keyring)
                .await
                .unwrap();
        assert_eq!(plain, CLEARTEXT);
        assert_eq!(valid_signatures.len(), 0);
    }

    #[async_std::test]
    async fn test_decrypt_signed_no_key() {
        // The validation does not have the public key of the signer.
        let mut decrypt_keyring = Keyring::new();
        decrypt_keyring.add(KEYS.bob_secret.clone());
        let mut sig_check_keyring = Keyring::new();
        sig_check_keyring.add(KEYS.bob_public.clone());
        let (plain, valid_signatures) = pk_decrypt(
            CTEXT_SIGNED.as_bytes().to_vec(),
            decrypt_keyring,
            &sig_check_keyring,
        )
        .await
        .unwrap();
        assert_eq!(plain, CLEARTEXT);
        assert_eq!(valid_signatures.len(), 0);
    }

    #[async_std::test]
    async fn test_decrypt_unsigned() {
        let mut decrypt_keyring = Keyring::new();
        decrypt_keyring.add(KEYS.bob_secret.clone());
        let sig_check_keyring = Keyring::new();
        let (plain, valid_signatures) = pk_decrypt(
            CTEXT_UNSIGNED.as_bytes().to_vec(),
            decrypt_keyring,
            &sig_check_keyring,
        )
        .await
        .unwrap();
        assert_eq!(plain, CLEARTEXT);
        assert_eq!(valid_signatures.len(), 0);
    }
}
