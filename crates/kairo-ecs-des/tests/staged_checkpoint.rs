#![cfg(feature = "checkpoint-json")]

use std::num::NonZeroU128;

use kairo_ecs_des::{
    CommittedUsageWindow, StagedActorId, StagedCheckpointError, StagedRequestId,
    StagedRequestState, StagedResourceId, StagedResourceRuntime, StagedRuntimeError,
    StagedUsageSource,
};
use kairo_ecs_types::{EntityId, SimTime};
use serde_json::{json, Value};

fn tick(value: u128) -> SimTime {
    SimTime::from_ticks(value)
}

struct Fixture {
    runtime: StagedResourceRuntime,
    actor: StagedActorId,
    resource: StagedResourceId,
    requests: [StagedRequestId; 5],
}

fn checkpoint_boundary() -> Fixture {
    let mut runtime = StagedResourceRuntime::new_with_usage(StagedUsageSource {
        run: "restart-run".into(),
        scenario: "fifo-contention".into(),
        branch: "main".into(),
        engine_revision: "scheduler-v1".into(),
        model_revision: "allocator-v1".into(),
        rng_revision: "deterministic".into(),
        time_unit: "tick".into(),
        ticks_per_time_unit: NonZeroU128::new(1).unwrap(),
    })
    .unwrap();
    let resource = runtime.create_resource_with_unit(3, "bed").unwrap();
    let actors: [_; 5] = std::array::from_fn(|_| runtime.spawn_actor().unwrap());
    let retired = runtime.submit(resource, actors[0], tick(0)).unwrap();
    assert_eq!(runtime.run_for(1).unwrap(), 1);
    let lease = runtime.request(retired).unwrap().lease.unwrap();
    runtime.release(lease, tick(1)).unwrap();
    let active = runtime
        .submit_quantity(resource, actors[1], tick(2), 2)
        .unwrap();
    let head = runtime
        .submit_quantity(resource, actors[2], tick(3), 2)
        .unwrap();
    let tail = runtime.submit(resource, actors[3], tick(3)).unwrap();
    let withdrawn = runtime.submit(resource, actors[4], tick(9)).unwrap();
    assert_eq!(runtime.run_for(4).unwrap(), 4);
    assert_eq!(
        runtime
            .committed_usage(resource, tick(0), tick(3))
            .unwrap()
            .occupied_unit_ticks,
        3
    );
    runtime
        .release(runtime.request(active).unwrap().lease.unwrap(), tick(10))
        .unwrap();
    assert!(runtime.close_request(active, tick(6)).unwrap());
    assert!(runtime.close_request(withdrawn, tick(5)).unwrap());
    assert!(runtime.close_request(tail, tick(6)).unwrap());
    Fixture {
        runtime,
        actor: actors[1],
        resource,
        requests: [retired, active, head, tail, withdrawn],
    }
}

fn finish(
    runtime: &mut StagedResourceRuntime,
    resource: StagedResourceId,
    requests: [StagedRequestId; 5],
) -> CommittedUsageWindow {
    assert_eq!(runtime.run_for(u64::MAX).unwrap(), 3);
    assert_eq!(runtime.now(), tick(6));
    assert_eq!(runtime.resource(resource).unwrap().available, 1);
    let head_lease = runtime.request(requests[2]).unwrap().lease.unwrap();
    runtime.release(head_lease, tick(8)).unwrap();
    assert_eq!(runtime.run_for(u64::MAX).unwrap(), 1);
    assert_eq!(runtime.resource(resource).unwrap().available, 3);
    for id in requests {
        assert_eq!(
            runtime.request(id).unwrap().state,
            StagedRequestState::Released
        );
    }
    runtime.committed_usage(resource, tick(0), tick(8)).unwrap()
}

