#!/usr/bin/env python3
"""Verify retained archive evidence, then prepare exact archive release subjects.

This adapter is local evidence preparation only. It does not acquire artifacts,
generate SBOM/provenance, publish packages, or establish release acceptance.
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
import stat
import subprocess
import sys
import tempfile
import types
from typing import Any


SCRIPT_DIR = Path(__file__).absolute().parent
VERIFIER = "verify_archive_supply_chain_evidence.py"
HELPERS = (
    "build_archive_supply_chain.py",
    "build_archive_release_manifest.py",
    "build_package_archive_bundle.py",
    "acquire_package_archive_bundle.py",
    "validate_archive_copy_provenance.py",
)
MAX_JSON_BYTES = 8 * 1024 * 1024
MAX_SOURCE_BYTES = 8 * 1024 * 1024
MAX_ARCHIVE_BYTES = 512 * 1024 * 1024
MAX_SUBPROCESS_SECONDS = 330
MAX_SUBPROCESS_OUTPUT = 64 * 1024
SHA256 = re.compile(r"^[0-9a-f]{64}$")
COMMIT = re.compile(r"^[0-9a-f]{40}$")
PROFILE = "kairos-archive-copy-evidence-v1"
CLAIM_SCOPE = "local consistency only; unsigned and untrusted builder; no SLSA level or release acceptance"
ECOSYSTEMS = frozenset({"rust", "python", "r", "julia", "typescript", "nuget", "go"})
ARCHIVE_KINDS = {"rust": "crate", "python": "python-distribution", "r": "r-source-package",
                 "julia": "julia-source-archive", "typescript": "npm-package",
                 "nuget": "nuget-package", "go": "go-source-archive"}
ARCHIVE_EXTENSIONS = {"rust": (".crate",), "python": (".whl", ".tar.gz"), "r": (".tar.gz",),
                      "julia": (".tar.gz",), "typescript": (".tgz",), "nuget": (".nupkg",),
                      "go": (".tar.gz",)}


class GateError(Exception):
    """A bounded, safe-to-report gate failure."""


def fail(code: str) -> None:
    raise GateError(code)


def absolute(path: Path) -> Path:
    raw = Path(path)
    if ".." in raw.parts:
        fail("path_parent_component")
    return Path(os.path.abspath(os.fspath(raw)))


def inspect_path(path: Path, *, kind: str, allow_missing: bool = False) -> Path:
    """Reject symlink ancestors and require a regular leaf or directory."""
    target = absolute(path)
    current = Path(target.anchor)
    parts = target.parts[1:]
    for index, part in enumerate(parts):
        current = current / part
        try:
            mode = os.lstat(current).st_mode
        except FileNotFoundError:
            if allow_missing and index == len(parts) - 1:
                return target
            fail("path_missing")
        if stat.S_ISLNK(mode):
            fail("path_symlink")
        leaf = index == len(parts) - 1
        if not leaf and not stat.S_ISDIR(mode):
            fail("path_parent_not_directory")
        if leaf:
            valid = stat.S_ISDIR(mode) if kind == "directory" else stat.S_ISREG(mode)
            if not valid:
                fail("path_wrong_type")
    if not parts:
        fail("path_root_not_allowed")
    return target


def secure_read(path: Path, limit: int, label: str) -> bytes:
    """Read a regular file through no-follow directory descriptors."""
    target = absolute(path)
    if not target.parts or target == Path(target.anchor):
        fail(label + "_path")
    flags_dir = os.O_RDONLY | getattr(os, "O_DIRECTORY", 0) | getattr(os, "O_NOFOLLOW", 0)
    flags_file = os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0) | getattr(os, "O_NONBLOCK", 0)
    fd = os.open(target.anchor, flags_dir)
    try:
        for component in target.parts[1:-1]:
            next_fd = os.open(component, flags_dir, dir_fd=fd)
            os.close(fd)
            fd = next_fd
        leaf_fd = os.open(target.parts[-1], flags_file, dir_fd=fd)
        try:
            if not stat.S_ISREG(os.fstat(leaf_fd).st_mode):
                fail(label + "_not_regular")
            data = bytearray()
            while len(data) <= limit:
                chunk = os.read(leaf_fd, min(65536, limit + 1 - len(data)))
                if not chunk:
                    break
                data.extend(chunk)
            if len(data) > limit:
                fail(label + "_too_large")
            return bytes(data)
        finally:
            os.close(leaf_fd)
    except OSError as exc:
        raise GateError(label + "_read_failed") from exc
    finally:
        os.close(fd)


def secure_file_digest(path: Path, limit: int, label: str) -> tuple[str, int]:
    """Hash a bounded regular file without loading an archive into memory."""
    target = absolute(path)
    if not target.parts or target == Path(target.anchor):
        fail(label + "_path")
    flags_dir = os.O_RDONLY | getattr(os, "O_DIRECTORY", 0) | getattr(os, "O_NOFOLLOW", 0)
    flags_file = os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0) | getattr(os, "O_NONBLOCK", 0)
    fd = os.open(target.anchor, flags_dir)
    try:
        for component in target.parts[1:-1]:
            next_fd = os.open(component, flags_dir, dir_fd=fd)
            os.close(fd)
            fd = next_fd
        leaf_fd = os.open(target.parts[-1], flags_file, dir_fd=fd)
        try:
            if not stat.S_ISREG(os.fstat(leaf_fd).st_mode):
                fail(label + "_not_regular")
            hasher = hashlib.sha256()
            size = 0
            while True:
                chunk = os.read(leaf_fd, 1024 * 1024)
                if not chunk:
                    break
                size += len(chunk)
                if size > limit:
                    fail(label + "_too_large")
                hasher.update(chunk)
            return hasher.hexdigest(), size
        finally:
            os.close(leaf_fd)
    except OSError as exc:
        raise GateError(label + "_read_failed") from exc
    finally:
        os.close(fd)


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def reject_constant(value: str) -> None:
    raise ValueError("non-finite JSON number")


def finite_float(value: str) -> float:
    number = float(value)
    if not math.isfinite(number):
        raise ValueError("non-finite JSON number")
    return number


def pairs_no_duplicates(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("duplicate JSON key")
        result[key] = value
    return result


def check_depth(value: Any, depth: int = 0) -> None:
    if depth > 128:
        raise ValueError("JSON nesting exceeds limit")
    if isinstance(value, dict):
        for child in value.values():
            check_depth(child, depth + 1)
    elif isinstance(value, list):
        for child in value:
            check_depth(child, depth + 1)


def strict_json(data: bytes, label: str) -> Any:
    try:
        value = json.loads(data.decode("utf-8"), object_pairs_hook=pairs_no_duplicates,
                           parse_constant=reject_constant, parse_float=finite_float)
        check_depth(value)
        return value
    except (UnicodeDecodeError, json.JSONDecodeError, ValueError, RecursionError) as exc:
        raise GateError(label + "_invalid_json") from exc


def contained_or_equal(left: Path, right: Path) -> bool:
    return left == right or left in right.parents or right in left.parents


def validate_expected_inputs(value: Any, source_commit: str) -> tuple[dict[str, str], str]:
    if not isinstance(value, dict) or value.get("source_commit") != source_commit:
        fail("expected_inputs_source_mismatch")
    index_hash = value.get("archive_index_sha256")
    if not isinstance(index_hash, str) or not SHA256.fullmatch(index_hash):
        fail("expected_inputs_index_hash")
    dependencies = value.get("dependencies")
    if not isinstance(dependencies, list):
        fail("expected_inputs_dependencies")
    result: dict[str, str] = {}
    for row in dependencies:
        if not isinstance(row, dict) or set(row) != {"id", "sha256"}:
            fail("expected_inputs_dependency_shape")
        identifier, value_hash = row["id"], row["sha256"]
        if not isinstance(identifier, str) or not isinstance(value_hash, str) or not SHA256.fullmatch(value_hash) or identifier in result:
            fail("expected_inputs_dependency_value")
        result[identifier] = value_hash
    required = {"schema:spdx-2.3", *("packaging/scripts/" + name for name in HELPERS)}
    if not required.issubset(result):
        fail("expected_inputs_missing_trusted_pin")
    return result, index_hash


def validate_verifier_result(data: bytes, archive_count: int, index_hash: str) -> None:
    result = strict_json(data, "verifier_result")
    required = {"valid", "profile", "archive_count", "ecosystem_count", "spdx_document_count",
                "evidence_file_count", "archive_index_sha256", "statement_sha256", "claim_scope"}
    if not isinstance(result, dict) or set(result) != required:
        fail("verifier_result_shape")
    if result["valid"] is not True or result["profile"] != PROFILE or result["claim_scope"] != CLAIM_SCOPE:
        fail("verifier_result_profile")
    for key in ("archive_count", "ecosystem_count", "spdx_document_count", "evidence_file_count"):
        if type(result[key]) is not int or result[key] < 1:
            fail("verifier_result_count")
    if result["archive_count"] != archive_count or result["ecosystem_count"] != 7 or result["spdx_document_count"] != archive_count + 1:
        fail("verifier_result_counts_mismatch")
    for key in ("archive_index_sha256", "statement_sha256"):
        if not isinstance(result[key], str) or not SHA256.fullmatch(result[key]):
            fail("verifier_result_digest")
    if result["archive_index_sha256"] != index_hash:
        fail("verifier_result_index_mismatch")


def load_acquisition_runner(source: bytes):
    """Load the process runner from already-pinned captured helper bytes."""
    module = types.ModuleType("_captured_archive_acquisition_runner")
    exec(compile(source, "acquire_package_archive_bundle.py", "exec"), module.__dict__)
    return module._run_bounded_process


def run_bounded(argv: list[str], runner) -> tuple[int, bytes, bytes]:
    """Run through the pinned helper's bounded process-group runner."""
    try:
        _, stdout = runner(argv, MAX_SUBPROCESS_OUTPUT, MAX_SUBPROCESS_SECONDS)
        return 0, stdout or b"", b""
    except subprocess.CalledProcessError as exc:
        return int(exc.returncode), b"", b""
    except (TimeoutError, ValueError) as exc:
        message = str(exc).lower()
        code = "subprocess_timeout" if "timed out" in message else "subprocess_output_limit" if "exceeds byte limit" in message else "subprocess_failed"
        raise GateError(code) from exc
    except BaseException as exc:
        raise GateError("subprocess_failed") from exc


