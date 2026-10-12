use kairo_ecs_core::checkpoint::{
    SchedulerCheckpointEntry, SchedulerCheckpointError, SchedulerCheckpointLimits,
    SchedulerCheckpointV1,
};
use kairo_ecs_core::Scheduler;
use kairo_ecs_types::{
    DispatchedEvent, EntityId, EventId, EventKind, ScheduleRequest, SimTime, StepOutcome,
};

fn request(at: u128, priority: i32, entity: Option<EntityId>, kind: u32) -> ScheduleRequest {
    ScheduleRequest {
        at: SimTime::from_ticks(at),
        priority,
        entity,
        kind: EventKind::custom(kind),
    }
}

fn limits(max_entries: usize) -> SchedulerCheckpointLimits {
    SchedulerCheckpointLimits { max_entries }
}

fn round_trip(scheduler: &Scheduler, max_entries: usize) -> Scheduler {
    let dto = scheduler.checkpoint_state(limits(max_entries)).unwrap();
    Scheduler::from_checkpoint_state(dto, limits(max_entries)).unwrap()
}

fn dispatched(
    id: EventId,
    at: u128,
    priority: i32,
    sequence: u64,
    entity: Option<EntityId>,
    kind: u32,
) -> StepOutcome {
    StepOutcome::Dispatched(DispatchedEvent {
        id,
        at: SimTime::from_ticks(at),
        priority,
        sequence,
        entity,
        kind: EventKind::custom(kind),
    })
}

#[test]
fn preserves_physical_tombstones_and_original_queue_continuation() {
    let mut original = Scheduler::new();
    let first = request(4, 0, None, 10);
    let cancelled = request(4, -1, None, 11);
    let tied = request(4, 0, Some(EntityId::new(7, 2)), 12);
    let late = request(9, i32::MIN, None, 13);
    let first_id = original.schedule(first);
    let cancelled_id = original.schedule(cancelled);
    let tied_id = original.schedule(tied);
    original.schedule(late);
    assert!(original.cancel(cancelled_id));

    let before = original.stats();
    let checkpoint = original.checkpoint_state(limits(4)).unwrap();
    assert_eq!(
        original.stats(),
        before,
        "export must not dispatch, prune, or alter counters"
    );
    assert_eq!(
        checkpoint.entries.len(),
        4,
        "cancelled heap entry is retained physically"
    );
    assert!(checkpoint
        .entries
        .iter()
        .any(|entry| !entry.live && entry.id == cancelled_id));

    // Move the detached DTO through an independent reconstruction boundary.
    let restored = Scheduler::from_checkpoint_state(checkpoint.clone(), limits(4)).unwrap();
    let mut control = original;
    let mut resumed = restored;
    for scheduler in [&mut control, &mut resumed] {
        assert_eq!(scheduler.step(), dispatched(first_id, 4, 0, 0, None, 10));
        assert_eq!(
            scheduler.step(),
            dispatched(tied_id, 4, 0, 2, tied.entity, 12)
        );
        assert_eq!(
            scheduler.step(),
            dispatched(late_id(&checkpoint), 9, i32::MIN, 3, None, 13)
        );
        assert_eq!(scheduler.step(), StepOutcome::Empty);
    }
    assert_eq!(control.stats(), resumed.stats());

    let expected_next = EventId::new(4, 4);
    let next_request = request(1, 0, None, 14); // Earlier than now is legal in Scheduler.
    assert_eq!(control.schedule(next_request), expected_next);
    assert_eq!(resumed.schedule(next_request), expected_next);
    assert_eq!(control.step(), dispatched(expected_next, 1, 0, 4, None, 14));
    assert_eq!(resumed.step(), dispatched(expected_next, 1, 0, 4, None, 14));
}

fn late_id(checkpoint: &SchedulerCheckpointV1) -> EventId {
    checkpoint
        .entries
        .iter()
        .find(|entry| entry.request.kind == EventKind::custom(13))
        .unwrap()
        .id
}

