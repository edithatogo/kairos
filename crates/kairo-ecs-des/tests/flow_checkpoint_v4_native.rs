use kairo_ecs_des::{
    FlowAllocationMode, FlowCheckpointCodecError, FlowCheckpointCodecs, FlowCheckpointError,
    FlowCheckpointLimits, FlowQueueOrderKey, FlowQueueOrdering, FlowRequestQueueOrderV4,
    FlowRuntime, RequestId, RequestState, ResourceId, FLOW_CHECKPOINT_VERSION_V4,
};
use kairo_ecs_types::{SimDuration, SimTime};
use std::cell::Cell;

thread_local! {
    static CODEC_CALLS: Cell<usize> = const { Cell::new(0) };
    static ENCODE_CALLS: Cell<usize> = const { Cell::new(0) };
}

fn at(value: u128) -> SimTime {
    SimTime::from_ticks(value)
}

fn duration(value: u128) -> SimDuration {
    SimDuration::from_ticks(value)
}

fn encode_u64(value: &u64, remaining: usize) -> Result<Vec<u8>, FlowCheckpointCodecError> {
    ENCODE_CALLS.with(|calls| calls.set(calls.get() + 1));
    if remaining < 8 {
        return Err(FlowCheckpointCodecError("payload limit".into()));
    }
    Ok(value.to_le_bytes().to_vec())
}

fn decode_u64(
    bytes: &[u8],
    _: &kairo_ecs_des::FlowCheckpointRebindV1,
) -> Result<u64, FlowCheckpointCodecError> {
    CODEC_CALLS.with(|calls| calls.set(calls.get() + 1));
    let bytes: [u8; 8] = bytes
        .try_into()
        .map_err(|_| FlowCheckpointCodecError("bad u64 payload".into()))?;
    Ok(u64::from_le_bytes(bytes))
}

fn codecs() -> FlowCheckpointCodecs {
    let mut codecs = FlowCheckpointCodecs::new();
    codecs
        .register_context("ctx", 1, encode_u64, decode_u64)
        .unwrap();
    codecs
}

fn key(value: &[u8]) -> FlowQueueOrderKey {
    FlowQueueOrderKey::new(value).unwrap()
}

fn mixed_seed() -> (
    FlowRuntime,
    FlowCheckpointCodecs,
    ResourceId,
    ResourceId,
    RequestId,
    RequestId,
    RequestId,
    RequestId,
    RequestId,
) {
    let mut flow = FlowRuntime::new();
    let owner = flow.spawn_actor().unwrap();
    let stable = flow
        .create_resource_with_queue_ordering(
            1,
            FlowAllocationMode::Eager,
            FlowQueueOrdering::StableKey,
        )
        .unwrap();
    let admission = flow.create_resource(1).unwrap();
    let work = flow
        .create_work(owner, duration(100), "ctx", 41_u64)
        .unwrap();
    let active = flow
        .acquire(stable)
        .owner(owner)
        .timed_work(work)
        .queue_order_key(key(b"holder"))
        .submit()
        .unwrap();
    flow.step().unwrap().unwrap();
    let late = flow
        .acquire(stable)
        .owner(owner)
        .queue_order_key(key(b"z-late"))
        .submit()
        .unwrap();
    flow.step().unwrap().unwrap();
    let early = flow
        .acquire(stable)
        .owner(owner)
        .queue_order_key(key(b"a-early"))
        .submit()
        .unwrap();
    flow.step().unwrap().unwrap();
    let admission_active = flow.acquire(admission).owner(owner).submit().unwrap();
    flow.step().unwrap().unwrap();
    let pending = flow
        .acquire(stable)
        .owner(owner)
        .at(at(10))
        .queue_order_key(key(b"0-future"))
        .submit()
        .unwrap();
    (
        flow,
        codecs(),
        stable,
        admission,
        active,
        late,
        early,
        admission_active,
        pending,
    )
}

