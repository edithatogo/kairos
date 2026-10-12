// Draft integration-test fragment for crates/kairo-ecs-des/tests/flow_queue_order_v1.rs.
// Uses only the public Flow runtime; merge the imports/helpers if the owner already has them.
use std::cell::Cell;
use std::rc::Rc;

use kairo_ecs_des::{
    FlowAllocationMode, FlowQueueOrderKey, FlowQueueOrdering, FlowRuntime, LifecycleTransition,
    PreemptionStrategy, RequestId, RequestState, WorkState,
};
use kairo_ecs_types::{SimDuration, SimTime};

fn at(tick: u128) -> SimTime {
    SimTime::from_ticks(tick)
}

fn duration(ticks: u128) -> SimDuration {
    SimDuration::from_ticks(ticks)
}

// E2 key shape: fixed-width big-endian ready tick followed by ASCII request ID.
fn ready_key(ready_tick: u128, request_id: &str) -> FlowQueueOrderKey {
    assert!(request_id.is_ascii());
    let mut bytes = ready_tick.to_be_bytes().to_vec();
    bytes.extend_from_slice(request_id.as_bytes());
    FlowQueueOrderKey::new(&bytes).unwrap()
}

fn stable_resource(flow: &mut FlowRuntime) -> kairo_ecs_des::ResourceId {
    stable_resource_mode(flow, FlowAllocationMode::Eager)
}

fn stable_resource_mode(
    flow: &mut FlowRuntime,
    mode: FlowAllocationMode,
) -> kairo_ecs_des::ResourceId {
    flow.create_resource_with_queue_ordering(1, mode, FlowQueueOrdering::StableKey)
        .unwrap()
}

fn queued_winner(submission: &[(&str, u128)]) -> String {
    let mut flow = FlowRuntime::new();
    let owner = flow.spawn_actor().unwrap();
    let resource = stable_resource(&mut flow);
    let holder = flow
        .acquire(resource)
        .owner(owner)
        .at(at(0))
        .priority(0)
        .queue_order_key(ready_key(0, "holder"))
        .submit()
        .unwrap();
    flow.step().unwrap().unwrap();
    let holder_lease = flow.request(holder).unwrap().lease.unwrap();

    let mut identities = Vec::<(&str, RequestId)>::new();
    for &(name, ready_tick) in submission {
        let request = flow
            .acquire(resource)
            .owner(owner)
            .at(at(1))
            .priority(0)
            .queue_order_key(ready_key(ready_tick, name))
            .submit()
            .unwrap();
        identities.push((name, request));
    }
    for _ in 0..submission.len() {
        flow.step().unwrap().unwrap();
    }
    assert_eq!(
        flow.resource(resource).unwrap().queued.len(),
        submission.len()
    );
    flow.release(holder_lease, at(2)).unwrap();
    flow.step().unwrap().unwrap();

    let allocation = flow.resource(resource).unwrap().allocations.remove(0);
    identities
        .into_iter()
        .find_map(|(name, request)| (request == allocation.request).then(|| name.to_owned()))
        .expect("the released capacity must be granted to one submitted waiter")
}

#[test]
fn queue_order_delayed_ready_precedes_earlier_admission() {
    let mut flow = FlowRuntime::new();
    let owner = flow.spawn_actor().unwrap();
    let resource = stable_resource(&mut flow);
    let holder = flow
        .acquire(resource)
        .owner(owner)
        .at(at(0))
        .priority(0)
        .queue_order_key(ready_key(0, "holder"))
        .submit()
        .unwrap();
    flow.step().unwrap().unwrap();
    let holder_lease = flow.request(holder).unwrap().lease.unwrap();

    // B is admitted at tick 2. A is only admitted at tick 5, despite its
    // immutable key recording readiness at tick 1 rather than tick 2.
    let b = flow
        .acquire(resource)
        .owner(owner)
        .at(at(2))
        .priority(0)
        .queue_order_key(ready_key(2, "B"))
        .submit()
        .unwrap();
    let a = flow
        .acquire(resource)
        .owner(owner)
        .at(at(5))
        .priority(0)
        .queue_order_key(ready_key(1, "A"))
        .submit()
        .unwrap();
    // Schedule release after A's tick-5 Submit so the real event loop observes
    // both waiters before selecting from the queue.
    flow.step().unwrap().unwrap(); // B Submit at 2
    flow.step().unwrap().unwrap(); // A Submit at 5
    assert_eq!(flow.request(b).unwrap().state, RequestState::Queued);
    assert_eq!(flow.request(a).unwrap().state, RequestState::Queued);
    assert!(
        flow.request(b).unwrap().admission_sequence.unwrap()
            < flow.request(a).unwrap().admission_sequence.unwrap()
    );
    flow.release(holder_lease, at(5)).unwrap();
    flow.step().unwrap().unwrap(); // Release grants by effective queue order.
    assert_eq!(flow.request(a).unwrap().state, RequestState::Active);
    assert_eq!(flow.request(b).unwrap().state, RequestState::Queued);
}

