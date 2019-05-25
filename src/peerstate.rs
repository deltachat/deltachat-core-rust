use std::ffi::CString;

use num_traits::FromPrimitive;

use crate::constants::*;
use crate::dc_aheader::*;
use crate::dc_chat::*;
use crate::dc_context::dc_context_t;
use crate::dc_hash::*;
use crate::dc_key::*;
use crate::dc_sqlite3::*;
use crate::dc_tools::{to_cstring, to_string};
use crate::types::*;
use crate::x::*;

/// Peerstate represents the state of an Autocrypt peer.
pub struct Peerstate<'a> {
    pub context: &'a dc_context_t,
    pub addr: Option<String>,
    pub last_seen: u64,
    pub last_seen_autocrypt: u64,
    pub prefer_encrypt: EncryptPreference,
    pub public_key: Option<Key>,
    pub public_key_fingerprint: Option<String>,
    pub gossip_key: Option<Key>,
    pub gossip_timestamp: u64,
    pub gossip_key_fingerprint: Option<String>,
    verified_key: VerifiedKey,
    pub verified_key_fingerprint: Option<String>,
    pub to_save: Option<ToSave>,
    pub degrade_event: Option<DegradeEvent>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, FromPrimitive, ToPrimitive)]
#[repr(u8)]
pub enum ToSave {
    Timestamps = 0x01,
    All = 0x02,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, FromPrimitive, ToPrimitive)]
#[repr(u8)]
pub enum DegradeEvent {
    /// Recoverable by an incoming encrypted mail.
    EncryptionPaused = 0x01,
    /// Recoverable by a new verify.
    FingerprintChanged = 0x02,
}

#[derive(Debug, Copy, Clone)]
pub enum VerifiedKey {
    Gossip,
    Public,
    None,
}

impl Default for VerifiedKey {
    fn default() -> Self {
        VerifiedKey::None
    }
}

impl VerifiedKey {
    pub fn is_none(&self) -> bool {
        match self {
            VerifiedKey::None => true,
            _ => false,
        }
    }

    pub fn is_some(&self) -> bool {
        !self.is_none()
    }
}

impl<'a> Peerstate<'a> {
    pub fn new(context: &'a dc_context_t) -> Self {
        Peerstate {
            context,
            addr: None,
            last_seen: 0,
            last_seen_autocrypt: 0,
            prefer_encrypt: Default::default(),
            public_key: None,
            public_key_fingerprint: None,
            gossip_key: None,
            gossip_key_fingerprint: None,
            gossip_timestamp: 0,
            verified_key: Default::default(),
            verified_key_fingerprint: None,
            to_save: None,
            degrade_event: None,
        }
    }

    pub fn verified_key(&self) -> Option<&Key> {
        match self.verified_key {
            VerifiedKey::Public => self.public_key.as_ref(),
            VerifiedKey::Gossip => self.gossip_key.as_ref(),
            VerifiedKey::None => None,
        }
    }

    pub fn from_header(context: &'a dc_context_t, header: &Aheader, message_time: u64) -> Self {
        let mut res = Self::new(context);

        res.addr = Some(header.addr.clone());
        res.last_seen = message_time;
        res.last_seen_autocrypt = message_time;
        res.to_save = Some(ToSave::All);
        res.prefer_encrypt = header.prefer_encrypt;
        res.public_key = Some(header.public_key.clone());
        res.recalc_fingerprint();

        res
    }

    pub fn from_gossip(
        context: &'a dc_context_t,
        gossip_header: &Aheader,
        message_time: u64,
    ) -> Self {
        let mut res = Self::new(context);

        res.addr = Some(gossip_header.addr.clone());
        res.gossip_timestamp = message_time;
        res.to_save = Some(ToSave::All);
        res.gossip_key = Some(gossip_header.public_key.clone());
        res.recalc_fingerprint();

        res
    }

