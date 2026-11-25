"""Capture one matched serial and pooled spatial workload on locked Kairos."""

from collections import Counter
from datetime import datetime, timezone
from hashlib import sha256
import json
import os
from pathlib import Path
import platform
import random
import statistics
import subprocess
import sys


EVIDENCE = Path(__file__).resolve().parent
REPO = EVIDENCE.parents[4]
TAPE = EVIDENCE / "tape.tsv"
MODEL = EVIDENCE / "model.rs"
SEED = 11_120_261_126
ENGINES = ("serial", "pooled")
METRICS = ("setup_ns", "dispatch_ns", "total_ns", "events", "cross_region_events", "workers")


def command(*args):
    return subprocess.check_output(args, cwd=REPO)


def frozen_tape():
    rng = random.Random(SEED)
    rows = [
        (region, rng.randrange(16), rng.randrange(16), rng.randrange(5))
        for region in range(4)
        for _ in range(96)
    ]
    return "".join("\t".join(map(str, row)) + "\n" for row in rows).encode()


def model_check(trace, input_bytes):
    """Validate the exact event history with independent integer neighbor search."""
    starts = [list() for _ in range(4)]
    for row in input_bytes.splitlines():
        region, x, y, wake = map(int, row.split(b"\t"))
        starts[region].append((x, y, wake))
    states = [[(x, y, 0, 0) for x, y, _ in region] for region in starts]
    final = {}
    expected_signals = Counter()
    received_signals = Counter()
    previous_key = [(-1, -1)] * 4
    moves = tied = changed_by_signal = 0

    def shift(x, delta):
        return min(15, max(0, x + delta))

    for row in trace.splitlines():
        label, *values = row.split(b"\t")
        fields = list(map(int, values))
        if label == b"final":
            region, actor, *state = fields
            if (region, actor) in final:
                raise ValueError("duplicate final state")
            final[region, actor] = tuple(state)
            continue
        if label != b"event":
            raise ValueError(f"unknown output row {label}")
        region, tick, source, kind, actor, x, y, move_count, signals, peer = fields
        key = (tick, source)
        if key < previous_key[region]:
            raise ValueError("elapsed time or source order regressed")
        tied += tick == previous_key[region][0]
        previous_key[region] = key
        x0, y0, prior_moves, prior_signals = states[region][actor]
        if kind == 0:
            moves += 1
            earliest = starts[region][actor][2] + prior_moves * (2 + actor % 4)
            if tick != earliest or source != region or prior_moves >= 8:
                raise ValueError("movement violated its schedule")
            squared = [
                ((x1 - x0) ** 2 + (y1 - y0) ** 2, other)
                for other, (x1, y1, _, _) in enumerate(states[region]) if other != actor
            ]
            nearest = min((pair for pair in squared if pair[0] <= 4), default=(0, -1))[1]
            if peer != nearest:
                raise ValueError("ordered history selected the wrong nearest neighbor")
            if nearest >= 0:
                expected_signals[(region + 1) % 4, tick + 2, region, nearest] += 1
                x1, y1, _, _ = states[region][nearest]
                dx = (x1 > x0) - (x1 < x0)
                dy = (y1 > y0) - (y1 < y0)
            else:
                dx, dy = 0, 1
            if dx == dy == 0:
                dx = 1
            sign = (-1) ** (prior_signals % 2)
            dest = (shift(x0, sign * dx), shift(y0, sign * dy))
            changed_by_signal += dest != (shift(x0, dx), shift(y0, dy))
            expected = (*dest, prior_moves + 1, prior_signals)
        elif kind == 1:
            if peer != -1 or region != (source + 1) % 4:
                raise ValueError("signal violated the declared route")
            received_signals[region, tick, source, actor] += 1
            expected = (x0, y0, prior_moves, prior_signals + 1)
        else:
            raise ValueError(f"unknown event kind {kind}")
        if (x, y, move_count, signals) != expected:
            raise ValueError("state differs from the integer event replay")
        states[region][actor] = expected

    reconstructed = {(r, a): s for r, region in enumerate(states) for a, s in enumerate(region)}
    if final != reconstructed or moves != 4 * 96 * 8 or received_signals != expected_signals:
        raise ValueError("incomplete terminal state, moves or delayed signals")
    signal_count = sum(received_signals.values())
    if not signal_count or not changed_by_signal or not tied:
        raise ValueError("frozen workload did not exercise the expected branches")
    return {
        "agents": len(final), "moves": moves, "signals": signal_count,
        "moves_changed_by_signals": changed_by_signal, "same_tick_events": tied,
    }


