//! Canonical bounded wire for the complete calibration/Flow bridge DTOs.
//!
//! The payload carries native continuation data only. Flow, seed, and route
//! owner validators remain authoritative when applying a decoded image.
use super::*;
use crate::route_receipt::checkpoint_wire::RouteReceiptWireLimits;
use crate::seed_map::checkpoint_wire::{decode_stream, encode_stream_image, SeedWireLimits};
use kairo_ecs_des::FlowCheckpointWireLimits;
use std::error::Error;
use std::fmt::{Display, Formatter};

const MAGIC: &[u8; 8] = b"KBRDG1\0\0";
const SCHEMA: u16 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct BridgeWireLimits {
    pub(crate) native: BridgeCheckpointLimits,
    pub(crate) max_wire_bytes: usize,
    pub(crate) seed: SeedWireLimits,
    pub(crate) flow: FlowCheckpointWireLimits,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum BridgeWireError {
    UnsupportedSchema(u16),
    InvalidTag,
    InvalidBoolean,
    InvalidUtf8,
    Truncated,
    TrailingBytes,
    LimitExceeded,
    AllocationFailed,
    InvalidData,
    Seed(String),
    Route(String),
    Dispatch(String),
}
impl Display for BridgeWireError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedSchema(v) => write!(f, "unsupported bridge wire schema {v}"),
            Self::InvalidTag => f.write_str("invalid bridge wire tag"),
            Self::InvalidBoolean => f.write_str("invalid bridge wire boolean"),
            Self::InvalidUtf8 => f.write_str("invalid bridge wire UTF-8"),
            Self::Truncated => f.write_str("truncated bridge wire data"),
            Self::TrailingBytes => f.write_str("trailing bridge wire data"),
            Self::LimitExceeded => f.write_str("bridge wire limit exceeded"),
            Self::AllocationFailed => f.write_str("bridge wire allocation failed"),
            Self::InvalidData => f.write_str("invalid bridge wire data"),
            Self::Seed(e) => write!(f, "seed wire: {e}"),
            Self::Route(e) => write!(f, "route wire: {e}"),
            Self::Dispatch(e) => write!(f, "dispatch wire: {e}"),
        }
    }
}
impl Error for BridgeWireError {}
type R<T = ()> = Result<T, BridgeWireError>;

impl BoundIntrinsicWorkCheckpointV1 {
    pub(crate) fn encode_wire_v1(&self, limits: BridgeWireLimits) -> R<Vec<u8>> {
        encode_bound(self, limits)
    }
    pub(crate) fn decode_wire_v1(bytes: &[u8], limits: BridgeWireLimits) -> R<Self> {
        preflight_bound(bytes, limits)?;
        decode_bound(bytes, limits)
    }
    pub(crate) fn work_entity_id(&self) -> EntityId {
        self.work
    }
    pub(crate) fn stream_identity(&self) -> &crate::seed_map::SeedIdentity {
        &self.stream.identity
    }
}

impl SubmittedIntrinsicWorkCheckpointV1 {
    pub(crate) fn encode_wire_v1(&self, limits: BridgeWireLimits) -> R<Vec<u8>> {
        encode_submitted(self, limits)
    }
    pub(crate) fn decode_wire_v1(bytes: &[u8], limits: BridgeWireLimits) -> R<Self> {
        preflight_submitted(bytes, limits)?;
        decode_submitted(bytes, limits)
    }
    pub(crate) fn work_entity_id(&self) -> EntityId {
        self.work
    }
    pub(crate) fn stream_identity(&self) -> &crate::seed_map::SeedIdentity {
        &self.stream.identity
    }
}

