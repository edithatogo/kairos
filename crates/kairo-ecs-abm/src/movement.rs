//! Scheduled, point-to-point movement on a model's existing Rust scheduler.

use crate::routing::{AccessProfile, RoutingError, SpatialRoute, SpatialTopology};
use kairo_ecs_core::Scheduler;
use kairo_ecs_state::World;
use kairo_ecs_types::{DispatchedEvent, EntityId, EventId, EventKind, ScheduleRequest, SimTime};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Scheduler event reserved for movement arrivals.
pub const MOVEMENT_ARRIVAL_EVENT_KIND: EventKind = EventKind::custom(0x4d4f5601);

static NEXT_RUNTIME: AtomicUsize = AtomicUsize::new(0);

/// Opaque operation handle scoped to a movement runtime and actor generation.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MovementId {
    runtime: usize,
    sequence: u64,
    actor: EntityId,
}

impl MovementId {
    pub fn actor(self) -> EntityId {
        self.actor
    }
}

/// Destination and scheduler parameters for a single actor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MovementRequest {
    pub actor: EntityId,
    pub destination: EntityId,
    pub depart_at: SimTime,
    pub access: AccessProfile,
    pub priority: i32,
}

/// A pending move's route, revision and exact scheduler arrival.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MovementPlan {
    pub id: MovementId,
    pub revision: u64,
    pub origin: EntityId,
    pub destination: EntityId,
    pub depart_at: SimTime,
    pub arrive_at: SimTime,
    pub arrival_event: EventId,
    pub route: SpatialRoute,
}

/// Terminal outcome of the currently scheduled arrival.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MovementArrivalOutcome {
    Arrived,
    ActorRetired,
    TopologyChanged,
}

/// The completed plan, causal scheduler event and resolution.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MovementArrival {
    pub plan: MovementPlan,
    pub event: DispatchedEvent,
    pub outcome: MovementArrivalOutcome,
}

#[derive(Debug, Eq, PartialEq, thiserror::Error)]
pub enum MovementError {
    #[error("movement identity or revision space exhausted")]
    IdentityExhausted,
    #[error("movement handle was not issued by this runtime")]
    UnknownMovement,
    #[error("movement {0:?} has already finished")]
    NotMoving(MovementId),
    #[error("actor {0:?} is not live")]
    ActorRetired(EntityId),
    #[error("actor {0:?} has no location")]
    ActorUnplaced(EntityId),
    #[error("actor {0:?} already has a pending move")]
    AlreadyMoving(EntityId),
    #[error("departure {depart_at:?} is earlier than scheduler time {now:?}")]
    DepartureInPast { depart_at: SimTime, now: SimTime },
    #[error("movement arrival time overflowed")]
    ArrivalOverflow,
    #[error("arrival event {0:?} is no longer pending in the scheduler")]
    ArrivalNotPending(EventId),
    #[error(transparent)]
    Route(#[from] RoutingError),
}

#[derive(Clone)]
struct PendingMovement {
    plan: MovementPlan,
    topology_identity: Arc<()>,
    access: AccessProfile,
    priority: i32,
}

impl PendingMovement {
    fn matches_topology(&self, topology: &SpatialTopology) -> bool {
        Arc::ptr_eq(&self.topology_identity, topology.identity())
            && self.plan.route.topology_version == topology.version()
    }
}

/// Owns placements and pending movement operations for one world and scheduler.
///
/// Snapshot this runtime together with its world, topology state and scheduler.
/// Keep handles and native events within the corresponding replay branch.
#[derive(Clone)]
pub struct MovementRuntime {
    runtime: usize,
    next_sequence: u64,
    locations: BTreeMap<EntityId, EntityId>,
    pending: BTreeMap<EntityId, PendingMovement>,
}

impl MovementRuntime {
    pub fn new() -> Result<Self, MovementError> {
        let runtime = NEXT_RUNTIME
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |next| {
                next.checked_add(1)
            })
            .map_err(|_| MovementError::IdentityExhausted)?;
        Ok(Self {
            runtime,
            next_sequence: 0,
            locations: BTreeMap::new(),
            pending: BTreeMap::new(),
        })
    }

