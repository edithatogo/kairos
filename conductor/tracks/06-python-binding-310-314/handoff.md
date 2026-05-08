# Handoff — 06 Python Binding 3.10-3.14

Last updated: 2026-05-08

## Summary

Python binding now has a minimal real R2 slice that is importable on the local interpreter without native build dependencies. The slice exposes package metadata, explicit native-FFI status, a Python-native scheduler facade, executable conformance-fixture bridge checks, event value contracts, scheduler stats, a dependency-light Arrow event-log smoke roundtrip, and an optional pyarrow table roundtrip facade behind the `arrow` extra. Native FFI loading remains explicitly not configured until Track 02/15 provide safe packaged native artifacts.

## Files changed

- `bindings/python/kairo_ecs/__init__.py`
- `bindings/python/kairo_ecs/_arrow.py`
- `bindings/python/kairo_ecs/_ffi.py`
- `bindings/python/kairo_ecs/_scheduler.py`
- `bindings/python/kairo_ecs/_types.py`
- `bindings/python/tests/test_arrow.py`
- `bindings/python/tests/test_ffi.py`
- `bindings/python/tests/test_import.py`
- `bindings/python/tests/test_scheduler.py`
- `bindings/python/tests/test_conformance.py`
- `conductor/tracks/06-python-binding-310-314/handoff.md`
- `conductor/tracks/06-python-binding-310-314/status.md`
- `conductor/tracks/06-python-binding-310-314/test-matrix.md`
- `bindings/python/pyproject.toml`
- `packaging/python/README.md`

## Contracts consumed

- Track 01 core types and scheduler contracts: fixed ticks, event handles, scheduler ordering by time, priority, then sequence.
- Track 02 FFI contract: stable C ABI names and the requirement that native handle lifecycle remains explicit.
- Track 04 Arrow schema contract: `kairo_ecs.event_log.v1` field names, order, status values, and fixed-size little-endian handle/tick encodings.
- Track 12 conformance fixture contracts: scheduler ordering and cancellation expectations from the ready bootstrap fixtures.

## Contracts changed

- No cross-track contract changes.
- Python adapter entrypoints added under the package surface: scheduler facade, event value types, event-log smoke roundtrip, and explicit `ffi_status()`.

## Tests added

- `bindings/python/tests/test_import.py` now verifies package identity, `self_check()`, and FFI-not-configured status.
- `bindings/python/tests/test_scheduler.py` verifies deterministic scheduler ordering, cancellation, bounded run behavior, and stats.
- `bindings/python/tests/test_scheduler.py` now also verifies unknown, duplicate, and already-dispatched cancellation rejection.
- `bindings/python/tests/test_scheduler.py` now verifies scheduled, pending, dispatched, and cancelled scheduler stats plus Track 01-aligned `run_until` behavior.
- `bindings/python/tests/test_conformance.py` now drives the Python scheduler through the deterministic-ordering, cancellation, and zero-delay guard fixtures instead of only reading fixture metadata.
- `bindings/python/tests/test_arrow.py` verifies event-log v1 field order and smoke-byte roundtrip.
- `bindings/python/tests/test_arrow.py` now also verifies pyarrow table roundtrip when `pyarrow` is installed; it skips cleanly on the current interpreter where `pyarrow` is missing.
- `bindings/python/tests/test_ffi.py` verifies native library paths are not loaded implicitly.
- `conductor/tracks/06-python-binding-310-314/validate-bindings06-11.ps1` guards the deterministic facade and metadata boundaries for binding Tracks 06-11 without requiring native artifacts or unavailable R/Julia runtimes.

## Known risks

- Python version skew across 3.10 through 3.14.
- Native-extension or wheel build drift if package metadata lands before the API stabilizes.
- Cross-language expectations creeping in before the shared fixture contract is finished.
- Current validation ran on the locally available interpreter only; the full 3.10-3.14 matrix remains a CI responsibility.
- `python -m build --sdist --wheel` could not complete locally because isolated venv creation and build-hook temp-file writes are denied under the available temp paths.
- `pyarrow` is not installed in the local interpreter, so the real Arrow table roundtrip gate is implemented but locally skipped until `kairo-ecs[arrow]` or `pyarrow` is available.
- `python -m pip install --dry-run .` was attempted as a metadata fallback and blocked by local temp build-tracker permissions before metadata resolution.
- The failed build attempts left `bindings/python/.tmp/pybuild` with access-denied subdirectories; remove it from a shell with sufficient permissions if it appears in local cleanup.
- The cross-binding validator `validate-bindings06-11.ps1` currently fails on Go Track 11 static expectations; this pass did not modify `bindings/go` because Track 06 owns only `bindings/python`.

## Integration notes

- Keep this track bounded to the Python surface and binding package shape.
- Do not expand into Track 15 packaging or Track 13 CI policy beyond the local gate commands above.
- Wire native FFI only after safe per-platform artifacts are available; until then `ffi_status()` is the supported boundary.
- Replace or layer the Python-native scheduler behind the native FFI facade once Track 02 packaging is stable, preserving the tests as binding-level conformance checks.

## Follow-up issues

No additional follow-up issues were recorded by this Conductor hygiene update.
## Phase closeout evidence

2026-05-08 review pass:

- `$conductor-review` finding: no in-scope code defect found in the preview scheduler or optional Arrow facade. Track 06 cannot advance because the wheel-build gate is blocked by local temp-directory ACL failures and the real pyarrow roundtrip gate cannot execute until `pyarrow` is installed.
- Accepted fixes applied inside Track 06 ownership: added optional `EventLogBatch.to_pyarrow_table()` / `from_pyarrow_table()`, an optional pytest roundtrip, `kairo-ecs[arrow]` extra metadata, and packaging notes.
- Deferred or blocked fixes: native FFI loading remains gated on Track 02/15 artifacts; isolated and non-isolated wheel builds remain locally blocked by filesystem permissions; pyarrow execution remains dependency-blocked.
- Validation commands: `python -m pytest -q`, `python -m ruff check .`, `python -m compileall kairo_ecs tests`, `python -c "import kairo_ecs; print(kairo_ecs.self_check())"`, `python -m pip check`, `python -c "import pyarrow, sys; print(pyarrow.__version__)"`, `python -m build --sdist --wheel`, `python -m build --sdist --wheel --no-isolation`, a retry with `TEMP`/`TMP` pointed at package-local `.tmp`, `powershell -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\06-python-binding-310-314\validate-bindings06-11.ps1`, and `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1`.
- Phase-gate result: passed after Track 06 closeout markers were made explicit.
- Commit SHA: blocked; no commit was created in this pass because required gates did not all pass.
- Pushed ref: blocked; no push was attempted.
- `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`: not run because the workspace contains unrelated worker edits and Track 06 remains blocked.
- Next-phase decision: keep Track 06 `In Progress` until the wheel-build gate and real Arrow table gate can be rerun in an environment with writable temp hooks and `pyarrow`.
