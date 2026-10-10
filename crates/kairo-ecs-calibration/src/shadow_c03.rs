//! Synthetic C-03 acceptance against actual Flow, C2 images and process boundaries.
use crate::checkpoint_envelope::{self, CheckpointBindingV1, CheckpointEnvelopeLimits};
use crate::residuals::LogicalKey;
use crate::shadow::*;
use crate::shadow_c2_model::{SyntheticFlowConfig, SyntheticFlowProbeModel, SyntheticWorld};
use crate::shadow_ledger::{InitialClaim, InitialResource, LedgerLimits, ObservedLedger};
use crate::shadow_native::{FlowProbeModel, NativeFlowAdapter};
use crate::shadow_pool::PoolLimits;
use crate::shadow_runner::{self, ShadowRunner, ShadowRunnerCheckpoint, TrustedRunDefinition};
use crate::shadow_wire::{self, RunImage, WireLimits};
use crate::trace_order::{EventKindRank, TraceOrderKeyV1};
use kairo_ecs_des::{FlowCheckpointCodecs, FlowCheckpointLimits, FlowDispatch, FlowRuntime};
use serde_json::{json, Value};
use std::{collections::BTreeMap, path::Path, process::Command};

struct GuardedModel {
    inner: SyntheticFlowProbeModel,
    forbid_start: bool,
}
impl FlowProbeModel for GuardedModel {
    type World = SyntheticWorld;
    fn start(&self, s: &LedgerSnapshot, i: &ProbeInput) -> Result<Self::World, ShadowError> {
        if self.forbid_start {
            return Err(ShadowError::Contract("restore attempted prediction start"));
        }
        self.inner.start(s, i)
    }
    fn flow<'a>(&self, w: &'a Self::World) -> &'a FlowRuntime {
        self.inner.flow(w)
    }
    fn flow_mut<'a>(&self, w: &'a mut Self::World) -> &'a mut FlowRuntime {
        self.inner.flow_mut(w)
    }
    fn codecs<'a>(&self, w: &'a Self::World) -> &'a FlowCheckpointCodecs {
        self.inner.codecs(w)
    }
    fn capture_limits(&self) -> FlowCheckpointLimits {
        self.inner.capture_limits()
    }
    fn completed(&self, w: &Self::World) -> Result<bool, ShadowError> {
        self.inner.completed(w)
    }
    fn after_dispatch(&self, w: &mut Self::World, d: &FlowDispatch) -> Result<(), ShadowError> {
        self.inner.after_dispatch(w, d)
    }
    fn checkpoint(&self, w: &Self::World, cap: usize) -> Result<Vec<u8>, ShadowError> {
        self.inner.checkpoint(w, cap)
    }
    fn restore(
        &self,
        s: &LedgerSnapshot,
        i: &ProbeInput,
        b: &[u8],
    ) -> Result<Self::World, ShadowError> {
        self.inner.restore(s, i, b)
    }
}
type Runner = ShadowRunner<NativeFlowAdapter<GuardedModel>>;
const IMAGE_CAP: usize = 4 * 1024 * 1024;
fn config() -> SyntheticFlowConfig {
    SyntheticFlowConfig {
        max_horizon_ticks: 100,
        ..Default::default()
    }
}
fn adapter(c: SyntheticFlowConfig, forbid_start: bool) -> NativeFlowAdapter<GuardedModel> {
    NativeFlowAdapter {
        model: GuardedModel {
            inner: SyntheticFlowProbeModel { config: c },
            forbid_start,
        },
        max_image_bytes: IMAGE_CAP,
    }
}
fn event(key: &str, tick: u128, source_order: u64) -> ObservedEvent {
    ObservedEvent {
        order: TraceOrderKeyV1 {
            relative_ticks: tick,
            case_key: "case".into(),
            occurrence: 0,
            event_kind_rank: EventKindRank::from_canonical_decimal("1").unwrap(),
            source_event_key: key.into(),
            source_order,
        },
        available_at: Some(tick),
        source_defined: true,
        transition: Transition::None,
        payload: vec![],
    }
}
fn definition(c: &SyntheticFlowConfig, observed: Option<u128>) -> TrustedRunDefinition {
    let model = SyntheticFlowProbeModel { config: c.clone() };
    let input = model.probe_input().unwrap();
    let mut events = vec![event("anchor", 0, 0)];
    if let Some(t) = observed {
        events.push(event("target", t, 1));
    }
    let spec = ProbeSpec {
        id: "p0".into(),
        key: LogicalKey {
            study_id: "c3-native-fixture".into(),
            dataset_id: "synthetic-c03".into(),
            scenario_id: "route-service".into(),
            seed_schedule_id: c.seed_schedule.clone(),
            replication_id: c.replication.to_string(),
            case_key: c.case_key.clone(),
            task_key: c.task_key.clone(),
            occurrence: 0,
            endpoint: input.target.clone(),
            seed_purpose: "service".into(),
            seed_map_ref: "c3-seed-map-v1".into(),
            mapping_version: "c3-synthetic-v1".into(),
        },
        run_id: "c03-run".into(),
        candidate_id: "c03-candidate".into(),
        anchor_event: "anchor".into(),
        target_event: observed.map(|_| "target".into()),
        observed_target: None,
        input,
        budget: ProbeBudget {
            horizon: 100,
            max_events: 64,
        },
    };
    TrustedRunDefinition {
        identity: b"synthetic-c03-acceptance-v1".to_vec(),
        events,
        initial_resources: BTreeMap::from([(
            c.target_resource.clone(),
            InitialResource {
                capacity: 2,
                claims: vec![InitialClaim {
                    id: "known-holder".into(),
                    units: 1,
                }],
            },
        )]),
        assumptions: vec![
            "Known initial holder remains occupied beyond the declared probe horizon".into(),
        ],
        resource_policy: ResourcePolicy::Diagnostic,
        ledger_limits: LedgerLimits {
            max_events: 32,
            max_payload_bytes: 1024,
            max_assumptions: 16,
            max_identifier_bytes: 8192,
            max_initial_resources: 8,
            max_initial_claims: 8,
            max_assumption_bytes: 4096,
        },
        probes: vec![spec],
        pool_limits: PoolLimits {
            max_probes: 8,
            max_snapshot_bytes: 64 * 1024,
            max_probe_image_bytes: IMAGE_CAP,
            max_checkpoint_bytes: 16 * 1024 * 1024,
        },
    }
}
fn finish(r: &mut Runner, count: usize) {
    while r.advance_source(0).unwrap() {}
    let frontier = r.frontier();
    for _ in 0..128 {
        if r.results().len() == count {
            break;
        }
        r.drive_probes(1).unwrap();
    }
    assert_eq!(r.results().len(), count);
    assert_eq!(r.frontier(), frontier);
    let terminal = r.results();
    r.drive_probes(20).unwrap();
    assert_eq!(r.results(), terminal);
}
fn completed(r: &ProbeResult) -> u128 {
    match r.outcome {
        ProbeOutcome::Completed { predicted } => predicted,
        ref x => panic!("expected completion, got {x:?}"),
    }
}
fn wire_limits() -> WireLimits {
    WireLimits {
        max_wire_bytes: 32 * 1024 * 1024,
        max_probes: 8,
        max_id_bytes: 1024,
        max_image_bytes: IMAGE_CAP,
        max_reason_bytes: 4096,
    }
}
fn encode(r: &Runner) -> Vec<u8> {
    let cp = r.checkpoint().unwrap();
    shadow_wire::encode(
        &RunImage {
            ledger_frontier: cp.ledger_frontier as u64,
            runner: cp.pool,
        },
        wire_limits(),
    )
    .unwrap()
}
fn results_json(r: &Runner) -> Value {
    json!({"frontier":r.frontier(),"results":r.results().iter().map(|p|json!({
    "id":p.id,"anchor":p.anchor_event,"observed":p.observed.map(|v|v.to_string()),
    "predicted":completed(p).to_string(),"events":p.events,"last_tick":p.last_tick.to_string(),
    "frontier":p.frontier,"snapshot_digest":p.snapshot_digest.iter().map(|v|format!("{v:02x}")).collect::<String>()})).collect::<Vec<_>>()})
}

