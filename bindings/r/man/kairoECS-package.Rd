\name{kairoECS-package}
\alias{kairoECS-package}
\alias{kairoecs_package_info}
\alias{kairoecs_surface_ready}
\docType{package}
\title{R bindings for KairoECS}
\description{
Minimal package surface for the R binding track. The exported helpers expose
package metadata, a deterministic pure-R scheduler facade, event-log schema
normalization, and an explicit not-configured native FFI status.
}
\details{
\code{kairoecs_package_info()} returns the package metadata as a named list.
\code{kairoecs_surface_ready()} returns \code{TRUE} when the package surface
metadata is internally consistent.
See \code{\link{kairoecs_new_scheduler}} for the scheduler facade and
\code{\link{kairoecs_arrow_roundtrip}} for the optional Arrow roundtrip helper.
}
\usage{
kairoecs_package_info()
kairoecs_surface_ready()
}
\seealso{
\code{\link{kairoecs_package_info}}
\code{\link{kairoecs_surface_ready}}
}
\keyword{package}