fn removed_terminal_seed() -> (
    FlowRuntime,
    FlowCheckpointCodecs,
    ResourceId,
    [RequestId; 2],
) {
    let mut flow = FlowRuntime::new();
    let owner = flow.spawn_actor().unwrap();
    let resource = flow
        .create_resource_with_queue_ordering(
            0,
            FlowAllocationMode::Eager,
            FlowQueueOrdering::StableKey,
        )
        .unwrap();
    let first = flow
        .acquire(resource)
        .owner(owner)
        .queue_order_key(key(b"first"))
        .submit()
        .unwrap();
    flow.step().unwrap().unwrap();
    let second = flow
        .acquire(resource)
        .owner(owner)
        .queue_order_key(key(b"second"))
        .submit()
        .unwrap();
    flow.step().unwrap().unwrap();
    flow.cancel(first, at(1)).unwrap();
    flow.cancel(second, at(1)).unwrap();
    flow.step().unwrap().unwrap();
    flow.step().unwrap().unwrap();
    assert_eq!(flow.request(first).unwrap().state, RequestState::Cancelled);
    assert_eq!(flow.request(second).unwrap().state, RequestState::Cancelled);
    flow.remove_resource(resource).unwrap();
    flow.step().unwrap().unwrap();
    (flow, FlowCheckpointCodecs::new(), resource, [first, second])
}

fn assert_reject_before_codecs(
    image: kairo_ecs_des::FlowCheckpointV4,
    codecs: &FlowCheckpointCodecs,
    limits: FlowCheckpointLimits,
) -> FlowCheckpointError {
    CODEC_CALLS.with(|calls| calls.set(0));
    let error = match FlowRuntime::restore_checkpoint_v4(image, codecs, limits) {
        Ok(_) => panic!("malformed or over-limit V4 image unexpectedly restored"),
        Err(error) => error,
    };
    CODEC_CALLS.with(|calls| assert_eq!(calls.get(), 0));
    error
}

#[test]
fn v4_admission_order_roundtrip_and_legacy_versions() {
    let mut source = FlowRuntime::new();
    let resource = source.create_resource(1).unwrap();
    let owner = source.spawn_actor().unwrap();
    let request = source.acquire(resource).owner(owner).submit().unwrap();
    source.step().unwrap().unwrap();
    let codecs = FlowCheckpointCodecs::new();
    let limits = FlowCheckpointLimits::default();
    let image = source.capture_checkpoint_v4(&codecs, limits).unwrap();
    assert_eq!(image.version, FLOW_CHECKPOINT_VERSION_V4);
    assert_eq!(
        image.queue_order_modes,
        vec![(resource, FlowQueueOrdering::AdmissionOrder)]
    );
    assert_eq!(
        image.request_queue_orders,
        vec![(request, FlowRequestQueueOrderV4::AdmissionOrder)]
    );
    let mut restored = FlowRuntime::restore_checkpoint_v4(image.clone(), &codecs, limits).unwrap();
    assert_eq!(
        restored.request(request).unwrap().state,
        RequestState::Active
    );
    assert_eq!(
        restored.capture_checkpoint_v4(&codecs, limits).unwrap(),
        image
    );

    let v1 = source.capture_checkpoint(&codecs, limits).unwrap();
    let v2 = source.capture_checkpoint_v2(&codecs, limits).unwrap();
    let v3 = source.capture_checkpoint_v3(&codecs, limits).unwrap();
    assert_eq!(
        FlowRuntime::restore_checkpoint(v1, &codecs, limits)
            .unwrap()
            .request(request)
            .unwrap()
            .state,
        RequestState::Active
    );
    assert_eq!(
        FlowRuntime::restore_checkpoint_v2(v2, &codecs, limits)
            .unwrap()
            .request(request)
            .unwrap()
            .state,
        RequestState::Active
    );
    assert_eq!(
        FlowRuntime::restore_checkpoint_v3(v3, &codecs, limits)
            .unwrap()
            .request(request)
            .unwrap()
            .state,
        RequestState::Active
    );

    let lease = source.request(request).unwrap().lease.unwrap();
    let restored_lease = restored.request(request).unwrap().lease.unwrap();
    source.release(lease, at(1)).unwrap();
    restored.release(restored_lease, at(1)).unwrap();
    assert_eq!(source.step().unwrap(), restored.step().unwrap());
    assert_eq!(
        source.request(request).unwrap(),
        restored.request(request).unwrap()
    );
    assert_eq!(
        source.request(request).unwrap().state,
        RequestState::Released
    );
    assert_eq!(
        source.capture_checkpoint_v4(&codecs, limits).unwrap(),
        restored.capture_checkpoint_v4(&codecs, limits).unwrap()
    );

    let mut empty = FlowRuntime::new();
    let empty_resource = empty.create_resource(0).unwrap();
    let empty_image = empty.capture_checkpoint_v4(&codecs, limits).unwrap();
    let empty_restored = FlowRuntime::restore_checkpoint_v4(empty_image, &codecs, limits).unwrap();
    assert_eq!(
        empty_restored
            .resource_queue_ordering(empty_resource)
            .unwrap(),
        FlowQueueOrdering::AdmissionOrder
    );
}

