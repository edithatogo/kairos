# Handoff

Updated: 2026-10-22. Maturity: planned; adoption decision pending.

## Summary

Separate dependency track registered. No runtime change, installation or accepted implementation phase.

## Files changed

Planning folder and coordinated registry/policy surfaces only.

## Contracts consumed

[Value policy](../../dependency-value-policy.md), existing owner contracts and prerequisites in spec.md.

## Contracts changed

Prospective replacement boundaries only; no runtime API changed.

## Tests added

Independent future tests specified in test-matrix.md; planning validators only this pass.

## Known risks

No adoption to replace a trivial objective function or absent optimisation loop; identify concrete code deletion/avoided implementation first.

## Follow-up issues

Record exact existing consumer, baseline and adopt/no-go decision in Phase 0. Parent adoption requires verified upstream receipts and its own claim.

## Integration notes

See [programme](../../robust-dependencies.md). Worktree integration, local main, hosted CI and runtime acceptance remain distinct.

## Phase closeout evidence

Pending: `$conductor-review`, accepted fixes, rejected findings, command/cwd/commit SHA/toolchain/seed/input hashes, exit codes/artifacts, pushed ref or blocker and next-phase decision. Use `pwsh -NoProfile -File scripts/validate_conductor_git_closeout.ps1 -RequireCleanWorkingTree` before advancement.

## Phase 0 decision — 1 November 2026

**Defer Argmin adoption.** At Kairos `41e58e0c012e3f2253559cbefb239db71ee7486c`,
the prospective CLI consumer has no iterative optimiser to replace. Track 78
merged its bounded Rayon executor in [PR #317](https://github.com/edithatogo/kairos/pull/317).
`OptunaBatchObjective::evaluate` computes RMSE over the common input prefix.
`AbcSmcCalibrator` evaluates caller-supplied particles, applies a tolerance to
mean absolute discrepancy and normalises accepted weights. Neither API proposes
the next parameter vector, checks solver convergence, or saves an optimiser
iteration. No production caller requiring such machinery was found in the
reviewed Rust source. Replacing the statistical weighting or the parallel
iterator would cross the track's contract.

**Native baseline.** On Darwin arm64 with Rust/Cargo 1.99.1, both commands
passed one selected test each (exit 0):

```sh
cargo test --locked --offline -p kairo-ecs-cli --lib optuna_abc_smc_calibration_hook
cargo test --locked --offline -p kairo-ecs-cli --test rayon_ensemble_oracles test_abc_smc_calibrator_parallel_bit_invariance -- --exact
```

Review added a separate analytical check because a zero loss on identical
inputs alone cannot distinguish RMSE from several other objectives. A temporary
Rust caller linked the actual built `kairo_ecs_cli` library. Trial 90, parameters
[3,4] and target [0,0] produced sqrt(12.5) = 3.5355339059327378. An identity
simulator, particles [0], [1], [4], reference [0] and tolerance 1 produced three
evaluations, acceptance [true,true,false] and weights [2/3,1/3,0]. These
fixtures use no stochastic inputs. The hook also returns zero for an empty
overlap and for [2] versus [2,9]. A future solver owner must explicitly define
input validity before treating zero loss as convergence.

**Candidate boundary.** Crates.io reported argmin 0.11.0 as the latest stable,
non-yanked release, MIT OR Apache-2.0 with no declared rust_version in that
release metadata. The upstream repository was unarchived at readback. The
Phase 0 stop criterion precedes installation: no Argmin feature, MSRV,
convergence tolerance, restart contract, timing or security qualification is
claimed. No production code or dependencies change. Reopen when an executable
consumer supplies a parameter-update loop or required solver, bounds, termination
criteria, noise/seed policy and resumable state. Compare that implementation
against a known optimum and an independent solver before replacing it.

The existing CareOps Sim handoff row `90-argmin-robustness` is unresolved with
impact review pending. CareOps retains its calibration and pin ownership.
The existing Kairos integrator retains shared registry and phase acceptance.

**Coordination.** Chat 01a19600-0a41-7ca2-9e4b-e27d7339c8e7 owns the track
directory in /Users/doughnut/.codex/worktrees/track48-rollback/kairos on
codex/track90-argmin-decision. Native scope was local tests and read-only
package/PR/consumer inspection. Next operation is protected integration.

Worktree-local raw receipt SHA-256 values:

- `.venv/hook90.log` — f5dc42e8efe7fc78a1781eab67f01f8af166bd5bd246348af7c3c3314b17256b
- `.venv/parity90.log` — a9000aef48f970aceedd63d4fa2c00d023501aee6bf50b3eb48052ba33086a34
- `.venv/track90_probe.rs` — ee89ab657e616238d3b7a9ca7b78f3890254cd87decd65c31353ec8151ea6122
- `.venv/probe90.log` — f8648a40bdc3e38f9c6ae7315842d13088b7d4d5c01734b73a8a2a7211e8dcf0
