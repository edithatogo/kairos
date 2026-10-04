#!/usr/bin/env python3
"""Bounded SimPy 4.1.2 differential for Q5.1 shared resource semantics.

This module's offline comparator uses only the standard library. SimPy is imported
only when the actual reference driver runs. AllOf is a completion join, not an
atomic multi-resource acquisition primitive; this tool makes no such claim.
"""
from __future__ import annotations

import argparse
import hashlib
import importlib.metadata
import json
import os
from pathlib import Path
import platform
import subprocess
import sys

FIXTURE = "queue_simpy_shared_v1"
FIXTURE_VERSION = 1
SIMPY_VERSION = "4.1.2"
KAIROS_VERSION = "0.1.0"

# Fixed, versioned inputs mirrored by the Rust example. Request times are distinct
# within equal-priority groups so engine-private equal-time ordering is excluded.
CASES = [
    {"id": "capacity_1", "capacity": 1, "claims": [
        {"name": "a", "at": 0, "duration": 4, "priority": 0},
        {"name": "b", "at": 1, "duration": 2, "priority": 0},
        {"name": "c", "at": 2, "duration": 1, "priority": 0},
    ]},
    {"id": "capacity_2", "capacity": 2, "claims": [
        {"name": "a", "at": 0, "duration": 4, "priority": 0},
        {"name": "b", "at": 1, "duration": 4, "priority": 0},
        {"name": "c", "at": 2, "duration": 2, "priority": 0},
    ]},
    {"id": "priority_fifo", "capacity": 1, "claims": [
        {"name": "holder", "at": 0, "duration": 4, "priority": 9},
        {"name": "a", "at": 1, "duration": 2, "priority": 5},
        {"name": "b", "at": 2, "duration": 1, "priority": 1},
        {"name": "c", "at": 3, "duration": 1, "priority": 1},
    ]},
    {"id": "queued_cancel", "capacity": 1, "claims": [
        {"name": "holder", "at": 0, "duration": 5, "priority": 0},
        {"name": "cancelled", "at": 1, "duration": 2, "priority": 0, "cancel_at": 2},
        {"name": "survivor", "at": 3, "duration": 2, "priority": 0},
    ]},
    {"id": "manual_release", "capacity": 1, "claims": [
        {"name": "holder", "at": 0, "duration": 2, "priority": 0},
        {"name": "waiter", "at": 1, "duration": 2, "priority": 0},
    ]},
]


def _row(case: str, at: int, op: str, request: str) -> dict[str, object]:
    return {"case": case, "at": at, "op": op, "request": request}


EXPECTED_EVENTS = [
    _row("capacity_1", 0, "grant", "a"),
    _row("capacity_1", 4, "release", "a"),
    _row("capacity_1", 4, "grant", "b"),
    _row("capacity_1", 6, "release", "b"),
    _row("capacity_1", 6, "grant", "c"),
    _row("capacity_1", 7, "release", "c"),
    _row("capacity_2", 0, "grant", "a"),
    _row("capacity_2", 1, "grant", "b"),
    _row("capacity_2", 4, "release", "a"),
    _row("capacity_2", 4, "grant", "c"),
    _row("capacity_2", 5, "release", "b"),
    _row("capacity_2", 6, "release", "c"),
    _row("priority_fifo", 0, "grant", "holder"),
    _row("priority_fifo", 4, "release", "holder"),
    _row("priority_fifo", 4, "grant", "b"),
    _row("priority_fifo", 5, "release", "b"),
    _row("priority_fifo", 5, "grant", "c"),
    _row("priority_fifo", 6, "release", "c"),
    _row("priority_fifo", 6, "grant", "a"),
    _row("priority_fifo", 8, "release", "a"),
    _row("queued_cancel", 0, "grant", "holder"),
    _row("queued_cancel", 2, "cancel", "cancelled"),
    _row("queued_cancel", 5, "release", "holder"),
    _row("queued_cancel", 5, "grant", "survivor"),
    _row("queued_cancel", 7, "release", "survivor"),
    _row("manual_release", 0, "grant", "holder"),
    _row("manual_release", 2, "release", "holder"),
    _row("manual_release", 2, "grant", "waiter"),
    _row("manual_release", 4, "release", "waiter"),
]


def canonical_bytes(value: object) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True).encode("ascii")


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def validate_trace(trace: object, *, engine: str, version: str,
                   expected_events: list[dict[str, object]] = EXPECTED_EVENTS) -> list[dict[str, object]]:
    if not isinstance(trace, dict):
        raise ValueError("trace must be a JSON object")
    if (trace.get("fixture") != FIXTURE or type(trace.get("version")) is not int
            or trace["version"] != FIXTURE_VERSION):
        raise ValueError("fixture identity/version mismatch")
    engine_version = trace.get("engine_version")
    if (trace.get("engine") != engine or not isinstance(engine_version, str)
            or not engine_version or engine_version != version):
        raise ValueError(f"wrong engine or version; expected {engine} {version}")
    events = trace.get("events")
    if not isinstance(events, list):
        raise ValueError("events must be a list")
    for event in events:
        if not isinstance(event, dict) or set(event) != {"case", "at", "op", "request"}:
            raise ValueError("event row missing or has unexpected fields")
        if (not isinstance(event["at"], int) or isinstance(event["at"], bool)
                or event["op"] not in {"grant", "release", "cancel"}
                or not isinstance(event["case"], str)
                or not isinstance(event["request"], str)):
            raise ValueError("event row has invalid types or operation")
    if events != expected_events:
        raise ValueError("event rows differ from the hand-derived expected trace")
    return events


