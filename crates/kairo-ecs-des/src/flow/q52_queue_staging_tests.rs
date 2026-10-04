use super::*;

fn dispatch(at: u128) -> FlowDispatch {
    FlowDispatch {
        event: EventId::new(0, 0),
        at: SimTime::from_ticks(at),
        records: Vec::new(),
        error: None,
        callback_batches: Vec::new(),
    }
}

#[test]
fn reprioritize_stages_one_large_target_queue_and_keeps_priority_order() {
    let mut runtime = FlowRuntime::new();
    let owner = runtime.spawn_actor().unwrap();
    let resource = runtime.create_resource(0).unwrap();
    let mut requests = Vec::new();
    for _ in 0..1_000 {
        let request = runtime.acquire(resource).owner(owner).submit().unwrap();
        runtime.step().unwrap().unwrap();
        requests.push(request);
    }
    let target = requests[500];
    for _ in 0..1_000 {
        runtime.create_resource(0).unwrap();
    }
    assert_eq!(runtime.resources.len(), 1_001);

    let before_queue = runtime
        .registry
        .get::<ClaimQueue>(resource.0)
        .unwrap()
        .clone();
    let before_requests = requests
        .iter()
        .map(|id| (*id, runtime.request(*id).unwrap()))
        .collect::<BTreeMap<_, _>>();
    let before_world = runtime.world.snapshot();
    let before_head = runtime.scheduler.peek_next();
    let before_stats = runtime.scheduler.stats();
    let before_budget = (
        runtime.budget_tick,
        runtime.budget_consumed,
        runtime.budget_halt,
    );
    let before_counters = (runtime.created, runtime.scheduled, runtime.destroyed);

    let mut outcome = dispatch(0);
    let plan = runtime
        .plan(Command::Reprioritize(target, -1), &mut outcome)
        .unwrap()
        .expect("valid rekey plan");

    assert_eq!(plan.resources.len(), 1);
    assert!(plan.resources.contains_key(&resource));
    assert_eq!(plan.requests.len(), 1);
    assert_eq!(plan.requests[&target].priority_level, -1);
    let actual_base = runtime.registry.get::<ClaimQueue>(resource.0).unwrap();
    assert_eq!(actual_base, &before_queue);
    assert_eq!(
        requests
            .iter()
            .map(|id| (*id, runtime.request(*id).unwrap()))
            .collect::<BTreeMap<_, _>>(),
        before_requests
    );
    assert_eq!(runtime.world.snapshot(), before_world);
    assert_eq!(runtime.scheduler.peek_next(), before_head);
    assert_eq!(runtime.scheduler.stats(), before_stats);
    assert_eq!(
        (
            runtime.budget_tick,
            runtime.budget_consumed,
            runtime.budget_halt
        ),
        before_budget
    );
    assert_eq!(
        (runtime.created, runtime.scheduled, runtime.destroyed),
        before_counters
    );

    let mut expected = BTreeSet::new();
    for key in &before_queue.requests {
        expected.insert(if key.request == target {
            PriorityKey { level: -1, ..*key }
        } else {
            *key
        });
    }
    let mut applied = before_queue.clone();
    plan.resources[&resource]
        .queue
        .apply(&mut applied.requests)
        .unwrap();
    assert_eq!(applied.requests, expected);
    assert_eq!(
        applied
            .requests
            .iter()
            .map(|key| key.request)
            .collect::<Vec<_>>(),
        expected.iter().map(|key| key.request).collect::<Vec<_>>()
    );

    runtime.commit_plan(plan);
    let committed = runtime.registry.get::<ClaimQueue>(resource.0).unwrap();
    assert_eq!(committed.requests, expected);
    assert_eq!(runtime.request(target).unwrap().priority_level, -1);
}

