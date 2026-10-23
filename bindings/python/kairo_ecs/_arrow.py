"""Event-log Schema v2 columnar helpers for the Python binding.

Track 70 Phase 2 (Task 2.2): the binding ships the explicit v2 schema
identifiers and 14 typed columns, exports real populated Arrow C Data arrays
(the fabricated-capsule fallback is removed: an unavailable native export
fails explicitly), and consumes Arrow tables column-first — the
``to_pylist()`` Python-row reconstruction detour is gone.
"""

from dataclasses import dataclass
from enum import Enum
from typing import Any

from ._types import DispatchedEvent, EntityId, EventId

SCHEMA_VERSION = 2
EVENT_LOG_STREAM = "kairo_ecs.event_log.v2"
TIME_SCALE_TICKS = "ticks"

# The Schema v2 event-log fields, defined once here and mirrored by
# `EVENT_LOG_FIELDS_V2` in `crates/kairo-ecs-arrow/src/lib.rs` (same order,
# names, contract types, nullability).
EVENT_LOG_FIELDS = (
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

_U64_MASK = (1 << 64) - 1

_SMOKE_FIELD_HEADER = "\t".join(field[0] for field in EVENT_LOG_FIELDS)


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
        # spec.md: nullable generational IDs null together.
        if self.entity_id is not None:
            if not isinstance(self.entity_id, EntityId):
                raise ValueError("entity_id must be an EntityId")


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
        # The smoke format is a human-readable dev roundtrip: cells correspond
        # 1:1 to EVENT_LOG_FIELDS in contract order; string cells are escaped,
        # numeric cells are decimal integers, and absent optional cells stay
        # empty.
        lines = [
            f"stream={EVENT_LOG_STREAM};schema_version={SCHEMA_VERSION}",
            _SMOKE_FIELD_HEADER,
        ]
        for record in self.records:
            entity_cells = (
                (
                    str(record.entity_id.index),
                    str(record.entity_id.generation),
                )
                if record.entity_id is not None
                else ("", "")
            )
            lines.append(
                "\t".join(
                    [
                        str(record.schema_version),
                        _escape_cell(record.run_id),
                        str(record.event_id.index),
                        str(record.event_id.generation),
                        entity_cells[0],
                        entity_cells[1],
                        str(record.time_ticks >> 64),
                        str(record.time_ticks & _U64_MASK),
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
        if lines[1] != _SMOKE_FIELD_HEADER:
            raise ValueError("unexpected field header")

        records: list[EventLogRecord] = []
        for line in lines[2:]:
            cells = line.split("\t")
            if len(cells) != len(EVENT_LOG_FIELDS):
                raise ValueError(
                    f"expected {len(EVENT_LOG_FIELDS)} cells, got {len(cells)}"
                )
            records.append(
                EventLogRecord(
                    schema_version=int(cells[0]),
                    run_id=_unescape_cell(cells[1]),
                    event_id=EventId(int(cells[2]), int(cells[3])),
                    entity_id=EntityId(int(cells[4]), int(cells[5]))
                    if cells[4] != "" or cells[5] != ""
                    else None,
                    time_ticks=(int(cells[6]) << 64) | int(cells[7]),
                    time_scale=_unescape_cell(cells[8]),
                    priority=int(cells[9]),
                    sequence=int(cells[10]),
                    event_kind=_unescape_cell(cells[11]),
                    status=EventStatus(cells[12]),
                    payload_ref=_unescape_cell(cells[13]) or None,
                )
            )
        return cls(records)

    def _column_values(self) -> dict[str, list[Any]]:
        """Columnar projection of the records — no per-row dictionaries."""

        values: dict[str, list[Any]] = {field[0]: [] for field in EVENT_LOG_FIELDS}
        for record in self.records:
            values["schema_version"].append(record.schema_version)
            values["run_id"].append(record.run_id)
            values["event_id_index"].append(record.event_id.index)
            values["event_id_gen"].append(record.event_id.generation)
            values["entity_id_index"].append(
                record.entity_id.index if record.entity_id is not None else None
            )
            values["entity_id_gen"].append(
                record.entity_id.generation if record.entity_id is not None else None
            )
            values["time_ticks_hi"].append(record.time_ticks >> 64)
            values["time_ticks_lo"].append(record.time_ticks & _U64_MASK)
            values["time_scale"].append(record.time_scale)
            values["priority"].append(record.priority)
            values["sequence"].append(record.sequence)
            values["event_kind"].append(record.event_kind)
            values["status"].append(record.status.value)
            values["payload_ref"].append(record.payload_ref)
        return values

    @staticmethod
    def _pyarrow_types() -> dict[str, Any]:
        import pyarrow as pa

        return {
            "schema_version": pa.uint16(),
            "run_id": pa.utf8(),
            "event_id_index": pa.uint64(),
            "event_id_gen": pa.uint32(),
            "entity_id_index": pa.uint64(),
            "entity_id_gen": pa.uint32(),
            "time_ticks_hi": pa.uint64(),
            "time_ticks_lo": pa.uint64(),
            "time_scale": pa.utf8(),
            "priority": pa.int32(),
            "sequence": pa.uint64(),
            "event_kind": pa.utf8(),
            "status": pa.utf8(),
            "payload_ref": pa.utf8(),
        }

    def to_pyarrow_table(self) -> Any:
        """Return a pyarrow Table using the Schema v2 event-log columns.

        Columnar construction: arrays are built per column, never from per-row
        Python dictionaries.
        """

        try:
            import pyarrow as pa
        except ModuleNotFoundError as exc:
            raise RuntimeError("pyarrow is required for Arrow table roundtrips") from exc

        values = self._column_values()
        types = self._pyarrow_types()
        arrays = [
            pa.array(values[field_name], type=types[field_name])
            for field_name, _, _ in EVENT_LOG_FIELDS
        ]
        schema = self._pyarrow_schema(pa, types)
        return pa.Table.from_arrays(arrays, schema=schema)

    @staticmethod
    def _pyarrow_schema(pa: Any, types: dict[str, Any]) -> Any:
        return pa.schema(
            [
                pa.field(field_name, types[field_name], nullable=nullable)
                for field_name, _, nullable in EVENT_LOG_FIELDS
            ]
        )

    def __arrow_c_schema__(self) -> Any:
        return self.to_pyarrow_table().schema.__arrow_c_schema__()

    def __arrow_c_array__(self, requested_schema: Any = None) -> tuple[Any, Any]:
        """Hand the columnar record batch across the Arrow C data interface.

        There is no fabricated-capsule fallback: if the native export is
        unavailable (pyarrow missing or conversion failing) the error surfaces
        explicitly, never as a fake ``+s`` capsule pair.
        """

        try:
            table = self.to_pyarrow_table()
            batches = table.combine_chunks().to_batches()
            if batches:
                return batches[0].__arrow_c_array__(requested_schema=requested_schema)
        except Exception as exc:
            raise RuntimeError(f"native Arrow export unavailable: {exc}") from exc

        import pyarrow as pa

        types = self._pyarrow_types()
        arrays = [
            pa.array([], type=types[field_name])
            for field_name, _, _ in EVENT_LOG_FIELDS
        ]
        empty_batch = pa.RecordBatch.from_arrays(
            arrays, schema=self._pyarrow_schema(pa, types)
        )
        return empty_batch.__arrow_c_array__(requested_schema=requested_schema)

    def export_arrow_c_capsule_descriptor(self) -> dict[str, Any]:
        schema_cap, array_cap = self.__arrow_c_array__()
        return {
            "schema_capsule": schema_cap,
            "array_capsule": array_cap,
            "format": "+s",
            "name": EVENT_LOG_STREAM,
            "records_count": len(self.records),
        }

    def to_polars_dict(self) -> dict[str, list[Any]]:
        """Columnar projection for the Polars consumer path (v2 columns).

        The v2 `sequence` column is the canonical per-run sort key.
        """

        return self._column_values()

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

        # Column-first reading: pull each column once, then zip rows. No
        # per-row dictionary reconstruction detour.
        columns = {}
        for field_name, _, _ in EVENT_LOG_FIELDS:
            columns[field_name] = table.column(field_name).to_pylist()

        records: list[EventLogRecord] = []
        for row in range(len(table)):
            entity_index = columns["entity_id_index"][row]
            entity_gen = columns["entity_id_gen"][row]
            # spec.md: nullable generational IDs null together.
            if (entity_index is None) != (entity_gen is None):
                raise ValueError(
                    "entity_id_index and entity_id_gen must null together"
                )
            records.append(
                EventLogRecord(
                    schema_version=int(columns["schema_version"][row]),
                    run_id=str(columns["run_id"][row]),
                    event_id=EventId(
                        int(columns["event_id_index"][row]),
                        int(columns["event_id_gen"][row]),
                    ),
                    entity_id=EntityId(int(entity_index), int(entity_gen))
                    if entity_index is not None
                    else None,
                    time_ticks=(
                        (int(columns["time_ticks_hi"][row]) << 64)
                        | int(columns["time_ticks_lo"][row])
                    ),
                    time_scale=str(columns["time_scale"][row]),
                    priority=int(columns["priority"][row]),
                    sequence=int(columns["sequence"][row]),
                    event_kind=str(columns["event_kind"][row]),
                    status=EventStatus(columns["status"][row]),
                    payload_ref=columns["payload_ref"][row],
                )
            )
        return cls(records)


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
