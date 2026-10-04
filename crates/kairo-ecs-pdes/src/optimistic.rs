//! Event-owned optimistic execution for the local PDES runtime.
//!
//! This module owns logical delivery identity, rollback history, anti-message
//! routing, and the local GVT floor. It does not measure distributed GVT.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use kairo_ecs_types::SimDuration;

use super::{LpId, PartitionPlan, RemoteEvent, Tick};

const MAX_CAUSAL_DEPTH: usize = 128;
static NEXT_RUNTIME_ID: AtomicU64 = AtomicU64::new(1);

/// Model state required to undo any handler-visible logical mutation.
///
/// Implementations must include component membership, deterministic random
/// state, and reversible output state in `Snapshot`. Shared interior mutation
/// and irreversible external side effects are outside this runtime contract.
pub trait OptimisticProcess {
    type Snapshot: Clone;

    fn snapshot(&self) -> Self::Snapshot;

    fn restore(&mut self, state: &Self::Snapshot) -> Result<(), OptimisticStateError>;

    fn on_event(&mut self, event: &RemoteEvent) -> Vec<RemoteEvent>;
}

/// A process restore failure with a caller-provided diagnostic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OptimisticStateError {
    message: String,
}

impl OptimisticStateError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for OptimisticStateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for OptimisticStateError {}

/// Finite bounds for local optimistic-runtime storage and causal ancestry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OptimisticLimits {
    pub max_lps: usize,
    pub max_pending_events: usize,
    pub max_history_events: usize,
    pub max_tombstones: usize,
    pub max_output_batch: usize,
    pub max_causal_depth: usize,
}

impl Default for OptimisticLimits {
    fn default() -> Self {
        Self {
            max_lps: 1_024,
            max_pending_events: 100_000,
            max_history_events: 100_000,
            max_tombstones: 100_000,
            max_output_batch: 10_000,
            max_causal_depth: MAX_CAUSAL_DEPTH,
        }
    }
}

/// A typed optimistic runtime failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OptimisticError {
    InvalidLimits,
    ProcessSetMismatch {
        missing: Vec<LpId>,
        unexpected: Vec<LpId>,
    },
    MissingTopologyEntry(LpId),
    UnknownTopologySource(LpId),
    UnknownTopologyDestination {
        source: LpId,
        destination: LpId,
    },
    DuplicateNeighbor {
        source: LpId,
        destination: LpId,
    },
    UnknownLogicalProcess(LpId),
    ScopedAuthorityRequiresOwnedRuntime,
    RouteMissing {
        source: LpId,
        destination: LpId,
    },
    InitialSchedulingClosed,
    EventBeforeGvt {
        event_tick: Tick,
        gvt: Tick,
    },
    GvtRegression {
        current: Tick,
        requested: Tick,
    },
    GvtBeyondPending {
        requested: Tick,
        pending: Tick,
    },
    HorizonRegression {
        previous: Tick,
        requested: Tick,
    },
    DuplicatePositive {
        source_lp: LpId,
        incarnation: u64,
    },
    ConflictingDelivery {
        source_lp: LpId,
        incarnation: u64,
    },
    ConflictingLogicalEvent {
        source_lp: LpId,
    },
    EnvelopeSourceMismatch {
        declared: LpId,
        actual: LpId,
    },
    PendingLimitExceeded {
        limit: usize,
    },
    HistoryLimitExceeded {
        limit: usize,
    },
    TombstoneLimitExceeded {
        limit: usize,
    },
    OutputBatchTooLarge {
        actual: usize,
        limit: usize,
    },
    OutputSourceMismatch {
        lp_id: LpId,
        declared: LpId,
    },
    OutputNotStrictlyFuture {
        input_tick: Tick,
        output_tick: Tick,
    },
    CausalDepthExceeded {
        depth: usize,
        limit: usize,
    },
    IncarnationExhausted(LpId),
    RuntimeIdentityExhausted,
    EpochExhausted(LpId),
    SnapshotPanicked(LpId),
    RestorePanicked(LpId),
    HandlerPanicked(LpId),
    RestoreFailed {
        lp_id: LpId,
        reason: OptimisticStateError,
    },
    Poisoned,
}

impl fmt::Display for OptimisticError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "optimistic PDES runtime error: {self:?}")
    }
}

impl std::error::Error for OptimisticError {}

/// Stable structural identity for one logical event, independent of arrival.
///
/// The node is private and immutable. Public constructors can only create a
/// root or append one checked child to an existing complete ordering key.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct LogicalEventId(Arc<LogicalNode>);

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum LogicalNode {
    Root {
        source_lp: LpId,
        sequence: u64,
    },
    Output {
        parent: OptimisticEventOrderKey,
        ordinal: u32,
    },
}

impl LogicalEventId {
    /// Creates a depth-zero root identity. The caller must use the event's
    /// actual `source_lp` as `source_lp` when constructing an envelope.
    pub fn root(source_lp: LpId, sequence: u64) -> Self {
        Self(Arc::new(LogicalNode::Root {
            source_lp,
            sequence,
        }))
    }

    /// Creates a structural child from a complete parent ordering key.
    pub fn child(parent: &OptimisticEventOrderKey, ordinal: u32) -> Result<Self, OptimisticError> {
        Self::child_with_limit(parent, ordinal, MAX_CAUSAL_DEPTH)
    }

    fn child_with_limit(
        parent: &OptimisticEventOrderKey,
        ordinal: u32,
        max_depth: usize,
    ) -> Result<Self, OptimisticError> {
        let depth = parent.logical_id.depth().saturating_add(1);
        if depth > max_depth || depth > MAX_CAUSAL_DEPTH {
            return Err(OptimisticError::CausalDepthExceeded {
                depth,
                limit: max_depth.min(MAX_CAUSAL_DEPTH),
            });
        }
        Ok(Self(Arc::new(LogicalNode::Output {
            parent: parent.clone(),
            ordinal,
        })))
    }

    pub fn depth(&self) -> usize {
        match self.0.as_ref() {
            LogicalNode::Root { .. } => 0,
            LogicalNode::Output { parent, .. } => parent.logical_id.depth() + 1,
        }
    }

    pub fn root_parts(&self) -> Option<(LpId, u64)> {
        match self.0.as_ref() {
            LogicalNode::Root {
                source_lp,
                sequence,
            } => Some((*source_lp, *sequence)),
            LogicalNode::Output { .. } => None,
        }
    }

    /// Returns the complete parent ordering key and output ordinal for an
    /// Output identity. Root identities have no parent or output ordinal.
    pub fn output_parts(&self) -> Option<(&OptimisticEventOrderKey, u32)> {
        match self.0.as_ref() {
            LogicalNode::Root { .. } => None,
            LogicalNode::Output { parent, ordinal } => Some((parent, *ordinal)),
        }
    }
}

/// Complete deterministic event ordering key `(tick, actual source, logical id)`.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct OptimisticEventOrderKey {
    tick: Tick,
    source_lp: LpId,
    logical_id: LogicalEventId,
}

impl OptimisticEventOrderKey {
    fn new(event: &RemoteEvent, logical_id: LogicalEventId) -> Self {
        Self {
            tick: event.tick,
            source_lp: event.source_lp,
            logical_id,
        }
    }

    /// Constructs an order key after validating its entire causal ancestry.
    ///
    /// This validates structural identity only. It does not admit an event to
    /// a runtime or authenticate a transport sender.
    pub fn try_from_parts(
        tick: Tick,
        source_lp: LpId,
        logical_id: LogicalEventId,
    ) -> Result<Self, OptimisticError> {
        validate_complete_ancestry(tick, source_lp, &logical_id)?;
        Ok(Self {
            tick,
            source_lp,
            logical_id,
        })
    }

    pub fn tick(&self) -> Tick {
        self.tick
    }

    pub fn source_lp(&self) -> LpId {
        self.source_lp
    }

    pub fn logical_id(&self) -> &LogicalEventId {
        &self.logical_id
    }
}

/// Positive deliveries apply work; anti-messages cancel an exact delivery.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum OptimisticMessageKind {
    Positive,
    Anti,
}

