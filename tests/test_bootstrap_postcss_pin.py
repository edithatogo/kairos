from __future__ import annotations

import importlib.util
import io
import json
from pathlib import Path
import shutil
import subprocess
import tarfile
import unittest


ROOT = Path(__file__).resolve().parents[1]
PREPARER_PATH = ROOT / "scripts/bootstrap-node-tools/prepare_npm_cli.py"
VALIDATOR_PATH = ROOT / "scripts/bootstrap-node-tools/validate_npm_cli.mjs"
PACKAGE_PATH = ROOT / "scripts/bootstrap-node-tools/package.json"
LOCK_PATH = ROOT / "scripts/bootstrap-node-tools/package-lock.json"
POSTCSS_VERSION = "7.1.6"
POSTCSS_SRI = "sha512-7qASPzhKF2l2KLboRZux8CCTRMdGiV08vWmyKzPz22qZ7ZjQBOeY7rNzNoCLSUiftJ7HUq0GERHmxw/t0dCdMw=="
QUERY_MEMBER = "package/node_modules/@npmcli/query/package.json"
PARSER_MEMBER = "package/node_modules/postcss-selector-parser/package.json"
NESTED_PARSER_MEMBER = (
    "package/node_modules/@npmcli/query/node_modules/postcss-selector-parser/package.json"
)


def load_preparer():
    spec = importlib.util.spec_from_file_location("prepare_npm_cli", PREPARER_PATH)
    if spec is None or spec.loader is None:
        raise AssertionError("could not load npm preparer")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


PREPARER = load_preparer()


def pinned_lock_entry(**updates):
    entry = {
        "version": POSTCSS_VERSION,
        "resolved": (
            "https://registry.npmjs.org/postcss-selector-parser/-/"
            "postcss-selector-parser-7.1.6.tgz"
        ),
        "integrity": POSTCSS_SRI,
        "license": "MIT",
    }
    entry.update(updates)
    return entry


def package_members(*, query_manifest=None, parser_manifest=None):
    root_manifest = {
        "name": "npm",
        "version": "12.1.0",
        "dependencies": {"@npmcli/query": "^5.0.0"},
        "bundleDependencies": ["make-fetch-happen", "node-gyp"],
    }
    query_manifest = query_manifest or {
        "name": "@npmcli/query",
        "version": "5.0.0",
        "dependencies": {"postcss-selector-parser": "^7.0.0"},
    }
    parser_manifest = parser_manifest or {
        "name": "postcss-selector-parser",
        "version": "7.1.4",
    }
    members = {
        "package/package.json": json.dumps(root_manifest).encode(),
        QUERY_MEMBER: json.dumps(query_manifest).encode(),
        "package/node_modules/@npmcli/query/index.js": b"module.exports = require('postcss-selector-parser')\n",
        PARSER_MEMBER: json.dumps(parser_manifest).encode(),
        "package/node_modules/postcss-selector-parser/index.js": b"module.exports = 'old bundled parser'\n",
        NESTED_PARSER_MEMBER: json.dumps(parser_manifest).encode(),
        "package/node_modules/@npmcli/query/node_modules/postcss-selector-parser/index.js": b"module.exports = 'nested old parser'\n",
    }
    return members


def npm_archive(members):
    buffer = io.BytesIO()
    with tarfile.open(fileobj=buffer, mode="w:gz") as archive:
        for name, content in members.items():
            info = tarfile.TarInfo(name)
            info.size = len(content)
            info.mode = 0o644
            archive.addfile(info, io.BytesIO(content))
    return buffer.getvalue()


def output_members(repacked):
    result = {}
    with tarfile.open(fileobj=io.BytesIO(repacked), mode="r:gz") as archive:
        for member in archive.getmembers():
            if member.isfile():
                stream = archive.extractfile(member)
                if stream is None:
                    raise AssertionError(f"missing repacked file contents: {member.name}")
                result[member.name] = stream.read()
    return result


