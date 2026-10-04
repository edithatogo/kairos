"""Offline regression tests for the local http-cache-semantics mitigation."""

from __future__ import annotations

import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest


SCRIPT = Path(__file__).resolve().parents[1] / "scripts/bootstrap-node-tools/apply_http_cache_fix.py"
SPEC = importlib.util.spec_from_file_location("apply_http_cache_fix", SCRIPT)
assert SPEC and SPEC.loader
patcher = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(patcher)


def fixture_source() -> bytes:
    return b"""class CachePolicy {
    evaluateRequest(req) {
        this._assertRequestHasHeaders(req);

""" + patcher.OLD_EVALUATE + b"""            return this._evaluateRequestMissResult(req);
        }
    }

""" + patcher.OLD_MAX_AGE_DOC + b"""    maxAge() {
""" + patcher.OLD_MAX_AGE_GUARD + b"""        if (this._resHeaders.vary === '*') {
            return 0;
        }

""" + patcher.OLD_PROXY_REVALIDATE + b"""        return 10;
    }

    _useStaleIfError() {
""" + patcher.OLD_STALE_IF_ERROR + b"""    }

    useStaleWhileRevalidate() {
        const swr = toNumberOrZero(this._rescc['stale-while-revalidate']);
""" + patcher.OLD_STALE_WHILE_REVALIDATE + b"""    }

    revalidatedPolicy(request, response) {
        this._assertRequestHasHeaders(request);
""" + patcher.OLD_REVALIDATED_POLICY + b"""            return { policy: this, modified: false, matches: true };
        }
    }
}
"""


class HttpCachePatchTests(unittest.TestCase):
    def setUp(self) -> None:
        self.original_source_sha = patcher.SOURCE_SHA256
        self.original_pr58_sha = patcher.PR58_PATCHED_SHA256
        self.original_patched_sha = patcher.PATCHED_SHA256
        self.source = fixture_source()
        # Unit tests use a compact offline source fixture; full released and PR
        # sources are separately checked against their immutable SHA-256 pins.
        patcher.SOURCE_SHA256 = hashlib.sha256(self.source).hexdigest()
        self.pr58_source = patcher.pr58_source(self.source)
        patcher.PR58_PATCHED_SHA256 = hashlib.sha256(self.pr58_source).hexdigest()
        self.patched = patcher.composed_source(self.pr58_source)
        patcher.PATCHED_SHA256 = hashlib.sha256(self.patched).hexdigest()

    def tearDown(self) -> None:
        patcher.SOURCE_SHA256 = self.original_source_sha
        patcher.PR58_PATCHED_SHA256 = self.original_pr58_sha
        patcher.PATCHED_SHA256 = self.original_patched_sha

    @staticmethod
    def make_package(root: Path, name: str = "http-cache-semantics", version: str = "4.2.0") -> Path:
        package = root / name
        package.mkdir(parents=True)
        (package / "package.json").write_text(
            json.dumps({"name": name, "version": version}), encoding="utf-8"
        )
        (package / "index.js").write_bytes(fixture_source())
        return package

    def test_patches_atomically_preserves_mode_and_is_idempotent(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp) / "node_modules"
            package = self.make_package(root)
            source_path = package / "index.js"
            source_path.chmod(0o640)

            first = patcher.patch_tree(root)
            self.assertEqual(len(first), 1)
            self.assertEqual(source_path.read_bytes(), self.patched)
            self.assertEqual(source_path.stat().st_mode & 0o777, 0o640)
            self.assertEqual(patcher.patch_tree(root)[0][1], hashlib.sha256(self.patched).hexdigest())
            self.assertEqual(source_path.read_bytes(), self.patched)

    def test_rejects_source_drift_without_writing(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp) / "node_modules"
            package = self.make_package(root)
            source_path = package / "index.js"
            source_path.write_bytes(self.source + b"// unexpected drift\n")
            original = source_path.read_bytes()
            with self.assertRaisesRegex(ValueError, "unexpected index.js SHA-256"):
                patcher.patch_tree(root)
            self.assertEqual(source_path.read_bytes(), original)

    def test_upgrades_released_and_exact_pr58_sources_and_is_idempotent(self) -> None:
        self.assertEqual(patcher.patched_source(self.source), self.patched)
        self.assertEqual(patcher.patched_source(self.pr58_source), self.patched)
        self.assertEqual(patcher.patched_source(self.patched), self.patched)

    def test_rejects_unknown_intermediate_source_without_writing(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp) / "node_modules"
            package = self.make_package(root)
            source_path = package / "index.js"
            intermediate = self.pr58_source + b"// unknown modification\n"
            source_path.write_bytes(intermediate)
            with self.assertRaisesRegex(ValueError, "unexpected index.js SHA-256"):
                patcher.patch_tree(root)
            self.assertEqual(source_path.read_bytes(), intermediate)

    def test_rejects_patched_output_hash_mismatch(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp) / "node_modules"
            package = self.make_package(root)
            source_path = package / "index.js"
            original = source_path.read_bytes()
            patcher.PATCHED_SHA256 = "0" * 64
            with self.assertRaisesRegex(ValueError, "patched output SHA-256 mismatch"):
                patcher.patch_tree(root)
            self.assertEqual(source_path.read_bytes(), original)

    def test_rejects_wrong_package_identity_without_writing(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp) / "node_modules"
            package = self.make_package(root, version="4.2.1")
            source_path = package / "index.js"
            with self.assertRaisesRegex(ValueError, "unexpected package identity"):
                patcher.patch_tree(root)
            self.assertEqual(source_path.read_bytes(), self.source)

    def test_rejects_symlinked_source(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp) / "node_modules"
            package = self.make_package(root)
            source_path = package / "index.js"
            source_path.unlink()
            actual = package / "actual.js"
            actual.write_bytes(self.source)
            source_path.symlink_to(actual)
            with self.assertRaisesRegex(ValueError, "symlink package source"):
                patcher.patch_tree(root)

    def test_validates_all_packages_before_first_write(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp) / "node_modules"
            good = self.make_package(root / "a")
            drifted = self.make_package(root / "b")
            (drifted / "index.js").write_bytes(self.source + b"\n")
            with self.assertRaisesRegex(ValueError, "unexpected index.js SHA-256"):
                patcher.patch_tree(root)
            self.assertEqual((good / "index.js").read_bytes(), self.source)

    def test_rejects_package_directory_symlink(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp) / "node_modules"
            root.mkdir()
            real = root / "real-package"
            real.mkdir()
            (real / "package.json").write_text(
                json.dumps({"name": "http-cache-semantics", "version": "4.2.0"}),
                encoding="utf-8",
            )
            (real / "index.js").write_bytes(self.source)
            (root / "http-cache-semantics").symlink_to(real, target_is_directory=True)
            with self.assertRaisesRegex(ValueError, "symlink package directory"):
                patcher.patch_tree(root)


if __name__ == "__main__":
    unittest.main()
