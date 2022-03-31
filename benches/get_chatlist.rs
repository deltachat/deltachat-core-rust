use criterion::async_executor::AsyncStdExecutor;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

use deltachat::chatlist::Chatlist;
use deltachat::context::Context;

async fn get_chat_list_benchmark(context: &Context) {
    Chatlist::try_load(&context, 0, None, None).await.unwrap();
}

fn criterion_benchmark(c: &mut Criterion) {
    // To enable this benchmark, set `DELTACHAT_BENCHMARK_DATABASE` to some large database with many
    // messages, such as your primary account.
    if let Ok(path) = std::env::var("DELTACHAT_BENCHMARK_DATABASE") {
        let context =
            async_std::task::block_on(async { Context::new(path.into(), 100).await.unwrap() });
        c.bench_function("chatlist:try_load (Get Chatlist)", |b| {
            b.to_async(AsyncStdExecutor)
                .iter(|| get_chat_list_benchmark(black_box(&context)))
        });
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
