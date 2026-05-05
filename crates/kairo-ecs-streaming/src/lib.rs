#![forbid(unsafe_code)]

use std::error::Error;
use std::fmt::{Display, Formatter};
use std::thread;
use std::time::{Duration, Instant};

pub mod arrow_schema {
    pub const EVENT_LOG_STREAM: &str = "kairo_ecs.event_log.v1";
    pub const METRIC_SAMPLE_STREAM: &str = "kairo_ecs.metric_sample.v1";
    pub const ENTITY_SNAPSHOT_STREAM: &str = "kairo_ecs.entity_snapshot.v1";
    pub const RESOURCE_SNAPSHOT_STREAM: &str = "kairo_ecs.resource_snapshot.v1";
    pub const CONFORMANCE_RESULT_STREAM: &str = "kairo_ecs.conformance_result.v1";

    pub const EVENT_LOG_FIELDS: &[&str] = &[
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
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StreamMessage {
    pub stream: &'static str,
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
            run_id: run_id.into(),
            event_id: None,
            entity_id: None,
            time_ticks,
            time_scale: "ticks".to_string(),
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
}

#[derive(Clone, Debug, Eq, PartialEq)]
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
        self.started_at.elapsed().abs_diff(target)
    }

    pub fn within_tolerance(&self, tick_index: u64) -> bool {
        self.drift_for_tick(tick_index) <= self.contract.tolerance
    }
}

pub mod adapters {
    use super::{EventSink, EventSource, SnapshotProvider, StreamError, StreamMessage};

    #[derive(Debug, Default)]
    pub struct InMemoryStream {
        messages: Vec<StreamMessage>,
        read_index: usize,
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
        assert_eq!(
            StreamMessage::arrow_field_names(),
            [
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
}
