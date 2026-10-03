#![cfg(feature = "time-warp")]

use kairo_ecs_pdes::{
    LpId, TimeWarpComponentKey, TimeWarpDelta, TimeWarpError, TimeWarpEvent, TimeWarpEventId,
    TimeWarpMessageKind, TimeWarpRuntime,
};
use kairo_ecs_types::{EntityId, SimTime};

fn event(tick: u128, sequence: u64, kind: TimeWarpMessageKind, amount: i128) -> TimeWarpEvent {
    TimeWarpEvent {
        id: TimeWarpEventId {
            source_lp: LpId(0),
            dest_lp: LpId(1),
            tick: SimTime::from_ticks(tick),
            sequence,
        },
        kind,
        deltas: if kind == TimeWarpMessageKind::Positive {
            vec![TimeWarpDelta {
                entity: EntityId::new(48, 0),
                component_slot: 0,
                amount,
            }]
        } else {
            Vec::new()
        },
    }
}

fn component_key() -> TimeWarpComponentKey {
    TimeWarpComponentKey {
        lp_id: LpId(1),
        entity: EntityId::new(48, 0),
        component_slot: 0,
    }
}

#[test]
fn rollback_before_first_checkpoint_preserves_initial_component_seed() {
    let mut runtime = TimeWarpRuntime::new();
    runtime.write_component(component_key(), 100);

    runtime
        .process_event(event(10, 1, TimeWarpMessageKind::Positive, 3))
        .unwrap();
    let report = runtime
        .process_event(event(5, 2, TimeWarpMessageKind::Positive, 2))
        .unwrap();

    assert_eq!(report.rollback_to, Some(SimTime::from_ticks(5)));
    assert_eq!(
        report.rolled_back_events,
        vec![event(10, 1, TimeWarpMessageKind::Positive, 3).id]
    );
    assert_eq!(
        runtime.component_value(LpId(1), EntityId::new(48, 0), 0),
        Some(102),
        "rollback should restore the seed before applying the straggler"
    );
}

#[test]
fn rollback_recreation_does_not_resurrect_an_initial_generation_token() {
    let mut runtime = TimeWarpRuntime::new();
    let stale_initial = runtime.write_component(component_key(), 100);

    runtime
        .process_event(event(10, 1, TimeWarpMessageKind::Positive, 3))
        .unwrap();
    assert_eq!(
        runtime.read_component(stale_initial),
        Err(TimeWarpError::StaleGeneration)
    );

    runtime
        .process_event(event(5, 2, TimeWarpMessageKind::Positive, 2))
        .unwrap();

    assert_eq!(
        runtime.read_component(stale_initial),
        Err(TimeWarpError::StaleGeneration),
        "rollback must not reuse a generation previously invalidated by the future event"
    );
}

#[test]
fn anti_cancellation_rebuild_preserves_initial_component_seed() {
    let mut runtime = TimeWarpRuntime::new();
    runtime.write_component(component_key(), 100);
    let future = event(10, 1, TimeWarpMessageKind::Positive, 3);
    let future_id = future.id;

    runtime.process_event(future).unwrap();
    let report = runtime
        .process_event(event(10, 1, TimeWarpMessageKind::Anti, 0))
        .unwrap();

    assert_eq!(report.canceled_events, vec![future_id]);
    assert!(runtime.is_canceled(future_id));
    assert_eq!(
        runtime.component_value(LpId(1), EntityId::new(48, 0), 0),
        Some(100),
        "rebuilding state after cancellation should retain the initial seed"
    );
}
