use std::collections::BTreeMap;
use std::ffi::{CStr, CString};
use std::io::Cursor;
use std::slice;

use libc;
use pgp::composed::{Deserializable, SignedPublicKey, SignedSecretKey};
use pgp::ser::Serialize;
use pgp::types::{KeyTrait, SecretKeyTrait};

use crate::constants::*;
use crate::context::Context;
use crate::dc_sqlite3::*;
use crate::dc_tools::*;
use crate::types::*;
use crate::x::*;

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

impl std::convert::TryInto<SignedSecretKey> for Key {
    type Error = ();

    fn try_into(self) -> Result<SignedSecretKey, Self::Error> {
        match self {
            Key::Public(_) => Err(()),
            Key::Secret(key) => Ok(key),
        }
    }
}

impl<'a> std::convert::TryInto<&'a SignedSecretKey> for &'a Key {
    type Error = ();

    fn try_into(self) -> Result<&'a SignedSecretKey, Self::Error> {
        match self {
            Key::Public(_) => Err(()),
            Key::Secret(key) => Ok(key),
        }
    }
}

impl std::convert::TryInto<SignedPublicKey> for Key {
    type Error = ();

    fn try_into(self) -> Result<SignedPublicKey, Self::Error> {
        match self {
            Key::Public(key) => Ok(key),
            Key::Secret(_) => Err(()),
        }
    }
}

impl<'a> std::convert::TryInto<&'a SignedPublicKey> for &'a Key {
    type Error = ();

    fn try_into(self) -> Result<&'a SignedPublicKey, Self::Error> {
        match self {
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
        println!("hello from_slice");
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

    pub fn from_binary(data: *const u8, len: libc::c_int, key_type: KeyType) -> Option<Self> {
        if data.is_null() || len == 0 {
            return None;
        }

        let bytes = unsafe { slice::from_raw_parts(data, len as usize) };
        Self::from_slice(bytes, key_type)
    }

    pub fn from_stmt(
        stmt: *mut sqlite3_stmt,
        index: libc::c_int,
        key_type: KeyType,
    ) -> Option<Self> {
        assert!(!stmt.is_null(), "missing statement");

        let data = unsafe { sqlite3_column_blob(stmt, index) as *const u8 };
        let len = unsafe { sqlite3_column_bytes(stmt, index) };

        Self::from_binary(data, len, key_type)
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
        self_addr: *const libc::c_char,
        sql: &SQLite,
    ) -> Option<Self> {
        if self_addr.is_null() {
            return None;
        }

        let stmt = unsafe {
            dc_sqlite3_prepare(
                context,
                sql,
                b"SELECT public_key FROM keypairs WHERE addr=? AND is_default=1;\x00" as *const u8
                    as *const libc::c_char,
            )
        };
        unsafe { sqlite3_bind_text(stmt, 1, self_addr, -1, None) };

        let key = if unsafe { sqlite3_step(stmt) } == 100 {
            Self::from_stmt(stmt, 0, KeyType::Public)
        } else {
            None
        };

        unsafe { sqlite3_finalize(stmt) };

        key
    }

    pub fn from_self_private(
        context: &Context,
        self_addr: *const libc::c_char,
        sql: &SQLite,
    ) -> Option<Self> {
        if self_addr.is_null() {
            return None;
        }

        let stmt = unsafe {
            dc_sqlite3_prepare(
                context,
                sql,
                b"SELECT private_key FROM keypairs WHERE addr=? AND is_default=1;\x00" as *const u8
                    as *const libc::c_char,
            )
        };
        unsafe { sqlite3_bind_text(stmt, 1, self_addr, -1, None) };

        let key = if unsafe { sqlite3_step(stmt) } == 100 {
            Self::from_stmt(stmt, 0, KeyType::Private)
        } else {
            None
        };
        unsafe { sqlite3_finalize(stmt) };

        key
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            Key::Public(k) => k.to_bytes().unwrap(),
            Key::Secret(k) => k.to_bytes().unwrap(),
        }
    }

    pub fn verify(&self) -> bool {
        match self {
            Key::Public(k) => k.verify().is_ok(),
            Key::Secret(k) => k.verify().is_ok(),
        }
    }

    pub fn to_base64(&self, break_every: usize) -> String {
        let buf = self.to_bytes();

        let encoded = base64::encode(&buf);
        encoded
            .as_bytes()
            .chunks(break_every)
            .fold(String::new(), |mut res, buf| {
                // safe because we are using a base64 encoded string
                res += unsafe { std::str::from_utf8_unchecked(buf) };
                res += " ";
                res
            })
            .trim()
            .to_string()
    }

    /// the result must be freed
    pub fn to_base64_c(&self, break_every: usize) -> *mut libc::c_char {
        let res = self.to_base64(break_every);
        let res_c = CString::new(res.trim()).unwrap();

        // need to use strdup to allocate the result with malloc
        // so it can be `free`d later.
        unsafe { strdup(res_c.as_ptr()) }
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

    /// Each header line must be terminated by `\r\n`, the result must be freed.
    pub fn to_asc_c(&self, header: Option<(&str, &str)>) -> *mut libc::c_char {
        let headers = header.map(|(key, value)| {
            let mut m = BTreeMap::new();
            m.insert(key.to_string(), value.to_string());
            m
        });

        let buf = self
            .to_armored_string(headers.as_ref())
            .expect("failed to serialize key");
        let buf_c = CString::new(buf).unwrap();

        // need to use strdup to allocate the result with malloc
        // so it can be `free`d later.
        unsafe { strdup(buf_c.as_ptr()) }
    }

    pub fn write_asc_to_file(&self, file: *const libc::c_char, context: &Context) -> bool {
        if file.is_null() {
            return false;
        }

        let file_content = self.to_asc_c(None);

        let success = if 0
            == unsafe {
                dc_write_file(
                    context,
                    file,
                    file_content as *const libc::c_void,
                    strlen(file_content),
                )
            } {
            error!(context, 0, "Cannot write key to {}", to_string(file));
            false
        } else {
            true
        };

        unsafe { free(file_content as *mut libc::c_void) };

        success
    }

    pub fn fingerprint(&self) -> String {
        match self {
            Key::Public(k) => hex::encode_upper(k.fingerprint()),
            Key::Secret(k) => hex::encode_upper(k.fingerprint()),
        }
    }

    pub fn fingerprint_c(&self) -> *mut libc::c_char {
        let res = CString::new(self.fingerprint()).unwrap();

        unsafe { strdup(res.as_ptr()) }
    }

    pub fn formatted_fingerprint(&self) -> String {
        let rawhex = self.fingerprint();
        dc_format_fingerprint(&rawhex)
    }

    pub fn formatted_fingerprint_c(&self) -> *mut libc::c_char {
        let res = CString::new(self.formatted_fingerprint()).unwrap();

        unsafe { strdup(res.as_ptr()) }
    }

    pub fn split_key(&self) -> Option<Key> {
        match self {
            Key::Public(_) => None,
            Key::Secret(k) => {
                let pub_key = k.public_key();
                pub_key.sign(k, || "".into()).map(|k| Key::Public(k)).ok()
            }
        }
    }
}

