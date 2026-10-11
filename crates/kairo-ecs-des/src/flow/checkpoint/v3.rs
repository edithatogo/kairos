use super::*;

#[doc(hidden)]
pub const FLOW_CHECKPOINT_VERSION_V3: u32 = 3;

#[doc(hidden)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FlowCommandV3 {
    Submit(RequestId),
    Release(LeaseId),
    Capacity(ResourceId, u32),
    Remove(ResourceId),
    Despawn(EntityId),
    Deadline(RequestId),
    Cancel(RequestId),
    Reprioritize(RequestId, i32),
    Completion(RequestId, LeaseId, u64, SimTime),
    Notify,
    Domain(WorkId, EventKind),
    DomainControl(WorkId, EventKind, FlowDomainControl),
    GrantEnabled(ResourceId, bool),
    AllocationFlush(ResourceId),
}

impl From<FlowCommandV1> for FlowCommandV3 {
    fn from(command: FlowCommandV1) -> Self {
        match command {
            FlowCommandV1::Submit(value) => Self::Submit(value),
            FlowCommandV1::Release(value) => Self::Release(value),
            FlowCommandV1::Capacity(resource, value) => Self::Capacity(resource, value),
            FlowCommandV1::Remove(value) => Self::Remove(value),
            FlowCommandV1::Despawn(value) => Self::Despawn(value),
            FlowCommandV1::Deadline(value) => Self::Deadline(value),
            FlowCommandV1::Cancel(value) => Self::Cancel(value),
            FlowCommandV1::Reprioritize(request, priority) => Self::Reprioritize(request, priority),
            FlowCommandV1::Completion(request, lease, revision, at) => {
                Self::Completion(request, lease, revision, at)
            }
            FlowCommandV1::Notify => Self::Notify,
            FlowCommandV1::Domain(work, kind) => Self::Domain(work, kind),
            FlowCommandV1::DomainControl(work, kind, action) => {
                Self::DomainControl(work, kind, action)
            }
        }
    }
}

impl From<FlowCommandV2> for FlowCommandV3 {
    fn from(command: FlowCommandV2) -> Self {
        match command {
            FlowCommandV2::Submit(value) => Self::Submit(value),
            FlowCommandV2::Release(value) => Self::Release(value),
            FlowCommandV2::Capacity(resource, value) => Self::Capacity(resource, value),
            FlowCommandV2::Remove(value) => Self::Remove(value),
            FlowCommandV2::Despawn(value) => Self::Despawn(value),
            FlowCommandV2::Deadline(value) => Self::Deadline(value),
            FlowCommandV2::Cancel(value) => Self::Cancel(value),
            FlowCommandV2::Reprioritize(request, priority) => Self::Reprioritize(request, priority),
            FlowCommandV2::Completion(request, lease, revision, at) => {
                Self::Completion(request, lease, revision, at)
            }
            FlowCommandV2::Notify => Self::Notify,
            FlowCommandV2::Domain(work, kind) => Self::Domain(work, kind),
            FlowCommandV2::DomainControl(work, kind, action) => {
                Self::DomainControl(work, kind, action)
            }
            FlowCommandV2::GrantEnabled(resource, enabled) => Self::GrantEnabled(resource, enabled),
        }
    }
}

impl TryFrom<Command> for FlowCommandV3 {
    type Error = FlowCheckpointError;

    fn try_from(command: Command) -> Result<Self, Self::Error> {
        Ok(match command {
            Command::Submit(value) => Self::Submit(value),
            Command::Release(value) => Self::Release(value),
            Command::Capacity(resource, value) => Self::Capacity(resource, value),
            Command::GrantEnabled(resource, enabled) => Self::GrantEnabled(resource, enabled),
            Command::AllocationFlush(resource) => Self::AllocationFlush(resource),
            Command::Remove(value) => Self::Remove(value),
            Command::Despawn(value) => Self::Despawn(value),
            Command::Deadline(value) => Self::Deadline(value),
            Command::Cancel(value) => Self::Cancel(value),
            Command::Reprioritize(request, priority) => Self::Reprioritize(request, priority),
            Command::Completion(request, lease, revision, at) => {
                Self::Completion(request, lease, revision, at)
            }
            Command::Notify => Self::Notify,
            Command::Domain(work, kind) => Self::Domain(work, kind),
            Command::DomainControl(work, kind, action) => Self::DomainControl(work, kind, action),
        })
    }
}

