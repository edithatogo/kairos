#!/usr/bin/env python3
"""Optional physical-v2 bridge for actual C1.3 pipeline captures.

This script is an interoperability helper, not part of the Rust runtime. The
capture populations are always the expected logical rows; files emitted by
Rust are read back and compared with those rows before raw source rows are
reconstructed from the transported data.
"""
from __future__ import annotations

import argparse
import collections
import hashlib
import importlib.util
import json
from pathlib import Path
import sys
from typing import Any, NoReturn

ROOT = Path(__file__).resolve().parents[2]
CODEC_PATH = ROOT / "conformance" / "c11" / "physical_schema.py"
SPEC = importlib.util.spec_from_file_location("c11_physical_schema", CODEC_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"cannot load physical codec: {CODEC_PATH}")
physical = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = physical
SPEC.loader.exec_module(physical)

import pyarrow as pa
import pyarrow.ipc as ipc
import pyarrow.parquet as parquet

TABLES = {
    "trace_event.v1": ("events", "events.ndjson"),
    "trace_exclusion.v1": ("exclusions", "exclusions.ndjson"),
    "outcome_observation.v1": ("outcomes", "outcomes.ndjson"),
}
FORMATS = ("ipc_file", "ipc_stream", "parquet")
LAYOUTS = (1, 2, 3)


def fail(message: str) -> NoReturn:
    raise ValueError(message)


def canonical(value: Any) -> str:
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"), allow_nan=False)


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def read_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"), parse_constant=lambda token: fail(f"invalid JSON constant {token}"))
    except (OSError, UnicodeError, json.JSONDecodeError, ValueError) as exc:
        raise ValueError(f"cannot read valid JSON from {path}: {exc}") from exc


def read_ndjson(path: Path) -> list[dict[str, Any]]:
    rows = []
    try:
        with path.open("r", encoding="utf-8") as stream:
            for number, line in enumerate(stream, 1):
                if not line.strip():
                    raise ValueError(f"blank NDJSON line at {path}:{number}")
                value = json.loads(line, parse_constant=lambda token: fail(f"invalid JSON constant {token}"))
                if not isinstance(value, dict):
                    raise ValueError(f"NDJSON row at {path}:{number} is not an object")
                rows.append(value)
    except (OSError, UnicodeError, json.JSONDecodeError, ValueError) as exc:
        raise ValueError(f"cannot read valid NDJSON from {path}: {exc}") from exc
    return rows


def load_capture(capture_dir: Path) -> dict[str, Any]:
    required = ("template.json", "source.json", "manifest.json", "events.ndjson", "exclusions.ndjson", "outcomes.ndjson")
    missing = [name for name in required if not (capture_dir / name).is_file()]
    if missing:
        fail(f"capture missing required files: {', '.join(missing)}")
    template = read_json(capture_dir / "template.json")
    source = read_json(capture_dir / "source.json")
    manifest = read_json(capture_dir / "manifest.json")
    if not isinstance(template, dict) or not isinstance(source, list) or any(not isinstance(row, dict) for row in source):
        fail("template.json must be an object and source.json an array of objects")
    if not isinstance(manifest, dict) or manifest.get("manifest_version") != "c1.ingestion.v1":
        fail("manifest.json is not a c1.ingestion.v1 capture")
    if manifest.get("candidate_conservation") is not True or manifest.get("physical_schema_version") != 2:
        fail("capture manifest lacks candidate conservation or physical schema v2")
    if manifest.get("source_rows") != len(source):
        fail("manifest source_rows differs from source.json")
    manifest_populations = manifest.get("populations")
    if not isinstance(manifest_populations, dict):
        fail("manifest populations must be an object")

    populations: dict[str, list[dict[str, Any]]] = {}
    evidence: dict[str, Any] = {}
    for record_type, (population_name, filename) in TABLES.items():
        path = capture_dir / filename
        rows = read_ndjson(path)
        declared = manifest_populations.get(population_name)
        if not isinstance(declared, dict) or declared.get("rows") != len(rows):
            fail(f"manifest row count mismatch for {population_name}")
        if declared.get("sha256") != sha256_file(path):
            fail(f"manifest hash mismatch for {population_name}")
        for index, row in enumerate(rows):
            if row.get("record_type") != record_type:
                fail(f"unexpected record_type in {filename} row {index + 1}")
            if not isinstance(row.get("dataset_id"), str):
                fail(f"missing dataset_id in {filename} row {index + 1}")
        populations[record_type] = rows
        evidence[population_name] = {"rows": len(rows), "sha256": sha256_file(path)}

    if not populations["trace_event.v1"]:
        fail("capture has zero events; refusing vacuous transport proof")
    return {"template": template, "source": source, "manifest": manifest,
            "populations": populations, "population_evidence": evidence}


