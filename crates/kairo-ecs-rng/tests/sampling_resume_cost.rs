#![forbid(unsafe_code)]

use kairo_ecs_rng::rand_distr::{Exp, Normal, Uniform};
use kairo_ecs_rng::DeterministicStream;
use kairo_ecs_types::EntityId;
use std::time::Instant;

#[test]
fn test_distribution_sampling_checkpoint_and_mid_buffer_restoration() {
    let mut original = DeterministicStream::from_run_seed(3001);

    let exp = Exp::new(1.5).unwrap();
    let norm = Normal::new(10.0, 2.5).unwrap();
    let unif = Uniform::new(100u32, 500u32).unwrap();

    // Consume some initial distribution draws
    for _ in 0..17 {
        let _: f64 = original.sample(exp);
        let _: f64 = original.sample(norm);
        let _: u32 = original.sample(unif);
    }

    // Checkpoint mid-stream
    let cp = original.checkpoint();

    // Draw reference sequence from original
    let mut ref_exp = Vec::new();
    let mut ref_norm = Vec::new();
    let mut ref_unif = Vec::new();

    for _ in 0..50 {
        ref_exp.push(original.sample(exp));
        ref_norm.push(original.sample(norm));
        ref_unif.push(original.sample(unif));
    }

    // Restore into a fresh stream
    let mut restored = DeterministicStream::restore(&cp).expect("restore checkpoint");

    // Draw from restored stream and assert exact bit-for-bit equality
    for i in 0..50 {
        let draw_e: f64 = restored.sample(exp);
        let draw_n: f64 = restored.sample(norm);
        let draw_u: u32 = restored.sample(unif);

        assert_eq!(
            draw_e.to_bits(),
            ref_exp[i].to_bits(),
            "Exp draw {i} mismatch"
        );
        assert_eq!(
            draw_n.to_bits(),
            ref_norm[i].to_bits(),
            "Normal draw {i} mismatch"
        );
        assert_eq!(draw_u, ref_unif[i], "Uniform draw {i} mismatch");
    }
}

#[test]
fn test_entity_stream_isolation_with_distributions() {
    let run_seed = 4242;
    let entity1 = EntityId::new(1, 0);
    let entity2 = EntityId::new(2, 0);

    let mut stream1_move =
        DeterministicStream::from_entity_purpose(run_seed, entity1, "agent.movement");
    let mut stream1_combat =
        DeterministicStream::from_entity_purpose(run_seed, entity1, "agent.combat");
    let mut stream2_move =
        DeterministicStream::from_entity_purpose(run_seed, entity2, "agent.movement");

    let norm = Normal::new(0.0, 1.0).unwrap();

    let s1_move_draws: Vec<f64> = (0..10).map(|_| stream1_move.sample(norm)).collect();
    let s1_combat_draws: Vec<f64> = (0..10).map(|_| stream1_combat.sample(norm)).collect();
    let s2_move_draws: Vec<f64> = (0..10).map(|_| stream2_move.sample(norm)).collect();

    // Purpose isolation on same entity
    assert_ne!(s1_move_draws, s1_combat_draws);
    // Entity isolation on same purpose
    assert_ne!(s1_move_draws, s2_move_draws);

    // Exact replay from same derivation
    let mut replica = DeterministicStream::from_entity_purpose(run_seed, entity1, "agent.movement");
    let replica_draws: Vec<f64> = (0..10).map(|_| replica.sample(norm)).collect();
    assert_eq!(s1_move_draws, replica_draws);
}

#[test]
fn test_distribution_sampling_throughput_budget() {
    let mut stream = DeterministicStream::from_run_seed(5555);
    let exp = Exp::new(2.0).unwrap();
    let norm = Normal::new(0.0, 1.0).unwrap();
    let unif = Uniform::new(0u64, 10_000u64).unwrap();

    let n = 200_000;
    let start = Instant::now();

    for _ in 0..n {
        let _: f64 = stream.sample(exp);
        let _: f64 = stream.sample(norm);
        let _: u64 = stream.sample(unif);
    }

    let elapsed = start.elapsed();
    let total_samples = n * 3;
    let rate = total_samples as f64 / elapsed.as_secs_f64();

    // Throughput should comfortably exceed 100,000 samples/sec in unoptimized debug mode
    assert!(
        rate > 100_000.0,
        "Sampling rate too slow: {rate:.0} samples/sec"
    );
}
