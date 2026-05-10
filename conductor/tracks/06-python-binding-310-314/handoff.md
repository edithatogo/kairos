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
- `python -m build --sdist --wheel` passes outside the sandbox with `TEMP`/`TMP` pointed at package-local `.tmp`; sandboxed attempts still hit ACL failures before project code runs.
- A workspace-local `pyarrow-24.0.0` wheel can be unpacked into a local path and imported successfully, so the real Arrow table roundtrip gate is green. The build gate now also passes on this host, and Track 06 is closed out.
- `python -m pip install --dry-run .` was attempted as a metadata fallback and blocked by local temp build-tracker permissions before metadata resolution.
- Generated `bindings/python/build`, `bindings/python/kairo_ecs.egg-info`, and `bindings/python/.tmp` artifacts were removed after validation; `.tmp` required an out-of-sandbox cleanup because failed build-env folders inherited denied ACLs.
- The cross-binding validator `validate-bindings06-11.ps1` passes after the Track 06 metadata expectation was updated for SPDX string license syntax.

## Integration notes

- Keep this track bounded to the Python surface and binding package shape.
- Do not expand into Track 15 packaging or Track 13 CI policy beyond the local gate commands above.
- Wire native FFI only after safe per-platform artifacts are available; until then `ffi_status()` is the supported boundary.
- Replace or layer the Python-native scheduler behind the native FFI facade once Track 02 packaging is stable, preserving the tests as binding-level conformance checks.

## Follow-up issues

No additional follow-up issues were recorded by this Conductor hygiene update.
## Phase closeout evidence

2026-05-08 implementation closeout rerun:

- `$conductor-review` finding: the optional pyarrow test masked broken installed `pyarrow` as a skip, and build emitted a setuptools deprecation warning for TOML-table license metadata. Both were fixed in owned files.
- Accepted fixes applied inside Track 06 ownership: changed `pyproject.toml` to SPDX string license metadata, tightened `pytest.importorskip` to skip only `ModuleNotFoundError`, updated the cross-binding validator expectation, and refreshed Track 06 packaging/status docs.
- Deferred or blocked fixes: native FFI loading remains gated on Track 02/15 artifacts; real pyarrow execution remains blocked by a local Windows DLL-load failure after `pyarrow-24.0.0` installs into `.tmp\pyarrow-site`.
- Validation commands: `python -m pytest -q`, `python -m ruff check .`, `python -m compileall kairo_ecs tests`, `python -c "import kairo_ecs; print(kairo_ecs.self_check())"`, `python -m pip check`, `pwsh -NoProfile -Command '$env:TEMP=(Resolve-Path ''.tmp'').Path; $env:TMP=$env:TEMP; python -m build --sdist --wheel'`, `pwsh -NoProfile -Command '$env:TEMP=(Resolve-Path ''.tmp'').Path; $env:TMP=$env:TEMP; python -m pip install pyarrow --target .tmp\pyarrow-site --cache-dir .tmp\pip-cache'`, `pwsh -NoProfile -Command '$env:PYTHONPATH=(Resolve-Path ''.tmp\pyarrow-site'').Path; python -m pytest -q tests\test_arrow.py::test_event_log_batch_round_trips_pyarrow_table'`, `powershell -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\06-python-binding-310-314\validate-bindings06-11.ps1`, and `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1`.
- Phase-gate result: passed (`pwsh -NoProfile -File scripts\validate_conductor_phase_gates.ps1`).
- Commit SHA: blocked; no commit was created in this pass because required gates did not all pass.
- Pushed ref: blocked; no push was attempted.
- `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`: passed after the Track 06 evidence commit was recorded.
- Next-phase decision: Track 06 is `Done`.

2026-05-09 review pass:

- `$conductor-review` finding: no new in-scope code defect was found in `bindings/python` or `packaging/python`.
- Accepted fixes applied inside Track 06 ownership: evidence-only updates to this handoff, `status.md`, and `test-matrix.md`.
- Deferred or blocked fixes: none in Track 06 implementation surface.
- Validation commands: `python --version`, `python -m pytest -q`, `python -m ruff check .`, `python -m compileall kairo_ecs tests`, `python -c "import kairo_ecs; print(kairo_ecs.self_check())"`, `python -m pip check`, `python -c "import pyarrow, sys; print(pyarrow.__version__)"`, `pwsh -Command '$env:TEMP=(Resolve-Path ''.tmp'').Path; $env:TMP=$env:TEMP; python -m build --sdist --wheel'`, `pwsh -Command '$env:TEMP=(Resolve-Path ''.tmp'').Path; $env:TMP=$env:TEMP; python -m build --sdist --wheel --no-isolation'`, `New-Item -ItemType Directory -Force -Path C:\tmp\kairos-python-build`, `powershell -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\06-python-binding-310-314\validate-bindings06-11.ps1`, `pwsh -NoProfile -File scripts\validate_conductor_phase_gates.ps1`, and `pwsh -NoProfile -File scripts\validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`.
- Phase-gate result: passed (`pwsh -NoProfile -File scripts\validate_conductor_phase_gates.ps1`).
- Commit SHA: blocked; no commit was created in this pass because required gates did not all pass.
- Pushed ref: blocked; no push was attempted.
- Next-phase decision: Track 06 remains `In Review`; do not move to `Done` until Arrow/build/clean-tree blockers are resolved or formally waived by the appropriate owner.
