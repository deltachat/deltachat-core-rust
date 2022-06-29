use std::path::PathBuf;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use deltachat::{
    config::Config,
    context::Context,
    imex::{imex, ImexMode},
    receive_imf::dc_receive_imf,
    Events,
};
use tempfile::tempdir;

async fn recv_all_emails(context: Context) -> Context {
    for i in 0..100 {
        let imf_raw = format!(
            "Subject: Benchmark
Message-ID: Mr.OssSYnOFkhR.{i}@testrun.org
Date: Sat, 07 Dec 2019 19:00:27 +0000
To: alice@example.com
From: sender@testrun.org
Chat-Version: 1.0
Chat-Disposition-Notification-To: sender@testrun.org
Chat-User-Avatar: 0
In-Reply-To: Mr.OssSYnOFkhR.{i_dec}@testrun.org
MIME-Version: 1.0

Content-Type: text/plain; charset=utf-8; format=flowed; delsp=no

Hello {i}",
            i = i,
            i_dec = i - 1,
        );
        dc_receive_imf(&context, black_box(imf_raw.as_bytes()), false)
            .await
            .unwrap();
    }
    context
}

async fn create_context() -> Context {
    let dir = tempdir().unwrap();
    let dbfile = dir.path().join("db.sqlite");
    let id = 100;
    let context = Context::new(&dbfile, id, Events::new()).await.unwrap();

    let backup: PathBuf = std::env::current_dir()
        .unwrap()
        .join("delta-chat-backup.tar");

    if backup.exists() {
        println!("Importing backup");
        imex(&context, ImexMode::ImportBackup, &backup, None)
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
    let mut group = c.benchmark_group("Receive messages");
    group.bench_function("Receive 100 simple text msgs", |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let context = rt.block_on(create_context());

        b.to_async(&rt).iter(|| {
            let ctx = context.clone();
            async move {
                recv_all_emails(black_box(ctx)).await;
            }
        });
    });
    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
