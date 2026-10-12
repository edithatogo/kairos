//! Actual Flow dispatch adapter for isolated C3 probes.
//!
//! The model owns domain mapping and complete C2 capture/restore. This layer
//! always dispatches through Flow; model callbacks may enqueue but not execute
//! additional events. The existing read-only checkpoint view supplies bounded
//! next-event inspection without introducing a public scheduler API.
use crate::shadow::{LedgerSnapshot, ProbeAdapter, ProbeInput, ProbeStep, ShadowError};
use kairo_ecs_des::{FlowCheckpointCodecs, FlowCheckpointLimits, FlowDispatch, FlowRuntime};

pub(crate) trait FlowProbeModel {
    type World;
    fn start(
        &self,
        snapshot: &LedgerSnapshot,
        input: &ProbeInput,
    ) -> Result<Self::World, ShadowError>;
    fn flow<'a>(&self, world: &'a Self::World) -> &'a FlowRuntime;
    fn flow_mut<'a>(&self, world: &'a mut Self::World) -> &'a mut FlowRuntime;
    fn codecs<'a>(&self, world: &'a Self::World) -> &'a FlowCheckpointCodecs;
    fn capture_limits(&self) -> FlowCheckpointLimits;
    fn completed(&self, world: &Self::World) -> Result<bool, ShadowError>;
    /// Consume the actual dispatch receipt to update model-owned C2 bridges.
    fn after_dispatch(
        &self,
        world: &mut Self::World,
        dispatch: &FlowDispatch,
    ) -> Result<(), ShadowError>;
    fn checkpoint(&self, world: &Self::World, max_bytes: usize) -> Result<Vec<u8>, ShadowError>;
    fn restore(
        &self,
        snapshot: &LedgerSnapshot,
        input: &ProbeInput,
        bytes: &[u8],
    ) -> Result<Self::World, ShadowError>;
}

pub(crate) struct NativeFlowAdapter<M> {
    pub model: M,
    pub max_image_bytes: usize,
}

impl<M: FlowProbeModel> ProbeAdapter for NativeFlowAdapter<M> {
    type Runtime = M::World;

    fn start(
        &self,
        snapshot: &LedgerSnapshot,
        input: &ProbeInput,
    ) -> Result<Self::Runtime, ShadowError> {
        let world = self.model.start(snapshot, input)?;
        if self.model.flow(&world).now().ticks() != snapshot.at {
            return Err(ShadowError::Contract(
                "native start clock differs from anchor",
            ));
        }
        Ok(world)
    }

    fn now(&self, runtime: &Self::Runtime) -> u128 {
        self.model.flow(runtime).now().ticks()
    }

    fn next_tick(&self, runtime: &Self::Runtime) -> Result<Option<u128>, ShadowError> {
        let image = self
            .model
            .flow(runtime)
            .capture_checkpoint_image(self.model.codecs(runtime), self.model.capture_limits())
            .map_err(|error| ShadowError::Adapter(format!("native preview: {error:?}")))?;
        Ok(image
            .as_ref()
            .scheduler()
            .entries
            .iter()
            .filter(|entry| entry.live)
            .map(|entry| entry.request.at.ticks())
            .min())
    }

    fn step(&self, runtime: &mut Self::Runtime) -> Result<ProbeStep, ShadowError> {
        let before = self
            .model
            .flow(runtime)
            .budget_snapshot()
            .scheduler
            .dispatched_events;
        let dispatch = self
            .model
            .flow_mut(runtime)
            .step()
            .map_err(|error| ShadowError::Adapter(format!("native dispatch: {error:?}")))?
            .ok_or(ShadowError::Contract("native step on empty queue"))?;
        let after_native_step = self
            .model
            .flow(runtime)
            .budget_snapshot()
            .scheduler
            .dispatched_events;
        let dispatches = after_native_step.saturating_sub(before);
        let dispatched_at = self.now(runtime);
        if let Some(error) = &dispatch.error {
            return Ok(ProbeStep {
                dispatched_at,
                target: None,
                dispatches: dispatches.max(1),
                failure: Some(ShadowError::Adapter(format!(
                    "native rejected dispatch: {error:?}"
                ))),
            });
        }
        if let Err(error) = self.model.after_dispatch(runtime, &dispatch) {
            let flow = self.model.flow(runtime);
            let total = flow.budget_snapshot().scheduler.dispatched_events;
            return Ok(ProbeStep {
                dispatched_at: flow.now().ticks(),
                target: None,
                dispatches: total.saturating_sub(before).max(dispatches).max(1),
                failure: Some(error),
            });
        }
        let flow = self.model.flow(runtime);
        let total = flow.budget_snapshot().scheduler.dispatched_events;
        let consumed = total.saturating_sub(before).max(dispatches).max(1);
        if consumed != 1 {
            return Ok(ProbeStep {
                dispatched_at: flow.now().ticks(),
                target: None,
                dispatches: consumed,
                failure: Some(ShadowError::Contract(
                    "model hook executed an extra native event",
                )),
            });
        }
        if flow.now() != dispatch.at || total != before.saturating_add(1) {
            return Ok(ProbeStep {
                dispatched_at: flow.now().ticks(),
                target: None,
                dispatches: consumed,
                failure: Some(ShadowError::Contract(
                    "native dispatch receipt differs from runtime",
                )),
            });
        }
        let target = match self.model.completed(runtime) {
            Ok(true) => Some(dispatch.at.ticks()),
            Ok(false) => None,
            Err(error) => {
                return Ok(ProbeStep {
                    dispatched_at: flow.now().ticks(),
                    target: None,
                    dispatches: consumed,
                    failure: Some(error),
                })
            }
        };
        Ok(ProbeStep {
            dispatched_at: flow.now().ticks(),
            target,
            dispatches: consumed,
            failure: None,
        })
    }

    fn target_at_start(&self, runtime: &Self::Runtime) -> Result<Option<u128>, ShadowError> {
        Ok(self.model.completed(runtime)?.then_some(self.now(runtime)))
    }

    fn checkpoint(
        &self,
        runtime: &Self::Runtime,
        max_bytes: usize,
    ) -> Result<Vec<u8>, ShadowError> {
        let cap = max_bytes.min(self.max_image_bytes);
        let bytes = self.model.checkpoint(runtime, cap)?;
        if bytes.len() > cap {
            return Err(ShadowError::LimitExceeded);
        }
        Ok(bytes)
    }

