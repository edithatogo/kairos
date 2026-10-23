"""Public Python package surface for KairoECS."""

from ._arrow import (
    EVENT_LOG_FIELDS,
    EVENT_LOG_STREAM,
    SCHEMA_VERSION,
    EventLogBatch,
    EventLogRecord,
    EventStatus,
)
from ._ffi import ffi_status
from ._native import NativeSchedulerBridge, native_extension_status
from ._scheduler import Scheduler
from ._types import (
    DispatchedEvent,
    EntityId,
    EventId,
    ScheduleRequest,
    SimTime,
    StepOutcome,
)
from .vecenv import ParallelVecEnv, PettingZooParallelAdapter

__all__ = [
    "EVENT_LOG_FIELDS",
    "EVENT_LOG_STREAM",
    "SCHEMA_VERSION",
    "DispatchedEvent",
    "EntityId",
    "EventId",
    "EventLogBatch",
    "EventLogRecord",
    "EventStatus",
    "NativeSchedulerBridge",
    "ParallelVecEnv",
    "PettingZooParallelAdapter",
    "ScheduleRequest",
    "Scheduler",
    "SimTime",
    "StepOutcome",
    "__version__",
    "ffi_status",
    "native_extension_status",
    "self_check",
]

__version__ = "0.1.0"


def self_check() -> dict[str, object]:
    """Return a small import-time health payload for smoke tests."""

    return {
        "package": "kairo_ecs",
        "version": __version__,
        "status": "ok",
        "ffi": ffi_status(),
    }
