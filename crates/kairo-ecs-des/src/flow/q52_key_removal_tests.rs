use super::*;

fn at(ticks: u128) -> SimTime {
    SimTime::from_ticks(ticks)
}

fn duration(ticks: u128) -> SimDuration {
    SimDuration::from_ticks(ticks)
}

fn queued_request(
    flow: &mut FlowRuntime,
    resource: ResourceId,
    owner: EntityId,
    priority: i32,
) -> RequestId {
    let request = flow
        .acquire(resource)
        .owner(owner)
        .priority(priority)
        .submit()
        .unwrap();
    flow.step().unwrap().unwrap();
    request
}

fn queue_keys(flow: &FlowRuntime, resource: ResourceId) -> Vec<PriorityKey> {
    flow.registry
        .get::<ClaimQueue>(resource.0)
        .unwrap()
        .requests
        .iter()
        .copied()
        .collect()
}

fn corrupt_waiting_key(
    flow: &mut FlowRuntime,
    resource: ResourceId,
    request: RequestId,
) -> PriorityKey {
    let value = flow.request(request).unwrap();
    let correct = waiting_key_for(request, &value).unwrap().unwrap();
    let wrong = PriorityKey {
        level: correct.level + 1,
        ..correct
    };
    let queue = flow
        .registry
        .store_mut::<ClaimQueue>()
        .unwrap()
        .get_mut(resource.0)
        .unwrap();
    assert!(queue.requests.remove(&correct));
    assert!(queue.requests.insert(wrong));
    wrong
}

fn remove_waiting_key(flow: &mut FlowRuntime, resource: ResourceId, request: RequestId) {
    let value = flow.request(request).unwrap();
    let correct = waiting_key_for(request, &value).unwrap().unwrap();
    assert!(flow
        .registry
        .store_mut::<ClaimQueue>()
        .unwrap()
        .get_mut(resource.0)
        .unwrap()
        .requests
        .remove(&correct));
}

#[test]
fn queued_cancel_removes_exact_key_and_preserves_tied_peer_order() {
    let mut flow = FlowRuntime::new();
    let owner = flow.spawn_actor().unwrap();
    let resource = flow.create_resource(0).unwrap();
    let first = queued_request(&mut flow, resource, owner, 4);
    let cancelled = queued_request(&mut flow, resource, owner, 4);
    let last = queued_request(&mut flow, resource, owner, 4);

    flow.cancel(cancelled, flow.now()).unwrap();
    flow.step().unwrap().unwrap();

    assert_eq!(
        flow.request(cancelled).unwrap().state,
        RequestState::Cancelled
    );
    assert_eq!(flow.resource(resource).unwrap().queued, vec![first, last]);
    assert_eq!(
        queue_keys(&flow, resource)
            .iter()
            .map(|key| key.request)
            .collect::<Vec<_>>(),
        vec![first, last]
    );
}

#[test]
fn suspended_cancel_removes_its_key_and_preserves_urgent_lease() {
    let mut flow = FlowRuntime::new();
    let owner = flow.spawn_actor().unwrap();
    let resource = flow.create_resource(1).unwrap();
    let low_work = flow
        .create_work(owner, duration(10), "q52.low", ())
        .unwrap();
    let low = flow
        .acquire(resource)
        .owner(owner)
        .timed_work(low_work)
        .priority(9)
        .preemptible(PreemptionStrategy::Suspend)
        .submit()
        .unwrap();
    flow.step().unwrap().unwrap();
    let urgent_work = flow
        .create_work(owner, duration(4), "q52.urgent", ())
        .unwrap();
    let urgent = flow
        .acquire(resource)
        .owner(owner)
        .timed_work(urgent_work)
        .priority(1)
        .can_preempt(true)
        .at(at(1))
        .submit()
        .unwrap();
    flow.step().unwrap().unwrap();

    assert_eq!(flow.request(low).unwrap().state, RequestState::Suspended);
    let urgent_lease = flow.request(urgent).unwrap().lease.unwrap();
    flow.cancel(low, flow.now()).unwrap();
    flow.step().unwrap().unwrap();

    let snapshot = flow.resource(resource).unwrap();
    assert_eq!(flow.request(low).unwrap().state, RequestState::Cancelled);
    assert_eq!(snapshot.active, vec![urgent_lease]);
    assert!(snapshot.queued.is_empty());
    assert_eq!(flow.request(urgent).unwrap().lease, Some(urgent_lease));
}

