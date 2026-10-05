#!/usr/bin/env python3
"""Independent PyArrow reader for the private C4.3 physical-v1 sidecars."""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import re
import sys
import tempfile
from pathlib import Path
from typing import Any

import pyarrow as pa
import pyarrow.ipc as ipc
import pyarrow.parquet as pq

ROOT = Path(__file__).resolve().parents[2]
C0_SCHEMA_PATH = ROOT / "conductor/research/c0.3-source-inputs-20261001/calibration-v1.schema.json"
C0_SCHEMA_SHA256 = "8c46db62f691f243385a4ebdf8a7a3d3670e2655dd0c3f82cba4335ced2a3842"
EVENT_SCHEMA_PATH = ROOT / "schemas/arrow/event_log_v1.schema.json"
EVENT_SCHEMA_SHA256 = "2e8025a5997f1e056956d47db3b18d3e01e3b0fd865d7235803811ae2aab9dcc"
U128_MAX = (1 << 128) - 1
U128_RE = re.compile(r"(?:0|[1-9][0-9]{0,38})\Z")
FORMATS = ("ipc_file", "ipc_stream", "parquet")

RESIDUAL_FIELDS = (
    "record_type", "schema_version", "dataset_id", "scenario_id", "run_id", "candidate_id",
    "case_key", "task_key", "occurrence", "endpoint", "fidelity", "observed_ticks",
    "predicted_ticks", "residual_status", "residual_sign", "residual_magnitude",
    "prediction_unclamped", "anchor_role", "feasibility", "censor_status", "study_id",
    "replication_id", "seed_schedule_id", "seed_purpose", "seed_map_ref",
    "seed_contract_version", "mapping_version", "parameter_hash", "graph_hash", "causal_ref",
)
METRIC_FIELDS = (
    "record_type", "schema_version", "metric", "algorithm_version", "endpoint", "strata",
    "window_start_ticks", "window_end_ticks", "units", "reference_count", "simulation_count",
    "excluded_count", "censored_count", "missing_count", "unmatched_count", "failed_count",
    "infeasible_count", "value", "status", "uncertainty", "provenance_dataset_id",
    "provenance_run_id", "provenance_mapping_version", "provenance_seed_schedule_id",
    "provenance_seed_map_ref", "provenance_seed_contract_version", "provenance_parameter_hash",
)
COUNT_FIELDS = ("reference_count", "simulation_count", "excluded_count", "censored_count",
                "missing_count", "unmatched_count", "failed_count", "infeasible_count")
RESIDUAL_TICK_FIELDS = ("observed_ticks", "predicted_ticks", "residual_magnitude")
METRIC_TICK_FIELDS = ("window_start_ticks", "window_end_ticks")


def md(**items: str) -> dict[bytes, bytes]:
    return {k.encode(): v.encode() for k, v in items.items()}


def field(name: str, typ: pa.DataType, nullable: bool = False, *,
          encoding: str | None = None, unit: str | None = None) -> pa.Field:
    meta = {}
    if encoding:
        meta["encoding"] = encoding
    if unit:
        meta["unit"] = unit
    return pa.field(name, typ, nullable=nullable, metadata=md(**meta) if meta else None)


def physical_schema(kind: str) -> pa.Schema:
    if kind == "residual":
        fields = []
        nullable_text = {"graph_hash", "causal_ref"}
        for name in RESIDUAL_FIELDS:
            if name in RESIDUAL_TICK_FIELDS:
                fields.append(field(name, pa.binary(16), True,
                                    encoding="unsigned_u128_le", unit="1ns"))
            elif name == "occurrence":
                fields.append(field(name, pa.uint32()))
            elif name == "prediction_unclamped":
                fields.append(field(name, pa.bool_()))
            else:
                fields.append(field(name, pa.string(), name in nullable_text))
        record_type = "calibration_residual.v1"
    elif kind == "metric":
        fields = []
        nullable_text = {"uncertainty", "provenance_seed_schedule_id", "provenance_seed_map_ref",
                         "provenance_seed_contract_version", "provenance_parameter_hash"}
        for name in METRIC_FIELDS:
            if name in METRIC_TICK_FIELDS:
                fields.append(field(name, pa.binary(16), False,
                                    encoding="unsigned_u128_le", unit="1ns"))
            elif name in COUNT_FIELDS:
                fields.append(field(name, pa.uint64()))
            elif name == "strata":
                fields.append(field(name, pa.string(), False, encoding="canonical_json_object"))
            elif name == "value":
                fields.append(field(name, pa.float64(), True))
            elif name == "uncertainty":
                fields.append(field(name, pa.string(), True))
            else:
                fields.append(field(name, pa.string(), name in nullable_text))
        record_type = "calibration_metric.v1"
    else:
        raise ValueError(f"unknown record kind {kind}")
    return pa.schema(fields, metadata=md(format="kairoecs.calibration.output", physical_version="1",
                                         logical_schema="calibration-v1", record_type=record_type,
                                         byte_order="little"))


