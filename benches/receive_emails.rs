use std::convert::TryInto;

use async_std::{path::PathBuf, task::block_on};
use criterion::{
    async_executor::AsyncStdExecutor, black_box, criterion_group, criterion_main, BatchSize,
    BenchmarkId, Criterion,
};
use deltachat::{
    config::Config,
    context::Context,
    dc_receive_imf::dc_receive_imf,
    imex::{imex, ImexMode},
};
use tempfile::tempdir;

async fn recv_emails(context: Context, emails: &[&[u8]]) -> Context {
    for (i, bytes) in emails.iter().enumerate() {
        dc_receive_imf(
            &context,
            bytes,
            "INBOX",
            black_box(i.try_into().unwrap()),
            false,
        )
        .await
        .unwrap();
    }
    context
}

async fn recv_all_emails(mut context: Context, needs_move_enabled: bool) -> Context {
    context.disable_needs_move = !needs_move_enabled;
    let emails = [
        include_bytes!("../test-data/message/allinkl-quote.eml").as_ref(),
        include_bytes!("../test-data/message/apple_cid_jpg.eml").as_ref(),
        include_bytes!("../test-data/message/attach_filename_apostrophed_cont.eml").as_ref(),
        include_bytes!("../test-data/message/attach_filename_apostrophed_cp1252.eml").as_ref(),
        include_bytes!("../test-data/message/attach_filename_apostrophed.eml").as_ref(),
        include_bytes!("../test-data/message/attach_filename_apostrophed_invalid.eml").as_ref(),
        include_bytes!("../test-data/message/attach_filename_apostrophed_windows1251.eml").as_ref(),
        include_bytes!("../test-data/message/attach_filename_combined.eml").as_ref(),
        include_bytes!("../test-data/message/attach_filename_encoded_words_bad_delimiter.eml")
            .as_ref(),
        include_bytes!("../test-data/message/attach_filename_encoded_words_binary.eml").as_ref(),
        include_bytes!("../test-data/message/attach_filename_encoded_words_cont.eml").as_ref(),
        include_bytes!("../test-data/message/attach_filename_encoded_words.eml").as_ref(),
        include_bytes!("../test-data/message/attach_filename_encoded_words_windows1251.eml")
            .as_ref(),
        include_bytes!("../test-data/message/attach_filename_simple.eml").as_ref(),
        include_bytes!("../test-data/message/AutocryptSetupMessage.eml").as_ref(),
        include_bytes!("../test-data/message/blockquote-tag.eml").as_ref(),
        include_bytes!("../test-data/message/cp1252-html.eml").as_ref(),
        include_bytes!("../test-data/message/gmail_ndn.eml").as_ref(),
        include_bytes!("../test-data/message/gmail_ndn_group.eml").as_ref(),
        include_bytes!("../test-data/message/gmx-forward.eml").as_ref(),
        include_bytes!("../test-data/message/gmx_ndn.eml").as_ref(),
        include_bytes!("../test-data/message/gmx-quote-body.eml").as_ref(),
        include_bytes!("../test-data/message/gmx-quote.eml").as_ref(),
        include_bytes!("../test-data/message/mail_attach_txt.eml").as_ref(),
        include_bytes!("../test-data/message/mailinglist_dhl.eml").as_ref(),
        include_bytes!("../test-data/message/mailinglist_dpd.eml").as_ref(),
        include_bytes!("../test-data/message/mailinglist_ttline.eml").as_ref(),
        include_bytes!("../test-data/message/mailinglist_with_mimepart_footer.eml").as_ref(),
        include_bytes!("../test-data/message/mailinglist_with_mimepart_footer_signed.eml").as_ref(),
        include_bytes!("../test-data/message/mailinglist_xing.eml").as_ref(),
        include_bytes!("../test-data/message/mailinglist_xt_local_microsoft.eml").as_ref(),
        include_bytes!("../test-data/message/mailinglist_xt_local_spiegel.eml").as_ref(),
        include_bytes!("../test-data/message/mail_with_user_and_group_avatars.eml").as_ref(),
        include_bytes!("../test-data/message/mail_with_user_avatar_deleted.eml").as_ref(),
        include_bytes!("../test-data/message/mail_with_user_avatar.eml").as_ref(),
        include_bytes!("../test-data/message/many_images_amazon_via_apple_mail.eml").as_ref(),
        include_bytes!("../test-data/message/pdf_filename_continuation.eml").as_ref(),
        include_bytes!("../test-data/message/pdf_filename_simple.eml").as_ref(),
        include_bytes!("../test-data/message/posteo_ndn.eml").as_ref(),
        include_bytes!("../test-data/message/protonmail-mixed-up.eml").as_ref(),
        include_bytes!("../test-data/message/protonmail-repaired.eml").as_ref(),
        include_bytes!("../test-data/message/quote_attach.eml").as_ref(),
        include_bytes!("../test-data/message/subj_with_multimedia_msg.eml").as_ref(),
        include_bytes!("../test-data/message/testrun_ndn_2.eml").as_ref(),
        include_bytes!("../test-data/message/testrun_ndn.eml").as_ref(),
        include_bytes!("../test-data/message/text_alt_html.eml").as_ref(),
        include_bytes!("../test-data/message/text_alt_plain.eml").as_ref(),
        include_bytes!("../test-data/message/text_alt_plain_html.eml").as_ref(),
        include_bytes!("../test-data/message/text_html.eml").as_ref(),
        include_bytes!("../test-data/message/text_plain_flowed.eml").as_ref(),
        include_bytes!("../test-data/message/text_plain_iso88591.eml").as_ref(),
        include_bytes!("../test-data/message/text_plain_unspecified.eml").as_ref(),
        include_bytes!("../test-data/message/tiscali_ndn.eml").as_ref(),
        include_bytes!("../test-data/message/videochat_invitation.eml").as_ref(),
        include_bytes!("../test-data/message/wrong-html.eml").as_ref(),
        include_bytes!("../test-data/message/yahoo_ndn.eml").as_ref(),
    ];
    recv_emails(context, &emails).await
}

async fn create_context() -> Context {
    let dir = tempdir().unwrap();
    let dbfile = dir.path().join("db.sqlite");
    let id = 100;
    let context = Context::new("FakeOS".into(), dbfile.into(), id)
        .await
        .unwrap();

    let backup: PathBuf = std::env::current_dir()
        .unwrap()
        .join("delta-chat-backup.tar")
        .into();
    if backup.exists().await {
        println!("Importing backup");
        imex(&context, ImexMode::ImportBackup, &backup)
            .await
            .unwrap();
    }

    let addr = "alice@example.com";
    context.set_config(Config::Addr, Some(addr)).await.unwrap();
    context
        .set_config(Config::ConfiguredAddr, Some(addr))
        .await
        .unwrap();
    context
        .set_config(Config::Configured, Some("1"))
        .await
        .unwrap();
    context
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("from_elem");
    for needs_move_enabled in [false, true] {
        group.bench_with_input(
            BenchmarkId::new("Receive many messages", needs_move_enabled),
            &needs_move_enabled,
            |b, needs_move_enabled| {
                b.to_async(AsyncStdExecutor).iter_batched(
                    || block_on(create_context()),
                    |context| recv_all_emails(black_box(context), *needs_move_enabled),
                    BatchSize::LargeInput,
                );
            },
        );
    }
    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
