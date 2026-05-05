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

kairoecs_ffi_status <- function() {
  list(
    configured = FALSE,
    reason = paste(
      "Native KairoECS FFI is not configured for this R package slice.",
      "Using the deterministic pure-R scheduler facade."
    ),
    abi = NA_character_
  )
}

kairoecs_new_scheduler <- function(run_id = "r-local", time_scale = "nanoseconds") {
  stopifnot(length(run_id) == 1, !is.na(run_id), nzchar(run_id))
  stopifnot(length(time_scale) == 1, !is.na(time_scale), nzchar(time_scale))

  scheduler <- list(
    run_id = as.character(run_id),
    time_scale = as.character(time_scale),
    current_time = 0,
    next_event_id = 1,
    next_sequence = 1,
    events = kairoecs_empty_event_log()
  )
  class(scheduler) <- "kairoecs_scheduler"
  scheduler
}

kairoecs_schedule_at <- function(
    scheduler,
    time_ticks,
    event_kind,
    priority = 0L,
    entity_id = NA,
    payload_ref = NA_character_) {
  assert_scheduler(scheduler)
  stopifnot(length(time_ticks) == 1, is.finite(time_ticks), time_ticks >= 0)
  stopifnot(length(event_kind) == 1, !is.na(event_kind), nzchar(event_kind))
  stopifnot(length(priority) == 1, is.finite(priority))
  stopifnot(length(entity_id) <= 1)
  stopifnot(length(payload_ref) <= 1)

  event <- data.frame(
    run_id = scheduler$run_id,
    event_id = scheduler$next_event_id,
    entity_id = normalize_nullable_number(entity_id),
    time_ticks = as.numeric(time_ticks),
    time_scale = scheduler$time_scale,
    priority = as.integer(priority),
    sequence = scheduler$next_sequence,
    event_kind = as.character(event_kind),
    status = "scheduled",
    payload_ref = normalize_nullable_character(payload_ref),
    stringsAsFactors = FALSE
  )

  scheduler$events <- rbind(scheduler$events, event)
  scheduler$next_event_id <- scheduler$next_event_id + 1
  scheduler$next_sequence <- scheduler$next_sequence + 1
  scheduler$events <- order_event_log(scheduler$events)
  scheduler
}

kairoecs_cancel_event <- function(scheduler, event_id) {
  assert_scheduler(scheduler)
  stopifnot(length(event_id) == 1, is.finite(event_id))

  idx <- scheduler$events$event_id == as.numeric(event_id) &
    scheduler$events$status == "scheduled"
  scheduler$events$status[idx] <- "cancelled"
  scheduler
}

kairoecs_run_until <- function(scheduler, until_ticks = Inf) {
  assert_scheduler(scheduler)
  stopifnot(length(until_ticks) == 1, !is.na(until_ticks))

  runnable <- scheduler$events$status == "scheduled" &
    scheduler$events$time_ticks <= until_ticks
  scheduler$events$status[runnable] <- "dispatched"

  if (any(runnable)) {
    scheduler$current_time <- max(scheduler$events$time_ticks[runnable])
  } else if (is.finite(until_ticks)) {
    scheduler$current_time <- max(scheduler$current_time, as.numeric(until_ticks))
  }

  scheduler$events <- order_event_log(scheduler$events)
  scheduler
}

kairoecs_event_log <- function(scheduler) {
  assert_scheduler(scheduler)
  scheduler$events
}

kairoecs_arrow_roundtrip <- function(event_log, use_arrow = FALSE) {
  event_log <- normalize_event_log(event_log)

  if (isTRUE(use_arrow)) {
    if (!requireNamespace("arrow", quietly = TRUE)) {
      stop("The optional arrow package is not installed.", call. = FALSE)
    }
    table <- arrow::Table$create(event_log)
    event_log <- as.data.frame(table)
  }

  attr(event_log, "kairoecs_schema") <- "kairo_ecs.event_log.v1"
  event_log
}

kairoecs_empty_event_log <- function() {
  data.frame(
    run_id = character(),
    event_id = numeric(),
    entity_id = numeric(),
    time_ticks = numeric(),
    time_scale = character(),
    priority = integer(),
    sequence = numeric(),
    event_kind = character(),
    status = character(),
    payload_ref = character(),
    stringsAsFactors = FALSE
  )
}

assert_scheduler <- function(scheduler) {
  if (!inherits(scheduler, "kairoecs_scheduler")) {
    stop("Expected a kairoecs_scheduler object.", call. = FALSE)
  }
}

normalize_nullable_number <- function(value) {
  if (length(value) == 0 || is.na(value[1])) {
    return(NA_real_)
  }
  as.numeric(value)
}

normalize_nullable_character <- function(value) {
  if (length(value) == 0 || is.na(value[1])) {
    return(NA_character_)
  }
  as.character(value)
}

normalize_event_log <- function(event_log) {
  required <- names(kairoecs_empty_event_log())
  missing <- setdiff(required, names(event_log))
  if (length(missing) > 0) {
    stop(
      paste("Event log is missing required fields:", paste(missing, collapse = ", ")),
      call. = FALSE
    )
  }

  event_log <- event_log[required]
  event_log$run_id <- as.character(event_log$run_id)
  event_log$event_id <- as.numeric(event_log$event_id)
  event_log$entity_id <- as.numeric(event_log$entity_id)
  event_log$time_ticks <- as.numeric(event_log$time_ticks)
  event_log$time_scale <- as.character(event_log$time_scale)
  event_log$priority <- as.integer(event_log$priority)
  event_log$sequence <- as.numeric(event_log$sequence)
  event_log$event_kind <- as.character(event_log$event_kind)
  event_log$status <- as.character(event_log$status)
  event_log$payload_ref <- as.character(event_log$payload_ref)
  order_event_log(event_log)
}

order_event_log <- function(event_log) {
  event_log[order(
    event_log$time_ticks,
    event_log$priority,
    event_log$sequence,
    event_log$event_id
  ), , drop = FALSE]
}
