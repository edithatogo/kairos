# Handoff: Track 74 — Serde Typed Control Contracts

Updated: 2026-10-24. Maturity: completed; Phase 0, 1, 2 accepted.

## Summary

Track 74 evaluated and adopted `serde` with derive macros to establish typed, validated data transfer objects across `kairo-ecs-cli` and `kairo-ecs-debug`. Handwritten JSON formatting was replaced at contract boundaries with declarative DTOs enforcing schema version tags, full-range IDs/ticks, and strict unknown field rejection (`deny_unknown_fields`).

## Files changed

- `crates/kairo-ecs-cli/Cargo.toml`
- `crates/kairo-ecs-cli/src/contracts.rs`
- `crates/kairo-ecs-cli/src/main.rs`
- `crates/kairo-ecs-debug/Cargo.toml`
- `crates/kairo-ecs-debug/src/contracts.rs`
- `crates/kairo-ecs-debug/src/lib.rs`
- `conductor/tracks/74-serde-robustness/*`
- `conductor/tracks.yaml`
- `conductor/tracks.md`
- `conductor/phase-closeout.yaml`

## Contracts consumed

- Track 70 accepted Polars and Arrow IPC baseline.
- `conductor/dependency-value-policy.md` robustness criteria.

## Contracts changed

- CLI control manifests now construct and validate typed Serde contracts (`RunManifestDto`, `SummaryDto`, `ReplayComparisonDto`, `ResumabilityPlanDto`, `CheckpointManifestDto`, `ResumeRequestDto`).
- Debug tracing defines typed DTO contracts (`TraceHeaderDto`, `TraceSnapshotDto`, `TraceDeltaDto`, `TraceRecordDto`).

## Tests added

- `crates/kairo-ecs-cli/src/contracts.rs`:
  - `run_manifest_dto_roundtrip_and_validation`
  - `run_manifest_dto_rejects_unknown_fields`
  - `run_manifest_dto_rejects_invalid_schema_version`
  - `summary_dto_roundtrip_full_ranges`
  - `resumability_plan_dto_roundtrip`
  - `checkpoint_manifest_and_resume_request_roundtrips`
- `crates/kairo-ecs-debug/src/contracts.rs`:
  - `trace_header_roundtrip_and_validation`
  - `trace_header_rejects_unknown_fields`
  - `snapshot_and_delta_roundtrip_full_range_integers`
  - `trace_record_enum_tagged_roundtrip`

## Known risks

- Internal tagging with `u128` ticks in Serde requires adjacently tagged or custom representation; resolved by adjacently tagged polymorphic `TraceRecordDto` (`#[serde(tag = "type", content = "payload")]`).

## Follow-up issues

- Track 75 (`serde_json metadata and control encoding`) directly follows Track 74 to promote `serde_json` to runtime production dependency and serialize/deserialize control files using these typed DTOs.

## Integration notes

- Zero new transitive dependencies added to `Cargo.lock` (`serde` was already resolved at v1.0.229 in workspace).

## Phase closeout evidence

- `$conductor-review`: Phase 0-2 review completed; Serde v1.0.229 qualified and adopted; derived typed DTO control contracts for CLI run manifests, summary records, replay comparisons, resumability plans, checkpoint manifests, and trace records; all unit and workspace tests passing.
- accepted fixes: Adjacently tagged polymorphic trace record DTO to ensure full-range `u128` ticks deserialization compatibility without lossy conversion.
- commit SHA: `1b0be3c810b099a7495a567585776dead280d24b` (integration ancestor; topic commit on `origin/feat/track-74-serde-robustness`)
- pushed ref: `origin/feat/track-74-serde-robustness`
- `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`: verified clean tree state across phase closeout.
- next-phase decision: Track 74 is closed; advance to Track 75.
