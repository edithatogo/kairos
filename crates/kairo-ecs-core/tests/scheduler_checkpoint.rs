use kairo_ecs_core::{Scheduler, SCHEDULER_CHECKPOINT_VERSION_V1};
use kairo_ecs_types::{EntityId, EventKind, ScheduleRequest, SimTime, StepOutcome, TypesError};

fn event(tick: u128, priority: i32, kind: u32) -> ScheduleRequest {
    ScheduleRequest {
        at: SimTime::from_ticks(tick),
        priority,
        entity: Some(EntityId::new(3, 5)),
        kind: EventKind::custom(kind),
    }
}

#[test]
fn checkpoint_preserves_pending_order_cancellation_and_new_event_identity() {
    let mut uninterrupted = Scheduler::new();
    let prior = uninterrupted.schedule(event(10, 0, 1));
    assert_eq!(uninterrupted.step().into_dispatched().unwrap().id, prior);
    let cancelled = uninterrupted.schedule(event(11, -2, 2));
    let retired = uninterrupted.schedule(event(12, -2, 3));
    let first = uninterrupted.schedule(event(12, 0, 4));
    let second = uninterrupted.schedule(event(12, 0, 5));
    assert!(uninterrupted.cancel(cancelled));

    let checkpoint = uninterrupted.checkpoint();
    assert_eq!(checkpoint.version, SCHEDULER_CHECKPOINT_VERSION_V1);
    let baseline = uninterrupted.stats();
    assert_eq!(baseline.now, SimTime::from_ticks(10));
    assert_eq!(baseline.pending_events, 3);
    assert_eq!(baseline.cancelled_events, 1);

    let mut restored = Scheduler::from_checkpoint(&checkpoint).unwrap();
    let mut sibling = Scheduler::from_checkpoint(&checkpoint).unwrap();
    assert_eq!(restored.stats(), baseline);
    assert_eq!(sibling.stats(), baseline);
    assert!(!restored.cancel(cancelled));
    assert!(sibling.cancel(retired));
    assert_eq!(restored.stats(), baseline);
    assert_eq!(uninterrupted.stats(), baseline);

    let request = event(12, 0, 6);
    let next = uninterrupted.schedule(request);
    assert_eq!(restored.schedule(request), next);
    for expected in [retired, first, second, next] {
        let actual = uninterrupted.step().into_dispatched().unwrap();
        assert_eq!(actual.id, expected);
        assert_eq!(restored.step(), StepOutcome::Dispatched(actual));
    }
    assert_eq!(restored.step(), StepOutcome::Empty);
    assert_eq!(restored.stats(), uninterrupted.stats());
    assert_eq!(sibling.step().into_dispatched().unwrap().id, first);
    assert_eq!(sibling.step().into_dispatched().unwrap().id, second);
    assert_eq!(sibling.step(), StepOutcome::Empty);
    assert_eq!(
        Scheduler::from_checkpoint(&checkpoint).unwrap().stats(),
        baseline
    );
}

#[test]
fn checkpoint_rejects_an_incompatible_version() {
    let scheduler = Scheduler::new();
    let mut checkpoint = scheduler.checkpoint();
    checkpoint.version = SCHEDULER_CHECKPOINT_VERSION_V1 + 1;
    assert!(matches!(
        Scheduler::from_checkpoint(&checkpoint),
        Err(TypesError::UnsupportedDtoVersion(version)) if version == checkpoint.version
    ));
    assert_eq!(scheduler.stats(), Scheduler::new().stats());
}
