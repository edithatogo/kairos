# 05 The Window: kairo-ecs-viz Visualization — spec.md

## Mission

Provide optional real-time visualization using WGPU/Bevy without polluting the headless core.

## Primary subagent

```text
viz-agent
```

## Dependencies

```text
ECS snapshot contract; can start UI architecture early.
```

## Owned paths

```text
crates/kairo-ecs-viz, examples/viz, website/docs/visualization/
```

## Parallel-safe with

Most tracks are parallel-safe after their contract inputs are accepted. See `conductor/parallel-execution.md` for the wave model.

## Inputs

- Track 01 ECS snapshot contract.
- Visualization rendering framework selection.

## Outputs

- Viz crate in `crates/kairo-ecs-viz`.
- Visualization examples in `examples/viz/`.
- Docs in `website/docs/visualization/`.

## Blocked paths

- `crates/kairo-ecs-core/` — owned by Track 01.


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


