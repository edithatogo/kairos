# Handoff — 07 R Binding

## Summary

R binding work stays inside `bindings/r/` and now exposes a concrete package surface with exported helpers, a testthat smoke suite, local check commands, and a minimal deterministic scheduler/event-log facade. Native FFI is explicitly reported as not configured until Track 02 provides a safely loadable stable C ABI for the R package.

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
- Smoke tests for base-R event-log roundtrip schema normalization.
- Base-R smoke script for environments without `testthat`.
- Local package validation via `testthat::test_dir()` and `R CMD check`.

## Validation status — 2026-05-06

- `Get-Command R -ErrorAction SilentlyContinue`: `R` resolves to a PowerShell alias (`Invoke-History`), not an R executable.
- `Get-Command Rscript -ErrorAction SilentlyContinue`: no `Rscript` executable found on `PATH`.
- `R CMD check --no-manual .`: not run because R is not available on this machine's `PATH`.
- Static fallback checks were used to verify package shape, export/definition alignment, and Rd alias coverage.

## Known risks

- Package metadata drift between local validation and future registry-ready packaging.
- Cross-language fixture mismatches if the shared contract changes after the R wrapper lands.
- Overreach into publishing or registry automation before Track 15 owns it.
- Full R CMD check remains blocked until R >= 4.2 and `Rscript` are available on `PATH`.

## Integration notes

- Keep the implementation at the R boundary until the shared conformance and packaging tracks are complete.
- Do not add CRAN release automation or cross-language adapters here.
- Replace the pure-R scheduler facade with stable-C-ABI calls only after Track 02 exposes a verified native library path and ownership/finalizer rules for R external pointers.
- No release, registry, or remote publication side effects were performed.

