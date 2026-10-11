use super::*;
use std::collections::{BTreeMap, BTreeSet};

/// Native Flow checkpoint version 4.
#[doc(hidden)]
pub const FLOW_CHECKPOINT_VERSION_V4: u32 = 4;

/// Queue-order tag for every retained request in a V4 image.
#[doc(hidden)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FlowRequestQueueOrderV4 {
    AdmissionOrder,
    StableKey(Vec<u8>),
}

type QueueOrderModeRows = Vec<(ResourceId, FlowQueueOrdering)>;
type RequestQueueOrderRows = Vec<(RequestId, FlowRequestQueueOrderV4)>;

struct CapturedQueueOrderMetadata {
    modes: QueueOrderModeRows,
    requests: RequestQueueOrderRows,
    key_bytes: usize,
}

struct FlowCheckpointV4Parts {
    base: FlowCheckpointV3,
    modes: QueueOrderModeRows,
    requests: RequestQueueOrderRows,
}

/// Complete lossless native Flow image with explicit stable queue-order metadata.
#[doc(hidden)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FlowCheckpointV4 {
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
    pub queue_order_modes: Vec<(ResourceId, FlowQueueOrdering)>,
    pub request_queue_orders: Vec<(RequestId, FlowRequestQueueOrderV4)>,
}

pub(super) struct ValidatedQueueOrderMetadata {
    pub(super) modes: BTreeMap<ResourceId, FlowQueueOrdering>,
    pub(super) keys: BTreeMap<RequestId, FlowQueueOrderKey>,
}

impl FlowCheckpointV4 {
    fn from_v3(
        image: FlowCheckpointV3,
        queue_order_modes: QueueOrderModeRows,
        request_queue_orders: RequestQueueOrderRows,
    ) -> Self {
        Self {
            version: FLOW_CHECKPOINT_VERSION_V4,
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
            commands: image.commands,
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
            allocation_modes: image.allocation_modes,
            queue_order_modes,
            request_queue_orders,
        }
    }

    fn into_parts(self) -> FlowCheckpointV4Parts {
        FlowCheckpointV4Parts {
            base: FlowCheckpointV3 {
                version: FLOW_CHECKPOINT_VERSION_V3,
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
                commands: self.commands,
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
                allocation_modes: self.allocation_modes,
            },
            modes: self.queue_order_modes,
            requests: self.request_queue_orders,
        }
    }
}

fn checked_key_byte_sum(current: usize, next: usize) -> Result<usize, FlowCheckpointError> {
    current
        .checked_add(next)
        .ok_or(FlowCheckpointError::LimitExceeded("key bytes"))
}

fn remaining_key_budget(
    limits: FlowCheckpointLimits,
    queue_key_bytes: usize,
) -> Result<FlowCheckpointLimits, FlowCheckpointError> {
    let remaining = limits
        .max_key_bytes
        .checked_sub(queue_key_bytes)
        .ok_or(FlowCheckpointError::LimitExceeded("key bytes"))?;
    Ok(FlowCheckpointLimits {
        max_key_bytes: remaining,
        ..limits
    })
}

fn preflight_request_key_rows(
    rows: &[(RequestId, FlowRequestQueueOrderV4)],
    max_rows: usize,
) -> Result<usize, FlowCheckpointError> {
    if rows.len() > max_rows {
        return Err(FlowCheckpointError::LimitExceeded(
            "queue order request rows",
        ));
    }
    let mut key_bytes = 0usize;
    for (_, ordering) in rows {
        if let FlowRequestQueueOrderV4::StableKey(bytes) = ordering {
            if bytes.is_empty() {
                return Err(FlowCheckpointError::InvalidState(
                    "stable queue-order key is empty",
                ));
            }
            if bytes.len() > MAX_FLOW_QUEUE_ORDER_KEY_BYTES {
                return Err(FlowCheckpointError::LimitExceeded("queue order key bytes"));
            }
            key_bytes = checked_key_byte_sum(key_bytes, bytes.len())?;
        }
    }
    Ok(key_bytes)
}

