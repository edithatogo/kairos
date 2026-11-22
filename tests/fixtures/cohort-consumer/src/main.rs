#![forbid(unsafe_code)]

use kairo_ecs_abm::ContinuousSpatialIndex2D;
use kairo_ecs_arrow::arrow_c_data::{domain, PrimitiveArray};
use kairo_ecs_arrow::cohort::{
    spatial_exposure_frame, transition_frame, SpatialExposure, TickUnit, Transition, TrialAddress,
};
use kairo_ecs_arrow_ffi::native_tables::{import_batch, import_stream};
use kairo_ecs_rng::{CauseId, CompetingCause, CompetingHazards, HazardRate, TieBreaker};
use kairo_ecs_types::{EntityId, SimTime};
use std::error::Error;

#[cfg(test)]
mod tests;

const SEED: u64 = 753;
const ENTITIES: u64 = 4_096;
const START: f64 = 2.0;
const END: f64 = 6.0;
const BEFORE: [f64; 2] = [0.14, 0.08];
const BASELINE: [f64; 2] = [0.05, 0.25];
const INTERVENTION: [f64; 2] = [0.5, 0.25];
type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn hazards(after: [f64; 2]) -> Result<CompetingHazards> {
    if BEFORE
        .iter()
        .chain(&after)
        .any(|rate| !rate.is_finite() || *rate < 0.0)
    {
        return Err("hazard rates must be finite and nonnegative".into());
    }
    let causes = BEFORE
        .into_iter()
        .zip(after)
        .enumerate()
        .map(|(index, (before, after))| CompetingCause {
            cause_id: CauseId(index as u32 + 1),
            priority: 0,
            hazard: HazardRate::PiecewiseConstant {
                intervals: vec![(START, before)],
                trailing_rate: after,
            },
        })
        .collect();
    Ok(CompetingHazards::new(causes, TieBreaker::LowestCauseId))
}

fn ticks(seconds: f64) -> SimTime {
    // This fixture samples bounded ages in seconds through END.
    SimTime::from_ticks((seconds * 1_000.0).floor() as u128)
}

fn arm(seed: u64, after: [f64; 2], entities: u64) -> Result<Vec<Transition>> {
    let model = hazards(after)?;
    let mut rows = Vec::new();
    for index in 0..entities {
        for (slot, start, end) in [(0, 0.0, START), (1, START, END)] {
            let trial = TrialAddress {
                entity: EntityId::new(index, 7),
                from_state: 0,
                visit: 0,
                slot,
            };
            let mut rng = trial.stream(seed);
            let rng_start = rng.checkpoint();
            if let Some(event) = model
                .sample_next(start, Some(end), &mut rng)
                .filter(|event| !event.is_censored)
            {
                rows.push(Transition {
                    trial,
                    stratum_id: (index % 2) as u32,
                    to_state: event.winning_cause.0,
                    entered_at: SimTime::ZERO,
                    event_at: ticks(start + event.delta_time),
                    rng_start,
                });
                break;
            }
        }
    }
    Ok(rows)
}

fn spatial_rows() -> Result<[SpatialExposure; 2]> {
    let mut index = ContinuousSpatialIndex2D::new();
    index.insert(EntityId::new(1, 0), 3.0, 4.0)?;
    index.insert(EntityId::new(2, 0), -9.0, 0.0)?;
    let observe = |seconds, spatial_cell_id, x, y| -> Result<SpatialExposure> {
        let radius_metres = 8.0;
        let nearby = index.locate_within_distance(x, y, radius_metres)?;
        Ok(SpatialExposure {
            entity: EntityId::new(0, 7),
            stratum_id: 0,
            observed_at: ticks(seconds),
            spatial_cell_id,
            radius_metres,
            features_within_radius: u32::try_from(nearby.len())?,
            nearest_feature_metres: nearby.first().map(|(_, distance)| *distance),
        })
    };
    Ok([observe(0.0, 10, 0.0, 0.0)?, observe(END, 11, 80.0, 80.0)?])
}

fn main() -> Result<()> {
    let baseline = arm(SEED, BASELINE, ENTITIES)?;
    let policy = arm(SEED, INTERVENTION, ENTITIES)?;
    assert!(baseline
        .iter()
        .filter(|row| row.event_at < ticks(START))
        .eq(policy.iter().filter(|row| row.event_at < ticks(START))));
    assert_ne!(baseline.len(), policy.len());
    assert_eq!(policy, arm(SEED, INTERVENTION, ENTITIES)?);

    let (frame, schema) = transition_frame(&policy, TickUnit::Millisecond, SEED)?;
    let source_address =
        frame.column("entity_id_index")?.u64()?.cont_slice()?[1..].as_ptr() as usize;
    let stream = domain::export_stream(&frame, &schema, 1, Some(&schema))?;
    drop(frame);
    let mut reader = import_stream(stream)?;
    assert_eq!(reader.schema(), &schema);
    drop(reader.next().ok_or("missing first stream batch")??);
    let retained = reader.next().ok_or("missing second stream batch")??;
    drop(reader);
    let entities = retained.values()[0]
        .as_any()
        .downcast_ref::<PrimitiveArray<u64>>()
        .ok_or("unexpected entity column type")?;
    assert_eq!(entities.value(0), policy[1].trial.entity.index);
    assert_eq!(
        entities.values().as_slice().as_ptr() as usize,
        source_address
    );
    drop(retained);

    let (frame, schema) = spatial_exposure_frame(&spatial_rows()?, TickUnit::Millisecond)?;
    let batch = domain::export_batch(&frame, &schema, Some(&schema))?;
    drop(frame);
    let (actual, exposure) = import_batch(batch)?;
    assert_eq!(actual, schema);
    assert_eq!(exposure.len(), 2);
    println!(
        "cohort fixture passed with {} baseline, {} paired policy and {} exposure rows",
        baseline.len(),
        policy.len(),
        exposure.len()
    );
    Ok(())
}
