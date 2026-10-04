#!/usr/bin/env python3
"""Bounded public FlowRuntime queue/preemption benchmark collector."""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import subprocess
import sys
import time
from datetime import datetime, timezone

ROOT = Path(__file__).resolve().parents[2]
SCENARIOS = ("fifo", "priority", "churn", "interruptions", "many_resources")
SIZES = (10, 1_000, 100_000)
CAPACITIES = (1, 10, 100)
SEED_DEFAULT = 42
SOURCE_PATHS = (
    "AGENTS.md",
    "crates/kairo-ecs-des/src/flow.rs",
    "crates/kairo-ecs-des/src/lib.rs",
    "crates/kairo-ecs-des/examples/flow_queue_benchmark_v1.rs",
    "benches/queue/run_v1.py",
)
CONTRACT_SHA256 = "dbd926007389daff42eee989402edb559dea78552617350c072dff6046eb0f4f"
RELEASE_BUILD_PREFIX = ["rustup", "run", "1.99.0", "cargo", "build", "--locked", "--release",
                        "-p", "kairo-ecs-des", "--example", "flow_queue_benchmark_v1"]
DEFAULT_TARGET_DIR = ".artifacts/q52-runtime/target"
BUILD_INPUT_PATHS = (
    "Cargo.lock", "Cargo.toml", "crates/kairo-ecs-des/Cargo.toml", *SOURCE_PATHS,
)


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def matrix_cases() -> list[dict[str, int | str]]:
    cases: list[dict[str, int | str]] = []
    for scenario in ("fifo", "priority", "churn"):
        for capacity in CAPACITIES:
            for n in SIZES:
                cases.append({"scenario": scenario, "n": n, "resources": 1, "capacity": capacity})
    for resources in (10, 100):
        for n in SIZES:
            cases.append({"scenario": "many_resources", "n": n, "resources": resources, "capacity": 1})
    for capacity in (1, 10):
        for n in SIZES:
            cases.append({"scenario": "interruptions", "n": n, "resources": 1, "capacity": capacity})
    return cases


def case_key(case: dict[str, int | str]) -> str:
    return "{scenario}-n{n}-r{resources}-c{capacity}".format(**case)


def _wait_child(process: subprocess.Popen[bytes], timeout_seconds: float) -> tuple[bool, int, object]:
    """Wait with wait4 so max RSS belongs to this child, not cumulative children."""
    if not hasattr(os, "wait4"):
        raise RuntimeError("per-child peak RSS requires os.wait4 on this platform")
    deadline = time.monotonic() + timeout_seconds
    while True:
        pid, status, usage = os.wait4(process.pid, os.WNOHANG)
        if pid == process.pid:
            process.returncode = os.waitstatus_to_exitcode(status)
            return False, process.returncode, usage
        if time.monotonic() >= deadline:
            try:
                process.kill()
            except ProcessLookupError:
                pass
            _, status, usage = os.wait4(process.pid, 0)
            process.returncode = os.waitstatus_to_exitcode(status)
            return True, process.returncode, usage
        time.sleep(min(0.02, max(0.0, deadline - time.monotonic())))


def peak_rss_bytes(usage: object, system: str | None = None) -> int:
    value = int(getattr(usage, "ru_maxrss"))
    system = system or platform.system()
    if system == "Darwin":
        return value
    if system == "Linux":
        return value * 1024
    raise RuntimeError(f"unverified ru_maxrss units for {system}")


def _is_int(value: object) -> bool:
    return isinstance(value, int) and not isinstance(value, bool)


