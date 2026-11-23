//! Forward-only resource occupancy from the staged DES runtime.

use crate::StagedRuntimeError;
use kairo_ecs_types::{EntityId, EventId, SimTime};
use std::num::NonZeroU128;

/// Version of the committed staged-resource usage contract.
pub const STAGED_USAGE_VERSION: u16 = 1;

/// Provenance and clock scale declared when the runtime is created.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(
    feature = "checkpoint-json",
    derive(serde::Serialize, serde::Deserialize),
    serde(deny_unknown_fields)
)]
pub struct StagedUsageSource {
    /// Stable run identifier.
    pub run: String,
    /// Scenario identifier.
    pub scenario: String,
    /// Deterministic branch of the model run.
    pub branch: String,
    /// Exact engine source version.
    pub engine_revision: String,
    /// Exact model version.
    pub model_revision: String,
    /// Declared RNG profile, including an explicit deterministic/no-RNG profile.
    pub rng_revision: String,
    /// Name of the time unit used by the model.
    pub time_unit: String,
    /// Number of simulation ticks in one declared time unit.
    pub ticks_per_time_unit: NonZeroU128,
}

impl StagedUsageSource {
    pub(crate) fn validate(&self) -> Result<(), StagedRuntimeError> {
        for value in [
            &self.run,
            &self.scenario,
            &self.branch,
            &self.engine_revision,
            &self.model_revision,
            &self.rng_revision,
            &self.time_unit,
        ] {
            if !valid_usage_label(value) {
                return Err(StagedRuntimeError::InvalidUsageSource);
            }
        }
        Ok(())
    }
}

pub(crate) fn valid_usage_label(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.trim() == value
        && !value.chars().any(char::is_control)
}

/// A scheduler dispatch that admitted or released an occupancy interval.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UsageBoundary {
    /// Time of the actual dispatch.
    pub at: SimTime,
    /// Causal scheduler event identifier.
    pub event: EventId,
}

/// An occupied portion of a committed observation window.
///
/// The lease key binds run, scenario, branch, resource, request and lease
/// revision. Add the containing window's start and end for a slice key.
/// Scheduler and entity IDs retain their generation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OccupancySlice {
    /// Actor holding the units.
    pub actor: EntityId,
    /// Original request identity within the run.
    pub request: EntityId,
    /// Revision of the holding lease.
    pub lease_revision: u64,
    /// Resource units held for the entire interval.
    pub quantity: u64,
    /// Actual admission dispatch.
    pub admission: UsageBoundary,
    /// Actual release observed by the requested endpoint, if present.
    pub release: Option<UsageBoundary>,
    /// Inclusive start of the intersection.
    pub start: SimTime,
    /// Exclusive end of the intersection.
    pub end: SimTime,
}

/// Deterministic half-open window of committed occupancy for one resource.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommittedUsageWindow {
    /// Contract version for downstream adapters.
    pub version: u16,
    /// Bound run and clock identity.
    pub source: StagedUsageSource,
    /// Resource holding the units.
    pub resource: EntityId,
    /// Physical quantity unit bound to the resource at creation.
    pub quantity_unit: String,
    /// Window start, inclusive.
    pub start: SimTime,
    /// Window end, exclusive.
    pub end: SimTime,
    /// Overlapping slices in request order. An absent release is right-censored.
    pub occupancy: Vec<OccupancySlice>,
    /// Sum of held resource units times overlapping simulation ticks.
    pub occupied_unit_ticks: u128,
}

impl CommittedUsageWindow {
    pub(crate) fn add(&mut self, slice: OccupancySlice) -> Result<(), StagedRuntimeError> {
        let ticks = slice.end.ticks() - slice.start.ticks();
        self.occupied_unit_ticks = ticks
            .checked_mul(u128::from(slice.quantity))
            .and_then(|count| self.occupied_unit_ticks.checked_add(count))
            .ok_or(StagedRuntimeError::UsageOverflow)?;
        self.occupancy.push(slice);
        Ok(())
    }
}
