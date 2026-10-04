#[path = "../src/residuals.rs"]
mod residuals;

use residuals::{
    summarize, Counts, GroupKey, LogicalKey, OutcomeStatus, Residual, ResidualSign, Row, Side,
    SummaryStatus, Window,
};

fn key(case: &str) -> LogicalKey {
    LogicalKey {
        study_id: "study".into(),
        dataset_id: "dataset".into(),
        scenario_id: "scenario".into(),
        seed_schedule_id: "schedule".into(),
        replication_id: "rep-1".into(),
        case_key: case.into(),
        task_key: "task".into(),
        occurrence: 0,
        endpoint: "end".into(),
        seed_purpose: "service".into(),
        seed_map_ref: "map-v1".into(),
        mapping_version: "mapping-v1".into(),
    }
}
fn row(
    side: Side,
    case: &str,
    status: OutcomeStatus,
    observed: Option<u128>,
    predicted: Option<u128>,
) -> Row {
    Row {
        key: key(case),
        side,
        group: GroupKey::default(),
        source_time: Some(5),
        status,
        observed_ticks: observed,
        predicted_ticks: predicted,
        prediction_unclamped: side == Side::Simulation && predicted.is_some(),
        tick_unit: "nanosecond".into(),
        provenance_supported: true,
        excluded: false,
        infeasible: false,
    }
}
#[test]
fn exact_residual_covers_full_u128_range() {
    assert_eq!(
        Residual::between(u128::MAX, 0),
        Residual {
            sign: ResidualSign::Positive,
            magnitude: u128::MAX
        }
    );
    assert_eq!(
        Residual::between(0, u128::MAX),
        Residual {
            sign: ResidualSign::Negative,
            magnitude: u128::MAX
        }
    );
    assert_eq!(
        Residual::between(7, 7),
        Residual {
            sign: ResidualSign::Zero,
            magnitude: 0
        }
    );
}
#[test]
fn matched_rows_are_prediction_minus_observation_and_order_invariant() {
    let reference = row(
        Side::Reference,
        "b",
        OutcomeStatus::Observed,
        Some(10),
        None,
    );
    let simulation = row(
        Side::Simulation,
        "b",
        OutcomeStatus::Predicted,
        None,
        Some(14),
    );
    let out = summarize(&[simulation.clone(), reference.clone()], &[], None);
    assert_eq!(out, summarize(&[reference, simulation], &[], None));
    assert_eq!(
        out[0].rows[0].residual,
        Some(Residual {
            sign: ResidualSign::Positive,
            magnitude: 4
        })
    );
    assert_eq!(out[0].bias, Some(4.0));
    assert_eq!(out[0].mae, Some(4.0));
    assert_eq!(out[0].rmse, Some(4.0));
}
#[test]
fn window_is_half_open_on_selection_time() {
    let mut a = row(Side::Reference, "a", OutcomeStatus::Observed, Some(1), None);
    let mut b = row(Side::Reference, "b", OutcomeStatus::Observed, Some(1), None);
    let mut c = row(Side::Reference, "c", OutcomeStatus::Observed, Some(1), None);
    a.source_time = Some(4);
    b.source_time = Some(5);
    c.source_time = Some(10);
    let out = summarize(
        &[a, b, c],
        &[],
        Some(Window {
            start: 5,
            end_exclusive: 10,
        }),
    );
    assert_eq!(out[0].counts.excluded, 2);
    assert_eq!(out[0].counts.reference, 1);
}
#[test]
fn missing_selection_time_does_not_fall_back_to_prediction() {
    let mut reference = row(Side::Reference, "x", OutcomeStatus::Observed, Some(1), None);
    let simulation = row(
        Side::Simulation,
        "x",
        OutcomeStatus::Predicted,
        None,
        Some(5),
    );
    reference.source_time = None;
    let out = summarize(
        &[reference, simulation],
        &[],
        Some(Window {
            start: 0,
            end_exclusive: 9,
        }),
    );
    assert_eq!(out[0].counts.missing_time, 1);
    assert_eq!(out[0].rows[0].residual, None);
}
#[test]
fn duplicate_logical_side_key_is_invalid_not_deduplicated() {
    let a = row(Side::Reference, "x", OutcomeStatus::Observed, Some(1), None);
    let out = summarize(&[a.clone(), a], &[], None);
    assert_eq!(out[0].status, SummaryStatus::Invalid);
}
#[test]
fn statuses_and_diagnostics_are_preserved_and_overlap() {
    let mut r = row(Side::Reference, "x", OutcomeStatus::Censored, None, None);
    r.excluded = true;
    let out = summarize(&[r], &[], None);
    assert_eq!(out[0].rows[0].reference[0].status, OutcomeStatus::Censored);
    assert_eq!(out[0].counts.censored, 1);
    assert_eq!(out[0].counts.excluded, 1);
    assert_eq!(out[0].counts.unmatched, 1);
}
#[test]
fn unsupported_provenance_is_unverified_and_retained() {
    let mut r = row(Side::Reference, "x", OutcomeStatus::Observed, Some(1), None);
    r.provenance_supported = false;
    let out = summarize(&[r], &[], None);
    assert_eq!(out[0].status, SummaryStatus::Unverified);
    assert_eq!(out[0].bias, None);
    assert_eq!(out[0].rows.len(), 1);
}
#[test]
fn invalid_window_is_reported() {
    let out = summarize(
        &[],
        &[],
        Some(Window {
            start: 9,
            end_exclusive: 9,
        }),
    );
    assert_eq!(out[0].status, SummaryStatus::Invalid);
}
#[test]
fn prespecified_empty_group_is_emitted() {
    let group = GroupKey {
        strata: vec![("site".into(), "north".into())],
    };
    let out = summarize(&[], std::slice::from_ref(&group), None);
    assert_eq!(out[0].group, group);
    assert_eq!(out[0].status, SummaryStatus::Empty);
    assert_eq!(out[0].counts, Counts::default());
}