def _expected_counts(case: dict[str, int | str]) -> dict[str, object]:
    n = int(case["n"])
    resources = int(case["resources"])
    capacity = int(case["capacity"])
    scenario = str(case["scenario"])
    slots = resources * capacity
    if scenario == "interruptions":
        events, records = capacity + 2 * n, 2 * capacity + 5 * n
        retained, terminal = capacity + n, n
        work_count = capacity + n
        work_states = {"pending": 0, "active": capacity, "suspended": 0,
                       "completed": n, "other_terminal": 0}
        preemptions = resumptions = completions = n
        occupancy = [(capacity, 0)]
    else:
        initial_queued = [n // resources + int(index < n % resources) for index in range(resources)]
        if scenario == "churn":
            rekeyed = n // 2
            final_queued = [rekeyed // resources + int(index < rekeyed % resources)
                            for index in range(resources)]
            events, records = slots + 2 * n, n + 2 * slots + rekeyed
            retained, terminal = slots + n, n - rekeyed
        else:
            final_queued = initial_queued
            events, records = n + slots, n + 2 * slots
            retained, terminal = n + slots, 0
        work_count = 0
        work_states = {"pending": 0, "active": 0, "suspended": 0,
                       "completed": 0, "other_terminal": 0}
        preemptions = resumptions = completions = 0
        occupancy = [(capacity, queued) for queued in final_queued]
    return {
        "events_completed": events, "lifecycle_records": records,
        "retained_request_count": retained, "terminal_request_count": terminal,
        "retained_work_count": work_count, "work_states": work_states,
        "preemptions": preemptions, "resumptions": resumptions,
        "completions": completions, "logical_waiters": n, "occupancy": occupancy,
    }


def _parse_child_row(stdout: bytes, case: dict[str, int | str], seed: int) -> dict[str, object]:
    lines = [line for line in stdout.decode("utf-8", errors="strict").splitlines() if line.strip()]
    if len(lines) != 1:
        raise ValueError(f"expected exactly one JSON row, found {len(lines)}")
    row = json.loads(lines[0])
    if not isinstance(row, dict) or not _is_int(row.get("schema")) or row["schema"] != 1 or row.get("status") != "ok":
        raise ValueError("child row missing schema=1/status=ok")
    if not isinstance(row.get("timing_scope"), str) or not row["timing_scope"]:
        raise ValueError("child row missing timing scope")
    for key, expected in case.items():
        actual = row.get(key)
        matches = _is_int(actual) and actual == expected if isinstance(expected, int) else actual == expected
        if not matches:
            raise ValueError(f"child row case mismatch for {key}: {actual!r} != {expected!r}")
    if not _is_int(row.get("seed")) or row["seed"] != seed:
        raise ValueError("child row seed mismatch")
    required_ints = (
        "setup_ns", "dispatch_ns", "initial_dispatch_ns", "churn_dispatch_ns",
        "events_completed", "lifecycle_records", "logical_waiters",
        "retained_request_count", "terminal_request_count", "retained_work_count",
        "preemptions", "resumptions", "completions",
    )
    for key in required_ints:
        if not _is_int(row.get(key)) or row[key] < 0:
            raise ValueError(f"child row has invalid {key}")
    if row["dispatch_ns"] != row["initial_dispatch_ns"] + row["churn_dispatch_ns"]:
        raise ValueError("dispatch_ns does not equal initial plus churn dispatch timing")
    for key in ("events_per_second", "waiters_per_second"):
        value = row.get(key)
        if isinstance(value, bool) or not isinstance(value, (int, float)) or not math.isfinite(value) or value < 0:
            raise ValueError(f"child row has invalid finite nonnegative {key}")
    expected = _expected_counts(case)
    for key in ("events_completed", "lifecycle_records", "logical_waiters",
                "retained_request_count", "terminal_request_count", "retained_work_count",
                "preemptions", "resumptions", "completions"):
        if row[key] != expected[key]:
            raise ValueError(f"child row {key} is inconsistent with scenario inputs")
    if row["terminal_request_count"] > row["retained_request_count"]:
        raise ValueError("terminal request count exceeds retained request count")
    work_states = row.get("work_states")
    if not isinstance(work_states, dict) or set(work_states) != set(expected["work_states"]):
        raise ValueError("child row work state counts are malformed")
    for key, count in expected["work_states"].items():
        if not _is_int(work_states.get(key)) or work_states[key] != count:
            raise ValueError(f"child row work state {key} is inconsistent with scenario inputs")
    occupancy = row.get("occupancy")
    expected_occupancy = expected["occupancy"]
    if not isinstance(occupancy, list) or len(occupancy) != len(expected_occupancy):
        raise ValueError("child row occupancy has wrong resource count")
    for index, (actual, expected_pair) in enumerate(zip(occupancy, expected_occupancy)):
        if not isinstance(actual, dict) or not _is_int(actual.get("resource_index")) or actual["resource_index"] != index:
            raise ValueError("child row occupancy resource index is invalid")
        active, queued = actual.get("active"), actual.get("queued")
        if not _is_int(active) or not _is_int(queued) or (active, queued) != expected_pair:
            raise ValueError(f"child row occupancy mismatch for resource {index}")
    return row


def run_repeat(
    binary: Path,
    case: dict[str, int | str],
    seed: int,
    timeout_seconds: float,
    repeat: int,
    repeat_dir: Path,
) -> dict[str, object]:
    repeat_dir.mkdir(parents=True, exist_ok=False)
    stdout_path = repeat_dir / "stdout.jsonl"
    stderr_path = repeat_dir / "stderr.txt"
    argv = [str(binary), "--scenario", str(case["scenario"]), "--n", str(case["n"]),
            "--resources", str(case["resources"]), "--capacity", str(case["capacity"]), "--seed", str(seed)]
    started = time.monotonic_ns()
    with stdout_path.open("wb") as stdout_file, stderr_path.open("wb") as stderr_file:
        process = subprocess.Popen(argv, cwd=ROOT, stdout=stdout_file, stderr=stderr_file, shell=False)
        timed_out, returncode, usage = _wait_child(process, timeout_seconds)
    elapsed_ns = time.monotonic_ns() - started
    stdout = stdout_path.read_bytes()
    stderr = stderr_path.read_bytes()
    result: dict[str, object] = {
        "repeat": repeat,
        "argv": argv,
        "cwd": str(ROOT),
        "status": "timeout" if timed_out else ("ok" if returncode == 0 else "failed"),
        "returncode": returncode,
        "timed_out": timed_out,
        "wall_elapsed_ns": elapsed_ns,
        "peak_rss_bytes": peak_rss_bytes(usage),
        "peak_rss_method": "wait4 RUSAGE_CHILDREN child ru_maxrss; Darwin bytes, Linux KiB normalized to bytes; includes process setup and child result readback",
        "stdout_path": str(stdout_path.relative_to(ROOT)),
        "stdout_sha256": sha256_bytes(stdout),
        "stderr_path": str(stderr_path.relative_to(ROOT)),
        "stderr_sha256": sha256_bytes(stderr),
    }
    if timed_out:
        result["timeout_phase"] = "dispatch" if b"Q52_PHASE=dispatch" in stderr else "setup"
        result["timeout_termination"] = "direct child process killed; executable is a standalone Rust example"
        result["error"] = f"case exceeded {timeout_seconds:g}s deadline during {result['timeout_phase']}; measurement unresolved"
        return result
    if returncode != 0:
        result["error"] = f"child exited {returncode}"
        return result
    try:
        row = _parse_child_row(stdout, case, seed)
    except (UnicodeDecodeError, json.JSONDecodeError, ValueError) as error:
        result["status"] = "invalid_result"
        result["error"] = str(error)
        return result
    result["row"] = row
    return result


def percentile(values: list[int], fraction: float) -> int | None:
    if not values:
        return None
    ordered = sorted(values)
    # Nearest-rank percentile, explicit for the small fixed repeat count.
    return ordered[max(0, math.ceil(fraction * len(ordered)) - 1)]


def summarize(repeats: list[dict[str, object]]) -> dict[str, object]:
    completed = [item for item in repeats if item.get("status") == "ok" and isinstance(item.get("row"), dict)]
    rows = [item["row"] for item in completed]
    summary: dict[str, object] = {"completed_repeats": len(rows)}
    for field in ("setup_ns", "dispatch_ns", "events_completed", "events_per_second", "waiters_per_second", "preemptions"):
        values = [row[field] for row in rows if isinstance(row.get(field), (int, float)) and not isinstance(row.get(field), bool)]
        summary[field] = {"p50": percentile(values, 0.50), "p95": percentile(values, 0.95), "samples": len(values)}
    rss_values = [int(item["peak_rss_bytes"]) for item in completed if isinstance(item.get("peak_rss_bytes"), int)]
    summary["peak_rss_bytes"] = {"p50": percentile(rss_values, 0.50), "p95": percentile(rss_values, 0.95), "samples": len(rss_values)}
    return summary


def read_toolchain() -> dict[str, object]:
    command = ["rustup", "run", "1.99.0", "rustc", "-Vv"]
    completed = subprocess.run(command, cwd=ROOT, text=True, capture_output=True, check=False)
    return {"argv": command, "returncode": completed.returncode, "stdout": completed.stdout.strip(),
            "stderr_sha256": sha256_bytes(completed.stderr.encode())}


def provenance_snapshot(paths: tuple[str, ...] = SOURCE_PATHS) -> dict[str, object]:
    head = subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True, capture_output=True, check=False)
    status = subprocess.run(["git", "status", "--porcelain=v1"], cwd=ROOT, text=True, capture_output=True, check=False)
    return {
        "git_head": head.stdout.strip() if head.returncode == 0 else None,
        "git_status_porcelain": status.stdout if status.returncode == 0 else None,
        "git_readback_returncodes": {"head": head.returncode, "status": status.returncode},
        "source_hashes": {name: sha256_file(ROOT / name) for name in paths},
    }


