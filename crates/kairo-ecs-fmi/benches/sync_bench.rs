use criterion::{black_box, criterion_group, criterion_main, Criterion};
use kairo_ecs_fmi::digital_twin::sync::{TwinStateEntry, TwinStateSnapshot};

fn generate_snapshot(tick: u64, size: usize, start_val: usize) -> TwinStateSnapshot {
    let entries = (0..size)
        .map(|i| TwinStateEntry::new(format!("key_{i}"), format!("val_{}", i + start_val)))
        .collect();
    TwinStateSnapshot::new(tick, entries)
}

fn bench_diff(c: &mut Criterion) {
    let size = 1000;

    // baseline
    let before = generate_snapshot(1, size, 0);
    // some overlap, some changed, some removed (by not being included)
    let mut after_entries = Vec::new();
    for i in (size / 4)..(size + size / 4) {
        // keep 75%, change 50% of kept, remove 25%, add 25% new
        if i % 2 == 0 {
            after_entries.push(TwinStateEntry::new(
                format!("key_{i}"),
                format!("val_{}", i),
            )); // Same
        } else {
            after_entries.push(TwinStateEntry::new(
                format!("key_{i}"),
                format!("val_{}", i + 100),
            )); // Changed
        }
    }
    let after = TwinStateSnapshot::new(2, after_entries);

    c.bench_function("TwinStateSnapshot::diff", |b| {
        b.iter(|| black_box(before.diff(black_box(&after))))
    });
}

criterion_group!(benches, bench_diff);
criterion_main!(benches);
