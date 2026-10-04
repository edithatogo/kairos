"""Proposed C0-preserving Arrow physical schemas and reversible row codecs."""
from __future__ import annotations

import json
import re
from datetime import datetime, timezone
from typing import Any

import pyarrow as pa

FORMAT = "careops.calibration.physical"
PHYSICAL_VERSION = "2"
LOGICAL_SCHEMA_SHA256 = "8c46db62f691f243385a4ebdf8a7a3d3670e2655dd0c3f82cba4335ced2a3842"
U128_MAX = (1 << 128) - 1
I128_MIN, I128_MAX = -(1 << 127), (1 << 127) - 1
_U128_RE = re.compile(r"(?:0|[1-9][0-9]{0,38})\Z")
_RANK_RE = re.compile(r"(?:0|[1-9][0-9]*)\Z")


def _metadata(**values: str) -> dict[bytes, bytes]:
    return {key.encode(): value.encode() for key, value in values.items()}


def _field(name: str, dtype: pa.DataType, nullable: bool = True, path: str | None = None,
           encoding: str | None = None, unit: str | None = None) -> pa.Field:
    metadata = {"logical_path": path or name}
    if encoding:
        metadata["encoding"] = encoding
    if unit:
        metadata["unit"] = unit
    return pa.field(name, dtype, nullable=nullable, metadata=_metadata(**metadata))


def _lineage(path: str) -> pa.StructType:
    return pa.struct([
        _field("status", pa.string(), False, path + ".status"),
        _field("mapping_version", pa.string(), False, path + ".mapping_version"),
        _field("evidence_ref", pa.string(), True, path + ".evidence_ref"),
        _field("derivation", pa.string(), True, path + ".derivation"),
    ])


def _time(path: str) -> pa.StructType:
    return pa.struct([
        _field("raw", pa.string(), False, path + ".raw"),
        _field("utc_text", pa.string(), False, path + ".utc"),
        _field("utc_i128_le", pa.binary(16), False, path + ".utc", "signed_i128_le", "ns_since_unix_epoch"),
        _field("source_offset_or_zone", pa.string(), True, path + ".source_offset_or_zone"),
        _field("relative_ticks", pa.binary(16), False, path + ".relative_ticks", "unsigned_u128_le", "1ns"),
        _field("tick_resolution", pa.string(), True, path + ".tick_resolution"),
        _field("source_precision", pa.string(), False, path + ".source_precision"),
        _field("lineage", _lineage(path + ".lineage"), False, path + ".lineage"),
    ])


def _raw_event() -> pa.StructType:
    return pa.struct([
        _field("source_family", pa.string(), False, "raw_event.source_family"),
        _field("source_event_type", pa.string(), False, "raw_event.source_event_type"),
        _field("source_record_id", pa.string(), True, "raw_event.source_record_id"),
        _field("source_fields_json", pa.string(), True, "raw_event.source_fields"),
    ])


def _schema(record_type: str, fields: list[pa.Field]) -> pa.Schema:
    return pa.schema(fields + [_field("presence_fields", pa.list_(pa.field("element", pa.string(), nullable=False)), False, "@presence")], metadata=_metadata(
        format=FORMAT, physical_version=PHYSICAL_VERSION, logical_schema="calibration-v1",
        logical_schema_sha256=LOGICAL_SCHEMA_SHA256, record_type=record_type, byte_order="little",
        optional_presence="sorted_logical_paths"))


TRACE_EVENT_SCHEMA = _schema("trace_event.v1", [
    _field("record_type", pa.string(), False), _field("schema_version", pa.string(), False),
    _field("dataset_id", pa.string(), False), _field("mapping_version", pa.string(), False),
    _field("case_key", pa.string(), False), _field("source_event_key", pa.string(), False),
    _field("occurrence", pa.uint32(), False), _field("event_kind", pa.string(), False),
    _field("relative_ticks", pa.binary(16), False, encoding="unsigned_u128_le", unit="1ns"), _field("source_order", pa.uint64(), False),
    _field("event_kind_rank", pa.string(), True), _field("occurrence_time", _time("occurrence_time"), False),
    _field("source_recorded_time", _time("source_recorded_time")),
    _field("message_created_time", _time("message_created_time")),
    _field("time_lineage", pa.struct([
        _field("occurrence", _lineage("time_lineage.occurrence"), False),
        _field("source_recorded", _lineage("time_lineage.source_recorded"), False),
        _field("message_created", _lineage("time_lineage.message_created"), False)]), False),
    _field("resource_key", pa.string()), _field("actor_key", pa.string()), _field("location_key", pa.string()),
    _field("quality_flags", pa.list_(pa.field("element", pa.string(), nullable=False)), True), _field("raw_event", _raw_event(), False),
    _field("disposition", pa.string(), False), _field("knowledge_availability", pa.struct([
        _field("status", pa.string(), False, "knowledge_availability.status"),
        _field("available_at", _time("knowledge_availability.available_at"), True,
               "knowledge_availability.available_at")]), False),
])

