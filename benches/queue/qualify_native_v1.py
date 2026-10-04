#!/usr/bin/env python3
"""Compare accepted F and the exact current head with isolated native Criterion runs."""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import re
import subprocess
import sys
import time
from datetime import datetime, timezone
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
ACCEPTED_F = "f18aba1f1930aec228e830c9e58c67e4081ac4bb"
RUST_VERSION = "1.99.0"
ARTIFACT_REL = Path(".artifacts/ci/q52-native")
BASELINE_REL = Path(".artifacts/ci/q52-native-baseline")
BUILD_TIMEOUT_SECONDS = 900
BENCH_TIMEOUT_SECONDS = 600
BENCHES = ("scheduler", "state", "hybrid")
SIDES = ("F", "current")
CANONICAL = {
    "schedule_1m_events": ("scheduler", "schedule_1m_events", "schedule"),
    "pop_1m_events": ("scheduler", "pop_1m_events", "pop"),
    "schedule_cancel_1m_mixed": ("scheduler", "schedule_cancel_1m_mixed", "schedule_cancel_pop"),
    "create_1m_entities": ("state", "create_1m_entities", "spawn"),
    "component_insert_1m": ("state", "component_insert_1m", "insert"),
    "hybrid_des_abm_smoke_100k": ("hybrid", "hybrid_des_abm_smoke_100k", "schedule_and_pop"),
}
LOCKSTEP_PATHS = (
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    "crates/kairo-ecs-bench/Cargo.toml",
    "crates/kairo-ecs-bench/src/lib.rs",
    "crates/kairo-ecs-bench/benches/scheduler.rs",
    "crates/kairo-ecs-bench/benches/state.rs",
    "crates/kairo-ecs-bench/benches/hybrid.rs",
    "benches/benchmark-smoke.json",
    "conformance/fixtures/manifest.json",
    "conductor/performance-thresholds.md",
    "benches/regression/compare.py",
)
DEPENDENCY_PREFIXES = (
    "crates/kairo-ecs-core/",
    "crates/kairo-ecs-state/",
    "crates/kairo-ecs-types/",
    "crates/kairo-ecs-rng/",
    "crates/kairo-ecs-bench/",
)
RUNNING_RE = re.compile(r"^\s*Running\s+.+?\s+\(([^)]+)\)\s*$", re.MULTILINE)


class QualificationError(RuntimeError):
    pass


def utc_now() -> str:
    return datetime.now(timezone.utc).isoformat()


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    temporary.replace(path)


def run_command(
    argv: list[str], cwd: Path, log_path: Path, timeout_seconds: int,
    env_overrides: dict[str, str] | None = None,
) -> dict[str, Any]:
    started = utc_now()
    start_ns = time.monotonic_ns()
    env = os.environ.copy()
    env.update(env_overrides or {})
    record: dict[str, Any] = {
        "argv": argv,
        "cwd": str(cwd),
        "started_utc": started,
        "timeout_seconds": timeout_seconds,
        "env_overrides": env_overrides or {},
        "log": str(log_path),
    }
    try:
        completed = subprocess.run(
            argv, cwd=cwd, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
            timeout=timeout_seconds, check=False,
        )
        output = completed.stdout or b""
        record["exit_code"] = completed.returncode
    except subprocess.TimeoutExpired as error:
        output = error.stdout or b""
        if isinstance(output, str):
            output = output.encode()
        record["exit_code"] = None
        record["timed_out"] = True
        record["error"] = f"command exceeded {timeout_seconds}s timeout"
    except OSError as error:
        output = f"launch error: {type(error).__name__}: {error}\n".encode()
        record["exit_code"] = None
        record["launch_error"] = f"{type(error).__name__}: {error}"
    log_path.parent.mkdir(parents=True, exist_ok=True)
    log_path.write_bytes(output)
    record["finished_utc"] = utc_now()
    record["elapsed_ns"] = time.monotonic_ns() - start_ns
    record["log_sha256"] = sha256_bytes(output)
    record["output"] = output.decode("utf-8", errors="replace")
    return record


