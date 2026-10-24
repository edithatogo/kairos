# Handoff: Track 78 — Rayon Bounded Independent-Work Execution

Updated: 2026-10-24. Maturity: complete; adoption decision: adopt.

## Summary

Phase 0, Phase 1, and Phase 2 completed. Adopted `rayon = "1.12"` in `crates/kairo-ecs-cli`. Replaced 4 instances of repeated `std::thread::scope` partition/spawn/join machinery in `crates/kairo-ecs-cli/src/ensemble.rs` (`ParallelEnsembleRunner`, `SobolSensitivityAnalyzer`, and `AbcSmcCalibrator`) with bounded Rayon thread pools and parallel iterators. Preserved bit-for-bit replay determinism across arbitrary worker thread counts (1, 2, 4, 8) through indexed results collection and fixed-order reductions. Verified work stealing for skewed workloads, panic propagation, reusable thread pool isolation, and high-throughput evaluation (>100k evals/sec).

## Files changed

- `crates/kairo-ecs-cli/Cargo.toml`
- `crates/kairo-ecs-cli/src/lib.rs`
- `crates/kairo-ecs-cli/src/main.rs`
- `crates/kairo-ecs-cli/src/ensemble.rs`
- `crates/kairo-ecs-cli/tests/rayon_ensemble_oracles.rs`
- `CHANGELOG.md`
- `conductor/tracks/78-rayon-robustness/*`
- `conductor/tracks.yaml`
- `conductor/tracks.md`
- `conductor/status.md`
- `conductor/phase-closeout.yaml`

## Contracts consumed

[Value policy](../../dependency-value-policy.md), `experiment-agent` ownership contract, and `spec.md`.

## Contracts changed

Exposed `build_bounded_thread_pool` and re-exported `rayon` on `kairo_ecs_cli::ensemble`. Added `run_sweep_with_pool`, `analyze_parallel_with_pool`, and `evaluate_particles_parallel_with_pool` for thread-pool reuse across sequential sweeps.

## Tests added

- `crates/kairo-ecs-cli/tests/rayon_ensemble_oracles.rs`:
  - `test_parallel_sweep_thread_count_bit_invariance`
  - `test_sobol_sensitivity_parallel_bit_invariance`
  - `test_abc_smc_calibrator_parallel_bit_invariance`
  - `test_skewed_task_distribution_work_stealing`
  - `test_panic_propagation_in_rayon_pool`
  - `test_reusable_pool_isolation_and_throughput`
  - `test_empty_and_single_point_boundaries`
  - `test_large_ensemble_throughput_budget`

## Known risks

Rayon thread pool is strictly confined to embarrassingly parallel parameter sweeps and Monte Carlo particle filtering; never used for DES event scheduling or ordering.

## Follow-up issues

Advance to Track 79 (`zip FMU archive replacement`) or next queued track following `conductor/dependency-sequencing.md`.

## Integration notes

All 47 unit & integration tests in `kairo-ecs-cli` passed. All workspace tests pass. Clippy and Rustfmt clean. Passes `cargo deny check advisories sources licenses bans` and `cargo audit`.

## Phase closeout evidence

- `$conductor-review`: Phase 0, Phase 1, and Phase 2 review executed and passed; see [review-report.md](review-report.md).
- accepted fixes: Bounded `rayon 1.12` with work-stealing parallel iterators, reusable thread pools, and independent test oracles.
- rejected findings: None.
- command/cwd/commit SHA/toolchain/seed/input hashes:
  - Command: `cargo test -p kairo-ecs-cli`
  - CWD: `/Volumes/PortableSSD/github/kairos`
  - Toolchain: `rustc 1.99.1`
  - Commit SHA: PENDING_COMMIT_SHA
  - Exit code: `0`
- pushed ref: origin/feat/track-78-rayon-robustness
- next-phase decision: Track 78 accepted and complete; advance to Track 79.
- Use `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` before advancement.
