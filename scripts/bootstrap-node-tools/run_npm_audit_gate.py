#!/usr/bin/env python3
"""Retain raw npm audit evidence and classify only pinned EXC-193/199 scope."""
from __future__ import annotations

import argparse
from datetime import datetime, timezone
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
EXCEPTIONS = ROOT / "conductor/tracks/20-openssf-supply-chain-institutional-trust/exceptions"
EVIDENCE_199 = EXCEPTIONS / "evidence/EXC-199"
PROOFS = {
    "scripts/bootstrap-node-tools/package-lock.json",
    "scripts/bootstrap-node-tools/apply_http_cache_fix.py",
    "scripts/bootstrap-node-tools/validate_npm_cli.mjs",
    "tests/test_http_cache_patch.py",
    "tests/http-cache-security-regression.mjs",
}
COPIES = {
    "node_modules/http-cache-semantics",
    "node_modules/npm/node_modules/http-cache-semantics",
}
EXPECTED_REPOSITORY = "edithatogo/kairos"
EXCEPTION_POLICY_PATHS = {
    "EXC-193": "EXC-193-http-cache.json",
    "EXC-199": "EXC-199-http-cache.json",
}
EXPECTED_BRANCH = "codex/kairos-implementation-programme"
EXPECTED_BRANCH_BY_PR = {
    193: "codex/kairos-implementation-programme",
    199: "codex/kairos-track48-optimistic-runtime",
}
EXPECTED_CONTEXTS_BY_PR = {
    193: frozenset({"development_pr_193", "alpha_package_dry_run", "beta_package_dry_run"}),
    199: frozenset({"development_pr_199", "alpha_package_dry_run", "beta_package_dry_run"}),
}
EXPECTED_FILE_HASHES_BY_EXCEPTION = {
    "EXC-193": {
        "scripts/bootstrap-node-tools/package-lock.json": "3905b6f36ea3b5625667f3a40a829e8eb7a351f6ff074372863f02b1d2216552",
        "scripts/bootstrap-node-tools/apply_http_cache_fix.py": "a928e08eacca08e497199ab58747ff751041643748a0f53ff988dbd7d2d0aa91",
        "scripts/bootstrap-node-tools/validate_npm_cli.mjs": "3757fcb8d2bc16ba842cc5f3868c1f09de591ef50a599f56c83ab1d51397b9a4",
        "tests/test_http_cache_patch.py": "542a970f0cf79242334b274cd93ec60ee32fb05b375d2302591cb822550479b9",
        "tests/http-cache-security-regression.mjs": "298c3537d14c19afdc188cde554596d4f2fc4f95c56d387b79af9ab851f510b2",
    },
    "EXC-199": {
        "scripts/bootstrap-node-tools/package-lock.json": "3905b6f36ea3b5625667f3a40a829e8eb7a351f6ff074372863f02b1d2216552",
        "scripts/bootstrap-node-tools/apply_http_cache_fix.py": "1745f11f6b2ae27c47ba00218970192ec0b0034d3d1467b524e411e2c3c9afa4",
        "scripts/bootstrap-node-tools/validate_npm_cli.mjs": "68361630ff540c9e32e1e62417c805b54c35b57195f5108c7679b6ca4c15bcb8",
        "tests/test_http_cache_patch.py": "4701a42699573255b6dac6c0815585137ac7e6132c8f2ebe3e7ddb95b9b0a641",
        "tests/http-cache-security-regression.mjs": "05d4c9990c5dfc691798336443d28638a405a751076ae7147efc7a19a5392d9c",
    },
}
EXPECTED_PATCHED_HASH_BY_EXCEPTION = {
    "EXC-193": "fc7b3f0265b7a7d0fee83bafa47186a66495720d3179801c2be3083de6d0cf76",
    "EXC-199": "5942c6d3df40fce2151d8e409e7ad7e7c9c4a8ee09b7066072edf3a939fc589c",
}
EXPECTED_GRAPH_HASH = "0b3e5f1d5f65b48f1a20618ba352e6f02529a134f62e0230126ac68c73b5fec8"
EXPECTED_AUDIT_COMMAND = [
    "node", "scripts/bootstrap-node-tools/node_modules/npm/bin/npm-cli.js", "audit",
    "--prefix", "scripts/bootstrap-node-tools", "--audit-level=moderate", "--json",
]
EXPECTED_EXC199_MANIFEST_SHA256 = "1f825f329e6419a02603173b434bcab2ace71a3d665e6a6ae596f72afd5812b3"
EXPECTED_EXC199_RUNTIME_SHA256 = "2df41a4349f4dc63e0a2bc0ac62ee896a2b3d27f3407f1442caf2f3765127e17"
EXPECTED_EXC199_AMENDMENT_SHA256 = "b4818bfc43ad2f2061087735489ffbc488b3d42f30dd599f44a7787df2a3c212"
EXPECTED_EXC199_AMENDMENT_MANIFEST_SHA256 = "e311be230541cc961c1428ab151f62a0a45baa01edacb97f9bbca04713e932e7"


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def read_json(text: str):
    return json.loads(text, object_pairs_hook=unique_object)


