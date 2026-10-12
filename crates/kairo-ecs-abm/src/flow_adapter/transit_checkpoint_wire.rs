//! Bounded, canonical byte transport for the experimental TransitContext image.
use super::*;
use std::sync::Arc;

const MAGIC: &[u8; 8] = b"KTCXCPV1";
const WIRE_SCHEMA: u32 = 1;
const SEGMENT_BYTES: usize = 64;

/// Errors from the experimental TransitContext v1 byte codec.
#[doc(hidden)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TransitContextWireError {
    UnsupportedVersion(u32),
    LimitExceeded,
    InvalidData,
    InvalidUtf8,
    Truncated,
    TrailingBytes,
    AllocationFailure,
    Native(TransitContextCheckpointError),
}

impl Display for TransitContextWireError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedVersion(version) => {
                write!(f, "unsupported transit wire version: {version}")
            }
            Self::LimitExceeded => f.write_str("transit wire limit exceeded"),
            Self::InvalidData => f.write_str("invalid transit wire data"),
            Self::InvalidUtf8 => f.write_str("invalid UTF-8 in transit wire data"),
            Self::Truncated => f.write_str("truncated transit wire data"),
            Self::TrailingBytes => f.write_str("trailing transit wire bytes"),
            Self::AllocationFailure => f.write_str("transit wire allocation failed"),
            Self::Native(error) => Display::fmt(error, f),
        }
    }
}

impl Error for TransitContextWireError {}

impl From<TransitContextCheckpointError> for TransitContextWireError {
    fn from(value: TransitContextCheckpointError) -> Self {
        match value {
            TransitContextCheckpointError::LimitExceeded => Self::LimitExceeded,
            other => Self::Native(other),
        }
    }
}

/// Register a byte codec bound to this source runtime and caller-approved graph.
/// The graph and source identity are captured by the caller-owned codec environment.
#[doc(hidden)]
pub fn register_transit_context_checkpoint_codec(
    codecs: &mut FlowCheckpointCodecs,
    codec_key: impl Into<String>,
    source_identity: FlowRuntimeIdentity,
    trusted_graph: Arc<TransitGraphV1>,
    limits: TransitContextCheckpointLimitsV1,
) -> Result<(), FlowCheckpointError> {
    codecs.register_context_with_owner::<TransitContext>(
        codec_key,
        1,
        move |context, remaining| {
            let mut bounded = limits;
            bounded.max_total_bytes = bounded.max_total_bytes.min(remaining);
            context
                .checkpoint_bytes_v1(&source_identity, bounded)
                .map_err(|error| FlowCheckpointCodecError(error.to_string()))
        },
        move |bytes, row_owner, view| {
            TransitContextCheckpointV1::restore_bytes_v1(
                bytes,
                &trusted_graph,
                view,
                row_owner,
                limits,
            )
            .map_err(|error| FlowCheckpointCodecError(error.to_string()))
        },
    )
}

impl TransitContext {
    /// Capture the complete context as canonical bounded v1 bytes.
    #[doc(hidden)]
    pub fn checkpoint_bytes_v1(
        &self,
        source_identity: &FlowRuntimeIdentity,
        limits: TransitContextCheckpointLimitsV1,
    ) -> Result<Vec<u8>, TransitContextWireError> {
        let image = self.checkpoint_v1(source_identity, relaxed_native_limits(limits)?)?;
        image.encode_bytes_v1(limits)
    }
}