#[test]
fn v4_stable_pending_queued_roundtrip_preserves_effective_order() {
    let (mut source, codecs, _, _, active, late, early, _, pending) = mixed_seed();
    assert_eq!(
        source.request(pending).unwrap().state,
        RequestState::Pending
    );
    assert_eq!(source.request(late).unwrap().state, RequestState::Queued);
    assert_eq!(source.request(early).unwrap().state, RequestState::Queued);
    assert!(
        source.request(late).unwrap().admission_sequence.unwrap()
            < source.request(early).unwrap().admission_sequence.unwrap()
    );
    let lease = source.request(active).unwrap().lease.unwrap();
    let image = source
        .capture_checkpoint_v4(&codecs, FlowCheckpointLimits::default())
        .unwrap();
    assert!(image.request_queue_orders.iter().any(|(id, row)| {
        *id == pending
            && matches!(row, FlowRequestQueueOrderV4::StableKey(bytes) if bytes == b"0-future")
    }));
    let mut restored =
        FlowRuntime::restore_checkpoint_v4(image, &codecs, FlowCheckpointLimits::default())
            .unwrap();
    assert_eq!(
        restored
            .capture_checkpoint_v4(&codecs, FlowCheckpointLimits::default())
            .unwrap(),
        source
            .capture_checkpoint_v4(&codecs, FlowCheckpointLimits::default())
            .unwrap()
    );
    let restored_lease = restored.request(active).unwrap().lease.unwrap();
    assert_eq!(lease, restored_lease);
    source.release(lease, at(1)).unwrap();
    restored.release(restored_lease, at(1)).unwrap();
    source.step().unwrap().unwrap();
    restored.step().unwrap().unwrap();
    assert_eq!(source.request(early).unwrap().state, RequestState::Active);
    assert_eq!(restored.request(early).unwrap().state, RequestState::Active);
    assert_eq!(source.request(late).unwrap().state, RequestState::Queued);
    assert_eq!(restored.request(late).unwrap().state, RequestState::Queued);
    assert_eq!(
        source.request(pending).unwrap().state,
        RequestState::Pending
    );
    assert_eq!(
        restored.request(pending).unwrap().state,
        RequestState::Pending
    );
}

