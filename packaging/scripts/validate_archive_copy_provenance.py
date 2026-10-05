#!/usr/bin/env python3
"""Validate unsigned local archive-copy provenance against independent inputs."""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import os
import re
import stat
import sys
from dataclasses import dataclass
from decimal import Decimal
from pathlib import Path
from typing import Any
from urllib.parse import quote, urlsplit

STATEMENT_TYPE = "https://in-toto.io/Statement/v1"
PREDICATE_TYPE = "https://slsa.dev/provenance/v1"
BUILD_TYPE = "urn:careops:build-type:verified-archive-copy:v1"
BUILDER_ID = "urn:careops:local-untrusted-builder:archive-copy"
DEFAULT_MAX_BYTES = 8 * 1024 * 1024
MAX_JSON_DEPTH = 128
_SHA256 = re.compile(r"^[0-9a-f]{64}$")
_COMMIT = re.compile(r"^[0-9a-f]{40}$")
_SCHEME = re.compile(r"^[A-Za-z][A-Za-z0-9+.-]*:")
_BAD_PERCENT = re.compile(r"%(?![0-9A-Fa-f]{2})")
_RFC3339_UTC = re.compile(
    r"^(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2})(?:\.(\d+))?Z$"
)
_URI_CHARS = frozenset(
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"
    "-._~:/?#[]@!$&'()*+,;=%"
)


@dataclass(frozen=True)
class ValidationIssue:
    code: str
    path: str
    message: str

    def as_dict(self) -> dict[str, str]:
        return {"code": self.code, "path": self.path, "message": self.message}


class InputError(ValueError):
    """An input file could not be parsed safely."""


def _reject_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise InputError(f"duplicate JSON object key: {key}")
        result[key] = value
    return result


def _reject_non_json_constant(value: str) -> None:
    raise InputError(f"non-JSON numeric constant is not allowed: {value}")


def load_json_bytes(data: bytes, label: str, max_bytes: int = DEFAULT_MAX_BYTES) -> Any:
    """Load bounded UTF-8 JSON and reject duplicate keys at every depth."""
    if len(data) > max_bytes:
        raise InputError(f"{label} exceeds the {max_bytes}-byte limit")
    try:
        text = data.decode("utf-8")
        parsed = json.loads(
            text,
            object_pairs_hook=_reject_duplicate_keys,
            parse_constant=_reject_non_json_constant,
        )
    except (UnicodeDecodeError, json.JSONDecodeError, RecursionError) as exc:
        raise InputError(f"{label} is not valid UTF-8 JSON: {exc}") from exc
    pending = [(parsed, 0)]
    while pending:
        value, depth = pending.pop()
        if depth > MAX_JSON_DEPTH:
            raise InputError(f"{label} exceeds the {MAX_JSON_DEPTH}-level JSON nesting limit")
        if isinstance(value, dict):
            pending.extend((child, depth + 1) for child in value.values())
        elif isinstance(value, list):
            pending.extend((child, depth + 1) for child in value)
    return parsed


def load_json_file(path: Path, label: str, max_bytes: int = DEFAULT_MAX_BYTES) -> tuple[Any, bytes]:
    descriptor = -1
    try:
        flags = os.O_RDONLY | getattr(os, "O_NONBLOCK", 0) | getattr(os, "O_NOFOLLOW", 0)
        descriptor = os.open(path, flags)
        if not stat.S_ISREG(os.fstat(descriptor).st_mode):
            raise InputError(f"{label} must be a regular file")
        with os.fdopen(descriptor, "rb") as handle:
            descriptor = -1
            data = handle.read(max_bytes + 1)
    except OSError as exc:
        raise InputError(f"cannot read {label}: {exc}") from exc
    finally:
        if descriptor >= 0:
            os.close(descriptor)
    if len(data) > max_bytes:
        raise InputError(f"{label} exceeds the {max_bytes}-byte limit")
    return load_json_bytes(data, label, max_bytes), data


