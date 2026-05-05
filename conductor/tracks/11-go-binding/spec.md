# 11 Go Binding — spec.md

## Mission

Provide Go module using cgo over the stable C ABI with explicit handle lifecycle, tests, and Go examples.

## Primary subagent

```text
go-agent
```

## Dependencies

```text
Track 02 FFI RC and Track 04 Arrow schema RC.
```

## Owned paths

```text
bindings/go, packaging/go
```

## Parallel-safe with

Most tracks are parallel-safe after their contract inputs are accepted. See `conductor/parallel-execution.md` for the wave model.

## Inputs

- Stable C ABI from Track 02 FFI RC.
- Arrow schema from Track 04.
- Conformance fixtures from Track 12.
- Go >= 1.22.

## Outputs

- Go module in `bindings/go/`.
- cgo bindings.
- Apache Arrow Go integration.
- Conformance test runner.

## Blocked paths

- `crates/` — owned by Tracks 01–05.
- `bindings/` (except `bindings/go/`) — owned by other binding tracks.


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



