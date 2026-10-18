#![allow(dead_code)]
#[path = "../src/residuals.rs"]
mod residuals;
#[path = "../src/shadow.rs"]
mod shadow;
#[path = "../src/shadow_ledger.rs"]
mod shadow_ledger;
#[path = "../src/shadow_pool.rs"]
mod shadow_pool;
#[path = "../src/shadow_report.rs"]
mod shadow_report;
#[path = "../src/shadow_runner.rs"]
mod shadow_runner;
#[path = "../src/trace_order.rs"]
mod trace_order;

mod seed_map {
    #[derive(Clone, Eq, PartialEq)]
    pub(crate) struct CalibrationStreamKey(pub u8);
    impl CalibrationStreamKey {
        pub(crate) fn checkpoint_identifier_bytes(&self) -> Result<usize, ()> {
            Ok(1)
        }
    }
}
use shadow::*;
use shadow_ledger::*;
use shadow_pool::*;
use shadow_runner::*;
use std::collections::BTreeMap;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use trace_order::*;

#[derive(Clone, Default)]
struct Adapter;
#[derive(Clone)]
struct Runtime {
    now: u128,
    next: u128,
}
impl ProbeAdapter for Adapter {
    type Runtime = Runtime;
    fn start(&self, s: &LedgerSnapshot, _: &ProbeInput) -> Result<Runtime, ShadowError> {
        Ok(Runtime {
            now: s.at,
            next: s.at + 2,
        })
    }
    fn now(&self, r: &Runtime) -> u128 {
        r.now
    }
    fn next_tick(&self, r: &Runtime) -> Result<Option<u128>, ShadowError> {
        Ok(Some(r.next))
    }
    fn step(&self, r: &mut Runtime) -> Result<ProbeStep, ShadowError> {
        r.now = r.next;
        Ok(ProbeStep {
            dispatches: 1,
            failure: None,
            dispatched_at: r.now,
            target: Some(r.now),
        })
    }
    fn target_at_start(&self, _: &Runtime) -> Result<Option<u128>, ShadowError> {
        Ok(None)
    }
    fn checkpoint(&self, r: &Runtime, _: usize) -> Result<Vec<u8>, ShadowError> {
        Ok((r.now as u64).to_le_bytes().to_vec())
    }
    fn restore(
        &self,
        _s: &LedgerSnapshot,
        _: &ProbeInput,
        b: &[u8],
    ) -> Result<Runtime, ShadowError> {
        let n = u64::from_le_bytes(
            b.try_into()
                .map_err(|_| ShadowError::IncompatibleCheckpoint)?,
        ) as u128;
        Ok(Runtime {
            now: n,
            next: n + 2,
        })
    }
}
fn limits() -> (LedgerLimits, PoolLimits) {
    (
        LedgerLimits {
            max_events: 32,
            max_payload_bytes: 4096,
            max_assumptions: 32,
            max_identifier_bytes: 4096,
            max_initial_resources: 8,
            max_initial_claims: 16,
            max_assumption_bytes: 4096,
        },
        PoolLimits {
            max_probes: 8,
            max_snapshot_bytes: 8192,
            max_probe_image_bytes: 64,
            max_checkpoint_bytes: 32768,
        },
    )
}
fn event(t: u128, key: &str) -> ObservedEvent {
    ObservedEvent {
        order: TraceOrderKeyV1 {
            relative_ticks: t,
            case_key: "case".into(),
            occurrence: 0,
            event_kind_rank: EventKindRank::from_canonical_decimal("1").unwrap(),
            source_event_key: key.into(),
            source_order: t as u64,
        },
        available_at: Some(t),
        source_defined: true,
        transition: Transition::None,
        payload: vec![],
    }
}
fn spec() -> ProbeSpec {
    ProbeSpec {
        id: "probe".into(),
        key: residuals::LogicalKey {
            study_id: "s".into(),
            dataset_id: "d".into(),
            scenario_id: "x".into(),
            seed_schedule_id: "ss".into(),
            replication_id: "0".into(),
            case_key: "case".into(),
            task_key: "task".into(),
            occurrence: 0,
            endpoint: "done".into(),
            seed_purpose: "calibration".into(),
            seed_map_ref: "map".into(),
            mapping_version: "v1".into(),
        },
        run_id: "run".into(),
        candidate_id: "candidate".into(),
        anchor_event: "anchor".into(),
        target_event: Some("target".into()),
        observed_target: Some(999),
        input: ProbeInput {
            target: "done".into(),
            seed_key: seed_map::CalibrationStreamKey(1),
            parameter_hash: [1; 32],
            adapter_hash: [2; 32],
            fidelity: "Micro".into(),
        },
        budget: ProbeBudget {
            horizon: 20,
            max_events: 10,
        },
    }
}
fn definition() -> TrustedRunDefinition {
    let (l, p) = limits();
    TrustedRunDefinition {
        identity: b"run-config-schema-v1".to_vec(),
        events: vec![event(2, "anchor"), event(4, "target")],
        initial_resources: BTreeMap::new(),
        assumptions: vec![],
        resource_policy: ResourcePolicy::Diagnostic,
        ledger_limits: l,
        probes: vec![spec()],
        pool_limits: p,
    }
}
#[test]
fn source_advance_admits_derived_exact_target_and_keeps_pool_time_separate() {
    let mut r = ShadowRunner::new(definition(), Adapter).unwrap();
    assert!(r.advance_source(0).unwrap());
    assert_eq!(r.frontier(), 1);
    assert!(r.results().is_empty());
    r.drive_probes(1).unwrap();
    let out = r.results();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].observed, Some(4));
    assert_eq!(out[0].outcome, ProbeOutcome::Completed { predicted: 4 });
    assert_eq!(r.frontier(), 1);
    assert!(r.advance_source(1).unwrap());
    assert_eq!(r.frontier(), 2);
    assert!(!r.advance_source(1).unwrap());
}
#[test]
fn checkpoint_restore_rebuilds_source_prefix_and_does_not_replay_probe_prefix() {
    let mut r = ShadowRunner::new(definition(), Adapter).unwrap();
    r.advance_source(0).unwrap();
    let cp = r.checkpoint().unwrap();
    let restored = ShadowRunner::restore(definition(), Adapter, cp).unwrap();
    assert_eq!(restored.frontier(), 1);
    assert!(restored.results().is_empty());
}
#[test]
fn mismatched_source_inventory_rejected() {
    let mut r = ShadowRunner::new(definition(), Adapter).unwrap();
    r.advance_source(0).unwrap();
    let cp = r.checkpoint().unwrap();
    let mut changed = definition();
    changed.events[1].order.relative_ticks = 5;
    assert!(matches!(
        ShadowRunner::restore(changed, Adapter, cp),
        Err(ShadowError::IncompatibleCheckpoint)
    ));
}