class BootstrapPostcssPinTests(unittest.TestCase):
    def test_lock_validator_accepts_nested_registry_pin_and_returns_lock_location(self):
        path = "node_modules/npm/node_modules/@npmcli/query/node_modules/postcss-selector-parser"
        accepted = PREPARER.validate_postcss_lock_entries({path: pinned_lock_entry()})
        self.assertEqual(set(accepted), {path})

    def test_lock_validator_rejects_missing_or_invalid_parser_records(self):
        cases = {
            "missing": {},
            "wrong version": {
                "node_modules/npm/node_modules/postcss-selector-parser": pinned_lock_entry(version="7.1.5"),
            },
            "bundled": {
                "node_modules/npm/node_modules/postcss-selector-parser": pinned_lock_entry(inBundle=True),
            },
            "malformed numeric bundle flag": {
                "node_modules/npm/node_modules/postcss-selector-parser": pinned_lock_entry(inBundle=0),
            },
            "malformed string bundle flag": {
                "node_modules/npm/node_modules/postcss-selector-parser": pinned_lock_entry(inBundle="false"),
            },
            "wrong integrity": {
                "node_modules/npm/node_modules/postcss-selector-parser": pinned_lock_entry(integrity="sha512-wrong"),
            },
            "noncanonical registry URL": {
                "node_modules/npm/node_modules/postcss-selector-parser": pinned_lock_entry(
                    resolved="https://example.invalid/postcss-selector-parser-7.1.6.tgz",
                ),
            },
            "noncanonical lock path": {
                "node_modules/../node_modules/postcss-selector-parser": pinned_lock_entry(),
            },
        }
        for label, packages in cases.items():
            with self.subTest(label=label), self.assertRaises(RuntimeError):
                PREPARER.validate_postcss_lock_entries(packages)

    def test_every_parser_lock_location_must_be_pinned_and_unbundled(self):
        safe = "node_modules/npm/node_modules/postcss-selector-parser"
        duplicate = "node_modules/another-package/node_modules/postcss-selector-parser"
        packages = {safe: pinned_lock_entry(), duplicate: pinned_lock_entry(inBundle=True)}
        with self.assertRaises(RuntimeError):
            PREPARER.validate_postcss_lock_entries(packages)

        packages[duplicate] = pinned_lock_entry()
        accepted = PREPARER.validate_postcss_lock_entries(packages)
        self.assertEqual(set(accepted), {safe, duplicate})

    def test_package_override_lock_pin_and_preparer_security_constants_agree(self):
        package = json.loads(PACKAGE_PATH.read_text(encoding="utf-8"))
        lock = json.loads(LOCK_PATH.read_text(encoding="utf-8"))
        self.assertEqual(package.get("overrides", {}).get("postcss-selector-parser"), POSTCSS_VERSION)
        accepted = PREPARER.validate_postcss_lock_entries(lock.get("packages", {}))
        self.assertTrue(accepted, "lock must contain the parser package reached by npm query")
        for path in accepted:
            record = lock["packages"][path]
            self.assertEqual(record.get("version"), package["overrides"]["postcss-selector-parser"])
            self.assertEqual(record.get("integrity"), POSTCSS_SRI)
            self.assertFalse(record.get("inBundle", False))
        self.assertEqual(PREPARER.POSTCSS_PARSER_VERSION, POSTCSS_VERSION)
        self.assertEqual(
            PREPARER.POSTCSS_PARSER_URL,
            "https://registry.npmjs.org/postcss-selector-parser/-/postcss-selector-parser-7.1.6.tgz",
        )
        self.assertEqual(PREPARER.POSTCSS_PARSER_SRI, POSTCSS_SRI)

    def test_repack_recursively_removes_bundled_parser_but_keeps_its_query_parent(self):
        source = npm_archive(package_members())
        repacked = PREPARER.repack(source)
        self.assertEqual(repacked, PREPARER.repack(source))
        retained = output_members(repacked)
        self.assertIn(QUERY_MEMBER, retained)
        self.assertIn("package/node_modules/@npmcli/query/index.js", retained)
        parser_paths = [
            name for name in retained
            if name.endswith("/postcss-selector-parser/package.json")
            or "/postcss-selector-parser/" in name
        ]
        self.assertEqual(parser_paths, [])

        manifest = json.loads(retained["package/package.json"])
        self.assertNotIn("postcss-selector-parser", manifest.get("bundleDependencies", []))
        self.assertNotIn("@npmcli/query", manifest.get("bundleDependencies", []))

    def test_repack_rejects_missing_or_wrong_transitive_parser_source_shape(self):
        cases = {
            "missing query manifest": package_members() | {QUERY_MEMBER: None},
            "wrong query dependency": package_members(query_manifest={
                "name": "@npmcli/query",
                "version": "5.0.0",
                "dependencies": {"postcss-selector-parser": "^6.0.0"},
            }),
            "missing parser manifest": package_members() | {PARSER_MEMBER: None},
            "wrong parser package name": package_members(parser_manifest={
                "name": "different-parser",
                "version": "7.1.4",
            }),
        }
        for label, members in cases.items():
            with self.subTest(label=label), self.assertRaises(RuntimeError):
                PREPARER.repack(npm_archive({k: v for k, v in members.items() if v is not None}))

    def test_validator_self_test_checks_runtime_query_resolution(self):
        node = shutil.which("node")
        self.assertIsNotNone(node, "Node.js is required to exercise npm query runtime resolution")
        result = subprocess.run(
            [node, str(VALIDATOR_PATH), "--self-test"],
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=False,
            timeout=30,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("postcss-selector-parser query resolution: PASS", result.stdout)


if __name__ == "__main__":
    unittest.main()
