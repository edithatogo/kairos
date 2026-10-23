"""Track 70 Phase 2 Task 2.1 — failing consumer fixtures for native handoff.

Following the Phase 0/1 characterization convention (`#[should_panic]` probes
with explicit markers in the Rust suite), these fixtures are strict ``xfail``
while Task 2.2 has not implemented owned populated Arrow C Data export and the
v2 native schema for the Python consumer. Baseline at ``2ba9fb9``:

- ``kairo_ecs._arrow`` still declares schema v1 (``FixedSizeBinary`` handles,
  single ``time_ticks`` fixed 16-byte field) instead of the v2 generational-ID
  and tick-limb columns.
- ``EventLogBatch.__arrow_c_array__`` falls back to fabricated capsules
  (``_allocate_arrow_schema_capsule``/``_allocate_arrow_array_capsule``,
  ``+s`` struct with ``n_children == 0`` and a single null buffer pointer)
  whenever the pyarrow path is unavailable or raises.
- ``EventLogBatch.from_pyarrow_table`` reconstructs rows via
  ``table.to_pylist()`` — the Python-row reconstruction detour the spec
  forbids.

When Task 2.2 lands, each fixture flips to ``XPASS`` and must be converted
into a real behavioural acceptance test. pyarrow acts as the independent
reader per the Track 70 test matrix row ``independent-columnar-interop``.
"""

from __future__ import annotations

from typing import Any

import pytest

import kairo_ecs
import kairo_ecs._arrow

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

_XFAIL = pytest.mark.xfail(
    reason=(
        "Track 70 Phase 2 pending (Task 2.2): Python still receives v1 "
        "descriptors/fabricated capsule fallbacks instead of owned populated "
        "v2 Arrow C Data tables"
    ),
    strict=True,
)


def _dispatched_batch(run_id: str, records: int) -> "kairo_ecs.EventLogBatch":
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


@_XFAIL
def test_arrow_c_interface_exposes_v2_native_schema_fields() -> None:
    pytest.importorskip("pyarrow", exc_type=ModuleNotFoundError)

    import pyarrow as pa

    batch = _dispatched_batch("run-70-p2-schema", 1)
    schema_capsule, array_capsule = batch.__arrow_c_array__()
    record_batch = pa.RecordBatch._import_from_c_capsule(schema_capsule, array_capsule)

    assert record_batch.schema.names == [field[0] for field in V2_EVENT_LOG_FIELDS]
    assert record_batch.num_rows == 1


@_XFAIL
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


@_XFAIL
def test_empty_event_log_export_stays_schema_valid() -> None:
    pytest.importorskip("pyarrow", exc_type=ModuleNotFoundError)

    import pyarrow as pa

    batch = kairo_ecs.EventLogBatch([])
    schema_capsule, array_capsule = batch.__arrow_c_array__()
    record_batch = pa.RecordBatch._import_from_c_capsule(schema_capsule, array_capsule)

    assert record_batch.num_rows == 0
    assert record_batch.schema.names == [field[0] for field in V2_EVENT_LOG_FIELDS]


@_XFAIL
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
