# Track 68: Inter-Unit Boundary Channels & Asynchronous Handover Sinks

## Objective

Introduce SimPy/DES-native asynchronous cross-boundary transit buffers (`BoundaryChannel`, `EgressSink`, `HandoverPayload`) to `crates/kairo-ecs-des`. Enables modular simulation components to transfer entities across service boundaries with stochastic transit times, handover communication delays, and receiver buffer capacity backpressure / boarding holds without requiring tight coupling or synchronous shared state across units.

## Inputs

- `crates/kairo-ecs-des` process and resource primitives from Track 64.
- `crates/kairo-ecs-streaming` event channels.
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

Non-breaking addition to `kairo-ecs-des`. Existing single-unit DES processes continue unaffected. Adds inter-unit decoupled queuing primitives.
