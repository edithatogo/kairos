#!/usr/bin/env python3
"""Track 18 benchmark reproducibility metadata checks."""

from __future__ import annotations

import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]

REQUIRED_READY_FIXTURES = {
    "scheduler_ordering_v1": "deterministic_ordering.json",
    "scheduler_cancellation_v1": "cancellation.json",
    "rng_reproducibility_v1": "rng_replay.json",
}

REQUIRED_SCENARIOS = {
    "schedule_1m_events",
    "pop_1m_events",
    "schedule_cancel_1m_mixed",
    "create_1m_entities",
    "component_insert_1m",
    "hybrid_des_abm_smoke_100k",
}


def load_json(relative_path: str) -> dict:
    with (ROOT / relative_path).open("r", encoding="utf-8") as handle:
        return json.load(handle)


def assert_nonempty_text(relative_path: str) -> None:
    path = ROOT / relative_path
    assert path.exists(), f"missing required text artifact: {relative_path}"
    assert path.read_text(encoding="utf-8").strip(), f"empty artifact: {relative_path}"


def main() -> None:
    smoke = load_json("benches/benchmark-smoke.json")
    manifest = load_json(smoke["source_manifest"])

    fixture_root = ROOT / manifest["root"]
    fixtures = {fixture["id"]: fixture for fixture in manifest["fixtures"]}
    benchmarks = {benchmark["id"]: benchmark for benchmark in manifest["benchmarks"]}
    smoke_scenarios = {scenario["id"]: scenario for scenario in smoke["scenarios"]}

    for fixture_id, source_name in REQUIRED_READY_FIXTURES.items():
        assert fixture_id in fixtures, f"missing ready fixture: {fixture_id}"
        fixture = fixtures[fixture_id]
        assert fixture["status"] == "ready", f"{fixture_id} must be ready"
        assert fixture["source"] == source_name, f"{fixture_id} source changed"
        assert (fixture_root / source_name).exists(), f"{fixture_id} source file missing"
        assert fixture["assertions"], f"{fixture_id} must record assertions"

    for scenario_id in REQUIRED_SCENARIOS:
        assert scenario_id in benchmarks, f"missing manifest benchmark: {scenario_id}"
        assert scenario_id in smoke_scenarios, f"missing smoke scenario: {scenario_id}"
        benchmark = benchmarks[scenario_id]
        scenario = smoke_scenarios[scenario_id]
        assert benchmark["status"] == "canonical", f"{scenario_id} must be canonical"
        assert scenario["expected_owner"] == benchmark["owner"], f"{scenario_id} owner mismatch"
        assert 0 < scenario["smoke_scale"] <= scenario["scale"], (
            f"{scenario_id} smoke_scale must be within scale"
        )

    assert_nonempty_text("benches/benchmark-plan.md")
    assert_nonempty_text("docs/benchmarks/benchmark-policy.md")
    assert_nonempty_text("docs/benchmarks/reproduce-comparison.md")

    print(
        json.dumps(
            {
                "status": "ok",
                "ready_fixtures": sorted(REQUIRED_READY_FIXTURES),
                "canonical_scenarios": sorted(REQUIRED_SCENARIOS),
                "source_manifest": smoke["source_manifest"],
                "smoke_metadata": "benches/benchmark-smoke.json",
            },
            indent=2,
        )
    )


if __name__ == "__main__":
    main()
