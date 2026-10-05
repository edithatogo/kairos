//! Test-first conformance fixture for the private C2.0 intrinsic work provider.
//! The red runner overlays this file into the real calibration crate's tests.

#[path = "../src/seed_map.rs"]
mod seed_map;
#[path = "../src/work_duration.rs"]
mod work_duration;

use kairo_ecs_types::SimDuration;
use seed_map::{CalibrationSeedMap, CalibrationStream, CalibrationStreamKey, SeedPurpose};
use work_duration::{IntrinsicDurationDistribution, IntrinsicWorkProvider, WorkDurationError};

const ROOT_SEED: u64 = 1234;
const REPLICATION: u64 = 7;
const STUDY: &str = "study-α";
const CASE: &str = "case-0001";
const TASK: &str = "triage:1";

fn identity(
    schedule: &str,
    purpose: SeedPurpose,
    task: &str,
) -> (CalibrationStream, CalibrationStreamKey) {
    let mut map = CalibrationSeedMap::new(1, STUDY, ROOT_SEED).unwrap();
    let key = map
        .key_for(schedule, REPLICATION, CASE, task, purpose)
        .unwrap();
    let stream = map
        .stream_for(schedule, REPLICATION, CASE, task, purpose)
        .unwrap();
    (stream, key)
}

fn service_identity(schedule: &str) -> (CalibrationStream, CalibrationStreamKey) {
    identity(schedule, SeedPurpose::Service, TASK)
}

#[test]
fn canonical_golden_samples_and_draw_positions_match() {
    let (mut stream, key) = service_identity("crn-v1");
    let distribution =
        IntrinsicDurationDistribution::weighted_ticks(vec![(10, 3), (20, 2), (30, 5)]).unwrap();

    for (duration, position) in [(30, 1), (10, 2), (20, 3)] {
        let sample = distribution.sample(&mut stream, &key).unwrap();
        assert_eq!(sample.duration(), SimDuration::from_ticks(duration));
        assert_eq!(sample.draw_before(), position - 1);
        assert_eq!(sample.draw_after(), position);
        assert_eq!(stream.draw_position(), position);
    }
}

#[test]
fn support_order_is_preserved_and_equal_order_is_repeatable() {
    let first =
        IntrinsicDurationDistribution::weighted_ticks(vec![(10, 3), (20, 2), (30, 5)]).unwrap();
    let reordered =
        IntrinsicDurationDistribution::weighted_ticks(vec![(30, 5), (20, 2), (10, 3)]).unwrap();
    let repeated =
        IntrinsicDurationDistribution::weighted_ticks(vec![(10, 3), (20, 2), (30, 5)]).unwrap();

    let (mut a, key_a) = service_identity("crn-v1");
    let (mut b, key_b) = service_identity("crn-v1");
    let (mut c, key_c) = service_identity("crn-v1");
    assert_eq!(
        first.sample(&mut a, &key_a).unwrap().duration(),
        SimDuration::from_ticks(30)
    );
    assert_eq!(
        reordered.sample(&mut b, &key_b).unwrap().duration(),
        SimDuration::from_ticks(10)
    );
    assert_eq!(
        repeated.sample(&mut c, &key_c).unwrap().duration(),
        SimDuration::from_ticks(30)
    );
}

#[test]
fn exact_cumulative_boundaries_select_the_next_support_bucket() {
    let distribution =
        IntrinsicDurationDistribution::weighted_ticks(vec![(10, 3), (20, 2), (30, 5)]).unwrap();
    let (mut first_boundary, first_key) = service_identity("boundary-11");
    let (mut second_boundary, second_key) = service_identity("boundary-18");

    // Independently derived real SplitMix64 values have residues 3 and 5.
    assert_eq!(
        distribution
            .sample(&mut first_boundary, &first_key)
            .unwrap()
            .duration(),
        SimDuration::from_ticks(20)
    );
    assert_eq!(
        distribution
            .sample(&mut second_boundary, &second_key)
            .unwrap()
            .duration(),
        SimDuration::from_ticks(30)
    );
}

