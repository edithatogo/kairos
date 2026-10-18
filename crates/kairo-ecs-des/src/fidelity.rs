//! Experimental C2 fidelity policy and admission adapter over actual Flow work.
//!
//! Execution fidelity is independent of observed replay policy. This adapter
//! neither changes work/resources nor samples service/transit randomness. The
//! public Rust surface is a preview, not a stable API. Its versioned checkpoint
//! DTO and bounded wire transport are experimental. Bindings and policy-boundary
//! checks cover only work admitted through this adapter. Decision lookups are
//! adapter-local; unknown or despawned work fails closed, and admission permits
//! cannot be cloned or serialized.

use super::{FlowRuntime, FlowRuntimeIdentity, WorkId, WorkState};
use kairo_ecs_types::EntityId;
use std::collections::BTreeMap;
use thiserror::Error;

mod checkpoint_wire;
pub use checkpoint_wire::{FidelityCheckpointWireError, FidelityCheckpointWireLimits};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FidelityMode {
    Macro,
    Micro,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FidelityScope {
    EntitySubsystem,
    Entity,
    Subsystem,
    Global,
}

#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum FidelityError {
    #[error("unsupported fidelity policy version {0}")]
    UnsupportedVersion(u32),
    #[error("invalid subsystem identity")]
    InvalidSubsystem,
    #[error("no configured fidelity policy applies")]
    MissingPolicy,
    #[error("unknown or inadmissible Flow work")]
    InvalidWork,
    #[error("work already has an immutable admission decision")]
    DuplicateAdmission,
    #[error("fidelity boundary has pending, active or suspended work")]
    BusyBoundary,
    #[error("no pending fidelity policy")]
    NoPendingPolicy,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FidelityDecision {
    pub mode: FidelityMode,
    pub scope: FidelityScope,
    pub policy_version: u32,
}

/// Configuration retains exact subsystem byte identity and generational actors.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FidelityPolicy {
    version: u32,
    global: Option<FidelityMode>,
    entity: BTreeMap<EntityId, FidelityMode>,
    subsystem: BTreeMap<String, FidelityMode>,
    entity_subsystem: BTreeMap<(EntityId, String), FidelityMode>,
}

fn validate_subsystem(id: &str) -> Result<(), FidelityError> {
    if id.is_empty() || id.len() > 1024 || id.trim() != id || id.chars().any(char::is_control) {
        Err(FidelityError::InvalidSubsystem)
    } else {
        Ok(())
    }
}

impl FidelityPolicy {
    pub fn new(version: u32, global: Option<FidelityMode>) -> Result<Self, FidelityError> {
        if version != 1 {
            return Err(FidelityError::UnsupportedVersion(version));
        }
        Ok(Self {
            version,
            global,
            entity: BTreeMap::new(),
            subsystem: BTreeMap::new(),
            entity_subsystem: BTreeMap::new(),
        })
    }

    pub fn set_entity(
        &mut self,
        entity: EntityId,
        mode: FidelityMode,
    ) -> Result<(), FidelityError> {
        self.entity.insert(entity, mode);
        Ok(())
    }

    pub fn set_subsystem(
        &mut self,
        subsystem: &str,
        mode: FidelityMode,
    ) -> Result<(), FidelityError> {
        validate_subsystem(subsystem)?;
        self.subsystem.insert(subsystem.to_owned(), mode);
        Ok(())
    }

    pub fn set_entity_subsystem(
        &mut self,
        entity: EntityId,
        subsystem: &str,
        mode: FidelityMode,
    ) -> Result<(), FidelityError> {
        validate_subsystem(subsystem)?;
        self.entity_subsystem
            .insert((entity, subsystem.to_owned()), mode);
        Ok(())
    }