#[test]
fn late_target_retains_observation_and_prediction_after_target_tick() {
    let mut d = definition();
    d.events[1].order.relative_ticks = 3;
    let mut r = ShadowRunner::new(d, Adapter).unwrap();
    r.advance_source(1).unwrap();
    let result = &r.results()[0];
    assert_eq!(result.observed, Some(3));
    assert_eq!(result.outcome, ProbeOutcome::Completed { predicted: 4 });
}

#[test]
fn unavailable_anchor_is_audited_and_strict_evaluation_rejects_subset() {
    let mut d = definition();
    d.events[0].available_at = None;
    let mut r = ShadowRunner::new(d, Adapter).unwrap();
    r.advance_source(1).unwrap();
    assert_eq!(
        r.skipped_probes().get("probe").map(String::as_str),
        Some("unavailable anchor")
    );
    assert!(
        !r.evaluate(EvaluationPolicy::Strict {
            max_late_numerator: 0,
            max_late_denominator: 1
        })
        .unwrap()
        .accepted
    );
}

#[test]
fn post_advance_admission_failure_poison_prevents_retry_and_checkpoint() {
    let mut d = definition();
    let mut second = spec();
    second.id = "z-probe".into();
    second.input.target = "wrong".into();
    d.probes.push(second);
    let mut r = ShadowRunner::new(d, Adapter).unwrap();
    assert!(r.advance_source(0).is_err());
    assert!(r.advance_source(0).is_err());
    assert!(r.checkpoint().is_err());
}

#[test]
fn hidden_historical_infeasibility_does_not_block_visible_probe_but_rejects_strict_candidate() {
    let mut d = definition();
    d.events = vec![
        event(0, "a1"),
        event(1, "a2"),
        event(2, "anchor"),
        event(4, "target"),
    ];
    d.events[0].available_at = Some(4);
    d.events[1].available_at = Some(4);
    d.events[0].transition = Transition::Acquire {
        resource: "beds".into(),
        claim: "one".into(),
        units: 1,
    };
    d.events[1].transition = Transition::Acquire {
        resource: "beds".into(),
        claim: "two".into(),
        units: 1,
    };
    d.initial_resources.insert(
        "beds".into(),
        InitialResource {
            capacity: 1,
            claims: vec![],
        },
    );
    d.probes[0].anchor_event = "anchor".into();
    d.resource_policy = ResourcePolicy::Diagnostic;
    let mut r = ShadowRunner::new(d, Adapter).unwrap();
    r.advance_source(1).unwrap();
    r.advance_source(1).unwrap();
    r.advance_source(1).unwrap();
    assert!(!r.historical_feasible());
    assert_eq!(r.results().len(), 1);
    assert_eq!(
        r.results()[0].outcome,
        ProbeOutcome::Completed { predicted: 4 }
    );
    assert!(
        !r.evaluate(EvaluationPolicy::Strict {
            max_late_numerator: 0,
            max_late_denominator: 1
        })
        .unwrap()
        .accepted
    );
}

