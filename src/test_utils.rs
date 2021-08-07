//! Utilities to help writing tests.
//!
//! This private module is only compiled for test runs.

use std::ops::Deref;
use std::str::FromStr;
use std::time::{Duration, Instant};
use std::{collections::BTreeMap, panic};
use std::{fmt, thread};

use ansi_term::Color;
use async_std::channel::Receiver;
use async_std::path::PathBuf;
use async_std::sync::{Arc, RwLock};
use async_std::{channel, pin::Pin};
use async_std::{future::Future, task};
use chat::ChatItem;
use once_cell::sync::Lazy;
use tempfile::{tempdir, TempDir};

use crate::chat::{self, Chat, ChatId};
use crate::chatlist::Chatlist;
use crate::config::Config;
use crate::constants::Chattype;
use crate::constants::{Viewtype, DC_CONTACT_ID_SELF, DC_MSG_ID_DAYMARKER, DC_MSG_ID_MARKER1};
use crate::contact::{Contact, Origin};
use crate::context::Context;
use crate::dc_receive_imf::dc_receive_imf;
use crate::dc_tools::EmailAddress;
use crate::events::{Event, EventType};
use crate::job::Action;
use crate::key::{self, DcKey};
use crate::message::{update_msg_state, Message, MessageState, MsgId};
use crate::mimeparser::MimeMessage;
use crate::param::{Param, Params};

type EventSink =
    dyn Fn(Event) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>> + Send + Sync + 'static;

/// Map of [`Context::id`] to names for [`TestContext`]s.
static CONTEXT_NAMES: Lazy<std::sync::RwLock<BTreeMap<u32, String>>> =
    Lazy::new(|| std::sync::RwLock::new(BTreeMap::new()));

/// A Context and temporary directory.
///
/// The temporary directory can be used to store the SQLite database,
/// see e.g. [test_context] which does this.
pub(crate) struct TestContext {
    pub ctx: Context,
    pub dir: TempDir,
    pub evtracker: EvTracker,
    /// Counter for fake IMAP UIDs in [recv_msg], for private use in that function only.
    recv_idx: RwLock<u32>,
    /// Functions to call for events received.
    event_sinks: Arc<RwLock<Vec<Box<EventSink>>>>,
    /// Receives panics from sinks ("sink" means "event handler" here)
    poison_receiver: channel::Receiver<String>,
}

impl fmt::Debug for TestContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TestContext")
            .field("ctx", &self.ctx)
            .field("dir", &self.dir)
            .field("recv_idx", &self.recv_idx)
            .field("event_sinks", &String::from("Vec<EventSink>"))
            .finish()
    }
}

impl TestContext {
    /// Creates a new [`TestContext`].
    ///
    /// The [Context] will be created and have an SQLite database named "db.sqlite" in the
    /// [TestContext.dir] directory.  This directory is cleaned up when the [TestContext] is
    /// dropped.
    ///
    /// [Context]: crate::context::Context
    pub async fn new() -> Self {
        Self::new_named(None).await
    }

    /// Creates a new [`TestContext`] with a set name used in event logging.
    pub async fn with_name(name: impl Into<String>) -> Self {
        Self::new_named(Some(name.into())).await
    }

    async fn new_named(name: Option<String>) -> Self {
        use rand::Rng;
        pretty_env_logger::try_init().ok();

        let dir = tempdir().unwrap();
        let dbfile = dir.path().join("db.sqlite");
        let id = rand::thread_rng().gen();
        if let Some(name) = name {
            let mut context_names = CONTEXT_NAMES.write().unwrap();
            context_names.insert(id, name);
        }
        let ctx = Context::new("FakeOS".into(), dbfile.into(), id)
            .await
            .expect("failed to create context");

        let events = ctx.get_event_emitter();

        let event_sinks: Arc<RwLock<Vec<Box<EventSink>>>> = Arc::new(RwLock::new(Vec::new()));
        let sinks = Arc::clone(&event_sinks);
        let (poison_sender, poison_receiver) = channel::bounded(1);
        let (evtracker_sender, evtracker_receiver) = channel::unbounded();

        async_std::task::spawn(async move {
            // Make sure that the test fails if there is a panic on this thread here:
            let current_id = task::current().id();
            let orig_hook = panic::take_hook();
            panic::set_hook(Box::new(move |panic_info| {
                if task::current().id() == current_id {
                    poison_sender.try_send(panic_info.to_string()).ok();
                }
                orig_hook(panic_info);
            }));

            while let Some(event) = events.recv().await {
                {
                    log::debug!("{:?}", event);
                    let sinks = sinks.read().await;
                    for sink in sinks.iter() {
                        sink(event.clone()).await;
                    }
                }
                receive_event(&event);
                evtracker_sender.send(event.typ).await.ok();
            }
        });

        Self {
            ctx,
            dir,
            evtracker: EvTracker(evtracker_receiver),
            recv_idx: RwLock::new(0),
            event_sinks,
            poison_receiver,
        }
    }