TRACE_EXCLUSION_SCHEMA = _schema("trace_exclusion.v1", [
    _field("record_type", pa.string(), False), _field("schema_version", pa.string(), False),
    _field("dataset_id", pa.string(), False), _field("mapping_version", pa.string(), False),
    _field("source_event_key", pa.string(), False), _field("raw_event", _raw_event(), False),
    _field("raw_time_values", pa.list_(pa.field("element", pa.struct([
        _field("key", pa.string(), False, "raw_time_values.@key"),
        _field("value", pa.string(), True, "raw_time_values.@value"),
    ]), nullable=False)), False, encoding="sorted_unique_entries_v2"),
    _field("exclusion_reason", pa.string(), False), _field("lineage", _lineage("lineage"), False),
    _field("detail", pa.string()),
])

OUTCOME_SCHEMA = _schema("outcome_observation.v1", [
    _field("record_type", pa.string(), False), _field("schema_version", pa.string(), False),
    _field("dataset_id", pa.string(), False), _field("case_key", pa.string(), False),
    _field("endpoint", pa.string(), False), _field("risk_start", _time("risk_start")),
    _field("last_observed", _time("last_observed")), _field("event_observed", pa.bool_(), False),
    _field("event_time", _time("event_time")), _field("event_cause", pa.string()),
    _field("censor_status", pa.string(), False), _field("censor_reason", pa.string()),
    _field("cluster_ids", pa.list_(pa.field("element", pa.string(), nullable=False)), False), _field("lineage", _lineage("lineage"), False),
])
SCHEMAS = {"trace_event.v1": TRACE_EVENT_SCHEMA, "trace_exclusion.v1": TRACE_EXCLUSION_SCHEMA,
           "outcome_observation.v1": OUTCOME_SCHEMA}
REQUIRED_TOP_LEVEL = {
    "trace_event.v1": {"record_type", "schema_version", "dataset_id", "mapping_version", "case_key",
                       "source_event_key", "occurrence", "event_kind", "relative_ticks", "source_order",
                       "occurrence_time", "time_lineage", "raw_event", "disposition", "knowledge_availability"},
    "trace_exclusion.v1": {"record_type", "schema_version", "dataset_id", "mapping_version",
                           "source_event_key", "raw_event", "raw_time_values", "exclusion_reason", "lineage"},
    "outcome_observation.v1": {"record_type", "schema_version", "dataset_id", "case_key", "endpoint",
                               "risk_start", "last_observed", "event_observed", "event_time", "event_cause",
                               "censor_status", "censor_reason", "cluster_ids", "lineage"},
}
REQUIRED_CHILDREN = {
    "time_value": {"raw", "utc", "relative_ticks", "source_precision", "lineage"},
    "lineage": {"status", "mapping_version"},
    "raw_event": {"source_family", "source_event_type"},
    "knowledge_availability": {"status", "available_at"},
    "time_lineage": {"occurrence", "source_recorded", "message_created"},
}
ABSENT_ONLY = {"event_kind_rank", "quality_flags", "raw_event.source_fields", "tick_resolution"}


def _rank_text(value: int) -> str:
    if isinstance(value, bool) or not isinstance(value, int) or value < 0:
        raise ValueError("event_kind_rank must be a nonnegative integer")
    parts = []
    while value >= 1_000_000_000:
        value, tail = divmod(value, 1_000_000_000)
        parts.append(f"{tail:09d}")
    return str(value) + "".join(reversed(parts))


def _rank_int(value: str) -> int:
    if not isinstance(value, str) or not _RANK_RE.fullmatch(value):
        raise ValueError("event_kind_rank must be canonical decimal UTF-8")
    number = 0
    for start in range(0, len(value), 9):
        chunk = value[start:start + 9]
        number = number * 10**len(chunk) + int(chunk)
    return number


