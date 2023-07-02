//! # [Autocrypt Peer State](https://autocrypt.org/level1.html#peer-state-management) module.

use std::collections::HashSet;

use anyhow::{Context as _, Error, Result};
use num_traits::FromPrimitive;

use crate::aheader::{Aheader, EncryptPreference};
use crate::chat::{self, Chat};
use crate::chatlist::Chatlist;
use crate::constants::Chattype;
use crate::contact::{addr_cmp, Contact, ContactAddress, Origin};
use crate::context::Context;
use crate::events::EventType;
use crate::key::{DcKey, Fingerprint, SignedPublicKey};
use crate::message::Message;
use crate::mimeparser::SystemMessage;
use crate::sql::Sql;
use crate::stock_str;

/// Type of the public key stored inside the peerstate.
#[derive(Debug)]
pub enum PeerstateKeyType {
    /// Pubilc key sent in the `Autocrypt-Gossip` header.
    GossipKey,

    /// Public key sent in the `Autocrypt` header.
    PublicKey,
}

/// Verification status of the contact peerstate.
#[derive(Debug, PartialEq, Eq, Clone, Copy, FromPrimitive)]
#[repr(u8)]
pub enum PeerstateVerifiedStatus {
    /// Peerstate is not verified.
    Unverified = 0,
    //Verified = 1, // not used
    /// Peerstate is verified and we assume that the contact has verified our peerstate.
    BidirectVerified = 2,
}

/// Peerstate represents the state of an Autocrypt peer.
#[derive(Debug, PartialEq, Eq)]
pub struct Peerstate {
    /// E-mail address of the contact.
    pub addr: String,

    /// Timestamp of the latest peerstate update.
    ///
    /// Updated when a message is received from a contact,
    /// either with or without `Autocrypt` header.
    pub last_seen: i64,

    /// Timestamp of the latest `Autocrypt` header reception.
    pub last_seen_autocrypt: i64,

    /// Encryption preference of the contact.
    pub prefer_encrypt: EncryptPreference,

    /// Public key of the contact received in `Autocrypt` header.
    pub public_key: Option<SignedPublicKey>,

    /// Fingerprint of the contact public key.
    pub public_key_fingerprint: Option<Fingerprint>,

    /// Public key of the contact received in `Autocrypt-Gossip` header.
    pub gossip_key: Option<SignedPublicKey>,

    /// Timestamp of the latest `Autocrypt-Gossip` header reception.
    ///
    /// It is stored to avoid applying outdated gossiped key
    /// from delayed or reordered messages.
    pub gossip_timestamp: i64,

    /// Fingerprint of the contact gossip key.
    pub gossip_key_fingerprint: Option<Fingerprint>,

    /// Public key of the contact at the time it was verified,
    /// either directly or via gossip from the verified contact.
    pub verified_key: Option<SignedPublicKey>,

    /// Fingerprint of the verified public key.
    pub verified_key_fingerprint: Option<Fingerprint>,

    /// True if it was detected
    /// that the fingerprint of the key used in chats with
    /// opportunistic encryption was changed after Peerstate creation.
    pub fingerprint_changed: bool,

    /// The address that verified this contact
    pub verifier: Option<String>,
}

impl Peerstate {
    /// Creates a peerstate from the `Autocrypt` header.
    pub fn from_header(header: &Aheader, message_time: i64) -> Self {
        Peerstate {
            addr: header.addr.clone(),
            last_seen: message_time,
            last_seen_autocrypt: message_time,
            prefer_encrypt: header.prefer_encrypt,
            public_key: Some(header.public_key.clone()),
            public_key_fingerprint: Some(header.public_key.fingerprint()),
            gossip_key: None,
            gossip_key_fingerprint: None,
            gossip_timestamp: 0,
            verified_key: None,
            verified_key_fingerprint: None,
            fingerprint_changed: false,
            verifier: None,
        }
    }

