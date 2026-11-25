//! Track 70 Phase 3 Task 3.3: Criterion benchmarks for durable run-output
//! publication (spec: "Benchmark allocations and zero-copy claims separately
//! from query speed", "Benchmark current small and large traces,
//! high-cardinality joins and concurrent workloads; preserve existing
//! regression gates").
//!
//! Measured surfaces: small vs. large trace publication, high-cardinality
//! workloads, concurrent multi-replication publication, completed-run
//! validation (hash + decode re-attestation), and manifest reads. Each
//! iteration publishes into a fresh unique run directory; the previous
//! iteration's directory is removed in the (unmeasured) setup closure.
//!
//! Regression gates: the hosted `bench-regression.yml` workflow watches
//! `crates/kairo-ecs-core`, `crates/kairo-ecs-state`, `benches/` and
//! `crates/kairo-ecs-bench` — untouched and preserved. These arrow-surface
//! benchmarks are additive; wiring them into a hosted threshold gate is
//! recorded as a Track 31/ci-agent follow-up in the track handoff.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use criterion::{criterion_group, criterion_main, BatchSize, Criterion, Throughput};
use kairo_ecs_arrow::run_output::{
    read_completed_manifest, validate_run_output, RunOutput, RunOutputConfig,
};
use kairo_ecs_arrow::{EventLogRecord, EventStatus, SCHEMA_VERSION_V2};
use kairo_ecs_types::{EntityId, EventId};
use std::hint::black_box;

const SMALL_ROWS: usize = 100;
const LARGE_BATCHES: usize = 20;
const LARGE_ROWS_PER_BATCH: usize = 1_000;
const CARDINALITY_ROWS: usize = 5_000;
const CONCURRENT_REPLICATIONS: u64 = 4;
const CONCURRENT_ROWS_PER_REPLICATION: usize = 1_250;

/// Previous iteration's run parent directory, removed in the next (unmeasured)
/// setup so measured time excludes cleanup.
static PREVIOUS_PARENT: Mutex<Option<PathBuf>> = Mutex::new(None);

