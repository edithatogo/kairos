#!/usr/bin/env python3
"""Read-only, fail-closed verifier for the Kairos archive-copy evidence profile.

This verifies local consistency and pinned inputs. It does not establish a
signature, trusted builder, SLSA level, production readiness, or release
acceptance. It never extracts archives, runs Syft, or writes to its inputs.
"""
from __future__ import annotations

import argparse
import gzip
import hashlib
import json
import math
import os
from pathlib import Path, PurePosixPath
import re
import stat
import struct
import signal
import sys
import tarfile
import time
import zipfile
from contextlib import contextmanager
from typing import Any, BinaryIO


MAX_JSON_BYTES = 8 * 1024 * 1024
MAX_JSON_DEPTH = 128
MAX_FILES = 128
MAX_TOTAL_BYTES = 4 * 1024**3
MAX_ARCHIVE_BYTES = 512 * 1024**2
MAX_ARCHIVE_PARSE_SECONDS = 30
MAX_PROFILE_SECONDS = 300
MAX_TAR_STREAM_BYTES = 512 * 1024**2
MAX_ZIP_CENTRAL_DIRECTORY_BYTES = 8 * 1024**2
MAX_ARCHIVE_MEMBERS = 10_000
MAX_ACQUISITION_ZIP_BYTES = 64 * 1024**2
MAX_ACQUISITION_ZIP_EXPANDED_BYTES = 512 * 1024**2
MAX_ACQUISITION_ZIP_MEMBERS = 200
MAX_SMALL_METADATA_BYTES = 1024 * 1024
MAX_ISSUES = 16
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
COMMIT_RE = re.compile(r"^[0-9a-f]{40}$")

ARCHIVES = {
    "rust": ("crate", (".crate",)),
    "python": ("python-distribution", (".whl", ".tar.gz")),
    "r": ("r-source-package", (".tar.gz",)),
    "julia": ("julia-source-archive", (".tar.gz",)),
    "typescript": ("npm-package", (".tgz",)),
    "nuget": ("nuget-package", (".nupkg",)),
    "go": ("go-source-archive", (".tar.gz",)),
}
ECOSYSTEMS = frozenset(ARCHIVES)
HELPERS = (
    "build_archive_supply_chain.py",
    "build_archive_release_manifest.py",
    "build_package_archive_bundle.py",
    "acquire_package_archive_bundle.py",
    "validate_archive_copy_provenance.py",
)
DEPENDENCY_IDS = frozenset(
    {
        "ARCHIVE-INDEX.json",
        "build-inputs/ARCHIVE-INDEX.json",
        "build-inputs/BUILD-RECEIPT.json",
        "build-inputs/acquisition.json",
        *(f"packaging/scripts/{name}" for name in HELPERS),
        "tool:syft",
        "schema:spdx-2.3",
    }
)
FIXED_EVIDENCE_PATHS = frozenset(
    {
        "RELEASE.txt",
        "SBOM-COVERAGE.json",
        "SHA256SUMS",
        "SUPPLY-CHAIN-SHA256SUMS",
        "expected-inputs.json",
        "provenance.json",
        "release-artifact-manifest.json",
        "sbom.spdx.json",
        "validation-result.json",
        "build-inputs/ARCHIVE-INDEX.json",
        "build-inputs/BUILD-RECEIPT.json",
        "build-inputs/acquisition.json",
    }
)
EXPECTED_BINDING_FIELDS = frozenset(
    {
        "schema_version",
        "repository",
        "source_commit",
        "producer_pr_head",
        "producer_tree",
        "original_run_id",
        "acquisition_artifact_id",
        "archive_zip_sha256",
        "archive_zip_bytes",
        "spdx_schema_sha256",
    }
)
EXPECTED_INPUT_FIELDS = frozenset(
    {"archive_index_sha256", "source_commit", "original_run_id", "acquisition_artifact_id", "dependencies"}
)
RELEASE_CLAIM_SCOPE = "local consistency only; unsigned and untrusted builder; no SLSA level or release acceptance"


class VerificationFailure(Exception):
    def __init__(self, code: str, path: str):
        super().__init__(code)
        self.code = code[:80]
        self.path = path[:256]


class ProfileTimeout(Exception):
    pass


@contextmanager
def profile_deadline(seconds: int):
    """Hard wall-clock guard for the CLI process on supported POSIX hosts."""
    if not hasattr(signal, "setitimer") or not hasattr(signal, "ITIMER_REAL"):
        fail("profile_deadline_unavailable", "$")
    previous_handler = signal.getsignal(signal.SIGALRM)
    previous_timer = signal.getitimer(signal.ITIMER_REAL)
    def expired(_signum: int, _frame: Any) -> None:
        raise ProfileTimeout
    started = time.monotonic()
    signal.signal(signal.SIGALRM, expired)
    signal.setitimer(signal.ITIMER_REAL, seconds)
    try:
        yield
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
        signal.signal(signal.SIGALRM, previous_handler)
        if previous_timer[0] > 0:
            remaining = max(0.001, previous_timer[0] - (time.monotonic() - started))
            signal.setitimer(signal.ITIMER_REAL, remaining, previous_timer[1])


def fail(code: str, path: str) -> None:
    raise VerificationFailure(code, path)


def _strict_pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("duplicate JSON object key")
        result[key] = value
    return result


def _reject_constant(_: str) -> None:
    raise ValueError("non-JSON constant")


def _finite_float(value: str) -> float:
    parsed = float(value)
    if not math.isfinite(parsed):
        raise ValueError("non-finite number")
    return parsed


def strict_json_bytes(data: bytes, label: str) -> Any:
    if len(data) > MAX_JSON_BYTES:
        fail("unsafe_or_oversized_json", label)
    try:
        parsed = json.loads(data.decode("utf-8"), object_pairs_hook=_strict_pairs,
                           parse_constant=_reject_constant, parse_float=_finite_float)
    except Exception:
        fail("invalid_json", label)
    pending = [(parsed, 0)]
    while pending:
        item, depth = pending.pop()
        if depth > MAX_JSON_DEPTH:
            fail("json_depth", label)
        if isinstance(item, dict):
            pending.extend((child, depth + 1) for child in item.values())
        elif isinstance(item, list):
            pending.extend((child, depth + 1) for child in item)
    return parsed


def check_path_ancestry(path: Path, label: str) -> Path:
    absolute = path.absolute()
    current = Path(absolute.anchor)
    for part in absolute.parts[1:]:
        current = current / part
        try:
            info = current.lstat()
        except OSError:
            fail("missing_path", label)
        if stat.S_ISLNK(info.st_mode):
            fail("symlink_path", label)
    return absolute


def bootstrap_json(path: Path, label: str) -> tuple[Any, bytes]:
    """Small stdlib bootstrap parser used before trusting/importing local helpers."""
    try:
        with secure_open(path, label) as stream:
            data = stream.read(MAX_JSON_BYTES + 1)
        if len(data) > MAX_JSON_BYTES:
            fail("unsafe_or_oversized_json", label)
        parsed = strict_json_bytes(data, label)
        return parsed, data
    except VerificationFailure:
        raise
    except Exception:
        fail("invalid_json", label)


@contextmanager
def secure_open(path: Path, label: str):
    """Open through directory descriptors so a concurrent path swap cannot redirect reads."""
    absolute = path.absolute()
    parts = absolute.parts[1:]
    directory_fd = -1
    descriptor = -1
    try:
        nofollow = getattr(os, "O_NOFOLLOW", 0)
        directory_fd = os.open(absolute.anchor, os.O_RDONLY | getattr(os, "O_DIRECTORY", 0))
        for part in parts[:-1]:
            next_fd = os.open(part, os.O_RDONLY | getattr(os, "O_DIRECTORY", 0) | nofollow, dir_fd=directory_fd)
            os.close(directory_fd)
            directory_fd = next_fd
        descriptor = os.open(parts[-1], os.O_RDONLY | nofollow | getattr(os, "O_NONBLOCK", 0), dir_fd=directory_fd)
        info = os.fstat(descriptor)
        if not stat.S_ISREG(info.st_mode) or info.st_nlink > 1:
            fail("not_regular_file", label)
        with os.fdopen(descriptor, "rb") as stream:
            descriptor = -1
            yield stream
    except VerificationFailure:
        raise
    except OSError:
        fail("unreadable_file", label)
    finally:
        if descriptor >= 0:
            os.close(descriptor)
        if directory_fd >= 0:
            os.close(directory_fd)


def read_file_bounded(path: Path, limit: int, label: str) -> bytes:
    try:
        with secure_open(path, label) as stream:
            data = stream.read(limit + 1)
            if len(data) > limit:
                fail("byte_limit", label)
            return data
    except VerificationFailure:
        raise


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def hash_stream(stream: BinaryIO, limit: int, label: str) -> tuple[str, int]:
    digest = hashlib.sha256()
    count = 0
    while True:
        chunk = stream.read(1024 * 1024)
        if not chunk:
            break
        count += len(chunk)
        if count > limit:
            fail("byte_limit", label)
        digest.update(chunk)
    return digest.hexdigest(), count