#[test]
fn queue_order_suspended_and_delayed_waiters_share_effective_order() {
    let mut flow = FlowRuntime::new();
    let owner = flow.spawn_actor().unwrap();
    let resource = stable_resource(&mut flow);

    let b_work = flow
        .create_work(owner, duration(20), "queue-order-b", ())
        .unwrap();
    let b = flow
        .acquire(resource)
        .owner(owner)
        .at(at(0))
        .timed_work(b_work)
        .priority(0)
        .queue_order_key(ready_key(2, "B"))
        .preemptible(PreemptionStrategy::Suspend)
        .submit()
        .unwrap();
    flow.step().unwrap().unwrap(); // B becomes Active.

    let urgent_work = flow
        .create_work(owner, duration(2), "queue-order-u", ())
        .unwrap();
    let urgent = flow
        .acquire(resource)
        .owner(owner)
        .at(at(1))
        .timed_work(urgent_work)
        .priority(-1)
        .can_preempt(true)
        .queue_order_key(ready_key(0, "U"))
        .submit()
        .unwrap();
    let urgent_dispatch = flow.step().unwrap().unwrap(); // U preempts B; B is Suspended.
    assert!(urgent_dispatch.records.iter().any(|record| {
        record.request == b
            && record.at == at(1)
            && record.transition == LifecycleTransition::Preempted
    }));
    assert!(urgent_dispatch.records.iter().any(|record| {
        record.request == urgent
            && record.at == at(1)
            && record.transition == LifecycleTransition::Granted
    }));
    assert_eq!(flow.request(b).unwrap().state, RequestState::Suspended);
    assert_eq!(flow.request(urgent).unwrap().state, RequestState::Active);

    let a_work = flow
        .create_work(owner, duration(1), "queue-order-a", ())
        .unwrap();
    let a = flow
        .acquire(resource)
        .owner(owner)
        .at(at(2))
        .timed_work(a_work)
        .priority(0)
        .queue_order_key(ready_key(1, "A"))
        .submit()
        .unwrap();
    flow.step().unwrap().unwrap(); // A is admitted after B was suspended.
    assert_eq!(flow.request(a).unwrap().state, RequestState::Queued);

    let urgent_completion = flow.step().unwrap().unwrap(); // U completes at 3; A wins over suspended B.
    assert!(urgent_completion.records.iter().any(|record| {
        record.request == urgent
            && record.at == at(3)
            && record.transition == LifecycleTransition::Completed
    }));
    assert!(urgent_completion.records.iter().any(|record| {
        record.request == a
            && record.at == at(3)
            && record.transition == LifecycleTransition::Granted
    }));
    assert_eq!(flow.request(a).unwrap().state, RequestState::Active);
    assert_eq!(flow.request(b).unwrap().state, RequestState::Suspended);
    let a_completion = flow.step().unwrap().unwrap(); // A completes at 4 and B resumes.
    assert!(a_completion.records.iter().any(|record| {
        record.request == a
            && record.at == at(4)
            && record.transition == LifecycleTransition::Completed
    }));
    assert!(a_completion.records.iter().any(|record| {
        record.request == b
            && record.at == at(4)
            && record.transition == LifecycleTransition::Resumed
    }));
    assert_eq!(flow.request(b).unwrap().state, RequestState::Active);
}

#[test]
fn queue_order_ascii_ready_ties_are_permutation_invariant() {
    // The same distinct full keys must produce the same logical winner when
    // submit order is reversed. The event loop, not a test-side sort, grants.
    let forward = queued_winner(&[("A", 7), ("z", 7), ("m", 7)]);
    let reverse = queued_winner(&[("m", 7), ("z", 7), ("A", 7)]);
    assert_eq!(forward, "A"); // ASCII byte order: 'A' < 'm' < 'z'.
    assert_eq!(reverse, forward);
}

#[test]
fn queue_order_duplicate_key_uses_native_admission_fallback() {
    // The actual event loop verifies the native admission fallback: the first
    // submitted duplicate key wins.
    let mut flow = FlowRuntime::new();
    let owner = flow.spawn_actor().unwrap();
    let resource = stable_resource(&mut flow);
    let holder = flow
        .acquire(resource)
        .owner(owner)
        .at(at(0))
        .priority(0)
        .queue_order_key(ready_key(0, "holder"))
        .submit()
        .unwrap();
    flow.step().unwrap().unwrap();
    let lease = flow.request(holder).unwrap().lease.unwrap();
    let first = flow
        .acquire(resource)
        .owner(owner)
        .at(at(1))
        .priority(0)
        .queue_order_key(ready_key(7, "same"))
        .submit()
        .unwrap();
    let second = flow
        .acquire(resource)
        .owner(owner)
        .at(at(1))
        .priority(0)
        .queue_order_key(ready_key(7, "same"))
        .submit()
        .unwrap();
    flow.step().unwrap().unwrap();
    flow.step().unwrap().unwrap();
    assert!(
        flow.request(first).unwrap().admission_sequence.unwrap()
            < flow.request(second).unwrap().admission_sequence.unwrap()
    );
    flow.release(lease, at(2)).unwrap();
    flow.step().unwrap().unwrap();
    assert_eq!(
        flow.resource(resource).unwrap().allocations[0].request,
        first
    );
}

