# Agent Contract — 01 The Heart: kairo-ecs-core & kairo-ecs-state

## Owner

This track is executed through the lane owners in `lanes.md`:

- 01A Types and time: `contracts-agent`
- 01B Scheduler: `core-scheduler-agent`
- 01C State: `ecs-agent`
- 01D RNG: `rng-agent`
- 01E Facade readiness: `core-scheduler-agent` + `ffi-agent`

## Owned paths

```text
crates/kairo-ecs-types, crates/kairo-ecs-core, crates/kairo-ecs-state, crates/kairo-ecs-rng
```

## Current implementation slice

Track 01 currently owns the deterministic heart slice that is already present in the workspace:

- `kairo-ecs-types` defines the time, ID, request, and outcome types that the scheduler and state crates consume.
- `kairo-ecs-core` implements the deterministic single-threaded scheduler with time/priority/sequence ordering, cancellation, bounded runs, scheduler stats, and replay-recording support.
- `kairo-ecs-core` also exposes a pure Rust `SchedulerFacade` with stable `CoreStatus`, `ScheduleStatus`, and `StepStatus` wrappers for Track 01E facade-readiness handoff.
- `kairo-ecs-state` implements the entity world, deterministic snapshot ordering, and generational component storage behavior.
- `kairo-ecs-rng` implements run-seed derivation and deterministic entity streams.

## Handoff rules

- Do not change public contracts without an ADR.
- Keep implementation changes inside the owned paths above unless a lane dependency is explicitly called out in `handoff.md`.
- Keep the docs aligned with `lanes.md`, `plan.md`, `spec.md`, and the validation surface in `test-matrix.md`.
- Add tests or test-plan updates before requesting integration.
- Update docs for user-visible behavior and for any change to lane ownership or validation gates.
