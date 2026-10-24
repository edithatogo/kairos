#![forbid(unsafe_code)]

use kairo_ecs_pdes::{
    LogicalEventId, LpId, OptimisticAuthority, OptimisticError, OptimisticEventOrderKey,
    OptimisticMessage, OptimisticMessageKind, RemoteEvent,
};
use kairo_ecs_types::SimTime;
use prost::Message;
use std::fmt;

/// Legacy big-endian wire envelope schema version (v1).
pub const ENVELOPE_SCHEMA_VERSION_LEGACY_V1: u32 = 1;

/// Standard Protobuf wire envelope schema version (v2).
pub const ENVELOPE_SCHEMA_VERSION_PROTOBUF_V2: u32 = 2;

/// Protocol version 1 (legacy big-endian framing).
pub const ENVELOPE_PROTOCOL_VERSION_V1: u16 = 1;

/// Protocol version 2 (typed Protobuf wire encoding).
pub const ENVELOPE_PROTOCOL_VERSION_V2: u16 = 2;

/// Maximum ancestry depth allowed in wire envelopes (matches MAX_CAUSAL_DEPTH = 128).
pub const MAX_ANCESTRY_DEPTH: usize = 128;

/// Maximum ancestry nodes (1 root + 128 output frames).
pub const MAX_ANCESTRY_NODES: usize = 129;

/// Maximum raw application event payload in bytes.
pub const MAX_PAYLOAD_BYTES: usize = 4_096;

/// Maximum total serialized envelope size in bytes.
pub const MAX_ENCODED_ENVELOPE_BYTES: usize = 65_536;

/// Maximum unresolved outbound sends allowed per channel.
pub const MAX_UNRESOLVED_SENDS_PER_CHANNEL: usize = 4_096;

/// Maximum retry attempts within the idempotency window.
pub const MAX_IDEMPOTENCY_WINDOW_ATTEMPTS: u32 = 8;

/// Wire-level representation of an ancestry frame in the causal tree.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AncestryFrame {
    Root {
        root_source_lp: u32,
        root_sequence: u64,
    },
    Output {
        parent_tick: u128,
        parent_source_lp: u32,
        ordinal: u32,
    },
}

/// Wire-level authority mode.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum OptimisticAuthorityMode {
    LocalPreview = 0,
    Scoped = 1,
}

impl OptimisticAuthorityMode {
    pub fn from_u32(val: u32) -> Option<Self> {
        match val {
            0 => Some(Self::LocalPreview),
            1 => Some(Self::Scoped),
            _ => None,
        }
    }
}

/// Wire-level message kind.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum WireMessageKind {
    Positive = 0,
    Anti = 1,
}

impl WireMessageKind {
    pub fn from_u32(val: u32) -> Option<Self> {
        match val {
            0 => Some(Self::Positive),
            1 => Some(Self::Anti),
            _ => None,
        }
    }

    pub fn to_pdes(self) -> OptimisticMessageKind {
        match self {
            Self::Positive => OptimisticMessageKind::Positive,
            Self::Anti => OptimisticMessageKind::Anti,
        }
    }

    pub fn from_pdes(kind: OptimisticMessageKind) -> Self {
        match kind {
            OptimisticMessageKind::Positive => Self::Positive,
            OptimisticMessageKind::Anti => Self::Anti,
        }
    }
}

/// Rejection codes R01 through R15 as defined in transport contract packet 2.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RejectionCode {
    /// R01: envelope_schema_version != 1
    UnsupportedEnvelopeVersion(u32),
    /// R02: protocol_id does not match enclosing transport constant
    ProtocolIdMismatch(String),
    /// R03: protocol_version != 1
    ProtocolVersionMismatch(u16),
    /// R04: Envelope, payload, or ancestry nodes exceeds limit
    EnvelopeTooLarge(&'static str, usize, usize),
    /// R05: Reserved field present and non-zero
    ReservedFieldPresent,
    /// R06: Non-minimal or duplicate field
    NonMinimalOrDuplicateField,
    /// R07: Authority mode mismatch
    AuthorityModeMismatch,
    /// R08: Stale authority epoch
    StaleAuthorityEpoch {
        source_lp: u32,
        expected: u64,
        actual: u64,
    },
    /// R09: Emission epoch mismatch
    EmissionEpochMismatch { expected: u64, actual: u64 },
    /// R10: Foreign simulation namespace
    ForeignSimulationNamespace { expected: u128, actual: u128 },
    /// R11: Unregistered or fenced source
    UnregisteredOrFencedSource(u32),
    /// R12: Source not owned by issuer
    SourceNotOwnedByIssuer(u32),
    /// R13: Ancestry depth exceeds 128 output frames
    AncestryDepthExceeded { depth: usize, limit: usize },
    /// R14: Innermost root source LP does not match frame source LP
    RootSourceMismatch { root_source: u32, frame_source: u32 },
    /// R15: Output parent tick is not strictly before child tick
    OutputTickNotStrictlyFuture { parent_tick: u128, child_tick: u128 },
    /// Malformed framing or unexpected EOF
    MalformedFraming(String),
    /// Underlying PDES error
    PdesError(String),
}

