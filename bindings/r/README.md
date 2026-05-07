# R Binding

This package root contains the Track 07 R binding surface.

The current slice is intentionally small and deterministic:

- `kairoecs_ffi_status()` reports native FFI as not configured.
- `kairoecs_new_scheduler()` creates a pure-R scheduler facade.
- `kairoecs_schedule_at()`, `kairoecs_cancel_event()`, and
  `kairoecs_run_until()` update scheduler state without side effects.
- `kairoecs_event_log()` returns an event-log data frame aligned to
  `kairo_ecs.event_log.v1`.
- `kairoecs_arrow_roundtrip()` normalizes the event-log schema in base R, or
  roundtrips through the optional `arrow` package when `use_arrow = TRUE`.

Current package files:

- `DESCRIPTION`
- `LICENSE`
- `NAMESPACE`
- `R/kairoecs.R`
- `man/kairoECS-package.Rd`
- `man/kairoecs-scheduler.Rd`
- `tests/testthat.R`
- `tests/helper-load.R`
- `tests/smoke-base.R`
- `tests/testthat/helper-load.R`
- `tests/testthat/test-smoke.R`
- `tests/testthat/test-conformance.R`

Local validation from `bindings/r/`:

- `Rscript tests/smoke-base.R`
- `Rscript -e "testthat::test_dir('tests', reporter = 'summary')"`
- `R CMD check --no-manual .`
- `Rscript -e "devtools::check(document = FALSE)"`

The testthat conformance tests use `jsonlite` to load the ready shared
fixtures and then drive the exported pure-R scheduler facade. The base smoke
script remains available for environments without optional R test packages.
