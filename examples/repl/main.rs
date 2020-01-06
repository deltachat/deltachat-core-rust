//! This is a CLI program and a little testing frame.  This file must not be
//! included when using Delta Chat Core as a library.
//!
//! Usage:  cargo run --example repl --release -- <databasefile>
//! All further options can be set using the set-command (type ? for help).

#[macro_use]
extern crate deltachat;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate rusqlite;

use std::borrow::Cow::{self, Borrowed, Owned};
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use deltachat::chat::ChatId;
use deltachat::config;
use deltachat::configure::*;
use deltachat::context::*;
use deltachat::job::*;
use deltachat::oauth2::*;
use deltachat::securejoin::*;
use deltachat::Event;
use rustyline::completion::{Completer, FilenameCompleter, Pair};
use rustyline::config::OutputStreamType;
use rustyline::error::ReadlineError;
use rustyline::highlight::{Highlighter, MatchingBracketHighlighter};
use rustyline::hint::{Hinter, HistoryHinter};
use rustyline::{
    Cmd, CompletionType, Config, Context as RustyContext, EditMode, Editor, Helper, KeyPress,
};

mod cmdline;
use self::cmdline::*;

// Event Handler

fn receive_event(_context: &Context, event: Event) {
    match event {
        Event::Info(msg) => {
            /* do not show the event as this would fill the screen */
            println!("{}", msg);
        }
        Event::SmtpConnected(msg) => {
            println!("[DC_EVENT_SMTP_CONNECTED] {}", msg);
        }
        Event::ImapConnected(msg) => {
            println!("[DC_EVENT_IMAP_CONNECTED] {}", msg);
        }
        Event::SmtpMessageSent(msg) => {
            println!("[DC_EVENT_SMTP_MESSAGE_SENT] {}", msg);
        }
        Event::Warning(msg) => {
            println!("[Warning] {}", msg);
        }
        Event::Error(msg) => {
            println!("\x1b[31m[DC_EVENT_ERROR] {}\x1b[0m", msg);
        }
        Event::ErrorNetwork(msg) => {
            println!("\x1b[31m[DC_EVENT_ERROR_NETWORK] msg={}\x1b[0m", msg);
        }
        Event::ErrorSelfNotInGroup(msg) => {
            println!("\x1b[31m[DC_EVENT_ERROR_SELF_NOT_IN_GROUP] {}\x1b[0m", msg);
        }
        Event::MsgsChanged { chat_id, msg_id } => {
            print!(
                "\x1b[33m{{Received DC_EVENT_MSGS_CHANGED(chat_id={}, msg_id={})}}\n\x1b[0m",
                chat_id, msg_id,
            );
        }
        Event::ContactsChanged(_) => {
            print!("\x1b[33m{{Received DC_EVENT_CONTACTS_CHANGED()}}\n\x1b[0m");
        }
        Event::LocationChanged(contact) => {
            print!(
                "\x1b[33m{{Received DC_EVENT_LOCATION_CHANGED(contact={:?})}}\n\x1b[0m",
                contact,
            );
        }
        Event::ConfigureProgress(progress) => {
            print!(
                "\x1b[33m{{Received DC_EVENT_CONFIGURE_PROGRESS({} ‰)}}\n\x1b[0m",
                progress,
            );
        }
        Event::ImexProgress(progress) => {
            print!(
                "\x1b[33m{{Received DC_EVENT_IMEX_PROGRESS({} ‰)}}\n\x1b[0m",
                progress,
            );
        }
        Event::ImexFileWritten(file) => {
            print!(
                "\x1b[33m{{Received DC_EVENT_IMEX_FILE_WRITTEN({})}}\n\x1b[0m",
                file.display()
            );
        }
        Event::ChatModified(chat) => {
            print!(
                "\x1b[33m{{Received DC_EVENT_CHAT_MODIFIED({})}}\n\x1b[0m",
                chat
            );
        }
        _ => {
            print!("\x1b[33m{{Received {:?}}}\n\x1b[0m", event);
        }
    }
}

