//! Cryptographic key module

use std::collections::BTreeMap;
use std::io::Cursor;
use std::path::Path;

use pgp::composed::Deserializable;
use pgp::ser::Serialize;
use pgp::types::{KeyTrait, SecretKeyTrait};

use crate::constants::*;
use crate::context::Context;
use crate::dc_tools::*;
use crate::sql::Sql;

// Re-export key types
pub use crate::pgp::KeyPair;
pub use pgp::composed::{SignedPublicKey, SignedSecretKey};

/// Error type for deltachat key handling.
#[derive(Fail, Debug)]
pub enum Error {
    #[fail(display = "Could not decode base64")]
    Base64Decode(#[cause] base64::DecodeError, failure::Backtrace),
    #[fail(display = "rPGP error: {}", _0)]
    PgpError(#[cause] pgp::errors::Error, failure::Backtrace),
}

impl From<base64::DecodeError> for Error {
    fn from(err: base64::DecodeError) -> Error {
        Error::Base64Decode(err, failure::Backtrace::new())
    }
}

impl From<pgp::errors::Error> for Error {
    fn from(err: pgp::errors::Error) -> Error {
        Error::PgpError(err, failure::Backtrace::new())
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// Convenience trait for working with keys.
///
/// This trait is implemented for rPGP's [SignedPublicKey] and
/// [SignedSecretKey] types and makes working with them a little
/// easier in the deltachat world.
pub trait DcKey: Serialize + Deserializable {
    type KeyType: Serialize + Deserializable;

    /// Create a key from some bytes.
    fn from_slice(bytes: &[u8]) -> Result<Self::KeyType> {
        Ok(<Self::KeyType as Deserializable>::from_bytes(Cursor::new(
            bytes,
        ))?)
    }

    /// Create a key from a base64 string.
    fn from_base64(data: &str) -> Result<Self::KeyType> {
        // strip newlines and other whitespace
        let cleaned: String = data.trim().split_whitespace().collect();
        let bytes = base64::decode(cleaned.as_bytes())?;
        Self::from_slice(&bytes)
    }

    /// Serialise the key to a base64 string.
    fn to_base64(&self) -> String {
        // Not using Serialize::to_bytes() to make clear *why* it is
        // safe to ignore this error.
        // Because we write to a Vec<u8> the io::Write impls never
        // fail and we can hide this error.
        let mut buf = Vec::new();
        self.to_writer(&mut buf).unwrap();
        base64::encode(&buf)
    }
}

impl DcKey for SignedPublicKey {
    type KeyType = SignedPublicKey;
}

impl DcKey for SignedSecretKey {
    type KeyType = SignedSecretKey;
}

/// Cryptographic key
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Key {
    Public(SignedPublicKey),
    Secret(SignedSecretKey),
}

impl From<SignedPublicKey> for Key {
    fn from(key: SignedPublicKey) -> Self {
        Key::Public(key)
    }
}

impl From<SignedSecretKey> for Key {
    fn from(key: SignedSecretKey) -> Self {
        Key::Secret(key)
    }
}

impl std::convert::TryFrom<Key> for SignedSecretKey {
    type Error = ();

    fn try_from(value: Key) -> std::result::Result<Self, Self::Error> {
        match value {
            Key::Public(_) => Err(()),
            Key::Secret(key) => Ok(key),
        }
    }
}

impl<'a> std::convert::TryFrom<&'a Key> for &'a SignedSecretKey {
    type Error = ();

    fn try_from(value: &'a Key) -> std::result::Result<Self, Self::Error> {
        match value {
            Key::Public(_) => Err(()),
            Key::Secret(key) => Ok(key),
        }
    }
}

impl std::convert::TryFrom<Key> for SignedPublicKey {
    type Error = ();

    fn try_from(value: Key) -> std::result::Result<Self, Self::Error> {
        match value {
            Key::Public(key) => Ok(key),
            Key::Secret(_) => Err(()),
        }
    }
}

impl<'a> std::convert::TryFrom<&'a Key> for &'a SignedPublicKey {
    type Error = ();

    fn try_from(value: &'a Key) -> std::result::Result<Self, Self::Error> {
        match value {
            Key::Public(key) => Ok(key),
            Key::Secret(_) => Err(()),
        }
    }
}

impl Key {
    pub fn is_public(&self) -> bool {
        match self {
            Key::Public(_) => true,
            Key::Secret(_) => false,
        }
    }

    pub fn is_secret(&self) -> bool {
        !self.is_public()
    }

    pub fn from_slice(bytes: &[u8], key_type: KeyType) -> Option<Self> {
        if bytes.is_empty() {
            return None;
        }
        let res: std::result::Result<Key, _> = match key_type {
            KeyType::Public => SignedPublicKey::from_bytes(Cursor::new(bytes)).map(Into::into),
            KeyType::Private => SignedSecretKey::from_bytes(Cursor::new(bytes)).map(Into::into),
        };

        match res {
            Ok(key) => Some(key),
            Err(err) => {
                eprintln!("Invalid key bytes: {:?}", err);
                None
            }
        }
    }

    pub fn from_armored_string(
        data: &str,
        key_type: KeyType,
    ) -> Option<(Self, BTreeMap<String, String>)> {
        let bytes = data.as_bytes();
        let res: std::result::Result<(Key, _), _> = match key_type {
            KeyType::Public => SignedPublicKey::from_armor_single(Cursor::new(bytes))
                .map(|(k, h)| (Into::into(k), h)),
            KeyType::Private => SignedSecretKey::from_armor_single(Cursor::new(bytes))
                .map(|(k, h)| (Into::into(k), h)),
        };

        match res {
            Ok(res) => Some(res),
            Err(err) => {
                eprintln!("Invalid key bytes: {:?}", err);
                None
            }
        }
    }

    pub fn from_self_public(
        context: &Context,
        self_addr: impl AsRef<str>,
        sql: &Sql,
    ) -> Option<Self> {
        let addr = self_addr.as_ref();

        sql.query_get_value(
            context,
            "SELECT public_key FROM keypairs WHERE addr=? AND is_default=1;",
            &[addr],
        )
        .and_then(|blob: Vec<u8>| Self::from_slice(&blob, KeyType::Public))
    }

    pub fn from_self_private(
        context: &Context,
        self_addr: impl AsRef<str>,
        sql: &Sql,
    ) -> Option<Self> {
        sql.query_get_value(
            context,
            "SELECT private_key FROM keypairs WHERE addr=? AND is_default=1;",
            &[self_addr.as_ref()],
        )
        .and_then(|blob: Vec<u8>| Self::from_slice(&blob, KeyType::Private))
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            Key::Public(k) => k.to_bytes().unwrap_or_default(),
            Key::Secret(k) => k.to_bytes().unwrap_or_default(),
        }
    }

    pub fn verify(&self) -> bool {
        match self {
            Key::Public(k) => k.verify().is_ok(),
            Key::Secret(k) => k.verify().is_ok(),
        }
    }

    pub fn to_base64(&self) -> String {
        let buf = self.to_bytes();
        base64::encode(&buf)
    }

    pub fn to_armored_string(
        &self,
        headers: Option<&BTreeMap<String, String>>,
    ) -> pgp::errors::Result<String> {
        match self {
            Key::Public(k) => k.to_armored_string(headers),
            Key::Secret(k) => k.to_armored_string(headers),
        }
    }

    /// Each header line must be terminated by `\r\n`
    pub fn to_asc(&self, header: Option<(&str, &str)>) -> String {
        let headers = header.map(|(key, value)| {
            let mut m = BTreeMap::new();
            m.insert(key.to_string(), value.to_string());
            m
        });

        self.to_armored_string(headers.as_ref())
            .expect("failed to serialize key")
    }

    pub fn write_asc_to_file(
        &self,
        file: impl AsRef<Path>,
        context: &Context,
    ) -> std::io::Result<()> {
        let file_content = self.to_asc(None).into_bytes();

        let res = dc_write_file(context, &file, &file_content);
        if res.is_err() {
            error!(context, "Cannot write key to {}", file.as_ref().display());
        }
        res
    }

    pub fn fingerprint(&self) -> String {
        match self {
            Key::Public(k) => hex::encode_upper(k.fingerprint()),
            Key::Secret(k) => hex::encode_upper(k.fingerprint()),
        }
    }

    pub fn formatted_fingerprint(&self) -> String {
        let rawhex = self.fingerprint();
        dc_format_fingerprint(&rawhex)
    }

    pub fn split_key(&self) -> Option<Key> {
        match self {
            Key::Public(_) => None,
            Key::Secret(k) => {
                let pub_key = k.public_key();
                pub_key.sign(k, || "".into()).map(Key::Public).ok()
            }
        }
    }
}

/// Use of a [KeyPair] for encryption or decryption.
///
/// This is used by [store_self_keypair] to know what kind of key is
/// being saved.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum KeyPairUse {
    /// The default key used to encrypt new messages.
    Default,
    /// Only used to decrypt existing message.
    ReadOnly,
}

