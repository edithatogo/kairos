#!/usr/bin/env python3
"""Derive independent verifier inputs from pinned retained acquisition records.

This command reads retained source evidence and a native-qualified Syft receipt;
it never runs a build, scanner, network request, or evidence generator.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import shutil
import stat
import subprocess
import sys
import types
import zipfile
from typing import Any

REPOSITORY = "edithatogo/kairos"
HELPERS = (
    "packaging/scripts/build_archive_supply_chain.py",
    "packaging/scripts/build_archive_release_manifest.py",
    "packaging/scripts/build_package_archive_bundle.py",
    "packaging/scripts/acquire_package_archive_bundle.py",
    "packaging/scripts/validate_archive_copy_provenance.py",
)
VERIFIER = "packaging/scripts/verify_archive_supply_chain_evidence.py"
BUILDER = "packaging/scripts/build_archive_evidence_expectations.py"
SCHEMA = "tests/fixtures/archive-supply-chain/spdx-2.3/spdx-schema.json"
SYFT_INSTALLER = "scripts/supply_chain/install_verified_syft.py"
SYFT_DARWIN_LOCK = "scripts/supply_chain/syft-darwin-verifier.lock"
SYFT_LINUX_LOCK = "scripts/supply_chain/syft-linux-verifier.lock"
SYFT_LOCKS = {
    "syft-darwin-verifier.lock": SYFT_DARWIN_LOCK,
    "syft-linux-verifier.lock": SYFT_LINUX_LOCK,
}
SYFT_QUALIFICATION_LIMIT = (
    "Native authenticated installation and version probe only; no package scan, release, or publication is represented."
)
COMMIT_RE = re.compile(r"^[0-9a-f]{40}$")
DIGEST_RE = re.compile(r"^[0-9a-f]{64}$")
MAX_JSON = 8 * 1024 * 1024
MAX_SMALL = 1024 * 1024
MAX_ZIP = 64 * 1024 * 1024
GIT_TIMEOUT = 15


class InputError(ValueError):
    pass


def fail(message: str) -> None:
    raise InputError(message)


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def strict_json(data: bytes, label: str) -> Any:
    if len(data) > MAX_JSON:
        fail(f"{label} exceeds size limit")

    def pairs(items: list[tuple[str, Any]]) -> dict[str, Any]:
        result: dict[str, Any] = {}
        for key, value in items:
            if key in result:
                fail(f"{label} has duplicate keys")
            result[key] = value
        return result

    def bad_constant(_: str) -> None:
        fail(f"{label} contains a non-JSON constant")

    try:
        value = json.loads(data.decode("utf-8"), object_pairs_hook=pairs, parse_constant=bad_constant)
    except InputError:
        raise
    except Exception as exc:
        raise InputError(f"{label} is invalid JSON") from exc
    stack = [(value, 0)]
    while stack:
        current, depth = stack.pop()
        if depth > 128:
            fail(f"{label} exceeds nesting limit")
        if isinstance(current, dict):
            stack.extend((item, depth + 1) for item in current.values())
        elif isinstance(current, list):
            stack.extend((item, depth + 1) for item in current)
    return value


def checked_path(path: Path, label: str, *, must_exist: bool = True) -> Path:
    absolute = Path(os.path.abspath(path))
    current = Path(absolute.anchor)
    for pos, component in enumerate(absolute.parts[1:]):
        current = current / component
        try:
            info = current.lstat()
        except FileNotFoundError:
            if must_exist or pos != len(absolute.parts[1:]) - 1:
                fail(f"{label} path is missing")
            continue
        if stat.S_ISLNK(info.st_mode):
            fail(f"{label} path contains a symlink")
        if pos < len(absolute.parts[1:]) - 1 and not stat.S_ISDIR(info.st_mode):
            fail(f"{label} parent is not a directory")
    return absolute


def read_nofollow(path: Path, limit: int, label: str) -> bytes:
    absolute = checked_path(path, label)
    flags = os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0) | getattr(os, "O_NONBLOCK", 0)
    descriptor = os.open(absolute, flags)
    try:
        info = os.fstat(descriptor)
        if not stat.S_ISREG(info.st_mode) or info.st_nlink != 1 or info.st_size > limit:
            fail(f"{label} is not a bounded private regular file")
        with os.fdopen(descriptor, "rb") as stream:
            descriptor = -1
            data = stream.read(limit + 1)
        if len(data) > limit:
            fail(f"{label} exceeds size limit")
        return data
    finally:
        if descriptor >= 0:
            os.close(descriptor)


def git(root: Path, *args: str, limit: int = MAX_JSON) -> bytes:
    try:
        result = subprocess.run(
            ["git", "-C", str(root), *args], stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, timeout=GIT_TIMEOUT,
            check=False, close_fds=True,
        )
    except (OSError, subprocess.TimeoutExpired) as exc:
        raise InputError("bounded Git read failed") from exc
    if result.returncode != 0 or len(result.stdout) > limit:
        fail("bounded Git read failed")
    return result.stdout


def trusted_blobs(root: Path, consumer_sha: str) -> tuple[dict[str, bytes], dict[str, str]]:
    if not COMMIT_RE.fullmatch(consumer_sha):
        fail("trusted consumer SHA must be full lowercase SHA-1")
    head = git(root, "rev-parse", "--verify", "HEAD", limit=128).decode("ascii", "strict").strip()
    if head != consumer_sha:
        fail("trusted consumer SHA differs from checkout HEAD")
    paths = (*HELPERS, VERIFIER, BUILDER, SCHEMA, SYFT_INSTALLER, SYFT_DARWIN_LOCK, SYFT_LINUX_LOCK)
    blobs: dict[str, bytes] = {}
    hashes: dict[str, str] = {}
    for relative in paths:
        blob = git(root, "cat-file", "blob", f"{consumer_sha}:{relative}", limit=2 * MAX_JSON)
        checkout = read_nofollow(root / relative, 2 * MAX_JSON, relative)
        if checkout != blob:
            fail(f"checkout bytes differ from trusted Git blob: {relative}")
        blobs[relative] = blob
        hashes[relative] = sha256(blob)
    return blobs, hashes


def contained(root: Path, relative: str) -> Path:
    if not isinstance(relative, str) or not relative or relative.startswith("/") or "\\" in relative:
        fail("unsafe archive path")
    parts = relative.split("/")
    if any(part in {"", ".", ".."} for part in parts):
        fail("unsafe archive path")
    path = root.joinpath(*parts)
    checked_path(path, "bundle file")
    return path


def write_exclusive(path: Path, data: bytes) -> None:
    descriptor = -1
    created_identity: tuple[int, int] | None = None
    try:
        descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | getattr(os, "O_NOFOLLOW", 0), 0o600)
        opened = os.fstat(descriptor)
        created_identity = (opened.st_dev, opened.st_ino)
        with os.fdopen(descriptor, "wb") as stream:
            descriptor = -1
            stream.write(data)
            stream.flush()
            os.fsync(stream.fileno())
    except BaseException:
        if descriptor >= 0:
            try:
                os.close(descriptor)
            except OSError:
                pass
        if created_identity is not None:
            try:
                current = path.lstat()
                if (current.st_dev, current.st_ino) == created_identity:
                    path.unlink()
            except OSError:
                pass
        raise


def ensure_directory(path: Path) -> None:
    absolute = Path(os.path.abspath(path))
    missing: list[Path] = []
    cursor = absolute
    while not cursor.exists():
        missing.append(cursor)
        cursor = cursor.parent
    checked_path(cursor, "output parent")
    for directory in reversed(missing):
        directory.mkdir(mode=0o700)
    checked_path(absolute, "output parent")


def canonical(value: Any) -> bytes:
    return (json.dumps(value, ensure_ascii=False, sort_keys=True, indent=2, allow_nan=False) + "\n").encode("utf-8")


def load_trusted_module(verifier: Any, blob: bytes, name: str, relative: str) -> Any:
    if verifier is not None:
        return verifier.load_verified_module(blob, name, relative)
    module = types.ModuleType(name)
    module.__file__ = relative
    module.__package__ = ""
    sys.modules[name] = module
    try:
        exec(compile(blob, relative, "exec", dont_inherit=True), module.__dict__)
    except Exception as exc:
        sys.modules.pop(name, None)
        raise InputError("trusted source blob could not be loaded") from exc
    return module


def validate_bundle(verifier: Any, bundle: Path, sums_bytes: bytes, rows: list[dict[str, Any]]) -> None:
    expected = {"ARCHIVE-INDEX.json", "BUILD-RECEIPT.json", "SHA256SUMS"}
    expected.update(row["path"] for row in rows)
    found: set[str] = set()
    stack = [bundle]
    while stack:
        directory = stack.pop()
        checked_path(directory, "bundle directory")
        try:
            entries = list(os.scandir(directory))
        except OSError as exc:
            raise InputError("bundle directory is unreadable") from exc
        for entry in entries:
            candidate = Path(entry.path)
            relative = candidate.relative_to(bundle).as_posix()
            checked_path(candidate, "bundle entry")
            info = candidate.lstat()
            if stat.S_ISDIR(info.st_mode):
                stack.append(candidate)
            elif stat.S_ISREG(info.st_mode) and relative not in found:
                found.add(relative)
            else:
                fail("bundle contains a duplicate or nonregular entry")
            if len(found) + len(stack) > 256:
                fail("bundle inventory exceeds entry limit")
    if found != expected:
        fail("bundle inventory differs from its archive index")
    checksum_map = verifier.parse_checksums(sums_bytes, "SHA256SUMS")
    expected_checksums = {row["path"]: row["sha256"] for row in rows}
    if checksum_map != expected_checksums:
        fail("bundle checksum list differs from archive index")
    # Reuse the trusted verifier's bounded parsers and no-follow readers for
    # each indexed package archive before comparing the original ZIP.
    verifier.inspect_archive_rows(bundle, rows)


def validate_syft(args: argparse.Namespace, hashes: dict[str, str], installer: Any) -> dict[str, Any]:
    if not DIGEST_RE.fullmatch(args.syft_sha256) or not DIGEST_RE.fullmatch(args.syft_receipt_sha256):
        fail("Syft binary and receipt pins must be lowercase SHA-256")
    binary = read_nofollow(args.syft, 256 * 1024 * 1024, "Syft binary")
    receipt_bytes = read_nofollow(args.syft_receipt, MAX_SMALL, "Syft qualification receipt")
    if sha256(binary) != args.syft_sha256 or sha256(receipt_bytes) != args.syft_receipt_sha256:
        fail("Syft binary or receipt differs from caller pin")
    receipt = strict_json(receipt_bytes, "Syft qualification receipt")
    try:
        target = installer.detect_target(platform.system(), platform.machine())
    except (RuntimeError, KeyError, TypeError) as exc:
        raise InputError("native Syft qualification is unavailable for this host") from exc
    if not isinstance(target, dict):
        fail("trusted installer target is invalid")
    target_key = target.get("key")
    target_platform = target.get("platform")
    target_lock_name = target.get("verifier_lock")
    lock_path = SYFT_LOCKS.get(target_lock_name) if isinstance(target_lock_name, str) else None
    target_lock_sha = target.get("verifier_lock_sha256")
    target_binary_sha = target.get("binary_sha256")
    if (not isinstance(target_key, str) or not target_key
            or not isinstance(target_platform, str) or not target_platform
            or lock_path is None
            or not isinstance(target_lock_sha, str) or not DIGEST_RE.fullmatch(target_lock_sha)
            or not isinstance(target_binary_sha, str) or not DIGEST_RE.fullmatch(target_binary_sha)):
        fail("trusted installer target lacks exact platform, binary, or verifier-lock pins")
    if hashes.get(lock_path) != target_lock_sha:
        fail("trusted target verifier lock differs from its Git blob")
    if args.syft_sha256 != target_binary_sha:
        fail("Syft binary pin differs from trusted installer target")
    if not isinstance(receipt, dict) or receipt.get("schema") != "kairos.verified-syft-installer.v1" or receipt.get("result") != "pass":
        fail("Syft receipt is not a passing qualified receipt")
    if (receipt.get("repository") != installer.REPOSITORY
            or receipt.get("ref") != installer.WORKFLOW_REF
            or receipt.get("target") != target_key
            or receipt.get("platform") != target_platform):
        fail("Syft receipt platform or release identity is not the currently qualified target")
    if (receipt.get("binary_sha256") != args.syft_sha256
            or receipt.get("version") != installer.VERSION
            or receipt.get("release_commit") != installer.RELEASE_COMMIT):
        fail("Syft receipt binary or version differs from explicit pin")
    version_probe = receipt.get("version_probe")
    if (not isinstance(version_probe, dict) or version_probe.get("application") != "syft"
            or version_probe.get("version") != receipt.get("version")
            or version_probe.get("platform") != receipt.get("platform")
            or version_probe.get("gitCommit") != receipt.get("release_commit")):
        fail("Syft version probe does not bind the qualified release identity")
    if receipt.get("installer_source_sha256") != hashes[SYFT_INSTALLER]:
        fail("Syft installer source differs from trusted checkout blob")
    verifier = receipt.get("verifier")
    if not isinstance(verifier, dict) or verifier.get("lock_path") != target_lock_name or verifier.get("lock_sha256") != hashes[lock_path]:
        fail("Syft verifier lock differs from trusted checkout blob")
    if receipt.get("qualification_limit") != SYFT_QUALIFICATION_LIMIT:
        fail("Syft qualification scope differs from the trusted installer contract")
    # A separate gate validates the upstream native receipt and signature. This
    # builder checks its immutable receipt, target, source, lock, and binary pins
    # but does not re-run that verifier or establish signature validity.
    return {"binary_sha256": args.syft_sha256, "receipt": receipt, "target": target, "lock_path": lock_path}


def derive(args: argparse.Namespace) -> tuple[dict[str, Any], dict[str, Any], dict[str, Any], dict[str, str], dict[str, Any]]:
    root = checked_path(args.repository, "consumer checkout")
    blobs, hashes = trusted_blobs(root, args.trusted_consumer_sha)
    verifier = load_trusted_module(None, blobs[VERIFIER], "kairos_trusted_archive_verifier", VERIFIER)
    installer = load_trusted_module(None, blobs[SYFT_INSTALLER], "kairos_trusted_syft_installer", SYFT_INSTALLER)
    acquisition_helper = load_trusted_module(verifier, blobs[HELPERS[3]], "kairos_trusted_archive_acquisition", HELPERS[3])
    provenance_helper = load_trusted_module(verifier, blobs[HELPERS[4]], "kairos_trusted_archive_provenance", HELPERS[4])
    syft_qualification = validate_syft(args, hashes, installer)
    syft_sha = syft_qualification["binary_sha256"]

    acquisition = checked_path(args.acquisition_dir, "retained acquisition")
    bundle = checked_path(args.archive_bundle, "retained bundle")
    archive_zip = checked_path(args.archive_zip, "retained archive ZIP")
    if bundle != acquisition / "bundle" or archive_zip != acquisition / f"{args.artifact_id}.zip":
        fail("bundle and ZIP must be the exact retained acquisition paths")
    schema_path = root / SCHEMA
    schema_bytes = blobs[SCHEMA]
    if read_nofollow(schema_path, MAX_JSON, "SPDX schema") != schema_bytes:
        fail("SPDX schema checkout differs from trusted blob")
    spdx_sha = hashes[SCHEMA]
    zip_sha = args.archive_zip_sha256
    if not DIGEST_RE.fullmatch(zip_sha) or type(args.archive_zip_bytes) is not int or args.archive_zip_bytes <= 0:
        fail("invalid archive ZIP digest or byte count pin")
    if type(args.run_id) is not int or args.run_id <= 0 or type(args.artifact_id) is not int or args.artifact_id <= 0:
        fail("run and artifact IDs must be positive integers")
    if not COMMIT_RE.fullmatch(args.source_commit) or not COMMIT_RE.fullmatch(args.producer_tree):
        fail("source and producer tree pins must be full lowercase commit SHAs")

    binding = {
        "schema_version": 1,
        "repository": REPOSITORY,
        "source_commit": args.source_commit,
        "producer_pr_head": args.source_commit,
        "producer_tree": args.producer_tree,
        "original_run_id": args.run_id,
        "acquisition_artifact_id": args.artifact_id,
        "archive_zip_sha256": zip_sha,
        "archive_zip_bytes": args.archive_zip_bytes,
        "spdx_schema_sha256": spdx_sha,
    }
    verifier.validate_binding(binding)
    index_bytes = read_nofollow(contained(bundle, "ARCHIVE-INDEX.json"), MAX_SMALL, "archive index")
    receipt_bytes = read_nofollow(contained(bundle, "BUILD-RECEIPT.json"), MAX_SMALL, "build receipt")
    sums_bytes = read_nofollow(contained(bundle, "SHA256SUMS"), MAX_SMALL, "bundle checksums")
    index = verifier.strict_json_bytes(index_bytes, "ARCHIVE-INDEX.json")
    build_receipt = verifier.strict_json_bytes(receipt_bytes, "BUILD-RECEIPT.json")
    rows = verifier.validate_builder_receipt(index, build_receipt, args.source_commit)
    validate_bundle(verifier, bundle, sums_bytes, rows)
    verifier.verify_acquisition_zip(archive_zip, binding, bundle, index_bytes, receipt_bytes, sums_bytes, rows)
    source_record = verifier.validate_acquisition_records(acquisition, binding, rows, sha256(index_bytes), provenance_helper, acquisition_helper)
    if source_record.get("main_mode") is not True:
        fail("retained records do not prove exact successful main-dispatch lineage")
    if (source_record["receipt"].get("workflow_run") != args.run_id
            or source_record["receipt"].get("artifact_id") != args.artifact_id
            or source_record["receipt"].get("producer_tree") != args.producer_tree
            or source_record["receipt"].get("archive_zip_sha256") != zip_sha):
        fail("caller pins differ from retained acquisition records")

    acquisition_document = {
        "archive_count": len(rows),
        "archive_index_sha256": sha256(index_bytes),
        "artifact_digest": "sha256:" + zip_sha,
        "artifact_id": args.artifact_id,
        "derivation": {"status": "derived local adapter receipt; not original acquisition history", "inputs": {
            "archive_index_sha256": sha256(index_bytes),
            "artifact_metadata_sha256": source_record["hashes"]["artifact-metadata.json"],
            "original_local_verification_receipt_sha256": source_record["hashes"]["receipt.json"],
            "source_commit_readback_sha256": source_record["hashes"]["source-commit-readback.json"],
            **{name.replace("-", "_").replace(".", "_") + "_sha256": digest
               for name, digest in source_record["hashes"].items()
               if name not in {"receipt.json", "artifact-metadata.json", "source-commit-readback.json"}},
        }},
        "ecosystems": sorted(verifier.ECOSYSTEMS),
        "repository": REPOSITORY,
        "run_id": args.run_id,
        "source_commit": args.source_commit,
    }
    if set(acquisition_document) != {"archive_count", "archive_index_sha256", "artifact_digest", "artifact_id", "derivation", "ecosystems", "repository", "run_id", "source_commit"}:
        fail("derived acquisition adapter schema mismatch")
    adapter_bytes = canonical(acquisition_document)
    dependencies = {
        f"https://github.com/{REPOSITORY}/actions/runs/{args.run_id}": zip_sha,
        "ARCHIVE-INDEX.json": sha256(index_bytes),
        "build-inputs/ARCHIVE-INDEX.json": sha256(index_bytes),
        "build-inputs/BUILD-RECEIPT.json": sha256(receipt_bytes),
        "build-inputs/acquisition.json": sha256(adapter_bytes),
        **{path: hashes[path] for path in HELPERS},
        "tool:syft": syft_sha,
        "schema:spdx-2.3": spdx_sha,
    }
    if len(dependencies) != 12:
        fail("expected exactly twelve dependency pins")
    expected_inputs = {
        "archive_index_sha256": sha256(index_bytes),
        "source_commit": args.source_commit,
        "original_run_id": args.run_id,
        "acquisition_artifact_id": args.artifact_id,
        "dependencies": [{"id": key, "sha256": value} for key, value in dependencies.items()],
    }
    verifier.validate_expected_inputs(expected_inputs, binding)
    return binding, expected_inputs, acquisition_document, hashes, syft_qualification


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repository", type=Path, default=Path.cwd())
    parser.add_argument("--trusted-consumer-sha", required=True)
    parser.add_argument("--acquisition-dir", type=Path, required=True)
    parser.add_argument("--archive-bundle", type=Path, required=True)
    parser.add_argument("--archive-zip", type=Path, required=True)
    parser.add_argument("--run-id", type=int, required=True)
    parser.add_argument("--artifact-id", type=int, required=True)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--producer-tree", required=True)
    parser.add_argument("--archive-zip-sha256", required=True)
    parser.add_argument("--archive-zip-bytes", type=int, required=True)
    parser.add_argument("--syft", type=Path, required=True)
    parser.add_argument("--syft-sha256", required=True)
    parser.add_argument("--syft-receipt", type=Path, required=True)
    parser.add_argument("--syft-receipt-sha256", required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--preparation-receipt", type=Path, required=True,
                        help="Separate fresh ignored evidence receipt path; not part of the three verifier inputs")
    args = parser.parse_args(argv)
    created = False
    owned_output: Path | None = None
    preparation_receipt: Path | None = None
    receipt_created = False
    try:
        output = Path(os.path.abspath(args.output_dir))
        preparation_receipt = Path(os.path.abspath(args.preparation_receipt))
        if output.exists() or output.is_symlink():
            fail("output directory must be fresh")
        if preparation_receipt.exists() or preparation_receipt.is_symlink():
            fail("preparation receipt path must be fresh")
        protected = (args.acquisition_dir, args.archive_bundle, args.archive_zip, args.syft, args.syft_receipt)
        for item in protected:
            protected_abs = Path(os.path.abspath(item))
            if (output == protected_abs or output in protected_abs.parents or protected_abs in output.parents
                    or preparation_receipt == protected_abs or preparation_receipt in protected_abs.parents
                    or protected_abs in preparation_receipt.parents):
                fail("output must be separate from source and trusted input paths")
        if (preparation_receipt == output or preparation_receipt in output.parents
                or output in preparation_receipt.parents):
            fail("preparation receipt must be separate from the verifier input directory")
        ensure_directory(output.parent)
        ensure_directory(preparation_receipt.parent)
        binding, expected, adapter, hashes, syft_qualification = derive(args)
        checked_path(output, "output directory", must_exist=False)
        output.mkdir(mode=0o700)
        created = True
        owned_output = output
        files = {
            "outer-binding.json": canonical(binding),
            "expected-inputs.json": canonical(expected),
            "acquisition.json": canonical(adapter),
        }
        for name, data in files.items():
            write_exclusive(output / name, data)
        receipt = syft_qualification["receipt"]
        preparation = {
            "schema_version": 1,
            "kind": "archive-evidence-expectations-preparation",
            "trusted_consumer_sha": args.trusted_consumer_sha,
            "trusted_builder_sha256": hashes[BUILDER],
            "trusted_verifier_sha256": hashes[VERIFIER],
            "spdx_schema_sha256": hashes[SCHEMA],
            "producer_pins": {"repository": REPOSITORY, "run_id": args.run_id, "artifact_id": args.artifact_id,
                              "source_commit": args.source_commit, "producer_tree": args.producer_tree,
                              "archive_zip_sha256": args.archive_zip_sha256, "archive_zip_bytes": args.archive_zip_bytes},
            "syft_qualification": {
                "receipt_path": str(args.syft_receipt.absolute()),
                "receipt_sha256": args.syft_receipt_sha256,
                "binary_path": str(args.syft.absolute()),
                "binary_sha256": syft_qualification["binary_sha256"],
                "version": receipt["version"],
                "target": syft_qualification["target"]["key"],
                "platform": receipt["platform"],
                "release_commit": receipt["release_commit"],
                "installer_source_sha256": hashes[SYFT_INSTALLER],
                "verifier_lock_path": syft_qualification["lock_path"],
                "verifier_lock_sha256": hashes[syft_qualification["lock_path"]],
                "status": "upstream native qualification receipt pinned; signature validation is a separate prerequisite",
            },
            "prepared_files": {name: sha256(data) for name, data in sorted(files.items())},
        }
        receipt_bytes = canonical(preparation)
        # The receipt is separate from the three verifier inputs. Its parent
        # must already be present in the evidence artifact area.
        checked_path(preparation_receipt.parent, "preparation receipt parent")
        write_exclusive(preparation_receipt, receipt_bytes)
        receipt_created = True
        print(json.dumps({"status": "derived", "output_dir": str(output), "trusted_consumer_sha": args.trusted_consumer_sha,
                          "verifier_sha256": hashes[VERIFIER], "spdx_schema_sha256": hashes[SCHEMA],
                          "preparation_receipt": str(preparation_receipt),
                          "preparation_receipt_sha256": sha256(receipt_bytes),
                          "files": sorted(files)}, sort_keys=True, separators=(",", ":")))
        return 0
    except BaseException as exc:
        if created and owned_output is not None:
            shutil.rmtree(owned_output, ignore_errors=True)
        if receipt_created and preparation_receipt is not None:
            try:
                preparation_receipt.unlink()
            except OSError:
                pass
        code = exc.__class__.__name__
        print(json.dumps({"status": "rejected", "reason": code}, sort_keys=True, separators=(",", ":")), file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