#[test]
fn v4_active_suspended_resume_preserves_key_and_enqueue_sequence() {
    let mut source = FlowRuntime::new();
    let owner = source.spawn_actor().unwrap();
    let resource = source
        .create_resource_with_queue_ordering(
            1,
            FlowAllocationMode::Eager,
            FlowQueueOrdering::StableKey,
        )
        .unwrap();
    let b_work = source
        .create_work(owner, duration(30), "ctx", 2_u64)
        .unwrap();
    let b = source
        .acquire(resource)
        .owner(owner)
        .timed_work(b_work)
        .preemptible(kairo_ecs_des::PreemptionStrategy::Suspend)
        .queue_order_key(key(b"b"))
        .submit()
        .unwrap();
    source.step().unwrap().unwrap();
    let u_work = source
        .create_work(owner, duration(3), "ctx", 3_u64)
        .unwrap();
    let urgent = source
        .acquire(resource)
        .owner(owner)
        .at(at(1))
        .timed_work(u_work)
        .priority(-1)
        .can_preempt(true)
        .queue_order_key(key(b"u"))
        .submit()
        .unwrap();
    source.step().unwrap().unwrap();
    assert_eq!(source.request(b).unwrap().state, RequestState::Suspended);
    assert_eq!(source.request(urgent).unwrap().state, RequestState::Active);
    let a_work = source
        .create_work(owner, duration(1), "ctx", 4_u64)
        .unwrap();
    let a = source
        .acquire(resource)
        .owner(owner)
        .at(at(2))
        .timed_work(a_work)
        .queue_order_key(key(b"a"))
        .submit()
        .unwrap();
    source.step().unwrap().unwrap();
    source.reprioritize(b, -2, at(2)).unwrap();
    source.step().unwrap().unwrap();
    let original_sequence = source.request(b).unwrap().admission_sequence.unwrap();
    let checkpoint_codecs = codecs();
    let limits = FlowCheckpointLimits::default();
    let image = source
        .capture_checkpoint_v4(&checkpoint_codecs, limits)
        .unwrap();
    let mut restored =
        FlowRuntime::restore_checkpoint_v4(image.clone(), &checkpoint_codecs, limits).unwrap();
    assert_eq!(
        restored
            .capture_checkpoint_v4(&checkpoint_codecs, limits)
            .unwrap(),
        image
    );
    assert_eq!(
        source
            .request_queue_order_key(b)
            .unwrap()
            .unwrap()
            .as_bytes(),
        restored
            .request_queue_order_key(b)
            .unwrap()
            .unwrap()
            .as_bytes()
    );
    assert_eq!(
        restored.request(b).unwrap().admission_sequence,
        Some(original_sequence)
    );
    for _ in 0..32 {
        let source_dispatch = source.step().unwrap();
        let restored_dispatch = restored.step().unwrap();
        assert_eq!(source_dispatch, restored_dispatch);
        for request in [a, b, urgent] {
            assert_eq!(
                source.request(request).unwrap(),
                restored.request(request).unwrap()
            );
        }
        for work in [a_work, b_work, u_work] {
            assert_eq!(
                source.work_progress(work).unwrap(),
                restored.work_progress(work).unwrap()
            );
            assert_eq!(
                source.work_context::<u64>(work).unwrap(),
                restored.work_context::<u64>(work).unwrap()
            );
        }
        if source_dispatch.is_none() {
            break;
        }
    }
    assert_eq!(source.request(a).unwrap().state, RequestState::Completed);
    assert_eq!(source.request(b).unwrap().state, RequestState::Completed);
    assert_eq!(
        source.request(urgent).unwrap().state,
        RequestState::Completed
    );
    assert_eq!(source.request(a).unwrap(), restored.request(a).unwrap());
    assert_eq!(source.request(b).unwrap(), restored.request(b).unwrap());
    assert_eq!(
        source.request(urgent).unwrap(),
        restored.request(urgent).unwrap()
    );
    for work in [a_work, b_work, u_work] {
        assert_eq!(
            source.work_progress(work).unwrap(),
            restored.work_progress(work).unwrap()
        );
        assert_eq!(
            source.work_context::<u64>(work).unwrap(),
            restored.work_context::<u64>(work).unwrap()
        );
    }
    assert_eq!(
        source
            .capture_checkpoint_v4(&checkpoint_codecs, limits)
            .unwrap(),
        restored
            .capture_checkpoint_v4(&checkpoint_codecs, limits)
            .unwrap()
    );
    assert_eq!(
        source.request(b).unwrap().admission_sequence,
        Some(original_sequence)
    );
    assert_eq!(
        restored.request(b).unwrap().admission_sequence,
        Some(original_sequence)
    );
}

#[test]
fn v4_removed_resource_terminal_keys_roundtrip() {
    let (source, codecs, resource, requests) = removed_terminal_seed();
    let image = source
        .capture_checkpoint_v4(&codecs, FlowCheckpointLimits::default())
        .unwrap();
    assert!(!image
        .queue_order_modes
        .iter()
        .any(|(id, _)| *id == resource));
    for request in requests {
        assert!(image.request_queue_orders.iter().any(|(id, row)| {
            *id == request && matches!(row, FlowRequestQueueOrderV4::StableKey(_))
        }));
    }
    let restored =
        FlowRuntime::restore_checkpoint_v4(image.clone(), &codecs, FlowCheckpointLimits::default())
            .unwrap();
    for request in requests {
        assert_eq!(
            restored.request(request).unwrap().state,
            RequestState::Cancelled
        );
        assert!(restored.request_queue_order_key(request).unwrap().is_some());
    }
    let mut inconsistent = image;
    let second = requests[1];
    let (_, row) = inconsistent
        .request_queue_orders
        .iter_mut()
        .find(|(id, _)| *id == second)
        .unwrap();
    *row = FlowRequestQueueOrderV4::AdmissionOrder;
    assert!(matches!(
        FlowRuntime::restore_checkpoint_v4(inconsistent, &codecs, FlowCheckpointLimits::default()),
        Err(FlowCheckpointError::InvalidState(_))
    ));
}