def safe_rel(value: Any) -> str:
    if not isinstance(value, str) or not value or "\\" in value:
        fail("archive_index_path")
    path = PurePosixPath(value)
    if path.is_absolute() or any(part in ("", ".", "..") for part in path.parts) or path.as_posix() != value:
        fail("archive_index_path")
    return value


def load_index_rows(bundle: Path, source_commit: str) -> tuple[bytes, list[dict[str, Any]]]:
    index_bytes = secure_read(bundle / "ARCHIVE-INDEX.json", MAX_JSON_BYTES, "archive_index")
    index = strict_json(index_bytes, "archive_index")
    if not isinstance(index, dict) or set(index) != {"schema_version", "source_commit", "created_at_utc", "artifacts"}:
        fail("archive_index_shape")
    rows = index["artifacts"]
    if (type(index["schema_version"]) is not int or index["schema_version"] != 1
            or index["source_commit"] != source_commit or not isinstance(rows, list) or not rows):
        fail("archive_index_value")
    receipt_bytes = secure_read(bundle / "BUILD-RECEIPT.json", MAX_JSON_BYTES, "build_receipt")
    receipt = strict_json(receipt_bytes, "build_receipt")
    if (not isinstance(receipt, dict) or set(receipt) != {"source_commit", "ecosystems"}
            or receipt["source_commit"] != source_commit or not isinstance(receipt["ecosystems"], dict)
            or set(receipt["ecosystems"]) != ECOSYSTEMS):
        fail("build_receipt_shape")
    builder_receipts = receipt["ecosystems"]
    for ecosystem, builder in builder_receipts.items():
        if (not isinstance(builder, dict) or builder.get("ecosystem") != ecosystem
                or builder.get("source_commit") != source_commit or type(builder.get("exit_status")) is not int
                or builder["exit_status"] != 0
                or any(not isinstance(builder.get(key), str) or not builder[key].strip()
                       for key in ("command", "toolchain", "platform"))):
            fail("build_receipt_builder")
    seen: set[str] = set()
    for row in rows:
        if not isinstance(row, dict) or set(row) != {"ecosystem", "kind", "path", "bytes", "sha256", "builder"}:
            fail("archive_index_row_shape")
        ecosystem = row["ecosystem"]
        if ecosystem not in ECOSYSTEMS:
            fail("archive_index_row_value")
        relative = safe_rel(row["path"])
        if (row["kind"] != ARCHIVE_KINDS[ecosystem] or PurePosixPath(relative).parts[0] != ecosystem
                or not any(relative.endswith(ext) for ext in ARCHIVE_EXTENSIONS[ecosystem]) or relative in seen):
            fail("archive_index_row_value")
        seen.add(relative)
        if row["builder"] != builder_receipts[ecosystem]:
            fail("archive_index_builder_mismatch")
        if not isinstance(row["sha256"], str) or not SHA256.fullmatch(row["sha256"]):
            fail("archive_index_row_digest")
        if type(row["bytes"]) is not int or row["bytes"] <= 0 or row["bytes"] > MAX_ARCHIVE_BYTES:
            fail("archive_index_row_value")
    if {row["ecosystem"] for row in rows} != ECOSYSTEMS or [row["path"] for row in rows] != sorted(row["path"] for row in rows):
        fail("archive_index_row_coverage")
    return index_bytes, rows


