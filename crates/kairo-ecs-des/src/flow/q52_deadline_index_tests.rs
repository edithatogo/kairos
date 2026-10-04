use super::*;

fn time(ticks: u128) -> SimTime {
    SimTime::from_ticks(ticks)
}

fn duration(ticks: u128) -> SimDuration {
    SimDuration::from_ticks(ticks)
}

fn dispatch(
    runtime: &mut FlowRuntime,
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
    if let Some(plan) = runtime.plan(command, &mut outcome)? {
        runtime.commit_plan(plan);
    }
    Ok(outcome)
}

fn waiting(
    runtime: &mut FlowRuntime,
    resource: ResourceId,
    owner: EntityId,
    deadline: u128,
    priority: i32,
) -> RequestId {
    let request = runtime
        .acquire(resource)
        .owner(owner)
        .deadline(time(deadline))
        .priority(priority)
        .submit()
        .unwrap();
    for _ in 0..4 {
        if runtime.request(request).unwrap().state == RequestState::Queued {
            break;
        }
        assert!(!runtime.run_for(1).unwrap().dispatches.is_empty());
    }
    assert_eq!(
        runtime.request(request).unwrap().state,
        RequestState::Queued
    );
    request
}

fn index(runtime: &FlowRuntime, resource: ResourceId) -> WaitingDeadlineIndex {
    runtime
        .registry
        .get::<WaitingDeadlineIndex>(resource.0)
        .unwrap()
        .clone()
}

#[test]
fn queued_deadline_tracks_rekey_cancel_and_inclusive_expiration() {
    let mut runtime = FlowRuntime::new();
    let owner = runtime.spawn_actor().unwrap();
    let resource = runtime.create_resource(0).unwrap();
    let request = waiting(&mut runtime, resource, owner, 20, 4);
    let original = runtime.request(request).unwrap();
    let old_key = waiting_key_for(request, &original).unwrap().unwrap();
    assert_eq!(
        index(&runtime, resource).entries,
        BTreeSet::from([(time(20), old_key)])
    );

    dispatch(&mut runtime, Command::Reprioritize(request, -2), time(0)).unwrap();
    let changed = runtime.request(request).unwrap();
    let new_key = waiting_key_for(request, &changed).unwrap().unwrap();
    assert_eq!(new_key.enqueue_sequence, old_key.enqueue_sequence);
    assert_eq!(
        index(&runtime, resource).entries,
        BTreeSet::from([(time(20), new_key)])
    );

    dispatch(&mut runtime, Command::Cancel(request), time(1)).unwrap();
    assert!(index(&runtime, resource).entries.is_empty());
    assert_eq!(index(&runtime, resource).expected_len, 0);
    assert_eq!(runtime.request(request).unwrap().deadline, None);

    let due = waiting(&mut runtime, resource, owner, 5, 1);
    let outcome = dispatch(&mut runtime, Command::Deadline(due), time(5)).unwrap();
    assert_eq!(outcome.records.len(), 1);
    assert_eq!(outcome.records[0].request, due);
    assert_eq!(runtime.request(due).unwrap().state, RequestState::TimedOut);
    assert!(index(&runtime, resource).entries.is_empty());
}

#[test]
fn due_prefix_restores_resource_then_priority_order_across_deadlines() {
    let mut runtime = FlowRuntime::new();
    let owner = runtime.spawn_actor().unwrap();
    let resource = runtime.create_resource(0).unwrap();
    let second_resource = runtime.create_resource(0).unwrap();
    let earlier_key_later_deadline = waiting(&mut runtime, resource, owner, 10, -1);
    let later_key_earlier_deadline = waiting(&mut runtime, resource, owner, 5, 1);
    let next_resource = waiting(&mut runtime, second_resource, owner, 3, -100);

    let outcome = dispatch(&mut runtime, Command::Despawn(owner), time(10)).unwrap();
    let observed: Vec<_> = outcome
        .records
        .iter()
        .map(|record| record.request)
        .collect();
    assert_eq!(
        observed,
        [
            earlier_key_later_deadline,
            later_key_earlier_deadline,
            next_resource
        ]
    );
    assert!(index(&runtime, resource).entries.is_empty());
    assert!(index(&runtime, second_resource).entries.is_empty());
}