impl fmt::Display for RejectionCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedEnvelopeVersion(v) => {
                write!(f, "R01: Unsupported envelope schema version: {v}")
            }
            Self::ProtocolIdMismatch(id) => write!(f, "R02: Protocol ID mismatch: {id}"),
            Self::ProtocolVersionMismatch(v) => write!(f, "R03: Protocol version mismatch: {v}"),
            Self::EnvelopeTooLarge(field, actual, limit) => {
                write!(
                    f,
                    "R04: Envelope field '{field}' too large: actual {actual} > limit {limit}"
                )
            }
            Self::ReservedFieldPresent => write!(f, "R05: Reserved field is present and non-zero"),
            Self::NonMinimalOrDuplicateField => {
                write!(f, "R06: Non-minimal or duplicate field in envelope")
            }
            Self::AuthorityModeMismatch => write!(f, "R07: Authority mode mismatch"),
            Self::StaleAuthorityEpoch {
                source_lp,
                expected,
                actual,
            } => {
                write!(f, "R08: Stale authority epoch for LP {source_lp}: expected {expected}, actual {actual}")
            }
            Self::EmissionEpochMismatch { expected, actual } => {
                write!(
                    f,
                    "R09: Emission epoch mismatch: expected {expected}, actual {actual}"
                )
            }
            Self::ForeignSimulationNamespace { expected, actual } => {
                write!(
                    f,
                    "R10: Foreign simulation namespace: expected {expected}, actual {actual}"
                )
            }
            Self::UnregisteredOrFencedSource(lp) => {
                write!(f, "R11: Unregistered or fenced source LP: {lp}")
            }
            Self::SourceNotOwnedByIssuer(lp) => {
                write!(f, "R12: Source LP {lp} is not owned by issuer")
            }
            Self::AncestryDepthExceeded { depth, limit } => {
                write!(
                    f,
                    "R13: Ancestry depth exceeded: depth {depth} > limit {limit}"
                )
            }
            Self::RootSourceMismatch {
                root_source,
                frame_source,
            } => {
                write!(
                    f,
                    "R14: Root source mismatch: root {root_source} != frame {frame_source}"
                )
            }
            Self::OutputTickNotStrictlyFuture {
                parent_tick,
                child_tick,
            } => {
                write!(f, "R15: Output tick not strictly future: parent {parent_tick} >= child {child_tick}")
            }
            Self::MalformedFraming(msg) => write!(f, "Malformed envelope framing: {msg}"),
            Self::PdesError(msg) => write!(f, "PDES error: {msg}"),
        }
    }
}

impl std::error::Error for RejectionCode {}

impl From<OptimisticError> for RejectionCode {
    fn from(err: OptimisticError) -> Self {
        Self::PdesError(format!("{err:?}"))
    }
}

/// The additive language-neutral wire envelope for optimistic distributed simulation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KairoOptimisticEnvelopeV1 {
    pub envelope_schema_version: u32,
    pub protocol_id: String,
    pub protocol_version: u16,
    pub authority_mode: OptimisticAuthorityMode,
    pub simulation_namespace: u128,
    pub ownership_epoch: u64,
    pub emission_epoch: u64,
    pub source_lp: u32,
    pub dest_lp: u32,
    pub tick: u128,
    pub ancestry_frames: Vec<AncestryFrame>,
    pub incarnation: u64,
    pub kind: WireMessageKind,
    pub event_payload: Vec<u8>,
}

impl KairoOptimisticEnvelopeV1 {
    /// Validates all mandatory field boundaries and rejection oracles before construction or mutation.
    pub fn validate(&self, expected_protocol_id: &str) -> Result<(), RejectionCode> {
        if self.envelope_schema_version != ENVELOPE_SCHEMA_VERSION_LEGACY_V1
            && self.envelope_schema_version != ENVELOPE_SCHEMA_VERSION_PROTOBUF_V2
        {
            return Err(RejectionCode::UnsupportedEnvelopeVersion(
                self.envelope_schema_version,
            ));
        }
        if self.protocol_id != expected_protocol_id {
            return Err(RejectionCode::ProtocolIdMismatch(self.protocol_id.clone()));
        }
        if self.protocol_version != ENVELOPE_PROTOCOL_VERSION_V1
            && self.protocol_version != ENVELOPE_PROTOCOL_VERSION_V2
        {
            return Err(RejectionCode::ProtocolVersionMismatch(
                self.protocol_version,
            ));
        }
        if self.event_payload.len() > MAX_PAYLOAD_BYTES {
            return Err(RejectionCode::EnvelopeTooLarge(
                "event_payload",
                self.event_payload.len(),
                MAX_PAYLOAD_BYTES,
            ));
        }
        if self.ancestry_frames.is_empty() {
            return Err(RejectionCode::MalformedFraming(
                "Ancestry frames cannot be empty".to_string(),
            ));
        }
        if self.ancestry_frames.len() > MAX_ANCESTRY_NODES {
            return Err(RejectionCode::EnvelopeTooLarge(
                "ancestry_frames",
                self.ancestry_frames.len(),
                MAX_ANCESTRY_NODES,
            ));
        }

        // Count output frames and check root frame
        let mut output_count = 0;
        let mut root_source = None;
        let mut prev_tick = None;

        for frame in &self.ancestry_frames {
            match frame {
                AncestryFrame::Root { root_source_lp, .. } => {
                    if root_source.is_some() {
                        return Err(RejectionCode::MalformedFraming(
                            "Multiple root frames found in ancestry chain".to_string(),
                        ));
                    }
                    root_source = Some(*root_source_lp);
                }
                AncestryFrame::Output { parent_tick, .. } => {
                    output_count += 1;
                    if let Some(prev) = prev_tick {
                        if *parent_tick <= prev {
                            return Err(RejectionCode::OutputTickNotStrictlyFuture {
                                parent_tick: prev,
                                child_tick: *parent_tick,
                            });
                        }
                    }
                    prev_tick = Some(*parent_tick);
                }
            }
        }

        if output_count > MAX_ANCESTRY_DEPTH {
            return Err(RejectionCode::AncestryDepthExceeded {
                depth: output_count,
                limit: MAX_ANCESTRY_DEPTH,
            });
        }

        let Some(root_src) = root_source else {
            return Err(RejectionCode::MalformedFraming(
                "No root frame found in ancestry chain".to_string(),
            ));
        };

        if root_src != self.source_lp && output_count == 0 {
            return Err(RejectionCode::RootSourceMismatch {
                root_source: root_src,
                frame_source: self.source_lp,
            });
        }

        if let Some(last_parent_tick) = prev_tick {
            if self.tick <= last_parent_tick {
                return Err(RejectionCode::OutputTickNotStrictlyFuture {
                    parent_tick: last_parent_tick,
                    child_tick: self.tick,
                });
            }
        }

        Ok(())
    }

