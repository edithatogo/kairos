"""Event-log v1 smoke roundtrip helpers for the Python binding."""

import ctypes
from dataclasses import dataclass
from enum import Enum
from typing import Any, TypeVar

from ._types import DispatchedEvent, EntityId, EventId

_HandleT = TypeVar("_HandleT", EventId, EntityId)

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
            (
                "schema_version\trun_id\tevent_id_hex\tentity_id_hex\t"
                "time_ticks_le_hex\ttime_scale\tpriority\tsequence\t"
                "event_kind\tstatus\tpayload_ref"
            ),
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

    def to_pyarrow_table(self) -> Any:
        """Return a pyarrow Table using the Track 04 event-log schema."""

        try:
            import pyarrow as pa
        except ModuleNotFoundError as exc:
            raise RuntimeError("pyarrow is required for Arrow table roundtrips") from exc

        rows = [
            {
                "schema_version": record.schema_version,
                "run_id": record.run_id,
                "event_id": _handle_bytes(record.event_id),
                "entity_id": _handle_bytes(record.entity_id) if record.entity_id else None,
                "time_ticks": record.time_ticks.to_bytes(16, "little"),
                "time_scale": record.time_scale,
                "priority": record.priority,
                "sequence": record.sequence,
                "event_kind": record.event_kind,
                "status": record.status.value,
                "payload_ref": record.payload_ref,
            }
            for record in self.records
        ]
        schema = pa.schema(
            [
                pa.field("schema_version", pa.uint16(), nullable=False),
                pa.field("run_id", pa.utf8(), nullable=False),
                pa.field("event_id", pa.binary(12), nullable=False),
                pa.field("entity_id", pa.binary(12), nullable=True),
                pa.field("time_ticks", pa.binary(16), nullable=False),
                pa.field("time_scale", pa.utf8(), nullable=False),
                pa.field("priority", pa.int32(), nullable=False),
                pa.field("sequence", pa.uint64(), nullable=False),
                pa.field("event_kind", pa.utf8(), nullable=False),
                pa.field("status", pa.utf8(), nullable=False),
                pa.field("payload_ref", pa.utf8(), nullable=True),
            ]
        )
        return pa.Table.from_pylist(rows, schema=schema)

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


    def __arrow_c_schema__(self) -> Any:
        try:
            return self.to_pyarrow_table().schema.__arrow_c_schema__()
        except (RuntimeError, AttributeError):
            return _allocate_arrow_schema_capsule()

    def __arrow_c_array__(self, requested_schema: Any = None) -> tuple[Any, Any]:
        try:
            table = self.to_pyarrow_table()
            batches = table.combine_chunks().to_batches()
            if batches:
                return batches[0].__arrow_c_array__(requested_schema=requested_schema)
            import pyarrow as pa

            empty_batch = pa.RecordBatch.from_pydict(self.to_column_dict(), schema=table.schema)
            return empty_batch.__arrow_c_array__(requested_schema=requested_schema)
        except (RuntimeError, AttributeError):
            return (_allocate_arrow_schema_capsule(), _allocate_arrow_array_capsule(len(self.records)))

    def export_arrow_c_capsule_descriptor(self) -> dict[str, Any]:
        schema_cap, array_cap = self.__arrow_c_array__()
        return {
            "schema_capsule": schema_cap,
            "array_capsule": array_cap,
            "format": "+s",
            "name": "event_log_batch",
            "records_count": len(self.records),
        }

    def to_polars_dict(self) -> dict[str, list[Any]]:
        return {
            "run_id": [r.run_id for r in self.records],
            "time_ticks": [r.time_ticks for r in self.records],
            "priority": [r.priority for r in self.records],
            "sequence": [r.sequence for r in self.records],
            "status": [r.status.value for r in self.records],
        }

    def to_duckdb_relation_spec(self) -> dict[str, Any]:
        return {
            "table_name": "kairo_ecs_event_log",
            "arrow_c_data_interface": self.export_arrow_c_capsule_descriptor(),
            "row_count": len(self.records),
        }

    @classmethod
    def from_pyarrow_table(cls, table: Any) -> "EventLogBatch":
        if tuple(table.schema.names) != tuple(field[0] for field in EVENT_LOG_FIELDS):
            raise ValueError("unexpected Arrow event-log field order")

        records = []
        for row in table.to_pylist():
            records.append(
                EventLogRecord(
                    schema_version=int(row["schema_version"]),
                    run_id=str(row["run_id"]),
                    event_id=_parse_handle_bytes(row["event_id"], EventId),
                    entity_id=_parse_handle_bytes(row["entity_id"], EntityId)
                    if row["entity_id"] is not None
                    else None,
                    time_ticks=int.from_bytes(row["time_ticks"], "little"),
                    time_scale=str(row["time_scale"]),
                    priority=int(row["priority"]),
                    sequence=int(row["sequence"]),
                    event_kind=str(row["event_kind"]),
                    status=EventStatus(row["status"]),
                    payload_ref=row["payload_ref"],
                )
            )
        return cls(records)


