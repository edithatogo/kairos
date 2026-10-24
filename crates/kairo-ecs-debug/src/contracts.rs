use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt::Formatter;
use thiserror::Error;

pub const TRACE_SCHEMA_V1: &str = "kairoecs.debug-trace.v1";
pub const MAX_JSONL_TOTAL_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_JSONL_RECORD_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DebugContractError {
    #[error("trace header missing schema")]
    MissingSchema,
    #[error("unsupported trace schema: {0}")]
    UnsupportedSchema(String),
    #[error("trace ticks out of order: {current} after {previous}")]
    TicksOutOfOrder { previous: u128, current: u128 },
    #[error("event kind cannot be empty")]
    EmptyKind,
    #[error("byte limit exceeded: actual {actual} bytes, limit {limit} bytes")]
    ExceededByteLimit { limit: usize, actual: usize },
}

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

    pub fn to_json_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    pub fn from_json_str(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }
}

fn deserialize_unique_map<'de, D>(deserializer: D) -> Result<BTreeMap<String, String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct UniqueMapVisitor;

    impl<'de> serde::de::Visitor<'de> for UniqueMapVisitor {
        type Value = BTreeMap<String, String>;

        fn expecting(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("a JSON map without duplicate keys")
        }

        fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
        where
            M: serde::de::MapAccess<'de>,
        {
            let mut map = BTreeMap::new();
            while let Some((key, value)) = access.next_entry::<String, String>()? {
                if map.contains_key(&key) {
                    return Err(serde::de::Error::custom(format!(
                        "duplicate map key: `{key}`"
                    )));
                }
                map.insert(key, value);
            }
            Ok(map)
        }
    }

    deserializer.deserialize_map(UniqueMapVisitor)
}

struct UniqueMapSeed;

impl<'de> serde::de::DeserializeSeed<'de> for UniqueMapSeed {
    type Value = BTreeMap<String, String>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserialize_unique_map(deserializer)
    }
}

/// Typed DTO for a point-in-time state snapshot in execution trace.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TraceSnapshotDto {
    pub tick: u128,
    #[serde(deserialize_with = "deserialize_unique_map")]
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
    #[serde(deserialize_with = "deserialize_unique_map")]
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

    pub fn validate(&self) -> Result<(), DebugContractError> {
        if self.kind.is_empty() {
            return Err(DebugContractError::EmptyKind);
        }
        Ok(())
    }
}

/// Polymorphic trace record DTO supporting both snapshots and deltas.
///
/// Implements custom serialization and deserialization to preserve full-range
/// `u128` ticks regardless of whether `type` precedes `payload` or `payload`
/// precedes `type` in the incoming JSON map.
#[derive(Clone, Debug, PartialEq, Eq)]
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

    pub fn validate(&self) -> Result<(), DebugContractError> {
        match self {
            Self::Snapshot(_) => Ok(()),
            Self::Delta(d) => d.validate(),
        }
    }

    pub fn to_json_line(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    pub fn from_json_str(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }
}

#[derive(Serialize)]
struct SerializeTraceRecord<'a> {
    #[serde(rename = "type")]
    record_type: &'static str,
    payload: SerializePayload<'a>,
}

#[derive(Serialize)]
#[serde(untagged)]
enum SerializePayload<'a> {
    Snapshot(&'a TraceSnapshotDto),
    Delta(&'a TraceDeltaDto),
}

impl Serialize for TraceRecordDto {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Snapshot(s) => {
                let helper = SerializeTraceRecord {
                    record_type: "snapshot",
                    payload: SerializePayload::Snapshot(s),
                };
                helper.serialize(serializer)
            }
            Self::Delta(d) => {
                let helper = SerializeTraceRecord {
                    record_type: "delta",
                    payload: SerializePayload::Delta(d),
                };
                helper.serialize(serializer)
            }
        }
    }
}

struct IntermediatePayload {
    tick: Option<u128>,
    state: Option<BTreeMap<String, String>>,
    event_id_index: Option<u64>,
    event_id_generation: Option<u32>,
    sequence: Option<u64>,
    kind: Option<String>,
    changes: Option<BTreeMap<String, String>>,
}

