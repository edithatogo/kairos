#![cfg(feature = "time-warp")]

use std::collections::BTreeMap;

use kairo_ecs_pdes::{
    LogicalEventId, LpId, OptimisticError, OptimisticEventOrderKey, OptimisticMessage,
    OptimisticMessageKind, OptimisticProcess, OptimisticRuntime, OptimisticStateError,
    PartitionPlan, RemoteEvent, Tick,
};
use kairo_ecs_types::{EntityId, SimDuration};

struct Emitter(LpId);

impl OptimisticProcess for Emitter {
    type Snapshot = ();

    fn snapshot(&self) -> Self::Snapshot {}

    fn restore(&mut self, _: &Self::Snapshot) -> Result<(), OptimisticStateError> {
        Ok(())
    }

    fn on_event(&mut self, event: &RemoteEvent) -> Vec<RemoteEvent> {
        let (destination, payload) = match (self.0, event.event_payload.as_slice()) {
            (LpId(1), [1]) => (LpId(2), 2),
            (LpId(2), [2]) => (LpId(0), 3),
            _ => return Vec::new(),
        };
        vec![RemoteEvent {
            source_lp: self.0,
            dest_lp: destination,
            tick: Tick::from_ticks(event.tick.ticks() + 1),
            event_payload: vec![payload],
        }]
    }
}

fn runtime() -> OptimisticRuntime<Emitter> {
    let lps = [LpId(0), LpId(1), LpId(2)];
    let partition = PartitionPlan::from_entities(
        3,
        SimDuration::from_ticks(1),
        vec![
            EntityId::new(1, 0),
            EntityId::new(2, 0),
            EntityId::new(3, 0),
        ],
    )
    .unwrap();
    let topology = BTreeMap::from([
        (LpId(0), vec![LpId(1)]),
        (LpId(1), vec![LpId(2)]),
        (LpId(2), vec![LpId(0)]),
    ]);
    let processes = lps.into_iter().map(|lp| (lp, Emitter(lp))).collect();
    OptimisticRuntime::new(partition, topology, processes, Default::default()).unwrap()
}

fn event(source_lp: LpId, dest_lp: LpId, tick: u128, event_payload: &[u8]) -> RemoteEvent {
    RemoteEvent {
        source_lp,
        dest_lp,
        tick: Tick::from_ticks(tick),
        event_payload: event_payload.to_vec(),
    }
}

fn rebuild_identity(id: &LogicalEventId, remaining_depth: usize) -> LogicalEventId {
    if let Some((parent_key, ordinal)) = id.output_parts() {
        assert!(remaining_depth > 0, "output ancestry must remain bounded");
        let parent_id = rebuild_identity(parent_key.logical_id(), remaining_depth - 1);
        let rebuilt_parent = OptimisticEventOrderKey::try_from_parts(
            parent_key.tick(),
            parent_key.source_lp(),
            parent_id,
        )
        .unwrap();
        LogicalEventId::child(&rebuilt_parent, ordinal).unwrap()
    } else {
        let (source_lp, sequence) = id.root_parts().expect("identity must be a root or output");
        LogicalEventId::root(source_lp, sequence)
    }
}

fn reconstruct(message: &OptimisticMessage, kind: OptimisticMessageKind) -> OptimisticMessage {
    let logical_id = rebuild_identity(message.logical_id(), 128);
    let rebuilt_key = OptimisticEventOrderKey::try_from_parts(
        message.event().tick,
        message.event().source_lp,
        logical_id.clone(),
    )
    .unwrap();
    assert_eq!(rebuilt_key, message.order_key());
    OptimisticMessage::try_from_parts(
        message.event().clone(),
        logical_id,
        message.incarnation(),
        kind,
    )
    .unwrap()
}

