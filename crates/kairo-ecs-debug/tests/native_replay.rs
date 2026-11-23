use std::collections::BTreeMap;
use std::num::NonZeroUsize;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Arc;

use kairo_ecs_core::Scheduler;
use kairo_ecs_debug::{Debugger, EventTrace, ReplayError, ReplaySession};
use kairo_ecs_rng::{entity_stream, DeterministicStream};
use kairo_ecs_types::{DispatchedEvent, EntityId, EventId, EventKind, ScheduleRequest, SimTime};
use serde::Deserialize;

const WIDE_TICK: u128 = u64::MAX as u128 + 40;

#[derive(Deserialize)]
struct RngFixture {
    run_seed: u64,
    entity: FixtureEntity,
    expected_stream: [u32; 4],
}

#[derive(Deserialize)]
struct FixtureEntity {
    index: u64,
    generation: u32,
}

#[derive(Clone)]
struct Model {
    readings: Arc<BTreeMap<u32, u32>>,
    rng: DeterministicStream,
    cancel: EventId,
    spawned: Option<EventId>,
}

fn apply(model: &mut Model, scheduler: &mut Scheduler, event: DispatchedEvent) {
    let EventKind::Custom(kind) = event.kind;
    Arc::make_mut(&mut model.readings).insert(kind, model.rng.next_u32());
    if kind == 1 {
        assert!(scheduler.cancel(model.cancel));
        model.spawned = Some(scheduler.schedule(ScheduleRequest {
            at: event.at,
            priority: -4,
            entity: event.entity,
            kind: EventKind::custom(4),
        }));
    }
}

fn start(interval: usize, limit: usize) -> (ReplaySession<Model>, EntityId, [u32; 4]) {
    // The existing cross-binding RNG oracle supplies expected draws independent of replay.
    let fixture: RngFixture = serde_json::from_str(include_str!(
        "../../../conformance/fixtures/rng_replay.json"
    ))
    .unwrap();
    let entity = EntityId {
        index: fixture.entity.index,
        generation: fixture.entity.generation,
    };
    let mut scheduler = Scheduler::new();
    let cancel = scheduler.schedule(request(WIDE_TICK, -1, entity, 9));
    scheduler.schedule(request(0, -2, entity, 1));
    scheduler.schedule(request(0, 1, entity, 2));
    scheduler.schedule(request(WIDE_TICK, 1, entity, 3));
    let replay = ReplaySession::new(
        scheduler,
        Model {
            readings: Arc::default(),
            rng: entity_stream(fixture.run_seed, entity),
            cancel,
            spawned: None,
        },
        apply,
        NonZeroUsize::new(interval).unwrap(),
        NonZeroUsize::new(limit).unwrap(),
    );
    (replay, entity, fixture.expected_stream)
}

fn request(tick: u128, priority: i32, entity: EntityId, kind: u32) -> ScheduleRequest {
    ScheduleRequest {
        at: SimTime::from_ticks(tick),
        priority,
        entity: Some(entity),
        kind: EventKind::custom(kind),
    }
}

fn expected_event(
    id: u32,
    tick: u128,
    priority: i32,
    entity: EntityId,
    kind: u32,
) -> DispatchedEvent {
    DispatchedEvent {
        id: EventId {
            index: u64::from(id),
            generation: id,
        },
        at: SimTime::from_ticks(tick),
        priority,
        sequence: u64::from(id),
        entity: Some(entity),
        kind: EventKind::custom(kind),
    }
}

#[test]
fn restores_same_tick_transitions_and_replays_the_exact_scheduler_and_rng_suffix() {
    let (mut session, entity, draws) = start(3, 2);
    let expected = [
        expected_event(1, 0, -2, entity, 1),
        expected_event(4, 0, -4, entity, 4),
        expected_event(2, 0, 1, entity, 2),
        expected_event(3, WIDE_TICK, 1, entity, 3),
    ];
    let mut readings = BTreeMap::new();
    let mut stats = Vec::new();
    for (event, draw) in expected.into_iter().zip(draws) {
        assert_eq!(session.step().unwrap(), Some(event));
        let EventKind::Custom(kind) = event.kind;
        readings.insert(kind, draw);
        assert_eq!(*session.model().readings, readings);
        stats.push(session.scheduler().stats());
    }
    assert_eq!(session.step(), Ok(None));
    assert_eq!(session.model().rng.word_pos(), 4);
    assert_eq!(session.scheduler().stats().cancelled_events, 1);
    assert_eq!(session.retained_positions(), 0..=4);

    assert_eq!(session.seek_tick(SimTime::ZERO), Ok(Some(expected[0])));
    assert_eq!(*session.model().readings, BTreeMap::from([(1, draws[0])]));
    assert_eq!(session.scheduler().stats(), stats[0]);
    session.seek(0).unwrap();
    assert_eq!(session.current_event(), None);
    assert!(session.model().readings.is_empty());
    assert_eq!(session.model().rng.word_pos(), 0);
    for event in expected {
        assert_eq!(session.step(), Ok(Some(event)));
    }
    assert_eq!(*session.model().readings, readings);
    assert_eq!(session.model().rng.word_pos(), 4);
    assert_eq!(session.scheduler().stats(), stats[3]);

    session.seek(2).unwrap();
    assert_eq!(session.current_event(), Some(expected[1]));
    assert_eq!(session.scheduler().stats(), stats[1]);
    assert_eq!(
        session.seek_tick(SimTime::from_ticks(WIDE_TICK)),
        Ok(Some(expected[3]))
    );
    assert_eq!(*session.model().readings, readings);
    assert_eq!(session.scheduler().stats(), stats[3]);
}

