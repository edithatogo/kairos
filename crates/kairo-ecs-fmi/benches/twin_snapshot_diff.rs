use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use kairo_ecs_fmi::digital_twin::sync::{TwinStateEntry, TwinStateSnapshot};

// This is the pre-optimization implementation retained as a benchmark baseline.
// Both algorithms receive the same constructor-produced sorted, unique inputs;
// only diff computation is timed (snapshot construction is outside the loop).
fn quadratic_diff(
    before: &TwinStateSnapshot,
    next: &TwinStateSnapshot,
) -> (Vec<TwinStateEntry>, Vec<String>) {
    let changed = next
        .entries
        .iter()
        .filter(|entry| {
            before
                .entries
                .iter()
                .find(|candidate| candidate.key == entry.key)
                .map(|candidate| candidate.value != entry.value)
                .unwrap_or(true)
        })
        .cloned()
        .collect();

    let removed = before
        .entries
        .iter()
        .filter(|entry| {
            !next
                .entries
                .iter()
                .any(|candidate| candidate.key == entry.key)
        })
        .map(|entry| entry.key.clone())
        .collect();

    (changed, removed)
}

fn snapshots(size: usize) -> (TwinStateSnapshot, TwinStateSnapshot) {
    let before = (0..size)
        .map(|index| TwinStateEntry::new(format!("key-{index:08}"), format!("value-{index:08}")))
        .collect();
    let next = (0..size)
        .map(|index| {
            let value = if index == size / 2 {
                format!("updated-{index:08}")
            } else {
                format!("value-{index:08}")
            };
            TwinStateEntry::new(format!("key-{index:08}"), value)
        })
        .collect();
    (
        TwinStateSnapshot::new(10, before),
        TwinStateSnapshot::new(11, next),
    )
}

fn bench_snapshot_diff(c: &mut Criterion) {
    let mut group = c.benchmark_group("digital_twin_snapshot_diff_one_changed_entry");
    for size in [128usize, 1_024, 8_192] {
        let (before, next) = snapshots(size);
        let baseline = quadratic_diff(&before, &next);
        let optimized = before.diff(&next);
        assert_eq!(
            baseline.0, optimized.changed,
            "changed output at size {size}"
        );
        assert_eq!(
            baseline.1, optimized.removed,
            "removed output at size {size}"
        );

        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::new("pre_optimization_quadratic", size),
            &size,
            |b, _| b.iter(|| black_box(quadratic_diff(black_box(&before), black_box(&next)))),
        );
        group.bench_with_input(BenchmarkId::new("current_diff", size), &size, |b, _| {
            b.iter(|| black_box(black_box(&before).diff(black_box(&next))))
        });
    }
    group.finish();
}

criterion_group!(benches, bench_snapshot_diff);
criterion_main!(benches);
