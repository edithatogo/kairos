//! Experimental, purpose-specific runtime for staged resource allocation.
//!
//! This is not a replacement for the removed general-purpose `FlowRuntime`.

use crate::usage::{
    valid_usage_label, CommittedUsageWindow, OccupancySlice, StagedUsageSource, UsageBoundary,
    STAGED_USAGE_VERSION,
};
use kairo_ecs_core::Scheduler;
use kairo_ecs_types::{EntityId, EventId, EventKind, ScheduleRequest, SimTime, StepOutcome};
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};

#[cfg(feature = "optimistic-usage")]
mod usage_replay;
#[cfg(feature = "optimistic-usage")]
pub use usage_replay::{
    committed_staged_usage, CommittedOptimisticUsageWindow, StagedResourceCheckpoint,
};

const SUBMIT_EVENT: u32 = 0x5354_4701;
const RELEASE_EVENT: u32 = 0x5354_4702;
const LEASE_REVISION: u64 = 1;
static NEXT_RUNTIME_ID: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct StagedActorId {
    runtime: usize,
    entity: EntityId,
}

impl fmt::Debug for StagedActorId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("StagedActorId").field(&self.entity).finish()
    }
}

impl StagedActorId {
    pub const fn entity_id(self) -> EntityId {
        self.entity
    }
}

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct StagedResourceId {
    runtime: usize,
    entity: EntityId,
}

impl fmt::Debug for StagedResourceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("StagedResourceId")
            .field(&self.entity)
            .finish()
    }
}

impl StagedResourceId {
    pub const fn entity_id(self) -> EntityId {
        self.entity
    }
}

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct StagedRequestId {
    runtime: usize,
    entity: EntityId,
}

impl fmt::Debug for StagedRequestId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("StagedRequestId")
            .field(&self.entity)
            .finish()
    }
}

