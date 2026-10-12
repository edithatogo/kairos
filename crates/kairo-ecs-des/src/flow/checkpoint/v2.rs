use super::*;
use std::ops::Deref;

#[doc(hidden)]
pub const FLOW_CHECKPOINT_VERSION_V2: u32 = 2;

#[doc(hidden)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FlowCommandV2 {
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
}

#[doc(hidden)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FlowCheckpointV2 {
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
    pub commands: Vec<(EventId, FlowCommandV2)>,
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
}

/// Full-state representation shared by capture, validation, and restore.
/// It never narrows commands or drops V2-only state.
pub(super) struct FlowCheckpointParts {
    state: FlowCheckpointV3,
    origin: PartsOrigin,
}

#[derive(Clone, Copy)]
pub(super) enum PartsOrigin {
    Capture,
    LegacyV1,
    NativeV2,
    NativeV3,
}

impl FlowCheckpointParts {
    pub(super) fn from_v2(image: FlowCheckpointV2) -> Self {
        Self {
            state: FlowCheckpointV3::from_v2(image),
            origin: PartsOrigin::NativeV2,
        }
    }

    pub(super) fn from_capture(image: FlowCheckpointV3) -> Self {
        Self {
            state: image,
            origin: PartsOrigin::Capture,
        }
    }

    pub(super) fn from_v1(image: FlowCheckpointV1) -> Self {
        Self {
            state: FlowCheckpointV3::from_v1(image),
            origin: PartsOrigin::LegacyV1,
        }
    }

    pub(super) fn from_v3(image: FlowCheckpointV3) -> Self {
        Self {
            state: image,
            origin: PartsOrigin::NativeV3,
        }
    }

    pub(super) fn into_v3(self) -> (FlowCheckpointV3, PartsOrigin) {
        (self.state, self.origin)
    }
}

impl Deref for FlowCheckpointParts {
    type Target = FlowCheckpointV3;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

#[doc(hidden)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FlowCheckpointImage {
    V1(FlowCheckpointV1),
    V2(FlowCheckpointV2),
    V3(FlowCheckpointV3),
    V4(FlowCheckpointV4),
}

