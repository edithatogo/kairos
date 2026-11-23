#![cfg(all(
    feature = "engine-checkpoint",
    any(
        feature = "hdf5",
        all(feature = "adios2", any(target_os = "linux", target_os = "macos"))
    )
))]

use std::collections::VecDeque;
use std::path::Path;
use std::process::Command;

use kairo_ecs_arrow::{
    engine_checkpoint::{restore_engine_checkpoint, write_engine_checkpoint},
    ArrowError, CheckpointFormat, CheckpointManifest, ParallelIoRecordBatch,
};
use kairo_ecs_core::Scheduler;
use kairo_ecs_rng::DeterministicStream;
use kairo_ecs_types::{DispatchedEvent, EntityId, EventId, EventKind, ScheduleRequest, SimTime};
use serde::{Deserialize, Serialize};

const SCHEMA: &str = "service-queue.v1";
const JOBS: u64 = 128;
const SERVERS: u64 = 3;
const START_TICK: u128 = u64::MAX as u128 + 31;
const CUT: usize = 28;
const SEED: u64 = 20261123;

#[derive(Debug, Deserialize, Serialize)]
struct WaitingJob {
    entity: (u64, u32),
    timeout: (u64, u32),
    arrived: u128,
}

#[derive(Debug, Deserialize, Serialize)]
struct Model {
    seed: u64,
    rng_position: u128,
    active: u64,
    waiting: VecDeque<WaitingJob>,
    completed: u64,
    expired: u64,
    total_wait: u128,
}

struct Run {
    model: Model,
    scheduler: Scheduler,
    rng: DeterministicStream,
}

impl Run {
    fn new() -> Self {
        let mut scheduler = Scheduler::new();
        for index in 0..JOBS {
            scheduler.schedule(request(
                START_TICK + u128::from(index / 3) * 2,
                2,
                EntityId::new(index, 1),
                0,
            ));
        }
        Self {
            model: Model {
                seed: SEED,
                rng_position: 0,
                active: 0,
                waiting: VecDeque::new(),
                completed: 0,
                expired: 0,
                total_wait: 0,
            },
            scheduler,
            rng: DeterministicStream::new(SEED),
        }
    }

    fn step(&mut self) -> Option<DispatchedEvent> {
        let event = self.scheduler.step().into_dispatched()?;
        let actor = event.entity.unwrap();
        let now = event.at.ticks();
        match event.kind.code() {
            0 if self.model.active < SERVERS => self.start(actor, now, now),
            0 => {
                let timeout = self.scheduler.schedule(request(now + 11, 1, actor, 2));
                self.model.waiting.push_back(WaitingJob {
                    entity: (actor.index, actor.generation),
                    timeout: (timeout.index, timeout.generation),
                    arrived: now,
                });
            }
            1 => {
                self.model.active -= 1;
                self.model.completed += 1;
                if let Some(next) = self.model.waiting.pop_front() {
                    assert!(self
                        .scheduler
                        .cancel(EventId::new(next.timeout.0, next.timeout.1)));
                    self.start(
                        EntityId::new(next.entity.0, next.entity.1),
                        next.arrived,
                        now,
                    );
                }
            }
            2 => {
                let index = self
                    .model
                    .waiting
                    .iter()
                    .position(|job| job.entity.0 == actor.index)
                    .unwrap();
                self.model.waiting.remove(index);
                self.model.expired += 1;
            }
            _ => unreachable!("unknown event in the service model"),
        }
        Some(event)
    }

    fn start(&mut self, actor: EntityId, arrived: u128, now: u128) {
        self.model.active += 1;
        self.model.total_wait += now - arrived;
        let duration = 3 + u128::from(self.rng.next_u32() % 8);
        self.model.rng_position = self.rng.word_pos();
        self.scheduler
            .schedule(request(now + duration, 0, actor, 1));
    }

