use super::*;

fn t(tick: u128) -> SimTime {
    SimTime::from_ticks(tick)
}

fn dispatch(
    flow: &mut FlowRuntime,
    command: Command,
    at: SimTime,
) -> Result<FlowDispatch, FlowError> {
    let mut outcome = FlowDispatch {
        event: EventId::new(0, 0),
        at,
        records: Vec::new(),
        error: None,
        callback_batches: Vec::new(),
    };
    if let Some(plan) = flow.plan(command, &mut outcome)? {
        flow.commit_plan(plan);
    }
    Ok(outcome)
}

fn cache(flow: &FlowRuntime, resource: ResourceId) -> PreemptingWaiters {
    flow.registry
        .get::<PreemptingWaiters>(resource.0)
        .unwrap()
        .clone()
}

fn key(flow: &FlowRuntime, request: RequestId) -> PriorityKey {
    waiting_key_for(request, &flow.request(request).unwrap())
        .unwrap()
        .unwrap()
}

fn restart_initial(initial: &u32) -> u32 {
    *initial
}

fn full_with_candidate() -> (FlowRuntime, EntityId, ResourceId, RequestId, RequestId) {
    let mut flow = FlowRuntime::new();
    let owner = flow.spawn_actor().unwrap();
    let resource = flow.create_resource(1).unwrap();
    let holder = flow.acquire(resource).owner(owner).submit().unwrap();
    flow.step().unwrap().unwrap();
    let candidate = flow
        .acquire(resource)
        .owner(owner)
        .priority(-1)
        .can_preempt(true)
        .submit()
        .unwrap();
    flow.step().unwrap().unwrap();
    (flow, owner, resource, holder, candidate)
}

fn stage_for<'a>(flow: &'a FlowRuntime, resource: ResourceId) -> ResourceStage<'a> {
    let queue = flow.registry.get::<ClaimQueue>(resource.0).unwrap();
    let deadline = flow
        .registry
        .get::<WaitingDeadlineIndex>(resource.0)
        .unwrap();
    let preempting = flow.registry.get::<PreemptingWaiters>(resource.0).unwrap();
    ResourceStage {
        capacity: flow
            .registry
            .get::<ResourceCapacity>(resource.0)
            .unwrap()
            .clone(),
        queue: QueueDelta::new(&queue.requests),
        deadline: QueueDelta::new(&deadline.entries),
        deadline_expected_len: deadline.expected_len,
        preempting: QueueDelta::new(&preempting.keys),
        preempting_expected_len: preempting.expected_len,
        active: flow
            .registry
            .get::<ActiveAllocations>(resource.0)
            .unwrap()
            .clone(),
    }
}

fn assert_first_invalid(
    resource: ResourceId,
    stage: &ResourceStage<'_>,
    base: &BTreeMap<RequestId, ResourceRequest>,
) {
    let lookup = |id: &RequestId| base.get(id);
    let requests = RecordDelta::new(&lookup);
    assert_eq!(
        first_preempting_waiter(resource, stage, &requests),
        Err(FlowError::InvalidState)
    );
}

