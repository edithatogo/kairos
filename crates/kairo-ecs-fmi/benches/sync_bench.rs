use criterion::{black_box, criterion_group, criterion_main, Criterion};
use kairo_ecs_fmi::digital_twin::sync::{TwinStateEntry, TwinStateSnapshot};

fn bench_sync(c: &mut Criterion) {
    let mut group = c.benchmark_group("TwinStateSync");

    let num_entries = 1000;
    let entries: Vec<TwinStateEntry> = (0..num_entries)
        .map(|i| TwinStateEntry::new(format!("key_{:04}", i), format!("value_{:04}", i)))
        .collect();

    let mut next_entries = entries.clone();
    // Change first 100
    for (i, entry) in next_entries.iter_mut().enumerate().take(100) {
        entry.value = format!("new_value_{:04}", i);
    }
    // Remove last 100
    for _ in 900..1000 {
        next_entries.remove(900);
    }
    // Add 100 new
    for i in 1000..1100 {
        next_entries.push(TwinStateEntry::new(
            format!("key_{:04}", i),
            format!("value_{:04}", i),
        ));
    }

    let snapshot_before = TwinStateSnapshot::new(1, entries);
    let snapshot_after = TwinStateSnapshot::new(2, next_entries);

    // Make sure we have a valid diff to apply
    let diff = snapshot_before.diff(&snapshot_after);

    group.bench_function("diff", |b| {
        b.iter(|| black_box(&snapshot_before).diff(black_box(&snapshot_after)))
    });

    group.bench_function("apply", |b| {
        b.iter(|| black_box(&snapshot_before).apply(black_box(&diff)))
    });

    group.finish();
}

criterion_group!(benches, bench_sync);
criterion_main!(benches);
