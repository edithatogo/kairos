#![allow(dead_code)]

#[path = "../src/residuals.rs"]
mod residuals;
mod seed_map {
    // The report tests exercise projection only; this opaque key stands in for
    // the already-admitted key owned by the shared seed-map module.
    #[derive(Clone, Eq, PartialEq)]
    pub(crate) struct CalibrationStreamKey;
}
#[path = "../src/shadow.rs"]
mod shadow;
#[path = "../src/shadow_report.rs"]
mod shadow_report;
#[path = "../src/trace_order.rs"]
mod trace_order;

use residuals::{GroupKey, LogicalKey, OutcomeStatus, ResidualSign, Side};
use shadow::{
    EvaluationPolicy, LimitReason, ProbeBudget, ProbeInput, ProbeOutcome, ProbeResult, ProbeSpec,
};
use shadow_report::{evaluate, project_rows};

fn result(id: &str, observed: Option<u128>, outcome: ProbeOutcome) -> ProbeResult {
    let last_tick = match &outcome {
        ProbeOutcome::Completed { predicted } => *predicted,
        _ => 10,
    };
    ProbeResult {
        id: id.into(),
        anchor_event: "anchor-1".into(),
        frontier: 1,
        snapshot_digest: [7; 32],
        observed,
        outcome,
        events: 1,
        last_tick,
        assumptions: vec!["initial occupancy declared".into()],
    }
}

fn logical_key(case_key: &str) -> LogicalKey {
    LogicalKey {
        study_id: "study".into(),
        dataset_id: "dataset".into(),
        scenario_id: "scenario".into(),
        seed_schedule_id: "schedule".into(),
        replication_id: "rep-1".into(),
        case_key: case_key.into(),
        task_key: "task".into(),
        occurrence: 0,
        endpoint: "departure".into(),
        seed_purpose: "service".into(),
        seed_map_ref: "map-v1".into(),
        mapping_version: "mapping-v1".into(),
    }
}

fn spec(id: &str, observed: Option<u128>) -> ProbeSpec {
    ProbeSpec {
        id: id.into(),
        key: logical_key(id),
        run_id: "run-1".into(),
        candidate_id: "candidate-1".into(),
        anchor_event: "anchor-1".into(),
        target_event: Some("target-1".into()),
        observed_target: observed,
        input: ProbeInput {
            target: "departure".into(),
            seed_key: seed_map::CalibrationStreamKey,
            parameter_hash: [1; 32],
            adapter_hash: [2; 32],
            fidelity: "Micro".into(),
        },
        budget: ProbeBudget {
            horizon: 100,
            max_events: 20,
        },
    }
}

fn strict(numerator: u64, denominator: u64) -> EvaluationPolicy {
    EvaluationPolicy::Strict {
        max_late_numerator: numerator,
        max_late_denominator: denominator,
    }
}

#[test]
fn projects_exact_signed_residual_at_u128_extremes() {
    let out = evaluate(
        &[
            result(
                "z-late",
                Some(0),
                ProbeOutcome::Completed {
                    predicted: u128::MAX,
                },
            ),
            result(
                "a-early",
                Some(u128::MAX),
                ProbeOutcome::Completed { predicted: 0 },
            ),
        ],
        EvaluationPolicy::Diagnostic,
    )
    .unwrap();
    assert_eq!(out.records[0].probe_id, "a-early");
    assert_eq!(
        out.records[0].residual.unwrap().sign,
        ResidualSign::Negative
    );
    assert_eq!(out.records[0].residual.unwrap().magnitude, u128::MAX);
    assert_eq!(
        out.records[1].residual.unwrap().sign,
        ResidualSign::Positive
    );
    assert_eq!(out.records[1].residual.unwrap().magnitude, u128::MAX);
    assert_eq!(out.counts.paired, 2);
    assert_eq!(out.counts.late, 1);
}