SCHEMAS = {"residual": physical_schema("residual"), "metric": physical_schema("metric")}


def fail(message: str) -> None:
    raise ValueError(message)


def canonical(value: Any) -> str:
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"), allow_nan=False)


def parse_u128(value: Any, label: str) -> int:
    if not isinstance(value, str) or not U128_RE.fullmatch(value):
        fail(f"{label} must be canonical unsigned decimal text")
    number = int(value)
    if number > U128_MAX:
        fail(f"{label} exceeds u128")
    return number


def u128_bytes(value: Any, label: str) -> bytes | None:
    if value is None:
        return None
    return parse_u128(value, label).to_bytes(16, "little")


def logical_flat(row: dict[str, Any], kind: str) -> dict[str, Any]:
    if kind == "residual":
        if set(row) != set(RESIDUAL_FIELDS):
            fail("logical residual fields differ from frozen C0 required-field order")
        out = dict(row)
        for name in RESIDUAL_TICK_FIELDS:
            out[name] = u128_bytes(row[name], name)
        return out
    if set(row) != {"record_type", "schema_version", "metric", "algorithm_version", "endpoint", "strata",
                    "window", "units", *COUNT_FIELDS, "value", "status", "uncertainty", "provenance"}:
        fail("logical metric fields differ from frozen C0 record")
    out = {k: row[k] for k in METRIC_FIELDS if not k.startswith("window_") and not k.startswith("provenance_")}
    out["strata"] = canonical(row["strata"])
    out["window_start_ticks"] = u128_bytes(row["window"]["start_ticks"], "window.start_ticks")
    out["window_end_ticks"] = u128_bytes(row["window"]["end_ticks"], "window.end_ticks")
    for suffix in ("dataset_id", "run_id", "mapping_version", "seed_schedule_id", "seed_map_ref",
                   "seed_contract_version", "parameter_hash"):
        out["provenance_" + suffix] = row["provenance"].get(suffix)
    return out


def validate_logical(records: list[dict[str, Any]], kind: str) -> None:
    schema = json.loads(C0_SCHEMA_PATH.read_text())
    target = "calibration_residual.v1" if kind == "residual" else "calibration_metric.v1"
    for i, row in enumerate(records):
        if not isinstance(row, dict):
            fail(f"{kind} logical row {i} must be an object")
        _validate_unchanged_c0(schema, row, f"{kind} logical row {i}")
        if row.get("record_type") != target:
            fail(f"{kind} logical row {i} has wrong record_type")
        if kind == "residual":
            _validate_residual_semantics(row, i)
        else:
            _validate_metric_semantics(row, i)
    if kind == "metric":
        cohort_units: dict[tuple[Any, ...], str] = {}
        for row in records:
            if row["metric"] != "W1":
                continue
            provenance = row["provenance"]
            key = (row["endpoint"], canonical(row["strata"]), row["window"]["start_ticks"],
                   row["window"]["end_ticks"], provenance["dataset_id"], provenance["run_id"],
                   provenance["mapping_version"], provenance["seed_schedule_id"], provenance["seed_map_ref"])
            previous = cohort_units.setdefault(key, row["units"])
            if previous != row["units"]:
                fail("W1 records in one endpoint/strata/window/provenance cohort use mixed units")
    keys = [_logical_key(row, kind) for row in records]
    if keys != sorted(keys):
        fail(f"{kind} logical records are not in frozen canonical order")
    if len(keys) != len(set(keys)):
        fail(f"duplicate {kind} logical output key")


def _logical_key(row: dict[str, Any], kind: str) -> tuple[Any, ...]:
    if kind == "residual":
        return (row["study_id"], row["dataset_id"], row["scenario_id"], row["seed_schedule_id"],
                row["replication_id"], row["case_key"], row["task_key"], row["occurrence"],
                row["endpoint"], row["seed_purpose"], row["seed_map_ref"], row["mapping_version"],
                row["candidate_id"], row["run_id"])
    provenance = row["provenance"]
    return (row["endpoint"], canonical(row["strata"]), int(row["window"]["start_ticks"]),
            int(row["window"]["end_ticks"]), provenance["dataset_id"], provenance["run_id"],
            provenance["mapping_version"], provenance["seed_schedule_id"] or "",
            provenance["seed_map_ref"] or "", provenance.get("seed_contract_version") or "",
            provenance.get("parameter_hash") or "", row["metric"], row["algorithm_version"])


def _validate_unchanged_c0(schema: dict[str, Any], row: dict[str, Any], label: str) -> None:
    def_name = {"calibration_residual.v1": "calibration_residual",
                "calibration_metric.v1": "calibration_metric"}.get(row.get("record_type"))
    if def_name is None or def_name not in schema.get("$defs", {}):
        fail(f"{label} record type has no frozen C0 definition")
    errors = list(_schema_errors(row, schema["$defs"][def_name], schema))
    if errors:
        fail(f"{label} fails unchanged C0 schema: {errors[0]}")


