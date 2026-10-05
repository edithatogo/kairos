#!/usr/bin/env python3
"""Independently read back a prepared actual-package-archives release tree.

This bounded profile checks copied archive bytes and their release manifest
against an independently pinned archive index. It does not verify SBOMs,
provenance, signatures, registries, publication, or release acceptance.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
from pathlib import Path, PurePosixPath
import re
import stat
import sys


PROFILE = "kairos-actual-archive-release-readback-v1"
CLAIM_SCOPE = (
    "actual archive output consistency only; no build, SBOM, provenance, signature, "
    "registry, publication, or release acceptance claim"
)
MAX_INDEX_BYTES = 8 * 1024 * 1024
MAX_MANIFEST_BYTES = 8 * 1024 * 1024
MAX_ARCHIVE_BYTES = 512 * 1024 * 1024
MAX_TOTAL_ARCHIVE_BYTES = 4 * 1024 * 1024 * 1024
MAX_TREE_ENTRIES = 64
SHA256 = re.compile(r"[0-9a-f]{64}\Z")
COMMIT = re.compile(r"[0-9a-f]{40}\Z")
ARCHIVES = {
    "rust": ("crate", (".crate",)),
    "python": ("python-distribution", (".whl", ".tar.gz")),
    "r": ("r-source-package", (".tar.gz",)),
    "julia": ("julia-source-archive", (".tar.gz",)),
    "typescript": ("npm-package", (".tgz",)),
    "nuget": ("nuget-package", (".nupkg",)),
    "go": ("go-source-archive", (".tar.gz",)),
}
OUTPUT_ROOT_FILES = {"release-artifact-manifest.json", "SHA256SUMS", "RELEASE.txt"}


class ValidationError(ValueError):
    """A safe, bounded readback failure."""


def fail(code: str) -> None:
    raise ValidationError(code)


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _absolute(path: Path, label: str) -> Path:
    raw = Path(path)
    if ".." in raw.parts:
        fail(label + "_path_parent_component")
    return Path(os.path.abspath(os.fspath(raw)))


def _directory_flags() -> int:
    return (os.O_RDONLY | getattr(os, "O_DIRECTORY", 0) | getattr(os, "O_NOFOLLOW", 0)
            | getattr(os, "O_CLOEXEC", 0))


def _open_directory(path: Path, label: str) -> int:
    target = _absolute(path, label)
    fd = os.open(target.anchor, _directory_flags())
    try:
        for component in target.parts[1:]:
            child = os.open(component, _directory_flags(), dir_fd=fd)
            os.close(fd)
            fd = child
        if not stat.S_ISDIR(os.fstat(fd).st_mode):
            fail(label + "_not_directory")
        return fd
    except BaseException:
        os.close(fd)
        raise


def _open_leaf(path: Path, label: str) -> tuple[int, int]:
    target = _absolute(path, label)
    if target == Path(target.anchor):
        fail(label + "_path")
    parent_fd = _open_directory(target.parent, label + "_parent")
    flags = os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0) | getattr(os, "O_NONBLOCK", 0) | getattr(os, "O_CLOEXEC", 0)
    try:
        leaf_fd = os.open(target.name, flags, dir_fd=parent_fd)
    except BaseException:
        os.close(parent_fd)
        raise
    return parent_fd, leaf_fd


def _read_regular(path: Path, limit: int, label: str) -> bytes:
    parent_fd, fd = _open_leaf(path, label)
    try:
        before = os.fstat(fd)
        if not stat.S_ISREG(before.st_mode):
            fail(label + "_not_regular")
        if before.st_size > limit:
            fail(label + "_too_large")
        chunks = bytearray()
        while len(chunks) <= limit:
            chunk = os.read(fd, min(65536, limit + 1 - len(chunks)))
            if not chunk:
                break
            chunks.extend(chunk)
        after = os.fstat(fd)
        if len(chunks) > limit:
            fail(label + "_too_large")
        if (len(chunks) != before.st_size or
                (before.st_dev, before.st_ino, before.st_size, before.st_mtime_ns, before.st_ctime_ns) !=
                (after.st_dev, after.st_ino, after.st_size, after.st_mtime_ns, after.st_ctime_ns)):
            fail(label + "_changed_during_read")
        return bytes(chunks)
    finally:
        os.close(fd)
        os.close(parent_fd)


def _hash_regular_at(root_fd: int, relative: str, limit: int, label: str) -> tuple[str, int]:
    parts = relative.split("/")
    if not parts or any(part in ("", ".", "..") for part in parts):
        fail(label + "_path")
    current_fd = os.dup(root_fd)
    try:
        for component in parts[:-1]:
            child_fd = os.open(component, _directory_flags(), dir_fd=current_fd)
            os.close(current_fd)
            current_fd = child_fd
        flags = os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0) | getattr(os, "O_NONBLOCK", 0) | getattr(os, "O_CLOEXEC", 0)
        file_fd = os.open(parts[-1], flags, dir_fd=current_fd)
        try:
            before = os.fstat(file_fd)
            if not stat.S_ISREG(before.st_mode):
                fail(label + "_not_regular")
            if before.st_size > limit:
                fail(label + "_too_large")
            digest = hashlib.sha256()
            total = 0
            while total <= limit:
                chunk = os.read(file_fd, min(1024 * 1024, limit + 1 - total))
                if not chunk:
                    break
                digest.update(chunk)
                total += len(chunk)
            after = os.fstat(file_fd)
            if total > limit:
                fail(label + "_too_large")
            if (total != before.st_size or
                    (before.st_dev, before.st_ino, before.st_size, before.st_mtime_ns, before.st_ctime_ns) !=
                    (after.st_dev, after.st_ino, after.st_size, after.st_mtime_ns, after.st_ctime_ns)):
                fail(label + "_changed_during_read")
            return digest.hexdigest(), total
        finally:
            os.close(file_fd)
    finally:
        os.close(current_fd)


def _pairs_without_duplicates(pairs: list[tuple[str, object]]) -> dict[str, object]:
    result: dict[str, object] = {}
    for key, value in pairs:
        if key in result:
            fail("json_duplicate_key")
        result[key] = value
    return result


def _finite_float(value: str) -> float:
    parsed = float(value)
    if not math.isfinite(parsed):
        fail("json_non_finite_number")
    return parsed


def _reject_constant(_: str) -> None:
    fail("json_non_finite_number")


def _check_depth(value: object, depth: int = 0) -> None:
    if depth > 128:
        fail("json_depth_exceeded")
    if isinstance(value, dict):
        for child in value.values():
            _check_depth(child, depth + 1)
    elif isinstance(value, list):
        for child in value:
            _check_depth(child, depth + 1)


def _strict_json(data: bytes, label: str) -> object:
    try:
        value = json.loads(data.decode("utf-8"), object_pairs_hook=_pairs_without_duplicates,
                           parse_constant=_reject_constant, parse_float=_finite_float)
        _check_depth(value)
        return value
    except (UnicodeDecodeError, json.JSONDecodeError, RecursionError, ValueError) as exc:
        if isinstance(exc, ValidationError):
            raise
        raise ValidationError(label + "_invalid_json") from exc


def _safe_relative(value: object) -> str:
    if not isinstance(value, str) or not value or "\\" in value or "\x00" in value:
        fail("archive_index_path_invalid")
    path = PurePosixPath(value)
    if (path.is_absolute() or path.as_posix() != value or ":" in value
            or any(part in ("", ".", "..") for part in path.parts)):
        fail("archive_index_path_invalid")
    return value


def _validate_index(data: bytes, expected_sha256: str, source_commit: str) -> tuple[str, list[dict[str, object]]]:
    if not SHA256.fullmatch(expected_sha256):
        fail("archive_index_sha256_invalid")
    if not COMMIT.fullmatch(source_commit):
        fail("release_source_commit_invalid")
    actual_sha256 = sha256(data)
    if actual_sha256 != expected_sha256:
        fail("archive_index_sha256_mismatch")
    index = _strict_json(data, "archive_index")
    if (not isinstance(index, dict) or set(index) != {"schema_version", "source_commit", "created_at_utc", "artifacts"}
            or type(index.get("schema_version")) is not int or index.get("schema_version") != 1
            or not isinstance(index.get("created_at_utc"), str) or not index["created_at_utc"]
            or not isinstance(index.get("artifacts"), list) or len(index["artifacts"]) != 8):
        fail("archive_index_shape_or_identity")
    if index.get("source_commit") != source_commit:
        fail("archive_index_source_commit_mismatch")

    rows: list[dict[str, object]] = []
    seen_paths: set[str] = set()
    seen_outputs: set[str] = set()
    ecosystems: set[str] = set()
    total_bytes = 0
    for row in index["artifacts"]:
        if not isinstance(row, dict) or set(row) != {"ecosystem", "kind", "path", "bytes", "sha256", "builder"}:
            fail("archive_index_row_shape")
        ecosystem = row["ecosystem"]
        if not isinstance(ecosystem, str) or ecosystem not in ARCHIVES:
            fail("archive_index_row_ecosystem")
        kind, extensions = ARCHIVES[ecosystem]
        relative = _safe_relative(row["path"])
        folded = relative.casefold()
        if (row["kind"] != kind or PurePosixPath(relative).parts[0] != ecosystem
                or not any(relative.endswith(ext) for ext in extensions)
                or relative in seen_paths or folded in seen_paths):
            fail("archive_index_row_path_or_kind")
        output_path = "archives/" + relative
        folded_output = output_path.casefold()
        components = folded_output.split("/")
        if (folded_output in seen_outputs
                or any("/".join(components[:depth]) in seen_outputs for depth in range(1, len(components)))
                or any(existing.startswith(folded_output + "/") for existing in seen_outputs)):
            fail("archive_index_output_path_collision")
        seen_paths.add(relative)
        seen_paths.add(folded)
        seen_outputs.add(folded_output)
        if not isinstance(row["sha256"], str) or not SHA256.fullmatch(row["sha256"]):
            fail("archive_index_row_sha256")
        if type(row["bytes"]) is not int or row["bytes"] <= 0 or row["bytes"] > MAX_ARCHIVE_BYTES:
            fail("archive_index_row_bytes")
        builder = row["builder"]
        if (not isinstance(builder, dict) or builder.get("ecosystem") != ecosystem
                or builder.get("source_commit") != source_commit or type(builder.get("exit_status")) is not int
                or builder.get("exit_status") != 0
                or any(not isinstance(builder.get(field), str) or not builder[field].strip()
                       for field in ("command", "toolchain", "platform"))):
            fail("archive_index_row_builder")
        total_bytes += row["bytes"]
        if total_bytes > MAX_TOTAL_ARCHIVE_BYTES:
            fail("archive_index_total_bytes")
        ecosystems.add(ecosystem)
        rows.append(row)
    if ecosystems != set(ARCHIVES):
        fail("archive_index_ecosystem_coverage")
    if [row["path"] for row in rows] != sorted(row["path"] for row in rows):
        fail("archive_index_row_order")
    return actual_sha256, rows


def _expected_outputs(rows: list[dict[str, object]], source_commit: str, index_sha256: str) -> tuple[dict[str, object], set[str], set[str]]:
    artifacts = []
    files = set(OUTPUT_ROOT_FILES)
    directories: set[str] = set()
    for row in rows:
        relative = "archives/" + str(row["path"])
        parts = PurePosixPath(relative).parts
        for depth in range(1, len(parts)):
            directories.add("/".join(parts[:depth]))
        files.add(relative)
        ecosystem = str(row["ecosystem"])
        artifacts.append({
            "path": relative,
            "sha256": row["sha256"],
            "bytes": row["bytes"],
            "ecosystem": "csharp" if ecosystem == "nuget" else ecosystem,
            "archive_ecosystem": ecosystem,
            "kind": row["kind"],
        })
    artifacts.sort(key=lambda item: str(item["path"]))
    manifest = {
        "schema_version": 1,
        "release_stage": "actual-package-archives",
        "source_commit": source_commit,
        "production_publish_enabled": False,
        "archive_index_sha256": index_sha256,
        "artifacts": artifacts,
    }
    return manifest, files, directories


def _tree_inventory(root_fd: int) -> tuple[set[str], set[str]]:
    files: set[str] = set()
    directories: set[str] = set()
    entries = 0

    def visit(directory_fd: int, prefix: str) -> None:
        nonlocal entries
        for name in os.listdir(directory_fd):
            entries += 1
            if entries > MAX_TREE_ENTRIES or name in ("", ".", "..") or "/" in name or "\\" in name:
                fail("release_tree_inventory_bound_or_name")
            relative = f"{prefix}/{name}" if prefix else name
            info = os.stat(name, dir_fd=directory_fd, follow_symlinks=False)
            if stat.S_ISLNK(info.st_mode):
                fail("release_tree_symlink")
            if stat.S_ISDIR(info.st_mode):
                directories.add(relative)
                child_fd = os.open(name, _directory_flags(), dir_fd=directory_fd)
                try:
                    opened = os.fstat(child_fd)
                    if (opened.st_dev, opened.st_ino) != (info.st_dev, info.st_ino):
                        fail("release_tree_directory_changed")
                    visit(child_fd, relative)
                finally:
                    os.close(child_fd)
            elif stat.S_ISREG(info.st_mode):
                files.add(relative)
            else:
                fail("release_tree_special_entry")

    visit(root_fd, "")
    return files, directories


def validate_release(release_root: Path, archive_index: Path, archive_index_sha256: str,
                     release_source_commit: str) -> dict[str, object]:
    release_path = _absolute(release_root, "release_root")
    index_path = _absolute(archive_index, "archive_index")
    if release_path == Path(release_path.anchor):
        fail("release_root_path_invalid")
    if index_path == release_path or release_path in index_path.parents:
        fail("archive_index_must_be_outside_release_root")
    index_bytes = _read_regular(index_path, MAX_INDEX_BYTES, "archive_index")
    index_sha256, rows = _validate_index(index_bytes, archive_index_sha256, release_source_commit)
    expected_manifest, expected_files, expected_directories = _expected_outputs(
        rows, release_source_commit, index_sha256
    )

    root_fd = _open_directory(release_path, "release_root")
    try:
        root_identity = os.fstat(root_fd)
        actual_files, actual_directories = _tree_inventory(root_fd)
        if actual_files != expected_files or actual_directories != expected_directories:
            fail("release_tree_inventory_mismatch")

        manifest_bytes = _read_regular_at(root_fd, "release-artifact-manifest.json", MAX_MANIFEST_BYTES,
                                          "release_manifest")
        manifest = _strict_json(manifest_bytes, "release_manifest")
        if (not isinstance(manifest, dict)
                or set(manifest) != {"schema_version", "release_stage", "source_commit",
                                     "production_publish_enabled", "archive_index_sha256", "artifacts"}
                or type(manifest.get("schema_version")) is not int
                or manifest.get("schema_version") != 1
                or manifest.get("release_stage") != "actual-package-archives"
                or manifest.get("source_commit") != release_source_commit
                or manifest.get("production_publish_enabled") is not False
                or manifest.get("archive_index_sha256") != index_sha256
                or not isinstance(manifest.get("artifacts"), list)
                or len(manifest["artifacts"]) != len(expected_manifest["artifacts"])):
            fail("release_manifest_mismatch")
        for artifact in manifest["artifacts"]:
            if (not isinstance(artifact, dict)
                    or set(artifact) != {"path", "sha256", "bytes", "ecosystem", "archive_ecosystem", "kind"}
                    or not isinstance(artifact.get("path"), str)
                    or not isinstance(artifact.get("sha256"), str)
                    or not SHA256.fullmatch(artifact["sha256"])
                    or type(artifact.get("bytes")) is not int
                    or not isinstance(artifact.get("ecosystem"), str)
                    or not isinstance(artifact.get("archive_ecosystem"), str)
                    or not isinstance(artifact.get("kind"), str)):
                fail("release_manifest_mismatch")
        if manifest != expected_manifest:
            fail("release_manifest_mismatch")

        expected_checksums = "".join(
            f"{item['sha256']}  {item['path']}\n" for item in expected_manifest["artifacts"]
        ).encode("utf-8")
        if _read_regular_at(root_fd, "SHA256SUMS", MAX_MANIFEST_BYTES, "release_checksums") != expected_checksums:
            fail("release_checksums_mismatch")
        expected_release_text = (
            f"Verified package archives from {release_source_commit}. "
            "Evidence preparation only; publication disabled.\n"
        ).encode("utf-8")
        if _read_regular_at(root_fd, "RELEASE.txt", MAX_MANIFEST_BYTES, "release_text") != expected_release_text:
            fail("release_text_mismatch")

        copied_bytes = 0
        for row in rows:
            relative = "archives/" + str(row["path"])
            digest, size = _hash_regular_at(root_fd, relative, MAX_ARCHIVE_BYTES, "release_archive")
            if size != row["bytes"] or digest != row["sha256"]:
                fail("release_archive_mismatch")
            copied_bytes += size
        final_files, final_directories = _tree_inventory(root_fd)
        if final_files != expected_files or final_directories != expected_directories:
            fail("release_tree_changed_during_readback")
        current_root_fd = _open_directory(release_path, "release_root")
        try:
            current_root = os.fstat(current_root_fd)
            if (root_identity.st_dev, root_identity.st_ino) != (current_root.st_dev, current_root.st_ino):
                fail("release_root_changed_during_readback")
        finally:
            os.close(current_root_fd)
        return {
            "schema": PROFILE,
            "result": "pass",
            "release_stage": "actual-package-archives",
            "source_commit": release_source_commit,
            "archive_index_sha256": index_sha256,
            "archive_count": len(rows),
            "ecosystem_count": len(ARCHIVES),
            "copied_archive_bytes": copied_bytes,
            "claim_scope": CLAIM_SCOPE,
        }
    finally:
        os.close(root_fd)


def _read_regular_at(root_fd: int, relative: str, limit: int, label: str) -> bytes:
    parts = relative.split("/")
    current_fd = os.dup(root_fd)
    try:
        for component in parts[:-1]:
            child_fd = os.open(component, _directory_flags(), dir_fd=current_fd)
            os.close(current_fd)
            current_fd = child_fd
        flags = os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0) | getattr(os, "O_NONBLOCK", 0) | getattr(os, "O_CLOEXEC", 0)
        fd = os.open(parts[-1], flags, dir_fd=current_fd)
        try:
            info = os.fstat(fd)
            if not stat.S_ISREG(info.st_mode):
                fail(label + "_not_regular")
            if info.st_size > limit:
                fail(label + "_too_large")
            data = bytearray()
            while len(data) <= limit:
                chunk = os.read(fd, min(65536, limit + 1 - len(data)))
                if not chunk:
                    break
                data.extend(chunk)
            after = os.fstat(fd)
            if len(data) > limit:
                fail(label + "_too_large")
            if (len(data) != info.st_size or
                    (info.st_dev, info.st_ino, info.st_size, info.st_mtime_ns, info.st_ctime_ns) !=
                    (after.st_dev, after.st_ino, after.st_size, after.st_mtime_ns, after.st_ctime_ns)):
                fail(label + "_changed_during_read")
            return bytes(data)
        finally:
            os.close(fd)
    finally:
        os.close(current_fd)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--release-root", type=Path, required=True)
    parser.add_argument("--verified-archive-index", type=Path, required=True)
    parser.add_argument("--archive-index-sha256", required=True)
    parser.add_argument("--release-source-commit", required=True)
    args = parser.parse_args(argv)
    try:
        report = validate_release(args.release_root, args.verified_archive_index,
                                  args.archive_index_sha256, args.release_source_commit)
    except (OSError, ValidationError, RecursionError) as exc:
        code = str(exc) if isinstance(exc, ValidationError) else "filesystem_read_failed"
        print(json.dumps({"schema": PROFILE, "result": "fail", "error": code}, sort_keys=True))
        return 1
    print(json.dumps(report, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
