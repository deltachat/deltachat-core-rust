use criterion::async_executor::AsyncStdExecutor;
use criterion::{criterion_group, criterion_main, Criterion};

use deltachat::context::Context;
use deltachat::{info, Event, EventType};
use tempfile::tempdir;

async fn send_events_benchmark(context: &Context) {
    let emitter = context.get_event_emitter();
    for _i in 0..1_000_000 {
        info!(context, "interesting event...");
    }
    info!(context, "DONE");

    loop {
        match emitter.recv().await.unwrap() {
            Event {
                typ: EventType::Info(info),
                ..
            } if info.contains("DONE") => {
                break;
            }
            _ => {}
        }
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    let dir = tempdir().unwrap();
    let dbfile = dir.path().join("db.sqlite");
    let id = 100;
    let context =
        async_std::task::block_on(async { Context::new(dbfile.into(), id).await.unwrap() });

    c.bench_function("Sending 1000 events", |b| {
        b.to_async(AsyncStdExecutor)
            .iter(|| send_events_benchmark(&context))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
