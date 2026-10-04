#!/usr/bin/env python3
"""Retain actual npm audit evidence; accept no vulnerability exceptions."""
from __future__ import annotations

import argparse
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[2]

# These are the reviewed bootstrap inputs at 61b3fb601ef5d765df399c03e7b11fb03a2dbe74.
# A source change requires a reviewed update to this gate's binding.
EXPECTED_SOURCE_SHA256 = {
    "scripts/bootstrap-node-tools/package.json": "4e8bf29c554e486e6ba942c452b7dc58ff0856f769bb52de2bbb2e65690c8628",
    "scripts/bootstrap-node-tools/package-lock.json": "3928f3049db0d21170bbf8fb564715eeb50d59f14b0a27ebcaa42381071b999e",
    "scripts/bootstrap-node-tools/prepare_npm_cli.py": "2749d7c0e2e4ece4bff92a3ddef376f89203d91806ccb6620d88bdd4f452b29f",
    "scripts/bootstrap-node-tools/validate_npm_cli.mjs": "f91c3049e700f0ae98270af9635e9c952e248ddfa7544873cd9ab32e84d8737e",
}
EXPECTED_NPM_CLI_SHA256 = "8e5f6f3429f8cdbe693cdc29904e9d5a7b127a494bd15c804bd54c7403bfcbe7"
SOURCE_COMMIT = "61b3fb601ef5d765df399c03e7b11fb03a2dbe74"
VULNERABILITY_COUNTERS = {"info", "low", "moderate", "high", "critical", "total"}
DEPENDENCY_COUNTERS = {"prod", "dev", "optional", "peer", "peerOptional", "total"}


def unique_object(pairs):
    value = {}
    for key, item in pairs:
        if key in value:
            raise ValueError("duplicate JSON key: " + key)
        value[key] = item
    return value


def nonnegative_integer_map(value, keys, label):
    if not isinstance(value, dict) or set(value) != keys:
        raise ValueError("invalid audit " + label + " counter schema")
    for key in keys:
        count = value[key]
        if type(count) is not int or count < 0:
            raise ValueError("invalid audit " + label + " counter: " + key)
    return value


def classify(stdout, stderr, exit_code):
    if type(exit_code) is not int or exit_code != 0 or stderr:
        raise ValueError("audit did not complete cleanly")
    report = json.loads(stdout, object_pairs_hook=unique_object)
    if not isinstance(report, dict) or type(report.get("auditReportVersion")) is not int or report["auditReportVersion"] != 2:
        raise ValueError("unsupported audit report")
    if report.get("vulnerabilities") != {}:
        raise ValueError("audit findings or missing vulnerability map")
    metadata = report.get("metadata")
    if not isinstance(metadata, dict):
        raise ValueError("invalid audit metadata")
    counters = nonnegative_integer_map(metadata.get("vulnerabilities"), VULNERABILITY_COUNTERS, "vulnerability")
    nonnegative_integer_map(metadata.get("dependencies"), DEPENDENCY_COUNTERS, "dependency")
    if any(counters[key] != 0 for key in VULNERABILITY_COUNTERS):
        raise ValueError("audit findings or nonzero vulnerability counters")
    return "passed_zero_reported_vulnerabilities"


def _sha256_file(path, label):
    path = Path(path)
    resolved = path.resolve(strict=True)
    if not resolved.is_file():
        raise ValueError(label + " must resolve to a regular file")
    return resolved, hashlib.sha256(resolved.read_bytes()).hexdigest()


def _source_hashes(root, expected):
    observed = {}
    for name, expected_hash in expected.items():
        path = Path(root) / name
        if path.is_symlink() or not path.is_file():
            raise ValueError("missing or symlink proof source: " + name)
        observed[name] = hashlib.sha256(path.read_bytes()).hexdigest()
        if observed[name] != expected_hash:
            raise ValueError("reviewed source hash mismatch: " + name)
    return observed


def _tool_state(root, node_path, node_sha256, expected_npm_cli_sha256):
    if not re.fullmatch(r"[0-9a-f]{64}", node_sha256):
        raise ValueError("--node-sha256 must be a lowercase SHA-256 digest")
    if not Path(node_path).is_absolute():
        raise ValueError("--node-path must be an absolute executable path")
    node_resolved, node_actual = _sha256_file(node_path, "Node executable")
    if node_actual != node_sha256:
        raise ValueError("Node executable SHA-256 mismatch")
    npm_path = Path(root) / "scripts/bootstrap-node-tools/node_modules/npm/bin/npm-cli.js"
    npm_resolved, npm_actual = _sha256_file(npm_path, "npm CLI")
    if not npm_resolved.is_relative_to(Path(root).resolve()):
        raise ValueError("npm CLI resolves outside the bootstrap checkout")
    if npm_actual != expected_npm_cli_sha256:
        raise ValueError("reviewed npm CLI SHA-256 mismatch")
    return {
        "node_path_argument": str(node_path),
        "node_path_resolved": str(node_resolved),
        "node_sha256": node_actual,
        "npm_cli_path_resolved": str(npm_resolved),
        "npm_cli_sha256": npm_actual,
    }


