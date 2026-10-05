"""Offline tests for the website-only http-cache-semantics 4.3.0 adapter."""

from __future__ import annotations

import base64
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import stat
import sys
import tarfile
import tempfile
import unittest
from unittest import mock


sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
ADAPTER_PATH = ROOT / "website/scripts/apply_http_cache_fix.py"
FIXTURE_PATH = ROOT / "tests/fixtures/http-cache-semantics-4.3.0.tgz"
FIXTURE_SHA256 = "d75e1e6a11587954da5e2f0e2b5c4b397a16d28cc2f7bdf64e9027fc2fe593ee"
FIXTURE_SRI = (
    "sha512-M5t5LlJpS1UHMjvwRQVdFHvPISGeLAxNcrWuJkeGh0KxsqCHZ1O3NXZU/"
    "8x7cD0BDcGW8kapxMKTvwlqrNkHkA=="
)
EXPECTED_SOURCE_SHA256 = "ede1cc404a492fa348eb9d97a3007a0d72aa717bd22cd86a56bd0824c19729ca"
EXPECTED_PR58_SHA256 = "7a23f143046560191aba075d4404821db051118d46d7419a2fd006154a1889eb"
EXPECTED_PATCHED_SHA256 = "1c7d64faf562b93a3a989fe931aec6877b18c4f3e18b7822bb8647e1b3e2678e"

spec = importlib.util.spec_from_file_location("website_apply_http_cache_fix", ADAPTER_PATH)
assert spec and spec.loader
adapter = importlib.util.module_from_spec(spec)
spec.loader.exec_module(adapter)


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def fixture_members() -> dict[str, bytes]:
    raw = FIXTURE_PATH.read_bytes()
    if sha256(raw) != FIXTURE_SHA256:
        raise AssertionError("4.3.0 source fixture SHA-256 mismatch")
    sri = "sha512-" + base64.b64encode(hashlib.sha512(raw).digest()).decode("ascii")
    if sri != FIXTURE_SRI:
        raise AssertionError("4.3.0 source fixture SRI mismatch")

    expected_names = {
        "package/LICENSE",
        "package/README.md",
        "package/index.js",
        "package/package.json",
    }
    selected = {"package/LICENSE", "package/index.js", "package/package.json"}
    result = {}
    with tarfile.open(fileobj=io.BytesIO(raw), mode="r:gz") as archive:
        members = archive.getmembers()
        if {item.name for item in members} != expected_names:
            raise AssertionError("unexpected 4.3.0 fixture member set")
        for item in members:
            path = Path(item.name)
            if path.is_absolute() or ".." in path.parts or not item.isfile():
                raise AssertionError(f"unsafe 4.3.0 fixture member: {item.name}")
            if item.name in selected:
                stream = archive.extractfile(item)
                if stream is None:
                    raise AssertionError(f"missing regular fixture member: {item.name}")
                result[item.name] = stream.read()

    manifest = json.loads(result["package/package.json"])
    if (manifest.get("name"), manifest.get("version"), manifest.get("license")) != (
        "http-cache-semantics",
        "4.3.0",
        "BSD-2-Clause",
    ):
        raise AssertionError("fixture package identity/license mismatch")
    if sha256(result["package/index.js"]) != EXPECTED_SOURCE_SHA256:
        raise AssertionError("fixture index.js source hash mismatch")
    return result


FIXTURE = fixture_members()
SOURCE = FIXTURE["package/index.js"]


def make_package(root: Path, *, rel: str = "", name: str = "http-cache-semantics", version: str = "4.3.0") -> Path:
    package = root / rel / "http-cache-semantics"
    package.mkdir(parents=True)
    manifest = json.loads(FIXTURE["package/package.json"])
    manifest["name"] = name
    manifest["version"] = version
    (package / "package.json").write_text(json.dumps(manifest), encoding="utf-8")
    (package / "index.js").write_bytes(SOURCE)
    (package / "LICENSE").write_bytes(FIXTURE["package/LICENSE"])
    return package