/// Rejects a bridge packet captured on the wrong side of a Flow dispatch.
/// This compares the actual source carrier to the captured Flow context row,
/// then checks the event/command/scheduler frontier that the bridge observed.
pub(crate) fn validate_coherent_cut<T: Clone + 'static, C: 'static>(
    flow_image: &kairo_ecs_des::FlowCheckpointV1,
    source: &FlowRuntime,
    bound: &BoundIntrinsicWork<T, C>,
    limits: BridgeCheckpointLimits,
) -> Result<(), BridgeCheckpointError> {
    if bound.runtime != source.identity() {
        return Err(BridgeCheckpointError::InvalidState);
    }
    let Some(carrier) = bound.carrier else {
        return Ok(());
    };
    let context = source
        .work_context::<TransitContext>(carrier)
        .map_err(BridgeCheckpointError::Flow)?;
    let context_limits = kairo_ecs_abm::TransitContextCheckpointLimitsV1::new(
        limits.max_route_segments,
        limits.max_canonical_bytes,
        limits.max_identifier_bytes,
        limits.max_canonical_bytes.saturating_add(4096),
    );
    let native = context
        .checkpoint_v1(&source.identity(), context_limits)
        .map_err(|_| BridgeCheckpointError::InvalidState)?;
    let context_wire = native
        .encode_bytes_v1(context_limits)
        .map_err(|_| BridgeCheckpointError::InvalidState)?;
    let work = carrier.entity_id();
    let registration = flow_image
        .work_registrations
        .iter()
        .find(|row| row.id == work)
        .ok_or(BridgeCheckpointError::InvalidState)?;
    let context_store = flow_image
        .context_stores
        .iter()
        .find(|store| store.codec_key == registration.context_codec_key)
        .ok_or(BridgeCheckpointError::InvalidState)?;
    let stored = context_store
        .rows
        .iter()
        .find(|(owner, _)| *owner == work)
        .ok_or(BridgeCheckpointError::InvalidState)?;
    if stored.1 != context_wire
        || native.service_work != bound.work.entity_id()
        || (!native.initial_start_pending
            && (native.carrier != Some(work) || native.kind != bound.kind))
        || native.acquire_owner != bound.acquire.owner
        || native.acquire_resource != bound.acquire.resource.entity_id()
    {
        return Err(BridgeCheckpointError::InvalidState);
    }

    let has_live = |event: EventId, at: SimTime, priority: i32| {
        flow_image.scheduler.entries.iter().any(|entry| {
            entry.live
                && entry.id == event
                && entry.request.at == at
                && entry.request.priority == priority
                && Some(entry.request.kind) == bound.kind
        })
    };
    let has_domain_command = |event: EventId| {
        flow_image.commands.iter().any(|(id, command)| {
            *id == event
                && matches!(command, kairo_ecs_des::FlowCommandV1::Domain(id, kind)
                if *id == carrier && Some(*kind) == bound.kind)
        })
    };
    let has_submit_command = |event: EventId, request: RequestId| {
        flow_image.commands.iter().any(|(id, command)| {
            *id == event
                && matches!(command, kairo_ecs_des::FlowCommandV1::Submit(actual)
                    if actual.entity_id() == request.entity_id())
        })
    };
    match (
        native.phase,
        native.expected_event,
        native.expected_due_ticks,
        bound.pending_event,
    ) {
        (
            kairo_ecs_abm::TransitPhase::Ready | kairo_ecs_abm::TransitPhase::Moving,
            Some((index, generation)),
            Some(due),
            Some(pending),
        ) => {
            let event = EventId::new(index, generation);
            let observed = pending == event
                && bound.owned_events.contains(&event)
                && has_live(
                    event,
                    SimTime::from_ticks(due),
                    bound
                        .pending_priority
                        .ok_or(BridgeCheckpointError::InvalidState)?,
                )
                && has_domain_command(event);
            let rejected_for_retry = pending == event
                && bound.owned_events.contains(&event)
                && bound
                    .retryable
                    .as_ref()
                    .is_some_and(|dispatch| dispatch.event == event)
                && !has_live(
                    event,
                    SimTime::from_ticks(due),
                    bound
                        .pending_priority
                        .ok_or(BridgeCheckpointError::InvalidState)?,
                )
                && !has_domain_command(event);
            if !observed && !rejected_for_retry {
                return Err(BridgeCheckpointError::InvalidState);
            }
        }
        (
            kairo_ecs_abm::TransitPhase::Paused,
            Some((index, generation)),
            Some(due),
            Some(pending),
        ) => {
            let event = EventId::new(index, generation);
            if pending != event
                || !has_live(
                    event,
                    SimTime::from_ticks(due),
                    bound
                        .pending_priority
                        .ok_or(BridgeCheckpointError::InvalidState)?,
                )
                || !has_domain_command(event)
            {
                return Err(BridgeCheckpointError::InvalidState);
            }
        }
        (kairo_ecs_abm::TransitPhase::Paused, None, None, Some(event)) => {
            if !bound.stale_events.contains(&event) || !bound.consumed_events.contains(&event) {
                return Err(BridgeCheckpointError::InvalidState);
            }
        }
        (kairo_ecs_abm::TransitPhase::Arrived, None, None, Some(event)) => {
            if !bound.owned_events.contains(&event)
                || bound.arrival_request.is_none()
                || bound.arrival_at.is_none()
                || !has_submit_command(
                    event,
                    bound
                        .arrival_request
                        .ok_or(BridgeCheckpointError::InvalidState)?,
                )
                || !flow_image.scheduler.entries.iter().any(|entry| {
                    entry.live
                        && entry.id == event
                        && Some(entry.request.at) == bound.arrival_at
                        && Some(entry.request.priority) == bound.pending_priority
                })
            {
                return Err(BridgeCheckpointError::InvalidState);
            }
        }
        (_, None, None, None) => {}
        _ => return Err(BridgeCheckpointError::InvalidState),
    }
    for (event, control) in &bound.controls {
        let Some(entry) = flow_image
            .scheduler
            .entries
            .iter()
            .find(|entry| entry.live && entry.id == *event)
        else {
            return Err(BridgeCheckpointError::InvalidState);
        };
        let action = match control {
            FlowDomainControl::Pause => kairo_ecs_des::FlowDomainControl::Pause,
            FlowDomainControl::Resume => kairo_ecs_des::FlowDomainControl::Resume,
        };
        if !flow_image.commands.iter().any(|(id, command)| {
            *id == *event
                && matches!(command,
            kairo_ecs_des::FlowCommandV1::DomainControl(id, kind, actual)
                if *id == carrier && Some(*kind) == bound.kind && *actual == action)
        }) || (entry.request.entity.is_some() && entry.request.entity != Some(work))
        {
            return Err(BridgeCheckpointError::InvalidState);
        }
    }
    Ok(())
}