#[test]
fn fork_has_an_independent_queue_state_and_rng_with_bounded_history() {
    let (mut parent, entity, draws) = start(2, 1);
    let first = parent.step().unwrap();
    let mut branch = parent.fork(|model, scheduler| {
        assert!(scheduler.cancel(model.spawned.unwrap()));
        assert_eq!(model.rng.next_u32(), draws[1]);
        scheduler.schedule(request(0, -4, entity, 5));
    });
    assert!(Arc::ptr_eq(
        &parent.model().readings,
        &branch.model().readings
    ));
    assert_eq!(branch.current_event(), first);
    let branch_event = expected_event(5, 0, -4, entity, 5);
    assert_eq!(branch.step(), Ok(Some(branch_event)));
    assert!(!Arc::ptr_eq(
        &parent.model().readings,
        &branch.model().readings
    ));
    assert_eq!(branch.model().readings.get(&5), Some(&draws[2]));
    assert_eq!(*parent.model().readings, BTreeMap::from([(1, draws[0])]));
    assert_eq!(parent.scheduler().stats().cancelled_events, 1);
    assert_eq!(branch.scheduler().stats().cancelled_events, 2);
    assert_eq!(parent.position(), 1);
    assert_eq!(parent.model().rng.word_pos(), 1);
    branch.seek(1).unwrap();
    assert_eq!(branch.model().rng.word_pos(), 2);
    assert_eq!(branch.step(), Ok(Some(branch_event)));
    assert_eq!(branch.model().readings.get(&5), Some(&draws[2]));

    assert_eq!(parent.step(), Ok(Some(expected_event(4, 0, -4, entity, 4))));
    assert_eq!(parent.model().readings.get(&4), Some(&draws[1]));
    assert_eq!(parent.retained_positions(), 2..=2);
    assert_eq!(
        parent.seek(1),
        Err(ReplayError::PositionUnavailable {
            position: 1,
            oldest: 2,
            newest: 2,
        })
    );
    assert_eq!(parent.position(), 2);
    assert_eq!(parent.model().rng.word_pos(), 2);
    assert_eq!(branch.retained_positions(), 1..=2);
}

#[test]
fn panicking_transition_requires_restore_before_dispatch_resumes() {
    let mut scheduler = Scheduler::new();
    let entity = EntityId {
        index: 42,
        generation: 0,
    };
    scheduler.schedule(request(0, 0, entity, 1));
    scheduler.schedule(request(1, 0, entity, 2));
    let mut session = ReplaySession::new(
        scheduler,
        0,
        |model, scheduler, event| {
            *model += 1;
            if event.kind == EventKind::custom(2) {
                scheduler.schedule(request(2, 0, event.entity.unwrap(), 3));
                panic!("transition interrupted after changing model and queue");
            }
        },
        NonZeroUsize::new(2).unwrap(),
        NonZeroUsize::new(2).unwrap(),
    );
    let first = session.step().unwrap();
    let stats = session.scheduler().stats();
    assert!(catch_unwind(AssertUnwindSafe(|| session.step())).is_err());
    assert_eq!(session.current_event(), first);
    assert_eq!(session.step(), Err(ReplayError::Interrupted));
    assert!(catch_unwind(AssertUnwindSafe(|| session.model())).is_err());
    assert!(catch_unwind(AssertUnwindSafe(|| session.scheduler())).is_err());
    session.seek(1).unwrap();
    assert_eq!(session.current_event(), first);
    assert_eq!(*session.model(), 1);
    assert_eq!(session.scheduler().stats(), stats);
    let mut branch = session.fork(|_, scheduler| {
        assert!(scheduler.cancel(EventId {
            index: 1,
            generation: 1,
        }));
    });
    assert_eq!(branch.step(), Ok(None));
    assert_eq!(session.scheduler().pending_events(), 1);
}

#[test]
fn trace_playback_applies_tick_zero_and_does_not_show_later_same_tick_state() {
    let mut scheduler = Scheduler::new();
    let entity = EntityId {
        index: 42,
        generation: 0,
    };
    scheduler.schedule(request(0, 0, entity, 1));
    scheduler.schedule(request(0, 0, entity, 2));
    let mut trace = EventTrace::default();
    let first = scheduler.step().into_dispatched().unwrap();
    trace.record_event(first, BTreeMap::from([("phase".into(), "first".into())]));
    assert_eq!(trace.reconstruct_at(0)["phase"], "first");

    let second = scheduler.step().into_dispatched().unwrap();
    trace.record_event(second, BTreeMap::from([("phase".into(), "second".into())]));
    trace.snapshot(
        SimTime::ZERO,
        BTreeMap::from([("phase".into(), "after".into())]),
    );
    assert_eq!(trace.reconstruct_at(0)["phase"], "after");
    let mut debugger = Debugger::new(trace);
    assert_eq!(debugger.inspect("phase"), None);
    assert_eq!(debugger.step().unwrap().event_id, first.id);
    assert_eq!(debugger.inspect("phase").as_deref(), Some("first"));
    assert_eq!(debugger.step().unwrap().event_id, second.id);
    assert_eq!(debugger.inspect("phase").as_deref(), Some("after"));
    assert_eq!(debugger.back().unwrap().event_id, first.id);
    assert_eq!(debugger.inspect("phase").as_deref(), Some("first"));
}
