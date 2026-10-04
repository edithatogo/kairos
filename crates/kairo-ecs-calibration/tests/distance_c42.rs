#[path = "../src/metrics.rs"]
mod metrics;

use metrics::{compare, MetricRequest, MetricStatus, Precision};

fn req<'a>(
    reference: &'a [Option<&'a str>],
    simulation: &'a [Option<&'a str>],
) -> MetricRequest<'a> {
    MetricRequest {
        reference,
        simulation,
        reference_weights: None,
        simulation_weights: None,
        algorithm_version: "empirical_equal.v1",
        origin: None,
        scale_ticks: "1",
    }
}

#[test]
fn identical_populations_compute_zero_distances() {
    let result = compare(&req(&[Some("0"), Some("1")], &[Some("0"), Some("1")]));
    assert_eq!(result.status, MetricStatus::Computed);
    assert_eq!(result.w1, Some(0.0));
    assert_eq!(result.ks_d, Some(0.0));
    assert_eq!((result.reference_count, result.simulation_count), (2, 2));
    assert_eq!(result.precision, Precision::NotApplicable);
}

#[test]
fn ties_are_aggregated_before_cdf_updates() {
    let result = compare(&req(
        &[Some("0"), Some("0"), Some("2")],
        &[Some("0"), Some("1"), Some("1")],
    ));
    assert_eq!(result.status, MetricStatus::Computed);
    assert_eq!(result.w1, Some(2.0 / 3.0));
    assert_eq!(result.ks_d, Some(1.0 / 3.0));
}

#[test]
fn signed_rational_supports_are_exactly_ordered() {
    let result = compare(&req(
        &[Some("-1/2"), Some("1/2")],
        &[Some("0"), Some("1/3")],
    ));
    assert_eq!(result.status, MetricStatus::Computed);
    assert_eq!(result.ks_d, Some(0.5));
}

#[test]
fn both_empty_populations_are_empty() {
    let result = compare(&req(&[], &[]));
    assert_eq!(result.status, MetricStatus::Empty);
    assert_eq!((result.w1, result.ks_d), (None, None));
}

#[test]
fn checked_arithmetic_overflow_is_invalid() {
    let huge_denominator = "340282366920938463463374607431768211455";
    let value = format!("1/{huge_denominator}");
    let result = compare(&req(&[Some("0"), Some(&value)], &[Some("0"), Some("0")]));
    assert_eq!(result.status, MetricStatus::Invalid);
    assert_eq!((result.reference_count, result.simulation_count), (0, 0));
    assert_eq!((result.w1, result.ks_d), (None, None));
}

#[test]
fn weighted_descriptive_uses_exact_per_side_normalization() {
    let reference = [Some("0"), Some("2")];
    let simulation = [Some("1"), Some("3")];
    let request = MetricRequest {
        reference: &reference,
        simulation: &simulation,
        reference_weights: Some(&["1", "3"]),
        simulation_weights: Some(&["2", "2"]),
        algorithm_version: "weighted_descriptive.v1",
        origin: None,
        scale_ticks: "1",
    };
    let result = compare(&request);
    assert_eq!(result.status, MetricStatus::Computed);
    assert_eq!(result.w1, Some(1.0));
    assert_eq!(result.ks_d, Some(0.5));
    assert_eq!((result.reference_count, result.simulation_count), (2, 2));
}

#[test]
fn null_aligned_zero_weight_stays_in_count_and_adds_no_mass() {
    let reference = [Some("0"), Some("2")];
    let simulation = [Some("0"), None];
    let request = MetricRequest {
        reference: &reference,
        simulation: &simulation,
        reference_weights: Some(&["1", "0"]),
        simulation_weights: Some(&["2", "not-a-weight"]),
        algorithm_version: "weighted_descriptive.v1",
        origin: None,
        scale_ticks: "1",
    };
    let result = compare(&request);
    assert_eq!(result.status, MetricStatus::Invalid);
    assert_eq!((result.reference_count, result.simulation_count), (0, 0));
}

