//! Database serialization module.
//!
//! The module contains functions to serialize database into a stream.
//!
//! Output format is based on [bencoding](http://bittorrent.org/beps/bep_0003.html).

/// Database version supported by the current serialization code.
///
/// Serialization code MUST be updated before increasing this number.
///
/// If this version is below the actual database version,
/// serialization code is outdated.
/// If this version is above the actual database version,
/// migrations have to be run first to update the database.
const SERIALIZE_DBVERSION: &str = "99";

use anyhow::{anyhow, Context as _, Result};
use rusqlite::types::ValueRef;
use rusqlite::Transaction;
use tokio::io::{AsyncWrite, AsyncWriteExt};

use super::Sql;

struct Encoder<'a, W: AsyncWrite + Unpin> {
    tx: Transaction<'a>,

    w: W,
}

async fn write_bytes(w: &mut (impl AsyncWrite + Unpin), b: &[u8]) -> Result<()> {
    let bytes_len = format!("{}:", b.len());
    w.write_all(bytes_len.as_bytes()).await?;
    w.write_all(b).await?;
    Ok(())
}

async fn write_str(w: &mut (impl AsyncWrite + Unpin), s: &str) -> Result<()> {
    write_bytes(w, s.as_bytes()).await?;
    Ok(())
}

async fn write_i64(w: &mut (impl AsyncWrite + Unpin), i: i64) -> Result<()> {
    let s = format!("{i}");
    w.write_all(b"i").await?;
    w.write_all(s.as_bytes()).await?;
    w.write_all(b"e").await?;
    Ok(())
}

async fn write_u32(w: &mut (impl AsyncWrite + Unpin), i: u32) -> Result<()> {
    let s = format!("{i}");
    w.write_all(b"i").await?;
    w.write_all(s.as_bytes()).await?;
    w.write_all(b"e").await?;
    Ok(())
}

async fn write_f64(w: &mut (impl AsyncWrite + Unpin), f: f64) -> Result<()> {
    write_bytes(w, &f.to_be_bytes()).await?;
    Ok(())
}

async fn write_bool(w: &mut (impl AsyncWrite + Unpin), b: bool) -> Result<()> {
    if b {
        w.write_all(b"i1e").await?;
    } else {
        w.write_all(b"i0e").await?;
    }
    Ok(())
}

impl<'a, W: AsyncWrite + Unpin> Encoder<'a, W> {
    fn new(tx: Transaction<'a>, w: W) -> Self {
        Self { tx, w }
    }

    /// Serializes `config` table.
    async fn serialize_config(&mut self) -> Result<()> {
        // FIXME: sort the dictionary in lexicographical order
        // dbversion should be the first, so store it as "_config._dbversion"

        let mut stmt = self.tx.prepare("SELECT keyname,value FROM config")?;
        let mut rows = stmt.query(())?;
        self.w.write_all(b"d").await?;
        while let Some(row) = rows.next()? {
            let keyname: String = row.get(0)?;
            let value: String = row.get(1)?;
            write_str(&mut self.w, &keyname).await?;
            write_str(&mut self.w, &value).await?;
        }
        self.w.write_all(b"e").await?;
        Ok(())
    }