def bounded_measurement_provenance_qualified(
    build: dict[str, object] | None,
    start: dict[str, object],
    end: dict[str, object],
    binary_sha_start: str,
    binary_sha_end: str,
    toolchain: dict[str, object],
) -> bool:
    if build is None or toolchain.get("returncode") != 0:
        return False
    version = str(toolchain.get("stdout", "")).splitlines()
    if not version or not version[0].startswith("rustc 1.99.0 "):
        return False
    empty_status = ""
    if build.get("git_status_before") != empty_status or build.get("git_status_after") != empty_status:
        return False
    if start.get("git_status_porcelain") != empty_status or end.get("git_status_porcelain") != empty_status:
        return False
    if build.get("head_drift") or build.get("source_drift"):
        return False
    if build.get("head_before") != build.get("head_after"):
        return False
    if build.get("head_after") != start.get("git_head") or start.get("git_head") != end.get("git_head"):
        return False
    for readbacks in (build.get("git_readback_returncodes_before"),
                      build.get("git_readback_returncodes_after"),
                      start.get("git_readback_returncodes"), end.get("git_readback_returncodes")):
        if not isinstance(readbacks, dict) or readbacks.get("head") != 0 or readbacks.get("status") != 0:
            return False
    build_sources = build.get("source_hashes_after")
    start_sources = start.get("source_hashes")
    end_sources = end.get("source_hashes")
    if not isinstance(build_sources, dict) or not isinstance(start_sources, dict) or not isinstance(end_sources, dict):
        return False
    if any(build_sources.get(name) != digest for name, digest in start_sources.items()):
        return False
    if start_sources != end_sources:
        return False
    built_binary_sha = build.get("binary_sha256")
    return bool(built_binary_sha and built_binary_sha == binary_sha_start == binary_sha_end)


