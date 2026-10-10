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
            .capture_checkpoint(self.model.codecs(runtime), self.model.capture_limits())
            .map_err(|error| ShadowError::Adapter(format!("native preview: {error:?}")))?;
        Ok(image
            .scheduler
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
        FlowCheckpointCodecError, FlowCheckpointRebindV1, FlowHandlerCodeIds, WorkHandlers, WorkId,
        WorkState,
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