    pub fn resolve(
        &self,
        entity: EntityId,
        subsystem: &str,
    ) -> Result<FidelityDecision, FidelityError> {
        validate_subsystem(subsystem)?;
        let selected = self
            .entity_subsystem
            .get(&(entity, subsystem.to_owned()))
            .map(|mode| (*mode, FidelityScope::EntitySubsystem))
            .or_else(|| {
                self.entity
                    .get(&entity)
                    .map(|mode| (*mode, FidelityScope::Entity))
            })
            .or_else(|| {
                self.subsystem
                    .get(subsystem)
                    .map(|mode| (*mode, FidelityScope::Subsystem))
            })
            .or_else(|| self.global.map(|mode| (mode, FidelityScope::Global)))
            .ok_or(FidelityError::MissingPolicy)?;
        Ok(FidelityDecision {
            mode: selected.0,
            scope: selected.1,
            policy_version: self.version,
        })
    }
}

/// Native, doc-hidden checkpoint policy value. This is an owned in-process DTO,
/// not a byte format or compatibility promise.
#[doc(hidden)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FidelityPolicyCheckpointV1 {
    pub version: u32,
    pub global: Option<FidelityMode>,
    pub entity_overrides: Vec<(EntityId, FidelityMode)>,
    pub subsystem_overrides: Vec<(String, FidelityMode)>,
    pub entity_subsystem_overrides: Vec<(EntityId, String, FidelityMode)>,
}

/// Native, doc-hidden checkpoint value for an adapter's exact admitted state.
#[doc(hidden)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FidelityAdapterCheckpointV1 {
    pub version: u32,
    pub current: FidelityPolicyCheckpointV1,
    pub pending: Option<FidelityPolicyCheckpointV1>,
    pub admitted: Vec<(WorkId, FidelityDecision)>,
    pub bound_runtime: bool,
}

/// Caller supplied bounds applied before checkpoint cloning or reconstruction.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FidelityCheckpointLimits {
    pub max_admitted: usize,
    pub max_overrides: usize,
    pub max_subsystem_bytes: usize,
}

/// Errors belonging only to the experimental native fidelity checkpoint seam.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum FidelityCheckpointError {
    #[error("unsupported fidelity checkpoint version {0}")]
    UnsupportedVersion(u32),
    #[error("fidelity checkpoint limits exceeded")]
    LimitExceeded,
    #[error("fidelity checkpoint policy data is invalid")]
    InvalidPolicy,
    #[error("fidelity checkpoint records are not in canonical order")]
    NonCanonical,
    #[error("fidelity checkpoint contains an invalid decision")]
    InvalidDecision,
    #[error("fidelity checkpoint source runtime does not match")]
    WrongRuntime,
    #[error("fidelity checkpoint work is missing or invalid")]
    InvalidWork,
    #[error("fidelity checkpoint work mapping is missing")]
    MissingMapping,
    #[error("fidelity checkpoint work mapping contains a duplicate")]
    DuplicateMapping,
    #[error("fidelity checkpoint work mapping contains an extra entry")]
    UnexpectedMapping,
}

fn checkpoint_policy_limits(
    current: &FidelityPolicyCheckpointV1,
    pending: Option<&FidelityPolicyCheckpointV1>,
    limits: FidelityCheckpointLimits,
) -> Result<(), FidelityCheckpointError> {
    let mut overrides = 0_usize;
    let mut bytes = 0_usize;
    for policy in std::iter::once(current).chain(pending) {
        if policy.version != 1 {
            return Err(FidelityCheckpointError::UnsupportedVersion(policy.version));
        }
        overrides = overrides
            .checked_add(policy.entity_overrides.len())
            .and_then(|n| n.checked_add(policy.subsystem_overrides.len()))
            .and_then(|n| n.checked_add(policy.entity_subsystem_overrides.len()))
            .ok_or(FidelityCheckpointError::LimitExceeded)?;
        for (subsystem, _) in &policy.subsystem_overrides {
            bytes = bytes
                .checked_add(subsystem.len())
                .ok_or(FidelityCheckpointError::LimitExceeded)?;
        }
        for (_, subsystem, _) in &policy.entity_subsystem_overrides {
            bytes = bytes
                .checked_add(subsystem.len())
                .ok_or(FidelityCheckpointError::LimitExceeded)?;
        }
    }
    if overrides > limits.max_overrides || bytes > limits.max_subsystem_bytes {
        return Err(FidelityCheckpointError::LimitExceeded);
    }
    Ok(())
}

