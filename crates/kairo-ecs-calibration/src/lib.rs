//! Optional calibration primitives; no empirical or clinical acceptance.
#![forbid(unsafe_code)]

#[expect(
    dead_code,
    reason = "Private C3 interfaces are being joined by bounded owner packets"
)]
mod shadow;
#[expect(
    dead_code,
    reason = "Private C3 observed replay join is under qualification"
)]
mod shadow_ledger;
#[cfg(feature = "flow")]
#[expect(
    dead_code,
    reason = "Private C3 native adapter is qualified by focused owner fixtures"
)]
mod shadow_native;
#[cfg_attr(
    not(all(test, feature = "flow")),
    expect(
        dead_code,
        reason = "Private C3 probe pool join is under qualification"
    )
)]
mod shadow_pool;
#[cfg_attr(
    not(all(test, feature = "flow", any(feature = "ipc", feature = "parquet"))),
    expect(
        dead_code,
        reason = "Private C3 residual projection join is under qualification"
    )
)]
mod shadow_report;
#[cfg(feature = "flow")]
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "Private C3 portable recovery join is under qualification"
    )
)]
mod shadow_wire;

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
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "Private C2 composite transaction is exercised by focused owner tests"
    )
)]
mod c2_portable_checkpoint;

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

#[cfg(all(test, feature = "flow"))]
#[path = "../../../conformance/c20/paired_flow_c20_v2.rs"]
mod paired_flow_c20_v2;

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
    not(any(test, feature = "flow")),
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

#[cfg(any(feature = "ipc", feature = "parquet"))]
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "Private C3 sidecar join awaits phase acceptance")
)]
mod shadow_sidecars;

#[cfg(feature = "flow")]
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "Private synthetic C3 native qualification model")
)]
mod shadow_c2_model;
#[expect(
    dead_code,
    reason = "Private C3 runner is undergoing native qualification"
)]
mod shadow_runner;

#[cfg(all(test, feature = "flow"))]
mod shadow_c03;

#[cfg(feature = "flow")]
mod c5_native;
#[cfg(feature = "flow")]
mod c5_study;
#[cfg(feature = "flow")]
#[doc(hidden)]
pub mod experimental_c5 {
    #[derive(Debug, thiserror::Error)]
    pub enum C5Error {
        #[error("C5 invalid input: {0}")]
        InvalidInput(String),
        #[error("C5 execution: {0}")]
        Execution(String),
    }
    fn parse(s: &str) -> Result<serde_json::Value, C5Error> {
        if s.len() > 16 * 1024 * 1024 {
            return Err(C5Error::InvalidInput("input byte limit".into()));
        }
        serde_json::from_str(s).map_err(|e| C5Error::InvalidInput(format!("invalid JSON: {e}")))
    }
    fn output(v: Result<serde_json::Value, String>) -> Result<String, C5Error> {
        serde_json::to_string(&v.map_err(C5Error::InvalidInput)?)
            .map_err(|e| C5Error::Execution(e.to_string()))
    }
    pub fn plan(study: &str) -> Result<String, C5Error> {
        output(crate::c5_study::plan(&parse(study)?))
    }
    pub fn summarize(study: &str, attempts: &str) -> Result<String, C5Error> {
        let a = parse(attempts)?;
        output(crate::c5_study::summarize(
            &parse(study)?,
            a.as_array()
                .ok_or_else(|| C5Error::InvalidInput("attempts must be array".into()))?,
        ))
    }
    pub fn evaluate(study: &str, task: &str) -> Result<String, C5Error> {
        let study = parse(study)?;
        let task = parse(task)?;
        crate::c5_native::admitted(&study, &task).map_err(C5Error::InvalidInput)?;
        let value = crate::c5_native::evaluate(&study, &task).map_err(C5Error::Execution)?;
        serde_json::to_string(&value).map_err(|e| C5Error::Execution(e.to_string()))
    }
    pub fn verify_trace(study: &str) -> Result<String, C5Error> {
        output(crate::c5_native::verify_trace(&parse(study)?))
    }
    pub fn replay(study: &str, mode: &str) -> Result<String, C5Error> {
        let study = parse(study)?;
        crate::c5_study::plan(&study).map_err(C5Error::InvalidInput)?;
        if mode != "shadow" && mode != "free" {
            return Err(C5Error::InvalidInput("unsupported replay mode".into()));
        }
        let value = crate::c5_native::replay(&study, mode).map_err(C5Error::Execution)?;
        serde_json::to_string(&value).map_err(|e| C5Error::Execution(e.to_string()))
    }
}
