use kairo_ecs_des::{
    CommittedUsageWindow, StagedRequestState, StagedResourceRuntime, StagedRuntimeError,
    StagedUsageSource, STAGED_USAGE_VERSION,
};
use kairo_ecs_types::SimTime;
use std::num::NonZeroU128;

fn tick(value: u128) -> SimTime {
    SimTime::from_ticks(value)
}

fn source() -> StagedUsageSource {
    StagedUsageSource {
        run: "synthetic-resource-fixture".into(),
        scenario: "three-requests".into(),
        branch: "base".into(),
        engine_revision: "fixture-kairos".into(),
        model_revision: "trajectory-v1".into(),
        rng_revision: "deterministic".into(),
        time_unit: "hour".into(),
        ticks_per_time_unit: NonZeroU128::new(2).unwrap(),
    }
}

fn fifo_trajectory() -> CommittedUsageWindow {
    let mut runtime = StagedResourceRuntime::new_with_usage(source()).unwrap();
    let actor = runtime.spawn_actor().unwrap();
    let resource = runtime.create_resource_with_unit(3, "instrument").unwrap();
    assert_eq!(
        runtime.submit_quantity(resource, actor, tick(0), 0),
        Err(StagedRuntimeError::InvalidQuantity)
    );
    assert_eq!(
        runtime.submit_quantity(resource, actor, tick(0), u64::MAX),
        Err(StagedRuntimeError::InvalidQuantity)
    );
    let first = runtime
        .submit_quantity(resource, actor, tick(0), 2)
        .unwrap();
    let second = runtime
        .submit_quantity(resource, actor, tick(1), 2)
        .unwrap();
    let third = runtime.submit(resource, actor, tick(1)).unwrap();
    assert_eq!(runtime.run_for(3).unwrap(), 3);
    let held = runtime.resource(resource).unwrap();
    assert_eq!(held.available, 1);
    assert_eq!(held.queued, [second, third]);
    let lease = runtime.request(first).unwrap().lease.unwrap();
    runtime.release(lease, tick(5)).unwrap();
    assert_eq!(
        runtime.release(lease, tick(5)),
        Err(StagedRuntimeError::InvalidState)
    );
    let early = runtime.committed_usage(resource, tick(0), tick(1)).unwrap();
    assert_eq!(early.occupied_unit_ticks, 2);
    assert_eq!(early.occupancy.len(), 1);
    assert_eq!(early.occupancy[0].release, None);

    assert_eq!(runtime.run_for(1).unwrap(), 1);
    let after = runtime.resource(resource).unwrap();
    assert_eq!(after.available, 0);
    assert!(after.queued.is_empty());
    assert_eq!(
        runtime.request(first).unwrap().state,
        StagedRequestState::Released
    );
    for request in [second, third] {
        let lease = runtime.request(request).unwrap().lease.unwrap();
        runtime.release(lease, tick(7)).unwrap();
    }
    runtime.run_for(1).unwrap();
    assert_eq!(
        runtime.committed_usage(resource, tick(0), tick(7)),
        Err(StagedRuntimeError::InvalidUsageWindow)
    );
    runtime.run_for(1).unwrap();
    assert_eq!(runtime.resource(resource).unwrap().available, 3);
    let all = runtime.committed_usage(resource, tick(0), tick(7)).unwrap();
    assert_eq!(all.version, STAGED_USAGE_VERSION);
    assert_eq!(all.source, source());
    assert_eq!(all.quantity_unit, "instrument");
    assert_eq!(all.occupied_unit_ticks, 16);
    assert_eq!(
        all.occupancy
            .iter()
            .map(|row| (row.start.ticks(), row.end.ticks(), row.quantity))
            .collect::<Vec<_>>(),
        [(0, 5, 2), (5, 7, 2), (5, 7, 1)]
    );
    assert_eq!(
        all.occupancy[0].release.unwrap(),
        all.occupancy[1].admission
    );
    assert_eq!(all.occupancy[1].admission, all.occupancy[2].admission);
    assert_eq!(
        runtime.committed_usage(resource, tick(0), tick(1)).unwrap(),
        early
    );
    assert_eq!(
        runtime.submit(resource, actor, tick(7)),
        Err(StagedRuntimeError::SubmissionInPast)
    );
    all
}

