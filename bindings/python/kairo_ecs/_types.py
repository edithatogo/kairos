"""Dependency-light public value contracts for the Python binding."""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum
from typing import ClassVar

MAX_U128 = (1 << 128) - 1
MAX_U64 = (1 << 64) - 1
MAX_U32 = (1 << 32) - 1


@dataclass(frozen=True, order=True)
class SimTime:
    """Fixed-tick simulation time."""

    ZERO: ClassVar["SimTime"]

    ticks: int = 0

    def __post_init__(self) -> None:
        if not 0 <= self.ticks <= MAX_U128:
            raise ValueError("ticks must fit in an unsigned 128-bit integer")

    @classmethod
    def from_ticks(cls, ticks: int) -> "SimTime":
        return cls(ticks)


@dataclass(frozen=True)
class EventId:
    """Generational event handle."""

    index: int
    generation: int

    def __post_init__(self) -> None:
        _validate_u64(self.index, "index")
        _validate_u32(self.generation, "generation")


@dataclass(frozen=True)
class EntityId:
    """Generational entity handle."""

    index: int
    generation: int

    def __post_init__(self) -> None:
        _validate_u64(self.index, "index")
        _validate_u32(self.generation, "generation")


@dataclass(frozen=True)
class ScheduleRequest:
    """Scheduler input."""

    at: SimTime
    priority: int
    kind: int
    entity: EntityId | None = None

    def __post_init__(self) -> None:
        if not isinstance(self.at, SimTime):
            object.__setattr__(self, "at", SimTime.from_ticks(int(self.at)))
        if not -(1 << 31) <= self.priority <= (1 << 31) - 1:
            raise ValueError("priority must fit in a signed 32-bit integer")
        _validate_u32(self.kind, "kind")


@dataclass(frozen=True)
class DispatchedEvent:
    """Event emitted by the scheduler."""

    id: EventId
    at: SimTime
    priority: int
    sequence: int
    kind: int
    entity: EntityId | None = None


class StepOutcome(str, Enum):
    """Result of advancing the scheduler."""

    DISPATCHED = "dispatched"
    EMPTY = "empty"
    LIMIT_REACHED = "limit_reached"


SimTime.ZERO = SimTime.from_ticks(0)


def _validate_u64(value: int, name: str) -> None:
    if not 0 <= value <= MAX_U64:
        raise ValueError(f"{name} must fit in an unsigned 64-bit integer")


def _validate_u32(value: int, name: str) -> None:
    if not 0 <= value <= MAX_U32:
        raise ValueError(f"{name} must fit in an unsigned 32-bit integer")
