//! Utilities to help writing tests.
//!
//! This module is only compiled for test runs.

use std::time::{Duration, Instant};
use std::{ops::Deref, str::FromStr};

use ansi_term::Color;
use async_std::path::PathBuf;
use async_std::sync::RwLock;
use tempfile::{tempdir, TempDir};

use crate::config::Config;
use crate::constants::DC_CONTACT_ID_SELF;
use crate::contact::Contact;
use crate::context::Context;
use crate::dc_receive_imf::dc_receive_imf;
use crate::dc_tools::EmailAddress;
use crate::job::Action;
use crate::key::{self, DcKey};
use crate::message::{update_msg_state, Message, MessageState, MsgId};
use crate::mimeparser::MimeMessage;
use crate::param::{Param, Params};
use crate::{chat, chatlist::Chatlist};
use crate::{chat::ChatItem, EventType};
use crate::{
    chat::{Chat, ChatId},
    contact::Origin,
};

use crate::constants::Viewtype;
use crate::constants::DC_MSG_ID_DAYMARKER;
use crate::constants::DC_MSG_ID_MARKER1;

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

        let events = ctx.get_event_emitter();
        async_std::task::spawn(async move {
            while let Some(event) = events.recv().await {
                receive_event(event.typ);
            }
        });
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
        key::store_self_keypair(&t, &keypair, key::KeyPairUse::Default)
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
                  ORDER BY desired_timestamp DESC;
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
        let id = MsgId::new(foreign_id as u32);
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
        update_msg_state(&self.ctx, id, MessageState::OutDelivered).await;
        SentMessage {
            params,
            blob_path,
            sender_msg_id: id,
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
        let received_msg =
            "Received: (Postfix, from userid 1000); Mon, 4 Dec 2006 14:51:39 +0100 (CET)\n"
                .to_owned()
                + &msg.payload();
        dc_receive_imf(&self.ctx, received_msg.as_bytes(), "INBOX", *idx, false)
            .await
            .unwrap();
    }

    /// Get the most recent message of a chat.
    ///
    /// Panics on errors or if the most recent message is a marker.
    pub async fn get_last_msg_in(&self, chat_id: ChatId) -> Message {
        let msgs = chat::get_chat_msgs(&self.ctx, chat_id, 0, None).await;
        let msg_id = if let ChatItem::Message { msg_id } = msgs.last().unwrap() {
            msg_id
        } else {
            panic!("Wrong item type");
        };
        Message::load_from_db(&self.ctx, *msg_id).await.unwrap()
    }

    /// Get the most recent message over all chats.
    pub async fn get_last_msg(&self) -> Message {
        let chats = Chatlist::try_load(&self.ctx, 0, None, None).await.unwrap();
        let msg_id = chats.get_msg_id(chats.len() - 1).unwrap();
        Message::load_from_db(&self.ctx, msg_id).await.unwrap()
    }

    pub async fn create_chat(&self, other: &TestContext) -> Chat {
        let (contact_id, _modified) = Contact::add_or_lookup(
            self,
            other
                .ctx
                .get_config(Config::Displayname)
                .await
                .unwrap_or_default(),
            other.ctx.get_config(Config::ConfiguredAddr).await.unwrap(),
            Origin::ManuallyCreated,
        )
        .await
        .unwrap();

        let chat_id = chat::create_by_contact_id(self, contact_id).await.unwrap();
        Chat::load_from_db(self, chat_id).await.unwrap()
    }

    pub async fn chat_with_contact(&self, name: &str, addr: &str) -> Chat {
        let contact = Contact::create(self, name, addr)
            .await
            .expect("failed to create contact");
        let chat_id = chat::create_by_contact_id(self, contact).await.unwrap();
        Chat::load_from_db(self, chat_id).await.unwrap()
    }

    pub async fn get_self_chat(&self) -> Chat {
        let chat_id = chat::create_by_contact_id(self, DC_CONTACT_ID_SELF)
            .await
            .unwrap();
        Chat::load_from_db(self, chat_id).await.unwrap()
    }

    /// Sends out the text message. If the other side shall receive it, you have to call `recv_msg()` with the returned `SentMessage`.
    pub async fn send_text(&self, chat_id: ChatId, txt: &str) -> SentMessage {
        let mut msg = Message::new(Viewtype::Text);
        msg.set_text(Some(txt.to_string()));
        chat::prepare_msg(&self, chat_id, &mut msg).await.unwrap();
        chat::send_msg(&self, chat_id, &mut msg).await.unwrap();
        self.pop_sent_msg().await
    }

    /// You can use this to debug your test by printing a chat structure
    // This code is mainly the same as `log_msglist` in `cmdline.rs`, so one day, we could merge them to a public function in the `deltachat` crate.
    #[allow(dead_code)]
    pub async fn print_chat(&self, chat: &Chat) {
        let msglist = chat::get_chat_msgs(&self, chat.get_id(), 0x1, None).await;
        let msglist: Vec<MsgId> = msglist
            .into_iter()
            .map(|x| match x {
                ChatItem::Message { msg_id } => msg_id,
                ChatItem::Marker1 => MsgId::new(DC_MSG_ID_MARKER1),
                ChatItem::DayMarker { .. } => MsgId::new(DC_MSG_ID_DAYMARKER),
            })
            .collect();

        let mut lines_out = 0;
        for msg_id in msglist {
            if msg_id == MsgId::new(DC_MSG_ID_DAYMARKER) {
                println!(
                "--------------------------------------------------------------------------------"
            );

                lines_out += 1
            } else if !msg_id.is_special() {
                if lines_out == 0 {
                    println!(
                    "--------------------------------------------------------------------------------",
                );
                    lines_out += 1
                }
                let msg = Message::load_from_db(&self, msg_id).await.unwrap();
                log_msg(self, "", &msg).await;
            }
        }
        if lines_out > 0 {
            println!(
                "--------------------------------------------------------------------------------"
            );
        }
    }
}

