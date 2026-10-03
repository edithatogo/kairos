# Calibration seed-map v1: reviewed implementation contract

Coordinator design decision, 2026-10-03, following independent Track01 RNG
source review and independently computed Python golden vector. This closes the
encoding design question for this version; executable conformance remains
required before accepting a capability. It does not replace the engine RNG.
Older C0 pending-derivation text is superseded for this reviewed v1 design only.

## Normative byte encoding

Domain exactly ASCII `kairos.calibration.seed-map`, immediately followed by u32
little-endian version1, u64LE rootseed, u64LE replicationid, then each of study_id,
seed_schedule_id, case_key, task_key as u64LE UTF8 byte length + exactUTF8bytes,
then u32LE purpose (service1, transit2, behavior3, calibration4). No delimiter,
case-fold, trimming or Unicode normalization. Domain is a fixed constant, not a
user-provided prefix. IDs are nonempty, <=1024UTF8bytes each, without leading/
trailing whitespace or control characters. Upstream owns canonical identifiers;
normalization-free exactbyte identity is normative, including non-ASCII. Only
pseudonymous synthetic case keys are allowed; no raw patient IDs, wall-clock,
thread/worker order or candidate identity hidden in the tuple.

SHA256 over all encoded bytes; digest bytes0..8 interpreted as u64LE. Explicit
rootseed may be zero. Shared seed_schedule_id provides paired candidate draws;
independent candidate runs explicitly select distinct schedule IDs. Purpose
streams are independent; transit or behavior draws never advance service state.
A 64bit result cannot guarantee absence of collisions. Register identities in a
study/run seed registry; if distinct identities produce the same u64 seed, fail
with a typed collision error. Duplicate registration of the same identity is
idempotent. This is finite-run detection, not a global collision-free guarantee.

## Stream and owned resume boundary

Reuse existing DeterministicStream::from_seed unchanged. Normative stream version1
is the current repeated `state=splitmix64(state)` recurrence, not an additive
counter. Wrapper draw_position:u64 counts completed next_u64 transitions;
next_u32 consumes one transition and returns low32bits. Preflight checked increment
before calling RNG so overflow leaves state/position unchanged.

In-memory owned snapshot preserves identity/rootseed, seed-map version1, stream
version1, derived seed, private current state and draw position. Opaque snapshots
are created by the wrapper and cannot be forged through public fields; restoring
this owned snapshot must reproduce the next draw, not restart from originalseed.
No public arbitrary-state constructor or serialization codec. Track22 owns
portable validation/codec; no untrusted-state restoration claim. Tests inside
module may inject counter/version faults; unknown versions fail explicitly.

## Independent golden

study=`study-α`, schedule=`crn-v1`, rootseed1234, replication7,
case=`case-0001`, task=`triage:1`, servicepurpose1. Encodedlength114bytes.
Hex: `6b6169726f732e63616c6962726174696f6e2e736565642d6d617001000000d2040000000000000700000000000000080000000000000073747564792dceb1060000000000000063726e2d76310900000000000000636173652d3030303108000000000000007472696167653a3101000000`
SHA256: `fba126d77ad6094874929e96c01d9b25adf9f6801235af165fc7c3d6fae930a2`.
Derived seed5190915868605194747. First draws hex
`cabf5867c66cf8ef`, `a20d8e837c7bea4e`, `ee85ff56f99bd5b4`.

Package is optional Rust1.88 floor, latest reviewed sha2=0.11.0 (MSRV1.85,
MIT/Apache2, nonyanked) and existing thiserror=2.0.20 shared with legacypackages.
Engine/package Rust1.76 contracts remain unchanged. Actual resolved package
floor builds and policy review are required; no universal graph floor claim.
No empirical duration, mode, route, simulation output or full C2 acceptance is
established by seed fixtures or this foundation package.
