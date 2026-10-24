from __future__ import annotations

import pytest

import kairo_ecs


def test_event_log_batch_v2_polars_roundtrip_zero_copy() -> None:
    pytest.importorskip("polars", exc_type=ModuleNotFoundError)
    pytest.importorskip("pyarrow", exc_type=ModuleNotFoundError)

    scheduler = kairo_ecs.Scheduler()
    scheduler.schedule_at(42, priority=-3, kind=5)
    outcome, event = scheduler.step()
    assert outcome is kairo_ecs.StepOutcome.DISPATCHED
    assert event is not None

    record = kairo_ecs.EventLogRecord.dispatched("run-v2", event)
    batch = kairo_ecs.EventLogBatch([record])

    # Task 2.2 requires native polars conversion without dict reconstruction
    # and Arrow C Data interface export
    assert hasattr(batch, "to_polars")
    df = batch.to_polars()
    assert df is not None
    assert df.height == 1


def test_arrow_c_capsule_schema_and_array() -> None:
    scheduler = kairo_ecs.Scheduler()
    scheduler.schedule_at(100, priority=0, kind=1)
    outcome, event = scheduler.step()
    assert outcome is kairo_ecs.StepOutcome.DISPATCHED

    record = kairo_ecs.EventLogRecord.dispatched("run-cap", event)
    batch = kairo_ecs.EventLogBatch([record])

    schema_cap = batch.__arrow_c_schema__()
    array_cap, _schema_cap2 = batch.__arrow_c_array__()
    assert schema_cap is not None
    assert array_cap is not None