#[test]
fn prune_frontier_round_trips_after_dispatch_and_preserves_tie_sequence() {
    let mut original = Scheduler::new();
    let ids: Vec<_> = [
        request(2, 5, None, 20),
        request(3, 0, None, 21),
        request(3, 0, None, 22),
        request(5, 0, None, 23),
    ]
    .into_iter()
    .map(|event| original.schedule(event))
    .collect();
    assert!(matches!(original.step(), StepOutcome::Dispatched(_)));
    assert!(original.cancel(ids[1]));
    assert_eq!(original.peek_next().unwrap().id, ids[2]); // Prunes tombstone.

    let mut resumed = round_trip(&original, 3);
    let original_next = original.schedule(request(3, 0, None, 24));
    let resumed_next = resumed.schedule(request(3, 0, None, 24));
    assert_eq!(original_next, resumed_next);
    loop {
        let left = original.step();
        let right = resumed.step();
        assert_eq!(left, right);
        if left == StepOutcome::Empty {
            break;
        }
    }
}

#[test]
fn rejects_invalid_checkpoint_invariants_with_exact_errors() {
    let mut source = Scheduler::new();
    source.schedule(request(8, 0, None, 30));
    source.schedule(request(9, 0, None, 31));
    let dto = source.checkpoint_state(limits(2)).unwrap();
    let source_stats = source.stats();

    assert_eq!(
        source.checkpoint_state(limits(1)),
        Err(SchedulerCheckpointError::LimitExceeded)
    );
    assert_eq!(source.stats(), source_stats);

    let mut unsupported_version = dto.clone();
    unsupported_version.schema_version = 2;
    let mut next_index_mismatch = dto.clone();
    next_index_mismatch.next_event_index += 1;
    let mut next_sequence_mismatch = dto.clone();
    next_sequence_mismatch.next_sequence += 1;
    let mut next_generation_mismatch = dto.clone();
    next_generation_mismatch.next_event_generation += 1;
    let mut event_index_sequence_mismatch = dto.clone();
    event_index_sequence_mismatch.entries[0].id.index += 1;
    let mut event_generation_mismatch = dto.clone();
    event_generation_mismatch.entries[0].id.generation += 1;
    let mut duplicate_event_id = dto.clone();
    duplicate_event_id.entries[1].id = duplicate_event_id.entries[0].id;
    let mut duplicate_sequence = dto.clone();
    duplicate_sequence.entries[1].sequence = duplicate_sequence.entries[0].sequence;
    let mut entry_at_next_index = dto.clone();
    entry_at_next_index.entries[1].id = EventId::new(2, 2);
    entry_at_next_index.entries[1].sequence = 2;
    let mut counter_overflow = dto.clone();
    counter_overflow.dispatched_events = u64::MAX;
    counter_overflow.cancelled_events = 1;
    let mut counter_mismatch = dto.clone();
    counter_mismatch.dispatched_events = 1;
    let mut live_accounting_mismatch = dto.clone();
    live_accounting_mismatch.entries[0].live = false;
    let mut tombstones_exceed_cancelled = dto.clone();
    tombstones_exceed_cancelled.now = SimTime::from_ticks(8);
    tombstones_exceed_cancelled.dispatched_events = 1;
    tombstones_exceed_cancelled.entries[0].live = false;
    let mut invalid_clock = dto.clone();
    invalid_clock.now = SimTime::from_ticks(1);

    let invalid_cases = [
        (
            "version",
            unsupported_version,
            SchedulerCheckpointError::UnsupportedVersion(2),
        ),
        (
            "next event index",
            next_index_mismatch,
            SchedulerCheckpointError::InvalidState,
        ),
        (
            "next sequence",
            next_sequence_mismatch,
            SchedulerCheckpointError::InvalidState,
        ),
        (
            "next generation",
            next_generation_mismatch,
            SchedulerCheckpointError::InvalidState,
        ),
        (
            "event index/sequence",
            event_index_sequence_mismatch,
            SchedulerCheckpointError::InvalidState,
        ),
        (
            "event generation",
            event_generation_mismatch,
            SchedulerCheckpointError::InvalidState,
        ),
        (
            "duplicate event identity",
            duplicate_event_id,
            SchedulerCheckpointError::InvalidState,
        ),
        (
            "duplicate sequence",
            duplicate_sequence,
            SchedulerCheckpointError::InvalidState,
        ),
        (
            "event at next index",
            entry_at_next_index,
            SchedulerCheckpointError::InvalidState,
        ),
        (
            "checked counter overflow",
            counter_overflow,
            SchedulerCheckpointError::InvalidState,
        ),
        (
            "counter mismatch",
            counter_mismatch,
            SchedulerCheckpointError::InvalidState,
        ),
        (
            "live accounting",
            live_accounting_mismatch,
            SchedulerCheckpointError::InvalidState,
        ),
        (
            "tombstone count",
            tombstones_exceed_cancelled,
            SchedulerCheckpointError::InvalidState,
        ),
        (
            "clock without dispatch",
            invalid_clock,
            SchedulerCheckpointError::InvalidState,
        ),
    ];
    for (name, bad, expected) in invalid_cases {
        assert_eq!(
            Scheduler::from_checkpoint_state(bad, limits(2)).unwrap_err(),
            expected,
            "unexpected checkpoint validation result for {name}"
        );
    }

    assert_eq!(
        Scheduler::from_checkpoint_state(dto.clone(), limits(1)).unwrap_err(),
        SchedulerCheckpointError::LimitExceeded
    );
    let mut reordered = dto.clone();
    reordered.entries.swap(0, 1);
    assert_eq!(
        Scheduler::from_checkpoint_state(reordered, limits(2)).unwrap_err(),
        SchedulerCheckpointError::InvalidState
    );
    assert_eq!(
        source.stats(),
        source_stats,
        "failed detached imports cannot mutate source"
    );
}

