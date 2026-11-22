use kairo_ecs_abm::{
    AccessProfile, CostUnit, EdgeAccessRule, MovementArrivalOutcome, MovementError, MovementPlan,
    MovementRequest, MovementRuntime, RoutingError, SpatialEdgeKind, SpatialNodeKind,
    SpatialTopology,
};
use kairo_ecs_core::Scheduler;
use kairo_ecs_state::World;
use kairo_ecs_types::{
    DispatchedEvent, EntityId, EventKind, ScheduleRequest, SimTime, StepOutcome,
};

const ORIGIN: EntityId = EntityId::new(10, 0);
const WAYPOINT: EntityId = EntityId::new(11, 0);
const DESTINATION: EntityId = EntityId::new(12, 0);

fn t(ticks: u128) -> SimTime {
    SimTime::from_ticks(ticks)
}

fn topology(unit: CostUnit) -> SpatialTopology {
    let mut topology = SpatialTopology::new(unit);
    for node in [ORIGIN, WAYPOINT, DESTINATION] {
        topology.add_node(node, 0, SpatialNodeKind::Room).unwrap();
    }
    add_edge(&mut topology, 1, ORIGIN, DESTINATION, 7);
    topology
}

fn add_edge(topology: &mut SpatialTopology, id: u64, from: EntityId, to: EntityId, cost: u64) {
    topology
        .add_edge(
            id,
            from,
            to,
            cost,
            SpatialEdgeKind::Doorway,
            EdgeAccessRule::default(),
        )
        .unwrap();
}

struct Model {
    topology: SpatialTopology,
    scheduler: Scheduler,
    world: World,
    movement: MovementRuntime,
    actor: EntityId,
}

impl Model {
    fn new(unit: CostUnit) -> Self {
        let topology = topology(unit);
        let mut world = World::new();
        let actor = world.spawn();
        let mut movement = MovementRuntime::new().unwrap();
        movement.place(&world, &topology, actor, ORIGIN).unwrap();
        Self {
            topology,
            scheduler: Scheduler::new(),
            world,
            movement,
            actor,
        }
    }

    fn schedule(
        &mut self,
        depart_at: u128,
        destination: EntityId,
    ) -> Result<MovementPlan, MovementError> {
        self.movement.schedule(
            &mut self.scheduler,
            &self.world,
            &self.topology,
            MovementRequest {
                actor: self.actor,
                destination,
                depart_at: t(depart_at),
                access: AccessProfile::standard(),
                priority: 0,
            },
        )
    }

    fn unrelated_at(&mut self, at: u128, priority: i32) {
        self.scheduler.schedule(ScheduleRequest {
            at: t(at),
            priority,
            entity: Some(self.actor),
            kind: EventKind::custom(32),
        });
    }

    fn step(&mut self) -> DispatchedEvent {
        let StepOutcome::Dispatched(event) = self.scheduler.step() else {
            panic!("expected scheduled event");
        };
        event
    }

    fn location(&self) -> Option<EntityId> {
        self.movement.location(&self.world, self.actor)
    }
}

#[test]
fn arrival_changes_location_once_at_exact_scheduler_tick() {
    let mut model = Model::new(CostUnit::Ticks);
    let stats = model.scheduler.stats();
    assert_eq!(
        model
            .topology
            .find_route_uncached(ORIGIN, DESTINATION, &AccessProfile::standard())
            .unwrap()
            .total_cost,
        7
    );
    assert_eq!(model.scheduler.stats(), stats);

    let plan = model.schedule(10, DESTINATION).unwrap();
    assert_eq!(plan.arrive_at, t(17));
    assert_eq!(plan.id.actor(), model.actor);
    assert_eq!(
        model.schedule(11, DESTINATION),
        Err(MovementError::AlreadyMoving(model.actor))
    );
    assert_eq!(
        model
            .movement
            .place(&model.world, &model.topology, model.actor, WAYPOINT),
        Err(MovementError::AlreadyMoving(model.actor))
    );
    model.unrelated_at(16, 0);
    let prior = model.step();
    assert_eq!(prior.at, t(16));
    assert_eq!(
        model
            .movement
            .on_arrival(&model.world, &model.topology, prior),
        None
    );
    assert_eq!(model.location(), Some(ORIGIN));
    let arrival = model.step();
    assert_eq!(arrival.id, plan.arrival_event);
    assert_eq!(
        model.movement.cancel(&mut model.scheduler, plan.id),
        Err(MovementError::ArrivalNotPending(arrival.id))
    );
    let outcome = model
        .movement
        .on_arrival(&model.world, &model.topology, arrival)
        .unwrap();
    assert_eq!(outcome.plan, plan);
    assert_eq!(outcome.event, arrival);
    assert_eq!(outcome.outcome, MovementArrivalOutcome::Arrived);
    assert_eq!(model.location(), Some(DESTINATION));
    assert_eq!(
        model
            .movement
            .on_arrival(&model.world, &model.topology, arrival),
        None
    );
    assert_eq!(model.movement.pending(plan.id), Ok(None));
    assert_eq!(
        model.movement.cancel(&mut model.scheduler, plan.id),
        Ok(false)
    );
}

