#![forbid(unsafe_code)]

use kairo_ecs_rng::rand_distr::{Exp1, Open01, Uniform};
use kairo_ecs_rng::{
    DeterministicStream, RNG_DISTRIBUTION_PROVENANCE_LEGACY, RNG_DISTRIBUTION_PROVENANCE_V1,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct SamplingReplayFixture {
    fixture: String,
    version: u32,
    distribution_provenance: String,
    run_seed: u64,
    open01_draws: Vec<f64>,
    exp1_draws: Vec<f64>,
    uniform_u32_draws: Vec<u32>,
    requirement: String,
}

#[test]
fn test_sampling_conformance_fixture_verification() {
    let fixture_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../conformance/fixtures/sampling_replay.json"
    );
    let fixture_content =
        std::fs::read_to_string(fixture_path).expect("must read sampling_replay.json fixture");
    let fixture: SamplingReplayFixture =
        serde_json::from_str(&fixture_content).expect("parse sampling_replay.json");

    assert_eq!(fixture.fixture, "sampling_replay");
    assert_eq!(fixture.version, 1);
    assert_eq!(
        fixture.distribution_provenance,
        RNG_DISTRIBUTION_PROVENANCE_V1
    );

    let mut stream = DeterministicStream::from_run_seed(fixture.run_seed);
    assert_eq!(
        stream.distribution_provenance(),
        RNG_DISTRIBUTION_PROVENANCE_V1
    );

    let mut observed_open01 = Vec::new();
    for _ in 0..fixture.open01_draws.len() {
        let u: f64 = stream.sample(Open01);
        observed_open01.push(u);
    }
    for (obs, exp) in observed_open01.iter().zip(&fixture.open01_draws) {
        assert!((obs - exp).abs() < 1e-12, "Open01 mismatch: {obs} vs {exp}");
    }

    let mut observed_exp1 = Vec::new();
    for _ in 0..fixture.exp1_draws.len() {
        let e: f64 = stream.sample(Exp1);
        observed_exp1.push(e);
    }
    for (obs, exp) in observed_exp1.iter().zip(&fixture.exp1_draws) {
        assert!((obs - exp).abs() < 1e-12, "Exp1 mismatch: {obs} vs {exp}");
    }

    let dist_u32 = Uniform::new_inclusive(1u32, 100u32).unwrap();
    let mut observed_u32 = Vec::new();
    for _ in 0..fixture.uniform_u32_draws.len() {
        let val: u32 = stream.sample(dist_u32);
        observed_u32.push(val);
    }
    assert_eq!(observed_u32, fixture.uniform_u32_draws);
}

#[test]
fn test_legacy_vs_rand_distr_provenance_separation() {
    let mut legacy_stream = DeterministicStream::from_run_seed(12345);
    legacy_stream.set_distribution_provenance(RNG_DISTRIBUTION_PROVENANCE_LEGACY);
    assert_eq!(
        legacy_stream.distribution_provenance(),
        RNG_DISTRIBUTION_PROVENANCE_LEGACY
    );

    let legacy_cp = legacy_stream.checkpoint();
    assert_eq!(
        legacy_cp.distribution_provenance,
        RNG_DISTRIBUTION_PROVENANCE_LEGACY
    );

    let restored_legacy =
        DeterministicStream::restore(&legacy_cp).expect("restore legacy checkpoint");
    assert_eq!(
        restored_legacy.distribution_provenance(),
        RNG_DISTRIBUTION_PROVENANCE_LEGACY
    );

    // Unsupported provenance rejected
    let mut invalid_cp = legacy_cp.clone();
    invalid_cp.distribution_provenance = "unsupported_sampling_engine_v99".to_string();
    assert!(DeterministicStream::restore(&invalid_cp).is_err());
}
