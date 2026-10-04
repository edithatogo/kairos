//! Optional calibration primitives; no empirical or clinical acceptance.
#![forbid(unsafe_code)]

pub mod seed_map;

// Kept crate-private under Track 21's no-public-API-without-ADR boundary.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "Private C1 adapters are exercised by conformance tests; public API review is pending"
    )
)]
mod trace_order;

// Private experimental C1 synthetic fixture adapter.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "Private C1 adapters are exercised by conformance tests; public API review is pending"
    )
)]
mod trace_mapping;

// Private bounded C1 source/timestamp normalization wrapper.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "Private C1 adapters are exercised by conformance tests; public API review is pending"
    )
)]
mod ingestion_normalize;

// Private experimental validation over sorted calibration records.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "Private C1 adapters are exercised by conformance tests; public API review is pending"
    )
)]
mod ingestion_validate;

// Private C1 ingestion stages; public API review remains separate.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "Private C1 adapters are exercised by conformance tests; public API review is pending"
    )
)]
mod ingestion_pipeline;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "Private C1 adapters are exercised by conformance tests; public API review is pending"
    )
)]
mod ingestion_sort;

// Exact C-01 invariance qualification over private ingestion interfaces.
#[cfg(test)]
mod ingestion_c01;