    /// Create a peerstate from the `Autocrypt-Gossip` header.
    pub fn from_gossip(gossip_header: &Aheader, message_time: i64) -> Self {
        Peerstate {
            addr: gossip_header.addr.clone(),
            last_seen: 0,
            last_seen_autocrypt: 0,

            // Non-standard extension. According to Autocrypt 1.1.0 gossip headers SHOULD NOT
            // contain encryption preference.
            //
            // Delta Chat includes encryption preference to ensure new users introduced to a group
            // learn encryption preferences of other members immediately and don't send unencrypted
            // messages to a group where everyone prefers encryption.
            prefer_encrypt: gossip_header.prefer_encrypt,
            public_key: None,
            public_key_fingerprint: None,
            gossip_key: Some(gossip_header.public_key.clone()),
            gossip_key_fingerprint: Some(gossip_header.public_key.fingerprint()),
            gossip_timestamp: message_time,
            verified_key: None,
            verified_key_fingerprint: None,
            fingerprint_changed: false,
            verifier: None,
        }
    }

    /// Loads peerstate corresponding to the given address from the database.
    pub async fn from_addr(context: &Context, addr: &str) -> Result<Option<Peerstate>> {
        let query = "SELECT addr, last_seen, last_seen_autocrypt, prefer_encrypted, public_key, \
                     gossip_timestamp, gossip_key, public_key_fingerprint, gossip_key_fingerprint, \
                     verified_key, verified_key_fingerprint, verifier \
                     FROM acpeerstates \
                     WHERE addr=? COLLATE NOCASE LIMIT 1;";
        Self::from_stmt(context, query, (addr,)).await
    }

    /// Loads peerstate corresponding to the given fingerprint from the database.
    pub async fn from_fingerprint(
        context: &Context,
        fingerprint: &Fingerprint,
    ) -> Result<Option<Peerstate>> {
        let query = "SELECT addr, last_seen, last_seen_autocrypt, prefer_encrypted, public_key, \
                     gossip_timestamp, gossip_key, public_key_fingerprint, gossip_key_fingerprint, \
                     verified_key, verified_key_fingerprint, verifier \
                     FROM acpeerstates  \
                     WHERE public_key_fingerprint=? \
                     OR gossip_key_fingerprint=? \
                     ORDER BY public_key_fingerprint=? DESC LIMIT 1;";
        let fp = fingerprint.hex();
        Self::from_stmt(context, query, (&fp, &fp, &fp)).await
    }

    /// Loads peerstate by address or verified fingerprint.
    ///
    /// If the address is different but verified fingerprint is the same,
    /// peerstate with corresponding verified fingerprint is preferred.
    pub async fn from_verified_fingerprint_or_addr(
        context: &Context,
        fingerprint: &Fingerprint,
        addr: &str,
    ) -> Result<Option<Peerstate>> {
        let query = "SELECT addr, last_seen, last_seen_autocrypt, prefer_encrypted, public_key, \
                     gossip_timestamp, gossip_key, public_key_fingerprint, gossip_key_fingerprint, \
                     verified_key, verified_key_fingerprint, verifier \
                     FROM acpeerstates  \
                     WHERE verified_key_fingerprint=? \
                     OR addr=? COLLATE NOCASE \
                     ORDER BY verified_key_fingerprint=? DESC, last_seen DESC LIMIT 1;";
        let fp = fingerprint.hex();
        Self::from_stmt(context, query, (&fp, &addr, &fp)).await
    }

