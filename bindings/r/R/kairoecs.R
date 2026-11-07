kairoecs_package_info <- function() {
  list(
    package = "kairoECS",
    title = "Event-Component Simulation Bindings",
    version = "0.1.0",
    surface = "bindings/r",
    track = "07-r-binding"
  )
}

# Runtime readiness requires a qualified Rust FFI adapter, not metadata alone.
kairoecs_surface_ready <- function() FALSE

kairoecs_ffi_status <- function() {
  list(
    configured = FALSE,
    reason = paste(
      "Native KairoECS FFI is not configured for this R package slice.",
      "A qualified Rust adapter is required; the pure-R scheduler is retired."
    ),
    abi = NA_character_,
    fallback_supported = FALSE
  )
}

# Track 70 Phase 2 (Task 2.3): explicit Schema v2 identifiers and the 14
# contract columns (generational-ID index/generation pairs, time_ticks_hi/lo
# limbs) projected onto a precision-safe pure-R data-frame facade. See
# README.md for the documented copies and limits.
KAIRECS_SCHEMA_NAME <- "kairo_ecs.event_log.v2"
KAIRECS_SCHEMA_VERSION <- 2L

kairoecs_native_required <- function() {
  stop(structure(
    list(
      message = paste(
        "The pure-R scheduler is retired. A qualified Rust native adapter is required.",
        "This package currently supports metadata and event-log conversion only."
      ),
      call = NULL
    ),
    class = c("kairoecs_native_required", "error", "condition")
  ))
}

kairoecs_new_scheduler <- function(run_id = "r-local", time_scale = "nanoseconds") {
  kairoecs_native_required()
}

kairoecs_schedule_at <- function(
    scheduler, time_ticks, event_kind, priority = 0L,
    entity_id = NA, payload_ref = NA_character_) {
  kairoecs_native_required()
}

kairoecs_cancel_event <- function(scheduler, event_id) kairoecs_native_required()
kairoecs_run_until <- function(scheduler, until_ticks = Inf) kairoecs_native_required()
kairoecs_event_log <- function(scheduler) kairoecs_native_required()

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
  # Limb ordering pairs the numeric view with a lexicographic decimal-string
  # tie-break so externally supplied rows above the 2^53 double limit still
  # sort in strict numeric limb order instead of collapsing in as.numeric().
  event_log[order(
    as.numeric(event_log$time_ticks_hi),
    event_log$time_ticks_hi,
    as.numeric(event_log$time_ticks_lo),
    event_log$time_ticks_lo,
    event_log$priority,
    as.numeric(event_log$sequence),
    event_log$sequence,
    event_log$event_id_index
  ), , drop = FALSE]
}

# Artifact discovery is metadata only. Do not claim that an arbitrary file is
# a loaded Rust engine or that it implements the Arrow C Data Interface.
kairoecs_native_library_loader <- function(lib_path = NULL) {
  ext <- ifelse(.Platform$OS.type == "windows", ".dll", ifelse(Sys.info()["sysname"] == "Darwin", ".dylib", ".so"))
  prefix <- ifelse(.Platform$OS.type == "windows", "", "lib")
  lib_name <- if (is.null(lib_path)) paste0(prefix, "kairo_ecs_ffi", ext) else lib_path
  list(
    native_library = lib_name,
    artifact_present = file.exists(lib_name),
    available = FALSE,
    arrow_c_data_interface_supported = FALSE,
    reason = "Rust library loading and ABI qualification are not implemented in this R binding."
  )
}