#[test]
fn runtime_cache_tracks_waiting_preemptors_and_excludes_active_requests() {
    let mut flow = FlowRuntime::new();
    let owner = flow.spawn_actor().unwrap();
    let resource = flow.create_resource(1).unwrap();
    let active = flow
        .acquire(resource)
        .owner(owner)
        .priority(20)
        .can_preempt(true)
        .submit()
        .unwrap();
    flow.step().unwrap().unwrap();
    let cache = flow.registry.get::<PreemptingWaiters>(resource.0).unwrap();
    assert!(cache.keys.is_empty());
    assert_eq!(cache.expected_len, 0);
    assert_eq!(flow.request(active).unwrap().state, RequestState::Active);

    let waiting = flow
        .acquire(resource)
        .owner(owner)
        .priority(10)
        .can_preempt(true)
        .submit()
        .unwrap();
    flow.step().unwrap().unwrap();
    let old_key = PriorityKey {
        level: 10,
        enqueue_sequence: flow.request(waiting).unwrap().admission_sequence.unwrap(),
        request: waiting,
    };
    let cache = flow.registry.get::<PreemptingWaiters>(resource.0).unwrap();
    assert_eq!(cache.keys, BTreeSet::from([old_key]));
    assert_eq!(cache.expected_len, 1);

    flow.reprioritize(waiting, 5, t(0)).unwrap();
    flow.step().unwrap().unwrap();
    let new_key = PriorityKey {
        level: 5,
        ..old_key
    };
    let cache = flow.registry.get::<PreemptingWaiters>(resource.0).unwrap();
    assert_eq!(cache.keys, BTreeSet::from([new_key]));
    assert_eq!(cache.expected_len, 1);

    flow.cancel(waiting, t(0)).unwrap();
    flow.step().unwrap().unwrap();
    let cache = flow.registry.get::<PreemptingWaiters>(resource.0).unwrap();
    assert!(cache.keys.is_empty());
    assert_eq!(cache.expected_len, 0);
    assert_eq!(
        flow.request(waiting).unwrap().state,
        RequestState::Cancelled
    );
}

#[test]
fn first_cached_key_is_validated_against_request_and_primary_queue() {
    let mut flow = FlowRuntime::new();
    let owner = flow.spawn_actor().unwrap();
    let resource_id = flow.create_resource(0).unwrap();
    let request_id = flow
        .acquire(resource_id)
        .owner(owner)
        .priority(7)
        .can_preempt(true)
        .submit()
        .unwrap();
    flow.step().unwrap().unwrap();

    let queue = flow.registry.get::<ClaimQueue>(resource_id.0).unwrap();
    let deadline = flow
        .registry
        .get::<WaitingDeadlineIndex>(resource_id.0)
        .unwrap();
    let cache = flow
        .registry
        .get::<PreemptingWaiters>(resource_id.0)
        .unwrap();
    let capacity = flow
        .registry
        .get::<ResourceCapacity>(resource_id.0)
        .unwrap()
        .clone();
    let active = flow
        .registry
        .get::<ActiveAllocations>(resource_id.0)
        .unwrap()
        .clone();
    let mut stage = ResourceStage {
        capacity,
        queue: QueueDelta::new(&queue.requests),
        deadline: QueueDelta::new(&deadline.entries),
        deadline_expected_len: deadline.expected_len,
        preempting: QueueDelta::new(&cache.keys),
        preempting_expected_len: cache.expected_len,
        active,
    };
    let lookup = |id: &RequestId| flow.registry.get::<ResourceRequest>(id.0);
    let requests = RecordDelta::new(&lookup);
    assert_eq!(
        first_preempting_waiter(resource_id, &stage, &requests).unwrap(),
        Some(WaitingCandidate {
            id: request_id,
            priority_level: 7,
            original_admission_sequence: flow
                .request(request_id)
                .unwrap()
                .admission_sequence
                .unwrap(),
            can_preempt: true,
        })
    );

    let key = waiting_key_for(request_id, &flow.request(request_id).unwrap())
        .unwrap()
        .unwrap();
    stage.preempting.remove(&key);
    assert_eq!(
        first_preempting_waiter(resource_id, &stage, &requests),
        Err(FlowError::InvalidState)
    );
}

