//! Benchmarks for data retrieval/storage and N+1 selection against the
//! in-memory store. Establishes a baseline we can track as the corpus grows.

use criterion::{criterion_group, criterion_main, Criterion};
use scaffold_api::engine;
use scaffold_api::repo::memory::MemoryRepo;
use scaffold_api::seed;
use scaffold_api::services::telemetry::MockTelemetry;
use scaffold_core::fsrs::Rating;

fn bench(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let (repo, physics_id, beginner) = rt.block_on(async {
        let repo = MemoryRepo::new();
        let res = seed::seed(&repo).await;
        let beginner = seed::id("profile", "beginner");
        (repo, res.physics.id, beginner)
    });

    c.bench_function("comprehension_map_read", |b| {
        b.to_async(&rt)
            .iter(|| async { engine::comprehension_map(&repo, beginner).await.unwrap() });
    });

    c.bench_function("recommend_n_plus_one", |b| {
        b.to_async(&rt)
            .iter(|| async { engine::recommend(&repo, beginner, physics_id).await.unwrap() });
    });

    let tele = MockTelemetry::default();
    let item = seed::id("item", "physics-201:fc-limits");
    c.bench_function("review_write", |b| {
        b.to_async(&rt).iter(|| async {
            engine::review(&repo, &tele, beginner, item, Rating::Good)
                .await
                .unwrap()
        });
    });
}

criterion_group!(benches, bench);
criterion_main!(benches);
