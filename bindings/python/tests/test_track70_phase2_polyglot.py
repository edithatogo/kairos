from __future__ import annotations

import kairo_ecs
import pytest


def test_event_log_batch_v2_polars_roundtrip_zero_copy() -> None:
    pytest.importorskip("polars", exc_type=ModuleNotFoundError)
    pytest.importorskip("pyarrow", exc_type=ModuleNotFoundError)

    # Static input record for data-conversion testing, not engine execution.
    event = kairo_ecs.DispatchedEvent(
        id=kairo_ecs.EventId(0, 0),
        at=kairo_ecs.SimTime.from_ticks(42), priority=-3,
        sequence=0, kind=5,
    )

    record = kairo_ecs.EventLogRecord.dispatched("run-v2", event)
    batch = kairo_ecs.EventLogBatch([record])

    # Task 2.2 requires native polars conversion without dict reconstruction
    # and Arrow C Data interface export
    assert hasattr(batch, "to_polars")
    df = batch.to_polars()
    assert df is not None
    assert df.height == 1


def test_arrow_c_capsule_schema_and_array() -> None:
    # Static input record for data-conversion testing, not engine execution.
    event = kairo_ecs.DispatchedEvent(
        id=kairo_ecs.EventId(0, 0),
        at=kairo_ecs.SimTime.from_ticks(100), priority=0,
        sequence=0, kind=1,
    )

    record = kairo_ecs.EventLogRecord.dispatched("run-cap", event)
    batch = kairo_ecs.EventLogBatch([record])

    schema_cap = batch.__arrow_c_schema__()
    array_cap, _schema_cap2 = batch.__arrow_c_array__()
    assert schema_cap is not None
    assert array_cap is not None