#[test]
fn queued_reprioritize_preserves_admission_sequence_and_tie_order() {
    let mut flow = FlowRuntime::new();
    let owner = flow.spawn_actor().unwrap();
    let resource = flow.create_resource(0).unwrap();
    let first = queued_request(&mut flow, resource, owner, 4);
    let second = queued_request(&mut flow, resource, owner, 4);
    let third = queued_request(&mut flow, resource, owner, 4);
    let original_sequence = flow.request(third).unwrap().admission_sequence;

    flow.reprioritize(third, 2, flow.now()).unwrap();
    flow.step().unwrap().unwrap();
    assert_eq!(
        flow.resource(resource).unwrap().queued,
        vec![third, first, second]
    );
    flow.reprioritize(third, 4, flow.now()).unwrap();
    flow.step().unwrap().unwrap();

    assert_eq!(
        flow.resource(resource).unwrap().queued,
        vec![first, second, third]
    );
    assert_eq!(
        flow.request(third).unwrap().admission_sequence,
        original_sequence
    );
}

#[test]
fn suspended_reprioritize_uses_original_admission_sequence_for_ties() {
    let mut flow = FlowRuntime::new();
    let owner = flow.spawn_actor().unwrap();
    let resource = flow.create_resource(1).unwrap();
    let low_work = flow
        .create_work(owner, duration(10), "q52.suspended", ())
        .unwrap();
    let low = flow
        .acquire(resource)
        .owner(owner)
        .timed_work(low_work)
        .priority(9)
        .preemptible(PreemptionStrategy::Suspend)
        .submit()
        .unwrap();
    flow.step().unwrap().unwrap();
    let first_waiter = queued_request(&mut flow, resource, owner, 3);
    let second_waiter = queued_request(&mut flow, resource, owner, 3);
    let urgent_work = flow
        .create_work(owner, duration(4), "q52.suspended.urgent", ())
        .unwrap();
    let urgent = flow
        .acquire(resource)
        .owner(owner)
        .timed_work(urgent_work)
        .priority(1)
        .can_preempt(true)
        .at(at(1))
        .submit()
        .unwrap();
    flow.step().unwrap().unwrap();
    let original_sequence = flow.request(low).unwrap().admission_sequence;
    assert_eq!(flow.request(low).unwrap().state, RequestState::Suspended);
    assert_eq!(flow.request(urgent).unwrap().state, RequestState::Active);

    flow.reprioritize(low, 3, flow.now()).unwrap();
    flow.step().unwrap().unwrap();

    assert_eq!(
        flow.resource(resource).unwrap().queued,
        vec![low, first_waiter, second_waiter]
    );
    assert_eq!(
        flow.request(low).unwrap().admission_sequence,
        original_sequence
    );
    assert_eq!(flow.request(urgent).unwrap().state, RequestState::Active);
}

#[test]
fn active_reprioritize_updates_lease_without_queue_membership() {
    let mut flow = FlowRuntime::new();
    let owner = flow.spawn_actor().unwrap();
    let resource = flow.create_resource(1).unwrap();
    let request = flow
        .acquire(resource)
        .owner(owner)
        .priority(2)
        .submit()
        .unwrap();
    flow.step().unwrap().unwrap();
    let lease = flow.request(request).unwrap().lease.unwrap();

    flow.reprioritize(request, -5, flow.now()).unwrap();
    flow.step().unwrap().unwrap();

    let snapshot = flow.resource(resource).unwrap();
    assert!(snapshot.queued.is_empty());
    assert_eq!(snapshot.active, vec![lease]);
    assert_eq!(snapshot.allocations[0].priority_level, -5);
    assert_eq!(flow.request(request).unwrap().priority_level, -5);
}

#[test]
fn pending_cancel_does_not_require_waiting_key() {
    let mut flow = FlowRuntime::new();
    let owner = flow.spawn_actor().unwrap();
    let resource = flow.create_resource(1).unwrap();
    let request = flow.submit(resource, owner, at(1)).unwrap();
    assert_eq!(flow.request(request).unwrap().state, RequestState::Pending);
    assert_eq!(
        waiting_key_for(request, &flow.request(request).unwrap()),
        Ok(None)
    );

    flow.cancel_with_scheduler_priority(request, at(1), -1)
        .unwrap();
    flow.step().unwrap().unwrap();

    assert_eq!(
        flow.request(request).unwrap().state,
        RequestState::Cancelled
    );
    assert!(flow.resource(resource).unwrap().queued.is_empty());
}

