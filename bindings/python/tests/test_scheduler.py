from __future__ import annotations

import kairo_ecs


def test_scheduler_dispatches_by_time_priority_then_sequence() -> None:
    scheduler = kairo_ecs.Scheduler()
    scheduler.schedule_at(10, priority=2, kind=3)
    scheduler.schedule_at(5, priority=9, kind=1)
    scheduler.schedule_at(10, priority=1, kind=2)
    scheduler.schedule_at(10, priority=1, kind=4)

    kinds: list[int] = []
    while True:
        outcome, event = scheduler.step()
        if outcome is kairo_ecs.StepOutcome.EMPTY:
            break
        assert event is not None
        kinds.append(event.kind)

    assert kinds == [1, 2, 4, 3]
    assert scheduler.stats() == {
        "current_time_ticks": 10,
        "scheduled_events": 4,
        "pending_events": 0,
        "dispatched_events": 4,
        "cancelled_events": 0,
    }


def test_scheduler_cancellation_skips_event_without_reordering_rest() -> None:
    scheduler = kairo_ecs.Scheduler()
    scheduler.schedule_at(1, kind=1)
    cancelled = scheduler.schedule_at(2, kind=2)
    scheduler.schedule_at(3, kind=3)

    assert scheduler.cancel(cancelled) is True
    assert scheduler.pending_events == 2

    scheduler.run_for(10)

    assert [event.kind for event in scheduler.trace] == [1, 3]


def test_scheduler_rejects_unknown_duplicate_and_dispatched_cancellation() -> None:
    scheduler = kairo_ecs.Scheduler()
    dispatched = scheduler.schedule_at(1, kind=1)
    cancelled = scheduler.schedule_at(2, kind=2)
    unknown = kairo_ecs.EventId(999, 0)

    assert scheduler.cancel(unknown) is False
    assert scheduler.cancel(cancelled) is True
    assert scheduler.cancel(cancelled) is False

    outcome, event = scheduler.step()

    assert outcome is kairo_ecs.StepOutcome.DISPATCHED
    assert event is not None
    assert event.id == dispatched
    assert scheduler.cancel(dispatched) is False
    assert scheduler.pending_events == 0


def test_run_until_reports_limit_when_future_event_remains() -> None:
    scheduler = kairo_ecs.Scheduler()
    scheduler.schedule_at(1, kind=1)
    scheduler.schedule_at(4, kind=2)

    outcome, event = scheduler.run_until(1)

    assert outcome is kairo_ecs.StepOutcome.DISPATCHED
    assert event is not None
    assert event.kind == 1
    assert scheduler.pending_events == 1


def test_scheduler_stats_track_scheduled_cancelled_and_dispatched() -> None:
    scheduler = kairo_ecs.Scheduler()
    first = scheduler.schedule_at(1, kind=1)
    second = scheduler.schedule_at(2, kind=2)

    assert scheduler.stats() == {
        "current_time_ticks": 0,
        "scheduled_events": 2,
        "pending_events": 2,
        "dispatched_events": 0,
        "cancelled_events": 0,
    }

    assert scheduler.cancel(second) is True
    assert scheduler.cancel(second) is False
    outcome, event = scheduler.step()

    assert outcome is kairo_ecs.StepOutcome.DISPATCHED
    assert event is not None
    assert event.id == first
    assert scheduler.stats() == {
        "current_time_ticks": 1,
        "scheduled_events": 2,
        "pending_events": 0,
        "dispatched_events": 1,
        "cancelled_events": 1,
    }