fn route_limits(l: BridgeWireLimits) -> RouteReceiptWireLimits {
    RouteReceiptWireLimits {
        native: RouteReceiptCheckpointLimits {
            max_identifier_bytes: l.native.max_identifier_bytes,
            max_canonical_bytes: l.native.max_canonical_bytes,
        },
        max_wire_bytes: l.max_wire_bytes,
    }
}
fn cap(bytes: &[u8], limits: BridgeWireLimits) -> R {
    if bytes.len() > limits.max_wire_bytes {
        Err(BridgeWireError::LimitExceeded)
    } else {
        Ok(())
    }
}
fn encode_header(w: &mut Writer, tag: u8) {
    w.raw(MAGIC);
    w.u16(SCHEMA);
    w.u8(tag);
}
fn encode_bound(image: &BoundIntrinsicWorkCheckpointV1, l: BridgeWireLimits) -> R<Vec<u8>> {
    if image.version != 1
        || image.owned_events.len() > l.native.max_owned_events
        || image.stale_events.len() > l.native.max_owned_events
        || image.consumed_events.len() > l.native.max_owned_events
        || image.controls.len() > l.native.max_controls
    {
        return Err(BridgeWireError::LimitExceeded);
    }
    let expected_length = measure_bound(image, l)?;
    let seed = encode_stream_image(&image.stream, l.seed)
        .map_err(|e| BridgeWireError::Seed(e.to_string()))?;
    let route = match &image.route_receipt {
        Some(x) => Some(
            x.encode_wire_v1(route_limits(l))
                .map_err(|e| BridgeWireError::Route(e.to_string()))?,
        ),
        None => None,
    };
    let meta = match &image.route_metadata {
        Some(x) => Some(
            x.encode_wire_v1(route_limits(l))
                .map_err(|e| BridgeWireError::Route(e.to_string()))?,
        ),
        None => None,
    };
    let dispatch = match &image.retryable {
        Some(x) => Some(
            x.encode_wire_v1(l.flow)
                .map_err(|e| BridgeWireError::Dispatch(e.to_string()))?,
        ),
        None => None,
    };
    let mut w = Writer::with_capacity(expected_length, l.max_wire_bytes)?;
    encode_header(&mut w, 0);
    w.u32(image.version);
    w.u8(decision_tag(image.decision));
    w.u32(image.decision.policy_version);
    w.bytes(&seed)?;
    w.u128(image.duration_ticks);
    w.u64(image.draw_before);
    w.u64(image.draw_after);
    encode_acquire(&mut w, &image.acquire);
    encode_transit(&mut w, &image.transit)?;
    w.option_bytes(meta.as_deref())?;
    w.option_bytes(route.as_deref())?;
    w.entity(image.work);
    w.option_entity(image.carrier);
    w.option_entity(image.carrier_actor);
    w.option_kind(image.kind);
    w.option_event(image.pending_event);
    w.option_i32(image.pending_priority);
    w.events(&image.owned_events)?;
    w.events(&image.stale_events)?;
    w.events(&image.consumed_events)?;
    w.count(image.controls.len())?;
    for (event, control) in &image.controls {
        w.event(*event);
        w.u8(match control {
            FlowDomainControl::Pause => 0,
            FlowDomainControl::Resume => 1,
        });
    }
    w.option_bytes(dispatch.as_deref())?;
    w.option_entity(image.arrival_request);
    w.option_time(image.arrival_at);
    let bytes = w.finish()?;
    if bytes.len() != expected_length {
        return Err(BridgeWireError::InvalidData);
    }
    Ok(bytes)
}
fn encode_submitted(image: &SubmittedIntrinsicWorkCheckpointV1, l: BridgeWireLimits) -> R<Vec<u8>> {
    if image.version != 1 {
        return Err(BridgeWireError::InvalidData);
    }
    let expected_length = measure_submitted(image, l)?;
    let seed = encode_stream_image(&image.stream, l.seed)
        .map_err(|e| BridgeWireError::Seed(e.to_string()))?;
    let mut w = Writer::with_capacity(expected_length, l.max_wire_bytes)?;
    encode_header(&mut w, 1);
    w.u32(image.version);
    w.u8(decision_tag(image.decision));
    w.u32(image.decision.policy_version);
    w.bytes(&seed)?;
    w.u128(image.duration_ticks);
    w.u64(image.draw_before);
    w.u64(image.draw_after);
    encode_acquire(&mut w, &image.acquire);
    w.entity(image.work);
    w.entity(image.request);
    let bytes = w.finish()?;
    if bytes.len() != expected_length {
        return Err(BridgeWireError::InvalidData);
    }
    Ok(bytes)
}
fn measure_bound(x: &BoundIntrinsicWorkCheckpointV1, l: BridgeWireLimits) -> R<usize> {
    let mut id_bytes = stream_ids(&x.stream)?;
    let mut canonical_bytes = 0usize;
    if let TransitRequestCheckpointV1::Route {
        mode,
        carrier_registration,
        graph_canonical_bytes,
        ..
    } = &x.transit
    {
        id_bytes = add(id_bytes, mode.len())?;
        id_bytes = add(id_bytes, carrier_registration.len())?;
        canonical_bytes = add(canonical_bytes, graph_canonical_bytes.len())?;
    }
    if let Some(meta) = &x.route_metadata {
        id_bytes = add(
            id_bytes,
            meta.checkpoint_identifier_bytes()
                .map_err(|_| BridgeWireError::InvalidData)?,
        )?;
    }
    if let Some(receipt) = &x.route_receipt {
        id_bytes = add(
            id_bytes,
            receipt
                .metadata_identifier_bytes()
                .map_err(|_| BridgeWireError::InvalidData)?,
        )?;
        canonical_bytes = add(canonical_bytes, receipt.canonical_bytes_len())?;
    }
    if id_bytes > l.native.max_identifier_bytes || canonical_bytes > l.native.max_canonical_bytes {
        return Err(BridgeWireError::LimitExceeded);
    }
    let seed = add(78, stream_ids(&x.stream)?)?;
    let mut n = 11;
    for part in [
        4,
        1,
        4,
        8,
        seed,
        16,
        16,
        acquire_len(x.acquire.deadline.is_some()),
    ] {
        n = add(n, part)?;
    }
    n = add(n, transit_len(&x.transit)?)?;
    n = add(
        n,
        option_len(
            x.route_metadata
                .as_ref()
                .map(|m| m.wire_len_v1(route_limits(l)))
                .transpose()
                .map_err(|e| BridgeWireError::Route(e.to_string()))?
                .unwrap_or(0),
        )?,
    )?;
    n = add(
        n,
        option_len(
            x.route_receipt
                .as_ref()
                .map(|m| m.wire_len_v1(route_limits(l)))
                .transpose()
                .map_err(|e| BridgeWireError::Route(e.to_string()))?
                .unwrap_or(0),
        )?,
    )?;
    n = add(
        n,
        [
            12,
            option_entity_len(x.carrier),
            option_entity_len(x.carrier_actor),
            option_kind_len(x.kind),
            option_event_len(x.pending_event),
            option_i32_len(x.pending_priority),
        ]
        .into_iter()
        .try_fold(0usize, add)?,
    )?;
    for events in [&x.owned_events, &x.stale_events, &x.consumed_events] {
        if events.len() > l.native.max_owned_events {
            return Err(BridgeWireError::LimitExceeded);
        }
        n = add(
            n,
            add(
                8,
                events
                    .len()
                    .checked_mul(12)
                    .ok_or(BridgeWireError::LimitExceeded)?,
            )?,
        )?;
    }
    if x.controls.len() > l.native.max_controls {
        return Err(BridgeWireError::LimitExceeded);
    }
    n = add(
        n,
        add(
            8,
            x.controls
                .len()
                .checked_mul(13)
                .ok_or(BridgeWireError::LimitExceeded)?,
        )?,
    )?;
    if let Some(dispatch) = &x.retryable {
        n = add(
            n,
            option_len(
                dispatch
                    .encoded_wire_len_v1(l.flow)
                    .map_err(|e| BridgeWireError::Dispatch(e.to_string()))?,
            )?,
        )?;
    } else {
        n = add(n, 1)?;
    }
    n = add(
        n,
        add(
            option_entity_len(x.arrival_request),
            option_time_len(x.arrival_at),
        )?,
    )?;
    if n > l.max_wire_bytes {
        return Err(BridgeWireError::LimitExceeded);
    }
    Ok(n)
}
fn measure_submitted(x: &SubmittedIntrinsicWorkCheckpointV1, l: BridgeWireLimits) -> R<usize> {
    let ids = stream_ids(&x.stream)?;
    if ids > l.native.max_identifier_bytes {
        return Err(BridgeWireError::LimitExceeded);
    }
    let seed = add(78, ids)?;
    let n = [
        11,
        4,
        1,
        4,
        8,
        seed,
        16,
        16,
        acquire_len(x.acquire.deadline.is_some()),
        24,
    ]
    .into_iter()
    .try_fold(0usize, add)?;
    if n > l.max_wire_bytes {
        return Err(BridgeWireError::LimitExceeded);
    }
    Ok(n)
}
fn acquire_len(deadline: bool) -> usize {
    51 + usize::from(deadline) * 16
}
fn option_len(n: usize) -> R<usize> {
    if n == 0 {
        Ok(1)
    } else {
        add(9, n)
    }
}
fn option_entity_len(x: Option<EntityId>) -> usize {
    if x.is_some() {
        13
    } else {
        1
    }
}
fn option_event_len(x: Option<EventId>) -> usize {
    if x.is_some() {
        13
    } else {
        1
    }
}
fn option_kind_len(x: Option<EventKind>) -> usize {
    if x.is_some() {
        5
    } else {
        1
    }
}
fn option_i32_len(x: Option<i32>) -> usize {
    if x.is_some() {
        5
    } else {
        1
    }
}
fn option_time_len(x: Option<SimTime>) -> usize {
    if x.is_some() {
        17
    } else {
        1
    }
}
fn transit_len(x: &TransitRequestCheckpointV1) -> R<usize> {
    match x {
        TransitRequestCheckpointV1::Zero => Ok(1),
        TransitRequestCheckpointV1::Route {
            graph_canonical_bytes,
            mode,
            carrier_registration,
            ..
        } => {
            if graph_canonical_bytes.len() > u32::MAX as usize {
                return Err(BridgeWireError::LimitExceeded);
            }
            let mut n = 1usize;
            for part in [
                4,
                8,
                graph_canonical_bytes.len(),
                8,
                8,
                8,
                mode.len(),
                8,
                8,
                12,
                8,
                carrier_registration.len(),
                4,
            ] {
                n = add(n, part)?;
            }
            Ok(n)
        }
    }
}
fn add(a: usize, b: usize) -> R<usize> {
    a.checked_add(b).ok_or(BridgeWireError::LimitExceeded)
}
fn stream_ids(stream: &CalibrationStreamStateV1) -> R<usize> {
    let id = &stream.identity;
    let total = id
        .study_id
        .len()
        .checked_add(id.seed_schedule_id.len())
        .and_then(|n| n.checked_add(id.case_key.len()))
        .and_then(|n| n.checked_add(id.task_key.len()))
        .ok_or(BridgeWireError::LimitExceeded)?;
    Ok(total)
}
fn decision_tag(x: FidelityDecision) -> u8 {
    let mode = match x.mode {
        FidelityMode::Macro => 0,
        FidelityMode::Micro => 1,
    };
    let scope = match x.scope {
        kairo_ecs_des::fidelity::FidelityScope::EntitySubsystem => 0,
        kairo_ecs_des::fidelity::FidelityScope::Entity => 1,
        kairo_ecs_des::fidelity::FidelityScope::Subsystem => 2,
        kairo_ecs_des::fidelity::FidelityScope::Global => 3,
    };
    mode | (scope << 1)
}
fn decode_decision(tag: u8, version: u32) -> R<FidelityDecision> {
    let mode = match tag & 1 {
        0 => FidelityMode::Macro,
        _ => FidelityMode::Micro,
    };
    let scope = match tag >> 1 {
        0 => kairo_ecs_des::fidelity::FidelityScope::EntitySubsystem,
        1 => kairo_ecs_des::fidelity::FidelityScope::Entity,
        2 => kairo_ecs_des::fidelity::FidelityScope::Subsystem,
        3 => kairo_ecs_des::fidelity::FidelityScope::Global,
        _ => return Err(BridgeWireError::InvalidTag),
    };
    if tag > 7 {
        return Err(BridgeWireError::InvalidTag);
    }
    Ok(FidelityDecision {
        mode,
        scope,
        policy_version: version,
    })
}