def _policy_module():
    module_path = EXCEPTIONS / "npm_audit_policy.py"
    spec = importlib.util.spec_from_file_location("npm_audit_policy", module_path)
    if spec is None or spec.loader is None:
        raise ValueError("npm audit policy module is missing")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def _scope(exception_id: str):
    module = _policy_module()
    record = module._POLICY_RECORDS.get(exception_id)
    if record is None:
        raise ValueError("exception is not in the immutable runner allowlist")
    return module, record


def _identity_for_branch(repository: str, branch: str):
    if repository != EXPECTED_REPOSITORY:
        raise ValueError("exception cannot apply outside the approved repository")
    for pull_request, expected_branch in EXPECTED_BRANCH_BY_PR.items():
        if branch == expected_branch:
            return pull_request
    raise ValueError("unapproved local branch")


def _git_output(argv: list[str]) -> str:
    result = subprocess.run(argv, cwd=ROOT, text=True, capture_output=True, timeout=10)
    if result.returncode or result.stderr:
        raise ValueError("cannot establish trusted local Git identity")
    return result.stdout.strip()


def local_repository() -> str:
    remote = _git_output(["git", "remote", "get-url", "origin"])
    if remote.startswith("https://github.com/"):
        value = remote.removeprefix("https://github.com/")
    elif remote.startswith("git@github.com:"):
        value = remote.removeprefix("git@github.com:")
    else:
        raise ValueError("local origin is outside the approved GitHub repository")
    return value.removesuffix(".git").strip("/")


def local_git_branch() -> str:
    branch = _git_output(["git", "branch", "--show-current"])
    if not branch:
        raise ValueError("detached local HEAD cannot establish an approved PR ref")
    return branch


