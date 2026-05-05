# 06 Python Binding 3.10-3.14 — spec.md

## Mission

Provide idiomatic Python package with wheels, pyarrow integration, conformance tests, and Python 3.10-3.14 support.

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
bindings/python, packaging/python
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

- Python package with wheels for CPython 3.10-3.14.
- pyarrow integration for telemetry reading.
- Conformance test runner that executes Track 12 shared fixtures.
- Package dry-run for PyPI publication.


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



