#[path = "../src/metric_cohorts.rs"]
mod metric_cohorts;
#[path = "../src/metrics.rs"]
mod metrics;
#[path = "../src/residuals.rs"]
mod residuals;
mod seed_map {
    // Mirrors the actual `SEED_MAP_VERSION_V1` source constant without
    // re-running the source module's unit tests in this integration target.
    pub(crate) const SEED_MAP_VERSION_V1: u32 = 1;
}
#[path = "../src/sidecar_adapter.rs"]
mod sidecar_adapter;

use kairo_ecs_arrow::{EventLogRecord, EventStatus, SCHEMA_VERSION};
use kairo_ecs_types::EventId;
use metric_cohorts::{Identity, Outcome, Row as MetricRow, Spec as MetricSpec};
use residuals::{GroupKey, LogicalKey, OutcomeStatus, Row, Side, Window};
use serde_json::{json, Value};
use sidecar_adapter::{
    build_sidecars, join_manifest_json, Evidence, GroupStratum, Input, MetricCohort, RunBinding,
    SidecarRow,
};

fn binding() -> RunBinding {
    RunBinding {
        study_id: "study".into(),
        dataset_id: "dataset".into(),
        scenario_id: "scenario".into(),
        candidate_id: "candidate-a".into(),
        run_id: "run-a".into(),
        replication_id: "rep-1".into(),
        seed_schedule_id: "schedule-v1".into(),
        seed_map_ref: "map-v1".into(),
        seed_map_version: 1,
        seed_contract_version: "kairoecs.seed-purpose.v1".into(),
        mapping_version: "mapping-v1".into(),
        fidelity: "ShadowAnchored".into(),
        parameter_hash: "a".repeat(64),
    }
}

fn logical(case: &str) -> LogicalKey {
    LogicalKey {
        study_id: "study".into(),
        dataset_id: "dataset".into(),
        scenario_id: "scenario".into(),
        seed_schedule_id: "schedule-v1".into(),
        replication_id: "rep-1".into(),
        case_key: case.into(),
        task_key: "task".into(),
        occurrence: 0,
        endpoint: "departure".into(),
        seed_purpose: "service".into(),
        seed_map_ref: "map-v1".into(),
        mapping_version: "mapping-v1".into(),
    }
}

fn evidence(causal_ref: Option<&str>, probe_id: Option<&str>) -> Evidence {
    Evidence {
        fidelity: "ShadowAnchored".into(),
        anchor_role: "observed_source_transition".into(),
        feasibility: "feasible".into(),
        censor_status: "not_censored".into(),
        seed_contract_version: "kairoecs.seed-purpose.v1".into(),
        parameter_hash: "a".repeat(64),
        graph_hash: None,
        causal_ref: causal_ref.map(str::to_owned),
        probe_id: probe_id.map(str::to_owned),
    }
}

fn residual_row(side: Side, case: &str, status: OutcomeStatus, ticks: Option<u128>) -> SidecarRow {
    let row = match side {
        Side::Reference => Row {
            key: logical(case),
            side,
            group: GroupKey::default(),
            source_time: Some(5),
            status,
            observed_ticks: ticks,
            predicted_ticks: None,
            prediction_unclamped: false,
            tick_unit: "nanosecond".into(),
            provenance_supported: true,
            excluded: false,
            infeasible: false,
        },
        Side::Simulation => Row {
            key: logical(case),
            side,
            group: GroupKey::default(),
            source_time: Some(5),
            status,
            observed_ticks: None,
            predicted_ticks: ticks,
            prediction_unclamped: ticks.is_some(),
            tick_unit: "nanosecond".into(),
            provenance_supported: true,
            excluded: false,
            infeasible: status == OutcomeStatus::Infeasible,
        },
    };
    SidecarRow {
        row,
        evidence: evidence(None, (side == Side::Simulation).then_some("probe-1")),
    }
}

fn metric_identity() -> Identity {
    Identity {
        dataset: "dataset".into(),
        endpoint: "departure".into(),
        unit: "nanosecond".into(),
        mapping: "mapping-v1".into(),
        seed_schedule: "schedule-v1".into(),
        seed_map: "map-v1".into(),
    }
}