#[test]
fn resource_creation_removal_and_pending_admission_track_exact_cache_membership() {
    let mut flow = FlowRuntime::new();
    let owner = flow.spawn_actor().unwrap();
    let resource = flow.create_resource(0).unwrap();
    assert_eq!(cache(&flow, resource), PreemptingWaiters::default());
    let pending = flow
        .acquire(resource)
        .owner(owner)
        .priority(4)
        .can_preempt(true)
        .at(t(1))
        .submit()
        .unwrap();
    assert_eq!(flow.request(pending).unwrap().state, RequestState::Pending);
    assert_eq!(cache(&flow, resource), PreemptingWaiters::default());
    flow.step().unwrap().unwrap();
    assert_eq!(flow.request(pending).unwrap().state, RequestState::Queued);
    assert_eq!(
        cache(&flow, resource).keys,
        BTreeSet::from([key(&flow, pending)])
    );
    assert_eq!(cache(&flow, resource).expected_len, 1);

    flow.cancel(pending, t(1)).unwrap();
    flow.step().unwrap().unwrap();
    assert_eq!(cache(&flow, resource), PreemptingWaiters::default());
    flow.remove_resource(resource).unwrap();
    flow.step().unwrap().unwrap();
    assert!(flow.registry.get::<PreemptingWaiters>(resource.0).is_none());
}

#[test]
fn timeout_despawn_and_grant_remove_waiting_preemptors() {
    // Timeout removes the exact indexed request before terminalizing it.
    let mut timed = FlowRuntime::new();
    let owner = timed.spawn_actor().unwrap();
    let resource = timed.create_resource(0).unwrap();
    let request = timed
        .acquire(resource)
        .owner(owner)
        .deadline(t(5))
        .can_preempt(true)
        .submit()
        .unwrap();
    timed.step().unwrap().unwrap();
    assert_eq!(
        cache(&timed, resource).keys,
        BTreeSet::from([key(&timed, request)])
    );
    dispatch(&mut timed, Command::Deadline(request), t(5)).unwrap();
    assert_eq!(
        timed.request(request).unwrap().state,
        RequestState::TimedOut
    );
    assert_eq!(cache(&timed, resource), PreemptingWaiters::default());

    // Despawn cancellation removes all indexed waiters for the owner.
    let mut despawned = FlowRuntime::new();
    let owner = despawned.spawn_actor().unwrap();
    let resource = despawned.create_resource(0).unwrap();
    let request = despawned
        .acquire(resource)
        .owner(owner)
        .can_preempt(true)
        .submit()
        .unwrap();
    despawned.step().unwrap().unwrap();
    assert_eq!(cache(&despawned, resource).expected_len, 1);
    dispatch(&mut despawned, Command::Despawn(owner), t(0)).unwrap();
    assert_eq!(
        despawned.request(request).unwrap().state,
        RequestState::Cancelled
    );
    assert_eq!(cache(&despawned, resource), PreemptingWaiters::default());

    // A queued preemptor granted by capacity growth leaves the waiter index.
    let mut granted = FlowRuntime::new();
    let owner = granted.spawn_actor().unwrap();
    let resource = granted.create_resource(0).unwrap();
    let request = granted
        .acquire(resource)
        .owner(owner)
        .can_preempt(true)
        .submit()
        .unwrap();
    granted.step().unwrap().unwrap();
    assert_eq!(cache(&granted, resource).expected_len, 1);
    granted.set_capacity(resource, 1).unwrap();
    granted.step().unwrap().unwrap();
    assert_eq!(
        granted.request(request).unwrap().state,
        RequestState::Active
    );
    assert_eq!(cache(&granted, resource), PreemptingWaiters::default());
}

#[test]
fn active_reprioritization_keeps_the_request_out_of_the_waiter_index() {
    let mut flow = FlowRuntime::new();
    let owner = flow.spawn_actor().unwrap();
    let resource = flow.create_resource(1).unwrap();
    let active = flow
        .acquire(resource)
        .owner(owner)
        .priority(8)
        .can_preempt(true)
        .submit()
        .unwrap();
    flow.step().unwrap().unwrap();
    flow.reprioritize(active, -10, t(0)).unwrap();
    flow.step().unwrap().unwrap();
    assert_eq!(flow.request(active).unwrap().state, RequestState::Active);
    assert_eq!(flow.request(active).unwrap().priority_level, -10);
    assert_eq!(cache(&flow, resource), PreemptingWaiters::default());
}

