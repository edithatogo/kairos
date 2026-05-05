# Core Contract: kairo-ecs-types, kairo-ecs-core, kairo-ecs-state

## Stable concepts

```text
SimTime: fixed tick-based virtual time, nanosecond precision mode supported.
SimDuration: non-negative fixed duration.
EventId: generational opaque event handle.
EntityId: generational opaque entity handle.
ComponentTypeId: stable internal component identifier.
EventKind: scheduler-visible event class.
Priority: deterministic signed/unsigned priority value.
Sequence: monotonic insertion sequence for stable ordering.
```

## Event ordering

Events dispatch by:

```text
(time_ticks ASC, priority ASC, sequence ASC)
```

This ordering is required by all language bindings and conformance fixtures.

## Run-loop controls

```text
step()
run_for(max_events)
run_until(time_limit)
run_until_or_for(time_limit, max_events)
```

Unbounded run loops must be explicitly named and guarded.

## ECS contract

A DES process, ABM agent, resource, queue, machine, vehicle, person, cell, or visualization object is an entity with components.

```text
Entity = stable handle
Component = typed data column/storage
System = event-triggered logic operating over components
```

## Safety rules

1. No raw `f64` for event ordering.
2. No host-language object inside hot event queue.
3. No `unsafe` in `kairo-ecs-core` or `kairo-ecs-state` unless approved by ADR.
4. All public behavior must be covered by deterministic fixture tests.
