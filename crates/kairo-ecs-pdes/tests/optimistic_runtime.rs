#![cfg(feature = "time-warp")]

use std::collections::BTreeMap;

use kairo_ecs_pdes::{
    LogicalEventId, LpId, OptimisticError, OptimisticLimits, OptimisticMessage,
    OptimisticMessageKind, OptimisticProcess, OptimisticRuntime, OptimisticStateError,
    PartitionPlan, RemoteEvent, Tick,
};
use kairo_ecs_types::{EntityId, SimDuration};

#[derive(Clone, Debug, Eq, PartialEq)]
struct State {
    total: u64,
    rng: u64,
    seen: Vec<(u128, Vec<u8>)>,
}

struct Model {
    lp: LpId,
    state: State,
    restore_fails: bool,
    panic_on_event: bool,
}

impl OptimisticProcess for Model {
    type Snapshot = State;

    fn snapshot(&self) -> Self::Snapshot {
        self.state.clone()
    }

    fn restore(&mut self, state: &Self::Snapshot) -> Result<(), OptimisticStateError> {
        if self.restore_fails {
            return Err(OptimisticStateError::new("injected restore failure"));
        }
        self.state = state.clone();
        Ok(())
    }

    fn on_event(&mut self, event: &RemoteEvent) -> Vec<RemoteEvent> {
        assert!(!self.panic_on_event, "injected handler panic");
        self.state.total = self.state.total.wrapping_add(u64::from(
            event.event_payload.first().copied().unwrap_or_default(),
        ));
        self.state.rng = self
            .state
            .rng
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1);
        self.state
            .seen
            .push((event.tick.ticks(), event.event_payload.clone()));
        if event.event_payload.len() == 4 && event.event_payload[0] == b'o' {
            let destination = LpId(u32::from(event.event_payload[1]));
            let tick = Tick::from_ticks(u128::from(event.event_payload[2]));
            vec![RemoteEvent {
                source_lp: self.lp,
                dest_lp: destination,
                tick,
                event_payload: vec![event.event_payload[3]],
            }]
        } else {
            Vec::new()
        }
    }
}

fn new_runtime(limits: OptimisticLimits) -> OptimisticRuntime<Model> {
    let lps = [LpId(0), LpId(1)];
    let partition = PartitionPlan::from_entities(
        2,
        SimDuration::from_ticks(1),
        vec![EntityId::new(1, 0), EntityId::new(2, 0)],
    )
    .unwrap();
    let topology = BTreeMap::from([(lps[0], vec![lps[1]]), (lps[1], vec![lps[0]])]);
    let processes = lps
        .into_iter()
        .map(|lp| {
            (
                lp,
                Model {
                    lp,
                    state: State {
                        total: 0,
                        rng: 7,
                        seen: Vec::new(),
                    },
                    restore_fails: false,
                    panic_on_event: false,
                },
            )
        })
        .collect();
    OptimisticRuntime::new(partition, topology, processes, limits).unwrap()
}

fn event(source: u32, destination: u32, tick: u64, payload: &[u8]) -> RemoteEvent {
    RemoteEvent {
        source_lp: LpId(source),
        dest_lp: LpId(destination),
        tick: Tick::from_ticks(u128::from(tick)),
        event_payload: payload.to_vec(),
    }
}

fn root_sender(event: RemoteEvent, sequence: u64) -> OptimisticMessage {
    let mut sender = new_runtime(OptimisticLimits::default());
    sender.schedule_initial(sequence, event).unwrap()
}

#[test]
fn root_identity_is_explicit_and_initial_scheduling_closes_on_execution() {
    let mut runtime = new_runtime(OptimisticLimits::default());
    let initial_token = runtime.state_token(LpId(0)).unwrap();
    let paused = runtime
        .run_until_with_budget(Tick::from_ticks(4), 0)
        .unwrap();
    assert_eq!(paused.budget_used, 0);
    assert!(runtime.validate_state_token(initial_token));
    runtime.schedule_initial(6, event(0, 0, 3, &[1])).unwrap();
    let first = runtime.schedule_initial(7, event(0, 0, 4, &[1])).unwrap();
    assert_eq!(first.logical_id().root_parts(), Some((LpId(0), 7)));
    assert_eq!(first.event().event_payload, vec![1]);
    let progress = runtime
        .run_until_with_budget(Tick::from_ticks(4), 1)
        .unwrap();
    assert_eq!(progress.budget_used, 1);
    assert_eq!(
        runtime.schedule_initial(8, event(0, 0, 5, &[1])),
        Err(OptimisticError::InitialSchedulingClosed)
    );
}

#[test]
fn structural_output_order_uses_full_parent_key_after_straggler_replay() {
    let mut runtime = new_runtime(OptimisticLimits::default());
    let late = root_sender(event(0, 0, 20, &[b'o', 1, 30, 2]), 20);
    runtime.receive(late).unwrap();
    let first = runtime
        .run_until_with_budget(Tick::from_ticks(30), 1)
        .unwrap();
    let first_output = first
        .published_messages
        .iter()
        .find(|m| m.kind() == OptimisticMessageKind::Positive)
        .unwrap();
    assert_eq!(first_output.event().event_payload, vec![2]);

    let early = root_sender(event(0, 0, 10, &[b'o', 1, 30, 1]), 10);
    runtime.receive(early).unwrap();
    let replayed = runtime
        .run_until_with_budget(Tick::from_ticks(30), 20)
        .unwrap();
    let seen = &runtime.process_at(LpId(1)).unwrap().state.seen;
    assert_eq!(
        seen.iter()
            .map(|(_, payload)| payload[0])
            .collect::<Vec<_>>(),
        vec![1, 2]
    );
    assert!(replayed
        .published_messages
        .iter()
        .any(|m| m.kind() == OptimisticMessageKind::Anti
            && m.logical_id() == first_output.logical_id()));
    assert_eq!(runtime.report().replay_pending, 0);
}

