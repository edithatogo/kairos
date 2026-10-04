#!/usr/bin/env python3
"""Run and validate commit-bound local Track 48 benchmark evidence."""

from __future__ import annotations

import hashlib
import json
import os
import platform
import re
import shutil
import subprocess
import sys
from pathlib import Path
from typing import Any, Callable

try:
    import tomllib
except ModuleNotFoundError:  # pragma: no cover - older Python fails closed when Cargo config exists
    tomllib = None

ROOT = Path(__file__).resolve().parents[2]
ARTIFACT_DIR = ROOT / "artifacts" / "track48-benchmark"
CARGO_ARGS = [
    "bench",
    "-p",
    "kairo-ecs-pdes",
    "--bench",
    "time_warp",
    "--features",
    "pdes,time-warp",
]
TOOLCHAIN = "1.98.1"
WRAPPER_ENVIRONMENT_KEYS = (
    "RUSTC_WRAPPER",
    "RUSTC_WORKSPACE_WRAPPER",
    "CARGO_BUILD_RUSTC_WRAPPER",
    "CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER",
)
TOOLCHAIN_SELECTION_ENVIRONMENT_KEYS = ("RUSTC", "CARGO_BUILD_RUSTC", "RUSTUP_TOOLCHAIN")
EXPECTED_SEED = 48_2027
EXPECTED_ROOTS_PER_LP = 8
EXPECTED_MAX_HOPS = 8
EXPECTED_LP_COUNTS = (4, 8)
EXPECTED_EMISSION = {"sparse": 10, "dense": 80}
EXPECTED_PROFILES = {
    (profile, lp_count)
    for profile in EXPECTED_EMISSION
    for lp_count in EXPECTED_LP_COUNTS
}
SOURCE_FILES = (
    "Cargo.toml",
    "Cargo.lock",
    ".gitignore",
    "rust-toolchain.toml",
    "rust-toolchain",
    "crates/kairo-ecs-pdes/Cargo.toml",
    "crates/kairo-ecs-pdes/build.rs",
    "crates/kairo-ecs-pdes/benches/time_warp.rs",
    "benches/pdes/collect_time_warp_evidence.py",
    "benches/pdes/test_collect_time_warp_evidence.py",
    "crates/kairo-ecs-types/Cargo.toml",
    "crates/kairo-ecs-types/build.rs",
    "crates/kairo-ecs-core/Cargo.toml",
    "crates/kairo-ecs-core/build.rs",
)
SOURCE_TREES = (
    "crates/kairo-ecs-pdes/src",
    "crates/kairo-ecs-types/src",
    "crates/kairo-ecs-core/src",
)


def canonical_hash(value: Any) -> str:
    data = json.dumps(value, sort_keys=True, separators=(",", ":")).encode("utf-8")
    return hashlib.sha256(data).hexdigest()


def is_int(value: Any) -> bool:
    return isinstance(value, int) and not isinstance(value, bool)


def require_nonnegative_int(value: Any, label: str) -> int:
    if not is_int(value) or value < 0:
        raise ValueError(f"{label} must be a nonnegative integer")
    return value


def require_int_equal(value: Any, expected: int, label: str) -> None:
    if not is_int(value) or value != expected:
        raise ValueError(f"{label} must be {expected}")


def splitmix64_once(initial: int) -> int:
    mask = (1 << 64) - 1
    state = (initial + 0x9E37_79B9_7F4A_7C15) & mask
    value = state
    value = ((value ^ (value >> 30)) * 0xBF58_476D_1CE4_E5B9) & mask
    value = ((value ^ (value >> 27)) * 0x94D0_49BB_1331_11EB) & mask
    return (value ^ (value >> 31)) & mask


def expected_input_events(lp_count: int) -> list[dict[str, int]]:
    events = []
    for lp_id in range(lp_count):
        for root_index in range(EXPECTED_ROOTS_PER_LP):
            root = lp_id * EXPECTED_ROOTS_PER_LP + root_index
            events.append(
                {
                    "source": lp_id,
                    "destination": lp_id,
                    "tick": lp_id * 64 + root_index * 4,
                    "root": root,
                    "remaining_hops": EXPECTED_MAX_HOPS,
                    "value": splitmix64_once(EXPECTED_SEED ^ root),
                }
            )
    return events