    fn finish(mut self) -> String {
        let mut suffix = Vec::new();
        while let Some(event) = self.step() {
            let actor = event.entity.unwrap();
            suffix.push((
                event.id.index,
                event.id.generation,
                event.at.ticks(),
                event.priority,
                event.sequence,
                actor.index,
                actor.generation,
                event.kind.code(),
            ));
        }
        assert_eq!(self.model.active, 0);
        assert!(self.model.waiting.is_empty());
        assert_eq!(self.model.completed + self.model.expired, JOBS);
        assert!(self.model.completed > 0 && self.model.expired > 0);
        let stats = self.scheduler.stats();
        assert!(stats.cancelled_events > 0);
        assert_eq!(stats.pending_events, 0);
        serde_json::to_string(&(
            suffix,
            self.model,
            (
                stats.now.ticks(),
                stats.scheduled_events,
                stats.dispatched_events,
                stats.cancelled_events,
                stats.pending_events,
            ),
            self.rng.next_u64(),
        ))
        .unwrap()
    }
}

fn request(tick: u128, priority: i32, actor: EntityId, kind: u32) -> ScheduleRequest {
    ScheduleRequest {
        at: SimTime::from_ticks(tick),
        priority,
        entity: Some(actor),
        kind: EventKind::custom(kind),
    }
}

#[test]
fn checkpoint_process() {
    let Ok(mode) = std::env::var("KAIRO_ENGINE_TEST_MODE") else {
        return;
    };
    let output = std::env::var_os("KAIRO_ENGINE_TEST_PATH").unwrap();
    let path = Path::new(&output);
    let format =
        CheckpointFormat::try_from(std::env::var("KAIRO_ENGINE_TEST_FORMAT").unwrap().as_str())
            .unwrap();
    if mode == "resume" {
        let restored = restore_engine_checkpoint(path, format, SCHEMA).unwrap();
        let model: Model = serde_json::from_slice(&restored.model_state).unwrap();
        assert_eq!(restored.manifest.checkpoint_id, "service-queue-cut");
        assert_eq!(restored.manifest.records.len(), CUT);
        assert_eq!(restored.scheduler.stats().dispatched_events, CUT as u64);
        assert!(restored.scheduler.pending_events() > 0 && !model.waiting.is_empty());
        let mut rng = DeterministicStream::new(model.seed);
        rng.set_word_pos(model.rng_position);
        println!(
            "RESTART_RESULT={}",
            Run {
                model,
                scheduler: restored.scheduler,
                rng
            }
            .finish()
        );
    } else {
        let mut run = Run::new();
        let events: Vec<_> = (0..CUT).map(|_| run.step().unwrap()).collect();
        assert_eq!(run.model.active, SERVERS);
        assert!(!run.model.waiting.is_empty());
        assert!(run.model.rng_position > 0);
        if mode == "write" {
            let batch = ParallelIoRecordBatch::from_dispatched("service-queue", events).unwrap();
            let checkpoint =
                CheckpointManifest::from_batch("service-queue-cut", format, &batch).unwrap();
            let model = serde_json::to_vec(&run.model).unwrap();
            let receipt =
                write_engine_checkpoint(path, &checkpoint, &run.scheduler, SCHEMA, &model).unwrap();
            assert!(!receipt.contract_only);
        } else {
            assert_eq!(mode, "reference");
            println!("RESTART_RESULT={}", run.finish());
        }
    }
}

fn child(mode: &str, path: &Path, format: CheckpointFormat) -> String {
    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "checkpoint_process", "--nocapture"])
        .env("KAIRO_ENGINE_TEST_MODE", mode)
        .env("KAIRO_ENGINE_TEST_PATH", path)
        .env("KAIRO_ENGINE_TEST_FORMAT", format.as_str())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{mode}: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