fn metric_spec() -> MetricSpec {
    MetricSpec {
        identity: metric_identity(),
        groups: vec!["candidate-a".into()],
        window: Some((0, 20)),
        algorithm_version: "empirical_equal.v1".into(),
        origin: None,
        scale_ticks: "1".into(),
        provenance_verified: true,
    }
}

fn metric_row(key: &str, value: &str, time: u128) -> MetricRow {
    MetricRow {
        key: key.into(),
        group: "candidate-a".into(),
        selection_time: Some(time),
        value: Some(value.into()),
        weight: None,
        outcome: Outcome::Point,
        excluded: false,
        censored: false,
        missing: false,
        failed: false,
        infeasible: false,
    }
}

pub(crate) fn fixture() -> Input {
    let event = EventLogRecord {
        schema_version: SCHEMA_VERSION,
        run_id: "run-a".into(),
        event_id: EventId::new(4, 2),
        entity_id: None,
        time_ticks: 12,
        time_scale: "ticks".into(),
        priority: 0,
        sequence: 1,
        event_kind: "custom:1".into(),
        status: EventStatus::Dispatched,
        payload_ref: None,
    };
    let mut ref_row = residual_row(Side::Reference, "case-1", OutcomeStatus::Observed, Some(10));
    ref_row.evidence.causal_ref = Some("event:4:2".into());
    let sim_row = residual_row(
        Side::Simulation,
        "case-1",
        OutcomeStatus::Predicted,
        Some(12),
    );
    Input {
        binding: binding(),
        run_records: vec![binding()],
        source_window: Window {
            start: 0,
            end_exclusive: 20,
        },
        residual_groups: vec![GroupKey::default()],
        residual_rows: vec![ref_row, sim_row],
        metric_spec: metric_spec(),
        reference_metric: MetricCohort {
            identity: metric_identity(),
            rows: vec![metric_row("r1", "10", 5)],
        },
        simulation_metric: MetricCohort {
            identity: metric_identity(),
            rows: vec![metric_row("s1", "12", 5)],
        },
        metric_strata: vec![GroupStratum {
            group: "candidate-a".into(),
            strata: json!({"candidate_id":"candidate-a"}),
        }],
        events: vec![event],
    }
}

#[test]
fn c43_emits_c0_sidecars_from_kernels_and_joins_explicit_event() {
    let out = build_sidecars(&fixture()).unwrap();
    assert_eq!(out.residuals.len(), 1);
    assert_eq!(out.residuals[0]["residual_status"], "computed");
    assert_eq!(out.residuals[0]["residual_sign"], "positive");
    assert_eq!(out.residuals[0]["residual_magnitude"], "2");
    assert_eq!(out.residuals[0]["causal_ref"], "event:4:2");
    assert_eq!(out.metrics.len(), 5);
    assert_eq!(out.joins[0].event_resolved, Some(true));
    assert_eq!(out.joins[0].probe_id.as_deref(), Some("probe-1"));
    let manifest = join_manifest_json(&out);
    assert_eq!(
        manifest["rows"][0]["event_id_le_hex"],
        "040000000000000002000000"
    );
    assert_eq!(
        manifest["rows"][0]["raw_records"].as_array().unwrap().len(),
        2
    );
}

#[test]
fn c43_max_u128_is_exact_and_window_exclusion_does_not_rewrite_point_residual() {
    let mut i = fixture();
    i.residual_rows[0].row.observed_ticks = Some(0);
    i.residual_rows[0].row.excluded = true;
    i.residual_rows[1].row.predicted_ticks = Some(u128::MAX);
    i.residual_rows[1].row.source_time = Some(30);
    i.residual_rows[0].row.source_time = Some(30);
    i.residual_rows[1].row.excluded = true;
    i.source_window = Window {
        start: 0,
        end_exclusive: 20,
    };
    let out = build_sidecars(&i).unwrap();
    assert_eq!(
        out.residuals[0]["residual_magnitude"],
        u128::MAX.to_string()
    );
    assert_eq!(out.residuals[0]["residual_status"], "computed");
    assert!(!out.joins[0].eligible);
    assert!(out.joins[0].excluded);
}

