use kairo_ecs_des::{HazardInterruptionPolicy, SurvivalClock};
use kairo_ecs_rng::{derive_entity_seed, CauseId, DeterministicStream, HazardRate, TieBreaker};
use kairo_ecs_types::EntityId;

#[test]
fn test_independent_entity_stream_isolation_and_replay() {
    let run_seed = 987654321;
    let entity_a = EntityId::new(10, 1);
    let entity_b = EntityId::new(20, 1);

    // Verify entity seeds are strictly isolated
    let seed_a = derive_entity_seed(run_seed, entity_a);
    let seed_b = derive_entity_seed(run_seed, entity_b);
    assert_ne!(seed_a, seed_b);

    // Entity A survival clock
    let mut stream_a1 = DeterministicStream::from_seed(seed_a);
    let mut clock_a1 = SurvivalClock::new(
        1,
        Some(entity_a),
        HazardInterruptionPolicy::RetainResidual,
        TieBreaker::LowestCauseId,
        100.0,
    );
    clock_a1.set_channel(
        CauseId(1),
        0,
        HazardRate::Weibull {
            shape: 1.5,
            scale: 50.0,
        },
        0.0,
        &mut stream_a1,
    );

    // Replay Entity A stream independently
    let mut stream_a2 = DeterministicStream::from_seed(seed_a);
    let mut clock_a2 = SurvivalClock::new(
        1,
        Some(entity_a),
        HazardInterruptionPolicy::RetainResidual,
        TieBreaker::LowestCauseId,
        100.0,
    );
    clock_a2.set_channel(
        CauseId(1),
        0,
        HazardRate::Weibull {
            shape: 1.5,
            scale: 50.0,
        },
        0.0,
        &mut stream_a2,
    );

    let ev_a1 = clock_a1.compute_earliest_event(0.0).unwrap();
    let ev_a2 = clock_a2.compute_earliest_event(0.0).unwrap();
    assert_eq!(
        ev_a1, ev_a2,
        "Identical stream seeds must reproduce exact event times"
    );

    // Entity B survival clock
    let mut stream_b = DeterministicStream::from_seed(seed_b);
    let mut clock_b = SurvivalClock::new(
        2,
        Some(entity_b),
        HazardInterruptionPolicy::RetainResidual,
        TieBreaker::LowestCauseId,
        100.0,
    );
    clock_b.set_channel(
        CauseId(1),
        0,
        HazardRate::Weibull {
            shape: 1.5,
            scale: 50.0,
        },
        0.0,
        &mut stream_b,
    );

    let ev_b = clock_b.compute_earliest_event(0.0).unwrap();
    assert_ne!(
        ev_a1.1, ev_b.1,
        "Distinct entities must draw distinct realization times"
    );
}

#[test]
fn test_multithreaded_or_worker_stream_domain_noninterference() {
    // Simulate 4 parallel worker ranks independently managing survival clocks
    let run_seed = 0xABCD_EF01_2345_6789;
    let worker_count = 4;

    let mut worker_events = Vec::new();

    for rank in 0..worker_count {
        // Derive per-rank independent entity
        let entity = EntityId::new(100 + rank as u64, 0);
        let mut stream = DeterministicStream::from_seed(derive_entity_seed(run_seed, entity));

        let mut clock = SurvivalClock::new(
            rank as u64,
            Some(entity),
            HazardInterruptionPolicy::RetainResidual,
            TieBreaker::LowestCauseId,
            10.0,
        );
        clock.set_channel(
            CauseId(1),
            0,
            HazardRate::Gompertz { eta: 0.1, b: 0.05 },
            0.0,
            &mut stream,
        );
        let ev = clock.compute_earliest_event(0.0).unwrap();
        worker_events.push(ev);
    }

    // Verify all ranks produced valid and distinct events without interference
    for i in 0..worker_count {
        for j in (i + 1)..worker_count {
            assert_ne!(
                worker_events[i].1, worker_events[j].1,
                "Worker ranks must have isolated draws without collisions"
            );
        }
    }
}
