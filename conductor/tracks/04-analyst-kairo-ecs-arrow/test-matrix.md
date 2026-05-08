# Test Matrix — 04 The Analyst: kairo-ecs-arrow Telemetry

## Required tests

- `cargo test -p kairo-ecs-arrow` for schema versioning and event-log roundtrip smoke checks.
- `cargo test -p kairo-ecs-arrow --test schema_compatibility` for the `arrow-schema-versioning` gate: stream version, field order, runtime schema fingerprint, checked-in JSON schema alignment, and smoke roundtrip compatibility.
- `cargo run -p kairo-ecs-arrow --example telemetry_event_log_roundtrip` for the telemetry example.
- `cargo test -p kairo-ecs-core` to keep the Arrow schema aligned with the event model it reports on.
- `cargo test -p kairo-ecs-state` to confirm the schema still reflects the current state transitions.
- `cargo fmt --all --check` before any schema or contract edit is handed off.
- `pwsh -NoProfile -File scripts\validate_conductor_setup.ps1 -SkipCargo` to confirm the conductor setup remains intact.
- `pwsh -NoProfile -File scripts\validate_track_coverage.ps1 -SkipCargo` to confirm Track 04 is still represented in the registry and wave policy.
- `cargo test --workspace` once a real `kairo-ecs-arrow` crate exists and starts exporting Arrow payloads.

## Deferred tests

- OTel export smoke test with an OTLP collector receiving spans. Deferred until the `otel-export` feature is implemented behind dependency policy review.
- Full Arrow IPC/Parquet reader roundtrip. Deferred until native Arrow dependency policy and cross-language reader fixtures are accepted.

## Current CI commands

```bash
cargo fmt --all --check
cargo fmt --package kairo-ecs-arrow --check
cargo check -p kairo-ecs-arrow --examples
cargo test -p kairo-ecs-arrow --test schema_compatibility
cargo test -p kairo-ecs-arrow
cargo run -p kairo-ecs-arrow --example telemetry_event_log_roundtrip
cargo test -p kairo-ecs-core
cargo test -p kairo-ecs-state
pwsh -NoProfile -File scripts\validate_conductor_setup.ps1 -SkipCargo
pwsh -NoProfile -File scripts\validate_track_coverage.ps1 -SkipCargo
```
## Phase closeout gate

- `pwsh -NoProfile -File scripts/validate_conductor_phase_gates.ps1` and `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1` must pass before any phase advances; this enforces `$conductor-review`, auto-apply of accepted fixes, phase-closeout ledger evidence, cleaned commit/push evidence, and blocker recording. At actual closeout, run `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` after commit and push.
