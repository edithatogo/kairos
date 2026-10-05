#!/usr/bin/env python3
"""Fail-closed verifier for the website cache-security evidence bundle."""

from __future__ import annotations

import argparse
import base64
import hashlib
import io
import json
import os
from pathlib import Path, PurePosixPath
import re
import subprocess
import sys
import tempfile
import tarfile


PACKAGE = "http-cache-semantics"
VERSION = "4.3.0"
PATCHED_INDEX_SHA256 = "1c7d64faf562b93a3a989fe931aec6877b18c4f3e18b7822bb8647e1b3e2678e"
LEGACY_HELPER_SHA256 = "1745f11f6b2ae27c47ba00218970192ec0b0034d3d1467b524e411e2c3c9afa4"
REGRESSION_SHA256 = "05d4c9990c5dfc691798336443d28638a405a751076ae7147efc7a19a5392d9c"
FIXTURE_SHA256 = "d75e1e6a11587954da5e2f0e2b5c4b397a16d28cc2f7bdf64e9027fc2fe593ee"
FIXTURE_SRI = "sha512-M5t5LlJpS1UHMjvwRQVdFHvPISGeLAxNcrWuJkeGh0KxsqCHZ1O3NXZU/8x7cD0BDcGW8kapxMKTvwlqrNkHkA=="

REPO_HASHES = {
    "website/package-lock.json": "be5c411f9088545c2883d17bbf7ad04b4f683130826f23340380236c7d90dcb4",
    "website/scripts/apply_http_cache_fix.py": "b999e9cb2e1bd2d292a1ca295a10c5149d9790311ae608ccb01b62f112153855",
    "scripts/bootstrap-node-tools/apply_http_cache_fix.py": LEGACY_HELPER_SHA256,
    "tests/http-cache-security-regression.mjs": REGRESSION_SHA256,
    "tests/fixtures/http-cache-semantics-4.3.0.tgz": FIXTURE_SHA256,
}
UNPINNED_SOURCE_PATHS = ("tests/test_website_http_cache_patch.py",)
COPIED_FILES = (
    "http-cache-semantics/index.js",
    "http-cache-semantics/package.json",
    "http-cache-semantics/LICENSE",
)
LOG_FILES = (
    "node-version.txt",
    "npm-version.txt",
    "adapter-tests.log",
    "install.log",
    "raw-audit.json",
    "mitigation.log",
    "regression.log",
    "negative-control.log",
    "source-commit.txt",
    "SHA256SUMS",
    *COPIED_FILES,
)
CHECKSUM_PATHS = (
    *REPO_HASHES,
    *UNPINNED_SOURCE_PATHS,
    "website/node_modules/http-cache-semantics/index.js",
    "website/scripts/verify_cache_security_evidence.py",
    "tests/test_website_cache_evidence.py",
)


class EvidenceError(ValueError):
    """Evidence is incomplete, inconsistent, or fails its frozen contract."""


def _no_duplicate_keys(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise EvidenceError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def _read_json(path: Path):
    try:
        return json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=_no_duplicate_keys)
    except (OSError, UnicodeError, json.JSONDecodeError) as exc:
        raise EvidenceError(f"cannot parse {path.name}: {exc}") from exc


def _sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def _regular_file(root: Path, relative: str) -> Path:
    path = root / relative
    try:
        path.relative_to(root)
    except ValueError as exc:
        raise EvidenceError(f"path escapes evidence directory: {relative}") from exc
    current = root
    for part in PurePosixPath(relative).parts:
        current = current / part
        if current.is_symlink():
            raise EvidenceError(f"symlink is forbidden: {relative}")
    if not path.is_file():
        raise EvidenceError(f"required regular file is missing: {relative}")
    return path


def _check_real_directory(root: Path):
    if root.is_symlink() or not root.is_dir():
        raise EvidenceError("evidence directory must be a real directory")
    if root != root.resolve():
        raise EvidenceError("evidence directory path must not traverse a symlink")
    if any(part.is_symlink() for part in root.parents):
        raise EvidenceError("evidence directory path must not traverse a symlink")


def _validate_tree(root: Path):
    _check_real_directory(root)
    for base, dirs, files in os.walk(root, followlinks=False):
        for name in dirs + files:
            path = Path(base) / name
            if path.is_symlink():
                raise EvidenceError(f"symlink is forbidden in evidence tree: {path.relative_to(root)}")


