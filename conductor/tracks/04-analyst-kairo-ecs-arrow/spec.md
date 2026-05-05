# 04 The Analyst: kairo-ecs-arrow Telemetry — spec.md

## Mission

Implement Arrow schemas, telemetry collection modes, IPC export, and cross-language analytics handoffs.

## Primary subagent

```text
arrow-agent
```

## Dependencies

```text
Track 01 type contract; can design schema in parallel.
```

## Owned paths

```text
crates/kairo-ecs-arrow, schemas/arrow, examples/telemetry/
```

## Parallel-safe with

Most tracks are parallel-safe after their contract inputs are accepted. See `conductor/parallel-execution.md` for the wave model.

## Inputs

- Track 01 event-log contract.
- Track 12 conformance fixtures.
- Track 26 standards review.
- `arrow-schema-contract.md`.

## Outputs

- Arrow IPC/Parquet serialization in `crates/kairo-ecs-arrow`.
- Schema definitions in `schemas/arrow/`.
- Telemetry examples in `examples/telemetry/`.

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