def git(root: Path, *args: str) -> str:
    result = subprocess.run(["git", "-C", str(root), *args], stdout=subprocess.PIPE,
                            stderr=subprocess.PIPE, check=False, text=True)
    if result.returncode:
        raise QualificationError(
            f"git {' '.join(args)} failed in {root}: {result.stderr.strip()}"
        )
    return result.stdout.strip()


def append_artifact_exclude(root: Path) -> None:
    exclude = Path(git(root, "rev-parse", "--git-path", "info/exclude"))
    if not exclude.is_absolute():
        exclude = root / exclude
    exclude.parent.mkdir(parents=True, exist_ok=True)
    current = exclude.read_text(encoding="utf-8") if exclude.exists() else ""
    line = "/.artifacts/ci/"
    if line not in current.splitlines():
        with exclude.open("a", encoding="utf-8") as handle:
            if current and not current.endswith("\n"):
                handle.write("\n")
            handle.write(line + "\n")


def status_porcelain(root: Path) -> str:
    return git(root, "status", "--porcelain", "--untracked-files=all")


def tracked_inputs(root: Path) -> dict[str, str | None]:
    names = git(root, "ls-files", "-z").split("\0")
    selected = set(LOCKSTEP_PATHS)
    selected.update(name for name in names if any(name.startswith(prefix) for prefix in DEPENDENCY_PREFIXES))
    result: dict[str, str | None] = {}
    for name in sorted(selected):
        path = root / name
        result[name] = sha256_file(path) if path.is_file() else None
    return result


def require_lockstep(base: dict[str, str | None], current: dict[str, str | None]) -> None:
    mismatches = [name for name in LOCKSTEP_PATHS if base.get(name) is None or current.get(name) is None
                  or base.get(name) != current.get(name)]
    if mismatches:
        raise QualificationError("canonical benchmark/build/threshold inputs differ: " + ", ".join(mismatches))


def require_unchanged_hashes(before: dict[str, Any], after: dict[str, Any], label: str) -> None:
    if before != after:
        changed = sorted(name for name in set(before) | set(after) if before.get(name) != after.get(name))
        raise QualificationError(f"{label} drifted during qualification: " + ", ".join(changed))


def require_head(actual: str, expected: str, label: str) -> None:
    if actual != expected:
        raise QualificationError(f"{label} HEAD mismatch: expected {expected}, found {actual}")


def parse_mean(path: Path) -> float:
    if not path.is_file():
        raise QualificationError(f"missing Criterion estimate: {path}")
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise QualificationError(f"malformed Criterion JSON {path}: {error}") from error
    mean = payload.get("mean") if isinstance(payload, dict) else None
    value = mean.get("point_estimate") if isinstance(mean, dict) else None
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise QualificationError(f"mean.point_estimate must be a numeric nanosecond value: {path}")
    try:
        numeric = float(value)
    except OverflowError as error:
        raise QualificationError(f"mean.point_estimate is outside finite numeric range: {path}") from error
    if not math.isfinite(numeric) or numeric <= 0:
        raise QualificationError(f"mean.point_estimate must be finite and positive: {path}")
    return numeric


def normalize_results(criterion_root: Path) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    records: list[dict[str, Any]] = []
    hashes: list[dict[str, str]] = []
    for benchmark_id, (bench, group, function) in CANONICAL.items():
        estimate = criterion_root / bench / group / function / "new" / "estimates.json"
        mean_ns = parse_mean(estimate)
        records.append({"id": benchmark_id, "mean_seconds": mean_ns / 1_000_000_000})
        hashes.append({"id": benchmark_id, "path": str(estimate), "sha256": sha256_file(estimate)})
    payload = {"benchmarks": records}
    validate_result_records(payload)
    return payload, hashes


