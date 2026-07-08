use criterion::{black_box, criterion_group, criterion_main, Criterion};
use kairo_ecs_debug::EventTrace;
use kairo_ecs_types::SimTime;
use std::collections::BTreeMap;

fn bench_reconstruct_at(c: &mut Criterion) {
    let mut trace = EventTrace::default();
    let num_snapshots = 10_000;

    // Add many snapshots
    for i in 1..=num_snapshots {
        let mut state = BTreeMap::new();
        state.insert("key".to_string(), format!("value_{}", i));
        trace.snapshot(SimTime::from_ticks(i as u128 * 10), state);
    }

    // Querying near the beginning (worst case for iter().rev())
    let target_tick = (num_snapshots / 10) as u128 * 10;

    c.bench_function("reconstruct_at", |b| {
        b.iter(|| {
            black_box(trace.reconstruct_at(black_box(target_tick)));
        })
    });
}

criterion_group!(benches, bench_reconstruct_at);
criterion_main!(benches);
