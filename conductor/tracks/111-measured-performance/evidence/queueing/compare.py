#!/usr/bin/env python3
"""Independent SimPy oracle and paired Track 111 queueing evidence runner."""

import argparse
from datetime import datetime, timezone
from hashlib import sha256
from importlib.metadata import distribution
import json
import os
from pathlib import Path
import platform
import random
import statistics
import subprocess
import sys
from time import perf_counter_ns
from zipfile import ZipFile

import simpy


EVIDENCE = Path(__file__).resolve().parent
REPO = EVIDENCE.parents[4]
TAPE = EVIDENCE / "tape.tsv"
SOURCE = REPO / "crates/kairo-ecs-pdes/examples/track111_queueing.rs"
SEED = 11_120_261_125
SIMPY_SHA256 = "43071f84b6512c9b4fcb33ef057f240ccb1d1f3b263f9b4f9229d072e310b372"
ENGINES = ("serial", "pdes", "simpy")
PHASES = ("setup_ns", "dispatch_ns", "total_ns")


def digest(data):
    return sha256(data).hexdigest()


def frozen_tape():
    rng = random.Random(SEED)
    ticks = [0, 0]
    rows = []
    for _ in range(512):
        for station in range(2):
            ticks[station] += rng.randrange(4)
            rows.append((station, ticks[station], rng.randrange(1, 8), rng.randrange(2, 10)))
    return "".join("\t".join(map(str, row)) + "\n" for row in rows).encode()


def check_runtime(wheel):
    if (sys.implementation.name, sys.version_info[:3], Path(sys.prefix)) != (
        "cpython", (3, 15, 2), REPO / ".venv"
    ):
        raise ValueError("select the repository's isolated CPython 3.15.2")
    simpy_dist = distribution("simpy")
    if simpy_dist.version != "4.1.2" or digest(wheel.read_bytes()) != SIMPY_SHA256:
        raise ValueError("use the exact SimPy 4.1.2 wheel")
    if Path(simpy.__file__).resolve() != simpy_dist.locate_file("simpy/__init__.py").resolve():
        raise ValueError("unexpected SimPy import path")
    with ZipFile(wheel) as archive:
        for path in archive.namelist():
            if path.endswith(("/", "/RECORD")):
                continue
            if archive.read(path) != simpy_dist.locate_file(path).read_bytes():
                raise ValueError(f"installed SimPy differs from the wheel at {path}")


def read_tape():
    stations = [[], []]
    for row in TAPE.read_text().splitlines():
        station, arrival, service, patience = map(int, row.split("\t"))
        stations[station].append((arrival, service, patience))
    return stations