#[test]
fn infeasible_valid_prediction_remains_in_error_summary() {
    let reference = row(Side::Reference, "x", OutcomeStatus::Observed, Some(2), None);
    let mut simulation = row(
        Side::Simulation,
        "x",
        OutcomeStatus::Infeasible,
        None,
        Some(7),
    );
    simulation.infeasible = true;
    let out = summarize(&[reference, simulation], &[], None);
    assert_eq!(out[0].counts.infeasible, 1);
    assert_eq!(out[0].counts.matched, 1);
    assert_eq!(
        out[0].rows[0].residual,
        Some(Residual {
            sign: ResidualSign::Positive,
            magnitude: 5
        })
    );
}

#[test]
fn missing_censored_failed_and_infeasible_attempts_keep_diagnostics() {
    let missing = row(Side::Reference, "m", OutcomeStatus::Missing, None, None);
    let censored = row(Side::Reference, "c", OutcomeStatus::Censored, None, None);
    let failed = row(Side::Simulation, "f", OutcomeStatus::Failed, None, None);
    let infeasible = row(Side::Simulation, "i", OutcomeStatus::Infeasible, None, None);
    let out = summarize(&[missing, censored, failed, infeasible], &[], None);
    assert_eq!(out[0].counts.missing, 1);
    assert_eq!(out[0].counts.censored, 1);
    assert_eq!(out[0].counts.failed, 1);
    assert_eq!(out[0].counts.infeasible, 1);
    assert_eq!(out[0].rows.len(), 4);
}

#[test]
fn exact_float_values_above_two_to_53_do_not_raise_rounding_flag() {
    let reference = row(Side::Reference, "x", OutcomeStatus::Observed, Some(0), None);
    let simulation = row(
        Side::Simulation,
        "x",
        OutcomeStatus::Predicted,
        None,
        Some(1u128 << 54),
    );
    let out = summarize(&[reference, simulation], &[], None);
    assert!(!out[0].approximate);
}

#[test]
fn contradictory_status_and_tick_inputs_are_invalid_and_retained() {
    let bad = row(Side::Reference, "x", OutcomeStatus::Censored, Some(2), None);
    let out = summarize(&[bad], &[], None);
    assert_eq!(out[0].status, SummaryStatus::Invalid);
    assert_eq!(out[0].rows.len(), 1);
    assert_eq!(out[0].rows[0].reference[0].status, OutcomeStatus::Censored);
    assert_eq!(out[0].counts.censored, 1);
}