class BoundedReader:
    """Cap decompressed tar bytes and elapsed parser time, including tar metadata."""
    def __init__(self, stream: BinaryIO, limit: int, seconds: int, label: str):
        self.stream = stream
        self.limit = limit
        self.deadline = time.monotonic() + seconds
        self.label = label
        self.count = 0

    def read(self, size: int = -1) -> bytes:
        if time.monotonic() > self.deadline:
            fail("archive_parse_deadline", self.label)
        request = 64 * 1024 if size < 0 else min(size, 64 * 1024)
        data = self.stream.read(request)
        self.count += len(data)
        if self.count > self.limit:
            fail("archive_stream_limit", self.label)
        if time.monotonic() > self.deadline:
            fail("archive_parse_deadline", self.label)
        return data

    def close(self) -> None:
        # The surrounding gzip context owns the underlying descriptor.
        return None


def preflight_zip(stream: BinaryIO, label: str) -> None:
    """Bound ZIP directory allocation before ZipFile reads its central directory."""
    try:
        stream.seek(0, os.SEEK_END)
        size = stream.tell()
        tail_size = min(size, 22 + 65535)
        stream.seek(size - tail_size)
        tail = stream.read(tail_size)
        eocd = tail.rfind(b"PK\x05\x06")
        if eocd < 0 or eocd + 22 > len(tail):
            fail("zip_directory_missing", label)
        signature, disk, central_disk, disk_entries, entries, directory_bytes, directory_offset, comment_size = struct.unpack_from("<4s4H2IH", tail, eocd)
        eocd_absolute = size - tail_size + eocd
        if signature != b"PK\x05\x06" or eocd + 22 + comment_size != len(tail) or disk != 0 or central_disk != 0 or disk_entries != entries:
            fail("zip_directory_shape", label)
        if disk_entries == 0xFFFF or entries == 0xFFFF or directory_bytes == 0xFFFFFFFF or directory_offset == 0xFFFFFFFF:
            fail("zip64_unsupported", label)
        if eocd >= 20 and tail[eocd - 20:eocd - 16] == b"PK\x06\x07":
            fail("zip64_unsupported", label)
        if entries > MAX_ARCHIVE_MEMBERS or directory_bytes > MAX_ZIP_CENTRAL_DIRECTORY_BYTES:
            fail("zip_directory_limit", label)
        if directory_offset + directory_bytes != eocd_absolute:
            fail("zip_directory_bounds", label)
        stream.seek(0)
    except VerificationFailure:
        raise
    except Exception:
        fail("zip_directory_invalid", label)


def require_digest(value: object, path: str) -> str:
    if not isinstance(value, str) or not SHA256_RE.fullmatch(value):
        fail("invalid_sha256", path)
    return value


def require_positive_int(value: object, path: str) -> int:
    if type(value) is not int or value <= 0:
        fail("invalid_positive_integer", path)
    return value


def safe_relative(value: object, label: str) -> str:
    if not isinstance(value, str) or not value or "\\" in value or ":" in value:
        fail("unsafe_relative_path", label)
    path = PurePosixPath(value)
    if path.is_absolute() or path.as_posix() != value or any(part in ("", ".", "..") for part in path.parts):
        fail("unsafe_relative_path", label)
    return value


def contained_file(root: Path, relative: str, label: str) -> Path:
    relative = safe_relative(relative, label)
    check_path_ancestry(root, label)
    path = root.joinpath(*PurePosixPath(relative).parts)
    current = root
    if root.is_symlink() or not root.is_dir():
        fail("unsafe_root", label)
    for part in PurePosixPath(relative).parts:
        current = current / part
        try:
            mode = current.lstat().st_mode
        except OSError:
            fail("missing_file", label)
        if stat.S_ISLNK(mode):
            fail("symlink_path", label)
    try:
        mode = path.lstat().st_mode
    except OSError:
        fail("missing_file", label)
    if not stat.S_ISREG(mode) or path.stat().st_nlink > 1:
        fail("not_regular_file", label)
    if not path.resolve().is_relative_to(root.resolve()):
        fail("escaped_path", label)
    return path


def hash_file(path: Path, limit: int, label: str) -> tuple[str, int]:
    with secure_open(path, label) as stream:
        return hash_stream(stream, limit, label)


def load_json(loader: Any, path: Path, label: str) -> tuple[Any, bytes]:
    try:
        data = read_file_bounded(path, MAX_JSON_BYTES, label)
        strict_value = strict_json_bytes(data, label)
        value = loader.load_json_bytes(data, label, MAX_JSON_BYTES)
        if value != strict_value:
            fail("bounded_loader_disagreement", label)
    except Exception:
        fail("invalid_json", label)
    return value, data


def parse_json_bytes(loader: Any, data: bytes, label: str) -> tuple[Any, bytes]:
    try:
        strict_value = strict_json_bytes(data, label)
        value = loader.load_json_bytes(data, label, MAX_JSON_BYTES)
        if value != strict_value:
            fail("bounded_loader_disagreement", label)
        return value, data
    except Exception:
        fail("invalid_json", label)


def load_verified_module(source: bytes, module_name: str, filename: str) -> Any:
    """Execute the exact bytes previously hashed, never reopening the source path."""
    import types

    module = types.ModuleType(module_name)
    module.__file__ = filename
    module.__package__ = ""
    sys.modules[module_name] = module
    try:
        code = compile(source, filename, "exec", dont_inherit=True)
        exec(code, module.__dict__)
    except Exception:
        sys.modules.pop(module_name, None)
        fail("trusted_helper_import_failed", filename)
    return module


def exact_keys(value: object, expected: frozenset[str], label: str) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != set(expected):
        fail("object_fields", label)
    return value


def scan_inventory(root: Path) -> tuple[dict[str, Path], set[str]]:
    if root.is_symlink() or not root.is_dir():
        fail("unsafe_root", str(root))
    result: dict[str, Path] = {}
    directories: set[str] = set()
    folded: set[str] = set()
    total = 0
    for directory, dirnames, filenames in os.walk(root, topdown=True, followlinks=False):
        base = Path(directory)
        for name in list(dirnames):
            candidate = base / name
            rel = candidate.relative_to(root).as_posix()
            safe_relative(rel, rel)
            key = rel.casefold()
            if key in folded:
                fail("path_case_collision", rel)
            folded.add(key)
            if candidate.is_symlink() or not candidate.is_dir():
                fail("symlink_path", candidate.relative_to(root).as_posix())
            directories.add(rel)
            if len(result) + len(directories) > MAX_FILES:
                fail("inventory_limit", rel)
        for name in filenames:
            candidate = base / name
            rel = candidate.relative_to(root).as_posix()
            safe_relative(rel, rel)
            key = rel.casefold()
            if key in folded:
                fail("path_case_collision", rel)
            folded.add(key)
            try:
                info = candidate.lstat()
            except OSError:
                fail("unreadable_file", rel)
            if stat.S_ISLNK(info.st_mode):
                fail("symlink_path", rel)
            if not stat.S_ISREG(info.st_mode) or info.st_nlink > 1:
                fail("not_regular_file", rel)
            total += info.st_size
            if len(result) >= MAX_FILES or total > MAX_TOTAL_BYTES:
                fail("inventory_limit", rel)
            result[rel] = candidate
    return result, directories


def validate_binding(value: object) -> dict[str, Any]:
    binding = exact_keys(value, EXPECTED_BINDING_FIELDS, "$expected_binding")
    if type(binding["schema_version"]) is not int or binding["schema_version"] != 1:
        fail("binding_schema_version", "$expected_binding.schema_version")
    if binding["repository"] != "edithatogo/kairos":
        fail("binding_repository", "$expected_binding.repository")
    for field in ("source_commit", "producer_pr_head", "producer_tree"):
        if not isinstance(binding[field], str) or not COMMIT_RE.fullmatch(binding[field]):
            fail("invalid_commit", "$expected_binding." + field)
    require_positive_int(binding["original_run_id"], "$expected_binding.original_run_id")
    require_positive_int(binding["acquisition_artifact_id"], "$expected_binding.acquisition_artifact_id")
    require_digest(binding["archive_zip_sha256"], "$expected_binding.archive_zip_sha256")
    if type(binding["archive_zip_bytes"]) is not int or binding["archive_zip_bytes"] <= 0:
        fail("invalid_byte_count", "$expected_binding.archive_zip_bytes")
    require_digest(binding["spdx_schema_sha256"], "$expected_binding.spdx_schema_sha256")
    return binding