    /// Register or replace a live actor's location while it is stationary.
    pub fn place(
        &mut self,
        world: &World,
        topology: &SpatialTopology,
        actor: EntityId,
        node: EntityId,
    ) -> Result<(), MovementError> {
        self.require_stationary(world, actor)?;
        if !topology.contains_node(node) {
            return Err(RoutingError::NodeNotFound(node).into());
        }
        self.locations.insert(actor, node);
        Ok(())
    }

    /// Last settled location for a live actor, including while a move is pending.
    pub fn location(&self, world: &World, actor: EntityId) -> Option<EntityId> {
        if !world.is_alive(actor) {
            return None;
        }
        self.locations.get(&actor).copied()
    }

    /// Current revision of a pending move.
    pub fn pending(&self, id: MovementId) -> Result<Option<&MovementPlan>, MovementError> {
        self.check_handle(id)?;
        Ok(self
            .pending
            .get(&id.actor)
            .filter(|p| p.plan.id == id)
            .map(|p| &p.plan))
    }

    /// Plan and enqueue an arrival. Location changes when its event is handled.
    pub fn schedule(
        &mut self,
        scheduler: &mut Scheduler,
        world: &World,
        topology: &SpatialTopology,
        request: MovementRequest,
    ) -> Result<MovementPlan, MovementError> {
        self.require_stationary(world, request.actor)?;
        let origin = self
            .locations
            .get(&request.actor)
            .copied()
            .ok_or(MovementError::ActorUnplaced(request.actor))?;
        let now = scheduler.now();
        if request.depart_at < now {
            return Err(MovementError::DepartureInPast {
                depart_at: request.depart_at,
                now,
            });
        }
        let route = topology.find_route_uncached(origin, request.destination, &request.access)?;
        let arrive_at = checked_arrival(request.depart_at, &route)?;
        let sequence = self.next_sequence;
        self.next_sequence = sequence
            .checked_add(1)
            .ok_or(MovementError::IdentityExhausted)?;
        let id = MovementId {
            runtime: self.runtime,
            sequence,
            actor: request.actor,
        };
        let plan = MovementPlan {
            id,
            revision: 0,
            origin,
            destination: request.destination,
            depart_at: request.depart_at,
            arrive_at,
            arrival_event: enqueue_arrival(scheduler, id.actor, arrive_at, request.priority),
            route,
        };
        self.pending.insert(
            id.actor,
            PendingMovement {
                plan: plan.clone(),
                topology_identity: Arc::clone(topology.identity()),
                access: request.access,
                priority: request.priority,
            },
        );
        Ok(plan)
    }

    /// Cancel the exact live arrival. Finished or already cancelled moves return false.
    pub fn cancel(
        &mut self,
        scheduler: &mut Scheduler,
        id: MovementId,
    ) -> Result<bool, MovementError> {
        let Some(plan) = self.pending(id)? else {
            return Ok(false);
        };
        cancel_arrival(scheduler, plan.arrival_event)?;
        self.pending.remove(&id.actor);
        Ok(true)
    }

    /// Remove placement and cancel any pending movement when retiring an actor.
    /// Returns the cancelled handle so the caller can close associated resources.
    pub fn forget_actor(
        &mut self,
        scheduler: &mut Scheduler,
        actor: EntityId,
    ) -> Result<Option<MovementId>, MovementError> {
        let id = self.pending.get(&actor).map(|p| p.plan.id);
        if let Some(id) = id {
            self.cancel(scheduler, id)?;
        }
        self.locations.remove(&actor);
        Ok(id)
    }