    /// Creates a new configured [`TestContext`].
    ///
    /// This is a shortcut which automatically calls [`TestContext::configure_alice`] after
    /// creating the context.
    pub async fn new_alice() -> Self {
        let t = Self::with_name("alice").await;
        t.configure_alice().await;
        t
    }

    /// Creates a new configured [`TestContext`].
    ///
    /// This is a shortcut which configures bob@example.net with a fixed key.
    pub async fn new_bob() -> Self {
        let t = Self::with_name("bob").await;
        let keypair = bob_keypair();
        t.configure_addr(&keypair.addr.to_string()).await;
        key::store_self_keypair(&t, &keypair, key::KeyPairUse::Default)
            .await
            .expect("Failed to save Bob's key");
        t
    }

    /// Sets a name for this [`TestContext`] if one isn't yet set.
    ///
    /// This will show up in events logged in the test output.
    pub fn set_name(&self, name: impl Into<String>) {
        let mut context_names = CONTEXT_NAMES.write().unwrap();
        context_names
            .entry(self.ctx.get_id())
            .or_insert_with(|| name.into());
    }

    /// Add a new callback which will receive events.
    ///
    /// The test context runs an async task receiving all events from the [`Context`], which
    /// are logged to stdout.  This allows you to register additional callbacks which will
    /// receive all events in case your tests need to watch for a specific event.
    pub async fn add_event_sink<F, R>(&self, sink: F)
    where
        // Aka `F: EventSink` but type aliases are not allowed.
        F: Fn(Event) -> R + Send + Sync + 'static,
        R: Future<Output = ()> + Send + 'static,
    {
        let mut sinks = self.event_sinks.write().await;
        sinks.push(Box::new(move |evt| Box::pin(sink(evt))));
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
        if let Some(name) = addr.split('@').next() {
            self.set_name(name);
        }
    }

    /// Retrieves a sent message from the jobs table.
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
                        let id: u32 = row.get(0)?;
                        let foreign_id: u32 = row.get(1)?;
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
        let id = MsgId::new(foreign_id);
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

    /// Parses a message.
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

    /// Gets the most recent message of a chat.
    ///
    /// Panics on errors or if the most recent message is a marker.
    pub async fn get_last_msg_in(&self, chat_id: ChatId) -> Message {
        let msgs = chat::get_chat_msgs(&self.ctx, chat_id, 0, None)
            .await
            .unwrap();
        let msg_id = if let ChatItem::Message { msg_id } = msgs.last().unwrap() {
            msg_id
        } else {
            panic!("Wrong item type");
        };
        Message::load_from_db(&self.ctx, *msg_id).await.unwrap()
    }

    /// Gets the most recent message over all chats.
    pub async fn get_last_msg(&self) -> Message {
        let chats = Chatlist::try_load(&self.ctx, 0, None, None)
            .await
            .expect("failed to load chatlist");
        // 0 is correct in the next line (as opposed to `chats.len() - 1`, which would be the last element):
        // The chatlist describes what you see when you open DC, a list of chats and in each of them
        // the first words of the last message. To get the last message overall, we look at the chat at the top of the
        // list, which has the index 0.
        let msg_id = chats.get_msg_id(0).unwrap().unwrap();
        Message::load_from_db(&self.ctx, msg_id)
            .await
            .expect("failed to load msg")
    }

    /// Creates or returns an existing 1:1 [`Chat`] with another account.
    ///
    /// This first creates a contact using the configured details on the other account, then
    /// creates a 1:1 chat with this contact.
    pub async fn create_chat(&self, other: &TestContext) -> Chat {
        let (contact_id, _modified) = Contact::add_or_lookup(
            self,
            &other
                .ctx
                .get_config(Config::Displayname)
                .await
                .unwrap_or_default()
                .unwrap_or_default(),
            &other
                .ctx
                .get_config(Config::ConfiguredAddr)
                .await
                .unwrap()
                .unwrap(),
            Origin::ManuallyCreated,
        )
        .await
        .unwrap();

        let chat_id = ChatId::create_for_contact(self, contact_id).await.unwrap();
        Chat::load_from_db(self, chat_id).await.unwrap()
    }