#[test]
fn victim_strategy_requeues_only_suspend_and_restart_preemptors() {
    for strategy in [
        PreemptionStrategy::Suspend,
        PreemptionStrategy::Restart,
        PreemptionStrategy::Abort,
    ] {
        let mut flow = FlowRuntime::new();
        let low_owner = flow.spawn_actor().unwrap();
        let urgent_owner = flow.spawn_actor().unwrap();
        let resource = flow.create_resource(1).unwrap();
        let low_work = if strategy == PreemptionStrategy::Restart {
            flow.create_restartable_work(
                low_owner,
                SimDuration::from_ticks(100),
                "q52-index-low",
                17u32,
                restart_initial,
            )
            .unwrap()
        } else {
            flow.create_work(low_owner, SimDuration::from_ticks(100), "q52-index-low", ())
                .unwrap()
        };
        let low = flow
            .acquire(resource)
            .owner(low_owner)
            .priority(9)
            .can_preempt(true)
            .timed_work(low_work)
            .preemptible(strategy)
            .submit()
            .unwrap();
        flow.step().unwrap().unwrap();
        assert_eq!(cache(&flow, resource), PreemptingWaiters::default());

        let urgent_work = flow
            .create_work(
                urgent_owner,
                SimDuration::from_ticks(1),
                "q52-index-urgent",
                (),
            )
            .unwrap();
        let urgent = flow
            .acquire(resource)
            .owner(urgent_owner)
            .at(t(1))
            .priority(0)
            .can_preempt(true)
            .timed_work(urgent_work)
            .submit()
            .unwrap();
        flow.step().unwrap().unwrap();
        let low_state = flow.request(low).unwrap().state;
        let low_cache = cache(&flow, resource);
        match strategy {
            PreemptionStrategy::Suspend | PreemptionStrategy::Restart => {
                assert_eq!(low_state, RequestState::Suspended);
                assert_eq!(low_cache.keys, BTreeSet::from([key(&flow, low)]));
                assert_eq!(low_cache.expected_len, 1);
            }
            PreemptionStrategy::Abort => {
                assert_eq!(low_state, RequestState::Aborted);
                assert_eq!(low_cache, PreemptingWaiters::default());
            }
        }
        assert_eq!(flow.request(urgent).unwrap().state, RequestState::Active);
        let urgent_key = PriorityKey {
            level: flow.request(urgent).unwrap().priority_level,
            enqueue_sequence: flow.request(urgent).unwrap().admission_sequence.unwrap(),
            request: urgent,
        };
        assert!(!low_cache.keys.contains(&urgent_key));
    }
}

#[test]
fn first_cached_key_rejects_request_and_queue_corruption_matrix() {
    let (flow, _owner, resource, _holder, candidate) = full_with_candidate();
    let correct_request = flow.request(candidate).unwrap();
    let correct_key = key(&flow, candidate);
    let correct_stage = stage_for(&flow, resource);

    // A cached key that names no retained request fails closed.
    let fake = PriorityKey {
        request: RequestId(EntityId::new(900_000, 0)),
        ..correct_key
    };
    let empty_records = BTreeMap::new();
    let mut stage = correct_stage.clone();
    assert!(stage.preempting.remove(&correct_key));
    assert!(stage.preempting.insert(fake));
    assert_first_invalid(resource, &stage, &empty_records);

    let mut wrong_resource_request = correct_request.clone();
    wrong_resource_request.resource = ResourceId(EntityId::new(900_002, 0));
    assert_first_invalid(
        resource,
        &correct_stage,
        &BTreeMap::from([(candidate, wrong_resource_request)]),
    );

    let mut wrong_state_request = correct_request.clone();
    wrong_state_request.state = RequestState::Active;
    assert_first_invalid(
        resource,
        &correct_stage,
        &BTreeMap::from([(candidate, wrong_state_request)]),
    );

    let mut nonpreempting_request = correct_request.clone();
    nonpreempting_request.can_preempt = false;
    assert_first_invalid(
        resource,
        &correct_stage,
        &BTreeMap::from([(candidate, nonpreempting_request)]),
    );

    let mut wrong_key_request = correct_request.clone();
    wrong_key_request.priority_level += 1;
    assert_first_invalid(
        resource,
        &correct_stage,
        &BTreeMap::from([(candidate, wrong_key_request)]),
    );

    let mut missing_primary_key = correct_stage;
    assert!(missing_primary_key.queue.remove(&correct_key));
    assert_first_invalid(
        resource,
        &missing_primary_key,
        &BTreeMap::from([(candidate, correct_request)]),
    );
}

