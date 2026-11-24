#![cfg(all(
    feature = "engine-checkpoint",
    any(
        feature = "hdf5",
        all(feature = "adios2", any(target_os = "linux", target_os = "macos"))
    )
))]

use std::{path::Path, process::Command};

use kairo_ecs_arrow::{
    engine_checkpoint::{restore_engine_checkpoint, write_engine_checkpoint},
    CheckpointFormat, CheckpointManifest, ParallelIoRecordBatch,
};
use kairo_ecs_core::Scheduler;
use kairo_ecs_rng::{DeterministicStream, RngCheckpoint};
use kairo_ecs_state::{ComponentStore, World};
use kairo_ecs_types::{DispatchedEvent, EntityId, EventKind, ScheduleRequest, SimTime};
use serde::{Deserialize, Serialize};

const SCHEMA: &str = "spatial-agents.v1";
const START: u128 = u64::MAX as u128 + 50;
const MOVE: u32 = 1;
const SPAWN: u32 = 2;
const RETIRE: u32 = 3;

#[derive(Serialize, Deserialize)]
struct Model {
    world_json: String,
    positions: Vec<((u64, u32), i64)>,
    rng: RngCheckpoint,
}

struct Run {
    scheduler: Scheduler,
    world: World,
    positions: ComponentStore<i64>,
    rng: DeterministicStream,
}

impl Run {
    fn at_cut() -> (Self, DispatchedEvent) {
        let mut world = World::new();
        let agents: [_; 6] = std::array::from_fn(|_| world.spawn());
        assert!(world.despawn(agents[4]));
        assert!(world.despawn(agents[1]));
        let replacement = world.spawn();
        assert_eq!(replacement, EntityId::new(1, 1));
        assert!(world.despawn(agents[3]));
        let mut positions = ComponentStore::new();
        for agent in [agents[0], replacement, agents[2], agents[5]] {
            assert!(positions.insert(agent, agent.index as i64 * 10));
        }

        let mut scheduler = Scheduler::new();
        for (offset, priority, entity, kind) in [
            (2, 0, Some(replacement), MOVE),
            (4, 0, Some(agents[1]), MOVE),
            (4, -1, None, SPAWN),
            (6, 0, Some(replacement), MOVE),
            (8, 0, None, SPAWN),
            (9, 0, Some(agents[0]), RETIRE),
            (10, 0, None, SPAWN),
        ] {
            scheduler.schedule(ScheduleRequest {
                at: SimTime::from_ticks(START + offset),
                priority,
                entity,
                kind: EventKind::custom(kind),
            });
        }
        let cancelled = scheduler.schedule(ScheduleRequest {
            at: SimTime::from_ticks(START + 11),
            priority: 0,
            entity: None,
            kind: EventKind::custom(SPAWN),
        });
        assert!(scheduler.cancel(cancelled));
        let cut = scheduler.step().into_dispatched().unwrap();
        let mut run = Self {
            scheduler,
            world,
            positions,
            rng: DeterministicStream::new(20261124),
        };
        assert_eq!(run.apply(cut), Some(replacement));
        (run, cut)
    }

    fn apply(&mut self, event: DispatchedEvent) -> Option<EntityId> {
        match event.kind.code() {
            MOVE => {
                let actor = event.entity.unwrap();
                if !self.world.is_alive(actor) {
                    return None;
                }
                *self.positions.get_mut(actor).unwrap() += 1 + (self.rng.next_u64() % 7) as i64;
                Some(actor)
            }
            SPAWN => {
                let actor = self.world.spawn();
                assert!(self
                    .positions
                    .insert(actor, (self.rng.next_u64() % 11) as i64));
                Some(actor)
            }
            RETIRE => {
                let actor = event.entity.unwrap();
                assert!(self.world.despawn(actor));
                assert!(self.positions.remove(actor).is_some());
                Some(actor)
            }
            other => panic!("unexpected event kind {other}"),
        }
    }

    fn model(&self) -> Model {
        Model {
            world_json: String::from_utf8(self.world.checkpoint_json().unwrap()).unwrap(),
            positions: self
                .world
                .snapshot()
                .entities()
                .iter()
                .map(|row| {
                    (
                        (row.id.index, row.id.generation),
                        *self.positions.get(row.id).unwrap(),
                    )
                })
                .collect(),
            rng: self.rng.checkpoint(),
        }
    }

