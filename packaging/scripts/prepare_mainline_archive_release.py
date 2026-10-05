#!/usr/bin/env python3
"""Prepare a source-bound archive release from exact retained mainline runs.

This native Linux orchestration step reacquires explicit producer and consumer
evidence, qualifies Syft locally, derives independent verifier inputs, verifies
the complete archive-copy evidence profile, and delegates exact subject copying
to the qualified adapter. It does not build packages, rescan archives, publish,
or enable a release.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import re
import stat
import subprocess
import sys
import time
import types
from typing import Any


REPOSITORY = "edithatogo/kairos"
PRODUCER_WORKFLOW = ".github/workflows/package-dry-run.yml"
CONSUMER_WORKFLOW = ".github/workflows/archive-supply-chain-main.yml"
SCHEMA = "tests/fixtures/archive-supply-chain/spdx-2.3/spdx-schema.json"
PRODUCER_PIN_KEYS = {"run_id", "artifact_id", "source_commit", "producer_tree", "archive_zip_sha256", "archive_zip_bytes"}
CONSUMER_PIN_KEYS = {"run_id", "run_attempt", "artifact_id", "source_commit", "archive_zip_sha256", "archive_zip_bytes"}
RELEASE_OUTPUT_NAME = "actual-package-archives"
MAX_PIN_BYTES = 128 * 1024
MAX_JSON_BYTES = 8 * 1024 * 1024
MAX_SOURCE_BYTES = 8 * 1024 * 1024
MAX_PROCESS_OUTPUT = 2 * 1024 * 1024
MAX_PROCESS_SECONDS = 3600
MAX_PIPELINE_SECONDS = 35 * 60
MAX_ARCHIVE_BYTES = 2 * 1024 * 1024 * 1024
COMMIT = re.compile(r"^[0-9a-f]{40}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")
DECIMAL = re.compile(r"^[1-9][0-9]{0,18}$")
SYFT_STABLE_FIELDS = (
    "schema", "result", "version", "release_tag", "release_commit", "target", "platform",
    "release_checksum_sha256", "release_bundle_sha256", "authenticated_asset", "signed_asset_sha256",
    "binary_path", "archive_sha256", "binary_sha256", "archive", "version_probe", "issuer_policy",
    "issuer_enforcement", "certificate_identity", "repository", "ref", "installer_source_sha256",
    "verifier", "qualification_limit",
)
SYFT_COMMAND_LABELS = (
    "download-checksums", "download-signature-bundle", "create-verifier-venv",
    "audit-pip-configuration", "install-hash-locked-verifier", "verify-signed-checksum-document",
    "download-syft-archive", "extract-syft-archive", "syft-version",
)
SYFT_CHECKSUM_URL = "https://github.com/anchore/syft/releases/download/v1.54.0/syft_1.54.0_checksums.txt"
SYFT_BUNDLE_URL = SYFT_CHECKSUM_URL + ".sigstore.json"
SYFT_ARCHIVE_URL = "https://github.com/anchore/syft/releases/download/v1.54.0/syft_1.54.0_linux_amd64.tar.gz"
SYFT_RELEASE_COMMIT = "cc326e45a6213360266dda4b30cc68095946d676"
SYFT_CERT_IDENTITY = "https://github.com/anchore/syft/.github/workflows/release.yaml@refs/heads/main"
CLAIM_SCOPE = "local consistency only; unsigned and untrusted builder; no SLSA level or release acceptance"
READBACK_VALIDATOR = "packaging/scripts/validate_actual_archive_release.py"
READBACK_CLAIM_SCOPE = ("actual archive output consistency only; no build, SBOM, provenance, signature, "
                        "registry, publication, or release acceptance claim")


class GateError(Exception):
    """A bounded, stable orchestration rejection."""


def fail(code: str) -> None:
    raise GateError(code)


def check_deadline(deadline: float | None) -> None:
    if deadline is not None and time.monotonic() >= deadline:
        fail("pipeline_deadline_exceeded")


def sanitized_environment(source: dict[str, str] | None = None) -> dict[str, str]:
    result = dict(os.environ if source is None else source)
    for name in tuple(result):
        if (name in {"GH_TOKEN", "GITHUB_TOKEN", "GH_ENTERPRISE_TOKEN", "ACTIONS_RUNTIME_TOKEN",
                     "ACTIONS_ID_TOKEN_REQUEST_TOKEN", "ACTIONS_ID_TOKEN_REQUEST_URL"}
                or name.startswith("GITHUB_APP_")):
            result.pop(name, None)
    return result


def absolute(path: Path) -> Path:
    if ".." in Path(path).parts:
        fail("path_parent_component")
    return Path(os.path.abspath(os.fspath(path)))


def require_path_chain(path: Path, *, leaf: str) -> Path:
    target = absolute(path)
    current = Path(target.anchor)
    parts = target.parts[1:]
    for index, component in enumerate(parts):
        current = current / component
        try:
            mode = os.lstat(current).st_mode
        except FileNotFoundError:
            if leaf == "missing-directory" and index == len(parts) - 1:
                continue
            fail("path_component_missing")
        if stat.S_ISLNK(mode):
            fail("path_symlink")
        if index < len(parts) - 1 or leaf == "directory":
            if not stat.S_ISDIR(mode):
                fail("path_component_not_directory")
        elif leaf == "file" and not stat.S_ISREG(mode):
            fail("path_leaf_not_regular")
    return target


def secure_read(path: Path, limit: int, label: str) -> bytes:
    target = absolute(path)
    if target == Path(target.anchor):
        fail(label + "_path")
    dir_flags = os.O_RDONLY | getattr(os, "O_DIRECTORY", 0) | getattr(os, "O_NOFOLLOW", 0)
    file_flags = os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0) | getattr(os, "O_NONBLOCK", 0)
    fd = os.open(target.anchor, dir_flags)
    try:
        for component in target.parts[1:-1]:
            next_fd = os.open(component, dir_flags, dir_fd=fd)
            os.close(fd)
            fd = next_fd
        leaf = os.open(target.parts[-1], file_flags, dir_fd=fd)
        try:
            info = os.fstat(leaf)
            if not stat.S_ISREG(info.st_mode) or info.st_nlink != 1 or info.st_size > limit:
                fail(label + "_not_bounded_regular")
            data = bytearray()
            while len(data) <= limit:
                block = os.read(leaf, min(65536, limit + 1 - len(data)))
                if not block:
                    break
                data.extend(block)
            if len(data) > limit:
                fail(label + "_too_large")
            return bytes(data)
        finally:
            os.close(leaf)
    except OSError as exc:
        raise GateError(label + "_read_failed") from exc
    finally:
        os.close(fd)


def secure_digest(path: Path, limit: int, label: str) -> tuple[str, int]:
    target = absolute(path)
    if target == Path(target.anchor):
        fail(label + "_path")
    dir_flags = os.O_RDONLY | getattr(os, "O_DIRECTORY", 0) | getattr(os, "O_NOFOLLOW", 0)
    file_flags = os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0) | getattr(os, "O_NONBLOCK", 0)
    fd = os.open(target.anchor, dir_flags)
    try:
        for component in target.parts[1:-1]:
            next_fd = os.open(component, dir_flags, dir_fd=fd)
            os.close(fd)
            fd = next_fd
        leaf = os.open(target.parts[-1], file_flags, dir_fd=fd)
        try:
            info = os.fstat(leaf)
            if not stat.S_ISREG(info.st_mode) or info.st_nlink != 1:
                fail(label + "_not_regular")
            digest = hashlib.sha256()
            size = 0
            while True:
                block = os.read(leaf, 1024 * 1024)
                if not block:
                    break
                size += len(block)
                if size > limit:
                    fail(label + "_too_large")
                digest.update(block)
            return digest.hexdigest(), size
        finally:
            os.close(leaf)
    except OSError as exc:
        raise GateError(label + "_read_failed") from exc
    finally:
        os.close(fd)


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

    def finite_float(value: str) -> float:
        parsed = float(value)
        if not math.isfinite(parsed):
            fail(label + "_non_finite")
        return parsed

    try:
        value = json.loads(data.decode("utf-8"), object_pairs_hook=pairs,
                           parse_constant=reject_constant, parse_float=finite_float)
    except GateError:
        raise
    except (UnicodeDecodeError, json.JSONDecodeError, ValueError, RecursionError) as exc:
        raise GateError(label + "_invalid_json") from exc
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


def positive_pin(value: Any, label: str, maximum: int = 2**63 - 1) -> int:
    if not isinstance(value, str) or not DECIMAL.fullmatch(value):
        fail(label + "_invalid")
    parsed = int(value)
    if parsed > maximum:
        fail(label + "_out_of_range")
    return parsed


def validate_pin_file(path: Path, expected: set[str], label: str) -> dict[str, Any]:
    return validate_pin_bytes(secure_read(path, MAX_PIN_BYTES, label), expected, label)


def validate_pin_bytes(data: bytes, expected: set[str], label: str) -> dict[str, Any]:
    value = strict_json(data, label, MAX_PIN_BYTES)
    if not isinstance(value, dict) or set(value) != expected:
        fail(label + "_shape")
    if any(not isinstance(item, str) or not item for item in value.values()):
        fail(label + "_values_must_be_strings")
    return value


def read_git_blob(repository: Path, revision: str, path: str, limit: int = MAX_SOURCE_BYTES) -> bytes:
    env = sanitized_environment()
    size = subprocess.run(["git", "-C", str(repository), "cat-file", "-s", revision + ":" + path],
                          stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, timeout=15, check=False, env=env)
    if size.returncode != 0 or len(size.stdout) > 32:
        fail("trusted_git_blob_missing")
    try:
        byte_count = int(size.stdout.strip())
    except ValueError as exc:
        raise GateError("trusted_git_blob_size_invalid") from exc
    if byte_count < 0 or byte_count > limit:
        fail("trusted_git_blob_too_large")
    blob = subprocess.run(["git", "-C", str(repository), "cat-file", "blob", revision + ":" + path],
                          stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, timeout=15, check=False, env=env)
    if blob.returncode != 0 or len(blob.stdout) != byte_count:
        fail("trusted_git_blob_read_failed")
    return blob.stdout


def trusted_source(repository: Path, revision: str, relative: str) -> bytes:
    blob = read_git_blob(repository, revision, relative)
    current = secure_read(repository / relative, MAX_SOURCE_BYTES, "checkout_source")
    if current != blob:
        fail("checkout_source_differs_from_trusted_git")
    return blob


def git_text(repository: Path, *args: str) -> str:
    result = subprocess.run(["git", "-C", str(repository), *args], stdout=subprocess.PIPE,
                            stderr=subprocess.DEVNULL, timeout=15, check=False, env=sanitized_environment())
    if result.returncode != 0 or len(result.stdout) > 256:
        fail("git_identity_readback_failed")
    try:
        return result.stdout.decode("ascii").strip()
    except UnicodeDecodeError as exc:
        raise GateError("git_identity_readback_invalid") from exc


def load_bounded_runner(source: bytes) -> Any:
    module = types.ModuleType("_trusted_mainline_archive_process_runner")
    module.__file__ = "packaging/scripts/acquire_package_archive_bundle.py"
    try:
        exec(compile(source, module.__file__, "exec", dont_inherit=True), module.__dict__)
    except Exception as exc:
        raise GateError("trusted_process_runner_load_failed") from exc
    runner = getattr(module, "_run_bounded_process", None)
    if not callable(runner):
        fail("trusted_process_runner_missing")
    return runner


def write_exclusive(path: Path, data: bytes) -> None:
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | getattr(os, "O_NOFOLLOW", 0), 0o600)
    with os.fdopen(fd, "wb") as stream:
        stream.write(data)
        stream.flush()
        os.fsync(stream.fileno())


def append_command_record(work_dir: Path, record: dict[str, Any]) -> None:
    path = work_dir / "command-records.json"
    if path.exists() or path.is_symlink():
        current = strict_json(secure_read(path, MAX_JSON_BYTES, "command_records"), "command_records")
    else:
        current = []
    if not isinstance(current, list):
        fail("command_records_shape")
    current.append(record)
    data = (json.dumps(current, sort_keys=True, indent=2, allow_nan=False) + "\n").encode()
    temp = work_dir / ".command-records.tmp"
    if temp.exists() or temp.is_symlink():
        fail("command_records_temp_exists")
    write_exclusive(temp, data)
    os.replace(temp, path)


def run_command(work_dir: Path, runner: Any, label: str, argv: list[str], *, timeout: int = MAX_PROCESS_SECONDS,
                deadline: float | None = None) -> bytes:
    start = time.monotonic()
    if deadline is not None:
        remaining = deadline - start
        if remaining < 1:
            fail("pipeline_deadline_exceeded")
        timeout = min(timeout, int(remaining))
    code = 0
    output = b""
    try:
        saved_env = dict(os.environ)
        sanitized = sanitized_environment(saved_env)
        if label in {"acquire-producer", "acquire-consumer"} and saved_env.get("GH_TOKEN"):
            sanitized["GH_TOKEN"] = saved_env["GH_TOKEN"]
        try:
            if sanitized is not None:
                os.environ.clear()
                os.environ.update(sanitized)
            _, captured = runner(argv, MAX_PROCESS_OUTPUT, timeout)
        finally:
            os.environ.clear()
            os.environ.update(saved_env)
        output = captured or b""
    except subprocess.CalledProcessError as exc:
        code = int(exc.returncode)
    except (TimeoutError, ValueError) as exc:
        code = 124 if isinstance(exc, TimeoutError) else 125
        record = {"step": label, "argv": argv, "exit_status": code,
                  "stdout_bytes": 0, "stdout_sha256": hashlib.sha256(b"").hexdigest(),
                  "duration_seconds": round(time.monotonic() - start, 3),
                  "runner_error": "timeout" if isinstance(exc, TimeoutError) else "bounded_runner_rejected"}
        append_command_record(work_dir, record)
        fail("command_" + label + "_failed")
    except Exception as exc:
        append_command_record(work_dir, {"step": label, "argv": argv, "exit_status": 125,
                                 "stdout_bytes": 0, "stdout_sha256": hashlib.sha256(b"").hexdigest(),
                                 "runner_error": type(exc).__name__})
        fail("command_" + label + "_failed")
    if len(output) > MAX_PROCESS_OUTPUT:
        fail("command_output_limit")
    log = work_dir / "logs" / (label + ".stdout")
    write_exclusive(log, output)
    append_command_record(work_dir, {"step": label, "argv": argv, "exit_status": code,
                            "stdout_bytes": len(output), "stdout_sha256": hashlib.sha256(output).hexdigest(),
                            "stdout_path": str(log.relative_to(work_dir)),
                            "duration_seconds": round(time.monotonic() - start, 3),
                            "stderr_policy": "discarded by trusted process runner"})
    if code != 0:
        fail("command_" + label + "_failed")
    check_deadline(deadline)
    return output


def run_trusted_script(repository: Path, revision: str, source_map: dict[str, bytes], work_dir: Path,
                       runner: Any, label: str, script: str, argv: list[str], *, timeout: int = MAX_PROCESS_SECONDS,
                       deadline: float | None = None) -> bytes:
    if script not in source_map:
        fail("trusted_script_missing")
    validate_source_map(repository, revision, source_map, work_dir, "before_execution")
    output = run_command(work_dir, runner, label, argv, timeout=timeout, deadline=deadline)
    validate_source_map(repository, revision, source_map, work_dir, "during_execution")
    return output


def validate_source_map(repository: Path, revision: str, source_map: dict[str, bytes],
                        work_dir: Path, phase: str) -> None:
    for relative, expected in source_map.items():
        captured = work_dir / "captured-tools" / relative
        if secure_read(captured, MAX_SOURCE_BYTES, "captured_script") != expected:
            fail("captured_script_changed_" + phase)
        if trusted_source(repository, revision, relative) != expected:
            fail("trusted_source_changed_" + phase)


def run_captured_validator(repository: Path, revision: str, source_map: dict[str, bytes],
                           work_dir: Path, runner: Any, captured: Path, expected_bytes: bytes,
                           argv: list[str], *, deadline: float) -> bytes:
    validate_source_map(repository, revision, source_map, work_dir, "before_execution")
    if secure_read(captured, MAX_SOURCE_BYTES, "captured_validator") != expected_bytes:
        fail("captured_validator_changed_before_execution")
    output = run_command(work_dir, runner, "validate_actual_archive_output", argv,
                         timeout=300, deadline=deadline)
    if secure_read(captured, MAX_SOURCE_BYTES, "captured_validator") != expected_bytes:
        fail("captured_validator_changed_during_execution")
    validate_source_map(repository, revision, source_map, work_dir, "during_execution")
    return output


def capture_tool_tree(work_dir: Path, source_map: dict[str, bytes]) -> dict[str, Path]:
    """Stage exact Git-verified helper bytes at their repository-relative paths."""
    root = work_dir / "captured-tools"
    root.mkdir(mode=0o700)
    captured: dict[str, Path] = {}
    for relative, data in sorted(source_map.items()):
        target = root / relative
        target.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
        write_exclusive(target, data)
        captured[relative] = target
    return captured


def validate_native_producer_records(root: Path, *, run_id: int, artifact_id: int, source: str,
                                     tree: str, zip_sha: str) -> None:
    run, _ = load_json_file(root / "run-metadata.json", "producer_run_metadata")
    origin, _ = load_json_file(root / "artifact-metadata.json", "producer_artifact_metadata")
    source_map, _ = load_json_file(root / "source-commit-readback.json", "producer_source_readback")
    receipt, _ = load_json_file(root / "receipt.json", "producer_receipt")
    if (run.get("id") != run_id or run.get("path") != PRODUCER_WORKFLOW or run.get("head_sha") != source
            or run.get("status") != "completed" or run.get("conclusion") != "success"
            or run.get("event") != "workflow_dispatch" or run.get("head_branch") != "main"
            or run.get("pull_requests") != []):
        fail("producer_native_run_readback_mismatch")
    repo, head_repo = run.get("repository"), run.get("head_repository")
    if (not isinstance(repo, dict) or not isinstance(head_repo, dict) or repo.get("full_name") != REPOSITORY
            or head_repo.get("full_name") != REPOSITORY or type(repo.get("id")) is not int
            or repo.get("id") <= 0 or head_repo.get("id") != repo.get("id")):
        fail("producer_native_repository_readback_mismatch")
    expected_name = "kairos-actual-package-archives-" + source
    workflow_origin = origin.get("workflow_run")
    if (origin.get("id") != artifact_id or origin.get("name") != expected_name or origin.get("expired") is not False
            or origin.get("digest") != "sha256:" + zip_sha or not isinstance(workflow_origin, dict)
            or workflow_origin.get("id") != run_id or workflow_origin.get("head_sha") != source
            or workflow_origin.get("head_branch") != "main" or workflow_origin.get("repository_id") != repo["id"]
            or workflow_origin.get("head_repository_id") != repo["id"]):
        fail("producer_native_artifact_readback_mismatch")
    identity = source_map.get(source)
    if (not isinstance(identity, dict) or identity.get("sha") != source
            or not isinstance(identity.get("tree"), dict) or identity["tree"].get("sha") != tree
            or not isinstance(identity.get("verification"), dict)
            or identity["verification"].get("verified") is not True
            or identity["verification"].get("reason") != "valid"):
        fail("producer_native_source_readback_mismatch")
    if (receipt.get("workflow_run") != run_id or receipt.get("artifact_id") != artifact_id
            or receipt.get("producer_checkout_source") != source or receipt.get("producer_pr_head") != source
            or receipt.get("producer_tree") != tree or receipt.get("archive_zip_sha256") != zip_sha
            or receipt.get("selection_policy") != "same-repository-main-workflow-dispatch"):
        fail("producer_acquisition_receipt_mismatch")


def validate_native_consumer_records(root: Path, *, run_id: int, attempt: int, artifact_id: int,
                                     source: str, zip_sha: str) -> None:
    run, _ = load_json_file(root / "run-metadata.json", "consumer_run_metadata")
    artifact, _ = load_json_file(root / "artifact-metadata.json", "consumer_artifact_metadata")
    receipt, _ = load_json_file(root / "receipt.json", "consumer_receipt")
    origin = artifact.get("workflow_run")
    expected_name = f"archive-main-evidence-{run_id}-{attempt}"
    if (run.get("id") != run_id or run.get("run_attempt") != attempt or run.get("path") != CONSUMER_WORKFLOW
            or run.get("head_sha") != source or run.get("status") != "completed" or run.get("conclusion") != "success"
            or run.get("event") != "workflow_dispatch" or run.get("head_branch") != "main"
            or run.get("pull_requests") != []):
        fail("consumer_native_run_readback_mismatch")
    repo, head_repo = run.get("repository"), run.get("head_repository")
    if (not isinstance(repo, dict) or not isinstance(head_repo, dict) or repo.get("full_name") != REPOSITORY
            or head_repo.get("full_name") != REPOSITORY or type(repo.get("id")) is not int
            or repo.get("id") <= 0 or head_repo.get("id") != repo.get("id")):
        fail("consumer_native_repository_readback_mismatch")
    if (artifact.get("id") != artifact_id or artifact.get("name") != expected_name or artifact.get("expired") is not False
            or artifact.get("digest") != "sha256:" + zip_sha or not isinstance(origin, dict)
            or origin.get("id") != run_id or origin.get("head_sha") != source
            or origin.get("repository_id") != repo["id"] or origin.get("head_repository_id") != repo["id"]
            or origin.get("head_branch") != "main"):
        fail("consumer_native_artifact_readback_mismatch")
    if (receipt.get("run_id") != run_id or receipt.get("run_attempt") != attempt
            or receipt.get("artifact_id") != artifact_id or receipt.get("source_commit") != source
            or receipt.get("archive_zip_sha256") != zip_sha):
        fail("consumer_acquisition_receipt_mismatch")


def validate_embedded_preparation(root: Path, prep: dict[str, Any], work_dir: Path,
                                  *, source: str, producer_values: dict[str, Any]) -> None:
    if prep.get("trusted_consumer_sha") != source:
        fail("consumer_preparation_trusted_source_mismatch")
    pins = prep.get("producer_pins")
    expected_pins = {"repository": REPOSITORY, "run_id": producer_values["run_id"],
                     "artifact_id": producer_values["artifact_id"], "source_commit": source,
                     "producer_tree": producer_values["producer_tree"],
                     "archive_zip_sha256": producer_values["archive_zip_sha256"],
                     "archive_zip_bytes": producer_values["archive_zip_bytes"]}
    if pins != expected_pins:
        fail("consumer_preparation_producer_binding_mismatch")
    prepared = prep.get("prepared_files")
    expected_names = {"acquisition.json", "expected-inputs.json", "outer-binding.json"}
    if not isinstance(prepared, dict) or set(prepared) != expected_names:
        fail("consumer_preparation_file_map_shape")
    expectations = root / "expectations"
    for name in sorted(expected_names):
        digest, _ = secure_digest(expectations / name, MAX_JSON_BYTES, "consumer_embedded_expectation")
        if digest != prepared[name] or not isinstance(prepared[name], str) or not SHA256.fullmatch(prepared[name]):
            fail("consumer_preparation_file_hash_mismatch")
    embedded_inputs, _ = load_json_file(expectations / "expected-inputs.json", "consumer_embedded_expected_inputs")
    embedded_binding, _ = load_json_file(expectations / "outer-binding.json", "consumer_embedded_outer_binding")
    if (embedded_inputs.get("source_commit") != source or embedded_binding.get("source_commit") != source
            or embedded_inputs.get("original_run_id") != producer_values["run_id"]
            or embedded_inputs.get("acquisition_artifact_id") != producer_values["artifact_id"]
            or embedded_binding.get("producer_tree") != producer_values["producer_tree"]
            or embedded_binding.get("archive_zip_sha256") != producer_values["archive_zip_sha256"]
            or embedded_binding.get("archive_zip_bytes") != producer_values["archive_zip_bytes"]):
        fail("consumer_embedded_expectations_mismatch")


def trusted_source_map(repository: Path, revision: str) -> dict[str, bytes]:
    paths = (
        "packaging/scripts/acquire_package_archive_bundle.py",
        "packaging/scripts/acquire_archive_consumer_evidence.py",
        "packaging/scripts/build_archive_evidence_expectations.py",
        "packaging/scripts/build_archive_supply_chain.py",
        "packaging/scripts/build_archive_release_manifest.py",
        "packaging/scripts/build_package_archive_bundle.py",
        "packaging/scripts/validate_archive_copy_provenance.py",
        "packaging/scripts/verify_archive_supply_chain_evidence.py",
        "packaging/scripts/prepare_verified_archive_release.py",
        "scripts/supply_chain/install_verified_syft.py",
        "scripts/supply_chain/verify_syft_installation_receipt.py",
        "scripts/supply_chain/syft-linux-verifier.lock",
        READBACK_VALIDATOR,
        SCHEMA,
    )
    return {path: trusted_source(repository, revision, path) for path in paths}


def load_json_file(path: Path, label: str) -> tuple[Any, bytes]:
    data = secure_read(path, MAX_JSON_BYTES, label)
    value = strict_json(data, label)
    if not isinstance(value, dict):
        fail(label + "_not_object")
    return value, data


def validate_report(report: Any, expected_index_sha: str) -> None:
    keys = {"valid", "profile", "archive_count", "ecosystem_count", "spdx_document_count",
            "evidence_file_count", "archive_index_sha256", "statement_sha256", "claim_scope"}
    if not isinstance(report, dict) or set(report) != keys:
        fail("archive_verifier_report_shape")
    if (report["valid"] is not True or report["profile"] != "kairos-archive-copy-evidence-v1"
            or report["archive_count"] != 8 or report["ecosystem_count"] != 7
            or report["spdx_document_count"] != 9 or report["evidence_file_count"] != 44
            or report["archive_index_sha256"] != expected_index_sha or report["claim_scope"] != CLAIM_SCOPE
            or not isinstance(report["statement_sha256"], str) or not SHA256.fullmatch(report["statement_sha256"])):
        fail("archive_verifier_report_profile")


def validate_readback_report(report: Any, expected_index_sha: str, source_commit: str) -> None:
    expected_keys = {"schema", "result", "release_stage", "source_commit", "archive_index_sha256",
                     "archive_count", "ecosystem_count", "copied_archive_bytes", "claim_scope"}
    if (not isinstance(report, dict) or set(report) != expected_keys
            or report.get("schema") != "kairos-actual-archive-release-readback-v1"
            or report.get("result") != "pass" or report.get("release_stage") != "actual-package-archives"
            or report.get("source_commit") != source_commit
            or report.get("archive_index_sha256") != expected_index_sha
            or report.get("archive_count") != 8 or report.get("ecosystem_count") != 7
            or type(report.get("copied_archive_bytes")) is not int or report["copied_archive_bytes"] <= 0
            or report.get("claim_scope") != READBACK_CLAIM_SCOPE):
        fail("actual_archive_readback_profile")


def validate_original_syft_commands(original_root: Path, receipt: dict[str, Any], *,
                                    run_id: int, attempt: int) -> list[dict[str, Any]]:
    commands = receipt.get("commands")
    if not isinstance(commands, list) or len(commands) != len(SYFT_COMMAND_LABELS):
        fail("original_syft_command_receipts_missing")
    first = commands[0]
    first_argv = first.get("argv") if isinstance(first, dict) else None
    if not isinstance(first_argv, list) or len(first_argv) != 6 or not all(isinstance(x, str) for x in first_argv):
        fail("original_syft_first_command_invalid")
    checksum_path = first_argv[4]
    suffix = "/downloads/syft-checksums.txt"
    if not checksum_path.endswith(suffix) or "\x00" in checksum_path:
        fail("original_syft_output_path_invalid")
    output = checksum_path[:-len(suffix)]
    if (not output.startswith("/") or ".." in Path(output).parts
            or Path(output).name != f"syft-linux-amd64-{run_id}-{attempt}"):
        fail("original_syft_output_path_invalid")
    python = first_argv[0]
    installer = first_argv[1]
    if (not Path(installer).is_absolute() or Path(installer).name != "install_verified_syft.py"
            or Path(python).name != "python"):
        fail("original_syft_installer_argv_invalid")
    checksum_file = output + "/downloads/syft-checksums.txt"
    bundle_file = output + "/downloads/syft-checksums.sigstore.json"
    archive_file = output + "/downloads/syft_1.54.0_linux_amd64.tar.gz"
    expected_argv = [
        [python, installer, "--fetch-internal", SYFT_CHECKSUM_URL, checksum_file, "65536"],
        [python, installer, "--fetch-internal", SYFT_BUNDLE_URL, bundle_file, "2097152"],
        [python, "-m", "venv", output + "/verifier"],
        [output + "/verifier/bin/python", output + "/pip-config-audit.py", output + "/pip-canary.ini"],
        [output + "/verifier/bin/python", "-m", "pip", "--isolated", "--disable-pip-version-check",
         "--no-input", "install", "--require-hashes", "-r", output + "/verifier.lock"],
        [output + "/verifier/bin/python", "-m", "sigstore", "verify", "github", "--bundle", bundle_file,
         "--cert-identity", SYFT_CERT_IDENTITY, "--sha", SYFT_RELEASE_COMMIT,
         "--repository", "anchore/syft", "--ref", "refs/heads/main", checksum_file],
        [python, installer, "--fetch-internal", SYFT_ARCHIVE_URL, archive_file, str(64 * 1024 * 1024)],
        [python, installer, "--extract-internal", archive_file, output + "/extract",
         str(receipt.get("archive", {}).get("archive_sha256", ""))],
        [output + "/bin/syft", "version", "-o", "json"],
    ]
    hashes: list[dict[str, Any]] = []
    for index, (record, label, argv) in enumerate(zip(commands, SYFT_COMMAND_LABELS, expected_argv, strict=True)):
        if (not isinstance(record, dict) or record.get("label") != label
                or type(record.get("exit_status")) is not int or record["exit_status"] != 0
                or record.get("argv") != argv or record.get("log_path", f"logs/{index:02d}-{label}.log") != f"logs/{index:02d}-{label}.log"):
            fail("original_syft_command_receipt_invalid")
        for field in ("stdout_sha256", "stderr_sha256", "log_sha256"):
            if not isinstance(record.get(field), str) or not SHA256.fullmatch(record[field]):
                fail("original_syft_command_digest_invalid")
        log_path = original_root / "logs" / f"{index:02d}-{label}.log"
        log_bytes = secure_read(log_path, 2 * 1024 * 1024, "original_syft_log")
        observed, size = hashlib.sha256(log_bytes).hexdigest(), len(log_bytes)
        if observed != record["log_sha256"] or size != record.get("log_bytes"):
            fail("original_syft_log_hash_mismatch")
        if label == "verify-signed-checksum-document":
            if (log_bytes != f"OK: {checksum_file}\n".encode()
                    or record["stdout_sha256"] != hashlib.sha256(b"").hexdigest()
                    or record["stderr_sha256"] != hashlib.sha256(log_bytes).hexdigest()):
                fail("original_syft_signature_log_invalid")
        elif label == "extract-syft-archive":
            extraction = strict_json(log_bytes, "original_syft_extraction_log", 2 * 1024 * 1024)
            if extraction != {"archive": receipt.get("archive"), "binary": "syft"}:
                fail("original_syft_extraction_log_invalid")
        elif label == "syft-version":
            if strict_json(log_bytes, "original_syft_version_log", 2 * 1024 * 1024) != receipt.get("version_probe"):
                fail("original_syft_version_log_invalid")
        hashes.append({"label": label, "bytes": size, "sha256": observed})
    return hashes


def validate_syft_receipt(original_root: Path, embedded_report_path: Path, fresh_root: Path,
                          fresh_report: Any, prep: dict[str, Any], work_dir: Path,
                          *, consumer_run: int, consumer_attempt: int) -> tuple[Path, Path, str]:
    original_receipt_path = original_root / "evidence/receipt.json"
    original_validation_path = original_root / "evidence/validation-report.json"
    original_receipt, original_bytes = load_json_file(original_receipt_path, "original_syft_receipt")
    original_report, _ = load_json_file(original_validation_path, "original_syft_validation")
    prep_syft = prep.get("syft_qualification")
    if not isinstance(prep_syft, dict):
        fail("preparation_syft_qualification_missing")
    original_sha = hashlib.sha256(original_bytes).hexdigest()
    if (prep_syft.get("receipt_sha256") != original_sha
            or prep_syft.get("binary_sha256") != original_receipt.get("binary_sha256")
            or prep_syft.get("target") != "linux-amd64" or prep_syft.get("platform") != "linux/amd64"):
        fail("preparation_syft_pin_mismatch")
    log_hashes = validate_original_syft_commands(original_root, original_receipt,
                                                run_id=consumer_run, attempt=consumer_attempt)
    if (original_report.get("schema") != "kairos.syft-installation-validation.v1"
            or original_report.get("result") != "pass" or original_report.get("target") != "linux-amd64"
            or original_report.get("platform") != "linux/amd64" or original_report.get("receipt_sha256") != original_sha
            or original_report.get("binary_sha256") != original_receipt.get("binary_sha256")
            or original_report.get("validated_commands") != 9 or original_report.get("validated_logs") != 9):
        fail("original_syft_report_binding_invalid")
    if (fresh_report.get("schema") != "kairos.syft-installation-validation.v1"
            or fresh_report.get("result") != "pass" or fresh_report.get("target") != "linux-amd64"
            or fresh_report.get("platform") != "linux/amd64" or fresh_report.get("validated_commands") != 9
            or fresh_report.get("validated_logs") != 9):
        fail("fresh_syft_report_binding_invalid")
    report_keys = {"schema", "result", "target", "platform", "version", "installer_source_sha256",
                   "verifier_lock_sha256", "receipt_sha256", "archive_sha256", "binary_sha256",
                   "validated_commands", "validated_logs", "validated_members"}
    if set(original_report) != report_keys or set(fresh_report) != report_keys:
        fail("syft_native_report_shape_invalid")
    if any(original_report.get(key) != fresh_report.get(key) for key in report_keys - {"receipt_sha256"}):
        fail("original_and_fresh_syft_report_identity_mismatch")
    fresh_receipt_path = fresh_root / "evidence/receipt.json"
    fresh_receipt, fresh_bytes = load_json_file(fresh_receipt_path, "fresh_syft_receipt")
    if (fresh_report.get("receipt_sha256") != hashlib.sha256(fresh_bytes).hexdigest()
            or fresh_report.get("binary_sha256") != fresh_receipt.get("binary_sha256")):
        fail("fresh_syft_receipt_report_mismatch")
    if (original_report != load_json_file(embedded_report_path, "embedded_syft_report")[0]
            or original_report.get("binary_sha256") != fresh_report.get("binary_sha256")):
        fail("original_and_fresh_syft_report_mismatch")
    for key in SYFT_STABLE_FIELDS:
        if original_receipt.get(key) != fresh_receipt.get(key):
            fail("original_and_fresh_syft_identity_mismatch")
    if (prep_syft.get("installer_source_sha256") != fresh_report.get("installer_source_sha256")
            or prep_syft.get("verifier_lock_sha256") != fresh_report.get("verifier_lock_sha256")
            or prep_syft.get("binary_sha256") != fresh_report.get("binary_sha256")):
        fail("preparation_syft_source_or_lock_mismatch")
    record = {"schema": "kairos.mainline-syft-requalification.v1", "original_receipt_sha256": original_sha,
              "fresh_receipt_sha256": hashlib.sha256(fresh_bytes).hexdigest(),
              "binary_sha256": fresh_report["binary_sha256"], "target": "linux-amd64",
              "original_validated_log_hashes": log_hashes,
              "receipt_sha_difference_expected": original_sha != hashlib.sha256(fresh_bytes).hexdigest(),
              "note": "Fresh receipt bytes may differ by installation context; binary digest and trusted installation identity must match."}
    write_exclusive(work_dir / "syft-requalification.json", (json.dumps(record, sort_keys=True, indent=2) + "\n").encode())
    binary = fresh_root / "bin/syft"
    binary_hash, _ = secure_digest(binary, 256 * 1024 * 1024, "fresh_syft_binary")
    if binary_hash != fresh_report["binary_sha256"]:
        fail("fresh_syft_binary_hash_mismatch")
    return binary, fresh_receipt_path, binary_hash


def prepare(args: argparse.Namespace) -> dict[str, Any]:
    deadline = time.monotonic() + MAX_PIPELINE_SECONDS
    repository = require_path_chain(args.repository, leaf="directory")
    release_source = args.release_source_commit
    if not isinstance(release_source, str) or not COMMIT.fullmatch(release_source):
        fail("release_source_commit_invalid")
    if platform.system() != "Linux" or platform.machine().lower() not in {"x86_64", "amd64"}:
        fail("native_linux_amd64_required")
    producer_pin_bytes = secure_read(absolute(args.producer_pins_json), MAX_PIN_BYTES, "producer_pins")
    consumer_pin_bytes = secure_read(absolute(args.consumer_pins_json), MAX_PIN_BYTES, "consumer_pins")
    producer = validate_pin_bytes(producer_pin_bytes, PRODUCER_PIN_KEYS, "producer_pins")
    consumer = validate_pin_bytes(consumer_pin_bytes, CONSUMER_PIN_KEYS, "consumer_pins")
    producer_run = positive_pin(producer["run_id"], "producer_run_id")
    producer_artifact = positive_pin(producer["artifact_id"], "producer_artifact_id")
    producer_zip_bytes = positive_pin(producer["archive_zip_bytes"], "producer_zip_bytes", MAX_ARCHIVE_BYTES)
    consumer_run = positive_pin(consumer["run_id"], "consumer_run_id")
    consumer_attempt = positive_pin(consumer["run_attempt"], "consumer_run_attempt", 2**31 - 1)
    consumer_artifact = positive_pin(consumer["artifact_id"], "consumer_artifact_id")
    consumer_zip_bytes = positive_pin(consumer["archive_zip_bytes"], "consumer_zip_bytes", MAX_PIN_BYTES)
    for label, value in (("producer_source_commit", producer["source_commit"]),
                         ("producer_tree", producer["producer_tree"]),
                         ("consumer_source_commit", consumer["source_commit"])):
        if not COMMIT.fullmatch(value):
            fail(label + "_invalid")
    for label, value in (("producer_zip_sha256", producer["archive_zip_sha256"]),
                         ("consumer_zip_sha256", consumer["archive_zip_sha256"])):
        if not SHA256.fullmatch(value):
            fail(label + "_invalid")
    if producer["source_commit"] != release_source or consumer["source_commit"] != release_source:
        fail("release_source_pin_mismatch")
    head = git_text(repository, "rev-parse", "HEAD")
    tree = git_text(repository, "rev-parse", release_source + "^{tree}")
    if head != release_source or not COMMIT.fullmatch(head) or not COMMIT.fullmatch(tree):
        fail("release_checkout_head_or_tree_mismatch")
    if producer["producer_tree"] != tree:
        fail("producer_tree_pin_mismatch")

    work_dir = absolute(args.work_dir)
    output = absolute(args.output)
    parent = work_dir.parent
    if work_dir == Path(work_dir.anchor) or output.parent != work_dir or output.name != RELEASE_OUTPUT_NAME:
        fail("work_or_output_layout_invalid")
    require_path_chain(parent, leaf="directory")
    if work_dir.exists() or work_dir.is_symlink():
        fail("work_directory_must_be_fresh")
    if output.exists() or output.is_symlink():
        fail("output_must_be_fresh")
    for pin_path in (absolute(args.producer_pins_json), absolute(args.consumer_pins_json)):
        if pin_path == work_dir or pin_path in work_dir.parents or work_dir in pin_path.parents:
            fail("pins_work_directory_overlap")

    # Require the selected release checkout itself to match the Git blobs used
    # by every executable subprocess and trust input before any API call.
    producer_script = "packaging/scripts/acquire_package_archive_bundle.py"
    consumer_script = "packaging/scripts/acquire_archive_consumer_evidence.py"
    expectation_script = "packaging/scripts/build_archive_evidence_expectations.py"
    verifier_script = "packaging/scripts/verify_archive_supply_chain_evidence.py"
    adapter_script = "packaging/scripts/prepare_verified_archive_release.py"
    installer_script = "scripts/supply_chain/install_verified_syft.py"
    syft_verifier = "scripts/supply_chain/verify_syft_installation_receipt.py"
    schema_path = repository / SCHEMA
    trusted_sources = trusted_source_map(repository, release_source)
    readback_validator_bytes = trusted_sources[READBACK_VALIDATOR]
    runner = load_bounded_runner(trusted_sources[producer_script])

    os.mkdir(work_dir, 0o700)
    (work_dir / "logs").mkdir(mode=0o700)
    captured_tools = capture_tool_tree(work_dir, trusted_sources)
    manifest = {"schema": "kairos.mainline-archive-release-preparation.v1", "repository": REPOSITORY,
                "release_source_commit": release_source, "release_tree": tree,
                "trusted_tool_sha256": {path: hashlib.sha256(data).hexdigest()
                                        for path, data in sorted(trusted_sources.items())},
                "producer_pins_sha256": hashlib.sha256(producer_pin_bytes).hexdigest(),
                "consumer_pins_sha256": hashlib.sha256(consumer_pin_bytes).hexdigest(),
                "commands": [], "claim_scope": "local archive evidence preparation only; publication disabled"}
    write_exclusive(work_dir / "preparation-start.json", (json.dumps(manifest, sort_keys=True, indent=2) + "\n").encode())
    original_cwd = Path.cwd()
    readback_path = work_dir / "captured-tools" / READBACK_VALIDATOR
    try:
        os.chdir(repository)
        producer_root = work_dir / "producer-acquisition"
        consumer_root = work_dir / "consumer-acquisition"
        producer_argv = [sys.executable, str(captured_tools[producer_script]), "--run-id", str(producer_run),
                         "--require-main-dispatch", "--acquisition-output", str(producer_root)]
        run_trusted_script(repository, release_source, trusted_sources, work_dir, runner,
                           "acquire-producer", producer_script, producer_argv, deadline=deadline)
        archive_zip = producer_root / f"{producer_artifact}.zip"
        validate_native_producer_records(producer_root, run_id=producer_run, artifact_id=producer_artifact,
                                         source=release_source, tree=tree, zip_sha=producer["archive_zip_sha256"])
        observed_zip_sha, observed_zip_bytes = secure_digest(archive_zip, MAX_ARCHIVE_BYTES, "producer_zip")
        if observed_zip_sha != producer["archive_zip_sha256"] or observed_zip_bytes != producer_zip_bytes:
            fail("reacquired_producer_zip_pin_mismatch")

        consumer_argv = [sys.executable, str(captured_tools[consumer_script]), "--run-id", str(consumer_run),
                         "--run-attempt", str(consumer_attempt), "--artifact-id", str(consumer_artifact),
                         "--source-commit", release_source, "--archive-zip-sha256", consumer["archive_zip_sha256"],
                         "--archive-zip-bytes", str(consumer_zip_bytes),
                         "--expected-acquisition-helper-sha256", hashlib.sha256(trusted_sources[producer_script]).hexdigest(),
                         "--output", str(consumer_root)]
        run_trusted_script(repository, release_source, trusted_sources, work_dir, runner,
                           "acquire-consumer", consumer_script, consumer_argv, deadline=deadline)
        validate_native_consumer_records(consumer_root, run_id=consumer_run, attempt=consumer_attempt,
                                         artifact_id=consumer_artifact, source=release_source,
                                         zip_sha=consumer["archive_zip_sha256"])
        observed_consumer_zip_sha, observed_consumer_zip_bytes = secure_digest(
            consumer_root / f"{consumer_artifact}.zip", MAX_PIN_BYTES, "consumer_zip")
        if (observed_consumer_zip_sha != consumer["archive_zip_sha256"]
                or observed_consumer_zip_bytes != consumer_zip_bytes):
            fail("reacquired_consumer_zip_pin_mismatch")
        consumer_payload = consumer_root / "evidence"
        prefix = f"archive-evidence-{consumer_run}-{consumer_attempt}"
        syft_prefix = f"syft-linux-amd64-{consumer_run}-{consumer_attempt}"
        evidence_dir = consumer_payload / prefix / "evidence"
        prep_path = consumer_payload / prefix / "preparation-receipt.json"
        prep, _ = load_json_file(prep_path, "consumer_preparation")
        producer_pins = prep.get("producer_pins")
        validate_embedded_preparation(consumer_payload / prefix, prep, work_dir, source=release_source,
            producer_values={"run_id": producer_run, "artifact_id": producer_artifact,
                             "producer_tree": tree, "archive_zip_sha256": producer["archive_zip_sha256"],
                             "archive_zip_bytes": producer_zip_bytes})
        original_syft_root = consumer_payload / syft_prefix
        original_validation_path = original_syft_root / "evidence/validation-report.json"
        original_report, _ = load_json_file(original_validation_path, "embedded_syft_report")

        fresh_syft_root = work_dir / "fresh-syft"
        run_trusted_script(repository, release_source, trusted_sources, work_dir, runner,
                    "install-fresh-syft", installer_script,
                    [sys.executable, str(captured_tools[installer_script]), "--output-dir", str(fresh_syft_root)], timeout=1800, deadline=deadline)
        fresh_report_bytes = run_trusted_script(repository, release_source, trusted_sources, work_dir, runner,
            "validate-fresh-syft", syft_verifier,
            [sys.executable, str(captured_tools[syft_verifier]), "--target", "linux-amd64",
             "--output-dir", str(fresh_syft_root), "--repo", str(repository)], timeout=300, deadline=deadline)
        write_exclusive(fresh_syft_root / "evidence/validation-report.json", fresh_report_bytes)
        fresh_report = strict_json(fresh_report_bytes, "fresh_syft_native_report")
        binary, fresh_receipt_path, binary_sha = validate_syft_receipt(
            original_syft_root, original_validation_path, fresh_syft_root, fresh_report, prep, work_dir,
            consumer_run=consumer_run, consumer_attempt=consumer_attempt)

        expectations = work_dir / "expectations"
        preparation_receipt = work_dir / "expectation-preparation.json"
        index_path = producer_root / "bundle/ARCHIVE-INDEX.json"
        index_sha, _ = secure_digest(index_path, MAX_JSON_BYTES, "archive_index")
        expected_inputs_path = expectations / "expected-inputs.json"
        expected_binding_path = expectations / "outer-binding.json"
        schema_path = repository / SCHEMA
        expectation_argv = [sys.executable, str(captured_tools[expectation_script]), "--repository", str(repository),
            "--trusted-consumer-sha", release_source, "--acquisition-dir", str(producer_root),
            "--archive-bundle", str(producer_root / "bundle"), "--archive-zip", str(archive_zip),
            "--run-id", str(producer_run), "--artifact-id", str(producer_artifact),
            "--source-commit", release_source, "--producer-tree", tree,
            "--archive-zip-sha256", producer["archive_zip_sha256"], "--archive-zip-bytes", str(producer_zip_bytes),
            "--syft", str(binary), "--syft-sha256", binary_sha,
            "--syft-receipt", str(fresh_receipt_path),
            "--syft-receipt-sha256", secure_digest(fresh_receipt_path, MAX_JSON_BYTES, "fresh_syft_receipt")[0],
            "--output-dir", str(expectations), "--preparation-receipt", str(preparation_receipt)]
        run_trusted_script(repository, release_source, trusted_sources, work_dir, runner,
                           "derive-independent-expectations", expectation_script, expectation_argv, timeout=300, deadline=deadline)
        expected_inputs, _ = load_json_file(expected_inputs_path, "independent_expected_inputs")
        expected_binding, _ = load_json_file(expected_binding_path, "independent_expected_binding")
        if expected_inputs.get("source_commit") != release_source or expected_inputs.get("archive_index_sha256") != index_sha:
            fail("derived_expectation_identity_mismatch")

        # Run the full evidence verifier directly and retain its native result;
        # the archive adapter repeats that verifier before building any copies.
        verifier_self_sha = hashlib.sha256(trusted_sources[verifier_script]).hexdigest()
        verifier_report_bytes = run_trusted_script(repository, release_source, trusted_sources, work_dir, runner,
            "verify-full-archive-profile", verifier_script,
            [sys.executable, str(captured_tools[verifier_script]), "--evidence-dir", str(evidence_dir),
             "--archive-bundle", str(producer_root / "bundle"), "--archive-zip", str(archive_zip),
             "--acquisition-dir", str(producer_root), "--expected-inputs", str(expected_inputs_path),
             "--expected-binding", str(expected_binding_path), "--expected-verifier-sha256", verifier_self_sha,
             "--spdx-schema", str(schema_path)], timeout=900, deadline=deadline)
        verifier_report = strict_json(verifier_report_bytes, "archive_verifier_report")
        validate_report(verifier_report, index_sha)

        adapter_argv = [sys.executable, str(captured_tools[adapter_script]), "--evidence-dir", str(evidence_dir),
            "--archive-bundle", str(producer_root / "bundle"), "--archive-zip", str(archive_zip),
            "--acquisition-dir", str(producer_root), "--expected-inputs", str(expected_inputs_path),
            "--expected-binding", str(expected_binding_path), "--expected-verifier-sha256", verifier_self_sha,
            "--spdx-schema", str(schema_path), "--release-source-commit", release_source,
            "--output", str(output)]
        adapter_bytes = run_trusted_script(repository, release_source, trusted_sources, work_dir, runner,
            "prepare-verified-archive-output", adapter_script, adapter_argv, timeout=1200, deadline=deadline)
        adapter_report = strict_json(adapter_bytes, "archive_adapter_report")
        if (not isinstance(adapter_report, dict) or adapter_report.get("valid") is not True
                or adapter_report.get("release_stage") != "actual-package-archives"
                or adapter_report.get("source_commit") != release_source or adapter_report.get("archive_count") != 8
                or adapter_report.get("archive_index_sha256") != index_sha or adapter_report.get("output") != str(output)
                or adapter_report.get("claim_scope") != CLAIM_SCOPE or not output.is_dir()):
            fail("archive_adapter_report_profile")
        readback_bytes = run_captured_validator(repository, release_source, trusted_sources, work_dir, runner,
            readback_path, readback_validator_bytes,
            [sys.executable, str(readback_path), "--release-root", str(output),
             "--verified-archive-index", str(index_path), "--archive-index-sha256", index_sha,
             "--release-source-commit", release_source], deadline=deadline)
        readback_report = strict_json(readback_bytes, "actual_archive_readback_report")
        validate_readback_report(readback_report, index_sha, release_source)
        write_exclusive(work_dir / "actual-archive-readback-report.json", (json.dumps(readback_report, sort_keys=True, indent=2) + "\n").encode())
        check_deadline(deadline)
        report_path = work_dir / "validation-report.json"
        final_report = {"schema": "kairos.mainline-archive-release-preparation-report.v1",
            "valid": True, "release_source_commit": release_source, "release_tree": tree,
            "producer_run_id": producer_run, "producer_artifact_id": producer_artifact,
            "consumer_run_id": consumer_run, "consumer_run_attempt": consumer_attempt,
            "consumer_artifact_id": consumer_artifact, "archive_index_sha256": index_sha,
            "producer_archive_zip_sha256": producer["archive_zip_sha256"],
            "producer_archive_zip_bytes": producer_zip_bytes,
            "consumer_archive_zip_sha256": consumer["archive_zip_sha256"],
            "consumer_archive_zip_bytes": consumer_zip_bytes,
            "archive_count": 8, "ecosystem_count": 7, "spdx_document_count": 9,
            "evidence_file_count": 44, "syft_binary_sha256": binary_sha,
            "readback_validator_sha256": hashlib.sha256(readback_validator_bytes).hexdigest(),
            "actual_archive_readback_report": "actual-archive-readback-report.json",
            "output": str(output), "publication_enabled": False,
            "claim_scope": "actual archive preparation only; not full Rust workspace readiness, signed release attestation, release acceptance, or publication"}
        write_exclusive(report_path, (json.dumps(final_report, sort_keys=True, indent=2) + "\n").encode())
        complete = {**manifest, "commands": strict_json(secure_read(work_dir / "command-records.json", MAX_JSON_BYTES,
                                                                        "command_records"), "command_records"),
                    "status": "prepared", "validation_report_sha256": hashlib.sha256(secure_read(report_path, MAX_JSON_BYTES,
                                                                                           "validation_report")).hexdigest()}
        write_exclusive(work_dir / "preparation-receipt.json", (json.dumps(complete, sort_keys=True, indent=2) + "\n").encode())
        return final_report
    except BaseException:
        raise
    finally:
        os.chdir(original_cwd)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repository", type=Path, required=True)
    parser.add_argument("--release-source-commit", required=True)
    parser.add_argument("--producer-pins-json", type=Path, required=True)
    parser.add_argument("--consumer-pins-json", type=Path, required=True)
    parser.add_argument("--work-dir", type=Path, required=True, help="new private evidence work directory")
    parser.add_argument("--output", type=Path, required=True, help="new actual-package-archives child of work-dir")
    args = parser.parse_args(argv)
    try:
        report = prepare(args)
    except Exception as exc:
        print(json.dumps({"valid": False, "reason": str(exc) if isinstance(exc, GateError) else type(exc).__name__},
                         sort_keys=True, separators=(",", ":")), file=sys.stderr)
        return 1
    print(json.dumps(report, sort_keys=True, separators=(",", ":")))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