    async fn serialize_acpeerstates(&mut self) -> Result<()> {
        let mut stmt = self.tx.prepare("SELECT addr, last_seen, last_seen_autocrypt, public_key, prefer_encrypted, gossip_timestamp, gossip_key, public_key_fingerprint, gossip_key_fingerprint, verified_key, verified_key_fingerprint FROM acpeerstates")?;
        let mut rows = stmt.query(())?;

        self.w.write_all(b"l").await?;
        while let Some(row) = rows.next()? {
            let addr: String = row.get("addr")?;
            let prefer_encrypted: i64 = row.get("prefer_encrypted")?;

            let last_seen: i64 = row.get("last_seen")?;

            let last_seen_autocrypt: i64 = row.get("last_seen_autocrypt")?;
            let public_key: Option<Vec<u8>> = row.get("public_key")?;
            let public_key_fingerprint: Option<String> = row.get("public_key_fingerprint")?;

            let gossip_timestamp: i64 = row.get("gossip_timestamp")?;
            let gossip_key: Option<Vec<u8>> = row.get("gossip_key")?;
            let gossip_key_fingerprint: Option<String> = row.get("gossip_key_fingerprint")?;

            let verified_key: Option<Vec<u8>> = row.get("verified_key")?;
            let verified_key_fingerprint: Option<String> = row.get("verified_key_fingerprint")?;

            self.w.write_all(b"d").await?;

            write_str(&mut self.w, "addr").await?;
            write_str(&mut self.w, &addr).await?;

            if let Some(gossip_key) = gossip_key {
                write_str(&mut self.w, "gossip_key").await?;
                write_bytes(&mut self.w, &gossip_key).await?;
            }

            if let Some(gossip_key_fingerprint) = gossip_key_fingerprint {
                write_str(&mut self.w, "gossip_key_fingerprint").await?;
                write_str(&mut self.w, &gossip_key_fingerprint).await?;
            }

            write_str(&mut self.w, "gossip_timestamp").await?;
            write_i64(&mut self.w, gossip_timestamp).await?;

            write_str(&mut self.w, "last_seen").await?;
            write_i64(&mut self.w, last_seen).await?;

            write_str(&mut self.w, "last_seen_autocrypt").await?;
            write_i64(&mut self.w, last_seen_autocrypt).await?;

            write_str(&mut self.w, "prefer_encrypted").await?;
            write_i64(&mut self.w, prefer_encrypted).await?;

            if let Some(public_key) = public_key {
                write_str(&mut self.w, "public_key").await?;
                write_bytes(&mut self.w, &public_key).await?;
            }

            if let Some(public_key_fingerprint) = public_key_fingerprint {
                write_str(&mut self.w, "public_key_fingerprint").await?;
                write_str(&mut self.w, &public_key_fingerprint).await?;
            }

            if let Some(verified_key) = verified_key {
                write_str(&mut self.w, "verified_key").await?;
                write_bytes(&mut self.w, &verified_key).await?;
            }

            if let Some(verified_key_fingerprint) = verified_key_fingerprint {
                write_str(&mut self.w, "verified_key_fingerprint").await?;
                write_str(&mut self.w, &verified_key_fingerprint).await?;
            }

            self.w.write_all(b"e").await?;
        }
        self.w.write_all(b"e").await?;
        Ok(())
    }

    /// Serializes chats.
    async fn serialize_chats(&mut self) -> Result<()> {
        let mut stmt = self.tx.prepare(
            "SELECT \
        id,\
        type,\
        name,\
        blocked,\
        grpid,\
        param,\
        archived,\
        gossiped_timestamp,\
        locations_send_begin,\
        locations_send_until,\
        locations_last_sent,\
        created_timestamp,\
        muted_until,\
        ephemeral_timer,\
        protected FROM chats",
        )?;
        let mut rows = stmt.query(())?;

        self.w.write_all(b"l").await?;
        while let Some(row) = rows.next()? {
            let id: u32 = row.get("id")?;
            let typ: u32 = row.get("type")?;
            let name: String = row.get("name")?;
            let blocked: u32 = row.get("blocked")?;
            let grpid: String = row.get("grpid")?;
            let param: String = row.get("param")?;
            let archived: bool = row.get("archived")?;
            let gossiped_timestamp: i64 = row.get("gossiped_timestamp")?;
            let locations_send_begin: i64 = row.get("locations_send_begin")?;
            let locations_send_until: i64 = row.get("locations_send_until")?;
            let locations_last_sent: i64 = row.get("locations_last_sent")?;
            let created_timestamp: i64 = row.get("created_timestamp")?;
            let muted_until: i64 = row.get("muted_until")?;
            let ephemeral_timer: i64 = row.get("ephemeral_timer")?;
            let protected: u32 = row.get("protected")?;

            self.w.write_all(b"d").await?;

            write_str(&mut self.w, "archived").await?;
            write_bool(&mut self.w, archived).await?;

            write_str(&mut self.w, "blocked").await?;
            write_u32(&mut self.w, blocked).await?;

            write_str(&mut self.w, "created_timestamp").await?;
            write_i64(&mut self.w, created_timestamp).await?;

            write_str(&mut self.w, "ephemeral_timer").await?;
            write_i64(&mut self.w, ephemeral_timer).await?;

            write_str(&mut self.w, "gossiped_timestamp").await?;
            write_i64(&mut self.w, gossiped_timestamp).await?;

            write_str(&mut self.w, "grpid").await?;
            write_str(&mut self.w, &grpid).await?;

            write_str(&mut self.w, "id").await?;
            write_u32(&mut self.w, id).await?;

            write_str(&mut self.w, "locations_last_sent").await?;
            write_i64(&mut self.w, locations_last_sent).await?;

            write_str(&mut self.w, "locations_send_begin").await?;
            write_i64(&mut self.w, locations_send_begin).await?;

            write_str(&mut self.w, "locations_send_until").await?;
            write_i64(&mut self.w, locations_send_until).await?;

            write_str(&mut self.w, "muted_until").await?;
            write_i64(&mut self.w, muted_until).await?;

            write_str(&mut self.w, "name").await?;
            write_str(&mut self.w, &name).await?;

            write_str(&mut self.w, "param").await?;
            write_str(&mut self.w, &param).await?;

            write_str(&mut self.w, "protected").await?;
            write_u32(&mut self.w, protected).await?;

            write_str(&mut self.w, "type").await?;
            write_u32(&mut self.w, typ).await?;

            self.w.write_all(b"e").await?;
        }
        self.w.write_all(b"e").await?;
        Ok(())
    }

