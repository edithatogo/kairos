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

