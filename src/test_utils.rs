//! Utilities to help writing tests.
//!
//! This private module is only compiled for test runs.

use std::collections::BTreeMap;
use std::ops::Deref;
use std::panic;
use std::thread;
use std::time::{Duration, Instant};

use ansi_term::Color;
use async_std::channel::{self, Receiver, Sender};
use async_std::prelude::*;
use async_std::sync::{Arc, RwLock};
use async_std::task;
use chat::ChatItem;
use once_cell::sync::Lazy;
use rand::Rng;
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
use crate::key::{self, DcKey, KeyPair, KeyPairUse};
use crate::message::{update_msg_state, Message, MessageState, MsgId};
use crate::mimeparser::MimeMessage;

#[allow(non_upper_case_globals)]
pub const AVATAR_900x900_BYTES: &[u8] = include_bytes!("../test-data/image/avatar900x900.png");

/// Map of [`Context::id`] to names for [`TestContext`]s.
static CONTEXT_NAMES: Lazy<std::sync::RwLock<BTreeMap<u32, String>>> =
    Lazy::new(|| std::sync::RwLock::new(BTreeMap::new()));

#[derive(Debug, Clone, Default)]
pub struct TestContextBuilder {
    key_pair: Option<KeyPair>,
    log_sink: Option<Sender<Event>>,
}

impl TestContextBuilder {
    /// Configures as alice@example.org with fixed secret key.
    ///
    /// This is a shortcut for `.with_key_pair(alice_keypair()).
    pub fn configure_alice(self) -> Self {
        self.with_key_pair(alice_keypair())
    }

    /// Configures as bob@example.net with fixed secret key.
    ///
    /// This is a shortcut for `.with_key_pair(bob_keypair()).
    pub fn configure_bob(self) -> Self {
        self.with_key_pair(bob_keypair())
    }

    /// Configures the new [`TestContext`] with the provided [`KeyPair`].
    ///
    /// This will extract the email address from the key and configure the context with the
    /// given identity.
    pub fn with_key_pair(mut self, key_pair: KeyPair) -> Self {
        self.key_pair = Some(key_pair);
        self
    }

    /// Attaches a [`LogSink`] to this [`TestContext`].
    ///
    /// This is useful when using multiple [`TestContext`] instances in one test: it allows
    /// using a single [`LogSink`] for both contexts.  This shows the log messages in
    /// sequence as they occurred rather than all messages from each context in a single
    /// block.
    pub fn with_log_sink(mut self, sink: Sender<Event>) -> Self {
        self.log_sink = Some(sink);
        self
    }

    /// Builds the [`TestContext`].
    pub async fn build(self) -> TestContext {
        let name = self.key_pair.as_ref().map(|key| key.addr.local.clone());

        let test_context = TestContext::new_internal(name, self.log_sink).await;

        if let Some(key_pair) = self.key_pair {
            test_context
                .configure_addr(&key_pair.addr.to_string())
                .await;
            key::store_self_keypair(&test_context, &key_pair, KeyPairUse::Default)
                .await
                .expect("Failed to save key");
        }
        test_context
    }
}

/// A Context and temporary directory.
///
/// The temporary directory can be used to store the SQLite database,
/// see e.g. [test_context] which does this.
#[derive(Debug)]
pub struct TestContext {
    pub ctx: Context,
    pub dir: TempDir,
    pub evtracker: EventTracker,
    /// Channels which should receive events from this context.
    event_senders: Arc<RwLock<Vec<Sender<Event>>>>,
    /// Receives panics from sinks ("sink" means "event handler" here)
    poison_receiver: Receiver<String>,
    /// Reference to implicit [`LogSink`] so it is dropped together with the context.
    ///
    /// Only used if no explicit `log_sender` is passed into [`TestContext::new_internal`]
    /// (which is assumed to be the sending end of a [`LogSink`]).
    ///
    /// This is a convenience in case only a single [`TestContext`] is used to avoid dealing
    /// with [`LogSink`].  Never read, thus "dead code", since the only purpose is to
    /// control when Drop is invoked.
    #[allow(dead_code)]
    log_sink: Option<LogSink>,
}

impl TestContext {
    /// Returns the builder to have more control over creating the context.
    pub fn builder() -> TestContextBuilder {
        TestContextBuilder::default()
    }

    /// Creates a new [`TestContext`].
    ///
    /// The [Context] will be created and have an SQLite database named "db.sqlite" in the
    /// [TestContext.dir] directory.  This directory is cleaned up when the [TestContext] is
    /// dropped.
    ///
    /// [Context]: crate::context::Context
    pub async fn new() -> Self {
        Self::new_internal(None, None).await
    }

    /// Creates a new configured [`TestContext`].
    ///
    /// This is a shortcut which automatically calls [`TestContext::configure_alice`] after
    /// creating the context.
    pub async fn new_alice() -> Self {
        Self::builder().configure_alice().build().await
    }