impl<'de> Deserialize<'de> for IntermediatePayload {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        enum PayloadField {
            Tick,
            State,
            EventIdIndex,
            EventIdGeneration,
            Sequence,
            Kind,
            Changes,
        }

        impl<'de> Deserialize<'de> for PayloadField {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct FieldVisitor;
                impl<'de> serde::de::Visitor<'de> for FieldVisitor {
                    type Value = PayloadField;
                    fn expecting(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                        f.write_str("trace payload field")
                    }
                    fn visit_str<E>(self, value: &str) -> Result<PayloadField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "tick" => Ok(PayloadField::Tick),
                            "state" => Ok(PayloadField::State),
                            "event_id_index" => Ok(PayloadField::EventIdIndex),
                            "event_id_generation" => Ok(PayloadField::EventIdGeneration),
                            "sequence" => Ok(PayloadField::Sequence),
                            "kind" => Ok(PayloadField::Kind),
                            "changes" => Ok(PayloadField::Changes),
                            _ => Err(serde::de::Error::unknown_field(
                                value,
                                &[
                                    "tick",
                                    "state",
                                    "event_id_index",
                                    "event_id_generation",
                                    "sequence",
                                    "kind",
                                    "changes",
                                ],
                            )),
                        }
                    }
                }
                deserializer.deserialize_identifier(FieldVisitor)
            }
        }

        struct IntermediatePayloadVisitor;
        impl<'de> serde::de::Visitor<'de> for IntermediatePayloadVisitor {
            type Value = IntermediatePayload;
            fn expecting(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                f.write_str("map of trace payload")
            }

            fn visit_map<V>(self, mut map: V) -> Result<IntermediatePayload, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut tick = None;
                let mut state = None;
                let mut event_id_index = None;
                let mut event_id_generation = None;
                let mut sequence = None;
                let mut kind = None;
                let mut changes = None;

                while let Some(key) = map.next_key::<PayloadField>()? {
                    match key {
                        PayloadField::Tick => {
                            if tick.is_some() {
                                return Err(serde::de::Error::duplicate_field("tick"));
                            }
                            tick = Some(map.next_value()?);
                        }
                        PayloadField::State => {
                            if state.is_some() {
                                return Err(serde::de::Error::duplicate_field("state"));
                            }
                            state = Some(map.next_value_seed(UniqueMapSeed)?);
                        }
                        PayloadField::EventIdIndex => {
                            if event_id_index.is_some() {
                                return Err(serde::de::Error::duplicate_field("event_id_index"));
                            }
                            event_id_index = Some(map.next_value()?);
                        }
                        PayloadField::EventIdGeneration => {
                            if event_id_generation.is_some() {
                                return Err(serde::de::Error::duplicate_field(
                                    "event_id_generation",
                                ));
                            }
                            event_id_generation = Some(map.next_value()?);
                        }
                        PayloadField::Sequence => {
                            if sequence.is_some() {
                                return Err(serde::de::Error::duplicate_field("sequence"));
                            }
                            sequence = Some(map.next_value()?);
                        }
                        PayloadField::Kind => {
                            if kind.is_some() {
                                return Err(serde::de::Error::duplicate_field("kind"));
                            }
                            let k: String = map.next_value()?;
                            if k.is_empty() {
                                return Err(serde::de::Error::custom("event kind cannot be empty"));
                            }
                            kind = Some(k);
                        }
                        PayloadField::Changes => {
                            if changes.is_some() {
                                return Err(serde::de::Error::duplicate_field("changes"));
                            }
                            changes = Some(map.next_value_seed(UniqueMapSeed)?);
                        }
                    }
                }

                Ok(IntermediatePayload {
                    tick,
                    state,
                    event_id_index,
                    event_id_generation,
                    sequence,
                    kind,
                    changes,
                })
            }
        }

        deserializer.deserialize_map(IntermediatePayloadVisitor)
    }
}

