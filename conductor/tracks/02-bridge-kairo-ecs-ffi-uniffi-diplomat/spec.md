# 02 The Bridge: kairo-ecs-ffi, UniFFI & Diplomat — spec.md

## Mission

Define and implement the stable handle-based ABI plus generated convenience surfaces for multi-language compatibility.

## Primary subagent

```text
ffi-agent + uniffi-agent + diplomat-agent
```

## Dependencies

```text
Track 01 core contract; implementation requires scheduler/ECS facade readiness.
```

## Owned paths

```text
crates/kairo-ecs-ffi, crates/kairo-ecs-uniffi, crates/kairo-ecs-diplomat, include/
```

## Parallel-safe with

Most tracks are parallel-safe after their contract inputs are accepted. See `conductor/parallel-execution.md` for the wave model.

## Inputs

- Track 01 core facade (SimTime, EventId, EntityId, EventKind, Scheduler).
- Track 12 conformance fixtures.
- `ffi-contract.md` ABI surface.

## Outputs

- Rust bridge facade in `crates/kairo-ecs-ffi`.
- UniFFI generated bindings in `crates/kairo-ecs-uniffi`.
- Diplomat generated headers.
- C header in `include/kairo_ecs.h`.

## Blocked paths

- `crates/kairo-ecs-core/` — owned by Track 01.
- `bindings/` — owned by Tracks 06–11.


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



