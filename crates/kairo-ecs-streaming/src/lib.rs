#![forbid(unsafe_code)]

use std::error::Error;
use std::fmt::{Display, Formatter};
use std::thread;
use std::time::{Duration, Instant};

pub mod arrow_schema {
    pub const SCHEMA_VERSION: u16 = 1;
    pub const EVENT_LOG_STREAM: &str = "kairo_ecs.event_log.v1";
    pub const METRIC_SAMPLE_STREAM: &str = "kairo_ecs.metric_sample.v1";
    pub const ENTITY_SNAPSHOT_STREAM: &str = "kairo_ecs.entity_snapshot.v1";
    pub const RESOURCE_SNAPSHOT_STREAM: &str = "kairo_ecs.resource_snapshot.v1";
    pub const CONFORMANCE_RESULT_STREAM: &str = "kairo_ecs.conformance_result.v1";
    pub const TIME_SCALE_TICKS: &str = "ticks";

    pub const EVENT_LOG_FIELDS: &[&str] = &[
        "schema_version",
        "run_id",
        "event_id",
        "entity_id",
        "time_ticks",
        "time_scale",
        "priority",
        "sequence",
        "event_kind",
        "status",
        "payload_ref",
    ];

    pub fn is_known_stream(stream: &str) -> bool {
        matches!(
            stream,
            EVENT_LOG_STREAM
                | METRIC_SAMPLE_STREAM
                | ENTITY_SNAPSHOT_STREAM
                | RESOURCE_SNAPSHOT_STREAM
                | CONFORMANCE_RESULT_STREAM
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StreamMessage {
    pub stream: &'static str,
    pub schema_version: u16,
    pub run_id: String,
    pub event_id: Option<u64>,
    pub entity_id: Option<u64>,
    pub time_ticks: u128,
    pub time_scale: String,
    pub priority: i32,
    pub sequence: u64,
    pub event_kind: String,
    pub status: StreamStatus,
    pub payload_ref: Option<String>,
}

impl StreamMessage {
    pub fn event_log(run_id: impl Into<String>, time_ticks: u128, sequence: u64) -> Self {
        Self {
            stream: arrow_schema::EVENT_LOG_STREAM,
            schema_version: arrow_schema::SCHEMA_VERSION,
            run_id: run_id.into(),
            event_id: Some(sequence),
            entity_id: None,
            time_ticks,
            time_scale: arrow_schema::TIME_SCALE_TICKS.to_string(),
            priority: 0,
            sequence,
            event_kind: "custom".to_string(),
            status: StreamStatus::Dispatched,
            payload_ref: None,
        }
    }

    pub fn arrow_field_names() -> &'static [&'static str] {
        arrow_schema::EVENT_LOG_FIELDS
    }

    pub fn validate_event_log_contract(&self) -> Result<(), StreamError> {
        if self.stream != arrow_schema::EVENT_LOG_STREAM {
            return Err(StreamError::new(format!(
                "stream {} does not match {}",
                self.stream,
                arrow_schema::EVENT_LOG_STREAM
            )));
        }
        if self.schema_version != arrow_schema::SCHEMA_VERSION {
            return Err(StreamError::new(format!(
                "schema_version must be {}, got {}",
                arrow_schema::SCHEMA_VERSION,
                self.schema_version
            )));
        }
        if self.run_id.trim().is_empty() {
            return Err(StreamError::new("run_id must not be empty"));
        }
        if self.event_id.is_none() {
            return Err(StreamError::new("event_id must be present"));
        }
        if self.time_scale != arrow_schema::TIME_SCALE_TICKS {
            return Err(StreamError::new(format!(
                "time_scale must be {}, got {}",
                arrow_schema::TIME_SCALE_TICKS,
                self.time_scale
            )));
        }
        if self.event_kind.trim().is_empty() {
            return Err(StreamError::new("event_kind must not be empty"));
        }
        if self
            .payload_ref
            .as_ref()
            .is_some_and(|payload_ref| payload_ref.trim().is_empty())
        {
            return Err(StreamError::new(
                "payload_ref must not be empty when present",
            ));
        }

        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StreamStatus {
    Dispatched,
    Cancelled,
    Skipped,
    Error,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StreamError {
    message: String,
}

impl StreamError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl Display for StreamError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for StreamError {}

pub trait EventSource {
    fn next_event(&mut self) -> Result<Option<StreamMessage>, StreamError>;
}

pub trait EventSink {
    fn publish(&mut self, message: StreamMessage) -> Result<(), StreamError>;
}

pub trait SnapshotProvider {
    fn snapshot(&self, stream: &'static str) -> Result<Vec<StreamMessage>, StreamError>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PacingContract {
    tick_duration: Duration,
    tolerance: Duration,
}

impl PacingContract {
    pub fn new(tick_duration: Duration, tolerance: Duration) -> Result<Self, StreamError> {
        if tick_duration.is_zero() {
            return Err(StreamError::new("tick_duration must be non-zero"));
        }

        Ok(Self {
            tick_duration,
            tolerance,
        })
    }

    pub fn tick_duration(self) -> Duration {
        self.tick_duration
    }

    pub fn tolerance(self) -> Duration {
        self.tolerance
    }
}

#[derive(Debug)]
pub struct WallClockPacer {
    contract: PacingContract,
    started_at: Instant,
}

impl WallClockPacer {
    pub fn start(contract: PacingContract) -> Self {
        Self {
            contract,
            started_at: Instant::now(),
        }
    }

    pub fn wait_for_tick(&self, tick_index: u64) -> Duration {
        let target = self
            .contract
            .tick_duration
            .saturating_mul(tick_index as u32);
        let elapsed = self.started_at.elapsed();
        if elapsed < target {
            let remaining = target - elapsed;
            thread::sleep(remaining);
            remaining
        } else {
            Duration::ZERO
        }
    }

    pub fn drift_for_tick(&self, tick_index: u64) -> Duration {
        let target = self
            .contract
            .tick_duration
            .saturating_mul(tick_index as u32);
        let elapsed = self.started_at.elapsed();
        if elapsed >= target {
            elapsed - target
        } else {
            target - elapsed
        }
    }

    pub fn within_tolerance(&self, tick_index: u64) -> bool {
        self.drift_for_tick(tick_index) <= self.contract.tolerance
    }
}

pub mod adapters {
    use super::{EventSink, EventSource, SnapshotProvider, StreamError, StreamMessage};
    use std::collections::HashMap;

    #[derive(Debug, Default)]
    pub struct InMemoryStream {
        messages: Vec<StreamMessage>,
        read_index: usize,
        last_sequence_by_run: HashMap<String, u64>,
    }

    impl InMemoryStream {
        pub fn len(&self) -> usize {
            self.messages.len()
        }

        pub fn is_empty(&self) -> bool {
            self.messages.is_empty()
        }
    }

    impl EventSink for InMemoryStream {
        fn publish(&mut self, message: StreamMessage) -> Result<(), StreamError> {
            message.validate_event_log_contract()?;
            if let Some(last_sequence) = self.last_sequence_by_run.get(&message.run_id) {
                if message.sequence <= *last_sequence {
                    return Err(StreamError::new(format!(
                        "sequence must increase per run_id: last {}, got {}",
                        last_sequence, message.sequence
                    )));
                }
            }
            self.last_sequence_by_run
                .insert(message.run_id.clone(), message.sequence);
            self.messages.push(message);
            Ok(())
        }
    }

    impl EventSource for InMemoryStream {
        fn next_event(&mut self) -> Result<Option<StreamMessage>, StreamError> {
            let Some(message) = self.messages.get(self.read_index).cloned() else {
                return Ok(None);
            };
            self.read_index += 1;
            Ok(Some(message))
        }
    }

    impl SnapshotProvider for InMemoryStream {
        fn snapshot(&self, stream: &'static str) -> Result<Vec<StreamMessage>, StreamError> {
            if !super::arrow_schema::is_known_stream(stream) {
                return Err(StreamError::new(format!("unknown stream: {stream}")));
            }

            Ok(self
                .messages
                .iter()
                .filter(|message| message.stream == stream)
                .cloned()
                .collect())
        }
    }
}

#[cfg(feature = "kafka")]
pub mod kafka {
    pub type KafkaStream = crate::adapters::InMemoryStream;
}

#[cfg(feature = "nats")]
pub mod nats {
    pub type NatsStream = crate::adapters::InMemoryStream;
}

#[cfg(feature = "websocket")]
pub mod websocket {
    pub type WebSocketBridge = crate::adapters::InMemoryStream;
}

#[cfg(feature = "arrow-flight")]
pub mod arrow_flight {
    pub type ArrowFlightEndpoint = crate::adapters::InMemoryStream;
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::adapters::InMemoryStream;
    use super::*;

    #[test]
    fn event_log_message_exposes_arrow_contract_fields() {
        let message = StreamMessage::event_log("run-1", 42, 7);

        assert_eq!(message.stream, arrow_schema::EVENT_LOG_STREAM);
        assert_eq!(message.schema_version, arrow_schema::SCHEMA_VERSION);
        assert_eq!(
            StreamMessage::arrow_field_names(),
            [
                "schema_version",
                "run_id",
                "event_id",
                "entity_id",
                "time_ticks",
                "time_scale",
                "priority",
                "sequence",
                "event_kind",
                "status",
                "payload_ref"
            ]
        );
        assert!(message.validate_event_log_contract().is_ok());
    }

    #[test]
    fn in_memory_stream_round_trips_messages() {
        let mut stream = InMemoryStream::default();
        let message = StreamMessage::event_log("run-1", 1, 1);

        stream.publish(message.clone()).expect("publish");

        assert_eq!(stream.next_event().expect("read"), Some(message));
        assert_eq!(stream.next_event().expect("read empty"), None);
    }

    #[test]
    fn pacing_contract_rejects_zero_tick_duration() {
        let contract = PacingContract::new(Duration::ZERO, Duration::from_millis(1));

        assert!(contract.is_err());
    }

    #[test]
    fn event_log_contract_rejects_invalid_required_fields() {
        let mut message = StreamMessage::event_log("run-1", 42, 7);
        message.run_id.clear();

        let error = message
            .validate_event_log_contract()
            .expect_err("blank run_id should fail");

        assert_eq!(error.to_string(), "run_id must not be empty");
    }

    #[test]
    fn event_log_contract_rejects_missing_event_id() {
        let mut message = StreamMessage::event_log("run-1", 42, 7);
        message.event_id = None;

        let error = message
            .validate_event_log_contract()
            .expect_err("missing event_id should fail");

        assert_eq!(error.to_string(), "event_id must be present");
    }

    #[test]
    fn event_log_contract_rejects_wrong_schema_version() {
        let mut message = StreamMessage::event_log("run-1", 42, 7);
        message.schema_version = 2;

        let error = message
            .validate_event_log_contract()
            .expect_err("wrong schema_version should fail");

        assert_eq!(error.to_string(), "schema_version must be 1, got 2");
    }

    #[test]
    fn in_memory_sink_rejects_invalid_contract_message() {
        let mut stream = InMemoryStream::default();
        let mut message = StreamMessage::event_log("run-1", 42, 7);
        message.payload_ref = Some(" ".to_string());

        let error = stream
            .publish(message)
            .expect_err("blank payload_ref should fail");

        assert_eq!(
            error.to_string(),
            "payload_ref must not be empty when present"
        );
        assert!(stream.is_empty());
    }

    #[test]
    fn in_memory_sink_rejects_sequence_regression_for_same_run() {
        let mut stream = InMemoryStream::default();

        stream
            .publish(StreamMessage::event_log("run-1", 42, 2))
            .expect("first publish");
        let error = stream
            .publish(StreamMessage::event_log("run-1", 43, 2))
            .expect_err("duplicate sequence should fail");

        assert_eq!(
            error.to_string(),
            "sequence must increase per run_id: last 2, got 2"
        );
        assert_eq!(stream.len(), 1);
    }

    #[test]
    fn snapshot_rejects_unknown_stream_name() {
        let stream = InMemoryStream::default();

        let error = stream
            .snapshot("kairo_ecs.unknown.v1")
            .expect_err("unknown stream should fail");

        assert_eq!(error.to_string(), "unknown stream: kairo_ecs.unknown.v1");
    }
}
