#![cfg(feature = "checkpoint-json")]

use std::{error::Error, fs, path::PathBuf, process::Command};

use kairo_ecs_core::{Scheduler, SchedulerCheckpointError};
use kairo_ecs_rng::{DeterministicStream, RngCheckpoint};
use kairo_ecs_types::{EntityId, EventId, EventKind, ScheduleRequest, SimTime};

const TICK: u128 = (1 << 100) + 7;
const ARRIVAL: u32 = 1;
const COMPLETION: u32 = 2;
const RESTART_DIR: &str = "KAIROS_SCHEDULER_TEST_RESTART_DIR";

fn request(at: u128, priority: i32, entity: Option<EntityId>, kind: u32) -> ScheduleRequest {
    ScheduleRequest {
        at: SimTime::from_ticks(at),
        priority,
        entity,
        kind: EventKind::custom(kind),
    }
}

fn finish_queue(
    mut scheduler: Scheduler,
    mut rng: DeterministicStream,
    mut completed_jobs: u64,
) -> String {
    assert_eq!(scheduler.now().ticks(), TICK);
    assert!(!scheduler.cancel(EventId::new(1, 1)));
    let extra = scheduler.schedule(request(TICK + 8, 0, None, ARRIVAL));
    assert_eq!(extra, EventId::new(6, 6));

    let mut trace = Vec::new();
    while let Some(event) = scheduler.step().into_dispatched() {
        if event.kind == EventKind::custom(ARRIVAL) {
            let service_ticks = u128::from(rng.next_u32() % 5) + 1;
            scheduler.schedule(request(
                event.at.ticks() + service_ticks,
                0,
                event.entity,
                COMPLETION,
            ));
        } else {
            completed_jobs += 1;
        }
        trace.push(event);
    }
    let replayed_sequences: Vec<_> = trace.iter().take(5).map(|event| event.sequence).collect();
    assert_eq!(replayed_sequences, [5, 2, 3, 4, 6]);
    assert_eq!(completed_jobs, 6);
    assert_eq!(scheduler.stats().pending_events, 0);
    assert_eq!(scheduler.stats().cancelled_events, 1);
    format!(
        "{:?}",
        (trace, scheduler.stats(), rng.checkpoint(), completed_jobs)
    )
}

#[test]
fn checkpoint_restarts_in_a_new_process() {
    if let Some(dir) = std::env::var_os(RESTART_DIR) {
        let dir = PathBuf::from(dir);
        let scheduler_bytes = fs::read(dir.join("scheduler.json")).unwrap();
        let scheduler = Scheduler::from_checkpoint_json(&scheduler_bytes).unwrap();
        assert_eq!(scheduler.checkpoint_json().unwrap(), scheduler_bytes);
        let (rng, completed_jobs): (RngCheckpoint, u64) =
            serde_json::from_slice(&fs::read(dir.join("model.json")).unwrap()).unwrap();
        let rng = DeterministicStream::restore(&rng).unwrap();
        fs::write(
            dir.join("resumed.txt"),
            finish_queue(scheduler, rng, completed_jobs),
        )
        .unwrap();
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    let mut rng = DeterministicStream::new(20261123);
    let mut scheduler = Scheduler::new();
    let prior = scheduler.schedule(request(TICK, 0, None, COMPLETION));
    assert_eq!(scheduler.step().into_dispatched().unwrap().id, prior);
    let _ = rng.next_u32();
    let completed_jobs = 1_u64;
    let cancelled = scheduler.schedule(request(TICK - 8, 0, None, ARRIVAL));
    let entity = Some(EntityId::new(u64::MAX, u32::MAX));
    scheduler.schedule(request(TICK + 8, i32::MIN, entity, ARRIVAL));
    scheduler.schedule(request(TICK + 8, 0, entity, ARRIVAL));
    scheduler.schedule(request(TICK + 8, 0, None, ARRIVAL));
    scheduler.schedule(request(TICK - 7, 0, None, COMPLETION));
    assert!(scheduler.cancel(cancelled));

    let checkpoint = scheduler.checkpoint_json().unwrap();
    assert_eq!(scheduler.peek_next().unwrap().sequence, 5);
    assert_eq!(scheduler.checkpoint_json().unwrap(), checkpoint);

    fs::write(dir.path().join("scheduler.json"), checkpoint).unwrap();
    fs::write(
        dir.path().join("model.json"),
        serde_json::to_vec(&(rng.checkpoint(), completed_jobs)).unwrap(),
    )
    .unwrap();
    let expected = finish_queue(scheduler, rng, completed_jobs);
    let child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "checkpoint_restarts_in_a_new_process",
            "--nocapture",
        ])
        .env(RESTART_DIR, dir.path())
        .output()
        .unwrap();
    assert!(
        child.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&child.stdout),
        String::from_utf8_lossy(&child.stderr)
    );
    assert_eq!(
        fs::read_to_string(dir.path().join("resumed.txt")).unwrap(),
        expected
    );
}

