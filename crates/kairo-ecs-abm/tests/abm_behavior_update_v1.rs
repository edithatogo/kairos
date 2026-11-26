use kairo_ecs_abm::{
    AgentBehavior, BehaviorContext, BehaviorDecision, BehaviorSimulation,
    BEHAVIOR_UPDATE_EVENT_KIND,
};
use kairo_ecs_des::{
    BoundaryChannel, DESContext, HandoverEnvelope, HandoverOperationId, StagedDispatchPreview,
    StagedDispatchSelection, StagedRequestState, StagedResourceRuntime, StagedRuntimeError,
    TransferState, TransitDelayDistribution,
};
use kairo_ecs_rng::DeterministicStream;
use kairo_ecs_types::{EntityId, EventKind, SimTime};

#[derive(Default)]
struct RecordingBehavior {
    updates: Vec<(u128, u32, u64)>,
}

impl AgentBehavior for RecordingBehavior {
    fn update(&mut self, context: BehaviorContext<'_>) -> BehaviorDecision {
        self.updates.push((
            context.event.at.ticks(),
            context.event.kind.code(),
            context.rng.next_u64(),
        ));
        BehaviorDecision::Continue
    }
}

#[test]
fn abm_behavior_update_v1_runs_updates_in_scheduler_order() {
    let mut simulation = BehaviorSimulation::new(7, RecordingBehavior::default());
    let first = simulation.spawn_agent();
    let second = simulation.spawn_agent();

    simulation.schedule_update(first, SimTime::from_ticks(30));
    simulation.schedule_update(second, SimTime::from_ticks(10));
    simulation.schedule_update(first, SimTime::from_ticks(20));

    let trace = simulation.run_for(3);
    let (_context, behavior) = simulation.into_parts();

    assert_eq!(trace.len(), 3);
    assert_eq!(
        behavior
            .updates
            .iter()
            .map(|(ticks, kind, _rng)| (*ticks, *kind))
            .collect::<Vec<_>>(),
        vec![
            (10, BEHAVIOR_UPDATE_EVENT_KIND),
            (20, BEHAVIOR_UPDATE_EVENT_KIND),
            (30, BEHAVIOR_UPDATE_EVENT_KIND),
        ]
    );
}

#[test]
fn abm_behavior_update_v1_replays_entity_rng_streams() {
    let mut first_run = BehaviorSimulation::new(99, RecordingBehavior::default());
    let first_agent = first_run.spawn_agent();
    first_run.schedule_update(first_agent, SimTime::from_ticks(1));
    let _ = first_run.run_for(1);
    let (_first_context, first_behavior) = first_run.into_parts();

    let mut second_run = BehaviorSimulation::new(99, RecordingBehavior::default());
    let second_agent = second_run.spawn_agent();
    second_run.schedule_update(second_agent, SimTime::from_ticks(1));
    let _ = second_run.run_for(1);
    let (_second_context, second_behavior) = second_run.into_parts();

    assert_eq!(first_agent, second_agent);
    assert_eq!(first_behavior.updates[0].2, second_behavior.updates[0].2);
}

