//! Optional calibration primitives; no empirical or clinical acceptance.
#![forbid(unsafe_code)]

#[cfg(feature = "flow")]
#[expect(
    dead_code,
    reason = "Complete C2 composite assembler consumes the reviewed envelope next"
)]
mod checkpoint_envelope;
#[cfg(feature = "flow")]
#[expect(
    dead_code,
    reason = "Complete C2 composite assembler consumes the reviewed section directory next"
)]
mod checkpoint_sections;

#[cfg(feature = "flow")]
mod route_receipt;
pub mod seed_map;
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "Private C2.3 model dispatch helper is qualified by its focused tests; runtime integration remains separate"
    )
)]
mod staff_dispatch;
#[cfg(feature = "flow")]
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "Private C2.3 resource lifecycle adapter is exercised by focused actual-Flow tests; coordinator acceptance remains separate"
    )
)]
mod staff_flow_lifecycle;
#[expect(
    dead_code,
    reason = "Private C2.2 provider is exercised by source conformance; Flow integration is a later instance"
)]
mod work_duration;

// Experimental adapter for actual Flow work; it remains crate-private while
// C2.2 transit, checkpoint and public-API joins are unfinished.
#[cfg(feature = "flow")]
mod c2_checkpoint_journal;
#[cfg(feature = "flow")]
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "Private C2.2 Flow bridge is exercised by focused actual-runtime tests; the paired runtime join is a later instance"
    )
)]
mod flow_bridge;

#[cfg(feature = "flow")]
#[doc(hidden)]
pub mod experimental_c2_replay {
    pub use crate::c2_checkpoint_journal::{restore_demo, save_demo, JournalError};
}

// Private C4.2 numeric implementation; no public API or sidecar promises.
#[expect(
    dead_code,
    reason = "Private cohort adapter is qualified by C4.2 integration tests; public API review remains open"
)]
mod metric_cohorts;
mod metrics;
#[expect(
    dead_code,
    reason = "Private residual adapter is qualified by C4.2 integration tests; public API review remains open"
)]
mod residuals;

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

// Private C4.3 outer IO adapter; pure default builds retain no Arrow IO dependency.
#[cfg(any(feature = "ipc", feature = "parquet"))]
mod arrow_output;
#[cfg(any(feature = "ipc", feature = "parquet"))]
#[expect(
    dead_code,
    reason = "Private C4.3 run/event adapter is qualified by integration tests; public API review remains open"
)]
mod sidecar_adapter;
