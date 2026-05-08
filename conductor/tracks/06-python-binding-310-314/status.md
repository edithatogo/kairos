# Status — 06 Python Binding 3.10-3.14

## 2026-05-06 R2 binding slice

Status: In progress, minimal real Python binding slice implemented.

Implemented within `bindings/python/`:

- Importable `kairo_ecs` package with metadata and `self_check()`.
- Explicit `ffi_status()` boundary that reports native FFI as `not_configured` or `not_loaded` and does not implicitly load external libraries.
- Dependency-light Python scheduler facade with deterministic time, priority, and sequence ordering.
- Event value contracts for `SimTime`, `EventId`, `EntityId`, `ScheduleRequest`, `DispatchedEvent`, and `StepOutcome`.
- Dependency-light `kairo_ecs.event_log.v1` smoke-byte encoder/decoder for Arrow event-log roundtrip tests.
- Focused pytest coverage for import, scheduler, Arrow, and FFI status behavior.

Validation from `bindings/python/`:

- `python -m pytest -q` — pass, `9 passed`; local pytest cache creation warned with access denied.
- `python -m compileall kairo_ecs tests` — pass.
- `python -c "import kairo_ecs; print(kairo_ecs.self_check())"` — pass.
- `python -m pip check` — pass.
- `python -m build --sdist --wheel` — unavailable because the local interpreter does not have the `build` module installed.
- `python -m pip install --dry-run .` — blocked by local temp build-tracker permission denied before metadata resolution.

Next integration dependency:

- Track 02/15 native artifact packaging before Python should load the C ABI automatically.

## 2026-05-08 optional Arrow and package-gate review

Status: In progress, binding slice strengthened but not advanced because required local gates remain blocked.

Implemented within `bindings/python/` and `packaging/python/`:

- Optional pyarrow table roundtrip facade for the Track 04 `kairo_ecs.event_log.v1` field order.
- `kairo-ecs[arrow]` optional dependency metadata for installing `pyarrow` without making the default import path heavy.
- Packaging note documenting the current wheel-build blocker and optional Arrow dependency boundary.

Validation from `bindings/python/`:

- `python -m pytest -q` — pass with `15 passed, 1 skipped`; the skip is the optional pyarrow table roundtrip because `pyarrow` is not installed locally. The known pytest cache warning remains.
- `python -m ruff check .` — pass.
- `python -m compileall kairo_ecs tests` — pass.
- `python -c "import kairo_ecs; print(kairo_ecs.self_check())"` — pass.
- `python -m pip check` — pass.
- `python -c "import pyarrow, sys; print(pyarrow.__version__)"` — blocked by missing dependency: `ModuleNotFoundError: No module named 'pyarrow'`.
- `python -m build --sdist --wheel` — blocked by local temp venv ACL denial before package build.
- `python -m build --sdist --wheel --no-isolation` — blocked by local backend hook temp-file ACL denial, including after pointing `TEMP`/`TMP` at package-local `.tmp`.
- `powershell -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\06-python-binding-310-314\validate-bindings06-11.ps1` — pass.
- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` — pass.

Next integration dependency:

- Rerun wheel-build and real pyarrow roundtrip gates in an environment with writable temp hook directories and `pyarrow` installed.
