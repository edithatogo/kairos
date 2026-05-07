candidate_roots <- c(getwd(), normalizePath(file.path(getwd(), ".."), mustWork = FALSE))
pkg_root <- candidate_roots[file.exists(file.path(candidate_roots, "R", "kairoecs.R"))][1]
if (is.na(pkg_root)) {
  library(kairoECS)
} else {
  source(file.path(pkg_root, "R", "kairoecs.R"), local = TRUE)
}

status <- kairoecs_ffi_status()
stopifnot(identical(status$configured, FALSE))

scheduler <- kairoecs_new_scheduler(run_id = "base-smoke")
scheduler <- kairoecs_schedule_at(scheduler, 2, "second", priority = 0)
scheduler <- kairoecs_schedule_at(scheduler, 1, "first", priority = 0)
scheduler <- kairoecs_run_until(scheduler, 2)
tryCatch(
  kairoecs_cancel_event(scheduler, 1),
  error = function(err) {
    stopifnot(grepl("not pending", conditionMessage(err), fixed = TRUE))
  }
)

log <- kairoecs_arrow_roundtrip(kairoecs_event_log(scheduler))
stopifnot(identical(log$event_kind, c("first", "second")))
stopifnot(identical(log$status, c("dispatched", "dispatched")))
stopifnot(identical(attr(log, "kairoecs_schema"), "kairo_ecs.event_log.v1"))

cat("base R smoke passed\n")
