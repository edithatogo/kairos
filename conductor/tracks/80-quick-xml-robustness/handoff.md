# Handoff: Track 80 (quick-xml FMU Metadata Replacement)

Updated: 2026-10-25. Maturity: complete; adoption decision: adopt.

## Summary

Phase 0, Phase 1, and Phase 2 completed. Adopted `quick-xml = "0.42.0"` with zero transitive runtime dependencies (`default-features = false`). Replaced fragile substring matching in `crates/kairo-ecs-fmi/src/import/fmu_loader.rs` with structured XML reading, validating root element identity, version attribute parsing, maximum nesting depth (`MAX_XML_DEPTH = 32`), and maximum document size (`MAX_XML_BYTES = 50 MB`). Implemented two-way structured serialization and deserialization in `crates/kairo-ecs-fmi/src/export/model_description.rs` (`ModelDescription::to_fmi2_xml` via `quick_xml::writer::Writer` and `ModelDescription::from_xml_str`). Deleted superseded hand-rolled XML entity replacement and string contains checks. Added 8 independent behavioral and negative oracle tests in `tests/quick_xml_metadata_oracles.rs`.

## Files changed

- `Cargo.lock`
- `crates/kairo-ecs-fmi/Cargo.toml`
- `crates/kairo-ecs-fmi/src/import/fmu_loader.rs`
- `crates/kairo-ecs-fmi/src/export/model_description.rs`
- `crates/kairo-ecs-fmi/tests/quick_xml_metadata_oracles.rs`
- `conductor/tracks/80-quick-xml-robustness/evidence/phase0-qualification.md`
- `conductor/tracks/80-quick-xml-robustness/adr-api-compatibility-and-redteam.md`
- `conductor/tracks/80-quick-xml-robustness/review-report.md`
- `conductor/tracks/80-quick-xml-robustness/plan.md`
- `conductor/tracks/80-quick-xml-robustness/handoff.md`
- `conductor/tracks/80-quick-xml-robustness/metadata.json`
- `conductor/tracks.yaml`
- `conductor/tracks.md`
- `conductor/phase-closeout.yaml`

## Contracts consumed

[Value policy](../../dependency-value-policy.md), `fmi-agent` ownership contract, and `spec.md`.

## Contracts changed

Extended `crates/kairo-ecs-fmi` with structured XML parsing:
- `pub const MAX_XML_BYTES: u64 = 50 * 1024 * 1024` (50 MB)
- `pub const MAX_XML_DEPTH: usize = 32`
- `ModelDescription::from_xml_str(xml: &str) -> FmiResult<ModelDescription>`
- `Causality::from_fmi2`, `Variability::from_fmi2`, `VariableType::from_fmi2`

## Tests added

- `crates/kairo-ecs-fmi/tests/quick_xml_metadata_oracles.rs`:
  - `oracle_false_root_in_comment_rejected`
  - `oracle_false_version_in_comment_rejected`
  - `oracle_fmi3_version_accepted`
  - `oracle_malformed_unclosed_xml_rejected`
  - `oracle_xml_nesting_depth_budget_enforced`
  - `oracle_xml_file_size_budget_enforced`
  - `oracle_roundtrip_serialization_and_attribute_order`
  - `oracle_validate_generated_fmi2_xml_oracle`

## Known risks

Resource limits protect against memory and stack exhaustion: depth bounded to 32 levels and size bounded to 50 MB. External DTD entity resolution is omitted by default in `quick-xml`, preventing XXE attacks.

## Follow-up issues

Advance to next sequenced track per `conductor/dependency-sequencing.md` and `conductor/track-map.md`.

## Integration notes

All 52 tests in `kairo-ecs-fmi` passed. Pure-Rust compilation verified with zero C or `libclang` dependencies. Passes `cargo deny check advisories sources licenses bans` (0 advisories, 0 bans, 0 license errors) and `cargo audit` (399 dependencies scanned, 0 vulnerabilities).

## Phase closeout evidence

- `$conductor-review`: Phase 0, Phase 1, and Phase 2 review executed and passed; see [review-report.md](review-report.md).
- accepted fixes: Adopted `quick-xml 0.42.0` with minimal features; structured root and version extraction; nesting depth and document size limits; attribute order agnosticism; clippy and fmt hygiene fixes.
- rejected findings: None.
- command/cwd/commit SHA/toolchain/seed/input hashes:
  - Command: `cargo test -p kairo-ecs-fmi --all-features`
  - CWD: `/Volumes/PortableSSD/github/kairos`
  - Toolchain: `rustc 1.99.1`
  - Commit SHA: pending-commit
  - Exit code: `0`
- pushed ref: origin/feat/track-80-quick-xml-fmu-metadata
- next-phase decision: Track 80 is Done; advance to next track.
- Use `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` before advancement.
