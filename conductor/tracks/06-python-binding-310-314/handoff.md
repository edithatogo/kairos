# Handoff — 06 Python Binding 3.10-3.14

Last updated: 2026-05-07

## Summary

Python binding now has a minimal real R2 slice that is importable on the local interpreter without native build dependencies. The slice exposes package metadata, explicit native-FFI status, a Python-native scheduler facade, executable conformance-fixture bridge checks, event value contracts, scheduler stats, and a dependency-light Arrow event-log smoke roundtrip. Native FFI loading remains explicitly not configured until Track 02/15 provide safe packaged native artifacts.

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
- `bindings/python/tests/test_ffi.py` verifies native library paths are not loaded implicitly.
- `conductor/tracks/06-python-binding-310-314/validate-bindings06-11.ps1` guards the deterministic facade and metadata boundaries for binding Tracks 06-11 without requiring native artifacts or unavailable R/Julia runtimes.

## Known risks

- Python version skew across 3.10 through 3.14.
- Native-extension or wheel build drift if package metadata lands before the API stabilizes.
- Cross-language expectations creeping in before the shared fixture contract is finished.
- Current validation ran on the locally available interpreter only; the full 3.10-3.14 matrix remains a CI responsibility.
- `python -m build --sdist --wheel` could not complete locally because isolated venv creation and build-hook temp-file writes are denied under the available temp paths.
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

Pending for the next actual phase closeout. Before this track advances, record `$conductor-review` findings, accepted fixes, deferred or blocked fixes, validation commands, cleanup state, commit SHA or explicit push blocker, and next-phase decision here.