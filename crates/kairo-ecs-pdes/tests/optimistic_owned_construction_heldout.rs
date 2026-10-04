use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use kairo_ecs_pdes::{
    LogicalEventId, LpId, OptimisticAuthority, OptimisticError, OptimisticLimits,
    OptimisticMessage, OptimisticMessageKind, OptimisticOwnedOptions, OptimisticProcess,
    OptimisticRuntime, OptimisticRuntimeReport, OptimisticStateError, OptimisticStateToken,
    PartitionPlan, RemoteEvent, Tick,
};
use kairo_ecs_types::{EntityId, SimDuration};

const LP0: LpId = LpId(0);
const LP1: LpId = LpId(1);
const LP2: LpId = LpId(2);
const LP3: LpId = LpId(3);
const NAMESPACE: u128 = u128::MAX;

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
            .wrapping_add(u64::from(event.tick.ticks() as u8));
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

fn entities(offset: u64) -> Vec<EntityId> {
    (0u64..3)
        .map(|index| EntityId::new(offset + index, 0))
        .collect()
}

fn partition() -> PartitionPlan {
    PartitionPlan::from_entities(3, SimDuration::from_ticks(7), entities(10)).unwrap()
}

fn topology() -> BTreeMap<LpId, Vec<LpId>> {
    BTreeMap::from([
        (LP0, vec![LP2, LP1]),
        (LP1, vec![LP2, LP0]),
        (LP2, vec![LP1, LP0]),
    ])
}

fn epochs() -> BTreeMap<LpId, u64> {
    BTreeMap::from([(LP0, 0), (LP1, 1), (LP2, u64::MAX)])
}

fn authorities(namespace: u128) -> BTreeMap<LpId, OptimisticAuthority> {
    epochs()
        .into_iter()
        .map(|(lp, epoch)| {
            (
                lp,
                OptimisticAuthority::Scoped {
                    simulation_namespace: namespace,
                    ownership_epoch: epoch,
                },
            )
        })
        .collect()
}

fn options_for(owned: &[LpId], namespace: u128) -> OptimisticOwnedOptions {
    let all_epochs = epochs();
    OptimisticOwnedOptions {
        simulation_namespace: namespace,
        current_authorities: authorities(namespace),
        emission_epochs: owned
            .iter()
            .map(|lp| (*lp, all_epochs.get(lp).copied().unwrap_or_default()))
            .collect(),
        local_limits: OptimisticLimits::default(),
        max_global_lps: 3,
        max_outbox_entries: 8,
        max_transition_entries: 8,
        max_receipt_entries: 8,
    }
}

fn processes(
    owned: &[LpId],
) -> (
    BTreeMap<LpId, Probe>,
    BTreeMap<LpId, Arc<AtomicUsize>>,
    BTreeMap<LpId, ProbeSnapshot>,
) {
    let mut values = BTreeMap::new();
    let mut calls = BTreeMap::new();
    let mut witnesses = BTreeMap::new();
    for &lp in owned {
        let process = Probe::new(lp);
        calls.insert(lp, Arc::clone(&process.snapshot_calls));
        witnesses.insert(lp, process.state());
        values.insert(lp, process);
    }
    (values, calls, witnesses)
}

fn make_runtime(
    owned: &[LpId],
    partition: PartitionPlan,
    topology: BTreeMap<LpId, Vec<LpId>>,
    options: OptimisticOwnedOptions,
) -> (
    OptimisticRuntime<Probe>,
    BTreeMap<LpId, Arc<AtomicUsize>>,
    BTreeMap<LpId, ProbeSnapshot>,
) {
    let (processes, calls, witnesses) = processes(owned);
    let runtime = OptimisticRuntime::new_owned(partition, topology, processes, options).unwrap();
    for calls_for_lp in calls.values() {
        assert_eq!(calls_for_lp.load(Ordering::Relaxed), 1);
    }
    (runtime, calls, witnesses)
}

fn runtime(owned: &[LpId]) -> (OptimisticRuntime<Probe>, BTreeMap<LpId, ProbeSnapshot>) {
    runtime_with_namespace(owned, NAMESPACE)
}