def validate_expected_inputs(value: object, binding: dict[str, Any]) -> dict[str, Any]:
    expected = exact_keys(value, EXPECTED_INPUT_FIELDS, "$expected_inputs")
    if not isinstance(expected["archive_index_sha256"], str) or not SHA256_RE.fullmatch(expected["archive_index_sha256"]):
        fail("expected_inputs_invalid", "$expected_inputs.archive_index_sha256")
    if not isinstance(expected["source_commit"], str) or not COMMIT_RE.fullmatch(expected["source_commit"]):
        fail("expected_inputs_invalid", "$expected_inputs.source_commit")
    require_positive_int(expected["original_run_id"], "$expected_inputs.original_run_id")
    require_positive_int(expected["acquisition_artifact_id"], "$expected_inputs.acquisition_artifact_id")
    if not isinstance(expected["dependencies"], list) or not expected["dependencies"]:
        fail("expected_inputs_invalid", "$expected_inputs.dependencies")
    for item in expected["dependencies"]:
        if not isinstance(item, dict) or set(item) != {"id", "sha256"} or not isinstance(item["id"], str) or not item["id"]:
            fail("expected_inputs_invalid", "$expected_inputs.dependencies")
        require_digest(item["sha256"], "$expected_inputs.dependencies.sha256")
    if expected["source_commit"] != binding["source_commit"]:
        fail("source_commit_mismatch", "$expected_inputs.source_commit")
    if expected["original_run_id"] != binding["original_run_id"]:
        fail("run_id_mismatch", "$expected_inputs.original_run_id")
    if expected["acquisition_artifact_id"] != binding["acquisition_artifact_id"]:
        fail("artifact_id_mismatch", "$expected_inputs.acquisition_artifact_id")
    deps = expected["dependencies"]
    ids = [item["id"] for item in deps]
    if len(set(ids)) != len(ids):
        fail("dependency_duplicates", "$expected_inputs.dependencies")
    required = set(DEPENDENCY_IDS) | {f"https://github.com/edithatogo/kairos/actions/runs/{binding['original_run_id']}"}
    if set(ids) != required:
        fail("dependency_set", "$expected_inputs.dependencies")
    by_id = {item["id"]: item["sha256"] for item in deps}
    if by_id["schema:spdx-2.3"] != binding["spdx_schema_sha256"]:
        fail("schema_pin_mismatch", "$expected_inputs.dependencies")
    if by_id[f"https://github.com/edithatogo/kairos/actions/runs/{binding['original_run_id']}"] != binding["archive_zip_sha256"]:
        fail("archive_zip_pin_mismatch", "$expected_inputs.dependencies")
    if expected["archive_index_sha256"] != by_id["ARCHIVE-INDEX.json"] or expected["archive_index_sha256"] != by_id["build-inputs/ARCHIVE-INDEX.json"]:
        fail("index_dependency_mismatch", "$expected_inputs.dependencies")
    return expected


def validate_builder_receipt(index: object, receipt: object, source_commit: str) -> list[dict[str, Any]]:
    if not isinstance(index, dict) or set(index) != {"schema_version", "source_commit", "created_at_utc", "artifacts"}:
        fail("archive_index_shape", "ARCHIVE-INDEX.json")
    if type(index["schema_version"]) is not int or index["schema_version"] != 1 or index["source_commit"] != source_commit:
        fail("archive_index_identity", "ARCHIVE-INDEX.json")
    if not isinstance(index["created_at_utc"], str) or not isinstance(index["artifacts"], list) or not index["artifacts"]:
        fail("archive_index_shape", "ARCHIVE-INDEX.json")
    if not isinstance(receipt, dict) or set(receipt) != {"source_commit", "ecosystems"} or receipt["source_commit"] != source_commit:
        fail("build_receipt_shape", "BUILD-RECEIPT.json")
    ecosystem_receipts = receipt["ecosystems"]
    if not isinstance(ecosystem_receipts, dict) or set(ecosystem_receipts) != set(ECOSYSTEMS):
        fail("build_receipt_ecosystems", "BUILD-RECEIPT.json")
    for ecosystem, row in ecosystem_receipts.items():
        if not isinstance(row, dict) or row.get("ecosystem") != ecosystem or row.get("source_commit") != source_commit:
            fail("build_receipt_identity", "BUILD-RECEIPT.json")
        if type(row.get("exit_status")) is not int or row["exit_status"] != 0:
            fail("build_receipt_exit", "BUILD-RECEIPT.json")
        if any(not isinstance(row.get(k), str) or not row[k].strip() for k in ("command", "toolchain", "platform")):
            fail("build_receipt_fields", "BUILD-RECEIPT.json")
    if set(ecosystem_receipts) != set(ECOSYSTEMS):
        fail("build_receipt_ecosystems", "BUILD-RECEIPT.json")
    rows: list[dict[str, Any]] = []
    seen: set[str] = set()
    for pos, item in enumerate(index["artifacts"]):
        label = f"ARCHIVE-INDEX.json.artifacts[{pos}]"
        if not isinstance(item, dict) or set(item) != {"ecosystem", "kind", "path", "bytes", "sha256", "builder"}:
            fail("archive_row_shape", label)
        ecosystem = item["ecosystem"]
        if ecosystem not in ARCHIVES or item["kind"] != ARCHIVES[ecosystem][0]:
            fail("archive_kind", label)
        relative = safe_relative(item["path"], label + ".path")
        if PurePosixPath(relative).parts[0] != ecosystem:
            fail("archive_ecosystem_path", label + ".path")
        if not any(relative.endswith(ext) for ext in ARCHIVES[ecosystem][1]):
            fail("archive_extension", label + ".path")
        if relative in seen or relative.casefold() in {v.casefold() for v in seen}:
            fail("duplicate_archive_path", label + ".path")
        seen.add(relative)
        require_digest(item["sha256"], label + ".sha256")
        if type(item["bytes"]) is not int or item["bytes"] <= 0:
            fail("archive_bytes", label + ".bytes")
        if item["builder"] != ecosystem_receipts.get(ecosystem):
            fail("archive_builder_mismatch", label + ".builder")
        rows.append(item)
    if {item["ecosystem"] for item in rows} != set(ECOSYSTEMS):
        fail("archive_ecosystems", "ARCHIVE-INDEX.json.artifacts")
    if [r["path"] for r in rows] != sorted(r["path"] for r in rows):
        fail("archive_order", "ARCHIVE-INDEX.json.artifacts")
    return rows


def stream_zip_member(archive: zipfile.ZipFile, info: zipfile.ZipInfo, limit: int, label: str, capture: bool = False) -> tuple[str, int, bytes | None]:
    buf = bytearray() if capture else None
    try:
        with archive.open(info, "r") as stream:
            digest = hashlib.sha256()
            total = 0
            while True:
                chunk = stream.read(1024 * 1024)
                if not chunk:
                    break
                total += len(chunk)
                if total > limit:
                    fail("archive_member_limit", label)
                digest.update(chunk)
                if buf is not None:
                    if len(buf) + len(chunk) > MAX_SMALL_METADATA_BYTES:
                        fail("metadata_limit", label)
                    buf.extend(chunk)
    except VerificationFailure:
        raise
    except Exception:
        fail("archive_corrupt", label)
    if total != info.file_size:
        fail("archive_member_size", label)
    return digest.hexdigest(), total, bytes(buf) if buf is not None else None


def zip_members(
    archive: zipfile.ZipFile,
    label: str,
    max_total: int = MAX_ARCHIVE_BYTES,
    max_members: int = MAX_ARCHIVE_MEMBERS,
) -> dict[str, zipfile.ZipInfo]:
    members: dict[str, zipfile.ZipInfo] = {}
    folded: set[str] = set()
    total = 0
    infos = archive.infolist()
    if len(infos) > max_members:
        fail("archive_member_count", label)
    for info in infos:
        name = info.filename
        rel = safe_relative(name.rstrip("/") if info.is_dir() else name, label)
        if info.flag_bits & 1:
            fail("archive_encrypted", label)
        mode = info.external_attr >> 16
        kind = stat.S_IFMT(mode)
        if kind not in (0, stat.S_IFREG, stat.S_IFDIR) or (info.is_dir() and kind == stat.S_IFREG) or (not info.is_dir() and kind == stat.S_IFDIR):
            fail("archive_special_member", label)
        key = rel.casefold()
        if key in folded:
            fail("archive_duplicate_member", label)
        folded.add(key)
        if info.file_size < 0 or info.file_size > max_total:
            fail("archive_member_limit", label)
        total += info.file_size
        if total > max_total:
            fail("archive_uncompressed_limit", label)
        if not info.is_dir():
            members[rel] = info
    return members


