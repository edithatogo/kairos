#!/usr/bin/env python3
"""Independently validate a completed, authenticated Syft installer run."""
from __future__ import annotations

import argparse
import hashlib
import io
import json
import math
import os
import re
import stat
import sys
import tarfile
from pathlib import Path

VERSION = "1.54.0"
TAG = "v1.54.0"
COMMIT = "cc326e45a6213360266dda4b30cc68095946d676"
REPOSITORY = "anchore/syft"
REF = "refs/heads/main"
IDENTITY = "https://github.com/anchore/syft/.github/workflows/release.yaml@refs/heads/main"
ISSUER = "https://token.actions.githubusercontent.com"
CHECKSUM_SHA256 = "e423344e663d7d14db62e51ddd31e4a5012818ed69e78358391dc487483f839c"
BUNDLE_SHA256 = "6a0dbf94cb89e2fb157f022bed752b3ceb5827cb557a4a4cfb6af67f98b99811"
TARGETS = {
    "linux-amd64": ("linux/amd64", "syft_1.54.0_linux_amd64.tar.gz", "54a87372498168b2d033e876fd41fa4e8035b872699e525a57046e1f2f09c860", "d46a9a61a6ae3d367f0a03748c5e9c59253e586c4388ab26ddcacebc2efa0d92", "syft-linux-verifier.lock", "e8c2913539b2dc4260ef8611e1f21daa56efbdf8b55199c34882808aecd6acea"),
    "darwin-arm64": ("darwin/arm64", "syft_1.54.0_darwin_arm64.tar.gz", "7e0bdad94c569fc6d5785c9a657bbae3d4c4e140ccb5eace3d0b5b6bc2b6dbcf", "835607cdfbdbfc59335b0beadeefc47aa6aab7d3b403c11cfa65627d92a27f61", "syft-darwin-verifier.lock", "bc22323572381258237ff65529b55f37387a3bdfddf1ccf82305d41443ddacf2"),
}
LABELS = ["download-checksums", "download-signature-bundle", "create-verifier-venv", "audit-pip-configuration", "install-hash-locked-verifier", "verify-signed-checksum-document", "download-syft-archive", "extract-syft-archive", "syft-version"]
ARGV_LENGTHS = [6, 6, 4, 3, 10, 16, 6, 6, 4]
MAX_RECEIPT = 1024 * 1024
MAX_LOG = 2 * 1024 * 1024
MAX_ARCHIVE = 64 * 1024 * 1024
MAX_BINARY = 96 * 1024 * 1024
MEMBERS = {"CHANGELOG.md", "LICENSE", "README.md", "syft"}


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def read_regular(path: Path, limit: int) -> bytes:
    """Read a bounded regular file without following any path component symlink."""
    absolute = Path(os.path.abspath(path))
    flags = os.O_RDONLY | getattr(os, "O_DIRECTORY", 0) | getattr(os, "O_NOFOLLOW", 0) | getattr(os, "O_CLOEXEC", 0)
    fd = os.open(os.sep, flags)
    try:
        for part in absolute.parts[1:-1]:
            child = os.open(part, flags, dir_fd=fd)
            os.close(fd)
            fd = child
        leaf = os.open(absolute.name, os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0) | getattr(os, "O_NONBLOCK", 0) | getattr(os, "O_CLOEXEC", 0), dir_fd=fd)
        try:
            info = os.fstat(leaf)
            if not stat.S_ISREG(info.st_mode) or info.st_size > limit:
                raise ValueError(f"not a bounded regular file: {path.name}")
            data = bytearray()
            while len(data) <= limit:
                block = os.read(leaf, min(65536, limit + 1 - len(data)))
                if not block:
                    break
                data.extend(block)
            if len(data) > limit:
                raise ValueError(f"file exceeds size limit: {path.name}")
            return bytes(data)
        finally:
            os.close(leaf)
    finally:
        os.close(fd)