    async fn serialize_chats_contacts(&mut self) -> Result<()> {
        let mut stmt = self
            .tx
            .prepare("SELECT chat_id, contact_id FROM chats_contacts")?;
        let mut rows = stmt.query(())?;

        self.w.write_all(b"l").await?;
        while let Some(row) = rows.next()? {
            let chat_id: u32 = row.get("chat_id")?;
            let contact_id: u32 = row.get("contact_id")?;

            self.w.write_all(b"d").await?;

            write_str(&mut self.w, "chat_id").await?;
            write_u32(&mut self.w, chat_id).await?;

            write_str(&mut self.w, "contact_id").await?;
            write_u32(&mut self.w, contact_id).await?;

            self.w.write_all(b"e").await?;
        }
        self.w.write_all(b"e").await?;
        Ok(())
    }

    /// Serializes contacts.
    async fn serialize_contacts(&mut self) -> Result<()> {
        let mut stmt = self.tx.prepare(
            "SELECT \
        id,\
        name,\
        addr,\
        origin,\
        blocked,\
        last_seen,\
        param,\
        authname,\
        selfavatar_sent,\
        status FROM contacts",
        )?;
        let mut rows = stmt.query(())?;
        self.w.write_all(b"l").await?;
        while let Some(row) = rows.next()? {
            let id: u32 = row.get("id")?;
            let name: String = row.get("name")?;
            let authname: String = row.get("authname")?;
            let addr: String = row.get("addr")?;
            let origin: u32 = row.get("origin")?;
            let blocked: Option<bool> = row.get("blocked")?;
            let blocked = blocked.unwrap_or_default();
            let last_seen: i64 = row.get("last_seen")?;
            let selfavatar_sent: i64 = row.get("selfavatar_sent")?;
            let param: String = row.get("param")?;
            let status: Option<String> = row.get("status")?;

            self.w.write_all(b"d").await?;

            write_str(&mut self.w, "addr").await?;
            write_str(&mut self.w, &addr).await?;

            write_str(&mut self.w, "authname").await?;
            write_str(&mut self.w, &authname).await?;

            write_str(&mut self.w, "blocked").await?;
            write_bool(&mut self.w, blocked).await?;

            write_str(&mut self.w, "id").await?;
            write_u32(&mut self.w, id).await?;

            write_str(&mut self.w, "last_seen").await?;
            write_i64(&mut self.w, last_seen).await?;

            write_str(&mut self.w, "name").await?;
            write_str(&mut self.w, &name).await?;

            write_str(&mut self.w, "origin").await?;
            write_u32(&mut self.w, origin).await?;

            // TODO: parse param instead of serializeing as is
            write_str(&mut self.w, "param").await?;
            write_str(&mut self.w, &param).await?;

            write_str(&mut self.w, "selfavatar_sent").await?;
            write_i64(&mut self.w, selfavatar_sent).await?;

            if let Some(status) = status {
                if !status.is_empty() {
                    write_str(&mut self.w, "status").await?;
                    write_str(&mut self.w, &status).await?;
                }
            }
            self.w.write_all(b"e").await?;
        }
        self.w.write_all(b"e").await?;
        Ok(())
    }

