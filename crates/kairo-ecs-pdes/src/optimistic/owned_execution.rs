use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::panic::{catch_unwind, AssertUnwindSafe};

use super::owned::{with_live_authorities, NativeAccountingAuthority};
use super::owned_routing::{NativeAdmissionCapability, NativeSendId, OptimisticOutboundRecord};
use super::{
    DeliveryIdentity, ExecutedEvent, LogicalEventId, LpId, OptimisticAuthority, OptimisticError,
    OptimisticEventOrderKey, OptimisticFossilReport, OptimisticMessage, OptimisticMessageKind,
    OptimisticProcess, OptimisticRunProgress, OptimisticRuntime, OptimisticTraceEntry, Tick,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeIntentKind {
    Present,
    Absent,
}

/// Runtime-issued identity for one bounded node in a source cohort history.
pub struct NativeTransitionId {
    issuer: NativeAccountingAuthority,
    sequence: u64,
}

impl Clone for NativeTransitionId {
    fn clone(&self) -> Self {
        Self {
            issuer: self.issuer.clone(),
            sequence: self.sequence,
        }
    }
}

impl fmt::Debug for NativeTransitionId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NativeTransitionId")
            .field("runtime_id", &self.runtime_id())
            .field("recovery_generation", &self.recovery_generation())
            .field("sequence", &self.sequence)
            .finish()
    }
}

impl PartialEq for NativeTransitionId {
    fn eq(&self, other: &Self) -> bool {
        self.sequence == other.sequence && self.issuer.same_issuer(&other.issuer)
    }
}

impl Eq for NativeTransitionId {}

impl NativeTransitionId {
    pub fn runtime_id(&self) -> u64 {
        self.issuer.runtime_id()
    }

    pub fn recovery_generation(&self) -> u64 {
        self.issuer.recovery_generation()
    }

    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    pub(super) fn key(&self) -> TransitionKey {
        (self.runtime_id(), self.recovery_generation(), self.sequence)
    }
}

pub(super) type TransitionKey = (u64, u64, u64);

#[derive(Clone, Debug)]
pub struct NativeRetirementRequest {
    transition_id: NativeTransitionId,
    predecessor_positive: OptimisticMessage,
    predecessor_anti: OptimisticMessage,
    sender: NativeAccountingAuthority,
    old_receiver: NativeAccountingAuthority,
}

impl NativeRetirementRequest {
    pub fn transition_id(&self) -> &NativeTransitionId {
        &self.transition_id
    }

    pub fn predecessor_positive(&self) -> &OptimisticMessage {
        &self.predecessor_positive
    }

    pub fn predecessor_anti(&self) -> &OptimisticMessage {
        &self.predecessor_anti
    }

    pub fn sender(&self) -> &NativeAccountingAuthority {
        &self.sender
    }

    pub fn old_receiver(&self) -> &NativeAccountingAuthority {
        &self.old_receiver
    }

    pub(super) fn same_request(&self, other: &Self) -> bool {
        self.transition_id == other.transition_id
            && self.predecessor_positive == other.predecessor_positive
            && self.predecessor_anti == other.predecessor_anti
            && self.sender.same_issuer(&other.sender)
            && self.old_receiver.same_issuer(&other.old_receiver)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeRetirementEffect {
    TombstonedAbsent,
    RemovedPending,
    RolledBackExecuted,
}

#[derive(Clone, Debug)]
pub struct NativeRetirementCapability {
    request: NativeRetirementRequest,
    recorded_revision: u64,
    applied_effect: NativeRetirementEffect,
}

impl NativeRetirementCapability {
    pub fn transition_id(&self) -> &NativeTransitionId {
        self.request.transition_id()
    }

    pub fn predecessor_positive(&self) -> &OptimisticMessage {
        self.request.predecessor_positive()
    }

    pub fn predecessor_anti(&self) -> &OptimisticMessage {
        self.request.predecessor_anti()
    }

    pub fn sender(&self) -> &NativeAccountingAuthority {
        self.request.sender()
    }

    pub fn old_receiver(&self) -> &NativeAccountingAuthority {
        self.request.old_receiver()
    }

    pub fn recorded_revision(&self) -> u64 {
        self.recorded_revision
    }

    pub fn applied_effect(&self) -> NativeRetirementEffect {
        self.applied_effect
    }
}

#[derive(Clone, Debug)]
pub struct NativeIntentView {
    id: NativeTransitionId,
    source_lp: LpId,
    authority: OptimisticAuthority,
    logical_id: LogicalEventId,
    kind: NativeIntentKind,
    message: Option<OptimisticMessage>,
    predecessor: Option<NativeTransitionId>,
    retirement_dependencies: Vec<NativeTransitionId>,
    current: bool,
    retirement_request: Option<NativeRetirementRequest>,
}

impl NativeIntentView {
    pub fn id(&self) -> &NativeTransitionId {
        &self.id
    }
    pub fn source_lp(&self) -> LpId {
        self.source_lp
    }
    pub fn authority(&self) -> OptimisticAuthority {
        self.authority
    }
    pub fn logical_id(&self) -> &LogicalEventId {
        &self.logical_id
    }
    pub fn kind(&self) -> NativeIntentKind {
        self.kind
    }
    pub fn message(&self) -> Option<&OptimisticMessage> {
        self.message.as_ref()
    }
    pub fn predecessor(&self) -> Option<&NativeTransitionId> {
        self.predecessor.as_ref()
    }
    pub fn retirement_dependencies(&self) -> &[NativeTransitionId] {
        &self.retirement_dependencies
    }
    pub fn is_current(&self) -> bool {
        self.current
    }
    pub fn retirement_request(&self) -> Option<&NativeRetirementRequest> {
        self.retirement_request.as_ref()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OptimisticOwnedStepKind {
    PositiveRound,
    Anti,
    Rollback,
}

#[derive(Clone, Debug)]
pub struct OptimisticOwnedStep {
    kind: OptimisticOwnedStepKind,
    selected: Vec<OptimisticMessage>,
    lp_id: Option<LpId>,
    trigger_key: Option<OptimisticEventOrderKey>,
}

impl OptimisticOwnedStep {
    pub fn kind(&self) -> OptimisticOwnedStepKind {
        self.kind
    }
    pub fn selected(&self) -> &[OptimisticMessage] {
        &self.selected
    }
    pub fn lp_id(&self) -> Option<LpId> {
        self.lp_id
    }
    pub fn trigger_key(&self) -> Option<&OptimisticEventOrderKey> {
        self.trigger_key.as_ref()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OptimisticOwnedFailurePhase {
    PreCallbackRejected,
    Compensated,
    PoisonedBeforePublication,
    CommittedCleanupFailed,
}

#[derive(Clone, Debug)]
pub struct OptimisticOwnedRunFailure {
    cause: Box<OptimisticError>,
    trigger_cause: Option<Box<OptimisticError>>,
    progress: Box<OptimisticRunProgress>,
    phase: OptimisticOwnedFailurePhase,
    attempted: Vec<OptimisticOwnedStep>,
    compensated_lps: Vec<LpId>,
    invalidated_lps: Vec<LpId>,
    poisoned: bool,
}

impl OptimisticOwnedRunFailure {
    pub fn cause(&self) -> &OptimisticError {
        &self.cause
    }
    pub fn trigger_cause(&self) -> Option<&OptimisticError> {
        self.trigger_cause.as_deref()
    }
    pub fn progress(&self) -> &OptimisticRunProgress {
        &self.progress
    }
    pub fn phase(&self) -> OptimisticOwnedFailurePhase {
        self.phase
    }
    pub fn attempted(&self) -> &[OptimisticOwnedStep] {
        &self.attempted
    }
    pub fn compensated_lps(&self) -> &[LpId] {
        &self.compensated_lps
    }
    pub fn invalidated_lps(&self) -> &[LpId] {
        &self.invalidated_lps
    }
    pub fn poisoned(&self) -> bool {
        self.poisoned
    }
}

impl fmt::Display for OptimisticOwnedRunFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "owned optimistic runtime failed: {}", self.cause)
    }
}

impl std::error::Error for OptimisticOwnedRunFailure {}

#[derive(Clone, Debug)]
pub struct OptimisticNativeCleanupFailure {
    runtime_id: u64,
    lp_id: Option<LpId>,
    cause: OptimisticError,
}

impl OptimisticNativeCleanupFailure {
    pub fn runtime_id(&self) -> u64 {
        self.runtime_id
    }
    pub fn lp_id(&self) -> Option<LpId> {
        self.lp_id
    }
    pub fn cause(&self) -> &OptimisticError {
        &self.cause
    }
}

#[derive(Clone, Debug)]
pub struct OptimisticNativeCutReport {
    committed_gvt: super::Tick,
    reports: Vec<OptimisticFossilReport>,
    cleanup_failures: Vec<OptimisticNativeCleanupFailure>,
}

impl OptimisticNativeCutReport {
    pub fn committed_gvt(&self) -> super::Tick {
        self.committed_gvt
    }
    pub fn reports(&self) -> &[OptimisticFossilReport] {
        &self.reports
    }
    pub fn cleanup_failures(&self) -> &[OptimisticNativeCleanupFailure] {
        &self.cleanup_failures
    }
}

#[derive(Clone, Debug)]
pub struct OptimisticNativeCutFailure {
    cause: Box<OptimisticError>,
    failed_runtime_id: Option<u64>,
    poisoned_participants: Vec<u64>,
}

impl OptimisticNativeCutFailure {
    pub fn cause(&self) -> &OptimisticError {
        &self.cause
    }
    pub fn failed_runtime_id(&self) -> Option<u64> {
        self.failed_runtime_id
    }
    pub fn poisoned_participants(&self) -> &[u64] {
        &self.poisoned_participants
    }
}

impl fmt::Display for OptimisticNativeCutFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "native optimistic group cut failed: {}",
            self.cause
        )
    }
}

impl std::error::Error for OptimisticNativeCutFailure {}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct IntentCohortKey {
    pub source_lp: LpId,
    pub authority: super::AuthorityStorageKey,
    pub logical_id: LogicalEventId,
}

#[derive(Clone, Debug)]
pub(super) struct NativeIntentRecord {
    pub view_id: NativeTransitionId,
    pub source_lp: LpId,
    pub authority: OptimisticAuthority,
    pub logical_id: LogicalEventId,
    pub kind: NativeIntentKind,
    pub message: Option<OptimisticMessage>,
    pub predecessor: Option<NativeTransitionId>,
    pub request: Option<NativeRetirementRequest>,
    pub current: bool,
    pub applied: bool,
    pub blocked_local: bool,
    pub local_pending_reserved: bool,
}

#[derive(Clone, Debug)]
pub(super) struct ReceiverRetirementRecord {
    pub request: NativeRetirementRequest,
    pub anti_admission: NativeAdmissionCapability,
    pub applied: Option<NativeRetirementCapability>,
    pub positive_readback_reserved: bool,
}

#[derive(Clone, Default)]
pub(super) struct OwnedExecutionState {
    pub next_transition_sequence: Option<u64>,
    pub intents: BTreeMap<TransitionKey, NativeIntentRecord>,
    pub heads: BTreeMap<IntentCohortKey, TransitionKey>,
    pub message_intents: BTreeMap<NativeSendId, TransitionKey>,
    pub receiver_retirements: BTreeMap<TransitionKey, ReceiverRetirementRecord>,
    pub source_applied: BTreeMap<TransitionKey, NativeRetirementCapability>,
    pub reserved_receipts: usize,
    pub reserved_tombstones: usize,
    pub reserved_pending: usize,
}

impl OwnedExecutionState {
    pub(super) fn transition_record_count(&self) -> usize {
        self.intents
            .keys()
            .chain(self.receiver_retirements.keys())
            .copied()
            .collect::<BTreeSet<_>>()
            .len()
    }

    pub(super) fn retained_receipt_count(&self) -> usize {
        self.source_applied
            .keys()
            .copied()
            .chain(
                self.receiver_retirements
                    .iter()
                    .filter(|(_, record)| {
                        record.applied.is_some()
                            || record
                                .request
                                .sender()
                                .same_issuer(record.request.old_receiver())
                    })
                    .map(|(key, _)| *key),
            )
            .collect::<BTreeSet<_>>()
            .len()
    }

    pub(super) fn unresolved_dependencies(
        &self,
        key: &TransitionKey,
    ) -> Result<Vec<NativeTransitionId>, OptimisticError> {
        let record = self
            .intents
            .get(key)
            .ok_or(OptimisticError::UnknownNativeTransition)?;
        let cohort = IntentCohortKey {
            source_lp: record.source_lp,
            authority: super::AuthorityStorageKey::from(record.authority),
            logical_id: record.logical_id.clone(),
        };
        let mut cursor = record.predecessor.as_ref();
        let mut visited = std::collections::BTreeSet::new();
        let mut dependencies = Vec::new();
        while let Some(id) = cursor {
            if visited.len() >= self.intents.len() || !visited.insert(id.key()) {
                return Err(OptimisticError::NativeTransitionCycle);
            }
            let parent_key = id.key();
            let parent = self
                .intents
                .get(&parent_key)
                .ok_or(OptimisticError::NativeTransitionDependencyMissing)?;
            let parent_cohort = IntentCohortKey {
                source_lp: parent.source_lp,
                authority: super::AuthorityStorageKey::from(parent.authority),
                logical_id: parent.logical_id.clone(),
            };
            if parent_cohort != cohort || !parent.view_id.issuer.same_issuer(&record.view_id.issuer)
            {
                return Err(OptimisticError::NativeTransitionDependencyMissing);
            }
            if self
                .intents
                .values()
                .filter(|child| {
                    child
                        .predecessor
                        .as_ref()
                        .is_some_and(|pred| pred.key() == parent_key)
                })
                .count()
                > 1
            {
                return Err(OptimisticError::NativeTransitionFork);
            }
            if parent.kind == NativeIntentKind::Absent && !parent.applied {
                if let Some(request) = &parent.request {
                    dependencies.push(request.transition_id.clone());
                } else {
                    return Err(OptimisticError::NativeTransitionDependencyMissing);
                }
            }
            cursor = parent.predecessor.as_ref();
        }
        dependencies.sort_by_key(NativeTransitionId::key);
        dependencies.dedup_by(|left, right| left == right);
        Ok(dependencies)
    }

    pub(super) fn send_metadata(
        &self,
        id: &NativeSendId,
    ) -> Result<
        (
            NativeTransitionId,
            Vec<NativeTransitionId>,
            Option<NativeRetirementRequest>,
        ),
        OptimisticError,
    > {
        let key = self
            .message_intents
            .get(id)
            .ok_or(OptimisticError::NativeTransitionDependencyMissing)?;
        let record = self
            .intents
            .get(key)
            .ok_or(OptimisticError::NativeTransitionDependencyMissing)?;
        Ok((
            record.view_id.clone(),
            self.unresolved_dependencies(key)?,
            record.request.clone(),
        ))
    }

