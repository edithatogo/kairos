pkg_root <- NULL

if (dir.exists("R")) {
  pkg_root <- "."
} else if (dir.exists(file.path("..", "R"))) {
  pkg_root <- ".."
} else if (dir.exists(file.path("..", "..", "R"))) {
  pkg_root <- file.path("..", "..")
} else {
  pkg_root <- NA_character_
}

if (is.na(pkg_root)) {
  library(kairoECS)
} else {
  r_dir <- file.path(pkg_root, "R")
  r_files <- sort(list.files(r_dir, pattern = "\\.[Rr]$", full.names = TRUE))
  for (path in r_files) {
    source(path, local = TRUE)
  }
}