#[test]
fn real_large_total_rejects_first_draw_and_counts_both_transitions() {
    let total = 0x8000_0000_0000_0001;
    let distribution = IntrinsicDurationDistribution::weighted_ticks(vec![(1, total)]).unwrap();
    let (mut stream, key) = service_identity("reject-4");

    // Independent framing/SHA-256/SplitMix derivation: d1=0x36fc95308a0b3f17
    // is below threshold 0x7fffffffffffffff; d2=0x9b2278b1b74dac29 is accepted.
    let sample = distribution.sample(&mut stream, &key).unwrap();
    assert_eq!(sample.duration(), SimDuration::from_ticks(1));
    assert_eq!(sample.draw_before(), 0);
    assert_eq!(sample.draw_after(), 2);
    assert_eq!(stream.draw_position(), 2);
}

#[test]
fn fixed_u128_max_is_lossless_and_consumes_no_draws() {
    let distribution = IntrinsicDurationDistribution::fixed(u128::MAX).unwrap();
    let (mut stream, key) = service_identity("fixed-max");
    let sample = distribution.sample(&mut stream, &key).unwrap();

    assert_eq!(sample.duration(), SimDuration::from_ticks(u128::MAX));
    assert_eq!(sample.draw_before(), 0);
    assert_eq!(sample.draw_after(), 0);
    assert_eq!(stream.draw_position(), 0);
}

#[test]
fn wrong_purpose_precedes_key_mismatch_and_failure_preserves_stream() {
    let distribution =
        IntrinsicDurationDistribution::weighted_ticks(vec![(8, 1), (12, 1)]).unwrap();
    let (service_stream, service_key) = service_identity("wrong-purpose");
    let (mut transit_stream, _) = identity("wrong-purpose", SeedPurpose::Transit, TASK);
    let mut continuation = transit_stream.snapshot().restore().unwrap();

    assert!(matches!(
        distribution.sample(&mut transit_stream, &service_key),
        Err(WorkDurationError::WrongPurpose)
    ));
    assert_eq!(transit_stream.draw_position(), 0);
    assert_eq!(
        transit_stream.next_u64().unwrap(),
        continuation.next_u64().unwrap()
    );

    // A Service stream with a Transit expected key passes the purpose gate
    // and then fails the complete logical-identity comparison.
    let (_, transit_key) = identity("wrong-purpose", SeedPurpose::Transit, TASK);
    let mut service_stream = service_stream;
    let mut continuation = service_stream.snapshot().restore().unwrap();
    assert!(matches!(
        distribution.sample(&mut service_stream, &transit_key),
        Err(WorkDurationError::IdentityMismatch)
    ));
    assert_eq!(service_stream.draw_position(), 0);
    assert_eq!(
        service_stream.next_u64().unwrap(),
        continuation.next_u64().unwrap()
    );
}

#[test]
fn wrong_expected_task_is_identity_mismatch_without_stream_advance() {
    let distribution = IntrinsicDurationDistribution::weighted_ticks(vec![(4, 1), (9, 1)]).unwrap();
    let (mut stream, _) = service_identity("wrong-task");
    let (_, wrong_key) = identity("wrong-task", SeedPurpose::Service, "another-task");
    let mut continuation = stream.snapshot().restore().unwrap();

    assert!(matches!(
        distribution.sample(&mut stream, &wrong_key),
        Err(WorkDurationError::IdentityMismatch)
    ));
    assert_eq!(stream.draw_position(), 0);
    assert_eq!(stream.next_u64().unwrap(), continuation.next_u64().unwrap());
}

#[test]
fn fixed_distribution_also_validates_purpose_and_identity() {
    let distribution = IntrinsicDurationDistribution::fixed(5).unwrap();
    let (service_stream, service_key) = service_identity("fixed-invalid-identity");
    let (mut transit_stream, _) = identity("fixed-invalid-identity", SeedPurpose::Transit, TASK);
    let mut continuation = transit_stream.snapshot().restore().unwrap();
    assert!(matches!(
        distribution.sample(&mut transit_stream, &service_key),
        Err(WorkDurationError::WrongPurpose)
    ));
    assert_eq!(
        transit_stream.next_u64().unwrap(),
        continuation.next_u64().unwrap()
    );

    let (_, transit_key) = identity("fixed-invalid-identity", SeedPurpose::Transit, TASK);
    let mut service_stream = service_stream;
    let mut continuation = service_stream.snapshot().restore().unwrap();
    assert!(matches!(
        distribution.sample(&mut service_stream, &transit_key),
        Err(WorkDurationError::IdentityMismatch)
    ));
    assert_eq!(
        service_stream.next_u64().unwrap(),
        continuation.next_u64().unwrap()
    );

    let (mut stream, _) = service_identity("fixed-wrong-task");
    let (_, wrong_key) = identity("fixed-wrong-task", SeedPurpose::Service, "wrong-task");
    let mut continuation = stream.snapshot().restore().unwrap();
    assert!(matches!(
        distribution.sample(&mut stream, &wrong_key),
        Err(WorkDurationError::IdentityMismatch)
    ));
    assert_eq!(stream.next_u64().unwrap(), continuation.next_u64().unwrap());
}

