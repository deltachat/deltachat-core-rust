//! Utilities to help writing tests.
//!
//! This module is only compiled for test runs.

use std::str::FromStr;
use std::time::{Duration, Instant};

use async_std::path::PathBuf;
use async_std::sync::RwLock;
use chat::ChatItem;
use tempfile::{tempdir, TempDir};

use crate::config::Config;
use crate::context::Context;
use crate::dc_receive_imf::dc_receive_imf;
use crate::dc_tools::EmailAddress;
use crate::job::Action;
use crate::key::{self, DcKey};
use crate::mimeparser::MimeMessage;
use crate::param::{Param, Params};
use crate::{
    chat::{self, ChatId},
    message::Message,
};

/// A Context and temporary directory.
///
/// The temporary directory can be used to store the SQLite database,
/// see e.g. [test_context] which does this.
#[derive(Debug)]
pub(crate) struct TestContext {
    pub ctx: Context,
    pub dir: TempDir,
    /// Counter for fake IMAP UIDs in [recv_msg], for private use in that function only.
    recv_idx: RwLock<u32>,
}

impl TestContext {
    /// Create a new [TestContext].
    ///
    /// The [Context] will be created and have an SQLite database named "db.sqlite" in the
    /// [TestContext.dir] directory.  This directory is cleaned up when the [TestContext] is
    /// dropped.
    ///
    /// [Context]: crate::context::Context
    pub async fn new() -> Self {
        use rand::Rng;

        let dir = tempdir().unwrap();
        let dbfile = dir.path().join("db.sqlite");
        let id = rand::thread_rng().gen();
        let ctx = Context::new("FakeOS".into(), dbfile.into(), id)
            .await
            .unwrap();
        Self {
            ctx,
            dir,
            recv_idx: RwLock::new(0),
        }
    }

    /// Create a new configured [TestContext].
    ///
    /// This is a shortcut which automatically calls [TestContext::configure_alice] after
    /// creating the context.
    pub async fn new_alice() -> Self {
        let t = Self::new().await;
        t.configure_alice().await;
        t
    }

    /// Create a new configured [TestContext].
    ///
    /// This is a shortcut which configures bob@example.net with a fixed key.
    pub async fn new_bob() -> Self {
        let t = Self::new().await;
        let keypair = bob_keypair();
        t.configure_addr(&keypair.addr.to_string()).await;
        key::store_self_keypair(&t.ctx, &keypair, key::KeyPairUse::Default)
            .await
            .expect("Failed to save Bob's key");
        t
    }

    /// Configure with alice@example.com.
    ///
    /// The context will be fake-configured as the alice user, with a pre-generated secret
    /// key.  The email address of the user is returned as a string.
    pub async fn configure_alice(&self) -> String {
        let keypair = alice_keypair();
        self.configure_addr(&keypair.addr.to_string()).await;
        key::store_self_keypair(&self.ctx, &keypair, key::KeyPairUse::Default)
            .await
            .expect("Failed to save Alice's key");
        keypair.addr.to_string()
    }

    /// Configure as a given email address.
    ///
    /// The context will be configured but the key will not be pre-generated so if a key is
    /// used the fingerprint will be different every time.
    pub async fn configure_addr(&self, addr: &str) {
        self.ctx.set_config(Config::Addr, Some(addr)).await.unwrap();
        self.ctx
            .set_config(Config::ConfiguredAddr, Some(addr))
            .await
            .unwrap();
        self.ctx
            .set_config(Config::Configured, Some("1"))
            .await
            .unwrap();
    }

    /// Retrieve a sent message from the jobs table.
    ///
    /// This retrieves and removes a message which has been scheduled to send from the jobs
    /// table.  Messages are returned in the order they have been sent.
    ///
    /// Panics if there is no message or on any error.
    pub async fn pop_sent_msg(&self) -> SentMessage {
        let start = Instant::now();
        let (rowid, foreign_id, raw_params) = loop {
            let row = self
                .ctx
                .sql
                .query_row(
                    r#"
                    SELECT id, foreign_id, param
                      FROM jobs
                     WHERE action=?
                  ORDER BY desired_timestamp;
                "#,
                    paramsv![Action::SendMsgToSmtp],
                    |row| {
                        let id: i64 = row.get(0)?;
                        let foreign_id: i64 = row.get(1)?;
                        let param: String = row.get(2)?;
                        Ok((id, foreign_id, param))
                    },
                )
                .await;
            if let Ok(row) = row {
                break row;
            }
            if start.elapsed() < Duration::from_secs(3) {
                async_std::task::sleep(Duration::from_millis(100)).await;
            } else {
                panic!("no sent message found in jobs table");
            }
        };
        let id = ChatId::new(foreign_id as u32);
        let params = Params::from_str(&raw_params).unwrap();
        let blob_path = params
            .get_blob(Param::File, &self.ctx, false)
            .await
            .expect("failed to parse blob from param")
            .expect("no Param::File found in Params")
            .to_abs_path();
        self.ctx
            .sql
            .execute("DELETE FROM jobs WHERE id=?;", paramsv![rowid])
            .await
            .expect("failed to remove job");
        SentMessage {
            id,
            params,
            blob_path,
        }
    }