def validate_package_archive(path: Path, row: dict[str, Any], label: str, metadata_path: str | None = None) -> tuple[set[str], dict[str, bytes]]:
    """Read each archive member under strict budgets; return names and bounded identity candidates."""
    names: set[str] = set()
    payloads: dict[str, bytes] = {}
    if row["bytes"] > MAX_ARCHIVE_BYTES:
        fail("archive_file_limit", label)
    if path.suffix in {".whl", ".nupkg"}:
        try:
            with secure_open(path, label) as archive_stream:
                preflight_zip(archive_stream, label)
                with zipfile.ZipFile(archive_stream) as archive:
                    members = zip_members(archive, label)
                    for name, info in members.items():
                        names.add(name)
                        digest, size, data = stream_zip_member(archive, info, MAX_ARCHIVE_BYTES, label, capture=name == metadata_path)
                        del digest, size
                        if data is not None:
                            payloads[name] = data
        except VerificationFailure:
            raise
        except Exception:
            fail("archive_corrupt", label)
    else:
        try:
            with secure_open(path, label) as archive_stream:
                with gzip.GzipFile(fileobj=archive_stream, mode="rb") as decompressed:
                    bounded = BoundedReader(decompressed, MAX_TAR_STREAM_BYTES, MAX_ARCHIVE_PARSE_SECONDS, label)
                    with tarfile.open(fileobj=bounded, mode="r|") as archive:
                        count = 0
                        total = 0
                        for member in archive:
                            count += 1
                            if count > MAX_ARCHIVE_MEMBERS:
                                fail("archive_member_count", label)
                            raw = member.name.rstrip("/") if member.isdir() else member.name
                            name = safe_relative(raw, label)
                            key = name.casefold()
                            if key in {existing.casefold() for existing in names}:
                                fail("archive_duplicate_member", label)
                            if not (member.isfile() or member.isdir()) or member.issym() or member.islnk() or member.isdev() or member.isfifo():
                                fail("archive_special_member", label)
                            if member.size < 0:
                                fail("archive_member_size", label)
                            if member.isdir():
                                if member.size != 0:
                                    fail("archive_directory_size", label)
                                names.add(name)
                                continue
                            total += member.size
                            if total > MAX_ARCHIVE_BYTES:
                                fail("archive_uncompressed_limit", label)
                            names.add(name)
                            member_stream = archive.extractfile(member)
                            if member_stream is None:
                                fail("archive_corrupt", label)
                            captured = bytearray() if name == metadata_path and member.size <= MAX_SMALL_METADATA_BYTES else None
                            read = 0
                            while True:
                                chunk = member_stream.read(64 * 1024)
                                if not chunk:
                                    break
                                read += len(chunk)
                                if read > member.size or read > MAX_ARCHIVE_BYTES:
                                    fail("archive_member_size", label)
                                if captured is not None:
                                    captured.extend(chunk)
                            if read != member.size:
                                fail("archive_member_size", label)
                            if captured is not None:
                                payloads[name] = bytes(captured)
                        while bounded.read(64 * 1024):
                            pass
        except VerificationFailure:
            raise
        except Exception:
            fail("archive_corrupt", label)
    return names, payloads


def inspect_archive_rows(bundle: Path, rows: list[dict[str, Any]]) -> dict[str, tuple[set[str], dict[str, bytes]]]:
    inspected: dict[str, tuple[set[str], dict[str, bytes]]] = {}
    for row in rows:
        rel = row["path"]
        path = contained_file(bundle, rel, "bundle/" + rel)
        digest, size = hash_file(path, MAX_ARCHIVE_BYTES, "bundle/" + rel)
        if digest != row["sha256"] or size != row["bytes"]:
            fail("archive_index_digest_mismatch", "bundle/" + rel)
        inspected[rel] = validate_package_archive(path, row, "bundle/" + rel)
    return inspected


def verify_acquisition_zip(
    archive_zip: Path,
    binding: dict[str, Any],
    bundle: Path,
    index_bytes: bytes,
    receipt_bytes: bytes,
    checksum_bytes: bytes,
    rows: list[dict[str, Any]],
) -> None:
    digest, size = hash_file(archive_zip, MAX_ACQUISITION_ZIP_BYTES, "archive_zip")
    if digest != binding["archive_zip_sha256"] or size != binding["archive_zip_bytes"]:
        fail("archive_zip_identity", "archive_zip")
    expected = {
        "ARCHIVE-INDEX.json": (sha256_bytes(index_bytes), len(index_bytes)),
        "BUILD-RECEIPT.json": (sha256_bytes(receipt_bytes), len(receipt_bytes)),
        "SHA256SUMS": (sha256_bytes(checksum_bytes), len(checksum_bytes)),
    }
    expected.update({row["path"]: (row["sha256"], row["bytes"]) for row in rows})
    try:
        with secure_open(archive_zip, "archive_zip") as zip_stream:
            preflight_zip(zip_stream, "archive_zip")
            with zipfile.ZipFile(zip_stream) as archive:
                members = zip_members(archive, "archive_zip", MAX_ACQUISITION_ZIP_EXPANDED_BYTES, MAX_ACQUISITION_ZIP_MEMBERS)
                if set(members) != set(expected):
                    fail("archive_zip_inventory", "archive_zip")
                for name, info in members.items():
                    wanted_digest, wanted_size = expected[name]
                    actual_digest, actual_size, _ = stream_zip_member(archive, info, MAX_ACQUISITION_ZIP_EXPANDED_BYTES, "archive_zip/" + name)
                    if actual_digest != wanted_digest or actual_size != wanted_size:
                        fail("archive_zip_member_mismatch", "archive_zip/" + name)
                    bundle_path = contained_file(bundle, name, "bundle/" + name)
                    bundle_digest, bundle_size = hash_file(bundle_path, MAX_ARCHIVE_BYTES, "bundle/" + name)
                    if (bundle_digest, bundle_size) != (actual_digest, actual_size):
                        fail("archive_bundle_copy_mismatch", "archive_zip/" + name)
    except VerificationFailure:
        raise
    except Exception:
        fail("archive_zip_corrupt", "archive_zip")


def parse_checksums(data: bytes, label: str) -> dict[str, str]:
    try:
        text = data.decode("ascii")
    except UnicodeDecodeError:
        fail("checksum_encoding", label)
    if not text.endswith("\n"):
        fail("checksum_terminator", label)
    entries: dict[str, str] = {}
    for line in text.splitlines():
        if len(line) < 67 or line[64:66] != "  ":
            fail("checksum_line", label)
        digest, path = line[:64], line[66:]
        require_digest(digest, label)
        path = safe_relative(path, label)
        if path in entries or path.casefold() in {p.casefold() for p in entries}:
            fail("checksum_duplicate", label)
        entries[path] = digest
    return entries


