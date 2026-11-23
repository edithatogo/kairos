use super::{
    valid_usage_label, RequestRecord, ResourceRecord, ScheduledAction, StagedActorId,
    StagedLeaseId, StagedRequestId, StagedRequestState, StagedResourceId, StagedResourceRuntime,
    StagedRuntimeError, StagedUsageSource, UsageBoundary, LEASE_REVISION, RELEASE_EVENT,
    SUBMIT_EVENT,
};
use kairo_ecs_core::{Scheduler, SchedulerCheckpointError};
use kairo_ecs_types::{EntityId, EventId, SimTime, StepOutcome};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::{error::Error, fmt};

const VERSION: u16 = 1;

/// The staged checkpoint could not be encoded or restored.
#[derive(Debug)]
#[non_exhaustive]
pub enum StagedCheckpointError {
    /// The staged checkpoint could not be encoded or decoded as JSON.
    Json(serde_json::Error),
    /// The embedded scheduler could not be encoded or restored.
    Scheduler(SchedulerCheckpointError),
    /// The staged checkpoint version is unsupported.
    UnsupportedVersion(u16),
    /// The checkpoint records disagree with allocator invariants.
    InvalidState(&'static str),
    /// A fresh allocator identity could not be acquired.
    Runtime(StagedRuntimeError),
}

impl fmt::Display for StagedCheckpointError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(error) => write!(f, "staged checkpoint JSON: {error}"),
            Self::Scheduler(error) => write!(f, "staged checkpoint scheduler: {error}"),
            Self::UnsupportedVersion(version) => {
                write!(f, "unsupported staged checkpoint version {version}")
            }
            Self::InvalidState(reason) => write!(f, "invalid staged checkpoint: {reason}"),
            Self::Runtime(error) => {
                write!(f, "could not allocate restored staged runtime: {error:?}")
            }
        }
    }
}

impl Error for StagedCheckpointError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Json(error) => Some(error),
            Self::Scheduler(error) => Some(error),
            _ => None,
        }
    }
}

