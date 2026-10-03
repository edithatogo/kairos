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
LINUX_PROC_ROOT = Path("/proc")
LINUX_SYS_ROOT = Path("/sys")


def command_output(
    command: list[str], *, timeout: int = 20, env: dict[str, str] | None = None
) -> str:
    try:
        result = subprocess.run(
            command,
            cwd=ROOT,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            check=False,
            timeout=timeout,
            env=env,
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


def parse_cpu_list(value: str) -> set[int] | None:
    """Parse the comma/range format used by Linux sysfs CPU lists."""
    cpus: set[int] = set()
    try:
        for part in value.strip().split(","):
            bounds = part.split("-")
            if len(bounds) == 1:
                start = end = int(bounds[0])
            elif len(bounds) == 2:
                start, end = map(int, bounds)
            else:
                return None
            if start < 0 or end < start or end - start > 1_000_000:
                return None
            cpus.update(range(start, end + 1))
    except ValueError:
        return None
    return cpus or None


def linux_sysfs_core_count(cpu_root: Path, logical: int) -> int | None:
    """Count kernel-visible core groups without guessing from logical CPUs."""
    cpu_dirs = sorted(
        (path for path in cpu_root.glob("cpu[0-9]*") if path.name[3:].isdigit()),
        key=lambda path: int(path.name[3:]),
    )
    online_path = cpu_root / "online"
    try:
        online = parse_cpu_list(online_path.read_text(encoding="utf-8"))
    except OSError:
        online = None
    if online is not None:
        cpu_dirs = [path for path in cpu_dirs if int(path.name[3:]) in online]
        if len(cpu_dirs) != len(online):
            return None
    if not cpu_dirs:
        return None

    package_core_ids: set[tuple[int, int]] = set()
    package_core_complete = True
    for cpu_dir in cpu_dirs:
        topology = cpu_dir / "topology"
        try:
            package = int((topology / "physical_package_id").read_text(encoding="utf-8").strip())
            core = int((topology / "core_id").read_text(encoding="utf-8").strip())
        except (OSError, ValueError):
            package_core_complete = False
            break
        if package < 0 or core < 0:
            package_core_complete = False
            break
        package_core_ids.add((package, core))
    if package_core_complete and package_core_ids:
        return len(package_core_ids)

    sibling_groups: set[tuple[int, ...]] = set()
    cpu_to_group: dict[int, tuple[int, ...]] = {}
    for cpu_dir in cpu_dirs:
        cpu = int(cpu_dir.name[3:])
        try:
            siblings = parse_cpu_list(
                (cpu_dir / "topology" / "thread_siblings_list").read_text(encoding="utf-8")
            )
        except OSError:
            return None
        if siblings is None or cpu not in siblings:
            return None
        group = tuple(sorted(siblings))
        sibling_groups.add(group)
        cpu_to_group[cpu] = group
    assigned: dict[int, tuple[int, ...]] = {}
    for group in sibling_groups:
        for cpu in group:
            previous = assigned.setdefault(cpu, group)
            if previous != group:
                return None
    if any(assigned.get(cpu) != group for cpu, group in cpu_to_group.items()):
        return None
    if logical > 0 and len(sibling_groups) > logical:
        return None
    return len(sibling_groups) if sibling_groups else None


def linux_proc_core_count(cpuinfo: str) -> int | None:
    """Read complete x86-style package/core pairs when the kernel provides them."""
    cores: set[tuple[int, int]] = set()
    blocks = [block for block in cpuinfo.split("\n\n") if block.strip()]
    if not blocks:
        return None
    for block in blocks:
        values = {
            key.strip(): value.strip()
            for line in block.splitlines() if ":" in line
            for key, value in [line.split(":", 1)]
        }
        if not all(key in values for key in ("processor", "physical id", "core id")):
            return None
        try:
            processor, package, core = (
                int(values[key].strip())
                for key in ("processor", "physical id", "core id")
            )
        except ValueError:
            return None
        if min(processor, package, core) < 0:
            return None
        cores.add((package, core))
    return len(cores) if cores else None


def linux_lscpu_core_count(output: str) -> int | None:
    """Parse lscpu's explicitly requested CPU/package/core CSV columns."""
    header: list[str] | None = None
    rows: list[list[str]] = []
    for line in output.splitlines():
        line = line.strip()
        if line.startswith("#"):
            fields = [field.strip().upper() for field in line.lstrip("# ").split(",")]
            if {"CPU", "SOCKET", "CORE"}.issubset(fields):
                header = fields
        elif line:
            rows.append([field.strip() for field in line.split(",")])
    if header is None or not rows:
        return None
    try:
        cpu_index, socket_index, core_index = (
            header.index("CPU"), header.index("SOCKET"), header.index("CORE")
        )
        groups: set[tuple[int, int]] = set()
        processors: set[int] = set()
        for row in rows:
            processor, socket, core = (
                int(row[index]) for index in (cpu_index, socket_index, core_index)
            )
            if min(processor, socket, core) < 0 or processor in processors:
                return None
            processors.add(processor)
            groups.add((socket, core))
    except (IndexError, ValueError):
        return None
    return len(groups) if groups else None


def linux_cpu_model(cpuinfo: str, lscpu: str) -> str:
    """Return a model string actually reported by procfs or lscpu."""
    preferred = ("model name", "Hardware", "Processor", "Model")
    values = dict(
        (key.strip().lower(), value.strip())
        for line in cpuinfo.splitlines() if ":" in line
        for key, value in [line.split(":", 1)]
    )
    for key in preferred:
        value = values.get(key.lower(), "")
        if (
            value and value.lower() not in {"unknown", "none", "-1"}
            and not re.fullmatch(r"\d+", value)
        ):
            return value
    implementer = values.get("cpu implementer", "")
    part = values.get("cpu part", "")
    if implementer and part:
        return f"ARM CPU identifiers reported by kernel: implementer {implementer}, part {part}"
    for line in lscpu.splitlines():
        key, separator, value = line.partition(":")
        if separator and key.strip().lower() in {"model name", "model", "hardware"}:
            value = value.strip()
            if value and value.lower() not in {"unknown", "none", "-1"}:
                return value
    return "unavailable (no CPU model field reported)"


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
        try:
            cpuinfo = (LINUX_PROC_ROOT / "cpuinfo").read_text(encoding="utf-8", errors="replace")
        except OSError:
            cpuinfo = ""
        logical = os.cpu_count() or 0
        stable_env = {**os.environ, "LC_ALL": "C"}
        lscpu = command_output(["lscpu"], env=stable_env)
        cpu_model = linux_cpu_model(cpuinfo, lscpu)
        cpu_root = LINUX_SYS_ROOT / "devices/system/cpu"
        physical = linux_sysfs_core_count(cpu_root, logical)
        topology_source = "sysfs"
        if physical is None:
            physical = linux_proc_core_count(cpuinfo)
            topology_source = "procfs"
        if physical is None:
            physical = linux_lscpu_core_count(
                command_output(["lscpu", "--parse=CPU,SOCKET,CORE"], env=stable_env)
            )
            topology_source = "lscpu"
        if physical is None:
            physical = 0
            cpu_topology = f"unknown (kernel core topology unavailable); {logical} logical CPUs"
        else:
            cpu_topology = f"{physical} kernel-reported core groups ({topology_source}); {logical} logical CPUs"
        try:
            meminfo = (LINUX_PROC_ROOT / "meminfo").read_text(encoding="utf-8", errors="replace")
        except OSError:
            meminfo = ""
        memory = next((line.split(":", 1)[1].strip() for line in meminfo.splitlines() if line.startswith("MemTotal:")), "")
        nodes = sorted((LINUX_SYS_ROOT / "devices/system/node").glob("node[0-9]*"))
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


def execution_metadata(input_scenario: dict, exit_status: int) -> dict:
    """Identify the exact generated scenario and completed benchmark invocation."""
    canonical_input = json.dumps(
        input_scenario, sort_keys=True, separators=(",", ":"), ensure_ascii=True
    ).encode("utf-8")
    return {
        "working_directory": ".",
        "benchmark_exit_status": exit_status,
        "input_scenario_sha256": "sha256:" + hashlib.sha256(canonical_input).hexdigest(),
        "input_hash_scope": "canonical input_scenario JSON; sorted keys, compact separators, ASCII escapes, UTF-8 bytes; payload generator defined by source commit",
    }


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
            raise ValueError("worker count must reflect the spawned LP cohort in a round")
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
    input_scenario = {
        "lp_counts": [4, 8, 16, 32],
        "topology": "directed ring; each LP sends to its successor",
        "payload_generator": "SplitMix64; implementation fixed by source commit",
        "scaling_profiles": {
            "strong": f"{benchmark['strong_total_events']} initial events total across LP counts",
            "weak": f"{benchmark['weak_events_per_lp']} initial events per LP",
        },
        "seed": args.seed,
        "repetitions": args.repetitions,
        "warmup_runs": benchmark["warmup_runs"],
    }
    environment = {
        **execution_metadata(input_scenario, completed.returncode),
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
                "runtime_workers": "worker_count is the maximum spawned LP worker cohort in one round; it does not measure simultaneous CPU execution",
                "threading": "runtime-owned OS threads; no external transport",
            },
            "feature_flags": ["pdes"],
            "input_scenario": input_scenario,
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