fn next_restart_context(template: &std::rc::Rc<std::cell::Cell<u32>>) -> u32 {
    let attempt = template.get();
    template.set(attempt + 1);
    attempt
}

#[test]
fn queue_order_priority_extremes_and_equal_priority_nonpreemption() {
    // Equal i32::MAX priority cannot preempt. Strictly smaller i32::MIN can.
    let mut flow = FlowRuntime::new();
    let owner = flow.spawn_actor().unwrap();
    let resource = stable_resource(&mut flow);
    let holder_work = flow
        .create_work(owner, duration(20), "queue-order-max-holder", ())
        .unwrap();
    let holder = flow
        .acquire(resource)
        .owner(owner)
        .at(at(0))
        .timed_work(holder_work)
        .priority(i32::MAX)
        .queue_order_key(ready_key(0, "holder"))
        .preemptible(PreemptionStrategy::Suspend)
        .submit()
        .unwrap();
    flow.step().unwrap().unwrap();
    assert_eq!(flow.request(holder).unwrap().state, RequestState::Active);

    let equal = flow
        .acquire(resource)
        .owner(owner)
        .at(at(1))
        .priority(i32::MAX)
        .can_preempt(true)
        .queue_order_key(ready_key(1, "equal"))
        .submit()
        .unwrap();
    flow.step().unwrap().unwrap();
    assert_eq!(flow.request(holder).unwrap().state, RequestState::Active);
    assert_eq!(flow.request(equal).unwrap().state, RequestState::Queued);

    let strict_work = flow
        .create_work(owner, duration(1), "queue-order-min-urgent", ())
        .unwrap();
    let strict = flow
        .acquire(resource)
        .owner(owner)
        .at(at(2))
        .timed_work(strict_work)
        .priority(i32::MIN)
        .can_preempt(true)
        .queue_order_key(ready_key(2, "strict"))
        .submit()
        .unwrap();
    flow.step().unwrap().unwrap();
    assert_eq!(flow.request(strict).unwrap().state, RequestState::Active);
    assert_eq!(flow.request(holder).unwrap().state, RequestState::Suspended);
    assert_eq!(flow.request(equal).unwrap().state, RequestState::Queued);

    // A separate nonpreemptible holder remains active even against i32::MIN.
    let mut control = FlowRuntime::new();
    let owner = control.spawn_actor().unwrap();
    let resource = stable_resource(&mut control);
    let nonpreemptible_work = control
        .create_work(owner, duration(20), "queue-order-nonpreemptible", ())
        .unwrap();
    let nonpreemptible = control
        .acquire(resource)
        .owner(owner)
        .at(at(0))
        .timed_work(nonpreemptible_work)
        .priority(i32::MAX)
        .queue_order_key(ready_key(0, "fixed-holder"))
        .submit()
        .unwrap();
    control.step().unwrap().unwrap();
    let urgent_work = control
        .create_work(owner, duration(1), "queue-order-ineligible-urgent", ())
        .unwrap();
    let urgent = control
        .acquire(resource)
        .owner(owner)
        .at(at(1))
        .timed_work(urgent_work)
        .priority(i32::MIN)
        .can_preempt(true)
        .queue_order_key(ready_key(1, "ineligible"))
        .submit()
        .unwrap();
    control.step().unwrap().unwrap();
    assert_eq!(
        control.request(nonpreemptible).unwrap().state,
        RequestState::Active
    );
    assert_eq!(control.request(urgent).unwrap().state, RequestState::Queued);
}

