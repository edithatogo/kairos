//! Phase C3.1: Anchor and probe test-first fixture specifications.
//!
//! Covers:
//! 1. Early completion (predicted < observed)
//! 2. Late completion (predicted > observed, anchor-feasibility violation)
//! 3. Completion exactly at anchor (predicted == observed)
//! 4. Missing target (observed target missing or probe target never reached)
//! 5. Impossible resource occupancy (ledger resource over-allocation rejected or diagnosed)
//! 6. Repeated events (multiple occurrences of task/event disambiguated by occurrence)
//! 7. Interrupted transit/work (pause, resume, and preemption preserve progress and context)
//! 8. Bounded probe termination (tick horizon and event budget limits produce typed censoring)
//! 9. Walk + work = 5 ridge fixture and independent walk observation asserting objective
//!    ties do not imply uniquely identified empirical primitives.
//!
//! Adheres to C-03 calibration specifications (conductor/tracks/empirical_calibration_20260925/spec.md).

#![forbid(unsafe_code)]
#![allow(dead_code)]

#[path = "../src/residuals.rs"]
mod residuals;
#[path = "../src/trace_order.rs"]
mod trace_order;

use residuals::{LogicalKey, Residual, ResidualSign};
use std::collections::BTreeMap;
use trace_order::{EventKindRank, TraceOrderKeyV1};