def validate_output(document: Any) -> dict[str, str]:
    if not isinstance(document, dict):
        raise ValueError("benchmark document must be an object")
    if document.get("schema_version") != "kairoecs.pdes.time_warp_benchmark.v1":
        raise ValueError("unexpected benchmark schema")
    require_int_equal(document.get("seed"), EXPECTED_SEED, "benchmark seed")
    require_int_equal(document.get("warmup_runs"), 1, "warmup runs")
    require_int_equal(document.get("repetitions"), 5, "repetitions")
    if document.get("timed_boundary") != "runtime_run_call":
        raise ValueError("timed boundary must isolate the runtime run call")
    excluded = document.get("excluded_costs")
    expected_excluded = {
        "process construction",
        "partition and topology construction",
        "initial event scheduling",
        "final state/report extraction",
        "parity and logical ID validation",
        "fossil collection",
    }
    if (
        not isinstance(excluded, list)
        or any(not isinstance(item, str) for item in excluded)
        or len(excluded) != len(expected_excluded)
        or set(excluded) != expected_excluded
    ):
        raise ValueError("benchmark must declare all setup and post-run costs excluded from timing")

    profiles = document.get("profiles")
    if not isinstance(profiles, list):
        raise ValueError("profiles must be an array")

    observed: set[tuple[str, int]] = set()
    hashes: dict[str, str] = {}
    by_lp: dict[int, str] = {}
    for row in profiles:
        if not isinstance(row, dict):
            raise ValueError("each profile must be an object")
        profile = row.get("profile")
        lp_count = row.get("lp_count")
        if not isinstance(profile, str) or profile not in EXPECTED_EMISSION:
            raise ValueError("profile name must be sparse or dense")
        if not is_int(lp_count) or lp_count not in EXPECTED_LP_COUNTS:
            raise ValueError("profile LP count must be 4 or 8")
        key = (profile, lp_count)
        if key in observed:
            raise ValueError(f"duplicate profile: {key!r}")
        observed.add(key)
        require_int_equal(row.get("emit_percent"), EXPECTED_EMISSION[profile], f"emission density for {key!r}")
        require_int_equal(row.get("roots_per_lp"), EXPECTED_ROOTS_PER_LP, f"roots_per_lp for {key!r}")
        require_int_equal(row.get("max_hops"), EXPECTED_MAX_HOPS, f"max_hops for {key!r}")
        require_int_equal(row.get("seed"), EXPECTED_SEED, f"row seed for {key!r}")
        horizon = (lp_count - 1) * 64 + (EXPECTED_ROOTS_PER_LP - 1) * 4 + EXPECTED_MAX_HOPS + 2
        require_int_equal(row.get("horizon"), horizon, f"horizon for {key!r}")
        if row.get("parity") is not True:
            raise ValueError(f"runtime parity was not established for {key!r}")

        events = row.get("input_events")
        expected_events = expected_input_events(lp_count)
        events_well_typed = isinstance(events, list) and all(
            isinstance(event, dict)
            and set(event) == {"source", "destination", "tick", "root", "remaining_hops", "value"}
            and all(is_int(value) for value in event.values())
            for event in events
        )
        if not events_well_typed or events != expected_events:
            raise ValueError(f"initial event count, type, route, root order, or payload is invalid for {key!r}")
        digest = canonical_hash(events)
        hashes[f"{profile}-lp{lp_count}"] = digest
        prior = by_lp.setdefault(lp_count, digest)
        if prior != digest:
            raise ValueError(f"sparse/dense pair does not share an identical input graph for LP count {lp_count}")

        expected_committed = row.get("expected_committed_events")
        if not is_int(expected_committed) or expected_committed < len(expected_events):
            raise ValueError(f"invalid committed event count for {key!r}")
        conservative = row.get("conservative_counters")
        optimistic = row.get("optimistic_counters")
        if not isinstance(conservative, dict):
            raise ValueError(f"conservative counters must be an object for {key!r}")
        if not isinstance(optimistic, dict):
            raise ValueError(f"optimistic counters must be an object for {key!r}")
        conservative_fields = (
            "processed_events",
            "remote_events",
            "emitted_events",
            "null_messages",
            "rounds",
            "spawned_worker_cohort_max",
        )
        optimistic_fields = (
            "executions",
            "first_attempt_executions",
            "extra_executions",
            "replay_executions",
            "rollback_attempts",
            "rolled_back_events",
            "max_rollback_depth",
            "canceled_sends",
            "fossil_collected_events",
            "checkpoints_before_fossil",
            "fossil_collected_checkpoints",
            "committed_logical_ids",
        )
        for name in conservative_fields:
            require_nonnegative_int(conservative.get(name), f"conservative {name} for {key!r}")
        for name in optimistic_fields:
            require_nonnegative_int(optimistic.get(name), f"optimistic {name} for {key!r}")

        if conservative["processed_events"] != expected_committed:
            raise ValueError(f"conservative event count does not match committed parity count for {key!r}")
        if optimistic["fossil_collected_events"] != expected_committed:
            raise ValueError(f"fossil event count does not match committed parity count for {key!r}")
        if optimistic["committed_logical_ids"] != expected_committed:
            raise ValueError(f"committed logical ID count does not match parity count for {key!r}")
        executions = optimistic["executions"]
        replay = optimistic["replay_executions"]
        committed = optimistic["committed_logical_ids"]
        if executions < committed:
            raise ValueError(f"total executions are fewer than committed events for {key!r}")
        if optimistic["first_attempt_executions"] != executions - replay:
            raise ValueError(f"first-attempt execution count is inconsistent for {key!r}")
        if executions != optimistic["first_attempt_executions"] + replay:
            raise ValueError(f"execution accounting is inconsistent for {key!r}")
        if optimistic["extra_executions"] != executions - committed:
            raise ValueError(f"extra execution count is inconsistent for {key!r}")
        for name in ("rollback_attempts", "rolled_back_events", "replay_executions"):
            if optimistic[name] == 0:
                raise ValueError(f"profile did not exercise {name}: {key!r}")
        if optimistic["max_rollback_depth"] == 0 or optimistic["fossil_collected_checkpoints"] == 0:
            raise ValueError(f"profile did not exercise rollback/checkpoint collection: {key!r}")

        for name in ("conservative_ns", "optimistic_ns"):
            samples = row.get(name)
            if (
                not isinstance(samples, list)
                or len(samples) != 5
                or any(not is_int(sample) or sample <= 0 for sample in samples)
            ):
                raise ValueError(f"{name} must contain five positive raw durations for {key!r}")

    if observed != EXPECTED_PROFILES:
        raise ValueError(f"profile matrix incomplete: {observed!r}")
    return hashes