#[test]
fn exact_common_origin_offsets_are_checked_before_conversion() {
    let origin = "9007199254740992";
    let reference = [Some(origin), Some("9007199254740994")];
    let simulation = [Some("9007199254740993"), Some("9007199254740995")];
    let request = MetricRequest {
        reference: &reference,
        simulation: &simulation,
        reference_weights: None,
        simulation_weights: None,
        algorithm_version: "empirical_equal.v1",
        origin: Some(origin),
        scale_ticks: "1",
    };
    let result = compare(&request);
    assert_eq!(result.status, MetricStatus::Computed);
    assert_eq!(result.precision, Precision::ExactOffsets);
    assert_eq!(result.w1, Some(1.0));

    let rejected = MetricRequest {
        reference: &[Some("0")],
        simulation: &[Some("9007199254740993")],
        origin: Some("0"),
        ..request
    };
    let result = compare(&rejected);
    assert_eq!(result.status, MetricStatus::Invalid);
    assert_eq!(result.precision, Precision::Rejected);
    assert_eq!((result.reference_count, result.simulation_count), (0, 0));
}

#[test]
fn scale_and_algorithm_weight_contract_are_enforced() {
    let mut request = req(&[Some("0")], &[Some("1")]);
    request.scale_ticks = "0";
    assert_eq!(compare(&request).status, MetricStatus::Invalid);
    request.scale_ticks = "1";
    request.reference_weights = Some(&["1"]);
    assert_eq!(compare(&request).status, MetricStatus::Invalid);
}

#[test]
#[ignore = "bounded synthetic distance timing receipt; coordinator executes once"]
fn deterministic_100k_point_shift_timing() {
    use std::time::Instant;
    const N: usize = 100_000;
    let reference_storage: Vec<String> = (0..N).map(|i| i.to_string()).collect();
    let simulation_storage: Vec<String> = (1..=N).map(|i| i.to_string()).collect();
    let reference: Vec<Option<&str>> = reference_storage.iter().map(|s| Some(s.as_str())).collect();
    let simulation: Vec<Option<&str>> = simulation_storage
        .iter()
        .map(|s| Some(s.as_str()))
        .collect();
    let request = MetricRequest {
        reference: &reference,
        simulation: &simulation,
        reference_weights: None,
        simulation_weights: None,
        algorithm_version: "empirical_equal.v1",
        origin: None,
        scale_ticks: "1",
    };
    let start = Instant::now();
    let result = compare(&request);
    let elapsed = start.elapsed();
    assert_eq!(result.status, MetricStatus::Computed);
    assert_eq!(result.w1, Some(1.0));
    assert_eq!(result.ks_d, Some(1.0 / N as f64));
    println!(
        "bounded synthetic points_per_side={N} elapsed_ms={}",
        elapsed.as_millis()
    );
}

#[test]
fn tied_weighted_support_permutation_is_bitwise_stable() {
    let a_ref = [Some("0"), Some("0"), Some("1")];
    let a_sim = [Some("0"), Some("1"), Some("1")];
    let b_ref = [Some("0"), Some("1"), Some("0")];
    let b_sim = [Some("1"), Some("0"), Some("1")];
    let a = compare(&MetricRequest {
        reference: &a_ref,
        simulation: &a_sim,
        reference_weights: Some(&["1/3", "1/2", "1/7"]),
        simulation_weights: Some(&["1/5", "1/3", "1/2"]),
        algorithm_version: "weighted_descriptive.v1",
        origin: None,
        scale_ticks: "1",
    });
    let b = compare(&MetricRequest {
        reference: &b_ref,
        simulation: &b_sim,
        reference_weights: Some(&["1/2", "1/7", "1/3"]),
        simulation_weights: Some(&["1/2", "1/5", "1/3"]),
        algorithm_version: "weighted_descriptive.v1",
        origin: None,
        scale_ticks: "1",
    });
    assert_eq!(a.status, MetricStatus::Computed);
    assert_eq!(b.status, MetricStatus::Computed);
    assert_eq!(a.w1, b.w1);
    assert_eq!(a.ks_d, b.ks_d);
}

#[test]
fn positive_rational_tick_scale_is_supported_exactly() {
    let reference = [Some("100"), Some("103")];
    let simulation = [Some("101"), Some("104")];
    let request = MetricRequest {
        reference: &reference,
        simulation: &simulation,
        reference_weights: None,
        simulation_weights: None,
        algorithm_version: "empirical_equal.v1",
        origin: Some("100"),
        scale_ticks: "3/2",
    };
    let result = compare(&request);
    assert_eq!(result.status, MetricStatus::Computed);
    assert_eq!(result.precision, Precision::ExactOffsets);
    assert_eq!(result.w1, Some(2.0 / 3.0));
    assert_eq!(result.ks_d, Some(0.5));
}
