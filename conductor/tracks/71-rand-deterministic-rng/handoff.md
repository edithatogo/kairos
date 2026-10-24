# Handoff: Track 71 — rand Deterministic RNG Adoption

Updated: 2026-10-24. Maturity: complete; adoption decision: adopt.

## Summary

Phase 0, Phase 1, and Phase 2 completed. Adopted `rand = "0.10"` and `rand_chacha = "0.10"` (with `default-features = false`) in `crates/kairo-ecs-rng`. Replaced bespoke SplitMix64 generator with pinned `ChaCha8Rng` deterministic generator, providing exact stream isolation (`set_stream(u64)` / `get_stream() -> u64`), mid-buffer word position restoration (`set_word_pos(u128)` / `get_word_pos() -> u128`), and serializable `RngCheckpoint` with generator/version validation. Preserved intended CRN coupling across scenarios and purpose namespace isolation. Rebased `rng_replay.json` conformance fixture to verified ChaCha8 draws.

## Files changed

- `crates/kairo-ecs-rng/Cargo.toml`
- `crates/kairo-ecs-rng/src/lib.rs`
- `Cargo.lock`
- `conformance/fixtures/rng_replay.json`
- `tests/conformance/runner.mjs`
- `conductor/tracks/71-rand-deterministic-rng/*`
- `conductor/tracks.yaml`
- `conductor/tracks.md`
- `conductor/status.md`
- `conductor/phase-closeout.yaml`

## Contracts consumed

[Value policy](../../dependency-value-policy.md), `rng-agent` and `conformance-agent` ownership contracts, and `spec.md`.

## Contracts changed

Replaced SplitMix64 generator with `ChaCha8Rng`. Introduced `RngCheckpoint` and `RngError`. Retained legacy convenience functions `derive_entity_seed` and `from_entity`. Updated `rng_replay.json` expected stream for intentional alpha break.

## Tests added

- `tests::entity_seed_is_reproducible`
- `tests::stream_replays_from_run_seed`
- `tests::distinct_entity_inputs_diverge`
- `tests::entity_stream_replays_exactly`
- `tests::different_run_seeds_diverge`
- `tests::stream_isolation_distinct_streams_diverge`
- `tests::checkpoint_roundtrip_and_mid_buffer_restoration`
- `tests::checkpoint_rejects_incompatible_generator_or_version`
- `tests::purpose_isolation_and_crn_coupling`
- `tests::known_answer_vector_chacha8`

## Known risks

Alpha trajectory break from SplitMix64 to ChaCha8 accepted and documented per spec. All 14 tests in `kairo-ecs-rng` and all workspace tests passing cleanly.

## Follow-up issues

Advance to Track 72 (`rand_distr statistical sampling adoption`) following the sequencing order in `conductor/dependency-sequencing.md`.

## Integration notes

All 14 unit tests in `kairo-ecs-rng` passed. Workspace tests pass. Clippy and Rustfmt clean. Passes `cargo deny check advisories sources licenses bans` and `cargo audit`.

## Phase closeout evidence

- `$conductor-review`: Phase 0, Phase 1, and Phase 2 review executed and passed; see [review-report.md](review-report.md).
- accepted fixes: Pinned ChaCha8 generator with stream isolation, mid-buffer resume, versioned checkpointing, and conformance fixtures verified.
- rejected findings: None.
- command/cwd/commit SHA/toolchain/seed/input hashes:
  - Command: `cargo test -p kairo-ecs-rng`
  - CWD: `/Volumes/PortableSSD/github/kairos`
  - Toolchain: `rustc 1.99.1`
  - Commit SHA: 372ce5484b14729312d7d6ebe97a8072ee960d77
  - Exit code: `0`
- pushed ref: origin/main
- next-phase decision: Track 71 accepted and complete; advance to Track 72.
- Use `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` before advancement.