fn preflight_bound(bytes: &[u8], l: BridgeWireLimits) -> R {
    cap(bytes, l)?;
    let mut r = Reader::new(bytes);
    r.header(0)?;
    let version = r.u32()?;
    if version != 1 {
        return Err(BridgeWireError::InvalidData);
    }
    let tag = r.u8()?;
    let policy_version = r.u32()?;
    decode_decision(tag, policy_version)?;
    let seed = r.bytes()?;
    let mut identifier_bytes = preflight_seed(seed, l.seed)?;
    r.raw(32)?;
    r.acquire()?;
    let (transit_ids, transit_canonical) = r.transit(l)?;
    identifier_bytes = add(identifier_bytes, transit_ids)?;
    let mut canonical_bytes = transit_canonical;
    if let Some(n) = r.option_nested_value(|bytes| {
        RouteMetadataCheckpointV1::preflight_wire_v1(bytes, route_limits(l))
            .map_err(|e| BridgeWireError::Route(e.to_string()))
    })? {
        identifier_bytes = add(identifier_bytes, n)?;
    }
    if let Some((n, canonical_len)) = r.option_nested_value(|bytes| {
        RouteReceiptCheckpointV1::preflight_wire_v1(bytes, route_limits(l))
            .map_err(|e| BridgeWireError::Route(e.to_string()))
    })? {
        identifier_bytes = add(identifier_bytes, n)?;
        canonical_bytes = add(canonical_bytes, canonical_len)?;
    }
    if identifier_bytes > l.native.max_identifier_bytes
        || canonical_bytes > l.native.max_canonical_bytes
    {
        return Err(BridgeWireError::LimitExceeded);
    }
    r.entity()?;
    r.option_entity()?;
    r.option_entity()?;
    r.option_kind()?;
    r.option_event()?;
    r.option_i32()?;
    let event_count = r.events(l.native.max_owned_events)?
        + r.events(l.native.max_owned_events)?
        + r.events(l.native.max_owned_events)?;
    if event_count > l.native.max_owned_events.saturating_mul(3) {
        return Err(BridgeWireError::LimitExceeded);
    }
    let n = r.count()?;
    if n > l.native.max_controls {
        return Err(BridgeWireError::LimitExceeded);
    }
    for _ in 0..n {
        r.event()?;
        if r.u8()? > 1 {
            return Err(BridgeWireError::InvalidTag);
        }
    }
    r.option_nested(|bytes| {
        FlowDispatch::preflight_wire_v1(bytes, l.flow)
            .map_err(|e| BridgeWireError::Dispatch(e.to_string()))
    })?;
    r.option_entity()?;
    r.option_time()?;
    r.finish()
}
fn preflight_submitted(bytes: &[u8], l: BridgeWireLimits) -> R {
    cap(bytes, l)?;
    let mut r = Reader::new(bytes);
    r.header(1)?;
    if r.u32()? != 1 {
        return Err(BridgeWireError::InvalidData);
    }
    let tag = r.u8()?;
    let policy_version = r.u32()?;
    decode_decision(tag, policy_version)?;
    preflight_seed(r.bytes()?, l.seed)?;
    r.raw(32)?;
    r.acquire()?;
    r.entity()?;
    r.entity()?;
    r.finish()
}
fn preflight_seed(bytes: &[u8], limits: SeedWireLimits) -> R<usize> {
    if bytes.len() > limits.max_wire_bytes {
        return Err(BridgeWireError::LimitExceeded);
    }
    let mut r = Reader::new(bytes);
    if r.raw(4)? != b"KSTW" || r.u8()? != 1 {
        return Err(BridgeWireError::InvalidTag);
    }
    if r.u32()? != 1 {
        return Err(BridgeWireError::InvalidData);
    }
    r.u32()?;
    r.u64()?;
    r.u64()?;
    let mut ids = 0usize;
    for _ in 0..4 {
        let text = r.u32_string_ref(limits.max_identifier_bytes)?;
        ids = add(ids, text.len())?;
    }
    if ids > limits.max_identifier_bytes {
        return Err(BridgeWireError::LimitExceeded);
    }
    if !(1..=4).contains(&r.u8()?) {
        return Err(BridgeWireError::InvalidTag);
    }
    let map_version = r.u32()?;
    let stream_version = r.u32()?;
    if map_version != 1 || stream_version != 1 {
        return Err(BridgeWireError::InvalidData);
    }
    r.raw(24)?;
    r.finish()?;
    Ok(ids)
}