    async fn serialize_dns_cache(&mut self) -> Result<()> {
        let mut stmt = self
            .tx
            .prepare("SELECT hostname, address, timestamp FROM dns_cache")?;
        let mut rows = stmt.query(())?;

        self.w.write_all(b"l").await?;
        while let Some(row) = rows.next()? {
            let hostname: String = row.get("hostname")?;
            let address: String = row.get("address")?;
            let timestamp: i64 = row.get("timestamp")?;

            self.w.write_all(b"d").await?;

            write_str(&mut self.w, "address").await?;
            write_str(&mut self.w, &address).await?;

            write_str(&mut self.w, "hostname").await?;
            write_str(&mut self.w, &hostname).await?;

            write_str(&mut self.w, "timestamp").await?;
            write_i64(&mut self.w, timestamp).await?;

            self.w.write_all(b"e").await?;
        }
        self.w.write_all(b"e").await?;
        Ok(())
    }

    async fn serialize_imap(&mut self) -> Result<()> {
        let mut stmt = self
            .tx
            .prepare("SELECT id, rfc724_mid, folder, target, uid, uidvalidity FROM imap")?;
        let mut rows = stmt.query(())?;

        self.w.write_all(b"l").await?;
        while let Some(row) = rows.next()? {
            let id: i64 = row.get("id")?;
            let rfc724_mid: String = row.get("rfc724_mid")?;
            let folder: String = row.get("folder")?;
            let target: String = row.get("target")?;
            let uid: i64 = row.get("uid")?;
            let uidvalidity: i64 = row.get("uidvalidity")?;

            self.w.write_all(b"d").await?;

            write_str(&mut self.w, "folder").await?;
            write_str(&mut self.w, &folder).await?;

            write_str(&mut self.w, "id").await?;
            write_i64(&mut self.w, id).await?;

            write_str(&mut self.w, "rfc724_mid").await?;
            write_str(&mut self.w, &rfc724_mid).await?;

            write_str(&mut self.w, "target").await?;
            write_str(&mut self.w, &target).await?;

            write_str(&mut self.w, "uid").await?;
            write_i64(&mut self.w, uid).await?;

            write_str(&mut self.w, "uidvalidity").await?;
            write_i64(&mut self.w, uidvalidity).await?;

            self.w.write_all(b"e").await?;
        }
        self.w.write_all(b"e").await?;
        Ok(())
    }

    async fn serialize_imap_sync(&mut self) -> Result<()> {
        let mut stmt = self
            .tx
            .prepare("SELECT folder, uidvalidity, uid_next, modseq FROM imap_sync")?;
        let mut rows = stmt.query(())?;

        self.w.write_all(b"l").await?;
        while let Some(row) = rows.next()? {
            let folder: String = row.get("folder")?;
            let uidvalidity: i64 = row.get("uidvalidity")?;
            let uidnext: i64 = row.get("uid_next")?;
            let modseq: i64 = row.get("modseq")?;

            self.w.write_all(b"d").await?;

            write_str(&mut self.w, "folder").await?;
            write_str(&mut self.w, &folder).await?;

            write_str(&mut self.w, "modseq").await?;
            write_i64(&mut self.w, modseq).await?;

            write_str(&mut self.w, "uidnext").await?;
            write_i64(&mut self.w, uidnext).await?;

            write_str(&mut self.w, "uidvalidity").await?;
            write_i64(&mut self.w, uidvalidity).await?;

            self.w.write_all(b"e").await?;
        }
        self.w.write_all(b"e").await?;
        Ok(())
    }