/// Declares the authority namespace carried by an optimistic envelope.
///
/// This metadata does not affect logical identity or ordering. Scoped values
/// can be reconstructed, but the all-local runtime rejects them.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OptimisticAuthority {
    /// Legacy single-runtime scheduling and preview delivery.
    LocalPreview,
    /// A future owned runtime's simulation namespace and ownership epoch.
    Scoped {
        simulation_namespace: u128,
        ownership_epoch: u64,
    },
}

/// Transportable event envelope with an opaque logical ID and exact incarnation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OptimisticMessage {
    event: RemoteEvent,
    logical_id: LogicalEventId,
    authority: OptimisticAuthority,
    incarnation: u64,
    kind: OptimisticMessageKind,
}

impl OptimisticMessage {
    fn new(
        event: RemoteEvent,
        logical_id: LogicalEventId,
        incarnation: u64,
        kind: OptimisticMessageKind,
    ) -> Self {
        Self::new_with_authority(
            event,
            logical_id,
            OptimisticAuthority::LocalPreview,
            incarnation,
            kind,
        )
    }

    fn new_with_authority(
        event: RemoteEvent,
        logical_id: LogicalEventId,
        authority: OptimisticAuthority,
        incarnation: u64,
        kind: OptimisticMessageKind,
    ) -> Self {
        Self {
            event,
            logical_id,
            authority,
            incarnation,
            kind,
        }
    }

    /// Reconstructs a transport envelope while checking the complete logical
    /// ancestry against the event's tick and actual source LP.
    ///
    /// A successful reconstruction does not authenticate the sender or bypass
    /// the destination runtime's topology, GVT, capacity, or duplicate checks.
    pub fn try_from_parts(
        event: RemoteEvent,
        logical_id: LogicalEventId,
        incarnation: u64,
        kind: OptimisticMessageKind,
    ) -> Result<Self, OptimisticError> {
        Self::try_from_authority_parts(
            event,
            logical_id,
            OptimisticAuthority::LocalPreview,
            incarnation,
            kind,
        )
    }

    /// Reconstructs an envelope with explicit authority metadata while
    /// validating its complete logical ancestry. This does not authenticate
    /// the sender or grant scoped execution authority.
    pub fn try_from_authority_parts(
        event: RemoteEvent,
        logical_id: LogicalEventId,
        authority: OptimisticAuthority,
        incarnation: u64,
        kind: OptimisticMessageKind,
    ) -> Result<Self, OptimisticError> {
        OptimisticEventOrderKey::try_from_parts(event.tick, event.source_lp, logical_id.clone())?;
        Ok(Self::new_with_authority(
            event,
            logical_id,
            authority,
            incarnation,
            kind,
        ))
    }

    /// Returns the immutable model event, including its original payload bytes.
    pub fn event(&self) -> &RemoteEvent {
        &self.event
    }

    pub fn logical_id(&self) -> &LogicalEventId {
        &self.logical_id
    }

    pub fn incarnation(&self) -> u64 {
        self.incarnation
    }

    pub fn authority(&self) -> OptimisticAuthority {
        self.authority
    }

    pub fn kind(&self) -> OptimisticMessageKind {
        self.kind
    }

    pub fn order_key(&self) -> OptimisticEventOrderKey {
        OptimisticEventOrderKey::new(&self.event, self.logical_id.clone())
    }

    /// Produces an anti for this exact source, logical ID, incarnation and event.
    pub fn as_anti(&self) -> Self {
        Self {
            kind: OptimisticMessageKind::Anti,
            ..self.clone()
        }
    }
}

/// Validates every key encoded by a logical identity without recursive calls.
/// The walk is bounded before following more than the native depth limit.
fn validate_complete_ancestry(
    tick: Tick,
    source_lp: LpId,
    logical_id: &LogicalEventId,
) -> Result<(), OptimisticError> {
    let mut current_tick = tick;
    let mut current_source = source_lp;
    let mut current_id = logical_id;
    let mut output_depth = 0usize;

    loop {
        match current_id.0.as_ref() {
            LogicalNode::Root {
                source_lp: declared,
                ..
            } => {
                if *declared != current_source {
                    return Err(OptimisticError::EnvelopeSourceMismatch {
                        declared: *declared,
                        actual: current_source,
                    });
                }
                return Ok(());
            }
            LogicalNode::Output { parent, .. } => {
                output_depth += 1;
                if output_depth > MAX_CAUSAL_DEPTH {
                    return Err(OptimisticError::CausalDepthExceeded {
                        depth: output_depth,
                        limit: MAX_CAUSAL_DEPTH,
                    });
                }
                if current_tick <= parent.tick {
                    return Err(OptimisticError::OutputNotStrictlyFuture {
                        input_tick: parent.tick,
                        output_tick: current_tick,
                    });
                }
                current_tick = parent.tick;
                current_source = parent.source_lp;
                current_id = &parent.logical_id;
            }
        }
    }
}

/// Per-call progress from `run_until_with_budget`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OptimisticRunProgress {
    pub budget_used: usize,
    pub budget_remaining: usize,
    pub budget_exhausted: bool,
    pub pending_positives: usize,
    pub pending_antis: usize,
    pub replay_pending: usize,
    pub published_messages: Vec<OptimisticMessage>,
}

/// Cumulative runtime counters and current bounded-storage pressure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OptimisticRuntimeReport {
    pub gvt: Tick,
    /// Maximum per-LP executed/fossil frontier minus GVT, saturating at zero.
    pub gvt_lag: SimDuration,
    pub logical_processes: usize,
    pub pending_events: usize,
    pub pending_positives: usize,
    pub pending_antis: usize,
    pub replay_pending: usize,
    pub history_events: usize,
    pub checkpoints: usize,
    pub tombstones: usize,
    pub executions: u64,
    pub replay_executions: u64,
    pub rollback_attempts: u64,
    pub rolled_back_events: u64,
    pub max_rollback_depth: usize,
    pub canceled_sends: u64,
    pub fossil_collected_events: u64,
}

/// One event made irreversible by a successful local fossil collection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OptimisticTraceEntry {
    pub lp_id: LpId,
    pub event: RemoteEvent,
    pub logical_id: LogicalEventId,
}

/// Result of advancing and applying a caller-proven local GVT floor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OptimisticFossilReport {
    pub previous_gvt: Tick,
    pub new_gvt: Tick,
    pub collected: Vec<OptimisticTraceEntry>,
    pub collected_checkpoints: usize,
    pub retained_history: usize,
    pub oldest_retained_tick: Option<Tick>,
    pub collected_tombstones: usize,
}

/// Runtime-bound validity token for one logical process state epoch.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OptimisticStateToken {
    runtime_id: u64,
    lp_id: LpId,
    epoch: u64,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct DeliveryIdentity {
    source_lp: LpId,
    logical_id: LogicalEventId,
    incarnation: u64,
}

impl From<&OptimisticMessage> for DeliveryIdentity {
    fn from(message: &OptimisticMessage) -> Self {
        Self {
            source_lp: message.event.source_lp,
            logical_id: message.logical_id.clone(),
            incarnation: message.incarnation,
        }
    }
}

#[derive(Clone, Debug)]
struct ExecutedEvent<S> {
    message: OptimisticMessage,
    key: OptimisticEventOrderKey,
    before: S,
    outputs: Vec<OptimisticMessage>,
}

type StagedEvent<S> = (LpId, ExecutedEvent<S>);

#[derive(Debug, Default)]
struct EventQueue {
    values: BTreeMap<OptimisticEventOrderKey, BTreeMap<u64, OptimisticMessage>>,
}

impl EventQueue {
    fn len(&self) -> usize {
        self.values.values().map(BTreeMap::len).sum()
    }

    fn contains(&self, identity: &DeliveryIdentity) -> Option<&OptimisticMessage> {
        self.values.values().find_map(|incarnations| {
            incarnations
                .get(&identity.incarnation)
                .filter(|message| DeliveryIdentity::from(*message) == *identity)
        })
    }

