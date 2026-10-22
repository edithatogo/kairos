#![forbid(unsafe_code)]

use kairo_ecs_des::{
    BoundaryChannel, BoundaryError, HandoverEnvelope, HandoverOperationId, TransferState,
    TransitDelayDistribution,
};
use kairo_ecs_rng::DeterministicStream;
use kairo_ecs_types::{EntityId, SimTime};

#[test]
fn test_fixed_transit_delay_preserves_ticks_and_tie_order() {
    let mut channel = BoundaryChannel::<String>::new(
        "depot_a_to_b",
        10,
        TransitDelayDistribution::Fixed(SimTime::from_ticks(50)),
    );
    let mut rng = DeterministicStream::new(42);

    let op1 = HandoverOperationId::new(1);
    let op2 = HandoverOperationId::new(2);

    let env1 = HandoverEnvelope::new(
        op1,
        EntityId::new(100, 1),
        "cargo_1".to_string(),
        "depot_a",
        "depot_b",
        SimTime::from_ticks(10),
        None,
    );
    let env2 = HandoverEnvelope::new(
        op2,
        EntityId::new(101, 1),
        "cargo_2".to_string(),
        "depot_a",
        "depot_b",
        SimTime::from_ticks(10),
        None,
    );

    assert_eq!(
        channel.request_transfer(env1).unwrap(),
        TransferState::Requested
    );
    assert_eq!(
        channel.request_transfer(env2).unwrap(),
        TransferState::Requested
    );

    // Both depart at time 10 with fixed delay 50 -> arrive at exactly 60
    let arr1 = channel
        .depart(op1, SimTime::from_ticks(10), &mut rng)
        .unwrap();
    let arr2 = channel
        .depart(op2, SimTime::from_ticks(10), &mut rng)
        .unwrap();

    assert_eq!(arr1, SimTime::from_ticks(60));
    assert_eq!(arr2, SimTime::from_ticks(60));

    // At time 59, nothing has arrived
    assert!(channel.collect_arrivals(SimTime::from_ticks(59)).is_empty());

    // At time 60, both arrive in insertion sequence order
    let arrivals = channel.collect_arrivals(SimTime::from_ticks(60));
    assert_eq!(arrivals.len(), 2);
    assert_eq!(arrivals[0].operation_id, op1);
    assert_eq!(arrivals[1].operation_id, op2);
}

#[test]
fn test_uniform_and_exponential_transit_distributions_replay_deterministically() {
    fn run_uniform_samples(seed: u64) -> Vec<SimTime> {
        let dist = TransitDelayDistribution::Uniform {
            min: SimTime::from_ticks(20),
            max: SimTime::from_ticks(100),
        };
        let mut rng = DeterministicStream::new(seed);
        (0..10).map(|_| dist.sample(&mut rng)).collect()
    }

    let run1 = run_uniform_samples(12345);
    let run2 = run_uniform_samples(12345);
    let run3 = run_uniform_samples(99999);

    assert_eq!(
        run1, run2,
        "Identical seed must yield identical uniform samples"
    );
    assert_ne!(
        run1, run3,
        "Different seed must yield different uniform samples"
    );

    for sample in run1 {
        assert!(sample.ticks() >= 20 && sample.ticks() <= 100);
    }

    fn run_exponential_samples(seed: u64) -> Vec<SimTime> {
        let dist = TransitDelayDistribution::Exponential { mean_ticks: 50 };
        let mut rng = DeterministicStream::new(seed);
        (0..10).map(|_| dist.sample(&mut rng)).collect()
    }

    let exp1 = run_exponential_samples(54321);
    let exp2 = run_exponential_samples(54321);
    let exp3 = run_exponential_samples(11111);

    assert_eq!(
        exp1, exp2,
        "Identical seed must yield identical exponential samples"
    );
    assert_ne!(
        exp1, exp3,
        "Different seed must yield different exponential samples"
    );
}

#[test]
fn test_boundary_channel_bounded_capacity_backpressure() {
    let mut channel = BoundaryChannel::<()>::new(
        "restricted_ch",
        2, // max 2 in transit
        TransitDelayDistribution::Fixed(SimTime::from_ticks(100)),
    );
    let mut rng = DeterministicStream::new(777);

    for i in 1..=3 {
        let op = HandoverOperationId::new(i);
        let env = HandoverEnvelope::new(
            op,
            EntityId::new(i, 1),
            (),
            "origin",
            "dest",
            SimTime::ZERO,
            None,
        );
        channel.request_transfer(env).unwrap();
    }

    // First two depart successfully
    channel
        .depart(HandoverOperationId::new(1), SimTime::ZERO, &mut rng)
        .unwrap();
    channel
        .depart(HandoverOperationId::new(2), SimTime::ZERO, &mut rng)
        .unwrap();

    assert_eq!(channel.in_transit_count(), 2);
    assert_eq!(channel.available_channel_capacity(), 0);

    // Third depart exceeds capacity
    let err = channel
        .depart(HandoverOperationId::new(3), SimTime::ZERO, &mut rng)
        .unwrap_err();

    assert_eq!(
        err,
        BoundaryError::CapacityExceeded {
            max_capacity: 2,
            current: 2,
        }
    );
}

#[test]
fn test_boundary_channel_expiry_and_cancellation() {
    let mut channel = BoundaryChannel::<()>::new(
        "timed_channel",
        5,
        TransitDelayDistribution::Fixed(SimTime::from_ticks(50)),
    );
    let mut rng = DeterministicStream::new(123);

    let op1 = HandoverOperationId::new(1);
    let env1 = HandoverEnvelope::new(
        op1,
        EntityId::new(10, 1),
        (),
        "origin",
        "dest",
        SimTime::ZERO,
        Some(SimTime::from_ticks(30)), // expires at tick 30
    );
    channel.request_transfer(env1).unwrap();

    // Trying to depart at tick 35 should fail due to expiry
    let err = channel
        .depart(op1, SimTime::from_ticks(35), &mut rng)
        .unwrap_err();
    assert_eq!(err, BoundaryError::TransferExpired(op1));

    // Cancellation test
    let op2 = HandoverOperationId::new(2);
    let env2 = HandoverEnvelope::new(
        op2,
        EntityId::new(20, 1),
        (),
        "origin",
        "dest",
        SimTime::ZERO,
        None,
    );
    channel.request_transfer(env2).unwrap();
    let cancelled = channel.cancel(op2).unwrap();
    assert_eq!(cancelled.state, TransferState::Cancelled);
}