def _validate_versions(root: Path):
    for name, pattern in (("node-version.txt", r"v?\d+\.\d+\.\d+"), ("npm-version.txt", r"\d+\.\d+\.\d+")):
        value = (root / name).read_text(encoding="utf-8").strip()
        if not re.fullmatch(pattern, value):
            raise EvidenceError(f"invalid {name} receipt")
    return {
        "node": (root / "node-version.txt").read_text(encoding="utf-8").strip(),
        "npm": (root / "npm-version.txt").read_text(encoding="utf-8").strip(),
    }


def _validate_unittests(text: str):
    matches = re.findall(r"^Ran\s+(\d+)\s+tests?\s+in\s+[^\n]+\n\s*\n(OK(?:\s+\([^\n]*\))?)\s*$", text, re.MULTILINE)
    if not matches or any(int(count) <= 0 for count, _ in matches):
        raise EvidenceError("adapter-tests.log does not show a nonzero unittest run")
    if re.search(r"\b(?:FAILED|ERROR|FAIL)\b", text) or len(matches) != len(re.findall(r"^Ran\s+\d+\s+tests?\s+in\s+[^\n]+$", text, re.MULTILINE)):
        raise EvidenceError("adapter-tests.log does not show successful unittest completion")
    return sum(int(count) for count, _ in matches)


def _validate_audit(root: Path):
    audit = _read_json(root / "raw-audit.json")
    if not isinstance(audit, dict):
        raise EvidenceError("raw-audit.json must contain an object")
    if type(audit.get("auditReportVersion")) is not int or audit["auditReportVersion"] != 2:
        raise EvidenceError("raw-audit.json must be npm audit report version 2")
    metadata = audit.get("metadata")
    counts = metadata.get("vulnerabilities") if isinstance(metadata, dict) else None
    if not isinstance(counts, dict):
        raise EvidenceError("raw-audit.json lacks metadata.vulnerabilities")
    severities = ("info", "low", "moderate", "high", "critical")
    if any(type(counts.get(name)) is not int or counts[name] < 0 for name in (*severities, "total")):
        raise EvidenceError("audit vulnerability counts must be nonnegative integers")
    if counts["total"] != sum(counts[name] for name in severities):
        raise EvidenceError("audit total does not equal severity counts")
    if any(counts[name] != 0 for name in ("moderate", "high", "critical")):
        raise EvidenceError("audit reports moderate or higher vulnerabilities")
    graph = audit.get("vulnerabilities")
    if not isinstance(graph, dict):
        raise EvidenceError("raw-audit.json lacks the npm v2 vulnerabilities object")
    for dependency, record in graph.items():
        if not isinstance(dependency, str) or not isinstance(record, dict):
            raise EvidenceError("raw-audit.json contains malformed vulnerability entries")
        severity = record.get("severity")
        if severity not in severities:
            raise EvidenceError(f"invalid severity in vulnerability entry: {dependency}")
        if severity in ("moderate", "high", "critical") or counts[severity] == 0:
            raise EvidenceError(f"audit vulnerability graph contradicts safe counts: {dependency}")
    for severity in severities:
        graph_count = sum(1 for record in graph.values() if record.get("severity") == severity)
        if graph_count > counts[severity]:
            raise EvidenceError(f"audit severity count is smaller than the reported graph: {severity}")
    if counts["total"] == 0 and graph:
        raise EvidenceError("audit vulnerability graph is nonempty while all counts are zero")
    return {name: counts[name] for name in (*severities, "total")}


