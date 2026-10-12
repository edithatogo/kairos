# ADR-0009: Private C2 route receipt v1

**Status:** accepted for this bounded internal experiment; no stable API or
checkpoint compatibility promise. C2, Track03, Track21, Track22, hosted, release,
and clinical acceptance gates remain open.

## Decision

Calibration may create a private v1 receipt only from the immutable `RoutePlan`
retained by an actual `TransitContext`. The ABM carrier exposes a doc-hidden
read-only `route_plan()` accessor for this adapter. It does not expose mutable
route state, accept a caller digest, or establish a semver commitment. The
receipt validator recomputes from the carrier's actual plan and compares both
canonical bytes and digest. It must not validate a copied transit request or
trust a caller-provided hash.

The domain-separated receipt encoding binds, in fixed little-endian and
length-framed form: receipt schema/version; separately declared trip-purpose
string; distance provenance; canonical graph bytes and graph version; exact
movement mode and speed; tick rate; origin and destination; route distance and
duration; and ordered segment IDs, endpoints, lengths, and cumulative tick
offsets. SHA-256 is applied in the calibration adapter using its existing
dependency. The geometry crate remains free of SHA dependencies and continues
to provide the canonical topology bytes.

Trip purpose is a route-use label, not `SeedPurpose`; it is validated for
nonempty bounded UTF-8 without control characters or edge whitespace. The
purpose field and every route field contribute to the digest, so another purpose
has a distinct identity. Existing seed-purpose values, logical stream keys,
derivation, and draw positions remain untouched.

Only `ConfiguredGeometry` is admissible as execution distance provenance.
`SensorObservationOnly` is rejected: a sensor-derived distance alone does not
define a configured route or speed. Mixed-use pauses mean explicit external
activity interrupts movement; paused wall time contributes no travel. This
receipt supports one movement profile per route. Multi-profile routes and
sensor-derived execution ground truth remain unsupported.

## Consequences

- Canonical topology order follows the existing graph byte contract. Equivalent
  input order produces the same receipt; changed topology, profile, tick rate,
  origin/destination, purpose, or route segments fails validation.
- Receipt identity is an integrity/reproducibility aid, not authenticity,
  provenance verification, a checkpoint codec, or proof of physical travel.
- The implementation remains crate-private and feature-gated with Flow. No
  manifest, dependency, lockfile, seed-map, or existing fixture changes are
  needed.
- Checkpoint serialization, runtime rebinding, general recovery, candidate
  worker equivalence, and portable state ownership remain separate Track22/C5/
  E3 work and gates.

## Bounded verification

The implementation adds tests for canonical input-order invariance, changed
topology/profile/origin/tick-rate rejection, distinct trip-purpose identity,
sensor-only rejection, and validation of an actual positive route after a
Flow pause/resume. The pause/resume oracle checks cursor progress and remaining
travel so paused wall time is not counted. These tests are local evidence only;
they do not close any C2 phase or owner acceptance gate.