impl TransitContextCheckpointV1 {
    /// Encode every owned field using canonical little-endian v1 bytes.
    #[doc(hidden)]
    pub fn encode_bytes_v1(
        &self,
        limits: TransitContextCheckpointLimitsV1,
    ) -> Result<Vec<u8>, TransitContextWireError> {
        validate_image_limits(self, limits)?;
        let length = encoded_len(self)?;
        if length > limits.max_total_bytes {
            return Err(TransitContextWireError::LimitExceeded);
        }
        let mut output = Vec::new();
        output
            .try_reserve_exact(length)
            .map_err(|_| TransitContextWireError::AllocationFailure)?;
        let mut writer = Writer { bytes: &mut output };
        writer.raw(MAGIC);
        writer.u32(WIRE_SCHEMA);
        writer.u32(self.schema_version);
        write_route(&mut writer, &self.route)?;
        writer.usize(self.segment_index)?;
        writer.u128(self.elapsed_in_segment_ticks);
        writer.entity(self.service_work);
        writer.entity(self.acquire_resource);
        writer.entity(self.acquire_owner);
        writer.option(self.acquire_work, |w, id| {
            w.entity(id);
            Ok(())
        })?;
        writer.u128(self.acquire_at_ticks);
        writer.i32(self.acquire_priority_level);
        writer.option(self.acquire_deadline_ticks, |w, ticks| {
            w.u128(ticks);
            Ok(())
        })?;
        writer.i32(self.acquire_scheduler_priority);
        writer.bool(self.acquire_timed);
        writer.bool(self.acquire_can_preempt);
        writer.option(self.acquire_preemptible, |w, strategy| {
            w.preemption(strategy);
            Ok(())
        })?;
        writer.option(self.carrier, |w, id| {
            w.entity(id);
            Ok(())
        })?;
        writer.option(self.kind, |w, kind| {
            w.u32(kind.code());
            Ok(())
        })?;
        writer.u128(self.start_at_ticks);
        writer.phase(self.phase);
        writer.option(self.paused_from, |w, phase| {
            w.phase(phase);
            Ok(())
        })?;
        writer.u128(self.last_advanced_at_ticks);
        writer.bool(self.initial_start_pending);
        writer.option(self.expected_event, |w, (index, generation)| {
            w.u64(index);
            w.u32(generation);
            Ok(())
        })?;
        writer.option(self.expected_due_ticks, |w, ticks| {
            w.u128(ticks);
            Ok(())
        })?;
        writer.option(self.command_ticket, |w, (batch, index, purpose)| {
            w.u64(batch);
            w.usize(index)?;
            w.purpose(purpose);
            Ok(())
        })?;
        writer.option(self.arrival_ticket, |w, (batch, index)| {
            w.u64(batch);
            w.usize(index)?;
            Ok(())
        })?;
        writer.option(self.next_progress_ticket, |w, (batch, index)| {
            w.u64(batch);
            w.usize(index)?;
            Ok(())
        })?;
        if output.len() != length {
            return Err(TransitContextWireError::InvalidData);
        }
        Ok(output)
    }

    /// Decode after an allocation-free complete preflight of the wire payload.
    #[doc(hidden)]
    pub fn decode_bytes_v1(
        bytes: &[u8],
        limits: TransitContextCheckpointLimitsV1,
    ) -> Result<Self, TransitContextWireError> {
        preflight(bytes, limits)?;
        let mut reader = Reader::new(bytes);
        reader.raw(MAGIC.len())?;
        let version = reader.u32()?;
        if version != WIRE_SCHEMA {
            return Err(TransitContextWireError::UnsupportedVersion(version));
        }
        let schema_version = reader.u32()?;
        let route = read_route(&mut reader)?;
        let segment_index = reader.usize()?;
        let elapsed_in_segment_ticks = reader.u128()?;
        let service_work = reader.entity()?;
        let acquire_resource = reader.entity()?;
        let acquire_owner = reader.entity()?;
        let acquire_work = reader.option(|r| r.entity())?;
        let acquire_at_ticks = reader.u128()?;
        let acquire_priority_level = reader.i32()?;
        let acquire_deadline_ticks = reader.option(|r| r.u128())?;
        let acquire_scheduler_priority = reader.i32()?;
        let acquire_timed = reader.bool()?;
        let acquire_can_preempt = reader.bool()?;
        let acquire_preemptible = reader.option(|r| r.preemption())?;
        let carrier = reader.option(|r| r.entity())?;
        let kind = reader.option(|r| r.u32().map(EventKind::custom))?;
        let start_at_ticks = reader.u128()?;
        let phase = reader.phase()?;
        let paused_from = reader.option(|r| r.phase())?;
        let last_advanced_at_ticks = reader.u128()?;
        let initial_start_pending = reader.bool()?;
        let expected_event = reader.option(|r| Ok((r.u64()?, r.u32()?)))?;
        let expected_due_ticks = reader.option(|r| r.u128())?;
        let command_ticket = reader.option(|r| Ok((r.u64()?, r.usize()?, r.purpose()?)))?;
        let arrival_ticket = reader.option(|r| Ok((r.u64()?, r.usize()?)))?;
        let next_progress_ticket = reader.option(|r| Ok((r.u64()?, r.usize()?)))?;
        if !reader.is_done() {
            return Err(TransitContextWireError::TrailingBytes);
        }
        Ok(Self {
            schema_version,
            route,
            segment_index,
            elapsed_in_segment_ticks,
            service_work,
            acquire_resource,
            acquire_owner,
            acquire_work,
            acquire_at_ticks,
            acquire_priority_level,
            acquire_deadline_ticks,
            acquire_scheduler_priority,
            acquire_timed,
            acquire_can_preempt,
            acquire_preemptible,
            carrier,
            kind,
            start_at_ticks,
            phase,
            paused_from,
            last_advanced_at_ticks,
            initial_start_pending,
            expected_event,
            expected_due_ticks,
            command_ticket,
            arrival_ticket,
            next_progress_ticket,
        })
    }