def canonical_dependency_uri(original_id: str, run_id: int) -> str:
    """Return the reviewed local URI; retain external HTTPS identifiers verbatim.

    The careops URN-shaped namespace is a local profile convention, not a claim
    of URN registration, public resolution, or trusted publication.
    """
    if not isinstance(original_id, str) or not original_id:
        raise ValueError("dependency identifier must be a non-empty string")
    if isinstance(run_id, bool) or not isinstance(run_id, int) or run_id <= 0:
        raise ValueError("run_id must be a positive integer")
    if original_id.startswith("https://"):
        return original_id
    encoded = quote(original_id, safe="")
    return f"urn:careops:archive-copy:v1:run:{run_id}:dependency:{encoded}"


def is_resource_uri(value: object) -> bool:
    """Check generic RFC 3986 URI syntax and in-toto scheme/authority casing."""
    if not isinstance(value, str) or not value or not value.isascii():
        return False
    if any(char not in _URI_CHARS for char in value):
        return False
    if _BAD_PERCENT.search(value) or not _SCHEME.match(value):
        return False
    try:
        parts = urlsplit(value)
    except ValueError:
        return False
    raw_scheme = value.split(":", 1)[0]
    if not parts.scheme or raw_scheme != raw_scheme.lower():
        return False
    # RFC 3986 requires authority to be present only after //; in-toto requires
    # any scheme/authority components to be case-normalized.
    if value[len(parts.scheme) + 1 :].startswith("//") and parts.netloc != parts.netloc.lower():
        return False
    return True


def _is_positive_int(value: object) -> bool:
    return isinstance(value, int) and not isinstance(value, bool) and value > 0


def _digest(value: object) -> bool:
    return isinstance(value, str) and bool(_SHA256.fullmatch(value))


def _add(issues: list[ValidationIssue], code: str, path: str, message: str) -> None:
    issues.append(ValidationIssue(code, path, message))


def _expected_inputs_valid(expected: object, issues: list[ValidationIssue]) -> bool:
    required = {
        "archive_index_sha256",
        "source_commit",
        "original_run_id",
        "acquisition_artifact_id",
        "dependencies",
    }
    if not isinstance(expected, dict):
        _add(issues, "expected_inputs_type", "$expected", "expected inputs must be a JSON object")
        return False
    if set(expected) != required:
        _add(issues, "expected_inputs_fields", "$expected", "expected inputs must contain exactly the documented fields")
    okay = True
    if not _digest(expected.get("archive_index_sha256")):
        _add(issues, "expected_index_digest", "$expected.archive_index_sha256", "must be lowercase SHA-256 hex")
        okay = False
    if not isinstance(expected.get("source_commit"), str) or not _COMMIT.fullmatch(expected.get("source_commit", "")):
        _add(issues, "expected_source_commit", "$expected.source_commit", "must be a 40-character lowercase Git commit id")
        okay = False
    for field in ("original_run_id", "acquisition_artifact_id"):
        if not _is_positive_int(expected.get(field)):
            _add(issues, "expected_integer", f"$expected.{field}", "must be a positive integer, not a boolean")
            okay = False
    deps = expected.get("dependencies")
    if not isinstance(deps, list) or not deps:
        _add(issues, "expected_dependencies", "$expected.dependencies", "must be a non-empty list")
        return False
    seen: set[str] = set()
    for position, dep in enumerate(deps):
        label = f"$expected.dependencies[{position}]"
        if not isinstance(dep, dict) or set(dep) != {"id", "sha256"}:
            _add(issues, "expected_dependency_shape", label, "each item must contain exactly id and sha256")
            okay = False
            continue
        dep_id = dep.get("id")
        if not isinstance(dep_id, str) or not dep_id:
            _add(issues, "expected_dependency_id", label + ".id", "must be a non-empty string")
            okay = False
        elif dep_id in seen:
            _add(issues, "expected_dependency_duplicate", label + ".id", "dependency IDs must be unique")
            okay = False
        else:
            seen.add(dep_id)
        if not _digest(dep.get("sha256")):
            _add(issues, "expected_dependency_digest", label + ".sha256", "must be lowercase SHA-256 hex")
            okay = False
    return okay


