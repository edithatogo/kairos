#![cfg(all(
    feature = "engine-checkpoint",
    any(
        feature = "hdf5",
        all(feature = "adios2", any(target_os = "linux", target_os = "macos"))
    )
))]

use std::num::NonZeroU128;
use std::path::Path;
use std::process::Command;

use kairo_ecs_arrow::{
    engine_checkpoint::{restore_engine_checkpoint, write_engine_checkpoint},
    CheckpointFormat, CheckpointManifest, ParallelIoRecordBatch,
};
use kairo_ecs_core::Scheduler;
use kairo_ecs_des::{StagedRequestId, StagedResourceId, StagedResourceRuntime, StagedUsageSource};
use kairo_ecs_rng::DeterministicStream;
use kairo_ecs_types::{DispatchedEvent, EntityId, EventKind, ScheduleRequest, SimTime};
use serde::{Deserialize, Serialize};

const SCHEMA: &str = "staged-service.v1";
const START: u128 = u64::MAX as u128 + 50;
const SEED: u64 = 20261124;

fn tick(offset: u128) -> SimTime {
    SimTime::from_ticks(START + offset)
}

#[derive(Deserialize, Serialize)]
struct Model {
    allocator_json: String,
    resource: (u64, u32),
    fifo_head: (u64, u32),
    rng_word_pos: u128,
}

struct Run {
    scheduler: Scheduler,
    allocator: StagedResourceRuntime,
    resource: StagedResourceId,
    fifo_head: StagedRequestId,
    rng: DeterministicStream,
}

impl Run {
    fn at_cut() -> (Self, DispatchedEvent) {
        let mut scheduler = Scheduler::new();
        let event = |offset, kind| ScheduleRequest {
            at: tick(offset),
            priority: 0,
            entity: Some(EntityId::new(42, 1)),
            kind: EventKind::custom(kind),
        };
        scheduler.schedule(event(2, 0));
        scheduler.schedule(event(5, 1));
        let superseded = scheduler.schedule(event(6, 99));
        assert!(scheduler.cancel(superseded));
        scheduler.schedule(event(12, 2));
        let cut = scheduler.step().into_dispatched().unwrap();

        let mut allocator = StagedResourceRuntime::new_with_usage(StagedUsageSource {
            run: "native-restart".into(),
            scenario: "staged-service".into(),
            branch: "reference".into(),
            engine_revision: "scheduler-v1".into(),
            model_revision: SCHEMA.into(),
            rng_revision: "deterministic-stream-v1".into(),
            time_unit: "tick".into(),
            ticks_per_time_unit: NonZeroU128::new(1).unwrap(),
        })
        .unwrap();
        let resource = allocator.create_resource_with_unit(3, "bed").unwrap();
        let actors: [_; 4] = std::array::from_fn(|_| allocator.spawn_actor().unwrap());
        let active = allocator
            .submit_quantity(resource, actors[0], tick(1), 2)
            .unwrap();
        let fifo_head = allocator
            .submit_quantity(resource, actors[1], tick(2), 2)
            .unwrap();
        let fifo_tail = allocator.submit(resource, actors[2], tick(2)).unwrap();
        let withdrawn = allocator.submit(resource, actors[3], tick(9)).unwrap();
        assert_eq!(allocator.run_for(3).unwrap(), 3);
        assert_eq!(allocator.now(), cut.at);
        assert_eq!(
            allocator.resource(resource).unwrap().queued,
            [fifo_head, fifo_tail]
        );
        assert_eq!(
            allocator
                .committed_usage(resource, tick(0), cut.at)
                .unwrap()
                .occupied_unit_ticks,
            2
        );
        allocator
            .release(allocator.request(active).unwrap().lease.unwrap(), tick(10))
            .unwrap();
        assert!(allocator.close_request(active, tick(5)).unwrap());
        assert!(allocator.close_request(withdrawn, tick(4)).unwrap());
        assert!(allocator.close_request(fifo_tail, tick(6)).unwrap());
        let mut rng = DeterministicStream::new(SEED);
        let _ = rng.next_u64();
        (
            Self {
                scheduler,
                allocator,
                resource,
                fifo_head,
                rng,
            },
            cut,
        )
    }

    fn model(&self) -> Model {
        let resource = self.resource.entity_id();
        let head = self.fifo_head.entity_id();
        Model {
            allocator_json: String::from_utf8(self.allocator.checkpoint_json().unwrap()).unwrap(),
            resource: (resource.index, resource.generation),
            fifo_head: (head.index, head.generation),
            rng_word_pos: self.rng.word_pos(),
        }
    }

    fn restore(path: &Path, format: CheckpointFormat) -> Self {
        let restored = restore_engine_checkpoint(path, format, SCHEMA).unwrap();
        assert_eq!(restored.manifest.checkpoint_id, "staged-service-cut");
        assert_eq!(restored.manifest.records.len(), 1);
        let model: Model = serde_json::from_slice(&restored.model_state).unwrap();
        let allocator =
            StagedResourceRuntime::from_checkpoint_json(model.allocator_json.as_bytes()).unwrap();
        let resource = allocator
            .resource_by_entity(EntityId::new(model.resource.0, model.resource.1))
            .unwrap();
        let fifo_head = allocator
            .request_by_entity(EntityId::new(model.fifo_head.0, model.fifo_head.1))
            .unwrap();
        let mut rng = DeterministicStream::new(SEED);
        rng.set_word_pos(model.rng_word_pos);
        assert_eq!(allocator.now(), restored.scheduler.now());
        Self {
            scheduler: restored.scheduler,
            allocator,
            resource,
            fifo_head,
            rng,
        }
    }