    /// Decode and restore against caller-approved graph and validated Flow view.
    #[doc(hidden)]
    pub fn restore_bytes_v1(
        bytes: &[u8],
        trusted_graph: &TransitGraphV1,
        rebind: &FlowCheckpointRebindV1,
        row_owner: EntityId,
        limits: TransitContextCheckpointLimitsV1,
    ) -> Result<TransitContext, TransitContextWireError> {
        Self::decode_bytes_v1(bytes, limits)?
            .restore_for_owner(
                trusted_graph,
                rebind,
                row_owner,
                relaxed_native_limits(limits)?,
            )
            .map_err(Into::into)
    }
}

fn validate_image_limits(
    image: &TransitContextCheckpointV1,
    limits: TransitContextCheckpointLimitsV1,
) -> Result<(), TransitContextWireError> {
    if image.schema_version != TRANSIT_CONTEXT_SCHEMA_V1 {
        return Err(TransitContextWireError::UnsupportedVersion(
            image.schema_version,
        ));
    }
    if image.route.schema_version != 1 {
        return Err(TransitContextWireError::UnsupportedVersion(
            image.route.schema_version,
        ));
    }
    let route_limits = RoutePlanImageLimitsV1::new(
        limits.max_segments,
        limits.max_graph_bytes,
        limits.max_mode_bytes,
        limits.max_total_bytes,
    );
    image
        .route
        .validate_limits(&route_limits)
        .map_err(|error| TransitContextWireError::Native(map_route_checkpoint_error(error)))?;
    Ok(())
}

fn relaxed_native_limits(
    limits: TransitContextCheckpointLimitsV1,
) -> Result<TransitContextCheckpointLimitsV1, TransitContextWireError> {
    Ok(TransitContextCheckpointLimitsV1 {
        max_total_bytes: limits
            .max_total_bytes
            .checked_add(TRANSIT_CONTEXT_FIXED_IMAGE_BYTES)
            .ok_or(TransitContextWireError::LimitExceeded)?,
        ..limits
    })
}

fn encoded_len(image: &TransitContextCheckpointV1) -> Result<usize, TransitContextWireError> {
    let mut length = 16usize; // magic, wire schema, image schema
    let mut add = |amount: usize| {
        length = length
            .checked_add(amount)
            .ok_or(TransitContextWireError::LimitExceeded)?;
        Ok::<(), TransitContextWireError>(())
    };
    add(28)?; // route schema, origin, destination, segment count
    add(image
        .route
        .segments
        .len()
        .checked_mul(SEGMENT_BYTES)
        .ok_or(TransitContextWireError::LimitExceeded)?)?;
    add(32)?; // route distance and duration
    add(8usize
        .checked_add(image.route.movement_mode.len())
        .ok_or(TransitContextWireError::LimitExceeded)?)?;
    add(28usize
        .checked_add(image.route.graph_canonical_bytes.len())
        .ok_or(TransitContextWireError::LimitExceeded)?)?;
    add(8 + 16 + 36)?; // cursor and three required entity IDs
    add(1 + image.acquire_work.map_or(0, |_| 12))?;
    add(16 + 4 + 1 + image.acquire_deadline_ticks.map_or(0, |_| 16) + 4 + 2)?;
    add(1 + image.acquire_preemptible.map_or(0, |_| 1))?;
    add(1 + image.carrier.map_or(0, |_| 12))?;
    add(1 + image.kind.map_or(0, |_| 4))?;
    add(16 + 1 + 1 + image.paused_from.map_or(0, |_| 1) + 16 + 1)?;
    add(1 + image.expected_event.map_or(0, |_| 12))?;
    add(1 + image.expected_due_ticks.map_or(0, |_| 16))?;
    add(1 + image.command_ticket.map_or(0, |(_, _, purpose)| {
        17 + match purpose {
            TransitCommandPurposeCheckpointV1::Start { .. }
            | TransitCommandPurposeCheckpointV1::Progress { .. } => 16,
            TransitCommandPurposeCheckpointV1::Arrival => 0,
        }
    }))?;
    add(1 + image.arrival_ticket.map_or(0, |_| 16))?;
    add(1 + image.next_progress_ticket.map_or(0, |_| 16))?;
    Ok(length)
}