fn capture_queue_order_metadata(
    runtime: &FlowRuntime,
    limits: FlowCheckpointLimits,
) -> Result<CapturedQueueOrderMetadata, FlowCheckpointError> {
    if runtime.resources.len() > limits.max_resources
        || runtime.queue_orderings.len() > limits.max_resources
    {
        return Err(FlowCheckpointError::LimitExceeded("queue order mode rows"));
    }
    if runtime.requests.len() > limits.max_requests
        || runtime.queue_order_keys.len() > limits.max_requests
    {
        return Err(FlowCheckpointError::LimitExceeded(
            "queue order request rows",
        ));
    }
    let mut key_bytes = 0usize;
    for key in runtime.queue_order_keys.values() {
        let len = key.as_bytes().len();
        if len == 0 {
            return Err(FlowCheckpointError::InvalidState(
                "stable queue-order key is empty",
            ));
        }
        if len > MAX_FLOW_QUEUE_ORDER_KEY_BYTES {
            return Err(FlowCheckpointError::LimitExceeded("queue order key bytes"));
        }
        key_bytes = checked_key_byte_sum(key_bytes, len)?;
    }
    if key_bytes > limits.max_key_bytes {
        return Err(FlowCheckpointError::LimitExceeded("key bytes"));
    }
    runtime.validate_queue_order_indexes().map_err(|_| {
        FlowCheckpointError::InvalidState("queue ordering indexes are inconsistent")
    })?;

    let mut removed_modes = BTreeMap::new();
    for request_id in &runtime.requests {
        let request = runtime
            .registry
            .get::<ResourceRequest>(request_id.entity_id())
            .ok_or(FlowCheckpointError::InvalidState(
                "request identity has no request row",
            ))?;
        if !runtime.resources.contains(&request.resource) {
            if !terminal(request.state) {
                return Err(FlowCheckpointError::InvalidState(
                    "nonterminal request references a removed resource",
                ));
            }
            let has_key = runtime.queue_order_keys.contains_key(request_id);
            if removed_modes
                .insert(request.resource, has_key)
                .is_some_and(|previous| previous != has_key)
            {
                return Err(FlowCheckpointError::InvalidState(
                    "removed-resource queue-order group is inconsistent",
                ));
            }
        }
    }

    let mut modes = Vec::new();
    modes
        .try_reserve_exact(runtime.queue_orderings.len())
        .map_err(|_| FlowCheckpointError::AllocationFailed)?;
    modes.extend(
        runtime
            .queue_orderings
            .iter()
            .map(|(resource, mode)| (*resource, *mode)),
    );

    let mut requests = Vec::new();
    requests
        .try_reserve_exact(runtime.requests.len())
        .map_err(|_| FlowCheckpointError::AllocationFailed)?;
    for request_id in &runtime.requests {
        let request = runtime
            .registry
            .get::<ResourceRequest>(request_id.entity_id())
            .ok_or(FlowCheckpointError::InvalidState(
                "request identity has no request row",
            ))?;
        let ordering = match runtime.queue_orderings.get(&request.resource) {
            Some(FlowQueueOrdering::AdmissionOrder) => {
                if runtime.queue_order_keys.contains_key(request_id) {
                    return Err(FlowCheckpointError::InvalidState(
                        "admission-order request has a stable key",
                    ));
                }
                FlowRequestQueueOrderV4::AdmissionOrder
            }
            Some(FlowQueueOrdering::StableKey) => {
                let key = runtime.queue_order_keys.get(request_id).ok_or(
                    FlowCheckpointError::InvalidState("stable-order request has no stable key"),
                )?;
                let mut bytes = Vec::new();
                bytes
                    .try_reserve_exact(key.as_bytes().len())
                    .map_err(|_| FlowCheckpointError::AllocationFailed)?;
                bytes.extend_from_slice(key.as_bytes());
                FlowRequestQueueOrderV4::StableKey(bytes)
            }
            None if terminal(request.state) => {
                if let Some(key) = runtime.queue_order_keys.get(request_id) {
                    let mut bytes = Vec::new();
                    bytes
                        .try_reserve_exact(key.as_bytes().len())
                        .map_err(|_| FlowCheckpointError::AllocationFailed)?;
                    bytes.extend_from_slice(key.as_bytes());
                    FlowRequestQueueOrderV4::StableKey(bytes)
                } else {
                    FlowRequestQueueOrderV4::AdmissionOrder
                }
            }
            None => {
                return Err(FlowCheckpointError::InvalidState(
                    "nonterminal request references a removed resource",
                ));
            }
        };
        requests.push((*request_id, ordering));
    }
    Ok(CapturedQueueOrderMetadata {
        modes,
        requests,
        key_bytes,
    })
}

