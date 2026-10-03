#!/usr/bin/env python3
"""Generate and independently re-read synthetic C1.1 Arrow fixtures."""
from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path
from typing import Iterable

import pyarrow as pa
import pyarrow.ipc as ipc
import pyarrow.parquet as pq

FIXTURE_DIR = Path(__file__).resolve().parent
PYARROW_VERSION = "25.0.1"
SCHEMA_METADATA = {
    b"kairos.fixture.name": b"interop-probe.v1",
    b"kairos.fixture.synthetic": b"true",
    b"kairos.fixture.contract": b"recordbatch-v1",
}
FIELD_METADATA = {
    "nullable_fixed16": {b"role": b"opaque-synthetic-bytes"},
    "ticks_u128_le": {b"unit": b"ns", b"encoding": b"little-endian-u128"},
}
SCHEMA = pa.schema(
    [
        pa.field("signed_i32", pa.int32(), nullable=False),
        pa.field("nullable_text", pa.string(), nullable=True),
        pa.field(
            "nullable_fixed16",
            pa.binary(16),
            nullable=True,
            metadata=FIELD_METADATA["nullable_fixed16"],
        ),
        pa.field(
            "ticks_u128_le",
            pa.binary(16),
            nullable=False,
            metadata=FIELD_METADATA["ticks_u128_le"],
        ),
        pa.field("synthetic_label", pa.string(), nullable=False),
    ],
    metadata=SCHEMA_METADATA,
)
EMPTY_SCHEMA = pa.schema([], metadata=SCHEMA_METADATA)
U128_MAX = (1 << 128) - 1
ROWS = [
    (-7, None, None, 0, "synthetic-a"),
    (0, "", bytes(16), 1, "é-二"),
    (19, "München/東京", bytes(range(16)), U128_MAX, "synthetic-c"),
]
BINARY_NAMES = (
    "mixed.ipc_file",
    "mixed.ipc_stream",
    "mixed.parquet",
    "typed_empty.ipc_file",
    "typed_empty.ipc_stream",
    "typed_empty.parquet",
    "typed_zero_row_batch.ipc_file",
    "typed_zero_row_batch.ipc_stream",
    "empty_schema.ipc_file",
    "empty_schema.ipc_stream",
)


def _batch(rows: list[tuple], schema: pa.Schema = SCHEMA) -> pa.RecordBatch:
    if len(schema) == 0:
        return pa.RecordBatch.from_arrays([], schema=schema)
    return pa.RecordBatch.from_arrays(
        [
            pa.array([row[0] for row in rows], type=pa.int32()),
            pa.array([row[1] for row in rows], type=pa.string()),
            pa.array([row[2] for row in rows], type=pa.binary(16)),
            pa.array([row[3].to_bytes(16, "little") for row in rows], type=pa.binary(16)),
            pa.array([row[4] for row in rows], type=pa.string()),
        ],
        schema=schema,
    )


def _ipc_file(schema: pa.Schema, batches: Iterable[pa.RecordBatch]) -> bytes:
    sink = pa.BufferOutputStream()
    with ipc.new_file(sink, schema) as writer:
        for batch in batches:
            writer.write_batch(batch)
    return sink.getvalue().to_pybytes()


def _ipc_stream(schema: pa.Schema, batches: Iterable[pa.RecordBatch]) -> bytes:
    sink = pa.BufferOutputStream()
    with ipc.new_stream(sink, schema) as writer:
        for batch in batches:
            writer.write_batch(batch)
    return sink.getvalue().to_pybytes()


def _parquet(schema: pa.Schema, batches: Iterable[pa.RecordBatch]) -> bytes:
    table = pa.Table.from_batches(list(batches), schema=schema)
    sink = pa.BufferOutputStream()
    pq.write_table(
        table,
        sink,
        row_group_size=2,
        compression="NONE",
        use_dictionary=False,
        write_statistics=False,
        store_schema=True,
        version="2.6",
    )
    return sink.getvalue().to_pybytes()


def build_artifacts() -> dict[str, bytes]:
    mixed_batches = [_batch(ROWS[:2]), _batch(ROWS[2:])]
    zero_row_batch = _batch([])
    return {
        "mixed.ipc_file": _ipc_file(SCHEMA, mixed_batches),
        "mixed.ipc_stream": _ipc_stream(SCHEMA, mixed_batches),
        "mixed.parquet": _parquet(SCHEMA, mixed_batches),
        "typed_empty.ipc_file": _ipc_file(SCHEMA, []),
        "typed_empty.ipc_stream": _ipc_stream(SCHEMA, []),
        "typed_empty.parquet": _parquet(SCHEMA, []),
        "typed_zero_row_batch.ipc_file": _ipc_file(SCHEMA, [zero_row_batch]),
        "typed_zero_row_batch.ipc_stream": _ipc_stream(SCHEMA, [zero_row_batch]),
        "empty_schema.ipc_file": _ipc_file(EMPTY_SCHEMA, []),
        "empty_schema.ipc_stream": _ipc_stream(EMPTY_SCHEMA, []),
    }


