//! Private, experimental calibration-to-Flow admission bridge.
//!
//! Intrinsic service sampling owns one Service stream. Nonzero Micro transit is
//! scheduled through the ABM TransitContext; Macro and zero routes submit work
//! without creating a transit carrier or event.

use crate::route_receipt::{
    RouteMetadata, RouteMetadataCheckpointV1, RouteReceipt, RouteReceiptCheckpointLimits,
    RouteReceiptCheckpointV1, RouteReceiptError,
};
use crate::seed_map::{
    CalibrationStream, CalibrationStreamKey, CalibrationStreamStateError,
    CalibrationStreamStateLimits, CalibrationStreamStateV1, SeedPurpose,
};
use crate::work_duration::{
    IntrinsicWorkProvider, SampledWorkDuration, SampledWorkDurationCheckpointError,
    WorkDurationError,
};
use kairo_ecs_abm::spatial::{MovementProfile, NodeId, TransitError, TransitGraphV1};
use kairo_ecs_abm::{
    schedule_transit_control, schedule_transit_start, TransitContext, TransitPhase,
};
use kairo_ecs_des::fidelity::{
    FidelityAdapter, FidelityAdmissionPermit, FidelityDecision, FidelityError, FidelityMode,
};
use kairo_ecs_des::{
    FlowAcquireCommand, FlowBatchReceipt, FlowCheckpointRebindV1, FlowDispatch, FlowDomainControl,
    FlowError, FlowRuntime, FlowRuntimeIdentity, LifecycleRecord, PreemptionStrategy, RequestId,
    ResourceId, WorkId, WorkState,
};
use kairo_ecs_types::{EntityId, EventId, EventKind, SimDuration, SimTime};
use std::marker::PhantomData;
use std::sync::Arc;

pub(crate) mod checkpoint_wire;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PreparationIdentity {
    pub(crate) owner: EntityId,
    pub(crate) subsystem: String,
    pub(crate) registration: String,
    pub(crate) stratum: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct AcquireIntent {
    pub(crate) resource: ResourceId,
    pub(crate) owner: EntityId,
    pub(crate) at: SimTime,
    pub(crate) priority_level: i32,
    pub(crate) deadline: Option<SimTime>,
    pub(crate) scheduler_priority: i32,
    pub(crate) can_preempt: bool,
    pub(crate) preemptible: Option<PreemptionStrategy>,
}

#[derive(Clone)]
pub(crate) enum TransitRequest {
    Zero,
    Route {
        graph: Arc<TransitGraphV1>,
        origin: NodeId,
        destination: NodeId,
        profile: MovementProfile,
        ticks_per_second: u64,
        carrier_actor: EntityId,
        carrier_registration: String,
        kind: EventKind,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TransitObservation {
    Progress,
    Paused,
    Resumed,
    IgnoredStale,
    Rejected,
    Arrived,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum BridgeError {
    Fidelity(FidelityError),
    Duration(WorkDurationError),
    Transit(TransitError),
    Flow(FlowError),
    ConflictingSubmission,
    InvalidDispatch,
    RouteReceipt(RouteReceiptError),
}

/// Caller-selected bounds for the private native bridge continuation packet.
/// This DTO is not a durable byte format or authentication claim.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct BridgeCheckpointLimits {
    pub(crate) max_identifier_bytes: usize,
    pub(crate) max_owned_events: usize,
    pub(crate) max_controls: usize,
    pub(crate) max_dispatch_records: usize,
    pub(crate) max_dispatch_batches: usize,
    pub(crate) max_dispatch_admissions: usize,
    pub(crate) max_route_segments: usize,
    pub(crate) max_canonical_bytes: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum BridgeCheckpointError {
    UnsupportedVersion,
    LimitExceeded,
    InvalidState,
    Stream(CalibrationStreamStateError),
    Sample(SampledWorkDurationCheckpointError),
    Route(RouteReceiptError),
    Flow(FlowError),
    Fidelity(FidelityError),
    Transit(TransitError),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct BoundIntrinsicWorkCheckpointV1 {
    version: u32,
    decision: FidelityDecision,
    stream: CalibrationStreamStateV1,
    duration_ticks: u128,
    draw_before: u64,
    draw_after: u64,
    acquire: AcquireIntentCheckpointV1,
    transit: TransitRequestCheckpointV1,
    route_metadata: Option<RouteMetadataCheckpointV1>,
    route_receipt: Option<RouteReceiptCheckpointV1>,
    work: EntityId,
    carrier: Option<EntityId>,
    carrier_actor: Option<EntityId>,
    kind: Option<EventKind>,
    pending_event: Option<EventId>,
    pending_priority: Option<i32>,
    owned_events: Vec<EventId>,
    stale_events: Vec<EventId>,
    consumed_events: Vec<EventId>,
    controls: Vec<(EventId, FlowDomainControl)>,
    retryable: Option<FlowDispatch>,
    arrival_request: Option<EntityId>,
    arrival_at: Option<SimTime>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SubmittedIntrinsicWorkCheckpointV1 {
    version: u32,
    decision: FidelityDecision,
    stream: CalibrationStreamStateV1,
    duration_ticks: u128,
    draw_before: u64,
    draw_after: u64,
    acquire: AcquireIntentCheckpointV1,
    work: EntityId,
    request: EntityId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct AcquireIntentCheckpointV1 {
    resource: EntityId,
    owner: EntityId,
    at: SimTime,
    priority_level: i32,
    deadline: Option<SimTime>,
    scheduler_priority: i32,
    can_preempt: bool,
    preemptible: Option<PreemptionStrategy>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum TransitRequestCheckpointV1 {
    Zero,
    Route {
        graph_version: u32,
        graph_canonical_bytes: Vec<u8>,
        origin: u64,
        destination: u64,
        mode: String,
        speed_mm_per_second: u64,
        ticks_per_second: u64,
        carrier_actor: EntityId,
        carrier_registration: String,
        kind: EventKind,
    },
}

impl BoundIntrinsicWorkCheckpointV1 {
    /// Borrow the exact retained rejection for trusted continuation coordination.
    /// This does not infer dispatch provenance or mutate the saved owner state.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "Consumed by complete C2 process restoration and retry coordination"
        )
    )]
    pub(crate) fn retryable_dispatch(&self) -> Option<&FlowDispatch> {
        self.retryable.as_ref()
    }

    /// Checks this record against caller-trusted stream and route bindings
    /// without constructing a route or allocating. A Zero transit record has
    /// no graph binding, so `trusted_graph` is unused for that variant.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "Consumed by the complete C2 composite trusted-binding preflight"
        )
    )]
    pub(crate) fn validate_trusted_binding(
        &self,
        expected_service_key: &CalibrationStreamKey,
        trusted_graph: Option<&TransitGraphV1>,
    ) -> Result<(), BridgeCheckpointError> {
        if self.stream.identity.purpose != SeedPurpose::Service
            || !expected_service_key.matches_identity(&self.stream.identity)
        {
            return Err(BridgeCheckpointError::InvalidState);
        }
        if let TransitRequestCheckpointV1::Route {
            graph_version,
            graph_canonical_bytes,
            ..
        } = &self.transit
        {
            let graph = trusted_graph.ok_or(BridgeCheckpointError::InvalidState)?;
            // TransitGraphV1 currently admits version 1 only. Its canonical
            // identity contains that version, and byte equality is borrowed.
            if *graph_version != 1 || graph.canonical_bytes_ref() != graph_canonical_bytes {
                return Err(BridgeCheckpointError::InvalidState);
            }
        }
        Ok(())
    }
}

impl SubmittedIntrinsicWorkCheckpointV1 {
    /// Checks the saved Service identity against a key supplied by the
    /// caller's trusted model configuration. This comparison is allocation-free.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "Consumed by the complete C2 composite trusted-binding preflight"
        )
    )]
    pub(crate) fn validate_trusted_binding(
        &self,
        expected_service_key: &CalibrationStreamKey,
    ) -> Result<(), BridgeCheckpointError> {
        if self.stream.identity.purpose != SeedPurpose::Service
            || !expected_service_key.matches_identity(&self.stream.identity)
        {
            return Err(BridgeCheckpointError::InvalidState);
        }
        Ok(())
    }
}

impl BoundIntrinsicWorkCheckpointV1 {
    pub(crate) fn capture<T: Clone + 'static, C: 'static>(
        bound: &BoundIntrinsicWork<T, C>,
        flow: &FlowRuntime,
        adapter: &FidelityAdapter,
        limits: BridgeCheckpointLimits,
    ) -> Result<Self, BridgeCheckpointError> {
        if bound.runtime != flow.identity()
            || adapter.decision(bound.work) != Some(&bound.decision)
            || bound.service_stream.purpose() != SeedPurpose::Service
            || bound.service_stream.key() != bound.expected_service_key
            || bound.sample.checkpoint_parts().1 != &bound.expected_service_key
            || bound.sample.draw_after() > bound.service_stream.draw_position()
            || bound.sample.draw_before() > bound.sample.draw_after()
        {
            return Err(BridgeCheckpointError::InvalidState);
        }
        if bound.owned_events.len() > limits.max_owned_events
            || bound.stale_events.len() > limits.max_owned_events
            || bound.consumed_events.len() > limits.max_owned_events
            || bound.controls.len() > limits.max_controls
        {
            return Err(BridgeCheckpointError::LimitExceeded);
        }
        if bound.arrival_request.is_some() != bound.arrival_at.is_some() {
            return Err(BridgeCheckpointError::InvalidState);
        }
        if bound.carrier.is_some() != bound.carrier_actor.is_some()
            || bound.carrier.is_some() != bound.kind.is_some()
            || bound.pending_event.is_some() != bound.pending_priority.is_some()
            || bound
                .pending_event
                .is_some_and(|event| !bound.owned_events.contains(&event))
        {
            return Err(BridgeCheckpointError::InvalidState);
        }
        let source_work = flow.work(bound.work).map_err(BridgeCheckpointError::Flow)?;
        if source_work.owner != bound.acquire.owner
            || source_work.original_duration != bound.sample.duration()
            || source_work.request.is_some() != bound.arrival_request.is_some()
        {
            return Err(BridgeCheckpointError::InvalidState);
        }

        let mut bridge_identifier_bytes = match &bound.transit {
            TransitRequest::Zero => 0,
            TransitRequest::Route {
                profile,
                carrier_registration,
                ..
            } => {
                checked_identifier_sum([profile.mode().as_str().len(), carrier_registration.len()])?
            }
        };
        if let Some(metadata) = &bound.route_metadata {
            bridge_identifier_bytes = bridge_identifier_bytes
                .checked_add(
                    metadata
                        .checkpoint_identifier_bytes()
                        .map_err(BridgeCheckpointError::Route)?,
                )
                .ok_or(BridgeCheckpointError::LimitExceeded)?;
        }
        if let Some(receipt) = &bound.route_receipt {
            bridge_identifier_bytes = bridge_identifier_bytes
                .checked_add(
                    receipt
                        .metadata_identifier_bytes()
                        .map_err(BridgeCheckpointError::Route)?,
                )
                .ok_or(BridgeCheckpointError::LimitExceeded)?;
        }
        if bridge_identifier_bytes > limits.max_identifier_bytes {
            return Err(BridgeCheckpointError::LimitExceeded);
        }
        let transit = TransitRequestCheckpointV1::capture(&bound.transit, limits)?;
        if let Some(carrier) = bound.carrier {
            let context = flow
                .work_context::<TransitContext>(carrier)
                .map_err(BridgeCheckpointError::Flow)?;
            validate_transit_context(
                &bound.transit,
                context,
                flow,
                carrier,
                bound.work,
                bound.carrier_actor,
                bound.kind,
            )?;
            bound.validate_route_context_bounded(flow, carrier, limits.max_canonical_bytes)?;
        }
        let stream_limits = CalibrationStreamStateLimits {
            max_identifier_bytes: limits.max_identifier_bytes - bridge_identifier_bytes,
        };
        let stream = bound
            .service_stream
            .checkpoint_state(stream_limits)
            .map_err(BridgeCheckpointError::Stream)?;
        let route_receipt = match &bound.route_receipt {
            Some(receipt) => {
                if bound.carrier.is_none() {
                    return Err(BridgeCheckpointError::InvalidState);
                }
                Some(
                    receipt
                        .checkpoint_v1(RouteReceiptCheckpointLimits {
                            max_identifier_bytes: limits.max_identifier_bytes
                                - bridge_identifier_bytes,
                            max_canonical_bytes: limits.max_canonical_bytes,
                        })
                        .map_err(BridgeCheckpointError::Route)?,
                )
            }
            None => None,
        };
        if let Some(dispatch) = &bound.retryable {
            validate_dispatch_limits(dispatch, limits)?;
            let (Some(carrier), Some(pending)) = (bound.carrier, bound.pending_event) else {
                return Err(BridgeCheckpointError::InvalidState);
            };
            if dispatch.event != pending
                || !matches!(
                    dispatch.callback_batches.as_slice(),
                    [FlowBatchReceipt::Rejected(_)]
                )
            {
                return Err(BridgeCheckpointError::InvalidState);
            }
            TransitContext::validate_retry_dispatch(flow, carrier, dispatch)
                .map_err(|_| BridgeCheckpointError::InvalidState)?;
        }
        let route_metadata = bound
            .route_metadata
            .as_ref()
            .map(|metadata| {
                metadata
                    .checkpoint_v1(RouteReceiptCheckpointLimits {
                        max_identifier_bytes: limits.max_identifier_bytes - bridge_identifier_bytes,
                        max_canonical_bytes: limits.max_canonical_bytes,
                    })
                    .map_err(BridgeCheckpointError::Route)
            })
            .transpose()?;
        let (duration, _, draw_before, draw_after) = bound.sample.checkpoint_parts();
        Ok(Self {
            version: 1,
            decision: bound.decision,
            stream,
            duration_ticks: duration.ticks(),
            draw_before,
            draw_after,
            acquire: AcquireIntentCheckpointV1::capture(&bound.acquire),
            transit,
            route_metadata,
            route_receipt,
            work: bound.work.entity_id(),
            carrier: bound.carrier.map(WorkId::entity_id),
            carrier_actor: bound.carrier_actor,
            kind: bound.kind,
            pending_event: bound.pending_event,
            pending_priority: bound.pending_priority,
            owned_events: bound.owned_events.clone(),
            stale_events: bound.stale_events.clone(),
            consumed_events: bound.consumed_events.clone(),
            controls: bound.controls.clone(),
            retryable: bound.retryable.clone(),
            arrival_request: bound.arrival_request.map(RequestId::entity_id),
            arrival_at: bound.arrival_at,
        })
    }

    pub(crate) fn restore<T: Clone + 'static, C: 'static>(
        self,
        flow: &FlowRuntime,
        adapter: &FidelityAdapter,
        expected_service_key: &CalibrationStreamKey,
        rebind: &FlowCheckpointRebindV1,
        trusted_graph: Option<Arc<TransitGraphV1>>,
        limits: BridgeCheckpointLimits,
    ) -> Result<BoundIntrinsicWork<T, C>, BridgeCheckpointError> {
        if self.version != 1 {
            return Err(BridgeCheckpointError::UnsupportedVersion);
        }
        validate_checkpoint_identifier_budget(&self, limits)?;
        validate_owned_vectors(
            &self.owned_events,
            &self.stale_events,
            &self.consumed_events,
            &self.controls,
            limits,
        )?;
        if flow.identity() != *rebind.identity()
            || self.duration_ticks == 0
            || self.draw_before > self.draw_after
            || self.arrival_request.is_some() != self.arrival_at.is_some()
        {
            return Err(BridgeCheckpointError::InvalidState);
        }
        let stream = self
            .stream
            .restore_for(
                expected_service_key,
                CalibrationStreamStateLimits {
                    max_identifier_bytes: limits.max_identifier_bytes,
                },
            )
            .map_err(BridgeCheckpointError::Stream)?;
        let work = rebind
            .resolve_work(self.work)
            .map_err(|_| BridgeCheckpointError::InvalidState)?;
        let spec = flow.work(work).map_err(BridgeCheckpointError::Flow)?;
        if adapter.decision(work) != Some(&self.decision) {
            return Err(BridgeCheckpointError::Fidelity(FidelityError::InvalidWork));
        }
        let acquire = self.acquire.restore(rebind)?;
        if acquire.owner != spec.owner
            || spec.original_duration.ticks() != self.duration_ticks
            || spec.request.is_some() != self.arrival_request.is_some()
            || self.carrier.is_some() != self.carrier_actor.is_some()
            || self.carrier.is_some() != self.kind.is_some()
            || self.pending_event.is_some() != self.pending_priority.is_some()
        {
            return Err(BridgeCheckpointError::InvalidState);
        }
        flow.resource(acquire.resource)
            .map_err(BridgeCheckpointError::Flow)?;
        let sample = SampledWorkDuration::from_checkpoint_parts(
            SimDuration::from_ticks(self.duration_ticks),
            expected_service_key.clone(),
            self.draw_before,
            self.draw_after,
            &stream,
            expected_service_key,
        )
        .map_err(BridgeCheckpointError::Sample)?;
        let transit = self.transit.restore(trusted_graph, limits)?;
        let carrier = self
            .carrier
            .map(|id| {
                rebind
                    .resolve_work(id)
                    .map_err(|_| BridgeCheckpointError::InvalidState)
            })
            .transpose()?;
        let carrier_actor = self
            .carrier_actor
            .map(|id| {
                rebind
                    .resolve_actor(id)
                    .map_err(|_| BridgeCheckpointError::InvalidState)
            })
            .transpose()?;
        if carrier.is_some() {
            let TransitRequest::Route {
                carrier_registration,
                carrier_actor: requested_actor,
                kind: requested_kind,
                ..
            } = &transit
            else {
                return Err(BridgeCheckpointError::InvalidState);
            };
            if carrier_actor != Some(*requested_actor) || self.kind != Some(*requested_kind) {
                return Err(BridgeCheckpointError::InvalidState);
            }
            let Some(carrier_id) = self.carrier else {
                return Err(BridgeCheckpointError::InvalidState);
            };
            let (owner, registration, domain_kind) = rebind
                .resolve_work_binding(carrier_id)
                .map_err(|_| BridgeCheckpointError::InvalidState)?;
            if owner != *requested_actor
                || registration != carrier_registration
                || domain_kind != Some(*requested_kind)
            {
                return Err(BridgeCheckpointError::InvalidState);
            }
        }
        let route_metadata = self
            .route_metadata
            .as_ref()
            .map(|metadata| {
                metadata
                    .restore(RouteReceiptCheckpointLimits {
                        max_identifier_bytes: limits.max_identifier_bytes,
                        max_canonical_bytes: limits.max_canonical_bytes,
                    })
                    .map_err(BridgeCheckpointError::Route)
            })
            .transpose()?;
        let route_receipt = match (self.route_receipt, carrier) {
            (Some(image), Some(carrier)) => {
                let context = flow
                    .work_context::<TransitContext>(carrier)
                    .map_err(BridgeCheckpointError::Flow)?;
                Some(
                    RouteReceipt::restore_checkpoint_v1(
                        image,
                        context,
                        RouteReceiptCheckpointLimits {
                            max_identifier_bytes: limits.max_identifier_bytes,
                            max_canonical_bytes: limits.max_canonical_bytes,
                        },
                    )
                    .map_err(BridgeCheckpointError::Route)?,
                )
            }
            (None, Some(_)) => None,
            (None, None) => None,
            _ => return Err(BridgeCheckpointError::InvalidState),
        };
        let pending_event = self
            .pending_event
            .map(|event| {
                let resolved = if self.retryable.is_some() {
                    rebind.resolve_issued_event(event)
                } else {
                    rebind.resolve_event(event)
                };
                resolved.map_err(|_| BridgeCheckpointError::InvalidState)
            })
            .transpose()?;
        let mut owned_events = Vec::new();
        for event in &self.owned_events {
            owned_events.push(
                rebind
                    .resolve_issued_event(*event)
                    .map_err(|_| BridgeCheckpointError::InvalidState)?,
            );
        }
        let mut stale_events = Vec::new();
        for event in &self.stale_events {
            stale_events.push(
                rebind
                    .resolve_issued_event(*event)
                    .map_err(|_| BridgeCheckpointError::InvalidState)?,
            );
        }
        let mut consumed_events = Vec::new();
        for event in &self.consumed_events {
            consumed_events.push(
                rebind
                    .resolve_issued_event(*event)
                    .map_err(|_| BridgeCheckpointError::InvalidState)?,
            );
        }
        let mut controls = Vec::new();
        for (event, action) in &self.controls {
            controls.push((
                rebind
                    .resolve_issued_event(*event)
                    .map_err(|_| BridgeCheckpointError::InvalidState)?,
                *action,
            ));
        }
        if let Some(event) = pending_event {
            if !owned_events.contains(&event) {
                return Err(BridgeCheckpointError::InvalidState);
            }
        }
        let retryable = self.retryable;
        if let Some(dispatch) = retryable.as_ref() {
            validate_dispatch_limits(dispatch, limits)?;
            validate_dispatch_references(dispatch, rebind)?;
            let (Some(carrier), Some(pending)) = (carrier, pending_event) else {
                return Err(BridgeCheckpointError::InvalidState);
            };
            if dispatch.event != pending
                || !matches!(
                    dispatch.callback_batches.as_slice(),
                    [FlowBatchReceipt::Rejected(_)]
                )
            {
                return Err(BridgeCheckpointError::InvalidState);
            }
            TransitContext::validate_retry_dispatch(flow, carrier, dispatch)
                .map_err(|_| BridgeCheckpointError::InvalidState)?;
        }
        if let Some(carrier) = carrier {
            let route_context = flow
                .work_context::<TransitContext>(carrier)
                .map_err(BridgeCheckpointError::Flow)?;
            validate_transit_context(
                &transit,
                route_context,
                flow,
                carrier,
                work,
                carrier_actor,
                self.kind,
            )?;
            if route_receipt.is_some() != route_metadata.is_some() {
                return Err(BridgeCheckpointError::InvalidState);
            }
            if let (Some(receipt), Some(metadata)) = (&route_receipt, &route_metadata) {
                receipt
                    .validate_context_bounded(route_context, metadata, limits.max_canonical_bytes)
                    .map_err(BridgeCheckpointError::Route)?;
            }
        }
        let arrival_request = self
            .arrival_request
            .map(|id| {
                rebind
                    .resolve_request(id)
                    .map_err(|_| BridgeCheckpointError::InvalidState)
            })
            .transpose()?;
        if let Some(request) = arrival_request {
            let saved = flow.request(request).map_err(BridgeCheckpointError::Flow)?;
            let at = self.arrival_at.ok_or(BridgeCheckpointError::InvalidState)?;
            if saved.work != Some(work)
                || saved.owner != acquire.owner
                || saved.resource != acquire.resource
                || !saved.timed
                || saved.submitted_at != at
                || saved.priority_level != acquire.priority_level
                || saved.deadline != acquire.deadline
                || saved.can_preempt != acquire.can_preempt
                || saved.preemptible != acquire.preemptible
                || flow
                    .work(work)
                    .map_err(BridgeCheckpointError::Flow)?
                    .request
                    != Some(request)
            {
                return Err(BridgeCheckpointError::InvalidState);
            }
        }
        Ok(BoundIntrinsicWork {
            decision: self.decision,
            expected_service_key: expected_service_key.clone(),
            service_stream: stream,
            sample,
            acquire,
            transit,
            route_metadata,
            route_receipt,
            runtime: flow.identity(),
            work,
            carrier,
            carrier_actor,
            kind: self.kind,
            pending_event,
            pending_priority: self.pending_priority,
            owned_events,
            stale_events,
            consumed_events,
            controls,
            retryable,
            arrival_request,
            arrival_at: self.arrival_at,
            _restart_types: PhantomData,
        })
    }
}

impl SubmittedIntrinsicWorkCheckpointV1 {
    pub(crate) fn capture<T: Clone, C: 'static>(
        submitted: &SubmittedIntrinsicWork<T, C>,
        flow: &FlowRuntime,
        adapter: &FidelityAdapter,
        limits: BridgeCheckpointLimits,
    ) -> Result<Self, BridgeCheckpointError> {
        if submitted.service_stream.purpose() != SeedPurpose::Service
            || submitted.service_stream.key() != submitted.expected_service_key
            || submitted.sample.checkpoint_parts().1 != &submitted.expected_service_key
            || submitted.sample.draw_after() > submitted.service_stream.draw_position()
            || submitted.sample.draw_before() > submitted.sample.draw_after()
            || adapter.decision(submitted.work) != Some(&submitted.decision)
        {
            return Err(BridgeCheckpointError::InvalidState);
        }
        let stream = submitted
            .service_stream
            .checkpoint_state(CalibrationStreamStateLimits {
                max_identifier_bytes: limits.max_identifier_bytes,
            })
            .map_err(BridgeCheckpointError::Stream)?;
        let spec = flow
            .work(submitted.work)
            .map_err(BridgeCheckpointError::Flow)?;
        let request = flow
            .request(submitted.request)
            .map_err(BridgeCheckpointError::Flow)?;
        if request.work != Some(submitted.work)
            || request.owner != spec.owner
            || request.resource != submitted.acquire.resource
            || request.submitted_at < submitted.acquire.at
            || request.priority_level != submitted.acquire.priority_level
            || request.deadline != submitted.acquire.deadline
            || request.can_preempt != submitted.acquire.can_preempt
            || request.preemptible != submitted.acquire.preemptible
            || !request.timed
            || spec.owner != submitted.acquire.owner
            || spec.original_duration != submitted.sample.duration()
            || spec.request != Some(submitted.request)
        {
            return Err(BridgeCheckpointError::InvalidState);
        }
        let (duration, _, draw_before, draw_after) = submitted.sample.checkpoint_parts();
        Ok(Self {
            version: 1,
            decision: submitted.decision,
            stream,
            duration_ticks: duration.ticks(),
            draw_before,
            draw_after,
            acquire: AcquireIntentCheckpointV1::capture(&submitted.acquire),
            work: submitted.work.entity_id(),
            request: submitted.request.entity_id(),
        })
    }

    pub(crate) fn restore<T: Clone, C: 'static>(
        self,
        flow: &FlowRuntime,
        adapter: &FidelityAdapter,
        expected_service_key: &CalibrationStreamKey,
        rebind: &FlowCheckpointRebindV1,
        limits: BridgeCheckpointLimits,
    ) -> Result<SubmittedIntrinsicWork<T, C>, BridgeCheckpointError> {
        if self.version != 1 || flow.identity() != *rebind.identity() {
            return Err(if self.version != 1 {
                BridgeCheckpointError::UnsupportedVersion
            } else {
                BridgeCheckpointError::InvalidState
            });
        }
        if self.duration_ticks == 0 || self.draw_before > self.draw_after {
            return Err(BridgeCheckpointError::InvalidState);
        }
        let stream = self
            .stream
            .restore_for(
                expected_service_key,
                CalibrationStreamStateLimits {
                    max_identifier_bytes: limits.max_identifier_bytes,
                },
            )
            .map_err(BridgeCheckpointError::Stream)?;
        let work = rebind
            .resolve_work(self.work)
            .map_err(|_| BridgeCheckpointError::InvalidState)?;
        let request = rebind
            .resolve_request(self.request)
            .map_err(|_| BridgeCheckpointError::InvalidState)?;
        let spec = flow.work(work).map_err(BridgeCheckpointError::Flow)?;
        let saved = flow.request(request).map_err(BridgeCheckpointError::Flow)?;
        let acquire = self.acquire.restore(rebind)?;
        if adapter.decision(work) != Some(&self.decision)
            || saved.work != Some(work)
            || saved.owner != spec.owner
            || saved.resource != acquire.resource
            || saved.submitted_at < acquire.at
            || saved.priority_level != acquire.priority_level
            || saved.deadline != acquire.deadline
            || saved.can_preempt != acquire.can_preempt
            || saved.preemptible != acquire.preemptible
            || !saved.timed
            || spec.owner != acquire.owner
            || spec.original_duration.ticks() != self.duration_ticks
            || spec.request != Some(request)
        {
            return Err(BridgeCheckpointError::InvalidState);
        }
        let sample = SampledWorkDuration::from_checkpoint_parts(
            SimDuration::from_ticks(self.duration_ticks),
            expected_service_key.clone(),
            self.draw_before,
            self.draw_after,
            &stream,
            expected_service_key,
        )
        .map_err(BridgeCheckpointError::Sample)?;
        Ok(SubmittedIntrinsicWork {
            decision: self.decision,
            expected_service_key: expected_service_key.clone(),
            service_stream: stream,
            sample,
            acquire,
            work,
            request,
            _restart_types: PhantomData,
        })
    }
}