fn round_trip(path: &Path, format: CheckpointFormat) {
    fn result(output: &str) -> &str {
        output
            .lines()
            .find_map(|line| line.strip_prefix("RESTART_RESULT="))
            .unwrap()
    }
    let reference = child("reference", path, format);
    child("write", path, format);
    let resumed = child("resume", path, format);
    assert_eq!(result(&reference), result(&resumed));
    assert_eq!(
        restore_engine_checkpoint(path, format, "service-queue.v2").unwrap_err(),
        ArrowError::InvalidArgument("engine model schema does not match")
    );
}

#[cfg(feature = "hdf5")]
#[test]
fn hdf5_restarts_the_complete_queue_and_rejects_corrupted_model_state() {
    use kairo_ecs_arrow::{CheckpointAdapter, Hdf5CheckpointAdapter};
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("service.h5");
    round_trip(&path, CheckpointFormat::Hdf5);
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
    assert!(matches!(
        Hdf5CheckpointAdapter.restore_manifest(&path),
        Err(ArrowError::UnexpectedStreamHeader)
    ));
    let saved = restore_engine_checkpoint(&path, CheckpointFormat::Hdf5, SCHEMA).unwrap();
    write_engine_checkpoint(
        &path,
        &saved.manifest,
        &saved.scheduler,
        SCHEMA,
        &saved.model_state,
    )
    .unwrap();
    let engine_bytes = std::fs::read(&path).unwrap();
    let mismatch = ArrowError::InvalidArgument("HDF5 checkpoint version would change");
    assert_eq!(
        Hdf5CheckpointAdapter.write_manifest(&path, &saved.manifest),
        Err(mismatch.clone())
    );
    assert_eq!(std::fs::read(&path).unwrap(), engine_bytes);
    let legacy_path = root.path().join("legacy.h5");
    Hdf5CheckpointAdapter
        .write_manifest(&legacy_path, &saved.manifest)
        .unwrap();
    let legacy_bytes = std::fs::read(&legacy_path).unwrap();
    assert_eq!(
        write_engine_checkpoint(
            &legacy_path,
            &saved.manifest,
            &saved.scheduler,
            SCHEMA,
            &saved.model_state,
        ),
        Err(mismatch)
    );
    assert_eq!(std::fs::read(&legacy_path).unwrap(), legacy_bytes);
    let file = hdf5_metno::File::open_rw(&path).unwrap();
    let dataset = file.dataset("engine_state").unwrap();
    let mut bytes = dataset.read_raw::<u8>().unwrap();
    *bytes.last_mut().unwrap() ^= 1;
    dataset.write_raw(&bytes).unwrap();
    drop(dataset);
    file.close().unwrap();
    assert_eq!(
        restore_engine_checkpoint(&path, CheckpointFormat::Hdf5, SCHEMA).unwrap_err(),
        ArrowError::InvalidNumber("checkpoint sha256")
    );
}

#[cfg(all(feature = "adios2", any(target_os = "linux", target_os = "macos")))]
#[test]
fn bp5_restarts_the_complete_queue_and_rejects_corrupted_model_state() {
    use kairo_ecs_arrow::{Adios2CheckpointAdapter, CheckpointAdapter};
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("service.bp");
    round_trip(&path, CheckpointFormat::Adios2);
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
    assert!(matches!(
        Adios2CheckpointAdapter.restore_manifest(&path),
        Err(ArrowError::UnexpectedStreamHeader)
    ));
    let native_data = path.join("data.0");
    let mut bytes = std::fs::read(&native_data).unwrap();
    let marker = b"\"rng_position\":";
    let index = bytes
        .windows(marker.len())
        .position(|part| part == marker)
        .unwrap();
    bytes[index] ^= 1;
    std::fs::write(native_data, bytes).unwrap();
    assert_eq!(
        restore_engine_checkpoint(&path, CheckpointFormat::Adios2, SCHEMA).unwrap_err(),
        ArrowError::InvalidNumber("checkpoint sha256")
    );
}
