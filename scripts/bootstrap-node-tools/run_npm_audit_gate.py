#!/usr/bin/env python3
"""Retain raw npm audit evidence and apply only approved EXC-193 policy."""
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

EXPECTED_FILE_HASHES = {'scripts/bootstrap-node-tools/package-lock.json': '3905b6f36ea3b5625667f3a40a829e8eb7a351f6ff074372863f02b1d2216552', 'scripts/bootstrap-node-tools/apply_http_cache_fix.py': 'a928e08eacca08e497199ab58747ff751041643748a0f53ff988dbd7d2d0aa91', 'scripts/bootstrap-node-tools/validate_npm_cli.mjs': '3757fcb8d2bc16ba842cc5f3868c1f09de591ef50a599f56c83ab1d51397b9a4', 'tests/test_http_cache_patch.py': '542a970f0cf79242334b274cd93ec60ee32fb05b375d2302591cb822550479b9', 'tests/http-cache-security-regression.mjs': '298c3537d14c19afdc188cde554596d4f2fc4f95c56d387b79af9ab851f510b2'}
EXPECTED_AUDIT_COMMAND = ['node', 'scripts/bootstrap-node-tools/node_modules/npm/bin/npm-cli.js', 'audit', '--prefix', 'scripts/bootstrap-node-tools', '--audit-level=moderate', '--json']
EXPECTED_PATCHED_HASH = 'fc7b3f0265b7a7d0fee83bafa47186a66495720d3179801c2be3083de6d0cf76'
EXPECTED_GRAPH_HASH = '0b3e5f1d5f65b48f1a20618ba352e6f02529a134f62e0230126ac68c73b5fec8'
EXPECTED_BRANCH = "codex/kairos-implementation-programme"

def digest(data):
    return hashlib.sha256(data).hexdigest()

def unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON key: {key}")
        result[key] = value
    return result

def read_json(text):
    return json.loads(text, object_pairs_hook=unique_object)

def execution_context(policy):
    if os.environ.get("GITHUB_ACTIONS") != "true":
        return "development_pr_193", 193
    if os.environ.get("GITHUB_REPOSITORY") != "edithatogo/kairos":
        raise ValueError("exception cannot apply outside the approved repository")
    event = read_json(Path(os.environ["GITHUB_EVENT_PATH"]).read_text())
    if event.get("repository", {}).get("full_name") != "edithatogo/kairos":
        raise ValueError("unapproved event repository")
    kind = os.environ.get("GITHUB_EVENT_NAME")
    branch = EXPECTED_BRANCH
    if policy["head_ref"] != branch:
        raise ValueError("exception branch drift")
    if kind == "pull_request":
        pr = event["pull_request"]
        if pr["head"]["ref"] != branch or pr["head"]["repo"]["full_name"] != "edithatogo/kairos":
            raise ValueError("unapproved PR source")
        return "development_pr_193", pr["number"]
    if (kind == "workflow_dispatch" and os.environ.get("GITHUB_REF") == "refs/heads/" + branch
            and event.get("ref") in (branch, "refs/heads/" + branch)):
        return "development_pr_193", 193
    raise ValueError("exception cannot apply to this event or branch")

def verify_sources(root, policy):
    if policy["file_sha256"] != EXPECTED_FILE_HASHES:
        raise ValueError("approved source fingerprints changed")
    if policy["patched_index_sha256"] != EXPECTED_PATCHED_HASH:
        raise ValueError("approved patch hash changed")
    if policy["raw_audit_command"] != EXPECTED_AUDIT_COMMAND:
        raise ValueError("raw audit command changed")
    if policy["vulnerabilities_sha256"] != EXPECTED_GRAPH_HASH:
        raise ValueError("approved finding graph changed")
    if policy["expires_at"] != "2026-10-10T00:00:00+10:00":
        raise ValueError("approved expiry changed")
    if set(policy["file_sha256"]) != PROOFS:
        raise ValueError("proof manifest must bind all required controls")
    for name, expected in policy["file_sha256"].items():
        path = root / name
        if path.is_symlink() or digest(path.read_bytes()) != expected:
            raise ValueError(f"proof drift: {name}")
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
        policy = read_json((EXCEPTIONS / "EXC-193-http-cache.json").read_text())
        receipt["policy_sha256"] = digest((EXCEPTIONS / "EXC-193-http-cache.json").read_bytes())
        context, pr = execution_context(policy)
        verify_sources(ROOT, policy)
        commands = [
            ([sys.executable, "-m", "unittest", "discover", "-s", "tests", "-p", "test_http_cache_patch.py", "-v"], "installer-tests"),
            (["node", "scripts/bootstrap-node-tools/validate_npm_cli.mjs"], "resolution"),
            (["node", "tests/http-cache-security-regression.mjs"], "behavior-top"),
            (["node", "tests/http-cache-security-regression.mjs", "scripts/bootstrap-node-tools/node_modules/npm/node_modules/http-cache-semantics/index.js"], "behavior-npm"),
        ]
        for argv, label in commands:
            result, _ = command(argv, label)
            if result.returncode:
                raise ValueError(f"mitigation check failed: {label}")
        for argv, label in [(["node", "--version"], "node-version"), (["git", "rev-parse", "HEAD"], "commit"), (["node", "scripts/bootstrap-node-tools/node_modules/npm/bin/npm-cli.js", "--version"], "npm-version")]:
            result, _ = command(argv, label)
            if result.returncode:
                raise ValueError(f"runtime metadata failed: {label}")
            receipt[label] = result.stdout.strip()
        if receipt["npm-version"] != "12.1.0":
            raise ValueError("npm tool version drift")
        receipt["raw_audit"] = {"status": "started", "argv": policy["raw_audit_command"]}
        result, raw = command(policy["raw_audit_command"], "raw-audit")
        receipt["raw_audit"] = {"status": "executed", **raw}
        module_path = EXCEPTIONS / "npm_audit_policy.py"
        spec = importlib.util.spec_from_file_location("npm_audit_policy", module_path)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        report = read_json(result.stdout)
        if report.get("vulnerabilities") and result.stderr:
            raise ValueError("audit stderr requires review; cannot apply exception")
        receipt["classification"] = module.classify(report, result.returncode, policy, context, pr)
        print(receipt["classification"] + "; raw audit exit=" + str(result.returncode))
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