    async fn from_stmt(
        context: &Context,
        query: &str,
        params: impl rusqlite::Params + Send,
    ) -> Result<Option<Peerstate>> {
        let peerstate = context
            .sql
            .query_row_optional(query, params, |row| {
                // all the above queries start with this: SELECT
                //   addr, last_seen, last_seen_autocrypt, prefer_encrypted,
                //   public_key, gossip_timestamp, gossip_key, public_key_fingerprint,
                //   gossip_key_fingerprint, verified_key, verified_key_fingerprint

                let res = Peerstate {
                    addr: row.get("addr")?,
                    last_seen: row.get("last_seen")?,
                    last_seen_autocrypt: row.get("last_seen_autocrypt")?,
                    prefer_encrypt: EncryptPreference::from_i32(row.get("prefer_encrypted")?)
                        .unwrap_or_default(),
                    public_key: row
                        .get("public_key")
                        .ok()
                        .and_then(|blob: Vec<u8>| SignedPublicKey::from_slice(&blob).ok()),
                    public_key_fingerprint: row
                        .get::<_, Option<String>>("public_key_fingerprint")?
                        .map(|s| s.parse::<Fingerprint>())
                        .transpose()
                        .unwrap_or_default(),
                    gossip_key: row
                        .get("gossip_key")
                        .ok()
                        .and_then(|blob: Vec<u8>| SignedPublicKey::from_slice(&blob).ok()),
                    gossip_key_fingerprint: row
                        .get::<_, Option<String>>("gossip_key_fingerprint")?
                        .map(|s| s.parse::<Fingerprint>())
                        .transpose()
                        .unwrap_or_default(),
                    gossip_timestamp: row.get("gossip_timestamp")?,
                    verified_key: row
                        .get("verified_key")
                        .ok()
                        .and_then(|blob: Vec<u8>| SignedPublicKey::from_slice(&blob).ok()),
                    verified_key_fingerprint: row
                        .get::<_, Option<String>>("verified_key_fingerprint")?
                        .map(|s| s.parse::<Fingerprint>())
                        .transpose()
                        .unwrap_or_default(),
                    fingerprint_changed: false,
                    verifier: {
                        let verifier: Option<String> = row.get("verifier")?;
                        verifier.filter(|verifier| !verifier.is_empty())
                    },
                };

                Ok(res)
            })
            .await?;
        Ok(peerstate)
    }

    /// Re-calculate `self.public_key_fingerprint` and `self.gossip_key_fingerprint`.
    /// If one of them was changed, `self.fingerprint_changed` is set to `true`.
    ///
    /// Call this after you changed `self.public_key` or `self.gossip_key`.
    pub fn recalc_fingerprint(&mut self) {
        if let Some(ref public_key) = self.public_key {
            let old_public_fingerprint = self.public_key_fingerprint.take();
            self.public_key_fingerprint = Some(public_key.fingerprint());

            if old_public_fingerprint.is_some()
                && old_public_fingerprint != self.public_key_fingerprint
            {
                self.fingerprint_changed = true;
            }
        }

        if let Some(ref gossip_key) = self.gossip_key {
            let old_gossip_fingerprint = self.gossip_key_fingerprint.take();
            self.gossip_key_fingerprint = Some(gossip_key.fingerprint());

            if old_gossip_fingerprint.is_none()
                || self.gossip_key_fingerprint.is_none()
                || old_gossip_fingerprint != self.gossip_key_fingerprint
            {
                // Warn about gossip key change only if there is no public key obtained from
                // Autocrypt header, which overrides gossip key.
                if old_gossip_fingerprint.is_some() && self.public_key_fingerprint.is_none() {
                    self.fingerprint_changed = true;
                }
            }
        }
    }

    /// Reset Autocrypt peerstate.
    ///
    /// Used when it is detected that the contact no longer uses Autocrypt.
    pub fn degrade_encryption(&mut self, message_time: i64) {
        self.prefer_encrypt = EncryptPreference::Reset;
        self.last_seen = message_time;
    }