    pub fn from_addr(context: &'a dc_context_t, sql: &dc_sqlite3_t, addr: &str) -> Option<Self> {
        let mut res = None;

        let stmt = unsafe {
            dc_sqlite3_prepare(
                context,
                sql,
                b"SELECT addr, last_seen, last_seen_autocrypt, prefer_encrypted, public_key, gossip_timestamp, gossip_key, public_key_fingerprint, gossip_key_fingerprint, verified_key, verified_key_fingerprint FROM acpeerstates  WHERE addr=? COLLATE NOCASE;\x00"
                    as *const u8 as *const libc::c_char)
        };
        let addr_c = CString::new(addr.as_bytes()).unwrap();
        unsafe { sqlite3_bind_text(stmt, 1, addr_c.as_ptr(), -1, None) };
        if unsafe { sqlite3_step(stmt) } == 100 {
            res = Some(Self::from_stmt(context, stmt));
        }

        unsafe { sqlite3_finalize(stmt) };
        res
    }

    pub fn from_fingerprint(
        context: &'a dc_context_t,
        sql: &dc_sqlite3_t,
        fingerprint: &str,
    ) -> Option<Self> {
        let mut res = None;

        let stmt = unsafe {
            dc_sqlite3_prepare(
                context,
                sql,
                b"SELECT addr, last_seen, last_seen_autocrypt, prefer_encrypted, public_key, gossip_timestamp, gossip_key, public_key_fingerprint, gossip_key_fingerprint, verified_key, verified_key_fingerprint FROM acpeerstates  WHERE public_key_fingerprint=? COLLATE NOCASE     OR gossip_key_fingerprint=? COLLATE NOCASE  ORDER BY public_key_fingerprint=? DESC;\x00"
                    as *const u8 as *const libc::c_char)
        };

        let fp_c = CString::new(fingerprint.as_bytes()).unwrap();
        unsafe {
            sqlite3_bind_text(stmt, 1, fp_c.as_ptr(), -1, None);
            sqlite3_bind_text(stmt, 2, fp_c.as_ptr(), -1, None);
            sqlite3_bind_text(stmt, 3, fp_c.as_ptr(), -1, None);
        }
        if unsafe { sqlite3_step(stmt) == 100 } {
            res = Some(Self::from_stmt(context, stmt));
        }

        unsafe { sqlite3_finalize(stmt) };

        res
    }

    fn from_stmt(context: &'a dc_context_t, stmt: *mut sqlite3_stmt) -> Self {
        let mut res = Self::new(context);

        res.addr = Some(to_string(sqlite3_column_text(stmt, 0) as *const _));
        res.last_seen = unsafe { sqlite3_column_int64(stmt, 1) } as u64;
        res.last_seen_autocrypt = unsafe { sqlite3_column_int64(stmt, 2) } as u64;
        res.prefer_encrypt =
            EncryptPreference::from_i32(unsafe { sqlite3_column_int(stmt, 3) }).unwrap_or_default();
        res.gossip_timestamp = unsafe { sqlite3_column_int(stmt, 5) } as u64;
        res.public_key_fingerprint = Some(to_string(sqlite3_column_text(stmt, 7) as *const _));
        res.gossip_key_fingerprint = Some(to_string(sqlite3_column_text(stmt, 8) as *const _));
        res.verified_key_fingerprint = Some(to_string(sqlite3_column_text(stmt, 10) as *const _));

        if unsafe { sqlite3_column_type(stmt, 4) } != 5 {
            res.public_key = Key::from_stmt(stmt, 4, KeyType::Public);
        }
        if unsafe { sqlite3_column_type(stmt, 6) } != 5 {
            res.gossip_key = Key::from_stmt(stmt, 6, KeyType::Public);
        }
        if unsafe { sqlite3_column_type(stmt, 9) } != 5 {
            let vk = Key::from_stmt(stmt, 9, KeyType::Public);
            res.verified_key = if vk == res.gossip_key {
                VerifiedKey::Gossip
            } else if vk == res.public_key {
                VerifiedKey::Public
            } else {
                VerifiedKey::None
            };
        }

        res
    }

