# Handoff: Track 77 — Clap CLI Consolidation

Updated: 2026-10-24. Maturity: complete; adoption decision: adopt.

## Summary

Phase 0, Phase 1, and Phase 2 completed. Consolidated CLI argument parsing in `crates/kairo-ecs-cli` using `clap` derive subcommands (`clap = { version = "4", features = ["derive"] }`). Replaced fragile handwritten argument parsing while preserving exact exit codes, stdout serialization, and domain contracts. Added zero new external dependencies since `clap 4.5.61` is already locked and used in the workspace.

## Files changed

- `crates/kairo-ecs-cli/Cargo.toml`
- `crates/kairo-ecs-cli/src/main.rs`
- `crates/kairo-ecs-cli/tests/cli_process_stdout.rs`
- `conductor/tracks/77-clap-robustness/*`
- `conductor/tracks.yaml`
- `conductor/tracks.md`
- `conductor/status.md`
- `conductor/phase-closeout.yaml`

## Contracts consumed

[Value policy](../../dependency-value-policy.md), `contracts-agent` ownership contract, and CLI subcommand stdout/status contracts.

## Contracts changed

Replaced bespoke argument parsing (`flag_path`, `optional_flag_path`, manual arg loops, and `print_help`) with standard typed Clap derive subcommands (`ValidateScenario`, `Replay`, `ResumePlan`, `Run`, `RunCaptured`, `Checkpoint`, `Resume`, `Careops`, `Nhra`). Preserved exact stdout serialization and error exit codes.

## Tests added

- `cli_process_stdout::test_cli_help_flag_succeeds`
- `cli_process_stdout::test_cli_empty_args_shows_help_or_error`
- `cli_process_stdout::test_cli_missing_required_flag_fails`
- `cli_process_stdout::test_cli_unknown_flag_fails`
- `cli_process_stdout::test_cli_reserved_track22_commands`

## Known risks

None identified. Deprecated handwritten parser functions completely deleted. All 37 CLI unit tests and 5 process integration tests passing.

## Follow-up issues

Advance to Track 95 (`thiserror contracts`) for standardizing error definitions across crates.

## Integration notes

All 42 tests in `kairo-ecs-cli` passed (37 unit tests + 5 process integration tests). Clippy and Rustfmt clean. Passes `cargo deny check advisories sources licenses bans`.

## Phase closeout evidence

- `$conductor-review`: Phase 0, Phase 1, and Phase 2 review executed and passed; see [review-report.md](review-report.md).
- accepted fixes: Zero defects remaining; standard Clap derive implemented; reserved Track 22 stub commands preserved; process integration tests verified.
- rejected findings: None.
- command/cwd/commit SHA/toolchain/seed/input hashes:
  - Commit SHA: 9f94fd334938e7ab3252749aa6a7ade5a1086238
- pushed ref: origin/main
- next-phase decision: Track 77 accepted and complete; advance to Track 95.
- Use `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` before advancement.