def command_output(
    args: list[str],
    cwd: Path,
    env: dict[str, str] | None = None,
    runner: Callable[..., Any] = subprocess.run,
) -> str:
    result = runner(args, cwd=cwd, env=env, text=True, capture_output=True, check=False)
    if result.returncode != 0:
        raise RuntimeError(f"metadata command failed ({result.returncode}): {args!r}: {result.stderr.strip()}")
    return result.stdout.strip()


def cargo_config_paths(root: Path, base_env: dict[str, str]) -> list[Path]:
    paths = []
    current = root.resolve()
    for directory in (current, *current.parents):
        for name in ("config.toml", "config"):
            path = directory / ".cargo" / name
            if path.is_file():
                paths.append(path.resolve())
    cargo_home = Path(base_env.get("CARGO_HOME", Path.home() / ".cargo")).expanduser()
    for name in ("config.toml", "config"):
        path = cargo_home / name
        if path.is_file():
            paths.append(path.resolve())
    return sorted(set(paths))


def cargo_wrapper_config_hashes(root: Path, base_env: dict[str, str]) -> dict[str, str]:
    hashes = {}
    for path in cargo_config_paths(root, base_env):
        text = path.read_text(encoding="utf-8")
        if tomllib is None:
            raise ValueError("Python 3.11+ tomllib is required to inspect Cargo build configuration")
        try:
            config = tomllib.loads(text)
        except tomllib.TOMLDecodeError as error:
            raise ValueError(f"Cargo configuration cannot be parsed safely: {path}") from error

        def nonempty(value: Any) -> bool:
            if isinstance(value, dict):
                value = value.get("value")
            return value not in (None, "", [])

        build = config.get("build", {})
        if not isinstance(build, dict):
            raise ValueError(f"Cargo build configuration is not a table: {path}")
        for key in ("rustc", "rustc-wrapper", "rustc-workspace-wrapper", "rustflags"):
            if nonempty(build.get(key)):
                raise ValueError(f"Cargo compiler wrapper or Rust flags are configured in {path}: build.{key}")
        target = config.get("target", {})
        if not isinstance(target, dict):
            raise ValueError(f"Cargo target configuration is not a table: {path}")
        for target_name, target_config in target.items():
            if isinstance(target_config, dict) and nonempty(target_config.get("rustflags")):
                raise ValueError(f"Cargo target Rust flags are configured in {path}: target.{target_name}")
        configured_env = config.get("env", {})
        if not isinstance(configured_env, dict):
            raise ValueError(f"Cargo environment configuration is not a table: {path}")
        protected_env_keys = (
            *WRAPPER_ENVIRONMENT_KEYS,
            *TOOLCHAIN_SELECTION_ENVIRONMENT_KEYS,
            "RUSTFLAGS",
            "CARGO_ENCODED_RUSTFLAGS",
            "CARGO_BUILD_RUSTFLAGS",
        )
        for key, value in configured_env.items():
            if (key in protected_env_keys or is_rustflag_environment_key(key)) and nonempty(value):
                raise ValueError(f"Cargo wrapper or Rust flags are configured in {path}: env.{key}")
        hashes[str(path)] = hash_file(path)
    return hashes


