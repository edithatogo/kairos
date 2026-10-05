#!/usr/bin/env python3
"""Retain one exact, successful mainline archive-evidence consumer artifact.

This is an acquisition boundary only. It does not accept release evidence,
build packages, scan archives, publish, or infer the latest workflow run.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
from pathlib import Path, PurePosixPath
import re
import shutil
import secrets
import stat
import struct
import tempfile
import types
import unicodedata
import zipfile
from typing import Any, Callable


REPOSITORY = "edithatogo/kairos"
WORKFLOW = ".github/workflows/archive-supply-chain-main.yml"
PROFILE = "kairos-archive-copy-evidence-v1"
CLAIM_SCOPE = "local consistency only; unsigned and untrusted builder; no SLSA level or release acceptance"
MAX_PIN_BYTES = 128 * 1024 * 1024
MAX_EXPANDED_BYTES = 512 * 1024 * 1024
MAX_MEMBER_BYTES = 64 * 1024 * 1024
MAX_MEMBERS = 20_000
MAX_API_BYTES = 8 * 1024 * 1024
MAX_JSON_BYTES = 8 * 1024 * 1024
API_TIMEOUT_SECONDS = 60
DOWNLOAD_TIMEOUT_SECONDS = 300
SHA256 = re.compile(r"^[0-9a-f]{64}$")
COMMIT = re.compile(r"^[0-9a-f]{40}$")

EVIDENCE_ROOT_FILES = {
    "RELEASE.txt", "SBOM-COVERAGE.json", "SHA256SUMS", "SUPPLY-CHAIN-SHA256SUMS",
    "expected-inputs.json", "provenance.json", "release-artifact-manifest.json",
    "sbom.spdx.json", "validation-result.json",
}
BUILD_INPUT_FILES = {"ARCHIVE-INDEX.json", "BUILD-RECEIPT.json", "acquisition.json"}
ACQUISITION_FILES = {
    "receipt.json", "artifact-metadata.json", "run-metadata.json",
    "source-commit-readback.json",
}


class AcquisitionError(ValueError):
    """A stable, non-secret acquisition failure."""


def fail(code: str) -> None:
    raise AcquisitionError(code)


def canonical_json(value: Any) -> bytes:
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, indent=2, allow_nan=False) + "\n").encode("utf-8")


def strict_json(data: bytes, label: str, limit: int = MAX_JSON_BYTES) -> Any:
    if len(data) > limit:
        fail(label + "_too_large")

    def pairs(items: list[tuple[str, Any]]) -> dict[str, Any]:
        result: dict[str, Any] = {}
        for key, value in items:
            if key in result:
                fail(label + "_duplicate_key")
            result[key] = value
        return result

    def reject_constant(_: str) -> None:
        fail(label + "_non_finite")

    def finite_float(raw: str) -> float:
        value = float(raw)
        if not math.isfinite(value):
            fail(label + "_non_finite")
        return value

    try:
        value = json.loads(data.decode("utf-8"), object_pairs_hook=pairs,
                           parse_constant=reject_constant, parse_float=finite_float)
    except AcquisitionError:
        raise
    except (UnicodeDecodeError, json.JSONDecodeError, ValueError, RecursionError) as exc:
        raise AcquisitionError(label + "_invalid_json") from exc
    stack = [(value, 0)]
    while stack:
        current, depth = stack.pop()
        if depth > 128:
            fail(label + "_too_deep")
        if isinstance(current, dict):
            stack.extend((child, depth + 1) for child in current.values())
        elif isinstance(current, list):
            stack.extend((child, depth + 1) for child in current)
    return value


def _open_nofollow(path: Path, *, directory: bool = False) -> int:
    absolute = Path(os.path.abspath(path))
    if absolute == Path(absolute.anchor):
        fail("path_root_not_allowed")
    directory_flags = os.O_RDONLY | getattr(os, "O_DIRECTORY", 0) | getattr(os, "O_NOFOLLOW", 0)
    file_flags = os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0) | getattr(os, "O_NONBLOCK", 0)
    descriptor = os.open(absolute.anchor, directory_flags)
    try:
        for component in absolute.parts[1:-1]:
            next_descriptor = os.open(component, directory_flags, dir_fd=descriptor)
            os.close(descriptor)
            descriptor = next_descriptor
        flags = directory_flags if directory else file_flags
        leaf = os.open(absolute.parts[-1], flags, dir_fd=descriptor)
        return leaf
    except OSError as exc:
        raise AcquisitionError("path_open_failed") from exc
    finally:
        os.close(descriptor)


def read_regular(path: Path, limit: int, label: str) -> bytes:
    descriptor = _open_nofollow(path)
    try:
        info = os.fstat(descriptor)
        if not stat.S_ISREG(info.st_mode) or info.st_nlink != 1 or info.st_size > limit:
            fail(label + "_not_bounded_regular")
        data = bytearray()
        while len(data) <= limit:
            chunk = os.read(descriptor, min(65536, limit + 1 - len(data)))
            if not chunk:
                break
            data.extend(chunk)
        if len(data) > limit:
            fail(label + "_too_large")
        return bytes(data)
    except OSError as exc:
        raise AcquisitionError(label + "_read_failed") from exc
    finally:
        os.close(descriptor)


def positive_integer(raw: str, label: str, maximum: int = 2**63 - 1) -> int:
    if not re.fullmatch(r"[1-9][0-9]{0,18}", raw):
        fail(label + "_invalid_integer")
    value = int(raw)
    if value > maximum:
        fail(label + "_integer_out_of_range")
    return value


def load_captured_helper(expected_sha256: str) -> types.ModuleType:
    if not isinstance(expected_sha256, str) or not SHA256.fullmatch(expected_sha256):
        fail("acquisition_helper_pin_invalid")
    root = Path(__file__).absolute().parent.parent.parent
    helper_path = root / "packaging/scripts/acquire_package_archive_bundle.py"
    source = read_regular(helper_path, 2 * MAX_JSON_BYTES, "acquisition_helper")
    if hashlib.sha256(source).hexdigest() != expected_sha256:
        fail("acquisition_helper_pin_mismatch")
    # Execute only the exact bytes that were just verified; never reopen source.
    module = types.ModuleType("_captured_package_acquisition_helper")
    module.__file__ = "packaging/scripts/acquire_package_archive_bundle.py"
    try:
        exec(compile(source, module.__file__, "exec", dont_inherit=True), module.__dict__)
    except Exception as exc:
        raise AcquisitionError("acquisition_helper_load_failed") from exc
    for name in ("_run_bounded_process", "MAX_SUBPROCESS_TIMEOUT_SECONDS", "API_TIMEOUT_SECONDS",
                 "API_OUTPUT_LIMIT", "DOWNLOAD_TIMEOUT_SECONDS", "MAX_BYTES", "API_COMMAND_PREFIX"):
        if not hasattr(module, name):
            fail("acquisition_helper_contract_mismatch")
    return module


def api_readback(helper: types.ModuleType, endpoint: str) -> tuple[bytes, dict[str, Any]]:
    if not endpoint.startswith("repos/" + REPOSITORY + "/") or "\n" in endpoint:
        fail("api_endpoint_invalid")
    try:
        _, data = helper._run_bounded_process(
            [*helper.API_COMMAND_PREFIX, endpoint], min(helper.API_OUTPUT_LIMIT, MAX_API_BYTES),
            min(helper.API_TIMEOUT_SECONDS, API_TIMEOUT_SECONDS))
    except Exception as exc:
        raise AcquisitionError("api_readback_failed") from exc
    if not isinstance(data, bytes):
        fail("api_readback_missing_bytes")
    value = strict_json(data, "api_readback")
    if not isinstance(value, dict):
        fail("api_readback_not_object")
    return data, value


def validate_run(run: dict[str, Any], run_id: int, attempt: int, source_commit: str) -> tuple[int, int]:
    if type(run.get("id")) is not int or run["id"] != run_id:
        fail("consumer_run_id_mismatch")
    if type(run.get("run_attempt")) is not int or run["run_attempt"] != attempt:
        fail("consumer_run_attempt_mismatch")
    if run.get("path") != WORKFLOW or run.get("status") != "completed" or run.get("conclusion") != "success":
        fail("consumer_run_not_successful")
    if run.get("event") != "workflow_dispatch" or run.get("head_branch") != "main" or run.get("pull_requests") != []:
        fail("consumer_run_not_main_dispatch")
    if run.get("head_sha") != source_commit or not COMMIT.fullmatch(source_commit):
        fail("consumer_run_source_mismatch")
    repository, head_repository = run.get("repository"), run.get("head_repository")
    if not isinstance(repository, dict) or not isinstance(head_repository, dict):
        fail("consumer_run_repository_missing")
    if repository.get("full_name") != REPOSITORY or head_repository.get("full_name") != REPOSITORY:
        fail("consumer_run_repository_mismatch")
    repository_id, head_repository_id = repository.get("id"), head_repository.get("id")
    if (type(repository_id) is not int or repository_id <= 0 or type(head_repository_id) is not int
            or head_repository_id != repository_id):
        fail("consumer_run_repository_id_mismatch")
    return repository_id, head_repository_id


def validate_artifact(artifact: dict[str, Any], *, run_id: int, attempt: int, artifact_id: int,
                      source_commit: str, repository_id: int, archive_zip_sha256: str,
                      archive_zip_bytes: int) -> None:
    expected_name = f"archive-main-evidence-{run_id}-{attempt}"
    if type(artifact.get("id")) is not int or artifact["id"] != artifact_id:
        fail("consumer_artifact_id_mismatch")
    if artifact.get("name") != expected_name:
        fail("consumer_artifact_name_mismatch")
    if artifact.get("expired") is not False:
        fail("consumer_artifact_expired")
    digest = artifact.get("digest")
    if digest != "sha256:" + archive_zip_sha256:
        fail("consumer_artifact_digest_mismatch")
    if type(artifact.get("size_in_bytes")) is not int or artifact["size_in_bytes"] != archive_zip_bytes:
        fail("consumer_artifact_size_mismatch")
    workflow_run = artifact.get("workflow_run")
    if not isinstance(workflow_run, dict):
        fail("consumer_artifact_origin_missing")
    expected = {"id": run_id, "repository_id": repository_id, "head_repository_id": repository_id}
    if any(type(workflow_run.get(key)) is not int or workflow_run[key] != value for key, value in expected.items()):
        fail("consumer_artifact_origin_mismatch")
    if workflow_run.get("head_sha") != source_commit or workflow_run.get("head_branch") != "main":
        fail("consumer_artifact_source_mismatch")


def _safe_member_name(info: zipfile.ZipInfo) -> tuple[str, bool]:
    raw = info.filename
    is_directory = info.is_dir()
    normalized = unicodedata.normalize("NFC", raw)
    if (not raw or raw != normalized or "\\" in raw or ":" in raw or len(raw) > 1024
            or raw.startswith("/") or raw.startswith("//")):
        fail("consumer_zip_member_path_invalid")
    path_value = raw[:-1] if is_directory else raw
    path = PurePosixPath(path_value)
    if (not path_value or path.is_absolute() or path.as_posix() != path_value
            or any(part in {"", ".", ".."} for part in path.parts) or len(path.parts) > 32):
        fail("consumer_zip_member_path_invalid")
    mode = info.external_attr >> 16
    file_type = stat.S_IFMT(mode)
    allowed_type = stat.S_IFDIR if is_directory else stat.S_IFREG
    if file_type not in (0, allowed_type):
        fail("consumer_zip_member_special_type")
    if (is_directory and info.file_size != 0) or (not is_directory and info.file_size > MAX_MEMBER_BYTES):
        fail("consumer_zip_member_size_invalid")
    if info.flag_bits & 1 or info.compress_type not in (zipfile.ZIP_STORED, zipfile.ZIP_DEFLATED):
        fail("consumer_zip_member_encoding_invalid")
    if info.file_size and info.compress_size == 0:
        fail("consumer_zip_member_ratio_invalid")
    if info.compress_size and info.file_size > info.compress_size * 1000:
        fail("consumer_zip_member_ratio_invalid")
    return path_value, is_directory


def preflight_zip(archive: Path, expected_digest: str, expected_bytes: int) -> list[tuple[str, bool]]:
    info = archive.lstat()
    if not stat.S_ISREG(info.st_mode) or info.st_nlink != 1 or info.st_size != expected_bytes:
        fail("consumer_zip_file_size_mismatch")
    if info.st_size > MAX_PIN_BYTES:
        fail("consumer_zip_too_large")
    digest = hashlib.sha256()
    with archive.open("rb") as stream:
        while block := stream.read(1024 * 1024):
            digest.update(block)
    if digest.hexdigest() != expected_digest:
        fail("consumer_zip_digest_mismatch")
    _preflight_eocd(archive, expected_bytes)
    try:
        with zipfile.ZipFile(archive) as zipped:
            members = zipped.infolist()
            if not members or len(members) > MAX_MEMBERS:
                fail("consumer_zip_member_count_invalid")
            expanded = 0
            aliases: set[str] = set()
            files: set[str] = set()
            directories: set[str] = set()
            result = []
            for member in members:
                name, is_directory = _safe_member_name(member)
                alias = name.rstrip("/").casefold()
                if alias in aliases:
                    fail("consumer_zip_duplicate_or_case_alias")
                aliases.add(alias)
                expanded += member.file_size
                if expanded > MAX_EXPANDED_BYTES:
                    fail("consumer_zip_expanded_size_exceeded")
                (directories if is_directory else files).add(name.rstrip("/"))
                result.append((name, is_directory))
            for file_name in files:
                parts = PurePosixPath(file_name).parts
                if any("/".join(parts[:index]) in files for index in range(1, len(parts))):
                    fail("consumer_zip_file_directory_conflict")
            for directory_name in directories:
                parts = PurePosixPath(directory_name).parts
                if any("/".join(parts[:index]) in files for index in range(1, len(parts) + 1)):
                    fail("consumer_zip_file_directory_conflict")
            if files & directories:
                fail("consumer_zip_file_directory_conflict")
            return result
    except AcquisitionError:
        raise
    except (OSError, zipfile.BadZipFile, RuntimeError, ValueError) as exc:
        raise AcquisitionError("consumer_zip_invalid") from exc


def _preflight_eocd(archive: Path, file_bytes: int) -> None:
    """Bound central-directory entry allocation before ZipFile reads its index."""
    tail_size = min(file_bytes, 22 + 65535 + 20)
    with archive.open("rb") as stream:
        stream.seek(file_bytes - tail_size)
        tail = stream.read(tail_size)
    signature = b"PK\x05\x06"
    cursor = len(tail)
    eocd = None
    while True:
        position = tail.rfind(signature, 0, cursor)
        if position < 0:
            break
        if position + 22 <= len(tail):
            fields = struct.unpack_from("<4s4H2LH", tail, position)
            comment_length = fields[-1]
            if position + 22 + comment_length == len(tail):
                eocd = (position, fields)
                break
        cursor = position
    if eocd is None:
        fail("consumer_zip_eocd_missing")
    position, fields = eocd
    _, disk_number, central_disk, disk_entries, total_entries, central_bytes, central_offset, _ = fields
    absolute_eocd = file_bytes - tail_size + position
    locator_position = absolute_eocd - 20
    with archive.open("rb") as stream:
        if locator_position >= 0:
            stream.seek(locator_position)
            locator = stream.read(4)
            if locator == b"PK\x06\x07":
                fail("consumer_zip64_not_supported")
    if (disk_number != 0 or central_disk != 0 or disk_entries != total_entries
            or total_entries in (0, 0xFFFF) or central_bytes == 0xFFFFFFFF or central_offset == 0xFFFFFFFF
            or total_entries > MAX_MEMBERS):
        fail("consumer_zip_central_directory_bounds")
    if central_offset + central_bytes != absolute_eocd:
        fail("consumer_zip_central_directory_bounds")


def validate_explicit_directories(members: list[tuple[str, bool]], run_id: int, attempt: int) -> None:
    files = {name for name, is_directory in members if not is_directory}
    allowed_prefixes = _consumer_prefixes(run_id, attempt)
    for name, is_directory in members:
        if not is_directory:
            continue
        normalized = name.rstrip("/")
        if not any(normalized == prefix or normalized.startswith(prefix + "/") for prefix in allowed_prefixes):
            fail("consumer_layout_extra_directory")
        if not any(path.startswith(normalized + "/") for path in files):
            fail("consumer_layout_empty_directory")


def _consumer_prefixes(run_id: int, attempt: int) -> tuple[str, str, str]:
    return (f"archive-evidence-{run_id}-{attempt}", f"archive-acquisition-{run_id}-{attempt}",
            f"syft-linux-amd64-{run_id}-{attempt}")


def validate_consumer_layout(root: Path, run_id: int, attempt: int, source_commit: str) -> dict[str, Any]:
    evidence_root, acquisition_root, syft_root = _consumer_prefixes(run_id, attempt)
    actual_files: set[str] = set()
    for current, dirs, files in os.walk(root, followlinks=False):
        base = Path(current)
        for name in dirs:
            if (base / name).is_symlink():
                fail("consumer_layout_symlink")
        for name in files:
            path = base / name
            info = path.lstat()
            if not stat.S_ISREG(info.st_mode) or info.st_nlink != 1:
                fail("consumer_layout_nonregular")
            actual_files.add(path.relative_to(root).as_posix())

    def under(prefix: str, rel: str) -> str:
        return f"{prefix}/{rel}"

    required = {
        under(evidence_root, "preparation-receipt.json"),
        under(evidence_root, "expectations/acquisition.json"),
        under(evidence_root, "expectations/expected-inputs.json"),
        under(evidence_root, "expectations/outer-binding.json"),
        under(evidence_root, "validation-report.json"),
        *(under(evidence_root, "evidence/" + name) for name in EVIDENCE_ROOT_FILES),
        *(under(evidence_root, "evidence/build-inputs/" + name) for name in BUILD_INPUT_FILES),
        under(acquisition_root, "receipt.json"),
        under(acquisition_root, "artifact-metadata.json"),
        under(acquisition_root, "run-metadata.json"),
        under(acquisition_root, "source-commit-readback.json"),
        under(syft_root, "evidence/receipt.json"),
        under(syft_root, "evidence/validation-report.json"),
    }
    if not required.issubset(actual_files):
        fail("consumer_layout_missing_required_file")
    allowed: set[str] = set(required)
    allowed.update(under(acquisition_root, name) for name in ACQUISITION_FILES)
    log_prefix = under(syft_root, "logs/")
    allowed.update(path for path in actual_files
                   if path.startswith(log_prefix)
                   and "/" not in path[len(log_prefix):]
                   and re.fullmatch(r"[A-Za-z0-9._-]+\.log", path[len(log_prefix):]))
    allowed.update(path for path in actual_files if path.startswith(under(evidence_root, "evidence/archives/")))
    allowed.update(path for path in actual_files if path.startswith(under(evidence_root, "evidence/component-sboms/")))
    if actual_files != allowed:
        fail("consumer_layout_extra_or_unrecognized_file")
    if not any(path in allowed and path.startswith(log_prefix) for path in actual_files):
        fail("consumer_layout_missing_native_logs")
    archive_files = sorted(path for path in actual_files if path.startswith(under(evidence_root, "evidence/archives/")))
    allowed_extensions = {"rust": (".crate",), "python": (".whl", ".tar.gz"), "r": (".tar.gz",),
                          "julia": (".tar.gz",), "typescript": (".tgz",), "nuget": (".nupkg",),
                          "go": (".tar.gz",)}
    observed_ecosystems = set()
    for path in archive_files:
        parts = path.split("/")
        if len(parts) != 5 or parts[3] not in allowed_extensions or not any(parts[4].endswith(ext) for ext in allowed_extensions[parts[3]]):
            fail("consumer_layout_archive_subject_invalid")
        observed_ecosystems.add(parts[3])
    if len(archive_files) != 8 or len(observed_ecosystems) != 7:
        fail("consumer_layout_archive_subject_count")
    component_files = sorted(path for path in actual_files if path.startswith(under(evidence_root, "evidence/component-sboms/")))
    component_rows: dict[str, set[str]] = {}
    component_pattern = re.compile(r"^" + re.escape(under(evidence_root, "evidence/component-sboms/"))
                                   + r"([0-9a-f]{64})\.(spdx\.json|stdout|stderr)$")
    for path in component_files:
        match = component_pattern.fullmatch(path)
        if match is None:
            fail("consumer_layout_component_sbom_path_invalid")
        component_rows.setdefault(match.group(1), set()).add(match.group(2))
    if len(component_rows) != 8 or any(suffixes != {"spdx.json", "stdout", "stderr"} for suffixes in component_rows.values()):
        fail("consumer_layout_component_sbom_count")

    top_roots = {path.split("/", 1)[0] for path in actual_files}
    if top_roots != {evidence_root, acquisition_root, syft_root}:
        fail("consumer_layout_top_level_mismatch")

    report = strict_json(read_regular(root / under(evidence_root, "validation-report.json"), MAX_JSON_BYTES,
                                      "consumer_validation_report"), "consumer_validation_report")
    expected_report = {"valid", "profile", "archive_count", "ecosystem_count", "spdx_document_count",
                       "evidence_file_count", "archive_index_sha256", "statement_sha256", "claim_scope"}
    if not isinstance(report, dict) or set(report) != expected_report:
        fail("consumer_validation_report_shape")
    if (report["valid"] is not True or report["profile"] != PROFILE or report["claim_scope"] != CLAIM_SCOPE
            or type(report["archive_count"]) is not int or report["archive_count"] != 8
            or type(report["ecosystem_count"]) is not int or report["ecosystem_count"] != 7
            or type(report["spdx_document_count"]) is not int or report["spdx_document_count"] != 9
            or type(report["evidence_file_count"]) is not int or report["evidence_file_count"] != 44
            or not isinstance(report["archive_index_sha256"], str) or not SHA256.fullmatch(report["archive_index_sha256"])
            or not isinstance(report["statement_sha256"], str) or not SHA256.fullmatch(report["statement_sha256"])):
        fail("consumer_validation_report_profile")

    prep = strict_json(read_regular(root / under(evidence_root, "preparation-receipt.json"), MAX_JSON_BYTES,
                                    "consumer_preparation"), "consumer_preparation")
    if (not isinstance(prep, dict) or prep.get("kind") != "archive-evidence-expectations-preparation"
            or prep.get("trusted_consumer_sha") != source_commit):
        fail("consumer_preparation_identity_mismatch")
    producer_pins = prep.get("producer_pins")
    if (not isinstance(producer_pins, dict)
            or set(producer_pins) != {"repository", "source_commit", "run_id", "artifact_id", "producer_tree",
                                     "archive_zip_sha256", "archive_zip_bytes"}
            or producer_pins.get("repository") != REPOSITORY
            or not isinstance(producer_pins.get("source_commit"), str)
            or not COMMIT.fullmatch(producer_pins["source_commit"])
            or type(producer_pins.get("run_id")) is not int or producer_pins["run_id"] <= 0
            or type(producer_pins.get("artifact_id")) is not int or producer_pins["artifact_id"] <= 0
            or not isinstance(producer_pins.get("producer_tree"), str)
            or not COMMIT.fullmatch(producer_pins["producer_tree"])
            or not isinstance(producer_pins.get("archive_zip_sha256"), str)
            or not SHA256.fullmatch(producer_pins["archive_zip_sha256"])
            or type(producer_pins.get("archive_zip_bytes")) is not int
            or not 1 <= producer_pins["archive_zip_bytes"] <= MAX_PIN_BYTES):
        fail("consumer_preparation_producer_mismatch")
    outer = strict_json(read_regular(root / under(evidence_root, "expectations/outer-binding.json"), MAX_JSON_BYTES,
                                     "consumer_outer_binding"), "consumer_outer_binding")
    expected_inputs = strict_json(read_regular(root / under(evidence_root, "expectations/expected-inputs.json"), MAX_JSON_BYTES,
                                               "consumer_expected_inputs"), "consumer_expected_inputs")
    acquisition = strict_json(read_regular(root / under(evidence_root, "expectations/acquisition.json"), MAX_JSON_BYTES,
                                           "consumer_derived_acquisition"), "consumer_derived_acquisition")
    for value, label in ((outer, "outer"), (expected_inputs, "expected_inputs"), (acquisition, "derived_acquisition")):
        if not isinstance(value, dict):
            fail("consumer_" + label + "_not_object")
    if (set(outer) != {"acquisition_artifact_id", "archive_zip_bytes", "archive_zip_sha256", "original_run_id",
                       "producer_pr_head", "producer_tree", "repository", "schema_version", "source_commit",
                       "spdx_schema_sha256"}
            or outer.get("repository") != REPOSITORY or type(outer.get("schema_version")) is not int
            or outer.get("schema_version") != 1 or outer.get("source_commit") != producer_pins["source_commit"]
            or outer.get("producer_pr_head") != producer_pins["source_commit"]
            or type(outer.get("original_run_id")) is not int or outer["original_run_id"] != producer_pins["run_id"]
            or type(outer.get("acquisition_artifact_id")) is not int or outer["acquisition_artifact_id"] != producer_pins["artifact_id"]
            or outer.get("producer_tree") != producer_pins["producer_tree"]
            or outer.get("archive_zip_sha256") != producer_pins["archive_zip_sha256"]
            or type(outer.get("archive_zip_bytes")) is not int or outer["archive_zip_bytes"] != producer_pins["archive_zip_bytes"]
            or not isinstance(outer.get("spdx_schema_sha256"), str)
            or not SHA256.fullmatch(outer["spdx_schema_sha256"])
            or prep.get("spdx_schema_sha256") != outer["spdx_schema_sha256"]):
        # The outer v1 binding has ten exact fields; the archive index is carried
        # by the independent five-field input map, not this binding.
        fail("consumer_outer_binding_shape_or_identity")
    if (set(expected_inputs) != {"archive_index_sha256", "source_commit", "original_run_id",
                                "acquisition_artifact_id", "dependencies"}
            or expected_inputs.get("source_commit") != producer_pins["source_commit"]
            or type(expected_inputs.get("original_run_id")) is not int
            or expected_inputs["original_run_id"] != producer_pins["run_id"]
            or type(expected_inputs.get("acquisition_artifact_id")) is not int
            or expected_inputs["acquisition_artifact_id"] != producer_pins["artifact_id"]
            or not isinstance(expected_inputs.get("archive_index_sha256"), str)
            or not SHA256.fullmatch(expected_inputs["archive_index_sha256"])):
        fail("consumer_expectation_source_mismatch")
    dependencies = expected_inputs.get("dependencies")
    if not isinstance(dependencies, list) or len(dependencies) != 12:
        fail("consumer_expected_inputs_dependencies")
    dependency_ids = set()
    for row in dependencies:
        if (not isinstance(row, dict) or set(row) != {"id", "sha256"}
                or not isinstance(row.get("id"), str) or not row["id"]
                or not isinstance(row.get("sha256"), str) or not SHA256.fullmatch(row["sha256"])
                or row["id"] in dependency_ids):
            fail("consumer_expected_input_dependency_invalid")
        dependency_ids.add(row["id"])
    prepared = prep.get("prepared_files")
    if not isinstance(prepared, dict) or set(prepared) != {"acquisition.json", "expected-inputs.json", "outer-binding.json"}:
        fail("consumer_preparation_file_map_invalid")
    for name, rel in (("acquisition.json", "expectations/acquisition.json"),
                      ("expected-inputs.json", "expectations/expected-inputs.json"),
                      ("outer-binding.json", "expectations/outer-binding.json")):
        file_bytes = read_regular(root / under(evidence_root, rel), MAX_JSON_BYTES, "consumer_prepared_file")
        if prepared[name] != hashlib.sha256(file_bytes).hexdigest():
            fail("consumer_preparation_file_hash_mismatch")
    if (set(acquisition) != {"archive_count", "archive_index_sha256", "artifact_digest", "artifact_id",
                            "derivation", "ecosystems", "repository", "run_id", "source_commit"}
            or acquisition.get("repository") != REPOSITORY
            or acquisition.get("source_commit") != producer_pins["source_commit"]
            or acquisition.get("run_id") != producer_pins["run_id"]
            or acquisition.get("artifact_id") != producer_pins["artifact_id"]
            or acquisition.get("archive_count") != 8
            or acquisition.get("archive_index_sha256") != expected_inputs["archive_index_sha256"]
            or acquisition.get("artifact_digest") != "sha256:" + producer_pins["archive_zip_sha256"]):
        fail("consumer_derived_acquisition_identity_mismatch")
    ecosystems = acquisition.get("ecosystems")
    if ecosystems != ["go", "julia", "nuget", "python", "r", "rust", "typescript"]:
        fail("consumer_derived_acquisition_ecosystems")
    if report["archive_index_sha256"] != expected_inputs["archive_index_sha256"]:
        fail("consumer_report_index_mismatch")
    if outer.get("original_run_id") != expected_inputs["original_run_id"]:
        fail("consumer_expectation_producer_identity_mismatch")
    syft = prep.get("syft_qualification")
    if not isinstance(syft, dict):
        fail("consumer_syft_qualification_missing")
    syft_receipt_bytes = read_regular(root / under(syft_root, "evidence/receipt.json"), MAX_JSON_BYTES,
                                      "consumer_syft_receipt")
    syft_receipt = strict_json(syft_receipt_bytes, "consumer_syft_receipt")
    syft_validation = strict_json(
        read_regular(root / under(syft_root, "evidence/validation-report.json"), MAX_JSON_BYTES,
                     "consumer_syft_validation"), "consumer_syft_validation")
    if (not isinstance(syft, dict) or syft.get("target") != "linux-amd64"
            or not isinstance(syft_receipt, dict)
            or syft_receipt.get("schema") != "kairos.verified-syft-installer.v1"
            or syft_receipt.get("result") != "pass" or syft_receipt.get("target") != "linux-amd64"
            or syft_receipt.get("platform") != "linux/amd64"
            or not isinstance(syft_validation, dict)
            or syft_validation.get("schema") != "kairos.syft-installation-validation.v1"
            or syft_validation.get("result") != "pass"
            or syft_validation.get("target") != "linux-amd64"
            or syft_validation.get("platform") != "linux/amd64"
            or not isinstance(syft.get("receipt_sha256"), str)
            or not SHA256.fullmatch(syft["receipt_sha256"])
            or hashlib.sha256(syft_receipt_bytes).hexdigest() != syft["receipt_sha256"]
            or not isinstance(syft.get("binary_sha256"), str)
            or not SHA256.fullmatch(syft["binary_sha256"])
            or syft_validation.get("receipt_sha256") != syft["receipt_sha256"]
            or syft_validation.get("binary_sha256") != syft["binary_sha256"]):
        fail("consumer_syft_receipt_binding_invalid")
    tool_rows = [row for row in dependencies if row["id"] == "tool:syft"]
    if len(tool_rows) != 1 or tool_rows[0]["sha256"] != syft["binary_sha256"]:
        fail("consumer_syft_dependency_mismatch")
    return report


def _write_exclusive(path: Path, data: bytes) -> None:
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | getattr(os, "O_NOFOLLOW", 0), 0o600)
    try:
        opened = os.fstat(descriptor)
        if not stat.S_ISREG(opened.st_mode):
            fail("output_file_not_regular")
        with os.fdopen(descriptor, "wb") as stream:
            descriptor = -1
            stream.write(data)
            stream.flush()
            os.fsync(stream.fileno())
    finally:
        if descriptor >= 0:
            os.close(descriptor)


def _move_into_exclusive_output(staging_fd: int, parent_fd: int, output_name: str) -> tuple[int, tuple[int, int]]:
    try:
        os.mkdir(output_name, mode=0o700, dir_fd=parent_fd)
    except FileExistsError as exc:
        raise AcquisitionError("output_already_exists") from exc
    try:
        entry = os.stat(output_name, dir_fd=parent_fd, follow_symlinks=False)
        if not stat.S_ISDIR(entry.st_mode):
            fail("output_reservation_invalid")
        identity = (entry.st_dev, entry.st_ino)
        output_fd = os.open(output_name, os.O_RDONLY | getattr(os, "O_DIRECTORY", 0) | getattr(os, "O_NOFOLLOW", 0), dir_fd=parent_fd)
        info = os.fstat(output_fd)
        if (info.st_dev, info.st_ino) != identity:
            os.close(output_fd)
            fail("output_reservation_replaced")
    except BaseException:
        try:
            if "identity" in locals():
                _remove_owned_output(parent_fd, output_name, identity)
        except OSError:
            pass
        raise
    try:
        for name in os.listdir(staging_fd):
            os.rename(name, name, src_dir_fd=staging_fd, dst_dir_fd=output_fd)
        return output_fd, identity
    except BaseException:
        try:
            _remove_owned_output(parent_fd, output_name, identity)
        except OSError:
            pass
        os.close(output_fd)
        raise


def _remove_tree_at(parent_fd: int, name: str) -> None:
    child_fd = os.open(name, os.O_RDONLY | getattr(os, "O_DIRECTORY", 0) | getattr(os, "O_NOFOLLOW", 0), dir_fd=parent_fd)
    try:
        for child in os.listdir(child_fd):
            info = os.stat(child, dir_fd=child_fd, follow_symlinks=False)
            if stat.S_ISDIR(info.st_mode):
                _remove_tree_at(child_fd, child)
            else:
                os.unlink(child, dir_fd=child_fd)
    finally:
        os.close(child_fd)
    os.rmdir(name, dir_fd=parent_fd)


def _remove_owned_output(parent_fd: int, name: str, identity: tuple[int, int]) -> bool:
    try:
        entry = os.stat(name, dir_fd=parent_fd, follow_symlinks=False)
        if not stat.S_ISDIR(entry.st_mode) or (entry.st_dev, entry.st_ino) != identity:
            return False
        output_fd = os.open(name, os.O_RDONLY | getattr(os, "O_DIRECTORY", 0) | getattr(os, "O_NOFOLLOW", 0), dir_fd=parent_fd)
    except FileNotFoundError:
        return False
    try:
        opened = os.fstat(output_fd)
        if (opened.st_dev, opened.st_ino) != identity:
            return False
        for child in os.listdir(output_fd):
            child_stat = os.stat(child, dir_fd=output_fd, follow_symlinks=False)
            if stat.S_ISDIR(child_stat.st_mode):
                _remove_tree_at(output_fd, child)
            else:
                os.unlink(child, dir_fd=output_fd)
    finally:
        os.close(output_fd)
    try:
        entry = os.stat(name, dir_fd=parent_fd, follow_symlinks=False)
    except FileNotFoundError:
        return False
    if not stat.S_ISDIR(entry.st_mode) or (entry.st_dev, entry.st_ino) != identity:
        return False
    os.rmdir(name, dir_fd=parent_fd)
    return True


def acquire(*, run_id: int, run_attempt: int, artifact_id: int, source_commit: str,
            archive_zip_sha256: str, archive_zip_bytes: int, expected_helper_sha256: str,
            output: Path, api_fetch: Callable[[types.ModuleType, str], tuple[bytes, dict[str, Any]]] | None = None,
            download: Callable[[types.ModuleType, str, Path], str] | None = None) -> dict[str, Any]:
    if type(run_id) is not int or not 1 <= run_id <= 2**63 - 1:
        fail("consumer_run_id_invalid")
    if type(run_attempt) is not int or not 1 <= run_attempt <= 2**31 - 1:
        fail("consumer_run_attempt_invalid")
    if type(artifact_id) is not int or not 1 <= artifact_id <= 2**63 - 1:
        fail("consumer_artifact_id_invalid")
    if not isinstance(source_commit, str) or not COMMIT.fullmatch(source_commit):
        fail("consumer_source_commit_invalid")
    if not isinstance(archive_zip_sha256, str) or not SHA256.fullmatch(archive_zip_sha256):
        fail("consumer_zip_sha256_invalid")
    if type(archive_zip_bytes) is not int or not 1 <= archive_zip_bytes <= MAX_PIN_BYTES:
        fail("consumer_zip_bytes_invalid")
    target = Path(os.path.abspath(output))
    if target == Path(target.anchor):
        fail("output_path_invalid")
    parent = target.parent
    try:
        parent_fd = _open_nofollow(parent, directory=True)
        parent_info = os.fstat(parent_fd)
        os.close(parent_fd)
    except (AcquisitionError, OSError) as exc:
        raise AcquisitionError("output_parent_invalid") from exc
    if not stat.S_ISDIR(parent_info.st_mode) or target.exists() or target.is_symlink():
        fail("output_already_exists" if target.exists() or target.is_symlink() else "output_parent_invalid")
    helper = load_captured_helper(expected_helper_sha256)
    endpoint_run = f"repos/{REPOSITORY}/actions/runs/{run_id}/attempts/{run_attempt}"
    endpoint_artifact = f"repos/{REPOSITORY}/actions/artifacts/{artifact_id}"
    fetch = api_fetch or api_readback
    run_bytes, run = fetch(helper, endpoint_run)
    repository_id, _ = validate_run(run, run_id, run_attempt, source_commit)
    artifact_bytes, artifact = fetch(helper, endpoint_artifact)
    validate_artifact(artifact, run_id=run_id, attempt=run_attempt, artifact_id=artifact_id,
                      source_commit=source_commit, repository_id=repository_id,
                      archive_zip_sha256=archive_zip_sha256, archive_zip_bytes=archive_zip_bytes)

    try:
        parent_fd = _open_nofollow(parent, directory=True)
        current_parent = os.fstat(parent_fd)
    except (AcquisitionError, OSError) as exc:
        raise AcquisitionError("output_parent_invalid") from exc
    if (current_parent.st_dev, current_parent.st_ino) != (parent_info.st_dev, parent_info.st_ino):
        os.close(parent_fd)
        fail("output_parent_changed")
    if target.exists() or target.is_symlink():
        os.close(parent_fd)
        fail("output_already_exists")

    cwd_fd = os.open(".", os.O_RDONLY | getattr(os, "O_DIRECTORY", 0))
    stage_name = ".archive-consumer-acquisition-" + secrets.token_hex(16)
    try:
        os.mkdir(stage_name, mode=0o700, dir_fd=parent_fd)
        stage_fd = os.open(stage_name, os.O_RDONLY | getattr(os, "O_DIRECTORY", 0) | getattr(os, "O_NOFOLLOW", 0), dir_fd=parent_fd)
    except BaseException:
        os.close(cwd_fd)
        os.close(parent_fd)
        raise
    output_fd = None
    try:
        # Keep all staging, extraction, receipt writes and final moves relative to
        # descriptors for the originally opened parent and staging directories.
        os.fchdir(stage_fd)
        archive_path = Path(f"{artifact_id}.zip")
        payload_path = Path("payload")
        if download is None:
            def download(helper_module: types.ModuleType, artifact_endpoint: str, archive: Path) -> str:
                try:
                    digest, _ = helper_module._run_bounded_process(
                        [*helper_module.API_COMMAND_PREFIX, artifact_endpoint],
                        MAX_PIN_BYTES, min(helper_module.DOWNLOAD_TIMEOUT_SECONDS, DOWNLOAD_TIMEOUT_SECONDS), archive)
                except Exception as exc:
                    raise AcquisitionError("consumer_artifact_download_failed") from exc
                return digest
        downloaded_digest = download(helper, f"repos/{REPOSITORY}/actions/artifacts/{artifact_id}/zip", archive_path)
        if downloaded_digest != "sha256:" + archive_zip_sha256:
            fail("consumer_zip_download_digest_mismatch")
        if archive_path.lstat().st_size != archive_zip_bytes:
            fail("consumer_zip_download_size_mismatch")
        members = preflight_zip(archive_path, archive_zip_sha256, archive_zip_bytes)
        validate_explicit_directories(members, run_id, run_attempt)
        try:
            helper.extract_verified(archive_path, payload_path, "sha256:" + archive_zip_sha256)
        except AcquisitionError:
            raise
        except Exception as exc:
            raise AcquisitionError("consumer_zip_extraction_failed") from exc
        after_digest = hashlib.sha256()
        with archive_path.open("rb") as stream:
            while block := stream.read(1024 * 1024):
                after_digest.update(block)
        if after_digest.hexdigest() != archive_zip_sha256:
            fail("consumer_zip_changed_during_extraction")
        report = validate_consumer_layout(payload_path, run_id, run_attempt, source_commit)
        _write_exclusive(Path("run-metadata.json"), run_bytes)
        _write_exclusive(Path("artifact-metadata.json"), artifact_bytes)
        files = []
        for current, _, names in os.walk(payload_path, followlinks=False):
            for name in names:
                path = Path(current) / name
                data = read_regular(path, MAX_MEMBER_BYTES, "consumer_layout_file")
                files.append({"path": path.relative_to(payload_path).as_posix(),
                              "bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()})
        files.sort(key=lambda row: row["path"])
        receipt = {
            "schema": "kairos.archive-consumer-evidence-acquisition.v1",
            "repository": REPOSITORY,
            "workflow_path": WORKFLOW,
            "run_id": run_id,
            "run_attempt": run_attempt,
            "artifact_id": artifact_id,
            "artifact_name": artifact["name"],
            "source_commit": source_commit,
            "archive_zip_sha256": archive_zip_sha256,
            "archive_zip_bytes": archive_zip_bytes,
            "acquisition_helper_sha256": expected_helper_sha256,
            "run_readback_sha256": hashlib.sha256(run_bytes).hexdigest(),
            "artifact_readback_sha256": hashlib.sha256(artifact_bytes).hexdigest(),
            "consumer_validation": report,
            "extracted_files": files,
            "scope": "exact successful mainline consumer artifact acquisition; Rust kairo-ecs-types plus six binding ecosystems only; not full Rust workspace or registry readiness; no release acceptance or publication",
        }
        _write_exclusive(Path("receipt.json"), canonical_json(receipt))
        os.rename("payload", "evidence")
        output_fd, output_identity = _move_into_exclusive_output(stage_fd, parent_fd, target.name)
        # Refuse success if the directory reached through the caller's original
        # parent pathname no longer names the directory used for these writes.
        check_fd = _open_nofollow(parent, directory=True)
        try:
            check_info = os.fstat(check_fd)
        finally:
            os.close(check_fd)
        if (check_info.st_dev, check_info.st_ino) != (parent_info.st_dev, parent_info.st_ino):
            _remove_owned_output(parent_fd, target.name, output_identity)
            fail("output_parent_changed_during_install")
        try:
            entry = os.stat(target.name, dir_fd=parent_fd, follow_symlinks=False)
        except FileNotFoundError:
            entry = None
        if (entry is None or not stat.S_ISDIR(entry.st_mode)
                or (entry.st_dev, entry.st_ino) != output_identity):
            _remove_owned_output(parent_fd, target.name, output_identity)
            fail("output_reservation_replaced")
        return receipt
    except BaseException:
        try:
            if output_fd is not None:
                _remove_owned_output(parent_fd, target.name, output_identity)
            os.fchdir(parent_fd)
            if stage_name in os.listdir(parent_fd):
                _remove_tree_at(parent_fd, stage_name)
        except OSError:
            pass
        raise
    finally:
        try:
            os.fchdir(parent_fd)
            try:
                _remove_tree_at(parent_fd, stage_name)
            except FileNotFoundError:
                pass
        finally:
            if output_fd is not None:
                os.close(output_fd)
            os.close(stage_fd)
            os.fchdir(cwd_fd)
            os.close(cwd_fd)
            os.close(parent_fd)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run-id", required=True)
    parser.add_argument("--run-attempt", required=True)
    parser.add_argument("--artifact-id", required=True)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--archive-zip-sha256", required=True)
    parser.add_argument("--archive-zip-bytes", required=True)
    parser.add_argument("--expected-acquisition-helper-sha256", required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    try:
        receipt = acquire(run_id=positive_integer(args.run_id, "consumer_run_id"),
                          run_attempt=positive_integer(args.run_attempt, "consumer_run_attempt", 2**31 - 1),
                          artifact_id=positive_integer(args.artifact_id, "consumer_artifact_id"),
                          source_commit=args.source_commit,
                          archive_zip_sha256=args.archive_zip_sha256,
                          archive_zip_bytes=positive_integer(args.archive_zip_bytes, "consumer_zip_bytes", MAX_PIN_BYTES),
                          expected_helper_sha256=args.expected_acquisition_helper_sha256,
                          output=args.output)
    except (AcquisitionError, OSError) as exc:
        parser.error(str(exc))
    print(json.dumps({"status": "retained", "run_id": receipt["run_id"],
                      "run_attempt": receipt["run_attempt"], "artifact_id": receipt["artifact_id"],
                      "source_commit": receipt["source_commit"], "output": str(args.output)}))


if __name__ == "__main__":
    main()
