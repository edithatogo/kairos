# Test Matrix — 07 R Binding

## Required tests

- `Rscript -e "testthat::test_dir('tests', reporter = 'summary')"` from `bindings/r/`.
- `Rscript tests/smoke-base.R` from `bindings/r/` for base-R smoke coverage without `testthat`.
- `R CMD check --no-manual .` from `bindings/r/`.
- On Windows PowerShell, prefer `Rcmd check --no-manual r` from `bindings/`
  because `R` is a built-in alias for `Invoke-History`.
- `Rscript -e "devtools::check(document = FALSE)"` once `devtools` is available in the local toolchain.
- `R CMD build .` when artifact validation is needed from `bindings/r/`.
- `Rscript -e "testthat::test_dir('tests/testthat', reporter = 'summary')"` if you want to narrow execution to the package smoke tests.

## Current slice validation — 2026-05-07

- Added deterministic pure-R scheduler smoke coverage for:
  - explicit native FFI not-configured status;
  - deterministic event ordering by `time_ticks`, `priority`, `sequence`, and `event_id`;
  - rejection of unknown, duplicate, and already-dispatched cancellation attempts;
  - base-R event-log roundtrip preserving the `kairo_ecs.event_log.v1` field order.
- Added testthat fixture bridge coverage that loads ready shared conformance fixtures with `jsonlite` and drives the exported scheduler facade for:
  - `deterministic_ordering.json`;
  - `cancellation.json`;
  - `zero_delay_guard.json`.
- Kept `rng_replay.json` as metadata-only until this track has an R RNG facade.
- Added `tests/smoke-base.R` so a base-R-only smoke path exists when `testthat` is not installed.
- Installed R 4.6.0 via Scoop and verified `Rscript` resolves to `C:\Users\60217257\scoop\shims\rscript.exe`.
- Installed the focused test dependencies `jsonlite` and `testthat` into the Scoop-persisted R site library.
- Updated the R package metadata, MIT license stub, and test helpers so validation works both from the source tree and under `Rcmd check`.
- Local validation performed with `LC_ALL=C`, `LC_CTYPE=C`, and `LANG=C` because this Windows R build warns on the inherited `C.UTF-8` locale variables.

## Focused local validation

- `node tests/conformance/track07_13_hardening_check.mjs` verifies this track no longer claims packaging ownership and records the no-release boundary.
- `powershell -NoProfile -ExecutionPolicy Bypass -File conductor\tracks\06-python-binding-310-314\validate-bindings06-11.ps1` verifies the R cancellation guard, package metadata, and no-native-runtime boundary without requiring R on PATH.
- `Rscript tests/smoke-base.R` passes from `bindings/r/`.
- `Rscript -e "testthat::test_dir('tests', reporter = 'summary')"` passes from `bindings/r/`.
- `Rcmd check --no-manual r` from `bindings/` completes cleanly when `_R_CHECK_FORCE_SUGGESTS_=false` and the locale is pinned to `LC_ALL=C`, `LC_CTYPE=C`, and `LANG=C`.

## Future-surface controls

- Do not add CRAN submission automation, registry credentials, or release publication here.
- Do not pull in Julia, Python, TypeScript, C#, or Go binding concerns.
- Do not widen to core runtime changes; remain at the R package boundary.
- Stop after local package validation until Track 12 owns fixture parity and Track 15 owns release dry-runs.

## CI command

```bash
Rscript -e "testthat::test_dir('tests', reporter = 'summary')" && R CMD check --no-manual .
```

## Local toolchain notes

R 4.6.0 and `Rscript` are now available locally through Scoop. Optional
packages `arrow`, `devtools`, `lintr`, and `pkgdown` are still not installed,
so local package checking used `_R_CHECK_FORCE_SUGGESTS_=false`.

## Current slice validation — 2026-05-10

- Added an explicit optional Arrow-backed roundtrip test for
  `kairo_ecs.event_log.v1` via `kairoecs_arrow_roundtrip(..., use_arrow = TRUE)`.
- The Arrow-backed lane is present and skips with a testthat reason when the
  optional R `arrow` package is unavailable.
- `Rscript tests/smoke-base.R` passes from `bindings/r/`.
- `Rscript -e "testthat::test_dir('tests', reporter = 'summary')"` passes from
  `bindings/r/` with one expected skip for missing `arrow`.
- `Rcmd check --no-manual r` from `bindings/` passes when `_R_CHECK_FORCE_SUGGESTS_=false`
  and the locale is pinned to `LC_ALL=C`, `LC_CTYPE=C`, and `LANG=C`.
- `Rcmd build r` from `bindings/` builds `kairoECS_0.1.0.tar.gz`.
- `Rcmd check --no-manual kairoECS_0.1.0.tar.gz` from `bindings/` completes
  with `Status: OK` when `_R_CHECK_FORCE_SUGGESTS_=false` and the locale is
  pinned to `LC_ALL=C`, `LC_CTYPE=C`, and `LANG=C`.
- `node tests/conformance/track07_13_hardening_check.mjs` passes.
- `powershell -NoProfile -ExecutionPolicy Bypass -File
  conductor\tracks\06-python-binding-310-314\validate-bindings06-11.ps1`
  passes.
- Optional R packages `arrow`, `devtools`, `lintr`, and `pkgdown` remain absent
  locally, so the Arrow-backed runtime lane and `devtools::check(document =
  FALSE)` remain future-surface work rather than closeout blockers.

## Prior slice validation — 2026-05-09

- `$conductor-review` found no in-scope correctness issues requiring code
  changes in `bindings/r/`.
- `Rscript --version` reports `Rscript (R) version 4.6.0 (2026-04-24)`.
- `Rscript tests/smoke-base.R` passes from `bindings/r/`.
- `Rscript -e "testthat::test_dir('tests', reporter = 'summary')"` passes
  from `bindings/r/` with one expected skip for missing optional `arrow`.
- `Rcmd check --no-manual r` from `bindings/` completed with `Status: 1 NOTE`
  before the locale-pinned rerun; the NOTE was the expected source-directory
  check warning that checking should be performed on sources prepared by
  `R CMD build`.
- `Rcmd build r` from `bindings/` builds `kairoECS_0.1.0.tar.gz`.
- `Rcmd check --no-manual kairoECS_0.1.0.tar.gz` from `bindings/` completed
  with `Status: OK` when `_R_CHECK_FORCE_SUGGESTS_=false`.
- Optional R packages `arrow`, `devtools`, `lintr`, and `pkgdown` remained
  absent locally in the 2026-05-09 pass; the 2026-05-10 locale-pinned rerun
  still treats them as future-surface work rather than closeout blockers.

## Phase closeout gate

- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` and `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1` must pass before any phase advances; this enforces `$conductor-review`, auto-apply of accepted fixes, phase-closeout ledger evidence, cleaned commit/push evidence, and blocker recording. Validation is green, but strict git closeout still fails because the shared worktree is dirty. At actual closeout, run `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` after commit and push.