def is_rustflag_environment_key(key: str) -> bool:
    return key in ("RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "CARGO_BUILD_RUSTFLAGS") or (
        key.startswith("CARGO_TARGET_") and key.endswith("_RUSTFLAGS")
    )


def rustflag_environment_keys(env: dict[str, str]) -> list[str]:
    return sorted(key for key in env if is_rustflag_environment_key(key))


def resolved_toolchain(
    root: Path,
    base_env: dict[str, str] | None = None,
    command_runner: Callable[..., Any] = subprocess.run,
) -> dict[str, Any]:
    base_env = dict(os.environ if base_env is None else base_env)
    rustup = shutil.which("rustup", path=base_env.get("PATH"))
    if rustup is None:
        raise RuntimeError("rustup executable is unavailable")
    rustup = str(Path(rustup).resolve())
    config_hashes = cargo_wrapper_config_hashes(root, base_env)
    inherited_wrappers = [key for key in WRAPPER_ENVIRONMENT_KEYS if base_env.get(key)]
    inherited_toolchain_selectors = [
        key for key in TOOLCHAIN_SELECTION_ENVIRONMENT_KEYS if base_env.get(key)
    ]
    rustflags = rustflag_environment_keys(base_env)
    nonempty_rustflags = [key for key in rustflags if base_env[key]]
    if nonempty_rustflags:
        fingerprints = [
            f"{key}=sha256:{hashlib.sha256(base_env[key].encode()).hexdigest()}"
            for key in nonempty_rustflags
        ]
        raise ValueError(
            "non-empty Rust flags environment overrides are rejected for the fixed benchmark: "
            + ", ".join(fingerprints)
        )
    sysroot = Path(
        command_output(
            [rustup, "run", TOOLCHAIN, "rustc", "--print", "sysroot"],
            root,
            base_env,
            command_runner,
        )
    ).resolve(strict=True)
    rustc = Path(
        command_output(
            [rustup, "which", "rustc", "--toolchain", TOOLCHAIN], root, base_env, command_runner
        )
    ).resolve(strict=True)
    cargo = Path(
        command_output(
            [rustup, "which", "cargo", "--toolchain", TOOLCHAIN], root, base_env, command_runner
        )
    ).resolve(strict=True)
    toolchain_bin = (sysroot / "bin").resolve(strict=True)
    if rustc.parent != toolchain_bin or cargo.parent != toolchain_bin:
        raise ValueError("resolved Rust tools do not belong to the requested toolchain sysroot")

    rustc_version = command_output([str(rustc), "--version", "-v"], root, base_env, command_runner)
    cargo_version = command_output([str(cargo), "--version"], root, base_env, command_runner)
    cargo_verbose = command_output([str(cargo), "-vV"], root, base_env, command_runner)
    if f"release: {TOOLCHAIN}" not in rustc_version.splitlines():
        raise ValueError(f"resolved rustc version is not {TOOLCHAIN}")
    if not cargo_version.startswith(f"cargo {TOOLCHAIN} "):
        raise ValueError(f"resolved cargo version is not {TOOLCHAIN}")
    if f"release: {TOOLCHAIN}" not in cargo_verbose.splitlines():
        raise ValueError(f"resolved cargo provenance is not {TOOLCHAIN}")

    return {
        "toolchain": TOOLCHAIN,
        "source_root": str(root.resolve()),
        "rustup_path": rustup,
        "sysroot": str(sysroot),
        "toolchain_bin": str(toolchain_bin),
        "rustc_path": str(rustc),
        "rustc_version": rustc_version,
        "rustc_sha256": hash_file(rustc),
        "cargo_path": str(cargo),
        "cargo_version": cargo_version,
        "cargo_verbose_version": cargo_verbose,
        "cargo_sha256": hash_file(cargo),
        "cargo_config_sha256": config_hashes,
        "disabled_wrapper_environment": inherited_wrappers,
        "disabled_toolchain_environment": inherited_toolchain_selectors,
        "cleared_empty_rustflags_environment": rustflags,
    }