impl AcquireIntentCheckpointV1 {
    fn capture(acquire: &AcquireIntent) -> Self {
        Self {
            resource: acquire.resource.entity_id(),
            owner: acquire.owner,
            at: acquire.at,
            priority_level: acquire.priority_level,
            deadline: acquire.deadline,
            scheduler_priority: acquire.scheduler_priority,
            can_preempt: acquire.can_preempt,
            preemptible: acquire.preemptible,
        }
    }

    fn restore(
        &self,
        rebind: &FlowCheckpointRebindV1,
    ) -> Result<AcquireIntent, BridgeCheckpointError> {
        Ok(AcquireIntent {
            resource: rebind
                .resolve_resource(self.resource)
                .map_err(|_| BridgeCheckpointError::InvalidState)?,
            owner: rebind
                .resolve_actor(self.owner)
                .map_err(|_| BridgeCheckpointError::InvalidState)?,
            at: self.at,
            priority_level: self.priority_level,
            deadline: self.deadline,
            scheduler_priority: self.scheduler_priority,
            can_preempt: self.can_preempt,
            preemptible: self.preemptible,
        })
    }
}

impl TransitRequestCheckpointV1 {
    fn capture(
        transit: &TransitRequest,
        limits: BridgeCheckpointLimits,
    ) -> Result<Self, BridgeCheckpointError> {
        match transit {
            TransitRequest::Zero => Ok(Self::Zero),
            TransitRequest::Route {
                graph,
                origin,
                destination,
                profile,
                ticks_per_second,
                carrier_actor,
                carrier_registration,
                kind,
            } => {
                let graph_bytes = graph.canonical_bytes_ref();
                if graph_bytes.len() > limits.max_canonical_bytes {
                    return Err(BridgeCheckpointError::LimitExceeded);
                }
                let plan = graph
                    .route(*origin, *destination, profile, *ticks_per_second)
                    .map_err(BridgeCheckpointError::Transit)?;
                if plan.segments().len() > limits.max_route_segments {
                    return Err(BridgeCheckpointError::LimitExceeded);
                }
                Ok(Self::Route {
                    graph_version: plan.graph_version(),
                    graph_canonical_bytes: graph_bytes.to_vec(),
                    origin: origin.value(),
                    destination: destination.value(),
                    mode: profile.mode().as_str().to_owned(),
                    speed_mm_per_second: profile.speed_mm_per_second().get(),
                    ticks_per_second: *ticks_per_second,
                    carrier_actor: *carrier_actor,
                    carrier_registration: carrier_registration.clone(),
                    kind: *kind,
                })
            }
        }
    }

    fn restore(
        self,
        graph: Option<Arc<TransitGraphV1>>,
        limits: BridgeCheckpointLimits,
    ) -> Result<TransitRequest, BridgeCheckpointError> {
        match (self, graph) {
            (Self::Zero, None) => Ok(TransitRequest::Zero),
            (
                Self::Route {
                    graph_version,
                    graph_canonical_bytes,
                    origin,
                    destination,
                    mode,
                    speed_mm_per_second,
                    ticks_per_second,
                    carrier_actor,
                    carrier_registration,
                    kind,
                },
                Some(graph),
            ) => {
                if graph_canonical_bytes.len() > limits.max_canonical_bytes {
                    return Err(BridgeCheckpointError::LimitExceeded);
                }
                let trusted_graph_bytes = graph.canonical_bytes_ref();
                if trusted_graph_bytes.len() > limits.max_canonical_bytes {
                    return Err(BridgeCheckpointError::LimitExceeded);
                }
                if trusted_graph_bytes != graph_canonical_bytes {
                    return Err(BridgeCheckpointError::InvalidState);
                }
                let origin = NodeId::new(origin);
                let destination = NodeId::new(destination);
                let profile = MovementProfile::new(&mode, speed_mm_per_second)
                    .map_err(BridgeCheckpointError::Transit)?;
                let plan = graph
                    .route(origin, destination, &profile, ticks_per_second)
                    .map_err(BridgeCheckpointError::Transit)?;
                if plan.graph_version() != graph_version
                    || plan.graph_canonical_bytes() != graph_canonical_bytes
                    || plan.segments().len() > limits.max_route_segments
                {
                    return Err(BridgeCheckpointError::InvalidState);
                }
                Ok(TransitRequest::Route {
                    graph,
                    origin,
                    destination,
                    profile,
                    ticks_per_second,
                    carrier_actor,
                    carrier_registration,
                    kind,
                })
            }
            _ => Err(BridgeCheckpointError::InvalidState),
        }
    }
}

fn validate_transit_context(
    transit: &TransitRequest,
    context: &TransitContext,
    flow: &FlowRuntime,
    carrier: WorkId,
    service_work: WorkId,
    bound_carrier_actor: Option<EntityId>,
    bound_kind: Option<EventKind>,
) -> Result<(), BridgeCheckpointError> {
    let TransitRequest::Route {
        graph,
        origin,
        destination,
        profile,
        ticks_per_second,
        carrier_actor,
        carrier_registration,
        kind,
    } = transit
    else {
        return Err(BridgeCheckpointError::InvalidState);
    };
    let approved_plan = graph
        .route(*origin, *destination, profile, *ticks_per_second)
        .map_err(BridgeCheckpointError::Transit)?;
    let carrier_spec = flow.work(carrier).map_err(BridgeCheckpointError::Flow)?;
    if bound_carrier_actor != Some(*carrier_actor)
        || bound_kind != Some(*kind)
        || carrier_spec.owner != *carrier_actor
        || carrier_spec.context_type_key != *carrier_registration
        || context.service_work() != service_work
        || context.route_plan() != &approved_plan
    {
        return Err(BridgeCheckpointError::InvalidState);
    }
    Ok(())
}

fn checked_identifier_sum<const N: usize>(
    lengths: [usize; N],
) -> Result<usize, BridgeCheckpointError> {
    lengths.into_iter().try_fold(0usize, |sum, len| {
        sum.checked_add(len)
            .ok_or(BridgeCheckpointError::LimitExceeded)
    })
}

fn validate_dispatch_limits(
    dispatch: &FlowDispatch,
    limits: BridgeCheckpointLimits,
) -> Result<(), BridgeCheckpointError> {
    if dispatch.records.len() > limits.max_dispatch_records
        || dispatch.callback_batches.len() > limits.max_dispatch_batches
    {
        return Err(BridgeCheckpointError::LimitExceeded);
    }
    let mut admissions = 0usize;
    for batch in &dispatch.callback_batches {
        if let FlowBatchReceipt::Accepted(items) = batch {
            admissions = admissions
                .checked_add(items.len())
                .ok_or(BridgeCheckpointError::LimitExceeded)?;
        }
    }
    if admissions > limits.max_dispatch_admissions {
        return Err(BridgeCheckpointError::LimitExceeded);
    }
    Ok(())
}

fn validate_dispatch_references(
    dispatch: &FlowDispatch,
    rebind: &FlowCheckpointRebindV1,
) -> Result<(), BridgeCheckpointError> {
    rebind
        .resolve_issued_event(dispatch.event)
        .map_err(|_| BridgeCheckpointError::InvalidState)?;
    for record in &dispatch.records {
        validate_lifecycle_record(record, rebind)?;
    }
    for batch in &dispatch.callback_batches {
        match batch {
            FlowBatchReceipt::Accepted(admissions) => {
                for admission in admissions {
                    let (batch, index) = admission.ticket.checkpoint_parts();
                    rebind
                        .resolve_ticket(batch, index)
                        .map_err(|_| BridgeCheckpointError::InvalidState)?;
                    rebind
                        .resolve_issued_event(admission.event)
                        .map_err(|_| BridgeCheckpointError::InvalidState)?;
                    if let Some(request) = admission.request {
                        rebind
                            .resolve_request(request.entity_id())
                            .map_err(|_| BridgeCheckpointError::InvalidState)?;
                    }
                    if let Some(event) = admission.deadline_event {
                        rebind
                            .resolve_issued_event(event)
                            .map_err(|_| BridgeCheckpointError::InvalidState)?;
                    }
                }
            }
            FlowBatchReceipt::Rejected(rejection) => {
                if let Some(ticket) = rejection.failed_ticket {
                    let (batch, index) = ticket.checkpoint_parts();
                    rebind
                        .resolve_ticket(batch, index)
                        .map_err(|_| BridgeCheckpointError::InvalidState)?;
                }
            }
        }
    }
    Ok(())
}

fn validate_lifecycle_record(
    record: &LifecycleRecord,
    rebind: &FlowCheckpointRebindV1,
) -> Result<(), BridgeCheckpointError> {
    rebind
        .resolve_request(record.request.entity_id())
        .map_err(|_| BridgeCheckpointError::InvalidState)?;
    rebind
        .resolve_resource(record.resource.entity_id())
        .map_err(|_| BridgeCheckpointError::InvalidState)?;
    rebind
        .resolve_issued_event(record.causal_event_id)
        .map_err(|_| BridgeCheckpointError::InvalidState)?;
    if let Some(lease) = record.lease {
        rebind
            .resolve_request(lease.request_id().entity_id())
            .map_err(|_| BridgeCheckpointError::InvalidState)?;
    }
    rebind
        .resolve_actor(record.snapshot.owner)
        .map_err(|_| BridgeCheckpointError::InvalidState)?;
    if let Some(work) = record.snapshot.work {
        rebind
            .resolve_work(work.entity_id())
            .map_err(|_| BridgeCheckpointError::InvalidState)?;
    }
    if let Some(request) = record.snapshot.preemptor_request {
        rebind
            .resolve_request(request.entity_id())
            .map_err(|_| BridgeCheckpointError::InvalidState)?;
    }
    if let Some(lease) = record.snapshot.causal_lease {
        rebind
            .resolve_request(lease.request_id().entity_id())
            .map_err(|_| BridgeCheckpointError::InvalidState)?;
    }
    Ok(())
}

fn validate_owned_vectors(
    owned: &[EventId],
    stale: &[EventId],
    consumed: &[EventId],
    controls: &[(EventId, FlowDomainControl)],
    limits: BridgeCheckpointLimits,
) -> Result<(), BridgeCheckpointError> {
    if owned.len() > limits.max_owned_events
        || stale.len() > limits.max_owned_events
        || consumed.len() > limits.max_owned_events
        || controls.len() > limits.max_controls
    {
        Err(BridgeCheckpointError::LimitExceeded)
    } else {
        Ok(())
    }
}

fn validate_checkpoint_identifier_budget(
    checkpoint: &BoundIntrinsicWorkCheckpointV1,
    limits: BridgeCheckpointLimits,
) -> Result<(), BridgeCheckpointError> {
    let mut total = checkpoint.stream.identity.study_id.len();
    for length in [
        checkpoint.stream.identity.seed_schedule_id.len(),
        checkpoint.stream.identity.case_key.len(),
        checkpoint.stream.identity.task_key.len(),
    ] {
        total = total
            .checked_add(length)
            .ok_or(BridgeCheckpointError::LimitExceeded)?;
    }
    if let TransitRequestCheckpointV1::Route {
        mode,
        carrier_registration,
        ..
    } = &checkpoint.transit
    {
        total = total
            .checked_add(mode.len())
            .and_then(|n| n.checked_add(carrier_registration.len()))
            .ok_or(BridgeCheckpointError::LimitExceeded)?;
    }
    if let Some(metadata) = &checkpoint.route_metadata {
        total = total
            .checked_add(
                metadata
                    .checkpoint_identifier_bytes()
                    .map_err(BridgeCheckpointError::Route)?,
            )
            .ok_or(BridgeCheckpointError::LimitExceeded)?;
    }
    if let Some(receipt) = &checkpoint.route_receipt {
        total = total
            .checked_add(
                receipt
                    .metadata_identifier_bytes()
                    .map_err(BridgeCheckpointError::Route)?,
            )
            .ok_or(BridgeCheckpointError::LimitExceeded)?;
    }
    if total > limits.max_identifier_bytes {
        Err(BridgeCheckpointError::LimitExceeded)
    } else {
        Ok(())
    }
}

pub(crate) struct WorkPreparationInput<T: Clone, C: 'static> {
    pub(crate) owner: EntityId,
    pub(crate) subsystem: String,
    pub(crate) stratum: String,
    pub(crate) expected_service_key: CalibrationStreamKey,
    pub(crate) service_stream: CalibrationStream,
    pub(crate) template: T,
    pub(crate) registration: String,
    pub(crate) make_context: fn(&T) -> C,
    pub(crate) acquire: AcquireIntent,
    pub(crate) transit: TransitRequest,
    route_metadata: Option<RouteMetadata>,
}

impl<T: Clone, C: 'static> WorkPreparationInput<T, C> {
    pub(crate) fn new(
        identity: PreparationIdentity,
        service_stream: CalibrationStream,
        expected_service_key: CalibrationStreamKey,
        template: T,
        make_context: fn(&T) -> C,
        acquire: AcquireIntent,
        transit: TransitRequest,
    ) -> Self {
        Self {
            owner: identity.owner,
            subsystem: identity.subsystem,
            stratum: identity.stratum,
            expected_service_key,
            service_stream,
            template,
            registration: identity.registration,
            make_context,
            acquire,
            transit,
            route_metadata: None,
        }
    }

    pub(crate) fn with_route_metadata(mut self, metadata: RouteMetadata) -> Self {
        self.route_metadata = Some(metadata);
        self
    }
}

pub(crate) struct PrepareFailure<T: Clone, C: 'static> {
    pub(crate) input: WorkPreparationInput<T, C>,
    pub(crate) error: BridgeError,
}

pub(crate) struct PreparedIntrinsicWork<'a, T: Clone, C: 'static> {
    permit: FidelityAdmissionPermit<'a>,
    decision: FidelityDecision,
    expected_service_key: CalibrationStreamKey,
    service_stream: CalibrationStream,
    sample: SampledWorkDuration,
    template: T,
    registration: String,
    make_context: fn(&T) -> C,
    acquire: AcquireIntent,
    transit: TransitRequest,
    route_metadata: Option<RouteMetadata>,
}

pub(crate) struct CreateFailure<'a, T: Clone, C: 'static> {
    pub(crate) prepared: PreparedIntrinsicWork<'a, T, C>,
    pub(crate) error: BridgeError,
}

pub(crate) struct CreatedIntrinsicWork<'a, T: Clone, C: 'static> {
    permit: FidelityAdmissionPermit<'a>,
    decision: FidelityDecision,
    expected_service_key: CalibrationStreamKey,
    service_stream: CalibrationStream,
    sample: SampledWorkDuration,
    acquire: AcquireIntent,
    transit: TransitRequest,
    route_metadata: Option<RouteMetadata>,
    work: WorkId,
    _restart_types: PhantomData<fn() -> (T, C)>,
}

pub(crate) struct BindFailure<'a, T: Clone, C: 'static> {
    pub(crate) created: CreatedIntrinsicWork<'a, T, C>,
    pub(crate) error: BridgeError,
}

pub(crate) struct BoundIntrinsicWork<T: Clone, C: 'static> {
    decision: FidelityDecision,
    expected_service_key: CalibrationStreamKey,
    service_stream: CalibrationStream,
    sample: SampledWorkDuration,
    acquire: AcquireIntent,
    transit: TransitRequest,
    route_metadata: Option<RouteMetadata>,
    route_receipt: Option<RouteReceipt>,
    runtime: FlowRuntimeIdentity,
    work: WorkId,
    carrier: Option<WorkId>,
    carrier_actor: Option<EntityId>,
    kind: Option<EventKind>,
    pending_event: Option<EventId>,
    pending_priority: Option<i32>,
    owned_events: Vec<EventId>,
    stale_events: Vec<EventId>,
    consumed_events: Vec<EventId>,
    controls: Vec<(EventId, FlowDomainControl)>,
    retryable: Option<FlowDispatch>,
    arrival_request: Option<RequestId>,
    arrival_at: Option<SimTime>,
    _restart_types: PhantomData<fn() -> (T, C)>,
}

/// One-shot, same-process continuation for one already-bound bridge item.
///
/// Owning the runtime and adapter prevents their engine APIs from advancing
/// while detached. The private fields and consuming resume make the handoff
/// opaque and non-forkable.
pub(crate) struct BoundWorkContinuation<T: Clone, C: 'static> {
    flow: FlowRuntime,
    adapter: FidelityAdapter,
    bound: BoundIntrinsicWork<T, C>,
}

impl<T: Clone + 'static, C: 'static> BoundWorkContinuation<T, C> {
    #[allow(clippy::result_large_err)]
    pub(crate) fn capture(
        flow: FlowRuntime,
        adapter: FidelityAdapter,
        bound: BoundIntrinsicWork<T, C>,
    ) -> Result<
        Self,
        (
            FlowRuntime,
            FidelityAdapter,
            BoundIntrinsicWork<T, C>,
            BridgeError,
        ),
    > {
        let valid_runtime = bound.runtime == flow.identity();
        let valid_decision = adapter.decision(bound.work) == Some(&bound.decision);
        let route_validation = if !valid_runtime {
            Ok(())
        } else {
            match (bound.carrier, bound.route_receipt.is_some()) {
                (Some(carrier), _) => bound.validate_route_context(&flow, carrier),
                (None, false) => Ok(()),
                (None, true) => Err(BridgeError::InvalidDispatch),
            }
        };
        if !valid_runtime || !valid_decision || route_validation.is_err() {
            let error = route_validation
                .err()
                .unwrap_or(BridgeError::Fidelity(FidelityError::InvalidWork));
            return Err((flow, adapter, bound, error));
        }
        Ok(Self {
            flow,
            adapter,
            bound,
        })
    }

    pub(crate) fn resume(self) -> (FlowRuntime, FidelityAdapter, BoundIntrinsicWork<T, C>) {
        (self.flow, self.adapter, self.bound)
    }
}

pub(crate) struct SubmitFailure<T: Clone, C: 'static> {
    pub(crate) bound: BoundIntrinsicWork<T, C>,
    pub(crate) error: BridgeError,
}

pub(crate) struct SubmittedIntrinsicWork<T: Clone, C: 'static> {
    decision: FidelityDecision,
    expected_service_key: CalibrationStreamKey,
    service_stream: CalibrationStream,
    sample: SampledWorkDuration,
    acquire: AcquireIntent,
    work: WorkId,
    request: RequestId,
    _restart_types: PhantomData<fn() -> (T, C)>,
}

impl<T: Clone + 'static, C: 'static> WorkPreparationInput<T, C> {
    #[allow(clippy::result_large_err)]
    pub(crate) fn prepare<'a>(
        mut self,
        flow: &FlowRuntime,
        adapter: &'a mut FidelityAdapter,
        provider: &IntrinsicWorkProvider,
    ) -> Result<PreparedIntrinsicWork<'a, T, C>, PrepareFailure<T, C>> {
        if let Some(metadata) = &self.route_metadata {
            if let Err(error) = metadata.validate() {
                return Err(PrepareFailure {
                    input: self,
                    error: BridgeError::RouteReceipt(error),
                });
            }
        }
        let permit = match adapter.prepare_admission(flow, self.owner, &self.subsystem) {
            Ok(permit) => permit,
            Err(error) => {
                return Err(PrepareFailure {
                    input: self,
                    error: BridgeError::Fidelity(error),
                });
            }
        };
        let decision = permit.decision();

        let validation = if self.owner != self.acquire.owner
            || self.acquire.at < flow.now()
            || self
                .acquire
                .deadline
                .is_some_and(|deadline| deadline <= self.acquire.at)
        {
            Err(BridgeError::InvalidDispatch)
        } else if flow.resource(self.acquire.resource).is_err() {
            Err(BridgeError::Flow(FlowError::InvalidResource))
        } else if self.service_stream.purpose() != SeedPurpose::Service {
            Err(BridgeError::Duration(WorkDurationError::WrongPurpose))
        } else if self.service_stream.key() != self.expected_service_key {
            Err(BridgeError::Duration(WorkDurationError::IdentityMismatch))
        } else {
            Ok(())
        };
        if let Err(error) = validation {
            drop(permit);
            return Err(PrepareFailure { input: self, error });
        }

        if decision.mode == FidelityMode::Micro {
            let zero_route = match &self.transit {
                TransitRequest::Zero => false,
                TransitRequest::Route {
                    graph,
                    origin,
                    destination,
                    profile,
                    ticks_per_second,
                    ..
                } => match graph.route(*origin, *destination, profile, *ticks_per_second) {
                    Ok(route) => route.duration() == SimDuration::ZERO,
                    Err(error) => {
                        drop(permit);
                        return Err(PrepareFailure {
                            input: self,
                            error: BridgeError::Transit(error),
                        });
                    }
                },
            };
            if zero_route {
                self.transit = TransitRequest::Zero;
            }
        }

        // Sampling is the last fallible operation in prepare. The provider
        // commits the temporary stream only on success; all earlier failures
        // return this unchanged input, while success moves its sole stream on.
        let sample = match provider.sample(
            &self.stratum,
            &mut self.service_stream,
            &self.expected_service_key,
        ) {
            Ok(sample) => sample,
            Err(error) => {
                drop(permit);
                return Err(PrepareFailure {
                    input: self,
                    error: BridgeError::Duration(error),
                });
            }
        };

        Ok(PreparedIntrinsicWork {
            permit,
            decision,
            expected_service_key: self.expected_service_key,
            service_stream: self.service_stream,
            sample,
            template: self.template,
            registration: self.registration,
            make_context: self.make_context,
            acquire: self.acquire,
            transit: self.transit,
            route_metadata: self.route_metadata,
        })
    }
}

impl<'a, T: Clone + 'static, C: 'static> PreparedIntrinsicWork<'a, T, C> {
    pub(crate) fn decision(&self) -> FidelityDecision {
        self.decision
    }

    pub(crate) fn sampled_duration(&self) -> SimDuration {
        self.sample.duration()
    }

    pub(crate) fn draw_bounds(&self) -> (u64, u64) {
        (self.sample.draw_before(), self.sample.draw_after())
    }

    pub(crate) fn service_draw_position(&self) -> u64 {
        self.service_stream.draw_position()
    }

    #[allow(clippy::result_large_err)]
    pub(crate) fn create(
        self,
        flow: &mut FlowRuntime,
    ) -> Result<CreatedIntrinsicWork<'a, T, C>, CreateFailure<'a, T, C>> {
        let Self {
            permit,
            decision,
            expected_service_key,
            service_stream,
            sample,
            template,
            registration,
            make_context,
            acquire,
            transit,
            route_metadata,
        } = self;
        match flow.create_restartable_work(
            acquire.owner,
            sample.duration(),
            &registration,
            template.clone(),
            make_context,
        ) {
            Ok(work) => Ok(CreatedIntrinsicWork {
                permit,
                decision,
                expected_service_key,
                service_stream,
                sample,
                acquire,
                transit,
                route_metadata,
                work,
                _restart_types: PhantomData,
            }),
            Err(error) => Err(CreateFailure {
                prepared: PreparedIntrinsicWork {
                    permit,
                    decision,
                    expected_service_key,
                    service_stream,
                    sample,
                    template,
                    registration,
                    make_context,
                    acquire,
                    transit,
                    route_metadata,
                },
                error: BridgeError::Flow(error),
            }),
        }
    }
}

impl<'a, T: Clone + 'static, C: 'static> CreatedIntrinsicWork<'a, T, C> {
    pub(crate) fn work(&self) -> WorkId {
        self.work
    }

    pub(crate) fn decision(&self) -> FidelityDecision {
        self.decision
    }

    #[allow(clippy::result_large_err)]
    pub(crate) fn bind(
        self,
        flow: &FlowRuntime,
    ) -> Result<BoundIntrinsicWork<T, C>, BindFailure<'a, T, C>> {
        match self.permit.bind(flow, self.work, self.sample.duration()) {
            Ok(_) => Ok(BoundIntrinsicWork {
                decision: self.decision,
                expected_service_key: self.expected_service_key,
                service_stream: self.service_stream,
                sample: self.sample,
                acquire: self.acquire,
                transit: self.transit,
                route_metadata: self.route_metadata,
                route_receipt: None,
                runtime: flow.identity(),
                work: self.work,
                carrier: None,
                carrier_actor: None,
                kind: None,
                pending_event: None,
                pending_priority: None,
                owned_events: Vec::new(),
                stale_events: Vec::new(),
                consumed_events: Vec::new(),
                controls: Vec::new(),
                retryable: None,
                arrival_request: None,
                arrival_at: None,
                _restart_types: PhantomData,
            }),
            Err((permit, error)) => Err(BindFailure {
                created: CreatedIntrinsicWork { permit, ..self },
                error: BridgeError::Fidelity(error),
            }),
        }
    }
}

impl<T: Clone + 'static, C: 'static> BoundIntrinsicWork<T, C> {
    pub(crate) fn work(&self) -> WorkId {
        self.work
    }

    // The disposable C20 runner overlays the paired fixture that reads this sample.
    #[allow(dead_code)]
    pub(crate) fn sampled_duration(&self) -> SimDuration {
        self.sample.duration()
    }

    pub(crate) fn draw_position(&self) -> u64 {
        self.service_stream.draw_position()
    }

    pub(crate) fn service_draw_position(&self) -> u64 {
        self.draw_position()
    }

    pub(crate) fn decision(&self) -> FidelityDecision {
        self.decision
    }

    pub(crate) fn service_identity_matches(&self) -> bool {
        self.service_stream.key() == self.expected_service_key
    }

    pub(crate) fn acquire_intent(&self) -> &AcquireIntent {
        &self.acquire
    }

    pub(crate) fn next_service_draw_probe_for_checkpoint(&self) -> u64 {
        self.service_stream
            .snapshot()
            .restore_for(&self.expected_service_key)
            .expect("owned snapshot identity")
            .next_u64()
            .expect("draw probe")
    }

    pub(crate) fn pending_event_for_checkpoint(&self) -> Option<EventId> {
        self.pending_event
    }

    pub(crate) fn carrier_id_for_checkpoint(&self) -> Option<WorkId> {
        self.carrier
    }

    pub(crate) fn route_receipt_sha_for_checkpoint(
        &self,
        flow: &FlowRuntime,
    ) -> Result<String, BridgeError> {
        let carrier = self.carrier.ok_or(BridgeError::InvalidDispatch)?;
        self.validate_route_context(flow, carrier)?;
        self.route_receipt
            .as_ref()
            .map(|receipt| receipt.checkpoint_sha256_hex())
            .ok_or(BridgeError::InvalidDispatch)
    }

    pub(crate) fn validate_route_context_for_checkpoint(
        &self,
        flow: &FlowRuntime,
    ) -> Result<(), BridgeError> {
        match self.carrier {
            Some(carrier) => self.validate_route_context(flow, carrier),
            None => Err(BridgeError::InvalidDispatch),
        }
    }