impl From<FlowCommandV3> for Command {
    fn from(command: FlowCommandV3) -> Self {
        match command {
            FlowCommandV3::Submit(value) => Self::Submit(value),
            FlowCommandV3::Release(value) => Self::Release(value),
            FlowCommandV3::Capacity(resource, value) => Self::Capacity(resource, value),
            FlowCommandV3::GrantEnabled(resource, enabled) => Self::GrantEnabled(resource, enabled),
            FlowCommandV3::AllocationFlush(resource) => Self::AllocationFlush(resource),
            FlowCommandV3::Remove(value) => Self::Remove(value),
            FlowCommandV3::Despawn(value) => Self::Despawn(value),
            FlowCommandV3::Deadline(value) => Self::Deadline(value),
            FlowCommandV3::Cancel(value) => Self::Cancel(value),
            FlowCommandV3::Reprioritize(request, priority) => Self::Reprioritize(request, priority),
            FlowCommandV3::Completion(request, lease, revision, at) => {
                Self::Completion(request, lease, revision, at)
            }
            FlowCommandV3::Notify => Self::Notify,
            FlowCommandV3::Domain(work, kind) => Self::Domain(work, kind),
            FlowCommandV3::DomainControl(work, kind, action) => {
                Self::DomainControl(work, kind, action)
            }
        }
    }
}

impl FlowCommandV3 {
    pub(super) fn try_into_v2(self) -> Result<FlowCommandV2, FlowCheckpointError> {
        Ok(match self {
            FlowCommandV3::Submit(value) => FlowCommandV2::Submit(value),
            FlowCommandV3::Release(value) => FlowCommandV2::Release(value),
            FlowCommandV3::Capacity(resource, value) => FlowCommandV2::Capacity(resource, value),
            FlowCommandV3::Remove(value) => FlowCommandV2::Remove(value),
            FlowCommandV3::Despawn(value) => FlowCommandV2::Despawn(value),
            FlowCommandV3::Deadline(value) => FlowCommandV2::Deadline(value),
            FlowCommandV3::Cancel(value) => FlowCommandV2::Cancel(value),
            FlowCommandV3::Reprioritize(request, priority) => {
                FlowCommandV2::Reprioritize(request, priority)
            }
            FlowCommandV3::Completion(request, lease, revision, at) => {
                FlowCommandV2::Completion(request, lease, revision, at)
            }
            FlowCommandV3::Notify => FlowCommandV2::Notify,
            FlowCommandV3::Domain(work, kind) => FlowCommandV2::Domain(work, kind),
            FlowCommandV3::DomainControl(work, kind, action) => {
                FlowCommandV2::DomainControl(work, kind, action)
            }
            FlowCommandV3::GrantEnabled(resource, enabled) => {
                FlowCommandV2::GrantEnabled(resource, enabled)
            }
            FlowCommandV3::AllocationFlush(_) => {
                return Err(FlowCheckpointError::InvalidState(
                    "V2 cannot represent deferred resource allocation",
                ));
            }
        })
    }
}

