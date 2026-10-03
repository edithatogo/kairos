#![cfg(feature = "pdes")]

use std::collections::BTreeMap;

use kairo_ecs_pdes::{
    ConservativeProcess, ConservativeRuntime, LpId, PartitionPlan, RemoteEvent, RuntimeError, Tick,
};
use kairo_ecs_types::{EntityId, SimDuration, SimTime};

struct Recorder {
    id: LpId,
    seen: Vec<(Tick, Vec<u8>)>,
    mode: Mode,
}

impl Default for Recorder {
    fn default() -> Self {
        Self {
            id: LpId(0),
            seen: Vec::new(),
            mode: Mode::default(),
        }
    }
}

#[derive(Clone, Copy, Default)]
enum Mode {
    #[default]
    Record,
    Ring {
        lp_count: u32,
        until: u128,
    },
    AdversarialRing {
        lp_count: u32,
        until: u128,
    },
    InvalidLookahead,
    MixedInvalid,
    WrongSource,
    LocalChain,
    ZeroDelayLoop,
    Panic,
    OverflowEmit,
}

impl ConservativeProcess for Recorder {
    fn on_event(&mut self, event: &RemoteEvent) -> Vec<RemoteEvent> {
        self.seen.push((event.tick, event.event_payload.clone()));
        match self.mode {
            Mode::Record => Vec::new(),
            Mode::OverflowEmit => vec![message(
                self.id,
                LpId(1),
                SimTime::from_ticks(u128::MAX),
                Vec::new(),
            )],
            Mode::Ring { lp_count, until } => {
                if event.tick.ticks() >= until {
                    return Vec::new();
                }
                let next = (self.id.0 + 1) % lp_count;
                let hash = event
                    .event_payload
                    .first()
                    .copied()
                    .unwrap_or(17)
                    .wrapping_mul(37)
                    .wrapping_add(self.id.0 as u8);
                let delay = u128::from((hash % 5) + 1);
                vec![message(
                    self.id,
                    LpId(next),
                    SimTime::from_ticks(event.tick.ticks() + delay),
                    vec![hash],
                )]
            }
            Mode::AdversarialRing { lp_count, until } => {
                if event.tick.ticks() >= until {
                    Vec::new()
                } else {
                    vec![message(
                        self.id,
                        LpId((self.id.0 + 1) % lp_count),
                        SimTime::from_ticks(event.tick.ticks() + 1),
                        Vec::new(),
                    )]
                }
            }
            Mode::InvalidLookahead => vec![message(
                self.id,
                LpId((self.id.0 + 1) % 2),
                event.tick,
                b"too-early".to_vec(),
            )],
            Mode::MixedInvalid if self.id == LpId(0) => vec![message(
                self.id,
                LpId(1),
                SimTime::from_ticks(event.tick.ticks() + 2),
                b"valid-but-staged".to_vec(),
            )],
            Mode::MixedInvalid => vec![message(self.id, LpId(0), event.tick, b"invalid".to_vec())],
            Mode::WrongSource => vec![message(
                LpId(self.id.0 + 1),
                LpId((self.id.0 + 1) % 2),
                SimTime::from_ticks(event.tick.ticks() + 1),
                Vec::new(),
            )],
            Mode::LocalChain => {
                if event.event_payload == b"start" {
                    vec![message(
                        self.id,
                        self.id,
                        SimTime::from_ticks(3),
                        b"followup".to_vec(),
                    )]
                } else {
                    Vec::new()
                }
            }
            Mode::ZeroDelayLoop => vec![message(self.id, self.id, event.tick, Vec::new())],
            Mode::Panic => panic!("test handler panic"),
        }
    }
}

fn message(source_lp: LpId, dest_lp: LpId, tick: Tick, event_payload: Vec<u8>) -> RemoteEvent {
    RemoteEvent {
        source_lp,
        dest_lp,
        tick,
        event_payload,
    }
}

fn plan(lp_count: u32, lookahead: u128) -> PartitionPlan {
    let entities = (0..lp_count)
        .map(|index| EntityId::new(u64::from(index), 0))
        .collect();
    PartitionPlan::from_entities(lp_count, SimDuration::from_ticks(lookahead), entities).unwrap()
}

fn processes(lp_count: u32, mode: impl Fn(LpId) -> Mode) -> BTreeMap<LpId, Recorder> {
    (0..lp_count)
        .map(|index| {
            let id = LpId(index);
            (
                id,
                Recorder {
                    id,
                    mode: mode(id),
                    ..Recorder::default()
                },
            )
        })
        .collect()
}