fn validate_policy_checkpoint(
    policy: &FidelityPolicyCheckpointV1,
) -> Result<(), FidelityCheckpointError> {
    if policy.version != 1 {
        return Err(FidelityCheckpointError::UnsupportedVersion(policy.version));
    }
    if policy
        .entity_overrides
        .windows(2)
        .any(|pair| pair[0].0 >= pair[1].0)
        || policy
            .subsystem_overrides
            .windows(2)
            .any(|pair| pair[0].0 >= pair[1].0)
        || policy
            .entity_subsystem_overrides
            .windows(2)
            .any(|pair| (pair[0].0, pair[0].1.as_str()) >= (pair[1].0, pair[1].1.as_str()))
    {
        return Err(FidelityCheckpointError::NonCanonical);
    }
    for (subsystem, _) in &policy.subsystem_overrides {
        validate_subsystem(subsystem).map_err(|_| FidelityCheckpointError::InvalidPolicy)?;
    }
    for (_, subsystem, _) in &policy.entity_subsystem_overrides {
        validate_subsystem(subsystem).map_err(|_| FidelityCheckpointError::InvalidPolicy)?;
    }
    Ok(())
}

fn policy_checkpoint(policy: &FidelityPolicy) -> FidelityPolicyCheckpointV1 {
    FidelityPolicyCheckpointV1 {
        version: policy.version,
        global: policy.global,
        entity_overrides: policy
            .entity
            .iter()
            .map(|(key, value)| (*key, *value))
            .collect(),
        subsystem_overrides: policy
            .subsystem
            .iter()
            .map(|(key, value)| (key.clone(), *value))
            .collect(),
        entity_subsystem_overrides: policy
            .entity_subsystem
            .iter()
            .map(|((entity, subsystem), value)| (*entity, subsystem.clone(), *value))
            .collect(),
    }
}

fn policy_from_checkpoint(checkpoint: FidelityPolicyCheckpointV1) -> FidelityPolicy {
    FidelityPolicy {
        version: checkpoint.version,
        global: checkpoint.global,
        entity: checkpoint.entity_overrides.into_iter().collect(),
        subsystem: checkpoint.subsystem_overrides.into_iter().collect(),
        entity_subsystem: checkpoint
            .entity_subsystem_overrides
            .into_iter()
            .map(|(entity, subsystem, mode)| ((entity, subsystem), mode))
            .collect(),
    }
}

fn live_policy_limits(
    current: &FidelityPolicy,
    pending: Option<&FidelityPolicy>,
    limits: FidelityCheckpointLimits,
) -> Result<(), FidelityCheckpointError> {
    let mut overrides = 0_usize;
    let mut bytes = 0_usize;
    for policy in std::iter::once(current).chain(pending) {
        if policy.version != 1 {
            return Err(FidelityCheckpointError::UnsupportedVersion(policy.version));
        }
        overrides = overrides
            .checked_add(policy.entity.len())
            .and_then(|n| n.checked_add(policy.subsystem.len()))
            .and_then(|n| n.checked_add(policy.entity_subsystem.len()))
            .ok_or(FidelityCheckpointError::LimitExceeded)?;
        for subsystem in policy.subsystem.keys() {
            bytes = bytes
                .checked_add(subsystem.len())
                .ok_or(FidelityCheckpointError::LimitExceeded)?;
        }
        for (_, subsystem) in policy.entity_subsystem.keys() {
            bytes = bytes
                .checked_add(subsystem.len())
                .ok_or(FidelityCheckpointError::LimitExceeded)?;
        }
    }
    if overrides > limits.max_overrides || bytes > limits.max_subsystem_bytes {
        return Err(FidelityCheckpointError::LimitExceeded);
    }
    Ok(())
}

