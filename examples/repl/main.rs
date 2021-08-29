//! This is a CLI program and a little testing frame.  This file must not be
//! included when using Delta Chat Core as a library.
//!
//! Usage:  cargo run --example repl --release -- <databasefile>
//! All further options can be set using the set-command (type ? for help).

#[macro_use]
extern crate deltachat;

use std::borrow::Cow::{self, Borrowed, Owned};
use std::io::{self, Write};
use std::process::Command;

use ansi_term::Color;
use anyhow::{bail, Error};
use async_std::path::Path;
use deltachat::chat::ChatId;
use deltachat::config;
use deltachat::context::*;
use deltachat::oauth2::*;
use deltachat::securejoin::*;
use deltachat::EventType;
use log::{error, info, warn};
use rustyline::completion::{Completer, FilenameCompleter, Pair};
use rustyline::config::OutputStreamType;
use rustyline::error::ReadlineError;
use rustyline::highlight::{Highlighter, MatchingBracketHighlighter};
use rustyline::hint::{Hinter, HistoryHinter};
use rustyline::validate::Validator;
use rustyline::{
    Cmd, CompletionType, Config, Context as RustyContext, EditMode, Editor, Helper, KeyEvent,
};

mod cmdline;
use self::cmdline::*;

/// Event Handler
fn receive_event(event: EventType) {
    let yellow = Color::Yellow.normal();
    match event {
        EventType::Info(msg) => {
            /* do not show the event as this would fill the screen */
            info!("{}", msg);
        }
        EventType::SmtpConnected(msg) => {
            info!("[SMTP_CONNECTED] {}", msg);
        }
        EventType::ImapConnected(msg) => {
            info!("[IMAP_CONNECTED] {}", msg);
        }
        EventType::SmtpMessageSent(msg) => {
            info!("[SMTP_MESSAGE_SENT] {}", msg);
        }
        EventType::Warning(msg) => {
            warn!("{}", msg);
        }
        EventType::Error(msg) => {
            error!("{}", msg);
        }
        EventType::ErrorSelfNotInGroup(msg) => {
            error!("[SELF_NOT_IN_GROUP] {}", msg);
        }
        EventType::MsgsChanged { chat_id, msg_id } => {
            info!(
                "{}",
                yellow.paint(format!(
                    "Received MSGS_CHANGED(chat_id={}, msg_id={})",
                    chat_id, msg_id,
                ))
            );
        }
        EventType::ContactsChanged(_) => {
            info!("{}", yellow.paint("Received CONTACTS_CHANGED()"));
        }
        EventType::LocationChanged(contact) => {
            info!(
                "{}",
                yellow.paint(format!("Received LOCATION_CHANGED(contact={:?})", contact))
            );
        }
        EventType::ConfigureProgress { progress, comment } => {
            if let Some(comment) = comment {
                info!(
                    "{}",
                    yellow.paint(format!(
                        "Received CONFIGURE_PROGRESS({} ‰, {})",
                        progress, comment
                    ))
                );
            } else {
                info!(
                    "{}",
                    yellow.paint(format!("Received CONFIGURE_PROGRESS({} ‰)", progress))
                );
            }
        }
        EventType::ImexProgress(progress) => {
            info!(
                "{}",
                yellow.paint(format!("Received IMEX_PROGRESS({} ‰)", progress))
            );
        }
        EventType::ImexFileWritten(file) => {
            info!(
                "{}",
                yellow.paint(format!("Received IMEX_FILE_WRITTEN({})", file.display()))
            );
        }
        EventType::ChatModified(chat) => {
            info!(
                "{}",
                yellow.paint(format!("Received CHAT_MODIFIED({})", chat))
            );
        }
        _ => {
            info!("Received {:?}", event);
        }
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

const DB_COMMANDS: [&str; 10] = [
    "info",
    "set",
    "get",
    "oauth2",
    "configure",
    "connect",
    "disconnect",
    "connectivity",
    "maybenetwork",
    "housekeeping",
];

const CHAT_COMMANDS: [&str; 33] = [
    "listchats",
    "listarchived",
    "chat",
    "createchat",
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
    "sendhtml",
    "videochat",
    "draft",
    "listmedia",
    "archive",
    "unarchive",
    "pin",
    "unpin",
    "mute",
    "unmute",
    "protect",
    "unprotect",
    "delchat",
    "accept",
    "blockchat",
];
const MESSAGE_COMMANDS: [&str; 7] = [
    "listmsgs",
    "msginfo",
    "listfresh",
    "forward",
    "markseen",
    "delmsg",
    "download",
];
const CONTACT_COMMANDS: [&str; 9] = [
    "listcontacts",
    "listverified",
    "addcontact",
    "contactinfo",
    "delcontact",
    "cleanupcontacts",
    "block",
    "unblock",
    "listblocked",
];
const MISC_COMMANDS: [&str; 10] = [
    "getqr",
    "getbadqr",
    "checkqr",
    "event",
    "fileinfo",
    "clear",
    "exit",
    "quit",
    "help",
    "estimatedeletion",
];

impl Hinter for DcHelper {
    type Hint = String;

    fn hint(&self, line: &str, pos: usize, ctx: &RustyContext<'_>) -> Option<Self::Hint> {
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

impl Highlighter for DcHelper {
    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(&self, prompt: &'p str, default: bool) -> Cow<'b, str> {
        if default {
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
impl Validator for DcHelper {}

async fn start(args: Vec<String>) -> Result<(), Error> {
    if args.len() < 2 {
        println!("Error: Bad arguments, expected [db-name].");
        bail!("No db-name specified");
    }
    let context = Context::new("CLI".into(), Path::new(&args[1]).to_path_buf(), 0).await?;

    let events = context.get_event_emitter();
    async_std::task::spawn(async move {
        while let Some(event) = events.recv().await {
            receive_event(event.typ);
        }
    });

    println!("Delta Chat Core is awaiting your commands.");

    let config = Config::builder()
        .history_ignore_space(true)
        .completion_type(CompletionType::List)
        .edit_mode(EditMode::Emacs)
        .output_stream(OutputStreamType::Stdout)
        .build();
    let mut selected_chat = ChatId::default();
    let (reader_s, reader_r) = async_std::channel::bounded(100);
    let input_loop = async_std::task::spawn_blocking(move || {
        let h = DcHelper {
            completer: FilenameCompleter::new(),
            highlighter: MatchingBracketHighlighter::new(),
            hinter: HistoryHinter {},
        };
        let mut rl = Editor::with_config(config);
        rl.set_helper(Some(h));
        rl.bind_sequence(KeyEvent::alt('N'), Cmd::HistorySearchForward);
        rl.bind_sequence(KeyEvent::alt('P'), Cmd::HistorySearchBackward);
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
                    async_std::task::block_on(reader_s.send(line)).unwrap();
                }
                Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => {
                    println!("Exiting...");
                    drop(reader_s);
                    break;
                }
                Err(err) => {
                    println!("Error: {}", err);
                    drop(reader_s);
                    break;
                }
            }
        }

        rl.save_history(".dc-history.txt")?;
        println!("history saved");
        Ok::<_, Error>(())
    });

    while let Ok(line) = reader_r.recv().await {
        match handle_cmd(line.trim(), context.clone(), &mut selected_chat).await {
            Ok(ExitResult::Continue) => {}
            Ok(ExitResult::Exit) => break,
            Err(err) => println!("Error: {}", err),
        }
    }
    context.stop_io().await;
    input_loop.await?;

    Ok(())
}

#[derive(Debug)]
enum ExitResult {
    Continue,
    Exit,
}

async fn handle_cmd(
    line: &str,
    ctx: Context,
    selected_chat: &mut ChatId,
) -> Result<ExitResult, Error> {
    let mut args = line.splitn(2, ' ');
    let arg0 = args.next().unwrap_or_default();
    let arg1 = args.next().unwrap_or_default();

    match arg0 {
        "connect" => {
            ctx.start_io().await;
        }
        "disconnect" => {
            ctx.stop_io().await;
        }
        "configure" => {
            ctx.configure().await?;
        }
        "oauth2" => {
            if let Some(addr) = ctx.get_config(config::Config::Addr).await? {
                let oauth2_url =
                    dc_get_oauth2_url(&ctx, &addr, "chat.delta:/com.b44t.messenger").await?;
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
            ctx.start_io().await;
            let group = arg1.parse::<u32>().ok().map(|id| ChatId::new(id));
            if let Some(mut qr) = dc_get_securejoin_qr(&ctx, group).await {
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
            ctx.start_io().await;
            if !arg0.is_empty() {
                dc_join_securejoin(&ctx, arg1).await?;
            }
        }
        "exit" | "quit" => return Ok(ExitResult::Exit),
        _ => cmdline(ctx.clone(), line, selected_chat).await?,
    }

    Ok(ExitResult::Continue)
}

fn main() -> Result<(), Error> {
    let _ = pretty_env_logger::try_init();

    let args = std::env::args().collect();
    async_std::task::block_on(async move { start(args).await })?;

    Ok(())
}