def _index_subjects(index: object, actual_index_sha256: str, expected: dict[str, Any], issues: list[ValidationIssue]) -> dict[str, str] | None:
    if actual_index_sha256 != expected.get("archive_index_sha256"):
        _add(issues, "archive_index_hash_mismatch", "$archive_index", "archive index bytes do not match the independently supplied SHA-256")
    if not isinstance(index, dict) or not isinstance(index.get("artifacts"), list):
        _add(issues, "archive_index_shape", "$archive_index.artifacts", "archive index must contain an artifacts array")
        return None
    if index.get("source_commit") != expected.get("source_commit"):
        _add(issues, "archive_index_source_commit", "$archive_index.source_commit", "must match the independently supplied source commit")
    output: dict[str, str] = {}
    for pos, item in enumerate(index["artifacts"]):
        label = f"$archive_index.artifacts[{pos}]"
        if not isinstance(item, dict):
            _add(issues, "archive_index_artifact_shape", label, "artifact entry must be an object")
            continue
        path = item.get("path")
        digest = item.get("sha256")
        if not isinstance(path, str) or not path or path.startswith("/") or "\\" in path:
            _add(issues, "archive_index_path", label + ".path", "must be a non-empty relative POSIX path")
            continue
        parts = path.split("/")
        if any(part in ("", ".", "..") for part in parts):
            _add(issues, "archive_index_path", label + ".path", "must not contain empty, dot, or parent segments")
            continue
        if not _digest(digest):
            _add(issues, "archive_index_digest", label + ".sha256", "must be lowercase SHA-256 hex")
            continue
        artifact_builder = item.get("builder")
        if not isinstance(artifact_builder, dict) or artifact_builder.get("source_commit") != expected.get("source_commit"):
            _add(issues, "archive_index_artifact_source_commit", label + ".builder.source_commit", "must match the independently supplied source commit")
        name = "archives/" + path
        if name in output:
            _add(issues, "archive_index_duplicate_path", label + ".path", "archive paths must be unique")
            continue
        output[name] = digest
    if not output:
        _add(issues, "archive_index_empty", "$archive_index.artifacts", "must contain at least one valid artifact")
        return None
    return output


def _parse_z_timestamp(value: object) -> tuple[dt.datetime, Decimal] | None:
    if not isinstance(value, str):
        return None
    match = _RFC3339_UTC.fullmatch(value)
    if match is None:
        return None
    try:
        parsed = dt.datetime.fromisoformat(match.group(1) + "+00:00")
    except ValueError:
        return None
    fraction = Decimal("0." + match.group(2)) if match.group(2) else Decimal(0)
    return (parsed, fraction) if parsed.utcoffset() == dt.timedelta(0) else None