/// Live in-process decisions with an experimental versioned checkpoint surface.
///
/// Keeping all bindings here prevents a caller from omitting active/suspended
/// work when requesting the global policy boundary. Unknown/despawned bindings
/// deliberately fail closed pending an explicit cleanup contract.
#[derive(Debug, Eq, PartialEq)]
#[cfg_attr(test, derive(Clone))] // State snapshots for tests; production adapters cannot fork.
pub struct FidelityAdapter {
    policy: FidelityPolicy,
    pending: Option<FidelityPolicy>,
    admitted: BTreeMap<WorkId, FidelityDecision>,
    bound_runtime: Option<super::FlowRuntimeIdentity>,
}

impl FidelityAdapter {
    pub fn new(policy: FidelityPolicy) -> Self {
        Self {
            policy,
            pending: None,
            admitted: BTreeMap::new(),
            bound_runtime: None,
        }
    }

    /// Capture exact policy and admitted decisions for the matching live Flow.
    ///
    /// This native DTO deliberately omits runtime identity and is not a byte
    /// codec. Work state is inspected only for reference validity; pending,
    /// active, suspended and terminal work retain their original decision.
    #[doc(hidden)]
    pub fn checkpoint(
        &self,
        flow: &FlowRuntime,
        limits: FidelityCheckpointLimits,
    ) -> Result<FidelityAdapterCheckpointV1, FidelityCheckpointError> {
        self.check_runtime(flow)
            .map_err(|_| FidelityCheckpointError::WrongRuntime)?;
        if self.admitted.len() > limits.max_admitted {
            return Err(FidelityCheckpointError::LimitExceeded);
        }
        live_policy_limits(&self.policy, self.pending.as_ref(), limits)?;
        for (work, decision) in &self.admitted {
            if decision.policy_version != 1 {
                return Err(FidelityCheckpointError::InvalidDecision);
            }
            flow.work(*work)
                .and_then(|_| flow.work_progress(*work))
                .map_err(|_| FidelityCheckpointError::InvalidWork)?;
        }
        let current = policy_checkpoint(&self.policy);
        let pending = self.pending.as_ref().map(policy_checkpoint);
        Ok(FidelityAdapterCheckpointV1 {
            version: 1,
            current,
            pending,
            admitted: self
                .admitted
                .iter()
                .map(|(work, decision)| (*work, *decision))
                .collect(),
            bound_runtime: self.bound_runtime.is_some(),
        })
    }