#[test]
fn missing_or_wrong_waiting_keys_fail_cancel_reprioritize_and_despawn_atomically() {
    for operation in 0..3 {
        for wrong_key_present in [false, true] {
            let mut flow = FlowRuntime::new();
            let owner = flow.spawn_actor().unwrap();
            let resource = flow.create_resource(0).unwrap();
            let request = queued_request(&mut flow, resource, owner, 4);
            if wrong_key_present {
                corrupt_waiting_key(&mut flow, resource, request);
            } else {
                remove_waiting_key(&mut flow, resource, request);
            }
            match operation {
                0 => flow.cancel(request, flow.now()).unwrap(),
                1 => flow.reprioritize(request, 2, flow.now()).unwrap(),
                _ => flow.despawn_actor(owner).unwrap(),
            }
            let head = flow.scheduler.peek_next();
            let stats = flow.scheduler.stats();
            let request_before = flow.request(request).unwrap();
            let queue_before = queue_keys(&flow, resource);
            let counters_before = (
                flow.scheduled,
                flow.destroyed,
                flow.next_admission,
                flow.next_lease,
            );
            let commands_before = flow.commands.keys().copied().collect::<Vec<_>>();

            assert_eq!(flow.step().unwrap_err(), FlowError::InvalidState);
            assert_eq!(flow.scheduler.peek_next(), head);
            assert_eq!(flow.scheduler.stats(), stats);
            assert_eq!(flow.request(request).unwrap(), request_before);
            assert_eq!(queue_keys(&flow, resource), queue_before);
            assert_eq!(queue_before.len(), usize::from(wrong_key_present));
            assert_eq!(
                (
                    flow.scheduled,
                    flow.destroyed,
                    flow.next_admission,
                    flow.next_lease,
                ),
                counters_before
            );
            assert_eq!(
                flow.commands.keys().copied().collect::<Vec<_>>(),
                commands_before
            );
            assert!(flow.actor(owner).is_ok());
        }
    }
}

#[test]
fn wrong_waiting_key_fails_replacement_atomically() {
    let mut flow = FlowRuntime::new();
    let owner = flow.spawn_actor().unwrap();
    let resource = flow.create_resource(1).unwrap();
    let holder_work = flow
        .create_work(owner, duration(10), "q52.holder", ())
        .unwrap();
    let holder = flow
        .acquire(resource)
        .owner(owner)
        .timed_work(holder_work)
        .priority(9)
        .preemptible(PreemptionStrategy::Suspend)
        .submit()
        .unwrap();
    flow.step().unwrap().unwrap();
    let waiting = queued_request(&mut flow, resource, owner, 1);
    flow.registry
        .store_mut::<ResourceRequest>()
        .unwrap()
        .get_mut(waiting.0)
        .unwrap()
        .can_preempt = true;
    let original_key = waiting_key_for(waiting, &flow.request(waiting).unwrap())
        .unwrap()
        .unwrap();
    {
        let index = flow
            .registry
            .store_mut::<PreemptingWaiters>()
            .unwrap()
            .get_mut(resource.0)
            .unwrap();
        assert!(index.keys.insert(original_key));
        index.expected_len = index.keys.len();
    }
    let preempting_before = flow
        .registry
        .get::<PreemptingWaiters>(resource.0)
        .unwrap()
        .clone();
    let wrong = corrupt_waiting_key(&mut flow, resource, waiting);
    let holder_progress = flow
        .registry
        .get::<WorkProgress>(holder_work.0)
        .unwrap()
        .clone();

    flow.set_capacity(resource, 1).unwrap();
    let head = flow.scheduler.peek_next();
    let stats = flow.scheduler.stats();
    let request_before = flow.request(waiting).unwrap();
    let queue_before = queue_keys(&flow, resource);
    let counters_before = (flow.scheduled, flow.destroyed, flow.next_lease);

    assert_eq!(flow.step().unwrap_err(), FlowError::InvalidState);
    assert_eq!(flow.scheduler.peek_next(), head);
    assert_eq!(flow.scheduler.stats(), stats);
    assert_eq!(flow.request(holder).unwrap().state, RequestState::Active);
    assert_eq!(flow.request(waiting).unwrap(), request_before);
    assert_eq!(
        flow.registry.get::<WorkProgress>(holder_work.0),
        Some(&holder_progress)
    );
    assert_eq!(queue_keys(&flow, resource), queue_before);
    assert_eq!(queue_before, vec![wrong]);
    assert_eq!(
        flow.registry.get::<PreemptingWaiters>(resource.0),
        Some(&preempting_before)
    );
    assert_eq!(
        (flow.scheduled, flow.destroyed, flow.next_lease),
        counters_before
    );
}

