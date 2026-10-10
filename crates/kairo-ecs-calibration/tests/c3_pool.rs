#![allow(dead_code)]
#[path = "../src/residuals.rs"]
mod residuals;
#[path = "../src/shadow.rs"]
mod shadow;
#[path = "../src/shadow_pool.rs"]
mod shadow_pool;
#[path = "../src/trace_order.rs"]
mod trace_order;

use residuals::LogicalKey;
use shadow::{
    LedgerSnapshot, ObservedEvent, ProbeAdapter, ProbeBudget, ProbeInput, ProbeOutcome, ProbeSpec,
    ProbeStep, ResourceState, ShadowError, Transition,
};
use shadow_pool::{PoolLimits, ProbePool};
use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

// The opaque seed key is only an identity token in this protocol harness.
// Seed derivation and registry conformance remain owned by seed_map tests.
mod seed_map {
    #[derive(Clone, Eq, PartialEq)]
    pub(crate) struct CalibrationStreamKey(pub(crate) u8);
    impl CalibrationStreamKey {
        pub(crate) fn checkpoint_identifier_bytes(&self) -> Result<usize, ()> {
            Ok(1)
        }
    }
}

#[derive(Clone)]
struct Adapter {
    restore_calls: Arc<AtomicUsize>,
    start_target: bool,
}
#[derive(Clone, Debug)]
struct Runtime {
    ticks: Vec<u128>,
    targets: Vec<bool>,
    index: usize,
    now: u128,
}
impl ProbeAdapter for Adapter {
    type Runtime = Runtime;
    fn start(&self, snapshot: &LedgerSnapshot, _: &ProbeInput) -> Result<Runtime, ShadowError> {
        Ok(Runtime {
            ticks: vec![snapshot.at + 1, snapshot.at + 2, snapshot.at + 3],
            targets: vec![false, true, true],
            index: 0,
            now: snapshot.at,
        })
    }
    fn now(&self, runtime: &Runtime) -> u128 {
        runtime.now
    }
    fn next_tick(&self, runtime: &Runtime) -> Result<Option<u128>, ShadowError> {
        Ok(runtime.ticks.get(runtime.index).copied())
    }
    fn step(&self, runtime: &mut Runtime) -> Result<ProbeStep, ShadowError> {
        let tick = *runtime
            .ticks
            .get(runtime.index)
            .ok_or(ShadowError::Contract("empty fake"))?;
        let target = runtime.targets[runtime.index].then_some(tick);
        runtime.index += 1;
        runtime.now = tick;
        Ok(ProbeStep {
            dispatched_at: tick,
            target,
            dispatches: 1,
            failure: None,
        })
    }
    fn target_at_start(&self, runtime: &Runtime) -> Result<Option<u128>, ShadowError> {
        Ok(self.start_target.then_some(runtime.now))
    }
    fn checkpoint(&self, runtime: &Runtime, _: usize) -> Result<Vec<u8>, ShadowError> {
        Ok(vec![runtime.index as u8])
    }
    fn restore(
        &self,
        snapshot: &LedgerSnapshot,
        _: &ProbeInput,
        bytes: &[u8],
    ) -> Result<Runtime, ShadowError> {
        self.restore_calls.fetch_add(1, Ordering::SeqCst);
        let index = *bytes.first().ok_or(ShadowError::IncompatibleCheckpoint)? as usize;
        let mut runtime = Runtime {
            ticks: vec![snapshot.at + 1, snapshot.at + 2, snapshot.at + 3],
            targets: vec![false, true, true],
            index,
            now: snapshot.at,
        };
        if index > 0 {
            runtime.now = runtime.ticks[index - 1];
        }
        Ok(runtime)
    }
}