    fn insert(&mut self, message: OptimisticMessage) {
        self.values
            .entry(message.order_key())
            .or_default()
            .insert(message.incarnation, message);
    }

    fn remove(&mut self, identity: &DeliveryIdentity) -> Option<OptimisticMessage> {
        let key = self.values.iter().find_map(|(key, incarnations)| {
            incarnations
                .get(&identity.incarnation)
                .filter(|message| DeliveryIdentity::from(*message) == *identity)
                .map(|_| key.clone())
        })?;
        let incarnations = self.values.get_mut(&key)?;
        let removed = incarnations.remove(&identity.incarnation);
        if incarnations.is_empty() {
            self.values.remove(&key);
        }
        removed
    }

    fn first(&self) -> Option<&OptimisticMessage> {
        self.values
            .first_key_value()
            .and_then(|(_, incarnations)| incarnations.first_key_value().map(|(_, m)| m))
    }

    fn first_through(&self, horizon: Tick) -> Option<&OptimisticMessage> {
        self.values
            .first_key_value()
            .filter(|(key, _)| key.tick <= horizon)
            .and_then(|(_, incarnations)| incarnations.first_key_value().map(|(_, m)| m))
    }

    fn all(&self) -> Vec<OptimisticMessage> {
        self.values
            .values()
            .flat_map(|incarnations| incarnations.values().cloned())
            .collect()
    }
}

struct LogicalProcessState<P: OptimisticProcess> {
    process: P,
    _initial_snapshot: P::Snapshot,
    positives: EventQueue,
    antis: EventQueue,
    history: Vec<ExecutedEvent<P::Snapshot>>,
    replay_pending: BTreeSet<DeliveryIdentity>,
    tombstones: BTreeMap<DeliveryIdentity, RemoteEvent>,
    epoch: u64,
    fossil_time: Tick,
}

impl<P: OptimisticProcess> LogicalProcessState<P> {
    fn pending_len(&self) -> usize {
        self.positives.len() + self.antis.len()
    }

    fn local_time(&self) -> Tick {
        self.history
            .last()
            .map(|entry| entry.key.tick)
            .unwrap_or(self.fossil_time)
            .max(self.fossil_time)
    }
}

#[derive(Default)]
struct Counters {
    executions: u64,
    replay_executions: u64,
    rollback_attempts: u64,
    rolled_back_events: u64,
    max_rollback_depth: usize,
    canceled_sends: u64,
    fossil_collected_events: u64,
}

/// Deterministic in-process optimistic PDES runtime with exact rollback identity.
pub struct OptimisticRuntime<P: OptimisticProcess> {
    runtime_id: u64,
    topology: BTreeMap<LpId, Vec<LpId>>,
    processes: BTreeMap<LpId, LogicalProcessState<P>>,
    next_incarnation: BTreeMap<LpId, Option<u64>>,
    known_deliveries: BTreeMap<DeliveryIdentity, RemoteEvent>,
    gvt: Tick,
    limits: OptimisticLimits,
    counters: Counters,
    last_horizon: Option<Tick>,
    initial_open: bool,
    poisoned: bool,
}