#[doc(hidden)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FlowCheckpointRef<'a> {
    V1(&'a FlowCheckpointV1),
    V2(&'a FlowCheckpointV2),
    V3(&'a FlowCheckpointV3),
    V4(&'a FlowCheckpointV4),
}

enum FlowCommandIter<'a> {
    V1(std::slice::Iter<'a, (EventId, FlowCommandV1)>),
    V2(std::slice::Iter<'a, (EventId, FlowCommandV2)>),
    V3(std::slice::Iter<'a, (EventId, FlowCommandV3)>),
}

impl Iterator for FlowCommandIter<'_> {
    type Item = (EventId, FlowCommandV3);

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::V1(iter) => iter.next().map(|(id, command)| (*id, (*command).into())),
            Self::V2(iter) => iter.next().map(|(id, command)| (*id, (*command).into())),
            Self::V3(iter) => iter.next().copied(),
        }
    }
}

impl FlowCheckpointImage {
    pub fn as_ref(&self) -> FlowCheckpointRef<'_> {
        match self {
            Self::V1(image) => FlowCheckpointRef::V1(image),
            Self::V2(image) => FlowCheckpointRef::V2(image),
            Self::V3(image) => FlowCheckpointRef::V3(image),
            Self::V4(image) => FlowCheckpointRef::V4(image),
        }
    }

    pub fn restore_with_rebind(
        self,
        codecs: &FlowCheckpointCodecs,
        limits: FlowCheckpointLimits,
    ) -> Result<(FlowRuntime, FlowCheckpointRebindV1), FlowCheckpointError> {
        match self {
            Self::V1(image) => FlowRuntime::restore_checkpoint_with_rebind(image, codecs, limits),
            Self::V2(image) => {
                FlowRuntime::restore_checkpoint_v2_with_rebind(image, codecs, limits)
            }
            Self::V3(image) => {
                FlowRuntime::restore_checkpoint_v3_with_rebind(image, codecs, limits)
            }
            Self::V4(image) => {
                FlowRuntime::restore_checkpoint_v4_with_rebind(image, codecs, limits)
            }
        }
    }
}

impl<'a> FlowCheckpointRef<'a> {
    pub fn scheduler(self) -> &'a SchedulerCheckpointV1 {
        match self {
            Self::V1(image) => &image.scheduler,
            Self::V2(image) => &image.scheduler,
            Self::V3(image) => &image.scheduler,
            Self::V4(image) => &image.scheduler,
        }
    }

    pub fn work_registrations(self) -> &'a [FlowWorkRegistrationV1] {
        match self {
            Self::V1(image) => &image.work_registrations,
            Self::V2(image) => &image.work_registrations,
            Self::V3(image) => &image.work_registrations,
            Self::V4(image) => &image.work_registrations,
        }
    }

    pub fn context_stores(self) -> &'a [FlowContextStoreV1] {
        match self {
            Self::V1(image) => &image.context_stores,
            Self::V2(image) => &image.context_stores,
            Self::V3(image) => &image.context_stores,
            Self::V4(image) => &image.context_stores,
        }
    }

    pub fn commands(self) -> impl Iterator<Item = (EventId, FlowCommandV3)> + 'a {
        match self {
            Self::V1(image) => FlowCommandIter::V1(image.commands.iter()),
            Self::V2(image) => FlowCommandIter::V2(image.commands.iter()),
            Self::V3(image) => FlowCommandIter::V3(image.commands.iter()),
            Self::V4(image) => FlowCommandIter::V3(image.commands.iter()),
        }
    }
}

impl From<FlowCommandV1> for FlowCommandV2 {
    fn from(command: FlowCommandV1) -> Self {
        match command {
            FlowCommandV1::Submit(x) => Self::Submit(x),
            FlowCommandV1::Release(x) => Self::Release(x),
            FlowCommandV1::Capacity(x, n) => Self::Capacity(x, n),
            FlowCommandV1::Remove(x) => Self::Remove(x),
            FlowCommandV1::Despawn(x) => Self::Despawn(x),
            FlowCommandV1::Deadline(x) => Self::Deadline(x),
            FlowCommandV1::Cancel(x) => Self::Cancel(x),
            FlowCommandV1::Reprioritize(x, n) => Self::Reprioritize(x, n),
            FlowCommandV1::Completion(a, b, c, d) => Self::Completion(a, b, c, d),
            FlowCommandV1::Notify => Self::Notify,
            FlowCommandV1::Domain(a, b) => Self::Domain(a, b),
            FlowCommandV1::DomainControl(a, b, c) => Self::DomainControl(a, b, c),
        }
    }
}

impl TryFrom<FlowCommandV2> for FlowCommandV1 {
    type Error = FlowCheckpointError;

    fn try_from(command: FlowCommandV2) -> Result<Self, Self::Error> {
        Ok(match command {
            FlowCommandV2::Submit(x) => Self::Submit(x),
            FlowCommandV2::Release(x) => Self::Release(x),
            FlowCommandV2::Capacity(x, n) => Self::Capacity(x, n),
            FlowCommandV2::Remove(x) => Self::Remove(x),
            FlowCommandV2::Despawn(x) => Self::Despawn(x),
            FlowCommandV2::Deadline(x) => Self::Deadline(x),
            FlowCommandV2::Cancel(x) => Self::Cancel(x),
            FlowCommandV2::Reprioritize(x, n) => Self::Reprioritize(x, n),
            FlowCommandV2::Completion(a, b, c, d) => Self::Completion(a, b, c, d),
            FlowCommandV2::Notify => Self::Notify,
            FlowCommandV2::Domain(a, b) => Self::Domain(a, b),
            FlowCommandV2::DomainControl(a, b, c) => Self::DomainControl(a, b, c),
            FlowCommandV2::GrantEnabled(..) => {
                return Err(FlowCheckpointError::InvalidState(
                    "V1 cannot represent a resource grant boundary",
                ));
            }
        })
    }
}

