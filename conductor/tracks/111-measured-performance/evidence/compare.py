"""Use two production benchmark binaries from one source tree on macOS."""

import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path

COSTS = (0, 512, 2048, 8192, 32768)
PAIRS = 10
SEED = 472026
PROGRAMS = dict(zip(("scoped", "pooled"), map(Path, sys.argv[1:3]), strict=True))


def run(variant, pair, work):
    program = PROGRAMS[variant]
    repetitions = 10 if work is None else 2
    command = ["/usr/bin/time", "-l", str(program), "--seed", str(SEED),
               "--repetitions", str(repetitions)]
    if work is not None:
        command += ["--event-work", str(work)]
    completed = subprocess.run(command, check=True, text=True, capture_output=True)
    result = json.loads(completed.stdout)
    expected = "kairoecs.pdes.benchmark.v1" if work is None else "kairoecs.pdes.crossover.v1"
    assert result["schema_version"] == expected
    assert result["pdes_executor"] == variant
    assert result["seed"] == SEED and result["repetitions"] == repetitions
    assert len(result["rows"]) == (8 if work is None else 1)
    for row in result["rows"]:
        assert row["parity"]
        assert row["ordered_state"]
        assert row["runtime_counters"]["processed_events"] == row["expected_processed_events"]
        for executor in ("sequential", "pdes"):
            total = row[f"{executor}_ns"]
            setup = row[f"{executor}_setup_ns"]
            dispatch = row[f"{executor}_dispatch_ns"]
            assert len(total) == repetitions
            assert all(0 < i + d <= t for t, i, d in zip(total, setup, dispatch, strict=True))
    rss = re.search(r"^\s*(\d+)\s+maximum resident set size$", completed.stderr, re.MULTILINE)
    assert rss is not None
    print(json.dumps({
        "pair": pair, "variant": variant, "work": work,
        "peak_rss_bytes": int(rss[1]), "time": completed.stderr, "result": result,
    }), flush=True)
    return [(row["input_tape_sha256"], row["ordered_state"], row["runtime_counters"])
            for row in result["rows"]]


if __name__ == "__main__":
    print(json.dumps({"binary_sha256": {
        name: hashlib.sha256(path.read_bytes()).hexdigest() for name, path in PROGRAMS.items()
    }}), flush=True)
    for pair in range(PAIRS):
        variants = tuple(PROGRAMS) if pair % 2 == 0 else tuple(reversed(PROGRAMS))
        costs = COSTS if pair % 2 == 0 else COSTS[::-1]
        for work in costs:
            first, second = (run(variant, pair, work) for variant in variants)
            assert first == second, (pair, work, "paired executor state drift")
    assert run("scoped", None, None) == run("pooled", None, None), "legacy matrix drift"
