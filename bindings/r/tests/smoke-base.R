candidate_roots <- c(getwd(), normalizePath(file.path(getwd(), ".."), mustWork = FALSE))
pkg_root <- candidate_roots[file.exists(file.path(candidate_roots, "R", "kairoecs.R"))][1]
if (is.na(pkg_root)) {
  library(kairoECS)
} else {
  source(file.path(pkg_root, "R", "kairoecs.R"), local = TRUE)
}

status <- kairoecs_ffi_status()
stopifnot(identical(status$configured, FALSE), identical(status$fallback_supported, FALSE))
stopifnot(identical(kairoecs_surface_ready(), FALSE))

# Every legacy entry point fails, even for fabricated scheduler objects.
operations <- list(
  function() kairoecs_new_scheduler(),
  function() kairoecs_schedule_at(list(), 1, "event"),
  function() kairoecs_cancel_event(list(), 1),
  function() kairoecs_run_until(list(), 1),
  function() kairoecs_event_log(list())
)
for (operation in operations) {
  error <- tryCatch({ operation(); NULL }, error = identity)
  stopifnot(inherits(error, "kairoecs_native_required"))
}

# This is data conversion evidence, not engine execution.
log <- data.frame(
  schema_version = 2L, run_id = "data-only", event_id_index = "1", event_id_gen = 0,
  entity_id_index = NA_character_, entity_id_gen = NA_real_,
  time_ticks_hi = "0", time_ticks_lo = "5", time_scale = "ticks", priority = 0L,
  sequence = "0", event_kind = "event", status = "dispatched", payload_ref = NA_character_
)
converted <- kairoecs_arrow_roundtrip(log)
stopifnot(identical(converted$event_kind, "event"))
stopifnot(identical(attr(converted, "kairoecs_schema"), "kairo_ecs.event_log.v2"))
cat("base R metadata and native-required checks passed; no engine execution claimed\n")