    /// Updates peerstate according to the given `Autocrypt` header.
    pub fn apply_header(&mut self, header: &Aheader, message_time: i64) {
        if !addr_cmp(&self.addr, &header.addr) {
            return;
        }

        if message_time > self.last_seen {
            self.last_seen = message_time;
            self.last_seen_autocrypt = message_time;
            if (header.prefer_encrypt == EncryptPreference::Mutual
                || header.prefer_encrypt == EncryptPreference::NoPreference)
                && header.prefer_encrypt != self.prefer_encrypt
            {
                self.prefer_encrypt = header.prefer_encrypt;
            }

            if self.public_key.as_ref() != Some(&header.public_key) {
                self.public_key = Some(header.public_key.clone());
                self.recalc_fingerprint();
            }
        }
    }

    /// Updates peerstate according to the given `Autocrypt-Gossip` header.
    pub fn apply_gossip(&mut self, gossip_header: &Aheader, message_time: i64) {
        if self.addr.to_lowercase() != gossip_header.addr.to_lowercase() {
            return;
        }

        if message_time > self.gossip_timestamp {
            self.gossip_timestamp = message_time;
            if self.gossip_key.as_ref() != Some(&gossip_header.public_key) {
                self.gossip_key = Some(gossip_header.public_key.clone());
                self.recalc_fingerprint();
            }

            // This is non-standard.
            //
            // According to Autocrypt 1.1.0 gossip headers SHOULD NOT
            // contain encryption preference, but we include it into
            // Autocrypt-Gossip and apply it one way (from
            // "nopreference" to "mutual").
            //
            // This is compatible to standard clients, because they
            // can't distinguish it from the case where we have
            // contacted the client in the past and received this
            // preference via Autocrypt header.
            if self.last_seen_autocrypt == 0
                && self.prefer_encrypt == EncryptPreference::NoPreference
                && gossip_header.prefer_encrypt == EncryptPreference::Mutual
            {
                self.prefer_encrypt = EncryptPreference::Mutual;
            }
        };
    }

    /// Returns the contents of the `Autocrypt-Gossip` header for outgoing messages.
    pub fn render_gossip_header(&self, min_verified: PeerstateVerifiedStatus) -> Option<String> {
        if let Some(key) = self.peek_key(min_verified) {
            let header = Aheader::new(
                self.addr.clone(),
                key.clone(), // TODO: avoid cloning
                // Autocrypt 1.1.0 specification says that
                // `prefer-encrypt` attribute SHOULD NOT be included,
                // but we include it anyway to propagate encryption
                // preference to new members in group chats.
                if self.last_seen_autocrypt > 0 {
                    self.prefer_encrypt
                } else {
                    EncryptPreference::NoPreference
                },
            );
            Some(header.to_string())
        } else {
            None
        }
    }

    /// Converts the peerstate into the contact public key.
    ///
    /// Similar to [`Self::peek_key`], but consumes the peerstate and returns owned key.
    pub fn take_key(mut self, min_verified: PeerstateVerifiedStatus) -> Option<SignedPublicKey> {
        match min_verified {
            PeerstateVerifiedStatus::BidirectVerified => self.verified_key.take(),
            PeerstateVerifiedStatus::Unverified => {
                self.public_key.take().or_else(|| self.gossip_key.take())
            }
        }
    }

    /// Returns a reference to the contact public key.
    ///
    /// `min_verified` determines the minimum required verification status of the key.
    /// If verified key is requested, returns the verified key,
    /// otherwise returns the Autocrypt key.
    ///
    /// Returned key is suitable for sending in `Autocrypt-Gossip` header.
    ///
    /// Returns `None` if there is no suitable public key.
    pub fn peek_key(&self, min_verified: PeerstateVerifiedStatus) -> Option<&SignedPublicKey> {
        match min_verified {
            PeerstateVerifiedStatus::BidirectVerified => self.verified_key.as_ref(),
            PeerstateVerifiedStatus::Unverified => {
                self.public_key.as_ref().or(self.gossip_key.as_ref())
            }
        }
    }