def execution_context() -> tuple[str, int, str, str, str]:
    """Return context, PR, repository, head ref and selected exception ID."""
    if os.environ.get("GITHUB_ACTIONS") != "true":
        repository = local_repository()
        branch = local_git_branch()
        pull_request = _identity_for_branch(repository, branch)
        return f"development_pr_{pull_request}", pull_request, repository, branch, f"EXC-{pull_request}"

    repository = os.environ.get("GITHUB_REPOSITORY", "")
    if repository != EXPECTED_REPOSITORY:
        raise ValueError("exception cannot apply outside the approved repository")
    event_path = os.environ.get("GITHUB_EVENT_PATH")
    if not event_path:
        raise ValueError("GitHub event payload is missing")
    event = read_json(Path(event_path).read_text())
    if event.get("repository", {}).get("full_name") != EXPECTED_REPOSITORY:
        raise ValueError("unapproved event repository")
    kind = os.environ.get("GITHUB_EVENT_NAME")
    if kind == "pull_request":
        pr = event.get("pull_request")
        if not isinstance(pr, dict):
            raise ValueError("pull request event payload is malformed")
        pull_request = pr.get("number")
        head = pr.get("head")
        if not isinstance(head, dict) or not isinstance(head.get("repo"), dict):
            raise ValueError("pull request source is malformed")
        branch = head.get("ref")
        if head["repo"].get("full_name") != EXPECTED_REPOSITORY:
            raise ValueError("unapproved PR source")
        expected_pull_request = _identity_for_branch(repository, branch)
        if isinstance(pull_request, bool) or not isinstance(pull_request, int) or pull_request != expected_pull_request:
            raise ValueError("pull request number does not match the exact branch mapping")
        context = f"development_pr_{pull_request}"
    elif kind == "workflow_dispatch":
        ref = event.get("ref")
        github_ref = os.environ.get("GITHUB_REF", "")
        if not isinstance(ref, str) or not isinstance(github_ref, str):
            raise ValueError("workflow dispatch ref is missing")
        if github_ref != "refs/heads/" + ref.removeprefix("refs/heads/"):
            raise ValueError("workflow dispatch ref mismatch")
        branch = ref.removeprefix("refs/heads/")
        pull_request = _identity_for_branch(repository, branch)
        inputs = event.get("inputs", {})
        if not isinstance(inputs, dict):
            raise ValueError("workflow dispatch inputs are malformed")
        context = inputs.get("audit_context", f"development_pr_{pull_request}")
    else:
        raise ValueError("exception cannot apply to this event")

    if (
        isinstance(pull_request, bool)
        or not isinstance(pull_request, int)
        or pull_request not in EXPECTED_BRANCH_BY_PR
        or branch != EXPECTED_BRANCH_BY_PR[pull_request]
    ):
        raise ValueError("unapproved PR/ref mapping")
    if not isinstance(context, str) or context not in EXPECTED_CONTEXTS_BY_PR[pull_request]:
        raise ValueError("event context is outside the exact PR exception scope")
    return context, pull_request, repository, branch, f"EXC-{pull_request}"


