# Handoff: Track 76 — toml Scenario Configuration Parsing

Updated: 2026-10-24. Maturity: complete; adoption decision: adopt.

## Summary

Phase 0, Phase 1, and Phase 2 completed. Adopted `toml = "0.8.23"` in `crates/kairo-ecs-cli` for robust scenario and seed manifest parsing, replacing fragile bespoke line parsing while preserving backward compatibility and domain contracts.

## Files changed

- `crates/kairo-ecs-cli/Cargo.toml`
- `crates/kairo-ecs-cli/src/scenario.rs`
- `Cargo.lock`
- `conductor/tracks/76-toml-robustness/*`
- `conductor/tracks.yaml`
- `conductor/tracks.md`
- `conductor/phase-closeout.yaml`

## Contracts consumed

[Value policy](../../dependency-value-policy.md), `scenario-runner-agent` ownership contract, and scenario schema definitions.

## Contracts changed

Replaced brittle ad-hoc line-by-line parser with standard `toml::from_str`. Added `streams: BTreeMap<String, u64>` to `SeedManifest` to parse and retain seed streams table. Supported both TOML native arrays and legacy comma strings for `expected_kind_order`. Enforced `#[serde(deny_unknown_fields)]` and duplicate key rejection.

## Tests added

- `scenario::tests::toml_sections_comments_and_escapes`
- `scenario::tests::toml_rejects_unknown_fields_in_scenario`
- `scenario::tests::toml_rejects_duplicate_keys`
- `scenario::tests::toml_legacy_comma_delimited_string_compatibility`
- `scenario::tests::load_scenario_reports_missing_field`
- `scenario::tests::load_scenario_reports_invalid_numeric_value`
- `scenario::tests::load_seed_manifest_reports_missing_field`
- `scenario::tests::load_seed_manifest_reports_invalid_numeric_value`
- `scenario::tests::test_validate_scenario_and_seed_success`

## Known risks

None identified. Deprecated ad-hoc parsing functions completely removed. Backward compatibility with comma-delimited string format verified via test suite.

## Follow-up issues

Advance to Track 77 (`clap command line parsing`) to replace handwritten CLI argument parsing in `crates/kairo-ecs-cli` and `crates/kairo-ecs-debug`.

## Integration notes

All 37 unit tests in `kairo-ecs-cli` passed. All workspace tests passed. Clippy and Rustfmt clean. Passes `cargo deny check advisories sources licenses bans`.

## Phase closeout evidence

- `$conductor-review`: Phase 0, Phase 1, and Phase 2 review executed and passed; see [review-report.md](review-report.md).
- accepted fixes: Zero defects remaining; compiler error on missing streams field resolved; TOML span/error mapping to `ScenarioError` resolved.
- rejected findings: None.
- command/cwd/commit SHA/toolchain/seed/input hashes:
  - Command: `cargo test -p kairo-ecs-cli && cargo test --workspace`
  - CWD: `/Volumes/PortableSSD/github/kairos`
  - Toolchain: `rustc 1.99.1`
  - Commit SHA: b0d8619f4cac4956aff9f26c3b54b2cc70911545
  - Exit code: `0`
- pushed ref: `refs/heads/feat/track-76-toml-robustness`
- next-phase decision: Track 76 accepted and complete; advance to Track 77.
- Use `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` before advancement.