def _handle_hex(handle: EventId | EntityId) -> str:
    return _handle_bytes(handle).hex()


def _handle_bytes(handle: EventId | EntityId) -> bytes:
    return handle.index.to_bytes(8, "little") + handle.generation.to_bytes(4, "little")


def _parse_handle(hex_value: str, kind: type[_HandleT]) -> _HandleT:
    if len(hex_value) != 24:
        raise ValueError("handle must be 12 bytes")
    return _parse_handle_bytes(bytes.fromhex(hex_value), kind)


def _parse_handle_bytes(
    payload: bytes, kind: type[_HandleT]
) -> _HandleT:
    if len(payload) != 12:
        raise ValueError("handle must be 12 bytes")
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


class _CArrowSchema(ctypes.Structure):
    pass


_SCHEMA_RELEASE_TYPE = ctypes.CFUNCTYPE(None, ctypes.POINTER(_CArrowSchema))
_CArrowSchema._fields_ = [
    ("format", ctypes.c_char_p),
    ("name", ctypes.c_char_p),
    ("metadata", ctypes.c_char_p),
    ("flags", ctypes.c_int64),
    ("n_children", ctypes.c_int64),
    ("children", ctypes.c_void_p),
    ("dictionary", ctypes.c_void_p),
    ("release", _SCHEMA_RELEASE_TYPE),
    ("private_data", ctypes.c_void_p),
]


class _CArrowArray(ctypes.Structure):
    pass


_ARRAY_RELEASE_TYPE = ctypes.CFUNCTYPE(None, ctypes.POINTER(_CArrowArray))
_CArrowArray._fields_ = [
    ("length", ctypes.c_int64),
    ("null_count", ctypes.c_int64),
    ("offset", ctypes.c_int64),
    ("n_buffers", ctypes.c_int64),
    ("n_children", ctypes.c_int64),
    ("buffers", ctypes.c_void_p),
    ("children", ctypes.c_void_p),
    ("dictionary", ctypes.c_void_p),
    ("release", _ARRAY_RELEASE_TYPE),
    ("private_data", ctypes.c_void_p),
]


@_SCHEMA_RELEASE_TYPE
def _noop_schema_release(ptr: Any) -> None:
    if ptr:
        ptr.contents.release = _SCHEMA_RELEASE_TYPE(0)


@_ARRAY_RELEASE_TYPE
def _noop_array_release(ptr: Any) -> None:
    if ptr:
        ptr.contents.release = _ARRAY_RELEASE_TYPE(0)


_LIVE_CAPSULE_KEEPALIVE: list[Any] = []
_PYCAPSULE_NEW = ctypes.pythonapi.PyCapsule_New
_PYCAPSULE_NEW.restype = ctypes.py_object
_PYCAPSULE_NEW.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_void_p]


def _allocate_arrow_schema_capsule() -> Any:
    schema = _CArrowSchema(
        format=b"+s",
        name=b"event_log_batch",
        metadata=None,
        flags=0,
        n_children=0,
        children=0,
        dictionary=0,
        release=_noop_schema_release,
        private_data=0,
    )
    _LIVE_CAPSULE_KEEPALIVE.append(schema)
    return _PYCAPSULE_NEW(ctypes.addressof(schema), b"arrow_schema", None)


def _allocate_arrow_array_capsule(length: int) -> Any:
    buf_ptr = (ctypes.c_void_p * 1)(0)
    array = _CArrowArray(
        length=length,
        null_count=0,
        offset=0,
        n_buffers=1,
        n_children=0,
        buffers=ctypes.addressof(buf_ptr),
        children=0,
        dictionary=0,
        release=_noop_array_release,
        private_data=0,
    )
    _LIVE_CAPSULE_KEEPALIVE.append((array, buf_ptr))
    return _PYCAPSULE_NEW(ctypes.addressof(array), b"arrow_array", None)

