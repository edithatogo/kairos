# Cohort export API review

[ADR 0015](../adr/0015-cohort-and-exposure-schemas.md) accepts the versioned
schemas and draw-address contract.

Track 116 adds `kairo_ecs_arrow::cohort` behind `cohort-native`. Use
`transition_frame` or `spatial_exposure_frame` to build a typed Polars
frame and its declared Arrow struct field. The existing trusted domain C Data
API exports the declaration and frame as an owned batch or bounded stream.
The default Arrow, native importer and minimal engine profiles retain their
existing dependency graphs.

## Version 1 fields

Both schemas use the full generational entity identifier and preserve input
row order. An unsigned 128-bit value is reconstructed as
`(u128::from(hi) << 64) | u128::from(lo)`. Every table declares
`kairo_ecs.tick_unit` on its struct field. Whole ticks can represent
milliseconds, seconds, minutes, hours or days.

| Schema | Required fields in order | Nullable fields |
|---|---|---|
| `kairo_ecs.cohort_transition.v1` | `entity_id_index` UInt64, `entity_id_gen` UInt32, `stratum_id` UInt32, `from_state` UInt32, `to_state` UInt32, `event_time_hi` and `event_time_lo` UInt64, `dwell_time_hi` and `dwell_time_lo` UInt64, `visit_index` UInt64, `draw_slot` UInt32, `rng_substream_id` UInt64 | None |
| `kairo_ecs.spatial_exposure.v1` | `entity_id_index` UInt64, `entity_id_gen` UInt32, `stratum_id` UInt32, `event_time_hi` and `event_time_lo` UInt64, `spatial_cell_id` UInt64, `radius_metres` Float64, `features_within_radius` UInt32 | `nearest_feature_metres` Float64, last field |

State, stratum and cell codes belong to the caller. Transition input describes
successive visits for each entity and may begin at a checkpoint. Each visit
exits to a different state. The previous exit time equals the next entry time,
and the previous destination equals the next origin. Same-tick transitions
between distinct states are valid. Dwell is checked by subtracting entry from
exit before conversion to limbs. Retain context across chunks if a consumer
needs to validate continuity across separately built tables.

Spatial observations occur in nondecreasing time per entity. The search radius
and nearest distance use projected metres. The nearest distance is present
exactly when the count is positive, and must be finite and within the search
radius. The geometry and projection belong to the caller's spatial frame.

## Paired trajectories

Use identical run seed, generational entity ID, origin state, visit index and
draw slot for the same opportunity in both arms. The visit index counts visits
to that entity's origin state, so extra visits to other states leave the common
state's address intact after reconvergence. The versioned purpose passed to
the existing entity-purpose RNG is
`kairo_ecs.cohort_transition.v1/<from_state>/<visit>/<slot>`. Store the
checkpoint captured before the draw. The exporter verifies its seed
derivation, stream ID, word position, generator, version and distribution
profile. The table records `kairo_ecs.run_seed` and `kairo_ecs.rng_profile`.
Share the same ordered set of cause identifiers at paired opportunities.
Represent a deactivated cause with zero hazard to preserve draw assignment.

The fixture exercises an existing piecewise competing-risk sampler and
continuous spatial query. Its independent oracle integrates constant
cause-specific hazards on either side of a dwell-age breakpoint and compares
them with incidence and survival from the imported trajectory rows. Its
paired arms check equal pre-policy rows, a reconverged trial following an
extra visit and the empirical variance of paired incidence differences.

## Compatibility and ownership

Readers negotiate the complete field, including nullability and metadata.
Changing a tick unit requires explicit conversion by the caller. Existing
event-log schemas, C exports and signatures are unaffected. Bounded C Stream
windows retain single-chunk primitive storage. The fixture tests the effective
address of a nonzero-offset window after the frame and reader are dropped.

`tests/fixtures/cohort-consumer` has its own Cargo workspace and lock. It uses
only fictional source inputs and checks the actual native importer. Installed
downstream adoption uses the consuming repository's own pin and qualification.