    pub(super) fn mint_id(
        &mut self,
        issuer: &NativeAccountingAuthority,
    ) -> Result<NativeTransitionId, OptimisticError> {
        let sequence = self.next_transition_sequence.ok_or(
            OptimisticError::NativeTransitionIdentityExhausted {
                runtime_id: issuer.runtime_id(),
            },
        )?;
        self.next_transition_sequence = sequence.checked_add(1);
        Ok(NativeTransitionId {
            issuer: issuer.clone(),
            sequence,
        })
    }
}

pub(super) fn make_request(
    transition_id: NativeTransitionId,
    predecessor_positive: OptimisticMessage,
    predecessor_anti: OptimisticMessage,
    sender: NativeAccountingAuthority,
    old_receiver: NativeAccountingAuthority,
) -> NativeRetirementRequest {
    NativeRetirementRequest {
        transition_id,
        predecessor_positive,
        predecessor_anti,
        sender,
        old_receiver,
    }
}

pub(super) fn _storage_id(message: &OptimisticMessage) -> NativeSendId {
    NativeSendId::from(message)
}

pub(super) fn _outbound(
    message: OptimisticMessage,
    issuer: NativeAccountingAuthority,
) -> OptimisticOutboundRecord {
    OptimisticOutboundRecord { message, issuer }
}

impl<P: OptimisticProcess> OptimisticRuntime<P> {
    pub fn native_intents(&self) -> Result<Vec<NativeIntentView>, OptimisticError> {
        self.ensure_healthy()?;
        let owned = self
            .owned
            .as_ref()
            .ok_or(OptimisticError::OwnedModeRequired)?;
        let peers = owned.peers();
        with_live_authorities(&peers, || {
            if !owned.peers_sealed() {
                return Err(OptimisticError::NativePeersNotSealed);
            }
            owned
                .execution()
                .intents
                .values()
                .map(|record| {
                    Ok(NativeIntentView {
                        id: record.view_id.clone(),
                        source_lp: record.source_lp,
                        authority: record.authority,
                        logical_id: record.logical_id.clone(),
                        kind: record.kind,
                        message: record.message.clone(),
                        predecessor: record.predecessor.clone(),
                        retirement_dependencies: owned
                            .execution()
                            .unresolved_dependencies(&record.view_id.key())?,
                        current: record.current,
                        retirement_request: record.request.clone(),
                    })
                })
                .collect::<Result<Vec<_>, OptimisticError>>()
        })
    }

    pub fn pending_native_retirement_requests(
        &self,
    ) -> Result<Vec<NativeRetirementRequest>, OptimisticError> {
        self.ensure_healthy()?;
        let owned = self
            .owned
            .as_ref()
            .ok_or(OptimisticError::OwnedModeRequired)?;
        let peers = owned.peers();
        with_live_authorities(&peers, || {
            if !owned.peers_sealed() {
                return Err(OptimisticError::NativePeersNotSealed);
            }
            Ok(owned
                .execution()
                .intents
                .values()
                .filter(|record| record.request.is_some() && !record.applied)
                .filter_map(|record| record.request.clone())
                .collect())
        })
    }

    pub fn applied_native_retirements(
        &self,
    ) -> Result<Vec<NativeRetirementCapability>, OptimisticError> {
        self.ensure_healthy()?;
        let owned = self
            .owned
            .as_ref()
            .ok_or(OptimisticError::OwnedModeRequired)?;
        let peers = owned.peers();
        with_live_authorities(&peers, || {
            if !owned.peers_sealed() {
                return Err(OptimisticError::NativePeersNotSealed);
            }
            Ok(owned
                .execution()
                .receiver_retirements
                .values()
                .filter_map(|record| record.applied.clone())
                .collect())
        })
    }

    pub fn receive_native_retirement(
        &mut self,
        request: &NativeRetirementRequest,
    ) -> Result<NativeAdmissionCapability, OptimisticError> {
        self.ensure_healthy()?;
        let (mut peers, peers_sealed) = {
            let owned = self
                .owned
                .as_ref()
                .ok_or(OptimisticError::OwnedModeRequired)?;
            (owned.peers(), owned.peers_sealed())
        };
        peers.push(request.sender().clone());
        peers.push(request.old_receiver().clone());
        with_live_authorities(&peers, || {
            if !peers_sealed {
                return Err(OptimisticError::NativePeersNotSealed);
            }
            self.receive_native_retirement_locked(request)
        })
    }

    fn receive_native_retirement_locked(
        &mut self,
        request: &NativeRetirementRequest,
    ) -> Result<NativeAdmissionCapability, OptimisticError> {
        let positive = request.predecessor_positive();
        let anti = request.predecessor_anti();
        let source_lp = positive.event().source_lp;
        let destination = positive.event().dest_lp;
        let owned = self.owned.as_ref().expect("owned mode checked");
        let own = owned.own_authority();
        let Some(sender) = owned
            .peers()
            .into_iter()
            .find(|peer| peer.same_issuer(request.sender()))
        else {
            return Err(OptimisticError::UnregisteredNativeIssuer {
                runtime_id: request.sender().runtime_id(),
                recovery_generation: request.sender().recovery_generation(),
            });
        };
        if !own.same_issuer(request.old_receiver()) {
            return Err(OptimisticError::NativeSendIssuerMismatch { source_lp });
        }
        if !sender.owned_lps().binary_search(&source_lp).is_ok()
            || !request.transition_id.issuer.same_issuer(&sender)
        {
            return Err(OptimisticError::NativeSendIssuerMismatch { source_lp });
        }
        let expected_authority = sender
            .current_authorities()
            .get(&source_lp)
            .copied()
            .ok_or(OptimisticError::NativeSendIssuerMismatch { source_lp })?;
        if positive.authority() != expected_authority || anti.authority() != expected_authority {
            return Err(OptimisticError::NativeAuthorityMismatch {
                source_lp,
                expected: expected_authority,
                actual: positive.authority(),
            });
        }
        if positive.kind() != OptimisticMessageKind::Positive
            || anti.kind() != OptimisticMessageKind::Anti
            || positive.event() != anti.event()
            || positive.logical_id() != anti.logical_id()
            || positive.incarnation() != anti.incarnation()
            || positive.authority() != anti.authority()
        {
            return Err(OptimisticError::ConflictingNativeReceipt);
        }
        if !owned.is_local_lp(destination) {
            return Err(OptimisticError::UnownedLogicalProcess(destination));
        }
        if !sender
            .global_partition()
            .segments()
            .iter()
            .any(|segment| segment.id == destination)
        {
            return Err(OptimisticError::UnknownLogicalProcess(destination));
        }
        if source_lp != destination
            && !sender
                .global_topology()
                .get(&source_lp)
                .is_some_and(|destinations| destinations.contains(&destination))
        {
            return Err(OptimisticError::RouteMissing {
                source: source_lp,
                destination,
            });
        }
        OptimisticEventOrderKey::try_from_parts(
            positive.event().tick,
            source_lp,
            positive.logical_id().clone(),
        )?;
        self.validate_gvt(positive.event().tick)?;
        self.validate_gvt(anti.event().tick)?;

        let key = request.transition_id.key();
        if own.same_issuer(request.sender()) {
            let receiver = owned
                .execution()
                .receiver_retirements
                .get(&key)
                .ok_or(OptimisticError::UnknownNativeTransition)?;
            if !receiver.request.same_request(request)
                || !request.transition_id.issuer.same_issuer(&own)
            {
                return Err(OptimisticError::ConflictingNativeReceipt);
            }
            return Ok(receiver.anti_admission.clone());
        }
        let anti_id = NativeSendId::from(anti);
        if let Some(existing) = owned.execution().receiver_retirements.get(&key) {
            if !existing.request.same_request(request) {
                return Err(OptimisticError::ConflictingNativeReceipt);
            }
            let retained = self.processes.get(&destination).is_some_and(|state| {
                state
                    .antis
                    .contains(&DeliveryIdentity::from(anti))
                    .is_some()
                    || state.tombstones.contains_key(&DeliveryIdentity::from(anti))
                    || state.history.iter().any(|event| {
                        DeliveryIdentity::from(&event.message) == DeliveryIdentity::from(positive)
                    })
            });
            if !retained {
                return Err(OptimisticError::ConflictingNativeReceipt);
            }
            return Ok(existing.anti_admission.clone());
        }
        let positive_id = NativeSendId::from(positive);
        let positive_retained = owned.routing().admissions.contains_key(&positive_id)
            || self.processes.get(&destination).is_some_and(|state| {
                state
                    .positives
                    .contains(&DeliveryIdentity::from(positive))
                    .is_some()
                    || state.history.iter().any(|event| {
                        DeliveryIdentity::from(&event.message) == DeliveryIdentity::from(positive)
                    })
                    || state
                        .tombstones
                        .contains_key(&DeliveryIdentity::from(positive))
            });
        let receipt_count = owned.routing().outbox.len()
            + owned.routing().completed.len()
            + owned.routing().admissions.len()
            + owned.execution().retained_receipt_count()
            + owned.execution().reserved_receipts;
        let needed_receipts = 2 + usize::from(!positive_retained);
        if owned.execution().transition_record_count() >= owned.max_transition_entries() {
            return Err(OptimisticError::TransitionLimitExceeded {
                limit: owned.max_transition_entries(),
            });
        }
        if receipt_count.saturating_add(needed_receipts) > owned.max_receipt_entries() {
            return Err(OptimisticError::ReceiptLimitExceeded {
                limit: owned.max_receipt_entries(),
            });
        }
        self.ensure_pending_capacity(1)?;
        let identity = DeliveryIdentity::from(anti);
        if !self.processes[&destination]
            .tombstones
            .contains_key(&identity)
            && self
                .total_tombstones()
                .saturating_add(owned.execution().reserved_tombstones)
                .saturating_add(1)
                > self.limits.max_tombstones
        {
            return Err(OptimisticError::TombstoneLimitExceeded {
                limit: self.limits.max_tombstones,
            });
        }
        let next_epoch = self.next_epoch(destination)?;
        let next_revision = owned.next_revision_value()?;
        let cap = NativeAdmissionCapability::new(
            anti_id.clone(),
            anti.clone(),
            own,
            sender,
            next_revision,
            super::NativeAdmissionMembership::Pending,
        );
        {
            let state = self
                .processes
                .get_mut(&destination)
                .expect("owned destination");
            state.epoch = next_epoch;
            state.antis.insert(anti.clone());
        }
        self.known_deliveries.insert(identity, anti.event().clone());
        let execution = self
            .owned
            .as_mut()
            .expect("owned mode checked")
            .execution_mut();
        execution.reserved_tombstones += 1;
        let remote_request = !request.sender().same_issuer(request.old_receiver());
        execution.reserved_receipts += needed_receipts - usize::from(remote_request);
        execution.receiver_retirements.insert(
            key,
            ReceiverRetirementRecord {
                request: request.clone(),
                anti_admission: cap.clone(),
                applied: None,
                positive_readback_reserved: !positive_retained,
            },
        );
        let owned = self.owned.as_mut().expect("owned mode checked");
        if remote_request {
            owned.routing_mut().admissions.insert(anti_id, cap.clone());
        }
        owned.commit_revision(next_revision);
        Ok(cap)
    }

    pub fn acknowledge_native_retirement(
        &mut self,
        cap: NativeRetirementCapability,
    ) -> Result<(), OptimisticError> {
        self.ensure_healthy()?;
        let (mut peers, peers_sealed) = {
            let owned = self
                .owned
                .as_ref()
                .ok_or(OptimisticError::OwnedModeRequired)?;
            (owned.peers(), owned.peers_sealed())
        };
        peers.push(cap.sender().clone());
        peers.push(cap.old_receiver().clone());
        with_live_authorities(&peers, || {
            if !peers_sealed {
                return Err(OptimisticError::NativePeersNotSealed);
            }
            self.acknowledge_native_retirement_locked(cap)
        })
    }

