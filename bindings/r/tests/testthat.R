library(testthat)

if (requireNamespace("kairoECS", quietly = TRUE)) {
  test_check("kairoECS")
} else {
  if (file.exists(file.path("tests", "helper-load.R"))) {
    source(file.path("tests", "helper-load.R"), local = TRUE)
    test_dir(file.path("tests", "testthat"), reporter = "summary")
  } else {
    source("helper-load.R", local = TRUE)
    test_dir("testthat", reporter = "summary")
  }
}
