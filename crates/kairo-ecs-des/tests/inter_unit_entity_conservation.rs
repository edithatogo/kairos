#![forbid(unsafe_code)]

use kairo_ecs_des::{
    BoundaryChannel, EgressSink, HandoverEnvelope, HandoverOperationId, TransferState,
    TransitDelayDistribution,
};
use kairo_ecs_rng::DeterministicStream;
use kairo_ecs_types::{EntityId, SimTime};
use std::collections::HashSet;

#[test]
fn test_two_depot_inter_unit_entity_conservation_and_idempotency() {
    let initial_count = 10;
    let mut origin_depot: HashSet<EntityId> =
        (1..=initial_count).map(|i| EntityId::new(i, 1)).collect();

    let mut channel = BoundaryChannel::<u64>::new(
        "depot_channel",
        5, // channel capacity
        TransitDelayDistribution::Fixed(SimTime::from_ticks(20)),
    );

    let mut dest_sink = EgressSink::<u64>::new(
        "destination_depot",
        5, // destination capacity
    );

    let mut rng = DeterministicStream::new(10101);

    // Track total entities invariant: origin + channel (in-transit) + destination (physically present)
    let check_conservation = |origin: &HashSet<EntityId>,
                              ch: &BoundaryChannel<u64>,
                              sink: &EgressSink<u64>,
                              expected: usize| {
        let total = origin.len() + ch.in_transit_count() + sink.occupied_count();
        assert_eq!(
            total, expected,
            "Conservation invariant violated: origin={}, in_transit={}, dest_occupied={}, total={}, expected={}",
            origin.len(),
            ch.in_transit_count(),
            sink.occupied_count(),
            total,
            expected
        );
    };

    check_conservation(&origin_depot, &channel, &dest_sink, initial_count as usize);

    // Step 1: Send 5 entities from Origin to Destination
    for i in 1..=5 {
        let ent = EntityId::new(i, 1);
        let op_id = HandoverOperationId::new(i);

        let env = HandoverEnvelope::new(
            op_id,
            ent,
            i * 10,
            "origin",
            "destination",
            SimTime::from_ticks(0),
            None,
        );

        // Reserve intake at destination sink
        let intake_res = dest_sink
            .request_intake(env.clone(), SimTime::from_ticks(0))
            .unwrap();
        assert_eq!(intake_res, TransferState::Reserved);

        // Submit to channel
        channel.request_transfer(env).unwrap();
        channel.reserve(op_id).unwrap();

        // Depart entity: physically departs origin depot
        origin_depot.remove(&ent);
        channel
            .depart(op_id, SimTime::from_ticks(0), &mut rng)
            .unwrap();

        // Test duplicate transfer request (idempotency check)
        let dup_env = HandoverEnvelope::new(
            op_id,
            ent,
            i * 10,
            "origin",
            "destination",
            SimTime::from_ticks(0),
            None,
        );
        let dup_res = channel.request_transfer(dup_env).unwrap();
        assert_eq!(
            dup_res,
            TransferState::InTransit,
            "Duplicate transfer request must return active state without duplicating"
        );
    }

    // Conservation holds while 5 items are in transit
    assert_eq!(origin_depot.len(), 5);
    assert_eq!(channel.in_transit_count(), 5);
    assert_eq!(dest_sink.occupied_count(), 0);
    assert_eq!(dest_sink.reserved_count(), 5);
    check_conservation(&origin_depot, &channel, &dest_sink, initial_count as usize);

    // Step 2: Time advances to tick 20 -> arrivals occur
    let arrivals = channel.collect_arrivals(SimTime::from_ticks(20));
    assert_eq!(arrivals.len(), 5);
    assert_eq!(channel.in_transit_count(), 0);

    for arr in arrivals {
        dest_sink
            .receive_arrival(arr.clone(), SimTime::from_ticks(20))
            .unwrap();
        channel
            .acknowledge(arr.operation_id, SimTime::from_ticks(20))
            .unwrap();
        dest_sink
            .acknowledge_handover(arr.operation_id, SimTime::from_ticks(20))
            .unwrap();
    }

    // Now 5 items are physically occupied in destination, 5 remain in origin
    assert_eq!(origin_depot.len(), 5);
    assert_eq!(channel.in_transit_count(), 0);
    assert_eq!(dest_sink.occupied_count(), 5);
    assert_eq!(dest_sink.reserved_count(), 0);
    check_conservation(&origin_depot, &channel, &dest_sink, initial_count as usize);

    // Step 3: Attempt to send 6th entity -> destination is completely full (occupied = 5, capacity = 5)
    let ent6 = EntityId::new(6, 1);
    let op6 = HandoverOperationId::new(6);
    let env6 = HandoverEnvelope::new(
        op6,
        ent6,
        60,
        "origin",
        "destination",
        SimTime::from_ticks(25),
        None,
    );

    let intake6 = dest_sink
        .request_intake(env6, SimTime::from_ticks(25))
        .unwrap();
    assert_eq!(intake6, TransferState::Offered);
    assert!(dest_sink.has_boarding_hold(op6));

    // Entity 6 remains in Origin depot during boarding hold!
    assert!(origin_depot.contains(&ent6));
    check_conservation(&origin_depot, &channel, &dest_sink, initial_count as usize);

    // Step 4: Destination discharges entity 1 -> capacity freed -> entity 6 automatically promoted!
    let ent1 = EntityId::new(1, 1);
    let discharged = dest_sink
        .release_occupancy(ent1, SimTime::from_ticks(30))
        .unwrap();
    assert_eq!(discharged.entity, ent1);

    // Discharged entity exits simulation (or arrives at subsequent sink)
    // op6 was promoted to Reserved
    assert_eq!(dest_sink.reserved_count(), 1);
    assert_eq!(dest_sink.occupied_count(), 4);
    assert!(!dest_sink.has_boarding_hold(op6));
}