fn decode_bound(bytes: &[u8], l: BridgeWireLimits) -> R<BoundIntrinsicWorkCheckpointV1> {
    let mut r = Reader::new(bytes);
    r.header(0)?;
    let version = r.u32()?;
    let decision = decode_decision(r.u8()?, r.u32()?)?;
    let stream =
        decode_stream(r.bytes()?, l.seed).map_err(|e| BridgeWireError::Seed(e.to_string()))?;
    let duration_ticks = r.u128()?;
    let draw_before = r.u64()?;
    let draw_after = r.u64()?;
    let acquire = r.acquire_owned()?;
    let transit = r.transit_owned(l)?;
    let route_metadata = r.option_owned(|b| {
        RouteMetadataCheckpointV1::decode_wire_v1(b, route_limits(l))
            .map_err(|e| BridgeWireError::Route(e.to_string()))
    })?;
    let route_receipt = r.option_owned(|b| {
        RouteReceiptCheckpointV1::decode_wire_v1(b, route_limits(l))
            .map_err(|e| BridgeWireError::Route(e.to_string()))
    })?;
    let work = r.entity()?;
    let carrier = r.option_entity()?;
    let carrier_actor = r.option_entity()?;
    let kind = r.option_kind()?;
    let pending_event = r.option_event()?;
    let pending_priority = r.option_i32()?;
    let owned_events = r.events_owned(l.native.max_owned_events)?;
    let stale_events = r.events_owned(l.native.max_owned_events)?;
    let consumed_events = r.events_owned(l.native.max_owned_events)?;
    let n = r.count()?;
    let mut controls = Vec::new();
    controls
        .try_reserve_exact(n)
        .map_err(|_| BridgeWireError::AllocationFailed)?;
    for _ in 0..n {
        let e = r.event()?;
        let c = match r.u8()? {
            0 => FlowDomainControl::Pause,
            1 => FlowDomainControl::Resume,
            _ => return Err(BridgeWireError::InvalidTag),
        };
        controls.push((e, c));
    }
    let retryable = r.option_owned(|b| {
        FlowDispatch::decode_wire_v1(b, l.flow)
            .map_err(|e| BridgeWireError::Dispatch(e.to_string()))
    })?;
    let arrival_request = r.option_entity()?;
    let arrival_at = r.option_time()?;
    r.finish()?;
    Ok(BoundIntrinsicWorkCheckpointV1 {
        version,
        decision,
        stream,
        duration_ticks,
        draw_before,
        draw_after,
        acquire,
        transit,
        route_metadata,
        route_receipt,
        work,
        carrier,
        carrier_actor,
        kind,
        pending_event,
        pending_priority,
        owned_events,
        stale_events,
        consumed_events,
        controls,
        retryable,
        arrival_request,
        arrival_at,
    })
}
fn decode_submitted(bytes: &[u8], l: BridgeWireLimits) -> R<SubmittedIntrinsicWorkCheckpointV1> {
    let mut r = Reader::new(bytes);
    r.header(1)?;
    let version = r.u32()?;
    let decision = decode_decision(r.u8()?, r.u32()?)?;
    let stream =
        decode_stream(r.bytes()?, l.seed).map_err(|e| BridgeWireError::Seed(e.to_string()))?;
    let duration_ticks = r.u128()?;
    let draw_before = r.u64()?;
    let draw_after = r.u64()?;
    let acquire = r.acquire_owned()?;
    let work = r.entity()?;
    let request = r.entity()?;
    r.finish()?;
    Ok(SubmittedIntrinsicWorkCheckpointV1 {
        version,
        decision,
        stream,
        duration_ticks,
        draw_before,
        draw_after,
        acquire,
        work,
        request,
    })
}

