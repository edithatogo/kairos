kairoecs_package_info <- function() {
  list(
    package = "kairoECS",
    title = "Event-Component Simulation Bindings",
    version = "0.1.0",
    surface = "bindings/r",
    track = "07-r-binding"
  )
}

kairoecs_surface_ready <- function() {
  info <- kairoecs_package_info()
  is.list(info) &&
    identical(info$package, "kairoECS") &&
    identical(info$surface, "bindings/r") &&
    identical(info$track, "07-r-binding")
}