#[test]
fn anti_before_positive_tombstones_exact_delivery_and_preserves_replacement() {
    let mut runtime = new_runtime(OptimisticLimits::default());
    let original = root_sender(event(0, 1, 5, &[4]), 3);
    let replacement = root_sender(event(0, 1, 5, &[8]), 4);
    let anti = original.as_anti();
    runtime.receive(anti.clone()).unwrap();
    runtime.receive(original.clone()).unwrap();
    runtime.receive(replacement.clone()).unwrap();
    let progress = runtime
        .run_until_with_budget(Tick::from_ticks(5), 8)
        .unwrap();
    assert_eq!(runtime.process_at(LpId(1)).unwrap().state.total, 8);
    assert_eq!(runtime.report().tombstones, 1);
    assert_eq!(progress.pending_positives, 0);
    assert_eq!(progress.pending_antis, 0);
    assert_eq!(progress.replay_pending, 0);
}

#[test]
fn anti_after_positive_cancels_it_and_fossil_collection_prunes_replay_metadata() {
    let mut runtime = new_runtime(OptimisticLimits::default());
    let message = root_sender(event(0, 1, 5, &[9]), 9);
    runtime.receive(message.clone()).unwrap();
    runtime
        .run_until_with_budget(Tick::from_ticks(5), 1)
        .unwrap();
    runtime.receive(message.as_anti()).unwrap();
    runtime
        .run_until_with_budget(Tick::from_ticks(5), 1)
        .unwrap();
    assert_eq!(runtime.process_at(LpId(1)).unwrap().state.total, 0);
    assert_eq!(runtime.report().replay_pending, 0);
    let collected = runtime.fossil_collect(Tick::from_ticks(6)).unwrap();
    assert_eq!(collected.new_gvt, Tick::from_ticks(6));
    assert_eq!(runtime.report().replay_pending, 0);
    assert!(matches!(
        runtime.receive(message),
        Err(OptimisticError::EventBeforeGvt { .. })
    ));
}

#[test]
fn stale_tokens_budget_resumption_and_equal_gvt_are_checked() {
    let mut runtime = new_runtime(OptimisticLimits::default());
    let foreign = new_runtime(OptimisticLimits::default());
    let token = runtime.state_token(LpId(0)).unwrap();
    assert!(!foreign.validate_state_token(token));
    runtime.schedule_initial(1, event(0, 0, 5, &[1])).unwrap();
    runtime.schedule_initial(2, event(0, 0, 6, &[2])).unwrap();
    let one = runtime
        .run_until_with_budget(Tick::from_ticks(6), 1)
        .unwrap();
    assert!(one.budget_exhausted);
    let two = runtime
        .run_until_with_budget(Tick::from_ticks(6), 1)
        .unwrap();
    assert_eq!(two.budget_used, 1);
    assert_eq!(runtime.process_at(LpId(0)).unwrap().state.total, 3);
    let gvt = runtime.fossil_collect(Tick::from_ticks(6)).unwrap();
    assert_eq!(gvt.previous_gvt, Tick::ZERO);
    assert_eq!(gvt.new_gvt, Tick::from_ticks(6));
    assert_eq!(gvt.collected.len(), 1);
    assert_eq!(
        runtime
            .fossil_collect(Tick::from_ticks(6))
            .unwrap()
            .previous_gvt,
        Tick::from_ticks(6)
    );
}

#[test]
fn invalid_post_handler_output_poisons_without_publishing_partial_batch() {
    let mut runtime = new_runtime(OptimisticLimits::default());
    runtime
        .schedule_initial(1, event(0, 0, 1, &[b'o', 1, 1, 2]))
        .unwrap();
    assert!(matches!(
        runtime.run_until_with_budget(Tick::from_ticks(2), 1),
        Err(OptimisticError::OutputNotStrictlyFuture { .. })
    ));
    assert!(matches!(
        runtime.run_until_with_budget(Tick::from_ticks(2), 1),
        Err(OptimisticError::Poisoned)
    ));
}

#[test]
fn child_identity_uses_parent_order_key_and_advances_causal_depth() {
    let mut runtime = new_runtime(OptimisticLimits::default());
    runtime
        .schedule_initial(4, event(0, 0, 1, &[b'o', 1, 2, 7]))
        .unwrap();
    let progress = runtime
        .run_until_with_budget(Tick::from_ticks(2), 1)
        .unwrap();
    let child = progress
        .published_messages
        .iter()
        .find(|message| message.kind() == OptimisticMessageKind::Positive)
        .unwrap();
    assert_eq!(child.logical_id().depth(), 1);
    assert_eq!(child.logical_id().root_parts(), None);
    assert_eq!(
        LogicalEventId::child(&child.order_key(), 0)
            .unwrap()
            .depth(),
        2
    );
    assert_eq!(child.event().event_payload, vec![7]);
}

#[test]
fn gvt_lag_reports_leading_frontier_even_with_idle_logical_process() {
    let mut runtime = new_runtime(OptimisticLimits::default());
    runtime.schedule_initial(1, event(0, 0, 20, &[1])).unwrap();
    runtime
        .run_until_with_budget(Tick::from_ticks(20), 1)
        .unwrap();
    assert_eq!(runtime.process_at(LpId(1)).unwrap().state.seen.len(), 0);
    assert_eq!(runtime.report().gvt_lag, SimDuration::from_ticks(20));
}