    /// Recompute a stale route and replace its queued arrival after validation.
    pub fn replan(
        &mut self,
        scheduler: &mut Scheduler,
        world: &World,
        topology: &SpatialTopology,
        id: MovementId,
    ) -> Result<MovementPlan, MovementError> {
        self.check_handle(id)?;
        let pending = self
            .pending
            .get_mut(&id.actor)
            .filter(|p| p.plan.id == id)
            .ok_or(MovementError::NotMoving(id))?;
        if !world.is_alive(id.actor) {
            return Err(MovementError::ActorRetired(id.actor));
        }
        if pending.matches_topology(topology) {
            return Ok(pending.plan.clone());
        }
        let plan = &mut pending.plan;
        let route = topology.find_route_uncached(plan.origin, plan.destination, &pending.access)?;
        let depart_at = plan.depart_at.max(scheduler.now());
        let arrive_at = checked_arrival(depart_at, &route)?;
        let revision = plan
            .revision
            .checked_add(1)
            .ok_or(MovementError::IdentityExhausted)?;
        cancel_arrival(scheduler, plan.arrival_event)?;
        plan.arrival_event = enqueue_arrival(scheduler, id.actor, arrive_at, pending.priority);
        plan.revision = revision;
        plan.depart_at = depart_at;
        plan.arrive_at = arrive_at;
        plan.route = route;
        pending.topology_identity = Arc::clone(topology.identity());
        Ok(plan.clone())
    }

    /// Resolve a native scheduler dispatch. Unrelated and stale events return none.
    pub fn on_arrival(
        &mut self,
        world: &World,
        topology: &SpatialTopology,
        event: DispatchedEvent,
    ) -> Option<MovementArrival> {
        if event.kind != MOVEMENT_ARRIVAL_EVENT_KIND {
            return None;
        }
        let actor = event.entity?;
        let pending = self.pending.get(&actor)?;
        if event.id != pending.plan.arrival_event
            || event.at != pending.plan.arrive_at
            || event.priority != pending.priority
        {
            return None;
        }
        let pending = self.pending.remove(&actor)?;
        let outcome = if !world.is_alive(actor) {
            self.locations.remove(&actor);
            MovementArrivalOutcome::ActorRetired
        } else if !pending.matches_topology(topology) {
            MovementArrivalOutcome::TopologyChanged
        } else {
            self.locations.insert(actor, pending.plan.destination);
            MovementArrivalOutcome::Arrived
        };
        Some(MovementArrival {
            plan: pending.plan,
            event,
            outcome,
        })
    }

    fn check_handle(&self, id: MovementId) -> Result<(), MovementError> {
        if id.runtime != self.runtime || id.sequence >= self.next_sequence {
            return Err(MovementError::UnknownMovement);
        }
        Ok(())
    }

    fn require_stationary(&self, world: &World, actor: EntityId) -> Result<(), MovementError> {
        if !world.is_alive(actor) {
            return Err(MovementError::ActorRetired(actor));
        }
        if self.pending.contains_key(&actor) {
            return Err(MovementError::AlreadyMoving(actor));
        }
        Ok(())
    }
}

fn checked_arrival(depart_at: SimTime, route: &SpatialRoute) -> Result<SimTime, MovementError> {
    depart_at
        .checked_add(route.duration()?)
        .ok_or(MovementError::ArrivalOverflow)
}

fn enqueue_arrival(
    scheduler: &mut Scheduler,
    actor: EntityId,
    at: SimTime,
    priority: i32,
) -> EventId {
    scheduler.schedule(ScheduleRequest {
        at,
        priority,
        entity: Some(actor),
        kind: MOVEMENT_ARRIVAL_EVENT_KIND,
    })
}

fn cancel_arrival(scheduler: &mut Scheduler, event: EventId) -> Result<(), MovementError> {
    if !scheduler.cancel(event) {
        return Err(MovementError::ArrivalNotPending(event));
    }
    Ok(())
}
