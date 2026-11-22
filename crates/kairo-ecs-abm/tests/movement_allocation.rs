use kairo_ecs_abm::{
    AccessProfile, CostUnit, EdgeAccessRule, MovementArrival, MovementArrivalOutcome,
    MovementError, MovementPlan, MovementRequest, MovementRuntime, RoutingError, SpatialEdgeKind,
    SpatialNodeKind, SpatialTopology,
};
use kairo_ecs_core::Scheduler;
use kairo_ecs_des::{
    StagedRequestId, StagedRequestState, StagedResourceId, StagedResourceRuntime, StagedUsageSource,
};
use kairo_ecs_state::World;
use kairo_ecs_types::{EntityId, EventKind, ScheduleRequest, SimTime, StepOutcome};
use std::num::NonZeroU128;

const ORIGIN: EntityId = EntityId::new(1, 0);
const DESTINATION: EntityId = EntityId::new(2, 0);
const DOOR: u64 = 9;

fn tick(value: u128) -> SimTime {
    SimTime::from_ticks(value)
}

struct Journey {
    scheduler: Scheduler,
    movement: MovementRuntime,
    world: World,
    topology: SpatialTopology,
    actor: EntityId,
    allocation: StagedResourceRuntime,
    porter: StagedResourceId,
    request: StagedRequestId,
}

impl Journey {
    fn new() -> Self {
        let mut topology = SpatialTopology::new(CostUnit::Ticks);
        for node in [ORIGIN, DESTINATION] {
            topology.add_node(node, 0, SpatialNodeKind::Room).unwrap();
        }
        topology
            .add_edge(
                DOOR,
                ORIGIN,
                DESTINATION,
                8,
                SpatialEdgeKind::Doorway,
                EdgeAccessRule::default(),
            )
            .unwrap();
        let mut world = World::new();
        let actor = world.spawn();
        let mut movement = MovementRuntime::new().unwrap();
        movement.place(&world, &topology, actor, ORIGIN).unwrap();
        let mut allocation = StagedResourceRuntime::new_with_usage(StagedUsageSource {
            run: "movement-allocation".into(),
            scenario: "porter-assisted-journey".into(),
            branch: "fixture".into(),
            engine_revision: "fixture".into(),
            model_revision: "fixture".into(),
            rng_revision: "deterministic".into(),
            time_unit: "minute".into(),
            ticks_per_time_unit: NonZeroU128::MIN,
        })
        .unwrap();
        let porter = allocation.create_resource_with_unit(1, "porter").unwrap();
        let staged_actor = allocation.spawn_actor().unwrap();
        let request = allocation.submit(porter, staged_actor, tick(2)).unwrap();
        Self {
            scheduler: Scheduler::new(),
            movement,
            world,
            topology,
            actor,
            allocation,
            porter,
            request,
        }
    }

    fn drain_allocation_through(&mut self, at: SimTime) {
        while self
            .allocation
            .next_event_time()
            .is_some_and(|event| event <= at)
        {
            assert_eq!(self.allocation.run_for(1), Ok(1));
        }
    }

    fn start(&mut self) -> Result<MovementPlan, MovementError> {
        self.at_model_event(2);
        assert_eq!(
            self.allocation.request(self.request).unwrap().state,
            StagedRequestState::Active
        );
        let now = self.scheduler.now();
        let plan = self.movement.schedule(
            &mut self.scheduler,
            &self.world,
            &self.topology,
            MovementRequest {
                actor: self.actor,
                destination: DESTINATION,
                depart_at: now,
                access: AccessProfile::standard(),
                priority: 0,
            },
        );
        if plan.is_err() {
            self.close(now);
        }
        plan
    }

    fn at_model_event(&mut self, at: u128) {
        let id = self.scheduler.schedule(ScheduleRequest {
            at: tick(at),
            priority: 0,
            entity: Some(self.actor),
            kind: EventKind::custom(42),
        });
        assert!(matches!(self.scheduler.step(), StepOutcome::Dispatched(event) if event.id == id));
        self.drain_allocation_through(tick(at));
    }

    fn dispatch_arrival(&mut self) -> MovementArrival {
        let StepOutcome::Dispatched(event) = self.scheduler.step() else {
            panic!("arrival must be queued");
        };
        self.drain_allocation_through(event.at);
        let resolved = self
            .movement
            .on_arrival(&self.world, &self.topology, event)
            .unwrap();
        self.close(event.at);
        resolved
    }

