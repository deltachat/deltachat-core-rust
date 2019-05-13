use std::collections::BTreeMap;
use std::ffi::{CStr, CString};
use std::io::Cursor;
use std::slice;

use libc;
use mmime::mailmime_content::*;
use mmime::mmapstring::*;
use mmime::other::*;
use pgp::composed::{Deserializable, SignedPublicKey, SignedSecretKey};
use pgp::ser::Serialize;

use crate::constants::*;
use crate::dc_context::dc_context_t;
use crate::dc_log::*;
use crate::dc_pgp::*;
use crate::dc_sqlite3::*;
use crate::dc_strbuilder::*;
use crate::dc_tools::*;
use crate::types::*;
use crate::x::*;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Key {
    Public(SignedPublicKey),
    Secret(SignedSecretKey),
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
        match key_type {
            KeyType::Public => SignedPublicKey::from_bytes(Cursor::new(bytes))
                .map(|k| Key::Public(k))
                .ok(),
            KeyType::Secret => SignedSecretKey::from_bytes(Cursor::new(bytes))
                .map(|k| Key::Secret(k))
                .ok(),
        }
    }

    pub fn from_binary(
        data: *const libc::c_void,
        len: libc::c_int,
        key_type: KeyType,
    ) -> Option<Self> {
        assert!(!data.is_null(), "missing data");
        assert!(len > 0);

        let bytes = unsafe { slice::from_raw_parts(data, len) };
        Self::from_slice(bytes, key_type)
    }

    pub fn from_stmt(
        stmt: *mut sqlite3_stmt,
        index: libc::c_int,
        key_type: KeyType,
    ) -> Option<Self> {
        assert!(!stmt.is_null(), "missing statement");

        let data = unsafe {
            sqlite3_column_blob(stmt, index) as *mut libc::c_uchar as *const libc::c_void
        };
        let len = unsafe { sqlite3_column_bytes(stmt, index) };

        Self::from_binary(data, len, key_type)
    }

    pub fn from_base64(encoded_data: &str, key_type: KeyType) -> Option<Self> {
        // TODO: strip newlines and other whitespace
        let bytes = encoded_data.as_bytes();

        base64::decode(bytes)
            .ok()
            .and_then(|decoded| Self::from_slice(&decoded, key_type))
    }

    pub fn from_self_public(
        context: &dc_context_t,
        self_addr: *const libc::c_char,
        sql: &dc_sqlite3_t,
    ) -> Option<Self> {
        if self_addr.is_null() {
            return None;
        }

        let stmt = dc_sqlite3_prepare(
            context,
            sql,
            b"SELECT public_key FROM keypairs WHERE addr=? AND is_default=1;\x00" as *const u8
                as *const libc::c_char,
        );
        sqlite3_bind_text(stmt, 1, self_addr, -1, None);

        let key = if sqlite3_step(stmt) == 100 {
            Self::from_stmt(stmt, 0, KeyType::Public);
        } else {
            None
        };

        sqlite3_finalize(stmt);

        key
    }

    pub fn from_self_private(
        context: &dc_context_t,
        self_addr: *const libc::c_char,
        sql: &dc_sqlite3_t,
    ) -> Option<Self> {
        if self_addr.is_null() {
            return None;
        }

        let stmt = dc_sqlite3_prepare(
            context,
            sql,
            b"SELECT private_key FROM keypairs WHERE addr=? AND is_default=1;\x00" as *const u8
                as *const libc::c_char,
        );
        sqlite3_bind_text(stmt, 1, self_addr, -1, None);

        let key = if sqlite3_step(stmt) == 100 {
            Self::from_stmt(stmt, 0, KeyType::Secret)
        } else {
            None
        };
        sqlite3_finalize(stmt);

        key
    }

    pub fn to_base64(&self, break_every: usize) -> String {
        let buf = self.0.to_bytes().expect("failed to serialize key");

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
        unsafe { libc::strdup(res_c.as_ptr()) }
    }

    /// Each header line must be terminated by `\r\n`, the result must be freed.
    pub fn to_asc_c(&self, header: Option<(&str, &str)>) -> *mut libc::c_char {
        let headers = header.map(|(key, value)| {
            let mut m = BTreeMap::new();
            m.insert(key.to_string(), value.to_string());
            m
        });

        let buf = self
            .0
            .to_armored_string(headers.as_ref())
            .expect("failed to serialize key");
        let buf_c = CString::new(buf).unwrap();

        // need to use strdup to allocate the result with malloc
        // so it can be `free`d later.
        unsafe { libc::strdup(buf_c.as_ptr()) }
    }

    pub fn write_asc_to_file(&self, file: *const libc::c_char, context: &dc_context_t) -> bool {
        if file.is_null() {
            return false;
        }

        let file_content = self.to_asc(None);

        let success = if 0
            == unsafe {
                dc_write_file(
                    context,
                    file,
                    file_content as *const libc::c_void,
                    strlen(file_content),
                )
            } {
            error!(context, 0, "Cannot write key to %s", file);
            false
        } else {
            true
        };

        free(file_content as *mut libc::c_void);

        success
    }

    pub fn fingerprint(&self) -> String {
        hex::encode_upper(self.0.fingerprint())
    }

    pub fn fingerprint_c(&self) -> *mut libc::c_char {
        let res = CString::new(self.fingerprint()).unwrap();

        unsafe { libc::strdup(res.as_ptr()) }
    }

    pub fn formatted_fingerprint(&self) -> String {
        let rawhex = self.fingerprint();
        dc_format_fingerprint(&rawhex)
    }

    pub fn formatted_fingerprint_c(&self) -> String {
        let res = CString::new(self.formatted_fingerprint()).unwrap();

        unsafe { libc::strdup(res.as_ptr()) }
    }
}