#[test]
fn real_early_exact_late_and_missing_observation_do_not_change_prediction() {
    let c = config();
    let mut prior_snapshot = None;
    for observed in [Some(8), Some(10), Some(12), None] {
        let mut r = Runner::new(definition(&c, observed), adapter(c.clone(), false)).unwrap();
        assert!(r.advance_source(0).unwrap());
        let inventory = r.trusted_inventory();
        if let Some(prior) = &prior_snapshot {
            assert_eq!(prior, &inventory[0].1);
        } else {
            prior_snapshot = Some(inventory[0].1.clone());
        }
        finish(&mut r, 1);
        let result = &r.results()[0];
        assert_eq!(completed(result), 10);
        assert_eq!(result.observed, observed);
        let eval = r.evaluate(EvaluationPolicy::Diagnostic).unwrap();
        assert_eq!(eval.counts.late, u64::from(observed == Some(8)));
        assert_eq!(
            eval.records[0].residual.map(|x| x.magnitude),
            observed.map(|x| 10u128.abs_diff(x))
        );
        if observed.is_none() {
            assert!(
                !r.evaluate(EvaluationPolicy::Strict {
                    max_late_numerator: 1,
                    max_late_denominator: 1
                })
                .unwrap()
                .accepted
            );
        }
    }
}
#[test]
fn slow_walk_retains_positive_residual_and_exact_macro_anchor() {
    let mut predictions = Vec::new();
    for walk in [3, 6] {
        let c = SyntheticFlowConfig {
            route_length_mm: walk * 1000,
            ..config()
        };
        let mut r = Runner::new(definition(&c, Some(10)), adapter(c, false)).unwrap();
        r.advance_source(0).unwrap();
        let initial = r.trusted_inventory()[0].1.clone();
        assert_eq!(initial.at, 0);
        finish(&mut r, 1);
        assert_eq!(r.frontier(), 2);
        assert_eq!(r.trusted_inventory()[0].1, initial);
        let evaluated = r.evaluate(EvaluationPolicy::Diagnostic).unwrap();
        predictions.push((
            completed(&r.results()[0]),
            evaluated.counts.late,
            evaluated.records[0].residual.unwrap().magnitude,
        ));
    }
    assert_eq!(predictions, vec![(10, 0, 0), (13, 1, 3)]);
}
#[test]
fn two_native_probes_share_history_but_not_resource_queues() {
    let c = config();
    let mut d = definition(&c, Some(1));
    let mut second = d.probes[0].clone();
    second.id = "p1".into();
    second.key.occurrence = 1;
    d.probes.push(second);
    let mut r = Runner::new(d, adapter(c, false)).unwrap();
    r.advance_source(0).unwrap();
    let inventory = r.trusted_inventory();
    assert_eq!(inventory[0].1, inventory[1].1);
    finish(&mut r, 2);
    let out = r.results();
    assert!(out.iter().all(|p| completed(p) == 10));
    assert_eq!(out[0].events, out[1].events);
    assert_eq!(r.frontier(), 2);
    assert!(r.trusted_inventory() == inventory);
}
#[test]
fn advancing_one_native_probe_leaves_the_other_image_unchanged() {
    let c = config();
    let mut d = definition(&c, Some(1));
    let mut other = d.probes[0].clone();
    other.id = "p1".into();
    other.key.occurrence = 1;
    d.probes.push(other);
    let (binding, inventory) = shadow_runner::trusted_inventory(&mut d, 1).unwrap();
    let mut pool = crate::shadow_pool::ProbePool::new(adapter(c, false), binding, d.pool_limits);
    for (spec, snapshot) in inventory {
        pool.admit(spec, snapshot).unwrap();
    }
    let before = pool
        .checkpoint()
        .unwrap()
        .probes
        .into_iter()
        .find(|p| p.spec.id == "p1")
        .unwrap();
    assert!(pool.advance("p0", 64).unwrap().is_some());
    let after = pool
        .checkpoint()
        .unwrap()
        .probes
        .into_iter()
        .find(|p| p.spec.id == "p1")
        .unwrap();
    assert!(before == after, "another probe's native image was mutated");
    assert_eq!(pool.results().len(), 1);
    pool.advance("p1", 64).unwrap();
    assert!(pool.results().iter().all(|p| completed(p) == 10));
}
#[test]
fn infeasible_snapshot_never_starts_native_world_and_limits_censor() {
    let c = config();
    let mut d = definition(&c, Some(1));
    d.initial_resources
        .get_mut(&c.target_resource)
        .unwrap()
        .capacity = 0;
    // Zero capacity is invalid source configuration, not an accepted infeasible probe.
    assert!(Runner::new(d, adapter(c.clone(), true)).is_err());
    let mut d = definition(&c, Some(1));
    let resource = d.initial_resources.get_mut(&c.target_resource).unwrap();
    resource.capacity = 1;
    d.events[0].transition = Transition::Acquire {
        resource: c.target_resource.clone(),
        claim: "second-observed-holder".into(),
        units: 1,
    };
    let mut r = Runner::new(d, adapter(c.clone(), true)).unwrap();
    finish(&mut r, 1);
    assert!(matches!(
        r.results()[0].outcome,
        ProbeOutcome::Infeasible { .. }
    ));
    assert_eq!(r.results()[0].events, 0);
    for budget in [
        ProbeBudget {
            horizon: 2,
            max_events: 64,
        },
        ProbeBudget {
            horizon: 100,
            max_events: 1,
        },
    ] {
        let mut d = definition(&c, Some(1));
        d.probes[0].budget = budget;
        let mut r = Runner::new(d, adapter(c.clone(), false)).unwrap();
        finish(&mut r, 1);
        assert!(matches!(
            r.results()[0].outcome,
            ProbeOutcome::Censored { .. }
        ));
        assert!(
            !r.evaluate(EvaluationPolicy::Strict {
                max_late_numerator: 1,
                max_late_denominator: 1
            })
            .unwrap()
            .accepted
        );
    }
}
#[test]
fn native_walk_plus_work_ridge_needs_independent_walk_observation() {
    let mut observations = Vec::new();
    let mut observed_walk = None;
    for walk in 1..=4u64 {
        let c = SyntheticFlowConfig {
            route_length_mm: walk * 1000,
            work_duration_ticks: u128::from(5 - walk),
            ..config()
        };
        let mut d = definition(&c, Some(5));
        d.events.push(event("independent-walk-observation", 2, 2));
        let input = d.probes[0].input.clone();
        let mut ledger = ObservedLedger::new(
            d.events,
            d.initial_resources,
            d.assumptions,
            d.resource_policy,
            d.ledger_limits,
        )
        .unwrap();
        let snapshot = ledger.advance().unwrap().unwrap().clone();
        let a = NativeFlowAdapter {
            model: SyntheticFlowProbeModel { config: c },
            max_image_bytes: IMAGE_CAP,
        };
        let mut world = a.start(&snapshot, &input).unwrap();
        let mut target = None;
        for _ in 0..64 {
            let receipt = a.step(&mut world).unwrap();
            assert!(receipt.failure.is_none());
            if receipt.target.is_some() {
                target = receipt.target;
                break;
            }
        }
        assert_eq!(target, Some(5));
        assert_eq!(a.model.arrival_tick(&world), Some(u128::from(walk)));
        observations.push((walk, 5 - walk, a.model.arrival_tick(&world).unwrap()));
        while let Some(source) = ledger.advance().unwrap() {
            if source.anchor_event == "independent-walk-observation" {
                observed_walk = Some(source.at);
            }
        }
    }
    // Independent source walk endpoint at tick 2 identifies one of the four
    // tied synthetic totals. A total objective tie alone identifies none.
    assert_eq!(observed_walk, Some(2));
    let compatible: Vec<_> = observations
        .iter()
        .filter(|x| Some(x.2) == observed_walk)
        .collect();
    assert_eq!(observations.len(), 4);
    assert_eq!(compatible, vec![&(2, 3, 2)]);
}

