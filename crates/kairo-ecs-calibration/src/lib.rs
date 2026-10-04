//! Optional calibration primitives; no empirical or clinical acceptance.
#![forbid(unsafe_code)]

pub mod seed_map;

// Kept crate-private under Track 21's no-public-API-without-ADR boundary.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the private C1 mapper consumes this ordering key before public API review"
    )
)]
mod trace_order;

// Private experimental C1 synthetic fixture adapter.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the C1.1 mapper is exercised by focused fixture tests before public API review"
    )
)]
mod trace_mapping;

// Private bounded C1 source/timestamp normalization wrapper.
mod ingestion_normalize;

// Private experimental validation over sorted calibration records.
mod ingestion_validate;

// Private C1 ingestion stages; public API review remains separate.
mod ingestion_pipeline;
mod ingestion_sort;

// Exact C-01 invariance qualification over private ingestion interfaces.
#[cfg(test)]
mod ingestion_c01;
