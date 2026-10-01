#!/usr/bin/env python3
"""Validate built package archives and assemble a checksummed retention tree."""

from __future__ import annotations

import argparse
import hashlib
import json
import shutil
import stat
import tarfile
import zipfile
from datetime import datetime, timezone
from pathlib import Path, PurePosixPath


ARCHIVES = {
    "rust": ("crate", {".crate"}),
    "python": ("python-distribution", {".whl", ".tar.gz"}),
    "r": ("r-source-package", {".tar.gz"}),
    "julia": ("julia-source-archive", {".tar.gz"}),
    "typescript": ("npm-package", {".tgz"}),
    "nuget": ("nuget-package", {".nupkg"}),
    "go": ("go-source-archive", {".tar.gz"}),
}


def is_archive(path: Path, extensions: set[str]) -> bool:
    return any(path.name.endswith(extension) for extension in extensions)


def validate_archive(path: Path) -> None:
    if path.name.endswith((".tar.gz", ".crate", ".tgz")):
        with tarfile.open(path, "r:gz") as archive:
            members = archive.getmembers()
            if not members:
                raise ValueError(f"empty tar archive: {path}")
            for member in members:
                name = PurePosixPath(member.name)
                if name.is_absolute() or ".." in name.parts:
                    raise ValueError(f"unsafe path in {path}: {member.name}")
                if member.issym() or member.islnk():
                    raise ValueError(f"links are not allowed in {path}: {member.name}")
        return
    if path.suffix in {".whl", ".nupkg"}:
        with zipfile.ZipFile(path) as archive:
            names = archive.namelist()
            if not names:
                raise ValueError(f"empty zip archive: {path}")
            for entry in names:
                name = PurePosixPath(entry)
                if name.is_absolute() or ".." in name.parts:
                    raise ValueError(f"unsafe path in {path}: {entry}")
                mode = archive.getinfo(entry).external_attr >> 16
                if stat.S_ISLNK(mode):
                    raise ValueError(f"links are not allowed in {path}: {entry}")
            bad_member = archive.testzip()
            if bad_member:
                raise ValueError(f"corrupt zip member in {path}: {bad_member}")
        return
    raise ValueError(f"unsupported archive type: {path}")