#[test]
fn staged_allocator_preserves_native_behavior_dispatch() {
    let tick = SimTime::from_ticks;
    let mut simulation = BehaviorSimulation::new(7, RecordingBehavior::default());
    let agent = simulation.spawn_agent();
    let cancelled = simulation.schedule_update(agent, tick(1));
    assert!(simulation.context_mut().scheduler.cancel(cancelled));
    let behavior_event = simulation.schedule_update(agent, tick(4));
    let model_before = simulation.context().scheduler.stats();

    let mut allocator = StagedResourceRuntime::new().unwrap();
    let resource = allocator.create_resource(1).unwrap();
    let owner = allocator.spawn_actor().unwrap();
    let next_actor = allocator.spawn_actor().unwrap();
    let request = allocator.submit(resource, owner, tick(4)).unwrap();
    let later = allocator.submit(resource, next_actor, tick(6)).unwrap();

    assert_eq!(
        allocator.peek_with_scheduler(&mut simulation.context_mut().scheduler, tick(3)),
        Ok(None)
    );
    assert_eq!(
        allocator.peek_with_scheduler(&mut simulation.context_mut().scheduler, tick(4)),
        Ok(Some(StagedDispatchPreview::Allocation(tick(4))))
    );
    assert_eq!(allocator.now(), tick(0));
    assert_eq!(
        allocator.request(request).unwrap().state,
        StagedRequestState::Scheduled
    );
    assert_eq!(allocator.run_until_or_for(tick(4), 1), Ok(1));
    assert_eq!(
        allocator.request(request).unwrap().state,
        StagedRequestState::Active
    );

    let Some(StagedDispatchPreview::Model(next)) = allocator
        .peek_with_scheduler(&mut simulation.context_mut().scheduler, tick(6))
        .unwrap()
    else {
        panic!("the native behavior update must precede the tick-six allocation");
    };
    assert_eq!(next.id, behavior_event);
    assert_eq!(next.at, tick(4));
    assert_eq!(simulation.context().scheduler.stats(), model_before);
    assert_eq!(allocator.next_event_time(), Some(tick(6)));

    let trace = simulation.run_for(1);
    let [update] = trace.updates() else {
        panic!("the native behavior handler must run exactly once");
    };
    assert_eq!(update.event.id, next.id);
    assert_eq!(update.agent, agent);
    assert_eq!(update.at, next.at);
    assert_eq!(allocator.close_request(request, update.at), Ok(true));
    assert_eq!(
        allocator.peek_with_scheduler(&mut simulation.context_mut().scheduler, tick(6)),
        Ok(Some(StagedDispatchPreview::Allocation(update.at)))
    );
    assert_eq!(allocator.run_until_or_for(update.at, 1), Ok(1));
    assert_eq!(allocator.resource(resource).unwrap().available, 1);
    assert_eq!(
        allocator.request(later).unwrap().state,
        StagedRequestState::Scheduled
    );

    let (_, behavior) = simulation.into_parts();
    let [(at, kind, _)] = behavior.updates.as_slice() else {
        panic!("expected one recorded behavior update");
    };
    assert_eq!((*at, *kind), (4, BEHAVIOR_UPDATE_EVENT_KIND));
}