def _metadata_dict(metadata: dict[bytes, bytes] | None) -> dict[str, str]:
    if metadata is None:
        return {}
    return {
        key.decode("utf-8"): value.decode("utf-8")
        for key, value in sorted(metadata.items())
    }


def _schema_document(schema: pa.Schema) -> dict:
    return {
        "metadata": _metadata_dict(schema.metadata),
        "fields": [
            {
                "name": field.name,
                "type": str(field.type),
                "nullable": field.nullable,
                "metadata": _metadata_dict(field.metadata),
            }
            for field in schema
        ],
    }


def _expected_rows() -> list[dict]:
    return [
        {
            "signed_i32": signed,
            "nullable_text": text,
            "nullable_fixed16_hex": None if fixed is None else fixed.hex(),
            "ticks_u128_decimal": str(ticks),
            "ticks_u128_le_hex": ticks.to_bytes(16, "little").hex(),
            "synthetic_label": label,
        }
        for signed, text, fixed, ticks, label in ROWS
    ]


def _logical_rows(batches: Iterable[pa.RecordBatch]) -> list[dict]:
    rows: list[dict] = []
    for batch in batches:
        if not batch.schema.equals(SCHEMA, check_metadata=True):
            raise AssertionError("record batch schema or metadata mismatch")
        columns = [batch.column(i).to_pylist() for i in range(batch.num_columns)]
        for values in zip(*columns):
            signed, text, fixed, ticks_le, label = values
            rows.append(
                {
                    "signed_i32": signed,
                    "nullable_text": text,
                    "nullable_fixed16_hex": None if fixed is None else fixed.hex(),
                    "ticks_u128_decimal": str(int.from_bytes(ticks_le, "little")),
                    "ticks_u128_le_hex": ticks_le.hex(),
                    "synthetic_label": label,
                }
            )
    return rows


def _golden(artifacts: dict[str, bytes]) -> dict:
    return {
        "fixture_version": 1,
        "synthetic_only": True,
        "generator": {"pyarrow": PYARROW_VERSION},
        "schema": _schema_document(SCHEMA),
        "empty_schema": _schema_document(EMPTY_SCHEMA),
        "logical_rows": _expected_rows(),
        "ipc_batch_counts": {
            "mixed": 2,
            "typed_empty": 0,
            "typed_zero_row_batch": 1,
            "empty_schema": 0,
        },
        "parquet_row_groups": {"mixed": 2, "typed_empty": 1},
        "sha256": {
            name: hashlib.sha256(artifacts[name]).hexdigest()
            for name in sorted(artifacts)
        },
    }


def _check_ipc_file(data: bytes, schema: pa.Schema, expected_batches: int,
                    expected_rows: list[dict]) -> None:
    reader = ipc.open_file(pa.BufferReader(data))
    if not reader.schema.equals(schema, check_metadata=True):
        raise AssertionError("IPC file schema/metadata mismatch")
    batches = [reader.get_batch(i) for i in range(reader.num_record_batches)]
    if len(batches) != expected_batches:
        raise AssertionError(f"IPC file batch count {len(batches)} != {expected_batches}")
    if schema.equals(SCHEMA, check_metadata=True) and _logical_rows(batches) != expected_rows:
        raise AssertionError("IPC file logical rows mismatch")


def _check_ipc_stream(data: bytes, schema: pa.Schema, expected_batches: int,
                      expected_rows: list[dict]) -> None:
    reader = ipc.open_stream(pa.BufferReader(data))
    if not reader.schema.equals(schema, check_metadata=True):
        raise AssertionError("IPC stream schema/metadata mismatch")
    batches = list(reader)
    if len(batches) != expected_batches:
        raise AssertionError(f"IPC stream batch count {len(batches)} != {expected_batches}")
    if schema.equals(SCHEMA, check_metadata=True) and _logical_rows(batches) != expected_rows:
        raise AssertionError("IPC stream logical rows mismatch")


def _check_parquet(data: bytes, schema: pa.Schema, expected_rows: list[dict],
                   expected_row_groups: int) -> None:
    reader = pq.ParquetFile(pa.BufferReader(data))
    if not reader.schema_arrow.equals(schema, check_metadata=True):
        raise AssertionError("Parquet schema/metadata mismatch")
    if reader.metadata.num_row_groups != expected_row_groups:
        raise AssertionError(
            f"Parquet row groups {reader.metadata.num_row_groups} != {expected_row_groups}"
        )
    batches = reader.iter_batches(batch_size=2)
    if schema.equals(SCHEMA, check_metadata=True) and _logical_rows(batches) != expected_rows:
        raise AssertionError("Parquet logical rows mismatch")


