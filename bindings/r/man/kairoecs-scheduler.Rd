\name{kairoecs-scheduler}
\alias{kairoecs_ffi_status}
\alias{kairoecs_new_scheduler}
\alias{kairoecs_schedule_at}
\alias{kairoecs_cancel_event}
\alias{kairoecs_run_until}
\alias{kairoecs_event_log}
\alias{kairoecs_arrow_roundtrip}
\title{Rust-required execution boundary and event-log conversion}
\description{
Legacy scheduler operations are retired and raise the typed
\code{kairoecs_native_required} condition. Execution requires a qualified
Rust native adapter, which is not implemented in this R package yet.
Event-log conversion of caller-supplied data remains available.
}
\usage{
kairoecs_ffi_status()

kairoecs_new_scheduler(run_id = "r-local", time_scale = "nanoseconds")

kairoecs_schedule_at(
  scheduler,
  time_ticks,
  event_kind,
  priority = 0L,
  entity_id = NA,
  payload_ref = NA_character_
)

kairoecs_cancel_event(scheduler, event_id)

kairoecs_run_until(scheduler, until_ticks = Inf)

kairoecs_event_log(scheduler)

kairoecs_arrow_roundtrip(event_log, use_arrow = FALSE)
}
\arguments{
\item{scheduler}{A \code{kairoecs_scheduler} object.}
\item{run_id}{Deterministic run/session identifier.}
\item{time_scale}{Time scale label, for example \code{"nanoseconds"} or
\code{"ticks"}.}
\item{time_ticks}{Non-negative simulation time.}
\item{event_kind}{Event classification string.}
\item{priority}{Integer priority. Lower values dispatch first at the same
time.}
\item{entity_id}{Optional entity identifier.}
\item{payload_ref}{Optional payload reference string.}
\item{event_id}{Event identifier returned in the event log.}
\item{until_ticks}{Simulation time through which scheduled events are
dispatched.}
\item{event_log}{A data frame matching \code{kairo_ecs.event_log.v2}.}
\item{use_arrow}{If \code{TRUE}, require the optional \pkg{arrow} package and
roundtrip through \code{arrow::Table}. If \code{FALSE}, normalize via base R.}
}
\value{
\code{kairoecs_ffi_status()} returns discovery metadata with native runtime
unconfigured. Every scheduler operation raises \code{kairoecs_native_required}.
\code{kairoecs_arrow_roundtrip()} returns a data frame with the
\code{kairo_ecs.event_log.v2} field order; it does not execute a simulation.
}
\examples{
kairoecs_ffi_status()
tryCatch(kairoecs_new_scheduler(), kairoecs_native_required = conditionMessage)
}