impl<'de> Deserialize<'de> for TraceRecordDto {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        enum Field {
            Type,
            Payload,
        }

        impl<'de> Deserialize<'de> for Field {
            fn deserialize<D>(deserializer: D) -> Result<Field, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct FieldVisitor;
                impl<'de> serde::de::Visitor<'de> for FieldVisitor {
                    type Value = Field;
                    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                        formatter.write_str("`type` or `payload`")
                    }
                    fn visit_str<E>(self, value: &str) -> Result<Field, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "type" => Ok(Field::Type),
                            "payload" => Ok(Field::Payload),
                            _ => Err(serde::de::Error::unknown_field(value, &["type", "payload"])),
                        }
                    }
                }
                deserializer.deserialize_identifier(FieldVisitor)
            }
        }

        struct TraceRecordVisitor;
        impl<'de> serde::de::Visitor<'de> for TraceRecordVisitor {
            type Value = TraceRecordDto;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("struct TraceRecordDto with `type` and `payload`")
            }

            fn visit_map<V>(self, mut map: V) -> Result<TraceRecordDto, V::Error>
            where
                V: serde::de::MapAccess<'de>,
            {
                let mut record_type: Option<String> = None;
                let mut payload: Option<IntermediatePayload> = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Type => {
                            if record_type.is_some() {
                                return Err(serde::de::Error::duplicate_field("type"));
                            }
                            record_type = Some(map.next_value()?);
                        }
                        Field::Payload => {
                            if payload.is_some() {
                                return Err(serde::de::Error::duplicate_field("payload"));
                            }
                            payload = Some(map.next_value()?);
                        }
                    }
                }

                let record_type =
                    record_type.ok_or_else(|| serde::de::Error::missing_field("type"))?;
                let payload = payload.ok_or_else(|| serde::de::Error::missing_field("payload"))?;
                let tick = payload
                    .tick
                    .ok_or_else(|| serde::de::Error::missing_field("tick"))?;

                match record_type.as_str() {
                    "snapshot" => {
                        if payload.event_id_index.is_some()
                            || payload.event_id_generation.is_some()
                            || payload.sequence.is_some()
                            || payload.kind.is_some()
                            || payload.changes.is_some()
                        {
                            return Err(serde::de::Error::custom(
                                "unexpected delta fields in snapshot record",
                            ));
                        }
                        let state = payload
                            .state
                            .ok_or_else(|| serde::de::Error::missing_field("state"))?;
                        Ok(TraceRecordDto::Snapshot(TraceSnapshotDto { tick, state }))
                    }
                    "delta" => {
                        if payload.state.is_some() {
                            return Err(serde::de::Error::custom(
                                "unexpected snapshot state field in delta record",
                            ));
                        }
                        let event_id_index = payload
                            .event_id_index
                            .ok_or_else(|| serde::de::Error::missing_field("event_id_index"))?;
                        let event_id_generation = payload.event_id_generation.ok_or_else(|| {
                            serde::de::Error::missing_field("event_id_generation")
                        })?;
                        let sequence = payload
                            .sequence
                            .ok_or_else(|| serde::de::Error::missing_field("sequence"))?;
                        let kind = payload
                            .kind
                            .ok_or_else(|| serde::de::Error::missing_field("kind"))?;
                        if kind.is_empty() {
                            return Err(serde::de::Error::custom("event kind cannot be empty"));
                        }
                        let changes = payload
                            .changes
                            .ok_or_else(|| serde::de::Error::missing_field("changes"))?;

                        Ok(TraceRecordDto::Delta(TraceDeltaDto {
                            tick,
                            event_id_index,
                            event_id_generation,
                            sequence,
                            kind,
                            changes,
                        }))
                    }
                    other => Err(serde::de::Error::custom(format!(
                        "unsupported record type `{other}`"
                    ))),
                }
            }
        }

        deserializer.deserialize_struct("TraceRecordDto", &["type", "payload"], TraceRecordVisitor)
    }
}

