"""Frozen-schema transport fixtures from independently validated Rust mapper results.

Each input request is its own dataset. The files are a fixture collection,
not a merged ingestion dataset; row_contexts preserves request membership.
"""
import argparse
import hashlib
import json
from pathlib import Path
import sys
import shutil

import pyarrow as pa
import pyarrow.ipc as ipc
import pyarrow.parquet as pq

ROOT = Path(__file__).resolve().parents[5]
sys.path.insert(0, str(ROOT / "conformance/c11"))
from physical_schema import SCHEMAS, decode_row, table_from_rows

NAMES = {"trace_event.v1": "trace", "trace_exclusion.v1": "exclusion",
         "outcome_observation.v1": "outcome"}
HERE = Path(__file__).resolve().parent


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def read_table(path, kind):
    if kind == "parquet":
        return pq.read_table(path)
    with pa.memory_map(str(path), "r") as source:
        reader = ipc.open_file(source) if kind == "ipc_file" else ipc.open_stream(source)
        return reader.read_all()


def check_table(table, record_type, expected):
    if not table.schema.equals(SCHEMAS[record_type], check_metadata=True):
        raise ValueError("physical schema/metadata drift: " + record_type)
    actual = [decode_row(row, record_type) for row in table.to_pylist()]
    if actual != expected:
        raise ValueError("logical record payload drift: " + record_type)


def generate(snapshot, validation_receipt):
    if validation_receipt is None:
        raise ValueError("independent C0 validation receipt is required before any writes")
    proof = json.loads(validation_receipt.read_text())
    if proof.get("exit_code") != 0 or proof.get("snapshot_sha256") != digest(snapshot) or proof.get("logical_schema_sha256") != SCHEMAS["trace_event.v1"].metadata[b"logical_schema_sha256"].decode():
        raise ValueError("C0 validation receipt does not qualify this actual capture")
    captures = json.loads(snapshot.read_text())
    if not isinstance(captures, list) or not captures:
        raise ValueError("actual mapper capture must be a nonempty request/result list")
    rows = {record_type: [] for record_type in SCHEMAS}
    contexts = {record_type: [] for record_type in SCHEMAS}
    for index, capture in enumerate(captures):
        if not isinstance(capture, dict) or "request" not in capture or "result" not in capture:
            raise ValueError("capture must contain raw request and actual result, never goldens")
        result = capture["result"]
        a = result["accounting"]
        if a["candidate_units"] != sum(a[k] for k in (
                "accepted_units", "excluded_units", "failed_units", "unresolved_units")):
            raise ValueError("candidate count conservation failed")
        events = result.get("records", [])
        outcomes = result.get("outcomes", [])
        if sum(r["record_type"] == "trace_event.v1" for r in events) != a["accepted_units"]:
            raise ValueError("accepted event count does not match actual rows")
        if sum(r["record_type"] == "trace_exclusion.v1" for r in events) != a["excluded_units"]:
            raise ValueError("excluded event count does not match actual rows")
        for record in events + outcomes:
            record_type = record["record_type"]
            rows[record_type].append(record)
            contexts[record_type].append(index)
    # Logical validation is a separate mandatory command in the qualification
    # packet. It uses the accepted C0 schema, not this transport projection.
    manifest = {"fixture_collection_not_dataset": True, "synthetic_only": True,
                "pyarrow": pa.__version__, "actual_snapshot_sha256": digest(snapshot),
                "request_count": len(captures), "requests": [c["request"] for c in captures],
                "consumer_commit": "658a7a845f117405a6c728c34279d8e1814ade82", "physical_version": "2", "row_contexts": contexts,
                "records": rows, "files": {}}
    if proof.get("request_count") != len(captures) or any(proof.get(key) != len(captures) for key in (
            "source_row_count_checks", "candidate_expansion_checks", "candidate_partition_checks", "outcome_population_checks")):
        raise ValueError("C0/source/outcome proof coverage is incomplete")
    if proof.get("validator_sha256") != digest(HERE / "validate_actual.py"):
        raise ValueError("independent validator source no longer matches proof")
    if proof.get("record_counts") != {record_type: len(records) for record_type, records in rows.items()}:
        raise ValueError("C0 proof record counts do not match actual capture")
    proof_target = HERE / "c0-validation.json"
    if validation_receipt.resolve() != proof_target.resolve():
        shutil.copyfile(validation_receipt, proof_target)
    manifest["c0_validation"] = {"file": proof_target.name, "sha256": digest(proof_target)}
    for record_type, records in rows.items():
        table = table_from_rows(record_type, records)
        for kind in ("ipc_file", "ipc_stream", "parquet"):
            path = HERE / (NAMES[record_type] + "." + kind)
            if kind == "parquet":
                pq.write_table(table, path, compression="NONE", row_group_size=2,
                               use_compliant_nested_type=True)
            else:
                with pa.OSFile(str(path), "wb") as sink:
                    factory = ipc.new_file if kind == "ipc_file" else ipc.new_stream
                    with factory(sink, table.schema) as writer:
                        for batch in table.to_batches(max_chunksize=2):
                            writer.write_batch(batch)
            check_table(read_table(path, kind), record_type, records)
            manifest["files"][path.name] = digest(path)
    (HERE / "manifest.json").write_text(json.dumps(manifest, ensure_ascii=False,
                                                   sort_keys=True, indent=2) + "\n")


def check(rust_out):
    manifest = json.loads((HERE / "manifest.json").read_text())
    proof_ref = manifest["c0_validation"]
    proof_path = HERE / proof_ref["file"]
    if digest(proof_path) != proof_ref["sha256"]:
        raise ValueError("independent C0 validation evidence changed")
    proof = json.loads(proof_path.read_text())
    if proof.get("exit_code") != 0 or proof.get("snapshot_sha256") != manifest["actual_snapshot_sha256"] or proof.get("request_count") != manifest["request_count"]:
        raise ValueError("C0 validation evidence does not bind the actual fixture capture")
    if any(proof.get(key) != manifest["request_count"] for key in (
            "source_row_count_checks", "candidate_expansion_checks", "candidate_partition_checks", "outcome_population_checks")):
        raise ValueError("independent C0/candidate/outcome coverage incomplete")
    if digest(HERE / "validate_actual.py") != proof["validator_sha256"]:
        raise ValueError("independent validator source changed")
    if manifest["pyarrow"] != "25.0.1":
        raise ValueError("fixture oracle version drift")
    for name, expected_hash in manifest["files"].items():
        if digest(HERE / name) != expected_hash:
            raise ValueError("frozen fixture bytes changed: " + name)
    for record_type, records in manifest["records"].items():
        for kind in ("ipc_file", "ipc_stream", "parquet"):
            name = NAMES[record_type] + "." + kind
            check_table(read_table(HERE / name, kind), record_type, records)
            if rust_out is not None:
                check_table(read_table(rust_out / name, kind), record_type, records)
    print("PASS: 3 physical tables, 9 files, exact schema/metadata and logical payloads")


if __name__ == "__main__":
    if pa.__version__ != "25.0.1":
        raise SystemExit("requires pinned PyArrow 25.0.1 before any writes")
    parser = argparse.ArgumentParser()
    parser.add_argument("--generate", type=Path)
    parser.add_argument("--check-rust", type=Path)
    parser.add_argument("--validation-receipt", type=Path)
    args = parser.parse_args()
    if args.generate is not None:
        generate(args.generate, args.validation_receipt)
    check(args.check_rust)