fn encode_acquire(w: &mut Writer, a: &AcquireIntentCheckpointV1) {
    w.entity(a.resource);
    w.entity(a.owner);
    w.u128(a.at.ticks());
    w.i32(a.priority_level);
    w.option_time(a.deadline);
    w.i32(a.scheduler_priority);
    w.u8(u8::from(a.can_preempt));
    match a.preemptible {
        None => w.u8(0),
        Some(PreemptionStrategy::Suspend) => w.u8(1),
        Some(PreemptionStrategy::Abort) => w.u8(2),
        Some(PreemptionStrategy::Restart) => w.u8(3),
    }
}
fn encode_transit(w: &mut Writer, t: &TransitRequestCheckpointV1) -> R {
    match t {
        TransitRequestCheckpointV1::Zero => w.u8(0),
        TransitRequestCheckpointV1::Route {
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
        } => {
            w.u8(1);
            w.u32(*graph_version);
            w.bytes(graph_canonical_bytes)?;
            w.u64(*origin);
            w.u64(*destination);
            w.string(mode)?;
            w.u64(*speed_mm_per_second);
            w.u64(*ticks_per_second);
            w.entity(*carrier_actor);
            w.string(carrier_registration)?;
            w.kind(*kind)
        }
    }
    Ok(())
}