    pub fn recalc_fingerprint(&mut self) {
        if let Some(ref public_key) = self.public_key {
            let old_public_fingerprint = self.public_key_fingerprint.take();
            self.public_key_fingerprint = Some(public_key.fingerprint());

            if old_public_fingerprint.is_none()
                || self.public_key_fingerprint.is_none()
                || old_public_fingerprint != self.public_key_fingerprint
            {
                self.to_save = Some(ToSave::All);
                if old_public_fingerprint.is_some() {
                    self.degrade_event = Some(DegradeEvent::FingerprintChanged);
                }
            }
        }

        if let Some(ref gossip_key) = self.gossip_key {
            let old_gossip_fingerprint = self.gossip_key_fingerprint.take();
            self.gossip_key_fingerprint = Some(gossip_key.fingerprint());

            if old_gossip_fingerprint.is_none()
                || self.gossip_key_fingerprint.is_none()
                || old_gossip_fingerprint != self.gossip_key_fingerprint
            {
                self.to_save = Some(ToSave::All);
                if old_gossip_fingerprint.is_some() {
                    self.degrade_event = Some(DegradeEvent::FingerprintChanged);
                }
            }
        }
    }

    pub fn degrade_encryption(&mut self, message_time: u64) {
        if self.prefer_encrypt == EncryptPreference::Mutual {
            self.degrade_event = Some(DegradeEvent::EncryptionPaused);
        }

        self.prefer_encrypt = EncryptPreference::Reset;
        self.last_seen = message_time;
        self.to_save = Some(ToSave::All);
    }

    pub fn apply_header(&mut self, header: &Aheader, message_time: u64) {
        if self.addr.is_none()
            || self.addr.as_ref().unwrap().to_lowercase() != header.addr.to_lowercase()
        {
            return;
        }

        if message_time > self.last_seen_autocrypt {
            self.last_seen = message_time;
            self.last_seen_autocrypt = message_time;
            self.to_save = Some(ToSave::Timestamps);
            if (header.prefer_encrypt == EncryptPreference::Mutual
                || header.prefer_encrypt == EncryptPreference::NoPreference)
                && header.prefer_encrypt != self.prefer_encrypt
            {
                if self.prefer_encrypt == EncryptPreference::Mutual
                    && header.prefer_encrypt != EncryptPreference::Mutual
                {
                    self.degrade_event = Some(DegradeEvent::EncryptionPaused);
                }
                self.prefer_encrypt = header.prefer_encrypt;
                self.to_save = Some(ToSave::All)
            }

            if self.public_key.as_ref() != Some(&header.public_key) {
                self.public_key = Some(header.public_key.clone());
                self.recalc_fingerprint();
                self.to_save = Some(ToSave::All);
            }
        }
    }

    pub fn apply_gossip(&mut self, gossip_header: &Aheader, message_time: u64) {
        if self.addr.is_none()
            || self.addr.unwrap().to_lowercase() != gossip_header.addr.to_lowercase()
        {
            return;
        }

        if message_time > self.gossip_timestamp {
            self.gossip_timestamp = message_time;
            self.to_save = Some(ToSave::Timestamps);
            if self.gossip_key.as_ref() != Some(&gossip_header.public_key) {
                self.gossip_key = Some(gossip_header.public_key.clone());
                self.recalc_fingerprint();
                self.to_save = Some(ToSave::All)
            }
        };
    }

    pub fn render_gossip_header(&self, min_verified: usize) -> Option<String> {
        if let Some(addr) = self.addr {
            if let Some(key) = self.peek_key(min_verified) {
                // TODO: avoid cloning
                let header = Aheader::new(addr, key.clone(), EncryptPreference::NoPreference);
                return Some(header.to_string());
            }
        }

        None
    }