#[test]
fn distribution_constructors_return_exact_typed_errors() {
    assert!(matches!(
        IntrinsicDurationDistribution::fixed(0),
        Err(WorkDurationError::ZeroDuration)
    ));
    assert!(matches!(
        IntrinsicDurationDistribution::weighted_ticks(vec![]),
        Err(WorkDurationError::EmptySupport)
    ));
    assert!(matches!(
        IntrinsicDurationDistribution::weighted_ticks(vec![(0, 0)]),
        Err(WorkDurationError::ZeroDuration)
    ));
    assert!(matches!(
        IntrinsicDurationDistribution::weighted_ticks(vec![(1, 0)]),
        Err(WorkDurationError::ZeroWeight)
    ));
    assert!(matches!(
        IntrinsicDurationDistribution::weighted_ticks(vec![(1, 1), (1, 2)]),
        Err(WorkDurationError::DuplicateDuration)
    ));
    assert!(matches!(
        IntrinsicDurationDistribution::weighted_ticks(vec![(1, u64::MAX), (2, 1)]),
        Err(WorkDurationError::WeightOverflow)
    ));
}

#[test]
fn provider_version_validation_precedes_strata_and_ids_are_fail_closed() {
    let fixed = || IntrinsicDurationDistribution::fixed(1).unwrap();
    assert!(matches!(
        IntrinsicWorkProvider::new(99, vec![]),
        Err(WorkDurationError::UnsupportedProviderVersion(99))
    ));
    assert!(matches!(
        IntrinsicWorkProvider::new(1, vec![]),
        Err(WorkDurationError::EmptySupport)
    ));
    assert!(matches!(
        IntrinsicWorkProvider::new(1, vec![("bad ".to_owned(), fixed())]),
        Err(WorkDurationError::InvalidStratum)
    ));
    assert!(matches!(
        IntrinsicWorkProvider::new(
            1,
            vec![
                ("duplicate".to_owned(), fixed()),
                ("duplicate".to_owned(), fixed()),
            ],
        ),
        Err(WorkDurationError::DuplicateStratum)
    ));
    assert!(matches!(
        IntrinsicWorkProvider::new(
            1,
            vec![
                ("duplicate".to_owned(), fixed()),
                ("duplicate".to_owned(), fixed()),
                ("late-invalid\n".to_owned(), fixed()),
            ],
        ),
        Err(WorkDurationError::InvalidStratum)
    ));
}

#[test]
fn malformed_and_missing_lookups_preserve_stream_and_do_not_alias() {
    let provider = IntrinsicWorkProvider::new(
        1,
        vec![(
            "triage".to_owned(),
            IntrinsicDurationDistribution::fixed(4).unwrap(),
        )],
    )
    .unwrap();
    let (mut stream, key) = service_identity("lookup-errors");
    let mut continuation = stream.snapshot().restore().unwrap();

    assert!(matches!(
        provider.sample(" unknown ", &mut stream, &key),
        Err(WorkDurationError::InvalidStratum)
    ));
    assert!(matches!(
        provider.sample("unknown", &mut stream, &key),
        Err(WorkDurationError::MissingStratum)
    ));
    assert_eq!(stream.draw_position(), 0);
    assert_eq!(stream.next_u64().unwrap(), continuation.next_u64().unwrap());
}

#[test]
fn provider_dispatches_exact_stratum_and_advances_only_on_success() {
    let provider = IntrinsicWorkProvider::new(
        1,
        vec![
            (
                "triage".to_owned(),
                IntrinsicDurationDistribution::fixed(4).unwrap(),
            ),
            (
                "procedure".to_owned(),
                IntrinsicDurationDistribution::fixed(9).unwrap(),
            ),
        ],
    )
    .unwrap();
    let (mut stream, key) = service_identity("provider-dispatch");
    let sample = provider.sample("procedure", &mut stream, &key).unwrap();

    assert_eq!(sample.duration(), SimDuration::from_ticks(9));
    assert_eq!(sample.draw_before(), 0);
    assert_eq!(sample.draw_after(), 0);
    assert_eq!(stream.draw_position(), 0);
}
