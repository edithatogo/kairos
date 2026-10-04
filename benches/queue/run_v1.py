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
RELEASE_BUILD_ARGV = ["rustup", "run", "1.99.0", "cargo", "build", "--locked", "--release",
                      "-p", "kairo-ecs-des", "--example", "flow_queue_benchmark_v1",
                      "--target-dir", ".artifacts/q52-runtime/target"]


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


def _expected_counts(case: dict[str, int | str]) -> tuple[int, int, int, list[tuple[int, int]]]:
    n = int(case["n"])
    resources = int(case["resources"])
    capacity = int(case["capacity"])
    scenario = str(case["scenario"])
    if scenario == "interruptions":
        return capacity + 2 * n, capacity + n, n, [(capacity, 0)]
    per_resource = [n // resources + int(index < n % resources) for index in range(resources)]
    active = [(capacity, queued) for queued in per_resource]
    if scenario == "churn":
        rekeyed = [n // 2 // resources + int(index < (n // 2) % resources) for index in range(resources)]
        return resources * capacity + 2 * n, resources * capacity + n, n - n // 2, [(capacity, q) for q in rekeyed]
    return resources * capacity + n, resources * capacity + n, 0, active


def _parse_child_row(stdout: bytes, case: dict[str, int | str], seed: int) -> dict[str, object]:
    lines = [line for line in stdout.decode("utf-8", errors="strict").splitlines() if line.strip()]
    if len(lines) != 1:
        raise ValueError(f"expected exactly one JSON row, found {len(lines)}")
    row = json.loads(lines[0])
    if not isinstance(row, dict) or row.get("schema") != 1 or row.get("status") != "ok":
        raise ValueError("child row missing schema=1/status=ok")
    if not isinstance(row.get("timing_scope"), str) or not row["timing_scope"]:
        raise ValueError("child row missing timing scope")
    for key, expected in case.items():
        if row.get(key) != expected:
            raise ValueError(f"child row case mismatch for {key}: {row.get(key)!r} != {expected!r}")
    if not _is_int(row.get("seed")) or row["seed"] != seed:
        raise ValueError("child row seed mismatch")
    required_ints = (
        "setup_ns", "dispatch_ns", "initial_dispatch_ns", "churn_dispatch_ns",
        "events_completed", "retained_request_count", "terminal_request_count",
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
    expected_events, expected_retained, expected_terminal, expected_occupancy = _expected_counts(case)
    if row["events_completed"] != expected_events or row["events_completed"] <= 0:
        raise ValueError("child row event count is inconsistent with scenario inputs")
    if row["retained_request_count"] != expected_retained:
        raise ValueError("child row retained request count is inconsistent with scenario inputs")
    if row["terminal_request_count"] != expected_terminal or row["terminal_request_count"] > row["retained_request_count"]:
        raise ValueError("child row terminal request count is inconsistent with scenario inputs")
    occupancy = row.get("occupancy")
    if not isinstance(occupancy, list) or len(occupancy) != len(expected_occupancy):
        raise ValueError("child row occupancy has wrong resource count")
    for index, (actual, expected) in enumerate(zip(occupancy, expected_occupancy)):
        if not isinstance(actual, dict) or actual.get("resource_index") != index:
            raise ValueError("child row occupancy resource index is invalid")
        active, queued = actual.get("active"), actual.get("queued")
        if not _is_int(active) or not _is_int(queued) or (active, queued) != expected:
            raise ValueError(f"child row occupancy mismatch for resource {index}")
    if str(case["scenario"]) == "interruptions":
        n = int(case["n"])
        if row["preemptions"] != n or row["resumptions"] != n or row["completions"] != n:
            raise ValueError("child row interruption lifecycle counts are inconsistent")
    elif row["preemptions"] != 0 or row["resumptions"] != 0:
        raise ValueError("non-interruption scenario reported preemption transitions")
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


def provenance_snapshot() -> dict[str, object]:
    head = subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True, capture_output=True, check=False)
    status = subprocess.run(["git", "status", "--porcelain=v1"], cwd=ROOT, text=True, capture_output=True, check=False)
    return {
        "git_head": head.stdout.strip() if head.returncode == 0 else None,
        "git_status_porcelain": status.stdout if status.returncode == 0 else None,
        "git_readback_returncodes": {"head": head.returncode, "status": status.returncode},
        "source_hashes": {name: sha256_file(ROOT / name) for name in SOURCE_PATHS},
    }


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
    parser.add_argument("--binary", required=True, type=Path)
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
    binary = args.binary if args.binary.is_absolute() else ROOT / args.binary
    binary = binary.resolve()
    if not binary.is_file() or not os.access(binary, os.X_OK):
        print(f"benchmark executable missing or not executable: {binary}", file=sys.stderr)
        return 2
    try:
        cases = _case_matrix_from_args(args)
    except ValueError as error:
        print(str(error), file=sys.stderr)
        return 2
    run_id = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%S%fZ")
    base_output = args.output_dir if args.output_dir.is_absolute() else ROOT / args.output_dir
    run_dir = base_output / run_id
    run_dir.mkdir(parents=True, exist_ok=False)
    provenance_start = provenance_snapshot()
    sources = provenance_start["source_hashes"]
    input_manifest = {"matrix": "q5.2-runtime-v1", "contract_sha256": CONTRACT_SHA256,
                      "cases": cases, "seed": args.seed, "repeats": args.repeats,
                      "timeout_seconds": args.timeout_seconds, "source_hashes": sources,
                      "binary_sha256": sha256_file(binary)}
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
    provenance_end = provenance_snapshot()
    head_drift = provenance_start["git_head"] != provenance_end["git_head"]
    source_drift = provenance_start["source_hashes"] != provenance_end["source_hashes"]
    if head_drift or source_drift or provenance_start["git_readback_returncodes"] != {"head": 0, "status": 0} or provenance_end["git_readback_returncodes"] != {"head": 0, "status": 0}:
        any_failure = True
    overall = "partial_unresolved" if any_timeout and any_failure else (
        "unresolved" if any_timeout else ("failed" if any_failure else "complete")
    )
    toolchain = read_toolchain()
    receipt = {
        "schema": 1,
        "status": overall,
        "run_id": run_id,
        "started_utc": run_id,
        "collector_argv": ["python3", *sys.argv] if argv is None else ["python3", "benches/queue/run_v1.py", *argv],
        "cwd": str(ROOT),
        "provenance_start": provenance_start,
        "provenance_end": provenance_end,
        "provenance_drift": {"head": head_drift, "sources": source_drift},
        "toolchain": toolchain,
        "required_release_build_argv": RELEASE_BUILD_ARGV,
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
