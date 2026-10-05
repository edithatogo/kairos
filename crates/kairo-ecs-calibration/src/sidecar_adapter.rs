//! Private C4.3 join adapter. This module is path-imported by owner tests until
//! the Arrow writer is integrated; it deliberately exposes no crate API.

use crate::metric_cohorts::{self, Identity, Row as MetricRow, Spec as MetricSpec};
use crate::residuals::{
    self, GroupKey, LogicalKey, OutcomeStatus, Row as ResidualRow, Side, SummaryStatus, Window,
};
use crate::seed_map::SEED_MAP_VERSION_V1;
use kairo_ecs_arrow::EventLogRecord;
use kairo_ecs_types::EventId;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const SEED_CONTRACT: &str = "kairoecs.seed-purpose.v1";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RunBinding {
    pub study_id: String,
    pub dataset_id: String,
    pub scenario_id: String,
    pub candidate_id: String,
    pub run_id: String,
    pub replication_id: String,
    pub seed_schedule_id: String,
    pub seed_map_ref: String,
    pub seed_map_version: u32,
    pub seed_contract_version: String,
    pub mapping_version: String,
    pub fidelity: String,
    pub parameter_hash: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Evidence {
    pub fidelity: String,
    pub anchor_role: String,
    pub feasibility: String,
    pub censor_status: String,
    pub seed_contract_version: String,
    pub parameter_hash: String,
    pub graph_hash: Option<String>,
    pub causal_ref: Option<String>,
    pub probe_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SidecarRow {
    pub row: ResidualRow,
    pub evidence: Evidence,
}

#[derive(Clone, Debug)]
pub(crate) struct MetricCohort {
    pub identity: Identity,
    pub rows: Vec<MetricRow>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GroupStratum {
    pub group: String,
    pub strata: Value,
}

#[derive(Debug)]
pub(crate) struct Input {
    pub binding: RunBinding,
    /// Runner inventory read independently from the metric/residual source rows.
    pub run_records: Vec<RunBinding>,
    pub source_window: Window,
    pub residual_groups: Vec<GroupKey>,
    pub residual_rows: Vec<SidecarRow>,
    pub metric_spec: MetricSpec,
    pub reference_metric: MetricCohort,
    pub simulation_metric: MetricCohort,
    pub metric_strata: Vec<GroupStratum>,
    pub events: Vec<EventLogRecord>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct JoinRecord {
    pub logical_key: Value,
    pub run_id: String,
    pub candidate_id: String,
    pub probe_id: Option<String>,
    pub causal_ref: Option<String>,
    pub event_id_le_hex: Option<String>,
    pub event_resolved: Option<bool>,
    pub eligible: bool,
    pub excluded: bool,
    pub source_time: Option<u128>,
    pub raw_reference_count: usize,
    pub raw_simulation_count: usize,
    pub residual_status: String,
    pub raw_records: Vec<Value>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Output {
    pub residuals: Vec<Value>,
    pub metrics: Vec<Value>,
    pub joins: Vec<JoinRecord>,
    pub group_diagnostics: Vec<Value>,
    pub metric_group_diagnostics: Vec<Value>,
    pub metric_raw_rows: Vec<Value>,
    pub raw_diagnostics: Value,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct AdapterFailure {
    pub reason: String,
    pub raw_rows: Vec<Value>,
    pub metric_raw_rows: Vec<Value>,
    pub raw_diagnostics: Value,
}

pub(crate) fn build_sidecars(input: &Input) -> Result<Output, AdapterFailure> {
    let raw_rows = input
        .residual_rows
        .iter()
        .map(raw_residual_row)
        .collect::<Vec<_>>();
    let metric_raw_rows = metric_raw_rows(input);
    let raw_diagnostics = raw_diagnostics(input, &raw_rows, &metric_raw_rows);
    let fail = |reason: &str| AdapterFailure {
        reason: reason.to_owned(),
        raw_rows: raw_rows.clone(),
        metric_raw_rows: metric_raw_rows.clone(),
        raw_diagnostics: raw_diagnostics.clone(),
    };
    validate_binding(input).map_err(|reason| fail(&reason))?;
    validate_events(input).map_err(|reason| fail(&reason))?;
    validate_residual_rows(input).map_err(|reason| fail(&reason))?;
    validate_metric_inputs(input).map_err(|reason| fail(&reason))?;

    let kernel_rows: Vec<_> = input.residual_rows.iter().map(|r| r.row.clone()).collect();
    let summaries = residuals::summarize(
        &kernel_rows,
        &input.residual_groups,
        Some(input.source_window),
    );
    if summaries
        .iter()
        .any(|s| matches!(s.status, SummaryStatus::Invalid | SummaryStatus::Unverified))
    {
        return Err(fail(
            "C4.2 residual kernel returned invalid or unverified batch",
        ));
    }

    let mut by_key: BTreeMap<LogicalKey, (Vec<&SidecarRow>, Vec<&SidecarRow>)> = BTreeMap::new();
    for item in &input.residual_rows {
        let sides = by_key.entry(item.row.key.clone()).or_default();
        match item.row.side {
            Side::Reference => sides.0.push(item),
            Side::Simulation => sides.1.push(item),
        }
    }
    let mut residuals_out = Vec::with_capacity(by_key.len());
    let mut joins = Vec::with_capacity(by_key.len());
    for (key, (reference, simulation)) in &by_key {
        let residual = residual_for_pair(reference, simulation);
        let (residual_status, sign, magnitude) = match residual {
            Some(r) => (
                "computed",
                match r.sign {
                    residuals::ResidualSign::Negative => "negative",
                    residuals::ResidualSign::Zero => "zero",
                    residuals::ResidualSign::Positive => "positive",
                },
                Some(r.magnitude.to_string()),
            ),
            None => {
                let status = noncomputed_status(reference, simulation);
                (status, "undefined", None)
            }
        };
        let evidence = merge_evidence(reference, simulation).map_err(|reason| fail(&reason))?;
        let observed_ticks = reference
            .first()
            .and_then(|r| r.row.observed_ticks)
            .map(|v| v.to_string());
        let predicted_ticks = simulation
            .first()
            .and_then(|r| r.row.predicted_ticks)
            .map(|v| v.to_string());
        let observed_out = if matches!(residual_status, "computed" | "probe_failed" | "infeasible")
        {
            observed_ticks.clone()
        } else {
            None
        };
        let predicted_out = if matches!(
            residual_status,
            "computed" | "missing_observed" | "censored" | "infeasible"
        ) {
            predicted_ticks.clone()
        } else {
            None
        };
        let row_value = json!({
            "record_type":"calibration_residual.v1",
            "schema_version":"calibration-v1",
            "dataset_id":key.dataset_id,
            "scenario_id":key.scenario_id,
            "run_id":input.binding.run_id,
            "candidate_id":input.binding.candidate_id,
            "case_key":key.case_key,
            "task_key":key.task_key,
            "occurrence":key.occurrence,
            "endpoint":key.endpoint,
            "fidelity":evidence.fidelity,
            "observed_ticks":observed_out,
            "predicted_ticks":predicted_out,
            "residual_status":residual_status,
            "residual_sign":sign,
            "residual_magnitude":magnitude,
            "prediction_unclamped":true,
            "anchor_role":evidence.anchor_role,
            "feasibility":evidence.feasibility,
            "censor_status":evidence.censor_status,
            "study_id":key.study_id,
            "replication_id":key.replication_id,
            "seed_schedule_id":key.seed_schedule_id,
            "seed_purpose":key.seed_purpose,
            "seed_map_ref":key.seed_map_ref,
            "seed_contract_version":evidence.seed_contract_version,
            "mapping_version":key.mapping_version,
            "parameter_hash":evidence.parameter_hash,
            "graph_hash":evidence.graph_hash,
            "causal_ref":evidence.causal_ref,
        });
        residuals_out.push(row_value);

        let source_row = simulation.first().or_else(|| reference.first()).unwrap();
        let source_time = source_row.row.source_time;
        let eligible = reference.first().is_some_and(|r| point_reference(&r.row))
            && simulation.first().is_some_and(|r| point_simulation(&r.row))
            && reference.first().is_some_and(|r| !r.row.excluded)
            && simulation.first().is_some_and(|r| !r.row.excluded)
            && reference.first().and_then(|r| r.row.source_time)
                == simulation.first().and_then(|r| r.row.source_time)
            && source_time.is_some_and(|time| {
                input.source_window.start <= time && time < input.source_window.end_exclusive
            });
        let excluded = reference.iter().chain(simulation).any(|r| r.row.excluded)
            || source_time.is_none_or(|time| {
                time < input.source_window.start || time >= input.source_window.end_exclusive
            });
        let event_id_le_hex = evidence.causal_ref.as_deref().map(|reference| {
            resolve_causal_ref(reference, &input.binding.run_id, &input.events)
                .expect("validated causal ref")
                .1
        });
        let event_resolved = evidence.causal_ref.is_none() || event_id_le_hex.is_some();
        joins.push(JoinRecord {
            logical_key: logical_key_json(key),
            run_id: input.binding.run_id.clone(),
            candidate_id: input.binding.candidate_id.clone(),
            probe_id: evidence.probe_id.clone(),
            causal_ref: evidence.causal_ref.clone(),
            event_id_le_hex,
            event_resolved: evidence.causal_ref.as_ref().map(|_| event_resolved),
            eligible,
            excluded,
            source_time,
            raw_reference_count: reference.len(),
            raw_simulation_count: simulation.len(),
            residual_status: residual_status.to_owned(),
            raw_records: reference
                .iter()
                .chain(simulation)
                .map(|r| raw_residual_row(r))
                .collect(),
        });
    }

    let (metrics, metric_group_diagnostics) =
        make_metrics(input, &summaries).map_err(|reason| fail(&reason))?;
    let group_diagnostics = residual_group_diagnostics(&summaries);
    residuals_out.sort_by_key(residual_sort_key);
    Ok(Output {
        residuals: residuals_out,
        metrics,
        joins,
        group_diagnostics,
        metric_group_diagnostics,
        metric_raw_rows,
        raw_diagnostics,
    })
}

fn raw_diagnostics(input: &Input, residual_rows: &[Value], metric_rows: &[Value]) -> Value {
    let residual_hash = canonical_rows_hash(residual_rows);
    let metric_hash = canonical_rows_hash(metric_rows);
    json!({
        "residual_raw_rows": input.residual_rows.len(),
        "residual_reference_rows": input.residual_rows.iter().filter(|r| r.row.side == Side::Reference).count(),
        "residual_simulation_rows": input.residual_rows.iter().filter(|r| r.row.side == Side::Simulation).count(),
        "residual_pairs": input.residual_rows.iter().map(|r| &r.row.key).collect::<BTreeSet<_>>().len(),
        "residual_raw_rows_canonical_sha256": residual_hash,
        "residual_raw_rows_hash_algorithm":"sha256(serde_json::to_vec(sort_by_compact_json(raw_rows)))",
        "metric_reference_raw_rows": input.reference_metric.rows.len(),
        "metric_simulation_raw_rows": input.simulation_metric.rows.len(),
        "metric_raw_rows_canonical_sha256": metric_hash,
        "metric_raw_rows_hash_algorithm":"sha256(serde_json::to_vec(sort_by_compact_json(side_tagged_metric_rows)))",
    })
}

fn canonical_rows_hash(rows: &[Value]) -> String {
    let mut canonical = rows.to_vec();
    canonical.sort_by_key(Value::to_string);
    let bytes =
        serde_json::to_vec(&canonical).expect("serde_json::Value rows serialize canonically");
    Sha256::digest(&bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}

pub(crate) fn join_manifest_json(output: &Output) -> Value {
    let joins = output
        .joins
        .iter()
        .map(|join| {
            json!({
                "logical_key":join.logical_key,
                "run_id":join.run_id,
                "candidate_id":join.candidate_id,
                "probe_id":join.probe_id,
                "causal_ref":join.causal_ref,
                "event_id_le_hex":join.event_id_le_hex,
                "event_resolved":join.event_resolved,
                "eligible":join.eligible,
                "excluded":join.excluded,
                "source_time":join.source_time.map(|t| t.to_string()),
                "raw_reference_count":join.raw_reference_count,
                "raw_simulation_count":join.raw_simulation_count,
                "residual_status":join.residual_status,
                "raw_records":join.raw_records,
            })
        })
        .collect::<Vec<_>>();
    json!({ "version":"c43.join_manifest.v1", "counts":output.raw_diagnostics,
        "rows":joins, "group_summaries":output.group_diagnostics,
        "metric_group_diagnostics":output.metric_group_diagnostics,
        "metric_raw_records":output.metric_raw_rows })
}

fn metric_raw_rows(input: &Input) -> Vec<Value> {
    let mut rows = input
        .reference_metric
        .rows
        .iter()
        .map(|r| raw_metric_row("reference", r))
        .collect::<Vec<_>>();
    rows.extend(
        input
            .simulation_metric
            .rows
            .iter()
            .map(|r| raw_metric_row("simulation", r)),
    );
    rows.sort_by_key(Value::to_string);
    rows
}

fn raw_metric_row(side: &str, row: &MetricRow) -> Value {
    json!({"side":side,"key":row.key,"group":row.group,"selection_time":row.selection_time.map(|v|v.to_string()),
        "value":row.value,"weight":row.weight,"outcome":format!("{:?}",row.outcome),"excluded":row.excluded,
        "censored":row.censored,"missing":row.missing,"failed":row.failed,"infeasible":row.infeasible})
}

fn residual_group_diagnostics(summaries: &[residuals::Summary]) -> Vec<Value> {
    summaries.iter().map(|s| json!({
        "group":s.group.strata,
        "status":format!("{:?}",s.status),
        "raw_reference_count":s.counts.raw_reference,"raw_simulation_count":s.counts.raw_simulation,
        "eligible_reference_count":s.counts.reference,"eligible_simulation_count":s.counts.simulation,
        "matched_count":s.counts.matched,"unmatched_count":s.counts.unmatched,
        "excluded_count":s.counts.excluded,"censored_count":s.counts.censored,"missing_count":s.counts.missing,
        "failed_count":s.counts.failed,"infeasible_count":s.counts.infeasible,"missing_time_count":s.counts.missing_time,
        "bias":s.bias,"mae":s.mae,"rmse":s.rmse,"approximate":s.approximate
    })).collect()
}

fn validate_binding(input: &Input) -> Result<(), String> {
    let b = &input.binding;
    if input.run_records.len() != 1 || input.run_records[0] != *b {
        return Err(
            "run inventory is missing, duplicated, or differs from bound provenance".into(),
        );
    }
    for value in [
        &b.study_id,
        &b.dataset_id,
        &b.scenario_id,
        &b.candidate_id,
        &b.run_id,
        &b.replication_id,
        &b.seed_schedule_id,
        &b.seed_map_ref,
        &b.mapping_version,
        &b.fidelity,
        &b.seed_contract_version,
    ] {
        if value.trim().is_empty() {
            return Err("run binding contains an empty required id".into());
        }
    }
    if b.seed_map_version != SEED_MAP_VERSION_V1 || b.seed_contract_version != SEED_CONTRACT {
        return Err("seed contract does not match the supported seed map v1".into());
    }
    if !valid_hash(&b.parameter_hash) {
        return Err("invalid run parameter hash".into());
    }
    if input.source_window.start >= input.source_window.end_exclusive {
        return Err("source window must be nonempty and half-open".into());
    }
    Ok(())
}

fn validate_events(input: &Input) -> Result<(), String> {
    let mut handles = BTreeSet::new();
    for event in &input.events {
        event
            .validate()
            .map_err(|e| format!("invalid event record: {e}"))?;
        if event.run_id != input.binding.run_id {
            return Err("event record run_id differs from bound run".into());
        }
        if !handles.insert((event.run_id.as_str(), event.event_id)) {
            return Err("duplicate run/event handle".into());
        }
    }
    for row in &input.residual_rows {
        if let Some(reference) = row.evidence.causal_ref.as_deref() {
            resolve_causal_ref(reference, &input.binding.run_id, &input.events)?;
        }
    }
    Ok(())
}

fn validate_residual_rows(input: &Input) -> Result<(), String> {
    let b = &input.binding;
    let mut pair_sides: BTreeMap<LogicalKey, (usize, usize)> = BTreeMap::new();
    let mut groups = BTreeSet::new();
    for group in &input.residual_groups {
        if !groups.insert(group) {
            return Err("duplicate fixed residual group".into());
        }
    }
    for item in &input.residual_rows {
        let row = &item.row;
        if row.key.study_id != b.study_id
            || row.key.dataset_id != b.dataset_id
            || row.key.scenario_id != b.scenario_id
            || row.key.seed_schedule_id != b.seed_schedule_id
            || row.key.replication_id != b.replication_id
            || row.key.seed_map_ref != b.seed_map_ref
            || row.key.mapping_version != b.mapping_version
            || row.key.endpoint != input.metric_spec.identity.endpoint
        {
            return Err("residual row identity differs from bound run provenance".into());
        }
        if row.tick_unit != "nanosecond" || !row.provenance_supported {
            return Err("residual row has unsupported unit or provenance".into());
        }
        if !groups.contains(&row.group) {
            return Err("residual row has undeclared group".into());
        }
        if item.evidence.fidelity != b.fidelity
            || item.evidence.parameter_hash != b.parameter_hash
            || item.evidence.seed_contract_version != b.seed_contract_version
        {
            return Err(
                "residual row hash, fidelity, or seed contract differs from run binding".into(),
            );
        }
        validate_evidence(&item.evidence)?;
        if row.side == Side::Simulation && item.evidence.probe_id.is_none() {
            return Err("simulation residual row has no probe_id join metadata".into());
        }
        match row.status {
            OutcomeStatus::Censored
                if !matches!(
                    item.evidence.censor_status.as_str(),
                    "left" | "right" | "interval"
                ) =>
            {
                return Err("censored row lacks a supported censor status".into());
            }
            OutcomeStatus::Observed | OutcomeStatus::Predicted
                if item.evidence.censor_status != "not_censored" =>
            {
                return Err("point row contradicts censor status".into());
            }
            OutcomeStatus::Missing
                if !matches!(item.evidence.censor_status.as_str(), "missing" | "unknown") =>
            {
                return Err("missing row contradicts censor status".into());
            }
            _ => {}
        }
        if row.infeasible != (item.evidence.feasibility == "infeasible") {
            return Err("infeasibility evidence contradicts residual row".into());
        }
        let counts = pair_sides.entry(row.key.clone()).or_default();
        match row.side {
            Side::Reference => counts.0 += 1,
            Side::Simulation => counts.1 += 1,
        }
        if counts.0 > 1 || counts.1 > 1 {
            return Err("duplicate logical pair side".into());
        }
    }
    Ok(())
}

fn validate_evidence(e: &Evidence) -> Result<(), String> {
    if !matches!(e.fidelity.as_str(), "ShadowAnchored" | "FreeRunning")
        || !matches!(
            e.anchor_role.as_str(),
            "observed_source_transition" | "none" | "unknown"
        )
        || !matches!(
            e.feasibility.as_str(),
            "feasible" | "infeasible" | "unknown"
        )
        || !matches!(
            e.censor_status.as_str(),
            "not_censored" | "left" | "right" | "interval" | "unknown" | "missing"
        )
        || !valid_hash(&e.parameter_hash)
        || e.graph_hash.as_deref().is_some_and(|h| !valid_hash(h))
        || e.probe_id.as_deref().is_some_and(|s| s.trim().is_empty())
    {
        return Err("invalid residual evidence enum, id, or hash".into());
    }
    Ok(())
}

fn validate_metric_inputs(input: &Input) -> Result<(), String> {
    let s = &input.metric_spec;
    let b = &input.binding;
    if !matches!(s.algorithm_version.as_str(), "empirical_equal.v1")
        || s.identity.unit != "nanosecond"
        || s.scale_ticks != "1"
        || s.window != Some((input.source_window.start, input.source_window.end_exclusive))
        || s.identity.dataset != b.dataset_id
        || s.identity.mapping != b.mapping_version
        || s.identity.seed_schedule != b.seed_schedule_id
        || s.identity.seed_map != b.seed_map_ref
    {
        return Err("metric spec differs from explicit run/window provenance".into());
    }
    if !same_identity(&input.reference_metric.identity, &s.identity)
        || !same_identity(&input.simulation_metric.identity, &s.identity)
    {
        return Err("metric cohort identity differs from metric spec".into());
    }
    if input.metric_strata.len() != s.groups.len() {
        return Err("fixed group/strata mapping count mismatch".into());
    }
    let fixed_groups: BTreeSet<&str> = s.groups.iter().map(String::as_str).collect();
    if fixed_groups.is_empty()
        || fixed_groups.len() != s.groups.len()
        || fixed_groups.contains("")
        || [
            &s.identity.dataset,
            &s.identity.endpoint,
            &s.identity.unit,
            &s.identity.mapping,
            &s.identity.seed_schedule,
            &s.identity.seed_map,
        ]
        .iter()
        .any(|v| v.trim().is_empty())
    {
        return Err("metric spec has an empty or duplicate fixed cohort identity".into());
    }
    let mut mapped = BTreeMap::new();
    for group in &input.metric_strata {
        if !s.groups.contains(&group.group)
            || group.strata.get("candidate_id") != Some(&Value::String(b.candidate_id.clone()))
            || !group.strata.is_object()
            || mapped.insert(group.group.as_str(), &group.strata).is_some()
        {
            return Err("invalid fixed group-to-strata mapping".into());
        }
    }
    if input
        .reference_metric
        .rows
        .iter()
        .chain(&input.simulation_metric.rows)
        .any(|r| !mapped.contains_key(r.group.as_str()))
    {
        return Err("metric row has an undeclared group".into());
    }
    validate_metric_rows(&input.reference_metric.rows, &fixed_groups)?;
    validate_metric_rows(&input.simulation_metric.rows, &fixed_groups)?;
    if input
        .residual_groups
        .iter()
        .any(|g| group_label_for_residual(g, input).is_empty())
    {
        return Err("residual group has no explicit metric strata mapping".into());
    }
    Ok(())
}

fn validate_metric_rows(rows: &[MetricRow], groups: &BTreeSet<&str>) -> Result<(), String> {
    let mut keys = BTreeSet::new();
    for row in rows {
        if row.key.trim().is_empty() || !keys.insert(row.key.as_str()) {
            return Err("metric cohort has an empty or duplicate source key".into());
        }
        if !groups.contains(row.group.as_str()) {
            return Err("metric cohort contains an unknown fixed group".into());
        }
        if (row.outcome == metric_cohorts::Outcome::Point) != row.value.is_some() {
            return Err("metric point/value status contradiction".into());
        }
        if row.weight.is_some() {
            return Err("empirical_equal.v1 does not admit weighted observations".into());
        }
    }
    Ok(())
}

fn residual_for_pair(
    reference: &[&SidecarRow],
    simulation: &[&SidecarRow],
) -> Option<residuals::Residual> {
    let ref_row = &reference.first()?.row;
    let sim_row = &simulation.first()?.row;
    if !point_reference(ref_row) || !point_simulation(sim_row) {
        return None;
    }
    let r = ref_row.observed_ticks?;
    let s = sim_row.predicted_ticks?;
    Some(residuals::Residual::between(s, r))
}

fn point_reference(row: &ResidualRow) -> bool {
    row.status == OutcomeStatus::Observed && row.observed_ticks.is_some()
}
fn point_simulation(row: &ResidualRow) -> bool {
    matches!(
        row.status,
        OutcomeStatus::Predicted | OutcomeStatus::Infeasible
    ) && row.predicted_ticks.is_some()
        && row.prediction_unclamped
}

fn merge_evidence(
    reference: &[&SidecarRow],
    simulation: &[&SidecarRow],
) -> Result<Evidence, String> {
    let r = reference.first().map(|r| &r.evidence);
    let s = simulation.first().map(|r| &r.evidence);
    if r.and_then(|e| e.graph_hash.as_ref())
        .zip(s.and_then(|e| e.graph_hash.as_ref()))
        .is_some_and(|(a, b)| a != b)
    {
        return Err("contradictory graph hashes for one logical pair".into());
    }
    let base = s.or(r).expect("pair has source evidence");
    // Preserve source censor lineage even when another side determines the
    // residual outcome (for example, a failed simulation with a censored source).
    let censor_evidence = reference
        .iter()
        .chain(simulation)
        .find(|r| r.row.status == OutcomeStatus::Censored)
        .or_else(|| {
            reference
                .iter()
                .chain(simulation)
                .find(|r| r.row.status == OutcomeStatus::Missing)
        });
    Ok(Evidence {
        fidelity: base.fidelity.clone(),
        anchor_role: s.or(r).unwrap().anchor_role.clone(),
        feasibility: s.or(r).unwrap().feasibility.clone(),
        censor_status: censor_evidence
            .map(|r| r.evidence.censor_status.clone())
            .or_else(|| s.map(|e| e.censor_status.clone()))
            .or_else(|| r.map(|e| e.censor_status.clone()))
            .unwrap(),
        seed_contract_version: base.seed_contract_version.clone(),
        parameter_hash: base.parameter_hash.clone(),
        graph_hash: s
            .and_then(|e| e.graph_hash.clone())
            .or_else(|| r.and_then(|e| e.graph_hash.clone())),
        causal_ref: s
            .and_then(|e| e.causal_ref.clone())
            .or_else(|| r.and_then(|e| e.causal_ref.clone())),
        probe_id: s.and_then(|e| e.probe_id.clone()),
    })
}

fn noncomputed_status(reference: &[&SidecarRow], simulation: &[&SidecarRow]) -> &'static str {
    if simulation
        .first()
        .is_some_and(|r| r.row.status == OutcomeStatus::Failed)
    {
        return "probe_failed";
    }
    if reference
        .first()
        .is_some_and(|r| r.row.status == OutcomeStatus::Censored)
        || simulation
            .first()
            .is_some_and(|r| r.row.status == OutcomeStatus::Censored)
    {
        return "censored";
    }
    if simulation
        .first()
        .is_some_and(|r| r.row.status == OutcomeStatus::Infeasible)
    {
        return "infeasible";
    }
    if simulation.is_empty() && reference.first().is_some_and(|r| point_reference(&r.row)) {
        return "probe_failed";
    }
    if reference
        .first()
        .is_some_and(|r| r.row.status == OutcomeStatus::Failed)
    {
        return "probe_failed";
    }
    "missing_observed"
}

fn make_metrics(
    input: &Input,
    summaries: &[residuals::Summary],
) -> Result<(Vec<Value>, Vec<Value>), String> {
    let cohort_id = &input.metric_spec.identity;
    let metric_groups = input
        .metric_strata
        .iter()
        .map(|g| (g.group.as_str(), &g.strata))
        .collect::<BTreeMap<_, _>>();
    let cohort_spec = MetricSpec {
        identity: cohort_id.clone(),
        groups: input.metric_spec.groups.clone(),
        window: input.metric_spec.window,
        algorithm_version: input.metric_spec.algorithm_version.clone(),
        origin: input.metric_spec.origin.clone(),
        scale_ticks: input.metric_spec.scale_ticks.clone(),
        provenance_verified: input.metric_spec.provenance_verified,
    };
    let mut output = Vec::new();
    let mut group_diagnostics = Vec::new();
    for group_metric in metric_cohorts::compare_groups(
        &cohort_spec,
        &input.reference_metric.identity,
        &input.simulation_metric.identity,
        &input.reference_metric.rows,
        &input.simulation_metric.rows,
    ) {
        let strata = metric_groups
            .get(group_metric.group.as_str())
            .ok_or("metric kernel emitted undeclared group")?;
        let status = status_metric(group_metric.status);
        group_diagnostics.push(json!({
            "group": group_metric.group,
            "strata": strata,
            "status": status,
            "reference_count": group_metric.reference_count,
            "simulation_count": group_metric.simulation_count,
            "reference_tie_count": group_metric.reference_tie_count,
            "simulation_tie_count": group_metric.simulation_tie_count,
            "coverage_warnings": group_metric.coverage_warnings,
        }));
        let mut diagnostics = group_metric.reference_diagnostics;
        add_diagnostics(&mut diagnostics, group_metric.simulation_diagnostics)?;
        let common = metric_record_common(
            input,
            &group_metric.group,
            strata,
            [
                diagnostics.excluded,
                diagnostics.censored,
                diagnostics.missing,
                diagnostics.failed,
                diagnostics.infeasible,
            ],
            group_metric.reference_count,
            group_metric.simulation_count,
            group_metric.unmatched_count,
            status,
        )?;
        for (metric, value, units) in [
            ("W1", group_metric.w1, cohort_id.unit.as_str()),
            ("KS_D", group_metric.ks_d, "dimensionless"),
        ] {
            output.push(metric_record(
                &common,
                metric,
                input.metric_spec.algorithm_version.as_str(),
                value,
                units,
            )?);
        }
    }
    for summary in summaries {
        let label = input
            .metric_strata
            .iter()
            .find(|s| s.group == group_label_for_residual(&summary.group, input))
            .ok_or("residual group lacks metric strata mapping")?;
        for (name, value) in [
            ("bias", summary.bias),
            ("mae", summary.mae),
            ("rmse", summary.rmse),
        ] {
            let mut strata = label.strata.clone();
            strata
                .as_object_mut()
                .unwrap()
                .insert("statistic".into(), Value::String(name.into()));
            let status = match summary.status {
                SummaryStatus::Computed => "computed",
                SummaryStatus::Empty => "empty",
                SummaryStatus::InsufficientData => "insufficient_data",
                SummaryStatus::Invalid => "invalid",
                SummaryStatus::Unverified => "unverified",
            };
            let matched_count = summary.counts.matched;
            let common = metric_record_common(
                input,
                &label.group,
                &strata,
                residual_counts(summary)?,
                matched_count,
                matched_count,
                summary.counts.unmatched,
                status,
            )?;
            output.push(metric_record(
                &common,
                "paired_residual_summary",
                "residual_summary.canonical_float.v1",
                value,
                "nanosecond",
            )?);
        }
    }
    output.sort_by_key(metric_sort_key);
    Ok((output, group_diagnostics))
}

fn group_label_for_residual<'a>(group: &GroupKey, input: &'a Input) -> &'a str {
    let mut object = Map::new();
    for (key, value) in &group.strata {
        object.insert(key.clone(), Value::String(value.clone()));
    }
    let group_value = Value::Object(object);
    input
        .metric_strata
        .iter()
        .find(|m| {
            let mut expected = m.strata.clone();
            expected.as_object_mut().map(|v| v.remove("candidate_id"));
            expected == group_value
        })
        .map(|m| m.group.as_str())
        .unwrap_or("")
}

#[derive(Clone)]
struct MetricCommon {
    strata: Value,
    reference: u64,
    simulation: u64,
    excluded: u64,
    censored: u64,
    missing: u64,
    unmatched: u64,
    failed: u64,
    infeasible: u64,
    status: String,
    endpoint: String,
    window_start: String,
    window_end: String,
    dataset: String,
    run: String,
    mapping: String,
    seed_schedule: Option<String>,
    seed_map: Option<String>,
    seed_contract: Option<String>,
    parameter_hash: Option<String>,
}

#[allow(
    clippy::too_many_arguments,
    reason = "keeps the C0 metric field inputs explicit at each kernel outcome"
)]
fn metric_record_common(
    input: &Input,
    _group: &str,
    strata: &Value,
    diagnostics: [usize; 5],
    reference: usize,
    simulation: usize,
    unmatched: usize,
    status: impl Into<String>,
) -> Result<MetricCommon, String> {
    let b = &input.binding;
    let convert = |value: usize, label: &str| {
        u64::try_from(value).map_err(|_| format!("{label} count overflow"))
    };
    Ok(MetricCommon {
        strata: strata.clone(),
        reference: convert(reference, "reference")?,
        simulation: convert(simulation, "simulation")?,
        excluded: convert(diagnostics[0], "excluded")?,
        censored: convert(diagnostics[1], "censored")?,
        missing: convert(diagnostics[2], "missing")?,
        unmatched: convert(unmatched, "unmatched")?,
        failed: convert(diagnostics[3], "failed")?,
        infeasible: convert(diagnostics[4], "infeasible")?,
        status: status.into(),
        endpoint: input.metric_spec.identity.endpoint.clone(),
        window_start: input.source_window.start.to_string(),
        window_end: input.source_window.end_exclusive.to_string(),
        dataset: b.dataset_id.clone(),
        run: b.run_id.clone(),
        mapping: b.mapping_version.clone(),
        seed_schedule: Some(b.seed_schedule_id.clone()),
        seed_map: Some(b.seed_map_ref.clone()),
        seed_contract: Some(b.seed_contract_version.clone()),
        parameter_hash: Some(b.parameter_hash.clone()),
    })
}

fn residual_counts(summary: &residuals::Summary) -> Result<[usize; 5], String> {
    let c = summary.counts;
    Ok([c.excluded, c.censored, c.missing, c.failed, c.infeasible])
}

fn add_diagnostics(
    a: &mut metric_cohorts::Counts,
    b: metric_cohorts::Counts,
) -> Result<(), String> {
    a.raw = a
        .raw
        .checked_add(b.raw)
        .ok_or("metric raw count overflow")?;
    a.excluded = a
        .excluded
        .checked_add(b.excluded)
        .ok_or("metric excluded count overflow")?;
    a.censored = a
        .censored
        .checked_add(b.censored)
        .ok_or("metric censored count overflow")?;
    a.missing = a
        .missing
        .checked_add(b.missing)
        .ok_or("metric missing count overflow")?;
    a.failed = a
        .failed
        .checked_add(b.failed)
        .ok_or("metric failed count overflow")?;
    a.infeasible = a
        .infeasible
        .checked_add(b.infeasible)
        .ok_or("metric infeasible count overflow")?;
    Ok(())
}

fn status_metric(status: metric_cohorts::Status) -> &'static str {
    match status {
        metric_cohorts::Status::Computed => "computed",
        metric_cohorts::Status::Empty => "empty",
        metric_cohorts::Status::InsufficientData => "insufficient_data",
        metric_cohorts::Status::Invalid => "invalid",
        metric_cohorts::Status::Unverified => "unverified",
    }
}

fn metric_record(
    c: &MetricCommon,
    metric: &str,
    algorithm: &str,
    value: Option<f64>,
    units: &str,
) -> Result<Value, String> {
    if value.is_some_and(|v| !v.is_finite()) {
        return Err("metric kernel returned non-finite value".into());
    }
    if (c.status == "computed") != value.is_some() {
        return Err("metric value contradicts kernel status".into());
    }
    Ok(json!({
        "record_type":"calibration_metric.v1", "schema_version":"calibration-v1",
        "metric":metric, "algorithm_version":algorithm, "endpoint":c.endpoint,
        "strata":c.strata, "window":{"start_ticks":c.window_start,"end_ticks":c.window_end}, "units":units,
        "reference_count":c.reference, "simulation_count":c.simulation,
        "excluded_count":c.excluded, "censored_count":c.censored, "missing_count":c.missing,
        "unmatched_count":c.unmatched, "failed_count":c.failed, "infeasible_count":c.infeasible,
        "value":value, "status":c.status, "uncertainty":Value::Null,
        "provenance":{"dataset_id":c.dataset, "run_id":c.run, "mapping_version":c.mapping,
            "seed_contract_version":c.seed_contract,"parameter_hash":c.parameter_hash,
            "seed_schedule_id":c.seed_schedule,"seed_map_ref":c.seed_map}
    }))
}

fn valid_hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn same_identity(a: &Identity, b: &Identity) -> bool {
    a.dataset == b.dataset
        && a.endpoint == b.endpoint
        && a.unit == b.unit
        && a.mapping == b.mapping
        && a.seed_schedule == b.seed_schedule
        && a.seed_map == b.seed_map
}

fn resolve_causal_ref(
    reference: &str,
    run_id: &str,
    events: &[EventLogRecord],
) -> Result<(EventId, String), String> {
    let mut parts = reference.split(':');
    if parts.next() != Some("event") {
        return Err("causal_ref must use event:<index>:<generation>".into());
    }
    let index_text = parts.next().ok_or("causal_ref missing event index")?;
    let generation_text = parts.next().ok_or("causal_ref missing event generation")?;
    if parts.next().is_some() {
        return Err("causal_ref has extra components".into());
    }
    let index = index_text
        .parse::<u64>()
        .map_err(|_| "causal_ref has invalid event index")?;
    let generation = generation_text
        .parse::<u32>()
        .map_err(|_| "causal_ref has invalid event generation")?;
    if format!("event:{index}:{generation}") != reference {
        return Err("causal_ref is not canonical decimal".into());
    }
    let id = EventId::new(index, generation);
    let mut matches = events
        .iter()
        .filter(|e| e.run_id == run_id && e.event_id == id);
    if matches.next().is_none() {
        return Err("causal_ref event is missing from bound run".into());
    }
    if matches.next().is_some() {
        return Err("causal_ref event is duplicated in bound run".into());
    }
    let hex = [
        id.index.to_le_bytes().as_slice(),
        id.generation.to_le_bytes().as_slice(),
    ]
    .concat()
    .iter()
    .map(|b| format!("{b:02x}"))
    .collect::<String>();
    Ok((id, hex))
}

fn logical_key_json(k: &LogicalKey) -> Value {
    json!({"study_id":k.study_id,"dataset_id":k.dataset_id,"scenario_id":k.scenario_id,
        "seed_schedule_id":k.seed_schedule_id,"replication_id":k.replication_id,"case_key":k.case_key,
        "task_key":k.task_key,"occurrence":k.occurrence,"endpoint":k.endpoint,
        "seed_purpose":k.seed_purpose,"seed_map_ref":k.seed_map_ref,"mapping_version":k.mapping_version})
}

fn raw_residual_row(row: &SidecarRow) -> Value {
    json!({"key":logical_key_json(&row.row.key), "side":format!("{:?}", row.row.side),
        "group":{"strata":row.row.group.strata}, "source_time":row.row.source_time.map(|v|v.to_string()),
        "status":format!("{:?}", row.row.status), "observed_ticks":row.row.observed_ticks.map(|v|v.to_string()),
        "predicted_ticks":row.row.predicted_ticks.map(|v|v.to_string()), "prediction_unclamped":row.row.prediction_unclamped,
        "tick_unit":row.row.tick_unit, "provenance_supported":row.row.provenance_supported,
        "excluded":row.row.excluded, "infeasible":row.row.infeasible,
        "evidence":{"fidelity":row.evidence.fidelity,"anchor_role":row.evidence.anchor_role,
            "feasibility":row.evidence.feasibility,"censor_status":row.evidence.censor_status,
            "seed_contract_version":row.evidence.seed_contract_version,"parameter_hash":row.evidence.parameter_hash,
            "graph_hash":row.evidence.graph_hash,"causal_ref":row.evidence.causal_ref,"probe_id":row.evidence.probe_id}})
}

type ResidualSortKey = (
    (String, String, String, String, String, String),
    (String, u64, String, String, String, String),
    (String, String),
);

fn residual_sort_key(value: &Value) -> ResidualSortKey {
    let k = &value;
    (
        (
            k["study_id"].as_str().unwrap_or_default().into(),
            k["dataset_id"].as_str().unwrap_or_default().into(),
            k["scenario_id"].as_str().unwrap_or_default().into(),
            k["seed_schedule_id"].as_str().unwrap_or_default().into(),
            k["replication_id"].as_str().unwrap_or_default().into(),
            k["case_key"].as_str().unwrap_or_default().into(),
        ),
        (
            k["task_key"].as_str().unwrap_or_default().into(),
            k["occurrence"].as_u64().unwrap_or_default(),
            k["endpoint"].as_str().unwrap_or_default().into(),
            k["seed_purpose"].as_str().unwrap_or_default().into(),
            k["seed_map_ref"].as_str().unwrap_or_default().into(),
            k["mapping_version"].as_str().unwrap_or_default().into(),
        ),
        (
            k["candidate_id"].as_str().unwrap_or_default().into(),
            k["run_id"].as_str().unwrap_or_default().into(),
        ),
    )
}
fn metric_sort_key(value: &Value) -> (String, String, String, String, String, String, String) {
    (
        value["endpoint"].as_str().unwrap_or_default().into(),
        value["strata"].to_string(),
        value["window"].to_string(),
        value["units"].as_str().unwrap_or_default().into(),
        value["provenance"].to_string(),
        value["metric"].as_str().unwrap_or_default().into(),
        value["algorithm_version"]
            .as_str()
            .unwrap_or_default()
            .into(),
    )
}
