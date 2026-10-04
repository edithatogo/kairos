#![cfg(feature = "time-warp")]

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use kairo_ecs_pdes::{
    LogicalEventId, LpId, NativeAccountingAuthority, OptimisticAuthority, OptimisticError,
    OptimisticLimits, OptimisticMessage, OptimisticMessageKind, OptimisticOwnedOptions,
    OptimisticProcess, OptimisticRuntime, OptimisticStateError, PartitionPlan, RemoteEvent, Tick,
};
use kairo_ecs_types::{EntityId, SimDuration};

#[derive(Clone, Debug, Eq, PartialEq)]
struct Snapshot(u64);

struct Model {
    snapshots: Arc<AtomicUsize>,
    value: u64,
}

impl OptimisticProcess for Model {
    type Snapshot = Snapshot;

    fn snapshot(&self) -> Self::Snapshot {
        self.snapshots.fetch_add(1, Ordering::SeqCst);
        Snapshot(self.value)
    }

    fn restore(&mut self, state: &Self::Snapshot) -> Result<(), OptimisticStateError> {
        self.value = state.0;
        Ok(())
    }

    fn on_event(&mut self, _event: &RemoteEvent) -> Vec<RemoteEvent> {
        self.value += 1;
        Vec::new()
    }
}

fn partition(lookahead: u128) -> PartitionPlan {
    PartitionPlan::from_entities(
        2,
        SimDuration::from_ticks(lookahead),
        vec![EntityId::new(1, 0), EntityId::new(2, 0)],
    )
    .unwrap()
}

fn partition_many(count: u32, lookahead: u128) -> PartitionPlan {
    PartitionPlan::from_entities(
        count,
        SimDuration::from_ticks(lookahead),
        (0..count)
            .map(|entity| EntityId::new(u64::from(entity) + 100, 0))
            .collect(),
    )
    .unwrap()
}

fn topology() -> BTreeMap<LpId, Vec<LpId>> {
    BTreeMap::from([(LpId(0), vec![LpId(1)]), (LpId(1), vec![LpId(0)])])
}

fn topology_chain(count: u32) -> BTreeMap<LpId, Vec<LpId>> {
    (0..count)
        .map(|lp| {
            let mut destinations = Vec::new();
            if lp > 0 {
                destinations.push(LpId(lp - 1));
            }
            if lp + 1 < count {
                destinations.push(LpId(lp + 1));
            }
            (LpId(lp), destinations)
        })
        .collect()
}

