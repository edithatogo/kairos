test_that("deterministic_ordering fixture is valid", {
  fixture <- jsonlite::read_json("../../conformance/fixtures/deterministic_ordering.json")
  expect_equal(fixture$version, 1)
  expect_equal(fixture$expected_kind_order, list(1, 2, 4, 3))
})

test_that("cancellation fixture is valid", {
  fixture <- jsonlite::read_json("../../conformance/fixtures/cancellation.json")
  expect_equal(fixture$expected_kind_order, list(1, 3))
})

test_that("rng_replay fixture is valid", {
  fixture <- jsonlite::read_json("../../conformance/fixtures/rng_replay.json")
  expect_equal(fixture$run_seed, 7)
  expect_length(fixture$expected_stream, 4)
})
