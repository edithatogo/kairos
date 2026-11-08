//! Trusted owned Arrow C Data imports shared by native domain consumers.
//!
//! This is the audited import boundary for the original private ownership
//! holders from `kairo-ecs-arrow`. Arbitrary raw pointers cannot enter its safe
//! APIs. Default builds have no Arrow, Polars or engine dependency. The existing
//! engine FFI crate reexports these same APIs without a second implementation.
#![deny(unsafe_code)]

/// Safe consumers of the original trusted owned domain exports.
#[cfg(feature = "native-data")]
#[allow(unsafe_code)]
pub mod native_tables;
