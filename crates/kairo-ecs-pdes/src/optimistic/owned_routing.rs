use std::collections::{BTreeMap, BTreeSet};

use super::owned::with_live_authorities;
use super::{
    AuthorityStorageKey, LogicalEventId, LpId, NativeAccountingAuthority, OptimisticAuthority,
    OptimisticError, OptimisticMessage, OptimisticMessageKind, OptimisticProcess,
    OptimisticRuntime, Tick,
};
use super::{DeliveryIdentity, OptimisticEventOrderKey};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct RootCohortId {
    pub source_lp: LpId,
    pub logical_id: LogicalEventId,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct NativeSendId {
    pub source_lp: LpId,
    pub authority: AuthorityStorageKey,
    pub logical_id: LogicalEventId,
    pub incarnation: u64,
    pub kind: OptimisticMessageKind,
}

impl From<&OptimisticMessage> for NativeSendId {
    fn from(message: &OptimisticMessage) -> Self {
        Self {
            source_lp: message.event().source_lp,
            authority: AuthorityStorageKey::from(message.authority()),
            logical_id: message.logical_id().clone(),
            incarnation: message.incarnation(),
            kind: message.kind(),
        }
    }
}

/// Exact storage identity for a native send. It intentionally has no public
/// ordering or hashing implementation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OptimisticSendKey {
    inner: NativeSendId,
}

impl OptimisticSendKey {
    pub fn source_lp(&self) -> LpId {
        self.inner.source_lp
    }

    pub fn authority(&self) -> OptimisticAuthority {
        self.inner.authority.into()
    }

    pub fn logical_id(&self) -> &LogicalEventId {
        &self.inner.logical_id
    }

    pub fn incarnation(&self) -> u64 {
        self.inner.incarnation
    }