def _checked_int(value: Any, signed: bool) -> int:
    if isinstance(value, bool) or not isinstance(value, int):
        raise ValueError("clock value must be an integer")
    lo, hi = (I128_MIN, I128_MAX) if signed else (0, U128_MAX)
    if value < lo or value > hi:
        raise ValueError("clock value outside 128-bit range")
    return value


def _u128(value: Any) -> int:
    if not isinstance(value, str) or not _U128_RE.fullmatch(value):
        raise ValueError("u128 must be canonical unsigned decimal text")
    return _checked_int(int(value), False)


def _bytes(value: int, signed: bool) -> bytes:
    return value.to_bytes(16, "little", signed=signed)


def _utc_nanos(value: Any) -> int:
    if not isinstance(value, str) or not value:
        raise ValueError("UTC text must be non-empty RFC3339 text")
    if value.endswith("-00:00"):
        raise ValueError("unknown -00:00 offset is not a known UTC instant")
    match = re.fullmatch(r"([0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2})(?:\.([0-9]+))?(Z|[+-][0-9]{2}:[0-9]{2})", value)
    if not match:
        raise ValueError("invalid RFC3339 UTC text")
    base, fraction, zone = match.groups()
    if zone != "Z":
        offset_hour, offset_minute = int(zone[1:3]), int(zone[4:6])
        if offset_hour > 23 or offset_minute > 59:
            raise ValueError("invalid RFC3339 UTC offset")
    text = base + ("+00:00" if zone == "Z" else zone)
    try:
        dt = datetime.fromisoformat(text)
    except ValueError as exc:
        raise ValueError("invalid RFC3339 UTC text") from exc
    if dt.tzinfo is None or dt.utcoffset() is None:
        raise ValueError("UTC text must include an offset")
    fraction = fraction or ""
    if len(fraction) > 9 and any(char != "0" for char in fraction[9:]):
        raise ValueError("UTC text has precision below one nanosecond")
    fraction_nanos = int(fraction[:9].ljust(9, "0") or "0")
    try:
        delta = dt.astimezone(timezone.utc) - datetime(1970, 1, 1, tzinfo=timezone.utc)
    except OverflowError as exc:
        raise ValueError("UTC instant is outside Gregorian year 0001 through 9999") from exc
    nanos = (delta.days * 86400 + delta.seconds) * 1_000_000_000 + fraction_nanos
    return _checked_int(nanos, True)


def _canonical_json(value: Any) -> str:
    if not isinstance(value, dict):
        raise ValueError("source_fields must be a JSON object")
    try:
        return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"), allow_nan=False)
    except (TypeError, ValueError) as exc:
        raise ValueError("source_fields must be finite JSON-compatible data") from exc


def _validate_scalar(value: Any, dtype: pa.DataType, path: str) -> Any:
    if value is None:
        return None
    key = path.rsplit(".", 1)[-1]
    if pa.types.is_string(dtype):
        if not isinstance(value, str):
            raise ValueError(f"{path} must be UTF-8 text")
        enums = {
            "source_precision": {"minute", "second", "millisecond", "microsecond", "nanosecond", "other"},
            "tick_resolution": {"1ns"},
            "disposition": {"provisional", "realized", "unknown"},
            "exclusion_reason": {"date_only_or_coarse_precision", "missing_timezone", "ambiguous_dst_fold",
                                 "nonexistent_dst_gap", "pre_origin", "sub_nanosecond", "overflow",
                                 "missing_required_time", "invalid_mapping", "other"},
            "censor_status": {"not_censored", "left", "right", "interval", "unknown", "missing"},
        }
        if path == "knowledge_availability.status":
            enums["status"] = {"known", "not_yet_known", "unknown"}
        elif key == "status":
            enums["status"] = {"observed", "derived", "defaulted", "unknown"}
        if key in enums and value not in enums[key]:
            raise ValueError(f"invalid {key} enum value")
        if key == "schema_version" and value != "calibration-v1":
            raise ValueError("schema_version must be calibration-v1")
        if key in {"dataset_id", "mapping_version", "case_key", "source_event_key", "event_kind",
                   "endpoint", "mapping_version", "source_family", "source_event_type", "status", "raw"} and not value:
            raise ValueError(f"{path} must be non-empty")
        if path.endswith("[]") and path.rsplit(".", 1)[-1] in {"quality_flags[]", "cluster_ids[]"} and not value:
            raise ValueError(f"{path} values must be non-empty")
    elif pa.types.is_boolean(dtype):
        if not isinstance(value, bool):
            raise ValueError(f"{path} must be boolean")
    elif pa.types.is_integer(dtype):
        if isinstance(value, bool) or not isinstance(value, int):
            raise ValueError(f"{path} must be an integer")
        bounds = {pa.uint32(): (0, (1 << 32) - 1), pa.uint64(): (0, (1 << 64) - 1)}
        if dtype in bounds and not bounds[dtype][0] <= value <= bounds[dtype][1]:
            raise ValueError(f"{path} integer outside physical range")
    return value


