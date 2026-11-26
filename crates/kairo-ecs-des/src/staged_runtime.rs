//! Experimental, purpose-specific runtime for staged resource allocation.
//!
//! This is not a replacement for the removed general-purpose `FlowRuntime`.

use crate::usage::{
    valid_usage_label, CommittedUsageWindow, OccupancySlice, StagedUsageSource, UsageBoundary,
    STAGED_USAGE_VERSION,
};
use kairo_ecs_core::{ScheduledEventPreview, Scheduler};
use kairo_ecs_types::{
    DispatchedEvent, EntityId, EventId, EventKind, ScheduleRequest, SimTime, StepOutcome,
};
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};

#[cfg(feature = "checkpoint-json")]
mod checkpoint;
#[cfg(feature = "checkpoint-json")]
pub use checkpoint::StagedCheckpointError;

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
#[cfg_attr(
    feature = "checkpoint-json",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "snake_case")
)]
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

/// Experimental result of a dispatch across staged allocation and the model scheduler.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagedDispatch {
    /// One allocation action executed at this tick.
    Allocation(SimTime),
    /// One model event is ready for its handler.
    Model(DispatchedEvent),
}

/// Experimental preview of the next dispatch across staged allocation and the model scheduler.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagedDispatchPreview {
    /// An allocation action is next at this tick.
    Allocation(SimTime),
    /// A model event is next in its own scheduler.
    Model(ScheduledEventPreview),
}