    async fn serialize_keypairs(&mut self) -> Result<()> {
        let mut stmt = self
            .tx
            .prepare("SELECT id,addr,is_default,private_key,public_key,created FROM keypairs")?;
        let mut rows = stmt.query(())?;

        self.w.write_all(b"l").await?;
        while let Some(row) = rows.next()? {
            let id: u32 = row.get("id")?;
            let addr: String = row.get("addr")?;
            let is_default: u32 = row.get("is_default")?;
            let is_default = is_default != 0;
            let private_key: Vec<u8> = row.get("private_key")?;
            let public_key: Vec<u8> = row.get("public_key")?;
            let created: i64 = row.get("created")?;

            self.w.write_all(b"d").await?;

            write_str(&mut self.w, "addr").await?;
            write_str(&mut self.w, &addr).await?;

            write_str(&mut self.w, "created").await?;
            write_i64(&mut self.w, created).await?;

            write_str(&mut self.w, "id").await?;
            write_u32(&mut self.w, id).await?;

            write_str(&mut self.w, "is_default").await?;
            write_bool(&mut self.w, is_default).await?;

            write_str(&mut self.w, "private_key").await?;
            write_bytes(&mut self.w, &private_key).await?;

            write_str(&mut self.w, "public_key").await?;
            write_bytes(&mut self.w, &public_key).await?;

            self.w.write_all(b"e").await?;
        }
        self.w.write_all(b"e").await?;
        Ok(())
    }

    async fn serialize_leftgroups(&mut self) -> Result<()> {
        let mut stmt = self.tx.prepare("SELECT grpid FROM leftgrps")?;
        let mut rows = stmt.query(())?;

        self.w.write_all(b"l").await?;
        while let Some(row) = rows.next()? {
            let grpid: String = row.get("grpid")?;
            write_str(&mut self.w, &grpid).await?;
        }
        self.w.write_all(b"e").await?;
        Ok(())
    }

    async fn serialize_locations(&mut self) -> Result<()> {
        let mut stmt = self
            .tx
            .prepare("SELECT id, latitude, longitude, accuracy, timestamp, chat_id, from_id, independent FROM locations")?;
        let mut rows = stmt.query(())?;

        self.w.write_all(b"l").await?;
        while let Some(row) = rows.next()? {
            let id: i64 = row.get("id")?;
            let latitude: f64 = row.get("latitude")?;
            let longitude: f64 = row.get("longitude")?;
            let accuracy: f64 = row.get("accuracy")?;
            let timestamp: i64 = row.get("timestamp")?;
            let chat_id: u32 = row.get("chat_id")?;
            let from_id: u32 = row.get("from_id")?;
            let independent: u32 = row.get("independent")?;

            self.w.write_all(b"d").await?;

            write_str(&mut self.w, "accuracy").await?;
            write_f64(&mut self.w, accuracy).await?;

            write_str(&mut self.w, "chat_id").await?;
            write_u32(&mut self.w, chat_id).await?;

            write_str(&mut self.w, "from_id").await?;
            write_u32(&mut self.w, from_id).await?;

            write_str(&mut self.w, "id").await?;
            write_i64(&mut self.w, id).await?;

            write_str(&mut self.w, "independent").await?;
            write_u32(&mut self.w, independent).await?;

            write_str(&mut self.w, "latitude").await?;
            write_f64(&mut self.w, latitude).await?;

            write_str(&mut self.w, "longitude").await?;
            write_f64(&mut self.w, longitude).await?;

            write_str(&mut self.w, "timestamp").await?;
            write_i64(&mut self.w, timestamp).await?;

            self.w.write_all(b"e").await?;
        }
        self.w.write_all(b"e").await?;
        Ok(())
    }

    /// Serializes MDNs.
    async fn serialize_mdns(&mut self) -> Result<()> {
        let mut stmt = self
            .tx
            .prepare("SELECT msg_id, contact_id, timestamp_sent FROM msgs_mdns")?;
        let mut rows = stmt.query(())?;

        self.w.write_all(b"l").await?;
        while let Some(row) = rows.next()? {
            let msg_id: u32 = row.get("msg_id")?;
            let contact_id: u32 = row.get("contact_id")?;
            let timestamp_sent: i64 = row.get("timestamp_sent")?;

            self.w.write_all(b"d").await?;

            write_str(&mut self.w, "contact_id").await?;
            write_u32(&mut self.w, contact_id).await?;

            write_str(&mut self.w, "msg_id").await?;
            write_u32(&mut self.w, msg_id).await?;

            write_str(&mut self.w, "timestamp_sent").await?;
            write_i64(&mut self.w, timestamp_sent).await?;

            self.w.write_all(b"e").await?;
        }
        self.w.write_all(b"e").await?;
        Ok(())
    }