    /// Rebind a complete checkpoint onto a fresh runtime using caller-supplied
    /// old-to-new WorkId pairs. Target work is validated without changing it.
    #[doc(hidden)]
    pub fn from_checkpoint(
        checkpoint: FidelityAdapterCheckpointV1,
        flow: &FlowRuntime,
        work_mapping: &[(WorkId, WorkId)],
        limits: FidelityCheckpointLimits,
    ) -> Result<Self, FidelityCheckpointError> {
        if checkpoint.version != 1 {
            return Err(FidelityCheckpointError::UnsupportedVersion(
                checkpoint.version,
            ));
        }
        if checkpoint.admitted.len() > limits.max_admitted {
            return Err(FidelityCheckpointError::LimitExceeded);
        }
        checkpoint_policy_limits(&checkpoint.current, checkpoint.pending.as_ref(), limits)?;
        validate_policy_checkpoint(&checkpoint.current)?;
        if let Some(pending) = checkpoint.pending.as_ref() {
            validate_policy_checkpoint(pending)?;
        }
        if checkpoint
            .admitted
            .windows(2)
            .any(|pair| pair[0].0 >= pair[1].0)
        {
            return Err(FidelityCheckpointError::NonCanonical);
        }
        if checkpoint
            .admitted
            .iter()
            .any(|(_, decision)| decision.policy_version != 1)
        {
            return Err(FidelityCheckpointError::InvalidDecision);
        }
        // The live adapter binds only when an admission is committed and has
        // no admission-removal operation. An empty bound image, or admitted
        // records without a binding, cannot be produced by a valid adapter.
        if checkpoint.bound_runtime != !checkpoint.admitted.is_empty() {
            return Err(FidelityCheckpointError::InvalidWork);
        }
        if work_mapping.len() < checkpoint.admitted.len() {
            return Err(FidelityCheckpointError::MissingMapping);
        }
        if work_mapping.len() > checkpoint.admitted.len() {
            return Err(FidelityCheckpointError::UnexpectedMapping);
        }

        let mut seen_old = std::collections::BTreeSet::new();
        let mut seen_new = std::collections::BTreeSet::new();
        let mut rebound_work = BTreeMap::new();
        for (old, new) in work_mapping {
            if !seen_old.insert(*old) || !seen_new.insert(*new) {
                return Err(FidelityCheckpointError::DuplicateMapping);
            }
            if checkpoint
                .admitted
                .binary_search_by_key(old, |(work, _)| *work)
                .is_err()
            {
                return Err(FidelityCheckpointError::UnexpectedMapping);
            }
            // Flow restoration preserves generational EntityIds. Rebinding
            // changes only the process-local runtime identity; remapping an
            // admission to a different actor/work could attach its frozen
            // decision to unrelated state.
            if old.entity_id() != new.entity_id() {
                return Err(FidelityCheckpointError::InvalidWork);
            }
            flow.work(*new)
                .and_then(|_| flow.work_progress(*new))
                .map_err(|_| FidelityCheckpointError::InvalidWork)?;
            rebound_work.insert(*old, *new);
        }
        if checkpoint
            .admitted
            .iter()
            .any(|(old, _)| !seen_old.contains(old))
        {
            return Err(FidelityCheckpointError::MissingMapping);
        }

        // All schema, count, reference and mapping checks precede destination
        // construction. Decisions are copied exactly, never re-resolved.
        let current = policy_from_checkpoint(checkpoint.current);
        let pending = checkpoint.pending.map(policy_from_checkpoint);
        let admitted = checkpoint
            .admitted
            .into_iter()
            .map(|(old, decision)| {
                let new = rebound_work
                    .get(&old)
                    .expect("complete mapping validated above");
                (*new, decision)
            })
            .collect();
        Ok(Self {
            policy: current,
            pending,
            admitted,
            bound_runtime: checkpoint.bound_runtime.then(|| flow.identity()),
        })
    }

    pub fn admit(
        &mut self,
        flow: &FlowRuntime,
        work: WorkId,
        subsystem: &str,
    ) -> Result<FidelityDecision, FidelityError> {
        self.check_runtime(flow)?;
        if self.admitted.contains_key(&work) {
            return Err(FidelityError::DuplicateAdmission);
        }
        let spec = flow.work(work).map_err(|_| FidelityError::InvalidWork)?;
        let progress = flow
            .work_progress(work)
            .map_err(|_| FidelityError::InvalidWork)?;
        if progress.state != WorkState::Pending {
            return Err(FidelityError::InvalidWork);
        }
        let decision = self.policy.resolve(spec.owner, subsystem)?;
        if self.bound_runtime.is_none() {
            self.bound_runtime = Some(flow.identity());
        }
        self.admitted.insert(work, decision);
        Ok(decision)
    }

