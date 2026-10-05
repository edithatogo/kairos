//! Actual C4.2 runtime → joined C4.3 sidecars → IPC/Parquet qualification.
#![cfg(any(feature = "ipc", feature = "parquet"))]
#[path = "../src/metric_cohorts.rs"]
mod metric_cohorts;
#[path = "../src/metrics.rs"]
mod metrics;
#[path = "../src/residuals.rs"]
mod residuals;
#[path = "../src/sidecar_adapter.rs"]
mod sidecar_adapter;

use kairo_ecs_arrow::{EventLogRecord, EventStatus, SCHEMA_VERSION};
use kairo_ecs_types::EventId;
use metric_cohorts::{Identity, Outcome, Row as MetricRow, Spec as MetricSpec};
use residuals::{GroupKey, LogicalKey, OutcomeStatus, Row, Side, Window};
use serde_json::{json, Value};
use std::{fs, path::PathBuf};
#[path = "../src/arrow_output.rs"]
mod arrow_output;
use sidecar_adapter::{
    build_sidecars, Evidence, GroupStratum, Input, MetricCohort, RunBinding, SidecarRow,
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

fn input() -> Input {
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

fn rich_input() -> Input {
    let mut x = input();
    x.reference_metric.rows = vec![metric_row("r1", "0", 5), metric_row("r2", "2", 5)];
    x.simulation_metric.rows = vec![metric_row("s1", "1", 5), metric_row("s2", "3", 5)];
    x.metric_spec.groups.push("empty".into());
    x.metric_strata.push(GroupStratum {
        group: "empty".into(),
        strata: json!({"candidate_id":"candidate-a","coverage":"empty"}),
    });
    for (case, rs, observed, ss, predicted) in [
        (
            "case-2",
            OutcomeStatus::Observed,
            Some(u128::MAX),
            OutcomeStatus::Predicted,
            Some(0),
        ),
        (
            "case-3",
            OutcomeStatus::Missing,
            None,
            OutcomeStatus::Predicted,
            Some(8),
        ),
        (
            "case-4",
            OutcomeStatus::Censored,
            None,
            OutcomeStatus::Predicted,
            Some(8),
        ),
        (
            "case-5",
            OutcomeStatus::Observed,
            Some(4),
            OutcomeStatus::Failed,
            None,
        ),
        (
            "case-7",
            OutcomeStatus::Observed,
            Some(1),
            OutcomeStatus::Predicted,
            Some(3),
        ),
        (
            "case-8",
            OutcomeStatus::Observed,
            Some(7),
            OutcomeStatus::Predicted,
            Some(9),
        ),
    ] {
        let mut a = residual_row(Side::Reference, case, rs, observed);
        let mut b = residual_row(Side::Simulation, case, ss, predicted);
        if rs == OutcomeStatus::Censored {
            a.evidence.censor_status = "right".into();
        }
        if case == "case-7" {
            a.row.source_time = Some(20);
            b.row.source_time = Some(20);
        }
        if case == "case-8" {
            b.row.infeasible = true;
            b.evidence.feasibility = "infeasible".into();
        }
        x.residual_rows.extend([a, b]);
    }
    x.residual_rows.push(residual_row(
        Side::Reference,
        "case-6",
        OutcomeStatus::Observed,
        Some(5),
    ));
    x
}
fn limits(chunk: usize) -> kairo_ecs_arrow_io::IoLimits {
    kairo_ecs_arrow_io::IoLimits {
        max_input_bytes: 1_000_000,
        max_output_bytes: 1_000_000,
        max_batch_rows: chunk,
        max_total_rows: 1000,
        max_batches: 1000,
        max_columns: 40,
    }
}
fn records(kind: &str, batches: &[arrow_array::RecordBatch]) -> Vec<Value> {
    batches
        .iter()
        .flat_map(|b| arrow_output::decode(kind, b).unwrap())
        .collect()
}
#[test]
fn c43_runtime_records_and_hashes_survive_permutation_and_physical_framing() {
    let first = build_sidecars(&rich_input()).expect("joined runtime records");
    let mut reversed = rich_input();
    reversed.residual_rows.reverse();
    reversed.events.reverse();
    reversed.reference_metric.rows.reverse();
    reversed.simulation_metric.rows.reverse();
    reversed.metric_spec.groups.reverse();
    reversed.metric_strata.reverse();
    let second = build_sidecars(&reversed).expect("permuted joined runtime records");
    assert_eq!(first.residuals, second.residuals);
    assert_eq!(first.metrics, second.metrics);
    let output = std::env::var_os("C43_OUTPUT_DIR").map(PathBuf::from);
    if let Some(ref dir) = output {
        fs::create_dir_all(dir).unwrap();
    }
    for (name, kind, logical) in [
        ("residual", "calibration_residual.v1", &first.residuals),
        ("metric", "calibration_metric.v1", &first.metrics),
    ] {
        let batch = arrow_output::encode(kind, logical).unwrap();
        let canonical = arrow_output::decode(kind, &batch).unwrap();
        let canonical_bytes = serde_json::to_vec(&canonical).unwrap();
        let expected_hash = <sha2::Sha256 as sha2::Digest>::digest(&canonical_bytes);
        if let Some(ref dir) = output {
            fs::write(dir.join(format!("{name}.json")), &canonical_bytes).unwrap();
        }
        for chunk in [1, 2, 64] {
            let batches: Vec<_> = (0..batch.num_rows())
                .step_by(chunk)
                .map(|i| batch.slice(i, chunk.min(batch.num_rows() - i)))
                .collect();
            let schema = arrow_output::schema(kind).unwrap();
            #[cfg(feature = "ipc")]
            for (format, bytes) in [
                (
                    "ipc_file",
                    kairo_ecs_arrow_io::write_ipc_file(schema.clone(), &batches, limits(chunk))
                        .unwrap(),
                ),
                (
                    "ipc_stream",
                    kairo_ecs_arrow_io::write_ipc_stream(schema.clone(), &batches, limits(chunk))
                        .unwrap(),
                ),
            ] {
                let read = if format == "ipc_file" {
                    kairo_ecs_arrow_io::read_ipc_file(&bytes, schema.clone(), limits(chunk))
                } else {
                    kairo_ecs_arrow_io::read_ipc_stream(&bytes, schema.clone(), limits(chunk))
                }
                .unwrap();
                let actual = records(kind, &read);
                assert_eq!(canonical, actual);
                assert_eq!(
                    expected_hash,
                    <sha2::Sha256 as sha2::Digest>::digest(serde_json::to_vec(&actual).unwrap())
                );
                if let Some(ref dir) = output {
                    fs::write(dir.join(format!("{name}.{format}")), bytes).unwrap();
                }
            }
            #[cfg(feature = "parquet")]
            {
                let bytes =
                    kairo_ecs_arrow_io::write_parquet(schema.clone(), &batches, limits(chunk))
                        .unwrap();
                let actual = records(
                    kind,
                    &kairo_ecs_arrow_io::read_parquet(&bytes, schema, limits(chunk)).unwrap(),
                );
                assert_eq!(canonical, actual);
                assert_eq!(
                    expected_hash,
                    <sha2::Sha256 as sha2::Digest>::digest(serde_json::to_vec(&actual).unwrap())
                );
                if let Some(ref dir) = output {
                    fs::write(dir.join(format!("{name}.parquet")), bytes).unwrap();
                }
            }
        }
    }
    if let Some(ref dir) = output {
        fs::write(
            dir.join("join_manifest.json"),
            serde_json::to_vec(&sidecar_adapter::join_manifest_json(&first)).unwrap(),
        )
        .unwrap();
        let x = rich_input();
        let events: Vec<_> = x.events.iter().map(|e|json!({"run_id":e.run_id,"event_id":format!("event:{}:{}",e.event_id.index,e.event_id.generation),"time_ticks":e.time_ticks.to_string()})).collect();
        fs::write(dir.join("source_manifest.json"),serde_json::to_vec(&json!({"runs":[{"run_id":"run-a","candidate_id":"candidate-a","dataset_id":"dataset","scenario_id":"scenario","study_id":"study","replication_id":"rep-1","seed_schedule_id":"schedule-v1","seed_map_ref":"map-v1","mapping_version":"mapping-v1","parameter_hash":"a".repeat(64)}],"events":events,"source_rows":x.residual_rows.len(),"source_window":{"start_ticks":"0","end_ticks":"20"}})).unwrap()).unwrap();
    }
}