def validate_acquisition_records(acquisition: Path, binding: dict[str, Any], rows: list[dict[str, Any]], index_sha: str, helper: Any, acquisition_helper: Any) -> dict[str, Any]:
    files = {}
    hashes = {}
    for name in ("receipt.json", "artifact-metadata.json", "source-commit-readback.json"):
        path = contained_file(acquisition, name, name)
        files[name], data = load_json(helper, path, name)
        hashes[name] = sha256_bytes(data)
    receipt = files["receipt.json"]
    metadata = files["artifact-metadata.json"]
    readback = files["source-commit-readback.json"]
    aid = binding["acquisition_artifact_id"]
    run_id = binding["original_run_id"]
    commit = binding["source_commit"]
    pr_head = binding["producer_pr_head"]
    tree = binding["producer_tree"]
    if not isinstance(receipt, dict) or receipt.get("exit_status") != 0:
        fail("acquisition_receipt_status", "receipt.json")
    for key, value in (("artifact_id", aid), ("workflow_run", run_id), ("producer_checkout_source", commit), ("producer_pr_head", pr_head), ("producer_tree", tree), ("archive_zip_sha256", binding["archive_zip_sha256"]), ("actual_archive_count", len(rows))):
        if receipt.get(key) != value:
            fail("acquisition_receipt_binding", "receipt.json")
    if sorted(receipt.get("ecosystems", [])) != sorted(ECOSYSTEMS):
        fail("acquisition_receipt_ecosystems", "receipt.json")
    if not isinstance(metadata, dict) or metadata.get("id") != aid or metadata.get("size_in_bytes") != binding["archive_zip_bytes"] or metadata.get("digest") != "sha256:" + binding["archive_zip_sha256"]:
        fail("artifact_metadata_binding", "artifact-metadata.json")
    run = metadata.get("workflow_run")
    if not isinstance(run, dict) or run.get("id") != run_id or run.get("head_sha") != pr_head:
        fail("artifact_workflow_binding", "artifact-metadata.json")
    if not isinstance(readback, dict) or set(readback) != {commit} or not isinstance(readback[commit], dict):
        fail("source_readback_missing", "source-commit-readback.json")
    commit_data = readback[commit]
    verification = commit_data.get("verification")
    if commit_data.get("sha") != commit or not isinstance(commit_data.get("tree"), dict) or commit_data["tree"].get("sha") != tree:
        fail("source_readback_binding", "source-commit-readback.json")
    parents = commit_data.get("parents")
    if not isinstance(parents, list) or not parents or any(
        not isinstance(parent, dict)
        or set(parent) != {"sha", "url", "html_url"}
        or not isinstance(parent.get("sha"), str)
        or not COMMIT_RE.fullmatch(parent["sha"])
        or parent.get("url") != f"https://api.github.com/repos/edithatogo/kairos/git/commits/{parent['sha']}"
        or parent.get("html_url") != f"https://github.com/edithatogo/kairos/commit/{parent['sha']}"
        for parent in parents
    ) or commit in {parent["sha"] for parent in parents} or len({parent["sha"] for parent in parents}) != len(parents):
        fail("source_readback_parent", "source-commit-readback.json")
    if not isinstance(verification, dict) or verification.get("verified") is not True or verification.get("reason") != "valid":
        fail("source_readback_verification", "source-commit-readback.json")
    if receipt.get("scope") != "Actual retained archive structural/source acquisition verification; no producer SLSA attestation, release or registry acceptance":
        fail("acquisition_receipt_scope", "receipt.json")
    compact_path = acquisition / "acquisition.json"
    compact_receipt = None
    compact_bytes = None
    if compact_path.exists() or compact_path.is_symlink():
        compact_receipt, compact_bytes = load_json(helper, contained_file(acquisition, "acquisition.json", "acquisition.json"), "acquisition.json")
    mode = compact_receipt.get("selection_policy") if isinstance(compact_receipt, dict) else None
    main_mode = mode == "same-repository-main-workflow-dispatch"
    main_names = ("run-metadata.json", "branch-main-readback.json")
    if main_mode:
        files["acquisition.json"] = compact_receipt
        hashes["acquisition.json"] = sha256_bytes(compact_bytes)
        raw_path = contained_file(acquisition, "source-commit-api-readback.json", "source-commit-api-readback.json")
        raw_map, raw_bytes = load_json(helper, raw_path, "source-commit-api-readback.json")
        hashes["source-commit-api-readback.json"] = sha256_bytes(raw_bytes)
        raw = raw_map.get(commit) if isinstance(raw_map, dict) and set(raw_map) == {commit} else None
        raw_parents = raw.get("parents") if isinstance(raw, dict) else None
        raw_tree = raw.get("tree") if isinstance(raw, dict) else None
        raw_verification = raw.get("verification") if isinstance(raw, dict) else None
        projected_parents = ([{key: parent.get(key) for key in ("sha", "url", "html_url")}
                              for parent in raw_parents]
                             if isinstance(raw_parents, list) and all(isinstance(parent, dict) for parent in raw_parents)
                             else None)
        if (not isinstance(raw, dict) or raw.get("sha") != commit
                or not isinstance(raw_tree, dict) or raw_tree.get("sha") != tree
                or projected_parents != parents or not isinstance(raw_verification, dict)
                or raw_verification.get("verified") is not True or raw_verification.get("reason") != "valid"):
            fail("source_api_readback_mismatch", "source-commit-api-readback.json")
        for name in main_names:
            path = contained_file(acquisition, name, name)
            files[name], data = load_json(helper, path, name)
            hashes[name] = sha256_bytes(data)
        run_record = files["run-metadata.json"]
        try:
            run_source = acquisition_helper.validate_main_dispatch_run(run_record, run_id)
        except Exception:
            fail("main_dispatch_admission", "run-metadata.json")
        if run_source != commit or pr_head != run_source or metadata.get("workflow_run", {}).get("head_sha") != run_source:
            fail("main_dispatch_source_binding", "run-metadata.json")
        workflow_run = metadata.get("workflow_run")
        run_repository = run_record.get("repository")
        head_repository = run_record.get("head_repository")
        if (metadata.get("name") != "kairos-actual-package-archives-" + commit
                or metadata.get("expired") is not False
                or type(metadata.get("id")) is not int
                or type(metadata.get("size_in_bytes")) is not int
                or not isinstance(workflow_run, dict)
                or type(workflow_run.get("id")) is not int
                or type(workflow_run.get("repository_id")) is not int
                or type(workflow_run.get("head_repository_id")) is not int
                or workflow_run.get("repository_id") != run_repository.get("id")
                or workflow_run.get("head_repository_id") != head_repository.get("id")
                or workflow_run.get("head_branch") != "main"):
            fail("main_artifact_metadata_binding", "artifact-metadata.json")
        acquisition_receipt = files["acquisition.json"]
        compact_parents = acquisition_receipt.get("build_commit_parents") if isinstance(acquisition_receipt, dict) else None
        compact_parent_projection = ([{key: parent.get(key) for key in ("sha", "url", "html_url")}
                                      for parent in compact_parents]
                                     if isinstance(compact_parents, list) and all(isinstance(parent, dict) for parent in compact_parents)
                                     else None)
        if (not isinstance(acquisition_receipt, dict)
                or acquisition_receipt.get("repository") != "edithatogo/kairos"
                or acquisition_receipt.get("run_id") != run_id
                or acquisition_receipt.get("source_commit") != commit
                or acquisition_receipt.get("head_commit") != run_source
                or acquisition_receipt.get("artifact_id") != aid
                or acquisition_receipt.get("artifact_digest") != "sha256:" + binding["archive_zip_sha256"]
                or acquisition_receipt.get("selection_policy") != mode
                or acquisition_receipt.get("archive_index_sha256") != index_sha
                or acquisition_receipt.get("scope") != "verified acquisition; not original build provenance or release acceptance"
                or compact_parent_projection != parents):
            fail("main_acquisition_binding", "acquisition.json")
        branch = files["branch-main-readback.json"]
        observed_main = branch.get("commit", {}).get("sha") if isinstance(branch, dict) and isinstance(branch.get("commit"), dict) else None
        if not isinstance(observed_main, str) or not COMMIT_RE.fullmatch(observed_main):
            fail("main_branch_readback", "branch-main-readback.json")
        comparison_path = acquisition / "compare-main-readback.json"
        comparison = None
        if observed_main == commit:
            if comparison_path.exists() or comparison_path.is_symlink():
                fail("main_compare_unexpected", "compare-main-readback.json")
        else:
            comparison, data = load_json(helper, contained_file(acquisition, "compare-main-readback.json", "compare-main-readback.json"), "compare-main-readback.json")
            hashes["compare-main-readback.json"] = sha256_bytes(data)
            if not isinstance(comparison, dict) or not isinstance(comparison.get("head_commit"), dict) or comparison["head_commit"].get("sha") != observed_main:
                fail("main_compare_head_binding", "compare-main-readback.json")
        try:
            ancestry = acquisition_helper.validate_main_ancestry(commit, branch, comparison)
        except Exception:
            fail("main_ancestry_admission", "branch-main-readback.json")
        if acquisition_receipt.get("main_ancestry") != ancestry:
            fail("main_ancestry_binding", "acquisition.json")
        if receipt.get("selection_policy") != mode or receipt.get("main_ancestry") != ancestry:
            fail("outer_main_ancestry_binding", "receipt.json")
    elif mode is not None:
        fail("acquisition_selection_policy", "receipt.json")
    elif pr_head not in {parent["sha"] for parent in parents}:
        fail("source_readback_parent", "source-commit-readback.json")
    return {"receipt": receipt, "metadata": metadata, "readback": commit_data, "hashes": hashes, "main_mode": main_mode}


def build_expected_inventory(rows: list[dict[str, Any]]) -> set[str]:
    expected = set(FIXED_EVIDENCE_PATHS)
    for row in rows:
        identifier = hashlib.sha256(row["path"].encode("utf-8")).hexdigest()
        expected.add("archives/" + row["path"])
        expected.add(f"component-sboms/{identifier}.spdx.json")
        expected.add(f"component-sboms/{identifier}.stdout")
        expected.add(f"component-sboms/{identifier}.stderr")
    return expected


def validate_archive_relationships(rows: list[dict[str, Any]], component_paths: dict[str, dict[str, Any]], relationships: object) -> None:
    if not isinstance(relationships, list):
        fail("spdx_relationships", "sbom.spdx.json.relationships")
    expected: set[tuple[str, str, str]] = set()
    for row in rows:
        rel = row["path"]
        identifier = hashlib.sha256(rel.encode("utf-8")).hexdigest()
        archive_package_id = "SPDXRef-archive-" + identifier
        document_ref = "DocumentRef-" + identifier
        expected.add(("SPDXRef-DOCUMENT", "DESCRIBES", archive_package_id))
        component = component_paths[rel]
        for package in component.get("packages", []):
            if package.get("primaryPackagePurpose") != "FILE":
                expected.add((archive_package_id, "CONTAINS", document_ref + ":" + package["SPDXID"]))
    actual = [
        (item.get("spdxElementId"), item.get("relationshipType"), item.get("relatedSpdxElement"))
        for item in relationships if isinstance(item, dict)
    ]
    if len(actual) != len(relationships) or len(actual) != len(expected) or set(actual) != expected:
        fail("spdx_relationship_binding", "sbom.spdx.json.relationships")


def validate_manifest(manifest: object, rows: list[dict[str, Any]], index_sha: str, commit: str) -> list[dict[str, Any]]:
    if not isinstance(manifest, dict) or set(manifest) != {"schema_version", "release_stage", "source_commit", "production_publish_enabled", "archive_index_sha256", "artifacts"}:
        fail("manifest_shape", "release-artifact-manifest.json")
    if type(manifest["schema_version"]) is not int or manifest["schema_version"] != 1 or manifest["release_stage"] != "actual-package-archives" or manifest["production_publish_enabled"] is not False or manifest["source_commit"] != commit or manifest["archive_index_sha256"] != index_sha:
        fail("manifest_identity", "release-artifact-manifest.json")
    artifacts = manifest["artifacts"]
    if not isinstance(artifacts, list) or len(artifacts) != len(rows):
        fail("manifest_artifacts", "release-artifact-manifest.json.artifacts")
    expected = []
    for row in rows:
        expected.append({
            "path": "archives/" + row["path"],
            "sha256": row["sha256"],
            "bytes": row["bytes"],
            "ecosystem": "csharp" if row["ecosystem"] == "nuget" else row["ecosystem"],
            "archive_ecosystem": row["ecosystem"],
            "kind": row["kind"],
        })
    expected.sort(key=lambda value: value["path"])
    if artifacts != expected:
        fail("manifest_archive_binding", "release-artifact-manifest.json.artifacts")
    return expected