const EMPTY: &str = r#"{"version":1,"now_ticks":0,"scheduled_events":0,"dispatched_events":0,"cancelled_events":0,"pending":[]}"#;

#[test]
fn durable_v1_preserves_wrapped_generation_and_full_width_time() {
    assert_eq!(
        Scheduler::new().checkpoint_json().unwrap(),
        EMPTY.as_bytes()
    );
    let bytes = br#"{"version":1,"now_ticks":0,"scheduled_events":4294967296,"dispatched_events":4294967295,"cancelled_events":0,"pending":[{"sequence":4294967295,"at_ticks":340282366920938463463374607431768211455,"priority":2147483647,"entity":null,"kind":4294967295}]}"#;
    let mut restored = Scheduler::from_checkpoint_json(bytes).unwrap();
    assert_eq!(restored.checkpoint_json().unwrap(), bytes);
    assert_eq!(
        restored.schedule(request(u128::MAX, i32::MAX, None, u32::MAX)),
        EventId::new(1 << 32, 0)
    );
    let first = restored.step().into_dispatched().unwrap();
    assert_eq!(first.at.ticks(), u128::MAX);
    assert_eq!(first.id, EventId::new(u64::from(u32::MAX), u32::MAX));
    assert_eq!(
        restored.step().into_dispatched().unwrap().id,
        EventId::new(1 << 32, 0)
    );
}

#[test]
fn durable_v1_rejects_malformed_or_inconsistent_checkpoints() {
    for malformed in ["{", r#"{"version":1,"version":1}"#, r#"{"unknown":0}"#] {
        let error = Scheduler::from_checkpoint_json(malformed.as_bytes()).unwrap_err();
        assert!(matches!(error, SchedulerCheckpointError::Json(_)));
        assert!(error.source().is_some());
        assert!(error.to_string().contains("checkpoint JSON"));
    }
    let error =
        Scheduler::from_checkpoint_json(br#"{"version":2,"events_and_clock":{"ticks":"0"}}"#)
            .unwrap_err();
    assert!(matches!(
        error,
        SchedulerCheckpointError::UnsupportedVersion(2)
    ));
    assert!(error.source().is_none());
    assert!(error.to_string().contains("version 2"));

    let invalid_counts = EMPTY.replace(r#""cancelled_events":0"#, r#""cancelled_events":1"#);
    let error = Scheduler::from_checkpoint_json(invalid_counts.as_bytes()).unwrap_err();
    assert!(matches!(error, SchedulerCheckpointError::InvalidState(_)));
    assert!(error.to_string().contains("event counts"));

    let invalid_clock = EMPTY.replace(r#""now_ticks":0"#, r#""now_ticks":1"#);
    let error = Scheduler::from_checkpoint_json(invalid_clock.as_bytes()).unwrap_err();
    assert!(matches!(error, SchedulerCheckpointError::InvalidState(_)));
    assert!(error.to_string().contains("clock"));

    for (first, second) in [(0, 0), (1, 0), (0, 2)] {
        let bad = format!(
            r#"{{"version":1,"now_ticks":0,"scheduled_events":2,"dispatched_events":0,"cancelled_events":0,"pending":[{{"sequence":{first},"at_ticks":0,"priority":0,"entity":null,"kind":1}},{{"sequence":{second},"at_ticks":0,"priority":0,"entity":null,"kind":1}}]}}"#
        );
        let error = Scheduler::from_checkpoint_json(bad.as_bytes()).unwrap_err();
        assert!(matches!(error, SchedulerCheckpointError::InvalidState(_)));
        assert!(error.to_string().contains("pending sequences"));
    }
}

#[test]
fn durable_v1_reports_exhausted_schedule_capacity() {
    let all_dispatched = |count| {
        format!(
            r#"{{"version":1,"now_ticks":0,"scheduled_events":{count},"dispatched_events":{count},"cancelled_events":0,"pending":[]}}"#
        )
    };
    let mut scheduler =
        Scheduler::from_checkpoint_json(all_dispatched(u64::MAX - 1).as_bytes()).unwrap();
    let id = scheduler.schedule(request(0, 0, None, COMPLETION));
    assert_eq!(id, EventId::new(u64::MAX - 1, u32::MAX - 1));
    let error = scheduler.checkpoint_json().unwrap_err();
    assert!(matches!(error, SchedulerCheckpointError::InvalidState(_)));
    assert!(error.to_string().contains("counter is exhausted"));

    let error = Scheduler::from_checkpoint_json(all_dispatched(u64::MAX).as_bytes()).unwrap_err();
    assert!(matches!(error, SchedulerCheckpointError::InvalidState(_)));
    assert!(error.to_string().contains("counter is exhausted"));
}