#[test]
fn counts_all_dispositions_and_applies_strict_denominators() {
    let values = vec![
        result("late", Some(10), ProbeOutcome::Completed { predicted: 11 }),
        result(
            "on-time",
            Some(10),
            ProbeOutcome::Completed { predicted: 10 },
        ),
        result(
            "missing-observation",
            None,
            ProbeOutcome::Completed { predicted: 8 },
        ),
        result("missing-prediction", Some(10), ProbeOutcome::Missing),
        result(
            "censored",
            Some(10),
            ProbeOutcome::Censored {
                reason: LimitReason::TickHorizon,
            },
        ),
        result(
            "failed",
            Some(10),
            ProbeOutcome::Failed {
                reason: "native".into(),
            },
        ),
        result(
            "infeasible",
            Some(10),
            ProbeOutcome::Infeasible {
                reason: "capacity".into(),
            },
        ),
    ];
    let diagnostic = evaluate(&values, EvaluationPolicy::Diagnostic).unwrap();
    assert_eq!(diagnostic.counts.total, 7);
    assert_eq!(diagnostic.counts.paired, 2);
    assert_eq!(diagnostic.counts.late, 1);
    assert_eq!(diagnostic.counts.missing_observation, 1);
    assert_eq!(diagnostic.counts.missing_prediction, 4);
    assert_eq!(diagnostic.counts.censored, 1);
    assert_eq!(diagnostic.counts.failed, 1);
    assert_eq!(diagnostic.counts.infeasible, 1); // resource infeasibility; late has its own count
    assert!(diagnostic.accepted);
    assert!(!evaluate(&values, strict(1, 2)).unwrap().accepted);

    let paired = [
        result("late", Some(10), ProbeOutcome::Completed { predicted: 11 }),
        result(
            "on-time",
            Some(10),
            ProbeOutcome::Completed { predicted: 10 },
        ),
    ];
    assert!(evaluate(&paired, strict(1, 2)).unwrap().accepted);
    assert!(!evaluate(&paired, strict(0, 1)).unwrap().accepted);
    assert!(!evaluate(&[], strict(0, 1)).unwrap().accepted);
}

#[test]
fn rejects_duplicate_ids_and_invalid_policy_or_outcome() {
    let duplicate = [
        result("same", Some(1), ProbeOutcome::Completed { predicted: 1 }),
        result("same", Some(1), ProbeOutcome::Completed { predicted: 1 }),
    ];
    assert!(evaluate(&duplicate, EvaluationPolicy::Diagnostic).is_err());
    assert!(evaluate(&[], strict(1, 0)).is_err());
    assert!(evaluate(&[], strict(2, 1)).is_err());
    let invalid = [result(
        "bad",
        None,
        ProbeOutcome::Failed {
            reason: "  ".into(),
        },
    )];
    assert!(evaluate(&invalid, EvaluationPolicy::Diagnostic).is_err());
    let mut mismatched_tick = result(
        "bad-tick",
        Some(1),
        ProbeOutcome::Completed { predicted: 2 },
    );
    mismatched_tick.last_tick = 1;
    assert!(evaluate(&[mismatched_tick], EvaluationPolicy::Diagnostic).is_err());
}

