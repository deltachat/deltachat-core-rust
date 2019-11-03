//! Integration tests for location streaming.

use std::collections::{HashMap, VecDeque};
use std::mem::discriminant;
use std::path::Path;
use std::sync::{atomic, Arc, Condvar, Mutex};
use std::thread;

use itertools::Itertools;
use libc::uintptr_t;
use serde::Deserialize;
use tempfile;

use deltachat::chat;
use deltachat::config::Config;
use deltachat::contact::Contact;
use deltachat::context::Context;
use deltachat::job;
use deltachat::location;
use deltachat::Event;

/// Credentials for a test account.
///
/// This is populated by the JSON returned from the account provider's
/// API.
#[derive(Debug, Deserialize)]
struct AccountCredentials {
    email: String,
    password: String,
}

impl AccountCredentials {
    /// Creates a new online account.
    ///
    /// Invoke the API of the account provider to create a new
    /// temporary account.
    fn new(provider_url: &str) -> AccountCredentials {
        let (post_url, token) = provider_url.splitn(2, '#').next_tuple().unwrap();
        let mut data: HashMap<&str, u64> = HashMap::new();
        data.insert("token_create_user", token.parse().unwrap());
        let client = reqwest::Client::new();
        let mut response = client.post(post_url).json(&data).send().unwrap();
        assert!(
            response.status().is_success(),
            format!("Failed to create new tmpuser: {}", response.status())
        );
        response.json().unwrap()
    }
}

#[derive(Debug)]
struct EventsItem {
    acc_name: String,
    when: std::time::Duration,
    event: Event,
}

#[derive(Debug)]
struct EventsQueue {
    name: String,
    events: Mutex<VecDeque<EventsItem>>,
    cond: Condvar,
}

impl EventsQueue {
    fn new(name: &str) -> EventsQueue {
        EventsQueue {
            name: name.to_string(),
            events: Mutex::new(VecDeque::new()),
            cond: Condvar::new(),
        }
    }

    fn push(&self, evt: EventsItem) {
        let mut queue = self.events.lock().unwrap();
        queue.push_back(evt);
        self.cond.notify_all();
    }

    fn wait_for(&self, event: Event, data: bool) -> Result<(), ()> {
        println!(
            "==> [{}] Waiting for: {:?} match-data={}",
            self.name, event, data
        );
        let mut queue = self.events.lock().unwrap();
        let start_time = std::time::Instant::now();
        loop {
            while let Some(item) = queue.pop_front() {
                let hit = match data {
                    true => event == item.event,
                    false => discriminant(&event) == discriminant(&item.event),
                };
                self.log_event(&item);
                if hit {
                    println!(
                        "<== [{}] Found {:?} match-data={} in {:?}",
                        self.name,
                        event,
                        data,
                        start_time.elapsed()
                    );
                    return Ok(());
                }
            }
            if start_time.elapsed().as_secs() > 25 {
                println!(
                    "=!= [{}] Timed out waiting for {:?} match-data={}",
                    self.name, event, data
                );
                return Err(());
            }
            queue = self.cond.wait(queue).unwrap();
        }
    }

    fn clear(&self) {
        let mut queue = self.events.lock().unwrap();
        while let Some(item) = queue.pop_front() {
            self.log_event(&item);
        }
    }

    fn log_event(&self, item: &EventsItem) {
        match &item.event {
            Event::Info(msg) => println!("I [{} {:?}]: {}", item.acc_name, item.when, msg),
            Event::Warning(msg) => println!("W [{} {:?}]: {}", item.acc_name, item.when, msg),
            Event::Error(msg) => println!("E [{} {:?}]: {}", item.acc_name, item.when, msg),
            _ => println!("Evt [{} {:?}]: {:?}", item.acc_name, item.when, item.event),
        }
    }

