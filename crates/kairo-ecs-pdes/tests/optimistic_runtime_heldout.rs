#![cfg(feature = "time-warp")]

use std::collections::BTreeMap;

use kairo_ecs_core::Scheduler;
use kairo_ecs_pdes::{
    LpId, OptimisticLimits, OptimisticMessage, OptimisticMessageKind, OptimisticProcess,
    OptimisticRuntime, OptimisticStateError, PartitionPlan, RemoteEvent,
};
use kairo_ecs_types::{EntityId, EventKind, ScheduleRequest, SimDuration, SimTime, StepOutcome};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Behavior {
    Normal,
    Cascade,
    StateDependentOutput,
    SnapshotPanic,
    RestorePanic,
    RestoreError,
    HandlerPanic,
    BadBatch,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Model {
    lp: LpId,
    value: u64,
    rng: u64,
    draws: Vec<u64>,
    output_dest: LpId,
    behavior: Behavior,
}

impl Model {
    fn new(lp: LpId, dest: LpId) -> Self {
        Self {
            lp,
            value: 0,
            rng: 123,
            draws: Vec::new(),
            output_dest: dest,
            behavior: Behavior::Normal,
        }
    }
}

fn event(source: u32, dest: u32, tick: u128, opcode: u8, digit: u8) -> RemoteEvent {
    RemoteEvent {
        source_lp: LpId(source),
        dest_lp: LpId(dest),
        tick: SimTime::from_ticks(tick),
        event_payload: vec![opcode, digit],
    }
}

impl OptimisticProcess for Model {
    type Snapshot = Self;

    fn snapshot(&self) -> Self {
        assert!(
            self.behavior != Behavior::SnapshotPanic || self.value == 0,
            "held-out snapshot fault"
        );
        self.clone()
    }

    fn restore(&mut self, state: &Self) -> Result<(), OptimisticStateError> {
        assert_ne!(
            self.behavior,
            Behavior::RestorePanic,
            "held-out restore fault"
        );
        if self.behavior == Behavior::RestoreError {
            return Err(OptimisticStateError::new("held-out epoch exhaustion"));
        }
        *self = state.clone();
        Ok(())
    }

    fn on_event(&mut self, input: &RemoteEvent) -> Vec<RemoteEvent> {
        if self.behavior == Behavior::HandlerPanic {
            self.value = 91;
            panic!("held-out handler fault");
        }
        if self.behavior == Behavior::BadBatch {
            self.value = 91;
            return vec![
                event(self.lp.0, self.output_dest.0, input.tick.ticks() + 1, 0, 7),
                event(self.lp.0, self.output_dest.0, input.tick.ticks(), 0, 8),
            ];
        }
        let mut digit = input.event_payload[1] as u64;
        if input.event_payload[0] == 2 {
            self.rng = self
                .rng
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            self.draws.push(self.rng);
            digit = (self.rng >> 32) % 10;
        }
        self.value = self.value * 10 + digit;
        if self.behavior == Behavior::StateDependentOutput {
            let (dest, tick) = if self.value >= 10 { (3, 40) } else { (2, 30) };
            return vec![event(self.lp.0, dest, tick, 0, self.value as u8)];
        }
        if input.event_payload[0] == 1 || input.event_payload[0] == 2 {
            return vec![event(self.lp.0, self.output_dest.0, 30, 0, digit as u8)];
        }
        if self.behavior == Behavior::Cascade {
            return vec![event(
                self.lp.0,
                self.output_dest.0,
                40,
                0,
                self.value as u8,
            )];
        }
        Vec::new()
    }
}

fn models(count: u32) -> BTreeMap<LpId, Model> {
    (0..count)
        .map(|id| (LpId(id), Model::new(LpId(id), LpId(count - 1))))
        .collect()
}

fn runtime(states: BTreeMap<LpId, Model>) -> OptimisticRuntime<Model> {
    let count = states.len() as u32;
    let partition = PartitionPlan::from_entities(
        count,
        SimDuration::from_ticks(1),
        (0..count).map(|id| EntityId::new(id as u64, 0)).collect(),
    )
    .unwrap();
    let topology = (0..count)
        .map(|id| {
            (
                LpId(id),
                (0..count).filter(|other| *other != id).map(LpId).collect(),
            )
        })
        .collect();
    OptimisticRuntime::new(partition, topology, states, OptimisticLimits::default()).unwrap()
}

fn envelope(seq: u64, input: RemoteEvent) -> OptimisticMessage {
    runtime(models(4)).schedule_initial(seq, input).unwrap()
}

fn drain(
    runtime: &mut OptimisticRuntime<Model>,
    horizon: u128,
    budget: usize,
) -> Vec<OptimisticMessage> {
    let mut messages = Vec::new();
    for _ in 0..512 {
        let progress = runtime
            .run_until_with_budget(SimTime::from_ticks(horizon), budget)
            .unwrap();
        assert!(progress.budget_used <= budget);
        assert_eq!(progress.budget_remaining, budget - progress.budget_used);
        messages.extend(progress.published_messages);
        if !progress.budget_exhausted {
            return messages;
        }
    }
    panic!("held-out finite scenario did not drain within its work bound");
}

fn value(rt: &OptimisticRuntime<Model>, lp: u32) -> u64 {
    rt.process_at(LpId(lp)).unwrap().value
}

fn values(runtime: &OptimisticRuntime<Model>, count: u32) -> Vec<(u64, u64, Vec<u64>)> {
    (0..count)
        .map(|id| {
            let model = runtime.process_at(LpId(id)).unwrap();
            (model.value, model.rng, model.draws.clone())
        })
        .collect()
}

fn sequential(
    mut states: BTreeMap<LpId, Model>,
    mut inputs: Vec<(u64, RemoteEvent)>,
) -> Vec<(u64, u64, Vec<u64>)> {
    let mut scheduler = Scheduler::new();
    let mut pending = BTreeMap::new();
    let mut next = 0u32;
    let schedule = |input: RemoteEvent,
                    scheduler: &mut Scheduler,
                    pending: &mut BTreeMap<u32, RemoteEvent>,
                    next: &mut u32| {
        let id = *next;
        *next += 1;
        scheduler.schedule(ScheduleRequest {
            at: input.tick,
            priority: input.source_lp.0 as i32,
            entity: None,
            kind: EventKind::Custom(id),
        });
        pending.insert(id, input);
    };
    inputs.sort_by_key(|(seq, input)| (input.tick, input.source_lp, *seq));
    for (_, input) in inputs {
        schedule(input, &mut scheduler, &mut pending, &mut next);
    }
    while let StepOutcome::Dispatched(dispatched) = scheduler.step() {
        let EventKind::Custom(id) = dispatched.kind;
        let input = pending.remove(&id).unwrap();
        for output in states.get_mut(&input.dest_lp).unwrap().on_event(&input) {
            schedule(output, &mut scheduler, &mut pending, &mut next);
        }
    }
    states
        .values()
        .map(|model| (model.value, model.rng, model.draws.clone()))
        .collect()
}

#[test]
fn equal_tick_reverse_source_and_sequence_arrival_matches_core_scheduler() {
    for same_source in [false, true] {
        let early = event(0, 2, 10, 0, 1);
        let future = event(if same_source { 0 } else { 1 }, 2, 10, 0, 2);
        let expected = sequential(models(3), vec![(1, early.clone()), (2, future.clone())]);
        assert_eq!(expected[2].0, 12);
        let mut rt = runtime(models(3));
        rt.schedule_initial(2, future).unwrap();
        drain(&mut rt, 10, 64);
        rt.receive(envelope(1, early)).unwrap();
        drain(&mut rt, 10, 64);
        assert_eq!(values(&rt, 3), expected);
    }
}

#[test]
fn late_parent_children_same_tick_keep_structural_order_and_rng_state() {
    for opcode in [1, 2] {
        let early = event(0, 1, 10, opcode, 1);
        let future = event(0, 1, 20, opcode, 2);
        let expected = sequential(models(3), vec![(1, early.clone()), (2, future.clone())]);
        if opcode == 1 {
            assert_eq!(expected[2].0, 12);
        }
        for reversed in [false, true] {
            for budget in [1, 64] {
                let mut rt = runtime(models(3));
                let (first_sequence, first, second_sequence, second) = if reversed {
                    (2, future.clone(), 1, early.clone())
                } else {
                    (1, early.clone(), 2, future.clone())
                };
                rt.schedule_initial(first_sequence, first).unwrap();
                drain(&mut rt, 30, budget);
                rt.receive(envelope(second_sequence, second)).unwrap();
                drain(&mut rt, 30, budget);
                assert_eq!(
                    values(&rt, 3),
                    expected,
                    "opcode={opcode}, reversed={reversed}, budget={budget}"
                );
                if reversed {
                    assert!(rt.report().replay_executions > 0);
                }
            }
        }
    }
}

fn cascade_models() -> BTreeMap<LpId, Model> {
    let mut states = models(4);
    states.get_mut(&LpId(1)).unwrap().output_dest = LpId(2);
    states.get_mut(&LpId(2)).unwrap().behavior = Behavior::Cascade;
    states
}

#[test]
fn canceling_earlier_parent_retracts_descendants_and_replays_surviving_inputs() {
    let early = event(0, 1, 10, 1, 1);
    let future = event(0, 1, 20, 1, 2);
    let expected = sequential(cascade_models(), vec![(2, future.clone())]);
    let mut rt = runtime(cascade_models());
    let canceled = rt.schedule_initial(1, early).unwrap();
    rt.schedule_initial(2, future).unwrap();
    drain(&mut rt, 40, 64);
    assert_eq!(value(&rt, 2), 12);
    assert_eq!(value(&rt, 3), 22);
    rt.receive(canceled.as_anti()).unwrap();
    drain(&mut rt, 40, 1);
    assert_eq!(values(&rt, 4), expected);
    assert_eq!(value(&rt, 3), 2);
    assert!(rt.report().canceled_sends > 0);
    assert_eq!(rt.report().replay_pending, 0);
}

#[test]
fn replayed_output_slot_can_change_payload_route_and_tick_without_identity_conflict() {
    let mut states = models(4);
    states.get_mut(&LpId(1)).unwrap().behavior = Behavior::StateDependentOutput;
    let surviving_input = event(0, 1, 20, 1, 2);
    let expected = sequential(states.clone(), vec![(2, surviving_input.clone())]);
    let mut rt = runtime(states);
    let canceled = rt.schedule_initial(1, event(0, 1, 10, 1, 1)).unwrap();
    rt.schedule_initial(2, surviving_input).unwrap();
    let original = drain(&mut rt, 40, 64);
    let old = original
        .iter()
        .find(|msg| msg.event().dest_lp == LpId(3))
        .unwrap();
    assert_eq!(old.event().tick, SimTime::from_ticks(40));
    assert_eq!(old.event().event_payload, vec![0, 12]);
    rt.receive(canceled.as_anti()).unwrap();
    let repaired = drain(&mut rt, 40, 1);
    let retraction = repaired
        .iter()
        .find(|msg| {
            msg.kind() == OptimisticMessageKind::Anti && msg.logical_id() == old.logical_id()
        })
        .unwrap();
    assert_eq!(retraction.event(), old.event());
    assert_eq!(retraction.incarnation(), old.incarnation());
    let replay = repaired
        .iter()
        .find(|msg| {
            msg.kind() == OptimisticMessageKind::Positive && msg.logical_id() == old.logical_id()
        })
        .unwrap();
    assert_ne!(replay.incarnation(), old.incarnation());
    assert_eq!(replay.event().dest_lp, LpId(2));
    assert_eq!(replay.event().tick, SimTime::from_ticks(30));
    assert_eq!(replay.event().event_payload, vec![0, 2]);
    assert_eq!(values(&rt, 4), expected);
}

#[test]
fn replacement_before_old_anti_survives_and_cascades_to_correct_final_state() {
    let mut sender = runtime(cascade_models());
    sender.schedule_initial(2, event(0, 1, 20, 1, 2)).unwrap();
    let first = drain(&mut sender, 40, 64);
    let old = first
        .iter()
        .find(|msg| msg.event().source_lp == LpId(1) && msg.event().dest_lp == LpId(2))
        .unwrap()
        .clone();
    sender.receive(envelope(1, event(0, 1, 10, 1, 1))).unwrap();
    let repaired = drain(&mut sender, 40, 64);
    let children: Vec<_> = repaired
        .into_iter()
        .filter(|msg| msg.event().source_lp == LpId(1) && msg.event().dest_lp == LpId(2))
        .collect();
    let replay = children
        .iter()
        .find(|msg| {
            msg.kind() == OptimisticMessageKind::Positive && msg.logical_id() == old.logical_id()
        })
        .unwrap()
        .clone();
    let earlier = children
        .iter()
        .find(|msg| {
            msg.kind() == OptimisticMessageKind::Positive && msg.logical_id() != old.logical_id()
        })
        .unwrap()
        .clone();
    assert_ne!(old.incarnation(), replay.incarnation());
    let expected = sequential(
        cascade_models(),
        vec![(1, event(0, 1, 10, 1, 1)), (2, event(0, 1, 20, 1, 2))],
    );
    let mut sink = runtime(cascade_models());
    for msg in [old.clone(), replay, earlier, old.as_anti(), old.as_anti()] {
        sink.receive(msg).unwrap();
        drain(&mut sink, 40, 1);
    }
    assert_eq!(&values(&sink, 4)[2..], &expected[2..]);
    assert_eq!(sink.report().replay_pending, 0);
}

#[test]
fn canceled_queued_replays_do_not_accumulate_across_fossil_cycles() {
    let mut rt = runtime(models(3));
    for cycle in 0..8u64 {
        let base = cycle as u128 * 40;
        let later = envelope(cycle * 2 + 2, event(0, 1, base + 20, 0, 2));
        rt.receive(later.clone()).unwrap();
        drain(&mut rt, base + 20, 64);
        rt.receive(envelope(cycle * 2 + 1, event(0, 1, base + 10, 0, 1)))
            .unwrap();
        let progress = rt
            .run_until_with_budget(SimTime::from_ticks(base + 20), 1)
            .unwrap();
        assert_eq!(
            progress.replay_pending, 1,
            "fixture must leave later input queued for replay"
        );
        rt.receive(later.as_anti()).unwrap();
        drain(&mut rt, base + 20, 1);
        assert_eq!(rt.report().replay_pending, 0);
        rt.fossil_collect(SimTime::from_ticks(base + 21)).unwrap();
        let report = rt.report();
        assert_eq!(report.pending_events, 0);
        assert_eq!(report.history_events, 0);
        assert_eq!(report.tombstones, 0);
        assert_eq!(report.replay_pending, 0);
    }
}

#[test]
fn anti_before_positive_does_not_apply_and_duplicate_delivery_is_unchanged() {
    let msg = envelope(77, event(0, 1, 10, 0, 9));
    let mut rt = runtime(models(3));
    rt.receive(msg.as_anti()).unwrap();
    drain(&mut rt, 10, 1);
    rt.receive(msg.clone()).unwrap();
    drain(&mut rt, 10, 1);
    assert_eq!(value(&rt, 1), 0);
    rt.receive(msg.as_anti()).unwrap();
    drain(&mut rt, 10, 1);
    assert_eq!(value(&rt, 1), 0);

    let mut live = runtime(models(3));
    live.receive(msg.clone()).unwrap();
    drain(&mut live, 10, 1);
    let before = values(&live, 3);
    let _duplicate_result = live.receive(msg);
    assert_eq!(values(&live, 3), before);
    drain(&mut live, 10, 1);
    assert_eq!(values(&live, 3), before);
}

#[test]
fn same_opaque_child_identity_from_distinct_emitters_cancels_independently() {
    let mut children = Vec::new();
    for (dest, digit) in [(1, 1), (2, 2)] {
        let mut sender = runtime(models(4));
        sender
            .schedule_initial(77, event(0, dest, 10, 1, digit))
            .unwrap();
        children.push(
            drain(&mut sender, 30, 64)
                .into_iter()
                .find(|msg| msg.event().source_lp == LpId(dest))
                .unwrap(),
        );
    }
    assert_eq!(children[0].logical_id(), children[1].logical_id());
    let mut sink = runtime(models(4));
    sink.receive(children[1].clone()).unwrap();
    drain(&mut sink, 30, 64);
    sink.receive(children[0].clone()).unwrap();
    drain(&mut sink, 30, 64);
    assert_eq!(value(&sink, 3), 12);
    sink.receive(children[0].as_anti()).unwrap();
    drain(&mut sink, 30, 64);
    assert_eq!(value(&sink, 3), 2);
}

#[test]
fn conflicting_exact_delivery_across_destination_lps_is_rejected_unchanged() {
    let first = envelope(77, event(0, 1, 10, 0, 1));
    let conflicting = envelope(77, event(0, 2, 10, 0, 9));
    assert_eq!(first.logical_id(), conflicting.logical_id());
    assert_eq!(first.incarnation(), conflicting.incarnation());
    let mut rt = runtime(models(3));
    rt.receive(first).unwrap();
    let before_report = rt.report();
    let before_queues: Vec<_> = (0..3)
        .map(|id| rt.pending_events(LpId(id)).unwrap())
        .collect();
    let tokens: Vec<_> = (0..3).map(|id| rt.state_token(LpId(id)).unwrap()).collect();
    assert!(rt.receive(conflicting).is_err());
    assert_eq!(rt.report(), before_report);
    for id in 0..3 {
        assert_eq!(
            rt.pending_events(LpId(id)).unwrap(),
            before_queues[id as usize]
        );
    }
    for token in tokens {
        assert!(rt.validate_state_token(token));
    }
    drain(&mut rt, 10, 64);
    assert_eq!(value(&rt, 1), 1);
    assert_eq!(value(&rt, 2), 0);
}

#[test]
fn work_at_gvt_remains_reversible_and_pre_floor_inputs_fail_unchanged() {
    let mut rt = runtime(models(3));
    rt.schedule_initial(1, event(0, 1, 10, 0, 1)).unwrap();
    let at_floor = rt.schedule_initial(2, event(0, 1, 20, 0, 2)).unwrap();
    drain(&mut rt, 20, 64);
    let collected = rt.fossil_collect(SimTime::from_ticks(20)).unwrap();
    assert_eq!(collected.collected.len(), 1);
    assert_eq!(collected.collected[0].event.tick, SimTime::from_ticks(10));
    rt.receive(at_floor.as_anti()).unwrap();
    drain(&mut rt, 20, 1);
    assert_eq!(value(&rt, 1), 1);
    let before = values(&rt, 3);
    let old = envelope(3, event(0, 1, 19, 0, 9));
    assert!(rt.receive(old.clone()).is_err());
    assert!(rt.receive(old.as_anti()).is_err());
    assert_eq!(values(&rt, 3), before);
    assert!(rt.fossil_collect(SimTime::from_ticks(19)).is_err());
    assert_eq!(values(&rt, 3), before);
}

#[test]
fn lp_state_tokens_are_runtime_bound_and_invalidated_by_mutation_and_restore() {
    let mut rt = runtime(models(3));
    let other = runtime(models(3));
    let idle = rt.state_token(LpId(0)).unwrap();
    let seed = rt.state_token(LpId(1)).unwrap();
    assert!(rt.validate_state_token(seed));
    assert!(!other.validate_state_token(seed));
    rt.schedule_initial(2, event(0, 1, 20, 0, 2)).unwrap();
    drain(&mut rt, 20, 64);
    assert!(!rt.validate_state_token(seed));
    assert!(rt.validate_state_token(idle));
    let speculative = rt.state_token(LpId(1)).unwrap();
    rt.receive(envelope(1, event(0, 1, 10, 0, 1))).unwrap();
    drain(&mut rt, 20, 1);
    assert!(!rt.validate_state_token(speculative));
    assert!(rt.validate_state_token(idle));
}

#[test]
fn invalid_complete_batch_and_handler_fault_publish_no_partial_outputs() {
    for behavior in [Behavior::BadBatch, Behavior::HandlerPanic] {
        let mut states = models(3);
        states.get_mut(&LpId(1)).unwrap().behavior = behavior;
        let mut rt = runtime(states);
        let token = rt.state_token(LpId(1)).unwrap();
        rt.schedule_initial(1, event(0, 1, 10, 0, 1)).unwrap();
        assert!(rt
            .run_until_with_budget(SimTime::from_ticks(30), 64)
            .is_err());
        assert_eq!(value(&rt, 2), 0);
        assert!(rt.pending_events(LpId(2)).unwrap().is_empty());
        assert!(!rt.validate_state_token(token));
        assert!(rt
            .run_until_with_budget(SimTime::from_ticks(30), 64)
            .is_err());
    }
}

#[test]
fn snapshot_and_restore_faults_poison_without_publishing_replacement_outputs() {
    for behavior in [
        Behavior::SnapshotPanic,
        Behavior::RestorePanic,
        Behavior::RestoreError,
    ] {
        let mut states = models(3);
        states.get_mut(&LpId(1)).unwrap().behavior = behavior;
        let mut rt = runtime(states);
        rt.schedule_initial(2, event(0, 1, 20, 0, 2)).unwrap();
        drain(&mut rt, 20, 64);
        let before = rt.process_at(LpId(2)).unwrap().clone();
        rt.receive(envelope(1, event(0, 1, 10, 0, 1))).unwrap();
        assert!(rt
            .run_until_with_budget(SimTime::from_ticks(20), 64)
            .is_err());
        assert_eq!(rt.process_at(LpId(2)).unwrap(), &before);
        assert!(rt.pending_events(LpId(2)).unwrap().is_empty());
        assert!(rt
            .run_until_with_budget(SimTime::from_ticks(20), 64)
            .is_err());
    }
}