impl StagedRequestId {
    pub const fn entity_id(self) -> EntityId {
        self.entity
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct StagedLeaseId {
    request: StagedRequestId,
    revision: u64,
}

impl StagedLeaseId {
    pub const fn request_id(self) -> StagedRequestId {
        self.request
    }
    pub const fn revision(self) -> u64 {
        self.revision
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagedRequestState {
    Scheduled,
    Queued,
    Active,
    /// Terminal after release or withdrawal, including requests never admitted.
    Released,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagedRuntimeError {
    IdExhausted,
    InvalidCapacity,
    UnknownActor,
    UnknownResource,
    UnknownRequest,
    InvalidState,
    InvalidReleaseTime,
    SubmissionInPast,
    RuntimeIdExhausted,
    InvalidQuantity,
    InvalidUsageSource,
    InvalidResourceUnit,
    MissingResourceUnit,
    MissingUsageSource,
    InvalidUsageWindow,
    UsageOverflow,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagedRequestSnapshot {
    pub actor: StagedActorId,
    pub resource: StagedResourceId,
    pub state: StagedRequestState,
    pub lease: Option<StagedLeaseId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagedResourceSnapshot {
    pub capacity: u64,
    pub available: u64,
    pub active: Vec<StagedLeaseId>,
    pub queued: Vec<StagedRequestId>,
}

#[derive(Clone, Copy, Debug)]
enum ScheduledAction {
    Submit(StagedRequestId),
    Close(StagedRequestId),
}

#[derive(Clone, Debug)]
struct RequestRecord {
    actor: StagedActorId,
    resource: StagedResourceId,
    state: StagedRequestState,
    lease: Option<StagedLeaseId>,
    pending_submit: Option<EventId>,
    pending_close: Option<(SimTime, EventId)>,
    quantity: u64,
    admission: Option<UsageBoundary>,
    release: Option<UsageBoundary>,
}

#[derive(Clone, Debug)]
struct ResourceRecord {
    unit: Option<String>,
    capacity: u64,
    available: u64,
    active: Vec<StagedLeaseId>,
    queued: VecDeque<StagedRequestId>,
}

/// Scheduler-backed experimental runtime for the CareOps staged allocation contract.
pub struct StagedResourceRuntime {
    runtime_id: usize,
    scheduler: Scheduler,
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
    pub fn new() -> Result<Self, StagedRuntimeError> {
        let runtime_id = NEXT_RUNTIME_ID
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |next| {
                next.checked_add(1)
            })
            .map_err(|_| StagedRuntimeError::RuntimeIdExhausted)?;
        Ok(Self {
            runtime_id,
            scheduler: Scheduler::new(),
            usage_source: None,
            usage_sealed_through: None,
            next_actor: 0,
            next_resource: 0,
            next_request: 0,
            actors: BTreeMap::new(),
            resources: BTreeMap::new(),
            requests: BTreeMap::new(),
            actions: HashMap::new(),
        })
    }

    /// Create a runtime with provenance bound for all committed usage exports.
    pub fn new_with_usage(source: StagedUsageSource) -> Result<Self, StagedRuntimeError> {
        source.validate()?;
        let mut runtime = Self::new()?;
        runtime.usage_source = Some(source);
        Ok(runtime)
    }

    pub fn now(&self) -> SimTime {
        self.scheduler.now()
    }

    /// Tick of the next live allocation action, without dispatching it.
    pub fn next_event_time(&mut self) -> Option<SimTime> {
        self.scheduler.peek_next().map(|event| event.at)
    }

    fn alloc(index: &mut u64) -> Result<EntityId, StagedRuntimeError> {
        let current = *index;
        *index = index
            .checked_add(1)
            .ok_or(StagedRuntimeError::IdExhausted)?;
        Ok(EntityId::new(current, 0))
    }

    pub fn spawn_actor(&mut self) -> Result<StagedActorId, StagedRuntimeError> {
        let id = StagedActorId {
            runtime: self.runtime_id,
            entity: Self::alloc(&mut self.next_actor)?,
        };
        self.actors.insert(id, ());
        Ok(id)
    }

    pub fn create_resource(
        &mut self,
        capacity: u64,
    ) -> Result<StagedResourceId, StagedRuntimeError> {
        self.create_resource_record(capacity, None)
    }

    /// Register a resource with its explicit, immutable usage unit.
    pub fn create_resource_with_unit(
        &mut self,
        capacity: u64,
        unit: impl Into<String>,
    ) -> Result<StagedResourceId, StagedRuntimeError> {
        let unit = unit.into();
        if !valid_usage_label(&unit) {
            return Err(StagedRuntimeError::InvalidResourceUnit);
        }
        self.create_resource_record(capacity, Some(unit))
    }

    fn create_resource_record(
        &mut self,
        capacity: u64,
        unit: Option<String>,
    ) -> Result<StagedResourceId, StagedRuntimeError> {
        if capacity == 0 {
            return Err(StagedRuntimeError::InvalidCapacity);
        }
        let id = StagedResourceId {
            runtime: self.runtime_id,
            entity: Self::alloc(&mut self.next_resource)?,
        };
        self.resources.insert(
            id,
            ResourceRecord {
                unit,
                capacity,
                available: capacity,
                active: Vec::new(),
                queued: VecDeque::new(),
            },
        );
        Ok(id)
    }

    pub fn submit(
        &mut self,
        resource: StagedResourceId,
        actor: StagedActorId,
        at: SimTime,
    ) -> Result<StagedRequestId, StagedRuntimeError> {
        self.submit_quantity(resource, actor, at, 1)
    }

    /// Submit an all-or-nothing FIFO request for a positive number of units.
    pub fn submit_quantity(
        &mut self,
        resource: StagedResourceId,
        actor: StagedActorId,
        at: SimTime,
        quantity: u64,
    ) -> Result<StagedRequestId, StagedRuntimeError> {
        if at < self.now() || self.usage_sealed_through.is_some_and(|end| at <= end) {
            return Err(StagedRuntimeError::SubmissionInPast);
        }
        if resource.runtime != self.runtime_id {
            return Err(StagedRuntimeError::UnknownResource);
        }
        if actor.runtime != self.runtime_id {
            return Err(StagedRuntimeError::UnknownActor);
        }
        let resource_record = self
            .resources
            .get(&resource)
            .ok_or(StagedRuntimeError::UnknownResource)?;
        if quantity == 0 || quantity > resource_record.capacity {
            return Err(StagedRuntimeError::InvalidQuantity);
        }
        if !self.actors.contains_key(&actor) {
            return Err(StagedRuntimeError::UnknownActor);
        }
        let request = StagedRequestId {
            runtime: self.runtime_id,
            entity: Self::alloc(&mut self.next_request)?,
        };
        let request_event = self.scheduler.schedule(ScheduleRequest {
            at,
            priority: 0,
            entity: Some(actor.entity),
            kind: EventKind::custom(SUBMIT_EVENT),
        });
        self.actions
            .insert(request_event, ScheduledAction::Submit(request));
        self.requests.insert(
            request,
            RequestRecord {
                actor,
                resource,
                state: StagedRequestState::Scheduled,
                lease: None,
                pending_submit: Some(request_event),
                pending_close: None,
                quantity,
                admission: None,
                release: None,
            },
        );
        Ok(request)
    }

    pub fn request(
        &self,
        request: StagedRequestId,
    ) -> Result<StagedRequestSnapshot, StagedRuntimeError> {
        if request.runtime != self.runtime_id {
            return Err(StagedRuntimeError::UnknownRequest);
        }
        let r = self
            .requests
            .get(&request)
            .ok_or(StagedRuntimeError::UnknownRequest)?;
        Ok(StagedRequestSnapshot {
            actor: r.actor,
            resource: r.resource,
            state: r.state,
            lease: r.lease,
        })
    }

    pub fn resource(
        &self,
        resource: StagedResourceId,
    ) -> Result<StagedResourceSnapshot, StagedRuntimeError> {
        if resource.runtime != self.runtime_id {
            return Err(StagedRuntimeError::UnknownResource);
        }
        let r = self
            .resources
            .get(&resource)
            .ok_or(StagedRuntimeError::UnknownResource)?;
        Ok(StagedResourceSnapshot {
            capacity: r.capacity,
            available: r.available,
            active: r.active.clone(),
            queued: r.queued.iter().copied().collect(),
        })
    }

    pub fn release(&mut self, lease: StagedLeaseId, at: SimTime) -> Result<(), StagedRuntimeError> {
        if lease.request.runtime != self.runtime_id {
            return Err(StagedRuntimeError::InvalidState);
        }
        if at < self.now() || self.usage_sealed_through.is_some_and(|end| at <= end) {
            return Err(StagedRuntimeError::InvalidReleaseTime);
        }
        let r = self
            .requests
            .get(&lease.request)
            .ok_or(StagedRuntimeError::UnknownRequest)?;
        if r.state != StagedRequestState::Active
            || r.lease != Some(lease)
            || r.pending_close.is_some()
        {
            return Err(StagedRuntimeError::InvalidState);
        }
        self.schedule_close(lease.request, at);
        Ok(())
    }

    /// Schedule withdrawal or release of a request in the allocator's native order.
    ///
    /// Returns false for an already closed request or an equal or earlier
    /// pending close. Otherwise, the action executes at `at` and replaces any
    /// later release. Requests withdrawn before admission produce no occupancy.
    pub fn close_request(
        &mut self,
        request: StagedRequestId,
        at: SimTime,
    ) -> Result<bool, StagedRuntimeError> {
        let record = self
            .requests
            .get(&request)
            .ok_or(StagedRuntimeError::UnknownRequest)?;
        if record.state == StagedRequestState::Released
            || record.pending_close.is_some_and(|(tick, _)| tick <= at)
        {
            return Ok(false);
        }
        if at < self.now() || self.usage_sealed_through.is_some_and(|end| at <= end) {
            return Err(StagedRuntimeError::InvalidReleaseTime);
        }
        self.schedule_close(request, at);
        Ok(true)
    }

    fn schedule_close(&mut self, request: StagedRequestId, at: SimTime) {
        let record = self.requests.get_mut(&request).expect("request validated");
        if let Some((_, event)) = record.pending_close {
            self.scheduler.cancel(event);
            self.actions.remove(&event);
        }
        let event = self.scheduler.schedule(ScheduleRequest {
            at,
            priority: 0,
            entity: Some(record.actor.entity),
            kind: EventKind::custom(RELEASE_EVENT),
        });
        self.actions.insert(event, ScheduledAction::Close(request));
        record.pending_close = Some((at, event));
    }

    pub fn run_for(&mut self, max_events: u64) -> Result<u64, StagedRuntimeError> {
        let mut dispatched = 0;
        while dispatched < max_events {
            let StepOutcome::Dispatched(event) = self.scheduler.step() else {
                break;
            };
            dispatched += 1;
            let action = self
                .actions
                .remove(&event.id)
                .ok_or(StagedRuntimeError::InvalidState)?;
            let boundary = UsageBoundary {
                at: self.now(),
                event: event.id,
            };
            match action {
                ScheduledAction::Submit(request) => self.activate_or_queue(request, boundary)?,
                ScheduledAction::Close(request) => self.close_at(request, boundary)?,
            }
        }
        Ok(dispatched)
    }

    /// Export a sealed historical window from this forward-only scheduler.
    ///
    /// All dispatches through the endpoint must have executed. Admissions at
    /// the endpoint contribute zero ticks. Releases after it remain censored.
    /// The result contains only occupancy, with resource units left unpriced.
    pub fn committed_usage(
        &mut self,
        resource: StagedResourceId,
        start: SimTime,
        end: SimTime,
    ) -> Result<CommittedUsageWindow, StagedRuntimeError> {
        let resource_record = self
            .resources
            .get(&resource)
            .ok_or(StagedRuntimeError::UnknownResource)?;
        if start >= end
            || end > self.now()
            || self
                .scheduler
                .peek_next()
                .is_some_and(|event| event.at <= end)
        {
            return Err(StagedRuntimeError::InvalidUsageWindow);
        }
        let window = occupancy_window(
            resource,
            resource_record,
            &self.requests,
            self.usage_source.as_ref(),
            start,
            end,
        )?;
        self.record_usage_seal(end);
        Ok(window)
    }

    fn record_usage_seal(&mut self, end: SimTime) {
        self.usage_sealed_through = Some(self.usage_sealed_through.map_or(end, |old| old.max(end)));
    }

    fn activate_or_queue(
        &mut self,
        request: StagedRequestId,
        boundary: UsageBoundary,
    ) -> Result<(), StagedRuntimeError> {
        let record = self
            .requests
            .get_mut(&request)
            .ok_or(StagedRuntimeError::UnknownRequest)?;
        record.pending_submit = None;
        let resource = self
            .resources
            .get_mut(&record.resource)
            .ok_or(StagedRuntimeError::UnknownResource)?;
        if resource.queued.is_empty() && record.quantity <= resource.available {
            Self::activate(record, resource, request, boundary)?;
        } else {
            resource.queued.push_back(request);
            record.state = StagedRequestState::Queued;
        }
        Ok(())
    }

    fn activate(
        record: &mut RequestRecord,
        resource: &mut ResourceRecord,
        request: StagedRequestId,
        boundary: UsageBoundary,
    ) -> Result<(), StagedRuntimeError> {
        if record.admission.is_some() || record.lease.is_some() {
            return Err(StagedRuntimeError::InvalidState);
        }
        let available = resource
            .available
            .checked_sub(record.quantity)
            .ok_or(StagedRuntimeError::InvalidState)?;
        let lease = StagedLeaseId {
            request,
            revision: LEASE_REVISION,
        };
        record.state = StagedRequestState::Active;
        record.lease = Some(lease);
        record.admission = Some(boundary);
        resource.active.push(lease);
        resource.available = available;
        Ok(())
    }

    fn release_and_promote(
        &mut self,
        request: StagedRequestId,
        boundary: UsageBoundary,
    ) -> Result<(), StagedRuntimeError> {
        let record = self
            .requests
            .get_mut(&request)
            .ok_or(StagedRuntimeError::UnknownRequest)?;
        let resource = self
            .resources
            .get_mut(&record.resource)
            .ok_or(StagedRuntimeError::UnknownResource)?;
        let released = record.lease.ok_or(StagedRuntimeError::InvalidState)?;
        let position = resource
            .active
            .iter()
            .position(|lease| *lease == released)
            .ok_or(StagedRuntimeError::InvalidState)?;
        let available = resource
            .available
            .checked_add(record.quantity)
            .filter(|available| *available <= resource.capacity)
            .ok_or(StagedRuntimeError::InvalidState)?;
        resource.active.remove(position);
        resource.available = available;
        record.state = StagedRequestState::Released;
        record.lease = None;
        record.pending_close = None;
        record.release = Some(boundary);
        Self::promote_waiting(&mut self.requests, resource, boundary)
    }

    fn close_at(
        &mut self,
        request: StagedRequestId,
        boundary: UsageBoundary,
    ) -> Result<(), StagedRuntimeError> {
        let record = self
            .requests
            .get_mut(&request)
            .ok_or(StagedRuntimeError::UnknownRequest)?;
        match record.state {
            StagedRequestState::Active => self.release_and_promote(request, boundary),
            StagedRequestState::Scheduled => {
                let event = record
                    .pending_submit
                    .take()
                    .ok_or(StagedRuntimeError::InvalidState)?;
                if !self.scheduler.cancel(event) {
                    return Err(StagedRuntimeError::InvalidState);
                }
                self.actions.remove(&event);
                record.state = StagedRequestState::Released;
                record.pending_close = None;
                Ok(())
            }
            StagedRequestState::Queued => {
                let resource = self
                    .resources
                    .get_mut(&record.resource)
                    .ok_or(StagedRuntimeError::UnknownResource)?;
                let position = resource
                    .queued
                    .iter()
                    .position(|&queued| queued == request)
                    .ok_or(StagedRuntimeError::InvalidState)?;
                resource.queued.remove(position);
                record.state = StagedRequestState::Released;
                record.pending_close = None;
                Self::promote_waiting(&mut self.requests, resource, boundary)
            }
            StagedRequestState::Released => Err(StagedRuntimeError::InvalidState),
        }
    }

    fn promote_waiting(
        requests: &mut BTreeMap<StagedRequestId, RequestRecord>,
        resource: &mut ResourceRecord,
        boundary: UsageBoundary,
    ) -> Result<(), StagedRuntimeError> {
        while let Some(&next) = resource.queued.front() {
            let record = requests
                .get_mut(&next)
                .ok_or(StagedRuntimeError::UnknownRequest)?;
            if record.quantity > resource.available {
                break;
            }
            Self::activate(record, resource, next, boundary)?;
            resource.queued.pop_front();
        }
        Ok(())
    }
}

fn occupancy_window(
    resource: StagedResourceId,
    resource_record: &ResourceRecord,
    requests: &BTreeMap<StagedRequestId, RequestRecord>,
    source: Option<&StagedUsageSource>,
    start: SimTime,
    end: SimTime,
) -> Result<CommittedUsageWindow, StagedRuntimeError> {
    let source = source.ok_or(StagedRuntimeError::MissingUsageSource)?;
    let mut window = CommittedUsageWindow {
        version: STAGED_USAGE_VERSION,
        source: source.clone(),
        resource: resource.entity,
        quantity_unit: resource_record
            .unit
            .as_ref()
            .ok_or(StagedRuntimeError::MissingResourceUnit)?
            .clone(),
        start,
        end,
        occupancy: Vec::new(),
        occupied_unit_ticks: 0,
    };
    for (&request, record) in requests {
        if record.resource != resource {
            continue;
        }
        let Some(admission) = record.admission else {
            continue;
        };
        let release = record.release.filter(|boundary| boundary.at <= end);
        let clipped_start = start.max(admission.at);
        let clipped_end = release.map_or(end, |boundary| boundary.at);
        if clipped_start < clipped_end {
            window.add(OccupancySlice {
                actor: record.actor.entity,
                request: request.entity,
                lease_revision: LEASE_REVISION,
                quantity: record.quantity,
                admission,
                release,
                start: clipped_start,
                end: clipped_end,
            })?;
        }
    }
    Ok(window)
}
