//! Cryptographic key module.

use std::collections::BTreeMap;
use std::fmt;
use std::io::Cursor;

use anyhow::{format_err, Result};
use async_trait::async_trait;
use num_traits::FromPrimitive;
use pgp::composed::Deserializable;
use pgp::ser::Serialize;
use pgp::types::{KeyTrait, SecretKeyTrait};

use crate::config::Config;
use crate::constants::KeyGenType;
use crate::context::Context;
use crate::dc_tools::{time, EmailAddress};

// Re-export key types
pub use crate::pgp::KeyPair;
pub use pgp::composed::{SignedPublicKey, SignedSecretKey};

/// Convenience trait for working with keys.
///
/// This trait is implemented for rPGP's [SignedPublicKey] and
/// [SignedSecretKey] types and makes working with them a little
/// easier in the deltachat world.
#[async_trait]
pub trait DcKey: Serialize + Deserializable + KeyTrait + Clone {
    type KeyType: Serialize + Deserializable + KeyTrait + Clone;

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

    /// Create a key from an ASCII-armored string.
    ///
    /// Returns the key and a map of any headers which might have been set in
    /// the ASCII-armored representation.
    fn from_asc(data: &str) -> Result<(Self::KeyType, BTreeMap<String, String>)> {
        let bytes = data.as_bytes();
        Self::KeyType::from_armor_single(Cursor::new(bytes))
            .map_err(|err| format_err!("rPGP error: {}", err))
    }

    /// Load the users' default key from the database.
    async fn load_self(context: &Context) -> Result<Self::KeyType>;

    /// Serialise the key as bytes.
    fn to_bytes(&self) -> Vec<u8> {
        // Not using Serialize::to_bytes() to make clear *why* it is
        // safe to ignore this error.
        // Because we write to a Vec<u8> the io::Write impls never
        // fail and we can hide this error.
        let mut buf = Vec::new();
        self.to_writer(&mut buf).unwrap();
        buf
    }

    /// Serialise the key to a base64 string.
    fn to_base64(&self) -> String {
        base64::encode(&DcKey::to_bytes(self))
    }

    /// Serialise the key to ASCII-armored representation.
    ///
    /// Each header line must be terminated by `\r\n`.  Only allows setting one
    /// header as a simplification since that's the only way it's used so far.
    // Since .to_armored_string() are actual methods on SignedPublicKey and
    // SignedSecretKey we can not generically implement this.
    fn to_asc(&self, header: Option<(&str, &str)>) -> String;

    /// The fingerprint for the key.
    fn fingerprint(&self) -> Fingerprint {
        Fingerprint::new(KeyTrait::fingerprint(self)).expect("Invalid fingerprint from rpgp")
    }
}

#[async_trait]
impl DcKey for SignedPublicKey {
    type KeyType = SignedPublicKey;

    async fn load_self(context: &Context) -> Result<Self::KeyType> {
        match context
            .sql
            .query_row_optional(
                r#"
            SELECT public_key
              FROM keypairs
             WHERE addr=(SELECT value FROM config WHERE keyname="configured_addr")
               AND is_default=1;
            "#,
                paramsv![],
                |row| {
                    let bytes: Vec<u8> = row.get(0)?;
                    Ok(bytes)
                },
            )
            .await?
        {
            Some(bytes) => Self::from_slice(&bytes),
            None => {
                let keypair = generate_keypair(context).await?;
                Ok(keypair.public)
            }
        }
    }

    fn to_asc(&self, header: Option<(&str, &str)>) -> String {
        // Not using .to_armored_string() to make clear *why* it is
        // safe to ignore this error.
        // Because we write to a Vec<u8> the io::Write impls never
        // fail and we can hide this error.
        let headers = header.map(|(key, value)| {
            let mut m = BTreeMap::new();
            m.insert(key.to_string(), value.to_string());
            m
        });
        let mut buf = Vec::new();
        self.to_armored_writer(&mut buf, headers.as_ref())
            .unwrap_or_default();
        std::string::String::from_utf8(buf).unwrap_or_default()
    }
}

#[async_trait]
impl DcKey for SignedSecretKey {
    type KeyType = SignedSecretKey;

