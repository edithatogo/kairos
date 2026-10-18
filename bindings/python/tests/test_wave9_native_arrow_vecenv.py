"""Wave 9 tests for Arrow C Data Interface, native extension bridge, and vecenv / PettingZoo adapters."""

import pytest
from kairo_ecs import EventLogBatch, EventLogRecord, EventStatus, EventId
from kairo_ecs._native import NativeSchedulerBridge, native_extension_status
from kairo_ecs.vecenv import ParallelVecEnv, PettingZooParallelAdapter

def test_arrow_c_data_interface_and_duckdb_polars_spec():
    record = EventLogRecord(
        schema_version=1,
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
    assert schema_capsule["name"] == "event_log_batch"
    
    arr_capsule, array_capsule = batch.__arrow_c_array__()
    assert array_capsule["length"] == 1
    
    polars_dict = batch.to_polars_dict()
    assert polars_dict["run_id"] == ["run-wave9"]
    
    duckdb_spec = batch.to_duckdb_relation_spec()
    assert duckdb_spec["table_name"] == "kairo_ecs_event_log"
    assert duckdb_spec["row_count"] == 1

def test_native_scheduler_bridge_and_status():
    status = native_extension_status()
    assert "native_library" in status
    assert "compiled" in status
    
    bridge = NativeSchedulerBridge("nonexistent.dylib")
    assert not bridge.available
    assert bridge.get_version() == 1

def test_parallel_vecenv_conformance():
    env = ParallelVecEnv(num_envs=2)
    obs, infos = env.reset()
    assert len(obs) == 2
    assert len(infos) == 2
    
    actions = [0, 1]
    obs, rewards, terminated, truncated, infos = env.step(actions)
    assert len(rewards) == 2
    assert not any(terminated)
    env.close()

def test_pettingzoo_parallel_adapter_conformance():
    env = PettingZooParallelAdapter(["p0", "p1"])
    assert len(env.possible_agents) == 2
    
    obs, infos = env.reset()
    assert "p0" in obs
    
    action_dict = {"p0": 0, "p1": 1}
    obs, rewards, terminations, truncations, infos = env.step(action_dict)
    assert "p0" in rewards
    assert not terminations["p0"]