def _optional_paths(dtype: pa.DataType, prefix: str = "") -> set[str]:
    paths: set[str] = set()
    if pa.types.is_struct(dtype):
        for field in dtype:
            path = prefix + "." + field.name if prefix else field.name
            if field.nullable:
                paths.add(path)
            paths.update(_optional_paths(field.type, path))
    elif pa.types.is_list(dtype):
        paths.update(_optional_paths(dtype.value_type, prefix + "[]"))
    return paths


def _encode_time(value: Any, path: str, absent: set[str]) -> dict[str, Any] | None:
    if value is None:
        return None
    if not isinstance(value, dict):
        raise ValueError("time_value must be an object or null")
    allowed = {"raw", "utc", "relative_ticks", "source_precision", "lineage",
               "source_offset_or_zone", "tick_resolution"}
    if set(value) - allowed:
        raise ValueError(f"unknown time_value fields: {sorted(set(value) - allowed)}")
    required = ("raw", "utc", "relative_ticks", "source_precision", "lineage")
    for key in required:
        if key not in value or value[key] is None:
            raise ValueError(f"required time field missing or null: {path}.{key}")
    _validate_scalar(value["raw"], pa.string(), path + ".raw")
    _validate_scalar(value["source_precision"], pa.string(), path + ".source_precision")
    out = {"raw": value["raw"], "utc_text": value["utc"],
           "utc_i128_le": _bytes(_utc_nanos(value["utc"]), True),
           "relative_ticks": _bytes(_u128(value["relative_ticks"]), False),
           "source_precision": value["source_precision"],
           "lineage": _encode_value(value["lineage"], _lineage(path + ".lineage"), path + ".lineage", absent)}
    for key in ("source_offset_or_zone", "tick_resolution"):
        if key not in value:
            absent.add(path + "." + key)
            out[key] = None
        else:
            if key == "tick_resolution" and value[key] is None:
                raise ValueError(f"present value cannot be null: {path}.{key}")
            _validate_scalar(value[key], pa.string(), path + "." + key)
            out[key] = value[key]
    return out


def _decode_time(value: Any, path: str, absent: set[str]) -> dict[str, Any] | None:
    if value is None:
        return None
    expected = {"raw", "utc_text", "utc_i128_le", "source_offset_or_zone", "relative_ticks",
                "tick_resolution", "source_precision", "lineage"}
    if not isinstance(value, dict) or set(value) != expected:
        raise ValueError("physical time_value fields do not match the declared schema")
    utc = value["utc_text"]
    if any(path + "." + key in absent for key in REQUIRED_CHILDREN["time_value"]):
        raise ValueError(f"required time field marked absent: {path}")
    if _bytes(_utc_nanos(utc), True) != value["utc_i128_le"]:
        raise ValueError("UTC text and physical i128 bytes disagree")
    if not isinstance(value["relative_ticks"], bytes) or len(value["relative_ticks"]) != 16:
        raise ValueError("physical u128 clock must be exactly 16 bytes")
    result = {"raw": value["raw"], "utc": utc,
              "relative_ticks": str(int.from_bytes(value["relative_ticks"], "little")),
              "source_precision": value["source_precision"],
              "lineage": _decode_value(value["lineage"], _lineage(path + ".lineage"), path + ".lineage", absent)}
    for key in ("source_offset_or_zone", "tick_resolution"):
        if path + "." + key in absent:
            if value[key] is not None:
                raise ValueError(f"invalid absence marker: {path}.{key}")
        else:
            result[key] = value[key]
    return result