    fn restore(
        &self,
        snapshot: &LedgerSnapshot,
        input: &ProbeInput,
        bytes: &[u8],
    ) -> Result<Self::Runtime, ShadowError> {
        if bytes.len() > self.max_image_bytes {
            return Err(ShadowError::LimitExceeded);
        }
        let world = self.model.restore(snapshot, input, bytes)?;
        if self.now(&world) < snapshot.at {
            return Err(ShadowError::IncompatibleCheckpoint);
        }
        Ok(world)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::seed_map::{CalibrationSeedMap, SeedPurpose};
    use crate::shadow_pool::{PoolLimits, ProbePool};
    use kairo_ecs_des::{
        FlowAllocationMode, FlowCheckpointCodecError, FlowCheckpointImage, FlowCheckpointLimits,
        FlowCheckpointRebindV1, FlowCommandV3, FlowHandlerCodeIds, FlowQueueOrderKey,
        FlowQueueOrdering, FlowRequestQueueOrderV4, PreemptionStrategy, RequestId, RequestState,
        ResourceId, WorkHandlers, WorkId, WorkState,
    };
    use kairo_ecs_types::{SimDuration, SimTime};
    use std::collections::BTreeMap;

    struct Model {
        illicit_extra_step: bool,
        fail_after_dispatch: bool,
    }
    struct World {
        flow: FlowRuntime,
        codecs: FlowCheckpointCodecs,
        work: WorkId,
    }
    fn enc(value: &u32, cap: usize) -> Result<Vec<u8>, FlowCheckpointCodecError> {
        if cap < 4 {
            return Err(FlowCheckpointCodecError("cap".into()));
        }
        Ok(value.to_le_bytes().to_vec())
    }
    fn dec(bytes: &[u8], _: &FlowCheckpointRebindV1) -> Result<u32, FlowCheckpointCodecError> {
        Ok(u32::from_le_bytes(
            bytes
                .try_into()
                .map_err(|_| FlowCheckpointCodecError("u32".into()))?,
        ))
    }
    impl FlowProbeModel for Model {
        type World = World;
        fn start(&self, _: &LedgerSnapshot, _: &ProbeInput) -> Result<World, ShadowError> {
            let mut flow = FlowRuntime::new();
            flow.register_work_handlers("c3.native", WorkHandlers::<u32>::default())
                .unwrap();
            let owner = flow.spawn_actor().unwrap();
            let resource = flow.create_resource(1).unwrap();
            let work = flow
                .create_work(owner, SimDuration::from_ticks(5), "c3.native", 7u32)
                .unwrap();
            flow.acquire(resource)
                .owner(owner)
                .at(SimTime::from_ticks(0))
                .timed_work(work)
                .submit()
                .unwrap();
            let mut codecs = FlowCheckpointCodecs::new();
            codecs
                .register_context::<u32>("c3.native", 1, enc, dec)
                .unwrap();
            codecs
                .register_work_handlers::<u32>(
                    "c3.native",
                    FlowHandlerCodeIds::default(),
                    WorkHandlers::default(),
                )
                .unwrap();
            Ok(World { flow, codecs, work })
        }
        fn flow<'a>(&self, w: &'a World) -> &'a FlowRuntime {
            &w.flow
        }
        fn flow_mut<'a>(&self, w: &'a mut World) -> &'a mut FlowRuntime {
            &mut w.flow
        }
        fn codecs<'a>(&self, w: &'a World) -> &'a FlowCheckpointCodecs {
            &w.codecs
        }
        fn capture_limits(&self) -> FlowCheckpointLimits {
            FlowCheckpointLimits::default()
        }
        fn completed(&self, w: &World) -> Result<bool, ShadowError> {
            Ok(w.flow.work_progress(w.work).unwrap().state == WorkState::Completed)
        }
        fn after_dispatch(&self, w: &mut World, _: &FlowDispatch) -> Result<(), ShadowError> {
            if self.illicit_extra_step {
                w.flow.step().unwrap();
            }
            if self.fail_after_dispatch {
                return Err(ShadowError::Adapter("hook failed after dispatch".into()));
            }
            Ok(())
        }
        fn checkpoint(&self, _: &World, _: usize) -> Result<Vec<u8>, ShadowError> {
            Err(ShadowError::Adapter(
                "fixture does not implement capture".into(),
            ))
        }
        fn restore(
            &self,
            _: &LedgerSnapshot,
            _: &ProbeInput,
            _: &[u8],
        ) -> Result<World, ShadowError> {
            Err(ShadowError::Adapter(
                "fixture does not implement restore".into(),
            ))
        }
    }

    struct GatedWorld {
        flow: FlowRuntime,
        codecs: FlowCheckpointCodecs,
        resource: ResourceId,
        request: RequestId,
        work: WorkId,
    }

    struct GatedModel {
        enable_at: Option<u128>,
    }

    impl FlowProbeModel for GatedModel {
        type World = GatedWorld;

        fn start(&self, _: &LedgerSnapshot, _: &ProbeInput) -> Result<GatedWorld, ShadowError> {
            let mut flow = FlowRuntime::new();
            flow.register_work_handlers("c3.native", WorkHandlers::<u32>::default())
                .unwrap();
            let owner = flow.spawn_actor().unwrap();
            let resource = flow.create_resource(1).unwrap();
            let work = flow
                .create_work(owner, SimDuration::from_ticks(5), "c3.native", 7u32)
                .unwrap();
            flow.schedule_resource_grant_enabled(resource, false, SimTime::from_ticks(0), 0)
                .unwrap();
            let request = flow
                .acquire(resource)
                .owner(owner)
                .timed_work(work)
                .at(SimTime::from_ticks(0))
                .submit()
                .unwrap();
            if let Some(enable_at) = self.enable_at {
                flow.schedule_resource_grant_enabled(
                    resource,
                    true,
                    SimTime::from_ticks(enable_at),
                    0,
                )
                .unwrap();
            }
            let mut codecs = FlowCheckpointCodecs::new();
            codecs
                .register_context::<u32>("c3.native", 1, enc, dec)
                .unwrap();
            codecs
                .register_work_handlers::<u32>(
                    "c3.native",
                    FlowHandlerCodeIds::default(),
                    WorkHandlers::default(),
                )
                .unwrap();
            Ok(GatedWorld {
                flow,
                codecs,
                resource,
                request,
                work,
            })
        }

        fn flow<'a>(&self, world: &'a GatedWorld) -> &'a FlowRuntime {
            &world.flow
        }

        fn flow_mut<'a>(&self, world: &'a mut GatedWorld) -> &'a mut FlowRuntime {
            &mut world.flow
        }

        fn codecs<'a>(&self, world: &'a GatedWorld) -> &'a FlowCheckpointCodecs {
            &world.codecs
        }

        fn capture_limits(&self) -> FlowCheckpointLimits {
            FlowCheckpointLimits::default()
        }

        fn completed(&self, world: &GatedWorld) -> Result<bool, ShadowError> {
            Ok(world.flow.work_progress(world.work).unwrap().state == WorkState::Completed)
        }

        fn after_dispatch(&self, _: &mut GatedWorld, _: &FlowDispatch) -> Result<(), ShadowError> {
            Ok(())
        }

        fn checkpoint(&self, _: &GatedWorld, _: usize) -> Result<Vec<u8>, ShadowError> {
            Err(ShadowError::Adapter(
                "gated fixture does not implement capture".into(),
            ))
        }

        fn restore(
            &self,
            _: &LedgerSnapshot,
            _: &ProbeInput,
            _: &[u8],
        ) -> Result<GatedWorld, ShadowError> {
            Err(ShadowError::Adapter(
                "gated fixture does not implement restore".into(),
            ))
        }
    }

    struct V3PreviewWorld {
        flow: FlowRuntime,
        codecs: FlowCheckpointCodecs,
        resource: ResourceId,
        request: RequestId,
        work: WorkId,
        last_dispatch: Option<FlowDispatch>,
    }

    struct V3PreviewModel {
        limits: FlowCheckpointLimits,
    }

    fn v3_preview_codecs() -> FlowCheckpointCodecs {
        let mut codecs = FlowCheckpointCodecs::new();
        codecs
            .register_context::<u32>("preview.v3", 1, enc, dec)
            .unwrap();
        codecs
            .register_work_handlers::<u32>(
                "preview.v3",
                FlowHandlerCodeIds::default(),
                WorkHandlers::default(),
            )
            .unwrap();
        codecs
    }

    fn count_preview_completion(
        context: &mut u32,
        _: &kairo_ecs_des::FlowCallbackSnapshot,
        _: &mut kairo_ecs_des::FlowCommandSink,
    ) {
        *context += 1;
    }

    fn v3_preview_callback_codecs() -> FlowCheckpointCodecs {
        let mut codecs = v3_preview_codecs();
        codecs
            .register_work_continuations(
                "preview.v3",
                kairo_ecs_des::FlowContinuationCodeIds {
                    on_complete: Some(kairo_ecs_des::FlowCallbackCodeV1 {
                        stable_id: "preview.v3.completion-counter".into(),
                        version: 1,
                    }),
                    ..Default::default()
                },
                kairo_ecs_des::FlowContinuations {
                    on_complete: Some(count_preview_completion),
                    ..Default::default()
                },
            )
            .unwrap();
        codecs
    }

    fn v3_preview_world(
        duration: u128,
        transition_limit: Option<std::num::NonZeroU64>,
    ) -> V3PreviewWorld {
        let mut flow = FlowRuntime::with_config(kairo_ecs_des::FlowConfig {
            max_same_tick_flow_transitions: transition_limit
                .unwrap_or_else(|| std::num::NonZeroU64::new(100_000).unwrap()),
        });
        flow.register_work_handlers("preview.v3", WorkHandlers::<u32>::default())
            .unwrap();
        let resource = flow
            .create_resource_with_allocation_mode(1, FlowAllocationMode::Deferred)
            .unwrap();
        let owner = flow.spawn_actor().unwrap();
        let work = flow
            .create_work(owner, SimDuration::from_ticks(duration), "preview.v3", 7u32)
            .unwrap();
        let request = flow
            .acquire(resource)
            .owner(owner)
            .timed_work(work)
            .at(SimTime::from_ticks(0))
            .submit()
            .unwrap();
        V3PreviewWorld {
            flow,
            codecs: v3_preview_codecs(),
            resource,
            request,
            work,
            last_dispatch: None,
        }
    }

    fn v3_preview_adapter(limits: FlowCheckpointLimits) -> NativeFlowAdapter<V3PreviewModel> {
        NativeFlowAdapter {
            model: V3PreviewModel { limits },
            max_image_bytes: 4096,
        }
    }

    fn v4_preview_codecs() -> FlowCheckpointCodecs {
        let mut codecs = FlowCheckpointCodecs::new();
        codecs
            .register_context::<u32>("preview.v4", 1, enc, dec)
            .unwrap();
        codecs
            .register_work_handlers::<u32>(
                "preview.v4",
                FlowHandlerCodeIds::default(),
                WorkHandlers::default(),
            )
            .unwrap();
        codecs
    }

    fn count_v4_preview_completion(
        context: &mut u32,
        _: &kairo_ecs_des::FlowCallbackSnapshot,
        _: &mut kairo_ecs_des::FlowCommandSink,
    ) {
        *context += 1;
    }

    fn v4_preview_callback_codecs() -> FlowCheckpointCodecs {
        let mut codecs = v4_preview_codecs();
        codecs
            .register_work_continuations(
                "preview.v4",
                kairo_ecs_des::FlowContinuationCodeIds {
                    on_complete: Some(kairo_ecs_des::FlowCallbackCodeV1 {
                        stable_id: "preview.v4.completion-counter".into(),
                        version: 1,
                    }),
                    ..Default::default()
                },
                kairo_ecs_des::FlowContinuations {
                    on_complete: Some(count_v4_preview_completion),
                    ..Default::default()
                },
            )
            .unwrap();
        codecs
    }

    fn v4_preview_world(
        duration: u128,
        mode: FlowAllocationMode,
        transition_limit: Option<std::num::NonZeroU64>,
        order_key: &[u8],
    ) -> V3PreviewWorld {
        let mut flow = FlowRuntime::with_config(kairo_ecs_des::FlowConfig {
            max_same_tick_flow_transitions: transition_limit
                .unwrap_or_else(|| std::num::NonZeroU64::new(100_000).unwrap()),
        });
        flow.register_work_handlers("preview.v4", WorkHandlers::<u32>::default())
            .unwrap();
        let resource = flow
            .create_resource_with_queue_ordering(1, mode, FlowQueueOrdering::StableKey)
            .unwrap();
        let owner = flow.spawn_actor().unwrap();
        let work = flow
            .create_work(owner, SimDuration::from_ticks(duration), "preview.v4", 7u32)
            .unwrap();
        let request = flow
            .acquire(resource)
            .owner(owner)
            .timed_work(work)
            .at(SimTime::from_ticks(0))
            .queue_order_key(FlowQueueOrderKey::new(order_key).unwrap())
            .submit()
            .unwrap();
        V3PreviewWorld {
            flow,
            codecs: v4_preview_codecs(),
            resource,
            request,
            work,
            last_dispatch: None,
        }
    }

    #[derive(Debug, Eq, PartialEq)]
    struct V4PreviewObservation {
        image: FlowCheckpointImage,
        now: SimTime,
        budget: kairo_ecs_des::FlowBudgetSnapshot,
        resource: kairo_ecs_des::ResourceSnapshot,
        grant_enabled: bool,
        request: kairo_ecs_des::ResourceRequest,
        progress: kairo_ecs_des::WorkProgress,
        context: u32,
    }

    fn v4_preview_observation(world: &V3PreviewWorld) -> V4PreviewObservation {
        V4PreviewObservation {
            image: world
                .flow
                .capture_checkpoint_image(&world.codecs, FlowCheckpointLimits::default())
                .unwrap(),
            now: world.flow.now(),
            budget: world.flow.budget_snapshot(),
            resource: world.flow.resource(world.resource).unwrap(),
            grant_enabled: world.flow.resource_grant_enabled(world.resource).unwrap(),
            request: world.flow.request(world.request).unwrap(),
            progress: world.flow.work_progress(world.work).unwrap(),
            context: *world.flow.work_context::<u32>(world.work).unwrap(),
        }
    }

    fn assert_v4_key(image: &FlowCheckpointImage, request: RequestId, expected: &[u8]) {
        let FlowCheckpointImage::V4(image) = image else {
            panic!("StableKey native preview must select V4");
        };
        assert!(image
            .queue_order_modes
            .iter()
            .any(|(_, mode)| *mode == FlowQueueOrdering::StableKey));
        assert!(image.request_queue_orders.iter().any(|(id, order)| {
            *id == request
                && matches!(order, FlowRequestQueueOrderV4::StableKey(bytes) if bytes == expected)
        }));
    }

    fn stale_v4_preview_world() -> (V3PreviewWorld, WorkId) {
        let mut flow = FlowRuntime::new();
        flow.register_work_handlers("preview.v4", WorkHandlers::<u32>::default())
            .unwrap();
        flow.register_work_continuations(
            "preview.v4",
            kairo_ecs_des::FlowContinuations {
                on_complete: Some(count_v4_preview_completion),
                ..Default::default()
            },
        )
        .unwrap();
        let resource = flow
            .create_resource_with_queue_ordering(
                1,
                FlowAllocationMode::Deferred,
                FlowQueueOrdering::StableKey,
            )
            .unwrap();
        let low_owner = flow.spawn_actor().unwrap();
        let low_work = flow
            .create_work(low_owner, SimDuration::from_ticks(10), "preview.v4", 11u32)
            .unwrap();
        let low = flow
            .acquire(resource)
            .owner(low_owner)
            .timed_work(low_work)
            .at(SimTime::from_ticks(1))
            .priority(10)
            .preemptible(PreemptionStrategy::Suspend)
            .queue_order_key(FlowQueueOrderKey::new(b"low").unwrap())
            .submit()
            .unwrap();
        let first_flush = flow
            .schedule_allocation_flush(resource, SimTime::from_ticks(1), i32::MAX)
            .unwrap();
        drain_shadow_through(&mut flow, first_flush, SimTime::from_ticks(1));
        assert_eq!(flow.request(low).unwrap().state, RequestState::Active);

        let urgent_owner = flow.spawn_actor().unwrap();
        let urgent_work = flow
            .create_work(urgent_owner, SimDuration::from_ticks(2), "preview.v4", 0u32)
            .unwrap();
        let urgent = flow
            .acquire(resource)
            .owner(urgent_owner)
            .timed_work(urgent_work)
            .at(SimTime::from_ticks(4))
            .priority(1)
            .can_preempt(true)
            .queue_order_key(FlowQueueOrderKey::new(b"urgent").unwrap())
            .submit()
            .unwrap();
        let preempt_flush = flow
            .schedule_allocation_flush(resource, SimTime::from_ticks(4), i32::MAX)
            .unwrap();
        drain_shadow_through(&mut flow, preempt_flush, SimTime::from_ticks(4));
        assert_eq!(flow.request(low).unwrap().state, RequestState::Suspended);
        assert_eq!(flow.request(urgent).unwrap().state, RequestState::Active);

        let resume_flush = flow
            .schedule_allocation_flush(resource, SimTime::from_ticks(7), i32::MAX)
            .unwrap();
        let completion = flow.step().unwrap().expect("urgent completion at six");
        assert_eq!(completion.at, SimTime::from_ticks(6));
        assert!(completion.records.iter().any(|record| {
            record.request == urgent
                && record.transition == kairo_ecs_des::LifecycleTransition::Completed
        }));
        let notification = flow.step().unwrap().expect("urgent callback notification");
        assert_eq!(notification.at, SimTime::from_ticks(6));
        assert_eq!(flow.work_context::<u32>(urgent_work), Ok(&1));
        drain_shadow_through(&mut flow, resume_flush, SimTime::from_ticks(7));
        assert_eq!(flow.request(low).unwrap().state, RequestState::Active);
        (
            V3PreviewWorld {
                flow,
                codecs: v4_preview_callback_codecs(),
                resource,
                request: low,
                work: low_work,
                last_dispatch: None,
            },
            urgent_work,
        )
    }

    impl FlowProbeModel for V3PreviewModel {
        type World = V3PreviewWorld;

        fn start(&self, _: &LedgerSnapshot, _: &ProbeInput) -> Result<Self::World, ShadowError> {
            Err(ShadowError::Contract(
                "V3 preview test constructs its own world",
            ))
        }

        fn flow<'a>(&self, world: &'a Self::World) -> &'a FlowRuntime {
            &world.flow
        }

        fn flow_mut<'a>(&self, world: &'a mut Self::World) -> &'a mut FlowRuntime {
            &mut world.flow
        }

        fn codecs<'a>(&self, world: &'a Self::World) -> &'a FlowCheckpointCodecs {
            &world.codecs
        }

        fn capture_limits(&self) -> FlowCheckpointLimits {
            self.limits
        }

        fn completed(&self, world: &Self::World) -> Result<bool, ShadowError> {
            Ok(world.flow.work_progress(world.work).unwrap().state == WorkState::Completed)
        }

        fn after_dispatch(
            &self,
            world: &mut Self::World,
            dispatch: &FlowDispatch,
        ) -> Result<(), ShadowError> {
            world.last_dispatch = Some(dispatch.clone());
            Ok(())
        }

        fn checkpoint(&self, _: &Self::World, _: usize) -> Result<Vec<u8>, ShadowError> {
            Err(ShadowError::Contract("unused in V3 preview fixture"))
        }

        fn restore(
            &self,
            _: &LedgerSnapshot,
            _: &ProbeInput,
            _: &[u8],
        ) -> Result<Self::World, ShadowError> {
            Err(ShadowError::Contract("unused in V3 preview fixture"))
        }
    }

    fn drain_shadow_through(
        flow: &mut FlowRuntime,
        event: kairo_ecs_types::EventId,
        tick: SimTime,
    ) {
        for _ in 0..16 {
            let dispatch = flow.step().unwrap().expect("scheduled event closure");
            assert_eq!(dispatch.at, tick);
            if dispatch.event == event {
                return;
            }
        }
        panic!("event closure exceeded bound");
    }

    #[test]
    fn preview_v4_deferred_pending_flush_shadow() {
        let adapter = v3_preview_adapter(FlowCheckpointLimits::default());
        let mut world = v4_preview_world(5, FlowAllocationMode::Deferred, None, b"flush");
        let submit = world.flow.step().unwrap().expect("Submit at zero");
        assert_eq!(submit.at, SimTime::from_ticks(0));
        assert_eq!(
            world.flow.request(world.request).unwrap().state,
            RequestState::Queued
        );
        assert!(world.flow.request(world.request).unwrap().lease.is_none());
        let flush = world
            .flow
            .schedule_allocation_flush(world.resource, SimTime::from_ticks(4), i32::MAX)
            .unwrap();
        let before = v4_preview_observation(&world);
        assert_v4_key(&before.image, world.request, b"flush");
        assert_eq!(adapter.next_tick(&world).unwrap(), Some(4));
        assert_eq!(v4_preview_observation(&world), before);
        let step = adapter.step(&mut world).unwrap();
        assert_eq!(world.last_dispatch.as_ref().unwrap().event, flush);
        assert_eq!(step.dispatched_at, 4);
        assert_eq!(step.target, None);
        assert_eq!(
            world.flow.request(world.request).unwrap().state,
            RequestState::Active
        );
        let after_grant = v4_preview_observation(&world);
        assert_v4_key(&after_grant.image, world.request, b"flush");
        assert_eq!(adapter.next_tick(&world).unwrap(), Some(9));
        assert_eq!(v4_preview_observation(&world), after_grant);
        let completion = adapter.step(&mut world).unwrap();
        assert_eq!(completion.dispatched_at, 9);
        assert_eq!(completion.target, Some(9));
        assert_eq!(
            world.flow.request(world.request).unwrap().state,
            RequestState::Completed
        );
        assert_v4_key(
            &v4_preview_observation(&world).image,
            world.request,
            b"flush",
        );
    }

    #[test]
    fn preview_v4_disabled_pending_submit_shadow() {
        let adapter = v3_preview_adapter(FlowCheckpointLimits::default());
        let mut flow = FlowRuntime::new();
        flow.register_work_handlers("preview.v4", WorkHandlers::<u32>::default())
            .unwrap();
        let resource = flow
            .create_resource_with_queue_ordering(
                1,
                FlowAllocationMode::Deferred,
                FlowQueueOrdering::StableKey,
            )
            .unwrap();
        let mut requests = Vec::new();
        let mut works = Vec::new();
        for key in [b"z".as_slice(), b"a".as_slice()] {
            let owner = flow.spawn_actor().unwrap();
            let work = flow
                .create_work(owner, SimDuration::from_ticks(5), "preview.v4", 0u32)
                .unwrap();
            let request = flow
                .acquire(resource)
                .owner(owner)
                .timed_work(work)
                .at(SimTime::from_ticks(0))
                .queue_order_key(FlowQueueOrderKey::new(key).unwrap())
                .submit()
                .unwrap();
            requests.push(request);
            works.push(work);
        }
        let disable = flow
            .schedule_resource_grant_enabled(resource, false, SimTime::from_ticks(0), i32::MIN)
            .unwrap();
        let flush = flow
            .schedule_allocation_flush(resource, SimTime::from_ticks(0), i32::MAX)
            .unwrap();
        let world = |flow, request, work| V3PreviewWorld {
            flow,
            codecs: v4_preview_codecs(),
            resource,
            request,
            work,
            last_dispatch: None,
        };
        let mut world = world(flow, requests[0], works[0]);
        assert_eq!(
            world.flow.request(requests[0]).unwrap().state,
            RequestState::Pending
        );
        assert_eq!(
            world.flow.request(requests[1]).unwrap().state,
            RequestState::Pending
        );
        let pending = v4_preview_observation(&world);
        assert_v4_key(&pending.image, requests[0], b"z");
        assert_v4_key(&pending.image, requests[1], b"a");
        assert_eq!(adapter.next_tick(&world).unwrap(), Some(0));
        assert_eq!(v4_preview_observation(&world), pending);
        let disabled = adapter.step(&mut world).unwrap();
        assert_eq!(world.last_dispatch.as_ref().unwrap().event, disable);
        assert_eq!(disabled.dispatched_at, 0);
        assert!(!world.flow.resource_grant_enabled(resource).unwrap());
        for _ in 0..2 {
            let submitted = adapter.step(&mut world).unwrap();
            assert_eq!(submitted.dispatched_at, 0);
        }
        assert_eq!(
            world.flow.request(requests[0]).unwrap().state,
            RequestState::Queued
        );
        assert_eq!(
            world.flow.request(requests[1]).unwrap().state,
            RequestState::Queued
        );
        let closed = adapter.step(&mut world).unwrap();
        assert_eq!(world.last_dispatch.as_ref().unwrap().event, flush);
        assert_eq!(closed.dispatched_at, 0);
        assert!(world.flow.request(requests[0]).unwrap().lease.is_none());
        assert!(world.flow.request(requests[1]).unwrap().lease.is_none());
        let closed_image = v4_preview_observation(&world);
        assert_v4_key(&closed_image.image, requests[0], b"z");
        assert_v4_key(&closed_image.image, requests[1], b"a");
        assert_eq!(adapter.next_tick(&world).unwrap(), None);
        assert_eq!(v4_preview_observation(&world), closed_image);
        let enable = world
            .flow
            .schedule_resource_grant_enabled(resource, true, SimTime::from_ticks(1), i32::MIN)
            .unwrap();
        let grant_flush = world
            .flow
            .schedule_allocation_flush(resource, SimTime::from_ticks(1), i32::MAX)
            .unwrap();
        assert_eq!(adapter.step(&mut world).unwrap().dispatched_at, 1);
        assert_eq!(world.last_dispatch.as_ref().unwrap().event, enable);
        assert_eq!(adapter.step(&mut world).unwrap().dispatched_at, 1);
        assert_eq!(world.last_dispatch.as_ref().unwrap().event, grant_flush);
        assert_eq!(
            world.flow.request(requests[1]).unwrap().state,
            RequestState::Active
        );
        assert_eq!(
            world.flow.request(requests[0]).unwrap().state,
            RequestState::Queued
        );
        assert_eq!(world.flow.request(requests[0]).unwrap().lease, None);
    }

    #[test]
    fn preview_v4_repeated_halt_shadow() {
        let adapter = v3_preview_adapter(FlowCheckpointLimits::default());
        let limit = std::num::NonZeroU64::new(1);
        let mut world = v4_preview_world(5, FlowAllocationMode::Deferred, limit, b"halt");
        let _submit = world.flow.step().unwrap().expect("Submit at zero");
        let flush = world
            .flow
            .schedule_allocation_flush(world.resource, SimTime::from_ticks(0), i32::MAX)
            .unwrap();
        let before = v4_preview_observation(&world);
        assert_v4_key(&before.image, world.request, b"halt");
        assert_eq!(adapter.next_tick(&world).unwrap(), Some(0));
        assert_eq!(v4_preview_observation(&world), before);
        let expected = ShadowError::Adapter(
            "native dispatch: SameTickBudgetExceeded { at_ticks: 0, limit: 1 }".into(),
        );
        assert_eq!(adapter.step(&mut world).unwrap_err(), expected);
        let halted = v4_preview_observation(&world);
        assert!(
            matches!(&halted.image, FlowCheckpointImage::V4(image) if image.budget_halt.is_some())
        );
        assert_v4_key(&halted.image, world.request, b"halt");
        assert_eq!(
            world.flow.peek_next().expect("live scheduler event").id,
            flush
        );
        assert_eq!(adapter.next_tick(&world).unwrap(), Some(0));
        assert_eq!(v4_preview_observation(&world), halted);
        assert_eq!(adapter.step(&mut world).unwrap_err(), expected);
        assert_eq!(v4_preview_observation(&world), halted);
    }

    #[test]
    fn preview_v4_stale_completion_and_limits_shadow() {
        let adapter = v3_preview_adapter(FlowCheckpointLimits::default());
        let (mut world, urgent_work) = stale_v4_preview_world();
        assert_eq!(world.flow.work_context::<u32>(world.work), Ok(&11));
        assert_eq!(world.flow.work_context::<u32>(urgent_work), Ok(&1));
        let before = v4_preview_observation(&world);
        assert_v4_key(&before.image, world.request, b"low");
        let stale = match &before.image {
            FlowCheckpointImage::V4(image) => image
                .commands
                .iter()
                .find_map(|(id, command)| {
                    matches!(command, FlowCommandV3::Completion(request, _, _, at)
                    if *request == world.request && *at == SimTime::from_ticks(11))
                    .then_some(*id)
                })
                .expect("live stale completion command at eleven"),
            _ => panic!("keyed runtime must capture V4"),
        };
        let actual = world.flow.peek_next().expect("live scheduler event");
        assert_eq!(actual.id, stale);
        assert_eq!(actual.at, SimTime::from_ticks(11));
        assert_eq!(adapter.next_tick(&world).unwrap(), Some(11));
        assert_eq!(v4_preview_observation(&world), before);
        let no_op = adapter.step(&mut world).unwrap();
        assert_eq!(world.last_dispatch.as_ref().unwrap().event, stale);
        assert_eq!(no_op.dispatched_at, 11);
        assert!(world.last_dispatch.as_ref().unwrap().records.is_empty());
        assert_eq!(world.flow.work_context::<u32>(world.work), Ok(&11));
        assert_eq!(world.flow.work_context::<u32>(urgent_work), Ok(&1));
        assert_eq!(
            world.flow.work_progress(world.work).unwrap().useful_elapsed,
            SimDuration::from_ticks(7)
        );
        assert_eq!(
            world.flow.work_progress(world.work).unwrap().remaining,
            SimDuration::from_ticks(3)
        );
        assert_eq!(adapter.next_tick(&world).unwrap(), Some(14));
        let useful = adapter.step(&mut world).unwrap();
        assert_eq!(useful.dispatched_at, 14);
        assert_eq!(useful.target, Some(14));

        let limited = v4_preview_world(5, FlowAllocationMode::Deferred, None, b"limit");
        let baseline = v4_preview_observation(&limited);
        assert_v4_key(&baseline.image, limited.request, b"limit");
        let limits = FlowCheckpointLimits {
            max_commands: 0,
            ..FlowCheckpointLimits::default()
        };
        let limit_adapter = v3_preview_adapter(limits);
        let expected = limited
            .flow
            .capture_checkpoint_image(&limited.codecs, limits)
            .unwrap_err();
        assert_eq!(
            limit_adapter.next_tick(&limited).unwrap_err(),
            ShadowError::Adapter(format!("native preview: {expected:?}"))
        );
        assert_eq!(v4_preview_observation(&limited), baseline);
    }

    fn stale_shadow_preview_world() -> (V3PreviewWorld, WorkId) {
        let mut flow = FlowRuntime::new();
        flow.register_work_handlers("preview.v3", WorkHandlers::<u32>::default())
            .unwrap();
        flow.register_work_continuations(
            "preview.v3",
            kairo_ecs_des::FlowContinuations {
                on_complete: Some(count_preview_completion),
                ..Default::default()
            },
        )
        .unwrap();
        let resource = flow
            .create_resource_with_allocation_mode(1, FlowAllocationMode::Deferred)
            .unwrap();
        let low_owner = flow.spawn_actor().unwrap();
        let low_work = flow
            .create_work(low_owner, SimDuration::from_ticks(10), "preview.v3", 11u32)
            .unwrap();
        let request = flow
            .acquire(resource)
            .owner(low_owner)
            .timed_work(low_work)
            .at(SimTime::from_ticks(1))
            .priority(10)
            .preemptible(PreemptionStrategy::Suspend)
            .submit()
            .unwrap();
        let flush_one = flow
            .schedule_allocation_flush(resource, SimTime::from_ticks(1), i32::MAX)
            .unwrap();
        drain_shadow_through(&mut flow, flush_one, SimTime::from_ticks(1));
        assert_eq!(flow.request(request).unwrap().state, RequestState::Active);

        let urgent_owner = flow.spawn_actor().unwrap();
        let urgent_work = flow
            .create_work(urgent_owner, SimDuration::from_ticks(2), "preview.v3", 0u32)
            .unwrap();
        let urgent = flow
            .acquire(resource)
            .owner(urgent_owner)
            .timed_work(urgent_work)
            .at(SimTime::from_ticks(4))
            .priority(1)
            .can_preempt(true)
            .submit()
            .unwrap();
        let flush_four = flow
            .schedule_allocation_flush(resource, SimTime::from_ticks(4), i32::MAX)
            .unwrap();
        drain_shadow_through(&mut flow, flush_four, SimTime::from_ticks(4));
        assert_eq!(
            flow.request(request).unwrap().state,
            RequestState::Suspended
        );
        assert_eq!(flow.request(urgent).unwrap().state, RequestState::Active);

        let flush_seven = flow
            .schedule_allocation_flush(resource, SimTime::from_ticks(7), i32::MAX)
            .unwrap();
        let completion = flow.step().unwrap().expect("urgent completion at six");
        assert_eq!(completion.at, SimTime::from_ticks(6));
        assert!(completion.records.iter().any(|record| {
            record.request == urgent
                && record.transition == kairo_ecs_des::LifecycleTransition::Completed
        }));
        let notification = flow.step().unwrap().expect("urgent completion callback");
        assert_eq!(notification.at, SimTime::from_ticks(6));
        assert_eq!(flow.work_context::<u32>(urgent_work), Ok(&1));
        drain_shadow_through(&mut flow, flush_seven, SimTime::from_ticks(7));
        assert_eq!(flow.request(request).unwrap().state, RequestState::Active);
        (
            V3PreviewWorld {
                flow,
                codecs: v3_preview_callback_codecs(),
                resource,
                request,
                work: low_work,
                last_dispatch: None,
            },
            urgent_work,
        )
    }

    fn inputs() -> (LedgerSnapshot, ProbeInput) {
        let snapshot = LedgerSnapshot {
            frontier: 1,
            at: 0,
            anchor_event: "a".into(),
            digest: [0; 32],
            visible_events: vec![crate::shadow::ObservedEvent {
                order: crate::trace_order::TraceOrderKeyV1 {
                    relative_ticks: 0,
                    case_key: "case".into(),
                    occurrence: 0,
                    event_kind_rank: crate::trace_order::EventKindRank::from_canonical_decimal("1")
                        .unwrap(),
                    source_event_key: "a".into(),
                    source_order: 0,
                },
                available_at: Some(0),
                source_defined: true,
                transition: crate::shadow::Transition::None,
                payload: Vec::new(),
            }],
            resources: BTreeMap::new(),
            resource_feasible: true,
            assumptions: Vec::new(),
        };
        let key = CalibrationSeedMap::new(1, "study", 7)
            .unwrap()
            .key_for("schedule", 0, "case", "task", SeedPurpose::Service)
            .unwrap();
        (
            snapshot,
            ProbeInput {
                target: "service_end".into(),
                seed_key: key,
                parameter_hash: [1; 32],
                adapter_hash: [2; 32],
                fidelity: "Macro".into(),
            },
        )
    }
    #[test]
    fn preview_v3_deferred_pending_flush_without_advancement() {
        let adapter = v3_preview_adapter(FlowCheckpointLimits::default());
        let mut world = v3_preview_world(5, None);
        let submit = world.flow.step().unwrap().expect("Submit at zero");
        assert_eq!(submit.at, SimTime::from_ticks(0));
        assert_eq!(
            world.flow.request(world.request).unwrap().state,
            RequestState::Queued
        );
        assert!(world.flow.request(world.request).unwrap().lease.is_none());
        let flush = world
            .flow
            .schedule_allocation_flush(world.resource, SimTime::from_ticks(4), i32::MAX)
            .unwrap();
        let before = world
            .flow
            .capture_checkpoint_image(&world.codecs, FlowCheckpointLimits::default())
            .unwrap();
        let budget = world.flow.budget_snapshot();
        assert!(matches!(before, FlowCheckpointImage::V3(_)));
        assert_eq!(adapter.next_tick(&world).unwrap(), Some(4));
        assert_eq!(
            world
                .flow
                .capture_checkpoint_image(&world.codecs, FlowCheckpointLimits::default())
                .unwrap(),
            before
        );
        assert_eq!(world.flow.budget_snapshot(), budget);
        assert_eq!(world.flow.now(), SimTime::from_ticks(0));

        let flushed = adapter.step(&mut world).unwrap();
        assert_eq!(flushed.dispatched_at, 4);
        assert_eq!(flushed.dispatches, 1);
        assert_eq!(flushed.failure, None);
        assert_eq!(world.last_dispatch.as_ref().unwrap().event, flush);
        assert!(world
            .last_dispatch
            .as_ref()
            .unwrap()
            .records
            .iter()
            .any(|record| {
                record.request == world.request
                    && record.transition == kairo_ecs_des::LifecycleTransition::Granted
            }));
        let before = world
            .flow
            .capture_checkpoint_image(&world.codecs, FlowCheckpointLimits::default())
            .unwrap();
        let budget = world.flow.budget_snapshot();
        assert_eq!(adapter.next_tick(&world).unwrap(), Some(9));
        assert_eq!(
            world
                .flow
                .capture_checkpoint_image(&world.codecs, FlowCheckpointLimits::default())
                .unwrap(),
            before
        );
        assert_eq!(world.flow.budget_snapshot(), budget);
        let completed = adapter.step(&mut world).unwrap();
        assert_eq!(completed.dispatched_at, 9);
        assert_eq!(completed.target, Some(9));
        assert_eq!(
            world.flow.work_progress(world.work).unwrap().state,
            WorkState::Completed
        );

        let (mut world, callback_work) = stale_shadow_preview_world();
        let callback_context_before = (
            *world.flow.work_context::<u32>(world.work).unwrap(),
            *world.flow.work_context::<u32>(callback_work).unwrap(),
        );
        assert_eq!(callback_context_before, (11, 1));
        let before = world
            .flow
            .capture_checkpoint_image(&world.codecs, FlowCheckpointLimits::default())
            .unwrap();
        assert_eq!(adapter.next_tick(&world).unwrap(), Some(11));
        assert_eq!(
            world
                .flow
                .capture_checkpoint_image(&world.codecs, FlowCheckpointLimits::default())
                .unwrap(),
            before
        );
        assert_eq!(
            (
                *world.flow.work_context::<u32>(world.work).unwrap(),
                *world.flow.work_context::<u32>(callback_work).unwrap(),
            ),
            callback_context_before
        );
        let FlowCheckpointImage::V3(before_v3) = before else {
            panic!("Deferred stale completion must capture as V3");
        };
        let stored = before_v3
            .builtins
            .work_progress
            .as_ref()
            .unwrap()
            .rows
            .iter()
            .find(|(id, _)| *id == world.work.entity_id())
            .unwrap()
            .1
            .clone();
        let stale = adapter.step(&mut world).unwrap();
        assert_eq!(stale.dispatched_at, 11);
        assert_eq!(stale.failure, None);
        assert!(world.last_dispatch.as_ref().unwrap().records.is_empty());
        assert_eq!(
            (
                *world.flow.work_context::<u32>(world.work).unwrap(),
                *world.flow.work_context::<u32>(callback_work).unwrap(),
            ),
            callback_context_before
        );
        assert_eq!(
            world.flow.request(world.request).unwrap().state,
            RequestState::Active
        );
        assert_eq!(
            world.flow.work_progress(world.work).unwrap().useful_elapsed,
            SimDuration::from_ticks(7)
        );
        assert_eq!(
            world.flow.work_progress(world.work).unwrap().remaining,
            SimDuration::from_ticks(3)
        );
        let after = world
            .flow
            .capture_checkpoint_image(&world.codecs, FlowCheckpointLimits::default())
            .unwrap();
        let FlowCheckpointImage::V3(after_v3) = after else {
            panic!("stale dispatch remains a V3 image");
        };
        assert_eq!(
            after_v3
                .builtins
                .work_progress
                .as_ref()
                .unwrap()
                .rows
                .iter()
                .find(|(id, _)| *id == world.work.entity_id())
                .unwrap()
                .1,
            stored
        );
        let budget = world.flow.budget_snapshot();
        assert_eq!(adapter.next_tick(&world).unwrap(), Some(14));
        assert_eq!(world.flow.budget_snapshot(), budget);
        let useful = adapter.step(&mut world).unwrap();
        assert_eq!(useful.dispatched_at, 14);
        assert_eq!(useful.target, Some(14));
    }

    #[test]
    fn native_preview_and_dispatch_preserve_exact_completion_tick() {
        let adapter = NativeFlowAdapter {
            model: Model {
                illicit_extra_step: false,
                fail_after_dispatch: false,
            },
            max_image_bytes: 4096,
        };
        let (snapshot, input) = inputs();
        let mut w = adapter.start(&snapshot, &input).unwrap();
        assert_eq!(adapter.next_tick(&w).unwrap(), Some(0));
        assert_eq!(adapter.target_at_start(&w).unwrap(), None);
        let mut target = None;
        for _ in 0..8 {
            let next = adapter.next_tick(&w).unwrap().unwrap();
            let result = adapter.step(&mut w).unwrap();
            assert_eq!(result.dispatched_at, next);
            assert_eq!(result.dispatches, 1);
            assert_eq!(result.failure, None);
            if result.target.is_some() {
                target = result.target;
                break;
            }
        }
        assert_eq!(target, Some(5));
    }

    #[test]
    fn preview_v3_disabled_queue_has_no_invented_completion() {
        let mut flow = FlowRuntime::new();
        flow.register_work_handlers("preview.v3", WorkHandlers::<u32>::default())
            .unwrap();
        let resource = flow
            .create_resource_with_allocation_mode(1, FlowAllocationMode::Deferred)
            .unwrap();
        let disable = flow
            .schedule_resource_grant_enabled(resource, false, SimTime::from_ticks(0), i32::MIN)
            .unwrap();
        let owner = flow.spawn_actor().unwrap();
        let work = flow
            .create_work(owner, SimDuration::from_ticks(5), "preview.v3", 7u32)
            .unwrap();
        let request = flow
            .acquire(resource)
            .owner(owner)
            .timed_work(work)
            .at(SimTime::from_ticks(0))
            .submit()
            .unwrap();
        let flush = flow
            .schedule_allocation_flush(resource, SimTime::from_ticks(0), i32::MAX)
            .unwrap();
        let mut world = V3PreviewWorld {
            flow,
            codecs: v3_preview_codecs(),
            resource,
            request,
            work,
            last_dispatch: None,
        };
        let adapter = v3_preview_adapter(FlowCheckpointLimits::default());
        assert_eq!(
            world.flow.request(request).unwrap().state,
            RequestState::Pending
        );
        assert!(world.flow.request(request).unwrap().lease.is_none());
        let pending_image = world
            .flow
            .capture_checkpoint_image(&world.codecs, FlowCheckpointLimits::default())
            .unwrap();
        assert_eq!(adapter.next_tick(&world).unwrap(), Some(0));
        assert_eq!(
            world
                .flow
                .capture_checkpoint_image(&world.codecs, FlowCheckpointLimits::default())
                .unwrap(),
            pending_image
        );

        let disable_result = adapter.step(&mut world).unwrap();
        assert_eq!(disable_result.dispatched_at, 0);
        assert_eq!(world.last_dispatch.as_ref().unwrap().event, disable);
        assert_eq!(world.flow.resource_grant_enabled(resource), Ok(false));
        let submit_event = world
            .flow
            .capture_checkpoint_image(&world.codecs, FlowCheckpointLimits::default())
            .unwrap()
            .as_ref()
            .commands()
            .find_map(|(event, command)| {
                (command == kairo_ecs_des::FlowCommandV3::Submit(request)).then_some(event)
            })
            .expect("live Submit command");
        let submit_result = adapter.step(&mut world).unwrap();
        assert_eq!(submit_result.dispatched_at, 0);
        assert_eq!(world.last_dispatch.as_ref().unwrap().event, submit_event);
        assert_eq!(
            world.flow.request(request).unwrap().state,
            RequestState::Queued
        );
        assert!(world.flow.request(request).unwrap().lease.is_none());
        let flush_result = adapter.step(&mut world).unwrap();
        assert_eq!(flush_result.dispatched_at, 0);
        assert_eq!(world.last_dispatch.as_ref().unwrap().event, flush);
        assert_eq!(
            world.flow.request(request).unwrap().state,
            RequestState::Queued
        );
        assert!(world.flow.request(request).unwrap().lease.is_none());
        assert!(world.flow.resource(resource).unwrap().active.is_empty());
        let before = world
            .flow
            .capture_checkpoint_image(&world.codecs, FlowCheckpointLimits::default())
            .unwrap();
        assert_eq!(adapter.next_tick(&world).unwrap(), None);
        assert_eq!(
            world
                .flow
                .capture_checkpoint_image(&world.codecs, FlowCheckpointLimits::default())
                .unwrap(),
            before
        );
    }

    #[test]
    fn preview_v3_halted_live_tick_and_repeat_step_error() {
        let limit = std::num::NonZeroU64::new(1).unwrap();
        let adapter = v3_preview_adapter(FlowCheckpointLimits::default());
        let mut world = v3_preview_world(4, Some(limit));
        let submit = world
            .flow
            .step()
            .unwrap()
            .expect("Submit consumes tick budget");
        assert_eq!(submit.at, SimTime::from_ticks(0));
        assert_eq!(
            world.flow.request(world.request).unwrap().state,
            RequestState::Queued
        );
        let flush = world
            .flow
            .schedule_allocation_flush(world.resource, world.flow.now(), i32::MAX)
            .unwrap();
        assert_eq!(adapter.next_tick(&world).unwrap(), Some(0));
        let first = adapter.step(&mut world).unwrap_err();
        assert_eq!(
            first,
            ShadowError::Adapter(
                "native dispatch: SameTickBudgetExceeded { at_ticks: 0, limit: 1 }".into()
            )
        );
        let halted = world
            .flow
            .capture_checkpoint_image(&world.codecs, FlowCheckpointLimits::default())
            .unwrap();
        let budget = world.flow.budget_snapshot();
        let FlowCheckpointImage::V3(image) = &halted else {
            panic!("budget-halted Deferred image must be V3");
        };
        assert!(image.budget_halt.is_some());
        assert!(image.commands.iter().any(|(event, command)| {
            *event == flush
                && *command == kairo_ecs_des::FlowCommandV3::AllocationFlush(world.resource)
        }));
        assert_eq!(adapter.next_tick(&world).unwrap(), Some(0));
        assert_eq!(
            world
                .flow
                .capture_checkpoint_image(&world.codecs, FlowCheckpointLimits::default())
                .unwrap(),
            halted
        );
        assert_eq!(world.flow.budget_snapshot(), budget);
        assert_eq!(adapter.step(&mut world).unwrap_err(), first);
        assert_eq!(world.flow.budget_snapshot(), budget);
        assert_eq!(adapter.next_tick(&world).unwrap(), Some(0));
        assert_eq!(
            world
                .flow
                .capture_checkpoint_image(&world.codecs, FlowCheckpointLimits::default())
                .unwrap(),
            halted
        );
    }

    #[test]
    fn preview_v3_capture_limit_error_without_legacy_fallback() {
        let world = v3_preview_world(5, None);
        let valid = world
            .flow
            .capture_checkpoint_image(&world.codecs, FlowCheckpointLimits::default())
            .unwrap();
        assert!(matches!(valid, FlowCheckpointImage::V3(_)));
        let limits = FlowCheckpointLimits {
            max_commands: 0,
            ..FlowCheckpointLimits::default()
        };
        let expected = world
            .flow
            .capture_checkpoint_image(&world.codecs, limits)
            .unwrap_err();
        assert!(matches!(
            expected,
            kairo_ecs_des::FlowCheckpointError::LimitExceeded(_)
        ));
        let adapter = v3_preview_adapter(limits);
        assert_eq!(
            adapter.next_tick(&world),
            Err(ShadowError::Adapter(format!(
                "native preview: {expected:?}"
            )))
        );
    }

    #[test]
    fn native_preview_handles_v2_pending_disable_and_horizon_without_advancing() {
        let adapter = NativeFlowAdapter {
            model: GatedModel { enable_at: Some(3) },
            max_image_bytes: 4096,
        };
        let (snapshot, input) = inputs();
        let mut world = adapter.start(&snapshot, &input).unwrap();
        let mut dispatched = Vec::new();

        loop {
            let before = world
                .flow
                .capture_checkpoint_image(&world.codecs, FlowCheckpointLimits::default())
                .unwrap();
            let next = adapter.next_tick(&world).unwrap().unwrap();
            assert_eq!(
                world
                    .flow
                    .capture_checkpoint_image(&world.codecs, FlowCheckpointLimits::default())
                    .unwrap(),
                before
            );
            if next > 6 {
                assert_eq!(next, 8);
                break;
            }
            let receipt = adapter.step(&mut world).unwrap();
            assert_eq!(receipt.dispatched_at, next);
            assert_eq!(receipt.failure, None);
            dispatched.push(receipt.dispatched_at);
        }

        assert_eq!(dispatched, vec![0, 0, 3]);
        assert_eq!(adapter.now(&world), 3);
        assert_eq!(
            world.flow.work_progress(world.work).unwrap().state,
            WorkState::Active
        );
        assert_eq!(
            world.flow.request(world.request).unwrap().state,
            RequestState::Active
        );
        assert_eq!(world.flow.resource(world.resource).unwrap().active.len(), 1);

        let mut completed_world = adapter.start(&snapshot, &input).unwrap();
        loop {
            let next = adapter.next_tick(&completed_world).unwrap().unwrap();
            let receipt = adapter.step(&mut completed_world).unwrap();
            assert_eq!(receipt.dispatched_at, next);
            assert_eq!(receipt.failure, None);
            if let Some(target) = receipt.target {
                assert_eq!(target, 8);
                break;
            }
        }
        assert_eq!(
            completed_world
                .flow
                .work_progress(completed_world.work)
                .unwrap()
                .state,
            WorkState::Completed
        );
    }

    #[test]
    fn native_preview_disabled_queue_has_no_next_event_or_completion() {
        let adapter = NativeFlowAdapter {
            model: GatedModel { enable_at: None },
            max_image_bytes: 4096,
        };
        let (snapshot, input) = inputs();
        let mut world = adapter.start(&snapshot, &input).unwrap();
        let mut dispatched = Vec::new();

        while let Some(next) = adapter.next_tick(&world).unwrap() {
            let receipt = adapter.step(&mut world).unwrap();
            assert_eq!(receipt.dispatched_at, next);
            assert_eq!(receipt.failure, None);
            assert_eq!(receipt.target, None);
            dispatched.push(next);
        }

        assert_eq!(dispatched, vec![0, 0]);
        assert_eq!(adapter.next_tick(&world).unwrap(), None);
        assert_eq!(adapter.now(&world), 0);
        assert_eq!(
            world.flow.request(world.request).unwrap().state,
            RequestState::Queued
        );
        assert_eq!(world.flow.resource(world.resource).unwrap().queued.len(), 1);
        assert_eq!(
            world.flow.work_progress(world.work).unwrap().state,
            WorkState::Pending
        );
    }
    #[test]
    fn native_model_cannot_hide_extra_dispatches_from_probe_budget() {
        let adapter = NativeFlowAdapter {
            model: Model {
                illicit_extra_step: true,
                fail_after_dispatch: false,
            },
            max_image_bytes: 4096,
        };
        let (snapshot, input) = inputs();
        let mut world = adapter.start(&snapshot, &input).unwrap();
        let receipt = adapter.step(&mut world).unwrap();
        assert_eq!(receipt.dispatches, 2);
        assert_eq!(receipt.dispatched_at, adapter.now(&world));
        assert!(matches!(receipt.failure, Some(ShadowError::Contract(_))));
        assert_eq!(receipt.target, None);
    }

    #[test]
    fn native_hook_failure_receipt_retains_consumed_dispatch() {
        let adapter = NativeFlowAdapter {
            model: Model {
                illicit_extra_step: false,
                fail_after_dispatch: true,
            },
            max_image_bytes: 4096,
        };
        let (snapshot, input) = inputs();
        let mut world = adapter.start(&snapshot, &input).unwrap();
        let next = adapter.next_tick(&world).unwrap().unwrap();
        let receipt = adapter.step(&mut world).unwrap();
        assert_eq!(receipt.dispatches, 1);
        assert_eq!(receipt.dispatched_at, next);
        assert_eq!(receipt.dispatched_at, adapter.now(&world));
        assert!(matches!(receipt.failure, Some(ShadowError::Adapter(_))));
        assert_eq!(receipt.target, None);
    }

    #[test]
    fn pool_counts_real_native_dispatch_when_hook_fails_after_dispatch() {
        let adapter = NativeFlowAdapter {
            model: Model {
                illicit_extra_step: false,
                fail_after_dispatch: true,
            },
            max_image_bytes: 4096,
        };
        let (snapshot, input) = inputs();
        let spec = crate::shadow::ProbeSpec {
            id: "native-failure".into(),
            key: crate::residuals::LogicalKey {
                study_id: "study".into(),
                dataset_id: "data".into(),
                scenario_id: "scenario".into(),
                seed_schedule_id: "schedule".into(),
                replication_id: "1".into(),
                case_key: "case".into(),
                task_key: "task".into(),
                occurrence: 0,
                endpoint: input.target.clone(),
                seed_purpose: "service".into(),
                seed_map_ref: "seed-v1".into(),
                mapping_version: "map-v1".into(),
            },
            run_id: "run".into(),
            candidate_id: "candidate".into(),
            anchor_event: "a".into(),
            target_event: Some("future-target".into()),
            observed_target: Some(5),
            input,
            budget: crate::shadow::ProbeBudget {
                horizon: 5,
                max_events: 1,
            },
        };
        let mut pool = ProbePool::new(
            adapter,
            [9; 32],
            PoolLimits {
                max_probes: 2,
                max_snapshot_bytes: 4096,
                max_probe_image_bytes: 4096,
                max_checkpoint_bytes: 16_384,
            },
        );
        pool.admit(spec, snapshot).unwrap();
        let result = pool.advance("native-failure", 1).unwrap().unwrap();
        assert_eq!(result.events, 1);
        assert_eq!(result.last_tick, 0);
        assert!(matches!(
            result.outcome,
            crate::shadow::ProbeOutcome::Failed { .. }
        ));
    }
}
