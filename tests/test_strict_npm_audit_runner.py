import copy
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import unittest

spec = importlib.util.spec_from_file_location(
    "strict_gate",
    Path(__file__).resolve().parents[1] / "scripts/bootstrap-node-tools/run_strict_npm_audit_gate.py",
)
gate = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gate)

REPORT = {
    "auditReportVersion": 2,
    "vulnerabilities": {},
    "metadata": {
        "vulnerabilities": {name: 0 for name in gate.VULNERABILITY_COUNTERS},
        "dependencies": {name: 0 for name in gate.DEPENDENCY_COUNTERS},
    },
}


class StrictAuditTests(unittest.TestCase):
    def test_zero_report_requires_integer_zero_exit_and_clean_stderr(self):
        self.assertEqual(gate.classify(json.dumps(REPORT), "", 0), "passed_zero_reported_vulnerabilities")
        for code, stderr in [(1, ""), (0, "warning"), (None, ""), (False, ""), (0.0, "")]:
            with self.subTest(code=code, stderr=stderr), self.assertRaises(ValueError):
                gate.classify(json.dumps(REPORT), stderr, code)

    def test_vulnerability_and_dependency_counter_schemas_are_exact(self):
        for key in gate.VULNERABILITY_COUNTERS:
            for value in [None, 1, True, "0", -1, 0.0]:
                report = copy.deepcopy(REPORT)
                report["metadata"]["vulnerabilities"][key] = value
                with self.subTest(kind="vulnerability", key=key, value=value), self.assertRaises(ValueError):
                    gate.classify(json.dumps(report), "", 0)
        for key in gate.DEPENDENCY_COUNTERS:
            for value in [None, True, "0", -1, 0.0]:
                report = copy.deepcopy(REPORT)
                report["metadata"]["dependencies"][key] = value
                with self.subTest(kind="dependency", key=key, value=value), self.assertRaises(ValueError):
                    gate.classify(json.dumps(report), "", 0)
        for kind in ("vulnerabilities", "dependencies"):
            report = copy.deepcopy(REPORT)
            report["metadata"][kind]["unexpected"] = 1
            with self.subTest(kind=kind, extra="nonzero"), self.assertRaises(ValueError):
                gate.classify(json.dumps(report), "", 0)
            del report["metadata"][kind]["unexpected"]
            report["metadata"][kind]["unexpected"] = 0
            with self.subTest(kind=kind, extra="zero"), self.assertRaises(ValueError):
                gate.classify(json.dumps(report), "", 0)

    def test_malformed_missing_duplicate_and_nonempty_reports_fail(self):
        reports = ["", "not json", "[]", '{"auditReportVersion":2,"auditReportVersion":2}', "{}"]
        for key in ("vulnerabilities", "metadata", "auditReportVersion"):
            report = copy.deepcopy(REPORT)
            del report[key]
            reports.append(json.dumps(report))
        for key in ("vulnerabilities", "dependencies"):
            report = copy.deepcopy(REPORT)
            del report["metadata"][key]
            reports.append(json.dumps(report))
        report = copy.deepcopy(REPORT)
        report["vulnerabilities"] = {"unsafe": {"severity": "high"}}
        reports.append(json.dumps(report))
        for report in reports:
            with self.subTest(report=report), self.assertRaises((ValueError, AttributeError, TypeError)):
                gate.classify(report, "", 0)

    def fixture(self, root):
        source_paths = {}
        for name in gate.EXPECTED_SOURCE_SHA256:
            path = root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("fixture:" + name)
            source_paths[name] = hashlib.sha256(path.read_bytes()).hexdigest()
        npm_cli = root / "scripts/bootstrap-node-tools/node_modules/npm/bin/npm-cli.js"
        npm_cli.parent.mkdir(parents=True, exist_ok=True)
        npm_cli.write_text("fixture npm cli")
        node = root / "tools/node"
        node.parent.mkdir(parents=True, exist_ok=True)
        node.write_text("fixture node executable")
        return source_paths, npm_cli, node

    def run_fixture(self, root, output, mode="success"):
        source_hashes, npm_cli, node = self.fixture(root)
        calls = []
        npm_hash = hashlib.sha256(npm_cli.read_bytes()).hexdigest()
        node_hash = hashlib.sha256(node.read_bytes()).hexdigest()

        def execute(argv, **kwargs):
            calls.append((list(argv), kwargs))
            if "audit" in argv:
                self.assertIn("--update-notifier=false", argv)
                if mode == "timeout":
                    raise subprocess.TimeoutExpired(argv, 180, output=b"partial", stderr=b"timeout")
                if mode == "start":
                    raise OSError("missing executable")
                report = copy.deepcopy(REPORT)
                if mode == "finding":
                    report["vulnerabilities"] = {"unsafe": {}}
                result = subprocess.CompletedProcess(argv, 1 if mode == "exit" else 0, json.dumps(report), "")
                if mode == "npm-drift":
                    npm_cli.write_text("changed npm cli")
                if mode == "source-drift":
                    (root / next(iter(source_hashes))).write_text("changed source")
                return result
            if mode == "validator" and argv[-1].endswith("validate_npm_cli.mjs"):
                return subprocess.CompletedProcess(argv, 1, "", "resolution failed")
            if argv[-1] == "--version" and len(argv) > 2:
                value = "12.1.0\n"
            elif argv[-1] == "--version":
                value = "v26.10.0\n"
            elif argv[0] == "git":
                value = gate.SOURCE_COMMIT + "\n"
            else:
                value = "resolved\n"
            return subprocess.CompletedProcess(argv, 0, value, "")

        result = gate.run_gate(
            root,
            output,
            node_path=node,
            node_sha256=node_hash,
            _execute=execute,
            _expected_source_sha256=source_hashes,
            _expected_npm_cli_sha256=npm_hash,
        )
        return result, calls, node, npm_cli, node_hash, npm_hash, source_hashes

    def test_receipts_bind_tools_sources_and_retain_command_failures(self):
        for mode in ("success", "finding", "exit", "timeout", "start", "validator", "npm-drift", "source-drift"):
            with self.subTest(mode=mode), tempfile.TemporaryDirectory() as temp:
                root = Path(temp)
                output = root / "proof"
                result, calls, node, npm_cli, node_hash, npm_hash, source_hashes = self.run_fixture(root, output, mode)
                receipt = json.loads((output / "receipt.json").read_text())
                passed = mode == "success"
                self.assertEqual(result, 0 if passed else 1)
                self.assertEqual(receipt["classification"], "passed_zero_reported_vulnerabilities" if passed else "failed")
                self.assertEqual(receipt["raw_audit"]["status"], "not_executed" if mode == "validator" else "failed_execution" if mode in ("timeout", "start") else "executed")
                if mode in ("npm-drift", "source-drift"):
                    self.assertIn("mismatch", receipt["reason"])
                if mode == "success":
                    self.assertEqual(receipt["source_sha256_expected"], source_hashes)
                    self.assertEqual(receipt["tool_hashes"]["node_sha256"], node_hash)
                    self.assertEqual(receipt["tool_hashes"]["npm_cli_sha256"], npm_hash)
                    self.assertTrue(all(call[0][0] != "node" for call in calls if call[0][0] != "git"))
                    self.assertEqual(receipt["npm-version"], "12.1.0")
                self.assertTrue(all(call[1]["cwd"] == root.resolve() and call[1]["timeout"] == 180 for call in calls))
                self.assertTrue((output / "receipt.json").is_file())
                with self.assertRaises(FileExistsError):
                    gate.run_gate(root, output, node_path=node, node_sha256=node_hash)

    def test_source_and_tool_mismatch_fail_before_any_command(self):
        for mismatch in ("source", "node", "npm"):
            with self.subTest(mismatch=mismatch), tempfile.TemporaryDirectory() as temp:
                root = Path(temp)
                source_hashes, npm_cli, node = self.fixture(root)
                calls = []
                expected_sources = dict(source_hashes)
                expected_npm = hashlib.sha256(npm_cli.read_bytes()).hexdigest()
                node_hash = hashlib.sha256(node.read_bytes()).hexdigest()
                if mismatch == "source":
                    expected_sources[next(iter(expected_sources))] = "0" * 64
                elif mismatch == "node":
                    node_hash = "0" * 64
                else:
                    expected_npm = "0" * 64
                result = gate.run_gate(
                    root,
                    root / "proof",
                    node_path=node,
                    node_sha256=node_hash,
                    _execute=lambda argv, **kwargs: calls.append(argv),
                    _expected_source_sha256=expected_sources,
                    _expected_npm_cli_sha256=expected_npm,
                )
                receipt = json.loads((root / "proof/receipt.json").read_text())
                self.assertEqual(result, 1)
                self.assertEqual(calls, [])
                self.assertEqual(receipt["raw_audit"]["status"], "not_executed")
                self.assertEqual(receipt["checks"], [])


if __name__ == "__main__":
    unittest.main()
