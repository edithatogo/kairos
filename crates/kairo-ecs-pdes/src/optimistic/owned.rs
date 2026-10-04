use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock, RwLockReadGuard, Weak};

use super::owned_routing::OwnedRootRoutingState;
use super::{LpId, OptimisticAuthority, OptimisticError, OptimisticLimits, PartitionPlan};

/// Bounded configuration for an owned optimistic-runtime issuer.
#[derive(Clone, Debug)]
pub struct OptimisticOwnedOptions {
    pub simulation_namespace: u128,
    pub current_authorities: BTreeMap<LpId, OptimisticAuthority>,
    pub emission_epochs: BTreeMap<LpId, u64>,
    pub local_limits: OptimisticLimits,
    pub max_global_lps: usize,
    pub max_outbox_entries: usize,
    pub max_transition_entries: usize,
    pub max_receipt_entries: usize,
}

#[derive(Clone, Debug)]
struct AuthorityConfiguration {
    runtime_id: u64,
    recovery_generation: u64,
    simulation_namespace: u128,
    owned_lps: Vec<LpId>,
    global_partition: PartitionPlan,
    global_topology: BTreeMap<LpId, Vec<LpId>>,
    current_authorities: BTreeMap<LpId, OptimisticAuthority>,
    emission_epochs: BTreeMap<LpId, u64>,
}

/// Opaque, process-local proof of one native runtime issuer.
///
/// This capability does not itself keep the issuer alive.
pub struct NativeAccountingAuthority {
    config: Arc<AuthorityConfiguration>,
    live: Weak<NativeAdmissionGate>,
}

pub(super) struct NativeAdmissionGate {
    active: AtomicBool,
    lock: RwLock<()>,
}

impl Clone for NativeAccountingAuthority {
    fn clone(&self) -> Self {
        Self {
            config: Arc::clone(&self.config),
            live: self.live.clone(),
        }
    }
}

impl fmt::Debug for NativeAccountingAuthority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NativeAccountingAuthority")
            .field("runtime_id", &self.runtime_id())
            .field("recovery_generation", &self.recovery_generation())
            .field("simulation_namespace", &self.simulation_namespace())
            .field("owned_lps", &self.owned_lps())
            .field("live", &self.is_live())
            .finish()
    }
}

impl NativeAccountingAuthority {
    pub fn runtime_id(&self) -> u64 {
        self.config.runtime_id
    }

    pub fn recovery_generation(&self) -> u64 {
        self.config.recovery_generation
    }

    pub fn simulation_namespace(&self) -> u128 {
        self.config.simulation_namespace
    }

    pub fn owned_lps(&self) -> &[LpId] {
        &self.config.owned_lps
    }

    pub fn is_live(&self) -> bool {
        self.live
            .upgrade()
            .is_some_and(|gate| gate.active.load(Ordering::Acquire))
    }

    pub fn global_partition(&self) -> &PartitionPlan {
        &self.config.global_partition
    }

    pub fn global_topology(&self) -> &BTreeMap<LpId, Vec<LpId>> {
        &self.config.global_topology
    }

    pub fn current_authorities(&self) -> &BTreeMap<LpId, OptimisticAuthority> {
        &self.config.current_authorities
    }

    pub fn emission_epochs(&self) -> &BTreeMap<LpId, u64> {
        &self.config.emission_epochs
    }

    pub(super) fn same_issuer(&self, other: &Self) -> bool {
        self.runtime_id() == other.runtime_id()
            && self.recovery_generation() == other.recovery_generation()
            && Weak::ptr_eq(&self.live, &other.live)
    }

    fn same_global_configuration(&self, other: &Self) -> bool {
        self.simulation_namespace() == other.simulation_namespace()
            && self.global_partition() == other.global_partition()
            && self.global_topology() == other.global_topology()
            && self.current_authorities() == other.current_authorities()
    }
}

pub(super) struct OwnedRuntimeState {
    own_authority: NativeAccountingAuthority,
    live_witness: Arc<NativeAdmissionGate>,
    peers: Vec<NativeAccountingAuthority>,
    max_global_lps: usize,
    max_outbox_entries: usize,
    max_transition_entries: usize,
    max_receipt_entries: usize,
    sealed: bool,
    initial_open: bool,
    revision: u64,
    routing: OwnedRootRoutingState,
}

pub(super) struct OwnedConstructionConfig {
    pub simulation_namespace: u128,
    pub owned_lps: Vec<LpId>,
    pub global_partition: PartitionPlan,
    pub global_topology: BTreeMap<LpId, Vec<LpId>>,
    pub current_authorities: BTreeMap<LpId, OptimisticAuthority>,
    pub emission_epochs: BTreeMap<LpId, u64>,
}

