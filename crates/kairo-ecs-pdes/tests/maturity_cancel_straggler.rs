#![cfg(feature = "time-warp")]

use kairo_ecs_pdes::{
    LpId, TimeWarpComponentKey, TimeWarpDelta, TimeWarpEvent, TimeWarpEventId,
    TimeWarpMessageKind, TimeWarpRuntime,
};
use kairo_ecs_types::{EntityId, SimTime};

fn pos_event(tick: u128, sequence: u64, amount: i128) -> TimeWarpEvent {
    TimeWarpEvent {
        id: TimeWarpEventId {
            source_lp: LpId(0),
            dest_lp: LpId(1),
            tick: SimTime::from_ticks(tick),
            sequence,
        },
        kind: TimeWarpMessageKind::Positive,
        deltas: vec![TimeWarpDelta {
            entity: EntityId::new(12, 0),
            component_slot: 0,
            amount,
        }],
    }
}

fn anti_event(tick: u128, sequence: u64) -> TimeWarpEvent {
    TimeWarpEvent {
        id: TimeWarpEventId {
            source_lp: LpId(0),
            dest_lp: LpId(1),
            tick: SimTime::from_ticks(tick),
            sequence,
        },
        kind: TimeWarpMessageKind::Anti,
        deltas: Vec::new(),
    }
}

#[test]
fn cancellation_crossing_straggler_rollback_matches_serial_state() {
    let entity = EntityId::new(12, 0);
    let key = TimeWarpComponentKey {
        lp_id: LpId(1),
        entity,
        component_slot: 0,
    };

    // Optimistic execution: future event at tick 20 (+10), straggler at tick 10 (+5),
    // then anti-message cancelling the tick-20 event, then tick 30 (+7).
    let mut optimistic = TimeWarpRuntime::new();
    optimistic.write_component(key, 100);
    optimistic.process_event(pos_event(20, 1, 10)).unwrap();
    let rollback_report = optimistic.process_event(pos_event(10, 2, 5)).unwrap();
    assert_eq!(rollback_report.rollback_to, Some(SimTime::from_ticks(10)));

    let future_id = pos_event(20, 1, 10).id;
    let _cancel_report = optimistic.process_event(anti_event(20, 1)).unwrap();
    assert!(optimistic.is_canceled(future_id));
    // Re-delivering the cancelled future event must not re-apply its delta.
    optimistic.process_event(pos_event(20, 1, 10)).unwrap();
    optimistic.process_event(pos_event(30, 3, 7)).unwrap();

    // Serial oracle over the surviving events (tick 10: +5, tick 30: +7).
    let mut serial = TimeWarpRuntime::new();
    serial.write_component(key, 100);
    serial.process_event(pos_event(10, 2, 5)).unwrap();
    serial.process_event(pos_event(30, 3, 7)).unwrap();

    assert_eq!(
        optimistic.component_value(LpId(1), entity, 0),
        Some(112),
        "no duplicate or phantom effect from cancelled future event"
    );
    assert_eq!(
        optimistic.component_value(LpId(1), entity, 0),
        serial.component_value(LpId(1), entity, 0)
    );
}
