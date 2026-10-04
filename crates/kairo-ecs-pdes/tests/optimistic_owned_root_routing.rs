#![cfg(feature = "time-warp")]

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Barrier, Mutex};
use std::thread;

use kairo_ecs_pdes::{
    LpId, NativeAccountingAuthority, NativeAdmissionMembership, OptimisticAuthority,
    OptimisticError, OptimisticLimits, OptimisticOwnedOptions, OptimisticProcess,
    OptimisticRuntime, OptimisticStateError, PartitionPlan, RemoteEvent, Tick,
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

    fn snapshot(&self) -> Snapshot {
        self.snapshots.fetch_add(1, Ordering::SeqCst);
        Snapshot(self.value)
    }

    fn restore(&mut self, snapshot: &Snapshot) -> Result<(), OptimisticStateError> {
        self.value = snapshot.0;
        Ok(())
    }

    fn on_event(&mut self, _event: &RemoteEvent) -> Vec<RemoteEvent> {
        self.value += 1;
        Vec::new()
    }
}

fn partition() -> PartitionPlan {
    PartitionPlan::from_entities(
        2,
        SimDuration::from_ticks(1),
        vec![EntityId::new(21, 0), EntityId::new(22, 0)],
    )
    .unwrap()
}

fn topology() -> BTreeMap<LpId, Vec<LpId>> {
    BTreeMap::from([(LpId(0), vec![LpId(1)]), (LpId(1), vec![LpId(0)])])
}

fn options(lp: u32, transition: usize, receipts: usize) -> OptimisticOwnedOptions {
    let namespace = 0xfeed_beef_0123_4567_89ab_cdef_7654_3210;
    let current_authorities = BTreeMap::from([
        (
            LpId(0),
            OptimisticAuthority::Scoped {
                simulation_namespace: namespace,
                ownership_epoch: 13,
            },
        ),
        (
            LpId(1),
            OptimisticAuthority::Scoped {
                simulation_namespace: namespace,
                ownership_epoch: 29,
            },
        ),
    ]);
    let emission_epochs = BTreeMap::from([(LpId(lp), if lp == 0 { 13 } else { 29 })]);
    OptimisticOwnedOptions {
        simulation_namespace: namespace,
        current_authorities,
        emission_epochs,
        local_limits: OptimisticLimits::default(),
        max_global_lps: 2,
        max_outbox_entries: 8,
        max_transition_entries: transition,
        max_receipt_entries: receipts,
    }
}

fn runtime(lp: u32, transition: usize) -> OptimisticRuntime<Model> {
    runtime_with_receipts(lp, transition, 8)
}

fn runtime_with_receipts(lp: u32, transition: usize, receipts: usize) -> OptimisticRuntime<Model> {
    OptimisticRuntime::new_owned(
        partition(),
        topology(),
        BTreeMap::from([(
            LpId(lp),
            Model {
                snapshots: Arc::new(AtomicUsize::new(0)),
                value: u64::from(lp),
            },
        )]),
        options(lp, transition, receipts),
    )
    .unwrap()
}

fn root(source: u32, destination: u32, sequence: u8) -> RemoteEvent {
    RemoteEvent {
        source_lp: LpId(source),
        dest_lp: LpId(destination),
        tick: Tick::from_ticks(u128::MAX - 100 + u128::from(sequence)),
        event_payload: vec![0, 255, sequence],
    }
}

fn pair(transition: usize) -> (OptimisticRuntime<Model>, OptimisticRuntime<Model>) {
    pair_with_receipts(transition, 8)
}

fn pair_with_receipts(
    transition: usize,
    receipts: usize,
) -> (OptimisticRuntime<Model>, OptimisticRuntime<Model>) {
    let mut a = runtime_with_receipts(0, transition, receipts);
    let mut b = runtime_with_receipts(1, transition, receipts);
    let issuer_a = a.native_accounting_authority().unwrap();
    let issuer_b = b.native_accounting_authority().unwrap();
    a.register_native_peer(issuer_b.clone()).unwrap();
    b.register_native_peer(issuer_a).unwrap();
    a.seal_native_peers().unwrap();
    b.seal_native_peers().unwrap();
    (a, b)
}