#[test]
fn v4_mixed_modes_have_canonical_complete_rows() {
    let (source, codecs, stable, admission, active, late, early, admission_active, pending) =
        mixed_seed();
    let image = source
        .capture_checkpoint_v4(&codecs, FlowCheckpointLimits::default())
        .unwrap();
    assert!(image
        .queue_order_modes
        .windows(2)
        .all(|pair| pair[0].0 < pair[1].0));
    assert_eq!(image.queue_order_modes.len(), image.resources.len());
    assert!(image
        .queue_order_modes
        .contains(&(stable, FlowQueueOrdering::StableKey)));
    assert!(image
        .queue_order_modes
        .contains(&(admission, FlowQueueOrdering::AdmissionOrder)));
    assert!(image
        .request_queue_orders
        .windows(2)
        .all(|pair| pair[0].0 < pair[1].0));
    assert_eq!(image.request_queue_orders.len(), image.requests.len());
    for request in [active, late, early, pending] {
        assert!(image.request_queue_orders.iter().any(|(id, row)| {
            *id == request && matches!(row, FlowRequestQueueOrderV4::StableKey(_))
        }));
    }
    assert!(image.request_queue_orders.iter().any(|(id, row)| {
        *id == admission_active && matches!(row, FlowRequestQueueOrderV4::AdmissionOrder)
    }));
}

fn foreign_identities() -> (ResourceId, RequestId) {
    let mut foreign = FlowRuntime::new();
    for _ in 0..64 {
        foreign.spawn_actor().unwrap();
    }
    let resource = foreign.create_resource(0).unwrap();
    let owner = foreign.spawn_actor().unwrap();
    let request = foreign.acquire(resource).owner(owner).submit().unwrap();
    (resource, request)
}