fn envelope_binding(binding: [u8; 32]) -> CheckpointBindingV1 {
    CheckpointBindingV1 {
        model_code: [0xC3; 32],
        configuration: binding,
        owner_schemas: [1; 32],
    }
}
fn envelope_limits() -> CheckpointEnvelopeLimits {
    CheckpointEnvelopeLimits {
        max_file_bytes: 33 * 1024 * 1024,
        max_body_bytes: 32 * 1024 * 1024,
    }
}
fn child(mode: &str, cut: &str, root: &Path) {
    let c = config();
    let mut d = definition(&c, Some(1));
    let binding = shadow_runner::trusted_inventory(&mut d, 0).unwrap().0;
    let path = root.join(format!("{cut}.checkpoint"));
    if mode == "baseline" {
        let mut r = Runner::new(d, adapter(c, false)).unwrap();
        finish(&mut r, 1);
        std::fs::write(
            root.join("baseline.json"),
            serde_json::to_vec(&results_json(&r)).unwrap(),
        )
        .unwrap();
        return;
    }
    if mode == "save" {
        let mut r = Runner::new(d, adapter(c.clone(), false)).unwrap();
        while r.advance_source(0).unwrap() {}
        let dispatches = if cut == "transit" { 1 } else { 3 };
        r.drive_probes(dispatches).unwrap();
        let cp = r.checkpoint().unwrap();
        assert!(matches!(
            cp.pool.probes[0].state,
            SavedProbeState::Pending(_)
        ));
        assert_eq!(cp.pool.probes[0].events, dispatches);
        if cut == "work" {
            assert_eq!(cp.pool.probes[0].last_tick, 3);
            let saved = &cp.pool.probes[0];
            let SavedProbeState::Pending(image) = &saved.state else {
                unreachable!()
            };
            let model = SyntheticFlowProbeModel { config: c.clone() };
            let world = model
                .restore(&saved.snapshot, &saved.spec.input, image)
                .unwrap();
            assert_eq!(
                model.arrival_tick(&world),
                Some(3),
                "work cut must be after native arrival"
            );
            assert!(!model.completed(&world).unwrap());
        }
        checkpoint_envelope::write_file_no_clobber(
            &path,
            &encode(&r),
            envelope_binding(binding),
            envelope_limits(),
        )
        .unwrap();
        return;
    }
    assert_eq!(mode, "restore");
    let bytes = checkpoint_envelope::read_file(&path, envelope_binding(binding), envelope_limits())
        .unwrap();
    let frontier =
        shadow_wire::frontier_hint(&bytes, binding, d.events.len() as u64, wire_limits()).unwrap();
    let (_, inventory) = shadow_runner::trusted_inventory(&mut d, frontier as usize).unwrap();
    let image = shadow_wire::decode(&bytes, binding, frontier, &inventory, wire_limits()).unwrap();
    let mut r = Runner::restore(
        d,
        adapter(c, true),
        ShadowRunnerCheckpoint {
            ledger_frontier: frontier as usize,
            pool: image.runner,
        },
    )
    .unwrap();
    assert_eq!(
        encode(&r),
        bytes,
        "restoration must not dispatch or resample"
    );
    finish(&mut r, 1);
    let baseline: Value =
        serde_json::from_slice(&std::fs::read(root.join("baseline.json")).unwrap()).unwrap();
    assert_eq!(results_json(&r), baseline);
    std::fs::write(
        root.join(format!("{cut}-restored.json")),
        serde_json::to_vec(&results_json(&r)).unwrap(),
    )
    .unwrap();
}
#[test]
#[ignore = "invoked only by the bounded fresh-process parent fixture"]
fn fresh_process_child() {
    child(
        &std::env::var("C3_CHILD_MODE").unwrap(),
        &std::env::var("C3_CHILD_CUT").unwrap(),
        Path::new(&std::env::var("C3_CHILD_DIR").unwrap()),
    );
}
#[test]
fn fresh_process_pending_and_overdue_native_images_match_complete_run() {
    let base = std::env::var_os("C3_ACCEPTANCE_OUTDIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let root = base.join(format!("c03-process-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    for (mode, cut) in [
        ("baseline", "baseline"),
        ("save", "transit"),
        ("restore", "transit"),
        ("save", "work"),
        ("restore", "work"),
    ] {
        let output = Command::new(std::env::current_exe().unwrap())
            .args([
                "--ignored",
                "--exact",
                "shadow_c03::fresh_process_child",
                "--nocapture",
            ])
            .env("C3_CHILD_MODE", mode)
            .env("C3_CHILD_CUT", cut)
            .env("C3_CHILD_DIR", &root)
            .output()
            .unwrap();
        std::fs::write(
            root.join(format!("{mode}-{cut}.log")),
            [output.stdout.clone(), output.stderr.clone()].concat(),
        )
        .unwrap();
        assert!(
            output.status.success(),
            "{mode}/{cut}: {} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[cfg(any(feature = "ipc", feature = "parquet"))]
#[test]
fn native_late_outcomes_join_unchanged_c4_physical_schema() {
    let c = config();
    let mut r = Runner::new(definition(&c, Some(1)), adapter(c, false)).unwrap();
    finish(&mut r, 1);
    let specs: Vec<_> = r.trusted_inventory().into_iter().map(|x| x.0).collect();
    let key = &specs[0].key;
    let binding = crate::sidecar_adapter::RunBinding {
        study_id: key.study_id.clone(),
        dataset_id: key.dataset_id.clone(),
        scenario_id: key.scenario_id.clone(),
        candidate_id: specs[0].candidate_id.clone(),
        run_id: specs[0].run_id.clone(),
        replication_id: key.replication_id.clone(),
        seed_schedule_id: key.seed_schedule_id.clone(),
        seed_map_ref: key.seed_map_ref.clone(),
        seed_map_version: 1,
        seed_contract_version: "kairoecs.seed-purpose.v1".into(),
        mapping_version: key.mapping_version.clone(),
        fidelity: "ShadowAnchored".into(),
        parameter_hash: specs[0]
            .input
            .parameter_hash
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect(),
    };
    let out = crate::shadow_sidecars::build(
        &r.results(),
        &specs,
        binding,
        crate::residuals::Window {
            start: 0,
            end_exclusive: 100,
        },
        None,
    )
    .unwrap();
    assert_eq!(out.residuals[0]["predicted_ticks"], "10");
    assert_eq!(out.residuals[0]["residual_magnitude"], "9");
    let batch = crate::arrow_output::encode("calibration_residual.v1", &out.residuals).unwrap();
    assert_eq!(
        crate::arrow_output::decode("calibration_residual.v1", &batch).unwrap(),
        out.residuals
    );
}