impl OwnedRuntimeState {
    pub(super) fn new(
        runtime_id: u64,
        config: OwnedConstructionConfig,
        options: &OptimisticOwnedOptions,
    ) -> Self {
        let live_witness = Arc::new(NativeAdmissionGate {
            active: AtomicBool::new(true),
            lock: RwLock::new(()),
        });
        let own_authority = NativeAccountingAuthority {
            config: Arc::new(AuthorityConfiguration {
                runtime_id,
                recovery_generation: 0,
                simulation_namespace: config.simulation_namespace,
                owned_lps: config.owned_lps,
                global_partition: config.global_partition,
                global_topology: config.global_topology,
                current_authorities: config.current_authorities,
                emission_epochs: config.emission_epochs,
            }),
            live: Arc::downgrade(&live_witness),
        };
        Self {
            own_authority: own_authority.clone(),
            live_witness,
            peers: vec![own_authority],
            max_global_lps: options.max_global_lps,
            max_outbox_entries: options.max_outbox_entries,
            max_transition_entries: options.max_transition_entries,
            max_receipt_entries: options.max_receipt_entries,
            sealed: false,
            initial_open: true,
            revision: 0,
            routing: OwnedRootRoutingState::default(),
        }
    }

    pub(super) fn own_authority(&self) -> NativeAccountingAuthority {
        self.own_authority.clone()
    }

    pub(super) fn owned_lps(&self) -> &[LpId] {
        self.own_authority.owned_lps()
    }

    pub(super) fn revision(&self) -> u64 {
        self.revision
    }

    pub(super) fn peers_sealed(&self) -> bool {
        self.sealed
    }

    pub(super) fn validate_live_peers(&self) -> Result<(), OptimisticError> {
        for peer in &self.peers {
            if !peer.is_live() {
                return Err(OptimisticError::StaleNativeAccountingAuthority {
                    runtime_id: peer.runtime_id(),
                    recovery_generation: peer.recovery_generation(),
                });
            }
        }
        Ok(())
    }

    pub(super) fn require_sealed_live(&self) -> Result<(), OptimisticError> {
        self.validate_live_peers()?;
        if !self.sealed {
            return Err(OptimisticError::NativePeersNotSealed);
        }
        Ok(())
    }

    pub(super) fn register_peer(
        &mut self,
        peer: NativeAccountingAuthority,
    ) -> Result<(), OptimisticError> {
        let mut authorities = self.peers.clone();
        authorities.push(peer.clone());
        with_live_authorities(&authorities, || self.register_peer_locked(peer))
    }

    fn register_peer_locked(
        &mut self,
        peer: NativeAccountingAuthority,
    ) -> Result<(), OptimisticError> {
        if !self.own_authority.same_global_configuration(&peer) {
            return Err(OptimisticError::NativePeerConfigurationMismatch);
        }
        if self
            .peers
            .iter()
            .any(|existing| existing.same_issuer(&peer))
        {
            return Ok(());
        }
        if self.sealed {
            return Err(OptimisticError::NativePeerRegistrationClosed);
        }
        let overlap = self
            .peers
            .iter()
            .flat_map(|existing| existing.owned_lps())
            .filter(|lp| peer.owned_lps().binary_search(lp).is_ok())
            .copied()
            .min();
        if let Some(overlap) = overlap {
            return Err(OptimisticError::NativePeerOwnershipOverlap(overlap));
        }
        if self.peers.len() >= self.max_global_lps {
            return Err(OptimisticError::GlobalLpLimitExceeded {
                actual: self.peers.len() + 1,
                limit: self.max_global_lps,
            });
        }
        let next_revision = self.next_revision()?;
        self.peers.push(peer);
        self.peers.sort_by_key(|authority| authority.owned_lps()[0]);
        self.revision = next_revision;
        Ok(())
    }

    pub(super) fn seal_peers(&mut self) -> Result<(), OptimisticError> {
        let authorities = self.peers.clone();
        with_live_authorities(&authorities, || self.seal_peers_locked())
    }

    fn seal_peers_locked(&mut self) -> Result<(), OptimisticError> {
        if self.sealed {
            return Ok(());
        }
        let expected = self
            .own_authority
            .global_partition()
            .segments()
            .iter()
            .map(|segment| segment.id)
            .collect::<BTreeSet<_>>();
        let mut actual = BTreeSet::new();
        for peer in &self.peers {
            actual.extend(peer.owned_lps().iter().copied());
        }
        let missing = expected.difference(&actual).copied().collect::<Vec<_>>();
        if !missing.is_empty() {
            return Err(OptimisticError::NativePeerCoverageIncomplete { missing });
        }
        let next_revision = self.next_revision()?;
        self.sealed = true;
        self.revision = next_revision;
        Ok(())
    }