#[test]
fn root_and_output_parts_roundtrip_full_native_ranges_and_message_kinds() {
    let root_id = LogicalEventId::root(LpId(u32::MAX), u64::MAX);
    assert_eq!(root_id.root_parts(), Some((LpId(u32::MAX), u64::MAX)));
    assert_eq!(root_id.output_parts(), None);
    let root = OptimisticMessage::try_from_parts(
        event(
            LpId(u32::MAX),
            LpId(u32::MAX),
            u128::MAX - 1,
            &[0, 255, 17, 0],
        ),
        root_id,
        u64::MAX,
        OptimisticMessageKind::Positive,
    )
    .unwrap();
    assert_eq!(root.event().tick.ticks(), u128::MAX - 1);
    assert_eq!(root.event().event_payload, [0, 255, 17, 0]);
    assert_eq!(root.incarnation(), u64::MAX);
    assert_eq!(
        root.order_key().logical_id().root_parts(),
        Some((LpId(u32::MAX), u64::MAX))
    );

    let output_id = LogicalEventId::child(&root.order_key(), u32::MAX).unwrap();
    let output = OptimisticMessage::try_from_parts(
        event(LpId(7), LpId(u32::MAX), u128::MAX, &[255, 0, 128, 1]),
        output_id,
        0,
        OptimisticMessageKind::Anti,
    )
    .unwrap();
    assert_eq!(output.event().source_lp, LpId(7));
    assert_eq!(output.event().tick.ticks(), u128::MAX);
    assert_eq!(output.event().event_payload, [255, 0, 128, 1]);
    assert_eq!(output.incarnation(), 0);
    assert_eq!(output.kind(), OptimisticMessageKind::Anti);
    let (parent, ordinal) = output.logical_id().output_parts().unwrap();
    assert_eq!(parent, &root.order_key());
    assert_eq!(ordinal, u32::MAX);
    assert_eq!(
        output.order_key().logical_id().output_parts(),
        Some((parent, ordinal))
    );

    let positive = OptimisticMessage::try_from_parts(
        output.event().clone(),
        output.logical_id().clone(),
        output.incarnation(),
        OptimisticMessageKind::Positive,
    )
    .unwrap();
    assert_eq!(positive.order_key(), output.order_key());
    assert_ne!(positive.kind(), output.kind());
    let different_incarnation = OptimisticMessage::try_from_parts(
        output.event().clone(),
        output.logical_id().clone(),
        u64::MAX,
        OptimisticMessageKind::Positive,
    )
    .unwrap();
    assert_eq!(different_incarnation.order_key(), output.order_key());
}

#[test]
fn actual_multi_emitter_outputs_and_their_antis_roundtrip_and_receive() {
    let mut source = runtime();
    let root = source
        .schedule_initial(0, event(LpId(0), LpId(1), 1, &[1]))
        .unwrap();
    let progress = source
        .run_until_with_budget(Tick::from_ticks(10), 2)
        .unwrap();
    let child = progress
        .published_messages
        .iter()
        .find(|message| message.event().event_payload == [2])
        .unwrap();
    let grandchild = progress
        .published_messages
        .iter()
        .find(|message| message.event().event_payload == [3])
        .unwrap();

    assert_eq!(child.event().source_lp, LpId(1));
    assert_eq!(grandchild.event().source_lp, LpId(2));
    let (child_parent, child_ordinal) = child.logical_id().output_parts().unwrap();
    assert_eq!(child_parent, &root.order_key());
    assert_eq!(child_ordinal, 0);
    let (grandchild_parent, grandchild_ordinal) = grandchild.logical_id().output_parts().unwrap();
    assert_eq!(grandchild_parent, &child.order_key());
    assert_eq!(grandchild_ordinal, 0);

    let rebuilt_child = reconstruct(child, OptimisticMessageKind::Positive);
    let rebuilt_anti = reconstruct(&child.as_anti(), OptimisticMessageKind::Anti);
    let rebuilt_grandchild = reconstruct(grandchild, OptimisticMessageKind::Positive);
    assert_eq!(rebuilt_child, *child);
    assert_eq!(rebuilt_anti, child.as_anti());
    assert_eq!(rebuilt_grandchild, *grandchild);
    assert_eq!(rebuilt_child.order_key(), rebuilt_anti.order_key());

    let mut positive_receiver = runtime();
    positive_receiver.receive(rebuilt_child.clone()).unwrap();
    let accepted = positive_receiver.report();
    assert_eq!(accepted.pending_positives, 1);
    assert_eq!(
        positive_receiver.receive(rebuilt_child.clone()),
        Err(OptimisticError::DuplicatePositive {
            source_lp: LpId(1),
            incarnation: rebuilt_child.incarnation(),
        })
    );
    assert_eq!(positive_receiver.report(), accepted);

    let mut conflicting_event = rebuilt_child.event().clone();
    conflicting_event.event_payload.push(0);
    let conflict = OptimisticMessage::try_from_parts(
        conflicting_event,
        rebuilt_child.logical_id().clone(),
        rebuilt_child.incarnation(),
        OptimisticMessageKind::Positive,
    )
    .unwrap();
    assert!(matches!(
        positive_receiver.receive(conflict),
        Err(OptimisticError::ConflictingDelivery { .. })
    ));
    assert_eq!(positive_receiver.report(), accepted);

    let mut anti_receiver = runtime();
    anti_receiver.receive(rebuilt_anti).unwrap();
    assert_eq!(anti_receiver.report().pending_antis, 1);
    anti_receiver.receive(rebuilt_child).unwrap();
    let canceled = anti_receiver
        .run_until_with_budget(Tick::from_ticks(10), 1)
        .unwrap();
    assert_eq!(canceled.pending_positives, 0);
    assert_eq!(canceled.pending_antis, 0);
    assert_eq!(anti_receiver.report().tombstones, 1);
}