def validate_coverage(coverage: object, rows: list[dict[str, Any]], expected_deps: dict[str, str], binding: dict[str, Any]) -> dict[str, dict[str, Any]]:
    if not isinstance(coverage, dict) or set(coverage) != {"coverage", "source_identities", "runtime", "syft_sha256", "schema_sha256", "provenance_scope"}:
        fail("coverage_shape", "SBOM-COVERAGE.json")
    if coverage.get("source_identities") != {name: expected_deps["packaging/scripts/" + name] for name in HELPERS}:
        fail("coverage_source_pins", "SBOM-COVERAGE.json.source_identities")
    if coverage.get("syft_sha256") != expected_deps["tool:syft"] or coverage.get("schema_sha256") != binding["spdx_schema_sha256"]:
        fail("coverage_tool_schema_pin", "SBOM-COVERAGE.json")
    if not isinstance(coverage.get("runtime"), dict) or coverage.get("provenance_scope") != "Unsigned local copying/evidence build; does not claim original compilation attestation or SLSA level.":
        fail("coverage_runtime", "SBOM-COVERAGE.json")
    entries = coverage["coverage"]
    if not isinstance(entries, list) or len(entries) != len(rows):
        fail("coverage_count", "SBOM-COVERAGE.json.coverage")
    result: dict[str, dict[str, Any]] = {}
    for pos, entry in enumerate(entries):
        label = f"SBOM-COVERAGE.json.coverage[{pos}]"
        if not isinstance(entry, dict) or set(entry) != {
            "archive", "archive_sha256", "identity", "scanner_software_packages",
            "scanner_identities", "manifest_identity_fallback", "component_sbom",
            "component_sbom_sha256",
        }:
            fail("coverage_row_shape", label)
        path = safe_relative(entry.get("archive"), label + ".archive")
        if path in result:
            fail("coverage_duplicate_archive", label)
        result[path] = entry
    for row in rows:
        path = row["path"]
        entry = result.get(path)
        identifier = hashlib.sha256(path.encode("utf-8")).hexdigest()
        component = f"component-sboms/{identifier}.spdx.json"
        if entry is None or entry.get("archive_sha256") != row["sha256"] or entry.get("component_sbom") != component:
            fail("coverage_archive_binding", "SBOM-COVERAGE.json")
        require_digest(entry.get("component_sbom_sha256"), "SBOM-COVERAGE.json.component_sbom_sha256")
        identity = entry.get("identity")
        if not isinstance(identity, dict) or set(identity) != {"name", "version", "metadata_path", "metadata_sha256", "repository_commit"}:
            fail("coverage_identity_shape", "SBOM-COVERAGE.json.identity")
        if not isinstance(identity["name"], str) or not identity["name"] or (identity["version"] is not None and not isinstance(identity["version"], str)):
            fail("coverage_identity", "SBOM-COVERAGE.json.identity")
        safe_relative(identity["metadata_path"], "SBOM-COVERAGE.json.identity.metadata_path")
        require_digest(identity["metadata_sha256"], "SBOM-COVERAGE.json.identity.metadata_sha256")
        if identity["repository_commit"] is not None and identity["repository_commit"] != binding["source_commit"]:
            fail("coverage_repository_commit", "SBOM-COVERAGE.json.identity.repository_commit")
        if type(entry.get("manifest_identity_fallback")) is not bool or type(entry.get("scanner_software_packages")) is not int or entry["scanner_software_packages"] < 0 or not isinstance(entry.get("scanner_identities"), list):
            fail("coverage_scan_fields", "SBOM-COVERAGE.json")
    return result


def validate_spdx(
    evidence: Path,
    rows: list[dict[str, Any]],
    coverage_rows: dict[str, dict[str, Any]],
    manifest_rows: list[dict[str, Any]],
    schema_bytes: bytes,
    binding: dict[str, Any],
    helper: Any,
    loader: Any,
) -> tuple[dict[str, Any], dict[str, Any]]:
    schema, _ = parse_json_bytes(loader, schema_bytes, "spdx_schema")
    try:
        import jsonschema
        validator_type = jsonschema.validators.validator_for(schema)
        validator_type.check_schema(schema)
        validator = validator_type(schema, format_checker=jsonschema.FormatChecker())
    except Exception:
        fail("schema_invalid", "spdx_schema")

    namespaces: set[str] = set()
    component_by_id: dict[str, dict[str, Any]] = {}
    component_paths: dict[str, dict[str, Any]] = {}
    for row in rows:
        rel = row["path"]
        comp_rel = coverage_rows[rel]["component_sbom"]
        comp_path = contained_file(evidence, comp_rel, comp_rel)
        comp, comp_bytes = load_json(loader, comp_path, comp_rel)
        if next(validator.iter_errors(comp), None) is not None:
            fail("spdx_schema_invalid", comp_rel)
        if not isinstance(comp, dict) or comp.get("spdxVersion") != "SPDX-2.3":
            fail("spdx_version", comp_rel)
        try:
            helper.validate_component_graph(comp, namespaces)
        except Exception:
            fail("spdx_component_graph", comp_rel)
        identifier = hashlib.sha256(rel.encode("utf-8")).hexdigest()
        ext_id = "DocumentRef-" + identifier
        ids = {comp.get("SPDXID")}
        for collection in ("packages", "files", "snippets"):
            for item in comp.get(collection, []):
                if isinstance(item, dict):
                    ids.add(item.get("SPDXID"))
        component_by_id[ext_id] = {"sha256": sha256_bytes(comp_bytes), "namespace": comp.get("documentNamespace"), "ids": ids, "document": comp}
        component_paths[rel] = comp
        if coverage_rows[rel]["component_sbom_sha256"] != sha256_bytes(comp_bytes):
            fail("component_sbom_digest", comp_rel)
        scanned_packages = [p for p in comp.get("packages", []) if p.get("primaryPackagePurpose") != "FILE"]
        expected_scanner_identities = [{"name": p.get("name"), "version": p.get("versionInfo")} for p in scanned_packages]
        coverage_row = coverage_rows[rel]
        if coverage_row.get("scanner_software_packages") != len(scanned_packages) or coverage_row.get("scanner_identities") != expected_scanner_identities:
            fail("coverage_scanner_inventory", "SBOM-COVERAGE.json")
        fallback_allowed = row["ecosystem"] in ("go", "julia") and not scanned_packages
        if coverage_row.get("manifest_identity_fallback") is not fallback_allowed:
            fail("coverage_fallback_scope", "SBOM-COVERAGE.json")
        _verify_identity_metadata(row, coverage_rows[rel], evidence, rel)

    root_path = contained_file(evidence, "sbom.spdx.json", "sbom.spdx.json")
    root, _ = load_json(loader, root_path, "sbom.spdx.json")
    if next(validator.iter_errors(root), None) is not None:
        fail("spdx_schema_invalid", "sbom.spdx.json")
    if not isinstance(root, dict) or root.get("spdxVersion") != "SPDX-2.3":
        fail("spdx_version", "sbom.spdx.json")
    try:
        helper.validate_component_graph(root, namespaces, component_by_id)
    except Exception:
        fail("spdx_root_graph", "sbom.spdx.json")
    packages = root.get("packages")
    if not isinstance(packages, list) or len(packages) != len(rows):
        fail("spdx_archive_packages", "sbom.spdx.json.packages")
    package_by_path: dict[str, dict[str, Any]] = {}
    for package in packages:
        if not isinstance(package, dict):
            fail("spdx_package_shape", "sbom.spdx.json.packages")
        file_name = package.get("packageFileName")
        if not isinstance(file_name, str) or not file_name.startswith("archives/"):
            fail("spdx_package_path", "sbom.spdx.json.packages")
        rel = safe_relative(file_name, "sbom.spdx.json.packageFileName")[len("archives/"):]
        if rel in package_by_path:
            fail("spdx_duplicate_archive_package", "sbom.spdx.json.packages")
        package_by_path[rel] = package
    if set(package_by_path) != {row["path"] for row in rows}:
        fail("spdx_archive_package_set", "sbom.spdx.json.packages")
    for row, manifest in zip(rows, manifest_rows):
        rel = row["path"]
        package = package_by_path[rel]
        coverage_row = coverage_rows[rel]
        identifier = hashlib.sha256(rel.encode("utf-8")).hexdigest()
        if package.get("SPDXID") != "SPDXRef-archive-" + identifier or package.get("name") != coverage_row["identity"]["name"]:
            fail("spdx_package_identity", "sbom.spdx.json.packages")
        if package.get("versionInfo") != coverage_row["identity"]["version"] and ("versionInfo" in package or coverage_row["identity"]["version"] is not None):
            fail("spdx_package_version", "sbom.spdx.json.packages")
        if package.get("checksums") != [{"algorithm": "SHA256", "checksumValue": row["sha256"]}]:
            fail("spdx_archive_checksum", "sbom.spdx.json.packages")
        source_info = package.get("sourceInfo")
        identity = coverage_row["identity"]
        expected_source = f"Identity from packaged {identity['metadata_path']} SHA256 {identity['metadata_sha256']}"
        if source_info != expected_source:
            fail("spdx_source_identity", "sbom.spdx.json.packages")
    # Require an explicit description edge for every archive package and at least
    # one component edge per archive where the component contains packages.
    relationships = root.get("relationships")
    validate_archive_relationships(rows, component_paths, relationships)
    described = {r.get("relatedSpdxElement") for r in relationships if isinstance(r, dict) and r.get("spdxElementId") == "SPDXRef-DOCUMENT" and r.get("relationshipType") == "DESCRIBES"}
    expected_package_ids = {p["SPDXID"] for p in package_by_path.values()}
    if described != expected_package_ids:
        fail("spdx_describes_edges", "sbom.spdx.json.relationships")
    return root, component_paths