    fn clear_log_events(&self) {
        let mut queue = self.events.lock().unwrap();
        for item in queue.iter() {
            self.log_event(item)
        }
        queue.retain(|item| match item.event {
            Event::Info(_) | Event::Warning(_) | Event::Error(_) => false,
            _ => true,
        });
    }
}

/// A Configured DeltaChat account.
#[derive(Debug)]
struct Account {
    name: String,
    creds: AccountCredentials,
    ctx: Arc<Context>,
    events: Arc<EventsQueue>,
    running: Arc<atomic::AtomicBool>,
    imap_handle: Option<thread::JoinHandle<()>>,
    mvbox_handle: Option<thread::JoinHandle<()>>,
    sentbox_handle: Option<thread::JoinHandle<()>>,
    smtp_handle: Option<thread::JoinHandle<()>>,
}

impl Account {
    fn new(name: &str, dir: &Path, keys: KeyPair, start: std::time::Instant) -> Account {
        // Create events queue and callback.
        let events = Arc::new(EventsQueue::new(name));
        let events_cb = Arc::clone(&events);
        let name_cb = name.to_string();
        let cb = move |_ctx: &Context, evt: Event| -> uintptr_t {
            events_cb.push(EventsItem {
                acc_name: name_cb.clone(),
                when: start.elapsed(),
                event: evt,
            });
            0
        };

        // Create and configure the context.
        let dbfile = dir.join(format!("{}.db", name));
        let creds = AccountCredentials::new(&Account::liveconfig_url());
        println!("Account credentials for {}: {:#?}", name, creds);
        let ctx = Arc::new(Context::new(Box::new(cb), "TestClient".into(), dbfile).unwrap());
        ctx.set_config(Config::Addr, Some(&creds.email)).unwrap();
        ctx.set_config(Config::MailPw, Some(&creds.password))
            .unwrap();
        keys.save_as_self(&ctx);
        deltachat::configure::configure(&ctx);

        // Start the threads.
        let running = Arc::new(atomic::AtomicBool::new(true));
        let imap_handle = Self::start_imap(name, Arc::clone(&ctx), Arc::clone(&running));
        let mvbox_handle = Self::start_mvbox(name, Arc::clone(&ctx), Arc::clone(&running));
        let sentbox_handle = Self::start_sentbox(name, Arc::clone(&ctx), Arc::clone(&running));
        let smtp_handle = Self::start_smtp(name, Arc::clone(&ctx), Arc::clone(&running));
        events.clear_log_events();

        Account {
            name: name.to_string(),
            creds,
            ctx,
            events,
            running,
            imap_handle: Some(imap_handle),
            mvbox_handle: Some(mvbox_handle),
            sentbox_handle: Some(sentbox_handle),
            smtp_handle: Some(smtp_handle),
        }
    }

    /// Find the liveconfig URL.
    ///
    /// Prefers the `DCC_TMPACCOUNT_PROVIDER`, will also use the
    /// `DCC_PY_LIVECONFIG` environment variable and finally fall back
    /// to finding a file named `liveconfig` and starting with
    /// `#:provider:https://`.
    fn liveconfig_url() -> String {
        if let Some(url) = std::env::var("DCC_TMPACCOUNT_PROVIDER").ok() {
            return url;
        }
        if let Some(url) = std::env::var("DCC_PY_LIVECONFIG").ok() {
            return url;
        }
        let mut dir = Some(Path::new(".").canonicalize().unwrap());
        loop {
            let cfg_fname = match dir {
                Some(path) => {
                    dir = path.parent().map(|p| p.to_path_buf());
                    path.join("liveconfig")
                }
                None => break,
            };
            if cfg_fname.is_file() {
                let raw_data = std::fs::read(&cfg_fname).unwrap();
                let data = String::from_utf8(raw_data).unwrap();
                for line in data.lines() {
                    if line.starts_with("#:provider:https://") {
                        let (_, url) = line.split_at(11);
                        return url.to_string();
                    }
                }
                panic!("No provider URL in {}", cfg_fname.display());
            }
        }
        panic!("Found no liveconfig");
    }