class Station:
    def __init__(self, env, arrivals):
        self.env = env
        self.arrivals = arrivals
        self.server = simpy.Resource(env, capacity=1)
        self.requests = {}
        self.customers = [["new", None, None, False] for _ in arrivals]
        self.trace = []
        self.ignored = 0
        self.removed = 0
        for customer, (tick, _, _) in enumerate(arrivals):
            self.schedule(tick, 2, self.arrive, customer)

    def schedule(self, tick, phase, action, *args):
        self.env.timeout(3 * tick + phase - self.env.now).callbacks.append(
            lambda _: action(tick, *args)
        )

    def state(self):
        active = next((self.requests[request] for request in self.server.users), None)
        waiting = [self.requests[request] for request in self.server.queue]
        return active, waiting

    def record(self, tick, label, customer):
        self.trace.append([tick, label, customer, *self.state()])

    def arrive(self, tick, customer):
        self.record(tick, "arrive", customer)
        request = self.server.request()
        self.requests[request] = customer
        if request.triggered:
            self.start(tick, request)
            return
        self.customers[customer][0] = "waiting"
        self.record(tick, "queue", customer)
        request.callbacks.append(lambda grant: self.start(self.env.now // 3, grant))
        self.schedule(tick + self.arrivals[customer][2], 1, self.deadline, customer, request)

    def start(self, tick, request):
        customer = self.requests[request]
        record = self.customers[customer]
        record[:] = ["serving", tick, None, record[0] == "waiting"]
        self.record(tick, "start", customer)
        self.schedule(tick + self.arrivals[customer][1], 0, self.finish, request)

    def finish(self, tick, request):
        customer = self.requests.pop(request)
        self.server.release(request)
        record = self.customers[customer]
        record[0], record[2] = "finished", tick
        self.record(tick, "finish", customer)

    def deadline(self, tick, customer, request):
        record = self.customers[customer]
        if request.triggered:
            self.ignored += 1
            return
        request.cancel()
        if request in self.server.queue:
            raise ValueError("SimPy did not remove an abandoned request")
        del self.requests[request]
        self.removed += 1
        record[0], record[2] = "abandoned", tick
        self.record(tick, "abandon", customer)

    def result(self):
        active, waiting = self.state()
        if self.requests or active is not None or waiting:
            raise ValueError("SimPy did not drain the station")
        return {"trace": self.trace, "customers": self.customers, "active": active, "waiting": waiting}


def run_simpy():
    tape = read_tape()
    start = perf_counter_ns()
    env = simpy.Environment()
    stations = [Station(env, arrivals) for arrivals in tape]
    setup = perf_counter_ns() - start
    env.run()
    total = perf_counter_ns() - start
    return {
        "result": [station.result() for station in stations],
        "metrics": {"setup_ns": setup, "dispatch_ns": total - setup, "total_ns": total},
        "diagnostics": {
            "cancelled_requests": sum(station.removed for station in stations),
            "ignored_timers": sum(station.ignored for station in stations),
        },
    }


def capture(command):
    return subprocess.check_output(command, cwd=REPO)


def matching_result(outputs):
    reference = outputs["simpy"]["result"]
    for name in ("serial", "pdes"):
        if outputs[name]["result"] != reference:
            raise ValueError(f"{name} disagrees with SimPy's ordered trace or final state")
    tape = read_tape()
    states = [customer for station in reference for customer in station["customers"]]
    traces = [row for station in reference for row in station["trace"]]
    counts = {
        "arrivals": len(states),
        "trace_rows": len(traces),
        "finished": sum(state[0] == "finished" for state in states),
        "abandoned": sum(state[0] == "abandoned" for state in states),
        "cancelled_patience_timers": sum(state[3] for state in states),
        "max_waiting": max(len(row[4]) for row in traces),
        "tied_arrivals": sum(
            current[0] == previous[0] for arrivals in tape
            for previous, current in zip(arrivals, arrivals[1:])
        ),
        "grants_at_queued_deadline": sum(
            state[3] and state[1] == arrival[0] + arrival[2]
            for station, arrivals in zip(reference, tape, strict=True)
            for state, arrival in zip(station["customers"], arrivals, strict=True)
        ),
    }
    live = counts["arrivals"] + counts["finished"] + counts["abandoned"]
    cancelled = counts["cancelled_patience_timers"]
    if len(states) != sum(map(len, tape)) or live != 2 * len(states) or not all(counts.values()):
        raise ValueError(f"incomplete queue workload or missing event case: {counts}")
    expected = {
        "serial": {"scheduled": live + cancelled, "dispatched": live, "cancelled_timers": cancelled},
        "pdes": {
            "dispatched": live + cancelled, "remote_events": 0, "worker_count": 2,
            "ignored_timers": cancelled, "pooled": True,
        },
        "simpy": {"cancelled_requests": counts["abandoned"], "ignored_timers": cancelled},
    }
    for name, counters in expected.items():
        if outputs[name]["diagnostics"] != counters:
            raise ValueError(f"{name} lifecycle counters differ from the model trace")
    return {"counts": counts, "sha256": digest(json.dumps(reference, separators=(",", ":")).encode())}


def summarize(rows):
    def interval(values):
        rng = random.Random(SEED)
        resampled = [statistics.median(rng.choices(values, k=len(values))) for _ in range(10_000)]
        quantiles = statistics.quantiles(resampled, n=40, method="inclusive")
        return {"median": statistics.median(values), "bootstrap_95": [quantiles[0], quantiles[-1]]}

    return {
        name: {phase: interval([row["metrics"][name][phase] for row in rows]) for phase in PHASES}
        for name in ENGINES
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("wheel", type=Path)
    parser.add_argument("receipt", type=Path)
    args = parser.parse_args()
    check_runtime(args.wheel)
    input_bytes = TAPE.read_bytes()
    if input_bytes != frozen_tape():
        raise ValueError("input tape differs from the frozen seed")
    commands = {
        "serial": [args.binary.resolve(), "serial", TAPE],
        "pdes": [args.binary.resolve(), "pdes", TAPE],
        "simpy": [sys.executable, "-I", Path(__file__).resolve(), "--simpy-child"],
    }
    rows = []
    for trial in range(11):
        offset = trial % len(ENGINES)
        order = ENGINES[offset:] + ENGINES[:offset]
        outputs = {name: json.loads(capture(commands[name])) for name in order}
        result = matching_result(outputs)
        if rows and result != rows[0]["result"]:
            raise ValueError("the same tape produced different ordered state on replay")
        rows.append({
            "trial": trial,
            "order": order,
            "result": result,
            "metrics": {name: outputs[name]["metrics"] for name in order},
            "diagnostics": {name: outputs[name]["diagnostics"] for name in order},
        })
    receipt = {
        "utc": datetime.now(timezone.utc).isoformat(timespec="seconds"),
        "source_base": capture(["git", "rev-parse", "HEAD"]).decode().strip(),
        "sha256": {
            "cargo_lock": digest((REPO / "Cargo.lock").read_bytes()),
            "rust_example": digest(SOURCE.read_bytes()),
            "oracle": digest(Path(__file__).read_bytes()),
            "executable": digest(args.binary.read_bytes()),
            "tape": digest(input_bytes),
            "simpy_wheel": SIMPY_SHA256,
        },
        "protocol": {
            "seed": SEED,
            "warmup_triplets": 1,
            "timed_triplets": 10,
            "units": "ns",
            "target_profile": "release",
            "cargo_features": ["pdes-worker-pool"],
        },
        "machine": {"platform": platform.platform(), "cpu_count": os.cpu_count()},
        "runtime": {
            "rustc": capture(["rustup", "run", "1.99.1", "rustc", "-Vv"]).decode().strip(),
            "python": sys.version,
            "simpy": distribution("simpy").version,
        },
        "warmup": rows[0],
        "samples": rows[1:],
        "summary": summarize(rows[1:]),
        "exit_code": 0,
    }
    if platform.system() == "Darwin":
        for key in ("hw.model", "machdep.cpu.brand_string", "hw.perflevel0.physicalcpu",
                    "hw.perflevel1.physicalcpu"):
            receipt["machine"][key] = capture(["sysctl", "-n", key]).decode().strip()
    with args.receipt.open("x") as file:
        json.dump(receipt, file, indent=2, sort_keys=True)
        file.write("\n")
    print(json.dumps(rows[0]["result"], sort_keys=True))


if __name__ == "__main__":
    if sys.argv[1:] == ["--simpy-child"]:
        print(json.dumps(run_simpy(), separators=(",", ":")))
    else:
        main()