struct Writer {
    bytes: Vec<u8>,
    cap: usize,
    failed: bool,
}
impl Writer {
    fn with_capacity(length: usize, cap: usize) -> R<Self> {
        if length > cap {
            return Err(BridgeWireError::LimitExceeded);
        }
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(length)
            .map_err(|_| BridgeWireError::AllocationFailed)?;
        Ok(Self {
            bytes,
            cap,
            failed: false,
        })
    }
    fn raw(&mut self, b: &[u8]) {
        let Some(len) = self.bytes.len().checked_add(b.len()) else {
            self.failed = true;
            return;
        };
        if len > self.cap {
            self.failed = true;
            return;
        }
        if self.bytes.try_reserve(b.len()).is_err() {
            self.failed = true;
            return;
        }
        self.bytes.extend_from_slice(b)
    }
    fn u8(&mut self, x: u8) {
        self.raw(&[x])
    }
    fn u16(&mut self, x: u16) {
        self.raw(&x.to_le_bytes())
    }
    fn u32(&mut self, x: u32) {
        self.raw(&x.to_le_bytes())
    }
    fn i32(&mut self, x: i32) {
        self.raw(&x.to_le_bytes())
    }
    fn u64(&mut self, x: u64) {
        self.raw(&x.to_le_bytes())
    }
    fn u128(&mut self, x: u128) {
        self.raw(&x.to_le_bytes())
    }
    fn count(&mut self, n: usize) -> R {
        self.u64(u64::try_from(n).map_err(|_| BridgeWireError::LimitExceeded)?);
        Ok(())
    }
    fn bytes(&mut self, b: &[u8]) -> R {
        self.count(b.len())?;
        self.raw(b);
        if self.failed {
            return Err(BridgeWireError::LimitExceeded);
        }
        Ok(())
    }
    fn string(&mut self, s: &str) -> R {
        self.bytes(s.as_bytes())
    }
    fn entity(&mut self, x: EntityId) {
        self.u64(x.index);
        self.u32(x.generation)
    }
    fn event(&mut self, x: EventId) {
        self.u64(x.index);
        self.u32(x.generation)
    }
    fn kind(&mut self, x: EventKind) {
        self.u32(x.code())
    }
    fn option_entity(&mut self, x: Option<EntityId>) {
        self.u8(u8::from(x.is_some()));
        if let Some(x) = x {
            self.entity(x)
        }
    }
    fn option_event(&mut self, x: Option<EventId>) {
        self.u8(u8::from(x.is_some()));
        if let Some(x) = x {
            self.event(x)
        }
    }
    fn option_kind(&mut self, x: Option<EventKind>) {
        self.u8(u8::from(x.is_some()));
        if let Some(x) = x {
            self.kind(x)
        }
    }
    fn option_i32(&mut self, x: Option<i32>) {
        self.u8(u8::from(x.is_some()));
        if let Some(x) = x {
            self.i32(x)
        }
    }
    fn option_time(&mut self, x: Option<SimTime>) {
        self.u8(u8::from(x.is_some()));
        if let Some(x) = x {
            self.u128(x.ticks())
        }
    }
    fn events(&mut self, x: &[EventId]) -> R {
        self.count(x.len())?;
        for e in x {
            self.event(*e)
        }
        if self.failed {
            return Err(BridgeWireError::LimitExceeded);
        }
        Ok(())
    }
    fn option_bytes(&mut self, x: Option<&[u8]>) -> R {
        self.u8(u8::from(x.is_some()));
        if let Some(x) = x {
            self.bytes(x)?
        }
        Ok(())
    }
    fn finish(self) -> R<Vec<u8>> {
        if self.failed {
            Err(BridgeWireError::LimitExceeded)
        } else {
            Ok(self.bytes)
        }
    }
}