    pub fn peek_key(&self, min_verified: usize) -> Option<&Key> {
        if self.public_key.is_none() && self.gossip_key.is_none() && self.verified_key.is_none() {
            return None;
        }

        if 0 != min_verified {
            return self.verified_key();
        }
        if self.public_key.is_some() {
            return self.public_key.as_ref();
        }

        self.gossip_key.as_ref()
    }

    pub fn set_verified(&mut self, which_key: usize, fingerprint: &str, verified: usize) -> bool {
        let mut success = false;
        if !(which_key != 0 && which_key != 1 || verified != 2) {
            if which_key == 1
                && self.public_key_fingerprint.is_some()
                && self.public_key_fingerprint.as_ref().unwrap() == fingerprint
            {
                self.to_save = Some(ToSave::All);
                self.verified_key = VerifiedKey::Public;
                self.verified_key_fingerprint = self.public_key_fingerprint.clone();
                success = true;
            }
            if which_key == 0
                && self.gossip_key_fingerprint.is_some()
                && self.gossip_key_fingerprint.as_ref().unwrap() == fingerprint
            {
                self.to_save = Some(ToSave::All);
                self.verified_key = VerifiedKey::Gossip;
                self.verified_key_fingerprint = self.gossip_key_fingerprint.clone();
                success = true;
            }
        }

        success
    }