#[test]
fn canonical_source_permutation_keeps_binding_and_inventory_stable() {
    let mut a = definition();
    let mut b = definition();
    b.events.reverse();
    b.probes.reverse();
    let (binding_a, inventory_a) = trusted_inventory(&mut a, 1).unwrap();
    let (binding_b, inventory_b) = trusted_inventory(&mut b, 1).unwrap();
    assert_eq!(binding_a, binding_b);
    assert!(inventory_a == inventory_b);
}

#[test]
fn strict_evaluation_rejects_unadvanced_requested_inventory() {
    let r = ShadowRunner::new(definition(), Adapter).unwrap();
    let evaluation = r
        .evaluate(EvaluationPolicy::Strict {
            max_late_numerator: 0,
            max_late_denominator: 1,
        })
        .unwrap();
    assert_eq!(evaluation.counts.total, 1);
    assert_eq!(evaluation.counts.missing_prediction, 1);
    assert!(!evaluation.accepted);
}

#[test]
fn strict_ledger_error_poison_preserves_failure_and_blocks_checkpoint_retry() {
    let mut d = definition();
    d.initial_resources.insert(
        "beds".into(),
        InitialResource {
            capacity: 1,
            claims: vec![],
        },
    );
    d.events[0].transition = Transition::Acquire {
        resource: "beds".into(),
        claim: "over".into(),
        units: 2,
    };
    d.resource_policy = ResourcePolicy::Strict;
    let mut r = ShadowRunner::new(d, Adapter).unwrap();
    assert!(matches!(
        r.advance_source(1),
        Err(ShadowError::ResourceInfeasible(_))
    ));
    assert!(r.checkpoint().is_err());
    assert!(r.advance_source(1).is_err());
    assert!(
        !r.evaluate(EvaluationPolicy::Strict {
            max_late_numerator: 0,
            max_late_denominator: 1
        })
        .unwrap()
        .accepted
    );
}

#[test]
fn trusted_inventory_preflights_aggregate_metadata_at_exact_cap() {
    let mut d = definition();
    let mut second = spec();
    second.id = "probe-2".into();
    second.key.case_key = "case-2".into();
    d.probes.push(second);
    d.pool_limits.max_checkpoint_bytes = 128 * 1024;
    let mut low = 0usize;
    let mut high = d.pool_limits.max_checkpoint_bytes;
    while low < high {
        let middle = low + (high - low) / 2;
        d.pool_limits.max_checkpoint_bytes = middle;
        if trusted_inventory(&mut d, 1).is_ok() {
            high = middle;
        } else {
            low = middle + 1;
        }
    }
    d.pool_limits.max_checkpoint_bytes = low;
    assert!(trusted_inventory(&mut d, 1).is_ok());
    assert!(low > 0);
    d.pool_limits.max_checkpoint_bytes = low - 1;
    assert!(matches!(
        trusted_inventory(&mut d, 1),
        Err(ShadowError::LimitExceeded)
    ));
}

#[derive(Clone)]
struct RestoreCountingAdapter(Arc<AtomicUsize>);
impl ProbeAdapter for RestoreCountingAdapter {
    type Runtime = Runtime;
    fn start(&self, s: &LedgerSnapshot, i: &ProbeInput) -> Result<Runtime, ShadowError> {
        Adapter.start(s, i)
    }
    fn now(&self, r: &Runtime) -> u128 {
        Adapter.now(r)
    }
    fn next_tick(&self, r: &Runtime) -> Result<Option<u128>, ShadowError> {
        Adapter.next_tick(r)
    }
    fn step(&self, r: &mut Runtime) -> Result<ProbeStep, ShadowError> {
        Adapter.step(r)
    }
    fn target_at_start(&self, r: &Runtime) -> Result<Option<u128>, ShadowError> {
        Adapter.target_at_start(r)
    }
    fn checkpoint(&self, r: &Runtime, cap: usize) -> Result<Vec<u8>, ShadowError> {
        Adapter.checkpoint(r, cap)
    }
    fn restore(
        &self,
        s: &LedgerSnapshot,
        i: &ProbeInput,
        b: &[u8],
    ) -> Result<Runtime, ShadowError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Adapter.restore(s, i, b)
    }
}

