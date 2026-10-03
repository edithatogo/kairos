"""Offline adversarial tests for the EXC-193 npm audit gate runner."""

from __future__ import annotations

import contextlib
import copy
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest import mock


# CI uses the checkout containing this test. The explicit override lets a worker
# exercise the root checkout's integrated runner from an isolated worktree.
ROOT = Path(
    os.environ.get("KAIROS_NPM_AUDIT_TEST_ROOT", Path(__file__).resolve().parents[1])
).resolve()
RUNNER_PATH = ROOT / "scripts/bootstrap-node-tools/run_npm_audit_gate.py"
EXCEPTIONS = ROOT / "conductor/tracks/20-openssf-supply-chain-institutional-trust/exceptions"


def _load_runner():
    spec = importlib.util.spec_from_file_location("run_npm_audit_gate", RUNNER_PATH)
    if spec is None or spec.loader is None:
        raise ModuleNotFoundError(f"npm audit runner is missing: {RUNNER_PATH}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


runner = _load_runner()


class NpmAuditRunnerTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.policy = json.loads((EXCEPTIONS / "EXC-193-http-cache.json").read_text())
        cls.installed_package = ROOT / "scripts/bootstrap-node-tools/node_modules/http-cache-semantics"
        if not cls.installed_package.is_dir():
            raise RuntimeError("runner tests require the gate's already-installed npm tree")

    def make_source_root(self, directory: Path) -> Path:
        """Build a small offline tree from tracked proofs and installed package bytes."""
        for relative in self.policy["file_sha256"]:
            source = ROOT / relative
            target = directory / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source, target)
        tools = directory / "scripts/bootstrap-node-tools"
        for relative in runner.COPIES:
            package = tools / "node_modules" / relative.removeprefix("node_modules/")
            package.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(self.installed_package / "index.js", package / "index.js")
            shutil.copyfile(self.installed_package / "package.json", package / "package.json")
        return directory

    def test_exact_two_installed_copies_and_pinned_proofs_pass(self):
        with tempfile.TemporaryDirectory() as temp:
            root = self.make_source_root(Path(temp))
            runner.verify_sources(root, copy.deepcopy(self.policy))

    def test_extra_copy_source_drift_identity_and_proof_removal_reject(self):
        cases = ("extra_copy", "source_drift", "identity_drift", "proof_removal", "manifest_removal")
        for case in cases:
            with self.subTest(case=case), tempfile.TemporaryDirectory() as temp:
                root = self.make_source_root(Path(temp))
                policy = copy.deepcopy(self.policy)
                if case == "extra_copy":
                    extra = root / "scripts/bootstrap-node-tools/node_modules/extra/http-cache-semantics"
                    extra.mkdir(parents=True)
                    shutil.copyfile(self.installed_package / "index.js", extra / "index.js")
                    shutil.copyfile(self.installed_package / "package.json", extra / "package.json")
                elif case == "source_drift":
                    (root / "scripts/bootstrap-node-tools/node_modules/http-cache-semantics/index.js").write_bytes(b"changed")
                elif case == "identity_drift":
                    package = root / "scripts/bootstrap-node-tools/node_modules/http-cache-semantics/package.json"
                    value = json.loads(package.read_text())
                    value["version"] = "4.2.1"
                    package.write_text(json.dumps(value))
                elif case == "proof_removal":
                    (root / "scripts/bootstrap-node-tools/package-lock.json").unlink()
                else:
                    del policy["file_sha256"]["tests/test_http_cache_patch.py"]
                with self.assertRaises((ValueError, OSError)):
                    runner.verify_sources(root, policy)

    def test_policy_hash_expiry_and_raw_argv_mutations_reject(self):
        mutations = (
            lambda policy: policy["file_sha256"].__setitem__("tests/test_http_cache_patch.py", "0" * 64),
            lambda policy: policy.__setitem__("patched_index_sha256", "0" * 64),
            lambda policy: policy.__setitem__("expires_at", "2027-10-10T00:00:00+10:00"),
            lambda policy: policy["raw_audit_command"].append("--ignore-scripts"),
        )
        for mutate in mutations:
            with self.subTest(mutation=mutate), tempfile.TemporaryDirectory() as temp:
                root = self.make_source_root(Path(temp))
                policy = copy.deepcopy(self.policy)
                mutate(policy)
                with self.assertRaises(ValueError):
                    runner.verify_sources(root, policy)

    def test_duplicate_json_keys_are_rejected(self):
        with self.assertRaisesRegex(ValueError, "duplicate JSON key"):
            runner.read_json('{"repository":"approved","repository":"attacker"}')

    def _dispatch(self, env: dict[str, str], event: dict):
        with tempfile.TemporaryDirectory() as temp:
            event_path = Path(temp) / "event.json"
            event_path.write_text(json.dumps(event))
            environment = {**env, "GITHUB_EVENT_PATH": str(event_path)}
            with mock.patch.dict(os.environ, environment, clear=True):
                return runner.execution_context(copy.deepcopy(self.policy))

    def test_dispatch_rejects_wrong_branch_and_fork_pull_request(self):
        base = {
            "repository": {"full_name": "edithatogo/kairos"},
            "pull_request": {
                "number": 193,
                "head": {
                    "ref": runner.EXPECTED_BRANCH,
                    "repo": {"full_name": "edithatogo/kairos"},
                },
            },
        }
        env = {"GITHUB_ACTIONS": "true", "GITHUB_REPOSITORY": "edithatogo/kairos", "GITHUB_EVENT_NAME": "pull_request"}
        self.assertEqual(self._dispatch(env, base), ("development_pr_193", 193))
        fork = copy.deepcopy(base)
        fork["pull_request"]["head"]["repo"]["full_name"] = "attacker/kairos"
        with self.assertRaises(ValueError):
            self._dispatch(env, fork)

        dispatch_env = {
            "GITHUB_ACTIONS": "true",
            "GITHUB_REPOSITORY": "edithatogo/kairos",
            "GITHUB_EVENT_NAME": "workflow_dispatch",
            "GITHUB_REF": "refs/heads/main",
        }
        with self.assertRaises(ValueError):
            self._dispatch(dispatch_env, {"repository": {"full_name": "edithatogo/kairos"}, "ref": "main"})

    def test_main_retains_raw_failure_receipt_and_stdout_when_classification_fails(self):
        policy = json.loads((EXCEPTIONS / "EXC-193-http-cache.json").read_text())
        raw_argv = policy["raw_audit_command"]

        def fake_run(argv, **kwargs):
            if argv == raw_argv:
                return subprocess.CompletedProcess(argv, 1, stdout='{"error":{"code":"ENETWORK"}}', stderr="")
            if argv == ["node", "--version"]:
                stdout = "v22.22.2\n"
            elif argv == ["node", "scripts/bootstrap-node-tools/node_modules/npm/bin/npm-cli.js", "--version"]:
                stdout = "12.1.0\n"
            elif argv == ["git", "rev-parse", "HEAD"]:
                stdout = "test-commit\n"
            else:
                stdout = "ok\n"
            return subprocess.CompletedProcess(argv, 0, stdout=stdout, stderr="")

        with tempfile.TemporaryDirectory() as temp:
            output = Path(temp) / "evidence"
            with (
                mock.patch.object(runner, "verify_sources"),
                mock.patch.object(runner.subprocess, "run", side_effect=fake_run) as run,
                mock.patch.dict(os.environ, {}, clear=True),
                mock.patch.object(sys, "argv", ["run_npm_audit_gate.py", "--report-dir", str(output)]),
                contextlib.redirect_stdout(io.StringIO()),
                contextlib.redirect_stderr(io.StringIO()),
            ):
                result = runner.main()

            self.assertEqual(result, 1)
            self.assertEqual(run.call_count, 8)
            receipt = json.loads((output / "receipt.json").read_text())
            raw = receipt["raw_audit"]
            self.assertEqual(raw["status"], "executed")
            self.assertEqual(raw["exit"], 1)
            raw_stdout = (output / "raw-audit.stdout").read_bytes()
            self.assertEqual(hashlib.sha256(raw_stdout).hexdigest(), raw["stdout_sha256"])
            self.assertEqual(raw_stdout, b'{"error":{"code":"ENETWORK"}}')
            raw_stderr = (output / "raw-audit.stderr").read_bytes()
            self.assertEqual(hashlib.sha256(raw_stderr).hexdigest(), raw["stderr_sha256"])


if __name__ == "__main__":
    unittest.main()
