#![cfg(feature = "time-warp")]

use std::collections::BTreeMap;

use kairo_ecs_pdes::{
    LogicalEventId, LpId, OptimisticAuthority, OptimisticError, OptimisticLimits,
    OptimisticMessage, OptimisticMessageKind, OptimisticProcess, OptimisticRuntime,
    OptimisticRuntimeReport, OptimisticStateError, OptimisticStateToken, PartitionPlan,
    RemoteEvent, Tick,
};
use kairo_ecs_types::{EntityId, SimDuration};

#[derive(Clone, Debug, Eq, PartialEq)]
struct ModelState {
    total: u64,
    rng: u64,
    seen: Vec<(u128, Vec<u8>)>,
}

struct Model {
    lp: LpId,
    state: ModelState,
}

impl OptimisticProcess for Model {
    type Snapshot = ModelState;

    fn snapshot(&self) -> Self::Snapshot {
        self.state.clone()
    }

    fn restore(&mut self, snapshot: &Self::Snapshot) -> Result<(), OptimisticStateError> {
        self.state = snapshot.clone();
        Ok(())
    }

    fn on_event(&mut self, event: &RemoteEvent) -> Vec<RemoteEvent> {
        self.state.total += u64::from(event.event_payload.last().copied().unwrap_or_default());
        self.state.rng = self
            .state
            .rng
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1);
        self.state
            .seen
            .push((event.tick.ticks(), event.event_payload.clone()));

        if let [0xEE, destination, payload] = event.event_payload.as_slice() {
            vec![RemoteEvent {
                source_lp: self.lp,
                dest_lp: LpId(u32::from(*destination)),
                tick: Tick::from_ticks(event.tick.ticks() + 1),
                event_payload: vec![*payload],
            }]
        } else {
            Vec::new()
        }
    }
}

fn runtime() -> OptimisticRuntime<Model> {
    let lps = [LpId(0), LpId(1)];
    let partition = PartitionPlan::from_entities(
        2,
        SimDuration::from_ticks(1),
        vec![EntityId::new(1, 0), EntityId::new(2, 0)],
    )
    .unwrap();
    let topology = BTreeMap::from([(LpId(0), vec![LpId(1)]), (LpId(1), vec![LpId(0)])]);
    let processes = lps
        .into_iter()
        .map(|lp| {
            (
                lp,
                Model {
                    lp,
                    state: ModelState {
                        total: 0,
                        rng: 7,
                        seen: Vec::new(),
                    },
                },
            )
        })
        .collect();
    OptimisticRuntime::new(partition, topology, processes, OptimisticLimits::default()).unwrap()
}

fn event(source: u32, destination: u32, tick: u128, payload: &[u8]) -> RemoteEvent {
    RemoteEvent {
        source_lp: LpId(source),
        dest_lp: LpId(destination),
        tick: Tick::from_ticks(tick),
        event_payload: payload.to_vec(),
    }
}

fn local_message(
    event: RemoteEvent,
    sequence: u64,
    incarnation: u64,
    kind: OptimisticMessageKind,
) -> OptimisticMessage {
    OptimisticMessage::try_from_parts(
        event.clone(),
        LogicalEventId::root(event.source_lp, sequence),
        incarnation,
        kind,
    )
    .unwrap()
}

struct RuntimeSnapshot {
    report: OptimisticRuntimeReport,
    pending: [Vec<OptimisticMessage>; 2],
    states: [ModelState; 2],
    tokens: [OptimisticStateToken; 2],
}

fn snapshot_runtime(runtime: &OptimisticRuntime<Model>) -> RuntimeSnapshot {
    RuntimeSnapshot {
        report: runtime.report(),
        pending: [
            runtime.pending_events(LpId(0)).unwrap(),
            runtime.pending_events(LpId(1)).unwrap(),
        ],
        states: [
            runtime.process_at(LpId(0)).unwrap().state.clone(),
            runtime.process_at(LpId(1)).unwrap().state.clone(),
        ],
        tokens: [
            runtime.state_token(LpId(0)).unwrap(),
            runtime.state_token(LpId(1)).unwrap(),
        ],
    }
}