#[test]
fn projects_paired_c4_rows_with_replay_role_and_statuses() {
    let results = [
        result("point", Some(10), ProbeOutcome::Completed { predicted: 12 }),
        result("missing", None, ProbeOutcome::Missing),
        result(
            "censored",
            Some(10),
            ProbeOutcome::Censored {
                reason: LimitReason::EventBudget,
            },
        ),
        result(
            "infeasible",
            Some(10),
            ProbeOutcome::Infeasible {
                reason: "capacity".into(),
            },
        ),
    ];
    let specs = [
        spec("point", Some(10)),
        spec("missing", None),
        spec("censored", Some(10)),
        spec("infeasible", Some(10)),
    ];
    let rows = project_rows(&results, &specs).unwrap();
    assert_eq!(rows.len(), 8);
    assert!(rows.iter().all(|row| row.group
        == GroupKey {
            strata: vec![("replay_role".into(), "ShadowAnchored".into())]
        }));
    let row_for = |case: &str, side| {
        rows.iter()
            .find(|row| row.key.case_key == case && row.side == side)
            .unwrap()
    };
    let point = row_for("point", Side::Simulation);
    assert_eq!(point.status, OutcomeStatus::Infeasible);
    assert_eq!(point.predicted_ticks, Some(12));
    assert!(point.prediction_unclamped && point.infeasible);
    assert_eq!(
        row_for("missing", Side::Simulation).status,
        OutcomeStatus::Missing
    );
    assert_eq!(
        row_for("censored", Side::Simulation).status,
        OutcomeStatus::Censored
    );
    assert_eq!(
        row_for("infeasible", Side::Simulation).status,
        OutcomeStatus::Infeasible
    );
    assert_eq!(
        row_for("point", Side::Reference).status,
        OutcomeStatus::Observed
    );
    let group = rows[0].group.clone();
    let summaries = residuals::summarize(&rows, &[group], None);
    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].status, residuals::SummaryStatus::Computed);
    assert_eq!(summaries[0].counts.matched, 1);
    assert_eq!(summaries[0].counts.missing, 2); // one missing source plus one missing prediction
    assert_eq!(summaries[0].counts.censored, 1);
    assert_eq!(summaries[0].counts.infeasible, 2);
    let point = summaries[0]
        .rows
        .iter()
        .find(|row| row.key.case_key == "point")
        .unwrap();
    let residual = point.residual.unwrap();
    assert_eq!(residual.sign, ResidualSign::Positive);
    assert_eq!(residual.magnitude, 2);
    let reordered = project_rows(&results.into_iter().rev().collect::<Vec<_>>(), &specs).unwrap();
    assert_eq!(rows, reordered);
}

#[test]
fn rejects_unbound_or_inconsistent_projection_metadata() {
    let probe_result = result("probe", Some(10), ProbeOutcome::Completed { predicted: 10 });
    assert!(project_rows(std::slice::from_ref(&probe_result), &[]).is_err());
    let mut wrong_observed = spec("probe", Some(9));
    assert!(project_rows(
        std::slice::from_ref(&probe_result),
        std::slice::from_ref(&wrong_observed)
    )
    .is_err());
    wrong_observed.observed_target = Some(10);
    wrong_observed.input.target = "other-endpoint".into();
    assert!(project_rows(&[probe_result], &[wrong_observed]).is_err());

    let first = spec("first", Some(10));
    let mut duplicate_key = spec("second", Some(10));
    duplicate_key.key = first.key.clone();
    assert!(project_rows(
        &[
            result("first", Some(10), ProbeOutcome::Completed { predicted: 10 }),
            result(
                "second",
                Some(10),
                ProbeOutcome::Completed { predicted: 10 }
            ),
        ],
        &[first, duplicate_key]
    )
    .is_err());

    let mut over_budget = result("probe", Some(10), ProbeOutcome::Completed { predicted: 10 });
    over_budget.events = 21;
    assert!(project_rows(&[over_budget], &[spec("probe", Some(10))]).is_err());

    let mut invalid_key = spec("probe", Some(10));
    invalid_key.key.task_key.clear();
    assert!(project_rows(
        &[result(
            "probe",
            Some(10),
            ProbeOutcome::Completed { predicted: 10 }
        )],
        &[invalid_key]
    )
    .is_err());
}

#[test]
fn fidelity_is_independent_of_replay_role() {
    let outcome = result("p", Some(10), ProbeOutcome::Completed { predicted: 9 });
    for fidelity in ["Macro", "Micro"] {
        let mut binding = spec("p", Some(10));
        binding.input.fidelity = fidelity.into();
        let rows = project_rows(std::slice::from_ref(&outcome), &[binding]).unwrap();
        assert_eq!(
            rows[0].group.strata,
            vec![("replay_role".into(), "ShadowAnchored".into())]
        );
    }
    let mut binding = spec("p", Some(10));
    binding.input.fidelity = "ShadowAnchored".into();
    assert!(project_rows(&[outcome], &[binding]).is_err());
}