def _validate_package(root: Path, repo_root: Path):
    package = _read_json(root / "http-cache-semantics/package.json")
    if not isinstance(package, dict):
        raise EvidenceError("retained package.json must contain an object")
    if package.get("name") != PACKAGE or package.get("version") != VERSION:
        raise EvidenceError("retained package identity is not http-cache-semantics@4.3.0")
    index = root / "http-cache-semantics/index.js"
    if _sha256(index) != PATCHED_INDEX_SHA256:
        raise EvidenceError("retained installed index.js does not match the frozen mitigation hash")
    license_file = root / "http-cache-semantics/LICENSE"
    if license_file.stat().st_size == 0:
        raise EvidenceError("retained package LICENSE is empty")
    archive_path = repo_root / "tests/fixtures/http-cache-semantics-4.3.0.tgz"
    archive_bytes = archive_path.read_bytes()
    if _sha256(archive_path) != FIXTURE_SHA256:
        raise EvidenceError("4.3.0 source fixture archive hash mismatch")
    sri = "sha512-" + base64.b64encode(hashlib.sha512(archive_bytes).digest()).decode("ascii")
    if sri != FIXTURE_SRI:
        raise EvidenceError("4.3.0 source fixture archive SRI mismatch")
    try:
        with tarfile.open(fileobj=io.BytesIO(archive_bytes), mode="r:gz") as archive:
            members = archive.getmembers()
            expected_members = {"package/LICENSE", "package/README.md", "package/index.js", "package/package.json"}
            if {item.name for item in members} != expected_members:
                raise EvidenceError("source fixture archive member list differs from contract")
            extracted = {}
            for item in members:
                member_path = PurePosixPath(item.name)
                if member_path.is_absolute() or ".." in member_path.parts or not item.isfile():
                    raise EvidenceError("source fixture contains an unsafe member")
                if item.name in ("package/LICENSE", "package/package.json"):
                    stream = archive.extractfile(item)
                    if stream is None:
                        raise EvidenceError("source fixture is missing a package member")
                    extracted[item.name] = stream.read()
    except (OSError, tarfile.TarError) as exc:
        raise EvidenceError(f"cannot read pinned source fixture: {exc}") from exc
    archived_package = _read_json_bytes(extracted["package/package.json"], "fixture package.json")
    if package != archived_package:
        raise EvidenceError("retained package.json differs from pinned 4.3.0 archive")
    if license_file.read_bytes() != extracted["package/LICENSE"]:
        raise EvidenceError("retained package LICENSE differs from pinned 4.3.0 archive")
    lock = _read_json(repo_root / "website/package-lock.json")
    if not isinstance(lock, dict) or not isinstance(lock.get("packages"), dict):
        raise EvidenceError("package-lock.json lacks the packages object")
    locked = lock["packages"].get("node_modules/http-cache-semantics", {})
    if not isinstance(locked, dict):
        raise EvidenceError("package-lock.json package entry must be an object")
    if locked.get("version") != VERSION or locked.get("integrity") != FIXTURE_SRI:
        raise EvidenceError("lockfile package version or fixture SRI differs from the frozen contract")
    return {"name": PACKAGE, "version": VERSION, "installed_index_sha256": PATCHED_INDEX_SHA256, "fixture_sri": FIXTURE_SRI}


def _read_json_bytes(data: bytes, label: str):
    try:
        return json.loads(data.decode("utf-8"), object_pairs_hook=_no_duplicate_keys)
    except (UnicodeError, json.JSONDecodeError) as exc:
        raise EvidenceError(f"cannot parse {label}: {exc}") from exc


def _parse_checksums(path: Path):
    entries = {}
    for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        match = re.fullmatch(r"([0-9a-f]{64})  ([^\r\n]+)", line)
        if not match:
            raise EvidenceError(f"malformed SHA256SUMS line {number}")
        digest, name = match.groups()
        posix = PurePosixPath(name)
        if posix.is_absolute() or ".." in posix.parts or "\\" in name or name in entries:
            raise EvidenceError(f"unsafe or duplicate SHA256SUMS path: {name}")
        entries[name] = digest
    if set(entries) != set(CHECKSUM_PATHS):
        raise EvidenceError("SHA256SUMS paths differ from the exact frozen source list")
    return entries


def _validate_checksums(root: Path, repo_root: Path):
    entries = _parse_checksums(root / "SHA256SUMS")
    for name, expected in REPO_HASHES.items():
        source = _regular_file(repo_root, name)
        actual = _sha256(source)
        if actual != expected or entries[name] != actual:
            raise EvidenceError(f"source hash mismatch: {name}")
    for name in UNPINNED_SOURCE_PATHS:
        source = _regular_file(repo_root, name)
        if entries[name] != _sha256(source):
            raise EvidenceError(f"SHA256SUMS does not match current source: {name}")
    installed_index = root / "http-cache-semantics/index.js"
    if entries["website/node_modules/http-cache-semantics/index.js"] != _sha256(installed_index):
        raise EvidenceError("SHA256SUMS installed index hash does not match retained package bytes")
    for name in ("website/scripts/verify_cache_security_evidence.py", "tests/test_website_cache_evidence.py"):
        source = _regular_file(repo_root, name)
        if entries[name] != _sha256(source):
            raise EvidenceError(f"SHA256SUMS does not match current source: {name}")
    return entries


