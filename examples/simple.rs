use tempfile::tempdir;

use deltachat::chat::{self, ChatId};
use deltachat::chatlist::*;
use deltachat::config;
use deltachat::contact::*;
use deltachat::context::*;
use deltachat::message::Message;
use deltachat::EventType;

fn cb(event: EventType) {
    match event {
        EventType::ConfigureProgress { progress, .. } => {
            log::info!("progress: {}", progress);
        }
        EventType::Info(msg) => {
            log::info!("{}", msg);
        }
        EventType::Warning(msg) => {
            log::warn!("{}", msg);
        }
        EventType::Error(msg) => {
            log::error!("{}", msg);
        }
        event => {
            log::info!("{:?}", event);
        }
    }
}

/// Run with `RUST_LOG=simple=info cargo run --release --example simple --features repl -- email pw`.
#[async_std::main]
async fn main() {
    pretty_env_logger::try_init_timed().ok();

    let dir = tempdir().unwrap();
    let dbfile = dir.path().join("db.sqlite");
    log::info!("creating database {:?}", dbfile);
    let ctx = Context::new(dbfile.into(), 0)
        .await
        .expect("Failed to create context");
    let info = ctx.get_info().await;
    log::info!("info: {:#?}", info);

    let events = ctx.get_event_emitter();
    let events_spawn = async_std::task::spawn(async move {
        while let Some(event) = events.recv().await {
            cb(event.typ);
        }
    });

    log::info!("configuring");
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

    log::info!("------ RUN ------");
    ctx.start_io().await;
    log::info!("--- SENDING A MESSAGE ---");

    let contact_id = Contact::create(&ctx, "dignifiedquire", "dignifiedquire@gmail.com")
        .await
        .unwrap();
    let chat_id = ChatId::create_for_contact(&ctx, contact_id).await.unwrap();

    for i in 0..1 {
        log::info!("sending message {}", i);
        chat::send_text_msg(&ctx, chat_id, format!("Hi, here is my {}nth message!", i))
            .await
            .unwrap();
    }

    // wait for the message to be sent out
    async_std::task::sleep(std::time::Duration::from_secs(1)).await;

    log::info!("fetching chats..");
    let chats = Chatlist::try_load(&ctx, 0, None, None).await.unwrap();

    for i in 0..chats.len() {
        let msg = Message::load_from_db(&ctx, chats.get_msg_id(i).unwrap().unwrap())
            .await
            .unwrap();
        log::info!("[{}] msg: {:?}", i, msg);
    }

    log::info!("stopping");
    ctx.stop_io().await;
    log::info!("closing");
    drop(ctx);
    events_spawn.await;
}