fn runtime_with_namespace(
    owned: &[LpId],
    namespace: u128,
) -> (OptimisticRuntime<Probe>, BTreeMap<LpId, ProbeSnapshot>) {
    let (runtime, _, witnesses) = make_runtime(
        owned,
        partition(),
        topology(),
        options_for(owned, namespace),
    );
    (runtime, witnesses)
}

fn assert_new_error(
    owned: &[LpId],
    partition: PartitionPlan,
    topology: BTreeMap<LpId, Vec<LpId>>,
    options: OptimisticOwnedOptions,
    expected: OptimisticError,
) {
    let (processes, calls, _) = processes(owned);
    let result = OptimisticRuntime::new_owned(partition, topology, processes, options);
    match result {
        Err(actual) => assert_eq!(actual, expected),
        Ok(_) => panic!("invalid owned-runtime configuration was accepted"),
    }
    // A rejected configuration must not invoke model snapshot code.
    for snapshot_calls in calls.values() {
        assert_eq!(snapshot_calls.load(Ordering::Relaxed), 0);
    }
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

fn event(source_lp: LpId, dest_lp: LpId, tick: u128) -> RemoteEvent {
    RemoteEvent {
        source_lp,
        dest_lp,
        tick: Tick::from_ticks(tick),
        event_payload: vec![5, 9, 1],
    }
}

fn assert_stale(error: OptimisticError, id: u64) {
    assert_eq!(
        error,
        OptimisticError::StaleNativeAccountingAuthority {
            runtime_id: id,
            recovery_generation: 0,
        }
    );
}

#[test]
fn disjoint_owners_seal_then_guard_mutations_without_remote_mirrors() {
    let (mut first, first_witnesses) = runtime(&[LP0]);
    let (mut rest, rest_witnesses) = runtime(&[LP1, LP2]);
    let first_authority = first.native_accounting_authority().unwrap();
    let rest_authority = rest.native_accounting_authority().unwrap();

    assert_eq!(first.report().logical_processes, 1);
    assert_eq!(rest.report().logical_processes, 2);
    assert!(first.process_at(LP1).is_none());
    assert_eq!(first.pending_events(LP1), None);
    assert_eq!(
        first.state_token(LP1),
        Err(OptimisticError::UnownedLogicalProcess(LP1))
    );
    assert!(rest.process_at(LP0).is_none());
    assert_eq!(
        rest.state_token(LP0),
        Err(OptimisticError::UnownedLogicalProcess(LP0))
    );
    assert_eq!(first_authority.simulation_namespace(), u128::MAX);
    assert_eq!(first_authority.owned_lps(), &[LP0]);
    assert_eq!(first_authority.global_partition(), &partition());
    assert_eq!(
        first_authority.global_topology(),
        &BTreeMap::from([
            (LP0, vec![LP1, LP2]),
            (LP1, vec![LP0, LP2]),
            (LP2, vec![LP0, LP1]),
        ])
    );
    assert_eq!(
        first_authority.current_authorities(),
        &authorities(NAMESPACE)
    );
    assert_eq!(
        first_authority.emission_epochs(),
        &BTreeMap::from([(LP0, 0)])
    );
    assert_eq!(
        rest_authority.emission_epochs(),
        &BTreeMap::from([(LP1, 1), (LP2, u64::MAX)])
    );
    assert_eq!(first.accounting_revision(), Ok(0));
    assert_eq!(rest.accounting_revision(), Ok(0));
    assert_eq!(
        first.process_at(LP0).unwrap().state(),
        first_witnesses[&LP0]
    );
    assert_eq!(rest.process_at(LP1).unwrap().state(), rest_witnesses[&LP1]);
    assert_eq!(rest.process_at(LP2).unwrap().state(), rest_witnesses[&LP2]);

    let token = first.state_token(LP0).unwrap();
    let before = observe(&first, &[LP0]);
    assert_eq!(
        first.schedule_initial(1, event(LP0, LP1, 10)),
        Err(OptimisticError::NativePeersNotSealed)
    );
    assert_eq!(
        first.close_initial_inputs(),
        Err(OptimisticError::NativePeersNotSealed)
    );
    assert_eq!(
        first.run_until_with_budget(Tick::from_ticks(10), 0),
        Err(OptimisticError::NativePeersNotSealed)
    );
    assert_eq!(observe(&first, &[LP0]), before);
    assert!(first.validate_state_token(token));

    assert_eq!(
        first.seal_native_peers(),
        Err(OptimisticError::NativePeerCoverageIncomplete {
            missing: vec![LP1, LP2],
        })
    );
    assert_eq!(first.accounting_revision(), Ok(0));
    first.register_native_peer(rest_authority.clone()).unwrap();
    rest.register_native_peer(first_authority.clone()).unwrap();
    assert_eq!(first.accounting_revision(), Ok(1));
    assert_eq!(rest.accounting_revision(), Ok(1));
    first.seal_native_peers().unwrap();
    rest.seal_native_peers().unwrap();
    assert_eq!(first.accounting_revision(), Ok(2));
    assert_eq!(rest.accounting_revision(), Ok(2));
    assert!(first.validate_state_token(token));

    let before_sealed_guard = observe(&first, &[LP0]);
    assert_eq!(
        first.schedule_initial(2, event(LP0, LP1, 10)),
        Err(OptimisticError::OwnedRuntimeJoinIncomplete)
    );
    assert_eq!(
        first.run_until_with_budget(Tick::from_ticks(10), 0),
        Err(OptimisticError::OwnedRuntimeJoinIncomplete)
    );
    // A rejected call must not store the horizon, even at budget zero.
    assert_eq!(
        first.run_until_with_budget(Tick::from_ticks(5), 0),
        Err(OptimisticError::OwnedRuntimeJoinIncomplete)
    );
    assert_eq!(
        first.schedule_initial(4, event(LP1, LP0, 11)),
        Err(OptimisticError::UnownedLogicalProcess(LP1))
    );
    assert_eq!(observe(&first, &[LP0]), before_sealed_guard);

    first.close_initial_inputs().unwrap();
    assert!(first.initial_inputs_closed());
    assert_eq!(first.accounting_revision(), Ok(3));
    first.close_initial_inputs().unwrap();
    assert_eq!(first.accounting_revision(), Ok(3));
    assert!(first.validate_state_token(token));
    assert_eq!(
        first.schedule_initial(3, event(LP0, LP1, 10)),
        Err(OptimisticError::InitialSchedulingClosed)
    );
    assert_eq!(first.report().logical_processes, 1);
}

#[test]
fn invalid_constructor_maps_topology_and_capacities_fail_before_snapshots() {
    let owned = [LP0];
    let base_partition = partition();
    let base_topology = topology();

    assert_new_error(
        &[],
        base_partition.clone(),
        base_topology.clone(),
        options_for(&[], NAMESPACE),
        OptimisticError::EmptyOwnedProcessSet,
    );
    assert_new_error(
        &[LP3],
        base_partition.clone(),
        base_topology.clone(),
        options_for(&[LP3], NAMESPACE),
        OptimisticError::ProcessSetMismatch {
            missing: vec![],
            unexpected: vec![LP3],
        },
    );

    let mut opts = options_for(&owned, NAMESPACE);
    opts.max_global_lps = 2;
    assert_new_error(
        &owned,
        base_partition.clone(),
        base_topology.clone(),
        opts,
        OptimisticError::GlobalLpLimitExceeded {
            actual: 3,
            limit: 2,
        },
    );

    for field in 0..4 {
        let mut opts = options_for(&owned, NAMESPACE);
        match field {
            0 => opts.max_global_lps = 0,
            1 => opts.max_outbox_entries = 0,
            2 => opts.max_transition_entries = 0,
            _ => opts.max_receipt_entries = 0,
        }
        assert_new_error(
            &owned,
            base_partition.clone(),
            base_topology.clone(),
            opts,
            OptimisticError::InvalidLimits,
        );
    }
    let mut opts = options_for(&owned, NAMESPACE);
    opts.local_limits.max_lps = 0;
    assert_new_error(
        &owned,
        base_partition.clone(),
        base_topology.clone(),
        opts,
        OptimisticError::InvalidLimits,
    );
    let two_owned = [LP0, LP1];
    let mut opts = options_for(&two_owned, NAMESPACE);
    opts.local_limits.max_lps = 1;
    assert_new_error(
        &two_owned,
        base_partition.clone(),
        base_topology.clone(),
        opts,
        OptimisticError::InvalidLimits,
    );

    let mut opts = options_for(&owned, NAMESPACE);
    opts.current_authorities.remove(&LP1);
    assert_new_error(
        &owned,
        base_partition.clone(),
        base_topology.clone(),
        opts,
        OptimisticError::AuthoritySetMismatch {
            missing: vec![LP1],
            unexpected: vec![],
        },
    );
    let mut opts = options_for(&owned, NAMESPACE);
    opts.current_authorities.insert(
        LP3,
        OptimisticAuthority::Scoped {
            simulation_namespace: NAMESPACE,
            ownership_epoch: 17,
        },
    );
    assert_new_error(
        &owned,
        base_partition.clone(),
        base_topology.clone(),
        opts,
        OptimisticError::AuthoritySetMismatch {
            missing: vec![],
            unexpected: vec![LP3],
        },
    );
    let mut opts = options_for(&owned, NAMESPACE);
    opts.current_authorities
        .insert(LP1, OptimisticAuthority::LocalPreview);
    assert_new_error(
        &owned,
        base_partition.clone(),
        base_topology.clone(),
        opts,
        OptimisticError::AuthorityModeMismatch(LP1),
    );
    let mut opts = options_for(&owned, NAMESPACE);
    opts.current_authorities.insert(
        LP2,
        OptimisticAuthority::Scoped {
            simulation_namespace: 7,
            ownership_epoch: u64::MAX,
        },
    );
    assert_new_error(
        &owned,
        base_partition.clone(),
        base_topology.clone(),
        opts,
        OptimisticError::AuthorityNamespaceMismatch {
            lp_id: LP2,
            expected: NAMESPACE,
            actual: 7,
        },
    );
    let mut opts = options_for(&owned, NAMESPACE);
    opts.emission_epochs.clear();
    assert_new_error(
        &owned,
        base_partition.clone(),
        base_topology.clone(),
        opts,
        OptimisticError::EmissionEpochSetMismatch {
            missing: vec![LP0],
            unexpected: vec![],
        },
    );
    let mut opts = options_for(&owned, NAMESPACE);
    opts.emission_epochs.insert(LP1, 1);
    assert_new_error(
        &owned,
        base_partition.clone(),
        base_topology.clone(),
        opts,
        OptimisticError::EmissionEpochSetMismatch {
            missing: vec![],
            unexpected: vec![LP1],
        },
    );
    let mut opts = options_for(&owned, NAMESPACE);
    opts.emission_epochs.insert(LP0, 8);
    assert_new_error(
        &owned,
        base_partition.clone(),
        base_topology.clone(),
        opts,
        OptimisticError::EmissionEpochMismatch {
            lp_id: LP0,
            expected: 0,
            actual: 8,
        },
    );

    let mut missing_topology = base_topology.clone();
    missing_topology.remove(&LP2);
    assert_new_error(
        &owned,
        base_partition.clone(),
        missing_topology,
        options_for(&owned, NAMESPACE),
        OptimisticError::MissingTopologyEntry(LP2),
    );
    let mut unknown_source = base_topology.clone();
    unknown_source.insert(LP3, vec![]);
    assert_new_error(
        &owned,
        base_partition.clone(),
        unknown_source,
        options_for(&owned, NAMESPACE),
        OptimisticError::UnknownTopologySource(LP3),
    );
    let mut unknown_destination = base_topology.clone();
    unknown_destination.insert(LP1, vec![LP3]);
    assert_new_error(
        &owned,
        base_partition.clone(),
        unknown_destination,
        options_for(&owned, NAMESPACE),
        OptimisticError::UnknownTopologyDestination {
            source: LP1,
            destination: LP3,
        },
    );
    let mut duplicate = base_topology.clone();
    duplicate.insert(LP0, vec![LP1, LP1]);
    assert_new_error(
        &owned,
        base_partition.clone(),
        duplicate,
        options_for(&owned, NAMESPACE),
        OptimisticError::DuplicateNeighbor {
            source: LP0,
            destination: LP1,
        },
    );
    let mut self_loop = base_topology.clone();
    self_loop.insert(LP0, vec![LP0]);
    assert_new_error(
        &owned,
        base_partition,
        self_loop,
        options_for(&owned, NAMESPACE),
        OptimisticError::DuplicateNeighbor {
            source: LP0,
            destination: LP0,
        },
    );
}

#[test]
fn peer_identity_uses_full_partition_and_canonical_topology_not_owner_subset() {
    let reversed = BTreeMap::from([
        (LP0, vec![LP1, LP2]),
        (LP1, vec![LP0, LP2]),
        (LP2, vec![LP0, LP1]),
    ]);
    let (mut first, _, _) = make_runtime(
        &[LP0],
        partition(),
        topology(),
        options_for(&[LP0], NAMESPACE),
    );
    let mut peer_options = options_for(&[LP1], NAMESPACE);
    peer_options.local_limits.max_lps = 4;
    peer_options.local_limits.max_pending_events = 37;
    peer_options.local_limits.max_history_events = 41;
    let (second, _, _) = make_runtime(&[LP1], partition(), reversed.clone(), peer_options);
    let first_authority = first.native_accounting_authority().unwrap();
    let second_authority = second.native_accounting_authority().unwrap();
    assert_ne!(first_authority.runtime_id(), second_authority.runtime_id());
    assert_eq!(first_authority.recovery_generation(), 0);
    assert_eq!(second_authority.recovery_generation(), 0);
    assert_eq!(first_authority.global_topology(), &reversed);
    assert_eq!(
        first_authority.current_authorities(),
        second_authority.current_authorities()
    );
    assert_eq!(
        first_authority.global_partition(),
        second_authority.global_partition()
    );
    assert_eq!(
        first_authority.emission_epochs(),
        &BTreeMap::from([(LP0, 0)])
    );
    assert_eq!(
        second_authority.emission_epochs(),
        &BTreeMap::from([(LP1, 1)])
    );

    first.register_native_peer(second_authority).unwrap();
    assert_eq!(first.accounting_revision(), Ok(1));

    let mut different_membership = entities(20);
    different_membership.reverse();
    let partition_membership =
        PartitionPlan::from_entities(3, SimDuration::from_ticks(7), different_membership).unwrap();
    let (membership_peer, _, _) = make_runtime(
        &[LP1],
        partition_membership,
        reversed.clone(),
        options_for(&[LP1], NAMESPACE),
    );
    let before = observe(&first, &[LP0]);
    assert_eq!(
        first.register_native_peer(membership_peer.native_accounting_authority().unwrap()),
        Err(OptimisticError::NativePeerConfigurationMismatch)
    );
    assert_eq!(observe(&first, &[LP0]), before);

    let other_lookahead =
        PartitionPlan::from_entities(3, SimDuration::from_ticks(8), entities(10)).unwrap();
    let (lookahead_peer, _, _) = make_runtime(
        &[LP1],
        other_lookahead,
        reversed.clone(),
        options_for(&[LP1], NAMESPACE),
    );
    let before = observe(&first, &[LP0]);
    assert_eq!(
        first.register_native_peer(lookahead_peer.native_accounting_authority().unwrap()),
        Err(OptimisticError::NativePeerConfigurationMismatch)
    );
    assert_eq!(observe(&first, &[LP0]), before);

    let mut different_route = reversed;
    different_route.insert(LP2, vec![LP0]);
    let (topology_peer, _, _) = make_runtime(
        &[LP1],
        partition(),
        different_route,
        options_for(&[LP1], NAMESPACE),
    );
    let before = observe(&first, &[LP0]);
    assert_eq!(
        first.register_native_peer(topology_peer.native_accounting_authority().unwrap()),
        Err(OptimisticError::NativePeerConfigurationMismatch)
    );
    assert_eq!(observe(&first, &[LP0]), before);

    let (zero_namespace, _) = runtime_with_namespace(&[LP2], 0);
    assert_eq!(
        zero_namespace
            .native_accounting_authority()
            .unwrap()
            .simulation_namespace(),
        0
    );
}

#[test]
fn issuer_retry_overlap_and_peer_coverage_are_revision_checked() {
    let (mut owner0, _) = runtime(&[LP0, LP2]);
    let (owner0_duplicate, _) = runtime(&[LP0, LP2]);
    let (owner1, _) = runtime(&[LP1]);
    let (multi_overlap, _) = runtime(&[LP1, LP2]);
    let h0 = owner0.native_accounting_authority().unwrap();
    let duplicate_h0 = owner0_duplicate.native_accounting_authority().unwrap();
    let h1 = owner1.native_accounting_authority().unwrap();
    let multi_overlap_authority = multi_overlap.native_accounting_authority().unwrap();
    assert_ne!(h0.runtime_id(), duplicate_h0.runtime_id());
    assert!(h0.is_live());
    assert_eq!(owner0.accounting_revision(), Ok(0));

    owner0.register_native_peer(h1.clone()).unwrap();
    assert_eq!(owner0.accounting_revision(), Ok(1));
    owner0.register_native_peer(h1.clone()).unwrap();
    assert_eq!(owner0.accounting_revision(), Ok(1));
    let before_overlap = observe(&owner0, &[LP0, LP2]);
    assert_eq!(
        owner0.register_native_peer(duplicate_h0),
        Err(OptimisticError::NativePeerOwnershipOverlap(LP0))
    );
    assert_eq!(observe(&owner0, &[LP0, LP2]), before_overlap);
    // The candidate overlaps self at LP2 and the registered peer at LP1.
    // Return the smallest LP in the union of overlaps, independent of which
    // peer record is visited first.
    let before_multi_overlap = observe(&owner0, &[LP0, LP2]);
    assert_eq!(
        owner0.register_native_peer(multi_overlap_authority),
        Err(OptimisticError::NativePeerOwnershipOverlap(LP1))
    );
    assert_eq!(observe(&owner0, &[LP0, LP2]), before_multi_overlap);
    assert_eq!(owner0.accounting_revision(), Ok(1));

    owner0.seal_native_peers().unwrap();
    assert_eq!(owner0.accounting_revision(), Ok(2));
    owner0.register_native_peer(h1).unwrap();
    owner0.seal_native_peers().unwrap();
    assert_eq!(owner0.accounting_revision(), Ok(2));
    assert_eq!(owner0.native_peers_sealed(), Ok(true));

    let (late_duplicate, _) = runtime(&[LP0]);
    let before_closed_registry = observe(&owner0, &[LP0, LP2]);
    assert_eq!(
        owner0.register_native_peer(late_duplicate.native_accounting_authority().unwrap()),
        Err(OptimisticError::NativePeerRegistrationClosed)
    );
    assert_eq!(observe(&owner0, &[LP0, LP2]), before_closed_registry);
}

#[test]
fn dropped_issuer_is_stale_before_or_after_seal_and_cannot_be_replaced() {
    let (mut receiver, _) = runtime(&[LP1]);
    let (issuer, _) = runtime(&[LP0]);
    let stale = issuer.native_accounting_authority().unwrap();
    let stale_id = stale.runtime_id();
    receiver.register_native_peer(stale.clone()).unwrap();
    drop(issuer);
    assert!(!stale.is_live());
    assert_eq!(stale.runtime_id(), stale_id);
    assert_eq!(stale.simulation_namespace(), NAMESPACE);
    assert_eq!(stale.owned_lps(), &[LP0]);
    assert_eq!(stale.global_partition(), &partition());

    let before_drop_seal = observe(&receiver, &[LP1]);
    assert_stale(receiver.seal_native_peers().unwrap_err(), stale_id);
    assert_eq!(observe(&receiver, &[LP1]), before_drop_seal);

    let (fresh_issuer, _) = runtime(&[LP0]);
    let fresh = fresh_issuer.native_accounting_authority().unwrap();
    assert!(fresh.is_live());
    assert_ne!(fresh.runtime_id(), stale_id);
    let (mut fresh_receiver, _) = runtime(&[LP2]);
    let before_stale_retry = observe(&fresh_receiver, &[LP2]);
    assert_stale(
        fresh_receiver
            .register_native_peer(stale.clone())
            .unwrap_err(),
        stale_id,
    );
    assert_eq!(observe(&fresh_receiver, &[LP2]), before_stale_retry);
    fresh_receiver.register_native_peer(fresh.clone()).unwrap();
    assert_eq!(fresh_receiver.accounting_revision(), Ok(1));

    let (mut sealed_receiver, _) = runtime(&[LP0]);
    let (other_issuer, _) = runtime(&[LP1]);
    let other_authority = other_issuer.native_accounting_authority().unwrap();
    let (last_issuer, _) = runtime(&[LP2]);
    let last_authority = last_issuer.native_accounting_authority().unwrap();
    sealed_receiver
        .register_native_peer(other_authority)
        .unwrap();
    sealed_receiver
        .register_native_peer(last_authority.clone())
        .unwrap();
    sealed_receiver.seal_native_peers().unwrap();
    let before_drop_after_seal = observe(&sealed_receiver, &[LP0]);
    drop(last_issuer);
    assert!(!last_authority.is_live());
    assert_stale(
        sealed_receiver
            .schedule_initial(77, event(LP0, LP1, 23))
            .unwrap_err(),
        last_authority.runtime_id(),
    );
    assert_eq!(observe(&sealed_receiver, &[LP0]), before_drop_after_seal);
    assert_stale(
        sealed_receiver
            .run_until_with_budget(Tick::from_ticks(23), 0)
            .unwrap_err(),
        last_authority.runtime_id(),
    );
    assert_eq!(observe(&sealed_receiver, &[LP0]), before_drop_after_seal);
    assert_stale(
        sealed_receiver.close_initial_inputs().unwrap_err(),
        last_authority.runtime_id(),
    );
    assert_eq!(observe(&sealed_receiver, &[LP0]), before_drop_after_seal);
}

#[test]
fn owned_raw_receive_and_legacy_issuer_remain_explicitly_gated() {
    let (mut owned, _) = runtime(&[LP0]);
    let local_preview = OptimisticMessage::try_from_authority_parts(
        event(LP0, LP0, 4),
        LogicalEventId::root(LP0, 44),
        OptimisticAuthority::LocalPreview,
        0,
        OptimisticMessageKind::Positive,
    )
    .unwrap();
    let scoped = OptimisticMessage::try_from_authority_parts(
        event(LP0, LP0, 4),
        LogicalEventId::root(LP0, 45),
        OptimisticAuthority::Scoped {
            simulation_namespace: NAMESPACE,
            ownership_epoch: 0,
        },
        0,
        OptimisticMessageKind::Positive,
    )
    .unwrap();
    let scoped_anti = scoped.as_anti();
    let before = observe(&owned, &[LP0]);
    assert_eq!(
        owned.receive(local_preview),
        Err(OptimisticError::AuthorityModeMismatch(LP0))
    );
    assert_eq!(
        owned.receive(scoped),
        Err(OptimisticError::VerifiedNativeAdmissionRequired)
    );
    assert_eq!(
        owned.receive(scoped_anti),
        Err(OptimisticError::VerifiedNativeAdmissionRequired)
    );
    assert_eq!(
        owned.fossil_collect(Tick::ZERO),
        Err(OptimisticError::NativeGroupCutRequired)
    );
    assert_eq!(observe(&owned, &[LP0]), before);

    let processes = [LP0, LP1, LP2]
        .into_iter()
        .map(|lp| (lp, Probe::new(lp)))
        .collect();
    let legacy = OptimisticRuntime::new(
        partition(),
        topology(),
        processes,
        OptimisticLimits::default(),
    )
    .unwrap();
    assert_eq!(
        legacy.native_accounting_authority().unwrap_err(),
        OptimisticError::OwnedModeRequired
    );
    assert_eq!(
        legacy.native_peers_sealed(),
        Err(OptimisticError::OwnedModeRequired)
    );
    assert_eq!(
        legacy.accounting_revision(),
        Err(OptimisticError::OwnedModeRequired)
    );
}
