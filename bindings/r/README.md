# R Binding

This package root contains the Track 07 R binding surface.

The current slice is intentionally small and deterministic:

The current slice is intentionally small and deterministic:

- `kairoecs_ffi_status()` reports native FFI as not configured.
- `kairoecs_new_scheduler()` creates a pure-R scheduler facade.
- `kairoecs_schedule_at()`, `kairoecs_cancel_event()`, and
  `kairoecs_run_until()` update scheduler state without side effects.
- `kairoecs_event_log()` returns an event-log data frame aligned to
  the Track 70 Schema v2 `kairo_ecs.event_log.v2` contract (14 columns:
  generational-ID `event_id_index`/`event_id_gen` and
  `entity_id_index`/`entity_id_gen` pairs, `time_ticks_hi`/`time_ticks_lo`
  limbs).
- `kairoecs_arrow_roundtrip()` normalizes the event-log schema in base R, or
  roundtrips through the optional `arrow` package when `use_arrow = TRUE`.

**Documented copies and unsupported targets (Track 70 Phase 2, Task 2.3):**

- The facade performs a **row copy** into a plain base-R data frame; there is
  no Arrow C Data Interface buffer hand-off in this slice. The optional
  `arrow` roundtrip is a table-level copy through `arrow::Table`.
- u64 columns (`event_id_index`, `entity_id_index`, `time_ticks_hi`,
  `time_ticks_lo`, `sequence`) project as **exact decimal character strings**
  (base-R doubles lose integer precision above 2^53); u32 generations stay
  numeric.
- Facade-local single-generation handles project **generation `0`**
  explicitly; entity projections are **null-together** (both columns set or
  both `NA`).
- Facade-local single-generation handles project **generation `0`**
  explicitly; entity projections are **null-together** (both columns set or
  both `NA`).
- Scheduler and entity inputs are exact-integer guarded: values beyond the
  2^53 double precision limit **reject explicitly** (facade limit; full
  u64/u128 interchange belongs to the native FFI surface). Populated Arrow
  C Data/Stream export is an **unsupported target** here until the native
  FFI artifact path is configured (`kairoecs_ffi_status()` stays
  `configured = FALSE`).

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
