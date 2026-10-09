//! Private C2 composite checkpoint transaction.
//!
//! The artifact carries bytes only. Expected model/configuration identities,
//! codecs, stream keys, and route graphs come from the current trusted caller.

use crate::checkpoint_envelope::{
    self, CheckpointBindingV1, CheckpointEnvelopeError, CheckpointEnvelopeLimits,
};
use crate::checkpoint_sections::{
    C2BodyPartsV1, C2RecordBytesV1, C2SectionDirectoryError, C2SectionDirectoryLimitsV1,
    C2SectionDirectoryViewV1,
};
use crate::flow_bridge::checkpoint_wire::{validate_coherent_cut, BridgeWireLimits};
use crate::flow_bridge::{
    BoundIntrinsicWork, BoundIntrinsicWorkCheckpointV1, BridgeCheckpointError,
    SubmittedIntrinsicWork, SubmittedIntrinsicWorkCheckpointV1,
};
use crate::seed_map::checkpoint_wire::{
    contains_registered_key, decode_seed_map, encode_seed_map, SeedWireError, SeedWireLimits,
};
use crate::seed_map::{
    CalibrationSeedMap, CalibrationStreamKey, SeedIdentity, SeedPurpose,
    SeedRegistryCheckpointLimits,
};
use crate::work_duration::{
    IntrinsicWorkProvider, IntrinsicWorkProviderCheckpointError, IntrinsicWorkProviderCheckpointV1,
    IntrinsicWorkProviderWireError, IntrinsicWorkProviderWireLimits,
};
use kairo_ecs_abm::spatial::TransitGraphV1;
use kairo_ecs_des::fidelity::{
    FidelityAdapter, FidelityAdapterCheckpointV1, FidelityCheckpointError,
    FidelityCheckpointWireLimits,
};
use kairo_ecs_des::{
    FlowCheckpointCodecs, FlowCheckpointV1, FlowCheckpointWireLimits, FlowRuntime,
};
use kairo_ecs_types::EntityId;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[doc(hidden)]
pub(crate) struct C2PortableCheckpointLimitsV1 {
    pub(crate) envelope: CheckpointEnvelopeLimits,
    pub(crate) sections: C2SectionDirectoryLimitsV1,
    pub(crate) flow: FlowCheckpointWireLimits,
    pub(crate) fidelity: FidelityCheckpointWireLimits,
    pub(crate) provider: IntrinsicWorkProviderWireLimits,
    pub(crate) seed: SeedWireLimits,
    pub(crate) bridge: BridgeWireLimits,
}