    pub fn kind(&self) -> OptimisticMessageKind {
        self.inner.kind
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OptimisticOutboundStatus {
    Ready,
    BlockedReplacement,
    RetiredPredecessorAwaitingAccounting,
}

#[derive(Clone, Debug)]
pub struct OptimisticOutboundView {
    key: OptimisticSendKey,
    message: OptimisticMessage,
    issuer: NativeAccountingAuthority,
    status: OptimisticOutboundStatus,
}

impl OptimisticOutboundView {
    pub fn key(&self) -> &OptimisticSendKey {
        &self.key
    }

    pub fn message(&self) -> &OptimisticMessage {
        &self.message
    }

    pub fn issuer(&self) -> &NativeAccountingAuthority {
        &self.issuer
    }

    pub fn status(&self) -> OptimisticOutboundStatus {
        self.status
    }
}

#[derive(Clone, Debug)]
pub struct NativeOutboundSend {
    key: OptimisticSendKey,
    message: OptimisticMessage,
    issuer: NativeAccountingAuthority,
}

impl NativeOutboundSend {
    pub fn key(&self) -> &OptimisticSendKey {
        &self.key
    }

    pub fn message(&self) -> &OptimisticMessage {
        &self.message
    }

    pub fn issuer(&self) -> &NativeAccountingAuthority {
        &self.issuer
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeAdmissionMembership {
    Pending,
    Executed,
    Tombstoned,
}

#[derive(Clone, Debug)]
pub struct NativeAdmissionCapability {
    key: OptimisticSendKey,
    message: OptimisticMessage,
    receiver: NativeAccountingAuthority,
    sender: NativeAccountingAuthority,
    recorded_revision: u64,
    recorded_membership: NativeAdmissionMembership,
}

impl NativeAdmissionCapability {
    pub fn key(&self) -> &OptimisticSendKey {
        &self.key
    }

    pub fn message(&self) -> &OptimisticMessage {
        &self.message
    }

    pub fn receiver(&self) -> &NativeAccountingAuthority {
        &self.receiver
    }

    pub fn sender(&self) -> &NativeAccountingAuthority {
        &self.sender
    }

    pub fn recorded_revision(&self) -> u64 {
        self.recorded_revision
    }

    pub fn recorded_membership(&self) -> NativeAdmissionMembership {
        self.recorded_membership
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OptimisticAccountingSnapshot {
    revision: u64,
    local_positive_count: usize,
    local_anti_count: usize,
    local_replay_count: usize,
    ready_positive_count: usize,
    ready_anti_count: usize,
    blocked_count: usize,
    retirement_count: usize,
    reserved_receipt_count: usize,
    retained_receipt_count: usize,
    local_minimum: Option<Tick>,
    outbound_minimum: Option<Tick>,
    minimum_obligation_tick: Option<Tick>,
    frontiers: BTreeMap<LpId, Tick>,
}

impl OptimisticAccountingSnapshot {
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn local_positive_count(&self) -> usize {
        self.local_positive_count
    }
    pub fn local_anti_count(&self) -> usize {
        self.local_anti_count
    }
    pub fn local_replay_count(&self) -> usize {
        self.local_replay_count
    }
    pub fn ready_positive_count(&self) -> usize {
        self.ready_positive_count
    }
    pub fn ready_anti_count(&self) -> usize {
        self.ready_anti_count
    }
    pub fn blocked_count(&self) -> usize {
        self.blocked_count
    }
    pub fn retirement_count(&self) -> usize {
        self.retirement_count
    }
    pub fn reserved_receipt_count(&self) -> usize {
        self.reserved_receipt_count
    }
    pub fn retained_receipt_count(&self) -> usize {
        self.retained_receipt_count
    }
    pub fn local_minimum(&self) -> Option<Tick> {
        self.local_minimum
    }
    pub fn outbound_minimum(&self) -> Option<Tick> {
        self.outbound_minimum
    }
    pub fn minimum_obligation_tick(&self) -> Option<Tick> {
        self.minimum_obligation_tick
    }
    pub fn frontiers(&self) -> &BTreeMap<LpId, Tick> {
        &self.frontiers
    }
}

#[derive(Default)]
pub(super) struct OwnedRootRoutingState {
    pub roots: BTreeSet<RootCohortId>,
    pub outbox: BTreeMap<NativeSendId, OptimisticOutboundRecord>,
    pub completed: BTreeMap<NativeSendId, OptimisticOutboundRecord>,
    pub admissions: BTreeMap<NativeSendId, NativeAdmissionCapability>,
}

#[derive(Clone)]
pub(super) struct OptimisticOutboundRecord {
    pub message: OptimisticMessage,
    pub issuer: NativeAccountingAuthority,
}

impl<P: OptimisticProcess> OptimisticRuntime<P> {
    pub fn outbound_pending(&self) -> Result<Vec<OptimisticOutboundView>, OptimisticError> {
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
                .routing()
                .outbox
                .iter()
                .map(|(id, record)| OptimisticOutboundView {
                    key: OptimisticSendKey { inner: id.clone() },
                    message: record.message.clone(),
                    issuer: record.issuer.clone(),
                    status: OptimisticOutboundStatus::Ready,
                })
                .collect())
        })
    }

    pub fn ready_native_sends(&self) -> Result<Vec<NativeOutboundSend>, OptimisticError> {
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
                .routing()
                .outbox
                .iter()
                .map(|(id, record)| NativeOutboundSend {
                    key: OptimisticSendKey { inner: id.clone() },
                    message: record.message.clone(),
                    issuer: record.issuer.clone(),
                })
                .collect())
        })
    }

    pub fn admit_native(
        &mut self,
        send: &NativeOutboundSend,
    ) -> Result<NativeAdmissionCapability, OptimisticError> {
        self.ensure_healthy()?;
        let own_state = self
            .owned
            .as_ref()
            .ok_or(OptimisticError::OwnedModeRequired)?;
        let mut peers = own_state.peers();
        peers.push(send.issuer.clone());
        let peers_sealed = own_state.peers_sealed();
        with_live_authorities(&peers, || {
            if !peers_sealed {
                return Err(OptimisticError::NativePeersNotSealed);
            }
            self.admit_native_locked(send)
        })
    }

