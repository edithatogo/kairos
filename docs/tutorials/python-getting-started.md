# Python Getting Started

This tutorial uses the Python-native scheduler facade under
`bindings/python/kairo_ecs/`. It does not require native wheels.

## What exists now

The Python facade currently mirrors the deterministic scheduler contract:

- `Scheduler.schedule_at(...)` and `Scheduler.schedule_after(...)` add events;
- `Scheduler.cancel(event_id)` returns `True` only for pending events;
- `Scheduler.step()` dispatches one event in scheduler order;
- `Scheduler.run_for(...)` and `Scheduler.run_until(...)` provide bounded runs;
- `Scheduler.stats()` exposes current time, pending events, and dispatched
  events.

## Minimal local session

From `bindings/python`, run the local tests when Python dependencies are
available:

```powershell
python -m pytest -q
python -m compileall kairo_ecs tests
```

The facade shape is:

```python
from kairo_ecs import Scheduler, StepOutcome

scheduler = Scheduler()
first = scheduler.schedule_at(5, priority=0, kind=1)
second = scheduler.schedule_after(10, priority=1, kind=2)

assert scheduler.cancel(second) is True
outcome, event = scheduler.step()

assert outcome is StepOutcome.DISPATCHED
assert event.id == first
assert scheduler.stats()["pending_events"] == 0
```

## Learning path

1. Read `bindings/python/README.md`.
2. Read `bindings/python/tests/test_scheduler.py` for the most current usage.
3. Compare cancellation behavior with the Rust scheduler tests.
4. Pick a model from [the model-building tutorial](model-building.md).
5. Use the model README as domain context, not as a claim that a packaged Python
   model runner is available.

## Example cross-links

- [Documentation example index](../../examples/docs/README.md)
- [Community model zoo](../community/model-zoo.md)
- [M/M/1 queue](../../examples/des/mm1_queue/README.md)
- [Emergency department flow](../../examples/hybrid/emergency_department_flow/README.md)

## Claim boundary

This page documents the local facade and tests. Native Python wheels, registry
publication, and production runtime support remain release-gated.