def prepare_toolchain_environment(
    base_env: dict[str, str], provenance: dict[str, Any]
) -> dict[str, str]:
    env = dict(base_env)
    toolchain_bin = provenance["toolchain_bin"]
    env["PATH"] = toolchain_bin + os.pathsep + env.get("PATH", "")
    env["RUSTC"] = provenance["rustc_path"]
    env["RUSTUP_TOOLCHAIN"] = provenance["toolchain"]
    for key in WRAPPER_ENVIRONMENT_KEYS:
        env.pop(key, None)
    env.pop("CARGO_BUILD_RUSTC", None)
    for key in provenance["cleared_empty_rustflags_environment"]:
        env.pop(key, None)
    return env


def verify_toolchain_environment(env: dict[str, str], provenance: dict[str, Any]) -> None:
    expected_rustc = Path(provenance["rustc_path"]).resolve(strict=True)
    expected_bin = Path(provenance["toolchain_bin"]).resolve(strict=True)
    configured_rustc = env.get("RUSTC")
    if configured_rustc is None or Path(configured_rustc).resolve(strict=True) != expected_rustc:
        raise ValueError("RUSTC does not point to the resolved toolchain compiler")
    path_entries = env.get("PATH", "").split(os.pathsep)
    if not path_entries or Path(path_entries[0]).resolve(strict=True) != expected_bin:
        raise ValueError("toolchain bin directory is not first on PATH")
    discovered_rustc = shutil.which("rustc", path=env.get("PATH"))
    if discovered_rustc is None or Path(discovered_rustc).resolve(strict=True) != expected_rustc:
        raise ValueError("PATH compiler does not match the resolved toolchain compiler")
    if env.get("RUSTUP_TOOLCHAIN") != provenance["toolchain"]:
        raise ValueError("RUSTUP_TOOLCHAIN does not match the resolved toolchain")
    if any(env.get(key) for key in (*WRAPPER_ENVIRONMENT_KEYS, "CARGO_BUILD_RUSTC")):
        raise ValueError("compiler wrapper or selector environment would override the resolved rustc")
    active_rustflags = [key for key in rustflag_environment_keys(env) if env[key]]
    if active_rustflags:
        raise ValueError(f"Rust flags environment changed after resolution: {active_rustflags!r}")
    if hash_file(expected_rustc) != provenance["rustc_sha256"]:
        raise ValueError("resolved rustc binary changed after provenance capture")
    if hash_file(Path(provenance["cargo_path"]).resolve(strict=True)) != provenance["cargo_sha256"]:
        raise ValueError("resolved cargo binary changed after provenance capture")
    current_config_hashes = cargo_wrapper_config_hashes(Path(provenance["source_root"]), env)
    if current_config_hashes != provenance["cargo_config_sha256"]:
        raise ValueError("Cargo compiler wrapper configuration changed after provenance capture")