    fn admit_native_locked(
        &mut self,
        send: &NativeOutboundSend,
    ) -> Result<NativeAdmissionCapability, OptimisticError> {
        let owned = self.owned.as_ref().expect("owned mode checked");
        let message = &send.message;
        let source_lp = message.event().source_lp;
        let registered = owned
            .peers()
            .into_iter()
            .find(|peer| peer.same_issuer(&send.issuer));
        let Some(source_issuer) = registered else {
            return Err(OptimisticError::UnregisteredNativeIssuer {
                runtime_id: send.issuer.runtime_id(),
                recovery_generation: send.issuer.recovery_generation(),
            });
        };
        if !source_issuer.owned_lps().binary_search(&source_lp).is_ok() {
            return Err(OptimisticError::NativeSendIssuerMismatch { source_lp });
        }
        let expected_authority = source_issuer
            .current_authorities()
            .get(&source_lp)
            .copied()
            .ok_or(OptimisticError::NativeSendIssuerMismatch { source_lp })?;
        if message.authority() != expected_authority {
            return Err(OptimisticError::NativeAuthorityMismatch {
                source_lp,
                expected: expected_authority,
                actual: message.authority(),
            });
        }
        let actual_id = NativeSendId::from(message);
        if send.key.inner != actual_id || message.kind() != OptimisticMessageKind::Positive {
            return Err(OptimisticError::UnknownNativeSend);
        }
        let event = message.event();
        if !owned.is_local_lp(event.dest_lp) {
            if !owned
                .own_authority_ref()
                .global_partition()
                .segments()
                .iter()
                .any(|segment| segment.id == event.dest_lp)
            {
                return Err(OptimisticError::UnknownLogicalProcess(event.dest_lp));
            }
            return Err(OptimisticError::UnownedLogicalProcess(event.dest_lp));
        }
        if !source_issuer
            .global_partition()
            .segments()
            .iter()
            .any(|segment| segment.id == event.dest_lp)
        {
            return Err(OptimisticError::UnknownLogicalProcess(event.dest_lp));
        }
        if event.source_lp != event.dest_lp
            && !source_issuer
                .global_topology()
                .get(&event.source_lp)
                .is_some_and(|destinations| destinations.contains(&event.dest_lp))
        {
            return Err(OptimisticError::RouteMissing {
                source: event.source_lp,
                destination: event.dest_lp,
            });
        }
        self.validate_gvt(event.tick)?;
        OptimisticEventOrderKey::try_from_parts(
            event.tick,
            event.source_lp,
            message.logical_id().clone(),
        )?;
        if let Some((root_source, _)) = message.logical_id().root_parts() {
            if root_source != event.source_lp {
                return Err(OptimisticError::EnvelopeSourceMismatch {
                    declared: root_source,
                    actual: event.source_lp,
                });
            }
        }
        let identity = DeliveryIdentity::from(message);
        let owned = self.owned.as_ref().expect("owned mode checked");
        let key = actual_id.clone();
        if let Some(existing) = owned.routing().admissions.get(&key) {
            if !existing.sender.same_issuer(&send.issuer) || existing.message != *message {
                return Err(OptimisticError::ConflictingNativeReceipt);
            }
            let still_pending = self.processes.get(&event.dest_lp).is_some_and(|state| {
                state
                    .positives
                    .contains(&identity)
                    .is_some_and(|stored| stored == message)
                    && self.known_deliveries.get(&identity) == Some(event)
            });
            if !still_pending {
                return Err(OptimisticError::ConflictingNativeReceipt);
            }
            return Ok(existing.clone());
        }
        if let Some(existing) = self.known_deliveries.get(&identity) {
            if existing != event {
                return Err(OptimisticError::ConflictingDelivery {
                    source_lp,
                    incarnation: message.incarnation(),
                });
            }
            return Err(OptimisticError::DuplicatePositive {
                source_lp,
                incarnation: message.incarnation(),
            });
        }
        self.ensure_pending_capacity(1)?;
        let receipt_count = owned.routing().outbox.len()
            + owned.routing().completed.len()
            + owned.routing().admissions.len();
        if receipt_count >= owned.max_receipt_entries() {
            return Err(OptimisticError::ReceiptLimitExceeded {
                limit: owned.max_receipt_entries(),
            });
        }
        let next_epoch = self.next_epoch(event.dest_lp)?;
        let next_revision = owned.next_revision_value()?;
        let receiver = owned.own_authority();
        let cap = NativeAdmissionCapability {
            key: OptimisticSendKey { inner: key.clone() },
            message: message.clone(),
            receiver,
            sender: send.issuer.clone(),
            recorded_revision: next_revision,
            recorded_membership: NativeAdmissionMembership::Pending,
        };
        let process = self
            .processes
            .get_mut(&event.dest_lp)
            .expect("owned destination validated");
        process.epoch = next_epoch;
        process.positives.insert(message.clone());
        self.known_deliveries.insert(identity, event.clone());
        self.owned
            .as_mut()
            .expect("owned mode checked")
            .routing_mut()
            .admissions
            .insert(key, cap.clone());
        self.owned
            .as_mut()
            .expect("owned mode checked")
            .commit_revision(next_revision);
        Ok(cap)
    }

