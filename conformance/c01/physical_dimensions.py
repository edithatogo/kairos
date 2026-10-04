#!/usr/bin/env python3
"""Qualify physical-v2 layouts against actual C01 pipeline captures.

The runner is optional evidence tooling. It uses the pinned C11 physical codec,
the C13 Arrow writer helper, and the existing Rust ingestion transport test.
It does not add a runtime dependency or public API.
"""
from __future__ import annotations

import argparse
import collections
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
from typing import Any, NoReturn

ROOT = Path(__file__).resolve().parents[2]
INTEROP_PATH = ROOT / "conformance" / "c13" / "interop.py"
SPEC = importlib.util.spec_from_file_location("c13_physical_interop", INTEROP_PATH)
if SPEC is None or SPEC.loader is None:
    raise RuntimeError(f"cannot load existing C13 physical bridge: {INTEROP_PATH}")
interop = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = interop
SPEC.loader.exec_module(interop)
physical = interop.physical

PROFILES = {
    "long-valid": {"source_rows": 7, "candidate_units": 7, "events": 6,
                   "quarantine": 0, "exclusions": 1, "outcomes": 2},
    "wide-valid": {"source_rows": 3, "candidate_units": 9, "events": 6,
                   "quarantine": 0, "exclusions": 3, "outcomes": 2},
    "long-quarantine": {"source_rows": 7, "candidate_units": 7, "events": 3,
                         "quarantine": 3, "exclusions": 1, "outcomes": 2},
}
FORMATS = ("ipc_file", "ipc_stream", "parquet")
PHYSICAL_TYPES = {
    "trace_event.v1": ("trace_event", ("events", "quarantine")),
    "trace_exclusion.v1": ("trace_exclusion", ("exclusions",)),
    "outcome_observation.v1": ("outcome_observation", ("outcomes",)),
}
LAYOUTS = tuple(
    (batch_rows, row_group_rows, row_order)
    for batch_rows in (1, 2)
    for row_group_rows in (1, 3)
    for row_order in ("forward", "reverse")
)
WRITER_LIMITS = (1, 2, 3)
RECEIPT_NAMES = {
    "baseline.json", "baseline-receipt.json", "baseline_receipt.json",
    "baseline-result.json", "baseline_result.json", "result.json",
}


class QualificationError(ValueError):
    """Fail-closed C01 evidence or transport mismatch."""


def fail(message: str) -> NoReturn:
    raise QualificationError(message)


def canonical(value: Any) -> str:
    return json.dumps(value, ensure_ascii=False, sort_keys=True,
                      separators=(",", ":"), allow_nan=False)


def digest_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def digest_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def digest_ordered(rows: list[dict[str, Any]]) -> str:
    data = ("\n".join(canonical(row) for row in rows) + "\n").encode("utf-8")
    return digest_bytes(data)


def digest_multiset(rows: list[dict[str, Any]]) -> str:
    ordered = sorted(canonical(row) for row in rows)
    return digest_bytes(("\n".join(ordered) + "\n").encode("utf-8"))