fn ring_topology(lp_count: u32) -> BTreeMap<LpId, Vec<LpId>> {
    (0..lp_count)
        .map(|index| (LpId(index), vec![LpId((index + 1) % lp_count)]))
        .collect()
}

#[test]
fn null_bounds_are_exclusive_and_event_at_the_bound_waits() {
    let mut runtime =
        ConservativeRuntime::new(plan(2, 3), ring_topology(2), processes(2, |_| Mode::Record))
            .unwrap();
    runtime
        .schedule_initial(message(
            LpId(0),
            LpId(1),
            SimTime::from_ticks(3),
            b"boundary".to_vec(),
        ))
        .unwrap();

    assert_eq!(
        runtime.run_until_with_budget(SimTime::from_ticks(3), 0),
        Err(RuntimeError::EventBudgetExceeded { budget: 0 })
    );
    assert!(runtime.report().null_messages > 0);
    assert!(runtime.processes()[&LpId(1)].seen.is_empty());
    runtime.run_until(SimTime::from_ticks(3)).unwrap();
    assert_eq!(runtime.processes()[&LpId(1)].seen.len(), 1);
}

#[test]
fn same_lp_followups_are_ordered_by_timestamp_before_existing_events() {
    let mut runtime = ConservativeRuntime::new(
        plan(1, 2),
        BTreeMap::from([(LpId(0), Vec::new())]),
        processes(1, |_| Mode::LocalChain),
    )
    .unwrap();
    runtime
        .schedule_initial(message(
            LpId(0),
            LpId(0),
            SimTime::from_ticks(1),
            b"start".to_vec(),
        ))
        .unwrap();
    runtime
        .schedule_initial(message(
            LpId(0),
            LpId(0),
            SimTime::from_ticks(7),
            b"later".to_vec(),
        ))
        .unwrap();

    runtime.run_until(SimTime::from_ticks(7)).unwrap();
    let seen = &runtime.processes()[&LpId(0)].seen;
    assert_eq!(
        seen.iter()
            .map(|(tick, _)| tick.ticks())
            .collect::<Vec<_>>(),
        [1, 3, 7]
    );
}

#[test]
fn invalid_outbound_batch_is_rejected_and_runtime_is_poisoned() {
    let mut runtime = ConservativeRuntime::new(
        plan(2, 2),
        ring_topology(2),
        processes(2, |_| Mode::InvalidLookahead),
    )
    .unwrap();
    runtime
        .schedule_initial(message(
            LpId(0),
            LpId(1),
            SimTime::from_ticks(0),
            Vec::new(),
        ))
        .unwrap();

    let error = runtime.run_until(SimTime::from_ticks(0)).unwrap_err();
    assert!(matches!(
        error,
        RuntimeError::OutboundLookaheadViolation { .. }
    ));
    assert_eq!(runtime.report().remote_events, 0);
    assert_eq!(runtime.report().emitted_events, 0);
    assert_eq!(
        runtime.run_until(SimTime::from_ticks(1)),
        Err(RuntimeError::Poisoned)
    );
}

#[test]
fn mixed_valid_and_invalid_outputs_commit_none_of_the_round() {
    let mut runtime = ConservativeRuntime::new(
        plan(2, 2),
        ring_topology(2),
        processes(2, |_| Mode::MixedInvalid),
    )
    .unwrap();
    for lp_id in [LpId(0), LpId(1)] {
        runtime
            .schedule_initial(message(
                lp_id,
                LpId((lp_id.0 + 1) % 2),
                SimTime::ZERO,
                Vec::new(),
            ))
            .unwrap();
    }
    assert!(matches!(
        runtime.run_until(SimTime::ZERO),
        Err(RuntimeError::OutboundLookaheadViolation { .. })
    ));
    assert_eq!(runtime.pending_events(), 0);
    assert_eq!(runtime.report().remote_events, 0);
    assert_eq!(runtime.report().emitted_events, 0);
    assert_eq!(
        runtime
            .processes()
            .values()
            .map(|process| process.seen.len())
            .sum::<usize>(),
        2
    );
}