    pub fn acknowledge_native_admission(
        &mut self,
        cap: NativeAdmissionCapability,
    ) -> Result<(), OptimisticError> {
        self.ensure_healthy()?;
        let own_state = self
            .owned
            .as_ref()
            .ok_or(OptimisticError::OwnedModeRequired)?;
        let mut peers = own_state.peers();
        peers.push(cap.sender.clone());
        peers.push(cap.receiver.clone());
        let peers_sealed = own_state.peers_sealed();
        with_live_authorities(&peers, || {
            if !peers_sealed {
                return Err(OptimisticError::NativePeersNotSealed);
            }
            self.acknowledge_native_locked(cap)
        })
    }

    fn acknowledge_native_locked(
        &mut self,
        cap: NativeAdmissionCapability,
    ) -> Result<(), OptimisticError> {
        let owned = self.owned.as_ref().expect("owned mode checked");
        let own = owned.own_authority();
        let source_lp = cap.message.event().source_lp;
        if !own.same_issuer(&cap.sender) {
            return Err(OptimisticError::NativeSendIssuerMismatch { source_lp });
        }
        let id = cap.key.inner.clone();
        if NativeSendId::from(&cap.message) != id
            || cap.message.kind() != OptimisticMessageKind::Positive
        {
            return Err(OptimisticError::ConflictingNativeReceipt);
        }
        let receiver = owned.peers().into_iter().find(|peer| {
            peer.owned_lps()
                .binary_search(&cap.message.event().dest_lp)
                .is_ok()
        });
        if !receiver.is_some_and(|registered| registered.same_issuer(&cap.receiver)) {
            return Err(OptimisticError::NativeSendIssuerMismatch { source_lp });
        }
        if let Some(completed) = owned.routing().completed.get(&id) {
            return if completed.message == cap.message && completed.issuer.same_issuer(&cap.sender)
            {
                Ok(())
            } else {
                Err(OptimisticError::ConflictingNativeReceipt)
            };
        }
        let Some(active) = owned.routing().outbox.get(&id) else {
            return Err(OptimisticError::UnknownNativeSend);
        };
        if active.message != cap.message || !active.issuer.same_issuer(&cap.sender) {
            return Err(OptimisticError::ConflictingNativeReceipt);
        }
        let next_revision = owned.next_revision_value()?;
        let routing = self
            .owned
            .as_mut()
            .expect("owned mode checked")
            .routing_mut();
        let record = routing
            .outbox
            .remove(&id)
            .expect("active record preflighted");
        routing.completed.insert(id, record);
        self.owned
            .as_mut()
            .expect("owned mode checked")
            .commit_revision(next_revision);
        Ok(())
    }