impl<P: OptimisticProcess> OptimisticRuntime<P> {
    /// Creates a local optimistic driver after validating the complete LP set
    /// and directed topology. Initial snapshots are captured before execution.
    pub fn new(
        partition: PartitionPlan,
        topology: BTreeMap<LpId, Vec<LpId>>,
        processes: BTreeMap<LpId, P>,
        limits: OptimisticLimits,
    ) -> Result<Self, OptimisticError> {
        validate_limits(limits)?;
        if processes.len() > limits.max_lps {
            return Err(OptimisticError::InvalidLimits);
        }
        let lp_ids = partition
            .segments()
            .iter()
            .map(|segment| segment.id)
            .collect::<BTreeSet<_>>();
        let process_ids = processes.keys().copied().collect::<BTreeSet<_>>();
        if lp_ids != process_ids {
            return Err(OptimisticError::ProcessSetMismatch {
                missing: lp_ids.difference(&process_ids).copied().collect(),
                unexpected: process_ids.difference(&lp_ids).copied().collect(),
            });
        }
        for lp_id in &lp_ids {
            if !topology.contains_key(lp_id) {
                return Err(OptimisticError::MissingTopologyEntry(*lp_id));
            }
        }
        for (&source, destinations) in &topology {
            if !lp_ids.contains(&source) {
                return Err(OptimisticError::UnknownTopologySource(source));
            }
            let mut unique = BTreeSet::new();
            for &destination in destinations {
                if !lp_ids.contains(&destination) {
                    return Err(OptimisticError::UnknownTopologyDestination {
                        source,
                        destination,
                    });
                }
                if source == destination {
                    return Err(OptimisticError::DuplicateNeighbor {
                        source,
                        destination,
                    });
                }
                if !unique.insert(destination) {
                    return Err(OptimisticError::DuplicateNeighbor {
                        source,
                        destination,
                    });
                }
            }
        }
        #[allow(deprecated)]
        let runtime_id = NEXT_RUNTIME_ID
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |value| {
                value.checked_add(1)
            })
            .map_err(|_| OptimisticError::RuntimeIdentityExhausted)?;
        let mut states = BTreeMap::new();
        for (lp_id, process) in processes {
            let snapshot = catch_unwind(AssertUnwindSafe(|| process.snapshot()))
                .map_err(|_| OptimisticError::SnapshotPanicked(lp_id))?;
            states.insert(
                lp_id,
                LogicalProcessState {
                    process,
                    _initial_snapshot: snapshot,
                    positives: EventQueue::default(),
                    antis: EventQueue::default(),
                    history: Vec::new(),
                    replay_pending: BTreeSet::new(),
                    tombstones: BTreeMap::new(),
                    epoch: 0,
                    fossil_time: Tick::ZERO,
                },
            );
        }
        Ok(Self {
            runtime_id,
            topology,
            processes: states,
            next_incarnation: lp_ids.into_iter().map(|lp| (lp, Some(0))).collect(),
            known_deliveries: BTreeMap::new(),
            gvt: Tick::ZERO,
            limits,
            counters: Counters::default(),
            last_horizon: None,
            initial_open: true,
            poisoned: false,
        })
    }

    /// Adds an initial root event with a stable caller sequence. Root source is
    /// derived from `event.source_lp`; no arrival counter contributes to order.
    pub fn schedule_initial(
        &mut self,
        stable_sequence: u64,
        event: RemoteEvent,
    ) -> Result<OptimisticMessage, OptimisticError> {
        self.ensure_healthy()?;
        if !self.initial_open {
            return Err(OptimisticError::InitialSchedulingClosed);
        }
        self.validate_event(&event)?;
        self.validate_gvt(event.tick)?;
        let lp_id = event.dest_lp;
        let logical_id = LogicalEventId::root(event.source_lp, stable_sequence);
        if self.known_deliveries.keys().any(|identity| {
            identity.source_lp == event.source_lp && identity.logical_id == logical_id
        }) {
            return Err(OptimisticError::ConflictingLogicalEvent {
                source_lp: event.source_lp,
            });
        }
        self.ensure_pending_capacity(1)?;
        let next_epoch = self.next_epoch(lp_id)?;
        let message = self.make_message(event, logical_id, OptimisticMessageKind::Positive)?;
        self.validate_logical_event(&message)?;
        self.observe_incarnation(message.event.source_lp, message.incarnation);
        let state = self.processes.get_mut(&lp_id).expect("validated LP");
        state.epoch = next_epoch;
        state.positives.insert(message.clone());
        self.known_deliveries
            .insert(DeliveryIdentity::from(&message), message.event.clone());
        Ok(message)
    }

    /// Queues a transport envelope. Malformed or pre-GVT deliveries are rejected
    /// before changing any queue, token epoch, tombstone, or process state.
    pub fn receive(&mut self, message: OptimisticMessage) -> Result<(), OptimisticError> {
        self.ensure_healthy()?;
        self.validate_message(&message)?;
        self.validate_gvt(message.event.tick)?;
        self.validate_logical_event(&message)?;
        let identity = DeliveryIdentity::from(&message);
        let destination = message.event.dest_lp;
        let state = self.processes.get(&destination).expect("validated LP");
        if let Some(existing) = state.tombstones.get(&identity) {
            return if existing == &message.event {
                Ok(())
            } else {
                Err(OptimisticError::ConflictingDelivery {
                    source_lp: identity.source_lp,
                    incarnation: identity.incarnation,
                })
            };
        }
        if let Some(existing) = state.antis.contains(&identity) {
            return if existing.event == message.event {
                Ok(())
            } else {
                Err(OptimisticError::ConflictingDelivery {
                    source_lp: identity.source_lp,
                    incarnation: identity.incarnation,
                })
            };
        }
        match message.kind {
            OptimisticMessageKind::Positive => {
                if let Some(existing) = state.positives.contains(&identity) {
                    return Err(if existing.event == message.event {
                        OptimisticError::DuplicatePositive {
                            source_lp: identity.source_lp,
                            incarnation: identity.incarnation,
                        }
                    } else {
                        OptimisticError::ConflictingDelivery {
                            source_lp: identity.source_lp,
                            incarnation: identity.incarnation,
                        }
                    });
                }
                if let Some(existing) = state
                    .history
                    .iter()
                    .find(|entry| DeliveryIdentity::from(&entry.message) == identity)
                {
                    return Err(if existing.message.event == message.event {
                        OptimisticError::DuplicatePositive {
                            source_lp: identity.source_lp,
                            incarnation: identity.incarnation,
                        }
                    } else {
                        OptimisticError::ConflictingDelivery {
                            source_lp: identity.source_lp,
                            incarnation: identity.incarnation,
                        }
                    });
                }
                self.ensure_pending_capacity(1)?;
                let next_epoch = self.next_epoch(destination)?;
                self.observe_incarnation(identity.source_lp, identity.incarnation);
                let event_for_index = message.event.clone();
                let state = self.processes.get_mut(&destination).expect("validated LP");
                state.epoch = next_epoch;
                state.positives.insert(message);
                self.known_deliveries.insert(identity, event_for_index);
            }
            OptimisticMessageKind::Anti => {
                self.check_tombstone_reservation(destination, &identity)?;
                self.ensure_pending_capacity(1)?;
                let next_epoch = self.next_epoch(destination)?;
                self.observe_incarnation(identity.source_lp, identity.incarnation);
                let event_for_index = message.event.clone();
                let state = self.processes.get_mut(&destination).expect("validated LP");
                state.epoch = next_epoch;
                state.antis.insert(message);
                self.known_deliveries.insert(identity, event_for_index);
            }
        }
        Ok(())
    }

    /// Executes deterministic LP rounds until the horizon or resumable work
    /// budget is reached. The budget counts handlers and queued anti steps.
    pub fn run_until_with_budget(
        &mut self,
        horizon: Tick,
        budget: usize,
    ) -> Result<OptimisticRunProgress, OptimisticError> {
        self.ensure_healthy()?;
        if let Some(previous) = self.last_horizon {
            if horizon < previous {
                return Err(OptimisticError::HorizonRegression {
                    previous,
                    requested: horizon,
                });
            }
        }
        self.last_horizon = Some(horizon);
        let mut budget_used = 0usize;
        let mut published = Vec::new();
        while budget_used < budget {
            if self.has_antis_through(horizon) {
                let selected = self.select_antis(horizon, budget - budget_used);
                if selected.is_empty() {
                    break;
                }
                for message in selected {
                    self.initial_open = false;
                    self.remove_pending(&message)?;
                    if let Err(error) = self.process_anti(message.clone(), &mut published) {
                        if !self.poisoned {
                            self.processes
                                .get_mut(&message.event.dest_lp)
                                .expect("validated LP")
                                .antis
                                .insert(message);
                        }
                        return Err(error);
                    }
                    budget_used += 1;
                }
                continue;
            }

            let selected = self.select_positives(horizon, budget - budget_used);
            if selected.is_empty() {
                break;
            }
            let mut rollback_started = false;
            for message in &selected {
                let lp_id = message.event.dest_lp;
                let key = message.order_key();
                let suffix = self.processes[&lp_id]
                    .history
                    .iter()
                    .position(|entry| entry.key > key);
                if let Some(index) = suffix {
                    self.rollback_suffix(lp_id, index, None, &mut published)?;
                    rollback_started = true;
                    break;
                }
            }
            if rollback_started {
                continue;
            }
            self.initial_open = false;
            let staged = self.execute_round(selected)?;
            budget_used += staged.len();
            let selected_lps = staged.iter().map(|(lp, _)| *lp).collect::<BTreeSet<_>>();
            let output_destinations = staged
                .iter()
                .flat_map(|(_, record)| record.outputs.iter().map(|output| output.event.dest_lp))
                .collect::<BTreeSet<_>>();
            for output in staged.iter().flat_map(|(_, record)| &record.outputs) {
                let identity = DeliveryIdentity::from(output);
                if self
                    .known_deliveries
                    .get(&identity)
                    .is_some_and(|existing| existing != &output.event)
                {
                    self.poisoned = true;
                    return Err(OptimisticError::ConflictingDelivery {
                        source_lp: identity.source_lp,
                        incarnation: identity.incarnation,
                    });
                }
            }
            for destination in output_destinations.difference(&selected_lps) {
                self.next_epoch(*destination).inspect_err(|_| {
                    self.poisoned = true;
                })?;
            }
            for destination in output_destinations.difference(&selected_lps).copied() {
                self.bump_epoch(destination).inspect_err(|_| {
                    self.poisoned = true;
                })?;
            }
            for (lp_id, record) in staged {
                for output in &record.outputs {
                    let identity = DeliveryIdentity::from(output);
                    let destination = self
                        .processes
                        .get_mut(&output.event.dest_lp)
                        .expect("output was validated before handler commit");
                    match output.kind {
                        OptimisticMessageKind::Positive => {
                            destination.positives.insert(output.clone());
                        }
                        OptimisticMessageKind::Anti => {
                            destination.antis.insert(output.clone());
                        }
                    }
                    self.known_deliveries.insert(identity, output.event.clone());
                    published.push(output.clone());
                }
                self.processes
                    .get_mut(&lp_id)
                    .expect("selected LP remains present")
                    .history
                    .push(record);
            }
        }
        let pending_positives = self.total_positives();
        let pending_antis = self.total_antis();
        let replay_pending = self.total_replay_pending();
        let has_runnable = self.has_antis_through(horizon) || self.has_positives_through(horizon);
        Ok(OptimisticRunProgress {
            budget_used,
            budget_remaining: budget.saturating_sub(budget_used),
            budget_exhausted: budget_used == budget && has_runnable,
            pending_positives,
            pending_antis,
            replay_pending,
            published_messages: published,
        })
    }

    /// Advances a caller-proven local GVT floor and returns the newly committed
    /// logical trace. Pending work, including anti-messages, bounds the floor.
    pub fn fossil_collect(&mut self, gvt: Tick) -> Result<OptimisticFossilReport, OptimisticError> {
        self.ensure_healthy()?;
        if gvt < self.gvt {
            return Err(OptimisticError::GvtRegression {
                current: self.gvt,
                requested: gvt,
            });
        }
        if let Some(pending) = self.minimum_pending_tick() {
            if gvt > pending {
                return Err(OptimisticError::GvtBeyondPending {
                    requested: gvt,
                    pending,
                });
            }
        }
        let previous_gvt = self.gvt;
        let mut collected = Vec::new();
        let mut removed_checkpoints = 0usize;
        let mut touched = BTreeMap::new();
        for (&lp_id, state) in &self.processes {
            let count = state
                .history
                .iter()
                .take_while(|entry| entry.key.tick < gvt)
                .count();
            if count > 0 || gvt != previous_gvt {
                touched.insert(lp_id, self.next_epoch(lp_id)?);
            }
        }
        for (&lp_id, &next_epoch) in &touched {
            let state = self.processes.get_mut(&lp_id).expect("known LP");
            state.epoch = next_epoch;
            let count = state
                .history
                .iter()
                .take_while(|entry| entry.key.tick < gvt)
                .count();
            for entry in state.history.drain(..count) {
                state.fossil_time = state.fossil_time.max(entry.key.tick);
                collected.push(OptimisticTraceEntry {
                    lp_id,
                    event: entry.message.event,
                    logical_id: entry.message.logical_id,
                });
            }
            removed_checkpoints += count;
        }
        collected.sort_by_key(|entry| {
            OptimisticEventOrderKey::new(&entry.event, entry.logical_id.clone())
        });
        self.gvt = gvt;
        let mut collected_tombstones = 0usize;
        for state in self.processes.values_mut() {
            let old = state
                .tombstones
                .iter()
                .filter(|(_, event)| event.tick < gvt)
                .map(|(identity, _)| identity.clone())
                .collect::<Vec<_>>();
            for identity in old {
                state.tombstones.remove(&identity);
                collected_tombstones += 1;
            }
        }
        self.known_deliveries.retain(|_, event| event.tick >= gvt);
        self.counters.fossil_collected_events = self
            .counters
            .fossil_collected_events
            .saturating_add(collected.len() as u64);
        let retained_history = self.total_history();
        let oldest_retained_tick = self
            .processes
            .values()
            .flat_map(|state| state.history.iter().map(|entry| entry.key.tick))
            .min();
        Ok(OptimisticFossilReport {
            previous_gvt,
            new_gvt: gvt,
            collected,
            collected_checkpoints: removed_checkpoints,
            retained_history,
            oldest_retained_tick,
            collected_tombstones,
        })
    }

    /// Returns cumulative counters and current bounded-storage pressure.
    pub fn report(&self) -> OptimisticRuntimeReport {
        let maximum_local = self
            .processes
            .values()
            .map(LogicalProcessState::local_time)
            .max()
            .unwrap_or(Tick::ZERO);
        OptimisticRuntimeReport {
            gvt: self.gvt,
            gvt_lag: maximum_local
                .duration_since(self.gvt)
                .unwrap_or(SimDuration::ZERO),
            logical_processes: self.processes.len(),
            pending_events: self.total_pending(),
            pending_positives: self.total_positives(),
            pending_antis: self.total_antis(),
            replay_pending: self.total_replay_pending(),
            history_events: self.total_history(),
            checkpoints: self.total_history() + self.processes.len(),
            tombstones: self.total_tombstones(),
            executions: self.counters.executions,
            replay_executions: self.counters.replay_executions,
            rollback_attempts: self.counters.rollback_attempts,
            rolled_back_events: self.counters.rolled_back_events,
            max_rollback_depth: self.counters.max_rollback_depth,
            canceled_sends: self.counters.canceled_sends,
            fossil_collected_events: self.counters.fossil_collected_events,
        }
    }

    /// Returns a sorted snapshot of the chosen LP's pending envelopes.
    pub fn pending_events(&self, lp_id: LpId) -> Option<Vec<OptimisticMessage>> {
        let state = self.processes.get(&lp_id)?;
        let mut messages = state.antis.all();
        messages.extend(state.positives.all());
        messages.sort_by(|left, right| {
            left.order_key()
                .cmp(&right.order_key())
                .then_with(|| left.kind.cmp(&right.kind))
                .then_with(|| left.incarnation.cmp(&right.incarnation))
        });
        Some(messages)
    }

    /// Creates an opaque token for the current LP epoch.
    pub fn state_token(&self, lp_id: LpId) -> Result<OptimisticStateToken, OptimisticError> {
        self.ensure_healthy()?;
        let state = self
            .processes
            .get(&lp_id)
            .ok_or(OptimisticError::UnknownLogicalProcess(lp_id))?;
        Ok(OptimisticStateToken {
            runtime_id: self.runtime_id,
            lp_id,
            epoch: state.epoch,
        })
    }

    /// Checks that a token belongs to this runtime and remains in the LP epoch.
    pub fn validate_state_token(&self, token: OptimisticStateToken) -> bool {
        token.runtime_id == self.runtime_id
            && self
                .processes
                .get(&token.lp_id)
                .is_some_and(|state| state.epoch == token.epoch)
            && !self.poisoned
    }

    /// Exposes a process immutably so all logical mutations remain event-owned.
    pub fn process_at(&self, lp_id: LpId) -> Option<&P> {
        self.processes.get(&lp_id).map(|state| &state.process)
    }

    fn ensure_healthy(&self) -> Result<(), OptimisticError> {
        if self.poisoned {
            Err(OptimisticError::Poisoned)
        } else {
            Ok(())
        }
    }

    fn validate_event(&self, event: &RemoteEvent) -> Result<(), OptimisticError> {
        if !self.processes.contains_key(&event.source_lp) {
            return Err(OptimisticError::UnknownLogicalProcess(event.source_lp));
        }
        if !self.processes.contains_key(&event.dest_lp) {
            return Err(OptimisticError::UnknownLogicalProcess(event.dest_lp));
        }
        if event.source_lp != event.dest_lp
            && !self
                .topology
                .get(&event.source_lp)
                .is_some_and(|destinations| destinations.contains(&event.dest_lp))
        {
            return Err(OptimisticError::RouteMissing {
                source: event.source_lp,
                destination: event.dest_lp,
            });
        }
        Ok(())
    }

    fn validate_message(&self, message: &OptimisticMessage) -> Result<(), OptimisticError> {
        if matches!(message.authority, OptimisticAuthority::Scoped { .. }) {
            return Err(OptimisticError::ScopedAuthorityRequiresOwnedRuntime);
        }
        self.validate_event(&message.event)?;
        if let Some((root_source, _)) = message.logical_id.root_parts() {
            if root_source != message.event.source_lp {
                return Err(OptimisticError::EnvelopeSourceMismatch {
                    declared: root_source,
                    actual: message.event.source_lp,
                });
            }
        }
        if message.logical_id.depth() > self.limits.max_causal_depth {
            return Err(OptimisticError::CausalDepthExceeded {
                depth: message.logical_id.depth(),
                limit: self.limits.max_causal_depth,
            });
        }
        Ok(())
    }

    fn validate_gvt(&self, event_tick: Tick) -> Result<(), OptimisticError> {
        if event_tick < self.gvt {
            Err(OptimisticError::EventBeforeGvt {
                event_tick,
                gvt: self.gvt,
            })
        } else {
            Ok(())
        }
    }

    fn validate_logical_event(&self, message: &OptimisticMessage) -> Result<(), OptimisticError> {
        if let Some(existing) = self.known_deliveries.get(&DeliveryIdentity::from(message)) {
            if existing != &message.event {
                return Err(OptimisticError::ConflictingDelivery {
                    source_lp: message.event.source_lp,
                    incarnation: message.incarnation,
                });
            }
        }
        Ok(())
    }

    fn make_message(
        &mut self,
        event: RemoteEvent,
        logical_id: LogicalEventId,
        kind: OptimisticMessageKind,
    ) -> Result<OptimisticMessage, OptimisticError> {
        let incarnation = self.allocate_incarnation(event.source_lp)?;
        Ok(OptimisticMessage::new(event, logical_id, incarnation, kind))
    }

    fn allocate_incarnation(&mut self, source_lp: LpId) -> Result<u64, OptimisticError> {
        let next = self
            .next_incarnation
            .get_mut(&source_lp)
            .ok_or(OptimisticError::UnknownLogicalProcess(source_lp))?;
        let current = (*next).ok_or(OptimisticError::IncarnationExhausted(source_lp))?;
        *next = current.checked_add(1);
        Ok(current)
    }

    fn observe_incarnation(&mut self, source_lp: LpId, incarnation: u64) {
        if let Some(next) = self.next_incarnation.get_mut(&source_lp) {
            if let Some(current) = *next {
                if incarnation >= current {
                    *next = incarnation.checked_add(1);
                }
            }
        }
    }

    fn ensure_pending_capacity(&self, additional: usize) -> Result<(), OptimisticError> {
        if self.total_pending().saturating_add(additional) > self.limits.max_pending_events {
            Err(OptimisticError::PendingLimitExceeded {
                limit: self.limits.max_pending_events,
            })
        } else {
            Ok(())
        }
    }

    fn total_pending(&self) -> usize {
        self.processes
            .values()
            .map(LogicalProcessState::pending_len)
            .sum()
    }

    fn total_positives(&self) -> usize {
        self.processes
            .values()
            .map(|state| state.positives.len())
            .sum()
    }

    fn total_antis(&self) -> usize {
        self.processes.values().map(|state| state.antis.len()).sum()
    }

    fn total_history(&self) -> usize {
        self.processes
            .values()
            .map(|state| state.history.len())
            .sum()
    }

    fn total_tombstones(&self) -> usize {
        self.processes
            .values()
            .map(|state| state.tombstones.len())
            .sum()
    }

    fn total_replay_pending(&self) -> usize {
        self.processes
            .values()
            .map(|state| state.replay_pending.len())
            .sum()
    }

    fn next_epoch(&self, lp_id: LpId) -> Result<u64, OptimisticError> {
        self.processes
            .get(&lp_id)
            .ok_or(OptimisticError::UnknownLogicalProcess(lp_id))?
            .epoch
            .checked_add(1)
            .ok_or(OptimisticError::EpochExhausted(lp_id))
    }

    fn bump_epoch(&mut self, lp_id: LpId) -> Result<(), OptimisticError> {
        let next = self.next_epoch(lp_id)?;
        self.processes
            .get_mut(&lp_id)
            .expect("validated logical process")
            .epoch = next;
        Ok(())
    }

    fn check_tombstone_reservation(
        &self,
        lp_id: LpId,
        identity: &DeliveryIdentity,
    ) -> Result<(), OptimisticError> {
        let state = self
            .processes
            .get(&lp_id)
            .ok_or(OptimisticError::UnknownLogicalProcess(lp_id))?;
        if state.tombstones.contains_key(identity) || state.antis.contains(identity).is_some() {
            return Ok(());
        }
        let reserved = self
            .processes
            .values()
            .map(|candidate| {
                candidate
                    .antis
                    .all()
                    .into_iter()
                    .filter(|anti| {
                        let id = DeliveryIdentity::from(anti);
                        !candidate.tombstones.contains_key(&id)
                    })
                    .count()
            })
            .sum::<usize>();
        if self
            .total_tombstones()
            .saturating_add(reserved)
            .saturating_add(1)
            > self.limits.max_tombstones
        {
            return Err(OptimisticError::TombstoneLimitExceeded {
                limit: self.limits.max_tombstones,
            });
        }
        Ok(())
    }

    fn select_antis(&self, horizon: Tick, budget: usize) -> Vec<OptimisticMessage> {
        self.processes
            .values()
            .filter_map(|state| state.antis.first_through(horizon).cloned())
            .take(budget)
            .collect()
    }

    fn select_positives(&self, horizon: Tick, budget: usize) -> Vec<OptimisticMessage> {
        self.processes
            .values()
            .filter_map(|state| state.positives.first_through(horizon).cloned())
            .take(budget)
            .collect()
    }

    fn has_antis_through(&self, horizon: Tick) -> bool {
        self.processes
            .values()
            .any(|state| state.antis.first_through(horizon).is_some())
    }

    fn has_positives_through(&self, horizon: Tick) -> bool {
        self.processes
            .values()
            .any(|state| state.positives.first_through(horizon).is_some())
    }

    fn remove_pending(&mut self, message: &OptimisticMessage) -> Result<(), OptimisticError> {
        let lp_id = message.event.dest_lp;
        let identity = DeliveryIdentity::from(message);
        let next_epoch = self.next_epoch(lp_id)?;
        let state = self.processes.get_mut(&lp_id).expect("validated LP");
        let removed = match message.kind {
            OptimisticMessageKind::Positive => state.positives.remove(&identity),
            OptimisticMessageKind::Anti => state.antis.remove(&identity),
        };
        if removed.is_some() {
            state.epoch = next_epoch;
        }
        Ok(())
    }

    fn execute_round(
        &mut self,
        selected: Vec<OptimisticMessage>,
    ) -> Result<Vec<StagedEvent<P::Snapshot>>, OptimisticError> {
        if self.total_history().saturating_add(selected.len()) > self.limits.max_history_events {
            return Err(OptimisticError::HistoryLimitExceeded {
                limit: self.limits.max_history_events,
            });
        }
        for message in &selected {
            self.next_epoch(message.event.dest_lp)?;
        }
        let mut staged = Vec::with_capacity(selected.len());
        for message in selected {
            let lp_id = message.event.dest_lp;
            let before = match catch_unwind(AssertUnwindSafe(|| {
                self.processes[&lp_id].process.snapshot()
            })) {
                Ok(snapshot) => snapshot,
                Err(_) => {
                    self.poisoned = true;
                    return Err(OptimisticError::SnapshotPanicked(lp_id));
                }
            };
            self.remove_pending(&message)?;
            let outputs = match catch_unwind(AssertUnwindSafe(|| {
                self.processes
                    .get_mut(&lp_id)
                    .expect("validated LP")
                    .process
                    .on_event(&message.event)
            })) {
                Ok(outputs) => outputs,
                Err(_) => {
                    self.poisoned = true;
                    return Err(OptimisticError::HandlerPanicked(lp_id));
                }
            };
            if outputs.len() > self.limits.max_output_batch {
                self.poisoned = true;
                return Err(OptimisticError::OutputBatchTooLarge {
                    actual: outputs.len(),
                    limit: self.limits.max_output_batch,
                });
            }
            for output in &outputs {
                if output.source_lp != lp_id {
                    self.poisoned = true;
                    return Err(OptimisticError::OutputSourceMismatch {
                        lp_id,
                        declared: output.source_lp,
                    });
                }
                if output.tick <= message.event.tick {
                    self.poisoned = true;
                    return Err(OptimisticError::OutputNotStrictlyFuture {
                        input_tick: message.event.tick,
                        output_tick: output.tick,
                    });
                }
                if let Err(error) = self.validate_event(output) {
                    self.poisoned = true;
                    return Err(error);
                }
            }
            let parent_key = message.order_key();
            let mut output_messages = Vec::with_capacity(outputs.len());
            for (ordinal, event) in outputs.into_iter().enumerate() {
                let ordinal = u32::try_from(ordinal).map_err(|_| {
                    self.poisoned = true;
                    OptimisticError::OutputBatchTooLarge {
                        actual: self.limits.max_output_batch.saturating_add(1),
                        limit: self.limits.max_output_batch,
                    }
                })?;
                let logical_id = match LogicalEventId::child_with_limit(
                    &parent_key,
                    ordinal,
                    self.limits.max_causal_depth,
                ) {
                    Ok(id) => id,
                    Err(error) => {
                        self.poisoned = true;
                        return Err(error);
                    }
                };
                let output_message =
                    match self.make_message(event, logical_id, OptimisticMessageKind::Positive) {
                        Ok(output) => output,
                        Err(error) => {
                            self.poisoned = true;
                            return Err(error);
                        }
                    };
                output_messages.push(output_message);
            }
            staged.push((
                lp_id,
                ExecutedEvent {
                    message,
                    key: parent_key,
                    before,
                    outputs: output_messages,
                },
            ));
        }
        let output_count = staged
            .iter()
            .map(|(_, record)| record.outputs.len())
            .sum::<usize>();
        if let Err(error) = self.ensure_pending_capacity(output_count) {
            self.poisoned = true;
            return Err(error);
        }
        let mut output_destinations = BTreeSet::new();
        for (_, record) in &staged {
            for output in &record.outputs {
                output_destinations.insert(output.event.dest_lp);
            }
        }
        for destination in output_destinations {
            if let Err(error) = self.next_epoch(destination) {
                self.poisoned = true;
                return Err(error);
            }
        }
        self.counters.executions = self.counters.executions.saturating_add(staged.len() as u64);
        for (_, record) in &staged {
            let identity = DeliveryIdentity::from(&record.message);
            if self
                .processes
                .get_mut(&record.message.event.dest_lp)
                .expect("validated LP")
                .replay_pending
                .remove(&identity)
            {
                self.counters.replay_executions = self.counters.replay_executions.saturating_add(1);
            }
        }
        Ok(staged)
    }

    fn process_anti(
        &mut self,
        anti: OptimisticMessage,
        published: &mut Vec<OptimisticMessage>,
    ) -> Result<(), OptimisticError> {
        let identity = DeliveryIdentity::from(&anti);
        let lp_id = anti.event.dest_lp;
        if let Some(existing) = self.processes[&lp_id].tombstones.get(&identity) {
            if existing != &anti.event {
                return Err(OptimisticError::ConflictingDelivery {
                    source_lp: identity.source_lp,
                    incarnation: identity.incarnation,
                });
            }
            return Ok(());
        }
        if let Some(existing) = self.processes[&lp_id].positives.contains(&identity) {
            if existing.event != anti.event {
                return Err(OptimisticError::ConflictingDelivery {
                    source_lp: identity.source_lp,
                    incarnation: identity.incarnation,
                });
            }
            let state = self.processes.get_mut(&lp_id).expect("known LP");
            let _ = state.positives.remove(&identity);
            state.replay_pending.remove(&identity);
            self.insert_tombstone(lp_id, identity, anti.event)?;
            return Ok(());
        }
        if let Some(index) = self.processes[&lp_id]
            .history
            .iter()
            .position(|entry| DeliveryIdentity::from(&entry.message) == identity)
        {
            if self.processes[&lp_id].history[index].message.event != anti.event {
                return Err(OptimisticError::ConflictingDelivery {
                    source_lp: identity.source_lp,
                    incarnation: identity.incarnation,
                });
            }
            self.rollback_suffix(lp_id, index, Some(identity), published)?;
            return Ok(());
        }
        self.insert_tombstone(lp_id, identity, anti.event)
    }

    fn insert_tombstone(
        &mut self,
        lp_id: LpId,
        identity: DeliveryIdentity,
        event: RemoteEvent,
    ) -> Result<(), OptimisticError> {
        if let Some(existing) = self.processes[&lp_id].tombstones.get(&identity) {
            return if existing == &event {
                Ok(())
            } else {
                Err(OptimisticError::ConflictingDelivery {
                    source_lp: identity.source_lp,
                    incarnation: identity.incarnation,
                })
            };
        }
        let reserved = self
            .processes
            .values()
            .map(|state| {
                state
                    .antis
                    .all()
                    .into_iter()
                    .filter(|anti| !state.tombstones.contains_key(&DeliveryIdentity::from(anti)))
                    .count()
            })
            .sum::<usize>();
        if self.total_tombstones().saturating_add(reserved) > self.limits.max_tombstones {
            return Err(OptimisticError::TombstoneLimitExceeded {
                limit: self.limits.max_tombstones,
            });
        }
        let state = self.processes.get_mut(&lp_id).expect("validated LP");
        state.tombstones.insert(identity.clone(), event.clone());
        self.known_deliveries.insert(identity, event);
        Ok(())
    }

    fn rollback_suffix(
        &mut self,
        lp_id: LpId,
        start: usize,
        canceled: Option<DeliveryIdentity>,
        published: &mut Vec<OptimisticMessage>,
    ) -> Result<(), OptimisticError> {
        let suffix = &self.processes[&lp_id].history[start..];
        if suffix.is_empty() {
            return Ok(());
        }
        let before = match catch_unwind(AssertUnwindSafe(|| suffix[0].before.clone())) {
            Ok(snapshot) => snapshot,
            Err(_) => {
                self.poisoned = true;
                return Err(OptimisticError::SnapshotPanicked(lp_id));
            }
        };
        let mut replay_inputs = Vec::new();
        let mut anti_outputs = Vec::new();
        let mut canceled_event = None;
        for entry in suffix {
            let identity = DeliveryIdentity::from(&entry.message);
            if canceled.as_ref() == Some(&identity) {
                canceled_event = Some(entry.message.event.clone());
            } else {
                replay_inputs.push(entry.message.clone());
            }
            anti_outputs.extend(entry.outputs.iter().map(OptimisticMessage::as_anti));
        }
        if let Some(identity) = canceled.as_ref() {
            if canceled_event.is_none() {
                return Err(OptimisticError::ConflictingDelivery {
                    source_lp: identity.source_lp,
                    incarnation: identity.incarnation,
                });
            }
        }
        let mut unique_anti_outputs =
            BTreeMap::<(LpId, DeliveryIdentity), OptimisticMessage>::new();
        for anti in anti_outputs {
            let identity = DeliveryIdentity::from(&anti);
            let destination = anti.event.dest_lp;
            if !self.processes[&destination]
                .tombstones
                .contains_key(&identity)
                && self.processes[&destination]
                    .antis
                    .contains(&identity)
                    .is_none()
            {
                unique_anti_outputs.insert((destination, identity), anti);
            }
        }
        let anti_outputs = unique_anti_outputs.into_values().collect::<Vec<_>>();
        let additions = replay_inputs.len().saturating_add(anti_outputs.len());
        if self.total_pending().saturating_add(additions) > self.limits.max_pending_events {
            return Err(OptimisticError::PendingLimitExceeded {
                limit: self.limits.max_pending_events,
            });
        }
        let mut affected = BTreeSet::from([lp_id]);
        affected.extend(replay_inputs.iter().map(|message| message.event.dest_lp));
        affected.extend(anti_outputs.iter().map(|message| message.event.dest_lp));
        let mut next_epochs = BTreeMap::new();
        for destination in &affected {
            next_epochs.insert(*destination, self.next_epoch(*destination)?);
        }
        let reserved = self
            .processes
            .values()
            .map(|state| {
                state
                    .antis
                    .all()
                    .into_iter()
                    .filter(|message| {
                        let identity = DeliveryIdentity::from(message);
                        !state.tombstones.contains_key(&identity)
                    })
                    .count()
            })
            .sum::<usize>();
        let tombstone_additions =
            anti_outputs.len()
                + usize::from(canceled.as_ref().is_some_and(|identity| {
                    !self.processes[&lp_id].tombstones.contains_key(identity)
                }));
        if self
            .total_tombstones()
            .saturating_add(reserved)
            .saturating_add(tombstone_additions)
            > self.limits.max_tombstones
        {
            return Err(OptimisticError::TombstoneLimitExceeded {
                limit: self.limits.max_tombstones,
            });
        }
        let restore_result = catch_unwind(AssertUnwindSafe(|| {
            self.processes
                .get_mut(&lp_id)
                .expect("known LP")
                .process
                .restore(&before)
        }));
        match restore_result {
            Ok(Ok(())) => {}
            Ok(Err(reason)) => {
                self.poisoned = true;
                return Err(OptimisticError::RestoreFailed { lp_id, reason });
            }
            Err(_) => {
                self.poisoned = true;
                return Err(OptimisticError::RestorePanicked(lp_id));
            }
        }
        let state = self.processes.get_mut(&lp_id).expect("known LP");
        let rolled_back = state.history.split_off(start);
        state.epoch = next_epochs[&lp_id];
        let depth = rolled_back.len();
        self.counters.rolled_back_events = self
            .counters
            .rolled_back_events
            .saturating_add(depth as u64);
        self.counters.max_rollback_depth = self.counters.max_rollback_depth.max(depth);
        for message in replay_inputs {
            let identity = DeliveryIdentity::from(&message);
            let state = self
                .processes
                .get_mut(&message.event.dest_lp)
                .expect("recorded LP remains present");
            state.epoch = next_epochs[&message.event.dest_lp];
            state.replay_pending.insert(identity);
            state.positives.insert(message);
        }
        if let Some(identity) = canceled {
            let event = canceled_event.expect("validated before restore");
            let state = self.processes.get_mut(&lp_id).expect("known LP");
            state.epoch = next_epochs[&lp_id];
            state.tombstones.insert(identity.clone(), event.clone());
            self.known_deliveries.insert(identity, event);
        }
        for anti in anti_outputs {
            let identity = DeliveryIdentity::from(&anti);
            let destination = anti.event.dest_lp;
            let state = self.processes.get_mut(&destination).expect("recorded LP");
            state.epoch = next_epochs[&destination];
            state.antis.insert(anti.clone());
            self.known_deliveries.insert(identity, anti.event.clone());
            self.counters.canceled_sends = self.counters.canceled_sends.saturating_add(1);
            published.push(anti);
        }
        self.counters.rollback_attempts = self.counters.rollback_attempts.saturating_add(1);
        Ok(())
    }

    fn minimum_pending_tick(&self) -> Option<Tick> {
        self.processes
            .values()
            .flat_map(|state| {
                state
                    .positives
                    .first()
                    .into_iter()
                    .chain(state.antis.first())
                    .map(|message| message.event.tick)
            })
            .min()
    }
}