#[test]
fn zero_duration_arrival_respects_same_tick_priority_and_insertion() {
    let mut model = Model::new(CostUnit::Ticks);
    model.unrelated_at(0, 0);
    let plan = model.schedule(0, ORIGIN).unwrap();
    model.unrelated_at(0, -1);
    model.unrelated_at(0, 0);
    assert_eq!(model.step().priority, -1);
    assert_eq!(model.step().kind, EventKind::custom(32));
    let arrival = model.step();
    assert_eq!(arrival.at, t(0));
    assert_eq!(arrival.id, plan.arrival_event);
    assert_eq!(
        model
            .movement
            .on_arrival(&model.world, &model.topology, arrival)
            .unwrap()
            .outcome,
        MovementArrivalOutcome::Arrived
    );
    assert_eq!(model.step().kind, EventKind::custom(32));
    assert_eq!(model.scheduler.step(), StepOutcome::Empty);
}

#[test]
fn invalid_arrival_requests_leave_placement_and_queue_unchanged() {
    let mut model = Model::new(CostUnit::Ticks);
    model.unrelated_at(3, 0);
    model.step();
    let stats = model.scheduler.stats();
    assert_eq!(
        model.schedule(2, DESTINATION),
        Err(MovementError::DepartureInPast {
            depart_at: t(2),
            now: t(3)
        })
    );
    assert_eq!(
        model.schedule(u128::MAX, DESTINATION),
        Err(MovementError::ArrivalOverflow)
    );
    assert!(matches!(
        model.schedule(3, WAYPOINT),
        Err(MovementError::Route(RoutingError::NoRouteFound { .. }))
    ));
    assert_eq!(model.scheduler.stats(), stats);
    assert_eq!(model.location(), Some(ORIGIN));

    let mut model = Model::new(CostUnit::Meters);
    assert_eq!(
        model.schedule(0, DESTINATION),
        Err(MovementError::Route(RoutingError::UnitMismatch {
            expected: CostUnit::Ticks,
            actual: CostUnit::Meters,
        }))
    );
    assert_eq!(model.scheduler.pending_events(), 0);
    let actor = model.world.spawn();
    assert_eq!(
        model
            .movement
            .place(&model.world, &model.topology, actor, EntityId::new(42, 0)),
        Err(MovementError::Route(RoutingError::NodeNotFound(
            EntityId::new(42, 0)
        )))
    );
    model.actor = actor;
    assert_eq!(
        model.schedule(0, DESTINATION),
        Err(MovementError::ActorUnplaced(actor))
    );
}

#[test]
fn cancellation_cannot_dispatch_or_cancel_a_replacement_move() {
    let mut model = Model::new(CostUnit::Ticks);
    let old = model.schedule(10, DESTINATION).unwrap();
    let mut foreign = MovementRuntime::new().unwrap();
    assert_eq!(
        foreign.cancel(&mut model.scheduler, old.id),
        Err(MovementError::UnknownMovement)
    );
    assert_eq!(
        model.movement.cancel(&mut model.scheduler, old.id),
        Ok(true)
    );
    assert_eq!(
        model.movement.cancel(&mut model.scheduler, old.id),
        Ok(false)
    );
    let new = model.schedule(20, DESTINATION).unwrap();
    assert_eq!(
        model.movement.cancel(&mut model.scheduler, old.id),
        Ok(false)
    );
    let checkpoint = model.scheduler.checkpoint();
    let mut replay = Scheduler::from_checkpoint(&checkpoint).unwrap();
    let mut movement = model.movement.clone();
    let arrival = model.step();
    assert_eq!(arrival.id, new.arrival_event);
    assert_eq!(arrival.at, t(27));
    assert_eq!(replay.step(), StepOutcome::Dispatched(arrival));
    assert_eq!(
        movement.on_arrival(&model.world, &model.topology, arrival),
        model
            .movement
            .on_arrival(&model.world, &model.topology, arrival),
    );
    assert_eq!(model.scheduler.step(), StepOutcome::Empty);
}

