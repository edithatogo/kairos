# Track 68: Inter-Unit Boundary Channels & Asynchronous Handover Sinks

## Objective

Introduce SimPy/DES-native asynchronous cross-boundary transit buffers (`BoundaryChannel`, `EgressSink`, `HandoverPayload`) to `crates/kairo-ecs-des`. Enables modular simulation components to transfer entities across service boundaries with stochastic transit times, handover communication delays, and receiver buffer capacity backpressure / boarding holds without requiring tight coupling or synchronous shared state across units.

## Inputs

- `crates/kairo-ecs-des` process and resource primitives from Track 64.
- Optional observability adapter to `crates/kairo-ecs-streaming`; the first single-process fixture uses the existing scheduler.
- Tracks 01, 03, 64 contracts.

## Outputs

- `crates/kairo-ecs-des/src/boundary.rs`
- `crates/kairo-ecs-des/src/egress.rs`
- `docs/design/boundary-channels.md`
- `conductor/tracks/68-inter-unit-boundary-channels-handover-sinks/`

## Owned paths

- `crates/kairo-ecs-des/`
- `crates/kairo-ecs-streaming/`
- `docs/design/boundary-channels.md`
- `conductor/tracks/68-inter-unit-boundary-channels-handover-sinks/`

## Blocked paths

- `crates/kairo-ecs-core/` (owned by Track 67).
- `crates/kairo-ecs-gpu/` (owned by Track 32).
- `crates/kairo-ecs-pdes/` (owned by Track 34/46).

## Release implications

Proposed additive DES extension; compatibility is an acceptance gate, not an assumption. Require conformance tests to preserve existing single-unit DES behaviour. Adds inter-unit decoupled queuing primitives.

## Composition contract refinement — 20 October 2026

Transfer is a versioned lifecycle with operation identity, ownership and explicit resource-release points. Receiver reservation, physical occupancy and responsibility are distinct. Require cancellation/expiry, retries, delayed acknowledgements and compensation after departure, not just an EgressSink callback. Start on one scheduler in one process; streaming is observability and a later transport adapter, not authoritative state. Generic envelopes carry opaque domain payloads; clinical acceptance, boarding policy, handover meaning and birth/completion callbacks belong downstream.

See [owner worklist and downstream join](../../design/careops-composition-plan-20261020.md). All work remains pending. Prospective source filenames are resolved at dispatch; they are not evidence that APIs exist.

## Acceptance and cross-owner gates

Require the phase test matrix, meaningful negative controls, exact input/source/output hashes, deterministic replay and failure/recovery receipts. Coordinate Arrow modifications through 04/62 and shared scheduler/public contracts through 01/25. Current broad registry paths are prospective: reserve bounded disjoint files and rebind on source drift. Publication, hardware runtime and clinical validation remain separate.

## Dependencies

Bind immutable source and only the scheduler/RNG/identity capabilities consumed
from 01/03/64. Their status labels cannot waive consumer conformance. Track66
interfaces are conditional on actual experiment use; no blanket completion gate.

## Parallel-safe tracks

Planning can proceed in disjoint files. Runtime changes require exact-file owner
handoffs: 67 overlaps existing core/state/ABM owners; 68 and69 overlap DES. Shared
scheduler/registry changes are serialized. A broad owned-path list is not a lease.

## Acceptance criteria

Pass the neutral negative controls and lifecycle/statistical oracles in test-matrix,
public API/schema/checkpoint compatibility, deterministic replay and bounded recovery.
Maturity remains planned/experimental until the existing coordinator accepts evidence.

## Quality gates

Run the test matrix and Conductor phase/DAG/artifact/coverage checks. Record exact
source/features/input/toolchain/host/exit/output hashes and independent review.
