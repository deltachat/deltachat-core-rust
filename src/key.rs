//! Cryptographic key module

use std::collections::BTreeMap;
use std::io::Cursor;
use std::path::Path;

use pgp::composed::{Deserializable, SignedPublicKey, SignedSecretKey};
use pgp::ser::Serialize;
use pgp::types::{KeyTrait, SecretKeyTrait};

use crate::constants::*;
use crate::context::Context;
use crate::dc_tools::*;
use crate::sql::{self, Sql};

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

    fn try_from(value: Key) -> Result<Self, Self::Error> {
        match value {
            Key::Public(_) => Err(()),
            Key::Secret(key) => Ok(key),
        }
    }
}

impl<'a> std::convert::TryFrom<&'a Key> for &'a SignedSecretKey {
    type Error = ();

    fn try_from(value: &'a Key) -> Result<Self, Self::Error> {
        match value {
            Key::Public(_) => Err(()),
            Key::Secret(key) => Ok(key),
        }
    }
}

impl std::convert::TryFrom<Key> for SignedPublicKey {
    type Error = ();

    fn try_from(value: Key) -> Result<Self, Self::Error> {
        match value {
            Key::Public(key) => Ok(key),
            Key::Secret(_) => Err(()),
        }
    }
}

impl<'a> std::convert::TryFrom<&'a Key> for &'a SignedPublicKey {
    type Error = ();

    fn try_from(value: &'a Key) -> Result<Self, Self::Error> {
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
        let res: Result<Key, _> = match key_type {
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
        let res: Result<(Key, _), _> = match key_type {
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

    pub fn from_base64(encoded_data: &str, key_type: KeyType) -> Option<Self> {
        // strip newlines and other whitespace
        let cleaned: String = encoded_data.trim().split_whitespace().collect();
        let bytes = cleaned.as_bytes();
        base64::decode(bytes)
            .ok()
            .and_then(|decoded| Self::from_slice(&decoded, key_type))
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

pub fn dc_key_save_self_keypair(
    context: &Context,
    public_key: &Key,
    private_key: &Key,
    addr: impl AsRef<str>,
    is_default: bool,
    sql: &Sql,
) -> bool {
    sql::execute(
        context,
        sql,
        "INSERT INTO keypairs (addr, is_default, public_key, private_key, created) VALUES (?,?,?,?,?);",
        params![addr.as_ref(), is_default as i32, public_key.to_bytes(), private_key.to_bytes(), time()],
    ).is_ok()
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

    #[test]
    fn test_normalize_fingerprint() {
        let fingerprint = dc_normalize_fingerprint(" 1234  567890 \n AbcD abcdef ABCDEF ");

        assert_eq!(fingerprint, "1234567890ABCDABCDEFABCDEF");
    }

    #[test]
    fn test_from_armored_string() {
        let (private_key, _) = Key::from_armored_string(
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
    #[ignore] // is too expensive
    fn test_from_slice_roundtrip() {
        let (public_key, private_key) = crate::pgp::create_keypair("hello").unwrap();

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
    #[ignore] // is too expensive
    fn test_ascii_roundtrip() {
        let (public_key, private_key) = crate::pgp::create_keypair("hello").unwrap();

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
}