#[test]
fn cancellation_and_retirement_respect_entity_generation() {
    let mut model = Model::new(CostUnit::Ticks);
    let retired = model.actor;
    let old = model.schedule(0, DESTINATION).unwrap();
    assert!(model.world.despawn(retired));
    let replacement = model.world.spawn();
    assert_eq!(replacement.index, retired.index);
    assert_ne!(replacement.generation, retired.generation);
    assert_eq!(model.location(), None);
    assert_eq!(
        model.schedule(0, DESTINATION),
        Err(MovementError::ActorRetired(retired))
    );
    model.actor = replacement;
    model
        .movement
        .place(&model.world, &model.topology, replacement, ORIGIN)
        .unwrap();
    let arrival = model.step();
    let outcome = model
        .movement
        .on_arrival(&model.world, &model.topology, arrival)
        .unwrap();
    assert_eq!(outcome.plan.id, old.id);
    assert_eq!(outcome.outcome, MovementArrivalOutcome::ActorRetired);
    assert_eq!(model.location(), Some(ORIGIN));
    let pending = model.schedule(7, DESTINATION).unwrap();
    assert_eq!(
        model
            .movement
            .forget_actor(&mut model.scheduler, replacement),
        Ok(Some(pending.id))
    );
    assert_eq!(
        model
            .movement
            .forget_actor(&mut model.scheduler, replacement),
        Ok(None)
    );
    assert_eq!(model.location(), None);
    assert_eq!(model.scheduler.step(), StepOutcome::Empty);
}

#[test]
fn replan_replaces_old_arrival_and_preserves_native_order() {
    let mut model = Model::new(CostUnit::Ticks);
    add_edge(&mut model.topology, 2, ORIGIN, WAYPOINT, 4);
    add_edge(&mut model.topology, 3, WAYPOINT, DESTINATION, 8);
    let old = model.schedule(10, DESTINATION).unwrap();
    assert_eq!(
        model
            .movement
            .replan(&mut model.scheduler, &model.world, &model.topology, old.id),
        Ok(old.clone())
    );
    model.unrelated_at(13, 0);
    model.step();
    model.topology.set_door_state(1, false).unwrap();
    model.unrelated_at(25, 0);
    let new = model
        .movement
        .replan(&mut model.scheduler, &model.world, &model.topology, old.id)
        .unwrap();
    assert_eq!(new.id, old.id);
    assert_eq!(new.revision, 1);
    assert_eq!(new.route.path, [ORIGIN, WAYPOINT, DESTINATION]);
    assert_eq!(new.depart_at, t(13));
    assert_eq!(new.arrive_at, t(25));
    assert_ne!(new.arrival_event, old.arrival_event);
    assert_eq!(
        model.movement.on_arrival(
            &model.world,
            &model.topology,
            DispatchedEvent {
                id: old.arrival_event,
                at: old.arrive_at,
                priority: 0,
                sequence: 0,
                entity: Some(model.actor),
                kind: kairo_ecs_abm::MOVEMENT_ARRIVAL_EVENT_KIND,
            }
        ),
        None
    );
    assert_eq!(model.step().kind, EventKind::custom(32));
    assert_eq!(model.location(), Some(ORIGIN));
    let arrival = model.step();
    assert_eq!(arrival.id, new.arrival_event);
    assert_eq!(
        model
            .movement
            .on_arrival(&model.world, &model.topology, arrival)
            .unwrap()
            .outcome,
        MovementArrivalOutcome::Arrived
    );
    assert_eq!(model.location(), Some(DESTINATION));
    assert_eq!(model.scheduler.step(), StepOutcome::Empty);
}

#[test]
fn failed_replan_invalidates_old_route_without_queuing_another_arrival() {
    let mut model = Model::new(CostUnit::Ticks);
    let old = model.schedule(0, DESTINATION).unwrap();
    model.topology.set_door_state(1, false).unwrap();
    let stats = model.scheduler.stats();
    assert!(matches!(
        model
            .movement
            .replan(&mut model.scheduler, &model.world, &model.topology, old.id),
        Err(MovementError::Route(RoutingError::NoRouteFound { .. }))
    ));
    assert_eq!(model.scheduler.stats(), stats);
    assert_eq!(model.movement.pending(old.id), Ok(Some(&old)));
    let event = model.step();
    assert_eq!(
        model
            .movement
            .on_arrival(&model.world, &model.topology, event)
            .unwrap()
            .outcome,
        MovementArrivalOutcome::TopologyChanged
    );
    assert_eq!(model.location(), Some(ORIGIN));
    assert_eq!(model.scheduler.step(), StepOutcome::Empty);
}

#[test]
fn topology_replacement_with_same_revision_is_detected() {
    let mut model = Model::new(CostUnit::Ticks);
    let old = model.schedule(10, DESTINATION).unwrap();
    model.topology = topology(CostUnit::Ticks);
    assert_eq!(model.topology.version(), old.route.topology_version);
    let replan = model
        .movement
        .replan(&mut model.scheduler, &model.world, &model.topology, old.id)
        .unwrap();
    assert_eq!(replan.revision, 1);
    assert_eq!(replan.depart_at, t(10));
    assert_eq!(replan.arrive_at, t(17));
    model.topology = topology(CostUnit::Ticks);
    let event = model.step();
    assert_eq!(event.id, replan.arrival_event);
    assert_eq!(
        model
            .movement
            .on_arrival(&model.world, &model.topology, event)
            .unwrap()
            .outcome,
        MovementArrivalOutcome::TopologyChanged
    );
}