#[test]
fn c43_noncomputed_statuses_keep_null_contract_and_overlapping_counts() {
    let mut i = fixture();
    i.residual_rows[1].row.status = OutcomeStatus::Censored;
    i.residual_rows[1].row.predicted_ticks = None;
    i.residual_rows[1].row.prediction_unclamped = false;
    i.residual_rows[1].evidence.censor_status = "right".into();
    i.residual_rows[1].row.excluded = true;
    let out = build_sidecars(&i).unwrap();
    assert_eq!(out.residuals[0]["residual_status"], "censored");
    assert_eq!(out.residuals[0]["residual_sign"], "undefined");
    assert_eq!(out.residuals[0]["residual_magnitude"], Value::Null);
    let paired = out
        .metrics
        .iter()
        .find(|m| m["metric"] == "paired_residual_summary")
        .unwrap();
    assert_eq!(paired["censored_count"], 1);
    assert_eq!(paired["excluded_count"], 1);
}

#[test]
fn c43_in_window_nonpoint_is_ineligible_but_not_window_excluded() {
    let mut i = fixture();
    i.residual_rows[1].row.status = OutcomeStatus::Censored;
    i.residual_rows[1].row.predicted_ticks = None;
    i.residual_rows[1].row.prediction_unclamped = false;
    i.residual_rows[1].evidence.censor_status = "right".into();
    let out = build_sidecars(&i).unwrap();
    assert!(!out.joins[0].eligible);
    assert!(!out.joins[0].excluded);
}

#[test]
fn c43_invalid_run_event_hash_and_duplicate_key_fail_with_raw_diagnostics() {
    let mut i = fixture();
    i.run_records.push(binding());
    let err = build_sidecars(&i).unwrap_err();
    assert!(!err.raw_rows.is_empty());
    assert_eq!(err.raw_diagnostics["residual_raw_rows"], err.raw_rows.len());
    assert_eq!(
        err.raw_diagnostics["residual_raw_rows_canonical_sha256"]
            .as_str()
            .unwrap()
            .len(),
        64
    );
    assert_eq!(err.metric_raw_rows.len(), 2);

    let mut i = fixture();
    i.residual_rows[0].evidence.causal_ref = Some("event:5:2".into());
    assert!(build_sidecars(&i).is_err());

    let mut i = fixture();
    i.residual_rows[1].evidence.parameter_hash = "z".repeat(64);
    assert!(build_sidecars(&i).is_err());

    let mut i = fixture();
    i.residual_rows.push(i.residual_rows[0].clone());
    assert!(build_sidecars(&i).is_err());

    let mut i = fixture();
    i.events.push(i.events[0].clone());
    assert!(build_sidecars(&i).is_err());
}

#[test]
fn c43_unmatched_missing_failed_and_infeasible_pairs_keep_schema_statuses() {
    let mut i = fixture();
    i.residual_rows.pop();
    let out = build_sidecars(&i).unwrap();
    assert_eq!(out.residuals[0]["residual_status"], "probe_failed");
    assert_eq!(out.residuals[0]["predicted_ticks"], Value::Null);

    let mut i = fixture();
    i.residual_rows.remove(0);
    let out = build_sidecars(&i).unwrap();
    assert_eq!(out.residuals[0]["residual_status"], "missing_observed");
    assert_eq!(out.residuals[0]["observed_ticks"], Value::Null);

    let mut i = fixture();
    i.residual_rows[1].row.status = OutcomeStatus::Failed;
    i.residual_rows[1].row.predicted_ticks = None;
    i.residual_rows[1].row.prediction_unclamped = false;
    let out = build_sidecars(&i).unwrap();
    assert_eq!(out.residuals[0]["residual_status"], "probe_failed");

    let mut i = fixture();
    i.residual_rows[1].row.status = OutcomeStatus::Infeasible;
    i.residual_rows[1].row.predicted_ticks = None;
    i.residual_rows[1].row.prediction_unclamped = false;
    i.residual_rows[1].row.infeasible = true;
    i.residual_rows[1].evidence.feasibility = "infeasible".into();
    let out = build_sidecars(&i).unwrap();
    assert_eq!(out.residuals[0]["residual_status"], "infeasible");
    assert_eq!(out.residuals[0]["residual_magnitude"], Value::Null);
}