#[test]
fn restore_keeps_admissions_pending_actions_and_causal_occupancy() {
    let Fixture {
        mut runtime,
        actor,
        resource,
        requests,
    } = checkpoint_boundary();
    let saved = runtime.checkpoint_json().unwrap();
    let mut restored = StagedResourceRuntime::from_checkpoint_json(&saved).unwrap();
    assert_eq!(restored.checkpoint_json().unwrap(), saved);
    assert_eq!(
        restored.resource(resource),
        Err(StagedRuntimeError::UnknownResource)
    );
    assert_eq!(
        restored.request(requests[1]),
        Err(StagedRuntimeError::UnknownRequest)
    );
    let stale_lease = runtime.request(requests[1]).unwrap().lease.unwrap();
    assert_eq!(
        restored.release(stale_lease, tick(7)),
        Err(StagedRuntimeError::InvalidState)
    );
    assert!(restored.request_by_entity(EntityId::new(1, 1)).is_none());
    let new_actor = restored.actor_by_entity(actor.entity_id()).unwrap();
    let new_resource = restored.resource_by_entity(resource.entity_id()).unwrap();
    let new_requests = requests.map(|id| restored.request_by_entity(id.entity_id()).unwrap());
    assert_ne!(new_actor, actor);
    assert_eq!(
        restored.resource(new_resource).unwrap().queued,
        new_requests[2..4]
    );
    assert_eq!(
        restored.submit(new_resource, new_actor, tick(3)),
        Err(StagedRuntimeError::SubmissionInPast)
    );
    let expected = finish(&mut runtime, resource, requests);
    assert_eq!(finish(&mut restored, new_resource, new_requests), expected);
    assert_eq!(expected.occupied_unit_ticks, 13);
    assert_eq!(
        expected
            .occupancy
            .iter()
            .map(|row| row.request.index)
            .collect::<Vec<_>>(),
        [0, 1, 2]
    );
    assert_eq!(expected.occupancy[1].release.unwrap().event.index, 7);
    assert_eq!(
        expected.occupancy[2].admission,
        expected.occupancy[1].release.unwrap()
    );
    assert_eq!(
        restored.checkpoint_json().unwrap(),
        runtime.checkpoint_json().unwrap()
    );

    assert_eq!(
        restored.spawn_actor().unwrap().entity_id(),
        EntityId::new(5, 0)
    );
    assert_eq!(
        restored.create_resource(1).unwrap().entity_id(),
        EntityId::new(1, 0)
    );
    assert_eq!(
        restored
            .submit(new_resource, new_actor, tick(9))
            .unwrap()
            .entity_id(),
        EntityId::new(5, 0)
    );
}

fn error_for(state: &Value) -> StagedCheckpointError {
    StagedResourceRuntime::from_checkpoint_json(&serde_json::to_vec(state).unwrap())
        .err()
        .unwrap()
}

#[test]
fn restore_rejects_wrong_versions_and_inconsistent_resource_state() {
    let saved = checkpoint_boundary().runtime.checkpoint_json().unwrap();
    let original: Value = serde_json::from_slice(&saved).unwrap();
    let mut wrong_version = original.clone();
    wrong_version["version"] = json!(2);
    assert!(matches!(
        error_for(&wrong_version),
        StagedCheckpointError::UnsupportedVersion(2)
    ));
    let mut bad_scheduler = original.clone();
    bad_scheduler["scheduler_json"] = json!("{}");
    assert!(matches!(
        error_for(&bad_scheduler),
        StagedCheckpointError::Scheduler(_)
    ));
    let mut extra_field = original.clone();
    extra_field["new_field"] = json!(true);
    assert!(matches!(
        error_for(&extra_field),
        StagedCheckpointError::Json(_)
    ));

    let invalid: [(&str, Value, &str); 16] = [
        ("/usage_source/run", json!(" "), "invalid usage provenance"),
        (
            "/actors/1",
            json!(0),
            "actor identities are not in canonical order",
        ),
        (
            "/resources/0/capacity",
            json!(0),
            "resource capacity or usage unit is invalid",
        ),
        (
            "/resources/0/unit",
            json!(" "),
            "resource capacity or usage unit is invalid",
        ),
        (
            "/requests/1/actor",
            json!(99),
            "request actor does not exist",
        ),
        (
            "/requests/1/resource",
            json!(99),
            "request resource does not exist",
        ),
        (
            "/requests/1/quantity",
            json!(4),
            "request quantity exceeds resource bounds",
        ),
        (
            "/requests/1/admission",
            Value::Null,
            "request state and recorded actions disagree",
        ),
        (
            "/requests/0/admission/at_ticks",
            json!(2),
            "release does not follow admission",
        ),
        (
            "/requests/3/pending_close/event",
            json!([7, 7]),
            "pending event belongs to more than one action",
        ),
        (
            "/resources/0/active",
            json!([99]),
            "resource membership references a missing request",
        ),
        (
            "/resources/0/active",
            json!([1, 1]),
            "resource membership and request state disagree",
        ),
        (
            "/resources/0/queued",
            json!([]),
            "active or queued request is missing from its resource",
        ),
        (
            "/requests/2/quantity",
            json!(1),
            "the FIFO head could already have been admitted",
        ),
        (
            "/requests/1/pending_close",
            Value::Null,
            "pending actions and scheduler count disagree",
        ),
        (
            "/requests/1/admission/at_ticks",
            json!(4),
            "occupancy boundary does not refer to a past dispatch",
        ),
    ];
    for (path, value, reason) in invalid {
        let mut candidate = original.clone();
        *candidate.pointer_mut(path).unwrap() = value;
        let error = error_for(&candidate);
        assert!(
            matches!(error, StagedCheckpointError::InvalidState(message) if message == reason),
            "{path}: {error}"
        );
    }

    let mut overbooked = original.clone();
    overbooked["requests"][2]["state"] = json!("active");
    overbooked["requests"][2]["admission"] = json!({"at_ticks": 3, "event": [3, 3]});
    overbooked["resources"][0]["active"] = json!([1, 2]);
    overbooked["resources"][0]["queued"] = json!([3]);
    assert!(matches!(
        error_for(&overbooked),
        StagedCheckpointError::InvalidState("active leases exceed capacity")
    ));
}