    fn start_imap(
        name: &str,
        ctx: Arc<Context>,
        running: Arc<atomic::AtomicBool>,
    ) -> thread::JoinHandle<()> {
        thread::Builder::new()
            .name(format!("{}-imap", name))
            .spawn(move || {
                while running.load(atomic::Ordering::Relaxed) {
                    job::perform_imap_jobs(&ctx);
                    job::perform_imap_fetch(&ctx);
                    if !running.load(atomic::Ordering::Relaxed) {
                        break;
                    }
                    job::perform_imap_idle(&ctx);
                }
            })
            .unwrap()
    }

    fn start_mvbox(
        name: &str,
        ctx: Arc<Context>,
        running: Arc<atomic::AtomicBool>,
    ) -> thread::JoinHandle<()> {
        thread::Builder::new()
            .name(format!("{}-mvbox", name))
            .spawn(move || {
                while running.load(atomic::Ordering::Relaxed) {
                    job::perform_mvbox_jobs(&ctx);
                    job::perform_mvbox_fetch(&ctx);
                    if !running.load(atomic::Ordering::Relaxed) {
                        break;
                    }
                    job::perform_mvbox_idle(&ctx);
                }
            })
            .unwrap()
    }

    fn start_sentbox(
        name: &str,
        ctx: Arc<Context>,
        running: Arc<atomic::AtomicBool>,
    ) -> thread::JoinHandle<()> {
        thread::Builder::new()
            .name(format!("{}-sentbox", name))
            .spawn(move || {
                while running.load(atomic::Ordering::Relaxed) {
                    job::perform_sentbox_jobs(&ctx);
                    job::perform_sentbox_fetch(&ctx);
                    if !running.load(atomic::Ordering::Relaxed) {
                        break;
                    }
                    job::perform_sentbox_idle(&ctx);
                }
            })
            .unwrap()
    }

    fn start_smtp(
        name: &str,
        ctx: Arc<Context>,
        running: Arc<atomic::AtomicBool>,
    ) -> thread::JoinHandle<()> {
        thread::Builder::new()
            .name(format!("{}-smtp", name))
            .spawn(move || {
                while running.load(atomic::Ordering::Relaxed) {
                    job::perform_smtp_jobs(&ctx);
                    job::perform_smtp_fetch(&ctx);
                    if !running.load(atomic::Ordering::Relaxed) {
                        break;
                    }
                    job::perform_smtp_idle(&ctx);
                }
            })
            .unwrap()
    }

    /// Goes through the events queue and prints all log events.
    ///
    /// Each processed event is removed from the queue.
    fn process_log_events(&self) {}
}

impl Drop for Account {
    fn drop(&mut self) {
        println!("Terminating Account {}", self.name);
        self.running.store(false, atomic::Ordering::Relaxed);
        job::interrupt_imap_idle(&self.ctx);
        job::interrupt_mvbox_idle(&self.ctx);
        self.imap_handle.take().unwrap().join().unwrap();
        self.mvbox_handle.take().unwrap().join().unwrap();
        self.events.clear();
        println!("Account {} Terminated", self.name);
    }
}

/// Helper struct to handle account key pairs.
struct KeyPair {
    public: deltachat::key::Key,
    private: deltachat::key::Key,
}

impl KeyPair {
    /// Create a new [KeyPair].
    ///
    /// # Example
    ///
    /// ```
    /// let alice_keys = KeyPair::new(
    ///     include_str!("../test-data/key/public.asc"),
    ///     include_str!("../test-data/key/private.asc"),
    /// );
    /// ```
    fn new(public_data: &str, private_data: &str) -> KeyPair {
        let public =
            deltachat::key::Key::from_base64(public_data, deltachat::constants::KeyType::Public)
                .unwrap();
        let private =
            deltachat::key::Key::from_base64(private_data, deltachat::constants::KeyType::Private)
                .unwrap();
        KeyPair { public, private }
    }

