use criterion::{black_box, criterion_group, criterion_main, Criterion};
use kairo_ecs_debug::EventTrace;
use kairo_ecs_types::{DispatchedEvent, EventId, EventKind, SimTime};
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

    c.bench_function("reconstruct_at/early_query_many_snapshots", |b| {
        b.iter(|| {
            black_box(trace.reconstruct_at(black_box(target_tick)));
        })
    });

    let mut deltas = EventTrace::default();
    for tick in 1..=num_snapshots {
        let mut changes = BTreeMap::new();
        changes.insert("machine.status".to_string(), format!("state_{tick}"));
        deltas.record_event(
            DispatchedEvent {
                id: EventId {
                    index: tick as u64,
                    generation: 0,
                },
                at: SimTime::from_ticks(tick as u128),
                priority: 0,
                sequence: tick as u64,
                entity: None,
                kind: EventKind::Custom(0),
            },
            changes,
        );
    }

    c.bench_function("reconstruct_at/early_query_many_deltas", |b| {
        b.iter(|| {
            black_box(deltas.reconstruct_at(black_box(target_tick)));
        })
    });
}

criterion_group!(benches, bench_reconstruct_at);
criterion_main!(benches);