def validate_provenance(
    statement: object,
    archive_index: object,
    expected: object,
    actual_index_sha256: str,
) -> list[ValidationIssue]:
    """Validate a statement against independently supplied inputs and index."""
    issues: list[ValidationIssue] = []
    if not _expected_inputs_valid(expected, issues):
        return issues
    assert isinstance(expected, dict)
    subjects_from_index = _index_subjects(archive_index, actual_index_sha256, expected, issues)

    if not isinstance(statement, dict):
        _add(issues, "statement_type", "$", "statement must be a JSON object")
        return issues
    if statement.get("_type") != STATEMENT_TYPE:
        _add(issues, "statement_type_id", "$_type", f"must equal {STATEMENT_TYPE}")
    if statement.get("predicateType") != PREDICATE_TYPE:
        _add(issues, "predicate_type", "$.predicateType", f"must equal {PREDICATE_TYPE}")
    if not isinstance(statement.get("subject"), list) or not statement.get("subject"):
        _add(issues, "subject_array", "$.subject", "must be a non-empty array")
    else:
        actual_subjects: dict[str, str] = {}
        for pos, subject in enumerate(statement["subject"]):
            label = f"$.subject[{pos}]"
            if not isinstance(subject, dict):
                _add(issues, "subject_shape", label, "subject must be an object")
                continue
            name = subject.get("name")
            if not isinstance(name, str) or not name:
                _add(issues, "subject_name", label + ".name", "must be a non-empty string")
                continue
            if name in actual_subjects:
                _add(issues, "subject_duplicate_name", label + ".name", "subject names must be unique")
                continue
            digest = subject.get("digest")
            if not isinstance(digest, dict) or set(digest) != {"sha256"} or not _digest(digest.get("sha256")):
                _add(issues, "subject_digest", label + ".digest", "must contain exactly one lowercase SHA-256 digest")
                continue
            actual_subjects[name] = digest["sha256"]
            if "uri" in subject and not is_resource_uri(subject["uri"]):
                _add(issues, "subject_uri", label + ".uri", "must be a normalized absolute ResourceURI")
        if subjects_from_index is not None:
            if set(actual_subjects) != set(subjects_from_index):
                _add(issues, "subject_set", "$.subject", "subject names must exactly match the bound archive index")
            for name in set(actual_subjects) & set(subjects_from_index):
                if actual_subjects[name] != subjects_from_index[name]:
                    _add(issues, "subject_digest_mismatch", f"$.subject[{name}]", "digest differs from the bound archive index")

    predicate = statement.get("predicate")
    if not isinstance(predicate, dict):
        _add(issues, "predicate_shape", "$.predicate", "SLSA predicate must be an object")
        return issues
    build = predicate.get("buildDefinition")
    run = predicate.get("runDetails")
    if not isinstance(build, dict):
        _add(issues, "build_definition", "$.predicate.buildDefinition", "must be an object")
        return issues
    if not isinstance(run, dict):
        _add(issues, "run_details", "$.predicate.runDetails", "must be an object")
        return issues
    if build.get("buildType") != BUILD_TYPE:
        _add(issues, "build_type", "$.predicate.buildDefinition.buildType", f"must equal {BUILD_TYPE}")

    external = build.get("externalParameters")
    expected_external = {
        "source_commit": expected["source_commit"],
        "original_run_id": expected["original_run_id"],
    }
    if (
        not isinstance(external, dict)
        or set(external) != set(expected_external)
        or external.get("source_commit") != expected_external["source_commit"]
        or not _is_positive_int(external.get("original_run_id"))
        or external.get("original_run_id") != expected_external["original_run_id"]
    ):
        _add(issues, "external_parameters", "$.predicate.buildDefinition.externalParameters", "must exactly match the independently supplied source commit and run ID; unknown keys are rejected")

    internal = build.get("internalParameters")
    if (
        not isinstance(internal, dict)
        or not _is_positive_int(internal.get("acquisition_artifact_id"))
        or internal.get("acquisition_artifact_id") != expected["acquisition_artifact_id"]
    ):
        _add(issues, "acquisition_artifact_id", "$.predicate.buildDefinition.internalParameters.acquisition_artifact_id", "must match the independently supplied positive artifact ID")

    builder = run.get("builder")
    if not isinstance(builder, dict) or builder.get("id") != BUILDER_ID:
        _add(issues, "builder_id", "$.predicate.runDetails.builder.id", "must identify the declared local untrusted archive-copy builder")
    metadata = run.get("metadata")
    if not isinstance(metadata, dict):
        _add(issues, "metadata_shape", "$.predicate.runDetails.metadata", "must be an object")
    else:
        started = _parse_z_timestamp(metadata.get("startedOn"))
        finished = _parse_z_timestamp(metadata.get("finishedOn"))
        if started is None:
            _add(issues, "timestamp_utc_z", "$.predicate.runDetails.metadata.startedOn", "must be a valid RFC 3339 UTC timestamp ending in literal Z")
        if finished is None:
            _add(issues, "timestamp_utc_z", "$.predicate.runDetails.metadata.finishedOn", "must be a valid RFC 3339 UTC timestamp ending in literal Z")
        if started is not None and finished is not None and started > finished:
            _add(issues, "timestamp_order", "$.predicate.runDetails.metadata", "startedOn must not be later than finishedOn")

    dep_map: dict[str, tuple[str, str]] = {}
    for pos, item in enumerate(expected["dependencies"]):
        if isinstance(item, dict) and isinstance(item.get("id"), str) and _digest(item.get("sha256")):
            uri = canonical_dependency_uri(item["id"], expected["original_run_id"])
            if not is_resource_uri(uri):
                _add(issues, "expected_dependency_uri", f"$expected.dependencies[{pos}].id", "canonical dependency URI is invalid")
            dep_map[uri] = (item["sha256"], item["id"])
    if len(dep_map) != sum(
        1 for item in expected["dependencies"]
        if isinstance(item, dict) and isinstance(item.get("id"), str) and _digest(item.get("sha256"))
    ):
        _add(issues, "expected_dependency_uri_collision", "$expected.dependencies", "canonical dependency URIs must be unique")
    for index_id in ("ARCHIVE-INDEX.json", "build-inputs/ARCHIVE-INDEX.json"):
        index_uri = canonical_dependency_uri(index_id, expected["original_run_id"])
        if dep_map.get(index_uri, (None, None))[0] != expected["archive_index_sha256"]:
            _add(issues, "expected_index_dependency", "$expected.dependencies", f"{index_id} must bind the exact archive index bytes")
    actual_deps = build.get("resolvedDependencies")
    if not isinstance(actual_deps, list):
        _add(issues, "dependencies_array", "$.predicate.buildDefinition.resolvedDependencies", "must be an array")
    else:
        actual_map: dict[str, tuple[str, str]] = {}
        for pos, item in enumerate(actual_deps):
            label = f"$.predicate.buildDefinition.resolvedDependencies[{pos}]"
            if not isinstance(item, dict):
                _add(issues, "dependency_shape", label, "dependency must be an object")
                continue
            uri = item.get("uri")
            if not is_resource_uri(uri):
                _add(issues, "dependency_uri", label + ".uri", "must be a normalized absolute in-toto ResourceURI; relative paths are invalid")
                continue
            if uri in actual_map:
                _add(issues, "dependency_duplicate_uri", label + ".uri", "dependency URIs must be unique")
                continue
            digest = item.get("digest")
            if not isinstance(digest, dict) or set(digest) != {"sha256"} or not _digest(digest.get("sha256")):
                _add(issues, "dependency_digest", label + ".digest", "must contain exactly one lowercase SHA-256 digest")
                continue
            name = item.get("name")
            if not isinstance(name, str) or not name:
                _add(issues, "dependency_name", label + ".name", "must preserve the original dependency identifier")
                continue
            actual_map[uri] = (digest["sha256"], name)
        if set(actual_map) != set(dep_map):
            _add(issues, "dependency_set", "$.predicate.buildDefinition.resolvedDependencies", "dependency URI set must exactly match the independently supplied dependency map")
        for uri in set(actual_map) & set(dep_map):
            if actual_map[uri][0] != dep_map[uri][0]:
                _add(issues, "dependency_digest_mismatch", f"$.predicate.buildDefinition.resolvedDependencies[{uri}]", "digest differs from the independently supplied source map")
            if actual_map[uri][1] != dep_map[uri][1]:
                _add(issues, "dependency_name_mismatch", f"$.predicate.buildDefinition.resolvedDependencies[{uri}].name", "must preserve the original dependency identifier")
    return issues