/// Experimental selection of the next dispatch across allocation and multiple model queues.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagedDispatchSelection {
    /// An allocation action is next at this tick.
    Allocation(SimTime),
    /// A model event is next in the selected scheduler.
    Model {
        /// Zero-based position in the supplied scheduler slice.
        scheduler_index: usize,
        /// The next live event in that scheduler.
        event: ScheduledEventPreview,
    },
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

    /// Dispatch up to `max_events` live actions through the inclusive time limit.
    ///
    /// Actions at the same tick keep their scheduler order. The allocator clock
    /// records the last dispatch. Callers can schedule more actions at the
    /// limit tick under the usual scheduling and usage rules.
    /// Use [`Self::next_event_time`] to check for due
    /// work after exhausting the budget.
    ///
    /// # Errors
    ///
    /// Propagates dispatch errors from [`Self::run_for`]. The scheduler may
    /// consume the failing event. Earlier dispatches have already taken effect.
    pub fn run_until_or_for(
        &mut self,
        time_limit: SimTime,
        max_events: u64,
    ) -> Result<u64, StagedRuntimeError> {
        let mut dispatched = 0;
        for _ in 0..max_events {
            if self.next_event_time().is_none_or(|tick| tick > time_limit) {
                break;
            }
            dispatched += self.run_for(1)?;
        }
        Ok(dispatched)
    }

    /// Preview the next live event across the allocator and the model scheduler.
    ///
    /// The earliest event through the inclusive `time_limit` wins. Allocation
    /// runs first at an equal tick. The selected event stays queued so the
    /// owning dispatcher can run it. Preview may prune cancelled entries.
    /// Both schedulers keep their clocks, counters and live pending events.
    /// Preview again after scheduling, cancelling or dispatching any event.
    /// Returns `Ok(None)` when both queues have no event due.
    ///
    /// # Errors
    ///
    /// An eligible event earlier than either scheduler's current tick yields
    /// [`StagedRuntimeError::InvalidState`].
    pub fn peek_with_scheduler(
        &mut self,
        scheduler: &mut Scheduler,
        time_limit: SimTime,
    ) -> Result<Option<StagedDispatchPreview>, StagedRuntimeError> {
        Ok(self
            .peek_with_schedulers(&mut [scheduler], time_limit)?
            .map(|selection| match selection {
                StagedDispatchSelection::Allocation(at) => StagedDispatchPreview::Allocation(at),
                StagedDispatchSelection::Model { event, .. } => StagedDispatchPreview::Model(event),
            }))
    }

    /// Preview the next live event across allocation and the supplied model schedulers.
    ///
    /// The time limit is inclusive. At equal ticks, allocation comes first,
    /// then model schedulers in slice order. Each queue keeps its own priority
    /// and insertion order. Keep the same schedulers in the same order for
    /// the run, including idle queues, and use a common time unit.
    ///
    /// The owning dispatcher runs the selected event. Preview leaves live
    /// events, clocks and counters in place and may prune cancelled entries.
    /// Preview again after scheduling, cancelling or dispatching work.
    /// Returns `Ok(None)` when no event is due through the limit.
    ///
    /// # Errors
    ///
    /// An eligible event earlier than any participating clock yields
    /// [`StagedRuntimeError::InvalidState`].
    pub fn peek_with_schedulers(
        &mut self,
        schedulers: &mut [&mut Scheduler],
        time_limit: SimTime,
    ) -> Result<Option<StagedDispatchSelection>, StagedRuntimeError> {
        let mut latest_tick = self.now();
        let mut next = self
            .next_event_time()
            .map(|at| (at, StagedDispatchSelection::Allocation(at)));
        for (scheduler_index, scheduler) in schedulers.iter_mut().enumerate() {
            latest_tick = latest_tick.max(scheduler.now());
            let Some(event) = scheduler.peek_next() else {
                continue;
            };
            if next.is_none_or(|(at, _)| event.at < at) {
                next = Some((
                    event.at,
                    StagedDispatchSelection::Model {
                        scheduler_index,
                        event,
                    },
                ));
            }
        }

        let Some((at, selection)) = next.filter(|(tick, _)| *tick <= time_limit) else {
            return Ok(None);
        };
        if at < latest_tick {
            return Err(StagedRuntimeError::InvalidState);
        }
        Ok(Some(selection))
    }

    /// Dispatch at most one event across the allocator and the model scheduler.
    ///
    /// The earliest live event through the inclusive `time_limit` wins.
    /// Allocation runs first at an equal tick. Each queue keeps its scheduler
    /// order. Only the selected scheduler advances. Use the returned event's
    /// tick to schedule reactions, including work at the same tick. Run a
    /// handler that reads the model scheduler's clock from a model event at
    /// the reaction tick.
    /// Returns `Ok(None)` when both queues have no event due.
    /// Use [`Self::peek_with_scheduler`] when the model dispatches its own events.
    ///
    /// # Errors
    ///
    /// An eligible event earlier than either scheduler's current tick yields
    /// [`StagedRuntimeError::InvalidState`] before dispatch. Allocation errors
    /// follow [`Self::run_until_or_for`], including consumption of a failing event.
    pub fn step_with_scheduler(
        &mut self,
        scheduler: &mut Scheduler,
        time_limit: SimTime,
    ) -> Result<Option<StagedDispatch>, StagedRuntimeError> {
        match self.peek_with_scheduler(scheduler, time_limit)? {
            Some(StagedDispatchPreview::Allocation(at)) => {
                if self.run_until_or_for(at, 1)? != 1 {
                    return Err(StagedRuntimeError::InvalidState);
                }
                Ok(Some(StagedDispatch::Allocation(at)))
            }
            Some(StagedDispatchPreview::Model(_)) => {
                let event = scheduler
                    .step()
                    .into_dispatched()
                    .ok_or(StagedRuntimeError::InvalidState)?;
                Ok(Some(StagedDispatch::Model(event)))
            }
            None => Ok(None),
        }
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
        self.committed_usage_with_schedulers(resource, start, end, &mut [])
    }

    /// Export occupancy at a coordinated observation tick in a forward-only run.
    ///
    /// One participating scheduler must have reached `end`. All queues,
    /// including the allocator, must have finished live work through that
    /// tick. Supply the model schedulers from the same run and finish their
    /// handlers and reactions before exporting. This lets a model observe
    /// through an interval when the allocator is idle.
    ///
    /// A successful export seals allocation through `end`. Event previews
    /// may prune cancelled entries. Clocks retain their last dispatch ticks.
    ///
    /// # Errors
    ///
    /// Returns [`StagedRuntimeError::InvalidUsageWindow`] for reversed or empty
    /// bounds, an unreached endpoint, or live work due through `end`. Returns
    /// the same resource, provenance and overflow errors as
    /// [`Self::committed_usage`].
    pub fn committed_usage_with_schedulers(
        &mut self,
        resource: StagedResourceId,
        start: SimTime,
        end: SimTime,
        schedulers: &mut [&mut Scheduler],
    ) -> Result<CommittedUsageWindow, StagedRuntimeError> {
        let resource_record = self
            .resources
            .get(&resource)
            .ok_or(StagedRuntimeError::UnknownResource)?;
        let reached =
            self.now() >= end || schedulers.iter().any(|scheduler| scheduler.now() >= end);
        if start >= end
            || !reached
            || self
                .scheduler
                .peek_next()
                .is_some_and(|event| event.at <= end)
            || schedulers
                .iter_mut()
                .any(|scheduler| scheduler.peek_next().is_some_and(|event| event.at <= end))
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