fn validate_queue_order_metadata(
    image: &FlowCheckpointV3,
    mode_rows: &[(ResourceId, FlowQueueOrdering)],
    request_rows: &[(RequestId, FlowRequestQueueOrderV4)],
) -> Result<ValidatedQueueOrderMetadata, FlowCheckpointError> {
    if mode_rows.windows(2).any(|pair| pair[0].0 >= pair[1].0)
        || request_rows.windows(2).any(|pair| pair[0].0 >= pair[1].0)
    {
        return Err(FlowCheckpointError::InvalidState(
            "queue-order metadata is not canonical",
        ));
    }
    let resources: BTreeSet<_> = image.resources.iter().copied().collect();
    if resources.len() != image.resources.len() || mode_rows.len() != resources.len() {
        return Err(FlowCheckpointError::InvalidState(
            "queue-order modes do not cover live resources",
        ));
    }
    let mut modes = BTreeMap::new();
    for (resource, mode) in mode_rows {
        if !resources.contains(resource) || modes.insert(*resource, *mode).is_some() {
            return Err(FlowCheckpointError::InvalidState(
                "queue-order mode names an unknown or duplicate resource",
            ));
        }
    }
    if image.requests.iter().collect::<BTreeSet<_>>().len() != image.requests.len()
        || request_rows.len() != image.requests.len()
    {
        return Err(FlowCheckpointError::InvalidState(
            "request queue-order rows do not cover retained requests",
        ));
    }
    let request_ids: BTreeSet<_> = image.requests.iter().copied().collect();
    let mut ordered_rows = BTreeMap::new();
    for (request, ordering) in request_rows {
        if !request_ids.contains(request) || ordered_rows.insert(*request, ordering).is_some() {
            return Err(FlowCheckpointError::InvalidState(
                "queue-order row names an unknown or duplicate request",
            ));
        }
    }
    let mut request_values = BTreeMap::new();
    if let Some(request_store) = image.builtins.requests.as_ref() {
        if request_store.rows.len() != image.requests.len() {
            return Err(FlowCheckpointError::InvalidState(
                "request store does not match retained request identities",
            ));
        }
        for (entity, request) in &request_store.rows {
            let id = RequestId(*entity);
            if request_values.insert(id, request).is_some() {
                return Err(FlowCheckpointError::InvalidState(
                    "duplicate request component identity",
                ));
            }
        }
    }
    if request_values.keys().copied().collect::<BTreeSet<_>>() != request_ids {
        return Err(FlowCheckpointError::InvalidState(
            "request store does not match retained request identities",
        ));
    }

    let mut keys = BTreeMap::new();
    let mut removed_modes = BTreeMap::new();
    for (request_id, row) in request_rows {
        let request = request_values
            .get(request_id)
            .ok_or(FlowCheckpointError::InvalidState(
                "request queue-order row has no request component",
            ))?;
        let stable = matches!(row, FlowRequestQueueOrderV4::StableKey(_));
        match modes.get(&request.resource) {
            Some(FlowQueueOrdering::AdmissionOrder) if stable => {
                return Err(FlowCheckpointError::InvalidState(
                    "request queue-order tag disagrees with admission-order resource",
                ));
            }
            Some(FlowQueueOrdering::StableKey) if !stable => {
                return Err(FlowCheckpointError::InvalidState(
                    "request queue-order tag disagrees with stable-order resource",
                ));
            }
            Some(_) => {}
            None => {
                if !terminal(request.state) {
                    return Err(FlowCheckpointError::InvalidState(
                        "nonterminal request references a removed resource",
                    ));
                }
                if removed_modes
                    .insert(request.resource, stable)
                    .is_some_and(|previous| previous != stable)
                {
                    return Err(FlowCheckpointError::InvalidState(
                        "removed-resource queue-order group is inconsistent",
                    ));
                }
            }
        }
        if let FlowRequestQueueOrderV4::StableKey(bytes) = row {
            let key = FlowQueueOrderKey::new(bytes).map_err(|_| {
                FlowCheckpointError::InvalidState("stable queue-order key is invalid")
            })?;
            keys.insert(*request_id, key);
        }
    }
    Ok(ValidatedQueueOrderMetadata { modes, keys })
}