    /// Serializes messages.
    async fn serialize_messages(&mut self) -> Result<()> {
        let mut stmt = self.tx.prepare(
            "SELECT
                        id,
                        rfc724_mid,
                        chat_id,
                        from_id, to_id,
                        timestamp,
                        type,
                        state,
                        msgrmsg,
                        bytes,
                        txt,
                        txt_raw,
                        param,
                        timestamp_sent,
                        timestamp_rcvd,
                        hidden,
                        mime_headers,
                        mime_in_reply_to,
                        mime_references,
                        location_id FROM msgs",
        )?;
        let mut rows = stmt.query(())?;

        self.w.write_all(b"l").await?;
        while let Some(row) = rows.next()? {
            let id: i64 = row.get("id")?;
            let rfc724_mid: String = row.get("rfc724_mid")?;
            let chat_id: i64 = row.get("chat_id")?;
            let from_id: i64 = row.get("from_id")?;
            let to_id: i64 = row.get("to_id")?;
            let timestamp: i64 = row.get("timestamp")?;
            let typ: i64 = row.get("type")?;
            let state: i64 = row.get("state")?;
            let msgrmsg: i64 = row.get("msgrmsg")?;
            let bytes: i64 = row.get("bytes")?;
            let txt: String = row.get("txt")?;
            let txt_raw: String = row.get("txt_raw")?;
            let param: String = row.get("param")?;
            let timestamp_sent: i64 = row.get("timestamp_sent")?;
            let timestamp_rcvd: i64 = row.get("timestamp_rcvd")?;
            let hidden: i64 = row.get("hidden")?;
            let mime_headers: Vec<u8> =
                row.get("mime_headers")
                    .or_else(|err| match row.get_ref("mime_headers")? {
                        ValueRef::Null => Ok(Vec::new()),
                        ValueRef::Text(text) => Ok(text.to_vec()),
                        ValueRef::Blob(blob) => Ok(blob.to_vec()),
                        ValueRef::Integer(_) | ValueRef::Real(_) => Err(err),
                    })?;
            let mime_in_reply_to: Option<String> = row.get("mime_in_reply_to")?;
            let mime_references: Option<String> = row.get("mime_references")?;
            let location_id: i64 = row.get("location_id")?;

            self.w.write_all(b"d").await?;

            write_str(&mut self.w, "bytes").await?;
            write_i64(&mut self.w, bytes).await?;

            write_str(&mut self.w, "chat_id").await?;
            write_i64(&mut self.w, chat_id).await?;

            write_str(&mut self.w, "from_id").await?;
            write_i64(&mut self.w, from_id).await?;

            write_str(&mut self.w, "hidden").await?;
            write_i64(&mut self.w, hidden).await?;

            write_str(&mut self.w, "id").await?;
            write_i64(&mut self.w, id).await?;

            write_str(&mut self.w, "location_id").await?;
            write_i64(&mut self.w, location_id).await?;

            write_str(&mut self.w, "mime_headers").await?;
            write_bytes(&mut self.w, &mime_headers).await?;

            if let Some(mime_in_reply_to) = mime_in_reply_to {
                write_str(&mut self.w, "mime_in_reply_to").await?;
                write_str(&mut self.w, &mime_in_reply_to).await?;
            }

            if let Some(mime_references) = mime_references {
                write_str(&mut self.w, "mime_references").await?;
                write_str(&mut self.w, &mime_references).await?;
            }

            write_str(&mut self.w, "msgrmsg").await?;
            write_i64(&mut self.w, msgrmsg).await?;

            write_str(&mut self.w, "param").await?;
            write_str(&mut self.w, &param).await?;

            write_str(&mut self.w, "rfc724_mid").await?;
            write_str(&mut self.w, &rfc724_mid).await?;

            write_str(&mut self.w, "state").await?;
            write_i64(&mut self.w, state).await?;

            write_str(&mut self.w, "timestamp").await?;
            write_i64(&mut self.w, timestamp).await?;

            write_str(&mut self.w, "timestamp_rcvd").await?;
            write_i64(&mut self.w, timestamp_rcvd).await?;

            write_str(&mut self.w, "timestamp_sent").await?;
            write_i64(&mut self.w, timestamp_sent).await?;

            write_str(&mut self.w, "to_id").await?;
            write_i64(&mut self.w, to_id).await?;

            write_str(&mut self.w, "txt").await?;
            write_str(&mut self.w, &txt).await?;

            write_str(&mut self.w, "txt_raw").await?;
            write_str(&mut self.w, &txt_raw).await?;

            write_str(&mut self.w, "type").await?;
            write_i64(&mut self.w, typ).await?;

            self.w.write_all(b"e").await?;
        }
        self.w.write_all(b"e").await?;
        Ok(())
    }