def source_paths(root: Path) -> list[str]:
    paths = {relative for relative in SOURCE_FILES if (root / relative).is_file()}
    required = {
        "Cargo.toml",
        "Cargo.lock",
        ".gitignore",
        "crates/kairo-ecs-pdes/Cargo.toml",
        "crates/kairo-ecs-pdes/benches/time_warp.rs",
        "benches/pdes/collect_time_warp_evidence.py",
        "benches/pdes/test_collect_time_warp_evidence.py",
        "crates/kairo-ecs-types/Cargo.toml",
        "crates/kairo-ecs-core/Cargo.toml",
    }
    missing = sorted(required - paths)
    if missing:
        raise FileNotFoundError(f"source integrity inputs are missing: {missing!r}")
    for relative in SOURCE_TREES:
        source_dir = root / relative
        if not source_dir.is_dir():
            raise FileNotFoundError(f"compiled source directory is missing: {relative}")
        paths.update(
            path.relative_to(root).as_posix()
            for path in source_dir.rglob("*")
            if path.is_file()
        )
    config_dir = root / ".cargo"
    if config_dir.is_dir():
        paths.update(
            path.relative_to(root).as_posix()
            for path in config_dir.rglob("*")
            if path.is_file()
        )
    return sorted(paths)


def hash_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def source_hashes(root: Path = ROOT) -> dict[str, str]:
    return {relative: hash_file(root / relative) for relative in source_paths(root)}


def git_status(root: Path) -> str:
    result = subprocess.run(
        ["git", "status", "--porcelain=v1", "--untracked-files=all"],
        cwd=root,
        text=True,
        capture_output=True,
        check=False,
    )
    if result.returncode != 0:
        raise RuntimeError(f"git status failed: {result.stderr.strip()}")
    return result.stdout


def source_state(root: Path) -> dict[str, Any]:
    return {
        "head": command_output(["git", "rev-parse", "HEAD"], root),
        "status": git_status(root),
        "source_sha256": source_hashes(root),
    }


def cpu_model(root: Path = ROOT) -> str:
    if platform.system() == "Darwin":
        return command_output(["sysctl", "-n", "machdep.cpu.brand_string"], root)
    if platform.system() == "Linux":
        try:
            for line in Path("/proc/cpuinfo").read_text().splitlines():
                if line.lower().startswith(("model name", "hardware")):
                    return line.split(":", 1)[1].strip()
        except OSError:
            pass
    return platform.processor() or "unavailable"


def runtime_metadata(root: Path, provenance: dict[str, Any]) -> dict[str, Any]:
    return {
        "toolchain": provenance,
        "hardware": {
            "system": platform.system(),
            "release": platform.release(),
            "machine": platform.machine(),
            "cpu_model": cpu_model(root),
            "logical_cpu_count": os.cpu_count(),
        },
    }


def write_attempt(artifact_dir: Path, attempt: dict[str, Any]) -> Path:
    path = artifact_dir / "time_warp_attempt.json"
    path.write_text(json.dumps(attempt, indent=2, sort_keys=True) + "\n")
    return path