def duplicate_checked(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            fail(f"duplicate JSON object member: {key}")
        result[key] = value
    return result


def strict_json_bytes(data: bytes, origin: str) -> Any:
    try:
        return json.loads(
            data.decode("utf-8"),
            object_pairs_hook=duplicate_checked,
            parse_constant=lambda value: fail(f"invalid JSON constant {value} in {origin}"),
        )
    except (UnicodeError, json.JSONDecodeError, QualificationError) as exc:
        raise QualificationError(f"invalid strict JSON at {origin}: {exc}") from exc


def read_json(path: Path) -> Any:
    try:
        return strict_json_bytes(path.read_bytes(), str(path))
    except OSError as exc:
        raise QualificationError(f"cannot read {path}: {exc}") from exc


def read_ndjson(path: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    try:
        raw = path.read_bytes()
    except OSError as exc:
        raise QualificationError(f"cannot read {path}: {exc}") from exc
    try:
        text = raw.decode("utf-8")
    except UnicodeError as exc:
        raise QualificationError(f"invalid UTF-8 at {path}: {exc}") from exc
    if raw and not raw.endswith(b"\n"):
        fail(f"NDJSON final LF is missing: {path}")
    for number, line in enumerate(text.splitlines(), 1):
        if not line:
            fail(f"blank NDJSON line at {path}:{number}")
        value = strict_json_bytes(line.encode("utf-8"), f"{path}:{number}")
        if not isinstance(value, dict):
            fail(f"NDJSON row is not an object at {path}:{number}")
        rows.append(value)
    return rows


def row_counter(rows: list[dict[str, Any]]) -> collections.Counter[str]:
    return collections.Counter(canonical(row) for row in rows)


def baseline_receipt(profile_dir: Path) -> tuple[Path, dict[str, Any]]:
    candidates = [
        path for path in profile_dir.iterdir()
        if path.is_file() and (
            path.name in RECEIPT_NAMES
            or ("baseline" in path.stem.lower() and path.suffix == ".json")
        )
    ]
    if len(candidates) != 1:
        fail(f"{profile_dir} must contain exactly one baseline receipt; found {len(candidates)}")
    value = read_json(candidates[0])
    if not isinstance(value, dict):
        fail(f"baseline receipt must be a JSON object: {candidates[0]}")
    if isinstance(value.get("status"), str) and value["status"].lower() not in {"pass", "passed", "success"}:
        fail(f"baseline receipt is not successful: {candidates[0]}")
    return candidates[0], value


def validate_baseline(profile: str, profile_dir: Path) -> dict[str, Any]:
    required = (
        "template.json", "source.json", "manifest.json", "events.ndjson",
        "quarantine.ndjson", "exclusions.ndjson", "outcomes.ndjson",
        "diagnostics.ndjson", "config.json",
    )
    missing = [name for name in required if not (profile_dir / name).is_file()]
    if missing:
        fail(f"{profile} capture missing: {', '.join(missing)}")
    template = read_json(profile_dir / "template.json")
    source = read_json(profile_dir / "source.json")
    manifest = read_json(profile_dir / "manifest.json")
    config = read_json(profile_dir / "config.json")
    if not isinstance(template, dict) or not isinstance(source, list) or any(not isinstance(r, dict) for r in source):
        fail(f"{profile} template/source has the wrong JSON shape")
    if not isinstance(manifest, dict) or manifest.get("manifest_version") != "c1.ingestion.v1":
        fail(f"{profile} does not have a c1.ingestion.v1 manifest")
    if not isinstance(config, dict) or not config:
        fail(f"{profile} config.json must contain frozen validation-policy evidence")
    receipt_path, receipt = baseline_receipt(profile_dir)
    if receipt.get("profile") != profile:
        fail(f"{profile} baseline receipt names a different profile")
    source_ndjson = ("".join(canonical(row) + "\n" for row in source)).encode("utf-8")
    if receipt.get("source_sha256") != digest_bytes(source_ndjson) or receipt.get("source_bytes") != len(source_ndjson):
        fail(f"{profile} baseline receipt source SHA/bytes differ from canonical source NDJSON")
    if manifest.get("candidate_conservation") is not True or manifest.get("physical_schema_version") != 2:
        fail(f"{profile} manifest lacks candidate conservation or physical schema v2")
    if len(source) > 100:
        fail(f"{profile} source rows exceed the frozen 100-row bound")
    raw_source = (json.dumps(source, ensure_ascii=False, sort_keys=True,
                             separators=(",", ":"), allow_nan=False) + "\n").encode("utf-8")
    if len(raw_source) > 1024 * 1024:
        fail(f"{profile} raw source exceeds the frozen 1 MiB bound")
    expected = PROFILES[profile]
    if manifest.get("source_rows") != expected["source_rows"] or len(source) != expected["source_rows"]:
        fail(f"{profile} source row count differs from the frozen profile")
    if manifest.get("candidate_units") != expected["candidate_units"]:
        fail(f"{profile} candidate count differs from the frozen profile")
    for name, count in expected.items():
        if name in {"source_rows", "candidate_units"}:
            continue
        declared = manifest.get("populations", {}).get(name, {}).get("rows")
        rows = read_ndjson(profile_dir / f"{name}.ndjson")
        if len(rows) != count or declared != count:
            fail(f"{profile} {name} population does not match the frozen count {count}")
        population_path = profile_dir / f"{name}.ndjson"
        digest = digest_file(population_path)
        byte_count = population_path.stat().st_size
        population = manifest["populations"][name]
        if population.get("sha256") != digest or population.get("bytes") != byte_count:
            fail(f"{profile} manifest population evidence mismatch for {name}")
        receipt_population = receipt.get("populations", {}).get(name)
        if not isinstance(receipt_population, dict) or receipt_population.get("sha256") != digest \
                or receipt_population.get("bytes") != byte_count or receipt_population.get("rows") != count:
            fail(f"{profile} baseline receipt population evidence mismatch for {name}")
        for index, row in enumerate(rows):
            if not isinstance(row.get("record_type"), str) or not isinstance(row.get("dataset_id"), str):
                fail(f"{profile} {name} row {index} lacks record_type/dataset_id")
            if row["dataset_id"] != manifest.get("dataset_id"):
                fail(f"{profile} {name} row {index} dataset_id differs from manifest")
            if name != "quarantine" and row["record_type"] != {
                "events": "trace_event.v1", "exclusions": "trace_exclusion.v1",
                "outcomes": "outcome_observation.v1",
            }[name]:
                fail(f"{profile} {name} row {index} has wrong record_type")
            if name == "quarantine" and row["record_type"] != "trace_event.v1":
                fail(f"{profile} quarantine row {index} is not a trace_event.v1 record")
            # The pinned codec performs independent exact C0 logical validation.
            physical.encode_row(row)
    diagnostics = read_ndjson(profile_dir / "diagnostics.ndjson")
    diagnostic_receipt = receipt.get("diagnostics")
    diagnostic_bytes = (profile_dir / "diagnostics.ndjson").stat().st_size
    if not isinstance(diagnostic_receipt, dict) \
            or diagnostic_receipt.get("sha256") != digest_file(profile_dir / "diagnostics.ndjson") \
            or diagnostic_receipt.get("bytes") != diagnostic_bytes \
            or diagnostic_receipt.get("rows") != len(diagnostics):
        fail(f"{profile} baseline receipt diagnostics evidence mismatch")
    reasons = collections.Counter(str(row.get("reason", "unspecified")) for row in diagnostics)
    conservation = sum(manifest.get(key, 0) for key in (
        "mapper_accepted_units", "mapper_excluded_units", "failed_units", "unresolved_units"))
    if conservation != manifest.get("candidate_units"):
        fail(f"{profile} candidate partition does not conserve candidates")
    validation = manifest.get("validation")
    if not isinstance(validation, dict):
        fail(f"{profile} manifest validation accounting is missing")
    if validation.get("input_events") != manifest.get("mapper_accepted_units"):
        fail(f"{profile} mapper/validation event counts differ")
    if validation.get("valid_events", 0) + validation.get("quarantined_events", 0) != validation.get("input_events"):
        fail(f"{profile} valid/quarantine event accounting does not reconcile")
    if validation.get("valid_events") != expected["events"] or validation.get("quarantined_events") != expected["quarantine"]:
        fail(f"{profile} validation event counts differ from the frozen profile")
    if manifest.get("outcomes") != expected["outcomes"]:
        fail(f"{profile} outcome accounting differs from the frozen profile")
    for record_type, population_name in (
        ("trace_event.v1", "events"), ("trace_event.v1", "quarantine"),
        ("trace_exclusion.v1", "exclusions"),
        ("outcome_observation.v1", "outcomes"),
    ):
        if record_type not in physical.SCHEMAS:
            fail(f"C0 codec lacks {record_type}")
        for row in read_ndjson(profile_dir / f"{population_name}.ndjson"):
            roundtrip = physical.decode_row(physical.encode_row(row), record_type)
            if roundtrip != row:
                fail(f"{profile} C0 codec roundtrip mismatch in {population_name}")
    return {
        "profile": profile,
        "capture_dir": str(profile_dir.resolve()),
        "template": template,
        "source": source,
        "manifest": manifest,
        "config_sha256": digest_file(profile_dir / "config.json"),
        "receipt_path": str(receipt_path.resolve()),
        "receipt_sha256": digest_file(receipt_path),
        "source_sha256": digest_file(profile_dir / "source.json"),
        "population_hashes": {name: digest_file(profile_dir / f"{name}.ndjson")
                              for name in ("events", "quarantine", "exclusions", "outcomes")},
        "diagnostics_rows": len(diagnostics),
        "diagnostic_reasons": dict(sorted(reasons.items())),
    }


def read_physical(path: Path, fmt: str, record_type: str) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    expected_schema = physical.SCHEMAS[record_type]
    batch_rows: list[int] = []
    group_rows: list[int] = []
    try:
        if fmt == "ipc_file":
            with interop.pa.memory_map(str(path), "r") as source:
                reader = interop.ipc.open_file(source)
                if not reader.schema.equals(expected_schema, check_metadata=True):
                    fail(f"exact IPC-file schema/metadata mismatch: {path}")
                batches = [reader.get_batch(index) for index in range(reader.num_record_batches)]
            batch_rows = [batch.num_rows for batch in batches]
            physical_rows = [row for batch in batches for row in batch.to_pylist()]
        elif fmt == "ipc_stream":
            with interop.pa.memory_map(str(path), "r") as source:
                reader = interop.ipc.open_stream(source)
                if not reader.schema.equals(expected_schema, check_metadata=True):
                    fail(f"exact IPC-stream schema/metadata mismatch: {path}")
                batches = list(reader)
            batch_rows = [batch.num_rows for batch in batches]
            physical_rows = [row for batch in batches for row in batch.to_pylist()]
        elif fmt == "parquet":
            reader = interop.parquet.ParquetFile(path)
            if not reader.schema_arrow.equals(expected_schema, check_metadata=True):
                fail(f"exact Parquet schema/metadata mismatch: {path}")
            group_rows = [reader.metadata.row_group(index).num_rows
                          for index in range(reader.metadata.num_row_groups)]
            physical_rows = reader.read().to_pylist()
        else:
            fail(f"unsupported physical format {fmt}")
    except (OSError, interop.pa.ArrowException) as exc:
        raise QualificationError(f"cannot decode {path}: {exc}") from exc
    physical_sizes = batch_rows if fmt.startswith("ipc_") else group_rows
    if physical_sizes and sum(physical_sizes) != len(physical_rows):
        fail(f"physical boundary row counts do not reconcile with decoded rows: {path}")
    logical_rows = [physical.decode_row(row, record_type) for row in physical_rows]
    return logical_rows, {"batch_rows": batch_rows, "row_group_rows": group_rows,
                          "ordered_sha256": digest_ordered(logical_rows),
                          "file_sha256": digest_file(path), "rows": len(logical_rows)}


def expected_rows(capture: dict[str, Any], record_type: str) -> list[dict[str, Any]]:
    _, populations = PHYSICAL_TYPES[record_type]
    rows = []
    for name in populations:
        rows.extend(read_ndjson(Path(capture["capture_dir"]) / f"{name}.ndjson"))
    return rows


def write_layout_inputs(capture: dict[str, Any], directory: Path, batch_rows: int,
                        row_group_rows: int, row_order: str) -> dict[str, Any]:
    directory.mkdir(parents=True, exist_ok=False)
    reports: dict[str, Any] = {}
    for record_type, (stem, _) in PHYSICAL_TYPES.items():
        rows = expected_rows(capture, record_type)
        ordered = rows if row_order == "forward" else list(reversed(rows))
        table = physical.table_from_rows(record_type, ordered)
        schema = physical.SCHEMAS[record_type]
        if not table.schema.equals(schema, check_metadata=True):
            fail(f"C0 codec emitted wrong exact schema/metadata for {record_type}")
        reports[record_type] = {}
        for fmt in FORMATS:
            path = directory / f"{stem}.{fmt}"
            interop.write_arrow(table, path, fmt, batch_rows, row_group_rows)
            decoded, physical_report = read_physical(path, fmt, record_type)
            if decoded != ordered:
                fail(f"PyArrow {fmt} writer changed physical row sequence for {record_type}")
            if row_counter(decoded) != row_counter(rows):
                fail(f"PyArrow {fmt} writer changed row population for {record_type}")
            if fmt.startswith("ipc_"):
                if not physical_report["batch_rows"] or max(physical_report["batch_rows"]) > batch_rows:
                    fail(f"PyArrow {fmt} batch boundary exceeds requested cap for {record_type}")
            else:
                if not physical_report["row_group_rows"] or max(physical_report["row_group_rows"]) > row_group_rows:
                    fail(f"PyArrow Parquet row-group boundary exceeds requested cap for {record_type}")
                codecs = {
                    interop.parquet.ParquetFile(path).metadata.row_group(index).column(0).compression
                    for index in range(interop.parquet.ParquetFile(path).metadata.num_row_groups)
                }
                if codecs != {"UNCOMPRESSED"}:
                    fail(f"Parquet must be explicitly UNCOMPRESSED: {path}")
            reports[record_type][fmt] = physical_report
    return reports


def validate_binding(row: dict[str, Any], template: dict[str, Any], *, exclusion: bool) -> None:
    raw_event = row.get("raw_event")
    if not isinstance(raw_event, dict) or not isinstance(raw_event.get("source_fields"), dict):
        fail(f"transported row {row.get('source_event_key')} lacks raw source fields")
    raw = raw_event["source_fields"]
    bindings = template.get("event_bindings")
    if not isinstance(bindings, list):
        fail("template event_bindings must be an array")
    source_type = raw_event.get("source_event_type")
    if exclusion:
        matches = [binding for binding in bindings
                   if binding.get("source_event_type") == source_type]
    else:
        matches = [binding for binding in bindings
                   if binding.get("kind") == row.get("event_kind")
                   and binding.get("source_event_type") == source_type]
    if len(matches) != 1:
        fail(f"source event type/kind has no unique declared binding: {source_type!r}")
    binding = matches[0]
    key_field = binding.get("key_field")
    if not isinstance(key_field, str) or not isinstance(raw.get(key_field), str):
        fail(f"binding key field is absent for {source_type!r}")
    if row.get("source_event_key") != raw[key_field]:
        fail(f"transported source key differs from declared key field {key_field}")
    source_id_field = template.get("source_record_id_field")
    expected_source_id = raw.get(source_id_field) if isinstance(source_id_field, str) else None
    if raw_event.get("source_record_id") != expected_source_id:
        fail("raw_event source record id differs from the template's declared source ID field")
    if not exclusion:
        order_field = binding.get("order_field")
        ordinal = binding.get("occurrence_index")
        if not isinstance(order_field, str):
            fail(f"binding order field is absent for {source_type!r}")
        order = raw.get(order_field)
        if isinstance(order, bool) or not isinstance(order, int) or order < 0:
            fail(f"raw binding ordinal {order_field} is not a nonnegative integer")
        if row.get("source_order") != order or row.get("occurrence") != ordinal:
            fail(f"transported source ordinal/occurrence differs from binding {source_type!r}")


def reconstruct_bundle(events: list[dict[str, Any]], exclusions: list[dict[str, Any]],
                       template: dict[str, Any]) -> list[dict[str, Any]]:
    # These are two distinct physical tables. Keep the observed order within
    # each table and retain the event-table-first first-seen ordering.
    recovered: dict[tuple[str, str], dict[str, Any]] = {}
    ordered: list[dict[str, Any]] = []
    case_field = template.get("case_key_field")
    if not isinstance(case_field, str) or not case_field:
        fail("template case_key_field must be nonempty text")
    for rows, is_exclusion in ((events, False), (exclusions, True)):
        for row in rows:
            validate_binding(row, template, exclusion=is_exclusion)
            raw = row["raw_event"]["source_fields"]
            case = row.get("case_key") if not is_exclusion else raw.get(case_field)
            if case is None:
                case = raw.get(case_field)
            if not isinstance(case, str) or not case:
                fail(f"source row lacks case identity for {row.get('source_event_key')}")
            identity = (case, canonical(raw))
            previous = recovered.get(identity)
            if previous is None:
                recovered[identity] = raw
                ordered.append(raw)
            elif canonical(previous) != canonical(raw):
                fail(f"wide candidate source identity has inconsistent raw JSON: {identity[0]}")
    if not ordered:
        fail("transported Rust outputs produced no source rows")
    return ordered


def compare_capture_source(rows: list[dict[str, Any]], capture: dict[str, Any], profile: str) -> None:
    expected = capture["source"]
    if row_counter(rows) != row_counter(expected):
        fail(f"{profile} reconstructed transport source multiset differs from baseline")
    if len(rows) != len(expected):
        fail(f"{profile} reconstructed source count differs from baseline")


def bundle_bytes(rows: list[dict[str, Any]]) -> bytes:
    return (json.dumps(rows, ensure_ascii=False, sort_keys=True,
                       separators=(",", ":"), allow_nan=False) + "\n").encode("utf-8")


def validate_rust_outputs(capture: dict[str, Any], input_rows: dict[str, list[dict[str, Any]]],
                          output_dir: Path, profile: str, row_order: str,
                          *, write_bundles: bool, bundle_dir: Path | None = None) -> dict[str, Any]:
    expected_names = {
        f"{record_type}.reversed.limit-{limit}.{fmt}"
        for record_type in PHYSICAL_TYPES
        for limit in WRITER_LIMITS
        for fmt in FORMATS
    }
    missing = sorted(name for name in expected_names if not (output_dir / name).is_file())
    extra = sorted(path.name for path in output_dir.iterdir() if path.is_file() and path.name not in expected_names)
    if missing or extra:
        fail(f"{profile} Rust output set differs: missing={missing}, extra={extra}")
    file_evidence: dict[str, Any] = {}
    decoded_by_layout: dict[tuple[int, str], dict[str, list[dict[str, Any]]]] = {}
    bundle_entries: list[dict[str, Any]] = []
    for limit in WRITER_LIMITS:
        for fmt in FORMATS:
            per_type: dict[str, list[dict[str, Any]]] = {}
            for record_type in PHYSICAL_TYPES:
                name = f"{record_type}.reversed.limit-{limit}.{fmt}"
                path = output_dir / name
                rows, report = read_physical(path, fmt, record_type)
                physical_sizes = report["batch_rows"] if fmt.startswith("ipc_") else report["row_group_rows"]
                if physical_sizes and max(physical_sizes) > limit:
                    fail(f"Rust output {name} exceeds its configured writer limit {limit}")
                if not physical_sizes and rows:
                    fail(f"Rust output {name} omitted its physical boundary evidence")
                input_sequence = input_rows[record_type]
                expected_sequence = list(reversed(input_sequence))
                capture_population = expected_rows(capture, record_type)
                if len(rows) != len(capture_population):
                    fail(f"row count mismatch in Rust output {name}")
                if row_counter(rows) != row_counter(capture_population):
                    fail(f"Rust output payload multiset differs from actual capture in {name}")
                if rows != expected_sequence:
                    fail(f"ordered payload differs from exact adapter reversal in {name}")
                per_type[record_type] = rows
                file_evidence[name] = report
            decoded_by_layout[(limit, fmt)] = per_type
            source_rows = reconstruct_bundle(
                per_type["trace_event.v1"], per_type["trace_exclusion.v1"], capture["template"])
            compare_capture_source(source_rows, capture, profile)
            if write_bundles:
                if bundle_dir is None:
                    fail("bundle output directory is required when writing index sources")
                path = bundle_dir / profile / f"b{capture['batch_rows']}-g{capture['row_group_rows']}-{row_order}" / f"limit-{limit}-{fmt}.json"
                path.parent.mkdir(parents=True, exist_ok=True)
                data = bundle_bytes(source_rows)
                try:
                    with path.open("xb") as stream:
                        stream.write(data)
                except FileExistsError as exc:
                    raise QualificationError(f"refusing to overwrite bundle: {path}") from exc
                bundle_entries.append({
                    "profile": profile,
                    "batch_rows": capture["batch_rows"],
                    "row_group_rows": capture["row_group_rows"],
                    "row_order": row_order,
                    "format": fmt,
                    "writer_limit": limit,
                    "path": str(path.relative_to(bundle_dir).as_posix()),
                    "sha256": digest_bytes(data),
                    "rows": len(source_rows),
                })
    return {"files": file_evidence, "bundles": bundle_entries,
            "source_rows_per_writer": {
                f"limit-{limit}.{fmt}": len(reconstruct_bundle(
                    decoded_by_layout[(limit, fmt)]["trace_event.v1"],
                    decoded_by_layout[(limit, fmt)]["trace_exclusion.v1"],
                    capture["template"]))
                for limit in WRITER_LIMITS for fmt in FORMATS
            }}


def run_cargo_adapter(cargo: Path, rustc: Path, rustdoc: Path, target_dir: Path,
                      input_dir: Path, rust_output_dir: Path, log_path: Path) -> dict[str, Any]:
    rust_output_dir.mkdir(parents=True, exist_ok=False)
    command = [str(cargo), "test", "--locked", "--offline", "-p", "kairo-ecs-arrow-io",
               "--no-default-features", "--features", "ipc,parquet", "--test", "ingestion_physical_v1"]
    env = os.environ.copy()
    env.update({
        "RUSTC": str(rustc), "RUSTDOC": str(rustdoc), "CARGO_TARGET_DIR": str(target_dir),
        "KAIROS_C13_PHYSICAL_INDIR": str(input_dir.resolve()),
        "KAIROS_C13_PHYSICAL_OUTDIR": str(rust_output_dir.resolve()),
        "PATH": str(cargo.parent) + os.pathsep + env.get("PATH", ""),
    })
    completed = subprocess.run(command, cwd=ROOT, env=env, text=True,
                               stdout=subprocess.PIPE, stderr=subprocess.STDOUT, check=False)
    data = completed.stdout.encode("utf-8")
    log_path.parent.mkdir(parents=True, exist_ok=True)
    log_path.write_bytes(data)
    receipt = {"argv": command, "cwd": str(ROOT), "input_dir": str(input_dir.resolve()),
               "output_dir": str(rust_output_dir.resolve()), "exit_code": completed.returncode,
               "log": str(log_path.resolve()), "log_sha256": digest_bytes(data)}
    if completed.returncode != 0:
        fail(f"Rust transport adapter failed ({completed.returncode}); see {log_path}")
    return receipt


def tamper_negative(capture: dict[str, Any], valid_output: Path, negative_dir: Path,
                    report_dir: Path, profile: str) -> dict[str, Any]:
    if negative_dir.exists():
        fail(f"tamper output already exists: {negative_dir}")
    shutil.copytree(valid_output, negative_dir)
    target_name = "trace_event.v1.reversed.limit-2.ipc_file"
    target = negative_dir / target_name
    original_sha = digest_file(target)
    rows, _ = read_physical(target, "ipc_file", "trace_event.v1")
    if not rows:
        fail("tamper negative requires at least one trace event")
    corrupted = [dict(row) for row in rows]
    key = corrupted[0].get("source_event_key")
    if not isinstance(key, str) or not key:
        fail("tamper target lacks a source_event_key")
    corrupted[0]["source_event_key"] = key + ".tampered-c01"
    table = physical.table_from_rows("trace_event.v1", corrupted)
    target.unlink()
    interop.write_arrow(table, target, "ipc_file", 2, 3)
    tampered_sha = digest_file(target)
    if tampered_sha == original_sha:
        fail("schema-valid payload tamper did not change file hash")
    tampered_rows, _ = read_physical(target, "ipc_file", "trace_event.v1")
    if len(tampered_rows) != len(rows):
        fail("schema-valid payload tamper changed row count")
    no_source = negative_dir / "source-bundles"
    no_index = negative_dir / "transport-index.json"
    try:
        validate_rust_outputs(capture, {
            record_type: expected_rows(capture, record_type)
            for record_type in PHYSICAL_TYPES
        }, negative_dir, profile, "forward", write_bundles=False)
    except QualificationError as exc:
        rejection = str(exc)
    else:
        fail("negative tamper was accepted by ordered physical verification")
    if no_source.exists() or no_index.exists():
        fail("tampered negative produced a source bundle or index")
    return {"profile": profile, "target": target_name, "original_sha256": original_sha,
            "tampered_sha256": tampered_sha, "rows": len(rows),
            "schema_valid": True, "rejected": True, "rejection": rejection,
            "source_or_index_created": False}


def profile_layout_name(batch_rows: int, row_group_rows: int, row_order: str) -> str:
    return f"b{batch_rows}-g{row_group_rows}-{row_order}"


def qualify(args: argparse.Namespace) -> dict[str, Any]:
    capture_dir = Path(args.capture_dir).resolve()
    output_dir = Path(args.output_dir).resolve()
    target_dir = Path(args.target_dir).resolve()
    cargo, rustc, rustdoc = (Path(args.cargo).resolve(), Path(args.rustc).resolve(), Path(args.rustdoc).resolve())
    if not capture_dir.is_dir():
        fail(f"actual capture directory is not available: {capture_dir}")
    if output_dir.exists():
        fail(f"output directory must be new: {output_dir}")
    if target_dir == output_dir or output_dir in target_dir.parents:
        fail("Cargo target directory must remain outside retained C01 evidence")
    if not all(path.is_file() for path in (cargo, rustc, rustdoc)):
        fail("explicit Cargo, rustc, and rustdoc executable paths are required")
    output_dir.mkdir(parents=True, exist_ok=False)
    try:
        output_dir.chmod(0o700)
    except OSError:
        pass
    profile_dirs = {name: capture_dir / name for name in PROFILES}
    missing_profiles = [name for name, path in profile_dirs.items() if not path.is_dir()]
    if missing_profiles:
        fail(f"actual capture missing required profiles: {', '.join(missing_profiles)}")
    extras = sorted(path.name for path in capture_dir.iterdir()
                    if path.is_dir() and path.name not in PROFILES)
    if extras:
        fail(f"actual capture has unsupported profile directories: {', '.join(extras)}")
    profiles = {name: validate_baseline(name, path) for name, path in profile_dirs.items()}
    cargo_version = subprocess.run([str(cargo), "--version"], cwd=ROOT, text=True,
                                   stdout=subprocess.PIPE, stderr=subprocess.STDOUT, check=False)
    rustc_version = subprocess.run([str(rustc), "--version"], cwd=ROOT, text=True,
                                   stdout=subprocess.PIPE, stderr=subprocess.STDOUT, check=False)
    if cargo_version.returncode or rustc_version.returncode:
        fail("cannot read explicit Rust toolchain versions")
    toolchain = {"python": sys.version, "pyarrow": interop.pa.__version__,
                 "cargo": cargo_version.stdout.strip(), "rustc": rustc_version.stdout.strip(),
                 "rustdoc": str(rustdoc), "target_dir": str(target_dir)}
    layouts: list[dict[str, Any]] = []
    all_bundles: list[dict[str, Any]] = []
    negatives: list[dict[str, Any]] = []
    completed_invocations = 0
    try:
        for profile_name, profile in profiles.items():
            input_boundaries: dict[str, dict[str, dict[str, list[int]]]] = {
                record_type: {fmt: {} for fmt in FORMATS} for record_type in PHYSICAL_TYPES
            }
            for batch_rows, row_group_rows, row_order in LAYOUTS:
                layout = profile_layout_name(batch_rows, row_group_rows, row_order)
                profile["batch_rows"] = batch_rows
                profile["row_group_rows"] = row_group_rows
                case_dir = output_dir / "profiles" / profile_name / layout
                input_dir = case_dir / "inputs"
                rust_dir = case_dir / "rust-output"
                input_dir.parent.mkdir(parents=True, exist_ok=True)
                input_report = write_layout_inputs(profile, input_dir, batch_rows,
                                                   row_group_rows, row_order)
                for record_type in PHYSICAL_TYPES:
                    for fmt in FORMATS:
                        boundary = input_report[record_type][fmt]
                        input_boundaries[record_type][fmt][layout] = (
                            boundary["batch_rows"] if fmt.startswith("ipc_")
                            else boundary["row_group_rows"]
                        )
                input_rows = {
                    record_type: (expected_rows(profile, record_type)
                                  if row_order == "forward"
                                  else list(reversed(expected_rows(profile, record_type))))
                    for record_type in PHYSICAL_TYPES
                }
                adapter = run_cargo_adapter(
                    cargo, rustc, rustdoc, target_dir, input_dir, rust_dir,
                    output_dir / "logs" / profile_name / f"{layout}.rust.log")
                completed_invocations += 1
                bundle_dir = output_dir / "bundles"
                output_report = validate_rust_outputs(
                    profile, input_rows, rust_dir, profile_name, row_order,
                    write_bundles=True, bundle_dir=bundle_dir)
                all_bundles.extend(output_report["bundles"])
                case_report = {
                    "profile": profile_name, "batch_rows": batch_rows,
                    "row_group_rows": row_group_rows, "row_order": row_order,
                    "input_files": input_report, "adapter": adapter,
                    "rust_outputs": output_report["files"],
                    "source_bundle_hashes": [entry["sha256"] for entry in output_report["bundles"]],
                    "source_bundle_count": len(output_report["bundles"]),
                }
                report_path = case_dir / "layout-result.json"
                report_path.write_text(json.dumps(case_report, indent=2, sort_keys=True) + "\n")
                layouts.append({"profile": profile_name, "layout": layout,
                                "report": str(report_path.resolve()),
                                "report_sha256": digest_file(report_path),
                                "adapter_exit": adapter["exit_code"]})
            for fmt, left, right in (
                ("ipc_file", "b1-g1-forward", "b2-g1-forward"),
                ("ipc_stream", "b1-g1-forward", "b2-g1-forward"),
                ("parquet", "b1-g1-forward", "b1-g3-forward"),
            ):
                left_sizes = input_boundaries["trace_event.v1"][fmt][left]
                right_sizes = input_boundaries["trace_event.v1"][fmt][right]
                if left_sizes == right_sizes:
                    fail(f"{profile_name} {fmt} did not demonstrate independent boundary variation")
            # One schema-valid payload mutation per profile. The untouched first
            # forward control has already passed all 27 checks.
            control = output_dir / "profiles" / profile_name / "b1-g1-forward" / "rust-output"
            negative = output_dir / "negative" / profile_name / "tampered-rust-output"
            negatives.append(tamper_negative(
                profile,
                control,
                negative,
                output_dir / "negative" / profile_name,
                profile_name))
    except Exception as exc:
        failure = {"status": "failed", "error": str(exc),
                   "completed_adapter_invocations": completed_invocations,
                   "layout_receipts": layouts, "negative_controls": negatives}
        (output_dir / "failure.json").write_text(json.dumps(failure, indent=2, sort_keys=True) + "\n")
        raise
    required_bundles = len(PROFILES) * len(LAYOUTS) * len(WRITER_LIMITS) * len(FORMATS)
    if len(all_bundles) != required_bundles or completed_invocations != len(PROFILES) * len(LAYOUTS):
        fail(f"transport index coverage mismatch: {len(all_bundles)} bundles, {completed_invocations} adapters")
    index = {"schema_version": "c01.transport-index.v1",
             "capture_dir": str(capture_dir), "bundle_root": "bundles",
             "bundles": all_bundles}
    index_path = output_dir / "transport-index.json"
    index_path.write_text(json.dumps(index, indent=2, sort_keys=True) + "\n")
    result = {
        "status": "ready_for_review",
        "scope": "C-01 actual physical dimension evidence; not full C1.4 acceptance",
        "capture_dir": str(capture_dir), "capture_profile_receipts": {
            name: {key: value for key, value in profile.items()
                   if key not in {"template", "source", "manifest"}}
            for name, profile in profiles.items()},
        "toolchain": toolchain,
        "physical_layouts_per_profile": len(LAYOUTS),
        "actual_adapter_invocations": completed_invocations,
        "verified_rust_files": sum(
            len(json.loads(Path(item["report"]).read_text())["rust_outputs"]) for item in layouts
        ),
        "transport_index": str(index_path.resolve()),
        "transport_index_sha256": digest_file(index_path),
        "bundle_count": len(all_bundles), "negative_controls": negatives,
        "layout_receipts": layouts,
        "limitations": ["synthetic C01 profiles only", "no clinical, release, or full C1.4 claim"],
    }
    result_path = output_dir / "qualification.json"
    result_path.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n")
    result["qualification_sha256"] = digest_file(result_path)
    return result


def positive_path(raw: str) -> Path:
    path = Path(raw).expanduser()
    if not path.is_absolute():
        path = Path.cwd() / path
    return path.resolve()


def parser() -> argparse.ArgumentParser:
    cli = argparse.ArgumentParser(description=__doc__)
    commands = cli.add_subparsers(dest="command", required=True)
    run = commands.add_parser("qualify", help="qualify three actual C01 captures across physical layouts")
    run.add_argument("capture_dir", help="root with long-valid, wide-valid, and long-quarantine captures")
    run.add_argument("output_dir", help="new private evidence directory")
    run.add_argument("--cargo", required=True, help="explicit Rust 1.99 Cargo executable")
    run.add_argument("--rustc", required=True, help="matching Rust 1.99 rustc executable")
    run.add_argument("--rustdoc", required=True, help="matching Rust 1.99 rustdoc executable")
    run.add_argument("--target-dir", required=True, help="external Cargo cache, outside retained evidence")
    return cli


def main(argv: list[str] | None = None) -> int:
    cli = parser()
    args = cli.parse_args(argv)
    try:
        args.capture_dir = positive_path(args.capture_dir)
        args.output_dir = positive_path(args.output_dir)
        args.cargo, args.rustc, args.rustdoc, args.target_dir = (
            positive_path(value) for value in (args.cargo, args.rustc, args.rustdoc, args.target_dir))
        result = qualify(args)
    except (OSError, QualificationError, interop.pa.ArrowException) as exc:
        print(json.dumps({"status": "failed", "error": str(exc)}, ensure_ascii=False), file=sys.stderr)
        return 2
    print(json.dumps(result, ensure_ascii=False, sort_keys=True, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
