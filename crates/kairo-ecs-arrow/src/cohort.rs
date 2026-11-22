//! Versioned transport for individual trajectories and proximity observations.

use crate::arrow_c_data::{ArrowDataType, ArrowField};
use crate::ArrowError;
use kairo_ecs_rng::{
    derive_canonical_seed, derive_stream_id, DeterministicStream, RngCheckpoint,
    RNG_CHECKPOINT_VERSION_V1, RNG_DISTRIBUTION_PROVENANCE_V1, RNG_GENERATOR_CHACHA8,
};
use kairo_ecs_types::{EntityId, SimTime};
use polars::prelude::*;
use polars_arrow::datatypes::Metadata;
use std::collections::HashMap;

#[cfg(test)]
mod tests;

pub const TRANSITION_SCHEMA: &str = "kairo_ecs.cohort_transition.v1";
pub const EXPOSURE_SCHEMA: &str = "kairo_ecs.spatial_exposure.v1";

/// Physical duration of one simulation tick for the entire table.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TickUnit {
    Millisecond,
    Second,
    Minute,
    Hour,
    Day,
}

impl TickUnit {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Millisecond => "millisecond",
            Self::Second => "second",
            Self::Minute => "minute",
            Self::Hour => "hour",
            Self::Day => "day",
        }
    }
}

/// Address of a random draw group shared by the paired arms.
///
/// Assign the same slot to the same sampling opportunity in each arm.
/// Visit indices advance separately for each entity and source state.
/// Slots within a visit can distinguish draws or separate time intervals.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TrialAddress {
    pub entity: EntityId,
    pub from_state: u32,
    pub visit: u64,
    pub slot: u32,
}

impl TrialAddress {
    fn purpose(self) -> String {
        format!(
            "{TRANSITION_SCHEMA}/{}/{}/{}",
            self.from_state, self.visit, self.slot
        )
    }

    /// Open a fresh deterministic stream for this address and replication seed.
    pub fn stream(self, run_seed: u64) -> DeterministicStream {
        DeterministicStream::from_entity_purpose(run_seed, self.entity, &self.purpose())
    }