def _encode_value(value: Any, dtype: pa.DataType, path: str, absent: set[str]) -> Any:
    if path == "raw_time_values":
        if not isinstance(value, dict) or any(not isinstance(k, str) or
                v is not None and not isinstance(v, str) for k, v in value.items()):
            raise ValueError("raw_time_values must map strings to nullable strings")
        return [{"key": k, "value": value[k]} for k in sorted(value)]
    if pa.types.is_struct(dtype):
        if value is None:
            return None
        if not isinstance(value, dict):
            raise ValueError(f"{path} must be an object")
        if "utc_text" in [f.name for f in dtype]:
            return _encode_time(value, path, absent)
        expected = {"source_fields" if f.name == "source_fields_json" else f.name for f in dtype}
        if set(value) - expected:
            raise ValueError(f"unknown fields at {path}: {sorted(set(value) - expected)}")
        result = {}
        for field in dtype:
            key = field.name
            child = path + (".source_fields" if key == "source_fields_json" else "." + key)
            logical_key = "source_fields" if key == "source_fields_json" else key
            if logical_key not in value:
                required = REQUIRED_CHILDREN.get(path.rsplit(".", 1)[-1], set())
                if logical_key in required or not field.nullable:
                    raise ValueError(f"required field missing: {child}")
                absent.add(child)
                result[key] = None
                continue
            item = value[logical_key]
            if child in ABSENT_ONLY and item is None:
                raise ValueError(f"present value cannot be null: {child}")
            if key == "source_fields_json" and item is not None:
                item = _canonical_json(item)
            if item is None and not field.nullable:
                raise ValueError(f"required field null: {child}")
            result[key] = _encode_value(item, field.type, child, absent)
        return result
    if pa.types.is_list(dtype):
        if value is None:
            return None
        if not isinstance(value, list) or any(item is None or not isinstance(item, str) or not item for item in value):
            raise ValueError(f"{path} must contain non-null UTF-8 values")
        if path.rsplit(".", 1)[-1] in {"quality_flags", "cluster_ids"} and len(value) != len(set(value)):
            raise ValueError(f"{path} must contain unique values")
        return [_encode_value(item, dtype.value_type, path + "[]", absent) for item in value]
    if path.endswith("relative_ticks"):
        return _bytes(_u128(value), False)
    if path.endswith("event_kind_rank"):
        if isinstance(value, bool) or not isinstance(value, int) or value < 0:
            raise ValueError("event_kind_rank must be a nonnegative integer")
        return _rank_text(value)
    if pa.types.is_map(dtype) and value is not None:
        if not isinstance(value, dict) or any(not isinstance(k, str) or v is not None and not isinstance(v, str)
                                               for k, v in value.items()):
            raise ValueError("raw_time_values must map strings to nullable strings")
        return sorted(value.items())
    return _validate_scalar(value, dtype, path)


def _decode_value(value: Any, dtype: pa.DataType, path: str, absent: set[str]) -> Any:
    if path == "raw_time_values":
        if not isinstance(value, list) or any(not isinstance(entry, dict) or
                set(entry) != {"key", "value"} or not isinstance(entry["key"], str) or
                entry["value"] is not None and not isinstance(entry["value"], str)
                for entry in value):
            raise ValueError("raw_time_values entries must have typed key/value fields")
        keys = [entry["key"] for entry in value]
        if keys != sorted(set(keys)):
            raise ValueError("raw_time_values keys must be sorted and unique")
        return {entry["key"]: entry["value"] for entry in value}
    if pa.types.is_struct(dtype):
        if value is None:
            return None
        if not isinstance(value, dict) or set(value) != {field.name for field in dtype}:
            raise ValueError(f"physical struct fields do not match schema at {path}")
        if "utc_text" in [f.name for f in dtype]:
            return _decode_time(value, path, absent)
        result = {}
        for field in dtype:
            key = field.name
            child = path + (".source_fields" if key == "source_fields_json" else "." + key)
            logical_key = "source_fields" if key == "source_fields_json" else key
            if child in absent:
                required = REQUIRED_CHILDREN.get(path.rsplit(".", 1)[-1], set())
                if logical_key in required or not field.nullable or value[key] is not None:
                    raise ValueError(f"invalid optional-presence marker: {child}")
                continue
            if value[key] is None and not field.nullable:
                raise ValueError(f"required physical field is null: {child}")
            item = _decode_value(value[key], field.type, child, absent)
            if child in ABSENT_ONLY and item is None:
                raise ValueError(f"present physical value cannot be null: {child}")
            if key == "source_fields_json" and item is not None:
                decoded = json.loads(item, parse_constant=lambda raw: (_ for _ in ()).throw(ValueError(raw)))
                if not isinstance(decoded, dict):
                    raise ValueError("source_fields_json must encode an object")
                if _canonical_json(decoded) != item:
                    raise ValueError("source_fields_json is not canonical JSON UTF-8")
                item = decoded
            result[logical_key] = item
        return result
    if pa.types.is_list(dtype) and value is not None:
        return [_decode_value(item, dtype.value_type, path + "[]", absent) for item in value]
    if pa.types.is_map(dtype) and value is not None:
        keys = [key for key, _ in value]
        if any(not isinstance(key, str) for key in keys):
            raise ValueError("raw_time_values keys must be UTF-8 strings")
        if keys != sorted(set(keys)):
            raise ValueError("raw_time_values keys must be sorted and unique")
        return dict(value)
    if path.endswith("relative_ticks") and value is not None:
        if not isinstance(value, bytes) or len(value) != 16:
            raise ValueError("physical u128 clock must be exactly 16 bytes")
        return str(int.from_bytes(value, "little", signed=False))
    if path.endswith("event_kind_rank") and value is not None:
        if not isinstance(value, str) or not _RANK_RE.fullmatch(value):
            raise ValueError("event_kind_rank must be canonical decimal UTF-8")
        return _rank_int(value)
    return _validate_scalar(value, dtype, path)