#[derive(Deserialize)]
struct CheckpointVersion {
    version: u16,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CheckpointV1 {
    version: u16,
    scheduler_json: String,
    usage_source: Option<StagedUsageSource>,
    usage_sealed_through: Option<u128>,
    actors: Vec<u64>,
    resources: Vec<ResourceV1>,
    requests: Vec<RequestV1>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ResourceV1 {
    unit: Option<String>,
    capacity: u64,
    active: Vec<u64>,
    queued: Vec<u64>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RequestV1 {
    actor: u64,
    resource: u64,
    state: StagedRequestState,
    quantity: u64,
    pending_submit: Option<(u64, u32)>,
    pending_close: Option<BoundaryV1>,
    admission: Option<BoundaryV1>,
    release: Option<BoundaryV1>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct BoundaryV1 {
    at_ticks: u128,
    event: (u64, u32),
}

impl From<UsageBoundary> for BoundaryV1 {
    fn from(value: UsageBoundary) -> Self {
        Self {
            at_ticks: value.at.ticks(),
            event: (value.event.index, value.event.generation),
        }
    }
}

impl From<BoundaryV1> for UsageBoundary {
    fn from(value: BoundaryV1) -> Self {
        Self {
            at: SimTime::from_ticks(value.at_ticks),
            event: EventId::new(value.event.0, value.event.1),
        }
    }
}

impl StagedResourceRuntime {
    /// Encode allocator state and its private scheduler in versioned JSON.
    ///
    /// Capture at a model event boundary. Store these bytes with the caller's
    /// model state, RNG and any outer scheduler. Persist entity IDs for handles
    /// the model needs after restart.
    ///
    /// # Errors
    /// Returns an error if the scheduler or allocator cannot be encoded.
    pub fn checkpoint_json(&self) -> Result<Vec<u8>, StagedCheckpointError> {
        let scheduler = self
            .scheduler
            .checkpoint_json()
            .map_err(StagedCheckpointError::Scheduler)?;
        let checkpoint = CheckpointV1 {
            version: VERSION,
            scheduler_json: String::from_utf8(scheduler).expect("scheduler JSON is UTF-8"),
            usage_source: self.usage_source.clone(),
            usage_sealed_through: self.usage_sealed_through.map(SimTime::ticks),
            actors: self.actors.keys().map(|actor| actor.entity.index).collect(),
            resources: self
                .resources
                .values()
                .map(|resource| ResourceV1 {
                    unit: resource.unit.clone(),
                    capacity: resource.capacity,
                    active: resource
                        .active
                        .iter()
                        .map(|lease| lease.request.entity.index)
                        .collect(),
                    queued: resource
                        .queued
                        .iter()
                        .map(|request| request.entity.index)
                        .collect(),
                })
                .collect(),
            requests: self
                .requests
                .values()
                .map(|request| RequestV1 {
                    actor: request.actor.entity.index,
                    resource: request.resource.entity.index,
                    state: request.state,
                    quantity: request.quantity,
                    pending_submit: request
                        .pending_submit
                        .map(|event| (event.index, event.generation)),
                    pending_close: request
                        .pending_close
                        .map(|(at, event)| UsageBoundary { at, event }.into()),
                    admission: request.admission.map(Into::into),
                    release: request.release.map(Into::into),
                })
                .collect(),
        };
        serde_json::to_vec(&checkpoint).map_err(StagedCheckpointError::Json)
    }

    /// Restore an independent allocator from trusted, integrity-checked bytes.
    ///
    /// Saved entity IDs retain their identity. Use the lookup methods to reacquire
    /// handles, then use `request` to retrieve the current lease.
    ///
    /// # Errors
    /// Returns an error for malformed input, an unsupported version, an
    /// inconsistent allocator or scheduler, or an exhausted runtime identity counter.
    pub fn from_checkpoint_json(bytes: &[u8]) -> Result<Self, StagedCheckpointError> {
        use StagedCheckpointError::InvalidState;

        let CheckpointVersion { version } =
            serde_json::from_slice(bytes).map_err(StagedCheckpointError::Json)?;
        if version != VERSION {
            return Err(StagedCheckpointError::UnsupportedVersion(version));
        }
        let checkpoint: CheckpointV1 =
            serde_json::from_slice(bytes).map_err(StagedCheckpointError::Json)?;
        let scheduler = Scheduler::from_checkpoint_json(checkpoint.scheduler_json.as_bytes())
            .map_err(StagedCheckpointError::Scheduler)?;
        if let Some(source) = &checkpoint.usage_source {
            source
                .validate()
                .map_err(|_| InvalidState("invalid usage provenance"))?;
        }

        let mut runtime = Self::new().map_err(StagedCheckpointError::Runtime)?;
        let owner = runtime.runtime_id;
        runtime.scheduler = scheduler;
        runtime.usage_source = checkpoint.usage_source;
        runtime.usage_sealed_through = checkpoint.usage_sealed_through.map(SimTime::from_ticks);
        runtime.next_actor = checkpoint.actors.len() as u64;
        runtime.next_resource = checkpoint.resources.len() as u64;
        runtime.next_request = checkpoint.requests.len() as u64;

        for (index, saved) in checkpoint.actors.into_iter().enumerate() {
            if saved != index as u64 {
                return Err(InvalidState("actor identities are not in canonical order"));
            }
            let id = StagedActorId {
                runtime: owner,
                entity: EntityId::new(saved, 0),
            };
            runtime.actors.insert(id, ());
        }
        let request_id = |index| StagedRequestId {
            runtime: owner,
            entity: EntityId::new(index, 0),
        };
        for (index, row) in checkpoint.resources.into_iter().enumerate() {
            if row.capacity == 0
                || row
                    .unit
                    .as_deref()
                    .is_some_and(|unit| !valid_usage_label(unit))
            {
                return Err(InvalidState("resource capacity or usage unit is invalid"));
            }
            let id = StagedResourceId {
                runtime: owner,
                entity: EntityId::new(index as u64, 0),
            };
            runtime.resources.insert(
                id,
                ResourceRecord {
                    unit: row.unit,
                    capacity: row.capacity,
                    available: row.capacity,
                    active: row
                        .active
                        .into_iter()
                        .map(|index| StagedLeaseId {
                            request: request_id(index),
                            revision: LEASE_REVISION,
                        })
                        .collect(),
                    queued: row.queued.into_iter().map(request_id).collect(),
                },
            );
        }
        for (index, row) in checkpoint.requests.into_iter().enumerate() {
            runtime.restore_request(request_id(index as u64), row)?;
        }
        runtime.validate_resource_membership()?;
        runtime.validate_checkpoint_actions()?;
        runtime.validate_checkpoint_occupancy()?;
        Ok(runtime)
    }

    /// Bind an actor's saved entity ID to this allocator.
    pub fn actor_by_entity(&self, entity: EntityId) -> Option<StagedActorId> {
        let id = StagedActorId {
            runtime: self.runtime_id,
            entity,
        };
        self.actors.contains_key(&id).then_some(id)
    }

    /// Bind a resource's saved entity ID to this allocator.
    pub fn resource_by_entity(&self, entity: EntityId) -> Option<StagedResourceId> {
        let id = StagedResourceId {
            runtime: self.runtime_id,
            entity,
        };
        self.resources.contains_key(&id).then_some(id)
    }

    /// Bind a request's saved entity ID to this allocator.
    pub fn request_by_entity(&self, entity: EntityId) -> Option<StagedRequestId> {
        let id = StagedRequestId {
            runtime: self.runtime_id,
            entity,
        };
        self.requests.contains_key(&id).then_some(id)
    }

    fn restore_request(
        &mut self,
        id: StagedRequestId,
        row: RequestV1,
    ) -> Result<(), StagedCheckpointError> {
        use StagedCheckpointError::InvalidState;
        use StagedRequestState::{Active, Queued, Released, Scheduled};

        let actor = self
            .actor_by_entity(EntityId::new(row.actor, 0))
            .ok_or(InvalidState("request actor does not exist"))?;
        let resource = self
            .resource_by_entity(EntityId::new(row.resource, 0))
            .ok_or(InvalidState("request resource does not exist"))?;
        if row.quantity == 0 || row.quantity > self.resources[&resource].capacity {
            return Err(InvalidState("request quantity exceeds resource bounds"));
        }
        let pending_submit = row
            .pending_submit
            .map(|(index, generation)| EventId::new(index, generation));
        let pending_close = row.pending_close.map(UsageBoundary::from);
        let admission = row.admission.map(UsageBoundary::from);
        let release = row.release.map(UsageBoundary::from);
        let valid_state = match row.state {
            Scheduled => pending_submit.is_some() && admission.is_none() && release.is_none(),
            Queued => pending_submit.is_none() && admission.is_none() && release.is_none(),
            Active => pending_submit.is_none() && admission.is_some() && release.is_none(),
            Released => {
                pending_submit.is_none()
                    && pending_close.is_none()
                    && admission.is_some() == release.is_some()
            }
        };
        if !valid_state {
            return Err(InvalidState("request state and recorded actions disagree"));
        }
        if let Some((start, end)) = admission.zip(release) {
            if (start.at, start.event.index) >= (end.at, end.event.index)
                || start.event == end.event
            {
                return Err(InvalidState("release does not follow admission"));
            }
        }
        for (event, action) in [
            (pending_submit, ScheduledAction::Submit(id)),
            (
                pending_close.map(|boundary| boundary.event),
                ScheduledAction::Close(id),
            ),
        ] {
            if let Some(event) = event {
                if self.actions.insert(event, action).is_some() {
                    return Err(InvalidState(
                        "pending event belongs to more than one action",
                    ));
                }
            }
        }
        self.requests.insert(
            id,
            RequestRecord {
                actor,
                resource,
                state: row.state,
                lease: (row.state == Active).then_some(StagedLeaseId {
                    request: id,
                    revision: LEASE_REVISION,
                }),
                pending_submit,
                pending_close: pending_close.map(|boundary| (boundary.at, boundary.event)),
                quantity: row.quantity,
                admission,
                release,
            },
        );
        Ok(())
    }

    fn validate_resource_membership(&mut self) -> Result<(), StagedCheckpointError> {
        use StagedCheckpointError::InvalidState;
        use StagedRequestState::{Active, Queued};

        let mut seen = BTreeSet::new();
        for (&id, resource) in &mut self.resources {
            let memberships = resource
                .active
                .iter()
                .map(|lease| (lease.request, Active))
                .chain(resource.queued.iter().map(|&request| (request, Queued)));
            for (request, expected_state) in memberships {
                let record = self.requests.get(&request).ok_or(InvalidState(
                    "resource membership references a missing request",
                ))?;
                if record.resource != id || record.state != expected_state || !seen.insert(request)
                {
                    return Err(InvalidState(
                        "resource membership and request state disagree",
                    ));
                }
                if expected_state == Active {
                    resource.available = resource
                        .available
                        .checked_sub(record.quantity)
                        .ok_or(InvalidState("active leases exceed capacity"))?;
                }
            }
            if resource
                .queued
                .front()
                .is_some_and(|head| self.requests[head].quantity <= resource.available)
            {
                return Err(InvalidState(
                    "the FIFO head could already have been admitted",
                ));
            }
        }
        if self
            .requests
            .values()
            .filter(|row| matches!(row.state, Active | Queued))
            .count()
            != seen.len()
        {
            return Err(InvalidState(
                "active or queued request is missing from its resource",
            ));
        }
        Ok(())
    }

    fn validate_checkpoint_actions(&self) -> Result<(), StagedCheckpointError> {
        use StagedCheckpointError::InvalidState;
        if self.actions.len() != self.scheduler.pending_events() {
            return Err(InvalidState("pending actions and scheduler count disagree"));
        }

        // Inspect the native dispatch order on a private scheduler copy.
        let mut inspection = Scheduler::from_checkpoint(&self.scheduler.checkpoint())
            .expect("checkpoint produced by this scheduler");
        while let StepOutcome::Dispatched(event) = inspection.step() {
            let (request, kind) = match self.actions.get(&event.id) {
                Some(ScheduledAction::Submit(request)) => (request, SUBMIT_EVENT),
                Some(ScheduledAction::Close(request)) => (request, RELEASE_EVENT),
                None => {
                    return Err(InvalidState(
                        "pending scheduler event has no matching action",
                    ))
                }
            };
            let record = &self.requests[request];
            let close_tick = record
                .pending_close
                .filter(|&(_, id)| id == event.id)
                .map(|(at, _)| at);
            if event.kind.code() != kind
                || event.entity != Some(record.actor.entity)
                || event.priority != 0
                || event.at < self.scheduler.now()
                || self
                    .usage_sealed_through
                    .is_some_and(|seal| event.at <= seal)
                || close_tick.is_some_and(|at| at != event.at)
            {
                return Err(InvalidState(
                    "pending action does not match scheduler event or cutoff",
                ));
            }
        }
        Ok(())
    }

    fn validate_checkpoint_occupancy(&self) -> Result<(), StagedCheckpointError> {
        use StagedCheckpointError::InvalidState;

        struct Dispatch {
            at: SimTime,
            resource: StagedResourceId,
            released: u64,
            admitted: u64,
        }

        let stats = self.scheduler.stats();
        let mut history = BTreeMap::new();
        for request in self.requests.values() {
            for (boundary, released, admitted) in [
                (request.release, request.quantity, 0),
                (request.admission, 0, request.quantity),
            ] {
                let Some(boundary) = boundary else {
                    continue;
                };
                if boundary.at > stats.now
                    || boundary.event.index >= stats.scheduled_events
                    || boundary.event.generation != boundary.event.index as u32
                    || self.actions.contains_key(&boundary.event)
                {
                    return Err(InvalidState(
                        "occupancy boundary does not refer to a past dispatch",
                    ));
                }
                let dispatch = history.entry(boundary.event).or_insert(Dispatch {
                    at: boundary.at,
                    resource: request.resource,
                    released: 0,
                    admitted: 0,
                });
                if dispatch.at != boundary.at
                    || dispatch.resource != request.resource
                    || (released != 0 && dispatch.released != 0)
                {
                    return Err(InvalidState("occupancy boundaries for a dispatch disagree"));
                }
                dispatch.released = dispatch.released.max(released);
                dispatch.admitted = dispatch
                    .admitted
                    .checked_add(admitted)
                    .ok_or(InvalidState("historical occupancy exceeds capacity"))?;
            }
        }
        if history.len() as u64 > stats.dispatched_events {
            return Err(InvalidState("occupancy references too many dispatches"));
        }
        let mut dispatches: Vec<_> = history.into_iter().collect();
        dispatches.sort_unstable_by_key(|(event, dispatch)| (dispatch.at, event.index));
        let mut occupied = BTreeMap::<_, u64>::new();
        for (_, dispatch) in dispatches {
            // A close releases capacity before its FIFO promotions consume it.
            let units = occupied.entry(dispatch.resource).or_default();
            *units = units
                .checked_sub(dispatch.released)
                .and_then(|remaining| remaining.checked_add(dispatch.admitted))
                .filter(|&total| total <= self.resources[&dispatch.resource].capacity)
                .ok_or(InvalidState("historical occupancy exceeds capacity"))?;
        }
        Ok(())
    }
}