    /// Set this peerstate to verified
    /// Make sure to call `self.save_to_db` to save these changes
    /// Params:
    /// verifier:
    ///   The address which verifies the given contact
    ///   If we are verifying the contact, use that contacts address
    pub fn set_verified(
        &mut self,
        which_key: PeerstateKeyType,
        fingerprint: Fingerprint,
        verified: PeerstateVerifiedStatus,
        verifier: String,
    ) -> Result<()> {
        if verified == PeerstateVerifiedStatus::BidirectVerified {
            match which_key {
                PeerstateKeyType::PublicKey => {
                    if self.public_key_fingerprint.is_some()
                        && self.public_key_fingerprint.as_ref().unwrap() == &fingerprint
                    {
                        self.verified_key = self.public_key.clone();
                        self.verified_key_fingerprint = Some(fingerprint);
                        self.verifier = Some(verifier);
                        Ok(())
                    } else {
                        Err(Error::msg(format!(
                            "{fingerprint} is not peer's public key fingerprint",
                        )))
                    }
                }
                PeerstateKeyType::GossipKey => {
                    if self.gossip_key_fingerprint.is_some()
                        && self.gossip_key_fingerprint.as_ref().unwrap() == &fingerprint
                    {
                        self.verified_key = self.gossip_key.clone();
                        self.verified_key_fingerprint = Some(fingerprint);
                        self.verifier = Some(verifier);
                        Ok(())
                    } else {
                        Err(Error::msg(format!(
                            "{fingerprint} is not peer's gossip key fingerprint",
                        )))
                    }
                }
            }
        } else {
            Err(Error::msg("BidirectVerified required"))
        }
    }