def strict_json(data: bytes):
    depth = 0
    quoted = False
    escaped = False
    for char in data:
        if quoted:
            if escaped:
                escaped = False
            elif char == 0x5C:
                escaped = True
            elif char == 0x22:
                quoted = False
            continue
        if char == 0x22:
            quoted = True
        elif char in (0x7B, 0x5B):
            depth += 1
            if depth > 128:
                raise ValueError("JSON nesting limit exceeded")
        elif char in (0x7D, 0x5D):
            depth -= 1
    def pairs(items):
        result = {}
        for key, value in items:
            if key in result:
                raise ValueError("duplicate JSON object key")
            result[key] = value
        return result
    def constant(_):
        raise ValueError("non-finite JSON number")
    def floating(value):
        number = float(value)
        if not math.isfinite(number):
            raise ValueError("non-finite JSON number")
        return number
    try:
        return json.loads(data, object_pairs_hook=pairs, parse_constant=constant, parse_float=floating)
    except RecursionError as exc:
        raise ValueError("JSON nesting limit exceeded") from exc


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def validate(output: Path, target_key: str, repo: Path) -> dict:
    require(target_key in TARGETS, "unsupported target")
    platform_name, asset, archive_hash, pinned_binary_hash, lock_name, lock_hash = TARGETS[target_key]
    evidence = output / "evidence" / "receipt.json"
    receipt = strict_json(read_regular(evidence, MAX_RECEIPT))
    require(isinstance(receipt, dict), "receipt must be an object")
    installer = repo / "scripts/supply_chain/install_verified_syft.py"
    lock_path = repo / "scripts/supply_chain" / lock_name
    source_bytes = read_regular(installer, 2 * 1024 * 1024)
    lock_bytes = read_regular(lock_path, 256 * 1024)
    require(digest(source_bytes) == receipt.get("installer_source_sha256"), "installer source hash differs from trusted checkout")
    require(digest(lock_bytes) == lock_hash, "verifier lock differs from pinned lock")
    require(digest(lock_bytes) == receipt.get("verifier", {}).get("lock_sha256"), "receipt lock hash mismatch")
    require(read_regular(output / "verifier.lock", 256 * 1024) == lock_bytes, "retained verifier lock differs from trusted checkout")
    require(receipt.get("schema") == "kairos.verified-syft-installer.v1" and receipt.get("result") == "pass", "receipt schema/result mismatch")
    for key, expected in {"version": VERSION, "release_tag": TAG, "release_commit": COMMIT, "repository": REPOSITORY, "ref": REF,
                          "certificate_identity": IDENTITY, "issuer_policy": ISSUER, "target": target_key, "platform": platform_name,
                          "authenticated_asset": asset, "release_checksum_sha256": CHECKSUM_SHA256, "release_bundle_sha256": BUNDLE_SHA256,
                          "signed_asset_sha256": archive_hash, "binary_path": "bin/syft"}.items():
        require(receipt.get(key) == expected, f"receipt {key} mismatch")
    require(receipt.get("verifier") == {"sigstore": "4.5.0", "lock_path": lock_name, "lock_sha256": lock_hash}, "verifier identity mismatch")
    commands = receipt.get("commands")
    require(isinstance(commands, list) and len(commands) == len(LABELS), "command list must contain exactly nine records")
    for index, (record, label) in enumerate(zip(commands, LABELS, strict=True)):
        require(isinstance(record, dict) and record.get("label") == label and type(record.get("exit_status")) is int and record["exit_status"] == 0, f"command {index} failed or mislabeled")
        argv = record.get("argv")
        require(isinstance(argv, list) and argv and all(isinstance(arg, str) for arg in argv), f"command {index} argv malformed")
        require(len(argv) == ARGV_LENGTHS[index] and all("\x00" not in arg for arg in argv), f"command {index} argv length or content invalid")
        log = output / "logs" / f"{index:02d}-{label}.log"
        log_bytes = read_regular(log, MAX_LOG)
        require(len(log_bytes) == record.get("log_bytes") and digest(log_bytes) == record.get("log_sha256"), f"command {label} log mismatch")
        for stream_name in ("stdout_sha256", "stderr_sha256", "log_sha256"):
            require(isinstance(record.get(stream_name), str) and re.fullmatch(r"[0-9a-f]{64}", record[stream_name]) is not None, f"command {label} {stream_name} invalid")
        require(type(record.get("log_bytes")) is int and record["log_bytes"] == len(log_bytes), f"command {label} log byte count invalid")
        if "log_path" in record:
            require(record["log_path"] == f"logs/{index:02d}-{label}.log", f"command {label} log path mismatch")
        expected_output = str(output)
        if label in {"download-checksums", "download-signature-bundle", "download-syft-archive", "extract-syft-archive"}:
            require(Path(argv[1]).name == "install_verified_syft.py", f"{label} did not use the reviewed installer")
        if label == "download-checksums":
            require(argv[2:] == ["--fetch-internal", "https://github.com/anchore/syft/releases/download/v1.54.0/syft_1.54.0_checksums.txt", f"{expected_output}/downloads/syft-checksums.txt", "65536"], "checksum download command arguments mismatch")
        elif label == "download-signature-bundle":
            require(argv[2:] == ["--fetch-internal", "https://github.com/anchore/syft/releases/download/v1.54.0/syft_1.54.0_checksums.txt.sigstore.json", f"{expected_output}/downloads/syft-checksums.sigstore.json", "2097152"], "bundle download command arguments mismatch")
        elif label == "create-verifier-venv":
            require(argv[1:] == ["-m", "venv", f"{expected_output}/verifier"], "verifier environment creation command mismatch")
        elif label == "audit-pip-configuration":
            require(Path(argv[0]).name == "python" and argv[1:] == [f"{expected_output}/pip-config-audit.py", f"{expected_output}/pip-canary.ini"], "pip configuration audit command mismatch")
        elif label == "install-hash-locked-verifier":
            require(argv[1:] == ["-m", "pip", "--isolated", "--disable-pip-version-check", "--no-input", "install", "--require-hashes", "-r", f"{expected_output}/verifier.lock"], "hash-locked verifier install command mismatch")
        if label == "verify-signed-checksum-document":
            require(argv[1:] == ["-m", "sigstore", "verify", "github", "--bundle", f"{expected_output}/downloads/syft-checksums.sigstore.json", "--cert-identity", IDENTITY, "--sha", COMMIT, "--repository", REPOSITORY, "--ref", REF, f"{expected_output}/downloads/syft-checksums.txt"], "Sigstore verification command does not match pinned release identity")
            require(log_bytes == f"OK: {expected_output}/downloads/syft-checksums.txt\n".encode(), "Sigstore verifier did not report successful identity verification")
            require(record["stdout_sha256"] == digest(b"") and record["stderr_sha256"] == digest(log_bytes), "Sigstore output stream digests mismatch")
        elif label == "download-syft-archive":
            require(argv[2:] == ["--fetch-internal", f"https://github.com/anchore/syft/releases/download/{TAG}/{asset}", f"{expected_output}/downloads/{asset}", str(MAX_ARCHIVE)], "archive download command arguments mismatch")
        elif label == "extract-syft-archive":
            require(argv[2:] == ["--extract-internal", f"{expected_output}/downloads/{asset}", f"{expected_output}/extract", archive_hash], "archive extraction command arguments mismatch")
        elif label == "syft-version":
            require(argv == [f"{expected_output}/bin/syft", "version", "-o", "json"], "Syft version command mismatch")
    archive_receipt = receipt.get("archive")
    require(isinstance(archive_receipt, dict), "archive report missing")
    require(archive_receipt.get("member_count") == 4 and set(archive_receipt.get("members", {})) == MEMBERS, "archive member profile mismatch")
    require(archive_receipt.get("archive_sha256") == archive_hash and receipt.get("archive_sha256") == archive_hash, "archive digest mismatch")
    require(archive_receipt.get("binary_sha256") == receipt.get("binary_sha256") == archive_receipt["members"].get("syft"), "binary digest linkage mismatch")
    if pinned_binary_hash:
        require(receipt.get("binary_sha256") == pinned_binary_hash, "binary digest differs from pinned target")
    archive_path = output / "downloads" / asset
    binary_path = output / "bin/syft"
    require(digest(read_regular(output / "downloads/syft-checksums.txt", 64 * 1024)) == CHECKSUM_SHA256, "signed checksum document bytes mismatch")
    require(digest(read_regular(output / "downloads/syft-checksums.sigstore.json", 2 * 1024 * 1024)) == BUNDLE_SHA256, "Sigstore bundle bytes mismatch")
    require(digest(read_regular(archive_path, MAX_ARCHIVE)) == archive_hash, "downloaded archive bytes mismatch")
    archive_bytes = read_regular(archive_path, MAX_ARCHIVE)
    extracted_members = {}
    expanded = 0
    with tarfile.open(fileobj=io.BytesIO(archive_bytes), mode="r:gz") as archive:
        for index, member in enumerate(archive):
            require(index < 5, "archive contains too many members")
            require(member.name in MEMBERS and member.name not in extracted_members, "archive member name is unexpected or duplicated")
            require(member.isfile() and member.size >= 0 and member.size <= MAX_BINARY, "archive member is not a bounded regular file")
            expanded += member.size
            require(expanded <= 128 * 1024 * 1024, "archive expansion exceeds bound")
            stream = archive.extractfile(member)
            require(stream is not None, "archive member could not be read")
            content = stream.read(member.size + 1)
            require(len(content) == member.size, "archive member length mismatch")
            extracted_members[member.name] = digest(content)
    require(set(extracted_members) == MEMBERS and extracted_members == archive_receipt["members"], "archive bytes do not match exact reported four-member profile")
    require(digest(read_regular(binary_path, MAX_BINARY)) == receipt.get("binary_sha256"), "installed executable bytes mismatch")
    extract_log = read_regular(output / "logs/07-extract-syft-archive.log", MAX_LOG)
    extraction = strict_json(extract_log)
    require(isinstance(extraction, dict) and extraction.get("archive") == archive_receipt, "extraction log archive report differs from receipt")
    require(extraction.get("binary") == "syft", "extraction log binary mismatch")
    probe = receipt.get("version_probe")
    require(isinstance(probe, dict) and probe.get("application") == "syft" and probe.get("version") == VERSION and probe.get("gitCommit") == COMMIT and probe.get("platform") == platform_name and probe.get("gitDescription") == TAG, "Syft version probe mismatch")
    version_log = strict_json(read_regular(output / "logs/08-syft-version.log", MAX_LOG))
    require(version_log == probe, "version command log differs from receipt probe")
    require(isinstance(receipt.get("python_toolchain"), dict) and receipt["python_toolchain"].get("version", "").startswith("3.14.8 "), "Python toolchain was not 3.14.8")
    return {"schema": "kairos.syft-installation-validation.v1", "result": "pass", "target": target_key,
            "platform": platform_name, "version": VERSION, "installer_source_sha256": digest(source_bytes),
            "verifier_lock_sha256": digest(lock_bytes), "receipt_sha256": digest(read_regular(evidence, MAX_RECEIPT)),
            "archive_sha256": archive_hash, "binary_sha256": receipt["binary_sha256"], "validated_commands": len(commands),
            "validated_logs": len(commands), "validated_members": sorted(MEMBERS)}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--target", choices=sorted(TARGETS), default="linux-amd64")
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--repo", type=Path, default=Path(__file__).resolve().parents[2])
    args = parser.parse_args()
    try:
        result = validate(args.output_dir, args.target, args.repo)
    except (OSError, ValueError, KeyError, TypeError, RecursionError) as exc:
        print(f"Syft receipt validation failed: {exc}", file=sys.stderr)
        return 1
    print(json.dumps(result, sort_keys=True, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