#[test]
fn component_count_and_deadline_pair_corruption_fail_before_head_mutation() {
    for corruption in 0..3 {
        let (mut flow, _owner, resource, _holder, _candidate) = full_with_candidate();
        if corruption == 0 {
            flow.registry.remove::<PreemptingWaiters>(resource.0);
        } else if corruption == 1 {
            let mut index = cache(&flow, resource);
            index.expected_len += 1;
            assert!(flow.registry.insert(resource.0, index));
        } else {
            let mut index = cache(&flow, resource);
            index.keys.insert(PriorityKey {
                level: i32::MIN,
                enqueue_sequence: 0,
                request: RequestId(EntityId::new(900_001, 0)),
            });
            index.expected_len = index.keys.len();
            assert!(flow.registry.insert(resource.0, index));
        }
        let request_ids: Vec<_> = flow.requests.iter().copied().collect();
        let requests_before: Vec<_> = request_ids
            .iter()
            .map(|id| (*id, flow.request(*id).unwrap()))
            .collect();
        let resource_before = flow.resource(resource).unwrap();
        let preempting_before = flow.registry.get::<PreemptingWaiters>(resource.0).cloned();
        let deadline_before = flow
            .registry
            .get::<WaitingDeadlineIndex>(resource.0)
            .unwrap()
            .clone();
        flow.set_capacity(resource, 1).unwrap();
        let head = flow.scheduler.peek_next();
        let stats = flow.scheduler.stats();
        assert_eq!(flow.step().unwrap_err(), FlowError::InvalidState);
        assert_eq!(flow.scheduler.peek_next(), head);
        assert_eq!(flow.scheduler.stats(), stats);
        assert_eq!(flow.resource(resource).unwrap(), resource_before);
        for (id, request) in requests_before {
            assert_eq!(flow.request(id).unwrap(), request);
        }
        assert_eq!(
            flow.registry.get::<PreemptingWaiters>(resource.0).cloned(),
            preempting_before
        );
        assert_eq!(
            flow.registry.get::<WaitingDeadlineIndex>(resource.0),
            Some(&deadline_before)
        );
    }

    let mut flow = FlowRuntime::new();
    let owner = flow.spawn_actor().unwrap();
    let resource = flow.create_resource(0).unwrap();
    let request = flow
        .acquire(resource)
        .owner(owner)
        .deadline(t(20))
        .can_preempt(true)
        .submit()
        .unwrap();
    flow.step().unwrap().unwrap();
    let request_before = flow.request(request).unwrap();
    let resource_before = flow.resource(resource).unwrap();
    let cache_before = cache(&flow, resource);
    let mut deadline = flow
        .registry
        .get::<WaitingDeadlineIndex>(resource.0)
        .unwrap()
        .clone();
    let (_, deadline_key) = *deadline.entries.iter().next().unwrap();
    deadline.entries.clear();
    deadline.entries.insert((t(19), deadline_key));
    let deadline_before = deadline.clone();
    assert!(flow.registry.insert(resource.0, deadline));
    flow.schedule(Command::Deadline(request), t(1)).unwrap();
    let head = flow.scheduler.peek_next();
    let stats = flow.scheduler.stats();
    assert_eq!(flow.step().unwrap_err(), FlowError::InvalidState);
    assert_eq!(flow.scheduler.peek_next(), head);
    assert_eq!(flow.scheduler.stats(), stats);
    assert_eq!(flow.request(request).unwrap(), request_before);
    assert_eq!(flow.resource(resource).unwrap(), resource_before);
    assert_eq!(cache(&flow, resource), cache_before);
    assert_eq!(
        flow.registry.get::<WaitingDeadlineIndex>(resource.0),
        Some(&deadline_before)
    );
}