#[test]
fn unclamped_attestation_and_tick_unit_are_required() {
    let reference = row(Side::Reference, "x", OutcomeStatus::Observed, Some(1), None);
    let mut simulation = row(
        Side::Simulation,
        "x",
        OutcomeStatus::Predicted,
        None,
        Some(2),
    );
    simulation.prediction_unclamped = false;
    assert_eq!(
        summarize(&[reference.clone(), simulation.clone()], &[], None)[0].status,
        SummaryStatus::Invalid
    );
    simulation.prediction_unclamped = true;
    simulation.tick_unit = "tick".into();
    assert_eq!(
        summarize(&[reference, simulation], &[], None)[0].status,
        SummaryStatus::Invalid
    );
}

#[test]
fn logical_key_requires_all_identity_strings_and_known_purpose() {
    let mut bad = row(Side::Reference, "x", OutcomeStatus::Observed, Some(1), None);
    bad.key.study_id.clear();
    assert_eq!(
        summarize(&[bad], &[], None)[0].status,
        SummaryStatus::Invalid
    );
    let mut bad = row(Side::Reference, "x", OutcomeStatus::Observed, Some(1), None);
    bad.key.seed_purpose = "other".into();
    assert_eq!(
        summarize(&[bad], &[], None)[0].status,
        SummaryStatus::Invalid
    );
}

#[test]
fn cohort_mismatch_is_invalid_across_prespecified_groups() {
    let mut first = row(Side::Reference, "a", OutcomeStatus::Observed, Some(1), None);
    let mut second = row(Side::Reference, "b", OutcomeStatus::Observed, Some(1), None);
    first.group = GroupKey {
        strata: vec![("site".into(), "north".into())],
    };
    second.group = GroupKey {
        strata: vec![("site".into(), "south".into())],
    };
    second.key.seed_schedule_id = "other-schedule".into();
    let groups = [first.group.clone(), second.group.clone()];
    let out = summarize(&[first, second], &groups, None);
    assert!(out
        .iter()
        .all(|summary| summary.status == SummaryStatus::Invalid));
}

#[test]
fn source_selection_times_must_match_within_pair() {
    let reference = row(Side::Reference, "x", OutcomeStatus::Observed, Some(1), None);
    let mut simulation = row(
        Side::Simulation,
        "x",
        OutcomeStatus::Predicted,
        None,
        Some(2),
    );
    simulation.source_time = Some(6);
    assert_eq!(
        summarize(&[reference, simulation], &[], None)[0].status,
        SummaryStatus::Invalid
    );
}

#[test]
fn invalid_grouping_retains_raw_rows_and_counts() {
    let group = GroupKey {
        strata: vec![("site".into(), "north".into())],
    };
    let mut reference = row(Side::Reference, "x", OutcomeStatus::Observed, Some(1), None);
    reference.group = group.clone();
    let out = summarize(&[reference], &[], None);
    let invalid = out.iter().find(|summary| summary.group == group).unwrap();
    assert_eq!(invalid.status, SummaryStatus::Invalid);
    assert_eq!(invalid.counts.raw_reference, 1);
    assert_eq!(invalid.counts.unmatched, 1);
    assert_eq!(invalid.rows.len(), 1);
}

#[test]
fn invalid_middle_pair_retains_full_batch_rows_counts_and_order() {
    let mut rows = Vec::new();
    for case in ["a", "b", "c"] {
        rows.push(row(
            Side::Reference,
            case,
            OutcomeStatus::Observed,
            Some(10),
            None,
        ));
        let mut sim = row(
            Side::Simulation,
            case,
            OutcomeStatus::Predicted,
            None,
            Some(12),
        );
        if case == "b" {
            sim.observed_ticks = Some(11);
        }
        rows.push(sim);
    }
    let forward = summarize(&rows, &[], None);
    rows.reverse();
    let reverse = summarize(&rows, &[], None);
    assert_eq!(forward, reverse);
    assert_eq!(forward[0].status, SummaryStatus::Invalid);
    assert_eq!(forward[0].counts.raw_reference, 3);
    assert_eq!(forward[0].counts.raw_simulation, 3);
    assert_eq!(forward[0].counts.unmatched, 6);
    assert_eq!(forward[0].rows.len(), 3);
    assert!(forward[0]
        .rows
        .iter()
        .all(|pair| pair.reference.len() == 1 && pair.simulation.len() == 1));
}

