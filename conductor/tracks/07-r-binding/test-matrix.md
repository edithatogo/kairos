# Test Matrix — 07 R Binding

## Required tests

- `Rscript -e "testthat::test_dir('tests', reporter = 'summary')"` from `bindings/r/`.
- `Rscript tests/smoke-base.R` from `bindings/r/` for base-R smoke coverage without `testthat`.
- `R CMD check --no-manual .` from `bindings/r/`.
- `Rscript -e "devtools::check(document = FALSE)"` once `devtools` is available in the local toolchain.
- `R CMD build .` when artifact validation is needed from `bindings/r/`.
- `Rscript -e "testthat::test_dir('tests/testthat', reporter = 'summary')"` if you want to narrow execution to the package smoke tests.

## Current slice validation — 2026-05-06

- Added deterministic pure-R scheduler smoke coverage for:
  - explicit native FFI not-configured status;
  - deterministic event ordering by `time_ticks`, `priority`, `sequence`, and `event_id`;
  - rejection of unknown, duplicate, and already-dispatched cancellation attempts;
  - base-R event-log roundtrip preserving the `kairo_ecs.event_log.v1` field order.
- Added `tests/smoke-base.R` so a base-R-only smoke path exists when `testthat` is not installed.
- `R CMD check --no-manual .` is blocked on this machine because `R` resolves to a PowerShell alias and `Rscript` is not on `PATH`.
- Fallback validation performed for this handoff:
  - checked that exported functions in `NAMESPACE` have matching definitions in `R/kairoecs.R`;
  - checked that exported functions have Rd aliases across `man/*.Rd`;
  - checked package skeleton files are present under `bindings/r/`.

## Focused local validation

- `node tests/conformance/track07_13_hardening_check.mjs` verifies this track no longer claims packaging ownership and records the no-release boundary.
- `powershell -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\06-python-binding-310-314\validate-bindings06-11.ps1` verifies the R cancellation guard, package metadata, and no-native-runtime boundary without requiring R on PATH.
- `Rscript tests/smoke-base.R` remains the preferred offline smoke command once `Rscript` is on `PATH`.

## Future-surface controls

- Do not add CRAN submission automation, registry credentials, or release publication here.
- Do not pull in Julia, Python, TypeScript, C#, or Go binding concerns.
- Do not widen to core runtime changes; remain at the R package boundary.
- Stop after local package validation until Track 12 owns fixture parity and Track 15 owns release dry-runs.

## CI command

```bash
Rscript -e "testthat::test_dir('tests', reporter = 'summary')" && R CMD check --no-manual .
```

## Local toolchain blocker

Install R >= 4.2 and ensure `Rscript` is on `PATH` before promoting this slice
from static/package-shape validation to full `R CMD check` evidence.

