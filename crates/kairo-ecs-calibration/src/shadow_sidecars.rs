//! C3 terminal inventories joined to the existing C4 physical schemas.
//!
//! The frozen v1 physical `fidelity` field historically stores replay policy.
//! Preserve that wire convention; actual Macro/Micro execution mode is explicit
//! in metric strata and the join manifest's raw residual groups. Never relabel
//! a Micro model as Macro or confuse execution mode with replay policy.
use crate::metric_cohorts::{Identity, Outcome, Row as MetricRow, Spec as MetricSpec};
use crate::residuals::{GroupKey, OutcomeStatus, Side, Window};
use crate::shadow::{ProbeResult, ProbeSpec};
use crate::sidecar_adapter::{
    self, Evidence, GroupStratum, Input, MetricCohort, Output, RunBinding, SidecarRow,
};
use serde_json::json;
use std::collections::BTreeMap;

pub(crate) fn build(
    results: &[ProbeResult],
    specs: &[ProbeSpec],
    binding: RunBinding,
    window: Window,
    graph_hash: Option<String>,
) -> Result<Output, String> {
    let mut rows = crate::shadow_report::project_rows(results, specs)
        .map_err(|e| format!("invalid C3 inventory: {e:?}"))?;
    let first = specs
        .first()
        .ok_or("C3 sidecar export needs an explicit nonempty endpoint inventory")?;
    if binding.fidelity != "ShadowAnchored" {
        return Err("C3 physical v1 binding requires ShadowAnchored replay policy".into());
    }
    let execution_mode = &first.input.fidelity;
    let endpoint = &first.key.endpoint;
    let parameter_hash = first
        .input
        .parameter_hash
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    if binding.parameter_hash != parameter_hash {
        return Err("C3 parameter hash differs from run binding".into());
    }
    let mut spec_by_key = BTreeMap::new();
    for spec in specs {
        if spec.run_id != binding.run_id
            || spec.candidate_id != binding.candidate_id
            || spec.input.fidelity != *execution_mode
            || spec.key.endpoint != *endpoint
            || spec.input.parameter_hash != first.input.parameter_hash
            || spec.input.adapter_hash != first.input.adapter_hash
        {
            return Err(
                "C3 export must contain one bound run, candidate, mode, adapter and endpoint"
                    .into(),
            );
        }
        spec_by_key.insert(&spec.key, spec);
    }
    let group = GroupKey {
        strata: vec![
            ("execution_mode".into(), execution_mode.clone()),
            ("replay_role".into(), "ShadowAnchored".into()),
        ],
    };
    let identity = Identity {
        dataset: binding.dataset_id.clone(),
        endpoint: endpoint.clone(),
        unit: "nanosecond".into(),
        mapping: binding.mapping_version.clone(),
        seed_schedule: binding.seed_schedule_id.clone(),
        seed_map: binding.seed_map_ref.clone(),
    };
    let mut reference = Vec::new();
    let mut simulation = Vec::new();
    let mut residual_rows = Vec::new();
    for mut row in rows.drain(..) {
        let spec = spec_by_key.get(&row.key).ok_or("unbound C3 row")?;
        row.group = group.clone();
        let ticks = match row.side {
            Side::Reference => row.observed_ticks,
            Side::Simulation => row.predicted_ticks,
        };
        let outcome = if ticks.is_some() {
            Outcome::Point
        } else {
            match row.status {
                OutcomeStatus::Censored => Outcome::Censored,
                OutcomeStatus::Failed => Outcome::Failed,
                OutcomeStatus::Infeasible => Outcome::Infeasible,
                _ => Outcome::Missing,
            }
        };
        let metric = MetricRow {
            key: spec.id.clone(),
            group: binding.candidate_id.clone(),
            selection_time: row.source_time,
            value: ticks.map(|t| t.to_string()),
            weight: None,
            outcome,
            excluded: false,
            censored: outcome == Outcome::Censored,
            missing: outcome == Outcome::Missing,
            failed: outcome == Outcome::Failed,
            infeasible: row.infeasible,
        };
        match row.side {
            Side::Reference => reference.push(metric),
            Side::Simulation => simulation.push(metric),
        }
        let evidence = Evidence {
            fidelity: "ShadowAnchored".into(),
            anchor_role: "observed_source_transition".into(),
            feasibility: if row.infeasible {
                "infeasible"
            } else {
                "feasible"
            }
            .into(),
            censor_status: match row.status {
                OutcomeStatus::Censored => "right",
                OutcomeStatus::Missing => "missing",
                _ => "not_censored",
            }
            .into(),
            seed_contract_version: binding.seed_contract_version.clone(),
            parameter_hash: binding.parameter_hash.clone(),
            graph_hash: graph_hash.clone(),
            causal_ref: None,
            probe_id: (row.side == Side::Simulation).then(|| spec.id.clone()),
        };
        residual_rows.push(SidecarRow { row, evidence });
    }
    let input = Input {
        binding: binding.clone(),
        run_records: vec![binding.clone()],
        source_window: window,
        residual_groups: vec![group],
        residual_rows,
        metric_spec: MetricSpec {
            identity: identity.clone(),
            groups: vec![binding.candidate_id.clone()],
            window: Some((window.start, window.end_exclusive)),
            algorithm_version: "empirical_equal.v1".into(),
            origin: None,
            scale_ticks: "1".into(),
            provenance_verified: true,
        },
        reference_metric: MetricCohort {
            identity: identity.clone(),
            rows: reference,
        },
        simulation_metric: MetricCohort {
            identity,
            rows: simulation,
        },
        metric_strata: vec![GroupStratum {
            group: binding.candidate_id.clone(),
            strata: json!({"candidate_id":binding.candidate_id,"execution_mode":execution_mode,"replay_role":"ShadowAnchored"}),
        }],
        events: vec![],
    };
    sidecar_adapter::build_sidecars(&input).map_err(|error| format!("C4 join: {error:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::residuals::LogicalKey;
    use crate::seed_map::{CalibrationSeedMap, SeedPurpose};
    use crate::shadow::{ProbeBudget, ProbeInput, ProbeOutcome};
    fn fixture() -> (Vec<ProbeResult>, Vec<ProbeSpec>, RunBinding) {
        let mut seeds = CalibrationSeedMap::new(1, "study", 42).unwrap();
        let seed_key = seeds
            .stream_for("schedule", 1, "case", "task", SeedPurpose::Service)
            .unwrap()
            .key();
        let spec = ProbeSpec {
            id: "probe".into(),
            key: LogicalKey {
                study_id: "study".into(),
                dataset_id: "data".into(),
                scenario_id: "scenario".into(),
                seed_schedule_id: "schedule".into(),
                replication_id: "1".into(),
                case_key: "case".into(),
                task_key: "task".into(),
                occurrence: 0,
                endpoint: "completion".into(),
                seed_purpose: "service".into(),
                seed_map_ref: "map".into(),
                mapping_version: "mapping".into(),
            },
            run_id: "run".into(),
            candidate_id: "candidate".into(),
            anchor_event: "anchor".into(),
            target_event: Some("target".into()),
            observed_target: Some(5),
            input: ProbeInput {
                target: "completion".into(),
                seed_key,
                parameter_hash: [1; 32],
                adapter_hash: [2; 32],
                fidelity: "Micro".into(),
            },
            budget: ProbeBudget {
                horizon: 20,
                max_events: 10,
            },
        };
        let result = ProbeResult {
            id: "probe".into(),
            anchor_event: "anchor".into(),
            frontier: 1,
            snapshot_digest: [3; 32],
            observed: Some(5),
            outcome: ProbeOutcome::Completed { predicted: 8 },
            events: 2,
            last_tick: 8,
            assumptions: vec![],
        };
        let binding = RunBinding {
            study_id: "study".into(),
            dataset_id: "data".into(),
            scenario_id: "scenario".into(),
            candidate_id: "candidate".into(),
            run_id: "run".into(),
            replication_id: "1".into(),
            seed_schedule_id: "schedule".into(),
            seed_map_ref: "map".into(),
            seed_map_version: 1,
            seed_contract_version: "kairoecs.seed-purpose.v1".into(),
            mapping_version: "mapping".into(),
            fidelity: "ShadowAnchored".into(),
            parameter_hash: "01".repeat(32),
        };
        (vec![result], vec![spec], binding)
    }
    fn export(r: &[ProbeResult], s: &[ProbeSpec], b: RunBinding) -> Output {
        build(
            r,
            s,
            b,
            Window {
                start: 0,
                end_exclusive: 20,
            },
            None,
        )
        .unwrap()
    }
    #[test]
    fn late_prediction_and_execution_mode_survive_existing_physical_sidecars() {
        let (r, s, b) = fixture();
        let output = export(&r, &s, b);
        assert_eq!(output.residuals[0]["observed_ticks"], "5");
        assert_eq!(output.residuals[0]["predicted_ticks"], "8");
        assert_eq!(output.residuals[0]["residual_magnitude"], "3");
        assert_eq!(output.residuals[0]["prediction_unclamped"], true);
        assert!(output
            .metrics
            .iter()
            .all(|m| m["strata"]["execution_mode"] == "Micro"
                && m["strata"]["replay_role"] == "ShadowAnchored"));
        let manifest = sidecar_adapter::join_manifest_json(&output);
        assert!(manifest.to_string().contains("execution_mode"));
        for (kind, records) in [
            ("calibration_residual.v1", &output.residuals),
            ("calibration_metric.v1", &output.metrics),
        ] {
            let batch = crate::arrow_output::encode(kind, records).unwrap();
            let schema = crate::arrow_output::schema(kind).unwrap();
            let limits = kairo_ecs_arrow_io::IoLimits {
                max_input_bytes: 1_000_000,
                max_output_bytes: 1_000_000,
                max_batch_rows: 100,
                max_total_rows: 100,
                max_batches: 100,
                max_columns: 40,
            };
            #[cfg(feature = "ipc")]
            {
                let bytes = kairo_ecs_arrow_io::write_ipc_file(
                    schema.clone(),
                    std::slice::from_ref(&batch),
                    limits,
                )
                .unwrap();
                let read =
                    kairo_ecs_arrow_io::read_ipc_file(&bytes, schema.clone(), limits).unwrap();
                assert_eq!(
                    crate::arrow_output::decode(kind, &read[0]).unwrap(),
                    *records
                );
            }
            #[cfg(feature = "parquet")]
            {
                let bytes = kairo_ecs_arrow_io::write_parquet(
                    schema.clone(),
                    std::slice::from_ref(&batch),
                    limits,
                )
                .unwrap();
                let read = kairo_ecs_arrow_io::read_parquet(&bytes, schema, limits).unwrap();
                assert_eq!(
                    crate::arrow_output::decode(kind, &read[0]).unwrap(),
                    *records
                );
            }
        }
    }
    #[test]
    fn failed_overshoot_is_preserved_as_failure_without_fabricating_prediction() {
        let (mut r, s, b) = fixture();
        r[0].events = 11;
        r[0].last_tick = 21;
        r[0].outcome = ProbeOutcome::Failed {
            reason: "native hook exceeded budget".into(),
        };
        let output = export(&r, &s, b);
        assert!(output.residuals[0]["predicted_ticks"].is_null());
        assert!(output.residuals[0]["residual_magnitude"].is_null());
    }
    #[test]
    fn mismatched_candidate_mode_hash_or_inventory_rejects() {
        let (r, mut s, b) = fixture();
        s[0].candidate_id = "other".into();
        assert!(build(
            &r,
            &s,
            b.clone(),
            Window {
                start: 0,
                end_exclusive: 20
            },
            None
        )
        .is_err());
        let (_, s, _) = fixture();
        let mut bad = b;
        bad.parameter_hash = "ff".repeat(32);
        assert!(build(
            &r,
            &s,
            bad,
            Window {
                start: 0,
                end_exclusive: 20
            },
            None
        )
        .is_err());
    }
}