pub fn dc_key_save_self_keypair(
    context: &dc_context_t,
    public_key: &Key,
    private_key: &Key,
    addr: *const libc::c_char,
    is_default: libc::c_int,
    sql: &dc_sqlite3_t,
) -> bool {
    if addr.is_null() {
        return 0;
    }

    let stmt = dc_sqlite3_prepare(
        context,
        sql,
        b"INSERT INTO keypairs (addr, is_default, public_key, private_key, created) VALUES (?,?,?,?,?);\x00"
            as *const u8 as *const libc::c_char
    );

    sqlite3_bind_text(stmt, 1, addr, -1, None);
    sqlite3_bind_int(stmt, 2, is_default);
    let pub_bytes = public_key.to_bytes();
    let sec_bytes = private_key.to_bytes();
    sqlite3_bind_blob(stmt, 3, pub_bytes.as_ptr(), pub_bytes.len(), None);
    sqlite3_bind_blob(stmt, 4, sec_bytes.as_ptr(), sec_bytes.len(), None);
    sqlite3_bind_int64(stmt, 5, time(0 as *mut time_t) as sqlite3_int64);
    let success = if sqlite3_step(stmt) == 101 {
        true
    } else {
        false
    };

    sqlite3_finalize(stmt);

    success
}

/// Make a fingerprint human-readable, in hex format.
pub fn dc_format_fingerprint(fingerprint: &str) -> String {
    // split key into chunks of 4 with space, and 20 newline
    fingerprint
        .as_bytes()
        .chunks(4)
        .chunks(5)
        .map(|chunk| chunk.join(" "))
        .join("\n")
}

/// Bring a human-readable or otherwise formatted fingerprint back to the 40-characters-uppercase-hex format.
pub unsafe fn dc_normalize_fingerprint(fp: &str) -> String {
    fp.to_uppercase()
        .chars()
        .filter(|c| c >= '0' && c <= '9' || c >= 'A' && c <= 'F')
        .join("")
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
    fn test_format_fingerprint() {
        let fingerprint = dc_normalize_fingerprint("1234567890ABCDABCDEFABCDEF1234567890ABCD");

        assert_eq!(
            fingerprint,
            "1234 5678 90AB CDAB CDEF\nABCD EF12 3456 7890 ABCD"
        );
    }

}