    fn close(&mut self, at: SimTime) {
        assert_eq!(self.allocation.close_request(self.request, at), Ok(true));
        self.drain_allocation_through(at);
    }

    fn verify_completion(&mut self, location: Option<EntityId>, duration: u128, end: u128) {
        assert_eq!(self.movement.location(&self.world, self.actor), location);
        assert_eq!(self.scheduler.step(), StepOutcome::Empty);
        let resource = self.allocation.resource(self.porter).unwrap();
        assert_eq!(resource.available, 1);
        assert!(resource.active.is_empty() && resource.queued.is_empty());
        assert_eq!(
            self.allocation.close_request(self.request, tick(end)),
            Ok(false)
        );
        assert_eq!(self.allocation.next_event_time(), None);
        let window = self
            .allocation
            .committed_usage(self.porter, tick(0), tick(end))
            .unwrap();
        assert_eq!(window.occupied_unit_ticks, duration);
        if duration == 0 {
            assert!(window.occupancy.is_empty());
        } else {
            assert_eq!(window.occupancy.len(), 1);
            assert_eq!(window.occupancy[0].admission.at, tick(2));
            assert_eq!(window.occupancy[0].release.unwrap().at, tick(end));
        }
    }
}

#[test]
fn arrival_releases_porter_at_its_dispatch_tick() {
    let mut journey = Journey::new();
    journey.start().unwrap();
    assert_eq!(
        journey.dispatch_arrival().outcome,
        MovementArrivalOutcome::Arrived
    );
    journey.verify_completion(Some(DESTINATION), 8, 10);
}

#[test]
fn cancellation_releases_porter_without_later_dispatch() {
    let mut journey = Journey::new();
    let plan = journey.start().unwrap();
    journey.at_model_event(5);
    assert_eq!(
        journey.movement.cancel(&mut journey.scheduler, plan.id),
        Ok(true)
    );
    journey.close(tick(5));
    journey.verify_completion(Some(ORIGIN), 3, 5);
}

#[test]
fn no_replacement_route_closes_the_associated_lease() {
    let mut journey = Journey::new();
    let plan = journey.start().unwrap();
    journey.at_model_event(5);
    journey.topology.set_door_state(DOOR, false).unwrap();
    assert!(matches!(
        journey.movement.replan(
            &mut journey.scheduler,
            &journey.world,
            &journey.topology,
            plan.id
        ),
        Err(MovementError::Route(RoutingError::NoRouteFound { .. }))
    ));
    assert_eq!(
        journey.movement.cancel(&mut journey.scheduler, plan.id),
        Ok(true)
    );
    journey.close(tick(5));
    journey.verify_completion(Some(ORIGIN), 3, 5);
}

#[test]
fn successful_replan_keeps_the_same_lease_until_new_arrival() {
    let mut journey = Journey::new();
    let plan = journey.start().unwrap();
    journey.at_model_event(5);
    journey.topology.set_edge_cost(DOOR, 7).unwrap();
    let replacement = journey
        .movement
        .replan(
            &mut journey.scheduler,
            &journey.world,
            &journey.topology,
            plan.id,
        )
        .unwrap();
    assert_eq!(replacement.id, plan.id);
    assert_eq!(replacement.arrive_at, tick(12));
    assert_eq!(
        journey
            .allocation
            .resource(journey.porter)
            .unwrap()
            .available,
        0
    );
    assert_eq!(
        journey.dispatch_arrival().outcome,
        MovementArrivalOutcome::Arrived
    );
    journey.verify_completion(Some(DESTINATION), 10, 12);
}

#[test]
fn retiring_actor_cancels_movement_and_releases_porter_immediately() {
    let mut journey = Journey::new();
    let plan = journey.start().unwrap();
    journey.at_model_event(5);
    assert!(journey.world.despawn(journey.actor));
    assert_eq!(
        journey
            .movement
            .forget_actor(&mut journey.scheduler, journey.actor),
        Ok(Some(plan.id))
    );
    journey.close(tick(5));
    journey.verify_completion(None, 3, 5);
}

#[test]
fn failed_start_releases_the_lease_at_the_admission_tick() {
    let mut journey = Journey::new();
    journey.topology.set_door_state(DOOR, false).unwrap();
    assert!(matches!(
        journey.start(),
        Err(MovementError::Route(RoutingError::NoRouteFound { .. }))
    ));
    journey.verify_completion(Some(ORIGIN), 0, 2);
}