#[test]
fn v4_malformed_metadata_rejects_before_codecs() {
    let (source, codecs, stable, _, active, _, _, _, _) = mixed_seed();
    let valid = source
        .capture_checkpoint_v4(&codecs, FlowCheckpointLimits::default())
        .unwrap();
    CODEC_CALLS.with(|calls| calls.set(0));
    FlowRuntime::restore_checkpoint_v4(valid.clone(), &codecs, FlowCheckpointLimits::default())
        .unwrap();
    CODEC_CALLS.with(|calls| assert!(calls.get() > 0));

    let mut wrong_version = valid.clone();
    wrong_version.version = 3;
    CODEC_CALLS.with(|calls| calls.set(0));
    assert!(matches!(
        FlowRuntime::restore_checkpoint_v4(wrong_version, &codecs, FlowCheckpointLimits::default()),
        Err(FlowCheckpointError::UnsupportedVersion(3))
    ));
    CODEC_CALLS.with(|calls| assert_eq!(calls.get(), 0));

    let mut noncanonical_modes = valid.clone();
    noncanonical_modes.queue_order_modes.swap(0, 1);
    assert!(matches!(
        assert_reject_before_codecs(noncanonical_modes, &codecs, FlowCheckpointLimits::default()),
        FlowCheckpointError::InvalidState(_)
    ));
    let mut duplicate_mode = valid.clone();
    duplicate_mode.queue_order_modes[1].0 = duplicate_mode.queue_order_modes[0].0;
    assert!(matches!(
        assert_reject_before_codecs(duplicate_mode, &codecs, FlowCheckpointLimits::default()),
        FlowCheckpointError::InvalidState(_)
    ));
    let mut missing_mode = valid.clone();
    missing_mode.queue_order_modes.pop();
    assert!(matches!(
        assert_reject_before_codecs(missing_mode, &codecs, FlowCheckpointLimits::default()),
        FlowCheckpointError::InvalidState(_)
    ));
    let (foreign_resource, foreign_request) = foreign_identities();
    let mut unknown_mode = valid.clone();
    unknown_mode.queue_order_modes.last_mut().unwrap().0 = foreign_resource;
    assert!(matches!(
        assert_reject_before_codecs(unknown_mode, &codecs, FlowCheckpointLimits::default()),
        FlowCheckpointError::InvalidState(_)
    ));

    let mut noncanonical_requests = valid.clone();
    noncanonical_requests.request_queue_orders.swap(0, 1);
    assert!(matches!(
        assert_reject_before_codecs(
            noncanonical_requests,
            &codecs,
            FlowCheckpointLimits::default()
        ),
        FlowCheckpointError::InvalidState(_)
    ));
    let mut duplicate_request = valid.clone();
    duplicate_request.request_queue_orders[1].0 = duplicate_request.request_queue_orders[0].0;
    assert!(matches!(
        assert_reject_before_codecs(duplicate_request, &codecs, FlowCheckpointLimits::default()),
        FlowCheckpointError::InvalidState(_)
    ));
    let mut missing_request = valid.clone();
    missing_request.request_queue_orders.pop();
    assert!(matches!(
        assert_reject_before_codecs(missing_request, &codecs, FlowCheckpointLimits::default()),
        FlowCheckpointError::InvalidState(_)
    ));
    let mut unknown_request = valid.clone();
    unknown_request.request_queue_orders.last_mut().unwrap().0 = foreign_request;
    assert!(matches!(
        assert_reject_before_codecs(unknown_request, &codecs, FlowCheckpointLimits::default()),
        FlowCheckpointError::InvalidState(_)
    ));

    let mut empty_key = valid.clone();
    let (_, row) = empty_key
        .request_queue_orders
        .iter_mut()
        .find(|(id, _)| *id == active)
        .unwrap();
    *row = FlowRequestQueueOrderV4::StableKey(Vec::new());
    assert!(matches!(
        assert_reject_before_codecs(empty_key, &codecs, FlowCheckpointLimits::default()),
        FlowCheckpointError::InvalidState(_)
    ));
    let mut oversized_key = valid.clone();
    let (_, row) = oversized_key
        .request_queue_orders
        .iter_mut()
        .find(|(id, _)| *id == active)
        .unwrap();
    *row = FlowRequestQueueOrderV4::StableKey(vec![b'k'; 129]);
    assert!(matches!(
        assert_reject_before_codecs(oversized_key, &codecs, FlowCheckpointLimits::default()),
        FlowCheckpointError::LimitExceeded("queue order key bytes")
    ));
    let mut live_tag_mismatch = valid.clone();
    let (_, row) = live_tag_mismatch
        .request_queue_orders
        .iter_mut()
        .find(|(id, _)| *id == active)
        .unwrap();
    *row = FlowRequestQueueOrderV4::AdmissionOrder;
    assert!(matches!(
        assert_reject_before_codecs(live_tag_mismatch, &codecs, FlowCheckpointLimits::default()),
        FlowCheckpointError::InvalidState(_)
    ));

    let (removed, removed_codecs, _, removed_requests) = removed_terminal_seed();
    let mut disagreement = removed
        .capture_checkpoint_v4(&removed_codecs, FlowCheckpointLimits::default())
        .unwrap();
    let (_, row) = disagreement
        .request_queue_orders
        .iter_mut()
        .find(|(id, _)| *id == removed_requests[0])
        .unwrap();
    *row = FlowRequestQueueOrderV4::AdmissionOrder;
    assert!(matches!(
        assert_reject_before_codecs(
            disagreement,
            &removed_codecs,
            FlowCheckpointLimits::default()
        ),
        FlowCheckpointError::InvalidState(_)
    ));

    let mut missing_live_resource = valid;
    missing_live_resource
        .resources
        .retain(|resource| *resource != stable);
    missing_live_resource
        .queue_order_modes
        .retain(|(resource, _)| *resource != stable);
    assert!(matches!(
        assert_reject_before_codecs(
            missing_live_resource,
            &codecs,
            FlowCheckpointLimits::default()
        ),
        FlowCheckpointError::InvalidState(_)
    ));
}

