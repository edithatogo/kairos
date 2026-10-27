//! Experimental, purpose-specific runtime for staged resource allocation.
//!
//! This is not a replacement for the removed general-purpose `FlowRuntime`.

use kairo_ecs_core::Scheduler;
use kairo_ecs_types::{EntityId, EventId, EventKind, ScheduleRequest, SimTime, StepOutcome};
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};

const SUBMIT_EVENT: u32 = 0x5354_4701;
const RELEASE_EVENT: u32 = 0x5354_4702;
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
    Release(StagedRequestId),
}

#[derive(Debug)]
struct RequestRecord {
    actor: StagedActorId,
    resource: StagedResourceId,
    state: StagedRequestState,
    lease: Option<StagedLeaseId>,
    release_pending: bool,
}

#[derive(Debug)]
struct ResourceRecord {
    capacity: u64,
    active: Vec<StagedLeaseId>,
    queued: VecDeque<StagedRequestId>,
}

/// Scheduler-backed experimental runtime for the CareOps staged allocation contract.
pub struct StagedResourceRuntime {
    runtime_id: usize,
    scheduler: Scheduler,
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
            next_actor: 0,
            next_resource: 0,
            next_request: 0,
            actors: BTreeMap::new(),
            resources: BTreeMap::new(),
            requests: BTreeMap::new(),
            actions: HashMap::new(),
        })
    }

    pub fn now(&self) -> SimTime {
        self.scheduler.now()
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
                capacity,
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
        if at < self.now() {
            return Err(StagedRuntimeError::SubmissionInPast);
        }
        if resource.runtime != self.runtime_id {
            return Err(StagedRuntimeError::UnknownResource);
        }
        if actor.runtime != self.runtime_id {
            return Err(StagedRuntimeError::UnknownActor);
        }
        if !self.resources.contains_key(&resource) {
            return Err(StagedRuntimeError::UnknownResource);
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
                release_pending: false,
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
            available: r.capacity - r.active.len() as u64,
            active: r.active.clone(),
            queued: r.queued.iter().copied().collect(),
        })
    }

    pub fn release(&mut self, lease: StagedLeaseId, at: SimTime) -> Result<(), StagedRuntimeError> {
        if lease.request.runtime != self.runtime_id {
            return Err(StagedRuntimeError::InvalidState);
        }
        if at < self.now() {
            return Err(StagedRuntimeError::InvalidReleaseTime);
        }
        let r = self
            .requests
            .get_mut(&lease.request)
            .ok_or(StagedRuntimeError::UnknownRequest)?;
        if r.state != StagedRequestState::Active || r.lease != Some(lease) || r.release_pending {
            return Err(StagedRuntimeError::InvalidState);
        }
        r.release_pending = true;
        let entity = r.actor.entity;
        let event = self.scheduler.schedule(ScheduleRequest {
            at,
            priority: 0,
            entity: Some(entity),
            kind: EventKind::custom(RELEASE_EVENT),
        });
        self.actions
            .insert(event, ScheduledAction::Release(lease.request));
        Ok(())
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
            match action {
                ScheduledAction::Submit(request) => self.activate_or_queue(request)?,
                ScheduledAction::Release(request) => self.release_and_promote(request)?,
            }
        }
        Ok(dispatched)
    }

    fn activate_or_queue(&mut self, request: StagedRequestId) -> Result<(), StagedRuntimeError> {
        let resource_id = self
            .requests
            .get(&request)
            .ok_or(StagedRuntimeError::UnknownRequest)?
            .resource;
        let r = self
            .resources
            .get_mut(&resource_id)
            .ok_or(StagedRuntimeError::UnknownResource)?;
        if (r.active.len() as u64) < r.capacity {
            Self::activate(&mut self.requests, r, request)?;
        } else {
            r.queued.push_back(request);
            self.requests.get_mut(&request).unwrap().state = StagedRequestState::Queued;
        }
        Ok(())
    }

    fn activate(
        requests: &mut BTreeMap<StagedRequestId, RequestRecord>,
        resource: &mut ResourceRecord,
        request: StagedRequestId,
    ) -> Result<(), StagedRuntimeError> {
        let record = requests
            .get_mut(&request)
            .ok_or(StagedRuntimeError::UnknownRequest)?;
        if record.lease.is_some() {
            return Err(StagedRuntimeError::InvalidState);
        }
        let lease = StagedLeaseId {
            request,
            revision: 1,
        };
        record.state = StagedRequestState::Active;
        record.lease = Some(lease);
        resource.active.push(lease);
        Ok(())
    }

    fn release_and_promote(&mut self, request: StagedRequestId) -> Result<(), StagedRuntimeError> {
        let resource_id = self
            .requests
            .get(&request)
            .ok_or(StagedRuntimeError::UnknownRequest)?
            .resource;
        let resource = self
            .resources
            .get_mut(&resource_id)
            .ok_or(StagedRuntimeError::UnknownResource)?;
        let released = self
            .requests
            .get(&request)
            .ok_or(StagedRuntimeError::UnknownRequest)?
            .lease
            .ok_or(StagedRuntimeError::InvalidState)?;
        let position = resource
            .active
            .iter()
            .position(|v| *v == released)
            .ok_or(StagedRuntimeError::InvalidState)?;
        resource.active.remove(position);
        let record = self.requests.get_mut(&request).unwrap();
        record.state = StagedRequestState::Released;
        record.release_pending = false;
        if let Some(next) = resource.queued.pop_front() {
            Self::activate(&mut self.requests, resource, next)?;
        }
        Ok(())
    }
}
