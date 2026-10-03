//! Optional calibration primitives; no empirical or clinical acceptance.
#![forbid(unsafe_code)]

pub mod seed_map;

// Kept crate-private under Track 21's no-public-API-without-ADR boundary.
mod trace_order;