def _schema_errors(value: Any, rule: dict[str, Any], root_schema: dict[str, Any], path: str = "$ "):
    """Evaluate the bounded keyword subset used by the frozen C0 output defs."""
    allowed = {"$ref", "type", "required", "properties", "additionalProperties", "const", "enum",
               "oneOf", "allOf", "if", "then", "pattern", "minimum", "maximum", "minLength",
               "title", "description"}
    unknown = set(rule) - allowed
    if unknown:
        yield f"unsupported C0 schema keyword(s) at {path}: {sorted(unknown)}"
        return
    if "$ref" in rule:
        ref = rule["$ref"]
        if not isinstance(ref, str) or not ref.startswith("#/$defs/"):
            yield f"unsupported C0 schema reference {ref!r} at {path}"
            return
        target: Any = root_schema
        for part in ref[2:].split("/"):
            target = target.get(part) if isinstance(target, dict) else None
        if not isinstance(target, dict):
            yield f"unresolved C0 schema reference {ref!r} at {path}"
            return
        yield from _schema_errors(value, target, root_schema, path)
    types = rule.get("type")
    if types is not None:
        types = [types] if isinstance(types, str) else types
        if not any(_matches_type(value, t) for t in types):
            yield f"{path} has wrong JSON type; expected {types}"
            return
    if "const" in rule and value != rule["const"]:
        yield f"{path} does not equal required constant"
    if "enum" in rule and value not in rule["enum"]:
        yield f"{path} is outside allowed enum"
    if isinstance(value, str):
        if "minLength" in rule and len(value) < rule["minLength"]:
            yield f"{path} is shorter than minLength"
        if "pattern" in rule and not re.search(rule["pattern"], value):
            yield f"{path} does not match required pattern"
    if isinstance(value, (int, float)) and not isinstance(value, bool):
        if "minimum" in rule and value < rule["minimum"]:
            yield f"{path} is below minimum"
        if "maximum" in rule and value > rule["maximum"]:
            yield f"{path} is above maximum"
    if isinstance(value, dict):
        for name in rule.get("required", []):
            if name not in value:
                yield f"{path} missing required property {name}"
        props = rule.get("properties", {})
        for name, child in value.items():
            if name in props:
                yield from _schema_errors(child, props[name], root_schema, f"{path}.{name}")
            elif rule.get("additionalProperties") is False:
                yield f"{path} has unknown property {name}"
    for child in rule.get("allOf", []):
        yield from _schema_errors(value, child, root_schema, path)
    if "if" in rule:
        condition_errors = list(_schema_errors(value, rule["if"], root_schema, path))
        if not condition_errors and "then" in rule:
            yield from _schema_errors(value, rule["then"], root_schema, path)
    if "oneOf" in rule:
        successful = sum(not list(_schema_errors(value, child, root_schema, path))
                         for child in rule["oneOf"])
        if successful != 1:
            yield f"{path} must match exactly one oneOf branch (matched {successful})"


def _matches_type(value: Any, expected: str) -> bool:
    return {
        "object": isinstance(value, dict), "array": isinstance(value, list),
        "string": isinstance(value, str),
        "integer": isinstance(value, int) and not isinstance(value, bool),
        "number": isinstance(value, (int, float)) and not isinstance(value, bool),
        "boolean": isinstance(value, bool), "null": value is None,
    }.get(expected, False)


def _validate_residual_semantics(row: dict[str, Any], i: int) -> None:
    for name in ("observed_ticks", "predicted_ticks", "residual_magnitude"):
        if row[name] is not None:
            parse_u128(row[name], name)
    status = row["residual_status"]
    if isinstance(row["occurrence"], bool):
        fail(f"residual row {i} occurrence must be UInt32 integer, not bool")
    if row["prediction_unclamped"] is not True:
        fail(f"residual row {i} prediction_unclamped must be true")
    if status == "computed":
        if row["observed_ticks"] is None or row["predicted_ticks"] is None:
            fail(f"computed residual row {i} requires both ticks")
        obs, pred = int(row["observed_ticks"]), int(row["predicted_ticks"])
        delta = pred - obs
        expected_sign = "positive" if delta > 0 else "negative" if delta < 0 else "zero"
        if row["residual_sign"] != expected_sign or int(row["residual_magnitude"]) != abs(delta):
            fail(f"computed residual row {i} sign/magnitude disagrees with checked predicted-observed")
    else:
        if row["residual_sign"] != "undefined" or row["residual_magnitude"] is not None:
            fail(f"noncomputed residual row {i} must use undefined sign and null magnitude")
        if status in ("missing_observed", "censored") and row["observed_ticks"] is not None:
            fail(f"{status} residual row {i} must have null observed tick")
        if status == "probe_failed" and row["predicted_ticks"] is not None:
            fail(f"probe_failed residual row {i} must have null predicted tick")
    if row["feasibility"] == "infeasible" and status not in ("infeasible", "computed"):
        fail(f"infeasible feasibility/status mismatch at residual row {i}")


