fixture_path <- function(name) {
  roots <- normalizePath(getwd(), mustWork = FALSE)
  for (i in seq_len(6)) {
    roots <- c(roots, normalizePath(file.path(tail(roots, 1), ".."), mustWork = FALSE))
  }

  candidates <- file.path(
    unique(roots),
    "conformance",
    "fixtures",
    name
  )
  candidates[file.exists(candidates)][1]
}

read_fixture <- function(name) {
  testthat::skip_if_not_installed("jsonlite")
  path <- fixture_path(name)
  if (is.na(path)) {
    stop(paste("missing fixture", name), call. = FALSE)
  }
  jsonlite::read_json(path)
}

run_scheduler_fixture <- function(fixture_name) {
  fixture <- read_fixture(fixture_name)
  scheduler <- kairoecs_new_scheduler(run_id = fixture$fixture, time_scale = "ticks")

  scheduled <- list()
  for (event in fixture$events) {
    scheduler <- kairoecs_schedule_at(
      scheduler,
      time_ticks = event$at_ticks,
      event_kind = as.character(event$kind),
      priority = event$priority,
      payload_ref = paste0("fixture:", fixture$fixture)
    )
    scheduled[[as.character(event$kind)]] <- scheduler$next_event_id - 1
  }

  for (event in fixture$events) {
    if (isTRUE(event$cancel)) {
      scheduler <- kairoecs_cancel_event(scheduler, scheduled[[as.character(event$kind)]])
    }
  }

  scheduler <- kairoecs_run_until(scheduler, Inf)
  log <- kairoecs_event_log(scheduler)
  dispatched <- log[log$status == "dispatched", , drop = FALSE]

  as.integer(dispatched$event_kind)
}

test_that("deterministic ordering fixture drives the scheduler facade", {
  expect_identical(
    run_scheduler_fixture("deterministic_ordering.json"),
    c(1L, 2L, 4L, 3L)
  )
})

test_that("cancellation fixture drives the scheduler facade", {
  expect_identical(
    run_scheduler_fixture("cancellation.json"),
    c(1L, 3L)
  )
})

test_that("zero-delay guard fixture keeps deterministic priority order", {
  expect_identical(
    run_scheduler_fixture("zero_delay_guard.json"),
    c(1L, 2L, 5L, 10L)
  )
})

test_that("rng_replay fixture remains metadata-only until the R RNG facade exists", {
  fixture <- read_fixture("rng_replay.json")
  expect_equal(fixture$run_seed, 7)
  expect_length(fixture$expected_stream, 4)
  expect_equal(fixture$requirement, "same run seed and entity handle produce the same stream")
})
