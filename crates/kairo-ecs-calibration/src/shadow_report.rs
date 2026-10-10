//! Canonical C3 outcome projection and C4 residual-row adapter.
//!
//! Evaluation is independent of provenance. `project_rows` requires the
//! caller's trusted admitted specs so it never invents a logical identity.

use crate::residuals::{self, GroupKey, LogicalKey, OutcomeStatus, Row, Side};
use crate::shadow::{
    Evaluation, EvaluationCounts, EvaluationPolicy, PredictionRecord, ProbeOutcome, ProbeResult,
    ProbeSpec, ShadowError,
};
use std::collections::BTreeMap;

const REPLAY_ROLE: &str = "ShadowAnchored";

/// Evaluate a terminal probe inventory in probe-ID order.
///
/// Diagnostic mode preserves all outcomes and accepts the evaluation. Strict
/// mode rejects incomplete, censored, failed, or infeasible records, then
/// applies the configured late fraction to paired point results.
pub(crate) fn evaluate(
    results: &[ProbeResult],
    policy: EvaluationPolicy,
) -> Result<Evaluation, ShadowError> {
    validate_policy(policy)?;
    let mut by_id = BTreeMap::new();
    for result in results {
        if result.id.trim().is_empty() {
            return Err(ShadowError::InvalidInput("empty probe ID"));
        }
        if by_id.insert(result.id.as_str(), result).is_some() {
            return Err(ShadowError::DuplicateIdentity(result.id.clone()));
        }
        validate_result(result)?;
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
                .map(|(observed, predicted)| residuals::Residual::between(predicted, observed)),
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

/// Project terminal probe results into paired C4 logical residual rows.
///
/// Specs are trusted admission records. Every result must match exactly one
/// spec, including its logical endpoint and observed tick; extra specs are
/// rejected to make the supplied inventory auditable.
pub(crate) fn project_rows(
    results: &[ProbeResult],
    specs: &[ProbeSpec],
) -> Result<Vec<Row>, ShadowError> {
    let mut spec_by_id = BTreeMap::new();
    let mut key_owners = BTreeMap::new();
    for spec in specs {
        if spec.id.trim().is_empty()
            || spec.input.target.trim().is_empty()
            || spec.run_id.trim().is_empty()
            || spec.candidate_id.trim().is_empty()
            || spec.anchor_event.trim().is_empty()
            || !matches!(spec.input.fidelity.as_str(), "Macro" | "Micro")
        {
            return Err(ShadowError::InvalidInput(
                "invalid probe, target, anchor, or fidelity identity",
            ));
        }
        if spec.input.target != spec.key.endpoint {
            return Err(ShadowError::InvalidInput(
                "probe target differs from C4 endpoint",
            ));
        }
        validate_logical_key(&spec.key)?;
        if key_owners
            .insert(spec.key.clone(), spec.id.as_str())
            .is_some()
        {
            return Err(ShadowError::DuplicateIdentity(
                "duplicate C4 logical endpoint".into(),
            ));
        }
        if spec_by_id.insert(spec.id.as_str(), spec).is_some() {
            return Err(ShadowError::DuplicateIdentity(spec.id.clone()));
        }
    }

    let mut result_by_id = BTreeMap::new();
    for result in results {
        validate_result(result)?;
        if result_by_id.insert(result.id.as_str(), result).is_some() {
            return Err(ShadowError::DuplicateIdentity(result.id.clone()));
        }
    }
    let mut rows = Vec::with_capacity(results.len().saturating_mul(2));
    for (id, result) in &result_by_id {
        let spec = spec_by_id
            .get(id)
            .ok_or_else(|| ShadowError::UnknownProbe((*id).to_owned()))?;
        if result.events > spec.budget.max_events || result.last_tick > spec.budget.horizon {
            return Err(ShadowError::Contract(
                "result exceeds admitted event or tick budget",
            ));
        }
        if result.anchor_event != spec.anchor_event || result.observed != spec.observed_target {
            return Err(ShadowError::Contract(
                "probe result differs from trusted admitted spec",
            ));
        }

        let group = GroupKey {
            strata: vec![("replay_role".into(), REPLAY_ROLE.into())],
        };
        let source_time = result.observed;
        rows.push(Row {
            key: spec.key.clone(),
            side: Side::Reference,
            group: group.clone(),
            source_time,
            status: if result.observed.is_some() {
                OutcomeStatus::Observed
            } else {
                OutcomeStatus::Missing
            },
            observed_ticks: result.observed,
            predicted_ticks: None,
            prediction_unclamped: false,
            // C4.2 residual rows use the existing unsigned nanosecond tick scale.
            tick_unit: "nanosecond".into(),
            provenance_supported: true,
            excluded: false,
            infeasible: false,
        });

        let (status, predicted, prediction_unclamped, infeasible) = match &result.outcome {
            ProbeOutcome::Completed { predicted } => {
                let late = result
                    .observed
                    .is_some_and(|observed| *predicted > observed);
                (
                    if late {
                        OutcomeStatus::Infeasible
                    } else {
                        OutcomeStatus::Predicted
                    },
                    Some(*predicted),
                    true,
                    late,
                )
            }
            ProbeOutcome::Missing => (OutcomeStatus::Missing, None, false, false),
            ProbeOutcome::Infeasible { .. } => (OutcomeStatus::Infeasible, None, false, true),
            ProbeOutcome::Censored { .. } => (OutcomeStatus::Censored, None, false, false),
            ProbeOutcome::Failed { .. } => (OutcomeStatus::Failed, None, false, false),
        };
        rows.push(Row {
            key: spec.key.clone(),
            side: Side::Simulation,
            group,
            source_time,
            status,
            observed_ticks: None,
            predicted_ticks: predicted,
            prediction_unclamped,
            tick_unit: "nanosecond".into(),
            provenance_supported: true,
            excluded: false,
            infeasible,
        });
    }
    if result_by_id.len() != spec_by_id.len() {
        return Err(ShadowError::InvalidInput(
            "trusted specs and terminal result inventory differ",
        ));
    }
    Ok(rows)
}

fn validate_policy(policy: EvaluationPolicy) -> Result<(), ShadowError> {
    if let EvaluationPolicy::Strict {
        max_late_numerator,
        max_late_denominator,
    } = policy
    {
        if max_late_denominator == 0 || max_late_numerator > max_late_denominator {
            return Err(ShadowError::InvalidInput("invalid strict late fraction"));
        }
    }
    Ok(())
}

fn validate_result(result: &ProbeResult) -> Result<(), ShadowError> {
    if result.id.trim().is_empty() || result.anchor_event.trim().is_empty() {
        return Err(ShadowError::InvalidInput("empty anchor event"));
    }
    match &result.outcome {
        ProbeOutcome::Completed { predicted } if *predicted != result.last_tick => Err(
            ShadowError::Contract("completed prediction differs from terminal tick"),
        ),
        ProbeOutcome::Infeasible { reason } | ProbeOutcome::Failed { reason }
            if reason.trim().is_empty() =>
        {
            Err(ShadowError::InvalidInput("empty terminal reason"))
        }
        _ => Ok(()),
    }
}

fn validate_logical_key(key: &LogicalKey) -> Result<(), ShadowError> {
    let fields = [
        &key.study_id,
        &key.dataset_id,
        &key.scenario_id,
        &key.seed_schedule_id,
        &key.replication_id,
        &key.case_key,
        &key.task_key,
        &key.endpoint,
        &key.seed_purpose,
        &key.seed_map_ref,
        &key.mapping_version,
    ];
    if fields.iter().any(|field| field.trim().is_empty())
        || !matches!(
            key.seed_purpose.as_str(),
            "service" | "transit" | "behavior" | "calibration"
        )
    {
        return Err(ShadowError::InvalidInput(
            "invalid C4 logical residual provenance",
        ));
    }
    Ok(())
}