#[test]
fn complete_ancestry_rejects_root_source_and_nonfuture_output_keys() {
    let root = LogicalEventId::root(LpId(1), 4);
    assert_eq!(
        OptimisticEventOrderKey::try_from_parts(Tick::from_ticks(1), LpId(2), root.clone()),
        Err(OptimisticError::EnvelopeSourceMismatch {
            declared: LpId(1),
            actual: LpId(2),
        })
    );
    assert_eq!(
        OptimisticMessage::try_from_parts(
            event(LpId(2), LpId(2), 1, &[9]),
            root,
            0,
            OptimisticMessageKind::Positive,
        ),
        Err(OptimisticError::EnvelopeSourceMismatch {
            declared: LpId(1),
            actual: LpId(2),
        })
    );

    let parent = OptimisticEventOrderKey::try_from_parts(
        Tick::from_ticks(10),
        LpId(1),
        LogicalEventId::root(LpId(1), 5),
    )
    .unwrap();
    let output_id = LogicalEventId::child(&parent, 7).unwrap();
    for output_tick in [9, 10] {
        assert_eq!(
            OptimisticEventOrderKey::try_from_parts(
                Tick::from_ticks(output_tick),
                LpId(2),
                output_id.clone(),
            ),
            Err(OptimisticError::OutputNotStrictlyFuture {
                input_tick: Tick::from_ticks(10),
                output_tick: Tick::from_ticks(output_tick),
            })
        );
    }

    let maximum_parent = OptimisticEventOrderKey::try_from_parts(
        Tick::from_ticks(u128::MAX),
        LpId(1),
        LogicalEventId::root(LpId(1), 6),
    )
    .unwrap();
    let impossible_child = LogicalEventId::child(&maximum_parent, 0).unwrap();
    assert_eq!(
        OptimisticEventOrderKey::try_from_parts(
            Tick::from_ticks(u128::MAX),
            LpId(2),
            impossible_child,
        ),
        Err(OptimisticError::OutputNotStrictlyFuture {
            input_tick: Tick::from_ticks(u128::MAX),
            output_tick: Tick::from_ticks(u128::MAX),
        })
    );
}

#[test]
fn bounded_ancestry_accepts_depth_128_and_rejects_a_129th_child() {
    let mut key = OptimisticEventOrderKey::try_from_parts(
        Tick::from_ticks(0),
        LpId(0),
        LogicalEventId::root(LpId(0), u64::MAX),
    )
    .unwrap();
    for depth in 1..=128u32 {
        let child = LogicalEventId::child(&key, depth).unwrap();
        key = OptimisticEventOrderKey::try_from_parts(
            Tick::from_ticks(u128::from(depth)),
            LpId(depth % 3),
            child,
        )
        .unwrap();
    }
    assert_eq!(key.logical_id().depth(), 128);
    assert!(matches!(
        LogicalEventId::child(&key, 129),
        Err(OptimisticError::CausalDepthExceeded {
            depth: 129,
            limit: 128
        })
    ));
}
