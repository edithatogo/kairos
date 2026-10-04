"""Independent C0 and source/candidate/outcome population validation.

Consumes actual Rust request/result captures, never hand-authored goldens.
Checks remain enabled under Python optimization.
"""
import hashlib
from importlib.metadata import version
import json
from pathlib import Path
import sys

from jsonschema import Draft202012Validator, FormatChecker

C0_SHA256 = "8c46db62f691f243385a4ebdf8a7a3d3670e2655dd0c3f82cba4335ced2a3842"
NAMES = {"trace_event.v1": "trace_event", "trace_exclusion.v1": "trace_exclusion",
         "outcome_observation.v1": "outcome_observation"}


def require(condition, detail):
    if not condition:
        raise ValueError(detail)


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, "duplicate JSON object key: " + key)
        result[key] = value
    return result


def load(path):
    def nonfinite(value):
        raise ValueError("nonfinite JSON value: " + value)
    return json.loads(path.read_text(), object_pairs_hook=unique_object,
                      parse_constant=nonfinite)


def validate(schema_path, snapshot_path):
    require(hashlib.sha256(schema_path.read_bytes()).hexdigest() == C0_SHA256,
            "accepted C0 logical schema hash changed")
    schema = load(schema_path)
    captures = load(snapshot_path)
    require(isinstance(captures, list) and captures, "nonempty actual capture required")
    validators = {record_type: Draft202012Validator(
        {"$ref": "#/$defs/" + name, "$defs": schema["$defs"]},
        format_checker=FormatChecker()) for record_type, name in NAMES.items()}
    counts = dict.fromkeys(NAMES, 0)
    per_request = []
    for index, capture in enumerate(captures):
        request, result = capture["request"], capture["result"]
        accounting = result["accounting"]
        rows, bindings = request["rows"], request["event_bindings"]
        require(accounting["source_rows"] == len(rows), (index, "source-row count"))
        if request["shape"] == "wide":
            expected_candidates = len(rows) * len(bindings)
        else:
            kind_field = request.get("event_kind_field")
            expected_candidates = sum(
                binding.get("kind") == row.get(kind_field)
                for row in rows if isinstance(row, dict)
                for binding in bindings if isinstance(binding, dict))
        require(accounting["candidate_units"] == expected_candidates,
                (index, "declared expansion count", accounting))
        partition = sum(accounting[key] for key in (
            "accepted_units", "excluded_units", "failed_units", "unresolved_units"))
        require(accounting["candidate_units"] == partition,
                (index, "candidate conservation", accounting))
        expected_outcomes = 0 if result["classification"] == "failed" else sum(
            isinstance(row.get("outcome"), dict) for row in rows)
        require(len(result["outcomes"]) == expected_outcomes,
                (index, "distinct source-row outcome population"))
        for record in result["records"] + result["outcomes"]:
            record_type = record["record_type"]
            errors = list(validators[record_type].iter_errors(record))
            require(not errors, (index, record_type,
                    [(list(error.path), error.message) for error in errors]))
            counts[record_type] += 1
        require(sum(record["record_type"] == "trace_event.v1"
                    for record in result["records"]) == accounting["accepted_units"],
                (index, "accepted records"))
        require(sum(record["record_type"] == "trace_exclusion.v1"
                    for record in result["records"]) == accounting["excluded_units"],
                (index, "excluded records"))
        per_request.append({"request_index": index, "source_rows": len(rows),
                            "candidate_units": expected_candidates,
                            "outcome_observations": expected_outcomes,
                            "partition": {key: accounting[key] for key in (
                                "accepted_units", "excluded_units", "failed_units",
                                "unresolved_units")}})
    return {"request_count": len(captures), "source_row_count_checks": len(captures),
            "candidate_expansion_checks": len(captures),
            "candidate_partition_checks": len(captures),
            "outcome_population_checks": len(captures), "per_request": per_request,
            "record_counts": counts, "logical_schema_sha256": C0_SHA256,
            "snapshot_sha256": hashlib.sha256(snapshot_path.read_bytes()).hexdigest(),
            "exit_code": 0}


if __name__ == "__main__":
    report = validate(Path(sys.argv[1]), Path(sys.argv[2]))
    report.update(validator_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                  python=sys.version.split()[0], jsonschema=version("jsonschema"),
                  argv=list(sys.orig_argv), cwd=str(Path.cwd()))
    print(json.dumps(report, sort_keys=True))
