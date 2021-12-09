use async_std::path::PathBuf;
use async_std::task::block_on;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use deltachat::accounts::Accounts;
use tempfile::tempdir;

async fn create_accounts(n: u32) {
    let dir = tempdir().unwrap();
    let p: PathBuf = dir.path().join("accounts").into();

    let mut accounts = Accounts::new(p.clone()).await.unwrap();

    for expected_id in 2..n {
        let id = accounts.add_account().await.unwrap();
        assert_eq!(id, expected_id);
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("create 1 account", |b| {
        b.iter(|| block_on(async { create_accounts(black_box(1)).await }))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
