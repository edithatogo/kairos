"""Compare exact-source production benchmark binaries on macOS."""
import hashlib
import json
import math
from pathlib import Path
import random
import re
import statistics
import subprocess
import sys

ROOT = Path(sys.argv[1])
PAIRS = 10
SEED = 472026
WORK = (0, 8192)
EXECUTORS = ("pooled", "scoped")
VERSIONS = ("base", "candidate")


def run(pair, executor, cost, variant):
    binary = ROOT / f"{variant}-{executor}"
    command = [
        "/usr/bin/time", "-l", str(binary), "--seed", str(SEED),
        "--repetitions", "2", "--event-work", str(cost),
    ]
    completed = subprocess.run(command, check=True, text=True, capture_output=True)
    result = json.loads(completed.stdout)
    assert result["pdes_executor"] == executor and result["seed"] == SEED
    assert result["warmup_runs"] == 1 and result["repetitions"] == 2
    assert len(result["rows"]) == 1
    row = result["rows"][0]
    assert row["event_work"] == cost and row["parity"]
    assert row["expected_processed_events"] == row["runtime_counters"]["processed_events"]
    for route in ("sequential", "pdes"):
        pairs = zip(row[f"{route}_ns"], row[f"{route}_setup_ns"], row[f"{route}_dispatch_ns"], strict=True)
        assert all(0 < setup + dispatch <= total for total, setup, dispatch in pairs)
    rss = re.search(r"^\s*(\d+)\s+maximum resident set size$", completed.stderr, re.MULTILINE)
    assert rss is not None
    print(json.dumps({
        "pair": pair, "executor": executor, "variant": variant, "event_work": cost,
        "command": command, "peak_rss_bytes": int(rss[1]), "result": result,
    }), flush=True)
    return row


def identity(row):
    return row["input_tape_sha256"], row["ordered_state"], row["runtime_counters"]


def compare(base, candidate, field):
    return math.prod(x / y for x, y in zip(base[field], candidate[field], strict=True)) ** 0.5


def interval(values):
    rng = random.Random(111)
    replicates = sorted(statistics.median(rng.choices(values, k=len(values))) for _ in range(10_000))
    return [replicates[250], replicates[9749]]


binary_paths = [ROOT / f"{version}-{executor}" for version in VERSIONS for executor in EXECUTORS]
print(json.dumps({"binary_sha256": {p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in binary_paths}}))
result = {(executor, cost): [] for executor in EXECUTORS for cost in WORK}
for pair in range(PAIRS):
    cases = list(result)
    if pair % 2:
        cases.reverse()
    for executor, cost in cases:
        order = VERSIONS if pair % 2 == 0 else VERSIONS[::-1]
        rows = {version: run(pair, executor, cost, version) for version in order}
        baseline, candidate = (rows[version] for version in VERSIONS)
        assert identity(baseline) == identity(candidate), (pair, executor, cost, "round semantics drift")
        result[executor, cost].append((baseline, candidate))

stats = {}
for (executor, cost), pairs in result.items():
    rows = {}
    for field in ("pdes_ns", "pdes_setup_ns", "pdes_dispatch_ns", "sequential_ns"):
        base = statistics.median(n for before, _ in pairs for n in before[field])
        candidate = statistics.median(n for _, after in pairs for n in after[field])
        ratios = [compare(before, after, field) for before, after in pairs]
        rows[field] = {
            "base_median": base,
            "candidate_median": candidate,
            "median_paired_ratio": statistics.median(ratios),
            "ratio_interval_95": interval(ratios),
        }
    stats[f"{executor}:{cost}"] = rows

target = stats["pooled:0"]["pdes_ns"]["median_paired_ratio"]
comparators = [case["pdes_ns"]["median_paired_ratio"] for case in stats.values()]
status = "pass" if target >= 1.25 and min(comparators) >= 1 / 1.05 else "below-threshold"
(ROOT / "comparison.json").write_text(json.dumps({"status": status, "cases": stats}, indent=2) + "\n")