#[test]
fn queue_order_restart_requeues_with_original_key() {
    let mut flow = FlowRuntime::new();
    let owner = flow.spawn_actor().unwrap();
    let resource = stable_resource(&mut flow);
    let template = Rc::new(Cell::new(0_u32));
    let low_work = flow
        .create_restartable_work(
            owner,
            duration(10),
            "queue-order-restart",
            template,
            next_restart_context,
        )
        .unwrap();
    let low_key = ready_key(2, "B");
    let low = flow
        .acquire(resource)
        .owner(owner)
        .at(at(0))
        .timed_work(low_work)
        .priority(0)
        .queue_order_key(low_key.clone())
        .preemptible(PreemptionStrategy::Restart)
        .submit()
        .unwrap();
    flow.step().unwrap().unwrap(); // Low starts at 0.

    let urgent_work = flow
        .create_work(owner, duration(3), "queue-order-restart-urgent", ())
        .unwrap();
    let urgent = flow
        .acquire(resource)
        .owner(owner)
        .at(at(4))
        .timed_work(urgent_work)
        .priority(-1)
        .can_preempt(true)
        .queue_order_key(ready_key(0, "U"))
        .submit()
        .unwrap();
    flow.step().unwrap().unwrap(); // U preempts and restarts low at 4.
    assert_eq!(flow.request(low).unwrap().state, RequestState::Suspended);

    // A has the same primary priority as the restarted request but an earlier
    // stable key. It arrives while U is active and must be granted first.
    let a = flow
        .acquire(resource)
        .owner(owner)
        .at(at(5))
        .priority(0)
        .queue_order_key(ready_key(1, "A"))
        .submit()
        .unwrap();
    flow.step().unwrap().unwrap(); // A is admitted at 5.
    assert_eq!(flow.request(a).unwrap().state, RequestState::Queued);
    flow.step().unwrap().unwrap(); // U completes at 7; A is selected before B.
    assert_eq!(flow.request(urgent).unwrap().state, RequestState::Completed);
    assert_eq!(flow.request(a).unwrap().state, RequestState::Active);
    assert_eq!(flow.request(low).unwrap().state, RequestState::Suspended);
    assert_eq!(flow.work_progress(low_work).unwrap().attempt_revision, 1);
    assert_eq!(*flow.work_context::<u32>(low_work).unwrap(), 0);
    assert_eq!(
        flow.request_queue_order_key(low).unwrap().unwrap(),
        &low_key,
        "Restart must retain the original immutable queue key"
    );

    flow.release(flow.request(a).unwrap().lease.unwrap(), at(8))
        .unwrap();
    flow.step().unwrap().unwrap(); // A releases; restarted B resumes at 8.
    assert_eq!(flow.request(low).unwrap().state, RequestState::Active);
    assert_eq!(*flow.work_context::<u32>(low_work).unwrap(), 1);
    assert_eq!(
        flow.work_progress(low_work).unwrap().useful_elapsed,
        duration(0)
    );

    let mut saw_old_deadline_noop = false;
    while let Some(dispatch) = flow.step().unwrap() {
        assert!(dispatch.error.is_none());
        if dispatch.at == at(10) && dispatch.records.is_empty() {
            saw_old_deadline_noop = true;
        }
    }
    assert!(
        saw_old_deadline_noop,
        "the pre-Restart completion token is stale"
    );
    assert_eq!(flow.request(low).unwrap().state, RequestState::Completed);
    let finished = flow.work_progress(low_work).unwrap();
    assert_eq!(finished.state, WorkState::Completed);
    assert_eq!(finished.useful_elapsed, duration(10));
    assert_eq!(finished.attempt_revision, 1);
    assert_eq!(
        flow.request_queue_order_key(low).unwrap().unwrap(),
        &low_key
    );
}

#[test]
fn queue_order_reprioritize_changes_primary_only() {
    let mut flow = FlowRuntime::new();
    let owner = flow.spawn_actor().unwrap();
    let resource = stable_resource(&mut flow);
    let holder = flow
        .acquire(resource)
        .owner(owner)
        .at(at(0))
        .priority(0)
        .queue_order_key(ready_key(0, "holder"))
        .submit()
        .unwrap();
    flow.step().unwrap().unwrap();
    let holder_lease = flow.request(holder).unwrap().lease.unwrap();

    let b = flow
        .acquire(resource)
        .owner(owner)
        .at(at(1))
        .priority(0)
        .queue_order_key(ready_key(2, "B"))
        .submit()
        .unwrap();
    let a_key = ready_key(1, "A");
    let a = flow
        .acquire(resource)
        .owner(owner)
        .at(at(1))
        .priority(10)
        .queue_order_key(a_key.clone())
        .submit()
        .unwrap();
    flow.step().unwrap().unwrap();
    flow.step().unwrap().unwrap();
    assert_eq!(flow.resource(resource).unwrap().queued, vec![b, a]);
    assert_eq!(flow.request_queue_order_key(a).unwrap().unwrap(), &a_key);

    flow.reprioritize(a, 0, at(2)).unwrap();
    flow.step().unwrap().unwrap(); // Reprioritize applies through real dispatch.
    assert_eq!(flow.request(a).unwrap().priority_level, 0);
    assert_eq!(flow.request_queue_order_key(a).unwrap().unwrap(), &a_key);
    assert_eq!(flow.resource(resource).unwrap().queued, vec![a, b]);

    flow.release(holder_lease, at(3)).unwrap();
    flow.step().unwrap().unwrap();
    assert_eq!(flow.request(a).unwrap().state, RequestState::Active);
    assert_eq!(flow.request(b).unwrap().state, RequestState::Queued);
    assert_eq!(flow.request_queue_order_key(a).unwrap().unwrap(), &a_key);
}