struct Writer<'a> {
    bytes: &'a mut Vec<u8>,
}
impl Writer<'_> {
    fn raw(&mut self, v: &[u8]) {
        self.bytes.extend_from_slice(v);
    }
    fn u8(&mut self, v: u8) {
        self.bytes.push(v);
    }
    fn u32(&mut self, v: u32) {
        self.raw(&v.to_le_bytes());
    }
    fn i32(&mut self, v: i32) {
        self.raw(&v.to_le_bytes());
    }
    fn u64(&mut self, v: u64) {
        self.raw(&v.to_le_bytes());
    }
    fn u128(&mut self, v: u128) {
        self.raw(&v.to_le_bytes());
    }
    fn usize(&mut self, v: usize) -> Result<(), TransitContextWireError> {
        self.u64(u64::try_from(v).map_err(|_| TransitContextWireError::LimitExceeded)?);
        Ok(())
    }
    fn bool(&mut self, v: bool) {
        self.u8(u8::from(v));
    }
    fn entity(&mut self, v: EntityId) {
        self.u64(v.index);
        self.u32(v.generation);
    }
    fn option<T>(
        &mut self,
        value: Option<T>,
        write: impl FnOnce(&mut Self, T) -> Result<(), TransitContextWireError>,
    ) -> Result<(), TransitContextWireError> {
        match value {
            None => self.u8(0),
            Some(v) => {
                self.u8(1);
                write(self, v)?;
            }
        }
        Ok(())
    }
    fn phase(&mut self, value: TransitPhase) {
        self.u8(match value {
            TransitPhase::Ready => 0,
            TransitPhase::Moving => 1,
            TransitPhase::Paused => 2,
            TransitPhase::Arrived => 3,
        });
    }
    fn preemption(&mut self, value: PreemptionStrategy) {
        self.u8(match value {
            PreemptionStrategy::Suspend => 0,
            PreemptionStrategy::Abort => 1,
            PreemptionStrategy::Restart => 2,
        });
    }
    fn purpose(&mut self, value: TransitCommandPurposeCheckpointV1) {
        match value {
            TransitCommandPurposeCheckpointV1::Start { due_ticks } => {
                self.u8(0);
                self.u128(due_ticks);
            }
            TransitCommandPurposeCheckpointV1::Progress { due_ticks } => {
                self.u8(1);
                self.u128(due_ticks);
            }
            TransitCommandPurposeCheckpointV1::Arrival => self.u8(2),
        }
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, at: 0 }
    }
    fn raw(&mut self, len: usize) -> Result<&'a [u8], TransitContextWireError> {
        let end = self
            .at
            .checked_add(len)
            .ok_or(TransitContextWireError::LimitExceeded)?;
        let out = self
            .bytes
            .get(self.at..end)
            .ok_or(TransitContextWireError::Truncated)?;
        self.at = end;
        Ok(out)
    }
    fn u8(&mut self) -> Result<u8, TransitContextWireError> {
        Ok(self.raw(1)?[0])
    }
    fn u32(&mut self) -> Result<u32, TransitContextWireError> {
        Ok(u32::from_le_bytes(
            self.raw(4)?
                .try_into()
                .map_err(|_| TransitContextWireError::Truncated)?,
        ))
    }
    fn i32(&mut self) -> Result<i32, TransitContextWireError> {
        Ok(i32::from_le_bytes(
            self.raw(4)?
                .try_into()
                .map_err(|_| TransitContextWireError::Truncated)?,
        ))
    }
    fn u64(&mut self) -> Result<u64, TransitContextWireError> {
        Ok(u64::from_le_bytes(
            self.raw(8)?
                .try_into()
                .map_err(|_| TransitContextWireError::Truncated)?,
        ))
    }
    fn u128(&mut self) -> Result<u128, TransitContextWireError> {
        Ok(u128::from_le_bytes(
            self.raw(16)?
                .try_into()
                .map_err(|_| TransitContextWireError::Truncated)?,
        ))
    }
    fn usize(&mut self) -> Result<usize, TransitContextWireError> {
        usize::try_from(self.u64()?).map_err(|_| TransitContextWireError::LimitExceeded)
    }
    fn bool(&mut self) -> Result<bool, TransitContextWireError> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(TransitContextWireError::InvalidData),
        }
    }
    fn entity(&mut self) -> Result<EntityId, TransitContextWireError> {
        Ok(EntityId::new(self.u64()?, self.u32()?))
    }
    fn option<T>(
        &mut self,
        read: impl FnOnce(&mut Self) -> Result<T, TransitContextWireError>,
    ) -> Result<Option<T>, TransitContextWireError> {
        match self.u8()? {
            0 => Ok(None),
            1 => read(self).map(Some),
            _ => Err(TransitContextWireError::InvalidData),
        }
    }
    fn phase(&mut self) -> Result<TransitPhase, TransitContextWireError> {
        match self.u8()? {
            0 => Ok(TransitPhase::Ready),
            1 => Ok(TransitPhase::Moving),
            2 => Ok(TransitPhase::Paused),
            3 => Ok(TransitPhase::Arrived),
            _ => Err(TransitContextWireError::InvalidData),
        }
    }
    fn preemption(&mut self) -> Result<PreemptionStrategy, TransitContextWireError> {
        match self.u8()? {
            0 => Ok(PreemptionStrategy::Suspend),
            1 => Ok(PreemptionStrategy::Abort),
            2 => Ok(PreemptionStrategy::Restart),
            _ => Err(TransitContextWireError::InvalidData),
        }
    }
    fn purpose(&mut self) -> Result<TransitCommandPurposeCheckpointV1, TransitContextWireError> {
        match self.u8()? {
            0 => Ok(TransitCommandPurposeCheckpointV1::Start {
                due_ticks: self.u128()?,
            }),
            1 => Ok(TransitCommandPurposeCheckpointV1::Progress {
                due_ticks: self.u128()?,
            }),
            2 => Ok(TransitCommandPurposeCheckpointV1::Arrival),
            _ => Err(TransitContextWireError::InvalidData),
        }
    }
    fn is_done(&self) -> bool {
        self.at == self.bytes.len()
    }
}