    fn restore(path: &Path, format: CheckpointFormat) -> Self {
        let restored = restore_engine_checkpoint(path, format, SCHEMA).unwrap();
        assert_eq!(restored.scheduler.now().ticks(), START + 2);
        assert_eq!(restored.manifest.checkpoint_id, "spatial-agents-cut");
        assert_eq!(restored.manifest.records.len(), 1);
        let model: Model = serde_json::from_slice(&restored.model_state).unwrap();
        let world = World::from_checkpoint_json(model.world_json.as_bytes()).unwrap();
        let mut positions = ComponentStore::new();
        assert_eq!(world.len(), model.positions.len());
        for (row, (id, position)) in world.snapshot().entities().iter().zip(model.positions) {
            assert_eq!((row.id.index, row.id.generation), id);
            assert!(positions.insert(row.id, position));
        }
        Self {
            scheduler: restored.scheduler,
            world,
            positions,
            rng: DeterministicStream::restore(&model.rng).unwrap(),
        }
    }

    fn finish(mut self) -> String {
        let mut updates = Vec::new();
        while let Some(event) = self.scheduler.step().into_dispatched() {
            let actor = self.apply(event).map(|id| (id.index, id.generation));
            updates.push((event.sequence, event.at.ticks(), event.priority, actor));
        }
        assert_eq!(
            updates,
            [
                (2, START + 4, -1, Some((3, 1))),
                (1, START + 4, 0, None),
                (3, START + 6, 0, Some((1, 1))),
                (4, START + 8, 0, Some((4, 1))),
                (5, START + 9, 0, Some((0, 0))),
                (6, START + 10, 0, Some((0, 1))),
            ]
        );
        assert_eq!(self.scheduler.stats().cancelled_events, 1);
        assert_eq!(self.rng.word_pos(), 10);
        serde_json::to_string(&(updates, self.model(), self.rng.next_u64())).unwrap()
    }
}

#[test]
fn world_checkpoint_process() {
    let Ok(role) = std::env::var("KAIRO_WORLD_RESTART_ROLE") else {
        return;
    };
    let destination = std::env::var_os("KAIRO_WORLD_RESTART_PATH").unwrap();
    let path = Path::new(&destination);
    let format = CheckpointFormat::try_from(
        std::env::var("KAIRO_WORLD_RESTART_FORMAT")
            .unwrap()
            .as_str(),
    )
    .unwrap();

    if role == "resume" {
        println!("WORLD_RESULT={}", Run::restore(path, format).finish());
        return;
    }
    let (run, cut) = Run::at_cut();
    if role == "reference" {
        println!("WORLD_RESULT={}", run.finish());
        return;
    }
    assert_eq!(role, "write");
    let records = ParallelIoRecordBatch::from_dispatched("spatial-agents", [cut]).unwrap();
    let manifest = CheckpointManifest::from_batch("spatial-agents-cut", format, &records).unwrap();
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
    let path = temp.path().join("world");
    let run = |role| {
        let output = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "world_checkpoint_process", "--nocapture"])
            .env("KAIRO_WORLD_RESTART_ROLE", role)
            .env("KAIRO_WORLD_RESTART_PATH", &path)
            .env("KAIRO_WORLD_RESTART_FORMAT", format.as_str())
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
            .find_map(|line| line.strip_prefix("WORLD_RESULT="))
            .map(str::to_owned)
    };
    let reference = run("reference").unwrap();
    assert!(run("write").is_none());
    assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 1);
    assert_eq!(run("resume").unwrap(), reference);
}

#[cfg(feature = "hdf5")]
#[test]
fn hdf5_restores_the_ecs_world_in_a_fresh_process() {
    round_trip(CheckpointFormat::Hdf5);
}

#[cfg(all(feature = "adios2", any(target_os = "linux", target_os = "macos")))]
#[test]
fn bp5_restores_the_ecs_world_in_a_fresh_process() {
    round_trip(CheckpointFormat::Adios2);
}