#[test]
fn local_roots_enqueue_once_and_remote_roots_cross_only_the_native_join() {
    let (mut a, mut b) = pair(8);
    let local_token = a.state_token(LpId(0)).unwrap();
    let local = a.schedule_initial(1, root(0, 0, 1)).unwrap();
    assert_eq!(
        local.authority(),
        OptimisticAuthority::Scoped {
            simulation_namespace: options(0, 8, 8).simulation_namespace,
            ownership_epoch: 13
        }
    );
    assert!(!a.validate_state_token(local_token));
    assert_eq!(a.pending_events(LpId(0)).unwrap(), vec![local.clone()]);
    assert_eq!(a.accounting_snapshot().unwrap().reserved_receipt_count(), 0);

    let source_token = a.state_token(LpId(0)).unwrap();
    let model_before = a.process_at(LpId(0)).unwrap().value;
    let remote = a.schedule_initial(2, root(0, 1, 2)).unwrap();
    assert!(a.validate_state_token(source_token));
    assert_eq!(a.process_at(LpId(0)).unwrap().value, model_before);
    assert!(a.pending_events(LpId(1)).is_none());
    assert_eq!(a.pending_events(LpId(0)).unwrap().len(), 1);
    let outbound = a.ready_native_sends().unwrap();
    assert_eq!(outbound.len(), 1);
    assert_eq!(outbound[0].message(), &remote);
    assert_eq!(outbound[0].key().source_lp(), LpId(0));
    assert_eq!(outbound[0].key().incarnation(), remote.incarnation());
    assert_eq!(a.accounting_snapshot().unwrap().reserved_receipt_count(), 1);

    let before_receiver = b.report();
    let receiver_token = b.state_token(LpId(1)).unwrap();
    let cap = b.admit_native(&outbound[0]).unwrap();
    assert_eq!(
        cap.recorded_membership(),
        NativeAdmissionMembership::Pending
    );
    assert_eq!(cap.message(), &remote);
    assert_eq!(
        b.report().pending_events,
        before_receiver.pending_events + 1
    );
    assert!(!b.validate_state_token(receiver_token));
    let rev = b.accounting_revision().unwrap();
    let cap_retry = b.admit_native(&outbound[0]).unwrap();
    assert_eq!(cap_retry.key(), cap.key());
    assert_eq!(cap_retry.message(), cap.message());
    assert_eq!(cap_retry.recorded_revision(), cap.recorded_revision());
    assert_eq!(b.accounting_revision().unwrap(), rev);
    assert_eq!(b.accounting_snapshot().unwrap().retained_receipt_count(), 1);

    b.close_initial_inputs().unwrap();
    let before_ack = a.accounting_revision().unwrap();
    a.acknowledge_native_admission(cap.clone()).unwrap();
    assert_eq!(a.accounting_revision().unwrap(), before_ack + 1);
    assert!(a.outbound_pending().unwrap().is_empty());
    assert_eq!(a.accounting_snapshot().unwrap().retained_receipt_count(), 1);
    let after_ack = a.accounting_revision().unwrap();
    a.acknowledge_native_admission(cap).unwrap();
    assert_eq!(a.accounting_revision().unwrap(), after_ack);

    let before = b.accounting_snapshot().unwrap();
    assert_eq!(
        a.schedule_initial(2, root(0, 0, 3)),
        Err(OptimisticError::ConflictingLogicalEvent { source_lp: LpId(0) })
    );
    assert_eq!(a.accounting_snapshot().unwrap().revision(), after_ack);
    assert_eq!(b.accounting_snapshot().unwrap(), before);
}

#[test]
fn transition_reservations_are_shared_persistent_and_preflighted_before_other_bounds() {
    let (mut local_first, _local_first_peer) = pair(1);
    local_first.schedule_initial(1, root(0, 0, 1)).unwrap();
    let before = local_first.accounting_snapshot().unwrap();
    assert_eq!(before.reserved_receipt_count(), 0);
    assert_eq!(
        local_first.schedule_initial(2, root(0, 1, 2)),
        Err(OptimisticError::TransitionLimitExceeded { limit: 1 })
    );
    assert_eq!(local_first.accounting_snapshot().unwrap(), before);

    let (mut remote_first, _remote_first_peer) = pair(1);
    remote_first.schedule_initial(1, root(0, 1, 1)).unwrap();
    let before = remote_first.accounting_snapshot().unwrap();
    assert_eq!(
        remote_first.schedule_initial(2, root(0, 0, 2)),
        Err(OptimisticError::TransitionLimitExceeded { limit: 1 })
    );
    assert_eq!(remote_first.accounting_snapshot().unwrap(), before);
    assert_eq!(
        remote_first.schedule_initial(1, root(0, 0, 3)),
        Err(OptimisticError::ConflictingLogicalEvent { source_lp: LpId(0) })
    );
    assert_eq!(remote_first.accounting_snapshot().unwrap(), before);

    let (mut local_local, _local_local_peer) = pair(1);
    local_local.schedule_initial(3, root(0, 0, 1)).unwrap();
    let before = local_local.accounting_snapshot().unwrap();
    assert_eq!(
        local_local.schedule_initial(4, root(0, 0, 2)),
        Err(OptimisticError::TransitionLimitExceeded { limit: 1 })
    );
    assert_eq!(local_local.accounting_snapshot().unwrap(), before);

    let (mut acked, mut receiver) = pair_with_receipts(1, 1);
    receiver.schedule_initial(90, root(1, 1, 7)).unwrap();
    acked.schedule_initial(5, root(0, 1, 1)).unwrap();
    let send = acked.ready_native_sends().unwrap().remove(0);
    let cap = receiver.admit_native(&send).unwrap();
    assert_eq!(receiver.admit_native(&send).unwrap().key(), cap.key());
    assert_eq!(
        receiver
            .accounting_snapshot()
            .unwrap()
            .retained_receipt_count(),
        1
    );
    acked.acknowledge_native_admission(cap).unwrap();
    let before = acked.accounting_snapshot().unwrap();
    assert_eq!(before.reserved_receipt_count(), 0);
    assert_eq!(before.retained_receipt_count(), 1);
    assert_eq!(
        acked.schedule_initial(6, root(0, 1, 2)),
        Err(OptimisticError::TransitionLimitExceeded { limit: 1 })
    );
    assert_eq!(acked.accounting_snapshot().unwrap(), before);
}