#[test]
fn v4_key_row_and_checked_total_limits_preflight_before_decode() {
    let mut flow = FlowRuntime::new();
    let owner = flow.spawn_actor().unwrap();
    let resource = flow
        .create_resource_with_queue_ordering(
            1,
            FlowAllocationMode::Eager,
            FlowQueueOrdering::StableKey,
        )
        .unwrap();
    let request = flow
        .acquire(resource)
        .owner(owner)
        .at(at(10))
        .queue_order_key(key(&[b'k'; 128]))
        .submit()
        .unwrap();
    let empty_codecs = FlowCheckpointCodecs::new();
    let exact = FlowCheckpointLimits {
        max_key_bytes: 128,
        ..FlowCheckpointLimits::default()
    };
    let image = flow.capture_checkpoint_v4(&empty_codecs, exact).unwrap();
    assert!(matches!(
        image.request_queue_orders.iter().find(|(id, _)| *id == request),
        Some((_, FlowRequestQueueOrderV4::StableKey(bytes))) if bytes.len() == 128
    ));
    FlowRuntime::restore_checkpoint_v4(image.clone(), &empty_codecs, exact).unwrap();
    let mut one_over = exact;
    one_over.max_key_bytes = 127;
    assert!(matches!(
        flow.capture_checkpoint_v4(&empty_codecs, one_over),
        Err(FlowCheckpointError::LimitExceeded("key bytes"))
    ));
    assert!(matches!(
        assert_reject_before_codecs(image.clone(), &empty_codecs, one_over),
        FlowCheckpointError::LimitExceeded("key bytes")
    ));

    let no_requests = FlowCheckpointLimits {
        max_requests: 0,
        ..FlowCheckpointLimits::default()
    };
    assert!(matches!(
        flow.capture_checkpoint_v4(&empty_codecs, no_requests),
        Err(FlowCheckpointError::LimitExceeded(
            "queue order request rows"
        ))
    ));
    assert!(matches!(
        assert_reject_before_codecs(image.clone(), &empty_codecs, no_requests),
        FlowCheckpointError::LimitExceeded("queue-order identity rows")
    ));
    let no_resources = FlowCheckpointLimits {
        max_resources: 0,
        ..FlowCheckpointLimits::default()
    };
    assert!(matches!(
        flow.capture_checkpoint_v4(&empty_codecs, no_resources),
        Err(FlowCheckpointError::LimitExceeded("queue order mode rows"))
    ));
    assert!(matches!(
        assert_reject_before_codecs(image, &empty_codecs, no_resources),
        FlowCheckpointError::LimitExceeded("queue-order identity rows")
    ));

    let mut aggregate_flow = FlowRuntime::new();
    let aggregate_owner = aggregate_flow.spawn_actor().unwrap();
    let aggregate_resource = aggregate_flow
        .create_resource_with_queue_ordering(
            1,
            FlowAllocationMode::Eager,
            FlowQueueOrdering::StableKey,
        )
        .unwrap();
    let aggregate_work = aggregate_flow
        .create_work(aggregate_owner, duration(20), "ctx", 61_u64)
        .unwrap();
    aggregate_flow
        .acquire(aggregate_resource)
        .owner(aggregate_owner)
        .timed_work(aggregate_work)
        .queue_order_key(key(&[b'a'; 128]))
        .submit()
        .unwrap();
    aggregate_flow.step().unwrap().unwrap();
    let aggregate_codecs = codecs();
    let aggregate_default = FlowCheckpointLimits::default();
    let baseline_image = aggregate_flow
        .capture_checkpoint_v4(&aggregate_codecs, aggregate_default)
        .unwrap();
    let serialized_string_bytes = baseline_image
        .work_registrations
        .iter()
        .map(|row| {
            row.context_runtime_key.len()
                + row.context_codec_key.len()
                + row.restart_codec_key.as_ref().map_or(0, String::len)
        })
        .sum::<usize>()
        + baseline_image
            .contexts
            .iter()
            .map(|row| row.runtime_key.len() + row.codec_key.len())
            .sum::<usize>()
        + baseline_image
            .context_stores
            .iter()
            .map(|store| store.codec_key.len())
            .sum::<usize>();
    assert_eq!(serialized_string_bytes, 15);

    let capture_registration_key_bytes = "ctx".len() + "ctx".len();
    assert_eq!(capture_registration_key_bytes, 6);
    let mut capture_exact = aggregate_default;
    capture_exact.max_key_bytes = 128 + capture_registration_key_bytes;
    ENCODE_CALLS.with(|calls| calls.set(0));
    let aggregate_image = aggregate_flow
        .capture_checkpoint_v4(&aggregate_codecs, capture_exact)
        .unwrap();
    ENCODE_CALLS.with(|calls| assert!(calls.get() > 0));
    let mut capture_one_over = capture_exact;
    capture_one_over.max_key_bytes -= 1;
    ENCODE_CALLS.with(|calls| calls.set(0));
    assert!(matches!(
        aggregate_flow.capture_checkpoint_v4(&aggregate_codecs, capture_one_over),
        Err(FlowCheckpointError::LimitExceeded("key bytes"))
    ));
    ENCODE_CALLS.with(|calls| assert_eq!(calls.get(), 0));

    let mut restore_exact = aggregate_default;
    restore_exact.max_key_bytes = 128 + serialized_string_bytes;
    FlowRuntime::restore_checkpoint_v4(aggregate_image.clone(), &aggregate_codecs, restore_exact)
        .unwrap();
    let mut restore_one_over = restore_exact;
    restore_one_over.max_key_bytes -= 1;
    assert!(matches!(
        assert_reject_before_codecs(aggregate_image, &aggregate_codecs, restore_one_over),
        FlowCheckpointError::LimitExceeded("key bytes")
    ));

    let mut payload_flow = FlowRuntime::new();
    let payload_owner = payload_flow.spawn_actor().unwrap();
    let payload_resource = payload_flow
        .create_resource_with_queue_ordering(
            1,
            FlowAllocationMode::Eager,
            FlowQueueOrdering::StableKey,
        )
        .unwrap();
    let work = payload_flow
        .create_work(payload_owner, duration(20), "ctx", 55_u64)
        .unwrap();
    payload_flow
        .acquire(payload_resource)
        .owner(payload_owner)
        .timed_work(work)
        .queue_order_key(key(b"payload"))
        .submit()
        .unwrap();
    let payload_codecs = codecs();
    let payload_exact = FlowCheckpointLimits {
        max_payload_bytes: 8,
        ..FlowCheckpointLimits::default()
    };
    ENCODE_CALLS.with(|calls| calls.set(0));
    payload_flow
        .capture_checkpoint_v4(&payload_codecs, payload_exact)
        .unwrap();
    ENCODE_CALLS.with(|calls| assert!(calls.get() > 0));
    let mut payload_short = payload_exact;
    payload_short.max_payload_bytes = 7;
    ENCODE_CALLS.with(|calls| calls.set(0));
    assert!(matches!(
        payload_flow.capture_checkpoint_v4(&payload_codecs, payload_short),
        Err(FlowCheckpointError::Codec { .. })
    ));
    ENCODE_CALLS.with(|calls| assert!(calls.get() > 0));
}