def measurement_platform_error() -> str | None:
    if os.name != "posix" or not callable(getattr(os, "wait4", None)):
        return "unsupported collector platform: per-child peak RSS requires POSIX os.wait4"
    if platform.system() not in ("Darwin", "Linux"):
        return f"unsupported collector platform: ru_maxrss units are not qualified for {platform.system()}"
    return None


def build_target_for_binary(binary: Path | None) -> Path:
    if binary is None:
        return (ROOT / DEFAULT_TARGET_DIR).resolve()
    resolved = binary if binary.is_absolute() else ROOT / binary
    resolved = resolved.resolve()
    expected_tail = Path("release/examples/flow_queue_benchmark_v1")
    if resolved.name != expected_tail.name or resolved.parent.name != "examples" or resolved.parent.parent.name != "release":
        raise ValueError("--build --binary must end in release/examples/flow_queue_benchmark_v1")
    return resolved.parents[2]


def _display_path(path: Path) -> str:
    try:
        return path.relative_to(ROOT).as_posix()
    except ValueError:
        return str(path)


def package_metadata() -> dict[str, str | None]:
    section = None
    found: dict[str, str] = {}
    for raw_line in (ROOT / "crates/kairo-ecs-des/Cargo.toml").read_text().splitlines():
        line = raw_line.strip()
        if line.startswith("[") and line.endswith("]"):
            section = line[1:-1]
        elif section == "package" and "=" in line:
            key, value = (part.strip() for part in line.split("=", 1))
            if key in ("name", "version") and len(value) >= 2 and value[0] == value[-1] == '"':
                found[key] = value[1:-1]
    return {"name": found.get("name"), "version": found.get("version")}