#[test]
fn identical_keys_from_a_recreated_issuer_cannot_reuse_or_poison_receipts() {
    let mut a = runtime(0, 8);
    let mut a_prime = runtime(0, 8);
    let mut b = runtime(1, 8);
    let issuer_a = a.native_accounting_authority().unwrap();
    let issuer_a_prime = a_prime.native_accounting_authority().unwrap();
    let issuer_b = b.native_accounting_authority().unwrap();
    a.register_native_peer(issuer_b.clone()).unwrap();
    a_prime.register_native_peer(issuer_b.clone()).unwrap();
    b.register_native_peer(issuer_a.clone()).unwrap();
    for runtime in [&mut a, &mut a_prime, &mut b] {
        runtime.seal_native_peers().unwrap();
    }
    a.schedule_initial(77, root(0, 1, 8)).unwrap();
    a_prime.schedule_initial(77, root(0, 1, 8)).unwrap();
    let send_a = a.ready_native_sends().unwrap().remove(0);
    let send_a_prime = a_prime.ready_native_sends().unwrap().remove(0);
    assert_eq!(send_a.key(), send_a_prime.key());
    let cap_a = b.admit_native(&send_a).unwrap();
    let receiver_before = b.accounting_snapshot().unwrap();
    assert!(matches!(
        b.admit_native(&send_a_prime),
        Err(OptimisticError::UnregisteredNativeIssuer { runtime_id, recovery_generation: 0 })
            if runtime_id == issuer_a_prime.runtime_id()
    ));
    assert_eq!(b.accounting_snapshot().unwrap(), receiver_before);
    assert!(matches!(
        a_prime.acknowledge_native_admission(cap_a.clone()),
        Err(OptimisticError::NativeSendIssuerMismatch { source_lp: LpId(0) })
    ));
    assert_eq!(a.acknowledge_native_admission(cap_a.clone()), Ok(()));
    let after = a.accounting_revision().unwrap();
    a.acknowledge_native_admission(cap_a).unwrap();
    assert_eq!(a.accounting_revision().unwrap(), after);
}

#[test]
fn issuer_drop_race_is_atomic_and_drop_invalidates_before_process_destructor() {
    let (mut a, b) = pair(8);
    let barrier = Arc::new(Barrier::new(2));
    let drop_barrier = Arc::clone(&barrier);
    let result = thread::scope(|scope| {
        let dropper = scope.spawn(move || {
            drop_barrier.wait();
            drop(b);
        });
        let route_barrier = Arc::clone(&barrier);
        let router = scope.spawn(move || {
            route_barrier.wait();
            let routed = a.schedule_initial(88, root(0, 1, 9));
            (routed, a)
        });
        dropper.join().unwrap();
        router.join().unwrap()
    });
    let (routed, a) = result;
    match routed {
        Ok(_) => {
            assert!(matches!(
                a.accounting_snapshot(),
                Err(OptimisticError::StaleNativeAccountingAuthority { .. })
            ));
            assert_eq!(a.accounting_revision().unwrap(), 3);
            assert_eq!(a.report().pending_events, 0);
        }
        Err(OptimisticError::StaleNativeAccountingAuthority { .. }) => {
            assert_eq!(a.accounting_revision().unwrap(), 2);
            assert_eq!(a.report().pending_events, 0);
        }
        other => panic!("unexpected routing race result: {other:?}"),
    }

    struct DropModel {
        witness: Arc<Mutex<Option<NativeAccountingAuthority>>>,
        observed_inactive: Arc<AtomicBool>,
    }
    impl OptimisticProcess for DropModel {
        type Snapshot = ();
        fn snapshot(&self) {}
        fn restore(&mut self, _: &()) -> Result<(), OptimisticStateError> {
            Ok(())
        }
        fn on_event(&mut self, _: &RemoteEvent) -> Vec<RemoteEvent> {
            Vec::new()
        }
    }
    impl Drop for DropModel {
        fn drop(&mut self) {
            if let Some(authority) = self.witness.lock().unwrap().as_ref() {
                self.observed_inactive
                    .store(!authority.is_live(), Ordering::SeqCst);
            }
        }
    }
    let witness = Arc::new(Mutex::new(None));
    let observed = Arc::new(AtomicBool::new(false));
    let runtime = OptimisticRuntime::new_owned(
        partition(),
        topology(),
        BTreeMap::from([(
            LpId(0),
            DropModel {
                witness: Arc::clone(&witness),
                observed_inactive: Arc::clone(&observed),
            },
        )]),
        options(0, 8, 8),
    )
    .unwrap();
    *witness.lock().unwrap() = Some(runtime.native_accounting_authority().unwrap());
    drop(runtime);
    assert!(observed.load(Ordering::SeqCst));
}
