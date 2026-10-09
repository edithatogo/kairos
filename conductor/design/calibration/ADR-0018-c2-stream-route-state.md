# ADR-0018 — Exact stream and route owner state

Date2026-10-09. Status: coordinator experimental component contract under the
continuing user C2 authority. Extends ADR-0016/0017; full Flow/outer restore and
fresh-process C2 matrix remain required. No complete portable acceptance.

## Owned calibration stream — Track21

Add a crate-private complete native version1 state image for CalibrationStream:
full SeedIdentity, seed-map/stream versions, derived seed, exact current RNG state
and completed draw position. Do not restart/replay from root seed. Capture checks
aggregate identity-byte bounds before cloning. Import takes the owner-derived
expected CalibrationStreamKey and limits, validates schema, both versions and
identity/rederived seed using existing opaque snapshot checks, then restores the
exact saved state/position. A zero-draw image must have current state=derived seed.
Preserve all purpose variants; never turn Service into Transit/Behavior/etc.
Existing public CalibrationSeedError and snapshot APIs/algorithm remain unchanged;
new state/errors are crate-private. Imported current state is data bound by the
outer integrity contract, not cryptographic proof of full RNG history. Do not
invent unbounded draw-history replay for validation or claim authenticity.

Tests advance varied next_u64/next_u32 prefixes, transport captured native fields,
restore against complete matching identity and compare many subsequent draws and
positions to uninterrupted controls. Include wrong logical identities/purposes,
versions/derived seeds/zero-draw state and bounds, source unchanged on rejection.
Byte serialization and bridge binding are separate next steps.

## Immutable route and progress — Track03 ABM

Add owner-defined native version1 route image with every original plan field:
origin/destination, ordered segments/edge endpoints/distances/start-end offsets,
total distance/duration, movement mode/speed, tick rate, graph version and canonical
bytes. Capture bounds complete segment/geometry/mode payload before cloning.
Import validates against a caller-supplied approved immutable TransitModel graph;
recompute its deterministic route solely as a validation oracle and require exact
plan equality. This is geometry validation, not simulation-prefix replay. Reject
unknown schema, changed topology/version/mode/speed/ticks/OD/path/distance/time,
invalid units and all configured count/byte bounds; preserve source untouched.
New checkpoint errors remain separate from existing public TransitError.

TransitProgressState gains a crate-private native cursor image: the complete route
image, actual segment index and elapsed-in-segment. Validate/restage the immutable
plan, then restore that exact cursor via current validation. Never reset or advance
from zero to imitate saved progress. Cover mid-edge, exact edge boundaries,
zero-tick segments, completed and invalid cursors with exact remaining/useful time
and next-edge transitions. No new speed/RNG/tie/rounding algorithm.

Geometry state import does not rebind TransitContext runtime/work/tickets; that
complete ABM context codec follows the Flow codec interface. Do not claim whole
transit/Flow/process restart from this geometry layer. Source schema uses all C2
required legal routes; no fixed graph/recipe restriction is introduced. Engine
core stays hash/serde free; no dependency or manifest changes in this leaf.

## Integration

Disjoint leaf paths; exact bases/ADRs and one-writer leases; Rust1.99-only targeted
and full affected-package gates, strict Clippy/format and retained command/source
hash logs. New payloads feed the complete context/bridge codec and durable outer
checkpoint. Whole state may be exposed only after compatibility/reference/budget
preflight and coherent frontier checks. Unknown contexts/aliases fail closed.