    /// Saves the peerstate to the database.
    pub async fn save_to_db(&self, sql: &Sql) -> Result<()> {
        sql.execute(
            "INSERT INTO acpeerstates (
                last_seen,
                last_seen_autocrypt,
                prefer_encrypted,
                public_key,
                gossip_timestamp,
                gossip_key,
                public_key_fingerprint,
                gossip_key_fingerprint,
                verified_key,
                verified_key_fingerprint,
                addr,
                verifier)
                VALUES (?,?,?,?,?,?,?,?,?,?,?,?)
                ON CONFLICT (addr)
                DO UPDATE SET
                  last_seen = excluded.last_seen,
                  last_seen_autocrypt = excluded.last_seen_autocrypt,
                  prefer_encrypted = excluded.prefer_encrypted,
                  public_key = excluded.public_key,
                  gossip_timestamp = excluded.gossip_timestamp,
                  gossip_key = excluded.gossip_key,
                  public_key_fingerprint = excluded.public_key_fingerprint,
                  gossip_key_fingerprint = excluded.gossip_key_fingerprint,
                  verified_key = excluded.verified_key,
                  verified_key_fingerprint = excluded.verified_key_fingerprint,
                  verifier = excluded.verifier",
            (
                self.last_seen,
                self.last_seen_autocrypt,
                self.prefer_encrypt as i64,
                self.public_key.as_ref().map(|k| k.to_bytes()),
                self.gossip_timestamp,
                self.gossip_key.as_ref().map(|k| k.to_bytes()),
                self.public_key_fingerprint.as_ref().map(|fp| fp.hex()),
                self.gossip_key_fingerprint.as_ref().map(|fp| fp.hex()),
                self.verified_key.as_ref().map(|k| k.to_bytes()),
                self.verified_key_fingerprint.as_ref().map(|fp| fp.hex()),
                &self.addr,
                self.verifier.as_deref().unwrap_or(""),
            ),
        )
        .await?;
        Ok(())
    }

    /// Returns true if verified key is contained in the given set of fingerprints.
    pub fn has_verified_key(&self, fingerprints: &HashSet<Fingerprint>) -> bool {
        if let Some(vkc) = &self.verified_key_fingerprint {
            fingerprints.contains(vkc) && self.verified_key.is_some()
        } else {
            false
        }
    }

    /// Returns the address that verified the contact
    pub fn get_verifier(&self) -> Option<&str> {
        self.verifier.as_deref()
    }

    /// Add an info message to all the chats with this contact, informing about
    /// a [`PeerstateChange`].
    ///
    /// Also, in the case of an address change (AEAP), replace the old address
    /// with the new address in all chats.
    async fn handle_setup_change(
        &self,
        context: &Context,
        timestamp: i64,
        change: PeerstateChange,
    ) -> Result<()> {
        if context.is_self_addr(&self.addr).await? {
            // Do not try to search all the chats with self.
            return Ok(());
        }

        let contact_id = context
            .sql
            .query_get_value(
                "SELECT id FROM contacts WHERE addr=? COLLATE NOCASE;",
                (&self.addr,),
            )
            .await?
            .with_context(|| format!("contact with peerstate.addr {:?} not found", &self.addr))?;

        let chats = Chatlist::try_load(context, 0, None, Some(contact_id)).await?;
        let msg = match &change {
            PeerstateChange::FingerprintChange => {
                stock_str::contact_setup_changed(context, &self.addr).await
            }
            PeerstateChange::Aeap(new_addr) => {
                let old_contact = Contact::get_by_id(context, contact_id).await?;
                stock_str::aeap_addr_changed(
                    context,
                    old_contact.get_display_name(),
                    &self.addr,
                    new_addr,
                )
                .await
            }
        };
        for (chat_id, msg_id) in chats.iter() {
            let timestamp_sort = if let Some(msg_id) = msg_id {
                let lastmsg = Message::load_from_db(context, *msg_id).await?;
                lastmsg.timestamp_sort
            } else {
                context
                    .sql
                    .query_get_value(
                        "SELECT created_timestamp FROM chats WHERE id=?;",
                        (chat_id,),
                    )
                    .await?
                    .unwrap_or(0)
            };

            if let PeerstateChange::Aeap(new_addr) = &change {
                let chat = Chat::load_from_db(context, *chat_id).await?;

                if chat.typ == Chattype::Group && !chat.is_protected() {
                    // Don't add an info_msg to the group, in order not to make the user think
                    // that the address was automatically replaced in the group.
                    continue;
                }

                // For security reasons, for now, we only do the AEAP transition if the fingerprint
                // is verified (that's what from_verified_fingerprint_or_addr() does).
                // In order to not have inconsistent group membership state, we then only do the
                // transition in verified groups and in broadcast lists.
                if (chat.typ == Chattype::Group && chat.is_protected())
                    || chat.typ == Chattype::Broadcast
                {
                    match ContactAddress::new(new_addr) {
                        Ok(new_addr) => {
                            let (new_contact_id, _) = Contact::add_or_lookup(
                                context,
                                "",
                                new_addr,
                                Origin::IncomingUnknownFrom,
                            )
                            .await?;
                            chat::remove_from_chat_contacts_table(context, *chat_id, contact_id)
                                .await?;
                            chat::add_to_chat_contacts_table(context, *chat_id, &[new_contact_id])
                                .await?;

                            context.emit_event(EventType::ChatModified(*chat_id));
                        }
                        Err(err) => {
                            warn!(
                                context,
                                "New address {:?} is not valid, not doing AEAP: {:#}.",
                                new_addr,
                                err
                            )
                        }
                    }
                }
            }

            chat::add_info_msg_with_cmd(
                context,
                *chat_id,
                &msg,
                SystemMessage::Unknown,
                timestamp_sort,
                Some(timestamp),
                None,
                None,
            )
            .await?;
        }

        Ok(())
    }

    /// Adds a warning to all the chats corresponding to peerstate if fingerprint has changed.
    pub(crate) async fn handle_fingerprint_change(
        &self,
        context: &Context,
        timestamp: i64,
    ) -> Result<()> {
        if self.fingerprint_changed {
            self.handle_setup_change(context, timestamp, PeerstateChange::FingerprintChange)
                .await?;
        }
        Ok(())
    }
}

