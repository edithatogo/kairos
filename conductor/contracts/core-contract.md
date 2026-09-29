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

## FlowRuntime event-kind allocation

The Kairos owner approved `EventKind::Custom(4000..=4003)` for FlowRuntime's
internal events on 2026-09-30. The allocation is local to FlowRuntime:

| Code | Symbol | Meaning |
| --- | --- | --- |
| 4000 | `FLOW_COMMAND_DISPATCH_EVENT_KIND` | Dispatch a Flow command |
| 4001 | `FLOW_WORK_COMPLETION_EVENT_KIND` | Complete scheduled work |
| 4002 | `FLOW_WAITING_DEADLINE_EVENT_KIND` | Expire a waiting claim |
| 4003 | `FLOW_CONTINUATION_NOTIFICATION_EVENT_KIND` | Notify a waiting continuation |

FlowRuntime must schedule these kinds through private constructors and reject
caller-supplied use of the reserved codes at every Flow ingress, including
handler command sinks and adapters. Raw core and ABM scheduler APIs retain their
current `EventKind::Custom(u32)` contract; this allocation does not enforce
global uniqueness. The scheduler order above and `event_log.v1` encoding
`custom:<code>` remain unchanged. Deterministic ingress, ordering and telemetry
fixtures are required before the FlowRuntime implementation is accepted. Track
25 reviews the additive public API.

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
