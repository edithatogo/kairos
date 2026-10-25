"""Track 70 Phase 2 Task 2.2 — real consumer acceptance tests for the native
handoff.

These were strict ``xfail`` characterization probes under Task 2.1 (baseline
at ``2ba9fb9``: v1 ``FixedSizeBinary`` schema, fabricated capsule fallbacks,
``to_pylist()`` Python-row reconstruction). Task 2.2 implements owned
populated Arrow C Data export and the explicit v2 native schema for the
Python consumer, so each fixture is now a behavioural acceptance test:

- ``kairo_ecs._arrow`` declares schema v2 (generational-ID and tick-limb
  columns; strings exported as LargeUtf8 for independent-reader portability).
- ``EventLogBatch.__arrow_c_array__`` exports real populated v2 capsules and
  raises explicitly when the native export is unavailable — the
  fabricated-capsule fallback is removed.
- ``EventLogBatch.from_pyarrow_table`` reads columns first (one pull per
  column) with the null-together generational-ID contract.

pyarrow acts as the independent reader per the Track 70 test matrix row
``independent-columnar-interop``.
"""

from __future__ import annotations

from typing import Any

import kairo_ecs
import kairo_ecs._arrow
import pytest

V2_EVENT_LOG_FIELDS = (
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


def _dispatched_batch(run_id: str, records: int) -> kairo_ecs.EventLogBatch:
    """Build a deterministic batch of `records` dispatched custom events."""

    scheduler = kairo_ecs.Scheduler()
    batch_records = []
    for tick in range(1, records + 1):
        scheduler.schedule_at(tick * 10, priority=-3, kind=5)
        outcome, event = scheduler.step()
        assert outcome is kairo_ecs.StepOutcome.DISPATCHED
        assert event is not None
        batch_records.append(kairo_ecs.EventLogRecord.dispatched(run_id, event))
    return kairo_ecs.EventLogBatch(batch_records)


def test_arrow_c_interface_exposes_v2_native_schema_fields() -> None:
    pytest.importorskip("pyarrow", exc_type=ModuleNotFoundError)

    import pyarrow as pa

    batch = _dispatched_batch("run-70-p2-schema", 1)
    schema_capsule, array_capsule = batch.__arrow_c_array__()
    record_batch = pa.RecordBatch._import_from_c_capsule(schema_capsule, array_capsule)

    assert record_batch.schema.names == [field[0] for field in V2_EVENT_LOG_FIELDS]
    assert record_batch.num_rows == 1


def test_populated_export_respects_null_together_generational_ids() -> None:
    pytest.importorskip("pyarrow", exc_type=ModuleNotFoundError)

    import dataclasses

    import pyarrow as pa

    batch = _dispatched_batch("run-70-p2-nulls", 2)
    entity_record = dataclasses.replace(
        batch.records[1], entity_id=kairo_ecs.EntityId(42, 1)
    )
    batch = kairo_ecs.EventLogBatch([batch.records[0], entity_record])

    schema_capsule, array_capsule = batch.__arrow_c_array__()
    record_batch = pa.RecordBatch._import_from_c_capsule(schema_capsule, array_capsule)

    entity_index = record_batch.column("entity_id_index")
    entity_gen = record_batch.column("entity_id_gen")
    # spec.md: generational IDs are "both null together for nullable IDs" —
    # exactly one entity-less row, so both limbs are null in the same row and
    # both are populated (index and generation) for the entity row.
    assert entity_index.null_count == 1
    assert entity_gen.null_count == 1


def test_empty_event_log_export_stays_schema_valid() -> None:
    pytest.importorskip("pyarrow", exc_type=ModuleNotFoundError)

    import pyarrow as pa

    batch = kairo_ecs.EventLogBatch([])
    schema_capsule, array_capsule = batch.__arrow_c_array__()
    record_batch = pa.RecordBatch._import_from_c_capsule(schema_capsule, array_capsule)

    assert record_batch.num_rows == 0
    assert record_batch.schema.names == [field[0] for field in V2_EVENT_LOG_FIELDS]


def test_failed_native_export_raises_instead_of_fabricating_capsules(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    pytest.importorskip("pyarrow", exc_type=ModuleNotFoundError)

    def _raise(self: Any) -> Any:
        raise RuntimeError("native export unavailable")

    monkeypatch.setattr(kairo_ecs._arrow.EventLogBatch, "to_pyarrow_table", _raise)
    batch = _dispatched_batch("run-70-p2-explicit", 1)

    # The spec forbids the fabricated capsule fallback: an unavailable native
    # export must fail explicitly, never hand the consumer an empty `+s`
    # capsule pair with no columns.
    with pytest.raises(RuntimeError, match="native export unavailable"):
        batch.__arrow_c_array__()