/// Do an AEAP transition, if necessary.
/// AEAP stands for "Automatic Email Address Porting."
///
/// In `drafts/aeap_mvp.md` there is a "big picture" overview over AEAP.
pub(crate) async fn maybe_do_aeap_transition(
    context: &Context,
    mime_parser: &mut crate::mimeparser::MimeMessage,
) -> Result<()> {
    let info = &mime_parser.decryption_info;
    if let Some(peerstate) = &info.peerstate {
        // If the from addr is different from the peerstate address we know,
        // we may want to do an AEAP transition.
        if !addr_cmp(&peerstate.addr, &mime_parser.from.addr)
                // Check if it's a chat message; we do this to avoid
                // some accidental transitions if someone writes from multiple
                // addresses with an MUA.
                && mime_parser.has_chat_version()
                // Check if the message is signed correctly.
                // If it's not signed correctly, the whole autocrypt header will be mostly
                // ignored anyway and the message shown as not encrypted, so we don't
                // have to handle this case.
                && !mime_parser.signatures.is_empty()
                // Check if the From: address was also in the signed part of the email.
                // Without this check, an attacker could replay a message from Alice 
                // to Bob. Then Bob's device would do an AEAP transition from Alice's
                // to the attacker's address, allowing for easier phishing.
                && mime_parser.from_is_signed
                && info.message_time > peerstate.last_seen
        {
            let info = &mut mime_parser.decryption_info;
            let peerstate = info.peerstate.as_mut().context("no peerstate??")?;
            // Add info messages to chats with this (verified) contact
            //
            peerstate
                .handle_setup_change(
                    context,
                    info.message_time,
                    PeerstateChange::Aeap(info.from.clone()),
                )
                .await?;

            peerstate.addr = info.from.clone();
            let header = info.autocrypt_header.as_ref().context(
                "Internal error: Tried to do an AEAP transition without an autocrypt header??",
            )?;
            peerstate.apply_header(header, info.message_time);

            peerstate.save_to_db(&context.sql).await?;
        }
    }

    Ok(())
}

/// Type of the peerstate change.
///
/// Changes to the peerstate are notified to the user via a message
/// explaining the happened change.
enum PeerstateChange {
    /// The contact's public key fingerprint changed, likely because
    /// the contact uses a new device and didn't transfer their key.
    FingerprintChange,
    /// The contact changed their address to the given new address
    /// (Automatic Email Address Porting).
    Aeap(String),
}