#[test]
fn handler_panic_fails_closed_and_poisons_runtime() {
    let mut runtime = ConservativeRuntime::new(
        plan(1, 1),
        BTreeMap::from([(LpId(0), Vec::new())]),
        processes(1, |_| Mode::Panic),
    )
    .unwrap();
    runtime
        .schedule_initial(message(LpId(0), LpId(0), SimTime::ZERO, Vec::new()))
        .unwrap();
    assert_eq!(
        runtime.run_until(SimTime::ZERO),
        Err(RuntimeError::HandlerPanicked(LpId(0)))
    );
    assert_eq!(
        runtime.run_until(SimTime::ZERO),
        Err(RuntimeError::Poisoned)
    );
}

#[test]
fn outbound_metadata_and_overflow_fail_with_typed_errors() {
    let mut runtime = ConservativeRuntime::new(
        plan(2, 1),
        ring_topology(2),
        processes(2, |_| Mode::WrongSource),
    )
    .unwrap();
    runtime
        .schedule_initial(message(LpId(0), LpId(1), SimTime::ZERO, Vec::new()))
        .unwrap();
    assert!(matches!(
        runtime.run_until(SimTime::ZERO),
        Err(RuntimeError::OutboundSourceMismatch { .. })
    ));

    let mut runtime = ConservativeRuntime::new(
        plan(1, 1),
        BTreeMap::from([(LpId(0), Vec::new())]),
        processes(1, |_| Mode::Record),
    )
    .unwrap();
    assert_eq!(
        runtime.run_until(SimTime::from_ticks(u128::MAX)),
        Err(RuntimeError::HorizonOverflow(SimTime::from_ticks(
            u128::MAX
        )))
    );
}

#[test]
fn empty_cyclic_topology_advances_without_per_tick_rounds() {
    let mut runtime =
        ConservativeRuntime::new(plan(8, 7), ring_topology(8), processes(8, |_| Mode::Record))
            .unwrap();
    let report = runtime.run_until(SimTime::from_ticks(10_000)).unwrap();
    assert_eq!(report.processed_events, 0);
    assert!(report.null_messages > 0);
    assert_eq!(runtime.gvt(), SimTime::from_ticks(10_000));
    assert!(report.rounds < 10_000);
}

#[test]
fn randomized_positive_lookahead_ring_progresses_through_ten_thousand_ticks() {
    let mut runtime = ConservativeRuntime::new(
        plan(8, 1),
        ring_topology(8),
        processes(8, |_| Mode::Ring {
            lp_count: 8,
            until: 10_000,
        }),
    )
    .unwrap();
    runtime
        .schedule_initial(message(LpId(0), LpId(0), SimTime::ZERO, vec![23]))
        .unwrap();
    let report = runtime.run_until(SimTime::from_ticks(10_000)).unwrap();
    assert!(report.processed_events > 1_000);
    assert!(report.remote_events > 1_000);
    assert!(report.null_messages > 0);
    assert!(report.gvt_history.windows(2).all(|pair| pair[0] <= pair[1]));
    assert_eq!(
        report.gvt_history.last(),
        Some(&SimTime::from_ticks(10_000))
    );
}

#[test]
fn bounded_same_tick_chain_can_resume_without_corrupting_runtime() {
    let mut runtime = ConservativeRuntime::new(
        plan(1, 1),
        BTreeMap::from([(LpId(0), Vec::new())]),
        processes(1, |_| Mode::ZeroDelayLoop),
    )
    .unwrap();
    runtime
        .schedule_initial(message(LpId(0), LpId(0), SimTime::ZERO, Vec::new()))
        .unwrap();
    assert_eq!(
        runtime.run_until_with_budget(SimTime::ZERO, 5),
        Err(RuntimeError::EventBudgetExceeded { budget: 5 })
    );
    assert_eq!(runtime.report().processed_events, 5);
    assert_eq!(runtime.processes()[&LpId(0)].seen.len(), 5);
}

#[test]
fn sparse_future_input_resumes_after_partial_horizon() {
    let mut runtime =
        ConservativeRuntime::new(plan(2, 2), ring_topology(2), processes(2, |_| Mode::Record))
            .unwrap();
    runtime
        .schedule_initial(message(
            LpId(0),
            LpId(1),
            SimTime::from_ticks(50),
            b"future".to_vec(),
        ))
        .unwrap();
    runtime.run_until(SimTime::from_ticks(10)).unwrap();
    assert!(runtime.processes()[&LpId(1)].seen.is_empty());
    assert_eq!(
        runtime.schedule_initial(message(LpId(0), LpId(1), SimTime::ZERO, Vec::new())),
        Err(RuntimeError::InitialEventsAlreadyClosed)
    );
    assert!(matches!(
        runtime.run_until(SimTime::from_ticks(9)),
        Err(RuntimeError::HorizonRegression { .. })
    ));
    runtime.run_until(SimTime::from_ticks(50)).unwrap();
    assert_eq!(runtime.processes()[&LpId(1)].seen.len(), 1);
}