/// Error saving a keypair to the database.
#[derive(Fail, Debug)]
#[fail(display = "SaveKeyError: {}", message)]
pub struct SaveKeyError {
    message: String,
    #[cause]
    cause: failure::Error,
    backtrace: failure::Backtrace,
}

impl SaveKeyError {
    fn new(message: impl Into<String>, cause: impl Into<failure::Error>) -> Self {
        Self {
            message: message.into(),
            cause: cause.into(),
            backtrace: failure::Backtrace::new(),
        }
    }
}

/// Store the keypair as an owned keypair for addr in the database.
///
/// This will save the keypair as keys for the given address.  The
/// "self" here refers to the fact that this DC instance owns the
/// keypair.  Usually `addr` will be [Config::ConfiguredAddr].
///
/// If either the public or private keys are already present in the
/// database, this entry will be removed first regardless of the
/// address associated with it.  Practically this means saving the
/// same key again overwrites it.
///
/// [Config::ConfiguredAddr]: crate::config::Config::ConfiguredAddr
pub fn store_self_keypair(
    context: &Context,
    keypair: &KeyPair,
    default: KeyPairUse,
) -> std::result::Result<(), SaveKeyError> {
    // Everything should really be one transaction, more refactoring
    // is needed for that.
    let public_key = keypair
        .public
        .to_bytes()
        .map_err(|err| SaveKeyError::new("failed to serialise public key", err))?;
    let secret_key = keypair
        .secret
        .to_bytes()
        .map_err(|err| SaveKeyError::new("failed to serialise secret key", err))?;
    context
        .sql
        .execute(
            "DELETE FROM keypairs WHERE public_key=? OR private_key=?;",
            params![public_key, secret_key],
        )
        .map_err(|err| SaveKeyError::new("failed to remove old use of key", err))?;
    if default == KeyPairUse::Default {
        context
            .sql
            .execute("UPDATE keypairs SET is_default=0;", params![])
            .map_err(|err| SaveKeyError::new("failed to clear default", err))?;
    }
    let is_default = match default {
        KeyPairUse::Default => true,
        KeyPairUse::ReadOnly => false,
    };
    context
        .sql
        .execute(
            "INSERT INTO keypairs (addr, is_default, public_key, private_key, created)
                VALUES (?,?,?,?,?);",
            params![
                keypair.addr.to_string(),
                is_default as i32,
                public_key,
                secret_key,
                time()
            ],
        )
        .map(|_| ())
        .map_err(|err| SaveKeyError::new("failed to insert keypair", err))
}

