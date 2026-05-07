#![forbid(unsafe_code)]

use kairo_ecs_types::{DispatchedEvent, EntityId, EventId};
use std::error::Error;
use std::fmt::{Display, Formatter};

pub const SCHEMA_VERSION: u16 = 1;
pub const EVENT_LOG_STREAM: &str = "kairo_ecs.event_log.v1";
pub const TIME_SCALE_TICKS: &str = "ticks";

const SMOKE_HEADER: &str = "schema_version\trun_id\tevent_id_hex\tentity_id_hex\ttime_ticks_le_hex\ttime_scale\tpriority\tsequence\tevent_kind\tstatus\tpayload_ref";

pub const EVENT_LOG_FIELDS: &[EventLogField] = &[
    EventLogField::new("schema_version", "UInt16", false),
    EventLogField::new("run_id", "Utf8", false),
    EventLogField::new("event_id", "FixedSizeBinary(12)", false),
    EventLogField::new("entity_id", "FixedSizeBinary(12)", true),
    EventLogField::new("time_ticks", "FixedSizeBinary(16)", false),
    EventLogField::new("time_scale", "Utf8", false),
    EventLogField::new("priority", "Int32", false),
    EventLogField::new("sequence", "UInt64", false),
    EventLogField::new("event_kind", "Utf8", false),
    EventLogField::new("status", "Utf8", false),
    EventLogField::new("payload_ref", "Utf8", true),
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EventLogField {
    pub name: &'static str,
    pub data_type: &'static str,
    pub nullable: bool,
}

impl EventLogField {
    pub const fn new(name: &'static str, data_type: &'static str, nullable: bool) -> Self {
        Self {
            name,
            data_type,
            nullable,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EventStatus {
    Dispatched,
    Cancelled,
    Skipped,
    Error,
}

impl EventStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Dispatched => "dispatched",
            Self::Cancelled => "cancelled",
            Self::Skipped => "skipped",
            Self::Error => "error",
        }
    }
}

impl Display for EventStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl TryFrom<&str> for EventStatus {
    type Error = ArrowError;

    fn try_from(value: &str) -> Result<Self, ArrowError> {
        match value {
            "dispatched" => Ok(Self::Dispatched),
            "cancelled" => Ok(Self::Cancelled),
            "skipped" => Ok(Self::Skipped),
            "error" => Ok(Self::Error),
            other => Err(ArrowError::InvalidStatus(other.to_string())),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventLogRecord {
    pub schema_version: u16,
    pub run_id: String,
    pub event_id: EventId,
    pub entity_id: Option<EntityId>,
    pub time_ticks: u128,
    pub time_scale: String,
    pub priority: i32,
    pub sequence: u64,
    pub event_kind: String,
    pub status: EventStatus,
    pub payload_ref: Option<String>,
}

impl EventLogRecord {
    pub fn dispatched(run_id: &str, event: DispatchedEvent) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            run_id: run_id.to_string(),
            event_id: event.id,
            entity_id: event.entity,
            time_ticks: event.at.ticks(),
            time_scale: TIME_SCALE_TICKS.to_string(),
            priority: event.priority,
            sequence: event.sequence,
            event_kind: format!("custom:{}", event.kind.code()),
            status: EventStatus::Dispatched,
            payload_ref: None,
        }
    }

    pub fn validate(&self) -> Result<(), ArrowError> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(ArrowError::UnsupportedSchemaVersion(self.schema_version));
        }
        if self.run_id.trim().is_empty() {
            return Err(ArrowError::EmptyField("run_id"));
        }
        if self.time_scale != TIME_SCALE_TICKS {
            return Err(ArrowError::InvalidTimeScale(self.time_scale.clone()));
        }
        if self.event_kind.trim().is_empty() {
            return Err(ArrowError::EmptyField("event_kind"));
        }
        if self
            .payload_ref
            .as_ref()
            .is_some_and(|payload_ref| payload_ref.trim().is_empty())
        {
            return Err(ArrowError::EmptyField("payload_ref"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventLogBatch {
    records: Vec<EventLogRecord>,
}

impl EventLogBatch {
    pub fn new(records: Vec<EventLogRecord>) -> Result<Self, ArrowError> {
        for record in &records {
            record.validate()?;
        }
        Ok(Self { records })
    }

    pub fn records(&self) -> &[EventLogRecord] {
        &self.records
    }

    pub fn schema(&self) -> &'static [EventLogField] {
        EVENT_LOG_FIELDS
    }

    pub fn to_smoke_bytes(&self) -> Vec<u8> {
        let mut lines = vec![
            format!("stream={EVENT_LOG_STREAM};schema_version={SCHEMA_VERSION}"),
            SMOKE_HEADER.to_string(),
        ];

        for record in &self.records {
            lines.push(
                [
                    record.schema_version.to_string(),
                    escape_cell(&record.run_id),
                    handle_hex(record.event_id.index, record.event_id.generation),
                    record
                        .entity_id
                        .map(|entity_id| handle_hex(entity_id.index, entity_id.generation))
                        .unwrap_or_default(),
                    record.time_ticks.to_le_bytes().encode_hex(),
                    escape_cell(&record.time_scale),
                    record.priority.to_string(),
                    record.sequence.to_string(),
                    escape_cell(&record.event_kind),
                    record.status.to_string(),
                    escape_cell(record.payload_ref.as_deref().unwrap_or_default()),
                ]
                .join("\t"),
            );
        }

        lines.push(String::new());
        lines.join("\n").into_bytes()
    }

    pub fn from_smoke_bytes(payload: &[u8]) -> Result<Self, ArrowError> {
        let text = std::str::from_utf8(payload).map_err(|_| ArrowError::InvalidUtf8)?;
        let mut lines = text.lines();
        let expected_stream = format!("stream={EVENT_LOG_STREAM};schema_version={SCHEMA_VERSION}");

        if lines.next() != Some(expected_stream.as_str()) {
            return Err(ArrowError::UnexpectedStreamHeader);
        }
        if lines.next() != Some(SMOKE_HEADER) {
            return Err(ArrowError::UnexpectedFieldHeader);
        }

        let mut records = Vec::new();
        for line in lines {
            if line.is_empty() {
                continue;
            }
            let cells: Vec<_> = line.split('\t').collect();
            if cells.len() != 11 {
                return Err(ArrowError::WrongCellCount {
                    expected: 11,
                    actual: cells.len(),
                });
            }

            records.push(EventLogRecord {
                schema_version: cells[0]
                    .parse()
                    .map_err(|_| ArrowError::InvalidNumber("schema_version"))?,
                run_id: unescape_cell(cells[1]),
                event_id: parse_event_handle(cells[2])?,
                entity_id: if cells[3].is_empty() {
                    None
                } else {
                    Some(parse_entity_handle(cells[3])?)
                },
                time_ticks: parse_time_ticks(cells[4])?,
                time_scale: unescape_cell(cells[5]),
                priority: cells[6]
                    .parse()
                    .map_err(|_| ArrowError::InvalidNumber("priority"))?,
                sequence: cells[7]
                    .parse()
                    .map_err(|_| ArrowError::InvalidNumber("sequence"))?,
                event_kind: unescape_cell(cells[8]),
                status: EventStatus::try_from(cells[9])?,
                payload_ref: {
                    let payload_ref = unescape_cell(cells[10]);
                    if payload_ref.is_empty() {
                        None
                    } else {
                        Some(payload_ref)
                    }
                },
            });
        }

        Self::new(records)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ArrowError {
    EmptyField(&'static str),
    InvalidHexLength { expected: usize, actual: usize },
    InvalidHexDigit,
    InvalidNumber(&'static str),
    InvalidStatus(String),
    InvalidTimeScale(String),
    InvalidUtf8,
    UnexpectedFieldHeader,
    UnexpectedStreamHeader,
    UnsupportedSchemaVersion(u16),
    WrongCellCount { expected: usize, actual: usize },
}

impl Display for ArrowError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyField(field) => write!(f, "{field} must not be empty"),
            Self::InvalidHexLength { expected, actual } => {
                write!(f, "expected {expected} hex characters, got {actual}")
            }
            Self::InvalidHexDigit => f.write_str("invalid hex digit"),
            Self::InvalidNumber(field) => write!(f, "{field} is not a valid number"),
            Self::InvalidStatus(status) => write!(f, "invalid event status: {status}"),
            Self::InvalidTimeScale(time_scale) => write!(f, "invalid time scale: {time_scale}"),
            Self::InvalidUtf8 => f.write_str("payload is not valid UTF-8"),
            Self::UnexpectedFieldHeader => f.write_str("unexpected event-log field header"),
            Self::UnexpectedStreamHeader => f.write_str("unexpected event-log stream header"),
            Self::UnsupportedSchemaVersion(version) => {
                write!(f, "unsupported event-log schema version: {version}")
            }
            Self::WrongCellCount { expected, actual } => {
                write!(f, "expected {expected} cells, got {actual}")
            }
        }
    }
}

impl Error for ArrowError {}

pub struct ArrowEventLog {
    run_id: String,
    records: Vec<EventLogRecord>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ArrowEventLogEntry<'a> {
    pub event_id: &'a str,
    pub entity_id: Option<&'a str>,
    pub time_ticks: u128,
    pub priority: i32,
    pub sequence: u64,
    pub kind: u32,
    pub status: &'a str,
}

impl ArrowEventLog {
    pub fn new(run_id: &str) -> Self {
        Self {
            run_id: run_id.to_string(),
            records: Vec::new(),
        }
    }

    pub fn record_event(&mut self, event: ArrowEventLogEntry<'_>) {
        self.records.push(EventLogRecord {
            schema_version: SCHEMA_VERSION,
            run_id: self.run_id.clone(),
            event_id: synthetic_handle(event.event_id),
            entity_id: event.entity_id.map(synthetic_handle_entity),
            time_ticks: event.time_ticks,
            time_scale: TIME_SCALE_TICKS.to_string(),
            priority: event.priority,
            sequence: event.sequence,
            event_kind: format!("custom:{}", event.kind),
            status: EventStatus::try_from(event.status).unwrap_or(EventStatus::Error),
            payload_ref: None,
        });
    }

    pub fn flush_json(&self) -> String {
        let events = self
            .records
            .iter()
            .map(record_json)
            .collect::<Vec<_>>()
            .join(",\n    ");

        format!(
            "{{\n  \"stream\": {},\n  \"schema_version\": {},\n  \"run_id\": {},\n  \"event_count\": {},\n  \"events\": [\n    {}\n  ]\n}}",
            json_string(EVENT_LOG_STREAM),
            SCHEMA_VERSION,
            json_string(&self.run_id),
            self.records.len(),
            events
        )
    }

    pub fn flush_ndjson(&self) -> String {
        self.records
            .iter()
            .map(record_json)
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}

fn synthetic_handle(value: &str) -> EventId {
    EventId::new(stable_hash(value), 0)
}

fn synthetic_handle_entity(value: &str) -> EntityId {
    EntityId::new(stable_hash(value), 0)
}

fn stable_hash(value: &str) -> u64 {
    value.bytes().fold(0xcbf29ce484222325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
    })
}

fn parse_event_handle(hex_value: &str) -> Result<EventId, ArrowError> {
    let bytes = parse_fixed_hex::<12>(hex_value)?;
    Ok(EventId::new(
        u64::from_le_bytes(bytes[..8].try_into().expect("slice length checked")),
        u32::from_le_bytes(bytes[8..].try_into().expect("slice length checked")),
    ))
}

fn parse_entity_handle(hex_value: &str) -> Result<EntityId, ArrowError> {
    let bytes = parse_fixed_hex::<12>(hex_value)?;
    Ok(EntityId::new(
        u64::from_le_bytes(bytes[..8].try_into().expect("slice length checked")),
        u32::from_le_bytes(bytes[8..].try_into().expect("slice length checked")),
    ))
}

fn parse_time_ticks(hex_value: &str) -> Result<u128, ArrowError> {
    let bytes = parse_fixed_hex::<16>(hex_value)?;
    Ok(u128::from_le_bytes(bytes))
}

fn parse_fixed_hex<const N: usize>(hex_value: &str) -> Result<[u8; N], ArrowError> {
    if hex_value.len() != N * 2 {
        return Err(ArrowError::InvalidHexLength {
            expected: N * 2,
            actual: hex_value.len(),
        });
    }

    let mut bytes = [0u8; N];
    for index in 0..N {
        bytes[index] = u8::from_str_radix(&hex_value[index * 2..index * 2 + 2], 16)
            .map_err(|_| ArrowError::InvalidHexDigit)?;
    }
    Ok(bytes)
}

fn handle_hex(index: u64, generation: u32) -> String {
    [
        index.to_le_bytes().as_slice(),
        generation.to_le_bytes().as_slice(),
    ]
    .concat()
    .encode_hex()
}

fn record_json(record: &EventLogRecord) -> String {
    let entity_id = record
        .entity_id
        .map(|entity_id| json_string(&handle_hex(entity_id.index, entity_id.generation)))
        .unwrap_or_else(|| "null".to_string());
    let payload_ref = record
        .payload_ref
        .as_ref()
        .map(|payload_ref| json_string(payload_ref))
        .unwrap_or_else(|| "null".to_string());

    format!(
        "{{\"schema_version\":{},\"run_id\":{},\"event_id\":{},\"entity_id\":{},\"time_ticks\":{},\"time_scale\":{},\"priority\":{},\"sequence\":{},\"event_kind\":{},\"status\":{},\"payload_ref\":{}}}",
        record.schema_version,
        json_string(&record.run_id),
        json_string(&handle_hex(record.event_id.index, record.event_id.generation)),
        entity_id,
        json_string(&record.time_ticks.to_string()),
        json_string(&record.time_scale),
        record.priority,
        record.sequence,
        json_string(&record.event_kind),
        json_string(record.status.as_str()),
        payload_ref
    )
}

fn json_string(value: &str) -> String {
    let mut output = String::with_capacity(value.len() + 2);
    output.push('"');
    for char in value.chars() {
        match char {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            other if other.is_control() => output.push_str(&format!("\\u{:04x}", other as u32)),
            other => output.push(other),
        }
    }
    output.push('"');
    output
}

fn escape_cell(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\n', "\\n")
}

fn unescape_cell(value: &str) -> String {
    let mut output = String::new();
    let mut chars = value.chars();
    while let Some(char) = chars.next() {
        if char == '\\' {
            match chars.next() {
                Some('t') => output.push('\t'),
                Some('n') => output.push('\n'),
                Some('\\') => output.push('\\'),
                Some(other) => {
                    output.push('\\');
                    output.push(other);
                }
                None => output.push('\\'),
            }
        } else {
            output.push(char);
        }
    }
    output
}

trait EncodeHex {
    fn encode_hex(&self) -> String;
}

impl<T: AsRef<[u8]>> EncodeHex for T {
    fn encode_hex(&self) -> String {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let bytes = self.as_ref();
        let mut output = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            output.push(HEX[(byte >> 4) as usize] as char);
            output.push(HEX[(byte & 0x0f) as usize] as char);
        }
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kairo_ecs_types::{DispatchedEvent, EventKind, SimTime};

    fn sample_event() -> DispatchedEvent {
        DispatchedEvent::new(
            EventId::new(1, 2),
            SimTime::from_ticks(10),
            -3,
            4,
            Some(EntityId::new(5, 6)),
            EventKind::custom(7),
        )
    }

    #[test]
    fn event_log_schema_matches_track_04_order() {
        let fields: Vec<_> = EVENT_LOG_FIELDS
            .iter()
            .map(|field| (field.name, field.data_type, field.nullable))
            .collect();

        assert_eq!(
            fields,
            vec![
                ("schema_version", "UInt16", false),
                ("run_id", "Utf8", false),
                ("event_id", "FixedSizeBinary(12)", false),
                ("entity_id", "FixedSizeBinary(12)", true),
                ("time_ticks", "FixedSizeBinary(16)", false),
                ("time_scale", "Utf8", false),
                ("priority", "Int32", false),
                ("sequence", "UInt64", false),
                ("event_kind", "Utf8", false),
                ("status", "Utf8", false),
                ("payload_ref", "Utf8", true),
            ]
        );
    }

    #[test]
    fn dispatched_record_maps_core_event_fields() {
        let record = EventLogRecord::dispatched("run-1", sample_event());

        assert_eq!(record.schema_version, SCHEMA_VERSION);
        assert_eq!(record.run_id, "run-1");
        assert_eq!(record.event_id, EventId::new(1, 2));
        assert_eq!(record.entity_id, Some(EntityId::new(5, 6)));
        assert_eq!(record.time_ticks, 10);
        assert_eq!(record.priority, -3);
        assert_eq!(record.sequence, 4);
        assert_eq!(record.event_kind, "custom:7");
        assert_eq!(record.status, EventStatus::Dispatched);
    }

    #[test]
    fn event_log_batch_round_trips_smoke_bytes() {
        let batch = EventLogBatch::new(vec![EventLogRecord::dispatched("run-1", sample_event())])
            .expect("valid batch");

        let decoded = EventLogBatch::from_smoke_bytes(&batch.to_smoke_bytes()).expect("roundtrip");

        assert_eq!(decoded, batch);
        assert_eq!(decoded.schema(), EVENT_LOG_FIELDS);
    }

    #[test]
    fn smoke_bytes_preserve_escaped_strings_and_payload_ref() {
        let mut record = EventLogRecord::dispatched("run\t1", sample_event());
        record.event_kind = "custom:\n7".to_string();
        record.payload_ref = Some("payload\\ref".to_string());
        let batch = EventLogBatch::new(vec![record]).expect("valid batch");

        let decoded = EventLogBatch::from_smoke_bytes(&batch.to_smoke_bytes()).expect("roundtrip");

        assert_eq!(decoded, batch);
    }

    #[test]
    fn validation_rejects_incompatible_schema_version() {
        let mut record = EventLogRecord::dispatched("run-1", sample_event());
        record.schema_version = 2;

        assert_eq!(
            EventLogBatch::new(vec![record]),
            Err(ArrowError::UnsupportedSchemaVersion(2))
        );
    }

    #[test]
    fn previous_facade_still_flushes_records() {
        let mut log = ArrowEventLog::new("test-1");
        log.record_event(ArrowEventLogEntry {
            event_id: "ev1",
            entity_id: Some("ent42"),
            time_ticks: 100,
            priority: 0,
            sequence: 1,
            kind: 5,
            status: "dispatched",
        });

        let json = log.flush_json();

        assert!(json.contains("kairo_ecs.event_log.v1"));
        assert!(json.contains(r#""event_count": 1"#));
        assert_eq!(log.flush_ndjson().lines().count(), 1);
    }
}