/// Validates a sequence of JSON Lines containing an optional header followed by trace records,
/// enforcing monotonically non-decreasing ticks, non-empty event kinds, and schema compliance.
pub fn validate_jsonl_trace_records(input: &str) -> Result<(), DebugContractError> {
    if input.len() > MAX_JSONL_TOTAL_BYTES {
        return Err(DebugContractError::ExceededByteLimit {
            limit: MAX_JSONL_TOTAL_BYTES,
            actual: input.len(),
        });
    }

    let mut lines = input.lines().filter(|l| !l.trim().is_empty());
    let first = match lines.next() {
        Some(line) => {
            if line.len() > MAX_JSONL_RECORD_BYTES {
                return Err(DebugContractError::ExceededByteLimit {
                    limit: MAX_JSONL_RECORD_BYTES,
                    actual: line.len(),
                });
            }
            line
        }
        None => return Ok(()),
    };

    let mut previous_tick = 0u128;
    let mut first_is_record = false;

    // Check if first line is a valid header
    if let Ok(header) = serde_json::from_str::<TraceHeaderDto>(first) {
        header.validate()?;
    } else {
        // Not a header; treat as first record
        let record: TraceRecordDto = serde_json::from_str(first).map_err(|e| {
            if e.to_string().contains("event kind cannot be empty") {
                DebugContractError::EmptyKind
            } else {
                DebugContractError::MissingSchema
            }
        })?;
        record.validate()?;
        previous_tick = record.tick();
        first_is_record = true;
    }

    for line in lines {
        if line.len() > MAX_JSONL_RECORD_BYTES {
            return Err(DebugContractError::ExceededByteLimit {
                limit: MAX_JSONL_RECORD_BYTES,
                actual: line.len(),
            });
        }
        let record: TraceRecordDto = serde_json::from_str(line).map_err(|e| {
            if e.to_string().contains("event kind cannot be empty") {
                DebugContractError::EmptyKind
            } else {
                DebugContractError::MissingSchema
            }
        })?;
        record.validate()?;
        let tick = record.tick();
        if (first_is_record || previous_tick > 0) && tick < previous_tick {
            return Err(DebugContractError::TicksOutOfOrder {
                previous: previous_tick,
                current: tick,
            });
        }
        previous_tick = tick;
    }
    Ok(())
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

    #[test]
    fn trace_record_deserializes_when_payload_precedes_type_with_u128() {
        let payload_first_snapshot = format!(
            r#"{{"payload":{{"tick":{},"state":{{"k":"v"}}}},"type":"snapshot"}}"#,
            u128::MAX
        );
        let de: TraceRecordDto = serde_json::from_str(&payload_first_snapshot)
            .expect("deserialize snapshot payload first");
        match de {
            TraceRecordDto::Snapshot(s) => {
                assert_eq!(s.tick, u128::MAX);
                assert_eq!(s.state.get("k").unwrap(), "v");
            }
            _ => panic!("expected snapshot"),
        }

        let payload_first_delta = format!(
            r#"{{"payload":{{"tick":{},"event_id_index":{},"event_id_generation":{},"sequence":{},"kind":"custom:99","changes":{{}}}},"type":"delta"}}"#,
            u128::MAX,
            u64::MAX,
            u32::MAX,
            u64::MAX - 1
        );
        let de: TraceRecordDto =
            serde_json::from_str(&payload_first_delta).expect("deserialize delta payload first");
        match de {
            TraceRecordDto::Delta(d) => {
                assert_eq!(d.tick, u128::MAX);
                assert_eq!(d.event_id_index, u64::MAX);
                assert_eq!(d.event_id_generation, u32::MAX);
                assert_eq!(d.sequence, u64::MAX - 1);
            }
            _ => panic!("expected delta"),
        }
    }

    #[test]
    fn unicode_and_control_character_escaping_in_trace_state() {
        let mut state = BTreeMap::new();
        let key = "entity.\"special\"\n\t\u{0000}\u{1F680}";
        let value = "val with quotes \" and \r\n and backslash \\";
        state.insert(key.to_string(), value.to_string());

        let snapshot = TraceSnapshotDto::new(100, state);
        let record = TraceRecordDto::Snapshot(snapshot);
        let json_line = record.to_json_line().expect("serialize to json line");

        assert!(json_line.contains(r#"\u0000"#));
        assert!(json_line.contains(r#"\"special\""#));
        assert!(json_line.contains(r#"\\\""#) || json_line.contains(r#"\""#));

        let de = TraceRecordDto::from_json_str(&json_line).expect("deserialize json line");
        match de {
            TraceRecordDto::Snapshot(s) => {
                assert_eq!(s.tick, 100);
                assert_eq!(s.state.get(key).unwrap(), value);
            }
            _ => panic!("expected snapshot"),
        }
    }

    #[test]
    fn exact_u128_tick_preservation_no_f64_narrowing() {
        // u128::MAX is 340282366920938463463374607431768211455
        // Standard f64 loses precision past 2^53 - 1 (9007199254740991)
        let huge_tick: u128 = 340282366920938463463374607431768211450;
        let mut changes = BTreeMap::new();
        changes.insert("status".to_string(), "ok".to_string());

        let delta = TraceDeltaDto::new(huge_tick, 1, 0, 1, "test_event", changes).unwrap();
        let record = TraceRecordDto::Delta(delta);

        let line = record.to_json_line().expect("serialize");
        let de = TraceRecordDto::from_json_str(&line).expect("deserialize");
        assert_eq!(de.tick(), huge_tick);
    }

    #[test]
    fn rejects_non_integer_floats_and_malformed_inputs() {
        // Floats in integer fields must be rejected
        let float_tick = r#"{"type":"snapshot","payload":{"tick":12.345,"state":{}}}"#;
        assert!(TraceRecordDto::from_json_str(float_tick).is_err());

        let malformed_record = r#"{"type":"snapshot","payload":{"tick":123,"state":"not-a-map"}}"#;
        assert!(TraceRecordDto::from_json_str(malformed_record).is_err());

        let malformed_header = r#"{"schema":"wrong.schema"}"#;
        let header = TraceHeaderDto::from_json_str(malformed_header).expect("parses as json");
        assert!(header.validate().is_err());
    }

    #[test]
    fn rejects_duplicate_map_keys_in_state_and_changes() {
        // Repeated key in snapshot state
        let duplicate_state_json =
            r#"{"type":"snapshot","payload":{"tick":10,"state":{"k1":"v1","k1":"v2"}}}"#;
        let res = TraceRecordDto::from_json_str(duplicate_state_json);
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("duplicate map key"));

        // Repeated key in delta changes
        let duplicate_changes_json = r#"{"type":"delta","payload":{"tick":10,"event_id_index":1,"event_id_generation":0,"sequence":1,"kind":"k","changes":{"k1":"v1","k1":"v2"}}}"#;
        let res = TraceRecordDto::from_json_str(duplicate_changes_json);
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("duplicate map key"));
    }

    #[test]
    fn rejects_missing_variant_required_fields_and_cross_variant_fields() {
        // Snapshot missing state
        let missing_state = r#"{"type":"snapshot","payload":{"tick":10}}"#;
        assert!(TraceRecordDto::from_json_str(missing_state).is_err());

        // Snapshot containing delta field (event_id_index)
        let snapshot_with_delta =
            r#"{"type":"snapshot","payload":{"tick":10,"state":{},"event_id_index":1}}"#;
        assert!(TraceRecordDto::from_json_str(snapshot_with_delta).is_err());

        // Delta missing changes
        let missing_changes = r#"{"type":"delta","payload":{"tick":10,"event_id_index":1,"event_id_generation":0,"sequence":1,"kind":"k"}}"#;
        assert!(TraceRecordDto::from_json_str(missing_changes).is_err());

        // Delta containing snapshot field (state)
        let delta_with_snapshot = r#"{"type":"delta","payload":{"tick":10,"event_id_index":1,"event_id_generation":0,"sequence":1,"kind":"k","changes":{},"state":{}}}"#;
        assert!(TraceRecordDto::from_json_str(delta_with_snapshot).is_err());
    }

    #[test]
    fn enforces_jsonl_byte_limits() {
        // Record exceeding MAX_JSONL_RECORD_BYTES
        let long_line = "x".repeat(MAX_JSONL_RECORD_BYTES + 1);
        let err = validate_jsonl_trace_records(&long_line).unwrap_err();
        assert!(matches!(
            err,
            DebugContractError::ExceededByteLimit {
                limit: MAX_JSONL_RECORD_BYTES,
                ..
            }
        ));

        // Stream exceeding MAX_JSONL_TOTAL_BYTES
        let huge_stream = "x".repeat(MAX_JSONL_TOTAL_BYTES + 1);
        let err = validate_jsonl_trace_records(&huge_stream).unwrap_err();
        assert!(matches!(
            err,
            DebugContractError::ExceededByteLimit {
                limit: MAX_JSONL_TOTAL_BYTES,
                ..
            }
        ));
    }

    #[test]
    fn jsonl_stream_encoding_and_monotonicity_validation() {
        let header = TraceHeaderDto::default().to_json_string().unwrap();
        let rec1 = TraceRecordDto::Snapshot(TraceSnapshotDto::new(10, BTreeMap::new()))
            .to_json_line()
            .unwrap();
        let rec2 = TraceRecordDto::Snapshot(TraceSnapshotDto::new(20, BTreeMap::new()))
            .to_json_line()
            .unwrap();

        let valid_stream = format!("{header}\n{rec1}\n{rec2}\n");
        assert!(validate_jsonl_trace_records(&valid_stream).is_ok());

        // Out of order ticks: 20 then 15
        let rec3_bad = TraceRecordDto::Snapshot(TraceSnapshotDto::new(15, BTreeMap::new()))
            .to_json_line()
            .unwrap();
        let invalid_stream = format!("{header}\n{rec1}\n{rec2}\n{rec3_bad}\n");
        let err = validate_jsonl_trace_records(&invalid_stream).unwrap_err();
        assert!(matches!(err, DebugContractError::TicksOutOfOrder { .. }));
    }

    #[test]
    fn rejects_empty_kind_in_delta_at_first_and_subsequent_positions() {
        // Empty kind delta at first position (headerless stream)
        let empty_kind_delta_first = r#"{"type":"delta","payload":{"tick":10,"event_id_index":1,"event_id_generation":0,"sequence":1,"kind":"","changes":{}}}"#;
        let err1 = validate_jsonl_trace_records(empty_kind_delta_first).unwrap_err();
        assert_eq!(err1, DebugContractError::EmptyKind);

        // Empty kind delta at subsequent position (after valid header and record)
        let header = TraceHeaderDto::default().to_json_string().unwrap();
        let valid_rec = TraceRecordDto::Snapshot(TraceSnapshotDto::new(10, BTreeMap::new()))
            .to_json_line()
            .unwrap();
        let stream = format!("{header}\n{valid_rec}\n{empty_kind_delta_first}\n");
        let err2 = validate_jsonl_trace_records(&stream).unwrap_err();
        assert_eq!(err2, DebugContractError::EmptyKind);
    }

    #[test]
    fn thiserror_debug_contract_error_messages_and_traits() {
        use std::error::Error;

        let err1 = DebugContractError::MissingSchema;
        assert_eq!(err1.to_string(), "trace header missing schema");
        assert!(err1.source().is_none());

        let err2 = DebugContractError::UnsupportedSchema("invalid.v2".to_string());
        assert_eq!(err2.to_string(), "unsupported trace schema: invalid.v2");

        let err3 = DebugContractError::TicksOutOfOrder {
            previous: 10,
            current: 5,
        };
        assert_eq!(err3.to_string(), "trace ticks out of order: 5 after 10");

        let err4 = DebugContractError::EmptyKind;
        assert_eq!(err4.to_string(), "event kind cannot be empty");

        let err5 = DebugContractError::ExceededByteLimit {
            limit: 100,
            actual: 200,
        };
        assert_eq!(
            err5.to_string(),
            "byte limit exceeded: actual 200 bytes, limit 100 bytes"
        );
    }
}