def compare_traces(kairos_trace: object, simpy_trace: object,
                   simpy_version: str = SIMPY_VERSION) -> dict[str, object]:
    if simpy_version != SIMPY_VERSION:
        raise ValueError(f"SimPy version must be exactly {SIMPY_VERSION}")
    kairos = validate_trace(kairos_trace, engine="kairos", version=KAIROS_VERSION)
    simpy = validate_trace(simpy_trace, engine="simpy", version=SIMPY_VERSION)
    if kairos != simpy:
        raise ValueError("Kairos and SimPy normalized rows differ")
    return {"events": EXPECTED_EVENTS, "events_sha256": sha256(canonical_bytes(EXPECTED_EVENTS))}


def run_simpy() -> tuple[str, list[dict[str, object]]]:
    installed = importlib.metadata.version("simpy")
    if installed != SIMPY_VERSION:
        raise RuntimeError(f"expected SimPy {SIMPY_VERSION}, found {installed}")
    import simpy
    if simpy.__version__ != SIMPY_VERSION:
        raise RuntimeError(f"SimPy module version mismatch: {simpy.__version__}")

    rows: list[dict[str, object]] = []
    for case in CASES:
        env = simpy.Environment()
        resource = simpy.PriorityResource(env, capacity=case["capacity"])

        def requester(claim: dict[str, object]):
            arrival = int(claim["at"])
            if arrival:
                yield env.timeout(arrival)
            request = resource.request(priority=int(claim["priority"]))
            cancel_at = claim.get("cancel_at")
            if cancel_at is not None:
                yield env.timeout(int(cancel_at) - arrival)
                if not request.triggered:
                    request.cancel()
                    rows.append(_row(str(case["id"]), int(env.now), "cancel", str(claim["name"])))
                    return
            yield request
            rows.append(_row(str(case["id"]), int(env.now), "grant", str(claim["name"])))
            yield env.timeout(int(claim["duration"]))
            resource.release(request)
            rows.append(_row(str(case["id"]), int(env.now), "release", str(claim["name"])))

        for claim in case["claims"]:
            env.process(requester(claim))
        env.run()
    return installed, rows


def _git_head(repo_root: Path) -> str:
    result = subprocess.run(["git", "rev-parse", "HEAD"], cwd=repo_root,
                            check=True, capture_output=True, text=True)
    return result.stdout.strip()


def _git_dirty(repo_root: Path) -> bool:
    result = subprocess.run(["git", "status", "--porcelain"], cwd=repo_root,
                            check=True, capture_output=True, text=True)
    return bool(result.stdout.strip())


def run_cli(rust_trace_path: Path, output_path: Path) -> dict[str, object]:
    rust_bytes = rust_trace_path.read_bytes()
    rust_trace = json.loads(rust_bytes)
    simpy_version, simpy_events = run_simpy()
    simpy_trace = {"fixture": FIXTURE, "version": FIXTURE_VERSION,
                   "engine": "simpy", "engine_version": simpy_version,
                   "events": simpy_events}
    checked = compare_traces(rust_trace, simpy_trace, simpy_version)
    repo_root = Path(__file__).resolve().parents[2]
    rust_source = repo_root / "crates/kairo-ecs-des/examples/flow_simpy_shared_v1.rs"
    python_source = Path(__file__).resolve()
    result = {
        "fixture": FIXTURE,
        "version": FIXTURE_VERSION,
        "matched_hand_derived_rows": True,
        "case_ids": [case["id"] for case in CASES],
        "comparison_checkout_commit": _git_head(repo_root),
        "comparison_checkout_dirty": _git_dirty(repo_root),
        "simpy_version": simpy_version,
        "python": {"executable": sys.executable, "version": sys.version.split()[0],
                   "platform": platform.platform(), "cwd": os.getcwd(), "argv": sys.argv},
        "input_sha256": sha256(canonical_bytes({"fixture": FIXTURE,
                                                 "version": FIXTURE_VERSION,
                                                 "cases": CASES})),
        "python_driver_sha256": sha256(python_source.read_bytes()),
        "rust_example_sha256": sha256(rust_source.read_bytes()),
        "rust_artifact_sha256": sha256(rust_bytes),
        "kairos_events_sha256": sha256(canonical_bytes(rust_trace["events"])),
        "simpy_events_sha256": sha256(canonical_bytes(simpy_events)),
        "expected_events_sha256": checked["events_sha256"],
        "events": checked["events"],
    }
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(json.dumps(result, sort_keys=True, indent=2) + "\n", encoding="utf-8")
    return result


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=(
        "Run fixed shared queue cases against SimPy 4.1.2 and compare with a Rust trace. "
        "AllOf is a join, not atomic acquisition; ties, preemption and scheduler cancellation "
        "stay with Kairos local oracles."
    ))
    parser.add_argument("--rust-trace", required=True, type=Path,
                        help="JSON artifact emitted by flow_simpy_shared_v1")
    parser.add_argument("--output", required=True, type=Path,
                        help="comparison/provenance JSON output path")
    args = parser.parse_args(argv)
    result = run_cli(args.rust_trace, args.output)
    print(json.dumps({"matched_hand_derived_rows": result["matched_hand_derived_rows"],
                      "output": str(args.output),
                      "expected_events_sha256": result["expected_events_sha256"]},
                     sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