#[test]
fn despawn_corrupt_second_queue_rejects_without_mutating_first_or_runtime() {
    let mut runtime = FlowRuntime::new();
    let owner = runtime.spawn_actor().unwrap();
    let first_resource = runtime.create_resource(0).unwrap();
    let second_resource = runtime.create_resource(0).unwrap();
    let first = runtime
        .acquire(first_resource)
        .owner(owner)
        .submit()
        .unwrap();
    runtime.step().unwrap().unwrap();
    let second = runtime
        .acquire(second_resource)
        .owner(owner)
        .submit()
        .unwrap();
    runtime.step().unwrap().unwrap();

    runtime
        .registry
        .store_mut::<ClaimQueue>()
        .unwrap()
        .get_mut(second_resource.0)
        .unwrap()
        .requests
        .clear();

    let before_first_queue = runtime
        .registry
        .get::<ClaimQueue>(first_resource.0)
        .unwrap()
        .clone();
    let before_second_queue = runtime
        .registry
        .get::<ClaimQueue>(second_resource.0)
        .unwrap()
        .clone();
    let before_first_request = runtime.request(first).unwrap();
    let before_second_request = runtime.request(second).unwrap();
    let before_world = runtime.world.snapshot();
    let before_head = runtime.scheduler.peek_next();
    let before_stats = runtime.scheduler.stats();
    let before_budget = (
        runtime.budget_tick,
        runtime.budget_consumed,
        runtime.budget_halt,
    );
    let before_counters = (runtime.created, runtime.scheduled, runtime.destroyed);

    let error = match runtime.plan(Command::Despawn(owner), &mut dispatch(0)) {
        Err(error) => error,
        Ok(_) => panic!("corrupt second queue must reject the full plan"),
    };
    assert_eq!(error, FlowError::InvalidState);
    assert_eq!(
        runtime
            .registry
            .get::<ClaimQueue>(first_resource.0)
            .unwrap(),
        &before_first_queue
    );
    assert_eq!(
        runtime
            .registry
            .get::<ClaimQueue>(second_resource.0)
            .unwrap(),
        &before_second_queue
    );
    assert_eq!(runtime.request(first).unwrap(), before_first_request);
    assert_eq!(runtime.request(second).unwrap(), before_second_request);
    assert_eq!(runtime.world.snapshot(), before_world);
    assert_eq!(runtime.scheduler.peek_next(), before_head);
    assert_eq!(runtime.scheduler.stats(), before_stats);
    assert_eq!(
        (
            runtime.budget_tick,
            runtime.budget_consumed,
            runtime.budget_halt
        ),
        before_budget
    );
    assert_eq!(
        (runtime.created, runtime.scheduled, runtime.destroyed),
        before_counters
    );
}

#[test]
fn absent_remove_target_keeps_explicit_invalid_resource_error_order() {
    let mut runtime = FlowRuntime::new();
    runtime.create_resource(1).unwrap();
    let absent = ResourceId(EntityId::new(u64::MAX, 0));
    let mut outcome = dispatch(0);

    let plan = runtime
        .plan(Command::Remove(absent), &mut outcome)
        .unwrap()
        .expect("explicit invalid-resource rejection is a dispatch plan");

    assert!(plan.resources.is_empty());
    assert_eq!(outcome.error, Some(FlowError::InvalidResource));
}

#[test]
fn missing_component_on_registered_target_is_structural_before_planning() {
    let mut runtime = FlowRuntime::new();
    let resource = runtime.create_resource(1).unwrap();
    runtime.registry.remove::<ClaimQueue>(resource.0);
    let before_head = runtime.scheduler.peek_next();
    let before_world = runtime.world.snapshot();
    let mut outcome = dispatch(0);

    let error = match runtime.plan(Command::Capacity(resource, 2), &mut outcome) {
        Err(error) => error,
        Ok(_) => panic!("missing registered queue must be structural corruption"),
    };

    assert_eq!(error, FlowError::InvalidState);
    assert_eq!(runtime.scheduler.peek_next(), before_head);
    assert_eq!(runtime.world.snapshot(), before_world);
}