    /// Converts this wire envelope into a native `OptimisticMessage` using the validated codec bridge.
    pub fn to_optimistic_message(
        &self,
        expected_protocol_id: &str,
    ) -> Result<OptimisticMessage, RejectionCode> {
        self.validate(expected_protocol_id)?;

        // Find the root frame (must be present)
        let root_frame = self
            .ancestry_frames
            .iter()
            .find_map(|f| match f {
                AncestryFrame::Root {
                    root_source_lp,
                    root_sequence,
                } => Some((*root_source_lp, *root_sequence)),
                _ => None,
            })
            .ok_or_else(|| RejectionCode::MalformedFraming("No root frame found".to_string()))?;

        // Construct the root LogicalEventId
        let mut current_id = LogicalEventId::root(LpId(root_frame.0), root_frame.1);

        // Chain output frames outward
        for frame in &self.ancestry_frames {
            if let AncestryFrame::Output {
                parent_tick,
                parent_source_lp,
                ordinal,
            } = frame
            {
                let parent_key = OptimisticEventOrderKey::try_from_parts(
                    SimTime::from_ticks(*parent_tick),
                    LpId(*parent_source_lp),
                    current_id,
                )?;
                current_id = LogicalEventId::child(&parent_key, *ordinal)?;
            }
        }

        let authority = match self.authority_mode {
            OptimisticAuthorityMode::LocalPreview => OptimisticAuthority::LocalPreview,
            OptimisticAuthorityMode::Scoped => OptimisticAuthority::Scoped {
                simulation_namespace: self.simulation_namespace,
                ownership_epoch: self.ownership_epoch,
            },
        };

        let remote_event = RemoteEvent {
            source_lp: LpId(self.source_lp),
            dest_lp: LpId(self.dest_lp),
            tick: SimTime::from_ticks(self.tick),
            event_payload: self.event_payload.clone(),
        };

        let message = OptimisticMessage::try_from_authority_parts(
            remote_event,
            current_id,
            authority,
            self.incarnation,
            self.kind.to_pdes(),
        )?;

        Ok(message)
    }

    /// Creates an envelope from a native `OptimisticMessage`.
    pub fn from_optimistic_message(
        message: &OptimisticMessage,
        protocol_id: &str,
        dest_lp: LpId,
        emission_epoch: u64,
    ) -> Result<Self, RejectionCode> {
        let event = message.event();
        let payload = event.event_payload.clone();
        if payload.len() > MAX_PAYLOAD_BYTES {
            return Err(RejectionCode::EnvelopeTooLarge(
                "event_payload",
                payload.len(),
                MAX_PAYLOAD_BYTES,
            ));
        }

        let (authority_mode, simulation_namespace, ownership_epoch) = match message.authority() {
            OptimisticAuthority::LocalPreview => {
                (OptimisticAuthorityMode::LocalPreview, 0u128, 0u64)
            }
            OptimisticAuthority::Scoped {
                simulation_namespace,
                ownership_epoch,
            } => (
                OptimisticAuthorityMode::Scoped,
                simulation_namespace,
                ownership_epoch,
            ),
        };

        // Extract ancestry frames from logical_id walking up to root
        let mut frames = Vec::new();
        let mut curr_id = message.logical_id().clone();

        loop {
            if let Some((parent_key, ordinal)) = curr_id.output_parts() {
                frames.push(AncestryFrame::Output {
                    parent_tick: parent_key.tick().ticks(),
                    parent_source_lp: parent_key.source_lp().0,
                    ordinal,
                });
                curr_id = parent_key.logical_id().clone();
            } else if let Some((root_src, root_seq)) = curr_id.root_parts() {
                frames.push(AncestryFrame::Root {
                    root_source_lp: root_src.0,
                    root_sequence: root_seq,
                });
                break;
            } else {
                return Err(RejectionCode::MalformedFraming(
                    "Invalid node shape".to_string(),
                ));
            }
        }

        // Frames were collected leaf-to-root; reverse to outermost ancestor first
        frames.reverse();

        let env = Self {
            envelope_schema_version: ENVELOPE_SCHEMA_VERSION_PROTOBUF_V2,
            protocol_id: protocol_id.to_string(),
            protocol_version: ENVELOPE_PROTOCOL_VERSION_V2,
            authority_mode,
            simulation_namespace,
            ownership_epoch,
            emission_epoch,
            source_lp: event.source_lp.0,
            dest_lp: dest_lp.0,
            tick: event.tick.ticks(),
            ancestry_frames: frames,
            incarnation: message.incarnation(),
            kind: WireMessageKind::from_pdes(message.kind()),
            event_payload: payload,
        };

        env.validate(protocol_id)?;
        Ok(env)
    }

    /// Serializes the envelope into deterministic protobuf binary bytes using `prost`.
    pub fn encode(&self) -> Result<Vec<u8>, RejectionCode> {
        let wire = self.to_wire()?;
        let encoded_len = wire.encoded_len();
        if encoded_len > MAX_ENCODED_ENVELOPE_BYTES {
            return Err(RejectionCode::EnvelopeTooLarge(
                "total_envelope",
                encoded_len,
                MAX_ENCODED_ENVELOPE_BYTES,
            ));
        }

        let mut buf = Vec::with_capacity(encoded_len);
        wire.encode(&mut buf)
            .map_err(|e| RejectionCode::MalformedFraming(e.to_string()))?;

        if buf.len() > MAX_ENCODED_ENVELOPE_BYTES {
            return Err(RejectionCode::EnvelopeTooLarge(
                "total_envelope",
                buf.len(),
                MAX_ENCODED_ENVELOPE_BYTES,
            ));
        }

        Ok(buf)
    }

