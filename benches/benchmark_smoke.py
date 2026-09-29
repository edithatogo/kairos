#!/usr/bin/env python3
"""Metadata-only benchmark smoke checks for Track 12."""

import json
import re
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
PLAN_PATH = "benches/benchmark-plan.md"
REQUIRED_SCENARIOS = [
    "schedule_1m_events",
    "pop_1m_events",
    "schedule_cancel_1m_mixed",
    "create_1m_entities",
    "component_insert_1m",
    "hybrid_des_abm_smoke_100k",
]
PLAN_ROW_RE = re.compile(
    r"^\| `(?P<id>[^`]+)` \| `(?P<owner>[^`]+)` \| `(?P<smoke_scale>\d+)` \| "
    r"`(?P<scale>[\d_]+)` \| (?P<measure>[^|]+?) \| (?P<behavior>[^|]+?) \|$"
)


def load_json(relative_path: str) -> dict:
    with (ROOT / relative_path).open("r", encoding="utf-8") as handle:
        return json.load(handle)


def load_text(relative_path: str) -> str:
    with (ROOT / relative_path).open("r", encoding="utf-8") as handle:
        return handle.read()


def parse_plan_rows(plan_text: str) -> dict[str, dict[str, object]]:
    rows: dict[str, dict[str, object]] = {}
    for raw_line in plan_text.splitlines():
        match = PLAN_ROW_RE.match(raw_line.strip())
        if not match:
            continue
        row = match.groupdict()
        row["smoke_scale"] = int(row["smoke_scale"])
        row["scale"] = int(row["scale"].replace("_", ""))
        row["measure"] = row["measure"].strip()
        row["behavior"] = row["behavior"].strip()
        rows[row["id"]] = row
    return rows


def main() -> None:
    smoke = load_json("benches/benchmark-smoke.json")
    plan = load_text(smoke["benchmark_plan"])
    manifest = load_json(smoke["source_manifest"])
    plan_rows = parse_plan_rows(plan)

    assert smoke["version"] == 1, "benchmark smoke metadata version must be 1"
    assert smoke["harness"] == "metadata-only", "benchmark smoke harness must stay metadata-only"
    assert smoke["requires_native_link_tests"] is False, "smoke check must not require native link tests"
    assert smoke["benchmark_plan"] == PLAN_PATH, "smoke metadata must point at the canonical benchmark plan"
    assert "metadata-only" in plan, "benchmark plan must describe the metadata-only smoke contract"
    assert "requires_native_link_tests = false" in plan, (
        "benchmark plan must describe the native-link smoke contract"
    )

    smoke_scenarios = {scenario["id"]: scenario for scenario in smoke["scenarios"]}
    manifest_scenarios = {benchmark["id"]: benchmark for benchmark in manifest["benchmarks"]}

    for scenario_id in REQUIRED_SCENARIOS:
        assert scenario_id in plan_rows, f"missing benchmark-plan scenario row: {scenario_id}"
        assert scenario_id in smoke_scenarios, f"missing smoke scenario: {scenario_id}"
        assert scenario_id in manifest_scenarios, f"missing manifest benchmark: {scenario_id}"
        plan_scenario = plan_rows[scenario_id]
        smoke_scenario = smoke_scenarios[scenario_id]
        manifest_scenario = manifest_scenarios[scenario_id]
        assert plan_scenario["owner"] == smoke_scenario["expected_owner"], f"{scenario_id} owner mismatch"
        assert plan_scenario["owner"] == manifest_scenario["owner"], f"{scenario_id} owner mismatch"
        assert plan_scenario["smoke_scale"] == smoke_scenario["smoke_scale"], (
            f"{scenario_id} smoke_scale mismatch"
        )
        assert plan_scenario["smoke_scale"] <= plan_scenario["scale"], (
            f"{scenario_id} smoke_scale must be between 1 and scale"
        )
        assert smoke_scenario["scale"] == plan_scenario["scale"], f"{scenario_id} scale mismatch"
        assert smoke_scenario["scale"] > 0, f"{scenario_id} scale must be positive"
        assert 0 < smoke_scenario["smoke_scale"] <= smoke_scenario["scale"], (
            f"{scenario_id} smoke_scale must be between 1 and scale"
        )
        assert manifest_scenario["status"] == "canonical", f"{scenario_id} must be canonical"
        assert manifest_scenario["measure"] == plan_scenario["measure"], f"{scenario_id} measure mismatch"

    print(
        json.dumps(
            {
                "metadata": "benches/benchmark-smoke.json",
                "benchmark_plan": smoke["benchmark_plan"],
                "scenarios": REQUIRED_SCENARIOS,
                "requires_native_link_tests": False,
                "status": "ok",
            },
            indent=2,
        )
    )


if __name__ == "__main__":
    main()
