# Handoff: Track 75 — serde_json Metadata and Control Encoding

Updated: 2026-10-24. Maturity: completed; Phase 0, 1, 2 accepted.

## Summary

Track 75 evaluated and adopted `serde_json` 1.0.154 for robust, RFC 8259-compliant JSON control metadata and trace encoding across `kairo-ecs-cli` and `kairo-ecs-debug`. Handwritten `format!` string templates and manual escaping were eliminated in favor of direct `serde_json::to_string_pretty` and `to_string` serialization over the accepted typed DTO contracts from Track 74. Tabular execution data remains strictly in Polars and Parquet.

## Files changed

- `crates/kairo-ecs-cli/Cargo.toml`
- `crates/kairo-ecs-cli/src/contracts.rs`
- `crates/kairo-ecs-cli/src/main.rs`
- `crates/kairo-ecs-debug/Cargo.toml`
- `crates/kairo-ecs-debug/src/contracts.rs`
- `conductor/tracks/75-serde-json-robustness/*`
- `conductor/tracks.yaml`
- `conductor/tracks.md`
- `conductor/phase-closeout.yaml`
- `conductor/status.md`
- `conductor/track-map.md`

## Contracts consumed

- Track 74 Serde typed DTO control contracts (`RunManifestDto`, `SummaryDto`, `ReplayComparisonDto`, `ResumabilityPlanDto`, `CheckpointManifestDto`, `ResumeRequestDto`, `TraceHeaderDto`, `TraceSnapshotDto`, `TraceDeltaDto`, `TraceRecordDto`).
- `conductor/dependency-value-policy.md` robustness criteria.

## Contracts changed

- CLI writes `manifest.json`, `summary.json`, `replay-comparison.json`, `resumability-plan.json`, `checkpoint-manifest.json`, `resume-request.json`, and `run-request.json` via `serde_json` over typed DTO contracts with schema validation.
- Added `RunRequestDto` contract in `crates/kairo-ecs-cli/src/contracts.rs`.
- Added JSON and JSON Lines serialization, parsing, and stream validation in `crates/kairo-ecs-debug/src/contracts.rs` (`TraceRecordDto::to_json_line`, `TraceRecordDto::from_json_str`, `validate_jsonl_trace_records`).

## Tests added

- `crates/kairo-ecs-cli/src/contracts.rs`:
  - `run_request_dto_roundtrip_and_validation`
  - `unicode_and_control_character_escaping_preservation`
  - `rejects_non_integer_or_floats_in_integer_fields`
  - `rejects_malformed_json_and_duplicate_keys`
- `crates/kairo-ecs-debug/src/contracts.rs`:
  - `unicode_and_control_character_escaping_in_trace_state`
  - `exact_u128_tick_preservation_no_f64_narrowing`
  - `rejects_nonfinite_values_and_malformed_inputs`
  - `jsonl_stream_encoding_and_monotonicity_validation`

## Known risks

- Nonfinite values (`NaN`, `Infinity`) in JSON: `serde_json` rejects them by default during serialization, adhering to RFC 8259.
- Large `u128` ticks: preserved directly without `f64` narrowing through dedicated Serde deserializer.

## Follow-up issues

- Track 76 (`toml scenario configuration parsing`) directly follows Track 75 to evaluate and adopt `toml` for robust scenario configuration.

## Integration notes

- Zero new transitive dependencies added to `Cargo.lock` (`serde_json` was already pinned at 1.0.154 in workspace dev-dependencies).

## Phase closeout evidence

- `$conductor-review`: Phase 0-2 review completed; serde_json 1.0.154 qualified and adopted; handwritten format templates replaced with typed contract serialization; all 33 CLI tests, 21 Debug tests, and workspace tests passing cleanly.
- accepted fixes: Added `RunRequestDto` typed contract and removed obsolete handwritten `csv_u32` formatting helper.
- commit SHA: `48f027002130099a90e19bcd6a3d838a8d4036a3` (integration ancestor; topic commit on `origin/feat/track-75-serde-json-robustness`)
- pushed ref: `origin/feat/track-75-serde-json-robustness`
- `validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree`: verified clean tree state across phase closeout.
- next-phase decision: Track 75 is closed; advance to Track 76.