fn write_route(
    w: &mut Writer<'_>,
    route: &RoutePlanImageV1,
) -> Result<(), TransitContextWireError> {
    w.u32(route.schema_version);
    w.u64(route.origin);
    w.u64(route.destination);
    w.usize(route.segments.len())?;
    for segment in &route.segments {
        w.u64(segment.edge_id);
        w.u64(segment.from);
        w.u64(segment.to);
        w.u64(segment.length_mm);
        w.u128(segment.start_offset_ticks);
        w.u128(segment.end_offset_ticks);
    }
    w.u128(route.distance_mm);
    w.u128(route.duration_ticks);
    w.usize(route.movement_mode.len())?;
    w.raw(route.movement_mode.as_bytes());
    w.u64(route.speed_mm_per_second);
    w.u64(route.ticks_per_second);
    w.u32(route.graph_version);
    w.usize(route.graph_canonical_bytes.len())?;
    w.raw(&route.graph_canonical_bytes);
    Ok(())
}

fn read_route(r: &mut Reader<'_>) -> Result<RoutePlanImageV1, TransitContextWireError> {
    let schema_version = r.u32()?;
    let origin = r.u64()?;
    let destination = r.u64()?;
    let count = r.usize()?;
    let mut segments = Vec::new();
    segments
        .try_reserve_exact(count)
        .map_err(|_| TransitContextWireError::AllocationFailure)?;
    for _ in 0..count {
        segments.push(RouteSegmentImageV1 {
            edge_id: r.u64()?,
            from: r.u64()?,
            to: r.u64()?,
            length_mm: r.u64()?,
            start_offset_ticks: r.u128()?,
            end_offset_ticks: r.u128()?,
        });
    }
    let distance_mm = r.u128()?;
    let duration_ticks = r.u128()?;
    let mode_len = r.usize()?;
    let mode =
        std::str::from_utf8(r.raw(mode_len)?).map_err(|_| TransitContextWireError::InvalidUtf8)?;
    let mut movement_mode = String::new();
    movement_mode
        .try_reserve_exact(mode_len)
        .map_err(|_| TransitContextWireError::AllocationFailure)?;
    movement_mode.push_str(mode);
    let speed_mm_per_second = r.u64()?;
    let ticks_per_second = r.u64()?;
    let graph_version = r.u32()?;
    let graph_len = r.usize()?;
    let graph_bytes = r.raw(graph_len)?;
    let mut graph_canonical_bytes = Vec::new();
    graph_canonical_bytes
        .try_reserve_exact(graph_len)
        .map_err(|_| TransitContextWireError::AllocationFailure)?;
    graph_canonical_bytes.extend_from_slice(graph_bytes);
    Ok(RoutePlanImageV1 {
        schema_version,
        origin,
        destination,
        segments,
        distance_mm,
        duration_ticks,
        movement_mode,
        speed_mm_per_second,
        ticks_per_second,
        graph_version,
        graph_canonical_bytes,
    })
}

