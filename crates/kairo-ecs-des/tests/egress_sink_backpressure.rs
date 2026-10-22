#![forbid(unsafe_code)]

use kairo_ecs_des::{
    EgressSink, HandoverEnvelope, HandoverOperationId, HandoverTelemetryEvent, TransferState,
};
use kairo_ecs_types::{EntityId, SimTime};

#[test]
fn test_egress_sink_reservation_vs_occupancy_accounting() {
    let mut sink = EgressSink::<String>::new("ward_bays", 2); // capacity = 2

    let op1 = HandoverOperationId::new(1);
    let op2 = HandoverOperationId::new(2);
    let op3 = HandoverOperationId::new(3);

    let ent1 = EntityId::new(10, 1);
    let ent2 = EntityId::new(20, 1);
    let ent3 = EntityId::new(30, 1);

    let env1 = HandoverEnvelope::new(
        op1,
        ent1,
        "patient_a".to_string(),
        "ed",
        "ward",
        SimTime::from_ticks(0),
        None,
    );
    let env2 = HandoverEnvelope::new(
        op2,
        ent2,
        "patient_b".to_string(),
        "ed",
        "ward",
        SimTime::from_ticks(0),
        None,
    );
    let env3 = HandoverEnvelope::new(
        op3,
        ent3,
        "patient_c".to_string(),
        "ed",
        "ward",
        SimTime::from_ticks(0),
        None,
    );

    // Request intake 1 -> granted immediate reservation
    let state1 = sink
        .request_intake(env1.clone(), SimTime::from_ticks(1))
        .unwrap();
    assert_eq!(state1, TransferState::Reserved);
    assert_eq!(sink.reserved_count(), 1);
    assert_eq!(sink.occupied_count(), 0);
    assert_eq!(sink.total_committed(), 1);
    assert_eq!(sink.available_capacity(), 1);

    // Request intake 2 -> granted immediate reservation
    let state2 = sink
        .request_intake(env2.clone(), SimTime::from_ticks(2))
        .unwrap();
    assert_eq!(state2, TransferState::Reserved);
    assert_eq!(sink.reserved_count(), 2);
    assert_eq!(sink.total_committed(), 2);
    assert_eq!(sink.available_capacity(), 0);

    // Request intake 3 -> capacity committed; placed in boarding hold at sender!
    let state3 = sink
        .request_intake(env3.clone(), SimTime::from_ticks(3))
        .unwrap();
    assert_eq!(state3, TransferState::Offered);
    assert!(sink.has_boarding_hold(op3));
    assert_eq!(sink.boarding_queue_len(), 1);
    assert_eq!(sink.reserved_count(), 2);
    assert_eq!(sink.occupied_count(), 0);

    // Physical arrival of entity 1: converts reservation into physical occupancy
    sink.receive_arrival(env1, SimTime::from_ticks(10)).unwrap();
    assert_eq!(sink.reserved_count(), 1);
    assert_eq!(sink.occupied_count(), 1);
    assert_eq!(sink.total_committed(), 2);
    assert!(sink.is_physically_present(ent1));

    // Acknowledge handover for entity 1
    let ack = sink
        .acknowledge_handover(op1, SimTime::from_ticks(12))
        .unwrap();
    assert_eq!(ack.state, TransferState::Acknowledged);

    // Physical arrival of entity 2: converts reservation into physical occupancy
    sink.receive_arrival(env2, SimTime::from_ticks(15)).unwrap();
    assert_eq!(sink.reserved_count(), 0);
    assert_eq!(sink.occupied_count(), 2);
    assert_eq!(sink.total_committed(), 2);
    assert!(sink.is_physically_present(ent2));

    // Discharge / exit of entity 1: physical occupancy freed -> automatically promotes boarding hold op3!
    let discharged = sink
        .release_occupancy(ent1, SimTime::from_ticks(50))
        .unwrap();
    assert_eq!(discharged.entity, ent1);
    assert!(!sink.is_physically_present(ent1));

    // op3 is now promoted to Reserved!
    assert_eq!(sink.boarding_queue_len(), 0);
    assert_eq!(sink.reserved_count(), 1);
    assert_eq!(sink.occupied_count(), 1); // entity 2 is still occupied
    assert_eq!(sink.total_committed(), 2);

    // Telemetry audit verification
    let events = sink.telemetry();
    assert!(events.iter().any(|e| matches!(
        e,
        HandoverTelemetryEvent::BoardingHoldEntered { op_id, .. } if *op_id == op3
    )));
    assert!(events.iter().any(|e| matches!(
        e,
        HandoverTelemetryEvent::BoardingHoldReleased { op_id, .. } if *op_id == op3
    )));
}

#[test]
fn test_egress_sink_compensation_promotes_waiting_boarding_holds() {
    let mut sink = EgressSink::<()>::new("sink", 1); // capacity = 1

    let op1 = HandoverOperationId::new(1);
    let op2 = HandoverOperationId::new(2);

    let env1 = HandoverEnvelope::new(
        op1,
        EntityId::new(1, 1),
        (),
        "depot_1",
        "depot_2",
        SimTime::ZERO,
        None,
    );
    let env2 = HandoverEnvelope::new(
        op2,
        EntityId::new(2, 1),
        (),
        "depot_1",
        "depot_2",
        SimTime::ZERO,
        None,
    );

    sink.request_intake(env1, SimTime::ZERO).unwrap(); // takes reservation
    sink.request_intake(env2, SimTime::ZERO).unwrap(); // enters boarding hold
    assert_eq!(sink.boarding_queue_len(), 1);

    // op1 is rejected / cancelled before arrival -> trigger compensation
    sink.compensate_failure(op1, "cancelled_by_sender", SimTime::from_ticks(5));

    // op1 reservation cancelled -> op2 automatically promoted to Reserved!
    assert_eq!(sink.boarding_queue_len(), 0);
    assert_eq!(sink.reserved_count(), 1);
    assert_eq!(sink.occupied_count(), 0);
}