struct BadReceipt(Adapter);
impl ProbeAdapter for BadReceipt {
    type Runtime = Runtime;
    fn start(&self, snapshot: &LedgerSnapshot, input: &ProbeInput) -> Result<Runtime, ShadowError> {
        self.0.start(snapshot, input)
    }
    fn now(&self, runtime: &Runtime) -> u128 {
        self.0.now(runtime)
    }
    fn next_tick(&self, runtime: &Runtime) -> Result<Option<u128>, ShadowError> {
        self.0.next_tick(runtime)
    }
    fn step(&self, runtime: &mut Runtime) -> Result<ProbeStep, ShadowError> {
        let mut step = self.0.step(runtime)?;
        step.dispatched_at += 1;
        Ok(step)
    }
    fn target_at_start(&self, runtime: &Runtime) -> Result<Option<u128>, ShadowError> {
        self.0.target_at_start(runtime)
    }
    fn checkpoint(&self, runtime: &Runtime, max_bytes: usize) -> Result<Vec<u8>, ShadowError> {
        self.0.checkpoint(runtime, max_bytes)
    }
    fn restore(
        &self,
        snapshot: &LedgerSnapshot,
        input: &ProbeInput,
        bytes: &[u8],
    ) -> Result<Runtime, ShadowError> {
        self.0.restore(snapshot, input, bytes)
    }
}

struct ExtraDispatch(Adapter);
impl ProbeAdapter for ExtraDispatch {
    type Runtime = Runtime;
    fn start(&self, snapshot: &LedgerSnapshot, input: &ProbeInput) -> Result<Runtime, ShadowError> {
        self.0.start(snapshot, input)
    }
    fn now(&self, runtime: &Runtime) -> u128 {
        self.0.now(runtime)
    }
    fn next_tick(&self, runtime: &Runtime) -> Result<Option<u128>, ShadowError> {
        self.0.next_tick(runtime)
    }
    fn step(&self, runtime: &mut Runtime) -> Result<ProbeStep, ShadowError> {
        let first = self.0.step(runtime)?;
        let second = self.0.step(runtime)?;
        Ok(ProbeStep {
            dispatched_at: second.dispatched_at,
            target: None,
            dispatches: first.dispatches + second.dispatches,
            failure: Some(ShadowError::Contract("extra fake dispatch")),
        })
    }
    fn target_at_start(&self, runtime: &Runtime) -> Result<Option<u128>, ShadowError> {
        self.0.target_at_start(runtime)
    }
    fn checkpoint(&self, runtime: &Runtime, max_bytes: usize) -> Result<Vec<u8>, ShadowError> {
        self.0.checkpoint(runtime, max_bytes)
    }
    fn restore(
        &self,
        snapshot: &LedgerSnapshot,
        input: &ProbeInput,
        bytes: &[u8],
    ) -> Result<Runtime, ShadowError> {
        self.0.restore(snapshot, input, bytes)
    }
}

fn limits() -> PoolLimits {
    PoolLimits {
        max_probes: 8,
        max_snapshot_bytes: 4096,
        max_probe_image_bytes: 32,
        max_checkpoint_bytes: 16_384,
    }
}
fn snapshot(at: u128) -> LedgerSnapshot {
    LedgerSnapshot {
        frontier: 4,
        at,
        anchor_event: "anchor".into(),
        digest: [7; 32],
        visible_events: vec![ObservedEvent {
            order: trace_order::TraceOrderKeyV1 {
                relative_ticks: at,
                case_key: "case".into(),
                occurrence: 0,
                event_kind_rank: trace_order::EventKindRank::from_canonical_decimal("1").unwrap(),
                source_event_key: "anchor".into(),
                source_order: 0,
            },
            available_at: Some(at),
            source_defined: true,
            transition: Transition::None,
            payload: vec![],
        }],
        resources: BTreeMap::<String, ResourceState>::new(),
        resource_feasible: true,
        assumptions: vec!["synthetic".into()],
    }
}
fn spec(id: &str, observed: Option<u128>, budget: ProbeBudget) -> ProbeSpec {
    ProbeSpec {
        id: id.into(),
        key: LogicalKey {
            study_id: "study".into(),
            dataset_id: "data".into(),
            scenario_id: "scenario".into(),
            seed_schedule_id: "schedule".into(),
            replication_id: "1".into(),
            case_key: id.into(),
            task_key: "task".into(),
            occurrence: 0,
            endpoint: "done".into(),
            seed_purpose: "calibration".into(),
            seed_map_ref: "seed-v1".into(),
            mapping_version: "map-v1".into(),
        },
        run_id: "run".into(),
        candidate_id: "candidate".into(),
        anchor_event: "anchor".into(),
        target_event: Some("target-event".into()),
        observed_target: observed,
        input: ProbeInput {
            target: "done".into(),
            seed_key: seed_map::CalibrationStreamKey(1),
            parameter_hash: [1; 32],
            adapter_hash: [2; 32],
            fidelity: "Micro".into(),
        },
        budget,
    }
}
fn pool(adapter: Adapter) -> ProbePool<Adapter> {
    ProbePool::new(adapter, [9; 32], limits())
}