impl FlowRuntime {
    /// Capture a complete native V4 image, including stable queue-order metadata.
    #[doc(hidden)]
    pub fn capture_checkpoint_v4(
        &self,
        codecs: &FlowCheckpointCodecs,
        limits: FlowCheckpointLimits,
    ) -> Result<FlowCheckpointV4, FlowCheckpointError> {
        let metadata = capture_queue_order_metadata(self, limits)?;
        let base_limits = remaining_key_budget(limits, metadata.key_bytes)?;
        let base = self
            .capture_checkpoint_parts_with_queue_order(codecs, base_limits, false, true)?
            .into_v3()
            .0;
        Ok(FlowCheckpointV4::from_v3(
            base,
            metadata.modes,
            metadata.requests,
        ))
    }

    /// Restore a complete native V4 image, including stable queue-order metadata.
    #[doc(hidden)]
    pub fn restore_checkpoint_v4(
        image: FlowCheckpointV4,
        codecs: &FlowCheckpointCodecs,
        limits: FlowCheckpointLimits,
    ) -> Result<Self, FlowCheckpointError> {
        Self::restore_checkpoint_v4_with_rebind(image, codecs, limits).map(|(runtime, _)| runtime)
    }

    /// Restore privately and return the validated native V4 rebind view.
    #[doc(hidden)]
    pub fn restore_checkpoint_v4_with_rebind(
        image: FlowCheckpointV4,
        codecs: &FlowCheckpointCodecs,
        limits: FlowCheckpointLimits,
    ) -> Result<(Self, FlowCheckpointRebindV1), FlowCheckpointError> {
        if image.version != FLOW_CHECKPOINT_VERSION_V4 {
            return Err(FlowCheckpointError::UnsupportedVersion(image.version));
        }
        if image.resources.len() > limits.max_resources
            || image.requests.len() > limits.max_requests
        {
            return Err(FlowCheckpointError::LimitExceeded(
                "queue-order identity rows",
            ));
        }
        if image.queue_order_modes.len() > limits.max_resources {
            return Err(FlowCheckpointError::LimitExceeded("queue order mode rows"));
        }
        let key_bytes =
            preflight_request_key_rows(&image.request_queue_orders, limits.max_requests)?;
        if image
            .builtins
            .requests
            .as_ref()
            .is_some_and(|store| store.rows.len() > limits.max_requests)
        {
            return Err(FlowCheckpointError::LimitExceeded("requests"));
        }
        let base_limits = remaining_key_budget(limits, key_bytes)?;
        let parts = image.into_parts();
        preflight_v3_normalization(&parts.base, base_limits)?;
        let metadata = validate_queue_order_metadata(&parts.base, &parts.modes, &parts.requests)?;
        Self::restore_checkpoint_parts(
            v2::FlowCheckpointParts::from_v3(parts.base),
            codecs,
            base_limits,
            Some(metadata),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static ENCODE_CALLS: AtomicUsize = AtomicUsize::new(0);

    fn encode_u64(value: &u64, remaining: usize) -> Result<Vec<u8>, FlowCheckpointCodecError> {
        ENCODE_CALLS.fetch_add(1, Ordering::SeqCst);
        if remaining < 8 {
            return Err(FlowCheckpointCodecError("payload limit".into()));
        }
        Ok(value.to_le_bytes().to_vec())
    }

    fn decode_u64(
        bytes: &[u8],
        _: &FlowCheckpointRebindV1,
    ) -> Result<u64, FlowCheckpointCodecError> {
        let bytes: [u8; 8] = bytes
            .try_into()
            .map_err(|_| FlowCheckpointCodecError("bad u64 payload".into()))?;
        Ok(u64::from_le_bytes(bytes))
    }

    #[test]
    fn v4_capture_corrupt_sidecar_fails_before_codec_without_mutation() {
        let mut runtime = FlowRuntime::new();
        let owner = runtime.spawn_actor().unwrap();
        let resource = runtime
            .create_resource_with_queue_ordering(
                1,
                FlowAllocationMode::Eager,
                FlowQueueOrdering::StableKey,
            )
            .unwrap();
        let work = runtime
            .create_work(owner, SimDuration::from_ticks(5), "ctx", 7_u64)
            .unwrap();
        let request = runtime
            .acquire(resource)
            .owner(owner)
            .timed_work(work)
            .queue_order_key(FlowQueueOrderKey::new(b"active").unwrap())
            .submit()
            .unwrap();
        runtime.step().unwrap().unwrap();

        let mut codecs = FlowCheckpointCodecs::new();
        codecs
            .register_context("ctx", 1, encode_u64, decode_u64)
            .unwrap();
        ENCODE_CALLS.store(0, Ordering::SeqCst);
        runtime
            .capture_checkpoint_v4(&codecs, FlowCheckpointLimits::default())
            .unwrap();
        assert!(ENCODE_CALLS.load(Ordering::SeqCst) > 0);

        let index_before = runtime.queue_order_indexes[&resource].clone();
        let ordering_before = runtime.queue_orderings.clone();
        let keys_before = runtime.queue_order_keys.clone();
        let request_before = runtime.request(request).unwrap();
        let queue_before = runtime
            .registry
            .get::<ClaimQueue>(resource.0)
            .unwrap()
            .clone();
        let scheduler_before = runtime
            .scheduler
            .checkpoint_state(SchedulerCheckpointLimits {
                max_entries: usize::MAX,
            })
            .unwrap();
        let world_before = runtime.world.snapshot();
        let counters_before = (
            runtime.created,
            runtime.destroyed,
            runtime.scheduled,
            runtime.next_admission,
            runtime.next_lease,
        );
        runtime
            .queue_order_indexes
            .get_mut(&resource)
            .unwrap()
            .queue_expected_len += 1;
        let corrupted_index = runtime.queue_order_indexes[&resource].clone();
        assert_ne!(corrupted_index, index_before);
        let corrupted_stats = runtime.scheduler.stats();
        ENCODE_CALLS.store(0, Ordering::SeqCst);
        assert!(matches!(
            runtime.capture_checkpoint_v4(&codecs, FlowCheckpointLimits::default()),
            Err(FlowCheckpointError::InvalidState(_))
        ));
        assert_eq!(ENCODE_CALLS.load(Ordering::SeqCst), 0);
        assert_eq!(runtime.queue_order_indexes[&resource], corrupted_index);
        assert_eq!(runtime.queue_orderings, ordering_before);
        assert_eq!(runtime.queue_order_keys, keys_before);
        assert_eq!(runtime.request(request).unwrap(), request_before);
        assert_eq!(
            runtime.registry.get::<ClaimQueue>(resource.0),
            Some(&queue_before)
        );
        assert_eq!(runtime.scheduler.stats(), corrupted_stats);
        assert_eq!(runtime.world.snapshot(), world_before);
        assert_eq!(
            (
                runtime.created,
                runtime.destroyed,
                runtime.scheduled,
                runtime.next_admission,
                runtime.next_lease,
            ),
            counters_before
        );
        assert_eq!(
            runtime
                .scheduler
                .checkpoint_state(SchedulerCheckpointLimits {
                    max_entries: usize::MAX,
                })
                .unwrap(),
            scheduler_before
        );
    }

    #[test]
    fn v4_checked_key_byte_sum_rejects_overflow() {
        assert_eq!(
            checked_key_byte_sum(usize::MAX, 1),
            Err(FlowCheckpointError::LimitExceeded("key bytes"))
        );
        assert_eq!(checked_key_byte_sum(usize::MAX - 1, 1), Ok(usize::MAX));
    }
}
