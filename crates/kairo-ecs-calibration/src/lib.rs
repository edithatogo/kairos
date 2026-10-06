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