    /// Parse a message.
    ///
    /// Parsing a message does not run the entire receive pipeline, but is not without
    /// side-effects either.  E.g. if the message includes autocrypt headers the relevant
    /// peerstates will be updated.  Later receiving the message using [recv_msg] is
    /// unlikely to be affected as the peerstate would be processed again in exactly the
    /// same way.
    pub async fn parse_msg(&self, msg: &SentMessage) -> MimeMessage {
        MimeMessage::from_bytes(&self.ctx, msg.payload().as_bytes())
            .await
            .unwrap()
    }

    /// Receive a message.
    ///
    /// Receives a message using the `dc_receive_imf()` pipeline.
    pub async fn recv_msg(&self, msg: &SentMessage) {
        let mut idx = self.recv_idx.write().await;
        *idx += 1;
        dc_receive_imf(&self.ctx, msg.payload().as_bytes(), "INBOX", *idx, false)
            .await
            .unwrap();
    }
}

/// A raw message as it was scheduled to be sent.
///
/// This is a raw message, probably in the shape DC was planning to send it but not having
/// passed through a SMTP-IMAP pipeline.
#[derive(Debug, Clone)]
pub struct SentMessage {
    id: ChatId,
    params: Params,
    blob_path: PathBuf,
}

impl SentMessage {
    /// The ChatId the message belonged to.
    pub fn id(&self) -> ChatId {
        self.id
    }

    /// A recipient the message was destined for.
    ///
    /// If there are multiple recipients this is just a random one, so is not very useful.
    pub fn recipient(&self) -> EmailAddress {
        let raw = self
            .params
            .get(Param::Recipients)
            .expect("no recipients in params");
        let rcpt = raw.split(' ').next().expect("no recipient found");
        rcpt.parse().expect("failed to parse email address")
    }

    /// The raw message payload.
    pub fn payload(&self) -> String {
        std::fs::read_to_string(&self.blob_path).unwrap()
    }
}

/// Load a pre-generated keypair for alice@example.com from disk.
///
/// This saves CPU cycles by avoiding having to generate a key.
///
/// The keypair was created using the crate::key::tests::gen_key test.
pub(crate) fn alice_keypair() -> key::KeyPair {
    let addr = EmailAddress::new("alice@example.com").unwrap();
    let public =
        key::SignedPublicKey::from_base64(include_str!("../test-data/key/alice-public.asc"))
            .unwrap();
    let secret =
        key::SignedSecretKey::from_base64(include_str!("../test-data/key/alice-secret.asc"))
            .unwrap();
    key::KeyPair {
        addr,
        public,
        secret,
    }
}

/// Load a pre-generated keypair for bob@example.net from disk.
///
/// Like [alice_keypair] but a different key and identity.
pub(crate) fn bob_keypair() -> key::KeyPair {
    let addr = EmailAddress::new("bob@example.net").unwrap();
    let public =
        key::SignedPublicKey::from_base64(include_str!("../test-data/key/bob-public.asc")).unwrap();
    let secret =
        key::SignedSecretKey::from_base64(include_str!("../test-data/key/bob-secret.asc")).unwrap();
    key::KeyPair {
        addr,
        public,
        secret,
    }
}

pub(crate) async fn get_chat_msg(
    t: &TestContext,
    chat_id: ChatId,
    get_index: usize,
    asserted_msgs_count: usize,
) -> Message {
    let msgs = chat::get_chat_msgs(&t.ctx, chat_id, 0, None).await;
    assert_eq!(msgs.len(), asserted_msgs_count);
    let msg_id = if let ChatItem::Message { msg_id } = msgs[get_index] {
        msg_id
    } else {
        panic!("Wrong item type");
    };
    Message::load_from_db(&t.ctx, msg_id).await.unwrap()
}
