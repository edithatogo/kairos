#!/usr/bin/env python3
"""Prepare and record C2.0 transactional-hook compile-red evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import shutil
import subprocess
import sys
import tarfile
from datetime import datetime, timezone


BASE_COMMIT = "c6d914fff4527f9c415c45df42db509cfdc95342"
CONTRACT_SHA256 = "40c8400a4030a836bcec41e18291cedc8ceee1dcab88fee43f88916863d0d491"
FLOW_SHA256 = "dbcaa36a3314a2a8d6b73056b0bfea3dbc8bbff64e45e01fce8550c28f118e92"
FIXTURE = Path("conformance/c20/flow_domain_plan_c20.rs")
CONTRACT = Path("conductor/design/calibration/c20-transit-execution-v1.md")
FLOW_SOURCE = Path("crates/kairo-ecs-des/src/flow.rs")
TEST_DESTINATION = Path("crates/kairo-ecs-des/tests/flow_domain_plan_c20.rs")
TEST_NAMES = (
    "planner_error_discards_staged_context_and_acquire_but_consumes_source_delivery",
    "poisoned_sink_discards_context_and_commands_even_if_planner_returns_ok",
    "invalid_acquire_batch_discards_context_and_does_not_create_a_request",
    "accepted_plan_commits_context_and_actual_timed_flow_claim_together",
    "legacy_mutable_hook_still_retains_context_effect_on_batch_rejection",
    "plan_carrier_is_unique_kind_bound_and_delivers_typed_pause_resume_in_order",
)
RED_MARKERS = (
    "register_domain_plan_hook",
    "FlowDomainControl",
    "schedule_domain_control",
    "DomainControl",
)
CANONICAL_PATH = "/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin"


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def run_logged(argv: list[str], cwd: Path, env: dict[str, str], logs: Path, name: str):
    completed = subprocess.run(argv, cwd=cwd, env=env, capture_output=True, check=False)
    (logs / f"{name}.stdout.bin").write_bytes(completed.stdout)
    (logs / f"{name}.stderr.bin").write_bytes(completed.stderr)
    return completed


def safe_extract(archive_path: Path, destination: Path) -> None:
    destination.mkdir(parents=True, exist_ok=True)
    with tarfile.open(archive_path, "r:") as archive:
        members = archive.getmembers()
        for member in members:
            name = PurePosixPath(member.name)
            if name.is_absolute() or ".." in name.parts:
                raise ValueError(f"unsafe archive member: {member.name}")
        archive.extractall(destination, members=members, filter="data")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--expect", choices=("red", "green"), default="red")
    expectation = parser.parse_args().expect
    repository = Path(__file__).resolve().parents[2]
    artifact_root = repository / ".artifacts/mvp/C2.0.red-tests.transaction-hook"
    logs_root = artifact_root / "logs"
    disposable_root = artifact_root / "disposable"
    logs_root.mkdir(parents=True, exist_ok=True)
    disposable_root.mkdir(parents=True, exist_ok=True)

    attempt_id = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%S.%fZ")
    attempt_logs = logs_root / attempt_id
    attempt_work = disposable_root / attempt_id
    attempt_logs.mkdir()
    attempt_work.mkdir()

    revision = subprocess.run(
        ["git", "rev-parse", "HEAD"], cwd=repository, capture_output=True,
        text=True, check=False,
    )
    if revision.returncode or revision.stdout.strip() != BASE_COMMIT:
        raise RuntimeError(f"base drift: expected {BASE_COMMIT}, got {revision.stdout.strip()}")
    if sha256(repository / CONTRACT) != CONTRACT_SHA256:
        raise RuntimeError("accepted interface contract hash drift")
    if sha256(repository / FLOW_SOURCE) != FLOW_SHA256:
        raise RuntimeError("full Flow input source hash drift")

    rustup = shutil.which("rustup", path=CANONICAL_PATH)
    if rustup != "/opt/homebrew/bin/rustup":
        raise RuntimeError(f"canonical rustup unavailable: {rustup!r}")
    env = os.environ.copy()
    env["PATH"] = CANONICAL_PATH
    env["CARGO_TARGET_DIR"] = str(attempt_work / "target")

    toolchain = []
    for name, argv in (
        ("rustc-version", [rustup, "run", "1.99.0", "rustc", "--version"]),
        ("cargo-version", [rustup, "run", "1.99.0", "cargo", "--version"]),
    ):
        result = run_logged(argv, repository, env, attempt_logs, name)
        if result.returncode:
            raise RuntimeError(f"toolchain probe failed: {name} exit {result.returncode}")
        toolchain.append({
            "name": name,
            "argv": argv,
            "exit_code": result.returncode,
            "stdout": result.stdout.decode("utf-8", "replace").strip(),
        })

    archive_path = attempt_work / "source.tar"
    with archive_path.open("wb") as archive_file:
        archive = subprocess.run(
            ["git", "archive", "--format=tar", BASE_COMMIT], cwd=repository,
            stdout=archive_file, stderr=subprocess.PIPE, check=False,
        )
    (attempt_logs / "git-archive.stderr.bin").write_bytes(archive.stderr)
    if archive.returncode:
        raise RuntimeError(f"git archive failed: exit {archive.returncode}")
    source_root = attempt_work / "source"
    safe_extract(archive_path, source_root)
    destination = source_root / TEST_DESTINATION
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(repository / FIXTURE, destination)
    if sha256(destination) != sha256(repository / FIXTURE):
        raise RuntimeError("fixture overlay hash mismatch")

    command = [
        rustup, "run", "1.99.0", "cargo", "test",
        "--manifest-path", str(source_root / "Cargo.toml"),
        "-p", "kairo-ecs-des", "--test", "flow_domain_plan_c20", "--locked",
    ]
    if expectation == "red":
        command.append("--no-run")
    completed = run_logged(command, source_root, env, attempt_logs, f"cargo-{expectation}")
    compiler_text = completed.stdout.decode("utf-8", "replace") + completed.stderr.decode(
        "utf-8", "replace"
    )
    errors = []
    observed_tests = [
        name for name in TEST_NAMES
        if f"test {name} ... ok" in compiler_text
    ]
    if expectation == "red":
        missing_markers = [marker for marker in RED_MARKERS if marker not in compiler_text]
        error_lines = [line.strip() for line in compiler_text.splitlines() if line.lstrip().startswith("error[")]
        unexpected_errors = [line for line in error_lines if not any(marker in line for marker in RED_MARKERS)]
        if completed.returncode != 101:
            errors.append(f"expected compile-red exit 101, observed {completed.returncode}")
        if missing_markers:
            errors.append(f"missing expected planned-hook API diagnostics: {missing_markers}")
        if unexpected_errors:
            errors.append(f"unexpected compiler errors outside planned-hook API: {unexpected_errors}")
        if observed_tests:
            errors.append(f"red expectation unexpectedly observed runtime test passes: {observed_tests}")
    else:
        missing_tests = [name for name in TEST_NAMES if name not in observed_tests]
        expected_summary = f"{len(TEST_NAMES)} passed; 0 failed; 0 ignored"
        if completed.returncode != 0:
            errors.append(f"expected runtime-green exit 0, observed {completed.returncode}")
        if missing_tests:
            errors.append(f"named fixture tests not observed passing: {missing_tests}")
        if expected_summary not in compiler_text:
            errors.append(f"missing exact zero-failure/zero-ignored summary: {expected_summary!r}")

    format_command = [
        rustup, "run", "1.99.0", "rustfmt", "--check", "--config",
        "skip_children=true", str(FIXTURE),
    ]
    format_result = run_logged(format_command, repository, env, attempt_logs, "rustfmt")
    if format_result.returncode:
        errors.append(f"rustfmt check failed: exit {format_result.returncode}")

    diff_command = ["git", "diff", "--check"]
    diff_result = run_logged(diff_command, repository, env, attempt_logs, "diff-check")
    if diff_result.returncode:
        errors.append(f"git diff --check failed: exit {diff_result.returncode}")

    record = {
        "schema_version": 1,
        "packet_id": "C2.0.red-tests.transaction-hook",
        "instance_id": "transaction-hook",
        "status": "ready_for_review" if not errors else "blocked",
        "expectation": expectation,
        "acceptance": (
            "compile-red preparation only; no runtime behavior passed"
            if expectation == "red"
            else "green compile and named tests observed; unverified pending independent review"
        ),
        "base_commit": BASE_COMMIT,
        "attempt_id": attempt_id,
        "input_hashes": {
            str(CONTRACT): CONTRACT_SHA256,
            str(FLOW_SOURCE): FLOW_SHA256,
        },
        "fixture_sha256": sha256(repository / FIXTURE),
        "archive_sha256": sha256(archive_path),
        "toolchain": toolchain,
        "environment": {
            "PATH": CANONICAL_PATH,
            "CARGO_TARGET_DIR": env["CARGO_TARGET_DIR"],
        },
        "cargo_expectation": {
            "mode": expectation,
            "argv": command,
            "cwd": str(source_root),
            "exit_code": completed.returncode,
            "stdout_path": str((attempt_logs / "cargo-red.stdout.bin").relative_to(repository)),
            "stderr_path": str((attempt_logs / "cargo-red.stderr.bin").relative_to(repository)),
            "diagnostic_markers": [marker for marker in RED_MARKERS if marker in compiler_text],
            "passing_fixture_tests": observed_tests,
        },
        "verification": [
            {
                "argv": format_command,
                "cwd": str(repository),
                "exit_code": format_result.returncode,
                "stdout_path": str((attempt_logs / "rustfmt.stdout.bin").relative_to(repository)),
                "stderr_path": str((attempt_logs / "rustfmt.stderr.bin").relative_to(repository)),
            },
            {
                "argv": diff_command,
                "cwd": str(repository),
                "exit_code": diff_result.returncode,
                "stdout_path": str((attempt_logs / "diff-check.stdout.bin").relative_to(repository)),
                "stderr_path": str((attempt_logs / "diff-check.stderr.bin").relative_to(repository)),
            },
        ],
        "errors": errors,
        "scope": [str(FIXTURE), "conformance/c20/run_hook_red.py"],
    }
    result_path = artifact_root / "result.json"
    result_path.write_text(json.dumps(record, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({
        "status": record["status"],
        "expectation": expectation,
        "compile_exit": completed.returncode,
        "diagnostic_markers": record["cargo_expectation"]["diagnostic_markers"],
        "passing_fixture_tests": observed_tests,
        "result": str(result_path.relative_to(repository)),
        "logs": str(attempt_logs.relative_to(repository)),
        "errors": errors,
    }, indent=2))
    return 0 if not errors else 1


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:  # preserve bounded failure as a concise runner error
        print(f"hook-red runner blocked: {error}", file=sys.stderr)
        raise SystemExit(2)
