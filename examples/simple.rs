extern crate deltachat;

use std::{thread, time};
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
            print!("  progress: {}\n", progress);
        }
        Event::Info(msg) | Event::Warning(msg) | Event::Error(msg) | Event::ErrorNetwork(msg) => {
            print!("  {}\n", msg);
        }
        _ => {
            print!("\n");
        }
    }
}

fn main() {
    let dir = tempdir().unwrap();
    let dbfile = dir.path().join("db.sqlite");
    println!("creating database {:?}", dbfile);
    let ctx =
        Context::new(Box::new(cb), "FakeOs".into(), dbfile).expect("Failed to create context");
    let info = ctx.get_info();
    let duration = time::Duration::from_millis(8000);
    println!("info: {:#?}", info);

    crossbeam::scope(|s| {
        let t1 = s.spawn(|_| {
            ctx.run();
        });

        println!("configuring");
        let args = std::env::args().collect::<Vec<String>>();
        assert_eq!(args.len(), 2, "missing password");
        let pw = args[1].clone();
        ctx.set_config(config::Config::Addr, Some("d@testrun.org"))
            .unwrap();
        ctx.set_config(config::Config::MailPw, Some(&pw)).unwrap();
        ctx.configure();

        thread::sleep(duration);

        println!("sending a message");
        let contact_id =
            Contact::create(&ctx, "dignifiedquire", "dignifiedquire@gmail.com").unwrap();
        let chat_id = chat::create_by_contact_id(&ctx, contact_id).unwrap();
        chat::send_text_msg(&ctx, chat_id, "Hi, here is my first message!".into()).unwrap();

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
        ctx.shutdown();

        println!("joining");
        t1.join().unwrap();

        println!("closing");
    })
    .unwrap();
}
