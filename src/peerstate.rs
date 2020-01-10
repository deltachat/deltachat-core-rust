//! # [Autocrypt Peer State](https://autocrypt.org/level1.html#peer-state-management) module
use std::collections::HashSet;
use std::fmt;

use num_traits::FromPrimitive;

use crate::aheader::*;
use crate::constants::*;
use crate::context::Context;
use crate::key::*;
use crate::sql::{self, Sql};

#[derive(Debug)]
pub enum PeerstateKeyType {
    GossipKey,
    PublicKey,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, FromPrimitive)]
#[repr(u8)]
pub enum PeerstateVerifiedStatus {
    Unverified = 0,
    //Verified = 1, // not used
    BidirectVerified = 2,
}

/// Peerstate represents the state of an Autocrypt peer.
pub struct Peerstate<'a> {
    pub context: &'a Context,
    pub addr: String,
    pub last_seen: i64,
    pub last_seen_autocrypt: i64,
    pub prefer_encrypt: EncryptPreference,
    pub public_key: Option<Key>,
    pub public_key_fingerprint: Option<String>,
    pub gossip_key: Option<Key>,
    pub gossip_timestamp: i64,
    pub gossip_key_fingerprint: Option<String>,
    pub verified_key: Option<Key>,
    pub verified_key_fingerprint: Option<String>,
    pub to_save: Option<ToSave>,
    pub degrade_event: Option<DegradeEvent>,
}

impl<'a> PartialEq for Peerstate<'a> {
    fn eq(&self, other: &Peerstate) -> bool {
        self.addr == other.addr
            && self.last_seen == other.last_seen
            && self.last_seen_autocrypt == other.last_seen_autocrypt
            && self.prefer_encrypt == other.prefer_encrypt
            && self.public_key == other.public_key
            && self.public_key_fingerprint == other.public_key_fingerprint
            && self.gossip_key == other.gossip_key
            && self.gossip_timestamp == other.gossip_timestamp
            && self.gossip_key_fingerprint == other.gossip_key_fingerprint
            && self.verified_key == other.verified_key
            && self.verified_key_fingerprint == other.verified_key_fingerprint
            && self.to_save == other.to_save
            && self.degrade_event == other.degrade_event
    }
}

impl<'a> Eq for Peerstate<'a> {}

impl<'a> fmt::Debug for Peerstate<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Peerstate")
            .field("addr", &self.addr)
            .field("last_seen", &self.last_seen)
            .field("last_seen_autocrypt", &self.last_seen_autocrypt)
            .field("prefer_encrypt", &self.prefer_encrypt)
            .field("public_key", &self.public_key)
            .field("public_key_fingerprint", &self.public_key_fingerprint)
            .field("gossip_key", &self.gossip_key)
            .field("gossip_timestamp", &self.gossip_timestamp)
            .field("gossip_key_fingerprint", &self.gossip_key_fingerprint)
            .field("verified_key", &self.verified_key)
            .field("verified_key_fingerprint", &self.verified_key_fingerprint)
            .field("to_save", &self.to_save)
            .field("degrade_event", &self.degrade_event)
            .finish()
    }
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

impl<'a> Peerstate<'a> {
    pub fn new(context: &'a Context, addr: String) -> Self {
        Peerstate {
            context,
            addr,
            last_seen: 0,
            last_seen_autocrypt: 0,
            prefer_encrypt: Default::default(),
            public_key: None,
            public_key_fingerprint: None,
            gossip_key: None,
            gossip_key_fingerprint: None,
            gossip_timestamp: 0,
            verified_key: None,
            verified_key_fingerprint: None,
            to_save: None,
            degrade_event: None,
        }
    }

    pub fn from_header(context: &'a Context, header: &Aheader, message_time: i64) -> Self {
        let mut res = Self::new(context, header.addr.clone());

        res.last_seen = message_time;
        res.last_seen_autocrypt = message_time;
        res.to_save = Some(ToSave::All);
        res.prefer_encrypt = header.prefer_encrypt;
        res.public_key = Some(header.public_key.clone());
        res.recalc_fingerprint();

        res
    }