#[test]
fn per_call_and_lifetime_budgets_are_exact_and_target_is_first_dispatch() {
    let adapter = Adapter {
        restore_calls: Arc::new(AtomicUsize::new(0)),
        start_target: false,
    };
    let mut pool = pool(adapter);
    pool.admit(
        spec(
            "a",
            Some(11),
            ProbeBudget {
                horizon: 12,
                max_events: 3,
            },
        ),
        snapshot(10),
    )
    .unwrap();
    assert_eq!(pool.advance("a", 1).unwrap(), None);
    assert_eq!(pool.advance("a", 1).unwrap().unwrap().events, 2);
    let result = pool.advance("a", 1).unwrap().unwrap();
    assert_eq!(result.events, 2);
    assert_eq!(result.outcome, ProbeOutcome::Completed { predicted: 12 });
    assert_eq!(result.observed, Some(11));
    assert_eq!(pool.advance("a", u64::MAX).unwrap(), Some(result.clone()));
    assert_eq!(pool.results(), vec![result]);
}

#[test]
fn exact_lifetime_event_boundary_censors_before_the_next_dispatch() {
    let mut p = pool(Adapter {
        restore_calls: Arc::new(AtomicUsize::new(0)),
        start_target: false,
    });
    p.admit(
        spec(
            "event-boundary",
            None,
            ProbeBudget {
                horizon: 20,
                max_events: 1,
            },
        ),
        snapshot(10),
    )
    .unwrap();
    assert_eq!(p.advance("event-boundary", 1).unwrap(), None);
    let result = p.advance("event-boundary", 1).unwrap().unwrap();
    assert_eq!(result.events, 1);
    assert_eq!(result.last_tick, 11);
    assert_eq!(
        result.outcome,
        ProbeOutcome::Censored {
            reason: shadow::LimitReason::EventBudget,
        }
    );
}

#[test]
fn admission_target_costs_zero_and_horizon_includes_final_tick() {
    let mut p = pool(Adapter {
        restore_calls: Arc::new(AtomicUsize::new(0)),
        start_target: true,
    });
    p.admit(
        spec(
            "now",
            Some(10),
            ProbeBudget {
                horizon: 10,
                max_events: 0,
            },
        ),
        snapshot(10),
    )
    .unwrap();
    let result = p.advance("now", 0).unwrap().unwrap();
    assert_eq!(result.events, 0);
    assert_eq!(result.outcome, ProbeOutcome::Completed { predicted: 10 });

    let adapter = Adapter {
        restore_calls: Arc::new(AtomicUsize::new(0)),
        start_target: false,
    };
    let mut p = pool(adapter);
    p.admit(
        spec(
            "horizon",
            None,
            ProbeBudget {
                horizon: 11,
                max_events: 5,
            },
        ),
        snapshot(10),
    )
    .unwrap();
    let result = p.advance("horizon", 5).unwrap().unwrap();
    assert_eq!(
        result.outcome,
        ProbeOutcome::Censored {
            reason: shadow::LimitReason::TickHorizon
        }
    );
    assert_eq!(result.events, 1);
}