thread_local! {
    static QUEUE_ORDER_CODEC_CALLS: Cell<usize> = const { Cell::new(0) };
}

fn encode_queue_order_u64(
    value: &u64,
    _: usize,
) -> Result<Vec<u8>, kairo_ecs_des::FlowCheckpointCodecError> {
    QUEUE_ORDER_CODEC_CALLS.with(|calls| calls.set(calls.get() + 1));
    Ok(value.to_le_bytes().to_vec())
}

fn decode_queue_order_u64(
    bytes: &[u8],
    _: &kairo_ecs_des::FlowCheckpointRebindV1,
) -> Result<u64, kairo_ecs_des::FlowCheckpointCodecError> {
    let bytes: [u8; 8] = bytes
        .try_into()
        .map_err(|_| kairo_ecs_des::FlowCheckpointCodecError("bad u64".into()))?;
    Ok(u64::from_le_bytes(bytes))
}

fn queue_order_codecs() -> kairo_ecs_des::FlowCheckpointCodecs {
    let mut codecs = kairo_ecs_des::FlowCheckpointCodecs::new();
    codecs
        .register_context(
            "queue-order-checkpoint",
            1,
            encode_queue_order_u64,
            decode_queue_order_u64,
        )
        .unwrap();
    codecs
        .register_work_handlers::<u64>(
            "queue-order-checkpoint",
            kairo_ecs_des::FlowHandlerCodeIds::default(),
            kairo_ecs_des::WorkHandlers::default(),
        )
        .unwrap();
    codecs
}

fn assert_queue_order_checkpoint_rejected(
    flow: &FlowRuntime,
    codecs: &kairo_ecs_des::FlowCheckpointCodecs,
) {
    let expected = kairo_ecs_des::FlowCheckpointError::InvalidState(
        "V1/V2/V3 cannot represent stable queue ordering",
    );
    for capture in ["v1", "v2", "v3"] {
        QUEUE_ORDER_CODEC_CALLS.with(|calls| calls.set(0));
        let error = match capture {
            "v1" => flow
                .capture_checkpoint(codecs, Default::default())
                .unwrap_err(),
            "v2" => flow
                .capture_checkpoint_v2(codecs, Default::default())
                .unwrap_err(),
            _ => flow
                .capture_checkpoint_v3(codecs, Default::default())
                .unwrap_err(),
        };
        assert_eq!(error, expected, "capture entrypoint {capture}");
        assert_eq!(
            QUEUE_ORDER_CODEC_CALLS.with(Cell::get),
            0,
            "capture entrypoint {capture} must reject before codecs"
        );
    }
}

