# ADR-0010: Optional C2 route metadata in the Flow bridge

**Status:** accepted for this bounded internal experiment; no stable API or
checkpoint compatibility promise. C2, Track03, Track21, Track22, hosted,
release, and clinical acceptance gates remain open.

## Decision

`WorkPreparationInput` may carry optional, explicitly versioned `RouteMetadata`
through a nonbreaking builder. Version 1 requires a validated trip-purpose
string and `ConfiguredGeometry` provenance. Unsupported versions, invalid
purpose values, and `SensorObservationOnly` fail before fidelity admission or
Service-stream sampling. Purpose is not inferred from the subsystem or
`SeedPurpose`; seed identities and draw positions are unchanged.

For a routed Micro request, the bridge creates and records the carrier without
scheduling a transit event, then creates the receipt from that actual carrier's
immutable route plan. It validates the actual carrier context through the
private adapter getter before scheduling and at dispatch, control,
continuation capture, and finish boundaries. The retained receipt is part of
the bound work and survives same-live continuation capture/resume. Metadata is
prevalidated before admission and Service sampling. An unexpected receipt
construction or context-read failure leaves the carrier owned by the bound
work and schedules no event, so the caller can retry without losing or
duplicating the carrier; it does not advance the Service stream or mutate
request association.

Legacy callers that omit metadata remain supported and explicitly
unreceipted. The bridge does not claim those calls satisfy the annotated route
identity contract. Macro and zero-duration Micro paths do not create a transit
carrier, route receipt, route event, or Transit RNG draw. A zero-duration route
is normalized to the existing zero-transit path. Multi-profile routes remain
unsupported.

## Consequences and boundaries

- Receipt integrity is checked against the immutable plan held by the live
  carrier, never a caller-provided digest or copied request topology.
- The receipt is private Flow-bridge evidence, not an authenticity signature,
  checkpoint codec, portable recovery protocol, or proof of physical travel.
- Same-live continuation is supported; reconstruction across a process or
  runtime boundary is outside this leaf. Track22 portable recovery and C5/E3
  requirements remain independent and open.
- After an accepted Resume replaces the pending movement event, the previous
  event is recorded as stale. Its later dispatch is consumed once as ignored;
  it cannot replace the resumed arrival or alter route progress.
- No manifest, lockfile, frozen C20 signature, or fixture changes are allowed.

## Verification

Local Rust 1.99.0 checks cover metadata rejection before admission and Service
draw, actual annotated carrier validation through pause/resume/arrival and
continuation, mismatch rejection with state preservation, and Macro/zero
parity. The lifecycle oracle pauses at tick 1, resumes at tick 3, consumes the
replaced event as stale at tick 5, and arrives at tick 7 with five useful
movement ticks and two paused ticks. The fixture uses root seed 19.

Executed from `/Users/doughnut/Documents/careops-sim-session-worktrees/kairos-c2-route-metadata-20261009`, base `dbe7c34c26aceac4dd2191e5758ed1f25234261b`:

- `CARGO_HOME=/tmp/c2-route-receipt-cargo-home cargo test -p kairo-ecs-calibration --features flow -F kairo-ecs-abm/test-support` — exit 0; 103 passed, 2 ignored. Log: `.artifacts/c2-route-metadata/calibration-test.log`, SHA-256 `0ebb6d99659f5e59eeb16fbb92000c9bb8b780a67778a1581247c591019aaede`.
- `CARGO_HOME=/tmp/c2-route-receipt-cargo-home cargo test -p kairo-ecs-abm --features test-support actual_flow_pause_resume_retains_route_and_submits_one_arrival_acquire` — exit 0; 1 passed. Log: `.artifacts/c2-route-metadata/abm-test.log`, SHA-256 `fee1f5bd2da22e53e53324f4c29a5925a91b6ce513058c254eb1372777a9820f`.
- `CARGO_HOME=/tmp/c2-route-receipt-cargo-home cargo clippy -p kairo-ecs-calibration --all-targets --features flow -F kairo-ecs-abm/test-support -- -D warnings` and `cargo fmt --all --check` — exit 0. Logs: `.artifacts/c2-route-metadata/clippy.log`, SHA-256 `f1f89e976fc6b578a0001c48d93f1fe69fc7562dfbcaf8c964332b1b756dfcc4`; `.artifacts/c2-route-metadata/fmt.log`, SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`.
- The pre-fix chronology regression failed on the replaced t=5 event as expected; the corrected test passes. Logs: `.artifacts/c2-route-metadata/lifecycle-red.log`, SHA-256 `d952a015726e29e4afd8069c507ad59aab6af6403fea03d751c5aa9e3f223725`; `.artifacts/c2-route-metadata/lifecycle-green.log`, SHA-256 `173d566cb1cb900086f0c7498a068d4e08a9b2e61d432842c94339b3df63b4d5`.

These checks do not close a phase gate or constitute maintainer acceptance.
