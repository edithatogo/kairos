#![forbid(unsafe_code)]

use std::error::Error;
use std::fmt::{Display, Formatter};

use kairo_ecs_types::{DispatchedEvent, EntityId, EventId, EventKind};

pub const SCHEMA_VERSION: u16 = 1;
pub const EVENT_LOG_STREAM: &str = "kairo_ecs.event_log.v1";
pub const TIME_SCALE_TICKS: &str = "ticks";

pub const EVENT_LOG_FIELDS: &[Field] = &[
    Field::new("schema_version", ArrowType::UInt16, false),
    Field::new("run_id", ArrowType::Utf8, false),
    Field::new("event_id", ArrowType::FixedSizeBinary(12), false),
    Field::new("entity_id", ArrowType::FixedSizeBinary(12), true),
    Field::new("time_ticks", ArrowType::FixedSizeBinary(16), false),
    Field::new("time_scale", ArrowType::Utf8, false),
    Field::new("priority", ArrowType::Int32, false),
    Field::new("sequence", ArrowType::UInt64, false),
    Field::new("event_kind", ArrowType::Utf8, false),
    Field::new("status", ArrowType::Utf8, false),
    Field::new("payload_ref", ArrowType::Utf8, true),
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Field {
    pub name: &'static str,
    pub data_type: ArrowType,
    pub nullable: bool,
}

impl Field {
    pub const fn new(name: &'static str, data_type: ArrowType, nullable: bool) -> Self {
        Self {
            name,
            data_type,
            nullable,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArrowType {
    Utf8,
    UInt16,
    UInt32,
    UInt64,
    Int32,
    FixedSizeBinary(u8),
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

    fn parse(value: &str) -> Result<Self, ArrowTelemetryError> {
        match value {
            "dispatched" => Ok(Self::Dispatched),
            "cancelled" => Ok(Self::Cancelled),
            "skipped" => Ok(Self::Skipped),
            "error" => Ok(Self::Error),
            _ => Err(ArrowTelemetryError::new(format!(
                "unknown event status: {value}"
            ))),
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
    pub fn dispatched(run_id: impl Into<String>, event: DispatchedEvent) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            run_id: run_id.into(),
            event_id: event.id,
            entity_id: event.entity,
            time_ticks: event.at.ticks(),
            time_scale: TIME_SCALE_TICKS.to_string(),
            priority: event.priority,
            sequence: event.sequence,
            event_kind: format_event_kind(&event.kind),
            status: EventStatus::Dispatched,
            payload_ref: None,
        }
    }

    pub fn validate(&self) -> Result<(), ArrowTelemetryError> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(ArrowTelemetryError::new(format!(
                "schema_version must be {SCHEMA_VERSION}, got {}",
                self.schema_version
            )));
        }
        if self.run_id.trim().is_empty() {
            return Err(ArrowTelemetryError::new("run_id must not be empty"));
        }
        if self.time_scale != TIME_SCALE_TICKS {
            return Err(ArrowTelemetryError::new(format!(
                "time_scale must be {TIME_SCALE_TICKS}, got {}",
                self.time_scale
            )));
        }
        if self.event_kind.trim().is_empty() {
            return Err(ArrowTelemetryError::new("event_kind must not be empty"));
        }
        if self
            .payload_ref
            .as_ref()
            .is_some_and(|payload_ref| payload_ref.trim().is_empty())
        {
            return Err(ArrowTelemetryError::new(
                "payload_ref must not be empty when present",
            ));
        }

        Ok(())
    }

    pub fn time_ticks_le_bytes(&self) -> [u8; 16] {
        self.time_ticks.to_le_bytes()
    }

    pub fn event_id_bytes(&self) -> [u8; 12] {
        event_id_bytes(self.event_id)
    }

    pub fn entity_id_bytes(&self) -> Option<[u8; 12]> {
        self.entity_id.map(entity_id_bytes)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventLogBatch {
    records: Vec<EventLogRecord>,
}

impl EventLogBatch {
    pub fn new(records: Vec<EventLogRecord>) -> Result<Self, ArrowTelemetryError> {
        for record in &records {
            record.validate()?;
        }
        Ok(Self { records })
    }

    pub fn records(&self) -> &[EventLogRecord] {
        &self.records
    }

    pub fn schema(&self) -> &'static [Field] {
        EVENT_LOG_FIELDS
    }

    pub fn to_smoke_bytes(&self) -> Vec<u8> {
        let mut output = format!("stream={EVENT_LOG_STREAM};schema_version={SCHEMA_VERSION}\n");
        output.push_str(
            "schema_version\trun_id\tevent_id_hex\tentity_id_hex\ttime_ticks_le_hex\ttime_scale\tpriority\tsequence\tevent_kind\tstatus\tpayload_ref\n",
        );

        for record in &self.records {
            let entity_id = record
                .entity_id_bytes()
                .map(|bytes| hex_bytes(&bytes))
                .unwrap_or_default();
            output.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                record.schema_version,
                escape_cell(&record.run_id),
                hex_bytes(&record.event_id_bytes()),
                entity_id,
                hex_bytes(&record.time_ticks_le_bytes()),
                escape_cell(&record.time_scale),
                record.priority,
                record.sequence,
                escape_cell(&record.event_kind),
                record.status.as_str(),
                escape_cell(record.payload_ref.as_deref().unwrap_or_default())
            ));
        }

        output.into_bytes()
    }

    pub fn from_smoke_bytes(bytes: &[u8]) -> Result<Self, ArrowTelemetryError> {
        let text = std::str::from_utf8(bytes)
            .map_err(|error| ArrowTelemetryError::new(format!("invalid utf8 payload: {error}")))?;
        let mut lines = text.lines();
        let header = lines
            .next()
            .ok_or_else(|| ArrowTelemetryError::new("missing stream header"))?;
        if header != format!("stream={EVENT_LOG_STREAM};schema_version={SCHEMA_VERSION}") {
            return Err(ArrowTelemetryError::new(format!(
                "unexpected stream header: {header}"
            )));
        }
        let fields = lines
            .next()
            .ok_or_else(|| ArrowTelemetryError::new("missing field header"))?;
        let expected_fields = "schema_version\trun_id\tevent_id_hex\tentity_id_hex\ttime_ticks_le_hex\ttime_scale\tpriority\tsequence\tevent_kind\tstatus\tpayload_ref";
        if fields != expected_fields {
            return Err(ArrowTelemetryError::new("unexpected field header"));
        }

        let mut records = Vec::new();
        for line in lines {
            let cells: Vec<&str> = line.split('\t').collect();
            if cells.len() != 11 {
                return Err(ArrowTelemetryError::new(format!(
                    "expected 11 cells, got {}",
                    cells.len()
                )));
            }
            let entity_id = match cells[3] {
                "" => None,
                value => Some(parse_entity_id(value)?),
            };

            records.push(EventLogRecord {
                schema_version: parse_cell(cells[0], "schema_version")?,
                run_id: unescape_cell(cells[1]),
                event_id: parse_event_id(cells[2])?,
                entity_id,
                time_ticks: parse_time_ticks(cells[4])?,
                time_scale: unescape_cell(cells[5]),
                priority: parse_cell(cells[6], "priority")?,
                sequence: parse_cell(cells[7], "sequence")?,
                event_kind: unescape_cell(cells[8]),
                status: EventStatus::parse(cells[9])?,
                payload_ref: match unescape_cell(cells[10]) {
                    value if value.is_empty() => None,
                    value => Some(value),
                },
            });
        }

        Self::new(records)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArrowTelemetryError {
    message: String,
}

impl ArrowTelemetryError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl Display for ArrowTelemetryError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for ArrowTelemetryError {}

pub fn format_event_kind(kind: &EventKind) -> String {
    match kind {
        EventKind::Custom(value) => format!("custom:{value}"),
    }
}

fn parse_cell<T>(cell: &str, field: &str) -> Result<T, ArrowTelemetryError>
where
    T: std::str::FromStr,
    T::Err: Display,
{
    cell.parse::<T>()
        .map_err(|error| ArrowTelemetryError::new(format!("invalid {field}: {error}")))
}

fn parse_time_ticks(hex: &str) -> Result<u128, ArrowTelemetryError> {
    let mut bytes = [0_u8; 16];
    if hex.len() != 32 {
        return Err(ArrowTelemetryError::new("time_ticks must be 16 bytes"));
    }

    for index in 0..16 {
        let offset = index * 2;
        bytes[index] = u8::from_str_radix(&hex[offset..offset + 2], 16)
            .map_err(|error| ArrowTelemetryError::new(format!("invalid time_ticks: {error}")))?;
    }

    Ok(u128::from_le_bytes(bytes))
}

fn parse_event_id(hex: &str) -> Result<EventId, ArrowTelemetryError> {
    let bytes = parse_handle_bytes(hex, "event_id")?;
    Ok(EventId {
        index: u64::from_le_bytes(bytes[0..8].try_into().expect("slice length")),
        generation: u32::from_le_bytes(bytes[8..12].try_into().expect("slice length")),
    })
}

fn parse_entity_id(hex: &str) -> Result<EntityId, ArrowTelemetryError> {
    let bytes = parse_handle_bytes(hex, "entity_id")?;
    Ok(EntityId {
        index: u64::from_le_bytes(bytes[0..8].try_into().expect("slice length")),
        generation: u32::from_le_bytes(bytes[8..12].try_into().expect("slice length")),
    })
}

fn parse_handle_bytes(hex: &str, field: &str) -> Result<[u8; 12], ArrowTelemetryError> {
    let mut bytes = [0_u8; 12];
    if hex.len() != 24 {
        return Err(ArrowTelemetryError::new(format!(
            "{field} must be 12 bytes"
        )));
    }

    for index in 0..12 {
        let offset = index * 2;
        bytes[index] = u8::from_str_radix(&hex[offset..offset + 2], 16)
            .map_err(|error| ArrowTelemetryError::new(format!("invalid {field}: {error}")))?;
    }

    Ok(bytes)
}

fn event_id_bytes(event_id: EventId) -> [u8; 12] {
    let mut bytes = [0_u8; 12];
    bytes[0..8].copy_from_slice(&event_id.index.to_le_bytes());
    bytes[8..12].copy_from_slice(&event_id.generation.to_le_bytes());
    bytes
}

fn entity_id_bytes(entity_id: EntityId) -> [u8; 12] {
    let mut bytes = [0_u8; 12];
    bytes[0..8].copy_from_slice(&entity_id.index.to_le_bytes());
    bytes[8..12].copy_from_slice(&entity_id.generation.to_le_bytes());
    bytes
}

fn hex_bytes(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn escape_cell(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\n', "\\n")
}

fn unescape_cell(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
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

#[cfg(test)]
mod tests {
    use kairo_ecs_types::{DispatchedEvent, EventId, SimTime};

    use super::*;

    fn dispatched_event() -> DispatchedEvent {
        DispatchedEvent {
            id: EventId {
                index: 7,
                generation: 1,
            },
            at: SimTime::from_ticks(42),
            priority: -3,
            sequence: 9,
            entity: Some(EntityId {
                index: 11,
                generation: 2,
            }),
            kind: EventKind::Custom(5),
        }
    }

    #[test]
    fn schema_exposes_versioned_event_log_fields() {
        let names: Vec<&str> = EVENT_LOG_FIELDS.iter().map(|field| field.name).collect();

        assert_eq!(EVENT_LOG_STREAM, "kairo_ecs.event_log.v1");
        assert_eq!(SCHEMA_VERSION, 1);
        assert_eq!(names.first(), Some(&"schema_version"));
        assert!(names.contains(&"time_ticks"));
        assert!(EVENT_LOG_FIELDS.iter().any(|field| {
            field.name == "time_ticks" && field.data_type == ArrowType::FixedSizeBinary(16)
        }));
    }

    #[test]
    fn dispatched_event_maps_to_event_log_record() {
        let record = EventLogRecord::dispatched("run-1", dispatched_event());

        assert_eq!(record.run_id, "run-1");
        assert_eq!(record.time_ticks, 42);
        assert_eq!(record.time_ticks_le_bytes(), 42_u128.to_le_bytes());
        assert_eq!(record.event_kind, "custom:5");
        assert_eq!(record.status, EventStatus::Dispatched);
        assert!(record.validate().is_ok());
    }

    #[test]
    fn event_log_batch_round_trips_smoke_bytes() {
        let record = EventLogRecord::dispatched("run-1", dispatched_event());
        let batch = EventLogBatch::new(vec![record]).expect("valid batch");

        let decoded = EventLogBatch::from_smoke_bytes(&batch.to_smoke_bytes()).expect("decode");

        assert_eq!(decoded, batch);
        assert_eq!(decoded.schema(), EVENT_LOG_FIELDS);
    }

    #[test]
    fn validation_rejects_blank_payload_ref() {
        let mut record = EventLogRecord::dispatched("run-1", dispatched_event());
        record.payload_ref = Some(" ".to_string());

        let error = record.validate().expect_err("blank payload_ref");

        assert_eq!(
            error.to_string(),
            "payload_ref must not be empty when present"
        );
    }
}
