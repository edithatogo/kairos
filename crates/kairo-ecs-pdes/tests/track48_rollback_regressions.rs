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

fn event_in_slot(
    tick: u128,
    sequence: u64,
    kind: TimeWarpMessageKind,
    component_slot: u32,
    amount: i128,
) -> TimeWarpEvent {
    let mut event = event(tick, sequence, kind, amount);
    if let Some(delta) = event.deltas.first_mut() {
        delta.component_slot = component_slot;
    }
    event
}

#[test]
fn component_token_from_another_runtime_is_stale() {
    let mut first = TimeWarpRuntime::new();
    let mut second = TimeWarpRuntime::new();
    let token = first.write_component(component_key(), 100);
    second.write_component(component_key(), 100);

    assert_eq!(
        second.read_component(token),
        Err(TimeWarpError::StaleGeneration),
        "matching keys and logical generations do not make tokens runtime-local"
    );
}

#[test]
fn checkpoint_restore_does_not_revive_a_stale_initial_token() {
    let mut runtime = TimeWarpRuntime::new();
    let seed_token = runtime.write_component(component_key(), 100);

    runtime
        .process_event(event_in_slot(2, 1, TimeWarpMessageKind::Positive, 1, 7))
        .unwrap();
    runtime
        .process_event(event_in_slot(10, 2, TimeWarpMessageKind::Positive, 0, 3))
        .unwrap();
    assert_eq!(
        runtime.read_component(seed_token),
        Err(TimeWarpError::StaleGeneration)
    );

    runtime
        .process_event(event_in_slot(5, 3, TimeWarpMessageKind::Positive, 1, 2))
        .unwrap();

    assert_eq!(
        runtime.component_value(LpId(1), EntityId::new(48, 0), 0),
        Some(100),
        "the checkpoint at tick 2 preserves the seed value"
    );
    assert_eq!(
        runtime.read_component(seed_token),
        Err(TimeWarpError::StaleGeneration),
        "restoring an older cell must issue a new validity stamp"
    );
}

#[test]
fn fossil_collection_retires_only_cancellations_strictly_before_gvt() {
    let mut runtime = TimeWarpRuntime::new();
    let anti = |tick| event(tick, 1, TimeWarpMessageKind::Anti, 0);
    for tick in [4, 5, 5, 6] {
        runtime.process_event(anti(tick)).unwrap();
    }
    assert_eq!(runtime.overhead_metrics().canceled_event_ids, 3);

    let floor = SimTime::from_ticks(5);
    runtime.fossil_collect(floor).unwrap();
    assert!(!runtime.is_canceled(anti(4).id));
    for kind in [TimeWarpMessageKind::Positive, TimeWarpMessageKind::Anti] {
        let late = event(4, 1, kind, 1);
        assert_eq!(
            runtime.process_event(late.clone()),
            Err(TimeWarpError::EventBeforeGvt {
                event_id: late.id,
                gvt: floor
            })
        );
    }
    assert!(matches!(
        runtime.fossil_collect(SimTime::from_ticks(4)),
        Err(TimeWarpError::GvtRegression { .. })
    ));
    assert_eq!(runtime.overhead_metrics().canceled_event_ids, 2);
    assert_eq!(runtime.gvt(), floor);
    for tick in [5, 6] {
        let positive = event(tick, 1, TimeWarpMessageKind::Positive, 10);
        let id = positive.id;
        let report = runtime.process_event(positive).unwrap();
        assert_eq!(report.canceled_events, vec![id]);
        assert_eq!(report.cancellation_metadata.len(), 1);
        assert_eq!(
            report.cancellation_metadata[0].reason,
            kairo_ecs_pdes::TimeWarpCancellationReason::PositiveArrivedAfterAntiMessage
        );
        assert!(runtime.is_canceled(id));
    }
    assert_eq!(runtime.executed_len(), 0);
    assert_eq!(
        runtime.component_value(LpId(1), EntityId::new(48, 0), 0),
        None
    );
}
