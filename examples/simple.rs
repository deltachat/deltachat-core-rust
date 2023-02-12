use tempfile::tempdir;

use deltachat::chat::{self, ChatId};
use deltachat::chatlist::*;
use deltachat::config;
use deltachat::contact::*;
use deltachat::context::*;
use deltachat::message::Message;
use deltachat::stock_str::StockStrings;
use deltachat::{EventType, Events};
use tracing::{error, info, warn};
use tracing_subscriber::{fmt, EnvFilter};

fn cb(event: EventType) {
    match event {
        EventType::ConfigureProgress { progress, .. } => {
            info!("progress: {progress}");
        }
        EventType::Info(msg) => {
            info!("{msg}");
        }
        EventType::Warning(msg) => {
            warn!("{msg}");
        }
        EventType::Error(msg) => {
            error!("{msg}");
        }
        event => {
            info!("{event:?}");
        }
    }
}

/// Run with `RUST_LOG=simple=info cargo run --release --example simple -- email pw`.
#[tokio::main]
async fn main() {
    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();
    fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();

    let dir = tempdir().unwrap();
    let dbfile = dir.path().join("db.sqlite");
    info!("creating database {:?}", dbfile);
    let ctx = Context::new(&dbfile, 0, Events::new(), StockStrings::new())
        .await
        .expect("Failed to create context");
    let info = ctx.get_info().await;
    info!("info: {:#?}", info);

    let events = ctx.get_event_emitter();
    let events_spawn = tokio::task::spawn(async move {
        while let Some(event) = events.recv().await {
            cb(event.typ);
        }
    });

    info!("configuring");
    let args = std::env::args().collect::<Vec<String>>();
    assert_eq!(args.len(), 3, "requires email password");
    let email = args[1].clone();
    let pw = args[2].clone();
    ctx.set_config(config::Config::Addr, Some(&email))
        .await
        .unwrap();
    ctx.set_config(config::Config::MailPw, Some(&pw))
        .await
        .unwrap();

    ctx.configure().await.unwrap();

    info!("------ RUN ------");
    ctx.start_io().await;
    info!("--- SENDING A MESSAGE ---");

    let contact_id = Contact::create(&ctx, "dignifiedquire", "dignifiedquire@gmail.com")
        .await
        .unwrap();
    let chat_id = ChatId::create_for_contact(&ctx, contact_id).await.unwrap();

    for i in 0..1 {
        info!("sending message {}", i);
        chat::send_text_msg(&ctx, chat_id, format!("Hi, here is my {i}nth message!"))
            .await
            .unwrap();
    }

    // wait for the message to be sent out
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    info!("fetching chats..");
    let chats = Chatlist::try_load(&ctx, 0, None, None).await.unwrap();

    for i in 0..chats.len() {
        let msg = Message::load_from_db(&ctx, chats.get_msg_id(i).unwrap().unwrap())
            .await
            .unwrap();
        info!("[{i}] msg: {msg:?}");
    }

    info!("stopping");
    ctx.stop_io().await;
    info!("closing");
    drop(ctx);
    events_spawn.await.unwrap();
}
