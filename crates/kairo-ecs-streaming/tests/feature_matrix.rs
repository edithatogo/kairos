use std::time::Duration;

use kairo_ecs_streaming::adapters::InMemoryStream;
use kairo_ecs_streaming::{
    EventSink, EventSource, PacingContract, SnapshotProvider, StreamMessage, WallClockPacer,
};

#[test]
fn wall_clock_pacing_starts_at_tick_zero() {
    let tick_duration = Duration::from_millis(10);
    let tolerance = Duration::from_millis(2);
    let contract = PacingContract::new(tick_duration, tolerance).expect("valid pacing contract");

    assert_eq!(contract.tick_duration(), tick_duration);
    assert_eq!(contract.tolerance(), tolerance);
    let pacer = WallClockPacer::start(contract);
    assert_eq!(pacer.wait_for_tick(0), Duration::ZERO);
}

#[test]
fn in_memory_stream_round_trips_the_full_message() {
    let mut stream = InMemoryStream::default();
    let message = StreamMessage::event_log("featureless", 10, 1);
    stream.publish(message.clone()).expect("publish");

    assert_eq!(stream.next_event().expect("next"), Some(message));
    assert!(stream.next_event().expect("empty stream").is_none());
}

#[test]
fn snapshot_provider_filters_by_arrow_stream_name() {
    let mut stream = InMemoryStream::default();
    stream
        .publish(StreamMessage::event_log("featureless", 10, 1))
        .expect("publish");

    let snapshot = stream
        .snapshot(kairo_ecs_streaming::arrow_schema::EVENT_LOG_STREAM)
        .expect("snapshot");

    assert_eq!(snapshot.len(), 1);
    assert_eq!(
        snapshot[0].schema_version,
        kairo_ecs_streaming::arrow_schema::SCHEMA_VERSION
    );
}

#[test]
fn in_memory_adapter_enforces_event_log_contract() {
    let mut stream = InMemoryStream::default();
    let mut message = StreamMessage::event_log("featureless", 10, 1);
    message.event_kind = " ".to_string();

    let error = stream
        .publish(message)
        .expect_err("blank event kind should be rejected");

    assert_eq!(error.to_string(), "event_kind must not be empty");
    assert!(stream.is_empty());
}

#[test]
fn in_memory_adapter_rejects_unversioned_event_log_contract() {
    let mut stream = InMemoryStream::default();
    let mut message = StreamMessage::event_log("featureless", 10, 1);
    message.schema_version = 0;

    let error = stream
        .publish(message)
        .expect_err("wrong schema version should be rejected");

    assert_eq!(error.to_string(), "schema_version must be 1, got 0");
    assert!(stream.is_empty());
}

#[test]
fn in_memory_adapter_rejects_sequence_regression_per_run() {
    let mut stream = InMemoryStream::default();
    stream
        .publish(StreamMessage::event_log("featureless", 10, 2))
        .expect("publish first message");

    let error = stream
        .publish(StreamMessage::event_log("featureless", 11, 1))
        .expect_err("lower sequence should be rejected");

    assert_eq!(
        error.to_string(),
        "sequence must increase per run_id: last 2, got 1"
    );
    assert_eq!(stream.len(), 1);
}

#[test]
fn in_memory_adapter_rejects_time_regression_per_run() {
    let mut stream = InMemoryStream::default();
    stream
        .publish(StreamMessage::event_log("featureless", 10, 1))
        .expect("publish first message");

    let error = stream
        .publish(StreamMessage::event_log("featureless", 9, 2))
        .expect_err("lower tick should be rejected");

    assert_eq!(
        error.to_string(),
        "time_ticks must not decrease per run_id: last 10, got 9"
    );
    assert_eq!(stream.len(), 1);
}

#[test]
fn snapshot_provider_rejects_unknown_stream_name() {
    let stream = InMemoryStream::default();

    let error = stream
        .snapshot("kairo_ecs.unknown.v1")
        .expect_err("unknown stream should be rejected");

    assert_eq!(error.to_string(), "unknown stream: kairo_ecs.unknown.v1");
}