impl FlowCheckpointV2 {
    pub(super) fn into_v1(self) -> Result<FlowCheckpointV1, FlowCheckpointError> {
        if self
            .resource_grant_flags
            .iter()
            .any(|(_, enabled)| !enabled)
        {
            return Err(FlowCheckpointError::InvalidState(
                "V1 cannot represent disabled resource grant eligibility",
            ));
        }
        let Self {
            config,
            callback_config,
            next_batch_identity,
            budget_tick,
            budget_consumed,
            budget_halt,
            scheduler,
            world,
            resources,
            requests,
            actors,
            actor_domains,
            work_registrations,
            contexts,
            handlers,
            continuations,
            domains,
            builtins,
            context_stores,
            restart_store_manifest,
            restart_stores,
            commands,
            notifications,
            pending_releases,
            pending_despawns,
            created,
            destroyed,
            scheduled,
            next_admission,
            next_lease,
            ..
        } = self;
        Ok(FlowCheckpointV1 {
            version: 1,
            config,
            callback_config,
            next_batch_identity,
            budget_tick,
            budget_consumed,
            budget_halt,
            scheduler,
            world,
            resources,
            requests,
            actors,
            actor_domains,
            work_registrations,
            contexts,
            handlers,
            continuations,
            domains,
            builtins,
            context_stores,
            restart_store_manifest,
            restart_stores,
            commands: commands
                .into_iter()
                .map(|(id, command)| Ok((id, command.try_into()?)))
                .collect::<Result<_, FlowCheckpointError>>()?,
            notifications,
            pending_releases,
            pending_despawns,
            created,
            destroyed,
            scheduled,
            next_admission,
            next_lease,
        })
    }
}

impl FlowCheckpointV1 {
    pub(super) fn into_v2(self) -> FlowCheckpointV2 {
        let frontier = (self.scheduler.dispatched_events > 0).then_some(self.scheduler.now);
        let mut resource_grant_flags: Vec<_> = self
            .resources
            .iter()
            .copied()
            .map(|id| (id, true))
            .collect();
        resource_grant_flags.sort_by_key(|(id, _)| *id);
        FlowCheckpointV2 {
            version: FLOW_CHECKPOINT_VERSION_V2,
            config: self.config,
            callback_config: self.callback_config,
            next_batch_identity: self.next_batch_identity,
            budget_tick: self.budget_tick,
            budget_consumed: self.budget_consumed,
            budget_halt: self.budget_halt,
            scheduler: self.scheduler,
            world: self.world,
            resource_grant_flags,
            started_frontier: frontier,
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
                .map(|(id, c)| (id, c.into()))
                .collect(),
            notifications: self.notifications,
            pending_releases: self.pending_releases,
            pending_despawns: self.pending_despawns,
            created: self.created,
            destroyed: self.destroyed,
            scheduled: self.scheduled,
            next_admission: self.next_admission,
            next_lease: self.next_lease,
        }
    }
}

impl TryFrom<Command> for FlowCommandV2 {
    type Error = FlowCheckpointError;