    pub fn from_gossip(context: &'a Context, gossip_header: &Aheader, message_time: i64) -> Self {
        let mut res = Self::new(context, gossip_header.addr.clone());

        res.gossip_timestamp = message_time;
        res.to_save = Some(ToSave::All);
        res.gossip_key = Some(gossip_header.public_key.clone());
        res.recalc_fingerprint();

        res
    }

    pub fn from_addr(context: &'a Context, _sql: &Sql, addr: &str) -> Option<Self> {
        let query = "SELECT addr, last_seen, last_seen_autocrypt, prefer_encrypted, public_key, gossip_timestamp, gossip_key, public_key_fingerprint, gossip_key_fingerprint, verified_key, verified_key_fingerprint FROM acpeerstates  WHERE addr=? COLLATE NOCASE;";
        Self::from_stmt(context, query, &[addr])
    }

    pub fn from_fingerprint(context: &'a Context, _sql: &Sql, fingerprint: &str) -> Option<Self> {
        let query = "SELECT addr, last_seen, last_seen_autocrypt, prefer_encrypted, public_key, \
                     gossip_timestamp, gossip_key, public_key_fingerprint, gossip_key_fingerprint, \
                     verified_key, verified_key_fingerprint \
                     FROM acpeerstates  \
                     WHERE public_key_fingerprint=? COLLATE NOCASE \
                     OR gossip_key_fingerprint=? COLLATE NOCASE  \
                     ORDER BY public_key_fingerprint=? DESC;";

        Self::from_stmt(
            context,
            query,
            params![fingerprint, fingerprint, fingerprint],
        )
    }

