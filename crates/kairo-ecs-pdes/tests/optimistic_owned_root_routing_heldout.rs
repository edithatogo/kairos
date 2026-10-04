#![cfg(feature = "time-warp")]

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Barrier};
use std::thread;
use std::time::Duration;

use kairo_ecs_pdes::{
    LogicalEventId, LpId, NativeAccountingAuthority, NativeAdmissionMembership,
    OptimisticAuthority, OptimisticError, OptimisticLimits, OptimisticMessage,
    OptimisticMessageKind, OptimisticOutboundStatus, OptimisticOwnedOptions, OptimisticProcess,
    OptimisticRuntime, OptimisticRuntimeReport, OptimisticStateError, OptimisticStateToken,
    PartitionPlan, RemoteEvent, Tick,
};
use kairo_ecs_types::{EntityId, SimDuration};

const LP0: LpId = LpId(0);
const LP1: LpId = LpId(1);
const LP2: LpId = LpId(2);
const NAMESPACE: u128 = u128::MAX;
const WIDE_EPOCH: u64 = u64::MAX;

#[derive(Clone, Debug, Eq, PartialEq)]
struct ProbeSnapshot {
    lp: LpId,
    value: u64,
    rng: u64,
    output_log: Vec<u64>,
}

#[derive(Clone)]
struct Probe {
    lp: LpId,
    value: u64,
    rng: u64,
    output_log: Vec<u64>,
    snapshot_calls: Arc<AtomicUsize>,
}