    async fn load_self(context: &Context) -> Result<Self::KeyType> {
        match context
            .sql
            .query_row_optional(
                r#"
            SELECT private_key
              FROM keypairs
             WHERE addr=(SELECT value FROM config WHERE keyname="configured_addr")
               AND is_default=1;
            "#,
                paramsv![],
                |row| {
                    let bytes: Vec<u8> = row.get(0)?;
                    Ok(bytes)
                },
            )
            .await?
        {
            Some(bytes) => Self::from_slice(&bytes),
            None => {
                let keypair = generate_keypair(context).await?;
                Ok(keypair.secret)
            }
        }
    }

    fn to_asc(&self, header: Option<(&str, &str)>) -> String {
        // Not using .to_armored_string() to make clear *why* it is
        // safe to do these unwraps.
        // Because we write to a Vec<u8> the io::Write impls never
        // fail and we can hide this error.  The string is always ASCII.
        let headers = header.map(|(key, value)| {
            let mut m = BTreeMap::new();
            m.insert(key.to_string(), value.to_string());
            m
        });
        let mut buf = Vec::new();
        self.to_armored_writer(&mut buf, headers.as_ref())
            .unwrap_or_default();
        std::string::String::from_utf8(buf).unwrap_or_default()
    }
}

/// Deltachat extension trait for secret keys.
///
/// Provides some convenience wrappers only applicable to [SignedSecretKey].
pub trait DcSecretKey {
    /// Create a public key from a private one.
    fn split_public_key(&self) -> Result<SignedPublicKey>;
}

impl DcSecretKey for SignedSecretKey {
    fn split_public_key(&self) -> Result<SignedPublicKey> {
        self.verify()?;
        let unsigned_pubkey = SecretKeyTrait::public_key(self);
        let signed_pubkey = unsigned_pubkey.sign(self, || "".into())?;
        Ok(signed_pubkey)
    }
}

async fn generate_keypair(context: &Context) -> Result<KeyPair> {
    let addr = context
        .get_config(Config::ConfiguredAddr)
        .await?
        .ok_or_else(|| format_err!("No address configured"))?;
    let addr = EmailAddress::new(&addr)?;
    let _guard = context.generating_key_mutex.lock().await;

    // Check if the key appeared while we were waiting on the lock.
    match context
        .sql
        .query_row_optional(
            r#"
        SELECT public_key, private_key
          FROM keypairs
         WHERE addr=?1
           AND is_default=1;
        "#,
            paramsv![addr],
            |row| {
                let pub_bytes: Vec<u8> = row.get(0)?;
                let sec_bytes: Vec<u8> = row.get(1)?;
                Ok((pub_bytes, sec_bytes))
            },
        )
        .await?
    {
        Some((pub_bytes, sec_bytes)) => Ok(KeyPair {
            addr,
            public: SignedPublicKey::from_slice(&pub_bytes)?,
            secret: SignedSecretKey::from_slice(&sec_bytes)?,
        }),
        None => {
            let start = std::time::SystemTime::now();
            let keytype = KeyGenType::from_i32(context.get_config_int(Config::KeyGenType).await?)
                .unwrap_or_default();
            info!(context, "Generating keypair with type {}", keytype);
            let keypair =
                async_std::task::spawn_blocking(move || crate::pgp::create_keypair(addr, keytype))
                    .await?;
            store_self_keypair(context, &keypair, KeyPairUse::Default).await?;
            info!(
                context,
                "Keypair generated in {:.3}s.",
                start.elapsed().unwrap_or_default().as_secs()
            );
            Ok(keypair)
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
pub async fn store_self_keypair(
    context: &Context,
    keypair: &KeyPair,
    default: KeyPairUse,
) -> Result<()> {
    // Everything should really be one transaction, more refactoring
    // is needed for that.
    let public_key = DcKey::to_bytes(&keypair.public);
    let secret_key = DcKey::to_bytes(&keypair.secret);
    context
        .sql
        .execute(
            "DELETE FROM keypairs WHERE public_key=? OR private_key=?;",
            paramsv![public_key, secret_key],
        )
        .await
        .map_err(|err| err.context("failed to remove old use of key"))?;
    if default == KeyPairUse::Default {
        context
            .sql
            .execute("UPDATE keypairs SET is_default=0;", paramsv![])
            .await
            .map_err(|err| err.context("failed to clear default"))?;
    }
    let is_default = match default {
        KeyPairUse::Default => true as i32,
        KeyPairUse::ReadOnly => false as i32,
    };

    let addr = keypair.addr.to_string();
    let t = time();

    context
        .sql
        .execute(
            "INSERT INTO keypairs (addr, is_default, public_key, private_key, created)
                VALUES (?,?,?,?,?);",
            paramsv![addr, is_default, public_key, secret_key, t],
        )
        .await
        .map_err(|err| err.context("failed to insert keypair"))?;

    Ok(())
}

/// A key fingerprint
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct Fingerprint(Vec<u8>);

impl Fingerprint {
    pub fn new(v: Vec<u8>) -> Result<Fingerprint> {
        match v.len() {
            20 => Ok(Fingerprint(v)),
            _ => Err(format_err!("Wrong fingerprint length")),
        }
    }

    /// Make a hex string from the fingerprint.
    ///
    /// Use [std::fmt::Display] or [ToString::to_string] to get a
    /// human-readable formatted string.
    pub fn hex(&self) -> String {
        hex::encode_upper(&self.0)
    }
}

impl fmt::Debug for Fingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Fingerprint")
            .field("hex", &self.hex())
            .finish()
    }
}

/// Make a human-readable fingerprint.
impl fmt::Display for Fingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Split key into chunks of 4 with space and newline at 20 chars
        for (i, c) in self.hex().chars().enumerate() {
            if i > 0 && i % 20 == 0 {
                writeln!(f)?;
            } else if i > 0 && i % 4 == 0 {
                write!(f, " ")?;
            }
            write!(f, "{}", c)?;
        }
        Ok(())
    }
}