#[test]
fn checkpoint_restore_is_staged_and_matches_complete_trusted_inventory() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut p = pool(Adapter {
        restore_calls: calls.clone(),
        start_target: false,
    });
    let trusted = spec(
        "pending",
        Some(12),
        ProbeBudget {
            horizon: 20,
            max_events: 5,
        },
    );
    let snap = snapshot(10);
    p.admit(trusted.clone(), snap.clone()).unwrap();
    p.advance("pending", 1).unwrap();
    let checkpoint = p.checkpoint().unwrap();

    let mut changed = trusted.clone();
    changed.budget.max_events += 1;
    let before = calls.load(Ordering::SeqCst);
    assert_eq!(
        ProbePool::restore(
            Adapter {
                restore_calls: calls.clone(),
                start_target: false
            },
            [9; 32],
            limits(),
            vec![(changed, snap.clone())],
            checkpoint.clone()
        )
        .err(),
        Some(ShadowError::IncompatibleCheckpoint)
    );
    assert_eq!(
        calls.load(Ordering::SeqCst),
        before,
        "no adapter decode before inventory validation"
    );

    let mut restored = ProbePool::restore(
        Adapter {
            restore_calls: calls.clone(),
            start_target: false,
        },
        [9; 32],
        limits(),
        vec![(trusted, snap)],
        checkpoint,
    )
    .unwrap();
    assert_eq!(
        restored.advance("pending", 3).unwrap().unwrap().outcome,
        ProbeOutcome::Completed { predicted: 12 }
    );
}

#[test]
fn duplicate_ids_are_rejected() {
    let mut p = pool(Adapter {
        restore_calls: Arc::new(AtomicUsize::new(0)),
        start_target: false,
    });
    let s = spec(
        "dup",
        None,
        ProbeBudget {
            horizon: 20,
            max_events: 5,
        },
    );
    p.admit(s.clone(), snapshot(10)).unwrap();
    assert_eq!(
        p.admit(s, snapshot(10)),
        Err(ShadowError::DuplicateIdentity("dup".into()))
    );
}

#[test]
fn infeasible_snapshot_is_terminal_without_starting_a_runtime() {
    let adapter = Adapter {
        restore_calls: Arc::new(AtomicUsize::new(0)),
        start_target: false,
    };
    let mut p = pool(adapter);
    let mut snap = snapshot(10);
    snap.resource_feasible = false;
    p.admit(
        spec(
            "infeasible",
            Some(12),
            ProbeBudget {
                horizon: 20,
                max_events: 5,
            },
        ),
        snap,
    )
    .unwrap();
    let result = p.advance("infeasible", 10).unwrap().unwrap();
    assert!(matches!(result.outcome, ProbeOutcome::Infeasible { .. }));
    assert_eq!(result.events, 0);
}

#[test]
fn contract_violation_poisoning_prevents_additional_dispatch() {
    let adapter = BadReceipt(Adapter {
        restore_calls: Arc::new(AtomicUsize::new(0)),
        start_target: false,
    });
    let mut pool = ProbePool::new(adapter, [9; 32], limits());
    pool.admit(
        spec(
            "bad",
            None,
            ProbeBudget {
                horizon: 20,
                max_events: 5,
            },
        ),
        snapshot(10),
    )
    .unwrap();
    assert_eq!(
        pool.advance("bad", 1),
        Err(ShadowError::Contract(
            "dispatch receipt or runtime tick mismatch"
        ))
    );
    let terminal = pool.advance("bad", 100).unwrap().unwrap();
    assert_eq!(terminal.events, 1);
    assert!(matches!(terminal.outcome, ProbeOutcome::Failed { .. }));
}