fn validate_limits(limits: OptimisticLimits) -> Result<(), OptimisticError> {
    if limits.max_lps == 0
        || limits.max_pending_events == 0
        || limits.max_history_events == 0
        || limits.max_tombstones == 0
        || limits.max_output_batch == 0
        || limits.max_output_batch > u32::MAX as usize
        || limits.max_causal_depth == 0
        || limits.max_causal_depth > MAX_CAUSAL_DEPTH
    {
        return Err(OptimisticError::InvalidLimits);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use kairo_ecs_types::EntityId;

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct UnitSnapshot(u64);

    struct UnitProcess {
        value: u64,
        outputs: Vec<RemoteEvent>,
    }

    impl OptimisticProcess for UnitProcess {
        type Snapshot = UnitSnapshot;

        fn snapshot(&self) -> Self::Snapshot {
            UnitSnapshot(self.value)
        }

        fn restore(&mut self, snapshot: &Self::Snapshot) -> Result<(), OptimisticStateError> {
            self.value = snapshot.0;
            Ok(())
        }

        fn on_event(&mut self, event: &RemoteEvent) -> Vec<RemoteEvent> {
            self.value += u64::from(event.event_payload.first().copied().unwrap_or_default());
            self.outputs.clone()
        }
    }

    fn runtime_with_outputs(outputs: Vec<RemoteEvent>) -> OptimisticRuntime<UnitProcess> {
        let partition = PartitionPlan::from_entities(
            2,
            SimDuration::from_ticks(1),
            vec![EntityId::new(1, 0), EntityId::new(2, 0)],
        )
        .unwrap();
        let topology = BTreeMap::from([(LpId(0), vec![LpId(1)]), (LpId(1), vec![LpId(0)])]);
        let processes = [LpId(0), LpId(1)]
            .into_iter()
            .map(|lp| {
                let lp_outputs = if lp == LpId(0) {
                    outputs.clone()
                } else {
                    Vec::new()
                };
                (
                    lp,
                    UnitProcess {
                        value: 0,
                        outputs: lp_outputs,
                    },
                )
            })
            .collect();
        OptimisticRuntime::new(partition, topology, processes, OptimisticLimits::default()).unwrap()
    }

    fn root_event(destination: LpId, tick: u128) -> RemoteEvent {
        RemoteEvent {
            source_lp: LpId(0),
            dest_lp: destination,
            tick: Tick::from_ticks(tick),
            event_payload: vec![1],
        }
    }

    fn output_batch(count: usize) -> Vec<RemoteEvent> {
        (0..count)
            .map(|ordinal| RemoteEvent {
                source_lp: LpId(0),
                dest_lp: LpId(1),
                tick: Tick::from_ticks(2),
                event_payload: vec![ordinal as u8],
            })
            .collect()
    }

    #[test]
    fn external_incarnation_and_epoch_exhaustion_reject_without_mutation() {
        let mut local = runtime_with_outputs(Vec::new());
        let before_token = local.state_token(LpId(0)).unwrap();
        let before_report = local.report();
        local.next_incarnation.insert(LpId(0), None);
        let error = local
            .schedule_initial(1, root_event(LpId(0), 1))
            .unwrap_err();
        assert_eq!(error, OptimisticError::IncarnationExhausted(LpId(0)));
        assert!(local.validate_state_token(before_token));
        assert_eq!(local.report(), before_report);
        assert!(local.pending_events(LpId(0)).unwrap().is_empty());

        let mut sender = runtime_with_outputs(Vec::new());
        let message = sender.schedule_initial(2, root_event(LpId(1), 1)).unwrap();
        let mut receiver = runtime_with_outputs(Vec::new());
        receiver.processes.get_mut(&LpId(1)).unwrap().epoch = u64::MAX;
        let token = receiver.state_token(LpId(1)).unwrap();
        let report = receiver.report();
        let next_incarnation = receiver.next_incarnation[&LpId(0)];
        assert_eq!(
            receiver.receive(message),
            Err(OptimisticError::EpochExhausted(LpId(1)))
        );
        assert!(receiver.validate_state_token(token));
        assert_eq!(receiver.report(), report);
        assert_eq!(receiver.next_incarnation[&LpId(0)], next_incarnation);
        assert!(receiver.pending_events(LpId(1)).unwrap().is_empty());
    }

    #[test]
    fn output_batch_reserves_one_epoch_per_destination_and_publishes_together() {
        let mut runtime = runtime_with_outputs(output_batch(2));
        runtime.processes.get_mut(&LpId(1)).unwrap().epoch = u64::MAX - 1;
        runtime.schedule_initial(1, root_event(LpId(0), 1)).unwrap();
        let progress = runtime
            .run_until_with_budget(Tick::from_ticks(2), 1)
            .unwrap();
        assert_eq!(progress.published_messages.len(), 2);
        assert_eq!(runtime.processes[&LpId(1)].epoch, u64::MAX);
        assert_eq!(runtime.report().pending_events, 2);
    }

    #[test]
    fn output_batch_epoch_exhaustion_poisons_without_partial_publication() {
        let mut runtime = runtime_with_outputs(output_batch(2));
        runtime.processes.get_mut(&LpId(1)).unwrap().epoch = u64::MAX;
        runtime.schedule_initial(1, root_event(LpId(0), 1)).unwrap();
        assert_eq!(
            runtime.run_until_with_budget(Tick::from_ticks(2), 1),
            Err(OptimisticError::EpochExhausted(LpId(1)))
        );
        assert_eq!(runtime.report().pending_events, 0);
        assert!(runtime.poisoned);
        assert_eq!(
            runtime.run_until_with_budget(Tick::from_ticks(2), 1),
            Err(OptimisticError::Poisoned)
        );
    }

    #[test]
    fn incarnation_exhaustion_in_middle_of_output_batch_poisons_before_publish() {
        let mut runtime = runtime_with_outputs(output_batch(2));
        runtime.schedule_initial(1, root_event(LpId(0), 1)).unwrap();
        runtime.next_incarnation.insert(LpId(0), Some(u64::MAX));
        assert_eq!(
            runtime.run_until_with_budget(Tick::from_ticks(2), 1),
            Err(OptimisticError::IncarnationExhausted(LpId(0)))
        );
        assert_eq!(runtime.report().pending_events, 0);
        assert!(runtime.poisoned);
    }

    #[test]
    fn rollback_requeues_multiple_inputs_with_one_checked_epoch_reservation() {
        let mut runtime = runtime_with_outputs(Vec::new());
        runtime.schedule_initial(1, root_event(LpId(0), 1)).unwrap();
        runtime.schedule_initial(2, root_event(LpId(0), 2)).unwrap();
        runtime
            .run_until_with_budget(Tick::from_ticks(1), 1)
            .unwrap();
        runtime
            .run_until_with_budget(Tick::from_ticks(2), 1)
            .unwrap();
        runtime.processes.get_mut(&LpId(0)).unwrap().epoch = u64::MAX - 1;
        runtime
            .rollback_suffix(LpId(0), 0, None, &mut Vec::new())
            .unwrap();
        assert_eq!(runtime.processes[&LpId(0)].epoch, u64::MAX);
        assert_eq!(runtime.report().pending_positives, 2);
        assert_eq!(runtime.report().replay_pending, 2);
    }
}