    fn matches_start(self, run_seed: u64, start: &RngCheckpoint) -> bool {
        let purpose = self.purpose();
        start.word_pos == 0
            && start.generator == RNG_GENERATOR_CHACHA8
            && start.version == RNG_CHECKPOINT_VERSION_V1
            && start.distribution_provenance == RNG_DISTRIBUTION_PROVENANCE_V1
            && start.stream_id == derive_stream_id(self.entity, &purpose)
            && start.seed == derive_canonical_seed(run_seed, self.entity, &purpose)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Transition {
    pub trial: TrialAddress,
    pub stratum_id: u32,
    pub to_state: u32,
    pub entered_at: SimTime,
    pub event_at: SimTime,
    /// Capture from `trial.stream(run_seed)` before the sampler draws.
    pub rng_start: RngCheckpoint,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpatialExposure {
    pub entity: EntityId,
    pub stratum_id: u32,
    pub observed_at: SimTime,
    pub spatial_cell_id: u64,
    pub radius_metres: f64,
    pub features_within_radius: u32,
    /// Distance to the nearest feature within the radius, when one exists.
    pub nearest_feature_metres: Option<f64>,
}

/// Build a typed frame and its exact C Data schema. Rows for an entity describe
/// successive state visits in input order. The first row can start at a
/// checkpointed visit. Times and derived dwell durations preserve all 128 bits.
pub fn transition_frame(
    rows: &[Transition],
    unit: TickUnit,
    run_seed: u64,
) -> Result<(DataFrame, ArrowField), ArrowError> {
    let mut previous: HashMap<EntityId, &Transition> = HashMap::new();
    let mut visits: HashMap<(EntityId, u32), u64> = HashMap::new();
    let mut dwells = Vec::with_capacity(rows.len());
    for row in rows {
        let trial = row.trial;
        let dwell = row
            .event_at
            .duration_since(row.entered_at)
            .ok_or(ArrowError::InvalidArgument("cohort dwell time is negative"))?;
        if trial.from_state == row.to_state {
            return Err(ArrowError::InvalidArgument("cohort state did not change"));
        }
        let previous_visit = visits.insert((trial.entity, trial.from_state), trial.visit);
        if previous.get(&trial.entity).is_some_and(|prior| {
            prior.event_at != row.entered_at || prior.to_state != trial.from_state
        }) || previous_visit.is_some_and(|visit| visit.checked_add(1) != Some(trial.visit))
        {
            return Err(ArrowError::InvalidArgument(
                "cohort visits are discontinuous or out of order",
            ));
        }
        if !trial.matches_start(run_seed, &row.rng_start) {
            return Err(ArrowError::InvalidArgument(
                "cohort RNG checkpoint does not match the trial address",
            ));
        }
        previous.insert(trial.entity, row);
        dwells.push(dwell.ticks());
    }

    let frame = df!(
        "entity_id_index" => rows.iter().map(|row| row.trial.entity.index).collect::<Vec<_>>(),
        "entity_id_gen" => rows.iter().map(|row| row.trial.entity.generation).collect::<Vec<_>>(),
        "stratum_id" => rows.iter().map(|row| row.stratum_id).collect::<Vec<_>>(),
        "from_state" => rows.iter().map(|row| row.trial.from_state).collect::<Vec<_>>(),
        "to_state" => rows.iter().map(|row| row.to_state).collect::<Vec<_>>(),
        "event_time_hi" => rows.iter().map(|row| upper_word(row.event_at.ticks())).collect::<Vec<_>>(),
        "event_time_lo" => rows.iter().map(|row| row.event_at.ticks() as u64).collect::<Vec<_>>(),
        "dwell_time_hi" => dwells.iter().copied().map(upper_word).collect::<Vec<_>>(),
        "dwell_time_lo" => dwells.iter().map(|&ticks| ticks as u64).collect::<Vec<_>>(),
        "visit_index" => rows.iter().map(|row| row.trial.visit).collect::<Vec<_>>(),
        "draw_slot" => rows.iter().map(|row| row.trial.slot).collect::<Vec<_>>(),
        "rng_substream_id" => rows.iter().map(|row| row.rng_start.stream_id).collect::<Vec<_>>()
    )
    .map_err(frame_error)?;
    let run_seed_text = run_seed.to_string();
    let schema = schema(
        &frame,
        TRANSITION_SCHEMA,
        unit,
        None,
        [
            ("kairo_ecs.run_seed", run_seed_text.as_str()),
            ("kairo_ecs.rng_profile", RNG_DISTRIBUTION_PROVENANCE_V1),
        ],
    );
    Ok((frame, schema))
}

/// Build a typed frame and exact schema from proximity queries in projected
/// metres. Entity observations are supplied in nondecreasing time order.
pub fn spatial_exposure_frame(
    rows: &[SpatialExposure],
    unit: TickUnit,
) -> Result<(DataFrame, ArrowField), ArrowError> {
    let mut previous = HashMap::new();
    for row in rows {
        if previous
            .insert(row.entity, row.observed_at)
            .is_some_and(|earlier| earlier > row.observed_at)
        {
            return Err(ArrowError::InvalidArgument(
                "spatial observations are out of order",
            ));
        }
        if !row.radius_metres.is_finite()
            || row.radius_metres < 0.0
            || row.nearest_feature_metres.is_some() != (row.features_within_radius > 0)
            || row.nearest_feature_metres.is_some_and(|distance| {
                !distance.is_finite() || distance < 0.0 || distance > row.radius_metres
            })
        {
            return Err(ArrowError::InvalidArgument("invalid spatial proximity"));
        }
    }

    let frame = df!(
        "entity_id_index" => rows.iter().map(|row| row.entity.index).collect::<Vec<_>>(),
        "entity_id_gen" => rows.iter().map(|row| row.entity.generation).collect::<Vec<_>>(),
        "stratum_id" => rows.iter().map(|row| row.stratum_id).collect::<Vec<_>>(),
        "event_time_hi" => rows.iter().map(|row| upper_word(row.observed_at.ticks())).collect::<Vec<_>>(),
        "event_time_lo" => rows.iter().map(|row| row.observed_at.ticks() as u64).collect::<Vec<_>>(),
        "spatial_cell_id" => rows.iter().map(|row| row.spatial_cell_id).collect::<Vec<_>>(),
        "radius_metres" => rows.iter().map(|row| row.radius_metres).collect::<Vec<_>>(),
        "features_within_radius" => rows.iter().map(|row| row.features_within_radius).collect::<Vec<_>>(),
        "nearest_feature_metres" => rows.iter().map(|row| row.nearest_feature_metres).collect::<Vec<_>>()
    )
    .map_err(frame_error)?;
    let schema = schema(
        &frame,
        EXPOSURE_SCHEMA,
        unit,
        Some("nearest_feature_metres"),
        [("kairo_ecs.length_unit", "metre")],
    );
    Ok((frame, schema))
}

fn upper_word(ticks: u128) -> u64 {
    (ticks >> 64) as u64
}

fn schema<'a>(
    frame: &DataFrame,
    name: &str,
    unit: TickUnit,
    nullable: Option<&str>,
    extra_metadata: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> ArrowField {
    let fields = frame
        .columns()
        .iter()
        .map(|column| {
            ArrowField::new(
                column.name().clone(),
                column.dtype().to_arrow(CompatLevel::oldest()),
                nullable == Some(column.name().as_str()),
            )
        })
        .collect();
    let metadata: Metadata = std::iter::once(("kairo_ecs.tick_unit", unit.as_str()))
        .chain(extra_metadata)
        .map(|(key, value)| (key.into(), value.into()))
        .collect();
    ArrowField::new(name.into(), ArrowDataType::Struct(fields), false).with_metadata(metadata)
}

fn frame_error(error: PolarsError) -> ArrowError {
    ArrowError::UnsupportedSchemaNegotiation(error.to_string())
}