def execute_release_build(target_dir: Path, receipt_dir: Path) -> dict[str, object]:
    target_arg = _display_path(target_dir)
    argv = [*RELEASE_BUILD_PREFIX, "--target-dir", target_arg]
    before = provenance_snapshot(BUILD_INPUT_PATHS)
    build_error = None
    try:
        completed = subprocess.run(argv, cwd=ROOT, text=True, capture_output=True, check=False)
        build_returncode = completed.returncode
        build_stdout, build_stderr = completed.stdout, completed.stderr
    except OSError as error:
        completed = None
        build_error = f"release build launch failed: {error}"
        build_returncode, build_stdout, build_stderr = None, "", build_error
    after = provenance_snapshot(BUILD_INPUT_PATHS)
    binary = target_dir / "release/examples/flow_queue_benchmark_v1"
    receipt_dir.mkdir(parents=True, exist_ok=True)
    log_text = "ARGV: " + json.dumps(argv) + "\nSTDOUT:\n" + build_stdout + "\nSTDERR:\n" + build_stderr
    log_path = receipt_dir / "build.log"
    log_path.write_text(log_text)
    receipt = {
        "argv": argv, "cwd": str(ROOT), "head_before": before["git_head"], "head_after": after["git_head"],
        "git_status_before": before["git_status_porcelain"], "git_status_after": after["git_status_porcelain"],
        "source_hashes_before": before["source_hashes"], "source_hashes_after": after["source_hashes"],
        "source_drift": before["source_hashes"] != after["source_hashes"],
        "head_drift": before["git_head"] != after["git_head"],
        "git_readback_returncodes_before": before["git_readback_returncodes"],
        "git_readback_returncodes_after": after["git_readback_returncodes"],
        "error": build_error,
        "cargo_lock_and_manifest_hashes_included": ["Cargo.lock", "Cargo.toml", "crates/kairo-ecs-des/Cargo.toml"],
        "package": package_metadata(),
        "build_profile": "release" if "--release" in argv else "debug",
        "feature_selection": {"default_features": True, "explicit_features": [],
                              "basis": "reviewed argv omits --no-default-features and --features"},
        "exit_code": build_returncode,
        "stdout_sha256": sha256_bytes(build_stdout.encode()),
        "stderr_sha256": sha256_bytes(build_stderr.encode()),
        "log_path": _display_path(log_path), "log_sha256": sha256_file(log_path),
        "binary_path": _display_path(binary),
        "binary_sha256": sha256_file(binary) if binary.is_file() else None,
        "binary_executable": binary.is_file() and os.access(binary, os.X_OK),
    }
    receipt_path = receipt_dir / "build.json"
    receipt["receipt_path"] = _display_path(receipt_path)
    receipt_path.write_text(json.dumps(receipt, indent=2, sort_keys=True) + "\n")
    return receipt


def cpu_model() -> str | None:
    if platform.system() == "Darwin":
        completed = subprocess.run(["sysctl", "-n", "machdep.cpu.brand_string"], text=True, capture_output=True)
        return completed.stdout.strip() if completed.returncode == 0 else None
    if platform.system() == "Linux":
        try:
            for line in Path("/proc/cpuinfo").read_text(errors="replace").splitlines():
                if line.lower().startswith("model name"):
                    return line.split(":", 1)[1].strip()
        except OSError:
            return None
    return None