fn assert_runtime_unchanged(runtime: &OptimisticRuntime<Model>, before: &RuntimeSnapshot) {
    assert_eq!(runtime.report(), before.report);
    for (index, lp) in [LpId(0), LpId(1)].into_iter().enumerate() {
        assert_eq!(runtime.pending_events(lp).unwrap(), before.pending[index]);
        assert_eq!(
            &runtime.process_at(lp).unwrap().state,
            &before.states[index]
        );
        assert_eq!(runtime.state_token(lp).unwrap(), before.tokens[index]);
        assert!(runtime.validate_state_token(before.tokens[index]));
    }
}

#[test]
fn authority_roundtrip_preserves_native_ranges_and_does_not_change_ordering() {
    let zero_authority = OptimisticAuthority::Scoped {
        simulation_namespace: 0,
        ownership_epoch: 0,
    };
    let zero_event = event(0, 1, 0, &[0, 255, 0]);
    let zero = OptimisticMessage::try_from_authority_parts(
        zero_event.clone(),
        LogicalEventId::root(LpId(0), 0),
        zero_authority,
        0,
        OptimisticMessageKind::Positive,
    )
    .unwrap();
    assert_eq!(zero.authority(), zero_authority);
    assert_eq!(zero.event(), &zero_event);

    let parent_event = event(0, 1, u128::MAX - 1, &[1, 254, 3]);
    let parent = local_message(
        parent_event,
        u64::MAX,
        u64::MAX,
        OptimisticMessageKind::Positive,
    );
    let logical_id = LogicalEventId::child(&parent.order_key(), u32::MAX).unwrap();
    let output_event = event(1, 0, u128::MAX, &[0, 255, 17, 0]);
    let maximum_authority = OptimisticAuthority::Scoped {
        simulation_namespace: u128::MAX,
        ownership_epoch: u64::MAX,
    };
    let scoped = OptimisticMessage::try_from_authority_parts(
        output_event.clone(),
        logical_id.clone(),
        maximum_authority,
        u64::MAX,
        OptimisticMessageKind::Positive,
    )
    .unwrap();
    let local = OptimisticMessage::try_from_parts(
        output_event.clone(),
        logical_id.clone(),
        u64::MAX,
        OptimisticMessageKind::Positive,
    )
    .unwrap();
    let different_incarnation = OptimisticMessage::try_from_authority_parts(
        output_event.clone(),
        logical_id.clone(),
        OptimisticAuthority::LocalPreview,
        0,
        OptimisticMessageKind::Anti,
    )
    .unwrap();

    assert_eq!(scoped.authority(), maximum_authority);
    assert_eq!(scoped.event(), &output_event);
    assert_eq!(scoped.logical_id(), &logical_id);
    assert_eq!(scoped.incarnation(), u64::MAX);
    assert_eq!(scoped.order_key(), local.order_key());
    assert_eq!(scoped.order_key(), different_incarnation.order_key());
    assert_eq!(local.authority(), OptimisticAuthority::LocalPreview);
    assert_eq!(scoped.clone(), scoped);

    let anti = scoped.as_anti();
    assert_eq!(anti.authority(), maximum_authority);
    assert_eq!(anti.kind(), OptimisticMessageKind::Anti);
    assert_eq!(anti.event(), scoped.event());
    assert_eq!(anti.logical_id(), scoped.logical_id());
    assert_eq!(anti.incarnation(), scoped.incarnation());
    assert_eq!(anti.order_key(), scoped.order_key());

    let bad_time = event(1, 0, u128::MAX - 1, &[1]);
    assert_eq!(
        OptimisticMessage::try_from_authority_parts(
            bad_time,
            logical_id,
            maximum_authority,
            1,
            OptimisticMessageKind::Positive,
        ),
        Err(OptimisticError::OutputNotStrictlyFuture {
            input_tick: Tick::from_ticks(u128::MAX - 1),
            output_tick: Tick::from_ticks(u128::MAX - 1),
        })
    );
}