#[test]
fn consumed_failure_and_hidden_dispatch_preserve_truthful_terminal_accounting() {
    let base = Adapter {
        restore_calls: Arc::new(AtomicUsize::new(0)),
        start_target: false,
    };
    let mut p = ProbePool::new(ExtraDispatch(base.clone()), [9; 32], limits());
    let trusted = spec(
        "overshoot",
        None,
        ProbeBudget {
            horizon: 11,
            max_events: 1,
        },
    );
    let snap = snapshot(10);
    p.admit(trusted.clone(), snap.clone()).unwrap();
    let terminal = p.advance("overshoot", 1).unwrap().unwrap();
    assert_eq!(terminal.events, 2);
    assert_eq!(terminal.last_tick, 12);
    assert!(matches!(terminal.outcome, ProbeOutcome::Failed { .. }));

    let checkpoint = p.checkpoint().unwrap();
    let restores = Arc::new(AtomicUsize::new(0));
    let mut restored = ProbePool::restore(
        ExtraDispatch(Adapter {
            restore_calls: restores.clone(),
            start_target: false,
        }),
        [9; 32],
        limits(),
        vec![(trusted, snap)],
        checkpoint,
    )
    .unwrap();
    assert_eq!(restored.advance("overshoot", 100).unwrap(), Some(terminal));
    assert_eq!(restores.load(Ordering::SeqCst), 0);
}

#[test]
fn snapshot_visibility_is_canonical_historical_and_excludes_selected_future_key() {
    let base = Adapter {
        restore_calls: Arc::new(AtomicUsize::new(0)),
        start_target: false,
    };
    let candidate = spec(
        "visible-target",
        None,
        ProbeBudget {
            horizon: 20,
            max_events: 3,
        },
    );
    let mut with_target = snapshot(10);
    let mut old_target = with_target.visible_events[0].clone();
    old_target.order.relative_ticks = 9;
    old_target.order.source_event_key = "target-event".into();
    old_target.available_at = Some(9);
    old_target.order.source_order = 0;
    with_target.visible_events.insert(0, old_target);
    with_target.frontier = 4;
    assert!(matches!(
        pool(base.clone()).admit(candidate.clone(), with_target),
        Err(ShadowError::InvalidInput(_))
    ));

    let mut future_known = snapshot(10);
    future_known.visible_events[0].available_at = Some(11);
    assert!(matches!(
        pool(base.clone()).admit(candidate.clone(), future_known),
        Err(ShadowError::InvalidInput(_))
    ));

    let mut future_occurrence = snapshot(10);
    future_occurrence.visible_events[0].order.relative_ticks = 11;
    assert!(matches!(
        pool(base.clone()).admit(candidate.clone(), future_occurrence),
        Err(ShadowError::InvalidInput(_))
    ));

    let mut duplicate_key = snapshot(10);
    let mut duplicate = duplicate_key.visible_events[0].clone();
    duplicate.order.relative_ticks = 9;
    duplicate.order.source_order = 0;
    duplicate_key.visible_events.insert(0, duplicate);
    duplicate_key.frontier = 4;
    assert!(matches!(
        pool(base.clone()).admit(candidate.clone(), duplicate_key),
        Err(ShadowError::InvalidInput(_))
    ));

    let mut target_is_anchor = candidate;
    target_is_anchor.target_event = Some("anchor".into());
    let mut unmatched_anchor = pool(Adapter {
        restore_calls: Arc::new(AtomicUsize::new(0)),
        start_target: false,
    });
    assert_eq!(
        unmatched_anchor.admit(target_is_anchor.clone(), snapshot(10)),
        Err(ShadowError::Contract(
            "selected target is the anchor but adapter reports incomplete"
        ))
    );
    let mut anchored_target = pool(Adapter {
        restore_calls: Arc::new(AtomicUsize::new(0)),
        start_target: true,
    });
    anchored_target
        .admit(target_is_anchor, snapshot(10))
        .unwrap();
    let result = anchored_target
        .advance("visible-target", 0)
        .unwrap()
        .unwrap();
    assert_eq!(result.events, 0);
    assert_eq!(result.outcome, ProbeOutcome::Completed { predicted: 10 });
}