def encode_row(record: dict[str, Any]) -> dict[str, Any]:
    """Encode a logical row with explicit null/absence and range preservation."""
    if not isinstance(record, dict) or record.get("record_type") not in SCHEMAS:
        raise ValueError("unsupported logical record type")
    expected = {field.name for field in SCHEMAS[record["record_type"]]} - {"presence_fields"}
    if set(record) - expected:
        raise ValueError(f"unknown logical fields: {sorted(set(record) - expected)}")
    schema = SCHEMAS[record["record_type"]]
    absent: set[str] = set()
    output: dict[str, Any] = {}
    for field in schema:
        if field.name == "presence_fields":
            continue
        name = field.name
        if name not in record:
            if name in REQUIRED_TOP_LEVEL[record["record_type"]] or not field.nullable:
                raise ValueError(f"required field missing: {name}")
            absent.add(name)
            output[name] = None
            continue
        value = record[name]
        if name in ABSENT_ONLY and value is None:
            raise ValueError(f"present value cannot be null: {name}")
        if value is None and not field.nullable:
            raise ValueError(f"required logical field is null: {name}")
        output[name] = _encode_value(value, field.type, name, absent)
    output["presence_fields"] = sorted(absent)
    return output


def decode_row(physical: dict[str, Any], record_type: str) -> dict[str, Any]:
    """Decode and validate one physical row against its fixed table schema."""
    if record_type not in SCHEMAS or not isinstance(physical, dict):
        raise ValueError("unsupported record type or invalid row")
    paths = physical.get("presence_fields")
    if not isinstance(paths, list) or any(not isinstance(path, str) for path in paths) or paths != sorted(set(paths)):
        raise ValueError("presence_fields must be sorted unique UTF-8 paths")
    schema = SCHEMAS[record_type]
    names = {field.name for field in schema}
    if set(physical) != names:
        raise ValueError("row fields do not match exact declared schema")
    absent = set(paths)
    allowed_absence = set()
    for item in schema:
        if item.name != "presence_fields" and item.nullable:
            allowed_absence.add(item.name)
        allowed_absence.update(_optional_paths(item.type, item.name))
    allowed_absence.discard("raw_event.source_fields_json")
    allowed_absence.add("raw_event.source_fields")
    if not absent <= allowed_absence:
        raise ValueError("presence_fields contains unknown or required logical paths")
    output: dict[str, Any] = {}
    for field in schema:
        name = field.name
        if name == "presence_fields":
            continue
        if name in absent:
            if name in REQUIRED_TOP_LEVEL[record_type] or not field.nullable or physical[name] is not None:
                raise ValueError(f"invalid top-level absence marker: {name}")
            continue
        if physical[name] is None and not field.nullable:
            raise ValueError(f"required physical field is null: {name}")
        value = _decode_value(physical[name], field.type, name, absent)
        if name in ABSENT_ONLY and value is None:
            raise ValueError(f"present physical value cannot be null: {name}")
        output[name] = value
    if output.get("record_type") != record_type:
        raise ValueError("record_type column does not match selected table")
    if encode_row(output) != physical:
        raise ValueError("physical row has inconsistent or ineffective presence/type values")
    return output


def table_from_rows(record_type: str, rows: list[dict[str, Any]]) -> pa.Table:
    """Build a table using only the declared schema; never infer columns."""
    try:
        schema = SCHEMAS[record_type]
    except KeyError as exc:
        raise ValueError("unsupported logical record type") from exc
    return pa.Table.from_pylist([encode_row(row) for row in rows], schema=schema)
