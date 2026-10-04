#![cfg(all(feature = "pdes", feature = "time-warp"))]

use std::collections::BTreeMap;

use kairo_ecs_pdes::{
    LogicalEventId, LpId, OptimisticAuthority, OptimisticError, OptimisticEventOrderKey,
    OptimisticLimits, OptimisticMessage, OptimisticMessageKind, OptimisticProcess,
    OptimisticRuntime, OptimisticRuntimeReport, OptimisticStateError, OptimisticStateToken,
    PartitionPlan, RemoteEvent, Tick,
};
use kairo_ecs_types::{EntityId, SimDuration, SimTime};

#[derive(Clone, Debug, Eq, PartialEq)]
struct Model {
    lp: LpId,
    value: u64,
    rng: u64,
    draws: Vec<u64>,
    panic_on_event: bool,
}

impl Model {
    fn new(lp: LpId) -> Self {
        Self {
            lp,
            value: 0,
            rng: 123,
            draws: Vec::new(),
            panic_on_event: false,
        }
    }
}

fn event(source: u32, destination: u32, tick: u128, opcode: u8, digit: u8) -> RemoteEvent {
    RemoteEvent {
        source_lp: LpId(source),
        dest_lp: LpId(destination),
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
        if self.panic_on_event {
            self.value = 999;
            panic!("authority held-out poison fixture");
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
        if self.lp == LpId(1) && input.event_payload[0] == 1 {
            return vec![event(self.lp.0, 2, input.tick.ticks() + 10, 0, digit as u8)];
        }
        Vec::new()
    }
}

fn models(count: u32) -> BTreeMap<LpId, Model> {
    (0..count)
        .map(|id| (LpId(id), Model::new(LpId(id))))
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

fn drain(
    runtime: &mut OptimisticRuntime<Model>,
    horizon: u128,
    budget: usize,
) -> Vec<OptimisticMessage> {
    let mut messages = Vec::new();
    for _ in 0..256 {
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
    panic!("finite authority fixture failed to drain within its work bound");
}

fn all_models(runtime: &OptimisticRuntime<Model>, count: u32) -> Vec<Model> {
    (0..count)
        .map(|id| runtime.process_at(LpId(id)).unwrap().clone())
        .collect()
}

fn authority_message(
    message: &OptimisticMessage,
    authority: OptimisticAuthority,
    kind: OptimisticMessageKind,
) -> OptimisticMessage {
    OptimisticMessage::try_from_authority_parts(
        message.event().clone(),
        message.logical_id().clone(),
        authority,
        message.incarnation(),
        kind,
    )
    .unwrap()
}

const MAX_AUTHORITY: OptimisticAuthority = OptimisticAuthority::Scoped {
    simulation_namespace: u128::MAX,
    ownership_epoch: u64::MAX,
};

#[test]
fn authority_constructor_preserves_full_nested_envelope_without_changing_order() {
    let root_id = LogicalEventId::root(LpId(0), u64::MAX);
    let root_key =
        OptimisticEventOrderKey::try_from_parts(Tick::from_ticks(u128::MAX - 2), LpId(0), root_id)
            .unwrap();
    let child_id = LogicalEventId::child(&root_key, u32::MAX).unwrap();
    let child_key = OptimisticEventOrderKey::try_from_parts(
        Tick::from_ticks(u128::MAX - 1),
        LpId(1),
        child_id.clone(),
    )
    .unwrap();
    let grandchild_id = LogicalEventId::child(&child_key, 17).unwrap();
    let original_event = RemoteEvent {
        source_lp: LpId(2),
        dest_lp: LpId(u32::MAX),
        tick: SimTime::from_ticks(u128::MAX),
        event_payload: vec![0x00, 0xff, 0x80, 0x7f],
    };

    let scoped = OptimisticMessage::try_from_authority_parts(
        original_event.clone(),
        grandchild_id.clone(),
        MAX_AUTHORITY,
        u64::MAX,
        OptimisticMessageKind::Positive,
    )
    .unwrap();
    assert_eq!(
        scoped.authority(),
        OptimisticAuthority::Scoped {
            simulation_namespace: u128::MAX,
            ownership_epoch: u64::MAX,
        }
    );
    assert_eq!(scoped.event(), &original_event);
    assert_eq!(scoped.incarnation(), u64::MAX);
    assert_eq!(scoped.kind(), OptimisticMessageKind::Positive);
    assert_eq!(scoped.logical_id().depth(), 2);
    let (grandchild_parent, ordinal) = scoped.logical_id().output_parts().unwrap();
    assert_eq!(ordinal, 17);
    assert_eq!(grandchild_parent.tick(), Tick::from_ticks(u128::MAX - 1));
    assert_eq!(grandchild_parent.source_lp(), LpId(1));
    let (child_parent, child_ordinal) = grandchild_parent.logical_id().output_parts().unwrap();
    assert_eq!(child_ordinal, u32::MAX);
    assert_eq!(child_parent.tick(), Tick::from_ticks(u128::MAX - 2));
    assert_eq!(child_parent.source_lp(), LpId(0));
    assert_eq!(
        child_parent.logical_id().root_parts(),
        Some((LpId(0), u64::MAX))
    );

    let local_legacy = OptimisticMessage::try_from_parts(
        original_event.clone(),
        grandchild_id.clone(),
        0,
        OptimisticMessageKind::Positive,
    )
    .unwrap();
    assert_eq!(local_legacy.authority(), OptimisticAuthority::LocalPreview);
    let explicit_local = OptimisticMessage::try_from_authority_parts(
        original_event.clone(),
        grandchild_id.clone(),
        OptimisticAuthority::LocalPreview,
        0,
        OptimisticMessageKind::Positive,
    )
    .unwrap();
    assert_eq!(explicit_local, local_legacy);
    assert_eq!(explicit_local.order_key(), scoped.order_key());

    let zero_scope = OptimisticMessage::try_from_authority_parts(
        original_event.clone(),
        grandchild_id.clone(),
        OptimisticAuthority::Scoped {
            simulation_namespace: 0,
            ownership_epoch: 0,
        },
        0,
        OptimisticMessageKind::Anti,
    )
    .unwrap();
    assert_eq!(zero_scope.kind(), OptimisticMessageKind::Anti);
    assert_eq!(zero_scope.incarnation(), 0);
    assert_eq!(zero_scope.order_key(), scoped.order_key());

    let clone = scoped.clone();
    assert_eq!(clone, scoped);
    let anti = scoped.as_anti();
    assert_eq!(anti.authority(), scoped.authority());
    assert_eq!(anti.event(), scoped.event());
    assert_eq!(anti.logical_id(), scoped.logical_id());
    assert_eq!(anti.incarnation(), scoped.incarnation());
    assert_eq!(anti.kind(), OptimisticMessageKind::Anti);
    assert_eq!(anti.order_key(), scoped.order_key());

    assert!(matches!(
        OptimisticMessage::try_from_authority_parts(
            event(1, 2, 1, 0, 0),
            LogicalEventId::root(LpId(0), 9),
            MAX_AUTHORITY,
            0,
            OptimisticMessageKind::Positive,
        ),
        Err(OptimisticError::EnvelopeSourceMismatch {
            declared: LpId(0),
            actual: LpId(1),
        })
    ));
    let parent = OptimisticEventOrderKey::try_from_parts(
        Tick::from_ticks(10),
        LpId(0),
        LogicalEventId::root(LpId(0), 10),
    )
    .unwrap();
    let invalid_child = LogicalEventId::child(&parent, 0).unwrap();
    assert!(matches!(
        OptimisticMessage::try_from_authority_parts(
            event(1, 2, 10, 0, 0),
            invalid_child,
            MAX_AUTHORITY,
            0,
            OptimisticMessageKind::Positive,
        ),
        Err(OptimisticError::OutputNotStrictlyFuture { .. })
    ));

    // The public checked path accepts the documented maximum ancestry depth,
    // retains the full namespace bits, and cannot construct a depth-129 child.
    let mut deep_key = OptimisticEventOrderKey::try_from_parts(
        Tick::from_ticks(u128::MAX - 128),
        LpId(0),
        LogicalEventId::root(LpId(0), 12),
    )
    .unwrap();
    for depth in 1..=128u32 {
        let child = LogicalEventId::child(&deep_key, depth).unwrap();
        let tick = u128::MAX - 128 + depth as u128;
        let emitter = LpId(depth % 3);
        deep_key = OptimisticEventOrderKey::try_from_parts(Tick::from_ticks(tick), emitter, child)
            .unwrap();
    }
    assert_eq!(deep_key.logical_id().depth(), 128);
    let depth_128 = OptimisticMessage::try_from_authority_parts(
        event(deep_key.source_lp().0, u32::MAX, u128::MAX, 0, 0),
        deep_key.logical_id().clone(),
        MAX_AUTHORITY,
        u64::MAX,
        OptimisticMessageKind::Positive,
    )
    .unwrap();
    assert_eq!(depth_128.logical_id().depth(), 128);
    assert_eq!(depth_128.authority(), MAX_AUTHORITY);
    assert_eq!(depth_128.event().tick.ticks(), u128::MAX);
    assert_eq!(depth_128.event().source_lp, deep_key.source_lp());
    assert_eq!(depth_128.order_key(), deep_key);
    assert!(matches!(
        LogicalEventId::child(&deep_key, 0),
        Err(OptimisticError::CausalDepthExceeded {
            depth: 129,
            limit: 128,
        })
    ));
}

#[test]
fn legacy_emission_and_rollback_antis_remain_local_preview() {
    let mut rt = runtime(models(3));
    let root = rt.schedule_initial(1, event(0, 1, 10, 1, 4)).unwrap();
    assert_eq!(root.authority(), OptimisticAuthority::LocalPreview);

    let initial = drain(&mut rt, 20, 8);
    let child = initial
        .iter()
        .find(|message| {
            message.kind() == OptimisticMessageKind::Positive && message.logical_id().depth() == 1
        })
        .unwrap()
        .clone();
    assert_eq!(child.authority(), OptimisticAuthority::LocalPreview);
    assert_eq!(child.event().source_lp, LpId(1));
    assert_eq!(child.event().dest_lp, LpId(2));

    rt.receive(root.as_anti()).unwrap();
    let retractions = drain(&mut rt, 20, 1);
    let child_anti = retractions
        .iter()
        .find(|message| {
            message.kind() == OptimisticMessageKind::Anti
                && message.logical_id() == child.logical_id()
        })
        .unwrap();
    assert_eq!(child_anti, &child.as_anti());
    assert_eq!(child_anti.authority(), OptimisticAuthority::LocalPreview);
    assert!(retractions
        .iter()
        .all(|message| message.authority() == OptimisticAuthority::LocalPreview));
}

#[derive(Clone, Copy, Debug)]
enum LegacyState {
    Empty,
    Pending,
    Executed,
    Tombstoned,
}

struct RuntimeSnapshot {
    report: OptimisticRuntimeReport,
    queues: Vec<Vec<OptimisticMessage>>,
    processes: Vec<Model>,
    tokens: Vec<OptimisticStateToken>,
}

fn capture(runtime: &OptimisticRuntime<Model>) -> RuntimeSnapshot {
    RuntimeSnapshot {
        report: runtime.report(),
        queues: (0..3)
            .map(|lp| runtime.pending_events(LpId(lp)).unwrap())
            .collect(),
        processes: all_models(runtime, 3),
        tokens: (0..3)
            .map(|lp| runtime.state_token(LpId(lp)).unwrap())
            .collect(),
    }
}

fn assert_snapshot_unchanged(runtime: &OptimisticRuntime<Model>, before: &RuntimeSnapshot) {
    assert_eq!(runtime.report(), before.report);
    assert_eq!(
        (0..3)
            .map(|lp| runtime.pending_events(LpId(lp)).unwrap())
            .collect::<Vec<_>>(),
        before.queues
    );
    assert_eq!(all_models(runtime, 3), before.processes);
    for token in before.tokens.iter().copied() {
        assert!(runtime.validate_state_token(token));
    }
}

fn seed_legacy_state(
    runtime: &mut OptimisticRuntime<Model>,
    message: &OptimisticMessage,
    state: LegacyState,
) {
    match state {
        LegacyState::Empty => {}
        LegacyState::Pending => runtime.receive(message.clone()).unwrap(),
        LegacyState::Executed => {
            runtime.receive(message.clone()).unwrap();
            drain(runtime, 10, 8);
        }
        LegacyState::Tombstoned => {
            runtime.receive(message.as_anti()).unwrap();
            drain(runtime, 10, 8);
        }
    }
}

#[test]
fn scoped_positive_and_anti_reject_unchanged_across_legacy_lifecycle_states() {
    let legacy = OptimisticMessage::try_from_parts(
        event(0, 1, 10, 2, 5),
        LogicalEventId::root(LpId(0), 40),
        4,
        OptimisticMessageKind::Positive,
    )
    .unwrap();
    for state in [
        LegacyState::Empty,
        LegacyState::Pending,
        LegacyState::Executed,
        LegacyState::Tombstoned,
    ] {
        let mut subject = runtime(models(3));
        let mut control = runtime(models(3));
        seed_legacy_state(&mut subject, &legacy, state);
        seed_legacy_state(&mut control, &legacy, state);

        for kind in [OptimisticMessageKind::Positive, OptimisticMessageKind::Anti] {
            let scoped = authority_message(&legacy, MAX_AUTHORITY, kind);
            let before = capture(&subject);
            assert!(matches!(
                subject.receive(scoped),
                Err(OptimisticError::ScopedAuthorityRequiresOwnedRuntime)
            ));
            assert_snapshot_unchanged(&subject, &before);
        }

        // A later ordinary event must still be accepted and reach the same
        // model/RNG state as a control with no scoped attempts.
        let next_event = OptimisticMessage::try_from_parts(
            event(0, 1, 20, 2, 3),
            LogicalEventId::root(LpId(0), 99),
            9,
            OptimisticMessageKind::Positive,
        )
        .unwrap();
        subject.receive(next_event.clone()).unwrap();
        control.receive(next_event).unwrap();
        drain(&mut subject, 30, 1);
        drain(&mut control, 30, 1);
        assert_eq!(all_models(&subject, 3), all_models(&control, 3));
        assert_eq!(subject.report(), control.report());
    }
}

#[test]
fn rejected_scoped_max_incarnation_does_not_advance_legacy_emitter() {
    let mut subject = runtime(models(3));
    let mut control = runtime(models(3));
    let event = event(0, 1, 10, 2, 8);
    let positive = OptimisticMessage::try_from_authority_parts(
        event.clone(),
        LogicalEventId::root(LpId(0), 501),
        MAX_AUTHORITY,
        u64::MAX,
        OptimisticMessageKind::Positive,
    )
    .unwrap();

    for message in [positive.clone(), positive.as_anti()] {
        let before = capture(&subject);
        assert!(matches!(
            subject.receive(message),
            Err(OptimisticError::ScopedAuthorityRequiresOwnedRuntime)
        ));
        assert_snapshot_unchanged(&subject, &before);
    }

    let emitted = subject.schedule_initial(501, event.clone()).unwrap();
    let expected = control.schedule_initial(501, event).unwrap();
    assert_eq!(emitted.incarnation(), 0);
    assert_eq!(emitted, expected);
    assert_eq!(emitted.authority(), OptimisticAuthority::LocalPreview);
    drain(&mut subject, 20, 8);
    drain(&mut control, 20, 8);
    assert_eq!(all_models(&subject, 3), all_models(&control, 3));
    assert_eq!(subject.report(), control.report());
}

#[test]
fn poisoned_runtime_error_precedes_scoped_authority_rejection() {
    let mut states = models(3);
    states.get_mut(&LpId(1)).unwrap().panic_on_event = true;
    let mut rt = runtime(states);
    let legacy = rt.schedule_initial(8, event(0, 1, 10, 0, 1)).unwrap();
    let tokens_before = (0..3)
        .map(|lp| rt.state_token(LpId(lp)).unwrap())
        .collect::<Vec<_>>();
    assert!(matches!(
        rt.run_until_with_budget(SimTime::from_ticks(10), 8),
        Err(OptimisticError::HandlerPanicked(LpId(1)))
    ));
    assert!(tokens_before
        .iter()
        .copied()
        .all(|token| !rt.validate_state_token(token)));

    let scoped = authority_message(&legacy, MAX_AUTHORITY, OptimisticMessageKind::Positive);
    let before_report = rt.report();
    let before_queues = (0..3)
        .map(|lp| rt.pending_events(LpId(lp)).unwrap())
        .collect::<Vec<_>>();
    let before_processes = all_models(&rt, 3);
    assert!(matches!(rt.receive(scoped), Err(OptimisticError::Poisoned)));
    assert_eq!(rt.report(), before_report);
    assert_eq!(
        (0..3)
            .map(|lp| rt.pending_events(LpId(lp)).unwrap())
            .collect::<Vec<_>>(),
        before_queues
    );
    assert_eq!(all_models(&rt, 3), before_processes);
}