#[test]
fn restore_checks_every_pending_scheduler_action() {
    let saved = checkpoint_boundary().runtime.checkpoint_json().unwrap();
    let original: Value = serde_json::from_slice(&saved).unwrap();
    let mut loopback_action = original.clone();
    loopback_action["requests"][1]["pending_close"]["event"] = json!([66, 66]);
    assert!(matches!(
        error_for(&loopback_action),
        StagedCheckpointError::InvalidState("pending scheduler event has no matching action")
    ));

    let inner: Value = serde_json::from_str(original["scheduler_json"].as_str().unwrap()).unwrap();
    for (field, wrong_value) in [
        ("kind", json!(9)),
        ("entity", json!([99, 0])),
        ("priority", json!(1)),
        ("at_ticks", json!(3)),
    ] {
        let mut mismatched = original.clone();
        let mut scheduler = inner.clone();
        scheduler["pending"][0][field] = wrong_value;
        mismatched["scheduler_json"] = json!(scheduler.to_string());
        assert!(
            matches!(
                error_for(&mismatched),
                StagedCheckpointError::InvalidState(
                    "pending action does not match scheduler event or cutoff"
                )
            ),
            "{field}"
        );
    }
    let mut past_due = original;
    let mut scheduler = inner;
    past_due["usage_sealed_through"] = Value::Null;
    scheduler["pending"][0]["at_ticks"] = json!(2);
    past_due["scheduler_json"] = json!(scheduler.to_string());
    assert!(matches!(
        error_for(&past_due),
        StagedCheckpointError::InvalidState(
            "pending action does not match scheduler event or cutoff"
        )
    ));
}

#[test]
fn restore_rejects_inconsistent_occupancy_history() {
    let mut runtime = StagedResourceRuntime::new().unwrap();
    let resource = runtime.create_resource(1).unwrap();
    let actor = runtime.spawn_actor().unwrap();
    let first = runtime.submit(resource, actor, tick(0)).unwrap();
    let second = runtime.submit(resource, actor, tick(0)).unwrap();
    assert_eq!(runtime.run_for(2).unwrap(), 2);
    assert!(runtime.close_request(first, tick(2)).unwrap());
    assert_eq!(runtime.run_for(1).unwrap(), 1);
    assert!(runtime.close_request(second, tick(3)).unwrap());
    assert_eq!(runtime.run_for(1).unwrap(), 1);
    let saved = runtime.checkpoint_json().unwrap();
    let restored = StagedResourceRuntime::from_checkpoint_json(&saved).unwrap();
    assert_eq!(restored.checkpoint_json().unwrap(), saved);
    let state: Value = serde_json::from_slice(&saved).unwrap();

    let mut split_dispatch = state.clone();
    split_dispatch["requests"][0]["release"]["at_ticks"] = json!(3);
    assert!(matches!(
        error_for(&split_dispatch),
        StagedCheckpointError::InvalidState("occupancy boundaries for a dispatch disagree")
    ));

    let mut overlapping_leases = state.clone();
    overlapping_leases["requests"][1]["admission"] = json!({"at_ticks": 1, "event": [1, 1]});
    assert!(matches!(
        error_for(&overlapping_leases),
        StagedCheckpointError::InvalidState("historical occupancy exceeds capacity")
    ));

    let mut too_many_boundaries = state;
    let mut inner: Value =
        serde_json::from_str(too_many_boundaries["scheduler_json"].as_str().unwrap()).unwrap();
    inner["dispatched_events"] = json!(2);
    inner["cancelled_events"] = json!(2);
    too_many_boundaries["scheduler_json"] = json!(inner.to_string());
    assert!(matches!(
        error_for(&too_many_boundaries),
        StagedCheckpointError::InvalidState("occupancy references too many dispatches")
    ));
}