#[test]
fn deadline_command_checks_future_tuple_and_component_witness() {
    let mut runtime = FlowRuntime::new();
    let owner = runtime.spawn_actor().unwrap();
    let resource = runtime.create_resource(0).unwrap();
    let request = waiting(&mut runtime, resource, owner, 50, 0);
    let request_data = runtime.request(request).unwrap();
    let key = waiting_key_for(request, &request_data).unwrap().unwrap();
    let mut corrupt = index(&runtime, resource);
    corrupt.entries.clear();
    corrupt.expected_len = 0;
    let _ = runtime.registry.insert(resource.0, corrupt);
    let head = runtime.scheduler.peek_next();
    let stats = runtime.scheduler.stats();
    assert_eq!(runtime.step().unwrap_err(), FlowError::InvalidState);
    assert_eq!(runtime.scheduler.peek_next(), head);
    assert_eq!(runtime.scheduler.stats(), stats);

    let _ = runtime.registry.insert(
        resource.0,
        WaitingDeadlineIndex {
            entries: BTreeSet::from([(time(50), key)]),
            expected_len: 1,
        },
    );
    let mut count_mismatch = index(&runtime, resource);
    count_mismatch.expected_len += 1;
    let _ = runtime.registry.insert(resource.0, count_mismatch);
    assert_eq!(
        dispatch(&mut runtime, Command::Deadline(request), time(1)).unwrap_err(),
        FlowError::InvalidState
    );

    let mut wrong_tuple = index(&runtime, resource);
    let (_, key) = *wrong_tuple.entries.iter().next().unwrap();
    wrong_tuple.entries.clear();
    wrong_tuple.entries.insert((time(49), key));
    wrong_tuple.expected_len = 1;
    let _ = runtime.registry.insert(resource.0, wrong_tuple);
    assert_eq!(
        dispatch(&mut runtime, Command::Deadline(request), time(1)).unwrap_err(),
        FlowError::InvalidState
    );

    let mut extra = index(&runtime, resource);
    extra.entries.insert((time(60), key));
    extra.expected_len = 2;
    let _ = runtime.registry.insert(resource.0, extra);
    assert_eq!(
        dispatch(&mut runtime, Command::Deadline(request), time(1)).unwrap_err(),
        FlowError::InvalidState
    );

    runtime.registry.remove::<WaitingDeadlineIndex>(resource.0);
    assert_eq!(runtime.step().unwrap_err(), FlowError::InvalidState);
    assert_eq!(runtime.scheduler.peek_next(), head);
    assert_eq!(runtime.scheduler.stats(), stats);
}

#[test]
fn pending_and_terminal_deadlines_are_not_indexed_or_rejected() {
    let mut runtime = FlowRuntime::new();
    let owner = runtime.spawn_actor().unwrap();
    let resource = runtime.create_resource(0).unwrap();
    let pending = runtime
        .acquire(resource)
        .owner(owner)
        .deadline(time(20))
        .submit()
        .unwrap();
    dispatch(&mut runtime, Command::Deadline(pending), time(0)).unwrap();
    assert!(index(&runtime, resource).entries.is_empty());

    let terminal = waiting(&mut runtime, resource, owner, 30, 0);
    dispatch(&mut runtime, Command::Despawn(owner), time(1)).unwrap();
    let terminal_request = runtime.request(terminal).unwrap();
    assert_eq!(terminal_request.state, RequestState::Cancelled);
    assert_eq!(terminal_request.deadline, Some(time(30)));
    assert!(index(&runtime, resource).entries.is_empty());
    dispatch(&mut runtime, Command::Deadline(terminal), time(2)).unwrap();
    let terminal_key = PriorityKey {
        level: terminal_request.priority_level,
        enqueue_sequence: terminal_request.admission_sequence.unwrap(),
        request: terminal,
    };
    let mut corrupt = index(&runtime, resource);
    corrupt.entries.insert((time(30), terminal_key));
    corrupt.expected_len = 1;
    let _ = runtime.registry.insert(resource.0, corrupt);
    assert_eq!(
        dispatch(&mut runtime, Command::Deadline(terminal), time(3)).unwrap_err(),
        FlowError::InvalidState
    );
}

#[test]
fn touched_active_deadline_is_invalid_and_removal_requires_empty_indexes() {
    let mut runtime = FlowRuntime::new();
    let owner = runtime.spawn_actor().unwrap();
    let resource = runtime.create_resource(1).unwrap();
    let active = runtime.acquire(resource).owner(owner).submit().unwrap();
    runtime.run_for(1).unwrap();
    let mut active_request = runtime.request(active).unwrap();
    active_request.deadline = Some(time(100));
    let _ = runtime.registry.insert(active.0, active_request);
    assert_eq!(
        dispatch(&mut runtime, Command::Cancel(active), time(1)).unwrap_err(),
        FlowError::InvalidState
    );

    let mut empty_runtime = FlowRuntime::new();
    let empty = empty_runtime.create_resource(0).unwrap();
    let _ = empty_runtime.registry.insert(
        empty.0,
        ClaimQueue::<PriorityKey> {
            requests: [PriorityKey {
                level: 0,
                enqueue_sequence: 0,
                request: RequestId(empty.0),
            }]
            .into(),
        },
    );
    let result = dispatch(&mut empty_runtime, Command::Remove(empty), time(0));
    assert_eq!(result.unwrap_err(), FlowError::InvalidState);
    assert!(empty_runtime.resources.contains(&empty));
}