fn fresh_parent(label: &str) -> PathBuf {
    let mut previous = PREVIOUS_PARENT.lock().expect("bench mutex");
    if let Some(path) = previous.take() {
        let _ = fs::remove_dir_all(path);
    }
    let parent = std::env::temp_dir().join(format!(
        "kairo-ecs-arrow-bench-p3-{label}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&parent);
    let builder = fs::DirBuilder::new();
    #[cfg(unix)]
    let builder = {
        use std::os::unix::fs::DirBuilderExt;
        let mut builder = builder;
        builder.mode(0o700);
        builder
    };
    builder.create(&parent).expect("private bench parent");
    *previous = Some(parent.clone());
    parent
}

fn sample_record(sequence: u64, time_ticks: u128, kind_suffix: u64) -> EventLogRecord {
    EventLogRecord {
        schema_version: SCHEMA_VERSION_V2,
        run_id: "bench-run".to_string(),
        event_id: EventId::new(sequence, 0),
        entity_id: Some(EntityId::new(sequence % 512, 0)),
        time_ticks,
        time_scale: "ticks".to_string(),
        priority: (sequence % 4) as i32,
        sequence,
        event_kind: format!("custom:kind-{kind_suffix}"),
        status: EventStatus::Dispatched,
        payload_ref: None,
    }
}

fn records(range: impl IntoIterator<Item = u64>, cardinality: u64) -> Vec<EventLogRecord> {
    range
        .into_iter()
        .map(|sequence| {
            let kind = if cardinality == 0 {
                sequence % 8
            } else {
                (sequence % cardinality).wrapping_mul(2_654_435_761) % cardinality
            };
            sample_record(sequence, u128::from(sequence) * 3, kind)
        })
        .collect()
}

fn config(max_rows_per_part: usize, replication_id: u64) -> RunOutputConfig {
    RunOutputConfig {
        run_id: "bench-run".to_string(),
        replication_id,
        seed: 0xBEEF,
        time_scale: "ticks".to_string(),
        effective_input_hash: "ab".repeat(32),
        raw_input_hash: Some("cd".repeat(32)),
        max_rows_per_part: std::num::NonZeroUsize::new(max_rows_per_part)
            .expect("nonzero part bound"),
    }
}

fn publish(parent: &Path, cfg: &RunOutputConfig, batches: &[Vec<EventLogRecord>]) -> usize {
    let mut run = RunOutput::create(cfg.clone(), parent).expect("bench create");
    for batch in batches {
        run.write_batch(batch).expect("bench write_batch");
    }
    let manifest = run.complete().expect("bench complete");
    manifest.parts.len()
}

fn bench_small_trace(c: &mut Criterion) {
    let mut group = c.benchmark_group("run_output_publication");
    group.throughput(Throughput::Elements(SMALL_ROWS as u64));
    group.bench_function("small_trace_100_rows_1_part", |b| {
        b.iter_batched(
            || fresh_parent("small"),
            |parent| {
                publish(
                    &parent,
                    &config(SMALL_ROWS, 1),
                    &[records(1..=SMALL_ROWS as u64, 0)],
                )
            },
            BatchSize::PerIteration,
        );
    });
    group.finish();
}

fn bench_large_trace(c: &mut Criterion) {
    let batches: Vec<Vec<EventLogRecord>> = (0..LARGE_BATCHES as u64)
        .map(|chunk| {
            records(
                chunk * LARGE_ROWS_PER_BATCH as u64 + 1..(chunk + 1) * LARGE_ROWS_PER_BATCH as u64,
                0,
            )
        })
        .collect();
    let mut group = c.benchmark_group("run_output_publication");
    group.throughput(Throughput::Elements(
        (LARGE_BATCHES * LARGE_ROWS_PER_BATCH) as u64,
    ));
    group.bench_function("large_trace_20000_rows_20_parts", |b| {
        b.iter_batched(
            || fresh_parent("large"),
            |parent| publish(&parent, &config(LARGE_ROWS_PER_BATCH, 1), &batches),
            BatchSize::PerIteration,
        );
    });
    group.finish();
}

fn bench_high_cardinality(c: &mut Criterion) {
    let batches: Vec<Vec<EventLogRecord>> = (0..5u64)
        .map(|chunk| {
            records(
                chunk * (CARDINALITY_ROWS as u64 / 5) + 1
                    ..(chunk + 1) * (CARDINALITY_ROWS as u64 / 5),
                CARDINALITY_ROWS as u64,
            )
        })
        .collect();
    let mut group = c.benchmark_group("run_output_publication");
    group.throughput(Throughput::Elements(CARDINALITY_ROWS as u64));
    group.bench_function("high_cardinality_5000_distinct_kinds", |b| {
        b.iter_batched(
            || fresh_parent("cardinality"),
            |parent| publish(&parent, &config(CARDINALITY_ROWS / 5, 1), &batches),
            BatchSize::PerIteration,
        );
    });
    group.finish();
}

fn bench_concurrent_publication(c: &mut Criterion) {
    let mut group = c.benchmark_group("run_output_publication");
    group.throughput(Throughput::Elements(
        CONCURRENT_REPLICATIONS * CONCURRENT_ROWS_PER_REPLICATION as u64,
    ));
    group.bench_function("concurrent_4_replications_5000_rows", |b| {
        b.iter_batched(
            || fresh_parent("concurrent"),
            |parent| {
                thread::scope(|scope| {
                    let handles: Vec<_> = (1..=CONCURRENT_REPLICATIONS)
                        .map(|replication| {
                            let parent = parent.clone();
                            scope.spawn(move || {
                                let rows_per_batch = CONCURRENT_ROWS_PER_REPLICATION / 2;
                                let batches: Vec<Vec<EventLogRecord>> = (0..2u64)
                                    .map(|chunk| {
                                        records(
                                            chunk * rows_per_batch as u64 + 1
                                                ..(chunk + 1) * rows_per_batch as u64,
                                            0,
                                        )
                                    })
                                    .collect();
                                publish(&parent, &config(rows_per_batch, replication), &batches)
                            })
                        })
                        .collect();
                    for handle in handles {
                        black_box(handle.join().expect("bench replication joins"));
                    }
                })
            },
            BatchSize::PerIteration,
        );
    });
    group.finish();
    #[cfg(feature = "parallel-io")]
    bench_contiguous_ipc(c);
}

fn completed_large_run() -> (PathBuf, PathBuf) {
    let parent = fresh_parent("validate");
    let run_dir = parent.join(format!("bench-run-rep{}", 1));
    let batches: Vec<Vec<EventLogRecord>> = (0..LARGE_BATCHES as u64)
        .map(|chunk| {
            records(
                chunk * LARGE_ROWS_PER_BATCH as u64 + 1..(chunk + 1) * LARGE_ROWS_PER_BATCH as u64,
                0,
            )
        })
        .collect();
    publish(&parent, &config(LARGE_ROWS_PER_BATCH, 1), &batches);
    (parent, run_dir)
}

#[cfg(feature = "parallel-io")]
fn bench_contiguous_ipc(c: &mut Criterion) {
    use kairo_ecs_arrow::native_ipc::NativeIpcBatch;
    use kairo_ecs_arrow::{ParallelIoRecordBatch, SCHEMA_VERSION};

    const WRITERS: u64 = 4;
    const ROWS_PER_STREAM: u64 = 16_384;
    let batches: Vec<_> = (0..WRITERS)
        .map(|writer| {
            let start = writer * ROWS_PER_STREAM;
            let mut records = records(start..start + ROWS_PER_STREAM, 512);
            for record in &mut records {
                record.schema_version = SCHEMA_VERSION;
            }
            let batch = ParallelIoRecordBatch::from_records(records).expect("validated event rows");
            NativeIpcBatch::from_event_log(&batch).expect("native IPC batch")
        })
        .collect();
    let expected: Vec<_> = batches
        .iter()
        .map(|batch| {
            batch
                .write_contiguous(std::io::sink())
                .expect("encode reference stream")
        })
        .collect();
    let write = |idx: usize, batch: &NativeIpcBatch, dir: &Path| {
        let receipt = batch
            .write_contiguous_to_path(dir.join(format!("part-{idx}.arrows")))
            .expect("publish IPC stream");
        assert_eq!(receipt, expected[idx]);
        black_box(receipt);
    };
    if let Some(destination) = std::env::var_os("KAIROS_IPC_BENCHMARK_CAPTURE") {
        let destination = Path::new(&destination);
        fs::create_dir(destination).expect("new capture directory");
        for (idx, batch) in batches.iter().enumerate() {
            write(idx, batch, destination);
            let mut reference = Vec::new();
            batch
                .write_contiguous(&mut reference)
                .expect("encode retained stream reference");
            let observed = fs::read(destination.join(format!("part-{idx}.arrows")))
                .expect("read retained stream");
            assert!(observed == reference, "retained stream {idx} differs");
        }
    }
    let mut group = c.benchmark_group("contiguous_ipc_publication");
    group.throughput(Throughput::Elements(WRITERS * ROWS_PER_STREAM));
    for (name, parallel) in [("serial", false), ("parallel_4", true)] {
        group.bench_function(name, |b| {
            b.iter_batched_ref(
                || tempfile::tempdir().expect("private benchmark directory"),
                |dir| {
                    let path = dir.path();
                    if parallel {
                        thread::scope(|scope| {
                            for (idx, batch) in batches.iter().enumerate() {
                                scope.spawn(move || write(idx, batch, path));
                            }
                        });
                    } else {
                        for (idx, batch) in batches.iter().enumerate() {
                            write(idx, batch, path);
                        }
                    }
                },
                BatchSize::PerIteration,
            );
        });
    }
    group.finish();
}

fn bench_validate_completed_run(c: &mut Criterion) {
    let mut group = c.benchmark_group("run_output_reading");
    group.throughput(Throughput::Elements(
        (LARGE_BATCHES * LARGE_ROWS_PER_BATCH) as u64,
    ));
    group.bench_function("validate_large_trace_20000_rows", |b| {
        b.iter_batched(
            completed_large_run,
            |(_parent, run_dir)| {
                let manifest = validate_run_output(&run_dir).expect("bench validate");
                black_box(manifest.parts.len());
            },
            BatchSize::PerIteration,
        );
    });
    group.finish();
}

fn bench_read_completed_manifest(c: &mut Criterion) {
    let mut group = c.benchmark_group("run_output_reading");
    group.throughput(Throughput::Elements(
        (LARGE_BATCHES * LARGE_ROWS_PER_BATCH) as u64,
    ));
    group.bench_function("read_completed_manifest", |b| {
        b.iter_batched(
            completed_large_run,
            |(_parent, run_dir)| {
                let manifest = read_completed_manifest(&run_dir).expect("bench read");
                black_box(manifest.parts.len());
            },
            BatchSize::PerIteration,
        );
    });
    group.finish();
}

criterion_group! {
    name = run_output_benches;
    config = Criterion::default()
        .sample_size(10)
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2));
    targets =
        bench_small_trace,
        bench_large_trace,
        bench_high_cardinality,
        bench_concurrent_publication,
        bench_validate_completed_run,
        bench_read_completed_manifest
}
criterion_main!(run_output_benches);
