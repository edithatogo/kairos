"""Fixture metadata checks only. These do not execute a simulator or RNG.

Native execution tests live in native_tests/ and require a real Rust library.
RNG replay and full dispatched-record conformance remain explicit native ABI
qualification work; inspecting fixture values is not acceptance of those methods.
"""

from __future__ import annotations

import json
from pathlib import Path

FIXTURES = Path(__file__).resolve().parents[3] / "conformance" / "fixtures"


def test_scheduler_fixture_metadata_is_available() -> None:
    for name in ("deterministic_ordering.json", "cancellation.json", "zero_delay_guard.json"):
        fixture = json.loads((FIXTURES / name).read_text(encoding="utf-8"))
        assert fixture["version"] == 1
        assert fixture["events"]
        assert fixture["expected_kind_order"]


def test_rng_fixture_metadata_is_available() -> None:
    fixture = json.loads((FIXTURES / "rng_replay.json").read_text(encoding="utf-8"))
    assert fixture["expected_stream"]
