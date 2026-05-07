# Handoff — 07 R Binding

## Summary

R binding work stays inside `bindings/r/` and now exposes a concrete package surface with exported helpers, a testthat smoke suite, local check commands, and a minimal deterministic scheduler/event-log facade. Native FFI is explicitly reported as not configured until Track 02 provides a safely loadable stable C ABI for the R package.

The current slice also bridges ready scheduler conformance fixtures into the R facade: deterministic ordering, cancellation, and zero-delay ordering fixtures are loaded from `conformance/fixtures` and executed through the exported scheduler helpers. RNG replay remains metadata-only until an R RNG facade exists.

## Files changed

`bindings/r/DESCRIPTION`
`bindings/r/LICENSE`
`bindings/r/NAMESPACE`
`bindings/r/R/kairoecs.R`
`bindings/r/man/kairoECS-package.Rd`
`bindings/r/man/kairoecs-scheduler.Rd`
`bindings/r/tests/testthat.R`
`bindings/r/tests/helper-load.R`
`bindings/r/tests/smoke-base.R`
`bindings/r/tests/testthat/helper-load.R`
`bindings/r/tests/testthat/test-conformance.R`
`bindings/r/tests/testthat/test-smoke.R`
`bindings/r/README.md`
`conductor/tracks/07-r-binding/test-matrix.md`
`conductor/tracks/07-r-binding/handoff.md`

## Contracts consumed

- Track 01 core type contracts.
- Track 02 FFI contract for the not-configured native status boundary.
- Track 04 Arrow schema contract for `kairo_ecs.event_log.v1` field order.
- Track 12 conformance fixture contracts.
- Track 14 docs workflow only if the R surface adds package docs.

## Contracts changed

- R wrapper signatures and package metadata only.

## Tests added

- Smoke tests for exported R helpers.
- Smoke tests for explicit native FFI not-configured status.
- Smoke tests for deterministic scheduler/event ordering.
- Smoke tests for rejecting unknown, duplicate, and already-dispatched cancellation attempts.
- Smoke tests for base-R event-log roundtrip schema normalization.
- Testthat fixture bridge checks for `deterministic_ordering.json`, `cancellation.json`, and `zero_delay_guard.json`.
- Metadata-only coverage for `rng_replay.json` pending an R RNG facade.
- Base-R smoke script for environments without `testthat`.
- Local package validation via `testthat::test_dir()` and `R CMD check`.
- Cross-binding static validation via `conductor/tracks/06-python-binding-310-314/validate-bindings06-11.ps1`.

## Validation status — 2026-05-07

- `Get-Command R -ErrorAction SilentlyContinue`: `R` resolves to a PowerShell alias (`Invoke-History`), not an R executable.
- `Get-Command Rscript`: resolves to `C:\Users\60217257\scoop\shims\rscript.exe`.
- `Rscript --version`: `Rscript (R) version 4.6.0 (2026-04-24)`.
- Installed focused test dependencies: `jsonlite` and `testthat`.
- `Rscript tests\smoke-base.R` from `bindings/r/`: pass.
- `Rscript -e "testthat::test_dir('tests', reporter = 'summary')"` from `bindings/r/`: pass.
- `Rcmd check --no-manual r` from `bindings/` with `_R_CHECK_FORCE_SUGGESTS_=false`: completes with one NOTE that checking should be performed on sources prepared by `R CMD build`.
- Validation commands used `LC_ALL=C`, `LC_CTYPE=C`, and `LANG=C` to avoid Windows R startup warnings from inherited `C.UTF-8` locale variables.

## Known risks

- Package metadata drift between local validation and future registry-ready packaging.
- Cross-language fixture mismatches if the shared contract changes after the R wrapper lands.
- Overreach into publishing or registry automation before Track 15 owns it.
- Optional packages `arrow`, `devtools`, `lintr`, and `pkgdown` are not installed locally; complete force-suggests checking remains future validation.

## Integration notes

- Keep the implementation at the R boundary until the shared conformance and packaging tracks are complete.
- Do not add CRAN release automation or cross-language adapters here.
- Replace the pure-R scheduler facade with stable-C-ABI calls only after Track 02 exposes a verified native library path and ownership/finalizer rules for R external pointers.
- No release, registry, or remote publication side effects were performed.

## Follow-up issues

No additional follow-up issues were recorded by this Conductor hygiene update.
## Phase closeout evidence

Pending for the next actual phase closeout. Before this track advances, record `$conductor-review` findings, accepted fixes, deferred or blocked fixes, validation commands, cleanup state, commit SHA or explicit push blocker, pushed ref, strict `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` result, and next-phase decision here.