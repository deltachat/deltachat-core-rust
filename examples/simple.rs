extern crate deltachat;

use std::ffi::CStr;
use std::sync::{Arc, RwLock};
use std::{thread, time};
use tempfile::tempdir;

use deltachat::chatlist::*;
use deltachat::config;
use deltachat::constants::Event;
use deltachat::contact::*;
use deltachat::context::*;
use deltachat::dc_chat::*;
use deltachat::dc_configure::*;
use deltachat::dc_job::{
    dc_perform_imap_fetch, dc_perform_imap_idle, dc_perform_imap_jobs, dc_perform_smtp_idle,
    dc_perform_smtp_jobs,
};
use deltachat::dc_lot::*;

extern "C" fn cb(_ctx: &Context, event: Event, data1: usize, data2: usize) -> usize {
    println!("[{:?}]", event);

    match event {
        Event::CONFIGURE_PROGRESS => {
            println!("  progress: {}", data1);
            0
        }
        Event::INFO | Event::WARNING | Event::ERROR | Event::ERROR_NETWORK => {
            println!(
                "  {}",
                unsafe { CStr::from_ptr(data2 as *const _) }
                    .to_str()
                    .unwrap()
            );
            0
        }
        _ => 0,
    }
}

fn main() {
    unsafe {
        let ctx = dc_context_new(Some(cb), std::ptr::null_mut(), None);
        let running = Arc::new(RwLock::new(true));
        let info = dc_get_info(&ctx);
        let info_s = CStr::from_ptr(info);
        let duration = time::Duration::from_millis(4000);
        println!("info: {}", info_s.to_str().unwrap());

        let ctx = Arc::new(ctx);
        let ctx1 = ctx.clone();
        let r1 = running.clone();
        let t1 = thread::spawn(move || {
            while *r1.read().unwrap() {
                dc_perform_imap_jobs(&ctx1);
                if *r1.read().unwrap() {
                    dc_perform_imap_fetch(&ctx1);

                    if *r1.read().unwrap() {
                        dc_perform_imap_idle(&ctx1);
                    }
                }
            }
        });

        let ctx1 = ctx.clone();
        let r1 = running.clone();
        let t2 = thread::spawn(move || {
            while *r1.read().unwrap() {
                dc_perform_smtp_jobs(&ctx1);
                if *r1.read().unwrap() {
                    dc_perform_smtp_idle(&ctx1);
                }
            }
        });

        let dir = tempdir().unwrap();
        let dbfile = dir.path().join("db.sqlite");

        println!("opening database {:?}", dbfile);

        assert!(dc_open(&ctx, dbfile.to_str().unwrap(), None));

        println!("configuring");
        let args = std::env::args().collect::<Vec<String>>();
        assert_eq!(args.len(), 2, "missing password");
        let pw = args[1].clone();
        ctx.set_config(config::Config::Addr, Some("d@testrun.org"))
            .unwrap();
        ctx.set_config(config::Config::MailPw, Some(&pw)).unwrap();
        dc_configure(&ctx);

        thread::sleep(duration);

        println!("sending a message");
        let contact_id =
            Contact::create(&ctx, "dignifiedquire", "dignifiedquire@gmail.com").unwrap();
        let chat_id = dc_create_chat_by_contact_id(&ctx, contact_id);
        dc_send_text_msg(&ctx, chat_id, "Hi, here is my first message!".into());

        println!("fetching chats..");
        let chats = Chatlist::try_load(&ctx, 0, None, None).unwrap();

        for i in 0..chats.len() {
            let summary = chats.get_summary(0, std::ptr::null_mut());
            let text1 = dc_lot_get_text1(summary);
            let text2 = dc_lot_get_text2(summary);

            let text1_s = if !text1.is_null() {
                Some(CStr::from_ptr(text1))
            } else {
                None
            };
            let text2_s = if !text2.is_null() {
                Some(CStr::from_ptr(text2))
            } else {
                None
            };
            println!("chat: {} - {:?} - {:?}", i, text1_s, text2_s,);
            dc_lot_unref(summary);
        }

        thread::sleep(duration);

        // let msglist = dc_get_chat_msgs(&ctx, chat_id, 0, 0);
        // for i in 0..dc_array_get_cnt(msglist) {
        //     let msg_id = dc_array_get_id(msglist, i);
        //     let msg = dc_get_msg(context, msg_id);
        //     let text = CStr::from_ptr(dc_msg_get_text(msg)).unwrap();
        //     println!("Message {}: {}\n", i + 1, text.to_str().unwrap());
        //     dc_msg_unref(msg);
        // }
        // dc_array_unref(msglist);

        println!("stopping threads");

        *running.clone().write().unwrap() = false;
        deltachat::dc_job::dc_interrupt_imap_idle(&ctx);
        deltachat::dc_job::dc_interrupt_smtp_idle(&ctx);

        println!("joining");
        t1.join().unwrap();
        t2.join().unwrap();

        println!("closing");
        dc_close(&ctx);
    }
}