    async fn serialize_msgs_status_updates(&mut self) -> Result<()> {
        let mut stmt = self
            .tx
            .prepare("SELECT id, msg_id, update_item FROM msgs_status_updates")?;
        let mut rows = stmt.query(())?;

        self.w.write_all(b"l").await?;
        while let Some(row) = rows.next()? {
            let id: i64 = row.get("id")?;
            let msg_id: i64 = row.get("msg_id")?;
            let update_item: String = row.get("update_item")?;

            self.w.write_all(b"d").await?;

            write_str(&mut self.w, "id").await?;
            write_i64(&mut self.w, id).await?;

            write_str(&mut self.w, "msg_id").await?;
            write_i64(&mut self.w, msg_id).await?;

            write_str(&mut self.w, "update_item").await?;
            write_str(&mut self.w, &update_item).await?;

            self.w.write_all(b"e").await?;
        }
        self.w.write_all(b"e").await?;
        Ok(())
    }

    /// Serializes reactions.
    async fn serialize_reactions(&mut self) -> Result<()> {
        let mut stmt = self
            .tx
            .prepare("SELECT msg_id, contact_id, reaction FROM reactions")?;
        let mut rows = stmt.query(())?;

        self.w.write_all(b"l").await?;
        while let Some(row) = rows.next()? {
            let msg_id: u32 = row.get("msg_id")?;
            let contact_id: u32 = row.get("contact_id")?;
            let reaction: String = row.get("reaction")?;

            self.w.write_all(b"d").await?;

            write_str(&mut self.w, "contact_id").await?;
            write_u32(&mut self.w, contact_id).await?;

            write_str(&mut self.w, "msg_id").await?;
            write_u32(&mut self.w, msg_id).await?;

            write_str(&mut self.w, "reaction").await?;
            write_str(&mut self.w, &reaction).await?;

            self.w.write_all(b"e").await?;
        }
        self.w.write_all(b"e").await?;
        Ok(())
    }

    async fn serialize_sending_domains(&mut self) -> Result<()> {
        let mut stmt = self
            .tx
            .prepare("SELECT domain, dkim_works FROM sending_domains")?;
        let mut rows = stmt.query(())?;

        self.w.write_all(b"l").await?;
        while let Some(row) = rows.next()? {
            let domain: String = row.get("domain")?;
            let dkim_works: i64 = row.get("dkim_works")?;

            self.w.write_all(b"d").await?;

            write_str(&mut self.w, "dkim_works").await?;
            write_i64(&mut self.w, dkim_works).await?;

            write_str(&mut self.w, "domain").await?;
            write_str(&mut self.w, &domain).await?;

            self.w.write_all(b"e").await?;
        }
        self.w.write_all(b"e").await?;
        Ok(())
    }

