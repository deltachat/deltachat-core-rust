extern crate deltachat;

use std::sync::{Arc, RwLock};
use std::{thread, time};
use tempfile::tempdir;

use deltachat::chat;
use deltachat::chatlist::*;
use deltachat::config;
use deltachat::contact::*;
use deltachat::context::*;
use deltachat::job::{
    perform_inbox_fetch, perform_inbox_idle, perform_inbox_jobs, perform_smtp_idle,
    perform_smtp_jobs,
};
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
    let ctx =
        Context::new(Box::new(cb), "FakeOs".into(), dbfile).expect("Failed to create context");
    let running = Arc::new(RwLock::new(true));
    let info = ctx.get_info();
    let duration = time::Duration::from_millis(4000);
    println!("info: {:#?}", info);

    let ctx = Arc::new(ctx);
    let ctx1 = ctx.clone();
    let r1 = running.clone();
    let t1 = thread::spawn(move || {
        while *r1.read().unwrap() {
            async_std::task::block_on(perform_inbox_jobs(&ctx1));
            if *r1.read().unwrap() {
                async_std::task::block_on(perform_inbox_fetch(&ctx1));

                if *r1.read().unwrap() {
                    async_std::task::block_on(perform_inbox_idle(&ctx1));
                }
            }
        }
    });

    let ctx1 = ctx.clone();
    let r1 = running.clone();
    let t2 = thread::spawn(move || {
        while *r1.read().unwrap() {
            async_std::task::block_on(perform_smtp_jobs(&ctx1));
            if *r1.read().unwrap() {
                async_std::task::block_on(perform_smtp_idle(&ctx1));
            }
        }
    });

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

    thread::sleep(duration);

    println!("sending a message");
    let contact_id = Contact::create(&ctx, "dignifiedquire", "dignifiedquire@gmail.com").unwrap();
    let chat_id = chat::create_by_contact_id(&ctx, contact_id).unwrap();
    chat::send_text_msg(&ctx, chat_id, "Hi, here is my first message!".into())
        .await
        .unwrap();

    println!("fetching chats..");
    let chats = Chatlist::try_load(&ctx, 0, None, None).unwrap();

    for i in 0..chats.len() {
        let summary = chats.get_summary(&ctx, 0, None);
        let text1 = summary.get_text1();
        let text2 = summary.get_text2();
        println!("chat: {} - {:?} - {:?}", i, text1, text2,);
    }

    thread::sleep(duration);

    println!("stopping threads");

    *running.write().unwrap() = false;
    deltachat::job::interrupt_inbox_idle(&ctx).await;
    deltachat::job::interrupt_smtp_idle(&ctx).await;

    println!("joining");
    t1.join().unwrap();
    t2.join().unwrap();

    println!("closing");
}