    /// Creates or returns an existing [`Contact`] and 1:1 [`Chat`] with another email.
    ///
    /// This first creates a contact from the `name` and `addr` and then creates a 1:1 chat
    /// with this contact.
    pub async fn create_chat_with_contact(&self, name: &str, addr: &str) -> Chat {
        let contact = Contact::create(self, name, addr)
            .await
            .expect("failed to create contact");
        let chat_id = ChatId::create_for_contact(self, contact).await.unwrap();
        Chat::load_from_db(self, chat_id).await.unwrap()
    }

    /// Retrieves the "self" chat.
    pub async fn get_self_chat(&self) -> Chat {
        let chat_id = ChatId::create_for_contact(self, DC_CONTACT_ID_SELF)
            .await
            .unwrap();
        Chat::load_from_db(self, chat_id).await.unwrap()
    }

    /// Sends out the text message.
    ///
    /// This is not hooked up to any SMTP-IMAP pipeline, so the other account must call
    /// [`TestContext::recv_msg`] with the returned [`SentMessage`] if it wants to receive
    /// the message.
    pub async fn send_text(&self, chat_id: ChatId, txt: &str) -> SentMessage {
        let mut msg = Message::new(Viewtype::Text);
        msg.set_text(Some(txt.to_string()));
        self.send_msg(chat_id, &mut msg).await
    }

    /// Sends out the message to the specified chat.
    ///
    /// This is not hooked up to any SMTP-IMAP pipeline, so the other account must call
    /// [`TestContext::recv_msg`] with the returned [`SentMessage`] if it wants to receive
    /// the message.
    pub async fn send_msg(&self, chat_id: ChatId, msg: &mut Message) -> SentMessage {
        chat::prepare_msg(self, chat_id, msg).await.unwrap();
        chat::send_msg(self, chat_id, msg).await.unwrap();
        self.pop_sent_msg().await
    }