// Threads for waiting for messages and for jobs

lazy_static! {
    static ref HANDLE: Arc<Mutex<Option<Handle>>> = Arc::new(Mutex::new(None));
    static ref IS_RUNNING: AtomicBool = AtomicBool::new(true);
}

struct Handle {
    handle_imap: Option<std::thread::JoinHandle<()>>,
    handle_mvbox: Option<std::thread::JoinHandle<()>>,
    handle_sentbox: Option<std::thread::JoinHandle<()>>,
    handle_smtp: Option<std::thread::JoinHandle<()>>,
}

macro_rules! while_running {
    ($code:block) => {
        if IS_RUNNING.load(Ordering::Relaxed) {
            $code
        } else {
            break;
        }
    };
}

fn start_threads(c: Arc<RwLock<Context>>) {
    if HANDLE.clone().lock().unwrap().is_some() {
        return;
    }

    println!("Starting threads");
    IS_RUNNING.store(true, Ordering::Relaxed);

    let ctx = c.clone();
    let handle_imap = std::thread::spawn(move || loop {
        while_running!({
            perform_inbox_jobs(&ctx.read().unwrap());
            perform_inbox_fetch(&ctx.read().unwrap());
            while_running!({
                let context = ctx.read().unwrap();
                perform_inbox_idle(&context);
            });
        });
    });

    let ctx = c.clone();
    let handle_mvbox = std::thread::spawn(move || loop {
        while_running!({
            perform_mvbox_fetch(&ctx.read().unwrap());
            while_running!({
                perform_mvbox_idle(&ctx.read().unwrap());
            });
        });
    });

    let ctx = c.clone();
    let handle_sentbox = std::thread::spawn(move || loop {
        while_running!({
            perform_sentbox_fetch(&ctx.read().unwrap());
            while_running!({
                perform_sentbox_idle(&ctx.read().unwrap());
            });
        });
    });

    let ctx = c;
    let handle_smtp = std::thread::spawn(move || loop {
        while_running!({
            perform_smtp_jobs(&ctx.read().unwrap());
            while_running!({
                perform_smtp_idle(&ctx.read().unwrap());
            });
        });
    });

    *HANDLE.clone().lock().unwrap() = Some(Handle {
        handle_imap: Some(handle_imap),
        handle_mvbox: Some(handle_mvbox),
        handle_sentbox: Some(handle_sentbox),
        handle_smtp: Some(handle_smtp),
    });
}

fn stop_threads(context: &Context) {
    if let Some(ref mut handle) = *HANDLE.clone().lock().unwrap() {
        println!("Stopping threads");
        IS_RUNNING.store(false, Ordering::Relaxed);

        interrupt_inbox_idle(context);
        interrupt_mvbox_idle(context);
        interrupt_sentbox_idle(context);
        interrupt_smtp_idle(context);

        handle.handle_imap.take().unwrap().join().unwrap();
        handle.handle_mvbox.take().unwrap().join().unwrap();
        handle.handle_sentbox.take().unwrap().join().unwrap();
        handle.handle_smtp.take().unwrap().join().unwrap();
    }
}

// === The main loop

struct DcHelper {
    completer: FilenameCompleter,
    highlighter: MatchingBracketHighlighter,
    hinter: HistoryHinter,
}

impl Completer for DcHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &RustyContext<'_>,
    ) -> Result<(usize, Vec<Pair>), ReadlineError> {
        self.completer.complete(line, pos, ctx)
    }
}

const IMEX_COMMANDS: [&str; 12] = [
    "initiate-key-transfer",
    "get-setupcodebegin",
    "continue-key-transfer",
    "has-backup",
    "export-backup",
    "import-backup",
    "export-keys",
    "import-keys",
    "export-setup",
    "poke",
    "reset",
    "stop",
];

const DB_COMMANDS: [&str; 11] = [
    "info",
    "open",
    "close",
    "set",
    "get",
    "oauth2",
    "configure",
    "connect",
    "disconnect",
    "maybenetwork",
    "housekeeping",
];