#[test]
fn staged_allocator_coordinates_separate_des_and_abm_queues() {
    let at = SimTime::from_ticks(4);
    let later = SimTime::from_ticks(6);
    let mut des = DESContext::new(7);
    let completion_id = des.schedule_at(at, 10, EventKind::custom(82));
    let future_des_id = des.schedule_at(later, 0, EventKind::custom(82));
    let mut abm = BehaviorSimulation::new(7, RecordingBehavior::default());
    let agent = abm.spawn_agent();
    let update_id = abm.schedule_update(agent, at);
    assert_eq!(completion_id, update_id);

    let mut allocator = StagedResourceRuntime::new().unwrap();
    let resource = allocator.create_resource(1).unwrap();
    let actor = allocator.spawn_actor().unwrap();
    let future_actor = allocator.spawn_actor().unwrap();
    let request = allocator.submit(resource, actor, at).unwrap();
    let future_request = allocator.submit(resource, future_actor, later).unwrap();

    assert_eq!(
        allocator.peek_with_schedulers(
            &mut [&mut des.scheduler, &mut abm.context_mut().scheduler],
            at,
        ),
        Ok(Some(StagedDispatchSelection::Allocation(at)))
    );
    assert_eq!(allocator.run_until_or_for(at, 1), Ok(1));
    assert_eq!(
        allocator.request(request).unwrap().state,
        StagedRequestState::Active
    );

    let Some(StagedDispatchSelection::Model {
        scheduler_index: 0,
        event,
    }) = allocator
        .peek_with_schedulers(
            &mut [&mut des.scheduler, &mut abm.context_mut().scheduler],
            at,
        )
        .unwrap()
    else {
        panic!("the DES queue must have precedence at the shared tick");
    };
    let completion = des.step().into_dispatched().unwrap();
    assert_eq!(
        (event.id, event.at, event.kind),
        (completion_id, at, EventKind::custom(82))
    );
    assert_eq!((completion.id, completion.at), (event.id, event.at));
    let follow_up_id = abm.schedule_update(agent, completion.at);

    let Some(StagedDispatchSelection::Model {
        scheduler_index: 1,
        event,
    }) = allocator
        .peek_with_schedulers(
            &mut [&mut des.scheduler, &mut abm.context_mut().scheduler],
            later,
        )
        .unwrap()
    else {
        panic!("the earlier ABM event must win even when all queues are eligible");
    };
    assert_eq!(
        (event.id, event.at, event.kind.code()),
        (update_id, at, BEHAVIOR_UPDATE_EVENT_KIND)
    );
    let behavior = abm.run_for(1);
    let [update] = behavior.updates() else {
        panic!("the ABM runner must invoke one behavior");
    };
    assert_eq!((update.event.id, update.at), (event.id, event.at));
    assert_eq!(allocator.close_request(request, update.at), Ok(true));
    assert_eq!(
        allocator.peek_with_schedulers(
            &mut [&mut des.scheduler, &mut abm.context_mut().scheduler],
            at,
        ),
        Ok(Some(StagedDispatchSelection::Allocation(at)))
    );
    assert_eq!(allocator.run_until_or_for(at, 1), Ok(1));
    assert_eq!(allocator.resource(resource).unwrap().available, 1);
    assert!(matches!(
        allocator.peek_with_schedulers(
            &mut [&mut des.scheduler, &mut abm.context_mut().scheduler],
            at,
        ),
        Ok(Some(StagedDispatchSelection::Model { scheduler_index: 1, event }))
            if event.id == follow_up_id
    ));
    let follow_up = abm.run_for(1);
    assert_eq!(follow_up.updates()[0].event.id, follow_up_id);
    assert_eq!(
        allocator.peek_with_schedulers(
            &mut [&mut des.scheduler, &mut abm.context_mut().scheduler],
            at,
        ),
        Ok(None)
    );
    assert_eq!(des.scheduler.peek_next().unwrap().id, future_des_id);
    assert_eq!(
        allocator.request(future_request).unwrap().state,
        StagedRequestState::Scheduled
    );
}

#[test]
fn coordinated_preview_checks_the_clock_of_an_idle_model_queue() {
    let tick = SimTime::from_ticks;
    let mut des = DESContext::new(7);
    des.schedule_at(tick(8), 0, EventKind::custom(82));
    assert_eq!(des.step().into_dispatched().unwrap().at, tick(8));
    assert_eq!(des.scheduler.pending_events(), 0);

    let mut abm = BehaviorSimulation::new(7, RecordingBehavior::default());
    let agent = abm.spawn_agent();
    let backdated = abm.schedule_update(agent, tick(7));
    let model_before = abm.context().scheduler.stats();
    let mut allocator = StagedResourceRuntime::new().unwrap();

    assert_eq!(
        allocator.peek_with_schedulers(
            &mut [&mut abm.context_mut().scheduler, &mut des.scheduler],
            tick(8),
        ),
        Err(StagedRuntimeError::InvalidState)
    );
    assert_eq!(abm.context().scheduler.stats(), model_before);
    assert_eq!(
        abm.context_mut().scheduler.peek_next().unwrap().id,
        backdated
    );
}