#[test]
fn multi_resource_due_failure_keeps_authoritative_indexes_unchanged() {
    let mut runtime = FlowRuntime::new();
    let owner = runtime.spawn_actor().unwrap();
    let first = runtime.create_resource(0).unwrap();
    let second = runtime.create_resource(0).unwrap();
    let first_request = waiting(&mut runtime, first, owner, 5, 0);
    let second_request = waiting(&mut runtime, second, owner, 5, 0);
    let before_first = index(&runtime, first);
    let mut bad_second = index(&runtime, second);
    let (_, key) = *bad_second.entries.iter().next().unwrap();
    bad_second.entries.clear();
    bad_second.entries.insert((time(4), key));
    let _ = runtime.registry.insert(second.0, bad_second);
    let before_bad_second = index(&runtime, second);

    let result = dispatch(&mut runtime, Command::Despawn(owner), time(5));
    assert_eq!(result.unwrap_err(), FlowError::InvalidState);
    assert_eq!(index(&runtime, first), before_first);
    assert_eq!(index(&runtime, second), before_bad_second);
    assert_eq!(
        runtime.request(first_request).unwrap().state,
        RequestState::Queued
    );
    assert_eq!(
        runtime.request(second_request).unwrap().state,
        RequestState::Queued
    );
}

#[test]
fn resource_removal_checks_deadline_index_and_cleans_component() {
    let mut runtime = FlowRuntime::new();
    let empty = runtime.create_resource(0).unwrap();
    runtime.remove_resource(empty).unwrap();
    runtime.run_for(1).unwrap();
    assert!(!runtime.resources.contains(&empty));
    assert!(runtime
        .registry
        .get::<WaitingDeadlineIndex>(empty.0)
        .is_none());

    let corrupt = runtime.create_resource(0).unwrap();
    let mut corrupt_index = index(&runtime, corrupt);
    let fake_key = PriorityKey {
        level: 0,
        enqueue_sequence: 0,
        request: RequestId(corrupt.0),
    };
    corrupt_index.entries.insert((time(10), fake_key));
    corrupt_index.expected_len = 1;
    let _ = runtime.registry.insert(corrupt.0, corrupt_index);
    assert_eq!(
        dispatch(&mut runtime, Command::Remove(corrupt), time(0)).unwrap_err(),
        FlowError::InvalidState
    );
    assert!(runtime.resources.contains(&corrupt));
}

#[test]
fn grant_and_preemption_do_not_restore_a_cleared_deadline() {
    let mut runtime = FlowRuntime::new();
    runtime
        .register_work_handlers("deadline-index", WorkHandlers::<u32>::default())
        .unwrap();
    let low_owner = runtime.spawn_actor().unwrap();
    let high_owner = runtime.spawn_actor().unwrap();
    let top_owner = runtime.spawn_actor().unwrap();
    let resource = runtime.create_resource(1).unwrap();
    let low_work = runtime
        .create_work(low_owner, duration(100), "deadline-index", 1u32)
        .unwrap();
    let high_work = runtime
        .create_work(high_owner, duration(100), "deadline-index", 2u32)
        .unwrap();
    let top_work = runtime
        .create_work(top_owner, duration(100), "deadline-index", 3u32)
        .unwrap();
    let low = runtime
        .acquire(resource)
        .owner(low_owner)
        .timed_work(low_work)
        .priority(10)
        .preemptible(PreemptionStrategy::Suspend)
        .submit()
        .unwrap();
    runtime.run_for(1).unwrap();
    assert_eq!(runtime.request(low).unwrap().state, RequestState::Active);

    let high = runtime
        .acquire(resource)
        .owner(high_owner)
        .timed_work(high_work)
        .deadline(time(50))
        .priority(0)
        .can_preempt(true)
        .preemptible(PreemptionStrategy::Suspend)
        .submit()
        .unwrap();
    runtime.run_for(1).unwrap();
    assert_eq!(runtime.request(high).unwrap().state, RequestState::Active);
    assert_eq!(runtime.request(high).unwrap().deadline, None);
    assert!(index(&runtime, resource).entries.is_empty());

    let top = runtime
        .acquire(resource)
        .owner(top_owner)
        .timed_work(top_work)
        .priority(-1)
        .can_preempt(true)
        .preemptible(PreemptionStrategy::Suspend)
        .submit()
        .unwrap();
    runtime.run_for(1).unwrap();
    assert_eq!(
        runtime.request(high).unwrap().state,
        RequestState::Suspended
    );
    assert_eq!(runtime.request(top).unwrap().state, RequestState::Active);
    assert_eq!(runtime.request(high).unwrap().deadline, None);
    assert!(index(&runtime, resource).entries.is_empty());

    let mut corrupted_suspended = runtime.request(high).unwrap();
    corrupted_suspended.deadline = Some(time(60));
    let _ = runtime.registry.insert(high.0, corrupted_suspended);
    assert_eq!(
        dispatch(&mut runtime, Command::Cancel(high), time(1)).unwrap_err(),
        FlowError::InvalidState
    );
    let mut restored_suspended = runtime.request(high).unwrap();
    restored_suspended.deadline = None;
    let _ = runtime.registry.insert(high.0, restored_suspended);

    runtime
        .release(runtime.request(top).unwrap().lease.unwrap(), time(0))
        .unwrap();
    runtime.run_for(1).unwrap();
    assert_eq!(runtime.request(high).unwrap().state, RequestState::Active);
    assert_eq!(runtime.request(high).unwrap().deadline, None);
    assert!(index(&runtime, resource).entries.is_empty());
}
