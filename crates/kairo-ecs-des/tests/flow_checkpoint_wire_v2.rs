use kairo_ecs_des::{
    FlowCheckpointCodecError, FlowCheckpointCodecs, FlowCheckpointImage, FlowCheckpointLimits,
    FlowCheckpointRebindV1, FlowCheckpointV2, FlowCheckpointWireLimits, FlowCommandV2, FlowRuntime,
    LifecycleTransition, PreemptionStrategy, RequestState,
};
use kairo_ecs_types::SimDuration;
use kairo_ecs_types::SimTime;

#[test]
fn routed_gated_image_roundtrips_as_v2_and_continues() {
    let mut source = FlowRuntime::new();
    let resource = source.create_resource(1).unwrap();
    source
        .schedule_resource_grant_enabled(resource, false, SimTime::from_ticks(0), 0)
        .unwrap();
    source.run_for(1).unwrap();
    source
        .schedule_resource_grant_enabled(resource, true, SimTime::from_ticks(8), 0)
        .unwrap();
    let limits = FlowCheckpointLimits::default();
    let image = source
        .capture_checkpoint_image(&Default::default(), limits)
        .unwrap();
    let bytes = image
        .encode_wire(FlowCheckpointWireLimits::default())
        .unwrap();
    let decoded =
        FlowCheckpointImage::decode_wire(&bytes, FlowCheckpointWireLimits::default()).unwrap();
    assert_eq!(decoded, image);
    let FlowCheckpointImage::V2(v2) = decoded else {
        panic!("gate checkpoint must route to V2");
    };
    let mut restored = FlowRuntime::restore_checkpoint_v2(v2, &Default::default(), limits).unwrap();
    assert_eq!(restored.run_for(8).unwrap(), source.run_for(8).unwrap());
}

#[test]
fn enabled_future_gate_routes_to_v2_and_survives_wire_restore() {
    let mut source = FlowRuntime::new();
    let resource = source.create_resource(1).unwrap();
    source
        .schedule_resource_grant_enabled(resource, true, SimTime::from_ticks(8), 0)
        .unwrap();
    let limits = FlowCheckpointLimits::default();
    let image = source
        .capture_checkpoint_image(&Default::default(), limits)
        .unwrap();
    assert!(matches!(
        image.as_ref(),
        kairo_ecs_des::FlowCheckpointRef::V2(_)
    ));
    let bytes = image
        .encode_wire(FlowCheckpointWireLimits::default())
        .unwrap();
    let decoded =
        FlowCheckpointImage::decode_wire(&bytes, FlowCheckpointWireLimits::default()).unwrap();
    let (mut restored, _) = decoded
        .restore_with_rebind(&Default::default(), limits)
        .unwrap();
    assert_eq!(source.run_for(9).unwrap(), restored.run_for(9).unwrap());
}

#[test]
fn ungated_routing_preserves_v1_wire_bytes() {
    let mut source = FlowRuntime::new();
    source.create_resource(1).unwrap();
    let limits = FlowCheckpointLimits::default();
    let image = source
        .capture_checkpoint_image(&Default::default(), limits)
        .unwrap();
    let FlowCheckpointImage::V1(v1) = image.clone() else {
        panic!("ungated checkpoint must route to V1");
    };
    assert_eq!(
        image
            .encode_wire(FlowCheckpointWireLimits::default())
            .unwrap(),
        v1.encode_wire_v1(FlowCheckpointWireLimits::default())
            .unwrap(),
    );
    assert_eq!(
        FlowCheckpointImage::decode_wire(
            &image
                .encode_wire(FlowCheckpointWireLimits::default())
                .unwrap(),
            FlowCheckpointWireLimits::default()
        )
        .unwrap(),
        image
    );
}