#[doc(hidden)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FlowCheckpointV3 {
    pub version: u32,
    pub config: FlowConfig,
    pub callback_config: FlowCallbackConfig,
    pub next_batch_identity: u64,
    pub budget_tick: Option<SimTime>,
    pub budget_consumed: u64,
    pub budget_halt: Option<FlowBudgetHalt>,
    pub scheduler: SchedulerCheckpointV1,
    pub world: WorldCheckpointV1,
    pub resources: Vec<ResourceId>,
    pub requests: Vec<RequestId>,
    pub actors: Vec<EntityId>,
    pub actor_domains: Vec<(EntityId, EntityId)>,
    pub work_registrations: Vec<FlowWorkRegistrationV1>,
    pub contexts: Vec<FlowContextRegistrationV1>,
    pub handlers: Vec<FlowCallbackRegistrationV1>,
    pub continuations: Vec<FlowCallbackRegistrationV1>,
    pub domains: Vec<FlowCallbackRegistrationV1>,
    pub builtins: FlowBuiltInStoresV1,
    pub context_stores: Vec<FlowContextStoreV1>,
    pub restart_store_manifest: Vec<(String, u32)>,
    pub restart_stores: Vec<FlowRestartStoreV1>,
    pub commands: Vec<(EventId, FlowCommandV3)>,
    pub notifications: Vec<(EventId, FlowNotificationV1)>,
    pub pending_releases: Vec<LeaseId>,
    pub pending_despawns: Vec<EntityId>,
    pub created: u64,
    pub destroyed: u64,
    pub scheduled: u64,
    pub next_admission: u64,
    pub next_lease: u64,
    pub resource_grant_flags: Vec<(ResourceId, bool)>,
    pub started_frontier: Option<SimTime>,
    pub allocation_modes: Vec<(ResourceId, FlowAllocationMode)>,
}

impl FlowCheckpointV3 {
    pub(super) fn from_v1(image: FlowCheckpointV1) -> Self {
        Self::from_v2(image.into_v2())
    }

    pub(super) fn from_v2(image: FlowCheckpointV2) -> Self {
        let mut allocation_modes: Vec<_> = image
            .resources
            .iter()
            .copied()
            .map(|resource| (resource, FlowAllocationMode::Eager))
            .collect();
        allocation_modes.sort_by_key(|(resource, _)| *resource);
        Self {
            version: FLOW_CHECKPOINT_VERSION_V3,
            config: image.config,
            callback_config: image.callback_config,
            next_batch_identity: image.next_batch_identity,
            budget_tick: image.budget_tick,
            budget_consumed: image.budget_consumed,
            budget_halt: image.budget_halt,
            scheduler: image.scheduler,
            world: image.world,
            resources: image.resources,
            requests: image.requests,
            actors: image.actors,
            actor_domains: image.actor_domains,
            work_registrations: image.work_registrations,
            contexts: image.contexts,
            handlers: image.handlers,
            continuations: image.continuations,
            domains: image.domains,
            builtins: image.builtins,
            context_stores: image.context_stores,
            restart_store_manifest: image.restart_store_manifest,
            restart_stores: image.restart_stores,
            commands: image
                .commands
                .into_iter()
                .map(|(event, command)| (event, command.into()))
                .collect(),
            notifications: image.notifications,
            pending_releases: image.pending_releases,
            pending_despawns: image.pending_despawns,
            created: image.created,
            destroyed: image.destroyed,
            scheduled: image.scheduled,
            next_admission: image.next_admission,
            next_lease: image.next_lease,
            resource_grant_flags: image.resource_grant_flags,
            started_frontier: image.started_frontier,
            allocation_modes,
        }
    }