    /// Deserializes an envelope with backward-compatible dual decoding:
    /// - If bytes begin with `[0, 0, 0, 1]`, decodes legacy v1 big-endian wire framing.
    /// - Otherwise, decodes typed Protobuf v2 wire framing (`WireOptimisticEnvelope`).
    pub fn decode(bytes: &[u8]) -> Result<Self, RejectionCode> {
        if bytes.len() > MAX_ENCODED_ENVELOPE_BYTES {
            return Err(RejectionCode::EnvelopeTooLarge(
                "total_envelope",
                bytes.len(),
                MAX_ENCODED_ENVELOPE_BYTES,
            ));
        }
        if bytes.is_empty() {
            return Err(RejectionCode::MalformedFraming(
                "Buffer is empty".to_string(),
            ));
        }

        // Backward-compatibility path: legacy v1 wire begins with [0, 0, 0, 1] (big-endian u32 = 1)
        if bytes.len() >= 4 && bytes[0..4] == [0, 0, 0, 1] {
            return Self::decode_legacy_v1(bytes);
        }

        let wire = WireOptimisticEnvelope::decode(bytes)
            .map_err(|e| RejectionCode::MalformedFraming(format!("Protobuf decode error: {e}")))?;

        Self::from_wire(wire)
    }

    /// Deserializes a legacy big-endian wire envelope (schema version 1) for backward compatibility.
    pub fn decode_legacy_v1(bytes: &[u8]) -> Result<Self, RejectionCode> {
        if bytes.len() > MAX_ENCODED_ENVELOPE_BYTES {
            return Err(RejectionCode::EnvelopeTooLarge(
                "total_envelope",
                bytes.len(),
                MAX_ENCODED_ENVELOPE_BYTES,
            ));
        }
        if bytes.len() < 4 {
            return Err(RejectionCode::MalformedFraming(
                "Buffer too short for schema version".to_string(),
            ));
        }

        let mut offset = 0;
        let schema_ver = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap());
        offset += 4;
        if schema_ver != ENVELOPE_SCHEMA_VERSION_LEGACY_V1 {
            return Err(RejectionCode::UnsupportedEnvelopeVersion(schema_ver));
        }

        if bytes.len() < offset + 2 {
            return Err(RejectionCode::MalformedFraming(
                "Buffer too short for protocol ID length".to_string(),
            ));
        }
        let proto_len = u16::from_be_bytes(bytes[offset..offset + 2].try_into().unwrap()) as usize;
        offset += 2;

        if bytes.len() < offset + proto_len {
            return Err(RejectionCode::MalformedFraming(
                "Buffer too short for protocol ID".to_string(),
            ));
        }
        let protocol_id =
            String::from_utf8(bytes[offset..offset + proto_len].to_vec()).map_err(|e| {
                RejectionCode::MalformedFraming(format!("Invalid protocol ID string: {e}"))
            })?;
        offset += proto_len;

        const FIXED_LEN: usize = 2 + 4 + 16 + 8 + 8 + 4 + 4 + 16 + 8 + 4;
        if bytes.len() < offset + FIXED_LEN {
            return Err(RejectionCode::MalformedFraming(
                "Buffer too short for fixed fields".to_string(),
            ));
        }