const CHAT_COMMANDS: [&str; 24] = [
    "listchats",
    "listarchived",
    "chat",
    "createchat",
    "createchatbymsg",
    "creategroup",
    "createverified",
    "addmember",
    "removemember",
    "groupname",
    "groupimage",
    "chatinfo",
    "sendlocations",
    "setlocation",
    "dellocations",
    "getlocations",
    "send",
    "sendimage",
    "sendfile",
    "draft",
    "listmedia",
    "archive",
    "unarchive",
    "delchat",
];
const MESSAGE_COMMANDS: [&str; 8] = [
    "listmsgs",
    "msginfo",
    "listfresh",
    "forward",
    "markseen",
    "star",
    "unstar",
    "delmsg",
];
const CONTACT_COMMANDS: [&str; 6] = [
    "listcontacts",
    "listverified",
    "addcontact",
    "contactinfo",
    "delcontact",
    "cleanupcontacts",
];
const MISC_COMMANDS: [&str; 9] = [
    "getqr", "getbadqr", "checkqr", "event", "fileinfo", "clear", "exit", "quit", "help",
];

impl Hinter for DcHelper {
    fn hint(&self, line: &str, pos: usize, ctx: &RustyContext<'_>) -> Option<String> {
        if !line.is_empty() {
            for &cmds in &[
                &IMEX_COMMANDS[..],
                &DB_COMMANDS[..],
                &CHAT_COMMANDS[..],
                &MESSAGE_COMMANDS[..],
                &CONTACT_COMMANDS[..],
                &MISC_COMMANDS[..],
            ] {
                if let Some(entry) = cmds.iter().find(|el| el.starts_with(&line[..pos])) {
                    if *entry != line && *entry != &line[..pos] {
                        return Some(entry[pos..].to_owned());
                    }
                }
            }
        }
        self.hinter.hint(line, pos, ctx)
    }
}

static COLORED_PROMPT: &str = "\x1b[1;32m> \x1b[0m";
static PROMPT: &str = "> ";

impl Highlighter for DcHelper {
    fn highlight_prompt<'p>(&self, prompt: &'p str) -> Cow<'p, str> {
        if prompt == PROMPT {
            Borrowed(COLORED_PROMPT)
        } else {
            Borrowed(prompt)
        }
    }

    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        Owned("\x1b[1m".to_owned() + hint + "\x1b[m")
    }

    fn highlight<'l>(&self, line: &'l str, pos: usize) -> Cow<'l, str> {
        self.highlighter.highlight(line, pos)
    }

    fn highlight_char(&self, line: &str, pos: usize) -> bool {
        self.highlighter.highlight_char(line, pos)
    }
}

impl Helper for DcHelper {}

fn main_0(args: Vec<String>) -> Result<(), failure::Error> {
    if args.len() < 2 {
        println!("Error: Bad arguments, expected [db-name].");
        return Err(format_err!("No db-name specified"));
    }
    let context = Context::new(
        Box::new(receive_event),
        "CLI".into(),
        Path::new(&args[1]).to_path_buf(),
    )?;

    println!("Delta Chat Core is awaiting your commands.");

    let ctx = Arc::new(RwLock::new(context));

    let config = Config::builder()
        .history_ignore_space(true)
        .completion_type(CompletionType::List)
        .edit_mode(EditMode::Emacs)
        .output_stream(OutputStreamType::Stdout)
        .build();
    let h = DcHelper {
        completer: FilenameCompleter::new(),
        highlighter: MatchingBracketHighlighter::new(),
        hinter: HistoryHinter {},
    };
    let mut rl = Editor::with_config(config);
    rl.set_helper(Some(h));
    rl.bind_sequence(KeyPress::Meta('N'), Cmd::HistorySearchForward);
    rl.bind_sequence(KeyPress::Meta('P'), Cmd::HistorySearchBackward);
    if rl.load_history(".dc-history.txt").is_err() {
        println!("No previous history.");
    }

    loop {
        let p = "> ";
        let readline = rl.readline(&p);
        match readline {
            Ok(line) => {
                // TODO: ignore "set mail_pw"
                rl.add_history_entry(line.as_str());
                let ctx = ctx.clone();
                match handle_cmd(line.trim(), ctx) {
                    Ok(ExitResult::Continue) => {}
                    Ok(ExitResult::Exit) => break,
                    Err(err) => println!("Error: {}", err),
                }
            }
            Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => {
                println!("Exiting...");
                break;
            }
            Err(err) => {
                println!("Error: {}", err);
                break;
            }
        }
    }
    rl.save_history(".dc-history.txt")?;
    println!("history saved");
    {
        stop_threads(&ctx.read().unwrap());
    }

    Ok(())
}

