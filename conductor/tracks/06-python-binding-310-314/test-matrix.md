# Test Matrix — 06 Python Binding 3.10-3.14

## Required tests

- `python -m pytest -q` against the Python binding surface in `bindings/python/`.
- `python -m compileall kairo_ecs tests` to catch syntax drift across the supported interpreter range.
- `python -m pip check` after build to catch packaging metadata issues.
- `python -m build --sdist --wheel` against the package metadata in `bindings/python/pyproject.toml`.
- `python -c "import kairo_ecs; print(kairo_ecs.self_check())"` as the smoke import check.

## Current R2 binding-slice coverage

- Import/package metadata: `kairo_ecs.__version__`, public exports, and `self_check()`.
- Native FFI boundary: `ffi_status()` returns `not_configured` by default and `not_loaded` when `KAIRO_ECS_FFI_LIBRARY` is set; this slice does not implicitly load native libraries.
- Scheduler facade: pure Python fixed-tick scheduler with `schedule_at`, `schedule_after`, `cancel`, `step`, `run_for`, `run_until`, `stats`, and dispatch trace.
- Cancellation contract: `cancel()` now rejects unknown, duplicate, and already-dispatched event IDs so stale cancellation handles cannot distort pending counts.
- Scheduler stats contract: `stats()` reports current time plus scheduled, pending, dispatched, and cancelled event counts.
- Event contracts: `SimTime`, `EventId`, `EntityId`, `ScheduleRequest`, `DispatchedEvent`, and `StepOutcome`.
- Arrow smoke facade: `kairo_ecs.event_log.v1` field order plus dependency-light `EventLogRecord`/`EventLogBatch` smoke-byte roundtrip.
- Focused tests: import/self-check, scheduler ordering, cancellation, bounded run, executable fixture bridge checks, Arrow schema/roundtrip, and explicit FFI-not-configured behavior.

## Validation evidence — 2026-05-06

Run from `bindings/python/`:

| Command | Result | Notes |
|---|---:|---|
| `python -m pytest -q` | Pass | `9 passed`; pytest emitted a cache write warning because `.pytest_cache` creation was denied locally. |
| `python -m compileall kairo_ecs tests` | Pass | Syntax check passed for package and tests. |
| `python -c "import kairo_ecs; print(kairo_ecs.self_check())"` | Pass | Returned package/version/status and FFI `not_configured`. |
| `python -m pip check` | Pass | `No broken requirements found.` |
| `python -m build --sdist --wheel` | Not run | Current interpreter does not have the `build` module installed: `No module named build`. |
| `python -m pip install --dry-run .` | Blocked | Pip failed before package metadata resolution with temp build-tracker permission denied under `AppData\Local\Temp`. |

## Validation evidence — 2026-05-06 cancellation hardening

Run from `bindings/python/` unless otherwise stated:

| Command | Result | Notes |
|---|---:|---|
| `python -m pytest -q` | Pass | `10 passed`; pytest emitted a cache write warning because `.pytest_cache` creation was denied locally. |
| `python -m compileall kairo_ecs tests` | Pass | Syntax check passed for package and tests. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\06-python-binding-310-314\validate-bindings06-11.ps1` | Pass | Cross-binding static facade and metadata guard for Tracks 06-11. |

## Future-surface controls

- Do not add PyPI publishing, release automation, or registry credentials here.
- Do not widen beyond the Python binding surface into core scheduler, conformance, or release-engineering work.
- Do not add language-bridge APIs for Julia, R, TypeScript, C#, or Go in this track.
- Stop at binding-level verification until Track 12 owns shared fixture parity and Track 15 owns packaging dry-runs.

## Validation evidence — 2026-05-07 fixture bridge and stats alignment

Run from `bindings/python/` unless otherwise stated:

| Command | Result | Notes |
|---|---:|---|
| `python -m pytest -q` | Pass | `15 passed`; pytest emitted a cache write warning because `.pytest_cache` creation was denied locally. |
| `python -m compileall kairo_ecs tests` | Pass | Syntax check passed for package and tests. |
| `python -c "import kairo_ecs; print(kairo_ecs.self_check())"` | Pass | Returned package/version/status and FFI `not_configured`. |
| `python -m pip check` | Pass | `No broken requirements found.` |
| `python -m pip install build` | Pass | Installed `build` and `pyproject_hooks` in the local interpreter so the build gate could be attempted. |
| `python -m build --sdist --wheel` | Blocked | Isolated build reached venv creation but failed on local temp directory permissions. |
| `python -m build --sdist --wheel --no-isolation` | Blocked | Build hook temp-file creation failed with local permission denied, including after pointing `TEMP`/`TMP` at package-local `.tmp`. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\06-python-binding-310-314\validate-bindings06-11.ps1` | Fail outside Track 06 | Validator currently fails on Go Track 11 expectations in `bindings/go/kairoecs.go` and `bindings/go/kairoecs_test.go`; no Go files were changed in this Track 06 pass. |

## CI command

```bash
python -m pytest -q && python -m compileall kairo_ecs tests && python -c "import kairo_ecs; print(kairo_ecs.self_check())"
```
## Phase closeout gate

- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` must pass before any phase advances; this enforces `$conductor-review`, auto-apply of accepted fixes, cleaned commit/push, and blocker recording.