#[test]
fn legacy_constructors_and_runtime_outputs_remain_local_preview() {
    let mut runtime = runtime();
    let initial = runtime
        .schedule_initial(1, event(0, 1, 1, &[0xEE, 0, 4]))
        .unwrap();
    assert_eq!(initial.authority(), OptimisticAuthority::LocalPreview);
    assert_eq!(
        initial.as_anti().authority(),
        OptimisticAuthority::LocalPreview
    );

    let progress = runtime
        .run_until_with_budget(Tick::from_ticks(3), 4)
        .unwrap();
    assert!(!progress.published_messages.is_empty());
    assert!(progress
        .published_messages
        .iter()
        .all(|message| message.authority() == OptimisticAuthority::LocalPreview));

    let reconstructed = OptimisticMessage::try_from_parts(
        event(1, 0, 4, &[9]),
        LogicalEventId::root(LpId(1), 2),
        0,
        OptimisticMessageKind::Positive,
    )
    .unwrap();
    assert_eq!(reconstructed.authority(), OptimisticAuthority::LocalPreview);
}

#[test]
fn scoped_positive_and_anti_reject_before_any_all_local_runtime_mutation() {
    let mut runtime = runtime();
    let known_event = event(1, 0, 2, &[5]);
    let known = local_message(known_event.clone(), 5, 0, OptimisticMessageKind::Positive);
    runtime.receive(known.clone()).unwrap();
    runtime
        .run_until_with_budget(Tick::from_ticks(2), 1)
        .unwrap();

    let scoped = OptimisticMessage::try_from_authority_parts(
        known_event,
        known.logical_id().clone(),
        OptimisticAuthority::Scoped {
            simulation_namespace: u128::MAX,
            ownership_epoch: u64::MAX,
        },
        known.incarnation(),
        OptimisticMessageKind::Positive,
    )
    .unwrap();
    let before = snapshot_runtime(&runtime);

    for message in [scoped.clone(), scoped.as_anti()] {
        assert_eq!(
            runtime.receive(message),
            Err(OptimisticError::ScopedAuthorityRequiresOwnedRuntime)
        );
        assert_runtime_unchanged(&runtime, &before);
    }

    let valid = local_message(event(1, 0, 3, &[7]), 6, 1, OptimisticMessageKind::Positive);
    runtime.receive(valid).unwrap();
    runtime
        .run_until_with_budget(Tick::from_ticks(3), 1)
        .unwrap();
    assert_eq!(runtime.process_at(LpId(0)).unwrap().state.total, 12);
}

#[test]
fn rejected_scoped_max_incarnation_does_not_advance_legacy_emission_allocation() {
    let mut runtime = runtime();
    let first_input = local_message(
        event(1, 0, 1, &[0xEE, 1, 3]),
        1,
        0,
        OptimisticMessageKind::Positive,
    );
    runtime.receive(first_input).unwrap();
    let first = runtime
        .run_until_with_budget(Tick::from_ticks(3), 3)
        .unwrap();
    let first_output = first
        .published_messages
        .iter()
        .find(|message| message.event().event_payload == [3])
        .unwrap();
    assert_eq!(first_output.incarnation(), 0);
    assert_eq!(first_output.authority(), OptimisticAuthority::LocalPreview);

    let scoped_max = OptimisticMessage::try_from_authority_parts(
        event(0, 1, 4, &[0xEF]),
        LogicalEventId::root(LpId(0), u64::MAX),
        OptimisticAuthority::Scoped {
            simulation_namespace: 9,
            ownership_epoch: u64::MAX,
        },
        u64::MAX,
        OptimisticMessageKind::Positive,
    )
    .unwrap();
    assert_eq!(
        runtime.receive(scoped_max),
        Err(OptimisticError::ScopedAuthorityRequiresOwnedRuntime)
    );

    let second_input = local_message(
        event(1, 0, 5, &[0xEE, 1, 4]),
        2,
        1,
        OptimisticMessageKind::Positive,
    );
    runtime.receive(second_input).unwrap();
    let second = runtime
        .run_until_with_budget(Tick::from_ticks(7), 3)
        .unwrap();
    let second_output = second
        .published_messages
        .iter()
        .find(|message| message.event().event_payload == [4])
        .unwrap();
    assert_eq!(second_output.incarnation(), 1);
    assert_eq!(second_output.authority(), OptimisticAuthority::LocalPreview);
}
