use kairo_ecs_core::Scheduler;
use kairo_ecs_rng::DeterministicStream;
use kairo_ecs_types::{EntityId, EventKind, ScheduleRequest, SimTime, StepOutcome};
use serde::Deserialize;

// ---------------------------------------------------------------------------
// Fixture deserialization
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct OrderingEvent {
    at_ticks: u64,
    priority: i32,
    sequence: u64,
    kind: u32,
}

#[derive(Deserialize)]
struct DeterministicOrderingFixture {
    fixture: String,
    version: u32,
    ordering: Vec<String>,
    events: Vec<OrderingEvent>,
    expected_kind_order: Vec<u32>,
}

#[derive(Deserialize)]
struct CancellationEvent {
    at_ticks: u64,
    priority: i32,
    kind: u32,
    cancel: Option<bool>,
}

#[derive(Deserialize)]
struct CancellationFixture {
    fixture: String,
    version: u32,
    events: Vec<CancellationEvent>,
    expected_kind_order: Vec<u32>,
}

#[derive(Deserialize)]
struct RngEntity {
    index: u64,
    generation: u32,
}

#[derive(Deserialize)]
struct RngReplayFixture {
    fixture: String,
    version: u32,
    run_seed: u64,
    entity: RngEntity,
    expected_stream: Vec<u32>,
}

#[derive(Deserialize)]
struct ZeroDelayGuardFixture {
    fixture: String,
    version: u32,
    ordering: Vec<String>,
    events: Vec<OrderingEvent>,
    expected_kind_order: Vec<u32>,
}

// ---------------------------------------------------------------------------
// Fixture loader
// ---------------------------------------------------------------------------

macro_rules! fixture {
    ($name:expr) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../conformance/fixtures/",
            $name
        ))
    };
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn conformance_deterministic_ordering() {
    let text = fixture!("deterministic_ordering.json");
    let f: DeterministicOrderingFixture = serde_json::from_str(text).unwrap();

    assert_eq!(f.fixture, "deterministic_ordering");
    assert_eq!(f.version, 1);

    let mut scheduler = Scheduler::new();
    for ev in &f.events {
        scheduler.schedule(ScheduleRequest {
            at: SimTime::from_ticks(ev.at_ticks as u128),
            priority: ev.priority,
            entity: None,
            kind: EventKind::Custom(ev.kind),
        });
    }

    let mut kinds = Vec::new();
    while let StepOutcome::Dispatched(d) = scheduler.step() {
        kinds.push(d.kind.code());
    }

    assert_eq!(kinds, f.expected_kind_order);
}

#[test]
fn conformance_cancellation() {
    let text = fixture!("cancellation.json");
    let f: CancellationFixture = serde_json::from_str(text).unwrap();

    assert_eq!(f.fixture, "cancellation");
    assert_eq!(f.version, 1);

    let mut scheduler = Scheduler::new();
    let mut cancel_ids = Vec::new();

    for ev in &f.events {
        let id = scheduler.schedule(ScheduleRequest {
            at: SimTime::from_ticks(ev.at_ticks as u128),
            priority: ev.priority,
            entity: None,
            kind: EventKind::Custom(ev.kind),
        });
        if ev.cancel.unwrap_or(false) {
            cancel_ids.push(id);
        }
    }

    for id in &cancel_ids {
        assert!(scheduler.cancel(*id));
    }

    let mut kinds = Vec::new();
    while let StepOutcome::Dispatched(d) = scheduler.step() {
        kinds.push(d.kind.code());
    }

    assert_eq!(kinds, f.expected_kind_order);
}

#[test]
fn conformance_rng_replay() {
    let text = fixture!("rng_replay.json");
    let f: RngReplayFixture = serde_json::from_str(text).unwrap();

    assert_eq!(f.fixture, "rng_replay");
    assert_eq!(f.version, 1);

    let entity = EntityId::new(f.entity.index, f.entity.generation);
    let mut stream = DeterministicStream::from_entity(f.run_seed, entity);

    for &expected in &f.expected_stream {
        assert_eq!(stream.next_u32(), expected);
    }
}

#[test]
fn conformance_zero_delay_guard() {
    let text = fixture!("zero_delay_guard.json");
    let f: ZeroDelayGuardFixture = serde_json::from_str(text).unwrap();

    assert_eq!(f.fixture, "zero_delay_guard");
    assert_eq!(f.version, 1);

    let mut scheduler = Scheduler::new();
    for ev in &f.events {
        scheduler.schedule(ScheduleRequest {
            at: SimTime::from_ticks(ev.at_ticks as u128),
            priority: ev.priority,
            entity: None,
            kind: EventKind::Custom(ev.kind),
        });
    }

    let mut kinds = Vec::new();
    while let StepOutcome::Dispatched(d) = scheduler.step() {
        kinds.push(d.kind.code());
    }

    assert_eq!(kinds, f.expected_kind_order);
}