def verify_exc199_evidence(root: Path, policy: dict) -> None:
    manifest_path = root / "conductor/tracks/20-openssf-supply-chain-institutional-trust/exceptions/evidence/EXC-199/manifest.json"
    manifest_bytes = manifest_path.read_bytes()
    if digest(manifest_bytes) != EXPECTED_EXC199_MANIFEST_SHA256:
        raise ValueError("EXC-199 evidence manifest changed")
    manifest = read_json(manifest_bytes.decode("utf-8"))
    evidence_dir = manifest_path.parent
    copies = manifest.get("copies")
    if not isinstance(copies, list) or len(copies) != 17:
        raise ValueError("EXC-199 evidence copy manifest is incomplete")
    for item in copies:
        if not isinstance(item, dict):
            raise ValueError("EXC-199 evidence copy row is malformed")
        relative = Path(item.get("path", ""))
        if relative.is_absolute() or ".." in relative.parts or not item.get("byte_identical"):
            raise ValueError("EXC-199 evidence copy path or provenance is invalid")
        path = evidence_dir / relative
        if path.is_symlink() or not path.is_file():
            raise ValueError(f"EXC-199 evidence copy is missing: {relative}")
        data = path.read_bytes()
        if len(data) != item.get("bytes") or digest(data) != item.get("sha256"):
            raise ValueError(f"EXC-199 evidence copy changed: {relative}")

    audit = manifest.get("audit_run", {})
    if (
        audit.get("source_commit") != "0a3b86aa33d1f158eaca2855e0e503cdac6e08ef"
        or audit.get("exit") != 1
        or audit.get("raw_audit_sha256") != policy.get("audit_baseline_sha256")
        or audit.get("vulnerabilities_sha256") != EXPECTED_GRAPH_HASH
        or audit.get("finding_counts", {}).get("high") != 19
    ):
        raise ValueError("EXC-199 audit evidence metadata changed")
    raw = evidence_dir / "audit/pr199-0a3b86a/raw-audit.json"
    report = read_json(raw.read_text())
    if digest(raw.read_bytes()) != policy.get("audit_baseline_sha256"):
        raise ValueError("EXC-199 raw audit baseline changed")
    graph = report.get("vulnerabilities")
    graph_hash = digest(json.dumps(graph, sort_keys=True, separators=(",", ":")).encode("utf-8"))
    if report.get("auditReportVersion") != 2 or graph_hash != EXPECTED_GRAPH_HASH:
        raise ValueError("EXC-199 archived finding graph changed")
    audit_receipt = read_json((evidence_dir / "audit/pr199-0a3b86a/receipt.json").read_text())
    if audit_receipt.get("source") != audit["source_commit"] or audit_receipt.get("exit") != 1:
        raise ValueError("EXC-199 audit receipt provenance changed")

    controls = manifest.get("compensating_controls", {})
    control_receipt = read_json((evidence_dir / "controls/e3306f4/receipt.json").read_text())
    if controls.get("source_commit") != "e3306f4ca3e560b81725a1b07a275d98643caefe":
        raise ValueError("EXC-199 control source changed")
    check_rows = control_receipt.get("checks")
    expected_labels = {"patch-tests", "resolution", "behavior-top", "behavior-npm", "signatures"}
    if not isinstance(check_rows, list) or {row.get("label") for row in check_rows} != expected_labels:
        raise ValueError("EXC-199 control checks changed")
    if any(row.get("exit") != 0 for row in check_rows):
        raise ValueError("EXC-199 compensating control did not pass")
    if control_receipt.get("source") != controls["source_commit"]:
        raise ValueError("EXC-199 control receipt source changed")

    hosted = manifest.get("hosted_pr199_attempt", {})
    hosted_receipt = read_json((evidence_dir / "hosted-pr199-0a3b86a/runner-receipt.json").read_text())
    if (
        hosted.get("raw_audit_status") != "not_executed"
        or hosted.get("runner_error") != "unapproved PR source"
        or hosted_receipt.get("raw_audit", {}).get("status") != "not_executed"
    ):
        raise ValueError("EXC-199 hosted attempt metadata changed")

    runtime = manifest.get("preparation_runtime_metadata", {})
    runtime_path = evidence_dir / "preparation/node-npm-versions.txt"
    if (
        runtime.get("sha256") != EXPECTED_EXC199_RUNTIME_SHA256
        or runtime.get("measurement_scope") != "Preparation-time metadata only; not retroactive runtime metadata for audit/control captures."
        or digest(runtime_path.read_bytes()) != EXPECTED_EXC199_RUNTIME_SHA256
        or policy.get("evidence", {}).get("runtime_metadata", {}).get("metadata_sha256") != EXPECTED_EXC199_RUNTIME_SHA256
    ):
        raise ValueError("EXC-199 preparation runtime metadata changed")
    runtime_text = runtime_path.read_text()
    if "v26.10.0" not in runtime_text or "12.1.0" not in runtime_text or "be54595e3fad310f952389520632cf9af6900e92" not in runtime_text:
        raise ValueError("EXC-199 preparation runtime metadata values changed")