    pub(super) fn close_initial_inputs(
        &mut self,
        publish_runtime_closed: impl FnOnce(),
    ) -> Result<(), OptimisticError> {
        let authorities = self.peers.clone();
        with_live_authorities(&authorities, || {
            self.close_initial_inputs_locked(publish_runtime_closed)
        })
    }

    fn close_initial_inputs_locked(
        &mut self,
        publish_runtime_closed: impl FnOnce(),
    ) -> Result<(), OptimisticError> {
        if !self.sealed {
            return Err(OptimisticError::NativePeersNotSealed);
        }
        if !self.initial_open {
            publish_runtime_closed();
            return Ok(());
        }
        let next_revision = self.next_revision()?;
        self.initial_open = false;
        publish_runtime_closed();
        self.revision = next_revision;
        Ok(())
    }

    fn next_revision(&self) -> Result<u64, OptimisticError> {
        self.revision
            .checked_add(1)
            .ok_or(OptimisticError::AccountingRevisionExhausted)
    }

    pub(super) fn peers(&self) -> Vec<NativeAccountingAuthority> {
        self.peers.clone()
    }

    pub(super) fn next_revision_value(&self) -> Result<u64, OptimisticError> {
        self.next_revision()
    }

    pub(super) fn own_authority_ref(&self) -> &NativeAccountingAuthority {
        &self.own_authority
    }

    pub(super) fn is_local_lp(&self, lp_id: LpId) -> bool {
        self.owned_lps().binary_search(&lp_id).is_ok()
    }

    pub(super) fn max_outbox_entries(&self) -> usize {
        self.max_outbox_entries
    }

    pub(super) fn max_transition_entries(&self) -> usize {
        self.max_transition_entries
    }

    pub(super) fn max_receipt_entries(&self) -> usize {
        self.max_receipt_entries
    }

    pub(super) fn routing(&self) -> &OwnedRootRoutingState {
        &self.routing
    }

    pub(super) fn routing_mut(&mut self) -> &mut OwnedRootRoutingState {
        &mut self.routing
    }

    pub(super) fn commit_revision(&mut self, next: u64) {
        self.revision = next;
    }

    pub(super) fn invalidate_before_runtime_drop(&mut self) {
        match self.live_witness.lock.write() {
            Ok(guard) => {
                self.live_witness.active.store(false, Ordering::Release);
                drop(guard);
            }
            Err(poisoned) => {
                let guard = poisoned.into_inner();
                self.live_witness.active.store(false, Ordering::Release);
                drop(guard);
            }
        }
    }
}

impl Drop for OwnedRuntimeState {
    fn drop(&mut self) {
        self.invalidate_before_runtime_drop();
    }
}

pub(super) fn with_live_authorities<R>(
    authorities: &[NativeAccountingAuthority],
    action: impl FnOnce() -> Result<R, OptimisticError>,
) -> Result<R, OptimisticError> {
    let mut gates = Vec::with_capacity(authorities.len());
    for authority in authorities {
        let gate = authority
            .live
            .upgrade()
            .ok_or_else(|| stale_error(authority))?;
        gates.push((
            authority.runtime_id(),
            Arc::as_ptr(&gate) as usize,
            gate,
            authority,
        ));
    }
    gates.sort_by_key(|(runtime_id, pointer, _, _)| (*runtime_id, *pointer));
    gates.dedup_by(|left, right| Arc::ptr_eq(&left.2, &right.2));
    let mut guards: Vec<RwLockReadGuard<'_, ()>> = Vec::with_capacity(gates.len());
    for (_, _, gate, authority) in &gates {
        let guard = gate
            .lock
            .read()
            .map_err(|_| OptimisticError::NativeAccountingUnavailable {
                runtime_id: authority.runtime_id(),
            })?;
        if !gate.active.load(Ordering::Acquire) {
            return Err(stale_error(authority));
        }
        guards.push(guard);
    }
    let result = action();
    drop(guards);
    result
}

fn stale_error(authority: &NativeAccountingAuthority) -> OptimisticError {
    OptimisticError::StaleNativeAccountingAuthority {
        runtime_id: authority.runtime_id(),
        recovery_generation: authority.recovery_generation(),
    }
}
