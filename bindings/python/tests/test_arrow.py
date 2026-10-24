from __future__ import annotations

import pytest

import kairo_ecs


def test_event_log_batch_round_trips_smoke_bytes() -> None:
    scheduler = kairo_ecs.Scheduler()
    scheduler.schedule_at(42, priority=-3, kind=5)
    outcome, event = scheduler.step()
    assert outcome is kairo_ecs.StepOutcome.DISPATCHED
    assert event is not None

    record = kairo_ecs.EventLogRecord.dispatched("run-1", event)
    batch = kairo_ecs.EventLogBatch([record])

    decoded = kairo_ecs.EventLogBatch.from_smoke_bytes(batch.to_smoke_bytes())

    assert decoded == batch
    assert decoded.schema == kairo_ecs.EVENT_LOG_FIELDS
    assert decoded.records[0].event_kind == "custom:5"


def test_event_log_batch_round_trips_pyarrow_table() -> None:
    pytest.importorskip("pyarrow", exc_type=ModuleNotFoundError)

    scheduler = kairo_ecs.Scheduler()
    scheduler.schedule_at(42, priority=-3, kind=5)
    outcome, event = scheduler.step()
    assert outcome is kairo_ecs.StepOutcome.DISPATCHED
    assert event is not None

    batch = kairo_ecs.EventLogBatch([kairo_ecs.EventLogRecord.dispatched("run-1", event)])
    table = batch.to_pyarrow_table()

    assert table.schema.names == [field[0] for field in kairo_ecs.EVENT_LOG_FIELDS]
    assert kairo_ecs.EventLogBatch.from_pyarrow_table(table) == batch


def test_event_log_schema_matches_track_04_order() -> None:
    assert kairo_ecs.EVENT_LOG_STREAM == "kairo_ecs.event_log.v1"
    assert kairo_ecs.EVENT_LOG_FIELDS == (
        ("schema_version", "UInt16", False),
        ("run_id", "Utf8", False),
        ("event_id", "FixedSizeBinary(12)", False),
        ("entity_id", "FixedSizeBinary(12)", True),
        ("time_ticks", "FixedSizeBinary(16)", False),
        ("time_scale", "Utf8", False),
        ("priority", "Int32", False),
        ("sequence", "UInt64", False),
        ("event_kind", "Utf8", False),
        ("status", "Utf8", False),
        ("payload_ref", "Utf8", True),
    )