def _validate_metric_semantics(row: dict[str, Any], i: int) -> None:
    win = row["window"]
    start = parse_u128(win["start_ticks"], "window.start_ticks")
    end = parse_u128(win["end_ticks"], "window.end_ticks")
    if start >= end:
        fail(f"metric row {i} window must be nonempty half-open [start,end)")
    for name in COUNT_FIELDS:
        value = row[name]
        if isinstance(value, bool) or not isinstance(value, int) or value < 0 or value > (1 << 64) - 1:
            fail(f"metric row {i} {name} must be UInt64-range integer, not bool")
    value = row["value"]
    if value is not None and (isinstance(value, bool) or not isinstance(value, (int, float)) or not math.isfinite(value)):
        fail(f"metric row {i} value must be finite numeric or null")
    if row["status"] != "computed" and value is not None:
        fail(f"noncomputed metric row {i} must have null value")
    if row["uncertainty"] is not None:
        fail("non-null uncertainty is unsupported until a separately named method exists")
    if row["metric"] == "W1" and row["units"] == "dimensionless":
        fail(f"W1 metric row {i} must carry cohort units")
    if row["metric"] == "KS_D" and row["units"] != "dimensionless":
        fail(f"KS_D metric row {i} must be dimensionless")
    for tick in (win["start_ticks"], win["end_ticks"]):
        parse_u128(tick, "metric window tick")


def _rows_to_table(records: list[dict[str, Any]], kind: str) -> pa.Table:
    schema = SCHEMAS[kind]
    flats = [logical_flat(row, kind) for row in records]
    return pa.Table.from_pylist(flats, schema=schema)


def read_file(path: Path, fmt: str) -> pa.Table:
    if fmt == "ipc_file":
        with path.open("rb") as stream:
            return ipc.open_file(stream).read_all()
    if fmt == "ipc_stream":
        with path.open("rb") as stream:
            return ipc.open_stream(stream).read_all()
    if fmt == "parquet":
        return pq.read_table(path)
    fail(f"unsupported format {fmt}")


