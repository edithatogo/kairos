"""Wave 9 tests for Arrow C Data Interface, native bridge, and vecenv / PettingZoo adapters."""

from kairo_ecs import (
    EventId,
    EventLogBatch,
    EventLogRecord,
    EventStatus,
    NativeSchedulerBridge,
    ParallelVecEnv,
    PettingZooParallelAdapter,
    native_extension_status,
)


def test_arrow_c_data_interface_and_duckdb_polars_spec() -> None:
    record = EventLogRecord(
        schema_version=2,
        run_id="run-wave9",
        event_id=EventId(1, 0),
        entity_id=None,
        time_ticks=10,
        time_scale="ticks",
        priority=0,
        sequence=1,
        event_kind="custom:test",
        status=EventStatus.DISPATCHED,
    )
    batch = EventLogBatch([record])
    schema_capsule = batch.__arrow_c_schema__()
    assert type(schema_capsule).__name__ == "PyCapsule"

    arr_schema_capsule, array_capsule = batch.__arrow_c_array__()
    assert type(arr_schema_capsule).__name__ == "PyCapsule"
    assert type(array_capsule).__name__ == "PyCapsule"

    polars_dict = batch.to_polars_dict()
    assert polars_dict["run_id"] == ["run-wave9"]

    duckdb_spec = batch.to_duckdb_relation_spec()
    assert duckdb_spec["table_name"] == "kairo_ecs_event_log"
    assert duckdb_spec["row_count"] == 1



def test_native_bridge_does_not_return_a_fabricated_version() -> None:
    import pytest
    from kairo_ecs import NativeBackendUnavailable

    with pytest.raises(NativeBackendUnavailable):
        NativeSchedulerBridge("nonexistent.dylib")
    status = native_extension_status()
    assert status["compiled"] is False
    assert status["fallback_supported"] is False


def test_parallel_vecenv_requires_rust_model() -> None:
    import pytest
    from kairo_ecs import UnsupportedNativeOperation

    with pytest.raises(UnsupportedNativeOperation):
        ParallelVecEnv(num_envs=2)
    with pytest.raises(UnsupportedNativeOperation):
        PettingZooParallelAdapter(["p0", "p1"])
