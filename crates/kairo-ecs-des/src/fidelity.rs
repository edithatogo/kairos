//! Experimental private C2 admission adapter over actual Flow work.
//!
//! Execution fidelity is independent of observed replay policy. This adapter
//! neither changes work/resources nor samples service/transit randomness.

use super::{FlowRuntime, WorkId, WorkState};
use kairo_ecs_types::EntityId;
use std::collections::BTreeMap;
use thiserror::Error;

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

/// Live in-process decisions; no portable checkpoint/serialization promise.
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