/// Removes duplicate peerstates from `acpeerstates` database table.
///
/// Normally there should be no more than one peerstate per address.
/// However, the database does not enforce this condition.
///
/// Previously there were bugs that caused creation of additional
/// peerstates when existing peerstate could not be read due to a
/// temporary database error or a failure to parse stored data.  This
/// procedure fixes the problem by removing duplicate records.
pub(crate) async fn deduplicate_peerstates(sql: &Sql) -> Result<()> {
    sql.execute(
        "DELETE FROM acpeerstates
         WHERE id NOT IN (
         SELECT MIN(id)
         FROM acpeerstates
         GROUP BY addr
         )",
        (),
    )
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::alice_keypair;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_peerstate_save_to_db() {
        let ctx = crate::test_utils::TestContext::new().await;
        let addr = "hello@mail.com";

        let pub_key = alice_keypair().public;

        let peerstate = Peerstate {
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
            fingerprint_changed: false,
            verifier: None,
        };

        assert!(
            peerstate.save_to_db(&ctx.ctx.sql).await.is_ok(),
            "failed to save to db"
        );

        let peerstate_new = Peerstate::from_addr(&ctx.ctx, addr)
            .await
            .expect("failed to load peerstate from db")
            .expect("no peerstate found in the database");

        assert_eq!(peerstate, peerstate_new);
        let peerstate_new2 = Peerstate::from_fingerprint(&ctx.ctx, &pub_key.fingerprint())
            .await
            .expect("failed to load peerstate from db")
            .expect("no peerstate found in the database");
        assert_eq!(peerstate, peerstate_new2);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_peerstate_double_create() {
        let ctx = crate::test_utils::TestContext::new().await;
        let addr = "hello@mail.com";
        let pub_key = alice_keypair().public;

        let peerstate = Peerstate {
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
            fingerprint_changed: false,
            verifier: None,
        };

        assert!(
            peerstate.save_to_db(&ctx.ctx.sql).await.is_ok(),
            "failed to save"
        );
        assert!(
            peerstate.save_to_db(&ctx.ctx.sql).await.is_ok(),
            "double-call with create failed"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_peerstate_with_empty_gossip_key_save_to_db() {
        let ctx = crate::test_utils::TestContext::new().await;
        let addr = "hello@mail.com";

        let pub_key = alice_keypair().public;

        let peerstate = Peerstate {
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
            fingerprint_changed: false,
            verifier: None,
        };

        assert!(
            peerstate.save_to_db(&ctx.ctx.sql).await.is_ok(),
            "failed to save"
        );

        let peerstate_new = Peerstate::from_addr(&ctx.ctx, addr)
            .await
            .expect("failed to load peerstate from db");

        assert_eq!(Some(peerstate), peerstate_new);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_peerstate_load_db_defaults() {
        let ctx = crate::test_utils::TestContext::new().await;
        let addr = "hello@mail.com";

        // Old code created peerstates with this code and updated
        // other values later.  If UPDATE failed, other columns had
        // default values, in particular fingerprints were set to
        // empty strings instead of NULL. This should not be the case
        // anymore, but the regression test still checks that defaults
        // can be loaded without errors.
        ctx.ctx
            .sql
            .execute("INSERT INTO acpeerstates (addr) VALUES(?)", (addr,))
            .await
            .expect("Failed to write to the database");

        let peerstate = Peerstate::from_addr(&ctx.ctx, addr)
            .await
            .expect("Failed to load peerstate from db")
            .expect("Loaded peerstate is empty");

        // Check that default values for fingerprints are treated like
        // NULL.
        assert_eq!(peerstate.public_key_fingerprint, None);
        assert_eq!(peerstate.gossip_key_fingerprint, None);
        assert_eq!(peerstate.verified_key_fingerprint, None);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_peerstate_degrade_reordering() {
        let addr = "example@example.org";
        let pub_key = alice_keypair().public;
        let header = Aheader::new(addr.to_string(), pub_key, EncryptPreference::Mutual);

        let mut peerstate = Peerstate {
            addr: addr.to_string(),
            last_seen: 0,
            last_seen_autocrypt: 0,
            prefer_encrypt: EncryptPreference::NoPreference,
            public_key: None,
            public_key_fingerprint: None,
            gossip_key: None,
            gossip_timestamp: 0,
            gossip_key_fingerprint: None,
            verified_key: None,
            verified_key_fingerprint: None,
            fingerprint_changed: false,
            verifier: None,
        };

        peerstate.apply_header(&header, 100);
        assert_eq!(peerstate.prefer_encrypt, EncryptPreference::Mutual);

        peerstate.degrade_encryption(300);
        assert_eq!(peerstate.prefer_encrypt, EncryptPreference::Reset);

        // This has message time 200, while encryption was degraded at timestamp 300.
        // Because of reordering, header should not be applied.
        peerstate.apply_header(&header, 200);
        assert_eq!(peerstate.prefer_encrypt, EncryptPreference::Reset);

        // Same header will be applied in the future.
        peerstate.apply_header(&header, 400);
        assert_eq!(peerstate.prefer_encrypt, EncryptPreference::Mutual);
    }
}