def write_table(table: pa.Table, path: Path, fmt: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    if fmt == "ipc_file":
        with path.open("wb") as stream, ipc.new_file(stream, table.schema) as writer:
            writer.write_table(table)
    elif fmt == "ipc_stream":
        with path.open("wb") as stream, ipc.new_stream(stream, table.schema) as writer:
            writer.write_table(table)
    elif fmt == "parquet":
        pq.write_table(table, path, compression="NONE")
    else:
        fail(f"unsupported format {fmt}")


def verify_physical(table: pa.Table, kind: str, expected_records: list[dict[str, Any]]) -> None:
    schema = SCHEMAS[kind]
    if not table.schema.equals(schema, check_metadata=True):
        fail(f"{kind} physical schema/type/order/nullability/metadata mismatch")
    if table.num_columns != len(schema) or table.num_rows != len(expected_records):
        fail(f"{kind} row/column count mismatch")
    expected = _rows_to_table(expected_records, kind)
    if not table.equals(expected, check_metadata=True):
        fail(f"{kind} physical values/nulls differ from actual logical JSON")


def load_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"), parse_constant=lambda v: fail(f"invalid JSON constant {v}"))


def check_event_joins(directory: Path, residuals: list[dict[str, Any]]) -> None:
    manifest_path = directory / "source_manifest.json"
    if manifest_path.exists():
        manifest = load_json(manifest_path)
        if not isinstance(manifest, dict) or not isinstance(manifest.get("runs"), list) or not isinstance(manifest.get("events"), list):
            fail("source_manifest.json must contain runs and events arrays")
        runs = {}
        for row in manifest["runs"]:
            if not isinstance(row, dict) or not isinstance(row.get("run_id"), str):
                fail("source manifest run entries require run_id")
            if row["run_id"] in runs:
                fail(f"duplicate source manifest run_id {row['run_id']}")
            runs[row["run_id"]] = row
        for i, row in enumerate(residuals):
            run = runs.get(row["run_id"])
            if run is None:
                fail(f"residual row {i} has no source manifest run")
            for name in ("candidate_id", "dataset_id", "scenario_id", "study_id", "replication_id",
                         "seed_schedule_id", "seed_map_ref", "mapping_version", "parameter_hash"):
                if row[name] != run.get(name):
                    fail(f"residual row {i} {name} differs from source manifest run")
        event_keys = set()
        for i, event in enumerate(manifest["events"]):
            if not isinstance(event, dict) or not isinstance(event.get("run_id"), str) or not isinstance(event.get("event_id"), str):
                fail(f"source manifest event {i} requires run_id/event_id strings")
            if event.get("time_ticks") is not None:
                parse_u128(event["time_ticks"], f"source manifest event {i} time_ticks")
            pair = (event["run_id"], event["event_id"])
            if pair in event_keys:
                fail(f"duplicate source manifest event key {pair}")
            event_keys.add(pair)
        for i, row in enumerate(residuals):
            causal = row["causal_ref"]
            if causal is not None and (row["run_id"], causal) not in event_keys:
                fail(f"residual row {i} causal_ref has no (run_id,event_id) manifest match")
        for event in manifest["events"]:
            eid = event["event_id"]
            if not re.fullmatch(r"event:(0|[1-9][0-9]*):(0|[1-9][0-9]*)", eid):
                fail("source manifest event_id is not canonical event:<index>:<generation>")
            match = re.fullmatch(r"event:(0|[1-9][0-9]*):(0|[1-9][0-9]*)", eid)
            encoded = (int(match.group(1)).to_bytes(8, "little") + int(match.group(2)).to_bytes(4, "little"))
            event["event_id_le_hex"] = encoded.hex()
        if manifest.get("source_window") != {"start_ticks": "0", "end_ticks": "20"}:
            fail("runtime fixture source window differs from frozen [0,20) case")
        join_path = directory / "join_manifest.json"
        if not join_path.is_file():
            fail("missing actual join_manifest.json")
        join = load_json(join_path)
        _validate_join_manifest(join, residuals, runs, event_keys, manifest)
        _assert_runtime_fixture(residuals, logical_metrics=load_json(directory / "metric.json"), join=join)
        return
    p = directory / "event_log.json"
    if not p.exists():
        return
    value = load_json(p)
    events = value if isinstance(value, list) else value.get("events") if isinstance(value, dict) else None
    if not isinstance(events, list):
        fail("event_log.json must be an array or an object containing events")
    schema_doc = load_json(EVENT_SCHEMA_PATH)
    fields = schema_doc["fields"]
    expected_names = [f["name"] for f in fields]
    by_run_event: set[tuple[str, str]] = set()
    for i, event in enumerate(events):
        if not isinstance(event, dict) or list(event) != expected_names:
            fail(f"event_log row {i} field order differs from unchanged event_log_v1 schema")
        run_id, event_id = event.get("run_id"), event.get("event_id")
        if not isinstance(run_id, str) or not isinstance(event_id, str):
            fail(f"event_log row {i} run_id/event_id must be strings")
        pair = (run_id, event_id)
        if pair in by_run_event:
            fail(f"duplicate event join key {pair}")
        by_run_event.add(pair)
    for i, row in enumerate(residuals):
        causal = row["causal_ref"]
        if causal is None:
            continue
        match = re.fullmatch(r"event:(0|[1-9][0-9]*):(0|[1-9][0-9]*)", causal)
        if not match:
            fail(f"residual row {i} causal_ref is not canonical event:<index>:<generation>")
        event_key = (row["run_id"], causal.removeprefix("event:"))
        if event_key not in by_run_event:
            fail(f"residual row {i} causal_ref has no (run_id,event_id) event match")


JOIN_KEYS = ("study_id", "dataset_id", "scenario_id", "seed_schedule_id", "replication_id", "case_key",
             "task_key", "occurrence", "endpoint", "seed_purpose", "seed_map_ref", "mapping_version")


def _validate_join_manifest(join: Any, residuals: list[dict[str, Any]], runs: dict[str, dict[str, Any]],
                            event_keys: set[tuple[str, str]], source_manifest: dict[str, Any]) -> None:
    if not isinstance(join, dict) or join.get("version") != "c43.join_manifest.v1":
        fail("join_manifest version must be c43.join_manifest.v1")
    counts = join.get("counts")
    rows = join.get("rows")
    if not isinstance(counts, dict) or not isinstance(rows, list):
        fail("join_manifest requires counts and rows")
    if counts.get("pairs") != len(rows) or len(rows) != len(residuals):
        fail("join_manifest pair count differs from residual records")
    if counts.get("reference_rows") != sum(_strict_count(r, "raw_reference_count") for r in rows):
        fail("join_manifest reference row count does not reconcile")
    if counts.get("simulation_rows") != sum(_strict_count(r, "raw_simulation_count") for r in rows):
        fail("join_manifest simulation row count does not reconcile")
    raw_records = []
    by_key = {}
    for i, row in enumerate(rows):
        if not isinstance(row, dict) or not isinstance(row.get("logical_key"), dict):
            fail(f"join_manifest row {i} has no logical_key object")
        key_obj = row["logical_key"]
        if set(key_obj) != set(JOIN_KEYS):
            fail(f"join_manifest row {i} logical_key differs from complete 12-field pairing key")
        if isinstance(key_obj["occurrence"], bool) or not isinstance(key_obj["occurrence"], int):
            fail(f"join_manifest row {i} occurrence must be UInt32 integer")
        key = tuple(key_obj[k] for k in JOIN_KEYS)
        if key in by_key:
            fail(f"duplicate join_manifest logical_key {key}")
        by_key[key] = row
        if not isinstance(row.get("eligible"), bool) or not isinstance(row.get("excluded"), bool):
            fail(f"join_manifest row {i} eligibility/excluded flags must be booleans")
        if row["excluded"] and row["eligible"]:
            fail(f"join_manifest row {i} cannot be both eligible and excluded")
        source_time = row.get("source_time")
        if source_time is not None:
            parse_u128(source_time, f"join_manifest row {i} source_time")
        for field_name in ("raw_reference_count", "raw_simulation_count"):
            _strict_count(row, field_name)
        raw = row.get("raw_records")
        if not isinstance(raw, list) or any(not isinstance(x, dict) for x in raw):
            fail(f"join_manifest row {i} raw_records must be an array of objects")
        raw_records.extend(raw)
        run = runs.get(row.get("run_id"))
        if run is None or row.get("candidate_id") != run.get("candidate_id"):
            fail(f"join_manifest row {i} run/candidate does not match source manifest")
        causal = row.get("causal_ref")
        if causal is not None and (row["run_id"], causal) not in event_keys:
            fail(f"join_manifest row {i} causal_ref does not resolve under (run_id,event_id)")
        event_hex = row.get("event_id_le_hex")
        if causal is not None and event_hex is None:
            fail(f"join_manifest row {i} resolved causal_ref requires event_id_le_hex")
        if event_hex is not None:
            if not isinstance(event_hex, str) or not re.fullmatch(r"[0-9a-f]{24}", event_hex):
                fail(f"join_manifest row {i} event_id_le_hex must be 12-byte lowercase hex")
            if causal is None or bytes.fromhex(event_hex) != _event_id_bytes(causal):
                fail(f"join_manifest row {i} event handle bytes disagree with causal_ref")
    if len(raw_records) != source_manifest.get("source_rows"):
        fail("join_manifest raw_records count differs from source_manifest source_rows")
    canonical_raw_rows = sorted(canonical(r) for r in raw_records)
    canonical_raw = ("[" + ",".join(canonical_raw_rows) + "]").encode("utf-8")
    digest = hashlib.sha256(canonical_raw).hexdigest()
    if counts.get("raw_rows_canonical_sha256") != digest:
        fail("join_manifest raw_rows_canonical_sha256 differs from canonical raw_records array")
    residual_keys = {}
    for row in residuals:
        key = tuple(row[k] for k in JOIN_KEYS)
        pair = (row["run_id"], row["candidate_id"])
        residual_keys.setdefault((key, pair), []).append(row)
    if len(residual_keys) != len(rows):
        fail("residual rows do not have one unique full pairing/run/candidate key")
    for (key, (run_id, candidate_id)), matches in residual_keys.items():
        joined = by_key.get(key)
        if joined is None or joined.get("run_id") != run_id or joined.get("candidate_id") != candidate_id:
            fail("residual has no exact join_manifest full-key/run/candidate match")
        if len(matches) != 1 or joined.get("residual_status") != matches[0]["residual_status"]:
            fail("join_manifest residual status/uniqueness differs from sidecar")


def _strict_count(row: dict[str, Any], name: str) -> int:
    value = row.get(name)
    if isinstance(value, bool) or not isinstance(value, int) or value < 0:
        fail(f"{name} must be a nonnegative integer")
    return value


def _event_id_bytes(causal_ref: str) -> bytes:
    match = re.fullmatch(r"event:(0|[1-9][0-9]*):(0|[1-9][0-9]*)", causal_ref)
    if not match:
        fail("causal_ref is not canonical event:<index>:<generation>")
    index, generation = int(match.group(1)), int(match.group(2))
    if index > (1 << 64) - 1 or generation > (1 << 32) - 1:
        fail("causal_ref event_id exceeds u64/u32 handle bounds")
    return index.to_bytes(8, "little") + generation.to_bytes(4, "little")


def _assert_runtime_fixture(residuals: list[dict[str, Any]], logical_metrics: Any, join: dict[str, Any]) -> None:
    """Assert coordinator-declared fixture facts against actual producer rows."""
    if len(residuals) != 8:
        fail(f"known C4.3 runtime fixture requires 8 pair residual rows, got {len(residuals)}")
    by_case: dict[str, list[dict[str, Any]]] = {}
    for row in residuals:
        by_case.setdefault(row["case_key"], []).append(row)
    cases = {
        "case-1": ("computed", "10", "12", "positive", "2"),
        "case-2": ("computed", str(U128_MAX), "0", "negative", str(U128_MAX)),
        "case-3": ("missing_observed", None, "8", "undefined", None),
        "case-4": ("censored", None, "8", "undefined", None),
        "case-5": ("probe_failed", "4", None, "undefined", None),
        "case-6": ("probe_failed", "5", None, "undefined", None),
        "case-7": ("computed", "1", "3", "positive", "2"),
        "case-8": ("computed", "7", "9", "positive", "2"),
    }
    for case, expected in cases.items():
        matches = by_case.get(case, [])
        if not matches:
            fail(f"known C4.3 runtime fixture missing {case}")
        if not any((r["residual_status"], r["observed_ticks"], r["predicted_ticks"],
                    r["residual_sign"], r["residual_magnitude"]) == expected for r in matches):
            fail(f"known C4.3 runtime fixture values differ for {case}")
    if not any(r["case_key"] == "case-8" and r["feasibility"] == "infeasible" for r in residuals):
        fail("known C4.3 fixture lost valid infeasible computed point")
    outside = [r for r in join["rows"] if r.get("logical_key", {}).get("case_key") == "case-7"]
    if not any(r.get("source_time") == "20" and r.get("eligible") is False and r.get("excluded") is True
               for r in outside):
        fail("known C4.3 fixture must retain outside-window case-7 as excluded in join manifest")
    if not isinstance(logical_metrics, list):
        logical_metrics = logical_metrics.get("records") if isinstance(logical_metrics, dict) else None
    if not isinstance(logical_metrics, list):
        fail("metric.json must contain actual canonical metric records")
    w1 = [r for r in logical_metrics if r.get("metric") == "W1" and r.get("status") == "computed"]
    ks = [r for r in logical_metrics if r.get("metric") == "KS_D" and r.get("status") == "computed"]
    if not any(r.get("value") == 1 and r.get("reference_count") == 2 and r.get("simulation_count") == 2 for r in w1):
        fail("known C4.3 fixture W1=1, n_reference=n_simulation=2 not found")
    if not any(r.get("value") == 0.5 and r.get("reference_count") == 2 and r.get("simulation_count") == 2 for r in ks):
        fail("known C4.3 fixture KS_D=0.5, n_reference=n_simulation=2 not found")
    empty = [r for r in logical_metrics if r.get("status") == "empty" and r.get("value") is None]
    if len(empty) < 2:
        fail("known C4.3 fixture must retain two empty fixed-group metric rows")
    summaries = [r for r in logical_metrics if r.get("metric") == "paired_residual_summary"]
    summary_stats = {r.get("strata", {}).get("statistic") for r in summaries}
    if summary_stats != {"bias", "mae", "rmse"}:
        fail("known C4.3 fixture must retain bias/mae/rmse paired summary rows")
    if not any(r.get("metric") == "paired_residual_summary" and r.get("excluded_count", 0) >= 1
               for r in logical_metrics):
        fail("known C4.3 paired summary must count the excluded outside-window point")


def readback(directory: Path) -> dict[str, Any]:
    directory = directory.resolve()
    if hashlib.sha256(C0_SCHEMA_PATH.read_bytes()).hexdigest() != C0_SCHEMA_SHA256:
        fail("C0 logical schema digest changed from packet-bound source")
    if hashlib.sha256(EVENT_SCHEMA_PATH.read_bytes()).hexdigest() != EVENT_SCHEMA_SHA256:
        fail("legacy event_log_v1 schema digest changed from packet-bound source")
    logical = {}
    evidence: dict[str, Any] = {"directory": str(directory), "formats": {}, "counts": {}}
    for kind in ("residual", "metric"):
        logical_path = directory / f"{kind}.json"
        if not logical_path.is_file():
            fail(f"missing actual producer logical record file: {logical_path.name}")
        logical_bytes = logical_path.read_bytes()
        payload = json.loads(logical_bytes, parse_constant=lambda v: fail(f"invalid JSON constant {v}"))
        records = payload if isinstance(payload, list) else None
        if not isinstance(records, list):
            fail(f"{logical_path.name} must contain an array of actual C0 records")
        if logical_bytes not in (canonical(records).encode(), (canonical(records) + "\n").encode()):
            fail(f"{logical_path.name} is not a canonical JSON array")
        validate_logical(records, kind)
        logical[kind] = records
        evidence["counts"][kind] = len(records)
        for fmt in FORMATS:
            path = directory / f"{kind}.{fmt}"
            if not path.is_file():
                fail(f"missing actual Rust {fmt} output: {path.name}")
            table = read_file(path, fmt)
            verify_physical(table, kind, records)
            evidence["formats"][path.name] = {"rows": table.num_rows, "sha256": hashlib.sha256(path.read_bytes()).hexdigest()}
    check_event_joins(directory, logical["residual"])
    evidence["event_join"] = "checked" if (directory / "join_manifest.json").exists() or (directory / "source_manifest.json").exists() else "not_supplied"
    evidence["legacy_event_schema_sha256"] = EVENT_SCHEMA_SHA256
    evidence["c0_logical_schema_sha256"] = hashlib.sha256(C0_SCHEMA_PATH.read_bytes()).hexdigest()
    evidence["pyarrow"] = pa.__version__
    evidence["python"] = sys.version
    return evidence


def _fixture_rows() -> tuple[list[dict[str, Any]], list[dict[str, Any]]]:
    m = "a" * 64
    residual = {
        "record_type": "calibration_residual.v1", "schema_version": "calibration-v1", "dataset_id": "d",
        "scenario_id": "s", "run_id": "r", "candidate_id": "c", "case_key": "case", "task_key": "task",
        "occurrence": 0, "endpoint": "departure", "fidelity": "FreeRunning", "observed_ticks": "0",
        "predicted_ticks": str(U128_MAX), "residual_status": "computed", "residual_sign": "positive",
        "residual_magnitude": str(U128_MAX), "prediction_unclamped": True, "anchor_role": "none",
        "feasibility": "feasible", "censor_status": "not_censored", "study_id": "study",
        "replication_id": "rep", "seed_schedule_id": "sched", "seed_purpose": "calibration",
        "seed_map_ref": "mapref", "seed_contract_version": "seed-v1", "mapping_version": "map-v1",
        "parameter_hash": m, "graph_hash": None, "causal_ref": None,
    }
    metric = {
        "record_type": "calibration_metric.v1", "schema_version": "calibration-v1", "metric": "W1",
        "algorithm_version": "empirical_equal.v1", "endpoint": "departure", "strata": {"arm": "candidate"},
        "window": {"start_ticks": "0", "end_ticks": str(U128_MAX)}, "units": "1ns",
        "reference_count": 1, "simulation_count": 1, "excluded_count": 0, "censored_count": 0,
        "missing_count": 0, "unmatched_count": 0, "failed_count": 0, "infeasible_count": 0,
        "value": float(U128_MAX), "status": "computed", "uncertainty": None,
        "provenance": {"dataset_id": "d", "run_id": "r", "mapping_version": "map-v1",
                       "seed_schedule_id": "sched", "seed_map_ref": "mapref",
                       "seed_contract_version": "seed-v1", "parameter_hash": m},
    }
    validate_logical([residual], "residual")
    validate_logical([metric], "metric")
    return [residual], [metric]


def _self_test() -> dict[str, Any]:
    residuals, metrics = _fixture_rows()
    expected = {"residual": residuals, "metric": metrics}
    with tempfile.TemporaryDirectory(prefix="c43-readback-") as temp:
        root = Path(temp)
        for kind, records in expected.items():
            table = _rows_to_table(records, kind)
            for fmt in FORMATS:
                write_table(table, root / f"{kind}.{fmt}", fmt)
            (root / f"{kind}.json").write_text(canonical(records) + "\n")
        readback(root)
        controls = []
        # Wrong field type/order/metadata must fail exact-schema comparison.
        table = _rows_to_table(residuals, "residual")
        bad = table.set_column(8, "occurrence", pa.array([0], type=pa.uint64()))
        write_table(bad, root / "residual.ipc_file", "ipc_file")
        rejection = _expect_reject(lambda: readback(root), "wrong_type")
        controls.append({"id": "wrong_type", "file": "residual.ipc_file", "rejection": rejection})
        write_table(table, root / "residual.ipc_file", "ipc_file")
        # A corrupt fixed-width integer payload must fail logical readback.
        flat = logical_flat(residuals[0], "residual")
        flat["residual_magnitude"] = b"\x01" * 16
        corrupt = pa.Table.from_pylist([flat], schema=SCHEMAS["residual"])
        write_table(corrupt, root / "residual.ipc_stream", "ipc_stream")
        rejection = _expect_reject(lambda: readback(root), "mutated_u128")
        controls.append({"id": "mutated_u128", "file": "residual.ipc_stream", "rejection": rejection})
        write_table(table, root / "residual.ipc_stream", "ipc_stream")
        # Null in a required field is preserved by Arrow and rejected on logical/schema admission.
        flat = logical_flat(residuals[0], "residual")
        flat["dataset_id"] = None
        relaxed_fields = [f.with_nullable(True) if f.name == "dataset_id" else f
                          for f in SCHEMAS["residual"]]
        relaxed_schema = pa.schema(relaxed_fields, metadata=SCHEMAS["residual"].metadata)
        nullable_violation = pa.Table.from_pylist([flat], schema=relaxed_schema)
        write_table(nullable_violation, root / "residual.parquet", "parquet")
        rejection = _expect_reject(lambda: readback(root), "required_null")
        controls.append({"id": "required_null", "file": "residual.parquet", "rejection": rejection})
        write_table(table, root / "residual.parquet", "parquet")
        # Schema metadata mutation must be detected independently of values.
        wrong_meta = table.replace_schema_metadata(md(format="wrong", physical_version="1"))
        write_table(wrong_meta, root / "residual.parquet", "parquet")
        rejection = _expect_reject(lambda: readback(root), "wrong_metadata")
        controls.append({"id": "wrong_metadata", "file": "residual.parquet", "rejection": rejection})
    return {"status": "pass", "pyarrow": pa.__version__, "logical_schema_validation": "bounded evaluator for unchanged C0 residual/metric definitions; fails closed on unsupported keywords",
            "controls": controls,
            "positive_oracle": "typed Arrow IPC file/stream and Parquet tables match independently frozen schemas and canonical logical records; residual 0-to-u128-max exact difference retained"}


def _expect_reject(action: Any, label: str) -> str:
    try:
        action()
    except Exception as exc:
        return f"{type(exc).__name__}: {exc}"
    fail(f"negative control unexpectedly accepted: {label}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("directory", nargs="?", type=Path)
    args = parser.parse_args()
    try:
        result = _self_test() if args.self_test else readback(args.directory) if args.directory else fail("directory is required unless --self-test")
        print(json.dumps(result, sort_keys=True, indent=2))
        return 0
    except Exception as exc:
        print(f"FAIL: {type(exc).__name__}: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
