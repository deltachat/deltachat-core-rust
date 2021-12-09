use async_std::task::block_on;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use deltachat::context::Context;
use std::path::Path;

async fn search_benchmark(path: impl AsRef<Path>) {
    let dbfile = path.as_ref();
    let id = 100;
    let context = Context::new(dbfile.into(), id).await.unwrap();

    for _ in 0..10u32 {
        context.search_msgs(None, "hello").await.unwrap();
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    // To enable this benchmark, set `DELTACHAT_BENCHMARK_DATABASE` to some large database with many
    // messages, such as your primary account.
    if let Ok(path) = std::env::var("DELTACHAT_BENCHMARK_DATABASE") {
        c.bench_function("search hello", |b| {
            b.iter(|| block_on(async { search_benchmark(black_box(&path)).await }))
        });
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