#[derive(Debug)]
enum ExitResult {
    Continue,
    Exit,
}

fn handle_cmd(line: &str, ctx: Arc<RwLock<Context>>) -> Result<ExitResult, failure::Error> {
    let mut args = line.splitn(2, ' ');
    let arg0 = args.next().unwrap_or_default();
    let arg1 = args.next().unwrap_or_default();

    match arg0 {
        "connect" => {
            start_threads(ctx);
        }
        "disconnect" => {
            stop_threads(&ctx.read().unwrap());
        }
        "smtp-jobs" => {
            if HANDLE.clone().lock().unwrap().is_some() {
                println!("smtp-jobs are already running in a thread.",);
            } else {
                perform_smtp_jobs(&ctx.read().unwrap());
            }
        }
        "imap-jobs" => {
            if HANDLE.clone().lock().unwrap().is_some() {
                println!("inbox-jobs are already running in a thread.");
            } else {
                perform_inbox_jobs(&ctx.read().unwrap());
            }
        }
        "configure" => {
            start_threads(ctx.clone());
            configure(&ctx.read().unwrap());
        }
        "oauth2" => {
            if let Some(addr) = ctx.read().unwrap().get_config(config::Config::Addr) {
                let oauth2_url = dc_get_oauth2_url(
                    &ctx.read().unwrap(),
                    &addr,
                    "chat.delta:/com.b44t.messenger",
                );
                if oauth2_url.is_none() {
                    println!("OAuth2 not available for {}.", &addr);
                } else {
                    println!("Open the following url, set mail_pw to the generated token and server_flags to 2:\n{}", oauth2_url.unwrap());
                }
            } else {
                println!("oauth2: set addr first.");
            }
        }
        "clear" => {
            println!("\n\n\n");
            print!("\x1b[1;1H\x1b[2J");
        }
        "getqr" | "getbadqr" => {
            start_threads(ctx.clone());
            if let Some(mut qr) = dc_get_securejoin_qr(
                &ctx.read().unwrap(),
                ChatId::new(arg1.parse().unwrap_or_default()),
            ) {
                if !qr.is_empty() {
                    if arg0 == "getbadqr" && qr.len() > 40 {
                        qr.replace_range(12..22, "0000000000")
                    }
                    println!("{}", qr);
                    let output = Command::new("qrencode")
                        .args(&["-t", "ansiutf8", qr.as_str(), "-o", "-"])
                        .output()
                        .expect("failed to execute process");
                    io::stdout().write_all(&output.stdout).unwrap();
                    io::stderr().write_all(&output.stderr).unwrap();
                }
            }
        }
        "joinqr" => {
            start_threads(ctx.clone());
            if !arg0.is_empty() {
                dc_join_securejoin(&ctx.read().unwrap(), arg1);
            }
        }
        "exit" | "quit" => return Ok(ExitResult::Exit),
        _ => dc_cmdline(&ctx.read().unwrap(), line)?,
    }

    Ok(ExitResult::Continue)
}

pub fn main() -> Result<(), failure::Error> {
    let _ = pretty_env_logger::try_init();

    let args: Vec<String> = std::env::args().collect();
    main_0(args)?;

    Ok(())
}