impl Probe {
    fn new(lp: LpId) -> Self {
        Self {
            lp,
            value: 100 + u64::from(lp.0),
            rng: 0x9e37_79b9_7f4a_7c15 ^ u64::from(lp.0),
            output_log: vec![u64::from(lp.0)],
            snapshot_calls: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn state(&self) -> ProbeSnapshot {
        ProbeSnapshot {
            lp: self.lp,
            value: self.value,
            rng: self.rng,
            output_log: self.output_log.clone(),
        }
    }
}

impl OptimisticProcess for Probe {
    type Snapshot = ProbeSnapshot;

    fn snapshot(&self) -> Self::Snapshot {
        self.snapshot_calls.fetch_add(1, Ordering::Relaxed);
        self.state()
    }

    fn restore(&mut self, state: &Self::Snapshot) -> Result<(), OptimisticStateError> {
        self.lp = state.lp;
        self.value = state.value;
        self.rng = state.rng;
        self.output_log.clone_from(&state.output_log);
        Ok(())
    }

    fn on_event(&mut self, event: &RemoteEvent) -> Vec<RemoteEvent> {
        self.value = self.value.wrapping_add(u64::from(
            event.event_payload.first().copied().unwrap_or_default(),
        ));
        self.rng = self
            .rng
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(event.tick.ticks() as u64);
        self.output_log.push(self.value ^ self.rng);
        Vec::new()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Observation {
    report: OptimisticRuntimeReport,
    revision: u64,
    states: BTreeMap<LpId, ProbeSnapshot>,
    pending: BTreeMap<LpId, Option<Vec<OptimisticMessage>>>,
    tokens: BTreeMap<LpId, OptimisticStateToken>,
    sealed: bool,
    inputs_closed: bool,
}

fn partition() -> PartitionPlan {
    let entities = (10..13).map(|id| EntityId::new(id, 0)).collect();
    PartitionPlan::from_entities(3, SimDuration::from_ticks(7), entities).unwrap()
}

fn topology() -> BTreeMap<LpId, Vec<LpId>> {
    BTreeMap::from([
        (LP0, vec![LP1, LP2]),
        (LP1, vec![LP0, LP2]),
        (LP2, vec![LP0, LP1]),
    ])
}

fn authorities(epoch: u64) -> BTreeMap<LpId, OptimisticAuthority> {
    [LP0, LP1, LP2]
        .into_iter()
        .map(|lp| {
            (
                lp,
                OptimisticAuthority::Scoped {
                    simulation_namespace: NAMESPACE,
                    ownership_epoch: epoch,
                },
            )
        })
        .collect()
}

fn options(
    owned: &[LpId],
    epoch: u64,
    max_outbox_entries: usize,
    max_transition_entries: usize,
    max_receipt_entries: usize,
) -> OptimisticOwnedOptions {
    OptimisticOwnedOptions {
        simulation_namespace: NAMESPACE,
        current_authorities: authorities(epoch),
        emission_epochs: owned.iter().map(|lp| (*lp, epoch)).collect(),
        local_limits: OptimisticLimits::default(),
        max_global_lps: 3,
        max_outbox_entries,
        max_transition_entries,
        max_receipt_entries,
    }
}

type RuntimePair = (
    OptimisticRuntime<Probe>,
    OptimisticRuntime<Probe>,
    NativeAccountingAuthority,
    NativeAccountingAuthority,
);

fn runtime(
    owned: &[LpId],
    epoch: u64,
    max_outbox_entries: usize,
    max_transition_entries: usize,
    max_receipt_entries: usize,
) -> OptimisticRuntime<Probe> {
    runtime_with_pending_limit(
        owned,
        epoch,
        max_outbox_entries,
        max_transition_entries,
        max_receipt_entries,
        OptimisticLimits::default().max_pending_events,
    )
}

fn runtime_with_pending_limit(
    owned: &[LpId],
    epoch: u64,
    max_outbox_entries: usize,
    max_transition_entries: usize,
    max_receipt_entries: usize,
    max_pending_events: usize,
) -> OptimisticRuntime<Probe> {
    let processes = owned
        .iter()
        .copied()
        .map(|lp| (lp, Probe::new(lp)))
        .collect();
    let mut runtime_options = options(
        owned,
        epoch,
        max_outbox_entries,
        max_transition_entries,
        max_receipt_entries,
    );
    runtime_options.local_limits.max_pending_events = max_pending_events;
    OptimisticRuntime::new_owned(partition(), topology(), processes, runtime_options).unwrap()
}

fn pair(
    epoch: u64,
    source_transition_limit: usize,
    receiver_transition_limit: usize,
    receipt_limit: usize,
) -> RuntimePair {
    let mut source = runtime(&[LP0], epoch, 8, source_transition_limit, receipt_limit);
    let mut receiver = runtime(
        &[LP1, LP2],
        epoch,
        8,
        receiver_transition_limit,
        receipt_limit,
    );
    let source_authority = source.native_accounting_authority().unwrap();
    let receiver_authority = receiver.native_accounting_authority().unwrap();
    source
        .register_native_peer(receiver_authority.clone())
        .unwrap();
    receiver
        .register_native_peer(source_authority.clone())
        .unwrap();
    source.seal_native_peers().unwrap();
    receiver.seal_native_peers().unwrap();
    (source, receiver, source_authority, receiver_authority)
}

fn pair_with_bounds(
    epoch: u64,
    source_outbox: usize,
    source_receipts: usize,
    receiver_receipts: usize,
    receiver_pending: usize,
) -> RuntimePair {
    let mut source = runtime_with_pending_limit(
        &[LP0],
        epoch,
        source_outbox,
        8,
        source_receipts,
        OptimisticLimits::default().max_pending_events,
    );
    let mut receiver = runtime_with_pending_limit(
        &[LP1, LP2],
        epoch,
        8,
        8,
        receiver_receipts,
        receiver_pending,
    );
    let source_authority = source.native_accounting_authority().unwrap();
    let receiver_authority = receiver.native_accounting_authority().unwrap();
    source
        .register_native_peer(receiver_authority.clone())
        .unwrap();
    receiver
        .register_native_peer(source_authority.clone())
        .unwrap();
    source.seal_native_peers().unwrap();
    receiver.seal_native_peers().unwrap();
    (source, receiver, source_authority, receiver_authority)
}

fn observe(runtime: &OptimisticRuntime<Probe>, owned: &[LpId]) -> Observation {
    Observation {
        report: runtime.report(),
        revision: runtime.accounting_revision().unwrap(),
        states: owned
            .iter()
            .map(|lp| (*lp, runtime.process_at(*lp).unwrap().state()))
            .collect(),
        pending: owned
            .iter()
            .map(|lp| (*lp, runtime.pending_events(*lp)))
            .collect(),
        tokens: owned
            .iter()
            .map(|lp| (*lp, runtime.state_token(*lp).unwrap()))
            .collect(),
        sealed: runtime.native_peers_sealed().unwrap(),
        inputs_closed: runtime.initial_inputs_closed(),
    }
}

fn event(source_lp: LpId, dest_lp: LpId, tick: u128, payload: &[u8]) -> RemoteEvent {
    RemoteEvent {
        source_lp,
        dest_lp,
        tick: Tick::from_ticks(tick),
        event_payload: payload.to_vec(),
    }
}

fn authority_is(authority: OptimisticAuthority, epoch: u64) {
    assert_eq!(
        authority,
        OptimisticAuthority::Scoped {
            simulation_namespace: NAMESPACE,
            ownership_epoch: epoch,
        }
    );
}

#[test]
fn full_width_epoch_tick_and_root_id_survive_real_admission_and_guarded_run() {
    let (mut source, mut receiver, source_issuer, receiver_issuer) = pair(WIDE_EPOCH, 8, 8, 1);
    let root_event = event(LP0, LP1, u128::MAX, &[0, 255, 17, 0, 93]);
    let source_before = observe(&source, &[LP0]);
    let receiver_before = observe(&receiver, &[LP1, LP2]);
    let receiver_before_accounting = receiver.accounting_snapshot().unwrap();
    let source_token = source.state_token(LP0).unwrap();
    let receiver_token = receiver.state_token(LP1).unwrap();

    let returned = source
        .schedule_initial(u64::MAX, root_event.clone())
        .unwrap();
    assert_eq!(returned.event(), &root_event);
    assert_eq!(returned.logical_id().root_parts(), Some((LP0, u64::MAX)));
    assert_eq!(returned.incarnation(), 0);
    assert_eq!(returned.kind(), OptimisticMessageKind::Positive);
    authority_is(returned.authority(), WIDE_EPOCH);
    assert!(source.validate_state_token(source_token));
    assert!(receiver.validate_state_token(receiver_token));
    assert_eq!(
        source.process_at(LP0).unwrap().state(),
        source_before.states[&LP0]
    );
    assert_eq!(
        receiver.process_at(LP1).unwrap().state(),
        receiver_before.states[&LP1]
    );
    assert_eq!(
        receiver.process_at(LP2).unwrap().state(),
        receiver_before.states[&LP2]
    );
    assert_eq!(source.pending_events(LP0), Some(Vec::new()));
    assert_eq!(receiver.pending_events(LP1), Some(Vec::new()));
    assert_eq!(receiver.pending_events(LP2), Some(Vec::new()));

    let snapshot = source.accounting_snapshot().unwrap();
    assert_eq!(snapshot.revision(), source_before.revision + 1);
    assert_eq!(snapshot.local_positive_count(), 0);
    assert_eq!(snapshot.ready_positive_count(), 1);
    assert_eq!(snapshot.reserved_receipt_count(), 1);
    assert_eq!(snapshot.retained_receipt_count(), 0);
    assert_eq!(
        snapshot.outbound_minimum(),
        Some(Tick::from_ticks(u128::MAX))
    );
    assert_eq!(
        snapshot.minimum_obligation_tick(),
        Some(Tick::from_ticks(u128::MAX))
    );
    assert_eq!(
        receiver.accounting_snapshot().unwrap(),
        receiver_before_accounting
    );

    let outbox = source.outbound_pending().unwrap();
    assert_eq!(outbox.len(), 1);
    assert_eq!(outbox[0].status(), OptimisticOutboundStatus::Ready);
    assert_eq!(outbox[0].issuer().runtime_id(), source_issuer.runtime_id());
    assert_eq!(outbox[0].message(), &returned);
    let send = source.ready_native_sends().unwrap().pop().unwrap();
    assert_eq!(send.key().source_lp(), LP0);
    assert_eq!(send.key().logical_id().root_parts(), Some((LP0, u64::MAX)));
    assert_eq!(send.key().incarnation(), 0);
    assert_eq!(send.key().kind(), OptimisticMessageKind::Positive);
    authority_is(send.key().authority(), WIDE_EPOCH);
    assert_eq!(send.message(), &returned);
    assert_eq!(send.issuer().runtime_id(), source_issuer.runtime_id());
    assert_eq!(source.accounting_snapshot().unwrap(), snapshot);

    let raw_before = observe(&receiver, &[LP1, LP2]);
    assert_eq!(
        receiver.receive(returned.clone()),
        Err(OptimisticError::VerifiedNativeAdmissionRequired)
    );
    assert_eq!(
        receiver.receive(returned.as_anti()),
        Err(OptimisticError::VerifiedNativeAdmissionRequired)
    );
    let forged = OptimisticMessage::try_from_authority_parts(
        root_event.clone(),
        LogicalEventId::root(LP0, u64::MAX),
        OptimisticAuthority::Scoped {
            simulation_namespace: NAMESPACE,
            ownership_epoch: WIDE_EPOCH,
        },
        0,
        OptimisticMessageKind::Positive,
    )
    .unwrap();
    assert_eq!(
        receiver.receive(forged),
        Err(OptimisticError::VerifiedNativeAdmissionRequired)
    );
    assert_eq!(observe(&receiver, &[LP1, LP2]), raw_before);

    let receiver_closed_revision = receiver.accounting_revision().unwrap();
    receiver.close_initial_inputs().unwrap();
    assert_eq!(
        receiver.accounting_revision(),
        Ok(receiver_closed_revision + 1)
    );
    let before_admission = observe(&receiver, &[LP1, LP2]);
    let receiver_before_admission_accounting = receiver.accounting_snapshot().unwrap();
    let capability = receiver.admit_native(&send).unwrap();
    assert_eq!(capability.key(), send.key());
    assert_eq!(capability.message(), send.message());
    assert_eq!(capability.sender().runtime_id(), source_issuer.runtime_id());
    assert_eq!(
        capability.receiver().runtime_id(),
        receiver_issuer.runtime_id()
    );
    assert_eq!(
        capability.recorded_membership(),
        NativeAdmissionMembership::Pending
    );
    assert_eq!(
        capability.recorded_revision(),
        before_admission.revision + 1
    );
    assert_eq!(
        receiver.pending_events(LP1).unwrap(),
        vec![returned.clone()]
    );
    assert_eq!(receiver.pending_events(LP2), Some(Vec::new()));
    assert_eq!(
        receiver.process_at(LP1).unwrap().state(),
        before_admission.states[&LP1]
    );
    assert_eq!(
        receiver.process_at(LP2).unwrap().state(),
        before_admission.states[&LP2]
    );
    let admitted_observation = observe(&receiver, &[LP1, LP2]);
    let admitted_accounting = receiver.accounting_snapshot().unwrap();
    assert_eq!(
        admitted_accounting.revision(),
        receiver_before_admission_accounting.revision() + 1
    );
    assert_eq!(admitted_accounting.local_positive_count(), 1);
    assert_eq!(admitted_accounting.retained_receipt_count(), 1);
    assert_eq!(admitted_accounting.reserved_receipt_count(), 0);
    assert!(!receiver.validate_state_token(receiver_token));

    let repeated = receiver.admit_native(&send).unwrap();
    assert_eq!(repeated.key(), capability.key());
    assert_eq!(repeated.message(), capability.message());
    assert_eq!(repeated.recorded_revision(), capability.recorded_revision());
    assert_eq!(
        repeated.recorded_membership(),
        capability.recorded_membership()
    );
    assert_eq!(observe(&receiver, &[LP1, LP2]), admitted_observation);
    assert_eq!(receiver.accounting_snapshot().unwrap(), admitted_accounting);

    let before_guarded_run = observe(&receiver, &[LP1, LP2]);
    assert_eq!(
        receiver.run_until_with_budget(Tick::from_ticks(u128::MAX), 10),
        Err(OptimisticError::OwnedRuntimeJoinIncomplete)
    );
    assert_eq!(observe(&receiver, &[LP1, LP2]), before_guarded_run);
    assert_eq!(
        receiver.fossil_collect(Tick::from_ticks(u128::MAX)),
        Err(OptimisticError::NativeGroupCutRequired)
    );
    assert_eq!(observe(&receiver, &[LP1, LP2]), before_guarded_run);

    let before_source_close = observe(&source, &[LP0]);
    source.close_initial_inputs().unwrap();
    assert_eq!(
        source.accounting_revision(),
        Ok(before_source_close.revision + 1)
    );
    let closed_source = observe(&source, &[LP0]);
    assert_eq!(
        source.schedule_initial(9, event(LP0, LP1, 99, &[4])),
        Err(OptimisticError::InitialSchedulingClosed)
    );
    assert_eq!(observe(&source, &[LP0]), closed_source);

    source
        .acknowledge_native_admission(capability.clone())
        .unwrap();
    let after_ack = observe(&source, &[LP0]);
    let after_ack_accounting = source.accounting_snapshot().unwrap();
    assert_eq!(after_ack.revision, closed_source.revision + 1);
    assert!(source.ready_native_sends().unwrap().is_empty());
    assert!(source.outbound_pending().unwrap().is_empty());
    assert_eq!(after_ack_accounting.reserved_receipt_count(), 0);
    assert_eq!(after_ack_accounting.retained_receipt_count(), 1);
    source.acknowledge_native_admission(capability).unwrap();
    assert_eq!(observe(&source, &[LP0]), after_ack);
    assert_eq!(source.accounting_snapshot().unwrap(), after_ack_accounting);

    // The destination's owned token changed once at admission; the source's
    // token and both models remained untouched by remote publication.
    assert!(source.validate_state_token(source_token));
    assert_eq!(
        source.process_at(LP0).unwrap().state(),
        source_before.states[&LP0]
    );
    assert_eq!(receiver.report().logical_processes, 2);
}

#[test]
fn local_root_is_enqueued_once_and_consumes_no_receipt_slot() {
    let mut local = runtime(&[LP0], 0, 8, 4, 1);
    let local_authority = local.native_accounting_authority().unwrap();
    let mut peer = runtime(&[LP1, LP2], 0, 8, 4, 1);
    let peer_authority = peer.native_accounting_authority().unwrap();
    local.register_native_peer(peer_authority.clone()).unwrap();
    peer.register_native_peer(local_authority).unwrap();
    local.seal_native_peers().unwrap();
    peer.seal_native_peers().unwrap();
    let before = observe(&local, &[LP0]);
    let token = local.state_token(LP0).unwrap();

    let message = local
        .schedule_initial(40, event(LP0, LP0, 3, &[7, 0, 8]))
        .unwrap();
    assert_eq!(message.event(), &event(LP0, LP0, 3, &[7, 0, 8]));
    assert_eq!(message.logical_id().root_parts(), Some((LP0, 40)));
    assert_eq!(local.pending_events(LP0), Some(vec![message.clone()]));
    assert!(local.outbound_pending().unwrap().is_empty());
    assert!(local.ready_native_sends().unwrap().is_empty());
    let accounting = local.accounting_snapshot().unwrap();
    assert_eq!(accounting.revision(), before.revision + 1);
    assert_eq!(accounting.local_positive_count(), 1);
    assert_eq!(accounting.ready_positive_count(), 0);
    assert_eq!(accounting.reserved_receipt_count(), 0);
    assert_eq!(accounting.retained_receipt_count(), 0);
    assert!(!local.validate_state_token(token));
    assert_eq!(peer.report().logical_processes, 2);
}

#[test]
fn actual_ticket_issuer_and_owned_destination_are_checked_before_receipt_cache() {
    let mut source = runtime(&[LP0], 0, 8, 8, 4);
    let mut receiver = runtime(&[LP1], 0, 8, 8, 4);
    let mut wrong_receiver = runtime(&[LP2], 0, 8, 8, 4);
    let source_authority = source.native_accounting_authority().unwrap();
    let receiver_authority = receiver.native_accounting_authority().unwrap();
    let wrong_authority = wrong_receiver.native_accounting_authority().unwrap();
    for runtime in [&mut source, &mut receiver, &mut wrong_receiver] {
        for authority in [&source_authority, &receiver_authority, &wrong_authority] {
            if authority.runtime_id() != runtime.native_accounting_authority().unwrap().runtime_id()
            {
                runtime.register_native_peer(authority.clone()).unwrap();
            }
        }
        runtime.seal_native_peers().unwrap();
    }

    let root = event(LP0, LP1, 19, &[1, 2, 0, 3]);
    source.schedule_initial(77, root.clone()).unwrap();
    let send = source.ready_native_sends().unwrap().pop().unwrap();
    let wrong_before = observe(&wrong_receiver, &[LP2]);
    assert_eq!(
        wrong_receiver.admit_native(&send).unwrap_err(),
        OptimisticError::UnownedLogicalProcess(LP1)
    );
    assert_eq!(observe(&wrong_receiver, &[LP2]), wrong_before);
    assert_eq!(
        wrong_receiver
            .accounting_snapshot()
            .unwrap()
            .retained_receipt_count(),
        0
    );

    let before_admit = observe(&receiver, &[LP1]);
    let capability = receiver.admit_native(&send).unwrap();
    assert_eq!(
        capability.receiver().runtime_id(),
        receiver_authority.runtime_id()
    );
    assert_eq!(
        capability.sender().runtime_id(),
        source_authority.runtime_id()
    );
    assert_eq!(
        receiver.pending_events(LP1),
        Some(vec![send.message().clone()])
    );
    assert_eq!(
        observe(&receiver, &[LP1]).revision,
        before_admit.revision + 1
    );
}

#[test]
fn sender_capabilities_are_root_specific_and_fresh_issuers_cannot_reuse_cached_receipts() {
    let mut source = runtime(&[LP0], 0, 8, 8, 4);
    let mut lookalike = runtime(&[LP0], 0, 8, 8, 4);
    let mut receiver = runtime(&[LP1, LP2], 0, 8, 8, 4);
    let source_authority = source.native_accounting_authority().unwrap();
    let lookalike_authority = lookalike.native_accounting_authority().unwrap();
    let receiver_authority = receiver.native_accounting_authority().unwrap();

    source
        .register_native_peer(receiver_authority.clone())
        .unwrap();
    lookalike
        .register_native_peer(receiver_authority.clone())
        .unwrap();
    receiver
        .register_native_peer(source_authority.clone())
        .unwrap();
    source.seal_native_peers().unwrap();
    lookalike.seal_native_peers().unwrap();
    receiver.seal_native_peers().unwrap();

    let first_event = event(LP0, LP1, 10, &[5, 1, 5]);
    let second_event = event(LP0, LP1, 12, &[8, 2, 8]);
    source.schedule_initial(55, first_event.clone()).unwrap();
    source.schedule_initial(56, second_event).unwrap();
    let source_sends = source.ready_native_sends().unwrap();
    assert_eq!(source_sends.len(), 2);
    let first_send = source_sends[0].clone();
    let second_send = source_sends[1].clone();

    lookalike.schedule_initial(55, first_event).unwrap();
    let lookalike_send = lookalike.ready_native_sends().unwrap().pop().unwrap();
    assert_eq!(lookalike_send.key(), first_send.key());
    assert_eq!(lookalike_send.message(), first_send.message());
    assert_ne!(
        lookalike_send.issuer().runtime_id(),
        first_send.issuer().runtime_id()
    );

    let first_capability = receiver.admit_native(&first_send).unwrap();
    let after_first_admission = observe(&receiver, &[LP1, LP2]);
    let after_first_accounting = receiver.accounting_snapshot().unwrap();
    assert_eq!(after_first_accounting.retained_receipt_count(), 1);
    let cached_first = receiver.admit_native(&first_send).unwrap();
    assert_eq!(cached_first.key(), first_capability.key());
    assert_eq!(observe(&receiver, &[LP1, LP2]), after_first_admission);
    assert_eq!(
        receiver.accounting_snapshot().unwrap(),
        after_first_accounting
    );

    let lookalike_before_ack = observe(&lookalike, &[LP0]);
    assert_eq!(
        lookalike.acknowledge_native_admission(first_capability.clone()),
        Err(OptimisticError::NativeSendIssuerMismatch { source_lp: LP0 })
    );
    assert_eq!(observe(&lookalike, &[LP0]), lookalike_before_ack);
    let receiver_before_lookalike = observe(&receiver, &[LP1, LP2]);
    let lookalike_rejection = receiver.admit_native(&lookalike_send).unwrap_err();
    assert_eq!(
        lookalike_rejection,
        OptimisticError::UnregisteredNativeIssuer {
            runtime_id: lookalike_authority.runtime_id(),
            recovery_generation: 0,
        }
    );
    assert_eq!(observe(&receiver, &[LP1, LP2]), receiver_before_lookalike);
    assert_eq!(
        receiver.accounting_snapshot().unwrap(),
        after_first_accounting
    );

    source
        .acknowledge_native_admission(first_capability.clone())
        .unwrap();
    let source_after_first_ack = source.accounting_snapshot().unwrap();
    assert_eq!(source_after_first_ack.ready_positive_count(), 1);
    assert_eq!(source_after_first_ack.retained_receipt_count(), 1);
    let remaining = source.outbound_pending().unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].key(), second_send.key());
    let after_first_ack = observe(&source, &[LP0]);
    source
        .acknowledge_native_admission(first_capability)
        .unwrap();
    assert_eq!(observe(&source, &[LP0]), after_first_ack);
    assert_eq!(
        source.accounting_snapshot().unwrap(),
        source_after_first_ack
    );

    let second_capability = receiver.admit_native(&second_send).unwrap();
    source
        .acknowledge_native_admission(second_capability.clone())
        .unwrap();
    assert!(source.ready_native_sends().unwrap().is_empty());
    assert_eq!(
        source
            .accounting_snapshot()
            .unwrap()
            .retained_receipt_count(),
        2
    );
}

#[test]
fn transition_capacity_covers_local_and_remote_roots_and_survives_ack() {
    let (mut local_first, _local_peer, _, _) = pair(0, 1, 8, 8);
    let local = event(LP0, LP0, 2, &[11]);
    local_first.schedule_initial(100, local.clone()).unwrap();
    let local_accounting = local_first.accounting_snapshot().unwrap();
    assert_eq!(local_accounting.local_positive_count(), 1);
    assert_eq!(local_accounting.reserved_receipt_count(), 0);
    assert_eq!(local_accounting.retained_receipt_count(), 0);
    let local_full = observe(&local_first, &[LP0]);
    let local_full_accounting = local_first.accounting_snapshot().unwrap();

    assert_eq!(
        local_first.schedule_initial(101, event(LP0, LP1, 3, &[12])),
        Err(OptimisticError::TransitionLimitExceeded { limit: 1 })
    );
    assert_eq!(observe(&local_first, &[LP0]), local_full);
    assert_eq!(
        local_first.accounting_snapshot().unwrap(),
        local_full_accounting
    );
    assert_eq!(
        local_first.schedule_initial(102, event(LP0, LP0, 4, &[13])),
        Err(OptimisticError::TransitionLimitExceeded { limit: 1 })
    );
    assert_eq!(observe(&local_first, &[LP0]), local_full);
    assert_eq!(
        local_first.schedule_initial(100, local.clone()),
        Err(OptimisticError::ConflictingLogicalEvent { source_lp: LP0 })
    );
    assert_eq!(
        local_first.schedule_initial(100, event(LP0, LP1, 2, &[99])),
        Err(OptimisticError::ConflictingLogicalEvent { source_lp: LP0 })
    );
    assert_eq!(observe(&local_first, &[LP0]), local_full);
    assert_eq!(
        local_first.accounting_snapshot().unwrap(),
        local_full_accounting
    );

    let (mut remote_first, mut receiver, _, _) = pair(0, 1, 1, 1);
    let root_event = event(LP0, LP1, 20, &[21, 22]);
    remote_first
        .schedule_initial(200, root_event.clone())
        .unwrap();
    let send = remote_first.ready_native_sends().unwrap().pop().unwrap();
    let sender_accounting = remote_first.accounting_snapshot().unwrap();
    assert_eq!(sender_accounting.ready_positive_count(), 1);
    assert_eq!(sender_accounting.reserved_receipt_count(), 1);
    assert_eq!(sender_accounting.retained_receipt_count(), 0);

    // Consume the receiver's only transition slot locally. Admission of the
    // sender's already-reserved root must use a stable receipt, not a second
    // receiver transition slot.
    receiver
        .schedule_initial(300, event(LP1, LP1, 4, &[31]))
        .unwrap();
    let receiver_before_admission = observe(&receiver, &[LP1, LP2]);
    let receiver_before_accounting = receiver.accounting_snapshot().unwrap();
    assert_eq!(receiver_before_accounting.reserved_receipt_count(), 0);
    assert_eq!(receiver_before_accounting.retained_receipt_count(), 0);
    let capability = receiver.admit_native(&send).unwrap();
    assert_eq!(receiver.pending_events(LP1).unwrap().len(), 2);
    assert_eq!(
        receiver
            .accounting_snapshot()
            .unwrap()
            .retained_receipt_count(),
        1
    );
    let repeated = receiver.admit_native(&send).unwrap();
    assert_eq!(repeated.key(), capability.key());
    assert_eq!(
        observe(&receiver, &[LP1, LP2]).revision,
        receiver_before_admission.revision + 1
    );
    assert_eq!(
        receiver.schedule_initial(301, event(LP1, LP1, 5, &[32])),
        Err(OptimisticError::TransitionLimitExceeded { limit: 1 })
    );
    assert_eq!(receiver.pending_events(LP1).unwrap().len(), 2);

    remote_first
        .acknowledge_native_admission(capability)
        .unwrap();
    let source_after_ack = observe(&remote_first, &[LP0]);
    let source_after_ack_accounting = remote_first.accounting_snapshot().unwrap();
    assert!(remote_first.outbound_pending().unwrap().is_empty());
    assert_eq!(source_after_ack_accounting.ready_positive_count(), 0);
    assert_eq!(source_after_ack_accounting.reserved_receipt_count(), 0);
    assert_eq!(source_after_ack_accounting.retained_receipt_count(), 1);
    assert_eq!(
        remote_first.schedule_initial(201, event(LP0, LP0, 21, &[23])),
        Err(OptimisticError::TransitionLimitExceeded { limit: 1 })
    );
    assert_eq!(observe(&remote_first, &[LP0]), source_after_ack);
    assert_eq!(
        remote_first.accounting_snapshot().unwrap(),
        source_after_ack_accounting
    );
    assert_eq!(
        remote_first.schedule_initial(200, event(LP0, LP0, 20, &[99])),
        Err(OptimisticError::ConflictingLogicalEvent { source_lp: LP0 })
    );
    assert_eq!(observe(&remote_first, &[LP0]), source_after_ack);
}

#[test]
fn outbox_and_sender_receiver_receipt_bounds_preflight_then_exact_retry_at_capacity() {
    let (mut source, mut receiver, _, _) = pair_with_bounds(0, 1, 8, 1, 16);
    let first = source
        .schedule_initial(400, event(LP0, LP1, 30, &[40, 41]))
        .unwrap();
    let first_send = source.ready_native_sends().unwrap().pop().unwrap();
    let source_full = observe(&source, &[LP0]);
    let source_full_accounting = source.accounting_snapshot().unwrap();
    assert_eq!(source_full_accounting.ready_positive_count(), 1);
    assert_eq!(source_full_accounting.reserved_receipt_count(), 1);
    assert_eq!(
        source.schedule_initial(401, event(LP0, LP1, 31, &[42])),
        Err(OptimisticError::OutboxLimitExceeded { limit: 1 })
    );
    assert_eq!(observe(&source, &[LP0]), source_full);
    assert_eq!(
        source.accounting_snapshot().unwrap(),
        source_full_accounting
    );
    assert_eq!(
        source.ready_native_sends().unwrap()[0].key(),
        first_send.key()
    );

    let first_capability = receiver.admit_native(&first_send).unwrap();
    let receiver_full = observe(&receiver, &[LP1, LP2]);
    let receiver_full_accounting = receiver.accounting_snapshot().unwrap();
    assert_eq!(receiver_full_accounting.retained_receipt_count(), 1);
    let repeated = receiver.admit_native(&first_send).unwrap();
    assert_eq!(repeated.key(), first_capability.key());
    assert_eq!(
        repeated.recorded_revision(),
        first_capability.recorded_revision()
    );
    assert_eq!(observe(&receiver, &[LP1, LP2]), receiver_full);
    assert_eq!(
        receiver.accounting_snapshot().unwrap(),
        receiver_full_accounting
    );

    source
        .acknowledge_native_admission(first_capability)
        .unwrap();
    let after_ack = source.accounting_snapshot().unwrap();
    assert_eq!(after_ack.reserved_receipt_count(), 0);
    assert_eq!(after_ack.retained_receipt_count(), 1);
    let second = source
        .schedule_initial(402, event(LP0, LP1, 32, &[43]))
        .unwrap();
    assert_eq!(second.logical_id().root_parts(), Some((LP0, 402)));
    let second_send = source.ready_native_sends().unwrap().pop().unwrap();
    let receiver_before_rejection = observe(&receiver, &[LP1, LP2]);
    let receiver_before_rejection_accounting = receiver.accounting_snapshot().unwrap();
    assert_eq!(
        receiver.admit_native(&second_send).unwrap_err(),
        OptimisticError::ReceiptLimitExceeded { limit: 1 }
    );
    assert_eq!(observe(&receiver, &[LP1, LP2]), receiver_before_rejection);
    assert_eq!(
        receiver.accounting_snapshot().unwrap(),
        receiver_before_rejection_accounting
    );
    let repeated_at_capacity = receiver.admit_native(&first_send).unwrap();
    assert_eq!(repeated_at_capacity.key(), first_send.key());
    assert_eq!(observe(&receiver, &[LP1, LP2]), receiver_before_rejection);
    assert_eq!(
        receiver.accounting_snapshot().unwrap(),
        receiver_before_rejection_accounting
    );
    assert_eq!(first.logical_id().root_parts(), Some((LP0, 400)));

    let (mut receipt_limited, mut receipt_receiver, _, _) = pair_with_bounds(0, 4, 1, 4, 16);
    receipt_limited
        .schedule_initial(410, event(LP0, LP1, 40, &[50]))
        .unwrap();
    let reserved_send = receipt_limited.ready_native_sends().unwrap().pop().unwrap();
    let outstanding_before = observe(&receipt_limited, &[LP0]);
    let outstanding_accounting = receipt_limited.accounting_snapshot().unwrap();
    assert_eq!(
        receipt_limited.schedule_initial(411, event(LP0, LP1, 41, &[51])),
        Err(OptimisticError::ReceiptLimitExceeded { limit: 1 })
    );
    assert_eq!(observe(&receipt_limited, &[LP0]), outstanding_before);
    assert_eq!(
        receipt_limited.accounting_snapshot().unwrap(),
        outstanding_accounting
    );
    let receipt = receipt_receiver.admit_native(&reserved_send).unwrap();
    receipt_limited
        .acknowledge_native_admission(receipt)
        .unwrap();
    let completed_before = observe(&receipt_limited, &[LP0]);
    let completed_accounting = receipt_limited.accounting_snapshot().unwrap();
    assert_eq!(completed_accounting.reserved_receipt_count(), 0);
    assert_eq!(completed_accounting.retained_receipt_count(), 1);
    assert_eq!(
        receipt_limited.schedule_initial(412, event(LP0, LP1, 42, &[52])),
        Err(OptimisticError::ReceiptLimitExceeded { limit: 1 })
    );
    assert_eq!(observe(&receipt_limited, &[LP0]), completed_before);
    assert_eq!(
        receipt_limited.accounting_snapshot().unwrap(),
        completed_accounting
    );
}

#[test]
fn pending_bound_rejects_new_ticket_without_mutation_and_keeps_exact_retry() {
    let (mut source, mut receiver, _, _) = pair_with_bounds(0, 4, 4, 4, 1);
    let first = source
        .schedule_initial(500, event(LP0, LP1, 50, &[60]))
        .unwrap();
    let first_send = source.ready_native_sends().unwrap().pop().unwrap();
    let first_capability = receiver.admit_native(&first_send).unwrap();
    assert_eq!(receiver.pending_events(LP1).unwrap().len(), 1);
    let receiver_full = observe(&receiver, &[LP1, LP2]);
    let receiver_full_accounting = receiver.accounting_snapshot().unwrap();
    assert_eq!(receiver_full_accounting.retained_receipt_count(), 1);
    let repeated = receiver.admit_native(&first_send).unwrap();
    assert_eq!(repeated.key(), first_capability.key());
    assert_eq!(observe(&receiver, &[LP1, LP2]), receiver_full);
    assert_eq!(
        receiver.accounting_snapshot().unwrap(),
        receiver_full_accounting
    );

    source
        .acknowledge_native_admission(first_capability)
        .unwrap();
    source
        .schedule_initial(501, event(LP0, LP1, 51, &[61]))
        .unwrap();
    let second_send = source.ready_native_sends().unwrap().pop().unwrap();
    assert_eq!(
        receiver.admit_native(&second_send).unwrap_err(),
        OptimisticError::PendingLimitExceeded { limit: 1 }
    );
    assert_eq!(observe(&receiver, &[LP1, LP2]), receiver_full);
    assert_eq!(
        receiver.accounting_snapshot().unwrap(),
        receiver_full_accounting
    );
    assert_eq!(first.logical_id().root_parts(), Some((LP0, 500)));
}

#[test]
fn concurrent_drop_and_admission_publish_completely_or_reject_unchanged() {
    let (source, mut receiver, source_authority, _) = pair(0, 8, 8, 4);
    let root_event = event(LP0, LP1, 15, &[41, 0, 42]);
    let mut source = source;
    source.schedule_initial(400, root_event).unwrap();
    let send = source.ready_native_sends().unwrap().pop().unwrap();
    let source_runtime_id = source_authority.runtime_id();
    let before = observe(&receiver, &[LP1, LP2]);
    let start = Arc::new(Barrier::new(3));
    let (result_tx, result_rx) = mpsc::channel();
    let (drop_tx, drop_rx) = mpsc::channel();

    let admission_start = Arc::clone(&start);
    let admission_thread = thread::spawn(move || {
        admission_start.wait();
        let result = receiver.admit_native(&send);
        let after = observe(&receiver, &[LP1, LP2]);
        result_tx.send((receiver, result, after)).unwrap();
    });
    let drop_start = Arc::clone(&start);
    let drop_thread = thread::spawn(move || {
        drop_start.wait();
        drop(source);
        drop_tx.send(()).unwrap();
    });
    start.wait();

    drop_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("issuer drop must complete without recursively blocking on a shared gate");
    let (receiver, admission, after) = result_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("admission must finish after drop releases its exclusive gate");
    drop_thread.join().unwrap();
    admission_thread.join().unwrap();
    assert!(!source_authority.is_live());

    let stale_accounting = receiver.accounting_snapshot();
    assert!(matches!(
        stale_accounting,
        Err(OptimisticError::StaleNativeAccountingAuthority {
            runtime_id,
            recovery_generation: 0,
        }) if runtime_id == source_runtime_id
    ));
    match admission {
        Ok(capability) => {
            assert_eq!(capability.sender().runtime_id(), source_runtime_id);
            assert_eq!(after.revision, before.revision + 1);
            assert_eq!(receiver.pending_events(LP1).unwrap().len(), 1);
            assert_eq!(receiver.pending_events(LP2), Some(Vec::new()));
            assert_eq!(after.states, before.states);
            assert_eq!(
                after.report.logical_processes,
                before.report.logical_processes
            );
            assert_ne!(after.tokens[&LP1], before.tokens[&LP1]);
        }
        Err(OptimisticError::StaleNativeAccountingAuthority {
            runtime_id,
            recovery_generation: 0,
        }) => {
            assert_eq!(runtime_id, source_runtime_id);
            assert_eq!(after, before);
        }
        Err(other) => panic!("unexpected concurrent admission result: {other:?}"),
    }
}