fn encode_u64(value: &u64, remaining: usize) -> Result<Vec<u8>, FlowCheckpointCodecError> {
    if remaining < 8 {
        return Err(FlowCheckpointCodecError("payload budget".into()));
    }
    Ok(value.to_le_bytes().to_vec())
}
fn decode_u64(bytes: &[u8], _: &FlowCheckpointRebindV1) -> Result<u64, FlowCheckpointCodecError> {
    let value: [u8; 8] = bytes
        .try_into()
        .map_err(|_| FlowCheckpointCodecError("u64 payload".into()))?;
    Ok(u64::from_le_bytes(value))
}
fn unit_codecs() -> FlowCheckpointCodecs {
    let mut codecs = FlowCheckpointCodecs::new();
    codecs
        .register_context(
            "unit",
            1,
            |_: &(), _: usize| Ok(Vec::new()),
            |bytes: &[u8], _: &FlowCheckpointRebindV1| {
                if bytes.is_empty() {
                    Ok(())
                } else {
                    Err(FlowCheckpointCodecError("empty".into()))
                }
            },
        )
        .unwrap();
    codecs
}
fn unit_v2(image: FlowCheckpointImage) -> kairo_ecs_des::FlowCheckpointV2 {
    let FlowCheckpointImage::V2(image) = image else {
        panic!("grant gate should route to V2")
    };
    image
}

#[test]
fn completion_first_started_frontier_roundtrips_through_wire() {
    let mut source = FlowRuntime::new();
    let resource = source.create_resource(1).unwrap();
    let owner = source.spawn_actor().unwrap();
    let first_work = source
        .create_work(owner, SimDuration::from_ticks(3), "unit", ())
        .unwrap();
    let second_work = source
        .create_work(owner, SimDuration::from_ticks(4), "unit", ())
        .unwrap();
    let first = source
        .acquire(resource)
        .owner(owner)
        .timed_work(first_work)
        .submit()
        .unwrap();
    let second = source
        .acquire(resource)
        .owner(owner)
        .timed_work(second_work)
        .at(SimTime::from_ticks(1))
        .submit()
        .unwrap();
    let disable = source
        .schedule_resource_grant_enabled(resource, false, SimTime::from_ticks(3), i32::MAX)
        .unwrap();
    source
        .schedule_resource_grant_enabled(resource, true, SimTime::from_ticks(5), i32::MAX)
        .unwrap();
    source.run_for(2).unwrap();
    let completion = source.step().unwrap().unwrap();
    assert_eq!(completion.at, SimTime::from_ticks(3));
    assert_ne!(completion.event, disable);
    assert!(completion
        .records
        .iter()
        .any(|r| r.request == first && r.transition == LifecycleTransition::Completed));
    let codecs = unit_codecs();
    let limits = FlowCheckpointLimits::default();
    let image = source.capture_checkpoint_image(&codecs, limits).unwrap();
    let v2 = unit_v2(image);
    assert_eq!(v2.started_frontier, Some(SimTime::from_ticks(3)));
    assert!(v2
        .commands
        .iter()
        .any(|(id, c)| *id == disable && *c == FlowCommandV2::GrantEnabled(resource, false)));
    let bytes = v2
        .encode_wire_v2(FlowCheckpointWireLimits::default())
        .unwrap();
    let decoded =
        FlowCheckpointV2::decode_wire_v2(&bytes, FlowCheckpointWireLimits::default()).unwrap();
    let mut restored = FlowRuntime::restore_checkpoint_v2(decoded, &codecs, limits).unwrap();
    let expected = source.run_for(20).unwrap();
    let actual = restored.run_for(20).unwrap();
    assert_eq!(actual, expected);
    let records: Vec<_> = actual.dispatches.iter().flat_map(|d| &d.records).collect();
    assert!(!records.iter().any(|r| r.request == second
        && r.transition == LifecycleTransition::Granted
        && r.at == SimTime::from_ticks(3)));
    assert!(records.iter().any(|r| r.request == second
        && r.transition == LifecycleTransition::Granted
        && r.at == SimTime::from_ticks(5)));
}

