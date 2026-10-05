"""Offline completion-gate tests using the pinned upstream archive and adapter."""

from __future__ import annotations

import base64
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import sys
import tarfile
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[1]
VERIFIER_PATH = ROOT / "website/scripts/verify_cache_security_evidence.py"
TEST_SOURCE_ROOT = Path(os.environ.get("KAIROS_CACHE_EVIDENCE_TEST_SOURCE_ROOT", str(ROOT))).resolve()
EXPECTED_COMMIT = "0123456789abcdef0123456789abcdef01234567"

spec = importlib.util.spec_from_file_location("website_cache_evidence_verifier", VERIFIER_PATH)
assert spec and spec.loader
verifier = importlib.util.module_from_spec(spec)
spec.loader.exec_module(verifier)


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def source_bytes(relative: str) -> bytes:
    """Read only repository source files for a controlled temporary fixture."""
    source = TEST_SOURCE_ROOT / relative
    if not source.is_file() or source.is_symlink():
        raise AssertionError(f"test fixture source is missing or unsafe: {source}")
    return source.read_bytes()


class EvidenceFixture:
    def __init__(self, base: Path):
        self.repo = base / "repo"
        self.evidence = base / "evidence"
        self.repo.mkdir()
        self.evidence.mkdir()
        self.sources = {name: source_bytes(name) for name in verifier.CHECKSUM_PATHS if name not in (
            "website/node_modules/http-cache-semantics/index.js",
            "website/scripts/verify_cache_security_evidence.py",
            "tests/test_website_cache_evidence.py",
        )}
        self.sources["website/scripts/verify_cache_security_evidence.py"] = VERIFIER_PATH.read_bytes()
        self.sources["tests/test_website_cache_evidence.py"] = Path(__file__).read_bytes()
        for name, data in self.sources.items():
            target = self.repo / name
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(data)

        package_root = self.evidence / "http-cache-semantics"
        package_root.mkdir()
        fixture = self.sources["tests/fixtures/http-cache-semantics-4.3.0.tgz"]
        self.assert_fixture_integrity(fixture)
        members = self._fixture_members(fixture)
        for name, data in members.items():
            (package_root / name).write_bytes(data)

        adapter_path = self.repo / "website/scripts/apply_http_cache_fix.py"
        spec = importlib.util.spec_from_file_location("test_adapter", adapter_path)
        assert spec and spec.loader
        adapter = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(adapter)
        adapter.patch_tree(self.evidence)

        files = {
            "node-version.txt": "v24.9.0\n",
            "npm-version.txt": "11.6.0\n",
            "adapter-tests.log": "Ran 14 tests in 0.032s\n\nOK\nRan 9 tests in 0.020s\n\nOK\n",
            "install.log": "added 419 packages, and audited 420 packages in 4s\nfound 0 vulnerabilities\n",
            "raw-audit.json": json.dumps({
                "auditReportVersion": 2,
                "metadata": {"vulnerabilities": {"info": 0, "low": 0, "moderate": 0, "high": 0, "critical": 0, "total": 0}},
                "vulnerabilities": {},
            }) + "\n",
            "mitigation.log": (
                f"http-cache-semantics/index.js: {verifier.PATCHED_INDEX_SHA256}\n"
                "LOCAL MITIGATION APPLIED; package identity remains http-cache-semantics@4.3.0 (not an official patched release)\n"
            ),
            "regression.log": "248 named cache-security and compatibility cases passed\n",
            "negative-control.log": "Released-source negative control reproduced unsafe reuse\n",
            "source-commit.txt": EXPECTED_COMMIT + "\n",
        }
        for name, text in files.items():
            (self.evidence / name).write_text(text, encoding="utf-8")
        self.write_checksums()

    @staticmethod
    def assert_fixture_integrity(raw: bytes):
        if sha256(raw) != verifier.FIXTURE_SHA256:
            raise AssertionError("fixture archive SHA-256 mismatch")
        sri = "sha512-" + base64.b64encode(hashlib.sha512(raw).digest()).decode("ascii")
        if sri != verifier.FIXTURE_SRI:
            raise AssertionError("fixture archive SRI mismatch")

    @staticmethod
    def _fixture_members(raw: bytes):
        selected = {
            "package/LICENSE": "LICENSE",
            "package/index.js": "index.js",
            "package/package.json": "package.json",
        }
        result = {}
        with tarfile.open(fileobj=io.BytesIO(raw), mode="r:gz") as archive:
            expected = {"package/LICENSE", "package/README.md", "package/index.js", "package/package.json"}
            items = archive.getmembers()
            if {member.name for member in items} != expected:
                raise AssertionError("unexpected archive member list")
            for member in items:
                if not member.isfile() or Path(member.name).is_absolute() or ".." in Path(member.name).parts:
                    raise AssertionError("unsafe archive member")
                if member.name in selected:
                    stream = archive.extractfile(member)
                    assert stream
                    result[selected[member.name]] = stream.read()
        package = json.loads(result["package.json"])
        if (package.get("name"), package.get("version")) != ("http-cache-semantics", "4.3.0"):
            raise AssertionError("fixture package identity mismatch")
        return result

    def write_checksums(self):
        entries = {}
        for name, data in self.sources.items():
            entries[name] = sha256(data)
        entries["website/node_modules/http-cache-semantics/index.js"] = sha256(
            (self.evidence / "http-cache-semantics/index.js").read_bytes()
        )
        entries["website/scripts/verify_cache_security_evidence.py"] = sha256(VERIFIER_PATH.read_bytes())
        entries["tests/test_website_cache_evidence.py"] = sha256(Path(__file__).read_bytes())
        self.entries = entries
        (self.evidence / "SHA256SUMS").write_text(
            "".join(f"{entries[name]}  {name}\n" for name in verifier.CHECKSUM_PATHS), encoding="utf-8"
        )

    def validate(self):
        return verifier.validate_evidence(self.evidence, self.repo, EXPECTED_COMMIT)

    def complete(self):
        return verifier.complete_evidence(self.evidence, self.repo, EXPECTED_COMMIT)


class WebsiteCacheEvidenceTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        # macOS exposes its system temporary directory through /var -> /private/var.
        # Canonicalize this test-created directory before testing evidence symlinks.
        self.fixture = EvidenceFixture(Path(self.temporary.name).resolve())

    def test_accepts_complete_pinned_evidence_and_writes_payload_hashes(self):
        result = self.fixture.complete()
        marker = json.loads((self.fixture.evidence / "COMPLETED.json").read_text())
        self.assertEqual(result["status"], "completed")
        self.assertEqual(result["source_commit"], EXPECTED_COMMIT)
        self.assertEqual(result["adapter_unittest_cases"], 23)
        self.assertEqual(result["named_regression_cases"], 248)
        self.assertEqual(marker["payload_sha256"]["negative-control.log"], sha256((self.fixture.evidence / "negative-control.log").read_bytes()))

    def test_rejects_each_missing_required_receipt_and_clears_stale_marker(self):
        for name in verifier.LOG_FILES:
            with self.subTest(name=name):
                marker = self.fixture.evidence / "COMPLETED.json"
                marker.write_text("stale", encoding="utf-8")
                path = self.fixture.evidence / name
                saved = path.read_bytes()
                path.unlink()
                with self.assertRaises(verifier.EvidenceError):
                    self.fixture.complete()
                self.assertFalse(marker.exists())
                path.write_bytes(saved)

    def test_rejects_nonzero_audit_severity_and_boolean_count(self):
        path = self.fixture.evidence / "raw-audit.json"
        original = path.read_text()
        for count in (1, True):
            audit = json.loads(original)
            audit["metadata"]["vulnerabilities"]["moderate"] = count
            audit["metadata"]["vulnerabilities"]["total"] = count
            path.write_text(json.dumps(audit))
            with self.assertRaises(verifier.EvidenceError):
                self.fixture.validate()
        path.write_text(original)

    def test_rejects_hidden_graph_finding_with_zero_audit_metadata(self):
        path = self.fixture.evidence / "raw-audit.json"
        audit = json.loads(path.read_text())
        audit["vulnerabilities"] = {"hidden-high": {"severity": "high"}}
        path.write_text(json.dumps(audit))
        with self.assertRaises(verifier.EvidenceError):
            self.fixture.validate()

    def test_rejects_bad_unittest_or_regression_receipts(self):
        tests = self.fixture.evidence / "adapter-tests.log"
        old_tests = tests.read_text()
        tests.write_text("Ran 0 tests in 0.001s\n\nOK\n")
        with self.assertRaises(verifier.EvidenceError):
            self.fixture.validate()
        tests.write_text(old_tests)
        regression = self.fixture.evidence / "regression.log"
        old_regression = regression.read_text()
        regression.write_text("247 named cache-security and compatibility cases passed\n")
        with self.assertRaises(verifier.EvidenceError):
            self.fixture.validate()
        regression.write_text("248 named cache-security and compatibility cases passed\nFAIL later\n")
        with self.assertRaises(verifier.EvidenceError):
            self.fixture.validate()
        regression.write_text(old_regression)

    def test_rejects_missing_or_incorrect_negative_control(self):
        path = self.fixture.evidence / "negative-control.log"
        original = path.read_text()
        path.write_text("released-source negative control did not reproduce\n")
        with self.assertRaises(verifier.EvidenceError):
            self.fixture.validate()
        path.write_text("Released-source negative control reproduced unsafe reuse\nnegative control failed afterward\n")
        with self.assertRaises(verifier.EvidenceError):
            self.fixture.validate()
        path.write_text(original)

    def test_rejects_install_log_without_successful_npm_ci_receipt(self):
        path = self.fixture.evidence / "install.log"
        original = path.read_text()
        path.write_text("audited 420 packages; install command exited with error\n")
        with self.assertRaises(verifier.EvidenceError):
            self.fixture.validate()
        path.write_text(
            "added 419 packages, and audited 420 packages in 4s\nnpm error code ERESOLVE\n",
            encoding="utf-8",
        )
        with self.assertRaises(verifier.EvidenceError):
            self.fixture.validate()
        path.write_text(original)

    def test_rejects_legacy_npm_error_after_successful_install_summary(self):
        path = self.fixture.evidence / "install.log"
        path.write_text("added 419 packages, and audited 420 packages in 4s\nnpm ERR! code ERESOLVE\n")
        with self.assertRaises(verifier.EvidenceError):
            self.fixture.validate()

    def test_rejects_wrong_commit_and_wrong_retained_index_bytes(self):
        commit = self.fixture.evidence / "source-commit.txt"
        commit.write_text("f" * 40 + "\n")
        with self.assertRaises(verifier.EvidenceError):
            self.fixture.validate()
        commit.write_text(EXPECTED_COMMIT + "\n")
        index = self.fixture.evidence / "http-cache-semantics/index.js"
        index.write_bytes(index.read_bytes() + b"tamper")
        with self.assertRaises(verifier.EvidenceError):
            self.fixture.validate()

    def test_rejects_duplicate_and_traversal_manifest_entries(self):
        path = self.fixture.evidence / "SHA256SUMS"
        original = path.read_text()
        first = original.splitlines()[0]
        path.write_text(original + first + "\n")
        with self.assertRaises(verifier.EvidenceError):
            self.fixture.validate()
        path.write_text(original.replace("website/package-lock.json", "../package-lock.json", 1))
        with self.assertRaises(verifier.EvidenceError):
            self.fixture.validate()
        path.write_text(original)

    def test_rejects_symlinks_and_symlink_completion_marker(self):
        package_license = self.fixture.evidence / "http-cache-semantics/LICENSE"
        saved = package_license.read_bytes()
        package_license.unlink()
        package_license.symlink_to(self.fixture.evidence / "install.log")
        with self.assertRaises(verifier.EvidenceError):
            self.fixture.validate()
        package_license.unlink()
        package_license.write_bytes(saved)
        marker = self.fixture.evidence / "COMPLETED.json"
        marker.symlink_to(self.fixture.evidence / "install.log")
        with self.assertRaises(verifier.EvidenceError):
            self.fixture.complete()
        self.assertTrue(marker.is_symlink())

    def test_rejects_symlink_directory_without_unlinking_outside_stale_marker(self):
        alias = Path(self.temporary.name) / "evidence-alias"
        alias.symlink_to(self.fixture.evidence, target_is_directory=True)
        outside_marker = self.fixture.evidence / "COMPLETED.json"
        outside_marker.write_text("stale", encoding="utf-8")
        with self.assertRaises(verifier.EvidenceError):
            verifier.complete_evidence(alias, self.fixture.repo, EXPECTED_COMMIT)
        self.assertEqual(outside_marker.read_text(), "stale")

    def test_rejects_package_identity_lock_sri_and_frozen_source_hash_drift(self):
        package = self.fixture.evidence / "http-cache-semantics/package.json"
        original = package.read_text()
        data = json.loads(original)
        data["version"] = "4.3.1"
        package.write_text(json.dumps(data))
        with self.assertRaises(verifier.EvidenceError):
            self.fixture.validate()
        package.write_text(original)
        lock = self.fixture.repo / "website/package-lock.json"
        lockdata = json.loads(lock.read_text())
        lockdata["packages"]["node_modules/http-cache-semantics"]["integrity"] = "sha512-wrong"
        lock.write_text(json.dumps(lockdata))
        with self.assertRaises(verifier.EvidenceError):
            self.fixture.validate()
        lock.write_bytes(self.fixture.sources["website/package-lock.json"])


if __name__ == "__main__":
    unittest.main()