#[test]
fn pool_restore_rejects_aggregate_metadata_before_native_restore() {
    let d = definition();
    let mut runner = ShadowRunner::new(d, Adapter).unwrap();
    runner.advance_source(0).unwrap();
    let checkpoint = runner.checkpoint().unwrap();
    let mut def = definition();
    let (_, trusted) = trusted_inventory(&mut def, 1).unwrap();
    let (_, mut limits) = limits();
    limits.max_checkpoint_bytes = 1;
    let calls = Arc::new(AtomicUsize::new(0));
    let result = ProbePool::restore(
        RestoreCountingAdapter(calls.clone()),
        checkpoint.pool.binding,
        limits,
        trusted,
        checkpoint.pool,
    );
    assert!(matches!(result, Err(ShadowError::LimitExceeded)));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[test]
fn oversized_saved_metadata_is_rejected_before_native_restore() {
    let mut runner = ShadowRunner::new(definition(), Adapter).unwrap();
    runner.advance_source(0).unwrap();
    let mut checkpoint = runner.checkpoint().unwrap();
    checkpoint.pool.probes[0].spec.id = "x".repeat(64 * 1024);
    let mut def = definition();
    let (_, trusted) = trusted_inventory(&mut def, 1).unwrap();
    let (_, limits) = limits();
    let calls = Arc::new(AtomicUsize::new(0));
    let result = ProbePool::restore(
        RestoreCountingAdapter(calls.clone()),
        checkpoint.pool.binding,
        limits,
        trusted,
        checkpoint.pool,
    );
    assert!(matches!(result, Err(ShadowError::LimitExceeded)));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[test]
fn admission_accepts_exact_metadata_cap_and_rejects_cap_minus_one() {
    let mut def = definition();
    let (_, trusted) = trusted_inventory(&mut def, 1).unwrap();
    let metadata = metadata_base_bytes().unwrap()
        + metadata_entry_bytes(&trusted[0].0, &trusted[0].1).unwrap();
    let (_, mut pool_limits) = limits();
    pool_limits.max_checkpoint_bytes = metadata;
    let (binding, _) = trusted_inventory(&mut definition(), 1).unwrap();
    let mut exact = ProbePool::new(Adapter, binding, pool_limits);
    exact
        .admit(trusted[0].0.clone(), trusted[0].1.clone())
        .unwrap();

    pool_limits.max_checkpoint_bytes = metadata - 1;
    let mut short = ProbePool::new(Adapter, binding, pool_limits);
    assert!(matches!(
        short.admit(trusted[0].0.clone(), trusted[0].1.clone()),
        Err(ShadowError::LimitExceeded)
    ));
}

#[test]
fn checkpoint_restore_accept_exact_cap_and_reject_cap_minus_one_pre_native_restore() {
    let mut def = definition();
    let (binding, trusted) = trusted_inventory(&mut def, 1).unwrap();
    let metadata = metadata_base_bytes().unwrap()
        + metadata_entry_bytes(&trusted[0].0, &trusted[0].1).unwrap();
    let exact_cap = metadata + 8; // Adapter checkpoint image is exactly one u64.
    let (_, mut pool_limits) = limits();
    pool_limits.max_checkpoint_bytes = exact_cap;
    let mut pool = ProbePool::new(Adapter, binding, pool_limits);
    pool.admit(trusted[0].0.clone(), trusted[0].1.clone())
        .unwrap();
    let checkpoint = pool.checkpoint().unwrap();

    let exact_calls = Arc::new(AtomicUsize::new(0));
    ProbePool::restore(
        RestoreCountingAdapter(exact_calls.clone()),
        binding,
        pool_limits,
        trusted.clone(),
        checkpoint.clone(),
    )
    .unwrap();
    assert_eq!(exact_calls.load(Ordering::SeqCst), 1);

    pool_limits.max_checkpoint_bytes = exact_cap - 1;
    let rejected_calls = Arc::new(AtomicUsize::new(0));
    assert!(matches!(
        ProbePool::restore(
            RestoreCountingAdapter(rejected_calls.clone()),
            binding,
            pool_limits,
            trusted,
            checkpoint,
        ),
        Err(ShadowError::LimitExceeded)
    ));
    assert_eq!(rejected_calls.load(Ordering::SeqCst), 0);
}