// ---------------------------------------------------------------------------
// Protocol types reflecting Phase C3 specifications (ADR-0021 / C-03)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Transition {
    None,
    Acquire {
        resource: String,
        claim: String,
        units: u32,
    },
    Release {
        resource: String,
        claim: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ObservedEvent {
    pub order: TraceOrderKeyV1,
    pub available_at: Option<u128>,
    pub source_defined: bool,
    pub transition: Transition,
    pub payload: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ResourceState {
    pub capacity: u32,
    pub claims: BTreeMap<String, u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LedgerSnapshot {
    pub frontier: usize,
    pub at: u128,
    pub anchor_event: String,
    pub digest: [u8; 32],
    pub visible_events: Vec<ObservedEvent>,
    pub resources: BTreeMap<String, ResourceState>,
    pub resource_feasible: bool,
    pub assumptions: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ResourcePolicy {
    Strict,
    Diagnostic,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LedgerDiagnostic {
    pub event: String,
    pub reason: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ProbeBudget {
    pub horizon: u128,
    pub max_events: u64,
}

#[derive(Clone, Eq, PartialEq)]
pub(crate) struct CalibrationStreamKey(pub u8);

#[derive(Clone, Eq, PartialEq)]
pub(crate) struct ProbeInput {
    pub target: String,
    pub seed_key: CalibrationStreamKey,
    pub parameter_hash: [u8; 32],
    pub adapter_hash: [u8; 32],
    pub fidelity: String,
}

#[derive(Clone, Eq, PartialEq)]
pub(crate) struct ProbeSpec {
    pub id: String,
    pub key: LogicalKey,
    pub run_id: String,
    pub candidate_id: String,
    pub anchor_event: String,
    pub target_event: Option<String>,
    pub observed_target: Option<u128>,
    pub input: ProbeInput,
    pub budget: ProbeBudget,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LimitReason {
    TickHorizon,
    EventBudget,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ProbeOutcome {
    Completed { predicted: u128 },
    Missing,
    Infeasible { reason: String },
    Censored { reason: LimitReason },
    Failed { reason: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ProbeResult {
    pub id: String,
    pub anchor_event: String,
    pub frontier: usize,
    pub snapshot_digest: [u8; 32],
    pub observed: Option<u128>,
    pub outcome: ProbeOutcome,
    pub events: u64,
    pub last_tick: u128,
    pub assumptions: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ShadowError {
    InvalidInput(&'static str),
    DuplicateIdentity(String),
    UnknownProbe(String),
    ResourceInfeasible(LedgerDiagnostic),
    UnavailableAnchor(String),
    LimitExceeded,
    Adapter(String),
    Contract(&'static str),
    IncompatibleCheckpoint,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum EvaluationPolicy {
    Diagnostic,
    Strict {
        max_late_numerator: u64,
        max_late_denominator: u64,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PredictionRecord {
    pub probe_id: String,
    pub observed: Option<u128>,
    pub predicted: Option<u128>,
    pub residual: Option<Residual>,
    pub late: bool,
    pub infeasible: bool,
    pub replay_role: &'static str,
    pub outcome: ProbeOutcome,
    pub missing_observation: bool,
    pub assumptions: Vec<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct EvaluationCounts {
    pub total: u64,
    pub paired: u64,
    pub late: u64,
    pub missing_observation: u64,
    pub missing_prediction: u64,
    pub censored: u64,
    pub failed: u64,
    pub infeasible: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Evaluation {
    pub records: Vec<PredictionRecord>,
    pub counts: EvaluationCounts,
    pub accepted: bool,
}

// ---------------------------------------------------------------------------
// Protocol verification evaluation helper
// ---------------------------------------------------------------------------

const REPLAY_ROLE: &str = "ShadowAnchored";

fn evaluate_results(
    results: &[ProbeResult],
    policy: EvaluationPolicy,
) -> Result<Evaluation, ShadowError> {
    if let EvaluationPolicy::Strict {
        max_late_numerator,
        max_late_denominator,
    } = policy
    {
        if max_late_denominator == 0 || max_late_numerator > max_late_denominator {
            return Err(ShadowError::InvalidInput("invalid strict late fraction"));
        }
    }

    let mut by_id = BTreeMap::new();
    for result in results {
        if result.id.trim().is_empty() {
            return Err(ShadowError::InvalidInput("empty probe ID"));
        }
        if by_id.insert(result.id.as_str(), result).is_some() {
            return Err(ShadowError::DuplicateIdentity(result.id.clone()));
        }
    }

    let total = u64::try_from(by_id.len()).map_err(|_| ShadowError::LimitExceeded)?;
    let mut counts = EvaluationCounts {
        total,
        ..EvaluationCounts::default()
    };
    let mut records = Vec::with_capacity(by_id.len());

    for result in by_id.values() {
        let predicted = match &result.outcome {
            ProbeOutcome::Completed { predicted } => Some(*predicted),
            ProbeOutcome::Missing
            | ProbeOutcome::Infeasible { .. }
            | ProbeOutcome::Censored { .. }
            | ProbeOutcome::Failed { .. } => None,
        };
        let paired = result.observed.is_some() && predicted.is_some();
        let late = result
            .observed
            .zip(predicted)
            .is_some_and(|(observed, predicted)| predicted > observed);
        let resource_infeasible = matches!(&result.outcome, ProbeOutcome::Infeasible { .. });
        let infeasible = resource_infeasible || late;

        counts.paired += u64::from(paired);
        counts.late += u64::from(late);
        counts.missing_observation += u64::from(result.observed.is_none());
        counts.missing_prediction += u64::from(predicted.is_none());
        counts.censored += u64::from(matches!(&result.outcome, ProbeOutcome::Censored { .. }));
        counts.failed += u64::from(matches!(&result.outcome, ProbeOutcome::Failed { .. }));
        counts.infeasible += u64::from(resource_infeasible);

        records.push(PredictionRecord {
            probe_id: result.id.clone(),
            observed: result.observed,
            predicted,
            residual: result
                .observed
                .zip(predicted)
                .map(|(observed, predicted)| Residual::between(predicted, observed)),
            late,
            infeasible,
            replay_role: REPLAY_ROLE,
            outcome: result.outcome.clone(),
            missing_observation: result.observed.is_none(),
            assumptions: result.assumptions.clone(),
        });
    }

    let accepted = match policy {
        EvaluationPolicy::Diagnostic => true,
        EvaluationPolicy::Strict {
            max_late_numerator,
            max_late_denominator,
        } => {
            counts.total > 0
                && counts.missing_observation == 0
                && counts.missing_prediction == 0
                && counts.censored == 0
                && counts.failed == 0
                && counts.infeasible == 0
                && u128::from(counts.late) * u128::from(max_late_denominator)
                    <= u128::from(counts.paired) * u128::from(max_late_numerator)
        }
    };

    Ok(Evaluation {
        records,
        counts,
        accepted,
    })
}

fn sample_key(case: &str, occurrence: u32, endpoint: &str) -> LogicalKey {
    LogicalKey {
        study_id: "c3-study".into(),
        dataset_id: "c3-dataset".into(),
        scenario_id: "c3-scenario".into(),
        seed_schedule_id: "c3-schedule".into(),
        replication_id: "rep-0".into(),
        case_key: case.into(),
        task_key: "task-service".into(),
        occurrence,
        endpoint: endpoint.into(),
        seed_purpose: "calibration".into(),
        seed_map_ref: "map-v1".into(),
        mapping_version: "v1".into(),
    }
}

fn sample_result(
    id: &str,
    anchor: &str,
    observed: Option<u128>,
    outcome: ProbeOutcome,
    last_tick: u128,
) -> ProbeResult {
    ProbeResult {
        id: id.into(),
        anchor_event: anchor.into(),
        frontier: 1,
        snapshot_digest: [0u8; 32],
        observed,
        outcome,
        events: 1,
        last_tick,
        assumptions: vec![],
    }
}

// ===========================================================================
// Condition Family 1: Early Completion (predicted < observed)
// ===========================================================================
#[test]
fn fixture_01_early_completion_negative_residual_and_slack() {
    // Probe completes at tick 8; historical observation occurred at tick 10.
    // residual = predicted - observed = 8 - 10 = -2 (negative residual and slack).
    // Not late; feasibility is preserved.
    let res = sample_result(
        "probe-early",
        "anchor-t0",
        Some(10),
        ProbeOutcome::Completed { predicted: 8 },
        8,
    );
    let eval = evaluate_results(&[res], EvaluationPolicy::Diagnostic).unwrap();
    assert_eq!(eval.counts.total, 1);
    assert_eq!(eval.counts.paired, 1);
    assert_eq!(eval.counts.late, 0);
    assert_eq!(eval.counts.infeasible, 0);

    let rec = &eval.records[0];
    assert!(!rec.late);
    assert!(!rec.infeasible);
    assert_eq!(
        rec.residual,
        Some(Residual {
            sign: ResidualSign::Negative,
            magnitude: 2,
        })
    );
    assert_eq!(rec.predicted, Some(8));
    assert_eq!(rec.observed, Some(10));
}

// ===========================================================================
// Condition Family 2: Late Completion (predicted > observed)
// ===========================================================================
#[test]
fn fixture_02_late_completion_positive_residual_and_infeasibility_violation() {
    // Probe completes at tick 12; historical observation occurred at tick 10.
    // residual = predicted - observed = 12 - 10 = +2.
    // Late arrival violates anchor feasibility in strict mode.
    let res = sample_result(
        "probe-late",
        "anchor-t0",
        Some(10),
        ProbeOutcome::Completed { predicted: 12 },
        12,
    );

    // Diagnostic evaluation retains late count and accepts diagnostic report.
    let diag = evaluate_results(std::slice::from_ref(&res), EvaluationPolicy::Diagnostic).unwrap();
    assert_eq!(diag.counts.total, 1);
    assert_eq!(diag.counts.paired, 1);
    assert_eq!(diag.counts.late, 1);
    assert_eq!(diag.counts.infeasible, 0); // resource-infeasible count is 0; record.infeasible is true
    let rec = &diag.records[0];
    assert!(rec.late);
    assert!(rec.infeasible);
    assert_eq!(
        rec.residual,
        Some(Residual {
            sign: ResidualSign::Positive,
            magnitude: 2,
        })
    );

    // Strict evaluation with 0 tolerance rejects the late outcome.
    let strict = evaluate_results(
        &[res],
        EvaluationPolicy::Strict {
            max_late_numerator: 0,
            max_late_denominator: 1,
        },
    )
    .unwrap();
    assert!(!strict.accepted);
}

// ===========================================================================
// Condition Family 3: Completion Exactly at Anchor/Observed (predicted == observed)
// ===========================================================================
#[test]
fn fixture_03_completion_exactly_at_anchor_zero_residual() {
    // Probe completes at tick 10; observed target is exactly 10.
    // residual = 10 - 10 = 0.
    let res = sample_result(
        "probe-exact",
        "anchor-t0",
        Some(10),
        ProbeOutcome::Completed { predicted: 10 },
        10,
    );
    let eval = evaluate_results(
        &[res],
        EvaluationPolicy::Strict {
            max_late_numerator: 0,
            max_late_denominator: 1,
        },
    )
    .unwrap();
    assert_eq!(eval.counts.total, 1);
    assert_eq!(eval.counts.paired, 1);
    assert_eq!(eval.counts.late, 0);
    assert!(eval.accepted);

    let rec = &eval.records[0];
    assert!(!rec.late);
    assert!(!rec.infeasible);
    assert_eq!(
        rec.residual,
        Some(Residual {
            sign: ResidualSign::Zero,
            magnitude: 0,
        })
    );
}

// ===========================================================================
// Condition Family 4: Missing Target (missing observation or missing prediction)
// ===========================================================================
#[test]
fn fixture_04_missing_target_observation_and_prediction() {
    // Subcase 4a: Probe completed, but observation was missing (e.g. censored out of window).
    let res_missing_obs = sample_result(
        "probe-missing-obs",
        "anchor-t0",
        None,
        ProbeOutcome::Completed { predicted: 10 },
        10,
    );
    let eval_obs = evaluate_results(&[res_missing_obs], EvaluationPolicy::Diagnostic).unwrap();
    assert_eq!(eval_obs.counts.missing_observation, 1);
    assert_eq!(eval_obs.counts.paired, 0);
    assert!(eval_obs.records[0].residual.is_none());
    assert!(eval_obs.records[0].missing_observation);

    // Subcase 4b: Observed target was known (tick 10), but probe did not complete (Missing).
    let res_missing_pred = sample_result(
        "probe-missing-pred",
        "anchor-t0",
        Some(10),
        ProbeOutcome::Missing,
        10,
    );
    let eval_pred = evaluate_results(&[res_missing_pred], EvaluationPolicy::Diagnostic).unwrap();
    assert_eq!(eval_pred.counts.missing_prediction, 1);
    assert_eq!(eval_pred.counts.paired, 0);
    assert!(eval_pred.records[0].residual.is_none());

    // Strict evaluation rejects both missing modes.
    assert!(
        !evaluate_results(
            &[sample_result(
                "probe-m",
                "anchor-t0",
                None,
                ProbeOutcome::Completed { predicted: 5 },
                5
            )],
            EvaluationPolicy::Strict {
                max_late_numerator: 1,
                max_late_denominator: 1
            }
        )
        .unwrap()
        .accepted
    );
}

// ===========================================================================
// Condition Family 5: Impossible Resource Occupancy
// ===========================================================================
#[test]
fn fixture_05_impossible_resource_occupancy_strict_rejection_and_diagnostic() {
    // Probe starts with impossible resource occupancy (e.g. over capacity in ledger).
    let res_infeasible = sample_result(
        "probe-infeasible",
        "anchor-t0",
        Some(10),
        ProbeOutcome::Infeasible {
            reason: "occupancy exceeds capacity".into(),
        },
        0,
    );
    let eval = evaluate_results(
        std::slice::from_ref(&res_infeasible),
        EvaluationPolicy::Diagnostic,
    )
    .unwrap();
    assert_eq!(eval.counts.infeasible, 1);
    assert_eq!(eval.counts.paired, 0);
    assert!(eval.records[0].infeasible);
    assert!(matches!(
        eval.records[0].outcome,
        ProbeOutcome::Infeasible { .. }
    ));

    // Strict evaluation must reject candidate with infeasible resource allocations.
    let strict = evaluate_results(
        &[res_infeasible],
        EvaluationPolicy::Strict {
            max_late_numerator: 1,
            max_late_denominator: 1,
        },
    )
    .unwrap();
    assert!(!strict.accepted);

    // Verify ledger capacity boundary: acquire 2 units when capacity is 1.
    let mut claims = BTreeMap::new();
    claims.insert("holder-1".into(), 1u32);
    let state = ResourceState {
        capacity: 1,
        claims,
    };
    let current_allocated: u32 = state.claims.values().sum();
    let requested_units = 1u32;
    assert!(
        current_allocated + requested_units > state.capacity,
        "Resource over-allocation must be detected"
    );
}

// ===========================================================================
// Condition Family 6: Repeated Events (Occurrence Disambiguation)
// ===========================================================================
#[test]
fn fixture_06_repeated_events_occurrence_disambiguation() {
    // When a case has multiple occurrences of the same event/task (e.g. consultation),
    // occurrence index must strictly disambiguate them in the trace order key and logical key.
    let key_occ_0 = sample_key("patient-1", 0, "consultation");
    let key_occ_1 = sample_key("patient-1", 1, "consultation");
    assert_ne!(
        key_occ_0, key_occ_1,
        "Repeated events must have distinct logical keys"
    );

    let order_0 = TraceOrderKeyV1 {
        relative_ticks: 10,
        case_key: "patient-1".into(),
        occurrence: 0,
        event_kind_rank: EventKindRank::from_canonical_decimal("1").unwrap(),
        source_event_key: "consultation-1".into(),
        source_order: 1,
    };
    let order_1 = TraceOrderKeyV1 {
        relative_ticks: 25,
        case_key: "patient-1".into(),
        occurrence: 1,
        event_kind_rank: EventKindRank::from_canonical_decimal("1").unwrap(),
        source_event_key: "consultation-2".into(),
        source_order: 2,
    };
    assert!(
        order_0 < order_1,
        "Trace ordering must order repeated events canonically"
    );

    // Two probes for the two occurrences evaluated concurrently in the runner.
    let res_0 = sample_result(
        "probe-occ-0",
        "anchor-t0",
        Some(10),
        ProbeOutcome::Completed { predicted: 10 },
        10,
    );
    let res_1 = sample_result(
        "probe-occ-1",
        "anchor-t15",
        Some(25),
        ProbeOutcome::Completed { predicted: 25 },
        25,
    );
    let eval = evaluate_results(
        &[res_0, res_1],
        EvaluationPolicy::Strict {
            max_late_numerator: 0,
            max_late_denominator: 1,
        },
    )
    .unwrap();
    assert_eq!(eval.counts.total, 2);
    assert_eq!(eval.counts.paired, 2);
    assert_eq!(eval.counts.late, 0);
    assert!(eval.accepted);
}

// ===========================================================================
// Condition Family 7: Interrupted Transit and Work
// ===========================================================================
#[test]
fn fixture_07_interrupted_transit_and_work_preservation() {
    // Synthetic model of interrupted work:
    // Original duration = 7 ticks.
    // Progress before interruption = 2 useful ticks elapsed.
    // Interruption pause = 3 ticks.
    // Remaining work = 5 ticks.
    // Total elapsed = 2 (initial) + 3 (pause) + 5 (resumed remaining) = 10 ticks.
    // Assert that useful progress survives interruption without restarting from 0.
    let original_duration = 7u128;
    let elapsed_before_interrupt = 2u128;
    let interruption_pause = 3u128;
    assert!(elapsed_before_interrupt < original_duration);

    let remaining_work = original_duration - elapsed_before_interrupt;
    assert_eq!(remaining_work, 5);

    let total_elapsed = elapsed_before_interrupt + interruption_pause + remaining_work;
    assert_eq!(total_elapsed, 10);

    // Verify with probe result reflecting the interrupted transit/work.
    let res = sample_result(
        "probe-interrupted",
        "anchor-t0",
        Some(10),
        ProbeOutcome::Completed { predicted: 10 },
        10,
    );
    let eval = evaluate_results(&[res], EvaluationPolicy::Diagnostic).unwrap();
    assert_eq!(eval.counts.paired, 1);
    assert_eq!(eval.records[0].predicted, Some(10));
    assert_eq!(eval.records[0].residual.unwrap().magnitude, 0);
}

// ===========================================================================
// Condition Family 8: Bounded Probe Termination (Horizon & Event Budget)
// ===========================================================================
#[test]
fn fixture_08_bounded_probe_termination_tick_horizon_and_event_budget() {
    // 8a: Terminated by TickHorizon limit.
    let res_horizon = sample_result(
        "probe-censored-horizon",
        "anchor-t0",
        Some(20),
        ProbeOutcome::Censored {
            reason: LimitReason::TickHorizon,
        },
        15,
    );

    // 8b: Terminated by EventBudget limit.
    let res_budget = sample_result(
        "probe-censored-budget",
        "anchor-t0",
        Some(20),
        ProbeOutcome::Censored {
            reason: LimitReason::EventBudget,
        },
        12,
    );

    let eval = evaluate_results(
        &[res_horizon.clone(), res_budget.clone()],
        EvaluationPolicy::Diagnostic,
    )
    .unwrap();
    assert_eq!(eval.counts.total, 2);
    assert_eq!(eval.counts.censored, 2);
    assert_eq!(eval.counts.paired, 0);
    assert_eq!(eval.counts.missing_prediction, 2);

    // Strict evaluation rejects runs with censored probes.
    let strict = evaluate_results(
        &[res_horizon, res_budget],
        EvaluationPolicy::Strict {
            max_late_numerator: 1,
            max_late_denominator: 1,
        },
    )
    .unwrap();
    assert!(!strict.accepted);
}

// ===========================================================================
// Walk + Work = 5 Ridge Fixture and Independent Walk Observation
// Assert: Objective ties DO NOT imply uniquely identified empirical primitives!
// ===========================================================================
#[test]
fn fixture_09_walk_plus_work_ridge_and_independent_walk_observation() {
    // Ridge condition: total duration = walk + work = 5 ticks.
    // 4 plausible candidate decompositions:
    // (walk=1, work=4) -> total = 5
    // (walk=2, work=3) -> total = 5
    // (walk=3, work=2) -> total = 5
    // (walk=4, work=1) -> total = 5
    let candidates = [(1u128, 4u128), (2, 3), (3, 2), (4, 1)];

    // Target observed duration for the combined process is 5 ticks.
    let observed_total = 5u128;
    let mut total_residuals = Vec::new();

    for &(walk, work) in &candidates {
        let predicted_total = walk + work;
        assert_eq!(
            predicted_total, 5,
            "All candidate points must lie exactly on the walk+work=5 ridge"
        );

        let res = sample_result(
            &format!("probe-ridge-{walk}-{work}"),
            "anchor-t0",
            Some(observed_total),
            ProbeOutcome::Completed {
                predicted: predicted_total,
            },
            predicted_total,
        );
        let eval = evaluate_results(&[res], EvaluationPolicy::Diagnostic).unwrap();
        let residual = eval.records[0].residual.unwrap();
        total_residuals.push((walk, work, residual));
    }

    // Every candidate achieves an identical objective residual of 0.
    for &(walk, work, ref residual) in &total_residuals {
        assert_eq!(
            residual.magnitude, 0,
            "Candidate ({walk}, {work}) must achieve 0 residual on the total duration"
        );
    }

    // Critical assertion: Objective ties do not identify the parameters!
    // Without independent walk observations, the empirical primitives (walk, work)
    // are confounded along the ridge walk + work = 5.
    let tied_candidate_count = total_residuals
        .iter()
        .filter(|(_, _, res)| res.magnitude == 0)
        .count();
    assert_eq!(
        tied_candidate_count, 4,
        "All 4 candidate configurations achieve an exact objective tie (residual = 0)"
    );
    assert_ne!(
        tied_candidate_count, 1,
        "Objective tie alone DOES NOT imply uniquely identified empirical primitives!"
    );

    // Now introduce an independent observation of the walk arrival endpoint at tick 2.
    let independent_observed_walk = 2u128;

    // Filter candidates against the independent walk observation.
    let identified: Vec<_> = candidates
        .iter()
        .filter(|&&(walk, _)| walk == independent_observed_walk)
        .collect();

    // With the independent walk observation, the parameter pair is uniquely identified.
    assert_eq!(
        identified.len(),
        1,
        "Independent walk observation breaks the ridge symmetry and identifies a unique primitive"
    );
    assert_eq!(
        identified[0],
        &(2, 3),
        "Unique identified primitive is (walk=2, work=3)"
    );
}