        let protocol_version = u16::from_be_bytes(bytes[offset..offset + 2].try_into().unwrap());
        offset += 2;
        let mode_raw = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap());
        offset += 4;
        let authority_mode = OptimisticAuthorityMode::from_u32(mode_raw)
            .ok_or(RejectionCode::AuthorityModeMismatch)?;

        let simulation_namespace =
            u128::from_be_bytes(bytes[offset..offset + 16].try_into().unwrap());
        offset += 16;
        let ownership_epoch = u64::from_be_bytes(bytes[offset..offset + 8].try_into().unwrap());
        offset += 8;
        let emission_epoch = u64::from_be_bytes(bytes[offset..offset + 8].try_into().unwrap());
        offset += 8;
        let source_lp = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap());
        offset += 4;
        let dest_lp = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap());
        offset += 4;
        let tick = u128::from_be_bytes(bytes[offset..offset + 16].try_into().unwrap());
        offset += 16;
        let incarnation = u64::from_be_bytes(bytes[offset..offset + 8].try_into().unwrap());
        offset += 8;
        let kind_raw = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap());
        offset += 4;
        let kind = WireMessageKind::from_u32(kind_raw).ok_or_else(|| {
            RejectionCode::MalformedFraming(format!("Unknown message kind: {kind_raw}"))
        })?;

        if bytes.len() < offset + 4 {
            return Err(RejectionCode::MalformedFraming(
                "Buffer too short for frame count".to_string(),
            ));
        }
        let frame_count =
            u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
        offset += 4;

        if frame_count > MAX_ANCESTRY_NODES {
            return Err(RejectionCode::EnvelopeTooLarge(
                "ancestry_frames",
                frame_count,
                MAX_ANCESTRY_NODES,
            ));
        }

        let mut ancestry_frames = Vec::with_capacity(frame_count);
        for _ in 0..frame_count {
            if bytes.len() < offset + 1 {
                return Err(RejectionCode::MalformedFraming(
                    "Buffer too short for frame tag".to_string(),
                ));
            }
            let tag = bytes[offset];
            offset += 1;
            match tag {
                0 => {
                    if bytes.len() < offset + 12 {
                        return Err(RejectionCode::MalformedFraming(
                            "Buffer too short for root frame".to_string(),
                        ));
                    }
                    let root_source_lp =
                        u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap());
                    offset += 4;
                    let root_sequence =
                        u64::from_be_bytes(bytes[offset..offset + 8].try_into().unwrap());
                    offset += 8;
                    ancestry_frames.push(AncestryFrame::Root {
                        root_source_lp,
                        root_sequence,
                    });
                }
                1 => {
                    if bytes.len() < offset + 24 {
                        return Err(RejectionCode::MalformedFraming(
                            "Buffer too short for output frame".to_string(),
                        ));
                    }
                    let parent_tick =
                        u128::from_be_bytes(bytes[offset..offset + 16].try_into().unwrap());
                    offset += 16;
                    let parent_source_lp =
                        u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap());
                    offset += 4;
                    let ordinal = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap());
                    offset += 4;
                    ancestry_frames.push(AncestryFrame::Output {
                        parent_tick,
                        parent_source_lp,
                        ordinal,
                    });
                }
                other => {
                    return Err(RejectionCode::MalformedFraming(format!(
                        "Unknown ancestry frame tag: {other}"
                    )));
                }
            }
        }

        if bytes.len() < offset + 4 {
            return Err(RejectionCode::MalformedFraming(
                "Buffer too short for payload length".to_string(),
            ));
        }
        let payload_len =
            u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
        offset += 4;

        if payload_len > MAX_PAYLOAD_BYTES {
            return Err(RejectionCode::EnvelopeTooLarge(
                "event_payload",
                payload_len,
                MAX_PAYLOAD_BYTES,
            ));
        }

        if bytes.len() < offset + payload_len {
            return Err(RejectionCode::MalformedFraming(
                "Buffer too short for payload bytes".to_string(),
            ));
        }
        let event_payload = bytes[offset..offset + payload_len].to_vec();

        Ok(Self {
            envelope_schema_version: schema_ver,
            protocol_id,
            protocol_version,
            authority_mode,
            simulation_namespace,
            ownership_epoch,
            emission_epoch,
            source_lp,
            dest_lp,
            tick,
            ancestry_frames,
            incarnation,
            kind,
            event_payload,
        })
    }

    /// Serializes to the legacy big-endian wire envelope (v1) for backward compatibility testing or transitional peers.
    pub fn encode_legacy_v1(&self) -> Result<Vec<u8>, RejectionCode> {
        let mut buf = Vec::with_capacity(256 + self.event_payload.len());
        buf.extend_from_slice(&ENVELOPE_SCHEMA_VERSION_LEGACY_V1.to_be_bytes());
        let proto_bytes = self.protocol_id.as_bytes();
        buf.extend_from_slice(&(proto_bytes.len() as u16).to_be_bytes());
        buf.extend_from_slice(proto_bytes);
        buf.extend_from_slice(&ENVELOPE_PROTOCOL_VERSION_V1.to_be_bytes());
        buf.extend_from_slice(&(self.authority_mode as u32).to_be_bytes());
        buf.extend_from_slice(&self.simulation_namespace.to_be_bytes());
        buf.extend_from_slice(&self.ownership_epoch.to_be_bytes());
        buf.extend_from_slice(&self.emission_epoch.to_be_bytes());
        buf.extend_from_slice(&self.source_lp.to_be_bytes());
        buf.extend_from_slice(&self.dest_lp.to_be_bytes());
        buf.extend_from_slice(&self.tick.to_be_bytes());
        buf.extend_from_slice(&self.incarnation.to_be_bytes());
        buf.extend_from_slice(&(self.kind as u32).to_be_bytes());

        buf.extend_from_slice(&(self.ancestry_frames.len() as u32).to_be_bytes());
        for frame in &self.ancestry_frames {
            match frame {
                AncestryFrame::Root {
                    root_source_lp,
                    root_sequence,
                } => {
                    buf.push(0u8);
                    buf.extend_from_slice(&root_source_lp.to_be_bytes());
                    buf.extend_from_slice(&root_sequence.to_be_bytes());
                }
                AncestryFrame::Output {
                    parent_tick,
                    parent_source_lp,
                    ordinal,
                } => {
                    buf.push(1u8);
                    buf.extend_from_slice(&parent_tick.to_be_bytes());
                    buf.extend_from_slice(&parent_source_lp.to_be_bytes());
                    buf.extend_from_slice(&ordinal.to_be_bytes());
                }
            }
        }

        buf.extend_from_slice(&(self.event_payload.len() as u32).to_be_bytes());
        buf.extend_from_slice(&self.event_payload);

        if buf.len() > MAX_ENCODED_ENVELOPE_BYTES {
            return Err(RejectionCode::EnvelopeTooLarge(
                "total_envelope",
                buf.len(),
                MAX_ENCODED_ENVELOPE_BYTES,
            ));
        }

        Ok(buf)
    }

    /// Converts this domain envelope to its typed Protobuf wire model (version 2).
    pub fn to_wire(&self) -> Result<WireOptimisticEnvelope, RejectionCode> {
        if self.ancestry_frames.len() > MAX_ANCESTRY_NODES {
            return Err(RejectionCode::EnvelopeTooLarge(
                "ancestry_frames",
                self.ancestry_frames.len(),
                MAX_ANCESTRY_NODES,
            ));
        }
        if self.event_payload.len() > MAX_PAYLOAD_BYTES {
            return Err(RejectionCode::EnvelopeTooLarge(
                "event_payload",
                self.event_payload.len(),
                MAX_PAYLOAD_BYTES,
            ));
        }

        let mut wire_frames = Vec::with_capacity(self.ancestry_frames.len());
        for frame in &self.ancestry_frames {
            let wire_frame = match frame {
                AncestryFrame::Root {
                    root_source_lp,
                    root_sequence,
                } => WireAncestryFrame {
                    frame: Some(WireFrameKind::Root(WireRootFrame {
                        root_source_lp: *root_source_lp,
                        root_sequence: *root_sequence,
                    })),
                },
                AncestryFrame::Output {
                    parent_tick,
                    parent_source_lp,
                    ordinal,
                } => WireAncestryFrame {
                    frame: Some(WireFrameKind::Output(WireOutputFrame {
                        parent_tick: parent_tick.to_be_bytes().to_vec(),
                        parent_source_lp: *parent_source_lp,
                        ordinal: *ordinal,
                    })),
                },
            };
            wire_frames.push(wire_frame);
        }

        Ok(WireOptimisticEnvelope {
            envelope_schema_version: ENVELOPE_SCHEMA_VERSION_PROTOBUF_V2,
            protocol_id: self.protocol_id.clone(),
            protocol_version: ENVELOPE_PROTOCOL_VERSION_V2 as u32,
            authority_mode: self.authority_mode as u32,
            simulation_namespace: self.simulation_namespace.to_be_bytes().to_vec(),
            ownership_epoch: self.ownership_epoch,
            emission_epoch: self.emission_epoch,
            source_lp: self.source_lp,
            dest_lp: self.dest_lp,
            tick: self.tick.to_be_bytes().to_vec(),
            ancestry_frames: wire_frames,
            incarnation: self.incarnation,
            kind: self.kind as u32,
            event_payload: self.event_payload.clone(),
        })
    }

    /// Reconstructs the domain envelope from its typed Protobuf wire model (version 2).
    pub fn from_wire(wire: WireOptimisticEnvelope) -> Result<Self, RejectionCode> {
        if wire.envelope_schema_version != ENVELOPE_SCHEMA_VERSION_PROTOBUF_V2 {
            return Err(RejectionCode::UnsupportedEnvelopeVersion(
                wire.envelope_schema_version,
            ));
        }
        if wire.protocol_version != ENVELOPE_PROTOCOL_VERSION_V2 as u32 {
            return Err(RejectionCode::ProtocolVersionMismatch(
                wire.protocol_version as u16,
            ));
        }
        let protocol_version = wire.protocol_version as u16;

        let authority_mode = OptimisticAuthorityMode::from_u32(wire.authority_mode)
            .ok_or(RejectionCode::AuthorityModeMismatch)?;

        if wire.simulation_namespace.len() != 16 {
            return Err(RejectionCode::MalformedFraming(
                "simulation_namespace must be exactly 16 bytes for 128-bit integer".to_string(),
            ));
        }
        let simulation_namespace =
            u128::from_be_bytes(wire.simulation_namespace[..16].try_into().unwrap());

        if wire.tick.len() != 16 {
            return Err(RejectionCode::MalformedFraming(
                "tick must be exactly 16 bytes for 128-bit integer".to_string(),
            ));
        }
        let tick = u128::from_be_bytes(wire.tick[..16].try_into().unwrap());

        if wire.ancestry_frames.len() > MAX_ANCESTRY_NODES {
            return Err(RejectionCode::EnvelopeTooLarge(
                "ancestry_frames",
                wire.ancestry_frames.len(),
                MAX_ANCESTRY_NODES,
            ));
        }

        let mut ancestry_frames = Vec::with_capacity(wire.ancestry_frames.len());
        for wf in wire.ancestry_frames {
            match wf.frame {
                Some(WireFrameKind::Root(r)) => {
                    ancestry_frames.push(AncestryFrame::Root {
                        root_source_lp: r.root_source_lp,
                        root_sequence: r.root_sequence,
                    });
                }
                Some(WireFrameKind::Output(o)) => {
                    if o.parent_tick.len() != 16 {
                        return Err(RejectionCode::MalformedFraming(
                            "parent_tick must be exactly 16 bytes for 128-bit integer".to_string(),
                        ));
                    }
                    let parent_tick = u128::from_be_bytes(o.parent_tick[..16].try_into().unwrap());
                    ancestry_frames.push(AncestryFrame::Output {
                        parent_tick,
                        parent_source_lp: o.parent_source_lp,
                        ordinal: o.ordinal,
                    });
                }
                None => {
                    return Err(RejectionCode::MalformedFraming(
                        "Missing oneof frame in AncestryFrame".to_string(),
                    ));
                }
            }
        }

        let kind = WireMessageKind::from_u32(wire.kind).ok_or_else(|| {
            RejectionCode::MalformedFraming(format!("Unknown message kind: {}", wire.kind))
        })?;

        if wire.event_payload.len() > MAX_PAYLOAD_BYTES {
            return Err(RejectionCode::EnvelopeTooLarge(
                "event_payload",
                wire.event_payload.len(),
                MAX_PAYLOAD_BYTES,
            ));
        }

        Ok(Self {
            envelope_schema_version: wire.envelope_schema_version,
            protocol_id: wire.protocol_id,
            protocol_version,
            authority_mode,
            simulation_namespace,
            ownership_epoch: wire.ownership_epoch,
            emission_epoch: wire.emission_epoch,
            source_lp: wire.source_lp,
            dest_lp: wire.dest_lp,
            tick,
            ancestry_frames,
            incarnation: wire.incarnation,
            kind,
            event_payload: wire.event_payload,
        })
    }
}