    pub(crate) fn start_transit(&mut self, flow: &mut FlowRuntime) -> Result<EventId, BridgeError> {
        if let Some(metadata) = &self.route_metadata {
            metadata.validate().map_err(BridgeError::RouteReceipt)?;
        }
        if self.runtime != flow.identity() {
            return Err(BridgeError::Fidelity(FidelityError::InvalidWork));
        }
        if self.decision.mode != FidelityMode::Micro {
            return Err(BridgeError::InvalidDispatch);
        }
        let (graph, origin, destination, profile, tps, actor, registration, kind) =
            match &self.transit {
                TransitRequest::Route {
                    graph,
                    origin,
                    destination,
                    profile,
                    ticks_per_second,
                    carrier_actor,
                    carrier_registration,
                    kind,
                } => (
                    graph,
                    *origin,
                    *destination,
                    profile,
                    *ticks_per_second,
                    *carrier_actor,
                    carrier_registration.as_str(),
                    *kind,
                ),
                TransitRequest::Zero => return Err(BridgeError::InvalidDispatch),
            };
        if self.pending_event.is_some() {
            return Err(BridgeError::InvalidDispatch);
        }
        let carrier = if let Some(carrier) = self.carrier {
            if self.carrier_actor != Some(actor) || self.kind != Some(kind) {
                return Err(BridgeError::InvalidDispatch);
            }
            self.ensure_route_receipt(flow, carrier)?;
            self.validate_route_context(flow, carrier)?;
            carrier
        } else {
            let route = graph
                .route(origin, destination, profile, tps)
                .map_err(BridgeError::Transit)?;
            if route.duration() == SimDuration::ZERO {
                return Err(BridgeError::InvalidDispatch);
            }
            let acquire = FlowAcquireCommand {
                resource: self.acquire.resource,
                owner: self.acquire.owner,
                work: Some(self.work),
                at: self.acquire.at,
                priority_level: self.acquire.priority_level,
                deadline: self.acquire.deadline,
                scheduler_priority: self.acquire.scheduler_priority,
                timed: true,
                can_preempt: self.acquire.can_preempt,
                preemptible: self.acquire.preemptible,
            };
            let context = TransitContext::new(flow, route, acquire, self.acquire.at)
                .map_err(BridgeError::Transit)?;
            let carrier = flow
                .create_actor_domain_context(actor, registration, kind, context)
                .map_err(BridgeError::Flow)?;
            self.carrier = Some(carrier);
            self.carrier_actor = Some(actor);
            self.kind = Some(kind);
            self.pending_priority = Some(self.acquire.scheduler_priority);
            self.ensure_route_receipt(flow, carrier)?;
            self.validate_route_context(flow, carrier)?;
            carrier
        };
        let event = schedule_transit_start(
            flow,
            carrier,
            kind,
            self.acquire.at,
            self.acquire.scheduler_priority,
        )
        .map_err(BridgeError::Flow)?;
        self.pending_event = Some(event);
        self.owned_events.push(event);
        Ok(event)
    }

    fn ensure_route_receipt(
        &mut self,
        flow: &FlowRuntime,
        carrier: WorkId,
    ) -> Result<(), BridgeError> {
        if self.route_receipt.is_none() {
            if let Some(metadata) = &self.route_metadata {
                let context = flow
                    .work_context::<TransitContext>(carrier)
                    .map_err(BridgeError::Flow)?;
                self.route_receipt = Some(
                    RouteReceipt::from_context(context, metadata)
                        .map_err(BridgeError::RouteReceipt)?,
                );
            }
        }
        Ok(())
    }

    pub(crate) fn schedule_transit_control(
        &mut self,
        flow: &mut FlowRuntime,
        action: FlowDomainControl,
        at: SimTime,
        priority: i32,
    ) -> Result<EventId, BridgeError> {
        if self.runtime != flow.identity() {
            return Err(BridgeError::Fidelity(FidelityError::InvalidWork));
        }
        let (Some(carrier), Some(kind)) = (self.carrier, self.kind) else {
            return Err(BridgeError::InvalidDispatch);
        };
        self.validate_route_context(flow, carrier)?;
        let event = schedule_transit_control(flow, carrier, kind, action, at, priority)
            .map_err(BridgeError::Flow)?;
        self.controls.push((event, action));
        self.owned_events.push(event);
        Ok(event)
    }

    pub(crate) fn observe_transit_dispatch(
        &mut self,
        flow: &FlowRuntime,
        dispatch: &FlowDispatch,
    ) -> Result<TransitObservation, BridgeError> {
        if self.runtime != flow.identity() {
            return Err(BridgeError::Fidelity(FidelityError::InvalidWork));
        }
        let (Some(carrier), Some(_kind), Some(pending)) =
            (self.carrier, self.kind, self.pending_event)
        else {
            return Err(BridgeError::InvalidDispatch);
        };
        self.validate_route_context(flow, carrier)?;
        if dispatch.at != flow.now()
            || !self.owned_events.contains(&dispatch.event)
            || self.consumed_events.contains(&dispatch.event)
        {
            return Err(BridgeError::InvalidDispatch);
        }
        if let Some(index) = self
            .controls
            .iter()
            .position(|(event, _)| *event == dispatch.event)
        {
            let action = self.controls[index].1;
            let [FlowBatchReceipt::Accepted(admissions)] = dispatch.callback_batches.as_slice()
            else {
                return match dispatch.callback_batches.as_slice() {
                    [FlowBatchReceipt::Rejected(rejection)] => {
                        Err(BridgeError::Flow(rejection.error))
                    }
                    _ => Err(BridgeError::InvalidDispatch),
                };
            };
            let context = flow
                .work_context::<TransitContext>(carrier)
                .map_err(BridgeError::Flow)?;
            if context.service_work() != self.work {
                return Err(BridgeError::InvalidDispatch);
            }
            let scheduled = match action {
                FlowDomainControl::Pause => {
                    if context.phase() != TransitPhase::Paused || !admissions.is_empty() {
                        return Err(BridgeError::InvalidDispatch);
                    }
                    None
                }
                FlowDomainControl::Resume => {
                    if !matches!(context.phase(), TransitPhase::Ready | TransitPhase::Moving) {
                        return Err(BridgeError::InvalidDispatch);
                    }
                    match admissions.as_slice() {
                        [] => None,
                        [admission]
                            if admission.request.is_none() && admission.event != dispatch.event =>
                        {
                            Some(admission.event)
                        }
                        _ => return Err(BridgeError::InvalidDispatch),
                    }
                }
            };
            self.controls.remove(index);
            self.consumed_events.push(dispatch.event);
            if let Some(event) = scheduled {
                if action == FlowDomainControl::Resume {
                    if let Some(replaced) = self.pending_event.filter(|pending| *pending != event) {
                        self.stale_events.push(replaced);
                    }
                }
                self.pending_event = Some(event);
                self.owned_events.push(event);
            }
            return Ok(match action {
                FlowDomainControl::Pause => TransitObservation::Paused,
                FlowDomainControl::Resume => TransitObservation::Resumed,
            });
        }
        if dispatch.event != pending {
            if self.stale_events.contains(&dispatch.event) {
                self.consumed_events.push(dispatch.event);
                return Ok(TransitObservation::IgnoredStale);
            }
            return Err(BridgeError::InvalidDispatch);
        }
        if matches!(
            dispatch.callback_batches.as_slice(),
            [FlowBatchReceipt::Rejected(_)]
        ) {
            if self.carrier_actor.is_none()
                || self.pending_priority.is_none()
                || flow
                    .work_context::<TransitContext>(carrier)
                    .map_err(BridgeError::Flow)?
                    .service_work()
                    != self.work
            {
                return Err(BridgeError::InvalidDispatch);
            }
            TransitContext::validate_retry_dispatch(flow, carrier, dispatch)
                .map_err(|_| BridgeError::InvalidDispatch)?;
            self.retryable = Some(dispatch.clone());
            return Ok(TransitObservation::Rejected);
        }
        let [FlowBatchReceipt::Accepted(admissions)] = dispatch.callback_batches.as_slice() else {
            return Err(BridgeError::InvalidDispatch);
        };
        let context = flow
            .work_context::<TransitContext>(carrier)
            .map_err(BridgeError::Flow)?;
        if context.service_work() != self.work {
            return Err(BridgeError::InvalidDispatch);
        }
        let phase = context.phase();
        let ticket = match phase {
            TransitPhase::Moving => context.next_progress_ticket(),
            TransitPhase::Arrived => context.arrival_ticket(),
            _ => None,
        };
        if let Some(ticket) = ticket {
            let [admission] = admissions.as_slice() else {
                return Err(BridgeError::InvalidDispatch);
            };
            if admission.ticket != ticket {
                return Err(BridgeError::InvalidDispatch);
            }
            if admission.event == dispatch.event {
                return Err(BridgeError::InvalidDispatch);
            }
            if phase == TransitPhase::Arrived {
                let request = admission.request.ok_or(BridgeError::InvalidDispatch)?;
                let saved = flow.request(request).map_err(BridgeError::Flow)?;
                if saved.work != Some(self.work)
                    || saved.owner != self.acquire.owner
                    || saved.resource != self.acquire.resource
                    || !saved.timed
                    || saved.submitted_at != dispatch.at
                    || saved.priority_level != self.acquire.priority_level
                    || saved.deadline != self.acquire.deadline
                    || saved.can_preempt != self.acquire.can_preempt
                    || saved.preemptible != self.acquire.preemptible
                    || flow.work(self.work).map_err(BridgeError::Flow)?.request != Some(request)
                {
                    return Err(BridgeError::InvalidDispatch);
                }
                self.consumed_events.push(dispatch.event);
                self.pending_event = Some(admission.event);
                self.owned_events.push(admission.event);
                self.arrival_request = Some(request);
                self.arrival_at = Some(dispatch.at);
                return Ok(TransitObservation::Arrived);
            }
            if admission.request.is_some() {
                return Err(BridgeError::InvalidDispatch);
            }
            self.consumed_events.push(dispatch.event);
            self.pending_event = Some(admission.event);
            self.owned_events.push(admission.event);
            return Ok(TransitObservation::Progress);
        }
        if !admissions.is_empty()
            || context.phase() != TransitPhase::Paused
            || context.expects_event(dispatch.event, dispatch.at)
        {
            return Err(BridgeError::InvalidDispatch);
        }
        self.consumed_events.push(dispatch.event);
        self.stale_events.push(dispatch.event);
        Ok(TransitObservation::IgnoredStale)
    }

    pub(crate) fn retry_transit(
        &mut self,
        flow: &mut FlowRuntime,
        rejected: &FlowDispatch,
    ) -> Result<EventId, BridgeError> {
        if self.runtime != flow.identity() {
            return Err(BridgeError::Fidelity(FidelityError::InvalidWork));
        }
        let (Some(carrier), Some(kind), Some(pending), Some(priority), Some(saved)) = (
            self.carrier,
            self.kind,
            self.pending_event,
            self.pending_priority,
            self.retryable.as_ref(),
        ) else {
            return Err(BridgeError::InvalidDispatch);
        };
        let TransitRequest::Route {
            carrier_actor,
            kind: expected_kind,
            ..
        } = &self.transit
        else {
            return Err(BridgeError::InvalidDispatch);
        };
        if self.carrier_actor != Some(*carrier_actor)
            || kind != *expected_kind
            || priority != self.acquire.scheduler_priority
            || rejected != saved
            || rejected.event != pending
            || self.consumed_events.contains(&pending)
            || !matches!(
                rejected.callback_batches.as_slice(),
                [FlowBatchReceipt::Rejected(_)]
            )
        {
            return Err(BridgeError::InvalidDispatch);
        }
        let context = flow
            .work_context::<TransitContext>(carrier)
            .map_err(BridgeError::Flow)?;
        self.validate_route_context(flow, carrier)?;
        if context.service_work() != self.work {
            return Err(BridgeError::InvalidDispatch);
        }
        let event = TransitContext::schedule_transit_retry(flow, carrier, kind, rejected, priority)
            .map_err(BridgeError::Flow)?;
        self.consumed_events.push(pending);
        self.pending_event = Some(event);
        self.owned_events.push(event);
        self.retryable = None;
        Ok(event)
    }

    pub(crate) fn finish_transit(
        self,
        flow: &FlowRuntime,
    ) -> Result<SubmittedIntrinsicWork<T, C>, BridgeError> {
        if self.runtime != flow.identity() {
            return Err(BridgeError::Fidelity(FidelityError::InvalidWork));
        }
        let (Some(carrier), Some(request), Some(at)) =
            (self.carrier, self.arrival_request, self.arrival_at)
        else {
            return Err(BridgeError::InvalidDispatch);
        };
        let context = flow
            .work_context::<TransitContext>(carrier)
            .map_err(BridgeError::Flow)?;
        self.validate_route_context(flow, carrier)?;
        if context.service_work() != self.work || context.phase() != TransitPhase::Arrived {
            return Err(BridgeError::InvalidDispatch);
        }
        let saved = flow.request(request).map_err(BridgeError::Flow)?;
        if saved.resource != self.acquire.resource
            || saved.owner != self.acquire.owner
            || saved.work != Some(self.work)
            || !saved.timed
            || saved.submitted_at != at
            || saved.priority_level != self.acquire.priority_level
            || saved.deadline != self.acquire.deadline
            || saved.can_preempt != self.acquire.can_preempt
            || saved.preemptible != self.acquire.preemptible
            || flow.work(self.work).map_err(BridgeError::Flow)?.request != Some(request)
        {
            return Err(BridgeError::InvalidDispatch);
        }
        Ok(SubmittedIntrinsicWork {
            decision: self.decision,
            expected_service_key: self.expected_service_key,
            service_stream: self.service_stream,
            sample: self.sample,
            acquire: self.acquire,
            work: self.work,
            request,
            _restart_types: PhantomData,
        })
    }

    fn validate_route_context(
        &self,
        flow: &FlowRuntime,
        carrier: WorkId,
    ) -> Result<(), BridgeError> {
        match (&self.route_metadata, &self.route_receipt) {
            (None, None) => Ok(()),
            (Some(metadata), Some(receipt)) => {
                let context = flow
                    .work_context::<TransitContext>(carrier)
                    .map_err(BridgeError::Flow)?;
                receipt
                    .validate_context(context, metadata)
                    .map_err(BridgeError::RouteReceipt)
            }
            _ => Err(BridgeError::InvalidDispatch),
        }
    }

    fn validate_route_context_bounded(
        &self,
        flow: &FlowRuntime,
        carrier: WorkId,
        max_canonical_bytes: usize,
    ) -> Result<(), BridgeCheckpointError> {
        match (&self.route_metadata, &self.route_receipt) {
            (None, None) => Ok(()),
            (Some(metadata), Some(receipt)) => {
                let context = flow
                    .work_context::<TransitContext>(carrier)
                    .map_err(BridgeCheckpointError::Flow)?;
                receipt
                    .validate_context_bounded(context, metadata, max_canonical_bytes)
                    .map_err(BridgeCheckpointError::Route)
            }
            _ => Err(BridgeCheckpointError::InvalidState),
        }
    }

    #[allow(clippy::result_large_err)]
    pub(crate) fn submit(
        self,
        flow: &mut FlowRuntime,
    ) -> Result<SubmittedIntrinsicWork<T, C>, SubmitFailure<T, C>> {
        if self.runtime != flow.identity() {
            return Err(SubmitFailure {
                bound: self,
                error: BridgeError::InvalidDispatch,
            });
        }
        if self.decision.mode == FidelityMode::Micro
            && matches!(&self.transit, TransitRequest::Route { .. })
        {
            return Err(SubmitFailure {
                bound: self,
                error: BridgeError::Flow(FlowError::InvalidState),
            });
        }
        if !self.service_identity_matches() {
            return Err(SubmitFailure {
                bound: self,
                error: BridgeError::Duration(WorkDurationError::IdentityMismatch),
            });
        }
        let spec = match flow.work(self.work) {
            Ok(spec) => spec,
            Err(error) => {
                return Err(SubmitFailure {
                    bound: self,
                    error: BridgeError::Flow(error),
                });
            }
        };
        if spec.request.is_some() {
            return Err(SubmitFailure {
                bound: self,
                error: BridgeError::ConflictingSubmission,
            });
        }
        let progress = match flow.work_progress(self.work) {
            Ok(progress) => progress,
            Err(error) => {
                return Err(SubmitFailure {
                    bound: self,
                    error: BridgeError::Flow(error),
                });
            }
        };
        if spec.owner != self.acquire.owner
            || spec.original_duration != self.sample.duration()
            || progress.state != WorkState::Pending
        {
            return Err(SubmitFailure {
                bound: self,
                error: BridgeError::InvalidDispatch,
            });
        }

        let mut builder = flow
            .acquire(self.acquire.resource)
            .owner(self.acquire.owner)
            .at(self.acquire.at)
            .priority(self.acquire.priority_level)
            .scheduler_priority(self.acquire.scheduler_priority)
            .can_preempt(self.acquire.can_preempt)
            .timed_work(self.work);
        if let Some(deadline) = self.acquire.deadline {
            builder = builder.deadline(deadline);
        }
        if let Some(strategy) = self.acquire.preemptible {
            builder = builder.preemptible(strategy);
        }
        match builder.submit() {
            Ok(request) => {
                // Treat the runtime's ID as a candidate until both sides of
                // the authoritative work/request association agree.
                let request_matches = flow.request(request).is_ok_and(|saved| {
                    saved.resource == self.acquire.resource
                        && saved.owner == self.acquire.owner
                        && saved.work == Some(self.work)
                        && saved.timed
                        && saved.submitted_at == self.acquire.at
                        && saved.priority_level == self.acquire.priority_level
                        && saved.deadline == self.acquire.deadline
                        && saved.can_preempt == self.acquire.can_preempt
                        && saved.preemptible == self.acquire.preemptible
                });
                let work_matches = flow
                    .work(self.work)
                    .is_ok_and(|saved| saved.request == Some(request));
                if !request_matches || !work_matches {
                    // The request may already exist, so retain the bound
                    // state for diagnosis; its normal retry path will fail
                    // closed on the existing WorkSpec.request association.
                    return Err(SubmitFailure {
                        bound: self,
                        error: BridgeError::InvalidDispatch,
                    });
                }
                Ok(SubmittedIntrinsicWork {
                    decision: self.decision,
                    expected_service_key: self.expected_service_key,
                    service_stream: self.service_stream,
                    sample: self.sample,
                    acquire: self.acquire,
                    work: self.work,
                    request,
                    _restart_types: PhantomData,
                })
            }
            Err(error) => Err(SubmitFailure {
                bound: self,
                error: BridgeError::Flow(error),
            }),
        }
    }
}

impl<T: Clone + 'static, C: 'static> SubmittedIntrinsicWork<T, C> {
    pub(crate) fn work(&self) -> WorkId {
        self.work
    }

    pub(crate) fn request(&self) -> RequestId {
        self.request
    }

    pub(crate) fn sampled_duration(&self) -> SimDuration {
        self.sample.duration()
    }

    pub(crate) fn draw_position(&self) -> u64 {
        self.service_stream.draw_position()
    }

    pub(crate) fn service_draw_position(&self) -> u64 {
        self.draw_position()
    }

    pub(crate) fn decision(&self) -> FidelityDecision {
        self.decision
    }

    pub(crate) fn service_identity_matches(&self) -> bool {
        self.service_stream.key() == self.expected_service_key
    }