    /// Prints out the entire chat to stdout.
    ///
    /// You can use this to debug your test by printing the entire chat conversation.
    // This code is mainly the same as `log_msglist` in `cmdline.rs`, so one day, we could
    // merge them to a public function in the `deltachat` crate.
    #[allow(dead_code)]
    #[allow(clippy::indexing_slicing)]
    pub async fn print_chat(&self, chat_id: ChatId) {
        let msglist = chat::get_chat_msgs(self, chat_id, 0x1, None).await.unwrap();
        let msglist: Vec<MsgId> = msglist
            .into_iter()
            .map(|x| match x {
                ChatItem::Message { msg_id } => msg_id,
                ChatItem::Marker1 => MsgId::new(DC_MSG_ID_MARKER1),
                ChatItem::DayMarker { .. } => MsgId::new(DC_MSG_ID_DAYMARKER),
            })
            .collect();

        let sel_chat = Chat::load_from_db(self, chat_id).await.unwrap();
        let members = chat::get_chat_contacts(self, sel_chat.id).await.unwrap();
        let subtitle = if sel_chat.is_device_talk() {
            "device-talk".to_string()
        } else if sel_chat.get_type() == Chattype::Single && !members.is_empty() {
            let contact = Contact::get_by_id(self, members[0]).await.unwrap();
            contact.get_addr().to_string()
        } else if sel_chat.get_type() == Chattype::Mailinglist && !members.is_empty() {
            "mailinglist".to_string()
        } else {
            format!("{} member(s)", members.len())
        };
        println!(
            "{}#{}: {} [{}]{}{}{} {}",
            sel_chat.typ,
            sel_chat.get_id(),
            sel_chat.get_name(),
            subtitle,
            if sel_chat.is_muted() { "🔇" } else { "" },
            if sel_chat.is_sending_locations() {
                "📍"
            } else {
                ""
            },
            match sel_chat.get_profile_image(self).await.unwrap() {
                Some(icon) => match icon.to_str() {
                    Some(icon) => format!(" Icon: {}", icon),
                    _ => " Icon: Err".to_string(),
                },
                _ => "".to_string(),
            },
            if sel_chat.is_protected() {
                "🛡️"
            } else {
                ""
            },
        );

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
                let msg = Message::load_from_db(self, msg_id).await.unwrap();
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

impl Drop for TestContext {
    fn drop(&mut self) {
        if !thread::panicking() {
            if let Ok(p) = self.poison_receiver.try_recv() {
                panic!("{}", p);
            }
        }
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
pub fn alice_keypair() -> key::KeyPair {
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
pub fn bob_keypair() -> key::KeyPair {
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

pub struct EvTracker(Receiver<EventType>);

impl EvTracker {
    pub async fn get_info_contains(&self, s: &str) -> EventType {
        loop {
            let event = self.0.recv().await.unwrap();
            if let EventType::Info(i) = &event {
                if i.contains(s) {
                    return event;
                }
            }
        }
    }
}

impl Deref for EvTracker {
    type Target = Receiver<EventType>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Gets a specific message from a chat and asserts that the chat has a specific length.
///
/// Panics if the length of the chat is not `asserted_msgs_count` or if the chat item at `index` is not a Message.
#[allow(clippy::indexing_slicing)]
pub(crate) async fn get_chat_msg(
    t: &TestContext,
    chat_id: ChatId,
    index: usize,
    asserted_msgs_count: usize,
) -> Message {
    let msgs = chat::get_chat_msgs(&t.ctx, chat_id, 0, None).await.unwrap();
    assert_eq!(msgs.len(), asserted_msgs_count);
    let msg_id = if let ChatItem::Message { msg_id } = msgs[index] {
        msg_id
    } else {
        panic!("Wrong item type");
    };
    Message::load_from_db(&t.ctx, msg_id).await.unwrap()
}

/// Pretty-print an event to stdout
///
/// Done during tests this is captured by `cargo test` and associated with the test itself.
fn receive_event(event: &Event) {
    let green = Color::Green.normal();
    let yellow = Color::Yellow.normal();
    let red = Color::Red.normal();

    let msg = match &event.typ {
        EventType::Info(msg) => format!("INFO: {}", msg),
        EventType::SmtpConnected(msg) => format!("[SMTP_CONNECTED] {}", msg),
        EventType::ImapConnected(msg) => format!("[IMAP_CONNECTED] {}", msg),
        EventType::SmtpMessageSent(msg) => format!("[SMTP_MESSAGE_SENT] {}", msg),
        EventType::Warning(msg) => format!("WARN: {}", yellow.paint(msg)),
        EventType::Error(msg) => format!("ERROR: {}", red.paint(msg)),
        EventType::ErrorSelfNotInGroup(msg) => {
            format!("{}", red.paint(format!("[SELF_NOT_IN_GROUP] {}", msg)))
        }
        EventType::MsgsChanged { chat_id, msg_id } => format!(
            "{}",
            green.paint(format!(
                "Received MSGS_CHANGED(chat_id={}, msg_id={})",
                chat_id, msg_id,
            ))
        ),
        EventType::ContactsChanged(_) => format!("{}", green.paint("Received CONTACTS_CHANGED()")),
        EventType::LocationChanged(contact) => format!(
            "{}",
            green.paint(format!("Received LOCATION_CHANGED(contact={:?})", contact))
        ),
        EventType::ConfigureProgress { progress, comment } => {
            if let Some(comment) = comment {
                format!(
                    "{}",
                    green.paint(format!(
                        "Received CONFIGURE_PROGRESS({} ‰, {})",
                        progress, comment
                    ))
                )
            } else {
                format!(
                    "{}",
                    green.paint(format!("Received CONFIGURE_PROGRESS({} ‰)", progress))
                )
            }
        }
        EventType::ImexProgress(progress) => format!(
            "{}",
            green.paint(format!("Received IMEX_PROGRESS({} ‰)", progress))
        ),
        EventType::ImexFileWritten(file) => format!(
            "{}",
            green.paint(format!("Received IMEX_FILE_WRITTEN({})", file.display()))
        ),
        EventType::ChatModified(chat) => format!(
            "{}",
            green.paint(format!("Received CHAT_MODIFIED({})", chat))
        ),
        _ => format!("Received {:?}", event),
    };
    let context_names = CONTEXT_NAMES.read().unwrap();
    match context_names.get(&event.id) {
        Some(name) => println!("{} {}", name, msg),
        None => println!("{} {}", event.id, msg),
    }
}

/// Logs an individual message to stdout.
///
/// This includes a bunch of the message meta-data as well.
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
        if msg.get_from_id() == 1u32 {
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