    fn from_stmt<P>(context: &'a Context, query: &str, params: P) -> Option<Self>
    where
        P: IntoIterator,
        P::Item: rusqlite::ToSql,
    {
        context
            .sql
            .query_row(query, params, |row| {
                /* all the above queries start with this: SELECT
                addr, last_seen, last_seen_autocrypt, prefer_encrypted,
                public_key, gossip_timestamp, gossip_key, public_key_fingerprint,
                gossip_key_fingerprint, verified_key, verified_key_fingerprint
                */
                let mut res = Self::new(context, row.get(0)?);

                res.last_seen = row.get(1)?;
                res.last_seen_autocrypt = row.get(2)?;
                res.prefer_encrypt = EncryptPreference::from_i32(row.get(3)?).unwrap_or_default();
                res.gossip_timestamp = row.get(5)?;

                res.public_key_fingerprint = row.get(7)?;
                if res
                    .public_key_fingerprint
                    .as_ref()
                    .map(|s| s.is_empty())
                    .unwrap_or_default()
                {
                    res.public_key_fingerprint = None;
                }
                res.gossip_key_fingerprint = row.get(8)?;
                if res
                    .gossip_key_fingerprint
                    .as_ref()
                    .map(|s| s.is_empty())
                    .unwrap_or_default()
                {
                    res.gossip_key_fingerprint = None;
                }
                res.verified_key_fingerprint = row.get(10)?;
                if res
                    .verified_key_fingerprint
                    .as_ref()
                    .map(|s| s.is_empty())
                    .unwrap_or_default()
                {
                    res.verified_key_fingerprint = None;
                }
                res.public_key = row
                    .get(4)
                    .ok()
                    .and_then(|blob: Vec<u8>| Key::from_slice(&blob, KeyType::Public));
                res.gossip_key = row
                    .get(6)
                    .ok()
                    .and_then(|blob: Vec<u8>| Key::from_slice(&blob, KeyType::Public));
                res.verified_key = row
                    .get(9)
                    .ok()
                    .and_then(|blob: Vec<u8>| Key::from_slice(&blob, KeyType::Public));

                Ok(res)
            })
            .ok()
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

    pub fn degrade_encryption(&mut self, message_time: i64) {
        if self.prefer_encrypt == EncryptPreference::Mutual {
            self.degrade_event = Some(DegradeEvent::EncryptionPaused);
        }

        self.prefer_encrypt = EncryptPreference::Reset;
        self.last_seen = message_time;
        self.to_save = Some(ToSave::All);
    }

    pub fn apply_header(&mut self, header: &Aheader, message_time: i64) {
        if self.addr.to_lowercase() != header.addr.to_lowercase() {
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

    pub fn apply_gossip(&mut self, gossip_header: &Aheader, message_time: i64) {
        if self.addr.to_lowercase() != gossip_header.addr.to_lowercase() {
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

    pub fn render_gossip_header(&self, min_verified: PeerstateVerifiedStatus) -> Option<String> {
        if let Some(key) = self.peek_key(min_verified) {
            // TODO: avoid cloning
            let header = Aheader::new(
                self.addr.clone(),
                key.clone(),
                EncryptPreference::NoPreference,
            );
            Some(header.to_string())
        } else {
            None
        }
    }

    pub fn peek_key(&self, min_verified: PeerstateVerifiedStatus) -> Option<&Key> {
        if self.public_key.is_none() && self.gossip_key.is_none() && self.verified_key.is_none() {
            return None;
        }

        if min_verified != PeerstateVerifiedStatus::Unverified {
            return self.verified_key.as_ref();
        }
        if self.public_key.is_some() {
            return self.public_key.as_ref();
        }

        self.gossip_key.as_ref()
    }

    pub fn set_verified(
        &mut self,
        which_key: PeerstateKeyType,
        fingerprint: &str,
        verified: PeerstateVerifiedStatus,
    ) -> bool {
        if verified == PeerstateVerifiedStatus::BidirectVerified {
            match which_key {
                PeerstateKeyType::PublicKey => {
                    if self.public_key_fingerprint.is_some()
                        && self.public_key_fingerprint.as_ref().unwrap() == fingerprint
                    {
                        self.to_save = Some(ToSave::All);
                        self.verified_key = self.public_key.clone();
                        self.verified_key_fingerprint = self.public_key_fingerprint.clone();
                        true
                    } else {
                        false
                    }
                }
                PeerstateKeyType::GossipKey => {
                    if self.gossip_key_fingerprint.is_some()
                        && self.gossip_key_fingerprint.as_ref().unwrap() == fingerprint
                    {
                        self.to_save = Some(ToSave::All);
                        self.verified_key = self.gossip_key.clone();
                        self.verified_key_fingerprint = self.gossip_key_fingerprint.clone();
                        true
                    } else {
                        false
                    }
                }
            }
        } else {
            false
        }
    }

    pub fn save_to_db(&self, sql: &Sql, create: bool) -> crate::sql::Result<()> {
        if create {
            sql::execute(
                self.context,
                sql,
                "INSERT INTO acpeerstates (addr) VALUES(?);",
                params![self.addr],
            )?;
        }

        if self.to_save == Some(ToSave::All) || create {
            sql::execute(
                self.context,
                sql,
                "UPDATE acpeerstates \
                 SET last_seen=?, last_seen_autocrypt=?, prefer_encrypted=?, \
                 public_key=?, gossip_timestamp=?, gossip_key=?, public_key_fingerprint=?, gossip_key_fingerprint=?, \
                 verified_key=?, verified_key_fingerprint=? \
                 WHERE addr=?;",
                params![
                    self.last_seen,
                    self.last_seen_autocrypt,
                    self.prefer_encrypt as i64,
                    self.public_key.as_ref().map(|k| k.to_bytes()),
                    self.gossip_timestamp,
                    self.gossip_key.as_ref().map(|k| k.to_bytes()),
                    &self.public_key_fingerprint,
                    &self.gossip_key_fingerprint,
                    self.verified_key.as_ref().map(|k| k.to_bytes()),
                    &self.verified_key_fingerprint,
                    &self.addr,
                ],
            )?;
        } else if self.to_save == Some(ToSave::Timestamps) {
            sql::execute(
                self.context,
                sql,
                "UPDATE acpeerstates SET last_seen=?, last_seen_autocrypt=?, gossip_timestamp=? \
                 WHERE addr=?;",
                params![
                    self.last_seen,
                    self.last_seen_autocrypt,
                    self.gossip_timestamp,
                    &self.addr
                ],
            )?;
        }

        Ok(())
    }

    pub fn has_verified_key(&self, fingerprints: &HashSet<String>) -> bool {
        if self.verified_key.is_some() && self.verified_key_fingerprint.is_some() {
            let vkc = self.verified_key_fingerprint.as_ref().unwrap();
            if fingerprints.contains(vkc) {
                return true;
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    use tempfile::TempDir;

    #[test]
    fn test_peerstate_save_to_db() {
        let ctx = crate::test_utils::dummy_context();
        let addr = "hello@mail.com";

        let pub_key = crate::key::Key::from_base64(
            include_str!("../test-data/key/public.asc"),
            KeyType::Public,
        )
        .unwrap();

        let mut peerstate = Peerstate {
            context: &ctx.ctx,
            addr: addr.into(),
            last_seen: 10,
            last_seen_autocrypt: 11,
            prefer_encrypt: EncryptPreference::Mutual,
            public_key: Some(pub_key.clone()),
            public_key_fingerprint: Some(pub_key.fingerprint()),
            gossip_key: Some(pub_key.clone()),
            gossip_timestamp: 12,
            gossip_key_fingerprint: Some(pub_key.fingerprint()),
            verified_key: Some(pub_key.clone()),
            verified_key_fingerprint: Some(pub_key.fingerprint()),
            to_save: Some(ToSave::All),
            degrade_event: None,
        };

        assert!(
            peerstate.save_to_db(&ctx.ctx.sql, true).is_ok(),
            "failed to save to db"
        );

        let peerstate_new = Peerstate::from_addr(&ctx.ctx, &ctx.ctx.sql, addr)
            .expect("failed to load peerstate from db");

        // clear to_save, as that is not persissted
        peerstate.to_save = None;
        assert_eq!(peerstate, peerstate_new);
        let peerstate_new2 =
            Peerstate::from_fingerprint(&ctx.ctx, &ctx.ctx.sql, &pub_key.fingerprint())
                .expect("failed to load peerstate from db");
        assert_eq!(peerstate, peerstate_new2);
    }

    #[test]
    fn test_peerstate_double_create() {
        let ctx = crate::test_utils::dummy_context();
        let addr = "hello@mail.com";

        let pub_key = crate::key::Key::from_base64(
            include_str!("../test-data/key/public.asc"),
            KeyType::Public,
        )
        .unwrap();

        let peerstate = Peerstate {
            context: &ctx.ctx,
            addr: addr.into(),
            last_seen: 10,
            last_seen_autocrypt: 11,
            prefer_encrypt: EncryptPreference::Mutual,
            public_key: Some(pub_key.clone()),
            public_key_fingerprint: Some(pub_key.fingerprint()),
            gossip_key: None,
            gossip_timestamp: 12,
            gossip_key_fingerprint: None,
            verified_key: None,
            verified_key_fingerprint: None,
            to_save: Some(ToSave::All),
            degrade_event: None,
        };

        assert!(
            peerstate.save_to_db(&ctx.ctx.sql, true).is_ok(),
            "failed to save"
        );
        assert!(
            peerstate.save_to_db(&ctx.ctx.sql, true).is_ok(),
            "double-call with create failed"
        );
    }

    #[test]
    fn test_peerstate_with_empty_gossip_key_save_to_db() {
        let ctx = crate::test_utils::dummy_context();
        let addr = "hello@mail.com";

        let pub_key = crate::key::Key::from_base64(
            include_str!("../test-data/key/public.asc"),
            KeyType::Public,
        )
        .unwrap();

        let mut peerstate = Peerstate {
            context: &ctx.ctx,
            addr: addr.into(),
            last_seen: 10,
            last_seen_autocrypt: 11,
            prefer_encrypt: EncryptPreference::Mutual,
            public_key: Some(pub_key.clone()),
            public_key_fingerprint: Some(pub_key.fingerprint()),
            gossip_key: None,
            gossip_timestamp: 12,
            gossip_key_fingerprint: None,
            verified_key: None,
            verified_key_fingerprint: None,
            to_save: Some(ToSave::All),
            degrade_event: None,
        };

        assert!(
            peerstate.save_to_db(&ctx.ctx.sql, true).is_ok(),
            "failed to save"
        );

        let peerstate_new = Peerstate::from_addr(&ctx.ctx, &ctx.ctx.sql, addr)
            .expect("failed to load peerstate from db");

        // clear to_save, as that is not persissted
        peerstate.to_save = None;
        assert_eq!(peerstate, peerstate_new);
    }

    // TODO: don't copy this from stress.rs
    #[allow(dead_code)]
    struct TestContext {
        ctx: Context,
        dir: TempDir,
    }
}
