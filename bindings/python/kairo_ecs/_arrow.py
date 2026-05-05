"""Event-log v1 smoke roundtrip helpers for the Python binding."""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum

from ._types import DispatchedEvent, EntityId, EventId

SCHEMA_VERSION = 1
EVENT_LOG_STREAM = "kairo_ecs.event_log.v1"
TIME_SCALE_TICKS = "ticks"

EVENT_LOG_FIELDS = (
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


class EventStatus(str, Enum):
    DISPATCHED = "dispatched"
    CANCELLED = "cancelled"
    SKIPPED = "skipped"
    ERROR = "error"


@dataclass(frozen=True)
class EventLogRecord:
    schema_version: int
    run_id: str
    event_id: EventId
    entity_id: EntityId | None
    time_ticks: int
    time_scale: str
    priority: int
    sequence: int
    event_kind: str
    status: EventStatus
    payload_ref: str | None = None

    @classmethod
    def dispatched(cls, run_id: str, event: DispatchedEvent) -> "EventLogRecord":
        return cls(
            schema_version=SCHEMA_VERSION,
            run_id=run_id,
            event_id=event.id,
            entity_id=event.entity,
            time_ticks=event.at.ticks,
            time_scale=TIME_SCALE_TICKS,
            priority=event.priority,
            sequence=event.sequence,
            event_kind=f"custom:{event.kind}",
            status=EventStatus.DISPATCHED,
        )

    def validate(self) -> None:
        if self.schema_version != SCHEMA_VERSION:
            raise ValueError(f"schema_version must be {SCHEMA_VERSION}")
        if not self.run_id.strip():
            raise ValueError("run_id must not be empty")
        if self.time_scale != TIME_SCALE_TICKS:
            raise ValueError(f"time_scale must be {TIME_SCALE_TICKS}")
        if not self.event_kind.strip():
            raise ValueError("event_kind must not be empty")
        if self.payload_ref is not None and not self.payload_ref.strip():
            raise ValueError("payload_ref must not be empty when present")


@dataclass(frozen=True)
class EventLogBatch:
    records: tuple[EventLogRecord, ...]

    def __init__(self, records: list[EventLogRecord] | tuple[EventLogRecord, ...]) -> None:
        for record in records:
            record.validate()
        object.__setattr__(self, "records", tuple(records))

    @property
    def schema(self) -> tuple[tuple[str, str, bool], ...]:
        return EVENT_LOG_FIELDS

    def to_smoke_bytes(self) -> bytes:
        lines = [
            f"stream={EVENT_LOG_STREAM};schema_version={SCHEMA_VERSION}",
            "schema_version\trun_id\tevent_id_hex\tentity_id_hex\t"
            "time_ticks_le_hex\ttime_scale\tpriority\tsequence\t"
            "event_kind\tstatus\tpayload_ref",
        ]
        for record in self.records:
            entity_hex = _handle_hex(record.entity_id) if record.entity_id else ""
            lines.append(
                "\t".join(
                    [
                        str(record.schema_version),
                        _escape_cell(record.run_id),
                        _handle_hex(record.event_id),
                        entity_hex,
                        record.time_ticks.to_bytes(16, "little").hex(),
                        _escape_cell(record.time_scale),
                        str(record.priority),
                        str(record.sequence),
                        _escape_cell(record.event_kind),
                        record.status.value,
                        _escape_cell(record.payload_ref or ""),
                    ]
                )
            )
        return ("\n".join(lines) + "\n").encode("utf-8")

    @classmethod
    def from_smoke_bytes(cls, payload: bytes) -> "EventLogBatch":
        lines = payload.decode("utf-8").splitlines()
        expected_header = f"stream={EVENT_LOG_STREAM};schema_version={SCHEMA_VERSION}"
        if len(lines) < 2 or lines[0] != expected_header:
            raise ValueError("unexpected stream header")
        expected_fields = (
            "schema_version\trun_id\tevent_id_hex\tentity_id_hex\t"
            "time_ticks_le_hex\ttime_scale\tpriority\tsequence\t"
            "event_kind\tstatus\tpayload_ref"
        )
        if lines[1] != expected_fields:
            raise ValueError("unexpected field header")

        records: list[EventLogRecord] = []
        for line in lines[2:]:
            cells = line.split("\t")
            if len(cells) != 11:
                raise ValueError(f"expected 11 cells, got {len(cells)}")
            records.append(
                EventLogRecord(
                    schema_version=int(cells[0]),
                    run_id=_unescape_cell(cells[1]),
                    event_id=_parse_handle(cells[2], EventId),
                    entity_id=_parse_handle(cells[3], EntityId) if cells[3] else None,
                    time_ticks=int.from_bytes(bytes.fromhex(cells[4]), "little"),
                    time_scale=_unescape_cell(cells[5]),
                    priority=int(cells[6]),
                    sequence=int(cells[7]),
                    event_kind=_unescape_cell(cells[8]),
                    status=EventStatus(cells[9]),
                    payload_ref=_unescape_cell(cells[10]) or None,
                )
            )
        return cls(records)


def _handle_hex(handle: EventId | EntityId) -> str:
    return handle.index.to_bytes(8, "little").hex() + handle.generation.to_bytes(4, "little").hex()


def _parse_handle(hex_value: str, kind: type[EventId] | type[EntityId]) -> EventId | EntityId:
    if len(hex_value) != 24:
        raise ValueError("handle must be 12 bytes")
    payload = bytes.fromhex(hex_value)
    return kind(int.from_bytes(payload[:8], "little"), int.from_bytes(payload[8:12], "little"))


def _escape_cell(value: str) -> str:
    return value.replace("\\", "\\\\").replace("\t", "\\t").replace("\n", "\\n")


def _unescape_cell(value: str) -> str:
    output = []
    index = 0
    while index < len(value):
        char = value[index]
        if char == "\\" and index + 1 < len(value):
            index += 1
            next_char = value[index]
            output.append({"t": "\t", "n": "\n", "\\": "\\"}.get(next_char, "\\" + next_char))
        else:
            output.append(char)
        index += 1
    return "".join(output)