def verify_exc199_amendment_evidence(root: Path, policy: dict) -> None:
    exception_dir = root / "conductor/tracks/20-openssf-supply-chain-institutional-trust/exceptions"
    amendment_path = exception_dir / "EXC-199-mitigation-amendment.json"
    amendment_bytes = amendment_path.read_bytes()
    if digest(amendment_bytes) != EXPECTED_EXC199_AMENDMENT_SHA256:
        raise ValueError("EXC-199 source amendment record changed")
    amendment = read_json(amendment_bytes.decode("utf-8"))
    module = _policy_module()
    record = module._POLICY_RECORDS.get("EXC-199")
    binding = policy.get("source_binding_amendment")
    if not isinstance(record, dict) or not isinstance(binding, dict):
        raise ValueError("EXC-199 source amendment binding is missing")
    if binding != record.get("source_binding_amendment"):
        raise ValueError("EXC-199 source amendment binding differs from immutable classifier record")
    if (
        amendment.get("id") != binding.get("id")
        or amendment.get("status") != "approved_pending_reviewed_integration"
        or amendment.get("required_pull_request") != 199
        or amendment.get("repository") != "edithatogo/kairos"
        or amendment.get("head_ref") != "codex/kairos-track48-optimistic-runtime"
        or amendment.get("mitigation_source_commit") != binding.get("mitigation_source_commit")
        or amendment.get("patched_index_sha256") != binding.get("patched_index_sha256")
        or amendment.get("file_sha256") != binding.get("file_sha256")
        or amendment.get("scope_amendment_approval") != binding.get("owner_approval")
        or policy.get("mitigation_review_status") != binding.get("review_status")
        or policy.get("file_sha256") != binding.get("file_sha256")
        or policy.get("patched_index_sha256") != binding.get("patched_index_sha256")
    ):
        raise ValueError("EXC-199 approved source amendment fields changed")
    if (
        amendment.get("vulnerabilities_sha256") != EXPECTED_GRAPH_HASH
        or amendment.get("raw_audit_sha256") != policy.get("audit_baseline_sha256")
        or amendment.get("raw_audit_exit") != 1
        or amendment.get("finding_count") != 19
        or amendment.get("expires_at") != policy.get("expires_at")
        or amendment.get("allowed_contexts") != policy.get("allowed_contexts")
        or amendment.get("excluded_contexts") != policy.get("excluded_contexts")
    ):
        raise ValueError("EXC-199 source amendment scope or audit binding changed")

    manifest_relative = amendment.get("evidence_manifest")
    if manifest_relative != binding.get("evidence_manifest"):
        raise ValueError("EXC-199 source amendment evidence path changed")
    manifest_path = exception_dir / manifest_relative
    if manifest_path.is_symlink():
        raise ValueError("EXC-199 source amendment manifest is a symlink")
    manifest_bytes = manifest_path.read_bytes()
    if (
        digest(manifest_bytes) != EXPECTED_EXC199_AMENDMENT_MANIFEST_SHA256
        or digest(manifest_bytes) != binding.get("evidence_manifest_sha256")
    ):
        raise ValueError("EXC-199 source amendment evidence manifest changed")
    manifest = read_json(manifest_bytes.decode("utf-8"))
    if not isinstance(manifest, dict) or len(manifest) != 17:
        raise ValueError("EXC-199 source amendment evidence manifest is incomplete")
    evidence_root = manifest_path.parent.resolve()
    for name, expected in manifest.items():
        relative = Path(name)
        if not isinstance(name, str) or relative.is_absolute() or ".." in relative.parts:
            raise ValueError("EXC-199 source amendment evidence path is unsafe")
        path = manifest_path.parent / relative
        try:
            path.resolve().relative_to(evidence_root)
        except ValueError:
            raise ValueError(f"EXC-199 source amendment evidence path escapes its root: {name}") from None
        if path.is_symlink() or not path.is_file():
            raise ValueError(f"EXC-199 source amendment evidence file is missing or unsafe: {name}")
        if not isinstance(expected, str) or digest(path.read_bytes()) != expected:
            raise ValueError(f"EXC-199 source amendment evidence copy changed: {name}")

    receipt = read_json((manifest_path.parent / "receipt.json").read_text())
    expected_candidate_files = {
        name: value for name, value in binding["file_sha256"].items()
        if name != "scripts/bootstrap-node-tools/package-lock.json"
    }
    if (
        receipt.get("candidate_head") != binding.get("mitigation_source_commit")
        or receipt.get("candidate_file_sha256") != expected_candidate_files
    ):
        raise ValueError("EXC-199 source amendment receipt does not bind the approved candidate")


