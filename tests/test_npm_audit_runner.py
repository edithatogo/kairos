"""Offline adversarial tests for the exact-scope npm audit gate runner."""

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
        fixture = os.environ.get("KAIROS_NPM_AUDIT_TEST_PACKAGE_SOURCE")
        cls.installed_package = (
            Path(fixture).resolve()
            if fixture
            else ROOT / "scripts/bootstrap-node-tools/node_modules/http-cache-semantics"
        )
        if not cls.installed_package.is_dir():
            cls.installed_package = None

    def make_source_root(self, directory: Path, policy=None) -> Path:
        """Build an offline tree from tracked proofs and, when present, installed bytes."""
        policy = self.policy if policy is None else policy
        for relative in policy["file_sha256"]:
            source = ROOT / relative
            target = directory / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source, target)
        exceptions = directory / "conductor/tracks/20-openssf-supply-chain-institutional-trust/exceptions"
        exceptions.mkdir(parents=True, exist_ok=True)
        exception_id = policy["id"]
        policy_path = EXCEPTIONS / f"{exception_id}-http-cache.json"
        shutil.copyfile(policy_path, exceptions / policy_path.name)
        if exception_id == "EXC-199":
            source_evidence = EXCEPTIONS / "evidence/EXC-199"
            shutil.copytree(source_evidence, exceptions / "evidence/EXC-199")
        if self.installed_package is not None:
            tools = directory / "scripts/bootstrap-node-tools"
            for relative in runner.COPIES:
                package = tools / "node_modules" / relative.removeprefix("node_modules/")
                package.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(self.installed_package / "index.js", package / "index.js")
                shutil.copyfile(self.installed_package / "package.json", package / "package.json")
        return directory

    def test_exact_two_installed_copies_and_pinned_proofs_pass(self):
        if self.installed_package is None:
            self.skipTest("installed npm dependency tree is absent; this task does not install packages")
        with tempfile.TemporaryDirectory() as temp:
            root = self.make_source_root(Path(temp))
            runner.verify_sources(root, copy.deepcopy(self.policy))

    def test_extra_copy_source_drift_identity_and_proof_removal_reject(self):
        if self.installed_package is None:
            self.skipTest("installed npm dependency tree is absent; this task does not install packages")
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
                return runner.execution_context()

    def test_dispatch_accepts_only_two_exact_pr_repository_and_ref_mappings(self):
        event = {
            "repository": {"full_name": "edithatogo/kairos"},
            "pull_request": {
                "number": 193,
                "head": {"ref": "codex/kairos-implementation-programme", "repo": {"full_name": "edithatogo/kairos"}},
            },
        }
        base_env = {"GITHUB_ACTIONS": "true", "GITHUB_REPOSITORY": "edithatogo/kairos", "GITHUB_EVENT_NAME": "pull_request"}
        self.assertEqual(
            self._dispatch(base_env, event),
            ("development_pr_193", 193, "edithatogo/kairos", "codex/kairos-implementation-programme", "EXC-193"),
        )
        pr199 = copy.deepcopy(event)
        pr199["pull_request"]["number"] = 199
        pr199["pull_request"]["head"]["ref"] = "codex/kairos-track48-optimistic-runtime"
        self.assertEqual(
            self._dispatch(base_env, pr199),
            ("development_pr_199", 199, "edithatogo/kairos", "codex/kairos-track48-optimistic-runtime", "EXC-199"),
        )

        invalid_cases = []
        wrong_pr = copy.deepcopy(pr199)
        wrong_pr["pull_request"]["number"] = 193
        invalid_cases.append((base_env, wrong_pr))
        wrong_ref = copy.deepcopy(pr199)
        wrong_ref["pull_request"]["head"]["ref"] = "main"
        invalid_cases.append((base_env, wrong_ref))
        fork = copy.deepcopy(pr199)
        fork["pull_request"]["head"]["repo"]["full_name"] = "attacker/kairos"
        invalid_cases.append((base_env, fork))
        wrong_event_repo = copy.deepcopy(pr199)
        wrong_event_repo["repository"]["full_name"] = "attacker/kairos"
        invalid_cases.append((base_env, wrong_event_repo))
        wrong_owner = {**base_env, "GITHUB_REPOSITORY": "attacker/kairos"}
        invalid_cases.append((wrong_owner, pr199))
        for env, candidate in invalid_cases:
            with self.subTest(env=env, event=candidate), self.assertRaises(ValueError):
                self._dispatch(env, candidate)

    def test_workflow_dispatch_requires_exact_ref_and_allowed_context(self):
        event = {"repository": {"full_name": "edithatogo/kairos"}, "ref": "refs/heads/codex/kairos-track48-optimistic-runtime"}
        env = {
            "GITHUB_ACTIONS": "true", "GITHUB_REPOSITORY": "edithatogo/kairos",
            "GITHUB_EVENT_NAME": "workflow_dispatch", "GITHUB_REF": "refs/heads/codex/kairos-track48-optimistic-runtime",
        }
        self.assertEqual(self._dispatch(env, event)[1:], (199, "edithatogo/kairos", "codex/kairos-track48-optimistic-runtime", "EXC-199"))
        allowed = copy.deepcopy(event)
        allowed["inputs"] = {"audit_context": "alpha_package_dry_run"}
        self.assertEqual(self._dispatch(env, allowed)[0], "alpha_package_dry_run")
        invalid = copy.deepcopy(event)
        invalid["inputs"] = {"audit_context": "publication"}
        with self.assertRaises(ValueError):
            self._dispatch(env, invalid)
        wrong_github_ref = {**env, "GITHUB_REF": "refs/heads/main"}
        with self.assertRaises(ValueError):
            self._dispatch(wrong_github_ref, event)

    @mock.patch.dict(os.environ, {}, clear=True)
    def test_local_main_unknown_or_main_branch_cannot_impersonate_pr193(self):
        with mock.patch.object(runner, "local_repository", return_value="edithatogo/kairos"), mock.patch.object(
            runner, "local_git_branch", return_value="codex/exc199-activation"
        ), self.assertRaisesRegex(ValueError, "unapproved local branch"):
            runner.execution_context()
        with mock.patch.object(runner, "local_repository", return_value="edithatogo/kairos"), mock.patch.object(
            runner, "local_git_branch", return_value="main"
        ), self.assertRaisesRegex(ValueError, "unapproved local branch"):
            runner.execution_context()
        with mock.patch.object(runner, "local_repository", return_value="edithatogo/kairos"), mock.patch.object(
            runner, "local_git_branch", return_value="codex/kairos-track48-optimistic-runtime"
        ):
            self.assertEqual(
                runner.execution_context(),
                ("development_pr_199", 199, "edithatogo/kairos", "codex/kairos-track48-optimistic-runtime", "EXC-199"),
            )
        with mock.patch.object(runner, "local_repository", return_value="attacker/kairos"), mock.patch.object(
            runner, "local_git_branch", return_value="codex/kairos-track48-optimistic-runtime"
        ), self.assertRaisesRegex(ValueError, "approved repository"):
            runner.execution_context()

    def test_pr199_verify_sources_checks_approved_proofs_evidence_and_installed_copies(self):
        if self.installed_package is None:
            self.skipTest("installed npm dependency tree is absent; this task does not install packages")
        policy = json.loads((EXCEPTIONS / "EXC-199-http-cache.json").read_text())
        proof = "scripts/bootstrap-node-tools/apply_http_cache_fix.py"
        with tempfile.TemporaryDirectory() as temp:
            root = self.make_source_root(Path(temp), policy)
            runner.verify_sources(root, copy.deepcopy(policy))
            source = root / proof
            source.write_bytes(source.read_bytes() + b"tampered")
            with self.assertRaisesRegex(ValueError, "proof drift"):
                runner.verify_sources(root, copy.deepcopy(policy))

        with tempfile.TemporaryDirectory() as temp:
            root = self.make_source_root(Path(temp), policy)
            record_path = root / "conductor/tracks/20-openssf-supply-chain-institutional-trust/exceptions/EXC-199-http-cache.json"
            record_path.write_bytes(record_path.read_bytes() + b"\n")
            with self.assertRaisesRegex(ValueError, "record bytes changed"):
                runner.verify_sources(root, copy.deepcopy(policy))

    def test_exc199_evidence_bundle_and_mutations_fail_closed(self):
        policy = json.loads((EXCEPTIONS / "EXC-199-http-cache.json").read_text())
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            destination = root / "conductor/tracks/20-openssf-supply-chain-institutional-trust/exceptions/evidence/EXC-199"
            destination.parent.mkdir(parents=True)
            shutil.copytree(EXCEPTIONS / "evidence/EXC-199", destination)
            runner.verify_exc199_evidence(root, policy)
            for relative in (
                "audit/pr199-0a3b86a/raw-audit.json",
                "controls/e3306f4/receipt.json",
                "preparation/node-npm-versions.txt",
                "manifest.json",
            ):
                with self.subTest(relative=relative), tempfile.TemporaryDirectory() as damaged_temp:
                    damaged_root = Path(damaged_temp)
                    damaged = damaged_root / "conductor/tracks/20-openssf-supply-chain-institutional-trust/exceptions/evidence/EXC-199"
                    damaged.parent.mkdir(parents=True)
                    shutil.copytree(EXCEPTIONS / "evidence/EXC-199", damaged)
                    target = damaged / relative
                    target.write_bytes(target.read_bytes() + b"tampered")
                    with self.assertRaises(ValueError):
                        runner.verify_exc199_evidence(damaged_root, policy)

    def _run_main_with_fake_commands(
        self,
        raw_stdout: str,
        raw_exit: int,
        raw_stderr: str = "",
        scope_error: str | None = None,
        source_error: str | None = None,
    ):
        policy = json.loads((EXCEPTIONS / "EXC-199-http-cache.json").read_text())
        raw_argv = policy["raw_audit_command"]

        def fake_run(argv, **kwargs):
            if argv == raw_argv:
                return subprocess.CompletedProcess(argv, raw_exit, stdout=raw_stdout, stderr=raw_stderr)
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
            identity = ("development_pr_199", 199, "edithatogo/kairos", "codex/kairos-track48-optimistic-runtime", "EXC-199")
            context_patch = (
                mock.patch.object(runner, "execution_context", return_value=identity)
                if scope_error is None
                else mock.patch.object(runner, "execution_context", side_effect=ValueError(scope_error))
            )
            with (
                context_patch,
                mock.patch.object(
                    runner, "verify_sources", side_effect=ValueError(source_error) if source_error else None
                ) as verify_sources,
                mock.patch.object(runner.subprocess, "run", side_effect=fake_run) as run,
                mock.patch.dict(os.environ, {}, clear=True),
                mock.patch.object(sys, "argv", ["run_npm_audit_gate.py", "--report-dir", str(output)]),
                contextlib.redirect_stdout(io.StringIO()),
                contextlib.redirect_stderr(io.StringIO()),
            ):
                result = runner.main()
            receipt = json.loads((output / "receipt.json").read_text())
            raw_bytes = (output / "raw-audit.stdout").read_bytes()
            raw_stderr = (output / "raw-audit.stderr").read_bytes()
            return result, run.call_count, receipt, raw_bytes, raw_stderr, verify_sources.call_count

    def test_main_retains_raw_audit_artifacts_when_approved_record_is_blocked(self):
        report_path = EXCEPTIONS / "evidence/EXC-199/audit/pr199-0a3b86a/raw-audit.json"
        raw_stdout = report_path.read_text()
        result, run_count, receipt, raw_bytes, raw_stderr, proof_calls = self._run_main_with_fake_commands(raw_stdout, 1)
        self.assertEqual(result, 1)
        self.assertEqual(run_count, 8)
        self.assertEqual(proof_calls, 1)
        self.assertEqual(receipt["classification"], "failed")
        self.assertIn("stale-fallback gap", receipt["error"])
        self.assertEqual(receipt["raw_audit"]["status"], "executed")
        self.assertEqual(receipt["raw_audit"]["exit"], 1)
        self.assertEqual(raw_bytes.decode(), raw_stdout)
        self.assertEqual(hashlib.sha256(raw_bytes).hexdigest(), receipt["raw_audit"]["stdout_sha256"])
        self.assertEqual(raw_stderr, b"")
        self.assertEqual(hashlib.sha256(raw_stderr).hexdigest(), receipt["raw_audit"]["stderr_sha256"])

    def test_main_clean_audit_passes_without_using_blocked_exception(self):
        clean = json.loads((EXCEPTIONS / "evidence/EXC-199/audit/pr199-0a3b86a/raw-audit.json").read_text())
        clean["vulnerabilities"] = {}
        clean["metadata"]["vulnerabilities"] = {key: 0 for key in ("info", "low", "moderate", "high", "critical", "total")}
        result, _, receipt, raw_bytes, _, proof_calls = self._run_main_with_fake_commands(
            json.dumps(clean), 0, source_error="verify_sources must not run for clean audits"
        )
        self.assertEqual(result, 0)
        self.assertEqual(proof_calls, 0)
        self.assertEqual(receipt["classification"], "clean")
        self.assertEqual(receipt["raw_audit"]["status"], "executed")
        self.assertEqual(receipt["raw_audit"]["exit"], 0)
        self.assertEqual(raw_bytes.decode(), json.dumps(clean))

    def test_unknown_local_or_pr_scope_can_pass_only_a_strict_clean_audit(self):
        clean = json.loads((EXCEPTIONS / "evidence/EXC-199/audit/pr199-0a3b86a/raw-audit.json").read_text())
        clean["vulnerabilities"] = {}
        clean["metadata"]["vulnerabilities"] = {key: 0 for key in ("info", "low", "moderate", "high", "critical", "total")}
        for scope_error in ("unapproved local branch", "unapproved PR source"):
            with self.subTest(scope_error=scope_error):
                result, _, receipt, _, _, proof_calls = self._run_main_with_fake_commands(
                    json.dumps(clean), 0, scope_error=scope_error,
                    source_error="verify_sources must not run without non-clean exception classification",
                )
                self.assertEqual(result, 0)
                self.assertEqual(proof_calls, 0)
                self.assertEqual(receipt["classification"], "clean")
                self.assertEqual(receipt["exception_scope"], {"status": "unapproved", "error": scope_error})
                self.assertEqual(receipt["raw_audit"]["status"], "executed")
                self.assertEqual(receipt["raw_audit"]["exit"], 0)

    def test_unknown_scope_nonclean_audit_is_retained_then_rejected(self):
        raw_stdout = (EXCEPTIONS / "evidence/EXC-199/audit/pr199-0a3b86a/raw-audit.json").read_text()
        result, _, receipt, raw_bytes, raw_stderr, proof_calls = self._run_main_with_fake_commands(
            raw_stdout, 1, scope_error="unapproved local branch"
        )
        self.assertEqual(result, 1)
        self.assertEqual(proof_calls, 0)
        self.assertEqual(receipt["classification"], "failed")
        self.assertIn("repository and head ref are required", receipt["error"])
        self.assertEqual(receipt["raw_audit"]["status"], "executed")
        self.assertEqual(receipt["raw_audit"]["exit"], 1)
        self.assertEqual(raw_bytes.decode(), raw_stdout)
        self.assertEqual(raw_stderr, b"")

    def test_clean_schema_and_counts_are_validated_before_exception_proofs(self):
        malformed = json.loads((EXCEPTIONS / "evidence/EXC-199/audit/pr199-0a3b86a/raw-audit.json").read_text())
        malformed["vulnerabilities"] = {}
        malformed["metadata"]["vulnerabilities"] = {key: 0 for key in ("info", "low", "moderate", "high", "critical", "total")}
        malformed["auditReportVersion"] = 3
        raw_stdout = json.dumps(malformed)
        result, run_count, receipt, raw_bytes, _, proof_calls = self._run_main_with_fake_commands(
            raw_stdout, 0, source_error="verify_sources must not run before clean schema validation"
        )
        self.assertEqual(result, 1)
        self.assertEqual(run_count, 1)
        self.assertEqual(proof_calls, 0)
        self.assertIn("unsupported audit report version", receipt["error"])
        self.assertEqual(receipt["raw_audit"]["status"], "executed")
        self.assertEqual(receipt["raw_audit"]["exit"], 0)
        self.assertEqual(raw_bytes.decode(), raw_stdout)

    def test_known_scope_proof_drift_is_rejected_after_retaining_raw_audit(self):
        raw_stdout = (EXCEPTIONS / "evidence/EXC-199/audit/pr199-0a3b86a/raw-audit.json").read_text()
        result, _, receipt, raw_bytes, raw_stderr, proof_calls = self._run_main_with_fake_commands(
            raw_stdout, 1, source_error="proof drift: scripts/bootstrap-node-tools/package-lock.json"
        )
        self.assertEqual(result, 1)
        self.assertEqual(proof_calls, 1)
        self.assertIn("proof drift", receipt["error"])
        self.assertEqual(receipt["raw_audit"]["status"], "executed")
        self.assertEqual(receipt["raw_audit"]["exit"], 1)
        self.assertEqual(raw_bytes.decode(), raw_stdout)
        self.assertEqual(raw_stderr, b"")


if __name__ == "__main__":
    unittest.main()