#[test]
fn boundary_wakeups_coordinate_with_native_abm_and_lease_release() {
    let tick = SimTime::from_ticks;
    let mut des = DESContext::new(7);
    let mut abm = BehaviorSimulation::new(7, RecordingBehavior::default());
    let recipient = abm.spawn_agent();
    let observer = abm.spawn_agent();
    let observer_event = abm.schedule_update(observer, tick(6));
    let mut allocation = StagedResourceRuntime::new().unwrap();
    let resource = allocation.create_resource(1).unwrap();
    let actor = allocation.spawn_actor().unwrap();
    let request = allocation.submit(resource, actor, tick(2)).unwrap();
    assert_eq!(allocation.run_until_or_for(tick(2), 1), Ok(1));

    let mut channel = BoundaryChannel::new("handover", 3, TransitDelayDistribution::Fixed(tick(5)));
    let accepted = HandoverOperationId::new(1);
    let expired = HandoverOperationId::new(2);
    let expired_at_arrival = HandoverOperationId::new(3);
    let mut rng = DeterministicStream::new(7);
    for (operation, entity, deadline) in [
        (accepted, recipient, 7),
        (expired, EntityId::new(9, 0), 4),
        (expired_at_arrival, EntityId::new(10, 0), 6),
    ] {
        channel
            .request_transfer(HandoverEnvelope::new(
                operation,
                entity,
                (),
                "sender",
                "receiver",
                allocation.now(),
                Some(tick(deadline)),
            ))
            .unwrap();
        assert_eq!(
            channel.depart(operation, allocation.now(), &mut rng),
            Ok(tick(7))
        );
    }

    let preview = |allocation: &mut StagedResourceRuntime,
                   des: &mut DESContext,
                   abm: &mut BehaviorSimulation<RecordingBehavior>| {
        allocation
            .peek_with_schedulers(
                &mut [&mut des.scheduler, &mut abm.context_mut().scheduler],
                tick(7),
            )
            .unwrap()
    };
    let boundary_event = EventKind::custom(83);
    let expiry_event = des.schedule_at(channel.next_event_time().unwrap(), 0, boundary_event);
    assert!(matches!(
        preview(&mut allocation, &mut des, &mut abm),
        Some(StagedDispatchSelection::Model { scheduler_index: 0, event })
            if event.id == expiry_event && event.at == tick(5)
    ));
    let expiry = des.step().into_dispatched().unwrap();
    assert_eq!(expiry.id, expiry_event);
    assert_eq!(channel.check_expiries(expiry.at), [expired]);
    assert!(channel.collect_arrivals(expiry.at).is_empty());
    assert_eq!(channel.available_channel_capacity(), 1);

    let arrival_event = des.schedule_at(channel.next_event_time().unwrap(), 0, boundary_event);
    assert!(matches!(
        preview(&mut allocation, &mut des, &mut abm),
        Some(StagedDispatchSelection::Model { scheduler_index: 1, event })
            if event.id == observer_event && event.at == tick(6)
    ));
    assert_eq!(abm.run_for(1).updates()[0].event.id, observer_event);
    assert!(matches!(
        preview(&mut allocation, &mut des, &mut abm),
        Some(StagedDispatchSelection::Model { scheduler_index: 0, event })
            if event.id == arrival_event && event.at == tick(7)
    ));
    let arrival = des.step().into_dispatched().unwrap();
    assert_eq!(arrival.id, arrival_event);
    assert_eq!(channel.check_expiries(arrival.at), [expired_at_arrival]);
    let arrivals = channel.collect_arrivals(arrival.at);
    let [delivery] = arrivals.as_slice() else {
        panic!("only the surviving transfer can arrive");
    };
    assert_eq!(delivery.operation_id, accepted);
    assert_eq!(delivery.state, TransferState::Arrived);
    assert_eq!(channel.next_event_time(), Some(tick(8)));
    assert_eq!(
        allocation.request(request).unwrap().state,
        StagedRequestState::Active
    );

    let receipt = abm.schedule_update(delivery.entity, arrival.at);
    assert!(matches!(
        preview(&mut allocation, &mut des, &mut abm),
        Some(StagedDispatchSelection::Model { scheduler_index: 1, event })
            if event.id == receipt && event.at == tick(7)
    ));
    let behavior = abm.run_for(1);
    let [update] = behavior.updates() else {
        panic!("the receiver must run exactly one behavior");
    };
    assert_eq!((update.event.id, update.agent), (receipt, recipient));
    let acknowledged = channel.acknowledge(accepted, update.at).unwrap();
    assert_eq!(acknowledged.state, TransferState::Acknowledged);
    assert_eq!(acknowledged.acknowledged_at, Some(tick(7)));
    assert_eq!(allocation.close_request(request, update.at), Ok(true));
    assert_eq!(
        preview(&mut allocation, &mut des, &mut abm),
        Some(StagedDispatchSelection::Allocation(tick(7)))
    );
    assert_eq!(allocation.run_until_or_for(tick(7), 1), Ok(1));
    assert_eq!(allocation.resource(resource).unwrap().available, 1);
    assert_eq!(
        channel.get_envelope(expired).unwrap().state,
        TransferState::Expired
    );
    assert_eq!(channel.next_event_time(), None);
}