    /// Creates a new configured [`TestContext`].
    ///
    /// This is a shortcut which configures bob@example.net with a fixed key.
    pub async fn new_bob() -> Self {
        Self::builder().configure_bob().build().await
    }

    /// Internal constructor.
    ///
    /// `name` is used to identify this context in e.g. log output.  This is useful mostly
    /// when you have multiple [`TestContext`]s in a test.
    ///
    /// `log_sender` is assumed to be the sender for a [`LogSink`].  If not supplied a new
    /// [`LogSink`] will be created so that events are logged to this test when the
    /// [`TestContext`] is dropped.
    async fn new_internal(name: Option<String>, log_sender: Option<Sender<Event>>) -> Self {
        let dir = tempdir().unwrap();
        let dbfile = dir.path().join("db.sqlite");
        let id = rand::thread_rng().gen();
        if let Some(name) = name {
            let mut context_names = CONTEXT_NAMES.write().unwrap();
            context_names.insert(id, name);
        }
        let ctx = Context::new(dbfile.into(), id)
            .await
            .expect("failed to create context");

        let events = ctx.get_event_emitter();

        let (log_sender, log_sink) = match log_sender {
            Some(sender) => (sender, None),
            None => {
                let (sender, sink) = LogSink::create();
                (sender, Some(sink))
            }
        };

        let (evtracker_sender, evtracker_receiver) = channel::unbounded();
        let event_senders = Arc::new(RwLock::new(vec![log_sender, evtracker_sender]));
        let senders = Arc::clone(&event_senders);
        let (poison_sender, poison_receiver) = channel::bounded(1);

        task::spawn(async move {
            // Make sure that the test fails if there is a panic on this thread here
            // (but not if there is a panic on another thread)
            let looptask_id = task::current().id();
            let orig_hook = panic::take_hook();
            panic::set_hook(Box::new(move |panic_info| {
                if let Some(panicked_task) = task::try_current() {
                    if panicked_task.id() == looptask_id {
                        poison_sender.try_send(panic_info.to_string()).ok();
                    }
                }
                orig_hook(panic_info);
            }));

            while let Some(event) = events.recv().await {
                {
                    let sinks = senders.read().await;
                    for sender in sinks.iter() {
                        // Don't block because someone wanted to use a oneshot receiver, use
                        // an unbounded channel if you want all events.
                        sender.try_send(event.clone()).ok();
                    }
                }
            }
        });

        Self {
            ctx,
            dir,
            evtracker: EventTracker(evtracker_receiver),
            event_senders,
            poison_receiver,
            log_sink,
        }
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

    /// Adds a new [`Event`]s sender.
    ///
    /// Once added, all events emitted by this context will be sent to this channel.  This
    /// is useful if you need to wait for events or make assertions on them.
    pub async fn add_event_sender(&self, sink: Sender<Event>) {
        self.event_senders.write().await.push(sink)
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
        let (rowid, msg_id, payload, recipients) = loop {
            let row = self
                .ctx
                .sql
                .query_row_optional(
                    r#"
                    SELECT id, msg_id, mime, recipients
                    FROM smtp
                    ORDER BY id DESC"#,
                    paramsv![],
                    |row| {
                        let rowid: i64 = row.get(0)?;
                        let msg_id: MsgId = row.get(1)?;
                        let mime: String = row.get(2)?;
                        let recipients: String = row.get(3)?;
                        Ok((rowid, msg_id, mime, recipients))
                    },
                )
                .await
                .expect("query_row_optional failed");
            if let Some(row) = row {
                break row;
            }
            if start.elapsed() < Duration::from_secs(3) {
                async_std::task::sleep(Duration::from_millis(100)).await;
            } else {
                panic!("no sent message found in jobs table");
            }
        };
        self.ctx
            .sql
            .execute("DELETE FROM jobs WHERE id=?;", paramsv![rowid])
            .await
            .expect("failed to remove job");
        update_msg_state(&self.ctx, msg_id, MessageState::OutDelivered)
            .await
            .expect("failed to update message state");
        SentMessage {
            payload,
            sender_msg_id: msg_id,
            recipients,
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
        let received_msg =
            "Received: (Postfix, from userid 1000); Mon, 4 Dec 2006 14:51:39 +0100 (CET)\n"
                .to_owned()
                + msg.payload();
        dc_receive_imf(&self.ctx, received_msg.as_bytes(), "INBOX", false)
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

/// A receiver of [`Event`]s which will log the events to the captured test stdout.
///
/// Tests redirect the stdout of the test thread and capture this, showing the captured
/// stdout if the test fails.  This means printing log messages must be done on the thread
/// of the test itself and not from a spawned task.
///
/// This sink achieves this by printing the events, in the order received, at the time it is
/// dropped.  Thus to use you must only make sure this sink is dropped in the test itself.
///
/// To use this create an instance using [`LogSink::create`] and then use the
/// [`TestContextBuilder::with_log_sink`].
#[derive(Debug)]
pub struct LogSink {
    events: Receiver<Event>,
}

impl LogSink {
    /// Creates a new [`LogSink`] and returns the attached event sink.
    pub fn create() -> (Sender<Event>, Self) {
        let (tx, rx) = channel::unbounded();
        (tx, Self { events: rx })
    }
}

impl Drop for LogSink {
    fn drop(&mut self) {
        while let Ok(event) = self.events.try_recv() {
            print_event(&event);
        }
    }
}

/// A raw message as it was scheduled to be sent.
///
/// This is a raw message, probably in the shape DC was planning to send it but not having
/// passed through a SMTP-IMAP pipeline.
#[derive(Debug, Clone)]
pub struct SentMessage {
    payload: String,
    recipients: String,
    pub sender_msg_id: MsgId,
}

impl SentMessage {
    /// A recipient the message was destined for.
    ///
    /// If there are multiple recipients this is just a random one, so is not very useful.
    pub fn recipient(&self) -> EmailAddress {
        let rcpt = self
            .recipients
            .split(' ')
            .next()
            .expect("no recipient found");
        rcpt.parse().expect("failed to parse email address")
    }

    /// The raw message payload.
    pub fn payload(&self) -> &str {
        &self.payload
    }
}

/// Load a pre-generated keypair for alice@example.org from disk.
///
/// This saves CPU cycles by avoiding having to generate a key.
///
/// The keypair was created using the crate::key::tests::gen_key test.
pub fn alice_keypair() -> KeyPair {
    let addr = EmailAddress::new("alice@example.org").unwrap();

    let public = key::SignedPublicKey::from_asc(include_str!("../test-data/key/alice-public.asc"))
        .unwrap()
        .0;
    let secret = key::SignedSecretKey::from_asc(include_str!("../test-data/key/alice-secret.asc"))
        .unwrap()
        .0;
    key::KeyPair {
        addr,
        public,
        secret,
    }
}

/// Load a pre-generated keypair for bob@example.net from disk.
///
/// Like [alice_keypair] but a different key and identity.
pub fn bob_keypair() -> KeyPair {
    let addr = EmailAddress::new("bob@example.net").unwrap();
    let public = key::SignedPublicKey::from_asc(include_str!("../test-data/key/bob-public.asc"))
        .unwrap()
        .0;
    let secret = key::SignedSecretKey::from_asc(include_str!("../test-data/key/bob-secret.asc"))
        .unwrap()
        .0;
    key::KeyPair {
        addr,
        public,
        secret,
    }
}

/// Utility to help wait for and retrieve events.
///
/// This buffers the events in order they are emitted.  This allows consuming events in
/// order while looking for the right events using the provided methods.
///
/// The methods only return [`EventType`] rather than the full [`Event`] since it can only
/// be attached to a single [`TestContext`] and therefore the context is already known as
/// you will be accessing it as [`TestContext::evtracker`].
#[derive(Debug)]
pub struct EventTracker(Receiver<Event>);

impl EventTracker {
    /// Consumes emitted events returning the first matching one.
    ///
    /// If no matching events are ready this will wait for new events to arrive and time out
    /// after 10 seconds.
    pub async fn get_matching<F: Fn(&EventType) -> bool>(&self, event_matcher: F) -> EventType {
        async move {
            loop {
                let event = self.0.recv().await.unwrap();
                if event_matcher(&event.typ) {
                    return event.typ;
                }
            }
        }
        .timeout(Duration::from_secs(10))
        .await
        .expect("timeout waiting for event match")
    }

    /// Consumes events looking for an [`EventType::Info`] with substring matching.
    pub async fn get_info_contains(&self, s: &str) -> EventType {
        self.get_matching(|evt| match evt {
            EventType::Info(ref msg) => msg.contains(s),
            _ => false,
        })
        .await
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
fn print_event(event: &Event) {
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

#[cfg(test)]
mod tests {
    use super::*;

    // The following three tests demonstrate, when made to fail, the log output being
    // directed to the correct test output.

    #[async_std::test]
    async fn test_with_alice() {
        let alice = TestContext::builder().configure_alice().build().await;
        alice.ctx.emit_event(EventType::Info("hello".into()));
        // panic!("Alice fails");
    }

    #[async_std::test]
    async fn test_with_bob() {
        let bob = TestContext::builder().configure_bob().build().await;
        bob.ctx.emit_event(EventType::Info("there".into()));
        // panic!("Bob fails");
    }

    #[async_std::test]
    async fn test_with_both() {
        let (log_sender, _log_sink) = LogSink::create();
        let alice = TestContext::builder()
            .configure_alice()
            .with_log_sink(log_sender.clone())
            .build()
            .await;
        let bob = TestContext::builder()
            .configure_bob()
            .with_log_sink(log_sender)
            .build()
            .await;
        alice.ctx.emit_event(EventType::Info("hello".into()));
        bob.ctx.emit_event(EventType::Info("there".into()));
        // panic!("Both fail");
    }
}
