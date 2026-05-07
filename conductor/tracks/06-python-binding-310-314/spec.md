# 06 Python Binding 3.10-3.14 — spec.md

## Mission

Provide an idiomatic Python package surface for Python 3.10-3.14. The current implementation slice is dependency-light and importable without native artifacts; wheels, pyarrow integration, and full conformance-runner packaging remain follow-up work after Track 02/15 artifacts are stable.

## Primary subagent

```text
python-agent
```

## Dependencies

```text
Track 02 FFI RC and Track 04 Arrow schema RC.
```

## Owned paths

```text
bindings/python
```

## Blocked paths

```text
crates/ — owned by Tracks 01-05 (core implementation)
bindings/r/, bindings/julia/, bindings/typescript/, bindings/csharp/, bindings/go/ — owned by other binding tracks
include/ — owned by Track 02 (C headers)
```

## Parallel-safe with

Tracks 07-11 (other language bindings) — all bindings are parallel-safe after FFI RC.

## Inputs

- Stable C ABI from Track 02 FFI RC.
- Arrow schema from Track 04.
- Conformance fixtures from Track 12.
- Compatibility policy from Track 25.

## Outputs

- Importable Python package surface under `bindings/python`.
- Explicit native-FFI status boundary until packaged native artifacts are available.
- Dependency-light scheduler and event-log smoke coverage aligned with Tracks 01, 02, and 04.
- Future outputs: wheels for CPython 3.10-3.14, pyarrow telemetry integration, Track 12 conformance runner, and package dry-run for PyPI publication.


## Python version matrix

Required support:

```text
Python 3.10
Python 3.11
Python 3.12
Python 3.13
Python 3.14
```

Preferred wheel strategy:

```text
PyO3 abi3-py310 if feasible, otherwise per-version wheels.
```

Quality gates:

```bash
pytest
ruff check
python -m build
maturin build or cibuildwheel
python -c "import kairo_ecs; kairo_ecs.self_check()"
```



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
