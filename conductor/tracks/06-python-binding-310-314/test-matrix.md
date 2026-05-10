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
- Optional Arrow table facade: `EventLogBatch.to_pyarrow_table()` and `EventLogBatch.from_pyarrow_table()` roundtrip the Track 04 field order when `pyarrow` is installed through the `arrow` extra.
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

## Validation evidence — 2026-05-08 optional Arrow and package gate review

Run from `bindings/python/` unless otherwise stated:

| Command | Result | Notes |
|---|---:|---|
| `python -m pytest -q` | Pass with skip | `15 passed, 1 skipped`; the skip is the optional pyarrow table roundtrip because `pyarrow` is not installed locally. Pytest still emitted the known cache write warning. |
| `python -m ruff check .` | Pass | All checks passed. |
| `python -m compileall kairo_ecs tests` | Pass | Syntax check passed for package and tests. |
| `python -c "import kairo_ecs; print(kairo_ecs.self_check())"` | Pass | Returned package/version/status and FFI `not_configured`. |
| `python -m pip check` | Pass | `No broken requirements found.` |
| `python -c "import pyarrow, sys; print(pyarrow.__version__)"` | Blocked | `ModuleNotFoundError: No module named 'pyarrow'`; real Arrow table gate is implemented but cannot execute on this interpreter until `kairo-ecs[arrow]` or `pyarrow` is installed. |
| `python -m build --sdist --wheel` | Blocked | Isolated build fails before package build due local temp venv ACL denial under `AppData\Local\Temp`. |
| `python -m build --sdist --wheel --no-isolation` | Blocked | Backend hook temp-file creation fails with permission denied. Retrying with `TEMP`/`TMP` pointed at package-local `.tmp` fails the same way; the generated `.tmp` directory was removed afterward. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\06-python-binding-310-314\validate-bindings06-11.ps1` | Pass | Cross-binding static facade and metadata guard passed; native runtime loading remains intentionally out of scope. |
| `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` | Pass | Passed after Track 06 closeout markers were made explicit. |

## Validation evidence — 2026-05-08 implementation closeout rerun

Run from `bindings/python/` unless otherwise stated:

| Command | Result | Notes |
|---|---:|---|
| `python -m pytest -q` | Pass with skip | `15 passed, 1 skipped`; the skip is the optional pyarrow table roundtrip because `pyarrow` is not installed on the default interpreter path. Pytest still emitted the known cache write warning. |
| `python -m ruff check .` | Pass | All checks passed. |
| `python -m compileall kairo_ecs tests` | Pass | Syntax check passed for package and tests. |
| `python -c "import kairo_ecs; print(kairo_ecs.self_check())"` | Pass | Returned package/version/status and FFI `not_configured`. |
| `python -m pip check` | Pass | `No broken requirements found.` |
| `pwsh -NoProfile -Command '$env:TEMP=(Resolve-Path ''.tmp'').Path; $env:TMP=$env:TEMP; python -m build --sdist --wheel'` | Pass outside sandbox | Built `kairo_ecs-0.1.0.tar.gz` and `kairo_ecs-0.1.0-py3-none-any.whl`; using a package-local temp directory removed the previous sandbox ACL blocker. |
| `pwsh -NoProfile -Command '$env:TEMP=(Resolve-Path ''.tmp'').Path; $env:TMP=$env:TEMP; python -m pip install pyarrow --target .tmp\pyarrow-site --cache-dir .tmp\pip-cache'` | Pass outside sandbox | Installed `pyarrow-24.0.0` into a workspace-local target. |
| `pwsh -NoProfile -Command '$env:PYTHONPATH=(Resolve-Path ''.tmp\pyarrow-site'').Path; python -m pytest -q tests\test_arrow.py::test_event_log_batch_round_trips_pyarrow_table'` | Blocked | The stricter optional gate now fails because `pyarrow.lib` cannot load a required DLL on this host. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\06-python-binding-310-314\validate-bindings06-11.ps1` | Pass | Cross-binding static facade and metadata guard passed after updating the Python license expectation to SPDX string form. |
| `pwsh -NoProfile -File scripts\validate_conductor_phase_gates.ps1` | Pass | Conductor phase gate validation passed with 0 errors and 0 warnings. |

## Remaining Done blocker

- Real pyarrow table roundtrip is implemented and no longer silently skipped when `pyarrow` is broken, but it cannot pass on this host until the missing Windows DLL/runtime dependency for `pyarrow.lib` is resolved.

## Validation evidence — 2026-05-09 review pass

Run from `bindings/python/` unless otherwise stated:

| Command | Result | Notes |
|---|---:|---|
| `python --version` | Pass | Python 3.13.12. |
| `python -m pytest -q` | Pass with skip | `15 passed, 1 skipped`; the skip is the optional pyarrow table roundtrip because `pyarrow` is not installed. Pytest emitted the known local cache ACL warning. |
| `python -m ruff check .` | Pass | All checks passed. |
| `python -m compileall kairo_ecs tests` | Pass | Syntax check passed for package and tests. |
| `python -c "import kairo_ecs; print(kairo_ecs.self_check())"` | Pass | Returned package/version/status and FFI `not_configured`. |
| `python -m pip check` | Pass | `No broken requirements found.` |
| `python -c "import pyarrow, sys; print(pyarrow.__version__)"` | Blocked | `ModuleNotFoundError: No module named 'pyarrow'`; no dependency install was attempted in this review pass. |
| `pwsh -Command '$env:TEMP=(Resolve-Path ''.tmp'').Path; $env:TMP=$env:TEMP; python -m build --sdist --wheel'` | Blocked | Isolated build failed before package code ran: local venv creation could not create `.tmp\build-env-*\Include` due WinError 5. |
| `pwsh -Command '$env:TEMP=(Resolve-Path ''.tmp'').Path; $env:TMP=$env:TEMP; python -m build --sdist --wheel --no-isolation'` | Blocked | Build backend temp-file creation failed under `.tmp\tmp*` with permission denied. |
| `New-Item -ItemType Directory -Force -Path C:\tmp\kairos-python-build` | Blocked | `C:\tmp` build-temp fallback was denied on this host. |
| `powershell -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\06-python-binding-310-314\validate-bindings06-11.ps1` | Pass | Cross-binding static facade and metadata guard passed. |
| `pwsh -NoProfile -File scripts\validate_conductor_phase_gates.ps1` | Pass | Conductor phase gate validation passed with 0 errors and 0 warnings. |
| `pwsh -NoProfile -File scripts\validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` | Fail outside Track 06 | Closeout validator failed because the wider working tree has unrelated uncommitted tracked or untracked changes. Track 06 owned paths were clean before this evidence update. |

## CI command

```bash
python -m pytest -q && python -m compileall kairo_ecs tests && python -c "import kairo_ecs; print(kairo_ecs.self_check())" && python -m build --sdist --wheel
```
## Phase closeout gate

- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` and `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1` must pass before any phase advances; this enforces `$conductor-review`, auto-apply of accepted fixes, phase-closeout ledger evidence, cleaned commit/push evidence, and blocker recording. At actual closeout, run `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` after commit and push.