/// Parse a human-readable or otherwise formatted fingerprint.
impl std::str::FromStr for Fingerprint {
    type Err = anyhow::Error;

    fn from_str(input: &str) -> std::result::Result<Self, Self::Err> {
        let hex_repr: String = input
            .to_uppercase()
            .chars()
            .filter(|&c| ('0'..='9').contains(&c) || ('A'..='F').contains(&c))
            .collect();
        let v: Vec<u8> = hex::decode(hex_repr)?;
        let fp = Fingerprint::new(v)?;
        Ok(fp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{alice_keypair, TestContext};

    use async_std::sync::Arc;
    use once_cell::sync::Lazy;

    static KEYPAIR: Lazy<KeyPair> = Lazy::new(alice_keypair);

    #[test]
    fn test_from_armored_string() {
        let (private_key, _) = SignedSecretKey::from_asc(
            "-----BEGIN PGP PRIVATE KEY BLOCK-----

xcLYBF0fgz4BCADnRUV52V4xhSsU56ZaAn3+3oG86MZhXy4X8w14WZZDf0VJGeTh
oTtVwiw9rVN8FiUELqpO2CS2OwS9mAGMJmGIt78bvIy2EHIAjUilqakmb0ChJxC+
ilSowab9slSdOgzQI1fzo+VZkhtczvRBq31cW8G05tuiLsnDSSS+sSH/GkvJqpzB
BWu6tSrMzth58KBM2XwWmozpLzy6wlrUBOYT8J79UVvs81O/DhXpVYYOWj2h4n3O
60qtK7SJBCjG7vGc2Ef8amsrjTDwUii0QQcF+BJN3ZuCI5AdOTpI39QuCDuD9UH2
NOKI+jYPQ4KB8pA1aYXBZzYyjuwCHzryXXsXABEBAAEAB/0VkYBJPNxsAd9is7fv
7QuTGW1AEPVvX1ENKr2226QH53auupt972t5NAKsPd3rVKVfHnsDn2TNGfP3OpXq
XCn8diZ8j7kPwbjgFE0SJiCAVR/R57LIEl6S3nyUbG03vJI1VxZ8wmxBTj7/CM3+
0d9/HY+TL3SMS5DFhazHm/1vrPbBz8FiNKtdTLHniW2/HUAN93aeALq0h4j7LKAC
QaQOs4ej/UeIvL7dihTGc2SwXfUA/5BEPDnlrBVhhCZhWuu3dF7nMMcEVP9/gFOH
khILR01b7fCfs+lxKHKxtAmHasOOi7xp26O61m3RQl//eid3CTdWpCNdxU4Y4kyp
9KsBBAD0IMXzkJOM6epVuD+sm5QDyKBow1sODjlc+RNIGUiUUOD8Ho+ra4qC391L
rn1T5xjJYExVqnnL//HVFGyGnkUZIwtztY5R8a2W9PnYQQedBL6XPnknI+6THEoe
Od9fIdsUaWd+Ab+svfpSoEy3wrFpP2G8340EGNBEpDcPIzqr6wQA8oRulFUMx0cS
ko65K4LCgpSpeEo6cI/PG/UNGM7Fb+eaF9UrF3Uq19ASiTPNAb6ZsJ007lmIW7+9
bkynYu75t4nhVnkiikTDS2KOeFQpmQbdTrHEbm9w614BtnCQEg4BzZU43dtTIhZN
Q50yYiAAhr5g+9H1QMOZ99yMzCIt/oUEAKZEISt1C6lf8iLpzCdKRlOEANmf7SyQ
P+7JZ4BXmaZEbFKGGQpWm1P3gYkYIT5jwnQsKsHdIAFiGfAZS4SPezesfRPlc4RB
9qLA0hDROrM47i5XK+kQPY3GPU7zNjbU9t60GyBhTzPAh+ikhUzNCBGj+3CqE8/3
NRMrGNvzhUwXOunNBzxoZWxsbz7CwIkEEAEIADMCGQEFAl0fg18CGwMECwkIBwYV
CAkKCwIDFgIBFiEEaeHEHjiV97rB+YeLMKMg0aJs7GIACgkQMKMg0aJs7GKh1gf+
Jx9A/7z5A3N6bzCjolnDMepktdVRAaW2Z/YDQ9eNxA3N0HHTN0StXGg55BVIrGZQ
2MbB++qx0nBQI4YM31RsWUIUfXm1EfPI8/07RAtrGdjfCsiG8Fi4YEEzDOgCRgQl
+cwioVPmcPWbQaZxpm6Z0HPG54VX3Pt/NXvc80GB6++13KMr+V87XWxsDjAnuo5+
edFWtreNq/qLE81xIwHSYgmzJbSAOhzhXfRYyWz8YM2YbEy0Ad3Zm1vkgQmC5q9m
Ge7qWdG+z2sYEy1TfM0evSO5B6/0YDeeNkyR6qXASMw9Yhsz8oxwzOfKdI270qaN
q6zaRuul7d5p3QJY2D0HIMfC2ARdH4M+AQgArioPOJsOhTcZfdPh/7I6f503YY3x
jqQ02WzcjzsJD4RHPXmF2l+N3F4vgxVe/voPPbvYDIu2leAnPoi7JWrBMSXH3Y5+
/TCC/I1JyhOG5r+OYiNmI7dgwfbuP41nDDb2sxbBUG/1HGNqVvwgayirgeJb4WEq
Gpk8dznS9Fb/THz5IUosnxeNjH3jyTDAL7c+L5i2DDCBi5JixX/EeV1wlH3xLiHB
YWEHMQ5S64ASWmnuvzrHKDQv0ClwDiP1o9FBiBsbcxszbvohyy+AmCiWV/D4ZGI9
nUid8MwLs0J+8jToqIhjiFmSIDPGpXOANHQLzSCxEN9Yj1G0d5B89NveiQARAQAB
AAf/XJ3LOFvkjdzuNmaNoS8DQse1IrCcCzGxVQo6BATt3Y2HYN6V2rnDs7N2aqvb
t5X8suSIkKtfbjYkSHHnq48oq10e+ugDCdtZXLo5yjc2HtExA2k1sLqcvqj0q2Ej
snAsIrJwHLlczDrl2tn612FqSwi3uZO1Ey335KMgVoVJAD/4nAj2Ku+Aqpw/nca5
w3mSx+YxmB/pwHIrr/0hfYLyVPy9QPJ/BqXVlAmSyZxzv7GOipCSouBLTibuEAsC
pI0TYRHtAnonY9F+8hiERda6qa+xXLaEwj1hiorEt62KaWYfiCC1Xr+Rlmo3GAwV
08X0yYFhdFMQ6wMhDdrHtB3iAQQA04O09JiUwIbNb7kjd3TpjUebjR2Vw5OT3a2/
4+73ESZPexDVJ/8dQAuRGDKx7UkLYsPJnU3Lc2IT456o4D0wytZJuGzwbMLo2Kn9
hAe+5KaN+/+MipsUcmC98zIMcRNDirIQV6vYmFo6WZVUsx1c+bH1EV7CmJuuY4+G
JKz0HMEEANLLWy/9enOvSpznYIUdtXxNG6evRHClkf7jZimM/VrAc4ICW4hqICK3
k5VMcRxVOa9hKZgg8vLfO8BRPRUB6Bc3SrK2jCKSli0FbtliNZS/lUBO1A7HRtY6
3coYUJBKqzmObLkh4C3RFQ5n/I6cJEvD7u9jzgpW71HtdI64NQvJBAC+88Q5irPg
07UZH9by8EVsCij8NFzChGmysHHGqeAMVVuI+rOqDqBsQA1n2aqxQ1uz5NZ9+ztu
Dn13hMEm8U2a9MtZdBhwlJrso3RzRf570V3E6qfdFqrQLoHDdRGRS9DMcUgMayo3
Hod6MFYzFVmbrmc822KmhaS3lBzLVpgkmEeJwsB2BBgBCAAgBQJdH4NfAhsMFiEE
aeHEHjiV97rB+YeLMKMg0aJs7GIACgkQMKMg0aJs7GLItQgAqKF63+HwAsjoPMBv
T9RdKdCaYV0MvxZyc7eM2pSk8cyfj6IPnxD8DPT699SMIzBfsrdGcfDYYgSODHL+
XsV31J215HfYBh/Nkru8fawiVxr+sJG2IDAeA9SBjsDCogfzW4PwLXgTXRqNFLVr
fK6hf6wpF56STV2U2D60b9xJeSAbBWlZFzCCQw3mPtGf/EGMHFxnJUE7MLEaaTEf
V2Fclh+G0sWp7F2ZS3nt0vX1hYG8TMIzM8Bj2eMsdXATOji9ST7EUxk/BpFax86D
i8pcjGO+IZffvyZJVRWfVooBJmWWbPB1pueo3tx8w3+fcuzpxz+RLFKaPyqXO+dD
7yPJeQ==
=KZk/
-----END PGP PRIVATE KEY BLOCK-----",
        )
        .expect("failed to decode");
        let binary = DcKey::to_bytes(&private_key);
        SignedSecretKey::from_slice(&binary).expect("invalid private key");
    }

    #[test]
    fn test_asc_roundtrip() {
        let key = KEYPAIR.public.clone();
        let asc = key.to_asc(Some(("spam", "ham")));
        let (key2, hdrs) = SignedPublicKey::from_asc(&asc).unwrap();
        assert_eq!(key, key2);
        assert_eq!(hdrs.len(), 1);
        assert_eq!(hdrs.get("spam"), Some(&String::from("ham")));

        let key = KEYPAIR.secret.clone();
        let asc = key.to_asc(Some(("spam", "ham")));
        let (key2, hdrs) = SignedSecretKey::from_asc(&asc).unwrap();
        assert_eq!(key, key2);
        assert_eq!(hdrs.len(), 1);
        assert_eq!(hdrs.get("spam"), Some(&String::from("ham")));
    }

    #[test]
    fn test_from_slice_roundtrip() {
        let public_key = KEYPAIR.public.clone();
        let private_key = KEYPAIR.secret.clone();

        let binary = DcKey::to_bytes(&public_key);
        let public_key2 = SignedPublicKey::from_slice(&binary).expect("invalid public key");
        assert_eq!(public_key, public_key2);

        let binary = DcKey::to_bytes(&private_key);
        let private_key2 = SignedSecretKey::from_slice(&binary).expect("invalid private key");
        assert_eq!(private_key, private_key2);
    }

    #[test]
    fn test_from_slice_bad_data() {
        let mut bad_data: [u8; 4096] = [0; 4096];
        for (i, v) in bad_data.iter_mut().enumerate() {
            *v = (i & 0xff) as u8;
        }
        for j in 0..(4096 / 40) {
            let slice = &bad_data.get(j..j + 4096 / 2 + j).unwrap();
            assert!(SignedPublicKey::from_slice(slice).is_err());
            assert!(SignedSecretKey::from_slice(slice).is_err());
        }
    }

    #[test]
    fn test_base64_roundtrip() {
        let key = KEYPAIR.public.clone();
        let base64 = key.to_base64();
        let key2 = SignedPublicKey::from_base64(&base64).unwrap();
        assert_eq!(key, key2);
    }

    #[async_std::test]
    async fn test_load_self_existing() {
        let alice = alice_keypair();
        let t = TestContext::new().await;
        t.configure_alice().await;
        let pubkey = SignedPublicKey::load_self(&t).await.unwrap();
        assert_eq!(alice.public, pubkey);
        let seckey = SignedSecretKey::load_self(&t).await.unwrap();
        assert_eq!(alice.secret, seckey);
    }

    #[async_std::test]
    async fn test_load_self_generate_public() {
        let t = TestContext::new().await;
        t.set_config(Config::ConfiguredAddr, Some("alice@example.org"))
            .await
            .unwrap();
        let key = SignedPublicKey::load_self(&t).await;
        assert!(key.is_ok());
    }

    #[async_std::test]
    async fn test_load_self_generate_secret() {
        let t = TestContext::new().await;
        t.set_config(Config::ConfiguredAddr, Some("alice@example.org"))
            .await
            .unwrap();
        let key = SignedSecretKey::load_self(&t).await;
        assert!(key.is_ok());
    }

    #[async_std::test]
    async fn test_load_self_generate_concurrent() {
        use std::thread;

        let t = TestContext::new().await;
        t.set_config(Config::ConfiguredAddr, Some("alice@example.org"))
            .await
            .unwrap();
        let thr0 = {
            let ctx = t.clone();
            thread::spawn(move || async_std::task::block_on(SignedPublicKey::load_self(&ctx)))
        };
        let thr1 = {
            let ctx = t.clone();
            thread::spawn(move || async_std::task::block_on(SignedPublicKey::load_self(&ctx)))
        };
        let res0 = thr0.join().unwrap();
        let res1 = thr1.join().unwrap();
        assert_eq!(res0.unwrap(), res1.unwrap());
    }

    #[test]
    fn test_split_key() {
        let pubkey = KEYPAIR.secret.split_public_key().unwrap();
        assert_eq!(pubkey.primary_key, KEYPAIR.public.primary_key);
    }

    #[async_std::test]
    async fn test_save_self_key_twice() {
        // Saving the same key twice should result in only one row in
        // the keypairs table.
        let t = TestContext::new().await;
        let ctx = Arc::new(t);

        let nrows = || async {
            ctx.sql
                .count("SELECT COUNT(*) FROM keypairs;", paramsv![])
                .await
                .unwrap()
        };
        assert_eq!(nrows().await, 0);
        store_self_keypair(&ctx, &KEYPAIR, KeyPairUse::Default)
            .await
            .unwrap();
        assert_eq!(nrows().await, 1);
        store_self_keypair(&ctx, &KEYPAIR, KeyPairUse::Default)
            .await
            .unwrap();
        assert_eq!(nrows().await, 1);
    }

    #[test]
    fn test_fingerprint_from_str() {
        let res = Fingerprint::new(vec![
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
        ])
        .unwrap();

        let fp: Fingerprint = "0102030405060708090A0B0c0d0e0F1011121314".parse().unwrap();
        assert_eq!(fp, res);

        let fp: Fingerprint = "zzzz 0102 0304 0506\n0708090a0b0c0D0E0F1011121314 yyy"
            .parse()
            .unwrap();
        assert_eq!(fp, res);

        assert!("1".parse::<Fingerprint>().is_err());
    }

    #[test]
    fn test_fingerprint_hex() {
        let fp = Fingerprint::new(vec![
            1, 2, 4, 8, 16, 32, 64, 128, 255, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
        ])
        .unwrap();
        assert_eq!(fp.hex(), "0102040810204080FF0A0B0C0D0E0F1011121314");
    }

    #[test]
    fn test_fingerprint_to_string() {
        let fp = Fingerprint::new(vec![
            1, 2, 4, 8, 16, 32, 64, 128, 255, 1, 2, 4, 8, 16, 32, 64, 128, 255, 19, 20,
        ])
        .unwrap();
        assert_eq!(
            fp.to_string(),
            "0102 0408 1020 4080 FF01\n0204 0810 2040 80FF 1314"
        );
    }
}
