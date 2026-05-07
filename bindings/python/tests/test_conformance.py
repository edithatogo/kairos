from __future__ import annotations

import json
from pathlib import Path

import kairo_ecs

FIXTURES = Path(__file__).resolve().parents[3] / "conformance" / "fixtures"


def _load_fixture(name: str) -> dict[str, object]:
    with (FIXTURES / name).open(encoding="utf-8") as handle:
        return json.load(handle)


def _dispatch_all(scheduler: kairo_ecs.Scheduler) -> list[int]:
    kinds: list[int] = []
    while True:
        outcome, event = scheduler.step()
        if outcome is kairo_ecs.StepOutcome.EMPTY:
            return kinds
        assert outcome is kairo_ecs.StepOutcome.DISPATCHED
        assert event is not None
        kinds.append(event.kind)


def test_deterministic_ordering():
    fixture = _load_fixture("deterministic_ordering.json")
    scheduler = kairo_ecs.Scheduler()

    for event in fixture["events"]:
        assert isinstance(event, dict)
        scheduler.schedule_at(
            int(event["at_ticks"]),
            priority=int(event["priority"]),
            kind=int(event["kind"]),
        )

    assert fixture["version"] == 1
    assert _dispatch_all(scheduler) == fixture["expected_kind_order"]


def test_cancellation():
    fixture = _load_fixture("cancellation.json")
    scheduler = kairo_ecs.Scheduler()

    for event in fixture["events"]:
        assert isinstance(event, dict)
        event_id = scheduler.schedule_at(
            int(event["at_ticks"]),
            priority=int(event["priority"]),
            kind=int(event["kind"]),
        )
        if event.get("cancel") is True:
            assert scheduler.cancel(event_id) is True

    assert _dispatch_all(scheduler) == fixture["expected_kind_order"]


def test_zero_delay_guard():
    fixture = _load_fixture("zero_delay_guard.json")
    scheduler = kairo_ecs.Scheduler()

    for event in fixture["events"]:
        assert isinstance(event, dict)
        scheduler.schedule_at(
            int(event["at_ticks"]),
            priority=int(event["priority"]),
            kind=int(event["kind"]),
        )

    assert _dispatch_all(scheduler) == fixture["expected_kind_order"]


def test_rng_replay():
    fixture = _load_fixture("rng_replay.json")

    assert fixture["run_seed"] == 7
    assert len(fixture["expected_stream"]) == 4