def ensure_new(path: Path, data: bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    try:
        with path.open("xb") as stream:
            stream.write(data)
    except FileExistsError as exc:
        raise ValueError(f"refusing to overwrite existing output: {path}") from exc


def write_arrow(table: pa.Table, path: Path, fmt: str, batch_rows: int, row_group_rows: int) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    try:
        with path.open("xb") as sink:
            if fmt == "ipc_file":
                with ipc.new_file(sink, table.schema) as writer:
                    writer.write_table(table, max_chunksize=batch_rows)
            elif fmt == "ipc_stream":
                with ipc.new_stream(sink, table.schema) as writer:
                    writer.write_table(table, max_chunksize=batch_rows)
            elif fmt == "parquet":
                parquet.write_table(table, sink, row_group_size=row_group_rows, compression="NONE")
            else:
                raise ValueError(f"unsupported physical format {fmt}")
    except FileExistsError as exc:
        raise ValueError(f"refusing to overwrite existing output: {path}") from exc


def encode(args: argparse.Namespace) -> dict[str, Any]:
    capture_dir = Path(args.capture_dir).resolve()
    output_dir = Path(args.input_dir).resolve()
    if args.batch_rows <= 0 or args.row_group_rows <= 0:
        fail("batch and row-group sizes must be positive")
    capture = load_capture(capture_dir)
    output_dir.mkdir(parents=True, exist_ok=True)
    populations = capture["populations"]
    output_hashes: dict[str, str] = {}
    row_counts: dict[str, int] = {}
    for record_type, rows in populations.items():
        ordered = list(reversed(rows)) if args.reverse else rows
        table = physical.table_from_rows(record_type, ordered)
        if not table.schema.equals(physical.SCHEMAS[record_type], check_metadata=True):
            fail(f"physical codec produced an unexpected schema for {record_type}")
        for fmt in FORMATS:
            path = output_dir / f"{record_type.split('.')[0]}.{fmt}"
            write_arrow(table, path, fmt, args.batch_rows, args.row_group_rows)
            output_hashes[path.name] = sha256_file(path)
        row_counts[record_type] = len(rows)
    report = {
        "classification": "actual_capture_physical_encoding_only",
        "capture_dir": str(capture_dir),
        "input_dir": str(output_dir),
        "template_sha256": sha256_file(capture_dir / "template.json"),
        "source_sha256": sha256_file(capture_dir / "source.json"),
        "manifest_sha256": sha256_file(capture_dir / "manifest.json"),
        "population_evidence": capture["population_evidence"],
        "row_counts": row_counts,
        "batch_rows": args.batch_rows,
        "row_group_rows": args.row_group_rows,
        "reverse": args.reverse,
        "outputs_sha256": output_hashes,
    }
    return report


def read_arrow(path: Path, fmt: str) -> pa.Table:
    try:
        if fmt == "ipc_file":
            with pa.memory_map(str(path), "r") as source:
                return ipc.open_file(source).read_all()
        if fmt == "ipc_stream":
            with pa.memory_map(str(path), "r") as source:
                return ipc.open_stream(source).read_all()
        if fmt == "parquet":
            return parquet.ParquetFile(path).read()
    except (OSError, pa.ArrowException) as exc:
        raise ValueError(f"cannot read transport file {path}: {exc}") from exc
    raise ValueError(f"unsupported physical format {fmt}")


def row_counter(rows: list[dict[str, Any]]) -> collections.Counter[str]:
    try:
        return collections.Counter(canonical(row) for row in rows)
    except (TypeError, ValueError) as exc:
        raise ValueError(f"logical rows are not canonical JSON values: {exc}") from exc


def verify_table(path: Path, fmt: str, record_type: str) -> list[dict[str, Any]]:
    table = read_arrow(path, fmt)
    schema = physical.SCHEMAS[record_type]
    if not table.schema.equals(schema, check_metadata=True):
        fail(f"exact schema or metadata mismatch in {path}")
    try:
        rows = [physical.decode_row(row, record_type) for row in table.to_pylist()]
    except (TypeError, ValueError, pa.ArrowException) as exc:
        raise ValueError(f"physical decode failed for {path}: {exc}") from exc
    return rows


def binding_order_fields(template: dict[str, Any]) -> list[str]:
    bindings = template.get("event_bindings")
    if not isinstance(bindings, list) or not bindings:
        fail("template event_bindings must be a nonempty array")
    fields = []
    for binding in bindings:
        if not isinstance(binding, dict) or not isinstance(binding.get("order_field"), str):
            fail("each template event binding must declare order_field")
        if binding["order_field"] not in fields:
            fields.append(binding["order_field"])
    return fields


def reconstruct_source(rows_by_type: dict[str, list[dict[str, Any]]], template: dict[str, Any]) -> list[dict[str, Any]]:
    case_field = template.get("case_key_field")
    if not isinstance(case_field, str) or not case_field:
        fail("template case_key_field must be nonempty text")
    order_fields = binding_order_fields(template)
    recovered: dict[tuple[str, str], tuple[dict[str, Any], int | None]] = {}
    keys_seen: dict[str, tuple[str, str]] = {}

    for record_type in ("trace_event.v1", "trace_exclusion.v1"):
        for row in rows_by_type[record_type]:
            raw_event = row.get("raw_event")
            if not isinstance(raw_event, dict) or not isinstance(raw_event.get("source_fields"), dict):
                fail(f"transported {record_type} row lacks raw_event.source_fields")
            raw = raw_event["source_fields"]
            key = row.get("source_event_key")
            source_id = raw_event.get("source_record_id")
            if not isinstance(key, str) or not key:
                fail(f"transported {record_type} row lacks source_event_key")
            if source_id is not None and not isinstance(source_id, str):
                fail(f"source_record_id is not text for {key}")
            case = row.get("case_key")
            if case is None:
                case = raw.get(case_field)
            if not isinstance(case, str) or not case:
                fail(f"cannot recover case identity for source event {key}")
            raw_identity = canonical(raw)
            previous_key_row = keys_seen.get(key)
            if previous_key_row is not None and previous_key_row != (case, raw_identity):
                fail(f"dataset-wide source key {key!r} maps to inconsistent case/raw rows")
            keys_seen[key] = (case, raw_identity)
            identity = (case, f"id:{source_id}" if source_id else f"row:{raw_identity}")

            order = row.get("source_order")
            raw_orders = []
            for field in order_fields:
                if field in raw:
                    candidate = raw[field]
                    if isinstance(candidate, bool) or not isinstance(candidate, int) or candidate < 0:
                        fail(f"raw source order field {field} is not a nonnegative integer")
                    raw_orders.append(candidate)
            if len(set(raw_orders)) > 1:
                fail(f"raw source order fields disagree for {key}")
            if raw_orders:
                candidate = raw_orders[0]
                if order is not None and candidate != order:
                    fail(f"raw source order disagrees with mapped source_order for {key}")
                order = candidate
            if order is not None and (isinstance(order, bool) or not isinstance(order, int) or order < 0):
                fail(f"invalid transported source_order for {key}")

            previous = recovered.get(identity)
            if previous is None:
                recovered[identity] = (raw, order)
            else:
                previous_raw, previous_order = previous
                if canonical(previous_raw) != canonical(raw):
                    fail(f"source identity {identity!r} maps to inconsistent raw rows")
                if previous_order is not None and order is not None and previous_order != order:
                    fail(f"source identity {identity!r} has inconsistent source_order")
                recovered[identity] = (previous_raw, previous_order if previous_order is not None else order)

    if not recovered:
        fail("Rust transport readback produced no source rows")
    ordered = sorted(
        ((identity, raw, order) for identity, (raw, order) in recovered.items()),
        key=lambda item: (item[2] is None, item[2] if item[2] is not None else 0, item[0][0].encode("utf-8"), item[0][1].encode("utf-8")),
    )
    return [raw for _, raw, _ in ordered]


def verify(args: argparse.Namespace) -> dict[str, Any]:
    capture_dir = Path(args.capture_dir).resolve()
    rust_output = Path(args.rust_output_dir).resolve()
    reingest_rows_path = Path(args.reingest_rows).resolve()
    capture = load_capture(capture_dir)
    expected_names = {
        f"{record_type}.reversed.limit-{limit}.{fmt}"
        for record_type in TABLES
        for limit in LAYOUTS
        for fmt in FORMATS
    }
    missing = sorted(name for name in expected_names if not (rust_output / name).is_file())
    if missing:
        fail(f"Rust readback directory is missing expected transport files: {', '.join(missing)}")

    expected = capture["populations"]
    file_hashes: dict[str, str] = {}
    counts: dict[str, Any] = {}
    selected_rows: dict[str, list[dict[str, Any]]] = {}
    for record_type, actual_rows in expected.items():
        expected_counter = row_counter(actual_rows)
        layout_counts: dict[str, int] = {}
        for limit in LAYOUTS:
            for fmt in FORMATS:
                name = f"{record_type}.reversed.limit-{limit}.{fmt}"
                path = rust_output / name
                decoded = verify_table(path, fmt, record_type)
                if len(decoded) != len(actual_rows):
                    fail(f"row count mismatch in {name}: {len(decoded)} != {len(actual_rows)}")
                if row_counter(decoded) != expected_counter:
                    fail(f"decoded transport payload differs from actual capture in {name}")
                layout_counts[f"limit-{limit}.{fmt}"] = len(decoded)
                file_hashes[name] = sha256_file(path)
                if limit == LAYOUTS[0] and fmt == FORMATS[0]:
                    selected_rows[record_type] = decoded
        counts[record_type] = {"rows": len(actual_rows), "layouts": layout_counts}

    transported_source = reconstruct_source(selected_rows, capture["template"])
    original_source = capture["source"]
    if row_counter(transported_source) != row_counter(original_source):
        fail("source rows reconstructed from Rust transport do not match original source.json")
    ensure_new(
        reingest_rows_path,
        (json.dumps(transported_source, ensure_ascii=False, separators=(",", ":"), allow_nan=False) + "\n").encode("utf-8"),
    )

    return {
        "classification": "actual_capture_rust_transport_readback_verified",
        "capture_dir": str(capture_dir),
        "rust_output_dir": str(rust_output),
        "reingest_rows": str(reingest_rows_path),
        "manifest_sha256": sha256_file(capture_dir / "manifest.json"),
        "source_sha256": sha256_file(capture_dir / "source.json"),
        "counts": counts,
        "transport_files_sha256": file_hashes,
        "transported_source_rows": len(transported_source),
        "transported_source_sha256": sha256_bytes((json.dumps(transported_source, ensure_ascii=False, separators=(",", ":"), allow_nan=False) + "\n").encode("utf-8")),
    }


def positive_int(raw: str) -> int:
    try:
        value = int(raw)
    except ValueError as exc:
        raise argparse.ArgumentTypeError("must be an integer") from exc
    if value <= 0:
        raise argparse.ArgumentTypeError("must be positive")
    return value


def parser() -> argparse.ArgumentParser:
    cli = argparse.ArgumentParser(description=__doc__)
    commands = cli.add_subparsers(dest="command", required=True)
    encode_parser = commands.add_parser("encode", help="encode captured actual rows to physical-v2 files")
    encode_parser.add_argument("capture_dir")
    encode_parser.add_argument("input_dir", help="new output directory for the 9 physical input files")
    encode_parser.add_argument("--batch-rows", type=positive_int, required=True)
    encode_parser.add_argument("--row-group-rows", type=positive_int, required=True)
    encode_parser.add_argument("--reverse", action="store_true", help="reverse each actual capture population before encoding")
    verify_parser = commands.add_parser("verify", help="verify Rust transport files and reconstruct raw rows from them")
    verify_parser.add_argument("capture_dir")
    verify_parser.add_argument("rust_output_dir")
    verify_parser.add_argument("reingest_rows", help="new JSON array path for rows decoded from Rust transport")
    return cli


def main(argv: list[str] | None = None) -> int:
    cli = parser()
    args = cli.parse_args(argv)
    try:
        report = encode(args) if args.command == "encode" else verify(args)
    except (OSError, ValueError, pa.ArrowException) as exc:
        print(json.dumps({"status": "failed", "error": str(exc)}, ensure_ascii=False), file=sys.stderr)
        return 2
    print(json.dumps({"status": "pass", **report}, ensure_ascii=False, sort_keys=True, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