def validate_output(bundle: Path, output: Path, source_commit: str, index_bytes: bytes, rows: list[dict[str, Any]]) -> dict[str, Any]:
    expected_artifacts: list[dict[str, Any]] = []
    expected_files = {"release-artifact-manifest.json", "SHA256SUMS", "RELEASE.txt"}
    expected_dirs: set[str] = set()
    for row in rows:
        if not isinstance(row, dict) or set(row) != {"ecosystem", "kind", "path", "bytes", "sha256", "builder"}:
            fail("archive_index_row_shape")
        rel = safe_rel(row["path"])
        if not isinstance(row["sha256"], str) or not SHA256.fullmatch(row["sha256"]):
            fail("archive_index_row_digest")
        if type(row["bytes"]) is not int or row["bytes"] < 0 or row["ecosystem"] not in {"rust", "python", "r", "julia", "typescript", "nuget", "go"} or not isinstance(row["kind"], str):
            fail("archive_index_row_value")
        if row["bytes"] > MAX_ARCHIVE_BYTES:
            fail("archive_index_row_too_large")
        source = inspect_path(bundle / rel, kind="file")
        source_sha, source_size = secure_file_digest(source, MAX_ARCHIVE_BYTES, "indexed_archive")
        if source_size != row["bytes"] or source_sha != row["sha256"]:
            fail("indexed_archive_changed")
        relative_output = "archives/" + rel
        expected_files.add(relative_output)
        parts = PurePosixPath(relative_output).parts
        expected_dirs.update("/".join(parts[:depth]) for depth in range(1, len(parts)))
        expected_artifacts.append({
            "path": relative_output,
            "sha256": row["sha256"],
            "bytes": row["bytes"],
            "ecosystem": "csharp" if row["ecosystem"] == "nuget" else row["ecosystem"],
            "archive_ecosystem": row["ecosystem"],
            "kind": row["kind"],
        })
    expected_artifacts.sort(key=lambda item: item["path"])
    actual_files: set[str] = set()
    actual_dirs: set[str] = set()
    for current, dirs, files in os.walk(output, followlinks=False):
        base = Path(current)
        for name in dirs:
            path = base / name
            if path.is_symlink():
                fail("output_symlink")
            actual_dirs.add(path.relative_to(output).as_posix())
        for name in files:
            path = base / name
            if path.is_symlink() or not path.is_file():
                fail("output_nonregular_file")
            actual_files.add(path.relative_to(output).as_posix())
    if actual_files != expected_files or actual_dirs != expected_dirs:
        fail("output_inventory_mismatch")
    manifest_bytes = secure_read(output / "release-artifact-manifest.json", MAX_JSON_BYTES, "output_manifest")
    manifest = strict_json(manifest_bytes, "output_manifest")
    expected_manifest = {
        "schema_version": 1,
        "release_stage": "actual-package-archives",
        "source_commit": source_commit,
        "production_publish_enabled": False,
        "archive_index_sha256": digest(index_bytes),
        "artifacts": expected_artifacts,
    }
    if manifest != expected_manifest:
        fail("output_manifest_mismatch")
    checksums = "".join(f"{item['sha256']}  {item['path']}\n" for item in expected_artifacts).encode()
    if secure_read(output / "SHA256SUMS", MAX_JSON_BYTES, "output_checksums") != checksums:
        fail("output_checksums_mismatch")
    release_text = f"Verified package archives from {source_commit}. Evidence preparation only; publication disabled.\n".encode()
    if secure_read(output / "RELEASE.txt", MAX_JSON_BYTES, "output_release_text") != release_text:
        fail("output_release_text_mismatch")
    for item in expected_artifacts:
        if item["bytes"] > MAX_ARCHIVE_BYTES:
            fail("output_archive_too_large")
        output_sha, output_size = secure_file_digest(output / item["path"], MAX_ARCHIVE_BYTES, "output_archive")
        if output_size != item["bytes"] or output_sha != item["sha256"]:
            fail("output_archive_mismatch")
    return manifest