def validate_result_records(payload: Any) -> dict[str, float]:
    if not isinstance(payload, dict) or not isinstance(payload.get("benchmarks"), list):
        raise QualificationError("normalized result must contain a benchmark list")
    results: dict[str, float] = {}
    for item in payload["benchmarks"]:
        if not isinstance(item, dict) or not isinstance(item.get("id"), str):
            raise QualificationError("normalized benchmark row has no string ID")
        benchmark_id = item["id"]
        if benchmark_id not in CANONICAL:
            raise QualificationError(f"unknown canonical benchmark ID: {benchmark_id}")
        if benchmark_id in results:
            raise QualificationError(f"duplicate canonical benchmark ID: {benchmark_id}")
        mean = item.get("mean_seconds")
        if isinstance(mean, bool) or not isinstance(mean, (int, float)):
            raise QualificationError(f"invalid positive finite mean_seconds for {benchmark_id}")
        try:
            numeric = float(mean)
        except OverflowError as error:
            raise QualificationError(f"invalid positive finite mean_seconds for {benchmark_id}") from error
        if not math.isfinite(numeric) or numeric <= 0:
            raise QualificationError(f"invalid positive finite mean_seconds for {benchmark_id}")
        results[benchmark_id] = numeric
    missing = sorted(set(CANONICAL) - set(results))
    if missing:
        raise QualificationError("missing canonical benchmark IDs: " + ", ".join(missing))
    return results


def validate_command_evidence(
    record: dict[str, Any], artifact_root: Path, allow_nonzero_exit: bool = False,
) -> None:
    log = Path(record.get("log", ""))
    if not log.is_absolute():
        log = artifact_root / log
    if not log.is_file():
        raise QualificationError(f"missing command log: {log}")
    if record.get("log_sha256") != sha256_file(log):
        raise QualificationError(f"command log hash mismatch: {log}")
    if (not allow_nonzero_exit and record.get("exit_code") != 0) or record.get("timed_out") or record.get("launch_error"):
        raise QualificationError(f"command did not complete successfully: {record.get('name', '<unnamed>')}")


def executable_from_running(log_text: str, target_dir: Path, cwd: Path) -> Path:
    matches = RUNNING_RE.findall(log_text)
    valid: list[Path] = []
    expected_deps = (target_dir / "release" / "deps").resolve()
    for spelling in matches:
        raw = Path(spelling.strip().strip("'\""))
        candidates = [raw] if raw.is_absolute() else [cwd / raw, ROOT / raw]
        for candidate in candidates:
            resolved = candidate.resolve()
            if resolved.parent == expected_deps and resolved.is_file() and resolved.suffix != ".d":
                if resolved not in valid:
                    valid.append(resolved)
    if len(valid) != 1:
        raise QualificationError(f"expected one actual Cargo Running executable under {expected_deps}; found {len(valid)}")
    return valid[0]


def parse_prebuild_executables(output: str, target_dir: Path, cwd: Path) -> dict[str, dict[str, str]]:
    found: dict[str, dict[str, str]] = {}
    expected_deps = (target_dir / "release" / "deps").resolve()
    for line in output.splitlines():
        if "Executable benches/" not in line:
            continue
        match = re.search(r"Executable benches/([^ ]+) \(([^)]+)\)", line)
        if not match:
            continue
        bench = Path(match.group(1)).stem
        raw = Path(match.group(2).strip().strip("'\""))
        candidates = [raw] if raw.is_absolute() else [cwd / raw, ROOT / raw]
        resolved = next((candidate.resolve() for candidate in candidates
                         if candidate.resolve().parent == expected_deps and candidate.resolve().is_file()), None)
        if bench in BENCHES and resolved is not None:
            if bench in found:
                raise QualificationError(f"prebuild reported duplicate executable for {bench}")
            found[bench] = {"path": str(resolved), "sha256": sha256_file(resolved)}
    if set(found) != set(BENCHES):
        raise QualificationError(f"prebuild did not identify all three benchmark executables: {sorted(found)}")
    return found


def validate_python_and_host() -> dict[str, Any]:
    if sys.version_info[:3] != (3, 14, 8):
        raise QualificationError(f"expected Python 3.14.8, found {platform.python_version()}")
    if not sys.platform.startswith("linux") or platform.machine() != "x86_64":
        raise QualificationError(f"expected Linux x86_64 runner, found {platform.platform()}")
    os_release = Path("/etc/os-release").read_text(encoding="utf-8")
    if 'VERSION_ID="24.04"' not in os_release and "VERSION_ID=24.04" not in os_release:
        raise QualificationError("expected Ubuntu 24.04 runner")
    return {"python": platform.python_version(), "host": platform.platform(),
            "machine": platform.machine(), "system": platform.system()}


