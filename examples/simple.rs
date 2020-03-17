extern crate deltachat;

use async_std::sync::{Arc, RwLock};
use std::time;
use tempfile::tempdir;

use deltachat::chat;
use deltachat::chatlist::*;
use deltachat::config;
use deltachat::contact::*;
use deltachat::context::*;
use deltachat::Event;

fn cb(_ctx: &Context, event: Event) {
    print!("[{:?}]", event);

    match event {
        Event::ConfigureProgress(progress) => {
            println!("  progress: {}", progress);
        }
        Event::Info(msg) | Event::Warning(msg) | Event::Error(msg) | Event::ErrorNetwork(msg) => {
            println!("  {}", msg);
        }
        _ => {
            println!();
        }
    }
}

#[async_std::main]
async fn main() {
    let dir = tempdir().unwrap();
    let dbfile = dir.path().join("db.sqlite");
    println!("creating database {:?}", dbfile);
    let ctx = Context::new("FakeOs".into(), dbfile.into())
        .await
        .expect("Failed to create context");
    let info = ctx.get_info().await;
    let duration = time::Duration::from_millis(4000);
    println!("info: {:#?}", info);

    ctx.run().await;

    println!("configuring");
    let args = std::env::args().collect::<Vec<String>>();
    assert_eq!(args.len(), 2, "missing password");
    let pw = args[1].clone();
    ctx.set_config(config::Config::Addr, Some("d@testrun.org"))
        .await
        .unwrap();
    ctx.set_config(config::Config::MailPw, Some(&pw))
        .await
        .unwrap();
    ctx.configure().await;

    async_std::task::sleep(duration).await;

    println!("sending a message");
    let contact_id = Contact::create(&ctx, "dignifiedquire", "dignifiedquire@gmail.com")
        .await
        .unwrap();
    let chat_id = chat::create_by_contact_id(&ctx, contact_id).await.unwrap();
    chat::send_text_msg(&ctx, chat_id, "Hi, here is my first message!".into())
        .await
        .unwrap();

    println!("fetching chats..");
    let chats = Chatlist::try_load(&ctx, 0, None, None).await.unwrap();

    for i in 0..chats.len() {
        let summary = chats.get_summary(&ctx, 0, None).await;
        let text1 = summary.get_text1();
        let text2 = summary.get_text2();
        println!("chat: {} - {:?} - {:?}", i, text1, text2,);
    }

    async_std::task::sleep(duration).await;

    println!("stopping");
    ctx.stop().await;
    println!("closing");
}