    fn try_from(command: Command) -> Result<Self, Self::Error> {
        Ok(match command {
            Command::Submit(x) => Self::Submit(x),
            Command::Release(x) => Self::Release(x),
            Command::Capacity(x, n) => Self::Capacity(x, n),
            Command::Remove(x) => Self::Remove(x),
            Command::Despawn(x) => Self::Despawn(x),
            Command::Deadline(x) => Self::Deadline(x),
            Command::Cancel(x) => Self::Cancel(x),
            Command::Reprioritize(x, n) => Self::Reprioritize(x, n),
            Command::Completion(a, b, c, d) => Self::Completion(a, b, c, d),
            Command::Notify => Self::Notify,
            Command::Domain(a, b) => Self::Domain(a, b),
            Command::DomainControl(a, b, c) => Self::DomainControl(a, b, c),
            Command::GrantEnabled(a, b) => Self::GrantEnabled(a, b),
            Command::AllocationFlush(..) => {
                return Err(FlowCheckpointError::InvalidState(
                    "V2 cannot represent deferred resource allocation",
                ));
            }
        })
    }
}

impl From<FlowCommandV2> for Command {
    fn from(command: FlowCommandV2) -> Self {
        match command {
            FlowCommandV2::Submit(x) => Self::Submit(x),
            FlowCommandV2::Release(x) => Self::Release(x),
            FlowCommandV2::Capacity(x, n) => Self::Capacity(x, n),
            FlowCommandV2::Remove(x) => Self::Remove(x),
            FlowCommandV2::Despawn(x) => Self::Despawn(x),
            FlowCommandV2::Deadline(x) => Self::Deadline(x),
            FlowCommandV2::Cancel(x) => Self::Cancel(x),
            FlowCommandV2::Reprioritize(x, n) => Self::Reprioritize(x, n),
            FlowCommandV2::Completion(a, b, c, d) => Self::Completion(a, b, c, d),
            FlowCommandV2::Notify => Self::Notify,
            FlowCommandV2::Domain(a, b) => Self::Domain(a, b),
            FlowCommandV2::DomainControl(a, b, c) => Self::DomainControl(a, b, c),
            FlowCommandV2::GrantEnabled(a, b) => Self::GrantEnabled(a, b),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_rejects_stale_grant_flag_keys_and_corrupt_boundary_cache() {
        let codecs = FlowCheckpointCodecs::new();
        let limits = FlowCheckpointLimits::default();

        let mut stale = FlowRuntime::new();
        let resource = stale.create_resource(1).unwrap();
        stale.resources.remove(&resource);
        // New resources use implicit true; insert an actual stale sparse flag.
        stale.grant_enabled.insert(resource, true);
        assert!(matches!(
            stale.capture_checkpoint_parts(&codecs, limits, true),
            Err(FlowCheckpointError::InvalidState(_))
        ));

        let mut corrupt = FlowRuntime::new();
        let resource = corrupt.create_resource(1).unwrap();
        corrupt
            .schedule_resource_grant_enabled(resource, false, SimTime::from_ticks(4), 0)
            .unwrap();
        corrupt.pending_grant_boundaries.clear();
        assert!(matches!(
            corrupt.capture_checkpoint_parts(&codecs, limits, true),
            Err(FlowCheckpointError::InvalidState(_))
        ));
    }
    #[test]
    fn capture_rejects_wrong_cache_event_resource_and_non_gate_command() {
        for case in 0..3 {
            let mut source = FlowRuntime::new();
            let resource = source.create_resource(1).unwrap();
            let other = source.create_resource(1).unwrap();
            let at = SimTime::from_ticks(4);
            let event = source
                .schedule_resource_grant_enabled(resource, false, at, 0)
                .unwrap();
            match case {
                0 => {
                    source
                        .pending_grant_boundaries
                        .insert((resource, at), EventId::new(u64::MAX, u32::MAX));
                }
                1 => {
                    source.pending_grant_boundaries.clear();
                    source.pending_grant_boundaries.insert((other, at), event);
                }
                _ => {
                    source.commands.insert(event, Command::Notify);
                }
            }
            assert!(
                matches!(
                    source.capture_checkpoint_image(
                        &FlowCheckpointCodecs::new(),
                        FlowCheckpointLimits::default()
                    ),
                    Err(FlowCheckpointError::InvalidState(_))
                ),
                "case {case}"
            );
        }
    }
}