    pub fn save_to_db(&self, sql: &dc_sqlite3_t, create: bool) -> bool {
        let current_block: u64;
        let mut success = false;

        if self.addr.is_none() {
            return success;
        }

        if create {
            let stmt = unsafe {
                dc_sqlite3_prepare(
                    self.context,
                    sql,
                    b"INSERT INTO acpeerstates (addr) VALUES(?);\x00" as *const u8
                        as *const libc::c_char,
                )
            };
            let addr_c = CString::new(self.addr.as_ref().unwrap().as_bytes()).unwrap();
            unsafe {
                sqlite3_bind_text(stmt, 1, addr_c.as_ptr(), -1, None);
                sqlite3_step(stmt);
                sqlite3_finalize(stmt);
            }
        }

        if self.to_save == Some(ToSave::All) || create {
            let stmt = unsafe {
                dc_sqlite3_prepare(
                    self.context,sql,
                    b"UPDATE acpeerstates    SET last_seen=?, last_seen_autocrypt=?, prefer_encrypted=?,        public_key=?, gossip_timestamp=?, gossip_key=?, public_key_fingerprint=?, gossip_key_fingerprint=?, verified_key=?, verified_key_fingerprint=?  WHERE addr=?;\x00"
                        as *const u8 as *const libc::c_char)
            };

            unsafe {
                sqlite3_bind_int64(stmt, 1, self.last_seen as sqlite3_int64);
                sqlite3_bind_int64(stmt, 2, self.last_seen_autocrypt as sqlite3_int64);
                sqlite3_bind_int64(stmt, 3, self.prefer_encrypt as sqlite3_int64);
            }

            let addr_c = CString::new(self.addr.as_ref().unwrap().as_bytes()).unwrap();
            let pub_bytes = self.public_key.as_ref().map(|k| k.to_bytes());
            let gossip_bytes = self.gossip_key.as_ref().map(|k| k.to_bytes());
            let ver_bytes = self.verified_key().map(|k| k.to_bytes());

            unsafe {
                sqlite3_bind_blob(
                    stmt,
                    4,
                    pub_bytes
                        .as_ref()
                        .map(|b| b.as_ptr())
                        .unwrap_or_else(|| std::ptr::null()) as *const _,
                    pub_bytes.as_ref().map(|b| b.len()).unwrap_or_else(|| 0) as libc::c_int,
                    None,
                );
                sqlite3_bind_int64(stmt, 5, self.gossip_timestamp as sqlite3_int64);
                sqlite3_bind_blob(
                    stmt,
                    6,
                    gossip_bytes
                        .as_ref()
                        .map(|b| b.as_ptr())
                        .unwrap_or_else(|| std::ptr::null()) as *const _,
                    gossip_bytes.as_ref().map(|b| b.len()).unwrap_or_else(|| 0) as libc::c_int,
                    None,
                );
                let pkc = self
                    .public_key_fingerprint
                    .as_ref()
                    .map(|fp| to_cstring(fp));
                let gkc = self
                    .gossip_key_fingerprint
                    .as_ref()
                    .map(|fp| to_cstring(fp));

                sqlite3_bind_text(
                    stmt,
                    7,
                    pkc.map(|fp| fp.as_ptr())
                        .unwrap_or_else(|| std::ptr::null()),
                    -1,
                    None,
                );
                sqlite3_bind_text(
                    stmt,
                    8,
                    gkc.map(|fp| fp.as_ptr())
                        .unwrap_or_else(|| std::ptr::null()),
                    -1,
                    None,
                );
                sqlite3_bind_blob(
                    stmt,
                    9,
                    ver_bytes
                        .as_ref()
                        .map(|b| b.as_ptr())
                        .unwrap_or_else(|| std::ptr::null()) as *const _,
                    ver_bytes.as_ref().map(|b| b.len()).unwrap_or_else(|| 0) as libc::c_int,
                    None,
                );

                let vkc = self
                    .verified_key_fingerprint
                    .as_ref()
                    .map(|fp| to_cstring(fp));
                let addr_c = self.addr.as_ref().map(|addr| to_cstring(addr));

                sqlite3_bind_text(
                    stmt,
                    10,
                    vkc.map(|fp| fp.as_ptr())
                        .unwrap_or_else(|| std::ptr::null()),
                    -1,
                    None,
                );
                sqlite3_bind_text(
                    stmt,
                    11,
                    addr_c
                        .map(|addr| addr.as_ptr())
                        .unwrap_or_else(|| std::ptr::null()),
                    -1,
                    None,
                );
            }

            if unsafe { sqlite3_step(stmt) } == 101 {
                success = true;
            }

            unsafe { sqlite3_finalize(stmt) };
        } else if self.to_save == Some(ToSave::Timestamps) {
            let stmt = unsafe {
                dc_sqlite3_prepare(
                    self.context,sql,
                    b"UPDATE acpeerstates SET last_seen=?, last_seen_autocrypt=?, gossip_timestamp=? WHERE addr=?;\x00"
                        as *const u8 as *const libc::c_char)
            };

            let addr_c = self.addr.as_ref().map(|fp| to_cstring(fp));

            unsafe {
                sqlite3_bind_int64(stmt, 1, self.last_seen as sqlite3_int64);
                sqlite3_bind_int64(stmt, 2, self.last_seen_autocrypt as sqlite3_int64);
                sqlite3_bind_int64(stmt, 3, self.gossip_timestamp as sqlite3_int64);
                sqlite3_bind_text(
                    stmt,
                    4,
                    addr_c
                        .map(|addr| addr.as_ptr())
                        .unwrap_or_else(|| std::ptr::null()),
                    -1,
                    None,
                );
            }

            if unsafe { sqlite3_step(stmt) } == 101 {
                success = true;
            }

            unsafe { sqlite3_finalize(stmt) };
        }

        if self.to_save == Some(ToSave::All) || create {
            unsafe { dc_reset_gossiped_timestamp(self.context, 0 as uint32_t) };
        }

        success
    }

    pub fn has_verified_key(&self, fingerprints: *const dc_hash_t) -> bool {
        if fingerprints.is_null() {
            return false;
        }

        if self.verified_key.is_some() && self.verified_key_fingerprint.is_some() {
            let vkc = to_cstring(self.verified_key_fingerprint.as_ref().unwrap());
            if !unsafe {
                dc_hash_find(
                    fingerprints,
                    vkc.as_ptr() as *const libc::c_void,
                    strlen(vkc.as_ptr()) as libc::c_int,
                )
                .is_null()
            } {
                return true;
            }
        }

        false
    }
}