    pub(crate) fn acquire_intent(&self) -> &AcquireIntent {
        &self.acquire
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::route_receipt::DistanceProvenance;
    use crate::seed_map::CalibrationSeedMap;
    use crate::work_duration::{IntrinsicDurationDistribution, INTRINSIC_WORK_PROVIDER_VERSION_V1};
    use kairo_ecs_abm::spatial::{EdgeId, MovementModeId, NodeId, TransitEdge};
    use kairo_ecs_abm::{
        register_transit_context, register_transit_context_checkpoint_codec,
        register_transit_context_checkpoint_domain, register_transit_context_reject_first_for_test,
        TransitContext, TransitContextCheckpointLimitsV1, TransitContextCheckpointV1,
    };
    use kairo_ecs_des::fidelity::{FidelityCheckpointLimits, FidelityPolicy};
    use kairo_ecs_des::{
        FlowBatchRejection, FlowCallbackCodeV1, FlowCheckpointCodecError, FlowCheckpointCodecs,
        FlowCheckpointLimits, FlowCheckpointRebindV1, FlowDispatch, FlowHandlerCodeIds,
        FlowRuntime, LifecycleTransition, RequestState, WorkHandlers,
    };
    use std::cell::RefCell;
    use std::rc::Rc;

    thread_local! {
        static TEST_CHECKPOINT_REBIND: RefCell<Option<FlowCheckpointRebindV1>> = const { RefCell::new(None) };
    }

    fn encode_bridge_test_context(
        value: &u32,
        max_bytes: usize,
    ) -> Result<Vec<u8>, FlowCheckpointCodecError> {
        if max_bytes < 4 {
            return Err(FlowCheckpointCodecError(
                "test context exceeds limit".to_owned(),
            ));
        }
        Ok(value.to_le_bytes().to_vec())
    }

    fn decode_bridge_test_context(
        bytes: &[u8],
        rebind: &FlowCheckpointRebindV1,
    ) -> Result<u32, FlowCheckpointCodecError> {
        let bytes: [u8; 4] = bytes
            .try_into()
            .map_err(|_| FlowCheckpointCodecError("invalid test context".to_owned()))?;
        TEST_CHECKPOINT_REBIND.with(|slot| *slot.borrow_mut() = Some(rebind.clone()));
        Ok(u32::from_le_bytes(bytes))
    }

    fn native_test_codecs() -> FlowCheckpointCodecs {
        let mut codecs = FlowCheckpointCodecs::new();
        codecs
            .register_context::<u32>(
                "bridge.context",
                1,
                encode_bridge_test_context,
                decode_bridge_test_context,
            )
            .unwrap();
        codecs
            .register_work_handlers::<u32>(
                "bridge.context",
                FlowHandlerCodeIds::default(),
                WorkHandlers::default(),
            )
            .unwrap();
        codecs
            .register_restart_template::<u32, u32>(
                "bridge.template",
                1,
                encode_bridge_test_context,
                decode_bridge_test_context,
            )
            .unwrap();
        codecs
            .register_restart_factory::<u32, u32>("bridge.template", "bridge.factory", make_context)
            .unwrap();
        codecs
    }

    fn joined_native_codecs(
        source: &FlowRuntime,
        trusted_graph: Arc<TransitGraphV1>,
    ) -> FlowCheckpointCodecs {
        const TRANSIT_LIMITS: TransitContextCheckpointLimitsV1 =
            TransitContextCheckpointLimitsV1::new(128, 16 * 1024, 1024, 32 * 1024);
        let source_identity = source.identity();
        let transit_state = Rc::new(RefCell::new(None::<TransitContextCheckpointV1>));
        let encode_state = transit_state.clone();
        let decode_state = transit_state;
        let mut codecs = native_test_codecs();
        codecs
            .register_context_with_owner::<TransitContext>(
                "bridge.transit",
                1,
                move |context, max_bytes| {
                    if max_bytes < 1 {
                        return Err(FlowCheckpointCodecError(
                            "transit test marker exceeds limit".to_owned(),
                        ));
                    }
                    let image = context
                        .checkpoint_v1(&source_identity, TRANSIT_LIMITS)
                        .map_err(|error| FlowCheckpointCodecError(error.to_string()))?;
                    *encode_state.borrow_mut() = Some(image);
                    Ok(vec![1])
                },
                move |bytes, row_owner, rebind| {
                    if bytes != [1] {
                        return Err(FlowCheckpointCodecError(
                            "invalid transit test marker".to_owned(),
                        ));
                    }
                    let image = decode_state.borrow_mut().take().ok_or_else(|| {
                        FlowCheckpointCodecError("missing test transit owner image".to_owned())
                    })?;
                    image
                        .restore_for_owner(&trusted_graph, rebind, row_owner, TRANSIT_LIMITS)
                        .map_err(|error| FlowCheckpointCodecError(error.to_string()))
                },
            )
            .unwrap();
        register_transit_context_checkpoint_domain(
            &mut codecs,
            "bridge.transit",
            EventKind::custom(0xC20),
            FlowCallbackCodeV1 {
                stable_id: "test.bridge.transit.plan".to_owned(),
                version: 1,
            },
            FlowCallbackCodeV1 {
                stable_id: "test.bridge.transit.accept".to_owned(),
                version: 1,
            },
        )
        .unwrap();
        codecs
    }

    fn restore_route_flow_with_view(
        source: &FlowRuntime,
        trusted_graph: Arc<TransitGraphV1>,
    ) -> (FlowRuntime, FlowCheckpointRebindV1) {
        let codecs = joined_native_codecs(source, trusted_graph);
        let image = source
            .capture_checkpoint(&codecs, FlowCheckpointLimits::default())
            .unwrap();
        FlowRuntime::restore_checkpoint_with_rebind(image, &codecs, FlowCheckpointLimits::default())
            .unwrap()
    }

    fn route_graph(bound: &BoundIntrinsicWork<u32, u32>) -> Arc<TransitGraphV1> {
        match &bound.transit {
            TransitRequest::Route { graph, .. } => graph.clone(),
            TransitRequest::Zero => panic!("route request expected"),
        }
    }

    fn restore_adapter_for_flow(
        adapter: &FidelityAdapter,
        source: &FlowRuntime,
        restored: &FlowRuntime,
        work: WorkId,
    ) -> FidelityAdapter {
        let limits = FidelityCheckpointLimits {
            max_admitted: 32,
            max_overrides: 32,
            max_subsystem_bytes: 4096,
        };
        FidelityAdapter::from_checkpoint(
            adapter.checkpoint(source, limits).unwrap(),
            restored,
            &[(work, work)],
            limits,
        )
        .unwrap()
    }

    fn restore_flow_with_test_rebind(flow: &FlowRuntime) -> (FlowRuntime, FlowCheckpointRebindV1) {
        TEST_CHECKPOINT_REBIND.with(|slot| *slot.borrow_mut() = None);
        let codecs = native_test_codecs();
        let image = flow
            .capture_checkpoint(&codecs, FlowCheckpointLimits::default())
            .unwrap();
        let restored =
            FlowRuntime::restore_checkpoint(image, &codecs, FlowCheckpointLimits::default())
                .unwrap();
        let rebind = TEST_CHECKPOINT_REBIND
            .with(|slot| slot.borrow_mut().take())
            .expect("registered context decoder captured the validated rebind view");
        (restored, rebind)
    }

    #[derive(Clone, Copy)]
    enum TransitIntent {
        Zero,
        Route,
    }

    fn make_context(template: &u32) -> u32 {
        *template
    }

    fn configured_route_metadata(purpose: &str) -> RouteMetadata {
        RouteMetadata::v1(purpose, DistanceProvenance::ConfiguredGeometry)
    }

    fn provider() -> IntrinsicWorkProvider {
        IntrinsicWorkProvider::new(
            INTRINSIC_WORK_PROVIDER_VERSION_V1,
            vec![(
                "triage".to_owned(),
                IntrinsicDurationDistribution::weighted_ticks(vec![(10, 1), (20, 1)]).unwrap(),
            )],
        )
        .unwrap()
    }

    fn input(
        mode: FidelityMode,
        transit: TransitIntent,
        purpose: SeedPurpose,
        mismatch_key: bool,
    ) -> (
        WorkPreparationInput<u32, u32>,
        FlowRuntime,
        FidelityAdapter,
        ResourceId,
    ) {
        let mut flow = FlowRuntime::new();
        let owner = flow.spawn_actor().unwrap();
        let carrier_actor = flow.spawn_actor().unwrap();
        let resource = flow.create_resource(1).unwrap();

        let mut seed_map = CalibrationSeedMap::new(1, "bridge-test", 19).unwrap();
        let key = seed_map
            .key_for("paired", 0, "case-a", "task-a", SeedPurpose::Service)
            .unwrap();
        let stream_task = if mismatch_key { "task-b" } else { "task-a" };
        let service_stream = seed_map
            .stream_for("paired", 0, "case-a", stream_task, purpose)
            .unwrap();
        let transit = match transit {
            TransitIntent::Zero => TransitRequest::Zero,
            TransitIntent::Route => {
                let node = |n| NodeId::new(n);
                let mode = MovementModeId::new("walk").unwrap();
                TransitRequest::Route {
                    graph: Arc::new(
                        TransitGraphV1::new(
                            1,
                            vec![node(1), node(2)],
                            vec![TransitEdge {
                                id: EdgeId::new(1),
                                from: node(1),
                                to: node(2),
                                length_mm: 1,
                                allowed_modes: vec![mode],
                            }],
                        )
                        .unwrap(),
                    ),
                    origin: node(1),
                    destination: node(2),
                    profile: MovementProfile::new("walk", 1).unwrap(),
                    ticks_per_second: 1,
                    carrier_actor,
                    carrier_registration: "bridge.transit".to_owned(),
                    kind: EventKind::custom(0xC20),
                }
            }
        };
        let input = WorkPreparationInput::new(
            PreparationIdentity {
                owner,
                subsystem: "assessment".to_owned(),
                registration: "bridge.context".to_owned(),
                stratum: "triage".to_owned(),
            },
            service_stream,
            key,
            42,
            make_context,
            AcquireIntent {
                resource,
                owner,
                at: SimTime::from_ticks(0),
                priority_level: 3,
                deadline: None,
                scheduler_priority: 7,
                can_preempt: false,
                preemptible: None,
            },
            transit,
        );
        let adapter = FidelityAdapter::new(FidelityPolicy::new(1, Some(mode)).unwrap());
        (input, flow, adapter, resource)
    }

    fn must_prepare<'a>(
        result: Result<PreparedIntrinsicWork<'a, u32, u32>, PrepareFailure<u32, u32>>,
    ) -> PreparedIntrinsicWork<'a, u32, u32> {
        match result {
            Ok(value) => value,
            Err(failure) => panic!("prepare failed: {:?}", failure.error),
        }
    }

    fn must_create<'a>(
        result: Result<CreatedIntrinsicWork<'a, u32, u32>, CreateFailure<'a, u32, u32>>,
    ) -> CreatedIntrinsicWork<'a, u32, u32> {
        match result {
            Ok(value) => value,
            Err(failure) => panic!("create failed: {:?}", failure.error),
        }
    }

    fn must_bind<'a>(
        result: Result<BoundIntrinsicWork<u32, u32>, BindFailure<'a, u32, u32>>,
    ) -> BoundIntrinsicWork<u32, u32> {
        match result {
            Ok(value) => value,
            Err(failure) => panic!("bind failed: {:?}", failure.error),
        }
    }

    fn must_submit(
        result: Result<SubmittedIntrinsicWork<u32, u32>, SubmitFailure<u32, u32>>,
    ) -> SubmittedIntrinsicWork<u32, u32> {
        match result {
            Ok(value) => value,
            Err(failure) => panic!("submit failed: {:?}", failure.error),
        }
    }

    fn bound_route_with_adapter() -> (FlowRuntime, FidelityAdapter, BoundIntrinsicWork<u32, u32>) {
        bound_route_with_metadata(false)
    }

    fn bound_annotated_route_with_adapter(
    ) -> (FlowRuntime, FidelityAdapter, BoundIntrinsicWork<u32, u32>) {
        bound_route_with_metadata(true)
    }

    fn bound_route_with_metadata(
        annotated: bool,
    ) -> (FlowRuntime, FidelityAdapter, BoundIntrinsicWork<u32, u32>) {
        let (input, mut flow, mut adapter, _) = input(
            FidelityMode::Micro,
            TransitIntent::Route,
            SeedPurpose::Service,
            false,
        );
        let input = if annotated {
            input.with_route_metadata(configured_route_metadata("patient-transfer"))
        } else {
            input
        };
        register_transit_context(&mut flow, "bridge.transit", EventKind::custom(0xC20)).unwrap();
        flow.register_work_handlers("bridge.context", WorkHandlers::<u32>::default())
            .unwrap();
        let prepared = must_prepare(input.prepare(&flow, &mut adapter, &provider()));
        let created = must_create(prepared.create(&mut flow));
        let bound = must_bind(created.bind(&flow));
        (flow, adapter, bound)
    }

    fn bound_simple_with_adapter(
        mode: FidelityMode,
        transit: TransitIntent,
    ) -> (FlowRuntime, FidelityAdapter, BoundIntrinsicWork<u32, u32>) {
        let (input, mut flow, mut adapter, _) = input(mode, transit, SeedPurpose::Service, false);
        flow.register_work_handlers("bridge.context", WorkHandlers::<u32>::default())
            .unwrap();
        let prepared = must_prepare(input.prepare(&flow, &mut adapter, &provider()));
        let created = must_create(prepared.create(&mut flow));
        let bound = must_bind(created.bind(&flow));
        (flow, adapter, bound)
    }

    fn bound_route_at_with_adapter(
        start: SimTime,
    ) -> (FlowRuntime, FidelityAdapter, BoundIntrinsicWork<u32, u32>) {
        let (mut input, mut flow, mut adapter, _) = input(
            FidelityMode::Micro,
            TransitIntent::Route,
            SeedPurpose::Service,
            false,
        );
        input.acquire.at = start;
        register_transit_context(&mut flow, "bridge.transit", EventKind::custom(0xC20)).unwrap();
        flow.register_work_handlers("bridge.context", WorkHandlers::<u32>::default())
            .unwrap();
        let prepared = must_prepare(input.prepare(&flow, &mut adapter, &provider()));
        let created = must_create(prepared.create(&mut flow));
        let bound = must_bind(created.bind(&flow));
        (flow, adapter, bound)
    }

    fn bound_simple_with_metadata(
        mode: FidelityMode,
        transit: TransitIntent,
    ) -> (FlowRuntime, FidelityAdapter, BoundIntrinsicWork<u32, u32>) {
        let (input, mut flow, mut adapter, _) = input(mode, transit, SeedPurpose::Service, false);
        let input = input.with_route_metadata(configured_route_metadata("patient-transfer"));
        flow.register_work_handlers("bridge.context", WorkHandlers::<u32>::default())
            .unwrap();
        let prepared = must_prepare(input.prepare(&flow, &mut adapter, &provider()));
        let created = must_create(prepared.create(&mut flow));
        let bound = must_bind(created.bind(&flow));
        (flow, adapter, bound)
    }

    fn bridge_checkpoint_limits() -> BridgeCheckpointLimits {
        BridgeCheckpointLimits {
            max_identifier_bytes: 4096,
            max_owned_events: 128,
            max_controls: 128,
            max_dispatch_records: 128,
            max_dispatch_batches: 128,
            max_dispatch_admissions: 128,
            max_route_segments: 128,
            max_canonical_bytes: 16 * 1024,
        }
    }

    fn bridge_wire_limits() -> checkpoint_wire::BridgeWireLimits {
        checkpoint_wire::BridgeWireLimits {
            native: bridge_checkpoint_limits(),
            max_wire_bytes: 4 * 1024 * 1024,
            seed: crate::seed_map::checkpoint_wire::SeedWireLimits {
                max_entries: 32,
                max_identifier_bytes: 4096,
                max_wire_bytes: 64 * 1024,
            },
            flow: kairo_ecs_des::FlowCheckpointWireLimits::default(),
        }
    }

    fn capture_real_transit_flow_image(
        flow: &FlowRuntime,
        bound: &BoundIntrinsicWork<u32, u32>,
    ) -> kairo_ecs_des::FlowCheckpointV1 {
        let mut codecs = native_test_codecs();
        register_transit_context_checkpoint_codec(
            &mut codecs,
            "bridge.transit",
            flow.identity(),
            route_graph(bound),
            TransitContextCheckpointLimitsV1::new(128, 16 * 1024, 1024, 32 * 1024),
        )
        .unwrap();
        register_transit_context_checkpoint_domain(
            &mut codecs,
            "bridge.transit",
            EventKind::custom(0xC20),
            FlowCallbackCodeV1 {
                stable_id: "test.bridge.transit.plan".to_owned(),
                version: 1,
            },
            FlowCallbackCodeV1 {
                stable_id: "test.bridge.transit.accept".to_owned(),
                version: 1,
            },
        )
        .unwrap();
        flow.capture_checkpoint(&codecs, FlowCheckpointLimits::default())
            .unwrap()
    }

    #[test]
    fn bound_bridge_wire_roundtrips_complete_zero_route_image() {
        let (flow, adapter, bound) =
            bound_simple_with_adapter(FidelityMode::Macro, TransitIntent::Zero);
        let image = BoundIntrinsicWorkCheckpointV1::capture(
            &bound,
            &flow,
            &adapter,
            bridge_checkpoint_limits(),
        )
        .unwrap();
        let bytes = image.encode_wire_v1(bridge_wire_limits()).unwrap();
        let decoded =
            BoundIntrinsicWorkCheckpointV1::decode_wire_v1(&bytes, bridge_wire_limits()).unwrap();
        assert_eq!(decoded, image);
        assert_eq!(decoded.work_entity_id(), image.work_entity_id());
        assert_eq!(decoded.stream_identity(), image.stream_identity());
        let mut trailing = bytes.clone();
        trailing.push(0);
        assert_eq!(
            BoundIntrinsicWorkCheckpointV1::decode_wire_v1(&trailing, bridge_wire_limits()),
            Err(checkpoint_wire::BridgeWireError::TrailingBytes)
        );
        let mut bad_schema = bytes.clone();
        bad_schema[8..10].copy_from_slice(&2u16.to_le_bytes());
        assert_eq!(
            BoundIntrinsicWorkCheckpointV1::decode_wire_v1(&bad_schema, bridge_wire_limits()),
            Err(checkpoint_wire::BridgeWireError::UnsupportedSchema(2))
        );
        let mut bad_tag = bytes.clone();
        bad_tag[10] = 9;
        assert_eq!(
            BoundIntrinsicWorkCheckpointV1::decode_wire_v1(&bad_tag, bridge_wire_limits()),
            Err(checkpoint_wire::BridgeWireError::InvalidTag)
        );
        let mut tiny = bridge_wire_limits();
        tiny.max_wire_bytes = bytes.len() - 1;
        assert_eq!(
            BoundIntrinsicWorkCheckpointV1::decode_wire_v1(&bytes, tiny),
            Err(checkpoint_wire::BridgeWireError::LimitExceeded)
        );
    }

    #[test]
    fn coherent_cut_requires_the_observed_domain_dispatch_frontier() {
        let (mut flow, adapter, mut bound) = bound_route_with_adapter();
        bound.start_transit(&mut flow).unwrap();
        let control = bound
            .schedule_transit_control(
                &mut flow,
                FlowDomainControl::Pause,
                SimTime::from_ticks(1),
                0,
            )
            .unwrap();
        let _image = BoundIntrinsicWorkCheckpointV1::capture(
            &bound,
            &flow,
            &adapter,
            bridge_checkpoint_limits(),
        )
        .unwrap();
        let mut codecs = native_test_codecs();
        register_transit_context_checkpoint_codec(
            &mut codecs,
            "bridge.transit",
            flow.identity(),
            route_graph(&bound),
            TransitContextCheckpointLimitsV1::new(128, 16 * 1024, 1024, 32 * 1024),
        )
        .unwrap();
        register_transit_context_checkpoint_domain(
            &mut codecs,
            "bridge.transit",
            EventKind::custom(0xC20),
            FlowCallbackCodeV1 {
                stable_id: "test.bridge.transit.plan".to_owned(),
                version: 1,
            },
            FlowCallbackCodeV1 {
                stable_id: "test.bridge.transit.accept".to_owned(),
                version: 1,
            },
        )
        .unwrap();
        let flow_image = flow
            .capture_checkpoint(&codecs, FlowCheckpointLimits::default())
            .unwrap();
        assert_eq!(
            checkpoint_wire::validate_coherent_cut(
                &flow_image,
                &flow,
                &bound,
                bridge_checkpoint_limits()
            ),
            Ok(())
        );
        let mut missing_control = flow_image.clone();
        missing_control.commands.retain(|(id, _)| *id != control);
        assert_eq!(
            checkpoint_wire::validate_coherent_cut(
                &missing_control,
                &flow,
                &bound,
                bridge_checkpoint_limits()
            ),
            Err(BridgeCheckpointError::InvalidState)
        );
        let mut inconsistent = flow_image.clone();
        inconsistent
            .commands
            .retain(|(id, _)| Some(*id) != bound.pending_event);
        assert_eq!(
            checkpoint_wire::validate_coherent_cut(
                &inconsistent,
                &flow,
                &bound,
                bridge_checkpoint_limits()
            ),
            Err(BridgeCheckpointError::InvalidState)
        );
    }

    #[test]
    fn coherent_cut_rejects_flow_step_before_bridge_observation() {
        let (mut flow, _adapter, mut bound) = bound_route_with_adapter();
        bound.start_transit(&mut flow).unwrap();
        let before = capture_real_transit_flow_image(&flow, &bound);
        assert_eq!(
            checkpoint_wire::validate_coherent_cut(
                &before,
                &flow,
                &bound,
                bridge_checkpoint_limits(),
            ),
            Ok(())
        );

        // The runtime applies the event and mutates TransitContext, but the
        // bridge has not consumed that dispatch. The two owners are therefore
        // on opposite sides of the observation frontier and cannot be joined.
        assert!(flow.step().unwrap().is_some());
        let unobserved = capture_real_transit_flow_image(&flow, &bound);
        assert_eq!(
            checkpoint_wire::validate_coherent_cut(
                &unobserved,
                &flow,
                &bound,
                bridge_checkpoint_limits(),
            ),
            Err(BridgeCheckpointError::InvalidState)
        );
    }

    #[test]
    fn coherent_cut_accepts_observed_arrival_and_queued_admission() {
        let (mut flow, adapter, mut bound) = bound_route_with_adapter();
        bound.start_transit(&mut flow).unwrap();
        loop {
            let dispatch = flow.step().unwrap().unwrap();
            let observation = bound.observe_transit_dispatch(&flow, &dispatch).unwrap();
            if observation == TransitObservation::Arrived {
                break;
            }
        }
        assert!(bound.arrival_request.is_some());
        let _image = BoundIntrinsicWorkCheckpointV1::capture(
            &bound,
            &flow,
            &adapter,
            bridge_checkpoint_limits(),
        )
        .unwrap();
        let flow_image = capture_real_transit_flow_image(&flow, &bound);
        assert_eq!(
            checkpoint_wire::validate_coherent_cut(
                &flow_image,
                &flow,
                &bound,
                bridge_checkpoint_limits(),
            ),
            Ok(())
        );
        let mut without_submit = flow_image;
        without_submit
            .commands
            .retain(|(event, _)| Some(*event) != bound.pending_event);
        assert_eq!(
            checkpoint_wire::validate_coherent_cut(
                &without_submit,
                &flow,
                &bound,
                bridge_checkpoint_limits(),
            ),
            Err(BridgeCheckpointError::InvalidState)
        );
    }

    #[test]
    fn native_bound_checkpoint_captures_zero_draw_macro_and_is_bounded_before_clone() {
        let (flow, adapter, bound) =
            bound_simple_with_adapter(FidelityMode::Macro, TransitIntent::Zero);
        let before = bound.draw_position();
        let image = BoundIntrinsicWorkCheckpointV1::capture(
            &bound,
            &flow,
            &adapter,
            bridge_checkpoint_limits(),
        )
        .unwrap();
        assert_eq!(image.version, 1);
        assert_eq!(image.decision, bound.decision());
        assert_eq!(image.duration_ticks, bound.sampled_duration().ticks());
        assert_eq!(image.draw_before, bound.sample.draw_before());
        assert_eq!(image.draw_after, bound.sample.draw_after());
        assert!(matches!(image.transit, TransitRequestCheckpointV1::Zero));
        assert_eq!(bound.draw_position(), before);

        let mut tiny = bridge_checkpoint_limits();
        tiny.max_identifier_bytes = 1;
        assert_eq!(
            BoundIntrinsicWorkCheckpointV1::capture(&bound, &flow, &adapter, tiny),
            Err(BridgeCheckpointError::Stream(
                CalibrationStreamStateError::LimitExceeded
            ))
        );
        assert_eq!(bound.draw_position(), before);
    }

    #[test]
    fn native_bound_checkpoint_restores_exact_service_stream_onto_fresh_flow() {
        let (flow, adapter, bound) =
            bound_simple_with_adapter(FidelityMode::Macro, TransitIntent::Zero);
        let limits = bridge_checkpoint_limits();
        let image =
            BoundIntrinsicWorkCheckpointV1::capture(&bound, &flow, &adapter, limits).unwrap();
        let work = bound.work();
        let adapter_image = adapter
            .checkpoint(
                &flow,
                FidelityCheckpointLimits {
                    max_admitted: 32,
                    max_overrides: 32,
                    max_subsystem_bytes: 4096,
                },
            )
            .unwrap();
        let key = bound.expected_service_key.clone();
        let expected_draws = [bound
            .service_stream
            .snapshot()
            .restore()
            .unwrap()
            .next_u64()
            .unwrap()];
        let (restored_flow, rebind) = restore_flow_with_test_rebind(&flow);
        let restored_adapter = FidelityAdapter::from_checkpoint(
            adapter_image,
            &restored_flow,
            &[(work, work)],
            FidelityCheckpointLimits {
                max_admitted: 32,
                max_overrides: 32,
                max_subsystem_bytes: 4096,
            },
        )
        .unwrap();
        let restored: BoundIntrinsicWork<u32, u32> = BoundIntrinsicWorkCheckpointV1::restore(
            image.clone(),
            &restored_flow,
            &restored_adapter,
            &key,
            &rebind,
            None,
            limits,
        )
        .unwrap();
        assert_eq!(restored.work(), work);
        assert_eq!(restored.draw_position(), bound.draw_position());
        assert_eq!(
            restored.next_service_draw_probe_for_checkpoint(),
            expected_draws[0]
        );

        let mut unsupported = image.clone();
        unsupported.version = 2;
        assert!(matches!(
            BoundIntrinsicWorkCheckpointV1::restore::<u32, u32>(
                unsupported,
                &restored_flow,
                &restored_adapter,
                &key,
                &rebind,
                None,
                limits,
            ),
            Err(BridgeCheckpointError::UnsupportedVersion)
        ));
        let mut tiny = limits;
        tiny.max_identifier_bytes = 1;
        assert!(matches!(
            BoundIntrinsicWorkCheckpointV1::restore::<u32, u32>(
                image.clone(),
                &restored_flow,
                &restored_adapter,
                &key,
                &rebind,
                None,
                tiny,
            ),
            Err(BridgeCheckpointError::LimitExceeded)
        ));
        let mut wrong_map = CalibrationSeedMap::new(1, "bridge-test", 19).unwrap();
        let wrong_key = wrong_map
            .key_for("paired", 0, "other-case", "task-a", SeedPurpose::Service)
            .unwrap();
        assert!(matches!(
            BoundIntrinsicWorkCheckpointV1::restore::<u32, u32>(
                image,
                &restored_flow,
                &restored_adapter,
                &wrong_key,
                &rebind,
                None,
                limits,
            ),
            Err(BridgeCheckpointError::Stream(_))
        ));
        assert_eq!(
            restored_flow.work(work).unwrap().owner,
            restored.acquire.owner
        );
    }

    #[test]
    fn bridge_checkpoint_trusted_binding_checks_key_and_graph_without_mutation() {
        let (flow, adapter, bound) = bound_route_with_adapter();
        let image = BoundIntrinsicWorkCheckpointV1::capture(
            &bound,
            &flow,
            &adapter,
            bridge_checkpoint_limits(),
        )
        .unwrap();
        let original = image.clone();
        let graph = route_graph(&bound);
        assert_eq!(
            image.validate_trusted_binding(&bound.expected_service_key, Some(&graph)),
            Ok(())
        );

        let mut different_task_map = CalibrationSeedMap::new(1, "bridge-test", 19).unwrap();
        let different_task_key = different_task_map
            .key_for("paired", 0, "case-a", "task-b", SeedPurpose::Service)
            .unwrap();
        let mut different_root_map = CalibrationSeedMap::new(1, "bridge-test", 20).unwrap();
        let different_root_key = different_root_map
            .key_for("paired", 0, "case-a", "task-a", SeedPurpose::Service)
            .unwrap();
        let mut different_purpose_map = CalibrationSeedMap::new(1, "bridge-test", 19).unwrap();
        let different_purpose_key = different_purpose_map
            .key_for("paired", 0, "case-a", "task-a", SeedPurpose::Transit)
            .unwrap();
        let other_graph = TransitGraphV1::new(
            1,
            vec![NodeId::new(1), NodeId::new(2)],
            vec![TransitEdge {
                id: EdgeId::new(1),
                from: NodeId::new(1),
                to: NodeId::new(2),
                length_mm: 2,
                allowed_modes: vec![MovementModeId::new("walk").unwrap()],
            }],
        )
        .unwrap();

        for result in [
            image.validate_trusted_binding(&different_task_key, Some(&graph)),
            image.validate_trusted_binding(&different_root_key, Some(&graph)),
            image.validate_trusted_binding(&different_purpose_key, Some(&graph)),
            image.validate_trusted_binding(&bound.expected_service_key, None),
            image.validate_trusted_binding(&bound.expected_service_key, Some(&other_graph)),
        ] {
            assert_eq!(result, Err(BridgeCheckpointError::InvalidState));
        }
        assert_eq!(image, original);

        let (zero_flow, zero_adapter, zero_bound) =
            bound_simple_with_adapter(FidelityMode::Macro, TransitIntent::Zero);
        let zero_image = BoundIntrinsicWorkCheckpointV1::capture(
            &zero_bound,
            &zero_flow,
            &zero_adapter,
            bridge_checkpoint_limits(),
        )
        .unwrap();
        assert_eq!(
            zero_image.validate_trusted_binding(&zero_bound.expected_service_key, None),
            Ok(())
        );
        // A graph binding is unused for a Zero transit payload and is ignored.
        assert_eq!(
            zero_image
                .validate_trusted_binding(&zero_bound.expected_service_key, Some(&other_graph)),
            Ok(())
        );

        let (mut submitted_flow, submitted_adapter, submitted_bound) =
            bound_simple_with_adapter(FidelityMode::Macro, TransitIntent::Zero);
        let submitted = must_submit(submitted_bound.submit(&mut submitted_flow));
        let submitted_image = SubmittedIntrinsicWorkCheckpointV1::capture(
            &submitted,
            &submitted_flow,
            &submitted_adapter,
            bridge_checkpoint_limits(),
        )
        .unwrap();
        let submitted_original = submitted_image.clone();
        assert_eq!(
            submitted_image.validate_trusted_binding(&submitted.expected_service_key),
            Ok(())
        );
        assert_eq!(
            submitted_image.validate_trusted_binding(&different_task_key),
            Err(BridgeCheckpointError::InvalidState)
        );
        assert_eq!(submitted_image, submitted_original);
    }

    #[test]
    fn joined_native_restore_rebinds_paused_route_and_rejects_carrier_mutations() {
        let (mut flow, adapter, mut bound) = bound_route_with_adapter();
        bound.start_transit(&mut flow).unwrap();
        let start = flow.step().unwrap().unwrap();
        assert_eq!(
            bound.observe_transit_dispatch(&flow, &start),
            Ok(TransitObservation::Progress)
        );
        let pause_at = flow.now();
        bound
            .schedule_transit_control(&mut flow, FlowDomainControl::Pause, pause_at, 7)
            .unwrap();
        let pause = flow.step().unwrap().unwrap();
        assert_eq!(
            bound.observe_transit_dispatch(&flow, &pause),
            Ok(TransitObservation::Paused)
        );
        bound.service_stream.next_u64().unwrap();
        let limits = bridge_checkpoint_limits();
        let image =
            BoundIntrinsicWorkCheckpointV1::capture(&bound, &flow, &adapter, limits).unwrap();
        assert!(image.draw_after < bound.draw_position());
        let graph = route_graph(&bound);
        let source_budget = flow.budget_snapshot();
        let source_position = bound.draw_position();
        let expected_next_draw = bound.next_service_draw_probe_for_checkpoint();
        let (mut restored_flow, rebind) = restore_route_flow_with_view(&flow, graph.clone());
        let restored_adapter =
            restore_adapter_for_flow(&adapter, &flow, &restored_flow, bound.work());
        let carrier_id = image.carrier.unwrap();
        assert_eq!(
            rebind.resolve_work_binding(carrier_id).unwrap(),
            (
                bound.carrier_actor.unwrap(),
                "bridge.transit",
                Some(bound.kind.unwrap())
            )
        );
        if let Some(pending) = image.pending_event {
            assert!(
                rebind.resolve_event(pending).is_ok(),
                "ordinary paused pending event should remain in scheduler membership: {pending:?}"
            );
        }
        let mapped_work = rebind.resolve_work(image.work).unwrap();
        let restored_spec = restored_flow.work(mapped_work).unwrap();
        assert_eq!(restored_flow.identity(), *rebind.identity());
        assert_eq!(
            restored_adapter.decision(mapped_work),
            Some(&image.decision)
        );
        assert_eq!(restored_spec.owner, image.acquire.owner);
        assert_eq!(
            restored_spec.original_duration.ticks(),
            image.duration_ticks
        );
        assert_eq!(
            restored_spec.request.is_some(),
            image.arrival_request.is_some()
        );
        let restored_acquire = image.acquire.restore(&rebind).unwrap();
        assert_eq!(restored_acquire.owner, restored_spec.owner);
        assert!(restored_flow.resource(restored_acquire.resource).is_ok());
        assert_eq!(image.carrier.is_some(), image.carrier_actor.is_some());
        assert_eq!(image.carrier.is_some(), image.kind.is_some());
        assert_eq!(
            image.pending_event.is_some(),
            image.pending_priority.is_some()
        );
        for event in &image.owned_events {
            assert!(rebind.resolve_issued_event(*event).is_ok());
        }
        for event in &image.consumed_events {
            assert!(rebind.resolve_issued_event(*event).is_ok());
        }
        for (event, _) in &image.controls {
            assert!(rebind.resolve_issued_event(*event).is_ok());
        }
        let restored_stream = image
            .stream
            .clone()
            .restore_for(
                &bound.expected_service_key,
                CalibrationStreamStateLimits {
                    max_identifier_bytes: limits.max_identifier_bytes,
                },
            )
            .unwrap();
        assert_eq!(
            SampledWorkDuration::from_checkpoint_parts(
                SimDuration::from_ticks(image.duration_ticks),
                bound.expected_service_key.clone(),
                image.draw_before,
                image.draw_after,
                &restored_stream,
                &bound.expected_service_key,
            )
            .unwrap()
            .duration()
            .ticks(),
            image.duration_ticks
        );
        let restored_transit = image
            .transit
            .clone()
            .restore(Some(graph.clone()), limits)
            .unwrap();
        let restored_carrier = rebind.resolve_work(carrier_id).unwrap();
        let restored_context = restored_flow
            .work_context::<TransitContext>(restored_carrier)
            .unwrap();
        assert!(validate_transit_context(
            &restored_transit,
            restored_context,
            &restored_flow,
            restored_carrier,
            mapped_work,
            Some(bound.carrier_actor.unwrap()),
            Some(bound.kind.unwrap()),
        )
        .is_ok());
        let mut restored = BoundIntrinsicWorkCheckpointV1::restore::<u32, u32>(
            image.clone(),
            &restored_flow,
            &restored_adapter,
            &bound.expected_service_key,
            &rebind,
            Some(graph.clone()),
            limits,
        )
        .unwrap();
        assert_eq!(restored.draw_position(), source_position);
        assert_eq!(
            restored.next_service_draw_probe_for_checkpoint(),
            expected_next_draw
        );
        assert_eq!(
            restored_flow
                .work_context::<TransitContext>(restored.carrier.unwrap())
                .unwrap()
                .phase(),
            TransitPhase::Paused
        );

        let mut wrong_actor = image.clone();
        wrong_actor.carrier_actor = Some(bound.acquire.owner);
        assert!(matches!(
            BoundIntrinsicWorkCheckpointV1::restore::<u32, u32>(
                wrong_actor,
                &restored_flow,
                &restored_adapter,
                &bound.expected_service_key,
                &rebind,
                Some(graph.clone()),
                limits,
            ),
            Err(BridgeCheckpointError::InvalidState)
        ));
        let mut wrong_both_actors = image.clone();
        wrong_both_actors.carrier_actor = Some(bound.acquire.owner);
        if let TransitRequestCheckpointV1::Route { carrier_actor, .. } =
            &mut wrong_both_actors.transit
        {
            *carrier_actor = bound.acquire.owner;
        }
        assert!(matches!(
            BoundIntrinsicWorkCheckpointV1::restore::<u32, u32>(
                wrong_both_actors,
                &restored_flow,
                &restored_adapter,
                &bound.expected_service_key,
                &rebind,
                Some(graph.clone()),
                limits,
            ),
            Err(BridgeCheckpointError::InvalidState)
        ));
        let mut wrong_kind = image.clone();
        let wrong_event_kind = EventKind::custom(0xC21);
        wrong_kind.kind = Some(wrong_event_kind);
        if let TransitRequestCheckpointV1::Route { kind, .. } = &mut wrong_kind.transit {
            *kind = wrong_event_kind;
        }
        assert!(matches!(
            BoundIntrinsicWorkCheckpointV1::restore::<u32, u32>(
                wrong_kind,
                &restored_flow,
                &restored_adapter,
                &bound.expected_service_key,
                &rebind,
                Some(graph.clone()),
                limits,
            ),
            Err(BridgeCheckpointError::InvalidState)
        ));
        let mut wrong_registration = image.clone();
        if let TransitRequestCheckpointV1::Route {
            carrier_registration,
            ..
        } = &mut wrong_registration.transit
        {
            *carrier_registration = "bridge.transit.other".to_owned();
        }
        assert!(matches!(
            BoundIntrinsicWorkCheckpointV1::restore::<u32, u32>(
                wrong_registration,
                &restored_flow,
                &restored_adapter,
                &bound.expected_service_key,
                &rebind,
                Some(graph.clone()),
                limits,
            ),
            Err(BridgeCheckpointError::InvalidState)
        ));

        assert_eq!(flow.budget_snapshot(), source_budget);
        assert_eq!(bound.draw_position(), source_position);
        assert_eq!(
            restored
                .schedule_transit_control(
                    &mut restored_flow,
                    FlowDomainControl::Resume,
                    SimTime::from_ticks(1),
                    7,
                )
                .unwrap(),
            bound
                .schedule_transit_control(
                    &mut flow,
                    FlowDomainControl::Resume,
                    SimTime::from_ticks(1),
                    7,
                )
                .unwrap()
        );
        loop {
            let source_dispatch = flow.step().unwrap().unwrap();
            let restored_dispatch = restored_flow.step().unwrap().unwrap();
            assert_eq!(source_dispatch, restored_dispatch);
            let source_observation = bound
                .observe_transit_dispatch(&flow, &source_dispatch)
                .unwrap();
            let restored_observation = restored
                .observe_transit_dispatch(&restored_flow, &restored_dispatch)
                .unwrap();
            assert_eq!(source_observation, restored_observation);
            if source_observation == TransitObservation::Arrived {
                break;
            }
        }
        assert_eq!(bound.owned_events, restored.owned_events);
        assert_eq!(bound.consumed_events, restored.consumed_events);
        assert_eq!(
            flow.work_progress(bound.work()).unwrap(),
            restored_flow.work_progress(restored.work()).unwrap()
        );
    }

    #[test]
    fn joined_native_restore_retries_a_rejected_issued_event_without_replay() {
        let max = SimTime::from_ticks(u128::MAX);
        let (mut flow, adapter, mut bound) = bound_route_at_with_adapter(max);
        let original_event = bound.start_transit(&mut flow).unwrap();
        let rejected = flow.step().unwrap().unwrap();
        assert_eq!(rejected.event, original_event);
        assert_overflow_rejection(&rejected);
        assert_eq!(
            bound.observe_transit_dispatch(&flow, &rejected),
            Ok(TransitObservation::Rejected)
        );
        assert_eq!(bound.retryable.as_ref(), Some(&rejected));

        let limits = bridge_checkpoint_limits();
        let image =
            BoundIntrinsicWorkCheckpointV1::capture(&bound, &flow, &adapter, limits).unwrap();
        let graph = route_graph(&bound);
        let source_budget = flow.budget_snapshot();
        let source_draw_position = bound.draw_position();
        let (mut restored_flow, rebind) = restore_route_flow_with_view(&flow, graph.clone());
        let restored_adapter =
            restore_adapter_for_flow(&adapter, &flow, &restored_flow, bound.work());
        let mut restored = BoundIntrinsicWorkCheckpointV1::restore::<u32, u32>(
            image.clone(),
            &restored_flow,
            &restored_adapter,
            &bound.expected_service_key,
            &rebind,
            Some(graph),
            limits,
        )
        .unwrap();
        assert_eq!(restored.retryable.as_ref(), Some(&rejected));
        assert_eq!(restored.pending_event, Some(original_event));
        assert_eq!(flow.budget_snapshot(), source_budget);
        assert_eq!(bound.draw_position(), source_draw_position);

        let restored_rejection = restored.retryable.clone().unwrap();
        let source_retry = bound.retry_transit(&mut flow, &rejected).unwrap();
        let restored_retry = restored
            .retry_transit(&mut restored_flow, &restored_rejection)
            .unwrap();
        assert_eq!(source_retry, restored_retry);
        let source_rejected_again = flow.step().unwrap().unwrap();
        let restored_rejected_again = restored_flow.step().unwrap().unwrap();
        assert_eq!(source_rejected_again, restored_rejected_again);
        assert_overflow_rejection(&source_rejected_again);
        assert_eq!(
            bound.observe_transit_dispatch(&flow, &source_rejected_again),
            Ok(TransitObservation::Rejected)
        );
        assert_eq!(
            restored.observe_transit_dispatch(&restored_flow, &restored_rejected_again),
            Ok(TransitObservation::Rejected)
        );
        assert_eq!(bound.owned_events, restored.owned_events);
        assert_eq!(bound.consumed_events, restored.consumed_events);
        assert_eq!(bound.retryable, restored.retryable);
    }

    #[test]
    fn joined_native_restore_preserves_arrival_and_historical_event_suffix() {
        let (mut flow, adapter, mut bound) = bound_annotated_route_with_adapter();
        bound.start_transit(&mut flow).unwrap();
        let start = flow.step().unwrap().unwrap();
        assert_eq!(
            bound.observe_transit_dispatch(&flow, &start),
            Ok(TransitObservation::Progress)
        );
        let pause_at = flow.now();
        bound
            .schedule_transit_control(&mut flow, FlowDomainControl::Pause, pause_at, 7)
            .unwrap();
        loop {
            let dispatch = flow.step().unwrap().unwrap();
            let observation = bound.observe_transit_dispatch(&flow, &dispatch).unwrap();
            if observation == TransitObservation::Paused {
                break;
            }
        }
        bound
            .schedule_transit_control(
                &mut flow,
                FlowDomainControl::Resume,
                SimTime::from_ticks(1),
                7,
            )
            .unwrap();
        loop {
            let dispatch = flow.step().unwrap().unwrap();
            let observation = bound.observe_transit_dispatch(&flow, &dispatch).unwrap();
            if observation == TransitObservation::Arrived {
                break;
            }
        }
        assert!(bound.arrival_request.is_some());
        assert!(!bound.consumed_events.is_empty());

        let limits = bridge_checkpoint_limits();
        let image =
            BoundIntrinsicWorkCheckpointV1::capture(&bound, &flow, &adapter, limits).unwrap();
        let graph = route_graph(&bound);
        let source_budget = flow.budget_snapshot();
        let source_history = (
            bound.owned_events.clone(),
            bound.stale_events.clone(),
            bound.consumed_events.clone(),
            bound.controls.clone(),
        );
        let (restored_flow, rebind) = restore_route_flow_with_view(&flow, graph.clone());
        let restored_adapter =
            restore_adapter_for_flow(&adapter, &flow, &restored_flow, bound.work());
        let restored = BoundIntrinsicWorkCheckpointV1::restore::<u32, u32>(
            image,
            &restored_flow,
            &restored_adapter,
            &bound.expected_service_key,
            &rebind,
            Some(graph),
            limits,
        )
        .unwrap();
        assert_eq!(restored.arrival_request, bound.arrival_request);
        assert_eq!(restored.arrival_at, bound.arrival_at);
        assert_eq!(restored.route_receipt, bound.route_receipt);
        assert_eq!(
            (
                restored.owned_events.clone(),
                restored.stale_events.clone(),
                restored.consumed_events.clone(),
                restored.controls.clone(),
            ),
            source_history
        );
        assert_eq!(flow.budget_snapshot(), source_budget);
        assert_eq!(
            restored_flow
                .work_context::<TransitContext>(restored.carrier.unwrap())
                .unwrap()
                .phase(),
            TransitPhase::Arrived
        );

        let source_submitted = bound.finish_transit(&flow).unwrap();
        let restored_submitted = restored.finish_transit(&restored_flow).unwrap();
        assert_eq!(source_submitted.work(), restored_submitted.work());
        assert_eq!(source_submitted.request(), restored_submitted.request());
        assert_eq!(source_submitted.decision(), restored_submitted.decision());
        assert_eq!(
            source_submitted.service_draw_position(),
            restored_submitted.service_draw_position()
        );
    }

    #[test]
    fn native_bound_checkpoint_keeps_approved_route_and_receipt_state() {
        let (mut flow, adapter, mut bound) = bound_annotated_route_with_adapter();
        bound.start_transit(&mut flow).unwrap();
        let image = BoundIntrinsicWorkCheckpointV1::capture(
            &bound,
            &flow,
            &adapter,
            bridge_checkpoint_limits(),
        )
        .unwrap();
        assert!(matches!(
            image.transit,
            TransitRequestCheckpointV1::Route { .. }
        ));
        assert!(image.route_metadata.is_some());
        assert!(image.route_receipt.is_some());
        assert_eq!(image.pending_event, bound.pending_event);
        assert_eq!(image.owned_events, bound.owned_events);
        let wire = image.encode_wire_v1(bridge_wire_limits()).unwrap();
        assert_eq!(
            BoundIntrinsicWorkCheckpointV1::decode_wire_v1(&wire, bridge_wire_limits()).unwrap(),
            image
        );

        let mut tiny = bridge_checkpoint_limits();
        tiny.max_canonical_bytes = 1;
        assert_eq!(
            BoundIntrinsicWorkCheckpointV1::capture(&bound, &flow, &adapter, tiny),
            Err(BridgeCheckpointError::LimitExceeded)
        );
        assert!(bound.route_receipt.is_some());
    }

    #[test]
    fn native_bound_checkpoint_rejects_carrier_request_and_flow_binding_mismatches() {
        let (mut flow, adapter, mut bound) = bound_route_with_adapter();
        bound.start_transit(&mut flow).unwrap();
        let limits = bridge_checkpoint_limits();
        let other_actor = flow.spawn_actor().unwrap();
        let actor = bound.carrier_actor.unwrap();
        let kind = bound.kind.unwrap();

        bound.carrier_actor = Some(other_actor);
        assert_eq!(
            BoundIntrinsicWorkCheckpointV1::capture(&bound, &flow, &adapter, limits),
            Err(BridgeCheckpointError::InvalidState)
        );
        bound.carrier_actor = Some(actor);

        bound.kind = Some(EventKind::custom(0xC21));
        assert_eq!(
            BoundIntrinsicWorkCheckpointV1::capture(&bound, &flow, &adapter, limits),
            Err(BridgeCheckpointError::InvalidState)
        );
        bound.kind = Some(kind);

        let original_registration = match &bound.transit {
            TransitRequest::Route {
                carrier_registration,
                ..
            } => carrier_registration.clone(),
            TransitRequest::Zero => panic!("route request expected"),
        };
        if let TransitRequest::Route {
            carrier_registration,
            ..
        } = &mut bound.transit
        {
            *carrier_registration = "bridge.transit.other".to_owned();
        }
        assert_eq!(
            BoundIntrinsicWorkCheckpointV1::capture(&bound, &flow, &adapter, limits),
            Err(BridgeCheckpointError::InvalidState)
        );
        if let TransitRequest::Route {
            carrier_registration,
            ..
        } = &mut bound.transit
        {
            *carrier_registration = original_registration;
        }

        if let TransitRequest::Route { carrier_actor, .. } = &mut bound.transit {
            *carrier_actor = other_actor;
        }
        bound.carrier_actor = Some(other_actor);
        assert_eq!(
            BoundIntrinsicWorkCheckpointV1::capture(&bound, &flow, &adapter, limits),
            Err(BridgeCheckpointError::InvalidState)
        );
        if let TransitRequest::Route { carrier_actor, .. } = &mut bound.transit {
            *carrier_actor = actor;
        }
        bound.carrier_actor = Some(actor);

        assert!(BoundIntrinsicWorkCheckpointV1::capture(&bound, &flow, &adapter, limits).is_ok());
    }

    #[test]
    fn native_bound_checkpoint_preflights_graph_limit_before_route_calculation() {
        let (flow, adapter, mut bound) = bound_route_with_adapter();
        let (graph_bytes, original_origin) = match &mut bound.transit {
            TransitRequest::Route { graph, origin, .. } => {
                let graph_bytes = graph.canonical_bytes_ref().len();
                let original_origin = *origin;
                *origin = NodeId::new(u64::MAX);
                (graph_bytes, original_origin)
            }
            TransitRequest::Zero => panic!("route request expected"),
        };
        let mut limits = bridge_checkpoint_limits();
        limits.max_canonical_bytes = graph_bytes - 1;
        let draw_position = bound.draw_position();
        let flow_budget = flow.budget_snapshot();

        assert_eq!(
            BoundIntrinsicWorkCheckpointV1::capture(&bound, &flow, &adapter, limits),
            Err(BridgeCheckpointError::LimitExceeded)
        );
        assert_eq!(bound.draw_position(), draw_position);
        assert_eq!(flow.budget_snapshot(), flow_budget);
        if let TransitRequest::Route { origin, .. } = &mut bound.transit {
            *origin = original_origin;
        }
    }

    #[test]
    fn native_transit_restore_preflights_trusted_graph_size_and_identity() {
        let (_flow, _adapter, bound) = bound_route_with_adapter();
        let image = TransitRequestCheckpointV1::capture(&bound.transit, bridge_checkpoint_limits())
            .unwrap();
        let TransitRequestCheckpointV1::Route {
            graph_canonical_bytes,
            ..
        } = &image
        else {
            panic!("route image expected");
        };
        let nodes = [1, 2, 3, 4].map(NodeId::new);
        let mode = MovementModeId::new("walk").unwrap();
        let larger_graph = Arc::new(
            TransitGraphV1::new(
                1,
                nodes.to_vec(),
                vec![
                    TransitEdge {
                        id: EdgeId::new(1),
                        from: nodes[0],
                        to: nodes[1],
                        length_mm: 1,
                        allowed_modes: vec![mode.clone()],
                    },
                    TransitEdge {
                        id: EdgeId::new(2),
                        from: nodes[2],
                        to: nodes[3],
                        length_mm: 1,
                        allowed_modes: vec![mode],
                    },
                ],
            )
            .unwrap(),
        );
        assert!(larger_graph.canonical_bytes_ref().len() > graph_canonical_bytes.len());
        let mut limits = bridge_checkpoint_limits();
        limits.max_canonical_bytes = graph_canonical_bytes.len();
        assert!(matches!(
            image.clone().restore(Some(larger_graph), limits),
            Err(BridgeCheckpointError::LimitExceeded)
        ));
    }

    #[test]
    fn native_submitted_checkpoint_retains_request_and_service_sample() {
        let (mut flow, adapter, bound) =
            bound_simple_with_adapter(FidelityMode::Macro, TransitIntent::Zero);
        let work = bound.work();
        let submitted = must_submit(bound.submit(&mut flow));
        let image = SubmittedIntrinsicWorkCheckpointV1::capture(
            &submitted,
            &flow,
            &adapter,
            bridge_checkpoint_limits(),
        )
        .unwrap();
        assert_eq!(image.work, work.entity_id());
        assert_eq!(image.request, submitted.request.entity_id());
        assert_eq!(image.duration_ticks, submitted.sample.duration().ticks());
        assert_eq!(image.draw_before, submitted.sample.draw_before());
        assert_eq!(image.draw_after, submitted.sample.draw_after());
        let wire = image.encode_wire_v1(bridge_wire_limits()).unwrap();
        let decoded =
            SubmittedIntrinsicWorkCheckpointV1::decode_wire_v1(&wire, bridge_wire_limits())
                .unwrap();
        assert_eq!(decoded, image);
        assert_eq!(decoded.work_entity_id(), image.work_entity_id());
        assert_eq!(decoded.stream_identity(), image.stream_identity());
    }

    #[test]
    fn native_submitted_checkpoint_restores_existing_request_without_resubmission() {
        let (mut flow, adapter, bound) =
            bound_simple_with_adapter(FidelityMode::Macro, TransitIntent::Zero);
        let work = bound.work();
        let submitted = must_submit(bound.submit(&mut flow));
        let image = SubmittedIntrinsicWorkCheckpointV1::capture(
            &submitted,
            &flow,
            &adapter,
            bridge_checkpoint_limits(),
        )
        .unwrap();
        let adapter_image = adapter
            .checkpoint(
                &flow,
                FidelityCheckpointLimits {
                    max_admitted: 32,
                    max_overrides: 32,
                    max_subsystem_bytes: 4096,
                },
            )
            .unwrap();
        let key = submitted.expected_service_key.clone();
        let expected_next = submitted
            .service_stream
            .snapshot()
            .restore()
            .unwrap()
            .next_u64()
            .unwrap();
        let (restored_flow, rebind) = restore_flow_with_test_rebind(&flow);
        let restored_adapter = FidelityAdapter::from_checkpoint(
            adapter_image,
            &restored_flow,
            &[(work, work)],
            FidelityCheckpointLimits {
                max_admitted: 32,
                max_overrides: 32,
                max_subsystem_bytes: 4096,
            },
        )
        .unwrap();
        let restored: SubmittedIntrinsicWork<u32, u32> =
            SubmittedIntrinsicWorkCheckpointV1::restore(
                image,
                &restored_flow,
                &restored_adapter,
                &key,
                &rebind,
                bridge_checkpoint_limits(),
            )
            .unwrap();
        assert_eq!(restored.work(), work);
        assert_eq!(restored.request(), submitted.request());
        assert_eq!(restored.draw_position(), submitted.draw_position());
        assert_eq!(
            restored
                .service_stream
                .snapshot()
                .restore()
                .unwrap()
                .next_u64()
                .unwrap(),
            expected_next
        );
        assert_eq!(
            restored_flow.request(restored.request()).unwrap().work,
            Some(work)
        );
    }

    #[test]
    fn invalid_route_metadata_rejects_before_admission_or_service_draw() {
        let invalid_metadata = [
            RouteMetadata::with_version(
                2,
                "patient-transfer",
                DistanceProvenance::ConfiguredGeometry,
            ),
            RouteMetadata::v1("", DistanceProvenance::ConfiguredGeometry),
            RouteMetadata::v1(
                "patient-transfer",
                DistanceProvenance::SensorObservationOnly,
            ),
        ];
        for metadata in invalid_metadata {
            let (input, flow, mut adapter, _) = input(
                FidelityMode::Micro,
                TransitIntent::Route,
                SeedPurpose::Service,
                false,
            );
            let before_draw = input.service_stream.draw_position();
            let before_budget = flow.budget_snapshot();
            adapter.stage_policy(FidelityPolicy::new(1, Some(FidelityMode::Macro)).unwrap());
            let failure =
                match input
                    .with_route_metadata(metadata)
                    .prepare(&flow, &mut adapter, &provider())
                {
                    Err(failure) => failure,
                    Ok(_) => panic!("invalid route metadata must reject preparation"),
                };
            assert!(matches!(failure.error, BridgeError::RouteReceipt(_)));
            assert_eq!(failure.input.service_stream.draw_position(), before_draw);
            assert_eq!(flow.budget_snapshot(), before_budget);
            assert_eq!(adapter.apply_at_boundary(&flow), Ok(()));
        }
    }

    #[test]
    fn route_metadata_mismatch_preserves_bound_state_and_rejects_continuation() {
        let (mut flow, adapter, mut bound) = bound_annotated_route_with_adapter();
        let work = bound.work();
        let event = bound.start_transit(&mut flow).unwrap();
        let receipt = bound.route_receipt.clone().unwrap();
        let draw_position = bound.service_draw_position();
        let progress = flow.work_progress(work).unwrap();
        let budget = flow.budget_snapshot();
        let service_request = flow.work(work).unwrap().request;
        bound.route_metadata = Some(configured_route_metadata("different-purpose"));
        let now = flow.now();

        assert_eq!(
            bound.schedule_transit_control(&mut flow, FlowDomainControl::Pause, now, 7,),
            Err(BridgeError::RouteReceipt(RouteReceiptError::RouteMismatch))
        );
        assert_eq!(bound.pending_event, Some(event));
        assert_eq!(bound.route_receipt, Some(receipt.clone()));
        assert_eq!(bound.service_draw_position(), draw_position);
        assert_eq!(flow.work_progress(work).unwrap(), progress);
        assert_eq!(flow.work(work).unwrap().request, service_request);
        assert_eq!(flow.budget_snapshot(), budget);

        let (returned_flow, returned_adapter, returned_bound, error) =
            match BoundWorkContinuation::capture(flow, adapter, bound) {
                Err(values) => values,
                Ok(_) => panic!("mismatched route purpose must reject continuation capture"),
            };
        assert_eq!(
            error,
            BridgeError::RouteReceipt(RouteReceiptError::RouteMismatch)
        );
        assert_eq!(returned_bound.route_receipt, Some(receipt));
        assert_eq!(returned_bound.service_draw_position(), draw_position);
        assert_eq!(returned_flow.work_progress(work).unwrap(), progress);
        assert_eq!(returned_flow.work(work).unwrap().request, service_request);
        assert_eq!(returned_flow.budget_snapshot(), budget);
        assert_eq!(
            returned_adapter.decision(work),
            Some(&returned_bound.decision)
        );
    }

    fn bound_route_at(
        start: SimTime,
        edge_count: u32,
    ) -> (BoundIntrinsicWork<u32, u32>, FlowRuntime) {
        bound_route_at_with_registration(start, edge_count, register_transit_context)
    }

    fn bound_route_at_with_registration(
        start: SimTime,
        edge_count: u32,
        register: fn(&mut FlowRuntime, &str, EventKind) -> Result<(), FlowError>,
    ) -> (BoundIntrinsicWork<u32, u32>, FlowRuntime) {
        let (mut input, mut flow, mut adapter, _) = input(
            FidelityMode::Micro,
            TransitIntent::Route,
            SeedPurpose::Service,
            false,
        );
        input.acquire.at = start;
        let carrier_actor = match &input.transit {
            TransitRequest::Route { carrier_actor, .. } => *carrier_actor,
            TransitRequest::Zero => unreachable!(),
        };
        let nodes = (1..=u64::from(edge_count) + 1)
            .map(NodeId::new)
            .collect::<Vec<_>>();
        let mode = MovementModeId::new("walk").unwrap();
        let edges = (0..edge_count)
            .map(|index| TransitEdge {
                id: EdgeId::new(index as u64 + 1),
                from: nodes[index as usize],
                to: nodes[index as usize + 1],
                length_mm: 1,
                allowed_modes: vec![mode.clone()],
            })
            .collect();
        input.transit = TransitRequest::Route {
            graph: Arc::new(TransitGraphV1::new(1, nodes.clone(), edges).unwrap()),
            origin: nodes[0],
            destination: *nodes.last().unwrap(),
            profile: MovementProfile::new("walk", 1).unwrap(),
            ticks_per_second: 1,
            carrier_actor,
            carrier_registration: "bridge.transit".to_owned(),
            kind: EventKind::custom(0xC20),
        };
        register(&mut flow, "bridge.transit", EventKind::custom(0xC20)).unwrap();
        flow.register_work_handlers("bridge.context", WorkHandlers::<u32>::default())
            .unwrap();
        let prepared = must_prepare(input.prepare(&flow, &mut adapter, &provider()));
        let created = must_create(prepared.create(&mut flow));
        (must_bind(created.bind(&flow)), flow)
    }

    fn assert_overflow_rejection(dispatch: &FlowDispatch) {
        assert_eq!(dispatch.error, Some(FlowError::CounterOverflow));
        assert!(matches!(
            dispatch.callback_batches.as_slice(),
            [FlowBatchReceipt::Rejected(rejection)]
                if rejection.error == FlowError::CounterOverflow
                    && rejection.failed_ticket.is_none()
        ));
    }

    #[test]
    fn owning_continuation_preserves_macro_and_zero_micro_values_and_service_draws() {
        for (mode, transit) in [
            (FidelityMode::Macro, TransitIntent::Route),
            (FidelityMode::Micro, TransitIntent::Zero),
        ] {
            let (flow, mut adapter, bound) = bound_simple_with_metadata(mode, transit);
            let (control_flow, mut control_adapter, mut control_bound) =
                bound_simple_with_adapter(mode, transit);
            assert!(bound.route_receipt.is_none());
            assert_eq!(
                flow.budget_snapshot().scheduler,
                control_flow.budget_snapshot().scheduler
            );
            assert_eq!(flow.budget_snapshot().scheduler.scheduled_events, 0);
            let identity = flow.identity();
            let work = bound.work();
            let progress = flow.work_progress(work).unwrap();
            let stream_position = bound.service_draw_position();
            adapter.stage_policy(FidelityPolicy::new(1, Some(FidelityMode::Micro)).unwrap());
            control_adapter
                .stage_policy(FidelityPolicy::new(1, Some(FidelityMode::Micro)).unwrap());

            let continuation = BoundWorkContinuation::capture(flow, adapter, bound)
                .ok()
                .expect("valid bound bridge values capture");
            let (flow, mut adapter, mut bound) = continuation.resume();
            assert_eq!(flow.identity(), identity);
            assert_eq!(flow.work_progress(work).unwrap(), progress);
            assert_eq!(adapter.decision(work), Some(&bound.decision));
            assert_eq!(bound.service_draw_position(), stream_position);
            assert_eq!(flow.budget_snapshot(), control_flow.budget_snapshot());
            assert_eq!(
                bound.service_stream.next_u64().unwrap(),
                control_bound.service_stream.next_u64().unwrap()
            );
            assert_eq!(
                bound.service_draw_position(),
                control_bound.service_draw_position()
            );
            assert_eq!(
                adapter.apply_at_boundary(&flow),
                Err(FidelityError::BusyBoundary)
            );
            assert_eq!(
                control_adapter.apply_at_boundary(&control_flow),
                Err(FidelityError::BusyBoundary)
            );
            assert_eq!(adapter.decision(work), control_adapter.decision(work));
        }
    }

    #[test]
    fn owning_continuation_preserves_started_and_observed_paused_transit() {
        let (mut flow, adapter, mut bound) = bound_annotated_route_with_adapter();
        let (mut control_flow, control_adapter, mut control_bound) = bound_route_with_adapter();
        let five_tick_graph = Arc::new(
            TransitGraphV1::new(
                1,
                vec![NodeId::new(1), NodeId::new(2)],
                vec![TransitEdge {
                    id: EdgeId::new(1),
                    from: NodeId::new(1),
                    to: NodeId::new(2),
                    length_mm: 5,
                    allowed_modes: vec![MovementModeId::new("walk").unwrap()],
                }],
            )
            .unwrap(),
        );
        for candidate in [&mut bound, &mut control_bound] {
            if let TransitRequest::Route { graph, .. } = &mut candidate.transit {
                *graph = Arc::clone(&five_tick_graph);
            } else {
                panic!("route fixture expected");
            }
        }
        let identity = flow.identity();
        let work = bound.work();
        let start = bound.start_transit(&mut flow).unwrap();
        let control_start = control_bound.start_transit(&mut control_flow).unwrap();
        assert_eq!(start, control_start);
        assert_eq!(control_bound.route_receipt, None);
        let expected_receipt = bound.route_receipt.clone().unwrap();
        assert!(bound
            .validate_route_context(&flow, bound.carrier.unwrap())
            .is_ok());

        let continuation = BoundWorkContinuation::capture(flow, adapter, bound)
            .ok()
            .expect("same-runtime bridge values capture after transit start");
        let (mut flow, adapter, mut bound) = continuation.resume();
        assert_eq!(flow.identity(), identity);
        assert_eq!(adapter.decision(work), Some(&bound.decision));
        assert_eq!(bound.pending_event, Some(start));
        assert_eq!(bound.route_receipt, Some(expected_receipt.clone()));
        assert_eq!(control_bound.route_receipt, None);
        assert_eq!(flow.budget_snapshot(), control_flow.budget_snapshot());

        let start_dispatch = flow.step().unwrap().unwrap();
        let control_start_dispatch = control_flow.step().unwrap().unwrap();
        assert_eq!(start_dispatch, control_start_dispatch);
        assert_eq!(
            bound.observe_transit_dispatch(&flow, &start_dispatch),
            Ok(TransitObservation::Progress)
        );
        assert_eq!(
            control_bound.observe_transit_dispatch(&control_flow, &control_start_dispatch),
            Ok(TransitObservation::Progress)
        );
        let replaced_progress_event = bound.pending_event.unwrap();
        let now = SimTime::from_ticks(1);
        let pause = bound
            .schedule_transit_control(&mut flow, FlowDomainControl::Pause, now, 7)
            .unwrap();
        let control_now = SimTime::from_ticks(1);
        let control_pause = control_bound
            .schedule_transit_control(&mut control_flow, FlowDomainControl::Pause, control_now, 7)
            .unwrap();
        assert_eq!(pause, control_pause);
        let resume = bound
            .schedule_transit_control(
                &mut flow,
                FlowDomainControl::Resume,
                SimTime::from_ticks(3),
                7,
            )
            .unwrap();
        let control_resume = control_bound
            .schedule_transit_control(
                &mut control_flow,
                FlowDomainControl::Resume,
                SimTime::from_ticks(3),
                7,
            )
            .unwrap();
        assert_eq!(resume, control_resume);
        let pause_dispatch = flow.step().unwrap().unwrap();
        let control_pause_dispatch = control_flow.step().unwrap().unwrap();
        assert_eq!(pause_dispatch, control_pause_dispatch);
        assert_eq!(
            bound.observe_transit_dispatch(&flow, &pause_dispatch),
            Ok(TransitObservation::Paused)
        );
        assert_eq!(
            control_bound.observe_transit_dispatch(&control_flow, &control_pause_dispatch),
            Ok(TransitObservation::Paused)
        );
        let carrier = bound.carrier.unwrap();
        let progress = flow.work_progress(work).unwrap();
        let context = flow.work_context::<TransitContext>(carrier).unwrap();
        assert_eq!(context.phase(), TransitPhase::Paused);
        let paused_progress = context.progress_at(flow.now()).unwrap();
        assert_eq!(paused_progress.useful_elapsed, SimDuration::from_ticks(1));
        assert_eq!(paused_progress.remaining, SimDuration::from_ticks(4));
        assert!(bound.validate_route_context(&flow, carrier).is_ok());
        let paused_image = BoundIntrinsicWorkCheckpointV1::capture(
            &bound,
            &flow,
            &adapter,
            bridge_checkpoint_limits(),
        )
        .unwrap();
        assert_eq!(paused_image.controls.len(), 1);
        assert_eq!(
            paused_image.route_receipt.as_ref(),
            Some(
                &bound
                    .route_receipt
                    .as_ref()
                    .unwrap()
                    .checkpoint_v1(RouteReceiptCheckpointLimits {
                        max_identifier_bytes: 4096,
                        max_canonical_bytes: 16 * 1024,
                    })
                    .unwrap()
            )
        );
        let control_context = control_flow
            .work_context::<TransitContext>(control_bound.carrier.unwrap())
            .unwrap();
        assert_eq!(context.phase(), control_context.phase());
        assert_eq!(
            context.progress_at(flow.now()),
            control_context.progress_at(flow.now())
        );

        let continuation = BoundWorkContinuation::capture(flow, adapter, bound)
            .ok()
            .expect("same-runtime bound bridge values capture after observed pause");
        let (mut flow, adapter, mut bound) = continuation.resume();
        assert_eq!(flow.identity(), identity);
        assert_eq!(flow.work_progress(work).unwrap(), progress);
        assert_eq!(bound.route_receipt, Some(expected_receipt.clone()));
        assert_eq!(adapter.decision(work), Some(&bound.decision));
        assert_eq!(bound.owned_events, control_bound.owned_events);
        assert_eq!(bound.consumed_events, control_bound.consumed_events);
        assert_eq!(flow.budget_snapshot(), control_flow.budget_snapshot());
        assert_eq!(
            bound.service_stream.next_u64().unwrap(),
            control_bound.service_stream.next_u64().unwrap()
        );

        // Both paused continuations resume through the same accepted arrival
        // callback exactly once, with matching bridge receipts and ownership.
        let dispatch = flow.step().unwrap().unwrap();
        let control_dispatch = control_flow.step().unwrap().unwrap();
        assert_eq!(dispatch, control_dispatch);
        assert_eq!(dispatch.event, resume);
        assert_eq!(
            bound.observe_transit_dispatch(&flow, &dispatch),
            Ok(TransitObservation::Resumed)
        );
        let resumed_context = flow.work_context::<TransitContext>(carrier).unwrap();
        assert_eq!(resumed_context.phase(), TransitPhase::Moving);
        let resumed_progress = resumed_context.progress_at(flow.now()).unwrap();
        assert_eq!(resumed_progress.useful_elapsed, SimDuration::from_ticks(1));
        assert_eq!(resumed_progress.remaining, SimDuration::from_ticks(4));
        assert_eq!(bound.route_receipt, Some(expected_receipt.clone()));
        assert_eq!(
            control_bound.observe_transit_dispatch(&control_flow, &control_dispatch),
            Ok(TransitObservation::Resumed)
        );
        let resumed_image = BoundIntrinsicWorkCheckpointV1::capture(
            &bound,
            &flow,
            &adapter,
            bridge_checkpoint_limits(),
        )
        .unwrap();
        assert_eq!(resumed_image.pending_event, bound.pending_event);
        assert_eq!(resumed_image.controls, bound.controls);
        let mut arrived = false;
        let mut saw_replaced_stale_event = false;
        while !arrived {
            let dispatch = flow.step().unwrap().unwrap();
            let control_dispatch = control_flow.step().unwrap().unwrap();
            assert_eq!(dispatch, control_dispatch);
            let observation = bound.observe_transit_dispatch(&flow, &dispatch).unwrap();
            let control_observation = control_bound
                .observe_transit_dispatch(&control_flow, &control_dispatch)
                .unwrap();
            assert_eq!(observation, control_observation);
            if dispatch.event == replaced_progress_event {
                assert_eq!(dispatch.at, SimTime::from_ticks(5));
                assert_eq!(observation, TransitObservation::IgnoredStale);
                saw_replaced_stale_event = true;
                let stale_image = BoundIntrinsicWorkCheckpointV1::capture(
                    &bound,
                    &flow,
                    &adapter,
                    bridge_checkpoint_limits(),
                )
                .unwrap();
                assert_eq!(stale_image.stale_events, vec![replaced_progress_event]);
                assert!(stale_image
                    .consumed_events
                    .contains(&replaced_progress_event));
            }
            arrived = observation == TransitObservation::Arrived;
        }
        assert!(saw_replaced_stale_event);
        assert_eq!(bound.stale_events, vec![replaced_progress_event]);
        assert!(bound.consumed_events.contains(&replaced_progress_event));
        assert_eq!(bound.owned_events.len(), 6);
        assert_eq!(bound.consumed_events.len(), 5);
        assert!(bound.arrival_request.is_some());
        assert_eq!(bound.route_receipt, Some(expected_receipt.clone()));
        assert_eq!(flow.now(), SimTime::from_ticks(7));
        let arrived_context = flow.work_context::<TransitContext>(carrier).unwrap();
        let arrived_progress = arrived_context.progress_at(flow.now()).unwrap();
        assert_eq!(arrived_progress.useful_elapsed, SimDuration::from_ticks(5));
        assert_eq!(arrived_progress.remaining, SimDuration::ZERO);
        assert!(bound
            .validate_route_context(&flow, bound.carrier.unwrap())
            .is_ok());
        let arrived_image = BoundIntrinsicWorkCheckpointV1::capture(
            &bound,
            &flow,
            &adapter,
            bridge_checkpoint_limits(),
        )
        .unwrap();
        assert_eq!(
            arrived_image.arrival_request,
            bound.arrival_request.map(RequestId::entity_id)
        );
        assert_eq!(arrived_image.arrival_at, bound.arrival_at);
        assert_eq!(bound.owned_events.len(), control_bound.owned_events.len());
        assert_eq!(
            bound.consumed_events.len(),
            control_bound.consumed_events.len()
        );
        assert_eq!(bound.owned_events, control_bound.owned_events);
        assert_eq!(bound.consumed_events, control_bound.consumed_events);
        assert_eq!(adapter.decision(work), control_adapter.decision(work));

        // Finish both arrived bridges and verify each returns the original
        // timed request without creating divergent work or progress state.
        assert_eq!(control_bound.route_receipt, None);
        let submitted = bound.finish_transit(&flow).unwrap();
        let control_submitted = control_bound.finish_transit(&control_flow).unwrap();
        assert_eq!(submitted.work(), control_submitted.work());
        assert_eq!(submitted.request(), control_submitted.request());
        assert_eq!(submitted.decision(), control_submitted.decision());
        assert_eq!(
            submitted.service_draw_position(),
            control_submitted.service_draw_position()
        );
        assert_eq!(submitted.work(), work);
        let request = flow.request(submitted.request()).unwrap();
        let control_request = control_flow.request(control_submitted.request()).unwrap();
        assert_eq!(request.work, Some(work));
        assert_eq!(request.work, control_request.work);
        assert_eq!(request.resource, control_request.resource);
        assert_eq!(request.owner, control_request.owner);
        assert!(request.timed);
        assert_eq!(request.timed, control_request.timed);
        assert_eq!(request.submitted_at, control_request.submitted_at);
        assert_eq!(
            flow.work_progress(work).unwrap(),
            control_flow.work_progress(work).unwrap()
        );
    }

    #[test]
    fn continuation_capture_mismatch_returns_the_original_values() {
        let (flow, adapter, bound) =
            bound_simple_with_adapter(FidelityMode::Macro, TransitIntent::Route);
        let original_identity = flow.identity();
        let original_work = bound.work();
        let foreign_flow = FlowRuntime::new();
        let foreign_identity = foreign_flow.identity();
        let result = BoundWorkContinuation::capture(foreign_flow, adapter, bound);
        let (returned_flow, returned_adapter, returned_bound, error) = match result {
            Err(values) => values,
            Ok(_) => panic!("foreign runtime must fail capture validation"),
        };
        assert_eq!(error, BridgeError::Fidelity(FidelityError::InvalidWork));
        assert_eq!(returned_flow.identity(), foreign_identity);
        assert_eq!(
            returned_adapter.decision(original_work),
            Some(&returned_bound.decision)
        );
        assert_eq!(returned_bound.runtime, original_identity);
    }

    #[test]
    fn continuation_capture_decision_mismatch_returns_original_values() {
        let (mut flow, mut adapter, mut bound) =
            bound_simple_with_adapter(FidelityMode::Macro, TransitIntent::Route);
        let work = bound.work();

        let blocker_actor = flow.spawn_actor().unwrap();
        let blocker_request = flow
            .acquire(bound.acquire.resource)
            .owner(blocker_actor)
            .submit()
            .unwrap();
        let _blocker_dispatch = flow.step().unwrap().expect("blocker submit is processed");
        assert_eq!(
            flow.request(blocker_request).unwrap().state,
            RequestState::Active
        );

        let waiter_actor = flow.spawn_actor().unwrap();
        let waiter_request = flow
            .acquire(bound.acquire.resource)
            .owner(waiter_actor)
            .submit()
            .unwrap();
        let _waiter_dispatch = flow.step().unwrap().expect("waiter submit is processed");
        assert_eq!(
            flow.request(waiter_request).unwrap().state,
            RequestState::Queued
        );

        let identity = flow.identity();
        let now = flow.now();
        let progress = flow.work_progress(work).unwrap();
        let budget = flow.budget_snapshot();
        let adapter_decision = *adapter.decision(work).unwrap();
        assert_eq!(adapter_decision.mode, FidelityMode::Macro);
        adapter.stage_policy(FidelityPolicy::new(1, Some(FidelityMode::Micro)).unwrap());
        assert_eq!(
            adapter.apply_at_boundary(&flow),
            Err(FidelityError::BusyBoundary)
        );

        let blocker_row = flow.request(blocker_request).unwrap();
        let waiter_row = flow.request(waiter_request).unwrap();
        let resource_snapshot = flow.resource(bound.acquire.resource).unwrap();
        assert_eq!(resource_snapshot.active.len(), 1);
        assert_eq!(resource_snapshot.queued, vec![waiter_request]);
        assert_eq!(resource_snapshot.allocations.len(), 1);
        let work_context = *flow.work_context::<u32>(work).unwrap();

        let runtime = bound.runtime.clone();
        let expected_service_key = bound.expected_service_key.clone();
        let service_stream_key = bound.service_stream.key();
        let service_seed = bound.service_stream.derived_seed();
        let service_draw_position = bound.service_stream.draw_position();
        let sample_duration = bound.sample.duration();
        let sample_draw_bounds = (bound.sample.draw_before(), bound.sample.draw_after());
        let acquire = bound.acquire.clone();
        let transit = match &bound.transit {
            TransitRequest::Zero => None,
            TransitRequest::Route {
                graph,
                origin,
                destination,
                profile,
                ticks_per_second,
                carrier_actor,
                carrier_registration,
                kind,
            } => Some((
                Arc::clone(graph),
                *origin,
                *destination,
                profile.clone(),
                *ticks_per_second,
                *carrier_actor,
                carrier_registration.clone(),
                *kind,
            )),
        };
        let carrier = bound.carrier;
        let carrier_actor = bound.carrier_actor;
        let kind = bound.kind;
        let pending_event = bound.pending_event;
        let pending_priority = bound.pending_priority;
        let owned_events = bound.owned_events.clone();
        let stale_events = bound.stale_events.clone();
        let consumed_events = bound.consumed_events.clone();
        let controls = bound.controls.clone();
        let retryable = bound.retryable.clone();
        let arrival_request = bound.arrival_request;
        let arrival_at = bound.arrival_at;

        // Isolate the decision predicate while retaining a valid same-runtime tuple.
        bound.decision.mode = FidelityMode::Micro;
        let mismatched_decision = bound.decision;
        assert_eq!(adapter.decision(work), Some(&adapter_decision));
        assert_ne!(Some(&mismatched_decision), adapter.decision(work));

        let mut control_bound =
            bound_simple_with_adapter(FidelityMode::Macro, TransitIntent::Route).2;
        let result = BoundWorkContinuation::capture(flow, adapter, bound);
        let (returned_flow, mut returned_adapter, mut returned_bound, error) = match result {
            Err(values) => values,
            Ok(_) => panic!("decision mismatch must fail capture validation"),
        };

        assert_eq!(error, BridgeError::Fidelity(FidelityError::InvalidWork));
        assert_eq!(returned_flow.identity(), identity);
        assert_eq!(returned_flow.now(), now);
        assert_eq!(returned_flow.work_progress(work).unwrap(), progress);
        assert_eq!(returned_flow.budget_snapshot(), budget);
        assert_eq!(returned_flow.request(blocker_request).unwrap(), blocker_row);
        assert_eq!(returned_flow.request(waiter_request).unwrap(), waiter_row);
        assert_eq!(
            returned_flow
                .resource(returned_bound.acquire.resource)
                .unwrap(),
            resource_snapshot
        );
        assert_eq!(
            *returned_flow.work_context::<u32>(work).unwrap(),
            work_context
        );
        assert_eq!(returned_adapter.decision(work), Some(&adapter_decision));
        assert_eq!(
            returned_adapter.apply_at_boundary(&returned_flow),
            Err(FidelityError::BusyBoundary)
        );

        assert_eq!(returned_bound.decision, mismatched_decision);
        assert_eq!(returned_bound.runtime, runtime);
        assert_eq!(returned_bound.work, work);
        assert_eq!(returned_bound.expected_service_key, expected_service_key);
        assert_eq!(returned_bound.service_stream.key(), service_stream_key);
        assert_eq!(returned_bound.service_stream.derived_seed(), service_seed);
        assert_eq!(
            returned_bound.service_stream.draw_position(),
            service_draw_position
        );
        assert_eq!(returned_bound.sample.duration(), sample_duration);
        assert_eq!(
            (
                returned_bound.sample.draw_before(),
                returned_bound.sample.draw_after()
            ),
            sample_draw_bounds
        );
        assert_eq!(returned_bound.acquire, acquire);
        match (&returned_bound.transit, transit) {
            (TransitRequest::Zero, None) => {}
            (
                TransitRequest::Route {
                    graph,
                    origin,
                    destination,
                    profile,
                    ticks_per_second,
                    carrier_actor,
                    carrier_registration,
                    kind,
                },
                Some((
                    expected_graph,
                    expected_origin,
                    expected_destination,
                    expected_profile,
                    expected_ticks_per_second,
                    expected_carrier_actor,
                    expected_registration,
                    expected_kind,
                )),
            ) => {
                assert!(Arc::ptr_eq(graph, &expected_graph));
                assert_eq!(*origin, expected_origin);
                assert_eq!(*destination, expected_destination);
                assert_eq!(profile, &expected_profile);
                assert_eq!(*ticks_per_second, expected_ticks_per_second);
                assert_eq!(*carrier_actor, expected_carrier_actor);
                assert_eq!(carrier_registration, &expected_registration);
                assert_eq!(*kind, expected_kind);
            }
            _ => panic!("capture changed the original transit request"),
        }
        assert_eq!(returned_bound.carrier, carrier);
        assert_eq!(returned_bound.carrier_actor, carrier_actor);
        assert_eq!(returned_bound.kind, kind);
        assert_eq!(returned_bound.pending_event, pending_event);
        assert_eq!(returned_bound.pending_priority, pending_priority);
        assert_eq!(returned_bound.owned_events, owned_events);
        assert_eq!(returned_bound.stale_events, stale_events);
        assert_eq!(returned_bound.consumed_events, consumed_events);
        assert_eq!(returned_bound.controls, controls);
        assert_eq!(returned_bound.retryable, retryable);
        assert_eq!(returned_bound.arrival_request, arrival_request);
        assert_eq!(returned_bound.arrival_at, arrival_at);
        assert_eq!(
            returned_bound.service_stream.next_u64().unwrap(),
            control_bound.service_stream.next_u64().unwrap()
        );
        assert_eq!(
            returned_bound.service_stream.draw_position(),
            control_bound.service_stream.draw_position()
        );
    }

    #[test]
    fn macro_and_zero_micro_share_service_draws_and_submit_actual_timed_work() {
        let (macro_input, mut macro_flow, mut macro_adapter, macro_resource) = input(
            FidelityMode::Macro,
            TransitIntent::Route,
            SeedPurpose::Service,
            false,
        );
        let macro_prepared =
            must_prepare(macro_input.prepare(&macro_flow, &mut macro_adapter, &provider()));
        assert_eq!(macro_prepared.decision().mode, FidelityMode::Macro);
        assert_eq!(macro_prepared.draw_bounds().0, 0);
        assert_eq!(macro_prepared.draw_bounds().1, 1);
        macro_flow
            .register_work_handlers("bridge.context", WorkHandlers::<u32>::default())
            .unwrap();
        let macro_work = must_create(macro_prepared.create(&mut macro_flow));
        assert_eq!(macro_work.decision().mode, FidelityMode::Macro);
        assert_eq!(macro_flow.budget_snapshot().scheduler.scheduled_events, 0);
        let macro_bound = must_bind(macro_work.bind(&macro_flow));
        assert_eq!(macro_bound.decision().mode, FidelityMode::Macro);
        assert!(macro_bound.service_identity_matches());
        assert_eq!(macro_bound.acquire_intent().scheduler_priority, 7);
        assert_eq!(macro_flow.budget_snapshot().scheduler.scheduled_events, 0);
        let macro_submitted = must_submit(macro_bound.submit(&mut macro_flow));
        assert_eq!(macro_flow.budget_snapshot().scheduler.scheduled_events, 1);

        let (micro_input, mut micro_flow, mut micro_adapter, micro_resource) = input(
            FidelityMode::Micro,
            TransitIntent::Zero,
            SeedPurpose::Service,
            false,
        );
        let micro_prepared =
            must_prepare(micro_input.prepare(&micro_flow, &mut micro_adapter, &provider()));
        assert_eq!(micro_prepared.decision().mode, FidelityMode::Micro);
        assert_eq!(micro_prepared.draw_bounds(), (0, 1));
        micro_flow
            .register_work_handlers("bridge.context", WorkHandlers::<u32>::default())
            .unwrap();
        let micro_work = must_create(micro_prepared.create(&mut micro_flow));
        assert_eq!(micro_work.decision().mode, FidelityMode::Micro);
        assert_eq!(micro_flow.budget_snapshot().scheduler.scheduled_events, 0);
        let micro_bound = must_bind(micro_work.bind(&micro_flow));
        assert_eq!(micro_bound.decision().mode, FidelityMode::Micro);
        assert_eq!(micro_flow.budget_snapshot().scheduler.scheduled_events, 0);
        let micro_submitted = must_submit(micro_bound.submit(&mut micro_flow));
        assert_eq!(micro_flow.budget_snapshot().scheduler.scheduled_events, 1);

        assert_eq!(
            macro_submitted.sampled_duration(),
            micro_submitted.sampled_duration()
        );
        assert_eq!(
            macro_submitted.draw_position(),
            micro_submitted.draw_position()
        );
        assert_eq!(
            macro_submitted.expected_service_key,
            micro_submitted.expected_service_key
        );
        assert_eq!(
            macro_submitted.service_stream.derived_seed(),
            micro_submitted.service_stream.derived_seed()
        );
        assert_eq!(
            macro_submitted.expected_service_key,
            macro_submitted.service_stream.key()
        );
        assert_eq!(
            micro_submitted.expected_service_key,
            micro_submitted.service_stream.key()
        );
        assert!(macro_submitted.service_identity_matches());
        assert!(micro_submitted.service_identity_matches());
        assert_eq!(macro_submitted.decision().mode, FidelityMode::Macro);
        assert_eq!(micro_submitted.decision().mode, FidelityMode::Micro);
        assert_eq!(macro_submitted.acquire_intent().scheduler_priority, 7);
        assert_eq!(micro_submitted.acquire_intent().scheduler_priority, 7);
        assert_eq!(macro_flow.budget_snapshot().scheduler.scheduled_events, 1);
        assert_eq!(micro_flow.budget_snapshot().scheduler.scheduled_events, 1);

        let macro_request = {
            let flow = &mut macro_flow;
            let spec = flow.work(macro_submitted.work()).unwrap();
            assert_eq!(spec.context_type_key, "bridge.context");
            assert_eq!(spec.original_duration, macro_submitted.sampled_duration());
            assert_eq!(spec.request, Some(macro_submitted.request()));
            let request = flow.request(macro_submitted.request()).unwrap();
            assert_eq!(request.resource, macro_resource);
            assert_eq!(request.owner, spec.owner);
            assert_eq!(request.work, Some(macro_submitted.work()));
            assert!(request.timed);
            assert_eq!(request.priority_level, 3);
            assert_eq!(request.deadline, None);
            assert!(!request.can_preempt);
            assert_eq!(request.state, RequestState::Pending);
            assert!(flow.step().unwrap().is_some());
            assert_eq!(
                flow.request(macro_submitted.request()).unwrap().state,
                RequestState::Active
            );
            request
        };
        let micro_request = {
            let flow = &mut micro_flow;
            let spec = flow.work(micro_submitted.work()).unwrap();
            assert_eq!(spec.context_type_key, "bridge.context");
            assert_eq!(spec.original_duration, micro_submitted.sampled_duration());
            assert_eq!(spec.request, Some(micro_submitted.request()));
            let request = flow.request(micro_submitted.request()).unwrap();
            assert_eq!(request.resource, micro_resource);
            assert_eq!(request.owner, spec.owner);
            assert_eq!(request.work, Some(micro_submitted.work()));
            assert!(request.timed);
            assert_eq!(request.priority_level, 3);
            assert_eq!(request.deadline, None);
            assert!(!request.can_preempt);
            assert_eq!(request.state, RequestState::Pending);
            assert!(flow.step().unwrap().is_some());
            assert_eq!(
                flow.request(micro_submitted.request()).unwrap().state,
                RequestState::Active
            );
            request
        };
        assert_eq!(macro_request, micro_request);

        let macro_run = macro_flow.run_for(10).unwrap();
        let micro_run = micro_flow.run_for(10).unwrap();
        assert!(!macro_run.budget_exhausted);
        assert!(!micro_run.budget_exhausted);
        assert_eq!(macro_run, micro_run);
        assert_eq!(macro_flow.now(), micro_flow.now());
        assert_eq!(
            macro_flow.now(),
            SimTime::from_ticks(0)
                .checked_add(macro_submitted.sampled_duration())
                .unwrap()
        );
        assert_eq!(
            macro_flow.work_progress(macro_submitted.work()).unwrap(),
            micro_flow.work_progress(micro_submitted.work()).unwrap()
        );
        assert_eq!(
            macro_flow
                .work_progress(macro_submitted.work())
                .unwrap()
                .state,
            WorkState::Completed
        );
        assert_eq!(
            macro_flow.request(macro_submitted.request()).unwrap().state,
            RequestState::Completed
        );
        assert_eq!(
            macro_flow.request(macro_submitted.request()).unwrap(),
            micro_flow.request(micro_submitted.request()).unwrap()
        );
        assert_eq!(
            macro_flow.budget_snapshot().scheduler.scheduled_events,
            micro_flow.budget_snapshot().scheduler.scheduled_events
        );
        assert_eq!(macro_flow.budget_snapshot().scheduler.scheduled_events, 2);
        // TransitContext contains deterministic route progress and command tickets,
        // but no CalibrationStream or other RNG state. This path submits only the
        // Service-purpose stream and schedules exactly the two Flow events above.
    }

    #[test]
    fn mixed_subsystems_keep_macro_and_routed_micro_work_associated_in_one_flow() {
        let t = SimTime::from_ticks;
        let mut flow = FlowRuntime::new();
        let owner = flow.spawn_actor().unwrap();
        let carrier_actor = flow.spawn_actor().unwrap();
        let macro_resource = flow.create_resource(1).unwrap();
        let micro_resource = flow.create_resource(1).unwrap();

        let mut policy = FidelityPolicy::new(1, None).unwrap();
        policy
            .set_subsystem("assessment.macro", FidelityMode::Macro)
            .unwrap();
        policy
            .set_subsystem("assessment.micro", FidelityMode::Micro)
            .unwrap();
        let mut adapter = FidelityAdapter::new(policy);

        let mut seeds = CalibrationSeedMap::new(1, "mixed-subsystem", 37).unwrap();
        let macro_key = seeds
            .key_for(
                "mixed-subsystem-v1",
                0,
                "case-1",
                "macro-task",
                SeedPurpose::Service,
            )
            .unwrap();
        let macro_stream = seeds
            .stream_for(
                "mixed-subsystem-v1",
                0,
                "case-1",
                "macro-task",
                SeedPurpose::Service,
            )
            .unwrap();
        let micro_key = seeds
            .key_for(
                "mixed-subsystem-v1",
                0,
                "case-1",
                "micro-task",
                SeedPurpose::Service,
            )
            .unwrap();
        let micro_stream = seeds
            .stream_for(
                "mixed-subsystem-v1",
                0,
                "case-1",
                "micro-task",
                SeedPurpose::Service,
            )
            .unwrap();

        let node = |id| NodeId::new(id);
        let walk = MovementModeId::new("walk").unwrap();
        let nodes = (1..=3).map(node).collect::<Vec<_>>();
        let edges = (0..2)
            .map(|index| TransitEdge {
                id: EdgeId::new(index + 1),
                from: nodes[index as usize],
                to: nodes[index as usize + 1],
                length_mm: 1,
                allowed_modes: vec![walk.clone()],
            })
            .collect();
        let graph = Arc::new(TransitGraphV1::new(1, nodes.clone(), edges).unwrap());
        let transit = || TransitRequest::Route {
            graph: Arc::clone(&graph),
            origin: nodes[0],
            destination: *nodes.last().unwrap(),
            profile: MovementProfile::new("walk", 1).unwrap(),
            ticks_per_second: 1,
            carrier_actor,
            carrier_registration: "bridge.transit".to_owned(),
            kind: EventKind::custom(0xC20),
        };
        let acquire = |resource| AcquireIntent {
            resource,
            owner,
            at: t(0),
            priority_level: 3,
            deadline: None,
            scheduler_priority: 7,
            can_preempt: false,
            preemptible: None,
        };
        let macro_input = WorkPreparationInput::new(
            PreparationIdentity {
                owner,
                subsystem: "assessment.macro".to_owned(),
                registration: "bridge.context".to_owned(),
                stratum: "triage".to_owned(),
            },
            macro_stream,
            macro_key.clone(),
            42,
            make_context,
            acquire(macro_resource),
            transit(),
        );
        let micro_input = WorkPreparationInput::new(
            PreparationIdentity {
                owner,
                subsystem: "assessment.micro".to_owned(),
                registration: "bridge.context".to_owned(),
                stratum: "triage".to_owned(),
            },
            micro_stream,
            micro_key.clone(),
            43,
            make_context,
            acquire(micro_resource),
            transit(),
        );

        flow.register_work_handlers("bridge.context", WorkHandlers::<u32>::default())
            .unwrap();
        register_transit_context(&mut flow, "bridge.transit", EventKind::custom(0xC20)).unwrap();

        let macro_prepared = must_prepare(macro_input.prepare(&flow, &mut adapter, &provider()));
        assert_eq!(macro_prepared.decision().mode, FidelityMode::Macro);
        let macro_work = must_create(macro_prepared.create(&mut flow));
        let macro_work_id = macro_work.work();
        let mut macro_bound = must_bind(macro_work.bind(&flow));
        assert_eq!(
            adapter.decision(macro_work_id).unwrap().mode,
            FidelityMode::Macro
        );
        assert_eq!(macro_bound.carrier, None);
        assert!(macro_bound.owned_events.is_empty());
        let macro_draw_position = macro_bound.service_stream.draw_position();
        let scheduled_events_before_macro_rejection =
            flow.budget_snapshot().scheduler.scheduled_events;
        assert_eq!(
            macro_bound.start_transit(&mut flow),
            Err(BridgeError::InvalidDispatch)
        );
        assert_eq!(
            flow.budget_snapshot().scheduler.scheduled_events,
            scheduled_events_before_macro_rejection
        );
        assert_eq!(macro_bound.carrier, None);
        assert!(macro_bound.owned_events.is_empty());
        assert_eq!(
            macro_bound.service_stream.draw_position(),
            macro_draw_position
        );
        let macro_submitted = must_submit(macro_bound.submit(&mut flow));
        let macro_request_id = macro_submitted.request();
        assert_eq!(macro_submitted.decision().mode, FidelityMode::Macro);
        assert_eq!(macro_submitted.expected_service_key, macro_key);
        assert!(macro_submitted.service_identity_matches());
        assert_eq!(macro_submitted.draw_position(), 1);
        assert_eq!(
            flow.work(macro_work_id).unwrap().request,
            Some(macro_request_id)
        );
        let macro_request = flow.request(macro_request_id).unwrap();
        assert_eq!(macro_request.work, Some(macro_work_id));
        assert_eq!(macro_request.resource, macro_resource);
        assert!(macro_request.timed);
        assert_eq!(macro_request.submitted_at, t(0));

        let micro_prepared = must_prepare(micro_input.prepare(&flow, &mut adapter, &provider()));
        assert_eq!(micro_prepared.decision().mode, FidelityMode::Micro);
        let micro_work = must_create(micro_prepared.create(&mut flow));
        let micro_work_id = micro_work.work();
        let mut micro_bound = must_bind(micro_work.bind(&flow));
        assert_eq!(
            adapter.decision(micro_work_id).unwrap().mode,
            FidelityMode::Micro
        );
        assert_eq!(micro_bound.carrier, None);
        assert!(micro_bound.owned_events.is_empty());
        assert_eq!(flow.work(micro_work_id).unwrap().request, None);
        let micro_start = micro_bound.start_transit(&mut flow).unwrap();
        let micro_carrier = micro_bound.carrier.expect("Micro route owns a carrier");
        assert_eq!(micro_bound.owned_events, vec![micro_start]);
        assert_eq!(
            flow.work_context::<TransitContext>(micro_carrier)
                .unwrap()
                .service_work(),
            micro_work_id
        );
        assert_eq!(
            flow.resource(micro_resource).unwrap().queued,
            Vec::<RequestId>::new()
        );
        assert_eq!(flow.resource(micro_resource).unwrap().allocations.len(), 0);

        let mut observed = Vec::new();
        loop {
            let dispatch = flow.step().unwrap().expect("Micro route must arrive");
            if micro_bound.pending_event == Some(dispatch.event) {
                let observation = micro_bound
                    .observe_transit_dispatch(&flow, &dispatch)
                    .unwrap();
                observed.push((dispatch.at, observation));
                if observation == TransitObservation::Arrived {
                    break;
                }
            }
        }
        assert_eq!(
            observed,
            vec![
                (t(0), TransitObservation::Progress),
                (t(1), TransitObservation::Progress),
                (t(2), TransitObservation::Arrived),
            ]
        );
        let micro_request_id = flow.work(micro_work_id).unwrap().request.unwrap();
        let micro_request = flow.request(micro_request_id).unwrap();
        assert_eq!(micro_request.work, Some(micro_work_id));
        assert_eq!(micro_request.resource, micro_resource);
        assert!(micro_request.timed);
        assert_eq!(micro_request.submitted_at, t(2));
        assert_eq!(micro_bound.arrival_request, Some(micro_request_id));
        let micro_submitted = micro_bound.finish_transit(&flow).unwrap();
        assert_eq!(micro_submitted.work(), micro_work_id);
        assert_eq!(micro_submitted.request(), micro_request_id);
        assert_eq!(micro_submitted.decision().mode, FidelityMode::Micro);
        assert_eq!(micro_submitted.expected_service_key, micro_key);
        assert!(micro_submitted.service_identity_matches());
        assert_eq!(micro_submitted.draw_position(), 1);
        assert_eq!(
            adapter.decision(macro_work_id).unwrap().mode,
            FidelityMode::Macro
        );
        assert_eq!(
            adapter.decision(micro_work_id).unwrap().mode,
            FidelityMode::Micro
        );

        let run = flow.run_for(100).unwrap();
        assert!(!run.budget_exhausted);
        for (work, request) in [
            (macro_work_id, macro_request_id),
            (micro_work_id, micro_request_id),
        ] {
            assert_eq!(flow.work(work).unwrap().request, Some(request));
            assert_eq!(flow.request(request).unwrap().work, Some(work));
            assert_eq!(
                flow.request(request).unwrap().state,
                RequestState::Completed
            );
            assert_eq!(
                flow.work_progress(work).unwrap().state,
                WorkState::Completed
            );
        }
        assert_eq!(flow.resource(macro_resource).unwrap().allocations.len(), 0);
        assert_eq!(flow.resource(micro_resource).unwrap().allocations.len(), 0);
    }

    #[test]
    fn macro_and_micro_reconcile_queue_transit_and_service_elapsed_time() {
        let t = SimTime::from_ticks;
        let d = SimDuration::from_ticks;

        let (macro_input, mut macro_flow, mut macro_adapter, macro_resource) = input(
            FidelityMode::Macro,
            TransitIntent::Route,
            SeedPurpose::Service,
            false,
        );
        let macro_blocker = macro_flow.spawn_actor().unwrap();
        let macro_blocker_request = macro_flow
            .acquire(macro_resource)
            .owner(macro_blocker)
            .at(t(0))
            .submit()
            .unwrap();
        macro_flow.step().unwrap().unwrap();
        let macro_blocker_lease = macro_flow.resource(macro_resource).unwrap().allocations[0].lease;
        assert_eq!(
            macro_flow.request(macro_blocker_request).unwrap().state,
            RequestState::Active
        );

        let macro_prepared =
            must_prepare(macro_input.prepare(&macro_flow, &mut macro_adapter, &provider()));
        macro_flow
            .register_work_handlers("bridge.context", WorkHandlers::<u32>::default())
            .unwrap();
        let macro_work = must_create(macro_prepared.create(&mut macro_flow));
        let macro_bound = must_bind(macro_work.bind(&macro_flow));
        let macro_submitted = must_submit(macro_bound.submit(&mut macro_flow));
        let service_duration = macro_submitted.sampled_duration();
        assert_eq!(
            macro_submitted.service_stream.derived_seed(),
            0x4e65_a0fe_ae85_3a9b
        );
        assert_eq!(service_duration, d(20));
        macro_flow.release(macro_blocker_lease, t(5)).unwrap();
        macro_flow.step().unwrap().unwrap();
        let macro_request = macro_flow.request(macro_submitted.request()).unwrap();
        let macro_submitted_at = macro_request.submitted_at;
        let macro_request_state = macro_request.state;
        assert_eq!(macro_submitted_at, t(0));
        assert_eq!(macro_request_state, RequestState::Queued);
        macro_flow.step().unwrap().unwrap();
        let macro_allocation = macro_flow
            .resource(macro_resource)
            .unwrap()
            .allocations
            .into_iter()
            .find(|allocation| allocation.request == macro_submitted.request())
            .unwrap();
        assert_eq!(macro_allocation.granted_at, t(5));
        let macro_completion = macro_allocation.completion_at.unwrap();
        assert_eq!(macro_flow.step().unwrap().unwrap().at, macro_completion);
        assert_eq!(
            macro_flow
                .work_progress(macro_submitted.work())
                .unwrap()
                .state,
            WorkState::Completed
        );

        let (mut micro_input, mut micro_flow, mut micro_adapter, micro_resource) = input(
            FidelityMode::Micro,
            TransitIntent::Route,
            SeedPurpose::Service,
            false,
        );
        let carrier_actor = match &micro_input.transit {
            TransitRequest::Route { carrier_actor, .. } => *carrier_actor,
            TransitRequest::Zero => unreachable!(),
        };
        let nodes = (1..=3).map(NodeId::new).collect::<Vec<_>>();
        let walk = MovementModeId::new("walk").unwrap();
        let edges = (0..2)
            .map(|index| TransitEdge {
                id: EdgeId::new(index + 1),
                from: nodes[index as usize],
                to: nodes[index as usize + 1],
                length_mm: 1,
                allowed_modes: vec![walk.clone()],
            })
            .collect();
        micro_input.transit = TransitRequest::Route {
            graph: Arc::new(TransitGraphV1::new(1, nodes.clone(), edges).unwrap()),
            origin: nodes[0],
            destination: *nodes.last().unwrap(),
            profile: MovementProfile::new("walk", 1).unwrap(),
            ticks_per_second: 1,
            carrier_actor,
            carrier_registration: "bridge.transit".to_owned(),
            kind: EventKind::custom(0xC20),
        };
        let micro_blocker = micro_flow.spawn_actor().unwrap();
        let micro_blocker_request = micro_flow
            .acquire(micro_resource)
            .owner(micro_blocker)
            .at(t(0))
            .submit()
            .unwrap();
        micro_flow.step().unwrap().unwrap();
        let micro_blocker_lease = micro_flow.resource(micro_resource).unwrap().allocations[0].lease;
        assert_eq!(
            micro_flow.request(micro_blocker_request).unwrap().state,
            RequestState::Active
        );

        let micro_prepared =
            must_prepare(micro_input.prepare(&micro_flow, &mut micro_adapter, &provider()));
        micro_flow
            .register_work_handlers("bridge.context", WorkHandlers::<u32>::default())
            .unwrap();
        register_transit_context(&mut micro_flow, "bridge.transit", EventKind::custom(0xC20))
            .unwrap();
        let micro_work = must_create(micro_prepared.create(&mut micro_flow));
        let mut micro_bound = must_bind(micro_work.bind(&micro_flow));
        micro_bound.start_transit(&mut micro_flow).unwrap();
        let carrier = micro_bound.carrier.unwrap();
        let start = micro_flow.step().unwrap().unwrap();
        assert_eq!(start.at, t(0));
        assert_eq!(
            micro_bound.observe_transit_dispatch(&micro_flow, &start),
            Ok(TransitObservation::Progress)
        );
        micro_flow.release(micro_blocker_lease, t(5)).unwrap();
        let progress = micro_flow.step().unwrap().unwrap();
        assert_eq!(progress.at, t(1));
        assert_eq!(
            micro_bound.observe_transit_dispatch(&micro_flow, &progress),
            Ok(TransitObservation::Progress)
        );
        let arrival = micro_flow.step().unwrap().unwrap();
        assert_eq!(arrival.at, t(2));
        assert_eq!(
            micro_bound.observe_transit_dispatch(&micro_flow, &arrival),
            Ok(TransitObservation::Arrived)
        );
        let transit = micro_flow
            .work_context::<TransitContext>(carrier)
            .unwrap()
            .progress_at(t(2))
            .unwrap();
        assert_eq!(transit.useful_elapsed, d(2));
        let micro_submitted = micro_bound.finish_transit(&micro_flow).unwrap();
        assert_eq!(micro_submitted.sampled_duration(), service_duration);
        assert_eq!(
            micro_submitted.draw_position(),
            macro_submitted.draw_position()
        );
        assert_eq!(
            micro_submitted.expected_service_key,
            macro_submitted.expected_service_key
        );
        assert_eq!(
            micro_submitted.service_stream.derived_seed(),
            macro_submitted.service_stream.derived_seed()
        );
        let submitted_dispatch = micro_flow.step().unwrap().unwrap();
        assert_eq!(submitted_dispatch.at, t(2));
        let micro_request = micro_flow.request(micro_submitted.request()).unwrap();
        let micro_submitted_at = micro_request.submitted_at;
        let micro_request_state = micro_request.state;
        assert_eq!(micro_submitted_at, t(2));
        assert_eq!(micro_request_state, RequestState::Queued);
        micro_flow.step().unwrap().unwrap();
        let micro_allocation = micro_flow
            .resource(micro_resource)
            .unwrap()
            .allocations
            .into_iter()
            .find(|allocation| allocation.request == micro_submitted.request())
            .unwrap();
        assert_eq!(micro_allocation.granted_at, t(5));
        let micro_completion = micro_allocation.completion_at.unwrap();
        assert_eq!(micro_flow.step().unwrap().unwrap().at, micro_completion);
        assert_eq!(
            micro_flow
                .work_progress(micro_submitted.work())
                .unwrap()
                .state,
            WorkState::Completed
        );

        let macro_queue = macro_allocation
            .granted_at
            .duration_since(macro_submitted_at)
            .unwrap();
        let micro_queue = micro_allocation
            .granted_at
            .duration_since(micro_submitted_at)
            .unwrap();
        let macro_service = macro_completion
            .duration_since(macro_allocation.granted_at)
            .unwrap();
        let micro_service = micro_completion
            .duration_since(micro_allocation.granted_at)
            .unwrap();
        assert_eq!(macro_queue, d(5));
        assert_eq!(micro_queue, d(3));
        assert_eq!(macro_service, service_duration);
        assert_eq!(micro_service, service_duration);
        assert_eq!(
            macro_completion,
            t(5).checked_add(service_duration).unwrap()
        );
        assert_eq!(micro_completion, macro_completion);
        assert_eq!(
            macro_completion.duration_since(t(0)).unwrap(),
            macro_queue.checked_add(macro_service).unwrap()
        );
        assert_eq!(
            micro_completion.duration_since(t(0)).unwrap(),
            transit
                .useful_elapsed
                .checked_add(micro_queue)
                .unwrap()
                .checked_add(micro_service)
                .unwrap()
        );
    }

    struct UrgentPreemptionOutcome {
        completion: SimTime,
        service_duration: SimDuration,
        service_key: crate::seed_map::CalibrationStreamKey,
        service_seed: u64,
        service_draw_position: u64,
        decision: FidelityDecision,
        transit_elapsed: SimDuration,
        has_transit_carrier: bool,
        terminal_progress: kairo_ecs_des::WorkProgress,
        terminal_request_state: RequestState,
    }

    fn recorded_step(
        flow: &mut FlowRuntime,
        records: &mut Vec<kairo_ecs_des::LifecycleRecord>,
    ) -> FlowDispatch {
        let dispatch = flow.step().unwrap().expect("scheduled Flow dispatch");
        records.extend(dispatch.records.iter().cloned());
        dispatch
    }

    fn run_urgent_preemption_case(
        mode: FidelityMode,
        preemption_strategy: PreemptionStrategy,
        transit_intent: TransitIntent,
    ) -> UrgentPreemptionOutcome {
        let t = SimTime::from_ticks;
        let d = SimDuration::from_ticks;
        let (mut work_input, mut flow, mut adapter, resource) =
            input(mode, transit_intent, SeedPurpose::Service, false);
        // Q4 interruption is enabled only for this fixture; production bridge
        // defaults remain non-preemptible.
        work_input.acquire.preemptible = Some(preemption_strategy);
        if mode == FidelityMode::Micro
            && matches!(&work_input.transit, TransitRequest::Route { .. })
        {
            let carrier_actor = match &work_input.transit {
                TransitRequest::Route { carrier_actor, .. } => *carrier_actor,
                TransitRequest::Zero => unreachable!(),
            };
            let nodes = (1..=3).map(NodeId::new).collect::<Vec<_>>();
            let walk = MovementModeId::new("walk").unwrap();
            let edges = (0..2)
                .map(|index| TransitEdge {
                    id: EdgeId::new(index + 1),
                    from: nodes[index as usize],
                    to: nodes[index as usize + 1],
                    length_mm: 1,
                    allowed_modes: vec![walk.clone()],
                })
                .collect();
            work_input.transit = TransitRequest::Route {
                graph: Arc::new(TransitGraphV1::new(1, nodes.clone(), edges).unwrap()),
                origin: nodes[0],
                destination: *nodes.last().unwrap(),
                profile: MovementProfile::new("walk", 1).unwrap(),
                ticks_per_second: 1,
                carrier_actor,
                carrier_registration: "bridge.transit".to_owned(),
                kind: EventKind::custom(0xC20),
            };
        }

        let blocker = flow.spawn_actor().unwrap();
        let blocker_request = flow
            .acquire(resource)
            .owner(blocker)
            .at(t(0))
            .submit()
            .unwrap();
        let mut records = Vec::new();
        let blocker_dispatch = recorded_step(&mut flow, &mut records);
        assert_eq!(blocker_dispatch.at, t(0));
        assert_eq!(
            flow.request(blocker_request).unwrap().state,
            RequestState::Active
        );
        let blocker_lease = flow.resource(resource).unwrap().allocations[0].lease;

        flow.register_work_handlers("bridge.context", WorkHandlers::<u32>::default())
            .unwrap();
        let prepared = must_prepare(work_input.prepare(&flow, &mut adapter, &provider()));
        assert_eq!(prepared.decision().mode, mode);
        let service_duration = prepared.sampled_duration();
        assert_eq!(service_duration, d(20));
        assert_eq!(prepared.draw_bounds(), (0, 1));
        let work = must_create(prepared.create(&mut flow));
        let work_id = work.work();
        let mut bound = must_bind(work.bind(&flow));
        let has_transit_carrier =
            mode == FidelityMode::Micro && matches!(&bound.transit, TransitRequest::Route { .. });
        let (transit_elapsed, submitted) = if mode == FidelityMode::Micro && has_transit_carrier {
            register_transit_context(&mut flow, "bridge.transit", EventKind::custom(0xC20))
                .unwrap();
            bound.start_transit(&mut flow).unwrap();
            let start = recorded_step(&mut flow, &mut records);
            assert_eq!(start.at, t(0));
            let start_observation = bound.observe_transit_dispatch(&flow, &start);
            assert_eq!(start_observation, Ok(TransitObservation::Progress));
            let progress = recorded_step(&mut flow, &mut records);
            assert_eq!(progress.at, t(1));
            let progress_observation = bound.observe_transit_dispatch(&flow, &progress);
            assert_eq!(progress_observation, Ok(TransitObservation::Progress));
            let arrival = recorded_step(&mut flow, &mut records);
            assert_eq!(arrival.at, t(2));
            let arrival_observation = bound.observe_transit_dispatch(&flow, &arrival);
            assert_eq!(arrival_observation, Ok(TransitObservation::Arrived));
            assert_eq!(
                [start_observation, progress_observation, arrival_observation],
                [
                    Ok(TransitObservation::Progress),
                    Ok(TransitObservation::Progress),
                    Ok(TransitObservation::Arrived),
                ]
            );
            let elapsed = flow
                .work_context::<TransitContext>(bound.carrier.unwrap())
                .unwrap()
                .progress_at(t(2))
                .unwrap()
                .useful_elapsed;
            assert_eq!(elapsed, d(2));
            let submitted = bound.finish_transit(&flow).unwrap();
            assert_eq!(submitted.work(), work_id);
            (elapsed, submitted)
        } else {
            (SimDuration::ZERO, must_submit(bound.submit(&mut flow)))
        };
        assert_eq!(submitted.work(), work_id);
        assert_eq!(submitted.sampled_duration(), d(20));
        assert_eq!(submitted.service_draw_position(), 1);
        let request_id = submitted.request();
        assert_eq!(flow.request(request_id).unwrap().work, Some(work_id));
        assert_eq!(
            flow.request(request_id).unwrap().preemptible,
            Some(preemption_strategy)
        );
        flow.release(blocker_lease, t(5)).unwrap();

        // Drain only until the original timed service starts at tick 5.
        while flow.request(request_id).unwrap().state != RequestState::Active {
            recorded_step(&mut flow, &mut records);
        }
        assert_eq!(flow.now(), t(5));
        assert_eq!(
            flow.work_progress(work_id).unwrap().useful_elapsed,
            SimDuration::ZERO
        );

        let urgent_owner = flow.spawn_actor().unwrap();
        let urgent_work = flow
            .create_work(urgent_owner, d(2), "bridge.context", 7u32)
            .unwrap();
        let urgent_request = flow
            .acquire(resource)
            .owner(urgent_owner)
            .timed_work(urgent_work)
            .priority(2)
            .can_preempt(true)
            .at(t(8))
            .submit()
            .unwrap();

        let at_eight = recorded_step(&mut flow, &mut records);
        assert_eq!(at_eight.at, t(8));
        assert_eq!(
            flow.request(request_id).unwrap().state,
            RequestState::Suspended
        );
        assert_eq!(
            flow.request(urgent_request).unwrap().state,
            RequestState::Active
        );
        let suspended = flow.work_progress(work_id).unwrap();
        assert_eq!(suspended.state, WorkState::Suspended);
        match preemption_strategy {
            PreemptionStrategy::Suspend => {
                assert_eq!(suspended.useful_elapsed, d(3));
                assert_eq!(suspended.remaining, d(17));
            }
            PreemptionStrategy::Restart => {
                assert_eq!(suspended.attempt_revision, 1);
                assert_eq!(suspended.useful_elapsed, SimDuration::ZERO);
                assert_eq!(suspended.remaining, d(20));
            }
            PreemptionStrategy::Abort => unreachable!("fixture does not exercise Abort"),
        }
        let preempted = at_eight
            .records
            .iter()
            .find(|row| {
                row.request == request_id && row.transition == LifecycleTransition::Preempted
            })
            .unwrap();
        assert_eq!(
            preempted.snapshot.progress.as_ref().unwrap().useful_elapsed,
            d(3)
        );
        assert_eq!(
            preempted.snapshot.progress.as_ref().unwrap().remaining,
            d(17)
        );
        let paused_resource = flow.resource(resource).unwrap();
        assert_eq!(
            paused_resource.total,
            paused_resource.available + paused_resource.active.len() as u32
        );
        assert_eq!(paused_resource.active.len(), 1);
        assert_eq!(paused_resource.queued, vec![request_id]);

        let at_ten = recorded_step(&mut flow, &mut records);
        assert_eq!(at_ten.at, t(10));
        assert_eq!(
            flow.request(urgent_request).unwrap().state,
            RequestState::Completed
        );
        assert_eq!(
            flow.request(request_id).unwrap().state,
            RequestState::Active
        );
        let resumed = flow.work_progress(work_id).unwrap();
        assert_eq!(resumed.state, WorkState::Active);
        match preemption_strategy {
            PreemptionStrategy::Suspend => {
                assert_eq!(resumed.useful_elapsed, d(3));
                assert_eq!(resumed.remaining, d(17));
                let resumed_record = at_ten
                    .records
                    .iter()
                    .find(|row| {
                        row.request == request_id && row.transition == LifecycleTransition::Resumed
                    })
                    .unwrap();
                assert_eq!(
                    resumed_record.snapshot.progress.as_ref().unwrap().remaining,
                    d(17)
                );
            }
            PreemptionStrategy::Restart => {
                assert_eq!(resumed.attempt_revision, 1);
                assert_eq!(resumed.useful_elapsed, SimDuration::ZERO);
                assert_eq!(resumed.remaining, d(20));
                let restarted_record = at_ten
                    .records
                    .iter()
                    .find(|row| {
                        row.request == request_id
                            && row.transition == LifecycleTransition::Restarted
                    })
                    .unwrap();
                assert_eq!(
                    restarted_record
                        .snapshot
                        .progress
                        .as_ref()
                        .unwrap()
                        .remaining,
                    d(20)
                );
            }
            PreemptionStrategy::Abort => unreachable!("fixture does not exercise Abort"),
        }

        let mut completion = None;
        while flow.request(request_id).unwrap().state != RequestState::Completed {
            let dispatch = recorded_step(&mut flow, &mut records);
            if dispatch.records.iter().any(|row| {
                row.request == request_id && row.transition == LifecycleTransition::Completed
            }) {
                completion = Some(dispatch.at);
            }
        }
        let completion = completion.unwrap();
        let expected_completion = if preemption_strategy == PreemptionStrategy::Restart {
            t(30)
        } else {
            t(27)
        };
        assert_eq!(completion, expected_completion);
        let completed = flow.work_progress(work_id).unwrap();
        assert_eq!(completed.state, WorkState::Completed);
        assert_eq!(completed.useful_elapsed, d(20));
        assert_eq!(completed.remaining, SimDuration::ZERO);

        let resource_end = flow.resource(resource).unwrap();
        assert_eq!(
            resource_end.total,
            resource_end.available + resource_end.active.len() as u32
        );
        assert_eq!(resource_end.available, 1);
        assert!(resource_end.active.is_empty());
        assert!(resource_end.queued.is_empty());
        assert_eq!(
            flow.budget_snapshot().scheduler.pending_events,
            0,
            "the isolated urgent-interruption fixture must leave no future Flow events"
        );
        let original_rows: Vec<_> = records
            .iter()
            .filter(|row| row.request == request_id)
            .map(|row| (row.at, row.transition))
            .collect();
        let submitted_at = flow.request(request_id).unwrap().submitted_at;
        let granted_at = original_rows
            .iter()
            .find(|(_, transition)| *transition == LifecycleTransition::Granted)
            .unwrap()
            .0;
        let return_transition = if preemption_strategy == PreemptionStrategy::Restart {
            LifecycleTransition::Restarted
        } else {
            LifecycleTransition::Resumed
        };
        assert_eq!(
            original_rows,
            vec![
                (submitted_at, LifecycleTransition::Queued),
                (t(5), LifecycleTransition::Granted),
                (t(8), LifecycleTransition::Preempted),
                (t(10), return_transition),
                (expected_completion, LifecycleTransition::Completed),
            ]
        );
        let bridge_lineage_rows: Vec<_> = records
            .iter()
            .filter(|row| row.snapshot.work == Some(work_id))
            .collect();
        assert_eq!(bridge_lineage_rows.len(), original_rows.len());
        assert!(bridge_lineage_rows
            .iter()
            .all(|row| row.request == request_id));
        let urgent_rows: Vec<_> = records
            .iter()
            .filter(|row| row.request == urgent_request)
            .map(|row| (row.at, row.transition))
            .collect();
        assert_eq!(
            urgent_rows,
            vec![
                (t(8), LifecycleTransition::Queued),
                (t(8), LifecycleTransition::Granted),
                (t(10), LifecycleTransition::Completed),
            ]
        );
        let queue_elapsed = granted_at.duration_since(submitted_at).unwrap();
        assert_eq!(
            queue_elapsed,
            if transit_elapsed == SimDuration::ZERO {
                d(5)
            } else {
                d(3)
            }
        );
        let useful_service = completed.useful_elapsed;
        let interruption = t(10).duration_since(t(8)).unwrap();
        assert_eq!(useful_service, d(20));
        assert_eq!(interruption, d(2));
        let discarded_attempt = if preemption_strategy == PreemptionStrategy::Restart {
            d(3)
        } else {
            SimDuration::ZERO
        };
        assert_eq!(
            transit_elapsed
                .checked_add(queue_elapsed)
                .unwrap()
                .checked_add(discarded_attempt)
                .unwrap()
                .checked_add(useful_service)
                .unwrap()
                .checked_add(interruption)
                .unwrap(),
            completion.duration_since(t(0)).unwrap()
        );

        UrgentPreemptionOutcome {
            completion,
            service_duration: submitted.sampled_duration(),
            service_key: submitted.expected_service_key.clone(),
            service_seed: submitted.service_stream.derived_seed(),
            service_draw_position: submitted.service_draw_position(),
            decision: submitted.decision(),
            transit_elapsed,
            has_transit_carrier,
            terminal_progress: completed,
            terminal_request_state: flow.request(request_id).unwrap().state,
        }
    }

    #[test]
    fn actual_flow_urgent_suspend_resumes_same_macro_and_routed_micro_service() {
        let macro_run = run_urgent_preemption_case(
            FidelityMode::Macro,
            PreemptionStrategy::Suspend,
            TransitIntent::Route,
        );
        let micro_run = run_urgent_preemption_case(
            FidelityMode::Micro,
            PreemptionStrategy::Suspend,
            TransitIntent::Route,
        );

        assert_eq!(macro_run.completion, SimTime::from_ticks(27));
        assert_eq!(micro_run.completion, macro_run.completion);
        assert_eq!(macro_run.service_duration, SimDuration::from_ticks(20));
        assert_eq!(micro_run.service_duration, macro_run.service_duration);
        assert_eq!(macro_run.service_key, micro_run.service_key);
        assert_eq!(macro_run.service_seed, micro_run.service_seed);
        assert_eq!(macro_run.service_draw_position, 1);
        assert_eq!(
            micro_run.service_draw_position,
            macro_run.service_draw_position
        );
        assert_eq!(macro_run.decision.mode, FidelityMode::Macro);
        assert_eq!(micro_run.decision.mode, FidelityMode::Micro);
        assert_eq!(macro_run.transit_elapsed, SimDuration::ZERO);
        assert_eq!(micro_run.transit_elapsed, SimDuration::from_ticks(2));
        let t = SimTime::from_ticks;
        let d = SimDuration::from_ticks;
        assert_eq!(
            macro_run.completion.duration_since(t(0)).unwrap(),
            d(5).checked_add(d(20)).unwrap().checked_add(d(2)).unwrap()
        );
        assert_eq!(
            micro_run.completion.duration_since(t(0)).unwrap(),
            d(2).checked_add(d(3))
                .unwrap()
                .checked_add(d(20))
                .unwrap()
                .checked_add(d(2))
                .unwrap()
        );
    }

    #[test]
    fn actual_flow_urgent_restart_reuses_sampled_service_for_macro_and_zero_transit_micro() {
        let macro_run = run_urgent_preemption_case(
            FidelityMode::Macro,
            PreemptionStrategy::Restart,
            TransitIntent::Zero,
        );
        let micro_run = run_urgent_preemption_case(
            FidelityMode::Micro,
            PreemptionStrategy::Restart,
            TransitIntent::Zero,
        );

        assert_eq!(macro_run.completion, SimTime::from_ticks(30));
        assert_eq!(micro_run.completion, macro_run.completion);
        assert_eq!(macro_run.service_duration, SimDuration::from_ticks(20));
        assert_eq!(micro_run.service_duration, macro_run.service_duration);
        assert_eq!(macro_run.service_key, micro_run.service_key);
        assert_eq!(macro_run.service_seed, micro_run.service_seed);
        assert_eq!(macro_run.service_draw_position, 1);
        assert_eq!(
            micro_run.service_draw_position,
            macro_run.service_draw_position
        );
        assert_eq!(macro_run.decision.mode, FidelityMode::Macro);
        assert_eq!(micro_run.decision.mode, FidelityMode::Micro);
        assert_eq!(macro_run.transit_elapsed, SimDuration::ZERO);
        assert_eq!(micro_run.transit_elapsed, SimDuration::ZERO);
        assert!(!macro_run.has_transit_carrier);
        assert!(!micro_run.has_transit_carrier);
        assert_eq!(macro_run.terminal_progress, micro_run.terminal_progress);
        assert_eq!(macro_run.terminal_request_state, RequestState::Completed);
        assert_eq!(
            micro_run.terminal_request_state,
            macro_run.terminal_request_state
        );

        let t = SimTime::from_ticks;
        let d = SimDuration::from_ticks;
        assert_eq!(
            macro_run.completion.duration_since(t(0)).unwrap(),
            d(5).checked_add(d(3))
                .unwrap()
                .checked_add(d(2))
                .unwrap()
                .checked_add(d(20))
                .unwrap()
        );
        assert_eq!(
            micro_run.completion.duration_since(t(0)).unwrap(),
            d(5).checked_add(d(3))
                .unwrap()
                .checked_add(d(2))
                .unwrap()
                .checked_add(d(20))
                .unwrap()
        );
    }

    #[test]
    fn micro_route_is_prepared_and_samples_service_once() {
        let (input, flow, mut adapter, _) = input(
            FidelityMode::Micro,
            TransitIntent::Route,
            SeedPurpose::Service,
            false,
        );
        let prepared = must_prepare(input.prepare(&flow, &mut adapter, &provider()));
        assert_eq!(prepared.draw_bounds(), (0, 1));
        assert_eq!(prepared.service_draw_position(), 1);
    }

    #[test]
    fn matching_rejected_transit_event_retries_once_and_replay_is_atomic() {
        let (input, mut flow, mut adapter, _) = input(
            FidelityMode::Micro,
            TransitIntent::Route,
            SeedPurpose::Service,
            false,
        );
        register_transit_context(&mut flow, "bridge.transit", EventKind::custom(0xC20)).unwrap();
        flow.register_work_handlers("bridge.context", WorkHandlers::<u32>::default())
            .unwrap();
        let prepared = must_prepare(input.prepare(&flow, &mut adapter, &provider()));
        let created = must_create(prepared.create(&mut flow));
        let mut bound = must_bind(created.bind(&flow));
        let original = bound.start_transit(&mut flow).unwrap();
        let rejected = FlowDispatch {
            event: original,
            at: flow.now(),
            records: Vec::new(),
            error: None,
            callback_batches: vec![FlowBatchReceipt::Rejected(FlowBatchRejection {
                failed_ticket: None,
                error: FlowError::InvalidState,
            })],
        };
        assert_eq!(
            bound.observe_transit_dispatch(&flow, &rejected),
            Ok(TransitObservation::Rejected)
        );
        let retry_image = BoundIntrinsicWorkCheckpointV1::capture(
            &bound,
            &flow,
            &adapter,
            bridge_checkpoint_limits(),
        )
        .unwrap();
        assert_eq!(retry_image.retryable, Some(rejected.clone()));
        assert_eq!(retry_image.retryable_dispatch(), Some(&rejected));
        let baseline = flow.budget_snapshot().scheduler;
        let mut wrong_event = rejected.clone();
        wrong_event.event = EventId::new(u64::MAX, u32::MAX);
        assert_eq!(
            bound.observe_transit_dispatch(&flow, &wrong_event),
            Err(BridgeError::InvalidDispatch)
        );
        let mut forged_accepted = rejected.clone();
        forged_accepted.callback_batches = vec![FlowBatchReceipt::Accepted(Vec::new())];
        assert_eq!(
            bound.observe_transit_dispatch(&flow, &forged_accepted),
            Err(BridgeError::InvalidDispatch)
        );
        let mut malformed = rejected.clone();
        malformed
            .callback_batches
            .push(FlowBatchReceipt::Accepted(Vec::new()));
        assert_eq!(
            bound.observe_transit_dispatch(&flow, &malformed),
            Err(BridgeError::InvalidDispatch)
        );
        let mut foreign = FlowRuntime::new();
        assert_eq!(
            bound.retry_transit(&mut foreign, &rejected),
            Err(BridgeError::Fidelity(FidelityError::InvalidWork))
        );
        assert_eq!(flow.budget_snapshot().scheduler, baseline);
        let carrier = bound.carrier;
        let kind = bound.kind;
        let priority = bound.pending_priority;
        bound.carrier = Some(bound.work);
        assert!(bound.retry_transit(&mut flow, &rejected).is_err());
        bound.carrier = carrier;
        bound.kind = Some(EventKind::custom(0xC21));
        assert_eq!(
            bound.retry_transit(&mut flow, &rejected),
            Err(BridgeError::InvalidDispatch)
        );
        bound.kind = kind;
        bound.pending_priority = priority.map(|value| value + 1);
        assert_eq!(
            bound.retry_transit(&mut flow, &rejected),
            Err(BridgeError::InvalidDispatch)
        );
        bound.pending_priority = priority;
        assert_eq!(flow.budget_snapshot().scheduler, baseline);
        let carrier = bound.carrier.unwrap();
        let failed_schedule = TransitContext::schedule_transit_retry(
            &mut flow,
            carrier,
            EventKind::custom(0xC21),
            &rejected,
            7,
        );
        assert_eq!(failed_schedule, Err(FlowError::InvalidWork));
        assert_eq!(flow.budget_snapshot().scheduler, baseline);
        let before_retry = flow.budget_snapshot().scheduler;
        let replacement = bound.retry_transit(&mut flow, &rejected).unwrap();
        assert_ne!(replacement, original);
        let after_retry = flow.budget_snapshot().scheduler;
        assert_eq!(
            after_retry.scheduled_events,
            before_retry.scheduled_events + 1
        );
        assert_eq!(
            bound.retry_transit(&mut flow, &rejected),
            Err(BridgeError::InvalidDispatch)
        );
        assert_eq!(flow.budget_snapshot().scheduler, after_retry);
    }

    #[test]
    fn actual_transit_rejection_retries_to_one_accepted_arrival_and_request() {
        let (mut bound, mut flow) = bound_route_at_with_registration(
            SimTime::from_ticks(0),
            1,
            register_transit_context_reject_first_for_test,
        );
        let original = bound.start_transit(&mut flow).unwrap();
        let work_id = bound.work();
        let carrier = bound.carrier.unwrap();

        let rejected = flow
            .step()
            .unwrap()
            .expect("actual scheduled start callback");
        assert_eq!(rejected.event, original);
        assert_eq!(rejected.at, SimTime::from_ticks(0));
        assert_eq!(rejected.error, Some(FlowError::InvalidState));
        assert!(matches!(
            rejected.callback_batches.as_slice(),
            [FlowBatchReceipt::Rejected(rejection)]
                if rejection.error == FlowError::InvalidState
                    && rejection.failed_ticket.is_none()
        ));
        assert_eq!(
            bound.observe_transit_dispatch(&flow, &rejected),
            Ok(TransitObservation::Rejected)
        );
        let ready = flow.work_context::<TransitContext>(carrier).unwrap();
        assert_eq!(ready.phase(), TransitPhase::Ready);
        assert!(ready.expects_event(original, SimTime::from_ticks(0)));
        assert_eq!(flow.work(bound.work()).unwrap().request, None);

        let replacement = bound.retry_transit(&mut flow, &rejected).unwrap();
        assert_ne!(replacement, original);
        assert!(flow
            .work_context::<TransitContext>(carrier)
            .unwrap()
            .expects_event(replacement, SimTime::from_ticks(0)));
        let after_retry = flow.budget_snapshot().scheduler;
        assert_eq!(
            bound.retry_transit(&mut flow, &rejected),
            Err(BridgeError::InvalidDispatch)
        );
        assert_eq!(flow.budget_snapshot().scheduler, after_retry);
        assert_eq!(bound.owned_events, vec![original, replacement]);
        assert_eq!(bound.consumed_events, vec![original]);

        let retried_start = flow.step().unwrap().expect("actual retried start callback");
        assert_eq!(retried_start.event, replacement);
        assert!(matches!(
            retried_start.callback_batches.as_slice(),
            [FlowBatchReceipt::Accepted(admissions)] if admissions.len() == 1
        ));
        assert_eq!(
            bound.observe_transit_dispatch(&flow, &retried_start),
            Ok(TransitObservation::Progress)
        );
        assert_eq!(bound.consumed_events, vec![original, replacement]);
        assert_eq!(flow.work(bound.work()).unwrap().request, None);
        assert_eq!(
            flow.work_context::<TransitContext>(carrier)
                .unwrap()
                .phase(),
            TransitPhase::Moving
        );

        let arrival = flow.step().unwrap().expect("actual route arrival callback");
        let arrival_event = bound.pending_event.unwrap();
        assert_ne!(arrival_event, original);
        assert_ne!(arrival_event, replacement);
        assert_eq!(arrival.event, arrival_event);
        assert!(matches!(
            arrival.callback_batches.as_slice(),
            [FlowBatchReceipt::Accepted(admissions)]
                if admissions.len() == 1 && admissions[0].request.is_some()
        ));
        assert_eq!(
            bound.observe_transit_dispatch(&flow, &arrival),
            Ok(TransitObservation::Arrived)
        );
        let [FlowBatchReceipt::Accepted(admissions)] = arrival.callback_batches.as_slice() else {
            unreachable!("arrival was asserted accepted above")
        };
        let request_event = admissions[0].event;
        assert_eq!(bound.pending_event, Some(request_event));
        assert_ne!(request_event, arrival_event);
        assert_eq!(
            bound.owned_events,
            vec![original, replacement, arrival_event, request_event]
        );
        assert_eq!(
            bound.consumed_events,
            vec![original, replacement, arrival_event]
        );
        let request = admissions[0].request.unwrap();
        let saved_work = flow.work(work_id).unwrap();
        let saved_request = flow.request(request).unwrap();
        assert_eq!(saved_work.request, Some(request));
        assert_eq!(saved_work.owner, bound.acquire.owner);
        assert_eq!(saved_request.work, Some(work_id));
        assert_eq!(saved_request.owner, bound.acquire.owner);
        assert_eq!(saved_request.resource, bound.acquire.resource);
        assert!(saved_request.timed);
        assert_eq!(saved_request.submitted_at, arrival.at);

        let submitted = bound.finish_transit(&flow).unwrap();
        assert_eq!(submitted.work(), work_id);
        assert_eq!(submitted.request(), request);
    }

    #[test]
    fn runtime_rejected_start_at_u128_max_retries_once_at_same_due_and_priority() {
        let max = SimTime::from_ticks(u128::MAX);
        let (mut bound, mut flow) = bound_route_at(max, 1);
        let original = bound.start_transit(&mut flow).unwrap();
        let carrier = bound.carrier.unwrap();
        let rejected_start = flow.step().unwrap().expect("actual scheduled start event");
        assert_eq!(rejected_start.event, original);
        assert_eq!(rejected_start.at, max);
        assert_overflow_rejection(&rejected_start);
        assert_eq!(
            bound.observe_transit_dispatch(&flow, &rejected_start),
            Ok(TransitObservation::Rejected)
        );
        let context = flow.work_context::<TransitContext>(carrier).unwrap();
        assert_eq!(context.phase(), TransitPhase::Ready);
        assert!(context.expects_event(original, max));

        let before_retry = flow.budget_snapshot().scheduler;
        let replacement = bound.retry_transit(&mut flow, &rejected_start).unwrap();
        assert_ne!(replacement, original);
        assert_eq!(
            flow.budget_snapshot().scheduler.scheduled_events,
            before_retry.scheduled_events + 1
        );
        assert!(flow
            .work_context::<TransitContext>(carrier)
            .unwrap()
            .expects_event(replacement, max));
        let after_retry = flow.budget_snapshot().scheduler;
        assert_eq!(
            bound.retry_transit(&mut flow, &rejected_start),
            Err(BridgeError::InvalidDispatch)
        );
        assert_eq!(flow.budget_snapshot().scheduler, after_retry);
        assert_eq!(bound.pending_priority, Some(7));

        // Bracket the retry with same-time events: the replacement sorts after
        // priority 6 and before priority 8, proving the retained priority is 7.
        let before_priority = flow
            .schedule_domain(carrier, EventKind::custom(0xC20), max, 6)
            .unwrap();
        let after_priority = flow
            .schedule_domain(carrier, EventKind::custom(0xC20), max, 8)
            .unwrap();
        assert_eq!(flow.step().unwrap().unwrap().event, before_priority);
        let retried = flow.step().unwrap().expect("replacement start retry");
        assert_eq!(retried.event, replacement);
        assert_eq!(retried.at, max);
        assert_overflow_rejection(&retried);
        assert_eq!(flow.step().unwrap().unwrap().event, after_priority);
    }

    #[test]
    fn runtime_rejected_progress_at_u128_max_retries_once_and_preserves_progress() {
        let max = SimTime::from_ticks(u128::MAX);
        let start = SimTime::from_ticks(u128::MAX - 1);
        let (mut bound, mut flow) = bound_route_at(start, 2);
        let start_event = bound.start_transit(&mut flow).unwrap();
        let carrier = bound.carrier.unwrap();
        let accepted_start = flow.step().unwrap().expect("actual scheduled start event");
        assert_eq!(accepted_start.event, start_event);
        assert_eq!(accepted_start.at, start);
        assert!(matches!(
            accepted_start.callback_batches.as_slice(),
            [FlowBatchReceipt::Accepted(_)]
        ));
        assert_eq!(
            bound.observe_transit_dispatch(&flow, &accepted_start),
            Ok(TransitObservation::Progress)
        );
        let progress_event = bound.pending_event.unwrap();

        let rejected_progress = flow
            .step()
            .unwrap()
            .expect("actual scheduled progress event");
        assert_eq!(rejected_progress.event, progress_event);
        assert_eq!(rejected_progress.at, max);
        assert_overflow_rejection(&rejected_progress);
        assert_eq!(
            bound.observe_transit_dispatch(&flow, &rejected_progress),
            Ok(TransitObservation::Rejected)
        );
        let context = flow.work_context::<TransitContext>(carrier).unwrap();
        assert_eq!(context.phase(), TransitPhase::Moving);
        let retained = context.progress_at(start).unwrap();
        assert_eq!(retained.useful_elapsed, SimDuration::ZERO);
        assert_eq!(retained.remaining, SimDuration::from_ticks(2));
        assert!(context.expects_event(progress_event, max));

        let before_retry = flow.budget_snapshot().scheduler;
        let replacement = bound.retry_transit(&mut flow, &rejected_progress).unwrap();
        assert_ne!(replacement, progress_event);
        assert_eq!(
            flow.budget_snapshot().scheduler.scheduled_events,
            before_retry.scheduled_events + 1
        );
        assert!(flow
            .work_context::<TransitContext>(carrier)
            .unwrap()
            .expects_event(replacement, max));
        let after_retry = flow.budget_snapshot().scheduler;
        assert_eq!(
            bound.retry_transit(&mut flow, &rejected_progress),
            Err(BridgeError::InvalidDispatch)
        );
        assert_eq!(flow.budget_snapshot().scheduler, after_retry);
        assert_eq!(bound.pending_priority, Some(7));

        let before_priority = flow
            .schedule_domain(carrier, EventKind::custom(0xC20), max, 6)
            .unwrap();
        let after_priority = flow
            .schedule_domain(carrier, EventKind::custom(0xC20), max, 8)
            .unwrap();
        assert_eq!(flow.step().unwrap().unwrap().event, before_priority);
        let retried = flow.step().unwrap().expect("replacement progress retry");
        assert_eq!(retried.event, replacement);
        assert_eq!(retried.at, max);
        assert_overflow_rejection(&retried);
        assert_eq!(flow.step().unwrap().unwrap().event, after_priority);
    }

    #[test]
    fn wrong_service_purpose_and_identity_reject_without_advancing() {
        for (purpose, mismatch) in [(SeedPurpose::Transit, false), (SeedPurpose::Service, true)] {
            let (input, flow, mut adapter, _) =
                input(FidelityMode::Macro, TransitIntent::Zero, purpose, mismatch);
            let failure = match input.prepare(&flow, &mut adapter, &provider()) {
                Ok(_) => panic!("invalid Service stream should be rejected"),
                Err(failure) => failure,
            };
            assert_eq!(failure.input.service_stream.draw_position(), 0);
            assert!(matches!(failure.error, BridgeError::Duration(_)));
        }
    }

    #[test]
    fn create_failure_retains_template_sample_and_stream_for_retry() {
        let (input, mut flow, mut adapter, _) = input(
            FidelityMode::Macro,
            TransitIntent::Zero,
            SeedPurpose::Service,
            false,
        );
        let mut prepared = must_prepare(input.prepare(&flow, &mut adapter, &provider()));
        let duration = prepared.sampled_duration();
        let draws = prepared.draw_bounds();
        let before = flow.budget_snapshot().scheduler;
        prepared.registration.clear();
        let failure = match prepared.create(&mut flow) {
            Ok(_) => panic!("invalid registration should fail creation"),
            Err(failure) => failure,
        };
        assert!(matches!(failure.error, BridgeError::Flow(_)));
        assert_eq!(failure.prepared.sampled_duration(), duration);
        assert_eq!(failure.prepared.draw_bounds(), draws);
        assert_eq!(failure.prepared.service_stream.draw_position(), draws.1);
        assert_eq!(flow.budget_snapshot().scheduler, before);
        let mut prepared = failure.prepared;
        prepared.registration = "bridge.context".to_owned();
        let created = must_create(prepared.create(&mut flow));
        assert_eq!(*flow.work_context::<u32>(created.work()).unwrap(), 42);
        assert_eq!(
            flow.work(created.work()).unwrap().original_duration,
            duration
        );
    }

    #[test]
    fn bind_failure_keeps_same_created_work_for_retry() {
        let (input, mut flow, mut adapter, _) = input(
            FidelityMode::Micro,
            TransitIntent::Zero,
            SeedPurpose::Service,
            false,
        );
        flow.register_work_handlers("bridge.context", WorkHandlers::<u32>::default())
            .unwrap();
        let prepared = must_prepare(input.prepare(&flow, &mut adapter, &provider()));
        let created = must_create(prepared.create(&mut flow));
        let work = created.work();
        let foreign = FlowRuntime::new();
        let failure = match created.bind(&foreign) {
            Ok(_) => panic!("foreign Flow runtime must not bind work"),
            Err(failure) => failure,
        };
        assert_eq!(
            failure.error,
            BridgeError::Fidelity(FidelityError::InvalidWork)
        );
        assert_eq!(failure.created.work(), work);
        assert_eq!(flow.work(work).unwrap().request, None);
        let bound = must_bind(failure.created.bind(&flow));
        assert_eq!(bound.work(), work);
    }

    #[test]
    fn foreign_runtime_submit_failure_retains_bound_work_for_retry() {
        let (input, mut flow, mut adapter, _) = input(
            FidelityMode::Macro,
            TransitIntent::Zero,
            SeedPurpose::Service,
            false,
        );
        flow.register_work_handlers("bridge.context", WorkHandlers::<u32>::default())
            .unwrap();
        let prepared = must_prepare(input.prepare(&flow, &mut adapter, &provider()));
        let created = must_create(prepared.create(&mut flow));
        let bound = must_bind(created.bind(&flow));
        let work = bound.work();
        let draw_position = bound.draw_position();
        let mut foreign = FlowRuntime::new();
        let foreign_events = foreign.budget_snapshot().scheduler.scheduled_events;
        let failure = match bound.submit(&mut foreign) {
            Ok(_) => panic!("foreign Flow runtime must not submit bound work"),
            Err(failure) => failure,
        };
        assert_eq!(failure.error, BridgeError::InvalidDispatch);
        assert_eq!(failure.bound.work(), work);
        assert_eq!(failure.bound.draw_position(), draw_position);
        assert_eq!(
            foreign.budget_snapshot().scheduler.scheduled_events,
            foreign_events
        );
        let submitted = must_submit(failure.bound.submit(&mut flow));
        assert_eq!(submitted.work(), work);
        assert_eq!(flow.work(work).unwrap().request, Some(submitted.request()));
    }

    #[test]
    fn conflicting_external_request_never_becomes_a_second_bridge_receipt() {
        let (input, mut flow, mut adapter, resource) = input(
            FidelityMode::Micro,
            TransitIntent::Zero,
            SeedPurpose::Service,
            false,
        );
        flow.register_work_handlers("bridge.context", WorkHandlers::<u32>::default())
            .unwrap();
        let prepared = must_prepare(input.prepare(&flow, &mut adapter, &provider()));
        let created = must_create(prepared.create(&mut flow));
        let bound = must_bind(created.bind(&flow));
        let work = bound.work();
        let draw_position = bound.draw_position();
        let owner = flow.work(work).unwrap().owner;
        let external = flow
            .acquire(resource)
            .owner(owner)
            .timed_work(work)
            .submit()
            .unwrap();
        let before_scheduler = flow.budget_snapshot().scheduler;
        let failure = match bound.submit(&mut flow) {
            Ok(_) => panic!("a pre-existing request must block a bridge receipt"),
            Err(failure) => failure,
        };
        assert_eq!(failure.error, BridgeError::ConflictingSubmission);
        assert_eq!(failure.bound.work(), work);
        assert_eq!(failure.bound.draw_position(), draw_position);
        assert_eq!(flow.work(work).unwrap().request, Some(external));
        assert_eq!(flow.budget_snapshot().scheduler, before_scheduler);
        assert_eq!(flow.request(external).unwrap().work, Some(work));
    }
}

#[cfg(test)]
#[path = "../../../conformance/c20/transit_flow_c20.rs"]
mod transit_flow_c20;