class Runner:
    def __init__(self, artifact_root: Path, receipt: dict[str, Any]):
        self.artifact_root = artifact_root
        self.receipt = receipt

    def command(self, name: str, argv: list[str], cwd: Path, timeout: int,
                env: dict[str, str] | None = None) -> dict[str, Any]:
        log = self.artifact_root / "logs" / f"{name}.log"
        result = run_command(argv, cwd, log, timeout, env)
        result["name"] = name
        result["log"] = str(log)
        self.receipt["commands"].append({k: v for k, v in result.items() if k != "output"})
        write_json(self.artifact_root / "result.json", self.receipt)
        validate_command_evidence(self.receipt["commands"][-1], self.artifact_root)
        if result.get("exit_code") != 0:
            detail = result.get("error") or result.get("launch_error") or f"exit {result.get('exit_code')}"
            raise QualificationError(f"{name} failed: {detail}")
        return result


def run_canonical_comparator(artifact_root: Path, receipt: dict[str, Any]) -> None:
    compare = run_command(
        [sys.executable, str(ROOT / "benches/regression/compare.py"),
         "--base", str(artifact_root / "baseline.json"), "--current", str(artifact_root / "current.json"),
         "--report", str(artifact_root / "comparison.json")],
        ROOT, artifact_root / "logs" / "canonical-comparator.log", 120,
    )
    compare["name"] = "canonical-comparator"
    receipt["commands"].append({k: v for k, v in compare.items() if k != "output"})
    write_json(artifact_root / "result.json", receipt)
    if (artifact_root / "comparison.json").is_file():
        receipt["comparison"] = json.loads((artifact_root / "comparison.json").read_text(encoding="utf-8"))
    validate_command_evidence(receipt["commands"][-1], artifact_root, allow_nonzero_exit=True)
    if compare["exit_code"] != 0:
        if receipt.get("comparison", {}).get("status") == "fail":
            receipt["status"] = "threshold_failure"
            raise QualificationError("canonical comparator failed; thresholds remain blocking")
        raise QualificationError("canonical comparator returned nonzero without a valid failure report")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--expected-head", required=True)
    parser.add_argument("--run-sha", required=True)
    args = parser.parse_args()
    artifact_root = (ROOT / ARTIFACT_REL).resolve()
    baseline_root = (ROOT / BASELINE_REL).resolve()
    receipt: dict[str, Any] = {
        "schema": 1,
        "status": "running",
        "accepted_baseline_sha": ACCEPTED_F,
        "expected_head": args.expected_head,
        "workflow_run_sha": args.run_sha,
        "current_root": str(ROOT),
        "baseline_root": str(baseline_root),
        "artifact_root": str(artifact_root),
        "started_utc": utc_now(),
        "commands": [],
    }
    artifact_root.mkdir(parents=True, exist_ok=True)
    write_json(artifact_root / "result.json", receipt)
    runner = Runner(artifact_root, receipt)
    try:
        append_artifact_exclude(ROOT)
        current_head = git(ROOT, "rev-parse", "HEAD")
        receipt["actual_producer_head"] = current_head
        receipt["current_status_before"] = status_porcelain(ROOT)
        require_head(current_head, args.expected_head, "current producer")
        if receipt["current_status_before"]:
            raise QualificationError("current tracked/untracked source tree is not clean before measurement")
        if args.run_sha and args.run_sha != current_head:
            receipt["workflow_run_head_differs_from_producer"] = True
        host = validate_python_and_host()
        receipt["host"] = host
        runner.command("install-rust", ["rustup", "toolchain", "install", RUST_VERSION, "--profile", "minimal"], ROOT, 600)
        toolchain = runner.command("toolchain", ["rustup", "run", RUST_VERSION, "rustc", "-vV"], ROOT, 60)
        tool_text = toolchain["output"]
        if f"release: {RUST_VERSION}" not in tool_text or "host: x86_64-unknown-linux-gnu" not in tool_text:
            raise QualificationError("Rust toolchain release/host does not match pinned CI target")
        receipt["toolchain"] = tool_text

        if baseline_root.exists():
            raise QualificationError(f"baseline worktree path already exists: {baseline_root}")
        baseline_root.parent.mkdir(parents=True, exist_ok=True)
        runner.command("baseline-worktree", ["git", "worktree", "add", "--force", "--detach", str(baseline_root), ACCEPTED_F], ROOT, 120)
        receipt["baseline_head_before"] = git(baseline_root, "rev-parse", "HEAD")
        require_head(receipt["baseline_head_before"], ACCEPTED_F, "accepted baseline")
        append_artifact_exclude(baseline_root)
        baseline_status = status_porcelain(baseline_root)
        if baseline_status:
            raise QualificationError("baseline worktree is not clean")
        receipt["baseline_status_before"] = baseline_status

        receipt["source_hashes_before"] = {
            "F": tracked_inputs(baseline_root),
            "current": tracked_inputs(ROOT),
        }
        receipt["implementation_hashes_before"] = {
            "qualification_script_sha256": sha256_file(Path(__file__).resolve()),
            "qualification_test_sha256": sha256_file(ROOT / "benches/queue/test_qualify_native_v1.py"),
            "workflow_sha256": sha256_file(ROOT / ".github/workflows/careops-native-owner.yml"),
        }
        require_lockstep(receipt["source_hashes_before"]["F"], receipt["source_hashes_before"]["current"])
        write_json(artifact_root / "source-hashes-before.json", receipt["source_hashes_before"])

        target_dirs = {side: artifact_root / "target" / side for side in SIDES}
        roots = {"F": baseline_root, "current": ROOT}
        for side in SIDES:
            target_dirs[side].mkdir(parents=True, exist_ok=True)
            build = runner.command(
                f"build-{side}",
                ["rustup", "run", RUST_VERSION, "cargo", "bench", "--locked", "-p", "kairo-ecs-bench",
                 "--no-run", "--target-dir", str(target_dirs[side])],
                roots[side], BUILD_TIMEOUT_SECONDS,
            )
            receipt.setdefault("prebuilt_executables", {})[side] = parse_prebuild_executables(
                build["output"], target_dirs[side], roots[side]
            )
            write_json(artifact_root / "result.json", receipt)

        command_order: list[tuple[str, str]] = []
        for bench in BENCHES:
            for side in SIDES:
                command_order.append((side, bench))
        receipt["timed_command_order"] = [f"{side}:{bench}" for side, bench in command_order]
        receipt["criterion_settings"] = "Criterion source defaults; no timing-setting overrides; --noplot and per-benchmark CRITERION_HOME only"
        receipt["runs"] = {side: {} for side in SIDES}
        for side, bench in command_order:
            criterion_home = artifact_root / "criterion" / side / bench
            criterion_home.mkdir(parents=True, exist_ok=False)
            record = runner.command(
                f"{side}-{bench}",
                ["rustup", "run", RUST_VERSION, "cargo", "bench", "--locked", "-p", "kairo-ecs-bench",
                 "--bench", bench, "--target-dir", str(target_dirs[side]), "--", "--noplot"],
                roots[side], BENCH_TIMEOUT_SECONDS, {"CRITERION_HOME": str(criterion_home)},
            )
            if "Compiling " in record["output"]:
                raise QualificationError(f"unexpected compilation during timed run {side}:{bench}")
            executable = executable_from_running(record["output"], target_dirs[side], roots[side])
            digest = sha256_file(executable)
            prebuilt = receipt["prebuilt_executables"][side][bench]
            if str(executable) != prebuilt["path"] or digest != prebuilt["sha256"]:
                raise QualificationError(f"timed executable differs from prebuilt executable for {side}:{bench}")
            receipt["runs"][side][bench] = {
                "command": f"{side}-{bench}", "executable": str(executable), "executable_sha256": digest,
                "criterion_home": str(criterion_home),
            }
            write_json(artifact_root / "result.json", receipt)

        normalized: dict[str, dict[str, Any]] = {}
        for side in SIDES:
            criterion_root = artifact_root / "criterion" / side
            normalized[side], hashes = normalize_results(criterion_root)
            receipt["runs"][side]["estimates"] = hashes
            dest = artifact_root / ("baseline.json" if side == "F" else "current.json")
            write_json(dest, normalized[side])
        write_json(artifact_root / "result.json", receipt)

        run_canonical_comparator(artifact_root, receipt)

        final_hashes = {"F": tracked_inputs(baseline_root), "current": tracked_inputs(ROOT)}
        require_lockstep(final_hashes["F"], final_hashes["current"])
        receipt["source_hashes_after"] = final_hashes
        require_unchanged_hashes(receipt["source_hashes_before"]["F"], final_hashes["F"], "F inputs")
        require_unchanged_hashes(receipt["source_hashes_before"]["current"], final_hashes["current"], "current inputs")
        implementation_after = {
            "qualification_script_sha256": sha256_file(Path(__file__).resolve()),
            "qualification_test_sha256": sha256_file(ROOT / "benches/queue/test_qualify_native_v1.py"),
            "workflow_sha256": sha256_file(ROOT / ".github/workflows/careops-native-owner.yml"),
        }
        receipt["implementation_hashes_after"] = implementation_after
        require_unchanged_hashes(receipt["implementation_hashes_before"], implementation_after, "qualification implementation")
        receipt["current_status_after"] = status_porcelain(ROOT)
        receipt["baseline_status_after"] = status_porcelain(baseline_root)
        if receipt["current_status_after"] or receipt["baseline_status_after"]:
            raise QualificationError("tracked/untracked source tree changed during qualification")
        receipt["status"] = "qualified_pass"
        returncode = 0
    except Exception as error:
        receipt["status"] = receipt.get("status") if receipt.get("status") == "threshold_failure" else "failed"
        receipt["error"] = f"{type(error).__name__}: {error}"
        returncode = 1
    finally:
        try:
            receipt["actual_producer_head_after"] = git(ROOT, "rev-parse", "HEAD")
            receipt["current_status_after"] = status_porcelain(ROOT)
            if baseline_root.exists():
                receipt["baseline_head_after"] = git(baseline_root, "rev-parse", "HEAD")
                receipt["baseline_status_after"] = status_porcelain(baseline_root)
            if "source_hashes_before" in receipt and baseline_root.exists():
                final_inputs = {"F": tracked_inputs(baseline_root), "current": tracked_inputs(ROOT)}
                receipt["final_source_hashes_after"] = final_inputs
                implementation_after = {
                    "qualification_script_sha256": sha256_file(Path(__file__).resolve()),
                    "qualification_test_sha256": sha256_file(ROOT / "benches/queue/test_qualify_native_v1.py"),
                    "workflow_sha256": sha256_file(ROOT / ".github/workflows/careops-native-owner.yml"),
                }
                receipt["implementation_hashes_after_final"] = implementation_after
                current_changed = (
                    receipt["actual_producer_head_after"] != receipt.get("actual_producer_head")
                    or receipt["current_status_after"] != ""
                )
                baseline_changed = (
                    receipt.get("baseline_head_after") != receipt.get("baseline_head_before")
                    or receipt.get("baseline_status_after") != ""
                )
                receipt["source_drift_after"] = (
                    final_inputs != receipt["source_hashes_before"]
                    or implementation_after != receipt.get("implementation_hashes_before")
                    or current_changed or baseline_changed
                )
                if receipt["source_drift_after"] and receipt.get("status") == "qualified_pass":
                    receipt["status"] = "failed"
                    receipt["error"] = "source or checkout state drifted before final readback"
                    returncode = 1
        except Exception as error:
            receipt["final_readback_error"] = f"{type(error).__name__}: {error}"
            if receipt.get("status") == "qualified_pass":
                receipt["status"] = "failed"
                receipt["error"] = "final provenance readback failed"
                returncode = 1
        receipt["finished_utc"] = utc_now()
        receipt["qualification_script_sha256_after"] = sha256_file(Path(__file__).resolve())
        write_json(artifact_root / "result.json", receipt)
    return returncode


if __name__ == "__main__":
    raise SystemExit(main())