#[test]
fn c43_empty_residual_population_still_emits_declared_empty_metric_summaries() {
    let mut i = fixture();
    i.residual_rows.clear();
    let out = build_sidecars(&i).unwrap();
    assert!(out.residuals.is_empty());
    assert_eq!(out.metrics.len(), 5);
    assert!(out
        .metrics
        .iter()
        .filter(|m| m["metric"] == "paired_residual_summary")
        .all(|m| m["status"] == "empty" && m["value"].is_null()));
}

#[test]
fn c43_paired_metric_denominator_is_matched_pairs_and_manifest_keeps_cohort_counts() {
    let mut i = fixture();
    i.residual_rows.push(residual_row(
        Side::Reference,
        "case-2",
        OutcomeStatus::Observed,
        Some(20),
    ));
    let out = build_sidecars(&i).unwrap();
    let paired = out
        .metrics
        .iter()
        .find(|m| m["metric"] == "paired_residual_summary")
        .unwrap();
    assert_eq!(paired["reference_count"], 1);
    assert_eq!(paired["simulation_count"], 1);
    assert_eq!(paired["unmatched_count"], 1);
    assert_eq!(out.group_diagnostics[0]["eligible_reference_count"], 2);
    assert_eq!(out.group_diagnostics[0]["eligible_simulation_count"], 1);
    assert_eq!(out.group_diagnostics[0]["matched_count"], 1);
    assert_eq!(out.group_diagnostics[0]["unmatched_count"], 1);
    let manifest = join_manifest_json(&out);
    assert_eq!(manifest["metric_raw_records"].as_array().unwrap().len(), 2);
    assert_eq!(
        manifest["counts"]["metric_raw_rows_canonical_sha256"]
            .as_str()
            .unwrap()
            .len(),
        64
    );
}

#[test]
fn c43_structurally_invalid_metric_rows_fail_with_both_raw_cohorts() {
    let mut i = fixture();
    i.reference_metric
        .rows
        .push(i.reference_metric.rows[0].clone());
    let err = build_sidecars(&i).unwrap_err();
    assert!(err.reason.contains("duplicate source key"));
    assert_eq!(err.metric_raw_rows.len(), 3);

    let mut i = fixture();
    i.simulation_metric.rows[0].outcome = Outcome::Missing;
    let err = build_sidecars(&i).unwrap_err();
    assert!(err.reason.contains("point/value status contradiction"));
    assert_eq!(err.metric_raw_rows.len(), 2);
}

#[test]
fn c43_input_permutation_preserves_logical_records_and_metric_bits() {
    let first = build_sidecars(&fixture()).unwrap();
    let mut permuted = fixture();
    permuted.residual_rows.reverse();
    permuted.reference_metric.rows.reverse();
    permuted.simulation_metric.rows.reverse();
    permuted.events.reverse();
    let second = build_sidecars(&permuted).unwrap();
    assert_eq!(first.residuals, second.residuals);
    assert_eq!(first.metrics, second.metrics);
    assert_eq!(first.joins, second.joins);
    assert_eq!(first.raw_diagnostics, second.raw_diagnostics);
    assert_eq!(first.metric_raw_rows, second.metric_raw_rows);
}

#[test]
fn c43_failed_prediction_preserves_reference_censor_and_missing_lineage() {
    for (status, category) in [
        (OutcomeStatus::Censored, "right"),
        (OutcomeStatus::Missing, "missing"),
    ] {
        let mut i = fixture();
        i.residual_rows[0].row.status = status;
        i.residual_rows[0].row.observed_ticks = None;
        i.residual_rows[0].evidence.censor_status = category.into();
        i.residual_rows[1].row.status = OutcomeStatus::Failed;
        i.residual_rows[1].row.predicted_ticks = None;
        i.residual_rows[1].row.prediction_unclamped = false;
        let out = build_sidecars(&i).unwrap();
        assert_eq!(out.residuals[0]["residual_status"], "probe_failed");
        assert_eq!(out.residuals[0]["censor_status"], category);
    }
}
