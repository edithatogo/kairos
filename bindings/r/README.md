# R binding

Track 07 owns this package. The pure-R scheduler is retired immediately.
`kairoecs_new_scheduler`, `kairoecs_schedule_at`, `kairoecs_cancel_event`,
`kairoecs_run_until` and `kairoecs_event_log` all raise the typed
`kairoecs_native_required` condition. There is no R core or execution fallback.

`kairoecs_ffi_status()` reports unconfigured, and `kairoecs_surface_ready()` is
false until a qualified Rust runtime adapter exists. Artifact discovery reports
file presence separately; it cannot claim a loaded ABI or Arrow support.

Package metadata and `kairoecs_arrow_roundtrip` remain available. Conversion
accepts an existing Schema v2 data frame, preserving u64 fields as decimal
strings and u32 generations as numeric values. It copies values into base R or
an optional `arrow::Table`. It does not schedule events, invent generation zero
IDs, or consume populated Rust Arrow buffers.

Actual engine support requires a Rust FFI adapter, owned-handle lifetime
management, full integer/error handling, artifact packaging, and native
execution tests that fail without Rust. Fixture parsing and conversion tests
are explicitly metadata/data evidence and cannot substitute for these gates.

Validation from this directory:

```bash
Rscript tests/smoke-base.R
Rscript -e "testthat::test_dir('tests/testthat', reporter = 'summary')"
R CMD check --no-manual .
```

Optional Arrow conversion tests require the R `arrow` package. Their absence
must not be interpreted as native runtime acceptance.