def verify_sources(root: Path, policy: dict) -> None:
    module = _policy_module()
    exception_id = policy.get("id")
    record = module._POLICY_RECORDS.get(exception_id)
    if record is None:
        raise ValueError("exception is not in the immutable policy allowlist")
    policy_path = root / "conductor/tracks/20-openssf-supply-chain-institutional-trust/exceptions" / record["policy_path"]
    policy_bytes = policy_path.read_bytes()
    if digest(policy_bytes) != record["raw_record_sha256"]:
        raise ValueError("approved exception record bytes changed")
    if read_json(policy_bytes.decode("utf-8")) != policy:
        raise ValueError("loaded exception record differs from the pinned file")
    canonical = json.dumps(policy, sort_keys=True, separators=(",", ":")).encode("utf-8")
    if digest(canonical) != record["record_sha256"]:
        raise ValueError("approved exception record fingerprint changed")
    expected_files = EXPECTED_FILE_HASHES_BY_EXCEPTION.get(exception_id)
    expected_patch = EXPECTED_PATCHED_HASH_BY_EXCEPTION.get(exception_id)
    if expected_files is None or expected_patch is None:
        raise ValueError("no source proof pins are available for this exception")
    if policy.get("file_sha256") != expected_files:
        raise ValueError("approved source fingerprints changed")
    if policy.get("patched_index_sha256") != expected_patch:
        raise ValueError("approved patch hash changed")
    if policy.get("raw_audit_command") != EXPECTED_AUDIT_COMMAND:
        raise ValueError("raw audit command changed")
    if policy.get("vulnerabilities_sha256") != EXPECTED_GRAPH_HASH:
        raise ValueError("approved finding graph changed")
    if policy.get("expires_at") != "2026-10-10T00:00:00+10:00":
        raise ValueError("approved expiry changed")
    if set(policy["file_sha256"]) != PROOFS:
        raise ValueError("proof manifest must bind all required controls")
    for name, expected in policy["file_sha256"].items():
        path = root / name
        if path.is_symlink() or digest(path.read_bytes()) != expected:
            raise ValueError(f"proof drift: {name}")
    if exception_id == "EXC-199":
        verify_exc199_evidence(root, policy)
        verify_exc199_amendment_evidence(root, policy)

    tools = root / "scripts/bootstrap-node-tools"
    modules = tools / "node_modules"
    if modules.is_symlink() or not modules.is_dir():
        raise ValueError("installed modules must be a real directory")
    found = set()
    for current, dirs, _ in os.walk(modules, followlinks=False):
        for name in dirs:
            path = Path(current) / name
            if path.is_symlink():
                raise ValueError(f"symlink directory in installed tree: {path}")
            if name == "http-cache-semantics":
                found.add(path.relative_to(tools).as_posix())
    if found != COPIES:
        raise ValueError("missing or unexpected cache dependency copies")
    for name in sorted(found):
        path = tools / name
        source, manifest = path / "index.js", path / "package.json"
        if source.is_symlink() or manifest.is_symlink():
            raise ValueError("symlink cache source")
        identity = read_json(manifest.read_text())
        if identity.get("name") != "http-cache-semantics" or identity.get("version") != "4.2.0":
            raise ValueError("cache identity drift")
        if digest(source.read_bytes()) != policy["patched_index_sha256"]:
            raise ValueError("unpatched cache source")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report-dir", type=Path, default=ROOT / "artifacts/npm-audit-gate")
    args = parser.parse_args()
    output = args.report_dir
    output.mkdir(parents=True, exist_ok=True)
    receipt = {"cwd": str(ROOT), "started_at": datetime.now(timezone.utc).isoformat(),
               "classification": "failed", "raw_audit": {"status": "not_executed"}, "checks": []}

    def command(argv, label):
        stdout, stderr = output / (label + ".stdout"), output / (label + ".stderr")
        try:
            result = subprocess.run(argv, cwd=ROOT, text=True, capture_output=True, timeout=180)
        except subprocess.TimeoutExpired as exc:
            def partial(value):
                return value.decode(errors="replace") if isinstance(value, bytes) else (value or "")
            stdout.write_text(partial(exc.stdout))
            stderr.write_text(partial(exc.stderr))
            item = {"argv": argv, "exit": None, "timed_out": True,
                    "stdout_sha256": digest(stdout.read_bytes()), "stderr_sha256": digest(stderr.read_bytes())}
            receipt["checks"].append(item)
            if label == "raw-audit":
                receipt["raw_audit"] = {"status": "timed_out", **item}
            raise
        stdout.write_text(result.stdout)
        stderr.write_text(result.stderr)
        item = {"argv": argv, "exit": result.returncode,
                "stdout_sha256": digest(stdout.read_bytes()), "stderr_sha256": digest(stderr.read_bytes())}
        receipt["checks"].append(item)
        return result, item

    try:
        try:
            context, pr, repository, head_ref, exception_id = execution_context()
            receipt["exception_scope"] = {"status": "approved_exact_scope"}
        except (OSError, ValueError, subprocess.SubprocessError) as scope_error:
            # A failed scope match must never grant an exception, but it should
            # not prevent an independent strict clean audit from succeeding.
            context = repository = head_ref = None
            pr = None
            exception_id = None
            receipt["exception_scope"] = {
                "status": "unapproved",
                "error": str(scope_error),
            }
        if exception_id is not None:
            if exception_id not in EXCEPTION_POLICY_PATHS:
                raise ValueError("exception is not in the static runner allowlist")
            receipt["exception_id"] = exception_id
        if context is not None:
            receipt.update({"repository": repository, "pull_request": pr, "head_ref": head_ref, "context": context})

        # Always retain raw output before checking exception-only proofs or
        # mitigation controls. The fixed argv prevents a changed policy record
        # from selecting a different command.
        receipt["raw_audit"] = {"status": "started", "argv": EXPECTED_AUDIT_COMMAND}
        raw_result, raw = command(EXPECTED_AUDIT_COMMAND, "raw-audit")
        receipt["raw_audit"] = {"status": "executed", **raw}

        if raw_result.stdout and raw_result.stderr:
            raise ValueError("audit stderr requires review; raw output retained")
        module = _policy_module()
        report = read_json(raw_result.stdout)
        is_clean = module.validate_audit_result(report, raw_result.returncode)

        if not is_clean:
            if context is None:
                # Findings cannot use an exception without a trusted exact
                # identity. classify() rejects after the raw result is saved.
                module.classify(report, raw_result.returncode, {}, "unapproved", 0)
            policy_path = EXCEPTIONS / EXCEPTION_POLICY_PATHS[exception_id]
            record = module._POLICY_RECORDS[exception_id]
            if record["policy_path"] != policy_path.name:
                raise ValueError("static exception policy path is inconsistent")
            policy_bytes = policy_path.read_bytes()
            policy = read_json(policy_bytes.decode("utf-8"))
            receipt["policy_sha256"] = digest(policy_bytes)
            verify_sources(ROOT, policy)

        commands = [
            ([sys.executable, "-m", "unittest", "discover", "-s", "tests", "-p", "test_http_cache_patch.py", "-v"], "installer-tests"),
            (["node", "scripts/bootstrap-node-tools/validate_npm_cli.mjs"], "resolution"),
            (["node", "tests/http-cache-security-regression.mjs"], "behavior-top"),
            (["node", "tests/http-cache-security-regression.mjs", "scripts/bootstrap-node-tools/node_modules/npm/node_modules/http-cache-semantics/index.js"], "behavior-npm"),
        ]
        for argv, label in commands:
            command_result, _ = command(argv, label)
            if command_result.returncode:
                raise ValueError(f"mitigation check failed: {label}")
        for argv, label in [(["node", "--version"], "node-version"), (["git", "rev-parse", "HEAD"], "commit"), (["node", "scripts/bootstrap-node-tools/node_modules/npm/bin/npm-cli.js", "--version"], "npm-version")]:
            command_result, _ = command(argv, label)
            if command_result.returncode:
                raise ValueError(f"runtime metadata failed: {label}")
            receipt[label] = command_result.stdout.strip()
        if receipt["npm-version"] != "12.1.0":
            raise ValueError("npm tool version drift")

        if is_clean:
            receipt["classification"] = "clean"
        else:
            receipt["classification"] = module.classify(
                report, raw_result.returncode, policy, context, pr,
                repository=repository, head_ref=head_ref,
            )
        print(receipt["classification"] + "; raw audit exit=" + str(raw_result.returncode))
        return 0
    except (OSError, ValueError, KeyError, TypeError, AttributeError, subprocess.SubprocessError) as exc:
        receipt["error"] = str(exc)
        print("npm audit gate failed: " + str(exc), file=sys.stderr)
        return 1
    finally:
        receipt["finished_at"] = datetime.now(timezone.utc).isoformat()
        (output / "receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")


if __name__ == "__main__":
    raise SystemExit(main())