def verify_artifacts(artifacts: dict[str, bytes]) -> None:
    rows = _expected_rows()
    _check_ipc_file(artifacts["mixed.ipc_file"], SCHEMA, 2, rows)
    _check_ipc_stream(artifacts["mixed.ipc_stream"], SCHEMA, 2, rows)
    _check_parquet(artifacts["mixed.parquet"], SCHEMA, rows, 2)
    _check_ipc_file(artifacts["typed_empty.ipc_file"], SCHEMA, 0, [])
    _check_ipc_stream(artifacts["typed_empty.ipc_stream"], SCHEMA, 0, [])
    _check_parquet(artifacts["typed_empty.parquet"], SCHEMA, [], 1)
    _check_ipc_file(artifacts["typed_zero_row_batch.ipc_file"], SCHEMA, 1, [])
    _check_ipc_stream(artifacts["typed_zero_row_batch.ipc_stream"], SCHEMA, 1, [])
    _check_ipc_file(artifacts["empty_schema.ipc_file"], EMPTY_SCHEMA, 0, [])
    _check_ipc_stream(artifacts["empty_schema.ipc_stream"], EMPTY_SCHEMA, 0, [])


def _golden_bytes(golden: dict) -> bytes:
    return (json.dumps(golden, ensure_ascii=False, indent=2, sort_keys=True) + "\n").encode("utf-8")


def generate_and_check() -> None:
    if pa.__version__ != PYARROW_VERSION:
        raise RuntimeError(f"expected PyArrow {PYARROW_VERSION}, found {pa.__version__}")
    first = build_artifacts()
    second = build_artifacts()
    if first != second:
        raise AssertionError("two in-memory fixture generations differ")
    verify_artifacts(first)
    for name, data in first.items():
        (FIXTURE_DIR / name).write_bytes(data)
    expected_golden = _golden(first)
    (FIXTURE_DIR / "golden.json").write_bytes(_golden_bytes(expected_golden))

    checked = {name: (FIXTURE_DIR / name).read_bytes() for name in BINARY_NAMES}
    if checked != first:
        raise AssertionError("written fixture bytes differ from deterministic generation")
    verify_artifacts(checked)
    if (FIXTURE_DIR / "golden.json").read_bytes() != _golden_bytes(_golden(checked)):
        raise AssertionError("golden JSON differs from independently decoded artifacts")


def check_rust_outputs(directory: Path) -> None:
    if pa.__version__ != PYARROW_VERSION:
        raise RuntimeError(f"expected PyArrow {PYARROW_VERSION}, found {pa.__version__}")
    data = {
        name: (directory / name).read_bytes()
        for name in ("rust.ipc_file", "rust.ipc_stream", "rust.parquet")
    }
    rows = _expected_rows()
    _check_ipc_file(data["rust.ipc_file"], SCHEMA, 2, rows)
    _check_ipc_stream(data["rust.ipc_stream"], SCHEMA, 2, rows)
    _check_parquet(data["rust.parquet"], SCHEMA, rows, 2)
    print("PASS: PyArrow read Rust IPC file/stream and Parquet outputs exactly")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--generate", action="store_true")
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--check-rust", type=Path)
    args = parser.parse_args()
    try:
        if args.check_rust is not None:
            check_rust_outputs(args.check_rust)
            return 0
        if not args.generate and not args.check:
            parser.error("select --generate and/or --check, or --check-rust DIR")
        if args.generate:
            generate_and_check()
        elif args.check:
            if pa.__version__ != PYARROW_VERSION:
                raise RuntimeError(f"expected PyArrow {PYARROW_VERSION}, found {pa.__version__}")
            first = build_artifacts()
            second = build_artifacts()
            if first != second:
                raise AssertionError("two in-memory fixture generations differ")
            verify_artifacts(first)
            checked = {name: (FIXTURE_DIR / name).read_bytes() for name in BINARY_NAMES}
            if checked != first:
                raise AssertionError("checked-in fixture bytes differ from generation")
            if (FIXTURE_DIR / "golden.json").read_bytes() != _golden_bytes(_golden(checked)):
                raise AssertionError("golden JSON/hash mismatch")
        print(f"PASS: deterministic PyArrow {PYARROW_VERSION} fixture schema, values, metadata and hashes")
        return 0
    except (AssertionError, OSError, RuntimeError, ValueError) as exc:
        print(f"FAIL: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
