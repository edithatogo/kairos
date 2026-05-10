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

Status: Advanced to In Review after implementation closeout. The wheel/sdist gate now passes outside the sandbox; the real pyarrow table gate remains blocked by a local Windows DLL-load failure after workspace-local installation.

Implemented within `bindings/python/` and `packaging/python/`:

- Optional pyarrow table roundtrip facade for the Track 04 `kairo_ecs.event_log.v1` field order.
- `kairo-ecs[arrow]` optional dependency metadata for installing `pyarrow` without making the default import path heavy.
- Packaging note documenting the successful local package build and optional Arrow runtime boundary.
- Package metadata now uses the SPDX string license form accepted by current setuptools without the deprecated TOML-table warning.
- The optional pyarrow test now skips only when `pyarrow` is absent; a broken installed `pyarrow` fails the gate instead of being masked as a skip.

Validation from `bindings/python/`:

- `python -m pytest -q` — pass with `15 passed, 1 skipped`; the skip is the optional pyarrow table roundtrip because `pyarrow` is not installed on the default interpreter path. The known pytest cache warning remains.
- `python -m ruff check .` — pass.
- `python -m compileall kairo_ecs tests` — pass.
- `python -c "import kairo_ecs; print(kairo_ecs.self_check())"` — pass.
- `python -m pip check` — pass.
- `pwsh -NoProfile -Command '$env:TEMP=(Resolve-Path ''.tmp'').Path; $env:TMP=$env:TEMP; python -m build --sdist --wheel'` — pass outside the sandbox; built `kairo_ecs-0.1.0.tar.gz` and `kairo_ecs-0.1.0-py3-none-any.whl`.
- `pwsh -NoProfile -Command '$env:TEMP=(Resolve-Path ''.tmp'').Path; $env:TMP=$env:TEMP; python -m pip install pyarrow --target .tmp\pyarrow-site --cache-dir .tmp\pip-cache'` — pass outside the sandbox; installed `pyarrow-24.0.0`.
- `pwsh -NoProfile -Command '$env:PYTHONPATH=(Resolve-Path ''.tmp\pyarrow-site'').Path; python -m pytest -q tests\test_arrow.py::test_event_log_batch_round_trips_pyarrow_table'` — fail; `pyarrow.lib` cannot load a required DLL on this host.
- `powershell -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\06-python-binding-310-314\validate-bindings06-11.ps1` — pass after updating the validator for SPDX license metadata.
- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` — pass.

Next integration dependency:

- Resolve the local `pyarrow.lib` DLL-load failure, then rerun the real pyarrow table roundtrip gate before any Done closeout.

## 2026-05-09 Track 06 review pass

Status: Remains In Review. No code defects were found in the Track 06 owned Python surface during this pass, and no in-scope code files required changes.

Validation from `bindings/python/`:

- `python --version` — pass, Python 3.13.12.
- `python -m pytest -q` — pass with `15 passed, 1 skipped`; the skip is the optional pyarrow table roundtrip because `pyarrow` is not installed. The known pytest cache ACL warning remains.
- `python -m ruff check .` — pass.
- `python -m compileall kairo_ecs tests` — pass.
- `python -c "import kairo_ecs; print(kairo_ecs.self_check())"` — pass.
- `python -m pip check` — pass.
- `python -c "import pyarrow, sys; print(pyarrow.__version__)"` — blocked, `ModuleNotFoundError: No module named 'pyarrow'`.
- `python -m build --sdist --wheel` with `TEMP`/`TMP` pointed at package-local `.tmp` — blocked by local WinError 5 while creating the isolated build venv.
- `python -m build --sdist --wheel --no-isolation` with `TEMP`/`TMP` pointed at package-local `.tmp` — blocked by local permission denied during build-backend temp-file creation.
- `powershell -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\06-python-binding-310-314\validate-bindings06-11.ps1` — pass.
- `pwsh -NoProfile -File scripts\validate_conductor_phase_gates.ps1` — pass.
- `pwsh -NoProfile -File scripts\validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` — fail outside Track 06 because the wider working tree has unrelated uncommitted changes.

Next integration dependency:

- Track 06 is not Done-eligible until the real Arrow table gate is either executed successfully with `pyarrow` available or explicitly waived, the build-temp ACL blocker is cleared enough to rerun the build gate, and the closeout validator can run on a clean integration tree.

## 2026-05-10 Arrow roundtrip recovery

Status: Still `In Review`, but the real Arrow roundtrip is now executable on this host when the wheel is unpacked directly into a local path and that unpacked tree is placed on `PYTHONPATH`.

Validation from `bindings/python/`:

- `python -m pytest -q tests\test_arrow.py::test_event_log_batch_round_trips_pyarrow_table` with `PYTHONPATH` pointed at the unpacked wheel tree — pass.
- `python -m pytest -q` with `PYTHONPATH` pointed at the unpacked wheel tree — pass with `16 passed`; the known pytest cache warning remains.
- `python -m ruff check kairo_ecs tests` — pass.
- `python -m compileall kairo_ecs tests` — pass.
- `python -c "import kairo_ecs; print(kairo_ecs.self_check())"` — pass.
- `python -m pip check` — pass.
- `powershell -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\06-python-binding-310-314\validate-bindings06-11.ps1` — pass.

Next integration dependency:

- The remaining local cleanup is closeout hygiene: keep the worktree clean, record the phase-closeout ledger, and push the reconciled Track 06 evidence. The Arrow roundtrip and build gates are no longer blockers.