#[test]
fn same_logical_key_in_different_groups_is_invalid_and_retained() {
    let mut reference = row(Side::Reference, "x", OutcomeStatus::Observed, Some(1), None);
    let mut simulation = row(
        Side::Simulation,
        "x",
        OutcomeStatus::Predicted,
        None,
        Some(2),
    );
    reference.group = GroupKey {
        strata: vec![("site".into(), "north".into())],
    };
    simulation.group = GroupKey {
        strata: vec![("site".into(), "south".into())],
    };
    let groups = [reference.group.clone(), simulation.group.clone()];
    let out = summarize(&[reference, simulation], &groups, None);
    assert_eq!(out.len(), 2);
    assert!(out
        .iter()
        .all(|summary| summary.status == SummaryStatus::Invalid));
    assert_eq!(
        out.iter().map(|summary| summary.rows.len()).sum::<usize>(),
        2
    );
    assert_eq!(
        out.iter()
            .map(|summary| summary.counts.unmatched)
            .sum::<usize>(),
        2
    );
}

#[test]
fn simulation_failed_or_infeasible_rows_cannot_carry_observed_ticks() {
    let mut failed = row(
        Side::Simulation,
        "failed",
        OutcomeStatus::Failed,
        Some(3),
        None,
    );
    let mut infeasible_without_prediction = row(
        Side::Simulation,
        "inf-none",
        OutcomeStatus::Infeasible,
        Some(3),
        None,
    );
    let mut infeasible_with_prediction = row(
        Side::Simulation,
        "inf-point",
        OutcomeStatus::Infeasible,
        Some(3),
        Some(4),
    );
    failed.prediction_unclamped = false;
    infeasible_without_prediction.prediction_unclamped = false;
    infeasible_with_prediction.prediction_unclamped = true;
    for item in [
        failed,
        infeasible_without_prediction,
        infeasible_with_prediction,
    ] {
        let out = summarize(&[item], &[], None);
        assert_eq!(out[0].status, SummaryStatus::Invalid);
        assert_eq!(out[0].counts.raw_simulation, 1);
        assert_eq!(out[0].rows.len(), 1);
    }
}

#[test]
fn bounded_large_paired_cohort_keeps_complete_counts() {
    const N: usize = 1024;
    let mut rows = Vec::with_capacity(N * 2);
    for i in 0..N {
        let case = format!("case-{i:04}");
        rows.push(row(
            Side::Reference,
            &case,
            OutcomeStatus::Observed,
            Some(i as u128),
            None,
        ));
        rows.push(row(
            Side::Simulation,
            &case,
            OutcomeStatus::Predicted,
            None,
            Some(i as u128 + 1),
        ));
    }
    let out = summarize(&rows, &[], None);
    assert_eq!(out[0].status, SummaryStatus::Computed);
    assert_eq!(out[0].counts.reference, N);
    assert_eq!(out[0].counts.simulation, N);
    assert_eq!(out[0].counts.matched, N);
    assert_eq!(out[0].rows.len(), N);
}

#[test]
fn unknown_row_group_is_retained_when_other_groups_are_declared() {
    let declared = GroupKey {
        strata: vec![("site".into(), "north".into())],
    };
    let unknown = GroupKey {
        strata: vec![("site".into(), "south".into())],
    };
    let mut reference = row(Side::Reference, "x", OutcomeStatus::Observed, Some(1), None);
    reference.group = unknown.clone();
    let out = summarize(&[reference], std::slice::from_ref(&declared), None);
    let invalid = out.iter().find(|summary| summary.group == unknown).unwrap();
    assert_eq!(invalid.status, SummaryStatus::Invalid);
    assert_eq!(invalid.counts.raw_reference, 1);
    assert_eq!(invalid.rows.len(), 1);
}