    /// Saves a key into the context as the default key of the self address.
    ///
    /// [Config::Addr] must already be set.
    fn save_as_self(&self, ctx: &Context) {
        let addr = ctx.get_config(Config::Addr).unwrap();
        let ok = deltachat::key::dc_key_save_self_keypair(
            &ctx,
            &self.public,
            &self.private,
            &addr,
            true,
            &ctx.sql,
        );
        assert_eq!(ok, true);
    }
}

#[test]
fn test_location_streaming() {
    // Create accounts
    let start = std::time::Instant::now();
    let tmpdir = tempfile::tempdir().unwrap();
    let alice_keys = KeyPair::new(
        include_str!("../test-data/key/public.asc"),
        include_str!("../test-data/key/private.asc"),
    );
    let alice = Account::new("alice", tmpdir.path(), alice_keys, start);
    let bob_keys = KeyPair::new(
        include_str!("../test-data/key/public2.asc"),
        include_str!("../test-data/key/private2.asc"),
    );
    let bob = Account::new("bob", tmpdir.path(), bob_keys, start);
    alice
        .events
        .wait_for(Event::ConfigureProgress(1000), true)
        .unwrap();
    bob.events
        .wait_for(Event::ConfigureProgress(1000), true)
        .unwrap();

    // Create contacts and chats.
    let contact_bob = Contact::create(&alice.ctx, "Bob", &bob.creds.email).unwrap();
    let contact_alice = Contact::create(&bob.ctx, "Alice", &bob.creds.email).unwrap();
    let alice_to_bob = deltachat::chat::create_by_contact_id(&alice.ctx, contact_bob).unwrap();
    let bob_to_alice = deltachat::chat::create_by_contact_id(&bob.ctx, contact_alice).unwrap();
    alice.events.clear();
    bob.events.clear();

    println!("### Starting location streaming from Alice to Bob");
    assert!(!location::is_sending_locations_to_chat(
        &alice.ctx,
        alice_to_bob
    ));
    assert!(!location::is_sending_locations_to_chat(
        &bob.ctx,
        bob_to_alice
    ));
    location::send_locations_to_chat(&alice.ctx, alice_to_bob, 100);
    assert!(location::is_sending_locations_to_chat(
        &alice.ctx,
        alice_to_bob
    ));
    alice
        .events
        .wait_for(Event::SmtpMessageSent(Default::default()), false)
        .unwrap();
    assert_eq!(location::set(&alice.ctx, 1.0, 1.0, 1.0), true);
    alice
        .events
        .wait_for(Event::LocationChanged(Default::default()), false)
        .unwrap();
    assert_eq!(location::set(&alice.ctx, 1.1, 1.1, 1.0), true);
    chat::send_text_msg(&alice.ctx, alice_to_bob, "ping".to_string()).unwrap();
    alice
        .events
        .wait_for(Event::SmtpMessageSent(Default::default()), false)
        .unwrap();

    println!("### Looking for location messages received by Bob");
    // First message is the "enabled-location-streaming" command.
    bob.events
        .wait_for(
            Event::MsgsChanged {
                chat_id: Default::default(),
                msg_id: Default::default(),
            },
            false,
        )
        .unwrap();
    // Core emits location changed before the incoming message.  Sadly
    // the the ordering requirement is brittle.
    bob.events
        .wait_for(Event::LocationChanged(Default::default()), false)
        .unwrap();
    // Next message is the "ping" one which should contain a location.
    bob.events
        .wait_for(
            Event::MsgsChanged {
                chat_id: Default::default(),
                msg_id: Default::default(),
            },
            false,
        )
        .unwrap();
    let positions = location::get_range(&bob.ctx, bob_to_alice, contact_alice, 0, 0);
    println!("pos len: {}", positions.len());
    println!("{:#?}", positions);
    assert!(false, "THE END");
}
