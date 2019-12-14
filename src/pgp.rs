//! OpenPGP helper module using [rPGP facilities](https://github.com/rpgp/rpgp)

use std::collections::{BTreeMap, HashSet};
use std::convert::TryInto;
use std::io;
use std::io::Cursor;

use pgp::armor::BlockType;
use pgp::composed::{
    Deserializable, KeyType as PgpKeyType, Message, SecretKeyParamsBuilder, SignedPublicKey,
    SignedPublicSubKey, SignedSecretKey, SubkeyParamsBuilder,
};
use pgp::crypto::{HashAlgorithm, SymmetricKeyAlgorithm};
use pgp::types::{
    CompressionAlgorithm, KeyTrait, Mpi, PublicKeyTrait, SecretKeyTrait, StringToKey,
};
use rand::{thread_rng, CryptoRng, Rng};

use crate::error::Result;
use crate::key::*;
use crate::keyring::*;

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

/// Split data from PGP Armored Data as defined in https://tools.ietf.org/html/rfc4880#section-6.2.
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

/// Create a new key pair.
pub fn create_keypair(addr: impl AsRef<str>) -> Option<(Key, Key)> {
    let user_id = format!("<{}>", addr.as_ref());

    let key_params = SecretKeyParamsBuilder::default()
        .key_type(PgpKeyType::Rsa(2048))
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
                .key_type(PgpKeyType::Rsa(2048))
                .can_encrypt(true)
                .passphrase(None)
                .build()
                .unwrap(),
        )
        .build()
        .expect("invalid key params");

    let key = key_params.generate().expect("invalid params");
    let private_key = key.sign(|| "".into()).expect("failed to sign secret key");

    let public_key = private_key.public_key();
    let public_key = public_key
        .sign(&private_key, || "".into())
        .expect("failed to sign public key");

    private_key.verify().expect("invalid private key generated");
    public_key.verify().expect("invalid public key generated");

    Some((Key::Public(public_key), Key::Secret(private_key)))
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
pub fn pk_encrypt(
    plain: &[u8],
    public_keys_for_encryption: &Keyring,
    private_key_for_signing: Option<&Key>,
) -> Result<String> {
    let lit_msg = Message::new_literal_bytes("", plain);
    let pkeys: Vec<SignedPublicKeyOrSubkey> = public_keys_for_encryption
        .keys()
        .iter()
        .filter_map(|key| {
            key.as_ref()
                .try_into()
                .ok()
                .and_then(select_pk_for_encryption)
        })
        .collect();
    let pkeys_refs: Vec<&SignedPublicKeyOrSubkey> = pkeys.iter().collect();

    let mut rng = thread_rng();

    // TODO: measure time
    let encrypted_msg = if let Some(private_key) = private_key_for_signing {
        let skey: &SignedSecretKey = private_key
            .try_into()
            .map_err(|_| format_err!("Invalid private key"))?;

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
}

#[allow(clippy::implicit_hasher)]
pub fn pk_decrypt(
    ctext: &[u8],
    private_keys_for_decryption: &Keyring,
    public_keys_for_validation: &Keyring,
    ret_signature_fingerprints: Option<&mut HashSet<String>>,
) -> Result<Vec<u8>> {
    let (msg, _) = Message::from_armor_single(Cursor::new(ctext))?;
    let skeys: Vec<&SignedSecretKey> = private_keys_for_decryption
        .keys()
        .iter()
        .filter_map(|key| {
            let k: &Key = &key;
            k.try_into().ok()
        })
        .collect();

    let (decryptor, _) = msg.decrypt(|| "".into(), || "".into(), &skeys[..])?;
    let msgs = decryptor.collect::<pgp::errors::Result<Vec<_>>>()?;
    ensure!(!msgs.is_empty(), "No valid messages found");

    let dec_msg = &msgs[0];

    if let Some(ret_signature_fingerprints) = ret_signature_fingerprints {
        if !public_keys_for_validation.keys().is_empty() {
            let pkeys: Vec<&SignedPublicKey> = public_keys_for_validation
                .keys()
                .iter()
                .filter_map(|key| {
                    let k: &Key = &key;
                    k.try_into().ok()
                })
                .collect();

            for pkey in &pkeys {
                if dec_msg.verify(&pkey.primary_key).is_ok() {
                    let fp = hex::encode_upper(pkey.fingerprint());
                    ret_signature_fingerprints.insert(fp);
                }
            }
        }
    }

    match dec_msg.get_content()? {
        Some(content) => Ok(content),
        None => bail!("Decrypted message is empty"),
    }
}

/// Symmetric encryption.
pub fn symm_encrypt(passphrase: &str, plain: &[u8]) -> Result<String> {
    let mut rng = thread_rng();
    let lit_msg = Message::new_literal_bytes("", plain);

    let s2k = StringToKey::new_default(&mut rng);
    let msg =
        lit_msg.encrypt_with_password(&mut rng, s2k, Default::default(), || passphrase.into())?;

    let encoded_msg = msg.to_armored_string(None)?;

    Ok(encoded_msg)
}

/// Symmetric decryption.
pub fn symm_decrypt<T: std::io::Read + std::io::Seek>(
    passphrase: &str,
    ctext: T,
) -> Result<Vec<u8>> {
    let (enc_msg, _) = Message::from_armor_single(ctext)?;
    let decryptor = enc_msg.decrypt_with_password(|| passphrase.into())?;

    let msgs = decryptor.collect::<pgp::errors::Result<Vec<_>>>()?;
    ensure!(!msgs.is_empty(), "No valid messages found");

    match msgs[0].get_content()? {
        Some(content) => Ok(content),
        None => bail!("Decrypted message is empty"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