def _unlink_new_output(output: Path) -> None:
    try:
        target = inspect_path(output, kind="directory")
    except GateError:
        return
    shutil.rmtree(target)


def install_new_output(staged: Path, output: Path) -> tuple[int, int]:
    """Reserve the destination atomically, then move only validated children into it."""
    try:
        os.mkdir(output, 0o700)
    except FileExistsError as exc:
        raise GateError("output_appeared_during_gate") from exc
    status = os.lstat(output)
    identity = (status.st_dev, status.st_ino)
    try:
        for child in staged.iterdir():
            os.rename(child, output / child.name)
        current = os.lstat(output)
        if (current.st_dev, current.st_ino) != identity or not stat.S_ISDIR(current.st_mode):
            fail("output_identity_changed")
        return identity
    except BaseException:
        current = os.lstat(output)
        if (current.st_dev, current.st_ino) == identity:
            shutil.rmtree(output)
        raise


def prepare(args: argparse.Namespace) -> dict[str, Any]:
    source_commit = args.release_source_commit
    if not isinstance(source_commit, str) or not COMMIT.fullmatch(source_commit):
        fail("release_source_commit_invalid")
    if not isinstance(args.expected_verifier_sha256, str) or not SHA256.fullmatch(args.expected_verifier_sha256):
        fail("verifier_pin_invalid")

    evidence = inspect_path(args.evidence_dir, kind="directory")
    bundle = inspect_path(args.archive_bundle, kind="directory")
    acquisition = inspect_path(args.acquisition_dir, kind="directory")
    archive_zip = inspect_path(args.archive_zip, kind="file")
    expected_inputs_path = inspect_path(args.expected_inputs, kind="file")
    expected_binding_path = inspect_path(args.expected_binding, kind="file")
    schema_path = inspect_path(args.spdx_schema, kind="file")
    output = absolute(args.output)
    if output == Path(output.anchor):
        fail("output_path_invalid")
    output_parent = inspect_path(output.parent, kind="directory")
    try:
        os.lstat(output)
    except FileNotFoundError:
        pass
    else:
        fail("output_must_not_exist")
    if bundle != acquisition / "bundle":
        fail("bundle_not_acquisition_child")
    if archive_zip.parent != acquisition:
        fail("archive_zip_not_acquisition_child")
    protected_inputs = (evidence, bundle, acquisition, archive_zip, expected_inputs_path, expected_binding_path, schema_path)
    if any(contained_or_equal(output, item) for item in protected_inputs):
        fail("output_input_overlap")
    if any(contained_or_equal(expected, untrusted) for expected in (expected_inputs_path, expected_binding_path, schema_path)
           for untrusted in (evidence, acquisition, archive_zip)):
        fail("trusted_input_not_independent")

    binding_bytes = secure_read(expected_binding_path, MAX_JSON_BYTES, "expected_binding")
    binding = strict_json(binding_bytes, "expected_binding")
    if not isinstance(binding, dict) or binding.get("source_commit") != source_commit:
        fail("binding_source_mismatch")
    inputs_bytes = secure_read(expected_inputs_path, MAX_JSON_BYTES, "expected_inputs")
    inputs = strict_json(inputs_bytes, "expected_inputs")
    dependency_hashes, index_hash = validate_expected_inputs(inputs, source_commit)
    if not isinstance(binding.get("spdx_schema_sha256"), str) or not SHA256.fullmatch(binding["spdx_schema_sha256"]):
        fail("binding_schema_pin")
    schema_bytes = secure_read(schema_path, MAX_JSON_BYTES, "spdx_schema")
    if digest(schema_bytes) != binding["spdx_schema_sha256"] or dependency_hashes["schema:spdx-2.3"] != digest(schema_bytes):
        fail("schema_pin_mismatch")

    # Capture and pin all executable sources before launching either child.
    source_bytes: dict[str, bytes] = {}
    verifier_source = secure_read(SCRIPT_DIR / VERIFIER, MAX_SOURCE_BYTES, "verifier_source")
    if digest(verifier_source) != args.expected_verifier_sha256:
        fail("verifier_pin_mismatch")
    source_bytes[VERIFIER] = verifier_source
    for helper in HELPERS:
        data = secure_read(SCRIPT_DIR / helper, MAX_SOURCE_BYTES, "helper_source")
        if digest(data) != dependency_hashes["packaging/scripts/" + helper]:
            fail("helper_pin_mismatch")
        source_bytes[helper] = data
    runner = load_acquisition_runner(source_bytes["acquire_package_archive_bundle.py"])

    index_bytes, rows = load_index_rows(bundle, source_commit)
    if digest(index_bytes) != index_hash:
        fail("outer_index_pin_mismatch")
    temp_root: str | None = None
    output_created = False
    try:
        # Stage on the destination filesystem so validated children can be
        # moved into an exclusively reserved output directory.
        temp_root = tempfile.mkdtemp(prefix=".kairos-archive-gate-", dir=output_parent)
        stage = Path(temp_root).resolve(strict=True)
        staged_inputs = {**source_bytes,
                         "expected-inputs.json": inputs_bytes,
                         "expected-binding.json": binding_bytes,
                         "spdx-schema.json": schema_bytes}
        for name, data in staged_inputs.items():
            target = stage / name
            flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL | getattr(os, "O_NOFOLLOW", 0)
            fd = os.open(target, flags, 0o600)
            with os.fdopen(fd, "wb") as stream:
                stream.write(data)
                stream.flush()
                os.fsync(stream.fileno())
            if digest(secure_read(target, MAX_SOURCE_BYTES, "staged_source")) != digest(data):
                fail("staged_source_mismatch")

        verifier_argv = [
            sys.executable, str(stage / VERIFIER),
            "--evidence-dir", str(evidence),
            "--archive-bundle", str(bundle),
            "--archive-zip", str(archive_zip),
            "--acquisition-dir", str(acquisition),
            "--expected-inputs", str(stage / "expected-inputs.json"),
            "--expected-binding", str(stage / "expected-binding.json"),
            "--expected-verifier-sha256", args.expected_verifier_sha256,
            "--spdx-schema", str(stage / "spdx-schema.json"),
        ]
        verifier_code, verifier_stdout, _ = run_bounded(verifier_argv, runner)
        if verifier_code != 0:
            fail("qualified_archive_verifier_failed")
        validate_verifier_result(verifier_stdout, len(rows), index_hash)

        staged_output = stage / "result"
        builder_argv = [sys.executable, str(stage / "build_archive_release_manifest.py"),
                        "--input", str(bundle), "--output", str(staged_output), "--source-commit", source_commit]
        builder_code, _, _ = run_bounded(builder_argv, runner)
        if builder_code != 0:
            fail("archive_manifest_builder_failed")
        manifest = validate_output(bundle, staged_output, source_commit, index_bytes, rows)
        output_identity = install_new_output(staged_output, output)
        output_created = True
        return {"valid": True, "release_stage": manifest["release_stage"], "source_commit": source_commit,
                "archive_count": len(manifest["artifacts"]), "archive_index_sha256": manifest["archive_index_sha256"],
                "output": str(output), "claim_scope": CLAIM_SCOPE}
    except BaseException:
        if output_created:
            current = os.lstat(output)
            if (current.st_dev, current.st_ino) == output_identity:
                _unlink_new_output(output)
        raise
    finally:
        if temp_root:
            shutil.rmtree(temp_root, ignore_errors=True)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--evidence-dir", required=True, type=Path)
    parser.add_argument("--archive-bundle", required=True, type=Path)
    parser.add_argument("--archive-zip", required=True, type=Path)
    parser.add_argument("--acquisition-dir", required=True, type=Path)
    parser.add_argument("--expected-inputs", required=True, type=Path)
    parser.add_argument("--expected-binding", required=True, type=Path)
    parser.add_argument("--expected-verifier-sha256", required=True)
    parser.add_argument("--spdx-schema", required=True, type=Path)
    parser.add_argument("--release-source-commit", required=True)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args(argv)
    try:
        result = prepare(args)
        print(json.dumps(result, sort_keys=True, separators=(",", ":")))
        return 0
    except GateError as exc:
        print(json.dumps({"valid": False, "error": str(exc)}, sort_keys=True, separators=(",", ":")))
        return 1
    except Exception:
        print(json.dumps({"valid": False, "error": "internal_gate_error"}, sort_keys=True, separators=(",", ":")))
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
