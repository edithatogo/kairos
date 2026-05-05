#!/usr/bin/env python3
"""Metadata-only benchmark smoke checks for Track 12."""

from __future__ import annotations

import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
REQUIRED_SCENARIOS = [
    "schedule_1m_events",
    "pop_1m_events",
    "schedule_cancel_1m_mixed",
    "create_1m_entities",
    "component_insert_1m",
    "hybrid_des_abm_smoke_100k",
]


def load_json(relative_path: str) -> dict:
    with (ROOT / relative_path).open("r", encoding="utf-8") as handle:
        return json.load(handle)


def main() -> None:
    smoke = load_json("benches/benchmark-smoke.json")
    manifest = load_json(smoke["source_manifest"])

    assert smoke["version"] == 1, "benchmark smoke metadata version must be 1"
    assert smoke["harness"] == "metadata-only", "benchmark smoke harness must stay metadata-only"
    assert smoke["requires_native_link_tests"] is False, "smoke check must not require native link tests"

    smoke_scenarios = {scenario["id"]: scenario for scenario in smoke["scenarios"]}
    manifest_scenarios = {benchmark["id"]: benchmark for benchmark in manifest["benchmarks"]}

    for scenario_id in REQUIRED_SCENARIOS:
        assert scenario_id in smoke_scenarios, f"missing smoke scenario: {scenario_id}"
        assert scenario_id in manifest_scenarios, f"missing manifest benchmark: {scenario_id}"
        smoke_scenario = smoke_scenarios[scenario_id]
        manifest_scenario = manifest_scenarios[scenario_id]
        assert smoke_scenario["scale"] > 0, f"{scenario_id} scale must be positive"
        assert 0 < smoke_scenario["smoke_scale"] <= smoke_scenario["scale"], (
            f"{scenario_id} smoke_scale must be between 1 and scale"
        )
        assert smoke_scenario["expected_owner"] == manifest_scenario["owner"], (
            f"{scenario_id} owner mismatch"
        )
        assert manifest_scenario["status"] == "canonical", f"{scenario_id} must be canonical"

    print(
        json.dumps(
            {
                "metadata": "benches/benchmark-smoke.json",
                "scenarios": REQUIRED_SCENARIOS,
                "requires_native_link_tests": False,
                "status": "ok",
            },
            indent=2,
        )
    )


if __name__ == "__main__":
    main()