struct Reader<'a> {
    b: &'a [u8],
    p: usize,
}
impl<'a> Reader<'a> {
    fn new(b: &'a [u8]) -> Self {
        Self { b, p: 0 }
    }
    fn raw(&mut self, n: usize) -> R<&'a [u8]> {
        let e = self
            .p
            .checked_add(n)
            .ok_or(BridgeWireError::LimitExceeded)?;
        let x = self.b.get(self.p..e).ok_or(BridgeWireError::Truncated)?;
        self.p = e;
        Ok(x)
    }
    fn u8(&mut self) -> R<u8> {
        Ok(self.raw(1)?[0])
    }
    fn u16(&mut self) -> R<u16> {
        Ok(u16::from_le_bytes(self.raw(2)?.try_into().unwrap()))
    }
    fn u32(&mut self) -> R<u32> {
        Ok(u32::from_le_bytes(self.raw(4)?.try_into().unwrap()))
    }
    fn i32(&mut self) -> R<i32> {
        Ok(i32::from_le_bytes(self.raw(4)?.try_into().unwrap()))
    }
    fn u64(&mut self) -> R<u64> {
        Ok(u64::from_le_bytes(self.raw(8)?.try_into().unwrap()))
    }
    fn u128(&mut self) -> R<u128> {
        Ok(u128::from_le_bytes(self.raw(16)?.try_into().unwrap()))
    }
    fn count(&mut self) -> R<usize> {
        usize::try_from(self.u64()?).map_err(|_| BridgeWireError::LimitExceeded)
    }
    fn bytes(&mut self) -> R<&'a [u8]> {
        let n = self.count()?;
        self.raw(n)
    }
    fn str_ref(&mut self, cap: usize) -> R<&'a str> {
        let bytes = self.bytes()?;
        if bytes.len() > cap {
            return Err(BridgeWireError::LimitExceeded);
        }
        std::str::from_utf8(bytes).map_err(|_| BridgeWireError::InvalidUtf8)
    }
    fn u32_string_ref(&mut self, cap: usize) -> R<&'a str> {
        let n = usize::try_from(self.u32()?).map_err(|_| BridgeWireError::LimitExceeded)?;
        if n > cap {
            return Err(BridgeWireError::LimitExceeded);
        }
        std::str::from_utf8(self.raw(n)?).map_err(|_| BridgeWireError::InvalidUtf8)
    }
    fn string(&mut self, cap: usize) -> R<String> {
        let b = self.bytes()?;
        if b.len() > cap {
            return Err(BridgeWireError::LimitExceeded);
        }
        let s = std::str::from_utf8(b).map_err(|_| BridgeWireError::InvalidUtf8)?;
        Ok(s.to_owned())
    }
    fn header(&mut self, tag: u8) -> R {
        if self.raw(8)? != MAGIC {
            return Err(BridgeWireError::InvalidTag);
        }
        let s = self.u16()?;
        if s != SCHEMA {
            return Err(BridgeWireError::UnsupportedSchema(s));
        }
        if self.u8()? != tag {
            return Err(BridgeWireError::InvalidTag);
        }
        Ok(())
    }
    fn finish(&self) -> R {
        if self.p == self.b.len() {
            Ok(())
        } else {
            Err(BridgeWireError::TrailingBytes)
        }
    }
    fn entity(&mut self) -> R<EntityId> {
        Ok(EntityId::new(self.u64()?, self.u32()?))
    }
    fn event(&mut self) -> R<EventId> {
        Ok(EventId::new(self.u64()?, self.u32()?))
    }
    fn kind(&mut self) -> R<EventKind> {
        Ok(EventKind::custom(self.u32()?))
    }
    fn boolean(&mut self) -> R<bool> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(BridgeWireError::InvalidBoolean),
        }
    }
    fn option_entity(&mut self) -> R<Option<EntityId>> {
        if self.boolean()? {
            Ok(Some(self.entity()?))
        } else {
            Ok(None)
        }
    }
    fn option_event(&mut self) -> R<Option<EventId>> {
        if self.boolean()? {
            Ok(Some(self.event()?))
        } else {
            Ok(None)
        }
    }
    fn option_kind(&mut self) -> R<Option<EventKind>> {
        if self.boolean()? {
            Ok(Some(self.kind()?))
        } else {
            Ok(None)
        }
    }
    fn option_i32(&mut self) -> R<Option<i32>> {
        if self.boolean()? {
            Ok(Some(self.i32()?))
        } else {
            Ok(None)
        }
    }
    fn option_time(&mut self) -> R<Option<SimTime>> {
        if self.boolean()? {
            Ok(Some(SimTime::from_ticks(self.u128()?)))
        } else {
            Ok(None)
        }
    }
    fn events(&mut self, cap: usize) -> R<usize> {
        let n = self.count()?;
        if n > cap {
            return Err(BridgeWireError::LimitExceeded);
        }
        for _ in 0..n {
            self.event()?;
        }
        Ok(n)
    }
    fn option_nested_value<T, F>(&mut self, mut f: F) -> R<Option<T>>
    where
        F: FnMut(&[u8]) -> R<T>,
    {
        if self.boolean()? {
            let b = self.bytes()?;
            Ok(Some(f(b)?))
        } else {
            Ok(None)
        }
    }
    fn events_owned(&mut self, cap: usize) -> R<Vec<EventId>> {
        let n = self.count()?;
        if n > cap {
            return Err(BridgeWireError::LimitExceeded);
        }
        let mut v = Vec::new();
        v.try_reserve_exact(n)
            .map_err(|_| BridgeWireError::AllocationFailed)?;
        for _ in 0..n {
            v.push(self.event()?)
        }
        Ok(v)
    }
    fn option_nested<F>(&mut self, mut f: F) -> R
    where
        F: FnMut(&[u8]) -> R,
    {
        if self.boolean()? {
            let b = self.bytes()?;
            f(b)?;
        }
        Ok(())
    }
    fn option_owned<T, F>(&mut self, mut f: F) -> R<Option<T>>
    where
        F: FnMut(&[u8]) -> R<T>,
    {
        if self.boolean()? {
            let b = self.bytes()?;
            Ok(Some(f(b)?))
        } else {
            Ok(None)
        }
    }
    fn acquire(&mut self) -> R {
        self.entity()?;
        self.entity()?;
        self.u128()?;
        self.i32()?;
        if self.boolean()? {
            self.u128()?;
        }
        self.i32()?;
        self.boolean()?;
        if self.u8()? > 3 {
            return Err(BridgeWireError::InvalidTag);
        }
        Ok(())
    }
    fn acquire_owned(&mut self) -> R<AcquireIntentCheckpointV1> {
        let resource = self.entity()?;
        let owner = self.entity()?;
        let at = SimTime::from_ticks(self.u128()?);
        let priority_level = self.i32()?;
        let deadline = self.option_time()?;
        let scheduler_priority = self.i32()?;
        let can_preempt = self.boolean()?;
        let preemptible = match self.u8()? {
            0 => None,
            1 => Some(PreemptionStrategy::Suspend),
            2 => Some(PreemptionStrategy::Abort),
            3 => Some(PreemptionStrategy::Restart),
            _ => return Err(BridgeWireError::InvalidTag),
        };
        Ok(AcquireIntentCheckpointV1 {
            resource,
            owner,
            at,
            priority_level,
            deadline,
            scheduler_priority,
            can_preempt,
            preemptible,
        })
    }
    fn transit(&mut self, l: BridgeWireLimits) -> R<(usize, usize)> {
        match self.u8()? {
            0 => Ok((0, 0)),
            1 => {
                self.u32()?;
                let b = self.bytes()?;
                if b.len() > l.native.max_canonical_bytes {
                    return Err(BridgeWireError::LimitExceeded);
                }
                self.u64()?;
                self.u64()?;
                let mode = self.str_ref(l.native.max_identifier_bytes)?;
                self.u64()?;
                self.u64()?;
                self.entity()?;
                let registration = self.str_ref(l.native.max_identifier_bytes)?;
                self.kind()?;
                Ok((add(mode.len(), registration.len())?, b.len()))
            }
            _ => Err(BridgeWireError::InvalidTag),
        }
    }
    fn transit_owned(&mut self, l: BridgeWireLimits) -> R<TransitRequestCheckpointV1> {
        match self.u8()? {
            0 => Ok(TransitRequestCheckpointV1::Zero),
            1 => {
                let graph_version = self.u32()?;
                let graph_canonical_bytes = self.bytes()?.to_vec();
                if graph_canonical_bytes.len() > l.native.max_canonical_bytes {
                    return Err(BridgeWireError::LimitExceeded);
                }
                let origin = self.u64()?;
                let destination = self.u64()?;
                let mode = self.string(l.native.max_identifier_bytes)?;
                let speed_mm_per_second = self.u64()?;
                let ticks_per_second = self.u64()?;
                let carrier_actor = self.entity()?;
                let carrier_registration = self.string(l.native.max_identifier_bytes)?;
                let kind = self.kind()?;
                Ok(TransitRequestCheckpointV1::Route {
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
                })
            }
            _ => Err(BridgeWireError::InvalidTag),
        }
    }
}