#[test]
fn actual_admission_promotes_fifo_without_partial_or_bypass() {
    fifo_trajectory();
}

#[test]
fn deterministic_dispatch_replay_preserves_usage_identity() {
    assert_eq!(fifo_trajectory(), fifo_trajectory());
}

#[test]
fn active_lease_is_censored_and_window_split_conserves_occupancy() {
    let mut runtime = StagedResourceRuntime::new_with_usage(source()).unwrap();
    let actor = runtime.spawn_actor().unwrap();
    let resource = runtime.create_resource_with_unit(1, "instrument").unwrap();
    let marker_resource = runtime.create_resource(1).unwrap();
    let request = runtime.submit(resource, actor, tick(2)).unwrap();
    runtime.run_for(1).unwrap();
    let lease = runtime.request(request).unwrap().lease.unwrap();
    runtime.release(lease, tick(7)).unwrap();
    runtime.submit(marker_resource, actor, tick(4)).unwrap();
    runtime.run_for(1).unwrap();
    let early = runtime.committed_usage(resource, tick(0), tick(4)).unwrap();
    assert_eq!(early.occupied_unit_ticks, 2);
    assert_eq!(early.occupancy[0].release, None);
    runtime.run_for(1).unwrap();
    let late = runtime.committed_usage(resource, tick(4), tick(7)).unwrap();
    assert_eq!(late.occupancy[0].admission.at, tick(2));
    assert_eq!(late.occupancy[0].release.unwrap().at, tick(7));
    assert_eq!(late.occupied_unit_ticks, 3);
    let whole = runtime.committed_usage(resource, tick(0), tick(7)).unwrap();
    assert_eq!(
        whole.occupied_unit_ticks,
        early.occupied_unit_ticks + late.occupied_unit_ticks
    );
    assert_eq!(
        runtime.committed_usage(resource, tick(0), tick(4)).unwrap(),
        early
    );
}

#[test]
fn window_rejects_arithmetic_overflow_without_wrap_or_partial_output() {
    let mut runtime = StagedResourceRuntime::new_with_usage(source()).unwrap();
    let actor = runtime.spawn_actor().unwrap();
    let resource = runtime
        .create_resource_with_unit(u64::MAX, "instrument")
        .unwrap();
    let request = runtime
        .submit_quantity(resource, actor, tick(0), u64::MAX)
        .unwrap();
    runtime.run_for(1).unwrap();
    let lease = runtime.request(request).unwrap().lease.unwrap();
    runtime.release(lease, tick(u128::MAX)).unwrap();
    runtime.run_for(1).unwrap();
    assert_eq!(
        runtime.committed_usage(resource, tick(0), tick(u128::MAX)),
        Err(StagedRuntimeError::UsageOverflow)
    );
    let tail = runtime
        .committed_usage(resource, tick(u128::MAX - 1), tick(u128::MAX))
        .unwrap();
    assert_eq!(tail.occupied_unit_ticks, u128::from(u64::MAX));
}

#[test]
fn usage_units_are_explicit_and_fixed() {
    let mut runtime = StagedResourceRuntime::new_with_usage(source()).unwrap();
    let actor = runtime.spawn_actor().unwrap();
    assert_eq!(
        runtime.create_resource_with_unit(1, " "),
        Err(StagedRuntimeError::InvalidResourceUnit)
    );
    let unlabelled = runtime.create_resource(1).unwrap();
    runtime.submit(unlabelled, actor, tick(2)).unwrap();
    runtime.run_for(1).unwrap();
    assert_eq!(
        runtime.committed_usage(unlabelled, tick(0), tick(2)),
        Err(StagedRuntimeError::MissingResourceUnit)
    );
}
