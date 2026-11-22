// SPDX-License-Identifier: Apache-2.0
//
// Append this module to the pinned CareOps allocation_kairos_conformance
// test to reuse its real CareOps grant acknowledgment path.
mod movement_qualification {
    use super::*;
    use careops_core::allocation::AllocationState;
    use kairo_ecs_abm::{
        AccessProfile, CostUnit, EdgeAccessRule, MovementArrivalOutcome, MovementRequest,
        MovementRuntime, SpatialEdgeKind, SpatialNodeKind, SpatialTopology,
    };
    use kairo_ecs_core::Scheduler;
    use kairo_ecs_des::StagedUsageSource;
    use kairo_ecs_state::World;
    use kairo_ecs_types::{EntityId, EventKind, ScheduleRequest, StepOutcome};
    use std::num::NonZeroU128;

    #[test]
    fn arrival_completes_handover_and_releases_porter() {
        journey(false);
    }

    #[test]
    fn cancellation_keeps_patient_location_and_releases_porter() {
        journey(true);
    }

    fn journey(cancel: bool) {
        let tick = SimTime::from_ticks;
        let origin = EntityId::new(100, 0);
        let destination = EntityId::new(200, 0);
        let mut topology = SpatialTopology::new(CostUnit::Ticks);
        topology.add_node(origin, 0, SpatialNodeKind::Room).unwrap();
        topology
            .add_node(destination, 1, SpatialNodeKind::Room)
            .unwrap();
        topology
            .add_edge(
                1,
                origin,
                destination,
                3,
                SpatialEdgeKind::Stairs,
                EdgeAccessRule {
                    step_free: false,
                    ..EdgeAccessRule::default()
                },
            )
            .unwrap();
        topology
            .add_edge(
                2,
                origin,
                destination,
                8,
                SpatialEdgeKind::Lift,
                EdgeAccessRule::default(),
            )
            .unwrap();
        let stair_route = topology
            .find_route_uncached(origin, destination, &AccessProfile::standard())
            .unwrap();
        assert_eq!(stair_route.segments[0].kind, SpatialEdgeKind::Stairs);

        let mut allocator = StagedAllocator::default();
        let operation = allocator
            .submit([CareOpsResourceId::new(77)], None, 1, false)
            .unwrap();
        let claim = allocator.next_claim_requests().unwrap()[0];
        let mut engine = StagedResourceRuntime::new_with_usage(StagedUsageSource {
            run: "careops-movement".into(),
            scenario: "porter-handover".into(),
            branch: "fictional".into(),
            engine_revision: "e2875387ce45c34cd19716ea66f0102251a6aa6f".into(),
            model_revision: "9af99b5730d6aa5cef1c05bd48e90fd377b80cf2".into(),
            rng_revision: "deterministic".into(),
            time_unit: "minute".into(),
            ticks_per_time_unit: NonZeroU128::MIN,
        })
        .unwrap();
        let porter = engine.create_resource_with_unit(1, "porter").unwrap();
        let owner = engine.spawn_actor().unwrap();
        let request = engine.submit(porter, owner, tick(2)).unwrap();
        allocator
            .bind_engine_request(
                claim,
                engine_request_id(request),
                engine_resource_id(porter),
            )
            .unwrap();
        assert_eq!(engine.run_for(1), Ok(1));
        record_observed_grant(&mut allocator, claim, &engine, request, porter);
        assert_eq!(allocator.state(operation), Some(AllocationState::Ready));
        let lease = allocator.claims(operation)[0].1;

        let mut world = World::new();
        let patient = world.spawn();
        let mut scheduler = Scheduler::new();
        let mut movements = MovementRuntime::new().unwrap();
        movements.place(&world, &topology, patient, origin).unwrap();
        let plan = movements
            .schedule(
                &mut scheduler,
                &world,
                &topology,
                MovementRequest {
                    actor: patient,
                    destination,
                    depart_at: engine.now(),
                    access: AccessProfile::step_free(),
                    priority: 0,
                },
            )
            .unwrap();
        assert_eq!(plan.depart_at, tick(2));
        assert_eq!(plan.arrive_at, tick(10));
        assert_eq!(plan.route.segments[0].kind, SpatialEdgeKind::Lift);
        assert_eq!(movements.location(&world, patient), Some(origin));

        let (end, location, state) = if cancel {
            let cancellation = scheduler.schedule(ScheduleRequest {
                at: tick(5),
                priority: 0,
                entity: Some(patient),
                kind: EventKind::custom(42),
            });
            assert!(
                matches!(scheduler.step(), StepOutcome::Dispatched(event) if event.id == cancellation)
            );
            allocator.cancel(operation, 5).unwrap();
            assert_eq!(movements.cancel(&mut scheduler, plan.id), Ok(true));
            assert_eq!(movements.cancel(&mut scheduler, plan.id), Ok(false));
            (tick(5), origin, AllocationState::Cancelled)
        } else {
            let StepOutcome::Dispatched(event) = scheduler.step() else {
                panic!("expected the native arrival");
            };
            assert_eq!(event.id, plan.arrival_event);
            assert_eq!(event.at, tick(10));
            let arrival = movements.on_arrival(&world, &topology, event).unwrap();
            assert_eq!(arrival.plan, plan);
            assert_eq!(arrival.outcome, MovementArrivalOutcome::Arrived);
            assert!(movements.on_arrival(&world, &topology, event).is_none());
            let work = allocator.start(operation).unwrap();
            allocator.complete(operation, work).unwrap();
            (tick(10), destination, AllocationState::Completed)
        };

        assert_eq!(end, scheduler.now());
        assert_eq!(movements.location(&world, patient), Some(location));
        assert!(movements.pending(plan.id).unwrap().is_none());
        assert_eq!(scheduler.step(), StepOutcome::Empty);
        assert_eq!(allocator.releases_due(operation), vec![lease]);
        assert_eq!(engine.close_request(request, end), Ok(true));
        assert_eq!(engine.run_for(1), Ok(1));
        assert_eq!(
            engine.request(request).unwrap().state,
            RequestState::Released
        );
        allocator.ack_release(operation, lease).unwrap();
        assert_eq!(allocator.state(operation), Some(state));
        assert_eq!(allocator.claim_count(), 0);
        assert_eq!(engine.resource(porter).unwrap().available, 1);
        assert_eq!(engine.close_request(request, end), Ok(false));

        let usage = engine.committed_usage(porter, tick(0), end).unwrap();
        assert_eq!(usage.resource, porter.entity_id());
        assert_eq!(usage.quantity_unit, "porter");
        assert_eq!(usage.occupied_unit_ticks, if cancel { 3 } else { 8 });
        assert_eq!(usage.occupancy.len(), 1);
        let interval = &usage.occupancy[0];
        assert_eq!(interval.actor, owner.entity_id());
        assert_eq!(interval.request, request.entity_id());
        assert_eq!(interval.lease_revision, lease.revision());
        assert_eq!(interval.quantity, 1);
        assert_eq!((interval.start, interval.end), (tick(2), end));
        assert_eq!(interval.admission.at, tick(2));
        assert_eq!(interval.release.unwrap().at, end);
    }
}
