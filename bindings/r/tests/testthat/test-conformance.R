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

# Fixture metadata is not runtime conformance evidence.
test_that("shared scheduler fixture metadata remains readable", {
  for (name in c("deterministic_ordering.json", "cancellation.json", "zero_delay_guard.json")) {
    fixture <- read_fixture(name)
    expect_gt(length(fixture$events), 0)
    expect_gt(length(fixture$expected_kind_order), 0)
  }
})

test_that("shared RNG fixture metadata remains readable without an R RNG core", {
  fixture <- read_fixture("rng_replay.json")
  expect_equal(fixture$run_seed, 7)
  expect_length(fixture$expected_stream, 4)
})