#[test]
fn eligible_process_batches_run_concurrently_across_logical_processes() {
    let mut runtime =
        ConservativeRuntime::new(plan(8, 2), ring_topology(8), processes(8, |_| Mode::Record))
            .unwrap();
    for index in 0..8 {
        let lp_id = LpId(index);
        runtime
            .schedule_initial(message(
                lp_id,
                LpId((index + 1) % 8),
                SimTime::ZERO,
                vec![index as u8],
            ))
            .unwrap();
    }
    let report = runtime.run_until(SimTime::ZERO).unwrap();
    assert_eq!(report.processed_events, 8);
    assert_eq!(report.worker_count, 8);
}

#[test]
fn adversarial_minimum_lookahead_cycle_executes_every_tick_on_eight_lps() {
    let mut runtime = ConservativeRuntime::new(
        plan(8, 1),
        ring_topology(8),
        processes(8, |_| Mode::AdversarialRing {
            lp_count: 8,
            until: 10_000,
        }),
    )
    .unwrap();
    runtime
        .schedule_initial(message(LpId(0), LpId(0), SimTime::ZERO, Vec::new()))
        .unwrap();
    let report = runtime.run_until(SimTime::from_ticks(10_000)).unwrap();
    assert_eq!(report.processed_events, 10_001);
    assert_eq!(report.remote_events, 10_000);
    assert_eq!(runtime.pending_events(), 0);
    assert_eq!(runtime.gvt(), SimTime::from_ticks(10_000));
    assert!(runtime
        .processes()
        .values()
        .all(|lp| lp.seen.len() >= 1_250));
    assert!(report.gvt_history.windows(2).all(|pair| pair[0] <= pair[1]));
}

#[test]
fn partition_process_sets_and_directed_routes_are_validated_before_execution() {
    let error =
        ConservativeRuntime::new(plan(2, 1), ring_topology(2), processes(1, |_| Mode::Record))
            .err()
            .unwrap();
    assert!(matches!(error, RuntimeError::ProcessSetMismatch { .. }));
    let invalid_topology = BTreeMap::from([(LpId(0), vec![LpId(9)]), (LpId(1), Vec::new())]);
    assert!(matches!(
        ConservativeRuntime::new(plan(2, 1), invalid_topology, processes(2, |_| Mode::Record))
            .err()
            .unwrap(),
        RuntimeError::UnknownTopologyDestination { .. }
    ));
    let topology = BTreeMap::from([(LpId(0), vec![LpId(1)]), (LpId(1), Vec::new())]);
    let mut runtime =
        ConservativeRuntime::new(plan(2, 1), topology, processes(2, |_| Mode::Record)).unwrap();
    assert!(matches!(
        runtime.schedule_initial(message(LpId(1), LpId(0), SimTime::ZERO, Vec::new())),
        Err(RuntimeError::InitialRouteMissing { .. })
    ));
    assert_eq!(runtime.pending_events(), 0);
}

#[test]
fn emitting_event_lookahead_overflow_is_typed_and_poisoning() {
    let topology = BTreeMap::from([(LpId(0), vec![LpId(1)]), (LpId(1), Vec::new())]);
    let mut runtime = ConservativeRuntime::new(
        plan(2, 2),
        topology,
        processes(2, |id| {
            if id == LpId(0) {
                Mode::OverflowEmit
            } else {
                Mode::Record
            }
        }),
    )
    .unwrap();
    let at = SimTime::from_ticks(u128::MAX - 1);
    runtime
        .schedule_initial(message(LpId(0), LpId(0), at, Vec::new()))
        .unwrap();
    assert_eq!(
        runtime.run_until(at),
        Err(RuntimeError::LookaheadOverflow {
            lp_id: LpId(0),
            event_tick: at,
            lookahead: SimDuration::from_ticks(2)
        })
    );
    assert_eq!(runtime.run_until(at), Err(RuntimeError::Poisoned));
}