def _verify_identity_metadata(row: dict[str, Any], coverage_row: dict[str, Any], evidence: Path, archive_rel: str) -> None:
    # Hash and basic identity are bound to bytes inside the indexed archive. The
    # root/component SPDX schema and graph are validated separately.
    identity = coverage_row["identity"]
    metadata_path = identity["metadata_path"]
    names, payloads = validate_package_archive(
        contained_file(evidence, "archives/" + archive_rel, "archives/" + archive_rel), row, "archives/" + archive_rel, metadata_path
    )
    if metadata_path not in names or metadata_path not in payloads:
        fail("identity_metadata_missing", "SBOM-COVERAGE.json.identity.metadata_path")
    data = payloads[metadata_path]
    if sha256_bytes(data) != identity["metadata_sha256"]:
        fail("identity_metadata_digest", "SBOM-COVERAGE.json.identity.metadata_sha256")
    try:
        text = data.decode("utf-8")
        ecosystem = row["ecosystem"]
        name: str | None = None
        version: str | None = None
        repository_commit: str | None = None
        if ecosystem == "go":
            match = re.findall(r"^module\s+(\S+)\s*$", text, re.M)
            name = match[0] if len(match) == 1 else None
        elif ecosystem in ("julia", "rust"):
            import tomllib
            parsed = tomllib.loads(text)
            if ecosystem == "rust":
                parsed = parsed["package"]
            name, version = parsed.get("name"), parsed.get("version")
        elif ecosystem == "typescript":
            parsed = json.loads(text)
            name, version = parsed.get("name"), parsed.get("version")
        elif ecosystem == "nuget":
            import xml.etree.ElementTree as ET
            doc = ET.fromstring(text)
            fields = {n.tag.rsplit("}", 1)[-1]: n.text for n in doc.iter()}
            name, version = fields.get("id"), fields.get("version")
            repos = [n for n in doc.iter() if n.tag.rsplit("}", 1)[-1] == "repository"]
            if len(repos) == 1:
                repository_commit = repos[0].get("commit")
        elif ecosystem == "python":
            from email.parser import Parser
            fields = Parser().parsestr(text)
            name, version = fields.get("Name"), fields.get("Version")
        elif ecosystem == "r":
            found = dict(re.findall(r"^(Package|Version):\s*(.+)$", text, re.M))
            name, version = found.get("Package"), found.get("Version")
        if name != identity["name"] or version != identity["version"] or repository_commit != identity["repository_commit"]:
            fail("identity_metadata_mismatch", "SBOM-COVERAGE.json.identity")
    except VerificationFailure:
        raise
    except Exception:
        fail("identity_metadata_invalid", "SBOM-COVERAGE.json.identity.metadata_path")


