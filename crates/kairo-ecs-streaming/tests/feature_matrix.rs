use kairo_ecs_streaming::adapters::InMemoryStream;
use kairo_ecs_streaming::{EventSink, EventSource, SnapshotProvider, StreamMessage};

#[test]
fn base_featureless_stream_contract_round_trips() {
    let mut stream = InMemoryStream::default();
    stream
        .publish(StreamMessage::event_log("featureless", 10, 1))
        .expect("publish");

    assert!(stream.next_event().expect("next").is_some());
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

#[cfg(feature = "kafka")]
#[test]
fn kafka_feature_exposes_adapter_type() {
    let stream = kairo_ecs_streaming::kafka::KafkaStream::default();
    assert!(stream.is_empty());
}

#[cfg(feature = "nats")]
#[test]
fn nats_feature_exposes_adapter_type() {
    let stream = kairo_ecs_streaming::nats::NatsStream::default();
    assert!(stream.is_empty());
}

#[cfg(feature = "websocket")]
#[test]
fn websocket_feature_exposes_adapter_type() {
    let stream = kairo_ecs_streaming::websocket::WebSocketBridge::default();
    assert!(stream.is_empty());
}

#[cfg(feature = "arrow-flight")]
#[test]
fn arrow_flight_feature_exposes_endpoint_type() {
    let stream = kairo_ecs_streaming::arrow_flight::ArrowFlightEndpoint::default();
    assert!(stream.is_empty());
}