def run_gate(root, output, *, node_path, node_sha256, _execute=subprocess.run,
             _expected_source_sha256=EXPECTED_SOURCE_SHA256,
             _expected_npm_cli_sha256=EXPECTED_NPM_CLI_SHA256):
    """Run the production gate; underscored injection arguments are for offline tests only."""
    root, output = Path(root).resolve(), Path(output)
    output.mkdir(parents=True, exist_ok=False)
    npm_relative = "scripts/bootstrap-node-tools/node_modules/npm/bin/npm-cli.js"
    receipt = {
        "classification": "failed",
        "cwd": str(root),
        "reviewed_source_reference_commit": SOURCE_COMMIT,
        "started_at": datetime.now(timezone.utc).isoformat(),
        "checks": [],
        "raw_audit": {"status": "not_executed"},
        "scope": "Private bootstrap tree audit; not public advisory closure or release acceptance",
        "tool_pin_boundary": "Node path and SHA-256 are supplied by the trusted CI tool setup; this runner records and enforces that pin.",
    }

    def verify_inputs():
        receipt["source_sha256_expected"] = dict(_expected_source_sha256)
        receipt["source_sha256_observed"] = _source_hashes(root, _expected_source_sha256)
        receipt["tool_hashes"] = _tool_state(root, node_path, node_sha256, _expected_npm_cli_sha256)

    def command(argv, label):
        verify_inputs()
        record = {"argv": [str(part) for part in argv], "cwd": str(root), "started_at": datetime.now(timezone.utc).isoformat()}
        try:
            result = _execute(argv, cwd=root, text=True, capture_output=True, timeout=180)
            stdout, stderr, code = result.stdout, result.stderr, result.returncode
        except subprocess.TimeoutExpired as error:
            stdout, stderr, code = error.stdout or "", error.stderr or "", None
            record["timed_out"] = True
        except OSError as error:
            stdout, stderr, code = "", str(error), None
            record["start_failed"] = True
        stdout = stdout.decode(errors="replace") if isinstance(stdout, bytes) else stdout
        stderr = stderr.decode(errors="replace") if isinstance(stderr, bytes) else stderr
        for channel, data in (("stdout", stdout), ("stderr", stderr)):
            path = output / (label + "." + channel)
            path.write_text(data)
            record[channel + "_sha256"] = hashlib.sha256(path.read_bytes()).hexdigest()
        record.update(exit=code, finished_at=datetime.now(timezone.utc).isoformat())
        receipt["checks"].append(record)
        if label == "raw-audit":
            receipt["raw_audit"] = {"status": "executed" if code is not None else "failed_execution", **record}
        verify_inputs()
        if type(code) is not int:
            raise ValueError("command failed or timed out: " + label)
        return stdout, stderr, code

    try:
        verify_inputs()
        node = receipt["tool_hashes"]["node_path_resolved"]
        npm = (root / npm_relative).resolve(strict=True)
        for argv, label in [
            (["git", "rev-parse", "HEAD"], "commit"),
            ([node, "--version"], "node-version"),
            ([node, str(npm), "--version"], "npm-version"),
            ([node, str(root / "scripts/bootstrap-node-tools/validate_npm_cli.mjs")], "resolution"),
        ]:
            stdout, stderr, code = command(argv, label)
            if code != 0 or stderr:
                raise ValueError("preflight failed: " + label)
            receipt[label] = stdout.strip()
        if receipt["npm-version"] != "12.1.0":
            raise ValueError("npm version differs")
        stdout, stderr, code = command([
            node, str(npm), "audit", "--prefix", "scripts/bootstrap-node-tools", "--json",
            "--registry=https://registry.npmjs.org/", "--update-notifier=false",
        ], "raw-audit")
        receipt["classification"] = classify(stdout, stderr, code)
    except Exception as error:
        receipt["classification"] = "failed"
        receipt["reason"] = str(error)
    finally:
        receipt["finished_at"] = datetime.now(timezone.utc).isoformat()
        (output / "receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")
    return 0 if receipt["classification"] == "passed_zero_reported_vulnerabilities" else 1


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report-dir", type=Path, default=ROOT / "artifacts/npm-audit-gate/strict")
    parser.add_argument("--node-path", type=Path, required=True)
    parser.add_argument("--node-sha256", required=True)
    args = parser.parse_args()
    return run_gate(ROOT, args.report_dir, node_path=args.node_path, node_sha256=args.node_sha256)


if __name__ == "__main__":
    raise SystemExit(main())