def _case_matrix_from_args(args: argparse.Namespace) -> list[dict[str, int | str]]:
    if args.matrix:
        if any(value is not None for value in (args.scenario, args.n, args.resources, args.capacity)):
            raise ValueError("--matrix cannot be combined with single-case selectors")
        return matrix_cases()
    if None in (args.scenario, args.n, args.resources, args.capacity):
        raise ValueError("single-case mode requires --scenario, --n, --resources and --capacity")
    case = {"scenario": args.scenario, "n": args.n, "resources": args.resources, "capacity": args.capacity}
    if args.n <= 0 or args.resources <= 0 or args.capacity <= 0:
        raise ValueError("n, resources and capacity must be positive")
    if args.scenario == "many_resources" and args.capacity != 1:
        raise ValueError("many_resources requires --capacity 1")
    if args.scenario == "interruptions" and args.resources != 1:
        raise ValueError("interruptions requires --resources 1")
    return [case]


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, help="binary to measure; without --build its producer provenance is unverified")
    parser.add_argument("--build", action="store_true", help="build and measure the reviewed Rust 1.99 release example")
    parser.add_argument("--matrix", action="store_true", help="run every frozen runtime case once per repeat")
    parser.add_argument("--scenario", choices=SCENARIOS)
    parser.add_argument("--n", type=int)
    parser.add_argument("--resources", type=int)
    parser.add_argument("--capacity", type=int)
    parser.add_argument("--repeats", type=int, default=5)
    parser.add_argument("--timeout-seconds", type=float, default=30.0)
    parser.add_argument("--seed", type=int, default=SEED_DEFAULT)
    parser.add_argument("--output-dir", type=Path, required=True)
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    if args.repeats < 1 or not math.isfinite(args.timeout_seconds) or args.timeout_seconds <= 0 or args.seed < 0:
        print("repeats, timeout and seed must be positive/nonnegative", file=sys.stderr)
        return 2
    if args.build and args.binary is not None:
        try:
            target_dir = build_target_for_binary(args.binary)
        except ValueError as error:
            print(str(error), file=sys.stderr)
            return 2
        binary = target_dir / "release/examples/flow_queue_benchmark_v1"
        provided_binary = args.binary if args.binary.is_absolute() else ROOT / args.binary
        if provided_binary.resolve() != binary.resolve():
            print("--build binary path does not match the exact release output being built", file=sys.stderr)
            return 2
    elif args.build:
        target_dir = build_target_for_binary(None)
        binary = target_dir / "release/examples/flow_queue_benchmark_v1"
    elif args.binary is not None:
        target_dir = None
        binary = args.binary if args.binary.is_absolute() else ROOT / args.binary
        binary = binary.resolve()
    else:
        print("provide --build or --binary", file=sys.stderr)
        return 2
    platform_error = measurement_platform_error()
    if platform_error:
        print(json.dumps({"status": "unsupported_platform", "error": platform_error}), file=sys.stderr)
        return 2
    try:
        cases = _case_matrix_from_args(args)
    except ValueError as error:
        print(str(error), file=sys.stderr)
        return 2
    if not args.build and (not binary.is_file() or not os.access(binary, os.X_OK)):
        print(f"benchmark executable missing or not executable: {binary}", file=sys.stderr)
        return 2
    run_id = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%S%fZ")
    base_output = args.output_dir if args.output_dir.is_absolute() else ROOT / args.output_dir
    run_dir = base_output / run_id
    run_dir.mkdir(parents=True, exist_ok=False)
    build_receipt = None
    if args.build:
        build_receipt = execute_release_build(target_dir, run_dir / "build")
        if (build_receipt["exit_code"] != 0 or build_receipt["source_drift"] or build_receipt["head_drift"]
                or not build_receipt["binary_executable"]):
            print(json.dumps({"status": "build_failed", "build_receipt": build_receipt["receipt_path"]}, sort_keys=True))
            return 1
    provenance_start = provenance_snapshot()
    sources = provenance_start["source_hashes"]
    producer = "verified_local_release_build" if build_receipt else "unverified_external_binary"
    binary_sha_start = sha256_file(binary)
    input_manifest = {"matrix": "q5.2-runtime-v1", "contract_sha256": CONTRACT_SHA256,
                      "cases": cases, "seed": args.seed, "repeats": args.repeats,
                      "timeout_seconds": args.timeout_seconds, "source_hashes": sources,
                      "binary_sha256": binary_sha_start, "binary_producer_provenance": producer}
    input_hash = sha256_bytes(json.dumps(input_manifest, sort_keys=True, separators=(",", ":")).encode())
    results: list[dict[str, object]] = []
    any_timeout = False
    any_failure = False
    for case in cases:
        repeats: list[dict[str, object]] = []
        for repeat in range(1, args.repeats + 1):
            repeat_dir = run_dir / case_key(case) / f"repeat-{repeat:02d}"
            try:
                result = run_repeat(binary, case, args.seed, args.timeout_seconds, repeat, repeat_dir)
            except OSError as error:
                repeat_dir.mkdir(parents=True, exist_ok=True)
                result = {"repeat": repeat, "status": "failed", "returncode": None, "timed_out": False,
                          "error": f"collector process launch/wait failed: {error}"}
            repeats.append(result)
            if result["status"] == "timeout":
                any_timeout = True
                break  # stop repeats for this case; continue with the next case
            if result["status"] != "ok":
                any_failure = True
                break
        status = "unresolved" if any(row.get("status") == "timeout" for row in repeats) else (
            "complete" if len(repeats) == args.repeats and all(row.get("status") == "ok" for row in repeats) else "failed"
        )
        results.append({"case": case, "case_key": case_key(case), "status": status,
                        "attempted_repeats": len(repeats), "requested_repeats": args.repeats,
                        "repeats": repeats, "summary": summarize(repeats)})
    binary_sha_end = sha256_file(binary)
    provenance_end = provenance_snapshot()
    head_drift = provenance_start["git_head"] != provenance_end["git_head"]
    source_drift = provenance_start["source_hashes"] != provenance_end["source_hashes"]
    binary_drift = binary_sha_start != binary_sha_end
    if (head_drift or source_drift or binary_drift
            or provenance_start["git_readback_returncodes"] != {"head": 0, "status": 0}
            or provenance_end["git_readback_returncodes"] != {"head": 0, "status": 0}):
        any_failure = True
    overall = "partial_unresolved" if any_timeout and any_failure else (
        "unresolved" if any_timeout else ("failed" if any_failure else "complete")
    )
    toolchain = read_toolchain()
    provenance_qualified = bounded_measurement_provenance_qualified(
        build_receipt, provenance_start, provenance_end, binary_sha_start, binary_sha_end, toolchain
    )
    receipt = {
        "schema": 1,
        "status": overall,
        "run_id": run_id,
        "started_utc": run_id,
        "collector_argv": ["python3", *sys.argv] if argv is None else ["python3", "benches/queue/run_v1.py", *argv],
        "cwd": str(ROOT),
        "provenance_start": provenance_start,
        "provenance_end": provenance_end,
        "provenance_drift": {"head": head_drift, "sources": source_drift, "binary": binary_drift},
        "toolchain": toolchain,
        "required_release_build_argv": build_receipt["argv"] if build_receipt else None,
        "build_receipt": build_receipt,
        "binary_producer_provenance": producer,
        "bounded_measurement_provenance_qualified": provenance_qualified,
        "qualification_scope": "bounded measurement source/build provenance only; does not establish Q5.2 performance acceptance",
        "q52_acceptance": "not assessed by runtime collector; requires the complete matrix and coordinator thresholds",
        "measurement_termination": "timeout sends kill to the direct standalone benchmark child; no descendant process tree is created",
        "host": {"platform": platform.platform(), "system": platform.system(), "machine": platform.machine(),
                 "cpu_model": cpu_model(), "python": platform.python_version()},
        "seed": args.seed,
        "input_manifest": input_manifest,
        "input_sha256": input_hash,
        "binary": str(binary),
        "binary_sha256": input_manifest["binary_sha256"],
        "timeout_seconds_per_repeat": args.timeout_seconds,
        "timing_scope": "FlowRuntime step plus lifecycle record classification/counting; full-world staging and commit are included; correctness readbacks are excluded",
        "percentile_method": "nearest-rank p50/p95 on completed raw repeats only; p95 with five repeats is the maximum sample, not a tail estimate",
        "requested_repeats_per_case": args.repeats,
        "case_count": len(cases),
        "cases": results,
        "raw_results_root": str(run_dir.relative_to(ROOT)),
    }
    receipt_path = run_dir / "result.json"
    receipt_path.write_text(json.dumps(receipt, indent=2, sort_keys=True) + "\n")
    print(json.dumps({"status": overall, "result": str(receipt_path.relative_to(ROOT)),
                      "cases": len(results), "timeouts": sum(1 for row in results if row["status"] == "unresolved"),
                      "input_sha256": input_hash}, sort_keys=True))
    return 0 if overall == "complete" else 1


if __name__ == "__main__":
    raise SystemExit(main())