#[test]
fn multi_resource_failure_rolls_back_earlier_preempting_index_changes() {
    let mut flow = FlowRuntime::new();
    let owner = flow.spawn_actor().unwrap();
    let first_resource = flow.create_resource(0).unwrap();
    let second_resource = flow.create_resource(0).unwrap();
    let first = flow
        .acquire(first_resource)
        .owner(owner)
        .can_preempt(true)
        .submit()
        .unwrap();
    flow.step().unwrap().unwrap();
    let second = flow
        .acquire(second_resource)
        .owner(owner)
        .can_preempt(true)
        .submit()
        .unwrap();
    flow.step().unwrap().unwrap();
    let mut corrupt = cache(&flow, second_resource);
    assert!(corrupt.keys.remove(&key(&flow, second)));
    corrupt.expected_len = corrupt.keys.len();
    assert!(flow.registry.insert(second_resource.0, corrupt));
    let first_request_before = flow.request(first).unwrap();
    let second_request_before = flow.request(second).unwrap();
    let first_queue_before = flow.resource(first_resource).unwrap();
    let second_queue_before = flow.resource(second_resource).unwrap();
    let first_index_before = cache(&flow, first_resource);
    let second_index_before = cache(&flow, second_resource);
    let first_deadline_before = flow
        .registry
        .get::<WaitingDeadlineIndex>(first_resource.0)
        .unwrap()
        .clone();
    let second_deadline_before = flow
        .registry
        .get::<WaitingDeadlineIndex>(second_resource.0)
        .unwrap()
        .clone();
    flow.despawn_actor(owner).unwrap();
    let head = flow.scheduler.peek_next();
    let stats = flow.scheduler.stats();
    assert_eq!(flow.step().unwrap_err(), FlowError::InvalidState);
    assert_eq!(flow.scheduler.peek_next(), head);
    assert_eq!(flow.scheduler.stats(), stats);
    assert_eq!(flow.request(first).unwrap(), first_request_before);
    assert_eq!(flow.request(second).unwrap(), second_request_before);
    assert_eq!(flow.resource(first_resource).unwrap(), first_queue_before);
    assert_eq!(flow.resource(second_resource).unwrap(), second_queue_before);
    assert_eq!(cache(&flow, first_resource), first_index_before);
    assert_eq!(cache(&flow, second_resource), second_index_before);
    assert_eq!(
        flow.registry.get::<WaitingDeadlineIndex>(first_resource.0),
        Some(&first_deadline_before)
    );
    assert_eq!(
        flow.registry.get::<WaitingDeadlineIndex>(second_resource.0),
        Some(&second_deadline_before)
    );
}

#[test]
fn empty_cache_returns_before_poisoned_holder_projection_on_ordinary_submit() {
    let mut flow = FlowRuntime::new();
    let owner = flow.spawn_actor().unwrap();
    let resource = flow.create_resource(1).unwrap();
    let active = flow.acquire(resource).owner(owner).submit().unwrap();
    flow.step().unwrap().unwrap();
    let ordinary_waiter = flow
        .acquire(resource)
        .owner(owner)
        .can_preempt(false)
        .submit()
        .unwrap();
    flow.registry.remove::<ResourceRequest>(active.0);
    let dispatch = flow.step().unwrap().unwrap();
    assert!(dispatch.error.is_none());
    assert_eq!(
        flow.request(ordinary_waiter).unwrap().state,
        RequestState::Queued
    );
    assert_eq!(cache(&flow, resource), PreemptingWaiters::default());
}