#[test]
fn overdue_boundary_wakeup_preserves_scheduled_arrival_order() {
    let tick = SimTime::from_ticks;
    let mut des = DESContext::new(7);
    let mut abm = BehaviorSimulation::new(7, RecordingBehavior::default());
    let [later, first, second, future] = std::array::from_fn(|_| abm.spawn_agent());
    let mut allocation = StagedResourceRuntime::new().unwrap();
    let mut channel = BoundaryChannel::new(
        "handover",
        4,
        TransitDelayDistribution::Uniform {
            min: tick(1),
            max: tick(5),
        },
    );
    let mut rng = DeterministicStream::new(155);
    for (id, entity, arrival) in [(1, later, 4), (8, first, 3), (3, second, 3), (4, future, 7)] {
        let operation = HandoverOperationId::new(id);
        channel
            .request_transfer(HandoverEnvelope::new(
                operation,
                entity,
                (),
                "sender",
                "receiver",
                tick(2),
                None,
            ))
            .unwrap();
        assert_eq!(
            channel.depart(operation, tick(2), &mut rng),
            Ok(tick(arrival))
        );
    }

    // The receiver has reached tick 5 when the model starts servicing the channel.
    let observer = abm.spawn_agent();
    abm.schedule_update(observer, tick(5));
    assert_eq!(abm.run_for(1).updates()[0].at, tick(5));
    let due = channel.next_event_time().unwrap();
    assert_eq!(due, tick(3));
    let wakeup = des.schedule_at(
        due.max(abm.context().scheduler.now()),
        0,
        EventKind::custom(83),
    );
    assert!(matches!(
        allocation.peek_with_schedulers(
            &mut [&mut des.scheduler, &mut abm.context_mut().scheduler],
            tick(5)
        ),
        Ok(Some(StagedDispatchSelection::Model { scheduler_index: 0, event }))
            if event.id == wakeup && event.at == tick(5)
    ));
    let event = des.step().into_dispatched().unwrap();
    assert_eq!(event.id, wakeup);
    assert!(channel.check_expiries(event.at).is_empty());
    let arrivals = channel.collect_arrivals(event.at);
    for arrival in &arrivals {
        abm.schedule_update(arrival.entity, event.at);
    }

    assert_eq!(
        abm.run_for(3)
            .updates()
            .iter()
            .map(|update| (update.at, update.agent))
            .collect::<Vec<_>>(),
        [(tick(5), first), (tick(5), second), (tick(5), later)]
    );
    assert_eq!(
        arrivals
            .iter()
            .map(|envelope| envelope.arrive_at)
            .collect::<Vec<_>>(),
        [Some(tick(3)), Some(tick(3)), Some(tick(4))]
    );
    assert_eq!(channel.next_event_time(), Some(tick(7)));
}
