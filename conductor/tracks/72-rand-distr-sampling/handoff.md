# Handoff: Track 72 — rand_distr Statistical Sampling Adoption

Updated: 2026-10-24. Maturity: complete; adoption decision: adopt.

## Summary

Phase 0, Phase 1, and Phase 2 completed. Adopted `rand_distr = { version = "0.6", default-features = false, features = ["alloc"] }` in `crates/kairo-ecs-rng`. Replaced hand-rolled `draw_uniform_open01` (53-bit mantissa shift) with library `rand_distr::Open01` and `draw_standard_threshold` with library `rand_distr::Exp1`. Re-exported `rand_distr` and `rand_distr::Distribution` in `crates/kairo-ecs-rng` and added `DeterministicStream::sample<T, D: Distribution<T>>(&mut self, distr: D) -> T`. Preserved non-Markovian survival integration/inversion (`HazardRate::invert_cumulative_hazard`, `cumulative_hazard`, `hazard_at`), tie-breaking, and censoring semantics. Verified independent oracles, covariate shift invariance, mid-buffer checkpoint replay determinism, and performance throughput.

## Files changed

- `crates/kairo-ecs-rng/Cargo.toml`
- `crates/kairo-ecs-rng/src/lib.rs`
- `crates/kairo-ecs-rng/src/competing_risks.rs`
- `crates/kairo-ecs-rng/tests/sampling_independent_oracles.rs`
- `crates/kairo-ecs-rng/tests/sampling_hazard_parity.rs`
- `crates/kairo-ecs-rng/tests/sampling_resume_cost.rs`
- `Cargo.lock`
- `CHANGELOG.md`
- `conductor/tracks/72-rand-distr-sampling/*`
- `conductor/tracks.yaml`
- `conductor/tracks.md`
- `conductor/status.md`
- `conductor/phase-closeout.yaml`

## Contracts consumed

[Value policy](../../dependency-value-policy.md), `rng-agent`, `vv-uq-agent`, and `des-api-agent` ownership contracts, and `spec.md`.

## Contracts changed

Exposed `rand_distr` and `rand_distr::Distribution` on `kairo_ecs_rng`. Added `DeterministicStream::sample`. Handcrafted threshold functions `draw_uniform_open01` and `draw_standard_threshold` now delegate to `Open01` and `Exp1`.

## Tests added

- `tests/sampling_independent_oracles.rs`:
  - `test_uniform_float_independent_oracle`
  - `test_uniform_integer_independent_oracle`
  - `test_open01_independent_oracle`
  - `test_exponential_independent_oracle`
  - `test_normal_independent_oracle`
  - `test_bernoulli_independent_oracle`
  - `test_binomial_independent_oracle`
  - `test_geometric_independent_oracle`
  - `test_weibull_independent_oracle`
- `tests/sampling_hazard_parity.rs`:
  - `test_draw_uniform_open01_strictly_open_and_unclamped`
  - `test_draw_standard_threshold_exponential_density`
  - `test_residual_survival_clock_invariance_under_covariate_shift`
  - `test_weibull_and_gompertz_competing_hazards_parity`
- `tests/sampling_resume_cost.rs`:
  - `test_distribution_sampling_checkpoint_and_mid_buffer_restoration`
  - `test_entity_stream_isolation_with_distributions`
  - `test_distribution_sampling_throughput_budget`

## Known risks

Distribution sampling trajectory divergence from legacy handwritten float truncation accepted and validated per specification. All 31 tests in `kairo-ecs-rng` and all workspace tests pass.

## Follow-up issues

Advance to Track 78 (`Rayon parallel execution consolidation`) following the sequencing order in `conductor/dependency-sequencing.md`.

## Integration notes

All 31 unit & integration tests in `kairo-ecs-rng` passed. Workspace tests pass. Clippy and Rustfmt clean. Passes `cargo deny check advisories sources licenses bans` and `cargo audit`.

## Phase closeout evidence

- `$conductor-review`: Phase 0, Phase 1, and Phase 2 review executed and passed; see [review-report.md](review-report.md).
- accepted fixes: Pinned `rand_distr 0.6` with Open01/Exp1/Uniform/Normal sampling, mid-buffer resume determinism, and independent test oracles.
- rejected findings: None.
- command/cwd/commit SHA/toolchain/seed/input hashes:
  - Command: `cargo test -p kairo-ecs-rng`
  - CWD: `/Volumes/PortableSSD/github/kairos`
  - Toolchain: `rustc 1.99.1`
  - Commit SHA: 1f19d8a0b56529f971574ffa1aa9bb520faa6966
  - Exit code: `0`
- pushed ref: origin/feat/track-72-rand-distr-sampling
- next-phase decision: Track 72 accepted and complete; advance to Track 78.
- Use `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` before advancement.