    pub fn accounting_snapshot(&self) -> Result<OptimisticAccountingSnapshot, OptimisticError> {
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
            let mut local_positive_count = 0;
            let mut local_anti_count = 0;
            let mut local_replay_count = 0;
            let mut local_minimum = None;
            let mut frontiers = BTreeMap::new();
            for (&lp_id, process) in &self.processes {
                local_positive_count += process.positives.len();
                local_anti_count += process.antis.len();
                local_replay_count += process.replay_pending.len();
                for message in process
                    .positives
                    .all()
                    .into_iter()
                    .chain(process.antis.all())
                {
                    local_minimum =
                        Some(local_minimum.map_or(message.event().tick, |tick: Tick| {
                            tick.min(message.event().tick)
                        }));
                }
                frontiers.insert(lp_id, process.local_time());
            }
            let outbox = &owned.routing().outbox;
            let outbound_minimum = outbox
                .values()
                .map(|record| record.message.event().tick)
                .min();
            let minimum_obligation_tick = local_minimum.into_iter().chain(outbound_minimum).min();
            Ok(OptimisticAccountingSnapshot {
                revision: owned.revision(),
                local_positive_count,
                local_anti_count,
                local_replay_count,
                ready_positive_count: outbox.len(),
                ready_anti_count: 0,
                blocked_count: 0,
                retirement_count: 0,
                reserved_receipt_count: outbox.len(),
                retained_receipt_count: owned.routing().completed.len()
                    + owned.routing().admissions.len(),
                local_minimum,
                outbound_minimum,
                minimum_obligation_tick,
                frontiers,
            })
        })
    }

    pub(super) fn schedule_owned_root(
        &mut self,
        stable_sequence: u64,
        event: super::RemoteEvent,
    ) -> Result<OptimisticMessage, OptimisticError> {
        self.ensure_healthy()?;
        if !self.initial_open {
            return Err(OptimisticError::InitialSchedulingClosed);
        }
        let owned = self
            .owned
            .as_ref()
            .ok_or(OptimisticError::OwnedModeRequired)?;
        let peers = owned.peers();
        with_live_authorities(&peers, || {
            self.schedule_owned_root_locked(stable_sequence, event)
        })
    }

    fn schedule_owned_root_locked(
        &mut self,
        stable_sequence: u64,
        event: super::RemoteEvent,
    ) -> Result<OptimisticMessage, OptimisticError> {
        let logical_id = LogicalEventId::root(event.source_lp, stable_sequence);
        let cohort = RootCohortId {
            source_lp: event.source_lp,
            logical_id: logical_id.clone(),
        };
        let (remote, next_revision, issuer, authority) = {
            let owned = self.owned.as_ref().expect("owned mode checked");
            if !owned.peers_sealed() {
                return Err(OptimisticError::NativePeersNotSealed);
            }
            if !owned.is_local_lp(event.source_lp) {
                return Err(OptimisticError::UnownedLogicalProcess(event.source_lp));
            }
            self.validate_owned_initial_event(&event)?;
            let authority = owned
                .own_authority_ref()
                .current_authorities()
                .get(&event.source_lp)
                .copied()
                .ok_or(OptimisticError::AuthorityModeMismatch(event.source_lp))?;
            if !matches!(authority, OptimisticAuthority::Scoped { .. }) {
                return Err(OptimisticError::AuthorityModeMismatch(event.source_lp));
            }
            if owned.routing().roots.contains(&cohort) {
                return Err(OptimisticError::ConflictingLogicalEvent {
                    source_lp: event.source_lp,
                });
            }
            if owned.routing().roots.len() >= owned.max_transition_entries() {
                return Err(OptimisticError::TransitionLimitExceeded {
                    limit: owned.max_transition_entries(),
                });
            }
            let remote = !owned.is_local_lp(event.dest_lp);
            if remote && owned.routing().outbox.len() >= owned.max_outbox_entries() {
                return Err(OptimisticError::OutboxLimitExceeded {
                    limit: owned.max_outbox_entries(),
                });
            }
            if remote {
                let receipt_count = owned.routing().outbox.len()
                    + owned.routing().completed.len()
                    + owned.routing().admissions.len();
                if receipt_count >= owned.max_receipt_entries() {
                    return Err(OptimisticError::ReceiptLimitExceeded {
                        limit: owned.max_receipt_entries(),
                    });
                }
            }
            (
                remote,
                owned.next_revision_value()?,
                owned.own_authority(),
                authority,
            )
        };
        if remote {
            // Remote roots reserve no local queue slot or destination token.
        } else {
            self.ensure_pending_capacity(1)?;
        }
        let next_epoch = if remote {
            None
        } else {
            Some(self.next_epoch(event.dest_lp)?)
        };
        let current_incarnation = self
            .next_incarnation
            .get(&event.source_lp)
            .copied()
            .flatten()
            .ok_or(OptimisticError::IncarnationExhausted(event.source_lp))?;
        let message = OptimisticMessage::new_with_authority(
            event.clone(),
            logical_id,
            authority,
            current_incarnation,
            OptimisticMessageKind::Positive,
        );
        OptimisticEventOrderKey::try_from_parts(
            event.tick,
            event.source_lp,
            message.logical_id().clone(),
        )?;
        let identity = DeliveryIdentity::from(&message);
        if let Some(existing) = self.known_deliveries.get(&identity) {
            return Err(if existing == &event {
                OptimisticError::DuplicatePositive {
                    source_lp: event.source_lp,
                    incarnation: current_incarnation,
                }
            } else {
                OptimisticError::ConflictingDelivery {
                    source_lp: event.source_lp,
                    incarnation: current_incarnation,
                }
            });
        }
        let next_incarnation = current_incarnation.checked_add(1);
        *self
            .next_incarnation
            .get_mut(&event.source_lp)
            .expect("source owned") = next_incarnation;
        self.known_deliveries.insert(identity, event.clone());
        self.owned
            .as_mut()
            .expect("owned mode checked")
            .routing_mut()
            .roots
            .insert(cohort);
        if let Some(epoch) = next_epoch {
            let state = self
                .processes
                .get_mut(&event.dest_lp)
                .expect("local destination validated");
            state.epoch = epoch;
            state.positives.insert(message.clone());
        } else {
            let id = NativeSendId::from(&message);
            self.owned
                .as_mut()
                .expect("owned mode checked")
                .routing_mut()
                .outbox
                .insert(
                    id,
                    OptimisticOutboundRecord {
                        message: message.clone(),
                        issuer,
                    },
                );
        }
        self.owned
            .as_mut()
            .expect("owned mode checked")
            .commit_revision(next_revision);
        Ok(message)
    }
}