#[derive(Debug, Error)]
#[doc(hidden)]
pub(crate) enum C2PortableCheckpointError {
    #[error("C2 composite owner checkpoint is invalid")]
    InvalidState,
    #[error("C2 composite owner checkpoint exceeds configured limits")]
    LimitExceeded,
    #[error("C2 composite owner checkpoint allocation failed")]
    Allocation,
    #[error("C2 composite envelope failed: {0}")]
    Envelope(#[from] CheckpointEnvelopeError),
    #[error("C2 composite section directory failed: {0}")]
    Sections(#[from] C2SectionDirectoryError),
    #[error("C2 seed registry failed: {0}")]
    Seed(#[from] SeedWireError),
    #[error("Flow checkpoint failed: {0}")]
    Flow(#[from] kairo_ecs_des::FlowCheckpointError),
    #[error("Flow checkpoint wire failed: {0}")]
    FlowWire(#[from] kairo_ecs_des::FlowCheckpointWireError),
    #[error("fidelity checkpoint failed: {0}")]
    Fidelity(#[from] FidelityCheckpointError),
    #[error("fidelity checkpoint wire failed: {0}")]
    FidelityWire(#[from] kairo_ecs_des::fidelity::FidelityCheckpointWireError),
    #[error("provider checkpoint wire failed: {0}")]
    Provider(#[from] IntrinsicWorkProviderWireError),
    #[error("provider checkpoint validation failed: {0}")]
    ProviderNative(#[from] IntrinsicWorkProviderCheckpointError),
    #[error("bridge checkpoint failed: {0:?}")]
    Bridge(BridgeCheckpointError),
}

/// Trusted per-work binding selected by current model code, never by artifact bytes.
#[derive(Clone)]
#[doc(hidden)]
pub(crate) struct TrustedC2WorkBindingV1 {
    pub(crate) service_key: CalibrationStreamKey,
    pub(crate) graph: Option<Arc<TransitGraphV1>>,
}

/// Resolve the trusted key and, for routed work, caller-approved graph for an
/// exact saved full work ID and stream identity. The callback must not treat
/// those saved values as authorization; it binds them against current model
/// configuration and the supplied runtime.
pub(crate) type TrustedC2BindingResolver<'a> = dyn FnMut(
        EntityId,
        &SeedIdentity,
        &FlowRuntime,
    ) -> Result<TrustedC2WorkBindingV1, C2PortableCheckpointError>
    + 'a;

pub(crate) enum RestoredBridgeRecord<T: Clone, C: 'static> {
    Bound(Box<BoundIntrinsicWork<T, C>>),
    Submitted(Box<SubmittedIntrinsicWork<T, C>>),
}

impl<T: Clone + 'static, C: 'static> RestoredBridgeRecord<T, C> {
    fn work_entity_id(&self) -> EntityId {
        match self {
            Self::Bound(record) => record.work().entity_id(),
            Self::Submitted(record) => record.work().entity_id(),
        }
    }
}

/// A complete staged restore. Constructed only after every owner and cross-link
/// has validated; the artifact itself is never returned as a runnable state.
pub(crate) struct RestoredC2<T: Clone, C: 'static> {
    pub(crate) flow: FlowRuntime,
    pub(crate) adapter: FidelityAdapter,
    pub(crate) provider: IntrinsicWorkProvider,
    pub(crate) seed_registry: CalibrationSeedMap,
    pub(crate) records: BTreeMap<EntityId, RestoredBridgeRecord<T, C>>,
}

/// Complete byte-backed C2 state image; no runtime identity, executable codec,
/// graph pointer, or self-selected trusted key is stored in the bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
#[doc(hidden)]
pub(crate) struct C2PortableCheckpointV1 {
    bytes: Vec<u8>,
}

fn cap(value: usize, remaining: usize) -> usize {
    value.min(remaining)
}

fn body_owner_budget(
    limits: C2PortableCheckpointLimitsV1,
    records: usize,
) -> Result<usize, C2PortableCheckpointError> {
    // The directory has a 14-byte header, four 9-byte singleton headers, and
    // 21 bytes of framing per work record. Reserve this before producing any
    // owned owner payload buffers.
    let framing = 14usize
        .checked_add(4 * 9)
        .and_then(|n| records.checked_mul(21).and_then(|r| n.checked_add(r)))
        .ok_or(C2PortableCheckpointError::LimitExceeded)?;
    let file_body_cap = limits
        .envelope
        .max_file_bytes
        .checked_sub(148)
        .ok_or(C2PortableCheckpointError::LimitExceeded)?;
    let body_cap = limits
        .sections
        .max_wire_bytes
        .min(limits.envelope.max_body_bytes)
        .min(file_body_cap);
    let payload_cap = limits.sections.max_payload_bytes.min(body_cap);
    payload_cap
        .checked_sub(framing)
        .ok_or(C2PortableCheckpointError::LimitExceeded)
}

fn take_payload_budget(
    remaining: &mut usize,
    bytes: &[u8],
) -> Result<(), C2PortableCheckpointError> {
    *remaining = remaining
        .checked_sub(bytes.len())
        .ok_or(C2PortableCheckpointError::LimitExceeded)?;
    Ok(())
}

fn flow_limits_with_budget(
    mut limits: FlowCheckpointWireLimits,
    remaining: usize,
) -> FlowCheckpointWireLimits {
    let native = &mut limits.flow;
    native.max_entities = cap(native.max_entities, remaining);
    native.max_scheduler_entries = cap(native.max_scheduler_entries, remaining);
    native.max_resources = cap(native.max_resources, remaining);
    native.max_requests = cap(native.max_requests, remaining);
    native.max_actors = cap(native.max_actors, remaining);
    native.max_works = cap(native.max_works, remaining);
    native.max_component_rows = cap(native.max_component_rows, remaining);
    native.max_commands = cap(native.max_commands, remaining);
    native.max_notifications = cap(native.max_notifications, remaining);
    native.max_pending_operations = cap(native.max_pending_operations, remaining);
    native.max_registrations = cap(native.max_registrations, remaining);
    native.max_key_bytes = cap(native.max_key_bytes, remaining);
    native.max_payload_bytes = cap(native.max_payload_bytes, remaining);
    limits.max_wire_bytes = cap(limits.max_wire_bytes, remaining);
    limits.max_total_records = cap(limits.max_total_records, remaining);
    limits
}

fn fidelity_limits_with_budget(
    mut limits: FidelityCheckpointWireLimits,
    remaining: usize,
) -> FidelityCheckpointWireLimits {
    limits.checkpoint.max_admitted = cap(limits.checkpoint.max_admitted, remaining);
    limits.checkpoint.max_overrides = cap(limits.checkpoint.max_overrides, remaining);
    limits.checkpoint.max_subsystem_bytes = cap(limits.checkpoint.max_subsystem_bytes, remaining);
    limits.max_wire_bytes = cap(limits.max_wire_bytes, remaining);
    limits.max_total_records = cap(limits.max_total_records, remaining);
    limits
}

fn bridge_limits_with_budget(mut limits: BridgeWireLimits, remaining: usize) -> BridgeWireLimits {
    let native = &mut limits.native;
    native.max_identifier_bytes = cap(native.max_identifier_bytes, remaining);
    native.max_owned_events = cap(native.max_owned_events, remaining);
    native.max_controls = cap(native.max_controls, remaining);
    native.max_dispatch_records = cap(native.max_dispatch_records, remaining);
    native.max_dispatch_batches = cap(native.max_dispatch_batches, remaining);
    native.max_dispatch_admissions = cap(native.max_dispatch_admissions, remaining);
    native.max_route_segments = cap(native.max_route_segments, remaining);
    native.max_canonical_bytes = cap(native.max_canonical_bytes, remaining);
    limits.max_wire_bytes = cap(limits.max_wire_bytes, remaining);
    limits.seed.max_entries = cap(limits.seed.max_entries, remaining);
    limits.seed.max_identifier_bytes = cap(limits.seed.max_identifier_bytes, remaining);
    limits.seed.max_wire_bytes = cap(limits.seed.max_wire_bytes, remaining);
    limits.flow = flow_limits_with_budget(limits.flow, remaining);
    limits
}

impl C2PortableCheckpointV1 {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn capture<T: Clone + 'static, C: 'static>(
        flow: &FlowRuntime,
        adapter: &FidelityAdapter,
        provider: &IntrinsicWorkProvider,
        seed_registry: &CalibrationSeedMap,
        bound: &[BoundIntrinsicWork<T, C>],
        submitted: &[SubmittedIntrinsicWork<T, C>],
        codecs: &FlowCheckpointCodecs,
        resolve_binding: &mut TrustedC2BindingResolver<'_>,
        binding: CheckpointBindingV1,
        limits: C2PortableCheckpointLimitsV1,
    ) -> Result<Self, C2PortableCheckpointError> {
        let record_count = bound
            .len()
            .checked_add(submitted.len())
            .ok_or(C2PortableCheckpointError::LimitExceeded)?;
        if bound.len() > limits.sections.max_bound_records
            || submitted.len() > limits.sections.max_submitted_records
            || record_count
                .checked_add(4)
                .is_none_or(|n| n > limits.sections.max_sections)
        {
            return Err(C2PortableCheckpointError::LimitExceeded);
        }
        let mut remaining = body_owner_budget(limits, record_count)?;
        let mut source_record_ids = Vec::new();
        source_record_ids
            .try_reserve_exact(record_count)
            .map_err(|_| C2PortableCheckpointError::Allocation)?;
        source_record_ids.extend(bound.iter().map(|record| record.work().entity_id()));
        source_record_ids.extend(submitted.iter().map(|record| record.work().entity_id()));
        source_record_ids.sort_unstable();
        if source_record_ids.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(C2PortableCheckpointError::InvalidState);
        }
        let flow_wire = flow_limits_with_budget(limits.flow, remaining);
        let flow_image = flow.capture_checkpoint(codecs, flow_wire.flow)?;
        let flow_bytes = flow_image.encode_wire_v1(flow_wire)?;
        take_payload_budget(&mut remaining, &flow_bytes)?;

        let fidelity_wire = fidelity_limits_with_budget(limits.fidelity, remaining);
        let fidelity_image = adapter.checkpoint(flow, fidelity_wire.checkpoint)?;
        let admitted_ids: Vec<_> = fidelity_image
            .admitted
            .iter()
            .map(|(work, _)| work.entity_id())
            .collect();
        if admitted_ids != source_record_ids {
            return Err(C2PortableCheckpointError::InvalidState);
        }
        let fidelity_bytes = fidelity_image.encode_wire_v1(fidelity_wire)?;
        take_payload_budget(&mut remaining, &fidelity_bytes)?;

        let mut provider_wire = limits.provider;
        provider_wire.provider.max_strata = cap(provider_wire.provider.max_strata, remaining);
        provider_wire.provider.max_total_support =
            cap(provider_wire.provider.max_total_support, remaining);
        provider_wire.provider.max_identifier_bytes =
            cap(provider_wire.provider.max_identifier_bytes, remaining);
        provider_wire.max_wire_bytes = cap(provider_wire.max_wire_bytes, remaining);
        let provider_bytes = provider.checkpoint_wire_v1(provider_wire)?;
        take_payload_budget(&mut remaining, &provider_bytes)?;

        let mut seed_wire = limits.seed;
        seed_wire.max_entries = cap(seed_wire.max_entries, remaining);
        seed_wire.max_identifier_bytes = cap(seed_wire.max_identifier_bytes, remaining);
        seed_wire.max_wire_bytes = cap(seed_wire.max_wire_bytes, remaining);
        let seed_bytes = encode_seed_map(seed_registry, seed_wire)?;
        take_payload_budget(&mut remaining, &seed_bytes)?;

        let mut bound_records = Vec::new();
        bound_records
            .try_reserve_exact(bound.len())
            .map_err(|_| C2PortableCheckpointError::Allocation)?;
        for record in bound {
            let bridge_wire = bridge_limits_with_budget(limits.bridge, remaining);
            let image =
                BoundIntrinsicWorkCheckpointV1::capture(record, flow, adapter, bridge_wire.native)
                    .map_err(C2PortableCheckpointError::Bridge)?;
            let work = image.work_entity_id();
            let expected = resolve_binding(work, image.stream_identity(), flow)?;
            if image.stream_identity().purpose != SeedPurpose::Service
                || !expected
                    .service_key
                    .matches_identity(image.stream_identity())
                || !contains_registered_key(seed_registry, &expected.service_key)?
            {
                return Err(C2PortableCheckpointError::InvalidState);
            }
            image
                .validate_trusted_binding(&expected.service_key, expected.graph.as_deref())
                .map_err(C2PortableCheckpointError::Bridge)?;
            let encoded = image
                .encode_wire_v1(bridge_wire)
                .map_err(|_| C2PortableCheckpointError::InvalidState)?;
            take_payload_budget(&mut remaining, &encoded)?;
            bound_records.push(C2RecordBytesV1::new(work, encoded));
            validate_coherent_cut(&flow_image, flow, record, bridge_wire.native)
                .map_err(C2PortableCheckpointError::Bridge)?;
        }

        let mut submitted_records = Vec::new();
        submitted_records
            .try_reserve_exact(submitted.len())
            .map_err(|_| C2PortableCheckpointError::Allocation)?;
        for record in submitted {
            let bridge_wire = bridge_limits_with_budget(limits.bridge, remaining);
            let image = SubmittedIntrinsicWorkCheckpointV1::capture(
                record,
                flow,
                adapter,
                bridge_wire.native,
            )
            .map_err(C2PortableCheckpointError::Bridge)?;
            let work = image.work_entity_id();
            let expected = resolve_binding(work, image.stream_identity(), flow)?;
            if image.stream_identity().purpose != SeedPurpose::Service
                || !expected
                    .service_key
                    .matches_identity(image.stream_identity())
                || !contains_registered_key(seed_registry, &expected.service_key)?
            {
                return Err(C2PortableCheckpointError::InvalidState);
            }
            image
                .validate_trusted_binding(&expected.service_key)
                .map_err(C2PortableCheckpointError::Bridge)?;
            let encoded = image
                .encode_wire_v1(bridge_wire)
                .map_err(|_| C2PortableCheckpointError::InvalidState)?;
            take_payload_budget(&mut remaining, &encoded)?;
            submitted_records.push(C2RecordBytesV1::new(work, encoded));
        }
        bound_records.sort_by_key(|record| record.source_work);
        submitted_records.sort_by_key(|record| record.source_work);
        let body = C2BodyPartsV1 {
            flow: &flow_bytes,
            fidelity: &fidelity_bytes,
            provider: &provider_bytes,
            seed_registry: &seed_bytes,
            bound: &bound_records,
            submitted: &submitted_records,
        }
        .encode(limits.sections)?;
        let bytes = checkpoint_envelope::encode(&body, binding, limits.envelope)?;
        Ok(Self { bytes })
    }

    pub(crate) fn from_bytes(
        bytes: Vec<u8>,
        binding: CheckpointBindingV1,
        limits: C2PortableCheckpointLimitsV1,
    ) -> Result<Self, C2PortableCheckpointError> {
        // The full envelope is verified before any owner decoder runs.
        let _body = checkpoint_envelope::decode(&bytes, binding, limits.envelope)?;
        Ok(Self { bytes })
    }

    pub(crate) fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub(crate) fn save_no_clobber(
        &self,
        target: &Path,
        binding: CheckpointBindingV1,
        limits: C2PortableCheckpointLimitsV1,
    ) -> Result<crate::checkpoint_envelope::CheckpointSaveOutcome, C2PortableCheckpointError> {
        let body = checkpoint_envelope::decode(&self.bytes, binding, limits.envelope)?;
        Ok(checkpoint_envelope::write_file_no_clobber(
            target,
            &body,
            binding,
            limits.envelope,
        )?)
    }

    pub(crate) fn read_file(
        source: &Path,
        binding: CheckpointBindingV1,
        limits: C2PortableCheckpointLimitsV1,
    ) -> Result<Self, C2PortableCheckpointError> {
        let body = checkpoint_envelope::read_file(source, binding, limits.envelope)?;
        let bytes = checkpoint_envelope::encode(&body, binding, limits.envelope)?;
        Ok(Self { bytes })
    }

    pub(crate) fn restore<T: Clone + 'static, C: 'static>(
        self,
        codecs: &FlowCheckpointCodecs,
        resolve_binding: &mut TrustedC2BindingResolver<'_>,
        binding: CheckpointBindingV1,
        limits: C2PortableCheckpointLimitsV1,
    ) -> Result<RestoredC2<T, C>, C2PortableCheckpointError> {
        let body = checkpoint_envelope::decode(&self.bytes, binding, limits.envelope)?;
        let sections = C2SectionDirectoryViewV1::parse(&body, limits.sections)?;

        // Owner wire formats preflight their complete bounded payloads before
        // constructing the staged Flow or invoking caller codecs. Each decoded
        // image remains detached until all cross-owner checks have succeeded.
        let flow_image = FlowCheckpointV1::decode_wire_v1(sections.flow(), limits.flow)?;
        let provider_image = IntrinsicWorkProviderCheckpointV1::decode_wire_v1(
            sections.provider(),
            limits.provider,
        )?;
        let seed_state = decode_seed_map(sections.seed_registry(), limits.seed)?;
        let mut bound_images = Vec::new();
        bound_images
            .try_reserve_exact(sections.bound_count())
            .map_err(|_| C2PortableCheckpointError::Allocation)?;
        for row in sections.bound_records() {
            let image = BoundIntrinsicWorkCheckpointV1::decode_wire_v1(row.payload, limits.bridge)
                .map_err(|_| C2PortableCheckpointError::InvalidState)?;
            if image.work_entity_id() != row.source_work {
                return Err(C2PortableCheckpointError::InvalidState);
            }
            bound_images.push((row.source_work, image));
        }
        let mut submitted_images = Vec::new();
        submitted_images
            .try_reserve_exact(sections.submitted_count())
            .map_err(|_| C2PortableCheckpointError::Allocation)?;
        for row in sections.submitted_records() {
            let image =
                SubmittedIntrinsicWorkCheckpointV1::decode_wire_v1(row.payload, limits.bridge)
                    .map_err(|_| C2PortableCheckpointError::InvalidState)?;
            if image.work_entity_id() != row.source_work {
                return Err(C2PortableCheckpointError::InvalidState);
            }
            submitted_images.push((row.source_work, image));
        }
        let (flow, rebind) = FlowRuntime::restore_checkpoint_with_rebind(
            flow_image.clone(),
            codecs,
            limits.flow.flow,
        )?;
        let fidelity_image = FidelityAdapterCheckpointV1::decode_wire_v1(
            sections.fidelity(),
            &rebind,
            limits.fidelity,
        )?;
        let mut admitted_ids: Vec<_> = fidelity_image
            .admitted
            .iter()
            .map(|(work, _)| work.entity_id())
            .collect();
        admitted_ids.sort_unstable();
        let provider = provider_image.restore(limits.provider.provider)?;
        let seed_registry = CalibrationSeedMap::from_checkpoint_state(
            seed_state,
            SeedRegistryCheckpointLimits {
                max_entries: limits.seed.max_entries,
                max_identifier_bytes: limits.seed.max_identifier_bytes,
            },
        )
        .map_err(|_| C2PortableCheckpointError::InvalidState)?;

        let mut mapping = Vec::new();
        mapping
            .try_reserve_exact(fidelity_image.admitted.len())
            .map_err(|_| C2PortableCheckpointError::Allocation)?;
        for (old, _) in &fidelity_image.admitted {
            let new = rebind
                .resolve_work(old.entity_id())
                .map_err(|_| C2PortableCheckpointError::InvalidState)?;
            mapping.push((*old, new));
        }
        let adapter = FidelityAdapter::from_checkpoint(
            fidelity_image,
            &flow,
            &mapping,
            limits.fidelity.checkpoint,
        )?;

        let mut records = BTreeMap::new();
        let expected_records = sections
            .bound_count()
            .checked_add(sections.submitted_count())
            .ok_or(C2PortableCheckpointError::LimitExceeded)?;
        let mut seen = BTreeSet::new();
        for (source_work, image) in bound_images {
            if !seen.insert(source_work) {
                return Err(C2PortableCheckpointError::InvalidState);
            }
            let trusted = resolve_binding(source_work, image.stream_identity(), &flow)?;
            if image.stream_identity().purpose != SeedPurpose::Service
                || !trusted
                    .service_key
                    .matches_identity(image.stream_identity())
                || !contains_registered_key(&seed_registry, &trusted.service_key)?
            {
                return Err(C2PortableCheckpointError::InvalidState);
            }
            image
                .validate_trusted_binding(&trusted.service_key, trusted.graph.as_deref())
                .map_err(C2PortableCheckpointError::Bridge)?;
            let work = rebind
                .resolve_work(source_work)
                .map_err(|_| C2PortableCheckpointError::InvalidState)?;
            if adapter.decision(work).is_none() {
                return Err(C2PortableCheckpointError::InvalidState);
            }
            let restored = image
                .restore::<T, C>(
                    &flow,
                    &adapter,
                    &trusted.service_key,
                    &rebind,
                    trusted.graph,
                    limits.bridge.native,
                )
                .map_err(C2PortableCheckpointError::Bridge)?;
            validate_coherent_cut(&flow_image, &flow, &restored, limits.bridge.native)
                .map_err(C2PortableCheckpointError::Bridge)?;
            records.insert(source_work, RestoredBridgeRecord::Bound(Box::new(restored)));
        }
        for (source_work, image) in submitted_images {
            if !seen.insert(source_work) {
                return Err(C2PortableCheckpointError::InvalidState);
            }
            let trusted = resolve_binding(source_work, image.stream_identity(), &flow)?;
            if image.stream_identity().purpose != SeedPurpose::Service
                || !trusted
                    .service_key
                    .matches_identity(image.stream_identity())
                || !contains_registered_key(&seed_registry, &trusted.service_key)?
            {
                return Err(C2PortableCheckpointError::InvalidState);
            }
            image
                .validate_trusted_binding(&trusted.service_key)
                .map_err(C2PortableCheckpointError::Bridge)?;
            let restored = image
                .restore::<T, C>(
                    &flow,
                    &adapter,
                    &trusted.service_key,
                    &rebind,
                    limits.bridge.native,
                )
                .map_err(C2PortableCheckpointError::Bridge)?;
            records.insert(
                source_work,
                RestoredBridgeRecord::Submitted(Box::new(restored)),
            );
        }
        if records.len() != expected_records
            || records.keys().copied().ne(admitted_ids.iter().copied())
        {
            return Err(C2PortableCheckpointError::InvalidState);
        }
        // The actual owner decoder validates work-context/graph row links. This
        // check also ensures there are no DTO work references outside the Flow.
        for (work, record) in &records {
            if *work != record.work_entity_id() {
                return Err(C2PortableCheckpointError::InvalidState);
            }
            rebind
                .resolve_work(*work)
                .map_err(|_| C2PortableCheckpointError::InvalidState)?;
        }
        Ok(RestoredC2 {
            flow,
            adapter,
            provider,
            seed_registry,
            records,
        })
    }
}

#[cfg(test)]
#[path = "c2_portable_checkpoint"]
mod tests {
    use super::*;
    use crate::flow_bridge::{
        AcquireIntent, BridgeCheckpointLimits, PreparationIdentity, TransitRequest,
        WorkPreparationInput,
    };
    use crate::work_duration::{
        IntrinsicDurationDistribution, IntrinsicWorkProviderCheckpointLimits,
    };
    use kairo_ecs_abm::spatial::{EdgeId, MovementModeId, MovementProfile, NodeId, TransitEdge};
    use kairo_ecs_des::fidelity::{FidelityMode, FidelityPolicy};
    use kairo_ecs_des::{
        FlowCheckpointCodecError, FlowCheckpointLimits, FlowCheckpointRebindV1, FlowHandlerCodeIds,
        WorkHandlers,
    };
    use kairo_ecs_types::{EventKind, SimTime};

    fn limits() -> C2PortableCheckpointLimitsV1 {
        let seed = SeedWireLimits {
            max_entries: 128,
            max_identifier_bytes: 16 * 1024,
            max_wire_bytes: 128 * 1024,
        };
        let flow = FlowCheckpointWireLimits {
            flow: FlowCheckpointLimits::default(),
            max_wire_bytes: 2 * 1024 * 1024,
            max_total_records: 100_000,
        };
        let bridge_native = BridgeCheckpointLimits {
            max_identifier_bytes: 16 * 1024,
            max_owned_events: 1024,
            max_controls: 1024,
            max_dispatch_records: 1024,
            max_dispatch_batches: 1024,
            max_dispatch_admissions: 4096,
            max_route_segments: 1024,
            max_canonical_bytes: 128 * 1024,
        };
        C2PortableCheckpointLimitsV1 {
            envelope: CheckpointEnvelopeLimits {
                max_file_bytes: 4 * 1024 * 1024,
                max_body_bytes: 4 * 1024 * 1024 - 148,
            },
            sections: C2SectionDirectoryLimitsV1 {
                max_wire_bytes: 4 * 1024 * 1024 - 148,
                max_sections: 4096,
                max_bound_records: 1024,
                max_submitted_records: 1024,
                max_payload_bytes: 4 * 1024 * 1024 - 148,
            },
            flow,
            fidelity: FidelityCheckpointWireLimits::default(),
            provider: IntrinsicWorkProviderWireLimits {
                provider: IntrinsicWorkProviderCheckpointLimits {
                    max_strata: 1024,
                    max_total_support: 16_384,
                    max_identifier_bytes: 16 * 1024,
                },
                max_wire_bytes: 128 * 1024,
            },
            seed,
            bridge: BridgeWireLimits {
                native: bridge_native,
                max_wire_bytes: 512 * 1024,
                seed,
                flow,
            },
        }
    }

    fn binding() -> CheckpointBindingV1 {
        CheckpointBindingV1 {
            model_code: [0x11; 32],
            configuration: [0x22; 32],
            owner_schemas: [0x33; 32],
        }
    }

    fn provider() -> IntrinsicWorkProvider {
        IntrinsicWorkProvider::new(
            1,
            vec![(
                "base".to_owned(),
                IntrinsicDurationDistribution::fixed(17).unwrap(),
            )],
        )
        .unwrap()
    }

    fn make_context(template: &u32) -> u32 {
        *template
    }

    fn encode_u32(value: &u32, max_bytes: usize) -> Result<Vec<u8>, FlowCheckpointCodecError> {
        if max_bytes < 4 {
            return Err(FlowCheckpointCodecError("context byte cap".to_owned()));
        }
        Ok(value.to_le_bytes().to_vec())
    }

    fn decode_u32(
        bytes: &[u8],
        _: &FlowCheckpointRebindV1,
    ) -> Result<u32, FlowCheckpointCodecError> {
        let raw: [u8; 4] = bytes
            .try_into()
            .map_err(|_| FlowCheckpointCodecError("invalid test context".to_owned()))?;
        Ok(u32::from_le_bytes(raw))
    }

    fn codecs() -> FlowCheckpointCodecs {
        let mut codecs = FlowCheckpointCodecs::new();
        codecs
            .register_context::<u32>("c2.context", 1, encode_u32, decode_u32)
            .unwrap();
        codecs
            .register_work_handlers::<u32>(
                "c2.context",
                FlowHandlerCodeIds::default(),
                WorkHandlers::default(),
            )
            .unwrap();
        codecs
            .register_restart_template::<u32, u32>("c2.template", 1, encode_u32, decode_u32)
            .unwrap();
        codecs
            .register_restart_factory::<u32, u32>("c2.template", "c2.factory", make_context)
            .unwrap();
        codecs
    }

    fn bound_source() -> (
        FlowRuntime,
        FidelityAdapter,
        CalibrationSeedMap,
        CalibrationStreamKey,
        BoundIntrinsicWork<u32, u32>,
    ) {
        bound_source_with_graph(None)
    }

    fn bound_source_with_graph(
        graph: Option<Arc<TransitGraphV1>>,
    ) -> (
        FlowRuntime,
        FidelityAdapter,
        CalibrationSeedMap,
        CalibrationStreamKey,
        BoundIntrinsicWork<u32, u32>,
    ) {
        let mut flow = FlowRuntime::new();
        let owner = flow.spawn_actor().unwrap();
        let carrier_actor = flow.spawn_actor().unwrap();
        let resource = flow.create_resource(1).unwrap();
        flow.register_work_handlers("c2.context", WorkHandlers::<u32>::default())
            .unwrap();
        let mut seed_registry = CalibrationSeedMap::new(1, "composite-test", 44).unwrap();
        let stream = seed_registry
            .stream_for("schedule", 2, "case", "task", SeedPurpose::Service)
            .unwrap();
        let key = stream.key().clone();
        let input = WorkPreparationInput::new(
            PreparationIdentity {
                owner,
                subsystem: "assessment".to_owned(),
                registration: "c2.context".to_owned(),
                stratum: "base".to_owned(),
            },
            stream,
            key.clone(),
            123_u32,
            make_context,
            AcquireIntent {
                resource,
                owner,
                at: SimTime::from_ticks(0),
                priority_level: 3,
                deadline: None,
                scheduler_priority: 0,
                can_preempt: false,
                preemptible: None,
            },
            match graph {
                Some(graph) => TransitRequest::Route {
                    graph,
                    origin: NodeId::new(1),
                    destination: NodeId::new(2),
                    profile: MovementProfile::new("walk", 1).unwrap(),
                    ticks_per_second: 1,
                    carrier_actor,
                    carrier_registration: "c2.transit".to_owned(),
                    kind: EventKind::custom(0xC20),
                },
                None => TransitRequest::Zero,
            },
        );
        let mut adapter =
            FidelityAdapter::new(FidelityPolicy::new(1, Some(FidelityMode::Macro)).unwrap());
        let prepared = input
            .prepare(&flow, &mut adapter, &provider())
            .ok()
            .unwrap();
        let created = prepared.create(&mut flow).ok().unwrap();
        let bound = created.bind(&flow).ok().unwrap();
        (flow, adapter, seed_registry, key, bound)
    }

    #[test]
    fn empty_roster_roundtrips_actual_owner_bytes_and_trusted_binding() {
        let flow = FlowRuntime::new();
        let adapter =
            FidelityAdapter::new(FidelityPolicy::new(1, Some(FidelityMode::Micro)).unwrap());
        let provider = provider();
        let seed_registry = CalibrationSeedMap::new(1, "composite-test", 44).unwrap();
        let codecs = FlowCheckpointCodecs::new();
        let limits = limits();
        let mut resolver = |_: EntityId,
                            _: &SeedIdentity,
                            _: &FlowRuntime|
         -> Result<TrustedC2WorkBindingV1, C2PortableCheckpointError> {
            Err(C2PortableCheckpointError::InvalidState)
        };

        let captured = C2PortableCheckpointV1::capture::<(), ()>(
            &flow,
            &adapter,
            &provider,
            &seed_registry,
            &[],
            &[],
            &codecs,
            &mut resolver,
            binding(),
            limits,
        )
        .unwrap();
        let bytes = captured.as_bytes().to_vec();
        let loaded = C2PortableCheckpointV1::from_bytes(bytes.clone(), binding(), limits).unwrap();
        let restored = loaded
            .restore::<(), ()>(&codecs, &mut resolver, binding(), limits)
            .unwrap();
        assert!(restored.records.is_empty());

        let recaptured = C2PortableCheckpointV1::capture::<(), ()>(
            &restored.flow,
            &restored.adapter,
            &restored.provider,
            &restored.seed_registry,
            &[],
            &[],
            &codecs,
            &mut resolver,
            binding(),
            limits,
        )
        .unwrap();
        assert_eq!(recaptured.as_bytes(), bytes);

        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "kairos-c2-composite-{}-{stamp}.bin",
            std::process::id()
        ));
        let _outcome = captured.save_no_clobber(&path, binding(), limits).unwrap();
        let from_disk = C2PortableCheckpointV1::read_file(&path, binding(), limits).unwrap();
        assert_eq!(from_disk.as_bytes(), bytes);
        assert!(matches!(
            captured.save_no_clobber(&path, binding(), limits),
            Err(C2PortableCheckpointError::Envelope(
                CheckpointEnvelopeError::TargetExists
            ))
        ));
        std::fs::remove_file(path).unwrap();

        let mut wrong_binding = binding();
        wrong_binding.configuration[0] ^= 0x01;
        assert!(matches!(
            C2PortableCheckpointV1::from_bytes(bytes.clone(), wrong_binding, limits),
            Err(C2PortableCheckpointError::Envelope(
                CheckpointEnvelopeError::BindingMismatch
            ))
        ));
        let mut corrupted = bytes;
        let last = corrupted.len() - 1;
        corrupted[last] ^= 0x80;
        assert!(C2PortableCheckpointV1::from_bytes(corrupted, binding(), limits).is_err());
    }

    #[test]
    fn bound_record_is_rebound_with_trusted_key_and_roster_is_complete() {
        let (flow, adapter, seed_registry, key, bound) = bound_source();
        let provider = provider();
        let codecs = codecs();
        let limits = limits();
        let binding = binding();
        let work = bound.work().entity_id();
        let key_for_restore = key.clone();
        let mut resolver =
            move |_: EntityId,
                  identity: &SeedIdentity,
                  _: &FlowRuntime|
                  -> Result<TrustedC2WorkBindingV1, C2PortableCheckpointError> {
                if !key_for_restore.matches_identity(identity) {
                    return Err(C2PortableCheckpointError::InvalidState);
                }
                Ok(TrustedC2WorkBindingV1 {
                    service_key: key_for_restore.clone(),
                    graph: None,
                })
            };
        let captured = C2PortableCheckpointV1::capture(
            &flow,
            &adapter,
            &provider,
            &seed_registry,
            std::slice::from_ref(&bound),
            &[],
            &codecs,
            &mut resolver,
            binding,
            limits,
        )
        .unwrap();
        let restored =
            C2PortableCheckpointV1::from_bytes(captured.as_bytes().to_vec(), binding, limits)
                .unwrap()
                .restore::<u32, u32>(&codecs, &mut resolver, binding, limits)
                .unwrap();
        assert_ne!(flow.identity(), restored.flow.identity());
        assert_eq!(restored.records.len(), 1);
        assert_eq!(restored.records.keys().copied().collect::<Vec<_>>(), [work]);
        assert!(matches!(
            restored.records.get(&work),
            Some(RestoredBridgeRecord::Bound(record)) if record.work().entity_id() == work
        ));

        let body =
            checkpoint_envelope::decode(captured.as_bytes(), binding, limits.envelope).unwrap();
        let parsed = C2SectionDirectoryViewV1::parse(&body, limits.sections).unwrap();
        let empty_roster = C2BodyPartsV1 {
            flow: parsed.flow(),
            fidelity: parsed.fidelity(),
            provider: parsed.provider(),
            seed_registry: parsed.seed_registry(),
            bound: &[],
            submitted: &[],
        }
        .encode(limits.sections)
        .unwrap();
        let valid_envelope =
            checkpoint_envelope::encode(&empty_roster, binding, limits.envelope).unwrap();
        let mut resolver =
            move |_: EntityId,
                  identity: &SeedIdentity,
                  _: &FlowRuntime|
                  -> Result<TrustedC2WorkBindingV1, C2PortableCheckpointError> {
                if !key.matches_identity(identity) {
                    return Err(C2PortableCheckpointError::InvalidState);
                }
                Ok(TrustedC2WorkBindingV1 {
                    service_key: key.clone(),
                    graph: None,
                })
            };
        assert!(
            C2PortableCheckpointV1::from_bytes(valid_envelope, binding, limits)
                .unwrap()
                .restore::<u32, u32>(&codecs, &mut resolver, binding, limits)
                .is_err()
        );

        let draw_position = bound.service_draw_position();
        assert!(C2PortableCheckpointV1::capture::<u32, u32>(
            &flow,
            &adapter,
            &provider,
            &seed_registry,
            &[],
            &[],
            &codecs,
            &mut resolver,
            binding,
            limits,
        )
        .is_err());
        assert_eq!(bound.service_draw_position(), draw_position);
        assert_eq!(adapter.decision(bound.work()), Some(&bound.decision()));
    }

    #[test]
    fn bound_route_requires_the_caller_approved_graph() {
        let mode = MovementModeId::new("walk").unwrap();
        let graph = Arc::new(
            TransitGraphV1::new(
                1,
                vec![NodeId::new(1), NodeId::new(2)],
                vec![TransitEdge {
                    id: EdgeId::new(1),
                    from: NodeId::new(1),
                    to: NodeId::new(2),
                    length_mm: 9,
                    allowed_modes: vec![mode.clone()],
                }],
            )
            .unwrap(),
        );
        let (flow, adapter, seed_registry, key, bound) =
            bound_source_with_graph(Some(graph.clone()));
        let provider = provider();
        let codecs = codecs();
        let limits = limits();
        let binding = binding();
        let trusted_graph = graph.clone();
        let key_for_wrong_graph = key.clone();
        let mut resolver =
            move |_: EntityId,
                  identity: &SeedIdentity,
                  _: &FlowRuntime|
                  -> Result<TrustedC2WorkBindingV1, C2PortableCheckpointError> {
                if !key.matches_identity(identity) {
                    return Err(C2PortableCheckpointError::InvalidState);
                }
                Ok(TrustedC2WorkBindingV1 {
                    service_key: key.clone(),
                    graph: Some(trusted_graph.clone()),
                })
            };
        let captured = C2PortableCheckpointV1::capture(
            &flow,
            &adapter,
            &provider,
            &seed_registry,
            std::slice::from_ref(&bound),
            &[],
            &codecs,
            &mut resolver,
            binding,
            limits,
        )
        .unwrap();
        let restored =
            C2PortableCheckpointV1::from_bytes(captured.as_bytes().to_vec(), binding, limits)
                .unwrap()
                .restore::<u32, u32>(&codecs, &mut resolver, binding, limits)
                .unwrap();
        assert_eq!(restored.records.len(), 1);
        // A different canonical graph with the same schema version must still
        // reject this artifact under the caller-trusted binding.
        let wrong_graph = Arc::new(
            TransitGraphV1::new(
                1,
                vec![NodeId::new(1), NodeId::new(2)],
                vec![TransitEdge {
                    id: EdgeId::new(1),
                    from: NodeId::new(1),
                    to: NodeId::new(2),
                    length_mm: 10,
                    allowed_modes: vec![mode],
                }],
            )
            .unwrap(),
        );
        let mut wrong_resolver =
            move |_: EntityId,
                  identity: &SeedIdentity,
                  _: &FlowRuntime|
                  -> Result<TrustedC2WorkBindingV1, C2PortableCheckpointError> {
                if !key_for_wrong_graph.matches_identity(identity) {
                    return Err(C2PortableCheckpointError::InvalidState);
                }
                Ok(TrustedC2WorkBindingV1 {
                    service_key: key_for_wrong_graph.clone(),
                    graph: Some(wrong_graph.clone()),
                })
            };
        assert!(
            C2PortableCheckpointV1::from_bytes(captured.as_bytes().to_vec(), binding, limits,)
                .unwrap()
                .restore::<u32, u32>(&codecs, &mut wrong_resolver, binding, limits)
                .is_err()
        );
    }

    #[test]
    fn sparse_slot_metadata_is_not_mistaken_for_wire_byte_budget() {
        let mut wire_limits = FlowCheckpointWireLimits {
            flow: FlowCheckpointLimits::default(),
            max_wire_bytes: 2048,
            max_total_records: 128,
        };
        wire_limits.flow.max_sparse_slots = 1_000_000;
        let bounded = flow_limits_with_budget(wire_limits, 128);
        assert_eq!(bounded.flow.max_sparse_slots, 1_000_000);
        assert_eq!(bounded.max_wire_bytes, 128);
        assert_eq!(bounded.flow.max_payload_bytes, 128);
    }

    #[cfg(test)]
    #[path = "process_tests.rs"]
    mod process_tests;

    #[cfg(test)]
    #[path = "generic_context_tests.rs"]
    mod generic_context_tests;
}