def _payload_hashes(root: Path):
    names = (*LOG_FILES, "COMPLETED.json")
    return {name: _sha256(root / name) for name in names if name != "COMPLETED.json"}


def validate_evidence(evidence_dir, repo_root, expected_commit: str):
    root = Path(evidence_dir).absolute()
    repo = Path(repo_root).resolve()
    _validate_tree(root)
    for name in LOG_FILES:
        _regular_file(root, name)
        if (root / name).stat().st_size == 0:
            raise EvidenceError(f"required evidence file is empty: {name}")

    environment = _validate_versions(root)
    unittest_count = _validate_unittests((root / "adapter-tests.log").read_text(encoding="utf-8"))
    audit_counts = _validate_audit(root)
    install_log = (root / "install.log").read_text(encoding="utf-8")
    if not re.search(r"^added\s+\d+\s+packages?,\s+and audited\s+\d+\s+packages?\s+in\s+.+$", install_log, re.IGNORECASE | re.MULTILINE):
        raise EvidenceError("install.log does not show successful npm ci package installation and audit")
    if re.search(r"^npm\s+(?:ERR!|error)\b|\b(?:error:|failed)\b", install_log, re.IGNORECASE | re.MULTILINE):
        raise EvidenceError("install.log contains a failure indication")
    mitigation = (root / "mitigation.log").read_text(encoding="utf-8")
    if PATCHED_INDEX_SHA256 not in mitigation or f"{PACKAGE}@{VERSION}" not in mitigation:
        raise EvidenceError("mitigation.log lacks the exact installed source hash and package identity")
    regression = (root / "regression.log").read_text(encoding="utf-8")
    if regression.strip() != "248 named cache-security and compatibility cases passed":
        raise EvidenceError("regression.log lacks the exact 248-case pass receipt")
    negative = (root / "negative-control.log").read_text(encoding="utf-8")
    if negative.strip() != "Released-source negative control reproduced unsafe reuse":
        raise EvidenceError("negative-control.log lacks the exact unsafe-reuse reproduction receipt")
    commit_text = (root / "source-commit.txt").read_text(encoding="utf-8").strip()
    if not re.fullmatch(r"[0-9a-f]{40}", expected_commit) or commit_text != expected_commit:
        raise EvidenceError("source-commit.txt does not match the actual source commit")
    package = _validate_package(root, repo)
    checksum_entries = _validate_checksums(root, repo)
    return {
        "schema_version": 1,
        "status": "completed",
        "source_commit": expected_commit,
        "environment": environment,
        "adapter_unittest_cases": unittest_count,
        "named_regression_cases": 248,
        "audit_vulnerabilities": audit_counts,
        "package": package,
        "source_identities": {name: checksum_entries[name] for name in CHECKSUM_PATHS},
        "payload_sha256": _payload_hashes(root),
    }


def complete_evidence(evidence_dir, repo_root, expected_commit: str):
    root = Path(evidence_dir).absolute()
    _check_real_directory(root)
    marker = root / "COMPLETED.json"
    if marker.is_symlink():
        raise EvidenceError("refusing to follow a symlink completion marker")
    try:
        payload = validate_evidence(root, repo_root, expected_commit)
    except Exception:
        if marker.exists():
            if marker.is_file() and not marker.is_symlink():
                marker.unlink()
            else:
                raise EvidenceError("cannot safely remove stale completion marker")
        raise
    data = json.dumps(payload, sort_keys=True, indent=2) + "\n"
    fd, temporary_name = tempfile.mkstemp(prefix=".COMPLETED.", dir=root)
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as stream:
            stream.write(data)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary_name, marker)
    finally:
        if os.path.exists(temporary_name):
            os.unlink(temporary_name)
    return payload


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("evidence_dir", type=Path)
    args = parser.parse_args(argv)
    repo_root = Path(__file__).resolve().parents[2]
    try:
        commit = subprocess.check_output(["git", "-C", str(repo_root), "rev-parse", "HEAD"], text=True).strip()
        payload = complete_evidence(args.evidence_dir, repo_root, commit)
    except (EvidenceError, OSError, subprocess.CalledProcessError, TypeError, AttributeError) as exc:
        print(f"FAIL: {exc}", file=sys.stderr)
        return 1
    print(json.dumps(payload, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