#[test]
fn queue_order_legacy_capture_fails_before_codec_for_all_key_lifetimes() {
    let mut flow = FlowRuntime::new();
    let owner = flow.spawn_actor().unwrap();
    flow.register_work_handlers::<u64>(
        "queue-order-checkpoint",
        kairo_ecs_des::WorkHandlers::default(),
    )
    .unwrap();
    let work = flow
        .create_work(owner, duration(20), "queue-order-checkpoint", 73_u64)
        .unwrap();
    let codecs = queue_order_codecs();
    let resource = stable_resource(&mut flow);

    // An empty opt-in resource is unrepresentable in direct legacy versions.
    assert_queue_order_checkpoint_rejected(&flow, &codecs);
    assert_eq!(
        flow.capture_checkpoint_image(&codecs, Default::default())
            .unwrap(),
        kairo_ecs_des::FlowCheckpointImage::V4(
            flow.capture_checkpoint_v4(&codecs, Default::default())
                .unwrap()
        )
    );

    // A pending request retains its immutable key before Submit dispatch.
    let pending = flow
        .acquire(resource)
        .owner(owner)
        .timed_work(work)
        .at(at(1))
        .queue_order_key(ready_key(1, "pending"))
        .preemptible(PreemptionStrategy::Suspend)
        .submit()
        .unwrap();
    assert_eq!(flow.request(pending).unwrap().state, RequestState::Pending);
    assert_queue_order_checkpoint_rejected(&flow, &codecs);

    flow.step().unwrap().unwrap();
    assert_eq!(flow.request(pending).unwrap().state, RequestState::Active);
    assert_queue_order_checkpoint_rejected(&flow, &codecs);

    let queued_work = flow
        .create_work(owner, duration(10), "queue-order-checkpoint", 74_u64)
        .unwrap();
    let queued = flow
        .acquire(resource)
        .owner(owner)
        .timed_work(queued_work)
        .at(at(2))
        .queue_order_key(ready_key(2, "queued"))
        .submit()
        .unwrap();
    flow.step().unwrap().unwrap();
    assert_eq!(flow.request(queued).unwrap().state, RequestState::Queued);
    assert_queue_order_checkpoint_rejected(&flow, &codecs);

    let urgent = flow
        .acquire(resource)
        .owner(owner)
        .at(at(3))
        .priority(-1)
        .can_preempt(true)
        .queue_order_key(ready_key(0, "urgent"))
        .submit()
        .unwrap();
    flow.step().unwrap().unwrap();
    assert_eq!(
        flow.request(pending).unwrap().state,
        RequestState::Suspended
    );
    assert_eq!(flow.request(urgent).unwrap().state, RequestState::Active);
    assert_queue_order_checkpoint_rejected(&flow, &codecs);

    flow.cancel(queued, at(3)).unwrap();
    flow.step().unwrap().unwrap();
    assert_eq!(flow.request(queued).unwrap().state, RequestState::Cancelled);
    assert_queue_order_checkpoint_rejected(&flow, &codecs);

    // After all keyed requests are terminal, resource removal leaves retained
    // immutable request audit keys. Legacy images still fail before codecs.
    flow.cancel(pending, at(3)).unwrap();
    flow.cancel(urgent, at(3)).unwrap();
    flow.step().unwrap().unwrap();
    flow.step().unwrap().unwrap();
    flow.remove_resource(resource).unwrap();
    flow.step().unwrap().unwrap();
    assert_eq!(
        flow.resource_queue_ordering(resource),
        Err(kairo_ecs_des::FlowError::InvalidResource)
    );
    assert_queue_order_checkpoint_rejected(&flow, &codecs);
    assert_eq!(
        flow.capture_checkpoint_image(&codecs, Default::default())
            .unwrap(),
        kairo_ecs_des::FlowCheckpointImage::V4(
            flow.capture_checkpoint_v4(&codecs, Default::default())
                .unwrap()
        )
    );

    // Keyed Deferred mode remains guarded even with grant eligibility disabled
    // and the keyed request already queued behind the gate.
    let mut deferred = FlowRuntime::new();
    let owner = deferred.spawn_actor().unwrap();
    deferred
        .register_work_handlers::<u64>(
            "queue-order-checkpoint",
            kairo_ecs_des::WorkHandlers::default(),
        )
        .unwrap();
    deferred
        .create_work(owner, duration(3), "queue-order-checkpoint", 76_u64)
        .unwrap();
    let resource = stable_resource_mode(&mut deferred, FlowAllocationMode::Deferred);
    deferred
        .schedule_resource_grant_enabled(resource, false, at(0), -1)
        .unwrap();
    let disabled = deferred
        .acquire(resource)
        .owner(owner)
        .at(at(0))
        .queue_order_key(ready_key(0, "disabled"))
        .submit()
        .unwrap();
    deferred.step().unwrap().unwrap();
    deferred.step().unwrap().unwrap();
    assert_eq!(
        deferred.request(disabled).unwrap().state,
        RequestState::Queued
    );
    assert_queue_order_checkpoint_rejected(&deferred, &codecs);

    // A completed request still retains the supplied key and therefore cannot
    // be represented by any legacy checkpoint version.
    let mut completed = FlowRuntime::new();
    let owner = completed.spawn_actor().unwrap();
    completed
        .register_work_handlers::<u64>(
            "queue-order-checkpoint",
            kairo_ecs_des::WorkHandlers::default(),
        )
        .unwrap();
    let work = completed
        .create_work(owner, duration(1), "queue-order-checkpoint", 77_u64)
        .unwrap();
    let resource = stable_resource(&mut completed);
    let request = completed
        .acquire(resource)
        .owner(owner)
        .at(at(0))
        .timed_work(work)
        .queue_order_key(ready_key(0, "completed"))
        .submit()
        .unwrap();
    completed.step().unwrap().unwrap();
    completed.step().unwrap().unwrap();
    assert_eq!(
        completed.request(request).unwrap().state,
        RequestState::Completed
    );
    assert_queue_order_checkpoint_rejected(&completed, &codecs);

    let mut released = FlowRuntime::new();
    let owner = released.spawn_actor().unwrap();
    let resource = stable_resource(&mut released);
    let request = released
        .acquire(resource)
        .owner(owner)
        .at(at(0))
        .queue_order_key(ready_key(0, "released"))
        .submit()
        .unwrap();
    released.step().unwrap().unwrap();
    let lease = released.request(request).unwrap().lease.unwrap();
    released.release(lease, at(0)).unwrap();
    released.step().unwrap().unwrap();
    assert_eq!(
        released.request(request).unwrap().state,
        RequestState::Released
    );
    assert_queue_order_checkpoint_rejected(&released, &codecs);

    let mut timed_out = FlowRuntime::new();
    let owner = timed_out.spawn_actor().unwrap();
    let resource = timed_out
        .create_resource_with_queue_ordering(
            0,
            FlowAllocationMode::Eager,
            FlowQueueOrdering::StableKey,
        )
        .unwrap();
    let request = timed_out
        .acquire(resource)
        .owner(owner)
        .at(at(0))
        .deadline(at(1))
        .queue_order_key(ready_key(0, "timed-out"))
        .submit()
        .unwrap();
    timed_out.step().unwrap().unwrap();
    timed_out.step().unwrap().unwrap();
    assert_eq!(
        timed_out.request(request).unwrap().state,
        RequestState::TimedOut
    );
    assert_queue_order_checkpoint_rejected(&timed_out, &codecs);

    let mut aborted = FlowRuntime::new();
    let owner = aborted.spawn_actor().unwrap();
    let resource = stable_resource(&mut aborted);
    let low_work = aborted
        .create_work(owner, duration(20), "queue-order-aborted", ())
        .unwrap();
    let low = aborted
        .acquire(resource)
        .owner(owner)
        .at(at(0))
        .timed_work(low_work)
        .preemptible(PreemptionStrategy::Abort)
        .queue_order_key(ready_key(0, "aborted"))
        .submit()
        .unwrap();
    aborted.step().unwrap().unwrap();
    let urgent = aborted
        .acquire(resource)
        .owner(owner)
        .at(at(1))
        .priority(-1)
        .can_preempt(true)
        .queue_order_key(ready_key(0, "aborter"))
        .submit()
        .unwrap();
    aborted.step().unwrap().unwrap();
    assert_eq!(aborted.request(low).unwrap().state, RequestState::Aborted);
    assert_eq!(aborted.request(urgent).unwrap().state, RequestState::Active);
    assert_queue_order_checkpoint_rejected(&aborted, &codecs);

    // An AdmissionOrder control remains capturable and actually exercises its
    // context encoder, so the zero-call keyed assertions are non-vacuous.
    let mut legacy = FlowRuntime::new();
    let owner = legacy.spawn_actor().unwrap();
    legacy
        .register_work_handlers::<u64>(
            "queue-order-checkpoint",
            kairo_ecs_des::WorkHandlers::default(),
        )
        .unwrap();
    legacy
        .create_work(owner, duration(2), "queue-order-checkpoint", 75_u64)
        .unwrap();
    legacy.create_resource(1).unwrap();
    QUEUE_ORDER_CODEC_CALLS.with(|calls| calls.set(0));
    let _ = legacy
        .capture_checkpoint(&codecs, Default::default())
        .unwrap();
    assert!(QUEUE_ORDER_CODEC_CALLS.with(Cell::get) > 0);
}