#[test]
fn despawn_directly_removes_all_owned_waiters_across_resources() {
    let mut flow = FlowRuntime::new();
    let owner = flow.spawn_actor().unwrap();
    let first_resource = flow.create_resource(0).unwrap();
    let second_resource = flow.create_resource(0).unwrap();
    let first = queued_request(&mut flow, first_resource, owner, 2);
    let second = queued_request(&mut flow, first_resource, owner, 2);
    let third = queued_request(&mut flow, second_resource, owner, 2);

    flow.despawn_actor(owner).unwrap();
    flow.step().unwrap().unwrap();

    assert_eq!(flow.request(first).unwrap().state, RequestState::Cancelled);
    assert_eq!(flow.request(second).unwrap().state, RequestState::Cancelled);
    assert_eq!(flow.request(third).unwrap().state, RequestState::Cancelled);
    assert!(flow.resource(first_resource).unwrap().queued.is_empty());
    assert!(flow.resource(second_resource).unwrap().queued.is_empty());
    assert_eq!(queue_keys(&flow, first_resource), Vec::<PriorityKey>::new());
    assert_eq!(
        queue_keys(&flow, second_resource),
        Vec::<PriorityKey>::new()
    );
    assert!(!flow.actors.contains(&owner));
    assert!(!flow.world.is_alive(owner));
}

#[test]
fn despawn_rolls_back_earlier_resource_when_later_waiting_key_is_corrupt() {
    let mut flow = FlowRuntime::new();
    let owner = flow.spawn_actor().unwrap();
    let first_resource = flow.create_resource(0).unwrap();
    let second_resource = flow.create_resource(0).unwrap();
    let first = queued_request(&mut flow, first_resource, owner, 2);
    let second = queued_request(&mut flow, second_resource, owner, 2);
    let corrupt = corrupt_waiting_key(&mut flow, second_resource, second);
    assert!(
        first < second,
        "despawn visits requests in request-id order"
    );

    flow.despawn_actor(owner).unwrap();
    let head = flow.scheduler.peek_next();
    let stats = flow.scheduler.stats();
    let first_request_before = flow.request(first).unwrap();
    let second_request_before = flow.request(second).unwrap();
    let first_resource_before = flow.resource(first_resource).unwrap();
    let second_resource_before = flow.resource(second_resource).unwrap();
    let first_queue_before = queue_keys(&flow, first_resource);
    let second_queue_before = queue_keys(&flow, second_resource);
    let counters_before = (
        flow.scheduled,
        flow.destroyed,
        flow.next_admission,
        flow.next_lease,
    );
    let commands_before = flow.commands.keys().copied().collect::<Vec<_>>();
    let pending_despawns_before = flow.pending_despawns.clone();

    assert_eq!(flow.step().unwrap_err(), FlowError::InvalidState);
    assert_eq!(flow.scheduler.peek_next(), head);
    assert_eq!(flow.scheduler.stats(), stats);
    assert_eq!(flow.request(first).unwrap(), first_request_before);
    assert_eq!(flow.request(second).unwrap(), second_request_before);
    assert_eq!(
        flow.resource(first_resource).unwrap(),
        first_resource_before
    );
    assert_eq!(
        flow.resource(second_resource).unwrap(),
        second_resource_before
    );
    assert_eq!(queue_keys(&flow, first_resource), first_queue_before);
    assert_eq!(queue_keys(&flow, second_resource), second_queue_before);
    assert_eq!(queue_keys(&flow, second_resource), vec![corrupt]);
    assert_eq!(
        (
            flow.scheduled,
            flow.destroyed,
            flow.next_admission,
            flow.next_lease,
        ),
        counters_before
    );
    assert_eq!(
        flow.commands.keys().copied().collect::<Vec<_>>(),
        commands_before
    );
    assert_eq!(flow.pending_despawns, pending_despawns_before);
    assert!(flow.actor(owner).is_ok());
    assert!(flow.world.is_alive(owner));
}