#[test]
fn v4_admission_default_rebuild_preserves_existing_payload() {
    let mut source = FlowRuntime::new();
    let owner = source.spawn_actor().unwrap();
    let resource = source.create_resource(1).unwrap();
    let active = source.acquire(resource).owner(owner).submit().unwrap();
    source.step().unwrap().unwrap();
    let queued = source
        .acquire(resource)
        .owner(owner)
        .can_preempt(true)
        .deadline(at(10))
        .submit()
        .unwrap();
    source.step().unwrap().unwrap();
    assert_eq!(source.request(active).unwrap().state, RequestState::Active);
    assert_eq!(source.request(queued).unwrap().state, RequestState::Queued);
    let codecs = FlowCheckpointCodecs::new();
    let limits = FlowCheckpointLimits::default();
    let before = source.capture_checkpoint_v3(&codecs, limits).unwrap();
    let v4 = source.capture_checkpoint_v4(&codecs, limits).unwrap();
    assert!(v4
        .queue_order_modes
        .iter()
        .all(|(_, mode)| *mode == FlowQueueOrdering::AdmissionOrder));
    assert!(v4
        .request_queue_orders
        .iter()
        .all(|(_, order)| matches!(order, FlowRequestQueueOrderV4::AdmissionOrder)));
    let restored = FlowRuntime::restore_checkpoint_v4(v4, &codecs, limits).unwrap();
    let after = restored.capture_checkpoint_v3(&codecs, limits).unwrap();
    assert_eq!(after, before);
    assert_eq!(after.builtins.queues, before.builtins.queues);
    assert_eq!(after.builtins.preempting, before.builtins.preempting);
    assert_eq!(after.builtins.deadlines, before.builtins.deadlines);
}