    /// Freeze the resolved policy while actual work is being created.
    ///
    /// The mutable borrow prevents staging or applying another policy between
    /// resolution and binding. Runtime lineage is committed only by a
    /// successful `bind`, so a failed create/bind can be retried safely.
    pub fn prepare_admission<'a>(
        &'a mut self,
        flow: &FlowRuntime,
        owner: EntityId,
        subsystem: &str,
    ) -> Result<FidelityAdmissionPermit<'a>, FidelityError> {
        self.check_runtime(flow)?;
        flow.validate_actor(owner)
            .map_err(|_| FidelityError::InvalidWork)?;
        let decision = self.policy.resolve(owner, subsystem)?;
        Ok(FidelityAdmissionPermit {
            adapter: self,
            runtime: flow.identity(),
            owner,
            decision,
        })
    }

    /// Adapter-local lookup, valid within the runtime bound by admission.
    /// This signature cannot attest the origin of a colliding foreign WorkId.
    pub fn decision(&self, work: WorkId) -> Option<&FidelityDecision> {
        self.admitted.get(&work)
    }

    fn check_runtime(&self, flow: &FlowRuntime) -> Result<(), FidelityError> {
        if self
            .bound_runtime
            .as_ref()
            .is_some_and(|identity| *identity != flow.identity())
        {
            return Err(FidelityError::InvalidWork);
        }
        Ok(())
    }

    pub fn stage_policy(&mut self, policy: FidelityPolicy) {
        self.pending = Some(policy);
    }

    pub fn apply_at_boundary(&mut self, flow: &FlowRuntime) -> Result<(), FidelityError> {
        if self.pending.is_none() {
            return Err(FidelityError::NoPendingPolicy);
        }
        self.check_runtime(flow)?;
        for work in self.admitted.keys() {
            let progress = flow
                .work_progress(*work)
                .map_err(|_| FidelityError::InvalidWork)?;
            if matches!(
                progress.state,
                WorkState::Pending | WorkState::Active | WorkState::Suspended
            ) {
                return Err(FidelityError::BusyBoundary);
            }
        }
        // Every failure above precedes mutation; applying a policy cannot edit
        // any admitted decision or its Flow work/progress/context.
        self.policy = self.pending.take().expect("pending policy checked above");
        Ok(())
    }
}

/// Non-cloneable proof that one actual task is bound to its frozen decision.
/// This remains in-memory state; it is never serialized with a checkpoint.
#[derive(Debug)]
pub struct FidelityAdmissionPermit<'a> {
    adapter: &'a mut FidelityAdapter,
    runtime: FlowRuntimeIdentity,
    owner: EntityId,
    decision: FidelityDecision,
}

impl FidelityAdmissionPermit<'_> {
    pub fn decision(&self) -> FidelityDecision {
        self.decision
    }

    pub fn bind(
        self,
        flow: &FlowRuntime,
        work: WorkId,
        expected: kairo_ecs_types::SimDuration,
    ) -> Result<FidelityDecision, (Self, FidelityError)> {
        let validation = (|| {
            if self.runtime != flow.identity() {
                return Err(FidelityError::InvalidWork);
            }
            self.adapter.check_runtime(flow)?;
            flow.validate_actor(self.owner)
                .map_err(|_| FidelityError::InvalidWork)?;
            if self.adapter.admitted.contains_key(&work) {
                return Err(FidelityError::DuplicateAdmission);
            }
            let spec = flow.work(work).map_err(|_| FidelityError::InvalidWork)?;
            let progress = flow
                .work_progress(work)
                .map_err(|_| FidelityError::InvalidWork)?;
            if spec.owner != self.owner
                || spec.original_duration != expected
                || progress.state != WorkState::Pending
            {
                return Err(FidelityError::InvalidWork);
            }
            Ok(())
        })();
        if let Err(error) = validation {
            return Err((self, error));
        }

        if self.adapter.bound_runtime.is_none() {
            self.adapter.bound_runtime = Some(self.runtime.clone());
        }
        self.adapter.admitted.insert(work, self.decision);
        Ok(self.decision)
    }
}

#[cfg(test)]
#[path = "fidelity_tests.rs"]
mod fidelity_tests;
