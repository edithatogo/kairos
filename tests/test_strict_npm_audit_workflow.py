import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
WORKFLOW = ROOT / ".github/workflows/package-dry-run.yml"
PRIVATE_FIXTURE = ROOT / "tests/http-cache-private-vendor-regression.mjs"
LEGACY_FIXTURE = ROOT / "tests/http-cache-legacy-60-regression.mjs"
PRIVATE_FIXTURE_SHA256 = "05d4c9990c5dfc691798336443d28638a405a751076ae7147efc7a19a5392d9c"
LEGACY_FIXTURE_SHA256 = "298c3537d14c19afdc188cde554596d4f2fc4f95c56d387b79af9ab851f510b2"


def npm_job(text):
    start = text.index("  npm-package:\n")
    end = text.index("  nuget-package:\n", start)
    return text[start:end]


class StrictNpmAuditWorkflowTests(unittest.TestCase):
    def setUp(self):
        self.text = WORKFLOW.read_text()
        self.job = npm_job(self.text)

    def test_pull_request_filters_cover_policy_vendor_and_regressions(self):
        for path in (
            "scripts/bootstrap-node-tools/run_strict_npm_audit_gate.py",
            "tests/test_strict_npm_audit_runner.py",
            "tests/test_strict_npm_audit_workflow.py",
            "tests/http-cache-private-vendor-regression.mjs",
            "vendor/http-cache-semantics-kairos-prototype/**",
            "vendor/http-cache-semantics-kairos-prototype-0.1.0.tgz",
            "scripts/bootstrap-node-tools/package-lock.json",
            "tests/http-cache-legacy-60-regression.mjs",
        ):
            self.assertIn(f"      - '{path}'", self.text)
        self.assertIn("  workflow_dispatch:", self.text)
        self.assertIn("  pull_request:", self.text)

    def test_node_binary_is_resolved_before_both_suites_and_reused(self):
        setup = self.job.index("uses: actions/setup-node@820762786026740c76f36085b0efc47a31fe5020")
        derive = self.job.index("- name: Resolve and hash setup-node executable")
        legacy = self.job.index("- name: Run preserved 60-case cache regression")
        private = self.job.index("- name: Run qualified 248-case private-vendor regression")
        self.assertLess(setup, derive)
        self.assertLess(derive, legacy)
        self.assertLess(derive, private)
        self.assertIn('KAIROS_NODE_BIN="$(realpath "$(command -v node)")"', self.job)
        self.assertIn('KAIROS_NODE_SHA256="$(sha256sum "$KAIROS_NODE_BIN" | awk', self.job)
        self.assertIn('>> "$GITHUB_ENV"', self.job)
        self.assertIn('run: |\n          "$KAIROS_NODE_BIN" tests/http-cache-legacy-60-regression.mjs', self.job)
        self.assertIn('run: |\n          "$KAIROS_NODE_BIN" tests/http-cache-private-vendor-regression.mjs scripts/bootstrap-node-tools/node_modules/http-cache-semantics/index.js', self.job)
        self.assertIn('run: python3 scripts/bootstrap-node-tools/run_strict_npm_audit_gate.py --node-path "$KAIROS_NODE_BIN" --node-sha256 "$KAIROS_NODE_SHA256"', self.job)

    def test_strict_success_copy_is_ordered_and_failure_stays_closed(self):
        marker = self.job.index("- name: Create fail-closed npm audit marker")
        strict = self.job.index("- name: Run strict private-tree npm audit")
        copy = self.job.index("- name: Publish strict success receipt at root")
        upload = self.job.index("- name: Retain raw npm audit and policy classification")
        self.assertLess(marker, strict)
        self.assertLess(strict, copy)
        self.assertLess(copy, upload)
        self.assertIn('"classification": "failed"', self.job)
        self.assertIn('"status": "not_executed"', self.job)
        self.assertIn('"record_kind": "initial_pre_run_fail_closed_marker"', self.job)
        copy_step = self.job[copy:upload]
        self.assertIn("if: success()", copy_step)
        self.assertIn("cp -- artifacts/npm-audit-gate/strict/receipt.json artifacts/npm-audit-gate/receipt.json", copy_step)
        self.assertIn("cmp -s artifacts/npm-audit-gate/strict/receipt.json artifacts/npm-audit-gate/receipt.json", copy_step)
        self.assertIn("if: always()", self.job[upload:])
        self.assertIn("artifacts/npm-audit-gate/", self.job[upload:])
        self.assertIn("retention-days: 7", self.job[upload:])

    def test_private_suite_is_exact_qualified_copy_and_legacy_fixture_is_unchanged(self):
        self.assertEqual(hashlib.sha256(PRIVATE_FIXTURE.read_bytes()).hexdigest(), PRIVATE_FIXTURE_SHA256)
        self.assertEqual(hashlib.sha256(LEGACY_FIXTURE.read_bytes()).hexdigest(), LEGACY_FIXTURE_SHA256)
        self.assertEqual(PRIVATE_FIXTURE.read_bytes()[:2], b"im")

    def test_job_no_longer_runs_exception_or_old_patch_gate(self):
        for forbidden in (
            "apply_http_cache_fix.py",
            "run_npm_audit_gate.py",
            "test_npm_audit_exception.py",
            "test_npm_audit_runner.py",
        ):
            self.assertNotIn(forbidden, self.job)
        self.assertIn("node-version: '22.22.2'", self.job)
        self.assertIn("python-version: '3.14'", self.job)
        self.assertIn("npm ci --ignore-scripts", self.job)
        self.assertIn("npm run build", self.job)
        self.assertIn("npm pack --ignore-scripts", self.job)

    def test_stale_exception_artifact_cannot_allow_a_real_finding(self):
        spec = importlib.util.spec_from_file_location(
            "strict_gate_for_workflow_contract",
            ROOT / "scripts/bootstrap-node-tools/run_strict_npm_audit_gate.py",
        )
        gate = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(gate)
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            expected_sources = {}
            for name in gate.EXPECTED_SOURCE_SHA256:
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text("fixture:" + name)
                expected_sources[name] = hashlib.sha256(path.read_bytes()).hexdigest()
            stale = root / "conductor/tracks/20-openssf-supply-chain-institutional-trust/exceptions/EXC-193.json"
            stale.parent.mkdir(parents=True)
            stale.write_text('{"status":"approved","scope":"stale fixture only"}')
            node = root / "tools/node"
            node.parent.mkdir()
            node.write_bytes(b"fixture node")
            npm = root / "scripts/bootstrap-node-tools/node_modules/npm/bin/npm-cli.js"
            npm.parent.mkdir(parents=True)
            npm.write_bytes(b"fixture npm cli")
            node_sha = hashlib.sha256(node.read_bytes()).hexdigest()
            npm_sha = hashlib.sha256(npm.read_bytes()).hexdigest()
            report = {
                "auditReportVersion": 2,
                "vulnerabilities": {"unsafe": {"severity": "high"}},
                "metadata": {
                    "vulnerabilities": {key: 0 for key in gate.VULNERABILITY_COUNTERS},
                    "dependencies": {key: 0 for key in gate.DEPENDENCY_COUNTERS},
                },
            }

            def execute(argv, **kwargs):
                if "audit" in argv:
                    return subprocess.CompletedProcess(argv, 1, json.dumps(report), "")
                if argv[0] == "git":
                    stdout = gate.SOURCE_COMMIT + "\n"
                elif argv[-1] == "--version" and len(argv) > 2:
                    stdout = "12.1.0\n"
                elif argv[-1] == "--version":
                    stdout = "v26.10.0\n"
                else:
                    stdout = "resolved\n"
                return subprocess.CompletedProcess(argv, 0, stdout, "")

            output = root / "proof"
            result = gate.run_gate(
                root,
                output,
                node_path=node,
                node_sha256=node_sha,
                _execute=execute,
                _expected_source_sha256=expected_sources,
                _expected_npm_cli_sha256=npm_sha,
            )
            receipt = json.loads((output / "receipt.json").read_text())
            raw = json.loads((output / "raw-audit.stdout").read_text())
            self.assertTrue(stale.is_file())
            self.assertEqual(result, 1)
            self.assertEqual(receipt["classification"], "failed")
            self.assertEqual(receipt["raw_audit"]["status"], "executed")
            self.assertEqual(raw["vulnerabilities"]["unsafe"]["severity"], "high")


if __name__ == "__main__":
    unittest.main()