#[test]
fn suspended_work_and_pending_enable_roundtrip_through_wire() {
    let mut source = FlowRuntime::new();
    let resource = source.create_resource(1).unwrap();
    let low_owner = source.spawn_actor().unwrap();
    let urgent_owner = source.spawn_actor().unwrap();
    let low_work = source
        .create_work(low_owner, SimDuration::from_ticks(10), "unit", ())
        .unwrap();
    let urgent_work = source
        .create_work(urgent_owner, SimDuration::from_ticks(2), "unit", ())
        .unwrap();
    let low = source
        .acquire(resource)
        .owner(low_owner)
        .timed_work(low_work)
        .priority(10)
        .preemptible(PreemptionStrategy::Suspend)
        .submit()
        .unwrap();
    let urgent = source
        .acquire(resource)
        .owner(urgent_owner)
        .timed_work(urgent_work)
        .at(SimTime::from_ticks(4))
        .priority(1)
        .can_preempt(true)
        .submit()
        .unwrap();
    let enable = source
        .schedule_resource_grant_enabled(resource, true, SimTime::from_ticks(8), 0)
        .unwrap();
    let disable = source
        .schedule_resource_grant_enabled(resource, false, SimTime::from_ticks(6), i32::MAX)
        .unwrap();
    let dispatch = (0..32)
        .find_map(|_| {
            let d = source.step().unwrap()?;
            (d.event == disable).then_some(d)
        })
        .unwrap();
    assert_eq!(dispatch.at, SimTime::from_ticks(6));
    assert_eq!(source.resource_grant_enabled(resource), Ok(false));
    assert_eq!(source.request(low).unwrap().state, RequestState::Suspended);
    assert_eq!(
        source.request(urgent).unwrap().state,
        RequestState::Completed
    );
    assert_eq!(
        source.work_progress(low_work).unwrap().useful_elapsed,
        SimDuration::from_ticks(4)
    );
    assert_eq!(
        source.work_progress(low_work).unwrap().remaining,
        SimDuration::from_ticks(6)
    );
    let codecs = unit_codecs();
    let limits = FlowCheckpointLimits::default();
    let image = unit_v2(source.capture_checkpoint_image(&codecs, limits).unwrap());
    assert_eq!(image.started_frontier, Some(SimTime::from_ticks(6)));
    assert!(image
        .commands
        .iter()
        .any(|(id, c)| *id == enable && *c == FlowCommandV2::GrantEnabled(resource, true)));
    let bytes = image
        .encode_wire_v2(FlowCheckpointWireLimits::default())
        .unwrap();
    let decoded =
        FlowCheckpointV2::decode_wire_v2(&bytes, FlowCheckpointWireLimits::default()).unwrap();
    let mut restored = FlowRuntime::restore_checkpoint_v2(decoded, &codecs, limits).unwrap();
    let expected = source.run_for(20).unwrap();
    let actual = restored.run_for(20).unwrap();
    assert_eq!(actual, expected);
    assert_eq!(restored.request(low).unwrap(), source.request(low).unwrap());
    assert_eq!(
        restored.request(urgent).unwrap(),
        source.request(urgent).unwrap()
    );
    assert_eq!(
        restored.work_progress(low_work).unwrap(),
        source.work_progress(low_work).unwrap()
    );
    let records: Vec<_> = actual.dispatches.iter().flat_map(|d| &d.records).collect();
    assert!(!records.iter().any(|r| r.request == low
        && r.transition == LifecycleTransition::Resumed
        && r.at == SimTime::from_ticks(6)));
    assert!(records.iter().any(|r| r.request == low
        && r.transition == LifecycleTransition::Resumed
        && r.at == SimTime::from_ticks(8)));
    assert!(records.iter().any(|r| r.request == low
        && r.transition == LifecycleTransition::Completed
        && r.at == SimTime::from_ticks(14)));
}

