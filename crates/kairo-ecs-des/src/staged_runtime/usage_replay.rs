use super::{
    occupancy_window, RequestRecord, ResourceRecord, ScheduledAction, StagedActorId,
    StagedRequestId, StagedResourceId, StagedResourceRuntime, StagedRuntimeError,
};
use crate::usage::{CommittedUsageWindow, StagedUsageSource};
use kairo_ecs_core::{Scheduler, SchedulerCheckpoint};
use kairo_ecs_pdes::{CommittedProcessSnapshot, LpId};
use kairo_ecs_types::{EventId, SimTime, Versioned};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

/// Immutable staged allocation state for an in-process optimistic checkpoint.
///
/// Include this in the process snapshot with any model state that holds staged
/// handles. Handles from an abandoned speculative branch must stay within it.
#[derive(Clone)]
pub struct StagedResourceCheckpoint(Arc<Checkpoint>);

impl AsRef<StagedResourceCheckpoint> for StagedResourceCheckpoint {
    fn as_ref(&self) -> &Self {
        self
    }
}

struct Checkpoint {
    runtime_id: usize,
    scheduler: Versioned<SchedulerCheckpoint>,
    usage_source: Option<StagedUsageSource>,
    usage_sealed_through: Option<SimTime>,
    next_actor: u64,
    next_resource: u64,
    next_request: u64,
    actors: BTreeMap<StagedActorId, ()>,
    resources: BTreeMap<StagedResourceId, ResourceRecord>,
    requests: BTreeMap<StagedRequestId, RequestRecord>,
    actions: HashMap<EventId, ScheduledAction>,
}

impl StagedResourceRuntime {
    /// Capture scheduler ordering, handles, allocation state and usage seals.
    pub fn checkpoint(&self) -> StagedResourceCheckpoint {
        StagedResourceCheckpoint(Arc::new(Checkpoint {
            runtime_id: self.runtime_id,
            scheduler: self.scheduler.checkpoint(),
            usage_source: self.usage_source.clone(),
            usage_sealed_through: self.usage_sealed_through,
            next_actor: self.next_actor,
            next_resource: self.next_resource,
            next_request: self.next_request,
            actors: self.actors.clone(),
            resources: self.resources.clone(),
            requests: self.requests.clone(),
            actions: self.actions.clone(),
        }))
    }

    /// Restore an immutable checkpoint created by this allocator.
    ///
    /// A checkpoint from another allocator returns `InvalidState`.
    pub fn restore(
        &mut self,
        checkpoint: &StagedResourceCheckpoint,
    ) -> Result<(), StagedRuntimeError> {
        let state = &checkpoint.0;
        if state.runtime_id != self.runtime_id {
            return Err(StagedRuntimeError::InvalidState);
        }
        let scheduler = Scheduler::from_checkpoint(&state.scheduler)
            .map_err(|_| StagedRuntimeError::InvalidState)?;
        *self = Self {
            runtime_id: state.runtime_id,
            scheduler,
            usage_source: state.usage_source.clone(),
            usage_sealed_through: state.usage_sealed_through,
            next_actor: state.next_actor,
            next_resource: state.next_resource,
            next_request: state.next_request,
            actors: state.actors.clone(),
            resources: state.resources.clone(),
            requests: state.requests.clone(),
            actions: state.actions.clone(),
        };
        Ok(())
    }

    /// Close allocator time through a model tick, including an idle interval.
    ///
    /// Dispatch every action due through `end` first. A successful seal
    /// prevents new submissions and releases at or before it. In optimistic
    /// models, establish the seal in an event and keep it in the checkpoint.
    pub fn seal_usage_through(&mut self, end: SimTime) -> Result<(), StagedRuntimeError> {
        if self.next_event_time().is_some_and(|tick| tick <= end) {
            return Err(StagedRuntimeError::InvalidUsageWindow);
        }
        self.record_usage_seal(end);
        Ok(())
    }
}

/// Staged occupancy with the producing logical process and exclusive cutoff.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommittedOptimisticUsageWindow {
    pub lp_id: LpId,
    pub exclusive_cutoff: SimTime,
    pub window: CommittedUsageWindow,
}

/// Export from a committed optimistic checkpoint with a sealed allocator window.
///
/// The endpoint must be strictly before the cutoff so that any release at that
/// tick is committed as well. The model snapshot may contain additional state
/// and expose its own allocator checkpoint through `AsRef`.
pub fn committed_staged_usage<S: AsRef<StagedResourceCheckpoint>>(
    snapshot: &CommittedProcessSnapshot<S>,
    resource: StagedResourceId,
    start: SimTime,
    end: SimTime,
) -> Result<CommittedOptimisticUsageWindow, StagedRuntimeError> {
    let exclusive_cutoff = snapshot.exclusive_cutoff();
    let state = &snapshot.state().as_ref().0;
    if start >= end
        || end >= exclusive_cutoff
        || state.usage_sealed_through.is_none_or(|seal| end > seal)
    {
        return Err(StagedRuntimeError::InvalidUsageWindow);
    }
    let record = state
        .resources
        .get(&resource)
        .ok_or(StagedRuntimeError::UnknownResource)?;
    let window = occupancy_window(
        resource,
        record,
        &state.requests,
        state.usage_source.as_ref(),
        start,
        end,
    )?;
    Ok(CommittedOptimisticUsageWindow {
        lp_id: snapshot.lp_id(),
        exclusive_cutoff,
        window,
    })
}