#[test]
fn near_wrap_generation_frontier_keeps_next_identifier_and_order() {
    let base = (1_u64 << 32) - 1;
    let state = SchedulerCheckpointV1 {
        schema_version: 1,
        now: SimTime::from_ticks(12),
        next_event_index: base + 2,
        next_event_generation: 1,
        next_sequence: base + 2,
        scheduled_events: base + 2,
        dispatched_events: base,
        cancelled_events: 0,
        entries: vec![
            SchedulerCheckpointEntry {
                request: request(11, -1, None, 40),
                id: EventId::new(base, u32::MAX),
                sequence: base,
                live: true,
            },
            SchedulerCheckpointEntry {
                request: request(11, -1, None, 41),
                id: EventId::new(base + 1, 0),
                sequence: base + 1,
                live: true,
            },
        ],
    };
    let mut restored = Scheduler::from_checkpoint_state(state, limits(2)).unwrap();
    let next = EventId::new(base + 2, 1);
    assert_eq!(restored.schedule(request(10, 0, None, 42)), next);
    assert_eq!(restored.step(), dispatched(next, 10, 0, base + 2, None, 42));
    assert_eq!(
        restored.step(),
        dispatched(EventId::new(base, u32::MAX), 11, -1, base, None, 40)
    );
    assert_eq!(
        restored.step(),
        dispatched(EventId::new(base + 1, 0), 11, -1, base + 1, None, 41)
    );
}

#[test]
fn empty_scheduler_checkpoint_roundtrips_and_continues() {
    let original = Scheduler::new();
    let state = original.checkpoint_state(limits(0)).unwrap();
    let mut restored = Scheduler::from_checkpoint_state(state, limits(0)).unwrap();
    assert_eq!(restored.stats(), original.stats());
    let next = EventId::new(0, 0);
    assert_eq!(restored.schedule(request(3, 0, None, 50)), next);
    assert_eq!(restored.step(), dispatched(next, 3, 0, 0, None, 50));
}
