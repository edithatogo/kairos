# 04 The Analyst: kairo-ecs-arrow Telemetry — plan.md

## Phase 0 — Track startup

- Read `conductor/workflow.md`, `conductor/contracts/arrow-schema-contract.md`, and `conductor/contracts/conformance-contract.md`.
- Read the current shared scaffolds in `schemas/arrow`, `conformance/fixtures`, and the core crates that already exist in the workspace.
- Confirm the intended owned paths remain `crates/kairo-ecs-arrow`, `schemas/arrow`, and `examples/telemetry/`; the crate, schema JSON, and telemetry example now exist as a minimal dependency-light event-log slice.
- Create `agent-contract.md`, `risk-register.md`, `test-matrix.md`, and `handoff.md`.

## Phase 1 — Contract alignment

- Define the Arrow schema for telemetry and event-log output.
- Align field names, order, and scalar types with the shared conformance fixtures and the core event sequence.
- Propose contract changes through ADR if required.
- Add fixture stubs for the telemetry payloads this schema will support.

## Phase 2 — Scaffold

- Maintain the `kairo-ecs-arrow` crate skeleton already registered in the workspace.
- Keep Arrow schema smoke tests that prove the event-log field order, schema version, and smoke-byte roundtrip compile under package-focused gates.
- Add schema docs that describe the telemetry payload shape without implying the exporter already exists.

## Phase 3 — Implementation

- Implement the smallest useful vertical slice: a schema-backed telemetry export path for event logs.
- Add unit tests and integration tests.
- Add fixture parity checks for the exporter output against Track 12.
- Add benchmarks where serialization or export volume matters.

### Phase 3b — OpenTelemetry export

- Add `tracing-opentelemetry` and `opentelemetry-otlp` as optional dependencies.
- Implement `OtelLayer` that translates `tracing` spans into OTel spans.
- Map KairoECS event-kinds to OTel span names: `kairo_ecs.event.dispatch`, `kairo_ecs.run.step`, etc.
- Map scheduler stats (event_count, cancelled_count, queue_depth) to OTel metrics.
- Add smoke test that starts an OTLP collector in CI and verifies span export.

## Phase 4 — Cross-track integration

- Run owned tests.
- Run affected shared conformance tests.
- Update docs and release notes.
- Ensure no other subagent-owned paths were modified without handoff, especially tracks 01, 03, and 12.

## Phase 5 — Closeout

- Complete `handoff.md`.
- Record the remaining schema decisions and follow-up tasks.
- Confirm CI gates.
- Mark the track ready for the next implementation wave, not as finished.

