# 03 The Flow: DES Trajectory API & ABM Behavior API — spec.md

## Mission

Create equal first-class modeling surfaces: DES trajectory/process API and ABM behavior/decision API over the same ECS/event kernel.

## Primary subagent

```text
des-api-agent + abm-api-agent
```

## Dependencies

```text
Track 01 contracts; implementation depends on scheduler/ECS ports.
```

## Owned paths

```text
crates/kairo-ecs-des, crates/kairo-ecs-abm, examples/flow/
```

## Parallel-safe with

Most tracks are parallel-safe after their contract inputs are accepted. See `conductor/parallel-execution.md` for the wave model.

## Inputs

- Track 01 ECS contract (EntityId, ComponentTypeId, System pattern).
- Track 26 standards review (DEVS mappings).

## Outputs

- DES trajectory API in `crates/kairo-ecs-des`.
- ABM behavior API in `crates/kairo-ecs-abm`.
- Flow examples in `examples/flow/`.

## Blocked paths

- `crates/kairo-ecs-core/` — owned by Track 01.
- `crates/kairo-ecs-ffi/` — owned by Track 02.


## Acceptance criteria

- Owned paths are created and documented.
- Contract inputs and outputs are explicit.
- Track tests or validation checks exist.
- CI gate is defined.
- Documentation impact is recorded.
- Release implications are recorded.
- `handoff.md` is completed before merge.


## Quality gates

Use the gates in `conductor/quality-gates.md`. Track-specific gates must be listed in `test-matrix.md`.

## Release implications

This track contributes to release readiness only through the acceptance criteria and quality gates listed here and in conductor/quality-gates.md. It does not independently authorize public release, registry publication, or production-readiness claims without the dependent packaging, supply-chain, compatibility, red-team, and wave-management gates.
