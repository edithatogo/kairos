# ADR 0015 — Cohort trajectories and spatial exposure

## Status

Accepted for the Track 116 provider and standalone fixture after independent
review on 23 November 2026. Installed adoption belongs to the receiving
application.

## Decision

Add separate versioned Arrow struct schemas for state transitions and proximity
observations. They share generational entity IDs, ordered high and low limbs
for 128-bit ticks, and a required tick-unit declaration. Splitting time avoids
truncation at the Arrow boundary. Numeric state, stratum and spatial cell
codes let each consumer retain its own model and geography.

Each transition carries the substream ID for a stable draw address, defined by
run seed, full entity ID, source state, state visit and slot. Export checks the
starting checkpoint against that address. Proximity rows express projected
metres, a bound, a count and a nullable nearest distance. The
[API review](../design/track116-cohort-export-api-review.md) defines field
ordering, nullability, metadata and the versioned address namespace.

## Compatibility and review

The opt-in `cohort-native` feature uses the existing Polars producer and
trusted Arrow import path. Plain event-log exports and their negotiated
schemas retain their current public contracts. A consumer requests the full
field, including unit and provenance metadata. Any incompatible field change
requires a new schema version and a qualified receiving adapter.

Review found that an entity-wide visit counter would shift the draw on return
to a common state. The accepted per-state counter separates that state from
extra visits elsewhere. Paired hazards also share their ordered cause
identifiers. The fixture verifies a reconverged sampled exit after an extra
transition and checks analytic outcomes using the actual imported rows.

The [standalone conformance fixture](../../tests/fixtures/cohort-consumer/README.md)
also verifies rejection of inconsistent provenance and schema, full-width
time and the retained lifetime of a nonzero-offset primitive buffer.
