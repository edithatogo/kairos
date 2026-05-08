test_that("package metadata is available", {
  info <- kairoecs_package_info()

  expect_type(info, "list")
  expect_identical(info$package, "kairoECS")
  expect_identical(info$surface, "bindings/r")
  expect_identical(info$track, "07-r-binding")
})

test_that("surface readiness check passes", {
  expect_true(kairoecs_surface_ready())
})

test_that("native FFI status is explicit", {
  status <- kairoecs_ffi_status()

  expect_false(status$configured)
  expect_true(grepl("not configured", status$reason, fixed = TRUE))
  expect_true(is.na(status$abi))
})

test_that("scheduler dispatch order is deterministic", {
  scheduler <- kairoecs_new_scheduler(run_id = "smoke")
  scheduler <- kairoecs_schedule_at(scheduler, 10, "late", priority = 1)
  scheduler <- kairoecs_schedule_at(scheduler, 5, "first", priority = 0)
  scheduler <- kairoecs_schedule_at(scheduler, 10, "same-time-earlier-priority", priority = 0)
  scheduler <- kairoecs_run_until(scheduler, 10)

  log <- kairoecs_event_log(scheduler)
  expect_identical(log$event_kind, c("first", "same-time-earlier-priority", "late"))
  expect_identical(log$status, rep("dispatched", 3))
  expect_identical(log$sequence, c(2, 3, 1))
})

test_that("scheduler rejects unknown duplicate and dispatched cancellation", {
  scheduler <- kairoecs_new_scheduler(run_id = "cancel")
  scheduler <- kairoecs_schedule_at(scheduler, 1, "dispatched")
  scheduler <- kairoecs_schedule_at(scheduler, 2, "cancelled")

  expect_error(kairoecs_cancel_event(scheduler, 999), "not pending")

  scheduler <- kairoecs_cancel_event(scheduler, 2)
  expect_error(kairoecs_cancel_event(scheduler, 2), "not pending")

  scheduler <- kairoecs_run_until(scheduler, 1)
  expect_error(kairoecs_cancel_event(scheduler, 1), "not pending")

  log <- kairoecs_event_log(scheduler)
  expect_identical(log$status, c("dispatched", "cancelled"))
})

test_that("event log roundtrip preserves the v1 schema facade", {
  scheduler <- kairoecs_new_scheduler(run_id = "roundtrip", time_scale = "ticks")
  scheduler <- kairoecs_schedule_at(scheduler, 1, "arrive", entity_id = 42)

  log <- kairoecs_arrow_roundtrip(kairoecs_event_log(scheduler))

  expect_identical(attr(log, "kairoecs_schema"), "kairo_ecs.event_log.v1")
  expect_identical(
    names(log),
    c(
      "run_id", "event_id", "entity_id", "time_ticks", "time_scale",
      "priority", "sequence", "event_kind", "status", "payload_ref"
    )
  )
  expect_identical(log$run_id, "roundtrip")
  expect_identical(log$entity_id, 42)
})

test_that("optional Arrow roundtrip preserves the v1 schema facade", {
  testthat::skip_if_not_installed("arrow")

  scheduler <- kairoecs_new_scheduler(run_id = "arrow-roundtrip", time_scale = "ticks")
  scheduler <- kairoecs_schedule_at(scheduler, 1, "arrive", entity_id = 42)

  log <- kairoecs_arrow_roundtrip(kairoecs_event_log(scheduler), use_arrow = TRUE)

  expect_identical(attr(log, "kairoecs_schema"), "kairo_ecs.event_log.v1")
  expect_identical(
    names(log),
    c(
      "run_id", "event_id", "entity_id", "time_ticks", "time_scale",
      "priority", "sequence", "event_kind", "status", "payload_ref"
    )
  )
  expect_identical(log$run_id, "arrow-roundtrip")
  expect_identical(log$event_kind, "arrive")
  expect_equal(log$entity_id, 42)
})