    fn acknowledge_native_retirement_locked(
        &mut self,
        cap: NativeRetirementCapability,
    ) -> Result<(), OptimisticError> {
        let request = &cap.request;
        let positive = request.predecessor_positive();
        let anti = request.predecessor_anti();
        let source_lp = positive.event().source_lp;
        let destination = positive.event().dest_lp;
        let owned = self.owned.as_ref().expect("owned mode checked");
        let own = owned.own_authority();
        if !own.same_issuer(request.sender()) {
            return Err(OptimisticError::NativeSendIssuerMismatch { source_lp });
        }
        let receiver = owned
            .peers()
            .into_iter()
            .find(|peer| peer.same_issuer(request.old_receiver()));
        if !receiver.is_some_and(|peer| peer.owned_lps().binary_search(&destination).is_ok()) {
            return Err(OptimisticError::NativeSendIssuerMismatch { source_lp });
        }
        let expected = own
            .current_authorities()
            .get(&source_lp)
            .copied()
            .ok_or(OptimisticError::NativeSendIssuerMismatch { source_lp })?;
        if positive.authority() != expected || anti.authority() != expected {
            return Err(OptimisticError::NativeAuthorityMismatch {
                source_lp,
                expected,
                actual: positive.authority(),
            });
        }
        if positive.kind() != OptimisticMessageKind::Positive
            || anti.kind() != OptimisticMessageKind::Anti
            || positive.event() != anti.event()
            || positive.logical_id() != anti.logical_id()
            || positive.incarnation() != anti.incarnation()
        {
            return Err(OptimisticError::ConflictingNativeReceipt);
        }
        OptimisticEventOrderKey::try_from_parts(
            positive.event().tick,
            source_lp,
            positive.logical_id().clone(),
        )?;
        self.validate_gvt(positive.event().tick)?;
        self.validate_gvt(anti.event().tick)?;

        let transition_key = request.transition_id.key();
        let existing = owned.execution().source_applied.get(&transition_key);
        if let Some(existing) = existing {
            if !existing.request.same_request(request)
                || existing.recorded_revision != cap.recorded_revision
                || existing.applied_effect != cap.applied_effect
            {
                return Err(OptimisticError::ConflictingNativeReceipt);
            }
            return Ok(());
        }
        let source_record = owned
            .execution()
            .intents
            .get(&transition_key)
            .ok_or(OptimisticError::UnknownNativeTransition)?;
        if source_record.kind != NativeIntentKind::Absent
            || !source_record
                .request
                .as_ref()
                .is_some_and(|recorded| recorded.same_request(request))
        {
            return Err(OptimisticError::ConflictingNativeReceipt);
        }
        if request.sender().same_issuer(request.old_receiver()) {
            let Some(receiver_record) = owned.execution().receiver_retirements.get(&transition_key)
            else {
                return Err(OptimisticError::UnknownNativeTransition);
            };
            if receiver_record.applied.as_ref().is_some_and(|applied| {
                applied.recorded_revision == cap.recorded_revision
                    && applied.applied_effect == cap.applied_effect
                    && applied.request.same_request(request)
            }) {
                return Ok(());
            }
            return Err(OptimisticError::ConflictingNativeReceipt);
        }

        let next_revision = owned.next_revision_value()?;
        let mut promotions = Vec::new();
        for (&key, intent) in &owned.execution().intents {
            if intent.kind == NativeIntentKind::Present
                && intent.current
                && intent.blocked_local
                && owned
                    .execution()
                    .unresolved_dependencies(&key)?
                    .iter()
                    .all(|dependency| {
                        dependency.key() == transition_key
                            || owned
                                .execution()
                                .intents
                                .get(&dependency.key())
                                .is_some_and(|record| record.applied)
                    })
            {
                if let Some(message) = &intent.message {
                    promotions.push((key, message.clone()));
                }
            }
        }
        let mut next_epochs = BTreeMap::new();
        for (_, message) in &promotions {
            next_epochs
                .entry(message.event().dest_lp)
                .or_insert(self.next_epoch(message.event().dest_lp)?);
        }
        if owned.execution().reserved_receipts == 0
            || owned.execution().reserved_pending < promotions.len()
        {
            return Err(OptimisticError::ConflictingNativeReceipt);
        }
        if owned.execution().reserved_pending > self.limits.max_pending_events {
            return Err(OptimisticError::PendingLimitExceeded {
                limit: self.limits.max_pending_events,
            });
        }
        let current = self.owned.as_ref().expect("owned mode checked");
        if current.revision().checked_add(1) != Some(next_revision) {
            return Err(OptimisticError::AccountingRevisionExhausted);
        }
        for lp_id in next_epochs.keys() {
            self.next_epoch(*lp_id)?;
        }

        for message in [positive, anti] {
            let id = NativeSendId::from(message);
            let routing = self
                .owned
                .as_mut()
                .expect("owned mode checked")
                .routing_mut();
            if let Some(record) = routing.outbox.remove(&id) {
                routing.completed.insert(id, record);
            }
        }
        {
            let execution = self
                .owned
                .as_mut()
                .expect("owned mode checked")
                .execution_mut();
            if execution.reserved_receipts == 0 {
                return Err(OptimisticError::ConflictingNativeReceipt);
            }
            execution.reserved_receipts -= 1;
            execution.source_applied.insert(transition_key, cap.clone());
            execution
                .intents
                .get_mut(&transition_key)
                .expect("source cancellation preflighted")
                .applied = true;
        }
        for (key, message) in promotions {
            let lp_id = message.event().dest_lp;
            let process = self
                .processes
                .get_mut(&lp_id)
                .expect("preflighted blocked local destination");
            process.positives.insert(message);
            process.epoch = next_epochs[&lp_id];
            let intent = self
                .owned
                .as_mut()
                .expect("owned mode checked")
                .execution_mut()
                .intents
                .get_mut(&key)
                .expect("promotion intent retained");
            intent.blocked_local = false;
            intent.local_pending_reserved = false;
            self.owned
                .as_mut()
                .expect("owned mode checked")
                .execution_mut()
                .reserved_pending -= 1;
        }
        self.owned
            .as_mut()
            .expect("owned mode checked")
            .commit_revision(next_revision);
        Ok(())
    }
}

impl<P: OptimisticProcess> OptimisticRuntime<P> {
    /// Commits one fossil floor for the complete registered native ownership
    /// group. All actor, coverage, obligation, revision and epoch checks happen
    /// before any participant publishes a floor.
    pub fn fossil_collect_native_group(
        participants: &mut [&mut OptimisticRuntime<P>],
        gvt: Tick,
    ) -> Result<OptimisticNativeCutReport, OptimisticNativeCutFailure> {
        let failure = |cause, failed_runtime_id| OptimisticNativeCutFailure {
            cause: Box::new(cause),
            failed_runtime_id,
            poisoned_participants: Vec::new(),
        };
        if participants.is_empty() {
            return Err(failure(OptimisticError::OwnedModeRequired, None));
        }
        participants.sort_by_key(|runtime| runtime.runtime_id);
        for pair in participants.windows(2) {
            if pair[0].runtime_id == pair[1].runtime_id {
                return Err(failure(
                    OptimisticError::NativeGroupParticipantDuplicate {
                        runtime_id: pair[0].runtime_id,
                    },
                    Some(pair[0].runtime_id),
                ));
            }
        }
        let Some(first_owned) = participants[0].owned.as_ref() else {
            return Err(failure(
                OptimisticError::OwnedModeRequired,
                Some(participants[0].runtime_id),
            ));
        };
        let reference = first_owned.own_authority();
        let expected_lps = reference
            .global_partition()
            .segments()
            .iter()
            .map(|segment| segment.id)
            .collect::<BTreeSet<_>>();
        let mut actual_lps = BTreeSet::new();
        let mut actual_authorities = BTreeMap::new();
        let mut gate_authorities = Vec::new();
        for runtime in participants.iter() {
            let Some(owned) = runtime.owned.as_ref() else {
                return Err(failure(
                    OptimisticError::OwnedModeRequired,
                    Some(runtime.runtime_id),
                ));
            };
            actual_lps.extend(owned.owned_lps().iter().copied());
            actual_authorities.insert(runtime.runtime_id, owned.own_authority());
            gate_authorities.extend(owned.peers());
        }
        let missing = expected_lps
            .difference(&actual_lps)
            .copied()
            .collect::<Vec<_>>();
        let unexpected = actual_lps
            .difference(&expected_lps)
            .copied()
            .collect::<Vec<_>>();
        if !missing.is_empty() || !unexpected.is_empty() {
            return Err(failure(
                OptimisticError::NativeGroupCoverageIncomplete {
                    missing,
                    unexpected,
                },
                None,
            ));
        }
        let mut owner_by_lp = BTreeMap::new();
        for runtime in participants.iter() {
            let owned = runtime
                .owned
                .as_ref()
                .expect("owned participants preflighted");
            for lp_id in owned.owned_lps() {
                owner_by_lp.insert(*lp_id, runtime.runtime_id);
            }
            let own = owned.own_authority_ref();
            if own.simulation_namespace() != reference.simulation_namespace()
                || own.global_partition() != reference.global_partition()
                || own.global_topology() != reference.global_topology()
                || own.current_authorities() != reference.current_authorities()
            {
                return Err(failure(
                    OptimisticError::NativePeerConfigurationMismatch,
                    Some(runtime.runtime_id),
                ));
            }
        }
        for runtime in participants.iter() {
            let owned = runtime
                .owned
                .as_ref()
                .expect("owned participants preflighted");
            for registered in owned.peers() {
                let expected_runtime_id = registered.runtime_id();
                let Some(actual) = actual_authorities.get(&expected_runtime_id) else {
                    let actual_runtime_id = registered
                        .owned_lps()
                        .iter()
                        .find_map(|lp| owner_by_lp.get(lp).copied())
                        .unwrap_or(0);
                    return Err(failure(
                        OptimisticError::NativeGroupIssuerMismatch {
                            participant_runtime_id: runtime.runtime_id,
                            expected_runtime_id,
                            actual_runtime_id,
                        },
                        Some(runtime.runtime_id),
                    ));
                };
                if !registered.same_issuer(actual) {
                    return Err(failure(
                        OptimisticError::NativeGroupIssuerMismatch {
                            participant_runtime_id: runtime.runtime_id,
                            expected_runtime_id,
                            actual_runtime_id: actual.runtime_id(),
                        },
                        Some(runtime.runtime_id),
                    ));
                }
            }
            let registered_ids = owned
                .peers()
                .iter()
                .map(NativeAccountingAuthority::runtime_id)
                .collect::<BTreeSet<_>>();
            if registered_ids != actual_authorities.keys().copied().collect() {
                let actual_runtime_id = actual_authorities
                    .keys()
                    .find(|id| !registered_ids.contains(id))
                    .copied()
                    .unwrap_or(runtime.runtime_id);
                return Err(failure(
                    OptimisticError::NativeGroupIssuerMismatch {
                        participant_runtime_id: runtime.runtime_id,
                        expected_runtime_id: registered_ids
                            .difference(&actual_authorities.keys().copied().collect())
                            .next()
                            .copied()
                            .unwrap_or(0),
                        actual_runtime_id,
                    },
                    Some(runtime.runtime_id),
                ));
            }
        }

        struct Plan {
            epochs: BTreeMap<LpId, u64>,
            next_revision: Option<u64>,
            history_counts: BTreeMap<LpId, usize>,
            tombstones: BTreeMap<LpId, Vec<DeliveryIdentity>>,
            execution: OwnedExecutionState,
            routing: super::owned_routing::OwnedRootRoutingState,
        }
        let mut plans = Vec::with_capacity(participants.len());
        let mut detached = Vec::<(usize, LpId, P::Snapshot)>::new();
        let mut reports = Vec::with_capacity(participants.len());
        let mut failed_runtime_id = None;
        let publication = with_live_authorities(&gate_authorities, || {
            plans.clear();
            // Unresolved source/receiver roles protect the exact shared proof
            // bundle in every actual participant, including lost admission ACKs.
            let mut protected = BTreeSet::new();
            for runtime in participants.iter() {
                let execution = runtime.owned.as_ref().expect("owned group").execution();
                for (key, intent) in &execution.intents {
                    execution.unresolved_dependencies(key)?;
                    if intent.request.is_some() && !intent.applied {
                        protected.insert(*key);
                    }
                }
                for (key, record) in &execution.receiver_retirements {
                    if record.applied.is_none() {
                        protected.insert(*key);
                    }
                }
            }
            for runtime in participants.iter() {
                let owned = runtime
                    .owned
                    .as_ref()
                    .expect("owned participants preflighted");
                failed_runtime_id = Some(runtime.runtime_id);
                if runtime.poisoned {
                    return Err(OptimisticError::Poisoned);
                }
                if !owned.peers_sealed() {
                    return Err(OptimisticError::NativePeersNotSealed);
                }
                if owned.initial_inputs_open() || runtime.initial_open {
                    return Err(OptimisticError::NativeGroupInputsOpen {
                        runtime_id: runtime.runtime_id,
                    });
                }
                if gvt < runtime.gvt {
                    return Err(OptimisticError::GvtRegression {
                        current: runtime.gvt,
                        requested: gvt,
                    });
                }
                let execution = owned.execution();
                let mut ancestor_obligations = Vec::new();
                for (key, intent) in &execution.intents {
                    if intent.kind == NativeIntentKind::Present && intent.current {
                        for dependency in execution.unresolved_dependencies(key)? {
                            let request = execution
                                .intents
                                .get(&dependency.key())
                                .and_then(|record| record.request.as_ref())
                                .ok_or(OptimisticError::NativeTransitionDependencyMissing)?;
                            ancestor_obligations.push(request.predecessor_positive().event().tick);
                        }
                    }
                }
                let obligation = runtime
                    .processes
                    .values()
                    .flat_map(|state| state.positives.all().into_iter().chain(state.antis.all()))
                    .map(|message| message.event().tick)
                    .chain(
                        owned
                            .routing()
                            .outbox
                            .values()
                            .map(|record| record.message.event().tick),
                    )
                    .chain(execution.intents.values().filter_map(|intent| {
                        (intent.current
                            && intent.kind == NativeIntentKind::Present
                            && intent.blocked_local)
                            .then(|| intent.message.as_ref().map(|message| message.event().tick))
                            .flatten()
                    }))
                    .chain(ancestor_obligations)
                    .chain(execution.intents.values().filter_map(|intent| {
                        (intent.request.is_some() && !intent.applied)
                            .then(|| {
                                intent
                                    .request
                                    .as_ref()
                                    .map(|request| request.predecessor_positive().event().tick)
                            })
                            .flatten()
                    }))
                    .chain(
                        execution
                            .receiver_retirements
                            .values()
                            .filter_map(|record| {
                                record
                                    .applied
                                    .is_none()
                                    .then_some(record.request.predecessor_positive().event().tick)
                            }),
                    )
                    .min();
                if let Some(pending) = obligation {
                    if gvt > pending {
                        return Err(OptimisticError::GvtBeyondPending {
                            requested: gvt,
                            pending,
                        });
                    }
                }
                let mut epochs = BTreeMap::new();
                let mut history_counts = BTreeMap::new();
                let mut tombstones = BTreeMap::new();
                let mut changed = gvt != runtime.gvt;
                let (pruned_execution, pruned_routing, proof_changed) = owned
                    .execution()
                    .stage_cut(owned.routing(), gvt, &protected)?;
                changed |= proof_changed;
                for (&lp_id, state) in &runtime.processes {
                    let history_count = state
                        .history
                        .iter()
                        .take_while(|entry| entry.key.tick < gvt)
                        .count();
                    let old_tombstones = state
                        .tombstones
                        .iter()
                        .filter(|(_, event)| event.tick < gvt)
                        .map(|(identity, _)| identity.clone())
                        .collect::<Vec<_>>();
                    if history_count > 0 || !old_tombstones.is_empty() || gvt != runtime.gvt {
                        let epoch = runtime.next_epoch(lp_id)?;
                        epochs.insert(lp_id, epoch);
                        changed = true;
                    }
                    history_counts.insert(lp_id, history_count);
                    tombstones.insert(lp_id, old_tombstones);
                }
                let next_revision = if changed {
                    Some(owned.next_revision_value()?)
                } else {
                    None
                };
                plans.push(Plan {
                    epochs,
                    next_revision,
                    history_counts,
                    tombstones,
                    execution: pruned_execution,
                    routing: pruned_routing,
                });
            }

            for (index, runtime) in participants.iter_mut().enumerate() {
                let runtime = &mut **runtime;
                let plan = &plans[index];
                let previous_gvt = runtime.gvt;
                let mut collected = Vec::new();
                let mut collected_checkpoints = 0;
                for (&lp_id, &count) in &plan.history_counts {
                    let state = runtime.processes.get_mut(&lp_id).expect("known LP");
                    if let Some(epoch) = plan.epochs.get(&lp_id) {
                        state.epoch = *epoch;
                    }
                    let removed = state.history.drain(..count).collect::<Vec<_>>();
                    collected_checkpoints += removed.len();
                    for entry in removed {
                        state.fossil_time = state.fossil_time.max(entry.key.tick);
                        collected.push(OptimisticTraceEntry {
                            lp_id,
                            event: entry.message.event().clone(),
                            logical_id: entry.message.logical_id().clone(),
                        });
                        detached.push((index, lp_id, entry.before));
                    }
                }
                collected.sort_by_key(|entry| {
                    OptimisticEventOrderKey::new(&entry.event, entry.logical_id.clone())
                });
                let mut collected_tombstones = 0;
                for (lp_id, identities) in &plan.tombstones {
                    let state = runtime.processes.get_mut(lp_id).expect("known LP");
                    for identity in identities {
                        collected_tombstones +=
                            usize::from(state.tombstones.remove(identity).is_some());
                    }
                }
                runtime
                    .known_deliveries
                    .retain(|_, event| event.tick >= gvt);
                runtime.gvt = gvt;
                runtime.counters.fossil_collected_events = runtime
                    .counters
                    .fossil_collected_events
                    .saturating_add(collected.len() as u64);
                let retained_history = runtime.total_history();
                let oldest_retained_tick = runtime
                    .processes
                    .values()
                    .flat_map(|state| state.history.iter().map(|entry| entry.key.tick))
                    .min();
                if let Some(owned) = runtime.owned.as_mut() {
                    *owned.execution_mut() = plans[index].execution.clone();
                    *owned.routing_mut() = plans[index].routing.clone();
                    if let Some(next_revision) = plan.next_revision {
                        owned.commit_revision(next_revision);
                    }
                }
                reports.push(OptimisticFossilReport {
                    previous_gvt,
                    new_gvt: gvt,
                    collected,
                    collected_checkpoints,
                    retained_history,
                    oldest_retained_tick,
                    collected_tombstones,
                });
            }
            Ok(())
        });
        if let Err(cause) = publication {
            return Err(failure(cause, failed_runtime_id));
        }

        let mut cleanup_failures = Vec::new();
        for (index, lp_id, snapshot) in detached {
            if catch_unwind(AssertUnwindSafe(|| drop(snapshot))).is_err() {
                let runtime = &mut *participants[index];
                runtime.poisoned = true;
                cleanup_failures.push(OptimisticNativeCleanupFailure {
                    runtime_id: runtime.runtime_id,
                    lp_id: Some(lp_id),
                    cause: OptimisticError::SnapshotDropPanicked(lp_id),
                });
            }
        }
        Ok(OptimisticNativeCutReport {
            committed_gvt: gvt,
            reports,
            cleanup_failures,
        })
    }
}

