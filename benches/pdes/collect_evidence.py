#!/usr/bin/env python3
"""Collect bounded Track 47 raw PDES benchmark evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import plistlib
import re
import subprocess
import sys
from datetime import datetime, timezone
from uuid import uuid4


ROOT = Path(__file__).resolve().parents[2]
EVIDENCE_ROOT = ROOT / "benches" / "pdes" / "evidence"
TOOLCHAIN = "1.98.1"


def command_output(command: list[str], *, timeout: int = 20) -> str:
    try:
        result = subprocess.run(
            command,
            cwd=ROOT,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            check=False,
            timeout=timeout,
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        return f"unavailable ({type(error).__name__}: {error})"
    if result.returncode != 0:
        return f"unavailable (command exited {result.returncode})"
    return result.stdout.strip() or f"command returned no output (exit {result.returncode})"


def git(*args: str) -> str:
    result = subprocess.run(
        ["git", *args], cwd=ROOT, text=True, stdout=subprocess.PIPE,
        stderr=subprocess.PIPE, check=True
    )
    return result.stdout.rstrip("\n")


def normalize_git_status(raw: str) -> str:
    """Remove command terminators without dropping porcelain status columns."""
    return raw.rstrip("\n")


def source_tree_digest() -> str:
    digest = hashlib.sha256()
    paths = git("ls-files", "-co", "--exclude-standard", "-z").split("\0")
    for relative in sorted(
        path for path in paths if path and not path.startswith("benches/pdes/evidence/")
    ):
        path = ROOT / relative
        if not path.is_file():
            continue
        digest.update(relative.encode("utf-8", errors="surrogateescape"))
        digest.update(b"\0")
        with path.open("rb") as handle:
            for block in iter(lambda: handle.read(1024 * 1024), b""):
                digest.update(block)
        digest.update(b"\0")
    return digest.hexdigest()


def non_evidence_source_changes(status: str) -> list[str]:
    changed = []
    for line in status.splitlines():
        relative = line[3:].split(" -> ")[-1].strip().strip('"')
        if not relative.startswith("benches/pdes/evidence/"):
            changed.append(relative)
    return sorted(set(changed))


def require_unchanged_source(before: str, after: str) -> None:
    if before != after:
        raise ValueError("source tree changed while the benchmark was running; discard this run and retry")


def validate_ref(ref: str, commit_sha: str) -> None:
    if ref == "local-only":
        return
    if not re.fullmatch(r"[A-Za-z0-9._/-]+", ref) or ref.startswith("-") or ".." in ref:
        raise ValueError("--pushed-ref must be local-only or a plain Git ref name")
    if ref.startswith("refs/remotes/origin/"):
        branch = ref.removeprefix("refs/remotes/origin/")
        result = subprocess.run(
            ["git", "ls-remote", "--exit-code", "origin", f"refs/heads/{branch}"],
            cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            check=False, timeout=30,
        )
        if result.returncode != 0:
            raise ValueError(f"pushed ref could not be verified on origin: {ref}")
        remote_sha = result.stdout.split()[0] if result.stdout.split() else ""
        if remote_sha != commit_sha:
            raise ValueError("pushed origin ref does not point at --commit-sha")
        return
    try:
        resolved = git("rev-parse", "--verify", f"{ref}^{{commit}}")
    except subprocess.CalledProcessError as error:
        raise ValueError(f"--pushed-ref does not resolve to a local commit: {ref}") from error
    if resolved != commit_sha:
        raise ValueError("--pushed-ref must resolve to --commit-sha for this local collection")


def hardware_metadata() -> dict[str, str | int]:
    if platform.system() == "Darwin":
        cpu_model = command_output(["sysctl", "-n", "machdep.cpu.brand_string"])
        physical = command_output(["sysctl", "-n", "hw.physicalcpu"])
        logical = command_output(["sysctl", "-n", "hw.logicalcpu"])
        memory = command_output(["sysctl", "-n", "hw.memsize"])
        memory_topology = f"{int(memory):,} bytes total; NUMA topology not exposed by this macOS host" if memory.isdigit() else memory
        cpu_topology = f"{physical} physical cores; {logical} logical CPUs" if physical.isdigit() and logical.isdigit() else f"physical={physical}; logical={logical}"
        physical = int(physical) if physical.isdigit() else 0
        memory_bytes = int(memory) if memory.isdigit() else 0
        accelerator = "none; this benchmark exercises CPU threads only"
        driver = "none; no accelerator driver is used by this benchmark"
    elif platform.system() == "Linux":
        cpuinfo = Path("/proc/cpuinfo").read_text(encoding="utf-8", errors="replace")
        cpu_model = next((line.split(":", 1)[1].strip() for line in cpuinfo.splitlines() if line.startswith("model name")), "")
        logical = os.cpu_count() or 0
        physical_ids = set()
        for block in cpuinfo.split("\n\n"):
            values = dict(line.split(":", 1) for line in block.splitlines() if ":" in line)
            if "physical id" in values and "core id" in values:
                physical_ids.add((values["physical id"].strip(), values["core id"].strip()))
        physical = len(physical_ids)
        memory = next((line.split(":", 1)[1].strip() for line in Path("/proc/meminfo").read_text().splitlines() if line.startswith("MemTotal:")), "")
        cpu_topology = f"{physical} physical cores; {logical} logical CPUs"
        nodes = sorted(Path("/sys/devices/system/node").glob("node[0-9]*"))
        numa_summary = "; ".join(
            f"{node.name} {next((line.split(':', 1)[1].strip() for line in (node / 'meminfo').read_text().splitlines() if 'MemTotal' in line), 'memory unreported')}"
            for node in nodes
        ) or "single node or NUMA detail not exposed"
        memory_topology = f"{memory}; NUMA nodes: {numa_summary}"
        memory_match = re.search(r"(\d+)\s+kB", memory)
        memory_bytes = int(memory_match.group(1)) * 1024 if memory_match else 0
        accelerator = "none; this benchmark exercises CPU threads only"
        driver = "none; no accelerator driver is used by this benchmark"
    else:
        cpu_model = platform.processor() or "platform.processor() returned empty"
        logical = os.cpu_count() or 0
        physical = 0
        cpu_topology = f"{physical}; {logical} logical CPUs"
        memory_topology = "total and NUMA memory topology not exposed by collector"
        memory_bytes = 0
        accelerator = "none; this benchmark exercises CPU threads only"
        driver = "none; no accelerator driver is used by this benchmark"
    return {
        "cpu_model": cpu_model,
        "cpu_topology": cpu_topology,
        "logical_cpu_count": os.cpu_count() or 0,
        "physical_cpu_count": physical,
        "memory_bytes": memory_bytes,
        "memory_topology": memory_topology,
        "accelerator_model": accelerator,
        "driver": driver,
    }


def filesystem_metadata() -> str:
    if platform.system() == "Darwin":
        # BSD stat %T reports file kind, not the backing filesystem. Resolve the
        # actual source volume through df, then retain only its filesystem type.
        try:
            volume = subprocess.run(
                ["df", "-P", str(ROOT)], cwd=ROOT, text=True,
                stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=True,
                timeout=20,
            )
            device = volume.stdout.splitlines()[1].split()[0]
            info = subprocess.run(
                ["diskutil", "info", "-plist", device], cwd=ROOT,
                stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=True,
                timeout=20,
            )
            metadata = plistlib.loads(info.stdout)
            result = metadata.get("FilesystemType")
            if metadata.get("Error") or not isinstance(result, str) or not result:
                raise ValueError("source volume filesystem type is absent")
        except (OSError, subprocess.SubprocessError, IndexError,
                plistlib.InvalidFileException) as error:
            raise ValueError("source volume filesystem type could not be collected") from error
    elif platform.system() == "Linux":
        result = command_output(["stat", "-f", "-c", "%T", str(ROOT)])
        if result.startswith(("unavailable", "command returned no output")):
            raise ValueError("filesystem type could not be collected")
    else:
        raise ValueError(f"filesystem type collection is unsupported on {platform.system()}")
    return f"{result}; local repository filesystem"


def validate_result(text: str, repetitions: int, seed: int) -> dict:
    result = json.loads(text)
    if result.get("schema_version") != "kairoecs.pdes.benchmark.v1":
        raise ValueError("benchmark output has an unsupported schema_version")
    if result.get("repetitions") != repetitions or result.get("seed") != seed:
        raise ValueError("benchmark output does not match requested seed/repetitions")
    rows = result.get("rows")
    if not isinstance(rows, list) or len(rows) != 8:
        raise ValueError("benchmark output must contain 8 strong/weak scaling rows")
    expected = {(scale, lp) for scale in ("strong", "weak") for lp in (4, 8, 16, 32)}
    observed = {(row.get("scaling"), row.get("lp_count")) for row in rows}
    if observed != expected:
        raise ValueError("benchmark row matrix is incomplete or contains unexpected cases")
    for row in rows:
        for key in ("sequential_ns", "pdes_ns"):
            samples = row.get(key)
            if not isinstance(samples, list) or len(samples) != repetitions or any(
                not isinstance(sample, int) or sample <= 0 for sample in samples
            ):
                raise ValueError(f"{key} must contain {repetitions} positive samples")
        if row.get("parity") is not True:
            raise ValueError("benchmark reported a sequential parity failure")
        counters = row.get("runtime_counters")
        if not isinstance(counters, dict):
            raise ValueError("runtime counters are missing")
        if counters.get("processed_events") != row.get("expected_processed_events"):
            raise ValueError("runtime processed-event count does not match expected workload")
        if counters.get("remote_events") != row.get("initial_events"):
            raise ValueError("cross-LP event count does not match emitted workload")
        if counters.get("emitted_events") != row.get("initial_events"):
            raise ValueError("emitted-event count does not match initial workload")
        if not all(isinstance(counters.get(key), int) for key in ("null_messages", "rounds", "gvt_ticks", "worker_count")):
            raise ValueError("null-message, round, GVT, and worker counters must be integers")
        if counters["worker_count"] < 1 or counters["rounds"] < 1:
            raise ValueError("runtime must report at least one worker and one scheduler round")
        if counters["worker_count"] != row["lp_count"]:
            raise ValueError("worker count must reflect every LP active in the concurrent round")
        if counters["gvt_ticks"] != 1 or counters["null_messages"] < 1:
            raise ValueError("runtime GVT and null-message progression is incomplete")
        for key in ("sequential_events_per_second", "pdes_events_per_second"):
            rates = row.get(key)
            if not isinstance(rates, list) or len(rates) != repetitions or any(
                not isinstance(rate, (int, float)) or rate <= 0 for rate in rates
            ):
                raise ValueError(f"{key} must contain {repetitions} positive throughput samples")
            time_key = "sequential_ns" if key.startswith("sequential") else "pdes_ns"
            expected_rates = [row["expected_processed_events"] * 1_000_000_000 / ns for ns in row[time_key]]
            if any(abs(actual - expected) / expected > 0.002 for actual, expected in zip(rates, expected_rates)):
                raise ValueError(f"{key} does not match the recorded event count and elapsed samples")
    return result


def write_exclusive(path: Path, text: str | bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    mode = "xb" if isinstance(text, bytes) else "x"
    with path.open(mode) as handle:
        handle.write(text)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--commit-sha", required=True, help="actual checked-out base commit SHA")
    parser.add_argument("--pushed-ref", required=True, help="ref associated with the supplied commit; use local-only before push")
    parser.add_argument("--evidence-class", choices=("scaffold", "live-hpc"), default="scaffold")
    parser.add_argument("--seed", type=int, default=47_2026)
    parser.add_argument("--repetitions", type=int, default=5)
    parser.add_argument("--reviewer", default="local benchmark collection")
    args = parser.parse_args()

    if not re.fullmatch(r"[0-9a-f]{40}", args.commit_sha):
        parser.error("--commit-sha must be a 40-character lowercase hexadecimal SHA")
    head = git("rev-parse", "HEAD")
    if args.commit_sha != head:
        parser.error(f"--commit-sha must identify checked-out HEAD ({head})")
    if not args.pushed_ref.strip():
        parser.error("--pushed-ref must be nonempty; use local-only before push")
    try:
        validate_ref(args.pushed_ref, args.commit_sha)
    except ValueError as error:
        parser.error(str(error))
    if args.evidence_class == "live-hpc" and args.pushed_ref == "local-only":
        parser.error("live-hpc evidence requires a pushed ref that resolves to the tested commit")
    if args.evidence_class == "live-hpc" and not args.pushed_ref.startswith("refs/remotes/origin/"):
        parser.error("live-hpc evidence requires a remotely verified refs/remotes/origin/<branch>")
    if args.evidence_class == "live-hpc" and (not args.reviewer.strip() or args.reviewer == "local benchmark collection"):
        parser.error("live-hpc evidence requires an explicit reviewer name or handle")
    if not 1 <= args.repetitions <= 100:
        parser.error("--repetitions must be in the range 1..100")

    git_status = normalize_git_status(git("status", "--porcelain", "--untracked-files=all"))
    dirty_sources = non_evidence_source_changes(git_status)
    if dirty_sources:
        parser.error("source inputs must be committed before collection: " + ", ".join(dirty_sources))
    dirty = False
    source_sha256 = source_tree_digest()
    hardware = hardware_metadata()
    try:
        filesystem = filesystem_metadata()
    except ValueError as error:
        parser.error(str(error))
    if (
        not hardware["cpu_model"] or not hardware["cpu_topology"]
        or not hardware["memory_topology"]
        or str(hardware["cpu_model"]).startswith("unavailable")
        or int(hardware["physical_cpu_count"]) < 1
        or int(hardware["logical_cpu_count"]) < 1
        or int(hardware["memory_bytes"]) < 1
    ):
        parser.error("host CPU, CPU topology, and memory topology must be collected")
    started_at = datetime.now(timezone.utc).isoformat(timespec="seconds")
    command = [
        "rustup", "run", TOOLCHAIN, "cargo", "bench", "-p", "kairo-ecs-pdes",
        "--bench", "production", "--features", "pdes", "--", "--seed",
        str(args.seed), "--repetitions", str(args.repetitions),
    ]
    completed = subprocess.run(
        command, cwd=ROOT, text=True, stdout=subprocess.PIPE,
        stderr=subprocess.PIPE, check=False, timeout=1800
    )
    if completed.returncode != 0:
        sys.stderr.write(completed.stderr)
        return completed.returncode
    benchmark = validate_result(completed.stdout, args.repetitions, args.seed)
    try:
        require_unchanged_source(source_sha256, source_tree_digest())
    except ValueError as error:
        parser.error(str(error))

    compiler = command_output(["rustup", "run", TOOLCHAIN, "rustc", "-Vv"])
    if compiler.startswith("unavailable") or compiler.startswith("command returned no output"):
        parser.error("Rust compiler metadata could not be collected")
    command_text = " ".join(command) + "\n"
    environment = {
        "platform": platform.platform(),
        "python": sys.version,
        "git_status_porcelain": git_status,
        "source_tree_dirty": dirty,
        "source_tree_sha256": source_sha256,
        "physical_cpu_count": hardware["physical_cpu_count"],
        "memory_bytes": hardware["memory_bytes"],
        "cpu_count_logical": os.cpu_count(),
        "filesystem": filesystem,
        "environment_variables": {
            key: os.environ[key]
            for key in ("RUSTFLAGS", "CARGO_PROFILE_BENCH_OPT_LEVEL", "CARGO_PROFILE_BENCH_LTO", "RUSTUP_TOOLCHAIN")
            if key in os.environ
        },
        "command": command_text.strip(),
    }
    timestamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    bundle_name = f"{timestamp}-{args.commit_sha[:10]}-{uuid4().hex[:8]}"
    bundle = EVIDENCE_ROOT / bundle_name
    bundle.mkdir(parents=True, exist_ok=False)
    raw_path = bundle / "benchmark-result.json"
    raw_bytes = completed.stdout.encode()
    write_exclusive(raw_path, raw_bytes)
    raw_sha = hashlib.sha256(raw_bytes).hexdigest()
    write_exclusive(bundle / "benchmark-command.txt", command_text)
    write_exclusive(bundle / "benchmark-stderr.txt", completed.stderr)
    write_exclusive(bundle / "benchmark-environment.json", json.dumps(environment, indent=2, sort_keys=True) + "\n")

    manifest = {
        "schema_version": "kairoecs.hpc.evidence.v1",
        "track_id": "47",
        "task_id": "3.2",
        "commit_sha": args.commit_sha,
        "pushed_ref": args.pushed_ref,
        "evidence_class": args.evidence_class,
        "capability": "Local strong/weak scaling timing and sequential final-state parity for the conservative PDES runtime",
        "hardware": {
            "cpu_model": hardware["cpu_model"],
            "cpu_topology": hardware["cpu_topology"],
            "memory_topology": hardware["memory_topology"],
            "accelerator_model": hardware["accelerator_model"],
            "driver": hardware["driver"],
        },
        "system": {"operating_system": platform.platform()},
        "toolchain": {
            "rust_toolchain": TOOLCHAIN,
            "compiler": compiler,
            "mpi_implementation": "not used; in-process local runtime",
            "scheduler": "local operating-system thread scheduler; no cluster scheduler",
        },
        "runtime": {
            "command": command_text.strip(),
            "environment": {
                **environment,
                "started_at_utc": started_at,
                "working_tree_state": "clean tested commit; generated evidence is stored separately",
                "runtime_workers": "reported per benchmark row by runtime worker_count counters",
                "threading": "runtime-owned OS threads; no external transport",
            },
            "feature_flags": ["pdes"],
            "input_scenario": {
                "lp_counts": [4, 8, 16, 32],
                "topology": "directed ring; each LP sends to its successor",
                "scaling_profiles": {
                    "strong": f"{benchmark['strong_total_events']} initial events total across LP counts",
                    "weak": f"{benchmark['weak_events_per_lp']} initial events per LP",
                },
                "seed": args.seed,
                "repetitions": args.repetitions,
                "warmup_runs": benchmark["warmup_runs"],
            },
        },
        "storage": {"filesystem_or_object_store": environment["filesystem"]},
        "result": {
            "expected": "All 8 strong/weak configurations complete with sequential final-state parity; raw timing samples and runtime counters are recorded.",
            "observed": f"All 8 configurations reported parity=true. Single-host local CPU/thread runtime measurement; dirty_tree={str(dirty).lower()}. This is not distributed HPC parity certification.",
            "raw_artifact_path": raw_path.relative_to(ROOT).as_posix(),
            "checksum": f"sha256:{raw_sha}",
        },
        "review": {"reviewer": args.reviewer, "evidence_date": datetime.now(timezone.utc).date().isoformat()},
        "waiver": {
            "status": "none" if args.evidence_class == "live-hpc" else "not-live",
            "owner": "none" if args.evidence_class == "live-hpc" else "Track 47",
            "expires": "none" if args.evidence_class == "live-hpc" else "Until reviewed as part of Track 55 evidence",
        },
    }
    write_exclusive(bundle / "manifest.json", json.dumps(manifest, indent=2, sort_keys=True) + "\n")
    print(json.dumps({"status": "ok", "manifest": (bundle / "manifest.json").relative_to(ROOT).as_posix(), "raw_artifact": raw_path.relative_to(ROOT).as_posix(), "checksum": f"sha256:{raw_sha}"}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
