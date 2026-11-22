#![cfg(feature = "optimistic-usage")]

use kairo_ecs_des::{
    committed_staged_usage, StagedRequestId, StagedRequestState, StagedResourceCheckpoint,
    StagedResourceId, StagedResourceRuntime, StagedRuntimeError, StagedUsageSource,
};
use kairo_ecs_pdes::{
    LpId, OptimisticLimits, OptimisticProcess, OptimisticRuntime, OptimisticStateError,
    PartitionPlan, RemoteEvent, Tick,
};
use kairo_ecs_types::{EntityId, SimDuration};
use std::collections::BTreeMap;
use std::num::NonZeroU128;

const LP: LpId = LpId(0);

fn tick(value: u128) -> Tick {
    Tick::from_ticks(value)
}

fn allocator() -> StagedResourceRuntime {
    StagedResourceRuntime::new_with_usage(StagedUsageSource {
        run: "replay-fixture".into(),
        scenario: "late-release".into(),
        branch: "base".into(),
        engine_revision: "fixture".into(),
        model_revision: "fixture".into(),
        rng_revision: "deterministic".into(),
        time_unit: "minute".into(),
        ticks_per_time_unit: NonZeroU128::MIN,
    })
    .unwrap()
}

#[derive(Clone)]
struct Snapshot {
    allocation: StagedResourceCheckpoint,
    handled: u64,
}

impl AsRef<StagedResourceCheckpoint> for Snapshot {
    fn as_ref(&self) -> &StagedResourceCheckpoint {
        &self.allocation
    }
}

struct Model {
    allocation: StagedResourceRuntime,
    request: StagedRequestId,
    handled: u64,
}

impl OptimisticProcess for Model {
    type Snapshot = Snapshot;

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            allocation: self.allocation.checkpoint(),
            handled: self.handled,
        }
    }

    fn restore(&mut self, state: &Snapshot) -> Result<(), OptimisticStateError> {
        self.allocation
            .restore(&state.allocation)
            .map_err(|error| OptimisticStateError::new(format!("{error:?}")))?;
        self.handled = state.handled;
        Ok(())
    }

    fn on_event(&mut self, event: &RemoteEvent) -> Vec<RemoteEvent> {
        self.handled += 1;
        if event.event_payload == b"release" {
            self.allocation
                .close_request(self.request, event.tick)
                .unwrap();
        }
        while self
            .allocation
            .next_event_time()
            .is_some_and(|at| at <= event.tick)
        {
            assert_eq!(self.allocation.run_for(1), Ok(1));
        }
        self.allocation.seal_usage_through(event.tick).unwrap();
        Vec::new()
    }
}

fn runtime() -> (OptimisticRuntime<Model>, StagedResourceId) {
    let mut allocation = allocator();
    let actor = allocation.spawn_actor().unwrap();
    let resource = allocation.create_resource_with_unit(1, "porter").unwrap();
    let request = allocation.submit(resource, actor, tick(2)).unwrap();
    let runtime = OptimisticRuntime::new(
        PartitionPlan::from_entities(1, SimDuration::from_ticks(1), vec![actor.entity_id()])
            .unwrap(),
        BTreeMap::from([(LP, Vec::new())]),
        BTreeMap::from([(
            LP,
            Model {
                allocation,
                request,
                handled: 0,
            },
        )]),
        OptimisticLimits::default(),
    )
    .unwrap();
    (runtime, resource)
}

fn event(at: u128, payload: &[u8]) -> RemoteEvent {
    RemoteEvent {
        source_lp: LP,
        dest_lp: LP,
        tick: tick(at),
        event_payload: payload.to_vec(),
    }
}

fn schedule_original(runtime: &mut OptimisticRuntime<Model>) {
    runtime.schedule_initial(1, event(2, b"admit")).unwrap();
    runtime.schedule_initial(2, event(4, b"observe")).unwrap();
    runtime.schedule_initial(3, event(8, b"observe")).unwrap();
    runtime.schedule_initial(4, event(10, b"release")).unwrap();
}