fn options_for(global: &[u32], owned: &[u32]) -> OptimisticOwnedOptions {
    let namespace = u128::MAX - 11;
    let current_authorities = global
        .iter()
        .map(|&lp| {
            (
                LpId(lp),
                OptimisticAuthority::Scoped {
                    simulation_namespace: namespace,
                    ownership_epoch: u64::MAX - 2 * u64::from(lp),
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    let emission_epochs = owned
        .iter()
        .map(|&lp| {
            let OptimisticAuthority::Scoped {
                ownership_epoch, ..
            } = current_authorities[&LpId(lp)]
            else {
                unreachable!()
            };
            (LpId(lp), ownership_epoch)
        })
        .collect();
    OptimisticOwnedOptions {
        simulation_namespace: namespace,
        current_authorities,
        emission_epochs,
        local_limits: OptimisticLimits::default(),
        max_global_lps: global.len(),
        max_outbox_entries: 8,
        max_transition_entries: 8,
        max_receipt_entries: 8,
    }
}

fn options(owned: &[u32]) -> OptimisticOwnedOptions {
    options_for(&[0, 1], owned)
}

fn runtime(lp: u32, snapshots: Arc<AtomicUsize>) -> OptimisticRuntime<Model> {
    OptimisticRuntime::new_owned(
        partition(5),
        topology(),
        BTreeMap::from([(
            LpId(lp),
            Model {
                snapshots,
                value: lp as u64,
            },
        )]),
        options(&[lp]),
    )
    .unwrap()
}

fn runtime_scope(global_count: u32, owned: &[u32]) -> OptimisticRuntime<Model> {
    let snapshots = Arc::new(AtomicUsize::new(0));
    OptimisticRuntime::new_owned(
        partition_many(global_count, 5),
        topology_chain(global_count),
        owned
            .iter()
            .map(|&lp| {
                (
                    LpId(lp),
                    Model {
                        snapshots: Arc::clone(&snapshots),
                        value: u64::from(lp),
                    },
                )
            })
            .collect(),
        options_for(&(0..global_count).collect::<Vec<_>>(), owned),
    )
    .unwrap()
}

fn event(source: u32, destination: u32) -> RemoteEvent {
    RemoteEvent {
        source_lp: LpId(source),
        dest_lp: LpId(destination),
        tick: Tick::from_ticks(2),
        event_payload: vec![1],
    }
}

fn raw_message(authority: OptimisticAuthority) -> OptimisticMessage {
    OptimisticMessage::try_from_authority_parts(
        event(0, 1),
        LogicalEventId::root(LpId(0), 7),
        authority,
        0,
        OptimisticMessageKind::Positive,
    )
    .unwrap()
}

fn assert_config(capability: &NativeAccountingAuthority, lp: u32) {
    assert_eq!(capability.recovery_generation(), 0);
    assert_eq!(capability.simulation_namespace(), u128::MAX - 11);
    assert_eq!(capability.owned_lps(), &[LpId(lp)]);
    assert_eq!(capability.global_partition(), &partition(5));
    assert_eq!(capability.global_topology(), &topology());
    assert_eq!(
        capability.current_authorities(),
        &options(&[lp]).current_authorities
    );
    assert_eq!(capability.emission_epochs().len(), 1);
    assert_eq!(
        capability.emission_epochs()[&LpId(lp)],
        u64::MAX - 2 * u64::from(lp)
    );
}

#[test]
fn snapshots_only_real_owned_processes_and_capability_retains_immutable_full_config() {
    let snap0 = Arc::new(AtomicUsize::new(0));
    let snap1 = Arc::new(AtomicUsize::new(0));
    let runtime0 = runtime(0, Arc::clone(&snap0));
    let runtime1 = runtime(1, Arc::clone(&snap1));
    assert_eq!(snap0.load(Ordering::SeqCst), 1);
    assert_eq!(snap1.load(Ordering::SeqCst), 1);
    assert!(runtime0.process_at(LpId(1)).is_none());
    assert!(runtime0.pending_events(LpId(1)).is_none());
    assert_eq!(
        runtime0.state_token(LpId(1)),
        Err(OptimisticError::UnownedLogicalProcess(LpId(1)))
    );
    let authority0 = runtime0.native_accounting_authority().unwrap();
    let authority1 = runtime1.native_accounting_authority().unwrap();
    assert_config(&authority0, 0);
    assert_config(&authority1, 1);
    assert!(authority0.is_live());

    let legacy = OptimisticRuntime::new(
        partition(5),
        topology(),
        BTreeMap::from([
            (
                LpId(0),
                Model {
                    snapshots: Arc::new(AtomicUsize::new(0)),
                    value: 0,
                },
            ),
            (
                LpId(1),
                Model {
                    snapshots: Arc::new(AtomicUsize::new(0)),
                    value: 1,
                },
            ),
        ]),
        OptimisticLimits::default(),
    )
    .unwrap();
    assert_eq!(
        legacy.native_accounting_authority().unwrap_err(),
        OptimisticError::OwnedModeRequired
    );
    assert_eq!(
        legacy.accounting_revision().unwrap_err(),
        OptimisticError::OwnedModeRequired
    );
}

#[test]
fn disjoint_live_peers_seal_close_and_retry_with_exact_checked_revisions() {
    let snap0 = Arc::new(AtomicUsize::new(0));
    let snap1 = Arc::new(AtomicUsize::new(0));
    let mut runtime0 = runtime(0, snap0);
    let mut runtime1 = runtime(1, snap1);
    let own0 = runtime0.native_accounting_authority().unwrap();
    let own1 = runtime1.native_accounting_authority().unwrap();
    runtime0.register_native_peer(own0.clone()).unwrap();
    assert_eq!(runtime0.accounting_revision().unwrap(), 0);
    assert_eq!(
        runtime0.seal_native_peers(),
        Err(OptimisticError::NativePeerCoverageIncomplete {
            missing: vec![LpId(1)]
        })
    );
    runtime0.register_native_peer(own1.clone()).unwrap();
    runtime1.register_native_peer(own0).unwrap();
    runtime0.register_native_peer(own1.clone()).unwrap();
    assert_eq!(runtime0.accounting_revision().unwrap(), 1);
    runtime0.seal_native_peers().unwrap();
    runtime0.seal_native_peers().unwrap();
    assert_eq!(runtime0.accounting_revision().unwrap(), 2);
    assert!(runtime0.native_peers_sealed().unwrap());
    runtime0.close_initial_inputs().unwrap();
    runtime0.close_initial_inputs().unwrap();
    assert!(runtime0.initial_inputs_closed());
    assert_eq!(runtime0.accounting_revision().unwrap(), 3);
    assert_eq!(
        runtime0.schedule_initial(4, event(0, 1)),
        Err(OptimisticError::InitialSchedulingClosed)
    );
    runtime0.register_native_peer(own1).unwrap();
    assert_eq!(runtime0.accounting_revision().unwrap(), 3);
    let distinct_snapshot = Arc::new(AtomicUsize::new(0));
    let distinct_issuer = runtime(1, distinct_snapshot);
    let distinct_authority = distinct_issuer.native_accounting_authority().unwrap();
    assert_eq!(
        runtime0.register_native_peer(distinct_authority),
        Err(OptimisticError::NativePeerRegistrationClosed)
    );
}

#[test]
fn overlap_and_dropped_peer_are_rejected_without_revision_change() {
    let snapshots = Arc::new(AtomicUsize::new(0));
    let mut issuer = runtime(0, Arc::clone(&snapshots));
    let overlap = OptimisticRuntime::new_owned(
        partition(5),
        topology(),
        BTreeMap::from([(
            LpId(0),
            Model {
                snapshots,
                value: 0,
            },
        )]),
        options(&[0]),
    )
    .unwrap();
    let other = overlap.native_accounting_authority().unwrap();
    assert_eq!(
        issuer.register_native_peer(other),
        Err(OptimisticError::NativePeerOwnershipOverlap(LpId(0)))
    );
    assert_eq!(issuer.accounting_revision().unwrap(), 0);

    let peer_snapshot = Arc::new(AtomicUsize::new(0));
    let peer = runtime(1, peer_snapshot);
    let stale = peer.native_accounting_authority().unwrap();
    issuer.register_native_peer(stale.clone()).unwrap();
    drop(peer);
    assert!(!stale.is_live());
    assert!(matches!(
        issuer.seal_native_peers(),
        Err(OptimisticError::StaleNativeAccountingAuthority { .. })
    ));
    assert_eq!(issuer.accounting_revision().unwrap(), 1);

    let mut sealed_owner = runtime(0, Arc::new(AtomicUsize::new(0)));
    let sealed_peer = runtime(1, Arc::new(AtomicUsize::new(0)));
    sealed_owner
        .register_native_peer(sealed_peer.native_accounting_authority().unwrap())
        .unwrap();
    sealed_owner.seal_native_peers().unwrap();
    drop(sealed_peer);
    assert!(matches!(
        sealed_owner.close_initial_inputs(),
        Err(OptimisticError::StaleNativeAccountingAuthority { .. })
    ));
}

#[test]
fn guarded_owned_operations_and_raw_admission_preserve_observable_state() {
    let mut runtime0 = runtime(0, Arc::new(AtomicUsize::new(0)));
    let mut runtime1 = runtime(1, Arc::new(AtomicUsize::new(0)));
    let authority0 = runtime0.native_accounting_authority().unwrap();
    let authority1 = runtime1.native_accounting_authority().unwrap();
    runtime0.register_native_peer(authority1.clone()).unwrap();
    runtime1.register_native_peer(authority0).unwrap();
    runtime0.seal_native_peers().unwrap();
    let before_schedule = runtime0.report();
    let before_queue = runtime0.pending_events(LpId(0));
    let before_value = runtime0.process_at(LpId(0)).unwrap().value;
    let token = runtime0.state_token(LpId(0)).unwrap();
    let scheduled = runtime0.schedule_initial(3, event(0, 1)).unwrap();
    assert_eq!(scheduled.event(), &event(0, 1));
    assert_eq!(runtime0.outbound_pending().unwrap().len(), 1);
    assert_eq!(runtime0.ready_native_sends().unwrap().len(), 1);
    let after_schedule = runtime0.report();
    let after_queue = runtime0.pending_events(LpId(0));
    let snapshot = runtime0.accounting_snapshot().unwrap();
    assert_eq!(snapshot.ready_positive_count(), 1);
    assert_eq!(snapshot.reserved_receipt_count(), 1);
    assert_eq!(snapshot.revision(), 3);
    assert_eq!(runtime0.report(), before_schedule);
    assert_eq!(runtime0.pending_events(LpId(0)), before_queue);
    assert_eq!(
        runtime0.run_until_with_budget(Tick::from_ticks(4), 0),
        Err(OptimisticError::OwnedRuntimeJoinIncomplete)
    );
    assert_eq!(
        runtime0.receive(raw_message(OptimisticAuthority::LocalPreview)),
        Err(OptimisticError::AuthorityModeMismatch(LpId(0)))
    );
    assert_eq!(
        runtime0.receive(raw_message(OptimisticAuthority::Scoped {
            simulation_namespace: u128::MAX - 11,
            ownership_epoch: u64::MAX
        })),
        Err(OptimisticError::VerifiedNativeAdmissionRequired)
    );
    assert_eq!(
        runtime0.fossil_collect(Tick::ZERO),
        Err(OptimisticError::NativeGroupCutRequired)
    );
    assert_eq!(runtime0.report(), after_schedule);
    assert_eq!(runtime0.pending_events(LpId(0)), after_queue);
    assert_eq!(runtime0.process_at(LpId(0)).unwrap().value, before_value);
    assert!(runtime0.validate_state_token(token));
    assert_eq!(runtime0.accounting_revision().unwrap(), 3);
}

#[test]
fn configuration_rejects_incomplete_maps_and_partition_mismatch_before_snapshot() {
    let snapshots = Arc::new(AtomicUsize::new(0));
    let mut invalid = options(&[0]);
    invalid.current_authorities.remove(&LpId(1));
    let result = OptimisticRuntime::new_owned(
        partition(5),
        topology(),
        BTreeMap::from([(
            LpId(0),
            Model {
                snapshots: Arc::clone(&snapshots),
                value: 0,
            },
        )]),
        invalid,
    );
    assert_eq!(
        result.err(),
        Some(OptimisticError::AuthoritySetMismatch {
            missing: vec![LpId(1)],
            unexpected: vec![]
        })
    );
    assert_eq!(snapshots.load(Ordering::SeqCst), 0);

    let result = OptimisticRuntime::new_owned(
        partition(6),
        topology(),
        BTreeMap::from([(
            LpId(0),
            Model {
                snapshots: Arc::clone(&snapshots),
                value: 0,
            },
        )]),
        options(&[0]),
    );
    let runtime = result.unwrap();
    assert_eq!(
        runtime
            .native_accounting_authority()
            .unwrap()
            .global_partition()
            .lookahead(),
        SimDuration::from_ticks(6)
    );
    assert_eq!(snapshots.load(Ordering::SeqCst), 1);
}

#[test]
fn constructor_rejects_each_configuration_boundary_before_snapshot_capture() {
    let snapshots = Arc::new(AtomicUsize::new(0));
    let build = |plan: PartitionPlan,
                 directed: BTreeMap<LpId, Vec<LpId>>,
                 settings: OptimisticOwnedOptions|
     -> Result<OptimisticRuntime<Model>, OptimisticError> {
        OptimisticRuntime::new_owned(
            plan,
            directed,
            BTreeMap::from([(
                LpId(0),
                Model {
                    snapshots: Arc::clone(&snapshots),
                    value: 0,
                },
            )]),
            settings,
        )
    };
    let plan = partition(5);

    let mut settings = options(&[0]);
    settings.max_global_lps = 1;
    assert_eq!(
        build(plan.clone(), topology(), settings).err(),
        Some(OptimisticError::GlobalLpLimitExceeded {
            actual: 2,
            limit: 1
        })
    );

    for capacity in 0..4 {
        let mut settings = options(&[0]);
        match capacity {
            0 => settings.max_outbox_entries = 0,
            1 => settings.max_transition_entries = 0,
            2 => settings.max_receipt_entries = 0,
            _ => settings.max_global_lps = 0,
        }
        assert_eq!(
            build(plan.clone(), topology(), settings).err(),
            Some(OptimisticError::InvalidLimits)
        );
    }

    let mut settings = options(&[0]);
    settings.current_authorities.insert(
        LpId(2),
        OptimisticAuthority::Scoped {
            simulation_namespace: settings.simulation_namespace,
            ownership_epoch: 1,
        },
    );
    assert_eq!(
        build(plan.clone(), topology(), settings).err(),
        Some(OptimisticError::AuthoritySetMismatch {
            missing: vec![],
            unexpected: vec![LpId(2)]
        })
    );

    let unexpected_process = OptimisticRuntime::new_owned(
        plan.clone(),
        topology(),
        BTreeMap::from([(
            LpId(2),
            Model {
                snapshots: Arc::clone(&snapshots),
                value: 0,
            },
        )]),
        options(&[0]),
    );
    assert_eq!(
        unexpected_process.err(),
        Some(OptimisticError::ProcessSetMismatch {
            missing: vec![],
            unexpected: vec![LpId(2)]
        })
    );

    let mut settings = options(&[0]);
    settings
        .current_authorities
        .insert(LpId(0), OptimisticAuthority::LocalPreview);
    assert_eq!(
        build(plan.clone(), topology(), settings).err(),
        Some(OptimisticError::AuthorityModeMismatch(LpId(0)))
    );

    let mut settings = options(&[0]);
    if let OptimisticAuthority::Scoped {
        simulation_namespace,
        ownership_epoch,
    } = settings.current_authorities[&LpId(0)]
    {
        settings.current_authorities.insert(
            LpId(0),
            OptimisticAuthority::Scoped {
                simulation_namespace: simulation_namespace - 1,
                ownership_epoch,
            },
        );
    }
    assert_eq!(
        build(plan.clone(), topology(), settings).err(),
        Some(OptimisticError::AuthorityNamespaceMismatch {
            lp_id: LpId(0),
            expected: u128::MAX - 11,
            actual: u128::MAX - 12
        })
    );

    let mut settings = options(&[0]);
    settings.emission_epochs.clear();
    assert_eq!(
        build(plan.clone(), topology(), settings).err(),
        Some(OptimisticError::EmissionEpochSetMismatch {
            missing: vec![LpId(0)],
            unexpected: vec![]
        })
    );

    let mut settings = options(&[0]);
    settings.emission_epochs.insert(LpId(1), 0);
    assert_eq!(
        build(plan.clone(), topology(), settings).err(),
        Some(OptimisticError::EmissionEpochSetMismatch {
            missing: vec![],
            unexpected: vec![LpId(1)]
        })
    );

    let mut settings = options(&[0]);
    settings.emission_epochs.insert(LpId(0), 0);
    assert_eq!(
        build(plan.clone(), topology(), settings).err(),
        Some(OptimisticError::EmissionEpochMismatch {
            lp_id: LpId(0),
            expected: u64::MAX,
            actual: 0
        })
    );

    let mut missing_entry = topology();
    missing_entry.remove(&LpId(1));
    assert_eq!(
        build(plan.clone(), missing_entry, options(&[0])).err(),
        Some(OptimisticError::MissingTopologyEntry(LpId(1)))
    );

    let mut unknown_source = topology();
    unknown_source.insert(LpId(2), vec![]);
    assert_eq!(
        build(plan.clone(), unknown_source, options(&[0])).err(),
        Some(OptimisticError::UnknownTopologySource(LpId(2)))
    );

    let mut unknown_destination = topology();
    unknown_destination.insert(LpId(0), vec![LpId(2)]);
    assert_eq!(
        build(plan.clone(), unknown_destination, options(&[0])).err(),
        Some(OptimisticError::UnknownTopologyDestination {
            source: LpId(0),
            destination: LpId(2)
        })
    );

    let mut duplicate_neighbor = topology();
    duplicate_neighbor.insert(LpId(0), vec![LpId(1), LpId(1)]);
    assert_eq!(
        build(plan.clone(), duplicate_neighbor, options(&[0])).err(),
        Some(OptimisticError::DuplicateNeighbor {
            source: LpId(0),
            destination: LpId(1)
        })
    );

    let mut self_loop = topology();
    self_loop.insert(LpId(0), vec![LpId(0)]);
    assert_eq!(
        build(plan, self_loop, options(&[0])).err(),
        Some(OptimisticError::DuplicateNeighbor {
            source: LpId(0),
            destination: LpId(0)
        })
    );
    assert_eq!(snapshots.load(Ordering::SeqCst), 0);
}

#[test]
fn topology_is_canonical_and_peer_configuration_includes_partition_and_lookahead() {
    let namespace = u128::MAX - 11;
    let plan3 = PartitionPlan::from_entities(
        3,
        SimDuration::from_ticks(5),
        vec![
            EntityId::new(1, 0),
            EntityId::new(2, 0),
            EntityId::new(3, 0),
        ],
    )
    .unwrap();
    let config = options_for(&[0, 1, 2], &[0]);
    let runtime3 = OptimisticRuntime::new_owned(
        plan3,
        BTreeMap::from([
            (LpId(0), vec![LpId(2), LpId(1)]),
            (LpId(1), vec![LpId(0)]),
            (LpId(2), vec![LpId(0)]),
        ]),
        BTreeMap::from([(
            LpId(0),
            Model {
                snapshots: Arc::new(AtomicUsize::new(0)),
                value: 0,
            },
        )]),
        config,
    )
    .unwrap();
    assert_eq!(
        runtime3
            .native_accounting_authority()
            .unwrap()
            .global_topology()[&LpId(0)],
        vec![LpId(1), LpId(2)]
    );

    let mut owner = runtime(0, Arc::new(AtomicUsize::new(0)));
    let different_lookahead = OptimisticRuntime::new_owned(
        partition(6),
        topology(),
        BTreeMap::from([(
            LpId(1),
            Model {
                snapshots: Arc::new(AtomicUsize::new(0)),
                value: 1,
            },
        )]),
        options(&[1]),
    )
    .unwrap();
    assert_eq!(
        owner.register_native_peer(different_lookahead.native_accounting_authority().unwrap()),
        Err(OptimisticError::NativePeerConfigurationMismatch)
    );

    let different_membership = PartitionPlan::from_entities(
        2,
        SimDuration::from_ticks(5),
        vec![EntityId::new(1, 0), EntityId::new(3, 0)],
    )
    .unwrap();
    let different_plan_issuer = OptimisticRuntime::new_owned(
        different_membership,
        topology(),
        BTreeMap::from([(
            LpId(1),
            Model {
                snapshots: Arc::new(AtomicUsize::new(0)),
                value: 1,
            },
        )]),
        options(&[1]),
    )
    .unwrap();
    assert_eq!(
        owner.register_native_peer(different_plan_issuer.native_accounting_authority().unwrap()),
        Err(OptimisticError::NativePeerConfigurationMismatch)
    );

    let empty_set = OptimisticRuntime::<Model>::new_owned(
        partition(5),
        topology(),
        BTreeMap::new(),
        options(&[]),
    );
    assert_eq!(empty_set.err(), Some(OptimisticError::EmptyOwnedProcessSet));

    let mut invalid_limits = options(&[]);
    invalid_limits.local_limits.max_lps = 0;
    let empty = OptimisticRuntime::<Model>::new_owned(
        partition(5),
        topology(),
        BTreeMap::new(),
        invalid_limits,
    );
    assert_eq!(empty.err(), Some(OptimisticError::InvalidLimits));
    let mut local_bound = options(&[0, 1]);
    local_bound.local_limits.max_lps = 1;
    let too_many_processes = OptimisticRuntime::new_owned(
        partition(5),
        topology(),
        BTreeMap::from([
            (
                LpId(0),
                Model {
                    snapshots: Arc::new(AtomicUsize::new(0)),
                    value: 0,
                },
            ),
            (
                LpId(1),
                Model {
                    snapshots: Arc::new(AtomicUsize::new(0)),
                    value: 1,
                },
            ),
        ]),
        local_bound,
    );
    assert_eq!(
        too_many_processes.err(),
        Some(OptimisticError::InvalidLimits)
    );
    assert_ne!(namespace, 0);
}

#[test]
fn overlapping_multi_lp_peer_reports_the_globally_first_lp() {
    let mut first_issuer = runtime_scope(101, &[0, 100]);
    let second_issuer = runtime_scope(101, &[2, 3]);
    let incoming_issuer = runtime_scope(101, &[3, 100]);
    first_issuer
        .register_native_peer(second_issuer.native_accounting_authority().unwrap())
        .unwrap();
    assert_eq!(
        first_issuer.register_native_peer(incoming_issuer.native_accounting_authority().unwrap()),
        Err(OptimisticError::NativePeerOwnershipOverlap(LpId(3)))
    );
    assert_eq!(first_issuer.accounting_revision().unwrap(), 1);
}
