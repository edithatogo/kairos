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
