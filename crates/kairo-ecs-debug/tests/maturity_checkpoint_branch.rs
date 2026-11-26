use std::collections::BTreeMap;
use std::num::NonZeroUsize;
use std::sync::Arc;

use kairo_ecs_core::Scheduler;
use kairo_ecs_debug::ReplaySession;
use kairo_ecs_rng::{entity_stream, DeterministicStream};
use kairo_ecs_types::{DispatchedEvent, EntityId, EventKind, ScheduleRequest, SimTime};

#[derive(Clone)]
struct BranchModel {
    readings: Arc<BTreeMap<u32, u32>>,
    rng: DeterministicStream,
}

fn apply(model: &mut BranchModel, scheduler: &mut Scheduler, event: DispatchedEvent) {
    let EventKind::Custom(kind) = event.kind;
    Arc::make_mut(&mut model.readings).insert(kind, model.rng.next_u32());
    if kind == 1 {
        scheduler.schedule(ScheduleRequest {
            at: SimTime::from_ticks(event.at.ticks() + 3),
            priority: 0,
            entity: event.entity,
            kind: EventKind::custom(4),
        });
    }
}

fn req(tick: u128, priority: i32, entity: EntityId, kind: u32) -> ScheduleRequest {
    ScheduleRequest {
        at: SimTime::from_ticks(tick),
        priority,
        entity: Some(entity),
        kind: EventKind::custom(kind),
    }
}

#[test]
fn forward_back_forward_reproduces_state_and_isolates_sibling_branch() {
    let entity = EntityId::new(1, 0);
    let mut scheduler = Scheduler::new();
    scheduler.schedule(req(1, 0, entity, 1));
    scheduler.schedule(req(2, 0, entity, 2));
    scheduler.schedule(req(5, 0, entity, 3));

    let mut primary = ReplaySession::new(
        scheduler,
        BranchModel {
            readings: Arc::default(),
            rng: entity_stream(7, entity),
        },
        apply,
        NonZeroUsize::new(1).unwrap(),
        NonZeroUsize::new(16).unwrap(),
    );

    // Step forward twice (dispatches kind 1 at tick 1, kind 2 at tick 2).
    assert!(primary.step().unwrap().is_some());
    assert!(primary.step().unwrap().is_some());
    let forward_readings = primary.model().readings.clone();
    let forward_tick = primary.scheduler().now();

    // Rewind primary back to step 1, then step forward again to verify forward-back-forward parity.
    primary.seek(1).unwrap();
    assert!(primary.step().unwrap().is_some());
    assert_eq!(primary.model().readings, forward_readings);
    assert_eq!(primary.scheduler().now(), forward_tick);

    // Fork a sibling branch with an intervening event; primary must remain isolated.
    let mut branch = primary.fork(|_model, sched| {
        sched.schedule(req(3, -1, entity, 9));
    });
    while branch.step().unwrap().is_some() {}

    assert!(branch.model().readings.contains_key(&9));
    assert!(!primary.model().readings.contains_key(&9));
    assert_eq!(primary.model().readings, forward_readings);
    assert_eq!(primary.scheduler().now(), forward_tick);

    // Stepping primary forward finishes only the original schedule (kind 4 at tick 4, kind 3 at tick 5).
    while primary.step().unwrap().is_some() {}
    assert_eq!(primary.scheduler().now(), SimTime::from_ticks(5));
    assert!(!primary.model().readings.contains_key(&9));
    assert!(primary.model().readings.contains_key(&4));
    assert!(primary.model().readings.contains_key(&3));
}