def _main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--statement", required=True, type=Path)
    parser.add_argument("--archive-index", required=True, type=Path)
    parser.add_argument("--expected-inputs", required=True, type=Path)
    parser.add_argument("--max-bytes", type=int, default=DEFAULT_MAX_BYTES)
    args = parser.parse_args(argv)
    if args.max_bytes < 1 or args.max_bytes > 64 * 1024 * 1024:
        parser.error("--max-bytes must be from 1 to 67108864")
    try:
        statement, statement_bytes = load_json_file(args.statement, "statement", args.max_bytes)
        archive_index, index_bytes = load_json_file(args.archive_index, "archive index", args.max_bytes)
        expected, _ = load_json_file(args.expected_inputs, "expected inputs", args.max_bytes)
    except InputError as exc:
        print(json.dumps({"valid": False, "input_error": str(exc)}, sort_keys=True), file=sys.stderr)
        return 2
    issues = validate_provenance(
        statement,
        archive_index,
        expected,
        hashlib.sha256(index_bytes).hexdigest(),
    )
    result = {
        "valid": not issues,
        "statement_sha256": hashlib.sha256(statement_bytes).hexdigest(),
        "archive_index_sha256": hashlib.sha256(index_bytes).hexdigest(),
        "issue_count": len(issues),
        "issues": [issue.as_dict() for issue in issues],
        "claim_scope": "local consistency only; unsigned and untrusted builder; no SLSA level or release acceptance",
    }
    print(json.dumps(result, sort_keys=True, indent=2))
    return 0 if not issues else 1


if __name__ == "__main__":
    raise SystemExit(_main())