    pub(super) fn into_v2(self) -> Result<FlowCheckpointV2, FlowCheckpointError> {
        if self
            .allocation_modes
            .iter()
            .any(|(_, mode)| *mode != FlowAllocationMode::Eager)
        {
            return Err(FlowCheckpointError::InvalidState(
                "V2 cannot represent deferred resource allocation",
            ));
        }
        Ok(FlowCheckpointV2 {
            version: FLOW_CHECKPOINT_VERSION_V2,
            config: self.config,
            callback_config: self.callback_config,
            next_batch_identity: self.next_batch_identity,
            budget_tick: self.budget_tick,
            budget_consumed: self.budget_consumed,
            budget_halt: self.budget_halt,
            scheduler: self.scheduler,
            world: self.world,
            resources: self.resources,
            requests: self.requests,
            actors: self.actors,
            actor_domains: self.actor_domains,
            work_registrations: self.work_registrations,
            contexts: self.contexts,
            handlers: self.handlers,
            continuations: self.continuations,
            domains: self.domains,
            builtins: self.builtins,
            context_stores: self.context_stores,
            restart_store_manifest: self.restart_store_manifest,
            restart_stores: self.restart_stores,
            commands: self
                .commands
                .into_iter()
                .map(|(event, command)| Ok((event, command.try_into_v2()?)))
                .collect::<Result<_, FlowCheckpointError>>()?,
            notifications: self.notifications,
            pending_releases: self.pending_releases,
            pending_despawns: self.pending_despawns,
            created: self.created,
            destroyed: self.destroyed,
            scheduled: self.scheduled,
            next_admission: self.next_admission,
            next_lease: self.next_lease,
            resource_grant_flags: self.resource_grant_flags,
            started_frontier: self.started_frontier,
        })
    }