impl Deref for TestContext {
    type Target = Context;

    fn deref(&self) -> &Context {
        &self.ctx
    }
}

/// A raw message as it was scheduled to be sent.
///
/// This is a raw message, probably in the shape DC was planning to send it but not having
/// passed through a SMTP-IMAP pipeline.
#[derive(Debug, Clone)]
pub struct SentMessage {
    params: Params,
    blob_path: PathBuf,
    pub sender_msg_id: MsgId,
}

impl SentMessage {
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

fn receive_event(event: EventType) {
    let green = Color::Green.normal();
    let yellow = Color::Yellow.normal();
    let red = Color::Red.normal();

    match event {
        EventType::Info(msg) => {
            /* do not show the event as this would fill the screen */
            println!("{}", msg);
        }
        EventType::SmtpConnected(msg) => {
            println!("[SMTP_CONNECTED] {}", msg);
        }
        EventType::ImapConnected(msg) => {
            println!("[IMAP_CONNECTED] {}", msg);
        }
        EventType::SmtpMessageSent(msg) => {
            println!("[SMTP_MESSAGE_SENT] {}", msg);
        }
        EventType::Warning(msg) => {
            println!("{}", yellow.paint(msg));
        }
        EventType::Error(msg) => {
            println!("{}", red.paint(msg));
        }
        EventType::ErrorNetwork(msg) => {
            println!("{}", red.paint(format!("[NETWORK] msg={}", msg)));
        }
        EventType::ErrorSelfNotInGroup(msg) => {
            println!("{}", red.paint(format!("[SELF_NOT_IN_GROUP] {}", msg)));
        }
        EventType::MsgsChanged { chat_id, msg_id } => {
            println!(
                "{}",
                green.paint(format!(
                    "Received MSGS_CHANGED(chat_id={}, msg_id={})",
                    chat_id, msg_id,
                ))
            );
        }
        EventType::ContactsChanged(_) => {
            println!("{}", green.paint("Received CONTACTS_CHANGED()"));
        }
        EventType::LocationChanged(contact) => {
            println!(
                "{}",
                green.paint(format!("Received LOCATION_CHANGED(contact={:?})", contact))
            );
        }
        EventType::ConfigureProgress { progress, comment } => {
            if let Some(comment) = comment {
                println!(
                    "{}",
                    green.paint(format!(
                        "Received CONFIGURE_PROGRESS({} ‰, {})",
                        progress, comment
                    ))
                );
            } else {
                println!(
                    "{}",
                    green.paint(format!("Received CONFIGURE_PROGRESS({} ‰)", progress))
                );
            }
        }
        EventType::ImexProgress(progress) => {
            println!(
                "{}",
                green.paint(format!("Received IMEX_PROGRESS({} ‰)", progress))
            );
        }
        EventType::ImexFileWritten(file) => {
            println!(
                "{}",
                green.paint(format!("Received IMEX_FILE_WRITTEN({})", file.display()))
            );
        }
        EventType::ChatModified(chat) => {
            println!(
                "{}",
                green.paint(format!("Received CHAT_MODIFIED({})", chat))
            );
        }
        _ => {
            println!("Received {:?}", event);
        }
    }
}

async fn log_msg(context: &Context, prefix: impl AsRef<str>, msg: &Message) {
    let contact = Contact::get_by_id(context, msg.get_from_id())
        .await
        .expect("invalid contact");

    let contact_name = contact.get_name();
    let contact_id = contact.get_id();

    let statestr = match msg.get_state() {
        MessageState::OutPending => " o",
        MessageState::OutDelivered => " √",
        MessageState::OutMdnRcvd => " √√",
        MessageState::OutFailed => " !!",
        _ => "",
    };
    let msgtext = msg.get_text();
    println!(
        "{}{}{}{}: {} (Contact#{}): {} {}{}{}{}{}",
        prefix.as_ref(),
        msg.get_id(),
        if msg.get_showpadlock() { "🔒" } else { "" },
        if msg.has_location() { "📍" } else { "" },
        &contact_name,
        contact_id,
        msgtext.unwrap_or_default(),
        if msg.get_from_id() == 1 as libc::c_uint {
            ""
        } else if msg.get_state() == MessageState::InSeen {
            "[SEEN]"
        } else if msg.get_state() == MessageState::InNoticed {
            "[NOTICED]"
        } else {
            "[FRESH]"
        },
        if msg.is_info() { "[INFO]" } else { "" },
        if msg.get_viewtype() == Viewtype::VideochatInvitation {
            format!(
                "[VIDEOCHAT-INVITATION: {}, type={}]",
                msg.get_videochat_url().unwrap_or_default(),
                msg.get_videochat_type().unwrap_or_default()
            )
        } else {
            "".to_string()
        },
        if msg.is_forwarded() {
            "[FORWARDED]"
        } else {
            ""
        },
        statestr,
    );
}
