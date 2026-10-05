#![cfg(feature = "time-warp")]

use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

use kairo_ecs_pdes::{
    LogicalEventId, LpId, NativeAdmissionCapability, NativeAdmissionMembership, NativeIntentKind,
    NativeOutboundSend, NativeRetirementRequest, OptimisticAuthority, OptimisticError,
    OptimisticLimits, OptimisticMessage, OptimisticMessageKind, OptimisticNativeCutReport,
    OptimisticOutboundStatus, OptimisticOwnedFailurePhase, OptimisticOwnedOptions,
    OptimisticOwnedStepKind, OptimisticProcess, OptimisticRuntime, OptimisticStateError,
    OptimisticStateToken, OptimisticTraceEntry, PartitionPlan, RemoteEvent, Tick,
};
use kairo_ecs_types::{EntityId, SimDuration};

const LP0: LpId = LpId(0);
const LP1: LpId = LpId(1);
const LP2: LpId = LpId(2);
const NAMESPACE: u128 = u128::MAX;
const EPOCH: u64 = u64::MAX;
type Runtime = OptimisticRuntime<Probe>;
type PeerHolder = Arc<Mutex<Option<Runtime>>>;

#[derive(Clone, Debug, Eq, PartialEq)]
enum ProbeAction {
    Restore(LpId),
    Handler(LpId, u128),
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Seen {
    event: RemoteEvent,
    value: u64,
    rng: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ProbeSnapshot {
    lp: LpId,
    value: u64,
    rng: u64,
    seen: Vec<Seen>,
}

#[derive(Clone)]
struct Probe {
    lp: LpId,
    value: u64,
    rng: u64,
    seen: Vec<Seen>,
    panic_lp1_once: Arc<AtomicBool>,
    snapshots: Arc<AtomicUsize>,
    drop_peer: Option<PeerHolder>,
    audit: Arc<Mutex<Vec<ProbeAction>>>,
}

impl Probe {
    fn new(lp: LpId, panic_lp1_once: Arc<AtomicBool>, audit: Arc<Mutex<Vec<ProbeAction>>>) -> Self {
        Self {
            lp,
            value: 100 + u64::from(lp.0),
            rng: 0x9e37_79b9_7f4a_7c15 ^ u64::from(lp.0),
            seen: Vec::new(),
            panic_lp1_once,
            snapshots: Arc::new(AtomicUsize::new(0)),
            drop_peer: None,
            audit,
        }
    }

    fn state(&self) -> ProbeSnapshot {
        ProbeSnapshot {
            lp: self.lp,
            value: self.value,
            rng: self.rng,
            seen: self.seen.clone(),
        }
    }
}

impl OptimisticProcess for Probe {
    type Snapshot = ProbeSnapshot;

    fn snapshot(&self) -> Self::Snapshot {
        self.snapshots.fetch_add(1, Ordering::Relaxed);
        self.state()
    }

    fn restore(&mut self, snapshot: &Self::Snapshot) -> Result<(), OptimisticStateError> {
        self.lp = snapshot.lp;
        self.value = snapshot.value;
        self.rng = snapshot.rng;
        self.seen.clone_from(&snapshot.seen);
        self.audit
            .lock()
            .unwrap()
            .push(ProbeAction::Restore(self.lp));
        Ok(())
    }

    fn on_event(&mut self, event: &RemoteEvent) -> Vec<RemoteEvent> {
        self.audit
            .lock()
            .unwrap()
            .push(ProbeAction::Handler(self.lp, event.tick.ticks()));
        let payload = &event.event_payload;
        self.value = self
            .value
            .wrapping_add(u64::from(payload.first().copied().unwrap_or_default()));
        self.rng = self
            .rng
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(event.tick.ticks() as u64)
            .wrapping_add(u64::from(payload.get(1).copied().unwrap_or_default()));
        self.seen.push(Seen {
            event: event.clone(),
            value: self.value,
            rng: self.rng,
        });

        if self.lp == LP1
            && payload.first() == Some(&u8::MAX)
            && self.panic_lp1_once.swap(false, Ordering::SeqCst)
        {
            panic!("held-out LP1 handler fault after mutating state");
        }

        if self.lp == LP0 && payload.first() == Some(&u8::MAX) {
            if let Some(holder) = &self.drop_peer {
                drop(holder.lock().unwrap().take());
            }
        }
        let mode = payload.get(1).copied().unwrap_or_default();
        if mode == 0 || (mode == 5 && !self.value.is_multiple_of(2)) {
            return Vec::new();
        }
        if mode == 4 {
            let delay = u128::from(payload.get(4).copied().unwrap_or(1));
            return [payload.get(2), payload.get(3)]
                .into_iter()
                .flatten()
                .map(|destination| RemoteEvent {
                    source_lp: self.lp,
                    dest_lp: LpId(u32::from(*destination)),
                    tick: Tick::from_ticks(event.tick.ticks() + delay),
                    event_payload: vec![self.value as u8, 0],
                })
                .collect();
        }
        if mode == 6 {
            let delay = u128::from(payload.get(4).copied().unwrap_or(1));
            let destinations = if self.value.is_multiple_of(2) {
                vec![payload.get(2), payload.get(3)]
            } else {
                vec![payload.get(2)]
            };
            return destinations
                .into_iter()
                .flatten()
                .map(|destination| RemoteEvent {
                    source_lp: self.lp,
                    dest_lp: LpId(u32::from(*destination)),
                    tick: Tick::from_ticks(event.tick.ticks() + delay),
                    event_payload: vec![self.value as u8, 7, LP2.0 as u8, 1],
                })
                .collect();
        }
        if mode == 7 {
            if self.lp != LP1 {
                return Vec::new();
            }
            return vec![RemoteEvent {
                source_lp: self.lp,
                dest_lp: LP2,
                tick: Tick::from_ticks(event.tick.ticks() + 1),
                event_payload: vec![self.value as u8, 0],
            }];
        }
        let destination = match mode {
            1 | 5 => LpId(u32::from(payload.get(2).copied().unwrap_or_default())),
            2 if self.value.is_multiple_of(2) => {
                LpId(u32::from(payload.get(2).copied().unwrap_or_default()))
            }
            2 => LpId(u32::from(payload.get(3).copied().unwrap_or_default())),
            3 => self.lp,
            _ => return Vec::new(),
        };
        let delay = if mode == 2 && !self.value.is_multiple_of(2) {
            payload
                .get(5)
                .or_else(|| payload.get(4))
                .copied()
                .map(u128::from)
                .unwrap_or(1)
        } else {
            u128::from(payload.get(4).copied().unwrap_or(1))
        };
        vec![RemoteEvent {
            source_lp: self.lp,
            dest_lp: destination,
            tick: Tick::from_ticks(event.tick.ticks() + delay),
            event_payload: vec![self.value as u8, 0],
        }]
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum FaultAction {
    Snapshot(LpId),
    Restore(LpId),
    Handler(LpId, u128),
    SnapshotDrop(LpId),
    PeerDropFromSnapshot(LpId),
    PeerDropFromSnapshotDrop(LpId),
}

#[derive(Clone, Copy)]
enum FaultPoint {
    SnapshotPanic,
    HandlerPanic,
    RestoreError,
    RestorePanic,
    PeerDropSnapshot,
}

#[derive(Default)]
struct FaultControl {
    snapshot_panic: Option<LpId>,
    handler_panic: Option<LpId>,
    restore_error: Option<LpId>,
    restore_panic: Option<LpId>,
    drop_panic: Option<LpId>,
    peer_drop_snapshot: Option<LpId>,
    peer_drop_snapshot_drop: Option<LpId>,
    next_snapshot_id: BTreeMap<LpId, u64>,
    drop_snapshot_target: Option<(LpId, u64)>,
    peer_drop_snapshot_target: Option<(LpId, u64)>,
}

type FaultRuntime = OptimisticRuntime<FaultProbe>;
type FaultPeerHolder = Arc<Mutex<Option<FaultRuntime>>>;
type FaultControlHandle = Arc<Mutex<FaultControl>>;
type FaultAudit = Arc<Mutex<Vec<FaultAction>>>;

struct FaultSnapshot {
    lp: LpId,
    value: u64,
    rng: u64,
    seen: Vec<Seen>,
    control: FaultControlHandle,
    audit: FaultAudit,
    peer_holder: Option<FaultPeerHolder>,
    drop_fault_eligible: bool,
    snapshot_id: u64,
}

impl Clone for FaultSnapshot {
    fn clone(&self) -> Self {
        let snapshot_id = {
            let mut control = self.control.lock().unwrap();
            let next = control.next_snapshot_id.entry(self.lp).or_insert(0);
            let id = *next;
            *next += 1;
            id
        };
        Self {
            lp: self.lp,
            value: self.value,
            rng: self.rng,
            seen: self.seen.clone(),
            control: Arc::clone(&self.control),
            audit: Arc::clone(&self.audit),
            peer_holder: self.peer_holder.clone(),
            drop_fault_eligible: false,
            snapshot_id,
        }
    }
}

impl Drop for FaultSnapshot {
    fn drop(&mut self) {
        self.audit
            .lock()
            .unwrap()
            .push(FaultAction::SnapshotDrop(self.lp));
        let (drop_peer, should_panic) = {
            let mut control = self.control.lock().unwrap();
            let targeted = control
                .drop_snapshot_target
                .is_none_or(|target| target == (self.lp, self.snapshot_id));
            let peer_targeted = control
                .peer_drop_snapshot_target
                .is_none_or(|target| target == (self.lp, self.snapshot_id));
            let drop_peer = self.drop_fault_eligible
                && peer_targeted
                && control.peer_drop_snapshot_drop == Some(self.lp);
            if drop_peer {
                control.peer_drop_snapshot_drop = None;
            }
            let should_panic =
                self.drop_fault_eligible && targeted && control.drop_panic == Some(self.lp);
            if should_panic {
                control.drop_panic = None;
            }
            (drop_peer, should_panic)
        };
        if drop_peer {
            self.audit
                .lock()
                .unwrap()
                .push(FaultAction::PeerDropFromSnapshotDrop(self.lp));
            if let Some(holder) = &self.peer_holder {
                drop(holder.lock().unwrap().take());
            }
        }
        if should_panic {
            panic!("held-out historical Snapshot::drop fault");
        }
    }
}

struct FaultProbe {
    lp: LpId,
    value: u64,
    rng: u64,
    seen: Vec<Seen>,
    control: FaultControlHandle,
    audit: FaultAudit,
    peer_holder: Option<FaultPeerHolder>,
}

impl FaultProbe {
    fn new(lp: LpId, control: FaultControlHandle, audit: FaultAudit) -> Self {
        Self {
            lp,
            value: 100 + u64::from(lp.0),
            rng: 0x9e37_79b9_7f4a_7c15 ^ u64::from(lp.0),
            seen: Vec::new(),
            control,
            audit,
            peer_holder: None,
        }
    }

    fn state(&self) -> ProbeSnapshot {
        ProbeSnapshot {
            lp: self.lp,
            value: self.value,
            rng: self.rng,
            seen: self.seen.clone(),
        }
    }

    fn take_fault(&self, point: FaultPoint) -> bool {
        let mut control = self.control.lock().unwrap();
        let flag = match point {
            FaultPoint::SnapshotPanic => &mut control.snapshot_panic,
            FaultPoint::HandlerPanic => &mut control.handler_panic,
            FaultPoint::RestoreError => &mut control.restore_error,
            FaultPoint::RestorePanic => &mut control.restore_panic,
            FaultPoint::PeerDropSnapshot => &mut control.peer_drop_snapshot,
        };
        let matches = *flag == Some(self.lp);
        if matches {
            *flag = None;
        }
        matches
    }
}

impl OptimisticProcess for FaultProbe {
    type Snapshot = FaultSnapshot;

    fn snapshot(&self) -> Self::Snapshot {
        self.audit
            .lock()
            .unwrap()
            .push(FaultAction::Snapshot(self.lp));
        let snapshot_id = {
            let mut control = self.control.lock().unwrap();
            let next = control.next_snapshot_id.entry(self.lp).or_insert(0);
            let id = *next;
            *next += 1;
            id
        };
        let drop_peer = self.take_fault(FaultPoint::PeerDropSnapshot);
        if drop_peer {
            self.audit
                .lock()
                .unwrap()
                .push(FaultAction::PeerDropFromSnapshot(self.lp));
            if let Some(holder) = &self.peer_holder {
                drop(holder.lock().unwrap().take());
            }
        }
        if self.take_fault(FaultPoint::SnapshotPanic) {
            panic!("held-out snapshot capture fault");
        }
        FaultSnapshot {
            lp: self.lp,
            value: self.value,
            rng: self.rng,
            seen: self.seen.clone(),
            control: Arc::clone(&self.control),
            audit: Arc::clone(&self.audit),
            peer_holder: self.peer_holder.clone(),
            drop_fault_eligible: true,
            snapshot_id,
        }
    }

    fn restore(&mut self, snapshot: &Self::Snapshot) -> Result<(), OptimisticStateError> {
        self.audit
            .lock()
            .unwrap()
            .push(FaultAction::Restore(self.lp));
        if self.take_fault(FaultPoint::RestorePanic) {
            panic!("held-out restore panic");
        }
        if self.take_fault(FaultPoint::RestoreError) {
            return Err(OptimisticStateError::new("held-out restore error"));
        }
        self.value = snapshot.value;
        self.rng = snapshot.rng;
        self.seen.clone_from(&snapshot.seen);
        Ok(())
    }

    fn on_event(&mut self, event: &RemoteEvent) -> Vec<RemoteEvent> {
        self.audit
            .lock()
            .unwrap()
            .push(FaultAction::Handler(self.lp, event.tick.ticks()));
        self.value = self.value.wrapping_add(u64::from(
            event.event_payload.first().copied().unwrap_or_default(),
        ));
        self.rng = self
            .rng
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(event.tick.ticks() as u64)
            .wrapping_add(u64::from(
                event.event_payload.get(1).copied().unwrap_or_default(),
            ));
        self.seen.push(Seen {
            event: event.clone(),
            value: self.value,
            rng: self.rng,
        });
        if self.take_fault(FaultPoint::HandlerPanic) {
            panic!("held-out handler panic after mutating state");
        }
        match event.event_payload.get(1).copied().unwrap_or_default() {
            1 => vec![RemoteEvent {
                source_lp: self.lp,
                dest_lp: LpId(u32::from(
                    event.event_payload.get(2).copied().unwrap_or_default(),
                )),
                tick: Tick::from_ticks(
                    event.tick.ticks()
                        + u128::from(event.event_payload.get(3).copied().unwrap_or(1)),
                ),
                event_payload: if self.lp == LP0 {
                    vec![self.value as u8, 1, LP2.0 as u8, 1]
                } else {
                    vec![self.value as u8, 0]
                },
            }],
            2 => vec![RemoteEvent {
                source_lp: LpId(self.lp.0.saturating_add(100)),
                dest_lp: LpId(u32::from(
                    event.event_payload.get(2).copied().unwrap_or_default(),
                )),
                tick: Tick::from_ticks(event.tick.ticks() + 1),
                event_payload: vec![1, 0],
            }],
            _ => Vec::new(),
        }
    }
}

fn fault_options(
    owned: &[LpId],
    max_pending: usize,
    max_receipts: usize,
) -> OptimisticOwnedOptions {
    let mut value = options(owned, max_pending);
    value.max_receipt_entries = max_receipts;
    value
}

fn new_fault_runtime(
    owned: &[LpId],
    max_pending: usize,
    max_receipts: usize,
    control: FaultControlHandle,
    audit: FaultAudit,
    peer_holder: Option<FaultPeerHolder>,
) -> FaultRuntime {
    let processes = owned
        .iter()
        .copied()
        .map(|lp| {
            let mut process = FaultProbe::new(lp, Arc::clone(&control), Arc::clone(&audit));
            process.peer_holder = peer_holder.clone();
            (lp, process)
        })
        .collect();
    OptimisticRuntime::new_owned(
        partition(),
        topology(),
        processes,
        fault_options(owned, max_pending, max_receipts),
    )
    .unwrap()
}

fn fault_world_three(
    max_pending: usize,
    max_receipts: usize,
    control: FaultControlHandle,
    audit: FaultAudit,
) -> (FaultRuntime, FaultRuntime, FaultRuntime) {
    let mut a = new_fault_runtime(
        &[LP0],
        max_pending,
        max_receipts,
        Arc::clone(&control),
        Arc::clone(&audit),
        None,
    );
    let mut b = new_fault_runtime(
        &[LP1],
        max_pending,
        max_receipts,
        Arc::clone(&control),
        Arc::clone(&audit),
        None,
    );
    let mut c = new_fault_runtime(
        &[LP2],
        max_pending,
        max_receipts,
        Arc::clone(&control),
        Arc::clone(&audit),
        None,
    );
    let authorities = [
        a.native_accounting_authority().unwrap(),
        b.native_accounting_authority().unwrap(),
        c.native_accounting_authority().unwrap(),
    ];
    a.register_native_peer(authorities[1].clone()).unwrap();
    a.register_native_peer(authorities[2].clone()).unwrap();
    b.register_native_peer(authorities[0].clone()).unwrap();
    b.register_native_peer(authorities[2].clone()).unwrap();
    c.register_native_peer(authorities[0].clone()).unwrap();
    c.register_native_peer(authorities[1].clone()).unwrap();
    a.seal_native_peers().unwrap();
    b.seal_native_peers().unwrap();
    c.seal_native_peers().unwrap();
    (a, b, c)
}

fn fault_close_three(a: &mut FaultRuntime, b: &mut FaultRuntime, c: &mut FaultRuntime) {
    a.close_initial_inputs().unwrap();
    b.close_initial_inputs().unwrap();
    c.close_initial_inputs().unwrap();
}

fn fault_drain_to_idle(runtime: &mut FaultRuntime, horizon: u128) {
    for _ in 0..32 {
        let progress = runtime
            .run_owned_until_with_budget(Tick::from_ticks(horizon), 1)
            .unwrap();
        if progress.budget_used == 0 {
            return;
        }
    }
    panic!("fault runtime did not become idle within the bounded fixture");
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Observation {
    report: kairo_ecs_pdes::OptimisticRuntimeReport,
    revision: u64,
    states: BTreeMap<LpId, ProbeSnapshot>,
    pending: BTreeMap<LpId, Option<Vec<OptimisticMessage>>>,
    tokens: BTreeMap<LpId, OptimisticStateToken>,
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

fn authorities() -> BTreeMap<LpId, OptimisticAuthority> {
    [LP0, LP1, LP2]
        .into_iter()
        .map(|lp| {
            (
                lp,
                OptimisticAuthority::Scoped {
                    simulation_namespace: NAMESPACE,
                    ownership_epoch: EPOCH,
                },
            )
        })
        .collect()
}

fn options(owned: &[LpId], max_pending: usize) -> OptimisticOwnedOptions {
    OptimisticOwnedOptions {
        simulation_namespace: NAMESPACE,
        current_authorities: authorities(),
        emission_epochs: owned.iter().map(|lp| (*lp, EPOCH)).collect(),
        local_limits: OptimisticLimits {
            max_pending_events: max_pending,
            ..OptimisticLimits::default()
        },
        max_global_lps: 3,
        max_outbox_entries: 64,
        max_transition_entries: 64,
        max_receipt_entries: 128,
    }
}

fn new_runtime(owned: &[LpId], max_pending: usize, panic_once: Arc<AtomicBool>) -> Runtime {
    new_runtime_with_peer_holder(owned, max_pending, panic_once, None)
}

fn new_runtime_with_peer_holder(
    owned: &[LpId],
    max_pending: usize,
    panic_once: Arc<AtomicBool>,
    peer_holder: Option<PeerHolder>,
) -> Runtime {
    new_runtime_with_bounds(owned, max_pending, 64, 64, 128, panic_once, peer_holder)
}

fn new_runtime_with_bounds(
    owned: &[LpId],
    max_pending: usize,
    max_outbox: usize,
    max_transitions: usize,
    max_receipts: usize,
    panic_once: Arc<AtomicBool>,
    peer_holder: Option<PeerHolder>,
) -> Runtime {
    let audit = Arc::new(Mutex::new(Vec::new()));
    let processes = owned
        .iter()
        .copied()
        .map(|lp| {
            let mut process = Probe::new(lp, Arc::clone(&panic_once), Arc::clone(&audit));
            process.drop_peer = peer_holder.clone();
            (lp, process)
        })
        .collect();
    let mut runtime_options = options(owned, max_pending);
    runtime_options.max_outbox_entries = max_outbox;
    runtime_options.max_transition_entries = max_transitions;
    runtime_options.max_receipt_entries = max_receipts;
    OptimisticRuntime::new_owned(partition(), topology(), processes, runtime_options).unwrap()
}

fn wire_three(a: &mut Runtime, b: &mut Runtime, c: &mut Runtime) {
    let authorities = [
        a.native_accounting_authority().unwrap(),
        b.native_accounting_authority().unwrap(),
        c.native_accounting_authority().unwrap(),
    ];
    a.register_native_peer(authorities[1].clone()).unwrap();
    a.register_native_peer(authorities[2].clone()).unwrap();
    b.register_native_peer(authorities[0].clone()).unwrap();
    b.register_native_peer(authorities[2].clone()).unwrap();
    c.register_native_peer(authorities[0].clone()).unwrap();
    c.register_native_peer(authorities[1].clone()).unwrap();
    a.seal_native_peers().unwrap();
    b.seal_native_peers().unwrap();
    c.seal_native_peers().unwrap();
}

fn close_three(a: &mut Runtime, b: &mut Runtime, c: &mut Runtime) {
    a.close_initial_inputs().unwrap();
    b.close_initial_inputs().unwrap();
    c.close_initial_inputs().unwrap();
}

fn three_world(max_pending: usize) -> (Runtime, Runtime, Runtime) {
    let panic_once = Arc::new(AtomicBool::new(false));
    let mut a = new_runtime(&[LP0], max_pending, Arc::clone(&panic_once));
    let mut b = new_runtime(&[LP1], max_pending, Arc::clone(&panic_once));
    let mut c = new_runtime(&[LP2], max_pending, panic_once);
    wire_three(&mut a, &mut b, &mut c);
    (a, b, c)
}

fn all_local(max_pending: usize) -> Runtime {
    let panic_once = Arc::new(AtomicBool::new(false));
    let mut runtime = new_runtime(&[LP0, LP1, LP2], max_pending, panic_once);
    runtime.seal_native_peers().unwrap();
    runtime
}

fn pair_world(max_pending: usize) -> (Runtime, Runtime) {
    let panic_once = Arc::new(AtomicBool::new(false));
    let mut local = new_runtime(&[LP0, LP1], max_pending, Arc::clone(&panic_once));
    let mut remote = new_runtime(&[LP2], max_pending, panic_once);
    let local_authority = local.native_accounting_authority().unwrap();
    let remote_authority = remote.native_accounting_authority().unwrap();
    local
        .register_native_peer(remote_authority.clone())
        .unwrap();
    remote
        .register_native_peer(local_authority.clone())
        .unwrap();
    local.seal_native_peers().unwrap();
    remote.seal_native_peers().unwrap();
    (local, remote)
}

fn close_pair(local: &mut Runtime, remote: &mut Runtime) {
    local.close_initial_inputs().unwrap();
    remote.close_initial_inputs().unwrap();
}

fn observe(runtime: &Runtime, owned: &[LpId]) -> Observation {
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
    }
}

fn event(source: LpId, destination: LpId, tick: u128, payload: &[u8]) -> RemoteEvent {
    RemoteEvent {
        source_lp: source,
        dest_lp: destination,
        tick: Tick::from_ticks(tick),
        event_payload: payload.to_vec(),
    }
}

fn take_root_ticket(
    sends: &mut Vec<NativeOutboundSend>,
    source: LpId,
    sequence: u64,
) -> NativeOutboundSend {
    let index = sends
        .iter()
        .position(|send| send.message().logical_id().root_parts() == Some((source, sequence)))
        .expect("expected root ticket is in source outbox");
    sends.remove(index)
}

fn deliver_positive(sender: &mut Runtime, receiver: &mut Runtime, send: &NativeOutboundSend) {
    assert_eq!(send.message().kind(), OptimisticMessageKind::Positive);
    let capability = receiver.admit_native(send).unwrap();
    sender.acknowledge_native_admission(capability).unwrap();
}

fn deliver_retirement_request(
    sender: &mut Runtime,
    receiver: &mut Runtime,
    request: &NativeRetirementRequest,
) -> NativeAdmissionCapability {
    let capability = receiver.receive_native_retirement(request).unwrap();
    sender
        .acknowledge_native_admission(capability.clone())
        .unwrap();
    capability
}

fn drain_one(runtime: &mut Runtime, horizon: u128) -> kairo_ecs_pdes::OptimisticRunProgress {
    runtime
        .run_until_with_budget(Tick::from_ticks(horizon), 1)
        .unwrap()
}

fn drain_to_idle(runtime: &mut Runtime, horizon: u128) {
    for _ in 0..64 {
        let progress = drain_one(runtime, horizon);
        // Authoritative remote/blocked obligations remain pending until transport
        // applies and acknowledges them; only runnable local work drains here.
        if progress.budget_used == 0 && !progress.budget_exhausted {
            return;
        }
    }
    panic!("runtime did not become idle within the bounded work fixture");
}

fn trace_rows(cut: &OptimisticNativeCutReport) -> Vec<OptimisticTraceEntry> {
    let mut rows = cut
        .reports()
        .iter()
        .flat_map(|report| report.collected.iter().cloned())
        .collect::<Vec<_>>();
    rows.sort_by_key(|entry| (entry.lp_id, entry.event.tick, entry.logical_id.clone()));
    rows
}

#[test]
fn budget_one_rollback_replays_the_same_seeded_state_and_committed_trace_as_all_local_control() {
    let (mut a, mut b, mut c) = three_world(64);
    let future = event(LP2, LP0, 10, &[4, 0]);
    let straggler = event(LP2, LP0, 5, &[3, 0]);
    c.schedule_initial(900, future.clone()).unwrap();
    c.schedule_initial(100, straggler.clone()).unwrap();
    let mut root_sends = c.ready_native_sends().unwrap();
    let future_send = take_root_ticket(&mut root_sends, LP2, 900);
    let straggler_send = take_root_ticket(&mut root_sends, LP2, 100);

    let mut control = all_local(64);
    control.schedule_initial(900, future).unwrap();
    control.schedule_initial(100, straggler).unwrap();
    close_three(&mut a, &mut b, &mut c);
    control.close_initial_inputs().unwrap();

    deliver_positive(&mut c, &mut a, &future_send);
    let baseline = observe(&a, &[LP0]);
    let future_progress = drain_one(&mut a, 20);
    assert_eq!(future_progress.budget_used, 1);
    assert_eq!(
        a.process_at(LP0).unwrap().value,
        baseline.states[&LP0].value + 4
    );
    assert_eq!(a.report().executions, 1);

    let token_before_straggler = a.state_token(LP0).unwrap();
    let straggler_cap = a.admit_native(&straggler_send).unwrap();
    c.acknowledge_native_admission(straggler_cap.clone())
        .unwrap();
    let before_rollback = observe(&a, &[LP0]);
    let rollback_before = before_rollback.report.rollback_attempts;
    let rollback = drain_one(&mut a, 20);
    assert_eq!(rollback.budget_used, 1);
    assert_eq!(a.report().rollback_attempts, rollback_before + 1);
    assert_eq!(a.process_at(LP0).unwrap().state(), baseline.states[&LP0]);
    assert!(!a.validate_state_token(token_before_straggler));
    assert_eq!(a.pending_events(LP0).unwrap().len(), 2);

    drain_to_idle(&mut a, 20);
    let control_progress = control
        .run_until_with_budget(Tick::from_ticks(20), 16)
        .unwrap();
    assert_eq!(control_progress.pending_positives, 0);
    assert_eq!(control_progress.pending_antis, 0);
    assert_eq!(
        a.process_at(LP0).unwrap().state(),
        control.process_at(LP0).unwrap().state()
    );
    assert_eq!(a.report().executions, 3);
    assert_eq!(a.report().replay_executions, 1);
    assert_eq!(control.report().executions, 2);
    assert_eq!(control.report().replay_executions, 0);
    assert!(a.pending_events(LP0).unwrap().is_empty());

    let split_cut = OptimisticRuntime::fossil_collect_native_group(
        &mut [&mut a, &mut b, &mut c],
        Tick::from_ticks(21),
    )
    .unwrap();
    let control_cut =
        OptimisticRuntime::fossil_collect_native_group(&mut [&mut control], Tick::from_ticks(21))
            .unwrap();
    assert_eq!(trace_rows(&split_cut), trace_rows(&control_cut));
    let before_late_ticket = observe(&a, &[LP0]);
    assert!(matches!(
        a.admit_native(&straggler_send),
        Err(OptimisticError::EventBeforeGvt { event_tick, gvt })
            if event_tick == Tick::from_ticks(5) && gvt == Tick::from_ticks(21)
    ));
    assert_eq!(observe(&a, &[LP0]), before_late_ticket);
    let before_late_ack = observe(&c, &[LP2]);
    assert!(matches!(
        c.acknowledge_native_admission(straggler_cap),
        Err(OptimisticError::EventBeforeGvt { event_tick, gvt })
            if event_tick == Tick::from_ticks(5) && gvt == Tick::from_ticks(21)
    ));
    assert_eq!(observe(&c, &[LP2]), before_late_ack);
    let expected_ids = BTreeSet::from([
        LogicalEventId::root(LP2, 100),
        LogicalEventId::root(LP2, 900),
    ]);
    assert_eq!(
        trace_rows(&split_cut)
            .into_iter()
            .map(|entry| entry.logical_id)
            .collect::<BTreeSet<_>>(),
        expected_ids
    );
}

#[test]
fn selected_round_compensates_every_lp_when_a_later_handler_panics() {
    let panic_once = Arc::new(AtomicBool::new(true));
    let mut runtime = new_runtime(&[LP0, LP1, LP2], 32, Arc::clone(&panic_once));
    runtime.seal_native_peers().unwrap();
    runtime
        .schedule_initial(1, event(LP0, LP0, 2, &[7, 0]))
        .unwrap();
    runtime
        .schedule_initial(2, event(LP1, LP1, 3, &[u8::MAX, 0]))
        .unwrap();
    runtime.close_initial_inputs().unwrap();
    let before = observe(&runtime, &[LP0, LP1, LP2]);

    let failure = runtime
        .run_owned_until_with_budget(Tick::from_ticks(10), 2)
        .unwrap_err();
    assert_eq!(failure.phase(), OptimisticOwnedFailurePhase::Compensated);
    assert_eq!(failure.cause(), &OptimisticError::HandlerPanicked(LP1));
    assert_eq!(failure.progress().budget_used, 2);
    assert!(failure.progress().published_messages.is_empty());
    assert_eq!(failure.attempted().len(), 1);
    assert_eq!(
        failure.attempted()[0].kind(),
        OptimisticOwnedStepKind::PositiveRound
    );
    assert_eq!(failure.attempted()[0].selected().len(), 2);
    assert_eq!(failure.compensated_lps(), &[LP0, LP1]);
    let after = observe(&runtime, &[LP0, LP1, LP2]);
    assert_eq!(after.states, before.states);
    assert_eq!(after.pending, before.pending);
    assert_eq!(after.report.executions, before.report.executions);
    assert_eq!(after.report.pending_events, before.report.pending_events);
    assert_eq!(after.revision, before.revision + 1);
    assert!(!runtime.validate_state_token(before.tokens[&LP0]));
    assert!(!runtime.validate_state_token(before.tokens[&LP1]));
    assert!(runtime.validate_state_token(before.tokens[&LP2]));

    let retried = runtime
        .run_until_with_budget(Tick::from_ticks(10), 2)
        .unwrap();
    assert_eq!(retried.budget_used, 2);
    assert_eq!(retried.pending_positives, 0);
    assert_eq!(runtime.report().executions, 2);
    assert_eq!(runtime.process_at(LP0).unwrap().value, 107);
    assert_eq!(
        runtime.process_at(LP1).unwrap().value,
        100 + u64::from(LP1.0) + u64::from(u8::MAX)
    );
}

#[test]
fn replacement_n2_waits_for_both_real_receiver_retirements_in_both_proof_orders() {
    replacement_chain_scenario(false);
    replacement_chain_scenario(true);
}

fn replacement_chain_scenario(n1_first: bool) {
    let (mut a, mut b, mut c) = three_world(64);
    let driver = event(LP2, LP0, 1, &[0, 2, 1, 2, 9]);
    let earlier = event(LP2, LP0, 0, &[1, 0]);
    let still_earlier = event(LP2, LP0, 0, &[1, 0]);
    c.schedule_initial(900, driver.clone()).unwrap();
    c.schedule_initial(100, earlier.clone()).unwrap();
    c.schedule_initial(50, still_earlier.clone()).unwrap();
    let mut roots = c.ready_native_sends().unwrap();
    let driver_send = take_root_ticket(&mut roots, LP2, 900);
    let earlier_send = take_root_ticket(&mut roots, LP2, 100);
    let still_earlier_send = take_root_ticket(&mut roots, LP2, 50);

    let mut control = all_local(64);
    control.schedule_initial(900, driver).unwrap();
    control.schedule_initial(100, earlier).unwrap();
    control.schedule_initial(50, still_earlier).unwrap();
    close_three(&mut a, &mut b, &mut c);
    control.close_initial_inputs().unwrap();

    // Initial even state emits P to LP1; keep the exact ticket for the trace oracle.
    deliver_positive(&mut c, &mut a, &driver_send);
    drain_to_idle(&mut a, 30);
    let p_send = a
        .ready_native_sends()
        .unwrap()
        .into_iter()
        .find(|send| {
            send.message().kind() == OptimisticMessageKind::Positive
                && send.message().event().dest_lp == LP1
                && send.message().event().tick == Tick::from_ticks(10)
        })
        .expect("first version P is emitted to LP1");
    let p_message = p_send.message().clone();
    deliver_positive(&mut a, &mut b, &p_send);
    drain_to_idle(&mut b, 20);
    assert_eq!(b.process_at(LP1).unwrap().value, 201);

    // The first straggler makes the parent replay emit N1 to LP2. Leave P's anti unapplied.
    deliver_positive(&mut c, &mut a, &earlier_send);
    assert_eq!(drain_one(&mut a, 30).budget_used, 1); // rollback-only unit
    drain_to_idle(&mut a, 30);
    let after_n1 = a.native_intents().unwrap();
    let p_intent = after_n1
        .iter()
        .find(|intent| {
            intent.kind() == NativeIntentKind::Present
                && intent.message().is_some_and(|message| {
                    message.logical_id() == p_message.logical_id()
                        && message.incarnation() == p_message.incarnation()
                })
        })
        .expect("P remains as the predecessor intent")
        .id()
        .clone();
    assert_eq!(
        after_n1
            .iter()
            .filter(|intent| intent.kind() == NativeIntentKind::Absent)
            .count(),
        1
    );
    assert!(after_n1
        .iter()
        .any(|intent| intent.predecessor() == Some(&p_intent)));
    let n1 = a
        .outbound_pending()
        .unwrap()
        .into_iter()
        .find(|view| {
            view.message().kind() == OptimisticMessageKind::Positive
                && view.message().logical_id() == p_message.logical_id()
                && view.message().event().dest_lp == LP2
        })
        .expect("N1 is retained behind P retirement")
        .message()
        .clone();
    let request_p = a
        .pending_native_retirement_requests()
        .unwrap()
        .into_iter()
        .find(|request| request.predecessor_positive().incarnation() == p_message.incarnation())
        .expect("P has one unresolved retirement request");

    // A second, earlier same-tick straggler supersedes N1 before either receiver applies its anti.
    deliver_positive(&mut c, &mut a, &still_earlier_send);
    assert_eq!(drain_one(&mut a, 30).budget_used, 1);
    drain_to_idle(&mut a, 30);
    let n2_view = a
        .outbound_pending()
        .unwrap()
        .into_iter()
        .find(|view| {
            view.message().kind() == OptimisticMessageKind::Positive
                && view.message().logical_id() == p_message.logical_id()
                && view.message().event().dest_lp == LP1
                && view.message().incarnation() > n1.incarnation()
        })
        .expect("N2 is the current version")
        .clone();
    assert_eq!(
        n2_view.status(),
        OptimisticOutboundStatus::BlockedReplacement
    );
    assert_eq!(n2_view.retirement_dependencies().len(), 2);
    assert!(a.ready_native_sends().unwrap().iter().all(|send| {
        send.message().kind() != OptimisticMessageKind::Positive
            || send.message().incarnation() < n2_view.message().incarnation()
    }));
    let request_n1 = a
        .pending_native_retirement_requests()
        .unwrap()
        .into_iter()
        .find(|request| request.predecessor_positive().incarnation() == n1.incarnation())
        .expect("superseded N1 has its own exact retirement request");
    assert_ne!(request_n1.transition_id(), request_p.transition_id());
    assert_eq!(
        request_n1.old_receiver().runtime_id(),
        c.native_accounting_authority().unwrap().runtime_id()
    );

    if n1_first {
        // N1 was never positively sent. Its real receiver records the
        // tombstone first, but that proof cannot release N2 while P remains.
        let anti_n1 = deliver_retirement_request(&mut a, &mut c, &request_n1);
        assert_eq!(anti_n1.message().kind(), OptimisticMessageKind::Anti);
        assert_eq!(drain_one(&mut c, 30).budget_used, 1);
        assert_eq!(c.report().tombstones, 1);
        let applied_n1 = c.applied_native_retirements().unwrap().pop().unwrap();
        a.acknowledge_native_retirement(applied_n1.clone()).unwrap();
        let after_n1_proof = observe(&a, &[LP0]);
        let applied_n1_retry = c.applied_native_retirements().unwrap().pop().unwrap();
        assert_eq!(applied_n1_retry.transition_id(), applied_n1.transition_id());
        assert_eq!(
            applied_n1_retry.recorded_revision(),
            applied_n1.recorded_revision()
        );
        a.acknowledge_native_retirement(applied_n1_retry).unwrap();
        assert_eq!(observe(&a, &[LP0]), after_n1_proof);
        let still_blocked = a
            .outbound_pending()
            .unwrap()
            .into_iter()
            .find(|view| view.intent_id() == n2_view.intent_id())
            .unwrap();
        assert_eq!(
            still_blocked.status(),
            OptimisticOutboundStatus::BlockedReplacement
        );
        assert_eq!(
            still_blocked.retirement_dependencies(),
            &[request_p.transition_id().clone()]
        );

        let anti_p = deliver_retirement_request(&mut a, &mut b, &request_p);
        assert_eq!(anti_p.message().kind(), OptimisticMessageKind::Anti);
        assert_eq!(drain_one(&mut b, 30).budget_used, 1);
        let applied_p = b.applied_native_retirements().unwrap().pop().unwrap();
        a.acknowledge_native_retirement(applied_p).unwrap();
    } else {
        // P's real receiver applies its anti first; N2 stays blocked on N1's
        // independent absence until LP2 records that anti.
        let anti_p = deliver_retirement_request(&mut a, &mut b, &request_p);
        assert_eq!(anti_p.message().kind(), OptimisticMessageKind::Anti);
        assert_eq!(drain_one(&mut b, 30).budget_used, 1);
        let applied_p = b.applied_native_retirements().unwrap().pop().unwrap();
        a.acknowledge_native_retirement(applied_p.clone()).unwrap();
        let after_first_proof_ack = observe(&a, &[LP0]);
        let applied_p_retry = b.applied_native_retirements().unwrap().pop().unwrap();
        assert_eq!(applied_p_retry.transition_id(), applied_p.transition_id());
        assert_eq!(
            applied_p_retry.recorded_revision(),
            applied_p.recorded_revision()
        );
        a.acknowledge_native_retirement(applied_p_retry).unwrap();
        assert_eq!(observe(&a, &[LP0]), after_first_proof_ack);
        let after_p_applied = a
            .outbound_pending()
            .unwrap()
            .into_iter()
            .find(|view| view.intent_id() == n2_view.intent_id())
            .unwrap();
        assert_eq!(
            after_p_applied.status(),
            OptimisticOutboundStatus::BlockedReplacement
        );
        assert_eq!(after_p_applied.retirement_dependencies().len(), 1);

        // N1 was never sent as positive: LP2 still records its anti/tombstone.
        let anti_n1 = deliver_retirement_request(&mut a, &mut c, &request_n1);
        assert_eq!(anti_n1.message().kind(), OptimisticMessageKind::Anti);
        assert_eq!(drain_one(&mut c, 30).budget_used, 1);
        assert_eq!(c.report().tombstones, 1);
        let applied_n1 = c.applied_native_retirements().unwrap().pop().unwrap();
        a.acknowledge_native_retirement(applied_n1).unwrap();
    }
    let released_n2 = a
        .outbound_pending()
        .unwrap()
        .into_iter()
        .find(|view| view.intent_id() == n2_view.intent_id())
        .unwrap();
    assert_eq!(released_n2.status(), OptimisticOutboundStatus::Ready);
    let n2_send = a
        .ready_native_sends()
        .unwrap()
        .into_iter()
        .find(|send| send.message().incarnation() == n2_view.message().incarnation())
        .unwrap();
    deliver_positive(&mut a, &mut b, &n2_send);
    drain_to_idle(&mut b, 30);
    assert_eq!(b.process_at(LP1).unwrap().value, 203);

    drain_to_idle(&mut control, 30);
    for lp in [LP0, LP1, LP2] {
        let split = match lp {
            LP0 => a.process_at(lp).unwrap().state(),
            LP1 => b.process_at(lp).unwrap().state(),
            LP2 => c.process_at(lp).unwrap().state(),
            _ => unreachable!(),
        };
        assert_eq!(split, control.process_at(lp).unwrap().state());
    }
    let split_cut = OptimisticRuntime::fossil_collect_native_group(
        &mut [&mut a, &mut b, &mut c],
        Tick::from_ticks(31),
    )
    .unwrap();
    let control_cut =
        OptimisticRuntime::fossil_collect_native_group(&mut [&mut control], Tick::from_ticks(31))
            .unwrap();
    assert_eq!(trace_rows(&split_cut), trace_rows(&control_cut));
}

#[test]
fn absent_replay_coalesces_one_retirement_and_reintroduction_waits_for_applied_proof() {
    let (mut a, mut b, mut c) = three_world(64);
    let driver = event(LP2, LP0, 1, &[0, 5, 1, 2, 9]);
    let first_absence = event(LP2, LP0, 0, &[1, 0]);
    let repeated_absence = event(LP2, LP0, 0, &[2, 0]);
    let reintroduction = event(LP2, LP0, 0, &[1, 0]);
    c.schedule_initial(900, driver.clone()).unwrap();
    c.schedule_initial(100, first_absence.clone()).unwrap();
    c.schedule_initial(50, repeated_absence.clone()).unwrap();
    c.schedule_initial(0, reintroduction.clone()).unwrap();
    let mut roots = c.ready_native_sends().unwrap();
    let driver_send = take_root_ticket(&mut roots, LP2, 900);
    let first_send = take_root_ticket(&mut roots, LP2, 100);
    let repeated_send = take_root_ticket(&mut roots, LP2, 50);
    let reintroduced_send = take_root_ticket(&mut roots, LP2, 0);

    let mut control = all_local(64);
    control.schedule_initial(900, driver).unwrap();
    control.schedule_initial(100, first_absence).unwrap();
    control.schedule_initial(50, repeated_absence).unwrap();
    control.schedule_initial(0, reintroduction).unwrap();
    close_three(&mut a, &mut b, &mut c);
    control.close_initial_inputs().unwrap();

    deliver_positive(&mut c, &mut a, &driver_send);
    drain_to_idle(&mut a, 30);
    let p_send = a
        .ready_native_sends()
        .unwrap()
        .into_iter()
        .find(|send| send.message().kind() == OptimisticMessageKind::Positive)
        .expect("initial output P is ready");
    let p_message = p_send.message().clone();
    // Keep the positive admission capability without its transport ACK. The
    // later applied-retirement proof must close this exact sender obligation.
    let old_p_cap = b.admit_native(&p_send).unwrap();

    deliver_positive(&mut c, &mut a, &first_send);
    assert_eq!(drain_one(&mut a, 30).budget_used, 1);
    drain_to_idle(&mut a, 30);
    let absent = a
        .native_intents()
        .unwrap()
        .into_iter()
        .find(|intent| {
            intent.kind() == NativeIntentKind::Absent
                && intent.predecessor().is_some_and(|id| {
                    a.native_intents().unwrap().iter().any(|parent| {
                        parent.id() == id
                            && parent.message().is_some_and(|message| {
                                message.logical_id() == p_message.logical_id()
                            })
                    })
                })
        })
        .expect("first reduced replay records P as absent");
    let absent_id = absent.id().clone();
    let request_id = absent.retirement_request().unwrap().transition_id().clone();
    assert_eq!(absent_id, request_id);
    let request_count = a.pending_native_retirement_requests().unwrap().len();
    assert_eq!(request_count, 1);

    // A second actual handler replay still emits nothing and must reuse the same absence head.
    deliver_positive(&mut c, &mut a, &repeated_send);
    assert_eq!(drain_one(&mut a, 30).budget_used, 1);
    drain_to_idle(&mut a, 30);
    let intents = a.native_intents().unwrap();
    let absences = intents
        .iter()
        .filter(|intent| intent.kind() == NativeIntentKind::Absent)
        .collect::<Vec<_>>();
    assert_eq!(absences.len(), 1);
    assert_eq!(absences[0].id(), &absent_id);
    assert_eq!(
        absences[0].retirement_request().unwrap().transition_id(),
        &request_id
    );
    assert_eq!(a.pending_native_retirement_requests().unwrap().len(), 1);
    assert_eq!(
        a.ready_native_sends()
            .unwrap()
            .iter()
            .filter(|send| send.message().kind() == OptimisticMessageKind::Anti)
            .count(),
        1
    );

    // A later even-state replay reintroduces the same logical output as a new present version.
    deliver_positive(&mut c, &mut a, &reintroduced_send);
    assert_eq!(drain_one(&mut a, 30).budget_used, 1);
    drain_to_idle(&mut a, 30);
    let n_view = a
        .outbound_pending()
        .unwrap()
        .into_iter()
        .find(|view| {
            view.message().kind() == OptimisticMessageKind::Positive
                && view.message().logical_id() == p_message.logical_id()
                && view.message().incarnation() > p_message.incarnation()
        })
        .expect("reintroduced version follows the absent node");
    assert_eq!(
        n_view.status(),
        OptimisticOutboundStatus::BlockedReplacement
    );
    assert_eq!(n_view.retirement_dependencies(), &[absent_id]);

    let request = a
        .pending_native_retirement_requests()
        .unwrap()
        .pop()
        .unwrap();
    let anti_send = a
        .ready_native_sends()
        .unwrap()
        .into_iter()
        .find(|send| send.message().kind() == OptimisticMessageKind::Anti)
        .expect("the anti ticket carries the canonical request");
    assert_eq!(
        anti_send.retirement_request().unwrap().transition_id(),
        request.transition_id()
    );
    // The anti admission is also deliberately left unacknowledged. Applying
    // it and returning the receiver's proof is sufficient to close both slots.
    let anti_cap = b.admit_native(&anti_send).unwrap();
    assert_eq!(anti_cap.message().kind(), OptimisticMessageKind::Anti);
    assert_eq!(b.pending_events(LP1).unwrap().len(), 2);
    assert_eq!(drain_one(&mut b, 30).budget_used, 1);
    assert_eq!(b.pending_events(LP1).unwrap().len(), 0);
    let applied = b.applied_native_retirements().unwrap().pop().unwrap();
    let before_proof_ack = observe(&a, &[LP0]);
    a.acknowledge_native_retirement(applied.clone()).unwrap();
    let after_proof_ack = observe(&a, &[LP0]);
    assert_ne!(after_proof_ack.revision, before_proof_ack.revision);
    let receiver_after_apply = observe(&b, &[LP1]);
    let old_positive_retry = b.admit_native(&p_send).unwrap();
    let old_anti_retry = b.admit_native(&anti_send).unwrap();
    assert_eq!(
        old_positive_retry.recorded_membership(),
        old_p_cap.recorded_membership()
    );
    assert_eq!(
        old_positive_retry.recorded_revision(),
        old_p_cap.recorded_revision()
    );
    assert_eq!(
        old_anti_retry.recorded_membership(),
        anti_cap.recorded_membership()
    );
    assert_eq!(
        old_anti_retry.recorded_revision(),
        anti_cap.recorded_revision()
    );
    assert_eq!(observe(&b, &[LP1]), receiver_after_apply);
    a.acknowledge_native_admission(old_p_cap.clone()).unwrap();
    a.acknowledge_native_admission(anti_cap.clone()).unwrap();
    a.acknowledge_native_retirement(applied.clone()).unwrap();
    assert_eq!(observe(&a, &[LP0]), after_proof_ack);
    assert_eq!(observe(&b, &[LP1]), receiver_after_apply);
    let released = a
        .outbound_pending()
        .unwrap()
        .into_iter()
        .find(|view| view.intent_id() == n_view.intent_id())
        .unwrap();
    assert_eq!(released.status(), OptimisticOutboundStatus::Ready);
    let n_send = a
        .ready_native_sends()
        .unwrap()
        .into_iter()
        .find(|send| send.message().incarnation() == n_view.message().incarnation())
        .unwrap();
    deliver_positive(&mut a, &mut b, &n_send);
    let original_retry = b.admit_native(&p_send).unwrap();
    assert_eq!(
        original_retry.recorded_membership(),
        old_p_cap.recorded_membership()
    );
    assert_eq!(
        original_retry.recorded_revision(),
        old_p_cap.recorded_revision()
    );
    drain_to_idle(&mut b, 30);
    assert_eq!(b.process_at(LP1).unwrap().value, 205);

    drain_to_idle(&mut control, 30);
    for lp in [LP0, LP1, LP2] {
        let split = match lp {
            LP0 => a.process_at(lp).unwrap().state(),
            LP1 => b.process_at(lp).unwrap().state(),
            LP2 => c.process_at(lp).unwrap().state(),
            _ => unreachable!(),
        };
        assert_eq!(split, control.process_at(lp).unwrap().state());
    }
    let split_cut = OptimisticRuntime::fossil_collect_native_group(
        &mut [&mut a, &mut b, &mut c],
        Tick::from_ticks(10),
    )
    .unwrap();
    let control_cut =
        OptimisticRuntime::fossil_collect_native_group(&mut [&mut control], Tick::from_ticks(10))
            .unwrap();
    assert_eq!(trace_rows(&split_cut), trace_rows(&control_cut));
    // Equality with each referenced envelope tick retains exact retries.
    let at_floor_a = observe(&a, &[LP0]);
    let at_floor_b = observe(&b, &[LP1]);
    let at_floor_positive = b.admit_native(&p_send).unwrap();
    let at_floor_anti = b.admit_native(&anti_send).unwrap();
    assert_eq!(
        at_floor_positive.recorded_revision(),
        old_p_cap.recorded_revision()
    );
    assert_eq!(
        at_floor_anti.recorded_revision(),
        anti_cap.recorded_revision()
    );
    a.acknowledge_native_admission(old_p_cap.clone()).unwrap();
    a.acknowledge_native_admission(anti_cap.clone()).unwrap();
    a.acknowledge_native_retirement(applied.clone()).unwrap();
    let retained_proof = b
        .applied_native_retirements()
        .unwrap()
        .into_iter()
        .find(|proof| proof.transition_id() == applied.transition_id())
        .unwrap();
    assert_eq!(
        retained_proof.recorded_revision(),
        applied.recorded_revision()
    );
    assert_eq!(retained_proof.applied_effect(), applied.applied_effect());
    assert_eq!(observe(&a, &[LP0]), at_floor_a);
    assert_eq!(observe(&b, &[LP1]), at_floor_b);
    let above_floor = OptimisticRuntime::fossil_collect_native_group(
        &mut [&mut a, &mut b, &mut c],
        Tick::from_ticks(11),
    )
    .unwrap();
    assert_eq!(above_floor.committed_gvt(), Tick::from_ticks(11));
    let after_above_a = observe(&a, &[LP0]);
    let after_above_b = observe(&b, &[LP1]);
    assert!(matches!(
        b.admit_native(&p_send),
        Err(OptimisticError::EventBeforeGvt { .. })
    ));
    assert!(matches!(
        b.admit_native(&anti_send),
        Err(OptimisticError::EventBeforeGvt { .. })
    ));
    assert!(matches!(
        a.acknowledge_native_admission(old_p_cap.clone()),
        Err(OptimisticError::EventBeforeGvt { .. })
    ));
    assert!(matches!(
        a.acknowledge_native_admission(anti_cap.clone()),
        Err(OptimisticError::EventBeforeGvt { .. })
    ));
    assert!(matches!(
        a.acknowledge_native_retirement(applied),
        Err(OptimisticError::EventBeforeGvt { .. })
    ));
    assert_eq!(observe(&a, &[LP0]), after_above_a);
    assert_eq!(observe(&b, &[LP1]), after_above_b);
}

#[test]
fn changed_destination_uses_reserved_local_slot_and_gvt_keeps_old30_tombstone_for_new25() {
    let (mut local, mut remote) = pair_world(2);
    let parent = event(LP0, LP0, 15, &[0, 2, 2, 1, 15, 10]);
    let straggler = event(LP2, LP0, 10, &[1, 0]);
    local.schedule_initial(77, parent.clone()).unwrap();
    remote.schedule_initial(88, straggler.clone()).unwrap();
    let mut remote_roots = remote.ready_native_sends().unwrap();
    let straggler_send = take_root_ticket(&mut remote_roots, LP2, 88);
    close_pair(&mut local, &mut remote);

    assert_eq!(drain_one(&mut local, 40).budget_used, 1);
    let old_p = local
        .ready_native_sends()
        .unwrap()
        .into_iter()
        .find(|send| send.message().event().tick == Tick::from_ticks(30))
        .expect("even parent version P30 escapes to LP2");
    assert_eq!(old_p.message().event().dest_lp, LP2);
    let before_rejected_cut_local = observe(&local, &[LP0, LP1]);
    let before_rejected_cut_remote = observe(&remote, &[LP2]);
    let rejected_cut = OptimisticRuntime::fossil_collect_native_group(
        &mut [&mut local, &mut remote],
        Tick::from_ticks(31),
    )
    .unwrap_err();
    assert!(matches!(
        rejected_cut.cause(),
        OptimisticError::GvtBeyondPending { requested, .. }
            if *requested == Tick::from_ticks(31)
    ));
    assert_eq!(observe(&local, &[LP0, LP1]), before_rejected_cut_local);
    assert_eq!(observe(&remote, &[LP2]), before_rejected_cut_remote);
    deliver_positive(&mut remote, &mut local, &straggler_send);
    assert_eq!(drain_one(&mut local, 40).budget_used, 1); // rollback parent P30
    assert_eq!(drain_one(&mut local, 40).budget_used, 1); // execute straggler10
    assert_eq!(drain_one(&mut local, 40).budget_used, 1); // replay parent15 into blocked N25
    let blocked_wait = local
        .run_until_with_budget(Tick::from_ticks(40), 1)
        .unwrap();
    assert_eq!(blocked_wait.budget_used, 0);
    assert_eq!(blocked_wait.pending_positives, 2); // retained remote P plus blocked local N
    assert_eq!(
        local
            .outbound_pending()
            .unwrap()
            .iter()
            .filter(|view| { view.message().kind() == OptimisticMessageKind::Positive })
            .count(),
        1
    );
    assert_eq!(
        local
            .accounting_snapshot()
            .unwrap()
            .reserved_pending_count(),
        1
    );

    let n = local
        .native_intents()
        .unwrap()
        .into_iter()
        .find(|intent| {
            intent.kind() == NativeIntentKind::Present
                && intent
                    .message()
                    .is_some_and(|message| message.event().tick == Tick::from_ticks(25))
        })
        .expect("odd replay creates local N25");
    assert_eq!(n.message().unwrap().event().dest_lp, LP1);
    let blocked = local.accounting_snapshot().unwrap();
    assert_eq!(blocked.blocked_count(), 1);
    assert_eq!(blocked.reserved_pending_count(), 1);
    assert_eq!(
        blocked.minimum_obligation_tick(),
        Some(Tick::from_ticks(25))
    );
    assert_eq!(local.pending_events(LP1), Some(Vec::new()));
    assert_eq!(local.report().pending_positives, 1);
    assert_eq!(local.report().pending_events, 1);
    let requests = local.pending_native_retirement_requests().unwrap();
    assert_eq!(requests.len(), 1);
    let anti = deliver_retirement_request(&mut local, &mut remote, &requests[0]);
    assert_eq!(anti.message().event().tick, Tick::from_ticks(30));
    assert_eq!(drain_one(&mut remote, 40).budget_used, 1);
    assert_eq!(remote.report().tombstones, 1);
    let applied = remote.applied_native_retirements().unwrap().pop().unwrap();
    local.acknowledge_native_retirement(applied).unwrap();
    let promoted = local.accounting_snapshot().unwrap();
    assert_eq!(promoted.reserved_pending_count(), 0);
    assert_eq!(local.pending_events(LP1).unwrap().len(), 1);
    assert_eq!(
        local.pending_events(LP1).unwrap()[0].event().tick,
        Tick::from_ticks(25)
    );
    assert_eq!(drain_one(&mut local, 40).budget_used, 1);
    assert_eq!(local.process_at(LP1).unwrap().value, 202);

    let mut control = all_local(16);
    control.schedule_initial(77, parent).unwrap();
    control.schedule_initial(88, straggler).unwrap();
    control.close_initial_inputs().unwrap();
    let control_progress = control
        .run_until_with_budget(Tick::from_ticks(26), 16)
        .unwrap();
    assert_eq!(control_progress.pending_positives, 0);
    assert_eq!(
        local.process_at(LP0).unwrap().state(),
        control.process_at(LP0).unwrap().state()
    );
    assert_eq!(
        local.process_at(LP1).unwrap().state(),
        control.process_at(LP1).unwrap().state()
    );
    assert_eq!(
        remote.process_at(LP2).unwrap().state(),
        control.process_at(LP2).unwrap().state()
    );

    let split_cut = OptimisticRuntime::fossil_collect_native_group(
        &mut [&mut local, &mut remote],
        Tick::from_ticks(26),
    )
    .unwrap();
    assert_eq!(split_cut.committed_gvt(), Tick::from_ticks(26));
    assert_eq!(remote.report().tombstones, 1);
    let before_late_p = observe(&remote, &[LP2]);
    let late_tombstone = remote.admit_native(&old_p).unwrap();
    assert_eq!(
        late_tombstone.recorded_membership(),
        NativeAdmissionMembership::Tombstoned
    );
    let tombstone_retry = remote.admit_native(&old_p).unwrap();
    assert_eq!(
        tombstone_retry.recorded_membership(),
        NativeAdmissionMembership::Tombstoned
    );
    assert_eq!(
        tombstone_retry.recorded_revision(),
        late_tombstone.recorded_revision()
    );
    assert_eq!(observe(&remote, &[LP2]), before_late_p);

    let control_cut =
        OptimisticRuntime::fossil_collect_native_group(&mut [&mut control], Tick::from_ticks(26))
            .unwrap();
    assert_eq!(trace_rows(&split_cut), trace_rows(&control_cut));
}

#[test]
fn first_rollback_candidate_preempts_the_rest_of_the_selected_positive_vector() {
    let (mut local, mut remote) = pair_world(32);
    let lp0_first = event(LP0, LP0, 10, &[2, 0]);
    let lp0_second = event(LP0, LP0, 15, &[3, 0]);
    let lp1_later = event(LP1, LP1, 20, &[4, 0]);
    let lp1_straggler = event(LP2, LP1, 5, &[1, 0]);
    local.schedule_initial(1, lp0_first.clone()).unwrap();
    local.schedule_initial(2, lp0_second.clone()).unwrap();
    local.schedule_initial(3, lp1_later.clone()).unwrap();
    remote.schedule_initial(50, lp1_straggler.clone()).unwrap();
    let mut roots = remote.ready_native_sends().unwrap();
    let straggler_send = take_root_ticket(&mut roots, LP2, 50);
    close_pair(&mut local, &mut remote);

    let mut control = all_local(32);
    control.schedule_initial(1, lp0_first).unwrap();
    control.schedule_initial(2, lp0_second).unwrap();
    control.schedule_initial(3, lp1_later).unwrap();
    control.schedule_initial(50, lp1_straggler).unwrap();
    control.close_initial_inputs().unwrap();

    let lp1_initial_state = local.process_at(LP1).unwrap().state();
    let first_round = local
        .run_until_with_budget(Tick::from_ticks(30), 2)
        .unwrap();
    assert_eq!(first_round.budget_used, 2);
    assert_eq!(local.report().executions, 2);
    let lp0_after_round = local.state_token(LP0).unwrap();
    let lp1_after_round = local.state_token(LP1).unwrap();
    let lp0_state_after_round = local.process_at(LP0).unwrap().state();

    deliver_positive(&mut remote, &mut local, &straggler_send);
    let before_scan = local.report();
    let audit = Arc::clone(&local.process_at(LP0).unwrap().audit);
    let audit_start = audit.lock().unwrap().len();
    let lp0_handlers_before = local.process_at(LP0).unwrap().seen.len();
    let preempted = local
        .run_until_with_budget(Tick::from_ticks(30), 2)
        .unwrap();
    assert_eq!(preempted.budget_used, 2);
    assert_eq!(
        local.report().rollback_attempts,
        before_scan.rollback_attempts + 1
    );
    assert_eq!(local.report().executions, before_scan.executions + 1);
    assert_eq!(
        local.process_at(LP0).unwrap().value,
        lp0_state_after_round.value + 3
    );
    assert_eq!(local.process_at(LP1).unwrap().state(), lp1_initial_state);
    assert!(!local.validate_state_token(lp0_after_round));
    assert!(!local.validate_state_token(lp1_after_round));
    assert_eq!(
        local.process_at(LP0).unwrap().seen.len(),
        lp0_handlers_before + 1
    );
    let actions = audit.lock().unwrap();
    let retry_actions = &actions[audit_start..];
    let restore_lp1 = retry_actions
        .iter()
        .position(|action| *action == ProbeAction::Restore(LP1))
        .expect("straggler rollback restores LP1 before reselecting positives");
    let handler_lp0 = retry_actions
        .iter()
        .position(|action| *action == ProbeAction::Handler(LP0, 15))
        .expect("remaining budget executes the unaffected LP0 candidate");
    assert!(restore_lp1 < handler_lp0);
    drop(actions);
    drain_to_idle(&mut local, 30);
    drain_to_idle(&mut control, 30);
    for lp in [LP0, LP1, LP2] {
        let actual = match lp {
            LP0 | LP1 => local.process_at(lp).unwrap().state(),
            LP2 => remote.process_at(lp).unwrap().state(),
            _ => unreachable!(),
        };
        assert_eq!(actual, control.process_at(lp).unwrap().state());
    }
    let split_cut = OptimisticRuntime::fossil_collect_native_group(
        &mut [&mut local, &mut remote],
        Tick::from_ticks(31),
    )
    .unwrap();
    let control_cut =
        OptimisticRuntime::fossil_collect_native_group(&mut [&mut control], Tick::from_ticks(31))
            .unwrap();
    assert_eq!(trace_rows(&split_cut), trace_rows(&control_cut));
}

#[test]
fn peer_drop_during_handler_revalidation_compensates_without_publishing_remote_output() {
    let holder: PeerHolder = Arc::new(Mutex::new(None));
    let panic_once = Arc::new(AtomicBool::new(false));
    let mut a =
        new_runtime_with_peer_holder(&[LP0], 16, Arc::clone(&panic_once), Some(holder.clone()));
    let mut b = new_runtime(&[LP1], 16, Arc::clone(&panic_once));
    let mut c = new_runtime(&[LP2], 16, panic_once);
    wire_three(&mut a, &mut b, &mut c);
    let dropped_authority = b.native_accounting_authority().unwrap();
    a.schedule_initial(1, event(LP0, LP0, 1, &[u8::MAX, 1, 1, 0, 4]))
        .unwrap();
    close_three(&mut a, &mut b, &mut c);
    *holder.lock().unwrap() = Some(b);
    let before = observe(&a, &[LP0]);

    let failure = a
        .run_owned_until_with_budget(Tick::from_ticks(10), 1)
        .unwrap_err();
    assert_eq!(failure.phase(), OptimisticOwnedFailurePhase::Compensated);
    assert!(matches!(
        failure.cause(),
        OptimisticError::StaleNativeAccountingAuthority { .. }
    ));
    assert_eq!(failure.progress().budget_used, 1);
    assert!(failure.progress().published_messages.is_empty());
    assert_eq!(failure.attempted().len(), 1);
    assert_eq!(failure.attempted()[0].selected().len(), 1);
    assert_eq!(a.process_at(LP0).unwrap().state(), before.states[&LP0]);
    assert_eq!(a.report().executions, before.report.executions);
    assert!(matches!(
        a.ready_native_sends(),
        Err(OptimisticError::StaleNativeAccountingAuthority { .. })
    ));
    assert!(matches!(
        a.outbound_pending(),
        Err(OptimisticError::StaleNativeAccountingAuthority { .. })
    ));
    assert_eq!(a.accounting_revision(), Ok(before.revision + 1));
    assert!(!a.validate_state_token(before.tokens[&LP0]));
    assert!(!dropped_authority.is_live());
    assert!(holder.lock().unwrap().is_none());
}

#[test]
fn mixed_local_remote_fanout_capacity_failure_publishes_neither_half_of_actual_batch() {
    let panic_once = Arc::new(AtomicBool::new(false));
    let mut local =
        new_runtime_with_bounds(&[LP0, LP1], 16, 1, 8, 16, Arc::clone(&panic_once), None);
    let mut remote = new_runtime_with_bounds(&[LP2], 16, 8, 8, 16, panic_once, None);
    let local_authority = local.native_accounting_authority().unwrap();
    let remote_authority = remote.native_accounting_authority().unwrap();
    local
        .register_native_peer(remote_authority.clone())
        .unwrap();
    remote.register_native_peer(local_authority).unwrap();
    local.seal_native_peers().unwrap();
    remote.seal_native_peers().unwrap();
    local
        .schedule_initial(1, event(LP0, LP0, 1, &[1, 1, 2, 0, 2]))
        .unwrap();
    local
        .schedule_initial(2, event(LP0, LP0, 2, &[2, 4, 1, 2, 5]))
        .unwrap();
    close_pair(&mut local, &mut remote);
    assert_eq!(drain_one(&mut local, 10).budget_used, 1);
    let retained_root = local
        .ready_native_sends()
        .unwrap()
        .into_iter()
        .find(|send| {
            let message = send.message();
            message.kind() == OptimisticMessageKind::Positive
                && message
                    .logical_id()
                    .output_parts()
                    .is_some_and(|(parent, ordinal)| {
                        parent.tick() == Tick::from_ticks(1)
                            && parent.source_lp() == LP0
                            && parent.logical_id().root_parts() == Some((LP0, 1))
                            && ordinal == 0
                    })
                && message.event().source_lp == LP0
                && message.event().dest_lp == LP2
                && message.event().tick == Tick::from_ticks(3)
                && message.event().event_payload == vec![101, 0]
        })
        .expect("the exact child output fills the one-slot source outbox");
    let before = observe(&local, &[LP0, LP1]);
    let remote_before = observe(&remote, &[LP2]);
    let accounting_before = local.accounting_snapshot().unwrap();

    let failure = local
        .run_owned_until_with_budget(Tick::from_ticks(10), 1)
        .unwrap_err();
    assert_eq!(failure.phase(), OptimisticOwnedFailurePhase::Compensated);
    assert_eq!(
        failure.cause(),
        &OptimisticError::OutboxLimitExceeded { limit: 1 }
    );
    assert_eq!(failure.progress().budget_used, 1);
    assert!(failure.progress().published_messages.is_empty());
    assert_eq!(failure.compensated_lps(), &[LP0]);
    assert_eq!(local.process_at(LP0).unwrap().state(), before.states[&LP0]);
    assert_eq!(local.process_at(LP1).unwrap().state(), before.states[&LP1]);
    assert_eq!(local.pending_events(LP1), Some(Vec::new()));
    assert_eq!(local.report().executions, before.report.executions);
    assert_eq!(local.pending_events(LP0).unwrap().len(), 1);
    let sends_after_failure = local.ready_native_sends().unwrap();
    assert_eq!(sends_after_failure.len(), 1);
    assert_eq!(sends_after_failure[0].message(), retained_root.message());
    assert_eq!(local.outbound_pending().unwrap().len(), 1);
    assert_eq!(
        local.outbound_pending().unwrap()[0].message(),
        retained_root.message()
    );
    assert_eq!(
        local.accounting_snapshot().unwrap().local_positive_count(),
        accounting_before.local_positive_count()
    );
    assert_eq!(
        local.accounting_snapshot().unwrap().revision(),
        accounting_before.revision() + 1
    );
    assert!(!local.validate_state_token(before.tokens[&LP0]));
    assert!(local.validate_state_token(before.tokens[&LP1]));
    assert_eq!(observe(&remote, &[LP2]), remote_before);
}

#[test]
fn native_group_cut_rejects_open_and_incomplete_participants_without_advancing_any_floor() {
    let (mut a, mut b, mut c) = three_world(16);
    let before_a = observe(&a, &[LP0]);
    let before_b = observe(&b, &[LP1]);
    let before_c = observe(&c, &[LP2]);
    let open = OptimisticRuntime::fossil_collect_native_group(
        &mut [&mut a, &mut b, &mut c],
        Tick::from_ticks(1),
    )
    .unwrap_err();
    assert!(matches!(
        open.cause(),
        OptimisticError::NativeGroupInputsOpen { .. }
    ));
    assert_eq!(observe(&a, &[LP0]), before_a);
    assert_eq!(observe(&b, &[LP1]), before_b);
    assert_eq!(observe(&c, &[LP2]), before_c);

    close_three(&mut a, &mut b, &mut c);
    let sealed_a = observe(&a, &[LP0]);
    let sealed_b = observe(&b, &[LP1]);
    let sealed_c = observe(&c, &[LP2]);
    let incomplete =
        OptimisticRuntime::fossil_collect_native_group(&mut [&mut a, &mut b], Tick::from_ticks(1))
            .unwrap_err();
    assert!(matches!(
        incomplete.cause(),
        OptimisticError::NativeGroupCoverageIncomplete { missing, unexpected }
            if missing == &vec![LP2] && unexpected.is_empty()
    ));
    assert_eq!(observe(&a, &[LP0]), sealed_a);
    assert_eq!(observe(&b, &[LP1]), sealed_b);
    assert_eq!(observe(&c, &[LP2]), sealed_c);
    let completed = OptimisticRuntime::fossil_collect_native_group(
        &mut [&mut a, &mut b, &mut c],
        Tick::from_ticks(1),
    )
    .unwrap();
    assert_eq!(completed.committed_gvt(), Tick::from_ticks(1));
}

#[test]
fn snapshot_panic_poison_invalidates_every_owned_lp_before_handlers_publish() {
    let control = Arc::new(Mutex::new(FaultControl::default()));
    let audit = Arc::new(Mutex::new(Vec::new()));
    let mut runtime = new_fault_runtime(
        &[LP0, LP1, LP2],
        32,
        32,
        Arc::clone(&control),
        Arc::clone(&audit),
        None,
    );
    runtime.seal_native_peers().unwrap();
    runtime
        .schedule_initial(1, event(LP0, LP0, 1, &[7, 0]))
        .unwrap();
    runtime
        .schedule_initial(2, event(LP1, LP1, 1, &[9, 0]))
        .unwrap();
    runtime.close_initial_inputs().unwrap();
    control.lock().unwrap().snapshot_panic = Some(LP1);
    let before_states = [LP0, LP1, LP2].map(|lp| (lp, runtime.process_at(lp).unwrap().state()));
    let before_pending = [LP0, LP1, LP2].map(|lp| (lp, runtime.pending_events(lp)));
    let old_tokens = [LP0, LP1, LP2].map(|lp| (lp, runtime.state_token(lp).unwrap()));

    let failure = runtime
        .run_owned_until_with_budget(Tick::from_ticks(10), 2)
        .unwrap_err();
    assert_eq!(
        failure.phase(),
        OptimisticOwnedFailurePhase::PoisonedBeforePublication
    );
    assert_eq!(failure.cause(), &OptimisticError::SnapshotPanicked(LP1));
    assert_eq!(failure.trigger_cause(), None);
    assert_eq!(failure.progress().budget_used, 2);
    assert!(failure.progress().published_messages.is_empty());
    assert_eq!(failure.invalidated_lps(), &[LP0, LP1, LP2]);
    assert!(failure.poisoned());
    assert_eq!(failure.attempted().len(), 1);
    assert_eq!(failure.attempted()[0].selected().len(), 2);
    for (lp, state) in before_states {
        assert_eq!(runtime.process_at(lp).unwrap().state(), state);
    }
    for (lp, pending) in before_pending {
        assert_eq!(runtime.pending_events(lp), pending);
    }
    for (lp, token) in old_tokens {
        assert!(
            !runtime.validate_state_token(token),
            "poison must invalidate {lp:?}"
        );
    }
    let actions = audit.lock().unwrap().clone();
    assert!(actions.contains(&FaultAction::Snapshot(LP0)));
    assert!(actions.contains(&FaultAction::Snapshot(LP1)));
    assert!(!actions
        .iter()
        .any(|action| matches!(action, FaultAction::Handler(..))));
}

#[test]
fn restore_error_preserves_handler_trigger_and_restores_other_selected_lp() {
    let control = Arc::new(Mutex::new(FaultControl {
        restore_error: Some(LP1),
        ..FaultControl::default()
    }));
    let audit = Arc::new(Mutex::new(Vec::new()));
    let mut runtime = new_fault_runtime(
        &[LP0, LP1, LP2],
        32,
        32,
        Arc::clone(&control),
        Arc::clone(&audit),
        None,
    );
    runtime.seal_native_peers().unwrap();
    runtime
        .schedule_initial(1, event(LP0, LP0, 1, &[7, 0]))
        .unwrap();
    runtime
        .schedule_initial(2, event(LP1, LP1, 2, &[9, 2, 2, 1]))
        .unwrap();
    runtime.close_initial_inputs().unwrap();
    let before_lp0 = runtime.process_at(LP0).unwrap().state();
    let before_lp2 = runtime.process_at(LP2).unwrap().state();
    let tokens = [LP0, LP1, LP2].map(|lp| (lp, runtime.state_token(lp).unwrap()));

    let failure = runtime
        .run_owned_until_with_budget(Tick::from_ticks(10), 2)
        .unwrap_err();
    assert_eq!(
        failure.phase(),
        OptimisticOwnedFailurePhase::PoisonedBeforePublication
    );
    assert!(matches!(
        failure.cause(),
        OptimisticError::RestoreFailed { lp_id, .. } if *lp_id == LP1
    ));
    assert!(matches!(
        failure.trigger_cause(),
        Some(OptimisticError::OutputSourceMismatch { .. })
    ));
    assert_eq!(failure.progress().budget_used, 2);
    assert!(failure.progress().published_messages.is_empty());
    assert_eq!(failure.invalidated_lps(), &[LP0, LP1, LP2]);
    assert_eq!(runtime.process_at(LP0).unwrap().state(), before_lp0);
    assert_ne!(runtime.process_at(LP1).unwrap().value, 101);
    assert_eq!(runtime.process_at(LP2).unwrap().state(), before_lp2);
    for (lp, token) in tokens {
        assert!(
            !runtime.validate_state_token(token),
            "poison must invalidate {lp:?}"
        );
    }
    let actions = audit.lock().unwrap().clone();
    assert!(actions.contains(&FaultAction::Restore(LP0)));
    assert!(actions.contains(&FaultAction::Restore(LP1)));
    assert!(actions.contains(&FaultAction::Handler(LP0, 1)));
    assert!(actions.contains(&FaultAction::Handler(LP1, 2)));
}

#[test]
fn restore_panic_is_a_fatal_compensation_error_and_poison_is_global() {
    let control = Arc::new(Mutex::new(FaultControl {
        handler_panic: Some(LP1),
        restore_panic: Some(LP1),
        ..FaultControl::default()
    }));
    let audit = Arc::new(Mutex::new(Vec::new()));
    let mut runtime = new_fault_runtime(
        &[LP0, LP1, LP2],
        32,
        32,
        Arc::clone(&control),
        Arc::clone(&audit),
        None,
    );
    runtime.seal_native_peers().unwrap();
    runtime
        .schedule_initial(1, event(LP0, LP0, 1, &[7, 0]))
        .unwrap();
    runtime
        .schedule_initial(2, event(LP1, LP1, 2, &[9, 2, 2, 1]))
        .unwrap();
    runtime.close_initial_inputs().unwrap();
    let before_lp0 = runtime.process_at(LP0).unwrap().state();
    let tokens = [LP0, LP1, LP2].map(|lp| runtime.state_token(lp).unwrap());

    let failure = runtime
        .run_owned_until_with_budget(Tick::from_ticks(10), 2)
        .unwrap_err();
    assert_eq!(
        failure.phase(),
        OptimisticOwnedFailurePhase::PoisonedBeforePublication
    );
    assert_eq!(failure.cause(), &OptimisticError::RestorePanicked(LP1));
    assert_eq!(
        failure.trigger_cause(),
        Some(&OptimisticError::HandlerPanicked(LP1))
    );
    assert_eq!(failure.progress().budget_used, 2);
    assert!(failure.progress().published_messages.is_empty());
    assert_eq!(failure.invalidated_lps(), &[LP0, LP1, LP2]);
    assert_eq!(runtime.process_at(LP0).unwrap().state(), before_lp0);
    for token in tokens {
        assert!(!runtime.validate_state_token(token));
    }
}

#[test]
fn snapshot_callback_can_drop_registered_peer_without_holding_runtime_gate() {
    let control = Arc::new(Mutex::new(FaultControl::default()));
    let audit = Arc::new(Mutex::new(Vec::new()));
    let holder: FaultPeerHolder = Arc::new(Mutex::new(None));
    let mut local = new_fault_runtime(
        &[LP0, LP1],
        32,
        32,
        Arc::clone(&control),
        Arc::clone(&audit),
        Some(Arc::clone(&holder)),
    );
    let mut peer = new_fault_runtime(
        &[LP2],
        32,
        32,
        Arc::clone(&control),
        Arc::clone(&audit),
        None,
    );
    let local_authority = local.native_accounting_authority().unwrap();
    let peer_authority = peer.native_accounting_authority().unwrap();
    local.register_native_peer(peer_authority.clone()).unwrap();
    peer.register_native_peer(local_authority).unwrap();
    local.seal_native_peers().unwrap();
    peer.seal_native_peers().unwrap();
    *holder.lock().unwrap() = Some(peer);
    local
        .schedule_initial(1, event(LP0, LP0, 1, &[7, 0]))
        .unwrap();
    local.close_initial_inputs().unwrap();
    control.lock().unwrap().peer_drop_snapshot = Some(LP0);
    let before_lp0 = local.process_at(LP0).unwrap().state();
    let before_lp1 = local.process_at(LP1).unwrap().state();
    let lp0_token = local.state_token(LP0).unwrap();
    let lp1_token = local.state_token(LP1).unwrap();

    let (tx, rx) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        let result = local.run_owned_until_with_budget(Tick::from_ticks(10), 1);
        tx.send((local, result)).unwrap();
    });
    let (local, result) = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("snapshot callback deadlocked while dropping a registered peer");
    worker.join().unwrap();
    let failure = result.unwrap_err();
    assert_eq!(failure.phase(), OptimisticOwnedFailurePhase::Compensated);
    assert!(matches!(
        failure.cause(),
        OptimisticError::StaleNativeAccountingAuthority { .. }
    ));
    assert_eq!(failure.progress().budget_used, 1);
    assert!(failure.progress().published_messages.is_empty());
    assert!(holder.lock().unwrap().is_none());
    assert_eq!(local.process_at(LP0).unwrap().state(), before_lp0);
    assert_eq!(local.process_at(LP1).unwrap().state(), before_lp1);
    assert!(!local.validate_state_token(lp0_token));
    assert!(local.validate_state_token(lp1_token));
    let actions = audit.lock().unwrap().clone();
    let drop_at = actions
        .iter()
        .position(|action| *action == FaultAction::PeerDropFromSnapshot(LP0))
        .unwrap();
    assert!(actions[drop_at + 1..]
        .iter()
        .any(|action| matches!(action, FaultAction::SnapshotDrop(LP0))));
}

#[test]
fn group_cut_commits_common_floor_even_when_retired_snapshot_drop_panics() {
    let control = Arc::new(Mutex::new(FaultControl::default()));
    let audit = Arc::new(Mutex::new(Vec::new()));
    let (mut a, mut b, mut c) = fault_world_three(32, 32, Arc::clone(&control), Arc::clone(&audit));
    a.schedule_initial(1, event(LP0, LP0, 1, &[7, 0])).unwrap();
    fault_close_three(&mut a, &mut b, &mut c);
    let history_snapshot_id = control
        .lock()
        .unwrap()
        .next_snapshot_id
        .get(&LP0)
        .copied()
        .unwrap_or(0);
    assert_eq!(
        a.run_until_with_budget(Tick::from_ticks(10), 1)
            .unwrap()
            .budget_used,
        1
    );
    let old_a = a.state_token(LP0).unwrap();
    let old_b = b.state_token(LP1).unwrap();
    let old_c = c.state_token(LP2).unwrap();
    {
        let mut control = control.lock().unwrap();
        control.drop_panic = Some(LP0);
        control.drop_snapshot_target = Some((LP0, history_snapshot_id));
    }

    let cut = OptimisticRuntime::fossil_collect_native_group(
        &mut [&mut a, &mut b, &mut c],
        Tick::from_ticks(2),
    )
    .expect("post-publication Snapshot::drop panic must not retract the common cut");
    assert_eq!(cut.committed_gvt(), Tick::from_ticks(2));
    assert_eq!(cut.reports().len(), 3);
    assert!(cut
        .reports()
        .iter()
        .all(|report| report.new_gvt == Tick::from_ticks(2)));
    assert_eq!(cut.cleanup_failures().len(), 1);
    let cleanup = &cut.cleanup_failures()[0];
    assert_eq!(cleanup.lp_id(), Some(LP0));
    assert_eq!(cleanup.cause(), &OptimisticError::SnapshotDropPanicked(LP0));
    assert!(!a.validate_state_token(old_a));
    assert!(!b.validate_state_token(old_b));
    assert!(!c.validate_state_token(old_c));
    let fresh_b = b.state_token(LP1).unwrap();
    let fresh_c = c.state_token(LP2).unwrap();
    assert!(b.validate_state_token(fresh_b));
    assert!(c.validate_state_token(fresh_c));
    assert_eq!(a.report().executions, 1);
    assert!(a.process_at(LP0).unwrap().seen.len() == 1);
}

#[test]
fn committed_anti_cleanup_failure_reports_its_published_descendant_anti_and_drop_peer() {
    let control = Arc::new(Mutex::new(FaultControl::default()));
    let audit = Arc::new(Mutex::new(Vec::new()));
    let holder: FaultPeerHolder = Arc::new(Mutex::new(None));
    let mut a = new_fault_runtime(
        &[LP0],
        32,
        32,
        Arc::clone(&control),
        Arc::clone(&audit),
        None,
    );
    let mut b = new_fault_runtime(
        &[LP1],
        32,
        32,
        Arc::clone(&control),
        Arc::clone(&audit),
        Some(Arc::clone(&holder)),
    );
    let mut c = new_fault_runtime(
        &[LP2],
        32,
        32,
        Arc::clone(&control),
        Arc::clone(&audit),
        None,
    );
    let authorities = [
        a.native_accounting_authority().unwrap(),
        b.native_accounting_authority().unwrap(),
        c.native_accounting_authority().unwrap(),
    ];
    for runtime in [&mut a, &mut b, &mut c] {
        for authority in &authorities {
            if authority.runtime_id() != runtime.native_accounting_authority().unwrap().runtime_id()
            {
                runtime.register_native_peer(authority.clone()).unwrap();
            }
        }
        runtime.seal_native_peers().unwrap();
    }
    a.schedule_initial(1, event(LP0, LP0, 1, &[0, 1, LP1.0 as u8, 1]))
        .unwrap();
    c.schedule_initial(2, event(LP2, LP0, 0, &[1, 0])).unwrap();
    let late_root_send = take_root_ticket(&mut c.ready_native_sends().unwrap(), LP2, 2);
    fault_close_three(&mut a, &mut b, &mut c);
    *holder.lock().unwrap() = Some(a);

    let p_send = {
        let mut source = holder.lock().unwrap();
        let source = source.as_mut().unwrap();
        assert_eq!(
            source
                .run_until_with_budget(Tick::from_ticks(10), 1)
                .unwrap()
                .budget_used,
            1
        );
        source
            .ready_native_sends()
            .unwrap()
            .into_iter()
            .find(|send| {
                send.message().kind() == OptimisticMessageKind::Positive
                    && send.message().event().dest_lp == LP1
            })
            .expect("source root emits P to LP1")
    };
    let p_cap = b.admit_native(&p_send).unwrap();
    holder
        .lock()
        .unwrap()
        .as_mut()
        .unwrap()
        .acknowledge_native_admission(p_cap)
        .unwrap();
    let p_history_snapshot_id = control
        .lock()
        .unwrap()
        .next_snapshot_id
        .get(&LP1)
        .copied()
        .unwrap_or(0);
    assert_eq!(
        b.run_until_with_budget(Tick::from_ticks(10), 1)
            .unwrap()
            .budget_used,
        1
    );
    let q_send = b
        .ready_native_sends()
        .unwrap()
        .into_iter()
        .find(|send| {
            send.message().kind() == OptimisticMessageKind::Positive
                && send.message().event().dest_lp == LP2
        })
        .expect("executed P emits descendant Q to LP2");
    let before_b = b.process_at(LP1).unwrap().state();
    assert_eq!(before_b.value, 201);
    assert_eq!(before_b.seen.len(), 1);
    let old_b_token = b.state_token(LP1).unwrap();

    {
        let mut source = holder.lock().unwrap();
        let source = source.as_mut().unwrap();
        let late_root_cap = source.admit_native(&late_root_send).unwrap();
        c.acknowledge_native_admission(late_root_cap).unwrap();
        assert_eq!(
            source
                .run_until_with_budget(Tick::from_ticks(10), 1)
                .unwrap()
                .budget_used,
            1
        );
    }
    let anti_send = holder
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .ready_native_sends()
        .unwrap()
        .into_iter()
        .find(|send| send.message().kind() == OptimisticMessageKind::Anti)
        .expect("straggler rollback emits P's exact anti");
    let request = anti_send.retirement_request().unwrap().clone();
    let anti_cap = b.receive_native_retirement(&request).unwrap();
    holder
        .lock()
        .unwrap()
        .as_mut()
        .unwrap()
        .acknowledge_native_admission(anti_cap)
        .unwrap();

    // Target the retained pre-P history snapshot; later compensation
    // snapshots and ordinary clones are non-targets.
    {
        let mut control = control.lock().unwrap();
        control.drop_panic = Some(LP1);
        control.drop_snapshot_target = Some((LP1, p_history_snapshot_id));
        control.peer_drop_snapshot_drop = Some(LP1);
        control.peer_drop_snapshot_target = Some((LP1, p_history_snapshot_id));
    }
    let failure = b
        .run_owned_until_with_budget(Tick::from_ticks(10), 1)
        .unwrap_err();
    assert_eq!(
        failure.phase(),
        OptimisticOwnedFailurePhase::CommittedCleanupFailed
    );
    assert_eq!(failure.cause(), &OptimisticError::SnapshotDropPanicked(LP1));
    assert_eq!(failure.trigger_cause(), None);
    assert_eq!(failure.progress().budget_used, 1);
    assert_eq!(failure.attempted().len(), 1);
    assert_eq!(failure.attempted()[0].kind(), OptimisticOwnedStepKind::Anti);
    assert_eq!(
        failure.attempted()[0].selected()[0],
        *request.predecessor_anti()
    );
    assert!(failure.poisoned());
    assert_eq!(failure.invalidated_lps(), &[LP1]);
    assert_eq!(b.process_at(LP1).unwrap().value, 101);
    assert!(b.process_at(LP1).unwrap().seen.is_empty());
    assert!(!b.validate_state_token(old_b_token));
    assert!(holder.lock().unwrap().is_none());
    assert!(
        failure.progress().published_messages.iter().any(|message| {
            message.kind() == OptimisticMessageKind::Anti
                && message.event().dest_lp == LP2
                && message.logical_id() == q_send.message().logical_id()
                && message.incarnation() == q_send.message().incarnation()
        }),
        "progress retains Q's committed anti despite the cleanup fault"
    );
    let actions = audit.lock().unwrap().clone();
    assert!(actions.contains(&FaultAction::PeerDropFromSnapshotDrop(LP1)));
}

#[test]
fn executed_descendant_cascade_restores_each_owner_and_blocks_gvt_until_final_anti() {
    let control_faults = Arc::new(Mutex::new(FaultControl::default()));
    let control_audit = Arc::new(Mutex::new(Vec::new()));
    let (mut a, mut b, mut c) = fault_world_three(
        64,
        64,
        Arc::clone(&control_faults),
        Arc::clone(&control_audit),
    );
    let parent = event(LP0, LP0, 1, &[0, 1, LP1.0 as u8, 1]);
    let late_root = event(LP1, LP0, 0, &[1, 0]);
    a.schedule_initial(1, parent.clone()).unwrap();
    b.schedule_initial(2, late_root.clone()).unwrap();
    let late_root_send = take_root_ticket(&mut b.ready_native_sends().unwrap(), LP1, 2);
    fault_close_three(&mut a, &mut b, &mut c);

    let control_faults = Arc::new(Mutex::new(FaultControl::default()));
    let control_audit = Arc::new(Mutex::new(Vec::new()));
    let mut control = new_fault_runtime(
        &[LP0, LP1, LP2],
        64,
        64,
        control_faults,
        control_audit,
        None,
    );
    control.seal_native_peers().unwrap();
    control.schedule_initial(1, parent).unwrap();
    control.schedule_initial(2, late_root).unwrap();
    control.close_initial_inputs().unwrap();

    assert_eq!(
        a.run_owned_until_with_budget(Tick::from_ticks(4), 1)
            .unwrap()
            .budget_used,
        1
    );
    let p_send = a
        .ready_native_sends()
        .unwrap()
        .into_iter()
        .find(|send| {
            send.message().kind() == OptimisticMessageKind::Positive
                && send.message().event().dest_lp == LP1
                && send.message().event().tick == Tick::from_ticks(2)
        })
        .expect("the initial parent emits P to LP1 at tick 2");
    let p_message = p_send.message().clone();
    let p_capability = b.admit_native(&p_send).unwrap();
    a.acknowledge_native_admission(p_capability).unwrap();
    assert_eq!(
        b.run_owned_until_with_budget(Tick::from_ticks(4), 1)
            .unwrap()
            .budget_used,
        1
    );
    let q_send = b
        .ready_native_sends()
        .unwrap()
        .into_iter()
        .find(|send| {
            send.message().kind() == OptimisticMessageKind::Positive
                && send.message().event().dest_lp == LP2
                && send.message().event().tick == Tick::from_ticks(3)
        })
        .expect("executing P emits descendant Q to LP2 at tick 3");
    let q_message = q_send.message().clone();
    let q_capability = c.admit_native(&q_send).unwrap();
    b.acknowledge_native_admission(q_capability).unwrap();
    assert_eq!(
        c.run_owned_until_with_budget(Tick::from_ticks(4), 1)
            .unwrap()
            .budget_used,
        1
    );
    assert_eq!(c.process_at(LP2).unwrap().value, 303);

    let late_root_capability = a.admit_native(&late_root_send).unwrap();
    b.acknowledge_native_admission(late_root_capability)
        .unwrap();
    assert_eq!(
        a.run_owned_until_with_budget(Tick::from_ticks(4), 1)
            .unwrap()
            .budget_used,
        1,
        "the straggler first commits its positive-triggered rollback unit"
    );
    assert_eq!(
        a.run_owned_until_with_budget(Tick::from_ticks(4), 1)
            .unwrap()
            .budget_used,
        1,
        "the late root then executes"
    );
    assert_eq!(
        a.run_owned_until_with_budget(Tick::from_ticks(4), 1)
            .unwrap()
            .budget_used,
        1,
        "replaying the parent emits its ordinary one-child batch"
    );
    let p2_view = a
        .outbound_pending()
        .unwrap()
        .into_iter()
        .find(|view| {
            view.message().kind() == OptimisticMessageKind::Positive
                && view.message().logical_id() == p_message.logical_id()
                && view.message().incarnation() > p_message.incarnation()
        })
        .expect("the ordinary replay retains a replacement for P's exact logical slot");
    assert_eq!(p2_view.message().event().tick, Tick::from_ticks(2));
    assert_eq!(p2_view.message().event().dest_lp, LP1);
    assert_eq!(
        p2_view.status(),
        OptimisticOutboundStatus::BlockedReplacement
    );
    assert!(b.process_at(LP1).unwrap().seen.len() == 1);
    assert_eq!(c.process_at(LP2).unwrap().value, 303);

    let p_retirement = a
        .pending_native_retirement_requests()
        .unwrap()
        .into_iter()
        .find(|request| request.predecessor_positive() == &p_message)
        .expect("P retirement is addressed to its real LP1 receiver");
    let p_anti_capability = b.receive_native_retirement(&p_retirement).unwrap();
    a.acknowledge_native_admission(p_anti_capability).unwrap();
    assert_eq!(
        b.run_owned_until_with_budget(Tick::from_ticks(4), 1)
            .unwrap()
            .budget_used,
        1
    );
    assert_eq!(b.process_at(LP1).unwrap().value, 101);
    assert!(b.process_at(LP1).unwrap().seen.is_empty());
    let q_retirement = b
        .pending_native_retirement_requests()
        .unwrap()
        .into_iter()
        .find(|request| request.predecessor_positive() == &q_message)
        .expect("rolling P back publishes Q's exact descendant anti");
    let applied_p = b.applied_native_retirements().unwrap().pop().unwrap();
    a.acknowledge_native_retirement(applied_p).unwrap();
    let released_p2 = a
        .outbound_pending()
        .unwrap()
        .into_iter()
        .find(|view| view.intent_id() == p2_view.intent_id())
        .unwrap();
    assert_eq!(released_p2.status(), OptimisticOutboundStatus::Ready);
    let p2_send = a
        .ready_native_sends()
        .unwrap()
        .into_iter()
        .find(|send| send.message().incarnation() == p2_view.message().incarnation())
        .unwrap();
    let p2_capability = b.admit_native(&p2_send).unwrap();
    a.acknowledge_native_admission(p2_capability).unwrap();
    assert_eq!(
        b.run_owned_until_with_budget(Tick::from_ticks(4), 1)
            .unwrap()
            .budget_used,
        1
    );
    let q2_view = b
        .outbound_pending()
        .unwrap()
        .into_iter()
        .find(|view| {
            view.message().kind() == OptimisticMessageKind::Positive
                && view.message().logical_id() == q_message.logical_id()
                && view.message().incarnation() > q_message.incarnation()
        })
        .expect("P2's child keeps Q's same full parent order key");
    assert_eq!(q2_view.message().event().tick, Tick::from_ticks(3));
    assert_eq!(
        q2_view.status(),
        OptimisticOutboundStatus::BlockedReplacement
    );

    let q_anti_capability = c.receive_native_retirement(&q_retirement).unwrap();
    b.acknowledge_native_admission(q_anti_capability).unwrap();
    let before_blocked_cut = [
        a.process_at(LP0).unwrap().state(),
        b.process_at(LP1).unwrap().state(),
        c.process_at(LP2).unwrap().state(),
    ];
    let revisions = [
        a.accounting_revision().unwrap(),
        b.accounting_revision().unwrap(),
        c.accounting_revision().unwrap(),
    ];
    let rejected_cut = OptimisticRuntime::fossil_collect_native_group(
        &mut [&mut a, &mut b, &mut c],
        Tick::from_ticks(4),
    )
    .unwrap_err();
    assert!(matches!(
        rejected_cut.cause(),
        OptimisticError::GvtBeyondPending { requested, .. } if *requested == Tick::from_ticks(4)
    ));
    assert_eq!(a.process_at(LP0).unwrap().state(), before_blocked_cut[0]);
    assert_eq!(b.process_at(LP1).unwrap().state(), before_blocked_cut[1]);
    assert_eq!(c.process_at(LP2).unwrap().state(), before_blocked_cut[2]);
    assert_eq!(
        [
            a.accounting_revision().unwrap(),
            b.accounting_revision().unwrap(),
            c.accounting_revision().unwrap()
        ],
        revisions
    );

    assert_eq!(
        c.run_owned_until_with_budget(Tick::from_ticks(4), 1)
            .unwrap()
            .budget_used,
        1
    );
    assert_eq!(c.process_at(LP2).unwrap().value, 102);
    assert!(c.process_at(LP2).unwrap().seen.is_empty());
    let applied_q = c.applied_native_retirements().unwrap().pop().unwrap();
    b.acknowledge_native_retirement(applied_q).unwrap();
    let released_q2 = b
        .outbound_pending()
        .unwrap()
        .into_iter()
        .find(|view| view.intent_id() == q2_view.intent_id())
        .unwrap();
    assert_eq!(released_q2.status(), OptimisticOutboundStatus::Ready);
    let q2_send = b
        .ready_native_sends()
        .unwrap()
        .into_iter()
        .find(|send| send.message().incarnation() == q2_view.message().incarnation())
        .unwrap();
    let q2_capability = c.admit_native(&q2_send).unwrap();
    b.acknowledge_native_admission(q2_capability).unwrap();
    assert_eq!(
        c.run_owned_until_with_budget(Tick::from_ticks(4), 1)
            .unwrap()
            .budget_used,
        1
    );
    assert_eq!(a.process_at(LP0).unwrap().value, 101);
    assert_eq!(b.process_at(LP1).unwrap().value, 202);
    assert_eq!(c.process_at(LP2).unwrap().value, 304);

    fault_drain_to_idle(&mut control, 4);
    for lp in [LP0, LP1, LP2] {
        let actual = match lp {
            LP0 => a.process_at(lp).unwrap().state(),
            LP1 => b.process_at(lp).unwrap().state(),
            LP2 => c.process_at(lp).unwrap().state(),
            _ => unreachable!(),
        };
        assert_eq!(actual, control.process_at(lp).unwrap().state());
    }
    let split_cut = OptimisticRuntime::fossil_collect_native_group(
        &mut [&mut a, &mut b, &mut c],
        Tick::from_ticks(4),
    )
    .unwrap();
    let control_cut =
        OptimisticRuntime::fossil_collect_native_group(&mut [&mut control], Tick::from_ticks(4))
            .unwrap();
    assert_eq!(trace_rows(&split_cut), trace_rows(&control_cut));
}

#[test]
fn local_self_retirement_uses_one_shared_receipt_and_applies_before_replay() {
    let panic_once = Arc::new(AtomicBool::new(false));
    let mut runtime = new_runtime_with_bounds(&[LP0], 8, 64, 64, 2, Arc::clone(&panic_once), None);
    let mut remote = new_runtime_with_bounds(&[LP1, LP2], 8, 64, 64, 1, panic_once, None);
    let local_authority = runtime.native_accounting_authority().unwrap();
    let remote_authority = remote.native_accounting_authority().unwrap();
    runtime
        .register_native_peer(remote_authority.clone())
        .unwrap();
    remote.register_native_peer(local_authority).unwrap();
    runtime.seal_native_peers().unwrap();
    remote.seal_native_peers().unwrap();
    let parent = event(LP0, LP0, 15, &[0, 3, 0, 0, 15]);
    runtime.schedule_initial(77, parent.clone()).unwrap();
    remote
        .schedule_initial(88, event(LP1, LP0, 10, &[1, 0]))
        .unwrap();
    let straggler_send = take_root_ticket(&mut remote.ready_native_sends().unwrap(), LP1, 88);
    close_pair(&mut runtime, &mut remote);
    assert_eq!(drain_one(&mut runtime, 40).budget_used, 1);
    let original_local_output = runtime
        .pending_events(LP0)
        .unwrap()
        .into_iter()
        .find(|message| message.kind() == OptimisticMessageKind::Positive)
        .expect("the parent emits one genuine local P");
    assert_eq!(original_local_output.event().tick, Tick::from_ticks(30));

    let straggler_cap = runtime.admit_native(&straggler_send).unwrap();
    let root_baseline = runtime.accounting_snapshot().unwrap();
    let root_intent = runtime
        .native_intents()
        .unwrap()
        .into_iter()
        .find(|intent| intent.logical_id().root_parts() == Some((LP0, 77)))
        .unwrap();
    let root_id = root_intent.id().clone();

    remote.acknowledge_native_admission(straggler_cap).unwrap();
    assert_eq!(drain_one(&mut runtime, 40).budget_used, 1); // rollback P suffix
    let request = runtime
        .pending_native_retirement_requests()
        .unwrap()
        .into_iter()
        .find(|request| request.predecessor_positive() == &original_local_output)
        .expect("local P cancellation has one retained request");
    assert_eq!(
        request.sender().runtime_id(),
        request.old_receiver().runtime_id()
    );
    let queued = runtime.pending_events(LP0).unwrap();
    assert_eq!(
        queued
            .iter()
            .filter(|message| message.kind() == OptimisticMessageKind::Anti)
            .count(),
        1
    );
    assert!(runtime.ready_native_sends().unwrap().is_empty());
    assert!(runtime.applied_native_retirements().unwrap().is_empty());
    let before_retry = observe(&runtime, &[LP0]);
    let before_accounting = runtime.accounting_snapshot().unwrap();
    assert_eq!(
        before_accounting.reserved_receipt_count() + before_accounting.retained_receipt_count()
            - root_baseline.reserved_receipt_count()
            - root_baseline.retained_receipt_count(),
        1
    );
    assert_eq!(before_accounting.retirement_count(), 1);
    assert_eq!(before_accounting.receiver_retirement_count(), 1);
    assert_eq!(
        before_accounting.reserved_receipt_count() + before_accounting.retained_receipt_count(),
        2,
        "the remote root admission and shared self request/anti occupy two records"
    );
    let self_capability = runtime.receive_native_retirement(&request).unwrap();
    runtime
        .acknowledge_native_admission(self_capability.clone())
        .unwrap();
    assert_eq!(observe(&runtime, &[LP0]), before_retry);
    let after_retry_accounting = runtime.accounting_snapshot().unwrap();
    assert_eq!(after_retry_accounting.retirement_count(), 1);
    assert_eq!(after_retry_accounting.receiver_retirement_count(), 1);
    assert_eq!(
        after_retry_accounting.reserved_receipt_count()
            + after_retry_accounting.retained_receipt_count(),
        2
    );
    assert_eq!(runtime.pending_events(LP0).unwrap(), queued);

    assert_eq!(drain_one(&mut runtime, 40).budget_used, 1); // apply local anti
    let applied = runtime.applied_native_retirements().unwrap().pop().unwrap();
    runtime
        .acknowledge_native_retirement(applied.clone())
        .unwrap();
    runtime.acknowledge_native_retirement(applied).unwrap();
    assert_eq!(drain_one(&mut runtime, 40).budget_used, 1); // straggler
    assert_eq!(drain_one(&mut runtime, 40).budget_used, 1); // replay parent
    assert_eq!(drain_one(&mut runtime, 40).budget_used, 1); // replacement local P
    let final_state = runtime.process_at(LP0).unwrap().state();
    assert_eq!(final_state.value, 202);
    assert_eq!(
        final_state
            .seen
            .iter()
            .map(|seen| (seen.event.tick, seen.value))
            .collect::<Vec<_>>(),
        vec![
            (Tick::from_ticks(10), 101),
            (Tick::from_ticks(15), 101),
            (Tick::from_ticks(30), 202),
        ]
    );
    assert_eq!(runtime.report().executions, 4);
    let mut control = all_local(8);
    control.schedule_initial(77, parent).unwrap();
    control
        .schedule_initial(88, event(LP1, LP0, 10, &[1, 0]))
        .unwrap();
    control.close_initial_inputs().unwrap();
    drain_to_idle(&mut control, 40);
    assert_eq!(
        runtime.process_at(LP0).unwrap().state(),
        control.process_at(LP0).unwrap().state()
    );
    assert_eq!(
        remote.process_at(LP1).unwrap().state(),
        control.process_at(LP1).unwrap().state()
    );
    assert_eq!(
        remote.process_at(LP2).unwrap().state(),
        control.process_at(LP2).unwrap().state()
    );
    let split_cut = OptimisticRuntime::fossil_collect_native_group(
        &mut [&mut runtime, &mut remote],
        Tick::from_ticks(31),
    )
    .unwrap();
    let control_cut =
        OptimisticRuntime::fossil_collect_native_group(&mut [&mut control], Tick::from_ticks(31))
            .unwrap();
    let after_cut_root = runtime
        .native_intents()
        .unwrap()
        .into_iter()
        .find(|intent| intent.id() == &root_id)
        .unwrap();
    assert_eq!(after_cut_root.logical_id(), root_intent.logical_id());
    assert_eq!(after_cut_root.kind(), NativeIntentKind::Present);
    assert_eq!(
        runtime.accounting_snapshot().unwrap().source_intent_count(),
        1,
        "the lifetime root keeps its original transition reservation after child pruning"
    );
    assert_eq!(trace_rows(&split_cut), trace_rows(&control_cut));
}

#[test]
fn reduced_fanout_two_to_one_retires_omitted_ordinal_and_real_descendant() {
    let (mut a, mut b, mut c) = three_world(64);
    let parent = event(LP0, LP0, 1, &[0, 6, 1, 2, 1]);
    let late_root = event(LP1, LP0, 0, &[1, 0]);
    a.schedule_initial(1, parent.clone()).unwrap();
    b.schedule_initial(2, late_root.clone()).unwrap();
    let late_root_send = take_root_ticket(&mut b.ready_native_sends().unwrap(), LP1, 2);
    close_three(&mut a, &mut b, &mut c);

    let mut control = all_local(64);
    control.schedule_initial(1, parent).unwrap();
    control.schedule_initial(2, late_root).unwrap();
    control.close_initial_inputs().unwrap();

    assert_eq!(
        a.run_owned_until_with_budget(Tick::from_ticks(4), 1)
            .unwrap()
            .budget_used,
        1
    );
    let p_send = a
        .ready_native_sends()
        .unwrap()
        .into_iter()
        .find(|send| {
            send.message().kind() == OptimisticMessageKind::Positive
                && send.message().event().dest_lp == LP1
                && send.message().event().tick == Tick::from_ticks(2)
        })
        .expect("the initial parent emits P to LP1 at tick 2");
    let p_message = p_send.message().clone();
    let r_send = a
        .ready_native_sends()
        .unwrap()
        .into_iter()
        .find(|send| send.message().event().dest_lp == LP2)
        .unwrap();
    let r_message = r_send.message().clone();
    assert_eq!(
        a.ready_native_sends()
            .unwrap()
            .iter()
            .filter(|send| send.message().kind() == OptimisticMessageKind::Positive)
            .count(),
        2
    );
    let (parent_key, p_ordinal) = p_message.logical_id().output_parts().unwrap();
    let (r_parent_key, r_ordinal) = r_message.logical_id().output_parts().unwrap();
    assert_eq!((p_ordinal, r_ordinal), (0, 1));
    assert_eq!(parent_key, r_parent_key);
    assert_eq!(parent_key.logical_id().root_parts(), Some((LP0, 1)));

    assert_eq!(p_send.message().event().event_payload, vec![100, 7, 2, 1]);
    assert_eq!(r_message.event().event_payload, vec![100, 7, 2, 1]);
    deliver_positive(&mut a, &mut c, &r_send);
    assert_eq!(drain_one(&mut c, 4).budget_used, 1);

    let p_capability = b.admit_native(&p_send).unwrap();
    a.acknowledge_native_admission(p_capability).unwrap();
    assert_eq!(
        b.run_owned_until_with_budget(Tick::from_ticks(4), 1)
            .unwrap()
            .budget_used,
        1
    );
    let q_send = b
        .ready_native_sends()
        .unwrap()
        .into_iter()
        .find(|send| {
            send.message().kind() == OptimisticMessageKind::Positive
                && send.message().event().dest_lp == LP2
                && send.message().event().tick == Tick::from_ticks(3)
        })
        .expect("executing P emits descendant Q to LP2 at tick 3");
    let q_message = q_send.message().clone();
    let q_capability = c.admit_native(&q_send).unwrap();
    b.acknowledge_native_admission(q_capability).unwrap();
    assert_eq!(
        c.run_owned_until_with_budget(Tick::from_ticks(4), 1)
            .unwrap()
            .budget_used,
        1
    );
    assert_eq!(c.process_at(LP2).unwrap().value, 403);

    let late_root_capability = a.admit_native(&late_root_send).unwrap();
    b.acknowledge_native_admission(late_root_capability)
        .unwrap();
    assert_eq!(
        a.run_owned_until_with_budget(Tick::from_ticks(4), 1)
            .unwrap()
            .budget_used,
        1,
        "the straggler first commits its positive-triggered rollback unit"
    );
    assert_eq!(
        a.run_owned_until_with_budget(Tick::from_ticks(4), 1)
            .unwrap()
            .budget_used,
        1,
        "the late root then executes"
    );
    assert_eq!(
        a.run_owned_until_with_budget(Tick::from_ticks(4), 1)
            .unwrap()
            .budget_used,
        1,
        "replaying the parent emits a reduced two-to-one batch"
    );
    let p2_view = a
        .outbound_pending()
        .unwrap()
        .into_iter()
        .find(|view| {
            view.message().kind() == OptimisticMessageKind::Positive
                && view.message().logical_id() == p_message.logical_id()
                && view.message().incarnation() > p_message.incarnation()
        })
        .expect("the ordinary replay retains a replacement for P's exact logical slot");
    assert_eq!(p2_view.message().event().tick, Tick::from_ticks(2));
    assert_eq!(p2_view.message().event().dest_lp, LP1);
    assert_eq!(
        p2_view.status(),
        OptimisticOutboundStatus::BlockedReplacement
    );
    assert!(b.process_at(LP1).unwrap().seen.len() == 1);
    assert_eq!(c.process_at(LP2).unwrap().value, 403);

    assert_eq!(
        a.native_intents()
            .unwrap()
            .iter()
            .filter(|intent| intent.is_current()
                && intent.kind() == NativeIntentKind::Present
                && intent
                    .logical_id()
                    .output_parts()
                    .is_some_and(|(key, _)| key == parent_key))
            .count(),
        1
    );
    let r_retirement = a
        .pending_native_retirement_requests()
        .unwrap()
        .into_iter()
        .find(|request| request.predecessor_positive() == &r_message)
        .unwrap();
    let omitted = a
        .native_intents()
        .unwrap()
        .into_iter()
        .find(|intent| intent.is_current() && intent.logical_id() == r_message.logical_id())
        .unwrap();
    assert_eq!(omitted.kind(), NativeIntentKind::Absent);
    assert!(omitted.message().is_none());
    assert_eq!(omitted.id(), r_retirement.transition_id());
    assert_eq!(
        a.outbound_pending()
            .unwrap()
            .iter()
            .filter(
                |view| view.message().kind() == OptimisticMessageKind::Positive
                    && view.message().logical_id() == r_message.logical_id()
                    && view.message().incarnation() > r_message.incarnation()
            )
            .count(),
        0
    );
    let p_retirement = a
        .pending_native_retirement_requests()
        .unwrap()
        .into_iter()
        .find(|request| request.predecessor_positive() == &p_message)
        .expect("P retirement is addressed to its real LP1 receiver");
    let p_anti_capability = b.receive_native_retirement(&p_retirement).unwrap();
    a.acknowledge_native_admission(p_anti_capability).unwrap();
    assert_eq!(
        b.run_owned_until_with_budget(Tick::from_ticks(4), 1)
            .unwrap()
            .budget_used,
        1
    );
    assert_eq!(b.process_at(LP1).unwrap().value, 101);
    assert!(b.process_at(LP1).unwrap().seen.is_empty());
    let q_retirement = b
        .pending_native_retirement_requests()
        .unwrap()
        .into_iter()
        .find(|request| request.predecessor_positive() == &q_message)
        .expect("rolling P back publishes Q's exact descendant anti");
    let applied_p = b.applied_native_retirements().unwrap().pop().unwrap();
    a.acknowledge_native_retirement(applied_p).unwrap();
    let released_p2 = a
        .outbound_pending()
        .unwrap()
        .into_iter()
        .find(|view| view.intent_id() == p2_view.intent_id())
        .unwrap();
    assert_eq!(released_p2.status(), OptimisticOutboundStatus::Ready);
    let p2_send = a
        .ready_native_sends()
        .unwrap()
        .into_iter()
        .find(|send| send.message().incarnation() == p2_view.message().incarnation())
        .unwrap();
    let p2_capability = b.admit_native(&p2_send).unwrap();
    a.acknowledge_native_admission(p2_capability).unwrap();
    assert_eq!(
        b.run_owned_until_with_budget(Tick::from_ticks(4), 1)
            .unwrap()
            .budget_used,
        1
    );
    let q2_view = b
        .outbound_pending()
        .unwrap()
        .into_iter()
        .find(|view| {
            view.message().kind() == OptimisticMessageKind::Positive
                && view.message().logical_id() == q_message.logical_id()
                && view.message().incarnation() > q_message.incarnation()
        })
        .expect("P2's child keeps Q's same full parent order key");
    assert_eq!(q2_view.message().event().tick, Tick::from_ticks(3));
    assert_eq!(
        q2_view.status(),
        OptimisticOutboundStatus::BlockedReplacement
    );

    let q_anti_capability = c.receive_native_retirement(&q_retirement).unwrap();
    b.acknowledge_native_admission(q_anti_capability).unwrap();
    let before_blocked_cut = [
        a.process_at(LP0).unwrap().state(),
        b.process_at(LP1).unwrap().state(),
        c.process_at(LP2).unwrap().state(),
    ];
    let revisions = [
        a.accounting_revision().unwrap(),
        b.accounting_revision().unwrap(),
        c.accounting_revision().unwrap(),
    ];
    let rejected_cut = OptimisticRuntime::fossil_collect_native_group(
        &mut [&mut a, &mut b, &mut c],
        Tick::from_ticks(4),
    )
    .unwrap_err();
    assert!(matches!(
        rejected_cut.cause(),
        OptimisticError::GvtBeyondPending { requested, .. } if *requested == Tick::from_ticks(4)
    ));
    assert_eq!(a.process_at(LP0).unwrap().state(), before_blocked_cut[0]);
    assert_eq!(b.process_at(LP1).unwrap().state(), before_blocked_cut[1]);
    assert_eq!(c.process_at(LP2).unwrap().state(), before_blocked_cut[2]);
    assert_eq!(
        [
            a.accounting_revision().unwrap(),
            b.accounting_revision().unwrap(),
            c.accounting_revision().unwrap()
        ],
        revisions
    );

    assert_eq!(
        c.run_owned_until_with_budget(Tick::from_ticks(4), 1)
            .unwrap()
            .budget_used,
        1
    );
    assert_eq!(c.process_at(LP2).unwrap().value, 202);
    assert_eq!(c.process_at(LP2).unwrap().seen.len(), 1);
    let applied_q = c.applied_native_retirements().unwrap().pop().unwrap();
    b.acknowledge_native_retirement(applied_q).unwrap();
    let released_q2 = b
        .outbound_pending()
        .unwrap()
        .into_iter()
        .find(|view| view.intent_id() == q2_view.intent_id())
        .unwrap();
    assert_eq!(released_q2.status(), OptimisticOutboundStatus::Ready);
    let q2_send = b
        .ready_native_sends()
        .unwrap()
        .into_iter()
        .find(|send| send.message().incarnation() == q2_view.message().incarnation())
        .unwrap();

    // Q is actually canceled before R so R does not replay its descendant suffix.
    let r_anti_cap = c.receive_native_retirement(&r_retirement).unwrap();
    a.acknowledge_native_admission(r_anti_cap).unwrap();
    assert!(matches!(
        OptimisticRuntime::fossil_collect_native_group(
            &mut [&mut a, &mut b, &mut c],
            Tick::from_ticks(4)
        )
        .unwrap_err()
        .cause(),
        OptimisticError::GvtBeyondPending { .. }
    ));
    assert_eq!(drain_one(&mut c, 4).budget_used, 1);
    assert_eq!(c.process_at(LP2).unwrap().value, 102);
    assert!(c.process_at(LP2).unwrap().seen.is_empty());
    let applied_r = c
        .applied_native_retirements()
        .unwrap()
        .into_iter()
        .find(|proof| proof.transition_id() == r_retirement.transition_id())
        .unwrap();
    a.acknowledge_native_retirement(applied_r).unwrap();
    let q2_capability = c.admit_native(&q2_send).unwrap();
    b.acknowledge_native_admission(q2_capability).unwrap();
    assert_eq!(
        c.run_owned_until_with_budget(Tick::from_ticks(4), 1)
            .unwrap()
            .budget_used,
        1
    );
    assert_eq!(a.process_at(LP0).unwrap().value, 101);
    assert_eq!(b.process_at(LP1).unwrap().value, 202);
    assert_eq!(c.process_at(LP2).unwrap().value, 304);

    // Independent arithmetic/RNG oracle describes only final committed history.
    let rng = |lp: LpId, steps: &[(u64, u64)]| {
        steps.iter().fold(
            0x9e37_79b9_7f4a_7c15_u64 ^ u64::from(lp.0),
            |r, (tick, mode)| {
                r.wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(*tick)
                    .wrapping_add(*mode)
            },
        )
    };
    for (lp, runtime, value, steps, events) in [
        (
            LP0,
            &a,
            101,
            vec![(0, 0), (1, 6)],
            vec![
                event(LP1, LP0, 0, &[1, 0]),
                event(LP0, LP0, 1, &[0, 6, 1, 2, 1]),
            ],
        ),
        (
            LP1,
            &b,
            202,
            vec![(2, 7)],
            vec![event(LP0, LP1, 2, &[101, 7, 2, 1])],
        ),
        (
            LP2,
            &c,
            304,
            vec![(3, 0)],
            vec![event(LP1, LP2, 3, &[202, 0])],
        ),
    ] {
        let state = runtime.process_at(lp).unwrap().state();
        assert_eq!(state.value, value);
        assert_eq!(state.rng, rng(lp, &steps));
        assert_eq!(
            state
                .seen
                .iter()
                .map(|seen| seen.event.clone())
                .collect::<Vec<_>>(),
            events
        );
        assert_eq!(state.seen.last().unwrap().value, value);
        assert_eq!(state.seen.last().unwrap().rng, rng(lp, &steps));
    }
    drain_to_idle(&mut control, 4);
    for lp in [LP0, LP1, LP2] {
        let actual = match lp {
            LP0 => a.process_at(lp).unwrap().state(),
            LP1 => b.process_at(lp).unwrap().state(),
            LP2 => c.process_at(lp).unwrap().state(),
            _ => unreachable!(),
        };
        assert_eq!(actual, control.process_at(lp).unwrap().state());
    }
    let split_cut = OptimisticRuntime::fossil_collect_native_group(
        &mut [&mut a, &mut b, &mut c],
        Tick::from_ticks(4),
    )
    .unwrap();
    let control_cut =
        OptimisticRuntime::fossil_collect_native_group(&mut [&mut control], Tick::from_ticks(4))
            .unwrap();
    assert_eq!(trace_rows(&split_cut), trace_rows(&control_cut));
}
