# Track 68 Plan: Inter-Unit Boundary Channels & Asynchronous Handover Sinks

## Phase 0 - Boundary Channel & Egress Sink Primitives

- [ ] Task 0.1: Implement `BoundaryChannel<T>`, `EgressSink<T>`, and `HandoverPayload` in `crates/kairo-ecs-des`.
- [ ] Task 0.2: Implement stochastic transit delay distributions and handover communication duration models.

## Phase 1 - Backpressure, Buffering & Boarding Mechanics

- [ ] Task 1.1: Implement receiver capacity constraints, bounded buffers, and upstream hold/boarding mechanics.
- [ ] Task 1.2: Add unit tests for upstream entity blockage when downstream ingress is saturated.

## Phase 2 - Multi-Unit Composition & Conservation Accounting

- [ ] Task 2.1: Implement conservation assertions guaranteeing zero lost entities across asynchronous boundaries.
- [ ] Task 2.2: Add telemetry tracing for cross-boundary handovers into Arrow event logs.

## Phase 3 - Closeout

- [ ] Task 3.1: Run track-specific test matrix, workspace tests, and Conductor validators.
- [ ] Task 3.2: Run `$conductor-review`, auto-apply accepted fixes, and update `handoff.md`.
- [ ] Task 3.3: Commit and push the cleaned slice and verify GitHub Actions.