fn preflight(
    bytes: &[u8],
    limits: TransitContextCheckpointLimitsV1,
) -> Result<(), TransitContextWireError> {
    if bytes.len() > limits.max_total_bytes {
        return Err(TransitContextWireError::LimitExceeded);
    }
    let mut r = Reader::new(bytes);
    if r.raw(MAGIC.len())? != MAGIC {
        return Err(TransitContextWireError::InvalidData);
    }
    let version = r.u32()?;
    if version != WIRE_SCHEMA {
        return Err(TransitContextWireError::UnsupportedVersion(version));
    }
    let image_version = r.u32()?;
    if image_version != TRANSIT_CONTEXT_SCHEMA_V1 {
        return Err(TransitContextWireError::UnsupportedVersion(image_version));
    }
    let route_version = r.u32()?;
    if route_version != 1 {
        return Err(TransitContextWireError::UnsupportedVersion(route_version));
    }
    let _origin = r.u64()?;
    let _destination = r.u64()?;
    let count = r.usize()?;
    if count > limits.max_segments {
        return Err(TransitContextWireError::LimitExceeded);
    }
    let segment_bytes = count
        .checked_mul(SEGMENT_BYTES)
        .ok_or(TransitContextWireError::LimitExceeded)?;
    r.raw(segment_bytes)?;
    let _distance = r.u128()?;
    let _duration = r.u128()?;
    let mode_len = r.usize()?;
    if mode_len > limits.max_mode_bytes {
        return Err(TransitContextWireError::LimitExceeded);
    }
    std::str::from_utf8(r.raw(mode_len)?).map_err(|_| TransitContextWireError::InvalidUtf8)?;
    let _speed = r.u64()?;
    let _ticks = r.u64()?;
    let _graph_version = r.u32()?;
    let graph_len = r.usize()?;
    if graph_len > limits.max_graph_bytes {
        return Err(TransitContextWireError::LimitExceeded);
    }
    r.raw(graph_len)?;
    let _cursor = r.usize()?;
    let _elapsed = r.u128()?;
    for _ in 0..3 {
        r.entity()?;
    }
    r.option(|r| r.entity())?;
    let _at = r.u128()?;
    let _priority = r.i32()?;
    r.option(|r| r.u128())?;
    let _scheduler = r.i32()?;
    r.bool()?;
    r.bool()?;
    r.option(|r| r.preemption())?;
    r.option(|r| r.entity())?;
    r.option(|r| r.u32())?;
    let _start = r.u128()?;
    r.phase()?;
    r.option(|r| r.phase())?;
    let _last = r.u128()?;
    r.bool()?;
    r.option(|r| Ok((r.u64()?, r.u32()?)))?;
    r.option(|r| r.u128())?;
    r.option(|r| {
        let _batch = r.u64()?;
        let _index = r.usize()?;
        r.purpose()
    })?;
    r.option(|r| {
        let _batch = r.u64()?;
        r.usize()
    })?;
    r.option(|r| {
        let _batch = r.u64()?;
        r.usize()
    })?;
    if !r.is_done() {
        return Err(TransitContextWireError::TrailingBytes);
    }
    Ok(())
}
