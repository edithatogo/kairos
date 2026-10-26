use kairo_ecs_core::{ScheduledEventPreview, Scheduler};
use kairo_ecs_types::{EntityId, EventId, EventKind, ScheduleRequest, SimTime, StepOutcome};
use serde::Deserialize;

#[derive(Deserialize)]
struct PreviewEvent {
    at_ticks: u64,
    priority: i32,
    kind: u32,
    cancel: Option<bool>,
}

#[derive(Deserialize)]
struct ExpectedPreview {
    event_list_index: usize,
    at_ticks: u64,
    priority: i32,
    sequence: u64,
    kind: u32,
}

#[derive(Deserialize)]
struct PreviewFixture {
    fixture: String,
    version: u32,
    ordering: Vec<String>,
    events: Vec<PreviewEvent>,
    expected_preview: ExpectedPreview,
    expected_kind_order: Vec<u32>,
    requirement: String,
}

fn fixture() -> PreviewFixture {
    let text = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../conformance/fixtures/track106-scheduler-preview.json"
    ));
    serde_json::from_str(text).expect("preview conformance fixture is valid")
}

#[test]
fn peek_returns_the_next_live_event_without_changing_observable_state() {
    let fixture = fixture();
    assert_eq!(fixture.fixture, "scheduler_preview");
    assert_eq!(fixture.version, 1);
    assert_eq!(
        fixture.ordering,
        ["at_ticks ASC", "priority ASC", "sequence ASC"]
    );
    assert_eq!(
        fixture.requirement,
        "Repeated preview returns the same next live event without dispatch or observable scheduler-state change; the next step dispatches it."
    );

    let mut scheduler = Scheduler::new();
    let mut ids = Vec::<EventId>::new();
    for event in &fixture.events {
        let id = scheduler.schedule(ScheduleRequest {
            at: SimTime::from_ticks(event.at_ticks as u128),
            priority: event.priority,
            entity: Some(EntityId::new(17, 4)),
            kind: EventKind::custom(event.kind),
        });
        ids.push(id);
        if event.cancel.unwrap_or(false) {
            assert!(scheduler.cancel(id));
        }
    }

    let before_now = scheduler.now();
    let before_stats = scheduler.stats();
    let expected = &fixture.expected_preview;
    let expected_id = ids[expected.event_list_index];
    let expected_preview = ScheduledEventPreview {
        id: expected_id,
        at: SimTime::from_ticks(expected.at_ticks as u128),
        priority: expected.priority,
        sequence: expected.sequence,
        entity: Some(EntityId::new(17, 4)),
        kind: EventKind::custom(expected.kind),
    };

    assert_eq!(scheduler.peek_next(), Some(expected_preview));
    assert_eq!(scheduler.peek_next(), Some(expected_preview));
    assert_eq!(scheduler.now(), before_now);
    assert_eq!(scheduler.stats(), before_stats);
    assert_eq!(scheduler.pending_events(), 3);

    let StepOutcome::Dispatched(first) = scheduler.step() else {
        panic!("the previewed live event should dispatch next");
    };
    assert_eq!(first.id, expected_id);
    assert_eq!(first.at, expected_preview.at);
    assert_eq!(first.priority, expected_preview.priority);
    assert_eq!(first.sequence, expected_preview.sequence);
    assert_eq!(first.entity, expected_preview.entity);
    assert_eq!(first.kind, expected_preview.kind);

    let mut kinds = vec![first.kind.code()];
    while let StepOutcome::Dispatched(event) = scheduler.step() {
        kinds.push(event.kind.code());
    }
    assert_eq!(kinds, fixture.expected_kind_order);
}

#[test]
fn peek_on_an_empty_scheduler_returns_none_without_dispatch() {
    let mut scheduler = Scheduler::new();
    let before_now = scheduler.now();
    let before_stats = scheduler.stats();

    assert_eq!(scheduler.peek_next(), None);
    assert_eq!(scheduler.now(), before_now);
    assert_eq!(scheduler.stats(), before_stats);
}

#[test]
fn peek_keeps_live_events_cancellable_and_returns_none_when_only_cancelled_events_remain() {
    let mut scheduler = Scheduler::new();
    let cancelled = scheduler.schedule(ScheduleRequest {
        at: SimTime::from_ticks(1),
        priority: 0,
        entity: None,
        kind: EventKind::custom(1),
    });
    let live = scheduler.schedule(ScheduleRequest {
        at: SimTime::from_ticks(2),
        priority: 0,
        entity: None,
        kind: EventKind::custom(2),
    });
    assert!(scheduler.cancel(cancelled));

    let before_now = scheduler.now();
    let before_stats = scheduler.stats();
    let preview = scheduler.peek_next().expect("the live event is visible");
    assert_eq!(preview.id, live);
    assert_eq!(scheduler.now(), before_now);
    assert_eq!(scheduler.stats(), before_stats);
    assert_eq!(scheduler.pending_events(), 1);

    assert!(scheduler.cancel(live));
    assert_eq!(scheduler.peek_next(), None);
    assert_eq!(scheduler.pending_events(), 0);
}