def checksum(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def build(
    source: Path,
    output: Path,
    source_commit: str,
) -> None:
    if output.exists() and any(output.iterdir()):
        raise ValueError(f"output directory must be empty: {output}")
    output.mkdir(parents=True, exist_ok=True)
    artifacts: list[dict[str, object]] = []
    receipts: dict[str, dict[str, str]] = {}

    for ecosystem, (kind, extensions) in ARCHIVES.items():
        source_dirs = [
            source / ecosystem,
            source / f"kairos-package-{ecosystem}",
            source / f"{ecosystem}-source",
        ]
        ecosystem_source = next((path for path in source_dirs if path.is_dir()), None)
        if ecosystem_source is None:
            raise ValueError(f"missing {ecosystem} archive directory under {source}")
        receipt_path = ecosystem_source / "BUILD-INFO.json"
        if not receipt_path.is_file():
            raise ValueError(f"missing build receipt for {ecosystem}: {receipt_path}")
        receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
        if receipt.get("ecosystem") != ecosystem or receipt.get("source_commit") != source_commit:
            raise ValueError(f"build receipt does not match {ecosystem} and source commit")
        if not all(receipt.get(key) for key in ("command", "toolchain", "platform")):
            raise ValueError(f"build receipt is incomplete for {ecosystem}")
        if receipt.get("exit_status") != 0:
            raise ValueError(f"package build did not complete successfully for {ecosystem}")
        if ecosystem in receipts:
            raise ValueError(f"duplicate source directory for ecosystem: {ecosystem}")
        receipts[ecosystem] = receipt
        candidates = sorted(
            path for path in ecosystem_source.rglob("*")
            if path.is_file() and is_archive(path, extensions)
        )
        if not candidates:
            raise ValueError(f"missing {ecosystem} package archive under {ecosystem_source}")
        target_dir = output / ecosystem
        target_dir.mkdir(parents=True, exist_ok=True)
        for item in candidates:
            if item.is_symlink():
                raise ValueError(f"archive symlinks are not allowed: {item}")
            validate_archive(item)
            target = target_dir / item.name
            shutil.copyfile(item, target)
            artifacts.append({
                "ecosystem": ecosystem,
                "kind": kind,
                "path": target.relative_to(output).as_posix(),
                "bytes": target.stat().st_size,
                "sha256": checksum(target),
                "builder": receipt,
            })

    artifacts.sort(key=lambda row: str(row["path"]))
    index = {
        "schema_version": 1,
        "source_commit": source_commit,
        "created_at_utc": datetime.now(timezone.utc).isoformat(),
        "artifacts": artifacts,
    }
    (output / "ARCHIVE-INDEX.json").write_text(
        json.dumps(index, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    (output / "BUILD-RECEIPT.json").write_text(
        json.dumps(
            {"source_commit": source_commit, "ecosystems": receipts},
            indent=2,
            sort_keys=True,
        )
        + "\n",
        encoding="utf-8",
    )
    checksums = "".join(
        f"{row['sha256']}  {row['path']}\n" for row in artifacts
    )
    (output / "SHA256SUMS").write_text(checksums, encoding="utf-8")


def verify(output: Path) -> None:
    index = json.loads((output / "ARCHIVE-INDEX.json").read_text(encoding="utf-8"))
    if index.get("schema_version") != 1:
        raise ValueError("unsupported archive index schema")
    if len(index.get("source_commit", "")) != 40:
        raise ValueError("archive index has no full source commit")
    receipt_path = output / "BUILD-RECEIPT.json"
    if not receipt_path.is_file():
        raise ValueError("package build receipt is missing")
    receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
    if receipt.get("source_commit") != index["source_commit"]:
        raise ValueError("package build receipt has a different source commit")
    if set(receipt.get("ecosystems", {})) != set(ARCHIVES):
        raise ValueError("package build receipt must cover all seven ecosystems")
    artifacts = index.get("artifacts")
    if not isinstance(artifacts, list) or not artifacts:
        raise ValueError("archive index has no artifacts")
    expected_checksums: list[str] = []
    expected_paths: set[str] = set()
    ecosystems: set[str] = set()
    for artifact in artifacts:
        ecosystem = artifact.get("ecosystem")
        if ecosystem not in ARCHIVES or artifact.get("kind") != ARCHIVES[ecosystem][0]:
            raise ValueError(f"unknown ecosystem or archive kind: {ecosystem}")
        if artifact.get("builder") != receipt["ecosystems"].get(ecosystem):
            raise ValueError(f"archive build receipt mismatch: {ecosystem}")
        ecosystems.add(ecosystem)
        relative = artifact["path"]
        if relative in expected_paths:
            raise ValueError(f"duplicate archive index path: {relative}")
        path = output / relative
        if path.is_symlink() or not path.is_file():
            raise ValueError(f"archive is missing or is a symlink: {relative}")
        digest = checksum(path)
        if digest != artifact["sha256"] or path.stat().st_size != artifact["bytes"]:
            raise ValueError(f"archive does not match index: {relative}")
        validate_archive(path)
        expected_checksums.append(f"{digest}  {relative}\n")
        expected_paths.add(relative)
    if ecosystems != set(ARCHIVES):
        raise ValueError("archive index must cover all seven package ecosystems")
    actual_paths = {
        p.relative_to(output).as_posix()
        for p in output.rglob("*")
        if p.is_file() and p.name not in {
            "ARCHIVE-INDEX.json", "BUILD-RECEIPT.json", "SHA256SUMS"
        }
    }
    if actual_paths != expected_paths:
        raise ValueError("archive tree and index entries differ")
    if (output / "SHA256SUMS").read_text(encoding="utf-8") != "".join(expected_checksums):
        raise ValueError("SHA256SUMS does not match archive index")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--source-commit", required=True)
    args = parser.parse_args()
    build(args.input.resolve(), args.output.resolve(), args.source_commit)
    verify(args.output.resolve())
    index_path = args.output / "ARCHIVE-INDEX.json"
    artifact_count = len(json.loads(index_path.read_text(encoding="utf-8"))["artifacts"])
    print(f"verified {artifact_count} package archives")


if __name__ == "__main__":
    main()