    pub(super) fn into_v1(self) -> Result<FlowCheckpointV1, FlowCheckpointError> {
        self.into_v2()?.into_v1()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    thread_local! { static ENCODER_CALLS: Cell<usize> = const { Cell::new(0) }; }

    fn encode(value: &u64, remaining: usize) -> Result<Vec<u8>, FlowCheckpointCodecError> {
        ENCODER_CALLS.with(|calls| calls.set(calls.get() + 1));
        if remaining < 8 {
            return Err(FlowCheckpointCodecError("payload limit".to_owned()));
        }
        Ok(value.to_le_bytes().to_vec())
    }

    fn decode(bytes: &[u8], _: &FlowCheckpointRebindV1) -> Result<u64, FlowCheckpointCodecError> {
        let bytes: [u8; 8] = bytes
            .try_into()
            .map_err(|_| FlowCheckpointCodecError("bad payload".to_owned()))?;
        Ok(u64::from_le_bytes(bytes))
    }

    fn assert_capture_rejects_without_mutation(
        runtime: &FlowRuntime,
        codecs: &FlowCheckpointCodecs,
    ) {
        let budget = runtime.budget_snapshot();
        let commands = runtime.commands.clone();
        let modes = runtime.allocation_modes.clone();
        let flushes = runtime.pending_allocation_flushes.clone();
        ENCODER_CALLS.with(|calls| calls.set(0));
        assert!(matches!(
            runtime.capture_checkpoint_v3(codecs, FlowCheckpointLimits::default()),
            Err(FlowCheckpointError::InvalidState(_))
        ));
        ENCODER_CALLS.with(|calls| assert_eq!(calls.get(), 0));
        assert_eq!(runtime.budget_snapshot(), budget);
        assert_eq!(runtime.commands, commands);
        assert_eq!(runtime.allocation_modes, modes);
        assert_eq!(runtime.pending_allocation_flushes, flushes);
    }

    #[test]
    fn native_v3_capture_cache_corruption_rejects_before_codecs() {
        let mut runtime = FlowRuntime::new();
        let resource = runtime
            .create_resource_with_allocation_mode(1, FlowAllocationMode::Deferred)
            .unwrap();
        let owner = runtime.spawn_actor().unwrap();
        runtime
            .create_work(owner, SimDuration::from_ticks(4), "ctx", 9_u64)
            .unwrap();
        runtime
            .schedule_allocation_flush(resource, SimTime::from_ticks(4), 0)
            .unwrap();
        let eager = runtime.create_resource(1).unwrap();
        let advance = runtime
            .schedule_resource_grant_enabled(eager, false, SimTime::from_ticks(1), 0)
            .unwrap();
        let dispatch = runtime.step().unwrap().unwrap();
        assert_eq!(dispatch.event, advance);
        assert_eq!(dispatch.error, None);
        assert_eq!(runtime.now(), SimTime::from_ticks(1));
        let mut codecs = FlowCheckpointCodecs::new();
        codecs.register_context("ctx", 1, encode, decode).unwrap();
        ENCODER_CALLS.with(|calls| calls.set(0));
        let original = runtime
            .capture_checkpoint_v3(&codecs, FlowCheckpointLimits::default())
            .unwrap();
        ENCODER_CALLS.with(|calls| assert!(calls.get() > 0));

        let scheduler_limits = SchedulerCheckpointLimits {
            max_entries: FlowCheckpointLimits::default().max_scheduler_entries,
        };
        let valid_scheduler = runtime
            .scheduler
            .checkpoint_state(scheduler_limits)
            .unwrap();
        let valid_time_flushes = runtime.pending_allocation_flushes.clone();
        let (&(flush_resource, _), &flush_event) = valid_time_flushes.iter().next().unwrap();
        let mut past_scheduler = valid_scheduler.clone();
        past_scheduler
            .entries
            .iter_mut()
            .find(|entry| entry.id == flush_event)
            .unwrap()
            .request
            .at = SimTime::ZERO;
        runtime.scheduler =
            kairo_ecs_core::Scheduler::from_checkpoint_state(past_scheduler, scheduler_limits)
                .unwrap();
        runtime.pending_allocation_flushes.clear();
        runtime
            .pending_allocation_flushes
            .insert((flush_resource, SimTime::ZERO), flush_event);
        assert_capture_rejects_without_mutation(&runtime, &codecs);
        runtime.scheduler =
            kairo_ecs_core::Scheduler::from_checkpoint_state(valid_scheduler, scheduler_limits)
                .unwrap();
        runtime.pending_allocation_flushes = valid_time_flushes;

        runtime.allocation_modes.remove(&resource);
        assert_capture_rejects_without_mutation(&runtime, &codecs);

        runtime
            .allocation_modes
            .insert(resource, FlowAllocationMode::Deferred);
        let valid_modes = runtime.allocation_modes.clone();
        let stale = ResourceId(kairo_ecs_types::EntityId::new(u64::MAX, u32::MAX));
        runtime.allocation_modes.remove(&resource);
        runtime
            .allocation_modes
            .insert(stale, FlowAllocationMode::Eager);
        assert_capture_rejects_without_mutation(&runtime, &codecs);
        runtime.allocation_modes = valid_modes;

        let valid_flushes = runtime.pending_allocation_flushes.clone();
        let (&key, &event) = valid_flushes.iter().next().unwrap();
        let corruptions = [
            ((key.0, SimTime::from_ticks(5)), event),
            ((stale, key.1), event),
            (key, kairo_ecs_types::EventId::new(u64::MAX, u32::MAX)),
        ];
        for (bad_key, bad_event) in corruptions {
            runtime.pending_allocation_flushes.clear();
            runtime
                .pending_allocation_flushes
                .insert(bad_key, bad_event);
            assert_capture_rejects_without_mutation(&runtime, &codecs);
        }
        runtime.pending_allocation_flushes = valid_flushes.clone();
        let command = runtime.commands.get(&event).copied().unwrap();
        runtime.commands.insert(event, Command::Notify);
        assert_capture_rejects_without_mutation(&runtime, &codecs);
        runtime.commands.insert(event, command);
        runtime.pending_allocation_flushes.clear();
        assert_capture_rejects_without_mutation(&runtime, &codecs);
        runtime.pending_allocation_flushes = valid_flushes;
        let removed_command = runtime.commands.remove(&event).unwrap();
        let removed_index = runtime.pending_allocation_flushes.remove(&key).unwrap();
        assert_capture_rejects_without_mutation(&runtime, &codecs);
        runtime.commands.insert(event, removed_command);
        runtime
            .pending_allocation_flushes
            .insert(key, removed_index);
        assert_eq!(
            runtime
                .capture_checkpoint_v3(&codecs, FlowCheckpointLimits::default())
                .unwrap(),
            original
        );
    }
}