def compare(left, right):
    if left != right:
        raise ValueError("serial and pooled ordered traces or final states differ")


def interval(values):
    rng = random.Random(SEED)
    draws = [statistics.median(rng.choices(values, k=len(values))) for _ in range(10_000)]
    bounds = statistics.quantiles(draws, n=40, method="inclusive")
    return {"median": statistics.median(values), "bootstrap_95": [bounds[0], bounds[-1]]}


def main():
    binary, receipt_path = map(Path, sys.argv[1:])
    if sys.version_info[:3] != (3, 15, 2) or Path(sys.prefix) != REPO / ".venv":
        raise ValueError("select the repository-local Python 3.15.2")
    if receipt_path.exists():
        raise ValueError("choose a fresh receipt path")
    input_bytes = TAPE.read_bytes()
    if input_bytes != frozen_tape():
        raise ValueError("initial states differ from the frozen seed")
    source = binary.read_bytes()
    rustc = command("rustup", "which", "--toolchain", "1.99.1", "rustc").decode().strip()
    compiler = command(rustc, "-Vv").decode().strip()
    commit = next(row.removeprefix("commit-hash: ") for row in compiler.splitlines()
                  if row.startswith("commit-hash: "))
    if b"/rustc/" + commit.encode() not in source:
        raise ValueError("executable does not embed the selected compiler revision")

    rows = []
    result_trace = None
    counts = None
    for pair in range(11):
        order = ENGINES if pair % 2 == 0 else ENGINES[::-1]
        records = {}
        traces = {}
        for engine in order:
            stdout = command(binary.resolve(), engine, TAPE)
            header, _, traces[engine] = stdout.partition(b"\n")
            records[engine] = dict(zip(METRICS, map(int, header.split(b"\t")), strict=True))
        compare(traces["serial"], traces["pooled"])
        if result_trace is None:
            result_trace = traces["serial"]
            counts = model_check(result_trace, input_bytes)
        else:
            compare(result_trace, traces["serial"])
        for engine, metrics in records.items():
            if (metrics["setup_ns"] + metrics["dispatch_ns"] != metrics["total_ns"]
                    or metrics["events"] != counts["moves"] + counts["signals"]
                    or metrics["cross_region_events"] != counts["signals"]
                    or metrics["workers"] != (1 if engine == "serial" else 4)):
                raise ValueError(f"counters differ for {engine}")
        rows.append({"pair": pair, "order": order, "metrics": records})

    timed = rows[1:]
    receipt = {
        "utc": datetime.now(timezone.utc).isoformat(timespec="seconds"),
        "source_base": command("git", "rev-parse", "HEAD").decode().strip(),
        "sha256": {
            "cargo_lock": sha256((REPO / "Cargo.lock").read_bytes()).hexdigest(),
            "model": sha256(MODEL.read_bytes()).hexdigest(),
            "capture": sha256(Path(__file__).read_bytes()).hexdigest(),
            "executable": sha256(source).hexdigest(),
            "tape": sha256(input_bytes).hexdigest(),
            "ordered_result": sha256(result_trace).hexdigest(),
        },
        "plan": {
            "seed": SEED, "warmup_pairs": 1, "timed_pairs": 10,
            "cargo_features": ["kairo-ecs-pdes/pdes-worker-pool"], "opt_level": 3,
        },
        "runtime": {"rustc": compiler, "python": sys.version},
        "host": {"platform": platform.platform(), "logical_cpus": os.cpu_count()},
        "model_counts": counts,
        "warmup": rows[0],
        "timed": timed,
        "summary_total_ns": {
            engine: interval([r["metrics"][engine]["total_ns"] for r in timed])
            for engine in ENGINES
        },
        "summary_paired_pooled_over_serial": interval([
            r["metrics"]["pooled"]["total_ns"] / r["metrics"]["serial"]["total_ns"] for r in timed
        ]),
        "exit_code": 0,
    }
    if platform.system() == "Darwin":
        for key in ("hw.model", "machdep.cpu.brand_string"):
            receipt["host"][key] = command("sysctl", "-n", key).decode().strip()
    with receipt_path.open("x") as output:
        json.dump(receipt, output, indent=2, sort_keys=True)
        output.write("\n")
    print(json.dumps({"counts": counts, "result_sha256": receipt["sha256"]["ordered_result"]}))


if __name__ == "__main__":
    main()