/// Typed Protobuf wire frame for Root causal node.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WireRootFrame {
    #[prost(uint32, tag = "1")]
    pub root_source_lp: u32,
    #[prost(uint64, tag = "2")]
    pub root_sequence: u64,
}

/// Typed Protobuf wire frame for Output causal node.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WireOutputFrame {
    #[prost(bytes = "vec", tag = "1")]
    pub parent_tick: Vec<u8>,
    #[prost(uint32, tag = "2")]
    pub parent_source_lp: u32,
    #[prost(uint32, tag = "3")]
    pub ordinal: u32,
}

/// Oneof payload for an ancestry frame.
#[derive(Clone, PartialEq, ::prost::Oneof)]
pub enum WireFrameKind {
    #[prost(message, tag = "1")]
    Root(WireRootFrame),
    #[prost(message, tag = "2")]
    Output(WireOutputFrame),
}

/// Typed Protobuf wire frame wrapping either a root or an output frame.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WireAncestryFrame {
    #[prost(oneof = "WireFrameKind", tags = "1, 2")]
    pub frame: Option<WireFrameKind>,
}

/// Typed Protobuf wire envelope matching `kairo.ecs.simulation.v1.OptimisticEnvelope`.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WireOptimisticEnvelope {
    #[prost(uint32, tag = "1")]
    pub envelope_schema_version: u32,
    #[prost(string, tag = "2")]
    pub protocol_id: String,
    #[prost(uint32, tag = "3")]
    pub protocol_version: u32,
    #[prost(uint32, tag = "4")]
    pub authority_mode: u32,
    #[prost(bytes = "vec", tag = "5")]
    pub simulation_namespace: Vec<u8>,
    #[prost(uint64, tag = "6")]
    pub ownership_epoch: u64,
    #[prost(uint64, tag = "7")]
    pub emission_epoch: u64,
    #[prost(uint32, tag = "8")]
    pub source_lp: u32,
    #[prost(uint32, tag = "9")]
    pub dest_lp: u32,
    #[prost(bytes = "vec", tag = "10")]
    pub tick: Vec<u8>,
    #[prost(message, repeated, tag = "11")]
    pub ancestry_frames: Vec<WireAncestryFrame>,
    #[prost(uint64, tag = "12")]
    pub incarnation: u64,
    #[prost(uint32, tag = "13")]
    pub kind: u32,
    #[prost(bytes = "vec", tag = "14")]
    pub event_payload: Vec<u8>,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WireRemoteEvent {
    #[prost(uint32, tag = "1")]
    pub source_lp: u32,
    #[prost(uint32, tag = "2")]
    pub dest_lp: u32,
    #[prost(uint64, tag = "3")]
    pub tick: u64,
    #[prost(bytes = "vec", tag = "4")]
    pub event_payload: Vec<u8>,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WireComponentBlob {
    #[prost(string, tag = "1")]
    pub component_type_id: String,
    #[prost(uint64, tag = "2")]
    pub generation: u64,
    #[prost(bytes = "vec", tag = "3")]
    pub payload: Vec<u8>,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WirePendingEvent {
    #[prost(uint32, tag = "1")]
    pub source_lp: u32,
    #[prost(uint32, tag = "2")]
    pub dest_lp: u32,
    #[prost(uint64, tag = "3")]
    pub tick: u64,
    #[prost(uint64, tag = "4")]
    pub sequence: u64,
    #[prost(uint64, tag = "5")]
    pub payload_bytes: u64,
    #[prost(bool, tag = "6")]
    pub anti_message: bool,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WireMigrationRequest {
    #[prost(uint64, tag = "1")]
    pub entity_index: u64,
    #[prost(uint32, tag = "2")]
    pub entity_generation: u32,
    #[prost(uint32, tag = "3")]
    pub source_lp: u32,
    #[prost(uint32, tag = "4")]
    pub dest_lp: u32,
    #[prost(string, tag = "5")]
    pub migration_id: String,
    #[prost(message, repeated, tag = "6")]
    pub components: Vec<WireComponentBlob>,
    #[prost(message, repeated, tag = "7")]
    pub pending_events: Vec<WirePendingEvent>,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WireMigrationAck {
    #[prost(string, tag = "1")]
    pub migration_id: String,
    #[prost(bool, tag = "2")]
    pub applied: bool,
    #[prost(string, tag = "3")]
    pub message: String,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WireTelemetryBatch {
    #[prost(uint32, tag = "1")]
    pub source_lp: u32,
    #[prost(bytes = "vec", tag = "2")]
    pub arrow_ipc_payload: Vec<u8>,
    #[prost(uint64, tag = "3")]
    pub tick_start: u64,
    #[prost(uint64, tag = "4")]
    pub tick_end: u64,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WireGvtProposal {
    #[prost(uint32, tag = "1")]
    pub source_lp: u32,
    #[prost(uint64, tag = "2")]
    pub local_min_tick: u64,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WireGvtDecision {
    #[prost(uint64, tag = "1")]
    pub gvt_tick: u64,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WireHeartbeat {
    #[prost(uint32, tag = "1")]
    pub source_lp: u32,
    #[prost(uint64, tag = "2")]
    pub observed_tick: u64,
    #[prost(uint64, tag = "3")]
    pub elapsed_since_last_seen_ms: u64,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WireHeartbeatAck {
    #[prost(uint32, tag = "1")]
    pub source_lp: u32,
    #[prost(bool, tag = "2")]
    pub accepted: bool,
    #[prost(string, tag = "3")]
    pub message: String,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WireOptimisticAck {
    #[prost(uint32, tag = "1")]
    pub source_lp: u32,
    #[prost(uint32, tag = "2")]
    pub dest_lp: u32,
    #[prost(bool, tag = "3")]
    pub accepted: bool,
    #[prost(string, tag = "4")]
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GRPC_PROTOCOL_ID;

    #[test]
    fn test_envelope_roundtrip_root() {
        let envelope = KairoOptimisticEnvelopeV1 {
            envelope_schema_version: ENVELOPE_SCHEMA_VERSION_PROTOBUF_V2,
            protocol_id: GRPC_PROTOCOL_ID.to_string(),
            protocol_version: ENVELOPE_PROTOCOL_VERSION_V2,
            authority_mode: OptimisticAuthorityMode::Scoped,
            simulation_namespace: 0x1234_5678_9abc_def0,
            ownership_epoch: 42,
            emission_epoch: 42,
            source_lp: 7,
            dest_lp: 8,
            tick: 100,
            ancestry_frames: vec![AncestryFrame::Root {
                root_source_lp: 7,
                root_sequence: 1,
            }],
            incarnation: 0,
            kind: WireMessageKind::Positive,
            event_payload: b"hello-pdes".to_vec(),
        };

        envelope
            .validate(GRPC_PROTOCOL_ID)
            .expect("should validate");
        let encoded = envelope.encode().expect("should encode");
        let decoded = KairoOptimisticEnvelopeV1::decode(&encoded).expect("should decode");
        assert_eq!(envelope, decoded);
    }

    #[test]
    fn test_envelope_roundtrip_with_ancestry_chain() {
        let envelope = KairoOptimisticEnvelopeV1 {
            envelope_schema_version: ENVELOPE_SCHEMA_VERSION_PROTOBUF_V2,
            protocol_id: GRPC_PROTOCOL_ID.to_string(),
            protocol_version: ENVELOPE_PROTOCOL_VERSION_V2,
            authority_mode: OptimisticAuthorityMode::Scoped,
            simulation_namespace: 0x9999,
            ownership_epoch: 10,
            emission_epoch: 10,
            source_lp: 7,
            dest_lp: 8,
            tick: 150,
            ancestry_frames: vec![
                AncestryFrame::Root {
                    root_source_lp: 7,
                    root_sequence: 1,
                },
                AncestryFrame::Output {
                    parent_tick: 50,
                    parent_source_lp: 7,
                    ordinal: 0,
                },
                AncestryFrame::Output {
                    parent_tick: 100,
                    parent_source_lp: 7,
                    ordinal: 1,
                },
            ],
            incarnation: 2,
            kind: WireMessageKind::Anti,
            event_payload: b"anti-payload".to_vec(),
        };

        envelope
            .validate(GRPC_PROTOCOL_ID)
            .expect("should validate");
        let encoded = envelope.encode().expect("should encode");
        let decoded = KairoOptimisticEnvelopeV1::decode(&encoded).expect("should decode");
        assert_eq!(envelope, decoded);
    }

    #[test]
    fn test_legacy_v1_envelope_roundtrip_backward_compatibility() {
        let legacy_env = KairoOptimisticEnvelopeV1 {
            envelope_schema_version: ENVELOPE_SCHEMA_VERSION_LEGACY_V1,
            protocol_id: GRPC_PROTOCOL_ID.to_string(),
            protocol_version: ENVELOPE_PROTOCOL_VERSION_V1,
            authority_mode: OptimisticAuthorityMode::Scoped,
            simulation_namespace: 0x1111_2222_3333_4444,
            ownership_epoch: 9,
            emission_epoch: 9,
            source_lp: 2,
            dest_lp: 4,
            tick: 88,
            ancestry_frames: vec![AncestryFrame::Root {
                root_source_lp: 2,
                root_sequence: 15,
            }],
            incarnation: 1,
            kind: WireMessageKind::Positive,
            event_payload: b"legacy-v1-bytes".to_vec(),
        };

        legacy_env
            .validate(GRPC_PROTOCOL_ID)
            .expect("legacy v1 envelope must validate");
        let legacy_bytes = legacy_env.encode_legacy_v1().expect("legacy v1 encode ok");
        // Verify legacy bytes start with [0, 0, 0, 1]
        assert_eq!(&legacy_bytes[0..4], &[0, 0, 0, 1]);

        let decoded = KairoOptimisticEnvelopeV1::decode(&legacy_bytes)
            .expect("decode must transparently handle legacy v1");
        assert_eq!(legacy_env, decoded);
    }

    #[test]
    fn test_conversion_to_and_from_optimistic_message() {
        // Construct root OptimisticMessage
        let root_id = LogicalEventId::root(LpId(7), 10);
        let event = RemoteEvent {
            source_lp: LpId(7),
            dest_lp: LpId(8),
            tick: SimTime::from_ticks(50),
            event_payload: b"root-data".to_vec(),
        };
        let authority = OptimisticAuthority::Scoped {
            simulation_namespace: 0xabcd,
            ownership_epoch: 5,
        };
        let msg = OptimisticMessage::try_from_authority_parts(
            event,
            root_id,
            authority,
            1,
            OptimisticMessageKind::Positive,
        )
        .expect("should construct native message");

        // Convert to envelope
        let env =
            KairoOptimisticEnvelopeV1::from_optimistic_message(&msg, GRPC_PROTOCOL_ID, LpId(8), 5)
                .expect("should convert to envelope");
        assert_eq!(env.source_lp, 7);
        assert_eq!(env.dest_lp, 8);
        assert_eq!(env.tick, 50);
        assert_eq!(
            env.envelope_schema_version,
            ENVELOPE_SCHEMA_VERSION_PROTOBUF_V2
        );
        assert_eq!(env.protocol_version, ENVELOPE_PROTOCOL_VERSION_V2);

        // Convert back to native message
        let recovered = env
            .to_optimistic_message(GRPC_PROTOCOL_ID)
            .expect("should convert back to native message");
        assert_eq!(recovered.event().source_lp, LpId(7));
        assert_eq!(recovered.event().dest_lp, LpId(8));
        assert_eq!(recovered.event().tick.ticks(), 50);
        assert_eq!(recovered.incarnation(), 1);
        assert_eq!(recovered.kind(), OptimisticMessageKind::Positive);
    }

    #[test]
    fn test_rejection_oracles() {
        // R01: invalid envelope schema version (3 is unsupported)
        let mut bad_version = KairoOptimisticEnvelopeV1 {
            envelope_schema_version: 3,
            protocol_id: GRPC_PROTOCOL_ID.to_string(),
            protocol_version: ENVELOPE_PROTOCOL_VERSION_V2,
            authority_mode: OptimisticAuthorityMode::Scoped,
            simulation_namespace: 1,
            ownership_epoch: 1,
            emission_epoch: 1,
            source_lp: 1,
            dest_lp: 2,
            tick: 10,
            ancestry_frames: vec![AncestryFrame::Root {
                root_source_lp: 1,
                root_sequence: 1,
            }],
            incarnation: 0,
            kind: WireMessageKind::Positive,
            event_payload: vec![],
        };
        assert_eq!(
            bad_version.validate(GRPC_PROTOCOL_ID),
            Err(RejectionCode::UnsupportedEnvelopeVersion(3))
        );

        // R02: protocol id mismatch
        bad_version.envelope_schema_version = ENVELOPE_SCHEMA_VERSION_PROTOBUF_V2;
        bad_version.protocol_id = "kairo.invalid".to_string();
        assert!(matches!(
            bad_version.validate(GRPC_PROTOCOL_ID),
            Err(RejectionCode::ProtocolIdMismatch(_))
        ));

        // R04: payload too large
        bad_version.protocol_id = GRPC_PROTOCOL_ID.to_string();
        bad_version.event_payload = vec![0u8; MAX_PAYLOAD_BYTES + 1];
        assert!(matches!(
            bad_version.validate(GRPC_PROTOCOL_ID),
            Err(RejectionCode::EnvelopeTooLarge("event_payload", _, _))
        ));

        // R14: root source mismatch
        bad_version.event_payload = vec![];
        bad_version.ancestry_frames = vec![AncestryFrame::Root {
            root_source_lp: 99,
            root_sequence: 1,
        }];
        assert_eq!(
            bad_version.validate(GRPC_PROTOCOL_ID),
            Err(RejectionCode::RootSourceMismatch {
                root_source: 99,
                frame_source: 1,
            })
        );

        // R15: output tick not strictly future
        bad_version.ancestry_frames = vec![
            AncestryFrame::Root {
                root_source_lp: 1,
                root_sequence: 1,
            },
            AncestryFrame::Output {
                parent_tick: 20,
                parent_source_lp: 1,
                ordinal: 0,
            },
        ];
        bad_version.tick = 15; // 15 < 20
        assert!(matches!(
            bad_version.validate(GRPC_PROTOCOL_ID),
            Err(RejectionCode::OutputTickNotStrictlyFuture { .. })
        ));
    }
}
