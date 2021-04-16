use async_std::task::block_on;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use deltachat::contact::Contact;
use deltachat::context::Context;
use tempfile::tempdir;

async fn address_book_benchmark(n: u32) {
    let dir = tempdir().unwrap();
    let dbfile = dir.path().join("db.sqlite");
    let id = 100;
    let context = Context::new("FakeOS".into(), dbfile.into(), id)
        .await
        .unwrap();

    let book = (0..n)
        .map(|i| format!("Name {}\naddr{}@example.org\n", i, i))
        .collect::<Vec<String>>()
        .join("");

    Contact::add_address_book(&context, book).await.unwrap();
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("create 500 contacts", |b| {
        b.iter(|| block_on(async { address_book_benchmark(black_box(500)).await }))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
