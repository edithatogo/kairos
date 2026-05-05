\name{kairoecs-scheduler}
\alias{kairoecs_ffi_status}
\alias{kairoecs_new_scheduler}
\alias{kairoecs_schedule_at}
\alias{kairoecs_cancel_event}
\alias{kairoecs_run_until}
\alias{kairoecs_event_log}
\alias{kairoecs_arrow_roundtrip}
\title{Deterministic scheduler facade}
\description{
Minimal pure-R scheduler and event-log helpers for the Track 07 package slice.
Native FFI is reported as not configured until the stable C ABI is available to
the package.
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
\item{event_log}{A data frame matching \code{kairo_ecs.event_log.v1}.}
\item{use_arrow}{If \code{TRUE}, require the optional \pkg{arrow} package and
roundtrip through \code{arrow::Table}. If \code{FALSE}, normalize via base R.}
}
\value{
\code{kairoecs_ffi_status()} returns a list. Scheduler mutators return an
updated scheduler object. \code{kairoecs_event_log()} and
\code{kairoecs_arrow_roundtrip()} return data frames with the
\code{kairo_ecs.event_log.v1} field order.
}
\examples{
scheduler <- kairoecs_new_scheduler("example")
scheduler <- kairoecs_schedule_at(scheduler, 10, "arrive")
scheduler <- kairoecs_run_until(scheduler, 10)
kairoecs_event_log(scheduler)
}
