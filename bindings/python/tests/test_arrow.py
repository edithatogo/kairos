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
    # Schema v2 (Track 70 Phase 2): generational IDs as index/generation
    # columns and full-range ticks as hi/lo limbs; strings export as LargeUtf8
    # for independent-reader portability.
    assert kairo_ecs.EVENT_LOG_STREAM == "kairo_ecs.event_log.v2"
    assert kairo_ecs.EVENT_LOG_FIELDS == (
        ("schema_version", "UInt16", False),
        ("run_id", "Utf8", False),
        ("event_id_index", "UInt64", False),
        ("event_id_gen", "UInt32", False),
        ("entity_id_index", "UInt64", True),
        ("entity_id_gen", "UInt32", True),
        ("time_ticks_hi", "UInt64", False),
        ("time_ticks_lo", "UInt64", False),
        ("time_scale", "Utf8", False),
        ("priority", "Int32", False),
        ("sequence", "UInt64", False),
        ("event_kind", "Utf8", False),
        ("status", "Utf8", False),
        ("payload_ref", "Utf8", True),
    )


def test_event_log_batch_round_trips_generation_entities_and_tick_limbs() -> None:
    pytest.importorskip("pyarrow", exc_type=ModuleNotFoundError)

    scheduler = kairo_ecs.Scheduler()
    scheduler.schedule_at(42, priority=-3, kind=5)
    outcome, event = scheduler.step()
    assert outcome is kairo_ecs.StepOutcome.DISPATCHED
    assert event is not None

    record = kairo_ecs.EventLogRecord.dispatched("run-1", event)
    record = kairo_ecs.EventLogRecord(
        schema_version=kairo_ecs.SCHEMA_VERSION,
        run_id=record.run_id,
        event_id=record.event_id,
        entity_id=kairo_ecs.EntityId(7, 2),
        time_ticks=(1 << 64) + 42,
        time_scale=record.time_scale,
        priority=record.priority,
        sequence=record.sequence,
        event_kind=record.event_kind,
        status=record.status,
    )
    batch = kairo_ecs.EventLogBatch([record])
    table = batch.to_pyarrow_table()

    # Max-range ticks must split into hi/lo limbs without losing the top bit.
    assert table.column("time_ticks_hi").to_pylist() == [1]
    assert table.column("time_ticks_lo").to_pylist() == [42]
    assert table.column("entity_id_index").to_pylist() == [7]
    assert table.column("entity_id_gen").to_pylist() == [2]

    assert kairo_ecs.EventLogBatch.from_pyarrow_table(table) == batch
