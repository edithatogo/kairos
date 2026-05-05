# 08 Julia Binding — spec.md

## Mission

Provide Julia package using ccall, Artifacts, Arrow.jl, Documenter.jl docs, and conformance tests.

## Primary subagent

```text
julia-agent
```

## Dependencies

```text
Track 02 FFI RC and Track 04 Arrow schema RC.
```

## Owned paths

```text
bindings/julia
```

Packaging, registry, and release dry-run work is explicitly out of scope for
this binding slice.

## Parallel-safe with

Most tracks are parallel-safe after their contract inputs are accepted. See `conductor/parallel-execution.md` for the wave model.

## Inputs

- Stable C ABI from Track 02 FFI RC.
- Arrow schema from Track 04.
- Conformance fixtures from Track 12.
- Julia >= 1.10 LTS.

## Outputs

- Julia package in `bindings/julia/`.
- Arrow.jl integration.
- Artifacts.toml for native library loading.
- Conformance test runner.

## Blocked paths

- `crates/` — owned by Tracks 01–05.
- `bindings/` (except `bindings/julia/`) — owned by other binding tracks.


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