def collect_run(
    root: Path = ROOT,
    artifact_dir: Path = ARTIFACT_DIR,
    command: list[str] | None = None,
    runner: Callable[..., Any] = subprocess.run,
    metadata: dict[str, Any] | None = None,
    toolchain_resolver: Callable[..., dict[str, Any]] = resolved_toolchain,
) -> Path:
    root = root.resolve()
    artifact_dir.mkdir(parents=True, exist_ok=True)
    target_dir = artifact_dir / "target"
    env = os.environ.copy()
    env["CARGO_TARGET_DIR"] = str(target_dir)
    attempt: dict[str, Any] = {
        "command": command,
        "cwd": str(root),
        "execution_environment": {"CARGO_TARGET_DIR": str(target_dir)},
        "exit_code": None,
        "validation_error": None,
    }
    try:
        before = source_state(root)
    except (OSError, RuntimeError, ValueError) as error:
        attempt["validation_error"] = f"could not snapshot build inputs: {error}"
        write_attempt(artifact_dir, attempt)
        raise ValueError(attempt["validation_error"]) from error
    attempt["head_before"] = before["head"]
    attempt["status_before"] = before["status"]
    attempt["source_sha256_before"] = before["source_sha256"]
    if before["status"]:
        attempt["validation_error"] = "source tree is not clean before benchmark execution"
        write_attempt(artifact_dir, attempt)
        raise ValueError(attempt["validation_error"])

    try:
        provenance = toolchain_resolver(root, env)
        env = prepare_toolchain_environment(env, provenance)
        verify_toolchain_environment(env, provenance)
    except (OSError, RuntimeError, ValueError, KeyError) as error:
        attempt["validation_error"] = f"could not resolve exact Rust {TOOLCHAIN} toolchain: {error}"
        write_attempt(artifact_dir, attempt)
        raise ValueError(attempt["validation_error"]) from error

    if command is None:
        command = [provenance["cargo_path"], *CARGO_ARGS]
    attempt["command"] = command
    attempt["compiler_provenance"] = provenance
    attempt["execution_environment"] = {
        "CARGO_TARGET_DIR": str(target_dir),
        "PATH_first": provenance["toolchain_bin"],
        "RUSTC": provenance["rustc_path"],
        "RUSTUP_TOOLCHAIN": provenance["toolchain"],
    }
    attempt.update(metadata if metadata is not None else runtime_metadata(root, provenance))

    try:
        result = runner(command, cwd=root, env=env, text=True, capture_output=True, check=False)
    except OSError as error:
        attempt["validation_error"] = f"benchmark command could not start: {error}"
        write_attempt(artifact_dir, attempt)
        raise RuntimeError(attempt["validation_error"]) from error

    stdout_path = artifact_dir / "time_warp.stdout.json"
    stderr_path = artifact_dir / "time_warp.stderr.log"
    stdout_path.write_text(result.stdout)
    stderr_path.write_text(result.stderr)
    attempt.update(
        {
            "exit_code": result.returncode,
            "stdout_path": str(stdout_path),
            "stdout_sha256": hashlib.sha256(result.stdout.encode()).hexdigest(),
            "stderr_path": str(stderr_path),
            "stderr_sha256": hashlib.sha256(result.stderr.encode()).hexdigest(),
        }
    )
    try:
        after = source_state(root)
    except (OSError, RuntimeError, ValueError) as error:
        attempt["validation_error"] = f"could not verify build inputs after benchmark: {error}"
        write_attempt(artifact_dir, attempt)
        raise ValueError(attempt["validation_error"]) from error
    attempt["head_after"] = after["head"]
    attempt["status_after"] = after["status"]
    attempt["source_sha256_after"] = after["source_sha256"]
    changed = before != after
    try:
        verify_toolchain_environment(env, provenance)
    except (OSError, ValueError, KeyError) as error:
        attempt["validation_error"] = f"Rust toolchain changed during benchmark execution: {error}"
        write_attempt(artifact_dir, attempt)
        raise ValueError(attempt["validation_error"]) from error
    if changed:
        attempt["validation_error"] = "HEAD, clean status, or source hashes changed during benchmark execution"
        write_attempt(artifact_dir, attempt)
        raise ValueError(attempt["validation_error"])
    if result.returncode != 0:
        attempt["validation_error"] = "benchmark command exited unsuccessfully"
        write_attempt(artifact_dir, attempt)
        raise RuntimeError(
            f"benchmark command failed ({result.returncode}); raw logs and attempt receipt saved under {artifact_dir}"
        )

    try:
        document = json.loads(result.stdout)
        input_hashes = validate_output(document)
    except (json.JSONDecodeError, ValueError, TypeError) as error:
        attempt["validation_error"] = f"benchmark output validation failed: {error}"
        write_attempt(artifact_dir, attempt)
        raise ValueError(attempt["validation_error"]) from error
    attempt["input_sha256"] = input_hashes
    write_attempt(artifact_dir, attempt)
    evidence = {
        "schema_version": "kairoecs.pdes.time_warp_evidence.v1",
        **attempt,
        "benchmark": document,
        "timing_claim_boundary": "single-host local runtime run-call comparison; worker cohorts are not a simultaneous CPU-execution measurement",
    }
    output = artifact_dir / "time_warp_evidence.json"
    output.write_text(json.dumps(evidence, indent=2, sort_keys=True) + "\n")
    return output


def collect() -> Path:
    return collect_run()


def main() -> int:
    try:
        path = collect()
    except (OSError, ValueError, RuntimeError, json.JSONDecodeError) as error:
        print(f"time-warp evidence collection failed: {error}", file=sys.stderr)
        return 1
    print(path)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