struct StagedOwnedOutput {
    message: OptimisticMessage,
    id: NativeTransitionId,
    cohort: IntentCohortKey,
    predecessor: Option<NativeTransitionId>,
    dependencies: Vec<NativeTransitionId>,
}

impl<P: OptimisticProcess> OptimisticRuntime<P> {
    fn owned_progress(
        &self,
        horizon: Tick,
        budget: usize,
        used: usize,
        published: &[OptimisticMessage],
    ) -> OptimisticRunProgress {
        let blocked_local = self
            .owned
            .as_ref()
            .map(|owned| {
                owned
                    .execution()
                    .intents
                    .values()
                    .filter(|intent| intent.current && intent.blocked_local)
                    .count()
            })
            .unwrap_or(0);
        let (outbound_positives, outbound_antis) = self
            .owned
            .as_ref()
            .map(|owned| {
                owned
                    .routing()
                    .outbox
                    .values()
                    .fold((0, 0), |(positives, antis), record| {
                        match record.message.kind() {
                            OptimisticMessageKind::Positive => (positives + 1, antis),
                            OptimisticMessageKind::Anti => (positives, antis + 1),
                        }
                    })
            })
            .unwrap_or_default();
        let has_runnable = self.has_antis_through(horizon) || self.has_positives_through(horizon);
        OptimisticRunProgress {
            budget_used: used,
            budget_remaining: budget.saturating_sub(used),
            budget_exhausted: used == budget && has_runnable,
            pending_positives: self.total_positives() + blocked_local + outbound_positives,
            pending_antis: self.total_antis() + outbound_antis,
            replay_pending: self.total_replay_pending(),
            published_messages: published.to_vec(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn owned_failure(
        &self,
        cause: OptimisticError,
        horizon: Tick,
        budget: usize,
        used: usize,
        published: &[OptimisticMessage],
        phase: OptimisticOwnedFailurePhase,
        attempted: Vec<OptimisticOwnedStep>,
        compensated_lps: Vec<LpId>,
        poisoned: bool,
    ) -> OptimisticOwnedRunFailure {
        let poisoned = poisoned || self.poisoned;
        let invalidated_lps = if poisoned {
            self.processes.keys().copied().collect()
        } else {
            compensated_lps.clone()
        };
        OptimisticOwnedRunFailure {
            cause: Box::new(cause),
            trigger_cause: None,
            progress: Box::new(self.owned_progress(horizon, budget, used, published)),
            phase,
            attempted,
            compensated_lps,
            invalidated_lps,
            poisoned,
        }
    }

    pub fn run_owned_until_with_budget(
        &mut self,
        horizon: Tick,
        budget: usize,
    ) -> Result<OptimisticRunProgress, OptimisticOwnedRunFailure> {
        let mut used = 0usize;
        let mut published = Vec::new();
        if let Err(error) = self.ensure_healthy() {
            return Err(self.owned_failure(
                error,
                horizon,
                budget,
                used,
                &published,
                OptimisticOwnedFailurePhase::PreCallbackRejected,
                Vec::new(),
                Vec::new(),
                false,
            ));
        }
        if let Some(previous) = self.last_horizon {
            if horizon < previous {
                return Err(self.owned_failure(
                    OptimisticError::HorizonRegression {
                        previous,
                        requested: horizon,
                    },
                    horizon,
                    budget,
                    used,
                    &published,
                    OptimisticOwnedFailurePhase::PreCallbackRejected,
                    Vec::new(),
                    Vec::new(),
                    false,
                ));
            }
        }
        let Some(owned) = self.owned.as_ref() else {
            return Err(self.owned_failure(
                OptimisticError::OwnedModeRequired,
                horizon,
                budget,
                used,
                &published,
                OptimisticOwnedFailurePhase::PreCallbackRejected,
                Vec::new(),
                Vec::new(),
                false,
            ));
        };
        if let Err(error) = owned.require_sealed_live() {
            return Err(self.owned_failure(
                error,
                horizon,
                budget,
                used,
                &published,
                OptimisticOwnedFailurePhase::PreCallbackRejected,
                Vec::new(),
                Vec::new(),
                false,
            ));
        }
        if budget == 0 {
            return Ok(self.owned_progress(horizon, budget, used, &published));
        }

        while used < budget {
            let owned = self.owned.as_ref().expect("owned mode checked");
            let peers = owned.peers();
            if let Err(error) = with_live_authorities(&peers, || {
                if !owned.peers_sealed() {
                    return Err(OptimisticError::NativePeersNotSealed);
                }
                Ok(())
            }) {
                return Err(self.owned_failure(
                    error,
                    horizon,
                    budget,
                    used,
                    &published,
                    OptimisticOwnedFailurePhase::PreCallbackRejected,
                    Vec::new(),
                    Vec::new(),
                    false,
                ));
            }

            let anti = self.select_antis(horizon, 1).into_iter().next();
            if let Some(message) = anti {
                let step = OptimisticOwnedStep {
                    kind: OptimisticOwnedStepKind::Anti,
                    lp_id: Some(message.event().dest_lp),
                    trigger_key: Some(message.order_key()),
                    selected: vec![message],
                };
                let anti = &step.selected[0];
                let destination = anti.event().dest_lp;
                let identity = DeliveryIdentity::from(anti);
                let start = self.processes[&destination]
                    .history
                    .iter()
                    .position(|entry| DeliveryIdentity::from(&entry.message) == identity);
                match self.execute_owned_control(destination, start, Some(anti), horizon, &step) {
                    Ok(messages) => {
                        used += 1;
                        published.extend(messages);
                    }
                    Err(failure) => {
                        let (error, phase, compensated, poisoned, trigger, committed) = *failure;
                        published.extend(committed);
                        if phase != OptimisticOwnedFailurePhase::PreCallbackRejected {
                            used += 1;
                        }
                        self.poisoned |= poisoned;
                        let mut failure = self.owned_failure(
                            error,
                            horizon,
                            budget,
                            used,
                            &published,
                            phase,
                            vec![step],
                            compensated,
                            poisoned,
                        );
                        failure.trigger_cause = trigger.map(Box::new);
                        return Err(failure);
                    }
                }
                continue;
            }

            let selected = self.select_positives(horizon, budget - used);
            if selected.is_empty() {
                break;
            }
            let mut step = OptimisticOwnedStep {
                kind: OptimisticOwnedStepKind::PositiveRound,
                selected: selected.clone(),
                lp_id: None,
                trigger_key: None,
            };
            let straggler = selected.iter().find_map(|message| {
                let lp_id = message.event().dest_lp;
                let key = message.order_key();
                self.processes.get(&lp_id).and_then(|state| {
                    state
                        .history
                        .iter()
                        .find(|entry| entry.key > key)
                        .map(|_| (lp_id, message.clone()))
                })
            });
            if let Some((lp_id, message)) = straggler {
                step.kind = OptimisticOwnedStepKind::Rollback;
                step.selected = vec![message.clone()];
                step.lp_id = Some(lp_id);
                step.trigger_key = Some(message.order_key());
                let start = self.processes[&lp_id]
                    .history
                    .iter()
                    .position(|entry| entry.key > message.order_key())
                    .expect("straggler suffix preflighted");
                match self.execute_owned_control(lp_id, Some(start), None, horizon, &step) {
                    Ok(messages) => {
                        used += 1;
                        published.extend(messages);
                    }
                    Err(failure) => {
                        let (error, phase, compensated, poisoned, trigger_cause, committed) =
                            *failure;
                        published.extend(committed);
                        if phase != OptimisticOwnedFailurePhase::PreCallbackRejected {
                            used += 1;
                        }
                        if poisoned {
                            self.poisoned = true;
                        }
                        let mut failure = self.owned_failure(
                            error,
                            horizon,
                            budget,
                            used,
                            &published,
                            phase,
                            vec![step],
                            compensated,
                            poisoned,
                        );
                        failure.trigger_cause = trigger_cause.map(Box::new);
                        return Err(failure);
                    }
                }
                continue;
            }

            let mut preflight_error = None;
            let owned = self.owned.as_ref().expect("owned mode checked");
            if let Err(error) = owned.next_revision_value() {
                preflight_error = Some(error);
            }
            for message in &selected {
                if self.next_epoch(message.event().dest_lp).is_err() {
                    preflight_error =
                        Some(OptimisticError::EpochExhausted(message.event().dest_lp));
                    break;
                }
                let Some(expected) = owned
                    .own_authority_ref()
                    .current_authorities()
                    .get(&message.event().source_lp)
                else {
                    preflight_error = Some(OptimisticError::NativeSendIssuerMismatch {
                        source_lp: message.event().source_lp,
                    });
                    break;
                };
                if *expected != message.authority() {
                    preflight_error = Some(OptimisticError::NativeSendIssuerMismatch {
                        source_lp: message.event().source_lp,
                    });
                    break;
                }
                if let Err(error) = self.validate_gvt(message.event().tick) {
                    preflight_error = Some(error);
                    break;
                }
            }
            if let Some(error) = preflight_error {
                return Err(self.owned_failure(
                    error,
                    horizon,
                    budget,
                    used,
                    &published,
                    OptimisticOwnedFailurePhase::PreCallbackRejected,
                    vec![step],
                    Vec::new(),
                    false,
                ));
            }
            let result = self.execute_owned_positive_round(selected, step.clone(), horizon);
            match result {
                Ok(messages) => {
                    used += step.selected.len();
                    published.extend(messages);
                    self.last_horizon = Some(horizon);
                }
                Err(failure) => {
                    let (error, phase, compensated_lps, poisoned, trigger_cause) = *failure;
                    if used < budget && phase != OptimisticOwnedFailurePhase::PreCallbackRejected {
                        used += step.selected.len();
                    }
                    if poisoned {
                        self.poisoned = true;
                    }
                    let mut failure = self.owned_failure(
                        error,
                        horizon,
                        budget,
                        used,
                        &published,
                        phase,
                        vec![step],
                        compensated_lps,
                        poisoned,
                    );
                    failure.trigger_cause = trigger_cause.map(Box::new);
                    return Err(failure);
                }
            }
        }
        Ok(self.owned_progress(horizon, budget, used, &published))
    }

    fn execute_owned_positive_round(
        &mut self,
        selected: Vec<OptimisticMessage>,
        step: OptimisticOwnedStep,
        horizon: Tick,
    ) -> Result<Vec<OptimisticMessage>, Box<OwnedRoundFailure>> {
        let owned = self.owned.as_ref().expect("owned mode checked");
        let peers = owned.peers();
        let next_revision = owned.next_revision_value().map_err(|error| {
            Box::new((
                error,
                OptimisticOwnedFailurePhase::PreCallbackRejected,
                Vec::new(),
                false,
                None,
            ))
        })?;
        // Reserve validity headroom for every LP in the atomic selected unit
        // before taking snapshots or invoking any model callback.
        let mut next_epochs = BTreeMap::new();
        for message in &selected {
            let lp_id = message.event().dest_lp;
            match self.next_epoch(lp_id) {
                Ok(epoch) => {
                    next_epochs.insert(lp_id, epoch);
                }
                Err(error) => {
                    return Err(Box::new((
                        error,
                        OptimisticOwnedFailurePhase::PreCallbackRejected,
                        Vec::new(),
                        false,
                        None,
                    )));
                }
            }
        }
        if let Err(error) = with_live_authorities(&peers, || {
            if !owned.peers_sealed() {
                return Err(OptimisticError::NativePeersNotSealed);
            }
            Ok(())
        }) {
            return Err(Box::new((
                error,
                OptimisticOwnedFailurePhase::PreCallbackRejected,
                Vec::new(),
                false,
                None,
            )));
        }

        let mut before = Vec::with_capacity(selected.len());
        for message in &selected {
            let lp_id = message.event().dest_lp;
            let snapshot = catch_unwind(AssertUnwindSafe(|| {
                self.processes
                    .get(&lp_id)
                    .expect("selected destination exists")
                    .process
                    .snapshot()
            }));
            match snapshot {
                Ok(snapshot) => before.push((lp_id, snapshot)),
                Err(_) => {
                    let captured = before.iter().map(|(id, _)| *id).collect::<Vec<_>>();
                    let result = self.compensate_owned_round(
                        OptimisticError::SnapshotPanicked(lp_id),
                        before,
                        &captured,
                        &step,
                    );
                    self.poisoned = true;
                    return Err(Box::new((
                        result.0,
                        OptimisticOwnedFailurePhase::PoisonedBeforePublication,
                        result.2,
                        true,
                        result.4,
                    )));
                }
            }
        }

        let mut changed = Vec::new();
        let mut stage_outputs = Vec::new();
        let mut incarnation_shadow = self.next_incarnation.clone();
        let mut sequence = owned.execution().next_transition_sequence;
        let issuer = owned.own_authority();
        let mut callback_error = None;
        for (index, message) in selected.iter().enumerate() {
            let lp_id = message.event().dest_lp;
            changed.push(lp_id);
            let outputs = match catch_unwind(AssertUnwindSafe(|| {
                self.processes
                    .get_mut(&lp_id)
                    .expect("selected destination exists")
                    .process
                    .on_event(message.event())
            })) {
                Ok(outputs) => outputs,
                Err(_) => {
                    callback_error = Some(OptimisticError::HandlerPanicked(lp_id));
                    break;
                }
            };
            if outputs.len() > self.limits.max_output_batch {
                callback_error = Some(OptimisticError::OutputBatchTooLarge {
                    actual: outputs.len(),
                    limit: self.limits.max_output_batch,
                });
                break;
            }
            let authority_map = issuer.current_authorities();
            let topology = issuer.global_topology();
            let partition = issuer.global_partition();
            for (ordinal, event) in outputs.into_iter().enumerate() {
                if event.source_lp != lp_id {
                    callback_error = Some(OptimisticError::OutputSourceMismatch {
                        lp_id,
                        declared: event.source_lp,
                    });
                    break;
                }
                if event.tick <= message.event().tick {
                    callback_error = Some(OptimisticError::OutputNotStrictlyFuture {
                        input_tick: message.event().tick,
                        output_tick: event.tick,
                    });
                    break;
                }
                if !partition
                    .segments()
                    .iter()
                    .any(|segment| segment.id == event.dest_lp)
                {
                    callback_error = Some(OptimisticError::UnknownLogicalProcess(event.dest_lp));
                    break;
                }
                if event.source_lp != event.dest_lp
                    && !topology
                        .get(&event.source_lp)
                        .is_some_and(|destinations| destinations.contains(&event.dest_lp))
                {
                    callback_error = Some(OptimisticError::RouteMissing {
                        source: event.source_lp,
                        destination: event.dest_lp,
                    });
                    break;
                }
                let parent_key =
                    OptimisticEventOrderKey::new(message.event(), message.logical_id().clone());
                let ordinal = match u32::try_from(ordinal) {
                    Ok(ordinal) => ordinal,
                    Err(_) => {
                        callback_error = Some(OptimisticError::OutputBatchTooLarge {
                            actual: ordinal.saturating_add(1),
                            limit: u32::MAX as usize,
                        });
                        break;
                    }
                };
                let logical_id = match LogicalEventId::child_with_limit(
                    &parent_key,
                    ordinal,
                    self.limits.max_causal_depth,
                ) {
                    Ok(id) => id,
                    Err(error) => {
                        callback_error = Some(error);
                        break;
                    }
                };
                let authority = match authority_map.get(&lp_id).copied() {
                    Some(authority) => authority,
                    None => {
                        callback_error =
                            Some(OptimisticError::NativeSendIssuerMismatch { source_lp: lp_id });
                        break;
                    }
                };
                let incarnation = incarnation_shadow.get(&lp_id).copied().flatten();
                let Some(incarnation) = incarnation else {
                    callback_error = Some(OptimisticError::IncarnationExhausted(lp_id));
                    break;
                };
                incarnation_shadow.insert(lp_id, incarnation.checked_add(1));
                let output = OptimisticMessage::new_with_authority(
                    event.clone(),
                    logical_id.clone(),
                    authority,
                    incarnation,
                    OptimisticMessageKind::Positive,
                );
                if let Err(error) =
                    OptimisticEventOrderKey::try_from_parts(event.tick, event.source_lp, logical_id)
                {
                    callback_error = Some(error);
                    break;
                }
                stage_outputs.push((index, output));
            }
            if callback_error.is_some() {
                break;
            }
        }
        if let Some(error) = callback_error {
            return Err(Box::new(
                self.compensate_owned_round(error, before, &changed, &step),
            ));
        }

        let mut staged_nodes = Vec::with_capacity(stage_outputs.len());
        for (_, message) in &stage_outputs {
            let key = IntentCohortKey {
                source_lp: message.event().source_lp,
                authority: super::AuthorityStorageKey::from(message.authority()),
                logical_id: message.logical_id().clone(),
            };
            let metadata = (|| {
                let execution = self.owned.as_ref().expect("owned mode checked").execution();
                let predecessor = execution
                    .heads
                    .get(&key)
                    .and_then(|head| execution.intents.get(head))
                    .map(|head| head.view_id.clone());
                let current_present = predecessor.as_ref().is_some_and(|_| {
                    execution
                        .heads
                        .get(&key)
                        .and_then(|head| execution.intents.get(head))
                        .is_some_and(|head| head.kind == NativeIntentKind::Present && head.current)
                });
                let mut dependencies = if let Some(parent_id) = &predecessor {
                    let parent = execution
                        .intents
                        .get(&parent_id.key())
                        .ok_or(OptimisticError::NativeTransitionDependencyMissing)?;
                    let mut dependencies = execution.unresolved_dependencies(&parent_id.key())?;
                    if parent.kind == NativeIntentKind::Absent && !parent.applied {
                        dependencies.push(parent.view_id.clone());
                    }
                    dependencies.sort_by_key(NativeTransitionId::key);
                    dependencies.dedup_by(|left, right| left == right);
                    dependencies
                } else {
                    Vec::new()
                };
                dependencies.shrink_to_fit();
                Ok((predecessor, dependencies, current_present))
            })();
            let (predecessor, dependencies, current_present) = match metadata {
                Ok(metadata) => metadata,
                Err(error) => {
                    return Err(Box::new(
                        self.compensate_owned_round(error, before, &changed, &step),
                    ))
                }
            };
            if current_present {
                return Err(Box::new(self.compensate_owned_round(
                    OptimisticError::ConflictingLogicalEvent {
                        source_lp: message.event().source_lp,
                    },
                    before,
                    &changed,
                    &step,
                )));
            }
            let Some(id_sequence) = sequence else {
                return Err(Box::new(self.compensate_owned_round(
                    OptimisticError::NativeTransitionIdentityExhausted {
                        runtime_id: self.runtime_id,
                    },
                    before,
                    &changed,
                    &step,
                )));
            };
            let id = NativeTransitionId {
                issuer: issuer.clone(),
                sequence: id_sequence,
            };
            sequence = id_sequence.checked_add(1);
            staged_nodes.push(StagedOwnedOutput {
                message: message.clone(),
                id,
                cohort: key,
                predecessor,
                dependencies,
            });
        }

        let mut seen = BTreeSet::new();
        for (_, message) in &stage_outputs {
            let identity = DeliveryIdentity::from(message);
            if self.known_deliveries.contains_key(&identity) || !seen.insert(identity) {
                return Err(Box::new(self.compensate_owned_round(
                    OptimisticError::ConflictingLogicalEvent {
                        source_lp: message.event().source_lp,
                    },
                    before,
                    &changed,
                    &step,
                )));
            }
        }

        let owned = self.owned.as_ref().expect("owned mode checked");
        let execution = owned.execution();
        let remote_outputs = stage_outputs
            .iter()
            .filter(|(_, message)| !owned.is_local_lp(message.event().dest_lp))
            .count();
        let local_outputs = stage_outputs.len() - remote_outputs;
        let transition_count = execution.transition_record_count();
        let receipt_count = owned.routing().outbox.len()
            + owned.routing().completed.len()
            + owned.routing().admissions.len()
            + execution.retained_receipt_count()
            + execution.reserved_receipts;
        let actual_pending = self.total_pending() + execution.reserved_pending;
        let resulting_pending = actual_pending
            .saturating_sub(selected.len())
            .saturating_add(local_outputs);
        let post_error = if transition_count.saturating_add(staged_nodes.len())
            > owned.max_transition_entries()
        {
            Some(OptimisticError::TransitionLimitExceeded {
                limit: owned.max_transition_entries(),
            })
        } else if owned.routing().outbox.len().saturating_add(remote_outputs)
            > owned.max_outbox_entries()
        {
            Some(OptimisticError::OutboxLimitExceeded {
                limit: owned.max_outbox_entries(),
            })
        } else if receipt_count.saturating_add(remote_outputs) > owned.max_receipt_entries() {
            Some(OptimisticError::ReceiptLimitExceeded {
                limit: owned.max_receipt_entries(),
            })
        } else if resulting_pending > self.limits.max_pending_events {
            Some(OptimisticError::PendingLimitExceeded {
                limit: self.limits.max_pending_events,
            })
        } else if self.total_history().saturating_add(selected.len())
            > self.limits.max_history_events
        {
            Some(OptimisticError::HistoryLimitExceeded {
                limit: self.limits.max_history_events,
            })
        } else {
            None
        };
        if let Some(error) = post_error {
            return Err(Box::new(
                self.compensate_owned_round(error, before, &changed, &step),
            ));
        }

        let mut affected = changed.iter().copied().collect::<BTreeSet<_>>();
        affected.extend(stage_outputs.iter().filter_map(|(_, message)| {
            owned
                .is_local_lp(message.event().dest_lp)
                .then_some(message.event().dest_lp)
        }));
        for lp_id in affected {
            if let std::collections::btree_map::Entry::Vacant(entry) = next_epochs.entry(lp_id) {
                match self.next_epoch(lp_id) {
                    Ok(epoch) => {
                        entry.insert(epoch);
                    }
                    Err(error) => {
                        return Err(Box::new(
                            self.compensate_owned_round(error, before, &changed, &step),
                        ));
                    }
                }
            }
        }

        let peers = self.owned.as_ref().expect("owned mode checked").peers();
        let mut committed_messages = None;
        let publish_result = with_live_authorities(&peers, || {
            let current = self.owned.as_ref().expect("owned mode checked");
            if !current.peers_sealed() {
                return Err(OptimisticError::NativePeersNotSealed);
            }
            if current.revision().checked_add(1) != Some(next_revision) {
                return Err(OptimisticError::AccountingRevisionExhausted);
            }
            for &lp_id in next_epochs.keys() {
                self.next_epoch(lp_id)?;
            }
            let before = std::mem::take(&mut before);
            let stage_outputs = std::mem::take(&mut stage_outputs);
            let staged_nodes = std::mem::take(&mut staged_nodes);
            let mut staged_by_lp = BTreeMap::new();
            for (index, message) in stage_outputs {
                staged_by_lp
                    .entry(index)
                    .or_insert_with(Vec::new)
                    .push(message);
            }
            let mut messages = Vec::new();
            let local_issuer = self
                .owned
                .as_ref()
                .expect("owned mode checked")
                .own_authority();
            for ((lp_id, snapshot), message) in before.into_iter().zip(selected.iter()) {
                let index = selected
                    .iter()
                    .position(|item| item.event().dest_lp == lp_id)
                    .expect("one selected message per LP");
                let outputs = staged_by_lp.remove(&index).unwrap_or_default();
                messages.extend(outputs.iter().cloned());
                self.processes
                    .get_mut(&lp_id)
                    .expect("selected LP remains present")
                    .history
                    .push(ExecutedEvent {
                        message: message.clone(),
                        key: message.order_key(),
                        before: snapshot,
                        outputs: outputs.clone(),
                    });
            }
            for message in &selected {
                let process = self
                    .processes
                    .get_mut(&message.event().dest_lp)
                    .expect("selected LP exists");
                let identity = DeliveryIdentity::from(message);
                process.positives.remove(&identity);
                if process.replay_pending.remove(&identity) {
                    self.counters.replay_executions =
                        self.counters.replay_executions.saturating_add(1);
                }
            }
            for (&lp_id, &epoch) in &next_epochs {
                self.processes
                    .get_mut(&lp_id)
                    .expect("preflighted LP")
                    .epoch = epoch;
            }
            {
                let owned = self.owned.as_mut().expect("owned mode checked");
                owned.close_initial_inputs_at_publication();
                owned.commit_revision(next_revision);
            }
            self.initial_open = false;
            self.last_horizon = Some(horizon);
            self.next_incarnation = std::mem::take(&mut incarnation_shadow);
            for message in &messages {
                let identity = DeliveryIdentity::from(message);
                self.known_deliveries
                    .insert(identity, message.event().clone());
            }
            for node in staged_nodes {
                let key = node.id.key();
                let record = NativeIntentRecord {
                    view_id: node.id.clone(),
                    source_lp: node.message.event().source_lp,
                    authority: node.message.authority(),
                    logical_id: node.message.logical_id().clone(),
                    kind: NativeIntentKind::Present,
                    message: Some(node.message.clone()),
                    predecessor: node.predecessor,
                    request: None,
                    current: true,
                    applied: false,
                    blocked_local: false,
                    local_pending_reserved: false,
                };
                if let Some(parent) = &record.predecessor {
                    if let Some(parent_record) = self
                        .owned
                        .as_mut()
                        .expect("owned mode checked")
                        .execution_mut()
                        .intents
                        .get_mut(&parent.key())
                    {
                        parent_record.current = false;
                    }
                }
                let local = self
                    .owned
                    .as_ref()
                    .expect("owned mode checked")
                    .is_local_lp(node.message.event().dest_lp);
                let blocked = !node.dependencies.is_empty();
                {
                    let execution = self
                        .owned
                        .as_mut()
                        .expect("owned mode checked")
                        .execution_mut();
                    execution
                        .message_intents
                        .insert(NativeSendId::from(&node.message), key);
                    execution.intents.insert(
                        key,
                        NativeIntentRecord {
                            blocked_local: local && blocked,
                            local_pending_reserved: local && blocked,
                            ..record
                        },
                    );
                    execution.heads.insert(node.cohort, key);
                    if local && blocked {
                        execution.reserved_pending += 1;
                    }
                }
                if local && !blocked {
                    self.processes
                        .get_mut(&node.message.event().dest_lp)
                        .expect("local destination exists")
                        .positives
                        .insert(node.message.clone());
                } else if !local {
                    self.owned
                        .as_mut()
                        .expect("owned mode checked")
                        .routing_mut()
                        .outbox
                        .insert(
                            NativeSendId::from(&node.message),
                            OptimisticOutboundRecord {
                                message: node.message,
                                issuer: local_issuer.clone(),
                            },
                        );
                }
            }
            self.owned
                .as_mut()
                .expect("owned mode checked")
                .execution_mut()
                .next_transition_sequence = sequence;
            self.counters.executions = self
                .counters
                .executions
                .saturating_add(selected.len() as u64);
            committed_messages = Some(messages);
            Ok(())
        });
        if let Err(error) = publish_result {
            return Err(Box::new(
                self.compensate_owned_round(error, before, &changed, &step),
            ));
        }
        let _ = step;
        Ok(committed_messages.unwrap_or_default())
    }

    fn compensate_owned_round(
        &mut self,
        trigger: OptimisticError,
        before: Vec<(LpId, P::Snapshot)>,
        changed: &[LpId],
        _step: &OptimisticOwnedStep,
    ) -> (
        OptimisticError,
        OptimisticOwnedFailurePhase,
        Vec<LpId>,
        bool,
        Option<OptimisticError>,
    ) {
        let mut first_fatal = None;
        let mut restored = Vec::new();
        for (lp_id, snapshot) in &before {
            let lp_id = *lp_id;
            if !changed.contains(&lp_id) {
                continue;
            }
            let result = catch_unwind(AssertUnwindSafe(|| {
                self.processes
                    .get_mut(&lp_id)
                    .expect("compensated LP exists")
                    .process
                    .restore(snapshot)
            }));
            match result {
                Ok(Ok(())) => restored.push(lp_id),
                Ok(Err(reason)) => {
                    first_fatal.get_or_insert(OptimisticError::RestoreFailed { lp_id, reason });
                }
                Err(_) => {
                    first_fatal.get_or_insert(OptimisticError::RestorePanicked(lp_id));
                }
            }
        }
        if first_fatal.is_none() {
            if let Some(owned) = self.owned.as_ref() {
                let own = owned.own_authority();
                if let Err(error) = with_live_authorities(&[own], || {
                    let next_revision = self
                        .owned
                        .as_ref()
                        .expect("owned mode checked")
                        .next_revision_value()?;
                    let mut epochs = BTreeMap::new();
                    for &lp_id in &restored {
                        epochs.insert(lp_id, self.next_epoch(lp_id)?);
                    }
                    for (lp_id, epoch) in epochs {
                        self.processes.get_mut(&lp_id).expect("restored LP").epoch = epoch;
                    }
                    self.owned
                        .as_mut()
                        .expect("owned mode checked")
                        .commit_revision(next_revision);
                    Ok(())
                }) {
                    first_fatal = Some(error);
                }
            }
        }
        for (lp_id, snapshot) in before {
            if catch_unwind(AssertUnwindSafe(|| drop(snapshot))).is_err() {
                first_fatal.get_or_insert(OptimisticError::SnapshotDropPanicked(lp_id));
            }
        }
        if let Some(error) = first_fatal {
            self.poisoned = true;
            return (
                error,
                OptimisticOwnedFailurePhase::PoisonedBeforePublication,
                restored,
                true,
                Some(trigger),
            );
        }
        let _ = trigger;
        (
            trigger,
            OptimisticOwnedFailurePhase::Compensated,
            restored,
            false,
            None,
        )
    }
}

// Metadata shadow contains no user values: cloning, publishing and destroying
// it cannot invoke Snapshot or process code.
struct OwnedQueueShadow {
    positives: Vec<OptimisticMessage>,
    antis: Vec<OptimisticMessage>,
    replay: BTreeSet<DeliveryIdentity>,
    tombstones: BTreeMap<DeliveryIdentity, super::RemoteEvent>,
}

type OwnedRoundFailure = (
    OptimisticError,
    OptimisticOwnedFailurePhase,
    Vec<LpId>,
    bool,
    Option<OptimisticError>,
);

type OwnedControlFailure = (
    OptimisticError,
    OptimisticOwnedFailurePhase,
    Vec<LpId>,
    bool,
    Option<OptimisticError>,
    Vec<OptimisticMessage>,
);

impl<P: OptimisticProcess> OptimisticRuntime<P> {
    fn execute_owned_control(
        &mut self,
        lp_id: LpId,
        start: Option<usize>,
        anti: Option<&OptimisticMessage>,
        horizon: Tick,
        step: &OptimisticOwnedStep,
    ) -> Result<Vec<OptimisticMessage>, Box<OwnedControlFailure>> {
        let rejected = |error| {
            Box::new((
                error,
                OptimisticOwnedFailurePhase::PreCallbackRejected,
                Vec::new(),
                false,
                None,
                Vec::new(),
            ))
        };
        let owned = self
            .owned
            .as_ref()
            .ok_or_else(|| rejected(OptimisticError::OwnedModeRequired))?;
        let peers = owned.peers();
        let next_revision = owned.next_revision_value().map_err(rejected)?;
        self.next_epoch(lp_id).map_err(rejected)?;
        with_live_authorities(&peers, || {
            if !owned.peers_sealed() {
                return Err(OptimisticError::NativePeersNotSealed);
            }
            self.validate_gvt(step.selected[0].event().tick)?;
            Ok(())
        })
        .map_err(rejected)?;
        let mut execution = owned.execution().clone();
        let mut routing = owned.routing().clone();
        let own = owned.own_authority();
        let mut queues = self
            .processes
            .iter()
            .map(|(&id, state)| {
                (
                    id,
                    OwnedQueueShadow {
                        positives: state.positives.all(),
                        antis: state.antis.all(),
                        replay: state.replay_pending.clone(),
                        tombstones: state.tombstones.clone(),
                    },
                )
            })
            .collect::<BTreeMap<_, _>>();
        let suffix = start
            .map(|index| {
                self.processes[&lp_id].history[index..]
                    .iter()
                    .map(|entry| (entry.message.clone(), entry.outputs.clone()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let mut touched = BTreeSet::from([lp_id]);
        let mut emitted = Vec::new();
        let mut known = self.known_deliveries.clone();
        let stage = (|| {
            // Validate the entire authoritative graph, not cached dependencies.
            for key in execution.intents.keys() {
                execution.unresolved_dependencies(key)?;
            }
            for (input, _) in &suffix {
                if anti.is_some_and(|message| {
                    DeliveryIdentity::from(message) == DeliveryIdentity::from(input)
                }) {
                    continue;
                }
                let queue = queues.get_mut(&lp_id).expect("local suffix destination");
                let identity = DeliveryIdentity::from(input);
                if !queue.tombstones.contains_key(&identity) {
                    if !queue
                        .positives
                        .iter()
                        .any(|message| DeliveryIdentity::from(message) == identity)
                    {
                        queue.positives.push(input.clone());
                    }
                    queue.replay.insert(identity);
                }
            }
            for (_, outputs) in &suffix {
                for positive in outputs {
                    let positive_id = NativeSendId::from(positive);
                    let predecessor_key = *execution
                        .message_intents
                        .get(&positive_id)
                        .ok_or(OptimisticError::NativeTransitionDependencyMissing)?;
                    let predecessor = execution
                        .intents
                        .get(&predecessor_key)
                        .ok_or(OptimisticError::NativeTransitionDependencyMissing)?
                        .clone();
                    let cohort = IntentCohortKey {
                        source_lp: predecessor.source_lp,
                        authority: super::AuthorityStorageKey::from(predecessor.authority),
                        logical_id: predecessor.logical_id.clone(),
                    };
                    let head = *execution
                        .heads
                        .get(&cohort)
                        .ok_or(OptimisticError::NativeTransitionDependencyMissing)?;
                    let head_record = execution
                        .intents
                        .get(&head)
                        .ok_or(OptimisticError::NativeTransitionDependencyMissing)?;
                    if head_record.kind == NativeIntentKind::Absent {
                        if head_record
                            .request
                            .as_ref()
                            .is_some_and(|request| request.predecessor_positive() == positive)
                        {
                            continue;
                        }
                        return Err(OptimisticError::ConflictingNativeTransition);
                    }
                    if head != predecessor_key
                        || !predecessor.current
                        || predecessor.message.as_ref() != Some(positive)
                    {
                        return Err(OptimisticError::ConflictingNativeTransition);
                    }
                    let destination = positive.event().dest_lp;
                    let receiver = peers
                        .iter()
                        .find(|peer| peer.owned_lps().binary_search(&destination).is_ok())
                        .ok_or(OptimisticError::UnownedLogicalProcess(destination))?
                        .clone();
                    let local = own.same_issuer(&receiver);
                    let id = execution.mint_id(&own)?;
                    let cancel = positive.as_anti();
                    let request = make_request(
                        id.clone(),
                        positive.clone(),
                        cancel.clone(),
                        own.clone(),
                        receiver.clone(),
                    );
                    execution
                        .intents
                        .get_mut(&predecessor_key)
                        .expect("predecessor validated")
                        .current = false;
                    if predecessor.local_pending_reserved {
                        execution.reserved_pending = execution
                            .reserved_pending
                            .checked_sub(1)
                            .ok_or(OptimisticError::ConflictingNativeReceipt)?;
                        let record = execution
                            .intents
                            .get_mut(&predecessor_key)
                            .expect("predecessor validated");
                        record.local_pending_reserved = false;
                        record.blocked_local = false;
                    }
                    execution.intents.insert(
                        id.key(),
                        NativeIntentRecord {
                            view_id: id.clone(),
                            source_lp: positive.event().source_lp,
                            authority: positive.authority(),
                            logical_id: positive.logical_id().clone(),
                            kind: NativeIntentKind::Absent,
                            message: None,
                            predecessor: Some(predecessor.view_id),
                            request: Some(request.clone()),
                            current: true,
                            applied: false,
                            blocked_local: false,
                            local_pending_reserved: false,
                        },
                    );
                    execution.heads.insert(cohort, id.key());
                    execution
                        .message_intents
                        .insert(NativeSendId::from(&cancel), id.key());
                    known.insert(DeliveryIdentity::from(&cancel), cancel.event().clone());
                    if local {
                        let queue = queues.get_mut(&destination).expect("owned receiver");
                        // A blocked version never entered the queue; its reserved
                        // slot is released above. Its cancellation still applies
                        // through the same actual receiver protocol.
                        queue.antis.push(cancel.clone());
                        touched.insert(destination);
                        execution.reserved_tombstones += 1;
                        execution.receiver_retirements.insert(
                            id.key(),
                            ReceiverRetirementRecord {
                                request: request.clone(),
                                anti_admission: NativeAdmissionCapability::new(
                                    NativeSendId::from(&cancel),
                                    cancel.clone(),
                                    receiver,
                                    own.clone(),
                                    next_revision,
                                    super::NativeAdmissionMembership::Pending,
                                ),
                                applied: None,
                                positive_readback_reserved: false,
                            },
                        );
                    } else {
                        // Keep any old P outbox/ACK reservation until applied
                        // proof, even if the successor had never become Ready.
                        routing.outbox.insert(
                            NativeSendId::from(&cancel),
                            OptimisticOutboundRecord {
                                message: cancel.clone(),
                                issuer: own.clone(),
                            },
                        );
                        execution.reserved_receipts += 1; // eventual source applied proof
                    }
                    emitted.push(cancel);
                }
            }
            if let Some(anti) = anti {
                let identity = DeliveryIdentity::from(anti);
                let (key, receiver_record) = execution
                    .receiver_retirements
                    .iter()
                    .find(|(_, record)| record.request.predecessor_anti() == anti)
                    .map(|(key, record)| (*key, record.clone()))
                    .ok_or(OptimisticError::UnknownNativeTransition)?;
                let queue = queues.get_mut(&lp_id).expect("owned anti receiver");
                if queue
                    .tombstones
                    .get(&identity)
                    .is_some_and(|event| event != anti.event())
                {
                    return Err(OptimisticError::ConflictingDelivery {
                        source_lp: identity.source_lp,
                        incarnation: identity.incarnation,
                    });
                }
                let pending = queue
                    .positives
                    .iter()
                    .any(|message| DeliveryIdentity::from(message) == identity);
                queue
                    .positives
                    .retain(|message| DeliveryIdentity::from(message) != identity);
                queue
                    .antis
                    .retain(|message| DeliveryIdentity::from(message) != identity);
                queue.replay.remove(&identity);
                queue
                    .tombstones
                    .insert(identity.clone(), anti.event().clone());
                known.insert(identity, anti.event().clone());
                execution.reserved_tombstones = execution
                    .reserved_tombstones
                    .checked_sub(1)
                    .ok_or(OptimisticError::ConflictingNativeReceipt)?;
                let self_cancel = own.same_issuer(receiver_record.request.sender());
                let effect = if start.is_some() {
                    NativeRetirementEffect::RolledBackExecuted
                } else if pending {
                    NativeRetirementEffect::RemovedPending
                } else {
                    NativeRetirementEffect::TombstonedAbsent
                };
                let cap = NativeRetirementCapability {
                    request: receiver_record.request.clone(),
                    recorded_revision: next_revision,
                    applied_effect: effect,
                };
                let record = execution
                    .receiver_retirements
                    .get_mut(&key)
                    .expect("actual request");
                record.applied = Some(cap);
                if !self_cancel {
                    execution.reserved_receipts = execution
                        .reserved_receipts
                        .checked_sub(1)
                        .ok_or(OptimisticError::ConflictingNativeReceipt)?;
                }
                if receiver_record.positive_readback_reserved {
                    execution.reserved_receipts = execution
                        .reserved_receipts
                        .checked_sub(1)
                        .ok_or(OptimisticError::ConflictingNativeReceipt)?;
                    let positive = receiver_record.request.predecessor_positive();
                    routing.admissions.insert(
                        NativeSendId::from(positive),
                        NativeAdmissionCapability::new(
                            NativeSendId::from(positive),
                            positive.clone(),
                            own.clone(),
                            receiver_record.request.sender().clone(),
                            next_revision,
                            super::NativeAdmissionMembership::Tombstoned,
                        ),
                    );
                    execution
                        .receiver_retirements
                        .get_mut(&key)
                        .expect("actual request")
                        .positive_readback_reserved = false;
                }
                if self_cancel {
                    execution
                        .intents
                        .get_mut(&key)
                        .ok_or(OptimisticError::NativeTransitionDependencyMissing)?
                        .applied = true;
                }
            }
            // Recompute readiness after the staged applied proof, allowing the
            // last ancestor to promote a local successor in this publication.
            let promotions = execution
                .intents
                .iter()
                .filter_map(|(key, record)| {
                    (record.current && record.blocked_local).then_some((*key, record.clone()))
                })
                .collect::<Vec<_>>();
            for (key, record) in promotions {
                if execution.unresolved_dependencies(&key)?.is_empty() {
                    let message = record
                        .message
                        .ok_or(OptimisticError::NativeTransitionDependencyMissing)?;
                    let destination = message.event().dest_lp;
                    queues
                        .get_mut(&destination)
                        .expect("local blocked receiver")
                        .positives
                        .push(message);
                    touched.insert(destination);
                    let record = execution.intents.get_mut(&key).expect("promotion retained");
                    record.blocked_local = false;
                    record.local_pending_reserved = false;
                    execution.reserved_pending = execution
                        .reserved_pending
                        .checked_sub(1)
                        .ok_or(OptimisticError::ConflictingNativeReceipt)?;
                }
            }
            if execution.transition_record_count() > owned.max_transition_entries() {
                return Err(OptimisticError::TransitionLimitExceeded {
                    limit: owned.max_transition_entries(),
                });
            }
            if routing.outbox.len() > owned.max_outbox_entries() {
                return Err(OptimisticError::OutboxLimitExceeded {
                    limit: owned.max_outbox_entries(),
                });
            }
            let receipts = routing.outbox.len()
                + routing.completed.len()
                + routing.admissions.len()
                + execution.retained_receipt_count()
                + execution.reserved_receipts;
            if receipts > owned.max_receipt_entries() {
                return Err(OptimisticError::ReceiptLimitExceeded {
                    limit: owned.max_receipt_entries(),
                });
            }
            let pending = queues
                .values()
                .map(|queue| queue.positives.len() + queue.antis.len())
                .sum::<usize>()
                + execution.reserved_pending;
            if pending > self.limits.max_pending_events {
                return Err(OptimisticError::PendingLimitExceeded {
                    limit: self.limits.max_pending_events,
                });
            }
            let tombstones = queues
                .values()
                .map(|queue| queue.tombstones.len())
                .sum::<usize>()
                + execution.reserved_tombstones;
            if tombstones > self.limits.max_tombstones {
                return Err(OptimisticError::TombstoneLimitExceeded {
                    limit: self.limits.max_tombstones,
                });
            }
            Ok(())
        })();
        // Known metadata/capacity rejection occurs before callbacks and costs
        // zero. The complete net shadow has not touched any native state.
        stage.map_err(rejected)?;
        let mut epochs = BTreeMap::new();
        for id in &touched {
            epochs.insert(*id, self.next_epoch(*id).map_err(rejected)?);
        }
        let mut before = Vec::new();
        if let Some(start) = start {
            let snapshot = catch_unwind(AssertUnwindSafe(|| {
                self.processes[&lp_id].process.snapshot()
            }));
            let snapshot = match snapshot {
                Ok(snapshot) => snapshot,
                Err(_) => {
                    self.poisoned = true;
                    return Err(Box::new((
                        OptimisticError::SnapshotPanicked(lp_id),
                        OptimisticOwnedFailurePhase::PoisonedBeforePublication,
                        Vec::new(),
                        true,
                        None,
                        Vec::new(),
                    )));
                }
            };
            before.push((lp_id, snapshot));
            let state = self.processes.get_mut(&lp_id).expect("rollback model");
            let restore = catch_unwind(AssertUnwindSafe(|| {
                state.process.restore(&state.history[start].before)
            }));
            let trigger = match restore {
                Ok(Ok(())) => None,
                Ok(Err(reason)) => Some(OptimisticError::RestoreFailed { lp_id, reason }),
                Err(_) => Some(OptimisticError::RestorePanicked(lp_id)),
            };
            if let Some(error) = trigger {
                let result = self.compensate_owned_round(error.clone(), before, &[lp_id], step);
                self.poisoned = true;
                return Err(Box::new((
                    result.0,
                    OptimisticOwnedFailurePhase::PoisonedBeforePublication,
                    result.2,
                    true,
                    result.4,
                    Vec::new(),
                )));
            }
        }
        let mut detached = Vec::new();
        let publication = with_live_authorities(&peers, || {
            let current = self.owned.as_ref().expect("owned control");
            if !current.peers_sealed() {
                return Err(OptimisticError::NativePeersNotSealed);
            }
            if current.revision().checked_add(1) != Some(next_revision) {
                return Err(OptimisticError::AccountingRevisionExhausted);
            }
            for id in epochs.keys() {
                self.next_epoch(*id)?;
            }
            if let Some(start) = start {
                let state = self.processes.get_mut(&lp_id).expect("rollback model");
                for entry in state.history.drain(start..) {
                    detached.push((lp_id, entry.before));
                }
            }
            for (id, queue) in std::mem::take(&mut queues) {
                let state = self.processes.get_mut(&id).expect("shadow LP");
                state.positives = super::EventQueue::default();
                state.antis = super::EventQueue::default();
                for message in queue.positives {
                    state.positives.insert(message);
                }
                for message in queue.antis {
                    state.antis.insert(message);
                }
                state.replay_pending = queue.replay;
                state.tombstones = queue.tombstones;
                if let Some(epoch) = epochs.get(&id) {
                    state.epoch = *epoch;
                }
            }
            self.known_deliveries = std::mem::take(&mut known);
            let owned = self.owned.as_mut().expect("owned control");
            *owned.execution_mut() = std::mem::take(&mut execution);
            *owned.routing_mut() = std::mem::take(&mut routing);
            owned.close_initial_inputs_at_publication();
            owned.commit_revision(next_revision);
            self.initial_open = false;
            self.last_horizon = Some(horizon);
            if start.is_some() {
                self.counters.rollback_attempts = self.counters.rollback_attempts.saturating_add(1);
                self.counters.rolled_back_events = self
                    .counters
                    .rolled_back_events
                    .saturating_add(suffix.len() as u64);
                self.counters.max_rollback_depth =
                    self.counters.max_rollback_depth.max(suffix.len());
            }
            self.counters.canceled_sends = self
                .counters
                .canceled_sends
                .saturating_add(emitted.len() as u64);
            Ok(())
        });
        if let Err(error) = publication {
            if start.is_none() {
                return Err(rejected(error));
            }
            let result = self.compensate_owned_round(error, before, &[lp_id], step);
            return Err(Box::new((
                result.0,
                result.1,
                result.2,
                result.3,
                result.4,
                Vec::new(),
            )));
        }
        detached.extend(before);
        let mut fatal = None;
        for (id, snapshot) in detached {
            if catch_unwind(AssertUnwindSafe(|| drop(snapshot))).is_err() {
                fatal.get_or_insert(OptimisticError::SnapshotDropPanicked(id));
            }
        }
        if let Some(error) = fatal {
            self.poisoned = true;
            return Err(Box::new((
                error,
                OptimisticOwnedFailurePhase::CommittedCleanupFailed,
                Vec::new(),
                true,
                None,
                emitted,
            )));
        }
        Ok(emitted)
    }
}

impl OwnedExecutionState {
    fn stage_cut(
        &self,
        routing: &super::owned_routing::OwnedRootRoutingState,
        gvt: Tick,
        protected: &BTreeSet<TransitionKey>,
    ) -> Result<(Self, super::owned_routing::OwnedRootRoutingState, bool), OptimisticError> {
        let mut next = self.clone();
        let mut next_routing = routing.clone();
        let mut remove = BTreeSet::new();
        for (key, record) in &self.intents {
            self.unresolved_dependencies(key)?;
            let root = record.logical_id.root_parts().is_some();
            let protected_request = protected.contains(key);
            let below = match (&record.message, &record.request) {
                (Some(message), _) => message.event().tick < gvt,
                (_, Some(request)) => {
                    request.predecessor_positive().event().tick < gvt
                        && request.predecessor_anti().event().tick < gvt
                }
                _ => return Err(OptimisticError::NativeTransitionDependencyMissing),
            };
            if !root && !protected_request && below {
                // A live P/anti outbox remains a real work obligation; it cannot
                // be collected just because its node looks historical.
                let has_outbox = self
                    .message_intents
                    .iter()
                    .any(|(id, node)| node == key && routing.outbox.contains_key(id));
                if !has_outbox && !(record.request.is_some() && !record.applied) {
                    remove.insert(*key);
                }
            }
        }
        // Rewire before removal by walking the bounded original graph. No
        // dangling parent or duplicated materialized dependency graph survives.
        for (key, record) in &mut next.intents {
            if remove.contains(key) {
                continue;
            }
            let mut parent = record.predecessor.clone();
            let mut visited = BTreeSet::new();
            while let Some(id) = &parent {
                if !remove.contains(&id.key()) {
                    break;
                }
                if !visited.insert(id.key()) || visited.len() > self.intents.len() {
                    return Err(OptimisticError::NativeTransitionCycle);
                }
                parent = self
                    .intents
                    .get(&id.key())
                    .ok_or(OptimisticError::NativeTransitionDependencyMissing)?
                    .predecessor
                    .clone();
            }
            record.predecessor = parent;
        }
        next.intents.retain(|key, _| !remove.contains(key));
        next.heads.retain(|_, key| !remove.contains(key));
        next.message_intents.retain(|_, key| !remove.contains(key));
        let request_eligible = |key: &TransitionKey, request: &NativeRetirementRequest| {
            !protected.contains(key)
                && request.predecessor_positive().event().tick < gvt
                && request.predecessor_anti().event().tick < gvt
        };
        next.receiver_retirements.retain(|key, record| {
            record.applied.is_none() || !request_eligible(key, &record.request)
        });
        next.source_applied
            .retain(|key, cap| !request_eligible(key, &cap.request));
        // Preserve an admission/completion while its coupled bundle remains,
        // using all referenced envelope ticks rather than its own tick alone.
        let mut retained_sends = BTreeSet::new();
        for record in next.receiver_retirements.values() {
            retained_sends.insert(NativeSendId::from(record.request.predecessor_positive()));
            retained_sends.insert(NativeSendId::from(record.request.predecessor_anti()));
        }
        for record in next.intents.values() {
            if let Some(request) = &record.request {
                retained_sends.insert(NativeSendId::from(request.predecessor_positive()));
                retained_sends.insert(NativeSendId::from(request.predecessor_anti()));
            }
        }
        next_routing
            .completed
            .retain(|id, record| record.message.event().tick >= gvt || retained_sends.contains(id));
        next_routing
            .admissions
            .retain(|id, cap| cap.message().event().tick >= gvt || retained_sends.contains(id));
        for key in next.intents.keys() {
            next.unresolved_dependencies(key)?;
        }
        let changed = next.intents.len() != self.intents.len()
            || next.receiver_retirements.len() != self.receiver_retirements.len()
            || next.source_applied.len() != self.source_applied.len()
            || next_routing.completed.len() != routing.completed.len()
            || next_routing.admissions.len() != routing.admissions.len();
        Ok((next, next_routing, changed))
    }
}

#[cfg(test)]
mod private_tests {
    use super::*;
    use kairo_ecs_types::{EntityId, SimDuration};
    #[derive(Clone)]
    struct Model {
        value: u8,
        fanout: usize,
        destination: LpId,
        snapshots: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    }
    impl OptimisticProcess for Model {
        type Snapshot = u8;
        fn snapshot(&self) -> u8 {
            self.snapshots
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            self.value
        }
        fn restore(&mut self, value: &u8) -> Result<(), super::super::OptimisticStateError> {
            self.value = *value;
            Ok(())
        }
        fn on_event(
            &mut self,
            event: &super::super::RemoteEvent,
        ) -> Vec<super::super::RemoteEvent> {
            self.value += 1;
            (0..self.fanout)
                .map(|_| super::super::RemoteEvent {
                    source_lp: event.dest_lp,
                    dest_lp: self.destination,
                    tick: Tick::from_ticks(event.tick.ticks() + 1),
                    event_payload: vec![1],
                })
                .collect()
        }
    }
    fn runtime(fanout: usize) -> OptimisticRuntime<Model> {
        let partition = super::super::PartitionPlan::from_entities(
            1,
            SimDuration::from_ticks(1),
            vec![EntityId::new(1, 0)],
        )
        .unwrap();
        let mut runtime = OptimisticRuntime::new_owned(
            partition,
            BTreeMap::from([(LpId(0), Vec::new())]),
            BTreeMap::from([(
                LpId(0),
                Model {
                    value: 0,
                    fanout,
                    destination: LpId(0),
                    snapshots: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
                },
            )]),
            super::super::OptimisticOwnedOptions {
                simulation_namespace: 19,
                current_authorities: BTreeMap::from([(
                    LpId(0),
                    OptimisticAuthority::Scoped {
                        simulation_namespace: 19,
                        ownership_epoch: 1,
                    },
                )]),
                emission_epochs: BTreeMap::from([(LpId(0), 1)]),
                local_limits: super::super::OptimisticLimits::default(),
                max_global_lps: 1,
                max_outbox_entries: 16,
                max_transition_entries: 32,
                max_receipt_entries: 32,
            },
        )
        .unwrap();
        runtime.seal_native_peers().unwrap();
        runtime
            .schedule_initial(
                1,
                super::super::RemoteEvent {
                    source_lp: LpId(0),
                    dest_lp: LpId(0),
                    tick: Tick::from_ticks(5),
                    event_payload: vec![0],
                },
            )
            .unwrap();
        runtime
    }
    fn graph() -> OwnedExecutionState {
        let mut runtime = runtime(1);
        runtime
            .run_owned_until_with_budget(Tick::from_ticks(5), 1)
            .unwrap();
        runtime.owned.as_ref().unwrap().execution().clone()
    }
    fn child(execution: &OwnedExecutionState) -> TransitionKey {
        *execution
            .intents
            .iter()
            .find(|(_, record)| record.logical_id.root_parts().is_none())
            .unwrap()
            .0
    }
    #[test]
    fn bounded_graph_rejects_missing_cycle_fork_and_cross_cohort() {
        let original = graph();
        let key = child(&original);
        let mut missing = original.clone();
        let mut unknown = missing.intents[&key].view_id.clone();
        unknown.sequence = 999;
        missing.intents.get_mut(&key).unwrap().predecessor = Some(unknown);
        assert_eq!(
            missing.unresolved_dependencies(&key).unwrap_err(),
            OptimisticError::NativeTransitionDependencyMissing
        );
        let mut cycle = original.clone();
        cycle.intents.get_mut(&key).unwrap().predecessor =
            Some(cycle.intents[&key].view_id.clone());
        assert_eq!(
            cycle.unresolved_dependencies(&key).unwrap_err(),
            OptimisticError::NativeTransitionCycle
        );
        let mut cross = original.clone();
        let root = cross
            .intents
            .iter()
            .find(|(_, record)| record.logical_id.root_parts().is_some())
            .unwrap()
            .1
            .view_id
            .clone();
        cross.intents.get_mut(&key).unwrap().predecessor = Some(root);
        assert_eq!(
            cross.unresolved_dependencies(&key).unwrap_err(),
            OptimisticError::NativeTransitionDependencyMissing
        );
        let mut fork = original.clone();
        let mut second = fork.intents[&key].clone();
        second.view_id.sequence = 999;
        second.predecessor = Some(fork.intents[&key].view_id.clone());
        let mut third = second.clone();
        third.view_id.sequence = 1000;
        fork.intents.insert(second.view_id.key(), second.clone());
        fork.intents.insert(third.view_id.key(), third);
        assert_eq!(
            fork.unresolved_dependencies(&second.view_id.key())
                .unwrap_err(),
            OptimisticError::NativeTransitionFork
        );
    }
    #[test]
    fn shadow_incarnation_and_id_batch_exhaustion_restore_without_allocator_leak() {
        for incarnation in [true, false] {
            let mut runtime = runtime(2);
            if incarnation {
                runtime.next_incarnation.insert(LpId(0), Some(u64::MAX));
            } else {
                runtime
                    .owned
                    .as_mut()
                    .unwrap()
                    .execution_mut()
                    .next_transition_sequence = Some(u64::MAX);
            }
            let allocator = runtime.next_incarnation.clone();
            let next_id = runtime
                .owned
                .as_ref()
                .unwrap()
                .execution()
                .next_transition_sequence;
            let pending = runtime.pending_events(LpId(0)).unwrap();
            let failure = runtime
                .run_owned_until_with_budget(Tick::from_ticks(5), 1)
                .unwrap_err();
            assert_eq!(failure.phase(), OptimisticOwnedFailurePhase::Compensated);
            assert_eq!(runtime.processes[&LpId(0)].process.value, 0);
            assert_eq!(runtime.next_incarnation, allocator);
            assert_eq!(
                runtime
                    .owned
                    .as_ref()
                    .unwrap()
                    .execution()
                    .next_transition_sequence,
                next_id
            );
            assert_eq!(runtime.pending_events(LpId(0)).unwrap(), pending);
            assert_eq!(runtime.native_intents().unwrap().len(), 1);
            assert!(runtime.ready_native_sends().unwrap().is_empty());
        }
    }
    #[test]
    fn epoch_headroom_rejects_before_snapshot_and_handler() {
        let mut runtime = runtime(2);
        runtime.processes.get_mut(&LpId(0)).unwrap().epoch = u64::MAX;
        let snapshot_count = runtime.processes[&LpId(0)]
            .process
            .snapshots
            .load(std::sync::atomic::Ordering::SeqCst);
        let failure = runtime
            .run_owned_until_with_budget(Tick::from_ticks(5), 1)
            .unwrap_err();
        assert_eq!(
            runtime.processes[&LpId(0)]
                .process
                .snapshots
                .load(std::sync::atomic::Ordering::SeqCst),
            snapshot_count
        );
        assert_eq!(
            failure.phase(),
            OptimisticOwnedFailurePhase::PreCallbackRejected
        );
        assert_eq!(failure.progress().budget_used, 0);
        assert_eq!(runtime.processes[&LpId(0)].process.value, 0);
    }
    #[test]
    fn pruning_rewires_surviving_parent_and_keeps_lifetime_root_reservation() {
        let mut execution = graph();
        let key = child(&execution);
        let mut survivor = execution.intents[&key].clone();
        survivor.view_id.sequence = 999;
        survivor.predecessor = Some(execution.intents[&key].view_id.clone());
        survivor.message.as_mut().unwrap().event.tick = Tick::from_ticks(30);
        execution.intents.get_mut(&key).unwrap().current = false;
        execution
            .intents
            .insert(survivor.view_id.key(), survivor.clone());
        let (pruned, _, changed) = execution
            .stage_cut(
                &super::super::owned_routing::OwnedRootRoutingState::default(),
                Tick::from_ticks(26),
                &BTreeSet::new(),
            )
            .unwrap();
        assert!(changed);
        assert!(!pruned.intents.contains_key(&key));
        assert!(pruned.intents[&survivor.view_id.key()]
            .predecessor
            .is_none());
        assert!(pruned
            .intents
            .values()
            .any(|record| record.logical_id.root_parts().is_some()));
        assert!(pruned
            .unresolved_dependencies(&survivor.view_id.key())
            .unwrap()
            .is_empty());
    }
    #[test]
    fn revision_exhaustion_rejects_before_callbacks_and_group_floor() {
        let mut runtime = runtime(1);
        runtime.owned.as_mut().unwrap().exhaust_revision_for_test();
        let failure = runtime
            .run_owned_until_with_budget(Tick::from_ticks(5), 1)
            .unwrap_err();
        assert_eq!(
            failure.cause(),
            &OptimisticError::AccountingRevisionExhausted
        );
        assert_eq!(failure.progress().budget_used, 0);
        assert_eq!(runtime.processes[&LpId(0)].process.value, 0);
        assert!(!runtime.initial_inputs_closed());
        let mut cut_runtime = self::runtime(0);
        cut_runtime.close_initial_inputs().unwrap();
        cut_runtime
            .owned
            .as_mut()
            .unwrap()
            .exhaust_revision_for_test();
        let before_report = cut_runtime.report();
        let before_accounting = cut_runtime.accounting_snapshot().unwrap();
        let before_queue = cut_runtime.pending_events(LpId(0)).unwrap();
        let token = cut_runtime.state_token(LpId(0)).unwrap();
        let failure = OptimisticRuntime::fossil_collect_native_group(
            &mut [&mut cut_runtime],
            Tick::from_ticks(4),
        )
        .unwrap_err();
        assert_eq!(
            failure.cause(),
            &OptimisticError::AccountingRevisionExhausted
        );
        assert_eq!(cut_runtime.gvt, Tick::ZERO);
        assert_eq!(cut_runtime.report(), before_report);
        assert_eq!(
            cut_runtime.accounting_snapshot().unwrap(),
            before_accounting
        );
        assert_eq!(cut_runtime.pending_events(LpId(0)).unwrap(), before_queue);
        assert!(cut_runtime.validate_state_token(token));
    }
    #[test]
    fn cut_retains_old30_new25_bundle_at26_and_equality_then_prunes_without_head_resurrection() {
        let mut execution = graph();
        let old_key = child(&execution);
        let mut old = execution.intents[&old_key].clone();
        old.message.as_mut().unwrap().event.tick = Tick::from_ticks(30);
        old.current = false;
        execution.intents.insert(old_key, old.clone());
        let issuer = old.view_id.issuer.clone();
        let absent_id = execution.mint_id(&issuer).unwrap();
        let positive = old.message.as_ref().unwrap().clone();
        let anti = positive.as_anti();
        let request = make_request(
            absent_id.clone(),
            positive.clone(),
            anti.clone(),
            issuer.clone(),
            issuer.clone(),
        );
        execution.intents.insert(
            absent_id.key(),
            NativeIntentRecord {
                view_id: absent_id.clone(),
                source_lp: old.source_lp,
                authority: old.authority,
                logical_id: old.logical_id.clone(),
                kind: NativeIntentKind::Absent,
                message: None,
                predecessor: Some(old.view_id.clone()),
                request: Some(request.clone()),
                current: false,
                applied: true,
                blocked_local: false,
                local_pending_reserved: false,
            },
        );
        let newer_id = execution.mint_id(&issuer).unwrap();
        let mut newer = old.clone();
        newer.view_id = newer_id.clone();
        newer.predecessor = Some(absent_id.clone());
        newer.current = true;
        newer.message.as_mut().unwrap().event.tick = Tick::from_ticks(25);
        execution.intents.insert(newer_id.key(), newer);
        let cohort = IntentCohortKey {
            source_lp: old.source_lp,
            authority: super::super::AuthorityStorageKey::from(old.authority),
            logical_id: old.logical_id.clone(),
        };
        execution.heads.insert(cohort.clone(), newer_id.key());
        let cap = NativeRetirementCapability {
            request: request.clone(),
            recorded_revision: 50,
            applied_effect: NativeRetirementEffect::TombstonedAbsent,
        };
        let anti_cap = NativeAdmissionCapability::new(
            NativeSendId::from(&anti),
            anti.clone(),
            issuer.clone(),
            issuer.clone(),
            49,
            super::super::NativeAdmissionMembership::Pending,
        );
        execution.receiver_retirements.insert(
            absent_id.key(),
            ReceiverRetirementRecord {
                request: request.clone(),
                anti_admission: anti_cap.clone(),
                applied: Some(cap.clone()),
                positive_readback_reserved: false,
            },
        );
        execution.source_applied.insert(absent_id.key(), cap);
        assert_eq!(execution.retained_receipt_count(), 1); // shared constant-size bundle, unique key
        let mut routing = super::super::owned_routing::OwnedRootRoutingState::default();
        routing
            .admissions
            .insert(NativeSendId::from(&anti), anti_cap);
        routing.admissions.insert(
            NativeSendId::from(&positive),
            NativeAdmissionCapability::new(
                NativeSendId::from(&positive),
                positive.clone(),
                issuer.clone(),
                issuer.clone(),
                50,
                super::super::NativeAdmissionMembership::Tombstoned,
            ),
        );
        routing.completed.insert(
            NativeSendId::from(&positive),
            OptimisticOutboundRecord {
                message: positive,
                issuer,
            },
        );
        let (at26, routing26, _) = execution
            .stage_cut(&routing, Tick::from_ticks(26), &BTreeSet::new())
            .unwrap();
        assert!(!at26.intents.contains_key(&newer_id.key()));
        assert!(!at26.heads.contains_key(&cohort));
        assert!(!at26.intents[&old_key].current);
        assert!(!at26.intents[&absent_id.key()].current);
        assert!(at26.source_applied.contains_key(&absent_id.key()));
        assert_eq!(routing26.admissions.len(), 2);
        assert!(at26.receiver_retirements.contains_key(&absent_id.key()));
        let (at30, routing30, changed) = at26
            .stage_cut(&routing26, Tick::from_ticks(30), &BTreeSet::new())
            .unwrap();
        assert!(!changed);
        assert!(at30.source_applied.contains_key(&absent_id.key()));
        let (lost, _, _) = at30
            .stage_cut(
                &routing30,
                Tick::from_ticks(31),
                &BTreeSet::from([absent_id.key()]),
            )
            .unwrap();
        assert!(lost.receiver_retirements.contains_key(&absent_id.key()));
        assert!(lost.source_applied.contains_key(&absent_id.key()));
        let (collected, routing31, _) = at30
            .stage_cut(&routing30, Tick::from_ticks(31), &BTreeSet::new())
            .unwrap();
        assert_eq!(collected.intents.len(), 1);
        assert!(collected.source_applied.is_empty());
        assert!(collected.receiver_retirements.is_empty());
        assert!(routing31.admissions.is_empty());
        assert!(routing31.completed.is_empty());
    }
    #[test]
    fn actual_self_cancellation_uses_one_bundle_and_promotes_reserved_local_successor_once() {
        let partition = super::super::PartitionPlan::from_entities(
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
            (LpId(2), Vec::new()),
        ]);
        let authorities = (0..3)
            .map(|id| {
                (
                    LpId(id),
                    OptimisticAuthority::Scoped {
                        simulation_namespace: 29,
                        ownership_epoch: 1,
                    },
                )
            })
            .collect::<BTreeMap<_, _>>();
        let make = |lps: &[u32], receipts, transitions| {
            OptimisticRuntime::new_owned(
                partition.clone(),
                topology.clone(),
                lps.iter()
                    .map(|id| {
                        (
                            LpId(*id),
                            Model {
                                value: 0,
                                fanout: usize::from(*id == 1),
                                destination: LpId(2),
                                snapshots: std::sync::Arc::new(
                                    std::sync::atomic::AtomicUsize::new(0),
                                ),
                            },
                        )
                    })
                    .collect(),
                super::super::OptimisticOwnedOptions {
                    simulation_namespace: 29,
                    current_authorities: authorities.clone(),
                    emission_epochs: lps.iter().map(|id| (LpId(*id), 1)).collect(),
                    local_limits: super::super::OptimisticLimits::default(),
                    max_global_lps: 3,
                    max_outbox_entries: 8,
                    max_transition_entries: transitions,
                    max_receipt_entries: receipts,
                },
            )
            .unwrap()
        };
        let mut source = make(&[0], 8, 8);
        let mut local = make(&[1, 2], 2, 5);
        let source_peer = source.native_accounting_authority().unwrap();
        let local_peer = local.native_accounting_authority().unwrap();
        source.register_native_peer(local_peer).unwrap();
        local.register_native_peer(source_peer).unwrap();
        source.seal_native_peers().unwrap();
        local.seal_native_peers().unwrap();
        source
            .schedule_initial(
                1,
                super::super::RemoteEvent {
                    source_lp: LpId(0),
                    dest_lp: LpId(1),
                    tick: Tick::from_ticks(5),
                    event_payload: vec![0],
                },
            )
            .unwrap();
        local
            .schedule_initial(
                2,
                super::super::RemoteEvent {
                    source_lp: LpId(1),
                    dest_lp: LpId(1),
                    tick: Tick::from_ticks(20),
                    event_payload: vec![0],
                },
            )
            .unwrap();
        local
            .run_owned_until_with_budget(Tick::from_ticks(20), 1)
            .unwrap();
        local
            .admit_native(&source.ready_native_sends().unwrap()[0])
            .unwrap();
        local
            .run_owned_until_with_budget(Tick::from_ticks(20), 1)
            .unwrap(); // rollback emits self anti at21
        local
            .run_owned_until_with_budget(Tick::from_ticks(20), 2)
            .unwrap(); // replay makes blocked N21
        let execution = local.owned.as_ref().unwrap().execution();
        assert_eq!(execution.transition_record_count(), 5);
        assert_eq!(execution.retained_receipt_count(), 1);
        assert!(execution.source_applied.is_empty());
        assert_eq!(execution.reserved_receipts, 0);
        assert_eq!(execution.reserved_pending, 1);
        assert_eq!(local.accounting_snapshot().unwrap().retirement_count(), 1);
        let request = local
            .pending_native_retirement_requests()
            .unwrap()
            .remove(0);
        let revision = local.accounting_revision().unwrap();
        let cap = local.receive_native_retirement(&request).unwrap();
        local.acknowledge_native_admission(cap).unwrap();
        assert_eq!(local.accounting_revision().unwrap(), revision);
        local
            .run_owned_until_with_budget(Tick::from_ticks(21), 1)
            .unwrap(); // anti first + promotion
        assert_eq!(local.accounting_revision().unwrap(), revision + 1);
        let execution = local.owned.as_ref().unwrap().execution();
        assert_eq!(execution.reserved_pending, 0);
        assert_eq!(execution.retained_receipt_count(), 1);
        assert!(execution.source_applied.is_empty());
        assert_eq!(local.accounting_snapshot().unwrap().retirement_count(), 0);
        let proof = local.applied_native_retirements().unwrap().remove(0);
        local.acknowledge_native_retirement(proof.clone()).unwrap();
        local.acknowledge_native_retirement(proof).unwrap();
        assert_eq!(local.accounting_revision().unwrap(), revision + 1);
        assert_eq!(local.pending_events(LpId(2)).unwrap().len(), 2);
        assert_eq!(
            local
                .accounting_snapshot()
                .unwrap()
                .retained_receipt_count(),
            2
        ); // remote root + shared self bundle
    }
}
