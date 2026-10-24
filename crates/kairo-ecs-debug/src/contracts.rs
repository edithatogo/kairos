use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};

pub const TRACE_SCHEMA_V1: &str = "kairoecs.debug-trace.v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DebugContractError {
    MissingSchema,
    UnsupportedSchema(String),
    TicksOutOfOrder { previous: u128, current: u128 },
    EmptyKind,
}

impl Display for DebugContractError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingSchema => write!(f, "trace header missing schema"),
            Self::UnsupportedSchema(schema) => write!(f, "unsupported trace schema: {schema}"),
            Self::TicksOutOfOrder { previous, current } => {
                write!(f, "trace ticks out of order: {current} after {previous}")
            }
            Self::EmptyKind => write!(f, "event kind cannot be empty"),
        }
    }
}

impl std::error::Error for DebugContractError {}

/// Typed DTO for trace header declaration.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TraceHeaderDto {
    pub schema: String,
}

impl Default for TraceHeaderDto {
    fn default() -> Self {
        Self {
            schema: TRACE_SCHEMA_V1.to_string(),
        }
    }
}

impl TraceHeaderDto {
    pub fn new(schema: impl Into<String>) -> Self {
        Self {
            schema: schema.into(),
        }
    }

    pub fn validate(&self) -> Result<(), DebugContractError> {
        if self.schema != TRACE_SCHEMA_V1 {
            return Err(DebugContractError::UnsupportedSchema(self.schema.clone()));
        }
        Ok(())
    }
}

/// Typed DTO for a point-in-time state snapshot in execution trace.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TraceSnapshotDto {
    pub tick: u128,
    pub state: BTreeMap<String, String>,
}

impl TraceSnapshotDto {
    pub fn new(tick: u128, state: BTreeMap<String, String>) -> Self {
        Self { tick, state }
    }
}

/// Typed DTO for an incremental delta record in execution trace.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TraceDeltaDto {
    pub tick: u128,
    pub event_id_index: u64,
    pub event_id_generation: u32,
    pub sequence: u64,
    pub kind: String,
    pub changes: BTreeMap<String, String>,
}

impl TraceDeltaDto {
    pub fn new(
        tick: u128,
        event_id_index: u64,
        event_id_generation: u32,
        sequence: u64,
        kind: impl Into<String>,
        changes: BTreeMap<String, String>,
    ) -> Result<Self, DebugContractError> {
        let kind = kind.into();
        if kind.is_empty() {
            return Err(DebugContractError::EmptyKind);
        }
        Ok(Self {
            tick,
            event_id_index,
            event_id_generation,
            sequence,
            kind,
            changes,
        })
    }
}

/// Polymorphic trace record DTO supporting both snapshots and deltas.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(
    tag = "type",
    content = "payload",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum TraceRecordDto {
    Snapshot(TraceSnapshotDto),
    Delta(TraceDeltaDto),
}

impl TraceRecordDto {
    pub fn tick(&self) -> u128 {
        match self {
            Self::Snapshot(s) => s.tick,
            Self::Delta(d) => d.tick,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trace_header_roundtrip_and_validation() {
        let header = TraceHeaderDto::default();
        assert!(header.validate().is_ok());

        let ser = serde_json::to_string(&header).expect("serialize");
        let de: TraceHeaderDto = serde_json::from_str(&ser).expect("deserialize");
        assert_eq!(header, de);

        let bad_header = TraceHeaderDto::new("kairoecs.debug-trace.v99");
        assert!(matches!(
            bad_header.validate(),
            Err(DebugContractError::UnsupportedSchema(_))
        ));
    }

    #[test]
    fn trace_header_rejects_unknown_fields() {
        let json = r#"{"schema": "kairoecs.debug-trace.v1", "unknown_key": "val"}"#;
        let res: Result<TraceHeaderDto, _> = serde_json::from_str(json);
        assert!(res.is_err());
    }

    #[test]
    fn snapshot_and_delta_roundtrip_full_range_integers() {
        let mut state = BTreeMap::new();
        state.insert("agent.0".to_string(), "active".to_string());
        state.insert("resource.bay".to_string(), "available".to_string());

        let snapshot = TraceSnapshotDto::new(u128::MAX, state.clone());
        let ser = serde_json::to_string(&snapshot).expect("serialize");
        let de: TraceSnapshotDto = serde_json::from_str(&ser).expect("deserialize");
        assert_eq!(snapshot, de);
        assert_eq!(de.tick, u128::MAX);

        let delta = TraceDeltaDto::new(
            u128::MAX - 1,
            u64::MAX,
            u32::MAX,
            u64::MAX - 5,
            "custom:42",
            state,
        )
        .expect("valid delta");
        let ser = serde_json::to_string(&delta).expect("serialize");
        let de: TraceDeltaDto = serde_json::from_str(&ser).expect("deserialize");
        assert_eq!(delta, de);
        assert_eq!(de.event_id_index, u64::MAX);
        assert_eq!(de.event_id_generation, u32::MAX);
        assert_eq!(de.sequence, u64::MAX - 5);
    }

    #[test]
    fn trace_record_enum_tagged_roundtrip() {
        let mut changes = BTreeMap::new();
        changes.insert("q.len".to_string(), "1".to_string());

        let delta = TraceDeltaDto::new(100, 1, 0, 10, "custom:1", changes).unwrap();
        let record = TraceRecordDto::Delta(delta);

        let ser = serde_json::to_string(&record).expect("serialize");
        assert!(ser.contains("\"type\":\"delta\""));

        let de: TraceRecordDto = serde_json::from_str(&ser).expect("deserialize");
        assert_eq!(record, de);
        assert_eq!(de.tick(), 100);
    }
}