#[test]
fn precommit_budget_halt_roundtrips_wire_without_consuming_pending_gate() {
    let config = kairo_ecs_des::FlowConfig {
        max_same_tick_flow_transitions: std::num::NonZeroU64::new(1).unwrap(),
    };
    let mut source = FlowRuntime::with_config(config);
    let owner = source.spawn_actor().unwrap();
    let resource = source.create_resource(1).unwrap();
    let work = source
        .create_work(owner, SimDuration::from_ticks(10), "unit", ())
        .unwrap();
    source
        .acquire(resource)
        .owner(owner)
        .timed_work(work)
        .submit()
        .unwrap();
    let gate = source
        .schedule_resource_grant_enabled(resource, false, SimTime::from_ticks(8), 0)
        .unwrap();
    assert!(matches!(
        source.step(),
        Err(kairo_ecs_des::FlowError::SameTickBudgetExceeded { at_ticks: 0, .. })
    ));
    assert_eq!(source.budget_snapshot().tick, None);
    assert_eq!(source.budget_snapshot().consumed, 0);
    let codecs = unit_codecs();
    let limits = FlowCheckpointLimits::default();
    let image = unit_v2(source.capture_checkpoint_image(&codecs, limits).unwrap());
    assert_eq!(image.started_frontier, None);
    assert!(image
        .commands
        .iter()
        .any(|(id, c)| *id == gate && *c == FlowCommandV2::GrantEnabled(resource, false)));
    let bytes = image
        .encode_wire_v2(FlowCheckpointWireLimits::default())
        .unwrap();
    let decoded =
        FlowCheckpointV2::decode_wire_v2(&bytes, FlowCheckpointWireLimits::default()).unwrap();
    let mut restored = FlowRuntime::restore_checkpoint_v2(decoded, &codecs, limits).unwrap();
    assert_eq!(restored.budget_snapshot(), source.budget_snapshot());
    assert_eq!(
        restored.capture_checkpoint_v2(&codecs, limits).unwrap(),
        image
    );
    assert_eq!(
        restored
            .capture_checkpoint_v2(&codecs, limits)
            .unwrap()
            .encode_wire_v2(FlowCheckpointWireLimits::default())
            .unwrap(),
        bytes
    );
    let source_repeat = source.step().unwrap_err();
    let restored_repeat = restored.step().unwrap_err();
    assert_eq!(source_repeat, restored_repeat);
    assert!(matches!(
        source_repeat,
        kairo_ecs_des::FlowError::SameTickBudgetExceeded { at_ticks: 0, .. }
    ));
    assert_eq!(
        source.capture_checkpoint_v2(&codecs, limits).unwrap(),
        restored.capture_checkpoint_v2(&codecs, limits).unwrap()
    );
}

#[test]
fn routed_v1_nonempty_context_bytes_and_continuation_match_legacy() {
    let mut source = FlowRuntime::new();
    let resource = source.create_resource(1).unwrap();
    let owner = source.spawn_actor().unwrap();
    let work = source
        .create_work(owner, SimDuration::from_ticks(4), "bytes", 0x1122_u64)
        .unwrap();
    let request = source
        .acquire(resource)
        .owner(owner)
        .timed_work(work)
        .submit()
        .unwrap();
    source.step().unwrap().unwrap();
    let mut codecs = FlowCheckpointCodecs::new();
    codecs
        .register_context("bytes", 1, encode_u64, decode_u64)
        .unwrap();
    let limits = FlowCheckpointLimits::default();
    let legacy = source.capture_checkpoint(&codecs, limits).unwrap();
    assert!(legacy
        .context_stores
        .iter()
        .flat_map(|s| &s.rows)
        .any(|(_, b)| !b.is_empty()));
    let routed = source.capture_checkpoint_image(&codecs, limits).unwrap();
    assert!(matches!(routed, FlowCheckpointImage::V1(_)));
    let legacy_bytes = legacy
        .encode_wire_v1(FlowCheckpointWireLimits::default())
        .unwrap();
    assert_eq!(
        routed
            .encode_wire(FlowCheckpointWireLimits::default())
            .unwrap(),
        legacy_bytes
    );
    let decoded =
        FlowCheckpointImage::decode_wire(&legacy_bytes, FlowCheckpointWireLimits::default())
            .unwrap();
    let (mut restored, _) = decoded.restore_with_rebind(&codecs, limits).unwrap();
    assert_eq!(restored.run_for(4).unwrap(), source.run_for(4).unwrap());
    assert_eq!(
        restored.request(request).unwrap(),
        source.request(request).unwrap()
    );
}