    fn finish(mut self) -> String {
        let first = self.scheduler.step().into_dispatched().unwrap();
        assert_eq!(
            (first.at, first.sequence, first.kind.code()),
            (tick(5), 1, 1)
        );
        assert_eq!(self.run_allocator_through(first.at), 2);
        let lease = self
            .allocator
            .request(self.fifo_head)
            .unwrap()
            .lease
            .unwrap();
        let duration = u128::from(2 + self.rng.next_u64() % 3);
        self.allocator.release(lease, tick(5 + duration)).unwrap();

        let last = self.scheduler.step().into_dispatched().unwrap();
        assert_eq!((last.at, last.sequence, last.kind.code()), (tick(12), 3, 2));
        assert_eq!(self.run_allocator_through(last.at), 2);
        assert!(self.scheduler.step().into_dispatched().is_none());
        let now = self.allocator.now();
        let usage = self
            .allocator
            .committed_usage(self.resource, tick(0), now)
            .unwrap();
        assert_eq!(usage.occupied_unit_ticks, 9 + 2 * duration);
        assert_eq!(
            usage
                .occupancy
                .iter()
                .map(|row| row.request.index)
                .collect::<Vec<_>>(),
            [0, 1, 2]
        );
        assert_eq!(
            usage.occupancy[1].admission,
            usage.occupancy[0].release.unwrap()
        );
        assert_eq!(self.allocator.resource(self.resource).unwrap().available, 3);

        let bounds = usage
            .occupancy
            .iter()
            .map(|row| {
                let release = row.release.unwrap();
                (
                    row.request.index,
                    row.quantity,
                    row.start.ticks(),
                    row.end.ticks(),
                    row.admission.event.index,
                    release.event.index,
                )
            })
            .collect::<Vec<_>>();
        serde_json::to_string(&(
            [first.sequence, last.sequence],
            self.scheduler.stats().cancelled_events,
            bounds,
            self.model(),
            self.rng.next_u64(),
        ))
        .unwrap()
    }

    fn run_allocator_through(&mut self, until: SimTime) -> u64 {
        let mut count = 0;
        while self
            .allocator
            .next_event_time()
            .is_some_and(|tick| tick <= until)
        {
            count += self.allocator.run_for(1).unwrap();
        }
        count
    }
}

#[test]
fn staged_checkpoint_process() {
    let Ok(role) = std::env::var("KAIRO_STAGED_RESTART_ROLE") else {
        return;
    };
    let destination = std::env::var_os("KAIRO_STAGED_RESTART_PATH").unwrap();
    let path = Path::new(&destination);
    let format = CheckpointFormat::try_from(
        std::env::var("KAIRO_STAGED_RESTART_FORMAT")
            .unwrap()
            .as_str(),
    )
    .unwrap();

    if role == "resume" {
        println!("STAGED_RESULT={}", Run::restore(path, format).finish());
        return;
    }
    let (run, cut) = Run::at_cut();
    if role == "reference" {
        println!("STAGED_RESULT={}", run.finish());
        return;
    }
    assert_eq!(role, "write");
    let records = ParallelIoRecordBatch::from_dispatched("staged-service", [cut]).unwrap();
    let manifest = CheckpointManifest::from_batch("staged-service-cut", format, &records).unwrap();
    let receipt = write_engine_checkpoint(
        path,
        &manifest,
        &run.scheduler,
        SCHEMA,
        &serde_json::to_vec(&run.model()).unwrap(),
    )
    .unwrap();
    assert!(!receipt.contract_only);
}

fn round_trip(format: CheckpointFormat) {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("staged");
    let run = |role| {
        let output = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "staged_checkpoint_process", "--nocapture"])
            .env("KAIRO_STAGED_RESTART_ROLE", role)
            .env("KAIRO_STAGED_RESTART_PATH", &path)
            .env("KAIRO_STAGED_RESTART_FORMAT", format.as_str())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{role}: {}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .find_map(|line| line.strip_prefix("STAGED_RESULT="))
            .map(str::to_owned)
    };
    let reference = run("reference").unwrap();
    assert!(run("write").is_none());
    assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 1);
    assert_eq!(run("resume").unwrap(), reference);
}

#[cfg(feature = "hdf5")]
#[test]
fn hdf5_restores_the_staged_allocator_in_a_fresh_process() {
    round_trip(CheckpointFormat::Hdf5);
}

#[cfg(all(feature = "adios2", any(target_os = "linux", target_os = "macos")))]
#[test]
fn bp5_restores_the_staged_allocator_in_a_fresh_process() {
    round_trip(CheckpointFormat::Adios2);
}
