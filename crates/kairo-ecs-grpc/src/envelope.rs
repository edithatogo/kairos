#![forbid(unsafe_code)]

use kairo_ecs_pdes::{
    LogicalEventId, LpId, OptimisticAuthority, OptimisticError, OptimisticEventOrderKey,
    OptimisticMessage, OptimisticMessageKind, RemoteEvent,
};
use kairo_ecs_types::SimTime;
use std::fmt;

use crate::GRPC_PROTOCOL_VERSION;

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
        if self.envelope_schema_version != 1 {
            return Err(RejectionCode::UnsupportedEnvelopeVersion(
                self.envelope_schema_version,
            ));
        }
        if self.protocol_id != expected_protocol_id {
            return Err(RejectionCode::ProtocolIdMismatch(self.protocol_id.clone()));
        }
        if self.protocol_version != GRPC_PROTOCOL_VERSION {
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
            envelope_schema_version: 1,
            protocol_id: protocol_id.to_string(),
            protocol_version: GRPC_PROTOCOL_VERSION,
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

    /// Serializes the envelope into deterministic binary bytes.
    pub fn encode(&self) -> Result<Vec<u8>, RejectionCode> {
        let mut buf = Vec::with_capacity(256 + self.event_payload.len());

        buf.extend_from_slice(&self.envelope_schema_version.to_be_bytes());
        let proto_bytes = self.protocol_id.as_bytes();
        buf.extend_from_slice(&(proto_bytes.len() as u16).to_be_bytes());
        buf.extend_from_slice(proto_bytes);
        buf.extend_from_slice(&self.protocol_version.to_be_bytes());
        buf.extend_from_slice(&(self.authority_mode as u32).to_be_bytes());
        buf.extend_from_slice(&self.simulation_namespace.to_be_bytes());
        buf.extend_from_slice(&self.ownership_epoch.to_be_bytes());
        buf.extend_from_slice(&self.emission_epoch.to_be_bytes());
        buf.extend_from_slice(&self.source_lp.to_be_bytes());
        buf.extend_from_slice(&self.dest_lp.to_be_bytes());
        buf.extend_from_slice(&self.tick.to_be_bytes());
        buf.extend_from_slice(&self.incarnation.to_be_bytes());
        buf.extend_from_slice(&(self.kind as u32).to_be_bytes());

        // Ancestry frames count and items
        buf.extend_from_slice(&(self.ancestry_frames.len() as u32).to_be_bytes());
        for frame in &self.ancestry_frames {
            match frame {
                AncestryFrame::Root {
                    root_source_lp,
                    root_sequence,
                } => {
                    buf.push(0u8); // tag 0 = Root
                    buf.extend_from_slice(&root_source_lp.to_be_bytes());
                    buf.extend_from_slice(&root_sequence.to_be_bytes());
                }
                AncestryFrame::Output {
                    parent_tick,
                    parent_source_lp,
                    ordinal,
                } => {
                    buf.push(1u8); // tag 1 = Output
                    buf.extend_from_slice(&parent_tick.to_be_bytes());
                    buf.extend_from_slice(&parent_source_lp.to_be_bytes());
                    buf.extend_from_slice(&ordinal.to_be_bytes());
                }
            }
        }

        // Payload
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

    /// Deserializes an envelope from binary bytes with strict preflight bounds checks.
    pub fn decode(bytes: &[u8]) -> Result<Self, RejectionCode> {
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
        if schema_ver != 1 {
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

        // Fixed fields:
        // protocol_version: 2
        // authority_mode: 4
        // simulation_namespace: 16
        // ownership_epoch: 8
        // emission_epoch: 8
        // source_lp: 4
        // dest_lp: 4
        // tick: 16
        // incarnation: 8
        // kind: 4
        // Total fixed = 74 bytes
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

        // Frames
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
                    // RootFrame: 4 + 8 = 12 bytes
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
                    // OutputFrame: 16 + 4 + 4 = 24 bytes
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

        // Payload
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GRPC_PROTOCOL_ID;

    #[test]
    fn test_envelope_roundtrip_root() {
        let envelope = KairoOptimisticEnvelopeV1 {
            envelope_schema_version: 1,
            protocol_id: GRPC_PROTOCOL_ID.to_string(),
            protocol_version: 1,
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
            envelope_schema_version: 1,
            protocol_id: GRPC_PROTOCOL_ID.to_string(),
            protocol_version: 1,
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
        // R01: invalid envelope schema version
        let mut bad_version = KairoOptimisticEnvelopeV1 {
            envelope_schema_version: 2,
            protocol_id: GRPC_PROTOCOL_ID.to_string(),
            protocol_version: 1,
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
            Err(RejectionCode::UnsupportedEnvelopeVersion(2))
        );

        // R02: protocol id mismatch
        bad_version.envelope_schema_version = 1;
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