def verify_profile(args: argparse.Namespace) -> dict[str, Any]:
    source_dir = Path(__file__).resolve().parent
    binding_value, binding_bytes = bootstrap_json(args.expected_binding, "expected_binding")
    binding = validate_binding(binding_value)
    expected_value, expected_bytes = bootstrap_json(args.expected_inputs, "expected_inputs")
    expected = validate_expected_inputs(expected_value, binding)
    expected_deps = {item["id"]: item["sha256"] for item in expected["dependencies"]}
    untrusted_roots = (args.evidence_dir, args.archive_bundle, args.acquisition_dir)
    for input_path, label in ((args.expected_binding, "expected_binding"), (args.expected_inputs, "expected_inputs"), (args.spdx_schema, "spdx_schema")):
        check_path_ancestry(input_path, label)
        resolved = input_path.resolve()
        if any(resolved.is_relative_to(root.resolve()) for root in untrusted_roots):
            fail("expected_input_not_independent", label)
    actual_self_hash, _ = hash_file(Path(__file__), MAX_JSON_BYTES, "verifier_source")
    if actual_self_hash != args.expected_verifier_sha256:
        fail("verifier_pin_mismatch", "verifier_source")
    schema_bytes = read_file_bounded(args.spdx_schema, MAX_JSON_BYTES, "spdx_schema")
    schema_hash = sha256_bytes(schema_bytes)
    if schema_hash != binding["spdx_schema_sha256"] or expected_deps["schema:spdx-2.3"] != schema_hash:
        fail("schema_hash_mismatch", "spdx_schema")
    verified_helper_bytes: dict[str, bytes] = {}
    for helper in HELPERS:
        path = source_dir / helper
        source_bytes = read_file_bounded(path, MAX_JSON_BYTES, "trusted_source/" + helper)
        actual = sha256_bytes(source_bytes)
        if actual != expected_deps["packaging/scripts/" + helper]:
            fail("helper_pin_mismatch", "trusted_source/" + helper)
        verified_helper_bytes[helper] = source_bytes
    # All executed helper source is pinned before importing these exact captured bytes.
    provenance = load_verified_module(
        verified_helper_bytes["validate_archive_copy_provenance.py"],
        "_archive_evidence_provenance", str(source_dir / "validate_archive_copy_provenance.py"))
    supply = load_verified_module(
        verified_helper_bytes["build_archive_supply_chain.py"],
        "_archive_evidence_supply", str(source_dir / "build_archive_supply_chain.py"))
    acquisition_helper = load_verified_module(
        verified_helper_bytes["acquire_package_archive_bundle.py"],
        "_archive_evidence_acquisition", str(source_dir / "acquire_package_archive_bundle.py"))
    if provenance.load_json_bytes(binding_bytes, "expected_binding", MAX_JSON_BYTES) != binding_value:
        fail("bootstrap_loader_disagreement", "expected_binding")
    if provenance.load_json_bytes(expected_bytes, "expected_inputs", MAX_JSON_BYTES) != expected_value:
        fail("bootstrap_loader_disagreement", "expected_inputs")

    evidence_inventory, evidence_directories = scan_inventory(args.evidence_dir)
    bundle_inventory, bundle_directories = scan_inventory(args.archive_bundle)
    expected_inner, expected_inner_bytes = load_json(provenance, contained_file(args.evidence_dir, "expected-inputs.json", "expected-inputs.json"), "bundled_expected_inputs")
    if expected_inner != expected:
        fail("bundled_expected_inputs_mismatch", "expected-inputs.json")
    expected_input_bytes_hash = sha256_bytes(expected_inner_bytes)

    index_path = contained_file(args.archive_bundle, "ARCHIVE-INDEX.json", "bundle/ARCHIVE-INDEX.json")
    receipt_path = contained_file(args.archive_bundle, "BUILD-RECEIPT.json", "bundle/BUILD-RECEIPT.json")
    sums_path = contained_file(args.archive_bundle, "SHA256SUMS", "bundle/SHA256SUMS")
    index, index_bytes = load_json(provenance, index_path, "bundle/ARCHIVE-INDEX.json")
    receipt, receipt_bytes = load_json(provenance, receipt_path, "bundle/BUILD-RECEIPT.json")
    sums_digest, sums_size = hash_file(sums_path, MAX_JSON_BYTES, "bundle/SHA256SUMS")
    checksum_bytes = read_file_bounded(sums_path, MAX_JSON_BYTES, "bundle/SHA256SUMS")
    index_sha = sha256_bytes(index_bytes)
    if index_sha != expected["archive_index_sha256"]:
        fail("archive_index_hash_mismatch", "bundle/ARCHIVE-INDEX.json")
    rows = validate_builder_receipt(index, receipt, binding["source_commit"])
    expected_bundle_inventory = {"ARCHIVE-INDEX.json", "BUILD-RECEIPT.json", "SHA256SUMS", *(row["path"] for row in rows)}
    expected_bundle_directories = {"/".join(PurePosixPath(p).parts[:i]) for p in expected_bundle_inventory for i in range(1, len(PurePosixPath(p).parts))}
    if set(bundle_inventory) != expected_bundle_inventory or bundle_directories != expected_bundle_directories:
        fail("archive_bundle_inventory", "archive_bundle")
    bundle_sums = parse_checksums(checksum_bytes, "bundle/SHA256SUMS")
    if set(bundle_sums) != {row["path"] for row in rows} or any(bundle_sums[row["path"]] != row["sha256"] for row in rows):
        fail("archive_bundle_checksums", "bundle/SHA256SUMS")
    inspected = inspect_archive_rows(args.archive_bundle, rows)
    del inspected
    verify_acquisition_zip(args.archive_zip, binding, args.archive_bundle, index_bytes, receipt_bytes, checksum_bytes, rows)
    if args.archive_bundle.resolve() != (args.acquisition_dir / "bundle").resolve():
        fail("archive_bundle_location", "archive_bundle")
    acquisition_records = validate_acquisition_records(args.acquisition_dir, binding, rows, index_sha, provenance, acquisition_helper)

    for rel in ("build-inputs/ARCHIVE-INDEX.json", "build-inputs/BUILD-RECEIPT.json", "build-inputs/acquisition.json"):
        actual, _ = hash_file(contained_file(args.evidence_dir, rel, rel), MAX_JSON_BYTES, rel)
        wanted = expected_deps[rel]
        if actual != wanted:
            fail("build_input_dependency", rel)
    evidence_index, evidence_index_bytes = load_json(provenance, contained_file(args.evidence_dir, "build-inputs/ARCHIVE-INDEX.json", "build-inputs/ARCHIVE-INDEX.json"), "evidence_index")
    evidence_receipt, evidence_receipt_bytes = load_json(provenance, contained_file(args.evidence_dir, "build-inputs/BUILD-RECEIPT.json", "build-inputs/BUILD-RECEIPT.json"), "evidence_build_receipt")
    acquisition_doc, acquisition_bytes = load_json(provenance, contained_file(args.evidence_dir, "build-inputs/acquisition.json", "build-inputs/acquisition.json"), "evidence_acquisition")
    if evidence_index_bytes != index_bytes or evidence_receipt_bytes != receipt_bytes:
        fail("retained_input_copy_mismatch", "build-inputs")
    if sha256_bytes(acquisition_bytes) != expected_deps["build-inputs/acquisition.json"]:
        fail("acquisition_dependency_mismatch", "build-inputs/acquisition.json")
    if not isinstance(acquisition_doc, dict) or set(acquisition_doc) != {"archive_count", "archive_index_sha256", "artifact_digest", "artifact_id", "derivation", "ecosystems", "repository", "run_id", "source_commit"} or acquisition_doc.get("repository") != "edithatogo/kairos" or acquisition_doc.get("run_id") != binding["original_run_id"] or acquisition_doc.get("artifact_id") != binding["acquisition_artifact_id"] or acquisition_doc.get("source_commit") != binding["source_commit"] or acquisition_doc.get("archive_index_sha256") != index_sha or acquisition_doc.get("artifact_digest") != "sha256:" + binding["archive_zip_sha256"] or acquisition_doc.get("archive_count") != len(rows) or acquisition_doc.get("ecosystems") != sorted(ECOSYSTEMS):
        fail("evidence_acquisition_binding", "build-inputs/acquisition.json")
    derivation = acquisition_doc.get("derivation")
    derivation_inputs = derivation.get("inputs") if isinstance(derivation, dict) else None
    required_derivation = {
        "archive_index_sha256": index_sha,
        "artifact_metadata_sha256": acquisition_records["hashes"]["artifact-metadata.json"],
        "original_local_verification_receipt_sha256": acquisition_records["hashes"]["receipt.json"],
        "source_commit_readback_sha256": acquisition_records["hashes"]["source-commit-readback.json"],
        **{name.replace("-", "_").replace(".", "_") + "_sha256": digest
           for name, digest in acquisition_records["hashes"].items()
           if name not in {"receipt.json", "artifact-metadata.json", "source-commit-readback.json"}},
    }
    if not isinstance(derivation, dict) or derivation.get("status") != "derived local adapter receipt; not original acquisition history" or derivation_inputs != required_derivation:
        fail("evidence_acquisition_derivation", "build-inputs/acquisition.json.derivation")

    for row in rows:
        rel = "archives/" + row["path"]
        evidence_file = contained_file(args.evidence_dir, rel, rel)
        digest, size = hash_file(evidence_file, MAX_ARCHIVE_BYTES, rel)
        if digest != row["sha256"] or size != row["bytes"]:
            fail("evidence_archive_copy", rel)
    manifest_value, _ = load_json(provenance, contained_file(args.evidence_dir, "release-artifact-manifest.json", "release-artifact-manifest.json"), "release_manifest")
    manifest_rows = validate_manifest(manifest_value, rows, index_sha, binding["source_commit"])
    coverage_value, _ = load_json(provenance, contained_file(args.evidence_dir, "SBOM-COVERAGE.json", "SBOM-COVERAGE.json"), "coverage")
    coverage_rows = validate_coverage(coverage_value, rows, expected_deps, binding)
    root_sbom, components = validate_spdx(args.evidence_dir, rows, coverage_rows, manifest_rows, schema_bytes, binding, supply, provenance)

    # Reuse the existing provenance oracle with its original five-field map.
    statement, statement_bytes = load_json(provenance, contained_file(args.evidence_dir, "provenance.json", "provenance.json"), "provenance")
    prov_issues = provenance.validate_provenance(statement, index, expected, index_sha)
    if prov_issues:
        fail("provenance_invalid", "provenance.json")
    receipt_result, _ = load_json(provenance, contained_file(args.evidence_dir, "validation-result.json", "validation-result.json"), "validation_result")
    receipt_expected = {
        "valid": True,
        "statement_sha256": sha256_bytes(statement_bytes),
        "archive_index_sha256": index_sha,
        "expected_inputs_sha256": expected_input_bytes_hash,
        "validator_sha256": expected_deps["packaging/scripts/validate_archive_copy_provenance.py"],
        "issue_count": 0,
        "issues": [],
        "claim_scope": RELEASE_CLAIM_SCOPE,
    }
    if receipt_result != receipt_expected:
        fail("validation_receipt_binding", "validation-result.json")

    expected_inventory = build_expected_inventory(rows)
    expected_evidence_directories = {"/".join(PurePosixPath(p).parts[:i]) for p in expected_inventory for i in range(1, len(PurePosixPath(p).parts))}
    if set(evidence_inventory) != expected_inventory or evidence_directories != expected_evidence_directories:
        fail("evidence_inventory", "evidence_dir")
    release_sums_data = read_file_bounded(contained_file(args.evidence_dir, "SHA256SUMS", "SHA256SUMS"), MAX_JSON_BYTES, "SHA256SUMS")
    release_sums = parse_checksums(release_sums_data, "SHA256SUMS")
    expected_release_sums = {item["path"]: item["sha256"] for item in manifest_rows}
    if release_sums != expected_release_sums:
        fail("release_checksums", "SHA256SUMS")
    supply_sums_data = read_file_bounded(contained_file(args.evidence_dir, "SUPPLY-CHAIN-SHA256SUMS", "SUPPLY-CHAIN-SHA256SUMS"), MAX_JSON_BYTES, "SUPPLY-CHAIN-SHA256SUMS")
    supply_sums = parse_checksums(supply_sums_data, "SUPPLY-CHAIN-SHA256SUMS")
    expected_supply_paths = expected_inventory - {"SUPPLY-CHAIN-SHA256SUMS"}
    if set(supply_sums) != expected_supply_paths:
        fail("supply_chain_checksum_inventory", "SUPPLY-CHAIN-SHA256SUMS")
    for rel, expected_hash in supply_sums.items():
        actual_hash, _ = hash_file(contained_file(args.evidence_dir, rel, rel), MAX_TOTAL_BYTES, rel)
        if actual_hash != expected_hash:
            fail("supply_chain_checksum_mismatch", "SUPPLY-CHAIN-SHA256SUMS")
    return {
        "valid": True,
        "profile": "kairos-archive-copy-evidence-v1",
        "archive_count": len(rows),
        "ecosystem_count": len(ECOSYSTEMS),
        "spdx_document_count": len(components) + 1,
        "evidence_file_count": len(expected_inventory),
        "archive_index_sha256": index_sha,
        "statement_sha256": sha256_bytes(statement_bytes),
        "claim_scope": RELEASE_CLAIM_SCOPE,
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--evidence-dir", required=True, type=Path)
    parser.add_argument("--archive-bundle", required=True, type=Path)
    parser.add_argument("--archive-zip", required=True, type=Path)
    parser.add_argument("--acquisition-dir", required=True, type=Path)
    parser.add_argument("--expected-inputs", required=True, type=Path, help="Independent five-field provenance map, outside evidence-dir")
    parser.add_argument("--expected-binding", required=True, type=Path, help="Independent outer acquisition/schema expectation object")
    parser.add_argument("--expected-verifier-sha256", required=True, help="Independent lowercase SHA-256 pin for this verifier source")
    parser.add_argument("--spdx-schema", required=True, type=Path)
    args = parser.parse_args(argv)
    try:
        with profile_deadline(MAX_PROFILE_SECONDS):
            result = verify_profile(args)
        print(json.dumps(result, sort_keys=True, separators=(",", ":")))
        return 0
    except VerificationFailure as exc:
        print(json.dumps({"valid": False, "issues": [{"code": exc.code, "path": exc.path}]}, sort_keys=True, separators=(",", ":")))
        return 1
    except ProfileTimeout:
        print(json.dumps({"valid": False, "issues": [{"code": "profile_deadline", "path": "$"}]}, separators=(",", ":")))
        return 2
    except Exception:
        print(json.dumps({"valid": False, "issues": [{"code": "internal_validation_error", "path": "$"}]}, separators=(",", ":")))
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