#[test]
fn rollback_exports_committed_occupancy_once_with_deterministic_causal_boundaries() {
    let (mut replay, resource) = runtime();
    let (mut serial, serial_resource) = runtime();
    schedule_original(&mut replay);
    schedule_original(&mut serial);
    let straggler = serial.schedule_initial(5, event(6, b"release")).unwrap();
    replay.run_until_with_budget(tick(10), 10).unwrap();
    let live = replay.process_at(LP).unwrap();
    assert_eq!(
        live.allocation.request(live.request).unwrap().state,
        StagedRequestState::Released
    );
    let initial = replay.committed_snapshot(LP).unwrap();
    assert_eq!(
        committed_staged_usage(&initial, resource, tick(0), tick(4)),
        Err(StagedRuntimeError::InvalidUsageWindow)
    );

    replay.fossil_collect(tick(5)).unwrap();
    let early = replay.committed_snapshot(LP).unwrap();
    let early_window = committed_staged_usage(&early, resource, tick(0), tick(4)).unwrap();
    assert_eq!(early.state().handled, 2);
    assert_eq!(early_window.window.occupied_unit_ticks, 2);
    assert_eq!(early_window.window.occupancy[0].release, None);

    replay.receive(straggler).unwrap();
    replay.run_until_with_budget(tick(10), 10).unwrap();
    assert!(replay.report().rolled_back_events >= 2);
    replay.fossil_collect(tick(9)).unwrap();
    let committed = replay.committed_snapshot(LP).unwrap();
    assert_eq!(committed.state().handled, 4);
    let output = committed_staged_usage(&committed, resource, tick(0), tick(8)).unwrap();
    assert_eq!(output.lp_id, LP);
    assert_eq!(output.exclusive_cutoff, tick(9));
    assert_eq!(output.window.occupied_unit_ticks, 4);
    assert_eq!(output.window.occupancy.len(), 1);
    let row = &output.window.occupancy[0];
    assert_eq!(row.actor, EntityId::new(0, 0));
    assert_eq!((row.start, row.end, row.quantity), (tick(2), tick(6), 1));
    assert_eq!(row.admission.at, tick(2));
    assert_eq!(row.release.unwrap().at, tick(6));
    assert_eq!(
        committed_staged_usage(&committed, serial_resource, tick(0), tick(8)),
        Err(StagedRuntimeError::UnknownResource)
    );
    for (start, end) in [(0, 9), (0, 10), (8, 8)] {
        assert_eq!(
            committed_staged_usage(&committed, resource, tick(start), tick(end)),
            Err(StagedRuntimeError::InvalidUsageWindow)
        );
    }

    serial.run_until_with_budget(tick(10), 10).unwrap();
    serial.fossil_collect(tick(9)).unwrap();
    let expected = committed_staged_usage(
        &serial.committed_snapshot(LP).unwrap(),
        serial_resource,
        tick(0),
        tick(8),
    )
    .unwrap();
    assert_eq!(output, expected);
    replay.fossil_collect(tick(11)).unwrap();
    let later = replay.committed_snapshot(LP).unwrap();
    assert_eq!(
        committed_staged_usage(&later, resource, tick(0), tick(8))
            .unwrap()
            .window,
        output.window
    );
    assert_eq!(
        committed_staged_usage(&later, resource, tick(0), tick(4))
            .unwrap()
            .window,
        early_window.window
    );
}

#[test]
fn an_allocator_seal_requires_dispatch_and_prevents_retroactive_actions() {
    let (mut runtime, resource) = runtime();
    runtime.fossil_collect(tick(9)).unwrap();
    let unsealed = runtime.committed_snapshot(LP).unwrap();
    assert_eq!(
        committed_staged_usage(&unsealed, resource, tick(0), tick(8)),
        Err(StagedRuntimeError::InvalidUsageWindow)
    );

    let mut allocation = allocator();
    let resource = allocation.create_resource_with_unit(1, "porter").unwrap();
    let actor = allocation.spawn_actor().unwrap();
    let request = allocation.submit(resource, actor, tick(2)).unwrap();
    assert_eq!(
        allocation.seal_usage_through(tick(2)),
        Err(StagedRuntimeError::InvalidUsageWindow)
    );
    assert_eq!(allocation.run_for(1), Ok(1));
    let checkpoint = allocation.checkpoint();
    allocation.seal_usage_through(tick(8)).unwrap();
    assert_eq!(
        allocation.close_request(request, tick(8)),
        Err(StagedRuntimeError::InvalidReleaseTime)
    );
    assert_eq!(
        allocation.submit(resource, actor, tick(8)),
        Err(StagedRuntimeError::SubmissionInPast)
    );
    assert_eq!(
        allocator().restore(&checkpoint),
        Err(StagedRuntimeError::InvalidState)
    );
    allocation.restore(&checkpoint).unwrap();
    assert_eq!(allocation.close_request(request, tick(8)), Ok(true));
    let closing = allocation.checkpoint();
    assert_eq!(allocation.run_for(1), Ok(1));
    assert_eq!(allocation.resource(resource).unwrap().available, 1);
    allocation.restore(&closing).unwrap();
    assert_eq!(allocation.resource(resource).unwrap().available, 0);
    assert_eq!(allocation.close_request(request, tick(8)), Ok(false));
    assert_eq!(allocation.next_event_time(), Some(tick(8)));
    assert_eq!(allocation.run_for(2), Ok(1));
    assert_eq!(allocation.resource(resource).unwrap().available, 1);
}