class WebsiteHttpCachePatchTests(unittest.TestCase):
    def test_fixture_identity_integrity_license_and_pr58_intermediate(self) -> None:
        self.assertEqual(adapter.FIXTURE_SHA256, FIXTURE_SHA256)
        self.assertEqual(adapter.FIXTURE_SRI, FIXTURE_SRI)
        self.assertEqual(sha256(SOURCE), EXPECTED_SOURCE_SHA256)
        intermediate = adapter.pr58_source(SOURCE)
        self.assertEqual(sha256(intermediate), EXPECTED_PR58_SHA256)
        self.assertEqual(sha256(adapter.composed_source(intermediate)), EXPECTED_PATCHED_SHA256)
        patched = adapter.patched_source(SOURCE)
        self.assertEqual(sha256(patched), EXPECTED_PATCHED_SHA256)
        self.assertEqual(adapter.patched_source(patched), patched)
        self.assertIn(b"Copyright 2016-2018 Kornel Lesi", FIXTURE["package/LICENSE"])

    def test_patches_real_nested_43_package_atomically_preserving_mode_and_license(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "node_modules"
            package = make_package(root, rel="outer/node_modules")
            source = package / "index.js"
            source.chmod(0o640)
            license_before = (package / "LICENSE").read_bytes()

            result = adapter.patch_tree(root)
            self.assertEqual(len(result), 1)
            self.assertEqual(result[0][1], EXPECTED_PATCHED_SHA256)
            self.assertEqual(sha256(source.read_bytes()), EXPECTED_PATCHED_SHA256)
            self.assertEqual(stat.S_IMODE(source.stat().st_mode), 0o640)
            self.assertEqual((package / "LICENSE").read_bytes(), license_before)
            self.assertEqual(json.loads((package / "package.json").read_text())["version"], "4.3.0")

            before = source.read_bytes()
            again = adapter.patch_tree(root)
            self.assertEqual(source.read_bytes(), before)
            self.assertEqual(again[0][1], EXPECTED_PATCHED_SHA256)

    def test_patches_multiple_valid_nested_packages_and_is_idempotent(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "node_modules"
            first = make_package(root, rel="outer/node_modules")
            second = make_package(root, rel="outer/inner/node_modules")
            first_source = first / "index.js"
            second_source = second / "index.js"
            first_source.chmod(0o640)
            second_source.chmod(0o604)
            licenses = [(first / "LICENSE").read_bytes(), (second / "LICENSE").read_bytes()]

            result = adapter.patch_tree(root)
            self.assertEqual({item[1] for item in result}, {EXPECTED_PATCHED_SHA256})
            self.assertEqual({item[0] for item in result}, {first_source, second_source})
            self.assertEqual(sha256(first_source.read_bytes()), EXPECTED_PATCHED_SHA256)
            self.assertEqual(sha256(second_source.read_bytes()), EXPECTED_PATCHED_SHA256)
            self.assertEqual(stat.S_IMODE(first_source.stat().st_mode), 0o640)
            self.assertEqual(stat.S_IMODE(second_source.stat().st_mode), 0o604)
            self.assertEqual((first / "LICENSE").read_bytes(), licenses[0])
            self.assertEqual((second / "LICENSE").read_bytes(), licenses[1])

            patched = [first_source.read_bytes(), second_source.read_bytes()]
            again = adapter.patch_tree(root)
            self.assertEqual(first_source.read_bytes(), patched[0])
            self.assertEqual(second_source.read_bytes(), patched[1])
            self.assertEqual({item[1] for item in again}, {EXPECTED_PATCHED_SHA256})
            self.assertEqual({item[0] for item in again}, {first_source, second_source})

    def test_rejects_tampered_helper_before_executing_it(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            marker = root / "executed"
            helper = root / "tampered.py"
            helper.write_text(f"from pathlib import Path\nPath({str(marker)!r}).write_text('bad')\n", encoding="utf-8")
            with mock.patch.object(adapter, "_helper_path", return_value=helper):
                with self.assertRaisesRegex(ValueError, "legacy patch helper hash mismatch"):
                    adapter._load_verified_helper()
            self.assertFalse(marker.exists())

    def test_rejects_unknown_source_without_write(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "node_modules"
            package = make_package(root)
            source = package / "index.js"
            source.write_bytes(SOURCE + b"\n// unexpected drift\n")
            original = source.read_bytes()
            with self.assertRaisesRegex(ValueError, "unexpected index.js SHA-256"):
                adapter.patch_tree(root)
            self.assertEqual(source.read_bytes(), original)

    def test_rejects_wrong_package_version_without_write(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "node_modules"
            package = make_package(root, version="4.2.0")
            source = package / "index.js"
            with self.assertRaisesRegex(ValueError, "unexpected package identity"):
                adapter.patch_tree(root)
            self.assertEqual(source.read_bytes(), SOURCE)

    def test_rejects_wrong_package_name_without_write(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "node_modules"
            package = make_package(root, name="other-package")
            source = package / "index.js"
            with self.assertRaisesRegex(ValueError, "unexpected package identity"):
                adapter.patch_tree(root)
            self.assertEqual(source.read_bytes(), SOURCE)

    def test_rejects_missing_package(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            with self.assertRaisesRegex(ValueError, "no http-cache-semantics package"):
                adapter.patch_tree(Path(temporary))

    def test_rejects_symlink_root(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            base = Path(temporary)
            actual = base / "real"
            actual.mkdir()
            alias = base / "alias"
            alias.symlink_to(actual, target_is_directory=True)
            with self.assertRaisesRegex(ValueError, "root must be a real directory"):
                adapter.patch_tree(alias)

    def test_rejects_symlink_package_directory(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "node_modules"
            root.mkdir()
            actual = root / "real-package"
            actual.mkdir()
            (actual / "package.json").write_text(
                '{"name":"http-cache-semantics","version":"4.3.0"}', encoding="utf-8"
            )
            (actual / "index.js").write_bytes(SOURCE)
            (root / "http-cache-semantics").symlink_to(actual, target_is_directory=True)
            with self.assertRaisesRegex(ValueError, "refusing symlink package directory"):
                adapter.patch_tree(root)

    def test_rejects_symlink_package_manifest(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "node_modules"
            package = make_package(root)
            manifest = package / "package.json"
            actual = package / "manifest.json"
            manifest.replace(actual)
            manifest.symlink_to(actual)
            with self.assertRaisesRegex(ValueError, "refusing symlink package source"):
                adapter.patch_tree(root)
            self.assertEqual((package / "index.js").read_bytes(), SOURCE)

    def test_rejects_symlink_source(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "node_modules"
            package = make_package(root)
            source = package / "index.js"
            source.unlink()
            actual = package / "actual.js"
            actual.write_bytes(SOURCE)
            source.symlink_to(actual)
            with self.assertRaisesRegex(ValueError, "refusing symlink package source"):
                adapter.patch_tree(root)

    def test_validates_entire_nested_tree_before_first_write(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "node_modules"
            valid = make_package(root, rel="a")
            invalid = make_package(root, rel="b")
            (invalid / "index.js").write_bytes(SOURCE + b"\n// drift\n")
            before = (valid / "index.js").read_bytes()
            with self.assertRaisesRegex(ValueError, "unexpected index.js SHA-256"):
                adapter.patch_tree(root)
            self.assertEqual((valid / "index.js").read_bytes(), before)

    def test_cli_labels_result_as_local_mitigation(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "node_modules"
            make_package(root)
            proc = __import__("subprocess").run(
                [sys.executable, str(ADAPTER_PATH), str(root)],
                cwd=ROOT,
                text=True,
                capture_output=True,
                check=False,
            )
            self.assertEqual(proc.returncode, 0, proc.stderr)
            self.assertIn("LOCAL MITIGATION APPLIED", proc.stdout)
            self.assertIn("http-cache-semantics@4.3.0", proc.stdout)
            self.assertIn("not an official patched release", proc.stdout)


if __name__ == "__main__":
    unittest.main()