pub fn dc_key_save_self_keypair(
    context: &Context,
    public_key: &Key,
    private_key: &Key,
    addr: *const libc::c_char,
    is_default: libc::c_int,
    sql: &SQLite,
) -> bool {
    if addr.is_null() {
        return false;
    }

    let stmt = unsafe {
        dc_sqlite3_prepare(
        context,
        sql,
        b"INSERT INTO keypairs (addr, is_default, public_key, private_key, created) VALUES (?,?,?,?,?);\x00"
            as *const u8 as *const libc::c_char
    )
    };

    unsafe {
        sqlite3_bind_text(stmt, 1, addr, -1, None);
        sqlite3_bind_int(stmt, 2, is_default)
    };
    let pub_bytes = public_key.to_bytes();
    let sec_bytes = private_key.to_bytes();
    unsafe {
        sqlite3_bind_blob(
            stmt,
            3,
            pub_bytes.as_ptr() as *const _,
            pub_bytes.len() as libc::c_int,
            None,
        )
    };
    unsafe {
        sqlite3_bind_blob(
            stmt,
            4,
            sec_bytes.as_ptr() as *const _,
            sec_bytes.len() as libc::c_int,
            None,
        )
    };
    unsafe { sqlite3_bind_int64(stmt, 5, time() as sqlite3_int64) };
    let success = if unsafe { sqlite3_step(stmt) } == 101 {
        true
    } else {
        false
    };

    unsafe { sqlite3_finalize(stmt) };

    success
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

pub fn dc_format_fingerprint_c(fp: *const libc::c_char) -> *mut libc::c_char {
    let input = unsafe { CStr::from_ptr(fp).to_str().unwrap() };
    let res = dc_format_fingerprint(input);
    let res_c = CString::new(res).unwrap();

    unsafe { strdup(res_c.as_ptr()) }
}

/// Bring a human-readable or otherwise formatted fingerprint back to the 40-characters-uppercase-hex format.
pub fn dc_normalize_fingerprint(fp: &str) -> String {
    fp.to_uppercase()
        .chars()
        .filter(|&c| c >= '0' && c <= '9' || c >= 'A' && c <= 'F')
        .collect()
}

pub fn dc_normalize_fingerprint_c(fp: *const libc::c_char) -> *mut libc::c_char {
    let input = unsafe { CStr::from_ptr(fp).to_str().unwrap() };
    let res = dc_normalize_fingerprint(input);
    let res_c = CString::new(res).unwrap();

    unsafe { strdup(res_c.as_ptr()) }
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
    fn test_from_base64() {
        let private_key = Key::from_base64(
            "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
            QCNYLJNVfJcrBDGvkDR9/bwEnw===GU1/
            ",
            KeyType::Private,
        )
        .expect("failed to decode");
        let binary = private_key.to_bytes();
        let private_key2 = Key::from_slice(&binary, KeyType::Private).expect("invalid private key");
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
        let (public_key, private_key) =
            crate::pgp::dc_pgp_create_keypair(CString::new("hello").unwrap().as_ptr()).unwrap();

        let binary = public_key.to_bytes();
        let public_key2 = Key::from_slice(&binary, KeyType::Public).expect("invalid public key");
        assert_eq!(public_key, public_key2);

        let binary = private_key.to_bytes();
        let private_key2 = Key::from_slice(&binary, KeyType::Private).expect("invalid private key");
        assert_eq!(private_key, private_key2);
    }
}
