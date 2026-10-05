# C2.3 spatial experimental API adoption and format v1

Coordinator approves combined route/time implementation in `pub mod spatial`
under existing kairo-ecs-abm. No crate-root aliases, new dependencies, DES or
calibration changes are authorized. Preserve every exact C20 signature/getter.
This resolves the [independent review](../../evidence/c2.3-spatial-api-20261005/review.json),
not runtime, full C2.3 or stable-release acceptance. Source base is2f69a39.

## Canonical topology bytes (exact format)

In this exact order, with no trailing bytes:

1. Literal ASCII `KAIROS-TRANSIT-GRAPH` followed by one NUL byte.
2. Graph/schema version u32 little-endian (only version1 supported).
3. Literal ASCII `mm` followed by one NUL byte.
4. Node count u64 little-endian; ascending numeric NodeIds as u64 little-endian.
5. Edge count u64 little-endian; edges in ascending numeric EdgeId order.
6. Per edge: id, from, to, length_mm, each u64 little-endian; allowed-mode count
   u64 little-endian; modes sorted by exact UTF-8 bytes, each encoded as byte
   length u64 little-endian then its exact UTF-8 bytes.

Check count/length conversions; overflow is TransitError::Overflow. Zero IDs
are data, not sentinels. No Unicode normalization, case folding, caller hash,
profile, speed, tick rate or runtime identity is encoded in topology bytes.
The new conformance golden fixes the bytes independently of the implementation.

## MovementModeId validation

Accept exact UTF-8 strings of1..1024bytes inclusive. Reject Unicode control
characters (Rust char::is_control) and leading/trailing Unicode whitespace
(Rust str::trim). Internal non-control whitespace is valid. Do not normalize or
case-fold. InvalidMovementMode reports any violation. Composed/decomposed
Unicode, and case differences, remain distinct identities. New external fixture
covers empty/control/boundary whitespace, byte-length boundaries and retention.
No site-specific mode catalog is imposed.

## Other review dispositions and execution gates

- Export only `kairo_ecs_abm::spatial`; immutable topology/profile/route state
  may Clone, with private RoutePlan fields and read-only frozen getters.
- Preserve deterministic (distance, hop count, full edge sequence) ordering,
  simple paths, checked u128 arithmetic and cumulative ceiling offsets.
- Keep original nine-test C20 fixture/runner immutable. Nine distinct named
  passing lines and the exact9-pass single-target summary imply once each;
  saved logs/hashes must still be inspected.
- Run the separate external `conformance/c23/spatial_api_smoke.rs` against actual
  production exports before API acceptance. It checks omitted getters, byte
  golden and movement identity rules. Never infer these from worker reports.
- Default1.76 native tests and canonical1.99 strict clippy/formatting, actual
  nine-case fixture, independent source/algorithm review and hosted exact-head
  CI remain mandatory. Missing tooling is unverified.

Route/time is one source family because actual RoutePlan timing is part of the
frozen routing interface. C2.3.time independently reviews numeric behavior on
that same commit. Progress/carriers and dispatch are later separate work.
Full C2.2 resume/owned streams, paired C2.1, portable checkpoints and ED MVP remain
open. A development experiment does not create a stable semver baseline.
