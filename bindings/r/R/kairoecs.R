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

# Track 70 Phase 2 (Task 2.3): explicit Schema v2 identifiers and the 14
# contract columns (generational-ID index/generation pairs, time_ticks_hi/lo
# limbs) projected onto a precision-safe pure-R data-frame facade. See
# README.md for the documented copies and limits.
KAIRECS_SCHEMA_NAME <- "kairo_ecs.event_log.v2"
KAIRECS_SCHEMA_VERSION <- 2L

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

  ticks_limbs <- kairoecs_u64_limbs(as.numeric(time_ticks))
  entity_limbs <- kairoecs_entity_limbs(entity_id)

  event <- data.frame(
    schema_version = KAIRECS_SCHEMA_VERSION,
    run_id = scheduler$run_id,
    event_id_index = kairoecs_u64_decimal(scheduler$next_event_id),
    event_id_gen = 0,
    entity_id_index = entity_limbs[["index"]],
    entity_id_gen = entity_limbs[["gen"]],
    time_ticks_hi = kairoecs_u64_decimal(ticks_limbs[["hi"]]),
    time_ticks_lo = kairoecs_u64_decimal(ticks_limbs[["lo"]]),
    time_scale = scheduler$time_scale,
    priority = as.integer(priority),
    sequence = kairoecs_u64_decimal(scheduler$next_sequence),
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

  idx <- scheduler$events$event_id_index == kairoecs_u64_decimal(event_id) &
    scheduler$events$status == "scheduled"
  if (!any(idx)) {
    stop("Event is not pending and cannot be cancelled.", call. = FALSE)
  }
  scheduler$events$status[idx] <- "cancelled"
  scheduler
}

kairoecs_run_until <- function(scheduler, until_ticks = Inf) {
  assert_scheduler(scheduler)
  stopifnot(length(until_ticks) == 1, !is.na(until_ticks))

  runnable <- scheduler$events$status == "scheduled" &
    kairoecs_row_ticks(scheduler$events) <= until_ticks
  scheduler$events$status[runnable] <- "dispatched"

  if (any(runnable)) {
    scheduler$current_time <- max(kairoecs_row_ticks(scheduler$events[runnable, , drop = FALSE]))
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

  attr(event_log, "kairoecs_schema") <- KAIRECS_SCHEMA_NAME
  event_log
}

kairoecs_empty_event_log <- function() {
  data.frame(
    schema_version = integer(),
    run_id = character(),
    event_id_index = character(),
    event_id_gen = numeric(),
    entity_id_index = character(),
    entity_id_gen = numeric(),
    time_ticks_hi = character(),
    time_ticks_lo = character(),
    time_scale = character(),
    priority = integer(),
    sequence = character(),
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

# u64 columns project as exact decimal character strings (base-R doubles lose
# integer precision above 2^53); u32 generations stay numeric.
kairoecs_u64_decimal <- function(value) {
  if (is.na(value)) {
    return(NA_character_)
  }
  sprintf("%.0f", as.numeric(value))
}

# Split an exact (<= 2^53) non-negative double tick count into u64 hi/lo
# limbs. Supporting SchedulerFacade inputs stays inside the 2^53 double
# precision limit; full u128 range interchange belongs to the native FFI
# surface (documented unsupported here).
kairoecs_u64_limbs <- function(value) {
  value <- as.numeric(value)
  stopifnot(value >= 0, value == floor(value), value <= 2^53)
  hi <- floor(value / 2^64)
  lo <- value - hi * 2^64
  list(hi = hi, lo = lo)
}

# Entity projections are null-together per the v2 contract: both columns are
# set together or both are NA. Facade-local single-generation handles project
# generation 0 explicitly (the pure-R facade cannot observe engine-side ID
# recycling).
kairoecs_entity_limbs <- function(entity_id) {
  if (length(entity_id) == 0 || is.na(entity_id[1])) {
    return(list(index = NA_character_, gen = NA_real_))
  }
  list(
    index = kairoecs_u64_decimal(as.numeric(entity_id)),
    gen = 0
  )
}

# Numeric tick value of a limb row bundle for dispatch comparisons;
# ordering uses (hi, lo) so the comparison stays inside the facade's
# documented double precision limits.
kairoecs_row_ticks <- function(event_log) {
  if (nrow(event_log) == 0) {
    return(numeric(0))
  }
  as.numeric(event_log$time_ticks_hi) * (2^64) + as.numeric(event_log$time_ticks_lo)
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
  event_log$schema_version <- as.integer(event_log$schema_version)
  event_log$run_id <- as.character(event_log$run_id)
  event_log$event_id_index <- as.character(event_log$event_id_index)
  event_log$event_id_gen <- as.numeric(event_log$event_id_gen)
  event_log$entity_id_index <- as.character(event_log$entity_id_index)
  event_log$entity_id_gen <- as.numeric(event_log$entity_id_gen)
  event_log$time_ticks_hi <- as.character(event_log$time_ticks_hi)
  event_log$time_ticks_lo <- as.character(event_log$time_ticks_lo)
  event_log$time_scale <- as.character(event_log$time_scale)
  event_log$priority <- as.integer(event_log$priority)
  event_log$sequence <- as.character(event_log$sequence)
  event_log$event_kind <- as.character(event_log$event_kind)
  event_log$status <- as.character(event_log$status)
  event_log$payload_ref <- as.character(event_log$payload_ref)
  order_event_log(event_log)
}

order_event_log <- function(event_log) {
  event_log[order(
    as.numeric(event_log$time_ticks_hi),
    as.numeric(event_log$time_ticks_lo),
    event_log$priority,
    as.numeric(event_log$sequence),
    event_log$event_id_index
  ), , drop = FALSE]
}

kairoecs_native_library_loader <- function(lib_path = NULL) {
  ext <- ifelse(.Platform$OS.type == "windows", ".dll", ifelse(Sys.info()["sysname"] == "Darwin", ".dylib", ".so"))
  lib_name <- paste0("libkairo_ecs_ffi", ext)
  found <- file.exists(lib_name)
  list(
    native_library = lib_name,
    available = found,
    arrow_c_data_interface_supported = TRUE
  )
}
