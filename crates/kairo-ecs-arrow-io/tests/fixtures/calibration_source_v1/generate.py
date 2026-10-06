#!/usr/bin/env python3
"""Create/check synthetic raw-source transport fixtures with pinned PyArrow."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

import pyarrow as pa
import pyarrow.ipc as ipc
import pyarrow.parquet as pq


HERE = Path(__file__).resolve().parent
REQUIRED_PYARROW_VERSION = "25.0.1"
META = {
    "kairos.fixture.contract": "recordbatch-v1",
    "kairos.fixture.name": "calibration-raw-source.v1",
    "kairos.fixture.precision-scope": "reported-source-precision-only; not inferred from clock text",
    "kairos.fixture.schema-scope": "raw-source-input-only; not normalized calibration schema",
    "kairos.fixture.synthetic": "true",
}
WIDE_FIELDS = [
    ("source_row_id", "string", False),
    ("encounter_id", "string", False),
    ("event_occurred_at", "string", True),
    ("event_recorded_at", "string", True),
    ("triage_at", "string", True),
    ("departure_admin_at", "string", True),
    ("departure_physical_at", "string", True),
    ("timezone_name", "string", True),
    ("naive_local_at", "string", True),
    ("occurrence_precision", "string", True),
    ("triage_precision", "string", True),
    ("canonical_rank", "string", False),
    ("relative_elapsed_ns", "string", True),
    ("cohort_denominator", "string", True),
    ("triage_denominator", "string", True),
    ("future_outcome", "string", True),
    ("duplicate_external_id", "string", True),
]
LONG_FIELDS = [
    ("source_row_id", "string", False),
    ("source_entity", "string", False),
    ("field_name", "string", False),
    ("field_value", "string", True),
]

# Literals model two fictional arrival rows as supplied by a source extract. Each
# long-form tuple is a physical row in a separate fictional key/value extract.
WIDE_ROWS = [
    [
        "arrival-001", "enc-001", "2024-11-03T01:30:00.123456789123-04:00",
        "2024-11-03T01:30:05.000000000001-04:00", "2024-11-03T01:42:00-04:00",
        "2024-11-03T03:10:00-05:00", "2024-11-03T03:17:00-05:00", "America/New_York",
        "2024-11-03T01:30:00.123456789123", "sub-nanosecond", "second", "18446744073709551616",
        "18446744073709551616", None, "", "positive synthetic", "dup-7",
    ],
    [
        "arrival-002", "enc-002", "1900-01-01T00:00:00.000000001Z", None,
        "2024-03-10T02:30:00", "2024-03-10T02:31:00", None, "America/New_York",
        "2024-03-10T02:30:00", "nanosecond", "minute", "7", None, "0", None, None, "dup-7",
    ],
]
LONG_ROWS = [
    ("arrival-001", "arrival", "source_row_id", "arrival-001"),
    ("arrival-001", "arrival", "encounter_id", "enc-001"),
    ("arrival-001", "arrival", "event_occurred_at", "2024-11-03T01:30:00.123456789123-04:00"),
    ("arrival-001", "arrival", "event_recorded_at", "2024-11-03T01:30:05.000000000001-04:00"),
    ("arrival-001", "arrival", "triage_at", "2024-11-03T01:42:00-04:00"),
    ("arrival-001", "arrival", "departure_admin_at", "2024-11-03T03:10:00-05:00"),
    ("arrival-001", "arrival", "departure_physical_at", "2024-11-03T03:17:00-05:00"),
    ("arrival-001", "arrival", "timezone_name", "America/New_York"),
    ("arrival-001", "arrival", "naive_local_at", "2024-11-03T01:30:00.123456789123"),
    ("arrival-001", "arrival", "occurrence_precision", "sub-nanosecond"),
    ("arrival-001", "arrival", "triage_precision", "second"),
    ("arrival-001", "arrival", "canonical_rank", "18446744073709551616"),
    ("arrival-001", "arrival", "relative_elapsed_ns", "18446744073709551616"),
    ("arrival-001", "arrival", "cohort_denominator", None),
    ("arrival-001", "arrival", "triage_denominator", ""),
    ("arrival-001", "arrival", "future_outcome", "positive synthetic"),
    ("arrival-001", "arrival", "duplicate_external_id", "dup-7"),
    ("arrival-002", "arrival", "source_row_id", "arrival-002"),
    ("arrival-002", "arrival", "encounter_id", "enc-002"),
    ("arrival-002", "arrival", "event_occurred_at", "1900-01-01T00:00:00.000000001Z"),
    ("arrival-002", "arrival", "event_recorded_at", None),
    ("arrival-002", "arrival", "triage_at", "2024-03-10T02:30:00"),
    ("arrival-002", "arrival", "departure_admin_at", "2024-03-10T02:31:00"),
    ("arrival-002", "arrival", "departure_physical_at", None),
    ("arrival-002", "arrival", "timezone_name", "America/New_York"),
    ("arrival-002", "arrival", "naive_local_at", "2024-03-10T02:30:00"),
    ("arrival-002", "arrival", "occurrence_precision", "nanosecond"),
    ("arrival-002", "arrival", "triage_precision", "minute"),
    ("arrival-002", "arrival", "canonical_rank", "7"),
    ("arrival-002", "arrival", "relative_elapsed_ns", None),
    ("arrival-002", "arrival", "cohort_denominator", "0"),
    ("arrival-002", "arrival", "triage_denominator", None),
    ("arrival-002", "arrival", "future_outcome", None),
    ("arrival-002", "arrival", "duplicate_external_id", "dup-7"),
]


def arrow_type(name: str) -> pa.DataType:
    return {"string": pa.string()}[name]


def schema(fields: list[tuple[str, str, bool]]) -> pa.Schema:
    return pa.schema(
        [pa.field(name, arrow_type(kind), nullable=nullable) for name, kind, nullable in fields],
        metadata={k.encode(): v.encode() for k, v in META.items()},
    )


def table(fields: list[tuple[str, str, bool]], rows: list[list[str | None]] | list[tuple[str, ...]]) -> pa.Table:
    return pa.Table.from_pylist(
        [dict(zip((field[0] for field in fields), row, strict=True)) for row in rows],
        schema=schema(fields),
    )


def artifacts() -> dict[str, pa.Table]:
    return {"wide": table(WIDE_FIELDS, WIDE_ROWS), "long": table(LONG_FIELDS, LONG_ROWS)}


def require_pinned_pyarrow() -> None:
    if pa.__version__ != REQUIRED_PYARROW_VERSION:
        raise RuntimeError(
            f"PyArrow {REQUIRED_PYARROW_VERSION} is required; found {pa.__version__}"
        )


def write_formats() -> None:
    for profile, data in artifacts().items():
        with (HERE / f"{profile}.ipc_file").open("wb") as stream:
            with ipc.new_file(stream, data.schema) as writer:
                writer.write_table(data, max_chunksize=2)
        with (HERE / f"{profile}.ipc_stream").open("wb") as stream:
            with ipc.new_stream(stream, data.schema) as writer:
                writer.write_table(data, max_chunksize=2)
        pq.write_table(data, HERE / f"{profile}.parquet", row_group_size=2, compression="NONE")
    binary_sha256 = {
        path.name: hashlib.sha256(path.read_bytes()).hexdigest()
        for profile in ("wide", "long")
        for path in (HERE / f"{profile}.{fmt}" for fmt in ("ipc_file", "ipc_stream", "parquet"))
    }
    manifest = {
        "scope": "test-local raw source fixtures; not the normalized calibration schema",
        "pyarrow_version": pa.__version__,
        "schema_metadata": META,
        "binary_sha256": binary_sha256,
        "precision_scope": "occurrence_precision and triage_precision are independent source annotations; values describe reported source precision only and are not inferred from the raw clock text",
        "raw_clock_cases": {
            "ambiguous_fold_literal": {"source_row_id": "arrival-001", "timezone_name": "America/New_York", "naive_local_at": "2024-11-03T01:30:00.123456789123", "offset_clock_literal": "2024-11-03T01:30:00.123456789123-04:00"},
            "nonexistent_gap_literal": {"source_row_id": "arrival-002", "timezone_name": "America/New_York", "naive_local_at": "2024-03-10T02:30:00"},
            "raw_units_are_not_inferred": True,
        },
        "profiles": {
            "wide": {"description": "one physical row per arrival, including raw arrival and triage fields", "fields": [
                {"name": n, "type": t, "nullable": z} for n, t, z in WIDE_FIELDS
            ], "rows": WIDE_ROWS},
            "long": {"description": "one physical row per source entity and raw field/value pair", "fields": [
                {"name": n, "type": t, "nullable": z} for n, t, z in LONG_FIELDS
            ], "rows": LONG_ROWS},
        },
    }
    (HERE / "manifest.json").write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n")


def read_python(path: Path, fmt: str) -> pa.Table:
    if fmt == "ipc_file":
        return ipc.open_file(path).read_all()
    if fmt == "ipc_stream":
        return ipc.open_stream(path).read_all()
    return pq.read_table(path)


def validate_inputs() -> dict[str, str]:
    hashes: dict[str, str] = {}
    for profile, expected in artifacts().items():
        for fmt in ("ipc_file", "ipc_stream", "parquet"):
            path = HERE / f"{profile}.{fmt}"
            actual = read_python(path, fmt)
            if not actual.schema.equals(expected.schema, check_metadata=True):
                raise AssertionError(f"{path.name}: schema differs from literal manifest")
            if actual.to_pylist() != expected.to_pylist():
                raise AssertionError(f"{path.name}: rows differ from literal source rows")
            hashes[path.name] = hashlib.sha256(path.read_bytes()).hexdigest()
    tables = {profile: read_python(HERE / f"{profile}.parquet", "parquet") for profile in ("wide", "long")}
    validate_wide_long_parity(tables["wide"], tables["long"])
    manifest = json.loads((HERE / "manifest.json").read_text())
    if manifest.get("pyarrow_version") != REQUIRED_PYARROW_VERSION:
        raise AssertionError("manifest PyArrow version does not match the required pinned version")
    if manifest.get("schema_metadata") != META:
        raise AssertionError("manifest schema metadata drift")
    if manifest.get("scope") != "test-local raw source fixtures; not the normalized calibration schema":
        raise AssertionError("manifest scope drift")
    if manifest.get("precision_scope") != "occurrence_precision and triage_precision are independent source annotations; values describe reported source precision only and are not inferred from the raw clock text":
        raise AssertionError("manifest precision scope drift")
    for name, fields in (("wide", WIDE_FIELDS), ("long", LONG_FIELDS)):
        got = [(f["name"], f["type"], f["nullable"]) for f in manifest["profiles"][name]["fields"]]
        expected_rows = WIDE_ROWS if name == "wide" else [list(row) for row in LONG_ROWS]
        if got != fields or manifest["profiles"][name].get("rows") != expected_rows:
            raise AssertionError(f"manifest schema drift for {name}")
    if manifest.get("binary_sha256") != {name: digest for name, digest in hashes.items() if name != "manifest.json"}:
        raise AssertionError("manifest source binary SHA-256 drift")
    hashes["manifest.json"] = hashlib.sha256((HERE / "manifest.json").read_bytes()).hexdigest()
    return hashes


def validate_wide_long_parity(wide: pa.Table, long: pa.Table) -> None:
    wide_rows = wide.to_pylist()
    long_rows = long.to_pylist()
    for wide_row in wide_rows:
        source_id = wide_row["source_row_id"]
        long_payload: dict[str, str | None] = {}
        for long_row in long_rows:
            if long_row["source_row_id"] == source_id:
                if long_row["source_entity"] != "arrival":
                    raise AssertionError(f"unexpected source entity for {source_id}")
                field_name = long_row["field_name"]
                if field_name in long_payload:
                    raise AssertionError(f"duplicate long field {source_id}.{field_name}")
                long_payload[field_name] = long_row["field_value"]
        if long_payload != wide_row:
            raise AssertionError(f"wide/long raw payload mismatch for {source_id}")


def check_rust(output_dir: Path) -> dict[str, str]:
    validate_inputs()
    hashes: dict[str, str] = {}
    for profile, expected in artifacts().items():
        for fmt in ("ipc_file", "ipc_stream", "parquet"):
            path = output_dir / f"{profile}.{fmt}"
            actual = read_python(path, fmt)
            if not actual.schema.equals(expected.schema, check_metadata=True):
                raise AssertionError(f"Rust {path.name}: schema differs from literal manifest")
            if actual.to_pylist() != expected.to_pylist():
                raise AssertionError(f"Rust {path.name}: rows differ from literal source rows")
            hashes[path.name] = hashlib.sha256(path.read_bytes()).hexdigest()
    return hashes


def main() -> None:
    parser = argparse.ArgumentParser()
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--generate-and-check", action="store_true")
    group.add_argument("--check-rust", type=Path)
    args = parser.parse_args()
    require_pinned_pyarrow()
    if args.generate_and_check:
        write_formats()
        print(json.dumps({"pyarrow_version": pa.__version__, "input_hashes": validate_inputs()}, indent=2))
    else:
        print(json.dumps({"pyarrow_version": pa.__version__, "rust_output_hashes": check_rust(args.check_rust)}, indent=2))


if __name__ == "__main__":
    main()
