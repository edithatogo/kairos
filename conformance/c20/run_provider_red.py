#!/usr/bin/env python3
"""Validate the expected missing-API native red on an isolated Git archive."""

from __future__ import annotations

import hashlib
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import sys
import argparse
import re
from collections import Counter

PACKET_ID = "C2.0.red-tests.provider"
BASE_COMMIT = "c6d914fff4527f9c415c45df42db509cfdc95342"
FIXTURE = Path("conformance/c20/work_duration_c20.rs")
PACKAGE = "kairo-ecs-calibration"
TEST_NAME = "work_duration_c20"
ARTIFACT = Path(".artifacts/mvp/C2.0.red-tests.provider")
TOOLCHAIN = Path("/Users/doughnut/.rustup/toolchains/1.99.0-aarch64-apple-darwin/bin")
EXPECTED_TESTS = {
    "canonical_golden_samples_and_draw_positions_match",
    "support_order_is_preserved_and_equal_order_is_repeatable",
    "exact_cumulative_boundaries_select_the_next_support_bucket",
    "real_large_total_rejects_first_draw_and_counts_both_transitions",
    "fixed_u128_max_is_lossless_and_consumes_no_draws",
    "wrong_purpose_precedes_key_mismatch_and_failure_preserves_stream",
    "wrong_expected_task_is_identity_mismatch_without_stream_advance",
    "fixed_distribution_also_validates_purpose_and_identity",
    "distribution_constructors_return_exact_typed_errors",
    "provider_version_validation_precedes_strata_and_ids_are_fail_closed",
    "malformed_and_missing_lookups_preserve_stream_and_do_not_alias",
    "provider_dispatches_exact_stratum_and_advances_only_on_success",
}


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def run(argv: list[str], cwd: Path, env: dict[str, str] | None = None) -> subprocess.CompletedProcess[str]:
    return subprocess.run(argv, cwd=cwd, env=env, text=True, stdout=subprocess.PIPE,
                          stderr=subprocess.STDOUT, check=False)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--expect", choices=("red", "green"), default="red")
    expectation = parser.parse_args().expect
    root = Path(__file__).resolve().parents[2]
    logs = root / ARTIFACT / "logs"
    disposable = root / ARTIFACT / "disposable"
    logs.mkdir(parents=True, exist_ok=True)
    if disposable.exists():
        shutil.rmtree(disposable)
    disposable.mkdir(parents=True)

    head = run(["git", "rev-parse", "HEAD"], root)
    if head.returncode != 0:
        raise RuntimeError("cannot resolve committed source revision")
    commit = head.stdout.strip()
    status = run(["git", "status", "--porcelain", "--untracked-files=no"], root)
    if status.returncode != 0 or status.stdout.strip():
        raise RuntimeError("runner requires committed tracked source changes")

    fixture_bytes = (root / FIXTURE).read_bytes()
    archive = subprocess.run(["git", "archive", "--format=tar", "HEAD"], cwd=root,
                             stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False)
    if archive.returncode != 0:
        raise RuntimeError("git archive failed: " + archive.stderr.decode(errors="replace")[-2000:])
    with tarfile.open(fileobj=io.BytesIO(archive.stdout), mode="r:") as tar:
        tar.extractall(disposable, filter="data")

    overlay = disposable / "crates/kairo-ecs-calibration/tests/work_duration_c20.rs"
    overlay.parent.mkdir(parents=True, exist_ok=True)
    overlay.write_bytes(fixture_bytes)
    target = disposable / ARTIFACT / "target"
    env = os.environ.copy()
    env["PATH"] = f"{TOOLCHAIN}:/opt/homebrew/bin:/usr/bin:/bin:/Users/doughnut/.cargo/bin"
    env["RUSTC"] = str(TOOLCHAIN / "rustc")
    env["RUSTDOC"] = str(TOOLCHAIN / "rustdoc")
    env["CARGO_TARGET_DIR"] = str(target)
    argv = ["rustup", "run", "1.99.0", "cargo", "test", "--locked", "-p", PACKAGE,
            "--test", TEST_NAME]
    if expectation == "green":
        argv.extend(["--", "--nocapture"])
    result = run(argv, disposable, env)
    log_path = logs / "cargo-native-red.log"
    log_path.write_text(result.stdout)
    missing_source = disposable / "crates/kairo-ecs-calibration/src/work_duration.rs"
    try:
        missing_source.stat()
        missing_source_errno = None
        missing_source_error = None
    except FileNotFoundError as exc:
        missing_source_errno = exc.errno
        missing_source_error = os.strerror(exc.errno)
    missing_diagnostic = re.search(
        r"error: couldn't (?:find|read) file `?[^`\n]*work_duration\.rs`?",
        result.stdout,
    )
    expected_missing_api = (
        result.returncode == 101
        and missing_diagnostic is not None
        and missing_source_errno == 2
        and missing_source_error == "No such file or directory"
    )
    observed_test_results = re.findall(
        r"^test ([A-Za-z0-9_]+) \.\.\. (ok|FAILED|ignored)$", result.stdout, re.MULTILINE
    )
    observed_counts = Counter(name for name, _status in observed_test_results)
    required_test_passes = {
        name: observed_counts.get(name, 0) == 1
        and (name, "ok") in observed_test_results
        and (name, "FAILED") not in observed_test_results
        and (name, "ignored") not in observed_test_results
        for name in EXPECTED_TESTS
    }
    summary = re.search(
        r"test result: ok\. (\d+) passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;",
        result.stdout,
    )
    green_verified = (
        result.returncode == 0
        and all(required_test_passes.values())
        and summary is not None
        and int(summary.group(1)) >= len(EXPECTED_TESTS)
    )
    verified = expected_missing_api if expectation == "red" else green_verified
    receipt = {
        "packet_id": PACKET_ID,
        "task_status": "ready_for_review" if verified else "blocked",
        "expectation": expectation,
        "green_expectation_unverified": expectation == "red",
        "claim": (
            "expected native missing-work_duration API red preparation only; not runtime pass or capability acceptance"
            if expectation == "red"
            else "green mode requires all 12 named fixture tests to run and pass; fixture success is not capability acceptance"
        ),
        "base_commit": BASE_COMMIT,
        "committed_workspace": commit,
        "fixture": str(FIXTURE),
        "fixture_sha256": sha256(fixture_bytes),
        "overlay_path": "crates/kairo-ecs-calibration/tests/work_duration_c20.rs",
        "toolchain": {
            "version": "1.99.0",
            "rustc": str(TOOLCHAIN / "rustc"),
            "rustdoc": str(TOOLCHAIN / "rustdoc"),
            "path_prefix": str(TOOLCHAIN),
        },
        "command": {
            "argv": argv,
            "cwd": str(disposable),
            "exit_status": result.returncode,
            "expected_exit_status": 101 if expectation == "red" else 0,
            "expected_missing_api_signature": expected_missing_api,
            "missing_api_compiler_diagnostic": missing_diagnostic.group(0) if missing_diagnostic else None,
            "missing_source_errno": missing_source_errno,
            "missing_source_error": missing_source_error,
            "expected_test_names": sorted(EXPECTED_TESTS) if expectation == "green" else [],
            "observed_test_results": [
                {"name": name, "status": status} for name, status in observed_test_results
            ],
            "required_test_passes": required_test_passes if expectation == "green" else {},
            "cargo_test_summary": summary.group(0) if summary else None,
            "green_oracle": green_verified if expectation == "green" else None,
            "log": str(log_path.relative_to(root)),
            "log_sha256": sha256(result.stdout.encode()),
        },
        "limitations": [
            "The disposable archive overlays the actual fixture into the native calibration crate test target.",
            "The nonzero native result is expected because work_duration.rs is absent; it is not behavioral qualification.",
            "No live crate source, Cargo manifest, or lockfile was changed.",
            "Runtime tests must be rerun after the reviewed provider implementation exists.",
        ],
    }
    result_path = root / ARTIFACT / "result.json"
    result_path.write_text(json.dumps(receipt, indent=2, sort_keys=True) + "\n")
    print(json.dumps({
        "task_status": receipt["task_status"],
        "committed_workspace": commit,
        "fixture_sha256": receipt["fixture_sha256"],
        "native_exit": result.returncode,
        "expected_red": expected_missing_api,
        "log": receipt["command"]["log"],
        "result": str(result_path.relative_to(root)),
    }, sort_keys=True))
    return 0 if verified else 1


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as exc:  # Preserve a concise runner failure for the reviewer.
        print(f"runner error: {exc}", file=sys.stderr)
        raise SystemExit(2)
