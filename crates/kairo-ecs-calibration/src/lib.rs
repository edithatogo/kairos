//! Optional calibration primitives; no empirical or clinical acceptance.
#![forbid(unsafe_code)]

pub mod seed_map;

// Kept crate-private under Track 21's no-public-API-without-ADR boundary.
mod trace_order;

// Private experimental C1 synthetic fixture adapter.
mod trace_mapping;

// Private bounded C1 source/timestamp normalization wrapper.
mod ingestion_normalize;