#[test]
fn queue_order_legacy_restore_roundtrips_captured_admission_images_and_continues() {
    use kairo_ecs_des::{FlowCheckpointImage, FlowCheckpointLimits};
    let codecs = kairo_ecs_des::FlowCheckpointCodecs::new();
    let mut source = FlowRuntime::new();
    let owner = source.spawn_actor().unwrap();
    let resource = source.create_resource(1).unwrap();
    let holder = source
        .acquire(resource)
        .owner(owner)
        .priority(0)
        .submit()
        .unwrap();
    source.step().unwrap().unwrap();
    let waiting_a = source
        .acquire(resource)
        .owner(owner)
        .priority(2)
        .submit()
        .unwrap();
    let waiting_b = source
        .acquire(resource)
        .owner(owner)
        .priority(2)
        .submit()
        .unwrap();
    source.step().unwrap().unwrap();
    source.step().unwrap().unwrap();

    // These bytes come from images captured by this candidate; literal historical
    // legacy-envelope parity is qualified separately by the outer C2 goldens.
    let limits = FlowCheckpointLimits::default();
    let v1 = source.capture_checkpoint(&codecs, limits).unwrap();
    let v2 = source.capture_checkpoint_v2(&codecs, limits).unwrap();
    let v3 = source.capture_checkpoint_v3(&codecs, limits).unwrap();
    let v1_bytes = v1.encode_wire_v1(Default::default()).unwrap();
    let v2_bytes = v2.encode_wire_v2(Default::default()).unwrap();
    let v3_bytes = v3.encode_wire_v3(Default::default()).unwrap();

    let (mut restored_v1, _) =
        FlowRuntime::restore_checkpoint_with_rebind(v1.clone(), &codecs, limits).unwrap();
    let (mut restored_v2, _) =
        FlowRuntime::restore_checkpoint_v2_with_rebind(v2.clone(), &codecs, limits).unwrap();
    let (mut restored_v3, _) =
        FlowRuntime::restore_checkpoint_v3_with_rebind(v3.clone(), &codecs, limits).unwrap();
    assert_eq!(restored_v1.capture_checkpoint(&codecs, limits).unwrap(), v1);
    assert_eq!(
        restored_v2.capture_checkpoint_v2(&codecs, limits).unwrap(),
        v2
    );
    assert_eq!(
        restored_v3.capture_checkpoint_v3(&codecs, limits).unwrap(),
        v3
    );
    assert_eq!(
        restored_v1
            .capture_checkpoint(&codecs, limits)
            .unwrap()
            .encode_wire_v1(Default::default())
            .unwrap(),
        v1_bytes
    );
    assert_eq!(
        restored_v2
            .capture_checkpoint_v2(&codecs, limits)
            .unwrap()
            .encode_wire_v2(Default::default())
            .unwrap(),
        v2_bytes
    );
    assert_eq!(
        restored_v3
            .capture_checkpoint_v3(&codecs, limits)
            .unwrap()
            .encode_wire_v3(Default::default())
            .unwrap(),
        v3_bytes
    );

    for restored in [&mut restored_v1, &mut restored_v2, &mut restored_v3] {
        assert_eq!(
            restored.resource(resource).unwrap().queued,
            vec![waiting_a, waiting_b]
        );
        restored
            .release(restored.request(holder).unwrap().lease.unwrap(), at(0))
            .unwrap();
        restored.step().unwrap().unwrap();
        assert_eq!(
            restored.request(waiting_a).unwrap().state,
            RequestState::Active
        );
        assert_eq!(
            restored.request(waiting_b).unwrap().state,
            RequestState::Queued
        );
    }

    let routed = source.capture_checkpoint_image(&codecs, limits).unwrap();
    assert_eq!(routed, FlowCheckpointImage::V1(v1));
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct QueueOrderCallbackContext {
    owner: kairo_ecs_types::EntityId,
    admission: kairo_ecs_des::ResourceId,
    stable: kairo_ecs_des::ResourceId,
}

fn emit_missing_stable_key_batch(
    context: &mut QueueOrderCallbackContext,
    snapshot: &kairo_ecs_des::FlowCallbackSnapshot,
    sink: &mut kairo_ecs_des::FlowCommandSink,
) {
    for resource in [context.admission, context.stable] {
        sink.emit(kairo_ecs_des::FlowOwnedCommand::Acquire(
            kairo_ecs_des::FlowAcquireCommand {
                resource,
                owner: context.owner,
                work: None,
                at: snapshot.delivery.at,
                priority_level: 0,
                deadline: None,
                scheduler_priority: 0,
                timed: false,
                can_preempt: false,
                preemptible: None,
            },
        ))
        .unwrap();
    }
}

#[test]
fn queue_order_callback_missing_key_rejects_whole_batch() {
    use kairo_ecs_des::{FlowBatchReceipt, FlowContinuations};
    let mut flow = FlowRuntime::new();
    let owner = flow.spawn_actor().unwrap();
    let admission = flow.create_resource(1).unwrap();
    let stable = stable_resource(&mut flow);
    flow.register_work_continuations(
        "queue-order-callback",
        FlowContinuations {
            on_complete: Some(emit_missing_stable_key_batch),
            ..FlowContinuations::default()
        },
    )
    .unwrap();
    let trigger_work = flow
        .create_work(
            owner,
            duration(1),
            "queue-order-callback",
            QueueOrderCallbackContext {
                owner,
                admission,
                stable,
            },
        )
        .unwrap();
    flow.acquire(admission)
        .owner(owner)
        .at(at(0))
        .timed_work(trigger_work)
        .submit()
        .unwrap();
    flow.step().unwrap().unwrap(); // Grant.
    flow.step().unwrap().unwrap(); // Completion schedules Notify.
    let before = flow.budget_snapshot();
    let before_work = flow.work_progress(trigger_work).unwrap();
    let before_context = *flow
        .work_context::<QueueOrderCallbackContext>(trigger_work)
        .unwrap();
    let notification = flow.step().unwrap().unwrap();
    assert_eq!(notification.callback_batches.len(), 1);
    match &notification.callback_batches[0] {
        FlowBatchReceipt::Rejected(rejection) => {
            assert_eq!(rejection.error, kairo_ecs_des::FlowError::InvalidState);
            assert!(rejection.failed_ticket.is_some());
        }
        FlowBatchReceipt::Accepted(_) => {
            panic!("keyless StableKey callback must reject atomically")
        }
    }
    assert_eq!(flow.work_progress(trigger_work).unwrap(), before_work);
    assert_eq!(
        *flow
            .work_context::<QueueOrderCallbackContext>(trigger_work)
            .unwrap(),
        before_context
    );
    let after = flow.budget_snapshot();
    assert_eq!(
        after.scheduler.scheduled_events,
        before.scheduler.scheduled_events
    );
    assert_eq!(
        after.scheduler.pending_events + 1,
        before.scheduler.pending_events
    );
    assert_eq!(
        after.scheduler.dispatched_events,
        before.scheduler.dispatched_events + 1
    );
    assert_eq!(
        flow.resource(admission).unwrap().queued,
        Vec::<kairo_ecs_des::RequestId>::new()
    );
    assert_eq!(
        flow.resource(stable).unwrap().queued,
        Vec::<kairo_ecs_des::RequestId>::new()
    );
    assert_eq!(flow.resource(admission).unwrap().allocations.len(), 0);
    assert_eq!(flow.resource(stable).unwrap().allocations.len(), 0);
    assert!(flow.step().unwrap().is_none());
}
