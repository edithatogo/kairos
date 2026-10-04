#![cfg(all(feature = "pdes", feature = "time-warp"))]

use std::collections::BTreeMap;

use kairo_ecs_core::Scheduler;
use kairo_ecs_pdes::{
    LogicalEventId, LpId, OptimisticError, OptimisticEventOrderKey, OptimisticLimits,
    OptimisticMessage, OptimisticMessageKind, OptimisticProcess, OptimisticRuntime,
    OptimisticStateError, PartitionPlan, RemoteEvent, Tick,
};
use kairo_ecs_types::{EntityId, EventKind, ScheduleRequest, SimDuration, SimTime, StepOutcome};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Behavior {
    Normal,
    Cascade,
    StateDependentOutput,
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
    fn new(lp: LpId, output_dest: LpId) -> Self {
        Self {
            lp,
            value: 0,
            rng: 123,
            draws: Vec::new(),
            output_dest,
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

    fn snapshot(&self) -> Self::Snapshot {
        self.clone()
    }

    fn restore(&mut self, state: &Self::Snapshot) -> Result<(), OptimisticStateError> {
        *self = state.clone();
        Ok(())
    }

    fn on_event(&mut self, input: &RemoteEvent) -> Vec<RemoteEvent> {
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
        if self.behavior == Behavior::Cascade {
            return vec![event(
                self.lp.0,
                self.output_dest.0,
                40,
                0,
                self.value as u8,
            )];
        }
        if input.event_payload[0] == 1 || input.event_payload[0] == 2 {
            return vec![event(self.lp.0, self.output_dest.0, 30, 0, digit as u8)];
        }
        Vec::new()
    }
}

fn models(count: u32) -> BTreeMap<LpId, Model> {
    (0..count)
        .map(|id| (LpId(id), Model::new(LpId(id), LpId(count - 1))))
        .collect()
}

fn cascade_models() -> BTreeMap<LpId, Model> {
    let mut states = models(4);
    states.get_mut(&LpId(1)).unwrap().output_dest = LpId(2);
    states.get_mut(&LpId(2)).unwrap().behavior = Behavior::Cascade;
    states
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

fn envelope(sequence: u64, input: RemoteEvent) -> OptimisticMessage {
    runtime(models(4))
        .schedule_initial(sequence, input)
        .unwrap()
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
    panic!("finite held-out scenario did not drain within its work bound");
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
    inputs.sort_by_key(|(sequence, input)| (input.tick, input.source_lp, *sequence));
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

// A deliberately small test-side projection models the fields a codec must
// preserve. Reconstruction goes only through the accepted public constructors.
#[derive(Clone, Debug, Eq, PartialEq)]
enum ProjectedId {
    Root {
        source: LpId,
        sequence: u64,
    },
    Output {
        parent_tick: Tick,
        parent_source: LpId,
        parent_id: Box<ProjectedId>,
        ordinal: u32,
    },
}

fn project_id(id: &LogicalEventId) -> ProjectedId {
    if let Some((source, sequence)) = id.root_parts() {
        return ProjectedId::Root { source, sequence };
    }
    let (parent, ordinal) = id.output_parts().expect("non-root identity is an output");
    ProjectedId::Output {
        parent_tick: parent.tick(),
        parent_source: parent.source_lp(),
        parent_id: Box::new(project_id(parent.logical_id())),
        ordinal,
    }
}

fn reconstruct_id(projected: ProjectedId) -> LogicalEventId {
    match projected {
        ProjectedId::Root { source, sequence } => LogicalEventId::root(source, sequence),
        ProjectedId::Output {
            parent_tick,
            parent_source,
            parent_id,
            ordinal,
        } => {
            let parent_id = reconstruct_id(*parent_id);
            let parent =
                OptimisticEventOrderKey::try_from_parts(parent_tick, parent_source, parent_id)
                    .unwrap();
            LogicalEventId::child(&parent, ordinal).unwrap()
        }
    }
}

fn through_public_bridge(message: &OptimisticMessage) -> OptimisticMessage {
    let logical_id = reconstruct_id(project_id(message.logical_id()));
    OptimisticMessage::try_from_parts(
        message.event().clone(),
        logical_id,
        message.incarnation(),
        message.kind(),
    )
    .unwrap()
}

fn root_source(id: &LogicalEventId) -> LpId {
    if let Some((source, _)) = id.root_parts() {
        source
    } else {
        root_source(id.output_parts().unwrap().0.logical_id())
    }
}

#[test]
fn emitted_multigeneration_messages_rebuild_and_late_old_antis_match_reference() {
    let early = event(0, 1, 10, 1, 1);
    let future = event(0, 1, 20, 1, 2);
    let expected = sequential(
        cascade_models(),
        vec![(1, early.clone()), (2, future.clone())],
    );

    let mut sender = runtime(cascade_models());
    sender.schedule_initial(2, future).unwrap();
    let initial = drain(&mut sender, 40, 64);
    let old_child = initial
        .iter()
        .find(|message| message.logical_id().depth() == 1)
        .unwrap()
        .clone();
    let old_grandchild = initial
        .iter()
        .find(|message| message.logical_id().depth() == 2)
        .unwrap()
        .clone();
    assert_eq!(root_source(old_child.logical_id()), LpId(0));
    assert_eq!(old_child.event().source_lp, LpId(1));
    assert_eq!(old_grandchild.event().source_lp, LpId(2));
    let (grandchild_parent, grandchild_ordinal) =
        old_grandchild.logical_id().output_parts().unwrap();
    assert_eq!(grandchild_parent.source_lp(), LpId(1));
    assert_eq!(grandchild_parent.logical_id(), old_child.logical_id());
    assert_eq!(grandchild_ordinal, 0);

    sender.receive(envelope(1, early)).unwrap();
    let repaired = drain(&mut sender, 40, 1);
    assert_eq!(values(&sender, 4), expected);
    let old_child_anti = repaired
        .iter()
        .find(|message| {
            message.kind() == OptimisticMessageKind::Anti
                && message.logical_id() == old_child.logical_id()
                && message.incarnation() == old_child.incarnation()
        })
        .unwrap();
    let old_grandchild_anti = repaired
        .iter()
        .find(|message| {
            message.kind() == OptimisticMessageKind::Anti
                && message.logical_id() == old_grandchild.logical_id()
                && message.incarnation() == old_grandchild.incarnation()
        })
        .unwrap();
    assert_eq!(old_child_anti, &old_child.as_anti());
    assert_eq!(old_grandchild_anti, &old_grandchild.as_anti());

    for old in [&old_child, &old_grandchild] {
        let replacement = repaired
            .iter()
            .find(|message| {
                message.kind() == OptimisticMessageKind::Positive
                    && message.logical_id() == old.logical_id()
            })
            .unwrap();
        assert_ne!(replacement.incarnation(), old.incarnation());
    }
    let old_grandchild_replacement = repaired
        .iter()
        .find(|message| {
            message.kind() == OptimisticMessageKind::Positive
                && message.logical_id() == old_grandchild.logical_id()
        })
        .unwrap();
    assert_ne!(old_grandchild_replacement.event(), old_grandchild.event());

    // Deliver every old and replacement positive first, then the old antis.
    // This is the hazardous arrival order: the exact stale incarnation must be
    // removed while the replacement of the same logical output slot survives.
    let mut sink = runtime(models(4));
    for message in initial
        .iter()
        .chain(repaired.iter())
        .filter(|message| message.kind() == OptimisticMessageKind::Positive)
    {
        sink.receive(through_public_bridge(message)).unwrap();
        drain(&mut sink, 40, 1);
    }
    for anti in [old_child_anti, old_grandchild_anti] {
        let rebuilt = through_public_bridge(anti);
        sink.receive(rebuilt.clone()).unwrap();
        drain(&mut sink, 40, 1);
        sink.receive(rebuilt).unwrap();
        drain(&mut sink, 40, 1);
    }
    assert_eq!(&values(&sink, 4)[2..], &expected[2..]);
    assert_eq!(sink.report().replay_pending, 0);
}

#[test]
fn full_parent_key_order_is_stable_under_root_arrival_permutations() {
    let roots = [
        envelope(7, event(0, 1, 10, 1, 3)),
        envelope(7, event(2, 1, 10, 1, 3)),
    ];
    let emitted = |order: [usize; 2]| {
        let mut receiver = runtime(models(4));
        for index in order {
            receiver
                .receive(through_public_bridge(&roots[index]))
                .unwrap();
        }
        let messages = drain(&mut receiver, 30, 1);
        let mut children: Vec<_> = messages
            .into_iter()
            .filter(|message| message.logical_id().depth() == 1)
            .collect();
        children.sort_by_key(OptimisticMessage::order_key);
        children
    };
    let forward = emitted([0, 1]);
    let reverse = emitted([1, 0]);
    assert_eq!(forward, reverse);
    assert_eq!(forward.len(), 2);
    assert_eq!(forward[0].event().source_lp, LpId(1));
    assert_eq!(forward[1].event().source_lp, LpId(1));
    let (first_parent, first_ordinal) = forward[0].logical_id().output_parts().unwrap();
    let (second_parent, second_ordinal) = forward[1].logical_id().output_parts().unwrap();
    assert_eq!(
        (first_parent.source_lp(), second_parent.source_lp()),
        (LpId(0), LpId(2))
    );
    assert_eq!((first_ordinal, second_ordinal), (0, 0));
    assert!(forward[0].order_key() < forward[1].order_key());

    let child_at = |parent_tick| {
        let mut producer = runtime(models(4));
        producer
            .schedule_initial(9, event(0, 1, parent_tick, 1, 4))
            .unwrap();
        drain(&mut producer, 30, 64)
            .into_iter()
            .find(|message| message.logical_id().depth() == 1)
            .unwrap()
    };
    let earlier_parent = child_at(10);
    let later_parent = child_at(20);
    assert_eq!(
        earlier_parent.order_key().tick(),
        later_parent.order_key().tick()
    );
    assert_eq!(
        earlier_parent.order_key().source_lp(),
        later_parent.order_key().source_lp()
    );
    assert_ne!(earlier_parent.logical_id(), later_parent.logical_id());
    assert!(earlier_parent.order_key() < later_parent.order_key());
}

#[test]
fn state_changing_replay_survives_its_old_anti_after_public_reconstruction() {
    let mut states = models(4);
    states.get_mut(&LpId(1)).unwrap().behavior = Behavior::StateDependentOutput;
    let surviving_input = event(0, 1, 20, 1, 2);
    let expected = sequential(states.clone(), vec![(2, surviving_input.clone())]);
    let mut sender = runtime(states);
    let canceled = sender.schedule_initial(1, event(0, 1, 10, 1, 1)).unwrap();
    sender.schedule_initial(2, surviving_input).unwrap();
    let initial = drain(&mut sender, 40, 64);
    let old = initial
        .iter()
        .find(|message| message.event().dest_lp == LpId(3))
        .unwrap()
        .clone();
    sender.receive(canceled.as_anti()).unwrap();
    let repaired = drain(&mut sender, 40, 1);
    let anti = repaired
        .iter()
        .find(|message| {
            message.kind() == OptimisticMessageKind::Anti
                && message.logical_id() == old.logical_id()
                && message.incarnation() == old.incarnation()
        })
        .unwrap();
    let replacement = repaired
        .iter()
        .find(|message| {
            message.kind() == OptimisticMessageKind::Positive
                && message.logical_id() == old.logical_id()
        })
        .unwrap();
    assert_eq!(anti, &old.as_anti());
    assert_ne!(replacement.incarnation(), old.incarnation());
    assert_eq!(replacement.logical_id(), old.logical_id());
    assert_eq!(replacement.event().dest_lp, LpId(2));
    assert_eq!(replacement.event().tick, SimTime::from_ticks(30));
    assert_eq!(replacement.event().event_payload, vec![0, 2]);

    let mut sink = runtime(models(4));
    sink.receive(through_public_bridge(&old)).unwrap();
    drain(&mut sink, 40, 1);
    sink.receive(through_public_bridge(replacement)).unwrap();
    drain(&mut sink, 40, 1);
    let rebuilt_anti = through_public_bridge(anti);
    sink.receive(rebuilt_anti.clone()).unwrap();
    drain(&mut sink, 40, 1);
    sink.receive(rebuilt_anti).unwrap();
    drain(&mut sink, 40, 1);
    assert_eq!(&values(&sink, 4)[2..], &expected[2..]);
}

#[test]
fn conflicting_decoded_metadata_and_duplicate_positive_leave_receiver_unchanged() {
    let mut producer = runtime(models(4));
    producer.schedule_initial(1, event(0, 1, 10, 1, 5)).unwrap();
    let message = drain(&mut producer, 30, 64)
        .into_iter()
        .find(|message| message.logical_id().depth() == 1)
        .unwrap();
    let mut receiver = runtime(models(4));
    let decoded = through_public_bridge(&message);
    receiver.receive(decoded.clone()).unwrap();

    let report_before = receiver.report().clone();
    let pending_before: Vec<_> = (0..4).map(|lp| receiver.pending_events(LpId(lp))).collect();
    let values_before = values(&receiver, 4);
    let tokens: Vec<_> = (0..4)
        .map(|lp| receiver.state_token(LpId(lp)).unwrap())
        .collect();

    assert!(matches!(
        receiver.receive(decoded.clone()),
        Err(OptimisticError::DuplicatePositive { .. })
    ));
    let mut conflicts = Vec::new();
    let mut changed_payload = decoded.event().clone();
    changed_payload.event_payload[1] ^= 0xff;
    conflicts.push(changed_payload);
    let mut changed_tick = decoded.event().clone();
    changed_tick.tick = SimTime::from_ticks(changed_tick.tick.ticks() + 1);
    conflicts.push(changed_tick);
    let mut changed_destination = decoded.event().clone();
    changed_destination.dest_lp = LpId(2);
    conflicts.push(changed_destination);

    for changed_event in conflicts {
        let conflict = OptimisticMessage::try_from_parts(
            changed_event,
            decoded.logical_id().clone(),
            decoded.incarnation(),
            decoded.kind(),
        )
        .unwrap();
        assert!(matches!(
            receiver.receive(conflict),
            Err(OptimisticError::ConflictingDelivery { .. })
        ));
        assert_eq!(receiver.report(), report_before);
        assert_eq!(
            (0..4)
                .map(|lp| receiver.pending_events(LpId(lp)))
                .collect::<Vec<_>>(),
            pending_before
        );
        assert_eq!(values(&receiver, 4), values_before);
        for (lp, token) in tokens.iter().enumerate() {
            assert!(
                receiver.validate_state_token(*token),
                "LP {lp} token changed"
            );
        }
    }
}

#[test]
fn checked_bridge_preserves_full_ranges_and_rejects_invalid_structure() {
    let max_lp = LpId(u32::MAX);
    let max_tick = Tick::from_ticks(u128::MAX);
    let root_id = LogicalEventId::root(max_lp, u64::MAX);
    let root_key =
        OptimisticEventOrderKey::try_from_parts(max_tick, max_lp, root_id.clone()).unwrap();
    let root = OptimisticMessage::try_from_parts(
        RemoteEvent {
            source_lp: max_lp,
            dest_lp: max_lp,
            tick: max_tick,
            event_payload: vec![0x00, 0xff, 0x80],
        },
        root_id,
        u64::MAX,
        OptimisticMessageKind::Positive,
    )
    .unwrap();
    assert_eq!(root.order_key(), root_key);
    assert_eq!(root.event().tick.ticks(), u128::MAX);
    assert_eq!(root.logical_id().root_parts(), Some((max_lp, u64::MAX)));
    assert_eq!(root.incarnation(), u64::MAX);
    assert_eq!(root.event().event_payload, vec![0x00, 0xff, 0x80]);

    let same_order_anti = OptimisticMessage::try_from_parts(
        root.event().clone(),
        root.logical_id().clone(),
        0,
        OptimisticMessageKind::Anti,
    )
    .unwrap();
    assert_eq!(root.order_key(), same_order_anti.order_key());
    assert_eq!(root.order_key(), through_public_bridge(&root).order_key());

    let parent = OptimisticEventOrderKey::try_from_parts(
        Tick::from_ticks(u128::MAX - 1),
        LpId(0),
        LogicalEventId::root(LpId(0), u64::MAX),
    )
    .unwrap();
    let output_id = LogicalEventId::child(&parent, u32::MAX).unwrap();
    let output = OptimisticMessage::try_from_parts(
        RemoteEvent {
            source_lp: LpId(1),
            dest_lp: LpId(2),
            tick: max_tick,
            event_payload: vec![0x00, 0xff, 0x80],
        },
        output_id,
        u64::MAX,
        OptimisticMessageKind::Anti,
    )
    .unwrap();
    let (output_parent, output_ordinal) = output.logical_id().output_parts().unwrap();
    assert_eq!(
        (output_parent.tick(), output_parent.source_lp()),
        (parent.tick(), parent.source_lp())
    );
    assert_eq!(output_ordinal, u32::MAX);
    assert_eq!(output.event().tick.ticks(), u128::MAX);
    assert_eq!(output.event().source_lp, LpId(1));
    assert_eq!(output.incarnation(), u64::MAX);
    assert_eq!(output.kind(), OptimisticMessageKind::Anti);

    assert!(matches!(
        OptimisticEventOrderKey::try_from_parts(
            Tick::from_ticks(1),
            LpId(1),
            LogicalEventId::root(LpId(0), 1),
        ),
        Err(OptimisticError::EnvelopeSourceMismatch {
            declared: LpId(0),
            actual: LpId(1),
        })
    ));

    let root_parent = OptimisticEventOrderKey::try_from_parts(
        Tick::from_ticks(10),
        LpId(0),
        LogicalEventId::root(LpId(0), 4),
    )
    .unwrap();
    let child_id = LogicalEventId::child(&root_parent, 0).unwrap();
    for invalid_tick in [10, 9] {
        assert!(matches!(
            OptimisticMessage::try_from_parts(
                event(1, 2, invalid_tick, 0, 1),
                child_id.clone(),
                0,
                OptimisticMessageKind::Positive,
            ),
            Err(OptimisticError::OutputNotStrictlyFuture { .. })
        ));
    }
    let child_key =
        OptimisticEventOrderKey::try_from_parts(Tick::from_ticks(11), LpId(1), child_id.clone())
            .unwrap();
    let grandchild = LogicalEventId::child(&child_key, 2).unwrap();
    assert!(matches!(
        OptimisticEventOrderKey::try_from_parts(Tick::from_ticks(11), LpId(2), grandchild),
        Err(OptimisticError::OutputNotStrictlyFuture { .. })
    ));

    let max_parent = OptimisticEventOrderKey::try_from_parts(
        max_tick,
        LpId(0),
        LogicalEventId::root(LpId(0), 5),
    )
    .unwrap();
    let impossible_future = LogicalEventId::child(&max_parent, 0).unwrap();
    assert!(matches!(
        OptimisticMessage::try_from_parts(
            RemoteEvent {
                source_lp: LpId(1),
                dest_lp: LpId(2),
                tick: max_tick,
                event_payload: Vec::new(),
            },
            impossible_future,
            0,
            OptimisticMessageKind::Positive,
        ),
        Err(OptimisticError::OutputNotStrictlyFuture { .. })
    ));

    let mut key = OptimisticEventOrderKey::try_from_parts(
        Tick::ZERO,
        LpId(0),
        LogicalEventId::root(LpId(0), 6),
    )
    .unwrap();
    let mut id = key.logical_id().clone();
    for depth in 1..=128u128 {
        id = LogicalEventId::child(&key, depth as u32).unwrap();
        key = OptimisticEventOrderKey::try_from_parts(Tick::from_ticks(depth), LpId(1), id.clone())
            .unwrap();
    }
    assert_eq!(id.depth(), 128);
    let depth_128 = OptimisticMessage::try_from_parts(
        event(1, 1, 129, 0, 1),
        id,
        0,
        OptimisticMessageKind::Positive,
    )
    .unwrap();
    assert_eq!(depth_128.logical_id().depth(), 128);
    assert!(matches!(
        LogicalEventId::child(&key, 0),
        Err(OptimisticError::CausalDepthExceeded {
            depth: 129,
            limit: 128,
        })
    ));
}