/// Make a fingerprint human-readable, in hex format.
pub fn dc_format_fingerprint(fingerprint: &str) -> String {
    // split key into chunks of 4 with space, and 20 newline
    let mut res = String::new();

    for (i, c) in fingerprint.chars().enumerate() {
        if i > 0 && i % 20 == 0 {
            res += "\n";
        } else if i > 0 && i % 4 == 0 {
            res += " ";
        }

        res += &c.to_string();
    }

    res
}

/// Bring a human-readable or otherwise formatted fingerprint back to the 40-characters-uppercase-hex format.
pub fn dc_normalize_fingerprint(fp: &str) -> String {
    fp.to_uppercase()
        .chars()
        .filter(|&c| c >= '0' && c <= '9' || c >= 'A' && c <= 'F')
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;
    use std::convert::TryFrom;

    use lazy_static::lazy_static;

    lazy_static! {
        static ref KEYPAIR: KeyPair = alice_keypair();
    }

    #[test]
    fn test_normalize_fingerprint() {
        let fingerprint = dc_normalize_fingerprint(" 1234  567890 \n AbcD abcdef ABCDEF ");

        assert_eq!(fingerprint, "1234567890ABCDABCDEFABCDEF");
    }

    #[test]
    fn test_from_armored_string() {
        let (private_key, _) = Key::from_armored_string(
            "-----BEGIN PGP PRIVATE KEY BLOCK-----

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
7yPJeQ==
=KZk/
-----END PGP PRIVATE KEY BLOCK-----",
            KeyType::Private,
        )
        .expect("failed to decode"); // NOTE: if you take out the ===GU1/ part, everything passes!
        let binary = private_key.to_bytes();
        Key::from_slice(&binary, KeyType::Private).expect("invalid private key");
    }

    #[test]
    fn test_format_fingerprint() {
        let fingerprint = dc_format_fingerprint("1234567890ABCDABCDEFABCDEF1234567890ABCD");

        assert_eq!(
            fingerprint,
            "1234 5678 90AB CDAB CDEF\nABCD EF12 3456 7890 ABCD"
        );
    }

    #[test]
    fn test_from_slice_roundtrip() {
        let public_key = Key::from(KEYPAIR.public.clone());
        let private_key = Key::from(KEYPAIR.secret.clone());

        let binary = public_key.to_bytes();
        let public_key2 = Key::from_slice(&binary, KeyType::Public).expect("invalid public key");
        assert_eq!(public_key, public_key2);

        let binary = private_key.to_bytes();
        let private_key2 = Key::from_slice(&binary, KeyType::Private).expect("invalid private key");
        assert_eq!(private_key, private_key2);
    }

    #[test]
    fn test_from_slice_bad_data() {
        let mut bad_data: [u8; 4096] = [0; 4096];

        for i in 0..4096 {
            bad_data[i] = (i & 0xff) as u8;
        }

        for j in 0..(4096 / 40) {
            let bad_key = Key::from_slice(
                &bad_data[j..j + 4096 / 2 + j],
                if 0 != j & 1 {
                    KeyType::Public
                } else {
                    KeyType::Private
                },
            );
            assert!(bad_key.is_none());
        }
    }

    #[test]
    fn test_ascii_roundtrip() {
        let public_key = Key::from(KEYPAIR.public.clone());
        let private_key = Key::from(KEYPAIR.secret.clone());

        let s = public_key.to_armored_string(None).unwrap();
        let (public_key2, _) =
            Key::from_armored_string(&s, KeyType::Public).expect("invalid public key");
        assert_eq!(public_key, public_key2);

        let s = private_key.to_armored_string(None).unwrap();
        println!("{}", &s);
        let (private_key2, _) =
            Key::from_armored_string(&s, KeyType::Private).expect("invalid private key");
        assert_eq!(private_key, private_key2);
    }

    #[test]
    fn test_split_key() {
        let private_key = Key::from(KEYPAIR.secret.clone());
        let public_wrapped = private_key.split_key().unwrap();
        let public = SignedPublicKey::try_from(public_wrapped).unwrap();
        assert_eq!(public.primary_key, KEYPAIR.public.primary_key);
    }

    #[test]
    fn test_save_self_key_twice() {
        // Saving the same key twice should result in only one row in
        // the keypairs table.
        let t = dummy_context();
        let nrows = || {
            t.ctx
                .sql
                .query_get_value::<_, u32>(&t.ctx, "SELECT COUNT(*) FROM keypairs;", params![])
                .unwrap()
        };
        assert_eq!(nrows(), 0);
        store_self_keypair(&t.ctx, &KEYPAIR, KeyPairUse::Default).unwrap();
        assert_eq!(nrows(), 1);
        store_self_keypair(&t.ctx, &KEYPAIR, KeyPairUse::Default).unwrap();
        assert_eq!(nrows(), 1);
    }
}