    async fn serialize_tokens(&mut self) -> Result<()> {
        let mut stmt = self
            .tx
            .prepare("SELECT id, namespc, foreign_id, token, timestamp FROM tokens")?;
        let mut rows = stmt.query(())?;

        self.w.write_all(b"l").await?;
        while let Some(row) = rows.next()? {
            let id: i64 = row.get("id")?;
            let namespace: u32 = row.get("namespc")?;
            let foreign_id: u32 = row.get("foreign_id")?;
            let token: String = row.get("token")?;
            let timestamp: i64 = row.get("timestamp")?;

            self.w.write_all(b"d").await?;

            write_str(&mut self.w, "foreign_id").await?;
            write_u32(&mut self.w, foreign_id).await?;

            write_str(&mut self.w, "id").await?;
            write_i64(&mut self.w, id).await?;

            write_str(&mut self.w, "namespace").await?;
            write_u32(&mut self.w, namespace).await?;

            write_str(&mut self.w, "timestamp").await?;
            write_i64(&mut self.w, timestamp).await?;

            write_str(&mut self.w, "token").await?;
            write_str(&mut self.w, &token).await?;

            self.w.write_all(b"e").await?;
        }
        self.w.write_all(b"e").await?;
        Ok(())
    }

    async fn serialize(&mut self) -> Result<()> {
        let dbversion: String = self.tx.query_row(
            "SELECT value FROM config WHERE keyname='dbversion'",
            (),
            |row| row.get(0),
        )?;
        if dbversion != SERIALIZE_DBVERSION {
            return Err(anyhow!(
                "cannot serialize database version {dbversion}, expected {SERIALIZE_DBVERSION}"
            ));
        }

        self.w.write_all(b"d").await?;

        write_str(&mut self.w, "_config").await?;
        self.serialize_config().await?;

        write_str(&mut self.w, "acpeerstates").await?;
        self.serialize_acpeerstates()
            .await
            .context("serialize autocrypt peerstates")?;

        write_str(&mut self.w, "chats").await?;
        self.serialize_chats().await?;

        write_str(&mut self.w, "chats_contacts").await?;
        self.serialize_chats_contacts()
            .await
            .context("serialize chats_contacts")?;

        write_str(&mut self.w, "contacts").await?;
        self.serialize_contacts().await?;

        write_str(&mut self.w, "dns_cache").await?;
        self.serialize_dns_cache()
            .await
            .context("serialize dns_cache")?;

        write_str(&mut self.w, "imap").await?;
        self.serialize_imap().await.context("serialize imap")?;

        write_str(&mut self.w, "imap_sync").await?;
        self.serialize_imap_sync()
            .await
            .context("serialize imap_sync")?;

        write_str(&mut self.w, "keypairs").await?;
        self.serialize_keypairs().await?;

        write_str(&mut self.w, "leftgroups").await?;
        self.serialize_leftgroups().await?;

        write_str(&mut self.w, "locations").await?;
        self.serialize_locations().await?;

        write_str(&mut self.w, "mdns").await?;
        self.serialize_mdns().await?;

        write_str(&mut self.w, "messages").await?;
        self.serialize_messages()
            .await
            .context("serialize messages")?;

        write_str(&mut self.w, "msgs_status_updates").await?;
        self.serialize_msgs_status_updates()
            .await
            .context("serialize msgs_status_updates")?;

        write_str(&mut self.w, "reactions").await?;
        self.serialize_reactions().await?;

        write_str(&mut self.w, "sending_domains").await?;
        self.serialize_sending_domains()
            .await
            .context("serialize sending_domains")?;

        write_str(&mut self.w, "tokens").await?;
        self.serialize_tokens().await?;

        // jobs table is skipped
        // multi_device_sync is skipped
        // imap_markseen is skipped, it is usually empty and the device exporting the
        // database should still be able to clear it.
        // smtp, smtp_mdns and smtp_status_updates tables are skipped, they are part of the
        // outgoing message queue.
        // devmsglabels is skipped, it is reset in `delete_and_reset_all_device_msgs()` on import
        // anyway
        // bobstate is not serialized, it is temporary for joining or adding a contact.
        //
        // TODO insert welcome message on import like done in `delete_and_reset_all_device_msgs()`?
        self.w.write_all(b"e").await?;
        self.w.flush().await?;
        Ok(())
    }
}

impl Sql {
    /// Serializes the database into a bytestream.
    pub async fn serialize(&self, w: impl AsyncWrite + Unpin) -> Result<()> {
        let mut conn = self.get_connection().await?;

        // Start a read transaction to take a database snapshot.
        let transaction = conn.transaction()?;
        let mut encoder = Encoder::new(transaction, w);
        encoder.serialize().await?;
        Ok(())
    }
}