#[test]
fn invalid_anchor_and_inconsistent_checkpoint_states_are_rejected() {
    let adapter = Adapter {
        restore_calls: Arc::new(AtomicUsize::new(0)),
        start_target: false,
    };
    let mut invalid_pool = pool(adapter.clone());
    let mut invalid = snapshot(10);
    invalid.visible_events[0].source_defined = false;
    assert!(matches!(
        invalid_pool.admit(
            spec(
                "invalid",
                None,
                ProbeBudget {
                    horizon: 20,
                    max_events: 5
                }
            ),
            invalid
        ),
        Err(ShadowError::InvalidInput(_))
    ));

    let mut infeasible = snapshot(10);
    infeasible.resource_feasible = false;
    let candidate = spec(
        "infeasible",
        None,
        ProbeBudget {
            horizon: 20,
            max_events: 5,
        },
    );
    let mut terminal_pool = pool(adapter.clone());
    terminal_pool
        .admit(candidate.clone(), infeasible.clone())
        .unwrap();
    let mut checkpoint = terminal_pool.checkpoint().unwrap();
    checkpoint.probes[0].state = shadow::SavedProbeState::Pending(vec![0]);
    let calls = adapter.restore_calls.load(Ordering::SeqCst);
    assert_eq!(
        ProbePool::restore(
            adapter.clone(),
            [9; 32],
            limits(),
            vec![(candidate.clone(), infeasible.clone())],
            checkpoint
        )
        .err(),
        Some(ShadowError::IncompatibleCheckpoint)
    );
    assert_eq!(calls, 0);

    let mut completed_checkpoint = terminal_pool.checkpoint().unwrap();
    completed_checkpoint.probes[0].state =
        shadow::SavedProbeState::Terminal(ProbeOutcome::Completed { predicted: 10 });
    assert_eq!(
        ProbePool::restore(
            adapter.clone(),
            [9; 32],
            limits(),
            vec![(candidate, infeasible)],
            completed_checkpoint,
        )
        .err(),
        Some(ShadowError::IncompatibleCheckpoint)
    );
    assert_eq!(adapter.restore_calls.load(Ordering::SeqCst), 0);
}

#[test]
fn restore_rejects_zero_cost_future_completion_and_blank_failure_reason() {
    let adapter = Adapter {
        restore_calls: Arc::new(AtomicUsize::new(0)),
        start_target: false,
    };
    let candidate = spec(
        "terminal-shape",
        None,
        ProbeBudget {
            horizon: 20,
            max_events: 3,
        },
    );
    let snap = snapshot(10);
    let mut source = pool(adapter.clone());
    source.admit(candidate.clone(), snap.clone()).unwrap();

    let mut future_completion = source.checkpoint().unwrap();
    future_completion.probes[0].state =
        shadow::SavedProbeState::Terminal(ProbeOutcome::Completed { predicted: 11 });
    assert_eq!(
        ProbePool::restore(
            adapter.clone(),
            [9; 32],
            limits(),
            vec![(candidate.clone(), snap.clone())],
            future_completion,
        )
        .err(),
        Some(ShadowError::IncompatibleCheckpoint)
    );

    let mut blank_failure = source.checkpoint().unwrap();
    blank_failure.probes[0].state = shadow::SavedProbeState::Terminal(ProbeOutcome::Failed {
        reason: "  ".into(),
    });
    assert_eq!(
        ProbePool::restore(
            adapter.clone(),
            [9; 32],
            limits(),
            vec![(candidate, snap)],
            blank_failure,
        )
        .err(),
        Some(ShadowError::IncompatibleCheckpoint)
    );
    assert_eq!(adapter.restore_calls.load(Ordering::SeqCst), 0);
}
