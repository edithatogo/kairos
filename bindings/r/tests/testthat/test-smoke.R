test_that("package metadata does not claim runtime readiness", {
  info <- kairoecs_package_info()
  expect_identical(info$package, "kairoECS")
  expect_identical(info$surface, "bindings/r")
  expect_false(kairoecs_surface_ready())
  status <- kairoecs_ffi_status()
  expect_false(status$configured)
  expect_false(status$fallback_supported)
  expect_true(is.na(status$abi))
})

test_that("all legacy runtime operations require Rust", {
  expect_error(kairoecs_new_scheduler(), class = "kairoecs_native_required")
  fabricated <- structure(list(), class = "kairoecs_scheduler")
  expect_error(kairoecs_schedule_at(fabricated, 1, "event"), class = "kairoecs_native_required")
  expect_error(kairoecs_cancel_event(fabricated, 1), class = "kairoecs_native_required")
  expect_error(kairoecs_run_until(fabricated, 1), class = "kairoecs_native_required")
  expect_error(kairoecs_event_log(fabricated), class = "kairoecs_native_required")
})

# Explicit data fixtures avoid presenting host-generated rows as Rust execution.
data_fixture <- function() data.frame(
  schema_version = 2L, run_id = "data-only", event_id_index = "18446744073709551615", event_id_gen = 3,
  entity_id_index = "42", entity_id_gen = 2,
  time_ticks_hi = "0", time_ticks_lo = "9007199254740993", time_scale = "ticks", priority = 0L,
  sequence = "0", event_kind = "event", status = "dispatched", payload_ref = NA_character_
)

test_that("event log conversion preserves exact external decimal IDs and ticks", {
  log <- kairoecs_arrow_roundtrip(data_fixture())
  expect_identical(attr(log, "kairoecs_schema"), "kairo_ecs.event_log.v2")
  expect_identical(log$event_id_index, "18446744073709551615")
  expect_identical(log$time_ticks_lo, "9007199254740993")
  expect_identical(log$entity_id_gen, 2)
  expect_error(kairoecs_arrow_roundtrip(data.frame()), "missing required fields")
})

test_that("a file is not evidence of a loaded native ABI", {
  artifact <- tempfile()
  writeLines("not a library", artifact)
  on.exit(unlink(artifact))
  status <- kairoecs_native_library_loader(artifact)
  expect_true(status$artifact_present)
  expect_false(status$available)
  expect_false(status$arrow_c_data_interface_supported)
})

test_that("optional Arrow conversion preserves the external data fixture", {
  testthat::skip_if_not_installed("arrow")
  log <- kairoecs_arrow_roundtrip(data_fixture(), use_arrow = TRUE)
  expect_identical(log$event_id_index, "18446744073709551615")
  expect_identical(log$time_ticks_lo, "9007199254740993")
})
